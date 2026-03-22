---
name: rust-flatbuffers
description: Core Rust implementation of FlatBuffers - zero-copy serialization library with maximum memory efficiency and cross-platform compatibility
type: sub-project
source: /home/darkvoid/Boxxed/@formulas/src.rust/src.wasm/flatbuffers/rust/flatbuffers/
---

# Rust FlatBuffers - Core Implementation

## Overview

The **rust-flatbuffers** crate is the official Rust implementation of Google's FlatBuffers serialization library. It provides zero-copy access to serialized data, meaning you can read data directly from the buffer without any parsing or unpacking overhead.

### Key Value Proposition

- **Zero-copy deserialization** - Access serialized data directly without parsing
- **Memory efficient** - No additional allocations beyond the initial buffer
- **Type-safe** - Full Rust type system guarantees at compile time
- **Schema-driven** - Generate code from `.fbs` schema files using `flatc`
- **Cross-platform** - Works with all FlatBuffers language implementations

### Example Usage

```rust
// Generated code from schema.fbs using: flatc --rust schema.fbs
use my_game::*;

// Creating a buffer
let mut builder = FlatBufferBuilder::new();
let name = builder.create_string("Orc");
let weapon = create_weapon(&mut builder, "Axe", 10);
let orc = create_monster(&mut builder, &MonsterArgs {
    name: Some(name),
    weapon: Some(weapon),
    ..Default::default()
});
builder.finish(orc, None);

// Access without parsing
let data = builder.finished_data();
let monster = root_as_monster(data).unwrap();
println!("Name: {}", monster.name());  // Direct memory access!
println!("Weapon: {}", monster.weapon().name());
```

## Directory Structure

```
/home/darkvoid/Boxxed/@formulas/src.rust/src.wasm/flatbuffers/rust/flatbuffers/
├── Cargo.toml                      # Package configuration
├── build.rs                        # Build script for schema generation
└── src/
    ├── lib.rs                      # Main library entry point
    ├── array.rs                    # Fixed-size array types
    ├── builder.rs                  # Buffer construction API
    ├── endian_scalar.rs            # Endianness handling
    ├── follow.rs                   # Follow trait for reading data
    ├── get_root.rs                 # Root type extraction
    ├── primitives.rs               # Core type primitives
    ├── push.rs                     # Push trait for writing data
    ├── table.rs                    # Table type implementation
    ├── vector.rs                   # Vector types (FixedVector, Vector)
    ├── verifier.rs                 # Safety verification for untrusted buffers
    ├── vtable.rs                   # Virtual table for field access
    └── vtable_writer.rs            # VTable serialization
```

## Core Components

### 1. Builder (`builder.rs`)

The `FlatBufferBuilder` is the primary API for constructing FlatBuffers.

```rust
pub struct FlatBufferBuilder<'b> {
    buf: Vec<u8>,           // Growing buffer
    head: usize,            // Current write position (from end)
    vtables: Vec<VTable>,   // Collected vtables
    finished: bool,         // Whether buffer is finished
    min_align: usize,       // Minimum alignment used
    _marker: PhantomData<&'b ()>,
}
```

**Key Methods:**

```rust
// Allocate space for a value
impl<'b> FlatBufferBuilder<'b> {
    pub fn new() -> Self { }

    pub fn push<V: Push>(&mut self, value: V) -> WIPOffset<V::Output> { }

    pub fn create_string(&mut self, s: &str) -> WIPOffset<&'b str> { }

    pub fn create_vector<'a, T: Push + 'a>(
        &mut self,
        items: &'a [T]
    ) -> WIPOffset<Vector<'b, T>> { }

    pub fn finish<W: Follow + Verifiable>(
        &mut self,
        root: WIPOffset<W>,
        file_identifier: Option<&[u8]>
    ) { }

    pub fn finished_data(&self) -> &[u8] { }
}
```

**Building Pattern:**

