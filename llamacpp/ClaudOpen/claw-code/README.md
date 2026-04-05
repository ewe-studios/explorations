# Claw Code Exploration Index

Comprehensive documentation for the Claw Code Rust implementation.

## Source Repository

**Location**: `/home/darkvoid/Boxxed/@formulas/src.rust/src.llamacpp/src.ClaudOpen/claw-code`

**GitHub**: [instructkr/claw-code](https://github.com/instructkr/claw-code)

---

## Document Overview

| Document | Purpose | Target Audience |
|----------|---------|-----------------|
| [00-zero-to-claw-code-engineer.md](./00-zero-to-claw-code-engineer.md) | Complete fundamentals guide | New engineers |
| [01-claw-code-exploration.md](./01-claw-code-exploration.md) | Full architecture deep-dive | System architects |
| [rust-revision.md](./rust-revision.md) | Rust-specific patterns and crate structure | Rust developers |
| [production-grade.md](./production-grade.md) | Production-ready implementation guide | Senior engineers |
| [tui-deep-dive.md](./tui-deep-dive.md) | TUI implementation and enhancement plan | UI developers |
| [deep-dives/memory-model-deep-dive.md](./deep-dives/memory-model-deep-dive.md) | Session persistence and compaction | Backend developers |
| [deep-dives/tool-system-hooks.md](./deep-dives/tool-system-hooks.md) | Tool execution and hooks | Backend developers |
| [deep-dives/source-files/main-and-entry-points.md](./deep-dives/source-files/main-and-entry-points.md) | CLI entry points and REPL flow | CLI developers |
| [deep-dives/source-files/api-client-layer.md](./deep-dives/source-files/api-client-layer.md) | API client, SSE, retry logic | API developers |
| [deep-dives/source-files/prompt-system.md](./deep-dives/source-files/prompt-system.md) | System prompt construction | Prompt engineers |

---

## Quick Reference

### Repository Structure

```
claw-code/
├── README.md                    # Project overview
├── PARITY.md                    # Feature parity analysis
├── assets/                      # Images and screenshots
├── src/                         # Python porting workspace
├── tests/                       # Python verification
└── rust/                        # Rust workspace (production)
    ├── Cargo.toml               # Workspace root
    ├── README.md                # Rust documentation
    ├── TUI-ENHANCEMENT-PLAN.md  # TUI roadmap
    └── crates/
        ├── api/                 # Anthropic API client
        ├── commands/            # Slash command registry
        ├── compat-harness/      # TS manifest extraction
        ├── runtime/             # Core agentic loop
        ├── rusty-claude-cli/    # Main CLI binary
        └── tools/               # Tool implementations
```

### Key Statistics

| Metric | Value |
|--------|-------|
| **Rust LOC** | ~20,000 lines |
| **Crates** | 6 in workspace |
| **Binary** | `claw` |
| **Default Model** | `claude-opus-4-6` |
| **Tools (MVP)** | 15 built-in |
| **Slash Commands** | 22 registered |

### Quick Start

```bash
# Build
cd rust/
cargo build --release

# Run interactive REPL
./target/release/claw

# One-shot prompt
./target/release/claw prompt "explain this codebase"

# Run tests
cargo test --workspace

# Format and lint
cargo fmt --all
cargo clippy --workspace -- -D warnings
```

---

## Document Summaries

### 00-zero-to-claw-code-engineer.md

**Purpose**: Take engineers from zero knowledge to productive understanding

**Contents**:
- What is Claw Code and its origins
- Repository structure overview
- Dual implementation strategy (Python/Rust)
- Core concepts (agentic loop, permissions, tools, sessions)
- Getting started guide
- Development workflow

**Key Sections**:
1. What is Claw Code?
2. Project Origins
3. Repository Structure
4. Dual Implementation Strategy
5. Getting Started
6. Core Concepts
7. Development Workflow

---

### 01-claw-code-exploration.md

**Purpose**: Comprehensive architecture deep-dive

**Contents**:
- Executive summary with statistics
- Detailed repository overview
- Rust workspace architecture (6 crates)
- Core runtime flow (startup, message handling)
- TypeScript source analysis
- Component deep dives (permissions, MCP, OAuth, sessions)
- Testing strategy
- Asset handling
- Configuration system

**Key Sections**:
1. Executive Summary
2. Repository Overview
3. Rust Workspace Architecture
4. Core Runtime Flow
5. TypeScript Source Analysis
6. Component Deep Dives
7. Testing Strategy
8. Configuration System

---

### rust-revision.md

**Purpose**: Rust-specific implementation patterns

**Contents**:
- Workspace organization
- Crate-by-crate deep dive (api, runtime, tools, commands)
- Key Rust patterns (traits, builders, newtypes)
- Async runtime design (Tokio)
- Error handling strategy
- Memory management
- Testing patterns
- Build and release process

**Key Sections**:
1. Workspace Overview
2. Crate-by-Crate Deep Dive
3. Key Rust Patterns
4. Async Runtime Design
5. Error Handling
6. Memory Management
7. Testing Patterns
8. Build and Release

---

### production-grade.md

**Purpose**: Guide for production-ready implementations

**Contents**:
- Production readiness assessment
- Architecture principles
- Feature implementation patterns
- Performance optimization
- Error handling and resilience
- Security considerations
- Observability and monitoring
- Deployment strategies

**Key Sections**:
1. Production Readiness Assessment
2. Architecture Principles
3. Feature Implementation Patterns
4. Performance Optimization
5. Error Handling and Resilience
6. Security Considerations
7. Observability and Monitoring
8. Deployment Strategies

---

### tui-deep-dive.md

**Purpose**: TUI implementation and enhancement guide

**Contents**:
- Current TUI architecture analysis
- 6-phase enhancement plan
- Component implementation guides
- Structural cleanup (Phase 0)
- Status bar & HUD (Phase 1)
- Enhanced streaming (Phase 2)
- Tool visualization (Phase 3)
- Advanced features (Phase 4-6)

**Key Sections**:
1. Current TUI Architecture
2. TUI Enhancement Plan
3. Component Implementation Guide
4. Phase 0: Structural Cleanup
5. Phase 1: Status Bar & Live HUD
6. Phase 2: Enhanced Streaming
7. Phase 3: Tool Visualization
8. Phase 4-6: Advanced Features

---

## Related Source Documents

These explorations are based on analysis of the following source documents:

| Source File | Purpose |
|-------------|---------|
| `README.md` | Project narrative and quickstart |
| `PARITY.md` | Feature gap analysis (TypeScript → Rust) |
| `rust/README.md` | Rust-specific documentation |
| `rust/TUI-ENHANCEMENT-PLAN.md` | TUI enhancement roadmap |
| `rust/Cargo.toml` | Workspace configuration |
| `CLAUDE.md` | Project instructions |

---

## Crate Quick Reference

### `api` Crate
- **Path**: `rust/crates/api/`
- **Purpose**: Anthropic API client with SSE streaming
- **Key Types**: `AnthropicClient`, `MessageRequest`, `StreamEvent`, `Usage`
- **Dependencies**: reqwest, tokio, serde

### `commands` Crate
- **Path**: `rust/crates/commands/`
- **Purpose**: Slash command registry and parsing
- **Key Types**: `SlashCommand`, `SlashCommandSpec`, `SlashCommandResult`
- **Commands**: 22 registered (/help, /status, /compact, etc.)

### `compat-harness` Crate
- **Path**: `rust/crates/compat-harness/`
- **Purpose**: Extract manifests from TypeScript source
- **Key Functions**: `extract_manifest`, `extract_commands`, `extract_tools`

### `runtime` Crate
- **Path**: `rust/crates/runtime/`
- **Purpose**: Core agentic loop, config, sessions, MCP
- **Key Types**: `ConversationRuntime`, `ConfigLoader`, `Session`, `PermissionPolicy`
- **Modules**: 20 (conversation, config, mcp_stdio, oauth, prompt, etc.)

### `tools` Crate
- **Path**: `rust/crates/tools/`
- **Purpose**: Built-in tool specs and execution
- **Key Functions**: `mvp_tool_specs`, `execute_tool`
- **Tools**: 15 MVP tools (bash, read_file, write_file, etc.)

### `rusty-claude-cli` Crate
- **Path**: `rust/crates/rusty-claude-cli/`
- **Purpose**: Main CLI binary with REPL
- **Binary**: `claw`
- **Key Types**: `LiveCli`, `TerminalRenderer`, `LineEditor`

---

## Learning Path

### Week 1: Fundamentals
1. Read `00-zero-to-claw-code-engineer.md`
2. Build and run the Rust CLI
3. Experiment with basic tools
4. Read `rust/README.md`

### Week 2: Architecture
1. Read `01-claw-code-exploration.md`
2. Study `rust/crates/runtime/src/conversation.rs`
3. Understand the permission system
4. Trace a tool execution end-to-end

### Week 3: Rust Patterns
1. Read `rust-revision.md`
2. Study crate organization
3. Review error handling patterns
4. Run and extend tests

### Week 4: Production
1. Read `production-grade.md`
2. Pick a feature to implement
3. Follow feature implementation patterns
4. Submit a PR

### Week 5: TUI (Optional)
1. Read `tui-deep-dive.md`
2. Implement Phase 0 (structural cleanup)
3. Add status bar (Phase 1)
4. Enhance tool visualization (Phase 3)

---

## Glossary

| Term | Definition |
|------|------------|
| **Agentic Loop** | The core conversation cycle: user input → API request → response → tool execution → repeat |
| **CLAUDE.md** | Project-level context and instructions file |
| **MCP** | Model Context Protocol for external tool/server integration |
| **SSE** | Server-Sent Events streaming from Anthropic API |
| **Tool Spec** | JSON schema defining a tool's name, description, and input |
| **Slash Command** | REPL commands prefixed with `/` (/help, /status, etc.) |
| **Permission Mode** | Security tier: ReadOnly, WorkspaceWrite, DangerFullAccess |
| **Session** | Persisted conversation state (messages, tokens) |
| **Hook** | Pre/post tool execution callbacks |
| **LiveCli** | Main REPL state struct in rusty-claude-cli |

---

## External Resources

- **oh-my-codex (OmX)**: [github.com/Yeachan-Heo/oh-my-codex](https://github.com/Yeachan-Heo/oh-my-codex)
- **instruct.kr Discord**: [instruct.kr](https://instruct.kr/)
- **GitHub Sponsors**: [github.com/sponsors/instructkr](https://github.com/sponsors/instructkr)

---

*Generated: 2026-04-02*
*Exploration based on commit: 2d8588c (ADD: cleanup)*
