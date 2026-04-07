# Claude Code State Module — Deep-Dive Exploration

**Module:** `src/state/`  
**Parent Project:** Claude Code CLI  
**Created:** 2026-04-07  
**Files:** 6 TypeScript/TSX files

---

## 1. Module Overview

The `state/` module implements the **centralized state management system** for Claude Code CLI using a custom Zustand-like store pattern. It provides the reactive state backbone that powers the CLI's UI, tool permissions, task tracking, MCP integration, and session management.

### Core Responsibilities

1. **State Store** — Centralized reactive state container:
   - `createStore()`: Lightweight Zustand-compatible store with `getState`, `setState`, `subscribe`
   - `AppStateProvider`: React context provider for state access
   - `useAppState()`: Selector-based subscription hook with `Object.is` comparison

2. **Application State** — Comprehensive state definition:
   - 50+ state fields covering settings, tasks, MCP, plugins, permissions, telemetry
   - `DeepImmutable` wrapper for type safety
   - Default state factory with proper initialization

3. **Selectors** — Derived state computation:
   - `getViewedTeammateTask()`: Current teammate view state
   - `getActiveAgentForInput()`: Input routing discriminant

4. **State Changes** — Side effects and persistence:
   - `onChangeAppState()`: Global change handler for persistence, notifications
   - `teammateViewHelpers.ts`: View transition helpers

### Key Design Patterns

- **Store Pattern**: Minimalist Zustand-compatible API with React integration
- **Selector Pattern**: Computed state derivation without side effects
- **Deep Immutability**: Type-level immutability enforcement
- **Change Subscription**: Fine-grained reactivity via selector-based subscriptions

---

## 2. File Inventory

| File | Lines | Key Exports | Description |
|------|-------|-------------|-------------|
| `store.ts` | ~35 | `createStore`, `Store<T>` | Core store primitive with getState/setState/subscribe |
| `AppStateStore.ts` | ~570 | `AppState`, `getDefaultAppState`, `AppStateStore` | Full state type definition with 50+ fields |
| `AppState.tsx` | ~200 | `AppStateProvider`, `useAppState`, `useSetAppState` | React integration hooks and context provider |
| `selectors.ts` | ~77 | `getViewedTeammateTask`, `getActiveAgentForInput` | Derived state selectors |
| `onChangeAppState.ts` | ~172 | `onChangeAppState`, `externalMetadataToAppState` | Global change handler with persistence |
| `teammateViewHelpers.ts` | ~142 | `enterTeammateView`, `exitTeammateView`, `stopOrDismissAgent` | Teammate view transition helpers |

**Total Lines:** ~1,196 lines

---

## 3. Key Exports

### 3.1 Store Primitive (`store.ts`)

```typescript
type Listener = () => void
type OnChange<T> = (args: { newState: T; oldState: T }) => void

export type Store<T> = {
  getState: () => T
  setState: (updater: (prev: T) => T) => void
  subscribe: (listener: Listener) => () => void
}

export function createStore<T>(
  initialState: T,
  onChange?: OnChange<T>,
): Store<T> {
  let state = initialState
  const listeners = new Set<Listener>()

  return {
    getState: () => state,

    setState: (updater: (prev: T) => T) => {
      const prev = state
      const next = updater(prev)
      if (Object.is(next, prev)) return  // No-op if identical
      state = next
      onChange?.({ newState: next, oldState: prev })
      for (const listener of listeners) listener()
    },

    subscribe: (listener: Listener) => {
      listeners.add(listener)
      return () => listeners.delete(listener)
    },
  }
}
```

**Design Notes:**
- `Object.is` comparison prevents unnecessary re-renders
- `onChange` callback invoked before listener notification
- Unsubscribe returned from `subscribe()` for cleanup

---

### 3.2 AppState Type (`AppStateStore.ts`)

