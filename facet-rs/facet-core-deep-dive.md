# facet-core Deep Dive

## Overview

`facet-core` is the foundational crate that defines the core types and traits for the facet ecosystem. It provides:

- The `Facet` trait
- The `Shape` struct and related type definitions
- VTable structures for dynamic operations
- Type-erased pointer utilities
- Implementations for core/std types

## The Facet Trait

```rust
pub unsafe trait Facet<'a>: 'a {
    /// The shape of this type
    const SHAPE: &'static Shape<'static>;

    /// Vtable for operations: Display, Debug, Clone, PartialEq, etc.
    const VTABLE: &'static ValueVTable;
}
```

### Safety Contract

This is an unsafe trait because incorrect implementations break the entire ecosystem:
- Type layout must be described correctly
- All invariants must be annotated
- VTable functions must match the actual type behavior

## Shape Structure

```rust
pub struct Shape<'shape> {
    /// Unique type identifier (from TypeId)
    pub id: ConstTypeId,

    /// Size and alignment
    pub layout: ShapeLayout,

    /// Operations vtable
    pub vtable: &'shape ValueVTable,

    /// Type classification
    pub ty: Type<'shape>,

    /// Semantic definition
    pub def: Def<'shape>,

    /// Human-readable name
    pub type_identifier: &'shape str,

    /// Generic parameters
    pub type_params: &'shape [TypeParam<'shape>],

    /// Doc comments
    pub doc: &'shape [&'shape str],

    /// Attributes
    pub attributes: &'shape [ShapeAttribute<'shape>],

    /// Type tag for self-describing formats
    pub type_tag: Option<&'shape str>,

    /// Inner type for newtype wrappers
    pub inner: Option<fn() -> &'shape Shape<'shape>>,
}
```

### ShapeLayout

```rust
pub enum ShapeLayout {
    Sized(Layout),    // Standard sized type
    Unsized,          // !Sized types like str, [T]
}
```

## Type Classification System

### Primitive Types

```rust
pub enum PrimitiveType {
    // Integer types
    Integer(IntegerType),
    // Floating point
    Float(FloatType),
    // Text types
    Textual(TextualType),
    // Boolean
    Boolean,
    // Never type
    Never,
}
```

### User Types

```rust
pub enum UserType<'shape> {
    Struct(StructType<'shape>),
    Enum(EnumType<'shape>),
    Union(UnionType<'shape>),
    Opaque,  // For opaque wrapper types
}
```

### Pointer Types

```rust
pub enum PointerType<'shape> {
    Reference(ValuePointerType<'shape>),  // & and &mut
    Raw(ValuePointerType<'shape>),        // *const and *mut
    Function(FunctionPointerDef),         // fn() -> T
}

pub struct ValuePointerType<'shape> {
    pub mutable: bool,      // mut or not
    pub wide: bool,         // fat pointer or thin
    pub target: fn() -> &'shape Shape<'shape>,
}
```

## VTable System

### ValueVTable

The vtable is an enum that handles both sized and unsized types:

```rust
pub enum ValueVTable {
    Sized(ValueVTableSized),
    Unsized(ValueVTableUnsized),
}

pub struct ValueVTableSized {
    pub type_name: TypeNameFn,
    pub marker_traits: fn() -> MarkerTraits,
    pub drop_in_place: fn() -> Option<DropInPlaceFn>,
    pub invariants: fn() -> Option<InvariantsFn>,
    pub display: fn() -> Option<DisplayFn>,
    pub debug: fn() -> Option<DebugFn>,
    pub default_in_place: fn() -> Option<DefaultInPlaceFn>,
    pub clone_into: fn() -> Option<CloneIntoFn>,
    pub partial_eq: fn() -> Option<PartialEqFn>,
    pub partial_ord: fn() -> Option<PartialOrdFn>,
    pub ord: fn() -> Option<CmpFn>,
    pub hash: fn() -> Option<HashFn>,
    pub parse: fn() -> Option<ParseFn>,
    pub try_from: fn() -> Option<TryFromFn>,
    pub try_into_inner: fn() -> Option<TryIntoInnerFn>,
    pub try_borrow_inner: fn() -> Option<TryBorrowInnerFn>,
}
```

### Marker Traits

```rust
bitflags! {
    pub struct MarkerTraits: u8 {
        const EQ           = 1 << 0;
        const SEND         = 1 << 1;
        const SYNC         = 1 << 2;
        const COPY         = 1 << 3;
        const UNPIN        = 1 << 4;
        const UNWIND_SAFE  = 1 << 5;
        const REF_UNWIND_SAFE = 1 << 6;
    }
}
```

