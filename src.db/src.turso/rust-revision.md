---
title: "Rust Revision: Turso/libSQL"
subtitle: "Translating libSQL embedded replica patterns to Rust"
location: /home/darkvoid/Boxxed/@dev/repo-expolorations/src.db/src.turso
explored_at: 2026-03-28
source: /home/darkvoid/Boxxed/@formulas/src.rust/src.db/src.turso
---

# Rust Revision: Turso/libSQL

## Overview

This document provides a Rust translation guide for libSQL's embedded replica architecture. The original libsql is written in Rust with C bindings to SQLite. We'll focus on the Rust components and how to replicate the patterns using Valtron (no async/tokio).

## Architecture Translation

### Original libSQL Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                    libsql-client-rs                          │
├─────────────────────────────────────────────────────────────┤
│  ┌─────────────────────────────────────────────────────┐    │
│  │  Embedded Replica (libsql-core)                     │    │
│  │  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐  │    │
│  │  │   SQLite    │  │    WAL      │  │   Sync      │  │    │
│  │  │   (C API)   │  │   Parser    │  │   Engine    │  │    │
│  │  │   via FFI   │  │             │  │             │  │    │
│  │  └─────────────┘  └─────────────┘  └─────────────┘  │    │
│  └─────────────────────────────────────────────────────┘    │
│                                                              │
│  ┌─────────────────────────────────────────────────────┐    │
│  │  HTTP Client (reqwest + tokio)                      │    │
│  └─────────────────────────────────────────────────────┘    │
└─────────────────────────────────────────────────────────────┘
```

### Valtron-Based Architecture (No Async)

```
┌─────────────────────────────────────────────────────────────┐
│                   foundation_core                            │
├─────────────────────────────────────────────────────────────┤
│  ┌─────────────────────────────────────────────────────┐    │
│  │  Embedded Replica (pure Rust)                       │    │
│  │  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐  │    │
│  │  │   SQLite    │  │    WAL      │  │   Sync      │  │    │
│  │  │   Parser    │  │   Engine    │  │  TaskIter   │  │    │
│  │  │  (rusqlite) │  │             │  │             │  │    │
│  │  └─────────────┘  └─────────────┘  └─────────────┘  │    │
│  └─────────────────────────────────────────────────────┘    │
│                                                              │
│  ┌─────────────────────────────────────────────────────┐    │
│  │  HTTP Client (Valtron algebraic effect)             │    │
│  └─────────────────────────────────────────────────────┘    │
└─────────────────────────────────────────────────────────────┘
```

## Core Types

### Database Connection

```rust
// Original libsql (simplified)
pub struct Database {
    conn: Arc<libsql_core::Connection>,
    sync_handle: Option<SyncHandle>,
}

impl Database {
    pub async fn open(path: &str, url: &str, auth: &str) -> Result<Self> {
        // Async initialization
    }

    pub async fn execute(&self, sql: &str, params: &[Value]) -> Result<ResultSet> {
        // Async execution
    }
}

// Valtron translation (no async)
pub struct Database {
    conn: rusqlite::Connection,
    wal_state: WalState,
    sync_config: SyncConfig,
}

pub struct SyncConfig {
    primary_url: String,
    auth_token: String,
    replica_id: String,
    sync_interval_secs: u64,
}

pub struct WalState {
    current_frame_offset: u64,
    last_sync_time: u64,
    is_fresh: bool,
}

impl Database {
    /// Open database (synchronous)
    pub fn open(path: &str) -> Result<Self, DbError> {
        let conn = rusqlite::Connection::open(path)?;

        // Enable WAL mode
        conn.execute("PRAGMA journal_mode=WAL", [])?;

        Ok(Self {
            conn,
            wal_state: WalState {
                current_frame_offset: 0,
                last_sync_time: 0,
                is_fresh: false,
            },
            sync_config: SyncConfig {
                primary_url: String::new(),
                auth_token: String::new(),
                replica_id: uuid::generate(),
                sync_interval_secs: 60,
            },
        })
    }

