---
title: "Data Structures Deep Dive"
subtitle: "Built-in state machines: Atom, List, Map, Counter, and more"
prerequisites: [01-aper-architecture-deep-dive.md](01-aper-architecture-deep-dive.md)
next: [03-sync-protocol-deep-dive.md](03-sync-protocol-deep-dive.md)
source: /home/darkvoid/Boxxed/@formulas/src.rust/src.driftingspace/aper/aper/src/data_structures/
---

# Data Structures Deep Dive

Aper provides a collection of built-in state machines that serve as building blocks for your applications. This document explores each data structure in detail.

## Table of Contents

1. [Atom: Atomic Value Replacement](#1-atom-atomic-value-replacement)
2. [AtomRc: Reference-Counted Atoms](#2-atomrc-reference-counted-atoms)
3. [Counter: Increment/Decrement](#3-counter-incrementdecrement)
4. [Constant: Immutable Values](#4-constant-immutable-values)
5. [List: CRDT-Style Ordered Lists](#5-list-crdt-style-ordered-lists)
6. [Map: Key-Value State Machines](#6-map-key-value-state-machines)
7. [Composing Data Structures](#7-composing-data-structures)

---

## 1. Atom: Atomic Value Replacement

`Atom<T>` is the simplest state machine - it holds a value that can only be completely replaced.

### Definition

```rust
#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Debug)]
pub struct Atom<T: Copy + PartialEq + Debug> {
    value: T,
}
```

### Transition Type

```rust
#[derive(Serialize, Deserialize, Clone, PartialEq, Debug)]
pub struct ReplaceAtom<T: Copy + PartialEq + Debug>(T);
```

### StateMachine Implementation

```rust
impl<T: Copy> StateMachine for Atom<T>
where
    T: 'static + Serialize + DeserializeOwned + Clone + PartialEq + Debug,
{
    type Transition = ReplaceAtom<T>;
    type Conflict = NeverConflict;

    fn apply(&self, transition_event: &Self::Transition) -> Result<Self, NeverConflict> {
        let ReplaceAtom(v) = transition_event;
        Ok(Atom::new(*v))
    }
}
```

### API

```rust
impl<T: Copy> Atom<T> {
    /// Create a new Atom with initial value
    pub fn new(initial: T) -> Self;

    /// Get the current value
    pub fn value(&self) -> &T;

    /// Create a transition to replace the value
    pub fn replace(&self, replacement: T) -> ReplaceAtom<T>;
}
```

### Usage Example

```rust
use aper::data_structures::Atom;

// Create an atom holding a u32
let mut atom = Atom::new(42);
assert_eq!(42, *atom.value());

// Replace the value
atom = atom.apply(&atom.replace(100)).unwrap();
assert_eq!(100, *atom.value());

// Works with any Copy type
let bool_atom = Atom::new(true);
let bool_atom = bool_atom.apply(&bool_atom.replace(false)).unwrap();
```

### When to Use Atom

| Use Atom When | Don't Use Atom When |
|---------------|---------------------|
| Simple value replacement | Need partial updates |
| Copy types ( Copy trait) | Non- Copy types |
| No validation needed | Need conflict detection |

### Common Patterns

**Pattern 1: Toggle Switch**
```rust
let switch = Atom::new(false);
let toggle = switch.replace(!*switch.value());
```

**Pattern 2: Selected Option**
```rust
let selected = Atom::new(None::<usize>);
let selected = selected.apply(&selected.replace(Some(3))).unwrap();
```

**Pattern 3: Numeric State**
```rust
let health = Atom::new(100u32);
let health = health.apply(&health.replace(health.value().saturating_sub(10))).unwrap();
```

---

## 2. AtomRc: Reference-Counted Atoms

`AtomRc<T>` is like `Atom` but for non- Copy types, using `Rc<T>` internally.

### Definition

```rust
#[derive(Clone, PartialEq, Debug)]
pub struct AtomRc<T> {
    value: Rc<T>,
}
```

### When to Use AtomRc

Use `AtomRc` when your value:
- Doesn't implement `Copy`
- Is expensive to clone
- Is large (String, Vec, custom structs)

### API

```rust
impl<T: PartialEq + Debug> AtomRc<T> {
    pub fn new(initial: T) -> Self;
    pub fn value(&self) -> &T;
    pub fn replace(&self, replacement: T) -> ReplaceAtomRc<T>;
}
```

### Usage Example

```rust
use aper::data_structures::AtomRc;

// Create an atom holding a String (not Copy)
let mut atom = AtomRc::new("Hello".to_string());
assert_eq!("Hello", atom.value());

// Replace with new value
atom = atom.apply(&atom.replace("World".to_string())).unwrap();
assert_eq!("World", atom.value());

// Works with complex types
let vec_atom = AtomRc::new(vec![1, 2, 3]);
let vec_atom = vec_atom.apply(&vec_atom.replace(vec![4, 5, 6])).unwrap();
```

### Custom Struct Example

```rust
#[derive(Clone, Debug, PartialEq)]
pub struct User {
    id: u32,
    name: String,
    email: String,
}

let user = AtomRc::new(User {
    id: 1,
    name: "Alice".to_string(),
    email: "alice@example.com".to_string(),
});

// Replace entire user
let updated = User {
    id: 1,
    name: "Alice".to_string(),
    email: "alice.new@example.com".to_string(),
};
let user = user.apply(&user.replace(updated)).unwrap();
```

---

## 3. Counter: Increment/Decrement

`Counter` is a state machine for integer values with increment/decrement operations.

### Definition

```rust
#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq)]
pub struct Counter {
    value: i64,
}
```

### Transition Type

```rust
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub enum CounterTransition {
    Set(i64),
    Increment(i64),
    Decrement(i64),
}
```

### StateMachine Implementation

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

### API

```rust
impl Counter {
    pub fn new(value: i64) -> Self;
    pub fn value(&self) -> i64;

    pub fn increment(by: i64) -> CounterTransition;
    pub fn decrement(by: i64) -> CounterTransition;
    pub fn set(to: i64) -> CounterTransition;
}
```

### Usage Example

```rust
use aper::data_structures::Counter;

let mut counter = Counter::new(0);

counter = counter.apply(&Counter::increment(5)).unwrap();
assert_eq!(5, counter.value());

counter = counter.apply(&Counter::decrement(2)).unwrap();
assert_eq!(3, counter.value());

counter = counter.apply(&Counter::set(100)).unwrap();
assert_eq!(100, counter.value());

// Negative values work
counter = counter.apply(&Counter::decrement(200)).unwrap();
assert_eq!(-100, counter.value());
```

### Common Patterns

**Pattern 1: Score Tracking**
```rust
#[derive(StateMachine)]
pub struct Game {
    player_score: Counter,
    enemy_score: Counter,
}
```

**Pattern 2: View Counter**
```rust
let views = Counter::default(); // Starts at 0
views.apply(&Counter::increment(1)); // +1 view
```

**Pattern 3: Inventory Count**
```rust
let inventory = Counter::new(10);
inventory.apply(&Counter::decrement(3)); // Sold 3
```

---

## 4. Constant: Immutable Values

`Constant<T>` is a state machine that can never change after creation.

### Definition

```rust
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Constant<T> {
    value: T,
}
```

### Key Characteristic

`Constant` has **no public constructor for its transition**, meaning no transitions can ever be created.

### Usage Example

```rust
use aper::data_structures::Constant;

// Create a constant
let config = Constant::new(Config {
    max_players: 10,
    game_duration: 300, // seconds
});

// Read the value
assert_eq!(10, config.value().max_players);

// No way to create a transition - this is immutable state
// There is no `config.replace()` or similar method
```

### When to Use Constant

| Use Constant When | Don't Use Constant When |
|-------------------|------------------------|
| Configuration data | State that changes |
| Metadata | User-modifiable values |
| Initial setup | Game state |

### Common Patterns

**Pattern 1: Application Configuration**
```rust
#[derive(StateMachine)]
pub struct AppState {
    config: Constant<AppConfig>,
    current_user: AtomRc<Option<User>>,
    settings: UserSettings,
}
```

**Pattern 2: Static Game Metadata**
```rust
#[derive(StateMachine)]
pub struct Game {
    metadata: Constant<GameMetadata>,
    state: PlayState,
}

pub struct GameMetadata {
    pub title: String,
    pub version: String,
    pub max_players: usize,
}
```

---

## 5. List: CRDT-Style Ordered Lists

`List<T>` is a sophisticated state machine for ordered collections that supports concurrent modifications.

### The Challenge

Ordinary lists break with concurrent modifications:

```
User A: Insert "B" at position 1    User B: Insert "C" at position 1
List: ["A"]                         List: ["A"]
Result: ["A", "B"]                  Result: ["A", "C"]

After sync: Who wins? Position 1 is ambiguous!
```

### The Solution: Fractional Indexing

List uses `ZenoIndex` for positions that can always be inserted between:

```rust
pub enum ListPosition {
    Beginning,
    End,
    AbsolutePosition(ZenoIndex),
    Before(Uuid, ZenoIndex),
    After(Uuid, ZenoIndex),
}
```

### ZenoIndex

`ZenoIndex` allows finding a position between any two adjacent items:

```
Initial: [A(0.0), B(1.0)]
Insert C between A and B: C(0.5)
Insert D between A and C: D(0.25)
Insert E between C and B: E(0.75)
Result: [A(0.0), D(0.25), C(0.5), E(0.75), B(1.0)]
```

### Definition

```rust
#[derive(Clone, PartialEq, Debug)]
pub struct List<T: StateMachine + PartialEq> {
    items: OrdMap<ZenoIndex, Uuid>,      // Position -> ID
    items_inv: OrdMap<Uuid, ZenoIndex>,  // ID -> Position
    pool: HashMap<Uuid, T>,              // ID -> Value
}
```

### Transition Type

```rust
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub enum ListOperation<T: StateMachine + PartialEq> {
    Insert(ListPosition, Uuid, T),
    Delete(Uuid),
    Move(Uuid, ZenoIndex),
    Apply(Uuid, <T as StateMachine>::Transition),
}
```

### API

```rust
impl<T: StateMachine + PartialEq> List<T> {
    pub fn new() -> Self;

    /// Append to end of list
    pub fn append(&self, value: T) -> OperationWithId<T>;

    /// Prepend to beginning
    pub fn prepend(&self, value: T) -> OperationWithId<T>;

    /// Insert at specific position
    pub fn insert(&self, location: ZenoIndex, value: T) -> OperationWithId<T>;

    /// Insert between two existing items
    pub fn insert_between(&self, id1: &Uuid, id2: &Uuid, value: T) -> OperationWithId<T>;

    /// Delete an item
    pub fn delete(&self, id: Uuid) -> ListOperation<T>;

    /// Move an item
    pub fn move_item(&self, id: Uuid, new_location: ZenoIndex) -> ListOperation<T>;

    /// Apply transition to an item
    pub fn map_item(&self, id: Uuid, fun: impl FnOnce(&T) -> T::Transition) -> ListOperation<T>;

    /// Iterate over items
    pub fn iter(&self) -> impl Iterator<Item = ListItem<T>>;
}
```

### Usage Example

```rust
use aper::data_structures::{List, Atom};

let mut list: List<Atom<String>> = List::new();

// Append items
let (id1, op1) = list.append(Atom::new("First"));
let (id2, op2) = list.append(Atom::new("Second"));

list = list.apply(&op1).unwrap();
list = list.apply(&op2).unwrap();

// Insert between
let (_id3, op3) = list.insert_between(&id1, &id2, Atom::new("Middle"));
list = list.apply(&op3).unwrap();

// Iterate
for item in list.iter() {
    println!("{}: {}", item.id, item.value.value());
}
// Output:
// <uuid1>: First
// <uuid3>: Middle
// <uuid2>: Second
```

### Concurrent Insert Example

```rust
// Two users concurrently insert at the same position

// User A's view
let mut list_a: List<Atom<String>> = List::new();
let (id_x, op_x) = list_a.append(Atom::new("X"));
list_a = list_a.apply(&op_x).unwrap();

// User B's view (same initial state)
let mut list_b: List<Atom<String>> = List::new();
let (id_x2, op_x2) = list_b.append(Atom::new("X"));
list_b = list_b.apply(&op_x2).unwrap();

// User A inserts "A" after X
let (_, op_a) = list_a.insert(
    ZenoIndex::new_after(&list_a.items_inv[&id_x]),
    Atom::new("A"),
);

// User B inserts "B" after X
let (_, op_b) = list_b.insert(
    ZenoIndex::new_after(&list_b.items_inv[&id_x2]),
    Atom::new("B"),
);

// Both operations are sent to server
// Server applies both (different fractional positions)
// Both clients receive and apply
// Final state: ["X", "A", "B"] or ["X", "B", "A"] (both valid!)
```

### ListConflict

```rust
#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
pub enum ListConflict<T: StateMachine> {
    /// No item exists with the given UUID
    ItemDoesNotExist(Uuid),
    /// Child state machine had a conflict
    ChildConflict(T::Conflict),
}
```

---

## 6. Map: Key-Value State Machines

`Map<K, V>` is a state machine for key-value mappings where values are state machines.

### Definition

```rust
#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct Map<T, V>
where
    T: Serialize + DeserializeOwned + Ord + PartialEq + Clone + Debug + 'static,
    V: StateMachine,
{
    inner: OrdMap<T, V>,
}
```

### Transition Type

```rust
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct MapTransition<T: PartialEq, V: PartialEq> {
    key: T,
    value: Option<V>,
}
```

### StateMachine Implementation

```rust
impl<T, V> StateMachine for Map<T, V>
where
    T: Serialize + DeserializeOwned + Ord + PartialEq + Clone + Debug + 'static,
    V: StateMachine + PartialEq,
{
    type Transition = MapTransition<T, V>;
    type Conflict = NeverConflict;

    fn apply(&self, transition: &Self::Transition) -> Result<Self, NeverConflict> {
        match &transition.value {
            Some(v) => {
                let mut c = self.inner.clone();
                c.insert(transition.key.clone(), v.clone());
                Ok(Map { inner: c })
            }
            None => {
                let mut c = self.inner.clone();
                c.remove(&transition.key);
                Ok(Map { inner: c })
            }
        }
    }
}
```

### API

```rust
impl<T, V> Map<T, V>
where
    T: Serialize + DeserializeOwned + Ord + PartialEq + Clone + Debug + 'static,
    V: StateMachine + PartialEq,
{
    pub fn new() -> Self;
    pub fn insert(&self, key: T, value: V) -> MapTransition<T, V>;
    pub fn delete(&self, key: T) -> MapTransition<T, V>;
    pub fn get(&self, key: &T) -> Option<&V>;
    pub fn iter(&self) -> impl Iterator<Item = (&T, &V)>;
}
```

### Usage Example

```rust
use aper::data_structures::{Map, Atom};

let mut map: Map<String, Atom<u32>> = Map::new();

// Insert values
map = map.apply(&map.insert("score".to_string(), Atom::new(0))).unwrap();
map = map.apply(&map.insert("level".to_string(), Atom::new(1))).unwrap();

// Get values
assert_eq!(0, *map.get(&"score".to_string()).unwrap().value());

// Insert returns Option<V> in transition - None means delete
map = map.apply(&map.delete("level".to_string())).unwrap();
assert!(map.get(&"level".to_string()).is_none());
```

### Common Patterns

**Pattern 1: Player Inventory**
```rust
type Inventory = Map<ItemId, Atom<u32>>; // Item ID -> Quantity

let mut inventory: Inventory = Map::new();
inventory = inventory.apply(&inventory.insert(
    ItemId::Sword,
    Atom::new(1),
)).unwrap();
```

**Pattern 2: User Sessions**
```rust
type Sessions = Map<SessionId, AtomRc<Session>>;

let sessions: Sessions = Map::new();
```

**Pattern 3: Chat Rooms**
```rust
type Rooms = Map<RoomId, RoomState>;

#[derive(StateMachine)]
pub struct RoomState {
    name: AtomRc<String>,
    members: Counter,
    messages: List<ChatMessage>,
}
```

---

## 7. Composing Data Structures

Real applications compose multiple data structures together.

### Example: Todo Application

```rust
use aper::{StateMachine, NeverConflict};
use aper::data_structures::{List, Atom, AtomRc, Counter};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct TodoItem {
    id: u32,
    text: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub enum TodoItemTransition {
    SetText(String),
    ToggleComplete,
}

#[derive(StateMachine)]
pub struct TodoItemState {
    item: Constant<TodoItem>,
    complete: Atom<bool>,
}

#[derive(StateMachine)]
pub struct TodoList {
    items: List<TodoItemState>,
    completed_count: Counter,
    filter: Atom<Filter>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub enum Filter {
    All,
    Active,
    Completed,
}
```

### Example: Collaborative Document

```rust
use aper::data_structures::{List, AtomRc, Map};

#[derive(StateMachine)]
pub struct Document {
    title: AtomRc<String>,
    content: List<TextBlock>,
    comments: Map<CommentId, Comment>,
    cursors: Map<UserId, Cursor>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct TextBlock {
    id: u32,
    text: String,
    style: TextStyle,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Cursor {
    position: usize,
    selection: Option<usize>,
}
```

### Example: Game State

```rust
use aper::data_structures::{Map, Counter, Atom, List};

#[derive(StateMachine)]
pub struct Game {
    players: Map<PlayerId, Player>,
    score: Counter,
    level: Atom<u32>,
    powerups: List<Powerup>,
    metadata: Constant<GameMetadata>,
}

#[derive(StateMachine)]
pub struct Player {
    position: Atom<(f32, f32)>,
    health: Counter,
    inventory: Map<ItemId, Atom<u32>>,
}
```

---

## Summary

| Data Structure | Use Case | Complexity |
|----------------|----------|------------|
| Atom<T> | Simple value replacement | O(1) |
| AtomRc<T> | Non-Copy value replacement | O(1) |
| Counter | Numeric increments | O(1) |
| Constant<T> | Immutable configuration | O(0) |
| List<T> | Ordered, concurrent-safe lists | O(log n) |
| Map<K,V> | Key-value state machines | O(log n) |

---

## Next Steps

Continue to [03-sync-protocol-deep-dive.md](03-sync-protocol-deep-dive.md) to learn about the synchronization protocol.
