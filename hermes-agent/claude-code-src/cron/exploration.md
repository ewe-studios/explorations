# Cron/Scheduling System Deep Dive Exploration

**Source Directory:** `claude-code-src/src/utils/`  
**Status:** Complete

---

## Module Overview

The cron/scheduling system in Claude Code enables users to schedule recurring or one-shot prompts to be executed automatically at specified times. The system consists of:

1. **Cron Scheduler** — Main timer loop that checks and fires tasks every second
2. **Cron Expression Parser** — Parses 5-field cron expressions in local timezone
3. **Task Storage** — Persists tasks to `.claude/scheduled_tasks.json` or memory
4. **Lock System** — Prevents double-firing across multiple Claude sessions
5. **Jitter System** — Staggered firing to prevent thundering herd at :00/:30
6. **GrowthBook Integration** — Runtime configuration of jitter parameters

**Key Design Decisions:**
- File-based locking with O_EXCL for atomic lease acquisition
- Chokidar file watcher for reactive task updates
- Per-task deterministic jitter from UUID (no shared randomness)
- Session vs durable tasks (memory vs disk)
- Teammate routing for in-process subagent tasks

---

## Architecture Diagram

```
┌─────────────────────────────────────────────────────────────────────────┐
│                         Claude Code REPL                                │
│  ┌─────────────────────────────────────────────────────────────────┐   │
│  │                    useScheduledTasks Hook                        │   │
│  │  ┌──────────────────────────────────────────────────────────┐   │   │
│  │  │              createCronScheduler()                        │   │   │
│  │  │  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐   │   │   │
│  │  │  │ check()      │  │ enable()     │  │ process()    │   │   │   │
│  │  │  │ (1s timer)   │  │ (lock+watch) │  │ (fire logic) │   │   │   │
│  │  │  └──────┬───────┘  └──────┬───────┘  └──────┬───────┘   │   │   │
│  │  │         │                 │                 │            │   │   │
│  │  │         ▼                 ▼                 ▼            │   │   │
│  │  │  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐   │   │   │
│  │  │  │ nextFireAt   │  │ chokidar     │  │ enqueuePending│   │   │   │
│  │  │  │ Map          │  │ watcher      │  │ notification │   │   │   │
│  │  │  └──────────────┘  └──────────────┘  └──────────────┘   │   │   │
│  │  └──────────────────────────────────────────────────────────┘   │   │
│  └─────────────────────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────────────────────┘
                                    │
                                    ▼
┌─────────────────────────────────────────────────────────────────────────┐
│                      File System (.claude/)                             │
│  ┌──────────────────────┐         ┌──────────────────────┐            │
│  │ scheduled_tasks.json │         │ scheduled_tasks.lock │            │
│  │ [                    │         │ {                    │            │
│  │   {id, cron, prompt, │         │   sessionId: "...",  │            │
│  │    lastFiredAt, ...} │         │   pid: 12345,        │            │
│  │ ]                    │         │   acquiredAt: ...    │            │
│  └──────────────────────┘         │ }                    │            │
│                                   └──────────────────────┘            │
└─────────────────────────────────────────────────────────────────────────┘
                                    ▲
                                    │
┌─────────────────────────────────────────────────────────────────────────┐
│                        CronCreate Tool                                  │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐                 │
│  │ validateInput│  │ addCronTask  │  │ setScheduled │                 │
│  │ (cron parse) │  │ (write JSON) │  │ TasksEnabled │                 │
│  └──────────────┘  └──────────────┘  └──────────────┘                 │
└─────────────────────────────────────────────────────────────────────────┘
```

---

## File Inventory

| File | Lines | Purpose |
|------|-------|---------|
| `cronScheduler.ts` | 566 | Main scheduler: timer loop, lock acquisition, fire logic |
| `cron.ts` | 309 | Cron expression parsing, next-run calculation |
| `cronTasks.ts` | 459 | Task storage (JSON read/write), jitter calculation |
| `cronTasksLock.ts` | 196 | Multi-session lock with PID liveness probe |
| `cronJitterConfig.ts` | 76 | Jitter configuration (GrowthBook-backed) |
| `CronCreateTool.ts` | 158 | Tool implementation for creating tasks |
| `prompt.ts` | 136 | User-facing prompts and validation |
| `useScheduledTasks.ts` | 140 | React hook that starts scheduler in REPL |
| `scheduleRemoteAgents.ts` | 448 | Remote trigger skill (separate system) |

