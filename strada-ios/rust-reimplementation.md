# Strada iOS - Rust Reimplementation Guide

## Overview

This document provides a comprehensive guide for reimplementing Strada iOS in Rust, targeting iOS platform integration with WebKit.

## Architecture Mapping

### Swift to Rust Component Mapping

| Swift Component | Rust Equivalent | Notes |
|-----------------|-----------------|-------|
| `Bridge` | `Bridge` struct | Core bridge management |
| `BridgeComponent` | `BridgeComponent` trait | Component interface |
| `BridgeDelegate` | `BridgeDelegate` struct | Message routing |
| `Message` | `Message` struct | Message data |
| `WKWebView` | `wkWebView::WKWebView` | Via webkit2gtk or objc2 |
| `WKScriptMessageHandler` | Custom handler | Message reception |
| `WKUserScript` | User script injection | JavaScript injection |

## Crate Structure

```
strada-rs/
├── Cargo.toml
├── src/
│   ├── lib.rs              # Library root
│   ├── bridge.rs           # Core bridge (Bridge.swift equivalent)
│   ├── component.rs        # Component trait (BridgeComponent.swift)
│   ├── delegate.rs         # Message routing (BridgeDelegate.swift)
│   ├── message.rs          # Message structure (Message.swift)
│   ├── javascript.rs       # JS evaluation helpers (JavaScript.swift)
│   ├── config.rs           # Configuration (StradaConfig.swift)
│   ├── logging.rs          # Logging setup (Logging.swift)
│   └── webview/
│       ├── mod.rs          # WebView abstraction
│       ├── ios.rs          # iOS-specific implementation
│       └── script_handler.rs # Message handler
├── examples/
│   └── basic_bridge/       # Basic usage example
└── tests/
    ├── bridge_tests.rs
    ├── component_tests.rs
    └── message_tests.rs
```

## Core Implementation

### Cargo.toml Dependencies

```toml
[package]
name = "strada-rs"
version = "0.1.0"
edition = "2021"
license = "MIT"
description = "Rust implementation of Strada iOS - WebView bridge for native components"

[dependencies]
# Async runtime
tokio = { version = "1.35", features = ["full"] }
async-trait = "0.1"

# JSON serialization
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"

# Logging
tracing = "0.1"
tracing-subscriber = "0.3"

# iOS/WebKit bindings (choose one approach)
# Option 1: Use webkit2gtk (Linux-first, may work on iOS with complications)
# webkit2gtk = "0.18"

# Option 2: Use objc2 for direct Objective-C/Swift interop
objc2 = "0.5"
objc2-foundation = "0.2"
objc2-web-kit = "0.2"  # If available, or use bindings

# Option 3: Generate bindings with bindgen
# (Create custom bindings for WKWebView)

# Error handling
thiserror = "1.0"
anyhow = "1.0"

# Type erasure for components
boxcar = "0.2"  # Or use Arc<dyn Trait>

[build-dependencies]
bindgen = "0.69"  # If generating custom bindings

[lib]
crate-type = ["staticlib", "rlib"]
```

### Message Structure (message.rs)

