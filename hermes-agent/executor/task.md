# Executor Deep Dive — Exploration Task List

**Project:** Executor  
**Source:** `/home/darkvoid/Boxxed/@formulas/src.rust/src.llamacpp/src.ClaudOpen/executor`  
**Created:** 2026-04-07  
**Total TypeScript Files:** 314+ across 50+ directories

---

## Exploration Guidelines

Each exploration must include:

1. **Module Overview** — Purpose and responsibilities
2. **File Inventory** — All files with line counts and descriptions
3. **Key Exports** — Main functions, classes, interfaces, types
4. **Line-by-Line Analysis** — Critical code sections with explanations
5. **Component Relationships** — How this module interacts with others
6. **Data Flow** — Input/output patterns, state changes
7. **Key Patterns** — Design patterns and architectural decisions
8. **Integration Points** — Dependencies and dependents
9. **Error Handling** — How errors are caught and propagated
10. **Testing Strategy** — Unit tests, integration tests, fixtures

---

## Phase 1: Core SDK (Highest Priority)

### 1.1 packages/core/sdk/ — **23 files**

**Priority:** CRITICAL — Main public API

| # | Sub-directory/File | Priority | Lines (est.) | Status |
|---|-------------------|----------|--------------|--------|
| 1.1.1 | `src/executor.ts` | CRITICAL | ~230 | [ ] |
| 1.1.2 | `src/tools.ts` | CRITICAL | ~400 | [ ] |
| 1.1.3 | `src/sources.ts` | CRITICAL | ~300 | [ ] |
| 1.1.4 | `src/secrets.ts` | CRITICAL | ~200 | [ ] |
| 1.1.5 | `src/policies.ts` | CRITICAL | ~250 | [ ] |
| 1.1.6 | `src/scope.ts` | CRITICAL | ~100 | [ ] |
| 1.1.7 | `src/plugin.ts` | CRITICAL | ~150 | [ ] |
| 1.1.8 | `src/elicitation.ts` | CRITICAL | ~200 | [ ] |
| 1.1.9 | `src/ids.ts` | HIGH | ~50 | [ ] |
| 1.1.10 | `src/errors.ts` | HIGH | ~200 | [ ] |
| 1.1.11 | `src/schema-refs.ts` | MEDIUM | ~150 | [ ] |
| 1.1.12 | `src/schema-types.ts` | MEDIUM | ~150 | [ ] |
| 1.1.13 | `src/schema-types.test.ts` | MEDIUM | ~100 | [ ] |
| 1.1.14 | `src/runtime-tools.ts` | MEDIUM | ~100 | [ ] |
| 1.1.15 | `src/testing.ts` | LOW | ~150 | [ ] |
| 1.1.16 | `src/in-memory/policy-engine.ts` | HIGH | ~200 | [ ] |
| 1.1.17 | `src/in-memory/secret-store.ts` | HIGH | ~200 | [ ] |
| 1.1.18 | `src/in-memory/tool-registry.ts` | HIGH | ~200 | [ ] |
| 1.1.19 | `src/plugins/in-memory-tools.ts` | MEDIUM | ~150 | [ ] |
| 1.1.20 | `src/plugin-kv.ts` | MEDIUM | ~100 | [ ] |
| 1.1.21 | `src/index.ts` | HIGH | ~100 | [ ] |
| 1.1.22 | `src/index.test.ts` | MEDIUM | ~150 | [ ] |
| 1.1.23 | `vitest.config.ts` | LOW | ~20 | [ ] |

**Output:** `executor/packages/core/sdk/exploration.md`

---

### 1.2 packages/core/execution/ — **7 files**

**Priority:** CRITICAL — Code execution engine

| # | Sub-directory/File | Priority | Lines (est.) | Status |
|---|-------------------|----------|--------------|--------|
| 1.2.1 | `src/engine.ts` | CRITICAL | ~347 | [ ] |
| 1.2.2 | `src/tool-invoker.ts` | CRITICAL | ~200 | [ ] |
| 1.2.3 | `src/tool-invoker.test.ts` | HIGH | ~150 | [ ] |
| 1.2.4 | `src/description.ts` | HIGH | ~150 | [ ] |
| 1.2.5 | `src/errors.ts` | HIGH | ~100 | [ ] |
| 1.2.6 | `src/index.ts` | MEDIUM | ~50 | [ ] |
| 1.2.7 | `vitest.config.ts` | LOW | ~20 | [ ] |