**Total:** 2,682 lines

---

## Key Components

### 1. Cron Expression Parser (`cron.ts`)

Parses 5-field cron expressions in local timezone: `M H DoM Mon DoW`

```typescript
export function parseCronExpression(expr: string): CronFields | null {
  const parts = expr.trim().split(/\s+/)
  if (parts.length !== 5) return null
  
  const expanded: number[][] = []
  for (let i = 0; i < 5; i++) {
    const result = expandField(parts[i]!, FIELD_RANGES[i]!)
    if (!result) return null
    expanded.push(result)
  }
  
  return {
    minute: expanded[0]!,
    hour: expanded[1]!,
    dayOfMonth: expanded[2]!,
    month: expanded[3]!,
    dayOfWeek: expanded[4]!,
  }
}

// Field expansion syntax:
// *       — All values (wildcard)
// */N     — Every N (step)
// N-M     — Range
// N-M/S   — Range with step
// N,M,O   — List
```

**Next Run Calculation:**

```typescript
export function computeNextCronRun(fields: CronFields, from: Date): Date | null {
  const minuteSet = new Set(fields.minute)
  const hourSet = new Set(fields.hour)
  const domSet = new Set(fields.dayOfMonth)
  const monthSet = new Set(fields.month)
  const dowSet = new Set(fields.dayOfWeek)

  const domWild = fields.dayOfMonth.length === 31  // Unconstrained
  const dowWild = fields.dayOfWeek.length === 7    // Unconstrained

  let t = new Date(from.getTime())
  t.setSeconds(0, 0)
  t.setMinutes(t.getMinutes() + 1) // Strictly after `from`

  for (let i = 0; i < 366 * 24 * 60; i++) {  // Max 1 year
    // Month check
    const month = t.getMonth() + 1
    if (!monthSet.has(month)) {
      t.setMonth(t.getMonth() + 1, 1)
      t.setHours(0, 0, 0, 0)
      continue
    }
    
    // Day check — OR semantics for DoM/DOW
    const dom = t.getDate()
    const dow = t.getDay()
    const dayMatches = domWild && dowWild ? true :
      domWild ? dowSet.has(dow) :
      dowWild ? domSet.has(dom) :
      domSet.has(dom) || dowSet.has(dom)  // OR: either matches

    if (!dayMatches) {
      t.setDate(t.getDate() + 1)
      t.setHours(0, 0, 0, 0)
      continue
    }

    // Hour and minute checks
    if (!hourSet.has(t.getHours())) {
      t.setHours(t.getHours() + 1)
      continue
    }
    if (!minuteSet.has(t.getMinutes())) {
      t.setMinutes(t.getMinutes() + 1)
      continue
    }

    return t
  }
  return null
}
```

**Key Design: OR Semantics**

When both `dayOfMonth` and `dayOfWeek` are constrained (not wildcards), the cron standard specifies **OR** semantics: a date matches if EITHER the day-of-month OR day-of-week matches. This is why `0 0 13 * 5` fires on the 13th of any month AND every Friday (not just Friday the 13th).

---

### 2. Task Storage (`cronTasks.ts`)

**Task Type Definition:**

```typescript
export type CronTask = {
  id: string           // 8-char hex UUID (from bootstrap/getSessionId)
  cron: string         // 5-field cron expression
  prompt: string       // Prompt to enqueue when fired
  createdAt: number    // Epoch ms — anchor for jitter/missed detection
  lastFiredAt?: number // Epoch ms — for recurring reschedule
  recurring?: boolean  // true = reschedule after fire, false = delete
  permanent?: boolean  // Exempt from 7-day auto-expiry (assistant mode)
  durable?: boolean    // true = persist to JSON, false = session memory only
  agentId?: string     // Route to teammate subagent (not lead session)
}
```

**Jitter System:**

The jitter system prevents a "thundering herd" when many users schedule tasks at common times like 9:00 AM (:00, :30). Each task gets a deterministic per-task offset based on its UUID.

