use anchor_lang::prelude::*;

#[account]
pub struct LiquidityLockbox {
  // Lockbox bump
  pub lockbox_bump: [u8; 1],
  // Bridged token mint address
  pub bridged_token_mint: Pubkey,
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
  pub fn seeds(&self) -> [&[u8]; 2] {
    [
      &b"liquidity_lockbox"[..],
      self.lockbox_bump.as_ref()
    ]
  }

  pub fn initialize(
    &mut self,
    bump: u8,
    bridged_token_mint: Pubkey
  ) -> Result<()> {
    self.bridged_token_mint = bridged_token_mint;
    self.total_liquidity = 0;
    self.lockbox_bump = [bump];
    self.position_liquidity = Vec::new();
    self.position_accounts = Vec::new();
    self.position_pda_ata = Vec::new();

    Ok(())
  }
}