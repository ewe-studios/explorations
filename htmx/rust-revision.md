---
source: /home/darkvoid/Boxxed/@formulas/src.UIFrameworks/src.HTMX/htmx
repository: git@github.com:bigskysoftware/htmx.git
revised_at: 2026-04-04
workspace: htmx-rs
---

# Rust Revision: HTMX in Rust/WASM

## Overview

This document describes how to translate HTMX's JavaScript functionality into a Rust/WebAssembly implementation. The goal is to create a drop-in replacement that provides the same hypermedia-driven web development experience while leveraging Rust's type safety, zero-cost abstractions, and WASM's near-native performance.

### Why Rust for HTMX?

1. **Type Safety**: Catch attribute parsing errors at compile time
2. **Performance**: Faster DOM manipulation through WASM
3. **Memory Safety**: No garbage collection pauses
4. **Smaller Bundle**: Tree-shaking and dead code elimination
5. **Concurrency**: Web Workers for background processing

## Workspace Structure

```
htmx-rs/
├── Cargo.toml                 # Workspace definition
├── htmx-core/                 # Core library (platform-agnostic)
│   ├── Cargo.toml
│   └── src/
│       ├── lib.rs
│       ├── attributes.rs      # Attribute parsing and validation
│       ├── ajax.rs            # AJAX request engine
│       ├── swap.rs            # DOM swapping strategies
│       ├── trigger.rs         # Event trigger system
│       ├── events.rs          # Event system
│       └── types.rs           # Core type definitions
├── htmx-wasm/                 # WASM browser bindings
│   ├── Cargo.toml
│   └── src/
│       ├── lib.rs
│       ├── dom.rs             # DOM manipulation
│       ├── xhr.rs             # XMLHttpRequest wrapper
│       ├── fetch.rs           # Fetch API wrapper
│       └── events.rs          # Browser event bindings
├── htmx-extensions/           # Official extensions
│   ├── Cargo.toml
│   └── src/
│       ├── lib.rs
│       ├── ws.rs              # WebSocket extension
│       ├── sse.rs             # SSE extension
│       ├── json_enc.rs        # JSON encoding
│       └── path_deps.rs       # Path dependencies
└── htmx-cli/                  # Development CLI
    ├── Cargo.toml
    └── src/
        ├── main.rs
        └── commands/
            ├── build.rs
            └── serve.rs
```

## Crate Breakdown

### htmx-core

**Purpose:** Platform-agnostic core logic

**Type:** Library

**Public API:**
```rust
pub use attributes::{AttributeParser, HxAttributes, SwapStyle};
pub use ajax::{AjaxEngine, RequestConfig, Response};
pub use trigger::{TriggerSpec, TriggerType};
pub use events::{EventBus, HtmxEvent};
pub use types::{HtmxConfig, HtmxResult};
```

**Dependencies:**
- `serde` - Serialization
- `thiserror` - Error handling
- `log` - Logging

### htmx-wasm

**Purpose:** Browser WASM bindings

**Type:** Library (cdylib for WASM)

**Public API:**
```rust
#[wasm_bindgen]
pub fn init() -> Result<(), JsValue>;

#[wasm_bindgen]
pub fn process(element: Element) -> Result<(), JsValue>;

#[wasm_bindgen]
pub fn ajax(
    verb: &str,
    path: &str,
    target: Element,
) -> Result<(), JsValue>;
```

**Dependencies:**
- `wasm-bindgen` - WASM bindings
- `web-sys` - Web APIs
- `js-sys` - JavaScript types
- `htmx-core` - Core library

### htmx-extensions

**Purpose:** Official HTMX extensions

**Type:** Library

**Dependencies:**
- `htmx-core`
- `wasm-bindgen`
- `serde_json`

## Type System Design

### Core Types

