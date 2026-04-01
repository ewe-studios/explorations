---
title: "TigerBeetle Consensus & Replication Deep Dive"
subtitle: "Viewstamped Replication protocol, leader election, and fault tolerance"
location: /home/darkvoid/Boxxed/@dev/repo-expolorations/src.db/src.tigerbeetle
related: 00-zero-to-ledger-engineer.md, 01-storage-engine-deep-dive.md, 02-query-execution-deep-dive.md, exploration.md
---

# 03 - Consensus & Replication Deep Dive: TigerBeetle

## Overview

This document covers TigerBeetle's distributed consensus implementation using Viewstamped Replication (VR) - how replicas coordinate, leader election, view changes, and fault tolerance guarantees.

## Part 1: Viewstamped Replication Fundamentals

### VR Protocol Overview

```
Viewstamped Replication (VR) is a consensus protocol similar to Raft/Paxos:

Key Concepts:
┌───────────────────────────────────────────────────────────┐
│ View: Current configuration (leader + followers)           │
│                                                          │
│ - View 0: Leader = Replica 0, Followers = [1, 2]         │
│ - View 1: Leader = Replica 1, Followers = [0, 2]         │
│ - View N: Leader = Replica (N mod 3), Followers = others │
│                                                          │
│ Leader rotates in round-robin fashion on failure          │
└───────────────────────────────────────────────────────────┘

┌───────────────────────────────────────────────────────────┐
│ Quorum: Majority must agree                               │
│                                                          │
│ - 3 replicas: 2 needed for quorum                        │
│ - 5 replicas: 3 needed for quorum                        │
│ - 7 replicas: 4 needed for quorum                        │
│                                                          │
│ Formula: quorum = (N / 2) + 1                            │
└───────────────────────────────────────────────────────────┘

┌───────────────────────────────────────────────────────────┐
│ Log Replication: All operations go through leader         │
│                                                          │
│ 1. Client sends operation to leader                      │
│ 2. Leader assigns sequence number                        │
│ 3. Leader replicates to followers                        │
│ 4. After quorum acks, leader commits                     │
│ 5. Leader notifies followers to commit                   │
│ 6. Leader responds to client                             │
└───────────────────────────────────────────────────────────┘

VR vs Raft vs Paxos:
┌──────────────────────────────────────────────────────────┐
│ Aspect        │ VR      │ Raft      │ Paxos      │       │
├──────────────────────────────────────────────────────────┤
│ Leader selection │ View-based │ Election  │ Proposer   │ │
│ Log format       │ Indexed   │ Indexed   │ Ballot     │ │
│ Membership change│ Implicit  │ Joint     │ Single     │ │
│ View change      │ Round-robin │ Election │ New ballot │ │
└──────────────────────────────────────────────────────────┘

VR advantage: Simpler view change (deterministic round-robin)
```

### Replica States

```
Replica State Machine:

┌───────────────────────────────────────────────────────────┐
│                                                            │
│                      ┌─────────────┐                       │
│                      │  FOLLOWER   │                       │
│                      └──────┬──────┘                       │
│                             │                               │
│              View change timeout                            │
│              (leader not heard)                             │
│                             │                               │
│                             ▼                               │
│                      ┌─────────────┐                       │
│                ┌────►│  CANDIDATE  │────┐                  │
│                │     └──────┬──────┘    │                  │
│                │            │           │                  │
│         Win election        │           │ Lose election    │
│                │            │           │                  │
│                ▼            │           ▼                  │
│         ┌─────────────┐    │    ┌─────────────┐           │
│         │   LEADER    │────┘    │  FOLLOWER   │           │
│         └─────────────┘         └─────────────┘           │
│                                                            │
│ State transitions are triggered by:                        │
│ - Timer expiration                                         │
│ - Received messages                                        │
│ - View change protocol                                     │
└────────────────────────────────────────────────────────────┘

Replica State Data:
```rust
struct Replica {
    /// This replica's ID (0, 1, or 2 for 3-replica cluster)
    id: u32,

    /// Current view number
    view: u64,

    /// Current role
    role: Role,

    /// Log of operations
    log: Vec<LogEntry>,

    /// Commit index (highest committed log index)
    commit_index: u64,

    /// Last applied index (highest applied to state machine)
    last_applied: u64,
}

enum Role {
    Leader {
        /// Next log index to send to each follower
        next_index: Vec<u64>,

        /// Highest log index known to be replicated
        match_index: Vec<u64>,
    },
    Follower {
        /// Current leader's ID
        leader_id: u32,

        /// Timeout for leader heartbeat
        election_timeout: Duration,
    },
    Candidate,
}
```

### Log Entry Structure

```
Log Entry Format (replicated via VR):

