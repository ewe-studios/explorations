---
location: /home/darkvoid/Boxxed/@formulas/src.rust/src.wasm/src.stdweb
repository: https://github.com/koute/stdweb
explored_at: 2026-03-22
language: Rust, JavaScript
---

# Project Exploration: stdweb

## Overview

**stdweb** is a standard library for the client-side Web. It provides Rust bindings to Web APIs and enables high interoperability between Rust and JavaScript. The project was created by Piotr Czarnecki (Koute) and aimed to be a comprehensive foundation for building web applications in Rust.

### Key Value Proposition

- **JavaScript embedding** - Direct inline JavaScript in Rust code via `js!` macro
- **Full Web API coverage** - Bindings for DOM, events, XHR, WebSocket, Canvas, WebGL, and more
- **Multiple backends** - Supports native wasm32-unknown-unknown, Emscripten, and asm.js
- **Cargo-web integration** - Dedicated cargo subcommand for building, testing, and serving
- **Closures and callbacks** - Full support for passing closures between Rust and JavaScript
- **Serde integration** - Serialize Rust structures for JavaScript consumption

### Example Usage

```rust
// Embed JavaScript directly in Rust
let message = "Hello, 世界!";
let result = js! {
    alert( @{message} );
    return 2 + 2 * 2;
};
println!( "2 + 2 * 2 = {:?}", result );

// Closures are supported
let print_hello = |name: String| {
    println!( "Hello, {}!", name );
};

js! {
    var print_hello = @{print_hello};
    print_hello( "Bob" );
    print_hello.drop(); // Must call drop() to free closure
}

// Pass structures via serde
#[derive(Serialize)]
struct Person {
    name: String,
    age: i32
}

js_serializable!( Person );

js! {
    var person = @{person};
    console.log( person.name + " is " + person.age + " years old." );
}

// Use Web APIs
let button = document().query_selector( "#hide-button" ).unwrap().unwrap();
button.add_event_listener( move |_: ClickEvent| {
    for anchor in document().query_selector_all( "#main a" ) {
        js!( @{anchor}.style = "display: none;"; );
    }
});

// Export Rust functions to JavaScript
#[js_export]
fn hash( string: String ) -> String {
    let mut hasher = Sha1::new();
    hasher.update( string.as_bytes() );
    hasher.digest().to_string()
}
```

## Repository Structure

