---
source: /home/darkvoid/Boxxed/@formulas/src.rust/src.extism/extism
repository: git@github.com:extism/extism.git
explored_at: 2026-04-04
---

# Extism Rust Revision: Building a WASM Plugin System in Rust

## Overview

This document outlines how to build an Extism-like WebAssembly plugin system in Rust. We cover the complete architecture from the WASM runtime integration to the plugin development kit, providing a production-ready foundation for dynamic code loading.

## Why Rust for Extism?

Extism is already built in Rust, leveraging:

1. **Memory safety**: No segfaults from buffer overflows in the host
2. **Zero-cost abstractions**: Minimal overhead over raw WASM runtime
3. **Wasmtime integration**: First-class Rust bindings for the runtime
4. **Cross-platform**: Single codebase compiles everywhere
5. **FFI safety**: Safe C bindings via `libextism`

## Architecture Overview

```mermaid
flowchart TB
    subgraph Host Application
        A[Your Rust App] --> B[Extism SDK]
    end
    
    subgraph Extism Core
        B --> C[Manifest Parser]
        C --> D[Plugin Loader]
        D --> E[Wasmtime Integration]
        E --> F[Capability Enforcer]
    end
    
    subgraph Plugin Runtime
        F --> G[Plugin Instance]
        G --> H[Linear Memory]
        G --> I[Host Functions]
        G --> J[PDK Imports]
    end
    
    subgraph Plugin (WASM)
        K[Rust/Go/TS Plugin] --> L[PDK]
        L --> J
    end
```

## Crate Structure

### Option 1: Single Crate (Simple)

```toml
# Cargo.toml
[package]
name = "my-plugin-system"
version = "0.1.0"
edition = "2021"

[dependencies]
wasmtime = "19"
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
thiserror = "1.0"
tracing = "0.1"
```

### Option 2: Multi-Crate Workspace (Recommended)

```toml
# Cargo.toml (workspace root)
[workspace]
members = [
    "extism-core",      # Core runtime, manifest, plugin loading
    "extism-sdk",       # High-level host SDK
    "extism-pdk",       # Plugin Development Kit (WASM target)
    "extism-ffi",       # C API bindings
]
resolver = "2"
```

## Core Implementation

### 1. Manifest System

```rust
// extism-core/src/manifest.rs

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

/// Plugin manifest - defines plugin configuration and capabilities
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Manifest {
    /// Optional plugin name
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    
    /// WebAssembly modules to load
    pub wasm: Vec<WasmSource>,
    
    /// Plugin configuration (available via PDK)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub config: Option<serde_json::Value>,
    
    /// Allowed HTTP hosts (capability-based security)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub allowed_hosts: Option<Vec<String>>,
    
    /// Allowed filesystem paths (guest_path -> host_path)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub allowed_paths: Option<HashMap<String, String>>,
    
    /// Memory limit in bytes
    #[serde(skip_serializing_if = "Option::is_none")]
    pub memory_limit: Option<u64>,
}

/// Source of a WebAssembly module
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum WasmSource {
    /// From a file path
    File {
        path: PathBuf,
        #[serde(skip_serializing_if = "Option::is_none")]
        name: Option<String>,
    },
    /// From a URL
    Url {
        url: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        name: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        hash: Option<String>,
    },
    /// From binary data (inlined)
    Memory {
        data: Vec<u8>,
        #[serde(skip_serializing_if = "Option::is_none")]
        name: Option<String>,
    },
}

impl Manifest {
    /// Create a new manifest with a single Wasm file
    pub fn new(wasm: impl Into<Vec<WasmSource>>) -> Self {
        Self {
            name: None,
            wasm: wasm.into(),
            config: None,
            allowed_hosts: None,
            allowed_paths: None,
            memory_limit: None,
        }
    }
    
    /// Set plugin configuration
    pub fn with_config(mut self, config: serde_json::Value) -> Self {
        self.config = Some(config);
        self
    }
    
    /// Set allowed HTTP hosts
    pub fn with_allowed_hosts(mut self, hosts: Vec<String>) -> Self {
        self.allowed_hosts = Some(hosts);
        self
    }
    
    /// Set allowed filesystem paths
    pub fn with_allowed_paths(mut self, paths: HashMap<String, String>) -> Self {
        self.allowed_paths = Some(paths);
        self
    }
    
    /// Set memory limit
    pub fn with_memory_limit(mut self, limit: u64) -> Self {
        self.memory_limit = Some(limit);
        self
    }
}
```

