# Reflection System Deep Dive

## Overview

This document provides a comprehensive deep dive into Facet's reflection system architecture, covering:
- The Shape/PointerType system
- Type metadata and vtables
- Dynamic type inspection
- Serialization/deserialization from reflection
- Production-level reproduction patterns

## Architecture Overview

```
┌─────────────────────────────────────────────────────────────────┐
│                    Facet Ecosystem                               │
├─────────────────────────────────────────────────────────────────┤
│  facet (main crate)                                              │
│    └── re-exports facet-core, facet-macros, facet-reflect       │
├─────────────────────────────────────────────────────────────────┤
│  facet-core                                                      │
│    ├── Facet trait (SHAPE + VTABLE)                             │
│    ├── Shape struct (type metadata)                              │
│    ├── Type enum (classification)                                │
│    ├── Def enum (semantic definition)                            │
│    └── ValueVTable (operations)                                  │
├─────────────────────────────────────────────────────────────────┤
│  facet-macros                                                    │
│    └── #[derive(Facet)] proc macro                              │
├─────────────────────────────────────────────────────────────────┤
│  facet-reflect                                                   │
│    ├── Peek (read reflection)                                    │
│    └── Partial (write/construct reflection)                      │
├─────────────────────────────────────────────────────────────────┤
│  Serialization/Deserialization                                   │
│    ├── facet-serialize / facet-deserialize                       │
│    └── Format crates (json, yaml, toml, msgpack, etc.)          │
└─────────────────────────────────────────────────────────────────┘
```

## The Shape System

### Shape Anatomy

```rust
pub struct Shape<'shape> {
    // Identity
    pub id: ConstTypeId,              // Compiler-provided TypeId
    pub type_identifier: &'shape str, // "HashMap", "Vec", "MyStruct"

    // Memory layout
    pub layout: ShapeLayout,          // Size + alignment

    // Operations
    pub vtable: &'shape ValueVTable,  // Function pointers

    // Type system
    pub ty: Type<'shape>,             // Primitive/Sequence/User/Pointer
    pub def: Def<'shape>,             // Scalar/Map/List/Array/etc.

    // Generics
    pub type_params: &'shape [TypeParam<'shape>],

    // Documentation
    pub doc: &'shape [&'shape str],
    pub attributes: &'shape [ShapeAttribute<'shape>],

    // Self-describing formats
    pub type_tag: Option<&'shape str>,

    // Newtype wrappers
    pub inner: Option<fn() -> &'shape Shape<'shape>>,
}
```

### Shape Generation (Macro-Expanded)

For a type like:
```rust
#[derive(Facet)]
struct Person {
    name: String,
    age: u32,
}
```

The macro generates:
```rust
unsafe impl<'a> Facet<'a> for Person {
    const SHAPE: &'static Shape<'static> = &const {
        Shape::builder_for_sized::<Self>()
            .type_identifier("Person")
            .ty(Type::User(UserType::Struct(StructType {
                repr: Repr::Rust,
                kind: StructKind::Struct,
                fields: &[
                    Field::builder()
                        .name("name")
                        .shape(<String as Facet>::SHAPE)
                        .offset(offset_of!(Person, name))
                        .flags(FieldFlags::EMPTY)
                        .doc(&[])
                        .vtable(&const { FieldVTable { ... } })
                        .build(),
                    Field::builder()
                        .name("age")
                        .shape(<u32 as Facet>::SHAPE)
                        .offset(offset_of!(Person, age))
                        .flags(FieldFlags::EMPTY)
                        .doc(&[])
                        .vtable(&const { FieldVTable { ... } })
                        .build(),
                ],
            })))
            .def(Def::Scalar(ScalarDef::builder()
                .affinity(&const { ScalarAffinity::opaque().build() })
                .build()))
            .vtable(&const { value_vtable!(Person, |f, _opts| write!(f, "Person")) })
            .build()
    };

    const VTABLE: &'static ValueVTable = &const {
        value_vtable!(Person, |f, _opts| write!(f, "Person"))
    };
}
```

## Pointer Type System

### Type-Erased Pointers

Facet uses custom pointer types for type erasure:

