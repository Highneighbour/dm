/// Crank distribution instruction - distributes quote fees to investors
use anchor_lang::prelude::*;
use anchor_spl::token::{self, Token, TokenAccount, Transfer};
use crate::constants::*;
use crate::errors::DammDistributorError;
use crate::state::*;
use crate::events::*;
use crate::utils::*;

/// Permissionless crank that claims and distributes quote fees
///
/// This instruction can be called once per 24-hour window, with support for pagination
/// across multiple transactions within the same day.
///
/// # Arguments
/// * `ctx` - Context containing all required accounts
/// * `page_index` - Current page index for pagination
/// * `is_final_page` - Whether this is the final page of the day
///
/// # Returns
/// Result indicating success or failure
///
/// # Errors
/// Returns various errors for validation failures, arithmetic errors, or policy violations
pub fn handler<'info>(
    ctx: Context<'_, '_, '_, 'info, CrankDistribution<'info>>,
    page_index: u32,
    is_final_page: bool,
) -> Result<()> {
    let policy = &ctx.accounts.policy;
    let clock = Clock::get()?;
    let now = clock.unix_timestamp;
    
    // Get the bump for the investor fee position owner PDA
    let investor_fee_pos_owner_bump = ctx.bumps.investor_fee_position_owner_pda;
    
    // Check if this is a new day or continuation of current day
    if ctx.accounts.progress.current_day_start == 0 || ctx.accounts.progress.day_finalized {
        // Starting a new day - check 24h gating
        require!(
            now >= ctx.accounts.progress.last_distribution_ts.checked_add(SECONDS_PER_DAY)
                .ok_or(DammDistributorError::ArithmeticOverflow)?,
            DammDistributorError::TooEarlyForNextDistribution
        );
        
        // Claim fees from honorary position (simulated for now)
        // In real implementation, this would be a CPI to cp-amm
        let claimed_amount = ctx.accounts.program_quote_treasury.amount;
        
        // Verify NO base fees were claimed (simulated)
        // In real implementation, check if any base tokens were received
        
        let progress = &mut ctx.accounts.progress;
        
        // Reset for new day
        progress.current_day_start = now;
        progress.total_claimed_day = claimed_amount;
        progress.cumulative_distributed = 0;
        progress.pagination_cursor = 0;
        progress.day_finalized = false;
        progress.daily_cap_remaining = policy.daily_cap_lamports;
        
        // Emit claim event
        emit!(QuoteFeesClaimed {
            amount: claimed_amount,
            timestamp: now,
            day_start: progress.current_day_start,
        });
        
        msg!("Claimed {} quote fees for day starting at {}", claimed_amount, progress.current_day_start);
    } else {
        // Continuation of current day - verify page index matches
        require!(
            page_index == ctx.accounts.progress.pagination_cursor,
            DammDistributorError::InvalidPaginationCursor
        );
    }
    
    // Process investor distributions for this page
    let (investors_paid, total_distributed_page) = distribute_to_investors(
        &ctx.remaining_accounts,
        &mut ctx.accounts.progress,
        policy,
        &ctx.accounts.program_quote_treasury,
        &ctx.accounts.investor_fee_position_owner_pda,
        &ctx.accounts.token_program,
        investor_fee_pos_owner_bump,
    )?;
    
    let progress = &mut ctx.accounts.progress;
    
    // Update progress
    progress.cumulative_distributed = checked_add(
        progress.cumulative_distributed,
        total_distributed_page
    )?;
    progress.pagination_cursor = checked_add(progress.pagination_cursor as u64, 1)? as u32;
    
    // Emit page event
    emit!(InvestorPayoutPage {
        page_index,
        investors_paid,
        total_amount_page: total_distributed_page,
        cumulative_day_amount: progress.cumulative_distributed,
        timestamp: now,
    });
    
    msg!("Page {} complete: {} investors paid, {} distributed", 
        page_index, investors_paid, total_distributed_page);
    
    // If this is the final page, route remainder to creator
    if is_final_page {
        let remainder = route_remainder_to_creator(
            &ctx.accounts.program_quote_treasury,
            &ctx.accounts.creator_quote_ata,
            &ctx.accounts.investor_fee_position_owner_pda,
            &ctx.accounts.token_program,
            progress,
            policy,
            investor_fee_pos_owner_bump,
        )?;
        
        // Finalize the day
        progress.day_finalized = true;
        progress.last_distribution_ts = progress.current_day_start;
        
        // Emit day closed event
        emit!(CreatorPayoutDayClosed {
            creator: policy.creator_quote_ata,
            remainder_amount: remainder,
            total_claimed_day: progress.total_claimed_day,
            total_to_investors: progress.cumulative_distributed,
            day_start: progress.current_day_start,
            day_end: now,
        });
        
        msg!("Day finalized: {} to investors, {} to creator", 
            progress.cumulative_distributed, remainder);
    }
    
    Ok(())
}

