use anchor_lang::prelude::*;
use anchor_spl::{
  token::{TokenAccount, Token},
};
use whirlpool::{
  self,
  state::{Whirlpool, TickArray, Position},
  cpi::accounts::ModifyLiquidity,
  math::sqrt_price_from_tick_index,
  math::{mul_u256, U256Muldiv},
  manager::liquidity_manager::calculate_liquidity_token_deltas,
};
use solana_program::{pubkey::Pubkey};

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
  //const POOL: Pubkey = pubkey!("");
  // Full range lower and upper indexes
  const TICK_LOWER_INDEX: i32 = -443632;
  const TICK_UPPER_INDEX: i32 = 443632;

  pub fn initialize(
    ctx: Context<InitializeLiquidityLockbox>,
    pool: Pubkey,
    bridged_token_mint: Pubkey,
    pda_bridged_token_account: Pubkey,
    pda_program_seed: String
  ) -> ProgramResult {
    let lockbox = &mut ctx.accounts.lockbox;
    lockbox.pool = pool;
    lockbox.bridged_token_mint = bridged_token_mint;
    lockbox.pda_bridged_token_account = pda_bridged_token_account;
    //lockbox.pda_program_seed = PDA_PROGRAM_SEED;
    lockbox.pda_program_seed = pda_program_seed;
    lockbox.num_position_accounts = 0;
    lockbox.first_available_position_account_index = 0;
    lockbox.total_liquidity = 0;

    Ok(())
  }

  // pub fn deposit() -> ProgramResult {
  //
  //   Ok(())
  // }

  pub fn get_position_info(ctx: Context<DelegatedModifyLiquidity>) -> ProgramResult {
    let position = &ctx.accounts.position.to_account_info();
    let data = &position.data;//position.data.try_unwrap();//.readAddress(8);
    let whirlpool = &data.readAddress(8);
    // let pos = Pos {
    //   whirlpool: position.whirlpool,
    //   position_mint: position.position_mint,
    //   liquidity: position.liquiduity,
    //   tick_lower_index: position.tick_lower_index,
    //   tick_upper_index: position.tick_upper_index
    // }

    Ok(())
  }

  pub fn decrease_liquidity(
    ctx: Context<DelegatedModifyLiquidity>,
    liquidity: u128,
    token_min_a: u64,
    token_min_b: u64,
  ) -> ProgramResult {

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
#[instruction(pda_program_seed: String)]
pub struct InitializeLiquidityLockbox<'info,> {
  #[account(init, payer = signer, space = 10000, seeds = [pda_program_seed.as_bytes().as_ref()], bump)]
  pub lockbox: Account<'info, LiquidityLockbox>,
  #[account(mut)]
  pub signer: Signer<'info,>, //signer must sign the transaction to create the account
  pub system_program: Program<'info, System>
}

#[account]
pub struct LiquidityLockbox {
  // TODO: pool
  // Whirlpool (LP) pool address
  pub pool: Pubkey,
  // Bridged token mint address
  pub bridged_token_mint: Pubkey,
  // PDA bridged ATA address
  pub pda_bridged_token_account: Pubkey,
  // PDA program seed string
  pub pda_program_seed: String,
  // Total number of token accounts (even those that hold no positions anymore)
  pub num_position_accounts: u32,
  // First available account index in the set of accounts;
  pub first_available_position_account_index: u32,
  // Total liquidity in a lockbox
  pub total_liquidity: u64,
  // Set of locked position data accounts
  pub position_accounts: Vec<Pubkey>,
  // Set of locked position PDA ATAs
  pub position_pda_ata: Vec<Pubkey>,
  // Set of locked position liquidity amounts
  pub position_liquidity: Vec<u64>
}


pub struct Pos {
  pub whirlpool: Pubkey,
  pub position_mint: Pubkey,
  pub liquidity: u128,
  pub tick_lower_index: i32,
  pub tick_upper_index: i32,
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
  pub token_program: Program<'info, Token>,
}


#[error]
pub enum ErrorCode {
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