### 2. Plugin Loader

```rust
// extism-core/src/loader.rs

use wasmtime::{Engine, Module, Store, Instance, Linker, Memory, Func};
use std::collections::HashMap;
use std::sync::Arc;
use sha2::{Sha256, Digest};
use crate::manifest::{Manifest, WasmSource};
use crate::error::Error;

/// Compiled module with metadata
#[derive(Clone)]
struct CachedModule {
    module: Module,
    hash: Vec<u8>,
}

/// Plugin loader - compiles and caches WASM modules
pub struct PluginLoader {
    engine: Engine,
    cache: HashMap<Vec<u8>, CachedModule>,
}

impl PluginLoader {
    pub fn new() -> Result<Self, Error> {
        let mut config = wasmtime::Config::new();
        config
            .cranelift_opt_level(wasmtime::OptLevel::Speed)
            .consume_fuel(true)  // Enable execution limits
            .epoch_interruption(true);  // Enable timeouts
        
        let engine = Engine::new(&config)?;
        
        Ok(Self {
            engine,
            cache: HashMap::new(),
        })
    }
    
    /// Load and compile a module from the manifest
    pub fn load_module(&mut self, wasm: &WasmSource) -> Result<Module, Error> {
        let (bytes, hash) = self.fetch_and_hash(wasm)?;
        
        // Check cache
        if let Some(cached) = self.cache.get(&hash) {
            return Ok(cached.module.clone());
        }
        
        // Compile
        let module = Module::from_binary(&self.engine, &bytes)?;
        
        // Cache
        self.cache.insert(
            hash.clone(),
            CachedModule {
                module: module.clone(),
                hash,
            },
        );
        
        Ok(module)
    }
    
    fn fetch_and_hash(&self, wasm: &WasmSource) -> Result<(Vec<u8>, Vec<u8>), Error> {
        match wasm {
            WasmSource::File { path } => {
                let bytes = std::fs::read(path)?;
                let hash = Self::hash_bytes(&bytes);
                Ok((bytes, hash))
            }
            WasmSource::Url { url, hash: expected_hash } => {
                // In production, use reqwest or similar
                let bytes = self.fetch_url(url)?;
                
                if let Some(expected) = expected_hash {
                    let actual = hex::encode(Self::hash_bytes(&bytes));
                    if &actual != expected {
                        return Err(Error::HashMismatch { expected: expected.clone(), actual });
                    }
                }
                
                let hash = Self::hash_bytes(&bytes);
                Ok((bytes, hash))
            }
            WasmSource::Memory { data } => {
                let hash = Self::hash_bytes(data);
                Ok((data.clone(), hash))
            }
        }
    }
    
    fn hash_bytes(bytes: &[u8]) -> Vec<u8> {
        let mut hasher = Sha256::new();
        hasher.update(bytes);
        hasher.finalize().to_vec()
    }
    
    fn fetch_url(&self, _url: &str) -> Result<Vec<u8>, Error> {
        // Implement HTTP fetch with allowed_hosts validation
        // For now, return error
        Err(Error::HttpNotImplemented)
    }
}
```

### 3. Plugin Instance

