---
title: "TigerBeetle Valtron Integration"
subtitle: "Edge deployment patterns with Lambda and algebraic effects"
location: /home/darkvoid/Boxxed/@dev/repo-expolorations/src.db/src.tigerbeetle
related: exploration.md, rust-revision.md
---

# 04 - TigerBeetle Valtron Integration

## Overview

This document covers integrating TigerBeetle with Valtron for edge deployment - implementing the TaskIterator pattern for financial transactions, RESP-like protocol handling, and Lambda deployment strategies.

## Part 1: TaskIterator Pattern for Financial Transactions

### TigerBeetle Task Types

```rust
//! TigerBeetle task iterator using Valtron pattern
//!
//! No async/await - uses algebraic effects via TaskResult

use valtron::TaskResult;

/// TigerBeetle operation types
#[derive(Debug, Clone)]
pub enum TigerBeetleOp {
    /// Create accounts
    CreateAccounts {
        accounts: Vec<Account>,
    },
    /// Create transfers
    CreateTransfers {
        transfers: Vec<Transfer>,
    },
    /// Lookup accounts
    LookupAccounts {
        account_ids: Vec<AccountId>,
    },
    /// Lookup transfers
    LookupTransfers {
        transfer_ids: Vec<TransferId>,
    },
    /// Two-phase commit
    CommitPending {
        pending_id: TransferId,
    },
    /// Void pending transfer
    VoidPending {
        pending_id: TransferId,
    },
}

/// Task result for TigerBeetle operations
pub type TigerBeetleResult<T> = TaskResult<T, TigerBeetleEffect>;

/// Effects needed for TigerBeetle operations
#[derive(Debug, Clone)]
pub enum TigerBeetleEffect {
    /// Network I/O (send to TigerBeetle cluster)
    NetworkSend {
        peer: String,
        data: Vec<u8>,
    },
    /// Network I/O (receive from cluster)
    NetworkRecv {
        timeout_ms: u64,
    },
    /// Local storage (WAL)
    StorageWrite {
        offset: u64,
        data: Vec<u8>,
    },
    /// Local storage (WAL read)
    StorageRead {
        offset: u64,
        length: usize,
    },
    /// Get current timestamp
    Timestamp,
}

/// Account structure (simplified)
#[derive(Debug, Clone)]
pub struct Account {
    pub id: u128,
    pub user_data: u128,
    pub ledger: u32,
    pub code: u16,
    pub flags: u16,
}

/// Transfer structure (simplified)
#[derive(Debug, Clone)]
pub struct Transfer {
    pub id: u128,
    pub debit_account_id: u128,
    pub credit_account_id: u128,
    pub amount: u64,
    pub ledger: u32,
    pub code: u16,
    pub flags: u16,
    pub timestamp: u64,
    pub timeout: u64,
    pub pending_id: Option<u128>,
}

/// Account/Transfer IDs
pub type AccountId = u128;
pub type TransferId = u128;
```

### TaskIterator Implementation

