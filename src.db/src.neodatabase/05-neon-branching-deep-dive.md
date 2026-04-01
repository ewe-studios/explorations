---
title: "Neon Database Instant Branching Deep Dive"
subtitle: "Copy-on-write storage, time travel, and Git-like database branching architecture"
location: /home/darkvoid/Boxxed/@dev/repo-expolorations/src.db/src.neodatabase
related: 03-cloning-copy-on-write-deep-dive.md
---

# Neon Database Instant Branching Deep Dive

## Overview

This document provides an in-depth analysis of Neon Database's Git-like branching architecture - how instant database branches are created using copy-on-write storage, separation of compute and storage, and how to implement similar capabilities in Rust.

## Part 1: Neon Architecture Overview

### Separation of Compute and Storage

```
Neon Architecture:

┌─────────────────────────────────────────────────────────┐
│                    Neon Cloud Platform                  │
├─────────────────────────────────────────────────────────┤
│                                                         │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐  │
│  │  Compute 1   │  │  Compute 2   │  │  Compute 3   │  │
│  │  (PostgreSQL)│  │  (PostgreSQL)│  │  (PostgreSQL)│  │
│  │              │  │              │  │              │  │
│  │ - Query proc │  │ - Query proc │  │ - Branch:    │  │
│  │ - Buffer mgr │  │ - Buffer mgr │  │   feature-x  │  │
│  │ - Connection │  │ - Connection │  │              │  │
│  └──────┬───────┘  └──────┬───────┘  └──────┬───────┘  │
│         │                 │                 │          │
│         └─────────────────┼─────────────────┘          │
│                           │                             │
│              ┌────────────▼────────────┐               │
│              │   Pageserver            │               │
│              │   (Storage Layer)       │               │
│              │                         │               │
│              │ - Receives WAL          │               │
│              │ - Stores page images    │               │
│              │ - Manages snapshots     │               │
│              │ - Time travel queries   │               │
│              └────────────┬────────────┘               │
│                           │                             │
│              ┌────────────▼────────────┐               │
│              │   S3 / Blob Storage     │               │
│              │                         │               │
│              │ - Immutable WAL files   │               │
│              │ - Page snapshots        │               │
│              │ - Historical branches   │               │
│              └─────────────────────────┘               │
│                                                         │
└───────────────────────────────────────────────────────────┘

Key Innovation:
- Compute is stateless (can be created/destroyed instantly)
- Storage is shared and immutable
- Branches are pointers to storage snapshots
```

### The Pageserver Component

```
Pageserver Architecture:

┌─────────────────────────────────────────────────────────┐
│ Pageserver (Storage Server)                             │
├─────────────────────────────────────────────────────────┤
│                                                         │
│  ┌───────────────────────────────────────────────────┐ │
│  │ Tenant Manager                                     │ │
│  │ - Manages multiple databases per tenant           │ │
│  │ - Each tenant has unique tenant_id                │ │
│  │ - Isolation between tenants                       │ │
│  └───────────────────────────────────────────────────┘ │
│                         │                               │
│  ┌───────────────────────────────────────────────────┐ │
│  │ Timeline Manager (Branch Manager)                 │ │
│  │ - Each branch is a "timeline"                     │ │
│  │ - Parent-child relationships between timelines    │ │
│  │ - LSN (Log Sequence Number) tracking              │ │
│  │ - Garbage collection policies                     │ │
│  └───────────────────────────────────────────────────┘ │
│                         │                               │
│  ┌───────────────────────────────────────────────────┐ │
│  │ WAL Receiver                                      │ │
│  │ - Receives WAL from compute nodes                 │ │
│  │ - Streams WAL to S3                               │ │
│  │ - Maintains WAL continuity                        │ │
│  └───────────────────────────────────────────────────┘ │
│                         │                               │
│  ┌───────────────────────────────────────────────────┐ │
│  │ Page Server                                       │ │
│  │ - Reconstructs pages from WAL                     │ │
│  │ - Serves page requests to compute                 │ │
│  │ - Caches frequently accessed pages                │ │
│  └───────────────────────────────────────────────────┘ │
│                         │                               │
│  ┌───────────────────────────────────────────────────┐ │
│  │ Storage Layer                                     │ │
│  │ - S3 client for blob storage                      │ │
│  │ - Local cache (disk/memory)                       │ │
│  │ - Compression (zstd)                              │ │
│  └───────────────────────────────────────────────────┘ │
└───────────────────────────────────────────────────────────┘
```

