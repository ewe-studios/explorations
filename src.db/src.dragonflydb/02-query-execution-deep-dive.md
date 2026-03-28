---
title: "Query Execution Deep Dive: DragonflyDB"
subtitle: "Command processing and VLL transaction framework"
location: /home/darkvoid/Boxxed/@dev/repo-expolorations/src.db/src.dragonflydb
related: 00-zero-to-db-engineer.md, 01-storage-engine-deep-dive.md, exploration.md
---

# 02 - Query Execution Deep Dive: DragonflyDB

## Overview

This document explains how DragonflyDB processes commands and executes transactions using the VLL (Virtual Lock Manager) framework for strict serializability.

## Part 1: Command Processing Pipeline

### Architecture Overview

```
Client Request Flow:

┌─────────────────────────────────────────────────────────────┐
│                      Client Connection                       │
│  (TCP socket, TLS if enabled)                               │
└──────────────────────┬──────────────────────────────────────┘
                       │
                       ▼
┌─────────────────────────────────────────────────────────────┐
│                   Listener Thread                           │
│  ┌─────────────────────────────────────────────────────┐    │
│  │ 1. Read bytes from socket                           │    │
│  │ 2. Parse RESP protocol                              │    │
│  │ 3. Build Command object                             │    │
│  └─────────────────────────────────────────────────────┘    │
└──────────────────────┬──────────────────────────────────────┘
                       │
                       ▼
┌─────────────────────────────────────────────────────────────┐
│                   Worker Thread (Shard)                     │
│  ┌─────────────────────────────────────────────────────┐    │
│  │ 4. Route to correct shard (hash key % num_shards)  │    │
│  │ 5. Acquire intent locks                             │    │
│  │ 6. Execute command                                  │    │
│  │ 7. Release locks                                    │    │
│  └─────────────────────────────────────────────────────┘    │
└──────────────────────┬──────────────────────────────────────┘
                       │
                       ▼
┌─────────────────────────────────────────────────────────────┐
│                   Response                                  │
│  ┌─────────────────────────────────────────────────────┐    │
│  │ 8. Format RESP response                             │    │
│  │ 9. Write to socket                                  │    │
│  └─────────────────────────────────────────────────────┘    │
└─────────────────────────────────────────────────────────────┘
```

### RESP Protocol Parsing

```
RESP (Redis Serialization Protocol) Format:

Simple Strings: +OK\r\n
Errors:        -Error message\r\n
Integers:      :1000\r\n
Bulk Strings:  $5\r\nhello\r\n
Arrays:        *2\r\n$3\r\nGET\r\n$3\r\nkey\r\n

Parser State Machine:

enum RespState {
    Start,
    Type,      // +, -, :, $, *
    Parsing,   // Reading length or content
    CRLF,      // Expecting \r\n
    Complete,
}

fn parse_resp(input: &[u8]) -> Result<RespValue, ParseError> {
    let mut state = RespState::Start;
    let mut pos = 0;

    loop {
        match state {
            RespState::Start => {
                match input[pos] {
                    b'+' => state = RespState::SimpleString,
                    b'-' => state = RespState::Error,
                    b':' => state = RespState::Integer,
                    b'$' => state = RespState::BulkString,
                    b'*' => state = RespState::Array,
                    _ => return Err(InvalidType),
                }
                pos += 1;
            }
            RespState::BulkString => {
                // Parse length
                let len = parse_integer(&input[pos..])?;
                pos += length_of_integer(len);
                pos += 2; // Skip \r\n

                // Read bulk data
                let data = &input[pos..pos + len];
                pos += len + 2; // Skip data + \r\n

                return Ok(RespValue::BulkString(data.to_vec()));
            }
            // ... other states
        }
    }
}
```

### Command Dispatch

