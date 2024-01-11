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
use anchor_lang::__private::CLOSED_ACCOUNT_DISCRIMINATOR;
use std::io::{Cursor, Write};
use std::ops::DerefMut;
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
      ctx.accounts.fee_collector_token_owner_account_b.key()
    )?;

    Ok(())
  }

  /// Deposits an NFT position under the Lockbox management and gets bridged tokens minted in return.
  ///
  /// ### Parameters
  /// - `id` - Lockbox position ID. Must be equal to the current total number of lockbox positions.
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

    // Check the discriminator
    let account = &ctx.accounts.position.to_account_info();

    let data = account.try_borrow_data()?;
    assert!(data.len() > 8);

    let mut discriminator = [0u8; 8];
    discriminator.copy_from_slice(&data[0..8]);
    if discriminator != POSITION_HEADER {
        return Err(ErrorCode::WrongPositionHeader.into());
    }

    // Check that the liquidity is within uint64 bounds
    if liquidity > std::u64::MAX as u128 {
      return Err(ErrorCode::LiquidityOverflow.into());
    }

    let position_liquidity = liquidity as u64;

    // Check for the minimum liquidity in position
    if position_liquidity == 0 {
      return Err(ErrorCode::LiquidityZero.into());
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

    // Check the lockbox PDA address correctness
    let lockbox_pda = Pubkey::find_program_address(&[b"liquidity_lockbox"], &PROGRAM_ID);
    if lockbox_pda.0 != ctx.accounts.lockbox.key() {
      return Err(ErrorCode::WrongLockboxPDA.into());
    }

    // Check the id that has to match the number of lockbox positions in order to create a correct account
    // The position needs to be provided as an argument since it's passed into the instruction field
    let num_positions = ctx.accounts.lockbox.num_positions;
    if num_positions != id {
      return Err(ErrorCode::WrongPositionId.into());
    }

    // Transfer position to the program PDA ATA
    token::transfer(
      CpiContext::new(
        ctx.accounts.token_program.to_account_info(),
        Transfer {
          from: ctx.accounts.position_token_account.to_account_info(),
          to: ctx.accounts.pda_position_account.to_account_info(),
          authority: ctx.accounts.signer.to_account_info(),
        },
      ),
      1,
    )?;

    // Close user position account
    invoke_signed(
      &close_account(
        ctx.accounts.token_program.key,
        ctx.accounts.position_token_account.to_account_info().key,
        ctx.accounts.signer.to_account_info().key,
        ctx.accounts.signer.to_account_info().key,
        &[],
      )?,
      &[
        ctx.accounts.token_program.to_account_info(),
        ctx.accounts.position_token_account.to_account_info(),
        ctx.accounts.signer.to_account_info(),
        ctx.accounts.signer.to_account_info(),
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

    emit!(DepositEvent {
      signer: ctx.accounts.signer.key(),
      pda_lockbox_position: ctx.accounts.pda_lockbox_position.key(),
      pda_position_account: ctx.accounts.pda_position_account.key(),
      position: ctx.accounts.position.key(),
      position_liquidity,
    });

    Ok(())
  }

  /// Withdraws a specified amount of liquidity for supplied bridged tokens.
  ///
  /// ### Parameters
  /// - `id` - Lockbox position ID. Must be smaller than the total number of lockbox positions.
  /// - `amount` - Amount of bridged tokens corresponding to the position liquidity part to withdraw.
  /// - `token_min_a` - The minimum amount of tokenA the user is willing to withdraw.
  /// - `token_min_b` - The minimum amount of tokenB the user is willing to withdraw.
  pub fn withdraw(
    ctx: Context<WithdrawLiquidityForTokens>,
    id: u32,
    amount: u64,
    token_min_a: u64,
    token_min_b: u64
  ) -> Result<()> {
    // Check if there is any liquidity left in the Lockbox
    if ctx.accounts.lockbox.total_liquidity == 0 {
      return Err(ErrorCode::TotalLiquidityZero.into());
    }

    // TODO: any other way to get PROGRAM_ID?
    // Get the lockbox position PDA ATA
    if id >= ctx.accounts.lockbox.num_positions {
      return Err(ErrorCode::WrongPositionId.into());
    }
    let lockbox_position = Pubkey::find_program_address(&[b"lockbox_position", id.to_be_bytes().as_ref()], &PROGRAM_ID);

    // Check that the calculated address matches the provided PDA lockbox position
    if lockbox_position.0 != ctx.accounts.pda_lockbox_position.key() {
      return Err(ErrorCode::WrongPDAPositionAccount.into());
    }

    // Check the lockbox PDA address correctness
    let lockbox_pda = Pubkey::find_program_address(&[b"liquidity_lockbox"], &PROGRAM_ID);
    if lockbox_pda.0 != ctx.accounts.lockbox.key() {
      return Err(ErrorCode::WrongLockboxPDA.into());
    }

    // Check that the first token mint is SOL
    if ctx.accounts.token_owner_account_a.mint != SOL || ctx.accounts.token_vault_a.mint != SOL {
      return Err(ErrorCode::WrongTokenMint.into());
    }

    // Check that the second token mint is OLAS
    if ctx.accounts.token_owner_account_b.mint != OLAS || ctx.accounts.token_vault_b.mint != OLAS {
      return Err(ErrorCode::WrongTokenMint.into());
    }

    // Check tick arrays owner
    if ctx.accounts.tick_array_lower.to_account_info().owner != &ORCA ||
      ctx.accounts.tick_array_upper.to_account_info().owner != &ORCA {
      return Err(ErrorCode::WrongOwner.into());
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

    // Check the Orca Whirlpool program address
    if ctx.accounts.whirlpool_program.key() != ORCA {
        return Err(ErrorCode::WrongOrcaAccount.into());
    }

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
    whirlpool::cpi::decrease_liquidity(cpi_ctx_modify_liquidity, amount as u128, token_min_a, token_min_b)?;

    // Get the post-withdraw token remainder
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

      // Close the pda_lockbox_position account and send all lamports to the receiver
      // Secure reference: https://github.com/coral-xyz/sealevel-attacks/blob/master/programs/9-closing-accounts/secure/src/lib.rs
      let dest_starting_lamports = ctx.accounts.signer.lamports();

      let account = ctx.accounts.pda_lockbox_position.to_account_info();
      **ctx.accounts.signer.lamports.borrow_mut() = dest_starting_lamports
        .checked_add(account.lamports())
        .unwrap();
      **account.lamports.borrow_mut() = 0;

      let mut data = account.try_borrow_mut_data()?;
      for byte in data.deref_mut().iter_mut() {
        *byte = 0;
      }

      let dst: &mut [u8] = &mut data;
      let mut cursor = Cursor::new(dst);
      cursor.write_all(&CLOSED_ACCOUNT_DISCRIMINATOR).unwrap();
    } else {
      // Update position liquidity
      ctx.accounts.pda_lockbox_position.position_liquidity = remainder;
    }

    // Decrease the total liquidity amount
    ctx.accounts.lockbox.total_liquidity -= amount;

    emit!(WithdrawEvent {
      signer: ctx.accounts.signer.key(),
      pda_lockbox_position: ctx.accounts.pda_lockbox_position.key(),
      pda_position_account: ctx.accounts.pda_position_account.key(),
      position: ctx.accounts.position.key(),
      token_owner_account_a: ctx.accounts.token_owner_account_a.key(),
      token_owner_account_b: ctx.accounts.token_owner_account_b.key(),
      amount,
      remainder
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

  #[account(constraint = signer.key == &fee_collector_token_owner_account_a.owner)]
  pub fee_collector_token_owner_account_a: Box<Account<'info, TokenAccount>>,
  #[account(constraint = signer.key == &fee_collector_token_owner_account_b.owner)]
  pub fee_collector_token_owner_account_b: Box<Account<'info, TokenAccount>>,

  #[account(address = token::ID)]
  pub token_program: Program<'info, Token>,
  pub system_program: Program<'info, System>,
  pub rent: Sysvar<'info, Rent>
}

#[derive(Accounts)]
#[instruction(id: u32)]
pub struct DepositPositionForLiquidity<'info> {
  #[account(mut)]
  pub signer: Signer<'info>,

  pub position: Box<Account<'info, Position>>,
  #[account(mut,
    constraint = signer.key == &position_token_account.owner,
    constraint = position_token_account.mint == position.position_mint,
    constraint = position_token_account.amount == 1
  )]
  pub position_token_account: Box<Account<'info, TokenAccount>>,

  #[account(address = position_token_account.mint,
    constraint = position_mint.supply == 1
  )]
  pub position_mint: Account<'info, Mint>,

  #[account(init,
    associated_token::authority = lockbox,
    associated_token::mint = position_mint,
    payer = signer)]
  pub pda_position_account: Box<Account<'info, TokenAccount>>,

  #[account(init,
    seeds = [
      b"lockbox_position".as_ref(),
      id.to_be_bytes().as_ref()
    ],
    bump,
    space = LockboxPosition::LEN,
    payer = signer)]
  pub pda_lockbox_position: Box<Account<'info, LockboxPosition>>,

  #[account(mut)]
  pub bridged_token_mint: Box<Account<'info, Mint>>,
  #[account(mut,
    constraint = bridged_token_account.mint == lockbox.bridged_token_mint,
    constraint = bridged_token_mint.key() == lockbox.bridged_token_mint,
    constraint = signer.key == &bridged_token_account.owner,
  )]
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
  #[account(mut,
    constraint = bridged_token_account.mint == lockbox.bridged_token_mint,
    constraint = lockbox.bridged_token_mint == bridged_token_mint.key(),
    constraint = signer.key == &bridged_token_account.owner,
  )]
  pub bridged_token_account: Box<Account<'info, TokenAccount>>,

  #[account(mut, has_one = whirlpool, has_one = position_mint)]
  pub position: Box<Account<'info, Position>>,
  #[account(mut,
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
    constraint = pda_lockbox_position.position_account == position.key(),
    constraint = pda_lockbox_position.position_pda_ata == pda_position_account.key(),
    constraint = pda_lockbox_position.to_account_info().owner == lockbox.to_account_info().owner
  )]
  pub pda_lockbox_position: Box<Account<'info, LockboxPosition>>,

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

  #[account(mut, has_one = whirlpool, constraint = tick_array_lower.key() != tick_array_upper.key())]
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
  #[msg("Wrong position PDA header")]
  WrongPositionHeader,
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
  #[msg("Provided wrong lockbox PDA")]
  WrongLockboxPDA,
  #[msg("Provided wrong position ATA")]
  WrongPositionAccount,
  #[msg("Provided wrong PDA position ATA")]
  WrongPDAPositionAccount,
  #[msg("Provided wrong Orca program account")]
  WrongOrcaAccount,
  #[msg("Wrong token mint")]
  WrongTokenMint
}


#[event]
pub struct DepositEvent {
    // Signer (user)
    #[index]
    pub signer: Pubkey,
    // Created PDA lockbox position account
    #[index]
    pub pda_lockbox_position: Pubkey,
    // Created PDA position account
    #[index]
    pub pda_position_account: Pubkey,

    // Position account
    pub position: Pubkey,
    // Position liquidity
    pub position_liquidity: u64
}

#[event]
pub struct WithdrawEvent {
    // Signer (user)
    #[index]
    pub signer: Pubkey,
    // Created PDA lockbox position account
    #[index]
    pub pda_lockbox_position: Pubkey,
    // Created PDA position account
    #[index]
    pub pda_position_account: Pubkey,

    // Position account
    pub position: Pubkey,
    // User ATA token A
    token_owner_account_a: Pubkey,
    // User ATA token B
    token_owner_account_b: Pubkey,
    // Withdraw amount
    pub amount: u64,
    // Position liquidity remainder
    pub remainder: u64
}