```rust
let mut builder = FlatBufferBuilder::new();

// Work backwards from innermost to outermost
let weapon_name = builder.create_string("Axe");
let weapon = create_weapon(&mut builder, weapon_name, 10);

let monster_name = builder.create_string("Orc");
let monster = create_monster(&mut builder, &MonsterArgs {
    name: Some(monster_name),
    weapon: Some(weapon),
    hp: 100,
    mana: 50,
});

builder.finish(monster, None);
let data = builder.finished_data();
```

### 2. Push Trait (`push.rs`)

The `Push` trait defines how types are written to the buffer:

```rust
pub trait Push: Sized {
    type Output;
    fn push(&self, dst: &mut [u8], _rest: &[u8], _written: &mut [u8]) -> Self::Output;
}

// Example implementation for u32
impl Push for u32 {
    type Output = u32;
    fn push(&self, dst: &mut [u8], _rest: &[u8], _written: &mut [u8]) -> u32 {
        dst.copy_from_slice(&self.to_le_bytes());
        u32::from_le_bytes(dst.try_into().unwrap())
    }
}
```

### 3. Follow Trait (`follow.rs`)

The `Follow` trait defines how types are read from the buffer:

```rust
pub trait Follow<'a> {
    unsafe fn follow(buf: &'a [u8], loc: usize) -> Self;
}

// Example for u32
impl<'a> Follow<'a> for u32 {
    unsafe fn follow(buf: &'a [u8], loc: usize) -> Self {
        let ptr = buf.as_ptr().add(loc);
        ptr.read_unaligned() as u32
    }
}
```

### 4. Table (`table.rs`)

Tables are the core data structure for FlatBuffers objects:

```rust
pub struct Table<'a> {
    buf: &'a [u8],
    loc: usize,  // Offset to vtable
}

impl<'a> Table<'a> {
    // Get a field from the table
    pub fn get<T: Follow<'a> + Verifiable>(
        &self,
        slot: usize,
        default: T
    ) -> T {
        let vtable = self.vtable();
        let offset = vtable.get(slot);
        if offset == 0 {
            return default;
        }
        T::follow(self.buf, self.loc + offset as usize)
    }
}
```

### 5. VTable (`vtable.rs`)

Virtual tables enable schema evolution and optional fields:

```
┌─────────────────────────────────────────────────────────────┐
│                      VTable Layout                          │
│                                                             │
│  ┌─────────────────┐  ← vtable start                       │
│  │ vtable_size     │  (2 bytes)                            │
│  ├─────────────────┤                                       │
│  │ object_size     │  (2 bytes)                            │
│  ├─────────────────┤                                       │
│  │ field_0_offset  │  (2 bytes) - relative to field start  │
│  ├─────────────────┤                                       │
│  │ field_1_offset  │  (2 bytes)                            │
│  ├─────────────────┤                                       │
│  │ ...             │                                       │
│  └─────────────────┘  ← vtable end                         │
│                                                             │
│  ┌─────────────────┐  ← object start (vtable_offset here)  │
│  │ vtable_offset   │  (4 bytes) - points to vtable         │
│  ├─────────────────┤                                       │
│  │ field_0_value   │                                       │
│  ├─────────────────┤                                       │
│  │ field_1_value   │                                       │
│  └─────────────────┘                                       │
└─────────────────────────────────────────────────────────────┘
```

**VTable Sharing:**

```rust
// Identical schemas share vtables
struct VTable {
    bytes: Vec<u8>,
    hash: u64,  // For quick comparison
}

impl FlatBufferBuilder {
    fn find_or_insert_vtable(&mut self, vt: &VTable) -> VTableOffset {
        // Check if identical vtable exists
        if let Some(existing) = self.vtables.iter().find(|v| v == vt) {
            return existing.offset;
        }
        // Write new vtable
        self.write_vtable(vt)
    }
}
```

### 6. Vector Types (`vector.rs`)

