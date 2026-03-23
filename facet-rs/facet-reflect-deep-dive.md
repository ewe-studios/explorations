# facet-reflect Deep Dive

## Overview

`facet-reflect` provides the core reflection capabilities for facet, allowing you to:
- **Peek**: Read from values of arbitrary types
- **Poke/Partial**: Build values incrementally (work-in-progress construction)

This is the engine that powers serialization, deserialization, and dynamic type inspection.

## The Peek System (Reading)

### Peek - Core Read Interface

```rust
pub struct Peek<'mem, 'facet, 'shape> {
    pub(crate) data: GenericPtr<'mem>,
    pub(crate) shape: &'shape Shape<'shape>,
    invariant: PhantomData<fn(&'facet ()) -> &'facet ()>,
}

pub enum GenericPtr<'mem> {
    Thin(PtrConst<'mem>),   // For sized types
    Wide(PtrConstWide<'mem>), // For unsized types
}
```

### Creating Peek Values

```rust
impl<'mem, 'facet, 'shape> Peek<'mem, 'facet, 'shape> {
    /// For known types
    pub fn new<T: Facet<'facet> + ?Sized>(t: &'mem T) -> Self;

    /// For type-erased usage (unsafe)
    pub unsafe fn unchecked_new(
        data: impl Into<GenericPtr<'mem>>,
        shape: &'shape Shape<'shape>,
    ) -> Self;
}
```

### Peek Operations

```rust
impl<'mem, 'facet, 'shape> Peek<'mem, 'facet, 'shape> {
    /// Get the vtable
    pub fn vtable(&self) -> &'shape ValueVTable;

    /// Get unique ID for cycle detection
    pub fn id(&self) -> ValueId<'shape>;

    /// Check pointer equality
    pub fn ptr_eq(&self, other: &Peek<'_, '_, '_>) -> bool;

    /// Partial equality
    pub fn partial_eq(&self, other: &Peek<'_, '_, '_>) -> Option<bool>;

    /// Comparison
    pub fn partial_ord(&self, other: &Peek<'_, '_, '_>) -> Option<Ordering>;

    /// Hash
    pub fn hash(&self) -> Option<u64>;

    /// Debug formatting
    pub fn debug(&self, f: &mut Formatter) -> core::fmt::Result;

    /// Display formatting
    pub fn display(&self, f: &mut Formatter) -> core::fmt::Result;
}
```

### PeekStruct - Struct Introspection

```rust
pub struct PeekStruct<'mem, 'facet, 'shape> {
    pub(crate) value: Peek<'mem, 'facet, 'shape>,
    pub(crate) ty: StructType<'shape>,
}

impl<'mem, 'facet, 'shape> PeekStruct<'mem, 'facet, 'shape> {
    /// Get struct definition
    pub fn ty(&self) -> &StructType;

    /// Number of fields
    pub fn field_count(&self) -> usize;

    /// Get field by index
    pub fn field(&self, index: usize) -> Result<Peek<'mem, 'facet, 'shape>, FieldError>;

    /// Get field by name
    pub fn field_by_name(&self, name: &str) -> Result<Peek<'mem, 'facet, 'shape>, FieldError>;

    /// Iterate over fields
    pub fn fields(&self) -> FieldIter<'mem, 'facet, 'shape>;
}
```

### PeekEnum - Enum Introspection

```rust
pub struct PeekEnum<'mem, 'facet, 'shape> {
    pub(crate) value: Peek<'mem, 'facet, 'shape>,
    pub(crate) ty: EnumType<'shape>,
}

impl<'mem, 'facet, 'shape> PeekEnum<'mem, 'facet, 'shape> {
    /// Get current variant index
    pub fn variant_index(&self) -> usize;

    /// Get current variant name
    pub fn variant_name(&self) -> &'shape str;

    /// Get variant definition
    pub fn variant(&self) -> &'shape Variant<'shape>;

    /// Get data for current variant as PeekStruct
    pub fn data(&self) -> PeekStruct<'mem, 'facet, 'shape>;
}
```

### PeekList - List Introspection

```rust
pub struct PeekList<'mem, 'facet, 'shape> {
    pub(crate) value: Peek<'mem, 'facet, 'shape>,
    pub(crate) def: ListDef<'shape>,
}

impl<'mem, 'facet, 'shape> PeekList<'mem, 'facet, 'shape> {
    /// Get length
    pub fn len(&self) -> usize;

    /// Check if empty
    pub fn is_empty(&self) -> bool;

    /// Get element at index
    pub fn get(&self, index: usize) -> Option<Peek<'mem, 'facet, 'shape>>;

    /// Iterate over elements
    pub fn iter(&self) -> PeekListIter<'mem, 'facet, 'shape>;
}
```

