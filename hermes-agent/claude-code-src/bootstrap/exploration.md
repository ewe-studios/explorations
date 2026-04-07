# Claude Code Bootstrap Module — Deep-Dive Exploration

**Module:** `src/bootstrap/`  
**Parent Project:** Claude Code CLI  
**Created:** 2026-04-07  
**Files:** 1 TypeScript file

---

## 1. Module Overview

The `bootstrap/` module is the **global state and initialization backbone** for Claude Code CLI. Despite having only a single file (`state.ts`), it serves as the central registry for session-wide state, configuration, telemetry, and feature flags that persist across the application lifecycle.

### Core Responsibilities

1. **Global State Registry** — Session-wide mutable state:
   - 100+ state fields for configuration, telemetry, session tracking
   - Getter/setter accessors for all state fields
   - Test utilities for state reset

2. **Session Management** — Session lifecycle and identity:
   - Session ID generation and switching
   - Parent session tracking for lineage
   - Session project directory management

3. **Telemetry and Metrics** — OpenTelemetry integration:
   - Meter, Logger, Tracer provider state
   - Counter initialization (sessions, LOC, PRs, commits, tokens, cost)
   - Stats store for custom metrics

4. **Cost and Usage Tracking** — API usage accumulation:
   - Per-model token usage tracking
   - Cost accumulation and reporting
   - Turn-scoped duration tracking

5. **Feature Flags and Configuration** — Runtime configuration:
   - Beta header latches (AFK mode, fast mode, cache editing)
   - Setting source allowlists
   - Plugin and channel configuration

6. **Specialized State** — Domain-specific tracking:
   - Agent color assignment
   - Skill invocation tracking
   - Slow operation monitoring (dev-only)
   - Cron task registry
   - Error log buffer

### Key Design Patterns

- **Module-Level State**: Single `STATE` object with getter/setter accessors
- **Encapsulation**: No direct state access — all through exported functions
- **Lazy Initialization**: Counters and providers initialized on demand
- **Test Reset Utilities**: `resetStateForTests()` for isolation

---

## 2. File Inventory

| File | Lines | Key Exports | Description |
|------|-------|-------------|-------------|
| `state.ts` | ~1,758 | 150+ getter/setter functions | Global state registry with accessors |

**Total Lines:** ~1,758 lines

**Note:** This is the ONLY file in the bootstrap module. All bootstrap state management is centralized here.

---

## 3. Key Exports

### 3.1 State Type Definition (lines 45-257)