```typescript
// Deterministic per-task fraction from UUID (0.0 to 1.0)
function jitterFrac(taskId: string): number {
  const frac = parseInt(taskId.slice(0, 8), 16) / 0x1_0000_0000
  return Number.isFinite(frac) ? frac : 0
}

// Recurring tasks: DELAY forward by fraction of interval
export function jitteredNextCronRunMs(
  cron: string,
  fromMs: number,
  taskId: string,
  cfg: CronJitterConfig = DEFAULT_CRON_JITTER_CONFIG,
): number | null {
  const t1 = nextCronRunMs(cron, fromMs)
  if (t1 === null) return null
  const t2 = nextCronRunMs(cron, t1)
  if (t2 === null) return t1 // No second match (pinned date, e.g., monthly 31st)
  
  // Delay = min(frac * interval, cap)
  const jitter = Math.min(
    jitterFrac(taskId) * cfg.recurringFrac * (t2 - t1),
    cfg.recurringCapMs,
  )
  return t1 + jitter // Fire LATE
}

// One-shot tasks: PULL BACKWARD when landing on hot minutes
export function oneShotJitteredNextCronRunMs(
  cron: string,
  fromMs: number,
  taskId: string,
  cfg: CronJitterConfig = DEFAULT_CRON_JITTER_CONFIG,
): number | null {
  const t1 = nextCronRunMs(cron, fromMs)
  if (t1 === null) return null
  
  // Only jitter if lands on :00 or :30 (configurable mod)
  if (new Date(t1).getMinutes() % cfg.oneShotMinuteMod !== 0) return t1
  
  // Pull backward by random amount up to max
  const lead = cfg.oneShotFloorMs +
    jitterFrac(taskId) * (cfg.oneShotMaxMs - cfg.oneShotFloorMs)
  return Math.max(t1 - lead, fromMs) // Fire EARLY, but not before creation
}
```

**Default Jitter Config:**

```typescript
export const DEFAULT_CRON_JITTER_CONFIG: CronJitterConfig = {
  recurringFrac: 0.1,           // 10% of interval
  recurringCapMs: 15 * 60 * 1000, // Max 15 min delay
  oneShotMaxMs: 90 * 1000,      // Max 90 sec early
  oneShotFloorMs: 0,            // Min early (can be raised)
  oneShotMinuteMod: 30,         // Jitter :00 and :30
  recurringMaxAgeMs: 7 * 24 * 60 * 60 * 1000, // 7 days auto-expiry
}
```

**GrowthBook Integration:**

Jitter config can be overridden at runtime via feature flags:

```typescript
export function getCronJitterConfig(): CronJitterConfig {
  const fromFlags = getFeatureValue_CACHED_MAY_BE_STALE(
    'tengu_kairos_cron_config',
    DEFAULT_CRON_JITTER_CONFIG,
  )
  // Deep merge with defaults
  return { ...DEFAULT_CRON_JITTER_CONFIG, ...fromFlags }
}
```

This allows ops to tune jitter parameters during incidents without a deploy.

---

### 3. Multi-Session Lock (`cronTasksLock.ts`)

When multiple Claude sessions share a working directory (e.g., multiple terminal tabs), only ONE should fire cron tasks to prevent duplicates.

**Lock Structure:**

```typescript
type SchedulerLock = {
  sessionId: string    // From getSessionId()
  pid: number          // Process ID for liveness probe
  acquiredAt: number   // Epoch ms
}
```

**Lock Acquisition (Atomic O_EXCL):**

```typescript
export async function tryAcquireSchedulerLock(
  opts?: SchedulerLockOptions,
): Promise<boolean> {
  const sessionId = opts?.lockIdentity ?? getSessionId()
  const lock: SchedulerLock = {
    sessionId,
    pid: process.pid,
    acquiredAt: Date.now(),
  }

  // Try atomic create — O_EXCL flag ('wx' in Node.js)
  if (await tryCreateExclusive(lock, dir)) {
    registerLockCleanup(opts)
    return true
  }

  const existing = await readLock(dir)

  // Already ours (re-acquire after --resume or restart)
  if (existing?.sessionId === sessionId) {
    if (existing.pid !== process.pid) {
      await writeFile(getLockPath(dir), jsonStringify(lock))
    }
    return true
  }

  // Another session owns it — check if alive
  if (existing && isProcessRunning(existing.pid)) {
    return false // Blocked
  }

  // Stale lock (dead process) — recover
  await unlink(getLockPath(dir)).catch(() => {})
  return await tryCreateExclusive(lock, dir)
}
```

**PID Liveness Probe:**

