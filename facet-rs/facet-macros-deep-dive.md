# facet-macros Deep Dive

## Overview

`facet-macros` is the procedural macro crate that automatically generates `Facet` implementations for user types. It uses `unsynn` (not `syn`) for fast, lightweight parsing.

## Architecture

```
facet-macros/
├── src/lib.rs              # Main entry point
├── parse/                  # Parsing logic (or in facet-macros-parse)
│   ├── mod.rs
│   ├── struct.rs
│   └── enum.rs
└── emit/                   # Code generation (or in facet-macros-emit)
    ├── mod.rs
    ├── struct.rs
    └── enum.rs
```

## Main Entry Point

```rust
use proc_macro::TokenStream;

#[proc_macro_derive(Facet, attributes(facet))]
pub fn derive_facet(input: TokenStream) -> TokenStream {
    // Parse input tokens
    let ast = parse::parse(input);

    // Generate Facet implementation
    emit::emit_facet(ast)
}
```

## Parsing with unsynn

unsynn is a lightweight alternative to syn. It's faster but supports less Rust syntax.

```rust
// parse/mod.rs
use unsynn::{TokenStream, Parse, TokenTree, Ident, punctuated::Punctuated};

pub fn parse(input: TokenStream) -> Ast {
    let mut tokens = input.into_iter().peekable();

    // Parse attributes
    let attrs = parse_attributes(&mut tokens);

    // Parse visibility
    let vis = parse_visibility(&mut tokens);

    // Parse type keyword (struct, enum, union)
    let type_keyword = expect_ident(&mut tokens, &["struct", "enum", "union"]);

    match type_keyword.as_str() {
        "struct" => Ast::Struct(parse_struct(tokens)),
        "enum" => Ast::Enum(parse_enum(tokens)),
        "union" => Ast::Union(parse_union(tokens)),
        _ => panic!("Unknown type keyword"),
    }
}

pub struct Struct {
    pub ident: Ident,
    pub generics: Generics,
    pub fields: Fields,
    pub attrs: Vec<Attribute>,
}

pub enum Fields {
    Named(Punctuated<Field, Token![,]>),
    Unnamed(Parenthesized<Punctuated<Field, Token![,]>>),
    Unit,
}

pub struct Field {
    pub ident: Option<Ident>,  // None for tuple struct fields
    pub ty: Type,
    pub attrs: Vec<Attribute>,
}
```

## Attribute Parsing

```rust
// parse/attributes.rs

pub struct FacetAttrs {
    pub rename_all: Option<RenameRule>,
    pub transparent: bool,
    pub deny_unknown_fields: bool,
    pub skip_serializing: bool,
    pub invariants: Option<Expr>,
}

pub struct FieldAttrs {
    pub rename: Option<String>,
    pub default: Option<DefaultKind>,
    pub sensitive: bool,
    pub flatten: bool,
    pub child: bool,
    pub skip_serializing: bool,
    pub skip_serializing_if: Option<Expr>,
}

fn parse_facet_attr(attr: &Attribute) -> Result<FacetAttrs> {
    // Parse #[facet(rename_all = "kebab-case")]
    // Parse #[facet(transparent)]
    // etc.
}
```

### Rename Rules

```rust
#[derive(Clone, Copy, Debug)]
pub enum RenameRule {
    LowerCase,       // snake_case
    UpperCase,       // SCREAMING_SNAKE_CASE
    PascalCase,
    CamelCase,
    KebabCase,       // kebab-case
    ScreamingKebab,  // SCREAMING-KEBAB-CASE
}

impl RenameRule {
    pub fn apply(&self, name: &str) -> String {
        match self {
            RenameRule::LowerCase => to_snake_case(name),
            RenameRule::UpperCase => to_screaming_snake_case(name),
            RenameRule::PascalCase => to_pascal_case(name),
            RenameRule::CamelCase => to_camel_case(name),
            RenameRule::KebabCase => to_kebab_case(name),
            RenameRule::ScreamingKebab => to_screaming_kebab_case(name),
        }
    }
}
```

## Code Emission

### Struct Emission

```rust
// emit/struct.rs

pub fn emit_struct(s: &Struct) -> TokenStream {
    let name = &s.ident;
    let type_identifier = s.ident.to_string();

    // Generate field definitions
    let fields = emit_fields(&s.fields);

    // Generate shape const
    let shape = quote! {
        const SHAPE: &'static Shape<'static> = &const {
            const FIELDS: &[Field] = &[#(#fields),*];

            Shape::builder_for_sized::<Self>()
                .type_identifier(#type_identifier)
                .ty(Type::User(UserType::Struct(StructType {
                    repr: Repr::Rust,
                    kind: get_struct_kind(&s.fields),
                    fields: FIELDS,
                })))
                .build()
        };
    };

    // Generate vtable const
    let vtable = quote! {
        const VTABLE: &'static ValueVTable = &const {
            value_vtable!(#name, |f, _opts| write!(f, #type_identifier))
        };
    };

    // Generate impl block
    quote! {
        unsafe impl<'a> Facet<'a> for #name {
            #shape
            #vtable
        }
    }
}
```

