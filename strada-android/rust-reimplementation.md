# Strada Android - Rust Reimplementation Guide

## Overview

This document provides a comprehensive guide for reimplementing Strada Android in Rust, targeting Android platform integration with WebView.

## Architecture Mapping

### Kotlin to Rust Component Mapping

| Kotlin Component | Rust Equivalent | Notes |
|-----------------|-----------------|-------|
| `Bridge` | `Bridge` struct | Core bridge management |
| `BridgeComponent` | `BridgeComponent` trait | Component interface |
| `BridgeDelegate` | `BridgeDelegate` struct | Message routing |
| `Message` | `Message` struct | Message data |
| `WebView` | `jni::JavaObject` | Via JNI bindings |
| `@JavascriptInterface` | `#[jni_method]` | Via jni-rs |
| `DefaultLifecycleObserver` | Lifecycle callbacks | Manual implementation |

## Crate Structure

```
strada-android-rs/
├── Cargo.toml
├── src/
│   ├── lib.rs              # Library root + JNI entry points
│   ├── bridge.rs           # Core bridge (Bridge.kt equivalent)
│   ├── component.rs        # Component trait (BridgeComponent.kt)
│   ├── delegate.rs         # Message routing (BridgeDelegate.kt)
│   ├── message.rs          # Message structure (Message.kt)
│   ├── config.rs           # Configuration (StradaConfig.kt)
│   ├── logging.rs          # Logging setup (StradaLog.kt)
│   ├── json.rs             # JSON serialization (serde_json)
│   └── lifecycle.rs        # Lifecycle handling
├── jni/
│   └── bindings.rs         # JNI interface definitions
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
name = "strada-android-rs"
version = "0.1.0"
edition = "2021"
license = "MIT"
description = "Rust implementation of Strada Android - WebView bridge for native components"

[lib]
crate-type = ["cdylib", "rlib"]
name = "strada_android"

[dependencies]
# JNI bindings
jni = "0.21"
jni-sys = "0.3"

# Async runtime
tokio = { version = "1.35", features = ["full"] }
async-trait = "0.1"

# JSON serialization
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"

# Logging
tracing = "0.1"
tracing-android = "0.2"
tracing-subscriber = "0.3"

# Error handling
thiserror = "1.0"
anyhow = "1.0"

# Android logging
android_logger = "0.13"

# Lazy initialization
once_cell = "1.19"

[build-dependencies]
cc = "1.0"
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

### Component Trait (component.rs)

```rust
use crate::message::Message;
use crate::delegate::BridgeDelegateTrait;
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

    #[error("JNI error: {0}")]
    JniError(String),
}

/// Trait for bridge components (equivalent to BridgeComponent subclass)
#[async_trait]
pub trait BridgeComponent: Send + Sync {
    /// Component name (must match web component name)
    fn name(&self) -> &str;

    /// Called when a message is received from the web
    fn on_receive(&mut self, message: Message) -> ComponentResult<()>;

    /// Lifecycle: Destination started
    fn on_start(&mut self) {}

    /// Lifecycle: Destination stopped
    fn on_stop(&mut self) {}

    /// Set the delegate (called by BridgeDelegate)
    fn set_delegate(&mut self, delegate: Arc<dyn BridgeDelegateTrait>);
}

/// Trait for bridge delegate (for components to reply)
#[async_trait]
pub trait BridgeDelegateTrait: Send + Sync {
    fn reply_with(&self, message: Message) -> ComponentResult<bool>;
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

    pub fn name(&self) -> &str {
        self.inner.name()
    }

    pub fn handle_message(&mut self, message: Message) -> ComponentResult<()> {
        // Cache message by event
        self.received_messages.insert(message.event.clone(), message.clone());
        // Forward to component
        self.inner.on_receive(message)
    }

    pub fn get_message(&self, event: &str) -> Option<&Message> {
        self.received_messages.get(event)
    }

    pub fn on_start(&mut self) {
        self.inner.on_start();
    }

    pub fn on_stop(&mut self) {
        self.inner.on_stop();
    }
}

