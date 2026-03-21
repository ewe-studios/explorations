---
source: /home/darkvoid/Boxxed/@dev/repo-expolorations/strada-production
repository: N/A
revised_at: 2026-03-21T00:00:00Z
workspace: strada-rust-bridge
---

# Rust Revision: Strada Production WebView Bridge

## Overview

This document translates the Strada Production exploration into idiomatic Rust, providing a production-ready WebView bridge implementation. The approach uses a hybrid architecture where Rust handles business logic, message processing, and state management, while Kotlin/Swift handle platform-specific APIs through FFI boundaries.

Key architectural decisions:
- **Core logic in Rust**: Message serialization, bridge component management, navigation state, offline queues
- **Platform bindings**: swift-bridge for iOS, jni-rs for Android
- **Async runtime**: tokio for async operations (network, file I/O)
- **Error handling**: thiserror for error types, Result-based API
- **State management**: Thread-safe state with Arc<Mutex<>> patterns

## Workspace Structure

```
strada-rust-bridge/
├── Cargo.toml                      # Workspace definition
├── strada-core/                    # Core bridge logic (platform-agnostic)
│   ├── Cargo.toml
│   └── src/
│       ├── lib.rs                  # Public API exports
│       ├── bridge/
│       │   ├── mod.rs              # Bridge core module
│       │   ├── component.rs        # BridgeComponent trait
│       │   ├── delegate.rs         # BridgeDelegate
│       │   ├── message.rs          # Message types and serialization
│       │   └── registry.rs         # Component registry
│       ├── navigation/
│       │   ├── mod.rs              # Navigation module
│       │   ├── state.rs            # Navigation state
│       │   ├── history.rs          # Back stack management
│       │   └── deep_link.rs        # Deep link parsing/validation
│       ├── components/             # Built-in bridge components
│       │   ├── mod.rs
│       │   ├── page.rs             # Page component (nav bar state)
│       │   ├── form.rs             # Form component
│       │   ├── text.rs             # Text field component
│       │   └── overlay.rs          # Overlay/modal component
│       ├── offline/
│       │   ├── mod.rs              # Offline module
│       │   ├── queue.rs            # Offline action queue
│       │   ├── storage.rs          # Persistent storage
│       │   └── sync.rs             # Sync coordination
│       ├── security/
│       │   ├── mod.rs              # Security module
│       │   ├── cert_pinning.rs     # Certificate pinning
│       │   ├── secure_storage.rs   # Encrypted storage abstraction
│       │   └── webview_config.rs   # WebView hardening config
│       └── utils/
│           ├── mod.rs
│           ├── connectivity.rs     # Network monitoring
│           └── cache.rs            # Caching utilities
├── estrada-ios/                    # iOS FFI bindings (swift-bridge)
│   ├── Cargo.toml
│   └── src/
│       ├── lib.rs                  # iOS FFI exports
│       ├── ffi_types.rs            # FFI-compatible type definitions
│       └── platform/
│           ├── keychain.rs         # iOS Keychain wrapper
│           └── webview.rs          # WKWebView integration
├── estrada-android/                # Android FFI bindings (jni-rs)
│   ├── Cargo.toml
│   └── src/
│       ├── lib.rs                  # Android JNI exports
│       ├── ffi_types.rs            # JNI type conversions
│       └── platform/
│           ├── shared_prefs.rs     # SharedPreferences wrapper
│           └── webview.rs          # WebView integration
├── estrada-testing/                # Testing utilities
│   ├── Cargo.toml
│   └── src/
│       ├── lib.rs
│       ├── mock_bridge.rs          # Mock bridge for testing
│       ├── message_factory.rs      # Test message builders
│       └── fixtures/               # Test fixtures
└── examples/
    ├── ios-app/                    # Example iOS integration
    └── android-app/                # Example Android integration
```

### Crate Breakdown

#### strada-core
- **Purpose:** Platform-agnostic bridge logic, message handling, component management
- **Type:** library
- **Public API:** `Bridge`, `BridgeComponent`, `Message`, `BridgeDelegate` traits
- **Dependencies:** serde, serde_json, tokio, thiserror, tracing, uuid

#### estrada-ios
- **Purpose:** iOS FFI bindings using swift-bridge
- **Type:** library (cdylib for FFI)
- **Public API:** FFI functions for Swift interop
- **Dependencies:** strada-core, swift-bridge, block2, objc2

#### estrada-android
- **Purpose:** Android FFI bindings using jni-rs
- **Type:** library (cdylib for FFI)
- **Public API:** JNI native methods
- **Dependencies:** strada-core, jni, jni-macros, android_logger

#### estrada-testing
- **Purpose:** Testing utilities and mocks
- **Type:** library
- **Public API:** Mock components, message factories
- **Dependencies:** strada-core, tokio-test

## Recommended Dependencies

| Purpose | Crate | Version | Rationale |
|---------|-------|---------|-----------|
| Serialization | serde + serde_json | 1.0 | Industry standard, derive macros |
| Async runtime | tokio | 1.0 | Mature, feature-rich async runtime |
| Error handling | thiserror | 1.0 | Ergonomic error type derivation |
| Logging | tracing + tracing-subscriber | 0.1 | Structured logging, async-aware |
| UUID generation | uuid | 1.0 | Message ID generation |
| iOS FFI | swift-bridge | 0.1 | Clean Swift/Rust interop |
| Android FFI | jni | 0.21 | Mature JNI bindings |
| HTTP client | reqwest | 0.11 | For offline queue sync |
| Encryption | ring | 0.17 | For secure storage (platform-agnostic crypto) |
| State management | tokio::sync::Mutex | - | Async-safe interior mutability |
| Time | chrono | 0.4 | Timestamp handling |

