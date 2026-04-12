# Kody MCP Assistant — Exploration Task List

**Project:** kody  
**Source:** `/home/darkvoid/Boxxed/@formulas/src.rust/src.llamacpp/src.AIResearch/kody`  
**Created:** 2026-04-12  
**Total TypeScript Files:** ~400 across 4 package directories

---

## Project Summary

Kody is an experimental personal assistant platform built on Cloudflare Workers and the Model Context Protocol (MCP). It ships a Remix UI, Worker-based request routing, chat-agent plumbing, and OAuth-protected MCP endpoints.

**Key characteristics:**
- Single-user personal assistant (built for `me@kentcdodds.com`)
- MCP-first architecture with compact tool surface
- Remix 3 (alpha) UI on Cloudflare Workers
- Nx monorepo structure

---

## Exploration Guidelines (Updated after Grandfather Review)

**REVIEW FINDING:** Initial `exploration.md` was HIGH-LEVEL only (~40% depth). Missing code-level substance.

### Required Depth for All Explorations

Each exploration document **MUST** include:

| # | Requirement | What It Means |
|---|-------------|---------------|
| 1 | **Module Overview** | Purpose and responsibilities (2-3 paragraphs) |
| 2 | **File Inventory** | **ALL** files with **actual line counts** and descriptions |
| 3 | **Key Exports** | Main functions, classes, interfaces, types **with signatures** |
| 4 | **Line-by-Line Analysis** | **Critical code sections** (50+ line blocks) with explanations |
| 5 | **Component Relationships** | Import/dependency graph, not just high-level boxes |
| 6 | **Data Flow** | Input/output patterns, state changes **with code examples** |
| 7 | **Key Patterns** | Design patterns **with code snippets** showing implementation |
| 8 | **Integration Points** | Dependencies **AND** dependents **with file references** |
| 9 | **Error Handling** | Error types, propagation paths, **recovery strategies** |
| 10 | **Testing Strategy** | Test files, fixtures, **coverage gaps identified** |

### Substance Requirements (Enforced)

- **No hand-waving:** "350 lines" → paste and explain the 350 lines
- **No partial inventories:** "256 files in worker/" → all 256 listed or explain why skipped
- **Code blocks required:** Every claim backed by actual code
- **Export signatures:** `function foo(x: T): R` not just "foo function"

---

## Deep-Dive Documents Required

**REVIEW FINDING:** Complex subsystems need dedicated deep-dive files.

The following deep-dive documents **MUST** be created and referenced from main `exploration.md`:

| Deep-Dive | Source File(s) | Lines | Priority |
|-----------|----------------|-------|----------|
| `mcp-search-deep-dive.md` | `packages/worker/src/mcp/tools/search.ts` | 851 | CRITICAL |
| `code-mode-executor-deep-dive.md` | `packages/worker/src/mcp/executor.ts` | ~400 | CRITICAL |
| `oauth-flow-deep-dive.md` | `packages/worker/src/oauth-handlers.ts` | 350 | CRITICAL |
| `capability-registry-deep-dive.md` | `packages/worker/src/mcp/capabilities/build-capability-registry.ts` | 151 | HIGH |
| `chat-agent-deep-dive.md` | `packages/worker/src/chat-agent.ts` | 434 | HIGH |
| `home-connector-adapters-deep-dive.md` | `packages/home-connector/adapters/*/` | ~600 | MEDIUM |

---

## Phase 1: Core Worker (Highest Priority)

**REVIEW FINDING:** Initial exploration covered only ~12 files of 256. Missing code-level analysis.

### 1.1 packages/worker/ — Main Application

**Priority:** CRITICAL — Entry point, routing, MCP server, OAuth

**Substance Requirements:**
- **ALL 256 files** must be inventoried with actual line counts (no `~estimates`)
- **Critical files** (marked below) require line-by-line analysis with code blocks
- **Key exports** must include function/class signatures