┌─────────────────────────────────────────────────────────┐
│ Log Entry Header (32 bytes)                               │
│ ┌─────────────────────────────────────────────────────┐ │
│ │ Term: u64 (8 bytes)                                 │ │
│ │     View number when entry was created              │ │
│ │ Log Index: u64 (8 bytes)                            │ │
│ │     Position in log (sequence number)               │ │
│ │ Entry Type: u8 (1 byte)                             │ │
│ │     0x01 = NoOp (leader election)                   │ │
│ │     0x02 = CreateAccounts                           │ │
│ │     0x03 = CreateTransfers                          │ │
│ │     0x04 = ConfigChange                             │ │
│ │ Timestamp: u64 (8 bytes)                            │ │
│ │     Nanosecond-precision timestamp                  │ │
│ │ Checksum: u32 (4 bytes)                             │ │
│ │     CRC32C of entry                                 │ │
│ │ Reserved: 3 bytes                                   │ │
│ └─────────────────────────────────────────────────────┘ │
├─────────────────────────────────────────────────────────┤
│ Log Entry Data (Variable, typically 128-1024 bytes)       │
│ ┌─────────────────────────────────────────────────────┐ │
│ │ For CreateAccounts:                                 │ │
│ │   Vec<Account> (serialized)                         │ │
│ │                                                     │ │
│ │ For CreateTransfers:                                │ │
│ │   Vec<Transfer> (serialized)                        │ │
│ │                                                     │ │
│ │ For ConfigChange:                                   │ │
│ │   ConfigChange struct                               │ │
│ └─────────────────────────────────────────────────────┘ │
└─────────────────────────────────────────────────────────┘

Log indexing:
- Index 0: First entry after cluster bootstrap
- Index N: Nth operation in cluster history
- Globally ordered across all replicas
```

## Part 2: Normal Operation (Leader Handling Requests)

### Request Processing Flow

```
Client Request Flow Through VR:

┌───────────────────────────────────────────────────────────┐
│ Step 1: Client Sends to Leader                             │
│                                                          │
│   Client ──────► Replica 0 (Leader)                      │
│   Request: create_transfers(...)                         │
│                                                          │
│   Client knows leader from previous response or          │
│   tries any replica (which redirects to leader)          │
└───────────────────────────────────────────────────────────┘
                         │
                         ▼
┌───────────────────────────────────────────────────────────┐
│ Step 2: Leader Assigns Log Index                           │
│                                                          │
│   Leader:                                                │
│   1. Validate request                                    │
│   2. Append to local log: log[42] = Transfer(...)        │
│   3. Increment log tail                                  │
│                                                          │
│   Log state:                                             │
│   [0..41] = Previous entries                             │
│   [42] = New transfer (uncommitted)                      │
└───────────────────────────────────────────────────────────┘
                         │
                         ▼
┌───────────────────────────────────────────────────────────┐
│ Step 3: Replicate to Followers                             │
│                                                          │
│   Leader ──────► Follower 1                              │
│   AppendEntries {                                        │
│     term: 5,                                             │
│     prev_log_index: 41,                                  │
│     prev_log_term: 5,                                    │
│     entries: [log[42]],                                  │
│     leader_commit: 41                                    │
│   }                                                      │
│                                                          │
│   Leader ──────► Follower 2                              │
│   (same message)                                         │
└───────────────────────────────────────────────────────────┘
                         │
                         ▼
┌───────────────────────────────────────────────────────────┐
│ Step 4: Followers Acknowledge                              │
│                                                          │
│   Follower 1 ──────► Leader                              │
│   AppendEntriesResponse {                                │
│     term: 5,                                             │
│     success: true,                                       │
│     match_index: 42                                      │
│   }                                                      │
│                                                          │
│   Follower 2 ──────► Leader                              │
│   (same response)                                        │
│                                                          │
│   Leader waits for quorum (2 of 3 replicas)              │
└───────────────────────────────────────────────────────────┘
                         │
                         ▼
┌───────────────────────────────────────────────────────────┐
│ Step 5: Leader Commits                                     │
│                                                          │
│   After quorum acks:                                     │
│   1. Mark log[42] as committed                           │
│   2. Update commit_index = 42                            │
│   3. Apply to state machine                              │
│   4. Respond to client: SUCCESS                          │
│                                                          │
│   Commitment is irreversible                             │
└───────────────────────────────────────────────────────────┘
                         │
                         ▼
