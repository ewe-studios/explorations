# Eyre Error Handling Library - Comprehensive Exploration

## Overview

Eyre is a flexible, concrete error handling library for Rust applications built on `std::error::Error`. It is a fork of [`anyhow`](https://github.com/dtolnay/anyhow) with support for **customized error reports** through the `EyreHandler` trait.

**Source**: `/home/darkvoid/Boxxed/@formulas/src.rust/src.eyre`

## Workspace Structure

```
eyre/
├── eyre/              # Core error handling library
├── color-eyre/        # Colorful error reports with tracing support
├── color-spantrace/   # Pretty printer for tracing_error::SpanTrace
├── indenter/          # Indentation formatter for error display
├── simple-eyre/       # Minimal handler without backtrace capture
└── stable-eyre/       # Backtrace capture on stable using backtrace-rs
```

## Key Design Principles

### 1. Handler-Based Customization

The core innovation of eyre is the `EyreHandler` trait, which allows complete customization of error report formatting:

```rust
pub trait EyreHandler: core::any::Any + Send + Sync {
    fn debug(
        &self,
        error: &(dyn StdError + 'static),
        f: &mut core::fmt::Formatter<'_>,
    ) -> core::fmt::Result;

    fn display(&self, error: &(dyn StdError + 'static), f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{}", error)?;
        if f.alternate() {
            for cause in crate::chain::Chain::new(error).skip(1) {
                write!(f, ": {}", cause)?;
            }
        }
        Result::Ok(())
    }

    fn track_caller(&mut self, location: &'static std::panic::Location<'static>) {}
}
```

### 2. Report Structure

The `Report` type is a thin wrapper around a type-erased error:

```rust
pub struct Report {
    inner: OwnedPtr<ErrorImpl<()>>,
}

#[repr(C)]
pub(crate) struct ErrorImpl<E = ()> {
    header: ErrorHeader,
    _object: E,
}

#[repr(C)]
pub(crate) struct ErrorHeader {
    vtable: &'static ErrorVTable,
    pub(crate) handler: Option<Box<dyn EyreHandler>>,
}
```

### 3. VTable-Based Type Erasure

Eyre uses a custom vtable for type-erased error storage:

```rust
struct ErrorVTable {
    object_drop: unsafe fn(OwnedPtr<ErrorImpl<()>>),
    object_ref: unsafe fn(RefPtr<'_, ErrorImpl<()>>) -> &(dyn StdError + Send + Sync + 'static),
    object_mut: unsafe fn(MutPtr<'_, ErrorImpl<()>>) -> &mut (dyn StdError + Send + Sync + 'static),
    object_boxed: unsafe fn(OwnedPtr<ErrorImpl<()>>) -> Box<dyn StdError + Send + Sync + 'static>,
    object_downcast: unsafe fn(RefPtr<'_, ErrorImpl<()>>, TypeId) -> Option<NonNull<()>>,
    object_downcast_mut: unsafe fn(MutPtr<'_, ErrorImpl<()>>, TypeId) -> Option<NonNull<()>>,
    object_drop_rest: unsafe fn(OwnedPtr<ErrorImpl<()>>, TypeId),
}
```

## Companion Crates

### color-eyre

The most feature-rich handler providing:
- Colorful backtrace rendering via `backtrace-rs`
- `SpanTrace` capture via `tracing-error`
- Custom sections (notes, warnings, suggestions)
- Panic hook integration
- GitHub issue URL generation

```rust
// Handler structure
pub struct Handler {
    filters: Arc<[Box<FilterCallback>]>,
    backtrace: Option<Backtrace>,
    suppress_backtrace: bool,
    span_trace: Option<SpanTrace>,
    sections: Vec<HelpInfo>,
    display_env_section: bool,
    display_location_section: bool,
    issue_url: Option<String>,
    issue_metadata: Arc<Vec<(String, Box<dyn Display + Send + Sync>)>>,
    issue_filter: Arc<IssueFilterCallback>,
    theme: Theme,
    location: Option<&'static Location<'static>>,
}
```

### color-spantrace

Provides pretty-printing for `tracing_error::SpanTrace`:

```rust
pub fn colorize(span_trace: &SpanTrace) -> impl fmt::Display + '_ {
    let theme = *THEME.get_or_init(Theme::dark);
    ColorSpanTrace { span_trace, theme }
}
```

### stable-eyre

Enables backtrace capture on stable Rust by using `backtrace-rs` instead of `std::backtrace::Backtrace`.

### simple-eyre

A minimal handler that captures no additional information - for when you don't need backtraces.

### indenter

A small utility for indenting text during formatting:

```rust
pub fn indented(f: &mut fmt::Formatter<'_>) -> Indented<'_> {
    Indented {
        inner: f,
        format: Format::Short,
        ..Default::default()
    }
}
```

## Error Chain Tracking

Eyre implements error chain iteration via the `Chain` struct:

```rust
pub struct Chain<'a> {
    state: ChainState<'a>,
}

enum ChainState<'a> {
    Linked { next: Option<&'a (dyn StdError + 'static)>, },
    Buffered { rest: vec::IntoIter<&'a (dyn StdError + 'static)>, },
}

impl<'a> Iterator for Chain<'_> {
    type Item = &'a (dyn StdError + 'static);

    fn next(&mut self) -> Option<Self::Item> {
        match &mut self.state {
            Linked { next } => {
                let error = (*next)?;
                *next = error.source();
                Some(error)
            }
            Buffered { rest } => rest.next(),
        }
    }
}
```

## Context Wrapping Mechanism

The `WrapErr` trait provides error context:

```rust
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
    E: ext::StdError + Send + Sync + 'static,
{
    fn wrap_err<D>(self, msg: D) -> Result<T, Report>
    where
        D: Display + Send + Sync + 'static,
    {
        match self {
            Ok(t) => Ok(t),
            Err(e) => Err(e.ext_report(msg)),
        }
    }
    // ...
}
```

The context is stored in a `ContextError`:

```rust
#[repr(C)]
pub(crate) struct ContextError<D, E> {
    pub(crate) msg: D,
    pub(crate) error: E,
}
```

## Backtrace Capture Mechanisms

### Stable Rust (1.65+)

```rust
#[cfg(backtrace)]
macro_rules! capture_backtrace {
    () => { Some(Backtrace::capture()) };
}

#[cfg(not(backtrace))]
macro_rules! capture_backtrace {
    () => { None };
}
```

### Nightly with Generic Member Access

On nightly, eyre can access backtraces from underlying errors:

```rust
#[cfg(generic_member_access)]
macro_rules! backtrace_if_absent {
    ($err:expr) => {
        match std::error::request_ref::<std::backtrace::Backtrace>($err as &dyn std::error::Error) {
            Some(_) => None,  // Already has backtrace
            None => capture_backtrace!(),
        }
    };
}
```

### Stable Backtrace (stable-eyre)

Uses `backtrace-rs` instead:

```rust
// In stable-eyre
use backtrace::Backtrace;  // Not std::backtrace::Backtrace

pub fn default(&self, error: &(dyn std::error::Error + 'static)) -> Handler {
    let backtrace = Some(Backtrace::new());  // Always available
    // ...
}
```

## Comparison with anyhow and thiserror

| Feature | eyre | anyhow | thiserror |
|---------|------|--------|-----------|
| Custom handlers | Yes | No | N/A |
| Backtrace (stable 1.65+) | Yes | Yes | N/A |
| SpanTrace support | Yes (color-eyre) | No | No |
| Custom sections | Yes (color-eyre) | No | No |
| Panic hook integration | Yes (color-eyre) | No | No |
| Derive macro | No | No | Yes |
| Library error types | Not recommended | Not recommended | Recommended |
| Application errors | Recommended | Recommended | Not ideal |

### When to Use Each

**Use eyre when:**
- You want customizable error reports
- You need colorful, formatted error output
- You want tracing/span trace integration
- You're building applications (not libraries)

**Use anyhow when:**
- You want simple, drop-in error handling
- You don't need customization
- You're building applications

**Use thiserror when:**
- You're building a library
- You need a public error type
- You need pattern matching on errors

## Key Files and Their Purposes

| File | Purpose |
|------|---------|
| `eyre/src/lib.rs` | Main library entry, Report struct, EyreHandler trait |
| `eyre/src/error.rs` | ErrorImpl, vtable functions, downcasting |
| `eyre/src/context.rs` | WrapErr trait, ContextError |
| `eyre/src/chain.rs` | Error chain iteration |
| `eyre/src/backtrace.rs` | Backtrace capture macros |
| `eyre/src/macros.rs` | eyre!, bail!, ensure! macros |
| `color-eyre/src/handler.rs` | color-eyre Handler implementation |
| `color-eyre/src/config.rs` | HookBuilder, Theme, Frame filtering |
| `color-eyre/src/section/mod.rs` | Section trait for custom sections |
| `color-spantrace/src/lib.rs` | SpanTrace pretty printing |

## Environment Variables

| Variable | Effect |
|----------|--------|
| `RUST_BACKTRACE=1` | Enable backtraces for panics |
| `RUST_LIB_BACKTRACE=1` | Enable backtraces for errors |
| `RUST_SPANTRACE=0` | Disable span trace capture |
| `COLORBT_SHOW_HIDDEN=1` | Show all backtrace frames |