### Field Emission

```rust
// emit/field.rs

pub fn emit_field(f: &Field, index: usize) -> TokenStream {
    let name = f.ident.as_ref()
        .map(|id| id.to_string())
        .unwrap_or_else(|| index.to_string());

    let field_ident = &f.ident;
    let ty = &f.ty;
    let offset = if f.ident.is_some() {
        quote!(::core::mem::offset_of!(Self, #field_ident))
    } else {
        // Tuple struct - use tuple_field_offset (unstable, so we compute manually)
        quote!(compute_tuple_field_offset::<#ty>(#index))
    };

    let flags = emit_field_flags(&f.attrs);
    let doc = emit_doc_comments(&f.attrs);
    let vtable = emit_field_vtable(&f.attrs);

    quote! {
        Field::builder()
            .name(#name)
            .shape(<#ty as Facet>::SHAPE)
            .offset(#offset)
            .flags(#flags)
            .doc(#doc)
            .vtable(&#vtable)
            .build()
    }
}

fn emit_field_flags(attrs: &[Attribute]) -> TokenStream {
    let mut flags = quote!(FieldFlags::EMPTY);

    for attr in attrs {
        match attr {
            Attribute::Sensitive => {
                flags = quote!(#flags.union(FieldFlags::SENSITIVE));
            }
            Attribute::SkipSerializing => {
                flags = quote!(#flags.union(FieldFlags::SKIP_SERIALIZING));
            }
            Attribute::Flatten => {
                flags = quote!(#flags.union(FieldFlags::FLATTEN));
            }
            Attribute::Child => {
                flags = quote!(#flags.union(FieldFlags::CHILD));
            }
            Attribute::Default => {
                flags = quote!(#flags.union(FieldFlags::DEFAULT));
            }
            _ => {}
        }
    }

    flags
}
```

### Enum Emission

```rust
// emit/enum.rs

pub fn emit_enum(e: &Enum) -> TokenStream {
    let name = &e.ident;
    let type_identifier = e.ident.to_string();

    // Determine enum repr
    let enum_repr = determine_enum_repr(e);

    // Generate variant definitions
    let variants = e.variants.iter().map(|v| {
        emit_variant(v, &enum_repr)
    });

    // Generate shape
    let shape = quote! {
        const SHAPE: &'static Shape<'static> = &const {
            const VARIANTS: &[Variant] = &[#(#variants),*];

            Shape::builder_for_sized::<Self>()
                .type_identifier(#type_identifier)
                .ty(Type::User(UserType::Enum(EnumType {
                    repr: Repr::Rust,
                    enum_repr: #enum_repr,
                    variants: VARIANTS,
                })))
                .build()
        };
    };

    quote! {
        unsafe impl<'a> Facet<'a> for #name {
            #shape
            const VTABLE: &'static ValueVTable = &const {
                value_vtable!(#name, |f, _opts| write!(f, #type_identifier))
            };
        }
    }
}

pub fn emit_variant(v: &Variant, enum_repr: &EnumRepr) -> TokenStream {
    let name = v.ident.to_string();
    let discriminant = v.discriminant.as_ref()
        .map(|d| quote!(Some(#d)))
        .unwrap_or(quote!(None));

    let data = emit_struct_type(&v.fields);

    quote! {
        Variant::builder()
            .name(#name)
            .discriminant(#discriminant)
            .data(#data)
            .doc(&[])
            .build()
    }
}
```

### Determining Enum Repr

```rust
fn determine_enum_repr(e: &Enum) -> EnumRepr {
    // Check for explicit #[repr(u8)], #[repr(i16)], etc.
    for attr in &e.attrs {
        if let Attribute::Repr(repr) = attr {
            return match repr {
                Repr::U8 => EnumRepr::U8,
                Repr::U16 => EnumRepr::U16,
                Repr::U32 => EnumRepr::U32,
                Repr::U64 => EnumRepr::U64,
                Repr::USize => EnumRepr::USize,
                Repr::I8 => EnumRepr::I8,
                Repr::I16 => EnumRepr::I16,
                Repr::I32 => EnumRepr::I32,
                Repr::I64 => EnumRepr::I64,
                Repr::ISize => EnumRepr::ISize,
                Repr::Rust => {
                    // Check for Option-like pattern for NPO
                    if is_option_like(e) {
                        EnumRepr::RustNPO
                    } else {
                        // Default based on variant count
                        default_repr_for_variants(e.variants.len())
                    }
                }
            };
        }
    }

    // Default repr
    if is_option_like(e) {
        EnumRepr::RustNPO
    } else {
        default_repr_for_variants(e.variants.len())
    }
}

fn is_option_like(e: &Enum) -> bool {
    // Two variants: one unit, one with single field
    // Or one variant that is None-like
    e.variants.len() == 2
        && e.variants.iter().any(|v| v.fields.is_empty())
        && e.variants.iter().any(|v| v.fields.len() == 1)
}
```

