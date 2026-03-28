# SpacetimeDB: Distributed Consensus and Replication Deep Dive

## Overview

This document explores distributed systems aspects of SpacetimeDB:
- Replication architecture
- Consensus protocols
- Leader election
- Conflict resolution
- Multi-node deployment

---

## 1. Replication Architecture

### 1.1 Primary-Replica Model

```
SpacetimeDB Replication:
┌───────────────────────────────────────────────────────┐
│                    Leader Node                         │
│  - Accepts all writes                                  │
│  - Orders transactions                                 │
│  - Replicates to followers                            │
│  - Responds to clients                                │
└───────────────────────────────────────────────────────┘
              │
              │ Replication stream
              ▼
┌───────────────────────────────────────────────────────┐
│                  Follower Nodes                        │
│  - Receive replicated transactions                    │
│  - Apply to local state                              │
│  - Can serve read-only queries                       │
│  - Ready to become leader if needed                  │
└───────────────────────────────────────────────────────┘
```

### 1.2 Replication Log

```rust
/// Replication log entry
struct ReplicationEntry {
    /// Logical timestamp (Lamport clock)
    timestamp: u64,

    /// Transaction ID
    txid: u64,

    /// Transaction type
    tx_type: TransactionType,

    /// Serialized transaction data
    payload: Vec<u8>,

    /// Checksum for verification
    checksum: u32,
}

enum TransactionType {
    /// Reducer call (user-defined transaction)
    Reducer {
        name: String,
        args: Vec<DbValue>,
        caller: Identity,
    },

    /// System operation
    System {
        op: SystemOp,
    },
}
```

### 1.3 Replication Protocol

```rust
struct ReplicationManager {
    /// Node's role in cluster
    role: Role,

    /// Current term (for leader election)
    current_term: u64,

    /// Known cluster members
    members: HashSet<NodeId>,

    /// Replication log
    log: ReplicationLog,

    /// Pending unacknowledged entries
    pending: HashMap<LogIndex, HashSet<NodeId>>,
}

enum Role {
    /// Leader - accepts writes, replicates
    Leader {
        /// Next log index for each follower
        next_index: HashMap<NodeId, LogIndex>,

        /// Match index for each follower
        match_index: HashMap<NodeId, LogIndex>,
    },

    /// Follower - receives replication
    Follower {
        /// Current leader
        leader: NodeId,

        /// Last heartbeat from leader
        last_heartbeat: Instant,
    },

    /// Candidate - running election
    Candidate {
        /// Votes received
        votes: HashSet<NodeId>,
    },
}

impl ReplicationManager {
    /// Leader: Append new entry and replicate
    fn append_and_replicate(&mut self, entry: ReplicationEntry) -> Result<()> {
        if !matches!(self.role, Role::Leader { .. }) {
            return Err(NotLeader);
        }

        // Append to local log
        let index = self.log.append(&entry);

        // Send to all followers
        if let Role::Leader { next_index, .. } = &mut self.role {
            for follower in &self.members {
                if *follower != self.self_id() {
                    self.send_append_entries(*follower, *next_index.get(follower).unwrap());
                }
            }
        }

        Ok(())
    }

    /// Follower: Handle append entries from leader
    fn handle_append_entries(&mut self, request: AppendEntriesRequest) -> AppendEntriesResponse {
        // Verify leader
        if request.term < self.current_term {
            return AppendEntriesResponse {
                success: false,
                term: self.current_term,
            };
        }

        // Update current leader
        self.current_term = request.term;
        if let Role::Follower { leader, last_heartbeat } = &mut self.role {
            *leader = request.leader_id;
            *last_heartbeat = Instant::now();
        }

        // Append entries to log
        for entry in &request.entries {
            self.log.append(entry);
        }

        AppendEntriesResponse {
            success: true,
            term: self.current_term,
        }
    }
}
```

---

## 2. Raft Consensus

### 2.1 Raft Fundamentals

```
Raft State Machine:
┌───────────────────────────────────────────────────────┐
│                    Raft Node                           │
│                                                       │
│  ┌──────────────┐  ┌──────────────┐  ┌────────────┐ │
│  │ State        │  │ Log          │  │ Commit     │ │
│  │ Machine      │  │ Replication  │  │ Index      │ │
│  │              │  │              │  │            │ │
│  │ - Current    │  │ - Log entries│  │ - Highest  │ │
│  │   state      │  │ - Indices    │  │   committed│
│  └──────────────┘  └──────────────┘  └────────────┘ │
└───────────────────────────────────────────────────────┘

Leader Election:
- Followers elect leader
- Leader sends heartbeats
- If no heartbeat, new election
```

### 2.2 Raft Implementation

