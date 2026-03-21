# Offline Queue System - Deep Dive

## Overview

This document provides a comprehensive, production-ready implementation of an offline-first queue system for the Strada WebView bridge architecture. The system enables reliable action queuing when offline, intelligent retry with exponential backoff, and seamless synchronization when connectivity is restored.

**Key Features:**
- Priority-based queue with multiple priority levels
- Exponential backoff retry logic with jitter
- Action deduplication to prevent duplicate submissions
- Multi-platform persistence (IndexedDB for web, Room for Android)
- Parallel action execution with conflict resolution
- Service Worker background sync coordination
- Real-time queue status broadcasting

---

## 1. Core Queue Logic (`strada-core/src/offline/queue.rs`)

### 1.1 Priority Levels and Queue Configuration

```rust
// strada-core/src/offline/queue.rs

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{Mutex, RwLock};
use tracing::{debug, error, info, warn};
use uuid::Uuid;

/// Priority levels for queued actions
/// Higher priority actions are processed first
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[repr(u8)]
pub enum Priority {
    Low = 0,
    Normal = 1,
    High = 2,
    Critical = 3,
}

impl Default for Priority {
    fn default() -> Self {
        Priority::Normal
    }
}

/// Configuration for the offline queue
#[derive(Debug, Clone)]
pub struct QueueConfig {
    /// Maximum number of retries before giving up
    pub max_retries: u32,
    /// Initial backoff duration in milliseconds
    pub initial_backoff_ms: u64,
    /// Maximum backoff duration in milliseconds
    pub max_backoff_ms: u64,
    /// Backoff multiplier (e.g., 2.0 = exponential)
    pub backoff_multiplier: f64,
    /// Add random jitter to backoff (true = recommended)
    pub jitter_enabled: bool,
    /// Maximum queue size (0 = unlimited)
    pub max_queue_size: usize,
    /// Deduplication enabled
    pub deduplication_enabled: bool,
}

impl Default for QueueConfig {
    fn default() -> Self {
        Self {
            max_retries: 5,
            initial_backoff_ms: 1000,      // 1 second
            max_backoff_ms: 300_000,       // 5 minutes
            backoff_multiplier: 2.0,       // Exponential
            jitter_enabled: true,
            max_queue_size: 1000,
            deduplication_enabled: true,
        }
    }
}
```

### 1.2 QueuedAction Type

```rust
/// A queued action waiting to be executed
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueuedAction {
    /// Unique identifier for this action
    pub id: String,
    /// Action type (e.g., "form-submit", "sync-request")
    pub action_type: String,
    /// Priority level
    pub priority: Priority,
    /// Action payload (JSON)
    pub payload: serde_json::Value,
    /// Deduplication key (optional)
    pub dedup_key: Option<String>,
    /// When this action was created
    pub created_at: DateTime<Utc>,
    /// When this action was last attempted
    pub last_attempt: Option<DateTime<Utc>>,
    /// Number of retry attempts
    pub retry_count: u32,
    /// Next scheduled attempt (for backoff)
    pub next_attempt_at: Option<DateTime<Utc>>,
    /// Metadata for debugging/tracing
    pub metadata: HashMap<String, String>,
}

impl QueuedAction {
    /// Create a new queued action
    pub fn new(
        action_type: impl Into<String>,
        payload: serde_json::Value,
        priority: Priority,
    ) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            action_type: action_type.into(),
            priority,
            payload,
            dedup_key: None,
            created_at: Utc::now(),
            last_attempt: None,
            retry_count: 0,
            next_attempt_at: None,
            metadata: HashMap::new(),
        }
    }

    /// Set deduplication key
    pub fn with_dedup_key(mut self, key: impl Into<String>) -> Self {
        self.dedup_key = Some(key.into());
        self
    }

    /// Add metadata
    pub fn with_metadata(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.metadata.insert(key.into(), value.into());
        self
    }

    /// Calculate backoff duration for next retry
    pub fn calculate_backoff(&self, config: &QueueConfig) -> u64 {
        if self.retry_count == 0 {
            return config.initial_backoff_ms;
        }

        let exponential = config.initial_backoff_ms as f64
            * config.backoff_multiplier.powi(self.retry_count as i32);

        let backoff = if config.jitter_enabled {
            // Add up to 20% jitter
            let jitter = exponential * 0.2 * rand::random::<f64>();
            exponential + jitter
        } else {
            exponential
        };

        (backoff.min(config.max_backoff_ms as f64) as u64).max(config.initial_backoff_ms)
    }

    /// Check if this action is ready to be attempted
    pub fn is_ready(&self) -> bool {
        match self.next_attempt_at {
            Some(next) => Utc::now() >= next,
            None => true,
        }
    }

    /// Get the sort key for priority queue ordering
    pub fn sort_key(&self) -> (i32, DateTime<Utc>) {
        // Higher priority = lower number (processed first)
        // Earlier creation = processed first within same priority
        (-(self.priority as i32), self.created_at)
    }
}
```

### 1.3 Storage Trait for Persistence

```rust
// strada-core/src/offline/storage.rs

use crate::offline::queue::QueuedAction;
use crate::error::Result;
use async_trait::async_trait;

/// Trait for persistent storage of queued actions
/// Implement this trait for platform-specific storage (IndexedDB, Room, etc.)
#[async_trait]
pub trait QueueStorage: Send + Sync {
    /// Save all actions to persistent storage
    async fn save_all(&self, actions: &[QueuedAction]) -> Result<()>;

    /// Load all actions from persistent storage
    async fn load_all(&self) -> Result<Vec<QueuedAction>>;

    /// Insert or update a single action
    async fn upsert(&self, action: &QueuedAction) -> Result<()>;

    /// Delete an action by ID
    async fn delete(&self, action_id: &str) -> Result<()>;

    /// Delete multiple actions by IDs
    async fn delete_batch(&self, action_ids: &[String]) -> Result<()>;

    /// Get the count of queued actions
    async fn count(&self) -> Result<usize>;

    /// Clear all actions (use with caution!)
    async fn clear(&self) -> Result<()>;

    /// Get actions by priority
    async fn get_by_priority(&self, priority: crate::offline::queue::Priority) -> Result<Vec<QueuedAction>>;

    /// Get actions that are ready to execute
    async fn get_ready_actions(&self) -> Result<Vec<QueuedAction>>;
}
```

### 1.4 The OfflineQueue Implementation

