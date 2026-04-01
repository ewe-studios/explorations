---
title: "OrbitingHail Sync Protocol Deep Dive"
subtitle: "SQLSync protocol, CRDT merge operations, and conflict resolution"
location: /home/darkvoid/Boxxed/@dev/repo-expolorations/src.db/src.orbitinghail
related: 01-storage-engine-deep-dive.md
---

# 02 - Sync Protocol Deep Dive: OrbitingHail

## Overview

This document covers the SQLSync protocol internals - message formats, CRDT merge operations, conflict resolution strategies, WebSocket communication patterns, and sync optimization techniques.

## Part 1: The Sync Protocol Architecture

### High-Level Sync Flow

```
┌──────────────────────────────────────────────────────────────────┐
│                    SQLSync Protocol Stack                        │
├──────────────────────────────────────────────────────────────────┤
│                                                                  │
│  ┌────────────────────────────────────────────────────────────┐ │
│  │              Application Layer (SQL Operations)            │ │
│  │  INSERT/UPDATE/DELETE → CRDT Operations → Change Records   │ │
│  └────────────────────────────────────────────────────────────┘ │
│                              │                                   │
│                              ▼                                   │
│  ┌────────────────────────────────────────────────────────────┐ │
│  │            Sync Protocol Layer (Message Builder)           │ │
│  │  ChangeLog → SyncRequest → Serialization → WebSocket       │ │
│  └────────────────────────────────────────────────────────────┘ │
│                              │                                   │
│                              ▼                                   │
│  ┌────────────────────────────────────────────────────────────┐ │
│  │           Transport Layer (WebSocket Communication)        │ │
│  │  Connect → Authenticate → Send/Receive → Reconnect         │ │
│  └────────────────────────────────────────────────────────────┘ │
│                              │                                   │
│                              ▼                                   │
│  ┌────────────────────────────────────────────────────────────┐ │
│  │             Server Layer (Sync Coordinator)                │ │
│  │  Validate → Merge → Conflict Resolution → Acknowledge      │ │
│  └────────────────────────────────────────────────────────────┘ │
│                                                                  │
└──────────────────────────────────────────────────────────────────┘
```

### Sync Phases

```
┌──────────────────────────────────────────────────────────────────┐
│                    Complete Sync Cycle                           │
├──────────────────────────────────────────────────────────────────┤
│                                                                  │
│  Phase 1: Client Preparation                                     │
│  ┌────────────────────────────────────────────────────────────┐ │
│  │  1. Read current vector clock                              │ │
│  │  2. Query change log for unsynced changes                  │ │
│  │  3. Group changes by table                                 │ │
│  │  4. Build SyncRequest message                              │ │
│  │  5. Serialize to JSON/MessagePack                          │ │
│  └────────────────────────────────────────────────────────────┘ │
│                              │                                   │
│                              ▼                                   │
│  Phase 2: Network Transmission                                   │
│  ┌────────────────────────────────────────────────────────────┐ │
│  │  1. Establish WebSocket connection                         │ │
│  │  2. Send authentication handshake                          │ │
│  │  3. Transmit SyncRequest                                   │ │
│  │  4. Wait for SyncResponse                                  │ │
│  │  5. Handle connection drops (retry with backoff)           │ │
│  └────────────────────────────────────────────────────────────┘ │
│                              │                                   │
│                              ▼                                   │
│  Phase 3: Server Processing                                      │
│  ┌────────────────────────────────────────────────────────────┐ │
│  │  1. Parse and validate request                             │ │
│  │  2. Check client clock against server state                │ │
│  │  3. Apply uploaded changes with CRDT merge                 │ │
│  │  4. Determine conflicts                                    │ │
│  │  5. Query changes client needs (download)                  │ │
│  │  6. Build SyncResponse                                     │ │
│  └────────────────────────────────────────────────────────────┘ │
│                              │                                   │
│                              ▼                                   │
│  Phase 4: Client Application                                     │
│  ┌────────────────────────────────────────────────────────────┐ │
│  │  1. Parse SyncResponse                                     │ │
│  │  2. Apply downloaded changes to local DB                   │ │
│  │  3. Mark uploaded changes as synced                        │ │
│  │  4. Merge server clock with local clock                    │ │
│  │  5. Notify application of sync complete                    │ │
│  └────────────────────────────────────────────────────────────┘ │
│                                                                  │
└──────────────────────────────────────────────────────────────────┘
```

## Part 2: Message Formats

### SyncRequest Structure

```rust
use serde::{Serialize, Deserialize};
use std::collections::HashMap;

/// Main sync request sent from client to server
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncRequest {
    /// Client's current vector clock - tells server what client knows
    pub client_clock: VectorClock,

    /// Changes grouped by table name
    /// Key: table name (e.g., "documents", "users")
    /// Value: list of changes to upload
    pub tables: HashMap<String, Vec<ChangeRecord>>,

    /// Optional: Client capabilities/features
    #[serde(skip_serializing_if = "Option::is_none")]
    pub capabilities: Option<ClientCapabilities>,

    /// Optional: Request metadata (client version, timestamp)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<RequestMetadata>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ClientCapabilities {
    /// Supports CRDT field-level merge
    pub field_level_merge: bool,

    /// Supports batch operations
    pub batch_operations: bool,

    /// Maximum change batch size client can handle
    pub max_download_batch: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RequestMetadata {
    /// Client version for compatibility checks
    pub client_version: String,

    /// Unix timestamp in microseconds
    pub request_timestamp: u64,

    /// Optional device identifier
    pub device_id: Option<String>,
}
```

### SyncResponse Structure

