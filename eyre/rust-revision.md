# Reproducing Eyre in Rust - Implementation Guide

## Overview

This guide explains how to reproduce Eyre's error handling functionality at a production level in Rust. We'll cover the core patterns and provide implementation details.

## Core Components

### 1. Type-Erased Error Wrapper

The foundation of Eyre is a type-erased error wrapper using vtables:

```rust
use std::{
    error::Error as StdError,
    fmt::{Debug, Display},
    mem::ManuallyDrop,
    ptr::{self, NonNull},
};

/// Type-erased error report
pub struct Report {
    inner: OwnedPtr<ErrorImpl<()>>,
}

/// Error implementation wrapper
#[repr(C)]
struct ErrorImpl<E = ()> {
    header: ErrorHeader,
    _object: E,
}

/// Header containing vtable and handler
#[repr(C)]
struct ErrorHeader {
    vtable: &'static ErrorVTable,
    handler: Option<Box<dyn EyreHandler>>,
}

/// VTable for type-erased operations
struct ErrorVTable {
    object_drop: unsafe fn(OwnedPtr<ErrorImpl<()>>),
    object_ref: unsafe fn(&ErrorImpl<()>) -> &(dyn StdError + Send + Sync + 'static),
    object_downcast: unsafe fn(&ErrorImpl<()>, TypeId) -> Option<NonNull<()>>,
    object_drop_rest: unsafe fn(OwnedPtr<ErrorImpl<()>>, TypeId),
}
```

### 2. Custom Handler Trait

Define a trait for customizable error handling:

```rust
use std::{any::Any, fmt};

pub trait EyreHandler: Any + Send + Sync {
    /// Format the error report
    fn debug(
        &self,
        error: &(dyn StdError + 'static),
        f: &mut fmt::Formatter<'_>,
    ) -> fmt::Result;

    /// Optional: Override display format
    fn display(
        &self,
        error: &(dyn StdError + 'static),
        f: &mut fmt::Formatter<'_>,
    ) -> fmt::Result {
        write!(f, "{}", error)?;
        if f.alternate() {
            for cause in Chain::new(error).skip(1) {
                write!(f, ": {}", cause)?;
            }
        }
        Ok(())
    }

    /// Optional: Track caller location
    fn track_caller(&mut self, _location: &'static std::panic::Location<'static>) {}
}
```

### 3. Global Hook System

Install a global hook for creating handlers:

```rust
use once_cell::sync::OnceCell;

type ErrorHook = Box<dyn Fn(&(dyn StdError + 'static)) -> Box<dyn EyreHandler> + Sync + Send>;

static HOOK: OnceCell<ErrorHook> = OnceCell::new();

/// Install a custom error hook
pub fn set_hook(hook: ErrorHook) -> Result<(), InstallError> {
    HOOK.set(hook).map_err(|_| InstallError)
}

/// Capture handler for new errors
fn capture_handler(error: &(dyn StdError + 'static)) -> Box<dyn EyreHandler> {
    let hook = HOOK.get_or_init(|| {
        Box::new(|_| Box::new(DefaultHandler::default()))
    });
    hook(error)
}

pub struct InstallError;
impl Display for InstallError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("a hook has already been installed")
    }
}
```

### 4. Default Handler with Backtrace

```rust
use std::backtrace::Backtrace;

pub struct DefaultHandler {
    backtrace: Option<Backtrace>,
}

impl DefaultHandler {
    pub fn default() -> Self {
        // Capture backtrace if enabled via env vars
        let backtrace = if should_capture_backtrace() {
            Some(Backtrace::capture())
        } else {
            None
        };
        Self { backtrace }
    }
}

impl EyreHandler for DefaultHandler {
    fn debug(&self, error: &(dyn StdError + 'static), f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if f.alternate() {
            return Debug::fmt(error, f);
        }

        // Display error chain
        writeln!(f, "{}", error)?;
        let mut source = error.source();
        while let Some(cause) = source {
            writeln!(f, "Caused by: {}", cause)?;
            source = cause.source();
        }

        // Display backtrace if captured
        if let Some(bt) = &self.backtrace {
            writeln!(f, "\nStack backtrace:\n{:?}", bt)?;
        }

        Ok(())
    }
}

fn should_capture_backtrace() -> bool {
    std::env::var("RUST_LIB_BACKTRACE")
        .or_else(|_| std::env::var("RUST_BACKTRACE"))
        .map(|v| v != "0")
        .unwrap_or(false)
}
```

