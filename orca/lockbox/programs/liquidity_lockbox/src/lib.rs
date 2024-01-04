pub mod state;
use anchor_lang::prelude::*;
use anchor_spl::token::{self, Mint, Token, TokenAccount, Transfer};
use whirlpool::{
  self,
  state::{Whirlpool, TickArray, Position},
  cpi::accounts::ModifyLiquidity,
  cpi::accounts::UpdateFeesAndRewards,
  cpi::accounts::CollectFees,
  cpi::accounts::ClosePosition
};
use solana_program::{pubkey::Pubkey, program::invoke_signed};
use spl_token::instruction::{burn_checked, close_account, mint_to};
pub use state::*;

declare_id!("7ahQGWysExobjeZ91RTsNqTCN3kWyHGZ43ud2vB7VVoZ");

#[program]
pub mod liquidity_lockbox {
  use super::*;
  use solana_program::pubkey;

  // Orca Whirlpool program address
  const ORCA: Pubkey = pubkey!("whirLbMiicVdio4qvUfM5KAg6Ct8VwpYzGff3uctyCc");
  // OLAS-SOL Whirlpool address
  const WHIRLPOOL: Pubkey = pubkey!("5dMKUYJDsjZkAD3wiV3ViQkuq9pSmWQ5eAzcQLtDnUT3");
  // Full range lower and upper indexes
  const TICK_LOWER_INDEX: i32 = -443584;
  const TICK_UPPER_INDEX: i32 = 443584;
  // Bridged token decimals
  const BRIDGED_TOKEN_DECIMALS: u8 = 8;


  /// Initializes a Lockbox account that stores state data.
  ///
  /// ### Parameters
  /// - `bridged_token_mint` - Bridged token mint for tokens issued in return for the position liquidity.
  pub fn initialize(
    ctx: Context<InitializeLiquidityLockbox>,
    bridged_token_mint: Pubkey
  ) -> Result<()> {
    // Get the lockbox account
    let lockbox = &mut ctx.accounts.lockbox;

    // Get the anchor-derived bump
    let bump = *ctx.bumps.get("lockbox").unwrap();

    Ok(lockbox.initialize(
      bump,
      bridged_token_mint
    )?)
  }

