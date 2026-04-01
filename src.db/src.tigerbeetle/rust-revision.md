---
title: "TigerBeetle Rust Revision"
subtitle: "Valtron-based Rust translation with algebraic effects"
location: /home/darkvoid/Boxxed/@dev/repo-expolorations/src.db/src.tigerbeetle
related: exploration.md, 00-zero-to-ledger-engineer.md
---

# Rust Revision: TigerBeetle with Valtron

## Overview

This document translates TigerBeetle's core concepts into Rust using the Valtron framework - implementing the storage engine, two-phase commit, and consensus protocol without async/await, using algebraic effects for I/O.

## Part 1: Core Data Structures

### Account and Transfer Types

```rust
//! TigerBeetle core types
//!
//! Following Valtron principles:
//! - No async/await
//! - Pure data structures
//! - Effects handled algebraically

/// Account identifier (128-bit)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct AccountId(pub u128);

/// Transfer identifier (128-bit)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TransferId(pub u128);

/// Ledger identifier (32-bit)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct LedgerId(pub u32);

/// Account flags (bitfield)
#[derive(Debug, Clone, Copy, Default)]
pub struct AccountFlags(pub u16);

impl AccountFlags {
    pub const LINKED: Self = Self(0x0001);
    pub const DEBITS_MUST_NOT_EXCEED_CREDITS: Self = Self(0x0002);
    pub const CREDITS_MUST_NOT_EXCEED_DEBITS: Self = Self(0x0004);
}

/// Account structure (128 bytes on disk)
#[derive(Debug, Clone, Copy)]
#[repr(C, align(16))]
pub struct Account {
    /// Unique account ID
    pub id: AccountId,

    /// Application-specific data
    pub user_data: u128,

    /// Ledger identifier
    pub ledger: LedgerId,

    /// Account type code
    pub code: u16,

    /// Account flags
    pub flags: AccountFlags,

    /// Pending debits (awaiting two-phase commit)
    pub debits_pending: u64,

    /// Posted debits (completed)
    pub debits_posted: u64,

    /// Pending credits (awaiting two-phase commit)
    pub credits_pending: u64,

    /// Posted credits (completed)
    pub credits_posted: u64,
}

impl Account {
    /// Calculate available balance
    pub fn balance(&self) -> i128 {
        (self.credits_posted as i128) - (self.debits_posted as i128)
    }

    /// Check if account can be debited
    pub fn can_debit(&self, amount: u64) -> bool {
        if self.flags.0 & AccountFlags::DEBITS_MUST_NOT_EXCEED_CREDITS.0 != 0 {
            self.credits_posted >= self.debits_posted + amount
        } else {
            true // No limit
        }
    }

    /// Serialize to bytes (for disk storage)
    pub fn to_bytes(&self) -> [u8; 128] {
        let mut bytes = [0u8; 128];

        bytes[0..16].copy_from_slice(&self.id.0.to_le_bytes());
        bytes[16..32].copy_from_slice(&self.user_data.to_le_bytes());
        bytes[32..36].copy_from_slice(&self.ledger.0.to_le_bytes());
        bytes[36..38].copy_from_slice(&self.code.to_le_bytes());
        bytes[38..40].copy_from_slice(&self.flags.0.to_le_bytes());
        bytes[40..48].copy_from_slice(&self.debits_pending.to_le_bytes());
        bytes[48..56].copy_from_slice(&self.debits_posted.to_le_bytes());
        bytes[56..64].copy_from_slice(&self.credits_pending.to_le_bytes());
        bytes[64..72].copy_from_slice(&self.credits_posted.to_le_bytes());
        // bytes[72..128] reserved

        bytes
    }

    /// Deserialize from bytes
    pub fn from_bytes(bytes: &[u8; 128]) -> Self {
        Self {
            id: AccountId(u128::from_le_bytes(bytes[0..16].try_into().unwrap())),
            user_data: u128::from_le_bytes(bytes[16..32].try_into().unwrap()),
            ledger: LedgerId(u32::from_le_bytes(bytes[32..36].try_into().unwrap())),
            code: u16::from_le_bytes(bytes[36..38].try_into().unwrap()),
            flags: AccountFlags(u16::from_le_bytes(bytes[38..40].try_into().unwrap())),
            debits_pending: u64::from_le_bytes(bytes[40..48].try_into().unwrap()),
            debits_posted: u64::from_le_bytes(bytes[48..56].try_into().unwrap()),
            credits_pending: u64::from_le_bytes(bytes[56..64].try_into().unwrap()),
            credits_posted: u64::from_le_bytes(bytes[64..72].try_into().unwrap()),
        }
    }
}
```

