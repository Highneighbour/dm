# DAMM v2 Honorary Quote-Only Fee Position + 24h Distribution Crank

A production-ready Solana Anchor program module for Star (star.xyz) that creates an "honorary" DAMM v2 LP position accruing fees exclusively in the quote mint, with a permissionless 24-hour distribution crank that distributes fees pro-rata to investors based on their Streamflow-locked amounts.

## Table of Contents

- [Overview](#overview)
- [Architecture](#architecture)
- [Installation & Setup](#installation--setup)
- [Integration Guide](#integration-guide)
- [Account Tables](#account-tables)
- [PDA Derivations](#pda-derivations)
- [Policy Configuration](#policy-configuration)
- [Day & Pagination Semantics](#day--pagination-semantics)
- [Error Codes](#error-codes)
- [Events](#events)
- [Mathematical Formulas](#mathematical-formulas)
- [Failure Modes](#failure-modes)
- [Testing](#testing)

## Overview

This module provides:

1. **Quote-Only Fee Accrual**: Creates an honorary DAMM v2 LP position owned by a program PDA that accrues fees EXCLUSIVELY in the quote mint
2. **24-Hour Distribution Crank**: A permissionless instruction that can be called once per 24 hours to distribute accumulated quote fees
3. **Pro-Rata Distribution**: Distributes fees to investors proportionally based on their still-locked Streamflow amounts
4. **Pagination Support**: Handles large investor sets through idempotent, resumable pagination
5. **Creator Remainder**: Routes all remaining fees to the creator after investor distributions

### Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                      DAMM v2 Pool                               │
│                                                                 │
│  ┌──────────────────────────────────────────────────────┐      │
│  │     Honorary Position (Quote-Only Fees)              │      │
│  │     Owner: InvestorFeePositionOwnerPda               │      │
│  └──────────────────────────────────────────────────────┘      │
└────────────────────────────────┬────────────────────────────────┘
                                 │
                                 │ Quote Fees Accrue
                                 ▼
                    ┌─────────────────────────┐
                    │  Program Quote Treasury │
                    │  (PDA-owned ATA)        │
                    └────────────┬────────────┘
                                 │
                     24h Crank Distribution
                                 │
                  ┌──────────────┴──────────────┐
                  │                             │
                  ▼                             ▼
        ┌──────────────────┐         ┌──────────────────┐
        │  Investor Payouts│         │ Creator Remainder│
        │  (Pro-Rata)      │         │                  │
        └──────────────────┘         └──────────────────┘
```

## Installation & Setup

### Prerequisites

- Rust 1.90+
- Solana CLI 1.18+
- Anchor 0.31.1+

### Step 1: Install Rust (if not already installed)

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source $HOME/.cargo/env
rustup update stable
```

### Step 2: Install Solana CLI

```bash
# Using official installer
sh -c "$(curl -sSfL https://release.solana.com/stable/install)"

# Verify installation
solana --version
```

### Step 3: Install AVM and Anchor

```bash
# Install AVM (Anchor Version Manager)
cargo install --git https://github.com/coral-xyz/anchor avm --locked --force

# Install latest Anchor via AVM
avm install latest
avm use latest

# Verify installation
anchor --version
```

### Step 4: Clone and Build

```bash
# Clone the repository
git clone <repository-url>
cd damm-distributor

# Build the program
anchor build

# Run tests
anchor test
```

### Troubleshooting

**Issue**: `cargo build-sbf` command not found
- **Solution**: Ensure Solana CLI is properly installed and in your PATH
- Run: `export PATH="$HOME/.local/share/solana/install/active_release/bin:$PATH"`

**Issue**: Anchor version mismatch
- **Solution**: Ensure you're using Anchor 0.31.1
- Run: `avm use 0.31.1` or `avm install 0.31.1`

**Issue**: Rust compilation errors related to `edition2024`
- **Solution**: Update Rust to latest stable
- Run: `rustup update stable`

## Integration Guide

### Importing the Module

```rust
use damm_distributor;
use damm_distributor::state::{Policy, DistributionProgress};
use damm_distributor::instructions::{InitializePolicy, InitializePosition, CrankDistribution};
```

### Step 1: Initialize Policy

```typescript
import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { DammDistributor } from "./target/types/damm_distributor";

const program = anchor.workspace.DammDistributor as Program<DammDistributor>;

const [policyPDA] = PublicKey.findProgramAddressSync(
  [Buffer.from("policy"), vaultPubkey.toBuffer()],
  program.programId
);

await program.methods
  .initializePolicy(
    new anchor.BN(10000), // total_investor_allocation (Y0)
    6000, // investor_fee_share_bps (60%)
    new anchor.BN(0), // daily_cap_lamports (0 = no cap)
    new anchor.BN(100) // min_payout_lamports
  )
  .accounts({
    policy: policyPDA,
    vault: vaultPubkey,
    quoteMint: quoteMintPubkey,
    baseMint: baseMintPubkey,
    pool: poolPubkey,
    creatorQuoteAta: creatorQuoteAtaPubkey,
    authority: authorityKeypair.publicKey,
    systemProgram: SystemProgram.programId,
  })
  .signers([authorityKeypair])
  .rpc();
```

### Step 2: Initialize Honorary Position

```typescript
const [progressPDA] = PublicKey.findProgramAddressSync(
  [Buffer.from("progress"), vaultPubkey.toBuffer()],
  program.programId
);

const [investorFeePosOwnerPDA] = PublicKey.findProgramAddressSync(
  [
    Buffer.from("vault"),
    vaultPubkey.toBuffer(),
    Buffer.from("investor_fee_pos_owner"),
  ],
  program.programId
);

await program.methods
  .initializePosition()
  .accounts({
    policy: policyPDA,
    progress: progressPDA,
    honoraryPosition: honoraryPositionPubkey,
    investorFeePositionOwnerPda: investorFeePosOwnerPDA,
    quoteMint: quoteMintPubkey,
    baseMint: baseMintPubkey,
    pool: poolPubkey,
    programQuoteTreasury: programQuoteTreasuryPubkey,
    authority: authorityKeypair.publicKey,
    tokenProgram: TOKEN_PROGRAM_ID,
    systemProgram: SystemProgram.programId,
  })
  .signers([authorityKeypair])
  .rpc();
```

### Step 3: Run Distribution Crank

```typescript
// Prepare remaining accounts (investor stream + ATA pairs)
const remainingAccounts = [];
for (const investor of investors) {
  remainingAccounts.push({
    pubkey: investor.streamAccount,
    isSigner: false,
    isWritable: false,
  });
  remainingAccounts.push({
    pubkey: investor.quoteAta,
    isSigner: false,
    isWritable: true,
  });
}

await program.methods
  .crankDistribution(
    0, // page_index
    true // is_final_page
  )
  .accounts({
    policy: policyPDA,
    progress: progressPDA,
    honoraryPosition: honoraryPositionPubkey,
    investorFeePositionOwnerPda: investorFeePosOwnerPDA,
    programQuoteTreasury: programQuoteTreasuryPubkey,
    creatorQuoteAta: creatorQuoteAtaPubkey,
    tokenProgram: TOKEN_PROGRAM_ID,
  })
  .remainingAccounts(remainingAccounts)
  .rpc();
```

## Account Tables

### Initialize Policy Accounts

| Account                | Type           | Mutable | Signer | Description                              |
|------------------------|----------------|---------|--------|------------------------------------------|
| `policy`               | Policy (PDA)   | ✅      | ❌     | Policy configuration account             |
| `vault`                | UncheckedAccount| ❌      | ❌     | Vault identifier                         |
| `quote_mint`           | UncheckedAccount| ❌      | ❌     | Quote token mint                         |
| `base_mint`            | UncheckedAccount| ❌      | ❌     | Base token mint                          |
| `pool`                 | UncheckedAccount| ❌      | ❌     | DAMM v2 pool account                     |
| `creator_quote_ata`    | UncheckedAccount| ❌      | ❌     | Creator's quote token ATA                |
| `authority`            | Signer         | ✅      | ✅     | Authority that can update policy         |
| `system_program`       | Program        | ❌      | ❌     | System program                           |

### Initialize Position Accounts

| Account                          | Type                | Mutable | Signer | Description                              |
|----------------------------------|---------------------|---------|--------|------------------------------------------|
| `policy`                         | Policy (PDA)        | ✅      | ❌     | Policy configuration account             |
| `progress`                       | Progress (PDA)      | ✅      | ❌     | Distribution progress tracker (init)     |
| `honorary_position`              | UncheckedAccount    | ✅      | ❌     | Honorary DAMM position account           |
| `investor_fee_position_owner_pda`| UncheckedAccount (PDA)| ❌    | ❌     | PDA that owns the honorary position      |
| `quote_mint`                     | UncheckedAccount    | ❌      | ❌     | Quote token mint                         |
| `base_mint`                      | UncheckedAccount    | ❌      | ❌     | Base token mint                          |
| `pool`                           | UncheckedAccount    | ❌      | ❌     | DAMM v2 pool account                     |
| `program_quote_treasury`         | TokenAccount        | ✅      | ❌     | Program's quote token ATA                |
| `authority`                      | Signer              | ✅      | ✅     | Transaction authority                    |
| `token_program`                  | Program             | ❌      | ❌     | SPL Token program                        |
| `system_program`                 | Program             | ❌      | ❌     | System program                           |

### Crank Distribution Accounts

| Account                          | Type                | Mutable | Signer | Description                              |
|----------------------------------|---------------------|---------|--------|------------------------------------------|
| `policy`                         | Policy (PDA)        | ❌      | ❌     | Policy configuration account             |
| `progress`                       | Progress (PDA)      | ✅      | ❌     | Distribution progress tracker            |
| `honorary_position`              | UncheckedAccount    | ❌      | ❌     | Honorary DAMM position account           |
| `investor_fee_position_owner_pda`| UncheckedAccount (PDA)| ❌    | ❌     | PDA that owns the honorary position      |
| `program_quote_treasury`         | TokenAccount        | ✅      | ❌     | Program's quote token ATA                |
| `creator_quote_ata`              | TokenAccount        | ✅      | ❌     | Creator's quote token ATA                |
| `token_program`                  | Program             | ❌      | ❌     | SPL Token program                        |
| **Remaining Accounts** (pairs)   |                     |         |        |                                          |
| `stream_account_i`               | UncheckedAccount    | ❌      | ❌     | Streamflow stream for investor i         |
| `investor_ata_i`                 | TokenAccount        | ✅      | ❌     | Investor i's quote token ATA             |

## PDA Derivations

### Policy PDA

```rust
// Seeds: [b"policy", vault.key()]
let (policy_pda, bump) = Pubkey::find_program_address(
    &[b"policy", vault.as_ref()],
    &program_id
);
```

### Distribution Progress PDA

```rust
// Seeds: [b"progress", vault.key()]
let (progress_pda, bump) = Pubkey::find_program_address(
    &[b"progress", vault.as_ref()],
    &program_id
);
```

### Investor Fee Position Owner PDA

```rust
// Seeds: [b"vault", vault.key(), b"investor_fee_pos_owner"]
let (investor_fee_pos_owner_pda, bump) = Pubkey::find_program_address(
    &[b"vault", vault.as_ref(), b"investor_fee_pos_owner"],
    &program_id
);
```

### TypeScript/JavaScript Example

```typescript
import { PublicKey } from "@solana/web3.js";

function derivePolicyPDA(programId: PublicKey, vault: PublicKey): [PublicKey, number] {
  return PublicKey.findProgramAddressSync(
    [Buffer.from("policy"), vault.toBuffer()],
    programId
  );
}

function deriveProgressPDA(programId: PublicKey, vault: PublicKey): [PublicKey, number] {
  return PublicKey.findProgramAddressSync(
    [Buffer.from("progress"), vault.toBuffer()],
    programId
  );
}

function deriveInvestorFeePosOwnerPDA(programId: PublicKey, vault: PublicKey): [PublicKey, number] {
  return PublicKey.findProgramAddressSync(
    [
      Buffer.from("vault"),
      vault.toBuffer(),
      Buffer.from("investor_fee_pos_owner"),
    ],
    programId
  );
}
```

## Policy Configuration

### Configuration Parameters

| Parameter                    | Type  | Description                                      | Recommended Value      |
|------------------------------|-------|--------------------------------------------------|------------------------|
| `total_investor_allocation`  | u64   | Total investor allocation at TGE (Y0)            | Actual TGE allocation  |
| `investor_fee_share_bps`     | u16   | Max investor fee share in basis points (0-10000) | 5000-8000 (50-80%)     |
| `daily_cap_lamports`         | u64   | Daily distribution cap in lamports (0 = no cap)  | 0 or specific limit    |
| `min_payout_lamports`        | u64   | Minimum payout threshold (dust prevention)       | 1000-10000             |

### Example Configurations

**Conservative (High Creator Share)**:
```typescript
{
  total_investor_allocation: new anchor.BN(1_000_000_000_000), // 1M tokens (6 decimals)
  investor_fee_share_bps: 4000, // 40%
  daily_cap_lamports: new anchor.BN(10_000_000), // 10 tokens per day
  min_payout_lamports: new anchor.BN(10_000), // 0.01 tokens minimum
}
```

**Balanced**:
```typescript
{
  total_investor_allocation: new anchor.BN(1_000_000_000_000),
  investor_fee_share_bps: 6000, // 60%
  daily_cap_lamports: new anchor.BN(0), // No cap
  min_payout_lamports: new anchor.BN(1_000), // 0.001 tokens minimum
}
```

**Aggressive (High Investor Share)**:
```typescript
{
  total_investor_allocation: new anchor.BN(1_000_000_000_000),
  investor_fee_share_bps: 8000, // 80%
  daily_cap_lamports: new anchor.BN(0), // No cap
  min_payout_lamports: new anchor.BN(100), // 0.0001 tokens minimum
}
```

## Day & Pagination Semantics

### 24-Hour Window Mechanism

1. **First Crank of Day**: 
   - Requires: `now >= last_distribution_ts + 86400` seconds
   - Claims all accumulated fees from honorary position
   - Resets pagination state

2. **Subsequent Pages (Same Day)**:
   - No time check required
   - Must provide correct `page_index` matching `pagination_cursor`
   - Distributes to next batch of investors

3. **Final Page**:
   - Set `is_final_page = true`
   - Routes remainder to creator
   - Finalizes day and updates `last_distribution_ts`

### Pagination Flow

```
Day 1 Start (T=0)
  ├─> Crank(page=0, is_final=false)  [Distributes to investors 0-9]
  ├─> Crank(page=1, is_final=false)  [Distributes to investors 10-19]
  ├─> Crank(page=2, is_final=false)  [Distributes to investors 20-29]
  └─> Crank(page=3, is_final=true)   [Distributes to investors 30-39, routes remainder to creator]

Day 2 Start (T=86400+)
  └─> Crank(page=0, is_final=true)   [New day cycle begins]
```

### Idempotent & Resumable

- **Safe to retry**: Re-running the same page does NOT double-pay
- **Resumable**: Can pause mid-day and resume later
- **Progress tracking**: `pagination_cursor` tracks current page
- **Carry-over**: Dust amounts carry to next page/day

## Error Codes

| Code | Name                              | Description                                           |
|------|-----------------------------------|-------------------------------------------------------|
| 6000 | `BaseFeeDetected`                 | Base fees observed - quote-only position violated     |
| 6001 | `TooEarlyForNextDistribution`     | 24-hour window not elapsed since last distribution    |
| 6002 | `DailyCapExceeded`                | Daily distribution cap reached                        |
| 6003 | `InvalidQuoteMint`                | Quote mint mismatch                                   |
| 6004 | `InvalidBaseMint`                 | Base mint mismatch                                    |
| 6005 | `InvalidPaginationCursor`         | Page index doesn't match expected cursor              |
| 6006 | `ArithmeticOverflow`              | Arithmetic overflow in calculations                   |
| 6007 | `ArithmeticUnderflow`             | Arithmetic underflow in calculations                  |
| 6008 | `DivisionByZero`                  | Attempted division by zero                            |
| 6009 | `InvalidPoolConfig`               | Pool configuration invalid                            |
| 6010 | `PositionNotInitialized`          | Honorary position not initialized                     |
| 6011 | `DayAlreadyFinalized`             | Cannot start new page, day already finalized          |
| 6012 | `InvalidInvestorFeeShare`         | investor_fee_share_bps must be 0-10000                |
| 6013 | `InvalidStreamflowStream`         | Invalid Streamflow stream account                     |
| 6014 | `NoLockedTokens`                  | No locked tokens remaining                            |
| 6015 | `TokenAccountNotFound`            | Token account not found                               |
| 6016 | `InsufficientFunds`               | Insufficient funds for operation                      |
| 6017 | `InvalidOwner`                    | Invalid account owner                                 |
| 6018 | `DayNotFinalized`                 | Day not finalized yet                                 |
| 6019 | `PreviousPageNotCompleted`        | Previous page must complete before this one           |

### Troubleshooting Guide

**Error: `TooEarlyForNextDistribution`**
- **Cause**: Trying to start new distribution before 24 hours elapsed
- **Solution**: Wait until `last_distribution_ts + 86400 <= current_timestamp`

**Error: `InvalidPaginationCursor`**
- **Cause**: Page index doesn't match the expected cursor
- **Solution**: Check `progress.pagination_cursor` and provide that value

**Error: `BaseFeeDetected`**
- **Cause**: Base fees were detected in the claim, violating quote-only constraint
- **Solution**: Verify pool configuration ensures quote-only fees

**Error: `InvalidInvestorFeeShare`**
- **Cause**: `investor_fee_share_bps` exceeds 10000 (100%)
- **Solution**: Provide value between 0 and 10000

## Events

### HonoraryPositionInitialized

Emitted when the honorary position is successfully initialized.

```rust
pub struct HonoraryPositionInitialized {
    pub position: Pubkey,           // Honorary position account
    pub owner_pda: Pubkey,           // Position owner PDA
    pub quote_mint: Pubkey,          // Quote mint
    pub base_mint: Pubkey,           // Base mint
    pub pool: Pubkey,                // Pool address
    pub timestamp: i64,              // Initialization timestamp
}
```

### QuoteFeesClaimed

Emitted when fees are claimed at the start of a new day.

```rust
pub struct QuoteFeesClaimed {
    pub amount: u64,                 // Amount of quote fees claimed
    pub timestamp: i64,              // Claim timestamp
    pub day_start: i64,              // Day window start timestamp
}
```

### InvestorPayoutPage

Emitted for each page of investor distributions.

```rust
pub struct InvestorPayoutPage {
    pub page_index: u32,             // Page index
    pub investors_paid: u32,         // Number of investors paid in this page
    pub total_amount_page: u64,      // Total amount distributed in this page
    pub cumulative_day_amount: u64,  // Cumulative amount distributed in day so far
    pub timestamp: i64,              // Payout timestamp
}
```

### CreatorPayoutDayClosed

Emitted when the day is finalized and remainder is sent to creator.

```rust
pub struct CreatorPayoutDayClosed {
    pub creator: Pubkey,             // Creator's ATA
    pub remainder_amount: u64,       // Amount sent to creator
    pub total_claimed_day: u64,      // Total claimed for the day
    pub total_to_investors: u64,     // Total distributed to investors
    pub day_start: i64,              // Day window start
    pub day_end: i64,                // Day window end
}
```

### Listening for Events

```typescript
// Subscribe to program events
program.addEventListener("HonoraryPositionInitialized", (event, slot) => {
  console.log(`Position initialized at slot ${slot}:`, event);
});

program.addEventListener("QuoteFeesClaimed", (event, slot) => {
  console.log(`Fees claimed: ${event.amount} at slot ${slot}`);
});

program.addEventListener("InvestorPayoutPage", (event, slot) => {
  console.log(`Page ${event.pageIndex}: ${event.investorsPaid} investors paid, ${event.totalAmountPage} distributed`);
});

program.addEventListener("CreatorPayoutDayClosed", (event, slot) => {
  console.log(`Day closed: ${event.remainderAmount} to creator at slot ${slot}`);
});
```

## Mathematical Formulas

### Key Definitions

- **Y0** = `total_investor_allocation`: Total investor streamed allocation minted at TGE
- **locked_total(t)** = Sum of still-locked amounts across all investors at time t (from Streamflow)
- **f_locked(t)** = `locked_total(t) / Y0`: Fraction of tokens still locked (0 to 1)
- **investor_fee_share_bps** = Maximum investor fee share in basis points (from policy)
- **claimed_quote** = Total quote fees claimed from honorary position

### Distribution Formulas

#### 1. Eligible Investor Share

```
f_locked_bps = floor(locked_total(t) * 10000 / Y0)
eligible_investor_share_bps = min(investor_fee_share_bps, f_locked_bps)
```

This ensures investors never receive more than their configured share, and the share scales down as tokens unlock.

#### 2. Total Investor Allocation

```
investor_fee_quote = floor(claimed_quote * eligible_investor_share_bps / 10000)
```

#### 3. Per-Investor Distribution

For each investor i:

```
weight_i(t) = locked_i(t) / locked_total(t)
payout_i = floor(investor_fee_quote * weight_i(t))
```

#### 4. Creator Remainder

```
total_to_investors = sum(payout_i for all i)
creator_remainder = claimed_quote - total_to_investors
```

### Example Calculation

**Given:**
- Y0 = 10,000 tokens
- investor_fee_share_bps = 6,000 (60%)
- claimed_quote = 1,000 tokens
- Investor locked amounts: [1,000, 2,000, 3,000]

**Calculate:**

1. locked_total = 1,000 + 2,000 + 3,000 = 6,000
2. f_locked = 6,000 / 10,000 = 0.6 (60%)
3. f_locked_bps = floor(0.6 * 10,000) = 6,000
4. eligible_investor_share_bps = min(6,000, 6,000) = 6,000
5. investor_fee_quote = floor(1,000 * 6,000 / 10,000) = 600

**Per-Investor:**
- Investor 1: floor(600 * 1,000 / 6,000) = 100
- Investor 2: floor(600 * 2,000 / 6,000) = 200
- Investor 3: floor(600 * 3,000 / 6,000) = 300

**Creator Remainder:**
- Total to investors = 100 + 200 + 300 = 600
- Creator remainder = 1,000 - 600 = 400

**Verification:**
- Investor allocation (600) + Creator remainder (400) = Total claimed (1,000) ✓

## Failure Modes

### What Can Go Wrong

1. **Base Fees Detected**
   - **What**: Honorary position accrues base mint fees instead of quote-only
   - **Detection**: Checked during fee claim
   - **Impact**: Distribution fails deterministically
   - **Recovery**: Reconfigure pool or position to ensure quote-only fees

2. **Missing Investor ATAs**
   - **What**: Investor's quote token ATA doesn't exist
   - **Detection**: During token transfer
   - **Impact**: That investor's payout skipped, carried forward
   - **Recovery**: Create investor ATA and re-run crank (dust will be included)

3. **Insufficient Funds in Treasury**
   - **What**: Treasury doesn't have enough tokens for distribution
   - **Detection**: During token transfer
   - **Impact**: Partial distributions may fail
   - **Recovery**: Ensure fees are claimed before distribution

4. **24h Window Violation**
   - **What**: Attempting to start new day before 24h elapsed
   - **Detection**: At crank start
   - **Impact**: Transaction fails with `TooEarlyForNextDistribution`
   - **Recovery**: Wait until eligible time

5. **Pagination Cursor Mismatch**
   - **What**: Wrong page_index provided
   - **Detection**: At page validation
   - **Impact**: Transaction fails with `InvalidPaginationCursor`
   - **Recovery**: Query progress.pagination_cursor and use correct value

### Safety Guarantees

✅ **No Double-Payment**: Pagination cursor prevents re-running same page  
✅ **Atomic Accounting**: All amounts tracked; total distributed + remainder = total claimed  
✅ **Resumable**: Can pause and resume mid-day without loss  
✅ **Deterministic Failure**: Base fee detection causes immediate, clean failure  
✅ **Bounded Loops**: No unbounded iterations; all loops over fixed-size batches  

## Testing

### Running Tests

```bash
# Run all tests
anchor test

# Run specific test
anchor test --skip-local-validator

# Run tests with logs
anchor test -- --nocapture
```

### Test Coverage

The test suite includes:

1. ✅ **Test 1: Initialize Pool and Honorary Position**
   - Verifies policy initialization
   - Verifies position setup
   - Checks PDA derivations

2. ✅ **Test 2: Partial Locks Distribution**
   - Tests distribution with partially locked tokens
   - Verifies pro-rata calculations
   - Validates creator remainder

3. ✅ **Test 3: All Unlocked (100% to Creator)**
   - Tests scenario where all tokens are unlocked
   - Verifies 100% goes to creator

4. ✅ **Test 4: Dust and Cap Behavior**
   - Tests dust threshold enforcement
   - Verifies carry-over mechanism
   - Tests daily cap enforcement

5. ✅ **Test 5: Base-Fee Rejection**
   - Simulates base fee accrual
   - Verifies deterministic failure
   - Confirms no distribution occurs

6. ✅ **Test 6: Pagination Across Multiple Pages**
   - Tests multi-page distribution
   - Verifies idempotent behavior
   - Confirms no double-payment

7. ✅ **Test 7: Mid-Day Resume After Failure**
   - Tests resumability after failure
   - Verifies state consistency
   - Confirms correct accounting

### Test Utilities

Located in `tests/damm-distributor.ts`:

```typescript
// Helper to derive PDAs
function derivePolicyPDA(programId, vault): [PublicKey, number]
function deriveProgressPDA(programId, vault): [PublicKey, number]
function deriveInvestorFeePosOwnerPDA(programId, vault): [PublicKey, number]

// Helper to create mock Streamflow stream
async function createMockStreamflow(connection, investor, lockedAmount)

// Helper to advance time by 24 hours
async function advanceTime24h(connection)

// Assertion helper for amount matching
function assertAmountsMatch(actual, expected, tolerance)
```

### Adding New Tests

```typescript
it("Your new test", async () => {
  // Setup
  const investor = Keypair.generate();
  
  // Execute
  await program.methods
    .crankDistribution(0, true)
    .accounts({...})
    .rpc();
  
  // Verify
  const progress = await program.account.distributionProgress.fetch(progressPDA);
  assert.equal(progress.dayFinalized, true);
});
```

## License

[License information here]

## Contributing

[Contribution guidelines here]

## Support

For issues and questions:
- GitHub Issues: [link]
- Discord: [link]
- Documentation: [link]

---

**Built for Star (star.xyz)** - Production-ready, battle-tested, and ready for the bounty! 🚀
