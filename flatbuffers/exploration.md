---
location: /home/darkvoid/Boxxed/@formulas/src.rust/src.wasm/flatbuffers
repository: https://github.com/google/flatbuffers
explored_at: 2026-03-22
language: C++, Rust, TypeScript, Multiple
---

# Project Exploration: FlatBuffers

## Overview

**FlatBuffers** is a cross-platform serialization library architected for **maximum memory efficiency**. It allows you to directly access serialized data without parsing/unpacking it first, while still having great forwards/backwards compatibility.

### Key Value Proposition

- **Zero-copy deserialization** - Access data directly from the serialized buffer
- **Memory efficient** - No additional allocations for parsing
- **Fast** - Direct access means no parsing overhead
- **Language agnostic** - 20+ supported languages
- **Schema evolution** - Forward and backward compatible

### Example Usage

```rust
// Generate code from schema
// flatc --rust schema.fbs

use my_game::*;

// Create a buffer
let mut builder = FlatBufferBuilder::new();
let name = builder.create_string("Orc");
let weapon = create_weapon(&mut builder, "Axe", 10);
let orc = create_monster(&mut builder, &MonsterArgs {
    name: Some(name),
    weapon: Some(weapon),
    ..Default::default()
});
builder.finish(orc, None);

// Write to disk/network
let data = builder.finished_data();

// READ WITHOUT PARSING
let monster = root_as_monster(data).unwrap();
println!("Monster name: {}", monster.name());  // Direct access!
println!("Weapon: {}", monster.weapon().name());
```

## Repository Structure

```
/home/darkvoid/Boxxed/@formulas/src.rust/src.wasm/flatbuffers/
├── include/                       # C++ header files
│   └── flatbuffers/               # Core C++ library
├── rust/                          # Rust implementation
│   ├── flatbuffers/               # Main Rust crate
│   ├── flexbuffers/               # Schema-less variant
│   └── reflection/                # Runtime reflection
├── ts/                            # TypeScript implementation
├── js/                            # JavaScript implementation
├── go/                            # Go implementation
├── java/                          # Java implementation
├── csharp/                        # C# implementation
├── python/                        # Python implementation
├── swift/                         # Swift implementation
├── kotlin/                        # Kotlin implementation
├── dart/                          # Dart implementation
├── php/                           # PHP implementation
├── lua/                           # Lua implementation
├── lobste/                        # Lobster implementation
├── nim/                           # Nim implementation
├── src/                           # flatc compiler source
│   ├── flatc/                     # Compiler frontend
│   ├── idl_parser.cpp             # Schema parser
│   ├── code_generators.cpp        # Language generators
│   └── reflection.cpp             # Reflection support
├── tests/                         # Test suite
├── samples/                       # Example code
├── benchmarks/                    # Performance tests
└── grpc/                          # gRPC integration
```

## Core Concepts

### 1. Schema Language

FlatBuffers uses an Interface Definition Language (IDL):

```protobuf
// schema.fbs

namespace MyGame;

table Monster {
  name: string;
  hp: int = 100;
  mana: int = 50;
  pos: Vec3;
  weapons: [Weapon];
  inventory: [ubyte];
  enemy: Monster;
}

struct Vec3 {
  x: float;
  y: float;
  z: float;
}

table Weapon {
  name: string;
  damage: int;
}

root_type Monster;
```

### 2. Binary Layout

```
┌─────────────────────────────────────────────────────────────────┐
│                  FlatBuffer Memory Layout                        │
│                                                                  │
│  Offset 0                          Offset N                     │
│  ┌─────────────────────────────────────────────────────────┐    │
│  │                    vtable offset                        │    │  <- Start here
│  ├─────────────────────────────────────────────────────────┤    │
│  │                   vtable (shared)                       │    │
│  │  ┌──────────┬──────────┬──────────┬──────────┐         │    │
│  │  │ vsize    │ tsize    │ offset 0 │ offset 1 │ ...     │    │
│  │  └──────────┴──────────┴──────────┴──────────┘         │    │
│  ├─────────────────────────────────────────────────────────┤    │
│  │                   Object data                           │    │
│  │  ┌─────────┬─────────┬─────────┬─────────────────┐     │    │
│  │  │ field 0 │ field 1 │  ...    │  variable data  │     │    │
│  │  └─────────┴─────────┴─────────┴─────────────────┘     │    │
│  └─────────────────────────────────────────────────────────┘    │
│                                                                  │
│  All offsets are relative to current position (backward)        │
└─────────────────────────────────────────────────────────────────┘
```

### 3. VTable Mechanism

VTables enable schema evolution:

```
Table Structure:
┌───────────────┐
│ vtable_offset │──┐
├───────────────┤  │  (negative offset)
│  field data   │  │
└───────────────┘  │
                   ▼
            ┌──────────────┐
            │ VTable       │
            ├──────────────┤
            │ vsize: 8     │  (vtable size)
            ├──────────────┤
            │ tsize: 24    │  (table size)
            ├──────────────┤
            │ offset[0]: 4 │  (field 0 offset)
            ├──────────────┤
            │ offset[1]: 8 │  (field 1 offset)
            ├──────────────┤
            │ offset[2]: 0 │  (field 2 missing!)
            └──────────────┘
```

Zero offset = field not present (uses default value)

### 4. Wire Format

```
Scalar values (int, float, etc.):
┌────────────────┐
│    value       │  (little-endian)
└────────────────┘

Strings:
┌────────────┬────────────────┬─────┐
│  length    │    data...     │ \0  │
│  (u32)     │  (UTF-8)       │     │
└────────────┴────────────────┴─────┘

Vectors (arrays):
┌────────────┬──────────────┬──────────┬──────────┬──────┐
│  length    │  element[0]  │ element  │  ...     │ pad  │
│  (u32)     │              │  [1]     │          │      │
└────────────┴──────────────┴──────────┴──────────┴──────┘

Tables:
┌───────────────┬──────────────────┐
│ vtable_off    │  field data...   │
└───────────────┴──────────────────┘
```

## Rust Implementation

### Crate Structure

```
rust/flatbuffers/
├── src/
│   ├── lib.rs              # Main entry point
│   ├── builder.rs          # FlatBufferBuilder
│   ├── follow.rs           # Follow trait for reading
│   ├── read.rs             # Low-level read operations
│   ├── table.rs            # Table abstraction
│   ├── vector.rs           # Vector types
│   ├── vtable.rs           # VTable handling
│   ├── primitive.rs        # Primitive type traits
│   ├── push.rs             # Push trait for writing
│   ├── endian_scalar.rs    # Endian conversion
│   └── verifiable.rs       # Verifiable trait
```

### Key Types

```rust
/// Builder for creating FlatBuffers
pub struct FlatBufferBuilder<'fbb> {
    buf: Vec<u8>,           // Growing buffer
    head: usize,            // Current write position (from end)
    minalign: usize,        // Minimum alignment seen
    vtable: Vec<usize>,     // Current vtable offsets
    vtables: Vec<TableOffset>, // Saved vtables
}

/// Trait for reading types from buffer
pub trait Follow<'a> {
    type Inner: 'a;
    fn follow(buf: &'a [u8], loc: usize) -> Self::Inner;
}

/// Table reference
pub struct Table<'a> {
    buf: &'a [u8],
    loc: usize,
}
```

### Generated Code Pattern

```rust
// Generated from: table Monster { name: string; hp: int = 100; }

pub struct Monster<'a> {
    _tab: Table<'a>,
}

impl<'a> flatbuffers::Follow<'a> for Monster<'a> {
    type Inner = Monster<'a>;
    fn follow(buf: &'a [u8], loc: usize) -> Self::Inner {
        Self { _tab: Table::new(buf, loc) }
    }
}

impl<'a> Monster<'a> {
    pub fn name(&self) -> Option<&'a str> {
        self._tab.get::<flatbuffers::ForwardsUOffset<&str>>(4, None)
    }

    pub fn hp(&self) -> i32 {
        self._tab.get::<i32>(6, Some(100)).unwrap_or(100)
    }
}

// Builder pattern
pub struct MonsterArgs<'a> {
    pub name: Option<flatbuffers::WIPOffset<&'a str>>,
    pub hp: i32,
}

impl Default for MonsterArgs<'_> {
    fn default() -> Self {
        MonsterArgs { name: None, hp: 100 }
    }
}
```

## FlexBuffers

**FlexBuffers** is a schema-less variant of FlatBuffers:

```rust
use flexbuffers;

// Create without schema
let mut b = flexbuffers::FlexBufferBuilder::new();
b.push_str("hello");
let buf = b.finish();

// Read back
let r = flexbuffers::Reader::get_root(buf).unwrap();
assert_eq!(r.as_str(), Some("hello"));

// Works with maps too
let mut b = flexbuffers::FlexBufferBuilder::new();
b.push_map(|b| {
    b.push_str("name", "Alice");
    b.push_i32("age", 30);
});
```

Use cases:
- Dynamic data structures
- Configuration files
- When schema is not known at compile time

## Performance Characteristics

### Zero-Copy Access

```
Traditional Serialization (protobuf, JSON):
Buffer → Parse → Allocate → Copy → Access
       ~100μs    ~50μs

FlatBuffers:
Buffer → Access
       <1μs
```

### Benchmarks (typical)

| Operation | FlatBuffers | Protobuf | JSON |
|-----------|-------------|----------|------|
| Parse time | 0μs (zero-copy) | 50-100μs | 100-200μs |
| Access time | Direct | Via getters | Via parsing |
| Memory overhead | 0% | 50-100% | 100-200% |
| Serialize time | Fast | Fast | Slow |

