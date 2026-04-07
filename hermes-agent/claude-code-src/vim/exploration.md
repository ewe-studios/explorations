# Vim Module — Deep-Dive Exploration

**Module:** `vim/`  
**Location:** `/home/darkvoid/Boxxed/@formulas/src.rust/src.llamacpp/src.ClaudOpen/claude-code-main/src/vim/`  
**Files:** 5 TypeScript files  
**Created:** 2026-04-07

---

## 1. Module Overview

The `vim/` module implements **vim mode input handling** — a complete state machine for vim-style text editing within Claude Code's terminal UI. It provides normal mode commands (operators, motions, text objects) and insert mode with dot-repeat functionality.

### Core Responsibilities

1. **State Machine** — Vim mode state management:
   - INSERT mode: tracks inserted text for dot-repeat
   - NORMAL mode: command state machine (idle, count, operator, find, etc.)
   - Persistent state: last change, last find, register content

2. **Operator Execution** — Text manipulation commands:
   - Delete (`d`), Change (`c`), Yank (`y`)
   - Line operations (`dd`, `cc`, `yy`)
   - Character delete (`x`), Replace (`r`)
   - Toggle case (`~`), Join lines (`J`)
   - Paste (`p`, `P`)

3. **Motion Resolution** — Cursor movement:
   - Simple motions (`h`, `j`, `k`, `l`, `w`, `b`, `e`, etc.)
   - Find motions (`f`, `F`, `t`, `T`)
   - Line motions (`0`, `^`, `$`, `g_`)
   - Special motions (`G`, `gg`, `%`)

4. **Text Objects** — Structural selection:
   - Inner/around for words, quotes, brackets
   - Scope types: `iw`, `aw`, `i"`, `a"`, `ib`, `ab`, etc.

5. **Transition Table** — Input handling:
   - State-based input routing
   - Count accumulation
   - Operator + motion composition

### Key Design Patterns

- **State Machine Pattern**: Exhaustive state types for compile-time safety
- **Pure Functions**: Operators and transitions are pure functions with context
- **Transition Table**: Scannable source of truth for state transitions
- **Grapheme-Aware**: Uses `firstGrapheme`/`lastGrapheme` for Unicode correctness
- **Recorded Changes**: Dot-repeat via change recording

---

## 2. File Inventory

| File | Lines | Description |
|------|-------|-------------|
| `types.ts` | ~200 | State machine types and constants |
| `operators.ts` | ~450 | Operator execution functions |
| `transitions.ts` | ~350+ | State transition table |
| `motions.ts` | ~200+ | Motion resolution functions |
| `textObjects.ts` | ~130 | Text object finding functions |

**Total:** ~1330+ lines across 5 files

---

## 3. Key Exports

### State Machine Types (`types.ts`)

```typescript
// Operator types
export type Operator = 'delete' | 'change' | 'yank'

// Find motion types
export type FindType = 'f' | 'F' | 't' | 'T'

// Text object scope
export type TextObjScope = 'inner' | 'around'

// Complete vim state
export type VimState =
  | { mode: 'INSERT'; insertedText: string }
  | { mode: 'NORMAL'; command: CommandState }

// Command state machine for NORMAL mode
export type CommandState =
  | { type: 'idle' }
  | { type: 'count'; digits: string }
  | { type: 'operator'; op: Operator; count: number }
  | { type: 'operatorCount'; op: Operator; count: number; digits: string }
  | { type: 'operatorFind'; op: Operator; count: number; find: FindType }
  | { type: 'operatorTextObj'; op: Operator; count: number; scope: TextObjScope }
  | { type: 'find'; find: FindType; count: number }
  | { type: 'g'; count: number }
  | { type: 'operatorG'; op: Operator; count: number }
  | { type: 'replace'; count: number }
  | { type: 'indent'; dir: '>' | '<'; count: number }

// Persistent state (survives across commands)
export type PersistentState = {
  lastChange: RecordedChange | null
  lastFind: { type: FindType; char: string } | null
  register: string
  registerIsLinewise: boolean
}

// Recorded change for dot-repeat
export type RecordedChange =
  | { type: 'insert'; text: string }
  | { type: 'operator'; op: Operator; motion: string; count: number }
  | { type: 'operatorTextObj'; op: Operator; objType: string; scope: TextObjScope; count: number }
  | { type: 'operatorFind'; op: Operator; find: FindType; char: string; count: number }
  | { type: 'replace'; char: string; count: number }
  | { type: 'x'; count: number }
  | { type: 'toggleCase'; count: number }
  | { type: 'indent'; dir: '>' | '<'; count: number }
  | { type: 'openLine'; direction: 'above' | 'below' }
  | { type: 'join'; count: number }
```

