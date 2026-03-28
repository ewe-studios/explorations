---
title: "Rust Revision: Alchemy for ewe_platform"
subtitle: "Complete Rust translation patterns for ewe_platform using Valtron"
location: /home/darkvoid/Boxxed/@dev/repo-expolorations/alchemy
explored_at: 2026-03-27
---

# Rust Revision: Alchemy for ewe_platform

## Overview

This document translates Alchemy's TypeScript patterns into Rust for implementation in `ewe_platform` using the Valtron executor. The key constraint: **NO async/await, NO tokio** - all code uses Valtron's TaskIterator pattern.

## Core Type Translations

### TypeScript: Resource

```typescript
// alchemy/src/resource.ts
export const ResourceKind = Symbol.for("alchemy::ResourceKind");
export const ResourceID = Symbol.for("alchemy::ResourceID");
export const ResourceFQN = Symbol.for("alchemy::ResourceFQN");

export interface Resource<Kind extends string> {
  [ResourceKind]: Kind;
  [ResourceID]: string;
  [ResourceFQN]: string;
  [ResourceScope]: Scope;
  [ResourceSeq]: number;
}

export function Resource<Type, Handler>(
  type: Type,
  handler: Handler
): Provider<Type, Handler> {
  // Registration logic
}
```

### Rust: Resource

```rust
// ewe_platform/backends/foundation_core/src/resource.rs

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

/// Global provider registry
lazy_static! {
    static ref PROVIDERS: Mutex<HashMap<String, ProviderFn>> = Mutex::new(HashMap::new());
}

type ProviderFn = Arc<dyn Fn(&str, &serde_json::Value) -> TaskResult + Send + Sync>;

/// Resource metadata stored as struct fields (no runtime symbols in Rust)
#[derive(Debug, Clone)]
pub struct ResourceMetadata {
    pub kind: String,
    pub id: String,
    pub fqn: String,
    pub seq: usize,
    pub scope_id: String,
}

/// Resource wrapper with metadata
pub struct Resource<Output> {
    pub metadata: ResourceMetadata,
    pub output: Output,
}

/// Provider registration
pub fn register_provider<F>(
    type_name: &str,
    handler: F,
) where
    F: Fn(&str, &serde_json::Value) -> TaskResult + Send + Sync + 'static,
{
    let mut providers = PROVIDERS.lock().unwrap();
    providers.insert(type_name.to_string(), Arc::new(handler));
}

/// Resource factory (Valtron-based)
pub fn resource<Output>(
    type_name: &str,
    id: &str,
    props: &serde_json::Value,
) -> ValtronResult<Resource<Output>>
where
    Output: serde::de::DeserializeOwned,
{
    let providers = PROVIDERS.lock().unwrap();
    let provider = providers.get(type_name)
        .cloned()
        .ok_or_else(|| format!("Provider '{}' not found", type_name))?;

    // Create pending resource
    let pending = PendingResource {
        type_name: type_name.to_string(),
        id: id.to_string(),
        props: props.clone(),
    };

    // Apply through Valtron executor
    apply(pending, provider)
}
```

### TypeScript: Scope

```typescript
// alchemy/src/scope.ts
export class Scope {
  public static storage = new AsyncLocalStorage<Scope>();
  public readonly resources = new Map<string, PendingResource>();
  public readonly state: StateStore;
  public readonly stage: string;
  public readonly phase: "up" | "destroy" | "read";

  static get current(): Scope {
    return Scope.storage.getStore();
  }

  async run<T>(fn: (scope: Scope) => Promise<T>): Promise<T> {
    return Scope.storage.run(this, () => fn(this));
  }
}
```

### Rust: Scope

