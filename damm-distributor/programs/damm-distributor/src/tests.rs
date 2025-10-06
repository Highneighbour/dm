/// Unit tests for DAMM Distributor
#[cfg(test)]
mod tests {
    use crate::utils::*;
    use crate::errors::DammDistributorError;

    #[test]
    fn test_checked_add_success() {
        let result = checked_add(100, 200);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 300);
    }

    #[test]
    fn test_checked_add_overflow() {
        let result = checked_add(u64::MAX, 1);
        assert!(result.is_err());
    }

    #[test]
    fn test_checked_mul_success() {
        let result = checked_mul(100, 200);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 20000);
    }

    #[test]
    fn test_checked_mul_overflow() {
        let result = checked_mul(u64::MAX, 2);
        assert!(result.is_err());
    }

    #[test]
    fn test_checked_sub_success() {
        let result = checked_sub(200, 100);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 100);
    }

    #[test]
    fn test_checked_sub_underflow() {
        let result = checked_sub(100, 200);
        assert!(result.is_err());
    }

    #[test]
    fn test_checked_div_success() {
        let result = checked_div(200, 2);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 100);
    }

    #[test]
    fn test_checked_div_zero() {
        let result = checked_div(100, 0);
        assert!(result.is_err());
    }

    #[test]
    fn test_proportional_floor_basic() {
        // 100 * 3 / 10 = 30
        let result = proportional_floor(100, 3, 10);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 30);
    }

    #[test]
    fn test_proportional_floor_with_remainder() {
        // 1000 * 3 / 7 = 428 (floors from 428.57...)
        let result = proportional_floor(1000, 3, 7);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 428);
    }

    #[test]
    fn test_proportional_floor_zero_denominator() {
        let result = proportional_floor(100, 3, 0);
        assert!(result.is_err());
    }

    #[test]
    fn test_bps_floor_basic() {
        // 1000 * 500 / 10000 = 50 (5%)
        let result = bps_floor(1000, 500);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 50);
    }

    #[test]
    fn test_bps_floor_60_percent() {
        // 1000 * 6000 / 10000 = 600 (60%)
        let result = bps_floor(1000, 6000);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 600);
    }

    #[test]
    fn test_bps_floor_100_percent() {
        // 1000 * 10000 / 10000 = 1000 (100%)
        let result = bps_floor(1000, 10000);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 1000);
    }

    #[test]
    fn test_validate_bps_valid() {
        assert!(validate_bps(0).is_ok());
        assert!(validate_bps(5000).is_ok());
        assert!(validate_bps(10000).is_ok());
    }

    #[test]
    fn test_validate_bps_invalid() {
        assert!(validate_bps(10001).is_err());
        assert!(validate_bps(20000).is_err());
    }

    // Test the exact mathematical formulas from the specification
    #[test]
    fn test_spec_example_calculation() {
        // Given from specification:
        // Y0 = 10,000 tokens
        // investor_fee_share_bps = 6,000 (60%)
        // claimed_quote = 1,000 tokens
        // Investors: [1,000 locked, 2,000 locked, 3,000 locked]
        
        let y0 = 10000u64;
        let investor_fee_share_bps = 6000u16;
        let claimed_quote = 1000u64;
        let locked_amounts = vec![1000u64, 2000u64, 3000u64];
        
        // Step 1: Calculate locked_total
        let locked_total: u64 = locked_amounts.iter().sum();
        assert_eq!(locked_total, 6000);
        
        // Step 2: Calculate f_locked_bps
        let f_locked_bps = proportional_floor(locked_total, 10000, y0).unwrap();
        assert_eq!(f_locked_bps, 6000);
        
        // Step 3: Determine eligible_investor_share_bps
        let eligible_bps = std::cmp::min(investor_fee_share_bps as u64, f_locked_bps);
        assert_eq!(eligible_bps, 6000);
        
        // Step 4: Calculate investor_fee_quote
        let investor_fee_quote = bps_floor(claimed_quote, eligible_bps as u16).unwrap();
        assert_eq!(investor_fee_quote, 600);
        
        // Step 5: Calculate per-investor payouts
        let payout_1 = proportional_floor(investor_fee_quote, locked_amounts[0], locked_total).unwrap();
        let payout_2 = proportional_floor(investor_fee_quote, locked_amounts[1], locked_total).unwrap();
        let payout_3 = proportional_floor(investor_fee_quote, locked_amounts[2], locked_total).unwrap();
        
        assert_eq!(payout_1, 100); // 1000/6000 * 600 = 100
        assert_eq!(payout_2, 200); // 2000/6000 * 600 = 200
        assert_eq!(payout_3, 300); // 3000/6000 * 600 = 300
        
        // Step 6: Calculate creator remainder
        let total_to_investors = payout_1 + payout_2 + payout_3;
        assert_eq!(total_to_investors, 600);
        
        let creator_remainder = claimed_quote - total_to_investors;
        assert_eq!(creator_remainder, 400);
        
        // Verify accounting invariant
        assert_eq!(total_to_investors + creator_remainder, claimed_quote);
    }

    #[test]
    fn test_all_unlocked_scenario() {
        // When locked_total = 0, no fees should go to investors
        let y0 = 10000u64;
        let locked_total = 0u64;
        let claimed_quote = 1000u64;
        
        // f_locked_bps = 0
        let f_locked_bps = if locked_total == 0 {
            0
        } else {
            proportional_floor(locked_total, 10000, y0).unwrap()
        };
        assert_eq!(f_locked_bps, 0);
        
        // eligible_bps = min(6000, 0) = 0
        let eligible_bps = std::cmp::min(6000u64, f_locked_bps);
        assert_eq!(eligible_bps, 0);
        
        // investor_fee_quote = 0
        let investor_fee_quote = bps_floor(claimed_quote, eligible_bps as u16).unwrap();
        assert_eq!(investor_fee_quote, 0);
        
        // All goes to creator
        let creator_remainder = claimed_quote - investor_fee_quote;
        assert_eq!(creator_remainder, 1000);
    }

    #[test]
    fn test_partial_lock_scaling() {
        // Test that investor share scales down as tokens unlock
        let y0 = 10000u64;
        let investor_fee_share_bps = 8000u16; // 80% max
        
        // Scenario 1: 50% locked
        let locked_total_50 = 5000u64;
        let f_locked_bps_50 = proportional_floor(locked_total_50, 10000, y0).unwrap();
        assert_eq!(f_locked_bps_50, 5000);
        let eligible_bps_50 = std::cmp::min(investor_fee_share_bps as u64, f_locked_bps_50);
        assert_eq!(eligible_bps_50, 5000); // Capped by lock percentage
        
        // Scenario 2: 90% locked
        let locked_total_90 = 9000u64;
        let f_locked_bps_90 = proportional_floor(locked_total_90, 10000, y0).unwrap();
        assert_eq!(f_locked_bps_90, 9000);
        let eligible_bps_90 = std::cmp::min(investor_fee_share_bps as u64, f_locked_bps_90);
        assert_eq!(eligible_bps_90, 8000); // Capped by investor_fee_share_bps
        
        // Verify that higher lock percentage doesn't exceed max share
        assert!(eligible_bps_90 <= investor_fee_share_bps as u64);
    }

    #[test]
    fn test_dust_threshold_logic() {
        // Small payout that would be below dust threshold
        let investor_fee_quote = 100u64;
        let locked_i = 1u64;
        let locked_total = 1000u64;
        let min_payout = 10u64;
        
        let payout = proportional_floor(investor_fee_quote, locked_i, locked_total).unwrap();
        // payout = floor(100 * 1 / 1000) = 0
        
        // This would be below dust threshold and should be carried over
        assert!(payout < min_payout);
    }

    #[test]
    fn test_rounding_consistency() {
        // Verify that floor division is consistent
        let total = 1000u64;
        let locked_amounts = vec![333u64, 333u64, 334u64];
        let locked_total: u64 = locked_amounts.iter().sum();
        assert_eq!(locked_total, 1000);
        
        let payout_1 = proportional_floor(total, locked_amounts[0], locked_total).unwrap();
        let payout_2 = proportional_floor(total, locked_amounts[1], locked_total).unwrap();
        let payout_3 = proportional_floor(total, locked_amounts[2], locked_total).unwrap();
        
        // Each should get floor of their share
        assert_eq!(payout_1, 333);
        assert_eq!(payout_2, 333);
        assert_eq!(payout_3, 334);
        
        // Total should equal or be slightly less than input due to floor
        let total_paid = payout_1 + payout_2 + payout_3;
        assert_eq!(total_paid, 1000);
    }

    #[test]
    fn test_large_numbers() {
        // Test with realistic token amounts (millions of tokens)
        let y0 = 1_000_000_000_000u64; // 1M tokens with 6 decimals
        let locked_total = 600_000_000_000u64; // 600K locked
        let claimed_quote = 100_000_000u64; // 100 tokens fees
        
        let f_locked_bps = proportional_floor(locked_total, 10000, y0).unwrap();
        assert_eq!(f_locked_bps, 6000); // 60% locked
        
        let eligible_bps = std::cmp::min(6000u64, f_locked_bps);
        let investor_fee_quote = bps_floor(claimed_quote, eligible_bps as u16).unwrap();
        assert_eq!(investor_fee_quote, 60_000_000); // 60% of fees
        
        let creator_remainder = claimed_quote - investor_fee_quote;
        assert_eq!(creator_remainder, 40_000_000); // 40% to creator
    }
}