```rust
/// Main sync response from server to client
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncResponse {
    /// Server's vector clock after processing
    /// Client must merge this with local clock
    pub server_clock: VectorClock,

    /// Which uploaded changes were accepted
    /// Key: table name
    /// Value: list of accepted change IDs (row_id or change_log id)
    pub accepted: HashMap<String, Vec<String>>,

    /// Changes rejected by server (conflicts that couldn't be auto-resolved)
    #[serde(skip_serializing_if = "HashMap::is_empty")]
    pub rejected: HashMap<String, Vec<RejectedChange>>,

    /// Changes to download (server → client)
    /// Key: table name
    /// Value: list of changes client needs
    pub download: HashMap<String, Vec<ChangeRecord>>,

    /// Server instructions/commands
    #[serde(skip_serializing_if = "Option::is_none")]
    pub instructions: Option<ServerInstructions>,

    /// Response metadata
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<ResponseMetadata>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RejectedChange {
    /// The rejected change
    pub change: ChangeRecord,

    /// Reason for rejection
    pub reason: RejectionReason,

    /// Suggested resolution (if any)
    pub suggested_resolution: Option<ChangeRecord>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "details")]
pub enum RejectionReason {
    /// Schema version mismatch
    SchemaMismatch {
        expected_version: u32,
        actual_version: u32,
    },

    /// Validation failed (constraint violation)
    ValidationFailed {
        constraint: String,
        message: String,
    },

    /// Causality violation (missing dependencies)
    CausalityViolation {
        missing_dependencies: Vec<String>,
    },

    /// Permission denied
    PermissionDenied {
        required_permission: String,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ServerInstructions {
    /// Tables client should subscribe to for push notifications
    pub subscribe_to_tables: Vec<String>,

    /// Suggested sync interval (milliseconds)
    pub suggested_sync_interval_ms: Option<u64>,

    /// Server is under load, reduce sync frequency
    pub backoff_requested: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResponseMetadata {
    /// Server version
    pub server_version: String,

    /// Processing time in milliseconds
    pub processing_time_ms: u32,

    /// Total changes processed
    pub changes_processed: usize,

    /// Total conflicts resolved
    pub conflicts_resolved: usize,
}
```

### ChangeRecord Structure

```rust
/// A single change to be synced
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChangeRecord {
    /// Unique identifier for this change (client-generated UUID or change_log id)
    pub id: String,

    /// Table name
    pub table: String,

    /// Row identifier
    pub row_id: String,

    /// Operation type
    pub operation: Operation,

    /// The actual changes (CRDT-encoded)
    pub changes: serde_json::Value,

    /// Vector clock at time of change
    pub clock: VectorClock,

    /// Timestamp in microseconds (for LWW tie-breaking)
    pub timestamp: u64,

    /// Schema version (for migration compatibility)
    pub schema_version: u32,

    /// Optional: Column-level timestamps for fine-grained LWW
    #[serde(skip_serializing_if = "Option::is_none")]
    pub column_timestamps: Option<HashMap<String, u64>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum Operation {
    /// INSERT INTO table (...) VALUES (...)
    Insert,

    /// UPDATE table SET ... WHERE ...
    Update,

    /// DELETE FROM table WHERE ...
    Delete,
}
```

### VectorClock Implementation

```rust
use serde::{Serialize, Deserialize};
use std::collections::HashMap;

/// Vector clock for causality tracking and concurrency detection
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
pub struct VectorClock {
    /// Map of actor_id → counter
    counters: HashMap<String, u64>,
}

impl VectorClock {
    /// Create a new empty vector clock
    pub fn new() -> Self {
        Self {
            counters: HashMap::new(),
        }
    }

    /// Create from existing counters
    pub fn from_counters(counters: HashMap<String, u64>) -> Self {
        Self { counters }
    }

    /// Increment the counter for this actor
    pub fn tick(&mut self, actor: &str) {
        let counter = self.counters.entry(actor.to_string()).or_insert(0);
        *counter += 1;
    }

    /// Get the counter for a specific actor
    pub fn get(&self, actor: &str) -> Option<u64> {
        self.counters.get(actor).copied()
    }

    /// Get all actors
    pub fn actors(&self) -> impl Iterator<Item = &String> {
        self.counters.keys()
    }

    /// Merge with another vector clock (pointwise maximum)
    pub fn merge(&mut self, other: &Self) {
        for (actor, count) in &other.counters {
            let entry = self.counters.entry(actor.clone()).or_insert(0);
            *entry = (*entry).max(*count);
        }
    }

    /// Compare two vector clocks
    /// Returns the ordering relationship
    pub fn compare(&self, other: &Self) -> ClockOrdering {
        let mut less = false;
        let mut greater = false;

        // Get all actors from both clocks
        let all_actors: std::collections::HashSet<_> = self
            .counters
            .keys()
            .chain(other.counters.keys())
            .collect();

        for actor in all_actors {
            let self_count = self.counters.get(actor).copied().unwrap_or(0);
            let other_count = other.counters.get(actor).copied().unwrap_or(0);

            if self_count < other_count {
                less = true;
            } else if self_count > other_count {
                greater = true;
            }
        }

        match (less, greater) {
            (true, true) => ClockOrdering::Concurrent,
            (true, false) => ClockOrdering::Before,
            (false, true) => ClockOrdering::After,
            (false, false) => ClockOrdering::Equal,
        }
    }

    /// Check if this clock dominates (happens-after) another
    pub fn dominates(&self, other: &Self) -> bool {
        self.compare(other) == ClockOrdering::After
    }

    /// Check if clocks are concurrent (neither dominates)
    pub fn is_concurrent(&self, other: &Self) -> bool {
        self.compare(other) == ClockOrdering::Concurrent
    }

    /// Serialize to compact binary format
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::new();

        // Write number of entries
        let len = self.counters.len() as u32;
        bytes.extend_from_slice(&len.to_le_bytes());

        // Write each (actor, count) pair
        for (actor, count) in &self.counters {
            // Actor length + actor bytes
            bytes.push(actor.len() as u8);
            bytes.extend_from_slice(actor.as_bytes());
            // Counter
            bytes.extend_from_slice(&count.to_le_bytes());
        }

        bytes
    }

    /// Deserialize from compact binary format
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, ClockParseError> {
        if bytes.len() < 4 {
            return Err(ClockParseError::TooShort);
        }

        let len = u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]) as usize;
        let mut counters = HashMap::with_capacity(len);

        let mut offset = 4;
        for _ in 0..len {
            if offset >= bytes.len() {
                return Err(ClockParseError::TooShort);
            }

            let actor_len = bytes[offset] as usize;
            offset += 1;

            if offset + actor_len + 8 > bytes.len() {
                return Err(ClockParseError::TooShort);
            }

            let actor = String::from_utf8(bytes[offset..offset + actor_len].to_vec())
                .map_err(|_| ClockParseError::InvalidUtf8)?;
            offset += actor_len;

            let count_bytes: [u8; 8] = bytes[offset..offset + 8]
                .try_into()
                .map_err(|_| ClockParseError::TooShort)?;
            let count = u64::from_le_bytes(count_bytes);
            offset += 8;

            counters.insert(actor, count);
        }

        Ok(Self { counters })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClockOrdering {
    /// self < other (self happened before other)
    Before,
    /// self > other (self happened after other)
    After,
    /// Concurrent (neither dominates - potential conflict)
    Concurrent,
    /// Equal (identical clocks)
    Equal,
}

#[derive(Debug, thiserror::Error)]
pub enum ClockParseError {
    #[error("Buffer too short")]
    TooShort,
    #[error("Invalid UTF-8 in actor ID")]
    InvalidUtf8,
}
```

