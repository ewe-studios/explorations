---
title: "Macro System Deep Dive"
subtitle: "Procedural macros, derive macros, attribute macros, and compile-time code generation in Rust"
location: /home/darkvoid/Boxxed/@dev/repo-expolorations/boxer/04-macro-system-deep-dive.md
related_to: ./exploration.md
created: 2026-03-27
status: complete
---

# Macro System Deep Dive

## Executive Summary

This deep dive covers Rust's macro system and how it applies to Boxer development:

1. **Procedural Macros Overview** - Types and use cases
2. **Derive Macros** - Automatic trait implementations
3. **Attribute Macros** - Custom attributes
4. **Function-like Macros** - DSL creation
5. **Compile-time Code Generation** - Metaprogramming patterns

**Note:** While Boxer currently doesn't use extensive macros, this guide shows how macros could enhance the framework.

---

## 1. Procedural Macros Overview

### Three Types of Procedural Macros

```rust
// 1. Derive macros - Implement traits automatically
#[derive(Debug, Clone, Serialize)]
struct MyStruct { ... }

// 2. Attribute macros - Custom attributes
#[my_custom_attribute]
fn my_function() { ... }

// 3. Function-like macros - Macro that looks like a function
let x = my_macro!(arg1, arg2);
```

### Macro Crate Structure

```
boxer-macros/
├── Cargo.toml              # proc-macro = true
├── src/
│   ├── lib.rs              # Entry point
│   ├── derive.rs           # Derive macro implementations
│   ├── attribute.rs        # Attribute macro implementations
│   └── function.rs         # Function-like macros
└── tests/
    └── ui/                 # UI tests for macro output
```

```toml
# Cargo.toml
[package]
name = "boxer-macros"
version = "0.1.0"
edition = "2021"

[lib]
proc-macro = true  # Required for procedural macros

[dependencies]
syn = "2.0"
quote = "1.0"
proc-macro2 = "1.0"
```

---

## 2. Derive Macros

### Basic Derive Macro

```rust
// boxer-macros/src/lib.rs

use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, DeriveInput};

#[proc_macro_derive(BoxerResource)]
pub fn derive_boxer_resource(input: TokenStream) -> TokenStream {
    // Parse the input tokens into a syntax tree
    let input = parse_macro_input!(input as DeriveInput);
    let name = &input.ident;

    // Generate the implementation
    let expanded = quote! {
        impl BoxerResource for #name {
            fn resource_type() -> &'static str {
                stringify!(#name)
            }

            fn validate(&self) -> Result<(), ValidationError> {
                Ok(())  // Default implementation
            }
        }
    };

    TokenStream::from(expanded)
}
```

### Usage Example

```rust
// In boxer crate
use boxer_macros::BoxerResource;

#[derive(BoxerResource)]
struct Dockerfile {
    from: String,
    commands: Vec<Command>,
}

// Expands to:
// impl BoxerResource for Dockerfile {
//     fn resource_type() -> &'static str { "Dockerfile" }
//     fn validate(&self) -> Result<(), ValidationError> { Ok(()) }
// }
```

### Advanced Derive with Attributes

```rust
// boxer-macros/src/derive.rs

use syn::{Data, Fields, Field};

pub fn derive_boxer_resource_impl(input: &DeriveInput) -> TokenStream2 {
    let name = &input.ident;

    // Extract fields
    let fields = match &input.data {
        Data::Struct(data) => &data.fields,
        _ => panic!("BoxerResource only supports structs"),
    };

    // Generate validation code for each field
    let field_validations = fields.iter().filter_map(|field| {
        let field_name = &field.ident;
        let attrs = &field.attrs;

        // Check for #[validate(required)] attribute
        if has_attribute(attrs, "validate", "required") {
            Some(quote! {
                if self.#field_name.is_empty() {
                    return Err(ValidationError::new(stringify!(#field_name)));
                }
            })
        } else {
            None
        }
    });

    quote! {
        impl BoxerResource for #name {
            fn resource_type() -> &'static str {
                stringify!(#name)
            }

            fn validate(&self) -> Result<(), ValidationError> {
                #(#field_validations)*
                Ok(())
            }
        }
    }
}
```