  /// Deposits an NFT position under the Lockbox management and gets bridged tokens minted in return.
  pub fn deposit(ctx: Context<DepositPositionForLiquidity>) -> Result<()> {
    let whirlpool = ctx.accounts.position.whirlpool;
    let position_mint = ctx.accounts.position.position_mint;
    let liquidity = ctx.accounts.position.liquidity;
    let tick_lower_index = ctx.accounts.position.tick_lower_index;
    let tick_upper_index = ctx.accounts.position.tick_upper_index;

    // Check the whirlpool
    if whirlpool != WHIRLPOOL {
        return Err(ErrorCode::WrongWhirlpool.into());
    }

    // Check for the zero liquidity in position
    if liquidity == 0 {
      return Err(ErrorCode::LiquidityZero.into());
    }
    // Check that the liquidity is within uint64 bounds
    if liquidity > std::u64::MAX as u128 {
      return Err(ErrorCode::LiquidityOverflow.into());
    }

    // Check tick values
    if tick_lower_index != TICK_LOWER_INDEX || tick_upper_index != TICK_UPPER_INDEX {
      return Err(ErrorCode::OutOfRange.into());
    }

    // Check the PDA ownership
    let owner = ctx.accounts.position.to_account_info().owner;
    if owner != &ORCA {
      return Err(ErrorCode::WrongOwner.into());
    }

    // Check the PDA address correctness
    let position_pda = Pubkey::try_find_program_address(&[b"position", position_mint.as_ref()], &ORCA);
    let position_pda_pubkey = position_pda.map(|(pubkey, _)| pubkey);
    if position_pda_pubkey.unwrap() != ctx.accounts.position.key() {
      return Err(ErrorCode::WrongPositionPDA.into());
    }

    let position_liquidity = liquidity as u64;

    // Transfer position to the program PDA ATA
    token::transfer(
      CpiContext::new(
        ctx.accounts.token_program.to_account_info(),
        Transfer {
          from: ctx.accounts.position_token_account.to_account_info(),
          to: ctx.accounts.pda_position_account.to_account_info(),
          authority: ctx.accounts.position_authority.to_account_info(),
        },
      ),
      1,
    )?;

    // Close user position account
    invoke_signed(
      &close_account(
        ctx.accounts.token_program.key,
        ctx.accounts.position_token_account.to_account_info().key,
        ctx.accounts.position_authority.to_account_info().key,
        ctx.accounts.position_authority.to_account_info().key,
        &[],
      )?,
      &[
        ctx.accounts.token_program.to_account_info(),
        ctx.accounts.position_token_account.to_account_info(),
        ctx.accounts.position_authority.to_account_info(),
        ctx.accounts.position_authority.to_account_info(),
      ],
      &[],
    )?;

    // Mint bridged tokens in the amount of position liquidity
    invoke_signed(
      &mint_to(
        ctx.accounts.token_program.key,
        ctx.accounts.bridged_token_mint.to_account_info().key,
        ctx.accounts.bridged_token_account.to_account_info().key,
        ctx.accounts.lockbox.to_account_info().key,
        &[ctx.accounts.lockbox.to_account_info().key],
        position_liquidity,
      )?,
      &[
        ctx.accounts.bridged_token_mint.to_account_info(),
        ctx.accounts.bridged_token_account.to_account_info(),
        ctx.accounts.lockbox.to_account_info(),
        ctx.accounts.token_program.to_account_info(),
      ],
      &[&ctx.accounts.lockbox.seeds()],
    )?;

    // Record position liquidity amount and its correspondent account address
    let lockbox = &mut ctx.accounts.lockbox;
    lockbox.position_accounts.push(ctx.accounts.position.key());
    lockbox.position_pda_ata.push(ctx.accounts.pda_position_account.key());
    lockbox.position_liquidity.push(position_liquidity);

    // Increase the amount of total bridged token liquidity
    lockbox.total_liquidity += position_liquidity;

    Ok(())
  }