## Part 3: CRDT Merge Operations

### LWW Register Merge (Scalar Columns)

```rust
/// Last-Write-Wins Register for scalar column values
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LWWRegister<T> {
    /// The current value
    value: T,

    /// Timestamp in microseconds
    timestamp: u64,

    /// Actor ID (for deterministic tie-breaking)
    actor: String,
}

impl<T: Clone + PartialEq> LWWRegister<T> {
    /// Create a new LWW Register
    pub fn new(value: T, timestamp: u64, actor: String) -> Self {
        Self { value, timestamp, actor }
    }

    /// Set a new value (local update)
    pub fn set(&mut self, value: T, timestamp: u64, actor: String) {
        // Always accept local updates with newer timestamps
        if timestamp > self.timestamp
            || (timestamp == self.timestamp && actor > self.actor)
        {
            self.value = value;
            self.timestamp = timestamp;
            self.actor = actor;
        }
    }

    /// Get the current value
    pub fn get(&self) -> &T {
        &self.value
    }

    /// Merge with another LWW Register (CRDT merge operation)
    /// This is the core conflict resolution logic
    pub fn merge(&mut self, other: &Self) {
        // LWW: Keep the value with the highest timestamp
        // Tie-breaker: Use actor ID lexicographically
        if other.timestamp > self.timestamp {
            self.value = other.value.clone();
            self.timestamp = other.timestamp;
            self.actor = other.actor.clone();
        } else if other.timestamp == self.timestamp && other.actor > self.actor {
            self.value = other.value.clone();
            self.timestamp = other.timestamp;
            self.actor = other.actor.clone();
        }
        // else: self is already the winner, keep it
    }

    /// Get metadata for debugging
    pub fn metadata(&self) -> LWWMetadata {
        LWWMetadata {
            timestamp: self.timestamp,
            actor: self.actor.clone(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct LWWMetadata {
    pub timestamp: u64,
    pub actor: String,
}

// Example: Merging conflicting updates to a user's name
#[cfg(test)]
mod lww_tests {
    use super::*;

    #[test]
    fn test_lww_merge_no_conflict() {
        // Two independent updates
        let mut register = LWWRegister::new("initial".to_string(), 100, "actor_a".to_string());
        let update = LWWRegister::new("updated".to_string(), 200, "actor_b".to_string());

        register.merge(&update);

        assert_eq!(register.get(), &"updated");
        assert_eq!(register.metadata().timestamp, 200);
    }

    #[test]
    fn test_lww_merge_conflict_timestamp() {
        // Conflict: Same timestamp, different actors
        let mut register_a = LWWRegister::new("alice".to_string(), 100, "actor_a".to_string());
        let register_b = LWWRegister::new("bob".to_string(), 100, "actor_b".to_string());

        register_a.merge(&register_b);

        // actor_b > actor_a lexicographically, so bob wins
        assert_eq!(register_a.get(), &"bob");
    }

    #[test]
    fn test_lww_merge_concurrent_wins() {
        // Actor A has newer timestamp
        let mut register_a = LWWRegister::new("newer".to_string(), 200, "actor_a".to_string());
        let register_b = LWWRegister::new("older".to_string(), 100, "actor_b".to_string());

        register_a.merge(&register_b);

        // Higher timestamp wins
        assert_eq!(register_a.get(), &"newer");
    }
}
```

### OR-Set Merge (Array/Set Columns)

