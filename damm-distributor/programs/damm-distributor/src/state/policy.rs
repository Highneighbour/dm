/// Policy configuration for fee distribution
use anchor_lang::prelude::*;

#[account]
#[derive(Default)]
pub struct Policy {
    /// The vault this policy is associated with
    pub vault: Pubkey,
    
    /// Quote mint for the pool
    pub quote_mint: Pubkey,
    
    /// Base mint for the pool
    pub base_mint: Pubkey,
    
    /// The pool address
    pub pool: Pubkey,
    
    /// Honorary position address
    pub honorary_position: Pubkey,
    
    /// Total investor allocation at TGE (Y0)
    pub total_investor_allocation: u64,
    
    /// Investor fee share in basis points (0-10000)
    pub investor_fee_share_bps: u16,
    
    /// Optional daily cap in lamports (0 = no cap)
    pub daily_cap_lamports: u64,
    
    /// Minimum payout threshold in lamports (dust threshold)
    pub min_payout_lamports: u64,
    
    /// Creator's quote token account (receives remainder)
    pub creator_quote_ata: Pubkey,
    
    /// Authority that can update policy (typically the program itself or admin)
    pub authority: Pubkey,
    
    /// Bump seed for PDA derivation
    pub bump: u8,
}

impl Policy {
    /// Space needed for the Policy account
    /// 8 (discriminator) + 32*6 (pubkeys) + 8*3 (u64s) + 2 (u16) + 1 (u8) = 8 + 192 + 24 + 2 + 1 = 227
    pub const LEN: usize = 8 + 32 * 6 + 8 * 3 + 2 + 1;
}