```typescript
export type AppState = DeepImmutable<{
  // Settings and configuration
  settings: SettingsJson
  verbose: boolean
  mainLoopModel: ModelSetting
  mainLoopModelForSession: ModelSetting
  
  // UI state
  statusLineText: string | undefined
  expandedView: 'none' | 'tasks' | 'teammates'
  isBriefOnly: boolean
  selectedIPAgentIndex: number
  coordinatorTaskIndex: number
  viewSelectionMode: 'none' | 'selecting-agent' | 'viewing-agent'
  footerSelection: FooterItem | null
  
  // Permission and tool state
  toolPermissionContext: ToolPermissionContext
  denialTracking?: DenialTrackingState
  
  // Task management
  tasks: { [taskId: string]: TaskState }
  agentNameRegistry: Map<string, AgentId>
  foregroundedTaskId?: string
  viewingAgentTaskId?: string
  
  // MCP and plugins
  mcp: {
    clients: MCPServerConnection[]
    tools: Tool[]
    commands: Command[]
    resources: Record<string, ServerResource[]>
    pluginReconnectKey: number
  }
  plugins: {
    enabled: LoadedPlugin[]
    disabled: LoadedPlugin[]
    commands: Command[]
    errors: PluginError[]
    installationStatus: {...}
    needsRefresh: boolean
  }
  
  // Remote session state (CCR bridge)
  replBridgeEnabled: boolean
  replBridgeExplicit: boolean
  replBridgeConnected: boolean
  replBridgeSessionActive: boolean
  replBridgeReconnecting: boolean
  replBridgeSessionUrl: string | undefined
  replBridgeEnvironmentId: string | undefined
  replBridgeSessionId: string | undefined
  replBridgeError: string | undefined
  
  // Notifications and elicitation
  notifications: {
    current: Notification | null
    queue: Notification[]
  }
  elicitation: {
    queue: ElicitationRequestEvent[]
  }
  
  // Speculation (predictive execution)
  speculation: SpeculationState
  speculationSessionTimeSavedMs: number
  
  // File history and attribution
  fileHistory: FileHistoryState
  attribution: AttributionState
  
  // Todos (per-agent)
  todos: { [agentId: string]: TodoList }
  
  // Session hooks
  sessionHooks: SessionHooksState
  
  // Auth version for cache invalidation
  authVersion: number
  
  // Initial message processing
  initialMessage: {
    message: UserMessage
    clearContext?: boolean
    mode?: PermissionMode
    allowedPrompts?: AllowedPrompt[]
  } | null
  
  // Active overlays for Escape key coordination
  activeOverlays: ReadonlySet<string>
  
  // Fast mode and effort
  fastMode?: boolean
  effortValue?: EffortValue
  
  // Ultraplan mode state
  ultraplanLaunching?: boolean
  ultraplanSessionUrl?: string
  ultraplanPendingChoice?: {...}
  ultraplanLaunchPending?: {...}
  isUltraplanMode?: boolean
  
  // Permission callbacks
  replBridgePermissionCallbacks?: BridgePermissionCallbacks
  channelPermissionCallbacks?: ChannelPermissionCallbacks
}>
```

**Key State Categories:**

| Category | Fields | Purpose |
|----------|--------|---------|
| Settings | `settings`, `verbose`, `mainLoopModel` | User configuration |
| UI State | `expandedView`, `viewSelectionMode`, `footerSelection` | View management |
| Tasks | `tasks`, `agentNameRegistry`, `foregroundedTaskId` | Task orchestration |
| MCP/Plugins | `mcp`, `plugins` | Extension system |
| Remote Bridge | `replBridge*` fields | CCR session sync |
| Permissions | `toolPermissionContext`, `denialTracking` | Tool authorization |
| Speculation | `speculation`, `speculationSessionTimeSavedMs` | Predictive execution |
| Notifications | `notifications`, `elicitation` | User prompts |

---

### 3.3 React Hooks (`AppState.tsx`)