### Transfer Types

```rust
/// Transfer flags (bitfield)
#[derive(Debug, Clone, Copy, Default)]
pub struct TransferFlags(pub u16);

impl TransferFlags {
    pub const LINKED: Self = Self(0x0001);
    pub const PENDING: Self = Self(0x0002);
    pub const POST_PENDING_TRANSFER: Self = Self(0x0004);
    pub const VOID_PENDING_TRANSFER: Self = Self(0x0004);
}

/// Transfer structure (128 bytes on disk)
#[derive(Debug, Clone, Copy)]
#[repr(C, align(16))]
pub struct Transfer {
    /// Unique transfer ID
    pub id: TransferId,

    /// Source account
    pub debit_account_id: AccountId,

    /// Destination account
    pub credit_account_id: AccountId,

    /// Amount in smallest currency unit
    pub amount: u64,

    /// Ledger identifier
    pub ledger: LedgerId,

    /// Transfer type code
    pub code: u16,

    /// Transfer flags
    pub flags: TransferFlags,

    /// Nanosecond timestamp
    pub timestamp: u64,

    /// Two-phase commit timeout
    pub timeout: u64,

    /// Pending transfer reference (for 2PC)
    pub pending_id: Option<TransferId>,
}

impl Transfer {
    /// Serialize to bytes
    pub fn to_bytes(&self) -> [u8; 128] {
        let mut bytes = [0u8; 128];

        bytes[0..16].copy_from_slice(&self.id.0.to_le_bytes());
        bytes[16..32].copy_from_slice(&self.debit_account_id.0.to_le_bytes());
        bytes[32..48].copy_from_slice(&self.credit_account_id.0.to_le_bytes());
        bytes[48..56].copy_from_slice(&self.amount.to_le_bytes());
        bytes[56..60].copy_from_slice(&self.ledger.0.to_le_bytes());
        bytes[60..62].copy_from_slice(&self.code.to_le_bytes());
        bytes[62..64].copy_from_slice(&self.flags.0.to_le_bytes());
        bytes[64..72].copy_from_slice(&self.timestamp.to_le_bytes());
        bytes[72..80].copy_from_slice(&self.timeout.to_le_bytes());

        if let Some(pending_id) = self.pending_id {
            bytes[80..96].copy_from_slice(&pending_id.0.to_le_bytes());
        }

        bytes
    }

    /// Deserialize from bytes
    pub fn from_bytes(bytes: &[u8; 128]) -> Self {
        let pending_id = {
            let pending_bytes = &bytes[80..96];
            if pending_bytes.iter().any(|&b| b != 0) {
                Some(TransferId(u128::from_le_bytes(pending_bytes.try_into().unwrap())))
            } else {
                None
            }
        };

        Self {
            id: TransferId(u128::from_le_bytes(bytes[0..16].try_into().unwrap())),
            debit_account_id: AccountId(u128::from_le_bytes(bytes[16..32].try_into().unwrap())),
            credit_account_id: AccountId(u128::from_le_bytes(bytes[32..48].try_into().unwrap())),
            amount: u64::from_le_bytes(bytes[48..56].try_into().unwrap()),
            ledger: LedgerId(u32::from_le_bytes(bytes[56..60].try_into().unwrap())),
            code: u16::from_le_bytes(bytes[60..62].try_into().unwrap()),
            flags: TransferFlags(u16::from_le_bytes(bytes[62..64].try_into().unwrap())),
            timestamp: u64::from_le_bytes(bytes[64..72].try_into().unwrap()),
            timeout: u64::from_le_bytes(bytes[72..80].try_into().unwrap()),
            pending_id,
        }
    }
}
```

