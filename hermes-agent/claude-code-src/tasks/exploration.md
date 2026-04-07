# Tasks Module Exploration

## File Inventory

| File | Lines | Key Exports | Description |
|------|-------|-------------|-------------|
| `types.ts` | 47 | `TaskState`, `BackgroundTaskState`, `isBackgroundTask()` | Task state unions and background task predicate |
| `DreamTask/DreamTask.ts` | 158 | `DreamTaskState`, `DreamTurn`, `registerDreamTask()`, `addDreamTurn()`, `kill()` | Memory consolidation subagent task |
| `InProcessTeammateTask/types.ts` | 122 | `InProcessTeammateTaskState`, `TeammateIdentity`, `TEAMMATE_MESSAGES_UI_CAP`, `appendCappedMessage()` | In-process teammate type definitions |
| `InProcessTeammateTask/InProcessTeammateTask.tsx` | 126 | `InProcessTeammateTask`, `requestTeammateShutdown()`, `appendTeammateMessage()`, `injectUserMessageToTeammate()`, `getRunningTeammatesSorted()` | Teammate lifecycle management |
| `LocalAgentTask/LocalAgentTask.tsx` | 250+ | `LocalAgentTaskState`, `ToolActivity`, `AgentProgress`, `ProgressTracker`, `updateProgressFromMessage()` | Local agent execution task |
| `LocalMainSessionTask.ts` | 480 | `LocalMainSessionTaskState`, `registerMainSessionTask()`, `startBackgroundSession()`, `completeMainSessionTask()` | Main session backgrounding |
| `LocalShellTask/guards.ts` | 42 | `LocalShellTaskState`, `BashTaskKind`, `isLocalShellTask()` | Type guards for shell tasks |
| `LocalShellTask/killShellTasks.ts` | 77 | `killTask()`, `killShellTasksForAgent()` | Shell task termination |
| `LocalShellTask/LocalShellTask.tsx` | 250+ | `LocalShellTask`, `spawnShellTask()`, `BACKGROUND_BASH_SUMMARY_PREFIX`, `looksLikePrompt()` | Shell command execution |
| `RemoteAgentTask/RemoteAgentTask.tsx` | 250+ | `RemoteAgentTaskState`, `RemoteTaskType`, `registerCompletionChecker()`, `extractPlanFromLog()` | Remote/cloud agent tasks |
| `pillLabel.ts` | 83 | `getPillLabel()`, `pillNeedsCta()` | Background task pill display |
| `stopTask.ts` | 101 | `StopTaskError`, `stopTask()` | Shared task stop logic |

**Total Source Lines:** ~2,000+ (across 12 files)

---

## Module Overview

### Purpose

The `tasks/` module implements a **unified task framework** for tracking asynchronous operations in Claude Code. It provides a registry pattern for managing diverse task types (bash, local agents, remote agents, teammates, workflows) with consistent lifecycle management, notification systems, and UI integration.

### Responsibilities

1. **Task Registry**: Centralized tracking of all running/completed/failed tasks
2. **Lifecycle Management**: Register, update, complete, fail, kill operations
3. **Task Framework**: Generic `Task` interface with type-specific implementations
4. **Notification System**: XML-based notifications for model and SDK consumers
5. **Background Task Management**: Pill display, CTA states, attention tracking
6. **Disk Output**: Per-task output files with symlink-based isolation
7. **Agent Context**: AsyncLocalStorage-based isolation for subagent tasks

### Architecture

```
tasks/
│
├── Core Framework
│   ├── types.ts (TaskState union, BackgroundTaskState)
│   └── utils/task/framework.ts (registerTask, updateTaskState)
│
├── Task Implementations
│   ├── LocalShellTask/* (bash commands, monitors)
│   ├── LocalAgentTask/* (local subagents)
│   ├── LocalMainSessionTask.ts (background main session)
│   ├── RemoteAgentTask/* (cloud sessions)
│   ├── InProcessTeammateTask/* (in-process teammates)
│   └── DreamTask/* (memory consolidation)
│
├── Task Operations
│   ├── stopTask.ts (shared stop logic)
│   ├── LocalShellTask/killShellTasks.ts (bash termination)
│   └── pillLabel.ts (UI display)
│
└── Integration Points
    ├── state/AppState.ts (task registry in global state)
    ├── utils/messageQueueManager.ts (notifications)
    ├── utils/task/diskOutput.ts (file output)
    └── utils/sdkEventQueue.ts (SDK events)
```

---

## Key Exports

### Type Definitions

#### `TaskState` Union (types.ts)

```typescript
export type TaskState =
  | LocalShellTaskState
  | LocalAgentTaskState
  | RemoteAgentTaskState
  | InProcessTeammateTaskState
  | LocalWorkflowTaskState
  | MonitorMcpTaskState
  | DreamTaskState
```

**Purpose**: Discriminated union of all task state types.

**Usage**: Type-safe task handling via discriminated union pattern.

---

#### `BackgroundTaskState` (types.ts)

```typescript
export type BackgroundTaskState =
  | LocalShellTaskState
  | LocalAgentTaskState
  | RemoteAgentTaskState
  | InProcessTeammateTaskState
  | LocalWorkflowTaskState
  | MonitorMcpTaskState
  | DreamTaskState
```

**Purpose**: Task types shown in background tasks indicator.

**Constraint**: Subset of `TaskState` excluding completed/failed tasks.

---

#### `LocalShellTaskState` (LocalShellTask/guards.ts)