```typescript
type State = {
  // Working directory state
  originalCwd: string
  projectRoot: string  // Stable root, never updated mid-session
  cwd: string
  sessionProjectDir: string | null
  
  // Cost and usage tracking
  totalCostUSD: number
  totalAPIDuration: number
  totalAPIDurationWithoutRetries: number
  totalToolDuration: number
  turnHookDurationMs: number
  turnToolDurationMs: number
  turnClassifierDurationMs: number
  turnToolCount: number
  turnHookCount: number
  turnClassifierCount: number
  totalLinesAdded: number
  totalLinesRemoved: number
  hasUnknownModelCost: boolean
  
  // Timing
  startTime: number
  lastInteractionTime: number
  
  // Model usage
  modelUsage: { [modelName: string]: ModelUsage }
  mainLoopModelOverride: ModelSetting | undefined
  initialMainLoopModel: ModelSetting
  modelStrings: ModelStrings | null
  
  // Session mode
  isInteractive: boolean
  kairosActive: boolean
  strictToolResultPairing: boolean
  sdkAgentProgressSummariesEnabled: boolean
  userMsgOptIn: boolean
  
  // Client identification
  clientType: string
  sessionSource: string | undefined
  questionPreviewFormat: 'markdown' | 'html' | undefined
  
  // Settings configuration
  flagSettingsPath: string | undefined
  flagSettingsInline: Record<string, unknown> | null
  allowedSettingSources: SettingSource[]
  sessionIngressToken: string | null | undefined
  oauthTokenFromFd: string | null | undefined
  apiKeyFromFd: string | null | undefined
  
  // Telemetry providers
  meter: Meter | null
  sessionCounter: AttributedCounter | null
  locCounter: AttributedCounter | null
  prCounter: AttributedCounter | null
  commitCounter: AttributedCounter | null
  costCounter: AttributedCounter | null
  tokenCounter: AttributedCounter | null
  codeEditToolDecisionCounter: AttributedCounter | null
  activeTimeCounter: AttributedCounter | null
  statsStore: { observe(name: string, value: number): void } | null
  
  // Session identity
  sessionId: SessionId
  parentSessionId: SessionId | undefined
  
  // Logger/Tracer providers
  loggerProvider: LoggerProvider | null
  eventLogger: ReturnType<typeof logs.getLogger> | null
  meterProvider: MeterProvider | null
  tracerProvider: BasicTracerProvider | null
  
  // Agent color assignment
  agentColorMap: Map<string, AgentColorName>
  agentColorIndex: number
  
  // API request tracking (for bug reports)
  lastAPIRequest: Omit<BetaMessageStreamParams, 'messages'> | null
  lastAPIRequestMessages: BetaMessageStreamParams['messages'] | null
  lastClassifierRequests: unknown[] | null
  cachedClaudeMdContent: string | null
  
  // Error log
  inMemoryErrorLog: Array<{ error: string; timestamp: string }>
  
  // Plugin configuration
  inlinePlugins: Array<string>
  chromeFlagOverride: boolean | undefined
  useCoworkPlugins: boolean
  
  // Permission mode
  sessionBypassPermissionsMode: boolean
  
  // Cron tasks (session-only, not persisted)
  scheduledTasksEnabled: boolean
  sessionCronTasks: SessionCronTask[]
  
  // Team tracking
  sessionCreatedTeams: Set<string>
  
  // Trust and persistence
  sessionTrustAccepted: boolean
  sessionPersistenceDisabled: boolean
  
  // Plan mode tracking
  hasExitedPlanMode: boolean
  needsPlanModeExitAttachment: boolean
  needsAutoModeExitAttachment: boolean
  lspRecommendationShownThisSession: boolean
  
  // SDK state
  initJsonSchema: Record<string, unknown> | null
  registeredHooks: Partial<Record<HookEvent, RegisteredHookMatcher[]>> | null
  
  // Plan slug cache
  planSlugCache: Map<string, string>
  
  // Teleport session tracking
  teleportedSessionInfo: {
    isTeleported: boolean
    hasLoggedFirstMessage: boolean
    sessionId: string | null
  } | null
  
  // Invoked skills (preserved across compaction)
  invokedSkills: Map<string, {
    skillName: string
    skillPath: string
    content: string
    invokedAt: number
    agentId: string | null
  }>
  
  // Slow operations (ant-only dev bar)
  slowOperations: Array<{
    operation: string
    durationMs: number
    timestamp: number
  }>
  
  // SDK betas
  sdkBetas: string[] | undefined
  
  // Agent configuration
  mainThreadAgentType: string | undefined
  isRemoteMode: boolean
  
  // Direct connect server URL
  directConnectServerUrl: string | undefined
  
  // System prompt section cache
  systemPromptSectionCache: Map<string, string | null>
  
  // Date tracking (for midnight cache busting)
  lastEmittedDate: string | null
  
  // Additional directories
  additionalDirectoriesForClaudeMd: string[]
  
  // Channel allowlist
  allowedChannels: ChannelEntry[]
  hasDevChannels: boolean
  
  // Prompt cache configuration
  promptCache1hAllowlist: string[] | null
  promptCache1hEligible: boolean | null
  
  // Beta header latches (sticky-on for cache stability)
  afkModeHeaderLatched: boolean | null
  fastModeHeaderLatched: boolean | null
  cacheEditingHeaderLatched: boolean | null
  thinkingClearLatched: boolean | null
  
  // Prompt correlation
  promptId: string | null
  
  // API request tracking
  lastMainRequestId: string | undefined
  lastApiCompletionTimestamp: number | null
  pendingPostCompaction: boolean
}
```

---

### 3.2 Session Management

```typescript
export function getSessionId(): SessionId {
  return STATE.sessionId
}

export function regenerateSessionId(
  options: { setCurrentAsParent?: boolean } = {},
): SessionId {
  if (options.setCurrentAsParent) {
    STATE.parentSessionId = STATE.sessionId
  }
  STATE.planSlugCache.delete(STATE.sessionId)  // Cleanup stale entry
  STATE.sessionId = randomUUID() as SessionId
  STATE.sessionProjectDir = null
  return STATE.sessionId
}

export function getParentSessionId(): SessionId | undefined {
  return STATE.parentSessionId
}

export function switchSession(
  sessionId: SessionId,
  projectDir: string | null = null,
): void {
  STATE.planSlugCache.delete(STATE.sessionId)
  STATE.sessionId = sessionId
  STATE.sessionProjectDir = projectDir
  sessionSwitched.emit(sessionId)
}

export const onSessionSwitch = sessionSwitched.subscribe

export function getSessionProjectDir(): string | null {
  return STATE.sessionProjectDir
}
```

**Session Switch Signal:**

```typescript
const sessionSwitched = createSignal<[id: SessionId]>()
```

**Purpose:** Allows `concurrentSessions.ts` to keep PID file in sync with `--resume`.

---

### 3.3 Working Directory Management

```typescript
export function getOriginalCwd(): string {
  return STATE.originalCwd
}

export function getProjectRoot(): string {
  return STATE.projectRoot  // Stable root, never updated mid-session
}

export function setProjectRoot(cwd: string): void {
  STATE.projectRoot = cwd.normalize('NFC')  // Only for --worktree startup
}

export function getCwdState(): string {
  return STATE.cwd
}

export function setCwdState(cwd: string): void {
  STATE.cwd = cwd.normalize('NFC')
}
```

**Design Rationale:**

- `projectRoot`: Set once at startup, anchors skills/history
- `cwd`: Updated dynamically (e.g., EnterWorktreeTool)
- `sessionProjectDir`: Derives transcript path for cross-project sessions

---

### 3.4 Cost and Duration Tracking

