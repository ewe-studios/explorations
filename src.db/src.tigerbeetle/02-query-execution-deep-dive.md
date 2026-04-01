---
title: "TigerBeetle Query Execution Deep Dive"
subtitle: "Transaction validation, two-phase commit processing, and execution pipeline"
location: /home/darkvoid/Boxxed/@dev/repo-expolorations/src.db/src.tigerbeetle
related: 00-zero-to-ledger-engineer.md, 01-storage-engine-deep-dive.md, exploration.md
---

# 02 - Query Execution Deep Dive: TigerBeetle

## Overview

This document covers TigerBeetle's query execution engine - how transactions are validated, the two-phase commit protocol implementation, linked transfer processing, and the complete execution pipeline.

## Part 1: Transaction Processing Architecture

### Single-Threaded Executor

```
TigerBeetle uses a deterministic single-threaded executor:

┌─────────────────────────────────────────────────────────┐
│                    Client Requests                        │
│                           │                               │
│                           ▼                               │
│  ┌─────────────────────────────────────────────────┐    │
│  │              Request Queue                       │    │
│  │         (FIFO, ordered by arrival)              │    │
│  └──────────────────────┬──────────────────────────┘    │
│                         │                                 │
│                         ▼                                 │
│  ┌─────────────────────────────────────────────────┐    │
│  │           Transaction Executor                   │    │
│  │  ┌─────────────────────────────────────────┐    │    │
│  │  │  Single Thread (deterministic)          │    │    │
│  │  │                                         │    │    │
│  │  │  1. Parse request                       │    │    │
│  │  │  2. Validate transaction                │    │    │
│  │  │  3. Apply to accounts                   │    │    │
│  │  │  4. Write to WAL                        │    │    │
│  │  │  5. Return result                       │    │    │
│  │  └─────────────────────────────────────────┘    │    │
│  └─────────────────────────────────────────────────┘    │
│                         │                                 │
│                         ▼                                 │
│  ┌─────────────────────────────────────────────────┐    │
│  │              Storage Engine                      │    │
│  │         (Accounts, Transfers, WAL)              │    │
│  └─────────────────────────────────────────────────┘    │
└─────────────────────────────────────────────────────────┘

Why single-threaded?
- Deterministic execution (critical for consensus)
- No locking overhead
- No race conditions
- Simpler reasoning about correctness
- Easier testing and debugging

Throughput: 50K-100K transactions/second (single thread)
Latency: < 1ms P99
```

### Request Processing Pipeline

```
Complete Request Pipeline:

┌─────────────────────────────────────────────────────────┐
│ Stage 1: Network Layer                                   │
│                                                          │
│ Client ──► TCP Connection ──► Request Buffer             │
│                                                          │
│ Protocol: TigerBeetle binary protocol (not TCP/IP)       │
│ Message format: [header: 8 bytes][body: variable]        │
└─────────────────────────────────────────────────────────┘
                         │
                         ▼
┌─────────────────────────────────────────────────────────┐
│ Stage 2: Request Parsing                                 │
│                                                          │
│ 1. Read message header                                   │
│ 2. Determine message type                                │
│    - 0x01: create_accounts                               │
│    - 0x02: create_transfers                              │
│    - 0x03: lookup_accounts                               │
│    - 0x04: lookup_transfers                              │
│ 3. Parse message body                                    │
│ 4. Validate message structure                            │
└─────────────────────────────────────────────────────────┘
                         │
                         ▼
┌─────────────────────────────────────────────────────────┐
│ Stage 3: Validation                                      │
│                                                          │
│ For create_accounts:                                     │
│ - Account ID unique                                      │
│ - Ledger is valid                                        │
│ - Code is valid                                          │
│ - Flags are valid                                        │
│                                                          │
│ For create_transfers:                                    │
│ - Transfer ID unique                                     │
│ - Debit account exists                                   │
│ - Credit account exists                                  │
│ - Same ledger                                            │
│ - Sufficient balance (if flag set)                       │
│ - Not exceeding limits                                   │
│ - Code is valid                                          │
│ - Pending ID valid (for two-phase)                       │
└─────────────────────────────────────────────────────────┘
                         │
                         ▼
┌─────────────────────────────────────────────────────────┐
│ Stage 4: Execution                                       │
│                                                          │
│ 1. Create WAL entry                                      │
│ 2. Append to WAL                                         │
│ 3. Apply to in-memory state                              │
│ 4. Update account balances                               │
│ 5. Insert transfer record                                │
└─────────────────────────────────────────────────────────┘
                         │
                         ▼
┌─────────────────────────────────────────────────────────┐
│ Stage 5: Response                                        │
│                                                          │
│ 1. Build response message                                │
│ 2. Include result flags for each transaction             │
│ 3. Send to client                                        │
└─────────────────────────────────────────────────────────┘

Total latency: 200-500μs (local), 1-5ms (networked)
```

