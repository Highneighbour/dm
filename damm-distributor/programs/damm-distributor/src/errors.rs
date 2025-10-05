/// Custom error codes for the DAMM Distributor program
use anchor_lang::prelude::*;

#[error_code]
pub enum DammDistributorError {
    #[msg("Base fees detected - quote-only position violated")]
    BaseFeeDetected,
    
    #[msg("24-hour window not elapsed since last distribution")]
    TooEarlyForNextDistribution,
    
    #[msg("Daily cap exceeded")]
    DailyCapExceeded,
    
    #[msg("Invalid quote mint")]
    InvalidQuoteMint,
    
    #[msg("Invalid base mint")]
    InvalidBaseMint,
    
    #[msg("Pagination cursor out of bounds")]
    InvalidPaginationCursor,
    
    #[msg("Arithmetic overflow occurred")]
    ArithmeticOverflow,
    
    #[msg("Arithmetic underflow occurred")]
    ArithmeticUnderflow,
    
    #[msg("Division by zero")]
    DivisionByZero,
    
    #[msg("Invalid pool configuration")]
    InvalidPoolConfig,
    
    #[msg("Position not initialized")]
    PositionNotInitialized,
    
    #[msg("Day already finalized")]
    DayAlreadyFinalized,
    
    #[msg("Invalid investor fee share (must be 0-10000 bps)")]
    InvalidInvestorFeeShare,
    
    #[msg("Invalid Streamflow stream account")]
    InvalidStreamflowStream,
    
    #[msg("No locked tokens remaining")]
    NoLockedTokens,
    
    #[msg("Token account not found")]
    TokenAccountNotFound,
    
    #[msg("Insufficient funds")]
    InsufficientFunds,
    
    #[msg("Invalid owner")]
    InvalidOwner,
    
    #[msg("Day not finalized yet")]
    DayNotFinalized,
    
    #[msg("Cannot proceed with this page - previous page not completed")]
    PreviousPageNotCompleted,
}
