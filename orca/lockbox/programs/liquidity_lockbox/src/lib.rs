pub mod state;
use anchor_lang::prelude::*;
use anchor_spl::token::{self, Mint, Token, TokenAccount, Transfer};
use anchor_spl::associated_token::AssociatedToken;
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

  // Program Id
  const PROGRAM_ID: Pubkey = pubkey!("7ahQGWysExobjeZ91RTsNqTCN3kWyHGZ43ud2vB7VVoZ");
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
  /// - `bridged_token_mint` - Bridged token mint for tokens issued in return for the position liquidity NFT.
  pub fn initialize(
    ctx: Context<InitializeLiquidityLockbox>,
    bridged_token_mint: Pubkey
  ) -> Result<()> {
    // Get the lockbox account
    let lockbox = &mut ctx.accounts.lockbox;

    // Get the anchor-derived bump
    let bump = *ctx.bumps.get("lockbox").unwrap();

    // Initialize lockbox account
    Ok(lockbox.initialize(
      bump,
      bridged_token_mint
    )?)
  }

  /// Deposits an NFT position under the Lockbox management and gets bridged tokens minted in return.
  pub fn deposit(ctx: Context<DepositPositionForLiquidity>, id: u32) -> Result<()> {
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

    // Check the id that has to match the number of positions in order to create a correct account
    // The position needs to be provided as an argument since it's passed into the instruction field
    let num_positions = ctx.accounts.lockbox.num_positions;
    if num_positions != id {
      return Err(ErrorCode::WrongPositionId.into());
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
    let pda_lockbox_position = &mut ctx.accounts.pda_lockbox_position;
    pda_lockbox_position.initialize(
      id,
      *ctx.bumps.get("pda_lockbox_position").unwrap(),
      position_liquidity,
      ctx.accounts.position.key(),
      ctx.accounts.pda_position_account.key()
    )?;

    // Increase the amount of total bridged token liquidity and the number of position accounts
    ctx.accounts.lockbox.total_liquidity += position_liquidity;
    ctx.accounts.lockbox.num_positions += 1;

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
    // Check if there is any liquidity left in the Lockbox
    if ctx.accounts.lockbox.total_liquidity == 0 {
      return Err(ErrorCode::TotalLiquidityZero.into());
    }

    // TODO: any other way to get PROGRAM_ID?
    // Get the lockbox position PDA ATA
    let id = ctx.accounts.lockbox.num_positions - 1;
    let lockbox_position = Pubkey::try_find_program_address(&[b"lockbox_position", id.to_be_bytes().as_ref()], &PROGRAM_ID);
    let lockbox_position_pubkey = lockbox_position.map(|(pubkey, _)| pubkey);

    // Check that the calculated address matches the provided PDA lockbox position
    if lockbox_position_pubkey.unwrap() != ctx.accounts.pda_lockbox_position.key() {
      return Err(ErrorCode::WrongPDAPositionAccount.into());
    }

    // Get the position liquidity
    let position_liquidity = ctx.accounts.pda_lockbox_position.position_liquidity;

    // Check that the liquidity is not zero - must never happen if the total liquidity is not zero
    if position_liquidity == 0 {
      return Err(ErrorCode::LiquidityZero.into());
    }

    // Check the requested amount to be smaller or equal than the position liquidity
    if amount > position_liquidity {
      return Err(ErrorCode::AmountExceedsPositionLiquidity.into());
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

      // TODO: Close the pda_lockbox_position account
    }

    // TODO: Check the CEI pattern if it makes sense, as it's not possible to declare the mutable before

    // Check the position remainder
    if remainder == 0 {
      // Decrease lockbox position counter
      ctx.accounts.lockbox.num_positions -= 1;
    } else {
      // Update position liquidity
      ctx.accounts.pda_lockbox_position.position_liquidity = remainder;
    }

    // Decrease the total liquidity amount
    ctx.accounts.lockbox.total_liquidity -= amount;

    Ok(())
  }
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
    space = LiquidityLockbox::LEN)]
  pub lockbox: Box<Account<'info, LiquidityLockbox>>,

  #[account(address = token::ID)]
  pub token_program: Program<'info, Token>,
  pub system_program: Program<'info, System>,
  pub rent: Sysvar<'info, Rent>
}

#[derive(Accounts)]
#[instruction(id: u32)]
pub struct DepositPositionForLiquidity<'info> {
  #[account(mut)]
  pub position_authority: Signer<'info>,

  pub position: Box<Account<'info, Position>>,
  #[account(mut,
    constraint = position_token_account.mint == position.position_mint,
    constraint = position_token_account.amount == 1
  )]
  pub position_token_account: Box<Account<'info, TokenAccount>>,

  #[account(address = position_token_account.mint)]
  pub position_mint: Account<'info, Mint>,

  #[account(init,
    associated_token::authority = lockbox,
    associated_token::mint = position_mint,
    payer = position_authority)]
  pub pda_position_account: Box<Account<'info, TokenAccount>>,

  #[account(init,
    seeds = [
      b"lockbox_position".as_ref(),
      id.to_be_bytes().as_ref()
    ],
    bump,
    space = LockboxPosition::LEN,
    payer = position_authority)]
  pub pda_lockbox_position: Box<Account<'info, LockboxPosition>>,

  #[account(mut)]
  pub bridged_token_mint: Box<Account<'info, Mint>>,
  #[account(mut, constraint = bridged_token_account.mint == bridged_token_mint.key())]
  pub bridged_token_account: Box<Account<'info, TokenAccount>>,

  #[account(mut)]
  pub lockbox: Box<Account<'info, LiquidityLockbox>>,
  #[account(address = token::ID)]
  pub token_program: Program<'info, Token>,

  pub system_program: Program<'info, System>,
  pub rent: Sysvar<'info, Rent>,
  pub associated_token_program: Program<'info, AssociatedToken>
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

  #[account(mut,
    constraint = pda_lockbox_position.position_account == position.key(),
    constraint = pda_lockbox_position.position_pda_ata == pda_position_account.key(),
    constraint = pda_lockbox_position.to_account_info().owner == lockbox.to_account_info().owner
  )]
  pub pda_lockbox_position: Box<Account<'info, LockboxPosition>>,

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


#[error_code]
pub enum ErrorCode {
  #[msg("Liquidity value overflow")]
  LiquidityOverflow,
  #[msg("Wrong whirlpool address")]
  WrongWhirlpool,
  #[msg("Wrong position ID")]
  WrongPositionId,
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
