pub mod state;
use anchor_lang::prelude::*;
use anchor_spl::token::{self, Mint, Token, TokenAccount, Transfer};
use whirlpool::{
  self,
  state::{Whirlpool, TickArray, Position},
  cpi::accounts::ModifyLiquidity,
  math::sqrt_price_from_tick_index,
  math::{mul_u256, U256Muldiv},
  manager::liquidity_manager::calculate_liquidity_token_deltas,
};
use solana_program::{pubkey::Pubkey, program::invoke_signed};
use spl_token::instruction::{burn_checked, close_account, mint_to};
pub use state::*;
//use crate::{LiquidityLockbox, LockboxBumps};

declare_id!("7ahQGWysExobjeZ91RTsNqTCN3kWyHGZ43ud2vB7VVoZ");

#[program]
pub mod liquidity_lockbox {
  use super::*;
  use solana_program::pubkey;

  // Orca Whirlpool program address
  const ORCA: Pubkey = pubkey!("whirLbMiicVdio4qvUfM5KAg6Ct8VwpYzGff3uctyCc");
  // PDA header for position account
  const PDA_HEADER: u64 = 0xd0f7407ae48fbcaa;
  // TODO: figure out the program seed
  // Program PDA seed
  //const PDA_PROGRAM_SEED: String = String { str: "pdaProgram".into() };
  // TODO: make the pool a constant variable
  //const WHIRLPOOL: Pubkey = pubkey!("");
  // Full range lower and upper indexes
  const TICK_LOWER_INDEX: i32 = -443632;
  const TICK_UPPER_INDEX: i32 = 443632;

  pub fn initialize(
    ctx: Context<InitializeLiquidityLockbox>,
    _bumps: LockboxBumps,
    whirlpool: Pubkey
  ) -> Result<()> {
    let bridged_token_mint = ctx.accounts.bridged_token_mint.key();
    let lockbox = &mut ctx.accounts.lockbox;
    let bump = *ctx.bumps.get("liquidity_lockbox").unwrap();

    Ok(lockbox.initialize(
      bump,
      whirlpool,
      bridged_token_mint,
      ctx.accounts.pda_bridged_token_account.key()
    )?)
  }

  pub fn deposit(ctx: Context<DepositPositionForLiquidity>) -> Result<()> {
    let whirlpool = ctx.accounts.position.whirlpool;
    let position_mint = ctx.accounts.position.position_mint;
    let liquidity = ctx.accounts.position.liquidity;

    // Check for the zero liquidity in position
    if liquidity == 0 {
      return Err(ErrorCode::LiquidityZero.into());
    }
    // Check that the liquidity is within uint64 bounds
    if liquidity > std::u64::MAX as u128 {
      return Err(ErrorCode::Overflow.into());
    }

    let tick_lower_index = ctx.accounts.position.tick_lower_index;
    let tick_upper_index = ctx.accounts.position.tick_upper_index;

    // Transfer position
    let position_token_account = ctx.accounts.position_token_account.to_account_info().key();

    // Mint bridged tokens
    invoke_signed(
      &mint_to(
        ctx.accounts.token_program.key,
        ctx.accounts.bridged_token_mint.to_account_info().key,
        ctx.accounts.bridged_token_account.to_account_info().key,
        ctx.accounts.lockbox.to_account_info().key,
        &[ctx.accounts.lockbox.to_account_info().key],
        liquidity as u64,
      )?,
      &[
        ctx.accounts.bridged_token_mint.to_account_info(),
        ctx.accounts.bridged_token_account.to_account_info(),
        ctx.accounts.lockbox.to_account_info(),
        ctx.accounts.token_program.to_account_info(),
      ],
      &[&ctx.accounts.lockbox.seeds()],
    )?;

    Ok(())
  }

  pub fn decrease_liquidity(
    ctx: Context<DelegatedModifyLiquidity>,
    liquidity: u128,
    token_min_a: u64,
    token_min_b: u64,
  ) -> Result<()> {

    msg!("begin");

    // CPI
    let cpi_program = ctx.accounts.whirlpool_program.to_account_info();
    msg!("after cpi_program");
    let cpi_accounts = ModifyLiquidity {
      whirlpool: ctx.accounts.whirlpool.to_account_info(),
      position: ctx.accounts.position.to_account_info(),
      position_authority: ctx.accounts.position_authority.to_account_info(),
      position_token_account: ctx.accounts.position_token_account.to_account_info(),
      tick_array_lower: ctx.accounts.tick_array_lower.to_account_info(),
      tick_array_upper: ctx.accounts.tick_array_upper.to_account_info(),
      token_owner_account_a: ctx.accounts.token_owner_account_a.to_account_info(),
      token_owner_account_b: ctx.accounts.token_owner_account_b.to_account_info(),
      token_vault_a: ctx.accounts.token_vault_a.to_account_info(),
      token_vault_b: ctx.accounts.token_vault_b.to_account_info(),
      token_program: ctx.accounts.token_program.to_account_info(),
    };
    msg!("after cpi_accounts");
    let cpi_ctx = CpiContext::new(
      cpi_program,
      cpi_accounts,
    );
    msg!("before CPI");
    whirlpool::cpi::decrease_liquidity(cpi_ctx, liquidity, token_min_a, token_min_b)?;

    Ok(())
  }
}