```rust
//! TaskIterator for TigerBeetle operations

use std::collections::VecDeque;

/// Result flags (matching TigerBeetle protocol)
#[derive(Debug, Clone, Copy)]
pub struct ResultFlags(pub u32);

impl ResultFlags {
    pub const SUCCESS: Self = Self(0x0000);
    pub const ID_EXISTS: Self = Self(0x0001);
    pub const ACCOUNT_NOT_FOUND: Self = Self(0x0002);
    pub const LEDGER_MISMATCH: Self = Self(0x0004);
    pub const INVALID_CODE: Self = Self(0x0008);
    pub const INVALID_FLAG: Self = Self(0x0010);
    pub const INSUFFICIENT_FUNDS: Self = Self(0x0020);
    pub const WOULD_EXCEED_DEBITS_LIMIT: Self = Self(0x0040);
    pub const WOULD_EXCEED_CREDITS_LIMIT: Self = Self(0x0080);
    pub const PENDING_ID_NOT_FOUND: Self = Self(0x0100);
    pub const PENDING_ALREADY_COMMITTED: Self = Self(0x0200);
    pub const TIMEOUT_EXPIRED: Self = Self(0x0400);
}

/// TaskIterator for processing TigerBeetle operations
pub struct TigerBeetleIterator {
    /// Pending operations
    queue: VecDeque<PendingOp>,
    /// Current batch being processed
    current_batch: Option<Batch>,
    /// Results accumulator
    results: Vec<ResultFlags>,
}

struct PendingOp {
    op: TigerBeetleOp,
    result_tx: std::sync::mpsc::Sender<Vec<ResultFlags>>,
}

struct Batch {
    ops: Vec<TigerBeetleOp>,
    current_index: usize,
}

impl TigerBeetleIterator {
    pub fn new() -> Self {
        Self {
            queue: VecDeque::new(),
            current_batch: None,
            results: Vec::new(),
        }
    }

    /// Submit operation for processing
    pub fn submit(&mut self, op: TigerBeetleOp) -> TigerBeetleResult<Vec<ResultFlags>> {
        let (tx, rx) = std::sync::mpsc::channel();
        self.queue.push_back(PendingOp { op, result_tx: tx });

        // Try to process immediately
        self.process_queue()?;

        // Receive result (blocking in iterator context)
        match rx.recv() {
            Ok(results) => TaskResult::Complete(results),
            Err(_) => TaskResult::Complete(vec![ResultFlags(0x9999)]), // Internal error
        }
    }

    /// Process queued operations
    fn process_queue(&mut self) -> TigerBeetleResult<()> {
        if let Some(batch) = &mut self.current_batch {
            // Continue processing current batch
            return self.process_batch(batch);
        }

        // Start new batch from queue
        if !self.queue.is_empty() {
            let ops: Vec<TigerBeetleOp> = self.queue.drain(..).map(|p| p.op).collect();
            let batch = Batch {
                ops,
                current_index: 0,
            };
            return self.process_batch(&mut batch);
        }

        TaskResult::Complete(())
    }

    /// Process a batch of operations
    fn process_batch(&mut self, batch: &mut Batch) -> TigerBeetleResult<()> {
        while batch.current_index < batch.ops.len() {
            let op = &batch.ops[batch.current_index];

            match self.execute_op(op)? {
                TaskResult::Complete(result) => {
                    self.results.push(result);
                    batch.current_index += 1;
                }
                TaskResult::Effect(effect, cont) => {
                    // Yield to effect handler
                    return TaskResult::Effect(effect, cont);
                }
                TaskResult::Suspend => {
                    return TaskResult::Suspend;
                }
            }
        }

        // Batch complete
        self.current_batch = None;
        TaskResult::Complete(())
    }

    /// Execute single operation
    fn execute_op(&self, op: &TigerBeetleOp) -> TigerBeetleResult<ResultFlags> {
        match op {
            TigerBeetleOp::CreateAccounts { accounts } => {
                self.create_accounts(accounts)
            }
            TigerBeetleOp::CreateTransfers { transfers } => {
                self.create_transfers(transfers)
            }
            TigerBeetleOp::LookupAccounts { account_ids } => {
                self.lookup_accounts(account_ids)
            }
            TigerBeetleOp::LookupTransfers { transfer_ids } => {
                self.lookup_transfers(transfer_ids)
            }
            TigerBeetleOp::CommitPending { pending_id } => {
                self.commit_pending(pending_id)
            }
            TigerBeetleOp::VoidPending { pending_id } => {
                self.void_pending(pending_id)
            }
        }
    }

    /// Create accounts
    fn create_accounts(&self, accounts: &[Account]) -> TigerBeetleResult<ResultFlags> {
        // Serialize accounts
        let mut data = Vec::new();
        for account in accounts {
            data.extend_from_slice(&account.id.to_le_bytes());
            data.extend_from_slice(&account.user_data.to_le_bytes());
            data.extend_from_slice(&account.ledger.to_le_bytes());
            data.extend_from_slice(&account.code.to_le_bytes());
            data.extend_from_slice(&account.flags.to_le_bytes());
        }

        // Send to TigerBeetle cluster
        let response = self.effect(TigerBeetleEffect::NetworkSend {
            peer: "tigerbeetle:3001".to_string(),
            data,
        })?;

        // Parse response (simplified)
        TaskResult::Complete(ResultFlags::SUCCESS)
    }

    /// Create transfers
    fn create_transfers(&self, transfers: &[Transfer]) -> TigerBeetleResult<ResultFlags> {
        // Serialize transfers
        let mut data = Vec::new();
        for transfer in transfers {
            data.extend_from_slice(&transfer.id.to_le_bytes());
            data.extend_from_slice(&transfer.debit_account_id.to_le_bytes());
            data.extend_from_slice(&transfer.credit_account_id.to_le_bytes());
            data.extend_from_slice(&transfer.amount.to_le_bytes());
            data.extend_from_slice(&transfer.ledger.to_le_bytes());
            data.extend_from_slice(&transfer.code.to_le_bytes());
            data.extend_from_slice(&transfer.flags.to_le_bytes());
            data.extend_from_slice(&transfer.timestamp.to_le_bytes());
            data.extend_from_slice(&transfer.timeout.to_le_bytes());
            if let Some(pending_id) = transfer.pending_id {
                data.extend_from_slice(&pending_id.to_le_bytes());
            } else {
                data.extend_from_slice(&[0u8; 16]);
            }
        }

        // Send to TigerBeetle cluster
        let response = self.effect(TigerBeetleEffect::NetworkSend {
            peer: "tigerbeetle:3001".to_string(),
            data,
        })?;

        TaskResult::Complete(ResultFlags::SUCCESS)
    }

    /// Lookup accounts
    fn lookup_accounts(&self, account_ids: &[AccountId]) -> TigerBeetleResult<ResultFlags> {
        let mut data = Vec::new();
        for id in account_ids {
            data.extend_from_slice(&id.to_le_bytes());
        }

        let response = self.effect(TigerBeetleEffect::NetworkSend {
            peer: "tigerbeetle:3001".to_string(),
            data,
        })?;

        TaskResult::Complete(ResultFlags::SUCCESS)
    }

    /// Lookup transfers
    fn lookup_transfers(&self, transfer_ids: &[TransferId]) -> TigerBeetleResult<ResultFlags> {
        let mut data = Vec::new();
        for id in transfer_ids {
            data.extend_from_slice(&id.to_le_bytes());
        }

        let response = self.effect(TigerBeetleEffect::NetworkSend {
            peer: "tigerbeetle:3001".to_string(),
            data,
        })?;

        TaskResult::Complete(ResultFlags::SUCCESS)
    }

    /// Commit pending transfer
    fn commit_pending(&self, pending_id: &TransferId) -> TigerBeetleResult<ResultFlags> {
        // Create commit transfer
        let commit = Transfer {
            id: generate_id(),
            debit_account_id: 0, // Special account for commits
            credit_account_id: 0,
            amount: 0,
            ledger: 0,
            code: 0,
            flags: 0x0004, // POST_PENDING_TRANSFER
            timestamp: 0,
            timeout: 0,
            pending_id: Some(*pending_id),
        };

        self.create_transfers(&[commit])
    }

    /// Void pending transfer
    fn void_pending(&self, pending_id: &TransferId) -> TigerBeetleResult<ResultFlags> {
        // Create void transfer
        let void = Transfer {
            id: generate_id(),
            debit_account_id: 0,
            credit_account_id: 0,
            amount: 0,
            ledger: 0,
            code: 0,
            flags: 0x0008, // VOID_PENDING_TRANSFER
            timestamp: 0,
            timeout: 0,
            pending_id: Some(*pending_id),
        };

        self.create_transfers(&[void])
    }

    /// Helper to yield effect
    fn effect<T: 'static>(&self, effect: TigerBeetleEffect) -> TigerBeetleResult<T> {
        TaskResult::Effect(effect, Box::new(|value| {
            TaskResult::Complete(value.downcast().unwrap())
        }))
    }
}

/// Generate unique ID
fn generate_id() -> u128 {
    use std::time::{SystemTime, UNIX_EPOCH};
    let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
    now.as_nanos() as u128
}
```