```
After parsing, commands are dispatched to shards:

struct Command {
    name: String,      // "GET", "SET", "MSET", etc.
    args: Vec<Vec<u8>>, // Command arguments
    keys: Vec<String>, // Extracted keys for routing
    shard_mask: u64,   // Bitmap of affected shards
}

fn dispatch_command(cmd: Command, connection: &Connection) {
    // Single-key command - direct dispatch
    if cmd.keys.len() == 1 {
        let shard_idx = hash(&cmd.keys[0]) % num_shards;
        let result = shards[shard_idx].execute(cmd);
        connection.send_response(result);
        return;
    }

    // Multi-key command - coordinate across shards
    let coordinator = TransactionCoordinator::new();
    let result = coordinator.execute_multi_key(cmd);
    connection.send_response(result);
}
```

## Part 2: VLL Transaction Framework

### What is VLL?

VLL (Virtual Lock Manager) is a transactional framework based on the paper ["VLL: a lock manager redesign for main memory database systems"](https://www.cs.umd.edu/~abadi/papers/vldbj-vll.pdf).

```
VLL Goals:
1. Strict serializability for multi-key operations
2. No mutexes or spinlocks (uses message passing)
3. Parallel execution across shards
4. No rollbacks once scheduled

Key Insight:
Instead of acquiring locks and blocking, VLL:
1. Schedules all transactions in a global order
2. Executes transactions in order per shard
3. Uses intent locks to track pending operations
```

### Transaction States

```
enum TransactionState {
    /// Transaction not yet started
    Init,

    /// Being scheduled across shards
    Scheduling {
        shards_pending: HashSet<ShardId>,
        sequence_num: u64,
    },

    /// Scheduled, waiting for earlier transactions
    Scheduled {
        sequence_num: u64,
        waiting_on: Vec<u64>,  // Earlier txns with overlapping keys
    },

    /// Currently executing
    Executing {
        sequence_num: u64,
        current_hop: usize,
    },

    /// Completed
    Committed,

    /// Failed (rare - only during scheduling)
    Aborted,
}
```

### Scheduling Phase

```
Scheduling ensures total order across shards:

fn schedule_transaction(txn: &mut Transaction) -> Result<(), ScheduleError> {
    let sequence_num = GLOBAL_SEQUENCE.fetch_add(1, Ordering::SeqCst);
    txn.sequence_num = sequence_num;

    // Send schedule message to all involved shards
    let mut shards_pending = HashSet::new();
    for key in &txn.keys {
        let shard_idx = hash(key) % num_shards;
        shards_pending.insert(shard_idx);

        // Send schedule message
        shards[shard_idx].send_schedule_msg(ScheduleMsg {
            txn_id: txn.id,
            sequence_num,
            keys: txn.keys_for_shard(shard_idx),
            intent_locks: txn.get_intent_locks(shard_idx),
        });
    }

    // Wait for all shards to acknowledge
    for shard_idx in &shards_pending {
        let ack = shards[*shard_idx].wait_for_ack();

        if ack == Ack::Conflict {
            // Another txn with lower seqnum has overlapping keys
            // Abort and retry with new sequence number
            abort_scheduling(txn);
            return Err(ScheduleError::Conflict);
        }
    }

    txn.state = TransactionState::Scheduled {
        sequence_num,
        waiting_on: vec![],
    };

    Ok(())
}
```

### Intent Locks

```
Intent locks track which keys transactions will access:

struct IntentLockMap {
    /// key -> count of pending transactions
    locks: HashMap<String, u32>,
}

impl IntentLockMap {
    fn acquire(&mut self, key: &str) {
        *self.locks.entry(key.to_string()).or_insert(0) += 1;
    }

    fn release(&mut self, key: &str) {
        let count = self.locks.get_mut(key).unwrap();
        *count -= 1;
        if *count == 0 {
            self.locks.remove(key);
        }
    }

    fn is_locked(&self, key: &str) -> bool {
        self.locks.get(key).map_or(false, |&c| c > 0)
    }
}

When scheduling:
1. For each key in transaction, acquire intent lock
2. If intent lock already held by earlier txn, record conflict
3. Intent locks don't block - just track dependencies

Purpose:
- Help determine transaction ordering
- Optimize conflict detection
- Enable out-of-order execution when safe
```

### Execution Phase

```
Once scheduled, transaction executes without rollback:

fn execute_transaction(txn: &Transaction) -> Result<Value, Error> {
    assert!(matches!(txn.state, TransactionState::Scheduled { .. }));

    txn.state = TransactionState::Executing {
        sequence_num: txn.sequence_num,
        current_hop: 0,
    };

    // Execute micro-operations
    for (hop_idx, micro_op) in txn.micro_ops.iter().enumerate() {
        let is_last_hop = hop_idx == txn.micro_ops.len() - 1;

        // Send to all involved shards
        let mut results = Vec::new();
        for shard_idx in micro_op.involved_shards() {
            let result = shards[shard_idx].send_execute_msg(ExecuteMsg {
                txn_id: txn.id,
                sequence_num: txn.sequence_num,
                op: micro_op.clone(),
                is_last_hop,
            });
            results.push(result);
        }

        // Combine results from all shards
        txn.state = TransactionState::Executing {
            sequence_num: txn.sequence_num,
            current_hop: hop_idx + 1,
        };
    }

    // Transaction complete
    txn.state = TransactionState::Committed;
    Ok(txn.combine_results())
}
```

### Sequence Diagram

```
Multi-Key Transaction (MSET key1 val1 key2 val2):

Coordinator              Shard 0                Shard 1
    │                       │                      │
    │──Schedule Txn 100────>│                      │
    │──Schedule Txn 100──────────────────────────>│
    │                       │                      │
    │<──Ack OK─────────────│                      │
    │<─────────────────────Ack OK────────────────│
    │                       │                      │
    │──Execute (key1=val1)─>│                      │
    │──Execute (key2=val2)──────────────────────>│
    │                       │                      │
    │<──OK─────────────────│                      │
    │<─────────────────────OK────────────────────│
    │                       │                      │
    │──Finish Txn 100─────>│                      │
    │──Finish Txn 100────────────────────────────>│
    │                       │                      │
    │                       │ (release locks)      │ (release locks)
    │                       │ (remove from queue)  │ (remove from queue)
```

## Part 3: Multi-Key Command Implementation

### MSET Implementation

```
MSET key1 val1 key2 val2 ... keyN valN

Single micro-op that writes to all shards:

struct MsetMicroOp {
    /// Key-value pairs for each shard
    shard_data: HashMap<ShardId, Vec<(String, Vec<u8>)>>,
}

impl MicroOp for MsetMicroOp {
    fn involved_shards(&self) -> Vec<ShardId> {
        self.shard_data.keys().copied().collect()
    }

    fn execute_on_shard(&self, shard: &Shard, shard_id: ShardId) -> Value {
        let pairs = &self.shard_data[&shard_id];

        for (key, value) in pairs {
            shard.dashtable.insert(key, value);
        }

        Value::SimpleString("OK".to_string())
    }

    fn is_last_hop(&self) -> bool {
        true  // MSET completes in one hop
    }
}
```

### MGET Implementation

```
MGET key1 key2 ... keyN

Single micro-op that reads from all shards:

struct MgetMicroOp {
    keys: Vec<String>,
    key_to_shard: HashMap<String, ShardId>,
}

impl MicroOp for MgetMicroOp {
    fn execute_on_shard(&self, shard: &Shard, _shard_id: ShardId) -> Value {
        let shard_keys: Vec<&String> = self.keys.iter()
            .filter(|k| self.key_to_shard[k] == _shard_id)
            .collect();

        let mut results = Vec::new();
        for key in shard_keys {
            match shard.dashtable.get(key) {
                Some(value) => results.push(Value::Bulk(value.clone())),
                None => results.push(Value::Nil),
            }
        }

        Value::Array(results)
    }
}

Coordinator combines results:
fn combine_results(&self, shard_results: Vec<Value>) -> Value {
    // Interleave results in original key order
    let mut combined = Vec::new();

    for key in &self.original_keys {
        let shard_id = self.key_to_shard[key];
        let shard_result = &shard_results[shard_id];

        if let Value::Array(results) = shard_result {
            combined.push(results[/* index for this key */].clone());
        }
    }

    Value::Array(combined)
}
```

### RENAME Implementation

```
RENAME old_key new_key

Two micro-ops:
1. Fetch data from old_key
2. Write data to new_key, delete old_key

struct RenameMicroOp1 {
    old_key: String,
}

impl MicroOp for RenameMicroOp1 {
    fn execute_on_shard(&self, shard: &Shard, _shard_id: ShardId) -> Value {
        match shard.dashtable.get(&self.old_key) {
            Some(data) => Value::Bulk(data),
            None => Value::Error("key not found".to_string()),
        }
    }
}

struct RenameMicroOp2 {
    old_key: String,
    new_key: String,
    old_data: Vec<u8>,  // From micro-op 1
}

impl MicroOp for RenameMicroOp2 {
    fn execute_on_shard(&self, shard: &Shard, _shard_id: ShardId) -> Value {
        shard.dashtable.insert(&self.new_key, &self.old_data);
        shard.dashtable.delete(&self.old_key);
        Value::SimpleString("OK".to_string())
    }

    fn is_last_hop(&self) -> bool {
        true
    }
}
```

## Part 4: Blocking Commands (BLPOP)

### The BLPOP Challenge

```
BLPOP key1 key2 key3 timeout

Requirements:
1. Check keys in order (key1, key2, key3)
2. Pop from first non-empty list
3. If all empty, block until data available
4. When unblocked, pop from first key with data

Ordering semantics:
- If multiple clients blocked on same key, FIFO order
- If multiple keys get data simultaneously, leftmost key wins
- Must be strictly serializable
```

### BLPOP Implementation

```
struct BlpopState {
    /// Keys to check (in order)
    keys: Vec<String>,
    /// Timeout (0 = infinite)
    timeout_ms: u64,
    /// Start time for timeout calculation
    start_time: Instant,
    /// Shards involved
    involved_shards: HashSet<ShardId>,
}

enum BlpopStatus {
    /// Found non-empty list, returning result
    Ready { key: String, value: Vec<u8> },

    /// All lists empty, client is blocked
    Blocked { wake_handle: WakeHandle },

    /// Timeout expired
    Timeout,
}

fn execute_blpop(txn: &Transaction, state: &BlpopState) -> BlpopStatus {
    // Phase 1: Check all keys in order
    for key in &state.keys {
        let shard = get_shard_for_key(key);

        if let Some(list) = shard.get_list(key) {
            if !list.is_empty() {
                // Found non-empty list!
                let value = list.lpop();
                return BlpopStatus::Ready {
                    key: key.clone(),
                    value,
                };
            }
        }
    }

    // Phase 2: All empty, need to block
    // Register wake-up handles on all involved shards
    let wake_handle = WakeHandle::new();

    for key in &state.keys {
        let shard = get_shard_for_key(key);
        shard.register_blpop_waiter(key, wake_handle.clone());
    }

    BlpopStatus::Blocked { wake_handle }
}
```

### Wake-Up Mechanism

```
When LPUSH/RPUSH modifies a list:

fn lpush(key: &str, value: Vec<u8>) {
    let shard = get_shard_for_key(key);
    let list = shard.get_or_create_list(key);

    // Push value
    list.lpush(value);

    // Wake up any blocked BLPOP clients
    if let Some(waiters) = shard.get_blpop_waiters(key) {
        for waiter in waiters {
            // Only one waiter should actually get the data
            // Others will re-check and re-block
            waiter.wake();
        }
    }
}

Woken BLPOP client:
fn on_wake(blpop_state: &BlpopState) -> BlpopStatus {
    // Re-check all keys in order
    for key in &blpop_state.keys {
        let shard = get_shard_for_key(key);

        if let Some(list) = shard.get_list(key) {
            if !list.is_empty() {
                let value = list.lpop();
                return BlpopStatus::Ready {
                    key: key.clone(),
                    value,
                };
            }
        }
    }

    // Spurious wake or another client got the data
    // Re-block (unless timeout expired)
    if blpop_state.start_time.elapsed().as_millis() >= blpop_state.timeout_ms {
        BlpopStatus::Timeout
    } else {
        // Re-register wake handles
        BlpopStatus::Blocked { .. }
    }
}
```

### Multi-Transaction Interaction

```
Client executes MULTI/EXEC with BLPOP:

MULTI
  BLPOP key1 key2 0
  SET other_key value
EXEC

In Dragonfly:
1. Entire MULTI block is one transaction
2. BLPOP blocking is disabled inside transactions
3. Returns nil if no data available (like Redis)

fn execute_multi_block(commands: Vec<Command>) -> Result<Vec<Value>, Error> {
    // Check if any command would block
    for cmd in &commands {
        if cmd.is_blocking() {
            cmd.set_non_blocking_mode();
        }
    }

    // Execute as normal transaction
    let txn = Transaction::new(commands);
    txn.execute()
}
```

## Part 5: Command Squashing

### The Problem

```
Redis transaction with 100 commands:

MULTI
  SET key1 value1
  SET key2 value2
  ... (100 commands)
EXEC

Traditional execution:
- 100 separate hops (one per command)
- Sequential execution (one at a time)
- Latency = sum of all command latencies

With 0.1ms per command:
Total latency = 100 * 0.1ms = 10ms
```

### Command Squashing Solution

```
Observation:
If all commands access different shards, they can run in parallel!

Squashed execution:
1. Group commands by shard
2. Send all commands in one hop
3. Each shard executes its commands inline
4. Combine results

Latency = max(per-shard latency), not sum!

With 0.1ms per command, 4 shards:
Total latency ≈ 25 * 0.1ms = 2.5ms (4X improvement)
```

### Squashing Algorithm

```
fn squash_commands(commands: Vec<Command>) -> SquashedBatch {
    let mut by_shard: HashMap<ShardId, Vec<Command>> = HashMap::new();

    for cmd in commands {
        if cmd.accesses_single_shard() {
            let shard = cmd.get_shard();
            by_shard.entry(shard).or_default().push(cmd);
        } else {
            // Multi-shard command - can't squash
            // Must execute as separate hop
            by_shard.entry(ANY_SHARD).or_default().push(cmd);
        }
    }

    SquashedBatch { by_shard }
}

fn execute_squashed(batch: SquashedBatch) -> Vec<Value> {
    let mut results = Vec::new();

    // Send to all shards in parallel
    let shard_handles: Vec<_> = batch.by_shard.iter()
        .map(|(shard_id, cmds)| {
            spawn(|| {
                let shard = get_shard(*shard_id);
                let mut shard_results = Vec::new();

                for cmd in cmds {
                    shard_results.push(shard.execute(cmd));
                }

                shard_results
            })
        })
        .collect();

    // Wait for all shards
    for handle in shard_handles {
        results.extend(handle.join());
    }

    results
}
```

## Part 6: Rust Implementation (Valtron-style)

### Transaction Coordinator

```rust
/// Coordinates multi-shard transactions without async
struct TransactionCoordinator {
    /// Current transaction state
    state: CoordinatorState,
    /// Pending effects to handle
    pending_effects: Vec<ShardEffect>,
    /// Results from shards
    shard_results: HashMap<ShardId, ShardResult>,
}

enum CoordinatorState {
    Init,
    Scheduling {
        sequence_num: u64,
        shards_pending: BitSet,
    },
    Executing {
        current_hop: usize,
        shards_pending: BitSet,
    },
    Complete,
}

enum ShardEffect {
    ScheduleMsg { shard: ShardId, txn_id: u64, seq: u64 },
    ExecuteMsg { shard: ShardId, op: MicroOp, is_last: bool },
    FinishMsg { shard: ShardId, txn_id: u64 },
}

impl Task for TransactionCoordinator {
    type Output = TransactionResult;
    type Effect = ShardEffect;

    fn next(&mut self) -> TaskResult<Self::Output, Self::Effect> {
        match &mut self.state {
            CoordinatorState::Init => {
                // Allocate sequence number
                let seq = GLOBAL_SEQUENCE.fetch_add(1, Ordering::SeqCst);

                self.state = CoordinatorState::Scheduling {
                    sequence_num: seq,
                    shards_pending: BitSet::new(),
                };

                // Schedule on all involved shards
                for shard in self.involved_shards() {
                    return TaskResult::Effect(ShardEffect::ScheduleMsg {
                        shard,
                        txn_id: self.txn_id,
                        seq,
                    });
                }

                TaskResult::Continue
            }

            CoordinatorState::Scheduling { sequence_num, shards_pending } => {
                // Process scheduling acknowledgments
                // ... handle acks ...

                if shards_pending.is_empty() {
                    // All shards acknowledged, start execution
                    self.state = CoordinatorState::Executing {
                        current_hop: 0,
                        shards_pending: BitSet::new(),
                    };

                    // Send first hop
                    let op = self.micro_ops[0].clone();
                    for shard in op.involved_shards() {
                        return TaskResult::Effect(ShardEffect::ExecuteMsg {
                            shard,
                            op: op.clone(),
                            is_last: self.micro_ops.len() == 1,
                        });
                    }
                }

                TaskResult::Continue
            }

            // ... executing state ...

            CoordinatorState::Complete => {
                TaskResult::Complete(self.final_result.take().unwrap())
            }
        }
    }
}
```

### Shard Message Handler

```rust
/// Handles transaction messages on shard thread
struct ShardHandler {
    /// Pending transactions in order
    tx_queue: VecDeque<ScheduledTransaction>,
    /// Intent locks for conflict detection
    intent_locks: HashMap<String, u32>,
    /// Current sequence number
    current_seq: u64,
}

struct ScheduleMessage {
    txn_id: u64,
    sequence_num: u64,
    keys: Vec<String>,
}

impl ShardHandler {
    fn handle_schedule(&mut self, msg: ScheduleMessage) -> Ack {
        // Check for conflicts with earlier transactions
        for key in &msg.keys {
            if self.intent_locks.get(key).map_or(false, |&c| c > 0) {
                // Conflict - earlier transaction has this key
                return Ack::Conflict;
            }
        }

        // No conflicts, add to queue
        self.tx_queue.push_back(ScheduledTransaction {
            txn_id: msg.txn_id,
            sequence_num: msg.sequence_num,
            keys: msg.keys.clone(),
            state: TxState::Scheduled,
        });

        // Acquire intent locks
        for key in msg.keys {
            *self.intent_locks.entry(key).or_insert(0) += 1;
        }

        Ack::Ok
    }

    fn handle_execute(&mut self, msg: ExecuteMessage) -> Value {
        // Find transaction in queue
        let txn = self.tx_queue.iter()
            .find(|t| t.txn_id == msg.txn_id)
            .unwrap();

        // Execute micro-op
        let result = msg.op.execute(&self.storage);

        if msg.is_last_hop {
            // Release intent locks
            for key in &txn.keys {
                let count = self.intent_locks.get_mut(key).unwrap();
                *count -= 1;
                if *count == 0 {
                    self.intent_locks.remove(key);
                }
            }

            // Remove from queue
            self.tx_queue.retain(|t| t.txn_id != msg.txn_id);
        }

        result
    }
}
```

---

*This document is part of the DragonflyDB exploration series. See [exploration.md](./exploration.md) for the complete index.*
