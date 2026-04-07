# Schemas Module — Deep-Dive Exploration

**Module:** `schemas/`  
**Location:** `/home/darkvoid/Boxxed/@formulas/src.rust/src.llamacpp/src.ClaudOpen/claude-code-main/src/schemas/`  
**Files:** 1 TypeScript file  
**Created:** 2026-04-07

---

## 1. Module Overview

The `schemas/` module implements **Zod schema definitions for hooks** — providing type-safe validation for hook configurations in settings files. This module was extracted from `utils/settings/types.ts` to break circular dependencies between settings types and plugin schemas.

### Core Responsibilities

1. **Hook Schema Definitions** — Typed validation for hook types:
   - Command hooks (shell execution)
   - Prompt hooks (LLM evaluation)
   - HTTP hooks (webhook POSTs)
   - Agent hooks (agentic verification)

2. **Matcher Configuration** — Hook triggering logic:
   - Matcher patterns for tool names
   - Hook arrays per matcher
   - Conditional execution via `if` field

3. **Settings Validation** — Full hooks config schema:
   - Event-keyed record structure
   - Array of matchers per event
   - Partial record (not all events required)

### Key Design Patterns

- **Schema Factory Pattern**: `buildHookSchemas()` returns individual schemas
- **Lazy Schema Evaluation**: `lazySchema()` for circular reference handling
- **Discriminated Unions**: Type narrowing via `type` field
- **Permission Rule Syntax**: `if` field uses tool permission patterns

---

## 2. File Inventory

| File | Lines | Description |
|------|-------|-------------|
| `hooks.ts` | ~223 | Hook schema definitions |

**Total:** ~223 lines

---

## 3. Key Exports

### Hook Command Schemas (`hooks.ts`)

```typescript
// Individual hook types (discriminated union members)
export const HookCommandSchema: lazySchema<z.ZodType<HookCommand>>

export type HookCommand = z.infer<ReturnType<typeof HookCommandSchema>>
export type BashCommandHook = Extract<HookCommand, { type: 'command' }>
export type PromptHook = Extract<HookCommand, { type: 'prompt' }>
export type AgentHook = Extract<HookCommand, { type: 'agent' }>
export type HttpHook = Extract<HookCommand, { type: 'http' }>

// Matcher configuration
export const HookMatcherSchema: lazySchema<z.ZodType<HookMatcher>>
export type HookMatcher = z.infer<ReturnType<typeof HookMatcherSchema>>

// Full hooks configuration
export const HooksSchema: lazySchema<z.ZodType<HooksSettings>>
export type HooksSettings = Partial<Record<HookEvent, HookMatcher[]>>
```

---

## 4. Line-by-Line Analysis

### 4.1 If Condition Schema (`hooks.ts` lines 19-27)

```typescript
// Shared schema for the `if` condition field.
// Uses permission rule syntax (e.g., "Bash(git *)", "Read(*.ts)") to filter hooks
// before spawning. Evaluated against the hook input's tool_name and tool_input.
const IfConditionSchema = lazySchema(() =>
  z
    .string()
    .optional()
    .describe(
      'Permission rule syntax to filter when this hook runs (e.g., "Bash(git *)"). ' +
        'Only runs if the tool call matches the pattern. Avoids spawning hooks for non-matching commands.',
    ),
)
```

**Permission Rule Syntax**: Same pattern as tool permissions — `ToolName(pattern)`.

**Purpose**: Prevent unnecessary hook execution by filtering at the tool level.

### 4.2 Bash Command Hook (`hooks.ts` lines 32-65)

