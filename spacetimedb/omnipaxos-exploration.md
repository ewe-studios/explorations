---
name: Omnipaxos
description: Consensus protocol implementation for distributed coordination and replication
type: sub-project
source: /home/darkvoid/Boxxed/@formulas/src.rust/src.SpacetimeDB/omnipaxos/
---

# OmniPaxos - Distributed Consensus Protocol

## Overview

OmniPaxos is a **distributed consensus protocol** implementation designed for high-throughput, low-latency replicated systems. It extends the classic Paxos algorithm with optimizations for modern distributed systems, particularly suited for:

- **State machine replication** across multiple nodes
- **Leader election** with fast failure detection
- **Log replication** with efficient batching
- **Dynamic membership** (nodes can join/leave)
- **Horizontal scaling** with sharding support

The implementation consists of multiple crates:
- `omnipaxos` - Core consensus protocol
- `omnipaxos_storage` - Persistent storage layer
- `omnipaxos_macros` - Procedural macros for simplifying implementation
- `omnipaxos_ui` - Visualization and debugging tools

## Directory Structure

```
omnipaxos/
├── .github/                         # CI/CD workflows
├── omnipaxos/                       # Core protocol implementation
│   ├── src/
│   │   ├── ballot_leader_election.rs # BLE: Ballot Leader Election
│   │   ├── omnipaxos.rs             # Main protocol state machine
│   │   ├── messages.rs              # Protocol message types
│   │   ├── storage.rs               # Storage trait definitions
│   │   ├── error.rs                 # Error types
│   │   ├── util.rs                  # Utilities and helpers
│   │   └── lib.rs                   # Crate root
│   ├── Cargo.toml
│   └── README.md
├── omnipaxos_macros/                # Procedural macros
│   ├── src/
│   │   └── lib.rs                   # Macro definitions
│   ├── Cargo.toml
│   └── README.md
├── omnipaxos_storage/               # Storage implementations
│   ├── src/
│   │   ├── memory.rs                # In-memory storage (testing)
│   │   ├── persistent.rs            # Disk-based storage
│   │   └── lib.rs
│   ├── Cargo.toml
│   └── README.md
├── omnipaxos_ui/                    # Visualization tools
│   ├── src/
│   │   └── lib.rs                   # UI components
│   ├── Cargo.toml
│   └── README.md
├── client-unity/                    # Unity client integration
├── server-csharp/                   # C# server bindings
├── server-rust/                     # Rust server implementation
├── DEVELOP.md                       # Development guide
├── README.md                        # Project overview
├── overview.png                     # Architecture diagram
└── crates-checklist.md              # Crate documentation
```

## Consensus Protocol Architecture

### High-Level Flow

```
┌─────────────────────────────────────────────────────────────────┐
│                    OmniPaxos Cluster                            │
│                                                                 │
│  ┌─────────┐     ┌─────────┐     ┌─────────┐                  │
│  │ Node 1  │◄───►│ Node 2  │◄───►│ Node 3  │                  │
│  │ Leader  │     │Follower │     │Follower │                  │
│  └────┬────┘     └────┬────┘     └────┬────┘                  │
│       │               │               │                         │
│       └───────────────┼───────────────┘                         │
│                       │                                         │
│              Consensus Messages                                 │
│              - Prepare                                          │
│              - Promise                                          │
│              - Accept                                           │
│              - Accepted                                         │
│              - Decided                                          │
└─────────────────────────────────────────────────────────────────┘
```

### Protocol Phases

OmniPaxos follows a multi-phase consensus protocol:

```
┌──────────────────────────────────────────────────────────────────┐
│ Phase 1: Leader Election (Ballot Leader Election - BLE)         │
├──────────────────────────────────────────────────────────────────┤
│                                                                  │
│  Node A                          Node B                          │
│     │                              │                             │
│     │───── Prepare(ballot=1) ────► │                             │
│     │                              │ (Compare with current)      │
│     │◄──── Promise(ballot=1) ─────│                             │
│     │                              │                             │
│     │───► Become Leader ◄─────────│                             │
│                                                                  │
├──────────────────────────────────────────────────────────────────┤
│ Phase 2: Log Replication                                        │
├──────────────────────────────────────────────────────────────────┤
│                                                                  │
│  Leader                          Follower                        │
│     │                              │                             │
│     │───── Accept(entry) ────────► │                             │
│     │                              │ (Validate & Append)         │
│     │◄──── Accepted(entry) ───────│                             │
│     │                              │                             │
│     │───── Decided(entry) ───────► │ (Commit)                    │
│                                                                  │
└──────────────────────────────────────────────────────────────────┘
```

