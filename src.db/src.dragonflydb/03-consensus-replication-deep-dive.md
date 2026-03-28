---
title: "Replication and Consensus Deep Dive: DragonflyDB"
subtitle: "Master-replica replication and consistency models"
location: /home/darkvoid/Boxxed/@dev/repo-expolorations/src.db/src.dragonflydb
related: 00-zero-to-db-engineer.md, 01-storage-engine-deep-dive.md, 02-query-execution-deep-dive.md, exploration.md
---

# 03 - Replication and Consensus Deep Dive: DragonflyDB

## Overview

This document explains DragonflyDB's replication architecture, consistency guarantees, and cluster mode implementation.

## Part 1: Replication Topologies

### Supported Topologies

```
1. Single Master → Single Replica
┌────────┐         ┌────────┐
│ Master │ ──────> │ Replica│
│ 16GB  │  Stream  │ 16GB   │
└────────┘         └────────┘


2. Single Master → Multiple Replicas
┌────────┐
│ Master │ ──────┬─────────────┬─────────────┐
│ 16GB   │       │             │             │
└────────┘    ┌───┴───┐    ┌───┴───┐    ┌───┴───┐
              │Replica│    │Replica│    │Replica│
              │ 16GB  │    │ 16GB  │    │ 16GB  │
              └───────┘    └───────┘    └───────┘


3. Master → Replica → Replica (Chained)
┌────────┐     ┌────────┐     ┌────────┐
│ Master │ ──> │  R1    │ ──> │  R2    │
└────────┘     └────────┘     └────────┘


4. Emulated Cluster Mode (Client-side sharding)
┌─────────┐     ┌─────────┐     ┌─────────┐
│ Shard 0 │     │ Shard 1 │     │ Shard 2 │
│ Master  │     │ Master  │     │ Master  │
└────┬────┘     └────┬────┘     └────┬────┘
     │              │              │
┌────┴────┐     ┌────┴────┐     ┌────┴────┐
│ Replica │     │ Replica │     │ Replica │
└─────────┘     └─────────┘     └─────────┘
```

### What Dragonfly Does NOT Support (as of 2024)

```
❌ Multi-Master Replication (Active-Active)
   - No conflict resolution for concurrent writes
   - No CRDT implementation

❌ True Distributed Cluster Mode
   - Each node is still a standalone Dragonfly instance
   - Cluster mode is "emulated" for client compatibility

❌ Automatic Failover
   - Requires external orchestration (Kubernetes, Sentinel-like)
```

## Part 2: Replication Protocol

### Full Sync Phase

```
When a replica connects to master:

Replica                          Master
   │                               │
   │────── PING ─────────────────>│
   │<────── PONG ─────────────────│
   │                               │
   │────── REPLCONF listening-port ──────>│
   │<────── OK ──────────────────│
   │                               │
   │────── PSYNC ? -1 ───────────>│
   │                               │
   │<────── FULLSYNC ─────────────│
   │     (starts sending RDB)      │
   │                               │
   │<────── RDB Stream ───────────│
   │     (N files for N shards)    │
   │                               │
   │  (load RDB into memory)       │
   │                               │
   │<────── Journal Stream ───────│
   │     (incremental changes)     │
   │                               │
```

### RDB Stream Format

```
Master with 4 shards sends 4 RDB files:

┌─────────────────────────────────────────────────────────────┐
│                    RDB Stream Header                        │
│  Magic: "DFLY" (Dragonfly-specific)                        │
│  Version: 9                                                 │
│  Num Shards: 4                                              │
└─────────────────────────────────────────────────────────────┘
         │
         ▼
┌─────────────────────────────────────────────────────────────┐
│              Shard 0 RDB (Bucket Format)                    │
│  ┌─────────────────────────────────────────────────────┐   │
│  │ Bucket 0: [Entry][Entry][Entry]                     │   │
│  │ Bucket 1: [Entry][Entry]                            │   │
│  │ ...                                                 │   │
│  │ Bucket N: [Entry]                                   │   │
│  └─────────────────────────────────────────────────────┘   │
│  CRC64 checksum                                             │
└─────────────────────────────────────────────────────────────┘
         │
         ▼
┌─────────────────────────────────────────────────────────────┐
│              Shard 1 RDB (Bucket Format)                    │
└─────────────────────────────────────────────────────────────┘
         │
         ▼
┌─────────────────────────────────────────────────────────────┐
│              Shard 2 RDB (Bucket Format)                    │
└─────────────────────────────────────────────────────────────┘
         │
         ▼
┌─────────────────────────────────────────────────────────────┐
│              Shard 3 RDB (Bucket Format)                    │
└─────────────────────────────────────────────────────────────┘
         │
         ▼
┌─────────────────────────────────────────────────────────────┐
│                    RDB Stream Footer                        │
│  EOF marker                                                 │
│  Final CRC64                                                │
└─────────────────────────────────────────────────────────────┘
```