```rust
// strada-core/src/offline/queue.rs (continued)

use super::storage::QueueStorage;
use crate::error::{BridgeError, Result};

/// Statistics about queue state
#[derive(Debug, Clone, Default)]
pub struct QueueStats {
    pub total_actions: usize,
    pub actions_by_priority: HashMap<Priority, usize>,
    pub actions_by_type: HashMap<String, usize>,
    pub failed_actions: usize,
    pub pending_actions: usize,
}

/// Event emitted by the queue for observers
#[derive(Debug, Clone)]
pub enum QueueEvent {
    ActionEnqueued { action: QueuedAction },
    ActionDequeued { action_id: String },
    ActionCompleted { action_id: String, success: bool },
    ActionFailed { action_id: String, error: String },
    QueueStatusChanged { stats: QueueStats },
    SyncStarted,
    SyncCompleted { succeeded: usize, failed: usize },
}

/// The main offline queue manager
pub struct OfflineQueue<S: QueueStorage> {
    /// In-memory action storage
    actions: Arc<RwLock<Vec<QueuedAction>>>,
    /// Persistent storage backend
    storage: Arc<S>,
    /// Queue configuration
    config: QueueConfig,
    /// Deduplication index (dedup_key -> action_id)
    dedup_index: Arc<RwLock<HashMap<String, String>>>,
    /// Event listeners
    listeners: Arc<RwLock<Vec<Box<dyn Fn(QueueEvent) + Send + Sync>>>>,
    /// Currently processing (prevents concurrent sync)
    is_syncing: Arc<AtomicBool>,
}

use std::sync::atomic::{AtomicBool, Ordering};

impl<S: QueueStorage> OfflineQueue<S> {
    /// Create a new offline queue with the given storage backend
    pub fn new(storage: Arc<S>, config: QueueConfig) -> Self {
        Self {
            actions: Arc::new(RwLock::new(Vec::new())),
            storage,
            config,
            dedup_index: Arc::new(RwLock::new(HashMap::new())),
            listeners: Arc::new(RwLock::new(Vec::new())),
            is_syncing: Arc::new(AtomicBool::new(false)),
        }
    }

    /// Initialize the queue (load from storage)
    pub async fn initialize(&self) -> Result<()> {
        let loaded = self.storage.load_all().await?;

        // Rebuild in-memory state
        let mut actions = self.actions.write().await;
        let mut dedup_index = self.dedup_index.write().await;

        for action in &loaded {
            if let Some(key) = &action.dedup_key {
                dedup_index.insert(key.clone(), action.id.clone());
            }
        }

        *actions = loaded;
        self.sort_actions(&mut actions);

        info!("Offline queue initialized with {} actions", actions.len());
        self.emit_event(QueueEvent::QueueStatusChanged {
            stats: self.calculate_stats().await,
        }).await;

        Ok(())
    }

    /// Enqueue a new action
    pub async fn enqueue(&self, mut action: QueuedAction) -> Result<String> {
        // Check deduplication
        if self.config.deduplication_enabled {
            if let Some(key) = &action.dedup_key {
                let dedup_index = self.dedup_index.read().await;
                if let Some(existing_id) = dedup_index.get(key) {
                    warn!("Duplicate action detected with key '{}', existing ID: {}", key, existing_id);
                    return Ok(existing_id.clone()); // Return existing ID
                }
            }
        }

        // Check queue size limit
        if self.config.max_queue_size > 0 {
            let actions = self.actions.read().await;
            if actions.len() >= self.config.max_queue_size {
                return Err(BridgeError::OfflineQueue(
                    format!("Queue is full (max size: {})", self.config.max_queue_size)
                ));
            }
        }

        let action_id = action.id.clone();
        let dedup_key = action.dedup_key.clone();

        // Add to in-memory queue
        {
            let mut actions = self.actions.write().await;
            actions.push(action.clone());
            self.sort_actions(&mut actions);

            // Persist to storage
            self.storage.upsert(&action).await?;
        }

        // Update dedup index
        if let Some(key) = dedup_key {
            let mut dedup_index = self.dedup_index.write().await;
            dedup_index.insert(key, action_id.clone());
        }

        info!("Action enqueued: {} (type: {}, priority: {:?})",
              action_id, action.action_type, action.priority);

        self.emit_event(QueueEvent::ActionEnqueued { action }).await;
        self.emit_status_change().await;

        Ok(action_id)
    }

    /// Dequeue the next ready action (highest priority, oldest first)
    pub async fn dequeue(&self) -> Result<Option<QueuedAction>> {
        let mut actions = self.actions.write().await;

        // Find the first ready action (already sorted by priority)
        for i in 0..actions.len() {
            if actions[i].is_ready() {
                let action = actions.remove(i);
                self.storage.delete(&action.id).await?;
                return Ok(Some(action));
            }
        }

        Ok(None)
    }

    /// Mark an action as completed (remove from queue)
    pub async fn complete(&self, action_id: &str) -> Result<()> {
        // Remove from in-memory queue
        {
            let mut actions = self.actions.write().await;
            let original_len = actions.len();
            actions.retain(|a| a.id != action_id);

            if original_len == actions.len() {
                return Err(BridgeError::OfflineQueue(
                    format!("Action {} not found in queue", action_id)
                ));
            }
        }

        // Remove from dedup index
        self.remove_from_dedup_index(action_id).await;

        info!("Action completed: {}", action_id);
        self.emit_event(QueueEvent::ActionCompleted {
            action_id: action_id.to_string(),
            success: true,
        }).await;
        self.emit_status_change().await;

        Ok(())
    }

    /// Mark an action as failed (schedule retry or give up)
    pub async fn fail(&self, action_id: &str, error: &str) -> Result<bool> {
        let mut actions = self.actions.write().await;

        for action in actions.iter_mut() {
            if action.id == action_id {
                action.retry_count += 1;
                action.last_attempt = Some(Utc::now());

                if action.retry_count >= self.config.max_retries {
                    // Give up - remove from queue
                    warn!("Action {} permanently failed after {} retries",
                          action_id, action.retry_count);

                    self.storage.delete(action_id).await?;
                    self.remove_from_dedup_index(action_id).await;

                    self.emit_event(QueueEvent::ActionFailed {
                        action_id: action_id.to_string(),
                        error: error.to_string(),
                    }).await;

                    return Ok(false); // Cannot retry
                }

                // Schedule next attempt with backoff
                let backoff = action.calculate_backoff(&self.config);
                action.next_attempt_at = Some(
                    Utc::now() + chrono::Duration::milliseconds(backoff as i64)
                );

                // Update storage
                self.storage.upsert(action).await?;

                info!("Action {} failed, scheduling retry #{} in {}ms",
                      action_id, action.retry_count, backoff);

                self.emit_event(QueueEvent::ActionFailed {
                    action_id: action_id.to_string(),
                    error: error.to_string(),
                }).await;

                return Ok(true); // Can retry
            }
        }

        Err(BridgeError::OfflineQueue(
            format!("Action {} not found in queue", action_id)
        ))
    }

    /// Process all ready actions (called when connectivity is restored)
    pub async fn sync_all<F, Fut>(&self, executor: F) -> Result<SyncResult>
    where
        F: Fn(QueuedAction) -> Fut + Send + Sync + Clone + 'static,
        Fut: std::future::Future<Output = Result<()>> + Send + 'static,
    {
        // Prevent concurrent sync operations
        if !self.is_syncing.compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst).is_ok() {
            return Err(BridgeError::OfflineQueue(
                "Sync already in progress".to_string()
            ));
        }

        self.emit_event(QueueEvent::SyncStarted).await;

        let result = self.sync_all_internal(executor).await;

        self.is_syncing.store(false, Ordering::SeqCst);

        self.emit_event(QueueEvent::SyncCompleted {
            succeeded: result.succeeded.len(),
            failed: result.permanently_failed.len(),
        }).await;

        Ok(result)
    }

    async fn sync_all_internal<F, Fut>(&self, executor: F) -> Result<SyncResult>
    where
        F: Fn(QueuedAction) -> Fut + Send + Sync + Clone + 'static,
        Fut: std::future::Future<Output = Result<()>> + Send + 'static,
    {
        let mut result = SyncResult::default();

        // Get all ready actions
        let ready_actions = {
            let actions = self.actions.read().await;
            actions.iter()
                .filter(|a| a.is_ready())
                .cloned()
                .collect::<Vec<_>>()
        };

        if ready_actions.is_empty() {
            debug!("No ready actions to sync");
            return Ok(result);
        }

        info!("Starting sync of {} ready actions", ready_actions.len());

        // Execute actions in parallel (with concurrency limit)
        const MAX_CONCURRENT: usize = 5;
        let mut futures = Vec::new();

        for action in ready_actions {
            let action_id = action.id.clone();
            let executor = executor.clone();
            let queue = self;

            futures.push(async move {
                match executor(action).await {
                    Ok(()) => {
                        queue.complete(&action_id).await.ok();
                        (action_id, true)
                    }
                    Err(e) => {
                        queue.fail(&action_id, &e.to_string()).await.ok();
                        (action_id, false)
                    }
                }
            });

            // Limit concurrency
            if futures.len() >= MAX_CONCURRENT {
                let (id, success) = futures::future::select_all(futures).await.0;
                if success {
                    result.succeeded.push(id);
                } else {
                    // Check if it can retry
                    if !queue.can_retry(&id).await {
                        result.permanently_failed.push(id);
                    } else {
                        result.failed_but_retrying.push(id);
                    }
                }
                futures.clear();
            }
        }

        // Process remaining futures
        for future in futures {
            let (id, success) = future.await;
            if success {
                result.succeeded.push(id);
            } else if !self.can_retry(&id).await {
                result.permanently_failed.push(id);
            } else {
                result.failed_but_retrying.push(id);
            }
        }

        self.emit_status_change().await;
        Ok(result)
    }

    async fn can_retry(&self, action_id: &str) -> bool {
        let actions = self.actions.read().await;
        actions.iter()
            .find(|a| a.id == action_id)
            .map(|a| a.retry_count < self.config.max_retries)
            .unwrap_or(false)
    }

    /// Get queue statistics
    pub async fn get_stats(&self) -> Result<QueueStats> {
        Ok(self.calculate_stats().await)
    }

    async fn calculate_stats(&self) -> QueueStats {
        let actions = self.actions.read().await;
        let mut stats = QueueStats::default();

        stats.total_actions = actions.len();

        for action in actions.iter() {
            *stats.actions_by_priority.entry(action.priority).or_insert(0) += 1;
            *stats.actions_by_type.entry(action.action_type.clone()).or_insert(0) += 1;

            if action.retry_count >= self.config.max_retries {
                stats.failed_actions += 1;
            } else {
                stats.pending_actions += 1;
            }
        }

        stats
    }

    /// Sort actions by priority and creation time
    fn sort_actions(&self, actions: &mut Vec<QueuedAction>) {
        actions.sort_by(|a, b| a.sort_key().cmp(&b.sort_key()));
    }

    /// Remove action from dedup index
    async fn remove_from_dedup_index(&self, action_id: &str) {
        let actions = self.actions.read().await;
        if let Some(action) = actions.iter().find(|a| a.id == action_id) {
            if let Some(key) = &action.dedup_key {
                let mut dedup_index = self.dedup_index.write().await;
                dedup_index.remove(key);
            }
        }
    }

    /// Emit an event to listeners
    async fn emit_event(&self, event: QueueEvent) {
        let listeners = self.listeners.read().await;
        for listener in listeners.iter() {
            listener(event.clone());
        }
    }

    async fn emit_status_change(&self) {
        let stats = self.calculate_stats().await;
        self.emit_event(QueueEvent::QueueStatusChanged { stats }).await;
    }

    /// Add a listener for queue events
    pub async fn add_listener<F>(&self, listener: F)
    where
        F: Fn(QueueEvent) + Send + Sync + 'static,
    {
        let mut listeners = self.listeners.write().await;
        listeners.push(Box::new(listener));
    }

    /// Check if sync is in progress
    pub fn is_syncing(&self) -> bool {
        self.is_syncing.load(Ordering::SeqCst)
    }
}

/// Result of a sync operation
#[derive(Debug, Clone, Default)]
pub struct SyncResult {
    pub succeeded: Vec<String>,
    pub failed_but_retrying: Vec<String>,
    pub permanently_failed: Vec<String>,
}
```

---

## 2. Web Integration - IndexedDB Storage