**Output:** `executor/packages/core/execution/exploration.md`

---

### 1.3 packages/core/api/ — **8 files**

**Priority:** HIGH — REST API client layer

| # | Sub-directory/File | Priority | Lines (est.) | Status |
|---|-------------------|----------|--------------|--------|
| 1.3.1 | `src/api.ts` | HIGH | ~300 | [ ] |
| 1.3.2 | `src/errors.ts` | HIGH | ~150 | [ ] |
| 1.3.3 | `src/index.ts` | MEDIUM | ~50 | [ ] |
| 1.3.4 | `src/executions/api.ts` | HIGH | ~200 | [ ] |
| 1.3.5 | `src/scope/api.ts` | MEDIUM | ~150 | [ ] |
| 1.3.6 | `src/secrets/api.ts` | MEDIUM | ~150 | [ ] |
| 1.3.7 | `src/sources/api.ts` | MEDIUM | ~150 | [ ] |
| 1.3.8 | `src/tools/api.ts` | HIGH | ~200 | [ ] |

**Output:** `executor/packages/core/api/exploration.md`

---

### 1.4 packages/core/config/ — **7 files**

**Priority:** HIGH — Configuration management

| # | Sub-directory/File | Priority | Lines (est.) | Status |
|---|-------------------|----------|--------------|--------|
| 1.4.1 | `src/schema.ts` | HIGH | ~200 | [ ] |
| 1.4.2 | `src/config-store.ts` | HIGH | ~200 | [ ] |
| 1.4.3 | `src/load.ts` | HIGH | ~150 | [ ] |
| 1.4.4 | `src/write.ts` | MEDIUM | ~100 | [ ] |
| 1.4.5 | `src/index.ts` | MEDIUM | ~50 | [ ] |
| 1.4.6 | `src/config.test.ts` | MEDIUM | ~150 | [ ] |
| 1.4.7 | `vitest.config.ts` | LOW | ~20 | [ ] |

**Output:** `executor/packages/core/config/exploration.md`

---

### 1.5 packages/core/storage-file/ — **11 files**

**Priority:** HIGH — SQLite persistence

| # | Sub-directory/File | Priority | Lines (est.) | Status |
|---|-------------------|----------|--------------|--------|
| 1.5.1 | `src/schema.ts` | HIGH | ~300 | [ ] |
| 1.5.2 | `src/secret-store.ts` | HIGH | ~200 | [ ] |
| 1.5.3 | `src/policy-engine.ts` | HIGH | ~250 | [ ] |
| 1.5.4 | `src/plugin-kv.ts` | MEDIUM | ~150 | [ ] |
| 1.5.5 | `src/index.ts` | MEDIUM | ~50 | [ ] |
| 1.5.6 | `src/index.test.ts` | MEDIUM | ~150 | [ ] |
| 1.5.7 | `src/migrations/index.ts` | MEDIUM | ~100 | [ ] |
| 1.5.8 | `src/migrations/0001_initial.ts` | LOW | ~200 | [ ] |
| 1.5.9 | `vitest.config.ts` | LOW | ~20 | [ ] |

**Output:** `executor/packages/core/storage-file/exploration.md`

---

### 1.6 packages/core/storage-postgres/ — **11 files**

**Priority:** MEDIUM — PostgreSQL persistence (alternative)

| # | Sub-directory/File | Priority | Lines (est.) | Status |
|---|-------------------|----------|--------------|--------|
| 1.6.1 | `src/schema.ts` | HIGH | ~300 | [ ] |
| 1.6.2 | `src/secret-store.ts` | HIGH | ~200 | [ ] |
| 1.6.3 | `src/policy-engine.ts` | HIGH | ~250 | [ ] |
| 1.6.4 | `src/index.ts` | MEDIUM | ~50 | [ ] |
| 1.6.5 | `drizzle/schema.ts` | MEDIUM | ~200 | [ ] |
| 1.6.6 | `drizzle/meta/*.ts` | LOW | ~100 | [ ] |

**Output:** `executor/packages/core/storage-postgres/exploration.md`

---

## Phase 2: Kernel (Core Runtime)

### 2.1 packages/kernel/core/ — **5 files**

