---
name: napi
description: Core napi-rs framework crate providing high-level Rust bindings for Node-API with type-safe JavaScript interoperability
type: sub-project
source: /home/darkvoid/Boxxed/@formulas/src.rust/src.wasm/src.napi-rs/napi/
---

# napi - Core Framework Crate

## Overview

The `napi` crate is the heart of the napi-rs ecosystem, providing high-level, ergonomic Rust bindings for Node-API. It abstracts away the low-level FFI complexity while maintaining zero-cost abstractions for building native Node.js add-ons.

## Package Structure

```
napi/
├── src/
│   ├── lib.rs                    # Main entry point, exports, prelude
│   ├── bindgen_prelude.rs        # Types and traits for proc-macro generated code
│   ├── env.rs                    # JavaScript environment (Env wrapper)
│   ├── value.rs                  # Core JavaScript value types
│   ├── string.rs                 # JsString implementation
│   ├── number.rs                 # JsNumber implementation
│   ├── boolean.rs                # JsBoolean implementation
│   ├── object.rs                 # JsObject operations
│   ├── array.rs                  # JsArray operations
│   ├── arraybuffer.rs            # ArrayBuffer and views
│   ├── buffer.rs                 # Node.js Buffer
│   ├── function.rs               # JsFunction and callbacks
│   ├── global.rs                 # Global objects (Object, Array, etc.)
│   ├── class.rs                  # JavaScript class bindings
│   ├── constructor.rs            # Constructor handling
│   ├── async.rs                  # AsyncTask and Promise support
│   ├── threadsafe_function.rs    # Thread-safe function calls
│   ├── error.rs                  # Error types and Result
│   ├── status.rs                 # napi_status codes
│   ├── type_tag.rs               # Type tagging for instanceof
│   ├── instance.rs               # Instance data management
│   ├── cleanup_env.rs            # Environment cleanup hooks
│   ├── ref_.rs                   # Persistent references
│   └── sys.rs                    # Re-export napi-sys
│
├── Cargo.toml
└── README.md
```

## Core abstractions

### ToNapiValue and FromNapiValue Traits

```rust
// crates/napi/src/bindgen_prelude.rs

/// Convert Rust types to JavaScript values
pub trait ToNapiValue {
    unsafe fn to_napi_value(env: sys::napi_env, val: Self) -> Result<sys::napi_value>;
}

/// Convert JavaScript values to Rust types
pub trait FromNapiValue: Sized {
    unsafe fn from_napi_value(env: sys::napi_env, napi_val: sys::napi_value) -> Result<Self>;
}

/// Validate JavaScript values before conversion
pub trait ValidateNapiValue: FromNapiValue {
    unsafe fn validate(env: sys::napi_env, napi_val: sys::napi_value) -> Result<sys::napi_value>;
}
```

### Implementation Examples

```rust
// String conversion
impl ToNapiValue for &str {
    unsafe fn to_napi_value(env: sys::napi_env, val: &str) -> Result<sys::napi_value> {
        let mut ptr = std::ptr::null_mut();
        check_status!(
            sys::napi_create_string_utf8(env, val.as_ptr() as *const _, val.len(), &mut ptr),
            "Failed to create JavaScript string"
        )?;
        Ok(ptr)
    }
}

impl FromNapiValue for String {
    unsafe fn from_napi_value(env: sys::napi_env, napi_val: sys::napi_value) -> Result<Self> {
        let mut len = 0;
        check_status!(
            sys::napi_get_value_string_utf8(env, napi_val, std::ptr::null_mut(), 0, &mut len),
            "Failed to get string length"
        )?;

        let mut result = String::with_capacity(len);
        check_status!(
            sys::napi_get_value_string_utf8(env, napi_val, result.as_mut_ptr() as _, len + 1, &mut len),
            "Failed to convert JavaScript string to Rust string"
        )?;

        result.set_len(len);
        Ok(result)
    }
}

// Number conversion
impl ToNapiValue for u32 {
    unsafe fn to_napi_value(env: sys::napi_env, val: u32) -> Result<sys::napi_value> {
        let mut ptr = std::ptr::null_mut();
        check_status!(
            sys::napi_create_uint32(env, val, &mut ptr),
            "Failed to create JavaScript number"
        )?;
        Ok(ptr)
    }
}

impl FromNapiValue for f64 {
    unsafe fn from_napi_value(env: sys::napi_env, napi_val: sys::napi_value) -> Result<Self> {
        let mut result: f64 = 0.0;
        check_status!(
            sys::napi_get_value_double(env, napi_val, &mut result),
            "Failed to convert JavaScript number to Rust f64"
        )?;
        Ok(result)
    }
}
```