```typescript
// Context for store access
export const AppStoreContext = React.createContext<AppStateStore | null>(null)

/**
 * Subscribe to a slice of AppState. Only re-renders when the selected value
 * changes (compared via Object.is).
 */
export function useAppState<T>(selector: (state: AppState) => T): T {
  const store = useAppStore()
  
  const get = () => {
    const state = store.getState()
    const selected = selector(state)
    
    // Development-only check for selector returning whole state
    if ("external" === 'ant' && state === selected) {
      throw new Error(`Your selector returned the original state...`)
    }
    
    return selected
  }
  
  return useSyncExternalStore(store.subscribe, get, get)
}

/**
 * Get the setAppState updater without subscribing to any state.
 * Returns a stable reference that never changes.
 */
export function useSetAppState(): (
  updater: (prev: AppState) => AppState,
) => void {
  return useAppStore().setState
}

/**
 * Get the store directly (for passing getState/setState to non-React code).
 */
export function useAppStateStore(): AppStateStore {
  return useAppStore()
}

/**
 * Safe version that returns undefined if called outside AppStateProvider.
 */
export function useAppStateMaybeOutsideOfProvider<T>(
  selector: (state: AppState) => T,
): T | undefined {
  const store = useContext(AppStoreContext)
  return useSyncExternalStore(
    store ? store.subscribe : NOOP_SUBSCRIBE,
    () => store ? selector(store.getState()) : undefined,
  )
}
```

**Key Design Patterns:**

- **Selector-based subscription**: Components only re-render when their selected slice changes
- **Stable setter**: `useSetAppState()` never changes, safe for dependency arrays
- **SSR support**: `useSyncExternalStore` with server snapshot
- **Safe fallback**: `useAppStateMaybeOutsideOfProvider` for edge cases

---

### 3.4 Selectors (`selectors.ts`)

```typescript
export function getViewedTeammateTask(
  appState: Pick<AppState, 'viewingAgentTaskId' | 'tasks'>,
): InProcessTeammateTaskState | undefined {
  const { viewingAgentTaskId, tasks } = appState
  
  if (!viewingAgentTaskId) return undefined
  
  const task = tasks[viewingAgentTaskId]
  if (!task) return undefined
  
  if (!isInProcessTeammateTask(task)) return undefined
  
  return task
}

export type ActiveAgentForInput =
  | { type: 'leader' }
  | { type: 'viewed'; task: InProcessTeammateTaskState }
  | { type: 'named_agent'; task: LocalAgentTaskState }

export function getActiveAgentForInput(
  appState: AppState,
): ActiveAgentForInput {
  const viewedTask = getViewedTeammateTask(appState)
  if (viewedTask) {
    return { type: 'viewed', task: viewedTask }
  }
  
  const { viewingAgentTaskId, tasks } = appState
  if (viewingAgentTaskId) {
    const task = tasks[viewingAgentTaskId]
    if (task?.type === 'local_agent') {
      return { type: 'named_agent', task }
    }
  }
  
  return { type: 'leader' }
}
```

**Purpose:** Input routing discriminant for multi-agent sessions.

---

## 4. Line-by-Line Analysis

### 4.1 Default State Factory (`AppStateStore.ts` lines 456-569)

