# Swamp Documentation

AI Native Automation CLI framework — Built for agents, there to empower humans.

## Document Index

| # | Document | Description |
|---|----------|-------------|
| 00 | [Overview](00-overview.html) | Philosophy, core concepts, quick start |
| 01 | [Architecture](01-architecture.html) | Layered architecture, component diagram |
| 02 | [CLI Layer](02-cli-layer.html) | Commands, argument parsing, context |
| 03 | [Domain Layer](03-domain-layer.html) | Business logic, domain models |
| 04 | [Extension System](04-extension-system.html) | Extension lifecycle, registry, loading |
| 05 | [Workflow Engine](05-workflow-engine.html) | DAG execution, job scheduling |
| 06 | [Model System](06-model-system.html) | Models, methods, CalVer versioning |
| 07 | [Vault & Datastore](07-vault-datastore.html) | Secret storage, pluggable backends |
| 08 | [Claude Skills](08-skills-claude.html) | Claude Code skills, AI integration |

## Quick Links

- **Source:** `/home/darkvoid/Boxxed/@formulas/src.rust/src.llamacpp/src.Swamp/`
- **Repository:** https://github.com/systeminit/swamp.git
- **License:** AGPL v3 with Extension Exception

## What is Swamp?

Swamp is a declarative infrastructure-as-code platform that enables AI agents to create operational workflows. Key concepts:

- **Models** — Typed abstractions of external systems
- **Definitions** — YAML files instantiating model types
- **Workflows** — Orchestrated model method executions
- **Data** — Versioned, immutable outputs from method runs
- **Vaults** — Secure secret storage

## Project Structure

```
src.Swamp/
├── setup-swamp/          # GitHub Action
├── swamp/                # Core CLI (TypeScript/Deno)
└── swamp-extensions/     # Official extensions
```
