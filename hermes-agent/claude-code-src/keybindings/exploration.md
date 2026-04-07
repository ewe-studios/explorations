# Keybindings Module — Deep-Dive Exploration

**Module:** `keybindings/`  
**Location:** `/home/darkvoid/Boxxed/@formulas/src.rust/src.llamacpp/src.ClaudOpen/claude-code-main/src/keybindings/`  
**Files:** 14 TypeScript/TSX files  
**Created:** 2026-04-07

---

## 1. Module Overview

The `keybindings/` module implements Claude Code's **customizable keyboard shortcut system** — a comprehensive infrastructure for defining, parsing, validating, and resolving keyboard shortcuts across multiple UI contexts. This enables users to customize keybindings via `~/.claude/keybindings.json` while providing React hooks for handling shortcuts in components.

### Core Responsibilities

1. **Schema Definition** — Zod schemas for keybindings.json validation:
   - 18 UI contexts (Global, Chat, Autocomplete, Confirmation, etc.)
   - 70+ action identifiers (app:interrupt, chat:submit, confirm:yes, etc.)
   - Command bindings (command:help, command:commit, etc.)

2. **Keystroke Parsing** — Text-to-keystroke conversion:
   - Modifier aliases (ctrl/control, alt/opt/option/meta, cmd/command/super)
   - Special key names (escape, enter, arrows)
   - Chord sequences ("ctrl+k ctrl+s")

3. **Key Matching** — Ink input to binding resolution:
   - Modifier matching (ctrl, shift, alt/meta, super)
   - Context-aware resolution (Chat bindings override Global)
   - Chord state machine (pending chord tracking)

4. **React Integration** — Hooks for handling shortcuts:
   - `useKeybinding()` — Single action handler
   - `useKeybindings()` — Multiple actions
   - `useRegisterKeybindingContext()` — Context activation
   - `useShortcutDisplay()` — Display text lookup

5. **User Configuration** — Loading and validation:
   - Hot-reload via chokidar file watching
   - Duplicate detection
   - Reserved shortcut warnings (OS/terminal conflicts)
   - Merge strategy (user overrides default)

6. **Platform Handling** — Cross-platform compatibility:
   - macOS reserved shortcuts (cmd+c, cmd+space, etc.)
   - Terminal VT mode detection (Windows Terminal quirks)
   - Platform-specific display strings (opt vs alt)

### Key Design Patterns

- **Context Priority**: More specific contexts (Chat) override Global
- **Last Binding Wins**: User bindings come after defaults for override semantics
- **Chord State Machine**: Track pending chords across key events
- **Ref-Based State**: Pending chord ref for immediate access (avoids React render delay)
- **Handler Registry**: Centralized handler invocation via ChordInterceptor
- **Feature Gating**: Customization gated to Anthropic employees (configurable)

---

## 2. File Inventory

| File | Lines | Description |
|------|-------|-------------|
| `KeybindingContext.tsx` | ~242 | React context provider, handler registry |
| `KeybindingProviderSetup.tsx` | — | Provider initialization wrapper |
| `defaultBindings.ts` | ~340 | Default keybindings for all contexts |
| `loadUserBindings.ts` | ~473 | User config loading with hot-reload |
| `match.ts` | ~120 | Ink Key to ParsedKeystroke matching |
| `parser.ts` | ~204 | Keystroke/chord parsing and formatting |
| `reservedShortcuts.ts` | ~128 | OS/terminal reserved shortcuts |
| `resolver.ts` | ~245 | Key resolution with chord state |
| `schema.ts` | ~237 | Zod schema for keybindings.json |
| `shortcutFormat.ts` | ~64 | Non-React shortcut display lookup |
| `template.ts` | ~53 | Template generator for keybindings.json |
| `useKeybinding.ts` | ~197 | React hooks for handling shortcuts |
| `useShortcutDisplay.ts` | ~60 | Hook for display text lookup |
| `validate.ts` | ~499 | Validation logic and warnings |

**Total:** ~2,862 lines across 14 files

---

## 3. Key Exports

### Schema Types (`schema.ts`)