```rust
// Fixed-size vector (known at compile time)
pub struct FixedVector<'a, T: 'a> {
    buf: &'a [u8],
    len: usize,
    _marker: PhantomData<T>,
}

impl<'a, T: Follow<'a>> FixedVector<'a, T> {
    pub fn get(&self, i: usize) -> T {
        unsafe { T::follow(self.buf, i * size_of::<T>()) }
    }

    pub fn iter(&self) -> impl Iterator<Item = T> + 'a {
        (0..self.len).map(|i| self.get(i))
    }
}

// Dynamic vector (length stored in buffer)
pub struct Vector<'a, T: 'a> {
    buf: &'a [u8],
    loc: usize,
    _marker: PhantomData<T>,
}

impl<'a, T: Follow<'a> + Verifiable> Vector<'a, T> {
    pub fn len(&self) -> usize {
        unsafe { read_scalar_at::<u32>(self.buf, self.loc) as usize }
    }

    pub fn get(&self, i: usize) -> T {
        let offset = (i * size_of::<T>()) + size_of::<u32>();
        unsafe { T::follow(self.buf, self.loc + offset) }
    }
}
```

### 7. Verifier (`verifier.rs`)

Safety verification for untrusted buffers:

```rust
pub struct Verifier {
    buf: usize,      // Pointer to buffer start
    len: usize,      // Buffer length
    cur: usize,      // Current position
    depth: usize,    // Nesting depth (prevent stack overflow)
    max_depth: usize,
}

impl Verifier {
    pub fn new(buf: &[u8], max_depth: usize) -> Self { }

    pub fn verify<T: Verifiable>(&mut self) -> Result<(), Error> {
        // Check bounds
        if self.cur + size_of::<T>() > self.len {
            return Err(Error::OutOfBounds);
        }
        // Verify vtable if table type
        // Recurse with depth limit
        Ok(())
    }
}

// Usage for untrusted data
let verifier = Verifier::new(&untrusted_data, 64);
match verifier.verify_table::<Monster>() {
    Ok(_) => {
        // Safe to access
        let monster = root_as_monster_unchecked(&untrusted_data);
    }
    Err(e) => {
        // Buffer is malformed
        eprintln!("Invalid buffer: {:?}", e);
    }
}
```

## Schema Generation

### Build Script (`build.rs`)

```rust
use std::env;
use std::path::PathBuf;

fn main() {
    let manifest_dir = env::var("CARGO_MANIFEST_DIR").unwrap();
    let schema_path = format!("{}/schema.fbs", manifest_dir);

    // Run flatc to generate Rust code
    println!("cargo:rerun-if-changed={}", schema_path);

    // flatc is invoked via build-time dependency or system installation
    let output = Command::new("flatc")
        .args(&["--rust", "-o", &manifest_dir, &schema_path])
        .output()
        .expect("Failed to run flatc");

    if !output.status.success() {
        panic!("flatc failed: {}", String::from_utf8_lossy(&output.stderr));
    }
}
```

### Example Schema

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
  color: Color = Blue;
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

enum Color: byte {
  Red = 0,
  Green = 1,
  Blue = 2,
}

root_type Monster;
```

### Generated Code Structure

```rust
// monster_generated.rs
pub struct Monster<'a> {
    _tab: flatbuffers::Table<'a>,
}

impl<'a> flatbuffers::Follow<'a> for Monster<'a> {
    type Inner = Monster<'a>;
    unsafe fn follow(buf: &'a [u8], loc: usize) -> Self {
        Self { _tab: flatbuffers::Table::new(buf, loc) }
    }
}

pub struct MonsterArgs<'a> {
    pub name: Option<flatbuffers::WIPOffset<&'a str>>,
    pub hp: i32,
    pub mana: i32,
    pub pos: Option<flatbuffers::WIPOffset<Vec3>>,
    pub weapons: Option<flatbuffers::WIPOffset<flatbuffers::Vector<'a, flatbuffers::ForwardsUOffset<Weapon<'a>>>>>,
    pub inventory: Option<flatbuffers::WIPOffset<flatbuffers::Vector<'a, u8>>>,
    pub color: Color,
}