```typescript
const BashCommandHookSchema = z.object({
  type: z.literal('command').describe('Shell command hook type'),
  command: z.string().describe('Shell command to execute'),
  if: IfConditionSchema(),
  shell: z
    .enum(SHELL_TYPES)
    .optional()
    .describe(
      "Shell interpreter. 'bash' uses your $SHELL (bash/zsh/sh); 'powershell' uses pwsh. Defaults to bash.",
    ),
  timeout: z
    .number()
    .positive()
    .optional()
    .describe('Timeout in seconds for this specific command'),
  statusMessage: z
    .string()
    .optional()
    .describe('Custom status message to display in spinner while hook runs'),
  once: z
    .boolean()
    .optional()
    .describe('If true, hook runs once and is removed after execution'),
  async: z
    .boolean()
    .optional()
    .describe('If true, hook runs in background without blocking'),
  asyncRewake: z
    .boolean()
    .optional()
    .describe(
      'If true, hook runs in background and wakes the model on exit code 2 (blocking error). Implies async.',
    ),
})
```

**Execution Modes**:
- **Sync** (default): Blocks query loop until complete
- **Async**: Background execution, no blocking
- **AsyncRewake**: Background + wakes model on exit code 2

**Shell Types**: bash (default, uses $SHELL), powershell, cmd, sh.

### 4.3 Prompt Hook (`hooks.ts` lines 67-95)

```typescript
const PromptHookSchema = z.object({
  type: z.literal('prompt').describe('LLM prompt hook type'),
  prompt: z
    .string()
    .describe(
      'Prompt to evaluate with LLM. Use $ARGUMENTS placeholder for hook input JSON.',
    ),
  if: IfConditionSchema(),
  timeout: z
    .number()
    .positive()
    .optional()
    .describe('Timeout in seconds for this specific prompt evaluation'),
  model: z
    .string()
    .optional()
    .describe(
      'Model to use for this prompt hook (e.g., "claude-sonnet-4-6"). If not specified, uses the default small fast model.',
    ),
  statusMessage: z
    .string()
    .optional()
    .describe('Custom status message to display in spinner while hook runs'),
  once: z
    .boolean()
    .optional()
    .describe('If true, hook runs once and is removed after execution'),
})
```

**Prompt Placeholder**: `$ARGUMENTS` replaced with hook input JSON.

**Model Selection**: Defaults to small fast model if not specified.

### 4.4 HTTP Hook (`hooks.ts` lines 97-126)

```typescript
const HttpHookSchema = z.object({
  type: z.literal('http').describe('HTTP hook type'),
  url: z.string().url().describe('URL to POST the hook input JSON to'),
  if: IfConditionSchema(),
  timeout: z
    .number()
    .positive()
    .optional()
    .describe('Timeout in seconds for this specific request'),
  headers: z
    .record(z.string(), z.string())
    .optional()
    .describe(
      'Additional headers to include in the request. Values may reference environment variables using $VAR_NAME or ${VAR_NAME} syntax (e.g., "Authorization": "Bearer $MY_TOKEN"). Only variables listed in allowedEnvVars will be interpolated.',
    ),
  allowedEnvVars: z
    .array(z.string())
    .optional()
    .describe(
      'Explicit list of environment variable names that may be interpolated in header values. Only variables listed here will be resolved; all other $VAR references are left as empty strings. Required for env var interpolation to work.',
    ),
  statusMessage: z
    .string()
    .optional()
    .describe('Custom status message to display in spinner while hook runs'),
  once: z
    .boolean()
    .optional()
    .describe('If true, hook runs once and is removed after execution'),
})
```

**Environment Variable Interpolation**: 
- Syntax: `$VAR_NAME` or `${VAR_NAME}`
- Must list in `allowedEnvVars` for security
- Unlisted vars → empty string

### 4.5 Agent Hook (`hooks.ts` lines 128-163)