```typescript
export function addToTotalDurationState(
  duration: number,
  durationWithoutRetries: number,
): void {
  STATE.totalAPIDuration += duration
  STATE.totalAPIDurationWithoutRetries += durationWithoutRetries
}

export function addToTotalCostState(
  cost: number,
  modelUsage: ModelUsage,
  model: string,
): void {
  STATE.modelUsage[model] = modelUsage
  STATE.totalCostUSD += cost
}

export function getTotalCostUSD(): number {
  return STATE.totalCostUSD
}

export function getTotalAPIDuration(): number {
  return STATE.totalAPIDuration
}

export function getTotalDuration(): number {
  return Date.now() - STATE.startTime
}
```

**Per-Turn Tracking:**

```typescript
export function addToToolDuration(duration: number): void {
  STATE.totalToolDuration += duration
  STATE.turnToolDurationMs += duration
  STATE.turnToolCount++
}

export function addToTurnHookDuration(duration: number): void {
  STATE.turnHookDurationMs += duration
  STATE.turnHookCount++
}

export function addToTurnClassifierDuration(duration: number): void {
  STATE.turnClassifierDurationMs += duration
  STATE.turnClassifierCount++
}
```

**Reset Functions (per-turn):**

```typescript
export function resetTurnHookDuration(): void {
  STATE.turnHookDurationMs = 0
  STATE.turnHookCount = 0
}

export function resetTurnToolDuration(): void {
  STATE.turnToolDurationMs = 0
  STATE.turnToolCount = 0
}

export function resetTurnClassifierDuration(): void {
  STATE.turnClassifierDurationMs = 0
  STATE.turnClassifierCount = 0
}
```

---

### 3.5 Telemetry Provider Setup

```typescript
export function setMeter(
  meter: Meter,
  createCounter: (name: string, options: MetricOptions) => AttributedCounter,
): void {
  STATE.meter = meter
  
  // Initialize all counters using the provided factory
  STATE.sessionCounter = createCounter('claude_code.session.count', {
    description: 'Count of CLI sessions started',
  })
  STATE.locCounter = createCounter('claude_code.lines_of_code.count', {
    description: "Count of lines of code modified...",
  })
  STATE.prCounter = createCounter('claude_code.pull_request.count', {
    description: 'Number of pull requests created',
  })
  STATE.commitCounter = createCounter('claude_code.commit.count', {
    description: 'Number of git commits created',
  })
  STATE.costCounter = createCounter('claude_code.cost.usage', {
    description: 'Cost of the Claude Code session',
    unit: 'USD',
  })
  STATE.tokenCounter = createCounter('claude_code.token.usage', {
    description: 'Number of tokens used',
    unit: 'tokens',
  })
  STATE.codeEditToolDecisionCounter = createCounter(
    'claude_code.code_edit_tool.decision',
    {
      description: 'Count of code editing tool permission decisions...',
    }
  )
  STATE.activeTimeCounter = createCounter('claude_code.active_time.total', {
    description: 'Total active time in seconds',
    unit: 's',
  })
}
```

**Provider Getters:**

```typescript
export function getMeter(): Meter | null
export function getSessionCounter(): AttributedCounter | null
export function getLocCounter(): AttributedCounter | null
export function getPrCounter(): AttributedCounter | null
export function getCommitCounter(): AttributedCounter | null
export function getCostCounter(): AttributedCounter | null
export function getTokenCounter(): AttributedCounter | null
export function getCodeEditToolDecisionCounter(): AttributedCounter | null
export function getActiveTimeCounter(): AttributedCounter | null

export function getLoggerProvider(): LoggerProvider | null
export function setLoggerProvider(provider: LoggerProvider | null): void

export function getEventLogger(): ReturnType<typeof logs.getLogger> | null
export function setEventLogger(logger: ReturnType<typeof logs.getLogger>): void

export function getMeterProvider(): MeterProvider | null
export function setMeterProvider(provider: MeterProvider | null): void

export function getTracerProvider(): BasicTracerProvider | null
export function setTracerProvider(provider: BasicTracerProvider | null): void
```

---

### 3.6 Token Budget Tracking

```typescript
let outputTokensAtTurnStart = 0
let currentTurnTokenBudget: number | null = null
let budgetContinuationCount = 0

export function getTurnOutputTokens(): number {
  return getTotalOutputTokens() - outputTokensAtTurnStart
}

export function getCurrentTurnTokenBudget(): number | null {
  return currentTurnTokenBudget
}

export function snapshotOutputTokensForTurn(budget: number | null): void {
  outputTokensAtTurnStart = getTotalOutputTokens()
  currentTurnTokenBudget = budget
  budgetContinuationCount = 0
}

export function getBudgetContinuationCount(): number {
  return budgetContinuationCount
}

export function incrementBudgetContinuationCount(): void {
  budgetContinuationCount++
}
```

**Module-Level Variables:** These are NOT in `STATE` — turn-scoped ephemeral state.

---

### 3.7 Interaction Time Tracking