### Operator Functions (`operators.ts`)

```typescript
// Operator context
export type OperatorContext = {
  cursor: Cursor
  text: string
  setText: (text: string) => void
  setOffset: (offset: number) => void
  enterInsert: (offset: number) => void
  getRegister: () => string
  setRegister: (content: string, linewise: boolean) => void
  getLastFind: () => { type: FindType; char: string } | null
  setLastFind: (type: FindType, char: string) => void
  recordChange: (change: RecordedChange) => void
}

// Operator execution
export function executeOperatorMotion(
  op: Operator,
  motion: string,
  count: number,
  ctx: OperatorContext,
): void

export function executeOperatorFind(
  op: Operator,
  findType: FindType,
  char: string,
  count: number,
  ctx: OperatorContext,
): void

export function executeOperatorTextObj(
  op: Operator,
  scope: TextObjScope,
  objType: string,
  count: number,
  ctx: OperatorContext,
): void

export function executeLineOp(
  op: Operator,
  count: number,
  ctx: OperatorContext,
): void

export function executeX(count: number, ctx: OperatorContext): void
export function executeReplace(char: string, count: number, ctx: OperatorContext): void
```

### Transition Functions (`transitions.ts`)

```typescript
// Transition context
export type TransitionContext = OperatorContext & {
  onUndo?: () => void
  onDotRepeat?: () => void
}

// Transition result
export type TransitionResult = {
  next?: CommandState
  execute?: () => void
}

// Main transition function
export function transition(
  state: CommandState,
  input: string,
  ctx: TransitionContext,
): TransitionResult
```

---

## 4. Line-by-Line Analysis

### 4.1 State Machine Diagram (`types.ts` lines 7-26)

```typescript
/**
 * State Diagram:
 * ```
 *                              VimState
 *   ┌──────────────────────────────┬──────────────────────────────────────┐
 *   │  INSERT                      │  NORMAL                              │
 *   │  (tracks insertedText)       │  (CommandState machine)              │
 *   │                              │                                      │
 *   │                              │  idle ──┬─[d/c/y]──► operator        │
 *   │                              │         ├─[1-9]────► count           │
 *   │                              │         ├─[fFtT]───► find            │
 *   │                              │         ├─[g]──────► g               │
 *   │                              │         ├─[r]──────► replace         │
 *   │                              │         └─[><]─────► indent          │
 *   │                              │                                      │
 *   │                              │  operator ─┬─[motion]──► execute     │
 *   │                              │            ├─[0-9]────► operatorCount│
 *   │                              │            ├─[ia]─────► operatorTextObj
 *   │                              │            └─[fFtT]───► operatorFind │
 *   └──────────────────────────────┴──────────────────────────────────────┘
 * ```
 */
```

**Visual Documentation**: ASCII state diagram shows all NORMAL mode transitions.

### 4.2 Key Groups (`types.ts` lines 125-182)

