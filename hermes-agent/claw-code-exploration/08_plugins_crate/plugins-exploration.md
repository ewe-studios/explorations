# Plugins Crate — Line-by-Line Exploration

**Crate:** `plugins`  
**Status:** NEW in claw-code-latest (not present in original claw-code)  
**Purpose:** Complete plugin lifecycle management, hooks, and tool registration  
**Total Lines:** ~1,800+ (lib.rs: ~1,700 + hooks.rs: 499)  
**Files:** `src/lib.rs`, `src/hooks.rs`

---

## Table of Contents

1. [Module Overview](#module-overview)
2. [Constants and Module Structure (Lines 1-22)](#constants-and-module-structure)
3. [PluginKind Enum (Lines 23-50)](#pluginkin-enum)
4. [PluginMetadata (Lines 52-62)](#pluginmetadata)
5. [PluginHooks (Lines 64-96)](#pluginhooks)
6. [PluginLifecycle (Lines 98-111)](#pluginlifecycle)
7. [PluginManifest (Lines 113-254)](#pluginmanifest)
8. [PluginTool System (Lines 205-346)](#plugintool-system)
9. [Plugin Types and Traits (Lines 352-594)](#plugin-types-and-traits)
10. [RegisteredPlugin (Lines 596-650)](#registeredplugin)
11. [PluginRegistry (Lines 658-842)](#pluginregistry)
12. [PluginManager (Lines 844-1519)](#pluginmanager)
13. [Manifest Loading and Validation (Lines 1520-1800+)](#manifest-loading-and-validation)
14. [Hooks Module (hooks.rs)](#hooks-module)
15. [Integration Points](#integration-points)

---

## Module Overview

The plugins crate provides a **complete plugin architecture** for claw-code with:

- **Three plugin types:** Builtin, Bundled, External
- **Lifecycle management:** init/shutdown commands
- **Hook system:** PreToolUse, PostToolUse, PostToolUseFailure
- **Tool registration:** Plugins can expose tools to claw-code
- **Install/enable/disable/uninstall flows**
- **Manifest validation** with Claude Code compatibility checking

This is a **major architectural addition** in claw-code-latest, enabling extensibility without modifying core code.

---

## Constants and Module Structure (Lines 1-22)

```rust
mod hooks;

use std::collections::{BTreeMap, BTreeSet};
// ... imports

pub use hooks::{HookEvent, HookRunResult, HookRunner};

const EXTERNAL_MARKETPLACE: &str = "external";
const BUILTIN_MARKETPLACE: &str = "builtin";
const BUNDLED_MARKETPLACE: &str = "bundled";
const SETTINGS_FILE_NAME: &str = "settings.json";
const REGISTRY_FILE_NAME: &str = "installed.json";
const MANIFEST_FILE_NAME: &str = "plugin.json";
const MANIFEST_RELATIVE_PATH: &str = ".claude-plugin/plugin.json";
```

### Marketplace Constants

| Constant | Value | Purpose |
|----------|-------|---------|
| `EXTERNAL_MARKETPLACE` | `"external"` | User-installed plugins |
| `BUILTIN_MARKETPLACE` | `"builtin"` | Compiled-in plugins |
| `BUNDLED_MARKETPLACE` | `"bundled"` | Shipped with claw-code |

### File Constants

| Constant | Value | Purpose |
|----------|-------|---------|
| `SETTINGS_FILE_NAME` | `"settings.json"` | Enabled plugins config |
| `REGISTRY_FILE_NAME` | `"installed.json"` | Installed plugin registry |
| `MANIFEST_FILE_NAME` | `"plugin.json"` | Plugin manifest filename |
| `MANIFEST_RELATIVE_PATH` | `.claude-plugin/plugin.json` | Alternative manifest location |

---

## PluginKind Enum (Lines 23-50)

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PluginKind {
    Builtin,
    Bundled,
    External,
}
```

### Variants

| Variant | Description |
|---------|-------------|
| `Builtin` | Compiled into claw-code binary |
| `Bundled` | Shipped with claw-code, auto-synced |
| `External` | User-installed from local path or git |

### `marketplace()` (Lines 42-49)
```rust
fn marketplace(self) -> &'static str {
    match self {
        Self::Builtin => BUILTIN_MARKETPLACE,
        Self::Bundled => BUNDLED_MARKETPLACE,
        Self::External => EXTERNAL_MARKETPLACE,
    }
}
```
Returns the marketplace identifier for plugin ID generation.

---

## PluginMetadata (Lines 52-62)

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PluginMetadata {
    pub id: String,
    pub name: String,
    pub version: String,
    pub description: String,
    pub kind: PluginKind,
    pub source: String,
    pub default_enabled: bool,
    pub root: Option<PathBuf>,
}
```

**Fields:**

| Field | Type | Purpose |
|-------|------|---------|
| `id` | `String` | Unique identifier (marketplace + name) |
| `name` | `String` | Human-readable name |
| `version` | `String` | Semantic version |
| `description` | `String` | One-line description |
| `kind` | `PluginKind` | Builtin/Bundled/External |
| `source` | `String` | Install source (path or URL) |
| `default_enabled` | `bool` | Whether enabled by default |
| `root` | `Option<PathBuf>` | Plugin root directory |

---

## PluginHooks (Lines 64-96)

```rust
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct PluginHooks {
    #[serde(rename = "PreToolUse", default)]
    pub pre_tool_use: Vec<String>,
    #[serde(rename = "PostToolUse", default)]
    pub post_tool_use: Vec<String>,
    #[serde(rename = "PostToolUseFailure", default)]
    pub post_tool_use_failure: Vec<String>,
}
```

### Hook Types

| Hook | When Triggered |
|------|----------------|
| `PreToolUse` | Before any tool executes |
| `PostToolUse` | After tool succeeds |
| `PostToolUseFailure` | After tool fails |

### Methods

#### `is_empty()` (Lines 75-80)
```rust
pub fn is_empty(&self) -> bool {
    self.pre_tool_use.is_empty()
        && self.post_tool_use.is_empty()
        && self.post_tool_use_failure.is_empty()
}
```

#### `merged_with()` (Lines 82-95)
```rust
pub fn merged_with(&self, other: &Self) -> Self {
    let mut merged = self.clone();
    merged.pre_tool_use.extend(other.pre_tool_use.iter().cloned());
    merged.post_tool_use.extend(other.post_tool_use.iter().cloned());
    merged.post_tool_use_failure.extend(other.post_tool_use_failure.iter().cloned());
    merged
}
```
Combines hooks from multiple plugins (used for aggregation).

---

## PluginLifecycle (Lines 98-111)

```rust
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct PluginLifecycle {
    #[serde(rename = "Init", default)]
    pub init: Vec<String>,
    #[serde(rename = "Shutdown", default)]
    pub shutdown: Vec<String>,
}
```

### Lifecycle Commands

| Command | When Executed |
|---------|---------------|
| `Init` | When plugin is enabled/loaded |
| `Shutdown` | When plugin is disabled/unloaded |

### `is_empty()` (Lines 107-110)
```rust
pub fn is_empty(&self) -> bool {
    self.init.is_empty() && self.shutdown.is_empty()
}
```

---

## PluginManifest (Lines 113-254)

### PluginManifest Struct (Lines 113-129)

```rust
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PluginManifest {
    pub name: String,
    pub version: String,
    pub description: String,
    pub permissions: Vec<PluginPermission>,
    #[serde(rename = "defaultEnabled", default)]
    pub default_enabled: bool,
    #[serde(default)]
    pub hooks: PluginHooks,
    #[serde(default)]
    pub lifecycle: PluginLifecycle,
    #[serde(default)]
    pub tools: Vec<PluginToolManifest>,
    #[serde(default)]
    pub commands: Vec<PluginCommandManifest>,
}
```

### PluginPermission (Lines 131-163)

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PluginPermission {
    Read,
    Write,
    Execute,
}
```

**Permissions:**
| Permission | Capability |
|------------|------------|
| `Read` | Read files |
| `Write` | Write to workspace |
| `Execute` | Run commands |

### PluginToolManifest (Lines 165-175)

```rust
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PluginToolManifest {
    pub name: String,
    pub description: String,
    #[serde(rename = "inputSchema")]
    pub input_schema: Value,
    pub command: String,
    #[serde(default)]
    pub args: Vec<String>,
    pub required_permission: PluginToolPermission,
}
```

### PluginToolPermission (Lines 177-203)

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum PluginToolPermission {
    ReadOnly,
    WorkspaceWrite,
    DangerFullAccess,
}
```

**Permission levels:**
| Permission | Tool Capability |
|------------|-----------------|
| `ReadOnly` | Read-only operations |
| `WorkspaceWrite` | Write within workspace |
| `DangerFullAccess` | Unrestricted access |

### PluginToolDefinition (Lines 205-212)

```rust
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PluginToolDefinition {
    pub name: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(rename = "inputSchema")]
    pub input_schema: Value,
}
```
Public-facing tool definition (without command execution details).

### PluginCommandManifest (Lines 214-219)

```rust
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PluginCommandManifest {
    pub name: String,
    pub description: String,
    pub command: String,
}
```
Plugin-exposed slash commands.

### RawPluginManifest (Lines 221-238)

Internal struct for parsing, handles string-based permissions before validation.

### RawPluginToolManifest (Lines 240-254)

Similar to `PluginToolManifest` but with string-based permission parsing.

---

## PluginTool System (Lines 256-346)

### PluginTool Struct (Lines 256-265)

```rust
#[derive(Debug, Clone, PartialEq)]
pub struct PluginTool {
    plugin_id: String,
    plugin_name: String,
    definition: PluginToolDefinition,
    command: String,
    args: Vec<String>,
    required_permission: PluginToolPermission,
    root: Option<PathBuf>,
}
```

### `execute()` (Lines 304-345)

```rust
pub fn execute(&self, input: &Value) -> Result<String, PluginError> {
    let input_json = input.to_string();
    let mut process = Command::new(&self.command);
    process
        .args(&self.args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .env("CLAWD_PLUGIN_ID", &self.plugin_id)
        .env("CLAWD_PLUGIN_NAME", &self.plugin_name)
        .env("CLAWD_TOOL_NAME", &self.definition.name)
        .env("CLAWD_TOOL_INPUT", &input_json);
    if let Some(root) = &self.root {
        process
            .current_dir(root)
            .env("CLAWD_PLUGIN_ROOT", root.display().to_string());
    }

    let mut child = process.spawn()?;
    if let Some(stdin) = child.stdin.as_mut() {
        use std::io::Write as _;
        stdin.write_all(input_json.as_bytes())?;
    }

    let output = child.wait_with_output()?;
    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        Err(PluginError::CommandFailed(format!(
            "plugin tool `{}` from `{}` failed for `{}`: {}",
            self.definition.name,
            self.plugin_id,
            self.command,
            if stderr.is_empty() {
                format!("exit status {}", output.status)
            } else {
                stderr
            }
        )))
    }
}
```

**Execution flow:**

1. **Line 305-306:** Serialize input to JSON, create Command
2. **Line 307-320:** Configure process:
   - Set args, stdin/stdout/stderr
   - Set environment variables for plugin context
   - Set working directory if root provided
3. **Line 322-326:** Spawn process, write input to stdin
4. **Line 328-344:** Wait for output, return stdout or error

**Environment variables set:**
| Variable | Value |
|----------|-------|
| `CLAWD_PLUGIN_ID` | Plugin identifier |
| `CLAWD_PLUGIN_NAME` | Plugin name |
| `CLAWD_TOOL_NAME` | Tool name |
| `CLAWD_TOOL_INPUT` | JSON input |
| `CLAWD_PLUGIN_ROOT` | Plugin root directory |

### `default_tool_permission_label()` (Lines 348-350)
```rust
fn default_tool_permission_label() -> String {
    "danger-full-access".to_string()
}
```
Default permission if not specified (most permissive).

---

## Plugin Types and Traits (Lines 352-594)

### PluginInstallSource (Lines 352-357)

```rust
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum PluginInstallSource {
    LocalPath { path: PathBuf },
    GitUrl { url: String },
}
```

**Install sources:**
| Variant | Description |
|---------|-------------|
| `LocalPath` | Install from local directory |
| `GitUrl` | Clone and install from git |

### InstalledPluginRecord (Lines 359-371)

```rust
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InstalledPluginRecord {
    #[serde(default = "default_plugin_kind")]
    pub kind: PluginKind,
    pub id: String,
    pub name: String,
    pub version: String,
    pub description: String,
    pub install_path: PathBuf,
    pub source: PluginInstallSource,
    pub installed_at_unix_ms: u128,
    pub updated_at_unix_ms: u128,
}
```

Tracks installed plugins with timestamps and source info.

### InstalledPluginRegistry (Lines 373-377)

```rust
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct InstalledPluginRegistry {
    #[serde(default)]
    pub plugins: BTreeMap<String, InstalledPluginRecord>,
}
```
Persistent registry of installed plugins.

### Plugin Trait (Lines 407-415)

```rust
pub trait Plugin {
    fn metadata(&self) -> &PluginMetadata;
    fn hooks(&self) -> &PluginHooks;
    fn lifecycle(&self) -> &PluginLifecycle;
    fn tools(&self) -> &[PluginTool];
    fn validate(&self) -> Result<(), PluginError>;
    fn initialize(&self) -> Result<(), PluginError>;
    fn shutdown(&self) -> Result<(), PluginError>;
}
```

**Core plugin interface** - implemented by all plugin types.

### PluginDefinition Enum (Lines 417-422)

```rust
pub enum PluginDefinition {
    Builtin(BuiltinPlugin),
    Bundled(BundledPlugin),
    External(ExternalPlugin),
}
```

### Builtin/Bundled/External Plugin Structs

Each plugin type has the same structure:
```rust
pub struct BuiltinPlugin {
    metadata: PluginMetadata,
    hooks: PluginHooks,
    lifecycle: PluginLifecycle,
    tools: Vec<PluginTool>,
}
// Same for BundledPlugin and ExternalPlugin
```

### Trait Implementations

#### Plugin for BuiltinPlugin (Lines 424-452)
- `validate()`: Always OK (trusted)
- `initialize()`: Always OK (no setup needed)
- `shutdown()`: Always OK

#### Plugin for BundledPlugin (Lines 454-494)
- `validate()`: Validates hook, lifecycle, and tool paths
- `initialize()`: Runs init lifecycle commands
- `shutdown()`: Runs shutdown lifecycle commands

#### Plugin for ExternalPlugin (Lines 496-536)
Same as BundledPlugin (full validation and lifecycle).

#### Plugin for PluginDefinition (Lines 537-594)
Delegates to the underlying concrete type.

---

## RegisteredPlugin (Lines 596-650)

```rust
#[derive(Debug, Clone, PartialEq)]
pub struct RegisteredPlugin {
    definition: PluginDefinition,
    enabled: bool,
}
```

### Methods

| Method | Purpose |
|--------|---------|
| `new()` | Create with definition and enabled state |
| `metadata()` | Get plugin metadata |
| `hooks()` | Get plugin hooks |
| `tools()` | Get plugin tools |
| `is_enabled()` | Check if enabled |
| `validate()` | Validate plugin |
| `initialize()` | Initialize plugin |
| `shutdown()` | Shutdown plugin |
| `summary()` | Get PluginSummary |

---

## PluginRegistry (Lines 658-842)

### PluginRegistry Struct (Lines 758-761)

```rust
#[derive(Debug, Clone, Default, PartialEq)]
pub struct PluginRegistry {
    plugins: Vec<RegisteredPlugin>,
}
```

### Key Methods

#### `new()` (Lines 764-768)
```rust
pub fn new(mut plugins: Vec<RegisteredPlugin>) -> Self {
    plugins.sort_by(|left, right| left.metadata().id.cmp(&right.metadata().id));
    Self { plugins }
}
```
Sorts plugins by ID for deterministic ordering.

#### `aggregated_hooks()` (Lines 792-800)
```rust
pub fn aggregated_hooks(&self) -> Result<PluginHooks, PluginError> {
    self.plugins
        .iter()
        .filter(|plugin| plugin.is_enabled())
        .try_fold(PluginHooks::default(), |acc, plugin| {
            plugin.validate()?;
            Ok(acc.merged_with(plugin.hooks()))
        })
}
```
Merges hooks from all enabled plugins.

#### `aggregated_tools()` (Lines 802-821)
```rust
pub fn aggregated_tools(&self) -> Result<Vec<PluginTool>, PluginError> {
    let mut tools = Vec::new();
    let mut seen_names = BTreeMap::new();
    for plugin in self.plugins.iter().filter(|plugin| plugin.is_enabled()) {
        plugin.validate()?;
        for tool in plugin.tools() {
            if let Some(existing_plugin) =
                seen_names.insert(tool.definition().name.clone(), tool.plugin_id().to_string())
            {
                return Err(PluginError::InvalidManifest(format!(
                    "plugin tool `{}` is defined by both `{existing_plugin}` and `{}`",
                    tool.definition().name,
                    tool.plugin_id()
                )));
            }
            tools.push(tool.clone());
        }
    }
    Ok(tools)
}
```
**Key validation:** Detects duplicate tool names across plugins.

#### `initialize()` / `shutdown()` (Lines 823-841)
Initialize/shutdown all enabled plugins (shutdown in reverse order).

---

## PluginManager (Lines 844-1519)

### PluginManagerConfig (Lines 844-866)

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PluginManagerConfig {
    pub config_home: PathBuf,
    pub enabled_plugins: BTreeMap<String, bool>,
    pub external_dirs: Vec<PathBuf>,
    pub install_root: Option<PathBuf>,
    pub registry_path: Option<PathBuf>,
    pub bundled_root: Option<PathBuf>,
}
```

### PluginManager (Lines 868-871)

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PluginManager {
    config: PluginManagerConfig,
}
```

### Core Operations

#### `install()` (Lines 1115-1156)
1. Parse install source (local path or git URL)
2. Materialize source to temp directory
3. Load and validate manifest
4. Copy to install location
5. Update registry
6. Write enabled state

#### `enable()` / `disable()` (Lines 1158-1174)
Update enabled state in config and settings.json.

#### `uninstall()` (Lines 1176-1194)
1. Remove from registry
2. Delete install directory
3. Clean up enabled state
4. **Blocks uninstall of bundled plugins**

#### `update()` (Lines 1196-1232)
Similar to install but preserves record and updates version.

### Discovery Methods

#### `plugin_registry_report()` (Lines 1069-1083)
Discovers all plugins:
1. Sync bundled plugins
2. Load builtin plugins
3. Discover installed plugins
4. Discover external directory plugins
5. Build registry report with failures

#### `sync_bundled_plugins()` (Lines 1356-1438)
Ensures bundled plugins are installed/updated:
1. Scan bundled root
2. Compare versions with installed
3. Copy if newer or missing
4. Remove stale bundled plugins
5. Update registry

---

## Manifest Loading and Validation (Lines 1520-1800+)

### `builtin_plugins()` (Lines 1521-1538)
Returns hardcoded builtin plugins (currently just an example scaffold).

### `load_plugin_from_directory()` (Lines 1582-1608)
1. Find manifest (plugin.json or .claude-plugin/plugin.json)
2. Read and parse JSON
3. Check Claude Code compatibility
4. Validate and build manifest

### `detect_claude_code_manifest_contract_gaps()` (Lines 1610-1666)

**Critical function** that rejects Claude Code plugin manifest features that claw doesn't support:

```rust
fn detect_claude_code_manifest_contract_gaps(
    raw_manifest: &Value,
) -> Vec<PluginManifestValidationError> {
    let mut errors = Vec::new();

    for (field, detail) in [
        ("skills", "plugin manifest field `skills` uses the Claude Code plugin contract; `claw` does not load plugin-managed skills..."),
        ("mcpServers", "plugin manifest field `mcpServers` uses the Claude Code plugin contract; `claw` does not import MCP servers from plugin manifests."),
        ("agents", "plugin manifest field `agents` uses the Claude Code plugin contract; `claw` does not load plugin-managed agent markdown catalogs..."),
    ] {
        if root.contains_key(field) {
            errors.push(PluginManifestValidationError::UnsupportedManifestContract {
                detail: detail.to_string(),
            });
        }
    }

    // Check for Claude Code-style commands
    if root.get("commands").and_then(Value::as_array).is_some_and(|commands| {
        commands.iter().any(Value::is_string)
    }) {
        errors.push(PluginManifestValidationError::UnsupportedManifestContract {
            detail: "plugin manifest field `commands` uses Claude Code-style directory globs; `claw` slash dispatch is still built-in...".to_string(),
        });
    }

    // Check for unsupported hooks
    if let Some(hooks) = root.get("hooks").and_then(Value::as_object) {
        for hook_name in hooks.keys() {
            if !matches!(hook_name.as_str(), "PreToolUse" | "PostToolUse" | "PostToolUseFailure") {
                errors.push(PluginManifestValidationError::UnsupportedManifestContract {
                    detail: format!("plugin hook `{hook_name}` uses the Claude Code lifecycle contract..."),
                });
            }
        }
    }

    errors
}
```

**Unsupported Claude Code features:**
| Feature | Reason |
|---------|--------|
| `skills` | claw discovers skills from directories |
| `mcpServers` | claw manages MCP separately |
| `agents` | claw has built-in agent system |
| String-based `commands` | claw slash commands are built-in |
| Extra hooks | Only Pre/Post ToolUse hooks supported |

---

## Hooks Module (hooks.rs)

The hooks.rs file (499 lines) provides:

### HookEvent Enum
```rust
pub enum HookEvent {
    PreToolUse { tool_name: String, input: Value },
    PostToolUse { tool_name: String, input: Value, output: String },
    PostToolUseFailure { tool_name: String, input: Value, error: String },
}
```

### HookCommandOutcome
```rust
pub enum HookCommandOutcome {
    Allow,
    Deny { reason: String },
    Failed { exit_code: i32, stderr: String },
}
```

### HookRunner
Executes hook commands with:
- Environment variables (HOOK_EVENT, HOOK_TOOL_NAME, etc.)
- Timeout support
- Output capture

---

## Integration Points

### Upstream Dependencies
| Crate | Usage |
|-------|-------|
| `serde`, `serde_json` | Manifest parsing |
| `std::process` | Tool/hook command execution |

### Downstream Dependents
| Crate | How it uses plugins |
|-------|---------------------|
| `runtime` | Plugin tool execution, hook dispatch |
| `tools` | Plugin tool registry integration |
| `rusty-claude-cli` | Plugin management commands |

### File Locations
```
~/.claude/
├── plugins/
│   ├── installed/       # Installed plugin directories
│   └── installed.json   # Plugin registry
├── settings.json        # Enabled plugins config
└── bundle/              # Bundled plugins (if applicable)
```

---

## Summary

The plugins crate is a **comprehensive plugin architecture** with:

| Component | Lines | Purpose |
|-----------|-------|---------|
| PluginKind/Metadata | 50 | Plugin identification |
| PluginHooks | 33 | Hook definitions |
| PluginLifecycle | 14 | Init/shutdown |
| PluginManifest | 142 | Manifest structures |
| PluginTool | 91 | Tool execution |
| Plugin trait/types | 243 | Type system |
| PluginRegistry | 185 | Aggregation |
| PluginManager | 676 | Lifecycle management |
| Manifest loading | 300+ | Validation, loading |
| hooks.rs | 499 | Hook execution |

**Key design patterns:**

1. **Three-tier plugin model** - Builtin (trusted), Bundled (shipped), External (user)
2. **Manifest validation** - Rejects Claude Code incompatible plugins
3. **Hook aggregation** - Merges hooks from all enabled plugins
4. **Tool deduplication** - Error on duplicate tool names
5. **Lifecycle commands** - Init/shutdown per plugin
6. **Persistent registry** - JSON-based plugin tracking
7. **Graceful degradation** - Reports failures, continues loading

**Comparison: claw-code vs claw-code-latest**

The plugins crate is **entirely new** in claw-code-latest. This represents a major architectural divergence from the original claw-code, adding an extensibility layer that the original lacks.