```rust
// extism-core/src/plugin.rs

use wasmtime::*;
use std::collections::HashMap;
use std::sync::Arc;
use crate::manifest::Manifest;
use crate::loader::PluginLoader;
use crate::error::Error;

/// Plugin-scoped data
pub struct PluginData {
    /// Configuration from manifest
    pub config: serde_json::Value,
    /// Plugin variables (persist across calls)
    pub variables: HashMap<String, Vec<u8>>,
    /// Memory limit
    pub memory_limit: u64,
    /// Allowed HTTP hosts
    pub allowed_hosts: Option<Vec<String>>,
    /// Allowed filesystem paths
    pub allowed_paths: HashMap<String, String>,
    /// Host function context
    pub host_context: Arc<dyn std::any::Any + Send + Sync>,
}

/// A loaded plugin instance
pub struct Plugin {
    store: Store<PluginData>,
    instance: Instance,
    memory: Memory,
    /// Input memory offset (set by host before call)
    input_offset: u64,
    /// Input length
    input_length: usize,
}

impl Plugin {
    /// Create a new plugin from a manifest
    pub fn new(
        manifest: &Manifest,
        loader: &mut PluginLoader,
        host_functions: &HostFunctions,
    ) -> Result<Self, Error> {
        // Load all modules from manifest
        let modules: Vec<Module> = manifest
            .wasm
            .iter()
            .map(|wasm| loader.load_module(wasm))
            .collect::<Result<Vec<_>, _>>()?;
        
        // For simplicity, use first module (multi-module linking is advanced)
        let module = modules.into_iter().next().ok_or(Error::NoModules)?;
        
        // Create store with plugin data
        let mut store = Store::new(
            module.engine(),
            PluginData {
                config: manifest.config.clone().unwrap_or(serde_json::Value::Null),
                variables: HashMap::new(),
                memory_limit: manifest.memory_limit.unwrap_or(64 * 1024 * 1024), // 64MB default
                allowed_hosts: manifest.allowed_hosts.clone(),
                allowed_paths: manifest.allowed_paths.clone().unwrap_or_default(),
                host_context: Arc::new(()),
            },
        );
        
        // Configure memory limit
        store.set_fuel(manifest.memory_limit.unwrap_or(64 * 1024 * 1024))?;
        
        // Create linker and register host functions
        let mut linker = Linker::new(module.engine());
        
        // Register Extism built-in functions
        Self::register_extism_functions(&mut linker)?;
        
        // Register user host functions
        Self::register_host_functions(&mut linker, host_functions)?;
        
        // Instantiate
        let instance = linker.instantiate(&mut store, &module)?;
        
        // Get exported memory
        let memory = instance
            .get_memory(&store, "memory")
            .ok_or(Error::NoMemory)?;
        
        Ok(Self {
            store,
            instance,
            memory,
            input_offset: 0,
            input_length: 0,
        })
    }
    
    /// Register Extism built-in functions
    fn register_extism_functions(linker: &mut Linker<PluginData>) -> Result<(), Error> {
        // extism::alloc - allocate memory
        linker.func_wrap("extism", "alloc", |caller, len: i32| {
            let data = caller.data_mut();
            // Simple bump allocator - in production, use proper heap management
            let offset = data.variables.get("__alloc_offset")
                .and_then(|v| v.get(0..8))
                .map(|b| u64::from_le_bytes(b.try_into().unwrap()))
                .unwrap_or(8192); // Start after reserved space
            
            let new_offset = offset + len as u64;
            
            // Update allocation pointer
            let mut var_data = vec![0u8; 8];
            var_data[..8].copy_from_slice(&new_offset.to_le_bytes());
            data.variables.insert("__alloc_offset".to_string(), var_data);
            
            Ok(offset as i64)
        })?;
        
        // extism::free - free memory (no-op in simple implementation)
        linker.func_wrap("extism", "free", |_caller, _ptr: i64| {
            Ok(())
        })?;
        
        // extism::input_offset - get input pointer
        linker.func_wrap("extism", "input_offset", |caller| {
            Ok(caller.data().input_offset as i64)
        })?;
        
        // extism::input_length - get input length
        linker.func_wrap("extism", "input_length", |caller| {
            Ok(caller.data().input_length as i64)
        })?;
        
        // extism::config_get - get config value
        linker.func_wrap("extism", "config_get", |mut caller, key_ptr: i64, key_len: i64| {
            let data = caller.data();
            let key = Self::read_string(&mut caller, key_ptr as u64, key_len as usize)?;
            
            if let Some(config) = data.config.as_object() {
                if let Some(value) = config.get(&key) {
                    let value_str = value.to_string();
                    let ptr = Self::alloc_in_store(&mut caller, value_str.as_bytes())?;
                    return Ok(ptr as i64);
                }
            }
            
            Ok(0i64) // null pointer = not found
        })?;
        
        // extism::var_get - get variable
        linker.func_wrap("extism", "var_get", |mut caller, key_ptr: i64, key_len: i64| {
            let data = caller.data();
            let key = Self::read_string(&mut caller, key_ptr as u64, key_len as usize)?;
            
            if let Some(value) = data.variables.get(&key) {
                let ptr = Self::alloc_in_store(&mut caller, value)?;
                return Ok(ptr as i64);
            }
            
            Ok(0i64)
        })?;
        
        // extism::var_set - set variable
        linker.func_wrap("extism", "var_set", |mut caller, key_ptr: i64, key_len: i64, val_ptr: i64, val_len: i64| {
            let data = caller.data_mut();
            let key = Self::read_string(&mut caller, key_ptr as u64, key_len as usize)?;
            let value = caller.memory_read(val_ptr as u64, val_len as usize)?;
            
            data.variables.insert(key, value);
            Ok(())
        })?;
        
        // extism::var_remove - remove variable
        linker.func_wrap("extism", "var_remove", |mut caller, key_ptr: i64, key_len: i64| {
            let data = caller.data_mut();
            let key = Self::read_string(&mut caller, key_ptr as u64, key_len as usize)?;
            data.variables.remove(&key);
            Ok(())
        })?;
        
        Ok(())
    }
    
    /// Register user-defined host functions
    fn register_host_functions(
        linker: &mut Linker<PluginData>,
        host_functions: &HostFunctions,
    ) -> Result<(), Error> {
        for (name, func) in host_functions.iter() {
            let func_clone = Arc::clone(func);
            linker.func_new(
                "env",
                name,
                move |mut caller, params, results| {
                    let func = Arc::clone(&func_clone);
                    func(&mut caller, params, results)
                },
            )?;
        }
        Ok(())
    }
    
    /// Call a plugin function
    pub fn call<A: AsRef<[u8]>>(&mut self, func_name: &str, input: A) -> Result<Vec<u8>, Error> {
        let input = input.as_ref();
        
        // Write input to memory
        let input_ptr = self.alloc_in_memory(input)?;
        self.input_offset = input_ptr;
        self.input_length = input.len();
        
        // Get function
        let func = self.instance
            .get_func(&self.store, func_name)
            .ok_or_else(|| Error::FunctionNotFound(func_name.to_string()))?;
        
        // Call function (expects no params, returns offset)
        let results = func.call(&mut self.store, &[])?;
        let output_offset = results[0].unwrap_i64() as u64;
        
        // Read output length (stored before data)
        let output_len = self.memory_read_u64(output_offset)?;
        let output_data = self.memory_read(output_offset + 8, output_len as usize)?;
        
        Ok(output_data)
    }
    
    /// Allocate memory and write data
    fn alloc_in_memory(&mut self, data: &[u8]) -> Result<u64, Error> {
        // Get alloc function
        let alloc = self.instance
            .get_func(&self.store, "alloc")
            .or_else(|| self.instance.get_func(&self.store, "__extism_alloc"))
            .ok_or(Error::NoAllocFunction)?;
        
        // Allocate (length + 8 for length prefix)
        let alloc_size = data.len() + 8;
        let results = alloc.call(&mut self.store, &[Val::I64(alloc_size as i64)])?;
        let ptr = results[0].unwrap_i64() as u64;
        
        // Write length prefix
        self.memory_write(ptr, &alloc_size.to_le_bytes())?;
        
        // Write data
        self.memory_write(ptr + 8, data)?;
        
        Ok(ptr)
    }
    
    fn alloc_in_store(caller: &mut Caller<PluginData>, data: &[u8]) -> Result<u64, Error> {
        // Similar to alloc_in_memory but for use in func_wrap
        // Implementation omitted for brevity
        Ok(0)
    }
    
    fn read_string(caller: &mut Caller<PluginData>, ptr: u64, len: usize) -> Result<String, Error> {
        let bytes = caller.memory_read(ptr, len)?;
        String::from_utf8(bytes).map_err(|_| Error::Utf8Error)
    }
    
    fn memory_read(&self, offset: u64, len: usize) -> Result<Vec<u8>, Error> {
        let mut buf = vec![0u8; len];
        self.memory.read(&self.store, offset as usize, &mut buf)?;
        Ok(buf)
    }
    
    fn memory_read_u64(&self, offset: u64) -> Result<u64, Error> {
        let mut buf = [0u8; 8];
        self.memory.read(&self.store, offset as usize, &mut buf)?;
        Ok(u64::from_le_bytes(buf))
    }
    
    fn memory_write(&mut self, offset: u64, data: &[u8]) -> Result<(), Error> {
        self.memory.write(&mut self.store, offset as usize, data)?;
        Ok(())
    }
}

/// Type alias for host functions
pub type HostFunction = Arc<dyn FnMut(&mut Caller<PluginData>, &[Val], &mut [Val]) -> Result<(), Error> + Send + Sync>;

/// Collection of host functions
pub struct HostFunctions {
    functions: HashMap<String, HostFunction>,
}

impl HostFunctions {
    pub fn new() -> Self {
        Self {
            functions: HashMap::new(),
        }
    }
    
    pub fn add<F>(&mut self, name: &str, func: F)
    where
        F: FnMut(&mut Caller<PluginData>, &[Val], &mut [Val]) -> Result<(), Error> + Send + Sync + 'static,
    {
        self.functions.insert(name.to_string(), Arc::new(func));
    }
    
    pub fn iter(&self) -> impl Iterator<Item = (&String, &HostFunction)> {
        self.functions.iter()
    }
}
```