```typescript
export const OPERATORS = {
  d: 'delete',
  c: 'change',
  y: 'yank',
} as const satisfies Record<string, Operator>

export function isOperatorKey(key: string): key is keyof typeof OPERATORS {
  return key in OPERATORS
}

export const SIMPLE_MOTIONS = new Set([
  'h', 'l', 'j', 'k',  // Basic movement
  'w', 'b', 'e', 'W', 'B', 'E',  // Word motions
  '0', '^', '$',  // Line positions
])

export const FIND_KEYS = new Set(['f', 'F', 't', 'T'])

export const TEXT_OBJ_SCOPES = {
  i: 'inner',
  a: 'around',
} as const satisfies Record<string, TextObjScope>

export const TEXT_OBJ_TYPES = new Set([
  'w', 'W',  // Word/WORD
  '"', "'", '`',  // Quotes
  '(', ')', 'b',  // Parens
  '[', ']',  // Brackets
  '{', '}', 'B',  // Braces
  '<', '>',  // Angle brackets
])
```

**Named Constants**: No magic strings — all key groups defined as constants.

### 4.3 Main Transition Function (`transitions.ts` lines 59-88)

```typescript
export function transition(
  state: CommandState,
  input: string,
  ctx: TransitionContext,
): TransitionResult {
  switch (state.type) {
    case 'idle':
      return fromIdle(input, ctx)
    case 'count':
      return fromCount(state, input, ctx)
    case 'operator':
      return fromOperator(state, input, ctx)
    case 'operatorCount':
      return fromOperatorCount(state, input, ctx)
    case 'operatorFind':
      return fromOperatorFind(state, input, ctx)
    case 'operatorTextObj':
      return fromOperatorTextObj(state, input, ctx)
    case 'find':
      return fromFind(state, input, ctx)
    case 'g':
      return fromG(state, input, ctx)
    case 'operatorG':
      return fromOperatorG(state, input, ctx)
    case 'replace':
      return fromReplace(state, input, ctx)
    case 'indent':
      return fromIndent(state, input, ctx)
  }
}
```

**Exhaustive Handling**: TypeScript ensures all state types are handled.

### 4.4 Normal Input Handling (`transitions.ts` lines 98-200)

```typescript
function handleNormalInput(
  input: string,
  count: number,
  ctx: TransitionContext,
): TransitionResult | null {
  // Operators
  if (isOperatorKey(input)) {
    return { next: { type: 'operator', op: OPERATORS[input], count } }
  }

  // Simple motions
  if (SIMPLE_MOTIONS.has(input)) {
    return {
      execute: () => {
        const target = resolveMotion(input, ctx.cursor, count)
        ctx.setOffset(target.offset)
      },
    }
  }

  // Find motions
  if (FIND_KEYS.has(input)) {
    return { next: { type: 'find', find: input as FindType, count } }
  }

  // Special commands
  if (input === 'g') return { next: { type: 'g', count } }
  if (input === 'r') return { next: { type: 'replace', count } }
  if (input === '>' || input === '<') {
    return { next: { type: 'indent', dir: input, count } }
  }
  if (input === '~') {
    return { execute: () => executeToggleCase(count, ctx) }
  }
  if (input === 'x') {
    return { execute: () => executeX(count, ctx) }
  }
  if (input === 'J') {
    return { execute: () => executeJoin(count, ctx) }
  }
  if (input === 'p' || input === 'P') {
    return { execute: () => executePaste(input === 'p', count, ctx) }
  }
  if (input === 'D') {
    return { execute: () => executeOperatorMotion('delete', '$', 1, ctx) }
  }
  if (input === 'C') {
    return { execute: () => executeOperatorMotion('change', '$', 1, ctx) }
  }
  if (input === 'Y') {
    return { execute: () => executeLineOp('yank', count, ctx) }
  }
  if (input === 'G') {
    return {
      execute: () => {
        if (count === 1) {
          ctx.setOffset(ctx.cursor.startOfLastLine().offset)
        } else {
          ctx.setOffset(ctx.cursor.goToLine(count).offset)
        }
      },
    }
  }
  if (input === '.') {
    return { execute: () => ctx.onDotRepeat?.() }
  }
  if (input === ';' || input === ',') {
    return { execute: () => executeRepeatFind(input === ',', count, ctx) }
  }
  if (input === 'u') {
    return { execute: () => ctx.onUndo?.() }
  }
  // ... insert mode entry (i, I, a, A, o, O)
}
```

**Command Coverage**: All standard vim normal mode commands handled.

### 4.5 Line Operation (`operators.ts` lines 102-166)

```typescript
export function executeLineOp(
  op: Operator,
  count: number,
  ctx: OperatorContext,
): void {
  const text = ctx.text
  const lines = text.split('\n')
  const currentLine = countCharInString(text.slice(0, ctx.cursor.offset), '\n')
  const linesToAffect = Math.min(count, lines.length - currentLine)
  const lineStart = ctx.cursor.startOfLogicalLine().offset
  let lineEnd = lineStart
  for (let i = 0; i < linesToAffect; i++) {
    const nextNewline = text.indexOf('\n', lineEnd)
    lineEnd = nextNewline === -1 ? text.length : nextNewline + 1
  }

  let content = text.slice(lineStart, lineEnd)
  // Ensure linewise content ends with newline for paste detection
  if (!content.endsWith('\n')) {
    content = content + '\n'
  }
  ctx.setRegister(content, true)

  if (op === 'yank') {
    ctx.setOffset(lineStart)
  } else if (op === 'delete') {
    let deleteStart = lineStart
    const deleteEnd = lineEnd

    // If deleting to end of file and there's a preceding newline, include it
    if (
      deleteEnd === text.length &&
      deleteStart > 0 &&
      text[deleteStart - 1] === '\n'
    ) {
      deleteStart -= 1
    }

    const newText = text.slice(0, deleteStart) + text.slice(deleteEnd)
    ctx.setText(newText || '')
    const maxOff = Math.max(0, newText.length - (lastGrapheme(newText).length || 1))
    ctx.setOffset(Math.min(deleteStart, maxOff))
  } else if (op === 'change') {
    // For single line, just clear it
    if (lines.length === 1) {
      ctx.setText('')
      ctx.enterInsert(0)
    } else {
      // Delete all affected lines, replace with single empty line, enter insert
      const beforeLines = lines.slice(0, currentLine)
      const afterLines = lines.slice(currentLine + linesToAffect)
      const newText = [...beforeLines, '', ...afterLines].join('\n')
      ctx.setText(newText)
      ctx.enterInsert(lineStart)
    }
  }

  ctx.recordChange({ type: 'operator', op, motion: op[0]!, count })
}
```

**Linewise Semantics**: Ensures register content ends with newline for paste detection.

**Grapheme-Aware**: Uses `lastGrapheme()` to handle Unicode grapheme clusters.

### 4.6 Character Delete (`operators.ts` lines 171-194)

```typescript
export function executeX(count: number, ctx: OperatorContext): void {
  const from = ctx.cursor.offset

  if (from >= ctx.text.length) return

  // Advance by graphemes, not code units
  let endCursor = ctx.cursor
  for (let i = 0; i < count && !endCursor.isAtEnd(); i++) {
    endCursor = endCursor.right()
  }
  const to = endCursor.offset

  const deleted = ctx.text.slice(from, to)
  const newText = ctx.text.slice(0, from) + ctx.text.slice(to)

  ctx.setRegister(deleted, false)
  ctx.setText(newText)
  const maxOff = Math.max(0, newText.length - (lastGrapheme(newText).length || 1))
  ctx.setOffset(Math.min(from, maxOff))
  ctx.recordChange({ type: 'x', count })
}
```

**Grapheme Iteration**: "Advance by graphemes, not code units" for Unicode correctness.

---

## 5. Integration Points

### 5.1 With `utils/Cursor.js`

| Component | Integration |
|-----------|-------------|
| `operators.ts` | Uses `Cursor` class for cursor positioning |
| `motions.ts` | Uses `Cursor` methods for motion resolution |

### 5.2 With `utils/intl.js`

| Component | Integration |
|-----------|-------------|
| `operators.ts` | Uses `firstGrapheme()`, `lastGrapheme()` |

### 5.3 With `utils/stringUtils.js`

| Component | Integration |
|-----------|-------------|
| `operators.ts` | Uses `countCharInString()` for line counting |

---

## 6. Data Flow

### 6.1 Input Processing Flow

```
User keypress
    │
    ▼
