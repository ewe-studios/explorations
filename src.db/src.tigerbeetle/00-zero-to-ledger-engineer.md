---
title: "Zero to Ledger Engineer: TigerBeetle"
subtitle: "Understanding financial accounting databases and double-entry bookkeeping"
location: /home/darkvoid/Boxxed/@dev/repo-expolorations/src.db/src.tigerbeetle
related: exploration.md
---

# 00 - Zero to Ledger Engineer: TigerBeetle

## Overview

This document explains the fundamentals of TigerBeetle - a distributed financial accounting database designed for mission-critical safety and performance.

## Part 1: Why Financial Accounting Databases Exist

### The Problem with Traditional Databases for Finance

```
Traditional Database for Finance:
┌─────────────────────────────────────────┐
│  Table: accounts                        │
│  ┌──────┬──────────┬────────────┐       │
│  │ id   │ balance  │ currency   │       │
│  ├──────┼──────────┼────────────┤       │
│  │ 1    │ 1000.00  │ USD        │       │
│  │ 2    │ 500.00   │ USD        │       │
│  └──────┴──────────┴────────────┘       │
│                                         │
│  Transfer $100 from 1 to 2:             │
│  UPDATE accounts SET balance = balance  │
│    - 100 WHERE id = 1;                  │
│  UPDATE accounts SET balance = balance  │
│    + 100 WHERE id = 2;                  │
│                                         │
│  Problems:                              │
│  - No audit trail                       │
│  - Balance can go negative              │
│  - No validation of accounting rules    │
│  - Can't reconstruct history            │
└─────────────────────────────────────────┘

Financial Accounting Database:
┌─────────────────────────────────────────┐
│  Accounts (current state only)          │
│  ┌──────┬──────────┬────────────┐       │
│  │ id   │ debits   │ credits    │       │
│  ├──────┼──────────┼────────────┤       │
│  │ 1    │ 500      │ 1500       │       │
│  │ 2    │ 1000     │ 500        │       │
│  └──────┴──────────┴────────────┘       │
│                                         │
│  Transfers (immutable audit trail)      │
│  ┌────┬────┬─────┬──────┬──────┬──────┐│
│  │ id │ts  │ code│ debit│credit│amount││
│  ├────┼────┼─────┼──────┼──────┼──────┤│
│  │ 1  │1000│ 1   │ 1    │ 2    │ 100  ││
│  │ 2  │1001│ 1   │ 2    │ 1    │ 50   ││
│  └────┴────┴─────┴──────┴──────┴──────┘│
│                                         │
│  Benefits:                              │
│  ✓ Complete audit trail                 │
│  ✓ Double-entry enforced                │
│  ✓ History reconstructible              │
│  ✓ Regulatory compliant                 │
└─────────────────────────────────────────┘
```

### What is Double-Entry Bookkeeping?

```
Double-Entry Bookkeeping (500+ year old system):

Fundamental Equation:
  Assets = Liabilities + Equity

Every transaction affects at least TWO accounts:
  - One account is DEBITED (value removed)
  - One account is CREDITED (value added)
  - Total debits MUST equal total credits

Example: Transfer $100 from Checking to Savings

  Checking Account          Savings Account
  ┌─────────────────┐      ┌─────────────────┐
  │ Debits: $1000   │      │ Debits: $500    │
  │ Credits: $1100  │      │ Credits: $500   │
  │ Balance: $100   │      │ Balance: $0     │
  └─────────────────┘      └─────────────────┘
         │                        │
         │  Debit $100            │  Credit $100
         └───────────┬────────────┘
                     │
              Transfer $100
                     │
         ┌───────────┴────────────┐
         ▼                        ▼
  ┌─────────────────┐      ┌─────────────────┐
  │ Debits: $1100   │      │ Debits: $500    │
  │ Credits: $1100  │      │ Credits: $600   │
  │ Balance: $0     │      │ Balance: $100   │
  └─────────────────┘      └─────────────────┘

Balance Check:
  Total Debits: $1100 + $500 = $1600
  Total Credits: $1100 + $600 = $1700
  Wait... that doesn't balance!

  Actually: Balance = Credits - Debits
  Checking: $1100 - $1100 = $0 ✓
  Savings: $600 - $500 = $100 ✓
```

## Part 2: TigerBeetle Core Concepts

### Account Model

