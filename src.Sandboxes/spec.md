# Sandboxes — Spec

## Source Codebase Location

- **Location:** `/home/darkvoid/Boxxed/@formulas/src.rust/src.llamacpp/src.Sandboxes/`
- **Repository:** N/A (filesystem collection of sandbox projects)
- **Languages:** Rust, TypeScript, Python, Go
- **Projects:** 9 sandbox implementations

## What This Project Is

A collection of 9 sandbox implementations spanning different approaches to secure code execution:

| Project | Language | Approach | Purpose |
|---------|----------|----------|---------|
| agent-safehouse | Shell/Bash | macOS Seatbelt | LLM agent sandboxing |
| CubeSandbox | Rust/Go | KVM microVM | High-performance isolation |
| deer-flow | Python/TS | Container-based | Super agent with sub-agents |
| flue | TypeScript | Container-based | Agent framework |
| Kami | Python/HTML | Browser-based | Document design system |
| ml-intern | Python | Container-based | ML research agent |
| shuru | Rust | MicroVM | Local sandbox for AI agents |
| superhq | Rust/GPUI | Sandboxed orchestration | AI agent platform |
| superpowers | TypeScript | Browser extension | Chrome extension for AI |

## Documentation Goal

After reading this documentation, an engineer should understand:

1. The different sandboxing approaches (Seatbelt, KVM, containers, microVMs)
2. The security/isolation spectrum from browser to bare metal
3. The use cases for each sandbox type
4. The architectural patterns common across implementations
5. Integration patterns with LLM agents
6. Performance vs security tradeoffs

## Documentation Structure

```
src.Sandboxes/
├── spec.md                      ← This file
├── exploration.md               ← Original exploration
├── markdown/
│   ├── README.md                ← Index
│   ├── 00-overview.md           ← Sandbox landscape
│   ├── 01-agent-safehouse.md    ← macOS Seatbelt
│   ├── 02-cubesandbox.md        ← KVM microVMs
│   ├── 03-deer-flow.md          ← Super agent harness
│   ├── 04-flue.md               ← Container framework
│   ├── 05-kami.md               ← Browser-based
│   ├── 06-shuru.md              ← MicroVM sandbox
│   ├── 07-superhq.md            ← Orchestration platform
│   └── 08-others.md             ← ml-intern, superpowers
├── html/
└── (uses ../build.py)
```

## Tasks

| Phase | Document | Status | Notes |
|-------|----------|--------|-------|
| 1 | Read source code | DONE | Via exploration agent |
| 2 | Create spec.md | DONE | This file |
| 3 | Write markdown files | DONE | 9 documents created |
| 4 | Generate HTML | DONE | All 9 documents generated |
| 5 | Grandfather review | DONE | ✅ Passed |

## Build System

**Script:** `../build.py`

```bash
python3 build.py src.Sandboxes
```

## Quality Requirements

All documents must meet the Iron Rules from the markdown directive.
