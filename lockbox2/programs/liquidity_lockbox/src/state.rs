use anchor_lang::prelude::*;

#[account]
pub struct LiquidityLockbox {
  // Lockbox bump
  pub bump: [u8; 1],
  // Bridged token mint address
  pub bridged_token_mint: Pubkey,
  // Fee collector ATA for token A
  pub fee_collector_token_owner_account_a: Pubkey,
  // Fee collector ATA for token B
  pub fee_collector_token_owner_account_b: Pubkey,
  // Liquidity position
  pub position: Pubkey,
  // PDA position ATA
  pub pda_position_account: Pubkey,
  // Total liquidity
  pub total_liquidity: u64
}

impl LiquidityLockbox {
  pub const LEN: usize = 8 + 1 + 32 * 5 + 8;

  pub fn seeds(&self) -> [&[u8]; 2] {
    [
      &b"liquidity_lockbox"[..],
      self.bump.as_ref()
    ]
  }

  pub fn initialize(
    &mut self,
    bump: u8,
    bridged_token_mint: Pubkey,
    fee_collector_token_owner_account_a: Pubkey,
    fee_collector_token_owner_account_b: Pubkey,
    position: Pubkey,
    pda_position_account: Pubkey
  ) -> Result<()> {
    self.bridged_token_mint = bridged_token_mint;
    self.fee_collector_token_owner_account_a = fee_collector_token_owner_account_a;
    self.fee_collector_token_owner_account_b = fee_collector_token_owner_account_b;
    self.position = position;
    self.pda_position_account = pda_position_account;
    self.total_liquidity = 0;
    self.bump = [bump];

    Ok(())
  }
}
