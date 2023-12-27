use anchor_lang::prelude::*;
use std::io::Result;

#[account]
pub struct LiquidityLockbox {
  // TODO: pool
  // Whirlpool (LP) pool address
  pub whirlpool: Pubkey,
  // Lockbox bump
  pub lockbox_bump: [u8; 1],
  // Bridged token mint address
  pub bridged_token_mint: Pubkey,
  // PDA bridged ATA address
  pub pda_bridged_token_account: Pubkey,
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

impl LiquidityLockbox {
  pub fn seeds(&self) -> [&[u8]; 4] {
    [
      &b"liquidity_lockbox"[..],
      self.bridged_token_mint.as_ref(),
      self.pda_bridged_token_account.as_ref(),
      self.lockbox_bump.as_ref(),
    ]
  }

  pub fn initialize(
    &mut self,
    bump: u8,
    whirlpool: Pubkey,
    bridged_token_mint: Pubkey,
    pda_bridged_token_account: Pubkey
  ) -> Result<()> {
    self.whirlpool = whirlpool;
    self.bridged_token_mint = bridged_token_mint;
    self.pda_bridged_token_account = pda_bridged_token_account;
    self.num_position_accounts = 0;
    self.first_available_position_account_index = 0;
    self.total_liquidity = 0;
    self.lockbox_bump = [bump];

    Ok(())
  }
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Default, Copy)]
pub struct LockboxBumps {
  pub lockbox_bump: u8,
}