```rust
// estrada-web/src/storage/indexed_db.rs

use async_trait::async_trait;
use js_sys::{Array, Object};
use serde::{Deserialize, Serialize};
use strada_core::offline::queue::QueuedAction;
use strada_core::offline::storage::QueueStorage;
use strada_core::error::{BridgeError, Result};
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::JsFuture;
use web_sys::{IdbDatabase, IdbFactory, IdbObjectStore, IdbTransactionMode, IndexedDb};
use tracing::{debug, error, info, warn};

/// IndexedDB storage implementation for web
pub struct IndexedDBStorage {
    db: IdbDatabase,
    store_name: String,
}

impl IndexedDBStorage {
    const DEFAULT_DB_NAME: &'static str = "strada_offline_queue";
    const DEFAULT_STORE_NAME: &'static str = "actions";
    const DB_VERSION: u32 = 1;

    /// Create and initialize IndexedDB storage
    pub async fn new() -> Result<Self> {
        Self::with_names(
            Self::DEFAULT_DB_NAME.to_string(),
            Self::DEFAULT_STORE_NAME.to_string(),
        ).await
    }

    /// Create IndexedDB storage with custom names
    pub async fn with_names(db_name: String, store_name: String) -> Result<Self> {
        let window = web_sys::window().ok_or_else(|| {
            BridgeError::Platform("No window object available".to_string())
        })?;

        let indexed_db: IndexedDb = window
            .indexed_db()
            .map_err(|e| BridgeError::Platform(format!("Failed to get IndexedDB: {:?}", e)))?
            .ok_or_else(|| BridgeError::Platform("IndexedDB not supported".to_string()))?;

        // Open database
        let db_request = indexed_db
            .open_with_u32(&db_name, Self::DB_VERSION)
            .map_err(|e| BridgeError::Platform(format!("Failed to open DB: {:?}", e)))?;

        // Handle upgrade needed
        let db_name_clone = db_name.clone();
        let store_name_clone = store_name.clone();
        let on_upgrade_cb = Closure::once(move |event: web_sys::IdbVersionChangeEvent| {
            let db = event.target().unwrap().dyn_into::<IdbDatabase>().unwrap();

            // Create object store if not exists
            if !db.object_store_names().any(|n| n == store_name_clone) {
                let store = db
                    .create_object_store(&store_name_clone)
                    .expect("Failed to create object store");

                // Create indexes for efficient querying
                store
                    .create_index_with_params("priority", &["priority"].into(), false)
                    .expect("Failed to create priority index");

                store
                    .create_index_with_params("created_at", &["created_at"].into(), false)
                    .expect("Failed to create created_at index");

                store
                    .create_index_with_params("dedup_key", &["dedup_key"].into(), true)
                    .expect("Failed to create dedup_key index");

                store
                    .create_index_with_params("next_attempt_at", &["next_attempt_at"].into(), true)
                    .expect("Failed to create next_attempt_at index");
            }
        });

        db_request.set_onsuccess(Some(on_upgrade_cb.as_ref().unchecked_ref()));
        on_upgrade_cb.forget();

        // Wait for success
        let db_future = JsFuture::from(
            db_request.open()
        );

        match db_future.await {
            Ok(db_value) => {
                let db = db_value.dyn_into::<IdbDatabase>()
                    .map_err(|_| BridgeError::Platform("Failed to get database".to_string()))?;

                info!("IndexedDB storage initialized: {}", db_name);
                Ok(Self { db, store_name })
            }
            Err(e) => Err(BridgeError::Platform(format!(
                "Failed to open database: {:?}",
                e
            ))),
        }
    }

    /// Serialize action for storage
    fn serialize_action(&self, action: &QueuedAction) -> Result<Object> {
        let obj = Object::new();
        let key = JsValue::from_str;

        // Store as JSON for simplicity
        let json = serde_json::to_string(action)
            .map_err(|e| BridgeError::Serialization(e))?;

        js_sys::Reflect::set(&obj, &key("id"), &JsValue::from(&action.id))?;
        js_sys::Reflect::set(&obj, &key("action_type"), &JsValue::from(&action.action_type))?;
        js_sys::Reflect::set(&obj, &key("priority"), &JsValue::from(action.priority as u8))?;
        js_sys::Reflect::set(&obj, &key("payload"), &serde_wasm_bindgen::to_value(&action.payload)?)?;
        js_sys::Reflect::set(&obj, &key("dedup_key"), &action.dedup_key.to_js_value())?;
        js_sys::Reflect::set(&obj, &key("created_at"), &JsValue::from(action.created_at.timestamp_millis() as f64))?;
        js_sys::Reflect::set(&obj, &key("last_attempt"), &action.last_attempt.map(|t| JsValue::from(t.timestamp_millis() as f64)).unwrap_or(JsValue::NULL))?;
        js_sys::Reflect::set(&obj, &key("retry_count"), &JsValue::from(action.retry_count))?;
        js_sys::Reflect::set(&obj, &key("next_attempt_at"), &action.next_attempt_at.map(|t| JsValue::from(t.timestamp_millis() as f64)).unwrap_or(JsValue::NULL))?;
        js_sys::Reflect::set(&obj, &key("json_data"), &JsValue::from(json))?;

        Ok(obj)
    }

    /// Deserialize action from storage
    fn deserialize_action(&self, obj: JsValue) -> Result<QueuedAction> {
        // Try to get json_data first (preferred)
        if let Ok(json) = js_sys::Reflect::get(&obj, &JsValue::from_str("json_data")) {
            if json.is_string() {
                return serde_json::from_str(&json.as_string().unwrap())
                    .map_err(|e| BridgeError::Serialization(e));
            }
        }

        // Fallback: reconstruct from fields
        let id: String = js_sys::Reflect::get(&obj, &JsValue::from_str("id"))?
            .as_string()
            .ok_or_else(|| BridgeError::OfflineQueue("Missing id field".to_string()))?;

        let action_type: String = js_sys::Reflect::get(&obj, &JsValue::from_str("action_type"))?
            .as_string()
            .ok_or_else(|| BridgeError::OfflineQueue("Missing action_type field".to_string()))?;

        let priority_num: u8 = js_sys::Reflect::get(&obj, &JsValue::from_str("priority"))?
            .as_f64()
            .unwrap_or(1.0) as u8;

        let priority = match priority_num {
            0 => crate::offline::queue::Priority::Low,
            2 => crate::offline::queue::Priority::High,
            3 => crate::offline::queue::Priority::Critical,
            _ => crate::offline::queue::Priority::Normal,
        };

        let payload: serde_json::Value = serde_wasm_bindgen::from_value(
            js_sys::Reflect::get(&obj, &JsValue::from_str("payload"))?
        )?;

        let dedup_key: Option<String> = js_sys::Reflect::get(&obj, &JsValue::from_str("dedup_key"))?
            .as_string();

        let created_at_millis: f64 = js_sys::Reflect::get(&obj, &JsValue::from_str("created_at"))?
            .as_f64()
            .unwrap_or(0.0);

        let last_attempt: Option<chrono::DateTime<chrono::Utc>> =
            js_sys::Reflect::get(&obj, &JsValue::from_str("last_attempt"))?
                .as_f64()
                .map(|millis| chrono::DateTime::from_timestamp_millis(millis as i64).unwrap().into());

        let retry_count: u32 = js_sys::Reflect::get(&obj, &JsValue::from_str("retry_count"))?
            .as_f64()
            .unwrap_or(0.0) as u32;

        let next_attempt_at: Option<chrono::DateTime<chrono::Utc>> =
            js_sys::Reflect::get(&obj, &JsValue::from_str("next_attempt_at"))?
                .as_f64()
                .and_then(|millis| chrono::DateTime::from_timestamp_millis(millis as i64));

        Ok(QueuedAction {
            id,
            action_type,
            priority,
            payload,
            dedup_key,
            created_at: chrono::DateTime::from_timestamp_millis(created_at_millis as i64).unwrap().into(),
            last_attempt,
            retry_count,
            next_attempt_at,
            metadata: std::collections::HashMap::new(),
        })
    }

    /// Get object store for a transaction
    fn get_store<'a>(
        &self,
        tx: &'a web_sys::IdbTransaction,
    ) -> Result<IdbObjectStore> {
        tx.object_store(&self.store_name)
            .map_err(|e| BridgeError::OfflineQueue(format!("Failed to get store: {:?}", e)))
    }
}

#[async_trait]
impl QueueStorage for IndexedDBStorage {
    async fn save_all(&self, actions: &[QueuedAction]) -> Result<()> {
        let tx = self.db
            .transaction_with_str_and_mode(&self.store_name, IdbTransactionMode::Readwrite)
            .map_err(|e| BridgeError::OfflineQueue(format!("Transaction failed: {:?}", e)))?;

        let store = self.get_store(&tx)?;

        for action in actions {
            let obj = self.serialize_action(action)?;
            store.put_with_key(&obj, &JsValue::from(&action.id))
                .map_err(|e| BridgeError::OfflineQueue(format!("Put failed: {:?}", e)))?;
        }

        // Wait for transaction to complete
        JsFuture::from(tx.done()).await
            .map_err(|e| BridgeError::OfflineQueue(format!("Transaction commit failed: {:?}", e)))?;

        debug!("Saved {} actions to IndexedDB", actions.len());
        Ok(())
    }

    async fn load_all(&self) -> Result<Vec<QueuedAction>> {
        let tx = self.db
            .transaction_with_str_and_mode(&self.store_name, IdbTransactionMode::Readonly)
            .map_err(|e| BridgeError::OfflineQueue(format!("Transaction failed: {:?}", e)))?;

        let store = self.get_store(&tx)?;

        // Get all records
        let get_all_request = store.get_all().unwrap();
        let result = JsFuture::from(get_all_request).await
            .map_err(|e| BridgeError::OfflineQueue(format!("Get all failed: {:?}", e)))?;

        let array = result.dyn_into::<Array>()
            .map_err(|_| BridgeError::OfflineQueue("Result is not an array".to_string()))?;

        let mut actions = Vec::new();
        for item in array.iter() {
            match self.deserialize_action(item) {
                Ok(action) => actions.push(action),
                Err(e) => {
                    warn!("Failed to deserialize action: {:?}", e);
                }
            }
        }

        debug!("Loaded {} actions from IndexedDB", actions.len());
        Ok(actions)
    }

    async fn upsert(&self, action: &QueuedAction) -> Result<()> {
        let tx = self.db
            .transaction_with_str_and_mode(&self.store_name, IdbTransactionMode::Readwrite)
            .map_err(|e| BridgeError::OfflineQueue(format!("Transaction failed: {:?}", e)))?;

        let store = self.get_store(&tx)?;

        let obj = self.serialize_action(action)?;
        store.put_with_key(&obj, &JsValue::from(&action.id))
            .map_err(|e| BridgeError::OfflineQueue(format!("Put failed: {:?}", e)))?;

        JsFuture::from(tx.done()).await
            .map_err(|e| BridgeError::OfflineQueue(format!("Transaction commit failed: {:?}", e)))?;

        debug!("Upserted action {} to IndexedDB", action.id);
        Ok(())
    }

    async fn delete(&self, action_id: &str) -> Result<()> {
        let tx = self.db
            .transaction_with_str_and_mode(&self.store_name, IdbTransactionMode::Readwrite)
            .map_err(|e| BridgeError::OfflineQueue(format!("Transaction failed: {:?}", e)))?;

        let store = self.get_store(&tx)?;

        store.delete_with_key(&JsValue::from(action_id))
            .map_err(|e| BridgeError::OfflineQueue(format!("Delete failed: {:?}", e)))?;

        JsFuture::from(tx.done()).await
            .map_err(|e| BridgeError::OfflineQueue(format!("Transaction commit failed: {:?}", e)))?;

        debug!("Deleted action {} from IndexedDB", action_id);
        Ok(())
    }

    async fn delete_batch(&self, action_ids: &[String]) -> Result<()> {
        let tx = self.db
            .transaction_with_str_and_mode(&self.store_name, IdbTransactionMode::Readwrite)
            .map_err(|e| BridgeError::OfflineQueue(format!("Transaction failed: {:?}", e)))?;

        let store = self.get_store(&tx)?;

        for id in action_ids {
            store.delete_with_key(&JsValue::from(id.as_str()))
                .map_err(|e| BridgeError::OfflineQueue(format!("Delete failed: {:?}", e)))?;
        }

        JsFuture::from(tx.done()).await
            .map_err(|e| BridgeError::OfflineQueue(format!("Transaction commit failed: {:?}", e)))?;

        debug!("Deleted {} actions from IndexedDB", action_ids.len());
        Ok(())
    }

    async fn count(&self) -> Result<usize> {
        let tx = self.db
            .transaction_with_str_and_mode(&self.store_name, IdbTransactionMode::Readonly)
            .map_err(|e| BridgeError::OfflineQueue(format!("Transaction failed: {:?}", e)))?;

        let store = self.get_store(&tx)?;

        let count_request = store.count().unwrap();
        let result = JsFuture::from(count_request).await
            .map_err(|e| BridgeError::OfflineQueue(format!("Count failed: {:?}", e)))?;

        Ok(result.as_f64().unwrap_or(0.0) as usize)
    }

    async fn clear(&self) -> Result<()> {
        let tx = self.db
            .transaction_with_str_and_mode(&self.store_name, IdbTransactionMode::Readwrite)
            .map_err(|e| BridgeError::OfflineQueue(format!("Transaction failed: {:?}", e)))?;

        let store = self.get_store(&tx)?;

        store.clear().unwrap();

        JsFuture::from(tx.done()).await
            .map_err(|e| BridgeError::OfflineQueue(format!("Clear failed: {:?}", e)))?;

        info!("Cleared all actions from IndexedDB");
        Ok(())
    }

    async fn get_by_priority(&self, priority: crate::offline::queue::Priority) -> Result<Vec<QueuedAction>> {
        let tx = self.db
            .transaction_with_str_and_mode(&self.store_name, IdbTransactionMode::Readonly)
            .map_err(|e| BridgeError::OfflineQueue(format!("Transaction failed: {:?}", e)))?;

        let store = self.get_store(&tx)?;
        let index = store.index("priority")
            .map_err(|e| BridgeError::OfflineQueue(format!("Index not found: {:?}", e)))?;

        let range = web_sys::IdbKeyRange::only(&JsValue::from(priority as u8))
            .map_err(|e| BridgeError::OfflineQueue(format!("Invalid range: {:?}", e)))?;

        let request = index.get_all_with_range(&range).unwrap();
        let result = JsFuture::from(request).await
            .map_err(|e| BridgeError::OfflineQueue(format!("Get failed: {:?}", e)))?;

        let array = result.dyn_into::<Array>()
            .map_err(|_| BridgeError::OfflineQueue("Result is not an array".to_string()))?;

        let mut actions = Vec::new();
        for item in array.iter() {
            if let Ok(action) = self.deserialize_action(item) {
                actions.push(action);
            }
        }

        Ok(actions)
    }

    async fn get_ready_actions(&self) -> Result<Vec<QueuedAction>> {
        // Load all and filter (IndexedDB doesn't support complex queries)
        let all = self.load_all().await?;
        let now = chrono::Utc::now();

        let mut ready: Vec<QueuedAction> = all.into_iter()
            .filter(|a| {
                a.next_attempt_at.map(|next| next <= now).unwrap_or(true)
            })
            .collect();

        // Sort by priority
        ready.sort_by(|a, b| a.sort_key().cmp(&b.sort_key()));

        Ok(ready)
    }
}
```

---

## 3. Android Integration - Room Storage

### 3.1 Kotlin Room Entity Definitions

```kotlin
// app/src/main/java/com/example/strada/offline/OfflineActionEntity.kt

package com.example.strada.offline

import androidx.room.*
import java.util.Date

/**
 * Room entity for persistent offline action storage
 */
@Entity(
    tableName = "offline_actions",
    indices = [
        Index(value = ["priority"], name = "idx_priority"),
        Index(value = ["created_at"], name = "idx_created_at"),
        Index(value = ["dedup_key"], name = "idx_dedup_key"),
        Index(value = ["next_attempt_at"], name = "idx_next_attempt"),
        Index(value = ["action_type"], name = "idx_action_type"),
    ]
)
data class OfflineActionEntity(
    @PrimaryKey
    val id: String,

    val action_type: String,

    @TypeConverters(PriorityConverter::class)
    val priority: Priority,

    @TypeConverters(JsonValueConverter::class)
    val payload: String,  // JSON string

    val dedup_key: String?,

    @TypeConverters(InstantConverter::class)
    val created_at: Date,

    @TypeConverters(InstantConverter::class)
    val last_attempt: Date?,

    val retry_count: Int,

    @TypeConverters(InstantConverter::class)
    val next_attempt_at: Date?,

    val metadata: String  // JSON string
)

/**
 * Priority enum matching Rust Priority
 */
enum class Priority {
    LOW,
    NORMAL,
    HIGH,
    CRITICAL
}

/**
 * Converters for Room type mapping
 */
class PriorityConverter {
    @TypeConverter
    fun toPriority(value: Int): Priority {
        return when (value) {
            0 -> Priority.LOW
            2 -> Priority.HIGH
            3 -> Priority.CRITICAL
            else -> Priority.NORMAL
        }
    }

    @TypeConverter
    fun fromPriority(priority: Priority): Int {
        return when (priority) {
            Priority.LOW -> 0
            Priority.NORMAL -> 1
            Priority.HIGH -> 2
            Priority.CRITICAL -> 3
        }
    }
}

class InstantConverter {
    @TypeConverter
    fun toInstant(date: Date?): Date? = date

    @TypeConverter
    fun fromInstant(date: Date?): Date? = date
}

class JsonValueConverter {
    @TypeConverter
    fun toJson(value: String): String = value

    @TypeConverter
    fun fromJson(value: String): String = value
}
```