```rust
/// HTMX attribute configuration
#[derive(Debug, Clone)]
pub struct HxAttributes {
    /// AJAX verb (get, post, put, patch, delete)
    pub verb: Option<AjaxVerb>,
    /// Request path
    pub path: Option<String>,
    /// Target selector
    pub target: Option<TargetSelector>,
    /// Swap strategy
    pub swap: SwapSpec,
    /// Trigger specification
    pub trigger: Vec<TriggerSpec>,
    /// Include selector
    pub include: Option<IncludeSpec>,
    /// Confirmation message
    pub confirm: Option<String>,
    /// Indicator selector
    pub indicator: Option<String>,
    /// Custom headers
    pub headers: HashMap<String, String>,
    /// Custom variables
    pub vars: HashMap<String, JsValue>,
}

/// AJAX verb types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AjaxVerb {
    Get,
    Post,
    Put,
    Patch,
    Delete,
}

/// Target selector types
#[derive(Debug, Clone)]
pub enum TargetSelector {
    /// CSS selector
    Css(String),
    /// Target self
    This,
    /// Target closest parent matching selector
    Closest(String),
    /// Target next sibling matching selector
    Next(String),
    /// Target previous sibling matching selector
    Previous(String),
    /// Target body
    Body,
}

/// Swap specification
#[derive(Debug, Clone)]
pub struct SwapSpec {
    /// Swap style
    pub style: SwapStyle,
    /// Delay before swap (ms)
    pub swap_delay: Option<u32>,
    /// Delay before settle (ms)
    pub settle_delay: Option<u32>,
    /// Enable CSS transitions
    pub transition: bool,
    /// Ignore title element
    pub ignore_title: bool,
    /// Scroll behavior
    pub scroll: Option<ScrollPosition>,
    /// Show behavior
    pub show: Option<ShowPosition>,
}

/// Swap styles
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SwapStyle {
    InnerHtml,
    OuterHtml,
    BeforeBegin,
    AfterBegin,
    BeforeEnd,
    AfterEnd,
    Delete,
    None,
}

/// Trigger specification
#[derive(Debug, Clone)]
pub struct TriggerSpec {
    /// Event type
    pub event: TriggerType,
    /// Only if value changed
    pub changed: bool,
    /// Fire only once
    pub once: bool,
    /// Consume event
    pub consume: bool,
    /// Debounce delay (ms)
    pub delay: Option<u32>,
    /// Throttle delay (ms)
    pub throttle: Option<u32>,
    /// Listen on different element
    pub from: Option<String>,
    /// Only if target matches
    pub target: Option<String>,
}

/// Trigger types
#[derive(Debug, Clone)]
pub enum TriggerType {
    /// Standard DOM event
    DomEvent(String),
    /// Element revealed (scroll into view)
    Revealed,
    /// Page load
    Load,
    /// Polling interval (ms)
    Every(u32),
}

/// Include specification
#[derive(Debug, Clone)]
pub enum IncludeSpec {
    /// All form inputs
    All,
    /// No additional inputs
    None,
    /// Specific selector
    Selector(String),
    /// Specific parameter names
    Params(Vec<String>),
}
```

### Error Types

```rust
use thiserror::Error;

#[derive(Error, Debug)]
pub enum HtmxError {
    #[error("Invalid attribute syntax: {0}")]
    InvalidAttribute(String),
    
    #[error("Unknown swap style: {0}")]
    UnknownSwapStyle(String),
    
    #[error("Target element not found: {0}")]
    TargetNotFound(String),
    
    #[error("Invalid trigger specification: {0}")]
    InvalidTrigger(String),
    
    #[error("Request failed with status {0}: {1}")]
    RequestFailed(u16, String),
    
    #[error("Network error: {0}")]
    NetworkError(String),
    
    #[error("Parse error: {0}")]
    ParseError(String),
    
    #[error("WASM error: {0}")]
    WasmError(String),
}

pub type HtmxResult<T> = Result<T, HtmxError>;
```

### Request/Response Types