**Priority:** CRITICAL — Core kernel logic

| # | Sub-directory/File | Priority | Lines (est.) | Status |
|---|-------------------|----------|--------------|--------|
| 2.1.1 | `src/engine.ts` | CRITICAL | ~400 | [ ] |
| 2.1.2 | `src/tool-invoker.ts` | CRITICAL | ~300 | [ ] |
| 2.1.3 | `src/index.ts` | HIGH | ~50 | [ ] |
| 2.1.4 | `vitest.config.ts` | LOW | ~20 | [ ] |

**Output:** `executor/packages/kernel/core/exploration.md`

---

### 2.2 packages/kernel/ir/ — **3 files**

**Priority:** HIGH — Intermediate representation

| # | Sub-directory/File | Priority | Lines (est.) | Status |
|---|-------------------|----------|--------------|--------|
| 2.2.1 | `src/types.ts` | HIGH | ~200 | [ ] |
| 2.2.2 | `src/transform.ts` | HIGH | ~250 | [ ] |
| 2.2.3 | `src/index.ts` | MEDIUM | ~50 | [ ] |

**Output:** `executor/packages/kernel/ir/exploration.md`

---

### 2.3 packages/kernel/runtime-quickjs/ — **2 files**

**Priority:** HIGH — QuickJS sandbox

| # | Sub-directory/File | Priority | Lines (est.) | Status |
|---|-------------------|----------|--------------|--------|
| 2.3.1 | `src/index.ts` | HIGH | ~300 | [ ] |
| 2.3.2 | `vitest.config.ts` | LOW | ~20 | [ ] |

**Output:** `executor/packages/kernel/runtime-quickjs/exploration.md`

---

### 2.4 packages/kernel/runtime-deno-subprocess/ — **4 files**

**Priority:** MEDIUM — Deno subprocess runtime

| # | Sub-directory/File | Priority | Lines (est.) | Status |
|---|-------------------|----------|--------------|--------|
| 2.4.1 | `src/index.ts` | HIGH | ~250 | [ ] |
| 2.4.2 | `src/subprocess.ts` | HIGH | ~200 | [ ] |
| 2.4.3 | `vitest.config.ts` | LOW | ~20 | [ ] |

**Output:** `executor/packages/kernel/runtime-deno-subprocess/exploration.md`

---

## Phase 3: Source Plugins

### 3.1 packages/plugins/openapi/ — **27 files**

**Priority:** CRITICAL — OpenAPI source plugin

| # | Sub-directory/File | Priority | Lines (est.) | Status |
|---|-------------------|----------|--------------|--------|
| 3.1.1 | `src/api/source.ts` | CRITICAL | ~400 | [ ] |
| 3.1.2 | `src/api/parser.ts` | CRITICAL | ~350 | [ ] |
| 3.1.3 | `src/api/auth.ts` | HIGH | ~250 | [ ] |
| 3.1.4 | `src/sdk/index.ts` | CRITICAL | ~300 | [ ] |
| 3.1.5 | `src/sdk/tool-builder.ts` | CRITICAL | ~350 | [ ] |
| 3.1.6 | `src/sdk/invoker.ts` | HIGH | ~250 | [ ] |
| 3.1.7 | `src/react/index.ts` | MEDIUM | ~150 | [ ] |
| 3.1.8 | `src/react/hooks.ts` | MEDIUM | ~200 | [ ] |
| 3.1.9 | `fixtures/*.ts` | LOW | ~300 | [ ] |

**Output:** `executor/packages/plugins/openapi/exploration.md`

---

### 3.2 packages/plugins/graphql/ — **23 files**

**Priority:** HIGH — GraphQL source plugin

| # | Sub-directory/File | Priority | Lines (est.) | Status |
|---|-------------------|----------|--------------|--------|
| 3.2.1 | `src/api/source.ts` | HIGH | ~350 | [ ] |
| 3.2.2 | `src/api/parser.ts` | HIGH | ~300 | [ ] |
| 3.2.3 | `src/api/introspection.ts` | HIGH | ~250 | [ ] |
| 3.2.4 | `src/sdk/index.ts` | HIGH | ~250 | [ ] |
| 3.2.5 | `src/sdk/tool-builder.ts` | HIGH | ~300 | [ ] |
| 3.2.6 | `src/sdk/invoker.ts` | MEDIUM | ~200 | [ ] |
| 3.2.7 | `src/react/index.ts` | LOW | ~100 | [ ] |
| 3.2.8 | `src/react/hooks.ts` | LOW | ~150 | [ ] |

