# SpacetimeDB Bindings Deep Dive

## Overview

SpacetimeDB provides bindings for multiple languages. This document explains how the binding system works, from the low-level WASM ABI to high-level SDKs.

## Binding Architecture

```
┌─────────────────────────────────────────────────────────────────────┐
│                    Language SDKs                                      │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐              │
│  │  Rust SDK    │  │  C# SDK      │  │ TypeScript   │              │
│  │  (bindings)  │  │  (bindings)  │  │   SDK        │              │
│  └──────────────┘  └──────────────┘  └──────────────┘              │
└─────────────────────────────────────────────────────────────────────┘
                              │
                              │ Codegen
                              ▼
┌─────────────────────────────────────────────────────────────────────┐
│                   WASM ABI Layer                                     │
│  ┌────────────────────────────────────────────────────────────────┐ │
│  │              spacetimedb-bindings-sys                           │ │
│  │  - Low-level FFI bindings                                       │ │
│  │  - Host function imports                                        │ │
│  │  - Module exports                                               │ │
│  └────────────────────────────────────────────────────────────────┘ │
└─────────────────────────────────────────────────────────────────────┘
                              │
                              │ WASM
                              ▼
┌─────────────────────────────────────────────────────────────────────┐
│                    Module Host                                       │
│  ┌────────────────────────────────────────────────────────────────┐ │
│  │              spacetimedb-core                                   │ │
│  │  - WASM runtime (wasmtime)                                      │ │
│  │  - Host function implementations                                │ │
│  └────────────────────────────────────────────────────────────────┘ │
└─────────────────────────────────────────────────────────────────────┘
```

## WASM ABI Reference

### Host Functions (Imports)

Modules import these functions from the host:

```wasm
(module
  (import "spacetimedb" "__host_log__"
    (func $log (param i32 i32)))  ; (level, message_ptr)

  (import "spacetimedb" "__host_table_insert__"
    (func $insert (param i32 i32 i32) (result i32)))
    ; table_id, row_ptr, row_len -> success

  (import "spacetimedb" "__host_table_delete__"
    (func $delete (param i32 i32) (result i32)))
    ; table_id, row_ptr -> success

  (import "spacetimedb" "__host_schedule_reducer__"
    (func $schedule (param i32 i32 i64 i32 i32) (result i32)))
    ; reducer_id, args_ptr, scheduled_at, args_len -> schedule_id

  (import "spacetimedb" "__host_cancel_reducer__"
    (func $cancel (param i32) (result i32)))
    ; schedule_id -> success

  (import "spacetimedb" "__host_console_print__"
    (func $console (param i32 i32 i32)))
    ; level, target, message

  (import "spacetimedb" "__host_call_reducer__"
    (func $call_reducer (param i32 i32 i32 i32 i32 i32 i64)))
    ; db_id, reducer_id, args_ptr, args_len, identity_ptr,
      identity_len, connection_id
)
```

### Module Exports

Modules export these functions:

```wasm
(module
  ;; Required exports
  (export "init" (func $init))
  (export "__call_reducer__" (func $call_reducer))

  ;; Optional exports
  (export "__init_client_connected__" (func $client_connected))
  (export "__init_client_disconnected__" (func $client_disconnected))
)
```

## Rust Bindings

### Macro System

```rust
// crates/bindings-macro/src/lib.rs

/// Table attribute
#[proc_macro_attribute]
pub fn table(args: TokenStream, input: TokenStream) -> TokenStream {
    // Generates:
    // - Table accessor methods
    // - Insert/delete methods
    // - Index accessors
    // - Serialization impls
}

/// Reducer attribute
#[proc_macro_attribute]
pub fn reducer(args: TokenStream, input: TokenStream) -> TokenStream {
    // Generates:
    // - Reducer registration
    // - Argument serialization
    // - Error handling wrapper
}

/// View attribute
#[proc_macro_attribute]
pub fn view(args: TokenStream, input: TokenStream) -> TokenStream {
    // Generates view accessor
}
```

### Table Implementation Pattern