```rust
/// Request configuration
#[derive(Debug, Clone)]
pub struct RequestConfig {
    pub verb: AjaxVerb,
    pub path: String,
    pub headers: HashMap<String, String>,
    pub parameters: HashMap<String, String>,
    pub target: TargetSelector,
    pub swap: SwapSpec,
}

/// Response structure
#[derive(Debug)]
pub struct Response {
    pub status: u16,
    pub status_text: String,
    pub headers: HashMap<String, String>,
    pub body: String,
}

impl Response {
    /// Get header value
    pub fn header(&self, name: &str) -> Option<&String> {
        self.headers.get(name)
    }
    
    /// Check if redirect
    pub fn is_redirect(&self) -> bool {
        self.header("HX-Redirect").is_some()
    }
    
    /// Get redirect URL
    pub fn redirect_url(&self) -> Option<&String> {
        self.header("HX-Redirect")
    }
    
    /// Get new target selector
    pub fn retarget(&self) -> Option<&String> {
        self.header("HX-Retarget")
    }
    
    /// Get new swap style
    pub fn reswap(&self) -> Option<&String> {
        self.header("HX-Reswap")
    }
}
```

## Attribute Parser

```rust
/// Parse HTMX attributes from an element
pub struct AttributeParser {
    // Configuration
}

impl AttributeParser {
    pub fn new() -> Self {
        Self {}
    }
    
    /// Parse all hx-* attributes from element
    pub fn parse(&self, element: &Element) -> HtmxResult<Option<HxAttributes>> {
        let mut attrs = HxAttributes {
            verb: None,
            path: None,
            target: None,
            swap: SwapSpec::default(),
            trigger: vec![],
            include: None,
            confirm: None,
            indicator: None,
            headers: HashMap::new(),
            vars: HashMap::new(),
        };
        
        // Check for AJAX attributes
        attrs.verb = self.parse_verb(element)?;
        if let Some(verb) = attrs.verb {
            attrs.path = self.parse_path(element, verb)?;
        }
        
        // Parse other attributes
        attrs.target = self.parse_target(element)?;
        attrs.swap = self.parse_swap(element)?;
        attrs.trigger = self.parse_trigger(element)?;
        attrs.include = self.parse_include(element)?;
        attrs.confirm = self.get_attribute(element, "hx-confirm");
        attrs.indicator = self.get_attribute(element, "hx-indicator");
        
        // Return None if no AJAX attribute
        if attrs.verb.is_none() {
            return Ok(None);
        }
        
        Ok(Some(attrs))
    }
    
    fn parse_verb(&self, element: &Element) -> HtmxResult<Option<AjaxVerb>> {
        for verb in &[AjaxVerb::Get, AjaxVerb::Post, AjaxVerb::Put, 
                      AjaxVerb::Patch, AjaxVerb::Delete] {
            let attr_name = format!("hx-{}", format!("{:?}", verb).to_lowercase());
            if let Some(path) = self.get_attribute(element, &attr_name) {
                if !path.is_empty() {
                    return Ok(Some(*verb));
                }
            }
        }
        Ok(None)
    }
    
    fn parse_trigger(&self, element: &Element) -> HtmxResult<Vec<TriggerSpec>> {
        let trigger_str = self.get_attribute(element, "hx-trigger")
            .unwrap_or_else(|| "click".to_string());
        
        self.parse_trigger_spec(&trigger_str)
    }
    
    fn parse_trigger_spec(&self, spec: &str) -> HtmxResult<Vec<TriggerSpec>> {
        let mut triggers = Vec::new();
        
        for part in spec.split(',') {
            let part = part.trim();
            let tokens: Vec<&str> = part.split_whitespace().collect();
            
            if tokens.is_empty() {
                continue;
            }
            
            let mut trigger = TriggerSpec {
                event: TriggerType::DomEvent(tokens[0].to_string()),
                changed: false,
                once: false,
                consume: false,
                delay: None,
                throttle: None,
                from: None,
                target: None,
            };
            
            for token in &tokens[1..] {
                match *token {
                    "changed" => trigger.changed = true,
                    "once" => trigger.once = true,
                    "consume" => trigger.consume = true,
                    t if t.starts_with("delay:") => {
                        trigger.delay = Some(self.parse_time(t)?);
                    }
                    t if t.starts_with("throttle:") => {
                        trigger.throttle = Some(self.parse_time(t)?);
                    }
                    t if t.starts_with("from:") => {
                        trigger.from = Some(t[5..].to_string());
                    }
                    t if t.starts_with("target:") => {
                        trigger.target = Some(t[7..].to_string());
                    }
                    _ => {}
                }
            }
            
            triggers.push(trigger);
        }
        
        Ok(triggers)
    }
    
    fn parse_time(&self, spec: &str) -> HtmxResult<u32> {
        // Parse "500ms" or "1s" format
        let spec = spec.trim_start_matches("delay:").trim_start_matches("throttle:");
        
        if spec.ends_with("ms") {
            spec[..spec.len()-2].parse::<u32>()
                .map_err(|_| HtmxError::InvalidTrigger(format!("Invalid time: {}", spec)))
        } else if spec.ends_with('s') {
            let secs: f32 = spec[..spec.len()-1].parse()
                .map_err(|_| HtmxError::InvalidTrigger(format!("Invalid time: {}", spec)))?;
            Ok((secs * 1000.0) as u32)
        } else {
            Err(HtmxError::InvalidTrigger(format!("Invalid time format: {}", spec)))
        }
    }
}
```