```rust
use serde::{Deserialize, Serialize};

/// A message passed between native and web components
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Message {
    /// Unique identifier for correlation
    pub id: String,

    /// Component name (e.g., "form", "page")
    pub component: String,

    /// Event type (e.g., "connect", "submit")
    pub event: String,

    /// Optional metadata (URL, etc.)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<MessageMetadata>,

    /// JSON data payload
    #[serde(default = "default_object")]
    pub data: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MessageMetadata {
    pub url: String,
}

fn default_object() -> serde_json::Value {
    serde_json::Value::Object(serde_json::Map::new())
}

impl Message {
    pub fn new(
        id: impl Into<String>,
        component: impl Into<String>,
        event: impl Into<String>,
        metadata: Option<MessageMetadata>,
        data: serde_json::Value,
    ) -> Self {
        Self {
            id: id.into(),
            component: component.into(),
            event: event.into(),
            metadata,
            data,
        }
    }

    /// Create a reply message with new data
    pub fn with_data<T: Serialize>(&self, data: &T) -> Result<Self, serde_json::Error> {
        Ok(Message {
            id: self.id.clone(),
            component: self.component.clone(),
            event: self.event.clone(),
            metadata: self.metadata.clone(),
            data: serde_json::to_value(data)?,
        })
    }

    /// Create a reply with a different event
    pub fn with_event(&self, event: impl Into<String>) -> Self {
        Message {
            id: self.id.clone(),
            component: self.component.clone(),
            event: event.into(),
            metadata: self.metadata.clone(),
            data: self.data.clone(),
        }
    }

    /// Extract typed data from message
    pub fn data_as<T: for<'de> Deserialize<'de>>(&self) -> Result<T, serde_json::Error> {
        serde_json::from_value(self.data.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_message_serialization() {
        let msg = Message::new(
            "msg-123",
            "form",
            "connect",
            Some(MessageMetadata { url: "https://example.com".to_string() }),
            serde_json::json!({"submitTitle": "Submit"}),
        );

        let json = serde_json::to_string(&msg).unwrap();
        let parsed: Message = serde_json::from_str(&json).unwrap();
        assert_eq!(msg, parsed);
    }
}
```

### Component Trait (component.rs)

```rust
use crate::message::Message;
use crate::delegate::BridgeDelegate;
use async_trait::async_trait;
use std::sync::Arc;

/// Result type for component operations
pub type ComponentResult<T> = Result<T, ComponentError>;

#[derive(Debug, thiserror::Error)]
pub enum ComponentError {
    #[error("Bridge not available")]
    BridgeUnavailable,

    #[error("Message not found for event: {0}")]
    MessageNotFound(String),

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("JavaScript evaluation failed: {0}")]
    JavaScriptError(String),
}

/// Trait for bridge components (equivalent to BridgeComponent subclass)
#[async_trait]
pub trait BridgeComponent: Send + Sync {
    /// Unique component name (must match web component name)
    fn name(&self) -> &'static str;

    /// Called when a message is received from the web
    async fn on_receive(&mut self, message: Message) -> ComponentResult<()>;

    /// Lifecycle: View loaded
    async fn on_view_did_load(&mut self) {}

    /// Lifecycle: View will appear
    async fn on_view_will_appear(&mut self) {}

    /// Lifecycle: View did appear
    async fn on_view_did_appear(&mut self) {}

    /// Lifecycle: View will disappear
    async fn on_view_will_disappear(&mut self) {}

    /// Lifecycle: View did disappear
    async fn on_view_did_disappear(&mut self) {}

    /// Set the delegate (called by BridgeDelegate)
    fn set_delegate(&mut self, delegate: Arc<dyn BridgeDelegateTrait>);
}

/// Trait for bridge delegate (for components to reply)
#[async_trait]
pub trait BridgeDelegateTrait: Send + Sync {
    async fn reply_with(&self, message: Message) -> ComponentResult<bool>;
}

/// Type-erased component container
pub struct BoxedComponent {
    inner: Box<dyn BridgeComponent>,
    received_messages: std::collections::HashMap<String, Message>,
}

impl BoxedComponent {
    pub fn new(component: impl BridgeComponent + 'static) -> Self {
        Self {
            inner: Box::new(component),
            received_messages: std::collections::HashMap::new(),
        }
    }

    pub fn name(&self) -> &'static str {
        self.inner.name()
    }

    pub async fn handle_message(&mut self, message: Message) -> ComponentResult<()> {
        // Cache message by event
        self.received_messages.insert(message.event.clone(), message.clone());
        // Forward to component
        self.inner.on_receive(message).await
    }

    pub fn get_message(&self, event: &str) -> Option<&Message> {
        self.received_messages.get(event)
    }

    // Delegate lifecycle methods...
}
```

### Bridge Structure (bridge.rs)