## Part 2: Valtron Effects System

### Effect Definitions

```rust
//! Valtron effects for TigerBeetle
//!
//! All I/O is expressed as algebraic effects

use valtron::{Effect, Handler, TaskResult};

/// Storage effects (disk I/O)
#[derive(Debug, Clone)]
pub enum StorageEffect {
    /// Read data from file at offset
    Read {
        file: String,
        offset: u64,
        length: usize,
    },
    /// Write data to file at offset
    Write {
        file: String,
        offset: u64,
        data: Vec<u8>,
    },
    /// Sync file to disk (fsync)
    Sync {
        file: String,
    },
    /// Get file size
    FileSize {
        file: String,
    },
}

/// Network effects (cluster communication)
#[derive(Debug, Clone)]
pub enum NetworkEffect {
    /// Send message to peer
    Send {
        peer_id: u32,
        data: Vec<u8>,
    },
    /// Receive message from peer
    Receive {
        timeout_ms: u64,
    },
    /// Get current time
    CurrentTime,
}

/// Random effects (for IDs, nonces)
#[derive(Debug, Clone)]
pub enum RandomEffect {
    /// Generate random u128
    RandomU128,
}

/// Combined TigerBeetle effect type
#[derive(Debug, Clone)]
pub enum TigerBeetleEffect {
    Storage(StorageEffect),
    Network(NetworkEffect),
    Random(RandomEffect),
}

/// Effect handler result
pub type EffectResult<T> = TaskResult<T, TigerBeetleEffect>;
```

### Effect Handlers

```rust
//! Effect handler implementations

use std::collections::HashMap;
use std::fs::{File, OpenOptions};
use std::io::{Read, Seek, SeekFrom, Write};
use std::sync::{Arc, Mutex};

/// Storage effect handler
pub struct StorageHandler {
    /// Open file handles
    files: Arc<Mutex<HashMap<String, File>>>,
    /// Base directory for data files
    base_dir: String,
}

impl StorageHandler {
    pub fn new(base_dir: String) -> Self {
        Self {
            files: Arc::new(Mutex::new(HashMap::new())),
            base_dir,
        }
    }

    fn get_file(&self, file: &str) -> std::io::Result<File> {
        let path = format!("{}/{}", self.base_dir, file);
        OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .open(&path)
    }

    pub fn handle(&self, effect: StorageEffect) -> EffectResult<Value> {
        match effect {
            StorageEffect::Read { file, offset, length } => {
                let mut f = self.get_file(&file)?;
                f.seek(SeekFrom::Start(offset))?;
                let mut buf = vec![0u8; length];
                f.read_exact(&mut buf)?;
                EffectResult::Complete(Value::Bytes(buf))
            }

            StorageEffect::Write { file, offset, data } => {
                let mut f = self.get_file(&file)?;
                f.seek(SeekFrom::Start(offset))?;
                f.write_all(&data)?;
                EffectResult::Complete(Value::Unit)
            }

            StorageEffect::Sync { file } => {
                let mut f = self.get_file(&file)?;
                f.sync_all()?;
                EffectResult::Complete(Value::Unit)
            }

            StorageEffect::FileSize { file } => {
                let f = self.get_file(&file)?;
                let size = f.metadata()?.len();
                EffectResult::Complete(Value::U64(size))
            }
        }
    }
}

/// Network effect handler (simulated for single-node)
pub struct NetworkHandler {
    /// Simulated peer messages
    message_queue: Arc<Mutex<Vec<(u32, Vec<u8>)>>>,
}

impl NetworkHandler {
    pub fn handle(&self, effect: NetworkEffect) -> EffectResult<Value> {
        match effect {
            NetworkEffect::Send { peer_id, data } => {
                // In real implementation, send over TCP
                // For simulation, just log
                eprintln!("Sending to peer {}: {} bytes", peer_id, data.len());
                EffectResult::Complete(Value::Unit)
            }

            NetworkEffect::Receive { timeout_ms } => {
                // In real implementation, receive from TCP
                // For simulation, return None
                EffectResult::Complete(Value::Option(None))
            }

            NetworkEffect::CurrentTime => {
                let now = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_nanos() as u64;
                EffectResult::Complete(Value::U64(now))
            }
        }
    }
}
```