  /// Withdraws a specified amount of liquidity for supplied bridged tokens.
  ///
  /// ### Parameters
  /// - `amount` - Amount of bridged tokens corresponding to the position liquidity part to withdraw.
  pub fn withdraw(
    ctx: Context<WithdrawLiquidityForTokens>,
    amount: u64,
  ) -> Result<()> {
    // Get the lockbox state
    let lockbox = &ctx.accounts.lockbox;

    // Check if there is any liquidity left in the Lockbox
    if lockbox.total_liquidity == 0 {
      return Err(ErrorCode::TotalLiquidityZero.into());
    }

    // Get the position liquidity
    let idx = lockbox.position_liquidity.len() - 1;
    let position_liquidity: u64 = lockbox.position_liquidity[idx];

    // Check that the liquidity is not zero - must never happen if the total liquidity is not zero
    if position_liquidity == 0 {
      return Err(ErrorCode::LiquidityZero.into());
    }

    // Check the requested amount to be smaller or equal than the position liquidity
    if amount > position_liquidity {
      return Err(ErrorCode::AmountExceedsPositionLiquidity.into());
    }

    // Check the position address
    let position_account = lockbox.position_accounts[idx];
    if position_account != ctx.accounts.position.key() {
      return Err(ErrorCode::WrongPositionAccount.into());
    }

    // Check the PDA position ATA
    let pda_position_account = lockbox.position_pda_ata[idx];
    if pda_position_account != ctx.accounts.pda_position_account.key() {
      return Err(ErrorCode::WrongPDAPositionAccount.into());
    }

    // Burn provided amount of bridged tokens
    invoke_signed(
      &burn_checked(
        ctx.accounts.token_program.key,
        ctx.accounts.bridged_token_account.to_account_info().key,
        ctx.accounts.bridged_token_mint.to_account_info().key,
        ctx.accounts.signer.to_account_info().key,
        &[],
        amount,
        BRIDGED_TOKEN_DECIMALS,
      )?,
      &[
        ctx.accounts.token_program.to_account_info(),
        ctx.accounts.bridged_token_account.to_account_info(),
        ctx.accounts.bridged_token_mint.to_account_info(),
        ctx.accounts.signer.to_account_info(),
      ],
      &[]
    )?;

    // Get program signer seeds
    let signer_seeds = &[&ctx.accounts.lockbox.seeds()[..]];

    // Update fees for the position
    let cpi_program_update_fees = ctx.accounts.whirlpool_program.to_account_info();
    let cpi_accounts_update_fees = UpdateFeesAndRewards {
      whirlpool: ctx.accounts.whirlpool.to_account_info(),
      position: ctx.accounts.position.to_account_info(),
      tick_array_lower: ctx.accounts.tick_array_lower.to_account_info(),
      tick_array_upper: ctx.accounts.tick_array_upper.to_account_info()
    };

    let cpi_ctx_update_fees = CpiContext::new_with_signer(
      cpi_program_update_fees,
      cpi_accounts_update_fees,
      signer_seeds
    );
    whirlpool::cpi::update_fees_and_rewards(cpi_ctx_update_fees)?;

    // Collect fees from the position
    let cpi_program_collect_fees = ctx.accounts.whirlpool_program.to_account_info();
    let cpi_accounts_collect_fees = CollectFees {
      whirlpool: ctx.accounts.whirlpool.to_account_info(),
      position_authority: ctx.accounts.lockbox.to_account_info(),
      position: ctx.accounts.position.to_account_info(),
      position_token_account: ctx.accounts.pda_position_account.to_account_info(),
      token_owner_account_a: ctx.accounts.token_owner_account_a.to_account_info(),
      token_owner_account_b: ctx.accounts.token_owner_account_b.to_account_info(),
      token_vault_a: ctx.accounts.token_vault_a.to_account_info(),
      token_vault_b: ctx.accounts.token_vault_b.to_account_info(),
      token_program: ctx.accounts.token_program.to_account_info()
    };

    let cpi_ctx_collect_fees = CpiContext::new_with_signer(
      cpi_program_collect_fees,
      cpi_accounts_collect_fees,
      signer_seeds
    );
    whirlpool::cpi::collect_fees(cpi_ctx_collect_fees)?;

    // CPI to decrease liquidity
    // TODO: find out how to keep the same cpi_program variable for all of the calls
    let cpi_program_modify_liquidity = ctx.accounts.whirlpool_program.to_account_info();
    let cpi_accounts_modify_liquidity = ModifyLiquidity {
      whirlpool: ctx.accounts.whirlpool.to_account_info(),
      position: ctx.accounts.position.to_account_info(),
      position_authority: ctx.accounts.lockbox.to_account_info(),
      position_token_account: ctx.accounts.pda_position_account.to_account_info(),
      tick_array_lower: ctx.accounts.tick_array_lower.to_account_info(),
      tick_array_upper: ctx.accounts.tick_array_upper.to_account_info(),
      token_owner_account_a: ctx.accounts.token_owner_account_a.to_account_info(),
      token_owner_account_b: ctx.accounts.token_owner_account_b.to_account_info(),
      token_vault_a: ctx.accounts.token_vault_a.to_account_info(),
      token_vault_b: ctx.accounts.token_vault_b.to_account_info(),
      token_program: ctx.accounts.token_program.to_account_info()
    };

    let cpi_ctx_modify_liquidity = CpiContext::new_with_signer(
      cpi_program_modify_liquidity,
      cpi_accounts_modify_liquidity,
      signer_seeds
    );
    whirlpool::cpi::decrease_liquidity(cpi_ctx_modify_liquidity, amount as u128, 0, 0)?;

    // Update the token remainder
    let remainder: u64 = position_liquidity - amount;

    // If requested amount can be fully covered by the current position liquidity, close the position
    if remainder == 0 {
      // Close the position
      let cpi_program_close_position = ctx.accounts.whirlpool_program.to_account_info();
      let cpi_accounts_close_position = ClosePosition {
        position_authority: ctx.accounts.lockbox.to_account_info(),
        receiver: ctx.accounts.signer.to_account_info(),
        position: ctx.accounts.position.to_account_info(),
        position_mint: ctx.accounts.position_mint.to_account_info(),
        position_token_account: ctx.accounts.pda_position_account.to_account_info(),
        token_program: ctx.accounts.token_program.to_account_info()
      };

      let cpi_ctx_close_position = CpiContext::new_with_signer(
        cpi_program_close_position,
        cpi_accounts_close_position,
        signer_seeds
      );
      whirlpool::cpi::close_position(cpi_ctx_close_position)?;
    }

    // TODO: Check the CEI pattern if it makes sense, as it's not possible to declare the mutable before
    let lockbox_mut = &mut ctx.accounts.lockbox;

    // Check the position remainder
    if remainder == 0 {
      // Pop first queue elements
      lockbox_mut.position_liquidity.pop();
      lockbox_mut.position_accounts.pop();
      lockbox_mut.position_pda_ata.pop();
    } else {
      // Update liquidity and its associated position account
      lockbox_mut.position_liquidity[idx] = remainder;
    }

    // Decrease the total liquidity amount
    lockbox_mut.total_liquidity -= amount;

    Ok(())
  }