## Environment (Env)

```rust
// crates/napi/src/env.rs

/// Represents the JavaScript environment
/// Provides context for all JavaScript operations
pub struct Env {
    pub(crate) inner: sys::napi_env,
}

impl Env {
    /// Create a string from &str
    pub fn create_string(&self, s: &str) -> Result<JsString> {
        let mut ptr = std::ptr::null_mut();
        check_status!(
            sys::napi_create_string_utf8(
                self.inner,
                s.as_ptr() as *const _,
                s.len(),
                &mut ptr
            ),
            "Failed to create JavaScript string"
        )?;
        Ok(JsString::from_raw_unchecked(self.inner, ptr))
    }

    /// Create a JavaScript object
    pub fn create_object(&self) -> Result<JsObject> {
        let mut ptr = std::ptr::null_mut();
        check_status!(
            sys::napi_create_object(self.inner, &mut ptr),
            "Failed to create JavaScript object"
        )?;
        Ok(JsObject::from_raw_unchecked(self.inner, ptr))
    }

    /// Create a JavaScript array with length
    pub fn create_array(&self, length: u32) -> Result<JsArray> {
        let mut ptr = std::ptr::null_mut();
        check_status!(
            sys::napi_create_array_with_length(self.inner, length as _, &mut ptr),
            "Failed to create JavaScript array"
        )?;
        Ok(JsArray::from_raw_unchecked(self.inner, ptr))
    }

    /// Create a ArrayBuffer
    pub fn create_arraybuffer(&self, length: usize) -> Result<JsArrayBuffer> {
        let mut ptr = std::ptr::null_mut();
        let mut data_ptr = std::ptr::null_mut();
        check_status!(
            sys::napi_create_arraybuffer(self.inner, length as _, &mut data_ptr, &mut ptr),
            "Failed to create ArrayBuffer"
        )?;
        Ok(JsArrayBuffer::from_raw_unchecked(self.inner, ptr, data_ptr, length))
    }

    /// Create a Buffer (Node.js specific)
    pub fn create_buffer(&self, data: Vec<u8>) -> Result<JsBuffer> {
        let mut ptr = std::ptr::null_mut();
        let mut data_ptr = std::ptr::null_mut();
        let len = data.len();

        // Create external buffer that Rust owns
        check_status!(
            sys::napi_create_external_buffer(
                self.inner,
                len as _,
                Box::into_raw(data.into_boxed_slice()) as *mut _,
                Some(finalize_buffer),
                std::ptr::null_mut(),
                &mut ptr,
                &mut data_ptr
            ),
            "Failed to create external buffer"
        )?;
        Ok(JsBuffer::from_raw_unchecked(self.inner, ptr, data_ptr, len))
    }

    /// Get the global object
    pub fn get_global(&self) -> Result<JsObject> {
        let mut ptr = std::ptr::null_mut();
        check_status!(
            sys::napi_get_global(self.inner, &mut ptr),
            "Failed to get global object"
        )?;
        Ok(JsObject::from_raw_unchecked(self.inner, ptr))
    }

    /// Get undefined value
    pub fn get_undefined(&self) -> Result<JsUndefined> {
        let mut ptr = std::ptr::null_mut();
        check_status!(
            sys::napi_get_undefined(self.inner, &mut ptr),
            "Failed to get undefined value"
        )?;
        Ok(JsUndefined::from_raw_unchecked(self.inner, ptr))
    }

    /// Create a Function from Rust closure
    pub fn create_function<N, V, F>(&self, name: N, callback: F) -> Result<JsFunction>
    where
        N: AsRef<str>,
        F: Fn(FunctionCallContext) -> Result<V> + 'static,
        V: ToNapiValue,
    {
        // ... implementation creates threadsafe function
    }

    /// Create a Promise
    pub fn create_promise<T>(&self) -> Result<(JsDeferred, JsObject)>
    where
        T: ToNapiValue,
    {
        JsDeferred::new(self)
    }

    /// Run JavaScript code in a script
    pub fn run_script<S: AsRef<str>>(&self, code: S, filename: Option<&str>) -> Result<JsValue> {
        let mut result = std::ptr::null_mut();
        let filename = filename.unwrap_or("anonymous");

        check_status!(
            sys::napi_run_script(self.inner, code.as_ref(), filename, &mut result),
            "Failed to run script"
        )?;
        Ok(JsValue::from_raw_unchecked(self.inner, result))
    }
}
```