## Part 2: Edge Cache Patterns

### Two-Phase Commit for Edge Payments

```rust
//! Edge payment processing with two-phase commit

use crate::iterator::{TigerBeetleIterator, TigerBeetleOp, TigerBeetleResult, ResultFlags};

/// Edge payment processor
pub struct EdgePaymentProcessor {
    /// Task iterator
    iterator: TigerBeetleIterator,
    /// Pending payments (idempotency tracking)
    pending_payments: std::collections::HashMap<String, u128>,
}

/// Payment request
#[derive(Debug, Clone)]
pub struct PaymentRequest {
    /// Idempotency key
    pub idempotency_key: String,

    /// Source account
    pub from_account: u128,

    /// Destination account
    pub to_account: u128,

    /// Amount (in cents)
    pub amount_cents: u64,

    /// Ledger
    pub ledger: u32,

    /// Timeout for two-phase commit (seconds)
    pub timeout_secs: u64,
}

/// Payment result
#[derive(Debug, Clone)]
pub struct PaymentResult {
    pub success: bool,
    pub transfer_id: Option<u128>,
    pub error: Option<String>,
}

impl EdgePaymentProcessor {
    pub fn new() -> Self {
        Self {
            iterator: TigerBeetleIterator::new(),
            pending_payments: std::collections::HashMap::new(),
        }
    }

    /// Process payment with two-phase commit
    pub fn process_payment(&mut self, request: PaymentRequest) -> TigerBeetleResult<PaymentResult> {
        // Check idempotency
        if let Some(transfer_id) = self.pending_payments.get(&request.idempotency_key) {
            // Already processed - return cached result
            return TaskResult::Complete(PaymentResult {
                success: true,
                transfer_id: Some(*transfer_id),
                error: None,
            });
        }

        // Phase 1: Create pending transfer
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos() as u64;

        let timeout_ns = now + (request.timeout_secs * 1_000_000_000);

        let pending_transfer = crate::iterator::Transfer {
            id: generate_id(),
            debit_account_id: request.from_account,
            credit_account_id: request.to_account,
            amount: request.amount_cents,
            ledger: request.ledger,
            code: 1, // Payment code
            flags: 0x0002, // PENDING
            timestamp: now,
            timeout: timeout_ns,
            pending_id: None,
        };

        let result = self.iterator.submit(TigerBeetleOp::CreateTransfers {
            transfers: vec![pending_transfer.clone()],
        })?;

        // Check result
        if result[0].0 != ResultFlags::SUCCESS.0 {
            return TaskResult::Complete(PaymentResult {
                success: false,
                transfer_id: None,
                error: Some(format!("Transfer failed: {:?}", result[0])),
            });
        }

        // Track pending payment
        self.pending_payments.insert(
            request.idempotency_key.clone(),
            pending_transfer.id,
        );

        // Return pending result (client must confirm)
        TaskResult::Complete(PaymentResult {
            success: true,
            transfer_id: Some(pending_transfer.id),
            error: None,
        })
    }

    /// Confirm payment (complete two-phase commit)
    pub fn confirm_payment(&mut self, idempotency_key: &str) -> TigerBeetleResult<PaymentResult> {
        let pending_id = match self.pending_payments.get(idempotency_key) {
            Some(id) => *id,
            None => {
                return TaskResult::Complete(PaymentResult {
                    success: false,
                    transfer_id: None,
                    error: Some("Payment not found".to_string()),
                });
            }
        };

        // Commit pending transfer
        let result = self.iterator.submit(TigerBeetleOp::CommitPending {
            pending_id,
        })?;

        // Remove from pending
        self.pending_payments.remove(idempotency_key);

        if result[0].0 == ResultFlags::SUCCESS.0 {
            TaskResult::Complete(PaymentResult {
                success: true,
                transfer_id: Some(pending_id),
                error: None,
            })
        } else {
            TaskResult::Complete(PaymentResult {
                success: false,
                transfer_id: None,
                error: Some(format!("Commit failed: {:?}", result[0])),
            })
        }
    }

    /// Cancel payment (void two-phase commit)
    pub fn cancel_payment(&mut self, idempotency_key: &str) -> TigerBeetleResult<PaymentResult> {
        let pending_id = match self.pending_payments.get(idempotency_key) {
            Some(id) => *id,
            None => {
                return TaskResult::Complete(PaymentResult {
                    success: false,
                    transfer_id: None,
                    error: Some("Payment not found".to_string()),
                });
            }
        };

        // Void pending transfer
        let result = self.iterator.submit(TigerBeetleOp::VoidPending {
            pending_id,
        })?;

        // Remove from pending
        self.pending_payments.remove(idempotency_key);

        if result[0].0 == ResultFlags::SUCCESS.0 {
            TaskResult::Complete(PaymentResult {
                success: true,
                transfer_id: Some(pending_id),
                error: None,
            })
        } else {
            TaskResult::Complete(PaymentResult {
                success: false,
                transfer_id: None,
                error: Some(format!("Void failed: {:?}", result[0])),
            })
        }
    }
}
```