```rust
/// Raft consensus state
struct RaftState {
    /// Persistent state
    current_term: u64,
    voted_for: Option<NodeId>,
    log: Vec<LogEntry>,

    /// Volatile state
    commit_index: u64,
    last_applied: u64,

    /// Leader state
    next_index: HashMap<NodeId, u64>,
    match_index: HashMap<NodeId, u64>,
}

struct LogEntry {
    term: u64,
    data: Vec<u8>,
}

impl RaftState {
    /// Start leader election
    fn start_election(&mut self) -> ElectionResult {
        self.current_term += 1;
        self.voted_for = Some(self.self_id());

        // Request votes from all nodes
        let vote_request = RequestVoteRequest {
            term: self.current_term,
            candidate_id: self.self_id(),
            last_log_index: self.log.len() as u64,
            last_log_term: self.log.last().map(|e| e.term).unwrap_or(0),
        };

        ElectionResult::InProgress {
            votes: 1,  // Vote for self
            needed: (self.num_nodes() / 2) + 1,
        }
    }

    /// Handle vote request
    fn handle_vote_request(&mut self, request: RequestVoteRequest) -> RequestVoteResponse {
        // Reject if stale term
        if request.term < self.current_term {
            return RequestVoteResponse {
                vote_granted: false,
                term: self.current_term,
            };
        }

        // Update term if needed
        if request.term > self.current_term {
            self.current_term = request.term;
            self.voted_for = None;
        }

        // Grant vote if haven't voted and candidate's log is up-to-date
        let log_ok = request.last_log_term > self.last_log_term() ||
            (request.last_log_term == self.last_log_term() &&
             request.last_log_index >= self.last_log_index());

        if self.voted_for.is_none() && log_ok {
            self.voted_for = Some(request.candidate_id);
            RequestVoteResponse {
                vote_granted: true,
                term: self.current_term,
            }
        } else {
            RequestVoteResponse {
                vote_granted: false,
                term: self.current_term,
            }
        }
    }

    /// Commit entries when majority replicated
    fn update_commit_index(&mut self) {
        if let Role::Leader { .. } = self.role {
            // Find highest index replicated on majority
            for n in (self.commit_index + 1..=self.log.len() as u64).rev() {
                if self.match_index.values().filter(|&&m| m >= n).count() > self.num_nodes() / 2 {
                    self.commit_index = n;
                    break;
                }
            }
        }
    }
}
```

### 2.3 Log Replication

```rust
impl RaftState {
    /// Send append entries to follower
    fn send_append_entries(&self, follower: NodeId) {
        let next_idx = self.next_index[&follower];

        let prev_log_index = next_idx - 1;
        let prev_log_term = if prev_log_index > 0 {
            self.log[prev_log_index as usize].term
        } else {
            0
        };

        let entries = if next_idx <= self.log.len() as u64 {
            &self.log[next_idx as usize..]
        } else {
            &[]
        };

        let request = AppendEntriesRequest {
            term: self.current_term,
            leader_id: self.self_id(),
            prev_log_index,
            prev_log_term,
            entries: entries.to_vec(),
            leader_commit: self.commit_index,
        };

        self.send_rpc(follower, request);
    }

    /// Handle append entries response
    fn handle_append_entries_response(
        &mut self,
        follower: NodeId,
        response: AppendEntriesResponse,
    ) {
        if let Role::Leader { next_index, match_index } = &mut self.role {
            if response.success {
                // Update progress
                next_index.insert(follower, response.last_index + 1);
                match_index.insert(follower, response.last_index);

                // Check if can commit
                self.update_commit_index();
            } else {
                // Decrement next_index and retry
                if let Some(idx) = next_index.get_mut(&follower) {
                    *idx = idx.saturating_sub(1);
                }
                self.send_append_entries(follower);
            }
        }
    }
}
```

---

## 3. Leader Election

### 3.1 Election Timeout

```rust
struct ElectionManager {
    /// Election timeout (randomized)
    election_timeout: Duration,

    /// Last heartbeat received
    last_heartbeat: Instant,

    /// Current election state
    state: ElectionState,
}

enum ElectionState {
    Follower { leader: NodeId },
    Candidate,
    Leader,
}

impl ElectionManager {
    /// Check if election should start
    fn check_election(&mut self) -> Option<ElectionTrigger> {
        if self.last_heartbeat.elapsed() > self.election_timeout {
            Some(ElectionTrigger::Timeout)
        } else {
            None
        }
    }

    /// Randomize election timeout (prevents split votes)
    fn randomize_timeout(&mut self) {
        // Raft uses randomized timeouts: 150-300ms typically
        let base = Duration::from_millis(150);
        let random = Duration::from_millis(rand::random::<u64>() % 150);
        self.election_timeout = base + random;
    }
}
```