```rust
// User code:
#[spacetimedb::table(accessor = players, public)]
pub struct Player {
    #[primary_key]
    id: u64,
    name: String,
    score: u32,
}

// Generated code (simplified):
pub mod players {
    // Table accessor
    pub fn count() -> u64 { /* ... */ }

    // Index accessors
    pub fn id() -> PlayerIdIndex { /* ... */ }

    // Iterator
    pub fn iter() -> impl Iterator<Item = PlayerRef> { /* ... */ }

    // Generated ref type
    pub struct PlayerRef<'a> {
        __row_ref: &'a spacetimedb::RowRef,
    }

    impl PlayerRef<'_> {
        pub fn id(&self) -> u64 { /* read column */ }
        pub fn name(&self) -> &str { /* read column */ }
        pub fn score(&self) -> u32 { /* read column */ }
    }
}
```

### Reducer Implementation

```rust
// User code:
#[spacetimedb::reducer]
pub fn add_player(ctx: &ReducerContext, name: String) -> Result<u64, String> {
    let player = Player {
        id: 0,  // Auto-generated
        name,
        score: 0,
    };
    Ok(ctx.db.players().insert(player)?.id)
}

// Generated code (simplified):
const REDUCER_ID: u32 = 0;

#[no_mangle]
extern "C" fn __call_reducer__(
    reducer_id: u32,
    args_ptr: u32,
    args_len: u32,
) -> u32 {
    match reducer_id {
        0 => call_add_player(args_ptr, args_len),
        _ => 1, // Unknown reducer
    }
}

fn call_add_player(args_ptr: u32, args_len: u32) -> u32 {
    // Deserialize arguments
    let (name,): (String,) = Deserialize::from_bytes(
        unsafe { slice::from_raw_parts(args_ptr, args_len) }
    )?;

    // Get context
    let ctx = ReducerContext::from_host();

    // Call user function
    let result = add_player(&ctx, name);

    // Serialize and return result
    serialize_and_return(result)
}
```

### Reducer Context

```rust
// crates/bindings/src/reducer_context.rs

pub struct ReducerContext {
    /// Identity of the caller
    pub sender: Identity,
    /// Connection ID (if applicable)
    pub connection_id: Option<ConnectionId>,
    /// Timestamp when reducer started
    pub timestamp: u64,
    /// Database accessor
    pub db: Database,
}

impl ReducerContext {
    /// Construct from host state
    fn from_host() -> Self {
        Self {
            sender: Identity::from_host(),
            connection_id: ConnectionId::from_host(),
            timestamp: timestamp(),
            db: Database,
        }
    }
}
```

## Type System Integration

### SATS Derivation

```rust
// crates/bindings/src/lib.rs

/// Types that can be stored in SpacetimeDB tables
pub trait SpacetimeType: Serialize + DeserializeOwned {
    fn algebraic_type() -> AlgebraicType;
}

// Auto-derived via proc macro:
#[derive(SpacetimeType)]
pub struct Player {
    pub id: u64,
    pub name: String,
}

// Generates:
impl SpacetimeType for Player {
    fn algebraic_type() -> AlgebraicType {
        AlgebraicType::Product(ProductType {
            elements: vec![
                ProductTypeElement {
                    name: Some("id".into()),
                    algebraic_type: AlgebraicType::U64,
                },
                ProductTypeElement {
                    name: Some("name".into()),
                    algebraic_type: AlgebraicType::String,
                },
            ],
        })
    }
}
```

### Custom Type Registration

```rust
// For types not using derive:

impl SpacetimeType for MyCustomType {
    fn algebraic_type() -> AlgebraicType {
        AlgebraicType::Product(ProductType {
            elements: vec![
                ProductTypeElement {
                    name: Some("field1".into()),
                    algebraic_type: AlgebraicType::U32,
                },
                // ...
            ],
        })
    }
}

impl Serialize for MyCustomType {
    fn serialize(&self, serializer: &mut impl Serializer) -> Result<(), SerializeError> {
        // Custom serialization logic
    }
}

impl Deserialize for MyCustomType {
    fn deserialize(deserializer: &mut impl Deserializer) -> Result<Self, DeserializeError> {
        // Custom deserialization logic
    }
}
```

## C# Bindings

### Table Declaration

```csharp
// crates/bindings-csharp/Runtime/Table.cs

[SpacetimeDB.Table(Accessor = "Player", Public = true)]
public partial struct Player
{
    [SpacetimeDB.PrimaryKey]
    public uint Id;

    public string Name;

    public uint Score;
}

// Generated:
public static partial class Player
{
    public static uint Count => __table_accessors.Count();

    public static PlayerIdIndex Id => new PlayerIdIndex();

    public static IEnumerable<Player> Iter() => __table_accessors.Iter();

    public static Player Insert(Player row) => __table_accessors.Insert(row);

    public static bool Delete(Player row) => __table_accessors.Delete(row);
}
```