```
TigerBeetle Account Structure:

struct Account {
    id: u128,              // Unique account ID
    user_data: u128,       // Custom user data
    ledger: u32,           // Ledger identifier (for multi-ledger)
    code: u16,             // Account type code
    flags: AccountFlags,   // Account flags
    debits_pending: u64,   // Pending debits (not yet posted)
    debits_posted: u64,    // Posted debits (completed)
    credits_pending: u64,  // Pending credits
    credits_posted: u64,   // Posted credits
}

Account Flags:
  - LINKED: Account is linked to another
  - DEBITS_MUST_NOT_EXCEED_CREDITS: Negative balance protection
  - CREDITS_MUST_NOT_EXCEED_DEBITS: For certain account types

Balance Calculation:
  balance = credits_posted - debits_posted

For asset accounts: positive balance = funds available
For liability accounts: positive balance = owed amount
```

### Transfer Model

```
TigerBeetle Transfer Structure:

struct Transfer {
    id: u128,              // Unique transfer ID
    debit_account_id: u128,
    credit_account_id: u128,
    amount: u64,           // Amount in smallest currency unit
    ledger: u32,
    code: u16,             // Transfer type code
    flags: TransferFlags,
    timestamp: u64,        // Nanosecond timestamp
    timeout: u64,          // Two-phase commit timeout
    pending_id: u128,      // For two-phase commits
}

Transfer Flags:
  - LINKED: Part of a linked transfer group
  - PENDING: Two-phase commit (needs commit/rollback)
  - POST_PENDING_TRANSFER: Complete pending transfer
  - VOID_PENDING_TRANSFER: Cancel pending transfer

Transfer Validation:
  1. Debit account exists
  2. Credit account exists
  3. Same ledger
  4. Sufficient balance (if flag set)
  5. Not exceeding limits
  6. Code is valid
```

### Two-Phase Commit

```
Two-Phase Commit in TigerBeetle:

Phase 1 - Prepare (Pending):
  Client: create_transfers(
    id=100,
    debit_account=1,
    credit_account=2,
    amount=100,
    flags=PENDING,
    timeout=60000  // 60 second timeout
  )

  Server:
    - Validates transfer
    - Reserves funds (debits_pending, credits_pending)
    - Returns SUCCESS or ERROR

  Result: Funds reserved but not transferred

Phase 2a - Commit:
  Client: create_transfers(
    id=101,
    pending_id=100,  // References pending transfer
    flags=POST_PENDING_TRANSFER
  )

  Server:
    - Converts pending to posted
    - debits_posted += amount
    - credits_posted += amount

  Result: Transfer complete

Phase 2b - Void (Rollback):
  Client: create_transfers(
    id=102,
    pending_id=100,
    flags=VOID_PENDING_TRANSFER
  )

  Server:
    - Releases reserved funds
    - debits_pending = 0
    - credits_pending = 0

  Result: Transfer cancelled

Use Cases:
  - Escrow payments
  - Atomic multi-step operations
  - Timeout-based rollbacks
```

## Part 3: Distributed Systems Fundamentals

### Why Distributed for Finance?

```
Single Node Problems:
┌─────────────────────────────────────────┐
│  Single Node Database                   │
│                                         │
│  Failure modes:                         │
│  - Disk corruption = data loss          │
│  - Server crash = downtime              │
│  - Data center fire = business over     │
│                                         │
│  Not acceptable for:                    │
│  - Banks (billions at risk)             │
│  - Payment processors (24/7 required)   │
│  - Stock exchanges (market confidence)  │
└─────────────────────────────────────────┘

Distributed Solution:
┌─────────────────────────────────────────┐
│  Distributed Cluster (3+ nodes)         │
│                                         │
│  ┌─────────┐   ┌─────────┐   ┌────────┐│
│  │ Node 1  │<──│ Node 2  │<──│ Node 3 ││
│  │ Leader  │──>│Follower │──>│Follower││
│  └─────────┘   └─────────┘   └────────┘│
│                                         │
│  Benefits:                              │
│  ✓ Tolerates node failures              │
│  ✓ No single point of failure           │
│  ✓ Geographic distribution              │
│  ✓ Consensus on all transactions        │
└─────────────────────────────────────────┘
```

### Viewstamped Replication

