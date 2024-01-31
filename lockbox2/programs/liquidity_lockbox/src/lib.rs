pub mod state;
use anchor_lang::prelude::*;
use anchor_spl::token::{self, Mint, Token, TokenAccount, Approve};
use whirlpool::{
  self,
  state::{Whirlpool, TickArray, Position},
  cpi::accounts::ModifyLiquidity,
  cpi::accounts::UpdateFeesAndRewards,
  cpi::accounts::CollectFees
};
use solana_program::{pubkey::Pubkey, program::invoke_signed};
use spl_token::instruction::{burn_checked, mint_to};
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
  // SOL address
  const SOL: Pubkey = pubkey!("So11111111111111111111111111111111111111112");
  // OLAS address
  const OLAS: Pubkey = pubkey!("Ez3nzG9ofodYCvEmw73XhQ87LWNYVRM2s7diB5tBZPyM");
  // Position account discriminator
  const POSITION_HEADER: [u8; 8] = [0xaa, 0xbc, 0x8f, 0xe4, 0x7a, 0x40, 0xf7, 0xd0];
  // Full range lower and upper indexes
  const TICK_LOWER_INDEX: i32 = -443584;
  const TICK_UPPER_INDEX: i32 = 443584;
  // Bridged token decimals
  const BRIDGED_TOKEN_DECIMALS: u8 = 8;


  /// Initializes a Lockbox account that stores state data.
  pub fn initialize(
    ctx: Context<InitializeLiquidityLockbox>
  ) -> Result<()> {
    let whirlpool = ctx.accounts.position.whirlpool;
    let position_mint = ctx.accounts.position.position_mint;
    let liquidity = ctx.accounts.position.liquidity;
    let tick_lower_index = ctx.accounts.position.tick_lower_index;
    let tick_upper_index = ctx.accounts.position.tick_upper_index;

    // Check the whirlpool
    if whirlpool != WHIRLPOOL {
        return Err(ErrorCode::WrongWhirlpool.into());
    }

    // Check the discriminator
    let account = &ctx.accounts.position.to_account_info();

    let data = account.try_borrow_data()?;
    if data.len() != Position::LEN {
        return Err(ErrorCode::WrongPositionHeader.into());
    }

    let mut discriminator = [0u8; 8];
    discriminator.copy_from_slice(&data[0..8]);
    if discriminator != POSITION_HEADER {
        return Err(ErrorCode::WrongPositionHeader.into());
    }

    // Check for the minimum liquidity in position
    if liquidity != 0 {
      return Err(ErrorCode::LiquidityNotZero.into());
    }

    // Check tick values
    if tick_lower_index != TICK_LOWER_INDEX || tick_upper_index != TICK_UPPER_INDEX {
      return Err(ErrorCode::OutOfRange.into());
    }

    // Check the PDA ownership
    if ctx.accounts.position.to_account_info().owner != &ORCA {
      return Err(ErrorCode::WrongOwner.into());
    }

    // Check the position PDA address correctness
    let position_pda = Pubkey::find_program_address(&[b"position", position_mint.as_ref()], &ORCA);
    if position_pda.0 != ctx.accounts.position.key() {
      return Err(ErrorCode::WrongPositionPDA.into());
    }

    // Check that the first token mint is SOL
    if ctx.accounts.fee_collector_token_owner_account_a.mint != SOL {
      return Err(ErrorCode::WrongTokenMint.into());
    }

    // Check that the second token mint is OLAS
    if ctx.accounts.fee_collector_token_owner_account_b.mint != OLAS {
      return Err(ErrorCode::WrongTokenMint.into());
    }

    // Get the lockbox account
    let lockbox = &mut ctx.accounts.lockbox;

    // Get the anchor-derived bump
    let bump = *ctx.bumps.get("lockbox").unwrap();

    // Initialize lockbox account
    lockbox.initialize(
      bump,
      ctx.accounts.bridged_token_mint.key(),
      ctx.accounts.fee_collector_token_owner_account_a.key(),
      ctx.accounts.fee_collector_token_owner_account_b.key(),
      ctx.accounts.position.key(),
      ctx.accounts.pda_position_account.key()
    )?;

    Ok(())
  }

  /// Deposits an NFT position under the Lockbox management and gets bridged tokens minted in return.
  ///
  /// ### Parameters
  /// - `liquidity_amount` - Requested liquidity amount.
  /// - `token_max_a` - Max amount of SOL token to be added for liquidity.
  /// - `token_max_b` - Max amount of OLAS token to be added for liquidity.
  pub fn deposit(ctx: Context<DepositPositionForLiquidity>,
    liquidity_amount: u64,
    token_max_a: u64,
    token_max_b: u64,
  ) -> Result<()> {
    // Check the initial token amounts
    if token_max_a == 0 || token_max_b == 0 {
      return Err(ErrorCode::LiquidityZero.into());
    }

    // Check the position account
    if ctx.accounts.position.key() != ctx.accounts.lockbox.position {
      return Err(ErrorCode::WrongPositionPDA.into());
    }

    // Check the position PDA address correctness
    let position_pda = Pubkey::find_program_address(&[b"position", ctx.accounts.position.position_mint.as_ref()], &ORCA);
    if position_pda.0 != ctx.accounts.position.key() {
      return Err(ErrorCode::WrongPositionPDA.into());
    }

    // Check the lockbox PDA address correctness
    let lockbox_pda = Pubkey::find_program_address(&[b"liquidity_lockbox"], &ID);
    if lockbox_pda.0 != ctx.accounts.lockbox.key() {
      return Err(ErrorCode::WrongLockboxPDA.into());
    }

    // Check the whirlpool
    if ctx.accounts.whirlpool.key() != WHIRLPOOL {
        return Err(ErrorCode::WrongWhirlpool.into());
    }

    // Check the Orca Whirlpool program address
    if ctx.accounts.whirlpool_program.key() != ORCA {
        return Err(ErrorCode::WrongOrcaAccount.into());
    }

    // Check that the first token mint is SOL
    if ctx.accounts.token_owner_account_a.mint != SOL || ctx.accounts.token_vault_a.mint != SOL {
      return Err(ErrorCode::WrongTokenMint.into());
    }

    // Check that the second token mint is OLAS
    if ctx.accounts.token_owner_account_b.mint != OLAS || ctx.accounts.token_vault_b.mint != OLAS {
      return Err(ErrorCode::WrongTokenMint.into());
    }

    // Calculate token deltas
    let tick_index_lower = ctx.accounts.position.tick_lower_index;
    let tick_index_upper = ctx.accounts.position.tick_upper_index;
    let tick_index_current = ctx.accounts.whirlpool.tick_current_index;

    // assuming InRange status
    if tick_index_current < tick_index_lower || tick_index_upper <= tick_index_current {
      return Err(ErrorCode::OutOfRange.into());
    }

    // Total liquidity update with the check
    ctx.accounts.lockbox.total_liquidity = match ctx.accounts.lockbox
      .total_liquidity
      .checked_add(liquidity_amount) {
        Some(new_liquidity) => new_liquidity,
        None => return Err(ErrorCode::LiquidityOverflow.into()),
      };

    // Approve SOL tokens for the lockbox
    token::approve(
      CpiContext::new(
        ctx.accounts.token_program.to_account_info(),
        Approve {
          to: ctx.accounts.token_owner_account_a.to_account_info(),
          delegate: ctx.accounts.lockbox.to_account_info(),
          authority: ctx.accounts.signer.to_account_info(),
        },
      ),
      token_max_a,
    )?;

    // Approve OLAS tokens for the lockbox
    token::approve(
      CpiContext::new(
        ctx.accounts.token_program.to_account_info(),
        Approve {
          to: ctx.accounts.token_owner_account_b.to_account_info(),
          delegate: ctx.accounts.lockbox.to_account_info(),
          authority: ctx.accounts.signer.to_account_info(),
        },
      ),
      token_max_b,
    )?;

    // Get lockbox signer seeds
    let signer_seeds = &[&ctx.accounts.lockbox.seeds()[..]];

    // CPI call to increase liquidity
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
      token_program: ctx.accounts.token_program.to_account_info(),
    };

    let cpi_ctx_modify_liquidity = CpiContext::new_with_signer(
      cpi_program_modify_liquidity,
      cpi_accounts_modify_liquidity,
      signer_seeds
    );
    whirlpool::cpi::increase_liquidity(cpi_ctx_modify_liquidity, liquidity_amount as u128, token_max_a, token_max_b)?;

    // Mint bridged tokens in the amount of position liquidity
    invoke_signed(
      &mint_to(
        ctx.accounts.token_program.key,
        ctx.accounts.bridged_token_mint.to_account_info().key,
        ctx.accounts.bridged_token_account.to_account_info().key,
        ctx.accounts.lockbox.to_account_info().key,
        &[ctx.accounts.lockbox.to_account_info().key],
        liquidity_amount,
      )?,
      &[
        ctx.accounts.bridged_token_mint.to_account_info(),
        ctx.accounts.bridged_token_account.to_account_info(),
        ctx.accounts.lockbox.to_account_info(),
        ctx.accounts.token_program.to_account_info(),
      ],
      &[&ctx.accounts.lockbox.seeds()],
    )?;

    // Revoke approval for unused SOL tokens
    token::approve(
      CpiContext::new(
        ctx.accounts.token_program.to_account_info(),
        Approve {
          to: ctx.accounts.token_owner_account_a.to_account_info(),
          delegate: ctx.accounts.lockbox.to_account_info(),
          authority: ctx.accounts.signer.to_account_info(),
        },
      ),
      0,
    )?;

    // Revoke approval for OLAS tokens
    token::approve(
      CpiContext::new(
        ctx.accounts.token_program.to_account_info(),
        Approve {
          to: ctx.accounts.token_owner_account_b.to_account_info(),
          delegate: ctx.accounts.lockbox.to_account_info(),
          authority: ctx.accounts.signer.to_account_info(),
        },
      ),
      0,
    )?;

    emit!(DepositEvent {
      signer: ctx.accounts.signer.key(),
      position: ctx.accounts.position.key(),
      deposit_liquidity: liquidity_amount,
      total_liquidity: ctx.accounts.lockbox.total_liquidity
    });

    Ok(())
  }

  /// Withdraws a specified amount of liquidity for supplied bridged tokens.
  ///
  /// ### Parameters
  /// - `amount` - Amount of bridged tokens corresponding to the position liquidity amount to withdraw.
  /// - `token_min_a` - The minimum amount of SOL the signer is willing to withdraw.
  /// - `token_min_b` - The minimum amount of OLAS the signer is willing to withdraw.
  pub fn withdraw(
    ctx: Context<WithdrawLiquidityForTokens>,
    amount: u64,
    token_min_a: u64,
    token_min_b: u64
  ) -> Result<()> {
    // Check if there is any liquidity left in the Lockbox
    if ctx.accounts.position.liquidity == 0 {
      return Err(ErrorCode::LiquidityZero.into());
    }

    // Check the token amount
    if amount == 0 {
      return Err(ErrorCode::LiquidityZero.into());
    }

    // Check the lockbox PDA address correctness
    let lockbox_pda = Pubkey::find_program_address(&[b"liquidity_lockbox"], &ID);
    if lockbox_pda.0 != ctx.accounts.lockbox.key() {
      return Err(ErrorCode::WrongLockboxPDA.into());
    }

    // Check the Orca Whirlpool program address
    if ctx.accounts.whirlpool_program.key() != ORCA {
        return Err(ErrorCode::WrongOrcaAccount.into());
    }

    // Check that the first token mint is SOL
    if ctx.accounts.token_owner_account_a.mint != SOL || ctx.accounts.token_vault_a.mint != SOL {
      return Err(ErrorCode::WrongTokenMint.into());
    }

    // Check that the second token mint is OLAS
    if ctx.accounts.token_owner_account_b.mint != OLAS || ctx.accounts.token_vault_b.mint != OLAS {
      return Err(ErrorCode::WrongTokenMint.into());
    }

    // Check the requested amount to be smaller or equal than the position liquidity
    if amount > ctx.accounts.position.liquidity as u64 {
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
      token_owner_account_a: ctx.accounts.fee_collector_token_owner_account_a.to_account_info(),
      token_owner_account_b: ctx.accounts.fee_collector_token_owner_account_b.to_account_info(),
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
    whirlpool::cpi::decrease_liquidity(cpi_ctx_modify_liquidity, amount as u128, token_min_a, token_min_b)?;

    // Update the position liquidity
    ctx.accounts.lockbox.total_liquidity = match ctx.accounts.lockbox
      .total_liquidity
      .checked_sub(amount) {
        Some(new_liquidity) => new_liquidity,
        None => return Err(ErrorCode::LiquidityUnderflow.into()),
      };

    emit!(WithdrawEvent {
      signer: ctx.accounts.signer.key(),
      position: ctx.accounts.position.key(),
      token_owner_account_a: ctx.accounts.token_owner_account_a.key(),
      token_owner_account_b: ctx.accounts.token_owner_account_b.key(),
      withdraw_liquidity: amount,
      total_liquidity: ctx.accounts.lockbox.total_liquidity
    });

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

  #[account(constraint = bridged_token_mint.mint_authority.unwrap() == lockbox.key())]
  pub bridged_token_mint: Box<Account<'info, Mint>>,

  #[account(constraint = signer.key == &fee_collector_token_owner_account_a.owner,
    constraint = fee_collector_token_owner_account_a.key() != fee_collector_token_owner_account_b.key()
  )]
  pub fee_collector_token_owner_account_a: Box<Account<'info, TokenAccount>>,
  #[account(constraint = signer.key == &fee_collector_token_owner_account_b.owner)]
  pub fee_collector_token_owner_account_b: Box<Account<'info, TokenAccount>>,

  #[account(has_one = whirlpool)]
  pub position: Box<Account<'info, Position>>,

  #[account(mut,
    constraint = pda_position_account.mint == position.position_mint,
    constraint = pda_position_account.amount == 1,
    constraint = lockbox.key() == pda_position_account.owner
  )]
  pub pda_position_account: Box<Account<'info, TokenAccount>>,

  pub whirlpool: Box<Account<'info, Whirlpool>>,

  #[account(address = token::ID)]
  pub token_program: Program<'info, Token>,
  pub system_program: Program<'info, System>,
  pub rent: Sysvar<'info, Rent>
}

