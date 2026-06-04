---
title: Configuration Worker — Config Store, Adapters, TTL
---

# Configuration Worker — Config Store, Adapters, TTL

**The Configuration Worker provides a JSON configuration store with schema validation, TTL expiration, environment variable expansion, and pluggable adapters (filesystem, bridge).**

## Architecture

Source: `workers/configuration/` (2,693 LOC)

```mermaid
flowchart TB
    subgraph ConfigWorker["ConfigurationWorker"]
        S1["ConfigurationStore (in-memory cache)"]
        A1["Adapters"]
        A2["bridge.rs: engine bridge"]
        A3["fs.rs: filesystem"]
        T1["ConfigurationTriggers"]
        E1["External change watcher"]
    end

    subgraph Functions["Registered Functions"]
        F1["config::register"]
        F2["config::set"]
        F3["config::get"]
        F4["config::list"]
        F5["config::schema::view"]
        F6["config::schema::register"]
    end

    F1 --> S1
    F2 --> S1
    F3 --> S1
    F4 --> S1
    F5 --> S1
    F6 --> S1
    S1 --> A1
    A1 --> A2
    A1 --> A3
    S1 --> T1
    E1 --> S1
```

## ConfigurationStore

Source: `workers/configuration/store.rs`

Lazy-loaded in-memory cache with adapter-backed persistence:

| Feature | Implementation |
|---------|---------------|
| Cache | `HashMap<String, ConfigurationEntry>` |
| Persistence | Pluggable `ConfigurationAdapter` |
| Validation | JSON Schema via `jsonschema` crate |
| Env expansion | `${VAR:default}` syntax |
| TTL | Per-entry expiration in seconds |

### Environment Variable Expansion

Source: `store.rs:30-43`

```rust
pub fn expand_value(v: &Value) -> Value {
    match v {
        Value::String(s) => Value::String(EngineConfig::expand_env_vars(s)),
        Value::Array(items) => Value::Array(items.iter().map(expand_value).collect()),
        Value::Object(map) => { ... }  // recursive
        other => other.clone(),
    }
}
```

**Aha:** Environment variable expansion is applied recursively to all string leaves in the JSON value tree. This means nested config objects like `{"database": {"url": "${DB_URL:localhost}"}}` are automatically expanded on every `get` operation.

## Adapters

### Bridge Adapter

Source: `workers/configuration/adapters/bridge.rs`

Persists configuration via the engine bridge (external connections).

### Filesystem Adapter

Source: `workers/configuration/adapters/fs.rs`

Persists configuration to the local filesystem as JSON files.

## External Change Watcher

Source: `workers/configuration/trigger.rs`

Watches for external configuration changes and fans out events to registered triggers:

```rust
// ExternalChange events trigger notifications
pub enum ExternalChange {
    Registered { id: String, kind: RegisterKind },
    Set { id: String },
    Removed { id: String },
}
```

## Trigger Type

Source: `workers/configuration/trigger.rs`

The `configuration` trigger type fires when config entries change:

```rust
pub const TRIGGER_TYPE: &str = "configuration";
```

Subscribers receive `ConfigurationEventData` with the event type (registered, set, removed) and entry data.

## Configuration Lifecycle

```mermaid
flowchart TD
    A[config::register] --> B[validate schema]
    B --> C[prime cache from adapter]
    C --> D[store in memory]
    
    E[config::set] --> F[expand env vars]
    F --> G[validate against schema]
    G --> H[update cache + adapter]
    H --> I[fire trigger if subscribed]
    
    J[config::get] --> K{cache hit?}
    K -->|Yes| L[return cached]
    K -->|No| M[fetch from adapter]
```

## What's Next

- [02 — Engine Functions](02-engine-functions.md) — Built-in function registry
- [00 — Overview](00-overview.md) — Return to overview
- [03 — REST API](03-rest-api.md) — Hot-reloadable routes