```typescript
// 18 UI contexts where keybindings apply
export const KEYBINDING_CONTEXTS = [
  'Global', 'Chat', 'Autocomplete', 'Confirmation', 'Help',
  'Transcript', 'HistorySearch', 'Task', 'ThemePicker', 'Settings',
  'Tabs', 'Attachments', 'Footer', 'MessageSelector', 'DiffDialog',
  'ModelPicker', 'Select', 'Plugin',
] as const

// 70+ action identifiers
export const KEYBINDING_ACTIONS = [
  // App-level
  'app:interrupt', 'app:exit', 'app:toggleTodos', 'app:toggleTranscript',
  'app:toggleBrief', 'app:toggleTeammatePreview', 'app:toggleTerminal',
  'app:redraw', 'app:globalSearch', 'app:quickOpen',
  // History
  'history:search', 'history:previous', 'history:next',
  // Chat
  'chat:cancel', 'chat:killAgents', 'chat:cycleMode', 'chat:modelPicker',
  'chat:fastMode', 'chat:thinkingToggle', 'chat:submit', 'chat:newline',
  'chat:undo', 'chat:externalEditor', 'chat:stash', 'chat:imagePaste',
  'chat:messageActions',
  // Autocomplete
  'autocomplete:accept', 'autocomplete:dismiss', 'autocomplete:previous',
  'autocomplete:next',
  // Confirmation
  'confirm:yes', 'confirm:no', 'confirm:previous', 'confirm:next',
  'confirm:nextField', 'confirm:previousField', 'confirm:cycleMode',
  'confirm:toggle', 'confirm:toggleExplanation',
  // ... and 40+ more
] as const

// Schema for a single keybinding block
export const KeybindingBlockSchema = z.object({
  context: z.enum(KEYBINDING_CONTEXTS),
  bindings: z.record(
    z.string(),  // Keystroke pattern
    z.union([
      z.enum(KEYBINDING_ACTIONS),
      z.string().regex(/^command:[a-zA-Z0-9:\-_]+$/),  // Command binding
      z.null(),  // Unbind default
    ])
  ),
})

// Full keybindings.json schema
export const KeybindingsSchema = z.object({
  $schema: z.string().optional(),
  $docs: z.string().optional(),
  bindings: z.array(KeybindingBlockSchema),
})
```

### Parser Functions (`parser.ts`)

```typescript
// Parse keystroke string to ParsedKeystroke
export function parseKeystroke(input: string): ParsedKeystroke

// Parse chord string to Chord (array of ParsedKeystrokes)
export function parseChord(input: string): Chord

// Convert ParsedKeystroke to display string
export function keystrokeToString(ks: ParsedKeystroke): string

// Convert Chord to display string
export function chordToString(chord: Chord): string

// Platform-specific display (opt vs alt on macOS)
export function keystrokeToDisplayString(
  ks: ParsedKeystroke,
  platform: DisplayPlatform = 'linux',
): string

// Parse keybinding blocks to flat list
export function parseBindings(blocks: KeybindingBlock[]): ParsedBinding[]
```

### Resolver Functions (`resolver.ts`)

```typescript
// Resolve key to action (no chord support)
export function resolveKey(
  input: string,
  key: Key,
  activeContexts: KeybindingContextName[],
  bindings: ParsedBinding[],
): ResolveResult

// Resolve key with chord state support
export function resolveKeyWithChordState(
  input: string,
  key: Key,
  activeContexts: KeybindingContextName[],
  bindings: ParsedBinding[],
  pending: ParsedKeystroke[] | null,
): ChordResolveResult

// Get display text for action
export function getBindingDisplayText(
  action: string,
  context: KeybindingContextName,
  bindings: ParsedBinding[],
): string | undefined
```

### React Hooks (`useKeybinding.ts`, `KeybindingContext.tsx`)

```typescript
// Handle single keybinding
export function useKeybinding(
  action: string,
  handler: () => void | false | Promise<void>,
  options: { context?: KeybindingContextName; isActive?: boolean } = {},
): void

// Handle multiple keybindings
export function useKeybindings(
  handlers: Record<string, () => void | false | Promise<void>>,
  options: { context?: KeybindingContextName; isActive?: boolean } = {},
): void

// Register context as active
export function useRegisterKeybindingContext(
  context: KeybindingContextName,
  isActive: boolean = true,
): void

// Get shortcut display text (with fallback)
export function useShortcutDisplay(
  action: string,
  context: KeybindingContextName,
  fallback: string,
): string

// Non-React shortcut lookup
export function getShortcutDisplay(
  action: string,
  context: KeybindingContextName,
  fallback: string,
): string
```

### Context Value (`KeybindingContext.tsx`)

```typescript
type KeybindingContextValue = {
  // Resolve key to action
  resolve: (
    input: string,
    key: Key,
    activeContexts: KeybindingContextName[],
  ) => ChordResolveResult

  // Update pending chord state
  setPendingChord: (pending: ParsedKeystroke[] | null) => void

  // Get display text for action
  getDisplayText: (
    action: string,
    context: KeybindingContextName,
  ) => string | undefined

  // All parsed bindings (for help display)
  bindings: ParsedBinding[]

  // Current pending chord
  pendingChord: ParsedKeystroke[] | null

  // Active contexts set
  activeContexts: Set<KeybindingContextName>

  // Register/unregister context
  registerActiveContext: (context: KeybindingContextName) => void
  unregisterActiveContext: (context: KeybindingContextName) => void

  // Handler registry
  registerHandler: (registration: HandlerRegistration) => () => void
  invokeAction: (action: string) => boolean
}
```

### Validation (`validate.ts`)

```typescript
// Validate user config
export function validateBindings(
  userBlocks: unknown,
  parsedBindings: ParsedBinding[],
): KeybindingWarning[]

// Check for duplicate keys in JSON string
export function checkDuplicateKeysInJson(
  jsonString: string,
): KeybindingWarning[]

// Check for duplicates within contexts
export function checkDuplicates(
  blocks: KeybindingBlock[],
): KeybindingWarning[]

// Check for reserved shortcuts
export function checkReservedShortcuts(
  bindings: ParsedBinding[],
): KeybindingWarning[]

// Format warnings for display
export function formatWarning(warning: KeybindingWarning): string
export function formatWarnings(warnings: KeybindingWarning[]): string
```