```typescript
export function getDefaultAppState(): AppState {
  const teammateUtils = require('../utils/teammate.js')
  const initialMode: PermissionMode =
    teammateUtils.isTeammate() && teammateUtils.isPlanModeRequired()
      ? 'plan'
      : 'default'

  return {
    settings: getInitialSettings(),
    tasks: {},
    agentNameRegistry: new Map(),
    verbose: false,
    mainLoopModel: null,
    mainLoopModelForSession: null,
    statusLineText: undefined,
    expandedView: 'none',
    isBriefOnly: false,
    showTeammateMessagePreview: false,
    selectedIPAgentIndex: -1,
    coordinatorTaskIndex: -1,
    viewSelectionMode: 'none',
    footerSelection: null,
    kairosEnabled: false,
    remoteSessionUrl: undefined,
    remoteConnectionStatus: 'connecting',
    remoteBackgroundTaskCount: 0,
    replBridgeEnabled: false,
    replBridgeExplicit: false,
    replBridgeOutboundOnly: false,
    replBridgeConnected: false,
    replBridgeSessionActive: false,
    replBridgeReconnecting: false,
    replBridgeConnectUrl: undefined,
    replBridgeSessionUrl: undefined,
    replBridgeEnvironmentId: undefined,
    replBridgeSessionId: undefined,
    replBridgeError: undefined,
    replBridgeInitialName: undefined,
    showRemoteCallout: false,
    toolPermissionContext: {
      ...getEmptyToolPermissionContext(),
      mode: initialMode,
    },
    // ... more fields
  }
}
```

**Key Observations:**

- **Lazy require** for `teammate.js` avoids circular dependency
- **Permission mode initialization** based on teammate context
- **Empty collections**: `{}`, `new Map()`, `[]` for collection fields
- **Null defaults** for optional/nullable fields
- **Boolean flags** default to `false`

---

### 4.2 Change Handler (`onChangeAppState.ts`)

```typescript
export function onChangeAppState({
  newState,
  oldState,
}: {
  newState: AppState
  oldState: AppState
}) {
  // toolPermissionContext.mode — single choke point for CCR/SDK mode sync
  const prevMode = oldState.toolPermissionContext.mode
  const newMode = newState.toolPermissionContext.mode
  
  if (prevMode !== newMode) {
    // Externalize internal mode names (bubble, ungated auto)
    const prevExternal = toExternalPermissionMode(prevMode)
    const newExternal = toExternalPermissionMode(newMode)
    
    if (prevExternal !== newExternal) {
      // Ultraplan = first plan cycle only
      const isUltraplan =
        newExternal === 'plan' &&
        newState.isUltraplanMode &&
        !oldState.isUltraplanMode
          ? true
          : null
      
      notifySessionMetadataChanged({
        permission_mode: newExternal,
        is_ultraplan_mode: isUltraplan,
      })
    }
    
    notifyPermissionModeChanged(newMode)
  }
  
  // mainLoopModel: remove from settings?
  if (
    newState.mainLoopModel !== oldState.mainLoopModel &&
    newState.mainLoopModel === null
  ) {
    updateSettingsForSource('userSettings', { model: undefined })
    setMainLoopModelOverride(null)
  }
  
  // mainLoopModel: add to settings?
  if (
    newState.mainLoopModel !== oldState.mainLoopModel &&
    newState.mainLoopModel !== null
  ) {
    updateSettingsForSource('userSettings', { model: newState.mainLoopModel })
    setMainLoopModelOverride(newState.mainLoopModel)
  }
  
  // expandedView → persist as showExpandedTodos + showSpinnerTree
  if (newState.expandedView !== oldState.expandedView) {
    const showExpandedTodos = newState.expandedView === 'tasks'
    const showSpinnerTree = newState.expandedView === 'teammates'
    
    if (
      getGlobalConfig().showExpandedTodos !== showExpandedTodos ||
      getGlobalConfig().showSpinnerTree !== showSpinnerTree
    ) {
      saveGlobalConfig(current => ({
        ...current,
        showExpandedTodos,
        showSpinnerTree,
      }))
    }
  }
  
  // verbose → persist to global config
  if (
    newState.verbose !== oldState.verbose &&
    getGlobalConfig().verbose !== newState.verbose
  ) {
    saveGlobalConfig(current => ({
      ...current,
      verbose: newState.verbose,
    }))
  }
  
  // tungstenPanelVisible (ant-only tmux panel sticky toggle)
  if (process.env.USER_TYPE === 'ant') {
    if (
      newState.tungstenPanelVisible !== oldState.tungstenPanelVisible &&
      newState.tungstenPanelVisible !== undefined &&
      getGlobalConfig().tungstenPanelVisible !== newState.tungstenPanelVisible
    ) {
      saveGlobalConfig(current => ({ ...current, tungstenPanelVisible }))
    }
  }
  
  // settings: clear auth-related caches when settings change
  if (newState.settings !== oldState.settings) {
    try {
      clearApiKeyHelperCache()
      clearAwsCredentialsCache()
      clearGcpCredentialsCache()
      
      // Re-apply environment variables when settings.env changes
      if (newState.settings.env !== oldState.settings.env) {
        applyConfigEnvironmentVariables()
      }
    } catch (error) {
      logError(toError(error))
    }
  }
}
```

