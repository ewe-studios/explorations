---
title: "Macro Codegen Deep Dive: Procedural Macros and Build-Time Code Generation"
subtitle: "Understanding how Pavex uses procedural macros for compile-time metaprogramming"
based_on: "pavex_macros crate - libs/pavex_macros/"
level: "Intermediate - Requires Rust fundamentals"
---

# Macro Codegen Deep Dive

## Overview

This deep dive explores how Pavex uses **procedural macros** to enable compile-time code generation. We'll examine the `pavex_macros` crate, understand how macros transform input tokens into generated code, and learn patterns you can replicate.

---

## 1. Procedural Macro Fundamentals

### 1.1 What Are Procedural Macros?

Procedural macros are functions that run **at compile time**, transforming token streams.

```
┌─────────────────────────────────────────────────────────┐
│              Procedural Macro Execution                  │
│                                                          │
│  Source Code         Macro               Generated Code │
│  ┌────────┐         ┌────────┐         ┌────────┐       │
│  │        │         │        │         │        │       │
│  │#[macro]│  --->   │macro   │  --->   │Expanded│       │
│  │fn foo(){│        │expands │         │code    │       │
│  │}       │         │        │         │        │       │
│  └────────┘         └────────┘         └────────┘       │
│                                                          │
│  (Parsed to        (Runs during        (Inserted back   │
│   TokenStream)     compilation)        into source)     │
└─────────────────────────────────────────────────────────┘
```

### 1.2 Types of Procedural Macros

| Type | Signature | Use Case |
|------|-----------|----------|
| **Function-like** | `fn(TokenStream) -> TokenStream` | `my_macro!(input)` |
| **Attribute** | `fn(TokenStream, TokenStream) -> TokenStream` | `#[my_attr]` |
| **Derive** | `fn(TokenStream) -> TokenStream` | `#[derive(MyTrait)]` |

### 1.3 The TokenStream Type

`TokenStream` is the fundamental unit:

```rust
use proc_macro::TokenStream;

// Tokens are what the Rust parser produces
// Example: `fn foo(x: i32) -> i32 { x + 1 }`
// Becomes tokens: [fn] [foo] [(] [x] [:] [i32] [)] [->] [i32] [{] [x] [+] [1] [}]
```

**Key crates for working with tokens:**

| Crate | Purpose |
|-------|---------|
| `proc_macro` | Standard library types (TokenStream) |
| `proc_macro2` | Stable wrapper (better Span handling) |
| `syn` | Parse TokenStream into AST |
| `quote` | Generate TokenStream from Rust-like syntax |

---

## 2. The pavex_macros Crate

### 2.1 Crate Structure

```
pavex_macros/
├── src/
│   ├── lib.rs                    # Macro exports
│   ├── constructor.rs            # #[constructor], #[singleton], etc.
│   ├── path_params.rs            # #[PathParams]
│   └── config_profile.rs         # ConfigProfile derive macro
├── Cargo.toml
└── tests/
```

### 2.2 Macro Catalog

| Macro | Type | Purpose |
|-------|------|---------|
| `#[PathParams(...)]` | Attribute | Parse route parameters from type |
| `#[constructor(...)]` | Attribute | Register as DI constructor |
| `#[singleton(...)]` | Attribute | Shorthand for Singleton constructor |
| `#[transient(...)]` | Attribute | Shorthand for Transient constructor |
| `#[request_scoped(...)]` | Attribute | Shorthand for RequestScoped constructor |
| `#[derive(ConfigProfile)]` | Derive | Configuration struct validation |
| `f!()` | Function-like | IDE-safe path reference |

---

## 3. The f! Macro

### 3.1 Purpose

The `f!` macro provides **IDE-safe path references**:

```rust
// Without f! - path is just a string
bp.constructor("crate::http_client", Lifecycle::Singleton);
// Problem: No go-to-definition, no rename refactoring

// With f! - path is validated
bp.constructor(f!(crate::http_client), Lifecycle::Singleton);
// Benefit: IDE can navigate to definition
```

### 3.2 Implementation

```rust
// libs/pavex/src/lib.rs (simplified)
#[macro_export]
macro_rules! f {
    ($path:path) => {{
        // IDE hint: force path resolution when cfg is set
        #[cfg(pavex_ide_hint)]
        {
            // This code never runs, but IDE sees the path
            // and provides completions/go-to-definition
            let _ = $path;
        }

        // Runtime: just stringify the path
        stringify!($path)
    }};
}
```