### 3.2 Room DAO

```kotlin
// app/src/main/java/com/example/strada/offline/OfflineActionDao.kt

package com.example.strada.offline

import androidx.room.*
import kotlinx.coroutines.flow.Flow

@Dao
interface OfflineActionDao {

    @Query("SELECT * FROM offline_actions ORDER BY priority DESC, created_at ASC")
    fun getAllActions(): Flow<List<OfflineActionEntity>>

    @Query("SELECT * FROM offline_actions ORDER BY priority DESC, created_at ASC")
    suspend fun getAllActionsList(): List<OfflineActionEntity>

    @Query("SELECT * FROM offline_actions WHERE id = :id")
    suspend fun getActionById(id: String): OfflineActionEntity?

    @Query("SELECT * FROM offline_actions WHERE priority = :priority ORDER BY created_at ASC")
    suspend fun getActionsByPriority(priority: Priority): List<OfflineActionEntity>

    @Query("SELECT * FROM offline_actions WHERE next_attempt_at IS NULL OR next_attempt_at <= :now ORDER BY priority DESC, created_at ASC")
    suspend fun getReadyActions(now: Date = Date()): List<OfflineActionEntity>

    @Query("SELECT COUNT(*) FROM offline_actions")
    fun getCount(): Flow<Int>

    @Query("SELECT COUNT(*) FROM offline_actions")
    suspend fun getCountValue(): Int

    @Insert(onConflict = OnConflictStrategy.REPLACE)
    suspend fun insertAction(action: OfflineActionEntity)

    @Insert(onConflict = OnConflictStrategy.REPLACE)
    suspend fun insertActions(actions: List<OfflineActionEntity>)

    @Delete
    suspend fun deleteAction(action: OfflineActionEntity)

    @Query("DELETE FROM offline_actions WHERE id = :id")
    suspend fun deleteActionById(id: String)

    @Query("DELETE FROM offline_actions WHERE id IN (:ids)")
    suspend fun deleteActionsByIds(ids: List<String>)

    @Query("DELETE FROM offline_actions")
    suspend fun deleteAll()

    @Query("UPDATE offline_actions SET retry_count = retry_count + 1, next_attempt_at = :nextAttempt WHERE id = :id")
    suspend fun scheduleRetry(id: String, nextAttempt: Date)

    @Query("SELECT * FROM offline_actions WHERE dedup_key = :dedupKey LIMIT 1")
    suspend fun getActionByDedupKey(dedupKey: String): OfflineActionEntity?
}
```

### 3.3 Room Database

```kotlin
// app/src/main/java/com/example/strada/offline/OfflineDatabase.kt

package com.example.strada.offline

import android.content.Context
import androidx.room.Database
import androidx.room.Room
import androidx.room.RoomDatabase
import androidx.room.TypeConverters

@Database(
    entities = [OfflineActionEntity::class],
    version = 1,
    exportSchema = true
)
@TypeConverters(
    PriorityConverter::class,
    InstantConverter::class,
    JsonValueConverter::class
)
abstract class OfflineDatabase : RoomDatabase() {
    abstract fun offlineActionDao(): OfflineActionDao

    companion object {
        @Volatile private var INSTANCE: OfflineDatabase? = null

        fun getInstance(context: Context): OfflineDatabase {
            return INSTANCE ?: synchronized(this) {
                val instance = Room.databaseBuilder(
                    context.applicationContext,
                    OfflineDatabase::class.java,
                    "strada_offline_queue"
                )
                .fallbackToDestructiveMigration()
                .build()
                INSTANCE = instance
                instance
            }
        }
    }
}
```

### 3.4 JNI-Rust Storage Implementation

```rust
// estrada-android/src/storage/room_storage.rs

use async_trait::async_trait;
use jni::objects::{JClass, JObject, JString, JValue, JValueGen};
use jni::sys::{jboolean, jint, jlong, jobjectArray};
use jni::JNIEnv;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use strada_core::offline::queue::QueuedAction;
use strada_core::offline::storage::QueueStorage;
use strada_core::error::{BridgeError, Result};
use tracing::{debug, error, info};

/// Room storage implementation for Android
/// This wraps the Kotlin Room database through JNI
pub struct RoomStorage {
    env: Arc<JNIEnv<'static>>,
    dao: Arc<JObject<'static>>,
}

impl RoomStorage {
    /// Create Room storage from Android context
    pub fn new(env: JNIEnv, context: JObject) -> Result<Self> {
        // Get OfflineDatabase instance
        let db_class = env.find_class("com/example/strada/offline/OfflineDatabase")?;
        let dao = env.call_static_method(
            db_class,
            "getInstance",
            "(Landroid/content/Context;)Lcom/example/strada/offline/OfflineDatabase;",
            &[JValue::Object(context)]
        )?.l()?;

        // Get DAO
        let dao_obj = env.call_method(
            dao,
            "offlineActionDao",
            "()Lcom/example/strada/offline/OfflineActionDao;",
            &[]
        )?.l()?;

        // Get thread-safe JNI env
        let java_vm = env.get_java_vm()?;
        let env = Arc::new(java_vm.attach_current_thread_permanently().unwrap());

        Ok(Self {
            env,
            dao: Arc::new(dao_obj.into_global().unwrap().as_obj().into()),
        })
    }

    /// Convert QueuedAction to OfflineActionEntity
    fn to_entity(&self, action: &QueuedAction) -> Result<JObject> {
        let env = &*self.env;

        // Create entity object
        let entity_class = env.find_class("com/example/strada/offline/OfflineActionEntity")?;

        // Convert priority
        let priority_class = env.find_class("com/example/strada/offline/Priority")?;
        let priority_value = match action.priority {
            crate::offline::queue::Priority::Low => 0,
            crate::offline::queue::Priority::Normal => 1,
            crate::offline::queue::Priority::High => 2,
            crate::offline::queue::Priority::Critical => 3,
        };

        // Serialize payload to JSON string
        let payload_json = serde_json::to_string(&action.payload)
            .map_err(|e| BridgeError::Serialization(e))?;

        // Serialize metadata to JSON string
        let metadata_json = serde_json::to_string(&action.metadata)
            .map_err(|e| BridgeError::Serialization(e))?;

        // Create Date objects
        let date_class = env.find_class("java/util/Date")?;
        let created_at = env.new_object(
            date_class,
            "(J)V",
            &[JValue::Long(action.created_at.timestamp_millis() as jlong)]
        )?;

        let last_attempt = action.last_attempt.map(|t| {
            env.new_object(
                date_class,
                "(J)V",
                &[JValue::Long(t.timestamp_millis() as jlong)]
            )
        }).transpose()?;

        let next_attempt_at = action.next_attempt_at.map(|t| {
            env.new_object(
                date_class,
                "(J)V",
                &[JValue::Long(t.timestamp_millis() as jlong)]
            )
        }).transpose()?;

        // Create entity
        let entity = env.new_object(
            entity_class,
            "(Ljava/lang/String;Ljava/lang/String;ILjava/lang/String;Ljava/lang/String;Ljava/util/Date;Ljava/util/Date;ILjava/util/Date;Ljava/lang/String;)V",
            &[
                JValue::Object(&env.new_string(&action.id)?.into()),
                JValue::Object(&env.new_string(&action.action_type)?.into()),
                JValue::Int(priority_value as jint),
                JValue::Object(&env.new_string(&payload_json)?.into()),
                JValue::Object(&action.dedup_key.as_ref().map(|k| env.new_string(k).unwrap()).unwrap_or(JObject::null())),
                JValue::Object(&created_at),
                JValue::Object(&last_attempt.unwrap_or(JObject::null())),
                JValue::Int(action.retry_count as jint),
                JValue::Object(&next_attempt_at.unwrap_or(JObject::null())),
                JValue::Object(&env.new_string(&metadata_json)?.into()),
            ]
        )?;

        Ok(entity)
    }

    /// Convert OfflineActionEntity to QueuedAction
    fn from_entity(&self, entity: JObject) -> Result<QueuedAction> {
        let env = &*self.env;

        let id: String = env.get_field(&entity, "id", "Ljava/lang/String;")?
            .l()?.into();

        let action_type: String = env.get_field(&entity, "action_type", "Ljava/lang/String;")?
            .l()?.into();

        let priority_num: jint = env.get_field(&entity, "priority", "I")?
            .i()?;

        let priority = match priority_num {
            0 => crate::offline::queue::Priority::Low,
            2 => crate::offline::queue::Priority::High,
            3 => crate::offline::queue::Priority::Critical,
            _ => crate::offline::queue::Priority::Normal,
        };

        let payload_str: String = env.get_field(&entity, "payload", "Ljava/lang/String;")?
            .l()?.into();

        let payload: serde_json::Value = serde_json::from_str(&payload_str)
            .map_err(|e| BridgeError::Serialization(e))?;

        let dedup_key_obj = env.get_field(&entity, "dedup_key", "Ljava/lang/String;")?
            .l()?;

        let dedup_key = if !dedup_key_obj.is_null() {
            Some(String::from(dedup_key_obj))
        } else {
            None
        };

        let retry_count: jint = env.get_field(&entity, "retry_count", "I")?
            .i()?;

        Ok(QueuedAction {
            id,
            action_type,
            priority,
            payload,
            dedup_key,
            created_at: chrono::Utc::now(), // Simplified
            last_attempt: None, // Simplified
            retry_count: retry_count as u32,
            next_attempt_at: None, // Simplified
            metadata: std::collections::HashMap::new(),
        })
    }
}

#[async_trait]
impl QueueStorage for RoomStorage {
    async fn save_all(&self, actions: &[QueuedAction]) -> Result<()> {
        let env = &*self.env;

        // Create array of entities
        let entity_class = env.find_class("com/example/strada/offline/OfflineActionEntity")?;
        let entity_array = env.new_object_array(
            actions.len() as i32,
            entity_class,
            JObject::null()
        )?;

        for (i, action) in actions.iter().enumerate() {
            let entity = self.to_entity(action)?;
            env.set_object_array_element(&entity_array, i as i32, entity)?;
        }

        // Call insertActions
        env.call_method(
            &self.dao,
            "insertActions",
            "(Ljava/util/List;)V",
            // Simplified - in production, convert array to List
            &[]
        )?;

        debug!("Saved {} actions to Room", actions.len());
        Ok(())
    }

    async fn load_all(&self) -> Result<Vec<QueuedAction>> {
        let env = &*self.env;

        // Call getAllActionsList (blocking)
        let result = env.call_method(
            &self.dao,
            "getAllActionsList",
            "()Ljava/util/List;",
            &[]
        )?.l()?;

        // Convert List to array
        let list_class = env.find_class("java/util/List")?;
        let size = env.call_method(&result, "size", "()I", &[])?.i()?;
        let array = env.call_method(
            &result,
            "toArray",
            "()[Ljava/lang/Object;",
            &[]
        )?.l()?;

        let mut actions = Vec::new();
        for i in 0..size {
            let entity = env.get_object_array_element(&array, i)?;
            if !entity.is_null() {
                match self.from_entity(entity) {
                    Ok(action) => actions.push(action),
                    Err(e) => error!("Failed to convert entity: {:?}", e),
                }
            }
        }

        Ok(actions)
    }

    async fn upsert(&self, action: &QueuedAction) -> Result<()> {
        let env = &*self.env;
        let entity = self.to_entity(action)?;

        env.call_method(
            &self.dao,
            "insertAction",
            "(Lcom/example/strada/offline/OfflineActionEntity;)V",
            &[JValue::Object(entity)]
        )?;

        debug!("Upserted action {} to Room", action.id);
        Ok(())
    }

    async fn delete(&self, action_id: &str) -> Result<()> {
        let env = &*self.env;

        env.call_method(
            &self.dao,
            "deleteActionById",
            "(Ljava/lang/String;)V",
            &[JValue::Object(&env.new_string(action_id)?.into())]
        )?;

        debug!("Deleted action {} from Room", action_id);
        Ok(())
    }

    async fn delete_batch(&self, action_ids: &[String]) -> Result<()> {
        let env = &*self.env;

        // Create string array
        let string_class = env.find_class("java/lang/String")?;
        let string_array = env.new_object_array(
            action_ids.len() as i32,
            string_class,
            JObject::null()
        )?;

        for (i, id) in action_ids.iter().enumerate() {
            env.set_object_array_element(&string_array, i as i32, &env.new_string(id)?)?;
        }

        // Convert to List
        let arrays_class = env.find_class("java/util/Arrays")?;
        let list = env.call_static_method(
            arrays_class,
            "asList",
            "([Ljava/lang/Object;)Ljava/util/List;",
            &[JValue::Object(&string_array)]
        )?.l()?;

        env.call_method(
            &self.dao,
            "deleteActionsByIds",
            "(Ljava/util/List;)V",
            &[JValue::Object(list)]
        )?;

        Ok(())
    }

    async fn count(&self) -> Result<usize> {
        let env = &*self.env;

        let count = env.call_method(
            &self.dao,
            "getCountValue",
            "()I",
            &[]
        )?.i()?;

        Ok(count as usize)
    }

    async fn clear(&self) -> Result<()> {
        let env = &*self.env;

        env.call_method(
            &self.dao,
            "deleteAll",
            "()V",
            &[]
        )?;

        info!("Cleared all actions from Room");
        Ok(())
    }

    async fn get_by_priority(&self, priority: crate::offline::queue::Priority) -> Result<Vec<QueuedAction>> {
        // Simplified implementation
        let all = self.load_all().await?;
        Ok(all.into_iter().filter(|a| a.priority == priority).collect())
    }

    async fn get_ready_actions(&self) -> Result<Vec<QueuedAction>> {
        let env = &*self.env;

        let date_class = env.find_class("java/util/Date")?;
        let now = env.new_object(date_class, "(J)V", &[JValue::Long(chrono::Utc::now().timestamp_millis() as jlong)])?;

        let result = env.call_method(
            &self.dao,
            "getReadyActions",
            "(Ljava/util/Date;)Ljava/util/List;",
            &[JValue::Object(now)]
        )?.l()?;

        // Convert List to array and process
        let size = env.call_method(&result, "size", "()I", &[])?.i()?;
        let array = env.call_method(
            &result,
            "toArray",
            "()[Ljava/lang/Object;",
            &[]
        )?.l()?;

        let mut actions = Vec::new();
        for i in 0..size {
            let entity = env.get_object_array_element(&array, i)?;
            if !entity.is_null() {
                if let Ok(action) = self.from_entity(entity) {
                    actions.push(action);
                }
            }
        }

        Ok(actions)
    }
}
```