### Journal Stream (Incremental Changes)

```
After RDB load, master streams journal entries:

Journal Entry Format:
┌────────────────────────────────────────────────┐
│ LSN (Log Sequence Number) - 8 bytes            │
│ Operation Type - 1 byte                        │
│   - 0x01: INSERT                               │
│   - 0x02: DELETE                               │
│   - 0x03: UPDATE                               │
│   - 0x04: EXPIRE                               │
│ Timestamp - 8 bytes                            │
│ Shard ID - 2 bytes                             │
│ Key Length - 4 bytes                           │
│ Key Data - variable                            │
│ Value Length - 4 bytes                         │
│ Value Data - variable (for INSERT/UPDATE)      │
│ CRC32 - 4 bytes                                │
└────────────────────────────────────────────────┘

Example stream:
LSN=1000, OP=INSERT, Shard=0, Key="user:123", Value="{name:Alice}"
LSN=1001, OP=INSERT, Shard=2, Key="user:456", Value="{name:Bob}"
LSN=1002, OP=UPDATE, Shard=0, Key="user:123", Value="{name:Alice2}"
LSN=1003, OP=DELETE, Shard=2, Key="user:456"
```

### Consistency During Full Sync

```
Problem: Master continues receiving writes during RDB snapshot

Solution: Version-based point-in-time consistency

Master Shard State:
struct ShardState {
    epoch: u64,  // Incremented on each write
    cut_epoch: u64,  // Snapshot cut point
}

Snapshot Fiber:
for entry in table.iterate() {
    if entry.version <= cut_epoch {
        // Entry hasn't been modified since snapshot started
        serialize(entry);
        entry.version = cut_epoch + 1;  // Mark as serialized
    }
}

OnWrite Hook (concurrent writes):
on_write(entry, new_value) {
    if entry.version <= cut_epoch {
        // Entry was already snapshotted, send delta
        journal_stream.write(UPDATE, entry.key, new_value);
    }
    entry.value = new_value;
    entry.version = shard.epoch++;
}

Result:
- Replica gets consistent snapshot at cut_epoch
- All writes during snapshot are journaled
- Replay order: RDB → Journal deltas
```

## Part 3: Replica Implementation

### Replica State Machine

```rust
enum ReplicaState {
    /// Initial state before connection
    PreInit,

    /// Connecting to master
    Connecting,

    /// Handshake complete, syncing
    Sync {
        rdb_files_received: u32,
        rdb_files_total: u32,
    },

    /// Full sync complete, catching up
    CatchUp {
        current_lsn: u64,
        master_lsn: u64,
    },

    /// Fully synced, stable replication
    Stable {
        last_ack_time: Instant,
        replication_offset: u64,
    },

    /// Error state
    Error {
        error: ReplicaError,
        retry_after: Duration,
    },
}
```

### Replica Data Flow