## JavaScript Value Types

### JsString

```rust
pub struct JsString {
    pub(crate) value: sys::napi_value,
}

impl JsString {
    pub fn utf8_len(&self) -> Result<usize> {
        let mut len = 0;
        check_status!(
            sys::napi_get_value_string_utf8(self.value, std::ptr::null_mut(), 0, &mut len),
            "Failed to get string length"
        )?;
        Ok(len)
    }

    pub fn into_utf8(self) -> Result<String> {
        let len = self.utf8_len()?;
        let mut result = String::with_capacity(len);
        check_status!(
            sys::napi_get_value_string_utf8(self.value, result.as_mut_ptr() as _, len + 1, &mut len),
            "Failed to convert to UTF8 string"
        )?;
        result.set_len(len);
        Ok(result)
    }
}
```

### JsObject

```rust
pub struct JsObject {
    pub(crate) value: sys::napi_value,
}

impl JsObject {
    /// Set a property on the object
    pub fn set<K, V>(&mut self, key: K, value: V) -> Result<()>
    where
        K: ToNapiValue,
        V: ToNapiValue,
    {
        let key_val = Key::to_napi_value(self.env, key)?;
        let value_val = Value::to_napi_value(self.env, value)?;

        check_status!(
            sys::napi_set_property(self.env, self.value, key_val, value_val),
            "Failed to set property on object"
        )
    }

    /// Get a property from the object
    pub fn get<K, V>(&self, key: K) -> Result<V>
    where
        K: ToNapiValue,
        V: FromNapiValue,
    {
        let key_val = Key::to_napi_value(self.env, key)?;
        let mut value_val = std::ptr::null_mut();

        check_status!(
            sys::napi_get_property(self.env, self.value, key_val, &mut value_val),
            "Failed to get property from object"
        )?;

        unsafe { V::from_napi_value(self.env, value_val) }
    }

    /// Check if object has a property
    pub fn has<K>(&self, key: K) -> Result<bool>
    where
        K: ToNapiValue,
    {
        let key_val = Key::to_napi_value(self.env, key)?;
        let mut result = false;

        check_status!(
            sys::napi_has_property(self.env, self.value, key_val, &mut result),
            "Failed to check if object has property"
        )?;

        Ok(result)
    }

    /// Delete a property
    pub fn delete<K>(&mut self, key: K) -> Result<bool>
    where
        K: ToNapiValue,
    {
        let key_val = Key::to_napi_value(self.env, key)?;
        let mut result = false;

        check_status!(
            sys::napi_delete_property(self.env, self.value, key_val, &mut result),
            "Failed to delete property"
        )?;

        Ok(result)
    }

    /// Get all property names
    pub fn get_property_names(&self) -> Result<JsArray> {
        let mut ptr = std::ptr::null_mut();
        check_status!(
            sys::napi_get_property_names(self.env, self.value, &mut ptr),
            "Failed to get property names"
        )?;
        Ok(JsArray::from_raw_unchecked(self.env, ptr))
    }
}
```