```rust
// ewe_platform/backends/foundation_core/src/scope.rs

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

thread_local! {
    static SCOPE_STACK: RefCell<Vec<Rc<Scope>>> = RefCell::new(Vec::new());
}

pub struct ScopeOptions {
    pub stage: String,
    pub phase: Phase,
    pub state_store: Box<dyn StateStore>,
    pub quiet: bool,
    pub parent: Option<Rc<Scope>>,
}

pub struct Scope {
    pub name: String,
    pub stage: String,
    pub phase: Phase,
    pub state: Box<dyn StateStore>,
    pub resources: HashMap<String, ResourceMetadata>,
    pub parent: Option<Rc<Scope>>,
    pub children: HashMap<String, Rc<Scope>>,
    pub seq: usize,
    pub data: HashMap<String, serde_json::Value>,
}

impl Scope {
    pub fn new(options: ScopeOptions) -> Rc<Self> {
        Rc::new(Scope {
            name: options.stage.clone(),
            stage: options.stage,
            phase: options.phase,
            state: options.state_store,
            resources: HashMap::new(),
            parent: options.parent,
            children: HashMap::new(),
            seq: 0,
            data: HashMap::new(),
        })
    }

    pub fn current() -> Option<Rc<Scope>> {
        SCOPE_STACK.with(|stack| stack.borrow().last().cloned())
    }

    pub fn run<F, T>(scope: Rc<Scope>, f: F) -> T
    where
        F: FnOnce() -> T,
    {
        SCOPE_STACK.with(|stack| {
            stack.borrow_mut().push(scope);
            let result = f();
            stack.borrow_mut().pop();
            result
        })
    }

    pub fn fqn(&self, resource_id: &str) -> String {
        let mut parts: Vec<&str> = self.chain();
        parts.push(resource_id);
        parts.join("/")
    }

    pub fn chain(&self) -> Vec<&str> {
        let mut chain = Vec::new();
        let mut current = Some(self);

        while let Some(scope) = current {
            chain.push(scope.name.as_str());
            current = scope.parent.as_ref().map(|s| s.as_ref());
        }

        chain.reverse();
        chain
    }

    pub fn seq(&mut self) -> usize {
        let seq = self.seq;
        self.seq += 1;
        seq
    }
}
```

### TypeScript: State

```typescript
// alchemy/src/state.ts
export interface State {
  status: "creating" | "created" | "updating" | "updated" | "deleting" | "deleted";
  kind: string;
  id: string;
  fqn: string;
  seq: number;
  data: Record<string, any>;
  props: ResourceProps;
  oldProps?: ResourceProps;
  output: ResourceAttributes;
}

export interface StateStore {
  list(): Promise<string[]>;
  get(key: string): Promise<State | undefined>;
  set(key: string, value: State): Promise<void>;
  delete(key: string): Promise<void>;
}
```

### Rust: State

```rust
// ewe_platform/backends/foundation_core/src/state.rs

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LifecycleStatus {
    Creating,
    Created,
    Updating,
    Updated,
    Deleting,
    Deleted,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct State<P, O> {
    pub status: LifecycleStatus,
    pub kind: String,
    pub id: String,
    pub fqn: String,
    pub seq: usize,
    #[serde(default)]
    pub data: serde_json::Map<String, serde_json::Value>,
    pub props: P,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub old_props: Option<P>,
    pub output: O,
    #[serde(default)]
    pub version: usize,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

pub trait StateStore {
    fn init(&self) -> Result<(), StateError>;
    fn list(&self) -> Result<Vec<String>, StateError>;
    fn get(&self, key: &str) -> Result<Option<serde_json::Value>, StateError>;
    fn set(&self, key: &str, value: serde_json::Value) -> Result<(), StateError>;
    fn delete(&self, key: &str) -> Result<(), StateError>;
}

#[derive(Debug)]
pub enum StateError {
    NotFound,
    Io(std::io::Error),
    Serialization(serde_json::Error),
    Lock(String),
}
```

## FileSystem State Store

### TypeScript

```typescript
// alchemy/src/fs/file-system-state-store.ts
export class FileSystemStateStore implements StateStore {
  private basePath: string;

  async get(key: string): Promise<State | undefined> {
    const filePath = path.join(this.basePath, `${key}.json`);
    const content = await fs.readFile(filePath, "utf8");
    return deserializeState(content);
  }

  async set(key: string, value: State): Promise<void> {
    const filePath = path.join(this.basePath, `${key}.json`);
    await fs.writeFile(filePath, serializeState(value), "utf8");
  }
}
```

