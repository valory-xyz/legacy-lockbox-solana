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
  // Bridged token decimals
  const BRIDGED_TOKEN_DECIMALS: u8 = 9;

  pub fn initialize(
    ctx: Context<InitializeLiquidityLockbox>,
    whirlpool: Pubkey
  ) -> Result<()> {
    let bridged_token_mint = ctx.accounts.bridged_token_mint.key();

    // Get the lockbox account
    let lockbox = &mut ctx.accounts.lockbox;

    // Get the anchor-derived bump
    let bump = *ctx.bumps.get("liquidity_lockbox").unwrap();

    Ok(lockbox.initialize(
      bump,
      whirlpool,
      bridged_token_mint
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

    let position_liquidity = liquidity as u64;

    let tick_lower_index = ctx.accounts.position.tick_lower_index;
    let tick_upper_index = ctx.accounts.position.tick_upper_index;

    // Transfer position
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
        ctx.accounts.receiver.to_account_info().key,
        ctx.accounts.position_authority.to_account_info().key,
        &[],
      )?,
      &[
        ctx.accounts.token_program.to_account_info(),
        ctx.accounts.position_token_account.to_account_info(),
        ctx.accounts.receiver.to_account_info(),
        ctx.accounts.position_authority.to_account_info(),
      ],
      &[],
    )?;

    // Mint bridged tokens
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

    // Increase the total number of positions
    lockbox.num_position_accounts += 1;
    // Increase the amount of total liquidity
    lockbox.total_liquidity += position_liquidity;

    Ok(())
  }

  pub fn decrease_liquidity(
    ctx: Context<WithdrawLiquidityForTokens>,
    amount: u64,
  ) -> Result<()> {
    // Get the lockbox state
    let lockbox = &ctx.accounts.lockbox;

    let idx: usize = lockbox.first_available_position_account_index as usize;
    let position_liquidity: u64 = lockbox.position_liquidity[idx];
    // TODO: check as this must never happen
    // Check that the token account exists
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
        ctx.accounts.token_authority.to_account_info().key,
        &[],
        amount,
        BRIDGED_TOKEN_DECIMALS,
      )?,
      &[
        ctx.accounts.token_program.to_account_info(),
        ctx.accounts.bridged_token_account.to_account_info(),
        ctx.accounts.bridged_token_mint.to_account_info(),
        ctx.accounts.token_authority.to_account_info(),
      ],
      &[]
    )?;

    // // Close user account is it has zero amount of tokens
    // invoke_signed(
    //   &close_account(
    //     token_program.key,
    //     position_token_account.to_account_info().key,
    //     receiver.key,
    //     token_authority.key,
    //     &[],
    //   )?,
    //   &[
    //     token_program.to_account_info(),
    //     position_token_account.to_account_info(),
    //     receiver.to_account_info(),
    //     token_authority.to_account_info(),
    //   ],
    //   &[],
    // )?;

    // CPI to decrease liquidity
    let cpi_program = ctx.accounts.whirlpool_program.to_account_info();
    msg!("after cpi_program");
    let cpi_accounts = ModifyLiquidity {
      whirlpool: ctx.accounts.whirlpool.to_account_info(),
      position: ctx.accounts.position.to_account_info(),
      position_authority: ctx.accounts.token_authority.to_account_info(),
      position_token_account: ctx.accounts.pda_position_account.to_account_info(),
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
    whirlpool::cpi::decrease_liquidity(cpi_ctx, amount as u128, 0, 0)?;

    // Update the token remainder
    let remainder: u64 = position_liquidity - amount;

    // If requested amount can be fully covered by the current position liquidity, close the position
    if remainder == 0 {
      // Update fees for the position
      // AccountMeta[4] metasUpdateFees = [
      //   AccountMeta({pubkey: pool, is_writable: true, is_signer: false}),
      //   AccountMeta({pubkey: positionAddress, is_writable: true, is_signer: false}),
      //   AccountMeta({pubkey: tx.accounts.tickArrayLower.key, is_writable: false, is_signer: false}),
      //   AccountMeta({pubkey: tx.accounts.tickArrayUpper.key, is_writable: false, is_signer: false})
      // ];
      // whirlpool.updateFeesAndRewards{accounts: metasUpdateFees, seeds: [[pdaProgramSeed, pdaBump]]}();
      //
      // // Collect fees from the position
      // AccountMeta[9] metasCollectFees = [
      //   AccountMeta({pubkey: pool, is_writable: true, is_signer: false}),
      //   AccountMeta({pubkey: pdaProgram, is_writable: false, is_signer: true}),
      //   AccountMeta({pubkey: positionAddress, is_writable: true, is_signer: false}),
      //   AccountMeta({pubkey: pdaPositionAta, is_writable: false, is_signer: false}),
      //   AccountMeta({pubkey: tx.accounts.userTokenAccountA.key, is_writable: true, is_signer: false}),
      //   AccountMeta({pubkey: tx.accounts.tokenVaultA.key, is_writable: true, is_signer: false}),
      //   AccountMeta({pubkey: tx.accounts.userTokenAccountB.key, is_writable: true, is_signer: false}),
      //   AccountMeta({pubkey: tx.accounts.tokenVaultB.key, is_writable: true, is_signer: false}),
      //   AccountMeta({pubkey: SplToken.tokenProgramId, is_writable: false, is_signer: false})
      // ];
      // whirlpool.collectFees{accounts: metasCollectFees, seeds: [[pdaProgramSeed, pdaBump]]}();
      //
      // // Close the position
      // AccountMeta[6] metasClosePosition = [
      //   AccountMeta({pubkey: pdaProgram, is_writable: false, is_signer: true}),
      //   AccountMeta({pubkey: tx.accounts.userWallet.key, is_writable: true, is_signer: false}),
      //   AccountMeta({pubkey: positionAddress, is_writable: true, is_signer: false}),
      //   AccountMeta({pubkey: tx.accounts.positionMint.key, is_writable: true, is_signer: false}),
      //   AccountMeta({pubkey: pdaPositionAta, is_writable: true, is_signer: false}),
      //   AccountMeta({pubkey: SplToken.tokenProgramId, is_writable: false, is_signer: false})
      // ];
      // whirlpool.closePosition{accounts: metasClosePosition, seeds: [[pdaProgramSeed, pdaBump]]}();
    }

    // TODO: Check the CEI pattern if it makes sense, as it's not possible to declare the mutable before
    let lockbox_mut = &mut ctx.accounts.lockbox;

    if remainder == 0 {
      // Increase the first available position account index
      lockbox_mut.first_available_position_account_index += 1;
    }

    // Decrease the total liquidity amount
    lockbox_mut.total_liquidity -= amount;
    // Update liquidity and its associated position account
    lockbox_mut.position_liquidity[idx] = remainder;

    Ok(())
  }
}