## Part 2: Copy-on-Write Storage Internals

### Page-Level Copy-on-Write

```
PostgreSQL Page Structure (8KB blocks):

┌─────────────────────────────────────────────────────────┐
│ PostgreSQL Heap Page (8192 bytes)                       │
├─────────────────────────────────────────────────────────┤
│ PageHeaderData (24 bytes)                               │
│   - pd_lsn: LSN when page was last modified             │
│   - pd_checksum: checksum for corruption detection      │
│   - pd_flags: page flags                                │
│   - pd_lower: offset to start of free space             │
│   - pd_upper: offset to end of free space               │
│   - pd_special: offset to special space (indexes)       │
│   - pd_pagesize: page size (8192)                       │
├─────────────────────────────────────────────────────────┤
│ Item Pointers (downward growing)                        │
│   - Pointers to actual data items                       │
│   - Fixed size (8 bytes each)                           │
├─────────────────────────────────────────────────────────┤
│ Free Space (middle)                                     │
│   - Grows/shrinks as data is added/removed              │
├─────────────────────────────────────────────────────────┤
│ Heap Tuples (upward growing)                            │
│   - Actual row data                                     │
│   - Variable length                                     │
│   - Includes tuple header + data                        │
└───────────────────────────────────────────────────────────┘
```

```
Neon's Copy-on-Write Page Storage:

Traditional PostgreSQL (In-Place Updates):
┌─────────────────────────────────────────────────────────┐
│ Before Update:                After Update:             │
│ ┌─────────────┐              ┌─────────────┐           │
│ │ Page 100    │              │ Page 100    │           │
│ │ Version 1   │  ──UPDATE──> │ Version 2   │           │
│ │ (overwritten)│             │ (overwritten)│          │
│ └─────────────┘              └─────────────┘           │
│                                                         │
│ Problem: Cannot access old version after update         │
└───────────────────────────────────────────────────────────┘

Neon Copy-on-Write:
┌─────────────────────────────────────────────────────────┐
│ Before Update:                After Update:             │
│ ┌─────────────┐              ┌─────────────┐           │
│ │ Page 100    │              │ Page 100    │           │
│ │ @ LSN 1000  │              │ @ LSN 1000  │           │
│ │ (kept!)     │              │ (unchanged) │           │
│ └─────────────┘              └──────┬──────┘           │
│                                    │                   │
│                              ┌─────▼──────┐           │
│                              │ Page 100   │           │
│                              │ @ LSN 2000 │           │
│                              │ (new copy) │           │
│                              │ (updated)  │           │
│                              └────────────┘           │
│                                                         │
│ Result: Both versions accessible via LSN                │
└───────────────────────────────────────────────────────────┘
```

### LSN (Log Sequence Number) Tracking

```
LSN Structure:

┌─────────────────────────────────────────────────────────┐
│ LSN Format: 64-bit integer                              │
│                                                         │
│ ┌──────────────┬───────────────┐                        │
│ │ High 32 bits │ Low 32 bits   │                        │
│ │ (log file)   │ (byte offset) │                        │
│ └──────────────┴───────────────┘                        │
│                                                         │
│ Example: 0x00000001/00000000                           │
│ - Log file 1, byte offset 0                             │
│                                                         │
│ LSN increases monotonically                             │
│ - Each WAL record gets unique LSN                       │
│ - Used for point-in-time recovery                       │
│ - Used for branch creation (LSN as timestamp)           │
└───────────────────────────────────────────────────────────┘
```

```
LSN-based Page Versioning:

┌─────────────────────────────────────────────────────────┐
│ Page Version History                                    │
│                                                         │
│ Page ID: 100                                            │
│                                                         │
│ LSN    │ Version │ Data                  │ Branch      │
│ ───────┼─────────┼───────────────────────┼────────────  │
│ 1000   │ V1      │ {name: "Alice"}       │ main @1000  │
│ 2000   │ V2      │ {name: "Alice",       │ main @2000  │
│        │         │  age: 30}             │             │
│ 3000   │ V3      │ {name: "Alice",       │ main @3000  │
│        │         │  age: 31}             │             │
│                                                         │
│ Branch created at LSN 2000:                             │
│ branch-feature sees V2 for all pages                    │
│ main continues to V3                                    │
│                                                         │
│ Query at LSN X:                                         │
│ - Find page version with LSN <= X                       │
│ - Reconstruct page from WAL if needed                   │
└───────────────────────────────────────────────────────────┘
```

