# Repo Explorations

A repository for systematic codebase explorations and architectural documentation generated using custom agents.

## Purpose

This repository stores detailed exploration documents for various codebases, providing:

- Comprehensive architecture documentation
- Deep dives into specific subsystems
- Cross-reference materials for engineering onboarding
- Foundation for Rust translation initiatives

## Structure

Each explored project has its own directory:

```
repo-expolorations/
├── penpot/
│   ├── exploration.md              # Main exploration document
│   ├── wasm-render-deep-dive.md    # Wasm rendering deep dive
│   ├── wasm-plugin-deep-dive.md    # Wasm plugin system deep dive
│   ├── backend-deep-dive.md        # Backend architecture deep dive
│   └── frontend-deep-dive.md       # Frontend architecture deep dive
├── [project-name]/
│   ├── exploration.md
│   └── rust-revision.md            # Rust translation (if applicable)
└── examples/
    └── [project-examples].md
```

## Explored Projects

| Project | Description | Status |
|---------|-------------|--------|
| [Penpot](./penpot/exploration.md) | Open-source design & code collaboration platform | Complete |
| | - Wasm Render Deep Dive | Complete |
| | - Wasm Plugin System Deep Dive | Complete |
| | - Backend Deep Dive | Complete |
| | - Frontend Deep Dive | Complete |

## Agents

This repository uses custom agents for systematic exploration:

| Agent | Purpose |
|-------|---------|
| **Exploration Agent** | Generate comprehensive codebase explorations with architecture diagrams |
| **Rust Revision Agent** | Translate explored projects into idiomatic Rust |

See [AGENTS.md](./AGENTS.md) for detailed agent documentation and usage.

## Workflow

1. **Explore** - Run exploration agent on target codebase
2. **Document** - Generate `exploration.md` with architecture details
3. **Deep Dive** - Create subsystem-specific deep dives for complex areas
4. **Translate** (optional) - Run Rust revision agent for translation proposals

## Usage

```bash
# Explore a codebase
/agents explore <directory_or_repo>

# Create Rust revision
/agents rust-revision <directory_or_repo>
```

## Conventions

- All exploration output is written to this repository, NOT the target project directory
- Deep dives are created for complex subsystems (Wasm, backend RPC, etc.)
- Commit messages use conventional format: `ADD:`, `FIX:`, `UPDATE:`, `REFACTOR:`
- No Claude attribution in commit messages

## License

Exploration documents are derivative works of their respective codebases. Refer to each project's original license.
