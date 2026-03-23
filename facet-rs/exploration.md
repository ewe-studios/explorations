# Facet-rs Exploration

## Overview

Facet is a reflection/introspection library for Rust that provides types with a `SHAPE` associated const containing details on layout, fields, doc comments, attributes, and more. It enables runtime (and const-time) reflection for use cases like serialization/deserialization, pretty-printing, debuggers, CLI parsing, and code generation.

**Repository**: https://github.com/facet-rs/facet
**Crates.io**: https://crates.io/crates/facet
**Documentation**: https://docs.rs/facet

## Project Structure

The facet-rs project is organized as a workspace with multiple crates:

### Core Crates

| Crate | Purpose |
|-------|---------|
| `facet-core` | Core types and traits for the facet ecosystem |
| `facet` | Main re-export crate with derive macro |
| `facet-macros` | Derive macro implementation (powered by unsynn) |
| `facet-macros-parse` | Parsing logic for the derive macro |
| `facet-macros-emit` | Code emission logic for the derive macro |
| `facet-reflect` | Reflection utilities for peeking and poking values |

### Serialization/Deserialization Crates

| Crate | Purpose |
|-------|---------|
| `facet-serialize` | Core serialization framework |
| `facet-deserialize` | Core deserialization framework |
| `facet-json` | JSON format support |
| `facet-yaml` | YAML format support |
| `facet-toml` | TOML format support |
| `facet-msgpack` | MessagePack format support |
| `facet-csv` | CSV format support |
| `facet-kdl` | KDL format support |
| `facet-xdr` | XDR format support |
| `facet-urlencoded` | URL-encoded form data support |
| `facet-jsonschema` | JSON Schema generation |

### Utility Crates

| Crate | Purpose |
|-------|---------|
| `facet-pretty` | Pretty-printing for Facet types |
| `facet-args` | CLI argument parsing (clap-like) |
| `facet-dev` | Development helpers |
| `facet-testhelpers` | Test utilities |
| `facet-bench` | Benchmarking utilities |

### External Projects in the Repository

| Project | Purpose |
|---------|---------|
| `autotrait` | Auto-trait exploration |
| `dylo` | Dynamic loading exploration |
| `fopro` | Foreign function exploration |
| `home` | Large application built with facet |
| `limpid` | Terminal UI framework |
| `merde` | Another serialization library |

## Key Concepts

### The Facet Trait

The core of facet is the `Facet` trait:

```rust
pub unsafe trait Facet<'a>: 'a {
    /// The shape of this type
    const SHAPE: &'static Shape<'static>;

    /// Vtable for operations: Display, Debug, Clone, PartialEq, etc.
    const VTABLE: &'static ValueVTable;
}
```

**Safety Note**: Implementing this trait incorrectly makes the entire ecosystem unsafe. You're responsible for describing type layout properly and annotating all invariants.

### The Shape System

`Shape` is the central data structure containing:

- **id**: `TypeId` for unique identification
- **layout**: Size and alignment information
- **vtable**: Function pointers for operations
- **ty**: Type classification (Primitive, Sequence, User, Pointer)
- **def**: Semantic definition (Scalar, Map, List, etc.)
- **type_identifier**: Human-readable name
- **type_params**: Generic parameters
- **doc**: Doc comments
- **attributes**: Custom attributes

### Type Classification (`Type` enum)

```rust
pub enum Type<'shape> {
    Primitive(PrimitiveType),      // Built-in primitives
    Sequence(SequenceType<'shape>), // Tuples, arrays, slices
    User(UserType<'shape>),         // Structs, enums, unions
    Pointer(PointerType<'shape>),   // References, raw pointers, function pointers
}
```

### Semantic Definition (`Def` enum)

```rust
pub enum Def<'shape> {
    Undefined,                      // No semantic definition
    Scalar(ScalarDef<'shape>),      // u32, String, bool, SocketAddr, etc.
    Map(MapDef<'shape>),            // HashMap<String, T>
    Set(SetDef<'shape>),            // HashSet<T>
    List(ListDef<'shape>),          // Vec<T>
    Array(ArrayDef<'shape>),        // [T; 3]
    Slice(SliceDef<'shape>),        // [T]
    Option(OptionDef<'shape>),      // Option<T>
    SmartPointer(SmartPointerDef<'shape>), // Arc<T>, Rc<T>
}
```

## Design Philosophy

1. **Const-friendly**: Most shape information is available at compile time
2. **Type-erased pointers**: Uses `PtrConst`, `PtrMut`, `PtrUninit` for type-erased operations
3. **VTable-based dispatch**: Operations are function pointers in vtables
4. **No `syn` dependency**: Uses `unsynn` for lightweight macro parsing
5. **no_std support**: Core crates support no_std environments

## Usage Example

```rust
use facet::Facet;

#[derive(Facet)]
struct Person {
    name: String,
    age: u32,
}

// Access shape information
println!("Type: {}", Person::SHAPE);
println!("Fields: {:?}", Person::SHAPE.def);
```

## Supported Attributes

### Container Attributes
- `rename_all = "kebab-case"` - Rename fields/variants
- `transparent` - Newtype wrapper
- `deny_unknown_fields` - Reject unknown fields during deserialization
- `skip_serializing` - Don't serialize
- `invariants = "..."` - Custom invariants

### Field Attributes
- `rename = "..."` - Rename field
- `default` - Use Default value if missing
- `sensitive` - Hide in debug output
- `flatten` - Flatten nested structures
- `skip_serializing_if = "..."` - Conditional serialization

## Known Limitations

- Soundness issues are tracked and prioritized
- Format crates are slower than serde equivalents (runtime cost of reflection)
- `type_eq` is not const, limiting const fn usage
- Arbitrary attributes design is still evolving