## Part 3: Timeline (Branch) Management

### Timeline Hierarchy

```
Timeline Tree Structure:

┌─────────────────────────────────────────────────────────┐
│ Timeline (Branch) Hierarchy                             │
│                                                         │
│ main (_timeline: 00000000000000000000000000000000)     │
│ │                                                        │
│ ├─ feature-auth (created at LSN 2000)                  │
│ │  └─ feature-auth-v2 (created at LSN 5000)            │
│ │                                                        │
│ ├─ feature-search (created at LSN 3000)                │
│ │                                                        │
│ └─ hotfix-security (created at LSN 4000)               │
│                                                         │
│ Each timeline has:                                      │
│ - timeline_id: UUID                                     │
│ - parent_timeline_id: UUID (or 0 for main)             │
│ - ancestor_lsn: LSN where branch was created            │
│ - latest_lsn: Most recent LSN in timeline               │
└───────────────────────────────────────────────────────────┘

Timeline Metadata (stored in pageserver):

```rust
#[derive(Debug, Clone)]
struct TimelineInfo {
    timeline_id: Uuid,
    parent_timeline_id: Option<Uuid>,
    ancestor_lsn: Lsn,
    latest_lsn: Lsn,
    creation_timestamp: SystemTime,
    pg_version: u32,
    state: TimelineState,
}

#[derive(Debug, Clone, PartialEq)]
enum TimelineState {
    Active,
    Standby,
    Stopping,
}
```
```

### Branch Creation Algorithm

```
Instant Branch Creation Process:

┌─────────────────────────────────────────────────────────┐
│ Step 1: Client Request                                  │
│                                                         │
│ POST /v1/project/{project_id}/branches                 │
│ {                                                       │
│   "name": "feature-auth",                               │
│   "parent_id": "main",                                  │
│   "parent_lsn": "0/1F4A8B0"  // Optional: exact point  │
│ }                                                       │
│                                                         │
│ Response (instant!):                                   │
│ {                                                       │
│   "id": "br-feature-auth-xyz",                          │
│   "name": "feature-auth",                               │
│   "primary_host": "feature-auth.proj.neon.tech",       │
│   "created_at": "2024-01-15T10:30:00Z"                 │
│ }                                                       │
└───────────────────────────────────────────────────────────┘
                         │
                         ▼
┌─────────────────────────────────────────────────────────┐
│ Step 2: Pageserver - Create Timeline Metadata          │
│                                                         │
│ - Generate new timeline_id                              │
│ - Look up parent timeline                               │
│ - Record ancestor_lsn                                   │
│ - Create empty timeline directory in S3                 │
│ - Update timeline index                                 │
│                                                         │
│ NO DATA COPIED! Only metadata created                   │
└───────────────────────────────────────────────────────────┘
                         │
                         ▼
┌─────────────────────────────────────────────────────────┐
│ Step 3: Compute Node Startup                           │
│                                                         │
│ - Request page from pageserver                         │
│ - Pageserver checks timeline chain:                    │
│   1. Check feature-auth timeline for page              │
│   2. If not found, check main timeline at ancestor_lsn │
│   3. Reconstruct page from WAL if needed               │
│   4. Return page to compute                            │
│                                                         │
│ Compute sees consistent snapshot at ancestor_lsn        │
└───────────────────────────────────────────────────────────┘
```

```
Page Retrieval Algorithm (with inheritance):

```rust
async fn get_page_at_lsn(
    &self,
    timeline_id: TimelineId,
    page_id: BlockNumber,
    lsn: Lsn,
) -> Result<Page> {
    let timeline = self.get_timeline(timeline_id)?;

    // Walk up the timeline tree
    let mut current_timeline = timeline;
    let mut current_lsn = lsn;

    loop {
        // Try to find page in current timeline
        match self.get_page_from_timeline(
            current_timeline.id,
            page_id,
            current_lsn,
        ).await? {
            Some(page) => return Ok(page),
            None => {
                // Page not found, walk to parent
                match current_timeline.parent {
                    Some(parent) => {
                        current_timeline = parent;
                        // Use ancestor LSN for parent lookup
                        current_lsn = current_timeline.ancestor_lsn;
                    }
                    None => {
                        // Reached root without finding page
                        // Return error or default
                        return Err(PageNotFoundError);
                    }
                }
            }
        }
    }
}
```
```