#[derive(Accounts)]
pub struct DepositPositionForLiquidity<'info> {
  #[account(mut)]
  pub signer: Signer<'info>,

  #[account(mut, address = lockbox.position, has_one = whirlpool)]
  pub position: Box<Account<'info, Position>>,

  #[account(mut,
    address = lockbox.pda_position_account.key(),
    constraint = lockbox.key() == pda_position_account.owner,
    constraint = pda_position_account.mint == position.position_mint,
    constraint = pda_position_account.amount == 1
  )]
  pub pda_position_account: Box<Account<'info, TokenAccount>>,

  #[account(mut, address = position.whirlpool)]
  pub whirlpool: Box<Account<'info, Whirlpool>>,

  #[account(mut,
    constraint = token_owner_account_a.mint == whirlpool.token_mint_a,
    constraint = token_owner_account_a.mint != token_owner_account_b.mint,
    constraint = signer.key == &token_owner_account_a.owner
  )]
  pub token_owner_account_a: Box<Account<'info, TokenAccount>>,
  #[account(mut,
    constraint = token_owner_account_b.mint == whirlpool.token_mint_b,
    constraint = signer.key == &token_owner_account_b.owner
  )]
  pub token_owner_account_b: Box<Account<'info, TokenAccount>>,

  #[account(mut,
    constraint = token_vault_a.key() == whirlpool.token_vault_a,
    constraint = token_vault_a.key() != token_vault_b.key()
  )]
  pub token_vault_a: Box<Account<'info, TokenAccount>>,
  #[account(mut, constraint = token_vault_b.key() == whirlpool.token_vault_b)]
  pub token_vault_b: Box<Account<'info, TokenAccount>>,

  #[account(mut, has_one = whirlpool,
    constraint = tick_array_lower.key() != tick_array_upper.key(),
    constraint = tick_array_lower.to_account_info().owner == &whirlpool_program.key()
  )]
  pub tick_array_lower: AccountLoader<'info, TickArray>,
  #[account(mut, has_one = whirlpool,
    constraint = tick_array_upper.to_account_info().owner == &whirlpool_program.key()
  )]
  pub tick_array_upper: AccountLoader<'info, TickArray>,

  #[account(mut, address = lockbox.bridged_token_mint)]
  pub bridged_token_mint: Box<Account<'info, Mint>>,
  #[account(mut,
    constraint = bridged_token_account.mint == lockbox.bridged_token_mint,
    constraint = bridged_token_account.mint == bridged_token_mint.key(),
    constraint = signer.key == &bridged_token_account.owner,
  )]
  pub bridged_token_account: Box<Account<'info, TokenAccount>>,

  #[account(mut)]
  pub lockbox: Box<Account<'info, LiquidityLockbox>>,
  pub whirlpool_program: Program<'info, whirlpool::program::Whirlpool>,

  #[account(address = token::ID)]
  pub token_program: Program<'info, Token>
}