### Validation Rules

```
Transaction Validation Matrix:

┌──────────────────────────────────────────────────────────┐
│ Validation Rule               │ Account │ Transfer │ Post │
├──────────────────────────────────────────────────────────┤
│ ID must be unique             │   ✓     │    ✓     │  N/A │
│ Ledger must be valid          │   ✓     │    ✓     │  N/A │
│ Code must be valid            │   ✓     │    ✓     │  N/A │
│ Flags must be valid           │   ✓     │    ✓     │  N/A │
│ Debit account exists          │   N/A   │    ✓     │  ✓   │
│ Credit account exists         │   N/A   │    ✓     │  ✓   │
│ Same ledger                   │   N/A   │    ✓     │  ✓   │
│ Sufficient balance            │   N/A   │    ✓*    │  ✓*  │
│ Not exceeding debits limit    │   N/A   │    ✓*    │  ✓*  │
│ Not exceeding credits limit   │   N/A   │    ✓*    │  ✓*  │
│ Pending ID exists (2PC)       │   N/A   │    ✓     │  ✓   │
│ Timeout not expired (2PC)     │   N/A   │    ✓     │  ✓   │
└──────────────────────────────────────────────────────────┘

* Conditional on account flags

Validation Error Codes (result flags):
- 0x01: ID exists
- 0x02: Account not found
- 0x04: Ledger mismatch
- 0x08: Invalid code
- 0x10: Invalid flags
- 0x20: Insufficient funds
- 0x40: Would exceed debits limit
- 0x80: Would exceed credits limit
- 0x100: Pending ID not found
- 0x200: Pending ID already committed
- 0x400: Timeout expired
```

## Part 2: Two-Phase Commit Implementation

### Pending Transfer Lifecycle

```
Two-Phase Commit State Machine:

┌───────────────────────────────────────────────────────────┐
│                                                            │
│                      ┌─────────────┐                       │
│                      │   INITIAL   │                       │
│                      └──────┬──────┘                       │
│                             │                               │
│                  create_transfers(                          │
│                    flags=PENDING                            │
│                  )                                          │
│                             │                               │
│                             ▼                               │
│                      ┌─────────────┐                       │
│              ┌───────│   PENDING   │───────┐               │
│              │       └──────┬──────┘       │               │
│              │              │               │               │
│              │              │               │               │
│         commit              │          timeout              │
│         (post)              │                               │
│              │              │ void                          │
│              │              │ (cancel)                      │
│              ▼              ▼                               │
│       ┌─────────────┐ ┌─────────────┐                      │
│       │   POSTED    │ │   VOIDED    │                      │
│       │  (complete) │ │ (cancelled) │                      │
│       └─────────────┘ └─────────────┘                      │
│                                                            │
│ Terminal states: POSTED, VOIDED                            │
│ Transitions are immutable                                  │
└────────────────────────────────────────────────────────────┘

State transitions:
INITIAL → PENDING:   Client creates pending transfer
PENDING → POSTED:    Client commits (post_pending_transfer)
PENDING → VOIDED:    Client voids OR timeout expires
POSTED → (none):     Terminal state
VOIDED → (none):     Terminal state
```

### Pending Transfer Data Structure

```
Pending Transfer Tracking:

In-Memory Pending Table:
```rust
struct PendingTable {
    /// Map: pending_id -> PendingEntry
    pending: HashMap<u128, PendingEntry>,

    /// Timeout queue: (timeout_timestamp, pending_id)
    timeouts: BinaryHeap<(u64, u128)>,
}

struct PendingEntry {
    /// Original transfer data
    transfer: Transfer,

    /// State: Pending, Posted, Voided
    state: PendingState,

    /// Timestamp when created
    created_at: u64,

