# Coordinator Module — Deep-Dive Exploration

**Module:** `coordinator/`  
**Location:** `/home/darkvoid/Boxxed/@formulas/src.rust/src.llamacpp/src.ClaudOpen/claude-code-main/src/coordinator/`  
**Files:** 1 TypeScript file  
**Created:** 2026-04-07

---

## 1. Module Overview

The `coordinator/` module implements **Coordinator Mode** — a specialized operational mode for Claude Code that transforms it from a single-agent executor into a multi-agent orchestrator. In Coordinator Mode, Claude Code spawns and manages multiple worker agents to parallelize research, implementation, and verification tasks.

### Core Responsibilities

1. **Mode Detection** — Environment variable and feature gate checks:
   - `CLAUDE_CODE_COORDINATOR_MODE` env var
   - `COORDINATOR_MODE` feature flag
   - Session mode matching for resumed sessions

2. **Worker Context Building** — Injecting worker capabilities into system prompts:
   - Tool allowlist construction
   - MCP server enumeration
   - Scratchpad directory configuration

3. **System Prompt Generation** — Comprehensive coordinator instructions:
   - Role definition (orchestrator vs executor)
   - Worker management guidelines
   - Task workflow phases (Research → Synthesis → Implementation → Verification)
   - Prompt writing best practices

4. **Session Mode Persistence** — Tracking coordinator mode across session resumes

### Key Design Patterns

- **Feature Gating**: Dual-gated by feature flag AND environment variable
- **Tool Filtering**: Workers get subset of coordinator's tools (no meta-operations)
- **Prompt Engineering**: Extensive examples and anti-patterns in system prompt
- **Dependency Injection**: Scratchpad dir passed in from QueryEngine (avoids circular deps)

---

## 2. File Inventory

| File | Lines | Description |
|------|-------|-------------|
| `coordinatorMode.ts` | ~370 | Coordinator mode logic, system prompt generation |

**Total:** ~370 lines in 1 file

---

## 3. Key Exports

### Mode Detection

```typescript
// Check if coordinator mode is enabled
export function isCoordinatorMode(): boolean

// Match session mode on resume, flip env var if mismatched
export function matchSessionMode(
  sessionMode: 'coordinator' | 'normal' | undefined,
): string | undefined

// Build worker context for system prompt
export function getCoordinatorUserContext(
  mcpClients: ReadonlyArray<{ name: string }>,
  scratchpadDir?: string,
): { [k: string]: string }

// Get full coordinator system prompt
export function getCoordinatorSystemPrompt(): string
```

### Constants

```typescript
// Tools internal to coordinator (workers can't use)
const INTERNAL_WORKER_TOOLS = new Set([
  TEAM_CREATE_TOOL_NAME,
  TEAM_DELETE_TOOL_NAME,
  SEND_MESSAGE_TOOL_NAME,
  SYNTHETIC_OUTPUT_TOOL_NAME,
])
```

---

## 4. Line-by-Line Analysis

### 4.1 Mode Detection (`isCoordinatorMode`)

```typescript
// Lines 25-41
function isScratchpadGateEnabled(): boolean {
  return checkStatsigFeatureGate_CACHED_MAY_BE_STALE('tengu_scratch')
}

export function isCoordinatorMode(): boolean {
  if (feature('COORDINATOR_MODE')) {
    return isEnvTruthy(process.env.CLAUDE_CODE_COORDINATOR_MODE)
  }
  return false
}
```

**Dual-Gating Pattern**:
1. **Feature Gate**: `feature('COORDINATOR_MODE')` — GrowthBook/statsig controlled rollout
2. **Env Var**: `CLAUDE_CODE_COORDINATOR_MODE` — Runtime opt-in

**Why Both**: Feature gate controls who CAN use it; env var controls whether session IS using it.

**Scratchpad Gate Check**: Separate function to avoid circular dependency:
- `filesystem.ts` → `permissions.ts` → `coordinatorMode.ts` would cycle
- Instead: caller passes `scratchpadDir` via dependency injection

### 4.2 Session Mode Matching (`matchSessionMode`)