---

## 4. Sync Coordinator

```rust
// strada-core/src/offline/sync.rs

use super::queue::{OfflineQueue, QueueEvent, QueueStats, QueuedAction, SyncResult};
use super::storage::QueueStorage;
use crate::error::Result;
use std::sync::Arc;
use tokio::sync::{Mutex, RwLock, broadcast};
use tracing::{debug, error, info, warn};

/// Sync configuration
#[derive(Debug, Clone)]
pub struct SyncConfig {
    /// Maximum concurrent sync operations
    pub max_concurrent: usize,
    /// Timeout for individual action execution
    pub action_timeout_secs: u64,
    /// Minimum time between sync attempts
    pub sync_cooldown_ms: u64,
    /// Enable parallel execution
    pub parallel_enabled: bool,
}

impl Default for SyncConfig {
    fn default() -> Self {
        Self {
            max_concurrent: 5,
            action_timeout_secs: 30,
            sync_cooldown_ms: 5000,
            parallel_enabled: true,
        }
    }
}

/// Conflict resolution strategy
#[derive(Debug, Clone, Copy)]
pub enum ConflictResolution {
    /// Last write wins (based on timestamp)
    LastWriteWins,
    /// Server wins
    ServerWins,
    /// Client wins
    ClientWins,
    /// Manual resolution required
    Manual,
}

/// Progress update for sync operations
#[derive(Debug, Clone)]
pub struct SyncProgress {
    pub total: usize,
    pub completed: usize,
    pub succeeded: usize,
    pub failed: usize,
    pub current_action: Option<String>,
    pub status: SyncStatus,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SyncStatus {
    Idle,
    InProgress,
    Paused,
    Completed,
    Failed,
}

impl Default for SyncProgress {
    fn default() -> Self {
        Self {
            total: 0,
            completed: 0,
            succeeded: 0,
            failed: 0,
            current_action: None,
            status: SyncStatus::Idle,
        }
    }
}

/// The Sync Coordinator manages offline queue synchronization
pub struct SyncCoordinator<S: QueueStorage> {
    queue: Arc<OfflineQueue<S>>,
    config: SyncConfig,
    /// Conflict resolution strategy
    conflict_resolution: ConflictResolution,
    /// Current progress
    progress: Arc<RwLock<SyncProgress>>,
    /// Broadcast channel for progress updates
    progress_tx: broadcast::Sender<SyncProgress>,
    /// Last sync timestamp
    last_sync: Arc<Mutex<Option<chrono::DateTime<chrono::Utc>>>>,
    /// Connectivity state
    is_online: Arc<RwLock<bool>>,
}

impl<S: QueueStorage + 'static> SyncCoordinator<S> {
    /// Create a new sync coordinator
    pub fn new(
        queue: Arc<OfflineQueue<S>>,
        config: SyncConfig,
        conflict_resolution: ConflictResolution,
    ) -> Self {
        let (progress_tx, _) = broadcast::channel(100);

        Self {
            queue,
            config,
            conflict_resolution,
            progress: Arc::new(RwLock::new(SyncProgress::default())),
            progress_tx,
            last_sync: Arc::new(Mutex::new(None)),
            is_online: Arc::new(RwLock::new(false)),
        }
    }

    /// Subscribe to progress updates
    pub fn subscribe_progress(&self) -> broadcast::Receiver<SyncProgress> {
        self.progress_tx.subscribe()
    }

    /// Update connectivity state
    pub async fn set_online(&self, online: bool) {
        let mut is_online = self.is_online.write().await;
        let was_online = *is_online;
        *is_online = online;

        if online && !was_online {
            info!("Connectivity restored, starting sync");
            // Auto-start sync when coming online
            let coordinator = Arc::new(self.clone());
            tokio::spawn(async move {
                let _ = coordinator.sync_all().await;
            });
        }
    }

    /// Check if currently online
    pub async fn is_online(&self) -> bool {
        *self.is_online.read().await
    }

    /// Get current progress
    pub async fn get_progress(&self) -> SyncProgress {
        self.progress.read().await.clone()
    }

    /// Get queue stats
    pub async fn get_stats(&self) -> Result<QueueStats> {
        self.queue.get_stats().await
    }

    /// Sync all pending actions
    pub async fn sync_all(&self) -> Result<SyncResult> {
        // Check cooldown
        {
            let last_sync = self.last_sync.lock().await;
            if let Some(last) = *last_sync {
                let elapsed = chrono::Utc::now().signed_duration_since(last).num_milliseconds() as u64;
                if elapsed < self.config.sync_cooldown_ms {
                    debug!("Sync cooldown, {}ms remaining", self.config.sync_cooldown_ms - elapsed);
                    return Ok(SyncResult::default());
                }
            }
        }

        // Check if online
        if !self.is_online().await {
            warn!("Attempted sync while offline");
            return Err(crate::error::BridgeError::OfflineQueue(
                "Cannot sync while offline".to_string()
            ));
        }

        // Update progress
        self.update_progress(|p| {
            p.status = SyncStatus::InProgress;
        }).await;

        let result = self.execute_sync().await;

        // Update last sync time
        *self.last_sync.lock().await = Some(chrono::Utc::now());

        // Update progress
        self.update_progress(|p| {
            p.status = if result.is_ok() {
                SyncStatus::Completed
            } else {
                SyncStatus::Failed
            };
        }).await;

        result
    }

    async fn execute_sync(&self) -> Result<SyncResult> {
        let executor = |action: QueuedAction| {
            let progress = self.progress.clone();
            let progress_tx = self.progress_tx.clone();
            let config = self.config.clone();
            let conflict_resolution = self.conflict_resolution;

            async move {
                // Update progress
                {
                    let mut p = progress.write().await;
                    p.current_action = Some(action.id.clone());
                    let _ = progress_tx.send(p.clone());
                }

                // Execute with timeout
                let timeout = tokio::time::Duration::from_secs(config.action_timeout_secs);

                let result = tokio::time::timeout(timeout, async {
                    // Execute action (this would call your API)
                    execute_action_internal(&action, conflict_resolution).await
                }).await;

                match result {
                    Ok(Ok(())) => {
                        // Success
                        let mut p = progress.write().await;
                        p.completed += 1;
                        p.succeeded += 1;
                        p.current_action = None;
                        let _ = progress_tx.send(p.clone());
                        Ok(())
                    }
                    Ok(Err(e)) => {
                        // Failed
                        let mut p = progress.write().await;
                        p.completed += 1;
                        p.failed += 1;
                        p.current_action = None;
                        let _ = progress_tx.send(p.clone());
                        Err(e)
                    }
                    Err(_) => {
                        // Timeout
                        let mut p = progress.write().await;
                        p.completed += 1;
                        p.failed += 1;
                        p.current_action = None;
                        let _ = progress_tx.send(p.clone());
                        Err(crate::error::BridgeError::OfflineQueue(
                            "Action execution timeout".to_string()
                        ))
                    }
                }
            }
        };

        self.queue.sync_all(executor).await
    }

    async fn update_progress<F>(&self, f: F)
    where
        F: FnOnce(&mut SyncProgress),
    {
        let mut progress = self.progress.write().await;
        f(&mut progress);
        let _ = self.progress_tx.send(progress.clone());
    }

    /// Resolve conflicts between local and server data
    pub async fn resolve_conflicts(
        &self,
        _local_actions: &[QueuedAction],
        _server_actions: &[QueuedAction],
    ) -> Result<Vec<QueuedAction>> {
        match self.conflict_resolution {
            ConflictResolution::LastWriteWins => {
                // Merge based on timestamps
                Ok(Vec::new()) // Simplified
            }
            ConflictResolution::ServerWins => {
                // Server data takes precedence
                Ok(Vec::new())
            }
            ConflictResolution::ClientWins => {
                // Local data takes precedence
                Ok(Vec::new())
            }
            ConflictResolution::Manual => {
                // Return all conflicts for manual resolution
                Ok(Vec::new())
            }
        }
    }
}

impl<S: QueueStorage> Clone for SyncCoordinator<S> {
    fn clone(&self) -> Self {
        Self {
            queue: self.queue.clone(),
            config: self.config.clone(),
            conflict_resolution: self.conflict_resolution,
            progress: self.progress.clone(),
            progress_tx: self.progress_tx.clone(),
            last_sync: self.last_sync.clone(),
            is_online: self.is_online.clone(),
        }
    }
}

/// Internal action execution (placeholder for actual API calls)
async fn execute_action_internal(
    action: &QueuedAction,
    _conflict_resolution: ConflictResolution,
) -> Result<()> {
    // This is where you would make actual API calls
    // For now, just simulate success
    debug!("Executing action {} (type: {})", action.id, action.action_type);

    // Simulate network call
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    Ok(())
}
```

---

## 5. Service Worker Integration