/// Factory for creating components
pub type ComponentFactory = fn() -> BoxedComponent;
```

### Bridge Structure (bridge.rs)

```rust
use crate::message::Message;
use crate::component::{ComponentResult, BridgeDelegateTrait};
use crate::internal_message::InternalMessage;
use jni::objects::{JObject, JString, JValue};
use jni::sys::jboolean;
use jni::JNIEnv;
use std::sync::{Arc, RwLock};
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

    #[error("JNI error: {0}")]
    JniError(#[from] jni::errors::Error),
}

/// Main bridge struct (equivalent to Bridge.kt)
pub struct Bridge {
    webView: Arc<RwLock<Option<JObject<'static>>>>,
    delegate: RwLock<Option<Arc<dyn BridgeDelegateTrait>>>,
    components_are_registered: RwLock<bool>,
}

impl Bridge {
    /// Initialize bridge with a WebView
    pub fn new(webView: JObject) -> Self {
        // Note: In real implementation, need to handle JNI global refs properly
        Self {
            webView: Arc::new(RwLock::new(Some(webView))),
            delegate: RwLock::new(None),
            components_are_registered: RwLock::new(false),
        }
    }

    /// Get or create bridge for webview (singleton pattern)
    pub fn initialize(env: &mut JNIEnv, webView: JObject) -> Arc<Self> {
        static INSTANCES: once_cell::sync::Lazy<
            RwLock<Vec<Arc<Bridge>>>
        > = once_cell::sync::Lazy::new(|| RwLock::new(Vec::new()));

        // Check existing, create if needed
        {
            let instances = INSTANCES.read().unwrap();
            for bridge in instances.iter() {
                let web_view_lock = bridge.webView.read().unwrap();
                if let Some(existing_webview) = web_view_lock.as_ref() {
                    // Compare object references (implementation depends on JNI)
                    if env.is_same_object(*existing_webview, webView).unwrap_or(false) {
                        return Arc::clone(bridge);
                    }
                }
            }
        }

        // Create new bridge
        let bridge = Arc::new(Bridge::new(webView));
        INSTANCES.write().unwrap().push(Arc::clone(&bridge));
        bridge
    }

    /// Add JavascriptInterface to WebView
    pub fn add_javascript_interface(&self, env: &mut JNIEnv) -> ComponentResult<()> {
        // In Rust, we'd use jni-rs to call webView.addJavascriptInterface()
        // This requires implementing the interface as a Java class
        todo!("Implement JavascriptInterface via JNI")
    }

    /// Load the JavaScript bridge
    pub fn load(&self, env: &mut JNIEnv) -> ComponentResult<()> {
        let user_script = self.get_user_script(env)?;
        self.evaluate_javascript(env, &user_script)?;
        Ok(())
    }

    /// Get user script from assets
    fn get_user_script(&self, env: &mut JNIEnv) -> Result<String, BridgeError> {
        // Read strada.js from assets
        // This requires access to Android Context/AssetManager
        todo!("Implement asset loading via JNI")
    }

    /// Register a single component
    pub fn register_component(&self, env: &mut JNIEnv, component: &str) -> ComponentResult<()> {
        let javascript = self.generate_javascript("register", &serde_json::json!([component]));
        self.evaluate_javascript(env, &javascript)?;
        Ok(())
    }

    /// Register multiple components
    pub fn register_components(&self, env: &mut JNIEnv, components: &[&str]) -> ComponentResult<()> {
        let javascript = self.generate_javascript("register", &serde_json::json!(components));
        self.evaluate_javascript(env, &javascript)?;
        Ok(())
    }

    /// Reply to web with message
    pub fn reply_with(&self, env: &mut JNIEnv, message: Message) -> ComponentResult<bool> {
        debug!("bridgeWillReplyWithMessage: {:?}", message);

        let internal_msg = InternalMessage::from(&message);
        let json_value = serde_json::to_value(&internal_msg)?;

        let javascript = self.generate_javascript("replyWith", &json_value);
        self.evaluate_javascript(env, &javascript)?;
        Ok(true)
    }

    /// Evaluate JavaScript in WebView
    fn evaluate_javascript(&self, env: &mut JNIEnv, javascript: &str) -> ComponentResult<()> {
        let webview = self.webView.read().unwrap();
        let webview = webview.as_ref().ok_or(BridgeError::WebViewMissing)?;

        let js_string = env.new_string(javascript)?;

        // Call webView.evaluateJavascript(javascript, null)
        // Note: This is simplified - real implementation needs callback handling
        env.call_method(
            *webview,
            "evaluateJavascript",
            "(Ljava/lang/String;Landroid/webkit/ValueCallback;)V",
            &[JValue::Object(js_string.into()), JValue::Null],
        )?;

        Ok(())
    }