**Output:** `executor/packages/plugins/graphql/exploration.md`

---

### 3.3 packages/plugins/mcp/ — **23 files**

**Priority:** HIGH — MCP source plugin

| # | Sub-directory/File | Priority | Lines (est.) | Status |
|---|-------------------|----------|--------------|--------|
| 3.3.1 | `src/api/source.ts` | HIGH | ~350 | [ ] |
| 3.3.2 | `src/api/client.ts` | HIGH | ~300 | [ ] |
| 3.3.3 | `src/api/protocol.ts` | HIGH | ~250 | [ ] |
| 3.3.4 | `src/sdk/index.ts` | HIGH | ~250 | [ ] |
| 3.3.5 | `src/sdk/tool-discovery.ts` | HIGH | ~250 | [ ] |
| 3.3.6 | `src/sdk/invoker.ts` | MEDIUM | ~200 | [ ] |
| 3.3.7 | `src/react/index.ts` | LOW | ~100 | [ ] |
| 3.3.8 | `src/react/hooks.ts` | LOW | ~150 | [ ] |

**Output:** `executor/packages/plugins/mcp/exploration.md`

---

### 3.4 packages/plugins/google-discovery/ — **21 files**

**Priority:** MEDIUM — Google APIs plugin

| # | Sub-directory/File | Priority | Lines (est.) | Status |
|---|-------------------|----------|--------------|--------|
| 3.4.1 | `src/api/source.ts` | MEDIUM | ~300 | [ ] |
| 3.4.2 | `src/api/parser.ts` | MEDIUM | ~250 | [ ] |
| 3.4.3 | `src/api/auth.ts` | HIGH | ~300 | [ ] |
| 3.4.4 | `src/sdk/index.ts` | MEDIUM | ~200 | [ ] |
| 3.4.5 | `src/sdk/tool-builder.ts` | MEDIUM | ~250 | [ ] |
| 3.4.6 | `src/sdk/invoker.ts` | LOW | ~150 | [ ] |
| 3.4.7 | `src/react/index.ts` | LOW | ~100 | [ ] |
| 3.4.8 | `src/react/hooks.ts` | LOW | ~150 | [ ] |
| 3.4.9 | `fixtures/*.ts` | LOW | ~200 | [ ] |

**Output:** `executor/packages/plugins/google-discovery/exploration.md`

---

### 3.5 packages/plugins/onepassword/ — **13 files**

**Priority:** LOW — 1Password secret provider

| # | Sub-directory/File | Priority | Lines (est.) | Status |
|---|-------------------|----------|--------------|--------|
| 3.5.1 | `src/api/client.ts` | MEDIUM | ~200 | [ ] |
| 3.5.2 | `src/api/auth.ts` | MEDIUM | ~150 | [ ] |
| 3.5.3 | `src/sdk/index.ts` | MEDIUM | ~150 | [ ] |
| 3.5.4 | `src/sdk/provider.ts` | HIGH | ~200 | [ ] |
| 3.5.5 | `src/react/index.ts` | LOW | ~100 | [ ] |
| 3.5.6 | `src/react/hooks.ts` | LOW | ~100 | [ ] |

**Output:** `executor/packages/plugins/onepassword/exploration.md`

---

### 3.6 packages/plugins/keychain/ — **5 files**

**Priority:** LOW — System keychain provider

| # | Sub-directory/File | Priority | Lines (est.) | Status |
|---|-------------------|----------|--------------|--------|
| 3.6.1 | `src/provider.ts` | MEDIUM | ~200 | [ ] |
| 3.6.2 | `src/index.ts` | LOW | ~50 | [ ] |

**Output:** `executor/packages/plugins/keychain/exploration.md`

---

### 3.7 packages/plugins/file-secrets/ — **1 file**

**Priority:** LOW — File-based secret storage

| # | Sub-directory/File | Priority | Lines (est.) | Status |
|---|-------------------|----------|--------------|--------|
| 3.7.1 | `src/index.ts` | LOW | ~150 | [ ] |

**Output:** `executor/packages/plugins/file-secrets/exploration.md`

---

## Phase 4: Hosts

