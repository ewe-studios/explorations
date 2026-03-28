---
title: "Actor Model Deep Dive"
subtitle: "Message passing, mailboxes, and supervision"
parent: exploration.md
---

# Actor Model Deep Dive

## Introduction

This document provides a comprehensive deep dive into the actor model of concurrency, covering message passing, mailboxes, supervision trees, and distribution patterns.

---

## Part 1: Actor Fundamentals

### What is an Actor?

An **Actor** is an independent unit of computation that:
1. Has **isolated state** (no shared memory)
2. Communicates via **messages**
3. Processes messages **sequentially**
4. Can **spawn** other actors
5. Can **supervise** child actors

```
┌─────────────────────────┐
│         Actor            │
│                         │
│  ┌───────────────────┐  │
│  │    Mailbox        │  │
│  │  [M1][M2][M3]    │  │
│  └─────────┬─────────┘  │
│            │            │
│            ▼            │
│  ┌───────────────────┐  │
│  │    Behavior       │  │
│  │  (message handler)│  │
│  └─────────┬─────────┘  │
│            │            │
│            ▼            │
│  ┌───────────────────┐  │
│  │     State         │  │
│  │  (private data)   │  │
│  └───────────────────┘  │
└─────────────────────────┘
```

### Actor Lifecycle

```
┌───────────┐
│  Created  │
└─────┬─────┘
      │ start()
      ▼
┌───────────┐     message     ┌───────────┐
│  Idle     │ ───────────────►│ Processing│
└─────┬─────┘                 └─────┬─────┘
      │                             │
      │◄────────────────────────────┘
      │ handle complete
      │
      │ stop()
      ▼
┌───────────┐
│ Stopped   │
└───────────┘
```

---

## Part 2: Message Passing

### Message Types

```rust
// Request-Response pattern
enum Message {
    GetCount { reply_to: Sender<u64> },
    Increment { amount: u32 },
}

// Fire-and-forget pattern
enum Message {
    Log { level: Level, msg: String },
    Shutdown,
}

// Stream pattern
enum Message {
    Data { items: Vec<Data> },
    EndOfStream,
}
```

### Actor Implementation

```rust
use tokio::sync::mpsc;

struct CounterActor {
    count: u64,
    mailbox: mpsc::Receiver<Message>,
}

enum Message {
    Increment(u32),
    GetCount(tokio::sync::oneshot::Sender<u64>),
    Stop,
}

impl CounterActor {
    fn new() -> Self {
        let (tx, rx) = mpsc::channel(100);
        CounterActor { count: 0, mailbox: rx }
    }

    fn handle(&mut self, msg: Message) -> bool {
        match msg {
            Message::Increment(n) => {
                self.count += n as u64;
                true  // Continue
            }
            Message::GetCount(reply) => {
                let _ = reply.send(self.count);
                true
            }
            Message::Stop => false,  // Stop
        }
    }

    async fn run(mut self) {
        while let Some(msg) = self.mailbox.recv().await {
            if !self.handle(msg) {
                break;
            }
        }
    }
}
```

---

## Part 3: Mailboxes

### Unbounded Mailbox

```rust
use tokio::sync::mpsc;

struct Mailbox<T> {
    tx: mpsc::UnboundedSender<T>,
    rx: mpsc::UnboundedReceiver<T>,
}

impl<T> Mailbox<T> {
    fn new() -> Self {
        let (tx, rx) = mpsc::unbounded_channel();
        Mailbox { tx, rx }
    }

    fn send(&self, msg: T) -> Result<(), SendError> {
        self.tx.send(msg)
    }

    async fn recv(&mut self) -> Option<T> {
        self.rx.recv().await
    }
}
```

### Bounded Mailbox

```rust
use tokio::sync::mpsc;

struct BoundedMailbox<T> {
    tx: mpsc::Sender<T>,
    rx: mpsc::Receiver<T>,
    capacity: usize,
}

impl<T> BoundedMailbox<T> {
    fn new(capacity: usize) -> Self {
        let (tx, rx) = mpsc::channel(capacity);
        BoundedMailbox { tx, rx, capacity }
    }

    async fn send(&self, msg: T) -> Result<(), SendError> {
        self.tx.send(msg).await
    }
}
```

