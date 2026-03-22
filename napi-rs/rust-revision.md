---
source: /home/darkvoid/Boxxed/@dev/repo-expolorations/napi-rs/
explored_at: 2026-03-22
revised_at: 2026-03-22
workspace: napi-rs-rust-workspace
---

# Rust Revision: napi-rs Clone

## Overview

This document provides guidance for building Node-API bindings in Rust, either using the existing napi-rs ecosystem or creating custom implementations for specific use cases.

## Workspace Structure

```
napi-rs-workspace/
├── Cargo.toml                    # Workspace definition
├── crates/
│   ├── napi-core/                # Core N-API bindings
│   ├── napi-derive/              # Procedural macros
│   ├── napi-build/               # Build helpers
│   └── napi-sys/                 # Low-level FFI bindings
├── examples/
│   ├── basic/                    # Basic example
│   ├── async/                    # Async patterns
│   └── class/                    # Class bindings
└── tests/
    ├── integration/              # Integration tests
    └── e2e/                      # End-to-end tests
```

## Crate 1: napi-sys (Low-Level Bindings)

### Purpose
Raw FFI bindings to Node-API (node.h)

### Cargo.toml

```toml
[package]
name = "napi-sys"
version = "0.1.0"
edition = "2021"
description = "Low-level Node-API bindings for Rust"

[dependencies]
libc = "0.2"

[build-dependencies]
bindgen = "0.68"
```

### Implementation