    /// Configure sync (call before first use)
    pub fn configure_sync(&mut self, url: String, auth: String) {
        self.sync_config.primary_url = url;
        self.sync_config.auth_token = auth;
    }

    /// Execute query (synchronous)
    pub fn execute(&self, sql: &str, params: &[Value]) -> Result<ResultSet, DbError> {
        // Check if we need to sync first
        if !self.wal_state.is_fresh {
            // Return error or sync synchronously
            return Err(DbError::NeedsSync);
        }

        // Execute using rusqlite
        let mut stmt = self.conn.prepare(sql)?;

        // Bind parameters
        for (i, param) in params.iter().enumerate() {
            match param {
                Value::Integer(v) => stmt.bind(i + 1, v)?,
                Value::Text(v) => stmt.bind(i + 1, v.as_str())?,
                Value::Real(v) => stmt.bind(i + 1, v)?,
                Value::Blob(v) => stmt.bind(i + 1, v.as_slice())?,
                Value::Null => stmt.bind(i + 1, ())?,
            }
        }

        // Execute and collect results
        let columns: Vec<String> = stmt
            .column_names()
            .iter()
            .map(|s| s.to_string())
            .collect();

        let rows: Vec<Vec<Value>> = stmt
            .query_map([], |row| {
                let mut values = Vec::new();
                for i in 0..row.as_ref().unwrap().column_count() {
                    let value: Value = row.get(i)?;
                    values.push(value);
                }
                Ok(values)
            })?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(ResultSet { columns, rows })
    }
}
```

### WAL Frame Types

```rust
/// WAL Frame header (matches SQLite format)
#[derive(Debug, Clone)]
pub struct WalFrameHeader {
    pub page_number: u32,
    pub db_size_pages: u32,  // 0 if not commit frame
    pub salt1: u32,
    pub salt2: u32,
    pub checksum1: u32,
    pub checksum2: u32,
}

impl WalFrameHeader {
    pub const SIZE: usize = 24;

    pub fn from_bytes(data: &[u8]) -> Result<Self, ParseError> {
        if data.len() < Self::SIZE {
            return Err(ParseError::InsufficientData);
        }

        Ok(Self {
            page_number: u32::from_be_bytes([data[0], data[1], data[2], data[3]]),
            db_size_pages: u32::from_be_bytes([data[4], data[5], data[6], data[7]]),
            salt1: u32::from_be_bytes([data[8], data[9], data[10], data[11]]),
            salt2: u32::from_be_bytes([data[12], data[13], data[14], data[15]]),
            checksum1: u32::from_be_bytes([data[16], data[17], data[18], data[19]]),
            checksum2: u32::from_be_bytes([data[20], data[21], data[22], data[23]]),
        })
    }

    pub fn to_bytes(&self) -> [u8; Self::SIZE] {
        let mut buf = [0u8; Self::SIZE];
        buf[0..4].copy_from_slice(&self.page_number.to_be_bytes());
        buf[4..8].copy_from_slice(&self.db_size_pages.to_be_bytes());
        buf[8..12].copy_from_slice(&self.salt1.to_be_bytes());
        buf[12..16].copy_from_slice(&self.salt2.to_be_bytes());
        buf[16..20].copy_from_slice(&self.checksum1.to_be_bytes());
        buf[20..24].copy_from_slice(&self.checksum2.to_be_bytes());
        buf
    }

    pub fn is_commit(&self) -> bool {
        self.db_size_pages > 0
    }
}

/// Complete WAL frame (header + page data)
#[derive(Debug, Clone)]
pub struct WalFrame {
    pub header: WalFrameHeader,
    pub page_data: Vec<u8>,
}

impl WalFrame {
    pub fn size_on_disk(&self) -> usize {
        WalFrameHeader::SIZE + self.page_data.len()
    }

    pub fn from_bytes(data: &[u8], page_size: usize) -> Result<Self, ParseError> {
        let header = WalFrameHeader::from_bytes(data)?;

        let data_start = WalFrameHeader::SIZE;
        let data_end = data_start + page_size;

        if data.len() < data_end {
            return Err(ParseError::InsufficientData);
        }

        Ok(Self {
            header,
            page_data: data[data_start..data_end].to_vec(),
        })
    }
}
```

## Sync Engine

### Sync Request/Response

```rust
use serde::{Deserialize, Serialize};