    /// Timeout timestamp (nanoseconds)
    timeout_at: u64,
}

enum PendingState {
    Pending,
    Posted,
    Voided,
}
```

Disk Persistence:
┌─────────────────────────────────────────────────────────┐
│ Transfer Record (128 bytes)                              │
│                                                          │
│ When flags & PENDING != 0:                               │
│ - Written to transfer table                              │
│ - debits_pending += amount                               │
│ - credits_pending += amount                              │
│ - pending_id indexed for lookup                          │
│                                                          │
│ On commit (POST_PENDING_TRANSFER):                       │
│ - debits_pending -= amount                               │
│ - credits_pending -= amount                              │
│ - debits_posted += amount                                │
│ - credits_posted += amount                               │
│ - State marked as POSTED                                 │
│                                                          │
│ On void (VOID_PENDING_TRANSFER):                         │
│ - debits_pending -= amount                               │
│ - credits_pending -= amount                              │
│ - State marked as VOIDED                                 │
└─────────────────────────────────────────────────────────┘
```

### Two-Phase Commit Code Path

```rust
/// Create transfers (including two-phase commit)
fn create_transfers(
    &mut self,
    transfers: Vec<Transfer>,
) -> Vec<ResultFlags> {
    let mut results = Vec::with_capacity(transfers.len());

    for transfer in transfers {
        let result = self.process_transfer(transfer);
        results.push(result);
    }

    results
}

fn process_transfer(&mut self, transfer: Transfer) -> ResultFlags {
    // Check for two-phase commit operations
    if transfer.flags.contains(TransferFlags::POST_PENDING) {
        return self.commit_pending_transfer(&transfer);
    }

    if transfer.flags.contains(TransferFlags::VOID_PENDING) {
        return self.void_pending_transfer(&transfer);
    }

    // Normal transfer creation
    if transfer.flags.contains(TransferFlags::PENDING) {
        self.create_pending_transfer(transfer)
    } else {
        self.create_immediate_transfer(transfer)
    }
}

fn create_pending_transfer(&mut self, transfer: Transfer) -> ResultFlags {
    // Validate transfer
    let validation = self.validate_transfer(&transfer);
    if !validation.is_valid() {
        return validation.error_flags;
    }

    // Check timeout
    let now = current_timestamp_ns();
    if transfer.timeout != 0 && transfer.timeout <= now {
        return ResultFlags::TIMEOUT_EXPIRED;
    }

    // Reserve funds (pending balances)
    let debit_account = self.get_account(transfer.debit_account_id);
    if debit_account.debits_pending + transfer.amount > debit_account.credits_posted {
        return ResultFlags::INSUFFICIENT_FUNDS;
    }

    // Update pending balances
    self.update_pending_balances(&transfer, true);

    // Write transfer to WAL
    self.wal.append(WALEntry {
        entry_type: EntryType::TransferCreate,
        data: transfer.to_bytes(),
    });

    // Insert into pending table
    self.pending_table.insert(transfer.id, PendingEntry {
        transfer: transfer.clone(),
        state: PendingState::Pending,
        created_at: now,
        timeout_at: transfer.timeout,
    });

    // Insert into transfer table (for audit trail)
    self.write_transfer(&transfer);

    ResultFlags::SUCCESS
}

fn commit_pending_transfer(&mut self, commit: &Transfer) -> ResultFlags {
    // Lookup pending transfer
    let pending = match self.pending_table.get(&commit.pending_id) {
        Some(p) => p,
        None => return ResultFlags::PENDING_ID_NOT_FOUND,
    };

    // Check state
    if pending.state != PendingState::Pending {
        return ResultFlags::PENDING_ALREADY_COMMITTED;
    }

    // Check timeout
    let now = current_timestamp_ns();
    if now > pending.timeout_at {
        return ResultFlags::TIMEOUT_EXPIRED;
    }

    // Convert pending to posted
    let transfer = &pending.transfer;
    self.update_pending_balances(transfer, false); // Remove pending
    self.update_posted_balances(transfer, true);   // Add posted

    // Update pending state
    pending.state = PendingState::Posted;

    // Write commit to WAL
    self.wal.append(WALEntry {
        entry_type: EntryType::TransferCommit,
        data: commit.to_bytes(),
    });

    ResultFlags::SUCCESS
}

fn void_pending_transfer(&mut self, void: &Transfer) -> ResultFlags {
    // Lookup pending transfer
    let pending = match self.pending_table.get(&commit.pending_id) {
        Some(p) => p,
        None => return ResultFlags::PENDING_ID_NOT_FOUND,
    };

    // Check state
    if pending.state != PendingState::Pending {
        return ResultFlags::PENDING_ALREADY_COMMITTED;
    }

    // Release pending funds
    let transfer = &pending.transfer;
    self.update_pending_balances(transfer, false);

    // Update pending state
    pending.state = PendingState::Voided;

    // Write void to WAL
    self.wal.append(WALEntry {
        entry_type: EntryType::TransferVoid,
        data: void.to_bytes(),
    });

    ResultFlags::SUCCESS
}
```