## Part 4: WAL Storage and Page Reconstruction

### WAL File Structure

```
Neon WAL Storage Format:

┌─────────────────────────────────────────────────────────┐
│ WAL File Organization                                   │
│                                                         │
│ S3 Path Structure:                                      │
│ s3://{bucket}/{tenant_id}/{timeline_id}/                │
│   wal/                                                  │
│     000000010000000000000001                            │
│     000000010000000000000002                            │
│     ...                                                 │
│   images/                                               │
│     000000010000000000001000 (base image at LSN)       │
│     000000010000000000002000                            │
│     ...                                                 │
│   metadata/                                             │
│     timeline_info.json                                  │
│     gc_info.json                                        │
│                                                         │
│ WAL File Format:                                        │
│ ┌─────────────────────────────────────────────────────┐│
│ │ WAL Record Header (24 bytes)                        ││
│ │   - xl_tot_len: total record length                 ││
│ │   - xl_info: info bits                              ││
│ │   - xl_rmid: resource manager ID                    ││
│ │   - xl_xid: transaction ID                          ││
│ │   - xl_prev: previous LSN                           ││
│ │   - xl_crc: CRC32C checksum                         ││
│ └─────────────────────────────────────────────────────┘│
│ ┌─────────────────────────────────────────────────────┐│
│ │ WAL Record Body (variable)                          ││
│ │   - BLOB 1: relation extension                      ││
│ │   - BLOB 2: page initialization                     ││
│ │   - BLOB 3: tuple insertion                         ││
│ │   - ... (multiple changes per record)               ││
│ └─────────────────────────────────────────────────────┘│
└───────────────────────────────────────────────────────────┘
```

### Page Reconstruction from WAL

```
Page Reconstruction Algorithm:

┌─────────────────────────────────────────────────────────┐
│ Request: Get page at LSN 5000                          │
│                                                         │
│ Step 1: Find nearest base image                        │
│   - Search images/ for latest image <= LSN 5000       │
│   - Found: image at LSN 4000                          │
│                                                         │
│ Step 2: Collect WAL records                            │
│   - Get WAL records from LSN 4000 to LSN 5000         │
│   - Filter to records affecting this page             │
│   - Records: [4100, 4250, 4500, 4800]                 │
│                                                         │
│ Step 3: Replay WAL                                     │
│   page = load_image(LSN 4000)  // Base page           │
│   page = apply_wal(page, LSN 4100)                    │
│   page = apply_wal(page, LSN 4250)                    │
│   page = apply_wal(page, LSN 4500)                    │
│   page = apply_wal(page, LSN 4800)                    │
│                                                         │
│   Result: Reconstructed page at LSN 5000              │
│                                                         │
│ Optimization: Cache reconstructed pages                │
└───────────────────────────────────────────────────────────┘

Implementation:

```rust
async fn reconstruct_page(
    &self,
    timeline_id: TimelineId,
    page_id: BlockNumber,
    target_lsn: Lsn,
) -> Result<Page> {
    // Step 1: Find base image
    let base_image_lsn = self.find_nearest_image(timeline_id, page_id, target_lsn)?;
    let mut page = self.load_page_image(timeline_id, page_id, base_image_lsn)?;

    // Step 2: Get WAL records to replay
    let wal_records = self.get_wal_records(
        timeline_id,
        page_id,
        base_image_lsn..=target_lsn,
    ).await?;

    // Step 3: Replay each record
    for record in wal_records {
        page = self.apply_wal_record(page, &record)?;
    }

    // Verify final LSN
    assert_eq!(page.lsn(), target_lsn);

    Ok(page)
}

fn apply_wal_record(&self, mut page: Page, record: &WalRecord) -> Result<Page> {
    match record.record_type {
        WalRecordType::Insert => {
            page.insert_tuple(&record.data)?;
        }
        WalRecordType::Update => {
            page.update_tuple(&record.data)?;
        }
        WalRecordType::Delete => {
            page.delete_tuple(&record.data)?;
        }
        WalRecordType::Clear => {
            page.clear()?;
        }
    }
    page.set_lsn(record.lsn);
    Ok(page)
}
```
```