### Rate Limiting with TigerBeetle

```rust
//! Rate limiting using TigerBeetle accounts

use crate::iterator::{TigerBeetleIterator, TigerBeetleOp, TigerBeetleResult, ResultFlags};

/// Rate limiter using TigerBeetle token bucket
pub struct TigerBeetleRateLimiter {
    iterator: TigerBeetleIterator,
    /// Token bucket accounts: user_id -> account_id
    bucket_accounts: std::collections::HashMap<String, u128>,
}

impl TigerBeetleRateLimiter {
    pub fn new() -> Self {
        Self {
            iterator: TigerBeetleIterator::new(),
            bucket_accounts: std::collections::HashMap::new(),
        }
    }

    /// Create rate limit bucket for user
    pub fn create_bucket(&mut self, user_id: &str, tokens: u64) -> TigerBeetleResult<ResultFlags> {
        let account_id = generate_id();

        let account = crate::iterator::Account {
            id: account_id,
            user_data: 0,
            ledger: 1,
            code: 1, // Token bucket account
            flags: 0x0002, // DEBITS_MUST_NOT_EXCEED_CREDITS
        };

        // Create account with initial tokens (credits)
        let result = self.iterator.submit(TigerBeetleOp::CreateAccounts {
            accounts: vec![account.clone()],
        })?;

        if result[0].0 == ResultFlags::SUCCESS.0 {
            // Initialize bucket with tokens
            let init_transfer = crate::iterator::Transfer {
                id: generate_id(),
                debit_account_id: 0, // System account
                credit_account_id: account_id,
                amount: tokens,
                ledger: 1,
                code: 1,
                flags: 0,
                timestamp: 0,
                timeout: 0,
                pending_id: None,
            };

            self.iterator.submit(TigerBeetleOp::CreateTransfers {
                transfers: vec![init_transfer],
            })?;

            self.bucket_accounts.insert(user_id.to_string(), account_id);
        }

        TaskResult::Complete(result[0])
    }

    /// Check and consume rate limit
    pub fn consume(&mut self, user_id: &str, tokens: u64) -> TigerBeetleResult<bool> {
        let account_id = match self.bucket_accounts.get(user_id) {
            Some(id) => *id,
            None => {
                // No bucket - create one with default limit
                self.create_bucket(user_id, 1000)?;
                self.bucket_accounts.get(user_id).copied().unwrap()
            }
        };

        // Try to consume tokens (debit from bucket)
        let transfer = crate::iterator::Transfer {
            id: generate_id(),
            debit_account_id: account_id,
            credit_account_id: 0, // System account (tokens disappear)
            amount: tokens,
            ledger: 1,
            code: 2, // Rate limit consumption
            flags: 0,
            timestamp: 0,
            timeout: 0,
            pending_id: None,
        };

        let result = self.iterator.submit(TigerBeetleOp::CreateTransfers {
            transfers: vec![transfer],
        })?;

        // Check if succeeded (had enough tokens)
        let allowed = result[0].0 == ResultFlags::SUCCESS.0;

        TaskResult::Complete(allowed)
    }

    /// Refill rate limit bucket
    pub fn refill(&mut self, user_id: &str, tokens: u64) -> TigerBeetleResult<ResultFlags> {
        let account_id = match self.bucket_accounts.get(user_id) {
            Some(id) => *id,
            None => {
                return TaskResult::Complete(ResultFlags::ACCOUNT_NOT_FOUND);
            }
        };

        // Add tokens to bucket (credit from system)
        let transfer = crate::iterator::Transfer {
            id: generate_id(),
            debit_account_id: 0, // System account
            credit_account_id: account_id,
            amount: tokens,
            ledger: 1,
            code: 2,
            flags: 0,
            timestamp: 0,
            timeout: 0,
            pending_id: None,
        };

        let result = self.iterator.submit(TigerBeetleOp::CreateTransfers {
            transfers: vec![transfer],
        })?;

        TaskResult::Complete(result[0])
    }
}
```