```rust
struct Replica {
    /// Master connection
    master_conn: TcpStream,

    /// Local shards
    local_shards: Vec<Shard>,

    /// Journal executor
    journal_executor: JournalExecutor,

    /// Current state
    state: ReplicaState,

    /// Ack thread (sends ACK to master)
    ack_thread: Option<JoinHandle<()>>,
}

impl Replica {
    fn run_sync(&mut self) -> Result<(), ReplicaError> {
        self.state = ReplicaState::Connecting;

        // 1. Handshake
        self.handshake()?;

        // 2. Receive RDB stream
        let num_shards = self.receive_rdb_stream()?;

        self.state = ReplicaState::Sync {
            rdb_files_received: 0,
            rdb_files_total: num_shards,
        };

        // 3. Load RDB files into shards
        for (shard_idx, rdb_data) in self.rdb_files.drain(..) {
            self.local_shards[shard_idx].load_rdb(rdb_data)?;
            self.state.rdb_files_received += 1;
        }

        // 4. Start journal streaming
        self.state = ReplicaState::CatchUp {
            current_lsn: 0,
            master_lsn: self.master_lsn,
        };

        // 5. Process journal entries
        while let Some(entry) = self.journal_stream.next()? {
            self.journal_executor.apply(entry)?;

            if self.current_lsn >= self.master_lsn {
                break;
            }
        }

        // 6. Stable replication
        self.state = ReplicaState::Stable {
            last_ack_time: Instant::now(),
            replication_offset: self.current_lsn,
        };

        // 7. Start ack thread
        self.ack_thread = Some(spawn(|| self.send_ack_loop()));

        Ok(())
    }
}
```

### Journal Executor

```rust
struct JournalExecutor {
    shards: Vec<Shard>,
    pending_transactions: HashMap<u64, Transaction>,
}

impl JournalExecutor {
    fn apply(&mut self, entry: JournalEntry) -> Result<(), ApplyError> {
        match entry.op {
            JournalOp::SingleShard { shard_id, op } => {
                self.apply_to_shard(shard_id, op)
            }
            JournalOp::MultiShard { tx_id, ops } => {
                // Multi-shard transaction
                let tx = self.pending_transactions
                    .entry(tx_id)
                    .or_insert_with(|| Transaction::new(tx_id));

                for (shard_id, op) in ops {
                    tx.add_operation(shard_id, op);
                }

                if tx.is_complete() {
                    // All shards received, execute
                    let tx = self.pending_transactions.remove(&tx_id).unwrap();
                    self.execute_transaction(tx)
                } else {
                    Ok(())  // Waiting for more entries
                }
            }
        }
    }

    fn apply_to_shard(&mut self, shard_id: usize, op: ShardOp) -> Result<(), ApplyError> {
        let shard = &mut self.shards[shard_id];

        match op {
            ShardOp::Insert { key, value } => {
                shard.dashtable.insert(&key, &value);
            }
            ShardOp::Delete { key } => {
                shard.dashtable.delete(&key);
            }
            ShardOp::Update { key, value } => {
                shard.dashtable.insert(&key, &value);
            }
            ShardOp::Expire { key, expiry_ms } => {
                shard.set_expiry(&key, expiry_ms);
            }
        }

        Ok(())
    }

    fn execute_transaction(&mut self, tx: Transaction) -> Result<(), ApplyError> {
        // Execute operations in order
        for (shard_id, op) in tx.operations {
            self.apply_to_shard(shard_id, op)?;
        }
        Ok(())
    }
}
```

## Part 4: Consistency Models

### Consistency Levels

```
┌─────────────────────────────────────────────────────────────┐
│                     Strong Consistency                       │
│                                                              │
│  Client ──> Master ──> (wait for replica ack) ──> Response  │
│                                                              │
│  Guarantees:                                                 │
│  - Read-your-writes                                          │
│  - Linearizable reads                                        │
│  - Replicas always up-to-date                                │
│                                                              │
│  Trade-offs:                                                 │
│  - Higher write latency                                      │
│  - Reduced availability during network partitions            │
└─────────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────────┐
│                    Eventual Consistency                      │
│                                                              │
│  Client ──> Master ──> Response (async replication)         │
│                    │                                         │
│                    └──> Replica (eventually)                 │
│                                                              │
│  Guarantees:                                                 │
│  - High availability                                         │
│  - Low write latency                                         │
│  - Replicas converge eventually                              │
│                                                              │
│  Trade-offs:                                                 │
│  - Stale reads possible                                      │
│  - Read-your-writes not guaranteed on replicas               │
└─────────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────────┐
│               Read-After-Write Consistency                   │
│                                                              │
│  Client ──> Master ──> Response                             │
│     │                                                         │
│     └────────────> Replica (with read-your-writes token)     │
│                                                              │
│  Guarantees:                                                 │
│  - Client sees own writes immediately                        │
│  - Can read from replicas after own writes                   │
│                                                              │
│  Implementation:                                             │
│  - Track LSN/timestamp of client's last write                │
│  - When reading from replica, wait until replica caught up   │
└─────────────────────────────────────────────────────────────┘
```