### Rust

```rust
// ewe_platform/backends/foundation_core/src/state/fs.rs

use std::path::{Path, PathBuf};
use std::fs::{self, File};
use std::io::{Read, Write};

pub struct FileSystemStateStore {
    base_path: PathBuf,
}

impl FileSystemStateStore {
    pub fn new(app_name: &str, stage: &str) -> Self {
        let base_path = PathBuf::from(".alchemy")
            .join(app_name)
            .join(stage);

        fs::create_dir_all(&base_path).ok();

        FileSystemStateStore { base_path }
    }

    fn state_path(&self, key: &str) -> PathBuf {
        self.base_path.join(format!("{}.json", key))
    }
}

impl StateStore for FileSystemStateStore {
    fn init(&self) -> Result<(), StateError> {
        fs::create_dir_all(&self.base_path)
            .map_err(StateError::Io)
    }

    fn list(&self) -> Result<Vec<String>, StateError> {
        let entries = fs::read_dir(&self.base_path)
            .map_err(StateError::Io)?;

        Ok(entries
            .filter_map(|e| e.ok())
            .filter_map(|e| {
                let path = e.path();
                if path.extension()?.to_str()? == "json" {
                    path.file_stem()?.to_str().map(String::from)
                } else {
                    None
                }
            })
            .collect())
    }

    fn get(&self, key: &str) -> Result<Option<serde_json::Value>, StateError> {
        let path = self.state_path(key);

        if !path.exists() {
            return Ok(None);
        }

        let mut file = File::open(&path).map_err(StateError::Io)?;
        let mut content = String::new();
        file.read_to_string(&mut content).map_err(StateError::Io)?;

        let value: serde_json::Value = serde_json::from_str(&content)
            .map_err(StateError::Serialization)?;

        Ok(Some(value))
    }

    fn set(&self, key: &str, value: serde_json::Value) -> Result<(), StateError> {
        let path = self.state_path(key);
        let content = serde_json::to_string_pretty(&value)
            .map_err(StateError::Serialization)?;

        let mut file = File::create(&path).map_err(StateError::Io)?;
        file.write_all(content.as_bytes()).map_err(StateError::Io)?;

        Ok(())
    }

    fn delete(&self, key: &str) -> Result<(), StateError> {
        let path = self.state_path(key);
        if path.exists() {
            fs::remove_file(&path).map_err(StateError::Io)?;
        }
        Ok(())
    }
}
```

## Apply Engine

### TypeScript

```typescript
// alchemy/src/apply.ts
async function _apply(resource, props, options) {
  const scope = resource[ResourceScope];
  let state = await scope.state.get(resource[ResourceID]);

  if (state === undefined) {
    // CREATE
    state = { status: "creating", ... };
  } else {
    // Check if update needed
    if (JSON.stringify(state.props) === JSON.stringify(props)) {
      return state.output;  // Skip
    }
    state.status = "updating";
  }

  const ctx = context({ scope, phase, props: state.oldProps, state });
  const output = await provider.handler.bind(ctx)(id, props);

  await scope.state.set(resource[ResourceID], {
    ...state,
    status: "created",
    output,
  });

  return output;
}
```

### Rust with Valtron