/// Distributes fees to investors pro-rata based on locked amounts
fn distribute_to_investors<'info>(
    remaining_accounts: &[AccountInfo<'info>],
    progress: &mut DistributionProgress,
    policy: &Policy,
    program_quote_treasury: &Account<'info, TokenAccount>,
    investor_fee_position_owner_pda: &AccountInfo<'info>,
    token_program: &Program<'info, Token>,
    investor_fee_pos_owner_bump: u8,
) -> Result<(u32, u64)> {
    
    // Remaining accounts come in pairs: [stream_account, investor_ata]
    let num_investors = remaining_accounts.len() / 2;
    
    if num_investors == 0 {
        return Ok((0, 0));
    }
    
    // Step 1: Read locked amounts from all investors on this page
    let mut locked_amounts: Vec<u64> = Vec::new();
    let mut locked_total: u64 = 0;
    
    for i in 0..num_investors {
        let stream_account = &remaining_accounts[i * 2];
        let locked_amount = read_locked_amount_from_streamflow(stream_account)?;
        locked_amounts.push(locked_amount);
        locked_total = checked_add(locked_total, locked_amount)?;
    }
    
    // If no locked tokens, no distribution
    if locked_total == 0 {
        return Ok((0, 0));
    }
    
    // Step 2: Calculate eligible investor share
    // f_locked(t) = locked_total(t) / Y0
    // eligible_investor_share_bps = min(investor_fee_share_bps, floor(f_locked(t) * 10000))
    let f_locked_bps = proportional_floor(locked_total, 10000, policy.total_investor_allocation)?;
    let eligible_investor_share_bps = std::cmp::min(
        policy.investor_fee_share_bps as u64,
        f_locked_bps
    );
    
    // Step 3: Calculate total amount available for investors
    let total_claimed = progress.total_claimed_day;
    let investor_fee_quote = bps_floor(total_claimed, eligible_investor_share_bps as u16)?;
    
    // Add carry-over from previous page
    let available_for_distribution = checked_add(investor_fee_quote, progress.carry_over)?;
    
    // Step 4: Distribute to each investor proportionally
    let mut total_distributed_page: u64 = 0;
    let mut investors_paid: u32 = 0;
    
    let vault_key = policy.vault;
    let investor_fee_pos_owner_seeds = &[
        VAULT_SEED,
        vault_key.as_ref(),
        INVESTOR_FEE_POS_OWNER_SEED,
        &[investor_fee_pos_owner_bump],
    ];
    let signer_seeds = &[&investor_fee_pos_owner_seeds[..]];
    
    for i in 0..num_investors {
        let locked_i = locked_amounts[i];
        if locked_i == 0 {
            continue;
        }
        
        // Calculate pro-rata share: weight_i = locked_i / locked_total
        // payout_i = floor(investor_fee_quote * weight_i)
        let payout = proportional_floor(available_for_distribution, locked_i, locked_total)?;
        
        // Apply dust threshold
        if payout < policy.min_payout_lamports {
            // Carry forward
            continue;
        }
        
        // Apply daily cap if configured
        if policy.daily_cap_lamports > 0 {
            if payout > progress.daily_cap_remaining {
                // Hit cap, carry forward excess
                continue;
            }
        }
        
        // Transfer to investor
        let investor_ata = &remaining_accounts[i * 2 + 1];
        
        let cpi_accounts = Transfer {
            from: program_quote_treasury.to_account_info(),
            to: investor_ata.clone(),
            authority: investor_fee_position_owner_pda.to_account_info(),
        };
        let cpi_program = token_program.to_account_info();
        let cpi_ctx = CpiContext::new_with_signer(cpi_program, cpi_accounts, signer_seeds);
        
        token::transfer(cpi_ctx, payout)?;
        
        total_distributed_page = checked_add(total_distributed_page, payout)?;
        investors_paid = checked_add(investors_paid as u64, 1)? as u32;
        
        // Update daily cap remaining
        if policy.daily_cap_lamports > 0 {
            progress.daily_cap_remaining = checked_sub(progress.daily_cap_remaining, payout)?;
        }
    }
    
    // Update carry-over for next page/day
    progress.carry_over = checked_sub(available_for_distribution, total_distributed_page)?;
    
    Ok((investors_paid, total_distributed_page))
}

