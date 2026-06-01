# Swamp — Spec

## Source Codebase Location

- **Location:** `/home/darkvoid/Boxxed/@formulas/src.rust/src.llamacpp/src.Swamp/`
- **Repository:** https://github.com/systeminit/swamp.git
- **Language:** TypeScript (Deno runtime)
- **Version:** Latest main branch
- **Author:** System Initiative, Inc.
- **License:** GNU Affero General Public License v3.0 with Swamp Extension and Definition Exception

## What This Project Is

Swamp is an AI Native Automation CLI framework that provides a declarative infrastructure-as-code platform. It enables AI agents to create operational workflows that are reviewable, shareable, and accurate. The core philosophy is "Built for agents, there to empower humans" — all data lives in a `.swamp/` directory within a Git repository.

Swamp uses a model-driven architecture where:
- **Models** represent typed abstractions of external systems (cloud resources, CLI tools, APIs)
- **Definitions** are YAML files that instantiate a model type with specific configuration
- **Workflows** orchestrate model method executions across parallel jobs and steps
- **Data** is versioned and immutable, produced by method runs
- **Vaults** provide secure storage for secrets referenced via CEL expressions

## Documentation Goal

After reading this documentation, an engineer should understand:

1. The layered architecture (CLI → libswamp → Domain → Infrastructure)
2. How the 30+ CLI commands are organized and implemented
3. The extension system and how to develop new extensions
4. The workflow execution engine and DAG scheduling
5. The model system with CalVer versioning
6. The vault and datastore abstractions
7. The CEL expression evaluation for dynamic values
8. The Claude Code skills integration
9. The testing strategy and evaluation framework
10. The design patterns used (Registry, Lazy Loading, Repository, Strategy)

## Documentation Structure

```
src.Swamp/swamp/
├── spec.md                      ← This file
├── exploration.md               ← Original exploration (kept for reference)
├── markdown/
│   ├── README.md                ← Index / table of contents
│   ├── 00-overview.md           ← What Swamp is, philosophy, quick architecture
│   ├── 01-architecture.md       ← Full dependency graph, layer diagram
│   ├── 02-cli-layer.md          ← CLI commands, argument parsing, context
│   ├── 03-domain-layer.md       ← Business logic, domain models
│   ├── 04-extension-system.md     ← Extension lifecycle, registry, loading
│   ├── 05-workflow-engine.md      ← DAG execution, job scheduling
│   ├── 06-model-system.md         ← Models, methods, CalVer versioning
│   ├── 07-vault-datastore.md      ← Secret storage, pluggable backends
│   └── 08-skills-claude.md        ← Claude Code skills, AI integration
├── html/
│   ├── index.html               ← Auto-generated
│   ├── styles.css               ← Shared styles
│   └── *.html                   ← Generated from markdown
└── (uses ../../build.py)
```

## Tasks

| Phase | Document | Status | Notes |
|-------|----------|--------|-------|
| 1 | Read source code | DONE | Via exploration agent |
| 2 | Create spec.md | DONE | This file |
| 3 | Write README.md | DONE | Index with navigation |
| 3 | Write 00-overview.md | DONE | Project philosophy |
| 3 | Write 01-architecture.md | DONE | Layer diagram |
| 3 | Write 02-cli-layer.md | DONE | Command structure |
| 3 | Write 03-domain-layer.md | DONE | Business logic |
| 3 | Write 04-extension-system.md | DONE | Extensions deep dive |
| 3 | Write 05-workflow-engine.md | DONE | Workflow execution |
| 3 | Write 06-model-system.md | DONE | Models deep dive |
| 3 | Write 07-vault-datastore.md | DONE | Storage systems |
| 3 | Write 08-skills-claude.md | DONE | AI integration |
| 4 | Generate HTML | DONE | All 9 documents generated |
| 5 | Grandfather review | DONE | ✅ All issues fixed |

## Build System

**Script:** `../../build.py`
**Dependencies:** Python 3.12+ (stdlib only)

```bash
# Build Swamp documentation
cd /home/darkvoid/Boxxed/@dev/repo-expolorations
python3 build.py src.Swamp/swamp

# Or build all projects
python3 build.py
```

## Quality Requirements

All documents must meet the Iron Rules from the markdown directive:

1. **Detailed sections with code snippets** — Every concept grounded in actual source code with file path references
2. **Teach key facts quickly** — First sentence of each section is its thesis
3. **Clear articulation** — No overly complex sentences
4. **Mermaid diagrams** — Minimum 2 diagrams per document
5. **Good visual assets** — Tables, ASCII art, code blocks
6. **Generated HTML** — All markdown builds to HTML
7. **Cross-references** — Link to related documents
8. **Source path references** — Include actual file paths
9. **Aha moments** — Non-obvious design decisions
10. **Navigation** — Index + prev/next buttons

## Expected Outcome

After reading the complete documentation, an engineer should be able to:

- Understand how to use Swamp for infrastructure automation
- Develop new extensions for Swamp
- Contribute to the core codebase
- Debug workflow execution issues
- Extend the CLI with new commands
- Integrate Swamp into CI/CD pipelines
- Understand the design decisions behind the architecture

## Resume Point

If interrupted, resume at the last uncompleted task in the Tasks table above. The source of truth is the filesystem — check what markdown files exist in `markdown/` and continue from there.