### Usage with Validation

```rust
use boxer_macros::BoxerResource;

#[derive(BoxerResource)]
struct Dockerfile {
    #[validate(required)]
    from: String,

    #[validate(required)]
    entrypoint: String,

    commands: Vec<Command>,  // Optional, no validation
}
```

---

## 3. Attribute Macros

### Item Attribute Macros

```rust
// boxer-macros/src/lib.rs

use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, ItemFn};

/// Marks a function as a Boxer command handler
#[proc_macro_attribute]
pub fn boxer_command(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let input = parse_macro_input!(item as ItemFn);
    let name = &input.sig.ident;
    let block = &input.block;
    let inputs = &input.sig.inputs;

    let expanded = quote! {
        #input

        // Generate registration code
        ::boxer::registry::register_command(
            stringify!(#name),
            |args| {
                #name(args)
            }
        );
    };

    TokenStream::from(expanded)
}
```

### Usage

```rust
use boxer_macros::boxer_command;

#[boxer_command]
fn build(args: BuildArgs) -> Result<()> {
    // Build logic here
    Ok(())
}

#[boxer_command]
fn run(args: RunArgs) -> Result<()> {
    // Run logic here
    Ok(())
}

// Automatically registers commands in global registry
```

### Attribute Macros with Parameters

```rust
// boxer-macros/src/lib.rs

#[proc_macro_attribute]
pub fn wasm_export(attr: TokenStream, item: TokenStream) -> TokenStream {
    let args = parse_macro_input!(attr as syn::AttributeArgs);
    let input = parse_macro_input!(item as ItemFn);

    // Parse export name from attribute
    let export_name = if args.is_empty() {
        input.sig.ident.to_string()
    } else {
        // Extract name from #[wasm_export(name = "custom_name")]
        extract_name(&args)
    };

    let name = &input.sig.ident;
    let block = &input.block;

    let expanded = quote! {
        // Original function
        #input

        // Export wrapper
        #[no_mangle]
        pub extern "C" fn #export_name() {
            #name()
        }
    };

    TokenStream::from(expanded)
}
```

### Usage

```rust
use boxer_macros::wasm_export;

#[wasm_export]  // Exports as "_start"
fn _start() {
    run_application();
}

#[wasm_export(name = "wasm_vfs_mount_in_memory")]
fn mount_in_memory() {
    mount_files();
}
```

---

## 4. Function-like Macros

### Basic Declarative Macro

```rust
// In boxer crate (not proc-macro)

/// Create a PathBuf from string literals
#[macro_export]
macro_rules! path {
    ($($segment:literal)/+) => {
        $crate::path::PathBuf::from(concat!($($segment, "/"),* ))
    };
    ($segment:literal) => {
        $crate::path::PathBuf::from($segment)
    };
}

// Usage
let root = path!("/");
let app = path!("/app");
let data = path!("/app/data");
```

### Advanced Procedural Function-like Macro

```rust
// boxer-macros/src/lib.rs

use syn::{parse_macro_input, Expr};

/// Generate FileDef struct at compile time
#[proc_macro]
pub fn file_def(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as syn::ExprTuple);

    let expanded = quote! {
        FileDef {
            path_off: #path_offset,
            data_off: #data_offset,
            data_len: #data_len,
        }
    };

    TokenStream::from(expanded)
}
```

### DSL Creation with Macros