#[derive(Accounts)]
pub struct WithdrawLiquidityForTokens<'info> {
  #[account(mut, address = position.whirlpool)]
  pub whirlpool: Box<Account<'info, Whirlpool>>,

  pub signer: Signer<'info>,

  #[account(mut, address = lockbox.bridged_token_mint)]
  pub bridged_token_mint: Box<Account<'info, Mint>>,
  #[account(mut,
    constraint = bridged_token_account.mint == lockbox.bridged_token_mint,
    constraint = bridged_token_account.mint == bridged_token_mint.key(),
    constraint = signer.key == &bridged_token_account.owner,
  )]
  pub bridged_token_account: Box<Account<'info, TokenAccount>>,

  #[account(mut, address = lockbox.position, has_one = whirlpool, has_one = position_mint)]
  pub position: Box<Account<'info, Position>>,
  #[account(mut,
    address = lockbox.pda_position_account.key(),
    constraint = pda_position_account.mint == position.position_mint,
    constraint = pda_position_account.amount == 1,
    constraint = lockbox.key() == pda_position_account.owner
  )]
  pub pda_position_account: Box<Account<'info, TokenAccount>>,

  #[account(mut,
    address = position.position_mint,
    constraint = position_mint.supply == 1
  )]
  pub position_mint: Box<Account<'info, Mint>>,

  #[account(mut,
    constraint = token_owner_account_a.mint == whirlpool.token_mint_a,
    constraint = token_owner_account_a.mint != token_owner_account_b.mint,
    constraint = signer.key == &token_owner_account_a.owner
  )]
  pub token_owner_account_a: Box<Account<'info, TokenAccount>>,
  #[account(mut,
    constraint = token_owner_account_b.mint == whirlpool.token_mint_b,
    constraint = signer.key == &token_owner_account_b.owner
  )]
  pub token_owner_account_b: Box<Account<'info, TokenAccount>>,

  #[account(mut, address = lockbox.fee_collector_token_owner_account_a)]
  pub fee_collector_token_owner_account_a: Box<Account<'info, TokenAccount>>,
  #[account(mut, address = lockbox.fee_collector_token_owner_account_b)]
  pub fee_collector_token_owner_account_b: Box<Account<'info, TokenAccount>>,

  #[account(mut,
    constraint = token_vault_a.key() == whirlpool.token_vault_a,
    constraint = token_vault_a.key() != token_vault_b.key()
  )]
  pub token_vault_a: Box<Account<'info, TokenAccount>>,
  #[account(mut, constraint = token_vault_b.key() == whirlpool.token_vault_b)]
  pub token_vault_b: Box<Account<'info, TokenAccount>>,

  #[account(mut, has_one = whirlpool,
    constraint = tick_array_lower.key() != tick_array_upper.key(),
    constraint = tick_array_lower.to_account_info().owner == &whirlpool_program.key()
  )]
  pub tick_array_lower: AccountLoader<'info, TickArray>,
  #[account(mut, has_one = whirlpool,
    constraint = tick_array_upper.to_account_info().owner == &whirlpool_program.key()
  )]
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
  #[msg("Liquidity value underflow")]
  LiquidityUnderflow,
  #[msg("Wrong whirlpool address")]
  WrongWhirlpool,
  #[msg("Wrong position PDA header")]
  WrongPositionHeader,
  #[msg("Wrong position ID")]
  WrongPositionId,
  #[msg("Liquidity is zero")]
  LiquidityZero,
  #[msg("Liquidity is not zero")]
  LiquidityNotZero,
  #[msg("Delta token amount bigger than the max allowed one")]
  DeltaAmountOverflow,
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
  #[msg("Provided wrong lockbox PDA")]
  WrongLockboxPDA,
  #[msg("Provided wrong position ATA")]
  WrongPositionAccount,
  #[msg("Provided wrong PDA position ATA")]
  WrongPDAPositionAccount,
  #[msg("Provided wrong Orca program account")]
  WrongOrcaAccount,
  #[msg("Wrong token mint")]
  WrongTokenMint,
  #[msg("Whirlpool number downcast")]
  WhirlpoolNumberDownCastError
}


#[event]
pub struct DepositEvent {
    // Signer (user)
    #[index]
    pub signer: Pubkey,
    // Liquidity position
    #[index]
    pub position: Pubkey,
    // Deposit liquidity amount
    pub deposit_liquidity: u64,
    // Total position liquidity
    pub total_liquidity: u64
}

#[event]
pub struct WithdrawEvent {
    // Signer (user)
    #[index]
    pub signer: Pubkey,
    // Liquidity position
    #[index]
    pub position: Pubkey,
    // User ATA token A
    token_owner_account_a: Pubkey,
    // User ATA token B
    token_owner_account_b: Pubkey,
    // Withdraw liquidity amount
    pub withdraw_liquidity: u64,
    // Total position liquidity
    pub total_liquidity: u64
}