## Core Components

### Ballot Leader Election (BLE)

```rust
// From omnipaxos/src/ballot_leader_election.rs
pub struct BallotLeaderElection<T> {
    /// Current ballot number (monotonically increasing)
    ballot: Ballot,

    /// Node's unique identifier
    nid: NodeId,

    /// Known nodes in the cluster
    nodes: HashSet<NodeId>,

    /// Leader state
    leader: Option<NodeId>,

    /// Pending promises
    promises: HashMap<NodeId, Ballot>,

    /// Event callback for leader changes
    event_callback: T,
}

impl<T: LeaderEvent> BallotLeaderElection<T> {
    /// Start leader election
    pub fn elect_leader(&mut self) {
        self.ballot = self.ballot.next();

        // Send Prepare to all nodes
        let msg = Message::Prepare(Prepare {
            ballot: self.ballot,
            from: self.nid,
        });

        for node in &self.nodes {
            self.send(*node, msg.clone());
        }
    }

    /// Handle received Prepare message
    pub fn on_prepare(&mut self, from: NodeId, prepare: Prepare) {
        if prepare.ballot > self.ballot {
            self.ballot = prepare.ballot;
            self.leader = None;

            // Send Promise
            self.send(from, Message::Promise(Promise {
                ballot: prepare.ballot,
                from: self.nid,
            }));
        }
    }

    /// Handle received Promise messages
    pub fn on_promise(&mut self, from: NodeId, promise: Promise) {
        self.promises.insert(from, promise.ballot);

        // Check if we have a quorum
        if self.promises.len() >= self.quorum_size() {
            self.become_leader();
        }
    }

    fn become_leader(&mut self) {
        self.leader = Some(self.nid);
        self.event_callback.on_leader_elected(self.nid);

        // Send heartbeat to establish authority
        self.send_heartbeat();
    }
}
```

### Log Replication

```rust
// From omnipaxos/src/omnipaxos.rs
pub struct Omnipaxos<S: Storage> {
    /// Node configuration
    config: OmniPaxosConfig,

    /// Current leader state
    leader_state: Option<LeaderState>,

    /// Follower state
    follower_state: Option<FollowerState>,

    /// Persistent storage
    storage: S,

    /// Log entries (in-memory cache)
    log: Vec<LogEntry>,

    /// Commit index
    commit_index: LogIndex,
}

impl<S: Storage> Omnipaxos<S> {
    /// Leader accepts a new entry
    pub fn leader_accept(&mut self, entry: LogEntry) -> Result<()> {
        let state = self.leader_state.as_mut().ok_or(Error::NotLeader)?;

        // Append to local log
        let idx = self.log.len() as LogIndex;
        self.log.push(entry.clone());

        // Send Accept to followers
        let msg = Message::Accept(Accept {
            ballot: state.ballot,
            entry: entry.clone(),
            index: idx,
        });

        for follower in &state.followers {
            self.send(*follower, msg.clone());
        }

        // Track pending accepts
        state.pending_accepts.insert(idx, PendingAccept {
            entry,
            accepts: 0,
        });

        Ok(())
    }

    /// Follower handles Accept message
    pub fn follower_on_accept(&mut self, accept: Accept) -> Result<()> {
        // Validate ballot
        if accept.ballot < self.current_ballot() {
            return Err(Error::StaleBallot);
        }

        // Validate index
        if accept.index != self.log.len() as LogIndex {
            return Err(Error::IndexMismatch);
        }

        // Append to local log
        self.storage.append(&accept.entry)?;
        self.log.push(accept.entry.clone());

        // Send Accepted acknowledgment
        self.send(accept.from, Message::Accepted(Accepted {
            ballot: accept.ballot,
            index: accept.index,
        }));

        Ok(())
    }

    /// Leader handles Accepted message
    pub fn leader_on_accepted(&mut self, from: NodeId, accepted: Accepted) {
        let state = self.leader_state.as_mut().unwrap();

        if let Some(pending) = state.pending_accepts.get_mut(&accepted.index) {
            pending.accepts += 1;

            // Check if we have a quorum
            if pending.accepts >= self.quorum_size() - 1 {
                // Entry is committed
                self.commit(accepted.index);

                // Notify followers
                self.send_decided(accepted.index);
            }
        }
    }

    /// Commit an entry
    fn commit(&mut self, index: LogIndex) {
        self.commit_index = index;
        self.storage.set_commit_index(index)?;

        // Apply entry to state machine
        let entry = &self.log[index as usize];
        self.state_machine.apply(entry);
    }
}
```

