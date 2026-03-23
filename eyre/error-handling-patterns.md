# Eyre Error Handling Patterns

## Error Context and Chain Tracking

### Basic Context Wrapping

The `WrapErr` trait provides two methods for adding context to errors:

```rust
use eyre::{WrapErr, Result};

fn read_config(path: &str) -> Result<Config> {
    let content = std::fs::read_to_string(path)
        .wrap_err_with(|| format!("Failed to read config from {}", path))?;

    let config: Config = serde_json::from_str(&content)
        .wrap_err("Failed to parse config JSON")?;

    Ok(config)
}
```

### How Context Wrapping Works Internally

1. **ContextError Creation**: When `wrap_err` is called, a `ContextError<D, E>` is created:

```rust
pub(crate) struct ContextError<D, E> {
    pub(crate) msg: D,
    pub(crate) error: E,
}
```

2. **VTable Setup**: A new vtable is created for the context type:

```rust
let vtable = &ErrorVTable {
    object_drop: object_drop::<ContextError<D, E>>,
    object_ref: object_ref::<ContextError<D, E>>,
    object_downcast: context_downcast::<D, E>,
    object_downcast_mut: context_downcast_mut::<D, E>,
    object_drop_rest: context_drop_rest::<D, E>,
    // ...
};
```

3. **Downcast Support**: The context downcast function checks both the message and underlying error:

```rust
unsafe fn context_downcast<D, E>(
    e: RefPtr<'_, ErrorImpl<()>>,
    target: TypeId,
) -> Option<NonNull<()>>
where
    D: 'static,
    E: 'static,
{
    if TypeId::of::<D>() == target {
        // Downcast to message type
        let unerased = e.cast::<ErrorImpl<ContextError<D, E>>>().as_ref();
        Some(NonNull::from(&unerased._object.msg).cast::<()>())
    } else if TypeId::of::<E>() == target {
        // Downcast to error type
        let unerased = e.cast::<ErrorImpl<ContextError<D, E>>>().as_ref();
        Some(NonNull::from(&unerased._object.error).cast::<()>())
    } else {
        None
    }
}
```

4. **Chain Iteration**: The error chain is traversed using `source()`:

```rust
impl<D, E> StdError for ContextError<D, E>
where
    D: Display,
    E: StdError + 'static,
{
    fn source(&self) -> Option<&(dyn StdError + 'static)> {
        Some(&self.error)
    }
}
```

### Context Chain with Multiple Wraps

```rust
fn process_data(path: &str) -> Result<Data> {
    let file = std::fs::File::open(path)
        .wrap_err("Failed to open data file")?;

    let reader = BufReader::new(file);
    let data = parse_reader(reader)
        .wrap_err("Failed to parse file contents")?;

    validate_data(&data)
        .wrap_err("Data validation failed")?;

    Ok(data)
}
```

Output:
```
Error: Data validation failed

Caused by:
    Failed to parse file contents
    Failed to open data file
    No such file or directory (os error 2)
```

## Backtrace Capture Mechanisms

### Environment-Controlled Capture

Backtrace capture is controlled by environment variables:

```rust
// In eyre/src/backtrace.rs
#[cfg(backtrace)]
pub(crate) use std::backtrace::Backtrace;

#[cfg(backtrace)]
macro_rules! capture_backtrace {
    () => { Some(Backtrace::capture()) };
}

#[cfg(not(backtrace))]
macro_rules! capture_backtrace {
    () => { None };
}
```

### Avoiding Duplicate Backtraces

On nightly Rust, eyre can check if an error already has a backtrace:

```rust
#[cfg(generic_member_access)]
macro_rules! backtrace_if_absent {
    ($err:expr) => {
        match std::error::request_ref::<std::backtrace::Backtrace>($err as &dyn std::error::Error) {
            Some(_) => None,  // Source already has backtrace
            None => capture_backtrace!(),
        }
    };
}

#[cfg(not(generic_member_access))]
macro_rules! backtrace_if_absent {
    ($err:expr) => {
        capture_backtrace!()  // Always capture
    };
}
```