```rust
use crate::message::Message;
use crate::component::{ComponentResult, BridgeDelegateTrait};
use crate::javascript::JavaScript;
use std::sync::{Arc, Weak};
use tokio::sync::RwLock;
use tracing::{debug, error, warn};

/// Bridge error types
#[derive(Debug, thiserror::Error)]
pub enum BridgeError {
    #[error("WebView not available")]
    WebViewMissing,

    #[error("JavaScript evaluation failed: {0}")]
    JavaScriptError(String),

    #[error("Component not registered: {0}")]
    ComponentNotRegistered(String),
}

/// Main bridge struct (equivalent to Bridge.swift)
pub struct Bridge {
    webview: Arc<dyn WebViewTrait>,
    delegate: RwLock<Option<Arc<dyn BridgeDelegateTrait>>>,
}

impl Bridge {
    /// Initialize bridge with a WebView
    pub fn new(webview: Arc<dyn WebViewTrait>) -> Self {
        let bridge = Self {
            webview,
            delegate: RwLock::new(None),
        };
        bridge.load_into_webview();
        bridge
    }

    /// Get or create bridge for webview (singleton pattern)
    pub fn initialize(webview: Arc<dyn WebViewTrait>) -> Arc<Self> {
        // In production, use a ConcurrentHashMap or similar
        static BRIDGES: once_cell::sync::Lazy<
            RwLock<std::collections::HashMap<usize, Arc<Bridge>>>
        > = once_cell::sync::Lazy::new(|| RwLock::new(std::collections::HashMap::new()));

        // Check existing, create if needed
        // Implementation depends on your WebView identity mechanism
        todo!("Implement singleton pattern")
    }

    fn load_into_webview(&self) {
        // Inject strada.js equivalent
        let user_script = self.make_user_script();
        self.webview.add_user_script(user_script);

        // Register message handler
        let handler = Arc::new(ScriptMessageHandler::new(self.clone()));
        self.webview.add_script_message_handler(handler, "strada");
    }

    fn make_user_script(&self) -> UserScript {
        let source = include_str!("../js/strada.js"); // Embedded JS
        UserScript {
            source: source.to_string(),
            injection_time: InjectionTime::AtDocumentStart,
            for_main_frame_only: true,
        }
    }

    /// Register a single component
    pub async fn register_component(&self, component: &str) -> ComponentResult<()> {
        self.call_bridge_function("register", &[serde_json::json!([component])])
            .await?;
        Ok(())
    }

    /// Register multiple components
    pub async fn register_components(&self, components: &[&str]) -> ComponentResult<()> {
        self.call_bridge_function("register", &[serde_json::json!(components)])
            .await?;
        Ok(())
    }

    /// Reply to web with message
    pub async fn reply_with(&self, message: Message) -> ComponentResult<bool> {
        debug!("bridgeWillReplyWithMessage: {:?}", message);

        let internal_msg = InternalMessage::from(&message);
        let json_value = internal_msg.to_json();

        self.call_bridge_function("replyWith", &[json_value]).await?;
        Ok(true)
    }

    async fn call_bridge_function(
        &self,
        function: &str,
        arguments: &[serde_json::Value],
    ) -> ComponentResult<serde_json::Value> {
        let js = JavaScript::call("window.nativeBridge", function, arguments);

        match self.webview.evaluate_javascript(&js).await {
            Ok(result) => Ok(result),
            Err(e) => {
                error!("Error evaluating JavaScript: {}", e);
                Err(ComponentError::JavaScriptError(e.to_string()))
            }
        }
    }

    pub fn set_delegate(&self, delegate: Arc<dyn BridgeDelegateTrait>) {
        // Implementation
    }
}

/// Internal message format for JSON serialization
#[derive(Debug, Serialize, Deserialize)]
struct InternalMessage {
    id: String,
    component: String,
    event: String,
    data: serde_json::Value,
}

impl From<&Message> for InternalMessage {
    fn from(msg: &Message) -> Self {
        Self {
            id: msg.id.clone(),
            component: msg.component.clone(),
            event: msg.event.clone(),
            data: msg.data.clone(),
        }
    }
}

impl InternalMessage {
    fn to_json(&self) -> serde_json::Value {
        serde_json::to_value(self).unwrap_or(serde_json::json!({}))
    }

    fn to_message(&self) -> Message {
        Message::new(
            self.id.clone(),
            self.component.clone(),
            self.event.clone(),
            None, // Extract from data if present
            self.data.clone(),
        )
    }
}
```