## Part 5: Rust Implementation

### Core Data Structures

```rust
use std::sync::Arc;
use uuid::Uuid;
use tokio::sync::RwLock;

/// Log Sequence Number - uniquely identifies position in WAL
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Lsn(u64);

impl Lsn {
    pub const INVALID: Lsn = Lsn(0);

    pub fn new(high: u32, low: u32) -> Self {
        Lsn(((high as u64) << 32) | (low as u64))
    }

    pub fn high(&self) -> u32 {
        (self.0 >> 32) as u32
    }

    pub fn low(&self) -> u32 {
        (self.0 & 0xFFFFFFFF) as u32
    }

    pub fn to_string_lossy(&self) -> String {
        format!("{:X}/{:X}", self.high(), self.low())
    }
}

/// Unique identifier for a timeline (branch)
pub type TimelineId = Uuid;

/// Timeline metadata - represents a branch
#[derive(Debug, Clone)]
pub struct Timeline {
    pub timeline_id: TimelineId,
    pub parent_timeline_id: Option<TimelineId>,
    pub ancestor_lsn: Lsn,
    pub latest_lsn: Lsn,
    pub state: TimelineState,
    pub created_at: std::time::SystemTime,
}

#[derive(Debug, Clone, PartialEq)]
pub enum TimelineState {
    Active,
    Standby,
    Stopping,
}

/// Manages all timelines for a tenant
pub struct TimelineManager {
    timelines: RwLock<std::collections::HashMap<TimelineId, Arc<Timeline>>>,
    storage: Arc<dyn PageStorage>,
}

impl TimelineManager {
    pub fn new(storage: Arc<dyn PageStorage>) -> Self {
        Self {
            timelines: RwLock::new(std::collections::HashMap::new()),
            storage,
        }
    }

    /// Create a new branch (instant, no data copy)
    pub async fn create_branch(
        &self,
        parent_id: TimelineId,
        branch_name: &str,
        branch_lsn: Option<Lsn>,
    ) -> Result<Arc<Timeline>, BranchError> {
        let parent = self
            .get_timeline(parent_id)
            .await
            .ok_or(BranchError::ParentNotFound)?;

        // Determine branch point LSN
        let ancestor_lsn = branch_lsn.unwrap_or(parent.latest_lsn);

        // Validate LSN is within parent's history
        if ancestor_lsn > parent.latest_lsn {
            return Err(BranchError::InvalidLsn);
        }

        // Create new timeline metadata
        let new_timeline = Arc::new(Timeline {
            timeline_id: Uuid::new_v4(),
            parent_timeline_id: Some(parent_id),
            ancestor_lsn,
            latest_lsn: ancestor_lsn, // Starts at branch point
            state: TimelineState::Active,
            created_at: std::time::SystemTime::now(),
        });

        // Create storage directory (empty, no data copied)
        self.storage
            .create_timeline(new_timeline.timeline_id)
            .await?;

        // Register timeline
        self.timelines
            .write()
            .await
            .insert(new_timeline.timeline_id, new_timeline.clone());

        Ok(new_timeline)
    }

    pub async fn get_timeline(&self, id: TimelineId) -> Option<Arc<Timeline>> {
        self.timelines.read().await.get(&id).cloned()
    }

    /// Get page at specific LSN with inheritance
    pub async fn get_page(
        &self,
        timeline_id: TimelineId,
        page_id: u32,
        lsn: Lsn,
    ) -> Result<Page, PageError> {
        let timeline = self
            .get_timeline(timeline_id)
            .await
            .ok_or(PageError::TimelineNotFound)?;

        // Walk up timeline tree to find page
        self.get_page_with_ancestry(&timeline, page_id, lsn)
            .await
    }

    async fn get_page_with_ancestry(
        &self,
        timeline: &Timeline,
        page_id: u32,
        lsn: Lsn,
    ) -> Result<Page, PageError> {
        // Try to get page from this timeline
        match self.storage.get_page(timeline.timeline_id, page_id, lsn).await {
            Ok(page) => return Ok(page),
            Err(PageError::NotFound) => {
                // Walk to parent
                if let Some(parent_id) = timeline.parent_timeline_id {
                    let parent = self
                        .get_timeline(parent_id)
                        .await
                        .ok_or(PageError::ParentNotFound)?;

                    // Use ancestor LSN for parent lookup
                    return self
                        .get_page_with_ancestry(&parent, page_id, timeline.ancestor_lsn)
                        .await;
                }
            }
            Err(e) => return Err(e),
        }

        Err(PageError::NotFound)
    }
}
```

