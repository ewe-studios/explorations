# Native-TS Module — Deep-Dive Exploration

**Module:** `native-ts/`  
**Location:** `/home/darkvoid/Boxxed/@formulas/src.rust/src.llamacpp/src.ClaudOpen/claude-code-main/src/native-ts/`  
**Files:** 4 TypeScript files across 3 subdirectories  
**Created:** 2026-04-07

---

## 1. Module Overview

The `native-ts/` module implements **pure TypeScript ports of native Rust NAPI modules** — providing fallback implementations for environments where native modules cannot be loaded. These ports maintain API compatibility with the Rust versions while eliminating native dependency requirements.

### Core Responsibilities

1. **Color Diff** (`color-diff/`) — Syntax-highlighted diff rendering:
   - Line-by-line diff with word-level highlighting
   - Syntax highlighting via highlight.js
   - ANSI escape code generation
   - Theme support (truecolor, 256-color, ANSI)

2. **File Index** (`file-index/`) — High-performance fuzzy file search:
   - Nucleo-inspired scoring algorithm
   - Character bitmap indexing for fast lookup
   - Async incremental indexing
   - Test file penalty scoring

3. **Yoga Layout** (`yoga-layout/`) — Flexbox layout engine enums:
   - Complete Yoga enum definitions
   - Type-safe enum values
   - Upstream-compatible values

### Key Design Patterns

- **API Compatibility**: Exact API match with native modules
- **Lazy Loading**: Deferred heavy dependency loading
- **Async Chunking**: Time-sliced indexing for responsiveness
- **Pure TypeScript**: No native dependencies

---

## 2. File Inventory

| File | Lines | Description |
|------|-------|-------------|
| `color-diff/index.ts` | ~850+ | Syntax-highlighted diff rendering |
| `file-index/index.ts` | ~350+ | Fuzzy file search index |
| `yoga-layout/enums.ts` | ~135 | Yoga layout enum definitions |
| `yoga-layout/index.ts` | ~2300+ | Yoga layout engine port |

**Total:** ~3600+ lines across 4 files

---

## 3. Key Exports

### Color Diff (`color-diff/index.ts`)

```typescript
// Diff result types
export type Hunk = {
  oldStart: number
  oldLines: number
  newStart: number
  newLines: number
  lines: string[]
}

export type SyntaxTheme = {
  theme: string
  source: string | null
}

// Main API
export type NativeModule = {
  ColorDiff: typeof ColorDiff
  ColorFile: typeof ColorFile
  getSyntaxTheme: (themeName: string) => SyntaxTheme
}

// ColorDiff class
export class ColorDiff {
  static diffLines(oldStr: string, newStr: string): Hunk[]
  static colorizeHunk(hunk: Hunk, options: ColorizeOptions): string
}

// ColorFile class
export class ColorFile {
  static colorize(content: string, language?: string): string
}
```

### File Index (`file-index/index.ts`)

```typescript
// Search result
export type SearchResult = {
  path: string
  score: number  // Lower = better
}

// Main class
export class FileIndex {
  constructor()
  
  // Synchronous loading
  loadFromFileList(fileList: string[]): void
  
  // Async loading (yields to event loop)
  loadFromFileListAsync(fileList: string[]): {
    queryable: Promise<void>
    done: Promise<void>
  }
  
  // Search
  search(query: string, limit?: number): SearchResult[]
}
```

### Yoga Layout (`yoga-layout/enums.ts`, `index.ts`)

```typescript
// Enum objects (const pattern, not TS enums)
export const Align = { Auto: 0, FlexStart: 1, Center: 2, ... }
export const FlexDirection = { Column: 0, Row: 2, ... }
export const Justify = { FlexStart: 0, Center: 1, ... }
export const Wrap = { NoWrap: 0, Wrap: 1, WrapReverse: 2 }

// Node class
export class YogaNode {
  calculateLayout(width: number, height: number): void
  getComputedLeft(): number
  getComputedTop(): number
  // ... full Yoga API
}
```

---

## 4. Line-by-Line Analysis

### 4.1 Color Diff: Lazy highlight.js (`color-diff/index.ts` lines 20-43)