```typescript
const AgentHookSchema = z.object({
  type: z.literal('agent').describe('Agentic verifier hook type'),
  // DO NOT add .transform() here. This schema is used by parseSettingsFile,
  // and updateSettingsForSource round-trips the parsed result through
  // JSON.stringify — a transformed function value is silently dropped,
  // deleting the user's prompt from settings.json (gh-24920, CC-79). The
  // transform (from #10594) wrapped the string in `(_msgs) => prompt`
  // for a programmatic-construction use case in ExitPlanModeV2Tool that
  // has since been refactored into VerifyPlanExecutionTool, which no
  // longer constructs AgentHook objects at all.
  prompt: z
    .string()
    .describe(
      'Prompt describing what to verify (e.g. "Verify that unit tests ran and passed."). Use $ARGUMENTS placeholder for hook input JSON.',
    ),
  if: IfConditionSchema(),
  timeout: z
    .number()
    .positive()
    .optional()
    .describe('Timeout in seconds for agent execution (default 60)'),
  model: z
    .string()
    .optional()
    .describe(
      'Model to use for this agent hook (e.g., "claude-sonnet-4-6"). If not specified, uses Haiku.',
    ),
  statusMessage: z
    .string()
    .optional()
    .describe('Custom status message to display in spinner while hook runs'),
  once: z
    .boolean()
    .optional()
    .describe('If true, hook runs once and is removed after execution'),
})
```

**No Transform Warning**: "DO NOT add .transform() here. This schema is used by parseSettingsFile, and updateSettingsForSource round-trips the parsed result through JSON.stringify — a transformed function value is silently dropped, deleting the user's prompt from settings.json (gh-24920, CC-79)."

**Default Model**: Haiku (cheapest) for agent hooks.

### 4.6 Hook Command Schema Factory (`hooks.ts` lines 176-189)

```typescript
export const HookCommandSchema = lazySchema(() => {
  const {
    BashCommandHookSchema,
    PromptHookSchema,
    AgentHookSchema,
    HttpHookSchema,
  } = buildHookSchemas()
  return z.discriminatedUnion('type', [
    BashCommandHookSchema,
    PromptHookSchema,
    AgentHookSchema,
    HttpHookSchema,
  ])
})
```

**Discriminated Union**: TypeScript narrows type based on `type` field.

**Lazy Schema**: Prevents circular reference issues.

### 4.7 Matcher Schema (`hooks.ts` lines 194-204)

```typescript
export const HookMatcherSchema = lazySchema(() =>
  z.object({
    matcher: z
      .string()
      .optional()
      .describe('String pattern to match (e.g. tool names like "Write")'),
    hooks: z
      .array(HookCommandSchema())
      .describe('List of hooks to execute when the matcher matches'),
  }),
)
```

**Matcher Pattern**: String matching against tool names or event data.

**Hook Array**: Multiple hooks can run for a single matcher.

### 4.8 Hooks Schema (`hooks.ts` lines 211-222)

```typescript
export const HooksSchema = lazySchema(() =>
  z.partialRecord(z.enum(HOOK_EVENTS), z.array(HookMatcherSchema())),
)
```

**Event-Keyed Record**: Keys are hook events (e.g., `beforeQuery`, `afterToolUse`).

**Partial Record**: Not all events need to be defined.

**Matcher Array**: Multiple matchers per event, checked in order.

---

## 5. Integration Points

### 5.1 With `entrypoints/agentSdkTypes.js`

| Component | Integration |
|-----------|-------------|
| `hooks.ts` | Uses `HOOK_EVENTS` array for event enum |

### 5.2 With `utils/settings/types.js`

| Component | Integration |
|-----------|-------------|
| `hooks.ts` | Breaks circular dependency with plugin schemas |

### 5.3 With `utils/shell/shellProvider.js`

| Component | Integration |
|-----------|-------------|
| `BashCommandHookSchema` | Uses `SHELL_TYPES` enum |

---

## 6. Data Flow

### 6.1 Hook Execution Flow

```
Hook event triggered (e.g., beforeQuery)
    │
    ▼
HooksSchema.parse(settings.hooks)
    │
    ▼
Get matchers for event
    │
    ▼
For each matcher:
    ├──► Check matcher pattern against event data
    │    └──► Match? → Continue
    │    └──► No match? → Skip
    │
    └──► For each hook in matcher:
         ├──► Check `if` condition (permission rule)
         │    └──► Match? → Execute hook
         │    └──► No match? → Skip
         │
         └──► Execute hook type:
              ├──► command → spawn shell
              ├──► prompt → LLM evaluation
              ├──► http → POST request
              └──► agent → Subagent verification
```