## Scalar Affinity System

Facet classifies scalars by their "affinity" - what they spiritually are:

```rust
pub enum ScalarAffinity<'shape> {
    Number(NumberAffinity<'shape>),
    ComplexNumber(ComplexNumberAffinity<'shape>),
    String(StringAffinity),
    Boolean(BoolAffinity),
    Empty(EmptyAffinity),
    SocketAddr(SocketAddrAffinity),
    IpAddr(IpAddrAffinity),
    Url(UrlAffinity),
    UUID(UuidAffinity),
    ULID(UlidAffinity),
    Time(TimeAffinity<'shape>),
    Opaque(OpaqueAffinity),
    Other(OtherAffinity),
    Char(CharAffinity),
    Path(PathAffinity),
}
```

### Number Affinity Details

```rust
pub struct NumberAffinity<'shape> {
    pub bits: NumberBits,
    pub min: PtrConst<'shape>,
    pub max: PtrConst<'shape>,
    pub positive_infinity: Option<PtrConst<'shape>>,
    pub negative_infinity: Option<PtrConst<'shape>>,
    pub nan_sample: Option<PtrConst<'shape>>,
    pub positive_zero: Option<PtrConst<'shape>>,
    pub negative_zero: Option<PtrConst<'shape>>,
    pub epsilon: Option<PtrConst<'shape>>,
}

pub enum NumberBits {
    Integer { size: IntegerSize, sign: Signedness },
    Float { sign_bits: usize, exponent_bits: usize, mantissa_bits: usize, ... },
    Fixed { sign_bits: usize, integer_bits: usize, fraction_bits: usize },
    Decimal { sign_bits: usize, integer_bits: usize, scale_bits: usize },
}
```

## Type-Erased Pointers

Facet uses type-erased pointers to work with values generically:

### PtrConst - Read-only pointer

```rust
pub struct PtrConst<'mem>(NonNull<u8>, PhantomData<&'mem ()>);

impl<'mem> PtrConst<'mem> {
    pub const fn new<T>(ptr: *const T) -> Self;
    pub const fn as_byte_ptr(self) -> *const u8;
    pub unsafe fn field(self, offset: usize) -> PtrConst<'mem>;
}
```

### PtrMut - Mutable pointer

```rust
pub struct PtrMut<'mem>(NonNull<u8>, PhantomData<&'mem mut ()>);

impl<'mem> PtrMut<'mem> {
    pub fn as_byte_ptr(self) -> *mut u8;
    pub unsafe fn field(self, offset: usize) -> PtrMut<'mem>;
}
```

### PtrUninit - Uninitialized memory pointer

```rust
pub struct PtrUninit<'mem>(*mut u8, PhantomData<&'mem mut ()>);

impl<'mem> PtrUninit<'mem> {
    pub fn new<T>(ptr: *mut T) -> Self;
    pub unsafe fn put<T>(self, value: T) -> PtrMut<'mem>;
    pub unsafe fn copy_from<'src>(self, src: PtrConst<'src>, ...) -> Result<PtrMut<'mem>, UnsizedError>;
}
```

## Def (Semantic Definition)

The `Def` enum describes what a type semantically is:

### ListDef

```rust
pub struct ListDef<'shape> {
    pub vtable: &'shape ListVTable,
    pub t: fn() -> &'shape Shape<'shape>,  // Element type
}

pub struct ListVTable {
    pub init_in_place_with_capacity: Option<ListInitInPlaceWithCapacityFn>,
    pub push: Option<ListPushFn>,
    pub len: ListLenFn,
    pub get: ListGetFn,
    pub get_mut: Option<ListGetMutFn>,
    pub as_ptr: Option<ListAsPtrFn>,
    pub as_mut_ptr: Option<ListAsMutPtrFn>,
    pub iter_vtable: IterVTable<PtrConst<'static>>,
}
```

### MapDef

```rust
pub struct MapDef<'shape> {
    pub vtable: &'shape MapVTable,
    pub k: fn() -> &'shape Shape<'shape>,  // Key type
    pub v: fn() -> &'shape Shape<'shape>,  // Value type
}
```

### OptionDef