### PeekMap - Map Introspection

```rust
pub struct PeekMap<'mem, 'facet, 'shape> {
    pub(crate) value: Peek<'mem, 'facet, 'shape>,
    pub(crate) def: MapDef<'shape>,
}

impl<'mem, 'facet, 'shape> PeekMap<'mem, 'facet, 'shape> {
    /// Get length
    pub fn len(&self) -> usize;

    /// Iterate over key-value pairs
    pub fn iter(&self) -> PeekMapIter<'mem, 'facet, 'shape>;
}

pub struct PeekMapIter<'mem, 'facet, 'shape> {
    // Iterates over (Peek<'mem, ...>, Peek<'mem, ...>) pairs
}
```

### PeekOption - Option Introspection

```rust
pub struct PeekOption<'mem, 'facet, 'shape> {
    pub(crate) value: Peek<'mem, 'facet, 'shape>,
}

impl<'mem, 'facet, 'shape> PeekOption<'mem, 'facet, 'shape> {
    /// Check if Some
    pub fn is_some(&self) -> bool;

    /// Check if None
    pub fn is_none(&self) -> bool;

    /// Get inner value if Some
    pub fn get(&self) -> Option<Peek<'mem, 'facet, 'shape>>;
}
```

### ScalarType - Scalar Value Access

```rust
pub enum ScalarType<'mem, 'facet, 'shape> {
    U8(u8),
    U16(u16),
    U32(u32),
    U64(u64),
    U128(u128),
    USize(usize),
    I8(i8),
    I16(i16),
    I32(i32),
    I64(i64),
    I128(i128),
    ISize(isize),
    F32(f32),
    F64(f64),
    Bool(bool),
    Char(char),
    Str(&'mem str),
    // ... more types
}

impl<'mem, 'facet, 'shape> Peek<'mem, 'facet, 'shape> {
    /// Try to get scalar value
    pub fn as_scalar(&self) -> Option<ScalarType<'mem, 'facet, 'shape>>;
}
```

## The Partial System (Writing)

### Partial - Work-In-Progress Value Construction

`Partial` (formerly `Wip`) allows building values incrementally:

```rust
pub struct Partial<'shape> {
    // Internal state for tracking construction progress
}

impl<'shape> Partial<'shape> {
    /// Allocate memory for a type
    pub fn alloc<T: Facet<'shape>>() -> Result<Self, ReflectError>;

    /// Allocate memory based on shape
    pub fn alloc_shape(shape: &'shape Shape<'shape>) -> Result<Self, ReflectError>;
}
```

### Basic Usage Pattern

```rust
// Allocate memory
let mut partial = Partial::alloc::<MyType>()?;

// Set simple fields
partial.set_field("name", "Alice")?;
partial.set_field("age", 30u32)?;

// Work with nested structures
partial.begin_field("address")?;
partial.set_field("street", "123 Main St")?;
partial.set_field("city", "Springfield")?;
partial.end()?;

// Build the final value
let value = partial.build()?;
```

### Chaining Style

```rust
let value = Partial::alloc::<T>()?
    .set_field("name", "Bob")?
    .begin_field("scores")?
        .set(vec![95, 87, 92])?
    .end()?
    .build()?;
```

### Setting Values

```rust
impl<'shape> Partial<'shape> {
    /// Set a field by name
    pub fn set_field(&mut self, name: &str, value: impl Facet<'shape>) -> Result<&mut Self, ReflectError>;

    /// Set a field by index
    pub fn set_nth_field(&mut self, index: usize, value: impl Facet<'shape>) -> Result<&mut Self, ReflectError>;

    /// Set the current value
    pub fn set(&mut self, value: impl Facet<'shape>) -> Result<&mut Self, ReflectError>;
}
```

### Beginning Nested Structures

```rust
impl<'shape> Partial<'shape> {
    /// Begin working on a field by name
    pub fn begin_field(&mut self, name: &str) -> Result<&mut Self, ReflectError>;

    /// Begin working on a field by index
    pub fn begin_nth_field(&mut self, index: usize) -> Result<&mut Self, ReflectError>;

    /// Begin working on a list item
    pub fn begin_list_item(&mut self) -> Result<&mut Self, ReflectError>;

    /// Begin working on a map key
    pub fn begin_map_key(&mut self, key: impl Facet<'shape>) -> Result<&mut Self, ReflectError>;

    /// Begin working on a nested value (general)
    pub fn begin(&mut self) -> Result<&mut Self, ReflectError>;

    /// Finish current nesting level
    pub fn end(&mut self) -> Result<&mut Self, ReflectError>;
}
```