### 6.2 Settings Round-Trip Flow

```
settings.json read
    │
    ▼
parseSettingsFile()
    │
    └──► HooksSchema.parse()
         │
         ▼
         Valid? → Use hooks
         Invalid? → Log error, skip
    │
    ▼
... (settings used in app)
    │
    ▼
updateSettingsForSource()
    │
    └──► JSON.stringify(parsed hooks)
         │
         ▼
         Write to settings.json
```

**Transform Warning**: Function transforms are dropped in JSON.stringify.

---

## 7. Key Patterns

### 7.1 Hook Event Types

```typescript
// From agentSdkTypes.js
export const HOOK_EVENTS = [
  'beforeQuery',
  'afterQuery',
  'beforeToolUse',
  'afterToolUse',
  // ... more events
] as const
```

**Event Points**: Hooks can run before/after queries and tool uses.

### 7.2 Matcher Pattern

```yaml
hooks:
  beforeQuery:
    - matcher: "Write"  # Match tool name
      hooks:
        - type: command
          command: "echo 'About to write: $ARGUMENTS'"
          if: "Write(*.ts)"  # Only for TypeScript files
```

**Two-Level Filtering**:
1. Matcher filters by string pattern
2. `if` field filters by permission rule syntax

### 7.3 Environment Variable Security

```yaml
hooks:
  beforeQuery:
    - hooks:
        - type: http
          url: https://api.example.com/hook
          headers:
            Authorization: "Bearer $API_TOKEN"
          allowedEnvVars:
            - API_TOKEN  # Must be explicitly listed
```

**Security Pattern**: Explicit allowlist prevents accidental env var leakage.

---

## 8. Example Configurations

### 8.1 Bash Command Hook

```yaml
hooks:
  beforeToolUse:
    - matcher: "Bash"
      hooks:
        - type: command
          command: "echo 'Running: $ARGUMENTS' >> /tmp/bash-log.txt"
          shell: bash
          async: true
```

### 8.2 Prompt Hook with Model

```yaml
hooks:
  afterQuery:
    - hooks:
        - type: prompt
          prompt: "Review this response for security issues: $ARGUMENTS"
          model: claude-sonnet-4-6
          timeout: 30
```

### 8.3 HTTP Hook with Env Vars

```yaml
hooks:
  afterToolUse:
    - matcher: "Write"
      hooks:
        - type: http
          url: https://api.example.com/file-changes
          headers:
            Authorization: "Bearer ${GITHUB_TOKEN}"
            X-Repo: "$REPO_NAME"
          allowedEnvVars:
            - GITHUB_TOKEN
            - REPO_NAME
```

### 8.4 Agent Verification Hook

```yaml
hooks:
  beforeQuery:
    - hooks:
        - type: agent
          prompt: "Verify that tests ran and passed. Check for test output in $ARGUMENTS."
          timeout: 120
          model: claude-haiku-4-6
          once: true  # Run only once per session
```

---

## 9. Summary

The `schemas/` module provides **Zod schema validation for hooks**:

1. **Hook Types** — Command, Prompt, HTTP, Agent schemas
2. **Matcher Configuration** — Pattern-based hook triggering
3. **Settings Validation** — Full hooks config schema
4. **Security Features** — Env var allowlisting, permission rule filtering

**Key Design Decisions**:
- **Extracted module** breaks circular dependencies
- **Discriminated unions** for type-safe hook handling
- **No transforms** to preserve data through JSON round-trips
- **Lazy schemas** for circular reference handling

**Hook Execution**:
- Event-triggered (beforeQuery, afterToolUse, etc.)
- Matcher-based filtering
- Permission rule `if` conditions
- Multiple execution modes (sync, async, asyncRewake)

---

**Last Updated:** 2026-04-07  
**Status:** Complete — 1 of 1 files analyzed
