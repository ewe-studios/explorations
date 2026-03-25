# tracing-web Implementation Details

This document provides a **code-level analysis** of tracing-web, covering the structure, key algorithms, and implementation patterns.

## Source Structure

```
src/
├── lib.rs                  # 36 lines - Public API exports
├── console_writer.rs       # 250 lines - Console output implementation
└── performance_layer.rs    # 286 lines - Performance API implementation
```

## Module: lib.rs

The library entry point is minimal, re-exporting public types:

```rust
mod performance_layer;
pub use performance_layer::{
    performance_layer, FormatSpan, FormatSpanFromFields, PerformanceEventsLayer,
};

mod console_writer;
pub use console_writer::{ConsoleWriter, MakeConsoleWriter, MakeWebConsoleWriter};
```

**Design Decision:** Uses private modules with selective public exports to control the public API surface.

## Module: console_writer.rs

### Public Types

```rust
pub struct MakeConsoleWriter;  // Legacy, discouraged
pub struct MakeWebConsoleWriter {
    use_pretty_label: bool,
}
pub struct ConsoleWriter {
    buffer: Vec<u8>,
    level: Level,
    log: LogDispatcher,
}
```

### Type-Level Dispatch Pattern

The code uses a sophisticated **type-level dispatch** pattern to avoid virtual function overhead:

```rust
// Step 1: Trait defining log behavior
trait LogImpl {
    fn log_simple(level: Level, msg: &str);
    fn log_pretty(level: Level, msg: &str);
}

// Step 2: Dummy types for each level
struct LogLevelTrace;
struct LogLevelDebug;
struct LogLevelInfo;
struct LogLevelWarn;
struct LogLevelError;
struct LogLevelFallback;

// Step 3: Macro generates impls
macro_rules! make_log_impl {
    ($T:ident {
        simple: $s:expr,
        pretty: { log: $p:expr, fmt: $f:expr, label_style: $l:expr }
    }) => {
        struct $T;
        impl LogImpl for $T {
            #[inline(always)]
            fn log_simple(_level: Level, msg: &str) {
                $s(&JsValue::from(msg));
            }
            #[inline(always)]
            fn log_pretty(_level: Level, msg: &str) {
                // CSS-styled console output
            }
        }
    };
}

// Step 4: Generate impls for each level
make_log_impl!(LogLevelTrace {
    simple: console::debug_1,
    pretty: {
        log: console::debug_4,
        fmt: "%cTRACE%c %s",
        label_style: "color: white; font-weight: bold; padding: 0 5px; background: #75507B;"
    }
});
// ... repeated for each level
```

**Why this pattern?**
1. **Zero-cost abstraction** - Function pointers resolved at compile time
2. **Code deduplication** - Macro generates repetitive boilerplate
3. **Inlining** - `#[inline(always)]` ensures no call overhead

### Style Selector Trait

```rust
trait LogImplStyle {
    fn get_dispatch<L: LogImpl>(&self) -> LogDispatcher;
}

struct SimpleStyle;
impl LogImplStyle for SimpleStyle {
    fn get_dispatch<L: LogImpl>(&self) -> LogDispatcher {
        L::log_simple  // Returns function pointer
    }
}

struct PrettyStyle;
impl LogImplStyle for PrettyStyle {
    fn get_dispatch<L: LogImpl>(&self) -> LogDispatcher {
        L::log_pretty
    }
}
```

### Runtime Dispatcher Selection

```rust
type LogDispatcher = fn(Level, &str);

fn select_dispatcher(style: impl LogImplStyle, level: Level) -> LogDispatcher {
    if level == Level::TRACE {
        style.get_dispatch::<LogLevelTrace>()
    } else if level == Level::DEBUG {
        style.get_dispatch::<LogLevelDebug>()
    } else if level == Level::INFO {
        style.get_dispatch::<LogLevelInfo>()
    } else if level == Level::WARN {
        style.get_dispatch::<LogLevelWarn>()
    } else if level == Level::ERROR {
        style.get_dispatch::<LogLevelError>()
    } else {
        style.get_dispatch::<LogLevelFallback>()
    }
}
```

**Result:** A single `fn` pointer that can be called without trait objects.

### MakeWriter Implementation