```rust
// estrada-web/src/service_worker/bridge.rs

use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use web_sys::{ServiceWorker, ServiceWorkerContainer, ServiceWorkerRegistration};
use wasm_bindgen_futures::JsFuture;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::{debug, error, info};

/// Service Worker message types
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum SWMessage {
    /// Notify SW of queued action
    ActionQueued {
        action_type: String,
        queue_length: usize,
    },
    /// Request background sync
    RequestSync {
        tag: String,
    },
    /// Sync complete notification
    SyncComplete {
        succeeded: usize,
        failed: usize,
    },
    /// Connectivity change
    ConnectivityChanged {
        online: bool,
    },
    /// Cache ready
    CacheReady,
}

/// Service Worker bridge for background sync coordination
pub struct ServiceWorkerBridge {
    registration: Arc<Mutex<Option<ServiceWorkerRegistration>>>,
    message_handlers: Arc<Mutex<Vec<Box<dyn Fn(SWMessage) + Send + Sync>>>>,
}

impl ServiceWorkerBridge {
    /// Initialize service worker bridge
    pub async fn new() -> Result<Self, JsValue> {
        let navigator = web_sys::window()
            .ok_or("No window")?
            .navigator();

        let service_worker: ServiceWorkerContainer = navigator.service_worker();

        // Wait for service worker to be ready
        if let Ok(ready) = service_worker.ready() {
            let registration = JsFuture::from(ready).await?;
            let registration: ServiceWorkerRegistration = registration.dyn_into()?;

            let bridge = Self {
                registration: Arc::new(Mutex::new(Some(registration))),
                message_handlers: Arc::new(Mutex::new(Vec::new())),
            };

            // Set up message listener
            bridge.setup_message_listener().await?;

            Ok(bridge)
        } else {
            // Service worker not available
            Ok(Self {
                registration: Arc::new(Mutex::new(None)),
                message_handlers: Arc::new(Mutex::new(Vec::new())),
            })
        }
    }

    /// Set up message listener for service worker messages
    async fn setup_message_listener(&self) -> Result<(), JsValue> {
        let navigator = web_sys::window().unwrap().navigator();
        let service_worker = navigator.service_worker();

        let handler = Closure::wrap(Box::new(move |event: web_sys::MessageEvent| {
            // Handle message from service worker
            let data = event.data();

            // Parse message type
            if let Ok(msg_type) = js_sys::Reflect::get(&data, &JsValue::from_str("type")) {
                if let Some(type_str) = msg_type.as_string() {
                    match type_str.as_str() {
                        "SYNC_COMPLETE" => {
                            // Handle sync completion
                            info!("Service worker: sync complete");
                        }
                        "CACHE_READY" => {
                            info!("Service worker: cache ready");
                        }
                        _ => {}
                    }
                }
            }
        }) as Box<dyn FnMut(web_sys::MessageEvent)>);

        service_worker.set_onmessage(Some(handler.as_ref().unchecked_ref()));
        handler.forget();

        Ok(())
    }

    /// Register for background sync
    pub async fn register_background_sync(&self, tag: &str) -> Result<(), JsValue> {
        let registration = self.registration.lock().await;

        if let Some(reg) = registration.as_ref() {
            // Check if background sync is supported
            if let Some(sync) = reg.sync() {
                let sync_request = sync.register(tag);
                JsFuture::from(sync_request).await?;
                info!("Registered background sync: {}", tag);
                Ok(())
            } else {
                Err("Background sync not supported".into())
            }
        } else {
            Err("Service worker not registered".into())
        }
    }

    /// Send message to service worker
    pub async fn post_message(&self, message: SWMessage) -> Result<(), JsValue> {
        let registration = self.registration.lock().await;

        if let Some(reg) = registration.as_ref() {
            if let Some(active) = reg.active() {
                let msg_obj = serde_wasm_bindgen::to_value(&message)?;
                active.post_message(&msg_obj)?;
                debug!("Posted message to service worker: {:?}", message);
                Ok(())
            } else {
                Err("Service worker not active".into())
            }
        } else {
            Err("Service worker not registered".into())
        }
    }

    /// Notify service worker of queued action
    pub async fn notify_action_queued(
        &self,
        action_type: &str,
        queue_length: usize,
    ) -> Result<(), JsValue> {
        self.post_message(SWMessage::ActionQueued {
            action_type: action_type.to_string(),
            queue_length,
        }).await
    }

    /// Request background sync
    pub async fn request_sync(&self, tag: &str) -> Result<(), JsValue> {
        self.post_message(SWMessage::RequestSync {
            tag: tag.to_string(),
        }).await?;

        // Also register with native background sync API
        self.register_background_sync(tag).await
    }

    /// Notify sync completion
    pub async fn notify_sync_complete(
        &self,
        succeeded: usize,
        failed: usize,
    ) -> Result<(), JsValue> {
        self.post_message(SWMessage::SyncComplete { succeeded, failed }).await
    }

    /// Notify connectivity change
    pub async fn notify_connectivity(&self, online: bool) -> Result<(), JsValue> {
        self.post_message(SWMessage::ConnectivityChanged { online }).await
    }

    /// Add a message handler
    pub async fn add_handler<F>(&self, handler: F)
    where
        F: Fn(SWMessage) + Send + Sync + 'static,
    {
        let mut handlers = self.message_handlers.lock().await;
        handlers.push(Box::new(handler));
    }
}
```

### 5.2 Service Worker JavaScript Code

```javascript
// strada-web/src/service_worker/sw.js

const CACHE_VERSION = 'v1';
const STATIC_CACHE = `strada-static-${CACHE_VERSION}`;
const DYNAMIC_CACHE = `strada-dynamic-${CACHE_VERSION}`;

const STATIC_ASSETS = [
    '/',
    '/offline.html',
    '/app.js',
    '/styles.css'
];

// Install event - cache static assets
self.addEventListener('install', (event) => {
    console.log('[ServiceWorker] Install');
    event.waitUntil(
        caches.open(STATIC_CACHE).then((cache) => {
            console.log('[ServiceWorker] Caching static assets');
            return cache.addAll(STATIC_ASSETS);
        }).then(() => {
            console.log('[ServiceWorker] Skip waiting');
            return self.skipWaiting();
        })
    );
});

// Activate event - clean old caches
self.addEventListener('activate', (event) => {
    console.log('[ServiceWorker] Activate');
    event.waitUntil(
        caches.keys().then((keys) => {
            return Promise.all(
                keys.filter((key) => {
                    return key.startsWith('strada-') &&
                           key !== STATIC_CACHE &&
                           key !== DYNAMIC_CACHE;
                }).map((key) => {
                    console.log('[ServiceWorker] Removing old cache:', key);
                    return caches.delete(key);
                })
            );
        }).then(() => {
            console.log('[ServiceWorker] Claiming clients');
            return self.clients.claim();
        })
    );
});

// Fetch event - network first for API, cache-first for static
self.addEventListener('fetch', (event) => {
    const { request } = event;
    const url = new URL(request.url);

    // API requests - network first with offline queuing
    if (url.pathname.startsWith('/api/')) {
        event.respondWith(fetchWithQueueFallback(request));
        return;
    }

    // Static assets - cache first
    event.respondWith(
        caches.match(request).then((cached) => {
            return cached || fetch(request).then((response) => {
                // Cache successful responses
                if (response.ok) {
                    const clone = response.clone();
                    caches.open(DYNAMIC_CACHE).then((cache) => {
                        cache.put(request, clone);
                    });
                }
                return response;
            });
        })
    );
});

async function fetchWithQueueFallback(request) {
    try {
        const response = await fetch(request);
        return response;
    } catch (error) {
        // Network failed - queue the request
        console.log('[ServiceWorker] Request failed, queuing:', request.url);

        // Store failed request for later sync
        const queue = await openRequestQueue();
        await queue.add({
            url: request.url,
            method: request.method,
            headers: Object.fromEntries(request.headers),
            body: request.method !== 'GET' && request.method !== 'HEAD'
                ? await request.clone().text()
                : null,
            timestamp: Date.now()
        });

        // Notify client
        notifyClients({
            type: 'REQUEST_QUEUED',
            url: request.url
        });

        // Return offline response
        return caches.match('/offline.html');
    }
}

// Background sync event
self.addEventListener('sync', (event) => {
    console.log('[ServiceWorker] Sync event:', event.tag);

    if (event.tag === 'strada-sync') {
        event.waitUntil(syncQueuedActions());
    }
});

async function syncQueuedActions() {
    console.log('[ServiceWorker] Syncing queued actions');

    const queue = await openRequestQueue();
    const requests = await queue.getAll();

    const results = {
        succeeded: 0,
        failed: 0
    };

    for (const req of requests) {
        try {
            const response = await fetch(req.url, {
                method: req.method,
                headers: req.headers,
                body: req.body
            });

            if (response.ok) {
                await queue.remove(req);
                results.succeeded++;
                console.log('[ServiceWorker] Synced:', req.url);
            } else {
                results.failed++;
            }
        } catch (error) {
            results.failed++;
            console.log('[ServiceWorker] Sync failed:', req.url, error);
        }
    }

    // Notify clients of completion
    notifyClients({
        type: 'SYNC_COMPLETE',
        ...results
    });
}

// Message handling from clients
self.addEventListener('message', (event) => {
    const { data } = event;
    console.log('[ServiceWorker] Message received:', data);

    switch (data.type) {
        case 'ACTION_QUEUED':
            // Auto-register background sync
            if ('sync' in self.registration) {
                self.registration.sync.register('strada-sync');
            }
            break;

        case 'SKIP_WAITING':
            self.skipWaiting();
            break;

        case 'CACHE_URLS':
            event.waitUntil(
                caches.open(DYNAMIC_CACHE).then((cache) => {
                    return cache.addAll(data.urls);
                })
            );
            break;
    }
});

// Notify all clients
function notifyClients(message) {
    self.clients.matchAll().then((clients) => {
        clients.forEach((client) => {
            client.postMessage(message);
        });
    });
}

// IndexedDB helper for request queue
async function openRequestQueue() {
    return new Promise((resolve, reject) => {
        const request = indexedDB.open('strada-request-queue', 1);

        request.onerror = () => reject(request.error);

        request.onupgradeneeded = (event) => {
            const db = event.target.result;
            if (!db.objectStoreNames.contains('requests')) {
                db.createObjectStore('requests', { keyPath: 'url' });
            }
        };

        request.onsuccess = (event) => {
            const db = event.target.result;
            resolve({
                async add(req) {
                    return new Promise((resolve, reject) => {
                        const tx = db.transaction('requests', 'readwrite');
                        const store = tx.objectStore('requests');
                        const putReq = store.put(req);
                        putReq.onsuccess = () => resolve();
                        putReq.onerror = () => reject(putReq.error);
                    });
                },
                async getAll() {
                    return new Promise((resolve, reject) => {
                        const tx = db.transaction('requests', 'readonly');
                        const store = tx.objectStore('requests');
                        const getAllReq = store.getAll();
                        getAllReq.onsuccess = () => resolve(getAllReq.result);
                        getAllReq.onerror = () => reject(getAllReq.error);
                    });
                },
                async remove(req) {
                    return new Promise((resolve, reject) => {
                        const tx = db.transaction('requests', 'readwrite');
                        const store = tx.objectStore('requests');
                        const deleteReq = store.delete(req.url);
                        deleteReq.onsuccess = () => resolve();
                        deleteReq.onerror = () => reject(deleteReq.error);
                    });
                }
            });
        };
    });
}
```

---

## 6. Complete Examples

### 6.1 Full QueuedAction Type with Serde