### Message Types

```rust
// From omnipaxos/src/messages.rs
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum Message {
    /// Phase 1a: Leader election preparation
    Prepare(Prepare),

    /// Phase 1b: Follower promises
    Promise(Promise),

    /// Phase 2a: Leader proposes entry
    Accept(Accept),

    /// Phase 2b: Follower acknowledges
    Accepted(Accepted),

    /// Phase 3: Entry is decided/committed
    Decided(Decided),

    /// Heartbeat to maintain leadership
    Heartbeat(Heartbeat),

    /// Catch-up request (new node or partition recovery)
    CatchUpRequest(CatchUpRequest),

    /// Catch-up response
    CatchUpResponse(CatchUpResponse),
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Prepare {
    pub ballot: Ballot,
    pub from: NodeId,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Promise {
    pub ballot: Ballot,
    pub from: NodeId,
    pub last_accepted: Option<(Ballot, LogIndex)>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Accept {
    pub ballot: Ballot,
    pub entry: LogEntry,
    pub index: LogIndex,
    pub from: NodeId,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Accepted {
    pub ballot: Ballot,
    pub index: LogIndex,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Decided {
    pub index: LogIndex,
    pub entry: LogEntry,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Heartbeat {
    pub from: NodeId,
    pub ballot: Ballot,
    pub commit_index: LogIndex,
}
```

## Storage Layer

### Storage Trait

```rust
// From omnipaxos/src/storage.rs
pub trait Storage: Send + Sync {
    type Error: std::error::Error;

    /// Append entry to log
    fn append(&mut self, entry: &LogEntry) -> Result<(), Self::Error>;

    /// Get entry at index
    fn get(&self, index: LogIndex) -> Option<&LogEntry>;

    /// Get range of entries
    fn get_range(&self, start: LogIndex, end: LogIndex) -> Vec<LogEntry>;

    /// Set commit index
    fn set_commit_index(&mut self, index: LogIndex) -> Result<(), Self::Error>;

    /// Get commit index
    fn get_commit_index(&self) -> LogIndex;

    /// Truncate log to length
    fn truncate(&mut self, len: LogIndex) -> Result<(), Self::Error>;

    /// Get last log index
    fn last_log_index(&self) -> LogIndex;
}
```

### Memory Storage (Testing)

```rust
// From omnipaxos_storage/src/memory.rs
pub struct MemoryStorage {
    log: Vec<LogEntry>,
    commit_index: LogIndex,
}

impl Storage for MemoryStorage {
    type Error = std::convert::Infallible;

    fn append(&mut self, entry: &LogEntry) -> Result<(), Self::Error> {
        self.log.push(entry.clone());
        Ok(())
    }

    fn get(&self, index: LogIndex) -> Option<&LogEntry> {
        self.log.get(index as usize)
    }

    fn get_range(&self, start: LogIndex, end: LogIndex) -> Vec<LogEntry> {
        self.log[start as usize..end as usize].to_vec()
    }

    fn set_commit_index(&mut self, index: LogIndex) -> Result<(), Self::Error> {
        self.commit_index = index;
        Ok(())
    }

    fn get_commit_index(&self) -> LogIndex {
        self.commit_index
    }

    fn truncate(&mut self, len: LogIndex) -> Result<(), Self::Error> {
        self.log.truncate(len as usize);
        Ok(())
    }

    fn last_log_index(&self) -> LogIndex {
        self.log.len() as LogIndex
    }
}
```

### Persistent Storage

```rust
// From omnipaxos_storage/src/persistent.rs
pub struct PersistentStorage {
    /// Log file handle
    log_file: File,

    /// Index file for fast lookups
    index: BTreeMap<LogIndex, u64>, // index -> file offset

    /// Commit index
    commit_index: LogIndex,

    /// Buffer for batched writes
    write_buffer: Vec<(LogEntry, LogIndex)>,
}

impl Storage for PersistentStorage {
    type Error = StorageError;

    fn append(&mut self, entry: &LogEntry) -> Result<(), StorageError> {
        let offset = self.log_file.metadata()?.len();
        let index = self.index.len() as LogIndex;

        // Serialize entry
        let bytes = bincode::serialize(entry)?;

        // Write length prefix + data
        self.log_file.write_all(&(bytes.len() as u32).to_le_bytes())?;
        self.log_file.write_all(&bytes)?;
        self.log_file.sync_all()?;

        // Update index
        self.index.insert(index, offset);

        Ok(())
    }

    fn get(&self, index: LogIndex) -> Option<&LogEntry> {
        let offset = *self.index.get(&index)?;

        // Seek to offset
        let mut file = File::open(&self.log_path).ok()?;
        file.seek(SeekFrom::Start(offset)).ok()?;

        // Read length
        let mut len_bytes = [0u8; 4];
        file.read_exact(&mut len_bytes).ok()?;
        let len = u32::from_le_bytes(len_bytes) as usize;

        // Read data
        let mut bytes = vec![0u8; len];
        file.read_exact(&mut bytes).ok()?;

        bincode::deserialize(&bytes).ok()
    }
}
```