```typescript
export type BashTaskKind = 'bash' | 'monitor'

export type LocalShellTaskState = TaskStateBase & {
  type: 'local_bash'
  command: string
  result?: {
    code: number
    interrupted: boolean
  }
  completionStatusSentInAttachment: boolean
  shellCommand: ShellCommand | null
  unregisterCleanup?: () => void
  cleanupTimeoutId?: NodeJS.Timeout
  lastReportedTotalLines: number
  isBackgrounded: boolean
  agentId?: AgentId
  kind?: BashTaskKind
}
```

**Purpose**: State for background bash commands and monitors.

**Key Fields**:
- `kind`: 'bash' shows command, 'monitor' shows description
- `agentId`: Tracks which subagent spawned this task (for orphan cleanup)
- `isBackgrounded`: false = foreground, true = background

---

#### `LocalAgentTaskState` (LocalAgentTask/LocalAgentTask.tsx)

```typescript
export type ToolActivity = {
  toolName: string
  input: Record<string, unknown>
  activityDescription?: string
  isSearch?: boolean
  isRead?: boolean
}

export type AgentProgress = {
  toolUseCount: number
  tokenCount: number
  lastActivity?: ToolActivity
  recentActivities?: ToolActivity[]
  summary?: string
}

export type ProgressTracker = {
  toolUseCount: number
  latestInputTokens: number
  cumulativeOutputTokens: number
  recentActivities: ToolActivity[]
}

export type LocalAgentTaskState = TaskStateBase & {
  type: 'local_agent'
  agentId: string
  prompt: string
  selectedAgent?: AgentDefinition
  agentType: string
  model?: string
  abortController?: AbortController
  unregisterCleanup?: () => void
  error?: string
  result?: AgentToolResult
  progress?: AgentProgress
  retrieved: boolean
  messages?: Message[]
  lastReportedToolCount: number
  lastReportedTokenCount: number
  isBackgrounded: boolean
  pendingMessages: string[]
  retain: boolean
  diskLoaded: boolean
  evictAfter?: number
}
```

**Purpose**: State for local subagent execution.

**Key Fields**:
- `agentType`: 'main-session' vs 'subagent' vs custom agent types
- `retain`: UI is holding this task (blocks eviction)
- `diskLoaded`: Bootstrap has read sidechain JSONL
- `evictAfter`: Panel visibility deadline timestamp
- `pendingMessages`: User messages queued mid-turn via SendMessage

---

#### `LocalMainSessionTaskState` (LocalMainSessionTask.ts)

```typescript
export type LocalMainSessionTaskState = LocalAgentTaskState & {
  agentType: 'main-session'
}
```

**Purpose**: Specialized LocalAgentTask for backgrounded main session.

**Inheritance**: Extends `LocalAgentTaskState` with `agentType` discriminator.

---

#### `RemoteAgentTaskState` (RemoteAgentTask/RemoteAgentTask.tsx)

```typescript
export type RemoteTaskType = 'remote-agent' | 'ultraplan' | 'ultrareview' | 'autofix-pr' | 'background-pr'

export type AutofixPrRemoteTaskMetadata = {
  owner: string
  repo: string
  prNumber: number
}

export type RemoteTaskCompletionChecker = (
  remoteTaskMetadata: RemoteTaskMetadata | undefined
) => Promise<string | null>

export type RemoteAgentTaskState = TaskStateBase & {
  type: 'remote_agent'
  remoteTaskType: RemoteTaskType
  remoteTaskMetadata?: RemoteTaskMetadata
  sessionId: string
  command: string
  title: string
  todoList: TodoList
  log: SDKMessage[]
  isLongRunning?: boolean
  pollStartedAt: number
  isRemoteReview?: boolean
  reviewProgress?: {
    stage?: 'finding' | 'verifying' | 'synthesizing'
    bugsFound: number
    bugsVerified: number
    bugsRefuted: number
  }
  isUltraplan?: boolean
  ultraplanPhase?: Exclude<UltraplanPhase, 'running'>
}
```

**Purpose**: State for remote/cloud agent sessions.

**Key Fields**:
- `remoteTaskType`: Task variant (remote-agent, ultraplan, ultrareview, autofix-pr, background-pr)
- `sessionId`: Remote session ID for API calls
- `pollStartedAt`: Local poll start time (prevents immediate timeout on restore)
- `ultraplanPhase`: 'needs_input' | 'plan_ready' | undefined (running)

---

#### `InProcessTeammateTaskState` (InProcessTeammateTask/types.ts)

```typescript
export type TeammateIdentity = {
  agentId: string // e.g., "researcher@my-team"
  agentName: string // e.g., "researcher"
  teamName: string
  color?: string
  planModeRequired: boolean
  parentSessionId: string // Leader's session ID
}

export const TEAMMATE_MESSAGES_UI_CAP = 50

export type InProcessTeammateTaskState = TaskStateBase & {
  type: 'in_process_teammate'
  identity: TeammateIdentity
  prompt: string
  model?: string
  selectedAgent?: AgentDefinition
  abortController?: AbortController
  currentWorkAbortController?: AbortController
  unregisterCleanup?: () => void
  awaitingPlanApproval: boolean
  permissionMode: PermissionMode
  error?: string
  result?: AgentToolResult
  progress?: AgentProgress
  messages?: Message[]
  inProgressToolUseIDs?: Set<string>
  pendingUserMessages: string[]
  spinnerVerb?: string
  pastTenseVerb?: string
  isIdle: boolean
  shutdownRequested: boolean
  onIdleCallbacks?: Array<() => void>
  lastReportedToolCount: number
  lastReportedTokenCount: number
}
```

**Purpose**: State for in-process teammates (AsyncLocalStorage-based isolation).

**Key Fields**:
- `identity`: Teammate identity (agentName@teamName)
- `isIdle`: Waiting for work vs actively processing
- `shutdownRequested`: Graceful shutdown flag
- `onIdleCallbacks`: Leader wait mechanism without polling
- `messages`: Capped at `TEAMMATE_MESSAGES_UI_CAP` (50) for zoomed view