## AJAX Engine in Rust

```rust
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::JsFuture;
use web_sys::{Request, RequestInit, RequestMode, Response, Headers};

/// AJAX request engine
pub struct AjaxEngine {
    event_bus: EventBus,
}

impl AjaxEngine {
    pub fn new(event_bus: EventBus) -> Self {
        Self { event_bus }
    }
    
    /// Issue AJAX request
    pub async fn request(
        &self,
        config: RequestConfig,
        element: &Element,
    ) -> HtmxResult<Response> {
        // Build fetch options
        let mut opts = RequestInit::new();
        opts.method(&config.verb.to_string());
        opts.mode(RequestMode::Cors);
        
        // Build URL with query params for GET
        let url = match config.verb {
            AjaxVerb::Get => {
                self.append_query_params(&config.path, &config.parameters)
            }
            _ => config.path.clone(),
        };
        
        // Set headers
        let headers = Headers::new()
            .map_err(|e| HtmxError::WasmError(format!("Failed to create headers: {:?}", e)))?;
        
        headers.append("HX-Request", "true")?;
        headers.append("HX-Trigger", &self.get_element_id(element))?;
        headers.append("HX-Target", &self.get_target_id(&config.target))?;
        
        // Set body for non-GET requests
        if config.verb != AjaxVerb::Get && !config.parameters.is_empty() {
            let body = self.encode_parameters(&config.parameters);
            opts.body(Some(&JsValue::from_str(&body)));
            headers.append("Content-Type", "application/x-www-form-urlencoded")?;
        }
        
        opts.headers(&headers);
        
        // Create and send request
        let request = Request::new_with_str_and_init(&url, &opts)
            .map_err(|e| HtmxError::WasmError(format!("Failed to create request: {:?}", e)))?;
        
        let window = web_sys::window()
            .ok_or_else(|| HtmxError::WasmError("No window object".to_string()))?;
        
        let response = window.fetch_with_request(&request);
        let response = JsFuture::from(response)
            .await
            .map_err(|e| HtmxError::NetworkError(format!("{:?}", e)))?;
        
        let response: Response = response.dyn_into()
            .map_err(|_| HtmxError::NetworkError("Invalid response type".to_string()))?;
        
        // Convert to our Response type
        self.convert_response(response).await
    }
    
    async fn convert_response(&self, response: Response) -> HtmxResult<Response> {
        let status = response.status();
        let status_text = response.status_text();
        
        // Get headers
        let headers_raw = response.headers()
            .raw()
            .map_err(|e| HtmxError::WasmError(format!("Failed to get headers: {:?}", e)))?;
        
        let headers = self.parse_headers(&headers_raw);
        
        // Get body text
        let body = response.text()
            .map_err(|e| HtmxError::NetworkError(format!("{:?}", e)))?;
        let body = JsFuture::from(body)
            .await
            .map_err(|e| HtmxError::NetworkError(format!("{:?}", e)))?;
        let body = body.as_string()
            .unwrap_or_default();
        
        Ok(Response {
            status,
            status_text,
            headers,
            body,
        })
    }
}
```