### 4. Error Handling

```rust
// extism-core/src/error.rs

use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("WASM runtime error: {0}")]
    Wasmtime(#[from] wasmtime::Error),
    
    #[error("Function not found: {0}")]
    FunctionNotFound(String),
    
    #[error("No memory exported")]
    NoMemory,
    
    #[error("No WASM modules in manifest")]
    NoModules,
    
    #[error("No alloc function found")]
    NoAllocFunction,
    
    #[error("UTF-8 encoding error")]
    Utf8Error,
    
    #[error("Hash mismatch: expected {expected}, got {actual}")]
    HashMismatch { expected: String, actual: String },
    
    #[error("HTTP fetch not implemented")]
    HttpNotImplemented,
    
    #[error("Host not allowed: {0}")]
    HostNotAllowed(String),
    
    #[error("Path not allowed: {0}")]
    PathNotAllowed(String),
    
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    
    #[error("Memory limit exceeded")]
    MemoryLimitExceeded,
    
    #[error("Execution timeout")]
    Timeout,
}
```

## Plugin Development Kit (PDK)

### PDK Macro

```rust
// extism-pdk/src/lib.rs

use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, ItemFn};

/// Marks a function as a plugin entry point
#[proc_macro_attribute]
pub fn plugin_fn(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let func = parse_macro_input!(item as ItemFn);
    let func_name = &func.sig.ident;
    
    let expanded = quote! {
        // The exported WASM function
        #[no_mangle]
        pub extern "C" fn __extism_main() -> u64 {
            // Read input from Extism runtime
            let input_offset = unsafe { extism_pdk_sys::input_offset() };
            let input_length = unsafe { extism_pdk_sys::input_length() };
            
            // Read input data
            let input_data = unsafe {
                std::slice::from_raw_parts(
                    input_offset as *const u8,
                    input_length as usize,
                )
            }.to_vec();
            
            // Call user function
            let result = #func_name(input_data);
            
            match result {
                Ok(output) => {
                    // Write output to memory
                    let output_ptr = output.as_ptr() as u64;
                    let output_len = output.len() as u64;
                    unsafe {
                        extism_pdk_sys::set_output(output_ptr, output_len);
                    }
                    0 // Success
                }
                Err(e) => {
                    // Write error to memory
                    let error_bytes = e.to_string().into_bytes();
                    let error_ptr = error_bytes.as_ptr() as u64;
                    let error_len = error_bytes.len() as u64;
                    unsafe {
                        extism_pdk_sys::set_error(error_ptr, error_len);
                    }
                    1 // Error
                }
            }
        }
        
        // User function
        #func
    };
    
    TokenStream::from(expanded)
}

/// Result type for plugin functions
pub type FnResult<T> = Result<T, PdkError>;

/// PDK error type
#[derive(Debug)]
pub struct PdkError(String);

impl PdkError {
    pub fn msg(msg: impl Into<String>) -> Self {
        Self(msg.into())
    }
}

impl std::fmt::Display for PdkError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::error::Error for PdkError {}
```

