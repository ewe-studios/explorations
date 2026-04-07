# Compat-Harness Crate — Line-by-Line Exploration

**Crate:** `compat-harness`  
**Status:** Identical in both claw-code and claw-code-latest  
**Purpose:** Extract command/tool manifests from upstream TypeScript source  
**Total Lines:** 362  
**Files:** `src/lib.rs` (single file crate)

---

## Table of Contents

1. [Module Overview](#module-overview)
2. [Path Resolution (Lines 9-48)](#path-resolution)
3. [Manifest Extraction (Lines 50-103)](#manifest-extraction)
4. [Command Extraction (Lines 105-152)](#command-extraction)
5. [Tool Extraction (Lines 154-184)](#tool-extraction)
6. [Bootstrap Plan Extraction (Lines 186-223)](#bootstrap-plan-extraction)
7. [Helper Functions (Lines 225-299)](#helper-functions)
8. [Unit Tests (Lines 301-362)](#unit-tests)
9. [Integration Points](#integration-points)

---

## Module Overview

The compat-harness crate provides **parity testing infrastructure** by extracting command and tool registries from the upstream TypeScript claw-code source. This enables:

- **Automated comparison** between upstream and claw-code implementations
- **Manifest validation** ensuring all upstream commands/tools are implemented
- **Bootstrap phase detection** for startup optimization tracking

The crate parses TypeScript source files and extracts structured metadata about:
- Available commands (builtin, internal-only, feature-gated)
- Available tools (base, conditional)
- Bootstrap optimization phases

---

## Path Resolution (Lines 9-48)

### UpstreamPaths Struct (Lines 9-11)

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UpstreamPaths {
    repo_root: PathBuf,
}
```

**Purpose:** Encapsulates paths to upstream TypeScript source files.

### Implementation

#### `from_repo_root()` (Lines 14-19)
```rust
#[must_use]
pub fn from_repo_root(repo_root: impl Into<PathBuf>) -> Self {
    Self {
        repo_root: repo_root.into(),
    }
}
```
Direct construction from known repo root.

#### `from_workspace_dir()` (Lines 21-32)
```rust
#[must_use]
pub fn from_workspace_dir(workspace_dir: impl AsRef<Path>) -> Self {
    let workspace_dir = workspace_dir
        .as_ref()
        .canonicalize()
        .unwrap_or_else(|_| workspace_dir.as_ref().to_path_buf());
    let primary_repo_root = workspace_dir
        .parent()
        .map_or_else(|| PathBuf::from(".."), Path::to_path_buf);
    let repo_root = resolve_upstream_repo_root(&primary_repo_root);
    Self { repo_root }
}
```

**Line-by-line:**
- Line 23-26: Canonicalize workspace path (or use as-is on error)
- Line 27-29: Get parent directory as potential repo root
- Line 30: Resolve actual upstream repo location
- Line 31: Construct paths struct

#### Path Accessors (Lines 34-47)

```rust
#[must_use]
pub fn commands_path(&self) -> PathBuf {
    self.repo_root.join("src/commands.ts")
}

#[must_use]
pub fn tools_path(&self) -> PathBuf {
    self.repo_root.join("src/tools.ts")
}

#[must_use]
pub fn cli_path(&self) -> PathBuf {
    self.repo_root.join("src/entrypoints/cli.tsx")
}
```

**Target files:**
| File | Purpose |
|------|---------|
| `src/commands.ts` | Command registry source |
| `src/tools.ts` | Tool registry source |
| `src/entrypoints/cli.tsx` | CLI bootstrap logic |

### `resolve_upstream_repo_root()` (Lines 57-63)

```rust
fn resolve_upstream_repo_root(primary_repo_root: &Path) -> PathBuf {
    let candidates = upstream_repo_candidates(primary_repo_root);
    candidates
        .into_iter()
        .find(|candidate| candidate.join("src/commands.ts").is_file())
        .unwrap_or_else(|| primary_repo_root.to_path_buf())
}
```

**Algorithm:**
1. Generate candidate paths
2. Find first candidate with `src/commands.ts`
3. Fall back to primary repo root if not found

### `upstream_repo_candidates()` (Lines 65-91)

```rust
fn upstream_repo_candidates(primary_repo_root: &Path) -> Vec<PathBuf> {
    let mut candidates = vec![primary_repo_root.to_path_buf()];

    if let Some(explicit) = std::env::var_os("CLAUDE_CODE_UPSTREAM") {
        candidates.push(PathBuf::from(explicit));
    }

    for ancestor in primary_repo_root.ancestors().take(4) {
        candidates.push(ancestor.join("claw-code"));
        candidates.push(ancestor.join("clawd-code"));
    }

    candidates.push(
        primary_repo_root
            .join("reference-source")
            .join("claw-code"),
    );
    candidates.push(primary_repo_root.join("vendor").join("claw-code"));

    let mut deduped = Vec::new();
    for candidate in candidates {
        if !deduped.iter().any(|seen: &PathBuf| seen == &candidate) {
            deduped.push(candidate);
        }
    }
    deduped
}
```

**Candidate sources (in order):**

1. **Line 66:** Primary repo root itself
2. **Line 68-70:** Explicit `CLAUDE_CODE_UPSTREAM` env var
3. **Line 72-75:** Ancestor directories (up to 4 levels):
   - `*/claw-code`
   - `*/clawd-code`
4. **Line 77-81:** `reference-source/claw-code` subdirectory
5. **Line 82:** `vendor/claw-code` subdirectory
6. **Line 84-90:** Deduplicate (preserve order, keep first occurrence)

**Example candidate list:**
```
/workspace/claw-code-latest
/workspace/claw-code
/workspace/clawd-code
/workspace/reference-source/claw-code
/workspace/vendor/claw-code
```

---

## Manifest Extraction (Lines 50-103)

### ExtractedManifest Struct (Lines 50-55)

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExtractedManifest {
    pub commands: CommandRegistry,
    pub tools: ToolRegistry,
    pub bootstrap: BootstrapPlan,
}
```

**Purpose:** Complete extracted metadata from upstream source.

### `extract_manifest()` (Lines 93-103)

```rust
pub fn extract_manifest(paths: &UpstreamPaths) -> std::io::Result<ExtractedManifest> {
    let commands_source = fs::read_to_string(paths.commands_path())?;
    let tools_source = fs::read_to_string(paths.tools_path())?;
    let cli_source = fs::read_to_string(paths.cli_path())?;

    Ok(ExtractedManifest {
        commands: extract_commands(&commands_source),
        tools: extract_tools(&tools_source),
        bootstrap: extract_bootstrap_plan(&cli_source),
    })
}
```

**Line-by-line:**
- Line 94-96: Read three TypeScript source files
- Line 98-102: Parse each file and construct manifest

**Error handling:** Returns `std::io::Error` if any file cannot be read.

---

## Command Extraction (Lines 105-152)

### `extract_commands()` (Lines 105-152)

```rust
#[must_use]
pub fn extract_commands(source: &str) -> CommandRegistry {
    let mut entries = Vec::new();
    let mut in_internal_block = false;

    for raw_line in source.lines() {
        let line = raw_line.trim();

        if line.starts_with("export const INTERNAL_ONLY_COMMANDS = [") {
            in_internal_block = true;
            continue;
        }

        if in_internal_block {
            if line.starts_with(']') {
                in_internal_block = false;
                continue;
            }
            if let Some(name) = first_identifier(line) {
                entries.push(CommandManifestEntry {
                    name,
                    source: CommandSource::InternalOnly,
                });
            }
            continue;
        }

        if line.starts_with("import ") {
            for imported in imported_symbols(line) {
                entries.push(CommandManifestEntry {
                    name: imported,
                    source: CommandSource::Builtin,
                });
            }
        }

        if line.contains("feature('") && line.contains("./commands/") {
            if let Some(name) = first_assignment_identifier(line) {
                entries.push(CommandManifestEntry {
                    name,
                    source: CommandSource::FeatureGated,
                });
            }
        }
    }

    dedupe_commands(entries)
}
```

**Algorithm breakdown:**

### State Machine

| State | Trigger | Action |
|-------|---------|--------|
| Normal | `INTERNAL_ONLY_COMMANDS = [` | Enter internal block |
| Internal | `]` | Exit internal block |
| Internal | identifier | Add as InternalOnly |

### Line Types Detected

1. **Internal-only block** (Lines 113-130):
   ```typescript
   export const INTERNAL_ONLY_COMMANDS = [
     'debug',
     'profile',
   ]
   ```
   - Detect start marker
   - Extract identifiers until closing `]`
   - Mark as `InternalOnly`

2. **Import statements** (Lines 132-139):
   ```typescript
   import { addDir, review } from './commands/add-dir.js';
   ```
   - Extract symbols from import
   - Mark as `Builtin`

3. **Feature-gated commands** (Lines 141-148):
   ```typescript
   const experimental = feature('experimental-commands')('./commands/experimental.js');
   ```
   - Detect `feature('` and `./commands/`
   - Extract variable name
   - Mark as `FeatureGated`

### Deduplication

```rust
fn dedupe_commands(entries: Vec<CommandManifestEntry>) -> CommandRegistry {
    let mut deduped = Vec::new();
    for entry in entries {
        let exists = deduped.iter().any(|seen: &CommandManifestEntry| {
            seen.name == entry.name && seen.source == entry.source
        });
        if !exists {
            deduped.push(entry);
        }
    }
    CommandRegistry::new(deduped)
}
```

Preserves first occurrence of each (name, source) pair.

---

## Tool Extraction (Lines 154-184)

### `extract_tools()` (Lines 154-184)

```rust
#[must_use]
pub fn extract_tools(source: &str) -> ToolRegistry {
    let mut entries = Vec::new();

    for raw_line in source.lines() {
        let line = raw_line.trim();
        if line.starts_with("import ") && line.contains("./tools/") {
            for imported in imported_symbols(line) {
                if imported.ends_with("Tool") {
                    entries.push(ToolManifestEntry {
                        name: imported,
                        source: ToolSource::Base,
                    });
                }
            }
        }

        if line.contains("feature('") && line.contains("Tool") {
            if let Some(name) = first_assignment_identifier(line) {
                if name.ends_with("Tool") || name.ends_with("Tools") {
                    entries.push(ToolManifestEntry {
                        name,
                        source: ToolSource::Conditional,
                    });
                }
            }
        }
    }

    dedupe_tools(entries)
}
```

### Line Types Detected

1. **Base tool imports** (Lines 160-169):
   ```typescript
   import { BashTool, ReadFileTool } from './tools/bash.js';
   ```
   - Must be import from `./tools/`
   - Symbol must end with `Tool`
   - Mark as `Base`

2. **Conditional tools** (Lines 171-180):
   ```typescript
   const experimentalTools = feature('experimental')('./tools/experimental.js');
   ```
   - Detect `feature('` and `Tool` in line
   - Variable name ends with `Tool` or `Tools`
   - Mark as `Conditional`

### Deduplication

```rust
fn dedupe_tools(entries: Vec<ToolManifestEntry>) -> ToolRegistry {
    let mut deduped = Vec::new();
    for entry in entries {
        let exists = deduped
            .iter()
            .any(|seen: &ToolManifestEntry| seen.name == entry.name && seen.source == entry.source);
        if !exists {
            deduped.push(entry);
        }
    }
    ToolRegistry::new(deduped)
}
```

---

## Bootstrap Plan Extraction (Lines 186-223)

### `extract_bootstrap_plan()` (Lines 186-223)

```rust
#[must_use]
pub fn extract_bootstrap_plan(source: &str) -> BootstrapPlan {
    let mut phases = vec![BootstrapPhase::CliEntry];

    if source.contains("--version") {
        phases.push(BootstrapPhase::FastPathVersion);
    }
    if source.contains("startupProfiler") {
        phases.push(BootstrapPhase::StartupProfiler);
    }
    if source.contains("--dump-system-prompt") {
        phases.push(BootstrapPhase::SystemPromptFastPath);
    }
    if source.contains("--claude-in-chrome-mcp") {
        phases.push(BootstrapPhase::ChromeMcpFastPath);
    }
    if source.contains("--daemon-worker") {
        phases.push(BootstrapPhase::DaemonWorkerFastPath);
    }
    if source.contains("remote-control") {
        phases.push(BootstrapPhase::BridgeFastPath);
    }
    if source.contains("args[0] === 'daemon'") {
        phases.push(BootstrapPhase::DaemonFastPath);
    }
    if source.contains("args[0] === 'ps'") || source.contains("args.includes('--bg')") {
        phases.push(BootstrapPhase::BackgroundSessionFastPath);
    }
    if source.contains("args[0] === 'new' || args[0] === 'list' || args[0] === 'reply'") {
        phases.push(BootstrapPhase::TemplateFastPath);
    }
    if source.contains("environment-runner") {
        phases.push(BootstrapPhase::EnvironmentRunnerFastPath);
    }
    phases.push(BootstrapPhase::MainRuntime);

    BootstrapPlan::from_phases(phases)
}
```

### Bootstrap Phases

| Phase | Detection Pattern | Purpose |
|-------|-------------------|---------|
| `CliEntry` | Always first | Entry point |
| `FastPathVersion` | `--version` | Quick version output |
| `StartupProfiler` | `startupProfiler` | Performance profiling |
| `SystemPromptFastPath` | `--dump-system-prompt` | Debug system prompt |
| `ChromeMcpFastPath` | `--claude-in-chrome-mcp` | Chrome MCP bypass |
| `DaemonWorkerFastPath` | `--daemon-worker` | Background worker |
| `BridgeFastPath` | `remote-control` | Remote control mode |
| `DaemonFastPath` | `args[0] === 'daemon'` | Daemon mode |
| `BackgroundSessionFastPath` | `args[0] === 'ps'` or `--bg` | Background session |
| `TemplateFastPath` | `args[0] === 'new'...` | Template shortcuts |
| `EnvironmentRunnerFastPath` | `environment-runner` | Environment execution |
| `MainRuntime` | Always last | Full runtime |

**Design:** The bootstrap plan represents the **startup decision tree** in the upstream CLI. Each fast-path is an early-exit optimization that bypasses the full runtime.

---

## Helper Functions (Lines 225-299)

### `imported_symbols()` (Lines 225-255)

```rust
fn imported_symbols(line: &str) -> Vec<String> {
    let Some(after_import) = line.strip_prefix("import ") else {
        return Vec::new();
    };

    let before_from = after_import
        .split(" from ")
        .next()
        .unwrap_or_default()
        .trim();
    if before_from.starts_with('{') {
        return before_from
            .trim_matches(|c| c == '{' || c == '}')
            .split(',')
            .filter_map(|part| {
                let trimmed = part.trim();
                if trimmed.is_empty() {
                    return None;
                }
                Some(trimmed.split_whitespace().next()?.to_string())
            })
            .collect();
    }

    let first = before_from.split(',').next().unwrap_or_default().trim();
    if first.is_empty() {
        Vec::new()
    } else {
        vec![first.to_string()]
    }
}
```

**Parses two import styles:**

1. **Named imports:**
   ```typescript
   import { Foo, Bar, Baz } from './module.js'
   ```
   - Strip `import ` prefix
   - Extract content between `{` and `}`
   - Split by comma
   - Take first whitespace token from each

2. **Default imports:**
   ```typescript
   import DefaultExport from './module.js'
   ```
   - Take first symbol before `from`

**Examples:**
```
Input:  "import { BashTool, ReadFileTool } from './tools/bash.js'"
Output: ["BashTool", "ReadFileTool"]

Input:  "import Commands from './commands/index.js'"
Output: ["Commands"]
```

### `first_assignment_identifier()` (Lines 257-261)

```rust
fn first_assignment_identifier(line: &str) -> Option<String> {
    let trimmed = line.trim_start();
    let candidate = trimmed.split('=').next()?.trim();
    first_identifier(candidate)
}
```

**Purpose:** Extract identifier being assigned to.

**Example:**
```
Input:  "const myVar = feature('x')('y')"
Output: Some("myVar")
```

### `first_identifier()` (Lines 263-273)

```rust
fn first_identifier(line: &str) -> Option<String> {
    let mut out = String::new();
    for ch in line.chars() {
        if ch.is_ascii_alphanumeric() || ch == '_' || ch == '-' {
            out.push(ch);
        } else if !out.is_empty() {
            break;
        }
    }
    (!out.is_empty()).then_some(out)
}
```

**Purpose:** Extract first valid identifier from a string.

**Valid characters:** `a-z`, `A-Z`, `0-9`, `_`, `-`

**Examples:**
```
Input:  "myVar"        → Some("myVar")
Input:  "my-var_name"  → Some("my-var_name")
Input:  "  spaced"     → Some("spaced")
Input:  ""             → None
```

---

## Unit Tests (Lines 301-362)

### Test Setup (Lines 305-314)

```rust
fn fixture_paths() -> UpstreamPaths {
    let workspace_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    UpstreamPaths::from_workspace_dir(workspace_dir)
}

fn has_upstream_fixture(paths: &UpstreamPaths) -> bool {
    paths.commands_path().is_file()
        && paths.tools_path().is_file()
        && paths.cli_path().is_file()
}
```

### Test 1: `extracts_non_empty_manifests_from_upstream_repo()` (Lines 316-326)

```rust
#[test]
fn extracts_non_empty_manifests_from_upstream_repo() {
    let paths = fixture_paths();
    if !has_upstream_fixture(&paths) {
        return;
    }
    let manifest = extract_manifest(&paths).expect("manifest should load");
    assert!(!manifest.commands.entries().is_empty());
    assert!(!manifest.tools.entries().is_empty());
    assert!(!manifest.bootstrap.phases().is_empty());
}
```

**Verifies:**
- Upstream files exist (or skips test)
- Commands extracted
- Tools extracted
- Bootstrap phases detected

### Test 2: `detects_known_upstream_command_symbols()` (Lines 328-344)

```rust
#[test]
fn detects_known_upstream_command_symbols() {
    let paths = fixture_paths();
    if !paths.commands_path().is_file() {
        return;
    }
    let commands = extract_commands(&fs::read_to_string(paths.commands_path()).expect("commands.ts"));
    let names: Vec<_> = commands
        .entries()
        .iter()
        .map(|entry| entry.name.as_str())
        .collect();
    assert!(names.contains(&"addDir"));
    assert!(names.contains(&"review"));
    assert!(!names.contains(&"INTERNAL_ONLY_COMMANDS"));
}
```

**Verifies:**
- Known commands detected (`addDir`, `review`)
- Array name not extracted as command

### Test 3: `detects_known_upstream_tool_symbols()` (Lines 346-360)

```rust
#[test]
fn detects_known_upstream_tool_symbols() {
    let paths = fixture_paths();
    if !paths.tools_path().is_file() {
        return;
    }
    let tools = extract_tools(&fs::read_to_string(paths.tools_path()).expect("tools.ts"));
    let names: Vec<_> = tools
        .entries()
        .iter()
        .map(|entry| entry.name.as_str())
        .collect();
    assert!(names.contains(&"AgentTool"));
    assert!(names.contains(&"BashTool"));
}
```

**Verifies:**
- Known tools detected (`AgentTool`, `BashTool`)

---

## Integration Points

### Upstream Dependencies

| Crate | Usage |
|-------|-------|
| `commands` | `CommandRegistry`, `CommandManifestEntry`, `CommandSource` |
| `tools` | `ToolRegistry`, `ToolManifestEntry`, `ToolSource` |
| `runtime` | `BootstrapPhase`, `BootstrapPlan` |

### Downstream Dependents

| Crate | How it uses compat-harness |
|-------|---------------------------|
| `rusty-claude-cli` | Parity validation at startup |
| Test harnesses | Verify claw-code implements all upstream features |

### Usage Pattern

```rust
// In test or validation harness
let paths = UpstreamPaths::from_workspace_dir(workspace);
let manifest = extract_manifest(&paths)?;

// Verify all upstream commands are implemented
for command in manifest.commands.entries() {
    assert!(
        claw_code_implements(&command.name),
        "Missing implementation for upstream command: {}",
        command.name
    );
}
```

---

## Summary

The compat-harness crate is a **specialized parsing module** that:

| Component | Lines | Purpose |
|-----------|-------|---------|
| Path resolution | 48 | Find upstream repo |
| Manifest extraction | 11 | Read TypeScript files |
| Command extraction | 48 | Parse commands.ts |
| Tool extraction | 31 | Parse tools.ts |
| Bootstrap detection | 38 | Parse cli.tsx |
| Helper functions | 75 | String parsing utilities |
| Tests | 62 | Validation against fixtures |

**Key design patterns:**

1. **Pattern-based extraction** - No TypeScript parser, just string matching
2. **Graceful degradation** - Tests skip if fixtures missing
3. **Source tracking** - Distinguish builtin/internal/gated commands
4. **Bootstrap modeling** - Detect optimization phases from source patterns

**Comparison: claw-code vs claw-code-latest**

The compat-harness crate is **identical** in both repositories. This stability makes sense because:
- The upstream TypeScript source format is stable
- The extraction logic is mechanical, not semantic
- Both claw-code versions target the same upstream