```typescript
let interactionTimeDirty = false

export function updateLastInteractionTime(immediate?: boolean): void {
  if (immediate) {
    flushInteractionTime_inner()
  } else {
    interactionTimeDirty = true
  }
}

export function flushInteractionTime(): void {
  if (interactionTimeDirty) {
    flushInteractionTime_inner()
  }
}

function flushInteractionTime_inner(): void {
  STATE.lastInteractionTime = Date.now()
  interactionTimeDirty = false
}
```

**Deferred Timestamp Design:**

- Default: Mark dirty, flush before next Ink render (batches keypresses)
- `immediate=true`: For useEffect callbacks (after render cycle)

---

### 3.8 Scroll Drain Suspension

```typescript
let scrollDraining = false  // Module-scope, not in STATE
let scrollDrainTimer: ReturnType<typeof setTimeout> | undefined
const SCROLL_DRAIN_IDLE_MS = 150

export function markScrollActivity(): void {
  scrollDraining = true
  if (scrollDrainTimer) clearTimeout(scrollDrainTimer)
  scrollDrainTimer = setTimeout(() => {
    scrollDraining = false
    scrollDrainTimer = undefined
  }, SCROLL_DRAIN_IDLE_MS)
  scrollDrainTimer.unref?.()
}

export function getIsScrollDraining(): boolean {
  return scrollDraining
}

export async function waitForScrollIdle(): Promise<void> {
  while (scrollDraining) {
    await new Promise(r => setTimeout(r, SCROLL_DRAIN_IDLE_MS).unref?.())
  }
}
```

**Purpose:** Background intervals skip work during scroll to avoid jank.

---

### 3.9 Plan Mode Transition Handling

```typescript
export function handlePlanModeTransition(
  fromMode: string,
  toMode: string,
): void {
  // If switching TO plan mode, clear any pending exit attachment
  if (toMode === 'plan' && fromMode !== 'plan') {
    STATE.needsPlanModeExitAttachment = false
  }

  // If switching out of plan mode, trigger the plan_mode_exit attachment
  if (fromMode === 'plan' && toMode !== 'plan') {
    STATE.needsPlanModeExitAttachment = true
  }
}

export function handleAutoModeTransition(
  fromMode: string,
  toMode: string,
): void {
  // Auto↔plan transitions are handled elsewhere
  if (
    (fromMode === 'auto' && toMode === 'plan') ||
    (fromMode === 'plan' && toMode === 'auto')
  ) {
    return
  }
  
  const fromIsAuto = fromMode === 'auto'
  const toIsAuto = toMode === 'auto'

  // If switching TO auto mode, clear pending exit attachment
  if (toIsAuto && !fromIsAuto) {
    STATE.needsAutoModeExitAttachment = false
  }

  // If switching out of auto mode, trigger exit attachment
  if (fromIsAuto && !toIsAuto) {
    STATE.needsAutoModeExitAttachment = true
  }
}
```

**Attachment Prevention:** Quick toggles don't send both enter and exit attachments.

---

### 3.10 Invoked Skills Tracking

```typescript
export function addInvokedSkill(
  skillName: string,
  skillPath: string,
  content: string,
  agentId: string | null = null,
): void {
  const key = `${agentId ?? ''}:${skillName}`  // Composite key prevents cross-agent overwrites
  STATE.invokedSkills.set(key, {
    skillName,
    skillPath,
    content,
    invokedAt: Date.now(),
    agentId,
  })
}

export function getInvokedSkills(): Map<string, InvokedSkillInfo> {
  return STATE.invokedSkills
}

export function getInvokedSkillsForAgent(
  agentId: string | undefined | null,
): Map<string, InvokedSkillInfo> {
  const normalizedId = agentId ?? null
  const filtered = new Map<string, InvokedSkillInfo>()
  for (const [key, skill] of STATE.invokedSkills) {
    if (skill.agentId === normalizedId) {
      filtered.set(key, skill)
    }
  }
  return filtered
}

export function clearInvokedSkills(
  preservedAgentIds?: ReadonlySet<string>,
): void {
  if (!preservedAgentIds || preservedAgentIds.size === 0) {
    STATE.invokedSkills.clear()
    return
  }
  // Preserve skills for specified agents
  for (const [key, skill] of STATE.invokedSkills) {
    if (skill.agentId === null || !preservedAgentIds.has(skill.agentId)) {
      STATE.invokedSkills.delete(key)
    }
  }
}
```

**Purpose:** Preserve invoked skills across compaction to prevent re-expansion.

---

### 3.11 Slow Operation Monitoring (Ant-Only)