**Side Effects Handled:**

| State Change | Side Effect |
|--------------|-------------|
| `toolPermissionContext.mode` | CCR metadata sync, SDK notification |
| `mainLoopModel` | Settings persistence, override registry |
| `expandedView` | Global config persistence |
| `verbose` | Global config persistence |
| `tungstenPanelVisible` | Global config (ant-only) |
| `settings` | Credential cache invalidation, env re-application |

**Design Insight:** Single choke point ensures all mode changes notify CCR consistently, regardless of mutation source (Shift+Tab, dialog, slash command, bridge).

---

### 4.3 Teammate View Helpers (`teammateViewHelpers.ts`)

```typescript
const PANEL_GRACE_MS = 30_000  // Inline to avoid cycle through BackgroundTasksDialog

function release(task: LocalAgentTaskState): LocalAgentTaskState {
  return {
    ...task,
    retain: false,
    messages: undefined,
    diskLoaded: false,
    evictAfter: isTerminalTaskStatus(task.status)
      ? Date.now() + PANEL_GRACE_MS
      : undefined,
  }
}

/**
 * Transitions the UI to view a teammate's transcript.
 * Sets viewingAgentTaskId and, for local_agent, retain: true.
 */
export function enterTeammateView(
  taskId: string,
  setAppState: (updater: (prev: AppState) => AppState) => void,
): void {
  logEvent('tengu_transcript_view_enter', {})
  
  setAppState(prev => {
    const task = prev.tasks[taskId]
    const prevId = prev.viewingAgentTaskId
    const prevTask = prevId !== undefined ? prev.tasks[prevId] : undefined
    const switching =
      prevId !== undefined &&
      prevId !== taskId &&
      isLocalAgent(prevTask) &&
      prevTask.retain
    const needsRetain =
      isLocalAgent(task) && (!task.retain || task.evictAfter !== undefined)
    const needsView =
      prev.viewingAgentTaskId !== taskId ||
      prev.viewSelectionMode !== 'viewing-agent'
    
    if (!needsRetain && !needsView && !switching) return prev
    
    let tasks = prev.tasks
    if (switching || needsRetain) {
      tasks = { ...prev.tasks }
      if (switching) tasks[prevId] = release(prevTask)
      if (needsRetain) {
        tasks[taskId] = { ...task, retain: true, evictAfter: undefined }
      }
    }
    
    return {
      ...prev,
      viewingAgentTaskId: taskId,
      viewSelectionMode: 'viewing-agent',
      tasks,
    }
  })
}

/**
 * Exit teammate transcript view and return to leader's view.
 */
export function exitTeammateView(
  setAppState: (updater: (prev: AppState) => AppState) => void,
): void {
  logEvent('tengu_transcript_view_exit', {})
  
  setAppState(prev => {
    const id = prev.viewingAgentTaskId
    const cleared = {
      ...prev,
      viewingAgentTaskId: undefined,
      viewSelectionMode: 'none' as const,
    }
    
    if (id === undefined) return prev.viewSelectionMode === 'none' ? prev : cleared
    
    const task = prev.tasks[id]
    if (!isLocalAgent(task) || !task.retain) return cleared
    
    return {
      ...cleared,
      tasks: { ...prev.tasks, [id]: release(task) },
    }
  })
}

/**
 * Context-sensitive x: running → abort, terminal → dismiss.
 */
export function stopOrDismissAgent(
  taskId: string,
  setAppState: (updater: (prev: AppState) => AppState) => void,
): void {
  setAppState(prev => {
    const task = prev.tasks[taskId]
    if (!isLocalAgent(task)) return prev
    
    if (task.status === 'running') {
      task.abortController?.abort()
      return prev
    }
    
    if (task.evictAfter === 0) return prev
    
    const viewingThis = prev.viewingAgentTaskId === taskId
    return {
      ...prev,
      tasks: {
        ...prev.tasks,
        [taskId]: { ...release(task), evictAfter: 0 },
      },
      ...(viewingThis && {
        viewingAgentTaskId: undefined,
        viewSelectionMode: 'none',
      }),
    }
  })
}
```

