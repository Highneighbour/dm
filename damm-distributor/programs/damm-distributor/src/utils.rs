/// Utility functions for the DAMM Distributor program
use anchor_lang::prelude::*;
use crate::errors::DammDistributorError;

/// Performs checked multiplication
///
/// # Arguments
/// * `a` - First operand
/// * `b` - Second operand
///
/// # Returns
/// Result containing the product or an error
///
/// # Errors
/// Returns `ArithmeticOverflow` if multiplication overflows
pub fn checked_mul(a: u64, b: u64) -> Result<u64> {
    a.checked_mul(b)
        .ok_or(DammDistributorError::ArithmeticOverflow.into())
}

/// Performs checked division
///
/// # Arguments
/// * `a` - Numerator
/// * `b` - Denominator
///
/// # Returns
/// Result containing the quotient or an error
///
/// # Errors
/// Returns `DivisionByZero` if denominator is zero
pub fn checked_div(a: u64, b: u64) -> Result<u64> {
    if b == 0 {
        return Err(DammDistributorError::DivisionByZero.into());
    }
    Ok(a / b)
}

/// Performs checked addition
///
/// # Arguments
/// * `a` - First operand
/// * `b` - Second operand
///
/// # Returns
/// Result containing the sum or an error
///
/// # Errors
/// Returns `ArithmeticOverflow` if addition overflows
pub fn checked_add(a: u64, b: u64) -> Result<u64> {
    a.checked_add(b)
        .ok_or(DammDistributorError::ArithmeticOverflow.into())
}

/// Performs checked subtraction
///
/// # Arguments
/// * `a` - Minuend
/// * `b` - Subtrahend
///
/// # Returns
/// Result containing the difference or an error
///
/// # Errors
/// Returns `ArithmeticUnderflow` if subtraction would result in negative value
pub fn checked_sub(a: u64, b: u64) -> Result<u64> {
    a.checked_sub(b)
        .ok_or(DammDistributorError::ArithmeticUnderflow.into())
}

/// Calculates proportional amount using floor division
///
/// # Arguments
/// * `total` - Total amount to distribute from
/// * `numerator` - Numerator for proportion
/// * `denominator` - Denominator for proportion
///
/// # Returns
/// Floor of (total * numerator) / denominator
///
/// # Errors
/// Returns error if arithmetic operations fail
///
/// # Example
/// ```ignore
/// let amount = proportional_floor(1000, 3, 10)?; // Returns 300
/// ```
pub fn proportional_floor(total: u64, numerator: u64, denominator: u64) -> Result<u64> {
    if denominator == 0 {
        return Err(DammDistributorError::DivisionByZero.into());
    }
    
    let product = checked_mul(total, numerator)?;
    Ok(product / denominator)
}

/// Calculates basis points amount using floor division
///
/// # Arguments
/// * `amount` - Base amount
/// * `bps` - Basis points (out of 10000)
///
/// # Returns
/// Floor of (amount * bps) / 10000
///
/// # Errors
/// Returns error if arithmetic operations fail
///
/// # Example
/// ```ignore
/// let fee = bps_floor(1000, 500)?; // Returns 50 (5% of 1000)
/// ```
pub fn bps_floor(amount: u64, bps: u16) -> Result<u64> {
    proportional_floor(amount, bps as u64, 10000)
}

/// Validates that a basis points value is within valid range (0-10000)
///
/// # Arguments
/// * `bps` - Basis points value to validate
///
/// # Returns
/// Ok if valid, error otherwise
///
/// # Errors
/// Returns `InvalidInvestorFeeShare` if bps > 10000
pub fn validate_bps(bps: u16) -> Result<()> {
    if bps > 10000 {
        return Err(DammDistributorError::InvalidInvestorFeeShare.into());
    }
    Ok(())
}