### 3.2 Split Vote Handling

```rust
impl RaftState {
    /// Handle election with split vote
    fn handle_split_vote(&mut self) {
        // If no majority, start new election with new timeout
        self.randomize_timeout();

        // Wait for timeout before trying again
        sleep(self.election_timeout);

        // Start new election with higher term
        self.start_election();
    }

    /// Campaign for leadership
    fn campaign(&mut self) -> CampaignResult {
        self.current_term += 1;
        self.role = Role::Candidate {
            votes: HashSet::new(),
        };

        // Vote for self
        if let Role::Candidate { votes } = &mut self.role {
            votes.insert(self.self_id());
        }

        // Request votes from all nodes
        let request = RequestVoteRequest {
            term: self.current_term,
            candidate_id: self.self_id(),
            last_log_index: self.log.len() as u64,
            last_log_term: self.log.last().map(|e| e.term).unwrap_or(0),
        };

        for node in &self.members {
            if *node != self.self_id() {
                self.send_vote_request(*node, request.clone());
            }
        }

        CampaignResult::InProgress
    }

    /// Collect vote
    fn collect_vote(&mut self, voter: NodeId, term: u64, granted: bool) {
        if term != self.current_term {
            return;  // Stale response
        }

        if !granted {
            return;  // Vote denied
        }

        if let Role::Candidate { votes } = &mut self.role {
            votes.insert(voter);

            // Check for majority
            if votes.len() > self.num_nodes() / 2 {
                self.become_leader();
            }
        }
    }

    /// Transition to leader
    fn become_leader(&mut self) {
        self.role = Role::Leader {
            next_index: self.members.iter()
                .map(|&n| (n, self.log.len() as u64 + 1))
                .collect(),
            match_index: self.members.iter()
                .map(|&n| (n, 0))
                .collect(),
        };

        // Send initial heartbeats
        for node in &self.members {
            if *node != self.self_id() {
                self.send_append_entries(*node);
            }
        }
    }
}
```

---

## 4. Conflict Resolution

### 4.1 Log Consistency

```rust
impl RaftState {
    /// Ensure log consistency with leader
    fn reconcile_log(&mut self, leader_entries: &[LogEntry], prev_idx: u64, prev_term: u64) -> bool {
        // Check prev log entry matches
        if prev_idx > 0 {
            if prev_idx > self.log.len() as u64 {
                return false;  // Missing entries
            }
            if self.log[prev_idx as usize].term != prev_term {
                // Mismatch - truncate
                self.log.truncate(prev_idx as usize);
                return false;
            }
        }

        // Append new entries
        for (i, entry) in leader_entries.iter().enumerate() {
            let idx = (prev_idx as usize + 1 + i) as usize;
            if idx >= self.log.len() {
                self.log.push(entry.clone());
            } else if self.log[idx].term != entry.term {
                // Conflict - replace and truncate followers
                self.log[idx] = entry.clone();
                self.log.truncate(idx + 1);
            }
        }

        true
    }
}
```

### 4.2 Network Partition Handling

```rust
/// Partition detector
struct PartitionDetector {
    /// Last successful communication with each node
    last_seen: HashMap<NodeId, Instant>,

    /// Known partition state
    partition_state: PartitionState,
}

enum PartitionState {
    /// No partition detected
    Healthy,

    /// Suspected partition
    Suspected {
        unreachable_nodes: HashSet<NodeId>,
    },

    /// Confirmed partition
    Partitioned {
        minority_nodes: HashSet<NodeId>,
    },
}

impl PartitionDetector {
    /// Check for partitions
    fn check(&mut self) -> PartitionState {
        let now = Instant::now();
        let unreachable: HashSet<_> = self.last_seen
            .iter()
            .filter(|(_, &last)| now - last > Duration::from_secs(10))
            .map(|(&node, _)| node)
            .collect();

        if unreachable.is_empty() {
            self.partition_state = PartitionState::Healthy;
        } else if unreachable.len() > self.num_nodes() / 2 {
            // We're in minority
            self.partition_state = PartitionState::Partitioned {
                minority_nodes: unreachable,
            };
        } else {
            self.partition_state = PartitionState::Suspected {
                unreachable_nodes: unreachable,
            };
        }

        self.partition_state.clone()
    }
}
```

---

## 5. SpacetimeDB Replication

### 5.1 Leader-Based Replication

