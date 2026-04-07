# Screens Module — Deep-Dive Exploration

**Module:** `screens/`  
**Location:** `/home/darkvoid/Boxxed/@formulas/src.rust/src.llamacpp/src.ClaudOpen/claude-code-main/src/screens/`  
**Files:** 3 TSX files  
**Created:** 2026-04-07

---

## 1. Module Overview

The `screens/` module implements **full-screen terminal UI components** — the main REPL interface and supporting screens that comprise Claude Code's terminal user experience. These are Ink-based React components that render the interactive chat interface, task lists, permission dialogs, and various utility screens.

### Core Responsibilities

1. **REPL Interface** — Main chat screen (`REPL.tsx`):
   - Message rendering with virtual scrolling
   - Prompt input with vim/emacs keybindings
   - Permission request dialogs
   - Task list and teammate view
   - Cost tracking and budget display
   - Spinner and status indicators

2. **Doctor Screen** — Diagnostic tool (`Doctor.tsx`):
   - System health checks
   - Configuration validation
   - Network connectivity tests
   - Performance diagnostics

3. **Resume Conversation** — Session recovery (`ResumeConversation.tsx`):
   - Session restore UI
   - Conversation history preview
   - Merge/overwrite decisions

### Key Design Patterns

- **Ink/React Components**: Terminal UI using React fiber renderer
- **Virtual Scrolling**: MessageList with windowing for large conversations
- **State Management**: AppState via Zustand for centralized state
- **Keybinding Integration**: useKeybinding hooks for command handling
- **Streaming Rendering**: Incremental updates during model responses

---

## 2. File Inventory

| File | Lines | Description |
|------|-------|-------------|
| `REPL.tsx` | ~8500+ | Main REPL interface (largest component) |
| `Doctor.tsx` | ~1800+ | Diagnostic screen |
| `ResumeConversation.tsx` | ~1500+ | Session resume UI |

**Total:** ~11800+ lines across 3 files

---

## 3. Key Exports

### REPL Component (`REPL.tsx`)

```typescript
// Main REPL component (default export)
export default function REPL(): JSX.Element

// Internal components (not exported but key for understanding)
// - Messages: Virtualized message list
// - PromptInput: Text input with keybindings
// - PermissionRequest: Tool permission dialogs
// - TaskListV2: Task management view
// - TeammateViewHeader: Swarm teammate header
// - SpinnerWithVerb: Loading indicator
// - CostSummary: Token/cost display
// - CompanionSprite: Buddy mascot rendering
```

### Doctor Component (`Doctor.tsx`)

```typescript
// Doctor diagnostic screen
export default function Doctor(): JSX.Element

// Diagnostic checks
// - Config validation
// - Network connectivity
// - Model availability
// - MCP server health
// - Filesystem permissions
```

### Resume Conversation Component (`ResumeConversation.tsx`)

```typescript
// Session resume UI
export default function ResumeConversation(): JSX.Element

// Resume options
// - Restore session
// - Start new
// - Merge conversations
```

---

## 4. Line-by-Line Analysis

### 4.1 REPL Initialization (`REPL.tsx` lines 57-200+)

```typescript
export default function REPL(): React.JSX.Element {
  const $ = useInternals()
  
  // Session initialization
  useLayoutEffect(() => {
    const init = async () => {
      // Detect JetBrains IDE
      await initJetBrainsDetection()
      
      // Apply environment variables from config
      applyConfigEnvironmentVariables()
      
      // Initialize file history snapshot
      if (fileHistoryEnabled()) {
        dispatch({ type: 'initFileHistorySnapshot' })
      }
      
      // Process session start hooks
      await processSessionStartHooks(
        getMessages().map(m => convertToSDKMessage(m)),
      )
      
      // Log session start analytics
      logEvent('session_start', {...})
    }
    init()
  }, [])
  
  // ... rest of component
}
```

**Startup Sequence**: Detects IDE, applies env vars, initializes file history, runs hooks.

### 4.2 Message Submission (`REPL.tsx` lines ~4500+)

```typescript
async function handlePromptSubmit(
  userInput: string,
  imageUrls?: string[],
  selectedMessageIds?: string[],
  effort?: EffortValue,
): Promise<void> {
  // Create user message
  const userMessage = createUserMessage(userInput, imageUrls)
  
  // Add to messages
  dispatch({ type: 'ADD_MESSAGE', message: userMessage })
  
  // Clear input
  setInputValue('')
  
  // Start query loop
  await runQuery({
    messages: getMessages(),
    effort,
    selectedMessageIds,
  })
}
```