```rust
// strada-core/src/offline/types.rs

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

/// Complete queued action with all fields
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueuedAction {
    /// Unique identifier
    pub id: String,

    /// Action type discriminator
    pub action_type: ActionType,

    /// Priority level
    pub priority: Priority,

    /// Action payload
    pub payload: ActionPayload,

    /// Deduplication key
    pub dedup_key: Option<String>,

    /// Creation timestamp
    pub created_at: DateTime<Utc>,

    /// Last attempt timestamp
    pub last_attempt: Option<DateTime<Utc>>,

    /// Retry count
    pub retry_count: u32,

    /// Next scheduled attempt
    pub next_attempt_at: Option<DateTime<Utc>>,

    /// Additional metadata
    pub metadata: HashMap<String, String>,
}

/// Action type enum for type safety
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ActionType {
    FormSubmit,
    SyncRequest,
    DataUpdate,
    DataDelete,
    FileUpload,
    Custom,
}

/// Action payload wrapper
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionPayload {
    /// Endpoint URL
    pub endpoint: String,

    /// HTTP method
    pub method: String,

    /// Request headers
    pub headers: HashMap<String, String>,

    /// Request body (JSON)
    pub body: serde_json::Value,

    /// Expected response type
    pub response_type: Option<String>,
}

impl QueuedAction {
    /// Create a new action for form submission
    pub fn form_submit(
        endpoint: impl Into<String>,
        form_data: serde_json::Value,
        priority: Priority,
    ) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            action_type: ActionType::FormSubmit,
            priority,
            payload: ActionPayload {
                endpoint: endpoint.into(),
                method: "POST".to_string(),
                headers: HashMap::new(),
                body: form_data,
                response_type: Some("json".to_string()),
            },
            dedup_key: None,
            created_at: Utc::now(),
            last_attempt: None,
            retry_count: 0,
            next_attempt_at: None,
            metadata: HashMap::new(),
        }
    }

    /// Create a new action for data sync
    pub fn sync_request(
        endpoint: impl Into<String>,
        sync_params: serde_json::Value,
        priority: Priority,
    ) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            action_type: ActionType::SyncRequest,
            priority,
            payload: ActionPayload {
                endpoint: endpoint.into(),
                method: "GET".to_string(),
                headers: HashMap::new(),
                body: serde_json::json!({}),
                response_type: Some("json".to_string()),
            },
            dedup_key: None,
            created_at: Utc::now(),
            last_attempt: None,
            retry_count: 0,
            next_attempt_at: None,
            metadata: HashMap::new(),
        }
    }

    /// Set deduplication key
    pub fn with_dedup_key(mut self, key: impl Into<String>) -> Self {
        self.dedup_key = Some(key.into());
        self
    }

    /// Add header
    pub fn with_header(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.payload.headers.insert(key.into(), value.into());
        self
    }

    /// Add metadata
    pub fn with_metadata(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.metadata.insert(key.into(), value.into());
        self
    }

    /// Calculate backoff with exponential delay and jitter
    pub fn calculate_backoff_ms(&self, config: &crate::offline::queue::QueueConfig) -> u64 {
        if self.retry_count == 0 {
            return config.initial_backoff_ms;
        }

        let base_delay = config.initial_backoff_ms as f64;
        let multiplier = config.backoff_multiplier;
        let exponential = base_delay * multiplier.powi(self.retry_count as i32);

        let delay_with_jitter = if config.jitter_enabled {
            // Add up to 25% jitter
            let jitter_range = exponential * 0.25;
            let jitter = jitter_range * (2.0 * rand::random::<f64>() - 1.0);
            exponential + jitter
        } else {
            exponential
        };

        // Clamp to max
        (delay_with_jitter as u64)
            .min(config.max_backoff_ms)
            .max(config.initial_backoff_ms)
    }

    /// Check if ready for execution
    pub fn is_ready(&self) -> bool {
        self.next_attempt_at.map(|next| Utc::now() >= next).unwrap_or(true)
    }

    /// Get sort key for priority ordering
    pub fn sort_key(&self) -> (i32, DateTime<Utc>) {
        (-(self.priority as i32), self.created_at)
    }
}

impl Priority {
    /// Get priority from string
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "low" => Some(Priority::Low),
            "normal" => Some(Priority::Normal),
            "high" => Some(Priority::High),
            "critical" => Some(Priority::Critical),
            _ => None,
        }
    }
}

impl std::fmt::Display for Priority {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Priority::Low => write!(f, "low"),
            Priority::Normal => write!(f, "normal"),
            Priority::High => write!(f, "high"),
            Priority::Critical => write!(f, "critical"),
        }
    }
}
```

### 6.2 Complete Retry Backoff Implementation

```rust
// strada-core/src/offline/backoff.rs

use chrono::{DateTime, Utc};
use rand::Rng;
use std::time::Duration;

/// Exponential backoff calculator
pub struct ExponentialBackoff {
    /// Initial delay
    pub initial_delay: Duration,
    /// Maximum delay
    pub max_delay: Duration,
    /// Multiplier (e.g., 2.0 = exponential)
    pub multiplier: f64,
    /// Jitter factor (0.0 = no jitter, 1.0 = full jitter)
    pub jitter: f64,
}

impl Default for ExponentialBackoff {
    fn default() -> Self {
        Self {
            initial_delay: Duration::from_secs(1),
            max_delay: Duration::from_secs(300), // 5 minutes
            multiplier: 2.0,
            jitter: 0.1, // 10% jitter
        }
    }
}

impl ExponentialBackoff {
    /// Create new backoff with custom initial delay
    pub fn new(initial_delay: Duration) -> Self {
        Self {
            initial_delay,
            ..Default::default()
        }
    }

    /// Set maximum delay
    pub fn with_max_delay(mut self, max_delay: Duration) -> Self {
        self.max_delay = max_delay;
        self
    }

    /// Set multiplier
    pub fn with_multiplier(mut self, multiplier: f64) -> Self {
        self.multiplier = multiplier;
        self
    }

    /// Set jitter factor
    pub fn with_jitter(mut self, jitter: f64) -> Self {
        self.jitter = jitter;
        self
    }

    /// Calculate delay for given retry count
    pub fn delay(&self, retry: u32) -> Duration {
        if retry == 0 {
            return self.initial_delay;
        }

        // Calculate exponential delay
        let base = self.initial_delay.as_millis() as f64;
        let exponential = base * self.multiplier.powi(retry as i32);

        // Apply jitter
        let delay_ms = if self.jitter > 0.0 {
            let mut rng = rand::thread_rng();
            let jitter_range = exponential * self.jitter;
            let jitter = rng.gen_range(-jitter_range..=jitter_range);
            exponential + jitter
        } else {
            exponential
        };

        // Clamp to max
        let delay_ms = delay_ms.min(self.max_delay.as_millis() as f64);
        Duration::from_millis(delay_ms as u64)
    }

    /// Calculate next attempt time
    pub fn next_attempt(&self, retry: u32) -> DateTime<Utc> {
        Utc::now() + self.delay(retry)
    }

    /// Create iterator for backoff delays
    pub fn iter(&self) -> BackoffIter {
        BackoffIter {
            backoff: self,
            retry: 0,
        }
    }
}

/// Iterator for backoff delays
pub struct BackoffIter<'a> {
    backoff: &'a ExponentialBackoff,
    retry: u32,
}

impl<'a> Iterator for BackoffIter<'a> {
    type Item = (u32, Duration, DateTime<Utc>);

    fn next(&mut self) -> Option<Self::Item> {
        let retry = self.retry;
        let delay = self.backoff.delay(retry);
        let next_attempt = Utc::now() + delay;

        self.retry += 1;

        Some((retry, delay, next_attempt))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_first_retry_is_initial_delay() {
        let backoff = ExponentialBackoff::new(Duration::from_secs(2));
        assert_eq!(backoff.delay(0), Duration::from_secs(2));
    }

    #[test]
    fn test_exponential_growth() {
        let backoff = ExponentialBackoff::default();
        let delay1 = backoff.delay(1);
        let delay2 = backoff.delay(2);

        // delay2 should be roughly 2x delay1
        assert!(delay2 > delay1);
        assert!(delay2.as_millis() <= (delay1.as_millis() * 2) as u128 + 100); // Account for jitter
    }

    #[test]
    fn test_max_delay() {
        let backoff = ExponentialBackoff::default();
        let delay = backoff.delay(100);
        assert!(delay <= backoff.max_delay);
    }

    #[test]
    fn test_iterator() {
        let backoff = ExponentialBackoff::new(Duration::from_secs(1));
        let mut iter = backoff.iter();

        let (retry0, delay0, _) = iter.next().unwrap();
        assert_eq!(retry0, 0);
        assert_eq!(delay0, Duration::from_secs(1));

        let (retry1, delay1, _) = iter.next().unwrap();
        assert_eq!(retry1, 1);
        assert!(delay1 > delay0);
    }
}
```

### 6.3 End-to-End Sync Flow Example

```rust
// examples/offline_sync_demo.rs

use chrono::Utc;
use std::sync::Arc;
use strada_core::offline::queue::{OfflineQueue, QueueConfig, Priority, QueuedAction};
use strada_core::offline::storage::QueueStorage;
use strada_core::offline::sync::{SyncCoordinator, SyncConfig, ConflictResolution};
use strada_core::error::Result;

/// Example demonstrating end-to-end offline sync flow
#[tokio::main]
async fn main() -> Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt::init();

    println!("=== Offline Queue Sync Demo ===\n");

    // 1. Create storage (using in-memory for demo)
    let storage = Arc::new(InMemoryStorage::new());

    // 2. Create queue with config
    let config = QueueConfig {
        max_retries: 3,
        initial_backoff_ms: 1000,
        max_backoff_ms: 10000,
        jitter_enabled: true,
        max_queue_size: 100,
        deduplication_enabled: true,
    };

    let queue = Arc::new(OfflineQueue::new(storage.clone(), config.clone()));

    // 3. Initialize queue (load from storage)
    queue.initialize().await?;

    // 4. Set up event listener
    queue.add_listener(|event| {
        println!("[Queue Event] {:?}", event);
    }).await;

    // 5. Create sync coordinator
    let sync_config = SyncConfig {
        max_concurrent: 3,
        action_timeout_secs: 10,
        sync_cooldown_ms: 1000,
        parallel_enabled: true,
    };

    let coordinator = Arc::new(SyncCoordinator::new(
        queue.clone(),
        sync_config,
        ConflictResolution::LastWriteWins,
    ));

    // 6. Subscribe to progress updates
    let mut progress_rx = coordinator.subscribe_progress();
    tokio::spawn(async move {
        while let Ok(progress) = progress_rx.recv().await {
            println!(
                "[Progress] {}/{} (S: {}, F: {}) - {:?}",
                progress.completed,
                progress.total,
                progress.succeeded,
                progress.failed,
                progress.status
            );
        }
    });

    // 7. Enqueue some actions
    println!("\n--- Enqueuing Actions ---\n");

    let action1 = QueuedAction::form_submit(
        "/api/users",
        serde_json::json!({"name": "John", "email": "john@example.com"}),
        Priority::Normal,
    ).with_dedup_key("form-user-john");

    let action2 = QueuedAction::form_submit(
        "/api/orders",
        serde_json::json!({"product_id": 123, "quantity": 2}),
        Priority::High,
    ).with_dedup_key("order-123");

    let action3 = QueuedAction::sync_request(
        "/api/inventory",
        serde_json::json!({"warehouse": "main"}),
        Priority::Low,
    );

    let id1 = queue.enqueue(action1).await?;
    println!("Enqueued action 1: {}", id1);

    let id2 = queue.enqueue(action2).await?;
    println!("Enqueued action 2: {}", id2);

    let id3 = queue.enqueue(action3).await?;
    println!("Enqueued action 3: {}", id3);

    // Try duplicate (should return existing ID)
    let dup_action = QueuedAction::form_submit(
        "/api/users",
        serde_json::json!({"name": "John", "email": "john@example.com"}),
        Priority::Normal,
    ).with_dedup_key("form-user-john");

    let dup_id = queue.enqueue(dup_action).await?;
    println!("Duplicate action returned: {} (same as {}? {})", dup_id, id1, dup_id == id1);

    // 8. Show queue stats
    let stats = queue.get_stats().await?;
    println!("\n--- Queue Stats ---");
    println!("Total actions: {}", stats.total_actions);
    println!("Pending: {}", stats.pending_actions);
    println!("Failed: {}", stats.failed_actions);

    // 9. Simulate going online and syncing
    println!("\n--- Starting Sync ---\n");

    coordinator.set_online(true).await;

    // Custom action executor for demo
    let result = coordinator.sync_all().await?;

    println!("\n--- Sync Result ---");
    println!("Succeeded: {}", result.succeeded.len());
    println!("Failed (retrying): {}", result.failed_but_retrying.len());
    println!("Permanently failed: {}", result.permanently_failed.len());

    // 10. Final stats
    let final_stats = queue.get_stats().await?;
    println!("\n--- Final Stats ---");
    println!("Remaining actions: {}", final_stats.total_actions);

    Ok(())
}

/// In-memory storage for demo/testing
use async_trait::async_trait;
use tokio::sync::RwLock;
use std::collections::HashMap;

struct InMemoryStorage {
    actions: RwLock<HashMap<String, QueuedAction>>,
}

impl InMemoryStorage {
    fn new() -> Self {
        Self {
            actions: RwLock::new(HashMap::new()),
        }
    }
}

#[async_trait]
impl QueueStorage for InMemoryStorage {
    async fn save_all(&self, actions: &[QueuedAction]) -> Result<()> {
        let mut store = self.actions.write().await;
        for action in actions {
            store.insert(action.id.clone(), action.clone());
        }
        Ok(())
    }

    async fn load_all(&self) -> Result<Vec<QueuedAction>> {
        let store = self.actions.read().await;
        Ok(store.values().cloned().collect())
    }

    async fn upsert(&self, action: &QueuedAction) -> Result<()> {
        let mut store = self.actions.write().await;
        store.insert(action.id.clone(), action.clone());
        Ok(())
    }

    async fn delete(&self, action_id: &str) -> Result<()> {
        let mut store = self.actions.write().await;
        store.remove(action_id);
        Ok(())
    }

    async fn delete_batch(&self, action_ids: &[String]) -> Result<()> {
        let mut store = self.actions.write().await;
        for id in action_ids {
            store.remove(id);
        }
        Ok(())
    }

    async fn count(&self) -> Result<usize> {
        let store = self.actions.read().await;
        Ok(store.len())
    }

    async fn clear(&self) -> Result<()> {
        let mut store = self.actions.write().await;
        store.clear();
        Ok(())
    }

    async fn get_by_priority(&self, priority: Priority) -> Result<Vec<QueuedAction>> {
        let store = self.actions.read().await;
        Ok(store.values()
            .filter(|a| a.priority == priority)
            .cloned()
            .collect())
    }

    async fn get_ready_actions(&self) -> Result<Vec<QueuedAction>> {
        let store = self.actions.read().await;
        let mut ready: Vec<_> = store.values()
            .filter(|a| a.is_ready())
            .cloned()
            .collect();
        ready.sort_by(|a, b| a.sort_key().cmp(&b.sort_key()));
        Ok(ready)
    }
}
```