```
/home/darkvoid/Boxxed/@formulas/src.rust/src.wasm/src.stdweb/
├── stdweb/                          # Main crate with Web API bindings
│   ├── src/
│   │   ├── lib.rs                   # Main entry point
│   │   ├── webcore/                 # Core infrastructure
│   │   │   ├── macros.rs            # js! macro implementation
│   │   │   ├── value.rs             # JavaScript Value type
│   │   │   ├── reference_type.rs    # Reference type trait
│   │   │   ├── serialization.rs     # JavaScript serialization
│   │   │   ├── callfn.rs            # Function calling infrastructure
│   │   │   ├── ffi/                 # FFI layer (emscripten, wasm, wasm-bindgen)
│   │   │   ├── promise.rs           # JavaScript Promise wrapper
│   │   │   ├── executor.rs          # Futures executor
│   │   │   └── ...
│   │   ├── webapi/                  # Web API bindings
│   │   │   ├── document.rs          # Document API
│   │   │   ├── window.rs            # Window API
│   │   │   ├── element.rs           # Element API
│   │   │   ├── node.rs              # Node API
│   │   │   ├── events/              # Event types
│   │   │   │   ├── mouse.rs         # Mouse events
│   │   │   │   ├── keyboard.rs      # Keyboard events
│   │   │   │   ├── touch.rs         # Touch events
│   │   │   │   ├── drag.rs          # Drag events
│   │   │   │   └── ...
│   │   │   ├── html_elements/       # HTML element types
│   │   │   ├── rendering_context.rs # Canvas rendering context
│   │   │   ├── web_socket.rs        # WebSocket API
│   │   │   ├── xml_http_request.rs  # XHR API
│   │   │   ├── mutation_observer.rs # MutationObserver API
│   │   │   ├── gamepad.rs           # Gamepad API
│   │   │   └── ...
│   │   └── ecosystem/               # Ecosystem integrations
│   │       ├── serde.rs             # Serde integration
│   │       └── serde_json.rs        # Serde JSON integration
│   ├── examples/
│   │   ├── minimal/                 # Minimal alert example
│   │   ├── todomvc/                 # TodoMVC application
│   │   ├── hasher/                  # Export Rust to JS example
│   │   ├── canvas/                  # Canvas API example
│   │   ├── webgl/                   # WebGL example
│   │   └── ...
│   ├── stdweb-derive/               # Procedural macros
│   ├── stdweb-internal-runtime/     # Runtime support
│   ├── stdweb-internal-macros/      # Internal macros
│   └── Cargo.toml
│
├── cargo-web/                       # Cargo subcommand
│   ├── src/
│   │   ├── lib.rs                   # Main library
│   │   ├── main.rs                  # CLI entry point
│   │   ├── cmd_build.rs             # Build command
│   │   ├── cmd_start.rs             # Start dev server command
│   │   ├── cmd_test.rs              # Test command
│   │   ├── cmd_deploy.rs            # Deploy command
│   │   ├── wasm_runtime.rs          # WASM runtime generation
│   │   ├── wasm_js_export.rs        # JS export handling
│   │   ├── wasm_js_snippet.rs       # JS snippet handling
│   │   ├── chrome_devtools.rs       # Chrome DevTools integration
│   │   └── ...
│   ├── test-crates/                 # Test fixtures
│   └── Cargo.toml
│
├── embed-wasm/                      # Embed WASM in native binaries
│   ├── embed-wasm/                  # Runtime library
│   ├── embed-wasm-build/            # Build-time library
│   └── README.md
│
├── speedy/                          # Fast binary serialization
│   ├── speedy/                      # Main crate
│   ├── speedy-derive/               # Derive macros
│   └── Cargo.toml
│
├── picoalloc/                       # Memory allocator
│   └── Cargo.toml
│
├── mnestic/                         # Markdown to HTML via WASM
│   ├── mnemnos-wasm/                # WASM module
│   ├── mnemnos-worker/              # Cloudflare worker
│   └── mnemnos-types/               # Shared types
│
├── recursion/                       # Recursive tree visualization
│   └── Cargo.toml
│
├── object/                          # Object handling library
│   └── Cargo.toml
│
└── tracing-honeycomb/               # Distributed tracing
    ├── tracing-honeycomb/
    ├── tracing-distributed/
    └── tracing-jaeger/
```

## Core Concepts

### 1. The js! Macro

The `js!` macro is the heart of stdweb, allowing inline JavaScript code:

```rust
// Basic usage
let result = js! {
    return Math.sqrt( 16 );
};

// Passing Rust values to JavaScript
let name = "Alice";
js! {
    console.log( "Hello, " + @{name} );
}

// Receiving JavaScript values
let value: Value = js! {
    return document.querySelector( "#main" );
};

// Type-safe conversion
let element: Element = js! {
    return document.querySelector( "#main" );
}.try_into().unwrap();

// No return value (optimized)
js! {
    console.log( "Hello" );
}
```

**How it works:**

```rust
// Macro expands to something like:
{
    let restore_point = ArenaRestorePoint::new();

    // Serialize arguments
    let arg_name = $crate::private::JsSerializeOwned::into_js_owned( &mut arg );

    // Call JavaScript via FFI
    let result = unsafe {
        snippet( arg_ptr );  // Calls into JavaScript runtime
    };

    result.deserialize()
}
```

### 2. JavaScript Value Types

```rust
use stdweb::Value;

// All JavaScript values are represented as:
enum Value {
    Undefined,
    Null,
    Bool( bool ),
    Number( Number ),
    String( String ),
    Reference( Reference ),
}

// References wrap JavaScript objects
let ref: Reference = js! { return {}; };

// Convert to specific types
let string: String = js! { return "hello"; }.try_into().unwrap();
let number: f64 = js! { return 42.0; }.try_into().unwrap();
```

### 3. Closure Handling

```rust
// FnOnce closures - called once then consumed
let once = move || {
    println!( "Called once" );
};
js! {
    var once = @{once};
    once();  // Called, closure consumed
}

// FnMut closures - can be called multiple times
let mut counter = 0;
let mut_cb = move || {
    counter += 1;
    println!( "Count: {}", counter );
};
js! {
    var mut_cb = @{mut_cb};
    mut_cb();
    mut_cb();
    mut_cb.drop();  // Must explicitly drop!
}

// Using Mut wrapper (safer, prevents recursive calls)
use stdweb::Mut;
let mut_cb = Mut::new( move || { ... } );
js! {
    var cb = @{mut_cb};
    cb();
}
```