```rust
/// Observed-Remove Set for array/set columns
/// Supports concurrent add/remove operations without conflicts
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ORSet<T> {
    /// Elements currently in the set
    /// Key: element value
    /// Value: list of (actor, unique_id) tags that added it
    elements: HashMap<T, Vec<(String, u64)>>,

    /// Tombstones (removed elements)
    /// Key: element value
    /// Value: list of (actor, unique_id) tags that were removed
    tombstones: HashMap<T, Vec<(String, u64)>>,

    /// Counter for generating unique IDs
    #[serde(skip)]
    unique_counter: u64,

    /// Actor ID for this instance
    #[serde(skip)]
    actor_id: String,
}

impl<T: Eq + std::hash::Hash + Clone + std::fmt::Debug> ORSet<T> {
    /// Create a new OR-Set
    pub fn new(actor_id: String) -> Self {
        Self {
            elements: HashMap::new(),
            tombstones: HashMap::new(),
            unique_counter: 0,
            actor_id,
        }
    }

    /// Generate a unique tag ID
    fn generate_unique_id(&mut self) -> u64 {
        let id = self.unique_counter;
        self.unique_counter += 1;
        id
    }

    /// Add an element to the set
    pub fn add(&mut self, element: T) -> (String, u64) {
        let unique_id = self.generate_unique_id();
        let tag = (self.actor_id.clone(), unique_id);

        self.elements
            .entry(element)
            .or_insert_with(Vec::new)
            .push(tag);

        tag
    }

    /// Remove an element from the set
    /// Moves all current tags to tombstones
    pub fn remove(&mut self, element: &T) {
        if let Some(tags) = self.elements.remove(element) {
            self.tombstones
                .entry(element.clone())
                .or_insert_with(Vec::new)
                .extend(tags);
        }
    }

    /// Check if element is in the set
    pub fn contains(&self, element: &T) -> bool {
        let element_tags = self.elements.get(element);
        let tombstone_tags = self.tombstones.get(element);

        match (element_tags, tombstone_tags) {
            (Some(tags), None) => !tags.is_empty(),
            (Some(tags), Some(tombs)) => {
                // Element exists if it has any tags not in tombstones
                tags.iter().any(|tag| !tombs.contains(tag))
            }
            _ => false,
        }
    }

    /// Get all elements in the set
    pub fn iter(&self) -> impl Iterator<Item = &T> {
        self.elements.keys().filter(move |e| self.contains(e))
    }

    /// Merge with another OR-Set (CRDT merge operation)
    /// Union of elements, union of tombstones
    pub fn merge(&mut self, other: &Self) {
        // Merge elements (union)
        for (element, tags) in &other.elements {
            self.elements
                .entry(element.clone())
                .or_insert_with(Vec::new)
                .extend(tags);
        }

        // Merge tombstones (union)
        for (element, tags) in &other.tombstones {
            self.tombstones
                .entry(element.clone())
                .or_insert_with(Vec::new)
                .extend(tags);
        }

        // Update unique counter to avoid collisions
        let max_other_counter = other.elements.values()
            .flat_map(|tags| tags.iter().map(|(_, id)| *id))
            .chain(
                other.tombstones.values()
                    .flat_map(|tags| tags.iter().map(|(_, id)| *id))
            )
            .max()
            .unwrap_or(0);

        self.unique_counter = self.unique_counter.max(max_other_counter + 1);
    }

    /// Get current state for serialization
    pub fn to_state(&self) -> ORSetState<T> {
        ORSetState {
            elements: self.elements.clone(),
            tombstones: self.tombstones.clone(),
        }
    }

    /// Restore from state (for initial sync)
    pub fn from_state(state: ORSetState<T>, actor_id: String) -> Self {
        let max_id = state.elements.values()
            .flat_map(|tags| tags.iter().map(|(_, id)| *id))
            .chain(
                state.tombstones.values()
                    .flat_map(|tags| tags.iter().map(|(_, id)| *id))
            )
            .max()
            .unwrap_or(0);

        Self {
            elements: state.elements,
            tombstones: state.tombstones,
            unique_counter: max_id + 1,
            actor_id,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ORSetState<T> {
    elements: HashMap<T, Vec<(String, u64)>>,
    tombstones: HashMap<T, Vec<(String, u64)>>,
}

// Example: Collaborative tagging
#[cfg(test)]
mod orset_tests {
    use super::*;

    #[test]
    fn test_orset_concurrent_add() {
        // Two actors independently add "rust" tag
        let mut set_a = ORSet::new("actor_a".to_string());
        let mut set_b = ORSet::new("actor_b".to_string());

        set_a.add("rust".to_string());
        set_b.add("rust".to_string());

        // Merge
        set_a.merge(&set_b);

        // Both adds preserved - element exists twice with different tags
        assert!(set_a.contains(&"rust".to_string()));
    }

    #[test]
    fn test_orset_add_remove_concurrent() {
        // Actor A adds, Actor B removes concurrently
        let mut set_a = ORSet::new("actor_a".to_string());
        let tag = set_a.add("item".to_string());

        let mut set_b = ORSet::new("actor_b".to_string());
        // set_b doesn't see the add, removes independently
        set_b.remove(&"item".to_string());

        // Merge
        set_a.merge(&set_b);

        // Add wins because B didn't have the tag to remove
        // (B's tombstone is empty for "item")
        // Actually, let's trace this:
        // - set_a: elements = {"item": [("actor_a", 0)]}
        // - set_b: tombstones = {"item": []} (nothing to remove)
        // After merge:
        // - elements = {"item": [("actor_a", 0)]}
        // - tombstones = {"item": []}
        // Contains: has tag ("actor_a", 0), not in tombstones -> true
        assert!(set_a.contains(&"item".to_string()));
    }

    #[test]
    fn test_orset_add_then_remove_merged() {
        // Proper sequence: add from A, merge to B, B removes
        let mut set_a = ORSet::new("actor_a".to_string());
        set_a.add("item".to_string());

        let mut set_b = ORSet::<String>::new("actor_b".to_string());
        set_b.merge(&set_a);  // B now has the item

        set_b.remove(&"item".to_string());  // B removes

        set_a.merge(&set_b);  // Merge back to A

        // Both should see removal
        assert!(!set_a.contains(&"item".to_string()));
        assert!(!set_b.contains(&"item".to_string()));
    }
}
```