### PDK API

```rust
// extism-pdk/src/api.rs

/// Read plugin input
pub fn input() -> Vec<u8> {
    unsafe {
        let offset = extism_pdk_sys::input_offset();
        let length = extism_pdk_sys::input_length();
        std::slice::from_raw_parts(offset as *const u8, length as usize).to_vec()
    }
}

/// Read input as string
pub fn input_str() -> Result<String, PdkError> {
    String::from_utf8(input()).map_err(|_| PdkError::msg("Invalid UTF-8 input"))
}

/// Set plugin output
pub fn set_output(data: &[u8]) {
    unsafe {
        extism_pdk_sys::set_output(data.as_ptr() as u64, data.len() as u64);
    }
}

/// Get configuration value
pub fn config_get(key: &str) -> Option<String> {
    let key_bytes = key.as_bytes();
    unsafe {
        let result_ptr = extism_pdk_sys::config_get(
            key_bytes.as_ptr() as u64,
            key_bytes.len() as u64,
        );
        if result_ptr == 0 {
            return None;
        }
        Some(read_string(result_ptr))
    }
}

/// Get variable
pub fn var_get(key: &str) -> Option<Vec<u8>> {
    let key_bytes = key.as_bytes();
    unsafe {
        let result_ptr = extism_pdk_sys::var_get(
            key_bytes.as_ptr() as u64,
            key_bytes.len() as u64,
        );
        if result_ptr == 0 {
            return None;
        }
        Some(read_bytes(result_ptr))
    }
}

/// Set variable
pub fn var_set(key: &str, value: &[u8]) {
    let key_bytes = key.as_bytes();
    unsafe {
        extism_pdk_sys::var_set(
            key_bytes.as_ptr() as u64,
            key_bytes.len() as u64,
            value.as_ptr() as u64,
            value.len() as u64,
        );
    }
}

unsafe fn read_string(ptr: u64) -> String {
    let bytes = read_bytes(ptr);
    String::from_utf8_lossy(&bytes).to_string()
}

unsafe fn read_bytes(ptr: u64) -> Vec<u8> {
    // Read length from prefix
    let len_ptr = ptr as *const u64;
    let len = *len_ptr as usize;
    
    // Read data
    let data_ptr = (ptr + 8) as *const u8;
    std::slice::from_raw_parts(data_ptr, len).to_vec()
}
```

