/// Initialize policy instruction
use anchor_lang::prelude::*;
use crate::constants::*;
use crate::errors::DammDistributorError;
use crate::state::*;
use crate::utils::validate_bps;

/// Initializes the policy configuration for fee distribution
///
/// # Arguments
/// * `ctx` - Context containing all required accounts
/// * `total_investor_allocation` - Total investor allocation at TGE (Y0)
/// * `investor_fee_share_bps` - Investor fee share in basis points (0-10000)
/// * `daily_cap_lamports` - Optional daily cap in lamports (0 = no cap)
/// * `min_payout_lamports` - Minimum payout threshold in lamports
///
/// # Returns
/// Result indicating success or failure
///
/// # Errors
/// Returns error if investor_fee_share_bps > 10000
pub fn handler(
    ctx: Context<InitializePolicy>,
    total_investor_allocation: u64,
    investor_fee_share_bps: u16,
    daily_cap_lamports: u64,
    min_payout_lamports: u64,
) -> Result<()> {
    // Validate basis points
    validate_bps(investor_fee_share_bps)?;
    
    let policy = &mut ctx.accounts.policy;
    
    policy.vault = ctx.accounts.vault.key();
    policy.quote_mint = ctx.accounts.quote_mint.key();
    policy.base_mint = ctx.accounts.base_mint.key();
    policy.pool = ctx.accounts.pool.key();
    policy.honorary_position = Pubkey::default(); // Will be set during position initialization
    policy.total_investor_allocation = total_investor_allocation;
    policy.investor_fee_share_bps = investor_fee_share_bps;
    policy.daily_cap_lamports = daily_cap_lamports;
    policy.min_payout_lamports = min_payout_lamports;
    policy.creator_quote_ata = ctx.accounts.creator_quote_ata.key();
    policy.authority = ctx.accounts.authority.key();
    policy.bump = ctx.bumps.policy;
    
    msg!("Policy initialized for vault: {}", policy.vault);
    msg!("Total investor allocation (Y0): {}", total_investor_allocation);
    msg!("Investor fee share: {} bps", investor_fee_share_bps);
    msg!("Daily cap: {} lamports", daily_cap_lamports);
    msg!("Min payout: {} lamports", min_payout_lamports);
    
    Ok(())
}

#[derive(Accounts)]
pub struct InitializePolicy<'info> {
    #[account(
        init,
        payer = authority,
        space = Policy::LEN,
        seeds = [POLICY_SEED, vault.key().as_ref()],
        bump
    )]
    pub policy: Account<'info, Policy>,
    
    /// CHECK: The vault account (can be any account, used as identifier)
    pub vault: AccountInfo<'info>,
    
    /// CHECK: Quote mint account
    pub quote_mint: AccountInfo<'info>,
    
    /// CHECK: Base mint account
    pub base_mint: AccountInfo<'info>,
    
    /// CHECK: Pool account
    pub pool: AccountInfo<'info>,
    
    /// CHECK: Creator's quote ATA
    pub creator_quote_ata: AccountInfo<'info>,
    
    #[account(mut)]
    pub authority: Signer<'info>,
    
    pub system_program: Program<'info, System>,
}