## Type System Design

### Core Types

```rust
// strada-core/src/bridge/message.rs

use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Unique message identifier
pub type MessageId = String;

/// Component identifier
pub type ComponentName = String;

/// Metadata for message context
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageMetadata {
    pub url: String,
    pub title: Option<String>,
    pub timestamp: i64,
}

/// Core message type for bridge communication
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub id: MessageId,
    pub component: ComponentName,
    pub event: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<MessageMetadata>,
    #[serde(rename = "jsonData")]
    pub json_data: String,
}

impl Message {
    pub fn new(component: impl Into<String>, event: impl Into<String>) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            component: component.into(),
            event: event.into(),
            metadata: None,
            json_data: "{}".to_string(),
        }
    }

    pub fn with_data<T: Serialize>(
        component: impl Into<String>,
        event: impl Into<String>,
        data: &T,
    ) -> Result<Self, serde_json::Error> {
        let json_data = serde_json::to_string(data)?;
        Ok(Self {
            id: Uuid::new_v4().to_string(),
            component: component.into(),
            event: event.into(),
            metadata: None,
            json_data,
        })
    }

    pub fn data<T: for<'de> Deserialize<'de>>(&self) -> Result<T, serde_json::Error> {
        serde_json::from_str(&self.json_data)
    }

    pub fn reply(&self, event: impl Into<String>, data: &impl Serialize) -> Result<Self, serde_json::Error> {
        Self::with_data(self.component.clone(), event, data)
    }
}
```

### Error Types

```rust
// strada-core/src/error.rs

use thiserror::Error;

#[derive(Debug, Error)]
pub enum BridgeError {
    #[error("Component not found: {0}")]
    ComponentNotFound(String),

    #[error("Message serialization failed: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("Invalid message format: {0}")]
    InvalidMessage(String),

    #[error("Navigation failed: {0}")]
    Navigation(String),

    #[error("Offline queue error: {0}")]
    OfflineQueue(String),

    #[error("Security error: {0}")]
    Security(String),

    #[error("Platform error: {0}")]
    Platform(String),

    #[error("Async runtime error: {0}")]
    Runtime(#[from] tokio::task::JoinError),
}

pub type Result<T> = std::result::Result<T, BridgeError>;
```

### Traits

```rust
// strada-core/src/bridge/component.rs

use crate::bridge::message::Message;
use crate::error::Result;

/// Trait for all bridge components
pub trait BridgeComponent: Send + Sync {
    /// Component name (e.g., "page", "form", "text")
    fn name(&self) -> &str;

    /// Handle incoming message from web
    fn on_receive(&mut self, message: &Message) -> Result<Option<Message>>;

    /// Called when component is registered
    fn on_init(&mut self) -> Result<()> {
        Ok(())
    }

    /// Called when web view loads
    fn on_web_view_load(&mut self) -> Result<()> {
        Ok(())
    }
}

// strada-core/src/bridge/delegate.rs

use super::message::Message;
use crate::error::Result;

/// Delegate for bridge communication with platform layer
pub trait BridgeDelegate: Send + Sync {
    /// Send message to web
    fn send_to_web(&self, message: Message) -> Result<()>;

    /// Reply to a message
    fn reply(&self, message: Message) -> Result<()> {
        self.send_to_web(message)
    }

    /// Evaluate JavaScript in web view
    fn evaluate_javascript(&self, script: &str) -> Result<()>;
}
```

### Bridge State

```rust
// strada-core/src/bridge/mod.rs

use crate::bridge::component::BridgeComponent;
use crate::bridge::delegate::BridgeDelegate;
use crate::bridge::message::Message;
use crate::error::{BridgeError, Result};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;

/// Main bridge coordinator
pub struct Bridge {
    /// Registered components
    components: Arc<Mutex<HashMap<String, Box<dyn BridgeComponent>>>>,
    /// Delegate for platform communication
    delegate: Arc<dyn BridgeDelegate>,
    /// Current navigation state
    navigation_state: Arc<Mutex<NavigationState>>,
}

impl Bridge {
    pub fn new(delegate: Arc<dyn BridgeDelegate>) -> Self {
        Self {
            components: Arc::new(Mutex::new(HashMap::new())),
            delegate,
            navigation_state: Arc::new(Mutex::new(NavigationState::default())),
        }
    }

    /// Register a bridge component
    pub fn register_component<C: BridgeComponent + 'static>(&mut self, component: C) {
        // Registration logic
    }

    /// Handle message from web
    pub async fn handle_message(&self, message: Message) -> Result<()> {
        let mut components = self.components.lock().await;
        if let Some(component) = components.get_mut(&message.component) {
            if let Some(reply) = component.on_receive(&message)? {
                self.delegate.reply(reply)?;
            }
        } else {
            return Err(BridgeError::ComponentNotFound(message.component));
        }
        Ok(())
    }

    /// Send message to web
    pub fn send_to_web(&self, message: Message) -> Result<()> {
        self.delegate.send_to_web(message)
    }
}
```

## Navigation & Routing in Rust

