use anchor_lang::prelude::*;

declare_id!("qq5wTF74RS5kWUoBrzxtii7QggF13obiutPaRyu9PMz");

pub mod constants;
pub mod errors;
pub mod events;
pub mod instructions;
pub mod state;
pub mod utils;

#[cfg(test)]
mod tests;

use instructions::*;

#[program]
pub mod damm_distributor {
    use super::*;

    /// Initializes the policy configuration for fee distribution
    ///
    /// # Arguments
    /// * `ctx` - Context containing all required accounts
    /// * `total_investor_allocation` - Total investor allocation at TGE (Y0)
    /// * `investor_fee_share_bps` - Investor fee share in basis points (0-10000)
    /// * `daily_cap_lamports` - Optional daily cap in lamports (0 = no cap)
    /// * `min_payout_lamports` - Minimum payout threshold in lamports
    pub fn initialize_policy(
        ctx: Context<InitializePolicy>,
        total_investor_allocation: u64,
        investor_fee_share_bps: u16,
        daily_cap_lamports: u64,
        min_payout_lamports: u64,
    ) -> Result<()> {
        instructions::initialize_policy::handler(
            ctx,
            total_investor_allocation,
            investor_fee_share_bps,
            daily_cap_lamports,
            min_payout_lamports,
        )
    }

    /// Initializes the honorary DAMM v2 LP position
    ///
    /// # Arguments
    /// * `ctx` - Context containing all required accounts
    pub fn initialize_position(ctx: Context<InitializePosition>) -> Result<()> {
        instructions::initialize_position::handler(ctx)
    }

    /// Permissionless crank that claims and distributes quote fees
    ///
    /// # Arguments
    /// * `ctx` - Context containing all required accounts
    /// * `page_index` - Current page index for pagination
    /// * `is_final_page` - Whether this is the final page of the day
    pub fn crank_distribution<'info>(
        ctx: Context<'_, '_, '_, 'info, CrankDistribution<'info>>,
        page_index: u32,
        is_final_page: bool,
    ) -> Result<()> {
        instructions::crank_distribution::handler(ctx, page_index, is_final_page)
    }
}