**Memory Context** (from comment, lines 96-99):
> BQ analysis (round 9, 2026-03-20) showed ~20MB RSS per agent at 500+ turn sessions and ~125MB per concurrent agent in swarm bursts. Whale session 9a990de8 launched 292 agents in 2 minutes and reached 36.8GB. The dominant cost is this array holding a second full copy of every message.

---

#### `DreamTaskState` (DreamTask/DreamTask.ts)

```typescript
export type DreamTurn = {
  text: string
  toolUseCount: number
}

export type DreamPhase = 'starting' | 'updating'

export type DreamTaskState = TaskStateBase & {
  type: 'dream'
  phase: DreamPhase
  sessionsReviewing: number
  filesTouched: string[]
  turns: DreamTurn[]
  abortController?: AbortController
  priorMtime: number
}
```

**Purpose**: State for memory consolidation subagent (auto-dream).

**Key Fields**:
- `turns`: Assistant responses with tool uses collapsed to count
- `filesTouched`: Paths from Edit/Write tool_use blocks (incomplete reflection)
- `priorMtime`: Lock mtime for rollback on kill
- `phase`: 'starting' → 'updating' when first Edit/Write lands

---

### Task Framework Types

#### `Task` Interface

```typescript
interface Task {
  name: string
  type: string
  kill: (taskId: string, setAppState: SetAppState) => Promise<void>
}
```

**Purpose**: Common interface for all task type implementations.

**Implementations**:
- `LocalShellTask`
- `LocalAgentTask`
- `RemoteAgentTask`
- `InProcessTeammateTask`
- `DreamTask`

---

### Functions

#### `isBackgroundTask()` (types.ts)

```typescript
export function isBackgroundTask(task: TaskState): task is BackgroundTaskState {
  if (task.status !== 'running' && task.status !== 'pending') {
    return false
  }
  if ('isBackgrounded' in task && task.isBackgrounded === false) {
    return false
  }
  return true
}
```

**Purpose**: Determine if task should appear in background tasks indicator.

**Logic**:
1. Status must be 'running' or 'pending'
2. If `isBackgrounded === false`, it's foreground (excluded)
3. Otherwise, it's a background task

---

#### `registerDreamTask()` (DreamTask/DreamTask.ts)

```typescript
export function registerDreamTask(
  setAppState: SetAppState,
  opts: {
    sessionsReviewing: number
    priorMtime: number
    abortController: AbortController
  },
): string
```

**Purpose**: Register a new dream task for memory consolidation.

**Returns**: Task ID string

**Implementation**:
```typescript
const id = generateTaskId('dream')
const task: DreamTaskState = {
  ...createTaskStateBase(id, 'dream', 'dreaming'),
  type: 'dream',
  status: 'running',
  phase: 'starting',
  sessionsReviewing: opts.sessionsReviewing,
  filesTouched: [],
  turns: [],
  abortController: opts.abortController,
  priorMtime: opts.priorMtime,
}
registerTask(task, setAppState)
return id
```

---

#### `addDreamTurn()` (DreamTask/DreamTask.ts)

```typescript
export function addDreamTurn(
  taskId: string,
  turn: DreamTurn,
  touchedPaths: string[],
  setAppState: SetAppState,
): void
```

**Purpose**: Add a dream agent turn to task state.

**Implementation** (lines 82-103):
```typescript
updateTaskState<DreamTaskState>(taskId, setAppState, task => {
  const seen = new Set(task.filesTouched)
  const newTouched = touchedPaths.filter(p => !seen.has(p) && seen.add(p))
  if (
    turn.text === '' &&
    turn.toolUseCount === 0 &&
    newTouched.length === 0
  ) {
    return task // Skip no-op updates
  }
  return {
    ...task,
    phase: newTouched.length > 0 ? 'updating' : task.phase,
    filesTouched: newTouched.length > 0
      ? [...task.filesTouched, ...newTouched]
      : task.filesTouched,
    turns: task.turns.slice(-(MAX_TURNS - 1)).concat(turn),
  }
})
```

**Key Optimizations**:
- Deduplicates `filesTouched` via Set
- Skips update if turn is empty AND no new files touched
- Caps `turns` array at `MAX_TURNS` (30) for display

---

#### `registerMainSessionTask()` (LocalMainSessionTask.ts)

```typescript
export function registerMainSessionTask(
  description: string,
  setAppState: SetAppState,
  mainThreadAgentDefinition?: AgentDefinition,
  existingAbortController?: AbortController,
): { taskId: string; abortSignal: AbortSignal }
```

**Purpose**: Register a backgrounded main session task.

**Key Implementation** (lines 107-110):
```typescript
// Link output to an isolated per-task transcript file
void initTaskOutputAsSymlink(
  taskId,
  getAgentTranscriptPath(asAgentId(taskId)),
)
```

**Design Note**: Uses isolated transcript path (not main session file) to survive `/clear` mid-run.

**Returns**: Task ID and abort signal for stopping the query.

---

#### `startBackgroundSession()` (LocalMainSessionTask.ts)

```typescript
export function startBackgroundSession({
  messages,
  queryParams,
  description,
  setAppState,
  agentDefinition,
}: {
  messages: Message[]
  queryParams: Omit<QueryParams, 'messages'>
  description: string
  setAppState: SetAppState
  agentDefinition?: AgentDefinition
}): string
```

**Purpose**: Start a fresh background session with current messages.