```typescript
function isProcessRunning(pid: number): boolean {
  try {
    process.kill(pid, 0) // Signal 0 = check existence, don't kill
    return true
  } catch {
    return false
  }
}
```

**Lock Recovery:**

If a session crashes, the lock file remains. New sessions probe the PID every 5 seconds and recover the lock if the process is dead.

---

### 4. Cron Scheduler (`cronScheduler.ts`)

**Main Scheduler Creation:**

```typescript
export function createCronScheduler(options: CronSchedulerOptions): CronScheduler {
  const {
    onFire,           // (prompt: string) => void
    isLoading,        // () => boolean
    onFireTask,       // (task: CronTask) => void (for teammate routing)
    onMissed,         // (tasks: CronTask[]) => void
    getJitterConfig,  // () => CronJitterConfig
    isKilled,         // () => boolean (GrowthBook killswitch)
    filter,           // (t: CronTask) => boolean
  } = options

  let tasks: CronTask[] = []
  let nextFireAt = new Map<string, number>()
  let isOwner = false
  let checkTimer: ReturnType<typeof setInterval> | null = null
  let watcher: chokidar.FSWatcher | null = null

  // Check every 1 second
  async function check() {
    if (isKilled?.()) return
    if (isLoading() && !assistantMode) return  // Defer during loading

    const now = Date.now()
    const seen = new Set<string>()
    const firedFileRecurring: string[] = []
    const jitterCfg = getJitterConfig?.() ?? DEFAULT_CRON_JITTER_CONFIG

    function process(t: CronTask, isSession: boolean) {
      // First sight — compute next fire time with jitter
      let next = nextFireAt.get(t.id)
      if (next === undefined) {
        next = t.recurring
          ? jitteredNextCronRunMs(t.cron, t.lastFiredAt ?? t.createdAt, t.id, jitterCfg)
          : oneShotJitteredNextCronRunMs(t.cron, t.createdAt, t.id, jitterCfg)
        nextFireAt.set(t.id, next)
      }

      // Not yet — skip
      if (now < next) return

      // Time to fire!
      seen.add(t.id)

      // Callback — either route to teammate or enqueue for lead
      if (onFireTask) {
        onFireTask(t)
      } else {
        onFire(t.prompt)
      }

      // Recurring: reschedule
      if (t.recurring) {
        const newNext = jitteredNextCronRunMs(t.cron, now, t.id, jitterCfg) ?? Infinity
        nextFireAt.set(t.id, newNext)
        if (!isSession) {
          firedFileRecurring.push(t.id) // Batch persist lastFiredAt
        }
      } else if (isSession) {
        // One-shot session: remove from memory
        removeSessionCronTasks([t.id])
        nextFireAt.delete(t.id)
      } else {
        // One-shot durable: delete from JSON
        inFlight.add(t.id)
        void removeCronTasks([t.id], dir).finally(() => inFlight.delete(t.id))
        nextFireAt.delete(t.id)
      }
    }

    // Process file tasks (only if we own scheduler lock)
    if (isOwner) {
      for (const t of tasks) process(t, false)
      // Batch persist lastFiredAt for recurring tasks
      if (firedFileRecurring.length > 0) {
        void markCronTasksFired(firedFileRecurring, now, dir)
      }
    }

    // Process session tasks (no lock needed — process-private)
    if (dir === undefined) {
      for (const t of getSessionCronTasks()) process(t, true)
    }
  }

  return {
    async start() {
      await enable()
    },
    stop() {
      stopped = true
      clearInterval(checkTimer)
      clearInterval(lockProbeTimer)
      watcher?.close()
    },
  }
}
```

**Lock Acquisition and File Watcher:**

```typescript
async function enable() {
  if (stopped) return
  
  const { default: chokidar } = await import('chokidar')
  
  // Acquire scheduler lock
  isOwner = await tryAcquireSchedulerLock(lockOpts).catch(() => false)
  
  if (!isOwner) {
    // Probe every 5 seconds to take over if owner dies
    lockProbeTimer = setInterval(() => {
      void tryAcquireSchedulerLock(lockOpts).then(owned => {
        if (owned) {
          isOwner = true
          clearInterval(lockProbeTimer)
        }
      })
    }, LOCK_PROBE_INTERVAL_MS)
  }

  // Initial load
  void load(true)

  // Watch file for changes (Chokidar for cross-platform reliability)
  watcher = chokidar.watch(path, {
    persistent: false,
    ignoreInitial: true,
    awaitWriteFinish: { stabilityThreshold: FILE_STABILITY_MS },
  })
  watcher.on('add', () => void load(false))
  watcher.on('change', () => void load(false))
  watcher.on('unlink', () => { tasks = []; nextFireAt.clear() })

  // Start 1-second check timer
  checkTimer = setInterval(check, CHECK_INTERVAL_MS)
  checkTimer.unref?.() // Don't keep process alive
}
```