### Timeout Processing

```
Automatic Timeout Handling:

Timeout Queue (Binary Heap - min-heap by timeout):
```
Timeout Queue (sorted by timeout_at):
┌─────────────────────────────────────────────────────────┐
│ (1711234567000000000, pending_id=100)  ◄── Earliest     │
│ (1711234568000000000, pending_id=101)                   │
│ (1711234569000000000, pending_id=102)                   │
│ (1711234570000000000, pending_id=103)                   │
│ (1711234571000000000, pending_id=104)  ◄── Latest       │
└─────────────────────────────────────────────────────────┘

Processing:
1. Check head of timeout queue
2. If timeout_at < current_time:
   - Void the pending transfer
   - Remove from queue
   - Repeat for next expired timeout
3. Stop when head timeout is in future

Efficiency: O(log N) per insert, O(1) to check next timeout
```

Timeout Sweep Code:
```rust
fn process_timeouts(&mut self) {
    let now = current_timestamp_ns();

    while let Some((timeout_at, pending_id)) = self.pending_table.peek_timeout() {
        if timeout_at > now {
            break; // No more expired timeouts
        }

        // Remove from queue
        self.pending_table.pop_timeout();

        // Void the pending transfer
        if let Some(pending) = self.pending_table.get_mut(&pending_id) {
            if pending.state == PendingState::Pending {
                // Release pending funds
                self.update_pending_balances(&pending.transfer, false);
                pending.state = PendingState::Voided;

                // Write timeout void to WAL
                self.wal.append(WALEntry {
                    entry_type: EntryType::TransferVoid,
                    data: Transfer {
                        id: pending_id,
                        pending_id,
                        flags: TransferFlags::VOID_PENDING,
                        ..Default::default()
                    }.to_bytes(),
                });
            }
        }
    }
}
```

## Part 3: Linked Transfers

### Atomic Transfer Groups

```
Linked Transfer Semantics:

Linked transfers execute atomically - all succeed or all fail:

Example: Triangular Transfer
┌───────────────────────────────────────────────────────────┐
│                                                            │
│  Account A ──$50──► Account B                              │
│  Account B ──$30──► Account C                              │
│                                                            │
│  Either both happen or neither happens                    │
│                                                            │
│  create_transfers([                                       │
│    Transfer {                                             │
│      id: 100,                                             │
│      debit_account_id: A,                                 │
│      credit_account_id: B,                                │
│      amount: 50,                                          │
│      flags: LINKED,  ◄── First in chain                   │
│    },                                                     │
│    Transfer {                                             │
│      id: 101,                                             │
│      debit_account_id: B,                                 │
│      credit_account_id: C,                                │
│      amount: 30,                                          │
│      flags: 0,  ◄── Last in chain (no LINKED flag)        │
│    },                                                     │
│  ]);                                                      │
│                                                            │
└────────────────────────────────────────────────────────────┘

LINKED flag semantics:
- Transfer with LINKED: First/intermediate in chain
- Transfer without LINKED: Last in chain
- All transfers in request are atomic
```

### Linked Transfer Validation

```
Linked Transfer Validation Rules:

┌───────────────────────────────────────────────────────────┐
│ Rule 1: LINKED flag pattern                               │
│                                                          │
│ Valid:                                                   │
│   [LINKED, LINKED, LINKED, none]  ◄── 4 transfers       │
│   [LINKED, none]                   ◄── 2 transfers       │
│   [none]                          ◄── 1 transfer         │
│                                                          │
│ Invalid:                                                 │
│   [none, LINKED]  ◄── LINKED after non-LINKED           │
│   [LINKED]        ◄── LINKED with no terminating transfer│
└───────────────────────────────────────────────────────────┘