```rust
impl<'a> MakeWriter<'a> for MakeWebConsoleWriter {
    type Writer = ConsoleWriter;

    fn make_writer(&'a self) -> Self::Writer {
        ConsoleWriter {
            buffer: vec![],
            level: Level::TRACE, // Fallback when level unknown
            log: if self.use_pretty_label {
                PrettyStyle.get_dispatch::<LogLevelFallback>()
            } else {
                SimpleStyle.get_dispatch::<LogLevelFallback>()
            },
        }
    }

    fn make_writer_for(&'a self, meta: &tracing_core::Metadata<'_>) -> Self::Writer {
        let level = *meta.level();
        let log_fn = if self.use_pretty_label {
            select_dispatcher(PrettyStyle, level)
        } else {
            select_dispatcher(SimpleStyle, level)
        };
        ConsoleWriter {
            buffer: vec![],
            level,
            log: log_fn,
        }
    }
}
```

**Key Insight:** `make_writer_for` receives `Metadata` which includes the log level, enabling level-specific dispatch.

### ConsoleWriter Write Implementation

```rust
impl Write for ConsoleWriter {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.buffer.write(buf)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        // Nothing to do - flush happens on drop
        Ok(())
    }
}

impl Drop for ConsoleWriter {
    fn drop(&mut self) {
        let message = String::from_utf8_lossy(&self.buffer);
        (self.log)(self.level, message.as_ref());
    }
}
```

**Design Trade-off:**
- Pro: Compatible with any code expecting `Write`
- Con: UTF-8 decode then re-encode as UTF-16 for JS (noted in TODO comment)

### CSS Styling for Console Output

```rust
// Format string uses console styling specifiers
const FORMAT: &str = "%cTRACE%c %s";
//                    │ │    │
//                    │ │    └── Message
//                    │ └────── Reset style
//                    └──────── Label style

// Color scheme per level:
// TRACE: #75507B (purple)
// DEBUG: #3465A4 (blue)
// INFO:  #4E9A06 (green)
// WARN:  #C4A000 (yellow)
// ERROR: #CC0000 (red)
```

## Module: performance_layer.rs

### FFI Bindings

```rust
#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_name = _fakeGlobal)]
    type Global;

    #[wasm_bindgen()]
    type Performance;

    #[wasm_bindgen(static_method_of = Global, js_class = "globalThis", getter)]
    fn performance() -> Performance;

    #[wasm_bindgen(method, catch, js_name = "mark")]
    fn do_mark(this: &Performance, name: &str) -> Result<(), JsValue>;

    #[wasm_bindgen(method, catch, js_name = "mark")]
    fn do_mark_with_details(
        this: &Performance,
        name: &str,
        details: &JsValue,
    ) -> Result<(), JsValue>;

    #[wasm_bindgen(method, catch, js_name = "measure")]
    fn do_measure_with_start_mark_and_end_mark(
        this: &Performance,
        name: &str,
        start: &str,
        end: &str,
    ) -> Result<(), JsValue>;

    #[wasm_bindgen(method, catch, js_name = "measure")]
    fn do_measure_with_details(
        this: &Performance,
        name: &str,
        details: &JsValue,
    ) -> Result<(), JsValue>;
}
```

**Note:** Uses `catch` to convert JS exceptions into `Result`.

### Performance API Wrapper

```rust
impl Performance {
    fn mark(&self, name: &str) -> Result<(), JsValue> {
        self.do_mark(name)
    }

    fn mark_detailed(&self, name: &str, details: &str) -> Result<(), JsValue> {
        // Create { detail: details } JavaScript object
        let details_obj = Object::create(JsValue::NULL.unchecked_ref::<Object>());
        let detail_prop = JsString::from(wasm_bindgen::intern("detail"));
        Reflect::set(&details_obj, &detail_prop, &JsValue::from(details)).unwrap();
        self.do_mark_with_details(name, &details_obj)
    }

    fn measure(&self, name: &str, start: &str, end: &str) -> Result<(), JsValue> {
        self.do_measure_with_start_mark_and_end_mark(name, start, end)
    }

    fn measure_detailed(
        &self,
        name: &str,
        start: &str,
        end: &str,
        details: &str,
    ) -> Result<(), JsValue> {
        let details_obj = Object::create(JsValue::NULL.unchecked_ref::<Object>());
        let detail_prop = JsString::from(wasm_bindgen::intern("detail"));
        let start_prop = JsString::from(wasm_bindgen::intern("start"));
        let end_prop = JsString::from(wasm_bindgen::intern("end"));
        Reflect::set(&details_obj, &detail_prop, &JsValue::from(details)).unwrap();
        Reflect::set(&details_obj, &start_prop, &JsValue::from(start)).unwrap();
        Reflect::set(&details_obj, &end_prop, &JsValue::from(end)).unwrap();
        self.do_measure_with_details(name, &details_obj)
    }
}
```

**Optimization:** Uses `wasm_bindgen::intern()` for string literals to avoid repeated allocations.

### Thread-Local Performance Handle