**Key Implementation** (lines 368-375):
```typescript
const agentContext: SubagentContext = {
  agentId: taskId,
  agentType: 'subagent',
  subagentName: 'main-session',
  isBuiltIn: true,
}

void runWithAgentContext(agentContext, async () => {
  // ... query() execution with agent context wrapping
})
```

**Purpose**: AsyncLocalStorage isolation for concurrent task execution.

---

#### `spawnShellTask()` (LocalShellTask/LocalShellTask.tsx)

```typescript
export async function spawnShellTask(input: LocalShellSpawnInput & {
  shellCommand: ShellCommand
}, context: TaskContext): Promise<TaskHandle>
```

**Purpose**: Spawn a background shell command or monitor.

**Key Implementation** (lines 216-221):
```typescript
registerTask(taskState, setAppState)

// Data flows through TaskOutput automatically
shellCommand.background(taskId)
const cancelStallWatchdog = startStallWatchdog(
  taskId, description, kind, toolUseId, agentId
)
```

**Stall Watchdog**: Monitors for interactive prompts (lines 46-104).

---

#### `killShellTasksForAgent()` (LocalShellTask/killShellTasks.ts)

```typescript
export function killShellTasksForAgent(
  agentId: AgentId,
  getAppState: () => AppState,
  setAppState: SetAppState,
): void
```

**Purpose**: Kill all running bash tasks spawned by a given agent.

**Usage**: Called from `runAgent.ts` finally block to prevent orphaned zombies.

**Implementation** (lines 53-76):
```typescript
const tasks = getAppState().tasks ?? {}
for (const [taskId, task] of Object.entries(tasks)) {
  if (
    isLocalShellTask(task) &&
    task.agentId === agentId &&
    task.status === 'running'
  ) {
    logForDebugging(
      `killShellTasksForAgent: killing orphaned shell task ${taskId}`
    )
    killTask(taskId, setAppState)
  }
}
// Purge queued notifications for dead agentId
dequeueAllMatching(cmd => cmd.agentId === agentId)
```

**Memory Leak Prevention**: Comment (lines 50-51):
> Called from runAgent.ts finally block so background processes don't outlive the agent that started them (prevents 10-day fake-logs.sh zombies).

---

#### `stopTask()` (stopTask.ts)

```typescript
export async function stopTask(
  taskId: string,
  context: StopTaskContext,
): Promise<StopTaskResult>
```

**Purpose**: Shared logic for stopping a running task (TaskStopTool + SDK).

**Parameters**:
- `taskId`: Task to stop
- `context`: `{ getAppState, setAppState }`

**Returns**: `{ taskId, taskType, command }`

**Throws**: `StopTaskError` with codes:
- `'not_found'`: Task doesn't exist
- `'not_running'`: Task status !== 'running'
- `'unsupported_type'`: No task implementation found

**Implementation** (lines 70-94):
```typescript
// Bash: suppress "exit code 137" notification (noise)
if (isLocalShellTask(task)) {
  let suppressed = false
  setAppState(prev => {
    const prevTask = prev.tasks[taskId]
    if (!prevTask || prevTask.notified) {
      return prev
    }
    suppressed = true
    return {
      ...prev,
      tasks: {
        ...prev.tasks,
        [taskId]: { ...prevTask, notified: true },
      },
    }
  })
  if (suppressed) {
    emitTaskTerminatedSdk(taskId, 'stopped', {
      toolUseId: task.toolUseId,
      summary: task.description,
    })
  }
}
```

**Key Design**: Suppresses noise for bash tasks but NOT for agent tasks (which need `extractPartialResult()` notification).

---

#### `getPillLabel()` (pillLabel.ts)

```typescript
export function getPillLabel(tasks: BackgroundTaskState[]): string
```

**Purpose**: Generate compact footer-pill label for background tasks.

**Implementation** (lines 10-67):
```typescript
export function getPillLabel(tasks: BackgroundTaskState[]): string {
  const n = tasks.length
  const allSameType = tasks.every(t => t.type === tasks[0]!.type)

  if (allSameType) {
    switch (tasks[0]!.type) {
      case 'local_bash': {
        const monitors = count(tasks, t => 
          t.type === 'local_bash' && t.kind === 'monitor'
        )
        const shells = n - monitors
        const parts: string[] = []
        if (shells > 0) parts.push(shells === 1 ? '1 shell' : `${shells} shells`)
        if (monitors > 0) parts.push(monitors === 1 ? '1 monitor' : `${monitors} monitors`)
        return parts.join(', ')
      }
      case 'in_process_teammate': {
        const teamCount = new Set(
          tasks.map(t => t.type === 'in_process_teammate' 
            ? t.identity.teamName : ''
          )
        ).size
        return teamCount === 1 ? '1 team' : `${teamCount} teams`
      }
      case 'local_agent':
        return n === 1 ? '1 local agent' : `${n} local agents`
      case 'remote_agent': {
        const first = tasks[0]!
        if (n === 1 && first.type === 'remote_agent' && first.isUltraplan) {
          switch (first.ultraplanPhase) {
            case 'plan_ready':
              return `${DIAMOND_FILLED} ultraplan ready`
            case 'needs_input':
              return `${DIAMOND_OPEN} ultraplan needs your input`
            default:
              return `${DIAMOND_OPEN} ultraplan`
          }
        }
        return n === 1 
          ? `${DIAMOND_OPEN} 1 cloud session`
          : `${DIAMOND_OPEN} ${n} cloud sessions`
      }
      // ... other cases
    }
  }
  return `${n} background ${n === 1 ? 'task' : 'tasks'}`
}
```

**Display Logic**:
- Homogeneous tasks: Specific label ("2 shells", "1 team", "3 local agents")
- Heterogeneous tasks: Generic label ("5 background tasks")
- Ultraplan phases: Diamond symbols (◇ open, ◆ filled)