```typescript
const MAX_SLOW_OPERATIONS = 10
const SLOW_OPERATION_TTL_MS = 10000

export function addSlowOperation(operation: string, durationMs: number): void {
  if (process.env.USER_TYPE !== 'ant') return
  
  // Skip editor sessions (user editing prompt file in $EDITOR)
  if (operation.includes('exec') && operation.includes('claude-prompt-')) {
    return
  }
  
  const now = Date.now()
  // Remove stale operations
  STATE.slowOperations = STATE.slowOperations.filter(
    op => now - op.timestamp < SLOW_OPERATION_TTL_MS,
  )
  // Add new operation
  STATE.slowOperations.push({ operation, durationMs, timestamp: now })
  // Keep only most recent
  if (STATE.slowOperations.length > MAX_SLOW_OPERATIONS) {
    STATE.slowOperations = STATE.slowOperations.slice(-MAX_SLOW_OPERATIONS)
  }
}

const EMPTY_SLOW_OPERATIONS: ReadonlyArray<...> = []

export function getSlowOperations(): ReadonlyArray<...> {
  if (STATE.slowOperations.length === 0) {
    return EMPTY_SLOW_OPERATIONS  // Stable reference for Object.is bail
  }
  const now = Date.now()
  // Only allocate new array when something expired
  if (STATE.slowOperations.some(op => now - op.timestamp >= SLOW_OPERATION_TTL_MS)) {
    STATE.slowOperations = STATE.slowOperations.filter(...)
    if (STATE.slowOperations.length === 0) {
      return EMPTY_SLOW_OPERATIONS
    }
  }
  return STATE.slowOperations  // Safe: array never mutated after assignment
}
```

**Optimization:** Stable empty array reference prevents re-renders when polling at 2fps.

---

### 3.12 Beta Header Latches

```typescript
// Sticky-on latches prevent prompt cache busting on toggle

export function getAfkModeHeaderLatched(): boolean | null
export function setAfkModeHeaderLatched(v: boolean): void

export function getFastModeHeaderLatched(): boolean | null
export function setFastModeHeaderLatched(v: boolean): void

export function getCacheEditingHeaderLatched(): boolean | null
export function setCacheEditingHeaderLatched(v: boolean): void

export function getThinkingClearLatched(): boolean | null
export function setThinkingClearLatched(v: boolean): void

/**
 * Reset beta header latches on /clear and /compact
 */
export function clearBetaHeaderLatches(): void {
  STATE.afkModeHeaderLatched = null
  STATE.fastModeHeaderLatched = null
  STATE.cacheEditingHeaderLatched = null
  STATE.thinkingClearLatched = null
}
```

**Cache Stability:** Once latched true, headers persist for session to avoid cache bust.

---

### 3.13 System Prompt Section Cache

```typescript
export function getSystemPromptSectionCache(): Map<string, string | null> {
  return STATE.systemPromptSectionCache
}

export function setSystemPromptSectionCacheEntry(
  name: string,
  value: string | null,
): void {
  STATE.systemPromptSectionCache.set(name, value)
}

export function clearSystemPromptSectionState(): void {
  STATE.systemPromptSectionCache.clear()
}
```

**Purpose:** Cache computed system prompt sections (computed once per session).

---

### 3.14 Session Cron Tasks

```typescript
export type SessionCronTask = {
  id: string
  cron: string
  prompt: string
  createdAt: number
  recurring?: boolean
  agentId?: string  // Created by in-process teammate
}

export function getSessionCronTasks(): SessionCronTask[]
export function addSessionCronTask(task: SessionCronTask): void
export function removeSessionCronTasks(ids: readonly string[]): number
```

**Session-Only:** Never persisted to disk — die with process.

---

### 3.15 Test Utilities

```typescript
export function resetStateForTests(): void {
  if (process.env.NODE_ENV !== 'test') {
    throw new Error('resetStateForTests can only be called in tests')
  }
  Object.entries(getInitialState()).forEach(([key, value]) => {
    STATE[key as keyof State] = value as never
  })
  outputTokensAtTurnStart = 0
  currentTurnTokenBudget = null
  budgetContinuationCount = 0
  sessionSwitched.clear()
}

export function resetCostStateForTests(): void {
  STATE.totalCostUSD = 0
  STATE.totalAPIDuration = 0
  STATE.totalAPIDurationWithoutRetries = 0
  STATE.totalToolDuration = 0
  STATE.startTime = Date.now()
  STATE.totalLinesAdded = 0
  STATE.totalLinesRemoved = 0
  STATE.hasUnknownModelCost = false
  STATE.modelUsage = {}
  STATE.promptId = null
}

export function resetModelStringsForTestingOnly(): void {
  STATE.modelStrings = null
}
```

---

## 4. Line-by-Line Analysis

### 4.1 Initial State Factory (lines 260-426)