### Page Storage Trait

```rust
use async_trait::async_trait;

/// Abstract storage interface for pages
#[async_trait]
pub trait PageStorage: Send + Sync {
    /// Create storage for new timeline (instant, metadata only)
    async fn create_timeline(&self, timeline_id: TimelineId) -> Result<(), StorageError>;

    /// Get page at specific LSN
    async fn get_page(
        &self,
        timeline_id: TimelineId,
        page_id: u32,
        lsn: Lsn,
    ) -> Result<Page, PageError>;

    /// Put a new page version
    async fn put_page(
        &self,
        timeline_id: TimelineId,
        page: Page,
    ) -> Result<(), StorageError>;

    /// Get WAL records in range
    async fn get_wal_records(
        &self,
        timeline_id: TimelineId,
        page_id: u32,
        lsn_range: std::ops::RangeInclusive<Lsn>,
    ) -> Result<Vec<WalRecord>, StorageError>;
}

/// Page data structure (PostgreSQL 8KB page)
#[derive(Debug, Clone)]
pub struct Page {
    pub data: Box<[u8; 8192]>,
    pub lsn: Lsn,
    pub timeline_id: TimelineId,
    pub page_id: u32,
}

impl Page {
    pub fn new(timeline_id: TimelineId, page_id: u32) -> Self {
        Self {
            data: Box::new([0u8; 8192]),
            lsn: Lsn::INVALID,
            timeline_id,
            page_id,
        }
    }

    pub fn with_data(
        data: [u8; 8192],
        lsn: Lsn,
        timeline_id: TimelineId,
        page_id: u32,
    ) -> Self {
        Self {
            data: Box::new(data),
            lsn,
            timeline_id,
            page_id,
        }
    }
}

/// WAL record for replay
#[derive(Debug, Clone)]
pub struct WalRecord {
    pub lsn: Lsn,
    pub record_type: WalRecordType,
    pub data: Vec<u8>,
    pub page_id: u32,
}

#[derive(Debug, Clone)]
pub enum WalRecordType {
    Insert,
    Update,
    Delete,
    Clear,
}

#[derive(Debug, thiserror::Error)]
pub enum StorageError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("S3 error: {0}")]
    S3(String),

    #[error("Timeline not found: {0}")]
    TimelineNotFound(TimelineId),
}

#[derive(Debug, thiserror::Error)]
pub enum PageError {
    #[error("Page not found")]
    NotFound,

    #[error("Timeline not found")]
    TimelineNotFound,

    #[error("Parent timeline not found")]
    ParentNotFound,

    #[error("Storage error: {0}")]
    Storage(#[from] StorageError),
}

#[derive(Debug, thiserror::Error)]
pub enum BranchError {
    #[error("Parent timeline not found")]
    ParentNotFound,

    #[error("Invalid LSN - beyond parent timeline")]
    InvalidLsn,

    #[error("Storage error: {0}")]
    Storage(#[from] StorageError),
}
```

### S3 Storage Implementation