```rust
/// Immutable type-erased pointer
pub struct PtrConst<'mem>(NonNull<u8>, PhantomData<&'mem ()>);

/// Mutable type-erased pointer
pub struct PtrMut<'mem>(NonNull<u8>, PhantomData<&'mem mut ()>);

/// Uninitialized type-erased pointer
pub struct PtrUninit<'mem>(*mut u8, PhantomData<&'mem mut ()>);

/// Wide pointer support (for slices, trait objects)
pub struct PtrConstWide<'mem>(*mut (), usize, PhantomData<&'mem ()>);
pub struct PtrMutWide<'mem>(*mut (), usize, PhantomData<&'mem mut ()>);
```

### GenericPtr - Unified Thin/Wide Handling

```rust
pub enum GenericPtr<'mem> {
    Thin(PtrConst<'mem>),    // For Sized types
    Wide(PtrConstWide<'mem>), // For !Sized types
}

impl<'mem> GenericPtr<'mem> {
    /// Create from typed pointer
    fn new<T: ?Sized>(ptr: *const T) -> Self {
        if size_of_val(&ptr) == size_of::<PtrConst>() {
            GenericPtr::Thin(PtrConst::new(ptr.cast::<()>()))
        } else {
            GenericPtr::Wide(PtrConstWide::new(ptr))
        }
    }

    /// Dereference to get typed reference (unsafe)
    unsafe fn get<T: ?Sized>(self) -> &'mem T {
        match self {
            GenericPtr::Thin(ptr) => transmute(ptr.as_byte_ptr()),
            GenericPtr::Wide(ptr) => ptr.get(),
        }
    }
}
```

## VTable System

### ValueVTable Structure

```rust
pub enum ValueVTable {
    Sized(ValueVTableSized),
    Unsized(ValueVTableUnsized),
}

pub struct ValueVTableSized {
    // Type information
    pub type_name: TypeNameFn,
    pub marker_traits: fn() -> MarkerTraits,

    // Memory management
    pub drop_in_place: fn() -> Option<DropInPlaceFn>,
    pub invariants: fn() -> Option<InvariantsFn>,

    // Formatting
    pub display: fn() -> Option<DisplayFn>,
    pub debug: fn() -> Option<DebugFn>,

    // Construction
    pub default_in_place: fn() -> Option<DefaultInPlaceFn>,
    pub clone_into: fn() -> Option<CloneIntoFn>,

    // Comparison
    pub partial_eq: fn() -> Option<PartialEqFn>,
    pub partial_ord: fn() -> Option<PartialOrdFn>,
    pub ord: fn() -> Option<CmpFn>,

    // Hashing
    pub hash: fn() -> Option<HashFn>,

    // Conversion
    pub parse: fn() -> Option<ParseFn>,
    pub try_from: fn() -> Option<TryFromFn>,

    // Newtype support
    pub try_into_inner: fn() -> Option<TryIntoInnerFn>,
    pub try_borrow_inner: fn() -> Option<TryBorrowInnerFn>,
}
```

### VTable Function Signatures

```rust
// Type name formatting
pub type TypeNameFn = fn(f: &mut Formatter, opts: TypeNameOpts) -> fmt::Result;

// Memory operations
pub type DropInPlaceFn = unsafe fn(PtrMut<'_>) -> PtrUninit<'_>;
pub type CloneIntoFn = unsafe fn(PtrConst<'_>, PtrUninit<'_>) -> PtrMut<'_>;
pub type DefaultInPlaceFn = unsafe fn(PtrUninit<'_>) -> PtrMut<'_>;

// Comparison
pub type PartialEqFn = unsafe fn(PtrConst<'_>, PtrConst<'_>) -> bool;
pub type PartialOrdFn = unsafe fn(PtrConst<'_>, PtrConst<'_>) -> Option<Ordering>;
pub type CmpFn = unsafe fn(PtrConst<'_>, PtrConst<'_>) -> Ordering;

// Hashing
pub type HashFn = unsafe fn(PtrConst<'_>, hasher: PtrMut<'_>, write: HasherWriteFn);

// Conversion
pub type ParseFn = unsafe fn(&str, PtrUninit<'_>) -> Result<PtrMut<'_>, ParseError>;
pub type TryFromFn = unsafe fn(
    source: PtrConst<'_>,
    source_shape: &Shape<'_>,
    target: PtrUninit<'_>,
) -> Result<PtrMut<'_>, TryFromError<'_>>;
```