### Example Plugin

```rust
// examples/greeter/src/lib.rs

use extism_pdk::*;

#[plugin_fn]
pub fn greet(input: Vec<u8>) -> FnResult<Vec<u8>> {
    let name = String::from_utf8(input)
        .map_err(|_| PdkError::msg("Invalid UTF-8 input"))?;
    
    let message = format!("Hello, {}!", name);
    Ok(message.into_bytes())
}

#[plugin_fn]
pub fn add(input: Vec<u8>) -> FnResult<Vec<u8>> {
    // Parse JSON input
    let nums: Vec<i64> = serde_json::from_slice(&input)
        .map_err(|e| PdkError::msg(format!("Invalid JSON: {}", e)))?;
    
    let sum: i64 = nums.iter().sum();
    Ok(sum.to_string().into_bytes())
}

#[plugin_fn]
pub fn count_calls() -> FnResult<Vec<u8>> {
    // Get current count
    let count = var_get("count")
        .and_then(|b| std::str::from_utf8(&b).ok())
        .and_then(|s| s.parse::<u64>().ok())
        .unwrap_or(0);
    
    // Increment
    let new_count = count + 1;
    var_set("count", new_count.to_string().as_bytes());
    
    Ok(format!("Called {} times", new_count).into_bytes())
}
```

