# Moreright Module — Deep-Dive Exploration

**Module:** `moreright/`  
**Location:** `/home/darkvoid/Boxxed/@formulas/src.rust/src.llamacpp/src.ClaudOpen/claude-code-main/src/moreright/`  
**Files:** 1 TSX file  
**Created:** 2026-04-07

---

## 1. Module Overview

The `moreright/` module implements **internal-only message enhancement hook** — a React hook that provides message transformation capabilities during query processing. This is an internal Anthropic tool (not available in external builds) for experimenting with message augmentation and turn processing.

### Core Responsibilities

1. **Query Interception** — Pre-query hook:
   - `onBeforeQuery()` called before each model request
   - Can modify or validate input
   - Returns boolean to allow/deny query

2. **Turn Completion** — Post-turn hook:
   - `onTurnComplete()` called after model response
   - Can analyze complete turn output
   - Supports aborted turn detection

3. **Message Manipulation** — State updates:
   - `setMessages()` for message array modifications
   - `setInputValue()` for prompt input control
   - `setToolJSX()` for tool UI rendering

### Key Design Patterns

- **Stub Pattern**: External builds receive no-op stub
- **Hook Return Object**: All callbacks returned as object
- **Internal Only**: Feature-gated out of external releases

---

## 2. File Inventory

| File | Lines | Description |
|------|-------|-------------|
| `useMoreRight.tsx` | ~26 | Internal hook (stub for external) |

**Total:** ~26 lines (stub implementation)

---

## 3. Key Exports

```typescript
// Internal hook (stub for external builds)
export function useMoreRight(_args: {
  enabled: boolean
  setMessages: (action: M[] | ((prev: M[]) => M[])) => void
  inputValue: string
  setInputValue: (s: string) => void
  setToolJSX: (args: M) => void
}): {
  onBeforeQuery: (input: string, all: M[], n: number) => Promise<boolean>
  onTurnComplete: (all: M[], aborted: boolean) => Promise<void>
  render: () => null
}
```

---

## 4. Line-by-Line Analysis

### 4.1 Stub Implementation (`useMoreRight.tsx` lines 7-25)

```typescript
// Stub for external builds — the real hook is internal only.
//
// Self-contained: no relative imports. Typecheck sees this file at
// scripts/external-stubs/src/moreright/ before overlay, where ../types/
// would resolve to scripts/external-stubs/src/types/ (doesn't exist).

// eslint-disable-next-line @typescript-eslint/no-explicit-any
type M = any

export function useMoreRight(_args: {
  enabled: boolean
  setMessages: (action: M[] | ((prev: M[]) => M[])) => void
  inputValue: string
  setInputValue: (s: string) => void
  setToolJSX: (args: M) => void
}): {
  onBeforeQuery: (input: string, all: M[], n: number) => Promise<boolean>
  onTurnComplete: (all: M[], aborted: boolean) => Promise<void>
  render: () => null
} {
  return {
    onBeforeQuery: async () => true,  // Always allow
    onTurnComplete: async () => {},    // No-op
    render: () => null,                // Nothing to render
  }
}
```

**Self-Contained**: "Self-contained: no relative imports. Typecheck sees this file at scripts/external-stubs/src/moreright/ before overlay."

**No-Op Implementation**:
- `onBeforeQuery`: Always returns `true` (allow query)
- `onTurnComplete`: Empty async function
- `render`: Returns `null`

**Build Process**: Real implementation overlaid during internal builds.

---

## 5. Integration Points

### 5.1 With `REPL.tsx`

| Component | Integration |
|-----------|-------------|
| `useMoreRight` | Called in REPL component for message processing |

### 5.2 With Build System

| Component | Integration |
|-----------|-------------|
| `useMoreRight.tsx` | External stub overlay during build |

---

## 6. Data Flow

### 6.1 Hook Usage Flow (Internal)

```
REPL component render
    │
    ▼
useMoreRight({enabled, setMessages, inputValue, ...})
    │
    ├──► Returns {onBeforeQuery, onTurnComplete, render}
    │
    ▼
Before query:
    onBeforeQuery(input, allMessages, turnNumber)
    │
    ├──► Can modify messages
    └──► Return true/false to allow/deny
    │
    ▼
After turn:
    onTurnComplete(allMessages, aborted)
    │
    ├──► Analyze complete turn
    └──► Update messages if needed
```

### 6.2 External Build Flow

```
External build
    │
    ▼
useMoreRight() → Stub implementation
    │
    ├──► onBeforeQuery() → true (always allow)
    ├──► onTurnComplete() → no-op
    └──► render() → null
    │
    ▼
No message modification
```

---

## 7. Key Patterns

### 7.1 External Stub Pattern

```typescript
// Internal build: Real implementation
export function useMoreRight(args) {
  // ... message augmentation logic
}

// External build: Stub (this file)
export function useMoreRight(_args) {
  return {
    onBeforeQuery: async () => true,
    onTurnComplete: async () => {},
    render: () => null,
  }
}
```

**Why**: Internal experimentation tool not for external release.

### 7.2 Hook Return Object

```typescript
return {
  onBeforeQuery: async (input, all, n) => boolean,
  onTurnComplete: async (all, aborted) => {},
  render: () => null,
}
```

**Pattern**: All callbacks returned as single object for destructuring.

---

## 8. Environment Variables

| Variable | Purpose | Values |
|----------|---------|--------|
| N/A | No environment variables | Stub is always no-op externally |

---

## 9. Summary

The `moreright/` module provides **internal message enhancement hook**:

1. **Query Interception** — Pre-query modification via `onBeforeQuery()`
2. **Turn Completion** — Post-turn analysis via `onTurnComplete()`
3. **External Stub** — No-op implementation for external builds

**Key Design Decisions**:
- **Stub pattern** for internal-only functionality
- **Self-contained** to avoid import resolution issues
- **No-op defaults** that don't interfere with normal operation

**Note**: This is an **internal Anthropic tool** — the real implementation is only available in internal builds. External users receive a stub that always allows queries and performs no modifications.

---

**Last Updated:** 2026-04-07  
**Status:** Complete — 1 of 1 files analyzed (stub implementation)