/// Sync request sent to primary
#[derive(Serialize)]
pub struct SyncRequest {
    pub frame_offset: u64,
    pub replica_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub page_numbers: Option<Vec<u32>>,
}

/// Sync response from primary
#[derive(Deserialize, Debug)]
pub struct SyncResponse {
    pub current_frame_offset: u64,
    pub frames: Vec<RemoteWalFrame>,
    pub database_size_pages: u32,
    pub checkpoint_seq: u64,
}

/// WAL frame as received from remote primary
#[derive(Deserialize, Debug, Clone)]
pub struct RemoteWalFrame {
    pub page_number: u32,
    #[serde(with = "base64")]
    pub page_data: Vec<u8>,
    pub db_size_after_commit: u32,
    pub salt1: u64,
    pub salt2: u64,
}

impl RemoteWalFrame {
    pub fn to_local_frame(&self) -> WalFrame {
        WalFrame {
            header: WalFrameHeader {
                page_number: self.page_number,
                db_size_pages: self.db_size_after_commit,
                salt1: (self.salt1 & 0xFFFFFFFF) as u32,
                salt2: (self.salt2 & 0xFFFFFFFF) as u32,
                checksum1: 0,  // Computed locally
                checksum2: 0,
            },
            page_data: self.page_data.clone(),
        }
    }
}
```

### Sync State Machine (Valtron Style)

```rust
/// Sync state for TaskIterator pattern
#[derive(Debug, Clone)]
pub enum SyncState {
    /// Initial state
    Init,

    /// Waiting to send request
    SendRequest {
        current_offset: u64,
    },

    /// Waiting for response
    WaitForResponse {
        current_offset: u64,
    },

    /// Processing response
    ProcessFrames {
        frames: Vec<RemoteWalFrame>,
        target_offset: u64,
        current_frame: usize,
    },

    /// Checkpointing
    Checkpoint,

    /// Complete
    Synced {
        new_offset: u64,
    },

    /// Error
    Error {
        error: SyncError,
        retry_after_secs: u64,
    },
}

/// Sync task iterator (no async!)
pub struct SyncTask {
    pub state: SyncState,
    pub config: SyncConfig,
    pub wal_file: WalFile,
    pub http_buffer: Vec<u8>,
}

impl SyncTask {
    pub fn new(config: SyncConfig, wal_file: WalFile) -> Self {
        Self {
            state: SyncState::Init,
            config,
            wal_file,
            http_buffer: Vec::new(),
        }
    }

    /// Advance the task (called by Valtron executor)
    pub fn next(&mut self) -> TaskAction<SyncOutput, SyncEffect> {
        match &self.state {
            SyncState::Init => {
                let offset = self.wal_file.current_offset();
                self.state = SyncState::SendRequest { current_offset: offset };
                TaskAction::Continue
            }

            SyncState::SendRequest { current_offset } => {
                // Request HTTP effect
                let request = SyncRequest {
                    frame_offset: *current_offset,
                    replica_id: self.config.replica_id.clone(),
                    page_numbers: None,
                };

                let body = serde_json::to_string(&request).unwrap();

                TaskAction::Effect(SyncEffect::HttpRequest {
                    url: format!("{}/v1/sync", self.config.primary_url),
                    method: "POST".to_string(),
                    headers: vec![
                        ("Authorization".to_string(), format!("Bearer {}", self.config.auth_token)),
                        ("Content-Type".to_string(), "application/json".to_string()),
                    ],
                    body,
                })
            }

            SyncState::WaitForResponse { current_offset } => {
                // Transition to processing (HTTP response handled externally)
                self.state = SyncState::ProcessFrames {
                    frames: Vec::new(),  // Populated from response
                    target_offset: *current_offset,
                    current_frame: 0,
                };
                TaskAction::Continue
            }

            SyncState::ProcessFrames { frames, target_offset, current_frame } => {
                if *current_frame >= frames.len() {
                    // All frames processed
                    self.state = SyncState::Checkpoint;
                    return TaskAction::Continue;
                }

                // Apply next frame
                let frame = &frames[*current_frame];
                let local_frame = frame.to_local_frame();

                // Append to WAL file
                self.wal_file.append_frame(&local_frame);

                *current_frame += 1;
                TaskAction::Continue
            }

            SyncState::Checkpoint => {
                // Checkpoint WAL to database
                self.wal_file.checkpoint()?;

                let new_offset = self.wal_file.current_offset();
                self.state = SyncState::Synced { new_offset };
                TaskAction::Complete(SyncOutput {
                    frames_applied: 0,  // Calculate
                    new_offset,
                })
            }

            SyncState::Synced { .. } => {
                TaskAction::Complete(SyncOutput {
                    frames_applied: 0,
                    new_offset: self.wal_file.current_offset(),
                })
            }

            SyncState::Error { retry_after_secs, .. } => {
                TaskAction::Sleep(*retry_after_secs)
            }
        }
    }

