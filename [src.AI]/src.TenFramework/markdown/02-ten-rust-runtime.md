# TEN Rust Runtime — ten_rust Crate Deep Dive

The `ten_rust` crate provides the core Rust functionality for TEN Framework: graph validation, JSON schema management, telemetry, logging, package metadata, binary value serialization, and crypto utilities. It compiles to both a `staticlib` (for FFI use by the C runtime) and a `rlib` (for use by `ten_manager`).

Source: `core/src/ten_rust/Cargo.toml:21` — `crate-type = ["staticlib", "rlib"]`

## Module Overview

```mermaid
graph TD
    ten_rust["ten_rust crate"] --> graph["graph/\nGraph model, validation,\nsubgraph flattening,\nselectors, msg_conversion"]
    ten_rust --> schema["schema/\nJSON schema definitions,\nruntime interface, store"]
    ten_rust --> service_hub["service_hub/\nHTTP server, OpenTelemetry\ntelemetry, Prometheus metrics"]
    ten_rust --> log["log/\nStructured logging, OTLP,\nencryption, formatters,\nfile appender"]
    ten_rust --> pkg_info["pkg_info/\nPackage metadata, manifest,\nproperty, predefined_graphs"]
    ten_rust --> value_buffer["value_buffer/\nBinary value serialization\n(18 types, 0x010E magic)"]
    ten_rust --> json["json/\nJSON utilities"]
    ten_rust --> json_schema["json_schema/\nJSON Schema validation"]
    ten_rust --> crypto["crypto/\nAES-CTR encryption\nfor log files"]
    ten_rust --> utils["utils/\nFile system, path, URI utilities"]
    ten_rust --> constants["constants/\nError messages, constants"]
    ten_rust --> base_dir_pkg_info["base_dir_pkg_info/\nPer-app package cache"]
```

## Graph Module (`graph/`)

The graph module is the largest sub-module, providing the data model and validation for TEN's declarative graph architecture.

### Key Types

**`Graph`** — The top-level structure representing a graph of extensions.
Source: `core/src/ten_rust/src/graph/mod.rs:171-184`

```rust
pub struct Graph {
    pub nodes: Vec<GraphNode>,
    pub connections: Option<Vec<GraphConnection>>,
    pub exposed_messages: Option<Vec<GraphExposedMessage>>,
    pub exposed_properties: Option<Vec<GraphExposedProperty>>,
}
```

**`GraphNode`** — Tagged enum for the three node types.
Source: `core/src/ten_rust/src/graph/node/mod.rs:122-137`

```rust
#[serde(tag = "type", rename_all = "lowercase")]
pub enum GraphNode {
    Extension { content: ExtensionNode },
    Subgraph { content: SubgraphNode },
    Selector { content: SelectorNode },
}
```

**`GraphConnection`** — Defines message flows from a source location to destinations.
Source: `core/src/ten_rust/src/graph/connection/mod.rs:195-208`

```rust
pub struct GraphConnection {
    pub loc: GraphLoc,
    pub cmd: Option<Vec<GraphMessageFlow>>,
    pub data: Option<Vec<GraphMessageFlow>>,
    pub audio_frame: Option<Vec<GraphMessageFlow>>,
    pub video_frame: Option<Vec<GraphMessageFlow>>,
}
```

**`GraphMessageFlow`** — A named message with destinations and optional sources.
Source: `core/src/ten_rust/src/graph/connection/mod.rs:271-284`

```rust
pub struct GraphMessageFlow {
    pub name: Option<String>,
    pub names: Option<Vec<String>>,  // Bulk expansion
    pub dest: Vec<GraphDestination>,
    pub source: Vec<GraphSource>,
}
```

### Validation Pipeline

The `Graph::validate_and_complete()` method performs a multi-phase validation:

1. **App URI analysis** — Determines if the graph is single-app or multi-app
2. **Node validation** — Each node validates its `app` field rules
3. **Connection validation** — Each connection validates message flow consistency
4. **Exposed property validation** — Verifies referenced extensions exist