### JavaScript Helper (javascript.rs)

```rust
use serde_json::Value;

#[derive(Debug, thiserror::Error)]
pub enum JavaScriptError {
    #[error("Invalid argument type")]
    InvalidArgument,
}

/// Builds JavaScript function calls
pub struct JavaScript {
    object: Option<String>,
    function: String,
    arguments: Vec<Value>,
}

impl JavaScript {
    pub fn call(object: &str, function: &str, arguments: &[Value]) -> String {
        let js = Self {
            object: Some(object.to_string()),
            function: function.to_string(),
            arguments: arguments.to_vec(),
        };
        js.to_string()
    }

    pub fn global(function: &str, arguments: &[Value]) -> String {
        let js = Self {
            object: None,
            function: function.to_string(),
            arguments: arguments.to_vec(),
        };
        js.to_string()
    }

    fn to_string(&self) -> String {
        let args_str = self.encode_arguments();
        let func_name = self.sanitize_function();

        if let Some(obj) = &self.object {
            format!("{}.{}({})", obj, func_name, args_str)
        } else {
            format!("{}({})", func_name, args_str)
        }
    }

    fn encode_arguments(&self) -> String {
        // Convert args to JSON array string, strip outer brackets
        let json = serde_json::to_string(&self.arguments)
            .unwrap_or_else(|_| "[]".to_string());

        // Remove outer [ and ]
        if json.len() > 2 {
            json[1..json.len()-1].to_string()
        } else {
            String::new()
        }
    }

    fn sanitize_function(&self) -> &str {
        // Strip trailing () if present
        self.function.strip_suffix("()").unwrap_or(&self.function)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_javascript_call() {
        let js = JavaScript::call(
            "window.nativeBridge",
            "register",
            &[serde_json::json!(["form", "page"])],
        );
        assert_eq!(js, r#"window.nativeBridge.register(["form","page"])"#);
    }

    #[test]
    fn test_global_function() {
        let js = JavaScript::global("console.log", &[serde_json::json!("hello")]);
        assert_eq!(js, r#"console.log("hello")"#);
    }
}
```

### WebView Trait (webview/mod.rs)

```rust
use async_trait::async_trait;
use serde_json::Value;

/// Injection time for user scripts
#[derive(Debug, Clone, Copy)]
pub enum InjectionTime {
    AtDocumentStart,
    AtDocumentEnd,
}

/// User script configuration
#[derive(Debug, Clone)]
pub struct UserScript {
    pub source: String,
    pub injection_time: InjectionTime,
    pub for_main_frame_only: bool,
}

/// Trait abstracting WKWebView
#[async_trait]
pub trait WebViewTrait: Send + Sync {
    /// Evaluate JavaScript and return result
    async fn evaluate_javascript(&self, js: &str) -> Result<Value, WebViewError>;

    /// Add a user script
    fn add_user_script(&self, script: UserScript);

    /// Add a script message handler
    fn add_script_message_handler(
        &self,
        handler: Arc<dyn ScriptMessageHandlerTrait>,
        name: &str,
    );
}

#[derive(Debug, thiserror::Error)]
pub enum WebViewError {
    #[error("WebView not initialized")]
    NotInitialized,

    #[error("JavaScript error: {0}")]
    JavaScript(String),

    #[error("Platform error: {0}")]
    Platform(String),
}

/// Script message handler trait
#[async_trait]
pub trait ScriptMessageHandlerTrait: Send + Sync {
    async fn handle_message(&self, body: serde_json::Value);
}
```