### Memory Layout Efficiency

```
FlatBuffers:
┌─────────────────────────────┐
│ Header (4 bytes)            │
│ Data (aligned)              │
│ No per-object overhead      │
└─────────────────────────────┘

Protobuf:
┌─────────────────────────────┐
│ Tag + Length + Data (each)  │
│ Varint encoding overhead    │
└─────────────────────────────┘
```

## flatc Compiler

### Usage

```bash
# Generate Rust code
flatc --rust schema.fbs

# Generate multiple languages
flatc --rust --cpp --ts schema.fbs

# With reflection
flatc --rust --reflect-types schema.fbs

# Binary schema (for reflection)
flatc --binary --schema schema.fbs

# JSON parsing
flatc --rust --schema schema.fbs data.json
```

### Compiler Architecture

```
schema.fbs
    │
    ▼
┌─────────────────┐
│  Lexer/Parser   │  (idl_parser.cpp)
└─────────────────┘
    │
    ▼
┌─────────────────┐
│  AST/IR         │  (Internal representation)
└─────────────────┘
    │
    ├──────────┬──────────┬──────────┐
    ▼          ▼          ▼          ▼
┌────────┐  ┌────────┐  ┌────────┐  ┌────────┐
│  Rust  │  │  C++   │  │   TS   │  │  ...   │
│ Codegen│  │ Codegen│  │ Codegen│  │ Codegen│
└────────┘  └────────┘  └────────┘  └────────┘
```

## Trade-offs

| Aspect | Benefit | Cost |
|--------|---------|------|
| Zero-copy | Fast reads | Buffer must stay immutable |
| VTables | Schema evolution | Slightly larger size |
| Backward offsets | Position independent | Can't stream parse |
| Alignment padding | Fast access | Some wasted space |
| Single buffer | Simple | All data in one blob |

## Reproducing the Design

### Step 1: Buffer Builder

```rust
pub struct SimpleBuilder {
    buf: Vec<u8>,
}

impl SimpleBuilder {
    pub fn new() -> Self { Self { buf: Vec::new() } }

    pub fn push_u32(&mut self, v: u32) -> usize {
        let pos = self.buf.len();
        self.buf.extend_from_slice(&v.to_le_bytes());
        pos
    }

    pub fn push_str(&mut self, s: &str) -> (usize, u32) {
        let len = s.len() as u32;
        let pos = self.buf.len();
        self.buf.extend_from_slice(&len.to_le_bytes());
        self.buf.extend_from_slice(s.as_bytes());
        self.buf.push(0);  // Null terminator
        (pos, len)
    }
}
```

### Step 2: Reader with Follow

```rust
pub trait Follow<'a> {
    fn follow(buf: &'a [u8], loc: usize) -> Self;
}

impl<'a> Follow<'a> for u32 {
    fn follow(buf: &'a [u8], loc: usize) -> Self {
        u32::from_le_bytes(buf[loc..loc+4].try_into().unwrap())
    }
}

impl<'a> Follow<'a> for &'a str {
    fn follow(buf: &'a [u8], loc: usize) -> Self {
        let len = u32::follow(buf, loc) as usize;
        std::str::from_utf8(&buf[loc+4..loc+4+len]).unwrap()
    }
}
```

### Step 3: VTable Lookup

```rust
fn get_field<'a, T: Follow<'a>>(
    buf: &'a [u8],
    table_loc: usize,
    vtable_loc: usize,
    field_offset: usize,
    default: T::Inner,
) -> T::Inner {
    let vtable_size = u16::follow(buf, vtable_loc) as usize;

    if field_offset >= vtable_size {
        return default;  // Field not in this vtable version
    }

    let field_pos = u16::follow(buf, vtable_loc + field_offset);
    if field_pos == 0 {
        return default;  // Field not present
    }

    T::follow(buf, table_loc + field_pos as usize)
}
```

## Use Cases in WASM

### Why FlatBuffers for WASM?

1. **Small binary size** - Important for download time
2. **No parsing overhead** - Fast startup
3. **Memory efficient** - WASM memory is limited
4. **Zero-copy** - No GC pressure in JS

### Example: WASM Module Config

```rust
// Schema for WASM module configuration
table WasmConfig {
  module_name: string;
  imports: [Import];
  exports: [Export];
  memory_pages: int = 1;
}

// Load directly from network buffer
let response = fetch("/module.wasm.config").await;
let bytes = response.array_buffer().await;
let config = root_as_wasm_config(&bytes).unwrap();
// Zero-copy access to config!
```

## Related Projects in Source Directory

- `as.wasm` - AssemblyScript WASM output
- `rust.wasm` - Rust WASM output (1.5MB)
- `test.js` - JavaScript test suite
- `index.ts` - TypeScript entry point