Source: `core/src/ten_rust/src/graph/mod.rs:298-338`

### Graph Flattening

The `flatten_graph()` method transforms declarative graphs into runtime-ready form:

Source: `core/src/ten_rust/src/graph/mod.rs:534-563`

```
expand_names → flatten_selectors → convert_reversed → flatten_subgraphs
```

### Static Checks

The `static_check()` method runs seven independent validations:

| Check | Function | What It Validates |
|-------|----------|-------------------|
| Extension uniqueness | `check_extension_uniqueness` | No duplicate extension names |
| Extension existence | `check_extension_existence` | All referenced extensions are defined |
| Connection extensions exist | `check_connection_extensions_exist` | All connection source/dest nodes exist |
| Subgraph references exist | `check_subgraph_references_exist` | All subgraph imports are valid |
| Extension uniqueness in connections | `check_extension_uniqueness_in_connections` | No duplicate destinations |
| Message names | `check_message_names` | Valid message name patterns |
| Message conversions | `check_msg_conversions` | Valid msg_conversion configurations |

Source: `core/src/ten_rust/src/graph/mod.rs:370-380`

### Graph Proxy Injection

The `inject_graph_proxy_from_exposed_messages()` method converts `exposed_messages` into a `ten:graph_proxy` extension node, enabling cross-graph message routing.

Source: `core/src/ten_rust/src/graph/mod.rs:582-832`

**Aha:** The graph_proxy injection is TEN's solution to the "external interface" problem. Instead of extensions hardcoding external message routes, the graph declares `exposed_messages` and TEN automatically creates a proxy node that bridges internal and external message flows. This keeps the graph declarative while supporting runtime composition.

## Schema Module (`schema/`)

The schema module manages JSON schema definitions for graph validation.

### Schema Store

Source: `core/src/ten_rust/src/schema/store.rs`

The `SchemaStore` holds JSON schemas for packages, enabling runtime validation of `property.json` files. It builds schemas from `manifest.json` API definitions.

### Runtime Interface

Source: `core/src/ten_rust/src/schema/runtime_interface.rs:1-199`

Defines the runtime interface schema used by the C runtime to validate extension configurations.

## Service Hub Module (`service_hub/`)

The service_hub provides an embedded HTTP server with OpenTelemetry telemetry.

### Architecture

Source: `core/src/ten_rust/src/service_hub/mod.rs:21-30`

```rust
pub struct ServiceHub {
    metrics_exporter: telemetry::MetricsExporter,
    server_thread_handle: Option<thread::JoinHandle<()>>,
    server_thread_shutdown_tx: Option<oneshot::Sender<()>>,
}
```

The service hub runs on a dedicated thread with graceful shutdown via oneshot channel. It binds to configurable endpoints for metrics and API access.

### Endpoint Binding Logic

Source: `core/src/ten_rust/src/service_hub/mod.rs:64-88`

```rust
fn determine_binding_addresses(
    telemetry_endpoint: &Option<String>,
    api_endpoint: &Option<String>,
) -> Vec<String>
```

When telemetry and API endpoints differ, the service hub uses actix-web guards to route requests to the correct handler based on port matching.

**Aha:** The dual-endpoint guard pattern is clever: when telemetry and API share the same address, a single actix-web app serves both; when they differ, two separate scope guards filter by port. This avoids running two HTTP servers while maintaining endpoint isolation.

### Telemetry Sub-module

Source: `core/src/ten_rust/src/service_hub/telemetry/`

| File | Purpose |
|------|---------|
| `mod.rs` | Telemetry module entry (73 lines) |
| `config.rs` | Telemetry configuration (246 lines) |
| `exporter.rs` | OpenTelemetry exporter setup (400 lines) |
| `metrics.rs` | Prometheus metrics registry (399 lines) |
| `http_endpoint.rs` | `/metrics` HTTP endpoint (53 lines) |