┌───────────────────────────────────────────────────────────┐
│ Step 6: Notify Followers to Commit                         │
│                                                          │
│   Leader ──────► Follower 1                              │
│   Commit {                                               │
│     term: 5,                                             │
│     commit_index: 42                                     │
│   }                                                      │
│                                                          │
│   Follower 1:                                            │
│   1. Apply log[42] to state machine                      │
│   2. Update commit_index = 42                            │
│                                                          │
│   (Same for Follower 2)                                  │
└───────────────────────────────────────────────────────────┘
```

### Leader Code Path

```rust
/// Leader processes client request
fn handle_client_request(&mut self, request: ClientRequest) {
    // Must be leader
    assert!(matches!(self.role, Role::Leader { .. }));

    // Assign log index
    let log_index = self.log.len() as u64;
    let term = self.view;

    // Create log entry
    let entry = LogEntry {
        term,
        index: log_index,
        entry_type: request.to_entry_type(),
        data: request.serialize(),
        timestamp: current_timestamp_ns(),
    };

    // Append to local log
    self.log.push(entry.clone());

    // Replicate to followers
    self.replicate_to_followers(log_index, &entry);

    // Store pending entry for when quorum arrives
    self.pending_commits.insert(log_index, PendingCommit {
        entry,
        acks: 1, // Leader counts as 1 ack
        client_tx: request.response_channel,
    });
}

/// Replicate entry to all followers
fn replicate_to_followers(&mut self, log_index: u64, entry: &LogEntry) {
    let prev_log_index = log_index.saturating_sub(1);
    let prev_log_term = if prev_log_index == 0 {
        0
    } else {
        self.log[prev_log_index as usize].term
    };

    for (follower_id, follower) in self.followers.iter_mut() {
        let msg = AppendEntries {
            term: self.view,
            leader_id: self.id,
            prev_log_index,
            prev_log_term,
            entries: vec![entry.clone()],
            leader_commit: self.commit_index,
        };

        follower.send(msg);
    }
}

/// Handle follower acknowledgment
fn handle_append_response(&mut self, response: AppendEntriesResponse) {
    if let Role::Leader { next_index, match_index } = &mut self.role {
        let follower_idx = response.from as usize;

        if response.success {
            // Update match index
            match_index[follower_idx] = response.match_index;

            // Count acks for pending commits
            if let Some(pending) = self.pending_commits.get_mut(&response.match_index) {
                pending.acks += 1;

                // Check if quorum reached
                if pending.acks >= self.quorum_size() {
                    self.commit_entry(response.match_index, pending);
                }
            }
        } else {
            // Follower log mismatch - decrement next_index and retry
            next_index[follower_idx] = next_index[follower_idx].saturating_sub(1);
            self.retry_replication(follower_idx);
        }
    }
}

/// Commit entry after quorum
fn commit_entry(&mut self, log_index: u64, pending: PendingCommit) {
    // Mark as committed
    self.commit_index = log_index;

    // Apply to state machine
    self.apply_to_state_machine(&pending.entry);

    // Respond to client
    pending.client_tx.send(Response::Success);

    // Notify followers to commit
    self.notify_commit(log_index);

    // Remove from pending
    self.pending_commits.remove(&log_index);
}
```

### Follower Code Path

```rust
/// Follower processes AppendEntries from leader
fn handle_append_entries(&mut self, msg: AppendEntries) {
    // Check term
    if msg.term < self.view {
        self.send(AppendEntriesResponse {
            from: self.id,
            term: self.view,
            success: false,
            match_index: 0,
        });
        return;
    }

    // Update view if term is higher
    if msg.term > self.view {
        self.view = msg.term;
        self.role = Role::Follower {
            leader_id: msg.leader_id,
            election_timeout: DEFAULT_ELECTION_TIMEOUT,
        };
    }

    // Reset election timeout
    if let Role::Follower { election_timeout, .. } = &mut self.role {
        *election_timeout = DEFAULT_ELECTION_TIMEOUT;
    }

    // Validate log continuity
    if msg.prev_log_index > 0 {
        if let Some(local_entry) = self.log.get(msg.prev_log_index as usize) {
            if local_entry.term != msg.prev_log_term {
                // Log mismatch - reject
                self.send(AppendEntriesResponse {
                    from: self.id,
                    term: self.view,
                    success: false,
                    match_index: 0,
                });
                return;
            }
        }
    }

    // Append entries to log
    for (i, entry) in msg.entries.iter().enumerate() {
        let log_index = (msg.prev_log_index as usize + 1 + i) as u64;

        if let Some(local_entry) = self.log.get_mut(log_index as usize) {
            if local_entry.term != entry.term {
                // Conflict - replace entry and all after
                self.log.truncate(log_index as usize);
                self.log.push(entry.clone());
            }
        } else {
            // Append new entry
            self.log.push(entry.clone());
        }
    }

    // Update commit index
    if msg.leader_commit > self.commit_index {
        self.commit_index = msg.leader_commit.min(self.log.len() as u64);

        // Apply committed entries
        while self.last_applied < self.commit_index {
            self.apply_to_state_machine(&self.log[self.last_applied as usize]);
            self.last_applied += 1;
        }
    }

    // Respond to leader
    let match_index = if msg.entries.is_empty() {
        msg.prev_log_index
    } else {
        msg.prev_log_index + msg.entries.len() as u64
    };

    self.send(AppendEntriesResponse {
        from: self.id,
        term: self.view,
        success: true,
        match_index,
    });
}
```

## Part 3: Leader Election and View Changes

### Election Trigger

```
When Does Election Occur?