```typescript
// Lines 43-78
/**
 * Checks if the current coordinator mode matches the session's stored mode.
 * If mismatched, flips the environment variable so isCoordinatorMode() returns
 * the correct value for the resumed session. Returns a warning message if
 * the mode was switched, or undefined if no switch was needed.
 */
export function matchSessionMode(
  sessionMode: 'coordinator' | 'normal' | undefined,
): string | undefined {
  // No stored mode (old session before mode tracking) — do nothing
  if (!sessionMode) {
    return undefined
  }

  const currentIsCoordinator = isCoordinatorMode()
  const sessionIsCoordinator = sessionMode === 'coordinator'

  if (currentIsCoordinator === sessionIsCoordinator) {
    return undefined
  }

  // Flip the env var — isCoordinatorMode() reads it live, no caching
  if (sessionIsCoordinator) {
    process.env.CLAUDE_CODE_COORDINATOR_MODE = '1'
  } else {
    delete process.env.CLAUDE_CODE_COORDINATOR_MODE
  }

  logEvent('tengu_coordinator_mode_switched', {
    to: sessionMode as unknown as AnalyticsMetadata_I_VERIFIED_THIS_IS_NOT_CODE_OR_FILEPATHS,
  })

  return sessionIsCoordinator
    ? 'Entered coordinator mode to match resumed session.'
    : 'Exited coordinator mode to match resumed session.'
}
```

**Why This Matters**: Sessions can be resumed days later, potentially with different env vars. This ensures the resumed session behaves consistently with how it started.

**No Caching**: `isCoordinatorMode()` reads env var live — no stale state issues.

### 4.3 Worker Context Building (`getCoordinatorUserContext`)

```typescript
// Lines 80-109
export function getCoordinatorUserContext(
  mcpClients: ReadonlyArray<{ name: string }>,
  scratchpadDir?: string,
): { [k: string]: string } {
  if (!isCoordinatorMode()) {
    return {}
  }

  const workerTools = isEnvTruthy(process.env.CLAUDE_CODE_SIMPLE)
    ? [BASH_TOOL_NAME, FILE_READ_TOOL_NAME, FILE_EDIT_TOOL_NAME]
        .sort()
        .join(', ')
    : Array.from(ASYNC_AGENT_ALLOWED_TOOLS)
        .filter(name => !INTERNAL_WORKER_TOOLS.has(name))
        .sort()
        .join(', ')

  let content = `Workers spawned via the ${AGENT_TOOL_NAME} tool have access to these tools: ${workerTools}`

  if (mcpClients.length > 0) {
    const serverNames = mcpClients.map(c => c.name).join(', ')
    content += `\n\nWorkers also have access to MCP tools from connected MCP servers: ${serverNames}`
  }

  if (scratchpadDir && isScratchpadGateEnabled()) {
    content += `\n\nScratchpad directory: ${scratchpadDir}\nWorkers can read and write here without permission prompts. Use this for durable cross-worker knowledge — structure files however fits the work.`
  }

  return { workerToolsContext: content }
}
```

**Tool Allowlist Logic**:
- **SIMPLE Mode**: Only Bash, Read, Edit (minimal worker)
- **Full Mode**: `ASYNC_AGENT_ALLOWED_TOOLS` minus internal tools

**Internal Tools** (coordinator-only):
- `TEAM_CREATE_TOOL_NAME` — Create team members
- `TEAM_DELETE_TOOL_NAME` — Delete team members
- `SEND_MESSAGE_TOOL_NAME` — Continue worker conversations
- `SYNTHETIC_OUTPUT_TOOL_NAME` — Synthetic output generation

**MCP Integration**: Enumerates connected MCP servers so workers know what external tools are available.

**Scratchpad Directory**: Shared filesystem space for cross-worker knowledge transfer without permission prompts.

### 4.4 Coordinator System Prompt (`getCoordinatorSystemPrompt`)

The system prompt is extensive (~300 lines). Key sections:

**Role Definition (lines 116-127):**

```typescript
return `You are Claude Code, an AI assistant that orchestrates software engineering tasks across multiple workers.

## 1. Your Role

You are a **coordinator**. Your job is to:
- Help the user achieve their goal
- Direct workers to research, implement and verify code changes
- Synthesize results and communicate with the user
- Answer questions directly when possible — don't delegate work that you can handle without tools

Every message you send is to the user. Worker results and system notifications are internal signals, not conversation partners — never thank or acknowledge them. Summarize new information for the user as it arrives.`
```