```rust
// boxer-macros/src/lib.rs

#[proc_macro]
pub fn define_box(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as syn::ItemStruct);
    let name = &input.ident;

    // Extract fields and generate builder pattern
    let fields = &input.fields;
    let field_names: Vec<_> = fields.iter()
        .filter_map(|f| f.ident.as_ref())
        .collect();

    let expanded = quote! {
        #input

        impl #name {
            pub fn builder() -> #name Builder {
                #name Builder::default()
            }
        }

        #[derive(Default)]
        pub struct #name Builder {
            #(pub #field_names: Option<#field_names>,)*
        }

        impl #name Builder {
            #(
                pub fn #field_names(mut self, value: impl Into<#field_names>) -> Self {
                    self.#field_names = Some(value.into());
                    self
                }
            )*

            pub fn build(self) -> Result<#name> {
                Ok(#name {
                    #(#field_names: self.#field_names.ok_or("Missing field")?,)*
                })
            }
        }
    };

    TokenStream::from(expanded)
}
```

### Usage

```rust
use boxer_macros::define_box;

#[define_box]
struct DockerBox {
    from: String,
    entrypoint: Vec<String>,
    env: HashMap<String, String>,
    volumes: Vec<String>,
}

// Usage
let box = DockerBox::builder()
    .from("scratch")
    .entrypoint(vec!["/app/main".to_string()])
    .env("RUST_LOG", "debug")
    .build()?;
```

---

## 5. Compile-time Code Generation

### Const Evaluation

```rust
// Compile-time path validation
const fn validate_path(path: &str) -> bool {
    // Simple validation at compile time
    if path.is_empty() {
        return false;
    }
    // Check for valid characters
    true
}

// Usage
const ROOT_PATH: &str = "/";
const APP_PATH: &str = "/app";

// Compile-time assertion
const _: () = assert!(validate_path(ROOT_PATH));
const _: () = assert!(validate_path(APP_PATH));
```

### Build-time Code Generation with build.rs

```rust
// boxer/build.rs

use std::env;
use std::fs;
use std::path::Path;

fn main() {
    // Generate syscall table at build time
    let out_dir = env::var("OUT_DIR").unwrap();
    let dest_path = Path::new(&out_dir).join("syscalls.rs");

    let syscalls = generate_syscall_table();

    fs::write(&dest_path, syscalls).unwrap();

    println!("cargo:rerun-if-changed=syscall_definitions.txt");
}

fn generate_syscall_table() -> String {
    // Read syscall definitions and generate Rust code
    let definitions = std::fs::read_to_string("syscall_definitions.txt").unwrap();

    let mut output = String::new();
    output.push_str("// Auto-generated syscall table\n\n");
    output.push_str("pub const SYSCALLS: &[(&str, SyscallHandler)] = &[\n");

    for line in definitions.lines() {
        if line.starts_with('#') || line.trim().is_empty() {
            continue;
        }

        let parts: Vec<&str> = line.split(',').collect();
        if parts.len() == 2 {
            output.push_str(&format!(
                "    (\"{}\", {}),\n",
                parts[0].trim(),
                parts[1].trim()
            ));
        }
    }

    output.push_str("];\n");
    output
}
```

### Usage of Generated Code

```rust
// In boxer crate

// Include generated syscall table
include!(concat!(env!("OUT_DIR"), "/syscalls.rs"));

// Use at runtime
pub fn handle_syscall(name: &str, args: &[u8]) -> Result<i32> {
    for &(syscall_name, handler) in SYSCALLS {
        if syscall_name == name {
            return handler(args);
        }
    }
    Err(Error::UnknownSyscall)
}
```

---

## 6. Macros for WASM

### WASM Export Macro

```rust
// boxer-macros/src/lib.rs

/// Generate WASM export with proper signature
#[proc_macro_attribute]
pub fn wasm_export(attr: TokenStream, item: TokenStream) -> TokenStream {
    let input = parse_macro_input!(item as ItemFn);
    let name = &input.sig.ident;
    let sig = &input.sig;
    let block = &input.block;

    // Parse export name
    let export_name = if attr.is_empty() {
        name.to_string()
    } else {
        parse_export_name(attr)
    };

    let expanded = quote! {
        // Original function (internal)
        fn #name #sig #block

        // WASM export wrapper
        #[no_mangle]
        pub extern "C" fn #export_name() {
            #name()
        }
    };

    TokenStream::from(expanded)
}
```