### Dragonfly's Default Consistency

```
Dragonfly uses eventual consistency for replication:

Write Flow:
Client ──> Master (write) ──> ACK to Client
               │
               └──> Journal Stream ──> Replica (async)

Replica Lag:
- Typical: <1ms in same datacenter
- Cross-region: Depends on network latency
- Under load: Can grow to seconds

Monitoring:
INFO REPLICATION shows:
  master_repl_offset: 123456789
  slave_repl_offset:  123456000
  lag: 789 bytes behind
```

## Part 5: Failure Scenarios

### Master Failure

```
Scenario: Master crashes during replication

┌────────┐         ┌────────┐
│ Master │  CRASH  │ Replica│
│   💥   │ ──────> │        │
└────────┘         └────────┘

Replica behavior:
1. Detect connection loss
2. Enter Error state with retry timer
3. Attempt reconnection every 5 seconds
4. On reconnect, request PSYNC with last LSN

PSYNC Flow:
Replica ──> NewMaster: PSYNC <replica_id> <last_lsn>
NewMaster ──> Replica:
  - CONTINUE (if has journal from last_lsn)
  - FULLSYNC (if journal expired/missing)
```

### Network Partition

```
Scenario: Network partition between master and replica

┌────────┐     ╔═══════╗     ┌────────┐
│ Master │     ║ PARTI ║     │ Replica│
│   ●    │─────║  TION ║────│   ●    │
└────────┘     ╚═══════╝     └────────┘

During partition:
- Master continues accepting writes
- Replica stops receiving updates
- Replica marks itself as stale
- Clients to replica see stale data

After partition heals:
- Replica reconnects
- Requests PSYNC with last known LSN
- If master has journal, partial sync
- If journal expired, full sync required
```

### Replica Failure

```
Scenario: Replica crashes

┌────────┐         ┌────────┐
│ Master │         │ Replica│
│   ●    │         │   💥   │
└────────┘         └────────┘
     │
     │ (no ack from replica)
     ▼

Master behavior:
- Continues normal operation
- Removes replica from connected replicas list
- Stops sending journal stream
- Other replicas unaffected

Recovery:
- Restart replica
- Replica initiates full sync
- Catches up from beginning
```

## Part 6: Cluster Mode (Emulated)

### How Emulated Cluster Works

```
Redis Cluster Protocol Compatibility:

Dragonfly with --cluster_mode=emulated:

1. Responds to CLUSTER SLOTS command
   ┌─────────────────────────────────────────────────────────┐
   │  CLUSTER SLOTS Response:                                │
   │  [                                                      │
   │    [0, 16383, ["127.0.0.1", 6379, "node_id"]],  # Slot  │
   │    ...                                                  │
   │  ]                                                      │
   │                                                         │
   │  All slots point to same node (single shard)            │
   └─────────────────────────────────────────────────────────┘

2. Supports MOVED redirection
   ┌─────────────────────────────────────────────────────────┐
   │  Client sends: GET {user:123}:profile                   │
   │  Slot: hash("user:123") % 16384 = 4567                  │
   │                                                         │
   │  Response: -MOVED 4567 127.0.0.1:6379                   │
   │                                                         │
   │  Client updates slot map and retries                    │
   └─────────────────────────────────────────────────────────┘

3. Multi-key operations with hash tags
   ┌─────────────────────────────────────────────────────────┐
   │  MGET {user:123}:name {user:123}:email {user:123}:age  │
   │  All keys hash to same slot (due to {user:123} tag)    │
   │                                                         │
   │  Dragonfly executes normally (single shard)            │
   │                                                         │
   │  MGET {user:123}:name {user:456}:name                  │
   │  Keys hash to different slots                           │
   │                                                         │
   │  Error: CROSSSLOT Keys not in same slot                │
   └─────────────────────────────────────────────────────────┘
```

### True Cluster Mode (Future)