### MV-Register Merge (Multi-Value for Concurrent Writes)

```rust
/// Multi-Value Register for handling concurrent writes
/// Keeps all concurrent values instead of discarding
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct MVRegister<T> {
    /// Values with their causal contexts
    values: Vec<VersionedValue<T>>,

    /// Actor ID
    #[serde(skip)]
    actor_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VersionedValue<T> {
    value: T,
    clock: VectorClock,
}

impl<T: Clone + PartialEq> MVRegister<T> {
    /// Create a new MV Register
    pub fn new(actor_id: String) -> Self {
        Self {
            values: Vec::new(),
            actor_id,
        }
    }

    /// Set a new value (replaces all current values)
    pub fn set(&mut self, value: T) {
        let mut clock = VectorClock::new();
        clock.tick(&self.actor_id);

        self.values = vec![VersionedValue { value, clock }];
    }

    /// Set with explicit clock (for sync operations)
    pub fn set_with_clock(&mut self, value: T, clock: VectorClock) {
        self.values = vec![VersionedValue { value, clock }];
    }

    /// Get current values (may be multiple if concurrent writes)
    pub fn get(&self) -> Vec<&T> {
        self.values.iter().map(|v| &v.value).collect()
    }

    /// Get single value (picks first if multiple)
    pub fn get_single(&self) -> Option<&T> {
        self.values.first().map(|v| &v.value)
    }

    /// Merge with another MV Register
    pub fn merge(&mut self, other: &Self) {
        let mut new_values = Vec::new();

        // Compare each pair of values
        for self_vv in &self.values {
            for other_vv in &other.values {
                match self_vv.clock.compare(&other_vv.clock) {
                    ClockOrdering::Before => {
                        // other dominates, will be added separately
                    }
                    ClockOrdering::After => {
                        // self dominates, keep it
                        if !new_values.iter().any(|v: &VersionedValue<T>| {
                            v.clock == self_vv.clock
                        }) {
                            new_values.push(self_vv.clone());
                        }
                    }
                    ClockOrdering::Concurrent => {
                        // Both are concurrent, keep both
                        if !new_values.iter().any(|v: &VersionedValue<T>| {
                            v.clock == self_vv.clock
                        }) {
                            new_values.push(self_vv.clone());
                        }
                        if !new_values.iter().any(|v: &VersionedValue<T>| {
                            v.clock == other_vv.clock
                        }) {
                            new_values.push(other_vv.clone());
                        }
                    }
                    ClockOrdering::Equal => {
                        // Same value, keep one
                        if !new_values.iter().any(|v: &VersionedValue<T>| {
                            v.clock == self_vv.clock && v.value == self_vv.value
                        }) {
                            new_values.push(self_vv.clone());
                        }
                    }
                }
            }
        }

        // Also add any other values not yet considered
        for other_vv in &other.values {
            if !new_values.iter().any(|v: &VersionedValue<T>| {
                v.clock == other_vv.clock
            }) {
                new_values.push(other_vv.clone());
            }
        }

        self.values = new_values;
    }

    /// Resolve conflicts by picking a value (application-specific logic)
    pub fn resolve<F>(&mut self, resolver: F)
    where
        F: Fn(Vec<&T>) -> T,
    {
        let values: Vec<&T> = self.get();
        if values.len() > 1 {
            let chosen = resolver(values);
            self.set(chosen);
        }
    }
}

// Example: Collaborative field with conflict visibility
#[cfg(test)]
mod mv_tests {
    use super::*;

    #[test]
    fn test_mv_concurrent_writes() {
        let mut reg_a = MVRegister::new("alice".to_string());
        let mut reg_b = MVRegister::new("bob".to_string());

        // Both set different values concurrently
        reg_a.set("alice's value".to_string());
        reg_b.set("bob's value".to_string());

        // Merge
        reg_a.merge(&reg_b);

        // Both values preserved (application must resolve)
        let values = reg_a.get();
        assert_eq!(values.len(), 2);
    }

    #[test]
    fn test_mv_sequential_writes() {
        let mut reg_a = MVRegister::new("alice".to_string());
        reg_a.set("first".to_string());

        let mut reg_b = MVRegister::new("bob".to_string());
        reg_b.merge(&reg_a);  // B sees "first"
        reg_b.set("second".to_string());  // B updates

        reg_a.merge(&reg_b);  // A sees B's update

        // Only one value (causal sequence)
        assert_eq!(reg_a.get_single(), Some(&"second".to_string()));
    }
}
```

## Part 4: Sync Server Implementation

### Basic Sync Handler