### 5. Context Wrapping

Implement the `WrapErr` trait for adding context:

```rust
/// Context error wrapper
#[repr(C)]
struct ContextError<D, E> {
    msg: D,
    error: E,
}

impl<D, E> Display for ContextError<D, E>
where
    D: Display,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        Display::fmt(&self.msg, f)
    }
}

impl<D, E> Debug for ContextError<D, E>
where
    D: Display,
    E: Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Error")
            .field("msg", &Quoted(&self.msg))
            .field("source", &self.error)
            .finish()
    }
}

impl<D, E> StdError for ContextError<D, E>
where
    D: Display,
    E: StdError + 'static,
{
    fn source(&self) -> Option<&(dyn StdError + 'static)> {
        Some(&self.error)
    }
}

/// Trait for adding context to errors
pub trait WrapErr<T, E>: Sized {
    fn wrap_err<D>(self, msg: D) -> Result<T, Report>
    where
        D: Display + Send + Sync + 'static;

    fn wrap_err_with<D, F>(self, msg: F) -> Result<T, Report>
    where
        D: Display + Send + Sync + 'static,
        F: FnOnce() -> D;
}

impl<T, E> WrapErr<T, E> for Result<T, E>
where
    E: StdError + Send + Sync + 'static,
{
    fn wrap_err<D>(self, msg: D) -> Result<T, Report>
    where
        D: Display + Send + Sync + 'static,
    {
        match self {
            Ok(t) => Ok(t),
            Err(e) => Err(Report::from_msg(msg, e)),
        }
    }

    fn wrap_err_with<D, F>(self, msg: F) -> Result<T, Report>
    where
        D: Display + Send + Sync + 'static,
        F: FnOnce() -> D,
    {
        match self {
            Ok(t) => Ok(t),
            Err(e) => Err(Report::from_msg(msg(), e)),
        }
    }
}
```

### 6. Error Chain Iterator

```rust
pub struct Chain<'a> {
    next: Option<&'a (dyn StdError + 'static)>,
}

impl<'a> Chain<'a> {
    pub fn new(head: &'a (dyn StdError + 'static)) -> Self {
        Self { next: Some(head) }
    }
}

impl<'a> Iterator for Chain<'a> {
    type Item = &'a (dyn StdError + 'static);

    fn next(&mut self) -> Option<Self::Item> {
        let error = self.next?;
        self.next = error.source();
        Some(error)
    }
}
```

### 7. Error Macros

```rust
/// Create an ad-hoc error
#[macro_export]
macro_rules! eyre {
    ($msg:literal $(,)?) => {{
        $crate::Report::msg(format!($msg))
    }};
    ($fmt:expr, $($arg:tt)*) => {{
        $crate::Report::msg(format!($fmt, $($arg)*))
    }};
}

/// Return early with error
#[macro_export]
macro_rules! bail {
    ($msg:literal $(,)?) => {{
        return Err($crate::eyre!($msg));
    }};
    ($fmt:expr, $($arg:tt)*) => {{
        return Err($crate::eyre!($fmt, $($arg)*));
    }};
}

/// Assert condition or return error
#[macro_export]
macro_rules! ensure {
    ($cond:expr, $msg:literal $(,)?) => {{
        if !$cond {
            return Err($crate::eyre!($msg));
        }
    }};
    ($cond:expr, $fmt:expr, $($arg:tt)*) => {{
        if !$cond {
            return Err($crate::eyre!($fmt, $($arg)*));
        }
    }};
}
```

### 8. Result Type Alias

```rust
pub type Result<T, E = Report> = core::result::Result<T, E>;
```

## Advanced Features

### 9. Backtrace Capture with Fallback

For production use, handle both stable and nightly Rust:

```rust
#[cfg(feature = "std-backtrace")]
use std::backtrace::Backtrace;

#[cfg(feature = "backtrace-rs")]
use backtrace::Backtrace;

pub struct AdvancedHandler {
    backtrace: Option<Backtrace>,
    #[cfg(feature = "tracing-error")]
    span_trace: Option<tracing_error::SpanTrace>,
}

impl AdvancedHandler {
    pub fn new(error: &(dyn StdError + 'static)) -> Self {
        // Check if error already has backtrace (nightly only)
        let backtrace = Self::capture_backtrace(error);

        #[cfg(feature = "tracing-error")]
        let span_trace = Self::capture_span_trace(error);

        Self {
            backtrace,
            #[cfg(feature = "tracing-error")]
            span_trace,
        }
    }

    fn capture_backtrace(error: &(dyn StdError + 'static)) -> Option<Backtrace> {
        #[cfg(feature = "generic-member-access")]
        {
            // Nightly: check if error already has backtrace
            if std::error::request_ref::<Backtrace>(error).is_some() {
                return None;
            }
        }

        // Capture if enabled
        if should_capture_backtrace() {
            Some(Backtrace::capture())
        } else {
            None
        }
    }

    #[cfg(feature = "tracing-error")]
    fn capture_span_trace(error: &(dyn StdError + 'static)) -> Option<tracing_error::SpanTrace> {
        // Check if any error in chain already has span trace
        if Chain::new(error).any(|e| {
            // Would need SpanTrace extension trait
            false
        }) {
            return None;
        }

        let enabled = std::env::var("RUST_SPANTRACE")
            .map(|v| v != "0")
            .unwrap_or(true);

        if enabled {
            Some(tracing_error::SpanTrace::capture())
        } else {
            None
        }
    }
}
```

### 10. Downcasting Support

```rust
impl Report {
    /// Downcast error to concrete type
    pub fn downcast<E>(self) -> Result<E, Self>
    where
        E: Display + Debug + Send + Sync + 'static,
    {
        let target = TypeId::of::<E>();
        unsafe {
            let addr = match (self.vtable().object_downcast)(self.inner.as_ref(), target) {
                Some(addr) => addr,
                None => return Err(self),
            };

            // Read the error value
            let error = ptr::read(addr.cast::<E>().as_ptr());

            // Drop remaining data structure
            let outer = ManuallyDrop::new(self);
            (outer.vtable().object_drop_rest)(outer.inner, target);

            Ok(error)
        }
    }

    /// Downcast by reference
    pub fn downcast_ref<E>(&self) -> Option<&E>
    where
        E: Display + Debug + Send + Sync + 'static,
    {
        let target = TypeId::of::<E>();
        unsafe {
            let addr = (self.vtable().object_downcast)(self.inner.as_ref(), target)?;
            Some(addr.cast::<E>().as_ref())
        }
    }
}
```

### 11. Handler Downcasting

```rust
impl dyn EyreHandler {
    pub fn is<T: EyreHandler>(&self) -> bool {
        TypeId::of::<T>() == self.type_id()
    }

    pub fn downcast_ref<T: EyreHandler>(&self) -> Option<&T> {
        if self.is::<T>() {
            unsafe { Some(&*(self as *const dyn EyreHandler as *const T)) }
        } else {
            None
        }
    }

    pub fn downcast_mut<T: EyreHandler>(&mut self) -> Option<&mut T> {
        if self.is::<T>() {
            unsafe { Some(&mut *(self as *mut dyn EyreHandler as *mut T)) }
        } else {
            None
        }
    }
}
```

### 12. Custom Sections (color-eyre style)