```typescript
function getInitialState(): State {
  // Resolve symlinks in cwd to match shell.ts setCwd behavior
  let resolvedCwd = ''
  if (typeof process !== 'undefined' && typeof process.cwd === 'function') {
    const rawCwd = cwd()
    try {
      resolvedCwd = realpathSync(rawCwd).normalize('NFC')
    } catch {
      // File Provider EPERM on CloudStorage mounts
      resolvedCwd = rawCwd.normalize('NFC')
    }
  }
  
  const state: State = {
    originalCwd: resolvedCwd,
    projectRoot: resolvedCwd,
    totalCostUSD: 0,
    totalAPIDuration: 0,
    totalAPIDurationWithoutRetries: 0,
    totalToolDuration: 0,
    turnHookDurationMs: 0,
    turnToolDurationMs: 0,
    turnClassifierDurationMs: 0,
    turnToolCount: 0,
    turnHookCount: 0,
    turnClassifierCount: 0,
    startTime: Date.now(),
    lastInteractionTime: Date.now(),
    totalLinesAdded: 0,
    totalLinesRemoved: 0,
    hasUnknownModelCost: false,
    cwd: resolvedCwd,
    modelUsage: {},
    mainLoopModelOverride: undefined,
    initialMainLoopModel: null,
    modelStrings: null,
    isInteractive: false,
    kairosActive: false,
    strictToolResultPairing: false,
    sdkAgentProgressSummariesEnabled: false,
    userMsgOptIn: false,
    clientType: 'cli',
    sessionSource: undefined,
    // ... more fields
    sessionId: randomUUID() as SessionId,
    parentSessionId: undefined,
    loggerProvider: null,
    eventLogger: null,
    meterProvider: null,
    tracerProvider: null,
    agentColorMap: new Map(),
    agentColorIndex: 0,
    lastAPIRequest: null,
    lastAPIRequestMessages: null,
    lastClassifierRequests: null,
    cachedClaudeMdContent: null,
    inMemoryErrorLog: [],
    inlinePlugins: [],
    chromeFlagOverride: undefined,
    useCoworkPlugins: false,
    sessionBypassPermissionsMode: false,
    scheduledTasksEnabled: false,
    sessionCronTasks: [],
    sessionCreatedTeams: new Set(),
    sessionTrustAccepted: false,
    sessionPersistenceDisabled: false,
    hasExitedPlanMode: false,
    needsPlanModeExitAttachment: false,
    needsAutoModeExitAttachment: false,
    lspRecommendationShownThisSession: false,
    initJsonSchema: null,
    registeredHooks: null,
    planSlugCache: new Map(),
    teleportedSessionInfo: null,
    invokedSkills: new Map(),
    slowOperations: [],
    sdkBetas: undefined,
    mainThreadAgentType: undefined,
    isRemoteMode: false,
    directConnectServerUrl: undefined,
    systemPromptSectionCache: new Map(),
    lastEmittedDate: null,
    additionalDirectoriesForClaudeMd: [],
    allowedChannels: [],
    hasDevChannels: false,
    sessionProjectDir: null,
    promptCache1hAllowlist: null,
    promptCache1hEligible: null,
    afkModeHeaderLatched: null,
    fastModeHeaderLatched: null,
    cacheEditingHeaderLatched: null,
    thinkingClearLatched: null,
    promptId: null,
    lastMainRequestId: undefined,
    lastApiCompletionTimestamp: null,
    pendingPostCompaction: false,
  }
  
  return state
}

// Module-level singleton state
const STATE: State = getInitialState()
```

**Key Observations:**

- **Symlink Resolution:** `realpathSync` matches shell.ts behavior for consistency
- **NFC Normalization:** Unicode normalization for cross-platform path consistency
- **Empty Collections:** `{}`, `[]`, `new Map()`, `new Set()` for mutable state
- **Null Defaults:** Nullable fields default to `null` or `undefined`
- **Boolean Flags:** All default to `false`
- **Timestamps:** `startTime` and `lastInteractionTime` initialized to `Date.now()`

---

### 4.2 Session ID Regeneration (lines 435-450)

```typescript
export function regenerateSessionId(
  options: { setCurrentAsParent?: boolean } = {},
): SessionId {
  if (options.setCurrentAsParent) {
    STATE.parentSessionId = STATE.sessionId  // Track lineage
  }
  // Drop outgoing session's plan-slug entry (prevent Map accumulation)
  STATE.planSlugCache.delete(STATE.sessionId)
  // Regenerate
  STATE.sessionId = randomUUID() as SessionId
  STATE.sessionProjectDir = null  // Reset to derive from originalCwd
  return STATE.sessionId
}
```

**Purpose:** Clear context (`/clear`, `/compact`) generates new session ID while tracking parent for lineage.

---

### 4.3 Session Switch Signal (lines 468-490)

```typescript
export function switchSession(
  sessionId: SessionId,
  projectDir: string | null = null,
): void {
  STATE.planSlugCache.delete(STATE.sessionId)
  STATE.sessionId = sessionId
  STATE.sessionProjectDir = projectDir
  sessionSwitched.emit(sessionId)
}

const sessionSwitched = createSignal<[id: SessionId]>()

/**
 * Register callback for session switches.
 * concurrentSessions.ts uses this to keep PID file in sync with --resume.
 */
export const onSessionSwitch = sessionSwitched.subscribe
```

**Signal Pattern:** Decoupled notification — bootstrap doesn't import listeners directly (DAG leaf).

---

### 4.4 Cost State Restore (lines 881-916)

```typescript
export function setCostStateForRestore({
  totalCostUSD,
  totalAPIDuration,
  totalAPIDurationWithoutRetries,
  totalToolDuration,
  totalLinesAdded,
  totalLinesRemoved,
  lastDuration,
  modelUsage,
}: {
  totalCostUSD: number
  totalAPIDuration: number
  totalAPIDurationWithoutRetries: number
  totalToolDuration: number
  totalLinesAdded: number
  totalLinesRemoved: number
  lastDuration: number | undefined
  modelUsage: { [modelName: string]: ModelUsage } | undefined
}): void {
  STATE.totalCostUSD = totalCostUSD
  STATE.totalAPIDuration = totalAPIDuration
  STATE.totalAPIDurationWithoutRetries = totalAPIDurationWithoutRetries
  STATE.totalToolDuration = totalToolDuration
  STATE.totalLinesAdded = totalLinesAdded
  STATE.totalLinesRemoved = totalLinesRemoved
  
  // Restore per-model usage breakdown
  if (modelUsage) {
    STATE.modelUsage = modelUsage
  }
  
  // Adjust startTime so wall duration accumulates correctly
  if (lastDuration) {
    STATE.startTime = Date.now() - lastDuration
  }
}
```