  /// Gets a set of position liquidity NFTs and accounts to retrieve OLAS and SOL in exchange of bridged tokens.
  ///
  /// ### Parameters
  /// - `amount` - Bridged token amount to withdraw.
  pub fn get_liquidity_amounts_and_positions(ctx:Context<LiquidityLockboxState>, amount: u64)
    -> Result<AmountsAndPositions>
  {
    let lockbox = &ctx.accounts.lockbox;

    // Check the amount
    if amount > lockbox.total_liquidity {
      return Err(ErrorCode::AmountExceedsTotalLiquidity.into());
    }

    let mut liquidity_sum: u64 = 0;
    let mut num_positions: u32 = 0;
    let mut amount_left: u64 = 0;

    // Get the number of allocated positions in the negative order, starting from the last one
    for position_liquidity in lockbox.position_liquidity.iter().rev() {
      // // Increase a total calculated liquidity and a number of positions to return
      liquidity_sum += position_liquidity;
      num_positions += 1;

      // Check if the accumulated liquidity is enough to cover the requested amount
      if liquidity_sum >= amount {
        amount_left = liquidity_sum - amount;
        break;
      }
    }

    // Allocate the necessary arrays and fill the values
    let mut pos = AmountsAndPositions {
      position_liquidity: Vec::new(),
      position_accounts: Vec::new(),
      position_pda_ata: Vec::new()
    };

    // Get the last array index
    let last = lockbox.position_accounts.len() - 1;
    for i in 0..num_positions as usize {
      let idx = last - i;
      pos.position_accounts.push(lockbox.position_accounts[idx]);
      pos.position_liquidity.push(lockbox.position_liquidity[idx]);
      pos.position_pda_ata.push(lockbox.position_pda_ata[idx]);
    }

    // Adjust the last position, if it was not fully allocated
    if num_positions > 0 && amount_left > 0 {
      let idx: usize = num_positions as usize - 1;
      pos.position_liquidity[idx] = amount_left;
    }

    // Return the tuple wrapped in an Ok variant of Result
    Ok(pos)
  }
}

#[derive(Debug, Clone, AnchorSerialize, AnchorDeserialize)]
pub struct AmountsAndPositions {
  pub position_liquidity: Vec<u64>,
  pub position_accounts: Vec<Pubkey>,
  pub position_pda_ata: Vec<Pubkey>
}

#[derive(Accounts)]
pub struct InitializeLiquidityLockbox<'info> {
  #[account(mut)]
  pub signer: Signer<'info>,

  #[account(init,
    seeds = [
      b"liquidity_lockbox".as_ref()
    ],
    bump,
    payer = signer,
    space = 10000)]
  pub lockbox: Box<Account<'info, LiquidityLockbox>>,

  #[account(address = token::ID)]
  pub token_program: Program<'info, Token>,
  pub system_program: Program<'info, System>,
  pub rent: Sysvar<'info, Rent>
}

