# Rust Revision: Reproducing Facet in Rust

## Overview

This guide explains how to reproduce Facet-like reflection functionality in Rust at a production level. It covers the core concepts, implementation patterns, and trade-offs involved in building a reflection system.

## What is Facet?

Facet is a reflection/introspection library that provides:
- **Compile-time shape generation** via derive macros
- **Runtime type inspection** via Shape metadata
- **Dynamic value manipulation** via type-erased pointers
- **Serialization/deserialization** without per-type code

## Core Concepts

### 1. The Shape Concept

A `Shape` is a runtime description of a type's structure:

```rust
pub struct Shape<'shape> {
    pub id: ConstTypeId,              // Unique type ID
    pub layout: ShapeLayout,          // Size + alignment
    pub vtable: &'shape ValueVTable,  // Operations
    pub ty: Type<'shape>,             // Classification
    pub def: Def<'shape>,             // Semantic meaning
    pub type_identifier: &'shape str, // Name
    // ... more metadata
}
```

### 2. The Facet Trait

```rust
pub unsafe trait Facet<'a>: 'a {
    const SHAPE: &'static Shape<'static>;
    const VTABLE: &'static ValueVTable;
}
```

**Why unsafe?** Incorrect implementations break memory safety. You must:
- Describe type layout correctly
- Provide accurate vtable functions
- Respect type invariants

### 3. Type Classification

```rust
pub enum Type<'shape> {
    Primitive(PrimitiveType),      // u32, bool, str, etc.
    Sequence(SequenceType<'shape>), // tuples, arrays, slices
    User(UserType<'shape>),         // structs, enums, unions
    Pointer(PointerType<'shape>),   // references, raw pointers, fn ptrs
}
```

### 4. Semantic Definition

```rust
pub enum Def<'shape> {
    Scalar(ScalarDef<'shape>),   // Atomic values
    List(ListDef<'shape>),       // Vec<T>, LinkedList<T>
    Map(MapDef<'shape>),         // HashMap<K, V>
    Struct(StructDef<'shape>),   // Named fields
    Enum(EnumDef<'shape>),       // Variants
    Option(OptionDef<'shape>),   // Option<T>
    // ... more
}
```

## Implementation Guide

### Step 1: Define Core Types

```rust
// shape.rs
use core::alloc::Layout;
use core::fmt;

/// Layout information for a shape
#[derive(Clone, Copy, Debug)]
pub enum ShapeLayout {
    Sized(Layout),
    Unsized,
}

/// Type classification
#[derive(Clone, Copy, Debug)]
pub enum Type<'shape> {
    Primitive(PrimitiveType),
    User(UserType<'shape>),
    // ...
}

#[derive(Clone, Copy, Debug)]
pub enum PrimitiveType {
    Integer(IntegerType),
    Float(FloatType),
    Boolean,
    Char,
    Str,
}

#[derive(Clone, Copy, Debug)]
pub enum UserType<'shape> {
    Struct(StructType<'shape>),
    Enum(EnumType<'shape>),
    Opaque,
}

/// Semantic definition
#[derive(Clone, Copy, Debug)]
pub enum Def<'shape> {
    Scalar(ScalarDef<'shape>),
    List(ListDef<'shape>),
    // ...
}
```

### Step 2: Define the VTable

```rust
// vtable.rs
use core::fmt;
use core::cmp::Ordering;

/// Type name formatting function
pub type TypeNameFn = fn(f: &mut fmt::Formatter, opts: TypeNameOpts) -> fmt::Result;

#[derive(Clone, Copy)]
pub struct TypeNameOpts {
    pub recurse_ttl: isize,  // -1 = infinite, 0 = none, n = n levels
}

/// VTable for sized types
#[derive(Clone, Copy)]
pub struct ValueVTableSized {
    pub type_name: TypeNameFn,
    pub drop_in_place: Option<DropInPlaceFn>,
    pub clone_into: Option<CloneIntoFn>,
    pub partial_eq: Option<PartialEqFn>,
    pub debug: Option<DebugFn>,
    // ... more operations
}

/// Drop in place function
pub type DropInPlaceFn = unsafe fn(PtrMut<'_>) -> PtrUninit<'_>;

/// Clone function
pub type CloneIntoFn = unsafe fn(PtrConst<'_>, PtrUninit<'_>) -> PtrMut<'_>;

/// PartialEq function
pub type PartialEqFn = unsafe fn(PtrConst<'_>, PtrConst<'_>) -> bool;

/// Debug formatting
pub type DebugFn = unsafe fn(PtrConst<'_>, f: &mut fmt::Formatter) -> fmt::Result;
```

