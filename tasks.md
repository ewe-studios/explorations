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
| 10 | iii Skills system | 1,156 (SKILL.md lines) | 4 docs | ✅ DONE |
| 11 | Browser SDK + Observability SDK | 4,584 | 5 docs | ✅ DONE |
| 15 | src.Uncloud | 60,523 | 12 docs | ✅ DONE |
| 14 | src.strukto-ai (Mirage) | 462,659 | 14 docs | ✅ DONE |

## Total Documented

- **Total LOC documented: ~705,000+**
- **Total documents created: 100+**
- **All grandfather reviews: PASSED**
- **All HTML generated: VERIFIED**

## Directory Structure

All documentation is under `src.AgentSandbox/`:

```
src.AgentSandbox/
├── src.iii/                    # iii ecosystem overview (16 docs)
├── src.iii-worker/             # Managed worker runtime (11 docs)
├── src.iii-filesystem/         # PassthroughFs VFS (9 docs)
├── src.iii-init/               # PID 1 init binary (6 docs)
├── src.iii-supervisor/         # In-VM process supervisor (6 docs)
├── src.iii-network/            # smoltcp TCP/IP stack (7 docs)
├── src.iii-shell-client/       # Shell exec channel (5 docs)
├── src.iii-console/            # Developer console (5 docs)
├── src.engine-workers/         # In-process workers (6 docs)
├── src.iii-skills/             # Agent-readable skills (4 docs)
├── src.iii-browser-obs/        # Browser + OTEL SDKs (5 docs)
├── src.agentfs/                # AgentFS comparison (7 docs)
└── src.Uncloud/                # Decentralized orchestration (12 docs)
```

## Skipped Tasks

### ~~TASK: Python SDK (10,884 LOC)~~
**Status:** SKIPPED per user request.

## Remaining Tasks

None — all identified tasks are complete.