## DOM Swapping in Rust

```rust
use web_sys::{Element, Node, DocumentFragment, Range};

/// Execute DOM swap
pub struct DomSwapper {
    window: web_sys::Window,
    document: web_sys::Document,
}

impl DomSwapper {
    pub fn new() -> HtmxResult<Self> {
        let window = web_sys::window()
            .ok_or_else(|| HtmxError::WasmError("No window object".to_string()))?;
        let document = window.document()
            .ok_or_else(|| HtmxError::WasmError("No document object".to_string()))?;
        
        Ok(Self { window, document })
    }
    
    /// Execute swap operation
    pub fn swap(
        &self,
        target: &Element,
        content: &str,
        spec: &SwapSpec,
    ) -> HtmxResult<()> {
        // Parse HTML string into fragment
        let fragment = self.parse_html(content)?;
        
        // Execute swap based on style
        match spec.style {
            SwapStyle::InnerHtml => self.swap_inner_html(target, fragment)?,
            SwapStyle::OuterHtml => self.swap_outer_html(target, fragment)?,
            SwapStyle::BeforeBegin => self.swap_before_begin(target, fragment)?,
            SwapStyle::AfterBegin => self.swap_after_begin(target, fragment)?,
            SwapStyle::BeforeEnd => self.swap_before_end(target, fragment)?,
            SwapStyle::AfterEnd => self.swap_after_end(target, fragment)?,
            SwapStyle::Delete => self.swap_delete(target)?,
            SwapStyle::None => {}
        }
        
        Ok(())
    }
    
    fn parse_html(&self, html: &str) -> HtmxResult<DocumentFragment> {
        let range = Range::new(&self.document)
            .map_err(|e| HtmxError::WasmError(format!("Failed to create range: {:?}", e)))?;
        
        let fragment = range.create_contextual_fragment(html)
            .map_err(|e| HtmxError::WasmError(format!("Failed to create fragment: {:?}", e)))?;
        
        Ok(fragment)
    }
    
    fn swap_inner_html(&self, target: &Element, fragment: DocumentFragment) -> HtmxResult<()> {
        // Clear existing content
        target.set_inner_html("");
        
        // Append new content
        target.append_child(&fragment)
            .map_err(|e| HtmxError::WasmError(format!("Failed to append child: {:?}", e)))?;
        
        Ok(())
    }
    
    fn swap_outer_html(&self, target: &Element, fragment: DocumentFragment) -> HtmxResult<()> {
        let parent = target.parent_element()
            .ok_or_else(|| HtmxError::WasmError("No parent element".to_string()))?;
        
        let next_sibling = target.next_sibling();
        
        // Insert fragment before removing target
        if let Some(sibling) = &next_sibling {
            parent.insert_before(&fragment, Some(sibling))
                .map_err(|e| HtmxError::WasmError(format!("Failed to insert before: {:?}", e)))?;
        } else {
            parent.append_child(&fragment)
                .map_err(|e| HtmxError::WasmError(format!("Failed to append: {:?}", e)))?;
        }
        
        // Remove old target
        parent.remove_child(target)
            .map_err(|e| HtmxError::WasmError(format!("Failed to remove child: {:?}", e)))?;
        
        Ok(())
    }
    
    fn swap_before_begin(&self, target: &Element, fragment: DocumentFragment) -> HtmxResult<()> {
        let parent = target.parent_element()
            .ok_or_else(|| HtmxError::WasmError("No parent element".to_string()))?;
        
        parent.insert_before(&fragment, Some(target))
            .map_err(|e| HtmxError::WasmError(format!("Failed to insert before: {:?}", e)))?;
        
        Ok(())
    }
    
    fn swap_after_begin(&self, target: &Element, fragment: DocumentFragment) -> HtmxResult<()> {
        let first_child = target.first_child();
        
        if let Some(first) = &first_child {
            target.insert_before(&fragment, Some(first))
                .map_err(|e| HtmxError::WasmError(format!("Failed to insert before: {:?}", e)))?;
        } else {
            target.append_child(&fragment)
                .map_err(|e| HtmxError::WasmError(format!("Failed to append: {:?}", e)))?;
        }
        
        Ok(())
    }
    
    fn swap_before_end(&self, target: &Element, fragment: DocumentFragment) -> HtmxResult<()> {
        target.append_child(&fragment)
            .map_err(|e| HtmxError::WasmError(format!("Failed to append: {:?}", e)))?;
        
        Ok(())
    }
    
    fn swap_after_end(&self, target: &Element, fragment: DocumentFragment) -> HtmxResult<()> {
        let parent = target.parent_element()
            .ok_or_else(|| HtmxError::WasmError("No parent element".to_string()))?;
        
        let next_sibling = target.next_sibling();
        
        if let Some(sibling) = &next_sibling {
            parent.insert_before(&fragment, Some(sibling))
                .map_err(|e| HtmxError::WasmError(format!("Failed to insert before: {:?}", e)))?;
        } else {
            parent.append_child(&fragment)
                .map_err(|e| HasmError::WasmError(format!("Failed to append: {:?}", e)))?;
        }
        
        Ok(())
    }
    
    fn swap_delete(&self, target: &Element) -> HtmxResult<()> {
        let parent = target.parent_element()
            .ok_or_else(|| HtmxError::WasmError("No parent element".to_string()))?;
        
        parent.remove_child(target)
            .map_err(|e| HtmxError::WasmError(format!("Failed to remove child: {:?}", e)))?;
        
        Ok(())
    }
}
```