---

#### `pillNeedsCta()` (pillLabel.ts)

```typescript
export function pillNeedsCta(tasks: BackgroundTaskState[]): boolean
```

**Purpose**: Determine if pill should show " · ↓ to view" call-to-action.

**Implementation** (lines 74-82):
```typescript
export function pillNeedsCta(tasks: BackgroundTaskState[]): boolean {
  if (tasks.length !== 1) return false
  const t = tasks[0]!
  return (
    t.type === 'remote_agent' &&
    t.isUltraplan === true &&
    t.ultraplanPhase !== undefined
  )
}
```

**Logic**: CTA only for ultraplan tasks in attention states ('needs_input', 'plan_ready').

---

#### `updateProgressFromMessage()` (LocalAgentTask/LocalAgentTask.tsx)

```typescript
export function updateProgressFromMessage(
  tracker: ProgressTracker,
  message: Message,
  resolveActivityDescription?: ActivityDescriptionResolver,
  tools?: Tools,
): void
```

**Purpose**: Update progress tracker from assistant message.

**Implementation** (lines 68-96):
```typescript
export function updateProgressFromMessage(tracker: ProgressTracker, message: Message, resolveActivityDescription?: ActivityDescriptionResolver, tools?: Tools): void {
  if (message.type !== 'assistant') {
    return
  }
  const usage = message.message.usage
  tracker.latestInputTokens = usage.input_tokens + 
    (usage.cache_creation_input_tokens ?? 0) + 
    (usage.cache_read_input_tokens ?? 0)
  tracker.cumulativeOutputTokens += usage.output_tokens
  
  for (const content of message.message.content) {
    if (content.type === 'tool_use') {
      tracker.toolUseCount++
      if (content.name !== SYNTHETIC_OUTPUT_TOOL_NAME) {
        const input = content.input as Record<string, unknown>
        const classification = tools 
          ? getToolSearchOrReadInfo(content.name, input, tools)
          : undefined
        tracker.recentActivities.push({
          toolName: content.name,
          input,
          activityDescription: resolveActivityDescription?.(content.name, input),
          isSearch: classification?.isSearch,
          isRead: classification?.isRead,
        })
      }
    }
  }
  while (tracker.recentActivities.length > MAX_RECENT_ACTIVITIES) {
    tracker.recentActivities.shift()
  }
}
```

**Token Tracking**:
- `latestInputTokens`: Cumulative (includes all previous context)
- `cumulativeOutputTokens`: Sum of per-turn output tokens

**Activity Classification**:
- Omit `SYNTHETIC_OUTPUT_TOOL_NAME` from preview (internal tool)
- Pre-compute `activityDescription` via tool resolver
- Cap `recentActivities` at `MAX_RECENT_ACTIVITIES` (5)

---

#### `registerCompletionChecker()` (RemoteAgentTask/RemoteAgentTask.tsx)

```typescript
export function registerCompletionChecker(
  remoteTaskType: RemoteTaskType,
  checker: RemoteTaskCompletionChecker,
): void
```

**Purpose**: Register a completion checker for a remote task type.

**Usage Pattern**:
```typescript
registerCompletionChecker('autofix-pr', async (metadata) => {
  if (!metadata) return null
  const { owner, repo, prNumber } = metadata as AutofixPrRemoteTaskMetadata
  const pr = await github.pulls.get({ owner, repo, pull_number: prNumber })
  if (pr.data.state === 'closed') {
    return `PR #${prNumber} was merged`
  }
  return null // Keep polling
})
```

**Polling**: Invoked on every poll tick for matching `remoteTaskType`.

---

#### `extractPlanFromLog()` (RemoteAgentTask/RemoteAgentTask.tsx)

```typescript
export function extractPlanFromLog(log: SDKMessage[]): string | null
```

**Purpose**: Extract ultraplan content from remote session log.

**Implementation** (lines 208-218):
```typescript
export function extractPlanFromLog(log: SDKMessage[]): string | null {
  for (let i = log.length - 1; i >= 0; i--) {
    const msg = log[i]
    if (msg?.type !== 'assistant') continue
    const fullText = extractTextContent(msg.message.content, '\n')
    const plan = extractTag(fullText, ULTRAPLAN_TAG)
    if (plan?.trim()) return plan.trim()
  }
  return null
}
```

**Algorithm**: Walk backwards through assistant messages to find first `<ultraplan>` tag.

---

#### `appendCappedMessage()` (InProcessTeammateTask/types.ts)

```typescript
export function appendCappedMessage<T>(
  prev: readonly T[] | undefined,
  item: T,
): T[]
```

**Purpose**: Append item to message array, capping at `TEAMMATE_MESSAGES_UI_CAP` (50).

**Implementation** (lines 108-121):
```typescript
export function appendCappedMessage<T>(
  prev: readonly T[] | undefined,
  item: T,
): T[] {
  if (prev === undefined || prev.length === 0) {
    return [item]
  }
  if (prev.length >= TEAMMATE_MESSAGES_UI_CAP) {
    const next = prev.slice(-(TEAMMATE_MESSAGES_UI_CAP - 1))
    next.push(item)
    return next
  }
  return [...prev, item]
}
```

**Immutability**: Always returns new array (AppState requirement).

**Capping Strategy**: Drop oldest, keep newest 49 + new item = 50 total.

---

#### `getRunningTeammatesSorted()` (InProcessTeammateTask/InProcessTeammateTask.tsx)

```typescript
export function getRunningTeammatesSorted(
  tasks: Record<string, TaskStateBase>,
): InProcessTeammateTaskState[]
```

**Purpose**: Get running teammates sorted alphabetically by agentName.

**Implementation** (lines 123-125):
```typescript
export function getRunningTeammatesSorted(
  tasks: Record<string, TaskStateBase>,
): InProcessTeammateTaskState[] {
  return getAllInProcessTeammateTasks(tasks)
    .filter(t => t.status === 'running')
    .sort((a, b) => a.identity.agentName.localeCompare(b.identity.agentName))
}
```

**Critical Consistency** (from comment, lines 118-122):
> Shared between TeammateSpinnerTree display, PromptInput footer selector, and useBackgroundTaskNavigation — selectedIPAgentIndex maps into this array, so all three must agree on sort order.

---

#### `looksLikePrompt()` (LocalShellTask/LocalShellTask.tsx)

```typescript
export function looksLikePrompt(tail: string): boolean
```

**Purpose**: Detect if command output ends with interactive prompt.

**Implementation** (lines 32-42):
```typescript
const PROMPT_PATTERNS = [
  /\(y\/n\)/i,      // (Y/n), (y/N)
  /\[y\/n\]/i,      // [Y/n], [Y/N]
  /\(yes\/no\)/i,
  /\b(?:Do you|Would you|Shall I|Are you sure|Ready to)\b.*\? *$/i,
  /Press (any key|Enter)/i,
  /Continue\?/i,
  /Overwrite\?/i,
]