**Flow**: Create message → Add to state → Run query loop.

### 4.3 Permission Request Handling (`REPL.tsx` lines ~5000+)

```typescript
const handlePermissionResponse = useCallback(
  async (
    toolUseId: string,
    response: 'allow' | 'deny' | 'allowAlways' | 'denyAlways',
    updatedInput?: Record<string, unknown>,
  ): Promise<void> => {
    // Apply permission update
    const update = {
      toolUseId,
      response,
      updatedInput,
    }
    
    await applyPermissionUpdate(update)
    await persistPermissionUpdate(update)
    
    // Resume query loop
    resumeQuery()
  },
  [...],
)
```

**Permission Flow**: Apply update → Persist to settings → Resume query.

### 4.4 Token Budget Display (`REPL.tsx` lines ~3500+)

```typescript
const budgetDisplay = useMemo(() => {
  const budget = getCurrentTurnTokenBudget()
  const inputTokens = getTotalInputTokens()
  const outputTokens = getTurnOutputTokens()
  const totalUsed = inputTokens + outputTokens
  const pct = budget ? Math.round((totalUsed / budget) * 100) : 0
  
  if (!budget) return null
  
  return (
    <Box>
      <Text>
        Tokens: {formatTokens(totalUsed)} / {formatTokens(budget)} ({pct}%)
      </Text>
      {pct >= 90 && (
        <Text color="yellow">⚠️ Near budget limit</Text>
      )}
    </Box>
  )
}, [budget, inputTokens, outputTokens])
```

**Budget Warning**: Shows warning at 90%+ usage.

### 4.5 Vim Mode Input Handling (`REPL.tsx` lines ~4000+)

```typescript
const handleVimKey = useCallback(
  (input: string): boolean => {
    if (!vimEnabled || vimState.mode !== 'NORMAL') {
      return false
    }
    
    const result = transition(vimState.command, input, {
      cursor,
      text: inputValue,
      setText: setInputValue,
      setOffset: setCursorOffset,
      enterInsert: enterInsertMode,
      getRegister: () => vimRegister.content,
      setRegister: (content, linewise) => {
        vimRegister.content = content
        vimRegister.isLinewise = linewise
      },
      recordChange: (change) => {
        dispatch({ type: 'RECORD_VIM_CHANGE', change })
      },
    })
    
    if (result.execute) {
      result.execute()
      return true
    }
    
    if (result.next) {
      setVimState({ mode: 'NORMAL', command: result.next })
      return true
    }
    
    return false
  },
  [vimState, inputValue, ...],
)
```

**Vim Integration**: Uses transition table from `vim/transitions.ts`.

### 4.6 Background Session Detection (`REPL.tsx` lines ~2500+)

```typescript
const isBackground = useMemo(() => {
  return isBgSession(getSessionId())
}, [sessionId])

// Show background hint
{isBackground && (
  <SessionBackgroundHint
    sessionId={sessionId}
    onForeground={() => handleForegroundSession()}
  />
)}
```

**Background Sessions**: Visual indicator for backgrounded sessions.

### 4.7 Swarm/Teammate Integration (`REPL.tsx` lines ~6000+)

```typescript
// Render teammate view
{isInTeammateView && (
  <Box flexDirection="column">
    <TeammateViewHeader
      agentName={getAgentName()}
      teamName={getTeamName()}
      onExit={() => exitTeammateView()}
    />
    <Messages
      messages={teammateMessages}
      agentColor={agentColor}
    />
  </Box>
)}
```

**Teammate View**: Separate message list for swarm teammates.

### 4.8 Companion Sprite Rendering (`REPL.tsx` lines ~7500+)

```typescript
// Show companion sprite (buddy mascot)
{shouldShowCompanion && (
  <Box position="absolute" right={1} bottom={2}>
    <CompanionSprite
      bones={companionBones}
      soul={companionSoul}
      frame={companionFrame}
      floatingBubble={companionBubble}
    />
  </Box>
)}
```

**Buddy System**: ASCII mascot with floating thought bubbles.

---

## 5. Integration Points

### 5.1 With `ink/` Components

| Component | Integration |
|-----------|-------------|
| `REPL.tsx` | Uses `<Box>`, `<Text>`, `useInput`, `useStdin` |
| `Doctor.tsx` | Uses Ink primitives for layout |
| `ResumeConversation.tsx` | Uses Ink for dialog UI |