## Part 3: Storage Engine

### WAL Implementation

```rust
//! Write-Ahead Log implementation

use crate::effects::{StorageEffect, EffectResult};
use crate::types::{Account, Transfer, AccountId, TransferId};

/// WAL entry types
#[derive(Debug, Clone, Copy)]
#[repr(u8)]
pub enum WALEntryType {
    AccountCreate = 0x01,
    AccountUpdate = 0x02,
    TransferCreate = 0x03,
    TransferCommit = 0x04,
    TransferVoid = 0x05,
    Checkpoint = 0x06,
}

/// WAL entry header (32 bytes)
#[derive(Debug, Clone)]
pub struct WALHeader {
    pub magic: u32,          // 0xDEADBEEF
    pub entry_type: WALEntryType,
    pub lsn: u64,            // Log Sequence Number
    pub timestamp: u64,
    pub checksum: u32,
}

impl WALHeader {
    pub const SIZE: usize = 32;

    pub fn to_bytes(&self) -> [u8; Self::SIZE] {
        let mut bytes = [0u8; Self::SIZE];
        bytes[0..4].copy_from_slice(&self.magic.to_le_bytes());
        bytes[4] = self.entry_type as u8;
        bytes[5..13].copy_from_slice(&self.lsn.to_le_bytes());
        bytes[13..21].copy_from_slice(&self.timestamp.to_le_bytes());
        bytes[21..25].copy_from_slice(&self.checksum.to_le_bytes());
        bytes
    }

    pub fn from_bytes(bytes: &[u8; Self::SIZE]) -> Option<Self> {
        let magic = u32::from_le_bytes(bytes[0..4].try_into().unwrap());
        if magic != 0xDEADBEEF {
            return None; // Invalid magic
        }

        Some(Self {
            magic,
            entry_type: unsafe { std::mem::transmute(bytes[4]) },
            lsn: u64::from_le_bytes(bytes[5..13].try_into().unwrap()),
            timestamp: u64::from_le_bytes(bytes[13..21].try_into().unwrap()),
            checksum: u32::from_le_bytes(bytes[21..25].try_into().unwrap()),
        })
    }
}

/// WAL entry
pub struct WALEntry {
    pub header: WALHeader,
    pub data: Vec<u8>,
}

impl WALEntry {
    pub const TOTAL_SIZE: usize = WALHeader::SIZE + 128; // Fixed 128 bytes data

    pub fn new(entry_type: WALEntryType, lsn: u64, timestamp: u64, data: Vec<u8>) -> Self {
        let header = WALHeader {
            magic: 0xDEADBEEF,
            entry_type,
            lsn,
            timestamp,
            checksum: 0, // Computed after
        };

        let mut entry = Self { header, data };
        entry.header.checksum = entry.compute_checksum();
        entry
    }

    fn compute_checksum(&self) -> u32 {
        // CRC32C checksum
        let header_bytes = self.header.to_bytes();
        crc32c::crc32c(&[&header_bytes[..21], &self.data].concat())
    }

    pub fn verify_checksum(&self) -> bool {
        self.header.checksum == self.compute_checksum()
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(Self::TOTAL_SIZE);
        bytes.extend_from_slice(&self.header.to_bytes());
        bytes.extend_from_slice(&self.data);
        bytes
    }
}

/// WAL manager
pub struct WAL {
    /// Current LSN
    lsn: u64,
    /// WAL file offset
    offset: u64,
    /// Circular buffer head
    head: u64,
}

impl WAL {
    pub fn new() -> Self {
        Self {
            lsn: 0,
            offset: 0,
            head: 0,
        }
    }

    /// Append entry to WAL
    pub fn append(
        &mut self,
        entry_type: WALEntryType,
        data: Vec<u8>,
    ) -> EffectResult<u64> {
        // Get current timestamp
        let timestamp = self.effect(StorageEffect::CurrentTime)?;

        // Create WAL entry
        let entry = WALEntry::new(entry_type, self.lsn, timestamp, data);

        // Write to WAL file
        self.effect(StorageEffect::Write {
            file: "wal.tigerbeetle".to_string(),
            offset: self.offset,
            data: entry.to_bytes(),
        })?;

        // Sync to disk
        self.effect(StorageEffect::Sync {
            file: "wal.tigerbeetle".to_string(),
        })?;

        // Update pointers
        let entry_lsn = self.lsn;
        self.lsn += 1;
        self.offset += WALEntry::TOTAL_SIZE as u64;

        EffectResult::Complete(entry_lsn)
    }

    /// Read entry from WAL
    pub fn read(&self, lsn: u64) -> EffectResult<Option<WALEntry>> {
        let offset = lsn * WALEntry::TOTAL_SIZE as u64;

        let bytes = self.effect(StorageEffect::Read {
            file: "wal.tigerbeetle".to_string(),
            offset,
            length: WALEntry::TOTAL_SIZE,
        })?;

        if bytes.is_empty() {
            return EffectResult::Complete(None);
        }

        let header_bytes: [u8; WALHeader::SIZE] = bytes[0..WALHeader::SIZE].try_into().unwrap();
        let header = match WALHeader::from_bytes(&header_bytes) {
            Some(h) => h,
            None => return EffectResult::Complete(None), // Invalid entry
        };

        let data = bytes[WALHeader::SIZE..].to_vec();
        let entry = WALEntry { header, data };

        if !entry.verify_checksum() {
            return EffectResult::Complete(None); // Corrupted entry
        }

        EffectResult::Complete(Some(entry))
    }
}
```