    /// Set response data (called after HTTP effect completes)
    pub fn set_response(&mut self, response: SyncResponse) {
        self.state = SyncState::ProcessFrames {
            frames: response.frames,
            target_offset: response.current_frame_offset,
            current_frame: 0,
        };
    }

    /// Set HTTP error
    pub fn set_error(&mut self, error: SyncError) {
        self.state = SyncState::Error {
            error,
            retry_after_secs: 60,
        };
    }
}

/// Task action for Valtron executor
pub enum TaskAction<O, E> {
    Continue,
    Effect(E),
    Sleep(u64),
    Complete(O),
}

/// Output from sync task
pub struct SyncOutput {
    pub frames_applied: usize,
    pub new_offset: u64,
}

/// Effects that sync task can request
pub enum SyncEffect {
    HttpRequest {
        url: String,
        method: String,
        headers: Vec<(String, String)>,
        body: String,
    },
}
```

## WAL File Implementation

```rust
use std::fs::{File, OpenOptions};
use std::io::{Read, Write, Seek, SeekFrom};
use std::path::Path;

/// WAL file on disk
pub struct WalFile {
    file: File,
    page_size: u32,
    current_offset: u64,
}

impl WalFile {
    pub fn open(path: &Path, page_size: u32) -> Result<Self, IoError> {
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .open(path)?;

        let current_offset = file.metadata()?.len();

        Ok(Self {
            file,
            page_size,
            current_offset,
        })
    }

    pub fn current_offset(&self) -> u64 {
        self.current_offset
    }

    pub fn append_frame(&mut self, frame: &WalFrame) -> Result<(), IoError> {
        // Write header
        self.file.seek(SeekFrom::Start(self.current_offset))?;
        self.file.write_all(&frame.header.to_bytes())?;

        // Write page data
        self.file.write_all(&frame.page_data)?;

        self.current_offset += frame.size_on_disk() as u64;

        Ok(())
    }

    pub fn read_frame(&mut self, offset: u64) -> Result<WalFrame, IoError> {
        self.file.seek(SeekFrom::Start(offset))?;

        let mut header_buf = [0u8; WalFrameHeader::SIZE];
        self.file.read_exact(&mut header_buf)?;

        let header = WalFrameHeader::from_bytes(&header_buf)?;

        let mut page_data = vec![0u8; self.page_size as usize];
        self.file.read_exact(&mut page_data)?;

        Ok(WalFrame { header, page_data })
    }