### JsArray

```rust
pub struct JsArray {
    pub(crate) value: sys::napi_value,
}

impl JsArray {
    pub fn len(&self) -> Result<u32> {
        let mut len: u32 = 0;
        check_status!(
            sys::napi_get_array_length(self.env, self.value, &mut len),
            "Failed to get array length"
        )?;
        Ok(len)
    }

    pub fn is_empty(&self) -> Result<bool> {
        Ok(self.len()? == 0)
    }

    pub fn get<V>(&self, index: u32) -> Result<V>
    where
        V: FromNapiValue,
    {
        let mut value = std::ptr::null_mut();
        check_status!(
            sys::napi_get_element(self.env, self.value, index, &mut value),
            "Failed to get array element"
        )?;
        unsafe { V::from_napi_value(self.env, value) }
    }

    pub fn set<V>(&mut self, index: u32, value: V) -> Result<()>
    where
        V: ToNapiValue,
    {
        let value_val = V::to_napi_value(self.env, value)?;
        check_status!(
            sys::napi_set_element(self.env, self.value, index, value_val),
            "Failed to set array element"
        )
    }

    /// Convert to Rust Vec
    pub fn to_vec<T>(&self) -> Result<Vec<T>>
    where
        T: FromNapiValue,
    {
        let len = self.len()?;
        let mut vec = Vec::with_capacity(len as usize);

        for i in 0..len {
            let item = self.get::<T>(i)?;
            vec.push(item);
        }

        Ok(vec)
    }
}
```

## Async Patterns

### AsyncTask

```rust
// crates/napi/src/async.rs

pub trait Task {
    type Output: Send + Sync + 'static;
    type JsValue: ToNapiValue;

    /// Compute runs on a background thread pool
    fn compute(&mut self) -> Result<Self::Output>;

    /// Resolve runs on the main thread after compute completes
    fn resolve(&mut self, env: Env, output: Self::Output) -> Result<Self::JsValue>;
}

pub struct AsyncTask<T: Task> {
    task: T,
}

impl<T: Task> AsyncTask<T> {
    pub fn new(task: T) -> Self {
        Self { task }
    }
}

impl<T: Task> ToNapiValue for AsyncTask<T> {
    unsafe fn to_napi_value(env: sys::napi_env, val: Self) -> Result<sys::napi_value> {
        // Create async work and schedule on thread pool
        let deferred = Box::new(AsyncTaskDeferred::new(env, val.task));
        deferred.schedule();
        Ok(deferred.promise())
    }
}

// Usage example:
#[napi]
pub fn compute_hash(input: String) -> AsyncTask<HashTask> {
    AsyncTask::new(HashTask { input })
}

pub struct HashTask {
    input: String,
}

impl Task for HashTask {
    type Output = String;
    type JsValue = String;

    fn compute(&mut self) -> Result<Self::Output> {
        // Heavy computation on background thread
        Ok(hex::encode(sha256::digest(&self.input)))
    }

    fn resolve(&mut self, _env: Env, output: Self::Output) -> Result<Self::JsValue> {
        // Convert result for JavaScript (runs on main thread)
        Ok(output)
    }
}
```

### JsDeferred (Promise)

