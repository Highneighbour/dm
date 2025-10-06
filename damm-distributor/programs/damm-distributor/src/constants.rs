/// Constants for the DAMM Distributor program

/// Seed for the vault
pub const VAULT_SEED: &[u8] = b"vault";

/// Seed for the investor fee position owner PDA
pub const INVESTOR_FEE_POS_OWNER_SEED: &[u8] = b"investor_fee_pos_owner";

/// Seed for the policy account
pub const POLICY_SEED: &[u8] = b"policy";

/// Seed for the distribution progress account
pub const PROGRESS_SEED: &[u8] = b"progress";

/// Number of seconds in 24 hours
pub const SECONDS_PER_DAY: i64 = 86400;

/// Maximum basis points (100%)
pub const MAX_BPS: u16 = 10000;