┌───────────────────────────────────────────────────────────┐
│ Scenario 1: Leader Failure                                 │
│                                                          │
│ Follower detects leader failure when:                    │
│ - No AppendEntries or heartbeat for election_timeout     │
│ - Typical timeout: 150-300ms (randomized)                │
│                                                          │
│ Timeline:                                                │
│ T=0ms:     Leader sends last heartbeat                   │
│ T=100ms:   Follower expects next heartbeat               │
│ T=250ms:   Follower timeout expires (randomized)         │
│ T=250ms:   Follower becomes candidate, starts election   │
└───────────────────────────────────────────────────────────┘

┌───────────────────────────────────────────────────────────┐
│ Scenario 2: Network Partition                              │
│                                                          │
│ Follower in partitioned segment:                         │
│ - Cannot reach leader                                    │
│ - Timeout expires                                        │
│ - Starts election                                        │
│                                                          │
│ If partition has quorum: New leader elected              │
│ If partition lacks quorum: No leader (read-only)         │
└───────────────────────────────────────────────────────────┘

┌───────────────────────────────────────────────────────────┐
│ Scenario 3: Leader Step Down                               │
│                                                          │
│ Leader steps down when:                                  │
│ - Receives message from higher term                      │
│ - Administratively triggered                             │
│                                                          │
│ Steps:                                                   │
│ 1. Become follower                                       │
│ 2. Start election after timeout                          │
└───────────────────────────────────────────────────────────┘
```

### Election Process

```
View Change Election Protocol:

┌───────────────────────────────────────────────────────────┐
│ Phase 1: Become Candidate                                  │
│                                                          │
│ Follower timeout expires:                                │
│ 1. Increment view: view = view + 1                       │
│ 2. Become candidate                                      │
│ 3. Vote for self                                         │
│ 4. Send RequestVote to all replicas                      │
│                                                          │
│ RequestVote {                                            │
│   term: new_view,                                        │
│   candidate_id: my_id,                                   │
│   last_log_index: log.len(),                             │
│   last_log_term: log.last().term                         │
│ }                                                        │
└───────────────────────────────────────────────────────────┘
                         │
                         ▼
┌───────────────────────────────────────────────────────────┐
│ Phase 2: Voting                                            │
│                                                          │
│ Other replicas receive RequestVote:                      │
│                                                          │
│ 1. Check term >= current term                            │
│ 2. Check candidate log is up-to-date                     │
│    (last_log_index >= my_last_log_index)                │
│ 3. If OK, vote YES:                                      │
│    - Update view                                          │
│    - Reset election timeout                               │
│    - Send VoteResponse { vote_granted: true }           │
│                                                          │
│ If term < current term or log stale:                     │
│    VoteResponse { vote_granted: false }                  │
└───────────────────────────────────────────────────────────┘
                         │
                         ▼
┌───────────────────────────────────────────────────────────┐
│ Phase 3: Count Votes                                       │
│                                                          │
│ Candidate collects votes:                                │
│                                                          │
│ If votes >= quorum:                                      │
│   - Win election                                         │
│   - Become leader                                        │
│   - Send heartbeat (empty AppendEntries)                 │
│   - Start handling client requests                       │
│                                                          │
│ If votes < quorum and timeout expires:                   │
│   - Election failed                                      │
│   - Start new election (increment view)                  │
│                                                          │
│ If receive message from higher term:                     │
│   - Step down to follower                                │
│   - Stop election                                        │
└───────────────────────────────────────────────────────────┘