### User Binding Loader (`loadUserBindings.ts`)

```typescript
// Check if customization is enabled
export function isKeybindingCustomizationEnabled(): boolean

// Load keybindings (async)
export async function loadKeybindings(): Promise<KeybindingsLoadResult>

// Load keybindings (sync, cached)
export function loadKeybindingsSync(): ParsedBinding[]
export function loadKeybindingsSyncWithWarnings(): KeybindingsLoadResult

// Initialize file watcher
export async function initializeKeybindingWatcher(): Promise<void>

// Cleanup watcher
export function disposeKeybindingWatcher(): void

// Subscribe to changes
export const subscribeToKeybindingChanges: (
  listener: (result: KeybindingsLoadResult) => void,
) => () => void

// Get cached warnings
export function getCachedKeybindingWarnings(): KeybindingWarning[]
```

---

## 4. Line-by-Line Analysis

### 4.1 Schema Definition (`schema.ts`)

**Context Descriptions (lines 36-59):**

```typescript
export const KEYBINDING_CONTEXT_DESCRIPTIONS: Record<
  (typeof KEYBINDING_CONTEXTS)[number],
  string
> = {
  Global: 'Active everywhere, regardless of focus',
  Chat: 'When the chat input is focused',
  Autocomplete: 'When autocomplete menu is visible',
  Confirmation: 'When a confirmation/permission dialog is shown',
  Help: 'When the help overlay is open',
  Transcript: 'When viewing the transcript',
  HistorySearch: 'When searching command history (ctrl+r)',
  Task: 'When a task/agent is running in the foreground',
  ThemePicker: 'When the theme picker is open',
  Settings: 'When the settings menu is open',
  Tabs: 'When tab navigation is active',
  Attachments: 'When navigating image attachments in a select dialog',
  Footer: 'When footer indicators are focused',
  MessageSelector: 'When the message selector (rewind) is open',
  DiffDialog: 'When the diff dialog is open',
  ModelPicker: 'When the model picker is open',
  Select: 'When a select/list component is focused',
  Plugin: 'When the plugin dialog is open',
}
```

**Command Binding Regex (lines 193-198):**

```typescript
z.string()
  .regex(/^command:[a-zA-Z0-9:\-_]+$/)
  .describe(
    'Command binding (e.g., "command:help", "command:compact"). Executes the slash command as if typed.',
  )
```

**Null Unbinding (line 199):**

```typescript
z.null().describe('Set to null to unbind a default shortcut')
```

### 4.2 Default Bindings (`defaultBindings.ts`)

**Platform-Specific Shortcuts (lines 14-30):**

```typescript
// Image paste: alt+v on Windows (ctrl+v is system paste)
const IMAGE_PASTE_KEY = getPlatform() === 'windows' ? 'alt+v' : 'ctrl+v'

// Mode cycle: shift+tab requires VT mode on Windows
const SUPPORTS_TERMINAL_VT_MODE =
  getPlatform() !== 'windows' ||
  (isRunningWithBun()
    ? satisfies(process.versions.bun, '>=1.2.23')
    : satisfies(process.versions.node, '>=22.17.0 <23.0.0 || >=24.2.0'))

const MODE_CYCLE_KEY = SUPPORTS_TERMINAL_VT_MODE ? 'shift+tab' : 'meta+m'
```

**VT Mode Detection**: Windows Terminal without VT mode can't capture shift+tab. Node 22.17.0+/24.2.0+ and Bun 1.2.23+ enabled VT mode.

**Feature-Gated Bindings (lines 45-59):**

```typescript
{
  ...(feature('KAIROS') || feature('KAIROS_BRIEF')
    ? { 'ctrl+shift+b': 'app:toggleBrief' as const }
    : {}),
  ...(feature('QUICK_SEARCH')
    ? {
        'ctrl+shift+f': 'app:globalSearch' as const,
        'cmd+shift+f': 'app:globalSearch' as const,
        'ctrl+shift+p': 'app:quickOpen' as const,
        'cmd+shift+p': 'app:quickOpen' as const,
      }
    : {}),
  ...(feature('TERMINAL_PANEL') ? { 'meta+j': 'app:toggleTerminal' } : {}),
}
```

**Reserved Shortcuts Comment (lines 36-39):**

```typescript
// ctrl+c and ctrl+d use special time-based double-press handling.
// They ARE defined here so the resolver can find them, but they
// CANNOT be rebound by users - validation in reservedShortcuts.ts
// will show an error if users try to override these keys.
```

**Voice Push-to-Talk (lines 91-96):**

```typescript
// Voice activation (hold-to-talk). Registered so getShortcutDisplay
// finds it without hitting the fallback analytics log. To rebind,
// add a voice:pushToTalk entry (last wins); to disable, use /voice
// — null-unbinding space hits a pre-existing useKeybinding.ts trap
// where 'unbound' swallows the event (space dead for typing).
...(feature('VOICE_MODE') ? { space: 'voice:pushToTalk' } : {}),
```

**Critical Note**: Null-unbinding space kills typing — space becomes dead key.

