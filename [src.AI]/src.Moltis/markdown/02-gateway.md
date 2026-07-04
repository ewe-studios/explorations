# Configuration: Loading, Validation, and Schema

The config crate handles all configuration loading, validation, environment substitution, and schema definitions. It's the backbone of Moltis configuration.

**Source:** `/home/darkvoid/Boxxed/@formulas/src.rust/src.AI/src.Moltis/moltis/crates/config/src/`

## Module Structure

```
crates/config/src/
├── lib.rs              ← Re-exports all public API
├── loader/
│   ├── config_io.rs    ← File I/O, discovery, save/load
│   └── workspace.rs    ← Workspace markdown loading
├── loader.rs           ← Loader entry point, port generation
├── env_subst.rs        ← ${ENV_VAR} substitution
├── validate.rs         ← Validation with diagnostics
├── schema.rs           ← All config structs (MoltisConfig, etc.)
├── defaults.rs         ← Default values
├── migrate.rs          ← Legacy config migration
├── provider_env.rs     ← Provider environment variables
├── template.rs         ← Template rendering
├── version.rs          ← Version constant
├── error.rs            ← Error types
├── agent_defs.rs       ← Agent definition types
└── container_mounts.rs ← Container mount configuration
```

## Config Discovery

**Source:** `loader.rs`

### Config File Names

```rust
// loader.rs:19
const CONFIG_FILENAMES: &[&str] = &["moltis.toml", "moltis.yaml", "moltis.yml", "moltis.json"];
```

Files are searched in order: `./` (project-local) then `~/.config/moltis/` (user-global).

### Directory Overrides

Three static overrides with `Mutex<Option<PathBuf>>`:

```rust
// loader.rs:22-29
static CONFIG_DIR_OVERRIDE: Mutex<Option<PathBuf>> = Mutex::new(None);
static DATA_DIR_OVERRIDE: Mutex<Option<PathBuf>> = Mutex::new(None);
static SHARE_DIR_OVERRIDE: Mutex<Option<PathBuf>> = Mutex::new(None);
```

Set via `set_config_dir()`, `set_data_dir()`, `set_share_dir()` — used by CLI args and tests.

### Share Directory Resolution

```rust
// loader.rs:90-100
pub fn share_dir() -> Option<PathBuf> {
    // 1. Programmatic override via set_share_dir()
    // 2. MOLTIS_SHARE_DIR env var
    // 3. /usr/share/moltis/ (Linux system packages) - only if exists
    // 4. data_dir()/share/ (~/.moltis/share/) - only if exists
    // 5. None (fall back to embedded assets)
}
```

### Random Port Generation

```rust
// loader.rs:11-17
fn generate_random_port() -> u16 {
    TcpListener::bind("127.0.0.1:0")
        .and_then(|listener| listener.local_addr())
        .map(|addr| addr.port())
        .unwrap_or(18789)  // Fallback default
}
```

Binds to port 0 to get an OS-assigned available port. Fallback is `18789`.

## MoltisConfig Schema

**Source:** `schema.rs`

The main config struct re-exports ~60 types:

```rust
// lib.rs:49-60
pub use schema::{
    AgentIdentity, AgentMemoryWriteMode, AgentPreset, AgentRuntimeLimitSource,
    AgentRuntimeLimits, AgentsConfig, AuthConfig, CacheRetention, CalDavAccountConfig,
    CalDavConfig, ChannelToolPolicyOverride, ChannelsConfig, ChatConfig, CodeIndexTomlConfig,
    CompactionConfig, CompactionMode, GeoLocation, GroupToolPolicy, HeartbeatConfig,
    HomeAssistantAccountConfig, HomeAssistantConfig, MemoryBackend, MemoryCitationsMode,
    MemoryProvider, MemoryScope, MemorySearchMergeStrategy, MemoryStyle, MessageQueueMode,
    ModePreset, ModesConfig, MoltisConfig, NgrokConfig, PresetMemoryConfig, PresetToolPolicy,
    PromptMemoryMode, ResolvedIdentity, SessionAccessPolicyConfig, SessionExportMode, Timezone,
    ToolMode, ToolPolicyConfig, ToolRegistryMode, UserProfile, UserProfileWriteMode,
    VoiceConfig, VoiceElevenLabsConfig, VoiceOpenAiConfig, VoiceSttConfig, VoiceSttProvider,
    VoiceTtsConfig, VoiceTtsProvider, VoiceWhisperConfig, VoiceWhisperLocalConfig, WireApi,
    parse_byte_size,
};
```