**Memory management:**

```rust
// Closures passed to JavaScript LEAK unless dropped!
js! {
    var leaky = @{|| println!( "leak" )};
    // Never dropped = memory leak!
}

// Correct pattern:
js! {
    var safe = @{|| println!( "no leak" )};
    safe.drop();  // Cleans up Rust closure
}
```

### 4. Exporting Rust to JavaScript

```rust
use stdweb::js_export;

#[js_export]
fn add( a: i32, b: i32 ) -> i32 {
    a + b
}

#[js_export]
fn greet( name: String ) -> String {
    format!( "Hello, {}!", name )
}
```

**Generated JavaScript:**

```js
// Called from JavaScript
Module.add( 1, 2 ).then( result => {
    console.log( result );  // 3
});

Module.greet( "Alice" ).then( name => {
    console.log( name );  // "Hello, Alice!"
});
```

### 5. Event Handling

```rust
use stdweb::web::{
    document,
    ClickEvent,
    IEventTarget
};

let button = document().query_selector( "#btn" ).unwrap().unwrap();

// Add event listener
button.add_event_listener( move |event: ClickEvent| {
    event.prevent_default();
    console_log!( "Button clicked!" );
});

// Remove event listener (via guard pattern)
let _listener = button.add_event_listener( move |_: ClickEvent| {
    // ...
});
// Listener removed when _listener goes out of scope
```

### 6. DOM Manipulation

```rust
use stdweb::web::{
    document,
    Element,
    INode,
    IElement,
    IParentNode
};

// Query elements
let main = document().query_selector( "#main" )
    .unwrap()
    .expect( "no #main element" );

// Create elements
let div = document().create_element( "div" ).unwrap();
div.set_text_content( "Hello, World!" );

// Manipulate attributes
div.set_attribute( "class", "container" ).unwrap();
let class = div.get_attribute( "class" ).unwrap();

// Append to DOM
document().body().unwrap().append_child( &div ).unwrap();

// Query all
let anchors = document().query_selector_all( "#main a" );
for anchor in anchors {
    // Process each anchor
}
```

## Cargo-web CLI

### Installation

```bash
cargo install cargo-web
```

### Commands

```bash
# Build project
cargo web build --target=wasm32-unknown-unknown

# Typecheck only
cargo web check

# Start dev server with auto-reload
cargo web start --auto-reload --open

# Run tests in headless Chrome
cargo web test

# Deploy for production
cargo web deploy --release

# Prepare Emscripten
cargo web prepare-emscripten
```

### Web.toml Configuration

```toml
# Web.toml next to Cargo.toml

# Default target
default-target = "wasm32-unknown-unknown"

# Prepend JavaScript to output
prepend-js = "src/runtime.js"

[cargo-web]
minimum-version = "0.6.0"

[target.wasm32-unknown-unknown]
prepend-js = "src/native_runtime.js"
```

### Runtime Options

```bash
# Standalone runtime (default)
cargo web build --runtime standalone

# Library ES6 module (for bundlers)
cargo web build --runtime library-es6

# Only loader (experimental)
cargo web build --runtime experimental-only-loader
```

**Library ES6 usage:**

```js
import factory from "./my-module.mjs";
import fs from "fs";

const bytecode = fs.readFileSync( "my-module.wasm" );
const wasm = new WebAssembly.Module( bytecode );

const instance = factory();
const compiled = new WebAssembly.Instance( wasm, instance.imports );
const exports = instance.initialize( compiled );

console.log( exports.add( 1, 2 ) );
```

## WASM Runtime Generation

Cargo-web generates a JavaScript runtime for WASM modules:

```rust
// cargo-web/src/wasm_runtime.rs

pub enum RuntimeKind {
    Standalone,      // Self-contained, works immediately
    LibraryEs6,      // ES6 module for bundlers
    WebExtension,    // For browser extensions
    OnlyLoader       // Minimal loader
}

pub fn generate_js(
    runtime: RuntimeKind,
    main_symbol: Option<String>,
    wasm_path: &Path,
    prepend_js: &str,
    snippets: &[JsSnippet],
    exports: &[JsExport]
) -> String {
    // Generates JavaScript runtime code
    // Templates: STANDALONE_TEMPLATE, LIBRARY_ES6_TEMPLATE, etc.
}
```

