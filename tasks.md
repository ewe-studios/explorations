# Todo - Exploration Tasks Status

## CRITICAL: Depth Requirement

All must follow our markdown engineering directive, writing the markdown and building the html with ./markdown_engineering/documentation_directive.md and build.py. Each must be detailed, not light, must be deep, pull the AHA! moments and be detailed and clear so someone junior in technical expertise can understand things fully and properly. Write fundamental documentation files to help engineers level up quickly with the gaps, ideas, technical design, data structures and processes used. See examples like ./pi and ./hermes. Follow the documentation directive.

**CRITICAL: Depth is non-negotiable.** A 200-line markdown for a large, multi-file project is unacceptable. The purpose of exploration is to teach. Read every source file. Document every significant function, type, algorithm, and data structure. Include actual code snippets with file paths. Length is not a constraint — write as much as needed to fully teach the project. If a project needs 50 pages, write 50 pages. Short documents are a failure of thoroughness, not a virtue. Grandfather review is mandatory, not optional.

Do it one by one, ensure you finished each, done grandfather review and have fixed all issues before moving to the next one.

## Completed Tasks

| # | Task | LOC | Documents | Status |
|---|------|-----|-----------|--------|
| 1 | src.iii ecosystem | 85,119 (engine) + 9 subprojects | 16 docs | ✅ DONE |
| 2 | src.iii-worker | 42,998 | 11 docs | ✅ DONE |
| 3 | src.iii-filesystem | 4,421 | 9 docs | ✅ DONE |
| 4 | src.iii-init | 6,429 | 6 docs | ✅ DONE |
| 5 | src.iii-supervisor | 1,201 | 6 docs | ✅ DONE |
| 6 | src.iii-network | 2,661 | 7 docs | ✅ DONE |
| 7 | src.iii-shell-client + proto | 2,183 | 5 docs | ✅ DONE |
| 8 | src.iii-console | 18,771 | 5 docs | ✅ DONE |
| 9 | Engine workers deep dive | 13,129 | 6 docs | ✅ DONE |
| 15 | **src.Uncloud** | **60,523** | **12 docs** | **✅ DONE** |

## Sequential Task List (Remaining)

### TASK 14: [src.strukto-ai]/ — Comprehensive Documentation
**Location:** `src.strukto-ai/`
**Status:** NOT STARTED. Per tasks.md item 2 — needs full exploration following directive.

### TASK 12: Skills System
**Location:** `iii/skills/`, `iii/new_skills/`
**Status:** NOT STARTED. Agent-readable reference material, SKILL.md format, skills-and-validation integration.

### TASK 11: Node browser + obs SDK (6,500 LOC)
**Location:** `iii/sdk/packages/node/iii-browser/` + `node/observability/`
**Status:** NOT STARTED. Browser SDK and observability SDK.

## Skipped Tasks

### ~~TASK 10: Python SDK (10,884 LOC)~~
**Status:** SKIPPED per user request.