#[derive(Accounts)]
#[instruction(bumps: LockboxBumps)]
pub struct InitializeLiquidityLockbox<'info> {
  #[account(mut)]
  pub signer: Signer<'info>, //signer must sign the transaction to create accounts

  pub bridged_token_mint: Account<'info, Mint>,

  #[account(init,
    seeds = [
      b"liquidity_lockbox".as_ref(),
      bridged_token_mint.key().as_ref(),
      pda_bridged_token_account.key().as_ref()
    ],
    bump,
    payer = signer,
    space = 10000)]
  pub lockbox: Box<Account<'info, LiquidityLockbox>>,

  #[account(init,
    payer = signer,
    token::mint = bridged_token_mint,
    token::authority = lockbox)]
  pub pda_bridged_token_account: Box<Account<'info, TokenAccount>>,

  #[account(address = token::ID)]
  pub token_program: Program<'info, Token>,
  pub system_program: Program<'info, System>,
  pub rent: Sysvar<'info, Rent>
}

// @mutableAccount(position_token_account userPositionAccount)
// @mutableAccount(pda_position_account pdaPositionAccount)
// @mutableAccount(bridged_token_account userBridgedTokenAccount)
// @mutableAccount(bridged_token_mint bridgedTokenMint)
// @account(position)
// @signer(position_authority userWallet)

#[derive(Accounts)]
pub struct DepositPositionForLiquidity<'info> {
  pub position_authority: Signer<'info>,

  pub position: Account<'info, Position>,
  #[account(mut,
    constraint = position_token_account.mint == position.position_mint,
    constraint = position_token_account.amount == 1
  )]
  pub position_token_account: Account<'info, TokenAccount>,

  #[account(mut,
    constraint = pda_position_account.mint == position.position_mint,
    constraint = pda_position_account.amount == 0
  )]
  pub pda_position_account: Box<Account<'info, TokenAccount>>,

  #[account(mut)]
  pub bridged_token_mint: Account<'info, Mint>,
  #[account(mut, constraint = bridged_token_account.mint == bridged_token_mint.key())]
  pub bridged_token_account: Account<'info, TokenAccount>,

  pub lockbox: Account<'info, LiquidityLockbox>,
  pub token_program: Program<'info, Token>
}

#[derive(Accounts)]
pub struct DelegatedModifyLiquidity<'info> {
  #[account(mut)]
  pub whirlpool: Account<'info, Whirlpool>,

  pub position_authority: Signer<'info>,

  #[account(mut, has_one = whirlpool)]
  pub position: Account<'info, Position>,
  #[account(
      constraint = position_token_account.mint == position.position_mint,
      constraint = position_token_account.amount == 1
  )]
  pub position_token_account: Box<Account<'info, TokenAccount>>,

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

  pub whirlpool_program: Program<'info, whirlpool::program::Whirlpool>,
  pub token_program: Program<'info, Token>
}


#[error_code]
pub enum ErrorCode {
  Overflow,
  LiquidityZero,
  OutOfRange,
  TooMuchAmount,
  WhirlpoolNumberDownCastError,
}


// LOGIC REFERENCE
// increaseLiquidityQuoteByInputTokenWithParams >> quotePositionInRange
// https://github.com/orca-so/whirlpools/blob/main/sdk/src/quotes/public/increase-liquidity-quote.ts#L167
// getLiquidityFromTokenA
// https://github.com/orca-so/whirlpools/blob/537306c096bcbbf9cb8d5cff337c989dcdd999b4/sdk/src/utils/position-util.ts#L69
fn get_liquidity_from_token_a(amount: u128, sqrt_price_lower_x64: u128, sqrt_price_upper_x64: u128 ) -> Result<u128> {
  // Δa = liquidity/sqrt_price_lower - liquidity/sqrt_price_upper
  // liquidity = Δa * ((sqrt_price_lower * sqrt_price_upper) / (sqrt_price_upper - sqrt_price_lower))
  assert!(sqrt_price_lower_x64 < sqrt_price_upper_x64);
  let sqrt_price_diff = sqrt_price_upper_x64 - sqrt_price_lower_x64;

  let numerator = mul_u256(sqrt_price_lower_x64, sqrt_price_upper_x64); // x64 * x64
  let denominator = U256Muldiv::new(0, sqrt_price_diff); // x64

  let (quotient, _remainder) = numerator.div(denominator, false);

  let liquidity = quotient
    .mul(U256Muldiv::new(0, amount))
    .shift_word_right()
    .try_into_u128()
    .or(Err(ErrorCode::WhirlpoolNumberDownCastError.into()));
  liquidity
}
// getLiquidityFromTokenB
// https://github.com/orca-so/whirlpools/blob/537306c096bcbbf9cb8d5cff337c989dcdd999b4/sdk/src/utils/position-util.ts#L86
fn _get_liquidity_from_token_b_not_implemented(_amount: u128, _sqrt_price_lower_x64: u128, _sqrt_price_upper_x64: u128 ) -> Result<u128> {
  // Leave to not take the opportunity to improve skills...
  Ok(0u128)
}


// to display println! : cargo test -- --nocapture
#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_get_liquidity_from_token_a() {
    let r0 = get_liquidity_from_token_a(
      100_000_000_000u128,
      58319427345345388u128,
      82674692782969588u128,
    ).unwrap();
    println!("r0 = {}", r0);
    assert_eq!(r0, 1073181681u128);
  }
}