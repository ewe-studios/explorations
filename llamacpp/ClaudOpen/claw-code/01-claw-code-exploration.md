# Claw Code Architecture Exploration

A comprehensive deep-dive into the Claw Code codebase architecture, crate structure, and runtime design.

## Table of Contents

1. [Executive Summary](#executive-summary)
2. [Repository Overview](#repository-overview)
3. [Rust Workspace Architecture](#rust-workspace-architecture)
4. [Core Runtime Flow](#core-runtime-flow)
5. [TypeScript Source Analysis](#typescript-source-analysis)
6. [Component Deep Dives](#component-deep-dives)
7. [Testing Strategy](#testing-strategy)
8. [Asset Handling](#asset-handling)
9. [Configuration System](#configuration-system)

---

## Executive Summary

**Claw Code** is a dual-implementation AI agent harness with:

| Metric | Value |
|--------|-------|
| **Rust LOC** | ~20,000 lines |
| **Crates** | 6 in workspace |
| **Binary** | `claw` (from `rusty-claude-cli`) |
| **Default Model** | `claude-opus-4-6` |
| **Tools (Rust MVP)** | 15 built-in |
| **Slash Commands** | 22 registered |
| **Python Subsystems** | 25+ modules |

### Implementation Status

| Component | Rust | Python | TypeScript (source) |
|-----------|------|--------|---------------------|
| Core API Client | ✅ Complete | ⚠️ Partial | ✅ Reference |
| Tool System | ✅ MVP (15 tools) | ⚠️ Metadata only | ✅ Complete (40+) |
| REPL/TUI | ✅ Inline + streaming | ❌ N/A | ✅ Full TUI |
| Session Management | ✅ Complete | ✅ Complete | ✅ Complete |
| MCP Support | ✅ Stdio/SSE | ⚠️ Partial | ✅ Complete |
| Hooks | ⚠️ Config only | ❌ Not implemented | ✅ Runtime |
| Plugins | ❌ Missing | ❌ Not implemented | ✅ Complete |
| Skills | ⚠️ Local files only | ⚠️ Metadata only | ✅ Registry + bundled |

---

## Repository Overview

### Top-Level Structure

```
claw-code/
├── README.md                    # Project narrative, quickstart, community
├── PARITY.md                    # Feature gap analysis (TypeScript → Rust)
├── CLAUDE.md                    # Instructions for Claude Code itself
├── assets/                      # Branding and documentation images
│   ├── clawd-hero.jpeg         # Main hero image (238KB)
│   ├── star-history.png        # Star growth chart (319KB)
│   ├── wsj-feature.png         # Wall Street Journal feature (894KB)
│   ├── tweet-screenshot.png    # Viral tweet (831KB)
│   ├── instructkr.png          # Instruct.kr branding (4.9KB)
│   └── omx/                    # oh-my-codex workflow screenshots
├── src/                         # Python porting workspace
│   ├── main.py                 # CLI entrypoint (10K+ LOC)
│   ├── runtime.py              # Core runtime simulation
│   ├── commands.py             # Command metadata registry
│   ├── tools.py                # Tool metadata registry
│   ├── query_engine.py         # Query routing engine
│   ├── port_manifest.py        # Porting status tracking
│   ├── context.py              # Context management
│   ├── permissions.py          # Permission definitions
│   ├── session_store.py        # Session persistence
│   └── [25+ subsystems/]       # Modular components
├── tests/                       # Python verification
│   └── test_porting_workspace.py  # Parity tests
├── rust/                        # Rust workspace (production)
│   ├── Cargo.toml              # Workspace definition
│   ├── Cargo.lock              # Dependency lockfile
│   ├── README.md               # Rust-specific docs
│   ├── TUI-ENHANCEMENT-PLAN.md # Terminal UI roadmap
│   ├── .claude/                # Project config
│   ├── .omc/                   # oh-my-codex plans
│   └── crates/                 # Individual crates
└── .claude/                     # Root configuration
    ├── settings.json           # Shared settings
    └── settings.local.json     # Local overrides
```

### Key Documentation Files

| File | Purpose |
|------|---------|
| `README.md` | Project overview, backstory, quickstart |
| `PARITY.md` | Detailed feature gap analysis between TS and Rust |
| `TUI-ENHANCEMENT-PLAN.md` | 6-phase terminal UI enhancement roadmap |
| `CLAUDE.md` | Instructions for Claude Code working on this repo |

---

## Rust Workspace Architecture

### Workspace Definition (`rust/Cargo.toml`)

```toml
[workspace]
members = ["crates/*"]
resolver = "2"

[workspace.package]
version = "0.1.0"
edition = "2021"
license = "MIT"
publish = false

[workspace.lints.rust]
unsafe_code = "forbid"

[workspace.lints.clippy]
all = { level = "warn", priority = -1 }
pedantic = { level = "warn", priority = -1 }
module_name_repetitions = "allow"
missing_panics_doc = "allow"
missing_errors_doc = "allow"
```

### Crate Dependency Graph

```
                         rusty-claude-cli (binary)
                        /      |       |       \
                       /       |       |        \
                  commands   api   runtime    tools
                                 |        \     /
                                 |         \   /
                                 |        compat-harness
                              (shared types)
```

### Individual Crates

#### 1. `api` (`rust/crates/api/`)

**Purpose**: Anthropic API client with streaming support

**Files**:
- `src/client.rs` - HTTP client, OAuth handling, SSE streaming
- `src/error.rs` - API error types
- `src/sse.rs` - Server-Sent Events parser
- `src/types.rs` - Request/response type definitions
- `src/lib.rs` - Public exports

**Key Types**:
```rust
pub struct AnthropicClient { /* HTTP client with auth */ }
pub struct MessageRequest { /* API request structure */ }
pub struct MessageResponse { /* API response */ }
pub enum StreamEvent { /* SSE event types */ }
pub struct Usage { /* Token counts */ }
```

**Dependencies**:
```toml
reqwest = { version = "0.12", features = ["json", "rustls-tls"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
tokio = { version = "1", features = ["io-util", "macros", "net", "rt-multi-thread", "time"] }
```

---

#### 2. `commands` (`rust/crates/commands/`)

**Purpose**: Slash command registry and parsing

**Files**:
- `src/lib.rs` - Command specs, parsing, handling

**Key Components**:
```rust
// Command specification
pub struct SlashCommandSpec {
    pub name: &'static str,
    pub summary: &'static str,
    pub argument_hint: Option<&'static str>,
    pub resume_supported: bool,
}

// Parsed command enum
pub enum SlashCommand {
    Help,
    Status,
    Compact,
    Model { model: Option<String> },
    Permissions { mode: Option<String> },
    // ... 18 more variants
}

// Command handling result
pub struct SlashCommandResult {
    pub message: String,
    pub session: Session,
}
```

**Registered Commands** (22 total):
- Core: `/help`, `/status`, `/compact`, `/clear`, `/cost`, `/version`
- Configuration: `/model`, `/permissions`, `/config`, `/memory`
- Sessions: `/resume`, `/session`, `/export`
- Git: `/diff`
- Planning: `/bughunter`, `/commit`, `/pr`, `/issue`, `/ultraplan`
- Debugging: `/debug-tool-call`, `/teleport`

**Dependencies**:
```toml
runtime = { path = "../runtime" }  # For Session, CompactionConfig
```

---

#### 3. `compat-harness` (`rust/crates/compat-harness/`)

**Purpose**: Extract command/tool manifests from TypeScript source for parity checking

**Files**:
- `src/lib.rs` - Manifest extraction logic

**Key Functions**:
```rust
pub fn extract_manifest(paths: &UpstreamPaths) -> ExtractedManifest {
    // Parses TypeScript source files to extract:
    // - Command registry
    // - Tool registry
    // - Bootstrap plan
}

pub fn extract_commands(source: &str) -> CommandRegistry {
    // Parses src/commands.ts
}

pub fn extract_tools(source: &str) -> ToolRegistry {
    // Parses src/tools.ts
}
```

**Discovery Paths**:
```rust
pub struct UpstreamPaths {
    repo_root: PathBuf,
}
// Discovers from:
// - CLAUDE_CODE_UPSTREAM env var
// - Ancestor directories (claw-code/, clawd-code/)
// - reference-source/claw-code/
// - vendor/claw-code/
```

**Dependencies**:
```toml
commands = { path = "../commands" }
tools = { path = "../tools" }
runtime = { path = "../runtime" }
```

---

#### 4. `runtime` (`rust/crates/runtime/`)

**Purpose**: Core agentic loop, configuration, sessions, MCP, permissions

**Files** (20 modules, ~5,300 lines):
- `src/lib.rs` - Public exports
- `src/conversation.rs` - **The core agentic loop** (~800 lines)
- `src/config.rs` - Configuration loading and merging (~900 lines)
- `src/session.rs` - Session persistence and management
- `src/permissions.rs` - Permission system
- `src/hooks.rs` - Hook execution
- `src/mcp.rs` - MCP configuration
- `src/mcp_client.rs` - MCP client abstraction
- `src/mcp_stdio.rs` - MCP stdio transport (~1,500 lines)
- `src/oauth.rs` - OAuth authentication (~500 lines)
- `src/prompt.rs` - System prompt construction (~700 lines)
- `src/compact.rs` - Conversation compaction (~400 lines)
- `src/file_ops.rs` - File operations (~450 lines)
- `src/bash.rs` - Bash execution
- `src/usage.rs` - Token usage tracking
- `src/remote.rs` - Remote session support
- `src/sandbox.rs` - Sandbox configuration
- `src/json.rs` - JSON utilities
- `src/bootstrap.rs` - Bootstrap phases

**Key Types**:
```rust
// The main runtime struct
pub struct ConversationRuntime<C, T> {
    session: Session,
    api_client: C,
    tool_executor: T,
    permission_policy: PermissionPolicy,
    system_prompt: Vec<String>,
    max_iterations: usize,
    usage_tracker: UsageTracker,
    hook_runner: HookRunner,
}

// Configuration loader
pub struct ConfigLoader {
    cwd: PathBuf,
    config_home: PathBuf,
}

// Permission modes
pub enum PermissionMode {
    ReadOnly,
    WorkspaceWrite,
    DangerFullAccess,
}

// MCP server types
pub enum McpServerConfig {
    Stdio(McpStdioServerConfig),
    Sse(McpRemoteServerConfig),
    Http(McpRemoteServerConfig),
    Ws(McpWebSocketServerConfig),
    Sdk(McpSdkServerConfig),
    ClaudeAiProxy(McpClaudeAiProxyServerConfig),
}
```

**The Core Loop** (`conversation.rs`):
```rust
pub fn run_turn(
    &mut self,
    user_input: impl Into<String>,
    mut prompter: Option<&mut dyn PermissionPrompter>,
) -> Result<TurnSummary, RuntimeError> {
    // 1. Add user message to session
    self.session.messages.push(ConversationMessage::user_text(user_input));

    loop {
        // 2. Build API request with current context
        let request = ApiRequest {
            system_prompt: self.system_prompt.clone(),
            messages: self.session.messages.clone(),
        };

        // 3. Stream response from API
        let events = self.api_client.stream(request)?;

        // 4. Build assistant message from events
        let (assistant_message, usage) = build_assistant_message(events)?;

        // 5. Check for tool calls
        let pending_tool_uses = assistant_message.blocks
            .iter()
            .filter_map(|block| match block {
                ContentBlock::ToolUse { id, name, input } => Some((id, name, input)),
                _ => None,
            })
            .collect();

        // 6. Execute tools if present
        for (tool_use_id, tool_name, input) in pending_tool_uses {
            // 6a. Check permissions
            let permission_outcome = self.permission_policy.authorize(...);

            // 6b. Run pre-tool hooks
            let pre_hook_result = self.hook_runner.run_pre_tool_use(...);

            // 6c. Execute tool
            let output = self.tool_executor.execute(&tool_name, &input)?;

            // 6d. Run post-tool hooks
            let post_hook_result = self.hook_runner.run_post_tool_use(...);

            // 6e. Add result to session
            self.session.messages.push(tool_result_message);
        }

        // 7. Exit if no tool calls (conversation complete)
        if pending_tool_uses.is_empty() {
            break;
        }
    }

    // 8. Return summary
    Ok(TurnSummary { ... })
}
```

**Dependencies**:
```toml
sha2 = "0.10"       # Session hashing
glob = "0.3"        # File globbing
regex = "1"         # Pattern matching
serde = "1"         # Serialization
serde_json = "1"    # JSON handling
tokio = "1"         # Async runtime
walkdir = "2"       # Directory traversal
```

---

#### 5. `tools` (`rust/crates/tools/`)

**Purpose**: Built-in tool specifications and execution

**Files**:
- `src/lib.rs` - Tool specs and executor (~800 lines)

**MVP Tool Specs** (15 tools):
```rust
pub fn mvp_tool_specs() -> Vec<ToolSpec> {
    vec![
        ToolSpec {
            name: "bash",
            description: "Execute a shell command",
            input_schema: json!({ /* JSON schema */ }),
            required_permission: PermissionMode::DangerFullAccess,
        },
        ToolSpec {
            name: "read_file",
            description: "Read a text file",
            input_schema: json!({ /* JSON schema */ }),
            required_permission: PermissionMode::ReadOnly,
        },
        // ... 13 more tools
    ]
}
```

**Tool Execution**:
```rust
pub fn execute_tool(
    tool_name: &str,
    input: &str,
    cwd: &Path,
    session: &Session,
) -> Result<String, ToolError> {
    let input: Value = serde_json::from_str(input)?;

    match tool_name {
        "bash" => execute_bash(input, cwd),
        "read_file" => read_file(input, cwd),
        "write_file" => write_file(input, cwd),
        "edit_file" => edit_file(input, cwd),
        "glob_search" => glob_search(input, cwd),
        "grep_search" => grep_search(input, cwd),
        "WebFetch" => web_fetch(input),
        "WebSearch" => web_search(input),
        "TodoWrite" => todo_write(input, session),
        "Skill" => load_skill(input, cwd),
        "Agent" => spawn_agent(input, cwd),
        "ToolSearch" => search_tools(input),
        "NotebookEdit" => notebook_edit(input, cwd),
        "Sleep" => sleep(input),
        "REPL" => run_repl(input, cwd),
        _ => Err(ToolError::new(format!("Unknown tool: {tool_name}"))),
    }
}
```

**Dependencies**:
```toml
api = { path = "../api" }
runtime = { path = "../runtime" }
reqwest = { version = "0.12", features = ["blocking", "rustls-tls"] }
serde = "1"
serde_json = "1"
tokio = "1"
```

---

#### 6. `rusty-claude-cli` (`rust/crates/rusty-claude-cli/`)

**Purpose**: Main CLI binary with REPL, streaming, and rendering

**Files**:
- `src/main.rs` - **CLI entrypoint and REPL loop** (~3,159 lines)
- `src/input.rs` - Line editor with completion (~270 lines)
- `src/render.rs` - Markdown rendering with syntax highlighting (~640 lines)
- `src/init.rs` - Repository initialization

**CLI Architecture**:
```rust
// Main entrypoint
fn main() {
    if let Err(error) = run() {
        eprintln!("error: {error}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().skip(1).collect();
    match parse_args(&args)? {
        CliAction::Prompt { prompt, model, .. } => {
            LiveCli::new(model, true, ..)?.run_turn_with_output(&prompt, ..)?
        }
        CliAction::Repl { model, .. } => run_repl(model, ..)?,
        CliAction::Login => run_login()?,
        CliAction::Logout => run_logout()?,
        CliAction::Init => run_init()?,
        // ... more actions
    }
    Ok(())
}
```

**LiveCli Struct** (the main REPL state):
```rust
struct LiveCli {
    model: String,
    session: Session,
    client: AnthropicClient,
    tool_definitions: Vec<ToolDefinition>,
    permission_policy: PermissionPolicy,
    system_prompt: Vec<String>,
    config_loader: ConfigLoader,
    allowed_tools: Option<BTreeSet<String>>,
}

impl LiveCli {
    fn run_turn(&mut self, prompt: &str) -> Result<(), RuntimeError> {
        // Execute one conversation turn
    }

    fn handle_slash_command(&mut self, input: &str) -> Result<(), RuntimeError> {
        // Parse and execute slash commands
    }

    fn stream_response(&mut self, events: Vec<AssistantEvent>) -> Result<(), io::Error> {
        // Stream and render API response
    }
}
```

**Input Handling** (`input.rs`):
```rust
pub struct LineEditor {
    prompt: String,
    editor: Editor<SlashCommandHelper, DefaultHistory>,
}

impl LineEditor {
    pub fn read_line(&mut self) -> io::Result<ReadOutcome> {
        match self.editor.readline(&self.prompt) {
            Ok(line) => Ok(ReadOutcome::Submit(line)),
            Err(ReadlineError::Interrupted) => Ok(ReadOutcome::Cancel),
            Err(ReadlineError::Eof) => Ok(ReadOutcome::Exit),
            Err(error) => Err(io::Error::other(error)),
        }
    }
}

// Slash command completion
impl Completer for SlashCommandHelper {
    fn complete(&self, line: &str, pos: usize, _ctx: &Context<'_>)
        -> rustyline::Result<(usize, Vec<Self::Candidate>)>
    {
        let Some(prefix) = slash_command_prefix(line, pos) else {
            return Ok((0, Vec::new()));
        };

        let matches = self.completions
            .iter()
            .filter(|c| c.starts_with(prefix))
            .map(|c| Pair { display: c.clone(), replacement: c.clone() })
            .collect();

        Ok((0, matches))
    }
}
```

**Markdown Rendering** (`render.rs`):
```rust
pub struct TerminalRenderer {
    theme: ColorTheme,
    stream_state: MarkdownStreamState,
}

impl TerminalRenderer {
    pub fn render_markdown(&mut self, markdown: &str, out: &mut impl Write) -> io::Result<()> {
        let parser = Parser::new_ext(markdown, Options::all());

        for event in parser {
            match event {
                Event::Start(Tag::Heading { level, .. }) => {
                    self.state.heading_level = Some(level as u8);
                }
                Event::Text(text) => {
                    let styled = self.state.style_text(&text, &self.theme);
                    write!(out, "{styled}")?;
                }
                Event::Code(code) => {
                    write!(out, "{}", self.state.style_inline_code(&code, &self.theme))?;
                }
                // ... more event types
            }
        }
        Ok(())
    }
}

// Syntax highlighting with syntect
fn highlight_code_block(code: &str, lang: Option<&str>, theme: &Theme) -> String {
    let syntax = lang.and_then(|l| ss.find_syntax_by_token(l))
        .unwrap_or_else(|| ss.find_syntax_plain_text());

    let mut highlighter = HighlightLines::new(syntax, theme);
    let mut result = String::new();

    for line in LinesWithEndings::from(code) {
        let regions = highlighter.highlight_line(line, &ss).unwrap();
        result.push_str(&as_24_bit_terminal_escaped(&regions[..], false));
    }

    result
}
```

**Dependencies**:
```toml
api = { path = "../api" }
commands = { path = "../commands" }
compat-harness = { path = "../compat-harness" }
crossterm = "0.28"        # Terminal control
pulldown-cmark = "0.13"   # Markdown parsing
rustyline = "15"          # Line editing
runtime = { path = "../runtime" }
serde_json = "1"
syntect = "5"             # Syntax highlighting
tokio = "1"
tools = { path = "../tools" }
```

---

## Core Runtime Flow

### Startup Sequence

```
1. CLI Argument Parsing (main.rs:parse_args)
   ├── Parse flags (--model, --permission-mode, --allowedTools)
   ├── Parse subcommands (prompt, login, init, etc.)
   └── Resolve model aliases (opus → claude-opus-4-6)

2. Authentication Resolution (api/src/client.rs)
   ├── Check ANTHROPIC_API_KEY env var
   ├── Check stored OAuth token (~/.claude/oauth.json)
   └── Initialize AnthropicClient with auth

3. Configuration Loading (runtime/src/config.rs)
   ├── Load user config (~/.claude/settings.json)
   ├── Load project config (.claude.json)
   ├── Load local config (.claude/settings.local.json)
   └── Merge with precedence

4. System Prompt Construction (runtime/src/prompt.rs)
   ├── Load base system prompt
   ├── Discover CLAUDE.md files
   ├── Inject tool definitions
   └── Add project context

5. Session Initialization (runtime/src/session.rs)
   ├── Create new session or load existing
   └── Initialize usage tracker

6. REPL Loop (main.rs:run_repl)
   ├── Display banner
   ├── Read user input (rustyline)
   ├── Execute turn (conversation loop)
   ├── Stream and render response
   └── Handle slash commands
```

### Message Flow

```
User Input
    │
    ▼
┌─────────────────────────────────────────────┐
│  LineEditor (rustyline)                     │
│  - Tab completion for slash commands        │
│  - History navigation                       │
│  - Multi-line input (Ctrl+J, Shift+Enter)   │
└─────────────────────────────────────────────┘
    │
    ▼
┌─────────────────────────────────────────────┐
│  Slash Command Parser                       │
│  - Check if input starts with /             │
│  - Parse command and arguments              │
│  - Execute handler or fall through          │
└─────────────────────────────────────────────┘
    │
    ▼
┌─────────────────────────────────────────────┐
│  ConversationRuntime::run_turn              │
│  - Append user message to session           │
│  - Build API request with context           │
└─────────────────────────────────────────────┘
    │
    ▼
┌─────────────────────────────────────────────┐
│  AnthropicClient::stream                    │
│  - POST to /v1/messages                     │
│  - SSE stream parsing                       │
│  - Event emission (TextDelta, ToolUse, etc.)│
└─────────────────────────────────────────────┘
    │
    ▼
┌─────────────────────────────────────────────┐
│  LiveCli::stream_response                   │
│  - Render text deltas (markdown)            │
│  - Display spinner during generation        │
│  - Show usage summary                       │
└─────────────────────────────────────────────┘
    │
    ▼
┌─────────────────────────────────────────────┐
│  Tool Execution Loop                        │
│  - Extract tool calls from response         │
│  - Check permissions                        │
│  - Run pre-tool hooks                       │
│  - Execute tool                             │
│  - Run post-tool hooks                      │
│  - Append result to session                 │
│  - Loop back to API request                 │
└─────────────────────────────────────────────┘
    │
    ▼
┌─────────────────────────────────────────────┐
│  Persist Session                            │
│  - Save to ~/.claude/sessions/              │
│  - Update usage tracking                    │
└─────────────────────────────────────────────┘
```

---

## TypeScript Source Analysis

### Source Location

The TypeScript source is referenced via the `compat-harness` crate but is **not tracked in the repository**. Discovery paths include:

1. `CLAUDE_CODE_UPSTREAM` environment variable
2. Ancestor directories: `claw-code/`, `clawd-code/`
3. `reference-source/claw-code/`
4. `vendor/claw-code/`

### Known TypeScript Structure (from PARITY.md)

```
src/
├── commands.ts                    # Command registry
├── tools.ts                       # Tool definitions
├── entrypoints/
│   └── cli.tsx                    # CLI entrypoint
├── assistant/                     # Agentic orchestration
│   └── sessionHistory.ts
├── cli/                           # Transport layers
│   ├── structuredIO.ts
│   ├── remoteIO.ts
│   └── transports/
├── commands/                      # Command implementations
│   ├── agents/
│   ├── hooks/
│   ├── mcp/
│   ├── memory/
│   ├── plugin/
│   ├── skills/
│   └── tasks/
├── hooks/                         # Hook command surface
├── plugins/                       # Plugin system
│   ├── builtinPlugins.ts
│   └── bundled/
├── skills/                        # Skills registry
│   ├── loadSkillsDir.ts
│   ├── bundledSkills.ts
│   └── bundled/
├── services/                      # Service layer
│   ├── api/                       # API services
│   ├── oauth/                     # OAuth services
│   ├── mcp/                       # MCP services
│   ├── plugins/                   # Plugin services
│   ├── tools/                     # Tool orchestration
│   │   ├── StreamingToolExecutor.ts
│   │   ├── toolExecution.ts
│   │   ├── toolHooks.ts
│   │   └── toolOrchestration.ts
│   └── [analytics, prompt, voice, etc.]
└── tools/                         # Tool implementations
    ├── AgentTool.ts
    ├── AskUserQuestionTool.ts
    ├── BashTool.ts
    ├── ConfigTool.ts
    ├── FileReadTool.ts
    ├── FileWriteTool.ts
    ├── GlobTool.ts
    ├── GrepTool.ts
    ├── LSPTool.ts
    ├── MCPTool.ts
    ├── McpAuthTool.ts
    ├── RemoteTriggerTool.ts
    ├── ScheduleCronTool.ts
    ├── SkillTool.ts
    ├── Task*.ts
    ├── Team*.ts
    ├── TodoWriteTool.ts
    ├── WebFetchTool.ts
    └── WebSearchTool.ts
```

### Parity Gap Summary

| Area | TypeScript | Rust | Gap |
|------|------------|------|-----|
| **Tools** | 40+ tools | 15 MVP tools | Major |
| **Hooks** | Runtime execution | Config only | Major |
| **Plugins** | Full lifecycle | Missing | Complete |
| **Skills** | Registry + bundled | Local files only | Major |
| **CLI** | 20+ commands | 22 commands | Moderate |
| **Services** | Analytics, voice, team sync | Core API/MCP only | Major |
| **Assistant** | Hook-aware orchestration | Core loop only | Moderate |

---

## Component Deep Dives

### Permission System

**Location**: `rust/crates/runtime/src/permissions.rs`

```rust
pub enum PermissionMode {
    ReadOnly,           // Inspection only
    WorkspaceWrite,     // File modifications
    DangerFullAccess,   // Full system access
}

pub struct PermissionPolicy {
    mode: PermissionMode,
    allowed_tools: Option<BTreeSet<String>>,
}

pub trait PermissionPrompter {
    fn prompt(&mut self, request: &PermissionRequest) -> PermissionPromptDecision;
}

pub struct PermissionRequest {
    pub tool_name: String,
    pub input: String,
    pub required_mode: PermissionMode,
}

pub enum PermissionOutcome {
    Allow,
    Deny,
    Skip,  // Tool doesn't require permission
}

impl PermissionPolicy {
    pub fn authorize(
        &self,
        tool_name: &str,
        input: &str,
        mut prompter: Option<&mut dyn PermissionPrompter>,
    ) -> PermissionOutcome {
        // 1. Check if tool is in allowed list
        if let Some(allowed) = &self.allowed_tools {
            if !allowed.contains(tool_name) {
                return PermissionOutcome::Deny;
            }
        }

        // 2. Check permission mode
        let required_mode = get_required_permission(tool_name);
        if self.mode >= required_mode {
            return PermissionOutcome::Allow;
        }

        // 3. Prompt user if in interactive mode
        if let Some(prompter) = prompter.as_mut() {
            match prompter.prompt(&PermissionRequest { ... }) {
                PermissionPromptDecision::AllowOnce => PermissionOutcome::Allow,
                PermissionPromptDecision::Deny => PermissionOutcome::Deny,
            }
        } else {
            PermissionOutcome::Deny
        }
    }
}
```

---

### MCP System

**Location**: `rust/crates/runtime/src/mcp*.rs`

**Server Configuration Types**:
```rust
pub enum McpServerConfig {
    Stdio(McpStdioServerConfig),      // Process-based
    Sse(McpRemoteServerConfig),       // HTTP SSE
    Http(McpRemoteServerConfig),      // HTTP POST
    Ws(McpWebSocketServerConfig),     // WebSocket
    Sdk(McpSdkServerConfig),          // SDK-based
    ClaudeAiProxy(McpClaudeAiProxyServerConfig),  // Claude.ai proxy
}

pub struct McpStdioServerConfig {
    pub command: String,
    pub args: Vec<String>,
    pub env: BTreeMap<String, String>,
}

pub struct McpRemoteServerConfig {
    pub url: String,
    pub headers: BTreeMap<String, String>,
    pub oauth: Option<McpOAuthConfig>,
}
```

**MCP Client Transport**:
```rust
pub trait McpClientTransport {
    fn initialize(&mut self) -> Result<McpInitializeResult, McpError>;
    fn list_tools(&mut self) -> Result<McpListToolsResult, McpError>;
    fn call_tool(&mut self, params: McpToolCallParams) -> Result<McpToolCallResult, McpError>;
}

pub struct McpStdioTransport {
    process: Child,
    pending_requests: BTreeMap<JsonRpcId, oneshot::Sender<JsonRpcResponse>>,
}

impl McpClientTransport for McpStdioTransport {
    fn initialize(&mut self) -> Result<McpInitializeResult, McpError> {
        // Send JSON-RPC initialize request
        // Read response from stdout
        // Parse McpInitializeResult
    }

    fn call_tool(&mut self, params: McpToolCallParams) -> Result<McpToolCallResult, McpError> {
        // Send JSON-RPC tool call
        // Wait for response
        // Parse result
    }
}
```

**MCP Server Manager** (stdio lifecycle):
```rust
pub struct McpServerManager {
    servers: BTreeMap<String, ManagedMcpServer>,
}

impl McpServerManager {
    pub fn spawn_stdio_process(
        command: &str,
        args: &[String],
        env: &BTreeMap<String, String>,
    ) -> Result<McpStdioProcess, McpError> {
        // Spawn child process
        // Capture stdin/stdout
        // Initialize with JSON-RPC handshake
    }

    pub fn bootstrap_servers(
        &mut self,
        config: &McpConfigCollection,
    ) -> Result<(), McpError> {
        // Spawn all configured MCP servers
        // Collect tool lists
        // Register tools in global registry
    }
}
```

---

### OAuth Flow

**Location**: `rust/crates/runtime/src/oauth.rs`, `rust/crates/api/src/client.rs`

**OAuth Configuration**:
```rust
pub struct OAuthConfig {
    pub client_id: String,
    pub authorize_url: String,
    pub token_url: String,
    pub callback_port: Option<u16>,
    pub manual_redirect_url: Option<String>,
    pub scopes: Vec<String>,
}

pub struct OAuthTokenSet {
    pub access_token: String,
    pub refresh_token: String,
    pub expires_at: Option<u64>,
}
```

**PKCE Flow**:
```rust
pub struct PkceCodePair {
    pub code_verifier: String,
    pub code_challenge: String,
}

pub fn generate_pkce_pair() -> PkceCodePair {
    // Generate random code_verifier
    // SHA256 hash for code_challenge
}

pub fn generate_state() -> String {
    // Random state parameter for CSRF protection
}
```

**Login Flow** (`main.rs`):
```rust
fn run_login() -> Result<(), Box<dyn std::error::Error>> {
    // 1. Generate PKCE challenge
    let pkce = generate_pkce_pair();

    // 2. Generate state parameter
    let state = generate_state();

    // 3. Start loopback server on port 4545
    let listener = TcpListener::bind(format!("127.0.0.1:{DEFAULT_OAUTH_CALLBACK_PORT}"))?;

    // 4. Build authorization URL
    let auth_url = format!(
        "{}?client_id={}&redirect_uri={}&response_type=code&scope={}&state={}&code_challenge={}&code_challenge_method=S256",
        oauth_config.authorize_url,
        oauth_config.client_id,
        callback_uri,
        scopes,
        state,
        pkce.code_challenge
    );

    // 5. Open browser
    Command::new("xdg-open").arg(&auth_url).spawn()?;

    // 6. Wait for callback
    let (stream, _) = listener.accept()?;
    let request = read_http_request(stream)?;
    let callback = parse_oauth_callback_request_target(&request)?;

    // 7. Exchange code for tokens
    let tokens = exchange_code_for_tokens(callback.code, &pkce.code_verifier)?;

    // 8. Save tokens
    save_oauth_credentials(&tokens)?;

    Ok(())
}
```

---

### Session Persistence

**Location**: `rust/crates/runtime/src/session.rs`

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    pub version: u32,
    pub messages: Vec<ConversationMessage>,
    pub input_tokens: u32,
    pub output_tokens: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ConversationMessage {
    #[serde(rename = "user")]
    User { content: String },
    #[serde(rename = "assistant")]
    Assistant { blocks: Vec<ContentBlock> },
    #[serde(rename = "tool_result")]
    ToolResult {
        tool_use_id: String,
        tool_name: String,
        content: String,
        is_error: bool,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ContentBlock {
    #[serde(rename = "text")]
    Text { text: String },
    #[serde(rename = "tool_use")]
    ToolUse { id: String, name: String, input: String },
}

impl Session {
    pub fn save(&self, path: &Path) -> Result<(), SessionError> {
        let json = serde_json::to_string_pretty(self)?;
        fs::write(path, json)?;
        Ok(())
    }

    pub fn load(path: &Path) -> Result<Self, SessionError> {
        let json = fs::read_to_string(path)?;
        let session = serde_json::from_str(&json)?;
        Ok(session)
    }

    pub fn new() -> Self {
        Self {
            version: 1,
            messages: Vec::new(),
            input_tokens: 0,
            output_tokens: 0,
        }
    }
}
```

---

## Testing Strategy

### Rust Tests

**Location**: Throughout crates, primarily in `src/lib.rs` files

**Test Organization**:
```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_slash_commands() {
        assert_eq!(SlashCommand::parse("/help"), Some(SlashCommand::Help));
        assert_eq!(SlashCommand::parse("/status"), Some(SlashCommand::Status));
    }

    #[test]
    fn compacts_sessions() {
        let session = Session {
            messages: vec![
                ConversationMessage::user_text("a".repeat(200)),
                ConversationMessage::assistant(vec![ContentBlock::Text { text: "b".repeat(200) }]),
            ],
        };

        let result = handle_slash_command("/compact", &session, CompactionConfig::default());
        assert!(result.unwrap().message.contains("Compacted"));
    }

    #[test]
    fn completes_slash_commands() {
        let helper = SlashCommandHelper::new(vec!["/help".to_string(), "/status".to_string()]);
        let history = DefaultHistory::new();
        let ctx = Context::new(&history);

        let (_, matches) = helper.complete("/he", 3, &ctx).unwrap();
        assert_eq!(matches.len(), 1);  // Only /help matches
    }
}
```

**Running Tests**:
```bash
# Full workspace
cargo test --workspace

# Specific crate
cargo test -p runtime
cargo test -p commands
cargo test -p tools

# With output
cargo test --workspace -- --nocapture

# Specific test
cargo test -p commands parses_slash_commands
```

### Python Tests

**Location**: `tests/test_porting_workspace.py`

```python
import unittest

class TestPortManifest(unittest.TestCase):
    def test_manifest_includes_subsystems(self):
        manifest = build_port_manifest()
        self.assertGreater(len(manifest.top_level_modules), 0)

    def test_commands_snapshot_exists(self):
        commands = get_commands()
        self.assertGreater(len(commands), 0)

    def test_tools_snapshot_exists(self):
        tools = get_tools()
        self.assertGreater(len(tools), 0)

class TestParityAudit(unittest.TestCase):
    def test_parity_audit_runs(self):
        result = run_parity_audit()
        self.assertIsNotNone(result)
```

**Running Tests**:
```bash
python3 -m unittest discover -s tests -v
```

---

## Asset Handling

### Asset Directory

**Location**: `assets/`

| File | Size | Purpose |
|------|------|---------|
| `clawd-hero.jpeg` | 238KB | Main hero image for README |
| `star-history.png` | 319KB | Star growth chart |
| `wsj-feature.png` | 894KB | Wall Street Journal feature screenshot |
| `tweet-screenshot.png` | 831KB | Viral announcement tweet |
| `instructkr.png` | 4.9KB | Instruct.kr branding |
| `omx/` | - | oh-my-codex workflow screenshots |

### Asset Usage in Documentation

Assets are referenced in README.md via relative paths:
```markdown
<p align="center">
  <img src="assets/clawd-hero.jpeg" alt="Claw" width="300" />
</p>

![Tweet screenshot](assets/tweet-screenshot.png)
![WSJ Feature](assets/wsj-feature.png)
```

### Asset Handling in Code

The Rust CLI does **not** embed assets - they are documentation-only. The Python workspace similarly doesn't reference assets programmatically.

If assets were to be embedded in the binary:
```rust
// Example (not implemented):
include_bytes!("../assets/clawd-hero.jpeg")

// Or with rust-embed:
#[derive(RustEmbed)]
#[folder = "../assets/"]
struct Assets;
```

---

## Configuration System

### Configuration Sources

**Location**: `rust/crates/runtime/src/config.rs`

```rust
pub enum ConfigSource {
    User,      // ~/.claude/settings.json
    Project,   // .claude.json (project root)
    Local,     // .claude/settings.local.json
}

pub struct ConfigLoader {
    cwd: PathBuf,
    config_home: PathBuf,
}

impl ConfigLoader {
    pub fn discover(&self) -> Vec<ConfigEntry> {
        vec![
            ConfigEntry {
                source: ConfigSource::User,
                path: self.config_home.parent().join(".claude.json"),
            },
            ConfigEntry {
                source: ConfigSource::User,
                path: self.config_home.join("settings.json"),
            },
            ConfigEntry {
                source: ConfigSource::Project,
                path: self.cwd.join(".claude.json"),
            },
            ConfigEntry {
                source: ConfigSource::Local,
                path: self.cwd.join(".claude/settings.local.json"),
            },
        ]
    }

    pub fn load(&self) -> Result<RuntimeConfig, ConfigError> {
        let entries = self.discover();
        let mut merged = BTreeMap::new();

        // Merge in order of precedence (later overrides earlier)
        for entry in entries {
            if entry.path.exists() {
                let content = fs::read_to_string(&entry.path)?;
                let json: JsonValue = serde_json::from_str(&content)?;
                merge_json(&mut merged, json);
            }
        }

        Ok(RuntimeConfig {
            merged,
            loaded_entries: entries,
            feature_config: self.extract_features(&merged),
        })
    }
}
```

### Configuration Schema

```json
{
  "model": "claude-sonnet-4-6",
  "permissionMode": "workspace-write",
  "allowedTools": ["read_file", "glob_search", "grep_search"],
  "dangerouslySkipPermissions": false,
  "outputFormat": "text",

  "hooks": {
    "preToolUse": [
      "echo 'Running {toolName}'",
      "scripts/validate.sh {toolName}"
    ],
    "postToolUse": [
      "echo 'Completed {toolName}'"
    ]
  },

  "mcpServers": {
    "filesystem": {
      "command": "npx",
      "args": ["-y", "@modelcontextprotocol/server-filesystem"],
      "cwd": "/path/to/watch"
    },
    "github": {
      "url": "https://github-mcp.example.com/sse",
      "headers": {
        "Authorization": "Bearer token"
      }
    }
  },

  "oauth": {
    "client_id": "...",
    "authorize_url": "...",
    "token_url": "...",
    "scopes": ["read", "write"]
  },

  "sandbox": {
    "filesystemIsolationMode": "full",
    "allowedDirectories": ["/workspace"]
  },

  "features": {
    "showThinking": false,
    "showTokenCount": true,
    "showCost": true
  }
}
```

### Environment Variables

| Variable | Purpose |
|----------|---------|
| `ANTHROPIC_API_KEY` | API key authentication |
| `ANTHROPIC_BASE_URL` | Custom API endpoint |
| `CLAUDE_CONFIG_HOME` | Override config directory |
| `CLAUDE_CODE_UPSTREAM` | TypeScript source path |
| `CLAUDE_CODE_AUTO_COMPACT_INPUT_TOKENS` | Auto-compaction threshold |

---

## Appendix: File Reference

### Key Source Files

| Path | Lines | Purpose |
|------|-------|---------|
| `rust/crates/rusty-claude-cli/src/main.rs` | ~3,159 | CLI entrypoint, REPL |
| `rust/crates/runtime/src/conversation.rs` | ~800 | Agentic loop |
| `rust/crates/runtime/src/config.rs` | ~900 | Configuration |
| `rust/crates/runtime/src/mcp_stdio.rs` | ~1,500 | MCP lifecycle |
| `rust/crates/runtime/src/oauth.rs` | ~500 | OAuth flow |
| `rust/crates/runtime/src/prompt.rs` | ~700 | System prompts |
| `rust/crates/tools/src/lib.rs` | ~800 | Tool specs |
| `rust/crates/rusty-claude-cli/src/render.rs` | ~640 | Markdown rendering |
| `rust/crates/rusty-claude-cli/src/input.rs` | ~270 | Line editing |
| `rust/crates/commands/src/lib.rs` | ~620 | Slash commands |
| `rust/crates/api/src/client.rs` | ~500 | HTTP client |
| `src/main.py` | ~200+ | Python CLI |

### Configuration Files

| Path | Purpose |
|------|---------|
| `rust/Cargo.toml` | Workspace definition |
| `rust/Cargo.lock` | Dependency versions |
| `.claude/settings.json` | Shared settings |
| `.claude/settings.local.json` | Local overrides |
| `CLAUDE.md` | Project instructions |

### Documentation Files

| Path | Purpose |
|------|---------|
| `README.md` | Project overview |
| `PARITY.md` | Feature parity analysis |
| `TUI-ENHANCEMENT-PLAN.md` | TUI roadmap |
| `rust/README.md` | Rust docs |

---

*Last updated: 2026-04-02*
*Based on commit: 2d8588c (ADD: cleanup)*