### 3.3 How It Works

**Step 1: Parse the path**

```rust
// Input: f!(crate::http_client)
// $path = crate::http_client (parsed as `path` fragment)
```

**Step 2: IDE hint (conditional)**

```rust
// When pavex_ide_hint cfg is set (via rust-analyzer settings):
let _ = crate::http_client;  // Forces IDE to resolve

// When compiling normally:
// (this block is elided entirely)
```

**Step 3: Stringify**

```rust
// stringify! converts tokens to string literal
stringify!(crate::http_client)  // "crate :: http_client"
```

**Output:**

```rust
// The macro expands to:
{
    #[cfg(pavex_ide_hint)]
    {
        let _ = crate::http_client;
    }
    "crate :: http_client"
}
```

### 3.4 Why Not Just Use Strings?

| Approach | Go-to-Definition | Rename Refactor | Compile Validation |
|----------|-----------------|-----------------|-------------------|
| `"crate::func"` | No | No | No |
| `f!(crate::func)` | Yes (with cfg) | Yes | Partial |
| `crate::func` (direct) | Yes | Yes | Yes (but can't pass as string) |

---

## 4. Constructor Attribute Macros

### 4.1 Usage

```rust
use pavex::blueprint::constructor::Lifecycle;

#[pavex::constructor(Lifecycle::Singleton)]
pub fn http_client() -> HttpClient {
    HttpClient::new()
}

// Shorthand variants:
#[pavex::singleton]
pub fn logger() -> Logger { Logger::new() }

#[pavex::transient]
pub fn config() -> Config { Config::load() }

#[pavex::request_scoped]
pub fn request_id() -> RequestId { RequestId::generate() }
```

### 4.2 Macro Implementation

```rust
// libs/pavex_macros/src/constructor.rs (simplified)
use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, ItemFn};

pub fn singleton(_metadata: TokenStream, input: TokenStream) -> TokenStream {
    constructor_impl("Singleton", input)
}

pub fn transient(_metadata: TokenStream, input: TokenStream) -> TokenStream {
    constructor_impl("Transient", input)
}

pub fn request_scoped(_metadata: TokenStream, input: TokenStream) -> TokenStream {
    constructor_impl("RequestScoped", input)
}

fn constructor_impl(lifecycle: &str, input: TokenStream) -> TokenStream {
    // Parse the input function
    let item_fn = parse_macro_input!(input as ItemFn);

    // Extract function details
    let fn_name = &item_fn.sig.ident;
    let fn_output = &item_fn.sig.output;
    let fn_body = &item_fn.block;
    let fn_inputs = &item_fn.sig.inputs;

    // Generate code
    let lifecycle_ident = syn::Ident::new(lifecycle, proc_macro2::Span::call_site());

    let expanded = quote! {
        // Original function (unchanged)
        #item_fn

        // Registration helper (conceptually)
        const _: () = {
            pavex::blueprint::register_constructor(
                stringify!(#fn_name),
                pavex::blueprint::constructor::Lifecycle::#lifecycle_ident,
            );
        };
    };

    TokenStream::from(expanded)
}
```

### 4.3 What Gets Generated

For this input:

```rust
#[pavex::singleton]
pub fn http_client() -> HttpClient {
    HttpClient::new()
}
```

The macro generates (conceptually):

```rust
// Original function preserved
pub fn http_client() -> HttpClient {
    HttpClient::new()
}

// Hidden registration marker
const _: () = {
    // This registers the constructor with the blueprint
    pavex::blueprint::internal::mark_constructor(
        "http_client",
        Lifecycle::Singleton,
        // Type information extracted from signature
    );
};
```

---

## 5. PathParams Attribute

### 5.1 Usage

```rust
use pavex::PathParams;

#[derive(PathParams)]
pub struct UserPath {
    user_id: Uuid,
}

// Used in routes:
// GET /users/{user_id}
// Extracts user_id from path and parses as Uuid
```

### 5.2 Implementation

```rust
// libs/pavex_macros/src/path_params.rs (simplified)
use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, Data, DeriveInput, Fields};

pub fn path_params(input: TokenStream) -> TokenStream {
    let item = parse_macro_input!(input as DeriveInput);
    let name = &item.ident;

    // Extract fields from struct
    let fields = match &item.data {
        Data::Struct(data) => match &data.fields {
            Fields::Named(fields) => &fields.named,
            _ => panic!("PathParams only supports named fields"),
        },
        _ => panic!("PathParams only supports structs"),
    };

    // Generate field extractors
    let field_extractions = fields.iter().map(|field| {
        let field_name = &field.ident;
        let field_type = &field.ty;

        quote! {
            let #field_name: #field_type = extract_path_param(params, stringify!(#field_name))?;
        }
    });

    let field_names = fields.iter().map(|f| &f.ident);

    let expanded = quote! {
        impl pavex::request::FromPathParams for #name {
            type Error = pavex::request::PathParamsError;

            fn from_path_params(
                params: &pavex::request::PathParamsMap,
            ) -> Result<Self, Self::Error> {
                #( #field_extractions )*
                Ok(Self {
                    #( #field_names ),*
                })
            }
        }
    };

    TokenStream::from(expanded)
}
```

### 5.3 Generated Code

For this input:

```rust
#[derive(PathParams)]
pub struct UserPath {
    user_id: Uuid,
    post_id: u32,
}
```

Generates:

```rust
impl pavex::request::FromPathParams for UserPath {
    type Error = pavex::request::PathParamsError;

    fn from_path_params(
        params: &pavex::request::PathParamsMap,
    ) -> Result<Self, Self::Error> {
        let user_id: Uuid = extract_path_param(params, "user_id")?;
        let post_id: u32 = extract_path_param(params, "post_id")?;
        Ok(Self { user_id, post_id })
    }
}
```

---

## 6. ConfigProfile Derive Macro

### 6.1 Usage

```rust
use pavex::ConfigProfile;
use serde::Deserialize;

#[derive(Deserialize, ConfigProfile)]
pub struct ServerConfig {
    #[pavex(env = "SERVER_PORT", default = "8080")]
    pub port: u16,

    #[pavex(env = "SERVER_HOST")]
    pub host: String,
}
```

### 6.2 Implementation Pattern

```rust
// libs/pavex_macros/src/config_profile.rs (simplified)
use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, DeriveInput, Data, Fields};

pub fn derive_config_profile(input: TokenStream) -> TokenStream {
    let item = parse_macro_input!(input as DeriveInput);
    let name = &item.ident;

    let fields = match &item.data {
        Data::Struct(data) => match &data.fields {
            Fields::Named(fields) => &fields.named,
            _ => panic!("ConfigProfile requires named fields"),
        },
        _ => panic!("ConfigProfile only supports structs"),
    };

    // Generate validation logic
    let validations = fields.iter().map(|field| {
        let field_name = &field.ident;

        // Parse pavex attributes
        let env_var = get_env_attr(field);
        let default = get_default_attr(field);

        quote! {
            // Validate field presence
            if self.#field_name.is_none() {
                if let Some(default) = #default {
                    self.#field_name = Some(default);
                } else {
                    return Err(ConfigError::MissingField(
                        stringify!(#field_name),
                        #env_var,
                    ));
                }
            }
        }
    });

    let expanded = quote! {
        impl pavex::config::ConfigProfile for #name {
            fn validate(&self) -> Result<(), pavex::config::ConfigError> {
                #( #validations )*
                Ok(())
            }
        }
    };

    TokenStream::from(expanded)
}
```

---

## 7. syn and quote Deep Dive

### 7.1 Parsing with syn

**Parse different item types:**

```rust
use syn::{parse_macro_input, ItemFn, ItemStruct, ItemImpl, AttributeArgs};

// For attribute macros on functions
let item_fn = parse_macro_input!(input as ItemFn);

// For attribute macros on structs
let item_struct = parse_macro_input!(input as ItemStruct);

// For derive macros
let derive_input = parse_macro_input!(input as DeriveInput);

// For function-like macros with arguments
let args = parse_macro_input!(input as AttributeArgs);
```

**ItemFn structure:**

```rust
pub struct ItemFn {
    pub attrs: Vec<Attribute>,
    pub vis: Visibility,
    pub sig: FnSig,
    pub block: Block,
}

pub struct FnSig {
    pub constness: Option<Token![const]>,
    pub asyncness: Option<Token![async]>,
    pub unsafety: Option<Token![unsafe]>,
    pub abi: Option<Abi>,
    pub fn_token: Token![fn],
    pub ident: Ident,
    pub generics: Generics,
    pub paren_token: token::Paren,
    pub inputs: Punctuated<FnArg, Token![,]>,
    pub output: ReturnType,
}
```

### 7.2 Generating with quote

**Basic quoting:**

```rust
use quote::quote;

let name = quote::format_ident!("my_function");
let body = quote! { println!("Hello"); };

let generated = quote! {
    fn #name() {
        #body
    }
};
```

**Interpolation modes:**

```rust
let ident = syn::Ident::new("foo", proc_macro2::Span::call_site());
let ty = quote!(i32);
let exprs = vec![quote!(1), quote!(2), quote!(3)];

quote! {
    // #ident - single identifier
    let #ident = 42;

    // #ty - type
    let x: #ty = 0;

    // #(#exprs),* - repetition with separator
    let arr = [#(#exprs),*];  // [1, 2, 3]

    // #(#exprs)* - repetition without separator
    #( #exprs )*  // 1 2 3
}
```

### 7.3 Span Preservation

**Why spans matter:** Spans determine where errors point to.

```rust
// Bad: All errors point to macro definition
fn bad_macro(input: TokenStream) -> TokenStream {
    let expanded = quote! {
        // Error points here (in macro)
        invalid_code!();
    };
    expanded.into()
}

// Good: Errors point to user's code
fn good_macro(input: TokenStream) -> TokenStream {
    let input_span = input.span();

    let expanded = quote_spanned! { input_span =>
        // Error points to macro input (user's code)
        potentially_invalid_code!();
    };
    expanded.into()
}
```

---

## 8. Build-Time Codegen Patterns

### 8.1 Pattern: Marker Const

Use a const to trigger side effects at compile time:

```rust
// Pattern used by pavex_macros
const _: () = {
    // This runs at compile time
    register_type::<MyType>();
};

// Equivalent to:
#[used]
static _REGISTRATION: () = register_type::<MyType>();
```

### 8.2 Pattern: Phantom Registration

```rust
// Create a phantom type that carries registration info
pub struct ConstructorMarker<T> {
    _phantom: PhantomData<T>,
}

impl<T> ConstructorMarker<T> {
    pub const fn new() -> Self {
        // Side effect: register with global registry
        register_constructor::<T>();
        Self { _phantom: PhantomData }
    }
}

// Usage in macro:
const _: ConstructorMarker<MyType> = ConstructorMarker::new();
```

### 8.3 Pattern: Attribute Aggregation

Collect information from multiple attributes:

```rust
// User writes:
#[api_endpoint(GET, "/users")]
#[api_param(query, "limit", u32)]
#[api_param(path, "id", Uuid)]
async fn get_user(...) { ... }

// Macro aggregates:
struct EndpointInfo {
    method: Method,
    path: String,
    params: Vec<Param>,
}

// Then generates router registration
```

---

## 9. Error Handling in Macros

### 9.1 Returning Compile Errors

```rust
use proc_macro2::TokenStream;
use quote::quote;

fn my_macro(input: TokenStream) -> TokenStream {
    match try_expand(input) {
        Ok(tokens) => tokens,
        Err(e) => e.to_compile_error(),  // Returns error as TokenStream
    }
}

fn try_expand(input: TokenStream) -> Result<TokenStream, syn::Error> {
    let item_fn = syn::parse::<ItemFn>(input)?;  // ? propagates errors

    if item_fn.sig.asyncness.is_none() {
        return Err(syn::Error::new_spanned(
            item_fn.sig.fn_token,
            "Handler functions must be async",
        ));
    }

    Ok(quote! { /* generated code */ })
}
```

### 9.2 Diagnostic Output

```rust
// Using pavex's diagnostic system
use pavex_cli_diagnostic::Diagnostic;

fn emit_diagnostic(error: &MyError) {
    let diag = Diagnostic::builder()
        .message("Invalid route path")
        .label(error.span, "this path pattern is invalid")
        .help("Use {param} for parameters")
        .build();

    eprintln!("{:?}", diag);
}
```

---

## 10. Testing Procedural Macros

### 10.1 trybuild for Compile Tests

```rust
// tests/ui.rs
#[test]
fn ui() {
    let t = trybuild::TestCases::new();

    // Tests that should compile
    t.pass("tests/ui/constructor_pass.rs");
    t.pass("tests/ui/singleton_pass.rs");

    // Tests that should fail with specific errors
    t.compile_fail("tests/ui/constructor_fail.rs");
    t.compile_fail("tests/ui/missing_lifetime.rs");
}
```

### 10.2 Example Test Cases

```rust
// tests/ui/constructor_pass.rs
use pavex::constructor;

#[constructor(pavex::blueprint::constructor::Lifecycle::Singleton)]
fn my_constructor() -> MyType {
    MyType::new()
}

fn main() {}

// tests/ui/constructor_fail.rs (should error)
use pavex::constructor;

#[constructor(pavex::blueprint::constructor::Lifecycle::Singleton)]
fn my_constructor() {  // ERROR: Missing return type
    // ...
}

fn main() {}
```

### 10.3 Macro Expansion Debugging

```bash
# Install cargo-expand
cargo install cargo-expand

# Expand macros
cargo expand --lib

# See what your macro generates
```

---

## 11. Complete Example: Mini DI Macro

Let's build a simplified version of Pavex's constructor system:

```rust
// my_di_macros/src/lib.rs
use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, parse_quote, ItemFn, ReturnType};

#[proc_macro_attribute]
pub fn singleton(_metadata: TokenStream, input: TokenStream) -> TokenStream {
    constructor_impl("Singleton", input)
}

#[proc_macro_attribute]
pub fn transient(_metadata: TokenStream, input: TokenStream) -> TokenStream {
    constructor_impl("Transient", input)
}

fn constructor_impl(lifecycle: &str, input: TokenStream) -> TokenStream {
    let item_fn = parse_macro_input!(input as ItemFn);

    // Validate: must have return type
    let output = match &item_fn.sig.output {
        ReturnType::Default => {
            return syn::Error::new_spanned(
                &item_fn.sig.fn_token,
                "Constructor must have a return type",
            )
            .to_compile_error()
            .into();
        }
        ReturnType::Type(_, ty) => ty.clone(),
    };

    let fn_name = &item_fn.sig.ident;
    let lifecycle_ident = syn::Ident::new(lifecycle, proc_macro2::Span::call_site());

    // Generate: original function + registration
    let expanded = quote! {
        // Original function
        #item_fn

        // Registration marker
        const _: #fn_name::RegistrationMarker = #fn_name::RegistrationMarker {
            _lifecycle: std::marker::PhantomData::<fn() -> #lifecycle_ident>,
        };

        // Helper module for this constructor
        mod #fn_name {
            use super::*;

            pub struct RegistrationMarker<L> {
                _lifecycle: std::marker::PhantomData<L>,
            }

            impl RegistrationMarker<#lifecycle_ident> {
                pub const fn type_info() -> &'static str {
                    stringify!(#output)
                }
            }
        }
    };

    TokenStream::from(expanded)
}
```

**Usage:**

```rust
use my_di_macros::{singleton, transient};

struct Config {
    value: String,
}

#[singleton]
fn config() -> Config {
    Config { value: "test".into() }
}

#[transient]
fn logger() -> Logger {
    Logger::new()
}
```

---

## Key Takeaways

1. **Procedural macros transform TokenStreams** - Parse with syn, generate with quote
2. **Spans matter for diagnostics** - Use `quote_spanned!` for accurate error locations
3. **The f! macro tricks IDEs** - Conditional compilation for IDE hints without runtime cost
4. **Const markers enable registration** - Compile-time side effects via `const _: ()`
5. **Attribute macros preserve input** - Original item is typically included in output
6. **Test with trybuild** - Verify both successful and failing compilations

---

## Related Files

- **Macro implementation**: `/home/darkvoid/Boxxed/@formulas/src.rust/src.BuildTooling/pavex/libs/pavex_macros/src/lib.rs`
- **Constructor macros**: `/home/darkvoid/Boxxed/@formulas/src.rust/src.BuildTooling/pavex/libs/pavex_macros/src/constructor.rs`
- **f! macro**: `/home/darkvoid/Boxxed/@formulas/src.rust/src.BuildTooling/pavex/libs/pavex/src/lib.rs`

---

*Next: [02-dependency-resolution-deep-dive.md](02-dependency-resolution-deep-dive.md)*