## Part 3: Lambda Deployment

### TigerBeetle Lambda Handler

```rust
//! AWS Lambda handler for TigerBeetle edge cache

use valtron::{TaskResult, TaskIterator};
use crate::iterator::{TigerBeetleIterator, TigerBeetleOp, TigerBeetleEffect};

/// Lambda event (API Gateway)
#[derive(Debug, serde::Deserialize)]
pub struct LambdaEvent {
    pub resource: String,
    pub path: String,
    pub http_method: String,
    pub headers: std::collections::HashMap<String, String>,
    pub body: Option<String>,
}

/// Lambda response
#[derive(Debug, serde::Serialize)]
pub struct LambdaResponse {
    pub status_code: u16,
    pub headers: std::collections::HashMap<String, String>,
    pub body: String,
}

/// TigerBeetle Lambda state
pub struct TigerBeetleLambda {
    iterator: TigerBeetleIterator,
    tigerbeetle_endpoint: String,
}

impl TigerBeetleLambda {
    pub fn new() -> Self {
        let endpoint = std::env::var("TIGERBEETLE_ENDPOINT")
            .unwrap_or_else(|_| "tigerbeetle:3001".to_string());

        Self {
            iterator: TigerBeetleIterator::new(),
            tigerbeetle_endpoint: endpoint,
        }
    }

    /// Handle Lambda invocation
    pub fn handle(&mut self, event: LambdaEvent) -> Result<LambdaResponse, String> {
        match event.path.as_str() {
            "/accounts" => self.handle_accounts(event),
            "/transfers" => self.handle_transfers(event),
            "/payments" => self.handle_payments(event),
            _ => Ok(LambdaResponse {
                status_code: 404,
                headers: std::collections::HashMap::new(),
                body: serde_json::json!({"error": "Not found"}).to_string(),
            }),
        }
    }

    /// Handle account operations
    fn handle_accounts(&mut self, event: LambdaEvent) -> Result<LambdaResponse, String> {
        match event.http_method.as_str() {
            "POST" => {
                // Create accounts
                let body = event.body.ok_or("Missing body")?;
                let accounts: Vec<AccountCreateRequest> = serde_json::from_str(&body)
                    .map_err(|e| format!("Invalid JSON: {}", e))?;

                // Convert to TigerBeetle accounts
                let tb_accounts: Vec<crate::iterator::Account> = accounts.into_iter().map(|a| {
                    crate::iterator::Account {
                        id: a.id,
                        user_data: a.user_data.unwrap_or(0),
                        ledger: a.ledger,
                        code: a.code.unwrap_or(1),
                        flags: a.flags.unwrap_or(0),
                    }
                }).collect();

                // Submit to iterator
                let result = self.iterator.submit(TigerBeetleOp::CreateAccounts {
                    accounts: tb_accounts,
                }).map_err(|e| format!("Iterator error: {:?}", e))?;

                Ok(LambdaResponse {
                    status_code: 201,
                    headers: std::collections::HashMap::new(),
                    body: serde_json::json!({"results": result}).to_string(),
                })
            }
            _ => Ok(LambdaResponse {
                status_code: 405,
                headers: std::collections::HashMap::new(),
                body: serde_json::json!({"error": "Method not allowed"}).to_string(),
            }),
        }
    }

    /// Handle transfer operations
    fn handle_transfers(&mut self, event: LambdaEvent) -> Result<LambdaResponse, String> {
        match event.http_method.as_str() {
            "POST" => {
                let body = event.body.ok_or("Missing body")?;
                let transfers: Vec<TransferCreateRequest> = serde_json::from_str(&body)
                    .map_err(|e| format!("Invalid JSON: {}", e))?;

                // Convert to TigerBeetle transfers
                let tb_transfers: Vec<crate::iterator::Transfer> = transfers.into_iter().map(|t| {
                    crate::iterator::Transfer {
                        id: t.id,
                        debit_account_id: t.debit_account_id,
                        credit_account_id: t.credit_account_id,
                        amount: t.amount,
                        ledger: t.ledger,
                        code: t.code.unwrap_or(1),
                        flags: t.flags.unwrap_or(0),
                        timestamp: 0,
                        timeout: t.timeout.unwrap_or(0),
                        pending_id: t.pending_id,
                    }
                }).collect();

                let result = self.iterator.submit(TigerBeetleOp::CreateTransfers {
                    transfers: tb_transfers,
                }).map_err(|e| format!("Iterator error: {:?}", e))?;

                Ok(LambdaResponse {
                    status_code: 201,
                    headers: std::collections::HashMap::new(),
                    body: serde_json::json!({"results": result}).to_string(),
                })
            }
            _ => Ok(LambdaResponse {
                status_code: 405,
                headers: std::collections::HashMap::new(),
                body: serde_json::json!({"error": "Method not allowed"}).to_string(),
            }),
        }
    }

    /// Handle payment operations (with two-phase commit)
    fn handle_payments(&mut self, event: LambdaEvent) -> Result<LambdaResponse, String> {
        use crate::payment::{EdgePaymentProcessor, PaymentRequest};

        match event.http_method.as_str() {
            "POST" => {
                let body = event.body.ok_or("Missing body")?;
                let req: PaymentRequest = serde_json::from_str(&body)
                    .map_err(|e| format!("Invalid JSON: {}", e))?;

                let mut processor = EdgePaymentProcessor::new();
                let result = processor.process_payment(req)
                    .map_err(|e| format!("Payment error: {:?}", e))?;

                Ok(LambdaResponse {
                    status_code: 200,
                    headers: std::collections::HashMap::new(),
                    body: serde_json::json!(result).to_string(),
                })
            }
            _ => Ok(LambdaResponse {
                status_code: 405,
                headers: std::collections::HashMap::new(),
                body: serde_json::json!({"error": "Method not allowed"}).to_string(),
            }),
        }
    }
}

/// Account creation request
#[derive(Debug, serde::Deserialize)]
struct AccountCreateRequest {
    id: u128,
    user_data: Option<u128>,
    ledger: u32,
    code: Option<u16>,
    flags: Option<u16>,
}

/// Transfer creation request
#[derive(Debug, serde::Deserialize)]
struct TransferCreateRequest {
    id: u128,
    debit_account_id: u128,
    credit_account_id: u128,
    amount: u64,
    ledger: u32,
    code: Option<u16>,
    flags: Option<u16>,
    timeout: Option<u64>,
    pending_id: Option<u128>,
}

// Lambda handler entry point
#[cfg(feature = "lambda")]
use aws_lambda_events::encodings::Body;

#[cfg(feature = "lambda")]
pub async fn handler(event: APIGatewayProxyRequest) -> Result<APIGatewayProxyResponse, String> {
    // In real implementation, this would use Valtron's effect handling
    // For Lambda, we'd need to integrate with the Lambda runtime

    let mut lambda = TigerBeetleLambda::new();

    // Convert Lambda event
    let lambda_event = LambdaEvent {
        resource: event.resource.unwrap_or_default(),
        path: event.path,
        http_method: event.http_method,
        headers: event.headers,
        body: event.body,
    };

    // Handle request
    let response = lambda.handle(lambda_event)?;

    // Convert to Lambda response
    Ok(APIGatewayProxyResponse {
        status_code: response.status_code as i64,
        headers: response.headers,
        body: Some(Body::Text(response.body)),
        ..Default::default()
    })
}
```