export function looksLikePrompt(tail: string): boolean {
  const lastLine = tail.trimEnd().split('\n').pop() ?? ''
  return PROMPT_PATTERNS.some(p => p.test(lastLine))
}
```

**Usage**: Stall watchdog fires notification only if output stalled AND tail looks like prompt.

**See**: CC-1175 for interactive prompt detection.

---

## Line-by-Line Analysis

### Task State Base Creation (DreamTask.ts lines 60-72)

```typescript
const id = generateTaskId('dream')
const task: DreamTaskState = {
  ...createTaskStateBase(id, 'dream', 'dreaming'),
  type: 'dream',
  status: 'running',
  phase: 'starting',
  sessionsReviewing: opts.sessionsReviewing,
  filesTouched: [],
  turns: [],
  abortController: opts.abortController,
  priorMtime: opts.priorMtime,
}
registerTask(task, setAppState)
```

**Purpose**: Create and register new dream task.

**`createTaskStateBase()` Fields** (probable):
- `id`: Task identifier
- `type`: 'dream'
- `description`: 'dreaming'
- `status`: 'running'
- `startTime`: Date.now()
- `notified`: false
- `toolUseId`: undefined

---

### Dream Task Kill with Lock Rollback (DreamTask.ts lines 136-156)

```typescript
async kill(taskId, setAppState) {
  let priorMtime: number | undefined
  updateTaskState<DreamTaskState>(taskId, setAppState, task => {
    if (task.status !== 'running') return task
    task.abortController?.abort()
    priorMtime = task.priorMtime
    return {
      ...task,
      status: 'killed',
      endTime: Date.now(),
      notified: true,
      abortController: undefined,
    }
  })
  // Rewind lock mtime so next session can retry
  if (priorMtime !== undefined) {
    await rollbackConsolidationLock(priorMtime)
  }
}
```

**Purpose**: Kill dream task and rollback consolidation lock.

**Key Pattern**: Extract `priorMtime` before state update, use after for lock rollback.

**Lock Rollback**: Same path as fork-failure catch in `autoDream.ts`.

---

### Shell Task Stall Watchdog (LocalShellTask.tsx lines 46-104)

```typescript
function startStallWatchdog(
  taskId: string,
  description: string,
  kind: BashTaskKind | undefined,
  toolUseId?: string,
  agentId?: AgentId,
): () => void {
  if (kind === 'monitor') return () => {}
  const outputPath = getTaskOutputPath(taskId)
  let lastSize = 0
  let lastGrowth = Date.now()
  let cancelled = false
  
  const timer = setInterval(() => {
    void stat(outputPath).then(s => {
      if (s.size > lastSize) {
        lastSize = s.size
        lastGrowth = Date.now()
        return
      }
      if (Date.now() - lastGrowth < STALL_THRESHOLD_MS) return
      void tailFile(outputPath, STALL_TAIL_BYTES).then(({ content }) => {
        if (cancelled) return
        if (!looksLikePrompt(content)) {
          lastGrowth = Date.now() // Reset for next check
          return
        }
        cancelled = true
        clearInterval(timer)
        // ... enqueue notification
      })
    })
  }, STALL_CHECK_INTERVAL_MS)
  
  return () => {
    cancelled = true
    clearInterval(timer)
  }
}
```

**Constants**:
- `STALL_CHECK_INTERVAL_MS = 5_000`
- `STALL_THRESHOLD_MS = 45_000` (45 seconds)
- `STALL_TAIL_BYTES = 1024`

**Logic Flow**:
1. Check file size every 5 seconds
2. If size hasn't grown in 45 seconds, tail last 1KB
3. If tail looks like prompt, fire notification
4. Return cancel function for cleanup

**Notification Pattern** (lines 74-88):
```typescript
const toolUseIdLine = toolUseId 
  ? `\n<${TOOL_USE_ID_TAG}>${toolUseId}</${TOOL_USE_ID_TAG}>` 
  : ''