Election Code:
```rust
/// Start election (called when timeout expires)
fn start_election(&mut self) {
    // Increment view
    self.view += 1;

    // Become candidate
    self.role = Role::Candidate;

    // Vote for self
    let votes = Votes::new(self.id);

    // Send RequestVote to all replicas
    let request = RequestVote {
        term: self.view,
        candidate_id: self.id,
        last_log_index: self.log.len() as u64,
        last_log_term: self.log.last().map(|e| e.term).unwrap_or(0),
    };

    for replica_id in self.all_replica_ids() {
        if replica_id != self.id {
            self.send_to(replica_id, request);
        }
    }

    self.votes = votes;
    self.election_deadline = Instant::now() + self.election_timeout();
}

/// Handle RequestVote from candidate
fn handle_request_vote(&mut self, request: RequestVote) {
    // Default: deny vote
    let mut vote_granted = false;

    // Check term
    if request.term >= self.view {
        // Update view if term is higher
        if request.term > self.view {
            self.view = request.term;
            self.role = Role::Follower {
                leader_id: 0, // No leader yet
                election_timeout: DEFAULT_ELECTION_TIMEOUT,
            };
        }

        // Check candidate log is at least as up-to-date as ours
        let my_last_index = self.log.len() as u64;
        let my_last_term = self.log.last().map(|e| e.term).unwrap_or(0);

        if request.last_log_term > my_last_term
            || (request.last_log_term == my_last_term
                && request.last_log_index >= my_last_index)
        {
            // Grant vote
            vote_granted = true;
        }
    }

    // Send response
    self.send_to(request.candidate_id, VoteResponse {
        term: self.view,
        vote_granted,
    });
}

/// Handle vote response (candidate only)
fn handle_vote_response(&mut self, response: VoteResponse) {
    if let Role::Candidate = self.role {
        if response.term > self.view {
            // Higher term - step down
            self.view = response.term;
            self.role = Role::Follower {
                leader_id: 0,
                election_timeout: DEFAULT_ELECTION_TIMEOUT,
            };
            return;
        }

        if response.vote_granted && response.term == self.view {
            self.votes.add_vote(response.from);

            // Check if won election
            if self.votes.count() >= self.quorum_size() {
                self.become_leader();
            }
        }
    }
}

/// Become leader after winning election
fn become_leader(&mut self) {
    let num_replicas = self.replica_ids.len();

    // Initialize leader state
    let mut next_index = vec![0; num_replicas];
    let mut match_index = vec![0; num_replicas];

    for (i, replica_id) in self.replica_ids.iter().enumerate() {
        if *replica_id != self.id {
            next_index[i] = self.log.len() as u64;
            match_index[i] = 0;
        }
    }

    self.role = Role::Leader {
        next_index,
        match_index,
    };

    // Send heartbeat (empty AppendEntries)
    self.send_heartbeats();
}
```

### Split Vote Handling

```
Split Vote Scenario (3 replicas):

┌───────────────────────────────────────────────────────────┐
│ Initial State:                                             │
│   Replica 0: Leader (view 5)                              │
│   Replica 1: Follower                                     │
│   Replica 2: Follower                                     │
└───────────────────────────────────────────────────────────┘

┌───────────────────────────────────────────────────────────┐
│ T=0: Leader (Replica 0) fails                              │
│                                                          │
│   Replica 1 timeout: 150ms                               │
│   Replica 2 timeout: 250ms (randomized)                  │
└───────────────────────────────────────────────────────────┘

┌───────────────────────────────────────────────────────────┐
│ T=150ms: Replica 1 timeout expires                         │
│                                                          │
│   Replica 1:                                             │
│   - view = 6                                             │
│   - Become candidate                                     │
│   - Vote for self (1 vote)                               │
│   - Send RequestVote to Replica 2                        │
│                                                          │
│   Replica 2:                                             │
│   - Receive RequestVote from Replica 1                   │
│   - Grant vote (1 vote for Replica 1)                    │
│   - Replica 1 has quorum (2 votes), becomes leader       │
└───────────────────────────────────────────────────────────┘

Split Vote Scenario (5 replicas with network issues):

┌───────────────────────────────────────────────────────────┐
│ T=0: Two followers timeout at same time                    │
│                                                          │
│   Replica 1: view = 6, candidate, votes: [1]             │
│   Replica 2: view = 6, candidate, votes: [2]             │
│                                                          │
│   Replica 3 receives both RequestVotes:                  │
│   - First from Replica 1: grants vote                    │
│   - Second from Replica 2: DENIES (already voted)        │
│                                                          │
│   Replica 4 receives both RequestVotes:                  │
│   - First from Replica 2: grants vote                    │
│   - Second from Replica 1: DENIES (already voted)        │
│                                                          │
│ Result:                                                   │
│   Replica 1: 2 votes (self + Replica 3) - NO QUORUM      │
│   Replica 2: 2 votes (self + Replica 4) - NO QUORUM      │
│                                                          │
│ Both timeout and start new election (view 7)             │
│ Randomized timeouts prevent repeat split                 │
└───────────────────────────────────────────────────────────┘
```

## Part 4: Log Consistency and Recovery

### Log Matching Property