**MessageActions Context (lines 268-295):**

```typescript
...(feature('MESSAGE_ACTIONS')
  ? [
      {
        context: 'MessageActions' as const,
        bindings: {
          up: 'messageActions:prev' as const,
          down: 'messageActions:next' as const,
          k: 'messageActions:prev' as const,
          j: 'messageActions:next' as const,
          'meta+up': 'messageActions:top' as const,
          'meta+down': 'messageActions:bottom' as const,
          'super+up': 'messageActions:top' as const,
          'super+down': 'messageActions:bottom' as const,
          'shift+up': 'messageActions:prevUser' as const,
          'shift+down': 'messageActions:nextUser' as const,
          escape: 'messageActions:escape' as const,
          'ctrl+c': 'messageActions:ctrlc' as const,
          enter: 'messageActions:enter' as const,
          c: 'messageActions:c' as const,
          p: 'messageActions:p' as const,
        },
      },
    ]
  : []),
```

**Dynamic Context**: MessageActions context only exists when MESSAGE_ACTIONS feature is enabled.

### 4.3 Keystroke Parsing (`parser.ts`)

**Modifier Aliases (lines 23-46):**

```typescript
for (const part of parts) {
  const lower = part.toLowerCase()
  switch (lower) {
    case 'ctrl':
    case 'control':
      keystroke.ctrl = true
      break
    case 'alt':
    case 'opt':
    case 'option':
      keystroke.alt = true
      break
    case 'shift':
      keystroke.shift = true
      break
    case 'meta':
      keystroke.meta = true
      break
    case 'cmd':
    case 'command':
    case 'super':
    case 'win':
      keystroke.super = true
      break
    // ... special keys
  }
}
```

**Chord Parsing (lines 80-84):**

```typescript
export function parseChord(input: string): Chord {
  // A lone space character IS the space key binding, not a separator
  if (input === ' ') return [parseKeystroke('space')]
  return input.trim().split(/\s+/).map(parseKeystroke)
}
```

**Edge Case**: Single space = space key, not empty chord.

**Display String Conversion (lines 157-176):**

```typescript
export function keystrokeToDisplayString(
  ks: ParsedKeystroke,
  platform: DisplayPlatform = 'linux',
): string {
  const parts: string[] = []
  if (ks.ctrl) parts.push('ctrl')
  // Alt/meta are equivalent in terminals, show platform-appropriate name
  if (ks.alt || ks.meta) {
    // Only macOS uses "opt", all other platforms use "alt"
    parts.push(platform === 'macos' ? 'opt' : 'alt')
  }
  if (ks.shift) parts.push('shift')
  if (ks.super) {
    parts.push(platform === 'macos' ? 'cmd' : 'super')
  }
  const displayKey = keyToDisplayName(ks.key)
  parts.push(displayKey)
  return parts.join('+')
}
```

**Platform Display**: macOS shows "opt", others show "alt"; macOS shows "cmd", others show "super".

### 4.4 Key Matching (`match.ts`)

**Key Name Extraction (lines 29-47):**

```typescript
export function getKeyName(input: string, key: Key): string | null {
  if (key.escape) return 'escape'
  if (key.return) return 'enter'
  if (key.tab) return 'tab'
  if (key.backspace) return 'backspace'
  if (key.delete) return 'delete'
  if (key.upArrow) return 'up'
  if (key.downArrow) return 'down'
  if (key.leftArrow) return 'left'
  if (key.rightArrow) return 'right'
  if (key.pageUp) return 'pageup'
  if (key.pageDown) return 'pagedown'
  if (key.wheelUp) return 'wheelup'
  if (key.wheelDown) return 'wheeldown'
  if (key.home) return 'home'
  if (key.end) return 'end'
  if (input.length === 1) return input.toLowerCase()
  return null
}
```

**Modifier Matching (lines 60-79):**

```typescript
function modifiersMatch(
  inkMods: InkModifiers,
  target: ParsedKeystroke,
): boolean {
  if (inkMods.ctrl !== target.ctrl) return false
  if (inkMods.shift !== target.shift) return false

  // Alt and meta both map to key.meta in Ink (terminal limitation)
  const targetNeedsMeta = target.alt || target.meta
  if (inkMods.meta !== targetNeedsMeta) return false

  // Super (cmd/win) is distinct from alt/meta
  if (inkMods.super !== target.super) return false

  return true
}
```

**Alt/Meta Collapse**: Terminals can't distinguish alt vs meta — both arrive as `key.meta`.

**Escape Key Quirk (lines 96-102):**

```typescript
// QUIRK: Ink sets key.meta=true when escape is pressed.
// We need to ignore the meta modifier when matching escape key itself.
if (key.escape) {
  return modifiersMatch({ ...inkMods, meta: false }, target)
}
```

### 4.5 Chord Resolution (`resolver.ts`)

**Chord State Resolution (lines 166-244):**