const summary = `${BACKGROUND_BASH_SUMMARY_PREFIX}"${description}" appears to be waiting for interactive input`
const message = `<${TASK_NOTIFICATION_TAG}>
<${TASK_ID_TAG}>${taskId}</${TASK_ID_TAG}>${toolUseIdLine}
<${OUTPUT_FILE_TAG}>${outputPath}</${OUTPUT_FILE_TAG}>
<${SUMMARY_TAG}>${escapeXml(summary)}</${SUMMARY_TAG}>
</${TASK_NOTIFICATION_TAG}>
Last output:
${content.trimEnd()}

The command is likely blocked on an interactive prompt...`
```

**Key Design**: No `<status>` tag — SDK treats unknown status as 'completed', falsely closing task.

---

### Notification Deduplication Pattern (Multiple Files)

**Pattern** (from LocalAgentTask.tsx lines 227-240):
```typescript
let shouldEnqueue = false
updateTaskState<LocalAgentTaskState>(taskId, setAppState, task => {
  if (task.notified) {
    return task
  }
  shouldEnqueue = true
  return { ...task, notified: true }
})

if (!shouldEnqueue) {
  return
}
```

**Purpose**: Atomically check-and-set `notified` flag to prevent duplicate notifications.

**Race Condition Prevention**:
```
Thread 1: Check notified (false) → Set notified (true) → Enqueue
Thread 2: Check notified (false) → Set notified (true) → Enqueue  // BUG!
```

**Fix**: Atomic check-and-set in single `updateTaskState` callback.

---

### Progress Delta Computation (LocalAgentTask.tsx lines 130-132)

```typescript
// Track what we last reported for computing deltas
lastReportedToolCount: number
lastReportedTokenCount: number
```

**Purpose**: Compute deltas for progress notifications.

**Usage Pattern**:
```typescript
const deltaTools = progress.toolUseCount - task.lastReportedToolCount
const deltaTokens = progress.tokenCount - task.lastReportedTokenCount
if (deltaTools > 0 || deltaTokens > 0) {
  emitTaskProgress(taskId, {
    toolUseCount: deltaTools,
    tokenCount: deltaTokens,
  })
}
updateTaskState(task => ({
  ...task,
  lastReportedToolCount: progress.toolUseCount,
  lastReportedTokenCount: progress.tokenCount,
}))
```

---

### Agent Context Wrapping (LocalMainSessionTask.ts lines 368-375)

```typescript
const agentContext: SubagentContext = {
  agentId: taskId,
  agentType: 'subagent',
  subagentName: 'main-session',
  isBuiltIn: true,
}

void runWithAgentContext(agentContext, async () => {
  // ... query() execution
})
```

**Purpose**: AsyncLocalStorage isolation for concurrent task execution.

**Benefit**: Skill invocations scope to this task's `agentId` (not `null`).

**Clear Conversation Integration** (from comment, lines 365-367):
> This lets `clearInvokedSkills(preservedAgentIds)` selectively preserve this task's skills across `/clear`.

---

### Ultraplan Phase Detection (pillLabel.ts lines 43-51)

```typescript
if (n === 1 && first.type === 'remote_agent' && first.isUltraplan) {
  switch (first.ultraplanPhase) {
    case 'plan_ready':
      return `${DIAMOND_FILLED} ultraplan ready`
    case 'needs_input':
      return `${DIAMOND_OPEN} ultraplan needs your input`
    default:
      return `${DIAMOND_OPEN} ultraplan`
  }
}
```

**Phase States**:
- `undefined`: Running normally (no attention needed)
- `'needs_input'`: Remote asked clarifying question, idle
- `'plan_ready'`: ExitPlanMode awaiting browser approval

**Symbol Convention**:
- `DIAMOND_OPEN` (◇): Running or needs input
- `DIAMOND_FILLED` (◆): Plan ready for approval

---

### Task Output Symlink Initialization (LocalMainSessionTask.ts lines 107-110)

```typescript
void initTaskOutputAsSymlink(
  taskId,
  getAgentTranscriptPath(asAgentId(taskId)),
)
```

**Purpose**: Link task output to isolated transcript file.

**Design Rationale** (from comment, lines 103-106):
> Do NOT use `getTranscriptPath()` — that's the main session's file, and writing there from a background query after `/clear` would corrupt the post-clear conversation. The isolated path lets this task survive `/clear`: the symlink re-link in `clearConversation` handles session ID changes.

---

### Files Touched Deduplication (DreamTask.ts lines 83-84)

```typescript
const seen = new Set(task.filesTouched)
const newTouched = touchedPaths.filter(p => !seen.has(p) && seen.add(p))
```

**Purpose**: Deduplicate file paths across dream turns.

**Pattern**: Use Set for O(1) lookup + in-place tracking via `seen.add(p)` return value.

---

### Message Array Capping (InProcessTeammateTask/types.ts lines 115-118)

```typescript
if (prev.length >= TEAMMATE_MESSAGES_UI_CAP) {
  const next = prev.slice(-(TEAMMATE_MESSAGES_UI_CAP - 1))
  next.push(item)
  return next
}
```

**Purpose**: Drop oldest message, keep newest 49 + new item = 50 total.

**Slice Math**: `slice(-(N - 1))` drops first `length - (N - 1)` elements.

---

## Integration Points

### Task Registry (state/AppState.ts)

**Location**: `AppState.tasks`

**Type**:
```typescript
tasks: Record<string, TaskState>
```

**Registration Flow**:
```
registerTask(task, setAppState)
    ↓
setAppState(prev => ({
  ...prev,
  tasks: { ...prev.tasks, [task.id]: task },
}))
    ↓