### iOS-Specific Implementation (webview/ios.rs)

```rust
//! iOS WKWebView implementation using objc2
//!
//! This requires proper Objective-C bindings for WKWebView

use super::{WebViewTrait, WebViewError, UserScript, InjectionTime, ScriptMessageHandlerTrait};
use objc2::rc::Retained;
use objc2_web_kit::{WKWebView, WKUserScript, WKUserContentController};
use async_trait::async_trait;
use serde_json::Value;
use std::sync::Arc;

pub struct IosWebView {
    webview: Retained<WKWebView>,
    content_controller: Retained<WKUserContentController>,
}

impl IosWebView {
    pub fn new(webview: Retained<WKWebView>) -> Self {
        let content_controller = webview.configuration().userContentController();
        Self {
            webview,
            content_controller,
        }
    }
}

#[async_trait]
impl WebViewTrait for IosWebView {
    async fn evaluate_javascript(&self, js: &str) -> Result<Value, WebViewError> {
        // Use objc2 to call evaluateJavaScript:completionHandler:
        // This is a simplified example - actual implementation needs GCD handling

        let js_string = objc2::rc::NSString::from_str(js);

        // Note: Actual async implementation needs to bridge
        // Grand Central Dispatch completion handlers to Rust futures
        todo!("Implement async JavaScript evaluation with objc2")
    }

    fn add_user_script(&self, script: UserScript) {
        let injection_time = match script.injection_time {
            InjectionTime::AtDocumentStart => WKUserScriptInjectionTime::AtDocumentStart,
            InjectionTime::AtDocumentEnd => WKUserScriptInjectionTime::AtDocumentEnd,
        };

        let user_script = unsafe {
            WKUserScript::initWithSource_injectionTime_forMainFrameOnly_(
                WKUserScript::alloc(),
                &NSString::from_str(&script.source),
                injection_time,
                script.for_main_frame_only,
            )
        };

        self.content_controller.addUserScript(&user_script);
    }

    fn add_script_message_handler(
        &self,
        handler: Arc<dyn ScriptMessageHandlerTrait>,
        name: &str,
    ) {
        // Create a wrapper that implements WKScriptMessageHandler
        // This requires creating an Objective-C class in Rust
        todo!("Implement WKScriptMessageHandler wrapper")
    }
}
```

### Delegate Implementation (delegate.rs)

```rust
use crate::message::Message;
use crate::component::{BoxedComponent, ComponentResult, BridgeDelegateTrait};
use crate::bridge::Bridge;
use std::sync::{Arc, RwLock};
use std::collections::HashMap;
use tracing::{debug, warn};

pub struct BridgeDelegate {
    location: String,
    components: RwLock<HashMap<&'static str, BoxedComponent>>,
    component_types: Vec<ComponentFactory>,
    bridge: RwLock<Option<Arc<Bridge>>>,
    destination_is_active: RwLock<bool>,
}

/// Factory for creating components
type ComponentFactory = fn() -> BoxedComponent;

impl BridgeDelegate {
    pub fn new(
        location: String,
        component_factories: Vec<ComponentFactory>,
    ) -> Self {
        Self {
            location,
            components: RwLock::new(HashMap::new()),
            component_types: component_factories,
            bridge: RwLock::new(None),
            destination_is_active: RwLock::new(true),
        }
    }

    pub fn set_bridge(&self, bridge: Arc<Bridge>) {
        *self.bridge.write().unwrap() = Some(bridge);
    }

    /// Handle message from bridge
    pub async fn handle_message(&self, message: Message) -> ComponentResult<bool> {
        let is_active = *self.destination_is_active.read().unwrap();

        if !is_active {
            warn!("Message received but destination is inactive");
            return Ok(false);
        }

        // Get or create component
        let mut components = self.components.write().unwrap();
        let component = components.entry(message.component.as_str())
            .or_insert_with(|| {
                // Find factory for this component type
                for factory in &self.component_types {
                    let component = factory();
                    if component.name() == message.component {
                        return component;
                    }
                }
                panic!("No factory registered for component: {}", message.component);
            });

        component.handle_message(message).await?;
        Ok(true)
    }

    /// Lifecycle: View did load
    pub async fn on_view_did_load(&self) {
        *self.destination_is_active.write().unwrap() = true;
        let mut components = self.components.write().unwrap();
        for component in components.values_mut() {
            component.on_view_did_load().await;
        }
    }

    /// Register all components on init
    pub async fn on_bridge_did_initialize(&self) -> ComponentResult<()> {
        let bridge = self.bridge.read().unwrap();
        if let Some(bridge) = bridge.as_ref() {
            let component_names: Vec<&str> = self.component_types
                .iter()
                .map(|_| todo!("Get component name"))
                .collect();

            bridge.register_components(&component_names).await?;
        }
        Ok(())
    }
}

#[async_trait]
impl BridgeDelegateTrait for BridgeDelegate {
    async fn reply_with(&self, message: Message) -> ComponentResult<bool> {
        let bridge = self.bridge.read().unwrap();
        match bridge.as_ref() {
            Some(bridge) => bridge.reply_with(message).await,
            None => Err(ComponentError::BridgeUnavailable),
        }
    }
}
```