```rust
thread_local! {
    static PERF: Performance = {
        let performance = Global::performance();
        assert!(!performance.is_undefined(), "browser seems to not support the Performance API");
        performance
    };
}
```

### PerformanceEventsLayer Structure

```rust
pub struct PerformanceEventsLayer<S, N = ()> {
    fmt_details: N,
    _inner: PhantomData<fn(S)>,
}
```

**Generics:**
- `S`: The subscriber type (must implement `Subscriber + LookupSpan`)
- `N`: The details formatter (implements `FormatSpan`)

### Layer Trait Implementation

```rust
impl<S, N> Layer<S> for PerformanceEventsLayer<S, N>
where
    S: Subscriber + for<'lookup> LookupSpan<'lookup>,
    N: FormatSpan,
{
    fn on_new_span(&self, attrs: &span::Attributes<'_>, span: &span::Id, ctx: Context<'_, S>) {
        let span = ctx.span(span).expect("can't find span, this is a bug");
        self.fmt_details.add_details(&mut span.extensions_mut(), attrs);
    }

    fn on_record(&self, span: &span::Id, values: &span::Record<'_>, ctx: Context<'_, S>) {
        let span = ctx.span(span).expect("can't find span, this is a bug");
        self.fmt_details.record_values(&mut span.extensions_mut(), values);

        let mark_name = self.span_record_name(&span);
        let _ = PERF.with(|p| {
            if let Some(details) = self.fmt_details.find_details(&span.extensions()) {
                p.mark_detailed(&mark_name, details)
            } else {
                p.mark(&mark_name)
            }
        });
    }

    fn on_enter(&self, span: &span::Id, ctx: Context<'_, S>) {
        let span = ctx.span(span).expect("can't find span, this is a bug");
        let mark_name = self.span_enter_name(&span);
        let _ = PERF.with(|p| {
            if let Some(details) = self.fmt_details.find_details(&span.extensions()) {
                p.mark_detailed(&mark_name, details)
            } else {
                p.mark(&mark_name)
            }
        });
    }

    fn on_exit(&self, span: &span::Id, ctx: Context<'_, S>) {
        let span = ctx.span(span).expect("can't find span, this is a bug");
        let mark_enter_name = self.span_enter_name(&span);
        let mark_exit_name = self.span_exit_name(&span);
        let mark_measure_name = self.span_measure_name(&span);

        let _ = PERF.with(|p| {
            if let Some(details) = self.fmt_details.find_details(&span.extensions()) {
                p.mark_detailed(&mark_exit_name, details)?;
                p.measure_detailed(&mark_measure_name, &mark_enter_name, &mark_exit_name, details)?;
            } else {
                p.mark(&mark_exit_name)?;
                p.measure(&mark_measure_name, &mark_enter_name, &mark_exit_name)?;
            }
            Result::<(), JsValue>::Ok(())
        });
    }

    fn on_id_change(&self, _: &span::Id, _: &span::Id, _ctx: Context<'_, S>) {
        web_sys::console::warn_1(&JsValue::from(
            "A span changed id, this is currently not supported"
        ));
        debug_assert!(false, "A span changed id, this is currently not supported");
    }
}
```

### Naming Convention

```rust
fn template_name(span: &SpanRef<'_, S>, event_name: &str) -> String {
    let span_id = span.id().into_u64();
    let name = span.metadata().name();
    format!("{name} [{span_id}]: {event_name}")
}

// Produces names like:
// "my_span [12345]: span-enter"
// "my_span [12345]: span-exit"
// "my_span [12345]: span-measure"
```

### FormatSpan Trait

```rust
pub trait FormatSpan: 'static {
    fn find_details<'ext>(&self, ext: &'ext Extensions<'_>) -> Option<&'ext str>;
    fn add_details(&self, ext: &mut ExtensionsMut<'_>, attrs: &span::Attributes<'_>);
    fn record_values(&self, ext: &mut ExtensionsMut<'_>, values: &span::Record<'_>);
}
```

**No-op implementation:**
```rust
impl FormatSpan for () {
    fn find_details<'ext>(&self, _: &'ext Extensions<'_>) -> Option<&'ext str> {
        None
    }
    fn add_details(&self, _: &mut ExtensionsMut<'_>, _: &span::Attributes<'_>) {}
    fn record_values(&self, _: &mut ExtensionsMut<'_>, _: &span::Record<'_>) {}
}
```

### FormatSpanFromFields Adapter