### VTable Generation Macro

```rust
macro_rules! value_vtable {
    ($type_name:ty, $type_name_fn:expr) => {
        const {
            $crate::ValueVTable::builder::<$type_name>()
                .type_name($type_name_fn)
                .display(|| {
                    if impls!($type_name: core::fmt::Display) {
                        Some(|data, f| Spez(data).spez_display(f))
                    } else { None }
                })
                .debug(|| { /* similar */ })
                .partial_eq(|| { /* similar */ })
                // ... more traits
                .build()
        }
    };
}
```

## Dynamic Type Inspection

### Using Peek

```rust
use facet_reflect::Peek;
use facet::Facet;

fn inspect<T: Facet<'static>>(value: &T) {
    let peek = Peek::new(value);

    // Get shape info
    println!("Type: {}", peek.shape().type_identifier);
    println!("Layout: {:?}", peek.shape().layout);

    // Match on type classification
    match peek.shape().ty {
        Type::Primitive(primitive) => {
            println!("Primitive: {:?}", primitive);
        }
        Type::User(UserType::Struct(struct_ty)) => {
            println!("Struct with {} fields", struct_ty.fields.len());
        }
        Type::User(UserType::Enum(enum_ty)) => {
            println!("Enum with {} variants", enum_ty.variants.len());
        }
        // ...
    }

    // Match on semantic definition
    match peek.shape().def {
        Def::Scalar(_) => println!("Scalar type"),
        Def::List(_) => println!("List type"),
        Def::Map(_) => println!("Map type"),
        // ...
    }
}
```

### Peeking into Structs

```rust
fn peek_struct<T: Facet<'static>>(value: &T) {
    let peek = Peek::new(value);

    if let Ok(struct_val) = peek.into_struct() {
        for (i, field) in struct_val.fields().enumerate() {
            let (name, field_peek) = field;
            println!("Field {}: {} = {:?}", i, name, field_peek.shape());

            // Get scalar value if possible
            if let Some(scalar) = field_peek.as_scalar() {
                println!("  Value: {:?}", scalar);
            }
        }
    }
}
```

### Peeking into Enums

```rust
fn peek_enum<T: Facet<'static>>(value: &T) {
    let peek = Peek::new(value);

    if let Ok(enum_val) = peek.into_enum() {
        println!("Variant: {} (index {})",
                 enum_val.variant_name(),
                 enum_val.variant_index());

        // Access variant data
        let data = enum_val.data();
        for field in data.fields() {
            // Process variant fields
        }
    }
}
```

## Serialization from Reflection

### Serializer Trait

```rust
pub trait Serializer<'shape> {
    type Error;

    // Primitives
    fn serialize_u64(&mut self, value: u64) -> Result<(), Self::Error>;
    fn serialize_i64(&mut self, value: i64) -> Result<(), Self::Error>;
    fn serialize_f64(&mut self, value: f64) -> Result<(), Self::Error>;
    fn serialize_bool(&mut self, value: bool) -> Result<(), Self::Error>;
    fn serialize_str(&mut self, value: &str) -> Result<(), Self::Error>;
    fn serialize_bytes(&mut self, value: &[u8]) -> Result<(), Self::Error>;

    // Special values
    fn serialize_none(&mut self) -> Result<(), Self::Error>;
    fn serialize_unit(&mut self) -> Result<(), Self::Error>;
    fn serialize_unit_variant(&mut self, idx: usize, name: &'shape str) -> Result<(), Self::Error>;

    // Containers
    fn start_object(&mut self, len: Option<usize>) -> Result<(), Self::Error>;
    fn serialize_field_name(&mut self, name: &'shape str) -> Result<(), Self::Error>;
    fn start_array(&mut self, len: Option<usize>) -> Result<(), Self::Error>;
    fn start_map(&mut self, len: Option<usize>) -> Result<(), Self::Error>;
}
```

### Core Serialization Logic

