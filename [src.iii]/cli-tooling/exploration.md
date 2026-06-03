---
location: /home/darkvoid/Boxxed/@formulas/src.rust/src.llamacpp/src.iii/cli-tooling
repository: git@github.com:iii-hq/cli-tooling
explored_at: 2026-06-03T00:00:00Z
language: Rust
---

# Project Exploration: iii CLI Tooling — Project Management CLI

## Overview

The iii CLI Tooling is a **Rust workspace providing project management CLIs** for the iii ecosystem. It ships two binaries — `iii-tools` for managing iii projects and `motia` for managing Motia projects (which integrates with iii) — both built on a shared `scaffolder-core` library that handles project scaffolding with template support.

```
┌──────────────────────────────────────────────┐
│              CLI Binaries                     │
│  ┌─────────────┐    ┌──────────────────┐     │
│  │  iii-tools  │    │      motia       │     │
│  │  (iii mgmt) │    │  (Motia + iii)   │     │
│  └──────┬──────┘    └───────┬──────────┘     │
│         └───────────────────┘                │
│                    │                          │
│         ┌──────────▼──────────┐              │
│         │  scaffolder-core    │              │
│         │  (template engine)  │              │
│         │  + tui (optional)   │              │
│         └─────────────────────┘              │
└──────────────────────────────────────────────┘
```

## Repository

- **Location:** `/home/darkvoid/Boxxed/@formulas/src.rust/src.llamacpp/src.iii/cli-tooling`
- **Remote:** `git@github.com:iii-hq/cli-tooling`
- **Primary Language:** Rust
- **Version:** 0.6.3, Edition 2021

## Directory Structure

```
cli-tooling/
├── Cargo.toml                      # Workspace definition
├── config.yaml                     # CLI configuration
├── scripts/                        # Build/utility scripts
├── templates/                      # Project templates
└── crates/
    ├── iii-tools/                  # ── iii-tools CLI ──
    │   ├── Cargo.toml              # iii-tools binary
    │   └── src/
    │       └── main.rs             # CLI entry point
    ├── motia-tools/                # ── motia CLI ──
    │   ├── Cargo.toml              # motia binary
    │   └── src/
    │       └── main.rs             # CLI entry point
    └── scaffolder-core/            # ── Shared library ──
        ├── Cargo.toml              # scaffolder-core library
        └── src/
```

## Crate Breakdown

### 1. scaffolder-core (library)

**Location:** `crates/scaffolder-core/`

Core library for project scaffolding with template support.

| Feature | Dependencies | Purpose |
|---------|-------------|---------|
| **default** | — | Basic scaffolding |
| **tui** | `cliclack`, `console`, `ctrlc` | Interactive terminal UI |

| Dependency | Purpose |
|------------|---------|
| `tokio` | Async runtime |
| `serde` | Serialization |
| `reqwest` | HTTP client (template downloads) |
| `zip` | Template archive handling |
| `clap` | CLI argument parsing |
| `walkdir` | Directory traversal |
| `cliclack` (tui) | Progress indicators, prompts |
| `console` (tui) | Terminal formatting |
| `ctrlc` (tui) | Signal handling |

### 2. iii-tools (binary)

**Location:** `crates/iii-tools/`

CLI for managing iii projects. Depends on `scaffolder-core[tui]`.

| Binary | Purpose |
|--------|---------|
| `iii-tools` | Create, manage, scaffold iii projects |

### 3. motia-tools (binary)

**Location:** `crates/motia-tools/`

CLI for managing Motia projects with iii integration. Depends on `scaffolder-core[tui]`.

| Binary | Purpose |
|--------|---------|
| `motia` | Create, manage, scaffold Motia projects |

## Configuration

**Location:** `config.yaml`

CLI configuration file for default settings, template locations, and project defaults.

## Templates

**Location:** `templates/`

Project templates used by scaffolder-core to bootstrap new iii and Motia projects. Templates are distributed as zip archives (handled via the `zip` crate).

## Key Insights

1. **Shared scaffolder-core eliminates duplication.** Both `iii-tools` and `motia` use the same scaffolding engine, meaning template formats, project creation flows, and TUI interactions are consistent across the ecosystem.

2. **TUI is an optional feature.** The `tui` feature gate (`cliclack`, `console`, `ctrlc`) means scaffolder-core can be used as a headless library by other tools, or with full interactive prompts when the `tui` feature is enabled.

3. **Remote template support via reqwest + zip.** Templates can be downloaded from remote URLs (via `reqwest`) and extracted (via `zip`), enabling centralized template distribution without requiring local template files.

## Open Questions

1. **Template format.** What is the template syntax? Are there variable substitutions, conditional sections, or hooks?

2. **iii-tools subcommands.** What specific commands does `iii-tools` provide beyond scaffolding?

3. **Motia integration.** What is Motia and how does it integrate with iii? The motia-tools CLI manages "Motia projects with iii integration" but the exact relationship needs investigation.

## Related Explorations

- [iii Engine](../iii/exploration.md) — The iii engine
- [Workers](../workers/exploration.md) — iii worker modules
- [Examples](../examples/exploration.md) — iii SDK examples

## Next Steps

1. Create `rust-revision.md` for idiomatic Rust patterns
2. Deep-dive into the scaffolder-core template engine
3. Document the iii-tools subcommand set