### Account Table

```rust
//! Account table management

use crate::types::{Account, AccountId};
use crate::wal::{WAL, WALEntryType};

/// Account table (in-memory + disk)
pub struct AccountTable {
    /// In-memory account cache
    accounts: HashMap<AccountId, Account>,
    /// WAL for persistence
    wal: WAL,
    /// Next account ID hint
    next_id: u128,
}

impl AccountTable {
    pub fn new() -> Self {
        Self {
            accounts: HashMap::new(),
            wal: WAL::new(),
            next_id: 1,
        }
    }

    /// Create account
    pub fn create_account(&mut self, mut account: Account) -> Result<(), AccountError> {
        // Check ID uniqueness
        if self.accounts.contains_key(&account.id) {
            return Err(AccountError::IDExists);
        }

        // Write to WAL
        self.wal.append(WALEntryType::AccountCreate, account.to_bytes().to_vec())?;

        // Insert into cache
        self.accounts.insert(account.id, account);

        Ok(())
    }

    /// Get account by ID
    pub fn get_account(&self, id: AccountId) -> Option<&Account> {
        self.accounts.get(&id)
    }

    /// Get mutable account by ID
    pub fn get_account_mut(&mut self, id: AccountId) -> Option<&mut Account> {
        self.accounts.get_mut(&id)
    }

    /// Update account (with WAL logging)
    pub fn update_account(&mut self, account: Account) -> Result<(), AccountError> {
        if !self.accounts.contains_key(&account.id) {
            return Err(AccountError::NotFound);
        }

        // Write to WAL
        self.wal.append(WALEntryType::AccountUpdate, account.to_bytes().to_vec())?;

        // Update cache
        self.accounts.insert(account.id, account);

        Ok(())
    }

    /// Generate next account ID
    pub fn next_account_id(&mut self) -> AccountId {
        let id = AccountId(self.next_id);
        self.next_id += 1;
        id
    }
}

#[derive(Debug)]
pub enum AccountError {
    IDExists,
    NotFound,
    WALError(String),
}
```

## Part 4: Transfer Engine

### Two-Phase Commit Implementation