```rust
pub struct FormatSpanFromFields<N> {
    inner: N,
}

impl<N> FormatSpan for FormatSpanFromFields<N>
where
    N: 'static + for<'writer> FormatFields<'writer>,
{
    fn find_details<'ext>(&self, ext: &'ext Extensions<'_>) -> Option<&'ext str> {
        let fields = ext.get::<FormattedFields<N>>()?;
        Some(&fields.fields)
    }

    fn add_details(&self, ext: &mut ExtensionsMut<'_>, attrs: &span::Attributes<'_>) {
        self.add_formatted_fields(ext, attrs);
    }

    fn record_values(&self, ext: &mut ExtensionsMut<'_>, values: &span::Record<'_>) {
        if let Some(fields) = ext.get_mut::<FormattedFields<N>>() {
            let _ = self.inner.add_fields(fields, values);
        } else {
            self.add_formatted_fields(ext, values);
        }
    }
}
```

**Key Optimization:** Reuses existing `FormattedFields` extension if present, avoiding duplicate formatting work.

## Dependency Graph

```
tracing-web
├── js-sys 0.3.59
│   └── wasm-bindgen
├── tracing-core 0.1.29
├── tracing-subscriber 0.3.15
│   ├── tracing-core
│   └── tracing-log (optional)
├── wasm-bindgen 0.2.82
└── web-sys 0.3.59
    ├── js-sys
    └── wasm-bindgen
```

## Key Algorithms

### 1. Level-Based Dispatcher Selection

```
Input: Level, Style
       │
       ▼
    ┌─────────────────┐
    │ level == TRACE? │───yes───► LogLevelTrace::log_*
    └─────────────────┘
           │ no
           ▼
    ┌─────────────────┐
    │ level == DEBUG? │───yes───► LogLevelDebug::log_*
    └─────────────────┘
           │ no
           ▼
    ┌─────────────────┐
    │ level == INFO?  │───yes───► LogLevelInfo::log_*
    └─────────────────┘
           │ no
           ▼
    ┌─────────────────┐
    │ level == WARN?  │───yes───► LogLevelWarn::log_*
    └─────────────────┘
           │ no
           ▼
    ┌─────────────────┐
    │ level == ERROR? │───yes───► LogLevelError::log_*
    └─────────────────┘
           │ no
           ▼
       LogLevelFallback::log_*
```

### 2. Span Extensions Storage

```
Registry
   │
   ▼
SpanData
   │
   └── Extensions (type-map)
          │
          ├── FormattedFields<Pretty> (if fmt::layer used)
          │     └── fields: "user_id=123 action=login"
          │
          └── (tracing-web can add custom types)
```

### 3. Performance Measure Creation

```
on_exit() called
    │
    ▼
1. mark("span-exit")
    │
    ▼
2. measure("span-measure", "span-enter", "span-exit")
    │
    ├── Creates visual bar in Performance tab
    │   showing span duration
    │
    └── Can be filtered/searched by name
```

## Error Handling Strategy

```rust
// Pattern 1: Silent failure for best-effort operations
let _ = PERF.with(|p| {
    p.mark(&name)
});

// Pattern 2: Result propagation within closure
let _ = PERF.with(|p| {
    p.mark_detailed(&mark_exit_name, details)?;
    p.measure_detailed(...)?;
    Result::<(), JsValue>::Ok(())
});

// Pattern 3: Debug assertion for unexpected states
fn on_id_change(&self, ...) {
    web_sys::console::warn_1(&JsValue::from("not supported"));
    debug_assert!(false, "span ID change not supported");
}
```

**Rationale:** Browser DevTools are debugging infrastructure - failures shouldn't crash production code.

## Memory Patterns

### Interned Strings

```rust
// String literals are interned to avoid repeated allocations
let fmt = JsValue::from(wasm_bindgen::intern("%cTRACE%c %s"));
let label_style = JsValue::from(wasm_bindgen::intern("color: white; ..."));
```

### Buffer Reuse

```rust
// ConsoleWriter allocates fresh buffer each time
// This is necessary because MakeWriter can be called concurrently
ConsoleWriter {
    buffer: vec![],  // New allocation
    level,
    log,
}
```

### PhantomData Usage

```rust
pub struct PerformanceEventsLayer<S, N = ()> {
    fmt_details: N,
    _inner: PhantomData<fn(S)>,  // Variance marker
}
```

**Purpose:** `PhantomData<fn(S)>` makes `S` contravariant, which is correct for a consumer of `S`.

## Testing Considerations

The code uses `debug_assert!` for development-time checking:

```rust
debug_assert!(false, "A span changed id, this is currently not supported");
```

This assertion only fires in debug builds, avoiding production overhead.

## See Also

- [Architecture](./architecture.md) - High-level design
- [tracing Ecosystem](./tracing-ecosystem.md) - Dependency context
- [Rust Replication Plan](./rust-revision.md) - Building similar libraries