```rust
// crates/napi-sys/src/lib.rs
#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(dead_code)]

use libc::{c_void, c_char, size_t, int32_t, uint32_t, int64_t, uint64_t};

// Opaque types
pub type napi_env = *mut c_void;
pub type napi_value = *mut c_void;
pub type napi_ref = *mut c_void;
pub type napi_deferred = *mut c_void;
pub type napi_threadsafe_function = *mut c_void;

// Status codes
#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum napi_status {
    napi_ok = 0,
    napi_invalid_arg,
    napi_object_expected,
    napi_function_expected,
    napi_generic_failure,
    napi_pending_exception,
    napi_cancelled,
    napi_escape_called,
    napi_name_mismatch,
    napi_closed,
    napi_exception_pending,
    napi_unhandled_rejection,
}

// Callback types
pub type napi_callback = Option<
    unsafe extern "C" fn(env: napi_env, info: napi_callback_info) -> napi_value
>;

pub type napi_callback_info = *mut c_void;
pub type napi_finalize = Option<
    unsafe extern "C" fn(env: napi_env, data: *mut c_void, finalize_hint: *mut c_void)
>;

// N-API Functions (version 1)
extern "C" {
    pub fn napi_module_register(mod_: *mut napi_module);

    pub fn napi_create_object(env: napi_env, result: *mut napi_value) -> napi_status;
    pub fn napi_create_array(env: napi_env, result: *mut napi_value) -> napi_status;
    pub fn napi_create_array_with_length(env: napi_env, length: size_t, result: *mut napi_value) -> napi_status;
    pub fn napi_create_int32(env: napi_env, value: int32_t, result: *mut napi_value) -> napi_status;
    pub fn napi_create_uint32(env: napi_env, value: uint32_t, result: *mut napi_value) -> napi_status;
    pub fn napi_create_double(env: napi_env, value: f64, result: *mut napi_value) -> napi_status;
    pub fn napi_create_string_utf8(env: napi_env, str_: *const c_char, length: size_t, result: *mut napi_value) -> napi_status;
    pub fn napi_create_bool(env: napi_env, value: bool, result: *mut napi_value) -> napi_status;
    pub fn napi_create_buffer(env: napi_env, length: size_t, data: *mut *mut c_void, result: *mut napi_value) -> napi_status;
    pub fn napi_create_external_buffer(env: napi_env, length: size_t, data: *mut c_void, finalize_cb: napi_finalize, finalize_hint: *mut c_void, result: *mut napi_value, data_out: *mut *mut c_void) -> napi_status;
    pub fn napi_create_function(env: napi_env, utf8name: *const c_char, length: size_t, cb: napi_callback, data: *mut c_void, result: *mut napi_value) -> napi_status;

    pub fn napi_get_value_int32(env: napi_env, value: napi_value, result: *mut int32_t) -> napi_status;
    pub fn napi_get_value_uint32(env: napi_env, value: napi_value, result: *mut uint32_t) -> napi_status;
    pub fn napi_get_value_double(env: napi_env, value: napi_value, result: *mut f64) -> napi_status;
    pub fn napi_get_value_string_utf8(env: napi_env, value: napi_value, buf: *mut c_char, bufsize: size_t, result: *mut size_t) -> napi_status;
    pub fn napi_get_value_bool(env: napi_env, value: napi_value, result: *mut bool) -> napi_status;
    pub fn napi_get_array_length(env: napi_env, value: napi_value, result: *mut uint32_t) -> napi_status;

    pub fn napi_set_named_property(env: napi_env, object: napi_value, utf8name: *const c_char, value: napi_value) -> napi_status;
    pub fn napi_get_named_property(env: napi_env, object: napi_value, utf8name: *const c_char, result: *mut napi_value) -> napi_status;
    pub fn napi_set_element(env: napi_env, object: napi_value, index: uint32_t, value: napi_value) -> napi_status;
    pub fn napi_get_element(env: napi_env, object: napi_value, index: uint32_t, result: *mut napi_value) -> napi_status;

    pub fn napi_call_function(env: napi_env, recv: napi_value, func: napi_value, argc: size_t, argv: *const napi_value, result: *mut napi_value) -> napi_status;
    pub fn napi_new_instance(env: napi_env, constructor: napi_value, argc: size_t, argv: *const napi_value, result: *mut napi_value) -> napi_status;

    pub fn napi_get_global(env: napi_env, result: *mut napi_value) -> napi_status;
    pub fn napi_get_undefined(env: napi_env, result: *mut napi_value) -> napi_status;
    pub fn napi_get_null(env: napi_env, result: *mut napi_value) -> napi_status;

    pub fn napi_throw(env: napi_env, error: napi_value) -> napi_status;
    pub fn napi_throw_error(env: napi_env, code: *const c_char, msg: *const c_char) -> napi_status;
    pub fn napi_throw_type_error(env: napi_env, code: *const c_char, msg: *const c_char) -> napi_status;
    pub fn napi_throw_range_error(env: napi_env, code: *const c_char, msg: *const c_char) -> napi_status;

    pub fn napi_create_promise(env: napi_env, deferred: *mut napi_deferred, promise: *mut napi_value) -> napi_status;
    pub fn napi_resolve_deferred(env: napi_env, deferred: napi_deferred, resolution: napi_value) -> napi_status;
    pub fn napi_reject_deferred(env: napi_env, deferred: napi_deferred, rejection: napi_value) -> napi_status;
}

// Module registration
#[repr(C)]
pub struct napi_module {
    pub nm_version: c_int,
    pub nm_flags: c_uint,
    pub nm_filename: *const c_char,
    pub nm_register_func: Option<unsafe extern "C" fn(env: napi_env, exports: napi_value) -> napi_value>,
    pub nm_modname: *const c_char,
    pub nm_priv: *mut c_void,
    pub reserved: [*mut c_void; 4],
}

// Macro for module registration
#[macro_export]
macro_rules! napi_module {
    ($name:ident, $func:ident) => {
        #[no_mangle]
        pub extern "C" fn napi_register_module_v1(env: $crate::napi_env, exports: $crate::napi_value) -> $crate::napi_value {
            $func(env, exports)
        }
    };
}
```

## Crate 2: napi-core (High-Level Bindings)

### Purpose
Safe, ergonomic Rust API for Node-API

### Cargo.toml