| # | Sub-directory/File | Priority | Lines | Analysis Depth | Status |
|---|-------------------|----------|-------|----------------|--------|
| 1.1.1 | `src/index.ts` | CRITICAL | 292 | Line-by-line (full) | [ ] |
| 1.1.2 | `src/app/handler.ts` | CRITICAL | 15 | Key exports only | [ ] |
| 1.1.3 | `src/app/router.ts` | CRITICAL | 121 | Key exports + flow | [ ] |
| 1.1.4 | `src/app/routes.ts` | HIGH | 34 | Key exports only | [ ] |
| 1.1.5 | `src/oauth-handlers.ts` | CRITICAL | 350 | Line-by-line (full) | [ ] |
| 1.1.6 | `src/mcp-auth.ts` | HIGH | 121 | Key exports + validation logic | [ ] |
| 1.1.7 | `src/mcp/index.ts` | CRITICAL | 73 | Line-by-line (full) | [ ] |
| 1.1.8 | `src/mcp/register-tools.ts` | CRITICAL | TBD | Line-by-line (full) | [ ] |
| 1.1.9 | `src/mcp/register-resources.ts` | HIGH | TBD | Key exports | [ ] |
| 1.1.10 | `src/mcp/executor.ts` | CRITICAL | TBD | Line-by-line → deep-dive | [ ] |
| 1.1.11 | `src/mcp/tools/search.ts` | CRITICAL | 851 | Line-by-line → deep-dive | [ ] |
| 1.1.12 | `src/mcp/tools/execute.ts` | CRITICAL | TBD | Line-by-line → deep-dive | [ ] |
| 1.1.13 | `src/mcp/tools/open_generated_ui.ts` | HIGH | TBD | Key exports | [ ] |
| 1.1.14 | `src/mcp/capabilities/registry.ts` | HIGH | 62 | Key exports + patterns | [ ] |
| 1.1.15 | `src/mcp/capabilities/builtin-domains.ts` | HIGH | 21 | Full content | [ ] |
| 1.1.16 | `src/mcp/capabilities/build-capability-registry.ts` | HIGH | 151 | Line-by-line | [ ] |
| 1.1.17 | `src/mcp/capabilities/apps/domain.ts` | MEDIUM | TBD | Full content | [ ] |
| 1.1.18 | `src/mcp/capabilities/coding/domain.ts` | MEDIUM | TBD | Full content | [ ] |
| 1.1.19 | `src/mcp/capabilities/meta/domain.ts` | MEDIUM | TBD | Full content | [ ] |
| 1.1.20 | `src/mcp/capabilities/secrets/domain.ts` | MEDIUM | TBD | Full content | [ ] |
| 1.1.21 | `src/mcp/capabilities/values/domain.ts` | MEDIUM | TBD | Full content | [ ] |
| 1.1.22 | `src/mcp/capabilities/home/domain.ts` | MEDIUM | TBD | Full content | [ ] |
| 1.1.23 | `src/mcp/observability.ts` | MEDIUM | TBD | Key exports | [ ] |
| 1.1.24 | `src/mcp/fetch-gateway.ts` | MEDIUM | TBD | Key exports | [ ] |
| 1.1.25 | `src/mcp/context.ts` | MEDIUM | TBD | Key exports | [ ] |
| 1.1.26 | `src/mcp/server-instructions.ts` | MEDIUM | TBD | Full content | [ ] |
| 1.1.27 | `src/chat-agent.ts` | CRITICAL | 434 | Line-by-line → deep-dive | [ ] |
| 1.1.28 | `src/chat-agent-routing.ts` | HIGH | TBD | Key exports | [ ] |
| 1.1.29 | `src/handler.ts` | HIGH | TBD | Key exports | [ ] |
| 1.1.30 | `src/router.ts` | HIGH | TBD | Key exports | [ ] |
| 1.1.31 | `src/routes.ts` | HIGH | 34 | Full content | [ ] |
| 1.1.32 | `src/d1-data-table-adapter.ts` | MEDIUM | TBD | Key exports | [ ] |
| 1.1.33 | `src/db.ts` | MEDIUM | TBD | Key exports | [ ] |
| 1.1.34 | `src/env-schema.ts` | HIGH | TBD | Full content + validation | [ ] |
| 1.1.35 | `src/sentry-options.ts` | LOW | TBD | Key exports | [ ] |
| 1.1.36 | `src/utils.ts` | MEDIUM | TBD | Key exports | [ ] |
| 1.1.37 | `src/user-id.ts` | MEDIUM | TBD | Full content | [ ] |
| 1.1.38 | `src/capability-maintenance.ts` | LOW | TBD | Key exports | [ ] |
| 1.1.39 | `src/memory-maintenance.ts` | LOW | TBD | Key exports | [ ] |
| 1.1.40 | `src/skill-maintenance.ts` | LOW | TBD | Key exports | [ ] |
| 1.1.41 | `src/ui-artifact-maintenance.ts` | LOW | TBD | Key exports | [ ] |
| 1.1.42 | `src/ui-artifact-urls.ts` | LOW | TBD | Full content | [ ] |
| 1.1.43 | `wrangler.jsonc` | HIGH | 200 | Full content + explanations | [ ] |
| 1.1.44 | `worker-configuration.d.ts` | MEDIUM | ~8000 | Key types only | [ ] |

**DEEP-DIVES REQUIRED (create separate files):**
- `kody/mcp-search-deep-dive.md` — Full 851-line analysis of `search.ts`
- `kody/code-mode-executor-deep-dive.md` — Full analysis of `executor.ts`
- `kody/oauth-flow-deep-dive.md` — Full 350-line analysis of `oauth-handlers.ts`
- `kody/chat-agent-deep-dive.md` — Full 434-line analysis of `chat-agent.ts`
- `kody/capability-registry-deep-dive.md` — Full analysis of `build-capability-registry.ts`