**Runtime responsibilities:**

1. **Memory management** - Handle WASM linear memory
2. **Import resolution** - Wire up JavaScript imports
3. **Export wrapping** - Wrap WASM exports with type conversions
4. **Snippet execution** - Execute inline JavaScript from `js!` macros
5. **Event loop** - Run Rust event loop in browser

## embed-wasm: Embedding WASM in Native Binaries

The `embed-wasm` crate allows embedding WASM build output in native Rust binaries:

### Build Script (build.rs)

```rust
use embed_wasm_build::compile_wasm;

fn main() {
    compile_wasm( "wasm" );  // Path to wasm Cargo project
}
```

### Runtime Library

```rust
use embed_wasm::{StaticLookup, IndexHandling, include_wasm};

// Include generated WASM blobs
include_wasm!();

// Serve static content
fn serve(path: &str) -> Option<Response<Body>> {
    STATIC_LOOKUP.get( path )
}
```

**How it works:**

```
┌─────────────────────────────────────────────────────────────┐
│                    Build Process                              │
│                                                               │
│  1. build.rs calls compile_wasm("wasm")                       │
│  2. cargo-web deploys WASM project                            │
│  3. All output files embedded as static bytes                 │
│  4. PHF (Perfect Hash Function) map generated                 │
│  5. Binary includes all WASM artifacts                        │
└─────────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────────┐
│                    Runtime Access                             │
│                                                               │
│  Native Binary                                                │
│    └── StaticLookup                                           │
│         └── WASM: phf::Map<&str, &[u8]>                       │
│              ├── "index.html" -> [bytes]                      │
│              ├── "app.wasm" -> [bytes]                        │
│              ├── "style.css" -> [bytes]                       │
│              └── ...                                          │
└─────────────────────────────────────────────────────────────┘
```

## speedy: Fast Binary Serialization

A companion crate for fast binary serialization:

```rust
use speedy::{Readable, Writable, Endianness};

#[derive(PartialEq, Debug, Readable, Writable)]
enum Enum {
    A,
    B,
    C,
}

#[derive(PartialEq, Debug, Readable, Writable)]
struct Struct<'a> {
    number: u64,
    string: String,
    vector: Vec<u8>,
    cow: Cow<'a, [i64]>,
    float: f32,
    enumeration: Enum
}

fn main() {
    let original = Struct { /* ... */ };

    // Serialize
    let bytes = original.write_to_vec().unwrap();

    // Deserialize
    let deserialized: Struct = Struct::read_from_buffer( &bytes ).unwrap();

    assert_eq!( original, deserialized );
}
```

**Field attributes:**

```rust
#[derive(Readable, Writable)]
struct Struct {
    // Variable length
    #[speedy(length = byte_count / 4)]
    data: Vec<u32>,

    // Custom length type
    #[speedy(length_type = u16)]
    items: Vec<String>,

    // Varint encoding
    #[speedy(varint)]
    big_number: u64,

    // Skip field
    #[speedy(skip)]
    ignored: String,

    // Default on EOF
    #[speedy(default_on_eof)]
    optional_field: Option<u32>,
}
```

## Architecture

### stdweb Layer Diagram

```
┌─────────────────────────────────────────────────────────────┐
│                    User Rust Code                             │
│  js! { ... }, Web API calls, #[js_export]                    │
└─────────────────────────────────────────────────────────────┘
                              │
                              │ stdweb macros
                              ▼
┌─────────────────────────────────────────────────────────────┐
│                   stdweb-derive                               │
│  #[derive(ReferenceType)], #[js_export], #[async_test]       │
└─────────────────────────────────────────────────────────────┘
                              │
                              │ FFI calls
                              ▼
┌─────────────────────────────────────────────────────────────┐
│              stdweb-internal-runtime                          │
│  - Arena-based memory allocation                            │
│  - Reference tracking                                       │
│  - Serialization/deserialization                            │
│  - Event loop management                                    │
└─────────────────────────────────────────────────────────────┘
                              │
              ┌───────────────┼───────────────┐
              ▼               ▼               ▼
     ┌─────────────┐ ┌─────────────┐ ┌─────────────┐
     │  Emscripten │ │ Native WASM │ │ wasm-bindgen│
     │   (asm.js)  │ │  (cargo-web)│ │   (compat)  │
     └─────────────┘ └─────────────┘ └─────────────┘
```