### Key Config Types

#### AuthConfig

```rust
pub struct AuthConfig {
    pub password: Option<String>,
    pub token: Option<String>,
    pub webauthn_enabled: bool,
    pub api_keys: Vec<ApiKeyConfig>,
}
```

#### ChannelsConfig

```rust
pub struct ChannelsConfig {
    pub offered: Vec<ChannelName>,  // Which channels to activate
    pub telegram: Option<TelegramConfig>,
    pub discord: Option<DiscordConfig>,
    pub slack: Option<SlackConfig>,
    pub signal: Option<SignalConfig>,
    pub whatsapp: Option<WhatsAppConfig>,
    pub matrix: Option<MatrixConfig>,
    pub msteams: Option<MSTeamsConfig>,
    pub nostr: Option<NostrConfig>,
    pub caldav: Option<CalDavConfig>,
}
```

#### ToolPolicyConfig

```rust
pub struct ToolPolicyConfig {
    pub registry_mode: ToolRegistryMode,  // Eager, Lazy
    pub agent_max_iterations: usize,
    pub agent_max_auto_continues: usize,
    pub agent_auto_continue_min_tool_calls: usize,
    pub tool_result_compaction_ratio: f64,
    pub preemptive_overflow_ratio: f64,
    pub compaction_min_iterations: usize,
    pub agent_loop_detector_window: usize,
    pub agent_loop_detector_strip_tools_on_second_fire: bool,
    pub max_tool_result_bytes: usize,
}
```

#### VoiceConfig

```rust
pub struct VoiceConfig {
    pub stt: VoiceSttConfig,   // Speech-to-text
    pub tts: VoiceTtsConfig,   // Text-to-speech
}

pub struct VoiceSttConfig {
    pub provider: VoiceSttProvider,  // whisper, whisper_local, openai
    pub whisper: VoiceWhisperConfig,
    pub whisper_local: VoiceWhisperLocalConfig,
}

pub struct VoiceTtsConfig {
    pub provider: VoiceTtsProvider,  // elevenlabs, openai
    pub elevenlabs: VoiceElevenLabsConfig,
    pub openai: VoiceOpenAiConfig,
}
```

### Memory Configuration

```rust
pub struct MemoryConfig {
    pub backend: MemoryBackend,           // sqlite, postgres
    pub provider: MemoryProvider,         // built-in, external
    pub style: MemoryStyle,               // citation, summary
    pub citations_mode: MemoryCitationsMode,
    pub search_merge_strategy: MemorySearchMergeStrategy,
    pub scope: MemoryScope,
}
```

### Compaction Configuration

```rust
pub struct CompactionConfig {
    pub mode: CompactionMode,    // summary, preserve
    pub retention: CacheRetention,
}
```

## Environment Substitution

**Source:** `env_subst.rs`

Supports `${ENV_VAR}` substitution in all string values:

```rust
// Any string value like "${OPENAI_API_KEY}" gets substituted
// from the environment at load time
```

## Validation

**Source:** `validate.rs`

```rust
pub enum Severity { Error, Warning, Info }

pub struct Diagnostic {
    pub severity: Severity,
    pub message: String,
    pub path: String,  // Config path like "channels.telegram"
}

pub struct ValidationResult {
    pub diagnostics: Vec<Diagnostic>,
}
```

## Provider Environment Variables

**Source:** `provider_env.rs`

```rust
pub fn generic_provider_env_source_for_provider(name: &str) -> &str;
pub fn generic_provider_api_key_from_env(name: &str) -> Option<String>;
pub fn normalize_provider_name(name: &str) -> String;
```

Handles provider-specific environment variable resolution.

## Migration

**Source:** `migrate.rs`

Handles legacy config format migration to current schema.

## Next

Continue to [04-channels](04-channels.md) for channel implementations.