## Procedural Macros

```rust
// From omnipaxos_macros/src/lib.rs
use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, DeriveInput};

/// Derive macro for generating message serialization
#[proc_macro_derive(Message)]
pub fn derive_message(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let name = &input.ident;

    let expanded = quote! {
        impl Message for #name {
            fn serialize(&self) -> Vec<u8> {
                bincode::serialize(self).unwrap()
            }

            fn deserialize(bytes: &[u8]) -> Option<Self> {
                bincode::deserialize(bytes).ok()
            }
        }
    };

    TokenStream::from(expanded)
}

/// Macro for generating state machine handlers
#[proc_macro]
pub fn state_machine(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as syn::ItemImpl);
    // Generate handler boilerplate
    // ...
}
```

## Network Layer

### Message Transport

```rust
pub struct MessageTransport {
    /// Outgoing message queue
    tx_queue: VecDeque<(NodeId, Message)>,

    /// Incoming message channel
    rx_channel: mpsc::Receiver<(NodeId, Message)>,

    /// Connection pool
    connections: HashMap<NodeId, TcpStream>,
}

impl MessageTransport {
    /// Send message to node
    pub fn send(&mut self, to: NodeId, msg: Message) {
        self.tx_queue.push_back((to, msg));
        self.flush_queue();
    }

    /// Flush outgoing queue
    fn flush_queue(&mut self) {
        while let Some((to, msg)) = self.tx_queue.pop_front() {
            if let Some(conn) = self.connections.get_mut(&to) {
                let bytes = msg.serialize();
                let _ = conn.write_all(&bytes);
            }
        }
    }

    /// Receive next message
    pub fn recv(&mut self) -> Option<(NodeId, Message)> {
        self.rx_channel.try_recv().ok()
    }
}
```

### Connection Management

```rust
pub struct ConnectionManager {
    /// Listen address
    listen_addr: SocketAddr,

    /// Peer connections
    peers: HashMap<NodeId, PeerConnection>,

    /// Reconnection backoff
    backoff: HashMap<NodeId, Duration>,
}

impl ConnectionManager {
    /// Connect to peer
    pub async fn connect(&mut self, node_id: NodeId, addr: SocketAddr) {
        match TcpStream::connect(addr).await {
            Ok(stream) => {
                let conn = PeerConnection::new(stream);
                self.peers.insert(node_id, conn);
                self.backoff.remove(&node_id);
            }
            Err(e) => {
                // Exponential backoff
                let backoff = self.backoff.entry(node_id)
                    .or_insert(Duration::from_millis(100));
                *backoff *= 2;

                // Schedule retry
                self.schedule_retry(node_id, addr, *backoff);
            }
        }
    }
}
```

## Failure Handling

### Leader Failure Detection

```rust
pub struct FailureDetector {
    /// Last heartbeat from leader
    last_heartbeat: Option<Instant>,

    /// Timeout duration
    timeout: Duration,

    /// Current leader
    leader: Option<NodeId>,
}

impl FailureDetector {
    /// Check if leader has failed
    pub fn check_leader_failure(&self) -> bool {
        if let Some(last) = self.last_heartbeat {
            last.elapsed() > self.timeout
        } else {
            true // No leader known
        }
    }

    /// Record heartbeat from leader
    pub fn on_heartbeat(&mut self, from: NodeId) {
        self.last_heartbeat = Some(Instant::now());
        self.leader = Some(from);
    }
}
```

### Network Partition Handling

