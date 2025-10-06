/// Distribution progress tracking for pagination and state management
use anchor_lang::prelude::*;

#[account]
#[derive(Default)]
pub struct DistributionProgress {
    /// The vault this progress tracker is associated with
    pub vault: Pubkey,
    
    /// Last distribution timestamp (start of last day)
    pub last_distribution_ts: i64,
    
    /// Current day window start timestamp
    pub current_day_start: i64,
    
    /// Total amount claimed from fees for current day
    pub total_claimed_day: u64,
    
    /// Cumulative amount distributed to investors in current day
    pub cumulative_distributed: u64,
    
    /// Carry-over amount from previous pages (dust)
    pub carry_over: u64,
    
    /// Current pagination cursor (page index)
    pub pagination_cursor: u32,
    
    /// Whether the day is finalized
    pub day_finalized: bool,
    
    /// Daily cap remaining for current day
    pub daily_cap_remaining: u64,
    
    /// Bump seed for PDA derivation
    pub bump: u8,
}

impl DistributionProgress {
    /// Space needed for the DistributionProgress account
    /// 8 (discriminator) + 32 (pubkey) + 8*5 (i64/u64s) + 4 (u32) + 1 (bool) + 1 (u8) = 8 + 32 + 40 + 4 + 1 + 1 = 86
    pub const LEN: usize = 8 + 32 + 8 * 5 + 4 + 1 + 1;
}