**Task Loading:**

```typescript
async function load(isInitial: boolean) {
  const loaded = await listAllCronTasks()
  const oldTasks = tasks
  tasks = loaded
  
  // Detect missed tasks (fired while Claude was closed)
  if (isInitial && onMissed) {
    const now = Date.now()
    const missed = loaded.filter(t => {
      const next = t.recurring
        ? jitteredNextCronRunMs(t.cron, t.lastFiredAt ?? t.createdAt, t.id, jitterCfg)
        : oneShotJitteredNextCronRunMs(t.cron, t.createdAt, t.id, jitterCfg)
      return next !== null && next < now
    })
    if (missed.length > 0) {
      onMissed(missed) // Surface to user
    }
  }

  // Reset nextFireAt for changed tasks
  for (const t of loaded) {
    const old = oldTasks.find(o => o.id === t.id)
    if (!old || old.cron !== t.cron || old.recurring !== t.recurring) {
      nextFireAt.delete(t.id) // Recompute on next check
    }
  }
}
```

---

### 5. CronCreate Tool (`CronCreateTool.ts`)

User-facing tool for creating cron tasks.

```typescript
export const CronCreateTool = buildTool({
  name: CRON_CREATE_TOOL_NAME,
  searchHint: 'schedule a recurring or one-shot prompt',
  shouldDefer: true,
  
  inputSchema: z.strictObject({
    cron: z.string().describe('5-field cron in local time'),
    prompt: z.string().describe('Prompt to enqueue'),
    recurring: semanticBoolean(z.boolean().optional()),
    durable: semanticBoolean(z.boolean().optional()),
  }),

  async validateInput(input): Promise<ValidationResult> {
    // Validate cron expression
    if (!parseCronExpression(input.cron)) {
      return { result: false, message: 'Invalid cron expression' }
    }
    // Validate cron has a future run
    if (nextCronRunMs(input.cron, Date.now()) === null) {
      return { result: false, message: 'No matching time in next year' }
    }
    // Limit total tasks
    const tasks = await listAllCronTasks()
    if (tasks.length >= MAX_JOBS) { // MAX_JOBS = 50
      return { result: false, message: 'Too many scheduled jobs (max 50)' }
    }
    // Prevent durable tasks in teammate sessions
    if (input.durable && getTeammateContext()) {
      return { result: false, message: 'Cannot create durable cron in teammate session' }
    }
    return { result: true }
  },

  async call({ cron, prompt, recurring = true, durable = false }) {
    const effectiveDurable = durable && isDurableCronEnabled()
    const teammate = getTeammateContext()
    
    const id = await addCronTask(
      cron,
      prompt,
      recurring,
      effectiveDurable,
      teammate?.agentId, // Route to subagent if teammate
    )
    
    // Enable scheduler — polls this flag
    setScheduledTasksEnabled(true)
    
    return {
      data: {
        id,
        humanSchedule: cronToHuman(cron), // "Every day at 9:00 AM"
        recurring,
        durable: effectiveDurable,
      },
    }
  },
})
```

**User Prompts (`prompt.ts`):**

```typescript
export function promptForCronCreate(): string {
  return `Schedule a recurring or one-shot prompt to run automatically.

Examples:
  cron="0 9 * * *" prompt="Check Linear for new issues" recurring=true
  cron="30 14 28 2 *" prompt="Run end-of-month report" recurring=false
  cron="*/15 * * * *" prompt="Poll for updates" recurring=true durable=false

Notes:
  - Cron is in YOUR local timezone
  - Jitter: :00/:30 times get 0-90s randomized offset to prevent thundering herd
  - Recurring tasks auto-expire after 7 days unless permanent=true (assistant mode)
  - durable=false tasks die when this session ends
`
}
```

---

### 6. React Hook (`useScheduledTasks.ts`)

Integrates scheduler with REPL.