```rust
// strada-core/src/navigation/mod.rs

use std::sync::Arc;
use tokio::sync::Mutex;

/// Navigation state holder
#[derive(Debug, Clone)]
pub struct NavigationState {
    pub current_url: String,
    pub title: Option<String>,
    pub can_go_back: bool,
    pub can_go_forward: bool,
    pub history: Vec<HistoryEntry>,
}

#[derive(Debug, Clone)]
pub struct HistoryEntry {
    pub url: String,
    pub title: Option<String>,
    pub timestamp: i64,
}

impl NavigationState {
    pub fn new(url: impl Into<String>) -> Self {
        Self {
            current_url: url.into(),
            title: None,
            can_go_back: false,
            can_go_forward: false,
            history: Vec::new(),
        }
    }

    pub fn update(&mut self, url: String, title: Option<String>) {
        // Push current to history
        if let Some(current_title) = self.title.clone() {
            self.history.push(HistoryEntry {
                url: self.current_url.clone(),
                title: Some(current_title),
                timestamp: chrono::Utc::now().timestamp(),
            });
        }

        self.current_url = url;
        self.title = title;
        self.can_go_back = !self.history.is_empty();
        self.can_go_forward = false; // Simplified
    }

    pub fn go_back(&mut self) -> Option<HistoryEntry> {
        self.history.pop()
    }
}

/// Deep link handler
pub struct DeepLinkHandler {
    allowed_hosts: Vec<String>,
    allowed_path_prefixes: Vec<String>,
}

impl DeepLinkHandler {
    pub fn new(allowed_hosts: Vec<String>, allowed_path_prefixes: Vec<String>) -> Self {
        Self {
            allowed_hosts,
            allowed_path_prefixes,
        }
    }

    pub fn validate(&self, url: &str) -> Result<bool, DeepLinkError> {
        let parsed = url::Url::parse(url)
            .map_err(|_| DeepLinkError::InvalidUrl)?;

        let host = parsed.host_str().ok_or(DeepLinkError::InvalidHost)?;

        if !self.allowed_hosts.iter().any(|h| h == host || host.ends_with(h)) {
            return Err(DeepLinkError::HostNotAllowed(host.to_string()));
        }

        let path = parsed.path();
        if !self.allowed_path_prefixes.iter().any(|p| path.starts_with(p)) {
            return Err(DeepLinkError::PathNotAllowed(path.to_string()));
        }

        Ok(true)
    }

    pub fn parse_path(&self, url: &str) -> Option<DeepLinkPath> {
        let parsed = url::Url::parse(url).ok()?;
        Some(DeepLinkPath {
            path: parsed.path().to_string(),
            segments: parsed.path_segments()?.collect(),
            query: parsed.query().map(|q| q.to_string()),
        })
    }
}

#[derive(Debug)]
pub struct DeepLinkPath {
    pub path: String,
    pub segments: Vec<String>,
    pub query: Option<String>,
}

#[derive(Debug, thiserror::Error)]
pub enum DeepLinkError {
    #[error("Invalid URL format")]
    InvalidUrl,
    #[error("Invalid host")]
    InvalidHost,
    #[error("Host not allowed: {0}")]
    HostNotAllowed(String),
    #[error("Path not allowed: {0}")]
    PathNotAllowed(String),
}
```

## Native UI Components in Rust

```rust
// strada-core/src/components/page.rs

use crate::bridge::component::BridgeComponent;
use crate::bridge::delegate::BridgeDelegate;
use crate::bridge::message::Message;
use crate::error::Result;
use serde::{Deserialize, Serialize};

/// Page component for navigation bar state
pub struct PageComponent<D: BridgeDelegate> {
    delegate: D,
    current_state: Option<PageState>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PageState {
    pub title: Option<String>,
    pub show_back_button: bool,
    pub right_bar_button_items: Vec<BarButtonAction>,
    pub loading: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BarButtonAction {
    pub identifier: String,
    pub title: String,
    pub icon_name: Option<String>,
}

impl<D: BridgeDelegate> PageComponent<D> {
    pub fn new(delegate: D) -> Self {
        Self {
            delegate,
            current_state: None,
        }
    }

    fn update_navigation_bar(&mut self, data: PageData) -> Result<()> {
        self.current_state = Some(PageState {
            title: data.title,
            show_back_button: data.show_back_button.unwrap_or(true),
            right_bar_button_items: data.right_bar_button_items.unwrap_or_default(),
            loading: false,
        });

        // Send command to native layer to update UI
        // Platform-specific implementation handles actual UI update
        Ok(())
    }

    fn show_loading(&mut self) -> Result<()> {
        if let Some(state) = &mut self.current_state {
            state.loading = true;
        }
        // Platform sends loading indicator command
        Ok(())
    }

    fn hide_loading(&mut self) -> Result<()> {
        if let Some(state) = &mut self.current_state {
            state.loading = false;
        }
        Ok(())
    }
}

impl<D: BridgeDelegate> BridgeComponent for PageComponent<D> {
    fn name(&self) -> &str {
        "page"
    }

    fn on_receive(&mut self, message: &Message) -> Result<Option<Message>> {
        match message.event.as_str() {
            "connect" => {
                let data: PageData = message.data()?;
                self.update_navigation_bar(data)?;
                Ok(None)
            }
            "navigation-state" => {
                let data: NavigationData = message.data()?;
                if let Some(state) = &mut self.current_state {
                    state.title = data.title;
                }
                Ok(None)
            }
            "show-native-loading" => {
                self.show_loading()?;
                Ok(None)
            }
            "hide-native-loading" => {
                self.hide_loading()?;
                Ok(None)
            }
            "back-tapped" => {
                // Request native back navigation
                let reply = message.reply("back-handled", &BackHandledData {
                    action: "native-back".to_string(),
                    handled: true,
                })?;
                Ok(Some(reply))
            }
            _ => Ok(None),
        }
    }
}
```

## Performance Optimization in Rust