```toml
[package]
name = "napi-core"
version = "0.1.0"
edition = "2021"

[dependencies]
napi-sys = { path = "../napi-sys" }
thiserror = "1"
serde = { version = "1", features = ["derive"] }
serde_json = "1"

[features]
async = ["tokio"]
```

### Core Types

```rust
// crates/napi-core/src/lib.rs
use napi_sys::*;
use std::ffi::CString;
use thiserror::Error;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Error, Debug)]
pub enum Error {
    #[error("N-API error: {0:?}")]
    Napi(napi_status),
    #[error("Invalid argument: {0}")]
    InvalidArg(String),
    #[error("String contains null byte")]
    NulError(#[from] std::ffi::NulError),
}

impl From<napi_status> for Error {
    fn from(status: napi_status) -> Self {
        Error::Napi(status)
    }
}

/// JavaScript environment wrapper
pub struct Env {
    inner: napi_env,
}

impl Env {
    pub fn from_raw(env: napi_env) -> Self {
        Self { inner: env }
    }

    pub fn raw(&self) -> napi_env {
        self.inner
    }

    pub fn create_object(&self) -> Result<JsObject> {
        let mut ptr = std::ptr::null_mut();
        unsafe {
            napi_create_object(self.inner, &mut ptr)?;
        }
        Ok(JsObject::from_raw(self.inner, ptr))
    }

    pub fn create_array(&self, length: usize) -> Result<JsArray> {
        let mut ptr = std::ptr::null_mut();
        unsafe {
            napi_create_array_with_length(self.inner, length, &mut ptr)?;
        }
        Ok(JsArray::from_raw(self.inner, ptr))
    }

    pub fn create_string(&self, s: &str) -> Result<JsString> {
        let c_str = CString::new(s)?;
        let mut ptr = std::ptr::null_mut();
        unsafe {
            napi_create_string_utf8(self.inner, c_str.as_ptr(), s.len(), &mut ptr)?;
        }
        Ok(JsString::from_raw(self.inner, ptr))
    }

    pub fn create_int32(&self, value: i32) -> Result<JsNumber> {
        let mut ptr = std::ptr::null_mut();
        unsafe {
            napi_create_int32(self.inner, value, &mut ptr)?;
        }
        Ok(JsNumber::from_raw(self.inner, ptr))
    }

    pub fn create_bool(&self, value: bool) -> Result<JsBoolean> {
        let mut ptr = std::ptr::null_mut();
        unsafe {
            napi_create_bool(self.inner, value, &mut ptr)?;
        }
        Ok(JsBoolean::from_raw(self.inner, ptr))
    }
}

/// JavaScript value wrapper
pub struct JsValue {
    env: napi_env,
    inner: napi_value,
}

impl JsValue {
    pub fn from_raw(env: napi_env, inner: napi_value) -> Self {
        Self { env, inner }
    }

    pub fn raw(&self) -> napi_value {
        self.inner
    }

    pub fn env(&self) -> Env {
        Env::from_raw(self.env)
    }
}

/// JavaScript string
pub struct JsString {
    env: napi_env,
    inner: napi_value,
}

impl JsString {
    pub fn from_raw(env: napi_env, inner: napi_value) -> Self {
        Self { env, inner }
    }

    pub fn to_string(&self) -> Result<String> {
        let mut len = 0;
        unsafe {
            napi_get_value_string_utf8(self.env, self.inner, std::ptr::null_mut(), 0, &mut len)?;
            let mut buf = vec![0u8; len + 1];
            napi_get_value_string_utf8(self.env, self.inner, buf.as_mut_ptr() as _, buf.len(), &mut len)?;
            buf.truncate(len);
            Ok(String::from_utf8_lossy(&buf).to_string())
        }
    }
}

/// JavaScript object
pub struct JsObject {
    env: napi_env,
    inner: napi_value,
}

impl JsObject {
    pub fn from_raw(env: napi_env, inner: napi_value) -> Self {
        Self { env, inner }
    }

    pub fn set<K, V>(&mut self, key: K, value: V) -> Result<()>
    where
        K: AsRef<str>,
        V: Into<JsValue>,
    {
        let c_key = CString::new(key.as_ref())?;
        unsafe {
            napi_set_named_property(
                self.env,
                self.inner,
                c_key.as_ptr(),
                value.into().raw(),
            )?;
        }
        Ok(())
    }

    pub fn get(&self, key: &str) -> Result<Option<JsValue>> {
        let c_key = CString::new(key)?;
        let mut ptr = std::ptr::null_mut();
        unsafe {
            napi_get_named_property(self.env, self.inner, c_key.as_ptr(), &mut ptr)?;
        }
        if ptr.is_null() {
            Ok(None)
        } else {
            Ok(Some(JsValue::from_raw(self.env, ptr)))
        }
    }
}

/// JavaScript array
pub struct JsArray {
    env: napi_env,
    inner: napi_value,
}

impl JsArray {
    pub fn from_raw(env: napi_env, inner: napi_value) -> Self {
        Self { env, inner }
    }

    pub fn len(&self) -> Result<usize> {
        let mut len: u32 = 0;
        unsafe {
            napi_get_array_length(self.env, self.inner, &mut len)?;
        }
        Ok(len as usize)
    }

    pub fn is_empty(&self) -> Result<bool> {
        Ok(self.len()? == 0)
    }

    pub fn push<V>(&mut self, value: V) -> Result<()>
    where
        V: Into<JsValue>,
    {
        let len = self.len()? as u32;
        unsafe {
            napi_set_element(self.env, self.inner, len, value.into().raw())?;
        }
        Ok(())
    }
}
```