## Example Component Implementation

```rust
use strada_rs::component::{BridgeComponent, BridgeDelegateTrait, ComponentResult};
use strada_rs::message::Message;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

pub struct FormComponent {
    delegate: RwLock<Option<Arc<dyn BridgeDelegateTrait>>>,
}

#[derive(Debug, Deserialize)]
struct FormData {
    submit_title: String,
}

#[derive(Debug, Serialize)]
struct FormReply {
    submitted: bool,
}

#[async_trait]
impl BridgeComponent for FormComponent {
    fn name(&self) -> &'static str {
        "form"
    }

    async fn on_receive(&mut self, message: Message) -> ComponentResult<()> {
        match message.event.as_str() {
            "connect" => {
                let data: FormData = message.data_as()?;
                debug!("Form connect: {}", data.submit_title);
                // Show native submit button
                Ok(())
            }
            "submit-enabled" => {
                // Enable submit button
                Ok(())
            }
            "submit-disabled" => {
                // Disable submit button
                Ok(())
            }
            _ => Ok(()),
        }
    }

    fn set_delegate(&mut self, delegate: Arc<dyn BridgeDelegateTrait>) {
        *self.delegate.write().unwrap() = Some(delegate);
    }
}
```

## JavaScript Bridge (strada.js)

The JavaScript bridge remains the same as the Swift version since it's injected into the WebView:

```javascript
// Embedded as include_str!("../js/strada.js")
(() => {
  class NativeBridge {
    constructor() {
      this.supportedComponents = []
      this.registerCalled = new Promise(resolve => this.registerResolver = resolve)
      document.addEventListener("web-bridge:ready", async () => {
        await this.setAdapter()
      })
    }

    async setAdapter() {
      await this.registerCalled
      this.webBridge.setAdapter(this)
    }

    register(component) {
      if (Array.isArray(component)) {
        this.supportedComponents = this.supportedComponents.concat(component)
      } else {
        this.supportedComponents.push(component)
      }
      this.registerResolver()
      this.notifyBridgeOfSupportedComponentsUpdate()
    }

    unregister(component) {
      const index = this.supportedComponents.indexOf(component)
      if (index != -1) {
        this.supportedComponents.splice(index, 1)
        this.notifyBridgeOfSupportedComponentsUpdate()
      }
    }

    notifyBridgeOfSupportedComponentsUpdate() {
      if (this.isStradaAvailable) {
        this.webBridge.adapterDidUpdateSupportedComponents()
      }
    }

    supportsComponent(component) {
      return this.supportedComponents.includes(component)
    }

    replyWith(message) {
      if (this.isStradaAvailable) {
        this.webBridge.receive(message)
      }
    }

    receive(message) {
      this.postMessage(message)
    }

    get platform() {
      return "ios"
    }

    postMessage(message) {
      webkit.messageHandlers.strada.postMessage(message)
    }

    get isStradaAvailable() {
      return window.Strada
    }

    get webBridge() {
      return window.Strada.web
    }
  }

  window.nativeBridge = new NativeBridge()
  window.nativeBridge.postMessage("ready")
})()
```