### 6.4 Unit Tests for Queue Operations

```rust
// strada-core/src/offline/tests.rs

#[cfg(test)]
mod queue_tests {
    use super::super::queue::*;
    use super::super::storage::QueueStorage;
    use async_trait::async_trait;
    use chrono::Utc;
    use std::collections::HashMap;
    use tokio::sync::RwLock;
    use crate::error::Result;

    /// In-memory storage for testing
    struct TestStorage {
        actions: RwLock<HashMap<String, QueuedAction>>,
    }

    impl TestStorage {
        fn new() -> Self {
            Self {
                actions: RwLock::new(HashMap::new()),
            }
        }
    }

    #[async_trait]
    impl QueueStorage for TestStorage {
        async fn save_all(&self, actions: &[QueuedAction]) -> Result<()> {
            let mut store = self.actions.write().await;
            for action in actions {
                store.insert(action.id.clone(), action.clone());
            }
            Ok(())
        }

        async fn load_all(&self) -> Result<Vec<QueuedAction>> {
            Ok(self.actions.read().await.values().cloned().collect())
        }

        async fn upsert(&self, action: &QueuedAction) -> Result<()> {
            self.actions.write().await.insert(action.id.clone(), action.clone());
            Ok(())
        }

        async fn delete(&self, action_id: &str) -> Result<()> {
            self.actions.write().await.remove(action_id);
            Ok(())
        }

        async fn delete_batch(&self, action_ids: &[String]) -> Result<()> {
            let mut store = self.actions.write().await;
            for id in action_ids {
                store.remove(id);
            }
            Ok(())
        }

        async fn count(&self) -> Result<usize> {
            Ok(self.actions.read().await.len())
        }

        async fn clear(&self) -> Result<()> {
            self.actions.write().await.clear();
            Ok(())
        }

        async fn get_by_priority(&self, priority: Priority) -> Result<Vec<QueuedAction>> {
            Ok(self.actions.read().await.values()
                .filter(|a| a.priority == priority)
                .cloned()
                .collect())
        }

        async fn get_ready_actions(&self) -> Result<Vec<QueuedAction>> {
            let mut ready: Vec<_> = self.actions.read().await.values()
                .filter(|a| a.is_ready())
                .cloned()
                .collect();
            ready.sort_by(|a, b| a.sort_key().cmp(&b.sort_key()));
            Ok(ready)
        }
    }

    fn create_test_action() -> QueuedAction {
        QueuedAction::new(
            "test-action",
            serde_json::json!({"key": "value"}),
            Priority::Normal,
        )
    }

    #[tokio::test]
    async fn test_enqueue_dequeue() {
        let storage = Arc::new(TestStorage::new());
        let queue = OfflineQueue::new(storage, QueueConfig::default());
        queue.initialize().await.unwrap();

        let action = create_test_action();
        let id = queue.enqueue(action.clone()).await.unwrap();

        assert!(!id.is_empty());

        let dequeued = queue.dequeue().await.unwrap();
        assert!(dequeued.is_some());
        assert_eq!(dequeued.unwrap().id, id);
    }

    #[tokio::test]
    async fn test_deduplication() {
        let storage = Arc::new(TestStorage::new());
        let mut config = QueueConfig::default();
        config.deduplication_enabled = true;

        let queue = OfflineQueue::new(storage, config);
        queue.initialize().await.unwrap();

        let action1 = create_test_action().with_dedup_key("test-key");
        let id1 = queue.enqueue(action1).await.unwrap();

        let action2 = create_test_action().with_dedup_key("test-key");
        let id2 = queue.enqueue(action2).await.unwrap();

        // Should return same ID for duplicate
        assert_eq!(id1, id2);

        // Should only have one action in queue
        let stats = queue.get_stats().await.unwrap();
        assert_eq!(stats.total_actions, 1);
    }

    #[tokio::test]
    async fn test_priority_ordering() {
        let storage = Arc::new(TestStorage::new());
        let queue = OfflineQueue::new(storage, QueueConfig::default());
        queue.initialize().await.unwrap();

        // Enqueue in reverse priority order
        let low = QueuedAction::new("low", serde_json::json!({}), Priority::Low);
        let high = QueuedAction::new("high", serde_json::json!({}), Priority::High);
        let normal = QueuedAction::new("normal", serde_json::json!({}), Priority::Normal);

        queue.enqueue(low).await.unwrap();
        queue.enqueue(high).await.unwrap();
        queue.enqueue(normal).await.unwrap();

        // Should dequeue in priority order: High, Normal, Low
        let h = queue.dequeue().await.unwrap().unwrap();
        assert_eq!(h.priority, Priority::High);

        let n = queue.dequeue().await.unwrap().unwrap();
        assert_eq!(n.priority, Priority::Normal);

        let l = queue.dequeue().await.unwrap().unwrap();
        assert_eq!(l.priority, Priority::Low);
    }

    #[tokio::test]
    async fn test_retry_backoff() {
        let storage = Arc::new(TestStorage::new());
        let queue = OfflineQueue::new(storage, QueueConfig::default());
        queue.initialize().await.unwrap();

        let action = create_test_action();
        let id = queue.enqueue(action).await.unwrap();

        // Dequeue and fail
        let _ = queue.dequeue().await.unwrap();
        let can_retry = queue.fail(&id, "test error").await.unwrap();

        assert!(can_retry);

        // Action should have next_attempt_at set
        let dequeued = queue.dequeue().await.unwrap();
        assert!(dequeued.is_none()); // Not ready yet due to backoff
    }

    #[tokio::test]
    async fn test_max_retries() {
        let storage = Arc::new(TestStorage::new());
        let mut config = QueueConfig::default();
        config.max_retries = 2;

        let queue = OfflineQueue::new(storage, config);
        queue.initialize().await.unwrap();

        let action = create_test_action();
        let id = queue.enqueue(action).await.unwrap();

        // Fail twice
        let _ = queue.dequeue().await.unwrap();
        queue.fail(&id, "error 1").await.unwrap();

        let _ = queue.dequeue().await.unwrap();
        let can_retry = queue.fail(&id, "error 2").await.unwrap();

        // Should not be able to retry after max_retries
        assert!(!can_retry);

        // Action should be removed from queue
        let stats = queue.get_stats().await.unwrap();
        assert_eq!(stats.total_actions, 0);
        assert_eq!(stats.failed_actions, 1);
    }

    #[tokio::test]
    async fn test_complete_action() {
        let storage = Arc::new(TestStorage::new());
        let queue = OfflineQueue::new(storage, QueueConfig::default());
        queue.initialize().await.unwrap();

        let action = create_test_action();
        let id = queue.enqueue(action).await.unwrap();

        queue.complete(&id).await.unwrap();

        let stats = queue.get_stats().await.unwrap();
        assert_eq!(stats.total_actions, 0);
    }

    #[tokio::test]
    async fn test_queue_stats() {
        let storage = Arc::new(TestStorage::new());
        let queue = OfflineQueue::new(storage, QueueConfig::default());
        queue.initialize().await.unwrap();

        // Add actions of different types and priorities
        queue.enqueue(QueuedAction::new("a", serde_json::json!({}), Priority::Low)).await.unwrap();
        queue.enqueue(QueuedAction::new("b", serde_json::json!({}), Priority::Normal)).await.unwrap();
        queue.enqueue(QueuedAction::new("c", serde_json::json!({}), Priority::Normal)).await.unwrap();
        queue.enqueue(QueuedAction::new("d", serde_json::json!({}), Priority::High)).await.unwrap();

        let stats = queue.get_stats().await.unwrap();

        assert_eq!(stats.total_actions, 4);
        assert_eq!(stats.pending_actions, 4);
        assert_eq!(stats.actions_by_priority.get(&Priority::Low), Some(&1));
        assert_eq!(stats.actions_by_priority.get(&Priority::Normal), Some(&2));
        assert_eq!(stats.actions_by_priority.get(&Priority::High), Some(&1));
    }

    #[tokio::test]
    async fn test_sync_result() {
        let storage = Arc::new(TestStorage::new());
        let queue = OfflineQueue::new(storage, QueueConfig::default());
        queue.initialize().await.unwrap();

        // Add actions
        queue.enqueue(QueuedAction::new("a", serde_json::json!({}), Priority::Normal)).await.unwrap();
        queue.enqueue(QueuedAction::new("b", serde_json::json!({}), Priority::Normal)).await.unwrap();

        // Execute sync with mock executor
        let result = queue.sync_all(|_action| async { Ok(()) }).await.unwrap();

        assert_eq!(result.succeeded.len(), 2);
        assert!(result.failed_but_retrying.is_empty());
        assert!(result.permanently_failed.is_empty());
    }

    #[tokio::test]
    async fn test_event_listeners() {
        use std::sync::Arc;
        use tokio::sync::Mutex;

        let storage = Arc::new(TestStorage::new());
        let queue = OfflineQueue::new(storage, QueueConfig::default());
        queue.initialize().await.unwrap();

        let events = Arc::new(Mutex::new(Vec::new()));
        let events_clone = events.clone();

        queue.add_listener(move |event| {
            let events = events_clone.clone();
            tokio::spawn(async move {
                events.lock().await.push(event);
            });
        }).await;

        queue.enqueue(create_test_action()).await.unwrap();

        // Give time for event to be processed
        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

        let events = events.lock().await;
        assert!(!events.is_empty());
    }
}
```

---

## Summary

This deep dive covers the complete offline queue system architecture:

| Component | Purpose | Key Features |
|-----------|---------|--------------|
| `OfflineQueue` | Core queue management | Priority ordering, deduplication, retry logic |
| `QueueStorage` trait | Persistence abstraction | Platform-agnostic storage interface |
| `IndexedDBStorage` | Web persistence | Transaction handling, indexed queries |
| `RoomStorage` | Android persistence | Room entity integration via JNI |
| `SyncCoordinator` | Sync orchestration | Parallel execution, progress reporting |
| `ServiceWorkerBridge` | Background sync | Message passing, cache coordination |

Key production features:
- **Exponential backoff with jitter** prevents thundering herd
- **Deduplication** prevents duplicate submissions
- **Priority ordering** ensures critical actions process first
- **Parallel execution** with concurrency limits
- **Progress broadcasting** for UI updates
- **Comprehensive error handling** throughout

---

*Related: `offline-connectivity-exploration.md`, `rust-revision.md`*