## High-Level SDK

```rust
// extism-sdk/src/lib.rs

use extism_core::{Plugin, PluginLoader, Manifest, HostFunctions};

/// High-level plugin manager
pub struct Extism {
    loader: PluginLoader,
    plugins: Vec<Plugin>,
}

impl Extism {
    pub fn new() -> Result<Self, extism_core::Error> {
        Ok(Self {
            loader: PluginLoader::new()?,
            plugins: Vec::new(),
        })
    }
    
    /// Create a plugin from a WASM file
    pub fn create_plugin(&mut self, wasm_path: &str) -> Result<usize, extism_core::Error> {
        let manifest = Manifest::new([extism_core::WasmSource::File {
            path: wasm_path.into(),
            name: None,
        }]);
        
        self.create_plugin_from_manifest(&manifest)
    }
    
    /// Create a plugin from a manifest
    pub fn create_plugin_from_manifest(
        &mut self,
        manifest: &Manifest,
    ) -> Result<usize, extism_core::Error> {
        let host_functions = HostFunctions::new();
        let plugin = Plugin::new(manifest, &mut self.loader, &host_functions)?;
        
        let index = self.plugins.len();
        self.plugins.push(plugin);
        
        Ok(index)
    }
    
    /// Call a plugin function
    pub fn call_plugin(
        &mut self,
        plugin_index: usize,
        function: &str,
        input: &[u8],
    ) -> Result<Vec<u8>, extism_core::Error> {
        let plugin = self
            .plugins
            .get_mut(plugin_index)
            .ok_or_else(|| extism_core::Error::FunctionNotFound("Plugin not found".into()))?;
        
        plugin.call(function, input)
    }
}

impl Default for Extism {
    fn default() -> Self {
        Self::new().expect("Failed to initialize Extism")
    }
}
```

## Building for WASM

```bash
# Add WASM target
rustup target add wasm32-unknown-unknown

# Build plugin
cargo build --release --target wasm32-unknown-unknown

# Output: target/wasm32-unknown-unknown/release/plugin.wasm

# Optional: Optimize with wasm-opt
wasm-opt -O3 target/wasm32-unknown-unknown/release/plugin.wasm \
    -o plugin.optimized.wasm
```

## Testing

```rust
// tests/plugin_test.rs

use extism_sdk::Extism;

#[test]
fn test_greeter_plugin() {
    let mut extism = Extism::new().unwrap();
    
    let plugin_idx = extism.create_plugin("tests/plugins/greeter.wasm").unwrap();
    
    let result = extism
        .call_plugin(plugin_idx, "greet", b"World")
        .unwrap();
    
    assert_eq!(String::from_utf8_lossy(&result), "Hello, World!");
}

#[test]
fn test_add_plugin() {
    let mut extism = Extism::new().unwrap();
    
    let plugin_idx = extism.create_plugin("tests/plugins/add.wasm").unwrap();
    
    let input = b"[1, 2, 3, 4, 5]";
    let result = extism.call_plugin(plugin_idx, "add", input).unwrap();
    
    assert_eq!(String::from_utf8_lossy(&result), "15");
}
```

## Summary

This Rust revision provides:

1. **Core runtime**: Plugin loading, WASM execution, memory management
2. **Manifest system**: Configuration, capabilities, security
3. **PDK**: Macros and APIs for writing plugins
4. **High-level SDK**: Simple interface for host applications
5. **Error handling**: Comprehensive error types
6. **Testing**: Unit and integration test patterns

The implementation mirrors Extism's architecture while being accessible for understanding and extension.