**Key Patterns:**

- **`retain` flag**: Blocks eviction, enables stream-append, triggers disk bootstrap
- **`evictAfter` timestamp**: Schedules eviction after grace period
- **Early return optimization**: Skip state update if no changes needed
- **Atomic updates**: All changes in single `setAppState` call

---

### 4.4 Speculation State (`AppStateStore.ts` lines 52-79)

```typescript
export type SpeculationState =
  | { status: 'idle' }
  | {
      status: 'active'
      id: string
      abort: () => void
      startTime: number
      messagesRef: { current: Message[] }  // Mutable ref - avoids array spreading per message
      writtenPathsRef: { current: Set<string> }  // Mutable ref - relative paths written to overlay
      boundary: CompletionBoundary | null
      suggestionLength: number
      toolUseCount: number
      isPipelined: boolean
      contextRef: { current: REPLHookContext }
      pipelinedSuggestion?: {
        text: string
        promptId: 'user_intent' | 'stated_intent'
        generationRequestId: string | null
      } | null
    }

export const IDLE_SPECULATION_STATE: SpeculationState = { status: 'idle' }
```

**Mutable Refs Optimization:**

```typescript
messagesRef: { current: Message[] }  // Avoids array spreading per message
writtenPathsRef: { current: Set<string> }  // Mutable set for tracking
```

**Why mutable refs?** Speculation updates messages incrementally. Using `ref.current` avoids creating new arrays on each append, reducing GC pressure during speculation active phase.

---

### 4.5 External Metadata Sync (`onChangeAppState.ts` lines 24-41)

```typescript
export function externalMetadataToAppState(
  metadata: SessionExternalMetadata,
): (prev: AppState) => AppState {
  return prev => ({
    ...prev,
    ...(typeof metadata.permission_mode === 'string' ? {
      toolPermissionContext: {
        ...prev.toolPermissionContext,
        mode: permissionModeFromString(metadata.permission_mode),
      },
    } : {}),
    ...(typeof metadata.is_ultraplan_mode === 'boolean' ? {
      isUltraplanMode: metadata.is_ultraplan_mode,
    } : {}),
  })
}
```

**Purpose:** Restore state from CCR external metadata on worker restart. Inverse of `onChangeAppState` sync.

---

## 5. Component Relationships

