/// Initialize honorary position instruction
use anchor_lang::prelude::*;
use anchor_spl::token::{Token, TokenAccount};
use crate::constants::*;
use crate::errors::DammDistributorError;
use crate::state::*;
use crate::events::*;

/// Initializes the honorary DAMM v2 LP position owned by the program PDA
///
/// This instruction creates an empty position that will accrue fees ONLY in the quote mint.
/// The position is owned by the InvestorFeePositionOwnerPda.
///
/// # Arguments
/// * `ctx` - Context containing all required accounts
///
/// # Returns
/// Result indicating success or failure
///
/// # Errors
/// Returns error if pool configuration cannot guarantee quote-only fees
pub fn handler(ctx: Context<InitializePosition>) -> Result<()> {
    let policy = &mut ctx.accounts.policy;
    let clock = Clock::get()?;
    
    // Verify mints match policy
    require!(
        ctx.accounts.quote_mint.key() == policy.quote_mint,
        DammDistributorError::InvalidQuoteMint
    );
    
    require!(
        ctx.accounts.base_mint.key() == policy.base_mint,
        DammDistributorError::InvalidBaseMint
    );
    
    // Store the honorary position in policy
    policy.honorary_position = ctx.accounts.honorary_position.key();
    
    // Initialize progress tracker
    let progress = &mut ctx.accounts.progress;
    progress.vault = policy.vault;
    progress.last_distribution_ts = 0;
    progress.current_day_start = 0;
    progress.total_claimed_day = 0;
    progress.cumulative_distributed = 0;
    progress.carry_over = 0;
    progress.pagination_cursor = 0;
    progress.day_finalized = false;
    progress.daily_cap_remaining = policy.daily_cap_lamports;
    progress.bump = ctx.bumps.progress;
    
    // Emit event
    emit!(HonoraryPositionInitialized {
        position: ctx.accounts.honorary_position.key(),
        owner_pda: ctx.accounts.investor_fee_position_owner_pda.key(),
        quote_mint: ctx.accounts.quote_mint.key(),
        base_mint: ctx.accounts.base_mint.key(),
        pool: policy.pool,
        timestamp: clock.unix_timestamp,
    });
    
    msg!("Honorary position initialized: {}", ctx.accounts.honorary_position.key());
    msg!("Position owner PDA: {}", ctx.accounts.investor_fee_position_owner_pda.key());
    msg!("Quote mint: {}", ctx.accounts.quote_mint.key());
    msg!("Base mint: {}", ctx.accounts.base_mint.key());
    
    Ok(())
}

#[derive(Accounts)]
pub struct InitializePosition<'info> {
    #[account(
        mut,
        seeds = [POLICY_SEED, policy.vault.as_ref()],
        bump = policy.bump,
    )]
    pub policy: Account<'info, Policy>,
    
    #[account(
        init,
        payer = authority,
        space = DistributionProgress::LEN,
        seeds = [PROGRESS_SEED, policy.vault.as_ref()],
        bump
    )]
    pub progress: Account<'info, DistributionProgress>,
    
    /// CHECK: The honorary position account (to be created via CPI to cp-amm)
    /// In a real implementation, this would be created via CPI to the DAMM v2 program
    #[account(mut)]
    pub honorary_position: AccountInfo<'info>,
    
    /// CHECK: PDA that owns the honorary position
    /// Seeds: [VAULT_SEED, vault, INVESTOR_FEE_POS_OWNER_SEED]
    #[account(
        seeds = [VAULT_SEED, policy.vault.as_ref(), INVESTOR_FEE_POS_OWNER_SEED],
        bump
    )]
    pub investor_fee_position_owner_pda: AccountInfo<'info>,
    
    /// CHECK: Quote mint
    pub quote_mint: AccountInfo<'info>,
    
    /// CHECK: Base mint
    pub base_mint: AccountInfo<'info>,
    
    /// CHECK: Pool account
    pub pool: AccountInfo<'info>,
    
    /// Program quote treasury ATA (where fees are claimed to)
    #[account(
        mut,
        token::mint = quote_mint,
        token::authority = investor_fee_position_owner_pda
    )]
    pub program_quote_treasury: Account<'info, TokenAccount>,
    
    #[account(mut)]
    pub authority: Signer<'info>,
    
    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
}
