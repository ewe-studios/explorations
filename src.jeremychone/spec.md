# Aipack -- Spec

## Source

- **Location:** `/home/darkvoid/Boxxed/@formulas/src.rust/src.llamacpp/src.jeremychone/aipack/`
- **Language:** Rust (edition 2024, MSRV 1.95)
- **Author:** Jeremy Chone (aipack.ai)
- **License:** MIT OR Apache-2.0
- **Version:** 0.8.24-WIP
- **Repository:** https://github.com/aipack-ai/aipack

## What This Project Is

Aipack is a command-line AI agent runner built in Rust. Agents are defined as `.aip` markdown files containing Lua scripts, TOML configuration, and prompt templates. The system supports a pack ecosystem (namespaced, installable agent packages), a ratatui TUI for interactive monitoring, SQLite-backed run tracking with detailed timing at every stage, multi-provider LLM routing (Anthropic, OpenAI, Gemini, Groq, xAI, etc.) with automatic pricing calculation, and an extensive Lua standard library with 30+ modules for file I/O, code parsing, web requests, document processing, and more. It is designed for production coding workflows — code generation, refactoring, documentation, and automation.

## Documentation Goal

A reader should understand:

1. How `.aip` agent files are parsed (lexer, state machine, section capture)
2. How the CLI dispatches to the executor and TUI
3. How the execution engine orchestrates before_all → tasks → after_all flows
4. How Lua scripts integrate with Rust via mlua and the aip.* module namespace
5. How the pack system works (packing, unpacking, installing, version checking)
6. How the SQLite database tracks runs, tasks, logs, and errors
7. How the ratatui TUI renders run lists, task details, and system info
8. How multi-provider LLM routing works with model aliases and pricing
9. How cancellation, concurrency, and error handling work
10. How to replicate these patterns in other languages

## Documentation Structure

| # | Document | Description |
|---|----------|-------------|
| 00 | Overview | What Aipack is, technology stack, architecture |
| 01 | CLI Structure | Subcommands, argument parsing, dispatch flow |
| 02 | Agent System | Agent definition, .aip parsing, options, references |
| 03 | Execution Engine | Executor, action dispatch, run orchestration |
| 04 | Run System | Run flow, task concurrency, AI processing, pricing |
| 05 | Lua Scripting | Lua engine, aip.* modules, AipackCustom responses |
| 06 | Pack System | Packing, unpacking, installation, version management |
| 07 | Directory Context | Workspace paths, pack directories, path resolution |
| 08 | Event System | Hub, flume channels, cancellation tokens |
| 09 | Database Schema | SQLite entities, modql BMCs, run/task tracking |
| 10 | TUI Architecture | Ratatui views, state machine, event handling |
| 11 | Runtime System | Clonable runtime, logging facade, model operations |
| 12 | Model & LLM | Multi-provider routing, pricing, model aliases |
| 13 | Template System | Handlebars integration, prompt rendering |
| 14 | Error System | Error enum, conversions, user-facing messages |
| 15 | Support Utilities | Markdown parsing, file ops, document processing |
| 16 | Initialization | Workspace setup, base directory, bundled assets |
| 17 | Rust Equivalents | How to replicate aipack patterns in other languages |
| 18 | Production Patterns | Concurrency, cancellation, caching, TUX performance |