```
┌─────────────────────────────────────────────────────────────────┐
│                      AppState.tsx (React Layer)                  │
│  - AppStateProvider: Context provider with store initialization │
│  - useAppState(): Selector-based subscription hook              │
│  - useSetAppState(): Stable setter for dependency arrays        │
│  - useAppStateStore(): Direct store access for non-React code   │
└────────────────────────┬────────────────────────────────────────┘
                         │
                         ▼
┌─────────────────────────────────────────────────────────────────┐
│                   AppStateStore.ts (Type Definition)            │
│  - AppState: 50+ field type with DeepImmutable wrapper          │
│  - getDefaultAppState(): Factory with proper initialization     │
│  - CompletionBoundary, SpeculationState types                   │
└────────────────────────┬────────────────────────────────────────┘
                         │
                         ▼
┌─────────────────────────────────────────────────────────────────┐
│                      store.ts (Core Primitive)                   │
│  - createStore(): Zustand-compatible factory                    │
│  - Store<T> interface: getState, setState, subscribe            │
│  - Object.is comparison for no-op detection                     │
└────────────────────────┬────────────────────────────────────────┘
                         │
         ┌───────────────┴───────────────┐
         │                               │
         ▼                               ▼
┌─────────────────┐            ┌─────────────────┐
│ selectors.ts    │            │ onChangeAppState│
│ - Derived state │            │ - Persistence   │
│ - Input routing │            │ - CCR sync      │
└─────────────────┘            └─────────────────┘
         │                               │
         ▼                               ▼
┌─────────────────────────────────────────────────────────────────┐
│                 teammateViewHelpers.ts (View Transitions)       │
│  - enterTeammateView(): Set retain, clear evictAfter            │
│  - exitTeammateView(): Release task, schedule eviction          │
│  - stopOrDismissAgent(): Abort running or dismiss terminal      │
└─────────────────────────────────────────────────────────────────┘
```

---

## 6. Data Flow

### 6.1 State Update Flow

```
Component calls setAppState(updater)
   │
   ▼
Store.setState() → prev = state
   │
   ▼
next = updater(prev)
   │
   ▼
Object.is(next, prev) check
   │
   ├─ identical → return (no-op)
   │
   └─ different → state = next
       │
       ├─ onChangeAppState({ newState, oldState })
       │   ├─ Mode change → CCR metadata sync
       │   ├─ Model change → Settings persistence
       │   ├─ View change → Global config
       │   └─ Settings change → Cache invalidation
       │
       └─ Listeners notified
           │
           └─ React re-renders (selected slices only)
```

### 6.2 Selector Subscription Flow

```
Component: useAppState(s => s.verbose)
   │
   └─ useSyncExternalStore(store.subscribe, get, get)
       │
       ├─ subscribe: Adds listener to Set
       │
       └─ get: Returns store.getState().verbose
           │
           └─ On change: Object.is comparison
               ├─ same → no re-render
               └─ different → re-render
```

### 6.3 Teammate View Transition Flow

```
User clicks teammate row
   │
   ▼
enterTeammateView(taskId, setAppState)
   │
   ├─ logEvent('tengu_transcript_view_enter')
   │
   └─ setAppState(prev => {
       ├─ If switching: release(prevTask)
       ├─ Set task.retain = true, evictAfter = undefined
       ├─ viewingAgentTaskId = taskId
       └─ viewSelectionMode = 'viewing-agent'
     })
   │
   ▼
Task now retained (no eviction, stream-append enabled)
```

---

## 7. Key Patterns

### 7.1 Deep Immutability

```typescript
export type AppState = DeepImmutable<{
  settings: SettingsJson
  // ...
}>
```

**Purpose:** Type-level enforcement that state is never mutated directly. All changes via `setState(updater)`.

### 7.2 Mutable Refs for Performance

```typescript
messagesRef: { current: Message[] }
writtenPathsRef: { current: Set<string> }
contextRef: { current: REPLHookContext }
```

**Rationale:** Speculation updates incrementally. Mutable refs avoid array/set recreation on each update.

### 7.3 Selector-Based Subscriptions

```typescript
const verbose = useAppState(s => s.verbose)
const model = useAppState(s => s.mainLoopModel)
```

**Benefit:** Components only re-render when their selected slice changes, not on every state update.

### 7.4 Grace Period Eviction

```typescript
evictAfter: isTerminalTaskStatus(task.status)
  ? Date.now() + PANEL_GRACE_MS  // 30 seconds
  : undefined
```

**UX Rationale:** Terminal tasks linger briefly after completion so users can see the result before the row disappears.

---

## 8. Integration Points

### 8.1 With CCR Bridge

