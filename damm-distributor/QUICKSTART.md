# Quick Start Guide

Get up and running with the DAMM Distributor in 5 minutes!

## Prerequisites

- Rust 1.90+
- Anchor 0.31.1+
- Node.js 16+
- Yarn or npm

## Installation

```bash
# Clone the repository
git clone <your-repo-url>
cd damm-distributor

# Install dependencies
yarn install

# Build the program
anchor build
```

## Deploy to Devnet

```bash
# Configure for devnet
solana config set --url devnet

# Get some devnet SOL
solana airdrop 2

# Deploy
anchor deploy --provider.cluster devnet
```

## Basic Usage

### 1. Initialize Policy

```typescript
import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { DammDistributor } from "./target/types/damm_distributor";
import { PublicKey, SystemProgram } from "@solana/web3.js";

const program = anchor.workspace.DammDistributor as Program<DammDistributor>;
const provider = anchor.AnchorProvider.env();

// Your configuration
const vaultPubkey = new PublicKey("YOUR_VAULT_PUBKEY");
const quoteMintPubkey = new PublicKey("YOUR_QUOTE_MINT");
const baseMintPubkey = new PublicKey("YOUR_BASE_MINT");
const poolPubkey = new PublicKey("YOUR_POOL_PUBKEY");
const creatorQuoteAta = new PublicKey("YOUR_CREATOR_ATA");

// Derive policy PDA
const [policyPDA] = PublicKey.findProgramAddressSync(
  [Buffer.from("policy"), vaultPubkey.toBuffer()],
  program.programId
);

// Initialize
await program.methods
  .initializePolicy(
    new anchor.BN(10_000_000_000), // total_investor_allocation (10K tokens, 6 decimals)
    6000,                           // investor_fee_share_bps (60%)
    new anchor.BN(0),               // daily_cap_lamports (0 = no cap)
    new anchor.BN(1000)             // min_payout_lamports
  )
  .accounts({
    policy: policyPDA,
    vault: vaultPubkey,
    quoteMint: quoteMintPubkey,
    baseMint: baseMintPubkey,
    pool: poolPubkey,
    creatorQuoteAta: creatorQuoteAta,
    authority: provider.wallet.publicKey,
    systemProgram: SystemProgram.programId,
  })
  .rpc();

console.log("Policy initialized!");
```

### 2. Initialize Honorary Position

```typescript
// Derive PDAs
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

// Initialize position
const honoraryPosition = anchor.web3.Keypair.generate();

await program.methods
  .initializePosition()
  .accounts({
    policy: policyPDA,
    progress: progressPDA,
    honoraryPosition: honoraryPosition.publicKey,
    investorFeePositionOwnerPda: investorFeePosOwnerPDA,
    quoteMint: quoteMintPubkey,
    baseMint: baseMintPubkey,
    pool: poolPubkey,
    programQuoteTreasury: programQuoteTreasuryPubkey,
    authority: provider.wallet.publicKey,
    tokenProgram: TOKEN_PROGRAM_ID,
    systemProgram: SystemProgram.programId,
  })
  .signers([honoraryPosition])
  .rpc();

console.log("Honorary position initialized!");
```

### 3. Run Distribution Crank

```typescript
import { TOKEN_PROGRAM_ID } from "@solana/spl-token";

// Prepare investor accounts (stream + ATA pairs)
const investors = [
  {
    streamAccount: new PublicKey("INVESTOR_1_STREAM"),
    quoteAta: new PublicKey("INVESTOR_1_ATA"),
  },
  {
    streamAccount: new PublicKey("INVESTOR_2_STREAM"),
    quoteAta: new PublicKey("INVESTOR_2_ATA"),
  },
  // ... more investors
];

// Build remaining accounts array
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

// Run crank
await program.methods
  .crankDistribution(
    0,    // page_index (start with 0)
    true  // is_final_page (true if this is the only/last page)
  )
  .accounts({
    policy: policyPDA,
    progress: progressPDA,
    honoraryPosition: honoraryPosition.publicKey,
    investorFeePositionOwnerPda: investorFeePosOwnerPDA,
    programQuoteTreasury: programQuoteTreasuryPubkey,
    creatorQuoteAta: creatorQuoteAta,
    tokenProgram: TOKEN_PROGRAM_ID,
  })
  .remainingAccounts(remainingAccounts)
  .rpc();

console.log("Distribution complete!");
```

## Pagination Example

For many investors, paginate the distribution:

```typescript
const INVESTORS_PER_PAGE = 10;
const totalInvestors = investors.length;
const totalPages = Math.ceil(totalInvestors / INVESTORS_PER_PAGE);

for (let pageIndex = 0; pageIndex < totalPages; pageIndex++) {
  const start = pageIndex * INVESTORS_PER_PAGE;
  const end = Math.min(start + INVESTORS_PER_PAGE, totalInvestors);
  const pageInvestors = investors.slice(start, end);
  
  const remainingAccounts = [];
  for (const investor of pageInvestors) {
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
      pageIndex,
      pageIndex === totalPages - 1 // is_final_page
    )
    .accounts({
      policy: policyPDA,
      progress: progressPDA,
      honoraryPosition: honoraryPosition.publicKey,
      investorFeePositionOwnerPda: investorFeePosOwnerPDA,
      programQuoteTreasury: programQuoteTreasuryPubkey,
      creatorQuoteAta: creatorQuoteAta,
      tokenProgram: TOKEN_PROGRAM_ID,
    })
    .remainingAccounts(remainingAccounts)
    .rpc();
  
  console.log(`Page ${pageIndex + 1}/${totalPages} complete`);
}
```

## Monitoring Events

```typescript
// Listen for distribution events
program.addEventListener("QuoteFeesClaimed", (event, slot) => {
  console.log(`Fees claimed: ${event.amount} tokens`);
  console.log(`Day started: ${new Date(event.dayStart * 1000)}`);
});

program.addEventListener("InvestorPayoutPage", (event, slot) => {
  console.log(`Page ${event.pageIndex}: ${event.investorsPaid} investors paid`);
  console.log(`Amount this page: ${event.totalAmountPage}`);
  console.log(`Total today: ${event.cumulativeDayAmount}`);
});

program.addEventListener("CreatorPayoutDayClosed", (event, slot) => {
  console.log(`Day closed!`);
  console.log(`Creator received: ${event.remainderAmount}`);
  console.log(`Total to investors: ${event.totalToInvestors}`);
  console.log(`Total claimed: ${event.totalClaimedDay}`);
});
```

## Common Issues

### "TooEarlyForNextDistribution"
- **Cause**: Trying to run crank before 24 hours elapsed
- **Solution**: Wait until at least 24 hours since last distribution

### "InvalidPaginationCursor"
- **Cause**: Wrong page_index provided
- **Solution**: Start with page 0, increment by 1 for each subsequent page

### "InvalidQuoteMint" or "InvalidBaseMint"
- **Cause**: Mint mismatch with policy configuration
- **Solution**: Ensure correct mint addresses match policy

## Next Steps

- Read the full [README.md](./README.md) for comprehensive documentation
- Check [IMPLEMENTATION_SUMMARY.md](./IMPLEMENTATION_SUMMARY.md) for technical details
- Review test examples in `tests/damm-distributor.ts`
- Join our Discord for support

## Need Help?

- 📖 Documentation: [README.md](./README.md)
- 🐛 Issues: [GitHub Issues](https://github.com/your-repo/issues)
- 💬 Discord: [Join our Discord](https://discord.gg/your-invite)

---

**Happy Distributing!** 🚀
