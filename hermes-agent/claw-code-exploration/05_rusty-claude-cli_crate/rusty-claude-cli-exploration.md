# Rusty-Claude-CLI Crate — Line-by-Line Exploration

**Crate:** `rusty-claude-cli`  
**Status:** Significantly enhanced in claw-code-latest (~6k → ~11k lines)  
**Purpose:** CLI binary entry point, REPL loop, argument parsing, slash command handling  
**Total Lines:** 
- claw-code: ~5,901 lines (6 files)
- claw-code-latest: ~11,035 lines (4 files)

---

## Table of Contents

1. [Module Overview](#module-overview)
2. [File Structure Comparison](#file-structure-comparison)
3. [Main Entry Point (Lines 1-175)](#main-entry-point)
4. [Argument Parsing (Lines 176-500+)](#argument-parsing)
5. [CLI Action Enum](#cli-action-enum)
6. [REPL Implementation](#repl-implementation)
7. [Slash Command Handling](#slash-command-handling)
8. [New Features in claw-code-latest](#new-features-in-claw-code-latest)
9. [Integration Points](#integration-points)

---

## Module Overview

The rusty-claude-cli crate is the **CLI binary** for claw-code. It provides:

- **Argument parsing** - CLI flags and subcommands
- **REPL loop** - Interactive chat interface
- **Slash command dispatch** - Handle `/command` inputs
- **Tool execution** - Call tools based on API responses
- **Session management** - Load/save conversation history
- **Output rendering** - Markdown streaming to terminal

---

## File Structure Comparison

### claw-code (6 files, ~5,901 lines)

| File | Lines | Purpose |
|------|-------|---------|
| `main.rs` | 3,897 | CLI entry point, REPL, argument parsing |
| `render.rs` | 796 | Terminal rendering, markdown streaming |
| `init.rs` | 433 | Repository initialization (`/init`) |
| `app.rs` | 398 | Application state (removed in latest) |
| `input.rs` | 269 | Input handling (enhanced in latest) |
| `args.rs` | 108 | Argument parsing (merged into main.rs) |

### claw-code-latest (4 files, ~11,035 lines)

| File | Lines | Purpose | Changes |
|------|-------|---------|---------|
| `main.rs` | 9,475 | CLI entry point, REPL | +5,578 lines, consolidated args/app |
| `render.rs` | 796 | Terminal rendering | Unchanged |
| `init.rs` | 434 | Repository initialization | +1 line |
| `input.rs` | 330 | Input handling | +61 lines |

**Key architectural change:** The latest version **consolidated** `app.rs` and `args.rs` into `main.rs`, growing from ~6k to ~11k lines (+87% growth).

---

## Main Entry Point (Lines 1-175)

### Standard Allowances (Lines 1-8)

```rust
#![allow(
    dead_code,
    unused_imports,
    unused_variables,
    clippy::unneeded_struct_pattern,
    clippy::unnecessary_wraps,
    clippy::unused_self
)]
```

Permissive lint allowances for CLI boilerplate.

### Module Declarations (Lines 9-11)

```rust
mod init;
mod input;
mod render;
```

Three internal modules:
- `init` - `/init` command for CLAUDE.md generation
- `input` - User input handling
- `render` - Terminal output rendering

### Imports (Lines 13-56)

Major imports from workspace crates:

| Crate | Imports |
|-------|---------|
| `api` | AnthropicClient, StreamEvent, ToolChoice, OAuth functions |
| `commands` | Slash command handling, skill/agent/MCP/plugin dispatch |
| `plugins` | PluginManager, PluginRegistry |
| `render` | TerminalRenderer, Spinner, MarkdownStreamState |
| `runtime` | Session, ToolExecutor, PermissionMode, McpServerManager |
| `tools` | GlobalToolRegistry, RuntimeToolDefinition |

### Main Function (Lines 98-112)

```rust
fn main() {
    if let Err(error) = run() {
        let message = error.to_string();
        if message.contains("`claw --help`") {
            eprintln!("error: {message}");
        } else {
            eprintln!(
                "error: {message}

Run `claw --help` for usage."
            );
        }
        std::process::exit(1);
    }
}
```

Standard CLI error handling pattern.

### Run Function (Lines 114-174)

```rust
fn run() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().skip(1).collect();
    match parse_args(&args)? {
        CliAction::DumpManifests { output_format } => dump_manifests(output_format)?,
        CliAction::BootstrapPlan { output_format } => print_bootstrap_plan(output_format)?,
        CliAction::Agents { args, output_format } => LiveCli::print_agents(...)?,
        CliAction::Mcp { args, output_format } => LiveCli::print_mcp(...)?,
        CliAction::Skills { args, output_format } => LiveCli::print_skills(...)?,
        CliAction::Plugins { action, target, output_format } => LiveCli::print_plugins(...)?,
        CliAction::PrintSystemPrompt { cwd, date, output_format } => print_system_prompt(...)?,
        CliAction::Version { output_format } => print_version(output_format)?,
        CliAction::ResumeSession { session_path, commands, output_format } => resume_session(...),
        CliAction::Status { model, permission_mode, output_format } => print_status_snapshot(...)?,
        CliAction::Sandbox { output_format } => print_sandbox_status_snapshot(output_format)?,
        CliAction::Prompt { prompt, model, output_format, allowed_tools, permission_mode } => {
            LiveCli::new(...)?.run_turn_with_output(&prompt, output_format)?
        },
        CliAction::Login { output_format } => run_login(output_format)?,
        CliAction::Logout { output_format } => run_logout(output_format)?,
        CliAction::Doctor { output_format } => run_doctor(output_format)?,
        CliAction::Init { output_format } => run_init(output_format)?,
        CliAction::Repl { model, allowed_tools, permission_mode } => run_repl(...)?,
        CliAction::HelpTopic(topic) => print_help_topic(topic),
        CliAction::Help { output_format } => print_help(output_format)?,
    }
    Ok(())
}
```

**Command dispatch table** - Maps CLI actions to handler functions.

---

## CLI Action Enum

### claw-code-latest (Lines 176-251)

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
enum CliAction {
    DumpManifests { output_format: CliOutputFormat },
    BootstrapPlan { output_format: CliOutputFormat },
    Agents { args: Option<String>, output_format: CliOutputFormat },
    Mcp { args: Option<String>, output_format: CliOutputFormat },
    Skills { args: Option<String>, output_format: CliOutputFormat },
    Plugins { action: Option<String>, target: Option<String>, output_format: CliOutputFormat },
    PrintSystemPrompt { cwd: PathBuf, date: String, output_format: CliOutputFormat },
    Version { output_format: CliOutputFormat },
    ResumeSession { session_path: PathBuf, commands: Vec<String>, output_format: CliOutputFormat },
    Status { model: String, permission_mode: PermissionMode, output_format: CliOutputFormat },
    Sandbox { output_format: CliOutputFormat },
    Prompt { prompt: String, model: String, output_format: CliOutputFormat, 
             allowed_tools: Option<AllowedToolSet>, permission_mode: PermissionMode },
    Login { output_format: CliOutputFormat },
    Logout { output_format: CliOutputFormat },
    Doctor { output_format: CliOutputFormat },
    Init { output_format: CliOutputFormat },
    Repl { model: String, allowed_tools: Option<AllowedToolSet>, permission_mode: PermissionMode },
    HelpTopic(LocalHelpTopic),
    Help { output_format: CliOutputFormat },
}
```

**New actions in claw-code-latest:**
- `Agents` - Agent catalog management
- `Mcp` - MCP server management
- `Skills` - Skill catalog management
- `Plugins` - Plugin lifecycle management
- `Status` - Session status snapshot
- `Sandbox` - Sandbox status
- `Doctor` - Diagnostic command
- `HelpTopic` - Scoped help

### CliOutputFormat (Lines 260-276)

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CliOutputFormat {
    Text,
    Json,
}
```

All commands support `--output-format json` for scripting.

---

## Argument Parsing

### parse_args() Function

The argument parser handles:

| Flag | Purpose |
|------|---------|
| `--help` / `-h` | Show help |
| `--version` / `-V` | Show version |
| `--model` | Set model (supports aliases: opus, sonnet, haiku) |
| `--output-format` | `text` or `json` |
| `--permission-mode` | `read-only`, `workspace-write`, `danger-full-access` |
| `--dangerously-skip-permissions` | Bypass permission prompts |
| `--allowedTools` / `--allowed-tools` | Restrict available tools |
| `--resume` | Load saved session |
| `-p` / `--print` | One-shot prompt mode |

### New Flags in claw-code-latest

| Flag | Purpose |
|------|---------|
| `--output-format` (enhanced) | Now available on all commands |
| `--resume` (enhanced) | Now supports `--resume=SESSION.json` syntax |

---

## REPL Implementation

The REPL loop (not fully shown due to size) handles:

1. **Input reading** - Multi-line input, slash commands
2. **Slash command parsing** - Dispatch to command handlers
3. **API request building** - Construct MessageRequest with tools
4. **Streaming response** - Render markdown as it arrives
5. **Tool execution** - Execute tools, send results back
6. **Session persistence** - Save conversation history

### LiveCli Structure

```rust
struct LiveCli {
    session: Session,
    tool_registry: GlobalToolRegistry,
    model: String,
    permission_mode: PermissionMode,
    // ... more fields
}
```

---

## Slash Command Handling

### Enhanced Command Set (claw-code-latest)

| Command | Status | New Features |
|---------|--------|--------------|
| `/help` | Enhanced | JSON output support |
| `/status` | **NEW** | Session status snapshot |
| `/compact` | Same | Session compaction |
| `/model` | Same | Model switching |
| `/permissions` | Same | Permission mode |
| `/clear` | Same | Clear session |
| `/cost` | Enhanced | JSON output |
| `/resume` | Same | Load session |
| `/config` | Same | Config inspection |
| `/memory` | Same | Memory files |
| `/init` | Same | Generate CLAUDE.md |
| `/diff` | Same | Git diff |
| `/version` | Enhanced | JSON output |
| `/bughunter` | Same | Code analysis |
| `/commit` | Same | Git commit |
| `/pr` | Same | PR creation |
| `/issue` | Same | Issue creation |
| `/ultraplan` | Same | Planning |
| `/teleport` | Same | File navigation |
| `/debug-tool-call` | Same | Debug tool calls |
| `/export` | Same | Export conversation |
| `/session` | Same | Session management |
| `/agents` | **NEW** | Agent catalog |
| `/mcp` | **NEW** | MCP servers |
| `/skills` | **NEW** | Skill catalog |
| `/plugins` | **NEW** | Plugin management |
| `/doctor` | **NEW** | Diagnostics |

---

## New Features in claw-code-latest

### 1. Plugin System Integration

```rust
use plugins::{PluginHooks, PluginManager, PluginManagerConfig, PluginRegistry};
```

Full plugin lifecycle management:
- `install` - Install plugins from path/git
- `enable` / `disable` - Toggle plugins
- `uninstall` - Remove plugins
- `update` - Update installed plugins

### 2. MCP Server Management

```rust
use runtime::McpServerManager;
```

MCP (Model Context Protocol) server handling:
- Server discovery
- Tool bridging
- Degraded startup reporting

### 3. Enhanced Output Formats

All commands now support `--output-format json`:

```rust
CliAction::Version { output_format } => print_version(output_format)?,
CliAction::Doctor { output_format } => run_doctor(output_format)?,
```

### 4. Diagnostic Commands

| Command | Purpose |
|---------|---------|
| `claw doctor` | System diagnostics |
| `claw status` | Session snapshot |
| `claw sandbox` | Sandbox status |

### 5. OAuth Flow Support

```rust
use api::{oauth_token_is_expired, generate_pkce_pair, generate_state, ...};
```

Full OAuth 2.0 with PKCE:
- `claw login` - OAuth flow
- `claw logout` - Clear credentials
- Automatic token refresh

---

## Integration Points

### Upstream Dependencies

| Crate | Usage |
|-------|-------|
| `api` | API client, streaming, OAuth |
| `commands` | Slash command parsing/handling |
| `tools` | Tool execution, registry |
| `runtime` | Session, permissions, MCP |
| `plugins` | Plugin management (latest only) |
| `render` | Terminal output |

### Key Constants

```rust
const DEFAULT_MODEL: &str = "claude-opus-4-6";
const DEFAULT_OAUTH_CALLBACK_PORT: u16 = 4545;
const VERSION: &str = env!("CARGO_PKG_VERSION");
const SESSION_REFERENCE_ALIASES: &[&str] = &["latest", "last", "recent"];
```

---

## Summary

The rusty-claude-cli crate grew from **~6k to ~11k lines** (+87%) with:

| Feature | claw-code | claw-code-latest |
|---------|-----------|------------------|
| Files | 6 | 4 (consolidated) |
| CLI actions | ~10 | ~19 |
| Slash commands | 22 | 27 |
| Plugin support | None | Full |
| MCP management | None | Full |
| OAuth support | Basic | Full PKCE flow |
| Output formats | Basic | JSON on all commands |
| Diagnostic commands | None | doctor, status, sandbox |

**Key architectural changes:**

1. **Consolidation** - Merged `app.rs` and `args.rs` into `main.rs`
2. **Plugin integration** - Full plugin lifecycle commands
3. **MCP support** - Server management built-in
4. **Enhanced output** - JSON output on all commands
5. **OAuth flows** - Complete PKCE implementation
6. **Diagnostics** - System health commands