**Purpose:** Session resume restores cost state so cumulative totals persist across restarts.

---

### 4.5 Hook Callback Registration (lines 1419-1461)

```typescript
export function registerHookCallbacks(
  hooks: Partial<Record<HookEvent, RegisteredHookMatcher[]>>,
): void {
  if (!STATE.registeredHooks) {
    STATE.registeredHooks = {}
  }
  
  // May be called multiple times — merge, don't overwrite
  for (const [event, matchers] of Object.entries(hooks)) {
    const eventKey = event as HookEvent
    if (!STATE.registeredHooks[eventKey]) {
      STATE.registeredHooks[eventKey] = []
    }
    STATE.registeredHooks[eventKey]!.push(...matchers)
  }
}

export function clearRegisteredPluginHooks(): void {
  if (!STATE.registeredHooks) return
  
  const filtered: Partial<Record<HookEvent, RegisteredHookMatcher[]>> = {}
  for (const [event, matchers] of Object.entries(STATE.registeredHooks)) {
    // Keep only callback hooks (those without pluginRoot)
    const callbackHooks = matchers.filter(m => !('pluginRoot' in m))
    if (callbackHooks.length > 0) {
      filtered[event as HookEvent] = callbackHooks
    }
  }
  
  STATE.registeredHooks = Object.keys(filtered).length > 0 ? filtered : null
}
```

**Merge Semantics:** Multiple registrations accumulate — plugins and SDK can both register.

---

## 5. State Categories

### 5.1 Directory and Path State

| Field | Purpose | Mutability |
|-------|---------|------------|
| `originalCwd` | Initial working directory (symlinks resolved) | Write-once |
| `projectRoot` | Stable project root (skills/history anchor) | Startup only |
| `cwd` | Current working directory | Dynamic |
| `sessionProjectDir` | Transcript directory for cross-project sessions | Per-session |

---

### 5.2 Cost and Usage State

| Field | Purpose | Reset Scope |
|-------|---------|-------------|
| `totalCostUSD` | Cumulative API cost | Session |
| `totalAPIDuration` | Total API time (including retries) | Session |
| `totalAPIDurationWithoutRetries` | API time excluding retries | Session |
| `totalToolDuration` | Total tool execution time | Session |
| `totalLinesAdded/Removed` | Cumulative LOC changed | Session |
| `turn*Duration/Count` | Turn-scoped metrics | Per-turn |
| `modelUsage` | Per-model token breakdown | Session |

---

### 5.3 Telemetry State

| Field | Type | Purpose |
|-------|------|---------|
| `meter` | `Meter` | OpenTelemetry metrics |
| `sessionCounter` | `AttributedCounter` | Session count |
| `locCounter` | `AttributedCounter` | Lines of code |
| `prCounter` | `AttributedCounter` | Pull requests |
| `commitCounter` | `AttributedCounter` | Commits |
| `costCounter` | `AttributedCounter` | Cost tracking |
| `tokenCounter` | `AttributedCounter` | Token usage |
| `codeEditToolDecisionCounter` | `AttributedCounter` | Permission decisions |
| `activeTimeCounter` | `AttributedCounter` | Active time |
| `statsStore` | `{ observe() }` | Custom metrics |

---

### 5.4 Session Identity State

| Field | Purpose |
|-------|---------|
| `sessionId` | Current session UUID |
| `parentSessionId` | Parent session (for /clear, /compact lineage) |
| `sessionProjectDir` | Project directory for transcript path |
| `planSlugCache` | sessionId → wordSlug mapping |

---

### 5.5 Feature Flag State

| Field | Purpose |
|-------|---------|
| `afkModeHeaderLatched` | AFK mode beta header (sticky-on) |
| `fastModeHeaderLatched` | Fast mode beta header (sticky-on) |
| `cacheEditingHeaderLatched` | Cache editing beta header (sticky-on) |
| `thinkingClearLatched` | Thinking clear header (sticky-on) |
| `promptCache1hAllowlist` | 1h TTL allowlist from GrowthBook |
| `promptCache1hEligible` | User eligibility for 1h TTL |

---

### 5.6 Plugin and Hook State

| Field | Purpose |
|-------|---------|
| `inlinePlugins` | Session-only plugins from `--plugin-dir` |
| `useCoworkPlugins` | Use `cowork_plugins` directory |
| `chromeFlagOverride` | Explicit `--chrome` / `--no-chrome` value |
| `registeredHooks` | SDK callbacks and plugin native hooks |
| `initJsonSchema` | SDK init event JSON schema |

---

### 5.7 Agent and Team State