**Output:** `kody/exploration.md` (main) + `kody/*-deep-dive.md` (subsystems)

---

## Phase 2-9: Remaining Exploration Tasks

The remaining phases should follow the same **substance requirements** as Phase 1:

| Phase | Output File | Key Files | Substance Requirements |
|-------|-------------|-----------|----------------------|
| 2. Shared | `exploration.md` (shared section) | `packages/shared/src/*.ts` | All exports, line counts |
| 3. Home Connector | `home-connector-adapters-deep-dive.md` | `packages/home-connector/adapters/*/` | Full adapter analysis |
| 4. Mock Servers | `exploration.md` (mocks section) | `packages/mock-servers/*/` | Mock configurations |
| 5. Documentation | `exploration.md` (docs section) | `docs/contributing/*.md` | Key architecture docs summarized |
| 6. Remix Patterns | `exploration.md` (remix section) | `docs/contributing/remix/` | Pattern summaries |
| 7. MCP Usage | `exploration.md` (usage section) | `docs/use/*.md` | MCP usage flows |
| 8. E2E Tests | `exploration.md` (tests section) | `e2e/*.spec.ts` | Test coverage analysis |
| 9. Root Config | `exploration.md` (config section) | `*.json`, `*.ts` | Config explanations |

---

## Progress Tracking

### Summary

| Phase | Directories | Files | Priority | Status |
|-------|-------------|-------|----------|--------|
| 1. Core Worker | 1 | 44+ | CRITICAL | [ ] |
| 2. Shared | 1 | ~5 | HIGH | [ ] |
| 3. Home Connector | 1 | ~10 | MEDIUM | [ ] |
| 4. Mock Servers | 1 | ~5 | LOW | [ ] |
| 5. Documentation | 1 | ~16 | HIGH | [ ] |
| 6. Remix Patterns | 1 | ~9 | MEDIUM | [ ] |
| 7. MCP Usage | 1 | ~1 | HIGH | [ ] |
| 8. E2E Tests | 1 | ~10 | MEDIUM | [ ] |
| 9. Root Config | 1 | ~9 | HIGH | [ ] |
| **Deep-Dives** | **5** | **5** | **CRITICAL** | **[ ]** |
| **Total** | **9** | **~114** | | |

### Completion Checklist

#### Main Exploration
- [ ] Phase 1: Core Worker (packages/worker/) — 44 files
- [ ] Phase 2: Shared Modules (packages/shared/)
- [ ] Phase 3: Home Connector (packages/home-connector/)
- [ ] Phase 4: Mock Servers (packages/mock-servers/)
- [ ] Phase 5: Documentation (docs/contributing/)
- [ ] Phase 6: Remix Patterns (docs/contributing/remix/)
- [ ] Phase 7: MCP Usage (docs/use/)
- [ ] Phase 8: E2E Tests (e2e/)
- [ ] Phase 9: Root Configuration

#### Deep-Dive Documents (Required for Substance)
- [ ] `mcp-search-deep-dive.md` — 851 lines analysis
- [ ] `code-mode-executor-deep-dive.md` — Execute sandbox analysis
- [ ] `oauth-flow-deep-dive.md` — 350 lines OAuth analysis
- [ ] `capability-registry-deep-dive.md` — Registry build analysis
- [ ] `chat-agent-deep-dive.md` — 434 lines Chat DO analysis

### Grandfather Review Checklist

Before marking any phase complete, verify:

- [ ] **File inventory complete** — All files listed with actual line counts
- [ ] **Key exports documented** — Function/class signatures included
- [ ] **Line-by-line analysis** — Critical files have code blocks with explanations
- [ ] **Error handling traced** — Error types and propagation paths documented
- [ ] **Component relationships** — Import/dependency graph included
- [ ] **Data flow shown** — Input/output patterns with code examples
- [ ] **Patterns documented** — Design patterns with implementation snippets
- [ ] **Tests analyzed** — Test files and coverage gaps identified

---

## Output Structure

All explorations should be saved to `./kody/` in this repository:

```
/home/darkvoid/Boxxed/@dev/repo-expolorations/kody/
├── task.md                     # This file
├── exploration.md              # Main exploration document
├── mcp-search-deep-dive.md     # Search tool (851 lines)
├── code-mode-executor-deep-dive.md  # Execute tool
├── oauth-flow-deep-dive.md     # OAuth handlers
├── capability-registry-deep-dive.md  # Registry build
├── chat-agent-deep-dive.md     # Chat agent DO
└── home-connector-adapters-deep-dive.md  # Home adapters
```

---

**Last Updated:** 2026-04-12 (Grandfather Review Applied — Substance Requirements Added)