```rust
pub fn serialize<'shape, S: Serializer<'shape>>(
    peek: &Peek<'_, '_, 'shape>,
    serializer: &mut S,
) -> Result<(), S::Error> {
    match peek.shape().def {
        Def::Scalar(scalar_def) => {
            match scalar_def.affinity {
                ScalarAffinity::Number(num) => {
                    // Extract number based on bits
                    match num.bits {
                        NumberBits::Integer { size, sign } => {
                            match (size, sign) {
                                (IntegerSize::Fixed(64), Signedness::Unsigned) => {
                                    let v = peek.as_scalar().unwrap().as_u64().unwrap();
                                    serializer.serialize_u64(v)
                                }
                                // ... handle other integer types
                            }
                        }
                        // ... handle floats
                    }
                }
                ScalarAffinity::String(_) => {
                    let s = peek.as_scalar().unwrap().as_str().unwrap();
                    serializer.serialize_str(s)
                }
                ScalarAffinity::Boolean(_) => {
                    let b = peek.as_scalar().unwrap().as_bool().unwrap();
                    serializer.serialize_bool(b)
                }
                // ... handle other scalar affinities
            }
        }

        Def::Struct(_) => {
            let struct_val = peek.into_struct().unwrap();
            serializer.start_object(Some(struct_val.field_count()))?;

            for field_result in struct_val.fields() {
                let (name, field_peek) = field_result;
                serializer.serialize_field_name(name)?;
                serialize(&field_peek, serializer)?;
            }

            // Object end handled by Drop or explicit call
        }

        Def::Enum(_) => {
            let enum_val = peek.into_enum().unwrap();
            let variant = enum_val.variant();

            if variant.data.fields.is_empty() {
                // Unit variant
                serializer.serialize_unit_variant(
                    enum_val.variant_index(),
                    variant.name,
                )?;
            } else {
                // Tuple/struct variant
                serializer.start_object(Some(variant.data.fields.len()))?;
                // Serialize variant fields...
            }
        }

        Def::List(_) => {
            let list = peek.into_list_like().unwrap();
            serializer.start_array(Some(list.len()))?;

            for item in list.iter() {
                serialize(&item, serializer)?;
            }
        }

        Def::Map(_) => {
            let map = peek.into_map().unwrap();
            serializer.start_map(Some(map.len()))?;

            for (key, value) in map.iter() {
                serialize(&key, serializer)?;
                serialize(&value, serializer)?;
            }
        }

        Def::Option(_) => {
            let opt = peek.into_option().unwrap();
            if opt.is_some() {
                serialize(&opt.get().unwrap(), serializer)?;
            } else {
                serializer.serialize_none()?;
            }
        }

        // ... handle Array, Set, SmartPointer, etc.
    }
}
```

## Deserialization from Reflection

### Deserializer Pattern

```rust
pub trait Deserializer<'input> {
    type Error;

    fn next(&mut self) -> Result<Outcome<'input>, Self::Error>;
    fn peek(&mut self) -> Result<Option<&Outcome<'input>>, Self::Error>;
}

pub enum Outcome<'input> {
    Scalar(Scalar<'input>),
    ListStarted,
    ListEnded,
    ObjectStarted,
    ObjectEnded,
    // ...
}
```

### Partial-Based Deserialization

```rust
pub fn deserialize<'input, 'shape, T: Facet<'shape>>(
    deserializer: &mut impl Deserializer<'input>,
) -> Result<T, DeserializerError> {
    let mut partial = Partial::alloc::<T>()?;
    deserialize_into(&mut partial, deserializer)?;
    Ok(partial.build()?)
}

fn deserialize_into<'input, 'shape>(
    partial: &mut Partial<'shape>,
    deserializer: &mut impl Deserializer<'input>,
) -> Result<(), DeserializerError> {
    match deserializer.next()? {
        Outcome::Scalar(scalar) => {
            // Match scalar to target type
            match scalar {
                Scalar::U64(v) => partial.set(v)?,
                Scalar::I64(v) => partial.set(v)?,
                Scalar::F64(v) => partial.set(v)?,
                Scalar::Bool(v) => partial.set(v)?,
                Scalar::String(s) => partial.set(s.as_ref())?,
                // ...
            }
        }

        Outcome::ObjectStarted => {
            // Handle struct/enum
            while let Some(key) = expect_object_key(deserializer)? {
                partial.begin_field(&key)?;
                deserialize_into(partial, deserializer)?;
                partial.end()?;
            }
            expect_object_end(deserializer)?;
        }

        Outcome::ListStarted => {
            // Handle list/array
            while !is_list_end(deserializer)? {
                partial.begin_list_item()?;
                deserialize_into(partial, deserializer)?;
                partial.end()?;
            }
            expect_list_end(deserializer)?;
        }

        // ... handle other outcomes
    }

    Ok(())
}
```

