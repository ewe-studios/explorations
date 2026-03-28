---
title: "Aper Architecture Deep Dive"
subtitle: "Understanding the StateMachine trait and core abstractions"
prerequisites: [00-zero-to-realtime-engineer.md](00-zero-to-realtime-engineer.md)
next: [02-data-structures-deep-dive.md](02-data-structures-deep-dive.md)
source: /home/darkvoid/Boxxed/@formulas/src.rust/src.driftingspace/aper/aper/src/lib.rs
---

# Aper Architecture Deep Dive

This document explores the core architecture of Aper, focusing on the StateMachine trait and how Aper enables deterministic state synchronization.

## Table of Contents

1. [The StateMachine Trait](#1-the-statemachine-trait)
2. [The StateMachine Derive Macro](#2-the-statemachine-derive-macro)
3. [NeverConflict Type](#3-neverconflict-type)
4. [Transition Patterns](#4-transition-patterns)
5. [Conflict Handling](#5-conflict-handling)
6. [Serialization Strategy](#6-serialization-strategy)

---

## 1. The StateMachine Trait

The `StateMachine` trait is the foundation of Aper. Here's the complete definition:

```rust
pub trait StateMachine: Clone + DeserializeOwned + Serialize + Debug + 'static {
    type Transition: Debug + Serialize + DeserializeOwned + Clone + PartialEq;
    type Conflict: Debug + Serialize + DeserializeOwned + Clone + PartialEq;

    fn apply(&self, transition: &Self::Transition) -> Result<Self, Self::Conflict>;
}
```

### Trait Bounds Explained

| Bound | Purpose |
|-------|---------|
| `Clone` | State must be cloneable for optimistic updates |
| `DeserializeOwned` | Can be deserialized from network messages |
| `Serialize` | Can be serialized for network transmission |
| `Debug` | Can be printed for debugging |
| `'static` | No lifetime parameters (owns all data) |

### Associated Types

**Transition**: Describes all possible changes to this state machine.

```rust
// For a counter, transitions are:
type Transition = CounterTransition;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
enum CounterTransition {
    Increment(i64),
    Decrement(i64),
    Set(i64),
}
```

**Conflict**: Describes what can go wrong when applying a transition.

```rust
// For operations that can fail:
type Conflict = AccountConflict;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
enum AccountConflict {
    InsufficientFunds,
    AccountNotFound,
    InvalidAmount,
}

// For operations that can never fail:
type Conflict = NeverConflict;
```

### The apply() Method

The `apply()` method is the heart of the state machine:

```rust
fn apply(&self, transition: &Self::Transition) -> Result<Self, Self::Conflict>;
```

**Key requirements:**

1. **Immutable**: Takes `&self`, returns new `Self`
2. **Deterministic**: Same input = same output, always
3. **Pure**: No side effects (no I/O, no random, no time)
4. **Complete**: Handle all transition variants

### Example Implementation

```rust
impl StateMachine for Counter {
    type Transition = CounterTransition;
    type Conflict = NeverConflict;

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

---

## 2. The StateMachine Derive Macro

Aper provides a derive macro that automatically implements `StateMachine` for structs with named fields.

### How It Works

```rust
use aper::StateMachine;

#[derive(StateMachine)]
pub struct GameState {
    pub score: Counter,
    pub level: Atom<u32>,
    pub player_name: Atom<String>,
}
```

The macro generates:

1. **Transition enum** with variants for each field:
```rust
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum GameStateTransform {
    ApplyScore(<Counter as StateMachine>::Transition),
    ApplyLevel(<Atom<u32> as StateMachine>::Transition),
    ApplyPlayerName(<Atom<String> as StateMachine>::Transition),
}
```

2. **Conflict enum** with variants for each field:
```rust
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum GameStateConflict {
    ScoreConflict(<Counter as StateMachine>::Conflict),
    LevelConflict(<Atom<u32> as StateMachine>::Conflict),
    PlayerNameConflict(<Atom<String> as StateMachine>::Conflict),
}
```

3. **StateMachine implementation**:
```rust
impl StateMachine for GameState {
    type Transition = GameStateTransform;
    type Conflict = GameStateConflict;

    fn apply(&self, transition: &Self::Transition) -> Result<Self, Self::Conflict> {
        match transition {
            GameStateTransform::ApplyScore(val) => {
                match self.score.apply(val) {
                    Ok(v) => {
                        let mut new_self = self.clone();
                        new_self.score = v;
                        Ok(new_self)
                    },
                    Err(e) => Err(GameStateConflict::ScoreConflict(e))
                }
            },
            // ... similar for other fields
        }
    }
}
```

4. **Accessor methods** for building transitions:
```rust
impl GameState {
    pub fn score(&self) -> &Counter { &self.score }

    pub fn map_score(&self, fun: impl FnOnce(&Counter) -> <Counter as StateMachine>::Transition)
        -> GameStateTransform {
        GameStateTransform::ApplyScore(fun(&self.score))
    }

    // ... similar for other fields
}
```

### When to Use Derive

**Use derive when:**
- Your state is a composition of other StateMachines
- You want automatic transition/conflict generation
- You're okay with the enum-based transition pattern

**Don't use derive when:**
- You need custom transition logic
- Your transitions don't map 1:1 to fields
- You want a flatter transition API

### Manual Implementation Pattern

```rust
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Player {
    pub health: u32,
    pub position: (f32, f32),
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub enum PlayerTransition {
    TakeDamage(u32),
    Heal(u32),
    MoveTo(f32, f32),
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub enum PlayerConflict {
    AlreadyDead,
    InvalidPosition,
}

impl StateMachine for Player {
    type Transition = PlayerTransition;
    type Conflict = PlayerConflict;

    fn apply(&self, transition: &PlayerTransition) -> Result<Self, PlayerConflict> {
        match transition {
            PlayerTransition::TakeDamage(amount) => {
                if self.health == 0 {
                    Err(PlayerConflict::AlreadyDead)
                } else {
                    Ok(Player {
                        health: self.health.saturating_sub(*amount),
                        position: self.position,
                    })
                }
            }
            PlayerTransition::Heal(amount) => {
                Ok(Player {
                    health: self.health.saturating_add(*amount),
                    position: self.position,
                })
            }
            PlayerTransition::MoveTo(x, y) => {
                // Validate position is within bounds
                if *x < 0.0 || *y < 0.0 {
                    Err(PlayerConflict::InvalidPosition)
                } else {
                    Ok(Player {
                        health: self.health,
                        position: (*x, *y),
                    })
                }
            }
        }
    }
}
```

---

## 3. NeverConflict Type

For state machines where conflicts are impossible, Aper provides `NeverConflict`:

```rust
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum NeverConflict {}
```

### Why NeverConflict?

An empty enum (no variants) represents an impossible value. If a function returns `Result<T, NeverConflict>`, the `Err` case can never happen.

### Usage Pattern

```rust
impl StateMachine for Counter {
    type Transition = CounterTransition;
    type Conflict = NeverConflict;

    fn apply(&self, event: &CounterTransition) -> Result<Self, NeverConflict> {
        // This always succeeds, but we must return Result
        // Match on impossible Err case:
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

### Unwrapping NeverConflict

Since `NeverConflict` can never be constructed, you can safely unwrap:

```rust
// This is safe - Err case is impossible
let new_state = state.apply(&transition).unwrap();

// Or use expect for better error messages
let new_state = state.apply(&transition).expect("NeverConflict is impossible");
```

### When to Use NeverConflict

| Use NeverConflict | Use Custom Conflict |
|-------------------|---------------------|
| Simple value types | Business logic validation |
| No preconditions | Operations can fail |
| Always succeeds | Requires validation |

---

## 4. Transition Patterns

There are several patterns for designing transitions:

### Pattern 1: Command Pattern

Each transition is a command to execute:

```rust
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub enum BankAccountTransition {
    Deposit { amount: u64, timestamp: u64 },
    Withdraw { amount: u64, timestamp: u64 },
    Transfer { to: AccountId, amount: u64 },
}
```

**Pros:**
- Clear intent
- Easy to log/audit
- Natural mapping to user actions

**Cons:**
- May carry redundant data
- Can expose internal structure

### Pattern 2: Delta Pattern

Transitions represent the change, not the intent:

```rust
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub enum InventoryTransition {
    AddItem { item_id: ItemId, quantity: u32 },
    RemoveItem { item_id: ItemId, quantity: u32 },
    SetQuantity { item_id: ItemId, quantity: u32 },
}
```

**Pros:**
- Minimal data
- Focus on state change
- Composable

**Cons:**
- Less semantic meaning
- Harder to validate intent

### Pattern 3: Event Sourcing Pattern

Transitions are immutable events:

```rust
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct TimestampedTransition<T> {
    pub timestamp: u64,
    pub actor: UserId,
    pub transition: T,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub enum DocumentTransition {
    Insert { position: usize, text: String },
    Delete { range: (usize, usize) },
    Format { range: (usize, usize), style: Style },
}
```

**Pros:**
- Full audit trail
- Can replay history
- Supports undo/redo

**Cons:**
- Larger message size
- More complex state reconstruction

### Aper's Pattern: TransitionEvent

Aper uses a timestamped event pattern:

```rust
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct TransitionEvent<T> {
    pub timestamp: DateTime<Utc>,
    pub client: Option<ClientId>,
    pub transition: T,
}
```

This pattern:
- Tracks who made the change
- Records when it happened
- Enables time-based operations (timers, delays)

---

## 5. Conflict Handling

Conflicts occur when a transition cannot be applied:

### Conflict vs Race Condition

**Race condition**: Two clients send transitions at the same time
- Handled by server ordering (not a conflict in Aper)

**Conflict**: A transition is invalid given current state
```rust
// Example: Trying to withdraw more than balance
account.apply(&Withdraw { amount: 1000 })
// Returns Err(InsufficientFunds) if balance < 1000
```

### Conflict Resolution Strategies

**Strategy 1: Reject and Notify**

```rust
match state.apply(&transition) {
    Ok(new_state) => { /* Accept */ }
    Err(conflict) => {
        // Notify client of rejection
        client.send(MessageToClient::Conflict {
            transition_number,
            conflict,
        });
    }
}
```

**Strategy 2: Automatic Compensation**

```rust
match state.apply(&Withdraw { amount }) {
    Ok(new_state) => Ok(new_state),
    Err(InsufficientFunds) => {
        // Withdraw maximum available instead
        let max = state.balance;
        state.apply(&Withdraw { amount: max })
    }
}
```

**Strategy 3: Queue for Later**

```rust
struct PendingTransition {
    transition: Transition,
    retry_count: u32,
}

// Retry when state changes
```

### Conflict Type Design

Design conflict types to be informative:

```rust
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub enum BankingConflict {
    InsufficientFunds {
        requested: u64,
        available: u64,
    },
    AccountNotFound {
        account_id: AccountId,
    },
    InvalidAmount {
        amount: i64,
        reason: String,
    },
    DailyLimitExceeded {
        attempted: u64,
        limit: u64,
        already_spent: u64,
    },
}
```

Include enough data for the client to:
1. Understand what went wrong
2. Display a helpful error message
3. Potentially auto-correct

---

## 6. Serialization Strategy

Aper uses Serde for serialization. This is critical for network transmission.

### Serialization Requirements

1. **Deterministic**: Same value = same bytes
2. **Stable**: Don't change format without versioning
3. **Efficient**: Minimize overhead

### Supported Formats

**JSON (Default for WebSocket)**
```rust
let json = serde_json::to_string(&transition).unwrap();
// {"Increment": 5}
```

**Bincode (Binary protocol)**
```rust
let bytes = bincode::serialize(&transition).unwrap();
// [1, 5, 0, 0, 0, 0, 0, 0, 0]
```

### Serialization Bounds

```rust
// Transition must be serializable
type Transition: Debug + Serialize + DeserializeOwned + Clone + PartialEq;
```

The `DeserializeOwned` bound means the type owns all its data (no references).

### Custom Serialization

Sometimes you need custom serialization:

```rust
use serde::{Serialize, Serializer};

#[derive(Clone, Debug, PartialEq)]
pub struct LargeData(pub Vec<u8>);

impl Serialize for LargeData {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        // Use base64 for binary data in JSON
        use base64::{Engine, engine::general_purpose::STANDARD};
        serializer.serialize_str(&STANDARD.encode(&self.0))
    }
}
```

### Versioning Considerations

When changing serialization format:

```rust
#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(tag = "version")]
pub enum DocumentTransition {
    #[serde(rename = "1")]
    V1 { text: String },
    #[serde(rename = "2")]
    V2 { text: String, metadata: Metadata },
}
```

---

## Summary

| Component | Purpose |
|-----------|---------|
| StateMachine Trait | Core abstraction for state + transitions |
| apply() Method | Deterministic state update function |
| Transition Type | Describes all possible changes |
| Conflict Type | Describes what can go wrong |
| NeverConflict | Empty type for impossible conflicts |
| StateMachine Derive | Auto-generates implementations |
| TransitionEvent | Timestamped, client-tagged transitions |

---

## Next Steps

Continue to [02-data-structures-deep-dive.md](02-data-structures-deep-dive.md) to learn about Aper's built-in data structures.
