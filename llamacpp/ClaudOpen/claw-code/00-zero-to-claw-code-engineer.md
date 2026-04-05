# Zero to Claw Code Engineer

A complete fundamentals guide for engineers new to the Claw Code project.

## Table of Contents

1. [What is Claw Code?](#what-is-claw-code)
2. [Project Origins](#project-origins)
3. [Repository Structure](#repository-structure)
4. [Dual Implementation Strategy](#dual-implementation-strategy)
5. [Getting Started](#getting-started)
6. [Core Concepts](#core-concepts)
7. [Development Workflow](#development-workflow)

---

## What is Claw Code?

Claw Code (stylized as `claw-code`, binary name `claw`) is an **AI agent harness** - a runtime system that orchestrates large language models with tool execution capabilities. Think of it as a bridge between an LLM's reasoning capabilities and real-world actions like:

- Reading and writing files
- Executing shell commands
- Searching codebases
- Making API calls
- Managing conversations and sessions

The system enables **agentic workflows** where an AI can iteratively solve complex tasks by:
1. Receiving a user prompt
2. Deciding which tools to use
3. Executing tools and observing results
4. Continuing the conversation with new context
5. Repeating until the task is complete

### Key Capabilities

| Capability | Description |
|------------|-------------|
| **Multi-turn conversation** | Maintains session state across back-and-forth exchanges |
| **Tool orchestration** | Coordinates 15+ built-in tools (bash, file ops, search, web, etc.) |
| **Permission system** | Three-tier security: ReadOnly, WorkspaceWrite, DangerFullAccess |
| **MCP support** | Model Context Protocol for external tool/server integration |
| **Session persistence** | Save and resume conversations across restarts |
| **Cost tracking** | Real-time token usage and USD estimation |
| **Hook system** | Pre/post tool execution callbacks for custom behavior |
| **CLAUDE.md** | Project-level context and instructions |

---

## Project Origins

Claw Code emerged in March 2026 as a response to the Claude Code source material becoming publicly available. The project has two key creators:

- **Sigrid Jin** ([@instructkr](https://github.com/instructkr)) - Primary author, featured in Wall Street Journal as a power user
- **Yeachan Heo** ([@bellman_ych](https://x.com/bellman_ych)) - Creator of oh-my-codex (OmX), the orchestration layer used to build Claw Code

### The Backstory

> At 4 AM on March 31, 2026, I woke up to my phone blowing up with notifications. The Claude Code source had been exposed, and the entire dev community was in a frenzy. [...] I sat down, ported the core features to Python from scratch, and pushed it before the sun came up.
>
> The whole thing was orchestrated end-to-end using [oh-my-codex (OmX)](https://github.com/Yeachan-Heo/oh-my-codex) — a workflow layer built on top of OpenAI's Codex.

The project gained unprecedented traction, reportedly surpassing **50,000 GitHub stars in just 2 hours** after publication.

### Philosophy

Claw Code is not simply a clone - it's a **clean-room reimplementation** that:
- Captures architectural patterns without copying proprietary source
- Focuses on "better harness tools" rather than archival storage
- Emphasizes Python-first development with an optimizing Rust port

---

## Repository Structure

```
claw-code/
├── README.md                    # Project overview and quickstart
├── PARITY.md                    # Feature parity analysis (TS vs Rust)
├── CLAUDE.md                    # Project instructions for Claude Code itself
├── assets/                      # Images, screenshots, branding
│   ├── clawd-hero.jpeg
│   ├── wsj-feature.png
│   ├── tweet-screenshot.png
│   └── omx/                     # OmX workflow screenshots
├── src/                         # Python porting workspace
│   ├── main.py                  # Python CLI entrypoint
│   ├── commands.py              # Command metadata
│   ├── tools.py                 # Tool metadata
│   ├── runtime.py               # Core runtime logic
│   ├── query_engine.py          # Query and routing
│   ├── port_manifest.py         # Porting status tracking
│   └── [20+ subsystems/]        # assistant, bootstrap, cli, coordinator, etc.
├── tests/                       # Python verification tests
│   └── test_porting_workspace.py
├── rust/                        # Rust workspace (active implementation)
│   ├── Cargo.toml               # Workspace root
│   ├── Cargo.lock
│   ├── README.md                # Rust-specific documentation
│   ├── TUI-ENHANCEMENT-PLAN.md  # Terminal UI roadmap
│   └── crates/
│       ├── api/                 # Anthropic API client + SSE streaming
│       ├── commands/            # Slash command registry
│       ├── compat-harness/      # TypeScript manifest extraction
│       ├── runtime/             # Core agentic loop, config, sessions
│       ├── rusty-claude-cli/    # Main CLI binary (`claw`)
│       └── tools/               # Built-in tool implementations
└── .claude/                     # Configuration directory
    ├── settings.json            # Shared settings
    └── settings.local.json      # Machine-local overrides
```

---

## Dual Implementation Strategy

Claw Code uses a **Python-first, Rust-optimized** dual implementation:

### Python Workspace (`src/`)

- **Purpose**: Rapid prototyping, porting analysis, manifest tracking
- **Entry Point**: `python3 -m src.main <command>`
- **Use Cases**:
  - Rendering porting summaries
  - Parity audits against TypeScript source
  - Command/tool inventory mirroring
  - Runtime simulation and testing

```bash
# Common Python commands
python3 -m src.main summary          # Porting workspace summary
python3 -m src.main manifest         # Current workspace manifest
python3 -m src.main subsystems       # List Python modules
python3 -m src.main commands --limit 10  # List mirrored commands
python3 -m src.main tools --limit 10     # List mirrored tools
python3 -m src.main parity-audit     # Compare against TypeScript source
```

### Rust Workspace (`rust/`)

- **Purpose**: Production-ready, high-performance CLI
- **Binary Name**: `claw`
- **Lines of Code**: ~20,000
- **Crates**: 6 in workspace

```bash
# Common Rust commands
cd rust/
cargo build --release
./target/release/claw              # Interactive REPL
./target/release/claw prompt "fix the bug"  # One-shot
./target/release/claw --model sonnet prompt "explain this"
./target/release/claw login        # OAuth authentication
```

### Why Two Implementations?

| Aspect | Python | Rust |
|--------|--------|------|
| Development Speed | Fast iteration | Careful, type-safe |
| Runtime Performance | Adequate | High-performance |
| Memory Safety | GC-managed | Compile-time guarantees |
| Binary Distribution | Requires interpreter | Single static binary |
| Primary Use | Porting analysis | Production CLI |

---

## Getting Started

### Prerequisites

**For Python Development:**
```bash
python3 --version  # Python 3.10+
```

**For Rust Development:**
```bash
rustc --version    # Latest stable
cargo --version
```

### Quickstart: Rust CLI

```bash
# 1. Clone and build
cd claw-code/rust
cargo build --release

# 2. Set up authentication
export ANTHROPIC_API_KEY="sk-ant-..."
# Or authenticate via OAuth:
./target/release/claw login

# 3. Run interactive REPL
./target/release/claw

# 4. Or use one-shot prompts
./target/release/claw prompt "What files are in this directory?"
./target/release/claw --model sonnet prompt "Explain the architecture"
```

### Quickstart: Python Analysis

```bash
cd claw-code

# View porting summary
python3 -m src.main summary

# List subsystems
python3 -m src.main subsystems --limit 16

# Run verification tests
python3 -m unittest discover -s tests -v
```

### Configuration

Claw Code reads configuration from multiple sources (in order of precedence):

1. `.claude/settings.local.json` - Machine-local overrides
2. `.claude.json` or `.claude/settings.json` - Project settings
3. `~/.claude/settings.json` - User-global settings
4. Environment variables (`ANTHROPIC_API_KEY`, etc.)

Example `.claude.json`:
```json
{
  "model": "claude-sonnet-4-6",
  "permissionMode": "workspace-write",
  "hooks": {
    "preToolUse": ["echo 'About to run: {toolName}'"],
    "postToolUse": ["echo 'Completed: {toolName}'"]
  },
  "mcpServers": {
    "filesystem": {
      "command": "npx",
      "args": ["-y", "@modelcontextprotocol/server-filesystem"]
    }
  }
}
```

---

## Core Concepts

### 1. The Agentic Loop

At the heart of Claw Code is the **conversation loop**:

```
┌─────────────────────────────────────────────────────────────┐
│                     User Input                              │
└─────────────────────────────────────────────────────────────┘
                           │
                           ▼
┌─────────────────────────────────────────────────────────────┐
│                  System Prompt                              │
│  (context, tools, permissions, project memory)              │
└─────────────────────────────────────────────────────────────┘
                           │
                           ▼
┌─────────────────────────────────────────────────────────────┐
│                  API Request                                │
│  (messages + system prompt → Anthropic API)                 │
└─────────────────────────────────────────────────────────────┘
                           │
                           ▼
┌─────────────────────────────────────────────────────────────┐
│               Assistant Response                            │
│  (text content + optional tool calls)                       │
└─────────────────────────────────────────────────────────────┘
                           │
              ┌────────────┴────────────┐
              │                         │
         Text only                  Tool calls
              │                         │
              ▼                         ▼
         Display              ┌─────────────────┐
                              │ Permission Check│
                              └─────────────────┘
                                      │
                              ┌───────┴───────┐
                              │               │
                           Allowed        Denied
                              │               │
                              ▼               ▼
                       ┌──────────┐    Return Error
                       │ Execute  │
                       │  Tool    │
                       └──────────┘
                              │
                              ▼
                       ┌──────────┐
                       │  Result  │
                       └──────────┘
                              │
                              ▼
                    Add to Conversation
                              │
                              └──────┐
                                     │
                              (Loop continues)
```

### 2. Permission Modes

Claw Code enforces a three-tier permission system:

| Mode | Description | Allowed Tools |
|------|-------------|---------------|
| `read-only` | Safe inspection | read_file, glob_search, grep_search, WebFetch, WebSearch |
| `workspace-write` | File modifications | All read tools + write_file, edit_file, NotebookEdit |
| `danger-full-access` | Full system access | All tools including bash, Agent |

Set via CLI flag:
```bash
./target/release/claw --permission-mode read-only
./target/release/claw --dangerously-skip-permissions  # Shorthand for danger-full-access
```

### 3. Tools

Tools are the actions the AI can take. The Rust MVP includes:

| Tool | Permission | Description |
|------|------------|-------------|
| `bash` | DangerFullAccess | Execute shell commands |
| `read_file` | ReadOnly | Read file contents |
| `write_file` | WorkspaceWrite | Create/overwrite files |
| `edit_file` | WorkspaceWrite | Replace text in files |
| `glob_search` | ReadOnly | Find files by pattern |
| `grep_search` | ReadOnly | Search file contents |
| `WebFetch` | ReadOnly | Fetch and analyze URLs |
| `WebSearch` | ReadOnly | Web search with citations |
| `TodoWrite` | WorkspaceWrite | Manage task list |
| `Skill` | ReadOnly | Load local skill definitions |
| `Agent` | DangerFullAccess | Spawn sub-agents |
| `ToolSearch` | ReadOnly | Find tools by name/keywords |
| `NotebookEdit` | WorkspaceWrite | Edit Jupyter notebooks |
| `Sleep` | ReadOnly | Wait without holding process |
| `REPL` | DangerFullAccess | Run code in interpreters |

### 4. Slash Commands

In the interactive REPL, slash commands provide session management:

| Command | Description |
|---------|-------------|
| `/help` | Show available commands |
| `/status` | Session status (model, tokens, cost) |
| `/compact` | Compress conversation history |
| `/model [name]` | Switch AI model |
| `/permissions` | Change permission mode |
| `/clear` | Start fresh conversation |
| `/cost` | Token usage breakdown |
| `/config [section]` | View configuration |
| `/memory` | Show CLAUDE.md contents |
| `/diff` | Git diff of workspace |
| `/export [path]` | Save conversation |
| `/session [action]` | Manage sessions |
| `/version` | CLI version info |

### 5. Sessions

Conversations are persisted as JSON files:

```json
{
  "version": 1,
  "messages": [
    {
      "role": "user",
      "content": "Explain the architecture"
    },
    {
      "role": "assistant",
      "content": "The system has three main layers..."
    }
  ],
  "inputTokens": 1500,
  "outputTokens": 800
}
```

Sessions enable:
- Resuming conversations: `claw --resume session.json`
- Cost tracking across turns
- Context compaction for long conversations

### 6. MCP (Model Context Protocol)

MCP enables connecting external tools and resources:

```json
{
  "mcpServers": {
    "filesystem": {
      "command": "npx",
      "args": ["-y", "@modelcontextprotocol/server-filesystem"],
      "cwd": "/path/to/watch"
    },
    "github": {
      "url": "https://github-mcp-server.example.com/sse"
    }
  }
}
```

### 7. Hooks

Hooks enable custom behavior around tool execution:

```json
{
  "hooks": {
    "preToolUse": [
      "echo 'Running {toolName} with input: {input}'",
      "scripts/validate-tool-input.sh {toolName}"
    ],
    "postToolUse": [
      "echo 'Tool {toolName} completed with result: {result}'"
    ]
  }
}
```

---

## Development Workflow

### Adding a New Tool

**In Rust (`rust/crates/tools/src/lib.rs`):**

1. Add tool spec to `mvp_tool_specs()`:
```rust
ToolSpec {
    name: "my_new_tool",
    description: "Does something useful",
    input_schema: json!({
        "type": "object",
        "properties": {
            "param1": { "type": "string" }
        },
        "required": ["param1"]
    }),
    required_permission: PermissionMode::ReadOnly,
}
```

2. Implement execution in the tool handler

3. Test with: `cargo test --workspace`

### Adding a Slash Command

**In Rust (`rust/crates/commands/src/lib.rs`):**

1. Add to `SLASH_COMMAND_SPECS`:
```rust
SlashCommandSpec {
    name: "mycommand",
    summary: "Does something",
    argument_hint: Some("[arg]"),
    resume_supported: true,
}
```

2. Add enum variant to `SlashCommand`

3. Implement handler in `handle_slash_command()`

### Running Tests

```bash
# Full workspace test
cargo test --workspace

# Specific crate
cargo test -p runtime

# With output
cargo test --workspace -- --nocapture

# Python tests
python3 -m unittest discover -s tests -v
```

### Formatting and Linting

```bash
cd rust/
cargo fmt --all
cargo clippy --workspace --all-targets -- -D warnings
```

---

## Architecture at a Glance

### Rust Crate Dependencies

```
rusty-claude-cli (main binary)
├── api          (HTTP client, SSE streaming)
├── commands     (slash command registry)
├── compat-harness (TS manifest extraction)
├── runtime      (conversation loop, config, sessions)
└── tools        (tool implementations)

runtime
├── api (types)
└── tools (execution)

tools
├── api (types)
└── runtime (config)
```

### Key Source Files

| File | Lines | Purpose |
|------|-------|---------|
| `rust/crates/rusty-claude-cli/src/main.rs` | ~3,159 | CLI entrypoint, REPL loop |
| `rust/crates/runtime/src/conversation.rs` | ~800 | Agentic loop implementation |
| `rust/crates/runtime/src/config.rs` | ~900 | Configuration loading |
| `rust/crates/tools/src/lib.rs` | ~800 | Tool specs and execution |
| `rust/crates/runtime/src/mcp_stdio.rs` | ~1,500 | MCP server management |

---

## Learning Path

### Week 1: Fundamentals
1. Read this document thoroughly
2. Build and run the Rust CLI
3. Experiment with basic tools (read_file, bash, glob)
4. Read `rust/README.md`

### Week 2: Deep Dive
1. Study `rust/crates/runtime/src/conversation.rs` (the core loop)
2. Understand the permission system in `permissions.rs`
3. Trace a tool execution end-to-end
4. Read `PARITY.md` for feature comparison

### Week 3: Contribution
1. Pick a small feature from `TUI-ENHANCEMENT-PLAN.md`
2. Add a simple slash command
3. Improve test coverage
4. Submit a PR

---

## Community and Support

- **GitHub**: [instructkr/claw-code](https://github.com/instructkr/claw-code)
- **Discord**: [instruct.kr](https://instruct.kr/) (Korean language LLM community)
- **Sponsor**: [GitHub Sponsors](https://github.com/sponsors/instructkr)

---

## Disclaimer

This repository:
- Does **not** claim ownership of the original Claude Code source material
- Is **not affiliated with, endorsed by, or maintained by Anthropic**
- Exists as a clean-room reimplementation for educational and research purposes
