# src.jeremychone: Overview

A monorepo of 21 projects by Jeremy Choney spanning Rust AI/agent crates, Python packages, and developer tools.

**Source:** `/home/darkvoid/Boxxed/@formulas/src.rust/src.AI/src.jeremychone/`
**License:** MIT (most packages)

## All 21 Projects

### Rust Crates (10)

| Crate | Files | Purpose |
|-------|-------|---------|
| `rust-genai` | 100+ | AI provider client library — 19 providers (OpenAI, Anthropic, Google, Ollama, etc.) |
| `rust-rpc-router` | 60+ | JSON-RPC 2.0 router with FromResources dependency injection |
| `rust-modql` | 80+ | Query language for SeaORM/rusqlite with filters, fields, ordering |
| `rust-sqlb` | 40+ | SQL builder for insert/select/update/delete |
| `rust-udiffx` | 100+ | Unified diff patcher with security policy, hunks, incremental apply |
| `rust-agentic` | 20+ | MCP (Model Context Protocol) agent framework |
| `rust-pretty-sqlite` | 10+ | SQLite pretty-printer with table rendering |
| `rust-dinf` | 20+ | Directory info CLI tool |
| `rust-ss3` | — | Server-side session/state |
| `rust-ssite` | — | Static site generator |

### Python Packages (10+)

| Package | Purpose |
|---------|---------|
| `fasthtml` | FastHTML web framework |
| `claudette` | Claude API wrapper |
| `RAGatouille` | RAG (Retrieval-Augmented Generation) toolkit |
| `rerankers` | Document reranking |
| `starlette` | ASGI framework |
| `vllm` | High-throughput LLM serving |
| `fastlite` | FastAPI utilities |
| `fh-deploy` | FastHTML deployment tools |
| `fork-htmd` | HTML-to-markdown converter |
| `htmd` | HTML-to-markdown converter |
| `packs-*` | Various pack utilities (pro, lab, jc) |

### Developer Tools

| Tool | Purpose |
|------|---------|
| `aipack` | AI agent packaging tools |
| `aipack-vscode` | VS Code extension for aipack |
| `aiprog` | AI programming assistant |

## Cross-References

- [rust-genai](01-rust-genai.md) — AI provider client
- [rust-rpc-router](02-rust-rpc-router.md) — JSON-RPC routing
- [rust-modql](03-rust-modql.md) — Query language
- [rust-sqlb](04-rust-sqlb.md) — SQL builder
- [rust-udiffx](05-rust-udiffx.md) — Unified diff patcher
- [rust-agentic](06-rust-agentic.md) — MCP agent framework
- [rust-pretty-sqlite](07-rust-pretty-sqlite.md) — SQLite pretty-printer
- [python-fasthtml](08-python-fasthtml.md) — Python packages
- [tools](09-tools.md) — Developer tools

[Next → rust-genai](01-rust-genai.md)