The telemetry pipeline supports three exporters:
- **Prometheus** — `/metrics` endpoint for scraping
- **OTLP** — Push to OpenTelemetry collectors (gRPC or HTTP)
- **Stdout** — Console output for debugging

## Log Module (`log/`)

A comprehensive logging system with encryption, OTLP export, and dynamic filtering.

Source: `core/src/ten_rust/src/log/mod.rs:1-579`

### Log Levels

Source: `core/src/ten_rust/src/log/mod.rs:43-63`

```rust
pub enum LogLevel {
    Invalid = 0,
    Debug = 1,
    Info = 2,
    Warn = 3,
    Error = 4,
}
```

### Emitter Types

Source: `core/src/ten_rust/src/log/mod.rs:169-175`

```rust
#[serde(tag = "type", content = "config")]
pub enum AdvancedLogEmitter {
    Console(ConsoleEmitterConfig),
    File(FileEmitterConfig),
    Otlp(OtlpEmitterConfig),
}
```

### Key Sub-modules

| File | Purpose | Lines |
|------|---------|-------|
| `mod.rs` | Log configuration, setup | 579 |
| `bindings.rs` | C FFI bindings | 377 |
| `encryption.rs` | AES-CTR log encryption | 151 |
| `decrypt.rs` | Log decryption utility | 54 |
| `file_appender.rs` | File-based log output | 134 |
| `dynamic_filter.rs` | Runtime log level filtering | 186 |
| `otel.rs` | OpenTelemetry log export | 408 |
| `reloadable.rs` | Hot-reloadable log config | 199 |
| `formatter/json.rs` | JSON log formatter | 346 |
| `formatter/plain.rs` | Plain text log formatter | 307 |

### OTLP Configuration

Source: `core/src/ten_rust/src/log/mod.rs:145-162`

```rust
pub enum OtlpProtocol { Grpc, Http }

pub struct OtlpEmitterConfig {
    pub endpoint: String,
    pub protocol: OtlpProtocol,       // default: Grpc
    pub headers: HashMap<String, String>,
    pub service_name: Option<String>,
}
```

**Aha:** The logging system supports AES-CTR encryption with dynamic key management. Log files can be encrypted at rest and decrypted on-demand via the `log/decrypt.rs` utility. This is designed for production environments where logs may contain sensitive conversational data.

## PkgInfo Module (`pkg_info/`)

Manages package metadata, manifest parsing, and property resolution.

### PkgInfo Structure

Source: `core/src/ten_rust/src/pkg_info/mod.rs:42-72`

```rust
pub struct PkgInfo {
    pub manifest: Manifest,
    pub property: Option<Property>,
    pub compatible_score: i32,
    pub is_installed: bool,
    pub url: String,
    pub hash: String,
    pub schema_store: Option<SchemaStore>,
    pub is_local_dependency: bool,
    pub local_dependency_path: Option<String>,
    pub local_dependency_base_dir: Option<String>,
}
```

### Manifest Structure

Source: `core/src/ten_rust/src/pkg_info/manifest/mod.rs:185-205`

```rust
pub struct Manifest {
    pub type_and_name: PkgTypeAndName,
    pub version: Version,
    pub description: Option<LocalizedField>,
    pub display_name: Option<LocalizedField>,
    pub readme: Option<LocalizedField>,
    pub dependencies: Option<Vec<ManifestDependency>>,
    pub dev_dependencies: Option<Vec<ManifestDependency>>,
    pub tags: Option<Vec<String>>,
    pub supports: Option<Vec<ManifestSupport>>,
    pub api: Option<ManifestApi>,
    pub package: Option<PackageConfig>,
    pub scripts: Option<HashMap<String, String>>,
    pub flattened_api: Arc<tokio::sync::RwLock<Option<ManifestApi>>>,
}
```