```rust
// ewe_platform/backends/foundation_core/src/apply.rs

use crate::valtron::{TaskIterator, TaskStatus, NoSpawner};
use crate::resource::{ResourceMetadata, PendingResource};
use crate::state::{State, StateStore, LifecycleStatus};

pub struct ApplyTask<P, O> {
    resource: PendingResource<P>,
    props: P,
    state_store: Box<dyn StateStore>,
    scope: Rc<Scope>,
    state: Option<State<P, O>>,
    phase: ApplyPhase,
    current_step: ApplyStep,
}

enum ApplyStep {
    LoadState,
    CheckChanges,
    ExecuteHandler,
    PersistState,
    Done,
}

enum ApplyPhase {
    Create,
    Update,
}

impl<P, O> TaskIterator for ApplyTask<P, O>
where
    P: serde::Serialize + serde::de::DeserializeOwned + Clone,
    O: serde::Serialize + serde::de::DeserializeOwned,
{
    type Ready = O;
    type Pending = ();
    type Spawner = NoSpawner;

    fn next(&mut self) -> Option<TaskStatus<Self::Ready, Self::Pending, Self::Spawner>> {
        match self.current_step {
            ApplyStep::LoadState => {
                // Load state from store (synchronous for now)
                let key = &self.resource.id;
                match self.state_store.get(key) {
                    Ok(Some(state_json)) => {
                        let state: State<P, O> = serde_json::from_value(state_json).ok()?;
                        self.state = Some(state);
                    }
                    Ok(None) => {
                        self.state = None;
                    }
                    Err(e) => {
                        return Some(TaskStatus::Error(format!("State load failed: {}", e)));
                    }
                }
                self.current_step = ApplyStep::CheckChanges;
                None  // Continue to next step
            }

            ApplyStep::CheckChanges => {
                // Determine phase
                if self.state.is_none() {
                    self.phase = ApplyPhase::Create;
                } else {
                    let state = self.state.as_ref().unwrap();
                    if state.props == self.props {
                        // No changes - return cached output
                        let output = state.output.clone();
                        return Some(TaskStatus::Ready(output));
                    }
                    self.phase = ApplyPhase::Update;
                }
                self.current_step = ApplyStep::ExecuteHandler;
                None
            }

            ApplyStep::ExecuteHandler => {
                // Get provider and execute
                let provider = get_provider(&self.resource.type_name)?;

                let ctx = Context {
                    phase: match self.phase {
                        ApplyPhase::Create => Phase::Create,
                        ApplyPhase::Update => Phase::Update,
                    },
                    id: self.resource.id.clone(),
                    fqn: self.scope.fqn(&self.resource.id),
                    scope: self.scope.clone(),
                    props: self.props.clone(),
                    state: self.state.clone(),
                };

                // Execute handler through Valtron
                let handler_task = provider(&self.resource.id, &self.props);

                // For now, assume handler completes synchronously
                // In real implementation, this would spawn a Valtron task
                match handler_task {
                    Ok(output) => {
                        self.output = Some(output);
                        self.current_step = ApplyStep::PersistState;
                        None
                    }
                    Err(e) => {
                        Some(TaskStatus::Error(format!("Handler failed: {}", e)))
                    }
                }
            }

            ApplyStep::PersistState => {
                // Persist state
                let output = self.output.take().unwrap();
                let state = State {
                    status: match self.phase {
                        ApplyPhase::Create => LifecycleStatus::Created,
                        ApplyPhase::Update => LifecycleStatus::Updated,
                    },
                    kind: self.resource.type_name.clone(),
                    id: self.resource.id.clone(),
                    fqn: self.scope.fqn(&self.resource.id),
                    seq: self.scope.seq(),
                    data: Default::default(),
                    props: self.props.clone(),
                    old_props: self.state.as_ref().and_then(|s| s.old_props.clone()),
                    output: output.clone(),
                    version: self.state.as_ref().map(|s| s.version).unwrap_or(0) + 1,
                    updated_at: chrono::Utc::now(),
                };

                let state_json = serde_json::to_value(&state).ok()?;
                self.state_store.set(&self.resource.id, state_json).ok()?;

                self.current_step = ApplyStep::Done;
                Some(TaskStatus::Ready(output))
            }

            ApplyStep::Done => {
                Some(TaskStatus::Done)
            }
        }
    }
}
```

## Valtron Executor Pattern

### No async/await - TaskIterator Instead