    fn generate_javascript(&self, function: &str, argument: &serde_json::Value) -> String {
        let func_name = function.strip_suffix("()").unwrap_or(function);
        format!("window.nativeBridge.{}({})", func_name, argument)
    }

    pub fn set_delegate(&self, delegate: Arc<dyn BridgeDelegateTrait>) {
        *self.delegate.write().unwrap() = Some(delegate);
    }

    pub fn is_ready(&self) -> bool {
        *self.components_are_registered.read().unwrap()
    }

    pub fn reset(&self) {
        *self.components_are_registered.write().unwrap() = false;
    }

    pub fn set_components_registered(&self) {
        *self.components_are_registered.write().unwrap() = true;
    }
}
```

### JNI Interface (lib.rs)

```rust
use jni::objects::{JClass, JObject, JString};
use jni::sys::{jboolean, jint, jobject};
use jni::JNIEnv;
use std::sync::Arc;
use once_cell::sync::Lazy;

mod bridge;
mod component;
mod delegate;
mod message;
mod internal_message;

use crate::bridge::Bridge;

/// Global bridge instances storage
static BRIDGES: Lazy<RwLock<Vec<Arc<Bridge>>>> = Lazy::new(|| RwLock::new(Vec::new()));

/// Native method: bridgeDidInitialize
#[no_mangle]
#[jni_name = "bridgeDidInitialize"]
pub extern "C" fn bridge_did_initialize(env: JNIEnv, this: JObject) {
    debug!("bridge_did_initialize called");

    // Get the bridge associated with this JavascriptInterface
    // Implementation depends on how we store the mapping
}

/// Native method: bridgeDidReceiveMessage
#[no_mangle]
#[jni_name = "bridgeDidReceiveMessage"]
pub extern "C" fn bridge_did_receive_message(
    env: JNIEnv,
    this: JObject,
    message: JString,
) {
    debug!("bridge_did_receive_message called");

    // Convert JString to Rust String
    let message_str: String = match env.get_string(&message) {
        Ok(s) => s.into(),
        Err(e) => {
            error!("Failed to get message string: {}", e);
            return;
        }
    };

    // Parse InternalMessage from JSON
    let internal_message: InternalMessage = match serde_json::from_str(&message_str) {
        Ok(msg) => msg,
        Err(e) => {
            error!("Failed to parse message JSON: {}", e);
            return;
        }
    };

    // Convert to Message and route to delegate
    let message = internal_message.to_message();

    // Find bridge and delegate, then route message
    todo!("Route message to appropriate component")
}

/// Native method: bridgeDidUpdateSupportedComponents
#[no_mangle]
#[jni_name = "bridgeDidUpdateSupportedComponents"]
pub extern "C" fn bridge_did_update_supported_components(
    env: JNIEnv,
    this: JObject,
) {
    debug!("bridge_did_update_supported_components called");

    // Mark components as registered
    todo!("Update component registration state")
}

/// Initialize bridge with WebView
#[no_mangle]
#[jni_name = "initialize"]
pub extern "C" fn initialize(
    env: JNIEnv,
    _class: JClass,
    webview: JObject,
) -> jlong {
    // Create new Bridge instance
    let bridge = Bridge::initialize(&mut env, webview);

    // Store and return pointer (or use handle-based approach)
    // This is simplified - real implementation needs proper memory management
    Arc::into_raw(bridge) as jlong
}
```

### Delegate Implementation (delegate.rs)

```rust
use crate::message::Message;
use crate::component::{BoxedComponent, ComponentResult, BridgeDelegateTrait, ComponentFactory};
use crate::bridge::Bridge;
use std::sync::{Arc, RwLock};
use std::collections::HashMap;
use tracing::{debug, warn};