```
VR Log Matching Guarantee:

┌───────────────────────────────────────────────────────────┐
│ Property: If two logs have an entry with the same          │
│ index and term, then all entries up to that index match.   │
│                                                          │
│ Proof by induction:                                      │
│ Base case: Index 0 (first entry) - trivially true        │
│ Inductive step: If true for index N, true for N+1        │
│   - Leader creates entries with unique (index, term)     │
│   - Followers only append entries from leader            │
│   - Entry at index N+1 only appended after N matches     │
└───────────────────────────────────────────────────────────┘

Log Inconsistency Detection:

┌───────────────────────────────────────────────────────────┐
│ Scenario: Follower log diverges from leader                │
│                                                          │
│ Leader Log:    [A, B, C, D, E]                            │
│ Follower Log:  [A, B, X, Y]                               │
│                                                          │
│ Leader sends AppendEntries {                             │
│   prev_log_index: 2,  ◄── Points to C                    │
│   prev_log_term: 5,   ◄── Term of C                      │
│   entries: [D, E]                                        │
│ }                                                        │
│                                                          │
│ Follower checks:                                         │
│   log[2].term == 5?                                      │
│   Follower log[2].term = 3 (X has different term)        │
│   MISMATCH!                                              │
│                                                          │
│ Follower rejects:                                        │
│   AppendEntriesResponse { success: false }               │
│                                                          │
│ Leader backs up and retries:                             │
│   Next AppendEntries { prev_log_index: 1, ... }          │
│   Follower log[1].term = 5 (B matches) ✓                 │
│   Follower accepts, replaces X, Y with C, D, E           │
│   Follower Log: [A, B, C, D, E] ✓                        │
└───────────────────────────────────────────────────────────┘
```

### Log Repair Code

```rust
/// Leader handles rejection from follower
fn handle_append_reject(&mut self, response: AppendEntriesResponse, follower_id: u32) {
    if let Role::Leader { next_index, .. } = &mut self.role {
        let follower_idx = follower_id as usize;

        // Decrement next_index and retry
        next_index[follower_idx] = next_index[follower_idx].saturating_sub(1);

        log::debug!(
            "Follower {} rejected, backing up to index {}",
            follower_id,
            next_index[follower_idx]
        );

        // Retry replication
        self.retry_replication(follower_idx);
    }
}

/// Retry sending entries to follower
fn retry_replication(&mut self, follower_idx: usize) {
    let next_index = self.next_index[follower_idx];
    let prev_log_index = next_index.saturating_sub(1);
    let prev_log_term = if prev_log_index == 0 {
        0
    } else {
        self.log[prev_log_index as usize].term
    };

    // Send entries starting from next_index
    let entries: Vec<LogEntry> = self.log[next_index as usize..].to_vec();

    let msg = AppendEntries {
        term: self.view,
        leader_id: self.id,
        prev_log_index,
        prev_log_term,
        entries,
        leader_commit: self.commit_index,
    };

    self.send_to_follower(follower_idx, msg);
}

/// Follower handles log mismatch by truncating
fn handle_log_mismatch(&mut self, prev_log_index: u64, prev_log_term: u64) {
    // Find conflict point
    for i in (0..=prev_log_index).rev() {
        if let Some(entry) = self.log.get(i as usize) {
            if entry.term == prev_log_term {
                // Found matching entry - truncate everything after
                self.log.truncate((i + 1) as usize);
                return;
            }
        }
    }

    // No match found - truncate entire log
    self.log.clear();
}
```

### Crash Recovery

```
Recovery After Replica Crash:

┌───────────────────────────────────────────────────────────┐
│ Step 1: Load Persistent State                              │
│                                                          │
│ On startup, replica loads from disk:                     │
│ - view (current term)                                    │
│ - voted_for (candidate voted for in current term)        │
│ - log (all log entries)                                  │
│ - commit_index (highest committed index)                 │
│ - state machine snapshot (accounts, transfers)           │
│                                                          │
│ All data persisted via:                                  │
│ - Vote state: fsync after each vote                      │
│ - Log: fsync after each append                           │
│ - State machine: checkpoint + WAL replay                 │
└───────────────────────────────────────────────────────────┘
                         │
                         ▼
┌───────────────────────────────────────────────────────────┐
│ Step 2: Replay Uncommitted Log Entries                     │
│                                                          │
│ For each log entry after last checkpoint:                │
│ 1. Read entry from WAL                                   │
│ 2. Verify checksum                                       │
│ 3. Apply to state machine                                │
│                                                          │
│ Entries are applied in order:                            │
│ - CreateAccounts: Create accounts                        │
│ - CreateTransfers: Execute transfers                     │
│                                                          │
│ After replay:                                            │
│ - State machine reflects all committed entries           │
│ - Uncommitted entries (if any) are discarded             │
└───────────────────────────────────────────────────────────┘
                         │
                         ▼
┌───────────────────────────────────────────────────────────┐
│ Step 3: Join Cluster                                       │
│                                                          │
│ Replica contacts other replicas:                         │
│ 1. Send JoinCluster { replica_id, view, log_length }     │
│ 2. Current leader responds with missing entries          │
│ 3. Replica syncs log with leader                         │
│ 4. Replica becomes follower                              │
│                                                          │
│ If this replica was leader:                              │
│ - It may no longer be leader (election happened)         │
│ - It discovers new leader via term mismatch              │
│ - It becomes follower and syncs from new leader          │
└───────────────────────────────────────────────────────────┘

Recovery Code:
```rust
/// Recover replica state from disk
fn recover_from_disk(&mut self) -> Result<()> {
    // Load metadata
    let metadata = self.storage.load_metadata()?;
    self.view = metadata.view;
    self.voted_for = metadata.voted_for;
    self.commit_index = metadata.commit_index;

    // Load log
    self.log = self.storage.load_log()?;

    // Load state machine snapshot
    let snapshot = self.storage.load_snapshot()?;
    self.state_machine = snapshot.state_machine;
    self.last_applied = snapshot.last_applied;

    // Replay log entries after snapshot
    for entry in &self.log[self.last_applied as usize..] {
        if entry.index <= self.commit_index {
            // Committed but not applied - replay
            self.apply_to_state_machine(entry);
            self.last_applied = entry.index;
        }
    }

    log::info!(
        "Recovery complete: view={}, log_len={}, commit_index={}",
        self.view,
        self.log.len(),
        self.commit_index
    );

    Ok(())
}
```

## Part 5: Fault Tolerance

### Failure Scenarios

```
Failure Scenario 1: Leader Crash