```rust
// ewe_platform/backends/foundation_core/src/valtron/mod.rs

/// Task execution status
pub enum TaskStatus<Ready, Pending, Spawner> {
    /// Task completed with result
    Ready(Ready),
    /// Task pending, may spawn subtasks
    Pending(Pending, Spawner),
    /// Task failed
    Error(String),
    /// Task completely done
    Done,
}

/// Task iterator trait (replaces async/await)
pub trait TaskIterator {
    type Ready;
    type Pending;
    type Spawner;

    fn next(&mut self) -> Option<TaskStatus<Self::Ready, Self::Pending, Self::Spawner>>;
}

/// No subtask spawning
pub struct NoSpawner;

/// Execute a task to completion
pub fn execute<T>(mut task: T) -> Result<T::Ready, String>
where
    T: TaskIterator<Spawner = NoSpawner>,
{
    loop {
        match task.next() {
            Some(TaskStatus::Ready(result)) => return Ok(result),
            Some(TaskStatus::Error(e)) => return Err(e),
            Some(TaskStatus::Done) => return Err("Task ended without result".into()),
            Some(TaskStatus::Pending(_, _)) => {
                return Err("Pending not supported without spawner".into());
            }
            None => continue,
        }
    }
}

/// Execute with streaming results
pub fn execute_stream<T, F>(mut task: T, mut on_item: F) -> Result<(), String>
where
    T: TaskIterator<Spawner = NoSpawner>,
    F: FnMut(T::Ready) -> Result<(), String>,
{
    loop {
        match task.next() {
            Some(TaskStatus::Ready(result)) => {
                on_item(result)?;
            }
            Some(TaskStatus::Error(e)) => return Err(e),
            Some(TaskStatus::Done) => return Ok(()),
            Some(TaskStatus::Pending(_, _)) => {
                return Err("Pending not supported".into());
            }
            None => continue,
        }
    }
}
```

## Cloudflare Provider in Rust

### Resource Definition

```rust
// ewe_platform/backends/foundation_core/src/providers/cloudflare/worker.rs

use crate::resource::{Resource, register_provider};
use crate::valtron::{TaskIterator, TaskStatus, NoSpawner};
use crate::state::State;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct WorkerProps {
    pub name: Option<String>,
    pub entrypoint: String,
    #[serde(default)]
    pub bindings: Vec<WorkerBinding>,
    pub compatibility_date: Option<String>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum WorkerBinding {
    D1 { name: String, database: String },
    KV { name: String, namespace: String },
    R2 { name: String, bucket: String },
    Secret { name: String, secret: String },
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct WorkerOutput {
    pub id: String,
    pub name: String,
    pub url: String,
}

pub struct WorkerCreateTask {
    props: WorkerProps,
    api: CloudflareApi,
    scope: Rc<Scope>,
    step: WorkerStep,
}

enum WorkerStep {
    Bundle,
    Upload,
    Bind,
    Done,
}

impl TaskIterator for WorkerCreateTask {
    type Ready = WorkerOutput;
    type Pending = ();
    type Spawner = NoSpawner;

    fn next(&mut self) -> Option<TaskStatus<Self::Ready, Self::Pending, Self::Spawner>> {
        match self.step {
            WorkerStep::Bundle => {
                // Bundle the worker code
                let bundle = bundle_worker(&self.props.entrypoint);
                self.bundle = Some(bundle);
                self.step = WorkerStep::Upload;
                None
            }

            WorkerStep::Upload => {
                let name = self.props.name.clone()
                    .unwrap_or_else(|| self.scope.create_physical_name("worker"));

                // Upload to Cloudflare
                let response = self.api.put_worker(&name, self.bundle.as_ref().unwrap());

                match response {
                    Ok(worker_info) => {
                        self.worker_info = Some(worker_info);
                        self.worker_name = Some(name);
                        self.step = WorkerStep::Bind;
                        None
                    }
                    Err(e) => {
                        Some(TaskStatus::Error(format!("Worker upload failed: {}", e)))
                    }
                }
            }

            WorkerStep::Bind => {
                // Create bindings
                let name = self.worker_name.as_ref().unwrap();
                for binding in &self.props.bindings {
                    self.api.put_worker_binding(name, binding);
                }

                let output = WorkerOutput {
                    id: name.clone(),
                    name: name.clone(),
                    url: format!("https://{}.workers.dev", name),
                };

                self.step = WorkerStep::Done;
                Some(TaskStatus::Ready(output))
            }

            WorkerStep::Done => {
                Some(TaskStatus::Done)
            }
        }
    }
}

// Register provider
register_provider("cloudflare::Worker", |id, props| {
    let props: WorkerProps = serde_json::from_value(props.clone())?;
    let api = CloudflareApi::from_env()?;
    let scope = Scope::current().unwrap();

    let task = WorkerCreateTask {
        props,
        api,
        scope,
        step: WorkerStep::Bundle,
        bundle: None,
        worker_info: None,
        worker_name: None,
    };

    execute(task)
});
```