┌───────────────────────────────────────────────────────────┐
│ Rule 2: Balance validation (deferred)                    │
│                                                          │
│ For linked transfers, validate total effect:             │
│                                                          │
│ Account B:                                               │
│   Credit: +$50 (from A)                                  │
│   Debit:  -$30 (to C)                                    │
│   Net: +$20                                              │
│                                                          │
│ Even if B starts with $0, this is valid because          │
│ the +$50 credit happens before the -$30 debit            │
│ (within the atomic group)                                │
└───────────────────────────────────────────────────────────┘

┌───────────────────────────────────────────────────────────┐
│ Rule 3: All-or-nothing execution                          │
│                                                          │
│ If ANY transfer in linked group fails:                   │
│   - ALL transfers fail                                   │
│   - No balances modified                                 │
│   - Client receives error for first failing transfer     │
│                                                          │
│ Example failure:                                         │
│   Transfer 1 (A→B): Would succeed                        │
│   Transfer 2 (B→C): Insufficient funds                   │
│   Result: BOTH fail, no changes                          │
└───────────────────────────────────────────────────────────┘
```

### Linked Transfer Implementation

```rust
/// Process linked transfers atomically
fn create_linked_transfers(
    &mut self,
    transfers: Vec<Transfer>,
) -> Vec<ResultFlags> {
    // Validate LINKED flag pattern
    if !Self::validate_linked_pattern(&transfers) {
        return vec![ResultFlags::INVALID_LINKED_FLAG; transfers.len()];
    }

    // Pre-validate all transfers (without applying)
    let validations: Vec<ValidationResult> = transfers
        .iter()
        .map(|t| self.pre_validate_transfer(t))
        .collect();

    // Check if any validation failed
    if let Some((idx, validation)) = validations.iter().enumerate()
        .find(|(_, v)| !v.is_valid())
    {
        // All transfers fail with the first error
        let mut results = vec![ResultFlags::SUCCESS; transfers.len()];
        results[idx] = validation.error_flags;
        return results;
    }

    // Simulate net effect on each account
    let net_effects = self.calculate_net_effects(&transfers);

    // Validate net effects don't violate constraints
    for (account_id, net_effect) in &net_effects {
        let account = self.get_account(*account_id);

        // Check debits limit
        if account.flags.contains(AccountFlags::DEBITS_MUST_NOT_EXCEED_CREDITS) {
            let new_debits = account.debits_posted + net_effect.total_debits;
            let new_credits = account.credits_posted + net_effect.total_credits;
            if new_debits > new_credits {
                return vec![ResultFlags::WOULD_EXCEED_DEBITS_LIMIT; transfers.len()];
            }
        }
    }

    // All validations passed - apply all transfers atomically
    let mut results = Vec::with_capacity(transfers.len());

    for (transfer, validation) in transfers.iter().zip(validations.iter()) {
        // Write to WAL
        self.wal.append(WALEntry {
            entry_type: EntryType::TransferCreate,
            data: transfer.to_bytes(),
        });

        // Apply to accounts
        self.apply_transfer(transfer);

        // Write transfer record
        self.write_transfer(transfer);

        results.push(ResultFlags::SUCCESS);
    }

    results
}

/// Calculate net effect of linked transfers on each account
fn calculate_net_effects(&self, transfers: &[Transfer]) -> HashMap<u128, NetEffect> {
    let mut effects: HashMap<u128, NetEffect> = HashMap::new();

    for transfer in transfers {
        // Debit account effect
        let debit_effect = effects.entry(transfer.debit_account_id).or_default();
        debit_effect.total_debits += transfer.amount;

        // Credit account effect
        let credit_effect = effects.entry(transfer.credit_account_id).or_default();
        credit_effect.total_credits += transfer.amount;
    }

    effects
}

struct NetEffect {
    total_debits: u64,
    total_credits: u64,
}
```

## Part 4: Balance Validation

### Balance Check Strategies

```
Balance Validation Approaches:

┌───────────────────────────────────────────────────────────┐
│ Strict Validation (default)                               │
│                                                          │
│ For each transfer:                                        │
│ 1. Read current debit account balance                     │
│ 2. Check: credits_posted >= debits_posted + amount        │
│ 3. If not: INSUFFICIENT_FUNDS error                       │
│                                                          │
│ Pros: Catches errors immediately                          │
│ Cons: Requires read before write                          │
└───────────────────────────────────────────────────────────┘

┌───────────────────────────────────────────────────────────┐
│ Optimistic Validation                                     │
│                                                          │
│ For each transfer:                                        │
│ 1. Apply transfer (may go negative)                       │
│ 2. Check final balance                                    │
│ 3. If negative: rollback and return error                 │
│                                                          │
│ Pros: Single pass                                         │
│ Cons: Rollback complexity                                 │
└───────────────────────────────────────────────────────────┘

┌───────────────────────────────────────────────────────────┐
│ Deferred Validation (linked transfers)                    │
│                                                          │
│ For linked group:                                         │
│ 1. Calculate net effect on each account                   │
│ 2. Validate net effect doesn't violate constraints        │
│ 3. Apply all transfers atomically                         │
│                                                          │
│ Pros: Allows intermediate negative balances               │
│ Cons: More complex validation logic                       │
└───────────────────────────────────────────────────────────┘

TigerBeetle uses Strict Validation for single transfers
and Deferred Validation for linked transfers.
```

### Account Flags Impact on Validation

```
Account Flag Effects:

┌───────────────────────────────────────────────────────────┐
│ DEBITS_MUST_NOT_EXCEED_CREDITS                            │
│                                                          │
│ When set:                                                 │
│   - Account cannot have negative balance                  │
│   - Validation: credits_posted >= debits_posted + amount  │
│                                                          │
│ Typical for:                                              │
│   - Asset accounts (cash, receivables)                    │
│   - Customer accounts (prevent overdrafts)                │
│   - Escrow accounts                                       │
│                                                          │
│ When NOT set:                                             │
│   - Account can have negative balance                     │
│   - No validation on transfer creation                    │
│                                                          │
│ Typical for:                                              │
│   - Liability accounts (payables)                         │
│   - Internal accounts                                     │
└───────────────────────────────────────────────────────────┘