```rust
//! Two-phase commit for transfers

use crate::types::{Transfer, TransferFlags, AccountId, TransferId};
use crate::account_table::{AccountTable, AccountError};
use crate::wal::{WAL, WALEntryType};

/// Pending transfer state
#[derive(Debug, Clone)]
pub enum PendingState {
    Pending,
    Posted,
    Voided,
}

/// Pending transfer tracking
pub struct PendingTransfer {
    pub transfer: Transfer,
    pub state: PendingState,
    pub timeout_at: u64,
}

/// Transfer engine
pub struct TransferEngine {
    /// Account table
    accounts: AccountTable,
    /// Pending transfers (for 2PC)
    pending: HashMap<TransferId, PendingTransfer>,
    /// WAL
    wal: WAL,
}

impl TransferEngine {
    pub fn new() -> Self {
        Self {
            accounts: AccountTable::new(),
            pending: HashMap::new(),
            wal: WAL::new(),
        }
    }

    /// Create transfer (immediate or pending)
    pub fn create_transfer(&mut self, transfer: Transfer) -> Result<(), TransferError> {
        // Handle two-phase commit operations
        if transfer.flags.0 & TransferFlags::POST_PENDING_TRANSFER.0 != 0 {
            return self.commit_pending(&transfer.pending_id.ok_or(TransferError::InvalidPendingID)?);
        }

        if transfer.flags.0 & TransferFlags::VOID_PENDING_TRANSFER.0 != 0 {
            return self.void_pending(&transfer.pending_id.ok_or(TransferError::InvalidPendingID)?);
        }

        // Normal transfer creation
        if transfer.flags.0 & TransferFlags::PENDING.0 != 0 {
            self.create_pending_transfer(transfer)
        } else {
            self.create_immediate_transfer(transfer)
        }
    }

    /// Create immediate transfer
    fn create_immediate_transfer(&mut self, mut transfer: Transfer) -> Result<(), TransferError> {
        // Validate transfer
        self.validate_transfer(&transfer)?;

        // Get accounts
        let debit_account = self.accounts
            .get_account(transfer.debit_account_id)
            .ok_or(TransferError::DebitAccountNotFound)?
            .clone();

        let credit_account = self.accounts
            .get_account(transfer.credit_account_id)
            .ok_or(TransferError::CreditAccountNotFound)?
            .clone();

        // Check balance
        if !debit_account.can_debit(transfer.amount) {
            return Err(TransferError::InsufficientFunds);
        }

        // Write to WAL
        self.wal.append(WALEntryType::TransferCreate, transfer.to_bytes().to_vec())?;

        // Apply to accounts
        let mut debit_account = debit_account;
        debit_account.debits_posted += transfer.amount;
        self.accounts.update_account(debit_account)?;

        let mut credit_account = credit_account;
        credit_account.credits_posted += transfer.amount;
        self.accounts.update_account(credit_account)?;

        Ok(())
    }

    /// Create pending transfer (two-phase commit phase 1)
    fn create_pending_transfer(&mut self, transfer: Transfer) -> Result<(), TransferError> {
        // Validate transfer
        self.validate_transfer(&transfer)?;

        // Get current time
        let now = self.get_current_time()?;

        // Check timeout
        if transfer.timeout != 0 && transfer.timeout <= now {
            return Err(TransferError::TimeoutExpired);
        }

        // Check pending balance
        let debit_account = self.accounts
            .get_account(transfer.debit_account_id)
            .ok_or(TransferError::DebitAccountNotFound)?;

        if debit_account.debits_pending + transfer.amount > debit_account.credits_posted {
            return Err(TransferError::InsufficientFunds);
        }

        // Update pending balances
        let mut debit_account = debit_account.clone();
        debit_account.debits_pending += transfer.amount;
        self.accounts.update_account(debit_account)?;

        let mut credit_account = self.accounts
            .get_account(transfer.credit_account_id)
            .ok_or(TransferError::CreditAccountNotFound)?
            .clone();
        credit_account.credits_pending += transfer.amount;
        self.accounts.update_account(credit_account)?;

        // Write to WAL
        self.wal.append(WALEntryType::TransferCreate, transfer.to_bytes().to_vec())?;

        // Track pending transfer
        self.pending.insert(transfer.id, PendingTransfer {
            transfer: transfer.clone(),
            state: PendingState::Pending,
            timeout_at: transfer.timeout,
        });

        Ok(())
    }

    /// Commit pending transfer (two-phase commit phase 2a)
    fn commit_pending(&mut self, pending_id: &TransferId) -> Result<(), TransferError> {
        let pending = self.pending
            .get_mut(pending_id)
            .ok_or(TransferError::PendingIDNotFound)?;

        if pending.state != PendingState::Pending {
            return Err(TransferError::PendingAlreadyCommitted);
        }

        // Check timeout
        let now = self.get_current_time()?;
        if now > pending.timeout_at {
            return Err(TransferError::TimeoutExpired);
        }

        let transfer = &pending.transfer;

        // Convert pending to posted
        let mut debit_account = self.accounts
            .get_account(transfer.debit_account_id)
            .ok_or(TransferError::DebitAccountNotFound)?
            .clone();
        debit_account.debits_pending -= transfer.amount;
        debit_account.debits_posted += transfer.amount;
        self.accounts.update_account(debit_account)?;

        let mut credit_account = self.accounts
            .get_account(transfer.credit_account_id)
            .ok_or(TransferError::CreditAccountNotFound)?
            .clone();
        credit_account.credits_pending -= transfer.amount;
        credit_account.credits_posted += transfer.amount;
        self.accounts.update_account(credit_account)?;

        // Update state
        pending.state = PendingState::Posted;

        // Write commit to WAL
        self.wal.append(WALEntryType::TransferCommit, transfer.to_bytes().to_vec())?;

        Ok(())
    }

    /// Void pending transfer (two-phase commit phase 2b)
    fn void_pending(&mut self, pending_id: &TransferId) -> Result<(), TransferError> {
        let pending = self.pending
            .get_mut(pending_id)
            .ok_or(TransferError::PendingIDNotFound)?;

        if pending.state != PendingState::Pending {
            return Err(TransferError::PendingAlreadyCommitted);
        }

        let transfer = &pending.transfer;

        // Release pending funds
        let mut debit_account = self.accounts
            .get_account(transfer.debit_account_id)
            .ok_or(TransferError::DebitAccountNotFound)?
            .clone();
        debit_account.debits_pending -= transfer.amount;
        self.accounts.update_account(debit_account)?;

        let mut credit_account = self.accounts
            .get_account(transfer.credit_account_id)
            .ok_or(TransferError::CreditAccountNotFound)?
            .clone();
        credit_account.credits_pending -= transfer.amount;
        self.accounts.update_account(credit_account)?;

        // Update state
        pending.state = PendingState::Voided;

        // Write void to WAL
        self.wal.append(WALEntryType::TransferVoid, transfer.to_bytes().to_vec())?;

        Ok(())
    }

    /// Validate transfer
    fn validate_transfer(&self, transfer: &Transfer) -> Result<(), TransferError> {
        // Check accounts exist
        if self.accounts.get_account(transfer.debit_account_id).is_none() {
            return Err(TransferError::DebitAccountNotFound);
        }
        if self.accounts.get_account(transfer.credit_account_id).is_none() {
            return Err(TransferError::CreditAccountNotFound);
        }

        // Check same ledger
        let debit_ledger = self.accounts.get_account(transfer.debit_account_id).unwrap().ledger;
        let credit_ledger = self.accounts.get_account(transfer.credit_account_id).unwrap().ledger;
        if debit_ledger != credit_ledger {
            return Err(TransferError::LedgerMismatch);
        }

        // Check ledger matches transfer
        if transfer.ledger != debit_ledger {
            return Err(TransferError::LedgerMismatch);
        }

        Ok(())
    }

    fn get_current_time(&self) -> Result<u64, TransferError> {
        // In real implementation, use effect system
        Ok(std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos() as u64)
    }
}

#[derive(Debug)]
pub enum TransferError {
    DebitAccountNotFound,
    CreditAccountNotFound,
    LedgerMismatch,
    InsufficientFunds,
    TimeoutExpired,
    PendingIDNotFound,
    PendingAlreadyCommitted,
    InvalidPendingID,
    AccountError(AccountError),
    WALError(String),
}
```

---

*This document is part of the TigerBeetle exploration series. See [exploration.md](./exploration.md) for the complete index.*