┌───────────────────────────────────────────────────────────┐
│ Initial:                                                   │
│   Leader: Replica 0 (view 5)                              │
│   Followers: Replica 1, Replica 2                         │
│   Log: [A, B, C] committed                                │
└───────────────────────────────────────────────────────────┘

┌───────────────────────────────────────────────────────────┐
│ T=0: Leader crashes while processing request D             │
│                                                          │
│   Leader log: [A, B, C, D] (D not committed)             │
│   D was sent to followers but not acked                  │
│                                                          │
│   Client: No response (will retry)                       │
└───────────────────────────────────────────────────────────┘

┌───────────────────────────────────────────────────────────┐
│ T=200ms: Followers detect leader failure                   │
│                                                          │
│   Replica 1:                                             │
│   - Timeout expires                                      │
│   - view = 6                                             │
│   - Become candidate                                     │
│   - RequestVote to Replica 2                             │
│                                                          │
│   Replica 2:                                             │
│   - Grant vote                                           │
│   - Replica 1 wins election                              │
│                                                          │
│   Replica 1 (new leader):                                │
│   - Send heartbeat                                       │
│   - Ready for client requests                            │
└───────────────────────────────────────────────────────────┘

┌───────────────────────────────────────────────────────────┐
│ T=300ms: Client retries request D                          │
│                                                          │
│   Client ──► Replica 1 (new leader)                      │
│   Replica 1:                                             │
│   - Assign log index: log[4] = D                         │
│   - Replicate to Replica 2                               │
│   - Commit after ack                                     │
│   - Respond to client: SUCCESS                           │
│                                                          │
│ Entry D is now committed (possibly twice - idempotent)   │
└───────────────────────────────────────────────────────────┘

Failure Scenario 2: Network Partition

┌───────────────────────────────────────────────────────────┐
│ Cluster: 5 replicas across 2 data centers                  │
│   DC1: Replica 0, 1, 2                                    │
│   DC2: Replica 3, 4                                       │
└───────────────────────────────────────────────────────────┘

┌───────────────────────────────────────────────────────────┐
│ T=0: Network partition between DC1 and DC2                 │
│                                                          │
│   DC1 segment: 3 replicas (has quorum)                   │
│   DC2 segment: 2 replicas (no quorum)                    │
│                                                          │
│   DC1: Can elect leader, process writes                  │
│   DC2: Cannot elect leader, read-only                    │
└───────────────────────────────────────────────────────────┘

┌───────────────────────────────────────────────────────────┐
│ During Partition:                                          │
│                                                          │
│   DC1 (majority):                                        │
│   - Leader: Replica 0                                    │
│   - Continues processing writes                          │
│   - Log: [A, B, C, D, E] committed                       │
│                                                          │
│   DC2 (minority):                                        │
│   - No leader (cannot form quorum)                       │
│   - Client requests rejected                             │
│   - Log: [A, B, C] (stale)                               │
└───────────────────────────────────────────────────────────┘

┌───────────────────────────────────────────────────────────┐
│ T=60s: Partition heals                                     │
│                                                          │
│   Replica 3, 4 contact DC1 leader                        │
│   Leader replicates missing entries D, E                 │
│   All replicas converge to [A, B, C, D, E]               │
│   Cluster fully operational                              │
└───────────────────────────────────────────────────────────┘
```

### F-Tolerance Guarantee

```
Fault Tolerance Formula:

For N replicas, can tolerate F failures where:
  N = 2F + 1

