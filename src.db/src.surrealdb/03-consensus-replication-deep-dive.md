# SurrealDB: Distributed Consensus and Replication

## Overview

This document explores SurrealDB's distributed systems aspects:
- Cluster architecture
- Consensus protocols
- Replication strategies
- Distributed transactions

---

## 1. Cluster Architecture

### Node Types

```rust
/// Cluster node
pub enum NodeRole {
    /// Leader node (handles writes)
    Leader,

    /// Follower node (replicates from leader)
    Follower,

    /// Observer node (read-only, no voting)
    Observer,
}

pub struct ClusterNode {
    pub id: NodeId,
    pub address: SocketAddr,
    pub role: NodeRole,
    pub term: u64,
}
```

### Cluster Membership

```rust
pub struct Cluster {
    /// Current leader
    pub leader: Option<NodeId>,

    /// All nodes
    pub nodes: HashMap<NodeId, ClusterNode>,

    /// Voter nodes
    pub voters: HashSet<NodeId>,

    /// Term number
    pub current_term: u64,
}

impl Cluster {
    /// Get quorum size
    pub fn quorum_size(&self) -> usize {
        (self.voters.len() / 2) + 1
    }

    /// Check if node is voter
    pub fn is_voter(&self, node: NodeId) -> bool {
        self.voters.contains(&node)
    }
}
```

---

## 2. Raft Consensus

### Raft State

```rust
pub struct RaftState {
    /// Persistent state
    pub current_term: u64,
    pub voted_for: Option<NodeId>,
    pub log: Vec<LogEntry>,

    /// Volatile state
    pub commit_index: u64,
    pub last_applied: u64,

    /// Leader state
    pub next_index: HashMap<NodeId, u64>,
    pub match_index: HashMap<NodeId, u64>,
}

pub struct LogEntry {
    pub term: u64,
    pub index: u64,
    pub data: Vec<u8>,
}
```

### Leader Election

```rust
impl RaftNode {
    /// Start election
    fn start_election(&mut self) {
        self.state.current_term += 1;
        self.state.voted_for = Some(self.id);
        self.role = Role::Candidate { votes: 1 };

        // Request votes from all nodes
        for node in &self.cluster.nodes {
            if node.0 != &self.id {
                self.send_vote_request(node.0);
            }
        }
    }

    /// Handle vote request
    fn handle_vote_request(&mut self, req: VoteRequest) -> VoteResponse {
        // Reject if stale term
        if req.term < self.state.current_term {
            return VoteResponse { term: self.state.current_term, granted: false };
        }

        // Check log up-to-date
        let log_ok = req.last_log_term > self.last_log_term() ||
            (req.last_log_term == self.last_log_term() &&
             req.last_log_index >= self.last_log_index());

        if self.state.voted_for.is_none() && log_ok {
            self.state.voted_for = Some(req.candidate_id);
            self.state.current_term = req.term;
            VoteResponse { term: req.term, granted: true }
        } else {
            VoteResponse { term: self.state.current_term, granted: false }
        }
    }

    /// Become leader
    fn become_leader(&mut self) {
        self.role = Role::Leader;
        self.cluster.leader = Some(self.id);

        // Initialize leader state
        let next_idx = self.state.log.len() as u64 + 1;
        for node in &self.cluster.nodes {
            if node.0 != &self.id {
                self.state.next_index.insert(*node.0, next_idx);
                self.state.match_index.insert(*node.0, 0);
            }
        }
    }
}
```

---

## 3. Replication

### Log Replication

```rust
impl RaftNode {
    /// Send append entries to followers
    fn send_append_entries(&mut self, follower: NodeId) {
        let next_idx = self.state.next_index[&follower];
        let prev_idx = next_idx - 1;

        let prev_term = if prev_idx > 0 {
            self.state.log[prev_idx as usize].term
        } else {
            0
        };

        let entries = if next_idx <= self.state.log.len() as u64 {
            &self.state.log[next_idx as usize..]
        } else {
            &[]
        };

        let req = AppendEntriesRequest {
            term: self.state.current_term,
            leader_id: self.id,
            prev_log_index: prev_idx,
            prev_log_term: prev_term,
            entries: entries.to_vec(),
            leader_commit: self.state.commit_index,
        };

        self.send_rpc(follower, req);
    }

    /// Handle append entries response
    fn handle_append_response(&mut self, follower: NodeId, resp: AppendEntriesResponse) {
        if resp.success {
            // Update progress
            self.state.next_index.insert(follower, resp.last_index + 1);
            self.state.match_index.insert(follower, resp.last_index);

            // Check if can commit
            self.update_commit_index();
        } else {
            // Decrement and retry
            let next = self.state.next_index.get_mut(&follower).unwrap();
            *next = next.saturating_sub(1);
            self.send_append_entries(follower);
        }
    }
}
```

---

## 4. Distributed Transactions

### Transaction Coordinator

```rust
pub struct DistributedTransaction {
    pub id: TxId,
    pub coordinator: NodeId,
    pub participants: HashSet<NodeId>,
    pub state: TxState,
}

#[derive(Clone, Copy, PartialEq)]
pub enum TxState {
    Active,
    Preparing,
    Prepared,
    Committing,
    Committed,
    Aborting,
    Aborted,
}

impl TransactionCoordinator {
    /// Begin distributed transaction
    pub fn begin(&mut self, nodes: Vec<NodeId>) -> TxId {
        let tx_id = TxId::generate();

        self.transactions.insert(tx_id, DistributedTransaction {
            id: tx_id,
            coordinator: self.node_id,
            participants: nodes.into_iter().collect(),
            state: TxState::Active,
        });

        tx_id
    }

    /// Two-phase commit
    pub fn commit(&mut self, tx_id: TxId) -> Result<()> {
        let tx = self.transactions.get(&tx_id).unwrap();

        // Phase 1: Prepare
        self.state = TxState::Preparing;
        for participant in &tx.participants {
            self.send_prepare(*participant, tx_id)?;
        }

        // Wait for all ACKs
        self.wait_for_prepares(&tx_id)?;
        self.state = TxState::Prepared;

        // Phase 2: Commit
        self.state = TxState::Committing;
        for participant in &tx.participants {
            self.send_commit(*participant, tx_id)?;
        }

        self.state = TxState::Committed;
        Ok(())
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