**Key Guidance**:
- Coordinator ≠ executor (delegate substantive work)
- Worker results arrive as user-role messages (XML `<task-notification>`)
- Never thank/acknowledge workers (they're internal signals)

**Tool Documentation (lines 128-140):**

```
## 2. Your Tools

- **Agent** - Spawn a new worker
- **SendMessage** - Continue an existing worker
- **TaskStop** - Stop a running worker
- **subscribe_pr_activity / unsubscribe_pr_activity** - GitHub PR events

When calling Agent:
- Do not use one worker to check on another
- Do not use workers to trivially report file contents or run commands
- Do not set the model parameter
- Continue workers via SendMessage to leverage loaded context
- After launching agents, briefly tell the user what you launched and end your response
```

**Task Notification Format (lines 142-165):**

```xml
<task-notification>
<task-id>{agentId}</task-id>
<status>completed|failed|killed</status>
<summary>{human-readable status summary}</summary>
<result>{agent's final text response}</result>
<usage>
  <total_tokens>N</total_tokens>
  <tool_uses>N</tool_uses>
  <duration_ms>N</duration_ms>
</usage>
</task-notification>
```

**Workflow Phases (lines 200-228):**

| Phase | Who | Purpose |
|-------|-----|---------|
| Research | Workers (parallel) | Investigate codebase, find files |
| Synthesis | **You** (coordinator) | Read findings, craft specs |
| Implementation | Workers | Make targeted changes |
| Verification | Workers | Test changes work |

**Critical Insight**: Synthesis is the coordinator's PRIMARY job — not delegating understanding to workers.

**Verification Guidance (lines 220-228):**

```
### What Real Verification Looks Like

Verification means **proving the code works**, not confirming it exists.

- Run tests **with the feature enabled**
- Run typechecks and **investigate errors**
- Be skeptical — if something looks off, dig in
- **Test independently** — don't rubber-stamp
```

**Continue vs. Spawn Decision Matrix (lines 280-293):**

| Situation | Mechanism | Why |
|-----------|-----------|-----|
| Research → same files need editing | **Continue** | Worker has context |
| Research broad, implementation narrow | **Spawn fresh** | Avoid exploration noise |
| Correcting a failure | **Continue** | Worker has error context |
| Verifying another worker's code | **Spawn fresh** | Fresh eyes, no assumptions |
| Wrong approach entirely | **Spawn fresh** | Clean slate |
| Unrelated task | **Spawn fresh** | No context overlap |

**No Universal Default**: "Think about how much of the worker's context overlaps with the next task."

**Prompt Quality Examples (lines 310-336):**

Good:
```
"Fix the null pointer in src/auth/validate.ts:42. The user field can be undefined when the session expires. Add a null check and return early with an appropriate error. Commit and report the hash."
```

Bad (Anti-patterns):
```
"Fix the bug we discussed" — no context, workers can't see your conversation
"Based on your findings, implement the fix" — lazy delegation; synthesize yourself
"Create a PR for the recent changes" — ambiguous scope
"Something went wrong with the tests, can you look?" — no error message, no file path
```

---

## 5. Integration Points

### 5.1 With `tools/AgentTool/`

| Coordinator Component | Integration |
|----------------------|-------------|
| `getCoordinatorUserContext` | References `AGENT_TOOL_NAME` for spawning |
| `getCoordinatorSystemPrompt` | Documents Agent tool usage patterns |

### 5.2 With `tools/ SendMessageTool/`

| Coordinator Component | Integration |
|----------------------|-------------|
| `getCoordinatorUserContext` | Excludes `SEND_MESSAGE_TOOL_NAME` from workers |
| `getCoordinatorSystemPrompt` | Documents continue mechanic |

### 5.3 With `services/analytics/`

| Coordinator Component | Integration |
|----------------------|-------------|
| `matchSessionMode` | Logs `tengu_coordinator_mode_switched` event |

### 5.4 With `bootstrap/state.js`

| Coordinator Component | Integration |
|----------------------|-------------|
| `isCoordinatorMode` | Uses `isEnvTruthy()` for env var parsing |

### 5.5 With `utils/permissions/filesystem.ts`

| Coordinator Component | Integration |
|----------------------|-------------|
| `isScratchpadGateEnabled` | Checks same `tengu_scratch` gate (without importing) |

---

## 6. Data Flow

### 6.1 Session Start (Coordinator Mode)

```
Session initialization
         │
         ▼
  isCoordinatorMode()
         │
         ├──► feature('COORDINATOR_MODE')
         └──► isEnvTruthy(CLAUDE_CODE_COORDINATOR_MODE)
         │
         ▼
  getCoordinatorUserContext(mcpClients, scratchpadDir)
         │
         ├──► Build worker tools allowlist
         ├──► Add MCP server names
         └──► Add scratchpad dir guidance
         │
         ▼
  Injected into system prompt as user context
```

### 6.2 Session Resume

```
Resume from session file
         │
         ▼
  Read stored sessionMode ('coordinator' or 'normal')
         │
         ▼
  matchSessionMode(sessionMode)
         │
         ├──► Compare current vs stored mode
         ├──► If mismatch: flip env var
         ├──► Log telemetry event
         └──► Return user message ("Entered/Exited coordinator mode")
         │
         ▼
  isCoordinatorMode() now returns correct value
```

### 6.3 Worker Spawn Flow

```
Coordinator calls Agent tool
         │
         ▼
  Worker receives system prompt with:
  - Coordinator system prompt
  - workerToolsContext (allowed tools)
  - MCP server list
  - Scratchpad dir (if enabled)
         │
         ▼
  Worker executes task
         │
         ▼
  <task-notification> sent to coordinator
         │
         ▼
  Coordinator synthesizes result for user
```

---

## 7. Key Patterns

### 7.1 Dual-Gating

```
Feature Gate (who CAN) + Env Var (who IS) = Coordinator Mode
```

**Why**: Gradual rollout via statsig, explicit opt-in per session.

### 7.2 Dependency Injection for Circular Deps

```typescript
// Instead of:
import { isScratchpadEnabled } from '../utils/permissions/filesystem.js'

// Use:
function isScratchpadGateEnabled(): boolean {
  return checkStatsigFeatureGate_CACHED_MAY_BE_STALE('tengu_scratch')
}

// Caller passes scratchpadDir as parameter
export function getCoordinatorUserContext(
  mcpClients: ...,
  scratchpadDir?: string,  // Injected from QueryEngine
)
```

### 7.3 Prompt Engineering Patterns

**Anti-Pattern Naming**:
- "lazy delegation" — delegating understanding instead of synthesizing
- "rubber-stamp verification" — confirming existence vs proving correctness

**Concrete Examples**:
- File paths with line numbers: `src/auth/validate.ts:42`
- Specific field names: `user field on Session`
- Exact error messages: `'Session expired'`

**Decision Frameworks**:
- Continue vs Spawn matrix with 6 scenarios
- Workflow phases with ownership (Who column)

---

## 8. Environment Variables

| Variable | Purpose | Values |
|----------|---------|--------|
| `CLAUDE_CODE_COORDINATOR_MODE` | Enable coordinator mode | `1`/`true` = on |
| `CLAUDE_CODE_SIMPLE` | Simple worker mode | `1`/`true` = minimal tools |

---

## 9. Feature Gates

| Gate | Purpose |
|------|---------|
| `COORDINATOR_MODE` | Master gate for coordinator mode |
| `tengu_scratch` | Scratchpad directory feature |

---

## 10. Telemetry Events

| Event | Location | Fields |
|-------|----------|--------|
| `tengu_coordinator_mode_switched` | `matchSessionMode()` | `to: 'coordinator' \| 'normal'` |

---

## 11. Testing Considerations

### 11.1 Mode Detection

```typescript
// Test: Coordinator mode detection
process.env.CLAUDE_CODE_COORDINATOR_MODE = '1'
// Mock feature() to return true
assert.strictEqual(isCoordinatorMode(), true)

delete process.env.CLAUDE_CODE_COORDINATOR_MODE
assert.strictEqual(isCoordinatorMode(), false)
```

### 11.2 Session Mode Matching

```typescript
// Test: Mode switch on resume
process.env.CLAUDE_CODE_COORDINATOR_MODE = undefined
const msg = matchSessionMode('coordinator')
assert.strictEqual(process.env.CLAUDE_CODE_COORDINATOR_MODE, '1')
assert.ok(msg?.includes('Entered coordinator mode'))
```

### 11.3 Worker Context Building

```typescript
// Test: Worker tools exclude internal tools
const ctx = getCoordinatorUserContext([{ name: 'github' }])
assert.ok(ctx.workerToolsContext.includes('Bash'))
assert.ok(!ctx.workerToolsContext.includes('SendMessage'))
assert.ok(ctx.workerToolsContext.includes('github'))
```

---

## 12. Summary

The `coordinator/` module transforms Claude Code into a **multi-agent orchestrator**:

1. **Mode Detection** — Dual-gated by feature flag and environment variable
2. **Worker Management** — Tool allowlisting, MCP enumeration, scratchpad config
3. **Prompt Engineering** — Comprehensive 300+ line system prompt with:
   - Role definition and examples
   - Task workflow phases
   - Continue vs Spawn decision matrix
   - Anti-pattern callouts
4. **Session Persistence** — Mode tracking across session resumes

**Key Architectural Decisions**:
- **Dependency Injection** avoids circular deps with permissions module
- **Live Env Var Reading** ensures no stale state on mode switch
- **Worker Tool Filtering** prevents meta-operations (can't spawn/kill workers)
- **Explicit Synthesis Ownership** — coordinator MUST understand before delegating

---

**Last Updated:** 2026-04-07  
**Status:** Complete — single file fully analyzed