### Reducer Declaration

```csharp
[SpacetimeDB.Reducer]
public static void AddPlayer(ReducerContext ctx, string name)
{
    var player = new Player
    {
        Id = 0,  // Auto-increment
        Name = name,
        Score = 0,
    };
    Player.Insert(player);
}
```

## TypeScript Bindings

### Table Declaration

```typescript
// crates/bindings-typescript/src/table.ts

import { table, t } from 'spacetimedb/server';

export const players = table(
  { name: 'players', public: true },
  {
    id: t.u64().primaryKey().autoInc(),
    name: t.string(),
    score: t.u32(),
  }
);

// Type inference:
type Player = typeof players.row;
```

### Reducer Declaration

```typescript
export const addPlayer = spacetimedb.reducer(
  { name: t.string() },
  (ctx, { name }) => {
    ctx.db.players.insert({
      id: 0n,
      name,
      score: 0,
    });
  }
);
```

## C++ Bindings

### Table Declaration

```cpp
// crates/bindings-cpp/include/spacetimedb/table.h

struct Player {
    uint64_t id;
    std::string name;
    uint32_t score;
};

SPACETIMEDB_STRUCT(Player, id, name, score)
SPACETIMEDB_TABLE(Player, players, Public)
FIELD_PrimaryKey(players, id)
```

### Reducer Declaration

```cpp
SPACETIMEDB_REDUCER(add_player, ReducerContext ctx, std::string name) {
    Player player{
        .id = 0,
        .name = std::move(name),
        .score = 0,
    };
    players::insert(player);
    return Ok();
}
```

## Code Generation

### SDK Generation Pipeline

```
Module Source (Rust/C#/TS/C++)
           │
           │ Compile
           ▼
    WASM Module / JS Bundle
           │
           │ spacetimedb generate
           ▼
    Client SDK Files
    ├── Rust: generated.rs
    ├── C#: Generated.cs
    ├── TS: generated.ts
    └── C++: generated.h
```

### Generated Client Code

```typescript
// Generated TypeScript client code

export namespace db {
  export namespace players {
    export interface Row {
      id: bigint;
      name: string;
      score: number;
    }

    export class Table {
      readonly id: UniqueIndex<bigint, Row>;
    }
  }
}

export namespace reducers {
  export function addPlayer(name: string): void;
}
```

## Error Handling

### Rust Error Types

```rust
// crates/bindings/src/error.rs

#[derive(Debug, thiserror::Error)]
pub enum ReducerError {
    #[error("Table not found: {0}")]
    TableNotFound(String),

    #[error("Unique constraint violation: {0}")]
    UniqueViolation(String),

    #[error("Type error: {0}")]
    TypeError(#[from] TypeError),

    #[error("BSATN error: {0}")]
    BsatnError(#[from] BsatnError),
}

// Result type used by reducers
pub type ReducerResult<T> = Result<T, ReducerError>;
```

### Error Propagation

```rust
// Errors in reducers become failed transactions
#[spacetimedb::reducer]
pub fn may_fail(ctx: &ReducerContext, input: String) -> Result<(), String> {
    if input.is_empty() {
        return Err("Input cannot be empty".into());
    }

    // This error will roll back the transaction
    ctx.db.players().insert(Player {
        id: 0,
        name: input,
        score: 0,
    })?;

    Ok(())
}
```

## Version Compatibility

### ABI Versioning

```rust
// crates/bindings-sys/src/abi_version.rs

/// Current WASM ABI version
pub const ABI_VERSION: u32 = 9;

/// Minimum supported module version
pub const MIN_MODULE_VERSION: u32 = 7;

/// Host provides version check
extern "C" {
    fn __host_abi_version__() -> u32;
}

/// Module exports its version
#[no_mangle]
pub extern "C" fn __abi_version__() -> u32 {
    ABI_VERSION
}
```

### Feature Flags

```rust
// crates/bindings/Cargo.toml features

[features]
# Unstable features (may change)
unstable = []

# Enable procedure support (beta)
procedures = ["unstable"]

# Enable logging macros
logging = []
```