### Step 3: Type-Erased Pointers

```rust
// ptr.rs
use core::ptr::NonNull;
use core::marker::PhantomData;

/// Immutable type-erased pointer
#[derive(Clone, Copy)]
pub struct PtrConst<'mem>(NonNull<u8>, PhantomData<&'mem ()>);

impl<'mem> PtrConst<'mem> {
    pub const fn new<T>(ptr: *const T) -> Self {
        unsafe { Self(NonNull::new_unchecked(ptr as *mut u8), PhantomData) }
    }

    pub const fn as_byte_ptr(self) -> *const u8 {
        self.0.as_ptr()
    }

    pub unsafe fn field(self, offset: usize) -> Self {
        Self(unsafe { NonNull::new_unchecked(self.0.as_ptr().add(offset)) }, PhantomData)
    }
}

/// Mutable type-erased pointer
#[derive(Clone, Copy)]
pub struct PtrMut<'mem>(NonNull<u8>, PhantomData<&'mem mut ()>);

impl<'mem> PtrMut<'mem> {
    pub fn as_byte_ptr(self) -> *mut u8 {
        self.0.as_ptr()
    }

    pub unsafe fn field(self, offset: usize) -> Self {
        Self(unsafe { NonNull::new_unchecked(self.0.as_ptr().add(offset)) }, PhantomData)
    }
}

/// Uninitialized pointer
#[derive(Clone, Copy)]
pub struct PtrUninit<'mem>(*mut u8, PhantomData<&'mem mut ()>);

impl<'mem> PtrUninit<'mem> {
    pub fn new<T>(ptr: *mut T) -> Self {
        Self(ptr as *mut u8, PhantomData)
    }

    pub unsafe fn put<T>(self, value: T) -> PtrMut<'mem> {
        unsafe {
            core::ptr::write(self.0 as *mut T, value);
            PtrMut(NonNull::new_unchecked(self.0 as *mut u8), PhantomData)
        }
    }

    pub fn as_mut_byte_ptr(self) -> *mut u8 {
        self.0
    }
}
```

### Step 4: The Facet Trait

```rust
// facet.rs
use crate::{Shape, ValueVTable};

pub unsafe trait Facet<'a>: 'a {
    const SHAPE: &'static Shape<'static>;
    const VTABLE: &'static ValueVTable;
}
```

### Step 5: Implement for Primitives

```rust
// impls.rs
use crate::{Facet, Shape, ShapeLayout, Type, Def, ScalarDef, ScalarAffinity, ValueVTable};
use core::alloc::Layout;
use core::fmt;

unsafe impl Facet<'_> for u32 {
    const SHAPE: &'static Shape<'static> = &Shape {
        id: TypeId::of::<u32>(),
        layout: ShapeLayout::Sized(Layout::new::<u32>()),
        vtable: &ValueVTable {
            type_name: |f, _opts| write!(f, "u32"),
            drop_in_place: None,  // Copy type
            clone_into: None,      // Copy type
            partial_eq: Some(|left, right| unsafe {
                let l = &*(left.as_byte_ptr() as *const u32);
                let r = &*(right.as_byte_ptr() as *const u32);
                l == r
            }),
            debug: Some(|ptr, f| unsafe {
                let v = &*(ptr.as_byte_ptr() as *const u32);
                write!(f, "{}", v)
            }),
        },
        ty: Type::Primitive(PrimitiveType::Integer(IntegerType::U32)),
        def: Def::Scalar(ScalarDef {
            affinity: &ScalarAffinity::number()
                .unsigned_integer(32)
                .min(PtrConst::new(&u32::MIN as *const _ as *const ()))
                .max(PtrConst::new(&u32::MAX as *const _ as *const ()))
                .build(),
        }),
        type_identifier: "u32",
        type_params: &[],
        doc: &[],
        attributes: &[],
        type_tag: Some("u32"),
        inner: None,
    };

    const VTABLE: &'static ValueVTable = &Self::SHAPE.vtable;
}
```

### Step 6: Derive Macro (Simplified)