```
Planned true cluster mode architecture:

┌─────────────────────────────────────────────────────────────┐
│                    Dragonfly Cluster                        │
├─────────────────────────────────────────────────────────────┤
│                                                             │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐        │
│  │   Node 1    │  │   Node 2    │  │   Node 3    │        │
│  │ Shards 0-3  │  │ Shards 4-7  │  │ Shards 8-11 │        │
│  │ 16384 slots │  │ 16384 slots │  │ 16384 slots │        │
│  │   (0-5460)  │  │ (5461-10922)│  │(10923-16383)│        │
│  └──────┬──────┘  └──────┬──────┘  └──────┬──────┘        │
│         │                │                │                 │
│         └────────────────┼────────────────┘                 │
│                          │                                  │
│                  Gossip Protocol                              │
│                  (Node discovery, slot ownership)            │
│                                                             │
└─────────────────────────────────────────────────────────────┘

Features needed:
- Distributed slot table
- Inter-node gossip protocol
- Cross-node transaction coordination
- Automatic resharding
- Node failover
```

## Part 7: Rust Implementation (Valtron-style)

### Replication Task

```rust
/// Replication task for Valtron executor
struct ReplicationTask {
    state: ReplicationState,
    config: ReplicationConfig,
    journal_stream: Vec<u8>,
    rdb_files: Vec<Vec<u8>>,
}

enum ReplicationState {
    Init,
    Handshaking,
    ReceivingRdb { received: u32, total: u32 },
    LoadingRdb { current_file: usize },
    CatchingUp { current_lsn: u64, target_lsn: u64 },
    Stable,
    Error { retry_after: Duration },
}

enum ReplicationEffect {
    Connect { host: String, port: u16 },
    SendHandshake { port: u16 },
    ReceiveRdbChunk,
    LoadRdb { shard: usize, data: Vec<u8> },
    ProcessJournal { entries: Vec<JournalEntry> },
    SendAck { lsn: u64 },
    Sleep(Duration),
}

impl Task for ReplicationTask {
    type Output = ReplicationStats;
    type Effect = ReplicationEffect;

    fn next(&mut self) -> TaskResult<Self::Output, Self::Effect> {
        match &mut self.state {
            ReplicationState::Init => {
                self.state = ReplicationState::Handshaking;
                TaskResult::Effect(ReplicationEffect::Connect {
                    host: self.config.master_host.clone(),
                    port: self.config.master_port,
                })
            }

            ReplicationState::Handshaking => {
                // Send handshake commands
                self.state = ReplicationState::ReceivingRdb {
                    received: 0,
                    total: self.config.num_shards,
                };
                TaskResult::Effect(ReplicationEffect::SendHandshake {
                    port: self.config.replica_port,
                })
            }

            ReplicationState::ReceivingRdb { received, total } => {
                // Receive RDB chunks
                match self.rdb_files.pop() {
                    Some(chunk) => {
                        *received += 1;
                        TaskResult::Effect(ReplicationEffect::LoadRdb {
                            shard: *received as usize,
                            data: chunk,
                        })
                    }
                    None if *received >= *total => {
                        self.state = ReplicationState::CatchingUp {
                            current_lsn: 0,
                            target_lsn: self.config.master_lsn,
                        };
                        TaskResult::Continue
                    }
                    None => TaskResult::Effect(ReplicationEffect::ReceiveRdbChunk),
                }
            }

            ReplicationState::CatchingUp { current_lsn, target_lsn } => {
                if *current_lsn >= *target_lsn {
                    self.state = ReplicationState::Stable;
                    return TaskResult::Continue;
                }

                // Process journal entries
                TaskResult::Effect(ReplicationEffect::ProcessJournal {
                    entries: self.read_journal_batch(),
                })
            }

            ReplicationState::Stable => {
                // Send periodic ACK
                TaskResult::Effect(ReplicationEffect::SendAck {
                    lsn: self.current_lsn,
                })
            }

            ReplicationState::Error { retry_after } => {
                TaskResult::Effect(ReplicationEffect::Sleep(*retry_after))
            }

            _ => TaskResult::Continue,
        }
    }
}
```

---

*This document is part of the DragonflyDB exploration series. See [exploration.md](./exploration.md) for the complete index.*
