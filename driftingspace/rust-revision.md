---
title: "Rust Revision: Driftingspace/Aper Patterns"
subtitle: "Rust implementation patterns for state machine synchronization"
prerequisites: [05-yew-frontend-deep-dive.md](05-yew-frontend-deep-dive.md)
next: [production-grade.md](production-grade.md)
---

# Rust Revision: Driftingspace/Aper Patterns

This document covers Rust-specific implementation patterns for building state machine synchronization systems like Aper.

## Table of Contents

1. [Ownership Patterns for State Machines](#1-ownership-patterns-for-state-machines)
2. [Rc for Shared State](#2-rc-for-shared-state)
3. [Serde Serialization Strategies](#3-serde-serialization-strategies)
4. [Type-Level Determinism Guarantees](#4-type-level-determinism-guarantees)
5. [Derive Macro Implementation](#5-derive-macro-implementation)
6. [CRDT Data Structures](#6-crdt-data-structures)
7. [Complete Implementation Example](#7-complete-implementation-example)

---

## 1. Ownership Patterns for State Machines

### Immutable State Updates

Aper uses immutable state updates - `apply()` takes `&self` and returns new `Self`:

```rust
pub trait StateMachine: Clone + DeserializeOwned + Serialize + Debug + 'static {
    type Transition: Debug + Serialize + DeserializeOwned + Clone + PartialEq;
    type Conflict: Debug + Serialize + DeserializeOwned + Clone + PartialEq;

    fn apply(&self, transition: &Self::Transition) -> Result<Self, Self::Conflict>;
}
```

**Why immutable?**
- Enables optimistic updates (keep old state for rollback)
- Thread-safe sharing with `Rc`
- Predictable reasoning about state
- Compatible with functional patterns

### Pattern: Clone and Modify

```rust
impl StateMachine for Counter {
    fn apply(&self, event: &CounterTransition) -> Result<Self, NeverConflict> {
        let mut new_self = self.clone();  // Clone entire state
        match event {
            CounterTransition::Add(i) => new_self.value += i,
            CounterTransition::Subtract(i) => new_self.value -= i,
            CounterTransition::Reset => new_self.value = 0,
        }
        Ok(new_self)
    }
}
```

### Pattern: Direct Construction

For simpler types, construct directly:

```rust
impl<T: Copy> StateMachine for Atom<T> {
    fn apply(&self, transition: &ReplaceAtom<T>) -> Result<Self, NeverConflict> {
        let ReplaceAtom(v) = transition;
        Ok(Atom::new(*v))  // Direct construction, no clone
    }
}
```

### Pattern: Field-wise Apply

For composite types, apply to specific fields:

```rust
impl StateMachine for GameState {
    fn apply(&self, transition: &GameStateTransform) -> Result<Self, Self::Conflict> {
        match transition {
            GameStateTransform::ApplyScore(val) => {
                match self.score.apply(val) {
                    Ok(v) => {
                        let mut new_self = self.clone();
                        new_self.score = v;  // Only one field changes
                        Ok(new_self)
                    },
                    Err(e) => Err(GameStateConflict::ScoreConflict(e))
                }
            },
            // ... other fields
        }
    }
}
```

---

## 2. Rc for Shared State

`Rc<T>` (Reference Counted) enables shared ownership without mutation.

### Why Rc?

```rust
pub struct StateClient<S: StateMachine> {
    golden_state: Rc<S>,        // Shared read-only access
    optimistic_state: Rc<S>,     // Updated frequently
    // ...
}
```

**Benefits:**
- Cheap cloning (increment ref count, not deep copy)
- Shared between multiple owners
- Immutable by design

### Rc Pattern in Aper

```rust
impl<S: StateMachine> StateClient<S> {
    pub fn new(state: S, version: StateVersionNumber) -> Self {
        let state = Rc::new(state);
        StateClient {
            golden_state: state.clone(),    // Clone Rc, not state
            optimistic_state: state,        // Move Rc
            transitions: VecDeque::new(),
            version,
            next_transition: ClientTransitionNumber::default(),
        }
    }

    pub fn state(&self) -> Rc<S> {
        self.optimistic_state.clone()  // Clone Rc (cheap)
    }
}
```

### Rc with Cell/RefCell for Interior Mutability

When you need mutation through Rc:

```rust
use std::cell::RefCell;
use std::rc::Rc;

// Not used in Aper core, but common pattern
let data: Rc<RefCell<Vec<i32>>> = Rc::new(RefCell::new(vec![1, 2, 3]));
data.borrow_mut().push(4);
```

### Arc for Thread Safety

For multi-threaded scenarios:

```rust
use std::sync::Arc;

pub struct SharedState {
    state: Arc<Mutex<StateMachine>>,
}
```

---

## 3. Serde Serialization Strategies

Aper relies heavily on Serde for network serialization.

### Derive Serialization

```rust
use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Counter {
    value: i64,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub enum CounterTransition {
    Set(i64),
    Increment(i64),
    Decrement(i64),
}
```

### Custom Serialization for Rc

```rust
impl<T: Serialize> Serialize for AtomRc<T> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        self.value.serialize(serializer)  // Serialize inner value
    }
}

impl<'de, T: Deserialize<'de>> Deserialize<'de> for AtomRc<T> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        Ok(AtomRc {
            value: Rc::new(T::deserialize(deserializer)?),
        })
    }
}
```

### Serde Bounds on Generics

```rust
#[derive(Serialize, Deserialize, Clone, Debug, Default)]
#[serde(bound = "")]  // Override default bounds
pub struct StateMachineContainerProgram<SM: StateMachine>(pub SM)
where
    <SM as StateMachine>::Transition: Send;
```

### Custom Serialization with Attributes

```rust
// Use milliseconds for timestamps
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct TransitionEvent<T> {
    #[serde(with = "ts_milliseconds")]
    pub timestamp: Timestamp,
    pub client: Option<ClientId>,
    pub transition: T,
}
```

### Handling PhantomData

```rust
pub struct List<T: StateMachine + PartialEq> {
    items: OrdMap<ZenoIndex, Uuid>,
    pool: HashMap<Uuid, T>,
    _marker: PhantomData<T>,  // Not serialized
}
```

---

## 4. Type-Level Determinism Guarantees

Rust's type system can enforce determinism constraints.

### NeverConflict for Impossible Errors

```rust
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum NeverConflict {}  // No variants = impossible to construct

impl<T: Copy> StateMachine for Atom<T> {
    type Conflict = NeverConflict;  // Can never fail

    fn apply(&self, transition: &Self::Transition) -> Result<Self, NeverConflict> {
        // Compiler knows Err case is impossible
        Ok(Atom::new(*transition.0))
    }
}
```

### Trait Bounds for Serialization

```rust
pub trait StateMachine:
    Clone +                  // Can clone for optimistic updates
    DeserializeOwned +       // Can deserialize from network
    Serialize +              // Can serialize for network
    Debug +                  // Can debug print
    'static                  // No lifetime parameters
{
    // ...
}
```

### Send/Sync for Thread Safety

```rust
pub trait StateProgram: StateMachine<Transition = TransitionEvent<Self::T>>
where
    <Self as StateProgram>::T: Unpin + Send + Sync,  // Thread-safe
{
    type T: Debug + Serialize + DeserializeOwned + Clone + PartialEq;
    // ...
}
```

---

## 5. Derive Macro Implementation

Aper's `#[derive(StateMachine)]` is a procedural macro.

### Macro Entry Point

```rust
#[proc_macro_derive(StateMachine)]
pub fn state_machine_derive(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    impl_state_machine_derive(input.into()).into()
}
```

### Parsing the Input

```rust
fn impl_state_machine_derive(input: TokenStream) -> TokenStream {
    let ast: ItemStruct = syn::parse2(input).expect("Should decorate a struct.");

    let name = &ast.ident;
    let transform_name = quote::format_ident!("{}Transform", name.to_string());
    let conflict_name = quote::format_ident!("{}Conflict", name.to_string());

    let fields: Vec<Field> = match &ast.fields {
        syn::Fields::Named(fields) => fields.named.iter().map(Field::new).collect(),
        _ => panic!("Only structs with named fields can derive StateMachine."),
    };
    // ...
}
```

### Generating the Transition Enum

```rust
fn generate_transform(enum_name: &Ident, fields: &[Field], visibility: &Visibility) -> TokenStream {
    let variants: TokenStream = fields
        .iter()
        .flat_map(Field::generate_enum_variant)
        .collect();

    quote! {
        #[derive(serde::Serialize, serde::Deserialize, Debug, Clone, PartialEq)]
        #visibility enum #enum_name {
            #variants
        }
    }
}

// For each field, generate variant like:
// ApplyScore(<Score as StateMachine>::Transition)
fn generate_enum_variant(&self) -> TokenStream {
    let Field { apply_variant, ty, .. } = self;
    quote! {
        #apply_variant(<#ty as StateMachine>::Transition),
    }
}
```

### Generating the apply() Implementation

```rust
fn generate_transition_case(
    &self,
    transition_name: &Ident,
    conflict_name: &Ident,
) -> TokenStream {
    let Field { name, apply_variant, conflict_variant, .. } = self;
    quote! {
        #transition_name::#apply_variant(val) => {
            match self.#name.apply(val) {
                Ok(v) => {
                    let mut new_self = self.clone();
                    new_self.#name = v;
                    Ok(new_self)
                },
                Err(e) => Err(#conflict_name::#conflict_variant(e))
            }
        },
    }
}
```

### Generating Accessor Methods

```rust
fn generate_accessor(&self, enum_name: &Ident) -> TokenStream {
    let Field { name, ty, map_fn_name, apply_variant, transition_ty } = self;

    quote! {
        pub fn #name(&self) -> &#ty {
            &self.#name
        }

        pub fn #map_fn_name(&self, fun: impl FnOnce(&#ty) -> #transition_ty) -> #enum_name {
            #enum_name::#apply_variant(fun(&self.#name))
        }
    }
}
```

---

## 6. CRDT Data Structures

Aper includes CRDT-style data structures for concurrent-safe collections.

### Fractional Indexing with ZenoIndex

```rust
// Uses fractional_index crate
use fractional_index::ZenoIndex;

// Insert between any two indices
let index_a = ZenoIndex::default();           // [0]
let index_b = ZenoIndex::new_after(&index_a); // [1]
let index_c = ZenoIndex::new_between(&index_a, &index_b).unwrap(); // [0.5]
```

### List with OrdMap

```rust
use im_rc::OrdMap;

#[derive(Clone, PartialEq, Debug)]
pub struct List<T: StateMachine + PartialEq> {
    items: OrdMap<ZenoIndex, Uuid>,      // Position -> ID
    items_inv: OrdMap<Uuid, ZenoIndex>,  // ID -> Position (inverse)
    pool: HashMap<Uuid, T>,              // ID -> Value
}
```

### Insert Operation

```rust
fn do_insert(
    &self,
    position: &ListPosition,
    id: &Uuid,
    value: T,
) -> Result<Self, ListConflict<T>> {
    let location = self.get_location(position);

    let mut new_self = self.clone();
    new_self.items.insert(location.clone(), *id);
    new_self.items_inv.insert(*id, location);
    new_self.pool.insert(*id, value);
    Ok(new_self)
}
```

### Get Location with Fallback

```rust
pub fn get_location(&self, position: &ListPosition) -> ZenoIndex {
    match position {
        ListPosition::Beginning => {
            return if let Some((i, _)) = self.items.iter().next() {
                ZenoIndex::new_before(i)
            } else {
                ZenoIndex::default()
            };
        }
        ListPosition::End => {
            return if let Some((i, _)) = self.items.iter().next_back() {
                ZenoIndex::new_after(i)
            } else {
                ZenoIndex::default()
            }
        }
        ListPosition::Before(uuid, fallback) => {
            if let Some(location) = self.items_inv.get(uuid) {
                ZenoIndex::new_before(location)
            } else {
                ZenoIndex::new_before(fallback)
            }
        }
        // ...
    }
}
```

---

## 7. Complete Implementation Example

Let's build a complete state machine from scratch:

### Step 1: Define the State

```rust
use serde::{Serialize, Deserialize};
use aper::{StateMachine, NeverConflict};
use aper::data_structures::{Counter, Atom, List};

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct Task {
    pub id: u32,
    pub title: String,
    pub completed: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct TodoList {
    tasks: List<Atom<Task>>,
    completed_count: Counter,
}
```

### Step 2: Define Transitions

```rust
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub enum TodoListTransition {
    AddTask { id: u32, title: String },
    CompleteTask(u32),
    DeleteTask(u32),
    ReorderTasks { from: usize, to: usize },
}
```

### Step 3: Implement StateMachine

```rust
use uuid::Uuid;

impl StateMachine for TodoList {
    type Transition = TodoListTransition;
    type Conflict = NeverConflict;

    fn apply(&self, transition: &Self::Transition) -> Result<Self, NeverConflict> {
        let mut new_self = self.clone();

        match transition {
            TodoListTransition::AddTask { id, title } => {
                let task = Task {
                    id: *id,
                    title: title.clone(),
                    completed: false,
                };
                let (_, op) = new_self.tasks.append(Atom::new(task));
                new_self.tasks = new_self.tasks.apply(&op).unwrap();
            }

            TodoListTransition::CompleteTask(task_id) => {
                // Find task and mark complete
                for item in new_self.tasks.iter() {
                    if item.value.value().id == *task_id {
                        let updated = Task {
                            completed: true,
                            ..item.value.value().clone()
                        };
                        let op = new_self.tasks.map_item(item.id, |_| {
                            ReplaceAtom(updated)
                        });
                        new_self.tasks = new_self.tasks.apply(&op).unwrap();
                        new_self.completed_count = new_self.completed_count
                            .apply(&Counter::increment(1)).unwrap();
                        break;
                    }
                }
            }

            TodoListTransition::DeleteTask(task_id) => {
                for item in new_self.tasks.iter() {
                    if item.value.value().id == *task_id {
                        let op = new_self.tasks.delete(item.id);
                        new_self.tasks = new_self.tasks.apply(&op).unwrap();
                        break;
                    }
                }
            }

            TodoListTransition::ReorderTasks { from, to } => {
                // Get IDs at positions
                let from_id: Option<Uuid> = new_self.tasks.iter()
                    .nth(*from).map(|item| item.id);
                let to_item: Option<_> = new_self.tasks.iter().nth(*to);

                if let (Some(from_id), Some(to_item)) = (from_id, to_item) {
                    let new_location = to_item.location.clone();
                    let op = new_self.tasks.move_item(from_id, new_location);
                    new_self.tasks = new_self.tasks.apply(&op).unwrap();
                }
            }
        }

        Ok(new_self)
    }
}
```

### Step 4: Add Convenience Methods

```rust
impl TodoList {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_task(&self, id: u32, title: String) -> TodoListTransition {
        TodoListTransition::AddTask { id, title }
    }

    pub fn complete_task(&self, task_id: u32) -> TodoListTransition {
        TodoListTransition::CompleteTask(task_id)
    }

    pub fn tasks(&self) -> impl Iterator<Item = &Atom<Task>> {
        self.tasks.iter().map(|item| item.value)
    }
}
```

---

## Summary

| Pattern | Use Case |
|---------|----------|
| Immutable apply() | Deterministic state updates |
| Rc for sharing | Efficient state clones |
| Serde derive | Network serialization |
| NeverConflict | Impossible errors |
| Procedural macros | Automatic trait impls |
| OrdMap + ZenoIndex | CRDT lists |

---

## Next Steps

Continue to [production-grade.md](production-grade.md) for production deployment considerations.