```typescript
export function resolveKeyWithChordState(
  input: string,
  key: Key,
  activeContexts: KeybindingContextName[],
  bindings: ParsedBinding[],
  pending: ParsedKeystroke[] | null,
): ChordResolveResult {
  // Cancel chord on escape
  if (key.escape && pending !== null) {
    return { type: 'chord_cancelled' }
  }

  // Build current keystroke
  const currentKeystroke = buildKeystroke(input, key)
  if (!currentKeystroke) {
    if (pending !== null) {
      return { type: 'chord_cancelled' }
    }
    return { type: 'none' }
  }

  // Build test chord
  const testChord = pending
    ? [...pending, currentKeystroke]
    : [currentKeystroke]

  // Filter by active contexts
  const ctxSet = new Set(activeContexts)
  const contextBindings = bindings.filter(b => ctxSet.has(b.context))

  // Check for longer chord prefixes
  const chordWinners = new Map<string, string | null>()
  for (const binding of contextBindings) {
    if (
      binding.chord.length > testChord.length &&
      chordPrefixMatches(testChord, binding)
    ) {
      chordWinners.set(chordToString(binding.chord), binding.action)
    }
  }

  // If could be longer chord, prefer that (enter chord-wait state)
  let hasLongerChords = false
  for (const action of chordWinners.values()) {
    if (action !== null) {
      hasLongerChords = true
      break
    }
  }

  if (hasLongerChords) {
    return { type: 'chord_started', pending: testChord }
  }

  // Check for exact matches (last one wins)
  let exactMatch: ParsedBinding | undefined
  for (const binding of contextBindings) {
    if (chordExactlyMatches(testChord, binding)) {
      exactMatch = binding
    }
  }

  if (exactMatch) {
    if (exactMatch.action === null) {
      return { type: 'unbound' }
    }
    return { type: 'match', action: exactMatch.action }
  }

  // No match, cancel chord if pending
  if (pending !== null) {
    return { type: 'chord_cancelled' }
  }

  return { type: 'none' }
}
```

**Key Insight**: If keystroke could start a longer chord, enter `chord_started` state even if there's an exact single-key match. This allows "ctrl+x" to wait for "ctrl+x ctrl+k" while still supporting single-key bindings.

**Null-Unbind Shadow (lines 196-208):**

```typescript
// Group by chord string so a later null-override shadows the default it unbinds
const chordWinners = new Map<string, string | null>()
for (const binding of contextBindings) {
  if (
    binding.chord.length > testChord.length &&
    chordPrefixMatches(testChord, binding)
  ) {
    chordWinners.set(chordToString(binding.chord), binding.action)
  }
}
```

**Why This Matters**: Without grouping, null-unbinding `ctrl+x ctrl+k` would still make `ctrl+x` enter chord-wait, preventing the single-key binding on `ctrl+x` from firing.

### 4.6 React Context (`KeybindingContext.tsx`)

**Handler Registry (lines 82-106):**

```typescript
const registerHandler = (registration: HandlerRegistration) => {
  const registry = handlerRegistryRef.current
  if (!registry) return () => {}

  if (!registry.has(registration.action)) {
    registry.set(registration.action, new Set())
  }
  registry.get(registration.action).add(registration)
  return () => {
    const handlers = registry.get(registration.action)
    if (handlers) {
      handlers.delete(registration)
      if (handlers.size === 0) {
        registry.delete(registration.action)
      }
    }
  }
}
```

**Why Ref**: Registry needs to persist across renders without triggering re-renders.

**Action Invocation (lines 108-133):**

```typescript
const invokeAction = (action: string): boolean => {
  const registry = handlerRegistryRef.current
  if (!registry) return false

  const handlers = registry.get(action)
  if (!handlers || handlers.size === 0) return false

  // Find handlers whose context is active
  for (const registration of handlers) {
    if (activeContexts.has(registration.context)) {
      registration.handler()
      return true
    }
  }
  return false
}
```

**Context Priority**: First handler with active context wins.

**Pending Chord Ref (lines 47-48):**

```typescript
/** Ref for immediate access to pending chord (avoids React state delay) */
pendingChordRef: RefObject<ParsedKeystroke[] | null>
```

**Why Ref**: Second key of chord might arrive before React re-renders with updated state.

### 4.7 Keybinding Hooks (`useKeybinding.ts`)

**Single Handler Hook (lines 33-97):**

```typescript
export function useKeybinding(
  action: string,
  handler: () => void | false | Promise<void>,
  options: Options = {},
): void {
  const { context = 'Global', isActive = true } = options
  const keybindingContext = useOptionalKeybindingContext()

  // Register handler with context
  useEffect(() => {
    if (!keybindingContext || !isActive) return
    return keybindingContext.registerHandler({ action, context, handler })
  }, [action, context, handler, keybindingContext, isActive])

  const handleInput = useCallback(
    (input: string, key: Key, event: InputEvent) => {
      if (!keybindingContext) return

      // Build context list: registered + this context + Global
      const contextsToCheck: KeybindingContextName[] = [
        ...keybindingContext.activeContexts,
        context,
        'Global',
      ]
      const uniqueContexts = [...new Set(contextsToCheck)]

      const result = keybindingContext.resolve(input, key, uniqueContexts)

      switch (result.type) {
        case 'match':
          keybindingContext.setPendingChord(null)
          if (result.action === action) {
            if (handler() !== false) {
              event.stopImmediatePropagation()
            }
          }
          break
        case 'chord_started':
          keybindingContext.setPendingChord(result.pending)
          event.stopImmediatePropagation()
          break
        case 'chord_cancelled':
        case 'unbound':
          keybindingContext.setPendingChord(null)
          event.stopImmediatePropagation()
          break
        case 'none':
          break  // Let other handlers try
      }
    },
    [action, context, handler, keybindingContext],
  )

  useInput(handleInput, { isActive })
}
```