/// Routes remaining fees to creator
fn route_remainder_to_creator<'info>(
    program_quote_treasury: &Account<'info, TokenAccount>,
    creator_quote_ata: &Account<'info, TokenAccount>,
    investor_fee_position_owner_pda: &AccountInfo<'info>,
    token_program: &Program<'info, Token>,
    progress: &DistributionProgress,
    policy: &Policy,
    investor_fee_pos_owner_bump: u8,
) -> Result<u64> {
    // Calculate remainder: total_claimed - cumulative_distributed
    let remainder = checked_sub(progress.total_claimed_day, progress.cumulative_distributed)?;
    
    if remainder == 0 {
        return Ok(0);
    }
    
    let vault_key = policy.vault;
    let investor_fee_pos_owner_seeds = &[
        VAULT_SEED,
        vault_key.as_ref(),
        INVESTOR_FEE_POS_OWNER_SEED,
        &[investor_fee_pos_owner_bump],
    ];
    let signer_seeds = &[&investor_fee_pos_owner_seeds[..]];
    
    let cpi_accounts = Transfer {
        from: program_quote_treasury.to_account_info(),
        to: creator_quote_ata.to_account_info(),
        authority: investor_fee_position_owner_pda.to_account_info(),
    };
    let cpi_program = token_program.to_account_info();
    let cpi_ctx = CpiContext::new_with_signer(cpi_program, cpi_accounts, signer_seeds);
    
    token::transfer(cpi_ctx, remainder)?;
    
    Ok(remainder)
}

/// Reads locked amount from Streamflow stream account
///
/// In a real implementation, this would parse the Streamflow stream account data.
/// For testing, we'll provide a mock implementation.
fn read_locked_amount_from_streamflow(stream_account: &AccountInfo) -> Result<u64> {
    // In real implementation:
    // 1. Verify account is owned by Streamflow program
    // 2. Deserialize stream data
    // 3. Calculate: initial_amount - withdrawn_amount - (rate * elapsed_time)
    // 4. Return max(0, locked_amount)
    
    // For mock/testing: we'll read from account data if available
    // Expected format: first 8 bytes = locked amount (u64 little-endian)
    if stream_account.data_len() >= 8 {
        let data = stream_account.try_borrow_data()?;
        let locked_amount = u64::from_le_bytes([
            data[0], data[1], data[2], data[3],
            data[4], data[5], data[6], data[7],
        ]);
        Ok(locked_amount)
    } else {
        Ok(0)
    }
}

#[derive(Accounts)]
pub struct CrankDistribution<'info> {
    #[account(
        seeds = [POLICY_SEED, policy.vault.as_ref()],
        bump = policy.bump,
    )]
    pub policy: Account<'info, Policy>,
    
    #[account(
        mut,
        seeds = [PROGRESS_SEED, policy.vault.as_ref()],
        bump = progress.bump,
    )]
    pub progress: Account<'info, DistributionProgress>,
    
    /// CHECK: The honorary position account
    pub honorary_position: AccountInfo<'info>,
    
    /// CHECK: PDA that owns the honorary position
    #[account(
        seeds = [VAULT_SEED, policy.vault.as_ref(), INVESTOR_FEE_POS_OWNER_SEED],
        bump
    )]
    pub investor_fee_position_owner_pda: AccountInfo<'info>,
    
    /// Program quote treasury ATA (where fees are claimed to and distributed from)
    #[account(
        mut,
        token::mint = policy.quote_mint,
        token::authority = investor_fee_position_owner_pda
    )]
    pub program_quote_treasury: Account<'info, TokenAccount>,
    
    /// Creator's quote ATA (receives remainder)
    #[account(
        mut,
        token::mint = policy.quote_mint,
    )]
    pub creator_quote_ata: Account<'info, TokenAccount>,
    
    pub token_program: Program<'info, Token>,
    // Remaining accounts (accessed via ctx.remaining_accounts):
    // [stream_account_0, investor_ata_0, stream_account_1, investor_ata_1, ...]
    // Each pair represents an investor's Streamflow stream and their quote token ATA
}