### Building the Final Value

```rust
impl<'shape> Partial<'shape> {
    /// Build the final value
    pub fn build<T: Facet<'shape>>(mut self) -> Result<T, ReflectError>;

    /// Build into a specific memory location
    pub fn build_at(self, ptr: PtrUninit<'_>) -> Result<PtrMut<'_>, ReflectError>;

    /// Get the shape being built
    pub fn shape(&self) -> &'shape Shape<'shape>;
}
```

### Working with Collections

#### Lists

```rust
let mut partial = Partial::alloc::<Vec<String>>()?;

// Add items
partial.begin_list_item()?;
partial.set("first")?;
partial.end()?;

partial.begin_list_item()?;
partial.set("second")?;
partial.end()?;

let vec = partial.build()?;
```

#### Maps

```rust
let mut partial = Partial::alloc::<HashMap<String, i32>>()?;

partial.begin_map_key("key1")?;
partial.set(100)?;
partial.end()?;

partial.begin_map_key("key2")?;
partial.set(200)?;
partial.end()?;

let map = partial.build()?;
```

### Working with Enums

```rust
let mut partial = Partial::alloc::<MyEnum>()?;

// Set the variant
partial.set_variant("VariantName")?;

// Set variant fields
partial.set_field("field1", value1)?;

let value = partial.build()?;
```

## Error Handling

### ReflectError

```rust
pub enum ReflectError<'shape> {
    /// Tried to access field that doesn't exist
    NoSuchField { shape: &'shape Shape<'shape>, field: String },

    /// Index out of bounds
    IndexOutOfBounds { shape: &'shape Shape<'shape>, index: usize, len: usize },

    /// Type mismatch during set
    TypeMismatch { expected: &'shape Shape<'shape>, got: &'shape Shape<'shape> },

    /// Value already set
    AlreadySet { shape: &'shape Shape<'shape> },

    /// Required field not set
    RequiredFieldNotSet { shape: &'shape Shape<'shape>, field: &'shape str },

    /// Invalid variant
    InvalidVariant { shape: &'shape Shape<'shape>, variant: String },

    /// Cannot begin operation
    CannotBegin { shape: &'shape Shape<'shape> },

    /// Cannot end operation
    CannotEnd { shape: &'shape Shape<'shape> },

    /// Invariant violation
    InvariantViolation { shape: &'shape Shape<'shape>, message: String },

    /// Unsized type
    Unsized { shape: &'shape Shape<'shape> },
}
```

## Internal Architecture

### HeapValue

For building values that will be heap-allocated:

```rust
pub struct HeapValue {
    ptr: PtrMut<'static>,
    shape: &'static Shape<'static>,
    drop_fn: Option<fn(PtrMut<'_>)>,
}

impl HeapValue {
    /// Allocate and build a value
    pub fn build(shape: &'static Shape<'static>) -> Self;

    /// Get peek at value
    pub fn peek(&self) -> Peek<'_, '_, '_>;
}
```

### Field Tracking

The Partial type tracks which fields have been set:

```rust
struct FieldState {
    set: bool,
    // Additional state for nested construction
}

struct PartialState {
    fields: Vec<FieldState>,
    // Stack for nested construction
    stack: Vec<Frame>,
}
```

### Stack Frames for Nested Construction

```rust
enum Frame {
    Struct { partial: Partial<'shape>, field_index: usize },
    Enum { partial: Partial<'shape>, variant: usize },
    List { partial: Partial<'shape> },
    Map { partial: Partial<'shape>, key: Option<HeapValue> },
    Option { partial: Partial<'shape> },
}
```

## Design Principles

1. **Type Safety**: Respects type invariants during construction
2. **Incremental Building**: Values can be built field-by-field
3. **Nested Support**: Handles arbitrarily nested structures
4. **Error Recovery**: Clear error messages for debugging
5. **Memory Safety**: Uses type-erased pointers with proper lifetime tracking

## Performance Considerations

- Uses const evaluation where possible
- Minimizes allocations during peek operations
- Heap allocation only when building values
- Zero-copy reads for scalar types