**Handler Return Value**: Returning `false` means "not consumed" — event propagates to other handlers.

**Context List Building**: Registered active contexts + handler's context + Global = all contexts to check.

### 4.8 User Binding Loader (`loadUserBindings.ts`)

**Feature Gate Check (lines 41-46):**

```typescript
export function isKeybindingCustomizationEnabled(): boolean {
  return getFeatureValue_CACHED_MAY_BE_STALE(
    'tengu_keybinding_customization_release',
    false,
  )
}
```

**Gated to Anthropic**: Currently only available for internal users.

**File Stability Detection (lines 49-56):**

```typescript
const FILE_STABILITY_THRESHOLD_MS = 500
const FILE_STABILITY_POLL_INTERVAL_MS = 200
```

**Chokidar Config (lines 386-396):**

```typescript
watcher = chokidar.watch(userPath, {
  persistent: true,
  ignoreInitial: true,
  awaitWriteFinish: {
    stabilityThreshold: FILE_STABILITY_THRESHOLD_MS,
    pollInterval: FILE_STABILITY_POLL_INTERVAL_MS,
  },
  ignorePermissionErrors: true,
  usePolling: false,
  atomic: true,
})
```

**Telemetry (lines 77-90):**

```typescript
let lastCustomBindingsLogDate: string | null = null

function logCustomBindingsLoadedOncePerDay(userBindingCount: number): void {
  const today = new Date().toISOString().slice(0, 10)
  if (lastCustomBindingsLogDate === today) return
  lastCustomBindingsLogDate = today
  logEvent('tengu_custom_keybindings_loaded', {
    user_binding_count: userBindingCount,
  })
}
```

**At Most Once Per Day**: Avoids flooding analytics with repeated loads.

### 4.9 Validation (`validate.ts`)

**Duplicate JSON Key Detection (lines 258-307):**

```typescript
export function checkDuplicateKeysInJson(
  jsonString: string,
): KeybindingWarning[] {
  const warnings: KeybindingWarning[] = []

  // Find "bindings" blocks with regex
  const bindingsBlockPattern =
    /"bindings"\s*:\s*\{([^{}]*(?:\{[^{}]*\}[^{}]*)*)\}/g

  let blockMatch
  while ((bindingsBlockPattern.exec(jsonString)) !== null) {
    const blockContent = blockMatch[1]
    // Find context for this block
    const contextMatch = textBeforeBlock.match(/"context"\s*:\s*"([^"]+)"[^{]*$/)
    const context = contextMatch?.[1] ?? 'unknown'

    // Find duplicate keys within block
    const keysByName = new Map<string, number>()
    for (const keyMatch of keyPattern.exec(blockContent)) {
      const count = (keysByName.get(key) ?? 0) + 1
      if (count === 2) {
        warnings.push({
          type: 'duplicate',
          severity: 'warning',
          message: `Duplicate key "${key}" in ${context} bindings`,
          suggestion: 'JSON uses last value, earlier values ignored',
        })
      }
    }
  }
  return warnings
}
```

**Why Regex**: JSON.parse silently uses last value — need raw string to detect duplicates.

**Reserved Shortcut Checking (lines 373-399):**

```typescript
export function checkReservedShortcuts(
  bindings: ParsedBinding[],
): KeybindingWarning[] {
  const warnings: KeybindingWarning[] = []
  const reserved = getReservedShortcuts()

  for (const binding of bindings) {
    const keyDisplay = chordToString(binding.chord)
    const normalizedKey = normalizeKeyForComparison(keyDisplay)

    for (const res of reserved) {
      if (normalizeKeyForComparison(res.key) === normalizedKey) {
        warnings.push({
          type: 'reserved',
          severity: res.severity,
          message: `"${keyDisplay}" may not work: ${res.reason}`,
        })
      }
    }
  }
  return warnings
}
```

### 4.10 Reserved Shortcuts (`reservedShortcuts.ts`)

**Non-Rebindable (lines 16-33):**

```typescript
export const NON_REBINDABLE: ReservedShortcut[] = [
  { key: 'ctrl+c', reason: 'Used for interrupt/exit (hardcoded)', severity: 'error' },
  { key: 'ctrl+d', reason: 'Used for exit (hardcoded)', severity: 'error' },
  { key: 'ctrl+m', reason: 'Identical to Enter in terminals', severity: 'error' },
]
```

**Terminal Reserved (lines 43-54):**

```typescript
export const TERMINAL_RESERVED: ReservedShortcut[] = [
  { key: 'ctrl+z', reason: 'Unix process suspend (SIGTSTP)', severity: 'warning' },
  { key: 'ctrl+\\', reason: 'Terminal quit signal (SIGQUIT)', severity: 'error' },
]
```