```rust
use tokio::sync::RwLock;
use std::sync::Arc;
use std::collections::HashMap;

/// Server-side sync handler
pub struct SyncServer {
    /// Central database connection
    db: Arc<DatabaseConnection>,

    /// Global server clock
    clock: Arc<RwLock<ServerClock>>,

    /// Connected clients
    clients: Arc<RwLock<HashMap<String, ClientConnection>>>,
}

impl SyncServer {
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self {
            db,
            clock: Arc::new(RwLock::new(ServerClock::new())),
            clients: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Process a sync request from a client
    pub async fn process_sync(
        &self,
        client_id: &str,
        request: SyncRequest,
    ) -> Result<SyncResponse, SyncError> {
        let mut server_clock = self.clock.write().await;

        // Step 1: Validate client clock
        self.validate_client_clock(&request.client_clock)?;

        // Step 2: Process uploaded changes
        let mut accepted = HashMap::new();
        let mut rejected = HashMap::new();
        let mut conflicts = 0;

        for (table, changes) in request.tables {
            let (table_accepted, table_rejected, table_conflicts) =
                self.process_table_changes(&table, changes, &mut server_clock).await?;

            accepted.insert(table.clone(), table_accepted);
            if !table_rejected.is_empty() {
                rejected.insert(table, table_rejected);
            }
            conflicts += table_conflicts;
        }

        // Step 3: Determine changes client needs
        let download = self.get_changes_for_client(
            client_id,
            &request.client_clock,
            &accepted,
        ).await?;

        // Step 4: Build response
        let response = SyncResponse {
            server_clock: server_clock.to_vector_clock(),
            accepted,
            rejected,
            download,
            instructions: None,
            metadata: Some(ResponseMetadata {
                server_version: env!("CARGO_PKG_VERSION").to_string(),
                processing_time_ms: 0, // Would track actual time
                changes_processed: accepted.values().map(|v| v.len()).sum(),
                conflicts_resolved: conflicts,
            }),
        };

        Ok(response)
    }

    /// Process changes for a single table
    async fn process_table_changes(
        &self,
        table: &str,
        changes: Vec<ChangeRecord>,
        server_clock: &mut ServerClock,
    ) -> Result<(Vec<String>, Vec<RejectedChange>, usize), SyncError> {
        let mut accepted = Vec::new();
        let mut rejected = Vec::new();
        let mut conflicts = 0;

        for change in changes {
            match self.apply_change_with_conflict_resolution(table, &change, server_clock).await {
                ApplyResult::Accepted(change_id) => {
                    accepted.push(change_id);
                }
                ApplyResult::Rejected(reason, suggestion) => {
                    rejected.push(RejectedChange {
                        change,
                        reason,
                        suggested_resolution: suggestion,
                    });
                }
                ApplyResult::ConflictResolved(_) => {
                    accepted.push(change.row_id.clone());
                    conflicts += 1;
                }
            }
        }

        Ok((accepted, rejected, conflicts))
    }

    /// Apply a single change with conflict resolution
    async fn apply_change_with_conflict_resolution(
        &self,
        table: &str,
        change: &ChangeRecord,
        server_clock: &mut ServerClock,
    ) -> ApplyResult {
        // Get current state from database
        let current_state = match self.db.get_row(table, &change.row_id).await {
            Ok(state) => state,
            Err(_) => return ApplyResult::Rejected(
                RejectionReason::ValidationFailed {
                    constraint: "row_not_found".to_string(),
                    message: format!("Row {} not found in table {}", change.row_id, table),
                },
                None,
            ),
        };

        // Check for conflicts using vector clocks
        if let Some(existing_clock) = current_state.clock {
            match existing_clock.compare(&change.clock) {
                ClockOrdering::After => {
                    // Server has newer change - this is stale
                    // But may still be applicable if no column overlap
                    if self.can_merge_stale_change(&current_state, change) {
                        // Field-level merge possible
                        return self.merge_field_level(&current_state, change, server_clock);
                    }
                    return ApplyResult::ConflictResolved(change.row_id.clone());
                }
                ClockOrdering::Concurrent => {
                    // True conflict - concurrent modifications
                    conflicts += 1;
                    return self.resolve_conflict(&current_state, change, server_clock);
                }
                _ => {
                    // Change is newer than server - just apply it
                }
            }
        }

        // Apply the change
        match change.operation {
            Operation::Insert => {
                self.db.insert_row(table, &change.row_id, &change.changes).await?;
            }
            Operation::Update => {
                self.db.update_row(table, &change.row_id, &change.changes).await?;
            }
            Operation::Delete => {
                self.db.delete_row(table, &change.row_id).await?;
            }
        }

        // Update server clock
        server_clock.merge(&change.clock);

        ApplyResult::Accepted(change.row_id.clone())
    }

    /// Check if a stale change can still be merged (field-level)
    fn can_merge_stale_change(
        &self,
        current: &RowState,
        change: &ChangeRecord,
    ) -> bool {
        // Get modified columns from change
        let changed_columns: std::collections::HashSet<_> =
            change.changes.as_object()
                .map(|o| o.keys().cloned().collect())
                .unwrap_or_default();

        // Get columns modified since the change's clock
        let server_modified = self.get_columns_modified_since(
            current,
            &change.clock,
        );

        // No overlap = can merge
        changed_columns.is_disjoint(&server_modified)
    }

    /// Merge at field level when possible
    fn merge_field_level(
        &self,
        current: &RowState,
        change: &ChangeRecord,
        server_clock: &mut ServerClock,
    ) -> ApplyResult {
        // Extract only non-conflicting fields
        let mut merged_changes = current.data.clone();

        if let (Some(merged_obj), Some(change_obj)) =
            (merged_changes.as_object_mut(), change.changes.as_object())
        {
            for (key, value) in change_obj {
                // Only merge if server hasn't modified this field
                if !self.server_modified_field(current, key) {
                    merged_obj.insert(key.clone(), value.clone());
                }
            }
        }

        // Apply merged changes
        // (Would execute UPDATE with merged_changes)

        server_clock.merge(&change.clock);

        ApplyResult::ConflictResolved(change.row_id.clone())
    }

    /// Resolve a true conflict
    fn resolve_conflict(
        &self,
        current: &RowState,
        change: &ChangeRecord,
        server_clock: &mut ServerClock,
    ) -> ApplyResult {
        // Default: Last-Write-Wins
        if change.timestamp >= current.timestamp {
            // Client wins
            ApplyResult::ConflictResolved(change.row_id.clone())
        } else {
            // Server wins - reject client change
            ApplyResult::Rejected(
                RejectionReason::ValidationFailed {
                    constraint: "conflict_lww".to_string(),
                    message: "Concurrent modification - server version is newer".to_string(),
                },
                Some(current.to_change_record()),
            )
        }
    }

    /// Get changes that client needs to catch up
    async fn get_changes_for_client(
        &self,
        client_id: &str,
        client_clock: &VectorClock,
        newly_accepted: &HashMap<String, Vec<String>>,
    ) -> Result<HashMap<String, Vec<ChangeRecord>>, SyncError> {
        // Query change log for changes client doesn't have
        let changes = self.db.get_changes_since_clock(client_clock, 100).await?;

        // Group by table
        let mut by_table: HashMap<String, Vec<ChangeRecord>> = HashMap::new();
        for change in changes {
            // Skip changes this client just uploaded
            if self.was_just_uploaded(newly_accepted, &change) {
                continue;
            }

            by_table
                .entry(change.table.clone())
                .or_insert_with(Vec::new)
                .push(change);
        }

        Ok(by_table)
    }

    fn was_just_uploaded(
        &self,
        accepted: &HashMap<String, Vec<String>>,
        change: &ChangeRecord,
    ) -> bool {
        accepted.get(&change.table)
            .map(|ids| ids.contains(&change.row_id))
            .unwrap_or(false)
    }

    fn validate_client_clock(&self, clock: &VectorClock) -> Result<(), SyncError> {
        // Ensure clock is well-formed
        // Could add authentication/authorization checks here
        Ok(())
    }
}

enum ApplyResult {
    Accepted(String),  // change_id
    Rejected(RejectionReason, Option<ChangeRecord>),
    ConflictResolved(String),
}

struct RowState {
    data: serde_json::Value,
    clock: Option<VectorClock>,
    timestamp: u64,
}

struct ServerClock {
    counters: HashMap<String, u64>,
}

impl ServerClock {
    fn new() -> Self {
        Self { counters: HashMap::new() }
    }

    fn merge(&mut self, clock: &VectorClock) {
        for actor in clock.actors() {
            let server_count = self.counters.entry(actor.clone()).or_insert(0);
            *server_count = (*server_count).max(clock.get(actor).unwrap_or(0));
        }
    }

    fn to_vector_clock(&self) -> VectorClock {
        VectorClock::from_counters(self.counters.clone())
    }
}
```