pub struct BridgeDelegate<D: 'static> {
    location: String,
    destination: Arc<D>,
    component_factories: Vec<ComponentFactory>,
    bridge: RwLock<Option<Arc<Bridge>>>,
    destination_is_active: RwLock<bool>,
    initialized_components: RwLock<HashMap<String, BoxedComponent>>,
}

impl<D> BridgeDelegate<D> {
    pub fn new(
        location: String,
        destination: Arc<D>,
        component_factories: Vec<ComponentFactory>,
    ) -> Self {
        Self {
            location,
            destination,
            component_factories,
            bridge: RwLock::new(None),
            destination_is_active: RwLock::new(true),
            initialized_components: RwLock::new(HashMap::new()),
        }
    }

    pub fn set_bridge(&self, bridge: Arc<Bridge>) {
        *self.bridge.write().unwrap() = Some(bridge);
    }

    /// Handle message from bridge
    pub fn handle_message(&self, message: Message) -> ComponentResult<bool> {
        let is_active = *self.destination_is_active.read().unwrap();

        if !is_active {
            warn!("Message received but destination is inactive");
            return Ok(false);
        }

        // Get or create component
        let component_name = message.component.clone();
        let mut components = self.initialized_components.write().unwrap();

        let component = components
            .entry(component_name)
            .or_insert_with(|| {
                // Find factory for this component type
                for factory in &self.component_factories {
                    let component = factory();
                    if component.name() == message.component {
                        return component;
                    }
                }
                panic!("No factory registered for component: {}", message.component);
            });

        component.handle_message(message)?;
        Ok(true)
    }

    /// Lifecycle: Destination started
    pub fn on_start(&self) {
        *self.destination_is_active.write().unwrap() = true;
        let mut components = self.initialized_components.write().unwrap();
        for component in components.values_mut() {
            component.on_start();
        }
    }

    /// Lifecycle: Destination stopped
    pub fn on_stop(&self) {
        let mut components = self.initialized_components.write().unwrap();
        for component in components.values_mut() {
            component.on_stop();
        }
        *self.destination_is_active.write().unwrap() = false;
    }

    /// Register all components on init
    pub fn on_bridge_did_initialize(&self) -> ComponentResult<()> {
        let bridge = self.bridge.read().unwrap();
        if let Some(bridge) = bridge.as_ref() {
            let component_names: Vec<&str> = self.component_factories
                .iter()
                .map(|_| todo!("Get component name from factory"))
                .collect();

            // Need JNIEnv to call bridge methods
            todo!("Register components with bridge")
        }
        Ok(())
    }

    /// Cold boot page started
    pub fn on_cold_boot_page_started(&self) {
        let bridge = self.bridge.read().unwrap();
        if let Some(bridge) = bridge.as_ref() {
            bridge.reset();
        }
    }

    /// Cold boot page completed
    pub fn on_cold_boot_page_completed(&self) -> ComponentResult<()> {
        let bridge = self.bridge.read().unwrap();
        if let Some(bridge) = bridge.as_ref() {
            // Need JNIEnv to load bridge
            todo!("Load bridge after cold boot")
        }
        Ok(())
    }

    /// WebView attached
    pub fn on_webview_attached(&self, bridge: Arc<Bridge>) {
        self.set_bridge(bridge);
    }

    /// WebView detached
    pub fn on_webview_detached(&self) {
        *self.bridge.write().unwrap() = None;
    }
}

#[async_trait]
impl<D> BridgeDelegateTrait for BridgeDelegate<D> {
    fn reply_with(&self, message: Message) -> ComponentResult<bool> {
        let bridge = self.bridge.read().unwrap();
        match bridge.as_ref() {
            Some(bridge) => {
                // Need JNIEnv to call bridge methods
                todo!("Reply with message via bridge")
            }
            None => Err(ComponentError::BridgeUnavailable),
        }
    }
}
```

### Example Component Implementation

```rust
use strada_android::component::{BridgeComponent, BridgeDelegateTrait, ComponentResult};
use strada_android::message::Message;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

pub struct FormComponent<D> {
    name: String,
    delegate: Arc<dyn BridgeDelegateTrait>,
    _destination: std::marker::PhantomData<D>,
}

#[derive(Debug, Deserialize)]
struct FormData {
    submit_title: String,
}

#[derive(Debug, Serialize)]
struct FormReply {
    submitted: bool,
}