**Note**: ctrl+s (XOFF) and ctrl+q (XON) NOT included — most modern terminals disable flow control.

**macOS Reserved (lines 59-67):**

```typescript
export const MACOS_RESERVED: ReservedShortcut[] = [
  { key: 'cmd+c', reason: 'macOS system copy', severity: 'error' },
  { key: 'cmd+v', reason: 'macOS system paste', severity: 'error' },
  { key: 'cmd+x', reason: 'macOS system cut', severity: 'error' },
  { key: 'cmd+q', reason: 'macOS quit application', severity: 'error' },
  { key: 'cmd+w', reason: 'macOS close window/tab', severity: 'error' },
  { key: 'cmd+tab', reason: 'macOS app switcher', severity: 'error' },
  { key: 'cmd+space', reason: 'macOS Spotlight', severity: 'error' },
]
```

---

## 5. Integration Points

### 5.1 With `ink.js`

| Component | Integration |
|-----------|-------------|
| `match.ts` | Uses `Key` type for modifier flags |
| `useKeybinding.ts` | Uses `useInput()` hook |
| `KeybindingContext.tsx` | Uses `InputEvent` for stopImmediatePropagation |

### 5.2 With `utils/platform.js`

| Component | Integration |
|-----------|-------------|
| `defaultBindings.ts` | `getPlatform()` for platform-specific bindings |
| `reservedShortcuts.ts` | `getPlatform()` for macOS reserved list |
| `parser.ts` | Display platform for opt/alt, cmd/super |

### 5.3 With `services/analytics/`

| Component | Integration |
|-----------|-------------|
| `loadUserBindings.ts` | Logs `tengu_custom_keybindings_loaded` |
| `useShortcutDisplay.ts` | Logs `tengu_keybinding_fallback_used` |
| `shortcutFormat.ts` | Logs fallback usage (non-React callers) |

### 5.4 With `utils/config.js`

| Component | Integration |
|-----------|-------------|
| `loadUserBindings.ts` | Reads keybindings.json path |

### 5.5 With `bootstrap/state.js`

| Component | Integration |
|-----------|-------------|
| `defaultBindings.ts` | Feature gates via `feature()` |

---

## 6. Data Flow

### 6.1 Keybinding Resolution Flow

```
User presses key
         │
         ▼
  Ink useInput handler fires
         │
         ▼
  handleInput(input, key, event)
         │
         ├──► buildKeystroke(input, key)
         ├──► resolveKeyWithChordState(...)
         │    ├──► Check escape (cancel chord)
         │    ├──► Check longer chord prefixes
         │    ├──► Check exact matches (last wins)
         │    └──► Return ChordResolveResult
         │
         ▼
  switch (result.type):
    match → invoke handler, stopPropagation
    chord_started → setPendingChord, stopPropagation
    chord_cancelled → clear pending
    unbound → clear pending, stopPropagation
    none → let others try
```

### 6.2 Context Priority Resolution

```
Context list: [registered active, handler context, Global]
         │
         ▼
  Filter bindings by context Set
         │
         ▼
  Iterate bindings (last wins)
         │
         ├──► Chat context bindings checked first
         ├──► Global context bindings checked last
         └──► Later binding overrides earlier
         │
         ▼
  Return matched action
```

### 6.3 Chord State Machine

```
Keystroke 1: "ctrl+k"
         │
         ▼
  Check if any binding starts with "ctrl+k"
         │
         ├──► Yes, longer chords exist
         │    └──► Return chord_started, pending = ["ctrl+k"]
         │
         └──► No longer chords
              └──► Check exact match, return result

Keystroke 2: "ctrl+s" (while pending)
         │
         ▼
  testChord = ["ctrl+k", "ctrl+s"]
         │
         ├──► Exact match found → Return match
         ├──► No match → Return chord_cancelled
         └──► Escape pressed → Return chord_cancelled
```

### 6.4 User Config Loading

```
App startup
         │
         ▼
  loadKeybindingsSync()
         │
         ├──► Check feature gate
         ├──► Read keybindings.json
         ├──► Parse and validate
         ├──► Merge with defaults (user after default)
         └──► Cache result
         │
         ▼
  initializeKeybindingWatcher()
         │
         ├──► chokidar.watch(~/.claude/keybindings.json)
         ├──► On change: loadKeybindings()
         └──► Emit keybindingsChanged signal
         │
         ▼
  React components re-render with new bindings
```

---

## 7. Key Patterns

### 7.1 Last Binding Wins

```typescript
// Merge strategy: defaults first, user after
const mergedBindings = [...defaultBindings, ...userParsed]

// Resolution: iterate all, last match wins
for (const binding of bindings) {
  if (matchesBinding(input, key, binding)) {
    match = binding  // Overwrites previous match
  }
}
```

**Why**: Users can override any default binding by adding their own.

### 7.2 Context Priority

```
Priority order: Registered active contexts → Handler context → Global
```

**Example**: ThemePicker's `ctrl+t` overrides Global's todo toggle when ThemePicker is active.

### 7.3 Chord Prefix Shadow

