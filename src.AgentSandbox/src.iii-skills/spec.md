---
source_location: /home/darkvoid/Boxxed/@formulas/src.rust/src.llamacpp/src.iii/iii/skills/
repository: part of git@github.com:iii-hq/iii (monorepo)
explored_at: 2026-06-04
documentation_goal: Document the iii Skills system — agent-readable reference material for building with the iii engine.
---

# Spec: iii Skills Documentation

## 1. Source Codebase

| Property | Value |
|----------|-------|
| Location | `iii/skills/` + `iii/new_skills/` |
| Language | Markdown (SKILL.md format) |
| License | Apache-2.0 |
| Skills | 6 top-level skills |

### Skills Catalog

| Skill | Lines | Purpose |
|-------|-------|---------|
| iii-getting-started | 226 | Install iii, create project, first worker |
| iii-core-primitives | 241 | Functions, triggers, workers, registry |
| iii-sdk-reference | 148 | Node.js, browser, Python, Rust SDK usage |
| iii-engine-config | 229 | Configure ports, workers, adapters, queues |
| iii-architecture-patterns | 202 | Workflows, reactive backends, CQRS |
| iii-error-handling | 110 | Engine/SDK errors, retryability, RBAC |

## 2. What iii Skills Are

Skills are agent-readable reference documents in SKILL.md format. They teach AI agents (Claude Code, Codex, etc.) how to build with iii — providing installable knowledge that agents can reference when writing iii code. Each skill is one folder with one SKILL.md file.

## 3. Documentation Goal

A reader should understand:
1. The SKILL.md format and how agents consume skills
2. The 6 top-level skills and their coverage
3. How skills integrate with skills-and-validation system
4. The skill installation flow (npx skills add)
5. How worker-backed capability skills stay with worker docs

## 4-9. Standard sections

| # | Document | Status |
|---|----------|--------|
| 0 | README.md | TODO |
| 1 | 00-overview.md | TODO |
| 2 | 01-skill-catalog.md | TODO |
| 3 | 02-skill-format.md | TODO |
| 4 | 03-skill-validation.md | TODO |
| 5 | Grandfather review | TODO |
| 6 | Fix findings | TODO |
| 7 | Generate HTML | TODO |

Build via `python3 build.py .`. Grandfather review mandatory.