## Build Configuration for iOS

### Creating a Static Library

```toml
# Cargo.toml
[lib]
crate-type = ["staticlib", "rlib"]
name = "strada_rs"
```

### Build Script (build.rs)

```rust
use std::env;

fn main() {
    let target = env::var("TARGET").unwrap();

    if target.contains("ios") {
        // iOS-specific build configuration
        println!("cargo:rustc-link-lib=framework=WebKit");
        println!("cargo:rustc-link-lib=framework=Foundation");
    }
}
```

### Xcode Integration

1. Build the Rust library:
```bash
cargo build --release --target aarch64-apple-ios
```

2. Add the `.a` file to Xcode project
3. Create bridging headers for Swift interoperability
4. Use C FFI or create Swift wrappers

## Testing Strategy

### Unit Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_message_serialization() {
        let msg = Message::new("1", "form", "connect", None, json!({}));
        let serialized = serde_json::to_string(&msg).unwrap();
        assert!(serialized.contains("form"));
    }

    #[tokio::test]
    async fn test_component_message_handling() {
        let mut component = FormComponent::new();
        let message = Message::new("1", "form", "connect", None, json!({}));
        let result = component.handle_message(message).await;
        assert!(result.is_ok());
    }
}
```

### Integration Tests

```rust
#[cfg(test)]
mod integration {
    use super::*;

    #[tokio::test]
    async fn test_bridge_initialization() {
        let webview = MockWebView::new();
        let bridge = Bridge::new(Arc::new(webview));
        // Verify strada.js was injected
    }
}
```

## Challenges and Considerations

### 1. Objective-C Interop

Rust doesn't have seamless Objective-C interop like Swift. Options:

- **objc2 crate**: Direct Objective-C runtime access
- **bindgen**: Generate Rust bindings from headers
- **C FFI bridge**: Write C wrapper, call from Rust

### 2. Async/Await Bridging

Grand Central Dispatch (GCD) completion handlers must be bridged to Rust futures:

```rust
// Requires custom GCD → Tokio bridge
fn evaluate_javascript_async(js: &str) -> impl Future<Output = Result<Value, Error>> {
    // Convert completion handler to Rust future
}
```

### 3. Memory Management

Swift uses ARC, Rust uses ownership. Care needed with:
- Reference counting across FFI boundary
- Retain cycles between Swift objects and Rust Arc
- Weak references (no direct Rust equivalent)

### 4. Threading

WKWebView requires main thread execution:
```rust
// May need to spawn on main thread
tokio::task::spawn_blocking(|| {
    // WKWebView calls here
});
```

## Recommended Approach

For a production implementation:

1. **Start with macOS**: Test on macOS WebKit before iOS
2. **Use objc2**: Most idiomatic Rust-Objective-C bridge
3. **Create Swift wrappers**: Swift manages WKWebView, Rust handles logic
4. **Hybrid approach**: Keep some Swift for UIKit integration

```
┌─────────────────────────────────────────────────────────┐
│                    Swift Layer                          │
│  - WKWebView lifecycle                                  │
│  - UIKit integration                                    │
│  - View controller management                           │
└─────────────────────────────────────────────────────────┘
                          │
                          │ C FFI / objc2
                          ▼
┌─────────────────────────────────────────────────────────┐
│                    Rust Layer                           │
│  - Message serialization                                │
│  - Component logic                                      │
│  - Business logic                                       │
└─────────────────────────────────────────────────────────┘
```

---

*This guide provides the architecture for reimplementing Strada iOS in Rust. The key challenge is the iOS WebKit integration, which requires careful handling of Objective-C interop.*