#[derive(Accounts)]
#[instruction(bumps: LockboxBumps)]
pub struct InitializeLiquidityLockbox<'info> {
  #[account(mut)]
  pub signer: Signer<'info>, //signer must sign the transaction to create accounts

  pub bridged_token_mint: Box<Account<'info, Mint>>,

  #[account(init,
    seeds = [
      b"liquidity_lockbox".as_ref(),
      bridged_token_mint.key().as_ref()
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

// @mutableAccount(position_token_account userPositionAccount)
// @mutableAccount(pda_position_account pdaPositionAccount)
// @mutableAccount(bridged_token_account userBridgedTokenAccount)
// @mutableAccount(bridged_token_mint bridgedTokenMint)
// @account(position)
// @signer(position_authority userWallet)

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
    constraint = pda_position_account.amount == 0
  )]
  pub pda_position_account: Box<Account<'info, TokenAccount>>,

  #[account(mut)]
  pub bridged_token_mint: Account<'info, Mint>,
  #[account(mut, constraint = bridged_token_account.mint == bridged_token_mint.key())]
  pub bridged_token_account: Account<'info, TokenAccount>,

  #[account(mut)]
  pub receiver: Account<'info, TokenAccount>,

  #[account(mut)]
  pub lockbox: Box<Account<'info, LiquidityLockbox>>,
  pub token_program: Program<'info, Token>
}

#[derive(Accounts)]
pub struct WithdrawLiquidityForTokens<'info> {
  #[account(mut)]
  pub whirlpool: Account<'info, Whirlpool>,

  pub token_authority: Signer<'info>,

  #[account(mut)]
  pub bridged_token_mint: Account<'info, Mint>,
  #[account(mut, constraint = bridged_token_account.mint == bridged_token_mint.key())]
  pub bridged_token_account: Account<'info, TokenAccount>,

  #[account(mut, has_one = whirlpool)]
  pub position: Account<'info, Position>,
  #[account(
      constraint = pda_position_account.mint == position.position_mint,
      constraint = pda_position_account.amount == 1
  )]
  pub pda_position_account: Box<Account<'info, TokenAccount>>,

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
  pub token_program: Program<'info, Token>
}


#[error_code]
pub enum ErrorCode {
  Overflow,
  LiquidityZero,
  AmountExceedsPositionLiquidity,
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