```rust
pub struct JsDeferred<T> {
    // Internal state
}

impl<T: ToNapiValue> JsDeferred<T> {
    pub fn new(env: &Env) -> Result<(Self, JsObject)> {
        let mut deferred_ptr = std::ptr::null_mut();
        let mut promise_ptr = std::ptr::null_mut();

        check_status!(
            sys::napi_create_promise(env, &mut deferred_ptr, &mut promise_ptr),
            "Failed to create promise"
        )?;

        let deferred = JsDeferred::from_raw(env, deferred_ptr);
        Ok((deferred, JsObject::from_raw(env, promise_ptr)))
    }

    pub fn resolve(self, value: T) -> Result<()> {
        let value_val = T::to_napi_value(self.env, value)?;
        check_status!(
            sys::napi_resolve_deferred(self.env, self.deferred, value_val),
            "Failed to resolve promise"
        )
    }

    pub fn reject(self, reason: Error) -> Result<()> {
        let error_val = reason.into_value(self.env)?;
        check_status!(
            sys::napi_reject_deferred(self.env, self.deferred, error_val),
            "Failed to reject promise"
        )
    }
}

// Usage:
#[napi]
pub fn async_fetch(url: String) -> Result<JsObject> {
    let (deferred, promise) = JsDeferred::<String>::new(&env)?;

    // Spawn async task
    tokio::spawn(async move {
        match fetch(&url).await {
            Ok(data) => deferred.resolve(data),
            Err(e) => deferred.reject(e),
        }
    });

    Ok(promise)
}
```

## Thread-safe Functions

```rust
// crates/napi/src/threadsafe_function.rs

pub struct ThreadsafeFunction<T, CallMode> {
    tsfn: sys::napi_threadsafe_function,
    _phantom: PhantomData<T>,
}

impl<T, CallMode> ThreadsafeFunction<T, CallMode>
where
    T: Send + 'static,
{
    /// Create a threadsafe function from a JavaScript callback
    pub fn create<F>(
        env: sys::napi_env,
        func: sys::napi_value,
        max_queue_size: usize,
        converter: F,
    ) -> Result<Self>
    where
        F: Fn(ThreadSafeCallContext<T>) -> Result<Vec<sys::napi_value>> + Send + 'static,
    {
        let mut tsfn = std::ptr::null_mut();

        check_status!(
            sys::napi_create_threadsafe_function(
                env,
                func,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                max_queue_size,
                1,  // initial thread count
                Some(finalize_js_function),
                Box::into_raw(Box::new(converter)) as *mut _,
                &mut tsfn
            ),
            "Failed to create threadsafe function"
        )?;

        Ok(ThreadsafeFunction {
            tsfn,
            _phantom: PhantomData,
        })
    }

    /// Call the JavaScript function from any thread
    pub fn call(&self, value: T, mode: ThreadsafeFunctionCallMode) -> Result<()> {
        check_status!(
            sys::napi_call_threadsafe_function(
                self.tsfn,
                Box::into_raw(Box::new(value)) as *mut _,
                match mode {
                    ThreadsafeFunctionCallMode::Blocking => sys::ThreadsafeFunctionCallMode::tsfn_call_mode_blocking,
                    ThreadsafeFunctionCallMode::NonBlocking => sys::ThreadsafeFunctionCallMode::tsfn_call_mode_nonblocking,
                }
            ),
            "Failed to call threadsafe function"
        )
    }

    /// Acquire a reference to prevent cleanup
    pub fn acquire(&mut self) -> Result<()> {
        check_status!(
            sys::napi_acquire_threadsafe_function(self.tsfn),
            "Failed to acquire threadsafe function"
        )
    }

    /// Release a reference
    pub fn release(self) -> Result<()> {
        check_status!(
            sys::napi_release_threadsafe_function(self.tsfn, sys::ThreadsafeFunctionReleaseMode::tsfn_release_mode_fulfilled),
            "Failed to release threadsafe function"
        )
    }
}

// Usage example:
#[napi]
pub fn start_watching<T: Fn(String) -> Result<()>>(callback: JsFunction) -> Result<Watcher> {
    let tsfn = callback.create_threadsafe_function(
        0,  // max queue size
        |ctx: ThreadSafeCallContext<String>| {
            // Convert Rust String to JavaScript string
            Ok(vec![ctx.env.create_string(&ctx.value)?])
        },
    )?;

    // Spawn native thread that will call the JS callback
    std::thread::spawn(move || {
        loop {
            let event = wait_for_event();
            tsfn.call(event, ThreadsafeFunctionCallMode::NonBlocking).ok();
        }
    });

    Ok(Watcher { tsfn })
}
```