```rust
// strada-core/src/utils/webview_pool.rs

use std::collections::VecDeque;
use std::sync::Arc;
use tokio::sync::Mutex;

/// WebView pool for pre-loaded instances
/// Note: Actual WebView handling is platform-specific
/// This manages the pool state and coordination
pub struct WebViewPoolConfig {
    pub max_pool_size: usize,
    pub preload_enabled: bool,
    pub preload_urls: Vec<String>,
}

impl Default for WebViewPoolConfig {
    fn default() -> Self {
        Self {
            max_pool_size: 2,
            preload_enabled: true,
            preload_urls: Vec::new(),
        }
    }
}

/// Pool state (platform implementations manage actual WebViews)
pub struct WebViewPoolState {
    pub available_count: usize,
    pub in_use_count: usize,
    pub last_acquired: Option<i64>,
}

/// WebView pool manager
/// Platform implementations will integrate this with actual WebView instances
pub struct WebViewPool {
    config: WebViewPoolConfig,
    state: Arc<Mutex<WebViewPoolState>>,
}

impl WebViewPool {
    pub fn new(config: WebViewPoolConfig) -> Self {
        Self {
            config,
            state: Arc::new(Mutex::new(WebViewPoolState {
                available_count: 0,
                in_use_count: 0,
                last_acquired: None,
            })),
        }
    }

    pub async fn acquire(&self) -> Result<WebViewPoolHandle> {
        let mut state = self.state.lock().await;
        state.available_count = state.available_count.saturating_sub(1);
        state.in_use_count += 1;
        state.last_acquired = Some(chrono::Utc::now().timestamp());

        Ok(WebViewPoolHandle {
            id: uuid::Uuid::new_v4().to_string(),
            // Platform creates/returns actual WebView
        })
    }

    pub async fn release(&self, handle: WebViewPoolHandle) -> Result<()> {
        let mut state = self.state.lock().await;
        state.in_use_count = state.in_use_count.saturating_sub(1);

        if state.available_count < self.config.max_pool_size {
            state.available_count += 1;
            // Platform returns WebView to pool
        }

        Ok(())
    }
}

pub struct WebViewPoolHandle {
    id: String,
    // Platform-specific WebView reference would be here
}

// Memory management
pub struct MemoryManager {
    threshold_mb: usize,
}

impl MemoryManager {
    pub fn new(threshold_mb: usize) -> Self {
        Self { threshold_mb }
    }

    pub fn check_memory_pressure(&self) -> MemoryPressure {
        // Platform-specific implementation
        // iOS: Check processInfo.processInfoMemoryUsage
        // Android: Check ActivityManager.getMemoryInfo
        MemoryPressure::Normal
    }

    pub fn handle_memory_warning(&self) {
        // Clear caches, release pooled resources
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum MemoryPressure {
    Normal,
    Warning,
    Critical,
}
```

## Security in Rust

```rust
// strada-core/src/security/cert_pinning.rs

use std::collections::HashSet;
use thiserror::Error;

/// Certificate pinning configuration
pub struct CertificatePinner {
    pinned_hashes: HashSet<String>,
}

impl CertificatePinner {
    pub fn new(pinned_hashes: Vec<String>) -> Self {
        Self {
            pinned_hashes: pinned_hashes.into_iter().collect(),
        }
    }

    pub fn validate(&self, certificate_hash: &str) -> Result<bool, CertificatePinError> {
        if self.pinned_hashes.contains(certificate_hash) {
            Ok(true)
        } else {
            Err(CertificatePinError::CertificateMismatch)
        }
    }
}

#[derive(Debug, Error)]
pub enum CertificatePinError {
    #[error("Certificate hash does not match pinned certificates")]
    CertificateMismatch,
    #[error("No pinned certificates configured")]
    NoPinnedCerts,
}

// strada-core/src/security/secure_storage.rs

use crate::error::Result;

/// Trait for secure storage operations
/// Platform implementations provide actual encryption
pub trait SecureStorage: Send + Sync {
    /// Store a value securely
    fn store(&self, key: &str, value: &str) -> Result<()>;

    /// Retrieve a value
    fn retrieve(&self, key: &str) -> Result<Option<String>>;

    /// Delete a value
    fn delete(&self, key: &str) -> Result<()>;

    /// Clear all stored values
    fn clear_all(&self) -> Result<()>;
}

/// Secure storage with accessibility levels
pub enum StorageAccessibility {
    /// Available when device is unlocked
    WhenUnlocked,
    /// Available after first unlock (until restart)
    WhenUnlockedThisDeviceOnly,
    /// Always available (less secure)
    Always,
}

pub struct SecureStorageConfig {
    pub accessibility: StorageAccessibility,
    pub require_biometric: bool,
}

impl Default for SecureStorageConfig {
    fn default() -> Self {
        Self {
            accessibility: StorageAccessibility::WhenUnlocked,
            require_biometric: false,
        }
    }
}
```

## Offline & Connectivity in Rust