## Reproduction Patterns for Production

### 1. Basic Shape Definition

```rust
use facet_core::{Facet, Shape, ValueVTable, Type, Def, ScalarDef, ScalarAffinity};

struct MyType {
    value: i32,
}

unsafe impl<'a> Facet<'a> for MyType {
    const SHAPE: &'static Shape<'static> = &const {
        Shape::builder_for_sized::<Self>()
            .type_identifier("MyType")
            .ty(Type::User(facet_core::UserType::Opaque))
            .def(Def::Scalar(
                ScalarDef::builder()
                    .affinity(&const { ScalarAffinity::opaque().build() })
                    .build(),
            ))
            .build()
    };

    const VTABLE: &'static ValueVTable = &const {
        facet_core::value_vtable!(MyType, |f, _opts| write!(f, "MyType"))
    };
}
```

### 2. Struct with Fields

```rust
use facet_core::{
    Facet, Shape, ValueVTable, Type, UserType, StructType, StructKind,
    Repr, Field, FieldFlags, FieldVTable, offset_of,
};

#[repr(C)]
struct Point {
    x: f64,
    y: f64,
}

unsafe impl<'a> Facet<'a> for Point {
    const SHAPE: &'static Shape<'static> = &const {
        const FIELDS: &[Field] = &[
            Field::builder()
                .name("x")
                .shape(<f64 as Facet>::SHAPE)
                .offset(offset_of!(Point, x))
                .flags(FieldFlags::EMPTY)
                .doc(&[])
                .vtable(&const { FieldVTable {
                    skip_serializing_if: None,
                    default_fn: None
                } })
                .build(),
            Field::builder()
                .name("y")
                .shape(<f64 as Facet>::SHAPE)
                .offset(offset_of!(Point, y))
                .flags(FieldFlags::EMPTY)
                .doc(&[])
                .vtable(&const { FieldVTable {
                    skip_serializing_if: None,
                    default_fn: None
                } })
                .build(),
        ];

        Shape::builder_for_sized::<Self>()
            .type_identifier("Point")
            .ty(Type::User(UserType::Struct(StructType {
                repr: Repr::C,
                kind: StructKind::Struct,
                fields: FIELDS,
            })))
            .build()
    };

    const VTABLE: &'static ValueVTable = &const {
        facet_core::value_vtable!(Point, |f, _opts| write!(f, "Point"))
    };
}
```

### 3. Generic Container

```rust
use facet_core::{
    Facet, Shape, ValueVTable, Type, Def, ListDef, ListVTable,
    PtrConst, PtrMut, PtrUninit,
};

struct Container<T> {
    items: Vec<T>,
}

unsafe impl<'a, T: Facet<'a>> Facet<'a> for Container<T> {
    const SHAPE: &'static Shape<'static> = &const {
        Shape::builder_for_sized::<Self>()
            .type_identifier("Container")
            .type_params(&[facet_core::TypeParam {
                name: "T",
                shape: T::SHAPE,
            }])
            .ty(Type::User(UserType::Opaque))
            .def(Def::List(ListDef::builder()
                .t(T::SHAPE)
                .vtable(&const {
                    ListVTable::builder()
                        .len(|list| unsafe {
                            let container = &*(list.as_byte_ptr() as *const Container<T>);
                            container.items.len()
                        })
                        .get(|list, index| unsafe {
                            let container = &*(list.as_byte_ptr() as *const Container<T>);
                            container.items.get(index).map(|item| {
                                PtrConst::new(item as *const T)
                            })
                        })
                        .as_ptr(|list| unsafe {
                            let container = &*(list.as_byte_ptr() as *const Container<T>);
                            PtrConst::new(container.items.as_ptr() as *const ())
                        })
                        .iter_vtable(facet_core::IterVTable {
                            // Iterator vtable implementation
                        })
                        .build()
                })
                .build()))
            .build()
    };

    const VTABLE: &'static ValueVTable = &const {
        facet_core::value_vtable!(Container<T>, |f, opts| {
            write!(f, "Container<{}>", T::SHAPE.type_identifier)
        })
    };
}
```