```rust
// In facet-macros/src/lib.rs

use proc_macro::TokenStream;

#[proc_macro_derive(Facet, attributes(facet))]
pub fn derive_facet(input: TokenStream) -> TokenStream {
    let ast = parse(input);  // Using unsynn or syn
    expand_facet(ast)
}

fn expand_facet(ast: Ast) -> TokenStream {
    match ast {
        Ast::Struct(s) => expand_struct(s),
        Ast::Enum(e) => expand_enum(e),
    }
}

fn expand_struct(s: Struct) -> TokenStream {
    let name = &s.ident;
    let type_identifier = s.ident.to_string();

    // Generate fields array
    let fields = s.fields.iter().enumerate().map(|(i, f)| {
        let field_name = f.ident.as_ref().map(|id| id.to_string())
            .unwrap_or_else(|| i.to_string());
        let field_type = &f.ty;
        let offset = quote!(::core::mem::offset_of!(#name, #field_name));

        quote! {
            Field::builder()
                .name(#field_name)
                .shape(<#field_type as Facet>::SHAPE)
                .offset(#offset)
                .flags(FieldFlags::EMPTY)
                .doc(&[])
                .vtable(&FieldVTable { /* ... */ })
                .build()
        }
    });

    quote! {
        unsafe impl<'a> Facet<'a> for #name {
            const SHAPE: &'static Shape<'static> = &const {
                const FIELDS: &[Field] = &[#(#fields),*];

                Shape::builder_for_sized::<Self>()
                    .type_identifier(#type_identifier)
                    .ty(Type::User(UserType::Struct(StructType {
                        repr: Repr::Rust,
                        kind: StructKind::Struct,
                        fields: FIELDS,
                    })))
                    .build()
            };

            const VTABLE: &'static ValueVTable = &const {
                value_vtable!(#name, |f, _opts| write!(f, #type_identifier))
            };
        }
    }
}
```

### Step 7: Peek (Read Reflection)

```rust
// peek.rs
use crate::{Facet, Shape, GenericPtr, ValueVTable};

pub struct Peek<'mem, 'shape> {
    data: GenericPtr<'mem>,
    shape: &'shape Shape<'shape>,
}

impl<'mem, 'shape> Peek<'mem, 'shape> {
    pub fn new<T: Facet<'shape>>(value: &'mem T) -> Self {
        Self {
            data: GenericPtr::new(value),
            shape: T::SHAPE,
        }
    }

    pub fn vtable(&self) -> &'shape ValueVTable {
        self.shape.vtable
    }

    pub fn debug(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if let Some(debug_fn) = (self.vtable().debug)() {
            unsafe { debug_fn(self.data.as_ptr_const(), f) }
        } else {
            write!(f, "<no Debug impl>")
        }
    }

    pub fn as_scalar(&self) -> Option<Scalar<'mem>> {
        match self.shape.def {
            Def::Scalar(scalar_def) => {
                match scalar_def.affinity {
                    ScalarAffinity::Number(num) => {
                        // Extract based on number bits
                        match num.bits {
                            NumberBits::Integer { size, sign } => {
                                match (size.bits(), sign) {
                                    (32, Signedness::Unsigned) => {
                                        let v = unsafe {
                                            *(self.data.as_ptr_const().as_byte_ptr() as *const u32)
                                        };
                                        Some(Scalar::U32(v))
                                    }
                                    // ... handle other integer types
                                    _ => None,
                                }
                            }
                            // ... handle floats
                            _ => None,
                        }
                    }
                    ScalarAffinity::String(_) => {
                        let s = unsafe {
                            &*(self.data.as_ptr_const().as_byte_ptr() as *const String)
                        };
                        Some(Scalar::Str(s.as_str()))
                    }
                    // ... handle other scalar affinities
                    _ => None,
                }
            }
            _ => None,
        }
    }
}
```

### Step 8: Partial (Write Reflection)