### Memory Arena

```rust
// stdweb uses an arena allocator for JavaScript values

pub struct ArenaRestorePoint {
    // Saved arena offset
}

impl ArenaRestorePoint {
    pub fn new() -> Self {
        // Records current offset
    }
}

impl Drop for ArenaRestorePoint {
    fn drop(&mut self) {
        // Restores arena to saved offset
        // Frees all values allocated after creation
    }
}

// Usage in js! macro:
let restore_point = ArenaRestorePoint::new();
// ... serialize arguments, call JavaScript ...
// restore_point dropped = memory freed
```

### Reference Type System

```rust
pub trait ReferenceType: Sized {
    unsafe fn from_reference_unchecked( Reference ) -> Self;

    fn as_reference( &self ) -> &Reference;
}

// All Web API types implement ReferenceType
impl ReferenceType for Element { ... }
impl ReferenceType for Document { ... }
impl ReferenceType for Window { ... }

// Type conversions via TryFrom
impl<'a, 'b, T, U> TryFrom<&'a T> for &'b U
where
    T: ReferenceType,
    U: ReferenceType,
{
    // Dynamic type checking via instanceof
}
```

## Event System

### Event Types

```rust
// Mouse events
ClickEvent, MouseDownEvent, MouseUpEvent, MouseMoveEvent,
MouseOverEvent, MouseOutEvent, MouseEnterEvent, MouseLeaveEvent,
ContextMenuEvent, MouseWheelEvent

// Keyboard events
KeyDownEvent, KeyUpEvent, KeyPressEvent

// Touch events
TouchStartEvent, TouchMoveEvent, TouchEndEvent, TouchCancelEvent

// Drag events
DragStartEvent, DragEndEvent, DragEnterEvent, DragLeaveEvent,
DragOverEvent, DragDropEvent

// Focus events
FocusEvent, BlurEvent

// Resource events
ResourceLoadEvent, ResourceAbortEvent, ResourceErrorEvent

// Socket events
SocketOpenEvent, SocketCloseEvent, SocketMessageEvent, SocketErrorEvent
```

### Event Traits

```rust
pub trait IEvent: ReferenceType {
    fn type_name( &self ) -> String;
    fn prevent_default( &self );
    fn stop_propagation( &self );
    fn stop_immediate_propagation( &self );
}

pub trait IMouseEvent: IEvent {
    fn client_x( &self ) -> i32;
    fn client_y( &self ) -> i32;
    fn button( &self ) -> MouseButton;
}

// Usage:
button.add_event_listener( |event: ClickEvent| {
    event.prevent_default();
    println!( "Clicked at: ({}, {})", event.client_x(), event.client_y() );
});
```

## Web API Coverage

### Document & DOM

```rust
use stdweb::web::{document, Document, IDocument};

// Document access
let doc = document();
let body = doc.body().unwrap();
let head = doc.head().unwrap();

// Element creation
let div = doc.create_element( "div" ).unwrap();
let text = doc.create_text_node( "Hello" );

// Query methods
let by_id = doc.get_element_by_id( "main" );
let by_selector = doc.query_selector( ".class" ).unwrap();
let all = doc.query_selector_all( "a" );
```

### Window & Location

```rust
use stdweb::web::{window, Window, IWindowOrWorker};

let win = window();

// Timers
let timeout_id = win.set_timeout( 1000, || {
    println!( "Timeout!" );
});
win.clear_timeout( timeout_id );

let interval_id = win.set_interval( 500, || {
    println!( "Tick" );
});
win.clear_interval( interval_id );

// Location
let location = win.location();
let href = location.href().unwrap();
location.set_href( "https://example.com" ).unwrap();
```

### Canvas

```rust
use stdweb::web::{
    document, CanvasElement,
    ICanvasRenderingContext2D, CompositeOperation
};

let canvas: CanvasElement = document()
    .query_selector( "#canvas" )
    .unwrap()
    .unwrap();

let ctx = canvas.get_context::<CanvasRenderingContext2d>()
    .unwrap()
    .unwrap();

// Drawing
ctx.fill_rect( 10.0, 10.0, 100.0, 100.0 );
ctx.set_fill_style( &"red" );
ctx.set_font( "16px Arial" );
ctx.fill_text( "Hello", 20.0, 50.0, None );

// Images
let image = document().create_element::<HtmlImageElement>().unwrap();
image.set_src( "image.png" );
ctx.draw_image( &image, 0.0, 0.0 ).unwrap();
```