## Crate 3: napi-derive (Procedural Macros)

### Purpose
Derive macros for ergonomic N-API bindings

### Cargo.toml

```toml
[package]
name = "napi-derive"
version = "0.1.0"
edition = "2021"

[lib]
proc-macro = true

[dependencies]
quote = "1"
syn = { version = "2", features = ["full"] }
proc-macro2 = "1"
```

### Implementation

```rust
// crates/napi-derive/src/lib.rs
use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, ItemFn, ItemStruct, ImplItem};

/// Mark a function as exported to JavaScript
#[proc_macro_attribute]
pub fn napi(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let input = parse_macro_input!(item as ItemFn);
    let name = &input.sig.ident;

    let wrapped = quote! {
        #input

        #[no_mangle]
        unsafe extern "C" fn #name##_napi(
            env: napi_sys::napi_env,
            info: napi_sys::napi_callback_info,
        ) -> napi_sys::napi_value {
            use napi_core::Env;

            let env = Env::from_raw(env);

            // Extract arguments
            let mut argc = 0;
            napi_sys::napi_get_cb_info(
                env.raw(),
                info,
                &mut argc,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                std::ptr::null_mut(),
            );

            let mut args = vec![std::ptr::null_mut(); argc];
            napi_sys::napi_get_cb_info(
                env.raw(),
                info,
                &mut argc,
                args.as_mut_ptr(),
                std::ptr::null_mut(),
                std::ptr::null_mut(),
            );

            // Convert arguments
            let rust_args = {
                let mut converted = Vec::new();
                for arg in args {
                    let val = napi_core::JsValue::from_raw(env.raw(), arg);
                    converted.push(val);
                }
                converted
            };

            // Call the original function
            let result = #name(rust_args);

            // Convert result back to JavaScript
            result.into_value(env.raw())
        }
    };

    wrapped.into()
}

/// Mark a struct as exported to JavaScript
#[proc_macro_derive(NapiObject)]
pub fn derive_napi_object(item: TokenStream) -> TokenStream {
    let input = parse_macro_input!(item as ItemStruct);
    let name = &input.ident;

    let expanded = quote! {
        impl napi_core::ToNapiValue for #name {
            unsafe fn to_napi_value(env: napi_sys::napi_env, val: Self) -> napi_core::Result<napi_sys::napi_value> {
                let env = napi_core::Env::from_raw(env);
                let mut obj = env.create_object()?;

                // Set each field
                #(
                    obj.set(stringify!(#name), val.#name)?;
                )*

                Ok(obj.into())
            }
        }
    };

    expanded.into()
}
```