```typescript
// Lazy: defers loading highlight.js until first render. The full bundle
// registers 190+ language grammars at require time (~50MB, 100-200ms on
// macOS, several× that on Windows). With a top-level import, any caller
// chunk that reaches this module — including test/preload.ts via
// StructuredDiff.tsx → colorDiff.ts — pays that cost at module-eval time
// and carries the heap for the rest of the process.
type HLJSApi = typeof hljsNamespace
let cachedHljs: HLJSApi | null = null

function hljs(): HLJSApi {
  if (cachedHljs) return cachedHljs
  const mod = require('highlight.js')
  // highlight.js uses `export =` (CJS). Under bun/ESM the interop wraps it
  // in .default; under node CJS the module IS the API. Check at runtime.
  cachedHljs = 'default' in mod && mod.default ? mod.default : mod
  return cachedHljs!
}
```

**Lazy Loading Rationale**: "With a top-level import, any caller chunk that reaches this module pays that cost at module-eval time and carries the heap for the rest of the process."

**Performance**: ~50MB, 100-200ms load time on macOS, several× that on Windows.

**Interop Handling**: Detects `.default` for Bun/ESM vs Node CJS.

### 4.2 File Index: Scoring Constants (`file-index/index.ts` lines 23-38)

```typescript
// nucleo-style scoring constants (approximating fzf-v2 / nucleo bonuses)
const SCORE_MATCH = 16
const BONUS_BOUNDARY = 8
const BONUS_CAMEL = 6
const BONUS_CONSECUTIVE = 4
const BONUS_FIRST_CHAR = 8
const PENALTY_GAP_START = 3
const PENALTY_GAP_EXTENSION = 1

const TOP_LEVEL_CACHE_LIMIT = 100
const MAX_QUERY_LEN = 64

// Yield to event loop after this many ms of sync work. Chunk sizes are
// time-based (not count-based) so slow machines get smaller chunks and
// stay responsive — 5k paths is ~2ms on M-series but could be 15ms+ on
// older Windows hardware.
const CHUNK_MS = 4
```

**Time-Based Chunking**: "Chunk sizes are time-based (not count-based) so slow machines get smaller chunks and stay responsive."

**Performance Target**: ~2ms on M-series, ~15ms on older Windows hardware per chunk.

### 4.3 File Index: Search Result Type (`file-index/index.ts` lines 18-21)

```typescript
export type SearchResult = {
  path: string
  score: number  // Lower = better
}
```

**Score Semantics**: "Score is position-in-results / result-count, so the best match is 0.0."

**Test Penalty**: "Paths containing 'test' get a 1.05× penalty (capped at 1.0) so non-test files rank slightly higher."

### 4.4 File Index: Async Loading (`file-index/index.ts` lines 83-93)

```typescript
/**
 * Async variant: yields to the event loop every ~8–12k paths so large
 * indexes (270k+ files) don't block the main thread for >10ms at a time.
 * Identical result to loadFromFileList.
 *
 * Returns { queryable, done }:
 *   - queryable: resolves as soon as the first chunk is indexed (search
 *     returns partial results). For a 270k-path list this is ~5–10ms of
 *     sync work after the paths array is available.
 *   - done: resolves when the entire index is built.
 */
loadFromFileListAsync(fileList: string[]): {
  queryable: Promise<void>
  done: Promise<void>
}
```

**Partial Results**: "queryable resolves as soon as the first chunk is indexed (search returns partial results)."

**Initial Latency**: ~5-10ms for 270k paths before first results available.

### 4.5 Yoga Enums: Const Pattern (`yoga-layout/enums.ts` lines 7-18)

```typescript
export const Align = {
  Auto: 0,
  FlexStart: 1,
  Center: 2,
  FlexEnd: 3,
  Stretch: 4,
  Baseline: 5,
  SpaceBetween: 6,
  SpaceAround: 7,
  SpaceEvenly: 8,
} as const

export type Align = (typeof Align)[keyof typeof Align]
```

**Const Pattern**: Using `const` objects instead of TypeScript enums per repo convention.

**Type Inference**: `keyof typeof` pattern for type-safe values.

### 4.6 Color Diff: Theme Detection (`color-diff/index.ts` lines 95-99)

```typescript
function detectColorMode(theme: string): ColorMode {
  if (theme.includes('ansi')) return 'ansi'
  const ct = process.env.COLORTERM ?? ''
  return ct === 'truecolor' || ct === '24bit' ? 'truecolor' : 'color256'
}
```

**Environment Detection**: Uses `COLORTERM` env var for color mode detection.

**ANSI Fallback**: Theme names containing 'ansi' force ANSI mode.

---

## 5. Integration Points

### 5.1 With `diff` Package

| Component | Integration |
|-----------|-------------|
| `color-diff/index.ts` | Uses `diffArrays` for word-level diffing |

### 5.2 With `highlight.js`