Task appears in UI
```

---

### Task Framework (utils/task/framework.ts)

**Core Functions**:
- `registerTask()`: Add task to state
- `updateTaskState()`: Immutable state update
- `generateTaskId()`: Task ID generation

**`updateTaskState()` Pattern**:
```typescript
updateTaskState<DreamTaskState>(taskId, setAppState, task => {
  // Mutate copy, return new state
  return { ...task, status: 'completed' }
})
```

---

### Notification System (utils/messageQueueManager.ts)

**Notification Format**:
```xml
<task_notification>
<task_id>abc123</task_id>
<tool_use_id>tool_456</tool_use_id>
<output_file>/path/to/output</output_file>
<status>completed</status>
<summary>Agent "foo" completed</summary>
</task_notification>
```

**Priority Levels**:
- `'now'`: Immediate (user-facing)
- `'next'`: After current tool round
- `'later'`: Batch with other notifications

**Enqueue Pattern**:
```typescript
enqueuePendingNotification({
  value: message,
  mode: 'task-notification',
  priority: 'later',
  agentId,
})
```

---

### Disk Output (utils/task/diskOutput.ts)

**Functions**:
- `initTaskOutput()`: Create task output directory
- `initTaskOutputAsSymlink()`: Create symlink to transcript
- `getTaskOutputPath()`: Get output file path
- `evictTaskOutput()`: Remove output after completion
- `appendTaskOutput()`: Append to output file

**Lifecycle**:
```
spawnShellTask()
    ↓
initTaskOutputAsSymlink(taskId, transcriptPath)
    ↓
Shell writes to output file
    ↓
evictTaskOutput(taskId) on complete/kill
```

---

### SDK Event Queue (utils/sdkEventQueue.ts)

**Events**:
- `emitTaskTerminatedSdk()`: Task completion/failure/kill
- `emitTaskProgress()`: Progress updates

**Usage** (stopTask.ts lines 90-93):
```typescript
emitTaskTerminatedSdk(taskId, 'stopped', {
  toolUseId: task.toolUseId,
  summary: task.description,
})
```

**Purpose**: Notify SDK consumers of task lifecycle events.

---

### Agent Context (utils/agentContext.ts)

**Pattern**:
```typescript
const agentContext: SubagentContext = {
  agentId: taskId,
  agentType: 'subagent',
  subagentName: 'main-session',
  isBuiltIn: true,
}

void runWithAgentContext(agentContext, async () => {
  // All async operations inherit agentId
  await someAsyncOperation()
})
```

**Benefit**: AsyncLocalStorage propagates `agentId` across async boundaries.

---

### Permission System (utils/permissions/PermissionMode.ts)

**Integration**: InProcessTeammateTask tracks `permissionMode` independently.

**Cycle**: Shift+Tab when viewing teammate cycles permission mode.

---

### Todo System (utils/todo/types.ts)

**Integration**: RemoteAgentTask has `todoList: TodoList`.

**Type**:
```typescript
type TodoList = {
  todos: TodoItem[]
  // ...
}
```

---

### Ultraplan System (utils/ultraplan/ccrSession.ts)

**Integration**: RemoteAgentTask tracks `ultraplanPhase`.

**Phases**:
- `'running'`: Normal execution
- `'needs_input'`: Awaiting user input
- `'plan_ready'`: Plan awaiting approval

---

## Task Lifecycle

### Registration

```
Task Spawn (e.g., spawnShellTask, registerDreamTask, backgroundAgentTask)
    ↓
generateTaskId(type) → unique ID
    ↓
createTaskStateBase(id, type, description) → base state
    ↓
{ ...baseState, ...typeSpecificFields } → complete state
    ↓
registerTask(task, setAppState) → AppState.tasks[taskId] = task
    ↓
initTaskOutputAsSymlink(taskId, transcriptPath) → disk output
    ↓
Task appears in background pill / coordinator panel
```

---

### Update

```
Task Progress (message received, tool executed, etc.)
    ↓
updateTaskState(taskId, setAppState, task => {
  return { ...task, progress: newProgress }
})
    ↓
AppState updated immutably
    ↓
React re-renders task UI components
```

---

### Completion

```
Task Finishes (command exits, agent completes, etc.)
    ↓
updateTaskState(taskId, setAppState, task => ({
  ...task,
  status: 'completed' | 'failed' | 'killed',
  endTime: Date.now(),
  notified: true,  // Atomic check-and-set prevents duplicates
}))
    ↓
evictTaskOutput(taskId) → Remove disk output
    ↓
enqueuePendingNotification({ value: xml, mode: 'task-notification' })
    ↓
Notification sent to model + SDK
```

---

### Kill

```
Kill Request (user presses stop, agent exits, cleanup)
    ↓
Task.kill(taskId, setAppState) → type-specific kill logic
    ↓
LocalShellTask: shellCommand.kill(), cleanupTimeout
LocalAgentTask: abortController.abort()
RemoteAgentTask: Archive remote session
InProcessTeammateTask: killInProcessTeammate()
DreamTask: abortController.abort(), rollbackConsolidationLock()
    ↓
updateTaskState(taskId, setAppState, task => ({
  ...task,
  status: 'killed',
  endTime: Date.now(),
  notified: true,
}))
    ↓
evictTaskOutput(taskId)
    ↓
dequeueAllMatching(cmd => cmd.agentId === agentId) → Purge queue
```

---

## Summary

The `tasks/` module is a **comprehensive task framework** implementing:

1. **7 Task Types**: LocalShellTask, LocalAgentTask, LocalMainSessionTask, RemoteAgentTask, InProcessTeammateTask, DreamTask, LocalWorkflowTask, MonitorMcpTask

2. **Unified Lifecycle**: Register → Update → Complete/Fail/Kill with consistent patterns

3. **Type-Safe Design**: Discriminated unions, branded IDs, exhaustive switches

4. **Memory Efficiency**: Capped arrays, delta tracking, eviction policies

5. **Notification System**: XML-based, deduplicated, priority-queued

6. **Disk Output**: Per-task symlinks, isolated transcripts, eviction on completion

7. **Agent Isolation**: AsyncLocalStorage for concurrent task execution

**~2,000 lines** across 12 files implementing a production-grade task management system.
