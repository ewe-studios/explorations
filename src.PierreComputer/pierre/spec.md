# PierreComputer — Spec

## Source Codebase Location

- **Location:** `/home/darkvoid/Boxxed/@formulas/src.rust/src.llamacpp/src.PierreComputer/`
- **Repository:** N/A (filesystem collection)
- **Languages:** TypeScript (primary), Go, Python
- **Total Files:** 3,595
- **Author:** The Pierre Computer Company

## What This Project Is

PierreComputer is a comprehensive software engineering ecosystem focused on code storage, version control, developer tools, and AI-powered development workflows. The collection spans multiple related projects:

- **pierre/** — Core platform monorepo with UI packages (diffs, trees, path-store)
- **sdk/** — Multi-language SDKs for code.storage (TypeScript, Python, Go)
- **just-bash/** — Virtual bash environment for AI agents with WASM support
- **just-code-storage/** — Git-flavored commands for just-bash
- **icons/** — 300+ React icon components
- **vscode-icons/** — VS Code extension
- **code-storage-skill/** — CLI installer for agent skills

## Documentation Goal

After reading this documentation, an engineer should understand:

1. The code.storage service architecture
2. The multi-language SDK design (TypeScript, Python, Go)
3. The just-bash virtual environment and WASM integration
4. The pierre monorepo structure (Bun workspaces)
5. The diff/tree UI components and shadow DOM architecture
6. The JWT-based authentication flow
7. The streaming data architecture (4MiB chunks)
8. The ephemeral branches concept

## Documentation Structure

```
src.PierreComputer/pierre/
├── spec.md                      ← This file
├── exploration.md               ← Original exploration
├── markdown/
│   ├── README.md                ← Index
│   ├── 00-overview.md           ← Ecosystem overview
│   ├── 01-code-storage.md       ← Core service
│   ├── 02-sdk.md                ← Multi-language SDKs
│   ├── 03-just-bash.md          ← Virtual bash environment
│   ├── 04-pierre-monorepo.md    ← UI components
│   └── 05-icons-vscode.md       ← Icon systems
├── html/
└── (uses ../../build.py)
```

## Tasks

| Phase | Document | Status | Notes |
|-------|----------|--------|-------|
| 1 | Read source code | DONE | Via exploration agent |
| 2 | Create spec.md | DONE | This file |
| 3 | Write markdown files | DONE | 6 documents |
| 4 | Generate HTML | DONE | All documents |
| 5 | Grandfather review | DONE | ✅ Issues fixed |

## Build System

**Script:** `../../build.py`

```bash
python3 build.py src.PierreComputer/pierre
```

## Quality Requirements

All documents must meet the Iron Rules from the markdown directive.

## Resume Point

Resume from the last uncompleted task in the Tasks table.