| Component | Integration |
|-----------|-------------|
| `color-diff/index.ts` | Lazy-loaded for syntax highlighting |

### 5.3 With `ink/stringWidth.js`

| Component | Integration |
|-----------|-------------|
| `color-diff/index.ts` | Uses `stringWidth` for grapheme-aware width |

### 5.4 With Native Module API

| Component | Integration |
|-----------|-------------|
| All native-ts files | Match vendor/*-src API exactly |

---

## 6. Data Flow

### 6.1 Color Diff Flow

```
User requests diff view
    │
    ▼
ColorDiff.diffLines(oldStr, newStr)
    │
    ├──► diffArrays(oldLines, newLines)
    │    └──► Returns hunks with added/removed/unchanged
    │
    ▼
ColorDiff.colorizeHunk(hunk, options)
    │
    ├──► For each line:
    │    ├──► Determine line type (add/remove/unchanged)
    │    ├──► Word-level diff for changed lines
    │    ├──► Syntax highlight via highlight.js (lazy)
    │    └──► Generate ANSI escape codes
    │
    ▼
Return colorized string
```

### 6.2 File Index Flow

```
File list collected (e.g., from rg)
    │
    ▼
FileIndex.loadFromFileListAsync(paths)
    │
    ├──► Build index in chunks (CHUNK_MS each)
    │    ├──► Deduplicate paths
    │    ├──► Build char bitmap for each path
    │    ├──► Store lowercase version
    │    └──► Yield to event loop between chunks
    │
    ▼
queryable Promise resolves (~5-10ms)
    │
    ▼
User types query
    │
    ▼
search(query, limit)
    │
    ├──► Score each path
    │    ├──► Character bitmap matching
    │    ├──► Apply bonuses (boundary, camel, consecutive)
    │    ├──► Apply penalties (gaps, test files)
    │    └──► Calculate final score
    │
    ├──► Sort by score (lower = better)
    └──► Return top N results
```

---

## 7. Key Patterns

### 7.1 API Compatibility Pattern

```typescript
// API matches vendor/color-diff-src/index.d.ts exactly so callers don't change
export type NativeModule = {
  ColorDiff: typeof ColorDiff
  ColorFile: typeof ColorFile
  getSyntaxTheme: (themeName: string) => SyntaxTheme
}
```

**Drop-In Replacement**: Same types, same methods, same behavior.

### 7.2 Time-Based Async Chunking

```typescript
const CHUNK_MS = 4

async buildAsync(fileList, markQueryable) {
  const startTime = Date.now()
  
  for (let i = 0; i < fileList.length; i++) {
    // Process path
    processPath(fileList[i])
    
    // Yield every CHUNK_MS
    if (i % 1000 === 0 && Date.now() - startTime > CHUNK_MS) {
      await Promise.resolve()  // Yield to event loop
      startTime = Date.now()
    }
  }
}
```

**Responsive Indexing**: Large indexes don't block main thread.

### 7.3 Character Bitmap Indexing

```typescript
// Reusable buffer: records where each needle char matched during indexOf scan
const posBuf = new Int32Array(MAX_QUERY_LEN)

// Character bitmap for fast lookup
private charBits: Int32Array = new Int32Array(0)

// For each character (a-z), store bitmask of positions where it appears
// Enables O(1) "does this char appear at position X?" queries
```

**Performance**: Bitmap enables fast character position lookups.

---

## 8. Environment Variables

| Variable | Purpose | Values |
|----------|---------|--------|
| `COLORTERM` | Color mode detection | `'truecolor'`, `'24bit'`, or empty |
| `BAT_THEME` | Syntax theme selection (stub) | Theme name |

---

## 9. Summary

The `native-ts/` module provides **pure TypeScript native module ports**:

1. **Color Diff** — Syntax-highlighted diff with ANSI output
2. **File Index** — High-performance fuzzy file search
3. **Yoga Layout** — Flexbox layout engine enums and implementation

**Key Design Decisions**:
- **API compatibility** with native Rust modules
- **Lazy loading** for heavy dependencies (highlight.js)
- **Time-based chunking** for responsive async indexing
- **Pure TypeScript** eliminates native dependency issues

**Performance Characteristics**:
- **Color Diff**: Lazy highlight.js (~50MB deferred)
- **File Index**: ~5-10ms to first results, ~4ms chunks
- **Yoga**: Pure JS layout calculations

**Use Cases**:
- Environments without native module support
- Development/testing without Rust toolchain
- Fallback when native modules fail to load

---

**Last Updated:** 2026-04-07  
**Status:** Complete — All 4 files analyzed