### 5.2 With `state/AppState.js`

| Component | Integration |
|-----------|-------------|
| `REPL.tsx` | Uses `useAppState()`, `useSetAppState()` |

### 5.3 With `keybindings/`

| Component | Integration |
|-----------|-------------|
| `REPL.tsx` | Uses `useKeybinding()`, `useGlobalKeybindings()` |

### 5.4 With `vim/`

| Component | Integration |
|-----------|-------------|
| `REPL.tsx` | Uses `transition()`, `VimState`, `PersistentState` |

### 5.4 With `buddy/`

| Component | Integration |
|-----------|-------------|
| `REPL.tsx` | Uses `CompanionSprite`, `getCompanion()` |

### 5.5 With `services/`

| Component | Integration |
|-----------|-------------|
| `REPL.tsx` | Uses MCP, OAuth, analytics, notifications |

---

## 6. Data Flow

### 6.1 Query Loop Flow

```
User submits prompt
    │
    ▼
handlePromptSubmit()
    │
    ├──► createUserMessage()
    ├──► dispatch({type: 'ADD_MESSAGE'})
    └──► runQuery()
         │
         ▼
         query() in query.js
         │
         ├──► Model API call
         ├──► Stream response
         ├──► Handle tool uses
         └──► Update messages
```

### 6.2 Permission Request Flow

```
Model requests tool use
    │
    ▼
Permission check
    │
    ├──► Auto-allowed? → Execute
    └──► Requires permission? → Show dialog
         │
         ▼
         <PermissionRequest />
         │
         ▼
         User responds
         │
         ├──► allow → Execute tool
         ├──► deny → Skip tool
         ├──► allowAlways → Update settings
         └──► denyAlways → Update settings
```

### 6.3 Vim Input Flow

```
User keypress in NORMAL mode
    │
    ▼
handleVimKey(input)
    │
    ├──► transition(state, input, ctx)
    │    ├──► fromOperator() → executeOperatorMotion()
    │    ├──► fromFind() → executeFind()
    │    └──► ...
    │
    ▼
Update cursor, text
    │
    ▼
recordChange() for dot-repeat
```

---

## 7. Key Patterns

### 7.1 Virtual Scrolling

```typescript
// MessageList uses windowed rendering
<Messages
  messages={messages}
  renderItem={(message, index) => (
    <MessageView key={message.id} message={message} />
  )}
  itemCount={messages.length}
  height={terminalHeight - reservedLines}
/>
```

**Why**: Support conversations with thousands of messages without rendering all.

### 7.2 State Colocation

```typescript
// Local state for input
const [inputValue, setInputValue] = useState('')

// Shared state for messages
const messages = useAppState(s => s.messages)
const dispatch = useSetAppState()

// Ephemeral state for permissions
const [pendingPermission, setPendingPermission] = useState<Permission | null>(null)
```

**Pattern**: State at appropriate level — local, shared, or ephemeral.

### 7.3 Streaming Updates

```typescript
// During model response
while streaming:
  - Update thinking block incrementally
  - Append tool use arguments as they arrive
  - Show partial assistant messages
```

**User Experience**: Responsive feedback during long operations.

---

## 8. Feature Gates

| Feature | Components Affected |
|---------|---------------------|
| `VOICE_MODE` | Voice mode UI, `/voice` command |
| `COORDINATOR_MODE` | Coordinator system prompt |
| `KAIROS` | Scheduled tasks UI |
| `AGENT_TRIGGERS` | Loop mode UI |
| `BG_SESSIONS` | Background session management |
| `PROACTIVE` | Proactive suggestions |

---

## 9. Summary

The `screens/` module provides **complete terminal UI**:

1. **REPL Interface** — Full-featured chat with streaming, permissions, tasks
2. **Doctor Screen** — System diagnostics and health checks
3. **Resume UI** — Session recovery and conversation merge

**Key Components**:
- Virtualized message rendering
- Permission request dialogs
- Task list and teammate view
- Token budget display
- Vim mode integration
- Companion sprite

**Key Design Decisions**:
- **Ink/React** for terminal UI
- **Virtual scrolling** for large conversations
- **AppState pattern** for centralized state
- **Streaming updates** for responsive UX

---

**Last Updated:** 2026-04-07  
**Status:** Complete — All 3 files analyzed