```
Viewstamped Replication (VR) Protocol:

VR is similar to Raft/Paxos but with key differences:

Basic Concepts:
  - View: Current configuration (leader + followers)
  - Leader: Handles all client requests
  - Followers: Replicate leader's log
  - Quorum: Majority must agree

Request Flow:
  Client → Leader → Followers → Quorum Ack → Client

┌─────────────────────────────────────────────────────────────┐
│                    VR Protocol Flow                          │
├─────────────────────────────────────────────────────────────┤
│                                                              │
│  1. Client sends operation to Leader                        │
│     Client: "Transfer $100 from A to B"                     │
│                                                              │
│  2. Leader assigns sequence number, appends to log          │
│     Leader: log[42] = Transfer(A, B, $100)                  │
│                                                              │
│  3. Leader sends to Followers                               │
│     Leader → Follower1: log[42] = Transfer(A, B, $100)      │
│     Leader → Follower2: log[42] = Transfer(A, B, $100)      │
│                                                              │
│  4. Followers acknowledge                                   │
│     Follower1 → Leader: ACK[42]                             │
│     Follower2 → Leader: ACK[42]                             │
│                                                              │
│  5. Leader commits after quorum                             │
│     Leader: commit log[42] (majority acked)                 │
│     Leader → Client: SUCCESS                                │
│                                                              │
│  6. Leader notifies followers to commit                     │
│     Leader → Follower1: COMMIT[42]                          │
│     Leader → Follower2: COMMIT[42]                          │
│                                                              │
└─────────────────────────────────────────────────────────────┘

View Changes (Leader Failure):
  1. Followers detect leader timeout
  2. Start view change protocol
  3. Elect new leader
  4. New leader syncs log
  5. Resume normal operation
```

### Consensus Guarantees

```
Safety Guarantees:
┌─────────────────────────────────────────────────────────────┐
│  Agreement: No two nodes decide different values            │
│  Validity: Only proposed values can be decided              │
│  Termination: All correct nodes eventually decide           │
│  Integrity: Each node decides at most once                  │
└─────────────────────────────────────────────────────────────┘

Liveness Guarantees:
┌─────────────────────────────────────────────────────────────┐
│  Progress: System makes progress if majority available      │
│  Recovery: Failed nodes can rejoin and sync                 │
│  Reconfiguration: Cluster can be modified safely            │
└─────────────────────────────────────────────────────────────┘

Failure Tolerance:
  N nodes can tolerate (N-1)/2 failures

  3 nodes: 1 failure tolerated
  5 nodes: 2 failures tolerated
  7 nodes: 3 failures tolerated

  Formula: Need 2f+1 nodes to tolerate f failures
```

## Part 4: ACID Properties in TigerBeetle

### Atomicity

```
Atomicity: All or Nothing

Transfer Operation is Atomic:
  BEGIN TRANSACTION
    Debit Account A: $100
    Credit Account B: $100
  END TRANSACTION

Either BOTH happen or NEITHER happens.

Never:
  - Account A debited but B not credited (bug!)
  - Account B credited but A not debited (free money!)

Implementation:
  - Write-Ahead Log (WAL)
  - Commit record only after both entries logged
  - Recovery replays committed transactions
```

### Consistency

```
Consistency: Database Rules Always Hold

TigerBeetle Consistency Rules:
  1. Sum of all balances = constant (per ledger)
  2. Debits must not exceed credits (if flag set)
  3. Account codes must be valid
  4. Transfer codes must be valid
  5. Ledger must match between accounts

Example Validation:

  // INVALID: Would violate consistency
  create_transfers(
    debit_account: 1,
    credit_account: 2,
    amount: 1000000,  // More than account balance
    ledger: 700
  )

  // Returns: inserted_flags.consistency_violation

  // VALID: Maintains consistency
  create_transfers(
    debit_account: 1,
    credit_account: 2,
    amount: 100,  // Within balance
    ledger: 700
  )

  // Returns: inserted_flags.success
```

### Isolation

```
Isolation: Concurrent Transactions Don't Interfere

TigerBeetle uses Serializability:
  - All transactions appear to execute sequentially
  - Even though they may execute concurrently
  - Equivalent to some total ordering

Example:

  Concurrent Transfers:
  T1: Transfer $50 from A to B
  T2: Transfer $30 from B to C

  Possible Serializations:
  1. T1 then T2: A=-50, B=+20, C=+30
  2. T2 then T1: A=-50, B=+20, C=+30

  Same result! (because commutative)

  But if T2 was: Transfer $100 from B to A
  1. T1 then T2: A=+50, B=-50 (T2 fails - insufficient funds)
  2. T2 then T1: A=+50, B=-50 (T1 fails - insufficient funds)

  Either way, one fails - consistent!
```