```rust
/// SpacetimeDB cluster node
struct SpacetimeNode {
    /// Node identity
    identity: Identity,

    /// Database engine
    engine: DatabaseEngine,

    /// Replication manager
    replication: ReplicationManager,

    /// Client connections
    clients: ClientManager,
}

impl SpacetimeNode {
    /// Handle client write request
    fn handle_client_write(&mut self, request: ClientRequest) -> Result<ClientResponse> {
        // If not leader, redirect
        if !self.replication.is_leader() {
            let leader = self.replication.get_leader()?;
            return Err(Redirect(leader));
        }

        // Execute transaction
        let result = self.engine.execute_transaction(&request)?;

        // Replicate to followers
        let entry = ReplicationEntry {
            timestamp: self.replication.logical_clock(),
            txid: result.txid,
            tx_type: TransactionType::Reducer {
                name: request.reducer.clone(),
                args: request.args.clone(),
                caller: request.caller,
            },
            payload: bincode::serialize(&result)?,
            checksum: crc32c::crc32c(&request.payload()),
        };

        self.replication.append_and_replicate(entry)?;

        Ok(ClientResponse::Success(result))
    }

    /// Handle replication from leader
    fn handle_replication(&mut self, entry: ReplicationEntry) -> Result<()> {
        // Verify entry
        if !self.replication.verify_entry(&entry) {
            return Err(InvalidEntry);
        }

        // Apply to local state
        self.engine.apply_transaction(&entry)?;

        // Acknowledge to leader
        self.replication.send_ack(entry.txid);

        Ok(())
    }
}
```

### 5.2 Snapshot and Catch-up

```rust
impl SpacetimeNode {
    /// Create snapshot for new followers
    fn create_snapshot(&self) -> Result<Snapshot> {
        Ok(Snapshot {
            commitlog_offset: self.engine.commitlog_offset(),
            table_data: self.engine.snapshot_tables()?,
            timestamp: Instant::now(),
            checksum: self.compute_checksum(),
        })
    }

    /// Install snapshot for lagging follower
    fn install_snapshot(&mut self, snapshot: Snapshot) -> Result<()> {
        // Verify snapshot
        if !self.verify_snapshot(&snapshot) {
            return Err(InvalidSnapshot);
        }

        // Clear current state
        self.engine.clear();

        // Restore from snapshot
        self.engine.restore_tables(snapshot.table_data)?;
        self.engine.set_commitlog_offset(snapshot.commitlog_offset);

        // Replay any entries after snapshot
        self.replication.catchup_from_leader(snapshot.commitlog_offset)?;

        Ok(())
    }
}
```

---

## 6. Multi-Node Deployment

### 6.1 Cluster Configuration

```rust
/// Cluster configuration
struct ClusterConfig {
    /// Cluster members
    members: Vec<MemberConfig>,

    /// Replication factor
    replication_factor: usize,

    /// Quorum size
    quorum_size: usize,
}

struct MemberConfig {
    /// Node ID
    id: NodeId,

    /// Network address
    address: SocketAddr,

    /// Is voter (participates in consensus)
    is_voter: bool,
}

impl ClusterConfig {
    /// Calculate quorum size
    fn calculate_quorum(&mut self) {
        let voters = self.members.iter().filter(|m| m.is_voter).count();
        self.quorum_size = (voters / 2) + 1;
    }

    /// Add new node
    fn add_node(&mut self, node: MemberConfig) {
        self.members.push(node);
        self.calculate_quorum();
    }

    /// Remove node
    fn remove_node(&mut self, node_id: NodeId) {
        self.members.retain(|m| m.id != node_id);
        self.calculate_quorum();
    }
}
```

### 6.2 Dynamic Membership

```rust
/// Joint consensus for cluster changes
struct JointConsensus {
    /// Old configuration
    old_config: ClusterConfig,

    /// New configuration
    new_config: ClusterConfig,

    /// Phase of joint consensus
    phase: JointPhase,
}

enum JointPhase {
    /// Both old and new configs active
    Joint,

    /// Only new config active
    New,

    /// Complete
    Done,
}

impl JointConsensus {
    /// Start cluster membership change
    fn start_change(&mut self, change: MembershipChange) {
        self.new_config = self.apply_change(change);
        self.phase = JointPhase::Joint;
    }

    /// Check if ready to transition
    fn check_transition(&mut self) {
        if let JointPhase::Joint = self.phase {
            // Need replication on both old and new majorities
            if self.replicated_on_both() {
                self.phase = JointPhase::New;
            }
        }
    }

    /// Check if complete
    fn is_complete(&self) -> bool {
        matches!(self.phase, JointPhase::Done)
    }
}
```

---

## Document History

| Date | Change |
|------|--------|
| 2026-03-27 | Initial consensus and replication deep dive created |

---

*This exploration is a living document. Revisit sections as concepts become clearer through implementation.*