impl<D> BridgeComponent for FormComponent<D> {
    fn name(&self) -> &str {
        &self.name
    }

    fn on_receive(&mut self, message: Message) -> ComponentResult<()> {
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
        self.delegate = delegate;
    }
}

impl<D> FormComponent<D> {
    pub fn new(name: String, delegate: Arc<dyn BridgeDelegateTrait>) -> Self {
        Self {
            name,
            delegate,
            _destination: std::marker::PhantomData,
        }
    }

    pub fn perform_submit(&self) -> ComponentResult<bool> {
        self.reply_to("connect")
    }

    fn reply_to(&self, event: &str) -> ComponentResult<bool> {
        // Implementation would need access to cached messages
        // This is a simplified example
        todo!("Implement reply logic")
    }
}
```

### Factory Registration

```rust
// In lib.rs or bridge_component_factories.rs

fn form_component_factory() -> BoxedComponent {
    // Factory needs delegate - this is passed at creation time
    // This requires a different pattern than Kotlin
    todo!("Implement factory pattern for Rust")
}

pub fn get_component_factories() -> Vec<ComponentFactory> {
    vec![
        form_component_factory,
        // Add other component factories
    ]
}
```

## JavaScript Bridge (strada.js)

The JavaScript bridge remains similar to the Kotlin version:

```javascript
// Embedded in assets or loaded from file
(() => {
    class NativeBridge {
        constructor() {
            this.supportedComponents = []
            this.adapterIsRegistered = false
        }

        register(component) {
            if (Array.isArray(component)) {
                this.supportedComponents = this.supportedComponents.concat(component)
            } else {
                this.supportedComponents.push(component)
            }

            if (!this.adapterIsRegistered) {
                this.registerAdapter()
            }
            this.notifyBridgeOfSupportedComponentsUpdate()
        }

        registerAdapter() {
            this.adapterIsRegistered = true

            if (this.isStradaAvailable) {
                this.webBridge.setAdapter(this)
            } else {
                document.addEventListener("web-bridge:ready", () =>
                    this.webBridge.setAdapter(this))
            }
        }

        notifyBridgeOfSupportedComponentsUpdate() {
            this.supportedComponentsUpdated()

            if (this.isStradaAvailable) {
                this.webBridge.adapterDidUpdateSupportedComponents()
            }
        }

        replyWith(message) {
            if (this.isStradaAvailable) {
                this.webBridge.receive(JSON.parse(message))
            }
        }

        receive(message) {
            this.postMessage(JSON.stringify(message))
        }

        get platform() {
            return "android"
        }

        ready() {
            StradaNative.bridgeDidInitialize()
        }

        supportedComponentsUpdated() {
            StradaNative.bridgeDidUpdateSupportedComponents()
        }

        postMessage(message) {
            StradaNative.bridgeDidReceiveMessage(message)
        }

        get isStradaAvailable() {
            return window.Strada
        }

        get webBridge() {
            return window.Strada.web
        }
    }

    if (document.readyState === 'interactive' || document.readyState === 'complete') {
        initializeBridge()
    } else {
        document.addEventListener("DOMContentLoaded", () => {
            initializeBridge()
        })
    }

    function initializeBridge() {
        window.nativeBridge = new NativeBridge()
        window.nativeBridge.ready()
    }
})()
```

## Build Configuration for Android

### Cargo.toml Library Type

```toml
[lib]
crate-type = ["cdylib", "rlib"]
name = "strada_android"
```

`cdylib` creates a shared library (.so) for Android.

### Android Build Script (build.rs)

```rust
fn main() {
    let target_os = std::env::var("CARGO_CFG_TARGET_OS").unwrap();

    if target_os == "android" {
        // Android-specific build configuration
        println!("cargo:rustc-link-lib=log");
    }
}
```

### Building for Android

```bash
# Install Android NDK target
rustup target add aarch64-linux-android

# Build for Android
cargo build --release --target aarch64-linux-android

# Output: target/aarch64-linux-android/release/libstrada_android.so
```

### Android Gradle Integration

```kotlin
// build.gradle.kts
android {
    sourceSets {
        getByName("main") {
            jniLibs.srcDirs("target/aarch64-linux-android/release")
        }
    }
}