┌───────────────────────────────────────────────────────────┐
│ CREDITS_MUST_NOT_EXCEED_DEBITS                            │
│                                                          │
│ When set:                                                 │
│   - Account credits cannot exceed debits                  │
│   - Validation: debits_posted >= credits_posted + amount  │
│                                                          │
│ Typical for:                                              │
│   - Loan accounts (can't borrow more than limit)          │
│   - Credit card accounts                                  │
│                                                          │
│ When NOT set:                                             │
│   - No limit on credits                                   │
│                                                          │
│ Typical for:                                              │
│   - Revenue accounts                                      │
│   - Income accounts                                       │
└───────────────────────────────────────────────────────────┘
```

### Balance Validation Code

```rust
/// Validate transfer against account balances
fn validate_balance(&self, transfer: &Transfer) -> ValidationResult {
    let debit_account = match self.get_account(transfer.debit_account_id) {
        Some(a) => a,
        None => return ValidationResult::error(ResultFlags::DEBIT_ACCOUNT_NOT_FOUND),
    };

    let credit_account = match self.get_account(transfer.credit_account_id) {
        Some(a) => a,
        None => return ValidationResult::error(ResultFlags::CREDIT_ACCOUNT_NOT_FOUND),
    };

    // Check debit account balance
    if debit_account.flags.contains(AccountFlags::DEBITS_MUST_NOT_EXCEED_CREDITS) {
        let available = debit_account.credits_posted - debit_account.debits_posted;
        if transfer.amount > available {
            return ValidationResult::error(ResultFlags::INSUFFICIENT_FUNDS);
        }
    }

    // Check credit account limit
    if credit_account.flags.contains(AccountFlags::CREDITS_MUST_NOT_EXCEED_DEBITS) {
        let room = credit_account.debits_posted - credit_account.credits_posted;
        if transfer.amount > room {
            return ValidationResult::error(ResultFlags::WOULD_EXCEED_CREDITS_LIMIT);
        }
    }

    ValidationResult::success()
}

/// For linked transfers, validate net effect
fn validate_net_effects(
    &self,
    transfers: &[Transfer],
    net_effects: &HashMap<u128, NetEffect>,
) -> ValidationResult {
    for (account_id, effect) in net_effects {
        let account = match self.get_account(*account_id) {
            Some(a) => a,
            None => return ValidationResult::error(ResultFlags::ACCOUNT_NOT_FOUND),
        };

        let new_debits = account.debits_posted + effect.total_debits;
        let new_credits = account.credits_posted + effect.total_credits;

        // Check debits limit
        if account.flags.contains(AccountFlags::DEBITS_MUST_NOT_EXCEED_CREDITS) {
            if new_debits > new_credits {
                return ValidationResult::error(ResultFlags::WOULD_EXCEED_DEBITS_LIMIT);
            }
        }

        // Check credits limit
        if account.flags.contains(AccountFlags::CREDITS_MUST_NOT_EXCEED_DEBITS) {
            if new_credits > new_debits {
                return ValidationResult::error(ResultFlags::WOULD_EXCEED_CREDITS_LIMIT);
            }
        }
    }

    ValidationResult::success()
}
```

## Part 5: Execution Pipeline Optimizations

### Pipelined Execution

```
Pipelined Request Processing:

Without Pipelining:
Request 1: Parse ──► Validate ──► Execute ──► Respond (500μs)
Request 2:                                     Parse ──► ... (500μs)
Total: 1000μs for 2 requests

With Pipelining:
Request 1: Parse ──► Validate ──► Execute ──► Respond
Request 2:            Parse ──► Validate ──► Execute ──► Respond
Request 3:                       Parse ──► Validate ──► Execute
Total: ~750μs for 3 requests (250μs/request avg)

TigerBeetle Pipeline Stages:
┌─────────────────────────────────────────────────────────┐
│ Stage 1: Network Read      (50μs)  ◄─── Parallel        │
│ Stage 2: Parse Request     (50μs)  ◄─── Parallel        │
│ Stage 3: Validate          (100μs) ◄─── Sequential      │
│ Stage 4: Execute           (200μs) ◄─── Sequential      │
│ Stage 5: Network Write     (50μs)  ◄─── Parallel        │
└─────────────────────────────────────────────────────────┘

Sequential stages (3, 4) run on single executor thread
Parallel stages (1, 2, 5) can overlap
```

### Batch Processing

```
Batch Transfer Processing:

Single Transfer:
Latency: 500μs
Throughput: 2,000 transfers/second

Batch of 100 Transfers:
Latency: 5ms (total for batch)
Throughput: 20,000 transfers/second (10x improvement)

Batch Processing Code:
```rust
fn process_batch(&mut self, batch: Vec<Transfer>) -> Vec<ResultFlags> {
    // Pre-sort by account to improve cache locality
    let mut sorted = batch.iter().enumerate().collect::<Vec<_>>();
    sorted.sort_by_key(|(_, t)| t.debit_account_id);

    // Group by account for batch validation
    let mut by_account: HashMap<u128, Vec<&Transfer>> = HashMap::new();
    for transfer in &batch {
        by_account.entry(transfer.debit_account_id).or_default().push(transfer);
    }

    // Validate all transfers
    let results: Vec<ResultFlags> = batch.iter()
        .map(|t| self.validate_transfer(t))
        .collect();

    // Check for any errors
    if results.iter().any(|r| !r.is_success()) {
        // Return error for all
        return results;
    }

    // Apply all transfers
    for transfer in &batch {
        self.apply_transfer(transfer);
    }

    vec![ResultFlags::SUCCESS; batch.len()]
}
```

### Cache Locality

```
Data Layout for Cache Efficiency:

Account Access Pattern:
```
Typical transfer:
1. Read debit account
2. Read credit account
3. Update debit account
4. Update credit account

Without optimization:
- Random access to account table
- Cache miss rate: ~80%
- Memory latency: 100ns per access

With hot/cold separation:
- Hot data (IDs, balances) in contiguous array
- Cold data (metadata) separate
- Cache miss rate: ~20%
- Memory latency: 20ns per access (cached)
```

Hot/Cold Account Structure:
```rust
/// Hot data - frequently accessed, fits in L1 cache
#[repr(C, align(64))]  // Cache line aligned
struct AccountHot {
    id: u128,           // 16 bytes
    debits_posted: u64, // 8 bytes
    credits_posted: u64,// 8 bytes
    debits_pending: u64,// 8 bytes
    credits_pending: u64,// 8 bytes
    // Total: 48 bytes (fits in one cache line with padding)
}

/// Cold data - rarely accessed
struct AccountCold {
    user_data: u128,    // 16 bytes
    ledger: u32,        // 4 bytes
    code: u16,          // 2 bytes
    flags: u16,         // 2 bytes
    // Total: 24 bytes
}

/// Full account (for creation/reconstruction)
struct Account {
    hot: AccountHot,
    cold: AccountCold,
}

/// Storage layout
struct AccountTable {
    hot_data: Vec<AccountHot>,  // Contiguous, prefetchable
    cold_data: Vec<AccountCold>, // Separate, loaded on demand
    index: HashMap<u128, usize>, // ID -> offset
}
```

## Part 6: Error Handling

### Error Classification

```
TigerBeetle Error Categories:

┌───────────────────────────────────────────────────────────┐
│ CLIENT_ERROR (4xx) - Invalid request                        │
│                                                          │
│ - 4001: INVALID_ID_FORMAT                                │
│ - 4002: INVALID_LEDGER                                   │
│ - 4003: INVALID_CODE                                     │
│ - 4004: INVALID_FLAG                                     │
│ - 4005: ID_EXISTS                                        │
│ - 4006: ACCOUNT_NOT_FOUND                                │
│ - 4007: LEDGER_MISMATCH                                  │
│ - 4008: INSUFFICIENT_FUNDS                               │
│ - 4009: WOULD_EXCEED_DEBITS_LIMIT                        │
│ - 4010: WOULD_EXCEED_CREDITS_LIMIT                       │
│ - 4011: PENDING_ID_NOT_FOUND                             │
│ - 4012: PENDING_ALREADY_COMMITTED                        │
│ - 4013: TIMEOUT_EXPIRED                                  │
│ - 4014: INVALID_LINKED_FLAG                              │
└───────────────────────────────────────────────────────────┘

┌───────────────────────────────────────────────────────────┐
│ SERVER_ERROR (5xx) - Internal error                        │
│                                                          │
│ - 5001: WAL_FULL                                         │
│ - 5002: CHECKPOINT_FAILED                                │
│ - 5003: RECOVERY_FAILED                                  │
│ - 5004: DATA_CORRUPTION                                  │
│ - 5005: OUT_OF_MEMORY                                    │
│ - 5006: INTERNAL_ERROR                                   │
└───────────────────────────────────────────────────────────┘

Error handling strategy:
- Client errors: Return immediately, no rollback needed
- Server errors: Panic, trigger recovery on restart
```

### Result Flags Encoding

```
Result Flags Bit Field (u32):

Bit Layout:
┌────────┬────────────────────────────────────────────────┐
│ 31..16 │ 15..0                                         │
│ Reserved │ Error Flags                                │
└────────┴────────────────────────────────────────────────┘

Error Flag Values:
- 0x0000: SUCCESS
- 0x0001: ID_EXISTS
- 0x0002: ACCOUNT_NOT_FOUND
- 0x0004: LEDGER_MISMATCH
- 0x0008: INVALID_CODE
- 0x0010: INVALID_FLAG
- 0x0020: INSUFFICIENT_FUNDS
- 0x0040: WOULD_EXCEED_DEBITS_LIMIT
- 0x0080: WOULD_EXCEED_CREDITS_LIMIT
- 0x0100: PENDING_ID_NOT_FOUND
- 0x0200: PENDING_ALREADY_COMMITTED
- 0x0400: TIMEOUT_EXPIRED
- 0x0800: INVALID_LINKED_FLAG
- 0x1000: DEBIT_ACCOUNT_NOT_FOUND
- 0x2000: CREDIT_ACCOUNT_NOT_FOUND

Multiple flags can be OR'd together for compound errors.

Response format:
```rust
struct CreateTransfersResult {
    results: Vec<u32>,  // One result flag per transfer
}

// Example response for batch of 3 transfers:
// [0x0000, 0x0020, 0x0000]
// Transfer 1: SUCCESS
// Transfer 2: INSUFFICIENT_FUNDS
// Transfer 3: SUCCESS
```

---

*This document is part of the TigerBeetle exploration series. See [exploration.md](./exploration.md) for the complete index.*