## Error Handling

```rust
// crates/napi/src/error.rs

#[derive(Debug, Clone)]
pub struct Error {
    pub status: Status,
    pub reason: String,
    pub message: Option<String>,
}

pub type Result<T> = std::result::Result<T, Error>;

impl Error {
    pub fn new(status: Status, reason: String) -> Self {
        Self {
            status,
            reason,
            message: None,
        }
    }

    pub fn from_status(status: Status) -> Self {
        Self {
            status,
            reason: status.to_string(),
            message: None,
        }
    }

    /// Convert to JavaScript error value
    pub fn into_value(self, env: sys::napi_env) -> Result<sys::napi_value> {
        let mut error = std::ptr::null_mut();

        // Create error message
        let message = format!("{}: {}", self.status, self.reason);
        let mut msg_val = std::ptr::null_mut();

        unsafe {
            sys::napi_create_string_utf8(
                env,
                message.as_ptr() as *const _,
                message.len(),
                &mut msg_val,
            );

            // Create Error object
            sys::napi_create_error(env, std::ptr::null_mut(), msg_val, &mut error);
        }

        Ok(error)
    }
}

/// Status codes from Node-API
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Status {
    Ok,
    InvalidArg,
    ObjectExpected,
    FunctionExpected,
    GenericFailure,
    PendingException,
    Cancelled,
    EscapeCalled,
    NameMismatch,
    WorkspaceInProcessContext,
    Closed,
    ExceptionPending,
    UnhandledRejection,
}

impl From<sys::napi_status> for Status {
    fn from(status: sys::napi_status) -> Self {
        match status {
            sys::napi_status::napi_ok => Status::Ok,
            sys::napi_status::napi_invalid_arg => Status::InvalidArg,
            sys::napi_status::napi_object_expected => Status::ObjectExpected,
            sys::napi_status::napi_function_expected => Status::FunctionExpected,
            sys::napi_status::napi_generic_failure => Status::GenericFailure,
            _ => Status::GenericFailure,
        }
    }
}

// Macro for checking status
macro_rules! check_status {
    ($code:expr, $msg:expr) => {{
        let status = Status::from($code);
        if status != Status::Ok {
            Err(Error::new(status, $msg.to_string()))
        } else {
            Ok(())
        }
    }};
}
```

## Memory Management

### Finalization Callbacks

```rust
// Buffer finalization
unsafe extern "C" fn finalize_buffer(
    env: sys::napi_env,
    finalize_data: *mut c_void,
    _finalize_hint: *mut c_void,
) {
    // Reconstruct the Box and drop it
    let _ = Box::from_raw(finalize_data as *mut [u8]);
}

// Reference finalization
pub fn create_reference<T>(
    env: sys::napi_env,
    value: sys::napi_value,
    finalizer: Option<extern "C" fn(sys::napi_env, *mut T, *mut T)>,
) -> Result<Ref> {
    let mut ref_ptr = std::ptr::null_mut();

    check_status!(
        sys::napi_create_reference(env, value, 1, &mut ref_ptr),
        "Failed to create reference"
    )?;

    Ok(Ref { inner: ref_ptr })
}
```

## Features

The `napi` crate has several optional features:

```toml
[features]
# N-API version support
napi1 = []
napi2 = ["napi1"]
napi3 = ["napi2"]
napi4 = ["napi3"]
napi5 = ["napi4"]
napi6 = ["napi5"]
napi7 = ["napi6"]
napi8 = ["napi7"]
napi9 = ["napi8"]

# Async support
async = ["tokio"]

# Serde JSON support
serde-json = ["serde_json"]

# Error context
error_anyhow = ["anyhow"]
```