impl<'a> Monster<'a> {
    pub fn name(&self) -> Option<&'a str> {
        self._tab.get::<flatbuffers::ForwardsUOffset<&str>>(4, None)
    }

    pub fn hp(&self) -> i32 {
        self._tab.get::<i32>(6, 100)
    }

    pub fn pos(&self) -> Option<Vec3> {
        self._tab.get::<Vec3>(8, None)
    }

    pub fn weapons(&self) -> Option<flatbuffers::Vector<'a, Weapon<'a>>> {
        self._tab.get::<flatbuffers::ForwardsUOffset<flatbuffers::Vector<'a, Weapon>>>(10, None)
    }
}
```

## Memory Layout

```
┌─────────────────────────────────────────────────────────────────┐
│                    FlatBuffer Memory Layout                      │
│                                                                 │
│  Start →                    Buffer                              │
│            ┌──────────────────────────────────────────────────┐│
│            │                  Header                           ││
│            │  ┌─────────────────────────────────────────────┐ ││
│            │  │              VTable 1                        │ ││
│            │  │  [vtable_size][object_size][field_offsets]  │ ││
│            │  └─────────────────────────────────────────────┘ ││
│            │  ┌─────────────────────────────────────────────┐ ││
│            │  │              VTable 2                        │ ││
│            │  └─────────────────────────────────────────────┘ ││
│            │                      ...                          ││
│            └──────────────────────────────────────────────────┘│
│                                                                 │
│            ┌──────────────────────────────────────────────────┐│
│            │              Object Data                          ││
│            │  ┌──────────────┐  ┌──────────────┐              ││
│            │  │   Monster 1  │  │   Monster 2  │  ...         ││
│            │  │ vtable_off   │  │ vtable_off   │              ││
│            │  │ field values │  │ field values │              ││
│            │  └──────────────┘  └──────────────┘              ││
│            └──────────────────────────────────────────────────┘│
│                                                                 │
│            ┌──────────────────────────────────────────────────┐│
│            │           Variable-Length Data                    ││
│            │  ┌────────┐  ┌────────┐  ┌────────┐             ││
│            │  │ String │  │ Vector │  │ String │  ...        ││
│            │  │ "Orc"  │  │ [1,2,3]│  │ "Axe"  │             ││
│            │  └────────┘  └────────┘  └────────┘             ││
│            └──────────────────────────────────────────────────┘│
│                                                                 │
│            ┌──────────────────────────────────────────────────┐│
│            │           Root Object Offset                      ││
│            │  [offset to first Monster object]                ││
│            └──────────────────────────────────────────────────┘│
│                                                                 │
│  End   →   └──────────────────────────────────────────────────┘│
└─────────────────────────────────────────────────────────────────┘
```

## Performance Characteristics

| Operation | Time Complexity | Memory |
|-----------|----------------|--------|
| Field access | O(1) | No allocation |
| Vector access | O(1) | No allocation |
| String access | O(1) + len for use | No allocation |
| Buffer construction | O(n) | 1x buffer size |
| Verification | O(n) | Stack depth limit |

## Integration with Other Languages

FlatBuffers enables cross-language communication:

```
┌──────────────┐         ┌──────────────┐
│   Rust App   │         │  Node.js App │
│              │         │              │
│  Monster {   │  ───►   │  Monster {   │
│    name      │  bytes  │    name      │
│    hp        │         │    hp        │
│    pos       │         │    pos       │
│  }           │         │  }           │
└──────────────┘         └──────────────┘
     │                        │
     └───────────┬────────────┘
                 ▼
        ┌────────────────┐
        │  .fbs Schema   │
        │  (shared)      │
        └────────────────┘
```

## Key Insights

1. **Zero-copy is key** - All reads are direct memory accesses, no deserialization
2. **Vtables enable evolution** - Adding fields doesn't break old readers
3. **Builder pattern enforces order** - Must build from innermost to outermost
4. **Verification for safety** - Always verify untrusted buffers before access
5. **Alignment matters** - Data is aligned for native access efficiency

## Open Questions

- How does the no_std build work for embedded systems?
- What are the limitations of the current verifier?
- How to handle very large buffers efficiently?