```rust
pub struct OptionDef<'shape> {
    pub t: fn() -> &'shape Shape<'shape>,  // Inner type
    pub is_some: fn(PtrConst) -> bool,
    pub get: fn(PtrConst) -> Option<PtrConst>,
    pub get_mut: fn(PtrMut) -> Option<PtrMut>,
    pub init_some: fn(PtrUninit, PtrConst) -> PtrMut,
    pub init_none: fn(PtrUninit) -> PtrMut,
}
```

## Field System

```rust
pub struct Field<'shape> {
    pub name: &'shape str,
    pub shape: &'shape Shape<'shape>,
    pub offset: usize,
    pub flags: FieldFlags,
    pub attributes: &'shape [FieldAttribute<'shape>],
    pub doc: &'shape [&'shape str],
    pub vtable: &'shape FieldVTable,
    pub flattened: bool,
}

bitflags! {
    pub struct FieldFlags: u64 {
        const EMPTY = 0;
        const SENSITIVE = 1 << 0;
        const SKIP_SERIALIZING = 1 << 1;
        const FLATTEN = 1 << 2;
        const CHILD = 1 << 3;
        const DEFAULT = 1 << 4;
    }
}
```

## Enum Representation

```rust
pub struct EnumType<'shape> {
    pub repr: Repr,              // Memory layout (C, Rust, packed, etc.)
    pub enum_repr: EnumRepr,     // Discriminant type (u8, u16, etc.)
    pub variants: &'shape [Variant<'shape>],
}

pub struct Variant<'shape> {
    pub name: &'shape str,
    pub discriminant: Option<i64>,
    pub attributes: &'shape [VariantAttribute<'shape>],
    pub data: StructType<'shape>,  // Fields of the variant
    pub doc: &'shape [&'shape str],
}

pub enum EnumRepr {
    RustNPO,  // Null pointer optimization (like Option)
    U8, U16, U32, U64, USize,
    I8, I16, I32, I64, ISize,
}
```

## Implementations for Core Types

facet-core provides `Facet` implementations for:

### Core Types
- Primitives: `u8`, `u16`, `u32`, `u64`, `u128`, `usize`, `i8`, etc.
- `bool`, `char`, `str`, `()`
- `PhantomData<T>`
- `NonZero<T>`
- `Option<T>`
- `Result<T, E>`
- Tuples (up to 4, more with `tuples-12` feature)
- Arrays `[T; N]`
- Slices `[T]`
- Function pointers (with `fn-ptr` feature)

### Alloc Types
- `String`, `Box<T>`, `Vec<T>`, `Rc<T>`, `Arc<T>`
- `BTreeMap`, `BTreeSet`, `BinaryHeap`, `LinkedList`, `VecDeque`

### Std Types
- `HashMap`, `HashSet`
- `Path`, `PathBuf`
- `SocketAddr`, `IpAddr`

### Optional Feature Types
- `camino::Utf8Path`, `camino::Utf8PathBuf`
- `uuid::Uuid`
- `ulid::Ulid`
- `time::PrimitiveDateTime`, etc.
- `chrono::DateTime`, etc.
- `url::Url`
- `ordered_float::OrderedFloat`
- `bytes::Bytes`

## Shape Builder Pattern

```rust
impl<'shape> Shape<'shape> {
    pub const fn builder_for_sized<'a, T: Facet<'a>>() -> ShapeBuilder<'shape>;
    pub const fn builder_for_unsized<'a, T: Facet<'a> + ?Sized>() -> ShapeBuilder<'shape>;
}

impl<'shape> ShapeBuilder<'shape> {
    pub const fn new(vtable: &'shape ValueVTable) -> Self;
    pub const fn id(mut self, id: ConstTypeId) -> Self;
    pub const fn layout(mut self, layout: Layout) -> Self;
    pub const fn def(mut self, def: Def<'shape>) -> Self;
    pub const fn ty(mut self, ty: Type<'shape>) -> Self;
    pub const fn type_identifier(mut self, type_identifier: &'shape str) -> Self;
    // ... more builders
    pub const fn build(self) -> Shape<'shape>;
}
```

## Key Design Decisions

1. **Const Evaluation**: Shapes are const values, computed at compile time
2. **Type Erasure**: Uses type-erased pointers for generic operations
3. **VTable Dispatch**: Runtime polymorphism through function pointers
4. **Indirection for Recursion**: `fn() -> &'shape Shape` prevents infinite recursion in recursive types
5. **Specialization**: Uses "autoderef specialization" via the `spez` module for trait-based conditionals