```rust
// strada-core/src/offline/queue.rs

use crate::error::{BridgeError, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::Mutex;

/// Offline action queue manager
pub struct OfflineQueue {
    actions: Arc<Mutex<Vec<QueuedAction>>>,
    max_retries: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueuedAction {
    pub id: String,
    pub action_type: String,
    pub payload: serde_json::Value,
    pub created_at: DateTime<Utc>,
    pub retry_count: u32,
    pub last_attempt: Option<DateTime<Utc>>,
}

impl OfflineQueue {
    pub fn new(max_retries: u32) -> Self {
        Self {
            actions: Arc::new(Mutex::new(Vec::new())),
            max_retries,
        }
    }

    /// Add action to queue
    pub async fn enqueue(&self, action_type: String, payload: serde_json::Value) -> Result<String> {
        let mut actions = self.actions.lock().await;
        let id = uuid::Uuid::new_v4().to_string();

        let action = QueuedAction {
            id: id.clone(),
            action_type,
            payload,
            created_at: Utc::now(),
            retry_count: 0,
            last_attempt: None,
        };

        actions.push(action);
        Ok(id)
    }

    /// Get next pending action
    pub async fn dequeue(&self) -> Result<Option<QueuedAction>> {
        let mut actions = self.actions.lock().await;
        Ok(actions.first().cloned())
    }

    /// Mark action as completed
    pub async fn complete(&self, action_id: &str) -> Result<()> {
        let mut actions = self.actions.lock().await;
        actions.retain(|a| a.id != action_id);
        Ok(())
    }

    /// Mark action as failed, increment retry count
    pub async fn fail(&self, action_id: &str) -> Result<bool> {
        let mut actions = self.actions.lock().await;

        for action in actions.iter_mut() {
            if action.id == action_id {
                action.retry_count += 1;
                action.last_attempt = Some(Utc::now());

                if action.retry_count >= self.max_retries {
                    // Remove from queue, mark as permanently failed
                    return Ok(false);
                }
                return Ok(true); // Can retry
            }
        }

        Ok(false)
    }

    /// Get queue status
    pub async fn status(&self) -> Result<QueueStatus> {
        let actions = self.actions.lock().await;
        Ok(QueueStatus {
            queue_length: actions.len(),
            has_pending: !actions.is_empty(),
        })
    }

    /// Process all pending actions (called when connectivity restored)
    pub async fn process_all<F>(&self, executor: F) -> Result<ProcessResult>
    where
        F: Fn(QueuedAction) -> futures::future::BoxFuture<'static, Result<()>>
            + Send
            + Sync
            + 'static,
    {
        let executor = Arc::new(executor);
        let actions = self.actions.lock().await.clone();
        let mut result = ProcessResult::default();

        for action in actions {
            match executor(action.clone()).await {
                Ok(()) => {
                    self.complete(&action.id).await?;
                    result.succeeded.push(action.id);
                }
                Err(_) => {
                    let can_retry = self.fail(&action.id).await?;
                    if !can_retry {
                        result.permanently_failed.push(action.id);
                    } else {
                        result.failed_but_retrying.push(action.id);
                    }
                }
            }
        }

        Ok(result)
    }
}

#[derive(Debug, Clone, Default)]
pub struct QueueStatus {
    pub queue_length: usize,
    pub has_pending: bool,
}

#[derive(Debug, Clone, Default)]
pub struct ProcessResult {
    pub succeeded: Vec<String>,
    pub failed_but_retrying: Vec<String>,
    pub permanently_failed: Vec<String>,
}
```

```rust
// strada-core/src/utils/connectivity.rs

use std::sync::Arc;
use tokio::sync::Mutex;

/// Network connectivity monitor
pub struct ConnectivityMonitor {
    state: Arc<Mutex<ConnectivityState>>,
    listeners: Arc<Mutex<Vec<Box<dyn Fn(bool) + Send + Sync>>>>,
}

#[derive(Debug, Clone)]
pub struct ConnectivityState {
    pub is_connected: bool,
    pub connection_type: ConnectionType,
    pub is_expensive: bool,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ConnectionType {
    WiFi,
    Cellular,
    Ethernet,
    Unknown,
}

impl ConnectivityMonitor {
    pub fn new() -> Self {
        Self {
            state: Arc::new(Mutex::new(ConnectivityState {
                is_connected: false,
                connection_type: ConnectionType::Unknown,
                is_expensive: false,
            })),
            listeners: Arc::new(Mutex::new(Vec::new())),
        }
    }

    /// Update connectivity state (called by platform layer)
    pub async fn update_state(&self, state: ConnectivityState) {
        let mut current = self.state.lock().await;
        let changed = current.is_connected != state.is_connected;
        *current = state;

        if changed {
            self.notify_listeners(current.is_connected).await;
        }
    }

    /// Check current connectivity
    pub async fn is_connected(&self) -> bool {
        self.state.lock().await.is_connected
    }

    /// Get current connectivity state
    pub async fn get_state(&self) -> ConnectivityState {
        self.state.lock().await.clone()
    }

    /// Add connectivity change listener
    pub async fn add_listener<F>(&self, listener: F)
    where
        F: Fn(bool) + Send + Sync + 'static,
    {
        let mut listeners = self.listeners.lock().await;
        listeners.push(Box::new(listener));
    }

    async fn notify_listeners(&self, connected: bool) {
        let listeners = self.listeners.lock().await;
        for listener in listeners.iter() {
            listener(connected);
        }
    }
}
```

## Testing in Rust

