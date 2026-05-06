# Aipack Exploration

A command-line AI agent runner built in Rust. Agents are defined as `.aip` markdown files containing Lua scripts, TOML configuration, and prompt templates. The system supports a pack ecosystem (namespaced, installable agent packages), a ratatui TUI for interactive monitoring, SQLite-backed run tracking with detailed timing at every stage, multi-provider LLM routing with automatic pricing calculation, and an extensive Lua standard library with 30+ modules.

## Source

- **Location:** `/home/darkvoid/Boxxed/@formulas/src.rust/src.llamacpp/src.jeremychone/aipack/`
- **Language:** Rust (edition 2024, MSRV 1.95)
- **Author:** Jeremy Chone (aipack.ai)
- **License:** MIT OR Apache-2.0
- **Version:** 0.8.24-WIP
- **Repository:** https://github.com/aipack-ai/aipack
- **Files:** 391 `.rs` files across 50+ modules

## Tech Stack

| Crate | Purpose |
|-------|---------|
| `tokio` | Async runtime |
| `genai` | Multi-provider LLM client |
| `clap` | CLI argument parsing |
| `ratatui` | Terminal UI rendering |
| `rusqlite` | SQLite database |
| `modql` | SQL query builder + ORM macros |
| `mlua` (vendored Lua 5.4) | Embedded scripting |
| `flume` | Async channels |
| `handlebars` | Template rendering |
| `serde_json` / `toml` / `serde_yaml_ng` | Data serialization |
| `derive_more` | Error enum conversions |
| `uuid` (v7) | Time-ordered identifiers |
| `blake3` | Fast hashing |
| `simple_fs` | File utilities |

## Documentation

| # | Document | Description |
|---|----------|-------------|
| 00 | [Overview](markdown/00-overview.md) | What Aipack is, technology stack, architecture |
| 01 | [CLI Structure](markdown/01-cli-structure.md) | Subcommands, argument parsing, dispatch flow |
| 02 | [Agent System](markdown/02-agent-system.md) | Agent definition, .aip parsing, options, references |
| 03 | [Execution Engine](markdown/03-execution-engine.md) | Executor, action dispatch, run orchestration |
| 04 | [Run System](markdown/04-run-system.md) | Run flow, task concurrency, AI processing, pricing |
| 05 | [Lua Scripting](markdown/05-lua-scripting.md) | Lua engine, aip.* modules, AipackCustom responses |
| 06 | [Pack System](markdown/06-pack-system.md) | Packing, unpacking, installation, version management |
| 07 | [Directory Context](markdown/07-directory-context.md) | Workspace paths, pack directories, path resolution |
| 08 | [Event System](markdown/08-event-system.md) | Hub, flume channels, cancellation tokens |
| 09 | [Database Schema](markdown/09-database-schema.md) | SQLite entities, modql BMCs, run/task tracking |
| 10 | [TUI Architecture](markdown/10-tui-architecture.md) | Ratatui views, state machine, event handling |
| 11 | [Runtime System](markdown/11-runtime-system.md) | Clonable runtime, logging facade, model operations |
| 12 | [Model & LLM](markdown/12-model-llm.md) | Multi-provider routing, pricing, model aliases |
| 13 | [Template System](markdown/13-template-system.md) | Handlebars integration, prompt rendering |
| 14 | [Error System](markdown/14-error-system.md) | Error enum, conversions, user-facing messages |
| 15 | [Support Utilities](markdown/15-support-utilities.md) | Markdown parsing, file ops, document processing |
| 16 | [Initialization](markdown/16-initialization.md) | Workspace setup, base directory, bundled assets |
| 17 | [Rust Equivalents](markdown/17-rust-equivalents.md) | How to replicate aipack patterns in other languages |
| 18 | [Production Patterns](markdown/18-production-patterns.md) | Concurrency, cancellation, caching, TUI performance |

## Key Design Decisions

1. **Lua scripting layer** — Agents embed Lua 5.4 (vendored) with 30+ `aip.*` modules for file I/O, HTTP, code parsing, document processing, and flow control. This separates agent logic from Rust host code.

2. **Event-driven architecture** — A global Hub singleton broadcasts all status events (logs, errors, stage changes) via a flume channel. The TUI subscribes to this for real-time updates.

3. **SQLite-backed tracking** — Every run, task, log entry, and error is stored in a single SQLite file with WAL mode for concurrent read access. The `modql` derive macros generate field extraction and row construction.

4. **Pack ecosystem** — `.aipack` files are ZIP archives with a `pack.toml` manifest. Packs are installed into `~/.aipack-base/pack/installed/` with version comparison preventing accidental downgrades.

5. **Cancellation via generation counter** — Instead of `tokio::CancellationToken`, aipack uses atomic generation counters that support reuse across runs without stale state.

6. **Pricing calculator** — Per-provider pricing data with longest-prefix model matching, cache token splitting (normal/cached/creation), and reasoning token tracking. Supports 9 LLM providers.

7. **Clonable Runtime** — The `Runtime` struct wraps `Arc<RuntimeInner>` for cheap cloning. Every component (Lua engine, run system, TUI) receives a `Runtime` clone with shared database, LLM client, and directory context.