#[derive(Accounts)]
pub struct DepositPositionForLiquidity<'info> {
  pub position_authority: Signer<'info>,

  pub position: Box<Account<'info, Position>>,
  #[account(mut,
    constraint = position_token_account.mint == position.position_mint,
    constraint = position_token_account.amount == 1
  )]
  pub position_token_account: Box<Account<'info, TokenAccount>>,

  #[account(mut,
    constraint = pda_position_account.mint == position.position_mint,
    constraint = pda_position_account.amount == 0,
    constraint = pda_position_account.owner == lockbox.key(),
  )]
  pub pda_position_account: Box<Account<'info, TokenAccount>>,

  #[account(mut)]
  pub bridged_token_mint: Account<'info, Mint>,
  #[account(mut, constraint = bridged_token_account.mint == bridged_token_mint.key())]
  pub bridged_token_account: Account<'info, TokenAccount>,

  #[account(mut)]
  pub lockbox: Box<Account<'info, LiquidityLockbox>>,
  #[account(address = token::ID)]
  pub token_program: Program<'info, Token>
}

#[derive(Accounts)]
pub struct WithdrawLiquidityForTokens<'info> {
  #[account(mut)]
  pub whirlpool: Box<Account<'info, Whirlpool>>,

  pub signer: Signer<'info>,

  #[account(mut)]
  pub bridged_token_mint: Box<Account<'info, Mint>>,
  #[account(mut, constraint = bridged_token_account.mint == bridged_token_mint.key())]
  pub bridged_token_account: Box<Account<'info, TokenAccount>>,

  #[account(mut, has_one = whirlpool)]
  pub position: Box<Account<'info, Position>>,
  #[account(mut,
    constraint = pda_position_account.mint == position.position_mint,
    constraint = pda_position_account.amount == 1
  )]
  pub pda_position_account: Box<Account<'info, TokenAccount>>,

  #[account(mut,
    address = position.position_mint,
    constraint = position.whirlpool == whirlpool.key()
  )]
  pub position_mint: Box<Account<'info, Mint>>,

  #[account(mut, constraint = token_owner_account_a.mint == whirlpool.token_mint_a)]
  pub token_owner_account_a: Box<Account<'info, TokenAccount>>,
  #[account(mut, constraint = token_owner_account_b.mint == whirlpool.token_mint_b)]
  pub token_owner_account_b: Box<Account<'info, TokenAccount>>,

  #[account(mut, constraint = token_vault_a.key() == whirlpool.token_vault_a)]
  pub token_vault_a: Box<Account<'info, TokenAccount>>,
  #[account(mut, constraint = token_vault_b.key() == whirlpool.token_vault_b)]
  pub token_vault_b: Box<Account<'info, TokenAccount>>,

  #[account(mut, has_one = whirlpool)]
  pub tick_array_lower: AccountLoader<'info, TickArray>,
  #[account(mut, has_one = whirlpool)]
  pub tick_array_upper: AccountLoader<'info, TickArray>,

  #[account(mut)]
  pub lockbox: Box<Account<'info, LiquidityLockbox>>,
  pub whirlpool_program: Program<'info, whirlpool::program::Whirlpool>,

  #[account(address = token::ID)]
  pub token_program: Program<'info, Token>
}

#[derive(Accounts)]
pub struct LiquidityLockboxState<'info> {
  pub lockbox: Box<Account<'info, LiquidityLockbox>>
}


#[error_code]
pub enum ErrorCode {
  #[msg("Liquidity value overflow")]
  LiquidityOverflow,
  #[msg("Wrong whirlpool address")]
  WrongWhirlpool,
  #[msg("Liquidity is zero")]
  LiquidityZero,
  #[msg("Total liquidity is zero")]
  TotalLiquidityZero,
  #[msg("Requested amount exceeds a position liquidity")]
  AmountExceedsPositionLiquidity,
  #[msg("Requested amount exceeds total liquidity")]
  AmountExceedsTotalLiquidity,
  #[msg("Tick out of range")]
  OutOfRange,
  #[msg("Wrong account owner")]
  WrongOwner,
  #[msg("Provided wrong position PDA")]
  WrongPositionPDA,
  #[msg("Provided wrong position ATA")]
  WrongPositionAccount,
  #[msg("Provided wrong PDA position ATA")]
  WrongPDAPositionAccount
}