```rust
// estrada-testing/src/mock_bridge.rs

use estrada_core::bridge::component::BridgeComponent;
use estrada_core::bridge::delegate::BridgeDelegate;
use estrada_core::bridge::message::Message;
use estrada_core::error::Result;
use std::sync::Arc;
use tokio::sync::Mutex;

/// Mock delegate for testing
pub struct MockBridgeDelegate {
    pub sent_messages: Arc<Mutex<Vec<Message>>>,
}

impl MockBridgeDelegate {
    pub fn new() -> Self {
        Self {
            sent_messages: Arc::new(Mutex::new(Vec::new())),
        }
    }

    pub async fn get_sent_messages(&self) -> Vec<Message> {
        self.sent_messages.lock().await.clone()
    }

    pub async fn last_sent_message(&self) -> Option<Message> {
        self.sent_messages.lock().await.last().cloned()
    }
}

impl BridgeDelegate for MockBridgeDelegate {
    fn send_to_web(&self, message: Message) -> Result<()> {
        // In a real scenario, this would be async
        // For testing, we use a blocking approach
        let rt = tokio::runtime::Handle::current();
        rt.block_on(async {
            self.sent_messages.lock().await.push(message);
        });
        Ok(())
    }

    fn evaluate_javascript(&self, _script: &str) -> Result<()> {
        Ok(())
    }
}

/// Message factory for testing
pub struct MessageFactory;

impl MessageFactory {
    pub fn connect(component: &str, data: &impl serde::Serialize) -> Message {
        Message::with_data(component, "connect", data).unwrap()
    }

    pub fn navigation_state(component: &str, title: &str) -> Message {
        #[derive(serde::Serialize)]
        struct NavData<'a> {
            title: &'a str,
            can_go_back: bool,
        }

        Message::with_data(
            component,
            "navigation-state",
            &NavData {
                title,
                can_go_back: true,
            },
        )
        .unwrap()
    }

    pub fn back_tapped(component: &str) -> Message {
        Message::new(component, "back-tapped")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use estrada_core::components::page::PageComponent;
    use estrada_core::components::page::PageData;

    #[tokio::test]
    async fn test_page_component_connect() {
        let delegate = MockBridgeDelegate::new();
        let mut component = PageComponent::new(delegate);

        let page_data = PageData {
            title: Some("Test Page".to_string()),
            show_back_button: Some(true),
            right_bar_button_items: None,
        };

        let message = MessageFactory::connect("page", &page_data);
        let result = component.on_receive(&message);

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_page_component_navigation_state() {
        let delegate = MockBridgeDelegate::new();
        let mut component = PageComponent::new(delegate);

        let message = MessageFactory::navigation_state("page", "New Title");
        let result = component.on_receive(&message);

        assert!(result.is_ok());
    }
}
```

### Integration Testing Example

```rust
// estrada-testing/src/integration.rs

use estrada_core::bridge::Bridge;
use estrada_core::components::page::PageComponent;
use crate::mock_bridge::MockBridgeDelegate;

/// Integration test helper
pub struct BridgeIntegrationTest {
    bridge: Bridge,
    delegate: MockBridgeDelegate,
}

impl BridgeIntegrationTest {
    pub fn new() -> Self {
        let delegate = MockBridgeDelegate::new();
        let mut bridge = Bridge::new(std::sync::Arc::new(delegate.clone()));

        // Register default components
        bridge.register_component(PageComponent::new(delegate.clone()));

        Self { bridge, delegate }
    }

    pub async fn send_message(&self, message: Message) -> Result<()> {
        self.bridge.handle_message(message).await
    }

    pub async fn get_sent_messages(&self) -> Vec<Message> {
        self.delegate.get_sent_messages().await
    }

    #[tokio::test]
    async fn test_full_navigation_flow() {
        let test = BridgeIntegrationTest::new();

        // Simulate page load
        let connect_msg = MessageFactory::connect("page", &PageData {
            title: Some("Home".to_string()),
            show_back_button: Some(false),
            right_bar_button_items: None,
        });
        test.send_message(connect_msg).await.unwrap();

        // Simulate navigation
        let nav_msg = MessageFactory::navigation_state("page", "Details");
        test.send_message(nav_msg).await.unwrap();

        // Verify messages sent to web
        let sent = test.get_sent_messages().await;
        assert!(!sent.is_empty());
    }
}
```

## Deployment & CI/CD for Rust

```yaml
# .github/workflows/rust-cicd.yml

name: Rust CI/CD

on:
  push:
    branches: [main, develop]
  pull_request:
    branches: [main]

env:
  CARGO_TERM_COLOR: always

jobs:
  lint:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4

      - name: Install Rust
        uses: dtolnay/rust-action@stable
        with:
          components: clippy, rustfmt

      - name: Run rustfmt
        run: cargo fmt --all -- --check

      - name: Run clippy
        run: cargo clippy --workspace --all-targets -- -D warnings

  test:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4

      - name: Install Rust
        uses: dtolnay/rust-action@stable

      - name: Run tests
        run: cargo test --workspace --all-targets

      - name: Upload coverage
        uses: actions/upload-artifact@v4
        with:
          name: coverage
          path: target/coverage/

  build-ios:
    runs-on: macos-14
    needs: [lint, test]
    steps:
      - uses: actions/checkout@v4

      - name: Install Rust
        uses: dtolnay/rust-action@stable
        with:
          targets: aarch64-apple-ios, x86_64-apple-ios

      - name: Build iOS library
        run: |
          cargo build --release --target aarch64-apple-ios -p estrada-ios

      - name: Create XCFramework
        run: |
          # Create XCFramework from built library
          xcodebuild -createXCFramework \
            -library target/aarch64-apple-ios/release/libstrada_ios.a \
            -output estrada-ios/StradaRust.xcframework

  build-android:
    runs-on: ubuntu-latest
    needs: [lint, test]
    steps:
      - uses: actions/checkout@v4

      - name: Install Rust
        uses: dtolnay/rust-action@stable
        with:
          targets: aarch64-linux-android, armv7-linux-androideabi

      - name: Install Android NDK
        uses: nttld/setup-ndk@v1

      - name: Build Android library
        run: |
          cargo build --release \
            --target aarch64-linux-android \
            -p estrada-android

      - name: Upload artifacts
        uses: actions/upload-artifact@v4
        with:
          name: android-libs
          path: target/aarch64-linux-android/release/libstrada_android.so
```

## Accessibility in Rust