**Aha:** The `LocalizedField` type supports i18n descriptions via a `HashMap<String, LocaleContent>` where each locale can have either inline `content` or an `import_uri` for lazy loading. The serialization is async-aware — `serialize_with_resolved_content()` resolves URIs before serializing.

### Package Collection

Source: `core/src/ten_rust/src/pkg_info/mod.rs:233-305`

The `get_app_installed_pkgs()` function walks an app directory and collects all packages from `ten_packages/{extension,protocol,addon_loader,system}/` subdirectories.

## Value Buffer Module (`value_buffer/`)

Implements binary serialization for TEN's value types, bridging Rust and the C runtime.

Source: `core/src/ten_rust/src/value_buffer/mod.rs:1-479`

### Buffer Header

Source: `core/src/ten_rust/src/value_buffer/mod.rs:13-15`

```rust
const VALUE_BUFFER_MAGIC: u16 = 0x010E;
const VALUE_BUFFER_VERSION: u8 = 1;
const VALUE_BUFFER_HEADER_SIZE: usize = 8;
```

### Supported Types (18 total)

Source: `core/src/ten_rust/src/value_buffer/mod.rs:18-39`

| Type ID | Name | Rust Type |
|---------|------|-----------|
| 0 | Invalid | — |
| 1 | Bool | bool |
| 2 | Int8 | i8 |
| 3 | Int16 | i16 |
| 4 | Int32 | i32 |
| 5 | Int64 | i64 |
| 6 | Uint8 | u8 |
| 7 | Uint16 | u16 |
| 8 | Uint32 | u32 |
| 9 | Uint64 | u64 |
| 10 | Float32 | f32 |
| 11 | Float64 | f64 |
| 12 | String | String |
| 13 | Buf | Vec<u8> |
| 14 | Array | Vec<Value> |
| 15 | Object | HashMap<String, Value> |
| 16 | Ptr | *mut c_void |
| 17 | JsonString | String |

**Aha:** The `JsonString` type (ID 17) is a performance optimization. Instead of deserializing JSON into the `Object`/`Array` tree, the runtime can carry pre-serialized JSON strings through the message pipeline and only deserialize when needed. This avoids double-serialization overhead in pass-through extensions.

## Crypto Module (`crypto/`)

Provides AES-CTR encryption for log files.

Source: `core/src/ten_rust/src/crypto/`

| File | Purpose |
|------|---------|
| `mod.rs` | Module entry, key generation |
| `bindings.rs` | C FFI bindings |
| `algorithm/aes.rs` | AES-CTR implementation (73 lines) |

Uses the `aes` and `ctr` crates for CTR-mode encryption.

## JSON Schema Module (`json_schema/`)

Provides JSON Schema validation for manifest and property files.

Source: `core/src/ten_rust/src/json_schema/`

| File | Purpose | Lines |
|------|---------|-------|
| `mod.rs` | Schema validation entry (156 lines) |
| `bindings.rs` | C FFI bindings (105 lines) |
| `definition.rs` | Schema definitions (17 lines) |

Uses the `jsonschema` crate (v0.32) for validation.

## Build Configuration

Source: `core/src/ten_rust/Cargo.toml:85-94`

```toml
[features]
default = ["service-hub"]
service-hub = []
deprecated = ["bindgen"]

[profile.release]
lto = true
opt-level = 3
codegen-units = 1
panic = "abort"
```

The release profile uses full LTO, maximum optimization, single codegen unit, and abort-on-panic — all chosen for production performance over compile speed.

## Cross-References

- [01-architecture.md](01-architecture.md) — Overall architecture and dependency graph
- [03-tman-cli.md](03-tman-cli.md) — How ten_manager uses ten_rust
- [04-graph-system.md](04-graph-system.md) — Graph module in detail
- [07-message-protocols.md](07-message-protocols.md) — Message types and value_buffer

## Next

Continue to [03-tman-cli.md](03-tman-cli.md) for the TMAN CLI deep dive.