```typescript
export function useScheduledTasks({
  isLoading,
  assistantMode = false,
  setMessages,
}: Props): void {
  const isLoadingRef = useRef(isLoading)
  isLoadingRef.current = isLoading

  useEffect(() => {
    if (!isKairosCronEnabled()) return

    // Enqueue for lead session
    const enqueueForLead = (prompt: string) =>
      enqueuePendingNotification({
        value: prompt,
        mode: 'prompt',
        priority: 'later',
        isMeta: true,
        workload: WORKLOAD_CRON, // Lower QoS attribution
      })

    const scheduler = createCronScheduler({
      // Fire with full task for teammate routing
      onFireTask: task => {
        if (task.agentId) {
          // Route to teammate subagent
          const teammate = findTeammateTaskByAgentId(
            task.agentId,
            store.getState().tasks,
          )
          if (teammate && !isTerminalTaskStatus(teammate.status)) {
            injectUserMessageToTeammate(teammate.id, task.prompt, setAppState)
            return
          }
          // Teammate gone — clean up orphan
          void removeCronTasks([task.id])
          return
        }
        // Lead session — show message and enqueue
        const msg = createScheduledTaskFireMessage(
          `Running scheduled task (${formatCronFireTime(new Date())})`,
        )
        setMessages(prev => [...prev, msg])
        enqueueForLead(task.prompt)
      },
      isLoading: () => isLoadingRef.current,
      assistantMode,
      getJitterConfig: getCronJitterConfig, // GrowthBook-backed
      isKilled: () => !isKairosCronEnabled(), // Runtime killswitch
    })
    
    scheduler.start()
    return () => scheduler.stop()
  }, [assistantMode])
}
```

---

## Task Type Matrix

| Dimension | Options | Description |
|-----------|---------|-------------|
| **Durability** | `durable=true` | Persist to `.claude/scheduled_tasks.json` |
| | `durable=false` | Session memory only (dies with process) |
| **Recurrence** | `recurring=true` | Reschedule after firing (until 7-day expiry) |
| | `recurring=false` | Delete after firing (one-shot) |
| **Routing** | `agentId=undefined` | Lead session (user-facing) |
| | `agentId=string` | Teammate subagent (in-process) |

**Common Combinations:**

| Use Case | durable | recurring | agentId | Notes |
|----------|---------|-----------|---------|-------|
| Personal reminder | true | false | - | One-shot, survives restart |
| Hourly poll | true | true | - | Recurring until 7-day expiry |
| Teammate task | false | true | abc123 | In-process subagent routing |
| Temporary timer | false | false | - | Dies with session |

---

## Lifecycle Flow

### 1. Task Creation

```
User → CronCreate Tool → validateInput()
                              │
                              ▼
                         parseCronExpression()
                              │
                              ▼
                         nextCronRunMs() (has future?)
                              │
                              ▼
                         addCronTask()
                              │
                              ▼
                         Write to .claude/scheduled_tasks.json
                              │
                              ▼
                         setScheduledTasksEnabled(true)
```

### 2. Scheduler Start

```
useScheduledTasks Hook → createCronScheduler()
                              │
                              ▼
                       scheduler.start() → enable()
                              │
                              ├──► tryAcquireSchedulerLock()
                              │       │
                              │       ├── Atomic O_EXCL create
                              │       │
                              │       └── Set isOwner = true/false
                              │
                              ├──► Load tasks (listAllCronTasks())
                              │
                              ├──► Detect missed tasks (onMissed callback)
                              │
                              └──► Start Chokidar watcher
                                      │
                                      └──► On file change → reload tasks
```

### 3. Check Loop (Every 1 Second)

```
check() ──┬──► isKilled()? (GrowthBook)
          │
          ├──► isLoading()? (Defer during bootstrap)
          │
          ├──► For each task:
          │     │
          │     ├──► Get nextFireAt from cache
          │     │       │
          │     │       └──► Undefined? Compute with jitter
          │     │               │
          │     │               ├──► recurring: jitteredNextCronRunMs()
          │     │               │
          │     │               └──► one-shot: oneShotJitteredNextCronRunMs()
          │     │
          │     ├──► now < next? Skip
          │     │
          │     └──► now >= next? FIRE!
          │             │
          │             ├──► onFireTask(task) callback
          │             │       │
          │             │       ├──► agentId set? Route to teammate
          │             │       │
          │             │       └──► agentId unset? Enqueue for lead
          │             │
          │             ├──► Recurring?
          │             │       │
          │             │       ├──► Recompute nextFireAt
          │             │       │
          │             │       └──► Batch lastFiredAt update
          │             │
          │             └──► One-shot?
          │                     │
          │                     ├──► Session: remove from memory
          │                     │
          │                     └──► Durable: delete from JSON
          │
          └──► Persist batch (markCronTasksFired)
```

