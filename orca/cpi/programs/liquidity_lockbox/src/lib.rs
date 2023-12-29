pub mod state;
use anchor_lang::prelude::*;
use anchor_spl::token::{self, Mint, Token, TokenAccount, Transfer};
use whirlpool::{
  self,
  state::{Whirlpool, TickArray, Position},
  cpi::accounts::ModifyLiquidity,
  cpi::accounts::UpdateFeesAndRewards,
  cpi::accounts::CollectFees,
  cpi::accounts::ClosePosition,
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
  const TICK_LOWER_INDEX: i32 = -443632; // -444928
  const TICK_UPPER_INDEX: i32 = 443632; // 439296
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
    let tick_lower_index = ctx.accounts.position.tick_lower_index;
    let tick_upper_index = ctx.accounts.position.tick_upper_index;

    // Check for the zero liquidity in position
    if liquidity == 0 {
      return Err(ErrorCode::LiquidityZero.into());
    }
    // Check that the liquidity is within uint64 bounds
    if liquidity > std::u64::MAX as u128 {
      return Err(ErrorCode::Overflow.into());
    }

    if tick_lower_index != TICK_LOWER_INDEX || tick_upper_index != TICK_UPPER_INDEX {
      return Err(ErrorCode::OutOfRange.into());
    }

    let owner = ctx.accounts.position.to_account_info().owner;
    if owner != &ORCA {
      return Err(ErrorCode::WrongOwner.into());
    }

    let position_pda = Pubkey::try_find_program_address(&[b"position", position_mint.as_ref()], &ORCA);
    let position_pda_pubkey = position_pda.map(|(pubkey, _)| pubkey);
    if position_pda_pubkey.unwrap() != ctx.accounts.position.key() {
      return Err(ErrorCode::WrongPositionPDA.into());
    }

    let position_liquidity = liquidity as u64;

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

  pub fn withdraw(
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

    // // Close user account is it has zero amount of tokens
    // invoke_signed(
    //   &close_account(
    //     token_program.key,
    //     position_token_account.to_account_info().key,
    //     signer.key,
    //     signer.key,
    //     &[],
    //   )?,
    //   &[
    //     token_program.to_account_info(),
    //     position_token_account.to_account_info(),
    //     signer.to_account_info(),
    //     signer.to_account_info(),
    //   ],
    //   &[],
    // )?;

    // CPI to decrease liquidity
    // TODO: find out how to keep the same cpi_program variable for all of the calls
    let cpi_program_modify_liquidity = ctx.accounts.whirlpool_program.to_account_info();
    msg!("after cpi_program");
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
    msg!("after cpi_accounts");

    let signer_seeds = &[&ctx.accounts.lockbox.seeds()[..]];
    let cpi_ctx_modify_liquidity = CpiContext::new_with_signer(
      cpi_program_modify_liquidity,
      cpi_accounts_modify_liquidity,
      signer_seeds
    );
    msg!("before CPI");
    whirlpool::cpi::decrease_liquidity(cpi_ctx_modify_liquidity, amount as u128, 0, 0)?;

    // Update the token remainder
    let remainder: u64 = position_liquidity - amount;

    // If requested amount can be fully covered by the current position liquidity, close the position
    if remainder == 0 {
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
  pub lockbox: Box<Account<'info, LiquidityLockbox>>,
  pub token_program: Program<'info, Token>
}

#[derive(Accounts)]
pub struct WithdrawLiquidityForTokens<'info> {
  #[account(mut)]
  pub whirlpool: Account<'info, Whirlpool>,

  pub signer: Signer<'info>,

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

  #[account(mut,
    address = position.position_mint,
    constraint = position.whirlpool == whirlpool.key()
  )]
  pub position_mint: Account<'info, Mint>,

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
  WrongOwner,
  WrongPositionPDA
}


// to display println! : cargo test -- --nocapture
#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_test() {
  }
}