| Field | Purpose |
|-------|---------|
| `agentColorMap` | Agent ID → color name mapping |
| `agentColorIndex` | Next color index for assignment |
| `sessionCreatedTeams` | Teams created this session (cleanup on exit) |
| `mainThreadAgentType` | Agent type from `--agent` flag |

---

### 5.8 Permission and Mode State

| Field | Purpose |
|-------|---------|
| `isInteractive` | Interactive vs. headless/SDK mode |
| `kairosActive` | Assistant mode fully enabled |
| `strictToolResultPairing` | Throw on tool/result mismatch (HFI) |
| `sessionBypassPermissionsMode` | Session-only bypass flag |
| `sessionTrustAccepted` | Trust dialog accepted (not persisted) |
| `sessionPersistenceDisabled` | Disable session persistence |

---

## 6. Integration Points

### 6.1 With `state/` Module

| Bootstrap State | state/ Integration |
|-----------------|-------------------|
| `sessionId` | Session identity |
| `modelUsage` | Usage tracking |
| `isInteractive` | Mode detection |
| `costCounter` | Telemetry |

---

### 6.2 With Telemetry Services

| Bootstrap Function | Consumer |
|-------------------|----------|
| `setMeter()` | `initTelemetry()` |
| `setLoggerProvider()` | `initLogging()` |
| `setTracerProvider()` | `initTracing()` |

---

### 6.3 With Session Management

| Bootstrap Function | Consumer |
|-------------------|----------|
| `regenerateSessionId()` | `/clear`, `/compact` |
| `switchSession()` | `--resume` |
| `getSessionProjectDir()` | Transcript path resolution |

---

### 6.4 With Cost Tracking

| Bootstrap Function | Consumer |
|-------------------|----------|
| `addToTotalCostState()` | `cost-tracker.ts` |
| `snapshotOutputTokensForTurn()` | Token budget enforcement |
| `setCostStateForRestore()` | Session resume |

---

## 7. Key Patterns

### 7.1 Getter/Setter Encapsulation

All state access through exported functions — no direct `STATE` access:

```typescript
// Good
const cost = getTotalCostUSD()
setTotalCostUSD(newCost)

// Bad (not possible — STATE is module-private)
STATE.totalCostUSD  // Error: STATE not exported
```

**Benefit:** Centralized control, test mocking, validation hooks.

---

### 7.2 Sticky-On Latches

Beta headers latched true once triggered:

```typescript
if (!getAfkModeHeaderLatched() && shouldEnableAfkMode()) {
  setAfkModeHeaderLatched(true)
}
// Once true, stays true for session (prevents cache bust on toggle)
```

**Rationale:** Prompt cache is ~50-70K tokens — toggling headers busts cache.

---

### 7.3 Stable Empty References

Return stable empty array/map for Object.is comparison:

```typescript
const EMPTY_SLOW_OPERATIONS: ReadonlyArray<...> = []

export function getSlowOperations(): ReadonlyArray<...> {
  if (STATE.slowOperations.length === 0) {
    return EMPTY_SLOW_OPERATIONS  // Same reference every time
  }
  // ...
}
```

**Benefit:** React `setState` bails via `Object.is` instead of re-rendering.

---

### 7.4 Deferred Timestamps

Batch `Date.now()` calls for performance:

```typescript
export function updateLastInteractionTime(immediate?: boolean): void {
  if (immediate) {
    flushInteractionTime_inner()  // Immediate
  } else {
    interactionTimeDirty = true  // Deferred until next Ink render
  }
}
```

**Rationale:** Avoids `Date.now()` on every keypress.

---

### 7.5 Session-Only State

Some state dies with process (never persisted):

```typescript
sessionCronTasks: []  // CronCreate with durable: false
sessionCreatedTeams: new Set()  // Cleanup on graceful shutdown
sessionTrustAccepted: false  // Trust dialog (not saved to disk)
```

---

## 8. Environment Variables

| Variable | Purpose | Default |
|----------|---------|---------|
| `USER_TYPE` | Ant-only features (slow ops, tungsten) | `undefined` |
| `NODE_ENV` | Test mode detection | `'development'` |
| `CLAUDE_CODE_WORKER_EPOCH` | Worker epoch for CCR v2 | Required for CCR |

---

## 9. Summary

The `bootstrap/` module is Claude Code's **global state registry** with:

1. **150+ Accessor Functions** — Getters and setters for all state fields
2. **Session Management** — ID generation, switching, lineage tracking
3. **Telemetry Integration** — OpenTelemetry providers and counters
4. **Cost Tracking** — Cumulative and per-turn metrics
5. **Feature Flags** — Beta header latches, prompt cache configuration
6. **Plugin/Hook State** — SDK callbacks, plugin registration
7. **Test Utilities** — State reset functions for isolation

The module follows **encapsulation by design** — `STATE` is module-private, all access through exported functions. This enables centralized control, test mocking, and future validation hooks without breaking callers.

Despite being a single file, `state.ts` serves as the **backbone** for session-wide state that persists across the application lifecycle, from initialization through graceful shutdown.

---

**Last Updated:** 2026-04-07  
**Status:** Complete — single file (1,758 lines) fully analyzed