### Backtrace in Handler

The handler stores and formats the backtrace:

```rust
// In color-eyre/src/handler.rs
impl eyre::EyreHandler for Handler {
    fn debug(&self, error: &(dyn std::error::Error + 'static), f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        // ... error chain display ...

        if !self.suppress_backtrace {
            if let Some(backtrace) = self.backtrace.as_ref() {
                let fmted_bt = self.format_backtrace(backtrace);
                write!(
                    indented(&mut separated.ready())
                        .with_format(Format::Uniform { indentation: "  " }),
                    "{}",
                    fmted_bt
                )?;
            }
        }
        Ok(())
    }
}
```

## Custom Error Types and Trait Objects

### Type-Erased Error Storage

Eyre stores errors in a type-erased manner using a vtable pattern:

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

### VTable Operations

The vtable provides all operations needed for type-erased errors:

```rust
struct ErrorVTable {
    // Memory management
    object_drop: unsafe fn(OwnedPtr<ErrorImpl<()>>),
    object_drop_rest: unsafe fn(OwnedPtr<ErrorImpl<()>>, TypeId),

    // Reference access
    object_ref: unsafe fn(RefPtr<'_, ErrorImpl<()>>) -> &(dyn StdError + Send + Sync + 'static),
    object_mut: unsafe fn(MutPtr<'_, ErrorImpl<()>>) -> &mut (dyn StdError + Send + Sync + 'static),
    object_boxed: unsafe fn(OwnedPtr<ErrorImpl<()>>) -> Box<dyn StdError + Send + Sync + 'static>,

    // Downcasting
    object_downcast: unsafe fn(RefPtr<'_, ErrorImpl<()>>, TypeId) -> Option<NonNull<()>>,
    object_downcast_mut: unsafe fn(MutPtr<'_, ErrorImpl<()>>, TypeId) -> Option<NonNull<()>>,
}
```

### Downcasting Implementation

```rust
impl Report {
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

            let outer = ManuallyDrop::new(self);
            let error = ptr::read(addr.cast::<E>().as_ptr());
            let inner = ptr::read(&outer.inner);
            (outer.vtable().object_drop_rest)(inner, target);

            Ok(error)
        }
    }
}
```

### Custom Handler Implementation

```rust
use eyre::EyreHandler;
use std::error::Error;
use std::fmt;

pub struct CustomHandler {
    custom_data: String,
}

impl EyreHandler for CustomHandler {
    fn debug(&self, error: &(dyn Error + 'static), f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Custom formatting
        writeln!(f, "Custom Error Report")?;
        writeln!(f, "Data: {}", self.custom_data)?;
        writeln!(f, "Error: {}", error)?;

        // Show cause chain
        let mut source = error.source();
        while let Some(cause) = source {
            writeln!(f, "Caused by: {}", cause)?;
            source = cause.source();
        }

        Ok(())
    }
}

// Install custom handler
fn install_custom_handler() -> Result<(), eyre::InstallError> {
    eyre::set_hook(Box::new(|_| {
        Box::new(CustomHandler {
            custom_data: "my custom data".to_string(),
        })
    }))
}
```

## Error Report Structure

The error report format in color-eyre consists of multiple sections:

```
Error:
   0: Top-level error message
   1: Wrapped context
   2: Underlying error

Location:
   src/main.rs:42

Stderr:
   command output here

━━━ SPANTRACE ━━━

 0: module::function with arg=value
    at src/file.rs:10

━━━ BACKTRACE ━━━

 6: my_crate::function::h1234567890abcdef
    at src/file.rs:10
 7: std::rt::lang_start
    ⋮ 5 frames hidden ⋮

Environment:
  OS: linux
  Arch: x86_64
```

### Section Ordering