    pub fn checkpoint(&mut self) -> Result<(), IoError> {
        // Read all frames and apply to database file
        // This is simplified - real implementation needs more care

        let db_path = self.file.path()
            .with_extension("");  // Remove -wal extension

        let mut db_file = OpenOptions::new()
            .read(true)
            .write(true)
            .open(&db_path)?;

        // Reset to beginning
        self.file.seek(SeekFrom::Start(0))?;

        // Read and apply each frame
        loop {
            let mut header_buf = [0u8; WalFrameHeader::SIZE];
            if self.file.read_exact(&mut header_buf).is_err() {
                break;  // EOF
            }

            let header = match WalFrameHeader::from_bytes(&header_buf) {
                Ok(h) => h,
                Err(_) => break,
            };

            // Read page data
            let mut page_data = vec![0u8; self.page_size as usize];
            self.file.read_exact(&mut page_data)?;

            // Write to database file at correct offset
            let db_offset = (header.page_number as u64 - 1) * self.page_size as u64;
            db_file.seek(SeekFrom::Start(db_offset))?;
            db_file.write_all(&page_data)?;
        }

        db_file.sync_all()?;

        // Truncate WAL
        self.file.set_len(0)?;
        self.current_offset = 0;

        Ok(())
    }
}
```

## Error Handling

```rust
#[derive(Debug)]
pub enum DbError {
    /// Database needs sync before queries
    NeedsSync,

    /// SQLite error
    Sqlite(rusqlite::Error),

    /// WAL parse error
    WalParse(ParseError),

    /// Sync error
    Sync(SyncError),

    /// I/O error
    Io(std::io::Error),

    /// JSON serialization error
    Json(serde_json::Error),
}

impl From<rusqlite::Error> for DbError {
    fn from(err: rusqlite::Error) -> Self {
        DbError::Sqlite(err)
    }
}

impl From<std::io::Error> for DbError {
    fn from(err: std::io::Error) -> Self {
        DbError::Io(err)
    }
}

impl From<serde_json::Error> for DbError {
    fn from(err: serde_json::Error) -> Self {
        DbError::Json(err)
    }
}

#[derive(Debug)]
pub enum ParseError {
    InsufficientData,
    InvalidMagic,
    InvalidChecksum,
    InvalidPageSize,
}

#[derive(Debug)]
pub enum SyncError {
    HttpError(u16),  // Status code
    InvalidFrame { page_number: u32 },
    CheckpointFailed,
    Timeout,
    ConnectionLost,
}
```

## Usage Example

```rust
fn main() -> Result<(), DbError> {
    // Open local database
    let mut db = Database::open("local.db")?;

    // Configure sync
    db.configure_sync(
        "https://your-db.turso.io".to_string(),
        "your-auth-token".to_string(),
    );

    // Create sync task
    let wal_file = WalFile::open(
        Path::new("local.db-wal"),
        4096,
    )?;

    let sync_config = SyncConfig {
        primary_url: "https://your-db.turso.io".to_string(),
        auth_token: "your-auth-token".to_string(),
        replica_id: uuid::generate(),
        sync_interval_secs: 60,
    };

    let mut sync_task = SyncTask::new(sync_config, wal_file);

    // Run sync (in Valtron executor)
    // This is pseudocode - actual Valtron integration varies
    loop {
        match sync_task.next() {
            TaskAction::Continue => {}
            TaskAction::Effect(SyncEffect::HttpRequest { url, method, headers, body }) => {
                // Execute HTTP request (outside Valtron)
                let response = execute_http_request(&url, &method, &headers, &body);
                sync_task.set_response(response);
            }
            TaskAction::Sleep(secs) => {
                std::thread::sleep(std::time::Duration::from_secs(secs));
            }
            TaskAction::Complete(output) => {
                println!("Sync complete: {} frames applied", output.frames_applied);
                break;
            }
        }
    }

    // Now database is synced, can query
    let results = db.execute("SELECT * FROM users", &[])?;

    for row in results.rows {
        println!("{:?}", row);
    }

    Ok(())
}
```

## Dependencies

```toml
[dependencies]
# SQLite bindings
rusqlite = { version = "0.31", features = ["bundled"] }

# Serialization
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
serde_bytes = "0.11"
base64 = "0.21"

# UUID for replica IDs
uuid = { version = "1.0", features = ["v4"] }

# For Valtron integration (hypothetical)
# valtron = { path = "../../ewe_platform/backends/foundation_core/src/valtron" }
```

---

*This document is part of the Turso/libSQL exploration series. See [exploration.md](./exploration.md) for the complete index.*
