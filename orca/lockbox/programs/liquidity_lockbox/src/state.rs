use anchor_lang::prelude::*;

#[account]
pub struct LiquidityLockbox {
  // Lockbox bump
  pub bump: [u8; 1],
  // Bridged token mint address
  pub bridged_token_mint: Pubkey,
  // Total liquidity in a lockbox
  pub total_liquidity: u64,
  // Total number of lockbox positions
  pub num_positions: u32
}

impl LiquidityLockbox {
  pub const LEN: usize = 8 + 1 + 32 + 8 + 4;

  pub fn seeds(&self) -> [&[u8]; 2] {
    [
      &b"liquidity_lockbox"[..],
      self.bump.as_ref()
    ]
  }

  pub fn initialize(
    &mut self,
    bump: u8,
    bridged_token_mint: Pubkey
  ) -> Result<()> {
    self.bridged_token_mint = bridged_token_mint;
    self.total_liquidity = 0;
    self.num_positions = 0;
    self.bump = [bump];

    Ok(())
  }
}

#[account]
pub struct LockboxPosition {
  // Position identifier
  pub id: [u8; 4],
  // Position bump
  pub bump: [u8; 1],
  // Locked position data account
  pub position_account: Pubkey,
  // Locked position PDA ATA
  pub position_pda_ata: Pubkey,
  // Locked position liquidity
  pub position_liquidity: u64
}

impl LockboxPosition {
  pub const LEN: usize = 8 + 4 + 1 + 32 + 32 + 8;

  pub fn seeds(&self) -> [&[u8]; 3] {
    [
      &b"lockbox_position"[..],
      self.id.as_ref(),
      self.bump.as_ref()
    ]
  }

  pub fn initialize(
    &mut self,
    id: u32,
    bump: u8,
    position_liquidity: u64,
    position_account: Pubkey,
    position_pda_ata: Pubkey
  ) -> Result<()> {
    self.id = id.to_be_bytes();
    self.bump = [bump];
    self.position_liquidity = position_liquidity;
    self.position_account = position_account;
    self.position_pda_ata = position_pda_ata;

    Ok(())
  }
}