```rust
// partial.rs
use crate::{Facet, Shape, PtrUninit, PtrMut};

pub struct Partial<'shape> {
    ptr: PtrUninit<'static>,
    shape: &'shape Shape<'shape>,
    state: ConstructionState,
}

enum ConstructionState {
    NotStarted,
    InProgress { fields_set: Vec<bool> },
    Complete,
}

impl<'shape> Partial<'shape> {
    pub fn alloc<T: Facet<'shape>>() -> Result<Self, ReflectError> {
        let shape = T::SHAPE;
        let layout = shape.layout.sized_layout()
            .map_err(|_| ReflectError::Unsized)?;

        // Allocate memory
        let ptr = if layout.size() == 0 {
            PtrUninit::new(layout.align() as *mut u8)
        } else {
            unsafe {
                let raw = alloc::alloc::alloc(layout);
                PtrUninit::new(raw)
            }
        };

        Ok(Self {
            ptr,
            shape,
            state: ConstructionState::NotStarted,
        })
    }

    pub fn set_field(&mut self, name: &str, value: impl Facet<'shape>) -> Result<&mut Self, ReflectError> {
        // Find field by name
        let field_index = self.find_field_by_name(name)?;
        let field = self.get_field(field_index)?;

        // Check type compatibility
        if field.shape != <impl Facet>::SHAPE {
            return Err(ReflectError::TypeMismatch {
                expected: field.shape,
                got: <impl Facet>::SHAPE,
            });
        }

        // Get field pointer
        let field_ptr = unsafe { self.ptr.field(field.offset) };

        // Write value
        unsafe {
            field_ptr.put(value);
        }

        // Mark field as set
        self.mark_field_set(field_index)?;

        Ok(self)
    }

    pub fn build<T: Facet<'shape>>(mut self) -> Result<T, ReflectError> {
        // Check all required fields are set
        self.check_required_fields()?;

        // Get the pointer back
        let ptr = self.ptr;

        // Prevent drop of Partial (we're taking ownership of the memory)
        core::mem::forget(self);

        // Read the value (for Copy types) or return pointer
        // This depends on your design
        unsafe {
            Ok(core::ptr::read(ptr.as_byte_ptr() as *const T))
        }
    }
}
```

### Step 9: Serialization Framework

```rust
// serialize.rs
use crate::{Peek, Facet, Def, ScalarAffinity};

pub trait Serializer {
    type Error;

    fn serialize_u64(&mut self, v: u64) -> Result<(), Self::Error>;
    fn serialize_i64(&mut self, v: i64) -> Result<(), Self::Error>;
    fn serialize_str(&mut self, v: &str) -> Result<(), Self::Error>;
    fn serialize_bool(&mut self, v: bool) -> Result<(), Self::Error>;

    fn start_object(&mut self, len: Option<usize>) -> Result<(), Self::Error>;
    fn end_object(&mut self) -> Result<(), Self::Error>;
    fn start_array(&mut self, len: Option<usize>) -> Result<(), Self::Error>;
    fn end_array(&mut self) -> Result<(), Self::Error>;
    fn field_name(&mut self, name: &str) -> Result<(), Self::Error>;
}

pub fn serialize<T: Facet<'static>, S: Serializer>(
    value: &T,
    serializer: &mut S,
) -> Result<(), S::Error> {
    let peek = Peek::new(value);
    serialize_peek(&peek, serializer)
}

fn serialize_peek<'shape, S: Serializer>(
    peek: &Peek<'_, 'shape>,
    serializer: &mut S,
) -> Result<(), S::Error> {
    match peek.shape().def {
        Def::Scalar(scalar_def) => {
            match scalar_def.affinity {
                ScalarAffinity::Number(num) => {
                    // Extract and serialize number
                    if let Some(Scalar::U64(v)) = peek.as_scalar().and_then(|s| s.as_u64()) {
                        serializer.serialize_u64(v)
                    } else if let Some(Scalar::I64(v)) = peek.as_scalar().and_then(|s| s.as_i64()) {
                        serializer.serialize_i64(v)
                    } else {
                        // Handle other number types
                        Err(S::Error::custom("unsupported number type"))
                    }
                }
                ScalarAffinity::String(_) => {
                    if let Some(Scalar::Str(s)) = peek.as_scalar().and_then(|s| s.as_str()) {
                        serializer.serialize_str(s)
                    } else {
                        Err(S::Error::custom("expected string"))
                    }
                }
                ScalarAffinity::Boolean(_) => {
                    if let Some(Scalar::Bool(b)) = peek.as_scalar().and_then(|s| s.as_bool()) {
                        serializer.serialize_bool(b)
                    } else {
                        Err(S::Error::custom("expected bool"))
                    }
                }
                _ => Err(S::Error::custom("unsupported scalar")),
            }
        }

        Def::Struct(struct_def) => {
            let peek_struct = peek.into_struct().unwrap();
            serializer.start_object(Some(peek_struct.field_count()))?;

            for field_result in peek_struct.fields() {
                let (name, field_peek) = field_result;
                serializer.field_name(name)?;
                serialize_peek(&field_peek, serializer)?;
            }

            serializer.end_object()
        }

        // ... handle other Def variants
    }
}
```

## Production-Level Considerations

### 1. Memory Safety