### 4.1 packages/hosts/mcp/ — **3 files**

**Priority:** HIGH — MCP server host

| # | Sub-directory/File | Priority | Lines (est.) | Status |
|---|-------------------|----------|--------------|--------|
| 4.1.1 | `src/server.ts` | HIGH | ~300 | [ ] |
| 4.1.2 | `src/protocol.ts` | HIGH | ~250 | [ ] |
| 4.1.3 | `src/index.ts` | MEDIUM | ~50 | [ ] |

**Output:** `executor/packages/hosts/mcp/exploration.md`

---

## Phase 5: UI Components

### 5.1 packages/ui/ — **~20 files**

**Priority:** MEDIUM — Shared UI components

| # | Sub-directory/File | Priority | Lines (est.) | Status |
|---|-------------------|----------|--------------|--------|
| 5.1.1 | `src/components/` | MEDIUM | ~500 | [ ] |
| 5.1.2 | `src/hooks/` | LOW | ~200 | [ ] |
| 5.1.3 | `src/lib/` | LOW | ~150 | [ ] |
| 5.1.4 | `src/styles/` | LOW | ~100 | [ ] |

**Output:** `executor/packages/ui/exploration.md`

---

### 5.2 packages/clients/react/ — **7 files**

**Priority:** MEDIUM — React client library

| # | Sub-directory/File | Priority | Lines (est.) | Status |
|---|-------------------|----------|--------------|--------|
| 5.2.1 | `src/client.ts` | HIGH | ~250 | [ ] |
| 5.2.2 | `src/index.ts` | MEDIUM | ~50 | [ ] |
| 5.2.3 | `src/atoms.ts` | MEDIUM | ~150 | [ ] |
| 5.2.4 | `src/use-scope.ts` | MEDIUM | ~150 | [ ] |
| 5.2.5 | `src/secret-provider-plugin.ts` | LOW | ~100 | [ ] |
| 5.2.6 | `src/source-plugin.ts` | LOW | ~100 | [ ] |

**Output:** `executor/packages/clients/react/exploration.md`

---

## Phase 6: Applications

### 6.1 apps/cli/ — **6 files**

**Priority:** HIGH — CLI application

| # | Sub-directory/File | Priority | Lines (est.) | Status |
|---|-------------------|----------|--------------|--------|
| 6.1.1 | `bin/executor.ts` | HIGH | ~50 | [ ] |
| 6.1.2 | `src/main.ts` | CRITICAL | ~400 | [ ] |
| 6.1.3 | `src/build.ts` | MEDIUM | ~150 | [ ] |
| 6.1.4 | `src/release.ts` | MEDIUM | ~150 | [ ] |
| 6.1.5 | `src/embedded-web-ui.ts` | MEDIUM | ~200 | [ ] |
| 6.1.6 | `src/embedded-web-ui.gen.ts` | LOW | ~100 | [ ] |

**Output:** `executor/apps/cli/exploration.md`

---

### 6.2 apps/server/ — **~15 files**

**Priority:** HIGH — Backend server

| # | Sub-directory/File | Priority | Lines (est.) | Status |
|---|-------------------|----------|--------------|--------|
| 6.2.1 | `src/index.ts` | HIGH | ~100 | [ ] |
| 6.2.2 | `src/main.ts` | HIGH | ~200 | [ ] |
| 6.2.3 | `src/mcp.ts` | HIGH | ~250 | [ ] |
| 6.2.4 | `src/handlers/executions.ts` | HIGH | ~200 | [ ] |
| 6.2.5 | `src/handlers/graphql.ts` | MEDIUM | ~150 | [ ] |
| 6.2.6 | `src/handlers/tools.ts` | HIGH | ~200 | [ ] |
| 6.2.7 | `src/handlers/sources.ts` | MEDIUM | ~150 | [ ] |
| 6.2.8 | `src/handlers/secrets.ts` | MEDIUM | ~150 | [ ] |
| 6.2.9 | `src/handlers/scope.ts` | MEDIUM | ~100 | [ ] |
| 6.2.10 | `src/handlers/google-discovery.ts` | LOW | ~150 | [ ] |
| 6.2.11 | `src/handlers/onepassword.ts` | LOW | ~100 | [ ] |
| 6.2.12 | `src/handlers/openapi.ts` | LOW | ~150 | [ ] |
| 6.2.13 | `src/services/engine.ts` | HIGH | ~300 | [ ] |
| 6.2.14 | `src/services/executor.ts` | HIGH | ~250 | [ ] |