### FFI Type Generation

```rust
// Generate FFI-compatible types
#[proc_macro_derive(WasmFfi)]
pub fn derive_wasm_ffi(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let name = &input.ident;

    let expanded = quote! {
        // Ensure FFI compatibility
        #[repr(C)]
        impl #name {
            /// Get size for FFI
            pub const fn ffi_size() -> usize {
                core::mem::size_of::<Self>()
            }

            /// Get alignment for FFI
            pub const fn ffi_align() -> usize {
                core::mem::align_of::<Self>()
            }

            /// Convert to bytes for FFI
            pub fn to_bytes(&self) -> &[u8] {
                unsafe {
                    core::slice::from_raw_parts(
                        self as *const Self as *const u8,
                        Self::ffi_size()
                    )
                }
            }
        }
    };

    TokenStream::from(expanded)
}
```

---

## 7. Macro Best Practices

### When to Use Macros

| Scenario | Use Macro? | Alternative |
|----------|------------|-------------|
| Reduce boilerplate | ✅ Yes | Manual impl |
| DSL creation | ✅ Yes | Regular functions |
| Compile-time validation | ✅ Yes | Runtime checks |
| Simple string formatting | ❌ No | `format!()` |
| One-off code generation | ❌ No | Copy-paste |

### Macro Hygiene

```rust
// BAD: Non-hygienic macro
#[proc_macro]
pub fn bad_macro(input: TokenStream) -> TokenStream {
    quote! {
        let result = #input;  // 'result' might conflict
        result
    }
}

// GOOD: Hygienic macro
#[proc_macro]
pub fn good_macro(input: TokenStream) -> TokenStream {
    let result_ident = syn::Ident::new(
        &format!("_result_{}", LineColumn::default().line),
        Span::call_site()
    );

    quote! {
        let #result_ident = #input;
        #result_ident
    }
}
```

### Error Handling in Macros

```rust
use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, Result, Error};

#[proc_macro_derive(BoxerResource)]
pub fn derive_boxer_resource(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    // Try to generate code
    match generate_impl(&input) {
        Ok(expanded) => TokenStream::from(expanded),
        Err(e) => e.to_compile_error().into(),
    }
}

fn generate_impl(input: &DeriveInput) -> Result<TokenStream2> {
    // Validate input
    match &input.data {
        Data::Struct(_) => Ok(generate_struct_impl(input)),
        Data::Enum(_) => Err(Error::new_spanned(
            input,
            "BoxerResource only supports structs, not enums"
        )),
        Data::Union(_) => Err(Error::new_spanned(
            input,
            "BoxerResource only supports structs, not unions"
        )),
    }
}
```

---

## 8. Summary

### Macro Types Reference

| Type | Syntax | Use Case |
|------|--------|----------|
| Derive | `#[derive(Trait)]` | Auto-implement traits |
| Attribute | `#[custom_attr]` | Modify items |
| Function-like | `macro!(args)` | DSL, code generation |
| Declarative | `macro_rules!` | Simple patterns |
| Procedural | `proc_macro` | Complex transformations |

### Boxer Macro Opportunities

| Feature | Macro Type | Benefit |
|---------|-----------|---------|
| Dockerfile DSL | Function-like | Declarative syntax |
| Syscall handlers | Attribute | Auto-registration |
| FFI structs | Derive | Consistent layout |
| Validation | Derive | Compile-time checks |
| WASM exports | Attribute | Consistent exports |

---

## Document History

| Date | Change |
|------|--------|
| 2026-03-27 | Initial deep dive creation |

---

*Continue to [Rust Revision](rust-revision.md) for native Rust implementation notes.*