```rust
// strada-core/src/accessibility/mod.rs

use crate::bridge::component::BridgeComponent;
use crate::bridge::message::Message;
use crate::error::Result;
use serde::{Deserialize, Serialize};

/// Accessibility manager for screen reader announcements
pub struct AccessibilityManager {
    // Platform-specific implementations handle actual accessibility
}

impl AccessibilityManager {
    /// Post an announcement to screen reader
    pub fn announce(&self, message: &str) -> Result<()> {
        // Platform sends to VoiceOver/TalkBack
        Ok(())
    }

    /// Post notification (page changed, layout changed)
    pub fn post_notification(&self, notification: AccessibilityNotification) -> Result<()> {
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub enum AccessibilityNotification {
    PageScrolled(String),  // Argument is page title/heading
    LayoutChanged,
    Announcement(String),
}

/// Accessibility bridge component
pub struct AccessibilityComponent {
    manager: AccessibilityManager,
}

impl AccessibilityComponent {
    pub fn new() -> Self {
        Self {
            manager: AccessibilityManager,
        }
    }
}

impl BridgeComponent for AccessibilityComponent {
    fn name(&self) -> &str {
        "accessibility"
    }

    fn on_receive(&mut self, message: &Message) -> Result<Option<Message>> {
        match message.event.as_str() {
            "announce" => {
                let data: AnnounceData = message.data()?;
                self.manager.announce(&data.message)?;
                Ok(None)
            }
            "move-focus" => {
                let data: FocusData = message.data()?;
                // Platform moves focus to specified target
                Ok(None)
            }
            _ => Ok(None),
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AnnounceData {
    pub message: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct FocusData {
    pub target: String,  // "webview", "error", "header", etc.
    pub message: Option<String>,
}
```

## Key Rust-Specific Changes

### 1. Message Passing Architecture

**Source Pattern:** JavaScript objects sent via `window.postMessage`

**Rust Translation:** Strongly-typed `Message` struct with serde serialization

**Rationale:** Type safety at compile time, clear error handling, IDE support

```rust
// Instead of loose JavaScript objects:
// { component: 'page', event: 'connect', data: {...} }

// Use strongly-typed Rust struct:
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub id: MessageId,
    pub component: ComponentName,
    pub event: String,
    pub json_data: String,
}

impl Message {
    pub fn data<T: for<'de> Deserialize<'de>>(&self) -> Result<T, serde_json::Error> {
        serde_json::from_str(&self.json_data)
    }
}
```

### 2. Component Registry Pattern

**Source Pattern:** Class-based inheritance in Swift/Kotlin

**Rust Translation:** Trait-based component system with dynamic registration

**Rationale:** Compile-time checking, flexible composition, no inheritance

```rust
pub trait BridgeComponent: Send + Sync {
    fn name(&self) -> &str;
    fn on_receive(&mut self, message: &Message) -> Result<Option<Message>>;
}

// Registry with trait objects
pub struct ComponentRegistry {
    components: HashMap<String, Box<dyn BridgeComponent>>,
}
```

### 3. Async State Management

**Source Pattern:** Callbacks and delegates in Swift/Kotlin

**Rust Translation:** Async traits with `tokio::sync::Mutex`

**Rationale:** Thread-safe, composable async operations

```rust
pub struct Bridge {
    components: Arc<Mutex<HashMap<String, Box<dyn BridgeComponent>>>>,
    state: Arc<Mutex<BridgeState>>,
}

impl Bridge {
    pub async fn handle_message(&self, message: Message) -> Result<()> {
        let mut components = self.components.lock().await;
        // Process message
    }
}
```

## Ownership & Borrowing Strategy

```rust
// Bridge ownership model
pub struct Bridge {
    // Shared ownership of components
    components: Arc<Mutex<HashMap<String, Box<dyn BridgeComponent>>>>,

    // Shared delegate for platform communication
    delegate: Arc<dyn BridgeDelegate>,

    // Navigation state with interior mutability
    navigation_state: Arc<Mutex<NavigationState>>,
}

// Components take mutable self reference for state changes
impl BridgeComponent for PageComponent {
    fn on_receive(&mut self, message: &Message) -> Result<Option<Message>> {
        // Mutable access to component state
        self.current_state = Some(new_state);
        Ok(None)
    }
}

// Messages are cloned when sent between threads
impl Bridge {
    pub fn send_to_web(&self, message: Message) -> Result<()> {
        // Message is moved to delegate
        self.delegate.send_to_web(message)
    }
}
```

## Concurrency Model

**Approach:** Async with tokio runtime

**Rationale:**
- Non-blocking I/O for network operations
- Efficient task scheduling
- Integration with platform async (Swift concurrency, Kotlin coroutines)

```rust
// Async component handling
impl Bridge {
    pub async fn process_message(&self, message: Message) -> Result<()> {
        let mut components = self.components.lock().await;

        if let Some(component) = components.get_mut(&message.component) {
            // Async processing
            let reply = component.on_receive(&message)?;

            if let Some(reply_msg) = reply {
                self.delegate.send_to_web(reply_msg)?;
            }
        }
        Ok(())
    }
}

// Parallel offline queue processing
pub async fn process_queue_parallel(
    actions: Vec<QueuedAction>,
    executor: Arc<dyn ActionExecutor>,
) -> ProcessResult {
    let futures = actions.into_iter().map(|action| {
        let executor = executor.clone();
        async move {
            executor.execute(action).await
        }
    });

    futures::future::join_all(futures).await
}
```

## Memory Considerations

- **Stack vs. Heap:** Messages and small structs on stack; large data (JSON payloads) on heap via `Box` and `String`
- **Arc for shared state:** `Arc<Mutex<T>>` for thread-safe shared state
- **Clone on send:** Messages cloned when crossing thread boundaries
- **No unsafe code:** All FFI handled by swift-bridge and jni-rs crates

## Edge Cases & Safety Guarantees