## Secret Handling

### TypeScript

```typescript
// alchemy/src/secret.ts
export class Secret {
  constructor(public unencrypted: string) {}
}

export function encrypt(value: string, password: string): string {
  const key = await deriveKey(password);
  const nonce = crypto.randomBytes(nonceLength);
  const ciphertext = crypto_secretbox_easy(value, nonce, key);
  return base64(nonce + ciphertext);
}
```

### Rust

```rust
// ewe_platform/backends/foundation_core/src/secret.rs

use sodiumoxide::crypto::secretbox;
use sodiumoxide::crypto::hash::sha256;
use base64::{Engine, engine::general_purpose};

#[derive(Debug, Clone)]
pub struct Secret {
    pub unencrypted: String,
}

impl Secret {
    pub fn new(value: impl Into<String>) -> Self {
        Secret {
            unencrypted: value.into(),
        }
    }
}

/// Derive encryption key from password
fn derive_key(password: &str) -> secretbox::Key {
    let hash = sha256::hash(password.as_bytes());
    let mut key = secretbox::Key([0u8; 32]);
    key.0.copy_from_slice(&hash.0);
    key
}

/// Encrypt a secret
pub fn encrypt(value: &str, password: &str) -> Result<String, SecretError> {
    let key = derive_key(password);

    // Generate random nonce
    let nonce = secretbox::gen_nonce();

    // Encrypt
    let ciphertext = secretbox::seal(value.as_bytes(), &nonce, &key);

    // Combine nonce + ciphertext
    let mut combined = Vec::with_capacity(nonce.0.len() + ciphertext.len());
    combined.extend_from_slice(&nonce.0);
    combined.extend_from_slice(&ciphertext);

    Ok(general_purpose::STANDARD.encode(&combined))
}

/// Decrypt a secret
pub fn decrypt(encrypted: &str, password: &str) -> Result<String, SecretError> {
    let key = derive_key(password);

    // Decode base64
    let combined = general_purpose::STANDARD.decode(encrypted)?;

    // Split nonce and ciphertext
    let nonce_len = secretbox::NonceSize::BYTES;
    if combined.len() < nonce_len {
        return Err(SecretError::InvalidFormat);
    }

    let mut nonce_bytes = [0u8; 24];
    nonce_bytes.copy_from_slice(&combined[..nonce_len]);
    let nonce = secretbox::Nonce(nonce_bytes);

    let ciphertext = &combined[nonce_len..];

    // Decrypt
    let plaintext = secretbox::open(ciphertext, &nonce, &key)?;
    String::from_utf8(plaintext).map_err(|_| SecretError::InvalidUtf8)
}

#[derive(Debug)]
pub enum SecretError {
    InvalidFormat,
    InvalidUtf8,
    DecryptionFailed,
    Base64(base64::DecodeError),
}
```

## Summary

Key translations:

| TypeScript | Rust/Valtron |
|------------|--------------|
| `async/await` | `TaskIterator` + `execute()` |
| `AsyncLocalStorage` | `thread_local!` scope stack |
| `Symbol.for()` | Struct fields |
| `Promise<T>` | `TaskStatus<Ready, ...>` |
| `Map<string, T>` | `HashMap<String, T>` |
| `JSON.stringify` | `serde_json::to_string` |
| `crypto.subtle` | `sodiumoxide` |
| `fetch()` | `reqwest` (blocking) |

For `ewe_platform`:
- Use Valtron TaskIterator instead of async
- Use thread-local storage for scope context
- Use serde for serialization
- Use sodiumoxide for encryption
- Implement StateStore trait for persistence

## Next Steps

- [production-grade.md](./production-grade.md) - Multi-tenant deployment, scaling
- [05-valtron-integration.md](./05-valtron-integration.md) - Lambda deployment for alchemy controller