// Or use cargo-apk or similar tooling
```

### JNI Loading in Kotlin

```kotlin
// Load the native library
class StradaNative {
    companion object {
        init {
            System.loadLibrary("strada_android")
        }

        @JvmStatic
        external fun bridgeDidInitialize()

        @JvmStatic
        external fun bridgeDidReceiveMessage(message: String)

        @JvmStatic
        external fun bridgeDidUpdateSupportedComponents()
    }
}
```

## Challenges and Considerations

### 1. JNI Complexity

Rust-JNI interop is more complex than Kotlin-Java:
- Manual memory management for JNI references
- Exception handling across the boundary
- Thread attachment/detachment for native threads

### 2. Lifecycle Integration

Kotlin uses `DefaultLifecycleObserver`:
```kotlin
class BridgeDelegate : DefaultLifecycleObserver
```

Rust must manually hook into Android lifecycle:
```rust
// Need to create a Java class that implements LifecycleObserver
// and calls into Rust via JNI
```

### 3. WebView Threading

WebView methods must run on UI thread:
```kotlin
runOnUiThread {
    webView.evaluateJavascript(...)
}
```

Rust needs JNI calls to schedule on UI thread:
```rust
// Get Looper.getMainLooper()
// Create Handler
// Post Runnable with the JavaScript evaluation
```

### 4. Asset Loading

Kotlin reads from assets easily:
```kotlin
context.assets.open("js/strada.js").use { String(it.readBytes()) }
```

Rust needs JNI to access AssetManager:
```rust
// Get AssetManager from Context
// Open file
// Read contents
```

## Recommended Approach

For a production implementation:

1. **Hybrid Architecture**: Keep Kotlin wrapper for JNI/Android APIs
2. **Rust for Business Logic**: Message handling, component logic in Rust
3. **Kotlin for Platform**: WebView, lifecycle, asset loading

```
┌─────────────────────────────────────────────────────────┐
│                    Kotlin Layer                         │
│  - WebView interaction                                  │
│  - @JavascriptInterface                                 │
│  - LifecycleObserver                                    │
│  - Asset loading                                        │
│  - Threading (runOnUiThread)                            │
└─────────────────────────────────────────────────────────┘
                          │
                          │ JNI (FFI boundary)
                          ▼
┌─────────────────────────────────────────────────────────┐
│                    Rust Layer                           │
│  - Message serialization (serde_json)                   │
│  - Component logic                                      │
│  - State management                                     │
│  - Business rules                                       │
└─────────────────────────────────────────────────────────┘
```

### Kotlin Wrapper Example

```kotlin
// StradaNative.kt
class StradaNative(private val rustBridge: Long) { // Rust pointer
    companion object {
        init { System.loadLibrary("strada_android") }

        @JvmStatic external fun nativeInit(webView: WebView): Long
        @JvmStatic external fun nativeOnMessage(pointer: Long, message: String)
        @JvmStatic external fun nativeOnStart(pointer: Long)
        @JvmStatic external fun nativeOnStop(pointer: Long)
    }

    @JavascriptInterface
    fun bridgeDidReceiveMessage(message: String) {
        runOnUiThread {
            nativeOnMessage(rustBridge, message)
        }
    }
}
```

### Rust FFI

```rust
#[no_mangle]
pub extern "C" fn native_on_message(pointer: jlong, message: JString) {
    let bridge = unsafe { Arc::from_raw(pointer as *const Bridge) };
    // Process message
    // Arc::increment_strong_count to keep alive
}
```

## Testing Strategy

### Unit Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_message_serialization() {
        let msg = Message::new("1", "form", "connect", None, json!({}));
        let serialized = serde_json::to_string(&msg).unwrap();
        assert!(serialized.contains("form"));
    }

    #[test]
    fn test_message_deserialization() {
        let json = r#"{"id":"1","component":"form","event":"connect","data":{}}"#;
        let msg: Message = serde_json::from_str(json).unwrap();
        assert_eq!(msg.component, "form");
    }
}
```

### Integration Tests

Require Android emulator or Robolectric for full integration testing.

---

*This guide provides the architecture for reimplementing Strada Android in Rust. The key challenge is JNI interop and Android platform integration, which suggests a hybrid Kotlin/Rust approach is most practical.*