### Durability

```
Durability: Once Committed, Never Lost

TigerBeetle Durability Strategy:
  1. Write-Ahead Log (WAL)
     - All changes logged before applied
     - fsync() ensures on disk
     - Replicated to quorum

  2. Snapshots
     - Periodic point-in-time copies
     - Faster recovery (don't replay from beginning)

  3. Replication
     - Multiple copies on different nodes
     - Survives disk failures, datacenter failures

Recovery Process:
  1. Load latest snapshot
  2. Replay WAL from snapshot point
  3. Apply all committed transactions
  4. Database restored to consistent state

  Recovery Time Objective (RTO): Minutes
  Recovery Point Objective (RPO): Zero (no data loss)
```

## Part 5: Common Operations

### Creating Accounts

```
// Create two accounts in ledger 700
create_accounts
  id=1, code=10, ledger=700,
  id=2, code=10, ledger=700;

// Account types (code):
// 1 = Asset (cash, receivables)
// 2 = Liability (payables, deposits)
// 3 = Equity (retained earnings)
// 4 = Revenue (income)
// 5 = Expense (costs)

// With flags
create_accounts
  id=3, code=1, ledger=700,
  flags=debits_must_not_exceed_credits;

// This prevents overdrafts on account 3
```

### Creating Transfers

```
// Simple transfer
create_transfers
  id=1,
  debit_account_id=1,
  credit_account_id=2,
  amount=100,
  ledger=700,
  code=1;

// Linked transfers (atomic group)
create_transfers
  id=10, debit_account_id=1, credit_account_id=2, amount=50, ledger=700, code=1, flags=linked,
  id=11, debit_account_id=2, credit_account_id=3, amount=30, ledger=700, code=1;

// Both succeed or both fail

// Two-phase commit
create_transfers
  id=100,
  debit_account_id=1,
  credit_account_id=2,
  amount=1000,
  ledger=700,
  code=1,
  flags=pending,
  timeout=3600;  // 1 hour to commit

// Later, commit or void
create_transfers
  id=101,
  pending_id=100,
  flags=post_pending_transfer;  // or void_pending_transfer
```

### Querying Accounts

```
// Lookup single account
lookup_accounts id=1;

// Lookup multiple accounts
lookup_accounts id=1, id=2, id=3;

// Response format
{
  "id": "1",
  "user_data": "0",
  "ledger": "700",
  "code": "1",
  "flags": "",
  "debits_pending": "0",
  "debits_posted": "150",
  "credits_pending": "0",
  "credits_posted": "250",
  // balance = credits_posted - debits_posted = 100
}
```

## Part 6: Common Pitfalls

### Pitfall 1: Integer Overflow

```
// DON'T: Use amounts that can overflow
amount: u64::MAX  // Will cause issues

// DO: Validate amounts
assert!(amount < MAX_ALLOWED_TRANSFER);

// TigerBeetle uses u64 for amounts
// Max: 18,446,744,073,709,551,615
// For USD cents: $184 quadrillion (usually sufficient)
```

### Pitfall 2: Ledger Mismatch

```
// DON'T: Transfer between different ledgers
create_transfers(
  debit_account: account_in_ledger_700,
  credit_account: account_in_ledger_800,  // ERROR!
  amount: 100
);

// DO: Ensure same ledger
assert!(debit_account.ledger == credit_account.ledger);

// For cross-ledger, use bridge accounts
```

### Pitfall 3: Not Handling Pending Transfers

```
// DON'T: Let pending transfers timeout without handling
create_transfers(flags=pending, timeout=60);
// ... forget about it ...
// Transfer auto-voids after timeout, but funds were reserved!

// DO: Track and resolve pending transfers
pending_transfers.insert(transfer_id, transfer);
// ... later ...
if should_commit {
  commit_pending(transfer_id);
} else {
  void_pending(transfer_id);
}
```

### Pitfall 4: Not Using Idempotency

```
// DON'T: Retry without idempotency keys
client.send(transfer);  // Network error
client.send(transfer);  // Duplicate!

// DO: Use idempotent transfer IDs
transfer_id = generate_deterministic_id(operation);
client.send(transfer { id: transfer_id });

// Network error
client.send(transfer { id: transfer_id });  // Same ID = no duplicate
```

---

*This document is part of the TigerBeetle exploration series. See [exploration.md](./exploration.md) for the complete index.*