### XMLHttpRequest

```rust
use stdweb::web::{XmlHttpRequest, IEventTarget};

let xhr = XmlHttpRequest::new().unwrap();
xhr.open_with_async( HttpMethod::Get, "/api/data", true ).unwrap();

xhr.add_event_listener( move |_: ResourceLoadEvent| {
    let text = xhr.response_text().unwrap();
    println!( "Response: {}", text );
});

xhr.send( None ).unwrap();
```

### WebSocket

```rust
use stdweb::web::{WebSocket, SocketBinaryType, IEventTarget};

let ws = WebSocket::new( "ws://localhost:8080" ).unwrap();
ws.set_binary_type( SocketBinaryType::ArrayBuffer );

ws.add_event_listener( |_: SocketOpenEvent| {
    println!( "Connected!" );
});

ws.add_event_listener( |event: SocketMessageEvent| {
    match event.data() {
        SocketMessageData::String( text ) => println!( "Text: {}", text ),
        SocketMessageData::Binary( data ) => println!( "Binary: {:?}", data ),
    }
});

ws.send_text( "Hello Server" ).unwrap();
```

## Comparison with wasm-bindgen

| Aspect | stdweb | wasm-bindgen |
|--------|--------|--------------|
| JavaScript embedding | `js!` macro (inline) | `#[wasm_bindgen]` extern blocks |
| Runtime | Arena-based, manual cleanup | Automatic via bindgen runtime |
| Closure handling | Manual `.drop()` required | Automatic lifetime management |
| Backend support | Native WASM, Emscripten, asm.js | Native WASM only |
| CLI tool | cargo-web | wasm-pack |
| Web API coverage | Comprehensive built-in | Generated via web-sys |
| Design philosophy | JavaScript-first ergonomics | Rust-first type safety |
| Closure leaks | Possible if not dropped | Prevented by design |
| Maturity | Legacy (maintenance mode) | Actively developed |

## Trade-offs

| Design Choice | Benefit | Cost |
|---------------|---------|------|
| Inline JavaScript | Natural JS syntax, easy debugging | No compile-time JS checking, string parsing |
| Arena allocation | Fast allocation, bulk deallocation | Memory leaks if restore points mismanaged |
| Manual closure drop | Explicit control | Easy to forget, causes leaks |
| Reference type system | Dynamic instanceof checks | Runtime overhead, potential panics |
| Multiple backends | Flexibility (Emscripten, WASM) | Complex runtime, larger codebase |
| js_serializable! macro | Easy serde integration | Additional macro complexity |

## Historical Context

**Timeline:**

- **2017** - stdweb created as early WebAssembly/Rust web framework
- **2018** - wasm-bindgen announced by Mozilla
- **2019-2020** - stdweb development slows, wasm-bindgen becomes dominant
- **2021+** - stdweb in maintenance mode

**Why wasm-bindgen won:**

1. **Mozilla backing** - Official Rust/WASM working group support
2. **Better type safety** - Compile-time bindings vs runtime checks
3. **Cleaner memory model** - No manual closure management
4. **web-sys** - Auto-generated bindings from WebIDL
5. **wasm-pack** - Polished tooling and publishing workflow

**stdweb's innovations:**

- First to support native `wasm32-unknown-unknown` target
- `js!` macro influenced similar patterns in other projects
- Early futures/async support for web Rust
- Pioneered closure FFI patterns

## Related Projects

### In this Repository

- **cargo-web** - Build tool for stdweb projects
- **speedy** - Fast binary serialization library
- **picoalloc** - Minimal memory allocator
- **embed-wasm** - Embed WASM in native binaries
- **mnestic** - Markdown to HTML via WASM workers
- **tracing-honeycomb** - Distributed tracing

### External

- **wasm-bindgen** - Modern alternative to stdweb
- **yew** - React-like framework (originally stdweb-based)
- **seed** - Elm-inspired web framework
- **Draco** - stdweb-based game engine

## References

- [stdweb Documentation](https://docs.rs/stdweb/)
- [stdweb GitHub](https://github.com/koute/stdweb)
- [cargo-web GitHub](https://github.com/koute/cargo-web)
- [Patreon Support](https://www.patreon.com/koute)