```rust
use aws_sdk_s3::{Client as S3Client, config::Builder as S3ConfigBuilder};
use bytes::Bytes;

/// S3-backed page storage
pub struct S3PageStorage {
    client: S3Client,
    bucket: String,
    tenant_id: String,
    cache: Arc<PageCache>,
}

impl S3PageStorage {
    pub fn new(
        client: S3Client,
        bucket: String,
        tenant_id: String,
        cache_size: usize,
    ) -> Self {
        Self {
            client,
            bucket,
            tenant_id,
            cache: Arc::new(PageCache::new(cache_size)),
        }
    }

    fn wal_path(&self, timeline_id: TimelineId, lsn: Lsn) -> String {
        format!(
            "{}/{}/{}/wal/{:08X}{:08X}",
            self.tenant_id,
            timeline_id,
            "wal",
            lsn.high(),
            lsn.low()
        )
    }

    fn image_path(&self, timeline_id: TimelineId, page_id: u32, lsn: Lsn) -> String {
        format!(
            "{}/{}/{}/images/{:08X}_{:08X}",
            self.tenant_id,
            timeline_id,
            page_id,
            lsn.high(),
            lsn.low()
        )
    }
}

#[async_trait]
impl PageStorage for S3PageStorage {
    async fn create_timeline(&self, timeline_id: TimelineId) -> Result<(), StorageError> {
        // Create empty prefix in S3 (metadata only)
        let metadata_key = format!("{}/{}/metadata/timeline_info.json", self.tenant_id, timeline_id);

        self.client
            .put_object()
            .bucket(&self.bucket)
            .key(&metadata_key)
            .body(Bytes::from("{}")) // Empty metadata
            .send()
            .await
            .map_err(|e| StorageError::S3(e.to_string()))?;

        Ok(())
    }

    async fn get_page(
        &self,
        timeline_id: TimelineId,
        page_id: u32,
        lsn: Lsn,
    ) -> Result<Page, PageError> {
        // Check cache first
        if let Some(cached) = self.cache.get(timeline_id, page_id, lsn) {
            return Ok(cached);
        }

        // Try to get base image
        let image_key = self.image_path(timeline_id, page_id, lsn);

        match self
            .client
            .get_object()
            .bucket(&self.bucket)
            .key(&image_key)
            .send()
            .await
        {
            Ok(response) => {
                let body = response.body.collect().await.map_err(|e| {
                    StorageError::S3(e.to_string())
                })?;

                let data: [u8; 8192] = body.into_bytes().as_ref().try_into().map_err(|_| {
                    PageError::Storage(StorageError::S3("Invalid page size".into()))
                })?;

                let page = Page::with_data(data, lsn, timeline_id, page_id);
                self.cache.put(page.clone());
                return Ok(page);
            }
            Err(_) => {
                // No base image, need to reconstruct from WAL
                // Implementation would walk WAL and replay
                return Err(PageError::NotFound);
            }
        }
    }

    async fn put_page(
        &self,
        timeline_id: TimelineId,
        page: Page,
    ) -> Result<(), StorageError> {
        let image_key = self.image_path(timeline_id, page.page_id, page.lsn);

        self.client
            .put_object()
            .bucket(&self.bucket)
            .key(&image_key)
            .body(Bytes::copy_from_slice(&page.data))
            .send()
            .await
            .map_err(|e| StorageError::S3(e.to_string()))?;

        self.cache.put(page.clone());
        Ok(())
    }

    async fn get_wal_records(
        &self,
        timeline_id: TimelineId,
        page_id: u32,
        lsn_range: std::ops::RangeInclusive<Lsn>,
    ) -> Result<Vec<WalRecord>, StorageError> {
        let mut records = Vec::new();

        for lsn in lsn_range {
            let wal_key = self.wal_path(timeline_id, lsn);

            match self
                .client
                .get_object()
                .bucket(&self.bucket)
                .key(&wal_key)
                .send()
                .await
            {
                Ok(response) => {
                    let body = response.body.collect().await.map_err(|e| {
                        StorageError::S3(e.to_string())
                    })?;

                    // Parse WAL record from bytes
                    if let Ok(record) = self.parse_wal_record(&body, page_id, lsn) {
                        records.push(record);
                    }
                }
                Err(_) => {
                    // WAL record not found, skip
                    continue;
                }
            }
        }

        Ok(records)
    }
}

impl S3PageStorage {
    fn parse_wal_record(
        &self,
        data: &bytes::Bytes,
        page_id: u32,
        lsn: Lsn,
    ) -> Result<WalRecord, StorageError> {
        // Parse WAL record header and body
        // Implementation depends on PostgreSQL WAL format
        todo!("Implement WAL record parsing")
    }
}

/// In-memory LRU cache for pages
pub struct PageCache {
    inner: RwLock<lru::LruCache<CacheKey, Page>>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct CacheKey {
    timeline_id: TimelineId,
    page_id: u32,
    lsn: Lsn,
}

impl PageCache {
    pub fn new(capacity: usize) -> Self {
        Self {
            inner: RwLock::new(lru::LruCache::new(capacity)),
        }
    }

    pub fn get(&self, timeline_id: TimelineId, page_id: u32, lsn: Lsn) -> Option<Page> {
        let key = CacheKey {
            timeline_id,
            page_id,
            lsn,
        };
        self.inner.write().get(&key).cloned()
    }

    pub fn put(&self, page: Page) {
        let key = CacheKey {
            timeline_id: page.timeline_id,
            page_id: page.page_id,
            lsn: page.lsn,
        };
        self.inner.write().put(key, page);
    }
}
```

