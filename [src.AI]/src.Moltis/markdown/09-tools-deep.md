# macOS FFI Bridges, Tool Registration, and Sandbox Routing

Comprehensive source-level documentation of the Swift-Rust FFI bridges, gateway tool registration flow, sandbox router with multiple backends, and background sandbox tasks.

**Source:** `/home/darkvoid/Boxxed/@formulas/src.rust/src.AI/src.Moltis/moltis/`

---

## Swift-Rust FFI Bridges

**Source:** `crates/swift-bridge/src/`

### Module Structure

```
swift-bridge/src/
├── lib.rs              ← Module declarations
├── types.rs            ← Request/response structs, error types
├── state.rs            ← Global bridge state (tokio runtime, registry)
├── callbacks.rs        ← Log, session event, network audit callbacks
├── helpers.rs          ← Encoding, parsing, metrics/tracing
├── chat.rs             ← Chat logic (provider resolution, streaming)
├── ffi_core.rs         ← Core FFI exports (version, providers, httpd)
├── ffi_sessions.rs     ← Session FFI exports
├── ffi_config.rs       ← Config, identity, soul, memory, env var FFI
├── ffi_auth.rs         ← Authentication FFI exports
└── ffi_sandbox.rs      ← Sandbox FFI exports
```

### Callback Types

**Source:** `callbacks.rs`

```rust
// Log callback
pub(crate) type LogCallback = unsafe extern "C" fn(log_json: *const c_char);
pub(crate) static LOG_CALLBACK: OnceLock<LogCallback> = OnceLock::new();

// Session event callback
pub(crate) type SessionEventCallback = unsafe extern "C" fn(event_json: *const c_char);
pub(crate) static SESSION_EVENT_CALLBACK: OnceLock<SessionEventCallback> = OnceLock::new();

// Network audit callback
pub(crate) type NetworkAuditCallback = unsafe extern "C" fn(event_json: *const c_char);
pub(crate) static NETWORK_AUDIT_CALLBACK: OnceLock<NetworkAuditCallback> = OnceLock::new();
```

### Log Bridge

```rust
// callbacks.rs:22-58
struct BridgeLogEvent<'a> {
    level: &'a str,
    target: &'a str,
    message: &'a str,
    fields: Option<&'a HashMap<&'a str, String>>,
}

pub(crate) fn emit_log_with_fields(
    level: &str,
    target: &str,
    message: &str,
    fields: Option<&HashMap<&str, String>>,
) {
    if let Some(callback) = LOG_CALLBACK.get() {
        let event = BridgeLogEvent { level, target, message, fields };
        if let Ok(json) = serde_json::to_string(&event)
            && let Ok(c_str) = CString::new(json)
        {
            // SAFETY: c_str is valid NUL-terminated, callback copies before returning
            unsafe { callback(c_str.as_ptr()); }
        }
    }
}
```

### Session Event Bridge

```rust
// callbacks.rs:71-76
struct BridgeSessionEvent {
    kind: &'static str,
    #[serde(rename = "sessionKey")]
    session_key: String,
}

pub(crate) fn emit_session_event(event: &SessionEvent) {
    if let Some(callback) = SESSION_EVENT_CALLBACK.get() {
        let event = BridgeSessionEvent {
            kind: event.kind.as_str(),
            session_key: event.session_key.clone(),
        };
        if let Ok(json) = serde_json::to_string(&event)
            && let Ok(c_str) = CString::new(json)
        {
            unsafe { callback(c_str.as_ptr()); }
        }
    }
}
```

### Network Audit Bridge

```rust
// callbacks.rs
pub(crate) fn emit_network_audit(domain: &str, port: u16, protocol: &str, action: &str, source: &str) {
    if let Some(callback) = NETWORK_AUDIT_CALLBACK.get() {
        let event = BridgeNetworkAuditEvent {
            domain, port, protocol, action, source,
        };
        if let Ok(json) = serde_json::to_string(&event)
            && let Ok(c_str) = CString::new(json)
        {
            unsafe { callback(c_str.as_ptr()); }
        }
    }
}
```

### Swift Side: MoltisClient.swift

**Source:** `apps/macos/Sources/MoltisClient.swift`

#### Log Callback

```swift
// MoltisClient.swift:7-47
private struct BridgeLogPayload: Decodable {
    let level: String
    let target: String
    let message: String
    let fields: [String: String]?
}

private var globalLogStore: LogStore?

private func rustLogCallbackHandler(logJson: UnsafePointer<CChar>?) {
    guard let logJson else { return }
    let jsonString = String(cString: logJson)
    let data = Data(jsonString.utf8)

    guard let payload = try? logDecoder.decode(BridgeLogPayload.self, from: data)
    else { return }

    let level: LogLevel
    switch payload.level {
    case "TRACE": level = .trace
    case "DEBUG": level = .debug
    case "INFO": level = .info
    case "WARN": level = .warn
    case "ERROR": level = .error
    default: level = .debug
    }

    DispatchQueue.main.async {
        globalLogStore?.log(level, target: payload.target, message: payload.message, fields: payload.fields ?? [:])
    }
}
```