### 4. Enum Implementation

```rust
use facet_core::{
    Facet, Shape, ValueVTable, Type, UserType, EnumType, EnumRepr,
    Variant, StructType, StructKind, Repr, Field,
};

#[repr(u8)]
enum Message {
    Quit,
    Move { x: i32, y: i32 },
    Write(String),
}

unsafe impl<'a> Facet<'a> for Message {
    const SHAPE: &'static Shape<'static> = &const {
        const VARIANTS: &[Variant] = &[
            Variant::builder()
                .name("Quit")
                .discriminant(0)
                .data(StructType::builder()
                    .repr(Repr::Rust)
                    .kind(StructKind::Unit)
                    .fields(&[])
                    .build())
                .build(),
            Variant::builder()
                .name("Move")
                .discriminant(1)
                .data(StructType::builder()
                    .repr(Repr::Rust)
                    .kind(StructKind::Struct)
                    .fields(&[
                        Field::builder()
                            .name("x")
                            .shape(<i32 as Facet>::SHAPE)
                            .offset(0)  // After discriminant
                            .build(),
                        Field::builder()
                            .name("y")
                            .shape(<i32 as Facet>::SHAPE)
                            .offset(4)
                            .build(),
                    ])
                    .build())
                .build(),
            // ... Write variant
        ];

        Shape::builder_for_sized::<Self>()
            .type_identifier("Message")
            .ty(Type::User(UserType::Enum(EnumType {
                repr: Repr::Rust,
                enum_repr: EnumRepr::U8,
                variants: VARIANTS,
            })))
            .build()
    };

    const VTABLE: &'static ValueVTable = &const {
        facet_core::value_vtable!(Message, |f, _opts| write!(f, "Message"))
    };
}
```

## Key Design Patterns

### 1. Lazy Shape Resolution

Use `fn() -> &'static Shape` for recursive types:

```rust
const fn node_shape() -> &'static Shape<'static> {
    <Node as Facet>::SHAPE
}

Field::builder()
    .name("children")
    .shape(node_shape)  // Indirection prevents infinite recursion
    // ...
```

### 2. Const Evaluation

All shapes are const-computed:

```rust
const SHAPE: &'static Shape<'static> = &const {
    // All computation happens at compile time
    Shape::builder_for_sized::<Self>()
        // ...
        .build()
};
```

### 3. Specialization via Spez

Autoderef specialization for trait detection:

```rust
pub mod spez {
    pub struct Spez<T>(pub T);

    impl<T: Display> Spez<&T> {
        pub fn spez_display(self, f: &mut Formatter) -> fmt::Result {
            self.0.fmt(f)
        }
    }

    // Fallback for non-Display types
    impl<T> Spez<&T> {
        pub fn spez_display(self, f: &mut Formatter) -> fmt::Result {
            write!(f, "<not implement Display>")
        }
    }
}
```

### 4. Marker Traits via Bitflags

```rust
bitflags! {
    pub struct MarkerTraits: u8 {
        const EQ = 1 << 0;
        const SEND = 1 << 1;
        const SYNC = 1 << 2;
        const COPY = 1 << 3;
    }
}

// Detection
marker_traits: || {
    let mut traits = MarkerTraits::empty();
    if impls!(T: Eq) {
        traits = traits.union(MarkerTraits::EQ);
    }
    if impls!(T: Send) {
        traits = traits.union(MarkerTraits::SEND);
    }
    traits
}
```

## Production Considerations

### Memory Layout

- Ensure `#[repr(C)]` for predictable offsets
- Use `offset_of!` macro for field offsets
- Consider alignment requirements

### Performance

- Minimize runtime computation (use const)
- Cache shape lookups where possible
- Use inlining hints for hot paths

### Safety

- Document all unsafe blocks
- Validate pointer operations
- Check bounds on field access

### Extensibility

- Use `ShapeAttribute::Arbitrary` for custom metadata
- Leverage `type_tag` for format-specific handling
- Consider `inner` for transparent wrappers