## Part 6: Branch Operations

### Branch Creation API

```rust
use axum::{
    extract::{Path, State},
    Json,
    routing::post,
    Router,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
pub struct CreateBranchRequest {
    pub name: String,
    pub parent_id: TimelineId,
    #[serde(default)]
    pub parent_lsn: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct CreateBranchResponse {
    pub id: TimelineId,
    pub name: String,
    pub parent_id: TimelineId,
    pub parent_lsn: String,
    pub created_at: String,
}

pub async fn create_branch(
    State(manager): State<Arc<TimelineManager>>,
    Path(tenant_id): Path<String>,
    Json(req): Json<CreateBranchRequest>,
) -> Result<Json<CreateBranchResponse>, BranchError> {
    // Parse LSN if provided
    let branch_lsn = req
        .parent_lsn
        .map(|s| parse_lsn(&s))
        .transpose()
        .map_err(|_| BranchError::InvalidLsn)?;

    // Create branch (instant!)
    let timeline = manager
        .create_branch(req.parent_id, &req.name, branch_lsn)
        .await?;

    Ok(Json(CreateBranchResponse {
        id: timeline.timeline_id,
        name: req.name,
        parent_id: req.parent_id,
        parent_lsn: timeline.ancestor_lsn.to_string_lossy(),
        created_at: format!("{:?}", timeline.created_at),
    }))
}

fn parse_lsn(s: &str) -> Result<Lsn, String> {
    let parts: Vec<&str> = s.split('/').collect();
    if parts.len() != 2 {
        return Err("Invalid LSN format".into());
    }

    let high = u32::from_str_radix(parts[0], 16).map_err(|_| "Invalid high")?;
    let low = u32::from_str_radix(parts[1], 16).map_err(|_| "Invalid low")?;

    Ok(Lsn::new(high, low))
}

// Router setup
fn app_routes(manager: Arc<TimelineManager>) -> Router {
    Router::new()
        .route(
            "/tenants/:tenant_id/branches",
            post(create_branch).with_state(manager),
        )
}
```

### Branch Deletion with Garbage Collection

```rust
impl TimelineManager {
    /// Delete a branch (mark for GC)
    pub async fn delete_branch(&self, timeline_id: TimelineId) -> Result<(), BranchError> {
        let mut timelines = self.timelines.write().await;

        let timeline = timelines
            .get(&timeline_id)
            .cloned()
            .ok_or(BranchError::ParentNotFound)?;

        // Check if any child timelines depend on this timeline
        let has_children = timelines
            .values()
            .any(|t| t.parent_timeline_id == Some(timeline_id));

        if has_children {
            // Cannot delete - children depend on this branch
            return Err(BranchError::HasDependents);
        }

        // Mark as stopping (will be GC'd later)
        // Don't delete immediately - need to check retention policy
        self.storage.mark_for_gc(timeline_id).await?;

        timelines.remove(&timeline_id);

        Ok(())
    }

    /// Garbage collection - remove old page versions
    pub async fn run_gc(&self, retention_period: Duration) -> Result<usize, StorageError> {
        let cutoff_lsn = self.get_lsn_before(retention_period).await?;
        let mut removed = 0;

        for timeline in self.timelines.read().await.values() {
            // Find pages older than cutoff that aren't needed by any branch
            let pages_to_remove = self
                .storage
                .find_gc_candidates(timeline.timeline_id, cutoff_lsn)
                .await?;

            for page in pages_to_remove {
                self.storage.delete_page(page).await?;
                removed += 1;
            }
        }

        Ok(removed)
    }
}

#[derive(Debug, thiserror::Error)]
pub enum BranchError {
    #[error("Parent timeline not found")]
    ParentNotFound,

    #[error("Invalid LSN - beyond parent timeline")]
    InvalidLsn,

    #[error("Cannot delete - branch has dependent timelines")]
    HasDependents,

    #[error("Storage error: {0}")]
    Storage(#[from] StorageError),
}
```

---

*This document is part of the Neodatabase exploration series. See [exploration.md](./exploration.md) for the complete index.*