#### Session Event Callback

```swift
// MoltisClient.swift:51-75
struct BridgeSessionEventPayload: Decodable {
    let kind: String
    let sessionKey: String
}

private var globalChatStore: ChatStore?

private func rustSessionEventCallbackHandler(eventJson: UnsafePointer<CChar>?) {
    guard let eventJson else { return }
    let jsonString = String(cString: eventJson)
    let data = Data(jsonString.utf8)

    guard let payload = try? sessionEventDecoder.decode(BridgeSessionEventPayload.self, from: data)
    else { return }

    DispatchQueue.main.async {
        globalChatStore?.handleSessionEvent(payload)
    }
}
```

#### Network Audit Callback

```swift
// MoltisClient.swift:79-125
private struct BridgeNetworkAuditPayload: Decodable {
    let domain: String
    let port: UInt16
    let networkProtocol: String
    let action: String
    let source: String
    let method: String?
    let url: String?
}

private var globalNetworkAuditStore: NetworkAuditStore?

private func rustNetworkAuditCallbackHandler(eventJson: UnsafePointer<CChar>?) {
    guard let eventJson else { return }
    let jsonString = String(cString: eventJson)
    let data = Data(jsonString.utf8)

    guard let payload = try? networkAuditDecoder.decode(BridgeNetworkAuditPayload.self, from: data)
    else { return }

    DispatchQueue.main.async {
        let entry = NetworkAuditEntry(
            id: UUID(),
            timestamp: Date(),
            domain: payload.domain,
            port: payload.port,
            networkProtocol: payload.networkProtocol,
            action: payload.action,
            source: payload.source,
            method: payload.method,
            url: payload.url
        )
        globalNetworkAuditStore?.push(entry)
    }
}
```

#### Installation at Startup

**Source:** `apps/macos/Sources/MoltisApp.swift:25-44`

```swift
init() {
    let settings = AppSettings()
    let logStore = LogStore()
    let networkAuditStore = NetworkAuditStore()

    // Install Rust→Swift log bridge before any FFI calls
    MoltisClient.installLogCallback(logStore: logStore)

    let providerStore = ProviderStore(logStore: logStore)
    let chatStore = ChatStore(settings: settings, providerStore: providerStore, logStore: logStore)

    // Install Rust→Swift session event bridge (shares bus with gateway)
    MoltisClient.installSessionEventCallback(chatStore: chatStore)

    // Install Rust→Swift network audit bridge
    MoltisClient.installNetworkAuditCallback(store: networkAuditStore)

    _settings = StateObject(wrappedValue: settings)
    _chatStore = StateObject(wrappedValue: chatStore)
    // ...
}
```

---

## Tool Registration

**Source:** `crates/gateway/src/server/prepare_core/tool_registration.rs`

### Voice Call Tool Registration

```rust
// tool_registration.rs:1-16
#[cfg(feature = "telephony")]
pub(super) async fn register_voice_call_tool(
    tool_registry: &mut moltis_agents::tool_registry::ToolRegistry,
    state: &crate::state::GatewayState,
) {
    let webhook_base = state
        .config
        .server
        .effective_external_url()
        .unwrap_or_default();

    if let Some(ref tp) = state.services.telephony_plugin {
        let voice_tool = moltis_telephony::VoiceCallTool::new(webhook_base, Arc::clone(tp));
        tool_registry.register(Box::new(voice_tool));
    }
}
```

The voice call tool is registered only when:
1. `telephony` feature is enabled
2. A telephony plugin is configured in `state.services.telephony_plugin`
3. The tool uses the gateway's external URL as the webhook base

---

## Sandbox Router: Multiple Backends

**Source:** `crates/gateway/src/server/prepare_core/sandbox.rs`

### Router Construction

```rust
// sandbox.rs:30-71
pub(super) fn build_sandbox_router(
    sandbox_config: &SandboxConfig,
    container_prefix: &str,
    timezone: Option<&str>,
) -> moltis_tools::sandbox::SandboxRouter {
    let mut config = sandbox_config.clone();
    config.container_prefix = Some(container_prefix.to_string());
    config.timezone = timezone.map(ToOwned::to_owned);

    let mut router = moltis_tools::sandbox::SandboxRouter::new(config.clone());

    // Register additional remote backends that have credentials configured.
    // Env vars (VERCEL_TOKEN, DAYTONA_API_KEY) are resolved by config crate.
    for (name, has_creds) in [
        ("vercel", has_secret(&config.vercel_token)),
        ("daytona", has_secret(&config.daytona_api_key)),
    ] {
        if has_creds && router.backend_name() != name {
            let backend = moltis_tools::sandbox::router::select_backend_by_name(name, &config);
            if backend.backend_name() == name {
                router.register_backend(backend);
            }
        }
    }

    #[cfg(target_os = "linux")]
    {
        let name = "firecracker";
        let has_creds = moltis_tools::sandbox::firecracker_bin_available(
            config.firecracker_bin.as_deref().map(FsPath::new),
        );
        if has_creds && router.backend_name() != name {
            let backend = moltis_tools::sandbox::router::select_backend_by_name(name, &config);
            if backend.backend_name() == name {
                router.register_backend(backend);
            }
        }
    }

    router
}
```