┌───────────────────────────────────────────────────────────┐
│ Replicas (N) │ Failures Tolerated (F) │ Quorum Size      │
├───────────────────────────────────────────────────────────┤
│ 3            │ 1                      │ 2                │
│ 5            │ 2                      │ 3                │
│ 7            │ 3                      │ 4                │
│ 9            │ 4                      │ 5                │
└───────────────────────────────────────────────────────────┘

Why 2F+1?
- Need F+1 replicas for quorum (majority)
- After F failures, N-F replicas remain
- Need N-F >= F+1 for quorum
- N >= 2F+1

Example: 3 replicas, 1 failure tolerated
┌───────────────────────────────────────────────────────────┐
│ Normal: [R0, R1, R2] - 3 replicas, quorum = 2            │
│ After 1 failure: [R0, R1] - 2 replicas, quorum = 2 ✓     │
│ After 2 failures: [R0] - 1 replica, quorum = 2 ✗         │
└───────────────────────────────────────────────────────────┘

TigerBeetle default: 3 replicas
- Tolerates 1 replica failure
- Can span 3 availability zones
- Low latency (small quorum)
- Cost-effective
```

## Part 6: Performance Considerations

### Replication Latency

```
Replication Latency Breakdown:

┌───────────────────────────────────────────────────────────┐
│ Same Region (single data center)                           │
│                                                          │
│ Network RTT: < 1ms                                       │
│ Replication overhead: ~2ms                               │
│ Total write latency: ~3-5ms                              │
│                                                          │
│ Flow:                                                    │
│ Client → Leader: 0.5ms                                   │
│ Leader → Followers: 0.5ms                                │
│ Followers → Leader: 0.5ms                                │
│ Leader → Client: 0.5ms                                   │
│ Processing: ~1ms                                         │
└───────────────────────────────────────────────────────────┘

┌───────────────────────────────────────────────────────────┐
│ Cross Region (multi-region deployment)                     │
│                                                          │
│ Network RTT: 50-100ms (cross-country)                    │
│ Replication overhead: ~150ms                             │
│ Total write latency: ~200-300ms                          │
│                                                          │
│ Flow:                                                    │
│ Client → Leader (US-East): 10ms                          │
│ Leader → Followers (US-West, EU): 75ms                   │
│ Followers → Leader: 75ms                                 │
│ Leader → Client: 10ms                                    │
│ Processing: ~5ms                                         │
│                                                          │
│ Optimization: Place leader near majority of clients      │
└───────────────────────────────────────────────────────────┘

Async Replication Option:

For read-heavy workloads, TigerBeetle supports async replication:
- Leader responds after local append (before quorum ack)
- Replication happens asynchronously
- Faster writes (~1ms vs ~150ms cross-region)
- Risk: Data loss if leader fails before replication

Use async for:
- Read-heavy workloads (90%+ reads)
- Eventual consistency acceptable
- Caching layers
```

### Batched Replication

```
Batching for Throughput:

Without Batching:
┌───────────────────────────────────────────────────────────┐
│ Request 1: [Transfer A] ──► Replicate ──► Commit         │
│ Request 2: [Transfer B] ──► Replicate ──► Commit         │
│ Request 3: [Transfer C] ──► Replicate ──► Commit         │
│                                                          │
│ 3 round trips, 3x replication overhead                   │
└───────────────────────────────────────────────────────────┘

With Batching:
┌───────────────────────────────────────────────────────────┐
│ Batch: [Transfer A, B, C] ──► Replicate ──► Commit       │
│                                                          │
│ 1 round trip, 1x replication overhead                    │
│ Throughput: 3x improvement                               │
└───────────────────────────────────────────────────────────┘

Batching Implementation:
```rust
struct ReplicationBatcher {
    pending: Vec<LogEntry>,
    max_batch_size: usize,
    max_batch_wait: Duration,
    timer: Instant,
}

impl ReplicationBatcher {
    fn add_entry(&mut self, entry: LogEntry) {
        self.pending.push(entry);

        // Flush if batch is full or timeout
        if self.pending.len() >= self.max_batch_size
            || self.timer.elapsed() > self.max_batch_wait
        {
            self.flush();
        }
    }

    fn flush(&mut self) {
        if self.pending.is_empty() {
            return;
        }

        // Create batched AppendEntries message
        let batch_msg = AppendEntries {
            entries: self.pending.clone(),
            ..
        };

        // Replicate to all followers
        self.replicate_to_all(batch_msg);

        self.pending.clear();
        self.timer = Instant::now();
    }
}

Batching trade-offs:
- Increased throughput (amortized network cost)
- Increased latency (batch wait time)
- Good for high-throughput workloads
- Bad for latency-sensitive workloads
```

---

*This document is part of the TigerBeetle exploration series. See [exploration.md](./exploration.md) for the complete index.*