### Priority Mailbox

```rust
use std::collections::BTreeMap;

struct PriorityMailbox<T> {
    queues: BTreeMap<u8, VecDeque<T>>,  // Higher = more priority
}

impl<T> PriorityMailbox<T> {
    fn send(&mut self, msg: T, priority: u8) {
        self.queue.entry(priority)
            .or_insert_with(VecDeque::new)
            .push_back(msg);
    }

    fn recv(&mut self) -> Option<T> {
        // Get highest priority non-empty queue
        for (_, queue) in self.queues.iter_mut().rev() {
            if let Some(msg) = queue.pop_front() {
                return Some(msg);
            }
        }
        None
    }
}
```

---

## Part 4: Supervision

### Supervision Tree

```
        Supervisor (root)
       /     |     \
   Worker1 Worker2 Worker3
    /  \          /   \
  Sub1 Sub2     Sub3  Sub4
```

### Supervision Strategies

```rust
enum SupervisionStrategy {
    /// Restart the failed actor
    Restart,

    /// Stop the actor permanently
    Stop,

    /// Escalate to parent supervisor
    Escalate,

    /// Resume without restart (lose state)
    Resume,
}
```

### Supervisor Implementation

```rust
struct Supervisor {
    children: HashMap<ActorId, ChildHandle>,
    strategy: SupervisionStrategy,
}

impl Supervisor {
    fn spawn_child(&mut self, props: ActorProps) -> ActorId {
        let id = ActorId::generate();
        let handle = ChildHandle::spawn(props);
        self.children.insert(id, handle);
        id
    }

    fn handle_failure(&mut self, child_id: ActorId, error: Error) {
        match self.strategy {
            SupervisionStrategy::Restart => {
                self.children[child_id].restart();
            }
            SupervisionStrategy::Stop => {
                self.children.remove(&child_id);
            }
            SupervisionStrategy::Escalate => {
                // Notify parent supervisor
            }
            SupervisionStrategy::Resume => {
                // Continue with new state
            }
        }
    }
}
```

---

## Part 5: Distribution Patterns

### Location Transparency

```rust
// Same API for local and remote actors
trait ActorRef<T> {
    fn tell(&self, msg: T) -> Result<(), SendError>;
    fn ask<R>(&self, msg: T) -> Result<R, AskError>;
}

// Local implementation
struct LocalActorRef<T> {
    mailbox: mpsc::Sender<T>,
}

// Remote implementation
struct RemoteActorRef<T> {
    address: ActorAddress,
    serializer: Box<dyn Serializer<T>>,
}
```

### Sharding

```
Actor Type: "User"
┌─────────────┬─────────────┬─────────────┐
│  Shard 0    │  Shard 1    │  Shard 2    │
│  [U0,U3]    │  [U1,U4]    │  [U2,U5]    │
└─────────────┴─────────────┴─────────────┘
     Node A       Node B       Node C
```

```rust
struct ShardManager {
    shards: HashMap<ShardId, ActorRef>,
    nodes: Vec<NodeId>,
}

impl ShardManager {
    fn get_shard(&self, actor_id: &str) -> &ActorRef {
        let shard_id = self.hash(actor_id) % self.shards.len();
        &self.shards[&shard_id]
    }

    fn hash(&self, key: &str) -> usize {
        // Consistent hashing
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut h = DefaultHasher::new();
        key.hash(&mut h);
        h.finish() as usize
    }
}
```

---

## Part 6: Actor Frameworks

### Akka-style (Scala/Java)

```scala
class CounterActor extends Actor {
  var count = 0

  def receive = {
    case Increment(n) => count += n
    case GetCount => sender() ! count
  }
}
```

### Rust Actix

```rust
use actix::prelude::*;

struct Counter {
    count: u64,
}

impl Actor for Counter {
    type Context = Context<Self>;
}

struct Increment(u32);

impl Handler<Increment> for Counter {
    type Result = ();

    fn handle(&mut self, msg: Increment, _ctx: &mut Self::Context) {
        self.count += msg.0 as u64;
    }
}
```

---

*Continued with more patterns and implementation details...*