**Output:** `executor/apps/server/exploration.md`

---

### 6.3 apps/web/ — **~15 files**

**Priority:** MEDIUM — React web UI

| # | Sub-directory/File | Priority | Lines (est.) | Status |
|---|-------------------|----------|--------------|--------|
| 6.3.1 | `src/App.tsx` | HIGH | ~300 | [ ] |
| 6.3.2 | `src/main.tsx` | MEDIUM | ~50 | [ ] |
| 6.3.3 | `src/components/tool-tree.tsx` | HIGH | ~250 | [ ] |
| 6.3.4 | `src/components/tool-detail.tsx` | HIGH | ~300 | [ ] |
| 6.3.5 | `src/components/mcp-install-card.tsx` | MEDIUM | ~200 | [ ] |
| 6.3.6 | `src/pages/secrets.tsx` | MEDIUM | ~200 | [ ] |
| 6.3.7 | `src/pages/source-detail.tsx` | MEDIUM | ~250 | [ ] |
| 6.3.8 | `src/lib/use-scope.ts` | MEDIUM | ~150 | [ ] |

**Output:** `executor/apps/web/exploration.md`

---

### 6.4 apps/desktop/ — **2 files**

**Priority:** LOW — Electron desktop app

| # | Sub-directory/File | Priority | Lines (est.) | Status |
|---|-------------------|----------|--------------|--------|
| 6.4.1 | `src/main.ts` | LOW | ~200 | [ ] |
| 6.4.2 | `src/preload.ts` | LOW | ~100 | [ ] |

**Output:** `executor/apps/desktop/exploration.md`

---

### 6.5 apps/marketing/ — **~5 files**

**Priority:** LOW — Marketing website

| # | Sub-directory/File | Priority | Lines (est.) | Status |
|---|-------------------|----------|--------------|--------|
| 6.5.1 | `src/pages/api/detect.ts` | LOW | ~100 | [ ] |

**Output:** `executor/apps/marketing/exploration.md`

---

## Phase 7: Tests

### 7.1 tests/

**Priority:** LOW — Integration and unit tests

| # | Sub-directory/File | Priority | Lines (est.) | Status |
|---|-------------------|----------|--------------|--------|
| 7.1.1 | `tests/e2e/` | LOW | ~300 | [ ] |
| 7.1.2 | `tests/fixtures/` | LOW | ~200 | [ ] |

**Output:** `executor/tests/exploration.md`

---

## Progress Tracking

### Summary

| Phase | Directories | Files | Priority | Status |
|-------|-------------|-------|----------|--------|
| 1. Core SDK | 6 | 57 | CRITICAL | [ ] |
| 2. Kernel | 4 | 14 | CRITICAL | [ ] |
| 3. Plugins | 7 | 99 | HIGH | [ ] |
| 4. Hosts | 1 | 3 | HIGH | [ ] |
| 5. UI | 2 | 27 | MEDIUM | [ ] |
| 6. Applications | 5 | 43 | HIGH | [ ] |
| 7. Tests | 2 | 10 | LOW | [ ] |
| **Total** | **27** | **253** | | |

### Completion Checklist

- [ ] Phase 1: Core SDK (6 modules)
- [ ] Phase 2: Kernel (4 modules)
- [ ] Phase 3: Plugins (7 modules)
- [ ] Phase 4: Hosts (1 module)
- [ ] Phase 5: UI (2 modules)
- [ ] Phase 6: Applications (5 modules)
- [ ] Phase 7: Tests (2 modules)

---

## Output Structure

All explorations should be saved to:

```
/home/darkvoid/Boxxed/@dev/repo-expolorations/hermes-agent/executor-deep-dive/
├── packages/
│   ├── core/
│   │   ├── sdk/
│   │   │   └── exploration.md
│   │   ├── execution/
│   │   │   └── exploration.md
│   │   └── ...
│   ├── plugins/
│   │   ├── openapi/
│   │   │   └── exploration.md
│   │   └── ...
│   └── ...
└── apps/
    ├── cli/
    │   └── exploration.md
    └── ...
```

---

**Last Updated:** 2026-04-07