```typescript
// Group by chord string to handle null-unbind correctly
const chordWinners = new Map<string, string | null>()
for (const binding of contextBindings) {
  if (binding.chord.length > testChord.length && chordPrefixMatches) {
    chordWinners.set(chordToString(binding.chord), binding.action)
  }
}
```

**Why**: Null-unbinding `ctrl+x ctrl+k` should prevent `ctrl+x` from entering chord-wait.

### 7.4 Handler Return Value

```typescript
if (handler() !== false) {
  event.stopImmediatePropagation()
}
```

**Pattern**: Returning `false` means "not consumed" — lets other handlers try.

### 7.5 Ref-Based State

```typescript
// Ref for immediate access (avoids React render delay)
pendingChordRef: RefObject<ParsedKeystroke[] | null>
```

**Why**: Second chord key may arrive before React re-renders with updated state.

---

## 8. Error Handling

### 8.1 Parse Errors

```typescript
// Invalid keystroke syntax
validateKeystroke('ctrl++') → parse_error
validateKeystroke('') → parse_error
```

### 8.2 Duplicate Detection

```typescript
// Same key twice in same context
checkDuplicates(blocks) → duplicate warning
```

### 8.3 Reserved Shortcut Warnings

```typescript
// macOS cmd+c
checkReservedShortcuts(bindings) → reserved warning (severity: error)
```

### 8.4 File Watch Errors

```typescript
try {
  await fs.mkdir(memoryDir)
} catch (e) {
  // Log but continue — prompt building doesn't block
  logForDebugging(`ensureMemoryDirExists failed: ${code ?? String(e)}`)
}
```

---

## 9. Testing Considerations

### 9.1 Keystroke Parsing

```typescript
// Test: Modifier aliases
assert.deepStrictEqual(parseKeystroke('ctrl+k'), parseKeystroke('control+k'))
assert.deepStrictEqual(parseKeystroke('opt+k'), parseKeystroke('alt+k'))
assert.deepStrictEqual(parseKeystroke('cmd+k'), parseKeystroke('super+k'))
```

### 9.2 Chord Resolution

```typescript
// Test: Chord state machine
let pending: ParsedKeystroke[] | null = null

// First keystroke
let result = resolveKeyWithChordState('k', ctrlKey, ['Global'], bindings, pending)
assert.strictEqual(result.type, 'chord_started')
pending = result.pending

// Second keystroke - match
result = resolveKeyWithChordState('s', ctrlKey, ['Global'], bindings, pending)
assert.strictEqual(result.type, 'match')
assert.strictEqual(result.action, 'app:save')
```

### 9.3 Context Priority

```typescript
// Test: Chat context overrides Global
const contexts = ['Chat', 'Global']
const result = resolveKey('enter', {}, contexts, bindings)
assert.strictEqual(result.action, 'chat:submit')  // Not global default
```

### 9.4 Validation

```typescript
// Test: Duplicate detection
const warnings = checkDuplicateKeysInJson('{"bindings": {"ctrl+k": "a", "ctrl+k": "b"}}')
assert.strictEqual(warnings.length, 1)
assert.ok(warnings[0].message.includes('Duplicate key'))
```

---

## 10. Environment Variables

| Variable | Purpose | Default |
|----------|---------|---------|
| `CLAUDE_CODE_COORDINATOR_MODE` | Enable coordinator mode | — |

---

## 11. Feature Gates

| Gate | Purpose |
|------|---------|
| `tengu_keybinding_customization_release` | Enable user customization |
| `KAIROS` / `KAIROS_BRIEF` | Toggle Brief toggle binding |
| `QUICK_SEARCH` | Enable global search bindings |
| `TERMINAL_PANEL` | Enable terminal panel toggle |
| `MESSAGE_ACTIONS` | Enable MessageActions context |
| `VOICE_MODE` | Enable voice push-to-talk |

---

## 12. Telemetry Events

| Event | Location | Fields |
|-------|----------|--------|
| `tengu_custom_keybindings_loaded` | `loadUserBindings.ts` | `user_binding_count` |
| `tengu_keybinding_fallback_used` | `useShortcutDisplay.ts`, `shortcutFormat.ts` | `action`, `context`, `fallback`, `reason` |

---

## 13. Summary

The `keybindings/` module is a **comprehensive keyboard shortcut system**:

1. **Schema-Driven** — Zod schemas for 18 contexts, 70+ actions
2. **Chord Support** — Multi-keystroke sequences like "ctrl+k ctrl+s"
3. **Context Priority** — Specific contexts override Global
4. **React Integration** — Hooks for handling and displaying shortcuts
5. **User Customization** — Hot-reload config with validation
6. **Platform Handling** — macOS reserved shortcuts, VT mode detection
7. **Feature Gating** — Customization gated to Anthropic employees

**Key Architectural Decisions**:
- **Last Binding Wins** — User bindings override defaults
- **Ref-Based Pending Chord** — Avoids React render delay
- **Handler Registry** — Centralized invocation via ChordInterceptor
- **Chord Prefix Shadow** — Null-unbind properly shadows prefixes
- **Platform Display** — "opt" vs "alt", "cmd" vs "super"

---

**Last Updated:** 2026-04-07  
**Status:** Complete — all 14 files analyzed