### WebSocket Transport Layer

```rust
use tokio::net::TcpListener;
use tokio_tungstenite::tungstenite::Message;
use futures_util::{SinkExt, StreamExt};

/// WebSocket sync endpoint
pub struct WebSocketSyncEndpoint {
    server: Arc<SyncServer>,
    addr: String,
}

impl WebSocketSyncEndpoint {
    pub fn new(server: Arc<SyncServer>, addr: String) -> Self {
        Self { server, addr }
    }

    /// Start the WebSocket server
    pub async fn run(self) -> Result<(), Box<dyn std::error::Error>> {
        let listener = TcpListener::bind(&self.addr).await?;
        println!("WebSocket sync server listening on {}", self.addr);

        while let Ok((stream, addr)) = listener.accept().await {
            let server = self.server.clone();

            tokio::spawn(async move {
                if let Err(e) = handle_connection(stream, server, addr).await {
                    eprintln!("Connection error: {}", e);
                }
            });
        }

        Ok(())
    }
}

async fn handle_connection(
    stream: tokio::net::TcpStream,
    server: Arc<SyncServer>,
    addr: std::net::SocketAddr,
) -> Result<(), Box<dyn std::error::Error>> {
    let ws_stream = tokio_tungstenite::accept_async(stream).await?;
    let (mut write, mut read) = ws_stream.split();

    // Authenticate client (extract client_id from handshake)
    let client_id = authenticate_client(&mut read).await?;

    // Register client connection
    server.clients.write().await.insert(
        client_id.clone(),
        ClientConnection { addr, connected_at: std::time::Instant::now() },
    );

    println!("Client {} connected from {}", client_id, addr);

    // Handle messages
    while let Some(msg) = read.next().await {
        let msg = msg?;

        match msg {
            Message::Text(text) => {
                // Parse sync request
                let request: SyncRequest = match serde_json::from_str(&text) {
                    Ok(r) => r,
                    Err(e) => {
                        let error_response = serde_json::json!({
                            "error": "invalid_request",
                            "message": e.to_string()
                        });
                        write.send(Message::Text(error_response.to_string())).await?;
                        continue;
                    }
                };

                // Process sync
                let response = server.process_sync(&client_id, request).await?;

                // Send response
                let response_text = serde_json::to_string(&response)?;
                write.send(Message::Text(response_text)).await?;
            }

            Message::Ping(data) => {
                write.send(Message::Pong(data)).await?;
            }

            Message::Close(_) => {
                break;
            }

            _ => {}
        }
    }

    // Unregister client
    server.clients.write().await.remove(&client_id);
    println!("Client {} disconnected", client_id);

    Ok(())
}

async fn authenticate_client(
    read: &mut futures_util::stream::SplitStream<tokio_tungstenite::WebSocketStream<tokio::net::TcpStream>>,
) -> Result<String, Box<dyn std::error::Error>> {
    // First message should be authentication
    if let Some(msg) = read.next().await {
        let msg = msg?;
        if let Message::Text(text) = msg {
            let auth: AuthMessage = serde_json::from_str(&text)?;
            // Validate token, extract client_id
            return Ok(auth.client_id);
        }
    }
    Err("Authentication required".into())
}

#[derive(Debug, serde::Deserialize)]
struct AuthMessage {
    #[serde(rename = "type")]
    message_type: String,
    client_id: String,
    token: String,
}

struct ClientConnection {
    addr: std::net::SocketAddr,
    connected_at: std::time::Instant,
}
```