### 4. Multi-Session Coordination

```
Session A (PID 12345)          Session B (PID 67890)
       │                              │
       ▼                              │
tryAcquireSchedulerLock()             │
       │                              │
       ├──► Atomic O_EXCL create      │
       │    Lock file: {sessionId: A, │
       │                pid: 12345}   │
       │    SUCCESS → isOwner = true  │
       │                              │
       ▼                              ▼
check() runs (isOwner=true)   tryAcquireSchedulerLock()
                                      │
                                      ├──► O_EXCL fails (exists)
                                      │
                                      ├──► Read existing lock
                                      │
                                      ├──► sessionId != B
                                      │
                                      ├──► isProcessRunning(12345)?
                                      │    │
                                      │    └──► YES → return false
                                      │
                                      └──► isOwner = false
                                           │
                                           └──► Start lock probe timer
                                                 │
                                                 └──► Every 5s: tryAcquire...
                                                       │
                                                       └──► If A dies, steal lock
```

---

## Jitter Deep Dive

### Why Jitter?

Without jitter, if 10,000 users schedule `0 9 * * *` (9 AM daily), all 10,000 tasks fire at exactly 9:00:00, causing:
- API rate limit spikes
- Database connection pool exhaustion
- Latency spikes for other users

**Jitter Solution:**
- Each task gets a deterministic per-task offset (0-15 min for recurring, 0-90 sec for one-shot)
- Offset is computed from task UUID — no shared randomness
- Same task always fires at the same offset (predictable for debugging)

### Tuning Knobs

| Parameter | Default | Effect |
|-----------|---------|--------|
| `recurringFrac` | 0.1 | 10% of interval added as delay |
| `recurringCapMs` | 15 min | Max delay for recurring |
| `oneShotMaxMs` | 90 sec | Max early pull for one-shot |
| `oneShotFloorMs` | 0 | Min early pull (raise to guarantee lead time) |
| `oneShotMinuteMod` | 30 | Jitter only :00 and :30 |

**Ops Emergency Tuning:**

```typescript
// GrowthBook flag: tengu_kairos_cron_config
{
  "recurringCapMs": 30 * 60 * 1000, // Increase to 30 min during incident
  "oneShotMinuteMod": 15            // Jitter :00, :15, :30, :45
}
```

---

## Related Files

**Module Documentation:**
- [bridge/exploration.md](../bridge/exploration.md) — REPL transport (how cron messages enqueue)
- [bootstrap/exploration.md](../bootstrap/exploration.md) — Session ID, state initialization
- [state/exploration.md](../state/exploration.md) — Zustand store (teammate routing)

**Related Modules:**
- `src/bootstrap/state.js` — Session ID, durable cron feature flag
- `src/tools/ScheduleCronTool/` — Full tool implementation
- `src/hooks/useScheduledTasks.ts` — React hook integration
- `src/utils/cron*.ts` — All cron utilities

---

## Integration Points

### With REPL Notification System

Cron tasks enqueue via `enqueuePendingNotification()`:

```typescript
enqueuePendingNotification({
  value: prompt,
  mode: 'prompt',
  priority: 'later',
  isMeta: true,
  workload: WORKLOAD_CRON, // Lower QoS than user prompts
})
```

### With Teammate System

Teammate tasks route to in-process subagents:

```typescript
if (task.agentId) {
  const teammate = findTeammateTaskByAgentId(task.agentId, store.getState().tasks)
  if (teammate) {
    injectUserMessageToTeammate(teammate.id, task.prompt, setAppState)
  }
}
```

### With GrowthBook

Feature flags control scheduler behavior:

| Flag | Type | Purpose |
|------|------|---------|
| `tengu_kairos_cron` | boolean | Killswitch (disable all cron) |
| `tengu_kairos_cron_config` | object | Jitter parameters |

---

*Deep dive created: 2026-04-07*