transition(state, input, ctx)
    │
    ├──► fromIdle() → {next, execute}
    ├──► fromCount() → {next, execute}
    ├──► fromOperator() → {next, execute}
    └──► ...
    │
    ▼
If execute: run function
    │
    ├──► executeOperatorMotion()
    ├──► executeOperatorFind()
    ├──► executeOperatorTextObj()
    └──► executeLineOp()
    │
    ▼
Update cursor, text, register
    │
    ▼
recordChange() for dot-repeat
```

### 6.2 Operator + Motion Flow

```
User presses 'd' (delete operator)
    │
    ▼
State: {type: 'operator', op: 'delete', count: 1}
    │
    ▼
User presses 'w' (word motion)
    │
    ▼
fromOperator() calls executeOperatorMotion('delete', 'w', 1, ctx)
    │
    ├──► resolveMotion('w', cursor, 1) → target cursor
    ├──► getOperatorRange() → {from, to, linewise}
    └──► applyOperator() → delete text[from:to]
    │
    ▼
recordChange({type: 'operator', op: 'delete', motion: 'w', count: 1})
```

### 6.3 Dot-Repeat Flow

```
User presses '.' (dot-repeat)
    │
    ▼
ctx.onDotRepeat?.()
    │
    ▼
Match lastChange type:
├── insert → insert recorded text
├── operator → executeOperatorMotion()
├── operatorFind → executeOperatorFind()
├── operatorTextObj → executeOperatorTextObj()
├── x → executeX()
└── ...
```

---

## 7. Key Patterns

### 7.1 State Machine Type Safety

```typescript
export type CommandState =
  | { type: 'idle' }
  | { type: 'count'; digits: string }
  | { type: 'operator'; op: Operator; count: number }
  // ...

