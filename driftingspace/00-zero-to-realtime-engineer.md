---
title: "Zero to Real-Time Synchronization Engineer"
subtitle: "First principles of real-time state synchronization"
prerequisites: None
next: [01-aper-architecture-deep-dive.md](01-aper-architecture-deep-dive.md)
---

# Zero to Real-Time Synchronization Engineer

This document takes you from zero knowledge of real-time synchronization to understanding the fundamentals needed to build collaborative applications with Aper.

## Table of Contents

1. [What is State Synchronization?](#1-what-is-state-synchronization)
2. [Centralized vs Peer-to-Peer Architectures](#2-centralized-vs-peer-to-peer-architectures)
3. [Deterministic State Machines](#3-deterministic-state-machines)
4. [Transition-Based Updates](#4-transition-based-updates)
5. [CRDTs vs State Machines](#5-crdts-vs-state-machines)
6. [Your First State Machine](#6-your-first-state-machine)

---

## 1. What is State Synchronization?

**State synchronization** is the process of keeping data consistent across multiple computers or browsers in real-time.

### The Problem

Imagine you're building a collaborative whiteboard application:

```
User A (Browser 1)          User B (Browser 2)
     ┌─────────┐                 ┌─────────┐
     │  Draw   │                 │  Draw   │
     │  Circle │                 │  Square │
     └────┬────┘                 └────┬────┘
          │                           │
          ▼                           ▼
     ┌─────────────────────────────────────┐
     │         What should each            │
     │         user see?                   │
     └─────────────────────────────────────┘
```

Without synchronization:
- User A sees: Circle only
- User B sees: Square only

With synchronization:
- User A sees: Circle AND Square
- User B sees: Circle AND Square

### Real-World Examples

| Application | What's Synchronized |
|-------------|---------------------|
| Google Docs | Document text, cursor positions |
| Figma | Shapes, colors, layers |
| Multiplayer Games | Player positions, game state |
| Collaborative Whiteboard | Drawings, sticky notes |
| Live Dashboards | Charts, metrics, alerts |

### Key Challenges

1. **Network Latency**: Messages take time to travel (10ms - 500ms)
2. **Out-of-Order Delivery**: Messages may arrive in different order
3. **Concurrent Modifications**: Two users editing the same thing
4. **Disconnections**: Users lose connection and reconnect
5. **Conflicts**: Incompatible changes from different users

---

## 2. Centralized vs Peer-to-Peer Architectures

There are two main approaches to synchronization:

### Centralized (Client-Server)

```
     Client 1 ──┐
     Client 2 ──┼──► Server (Authoritative) ──► True State
     Client 3 ──┘
```

**How it works:**
1. Clients send changes to server
2. Server validates and applies changes
3. Server broadcasts changes to all clients
4. All clients converge to same state

**Pros:**
- Simple conflict resolution (server decides)
- Easy to add validation/authorization
- Single source of truth

**Cons:**
- Server is a bottleneck
- Server is a single point of failure
- Higher latency (must go through server)

**Aper uses this model.**

### Peer-to-Peer (P2P)

```
     Client 1 ◄──► Client 2
        ▲           ▲
        └────►◄─────┘
          Client 3
```

**How it works:**
1. Clients broadcast changes directly
2. Each client maintains own state
3. Clients reconcile incoming changes

**Pros:**
- No server bottleneck
- Direct communication (lower latency)
- No single point of failure

**Cons:**
- Complex conflict resolution
- Harder to secure
- Each peer must handle all logic

**CRDTs are designed for P2P.**

---

## 3. Deterministic State Machines

A **state machine** is a system that:
1. Has a well-defined current state
2. Can transition to new states based on inputs
3. Produces the same result every time for the same input

### Determinism is Critical

For synchronization to work, state machines must be **deterministic**:

```rust
// DETERMINISTIC: Same input always produces same output
fn apply(state: State, transition: Add) -> State {
    State { value: state.value + transition.amount }
}
// Add(5) to State(10) always = State(15)

// NON-DETERMINISTIC: DO NOT DO THIS
fn apply(state: State, transition: AddRandom) -> State {
    State { value: state.value + random_number() }
}
// AddRandom() to State(10) = ??? (different on each machine!)
```

### Why Determinism Matters

```
Time 0: Client A and Client B both have State(10)

Client A: apply(Add(5)) = State(15) ✓
Client B: apply(Add(5)) = State(15) ✓

Both clients now agree!
```

If `apply()` were non-deterministic:

```
Time 0: Client A and Client B both have State(10)

Client A: apply(Add(5)) = State(17) (random added 2)
Client B: apply(Add(5)) = State(12) (random added -3)

States diverged! Synchronization broken!
```

### The StateMachine Trait

Aper defines a trait for state machines:

```rust
pub trait StateMachine {
    type Transition: Debug + Serialize + DeserializeOwned + Clone + PartialEq;
    type Conflict: Debug + Serialize + DeserializeOwned + Clone + PartialEq;

    fn apply(&self, transition: &Self::Transition) -> Result<Self, Self::Conflict>;
}
```

- **Transition**: The type of changes this state machine accepts
- **Conflict**: The type of errors that can occur
- **apply()**: The deterministic function that updates state

---

## 4. Transition-Based Updates

Instead of sending entire state on every change, Aper sends **transitions**:

### Full State Updates (Inefficient)

```
Initial: { counter: 5, items: [...], user: {...} }

User increments counter:
Client → Server: { counter: 6, items: [...], user: {...} }
Server → Clients: { counter: 6, items: [...], user: {...} }

Wasteful! We sent unchanged data.
```

### Transition Updates (Efficient)

```
Initial: Counter(5)

User increments:
Client → Server: Increment(1)
Server → Clients: Increment(1)

Each client applies: Counter(5).apply(Increment(1)) = Counter(6)

Efficient! We only sent the change.
```

### Transition Structure

```rust
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub enum CounterTransition {
    Add(i64),
    Subtract(i64),
    Reset,
}
```

Transitions are:
- **Serializable**: Can be sent over network
- **Cloneable**: Can be replayed
- **Debuggable**: Can be logged for debugging
- **Comparable**: Can detect duplicates

### Building Transitions

```rust
impl Counter {
    pub fn increment(by: i64) -> CounterTransition {
        CounterTransition::Increment(by)
    }
}

// Usage:
let transition = counter.increment(5);
// transition is CounterTransition::Increment(5)
```

---

## 5. CRDTs vs State Machines

Both approaches solve synchronization, but differently:

### CRDTs (Conflict-Free Replicated Data Types)

**Core idea**: Operations are **commutative** (order doesn't matter)

```
Client A: op1, op2
Client B: op2, op1

Both arrive at same state because op1 ∘ op2 = op2 ∘ op1
```

**Examples:**
- G-Counter: Only increments (no decrements)
- PN-Counter: Increment and decrement (separate counters)
- OR-Set: Set with add/remove operations
- LWW-Register: Last-Writer-Wins register

**Pros:**
- Works peer-to-peer (no server needed)
- Mathematically guaranteed convergence
- Handles disconnections gracefully

**Cons:**
- Limited to commutative operations
- Can't express complex business logic
- State can grow unbounded (tombstones)

### State Machines (Aper's Approach)

**Core idea**: Central authority serializes operations

```
Client A ──┐
           ├──► Server (orders: op1, op2, op3) ──► Broadcast
Client B ──┘
```

**Pros:**
- Can express any logic (not just commutative ops)
- Simple conflict resolution (server decides)
- Efficient state representation

**Cons:**
- Requires central server
- Server is bottleneck and single point of failure

### When to Use Which

| Use CRDTs When | Use State Machines When |
|----------------|------------------------|
| P2P architecture | Client-server architecture |
| Simple data types | Complex business logic |
| Offline-first | Always-online |
| Eventual consistency OK | Strong consistency needed |

**Aper uses state machines** because:
- Most apps already use client-server
- Game logic requires complex rules
- Simpler to reason about

---

## 6. Your First State Machine

Let's build a simple counter state machine from scratch:

### Step 1: Define the State

```rust
use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct Counter {
    value: i64,
}

impl Counter {
    pub fn new(value: i64) -> Self {
        Counter { value }
    }

    pub fn value(&self) -> i64 {
        self.value
    }
}
```

### Step 2: Define the Transitions

```rust
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub enum CounterTransition {
    Set(i64),
    Increment(i64),
    Decrement(i64),
}
```

### Step 3: Implement StateMachine

```rust
use aper::{StateMachine, NeverConflict};

impl StateMachine for Counter {
    type Transition = CounterTransition;
    type Conflict = NeverConflict; // Conflicts impossible

    fn apply(&self, event: &CounterTransition) -> Result<Self, NeverConflict> {
        match event {
            CounterTransition::Set(value) => Ok(Counter { value: *value }),
            CounterTransition::Increment(amount) => Ok(Counter {
                value: self.value + amount,
            }),
            CounterTransition::Decrement(amount) => Ok(Counter {
                value: self.value - amount,
            }),
        }
    }
}
```

### Step 4: Use the State Machine

```rust
fn main() {
    let mut counter = Counter::new(0);
    println!("Initial: {}", counter.value()); // 0

    // Create and apply transitions
    counter = counter.apply(&CounterTransition::Increment(5)).unwrap();
    println!("After +5: {}", counter.value()); // 5

    counter = counter.apply(&CounterTransition::Decrement(2)).unwrap();
    println!("After -2: {}", counter.value()); // 3

    counter = counter.apply(&CounterTransition::Set(100)).unwrap();
    println!("After set 100: {}", counter.value()); // 100
}
```

### Key Takeaways

1. **State is immutable**: `apply()` returns a new state, doesn't modify self
2. **Transitions are data**: They describe what to do, not how to do it
3. **Deterministic**: Same transition on same state = same result
4. **Serializable**: Can be sent over network

---

## Exercises

### Exercise 1: Todo List State Machine

Implement a simple todo list:

```rust
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct TodoList {
    items: Vec<TodoItem>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct TodoItem {
    id: u32,
    text: String,
    done: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub enum TodoListTransition {
    AddItem { id: u32, text: String },
    ToggleItem(u32),
    RemoveItem(u32),
}

impl StateMachine for TodoList {
    type Transition = TodoListTransition;
    type Conflict = NeverConflict;

    fn apply(&self, event: &TodoListTransition) -> Result<Self, NeverConflict> {
        // TODO: Implement this
        todo!()
    }
}
```

### Exercise 2: Think About Conflicts

What conflicts could occur in a todo list? How would you handle:
- Two users adding items at the same time?
- Two users editing the same item?
- One user deleting an item another is editing?

---

## Summary

| Concept | Key Point |
|---------|-----------|
| State Synchronization | Keeping data consistent across machines |
| Centralized | Server is authoritative source of truth |
| Deterministic | Same input always produces same output |
| Transition | A description of a state change |
| StateMachine Trait | Defines apply() for deterministic updates |
| CRDTs vs State Machines | P2P commutative vs centralized ordered |

---

## Next Steps

Continue to [01-aper-architecture-deep-dive.md](01-aper-architecture-deep-dive.md) to learn about Aper's architecture in detail.