## Event System

```rust
use wasm_bindgen::closure::Closure;
use wasm_bindgen::JsCast;
use web_sys::{Event, EventTarget, CustomEvent, CustomEventInit};

/// Event bus for HTMX events
pub struct EventBus {
    target: EventTarget,
}

impl EventBus {
    pub fn new() -> HtmxResult<Self> {
        let window = web_sys::window()
            .ok_or_else(|| HtmxError::WasmError("No window object".to_string()))?;
        
        Ok(Self { 
            target: window.dyn_into::<EventTarget>()
                .map_err(|_| HtmxError::WasmError("Failed to get EventTarget".to_string()))?
        })
    }
    
    /// Trigger custom event
    pub fn trigger(&self, name: &str, detail: &JsValue) -> HtmxResult<()> {
        let mut opts = CustomEventInit::new();
        opts.detail(detail);
        
        let event = CustomEvent::new_with_event_init_dict(name, &opts)
            .map_err(|e| HtmxError::WasmError(format!("Failed to create event: {:?}", e)))?;
        
        self.target.dispatch_event(&event)
            .map_err(|e| HtmxError::WasmError(format!("Failed to dispatch event: {:?}", e)))?;
        
        Ok(())
    }
    
    /// Add event listener
    pub fn on<F>(&self, name: &str, handler: F) -> HtmxResult<Closure<dyn FnMut(Event)>>
    where
        F: FnMut(Event) + 'static,
    {
        let closure = Closure::wrap(Box::new(handler) as Box<dyn FnMut(Event)>);
        
        self.target.add_event_listener_with_callback(
            name, 
            closure.as_ref().unchecked_ref()
        )
        .map_err(|e| HtmxError::WasmError(format!("Failed to add listener: {:?}", e)))?;
        
        Ok(closure)
    }
}

/// HTMX event types
#[derive(Debug, Clone)]
pub enum HtmxEvent {
    Confirm { question: String },
    BeforeRequest { path: String, verb: String },
    AfterRequest { status: u16 },
    BeforeSwap { content: String },
    AfterSwap { target: String },
    AfterSettle,
    SendError { message: String },
    SendAbort,
    Timeout,
    ResponseError { status: u16, message: String },
}
```

## Key Rust-Specific Changes

### 1. Owned vs Borrowed Strings