```rust
// Always validate pointers before dereferencing
pub unsafe fn get_field_ptr<'mem>(
    base: PtrConst<'mem>,
    field_offset: usize,
    field_layout: Layout,
) -> Result<PtrConst<'mem>, ReflectError> {
    // Validate offset is within bounds
    // Validate alignment
    // Return safe pointer
}
```

### 2. Const Evaluation

Use `const {}` blocks for compile-time computation:

```rust
const SHAPE: &'static Shape<'static> = &const {
    // Everything here is computed at compile time
    Shape::builder_for_sized::<Self>()
        .type_identifier("MyType")
        // ...
        .build()
};
```

### 3. no_std Support

```rust
#![cfg_attr(not(feature = "std"), no_std)]

extern crate alloc;

use alloc::vec::Vec;
use alloc::string::String;

// Use core::* instead of std::*
```

### 4. Error Handling

```rust
#[derive(Debug, Clone, PartialEq)]
pub enum ReflectError {
    NoSuchField { type_name: String, field: String },
    TypeMismatch { expected: String, got: String },
    RequiredFieldNotSet(String),
    InvariantViolation(String),
    UnsizedType,
}

impl core::fmt::Display for ReflectError {
    fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
        match self {
            ReflectError::NoSuchField { type_name, field } => {
                write!(f, "No field '{}' on type '{}'", field, type_name)
            }
            // ... other variants
        }
    }
}
```

### 5. Performance Optimization

```rust
// Cache shape lookups
static CACHED_SHAPE: OnceLock<&Shape> = OnceLock::new();

fn get_shape() -> &'static Shape {
    *CACHED_SHAPE.get_or_init(|| compute_shape())
}

// Use inline hints for hot paths
#[inline(always)]
fn hot_path(ptr: PtrConst) -> Result<PtrConst, ReflectError> {
    // ...
}
```

## Trade-offs

### Advantages of Facet-like Design

1. **Compile-time computation**: Most metadata is const-evaluated
2. **Type safety**: Compile-time checked where possible
3. **Zero-cost abstraction**: No runtime overhead for metadata access
4. **Flexibility**: Dynamic type inspection at runtime

### Disadvantages

1. **Binary size**: Shape metadata increases binary size
2. **Compile time**: Derive macros add to compilation time
3. **Complexity**: More complex than simple trait bounds
4. **Runtime cost**: Type-erased operations are slower than monomorphization

### Comparison with Alternatives

| Approach | Pros | Cons |
|----------|------|------|
| **Facet (reflection)** | Dynamic, ergonomic, const-friendly | Runtime cost, binary size |
| **Serde (monomorphized)** | Fast, zero-abstraction | Per-type codegen, compile time |
| **bevy-reflect** | ECS-integrated, runtime registration | More complex, heap allocations |
| **typetag (trait objects)** | Simple, works with trait objects | Limited introspection |

## Example: Complete Implementation

Here's a complete minimal implementation:

```rust
// lib.rs
#![cfg_attr(not(feature = "std"), no_std)]
extern crate alloc;

mod shape;
mod vtable;
mod ptr;
mod facet;
mod peek;

pub use shape::*;
pub use vtable::*;
pub use ptr::*;
pub use facet::*;
pub use peek::*;

// Example usage
#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Facet)]
    struct Point {
        x: f64,
        y: f64,
    }

    #[test]
    fn test_point_reflection() {
        let p = Point { x: 1.0, y: 2.0 };
        let peek = Peek::new(&p);

        assert_eq!(peek.shape().type_identifier, "Point");

        let peek_struct = peek.into_struct().unwrap();
        assert_eq!(peek_struct.field_count(), 2);

        let x_field = peek_struct.field_by_name("x").unwrap();
        assert_eq!(x_field.shape().type_identifier, "f64");
    }
}
```

## Conclusion

Building a Facet-like reflection system in Rust requires:

1. **Unsafe trait** for shape registration (with strict safety requirements)
2. **Type-erased pointers** for generic value manipulation
3. **VTable-based dispatch** for runtime operations
4. **Const evaluation** for compile-time metadata generation
5. **Derive macros** for automatic shape generation

The key insight is separating **type metadata** (Shape) from **value operations** (VTable) and using type-erased pointers to bridge them at runtime.

For production use, consider:
- Starting with a minimal subset (primitives + structs)
- Adding complexity incrementally (enums, generics, etc.)
- Testing extensively with Miri for memory safety
- Benchmarking against alternatives