| State Field | Bridge Integration | Sync Direction |
|-------------|-------------------|----------------|
| `toolPermissionContext.mode` | `notifySessionMetadataChanged()` | Bidirectional |
| `isUltraplanMode` | `external_metadata.is_ultraplan_mode` | Bidirectional |
| `replBridge*` fields | Bridge connection state | Bridge → State |
| `replBridgePermissionCallbacks` | Bridge permission checks | State → Bridge |

### 8.2 With Settings System

| State Field | Settings Integration |
|-------------|---------------------|
| `settings` | Mirrors `getInitialSettings()` |
| `mainLoopModel` | Persists to `userSettings.model` |
| `verbose` | Persists to global config |
| `expandedView` | Persists as `showExpandedTodos` + `showSpinnerTree` |

### 8.3 With Task System

| State Field | Task Integration |
|-------------|-----------------|
| `tasks` | TaskState registry by taskId |
| `viewingAgentTaskId` | Current viewed task |
| `foregroundedTaskId` | Main view task |
| `agentNameRegistry` | Name → AgentId routing |

### 8.4 With MCP System

| State Field | MCP Integration |
|-------------|----------------|
| `mcp.clients` | Server connections |
| `mcp.tools` | Available tools |
| `mcp.pluginReconnectKey` | Plugin reload trigger |

---

## 9. Testing Considerations

### 9.1 Store Unit Tests

```typescript
// Test: setState with identical value is no-op
const store = createStore({ count: 0 })
let changeCount = 0
store.subscribe(() => changeCount++)

store.setState(prev => ({ count: prev.count }))  // No change
assert.strictEqual(changeCount, 0)

store.setState(prev => ({ count: prev.count + 1 }))  // Change
assert.strictEqual(changeCount, 1)
```

### 9.2 Selector Tests

```typescript
// Test: getViewedTeammateTask returns undefined when not viewing
const state = {
  viewingAgentTaskId: undefined,
  tasks: {},
}
assert.strictEqual(getViewedTeammateTask(state), undefined)

// Test: returns task when viewing in-process teammate
const state = {
  viewingAgentTaskId: 'task_123',
  tasks: {
    task_123: { type: 'in_process_teammate', ... },
  },
}
assert.ok(getViewedTeammateTask(state))
```

### 9.3 onChangeAppState Tests

```typescript
// Test: mode change triggers CCR notification
const oldState = getDefaultAppState()
const newState = {
  ...oldState,
  toolPermissionContext: {
    ...oldState.toolPermissionContext,
    mode: 'plan' as const,
  },
}

const notifySpy = spy()
// Mock notifySessionMetadataChanged
onChangeAppState({ newState, oldState })

assert.ok(notifySpy.calledWith({
  permission_mode: 'plan',
  is_ultraplan_mode: null,
}))
```

---

## 10. Environment Variables

| Variable | Purpose | Default |
|----------|---------|---------|
| `USER_TYPE` | ant-only features (tungsten, verbose persist) | `undefined` |
| `CLAUDE_CODE_OVERRIDE_DATE` | Date override for testing | `undefined` |

---

## 11. Summary

The `state/` module provides Claude Code's **reactive state backbone** with:

1. **Minimalist Store** — Zustand-compatible API with `getState`, `setState`, `subscribe`
2. **Comprehensive State** — 50+ fields covering settings, tasks, MCP, plugins, permissions, remote bridge
3. **React Integration** — Context provider, selector hooks, stable setters
4. **Derived State** — Selectors for teammate view, input routing
5. **Change Handling** — Global change handler for persistence and CCR sync
6. **View Transitions** — Helpers for teammate view enter/exit with retention management

The module balances **simplicity** (minimal store primitive) with **expressiveness** (rich state type, selectors, change handlers) while maintaining **type safety** (DeepImmutable wrapper, discriminated unions).

---

**Last Updated:** 2026-04-07  
**Status:** Complete — all 6 files inventoried and analyzed