```rust
// JavaScript version
function parseAttribute(elt, attrName) {
    return elt.getAttribute(attrName); // Returns string or null
}

// Rust version - explicit ownership
fn parse_attribute(element: &Element, attr_name: &str) -> Option<String> {
    element.get_attribute(attr_name).ok().flatten()
}
```

### 2. Result-based Error Handling

```rust
// JavaScript version
function swap(target, content, spec) {
    if (!target) {
        console.error('Target not found');
        return;
    }
    // ... swap logic
}

// Rust version - explicit error handling
fn swap(target: &Element, content: &str, spec: &SwapSpec) -> HtmxResult<()> {
    // Error propagated to caller
    // ... swap logic
}
```

### 3. Async/Await with Futures

```rust
// JavaScript version
async function issueAjaxRequest(elt, event) {
    const response = await fetch(url, options);
    const html = await response.text();
    swap(target, html, spec);
}

// Rust version - same async pattern
async fn issue_ajax_request(
    &self, 
    config: RequestConfig, 
    element: &Element
) -> HtmxResult<()> {
    let response = self.request(config, element).await?;
    self.swapper.swap(&target, &response.body, &config.swap)?;
    Ok(())
}
```

## Recommended Dependencies

| Purpose | Crate | Version |
|---------|-------|---------|
| WASM bindings | wasm-bindgen | 0.2 |
| Web APIs | web-sys | 0.3 |
| JS types | js-sys | 0.3 |
| Async futures | wasm-bindgen-futures | 0.4 |
| Serialization | serde + serde_json | 1.0 |
| Error handling | thiserror | 1.0 |
| Logging | log + console_log | 0.4 |

## Migration Path

1. **Phase 1: Core Library**
   - Implement attribute parser
   - Implement types
   - Write unit tests

2. **Phase 2: WASM Bindings**
   - Implement DOM manipulation
   - Implement AJAX engine
   - Test in browser

3. **Phase 3: Extensions**
   - WebSocket extension
   - SSE extension
   - Other official extensions

4. **Phase 4: Compatibility Layer**
   - JavaScript shim for gradual migration
   - Feature parity testing

## Code Examples

### Complete Attribute Parser

```rust
use wasm_bindgen::prelude::*;
use web_sys::Element;

#[wasm_bindgen]
pub struct HtmxEngine {
    parser: AttributeParser,
    engine: AjaxEngine,
    swapper: DomSwapper,
}

#[wasm_bindgen]
impl HtmxEngine {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Result<HtmxEngine, JsValue> {
        console_log::init_with_level(log::Level::Debug)
            .map_err(|e| JsValue::from_str(&format!("Init failed: {:?}", e)))?;
        
        Ok(Self {
            parser: AttributeParser::new(),
            engine: AjaxEngine::new(EventBus::new()?),
            swapper: DomSwapper::new()?,
        })
    }
    
    #[wasm_bindgen]
    pub fn process(&self, element: &Element) -> Result<(), JsValue> {
        if let Some(attrs) = self.parser.parse(element)? {
            log::info!("Processing element with attrs: {:?}", attrs);
            // Bind event listeners
        }
        Ok(())
    }
    
    #[wasm_bindgen]
    pub async fn ajax(
        &self,
        verb: &str,
        path: &str,
        target: &Element,
    ) -> Result<(), JsValue> {
        let config = RequestConfig {
            verb: verb.parse()?,
            path: path.to_string(),
            // ... other fields
        };
        
        let response = self.engine.request(config, target).await?;
        self.swapper.swap(target, &response.body, &SwapSpec::default())?;
        Ok(())
    }
}
```

## Conclusion

This Rust revision provides a type-safe, performant implementation of HTMX using WASM. Key benefits include:

1. **Compile-time safety**: Attribute parsing errors caught early
2. **Better performance**: WASM execution speed
3. **Smaller bundles**: Tree-shaking and optimization
4. **Modern Rust patterns**: Async/await, Result-based errors
5. **Full compatibility**: Drop-in replacement for JS version