### Lambda Deployment Configuration

```yaml
# serverless.yml - TigerBeetle Lambda deployment
service: tigerbeetle-edge

provider:
  name: aws
  runtime: provided.al2023
  architecture: arm64
  region: us-east-1
  memorySize: 3008  # Max for Lambda
  timeout: 29  # Under 30s for synchronous invocation
  environment:
    TIGERBEETLE_ENDPOINT: ${self:custom.tigerbeetleEndpoint}
    LOG_LEVEL: info
  vpc:
    securityGroupIds:
      - ${self:custom.securityGroupId}
    subnetIds:
      - ${self:custom.subnetId1}
      - ${self:custom.subnetId2}
  iamRoleStatements:
    - Effect: Allow
      Action:
        - ec2:CreateNetworkInterface
        - ec2:DescribeNetworkInterfaces
        - ec2:DeleteNetworkInterface
      Resource: '*'
    - Effect: Allow
      Action:
        - secretsmanager:GetSecretValue
      Resource: !Ref TigerBeetleCredentials

custom:
  tigerbeetleEndpoint: tigerbeetle.internal:3001
  securityGroupId: sg-xxxxx
  subnetId1: subnet-xxxxx
  subnetId2: subnet-xxxxx

functions:
  accounts:
    handler: target/lambda/tigerbeetle-edge/bootstrap
    events:
      - http:
          path: /accounts
          method: post
          cors: true
    reservedConcurrency: 100

  transfers:
    handler: target/lambda/tigerbeetle-edge/bootstrap
    events:
      - http:
          path: /transfers
          method: post
          cors: true
    reservedConcurrency: 100

  payments:
    handler: target/lambda/tigerbeetle-edge/bootstrap
    events:
      - http:
          path: /payments
          method: post
          cors: true
    reservedConcurrency: 50

resources:
  Resources:
    TigerBeetleCredentials:
      Type: AWS::SecretsManager::Secret
      Properties:
        Name: tigerbeetle/credentials
        SecretString: '{"admin_token": "xxx"}'
```

---

*This document is part of the TigerBeetle exploration series. See [exploration.md](./exploration.md) for the complete index.*