```rust
impl<S: Storage> Omnipaxos<S> {
    /// Handle suspected network partition
    pub fn on_partition_detected(&mut self) {
        // Step down if we were leader
        if self.leader_state.is_some() {
            self.leader_state = None;
            self.follower_state = Some(FollowerState::new());
        }

        // Clear pending operations
        self.pending_ops.clear();

        // Request catch-up from any reachable node
        self.request_catch_up();
    }

    /// Catch up after partition heals
    pub fn request_catch_up(&mut self) {
        for node in &self.config.nodes {
            if *node != self.config.node_id {
                self.send(*node, Message::CatchUpRequest(CatchUpRequest {
                    from: self.config.node_id,
                    my_commit_index: self.commit_index,
                }));
            }
        }
    }

    /// Handle catch-up response
    pub fn on_catch_up_response(&mut self, response: CatchUpResponse) {
        // Truncate log to match response base
        self.log.truncate(response.base_index as usize);

        // Append missing entries
        for entry in response.entries {
            self.log.push(entry);
        }

        // Update commit index
        self.commit_index = response.commit_index;
    }
}
```

## Performance Optimizations

### Batch Processing

```rust
pub struct BatchProcessor {
    /// Pending entries to propose
    pending: Vec<LogEntry>,

    /// Maximum batch size
    max_batch_size: usize,

    /// Batch timeout
    timeout: Duration,
}

impl BatchProcessor {
    /// Add entry to batch
    pub fn add(&mut self, entry: LogEntry) {
        self.pending.push(entry);

        if self.pending.len() >= self.max_batch_size {
            self.flush();
        }
    }

    /// Flush batch as single proposal
    fn flush(&mut self) {
        let batch = std::mem::take(&mut self.pending);

        // Propose entire batch as single entry
        self.omnipaxos.leader_accept(LogEntry::Batch(batch));
    }
}
```

### Log Compaction

```rust
pub struct LogCompactor<S: Storage> {
    storage: S,
    /// Index to compact up to
    compact_until: LogIndex,
    /// Snapshot at compact index
    snapshot: Option<Snapshot>,
}

impl<S: Storage> LogCompactor<S> {
    /// Compact log up to commit index
    pub fn compact(&mut self) -> Result<(), StorageError> {
        let commit_index = self.storage.get_commit_index();

        if commit_index > self.compact_until {
            // Take snapshot of state
            let snapshot = self.take_snapshot(commit_index)?;

            // Truncate log
            self.storage.truncate(commit_index + 1)?;

            // Store snapshot
            self.snapshot = Some(snapshot);
            self.compact_until = commit_index;
        }

        Ok(())
    }

    /// Restore from snapshot
    pub fn restore(&mut self) -> Result<(), StorageError> {
        if let Some(snapshot) = &self.snapshot {
            self.storage.restore_snapshot(snapshot)?;
        }
        Ok(())
    }
}
```

## Integration with SpacetimeDB

OmniPaxos is used by SpacetimeDB for:

1. **Module Replication**: Ensuring all nodes in a cluster have consistent module state
2. **Leader Election**: Selecting the primary node for writes
3. **Consensus on Transactions**: Ordering transactions across replicas

```rust
// SpacetimeDB integration
pub struct SpacetimeDBCluster {
    omnipaxos: Omnipaxos<PersistentStorage>,
    module_state: ModuleState,
}

impl SpacetimeDBCluster {
    /// Apply a reducer transaction
    pub fn apply_reducer(&mut self, reducer: ReducerCall) -> Result<()> {
        // Go through consensus
        let entry = LogEntry::Transaction(Transaction {
            reducer,
            timestamp: Instant::now(),
        });

        self.omnipaxos.leader_accept(entry)?;

        Ok(())
    }

    /// On entry committed
    fn on_committed(&mut self, entry: LogEntry) {
        match entry {
            LogEntry::Transaction(tx) => {
                // Apply to module state
                self.module_state.apply_reducer(tx.reducer);
            }
            LogEntry::Batch(batch) => {
                for tx in batch {
                    self.module_state.apply_reducer(tx.reducer);
                }
            }
        }
    }
}
```

## Comparison with Other Consensus Protocols

| Protocol | Leader Election | Log Replication | Membership | Use Case |
|----------|-----------------|-----------------|------------|----------|
| Raft | Heartbeat-based | Single leader | Joint consensus | General purpose |
| Multi-Paxos | Implicit | Leader-based | Static | High throughput |
| OmniPaxos | BLE | Batched | Dynamic | SpacetimeDB |
| Zab | Epoch-based | Zookeeper order | Dynamic | ZooKeeper |

## Sources

- Source: `/home/darkvoid/Boxxed/@formulas/src.rust/src.SpacetimeDB/omnipaxos/`
- OmniPaxos Paper: https://arxiv.org/abs/2302.08992
- SpacetimeDB Documentation: https://spacetimedb.com/docs