| Edge Case | Rust Handling |
|-----------|---------------|
| Message deserialization failure | `Result<T, serde_json::Error>` - explicit error |
| Component not registered | `BridgeError::ComponentNotFound` at runtime |
| Concurrent message handling | `Mutex` ensures single-threaded access per component |
| Platform FFI failure | `BridgeError::Platform` with error message |
| Offline queue corruption | Validate JSON schema before processing |
| Certificate pinning bypass | Compile-time pinned hashes, fail-closed |

## Code Examples

### Complete Bridge Initialization

```rust
// lib.rs - Platform initialization

use estrada_core::bridge::Bridge;
use estrada_core::components::{PageComponent, FormComponent, TextComponent};
use std::sync::Arc;

/// Initialize the Strada bridge
/// Called from platform (Swift/Kotlin) on app launch
pub fn init_bridge(delegate: impl BridgeDelegate + 'static) -> Arc<Bridge> {
    let mut bridge = Bridge::new(Arc::new(delegate));

    // Register built-in components
    bridge.register_component(PageComponent::new(delegate.clone()));
    bridge.register_component(FormComponent::new(delegate.clone()));
    bridge.register_component(TextComponent::new(delegate.clone()));

    Arc::new(bridge)
}

/// Handle message from web view
/// Called from platform when WebView receives message
pub async fn handle_web_message(
    bridge: Arc<Bridge>,
    json_message: &str,
) -> Result<(), BridgeError> {
    let message: Message = serde_json::from_str(json_message)?;
    bridge.handle_message(message).await
}

/// Send message to web view
pub fn send_to_web(bridge: Arc<Bridge>, message: Message) -> Result<(), BridgeError> {
    bridge.send_to_web(message)
}
```

### iOS FFI Example (swift-bridge)

```rust
// estrada-ios/src/lib.rs

use swift_bridge::swift_bridge;
use estrada_core::bridge::Bridge;
use std::sync::Arc;

#[swift_bridge::bridge]
mod ffi {
    extern "Rust" {
        type StradaBridge;
        type ArcBridge;

        #[swift_bridge(init)]
        fn new() -> ArcBridge;

        fn handle_message(&self, json: &str) -> Result<(), String>;
        fn send_message(&self, json: &str) -> Result<(), String>;
    }
}

pub struct StradaBridge {
    bridge: Arc<Bridge>,
}

impl StradaBridge {
    pub fn new() -> Arc<Self> {
        // Initialize with iOS delegate
        let delegate = IosBridgeDelegate::new();
        Arc::new(Self {
            bridge: estrada_core::init_bridge(delegate),
        })
    }

    pub fn handle_message(&self, json: &str) -> Result<(), String> {
        let bridge = self.bridge.clone();
        let json = json.to_string();

        // Run async in background
        tokio::spawn(async move {
            let msg: Message = serde_json::from_str(&json).map_err(|e| e.to_string())?;
            bridge.handle_message(msg).await.map_err(|e| e.to_string())
        });

        Ok(())
    }
}
```

### Android FFI Example (jni-rs)

```rust
// estrada-android/src/lib.rs

use jni::objects::{JClass, JString, JObject};
use jni::sys::{jboolean, jint};
use jni::JNIEnv;
use std::sync::Arc;

static mut BRIDGE_INSTANCE: Option<Arc<Bridge>> = None;

#[no_mangle]
#[jni::native_method]
fn nativeInitBridge(env: JNIEnv, _class: JClass) -> jlong {
    let delegate = AndroidBridgeDelegate::new(&env);
    let bridge = estrada_core::init_bridge(delegate);

    let ptr = Arc::into_raw(bridge) as jlong;
    unsafe {
        BRIDGE_INSTANCE = Some(Arc::from_raw(ptr as *const Bridge));
    }
    ptr
}

#[no_mangle]
#[jni::native_method]
fn nativeHandleMessage(
    env: JNIEnv,
    _class: JClass,
    bridge_ptr: jlong,
    json_message: JString,
) -> jboolean {
    let bridge = unsafe { BRIDGE_INSTANCE.as_ref().unwrap().clone() };
    let json: String = env.get_string(&json_message).unwrap().into();

    let rt = tokio::runtime::Handle::current();
    let result = rt.block_on(async {
        let msg: Message = match serde_json::from_str(&json) {
            Ok(m) => m,
            Err(_) => return false,
        };
        bridge.handle_message(msg).await.is_ok()
    });

    result as jboolean
}
```

## Migration Path

1. **Phase 1: Core Library** - Implement `strada-core` with message types and traits
2. **Phase 2: iOS Integration** - Create swift-bridge bindings, integrate with existing iOS app
3. **Phase 3: Android Integration** - Create jni-rs bindings, integrate with existing Android app
4. **Phase 4: Component Migration** - Migrate existing Swift/Kotlin components to Rust incrementally
5. **Phase 5: Platform API Expansion** - Add more platform-specific functionality (secure storage, etc.)

## Performance Considerations

- **Message serialization:** serde_json is fast but consider bincode for internal messages
- **Async overhead:** Minimal with tokio; batch small operations
- **FFI cost:** swift-bridge and jni-rs have small overhead; minimize crossing boundary
- **Memory:** Use `&str` instead of `String` where possible; avoid unnecessary clones

## Testing Strategy

- **Unit tests:** Test message serialization, component logic with `tokio-test`
- **Integration tests:** Use `MockBridgeDelegate` to test full message flows
- **FFI tests:** Platform-specific tests for Swift/Kotlin interop
- **E2E tests:** Maestro/Detox tests for full app flows

## Open Considerations

- **Hot reload:** Consider how to handle component updates without app restart
- **Plugin system:** Design for third-party bridge components
- **Version negotiation:** Handle web/native version mismatches gracefully
- **Logging:** Structured logging with platform integration (os_log, Logcat)