## Crate 4: napi-build (Build Helper)

### Purpose
Build script helpers for N-API modules

### Cargo.toml

```toml
[package]
name = "napi-build"
version = "0.1.0"
edition = "2021"

[dependencies]
```

### Implementation

```rust
// crates/napi-build/src/lib.rs
use std::env;

/// Setup build environment for N-API module
pub fn setup() {
    // Set cargo output directory
    println!("cargo:rustc-cdylib-link-arg=-Wl,-undefined,dynamic_lookup");

    // Rebuild if Node.js version changes
    if let Ok(node_version) = env::var("NAPI_NODE_VERSION") {
        println!("cargo:rerun-if-env-changed=NAPI_NODE_VERSION");
    }

    // Platform-specific flags
    let target = env::var("TARGET").unwrap_or_default();

    if target.contains("linux") {
        println!("cargo:rustc-link-lib=dylib=pthread");
    }

    if target.contains("windows") {
        println!("cargo:rustc-link-lib=dylib=node");
    }
}
```

## Example Usage

### Basic Module

```rust
// examples/basic/src/lib.rs
use napi_core::{Env, JsValue, Result};
use napi_derive::napi;

#[napi]
pub fn add(a: i32, b: i32) -> i32 {
    a + b
}

#[napi]
pub fn greet(name: String) -> String {
    format!("Hello, {}!", name)
}

#[napi]
pub fn get_user_data() -> UserData {
    UserData {
        id: 1,
        name: "Alice".to_string(),
        email: "alice@example.com".to_string(),
    }
}

#[derive(napi_derive::NapiObject)]
pub struct UserData {
    pub id: i32,
    pub name: String,
    pub email: String,
}

// Module registration
napi_core::register_module!(init);

fn init(env: &Env, exports: &mut JsObject) -> Result<()> {
    exports.set("add", add)?;
    exports.set("greet", greet)?;
    exports.set("getUserData", get_user_data)?;
    Ok(())
}
```

### Async Example

```rust
// examples/async/src/lib.rs
use napi_core::{Env, JsDeferred, JsObject, Result};
use std::time::Duration;

#[napi_derive::napi]
pub async fn sleep(ms: u64) -> Result<String> {
    tokio::time::sleep(Duration::from_millis(ms)).await;
    Ok(format!("Slept for {}ms", ms))
}

#[napi_derive::napi]
pub fn read_file(path: String) -> Result<JsObject> {
    let (deferred, promise) = JsDeferred::new()?;

    std::thread::spawn(move || {
        let content = std::fs::read_to_string(&path);
        match content {
            Ok(text) => deferred.resolve(text),
            Err(e) => deferred.reject(e.to_string()),
        }
    });

    Ok(promise)
}
```

## JavaScript/TypeScript Usage

```typescript
// Generated TypeScript definitions
export declare function add(a: number, b: number): number;
export declare function greet(name: string): string;
export declare function getUserData(): UserData;

export interface UserData {
  id: number;
  name: string;
  email: string;
}

export declare function sleep(ms: number): Promise<string>;
export declare function readFile(path: string): Promise<string>;

// Usage
import { add, greet, getUserData, sleep } from './index';

console.log(add(1, 2));           // 3
console.log(greet('World'));       // Hello, World!
console.log(getUserData());        // { id: 1, name: 'Alice', email: 'alice@example.com' }

const result = await sleep(1000);
console.log(result);               // Slept for 1000ms
```

## Summary

This Rust revision provides:
- **Low-level FFI bindings** (napi-sys)
- **Safe high-level API** (napi-core)
- **Procedural macros** (napi-derive)
- **Build helpers** (napi-build)
- **Example patterns** for common use cases