## Handling Generics

```rust
// emit/generics.rs

pub fn emit_generics(generics: &Generics) -> TokenStream {
    let params = generics.params.iter().map(|p| {
        match p {
            GenericParam::Type(t) => {
                let name = &t.ident;
                let bounds = &t.bounds;
                quote!(#name: #bounds)
            }
            GenericParam::Lifetime(l) => {
                let name = &l.ident;
                quote!(#name)
            }
            GenericParam::Const(c) => {
                let name = &c.ident;
                let ty = &c.ty;
                quote!(const #name: #ty)
            }
        }
    });

    quote!(<#(#params),*>)
}

pub fn emit_type_params(generics: &Generics) -> TokenStream {
    let params = generics.params.iter().filter_map(|p| {
        match p {
            GenericParam::Type(t) => {
                let name = t.ident.to_string();
                let ident = &t.ident;
                Some(quote! {
                    TypeParam {
                        name: #name,
                        shape: #ident::SHAPE,
                    }
                })
            }
            _ => None,
        }
    });

    quote!(&[#(#params),*])
}
```

## Handling Special Attributes

### Transparent

```rust
// For #[facet(transparent)]
if attrs.transparent {
    // Don't generate struct-like shape
    // Instead, wrap the inner type's shape
    return quote! {
        const SHAPE: &'static Shape<'static> = &const {
            Shape::builder_for_sized::<Self>()
                .type_identifier(#type_identifier)
                .inner(|| <#inner_ty as Facet>::SHAPE)
                .ty(Type::User(UserType::Opaque))
                .build()
        };
    };
}
```

### Deny Unknown Fields

```rust
// For #[facet(deny_unknown_fields)]
let attributes = if attrs.deny_unknown_fields {
    quote!(&[ShapeAttribute::DenyUnknownFields])
} else {
    quote!(&[])
};
```

### Invariants

```rust
// For #[facet(invariants = "my_check")]
if let Some(invariant_fn) = attrs.invariants {
    quote! {
        .invariants(|| Some(|ptr| {
            #invariant_fn(unsafe { &*ptr.as_byte_ptr() as *const Self })
        }))
    }
}
```

## Error Handling

```rust
use proc_macro::Diagnostic;

fn emit_facet_or_error(input: TokenStream) -> TokenStream {
    match emit_facet_impl(input) {
        Ok(tokens) => tokens,
        Err(e) => {
            e.emit();
            TokenStream::new()
        }
    }
}

#[derive(Debug)]
pub struct Error {
    span: Span,
    message: String,
}

impl Error {
    pub fn emit(self) {
        // Use proc_macro diagnostic API
        // or panic with span info
    }
}
```

## Testing the Macro

```rust
// tests/facet_derive.rs

use facet::Facet;

#[test]
fn test_simple_struct() {
    #[derive(Facet)]
    struct Point {
        x: i32,
        y: i32,
    }

    assert_eq!(Point::SHAPE.type_identifier, "Point");
    assert_eq!(Point::SHAPE.def, /* expected */);
}

#[test]
fn test_tuple_struct() {
    #[derive(Facet)]
    struct Wrapper(i32, String);

    // Test tuple field access
}

#[test]
fn test_enum() {
    #[derive(Facet)]
    #[repr(u8)]
    enum Message {
        Quit,
        Move { x: i32, y: i32 },
        Write(String),
    }

    // Test variant detection
}

#[test]
fn test_rename_all() {
    #[derive(Facet)]
    #[facet(rename_all = "kebab-case")]
    struct MyStruct {
        my_field: String,
    }

    // Field should be renamed
    let field = MyStruct::SHAPE.ty.as_struct().unwrap().fields[0];
    assert_eq!(field.name, "my-field");
}
```

## Performance Considerations

1. **Use unsynn**: Faster than syn, though less feature-complete
2. **Const evaluation**: Generate `const {}` blocks where possible
3. **Minimize allocations**: Use `&'static str` and static arrays
4. **Quote caching**: Cache repeated token streams

## Known Limitations

1. **unsynn limitations**: Doesn't support all Rust syntax
2. **Complex generics**: Some generic scenarios may not work
3. **Macro hygiene**: Some identifiers may need `::core::` prefix
4. **Compile errors**: Error messages could be improved

## Future Improvements

1. Better error messages with span information
2. Support for more Rust syntax patterns
3. Option to use syn instead of unsynn
4. Incremental compilation improvements