// TypeScript ensures exhaustive handling
switch (state.type) {
  case 'idle': ...
  case 'count': ...
  // Missing a case? TypeScript error!
}
```

### 7.2 Context Pattern

```typescript
export type OperatorContext = {
  cursor: Cursor
  text: string
  setText: (text: string) => void
  setOffset: (offset: number) => void
  enterInsert: (offset: number) => void
  getRegister: () => string
  setRegister: (content: string, linewise: boolean) => void
  getLastFind: () => {...} | null
  setLastFind: (type: FindType, char: string) => void
  recordChange: (change: RecordedChange) => void
}
```

**Why**: Pure functions with context — testable, no hidden dependencies.

### 7.3 Grapheme Awareness

```typescript
// Wrong: code unit iteration
for (let i = 0; i < count; i++) {
  offset++  // Breaks for emoji, combined characters
}

// Correct: grapheme iteration
let endCursor = ctx.cursor
for (let i = 0; i < count && !endCursor.isAtEnd(); i++) {
  endCursor = endCursor.right()  // Uses Intl.Segmenter internally
}
```

### 7.4 Transition Table

```typescript
// Scannable source of truth
function transition(state, input, ctx): TransitionResult {
  switch (state.type) {
    case 'idle': return fromIdle(input, ctx)
    case 'count': return fromCount(state, input, ctx)
    // ...each state has its own handler
  }
}
```

---

## 8. Summary

The `vim/` module provides **complete vim mode emulation**:

1. **State Machine** — INSERT and NORMAL modes with full command parsing
2. **Operator System** — Delete, change, yank with motions, text objects, finds
3. **Motion Resolution** — All standard vim motions (simple, find, special)
4. **Text Objects** — Inner/around for words, quotes, brackets, braces
5. **Dot-Repeat** — Change recording and replay
6. **Unicode Support** — Grapheme-aware cursor movement

**Key Design Decisions**:
- **Type-safe state machine** — TypeScript exhaustiveness checking
- **Pure functions with context** — Testable, no hidden state
- **Grapheme awareness** — Correct Unicode handling
- **Transition table pattern** — Scannable, maintainable

---

**Last Updated:** 2026-04-07  
**Status:** Complete — All 5 files analyzed
