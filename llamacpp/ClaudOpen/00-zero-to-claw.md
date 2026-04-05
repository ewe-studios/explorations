# Zero to Claw: Getting Started with ClaudOpen

**Source:** `/home/darkvoid/Boxxed/@formulas/src.rust/src.llamacpp/src.ClaudOpen`

This guide takes you from zero knowledge to a working understanding of the ClaudOpen project — a high-performance Rust implementation of an AI agent harness inspired by Claude Code.

---

## Table of Contents

1. [What is ClaudOpen?](#what-is-claudeopen)
2. [Why Rust?](#why-rust)
3. [Project Origins](#project-origins)
4. [Quick Start](#quick-start)
5. [Core Concepts](#core-concepts)
6. [Architecture Overview](#architecture-overview)
7. [Next Steps](#next-steps)

---

## What is ClaudOpen?

**ClaudOpen** (marketed as **Claw Code**) is an open-source CLI tool that provides an interactive AI coding assistant. It implements the core patterns of agentic AI systems:

- **Tool-based interaction** — The AI can read/write files, run commands, search code, fetch web content
- **Permission system** — Users control what actions the AI can take autonomously
- **Session management** — Conversations persist across restarts
- **MCP support** — Model Context Protocol servers extend functionality
- **Hooks system** — Run custom scripts before/after tool execution
- **Multi-agent orchestration** — Spawn sub-agents for specialized tasks

Think of it as a **local, customizable AI development partner** that runs in your terminal.

### Key Features at a Glance

| Feature | Description |
|---------|-------------|
| Interactive REPL | Chat with AI in your terminal |
| Tool execution | Bash, file ops, search, web tools |
| Permission modes | Read-only → Workspace write → Full access |
| Session persistence | Resume conversations anytime |
| MCP servers | Connect external tools via standard protocol |
| Hooks | Pre/post tool execution scripts |
| Model aliases | `opus`, `sonnet`, `haiku` shortcuts |

---

## Why Rust?

The original inspiration (Claude Code) was built in TypeScript. ClaudOpen is a **ground-up Rust rewrite** for several reasons:

### Performance Benefits

| Aspect | TypeScript | Rust |
|--------|-----------|------|
| Startup time | ~500ms | ~50ms |
| Memory footprint | ~200MB | ~30MB |
| Tool execution | Event loop overhead | Native subprocess |
| Binary distribution | Requires Node.js | Single static binary |

### Safety Guarantees

- **Memory safety** — No segfaults, buffer overflows, or use-after-free bugs
- **Thread safety** — Fearless concurrency with the type system
- **No undefined behavior** — The compiler catches entire classes of bugs

### Developer Experience

```rust
// Type-safe tool execution
pub trait ToolExecutor {
    fn execute(&mut self, tool_name: &str, input: &str) -> Result<String, ToolError>;
}

// Compile-time permission checking
pub enum PermissionMode {
    ReadOnly,      // Can only read files
    WorkspaceWrite, // Can modify workspace
    DangerFullAccess, // Can run any command
}
```

---

## Project Origins

The project emerged in March 2026 following the public exposure of the Claude Code codebase. Rather than directly copying proprietary code, the author:

1. **Analyzed behavioral patterns** — How the system responds to various inputs
2. **Reconstructed from scratch** — Implemented similar functionality independently
3. **Focused on learning** — Documented patterns for educational purposes

### The Story

> At 4 AM on March 31, 2026, I woke up to my phone blowing up with notifications. The Claude Code source had been exposed, and the entire dev community was in a frenzy... I sat down, ported the core features to Python from scratch, and pushed it before the sun came up.

The project quickly gained traction, reaching 50K GitHub stars in just 2 hours. The current Rust implementation is the third iteration:

1. **Python prototype** — Initial proof of concept
2. **TypeScript analysis** — Understanding the original architecture
3. **Rust rewrite** — Production-grade implementation

---

## Quick Start

### Prerequisites

- Rust toolchain (`rustup install stable`)
- Anthropic API key or OAuth credentials

### Installation

```bash
# Clone the repository
git clone https://github.com/instructkr/claw-code.git
cd claw-code/rust

# Build
cargo build --release

# The binary is now at:
./target/release/claw
```

### Configuration

```bash
# Set your API key
export ANTHROPIC_API_KEY="sk-ant-..."

# Or use OAuth
./target/release/claw login
```

### First Run

```bash
# Start interactive mode
./target/release/claw

# One-shot prompt
./target/release/claw prompt "Explain this codebase"

# With specific model
./target/release/claw --model sonnet prompt "Fix the bug in main.rs"
```

### Permission Modes

```bash
# Read-only (safest)
./target/release/claw --permission-mode read-only

# Workspace write (default)
./target/release/claw --permission-mode workspace-write

# Full access (use with caution)
./target/release/claw --permission-mode danger-full-access
```

---

## Core Concepts

### 1. The Conversation Loop

At its heart, ClaudOpen implements a simple loop:

```
User Input → API Request → Response → Tool Execution → Result → Repeat
```

```rust
pub fn run_turn(
    &mut self,
    user_input: impl Into<String>,
    prompter: Option<&mut dyn PermissionPrompter>
) -> Result<TurnSummary, RuntimeError> {
    // 1. Add user message to session
    self.session.messages.push(ConversationMessage::user_text(user_input));

    // 2. Stream response from API
    let events = self.api_client.stream(request)?;

    // 3. Extract tool calls from response
    for tool_use in pending_tool_uses {
        // 4. Check permissions
        let outcome = self.permission_policy.authorize(tool_name, input, prompter);

        // 5. Execute if allowed
        if let PermissionOutcome::Allow = outcome {
            let result = self.tool_executor.execute(tool_name, input)?;
            // 6. Add result to session
        }
    }
}
```

### 2. Permission System

The permission system controls what the AI can do without asking:

```
PermissionMode Hierarchy (low to high):

    ReadOnly
        ↓
    WorkspaceWrite
        ↓
    DangerFullAccess
```

| Mode | Allowed Operations |
|------|-------------------|
| `read-only` | Read files, search, grep, web fetch |
| `workspace-write` | Also: create/edit files, todo tracking |
| `danger-full-access` | Also: bash commands, sub-agents |

### 3. Session Management

Sessions persist to disk and can be resumed:

```rust
#[derive(Debug, Clone)]
pub struct Session {
    pub version: u32,
    pub messages: Vec<ConversationMessage>,
}

// Save session
session.save_to_path("/path/to/session.json")?;

// Load session
let session = Session::load_from_path("/path/to/session.json")?;
```

### 4. Tool Architecture

Tools are the actions the AI can take:

```rust
// Tool definition
pub struct ToolSpec {
    pub name: &'static str,
    pub description: &'static str,
    pub input_schema: Value,  // JSON Schema
    pub required_permission: PermissionMode,
}

// Available tools
pub fn mvp_tool_specs() -> Vec<ToolSpec> {
    vec![
        ToolSpec { name: "bash", ... },
        ToolSpec { name: "read_file", ... },
        ToolSpec { name: "write_file", ... },
        // ... 18 total tools
    ]
}
```

---

## Architecture Overview

### Crate Structure

```
rust/
├── Cargo.toml              # Workspace definition
└── crates/
    ├── api/                # Anthropic API client
    │   ├── client.rs       # HTTP client, SSE streaming
    │   ├── types.rs        # Request/response types
    │   └── sse.rs          # Server-sent events parser
    │
    ├── commands/           # Slash command registry
    │   └── lib.rs          # /help, /status, /compact, etc.
    │
    ├── runtime/            # Core agentic loop
    │   ├── conversation.rs # ConversationRuntime
    │   ├── session.rs      # Session persistence
    │   ├── config.rs       # Configuration loading
    │   ├── permissions.rs  # Permission system
    │   ├── prompt.rs       # System prompt builder
    │   └── mcp_*.rs        # MCP client implementation
    │
    ├── tools/              # Built-in tools
    │   └── lib.rs          # Tool specs + execution
    │
    └── rusty-claude-cli/   # Main CLI binary
        ├── main.rs         # Entry point, REPL loop
        ├── render.rs       # Markdown → terminal rendering
        └── input.rs        # Line editor (rustyline)
```

### Data Flow

```
┌─────────────┐     ┌──────────────┐     ┌─────────────┐
│   User      │────▶│  rustyline   │────▶│  LiveCli    │
│  Input      │     │  (input.rs)  │     │  (main.rs)  │
└─────────────┘     └──────────────┘     └──────┬──────┘
                                                │
                              ┌─────────────────┼─────────────────┐
                              │                 │                 │
                              ▼                 ▼                 ▼
                     ┌────────────────┐ ┌──────────────┐ ┌────────────┐
                     │ Conversation   │ │  Commands    │ │  Tools     │
                     │ Runtime        │ │  Registry    │ │  Executor  │
                     │ (runtime/)     │ │ (commands/)  │ │ (tools/)   │
                     └───────┬────────┘ └──────────────┘ └─────┬──────┘
                             │                                 │
                             ▼                                 ▼
                     ┌────────────────┐              ┌────────────────┐
                     │  API Client    │              │  File/Bash/    │
                     │  (api/)        │              │  Web Tools     │
                     └────────────────┘              └────────────────┘
```

---

## Next Steps

Now that you understand the basics, dive deeper:

1. **[01-claw-exploration.md](01-claw-exploration.md)** — Detailed architecture deep-dive
2. **[deep-dives/wasm-render-deep-dive.md](deep-dives/wasm-render-deep-dive.md)** — Terminal rendering internals
3. **[deep-dives/backend-deep-dive.md](deep-dives/backend-deep-dive.md)** — API client and streaming
4. **[rust-revision.md](rust-revision.md)** — Rust-specific design patterns
5. **[production-grade.md](production-grade.md)** — Building for production

---

## Glossary

| Term | Definition |
|------|------------|
| **REPL** | Read-Eval-Print Loop — interactive terminal interface |
| **MCP** | Model Context Protocol — standard for extending AI tools |
| **SSE** | Server-Sent Events — streaming protocol for API responses |
| **Tool** | An action the AI can execute (read file, run command) |
| **Hook** | Script that runs before/after tool execution |
| **Session** | A persistent conversation history |
| **Slash command** | In-REPL commands starting with `/` |

---

## Resources

- **Source Repository:** `/home/darkvoid/Boxxed/@formulas/src.rust/src.llamacpp/src.ClaudOpen`
- **Rust Implementation:** `/home/darkvoid/Boxxed/@formulas/src.rust/src.llamacpp/src.ClaudOpen/claw-code/rust`
- **System Prompts:** `/home/darkvoid/Boxxed/@formulas/src.rust/src.llamacpp/src.ClaudOpen/claude-code-system-prompts`

---

*Generated: 2026-04-02*