1. Error chain (always first)
2. Location section (if enabled)
3. Error sections (attached via `Section` trait)
4. Custom sections
5. SpanTrace (if captured)
6. Backtrace (if captured)
7. Environment section (if enabled)
8. GitHub issue section (if configured)

## Section Trait for Custom Content

```rust
pub trait Section: Sized {
    type Return;

    fn section<D>(self, section: D) -> Self::Return
    where
        D: Display + Send + Sync + 'static;

    fn with_section<D, F>(self, section: F) -> Self::Return
    where
        D: Display + Send + Sync + 'static,
        F: FnOnce() -> D;

    fn note<D>(self, note: D) -> Self::Return;
    fn warning<D>(self, warning: D) -> Self::Return;
    fn suggestion<D>(self, suggestion: D) -> Self::Return;
}
```

### Usage Example

```rust
use color_eyre::{Section, SectionExt, eyre::eyre};

fn run_command(cmd: &str) -> color_eyre::Result<String> {
    let output = std::process::Command::new(cmd)
        .output()
        .map_err(|e| eyre!("Failed to execute command"))
        .with_section(|| format!("Command: {}", cmd).header("Command:"))?;

    if !output.status.success() {
        return Err(eyre!("Command failed"))
            .with_section(|| String::from_utf8_lossy(&output.stderr).header("Stderr:"))
            .with_section(|| String::from_utf8_lossy(&output.stdout).header("Stdout:"))
            .suggestion("Check if the command exists and is executable");
    }

    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}
```

## Production-Level Error Handling Patterns

### 1. Library vs Application Boundary

```rust
// In a library - use thiserror
use thiserror::Error;

#[derive(Error, Debug)]
pub enum DatabaseError {
    #[error("Connection failed: {0}")]
    Connection(#[from] sqlx::Error),

    #[error("Query failed: {0}")]
    Query(String),
}

// In the application - wrap library errors with eyre
use eyre::{WrapErr, Result};

fn load_user(id: i64) -> Result<User> {
    db::get_user(id)
        .await
        .wrap_err_with(|| format!("Failed to load user {}", id))
}
```

### 2. Aggregating Multiple Errors

```rust
use color_eyre::{Section, eyre::eyre};

fn validate_batch(items: &[Item]) -> color_eyre::Result<()> {
    let mut errors = Vec::new();

    for (i, item) in items.iter().enumerate() {
        if let Err(e) = validate_item(item) {
            errors.push(format!("Item {}: {}", i, e));
        }
    }

    if !errors.is_empty() {
        return Err(eyre!("{} items failed validation", errors.len()))
            .with_section(|| errors.join("\n").header("Failures:"));
    }

    Ok(())
}
```

### 3. Supressing Backtraces for Expected Errors

```rust
use color_eyre::Section;

fn check_file_exists(path: &str) -> color_eyre::Result<()> {
    if !std::path::Path::new(path).exists() {
        return Err(eyre::eyre!("File not found: {}", path))
            .suggestion("Check the file path and try again")
            .suppress_backtrace(true);  // Don't show backtrace for expected errors
    }
    Ok(())
}
```

### 4. Adding GitHub Issue Links

```rust
use color_eyre::config::HookBuilder;

fn main() -> color_eyre::Result<()> {
    HookBuilder::default()
        .issue_url("https://github.com/my-org/my-app/issues/new")
        .add_issue_metadata("version", env!("CARGO_PKG_VERSION"))
        .add_issue_metadata("commit", env!("GIT_COMMIT"))
        .install()?;

    // Application code...
}
```

### 5. Custom Frame Filtering

```rust
use color_eyre::config::HookBuilder;

HookBuilder::default()
    .add_frame_filter(Box::new(|frames| {
        let filters = &["tokio::", "async_std::", "my_crate::internal::"];
        frames.retain(|frame| {
            !filters.iter().any(|f| {
                frame.name.as_ref()
                    .map(|n| n.starts_with(f))
                    .unwrap_or(true)
            })
        });
    }))
    .install()?;
```