```rust
/// Help information for error reports
pub enum HelpInfo {
    Custom(Box<dyn Display + Send + Sync>),
    Error(Box<dyn StdError + Send + Sync>),
    Note(Box<dyn Display + Send + Sync>),
    Warning(Box<dyn Display + Send + Sync>),
    Suggestion(Box<dyn Display + Send + Sync>),
}

/// Trait for adding sections to error reports
pub trait Section: Sized {
    type Return;

    fn section<D>(self, section: D) -> Self::Return
    where
        D: Display + Send + Sync + 'static;

    fn with_section<D, F>(self, f: F) -> Self::Return
    where
        D: Display + Send + Sync + 'static,
        F: FnOnce() -> D;

    fn note<D>(self, note: D) -> Self::Return
    where
        D: Display + Send + Sync + 'static;

    fn suggestion<D>(self, suggestion: D) -> Self::Return
    where
        D: Display + Send + Sync + 'static;
}

impl Section for Result<(), Report> {
    type Return = Self;

    fn section<D>(self, section: D) -> Self::Return
    where
        D: Display + Send + Sync + 'static,
    {
        self.map_err(|mut e| {
            if let Some(handler) = e.handler_mut().downcast_mut::<AdvancedHandler>() {
                handler.sections.push(HelpInfo::Custom(Box::new(section)));
            }
            e
        })
    }

    fn note<D>(self, note: D) -> Self::Return
    where
        D: Display + Send + Sync + 'static,
    {
        self.map_err(|mut e| {
            if let Some(handler) = e.handler_mut().downcast_mut::<AdvancedHandler>() {
                handler.sections.push(HelpInfo::Note(Box::new(note)));
            }
            e
        })
    }

    fn suggestion<D>(self, suggestion: D) -> Self::Return
    where
        D: Display + Send + Sync + 'static,
    {
        self.map_err(|mut e| {
            if let Some(handler) = e.handler_mut().downcast_mut::<AdvancedHandler>() {
                handler.sections.push(HelpInfo::Suggestion(Box::new(suggestion)));
            }
            e
        })
    }

    fn with_section<D, F>(self, f: F) -> Self::Return
    where
        D: Display + Send + Sync + 'static,
        F: FnOnce() -> D,
    {
        self.section(f())
    }
}
```

## Complete Minimal Implementation

Here's a minimal working implementation:

```rust
// lib.rs
use std::{
    backtrace::Backtrace,
    error::Error as StdError,
    fmt::{self, Debug, Display},
};
use once_cell::sync::OnceCell;

type Result<T, E = Report> = core::result::Result<T, E>;

// Handler trait
pub trait EyreHandler: std::any::Any + Send + Sync {
    fn debug(&self, error: &(dyn StdError + 'static), f: &mut fmt::Formatter<'_>) -> fmt::Result;
}

// Default handler
pub struct DefaultHandler {
    backtrace: Option<Backtrace>,
}

impl DefaultHandler {
    pub fn new() -> Self {
        let backtrace = std::env::var("RUST_LIB_BACKTRACE")
            .or_else(|_| std::env::var("RUST_BACKTRACE"))
            .map(|v| v != "0")
            .unwrap_or(false)
            .then(Backtrace::capture);

        Self { backtrace }
    }
}

impl EyreHandler for DefaultHandler {
    fn debug(&self, error: &(dyn StdError + 'static), f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if f.alternate() {
            return Debug::fmt(error, f);
        }

        writeln!(f, "{}", error)?;
        let mut source = error.source();
        while let Some(cause) = source {
            writeln!(f, "Caused by: {}", cause)?;
            source = cause.source();
        }

        if let Some(bt) = &self.backtrace {
            if matches!(bt.status(), std::backtrace::BacktraceStatus::Captured) {
                writeln!(f, "\n{}", bt)?;
            }
        }

        Ok(())
    }
}

// Hook system
type ErrorHook = Box<dyn Fn(&(dyn StdError + 'static)) -> Box<dyn EyreHandler> + Sync + Send>;
static HOOK: OnceCell<ErrorHook> = OnceCell::new();

pub fn set_hook(hook: ErrorHook) -> Result<(), InstallError> {
    HOOK.set(hook).map_err(|_| InstallError)
}

fn capture_handler(_error: &(dyn StdError + 'static)) -> Box<dyn EyreHandler> {
    let hook = HOOK.get_or_init(|| Box::new(|_| Box::new(DefaultHandler::new())));
    hook(_error)
}

// Report type
pub struct Report {
    error: Box<dyn StdError + Send + Sync + 'static>,
    handler: Box<dyn EyreHandler>,
}

impl Report {
    pub fn new<E>(error: E) -> Self
    where
        E: StdError + Send + Sync + 'static,
    {
        let handler = capture_handler(&error);
        Self {
            error: Box::new(error),
            handler,
        }
    }

    pub fn msg<M>(message: M) -> Self
    where
        M: Display + Debug + Send + Sync + 'static,
    {
        Self::new(MessageError(message))
    }
}

impl<E> From<E> for Report
where
    E: StdError + Send + Sync + 'static,
{
    fn from(error: E) -> Self {
        Self::new(error)
    }
}

impl Display for Report {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        Display::fmt(&self.error, f)
    }
}

impl Debug for Report {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.handler.debug(&self.error, f)
    }
}

impl StdError for Report {
    fn source(&self) -> Option<&(dyn StdError + 'static)> {
        self.error.source()
    }
}

// Simple message error
#[derive(Debug)]
struct MessageError<M>(M);

impl<M: Display> Display for MessageError<M> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        Display::fmt(&self.0, f)
    }
}

impl<M: Display + Debug> StdError for MessageError<M> {}

// WrapErr trait
pub trait WrapErr<T, E>: Sized {
    fn wrap_err<D>(self, msg: D) -> Result<T, Report>
    where
        D: Display + Send + Sync + 'static;
}

impl<T, E> WrapErr<T, E> for Result<T, E>
where
    E: StdError + Send + Sync + 'static,
{
    fn wrap_err<D>(self, msg: D) -> Result<T, Report>
    where
        D: Display + Send + Sync + 'static,
    {
        match self {
            Ok(t) => Ok(t),
            Err(e) => Err(Report::new(ContextError { msg, error: e })),
        }
    }
}

#[derive(Debug)]
struct ContextError<D, E> {
    msg: D,
    error: E,
}

impl<D: Display, E> Display for ContextError<D, E> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        Display::fmt(&self.msg, f)
    }
}

impl<D: Display, E: StdError> StdError for ContextError<D, E> {
    fn source(&self) -> Option<&(dyn StdError + 'static)> {
        Some(&self.error)
    }
}

// Macros
#[macro_export]
macro_rules! eyre {
    ($msg:literal) => {
        $crate::Report::msg(format!($msg))
    };
    ($fmt:expr, $($arg:tt)*) => {
        $crate::Report::msg(format!($fmt, $($arg)*))
    };
}

#[macro_export]
macro_rules! bail {
    ($msg:literal) => {
        return Err($crate::eyre!($msg));
    };
    ($fmt:expr, $($arg:tt)*) => {
        return Err($crate::eyre!($fmt, $($arg)*));
    };
}
```

## Cargo.toml Configuration

```toml
[package]
name = "my-eyre-clone"
version = "0.1.0"
edition = "2021"

[dependencies]
once_cell = "1.18"

[features]
default = ["std-backtrace"]
std-backtrace = []
backtrace-rs = ["backtrace"]
tracing-error = ["dep:tracing-error"]

[dependencies.backtrace]
version = "0.3"
optional = true
features = ["gimli-symbolize"]

[dependencies.tracing-error]
version = "0.2"
optional = true
```

## Usage Example

```rust
use my_eyre_clone::{eyre, WrapErr, Result};

fn main() -> Result<()> {
    // Install custom hook if desired
    // my_eyre_clone::set_hook(Box::new(|_| {
    //     Box::new(MyCustomHandler::new())
    // }))?;

    read_config("config.json")?;
    Ok(())
}

fn read_config(path: &str) -> Result<Config> {
    let content = std::fs::read_to_string(path)
        .wrap_err_with(|| format!("Failed to read config from {}", path))?;

    let config: Config = serde_json::from_str(&content)
        .wrap_err("Failed to parse JSON")?;

    Ok(config)
}
```

## Key Production Considerations

1. **Thread Safety**: Use `Arc` for shared state in handlers
2. **Performance**: Backtrace capture is expensive - make it configurable
3. **Memory**: Type erasure prevents stack overflow from deep error chains
4. **Compatibility**: Support both stable (1.65+) and nightly features
5. **Integration**: Provide panic hook integration for full coverage
6. **Formatting**: Support multiple verbosity levels via environment variables