### Available Sandbox Backends

| Backend | Platform | Credential | Detection |
|---------|----------|------------|-----------|
| **Docker** | Linux, macOS | — | Default |
| **Podman** | Linux, macOS | — | Config setting |
| **Apple Container** | macOS | — | macOS only |
| **Vercel** | Cloud | `VERCEL_TOKEN` | Secret not empty |
| **Daytona** | Cloud | `DAYTONA_API_KEY` | Secret not empty |
| **Firecracker** | Linux | Binary available | `firecracker_bin_available()` |
| **WASM** | Cross-platform | — | Config setting |

### Secret Detection

```rust
// sandbox.rs:23-27
fn has_secret(secret: &Option<Secret<String>>) -> bool {
    secret
        .as_ref()
        .is_some_and(|secret| !secret.expose_secret().is_empty())
}
```

---

## Background Sandbox Tasks

**Source:** `crates/gateway/src/server/prepare_core/sandbox.rs`

### Image Pre-Build

```rust
// sandbox.rs:75-120
pub(super) fn spawn_sandbox_background_tasks(
    sandbox_router: &Arc<SandboxRouter>,
    deferred_state: &Arc<DeferredState>,
) {
    // Background image pre-build.
    {
        let router = Arc::clone(sandbox_router);
        let backends = router.available_backend_instances();
        let default_backend_name = router.backend_name().to_string();
        let packages = router.config().packages.clone();
        let base_image = router.config().image
            .clone()
            .unwrap_or_else(|| moltis_tools::sandbox::DEFAULT_SANDBOX_IMAGE.to_string());

        if should_prebuild_sandbox_image(router.mode(), &packages) {
            let deferred_for_build = Arc::clone(deferred_state);
            sandbox_router.building_flag.store(true, Ordering::Relaxed);
            let build_router = Arc::clone(sandbox_router);

            tokio::spawn(async move {
                if let Some(state) = deferred_for_build.get() {
                    broadcast(state, "sandbox.image.build", json!({
                        "status": "starting",
                        "base_image": base_image,
                        "packages": packages,
                    })).await;

                    match build_router.prebuild_image(&base_image, &packages).await {
                        Ok(_) => {
                            broadcast(state, "sandbox.image.build", json!({
                                "status": "complete",
                                "base_image": base_image,
                            })).await;
                        },
                        Err(e) => {
                            broadcast(state, "sandbox.image.build", json!({
                                "status": "error",
                                "error": e.to_string(),
                            })).await;
                        },
                    }
                    sandbox_router.building_flag.store(false, Ordering::Relaxed);
                }
            });
        }
    }

    // Host provisioning (install packages on host for non-container backends).
    // Startup container garbage collection (remove leftover containers).
}
```

### Build Progress Broadcasting

The sandbox image build progress is broadcast to connected clients via WebSocket:

```json
{"type": "sandbox.image.build", "data": {"status": "starting", "base_image": "...", "packages": [...]}}
{"type": "sandbox.image.build", "data": {"status": "complete", "base_image": "..."}}
{"type": "sandbox.image.build", "data": {"status": "error", "error": "..."}}
```

---

## Provider Discovery and Ranking

**Source:** `crates/gateway/src/provider_setup.rs`

### Key Store

```rust
// provider_setup.rs
pub struct KeyStore {
    // Persisted API keys from previous runs
}

impl KeyStore {
    pub fn new() -> Self { ... }
}
```

### Config with Saved Keys

```rust
pub fn config_with_saved_keys(
    base_config: &ProviderConfig,
    key_store: &KeyStore,
    local_model_ids: &[String],
) -> EffectiveProviderConfig {
    // Merge base config with saved API keys
    // Local models are added if local-llm feature is enabled
}
```

### Auto-Detection

```rust
pub fn detect_auto_provider_sources_with_overrides(
    providers: &ProviderConfig,
    deploy_platform: Option<&str>,
    env_overrides: &EnvOverrides,
) -> Vec<DetectedProviderSource> {
    // Detects providers from environment variables:
    // - OPENAI_API_KEY → OpenAI
    // - ANTHROPIC_API_KEY → Anthropic
    // - GROQ_API_KEY → Groq
    // - GEMINI_API_KEY → Google
    // - OLLAMA_BASE_URL → Ollama
    // Hides local providers when deploy_platform is set (cloud deploy)
}
```

### Provider Has Explicit Settings

```rust
pub fn has_explicit_provider_settings(providers: &ProviderConfig) -> bool {
    // Returns true if user explicitly configured any provider
    // When true, auto-detection is skipped
}
```

---

## Next

Continue to [06-memory](06-memory.md) for the memory system.