## Part 5: Sync Optimization Techniques

### Batch Processing

```rust
/// Sync request with batching support
pub struct BatchedSyncProcessor {
    max_batch_size: usize,
    processing_delay_ms: u64,
}

impl BatchedSyncProcessor {
    pub fn new(max_batch_size: usize, processing_delay_ms: u64) -> Self {
        Self {
            max_batch_size,
            processing_delay_ms,
        }
    }

    /// Process changes in batches for efficiency
    pub async fn process_batch(
        &self,
        changes: Vec<ChangeRecord>,
    ) -> Vec<BatchResult> {
        let mut results = Vec::new();

        // Split into batches
        for batch in changes.chunks(self.max_batch_size) {
            let batch_result = self.process_single_batch(batch.to_vec()).await;
            results.push(batch_result);

            // Small delay between batches to avoid overwhelming DB
            tokio::time::sleep(
                std::time::Duration::from_millis(self.processing_delay_ms)
            ).await;
        }

        results
    }

    async fn process_single_batch(
        &self,
        batch: Vec<ChangeRecord>,
    ) -> BatchResult {
        let start = std::time::Instant::now();

        // Use a single transaction for the batch
        let tx = self.db.begin_transaction().await;

        let mut accepted = 0;
        let mut rejected = 0;

        for change in batch {
            match self.apply_change(&change).await {
                Ok(_) => accepted += 1,
                Err(_) => rejected += 1,
            }
        }

        tx.commit().await?;

        BatchResult {
            accepted,
            rejected,
            processing_time_ms: start.elapsed().as_millis() as u32,
        }
    }
}

#[derive(Debug)]
pub struct BatchResult {
    pub accepted: usize,
    pub rejected: usize,
    pub processing_time_ms: u32,
}
```

### Delta Sync (Only Send Changes)

```rust
/// Compute minimal sync delta
pub struct DeltaSyncCalculator {
    db: Arc<DatabaseConnection>,
}

impl DeltaSyncCalculator {
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }

    /// Calculate what client needs based on their clock
    pub async fn calculate_delta(
        &self,
        client_clock: &VectorClock,
        client_subscriptions: &[String],
    ) -> Result<SyncDelta, SyncError> {
        // Get all changes since client's clock
        let changes = self.db.get_changes_since_clock(
            client_clock,
            1000, // max batch size
        ).await?;

        // Filter to subscribed tables
        let relevant_changes: Vec<_> = changes
            .into_iter()
            .filter(|c| client_subscriptions.contains(&c.table))
            .collect();

        // Group by table
        let mut by_table: HashMap<String, Vec<ChangeRecord>> = HashMap::new();
        for change in relevant_changes {
            by_table
                .entry(change.table.clone())
                .or_insert_with(Vec::new)
                .push(change);
        }

        Ok(SyncDelta {
            tables: by_table,
            is_complete: relevant_changes.len() < 1000,
            next_clock: self.compute_max_clock(&by_table),
        })
    }

    fn compute_max_clock(&self, tables: &HashMap<String, Vec<ChangeRecord>>) -> VectorClock {
        let mut clock = VectorClock::new();
        for changes in tables.values() {
            for change in changes {
                clock.merge(&change.clock);
            }
        }
        clock
    }
}

#[derive(Debug)]
pub struct SyncDelta {
    pub tables: HashMap<String, Vec<ChangeRecord>>,
    pub is_complete: bool,  // false = more changes available
    pub next_clock: VectorClock,
}
```

### Compression

```rust
/// Compress sync messages
pub mod sync_compression {
    use flate2::compression::Gzip;
    use flate2::write::GzEncoder;
    use std::io::Write;

    /// Compress a sync response
    pub fn compress_response(response: &SyncResponse) -> Result<Vec<u8>, CompressionError> {
        let json = serde_json::to_vec(response)?;

        let mut encoder = GzEncoder::new(Vec::new(), flate2::Compression::default());
        encoder.write_all(&json)?;
        let compressed = encoder.finish()?;

        // Check if compression helped
        if compressed.len() >= json.len() {
            // Return uncompressed with marker
            let mut result = vec![0u8];  // 0 = uncompressed
            result.extend(json);
            Ok(result)
        } else {
            let mut result = vec![1u8];  // 1 = gzip compressed
            result.extend(compressed);
            Ok(result)
        }
    }

    /// Decompress a sync request
    pub fn decompress_request(data: &[u8]) -> Result<SyncRequest, CompressionError> {
        if data.is_empty() {
            return Err(CompressionError::Empty);
        }

        match data[0] {
            0 => {
                // Uncompressed
                Ok(serde_json::from_slice(&data[1..])?)
            }
            1 => {
                // Gzip compressed
                let mut decoder = flate2::read::GzDecoder::new(&data[1..]);
                let mut json = Vec::new();
                decoder.read_to_end(&mut json)?;
                Ok(serde_json::from_slice(&json)?)
            }
            _ => Err(CompressionError::UnknownFormat),
        }
    }

    #[derive(Debug, thiserror::Error)]
    pub enum CompressionError {
        #[error("Empty data")]
        Empty,
        #[error("Unknown format")]
        UnknownFormat,
        #[error("IO error: {0}")]
        Io(#[from] std::io::Error),
        #[error("JSON error: {0}")]
        Json(#[from] serde_json::Error),
    }
}
```

---

*This document is part of the OrbitingHail exploration series. See [exploration.md](./exploration.md) for the complete index.*
