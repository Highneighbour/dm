/// Events emitted by the DAMM Distributor program
use anchor_lang::prelude::*;

#[event]
pub struct HonoraryPositionInitialized {
    pub position: Pubkey,
    pub owner_pda: Pubkey,
    pub quote_mint: Pubkey,
    pub base_mint: Pubkey,
    pub pool: Pubkey,
    pub timestamp: i64,
}

#[event]
pub struct QuoteFeesClaimed {
    pub amount: u64,
    pub timestamp: i64,
    pub day_start: i64,
}

#[event]
pub struct InvestorPayoutPage {
    pub page_index: u32,
    pub investors_paid: u32,
    pub total_amount_page: u64,
    pub cumulative_day_amount: u64,
    pub timestamp: i64,
}

#[event]
pub struct CreatorPayoutDayClosed {
    pub creator: Pubkey,
    pub remainder_amount: u64,
    pub total_claimed_day: u64,
    pub total_to_investors: u64,
    pub day_start: i64,
    pub day_end: i64,
}
