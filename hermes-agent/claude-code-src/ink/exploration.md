# Ink Module — Comprehensive Deep-Dive Exploration

**Module:** `ink/`  
**Location:** `/home/darkvoid/Boxxed/@formulas/src.rust/src.llamacpp/src.ClaudOpen/claude-code-main/src/ink/`  
**Files:** 96 TypeScript/TSX files  
**Total Lines:** ~13,306 lines  
**Primary Purpose:** Terminal UI rendering framework built on React reconciler, Yoga layout engine, and ANSI escape sequence output

---

## 1. File Inventory

### Root Files (Core Engine)

| File | Lines | Key Exports | Description |
|------|-------|-------------|-------------|
| `ink.tsx` | 1,722 | `Ink` (default class) | Main Ink instance: render loop, frame management, terminal I/O, selection overlay |
| `reconciler.ts` | 512 | `dispatcher`, `createRenderer` | React reconciler configuration, commit instrumentation, scroll profiling |
| `renderer.ts` | 178 | `createRenderer` (default) | Converts DOM nodes to Output operations, cursor positioning, alt-screen handling |
| `output.ts` | 797 | `Output` (default class) | Operation collector (write/blit/clip/clear), Screen buffer mutator |
| `screen.ts` | 1,486 | `Screen`, `CharPool`, `StylePool`, `HyperlinkPool`, `setCellAt`, `blitRegion` | Screen buffer data structure, character/style interning pools, cell operations |
| `render-node-to-output.ts` | 1,462 | `renderNodeToOutput` (default), `getScrollHint`, `consumeFollowScroll` | Tree walker that converts DOM nodes to Output operations, scroll optimization |
| `render-to-screen.ts` | 231 | `applyPositionedHighlight`, `scanPositions` | Search highlight application at screen level |
| `dom.ts` | 484 | `createNode`, `appendChildNode`, `markDirty`, `DOMElement`, `TextNode` | Virtual DOM node creation, tree manipulation, dirty marking |
| `root.ts` | 184 | `renderSync`, `createRoot` | Public API for creating Ink roots (like react-dom's createRoot) |
| `instances.ts` | 10 | `instances` (Map) | Singleton instance registry by stdout stream |
| `frame.ts` | 124 | `Frame`, `FrameEvent`, `emptyFrame` | Frame buffer type definition, timing instrumentation |
| `constants.ts` | 2 | `FRAME_INTERVAL_MS` | Frame timing constant (16ms = ~60fps) |

### Layout System

| File | Lines | Key Exports | Description |
|------|-------|-------------|-------------|
| `layout/engine.ts` | 1 | `createLayoutNode` | Layout node factory (delegates to Yoga) |
| `layout/node.ts` | ~150 (types) | `LayoutNode`, `LayoutEdge`, `LayoutFlexDirection`, etc. | Yoga layout interface types and enums |
| `layout/geometry.ts` | — | `Rectangle`, `Point`, `Size`, `unionRect` | Geometric types and utilities |
| `layout/yoga.ts` | — | `createYogaLayoutNode` | Yoga WASM bindings implementation |

### Styles & Text

| File | Lines | Key Exports | Description |
|------|-------|-------------|-------------|
| `styles.ts` | 771 | `Styles`, `TextStyles`, `Color` types | Style type definitions, style application to Yoga nodes |
| `colorize.ts` | 231 | `applyTextStyles`, `styledCharsFromTokens` | Text styling with ANSI codes |
| `wrap-text.ts` | 74 | `wrapText` (default) | Text wrapping with truncate options |
| `squash-text-nodes.ts` | 92 | `squashTextNodesToSegments` | Combines adjacent text nodes for efficient rendering |
| `measure-text.ts` | 47 | `measureText` (default) | Text measurement for Yoga layout |
| `measure-element.ts` | 23 | `measureElement` | Element dimension measurement |
| `stringWidth.ts` | 222 | `stringWidth` | String width calculation (grapheme-aware) |
| `widest-line.ts` | 19 | `widestLine` | Find widest line in multiline string |
| `wrapAnsi.ts` | 20 | `wrapAnsi` | Wrap ANSI strings |
| `bidi.ts` | 139 | `reorderBidi` | Bidirectional text reordering |
| `tabstops.ts` | 46 | `expandTabs` | Tab expansion to spaces |

### Input & Events

| File | Lines | Key Exports | Description |
|------|-------|-------------|-------------|
| `parse-keypress.ts` | 801 | `parseKeypress`, `InputEvent`, `TerminalResponse` | Keyboard input parser, CSI sequence recognition |
| `events/input-event.ts` | 206 | `InputEvent`, `Key`, `parseKey` | Input event class, key flag extraction |
| `events/keyboard-event.ts` | — | `KeyboardEvent` | Keyboard event class |
| `events/click-event.ts` | — | `ClickEvent` | Mouse click event class |
| `events/focus-event.ts` | — | `FocusEvent` | Focus/blur event class |
| `events/event-handlers.ts` | 74 | `EVENT_HANDLER_PROPS`, `HANDLER_FOR_EVENT` | Event handler property definitions |
| `events/dispatcher.ts` | — | `Dispatcher` | Event capture/bubble dispatcher |
| `events/emitter.ts` | — | `EventEmitter` | Event emitter utility |
| `focus.ts` | 181 | `FocusManager`, `getFocusManager`, `getRootNode` | DOM-like focus management, tab order |

### Hooks (React)

| File | Lines | Key Exports | Description |
|------|-------|-------------|-------------|
| `hooks/use-app.ts` | 8 | `useApp` (default) | Access exit function from components |
| `hooks/use-input.ts` | 92 | `useInput` (default) | Handle keyboard input, raw mode management |
| `hooks/use-stdin.ts` | 8 | `useStdin` (default) | Access stdin stream and raw mode |
| `hooks/use-terminal-size.ts` | — | `useTerminalSize` | Terminal dimensions hook |
| `hooks/use-terminal-focus.ts` | — | `useTerminalFocus` | Terminal focus state hook |
| `hooks/use-terminal-title.ts` | — | `useTerminalTitle` | Set terminal title via OSC |
| `hooks/use-terminal-viewport.ts` | — | `useTerminalViewport` | Terminal viewport info |
| `hooks/use-declared-cursor.ts` | — | `useDeclaredCursor` | Declare cursor position for IME |
| `hooks/use-interval.ts` | — | `useInterval` | Interval timer hook |
| `hooks/use-animation-frame.ts` | — | `useAnimationFrame` | Animation frame hook |
| `hooks/use-selection.ts` | — | `useSelection`, `useHasSelection` | Text selection state hooks |
| `hooks/use-search-highlight.ts` | — | `useSearchHighlight` | Search highlight state hook |
| `hooks/use-tab-status.ts` | — | `useTabStatus` | Terminal tab status (iTerm2) |

### Components

| File | Lines | Key Exports | Description |
|------|-------|-------------|-------------|
| `components/App.tsx` | ~600 | `App` (default) | Root app component, event loop, focus management |
| `components/AppContext.ts` | 21 | `AppContext` (default) | Context for exit function |
| `components/Box.tsx` | 214 | `Box` (default) | Flexbox container component (like `<div style="display:flex">`) |
| `components/Text.tsx` | 254 | `Text` (default) | Styled text component with color, bold, italic, etc. |
| `components/AlternateScreen.tsx` | 80 | `AlternateScreen` | Alternate screen buffer wrapper (DEC 1049) |
| `components/TerminalSizeContext.tsx` | 7 | `TerminalSizeContext` | Terminal dimensions context |
| `components/TerminalFocusContext.tsx` | — | `TerminalFocusContext` | Terminal focus context |
| `components/StdinContext.ts` | — | `StdinContext` | Stdin stream context |
| `components/CursorDeclarationContext.ts` | — | `CursorDeclarationContext` | Cursor positioning context |
| `components/Button.tsx` | — | `Button` | Button component |
| `components/Link.tsx` | — | `Link` | Clickable link with OSC 8 hyperlinks |
| `components/ScrollBox.tsx` | — | `ScrollBox` | Scrollable container with overflow handling |
| `components/Spacer.tsx` | — | `Spacer` | Flexible spacing component |
| `components/Newline.tsx` | — | `Newline` | Explicit newline component |
| `components/NoSelect.tsx` | — | `NoSelect` | Mark region as non-selectable (gutters) |
| `components/RawAnsi.tsx` | — | `RawAnsi` | Raw ANSI escape sequence rendering |
| `components/Ansi.tsx` | 291 | `Ansi` | ANSI code parser and renderer |
| `components/ErrorOverview.tsx` | — | `ErrorOverview` | Error display component |
| `components/ClockContext.tsx` | — | `ClockContext` | Clock/time context for animations |

### Terminal I/O

| File | Lines | Key Exports | Description |
|------|-------|-------------|-------------|
| `terminal.ts` | 248 | `Terminal`, `writeDiffToTerminal`, `isXtermJs`, `SYNC_OUTPUT_SUPPORTED` | Terminal write operations, sync output (DEC 2026) |
| `terminal-querier.ts` | 212 | `TerminalQuerier` | Query terminal capabilities (XTVERSION, DA1, etc.) |
| `terminal-focus-state.ts` | 47 | `TerminalFocusState` | Terminal focus tracking |
| `log-update.ts` | 773 | `LogUpdate` | Main-screen output management, scroll optimization |
| `clearTerminal.ts` | 74 | `clearTerminal` | Terminal clear sequences |
| `supports-hyperlinks.ts` | 57 | `supportsHyperlinks` | Hyperlink capability detection |
| `termio.ts` | 42 | Various termio exports | Terminal I/O aggregation |
| `termio/ansi.ts` | — | `BEL`, `ESC`, `SEP` | ANSI constants |
| `termio/csi.ts` | — | `CURSOR_HOME`, `eraseLines`, `cursorMove` | CSI sequences |
| `termio/dec.ts` | — | `SHOW_CURSOR`, `HIDE_CURSOR`, `ENTER_ALT_SCREEN` | DEC private mode sequences |
| `termio/osc.ts` | — | `link`, `setClipboard`, `CLEAR_TAB_STATUS` | OSC sequences |
| `termio/parser.ts` | — | `createParser` | Terminal response parser |
| `termio/tokenize.ts` | — | `createTokenizer` | Input tokenizer for escape sequences |
| `termio/sgr.ts` | — | SGR utilities | SGR (Select Graphic Rendition) parsing |
| `termio/types.ts` | — | Termio types | Terminal I/O types |

### Selection & Highlighting

| File | Lines | Key Exports | Description |
|------|-------|-------------|-------------|
| `selection.ts` | 917 | `SelectionState`, `startSelection`, `extendSelection`, `getSelectedText`, `applySelectionOverlay` | Fullscreen text selection (alt-screen), word/line selection, scroll-off accumulation |
| `searchHighlight.ts` | 93 | `applySearchHighlight` | Search query highlighting (inverse cells) |
| `searchHighlight.ts` | 93 | `applySearchHighlight` | Search query highlighting |
| `hit-test.ts` | 130 | `dispatchClick`, `dispatchHover` | Mouse hit testing for click/hover events |

### Optimization & Caching

| File | Lines | Key Exports | Description |
|------|-------|-------------|-------------|
| `optimizer.ts` | 93 | `optimize` | Render optimization utilities |
| `node-cache.ts` | 54 | `nodeCache`, `pendingClears`, `consumeAbsoluteRemovedFlag` | Node caching for blit optimization, absolute position clears |
| `line-width-cache.ts` | 24 | Line width caching | Cache for line width calculations |
| `warn.ts` | 9 | Warning utilities | Development warnings |

### Utilities

| File | Lines | Key Exports | Description |
|------|-------|-------------|-------------|
| `get-max-width.ts` | 27 | `getMaxWidth` | Calculate max width of children |
| `useTerminalNotification.ts` | 126 | `TerminalWriteContext`, `TerminalWriteProvider` | Terminal write context provider |
| `colorize.ts` | 231 | `colorize` | Color utilities |

---

## 2. Module Overview

### Ink Framework Architecture

The `ink/` module implements a **React reconciler for terminal UIs**. It translates React component trees into ANSI escape sequences that render in terminal emulators. The architecture consists of:

```
┌─────────────────────────────────────────────────────────────────┐
│                     React Component Layer                        │
│  (App, Box, Text, ScrollBox, AlternateScreen, custom components) │
└─────────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│                   React Reconciler (react-reconciler)            │
│  - Converts React elements to DOM-like nodes (ink-box, ink-text) │
│  - Manages Fiber tree, commit phases, dirty marking              │
└─────────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│                      Virtual DOM (dom.ts)                        │
│  - DOMElement, TextNode with yogaNode attachments                │
│  - Tree structure with parentNode, childNodes                    │
└─────────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│                    Layout Engine (Yoga via WASM)                 │
│  - Flexbox layout calculation (layout/node.ts, layout/yoga.ts)   │
│  - Computed positions: getComputedLeft/Top/Width/Height          │
└─────────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│                  Render Pipeline (render-*.ts)                   │
│  - renderNodeToOutput: Tree walk → Output operations             │
│  - Output: Collects write, blit, clip, clear operations          │
│  - Screen: Cell buffer with CharPool, StylePool interning        │
└─────────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│                    Frame Diff & Terminal Write                   │
│  - ink.tsx: Front/back frame comparison                          │
│  - terminal.ts: ANSI escape sequence emission (CSI, DEC, OSC)    │
│  - log-update.ts: Main-screen scroll optimization                │
└─────────────────────────────────────────────────────────────────┘
```

### Terminal UI Rendering Architecture

**Double-Buffered Frame System:**
- **Front Frame**: Currently displayed screen buffer
- **Back Frame**: Next frame being rendered
- After render, frames are compared cell-by-cell; only changed cells emit ANSI sequences

**Screen Buffer Structure:**
```typescript
type Screen = {
  width: number
  height: number
  cells: Uint32Array      // Packed charId (20 bits) + styleId (11 bits) + width (2 bits)
  styleIds: Uint16Array   // Style IDs per cell
  hyperlinkIds: Uint16Array // Hyperlink IDs per cell
  noSelect: Uint8Array    // Non-selectable region flags
  softWrap: Uint32Array   // Soft-wrap boundary tracking
  damage: Rectangle | null // Damaged region for incremental diff
  charPool: CharPool      // String interning
  stylePool: StylePool    // Style interning
  hyperlinkPool: HyperlinkPool // Hyperlink interning
}
```

**Cell Packing (4 bytes per cell):**
```
Bits 0-19:   Character ID (into CharPool.strings)
Bits 20-30:  Style ID (into StylePool.styles)
Bits 31-32:  CellWidth enum (Narrow=1, Wide=2, SpacerHead=3, SpacerTail=4)
```

---

## 3. Core Ink Exports

### React-Reconciler Primitives

The module re-exports standard Ink hooks that map to terminal-specific functionality:

| Export | Type | Description |
|--------|------|-------------|
| `useApp()` | Hook | Get `exit()` function to unmount the app |
| `useInput(handler, options)` | Hook | Handle keyboard input with raw mode auto-management |
| `useStdin()` | Hook | Access stdin stream, `setRawMode()`, event emitter |
| `useTerminalSize()` | Hook | Get `{ columns, rows }` terminal dimensions |
| `useTerminalFocus()` | Hook | Subscribe to terminal focus changes |
| `useTerminalTitle()` | Hook | Set terminal title via OSC 2 |
| `useTerminalViewport()` | Hook | Get viewport scroll position |
| `useDeclaredCursor()` | Hook | Declare cursor position for IME/screen readers |
| `useSelection()` | Hook | Get selection state (alt-screen text selection) |
| `useHasSelection()` | Hook | Subscribe to selection changes |
| `useSearchHighlight()` | Hook | Set search query for highlight overlay |
| `useAnimationFrame()` | Hook | Request animation frame for smooth updates |
| `useInterval()` | Hook | Interval timer with cleanup |

### Ink Component Primitives

| Component | Description |
|-----------|-------------|
| `<Box>` | Flexbox container (`display: flex`), all layout props |
| `<Text>` | Styled text with color, bold, italic, underline, etc. |
| `<AlternateScreen>` | Switch to alternate screen buffer (fullscreen mode) |
| `<ScrollBox>` | Scrollable container with `overflow: scroll/hidden` |
| `<Link>` | OSC 8 hyperlink wrapper |
| `<NoSelect>` | Mark region as non-selectable (gutters, sigils) |
| `<RawAnsi>` | Render raw ANSI escape sequences |
| `<Button>` | Interactive button component |
| `<Spacer>` | Flexible spacing element |
| `<Newline>` | Explicit line break |

---

## 4. Custom Ink Components

### Terminal-Specific Components

#### `<AlternateScreen>` (AlternateScreen.tsx)

```typescript
type Props = {
  mouseTracking?: boolean  // Default: true
  children: ReactNode
}
```

**Purpose:** Enter the terminal's alternate screen buffer (DEC 1049), constraining content to viewport height.

**Implementation Details:**
```typescript
export function AlternateScreen({ children, mouseTracking = true }: Props) {
  const size = useContext(TerminalSizeContext)  // { columns, rows }
  const writeRaw = useContext(TerminalWriteContext)
  
  useInsertionEffect(() => {
    const ink = instances.get(process.stdout)
    // Enter alt-screen, clear, home cursor, optionally enable mouse tracking
    writeRaw(
      ENTER_ALT_SCREEN + '\x1b[2J\x1b[H' + 
      (mouseTracking ? ENABLE_MOUSE_TRACKING : '')
    )
    ink?.setAltScreenActive(true, mouseTracking)
    
    return () => {
      ink?.setAltScreenActive(false)
      ink?.clearTextSelection()
      writeRaw(
        (mouseTracking ? DISABLE_MOUSE_TRACKING : '') + EXIT_ALT_SCREEN
      )
    }
  }, [writeRaw, mouseTracking])
  
  // Constrain height to terminal rows, force overflow handling via flexbox
  return (
    <Box flexDirection="column" height={size?.rows ?? 24} width="100%" flexShrink={0}>
      {children}
    </Box>
  )
}
```

**Key Behaviors:**
- Uses `useInsertionEffect` (not `useLayoutEffect`) to fire BEFORE the reconciler's `resetAfterCommit` — ensures alt-screen escape reaches terminal before first frame
- Notifies `Ink` instance via `setAltScreenActive()` so renderer clamps cursor.y inside viewport (prevents cursor-restore LF from scrolling)
- On unmount: exits alt-screen, restores main screen content (preserved by alternate buffer)
- Mouse tracking enables wheel/click-drag events for selection

#### `<NoSelect>` (NoSelect.tsx)

```typescript
type Props = {
  children: ReactNode
}
```

**Purpose:** Mark a region as non-selectable — excluded from fullscreen text selection copy and highlight.

**Use Cases:**
- Line number gutters
- Diff sigils (+/- markers)
- UI chrome that shouldn't be copied

**Implementation:** Emits `noSelect` operation to Output, which sets `screen.noSelect` bitmap AFTER all blit/write operations (so it wins over any blitted content).

#### `<RawAnsi>` (RawAnsi.tsx)

**Purpose:** Render pre-formatted ANSI escape sequences directly.

**Use Cases:**
- Syntax-highlighted code from external tools
- Pre-styled strings from libraries like `chalk`

---

### Input Handling Components

#### `useInput` Hook Deep Dive

```typescript
function useInput(
  inputHandler: (input: string, key: Key, event: InputEvent) => void,
  options: { isActive?: boolean } = {}
) {
  const { setRawMode, internal_exitOnCtrlC, internal_eventEmitter } = useStdin()
  
  // Enable raw mode synchronously during commit phase
  useLayoutEffect(() => {
    if (options.isActive === false) return
    setRawMode(true)
    return () => setRawMode(false)
  }, [options.isActive, setRawMode])
  
  // Stable listener registration (useEventCallback keeps ref stable)
  const handleData = useEventCallback((event: InputEvent) => {
    if (options.isActive === false) return
    const { input, key } = event
    
    // Ctrl+C handling: defer to app unless exitOnCtrlC=false
    if (!(input === 'c' && key.ctrl) || !internal_exitOnCtrlC) {
      inputHandler(input, key, event)
    }
  })
  
  useEffect(() => {
    internal_eventEmitter?.on('input', handleData)
    return () => internal_eventEmitter?.removeListener('input', handleData)
  }, [internal_eventEmitter, handleData])
}
```

**Key Design Decisions:**

1. **`useLayoutEffect` for raw mode**: Ensures terminal is in raw mode BEFORE first render — without this, keystrokes echo and cursor is visible until effect fires (next event loop tick).

2. **`useEventCallback` for stable listener**: Keeps handler reference stable across re-renders, preventing re-append on `isActive` toggle. This preserves `stopImmediatePropagation()` ordering when multiple `useInput` hooks compete.

3. **Ctrl+C handling**: Respects `exitOnCtrlC` from stdin — if true, Ctrl+C exits app without reaching input handler.

---

### Layout Utilities

#### `<Box>` Component (Box.tsx)

```typescript
type Props = Except<Styles, 'textWrap'> & {
  ref?: Ref<DOMElement>
  tabIndex?: number          // Tab order for focus cycling
  autoFocus?: boolean        // Auto-focus on mount
  onClick?: (e: ClickEvent) => void
  onFocus?: (e: FocusEvent) => void
  onBlur?: (e: FocusEvent) => void
  onKeyDown?: (e: KeyboardEvent) => void
  onMouseEnter?: () => void  // Only in <AlternateScreen>
  onMouseLeave?: () => void
  // Plus all flexbox styles: flexDirection, flexGrow, flexShrink, 
  // flexWrap, margin*, padding*, gap*, position, overflow, etc.
}
```

**Implementation highlights:**
- Validates integer spacing props (warns on fractional margins/padding)
- Defaults: `flexWrap='nowrap'`, `flexDirection='row'`, `flexGrow=0`, `flexShrink=1`
- Resolves `overflowX/Y` from `overflow` shorthand
- Creates `ink-box` host element with yogaNode for layout

#### Text Styling (`Text.tsx`)

```typescript
type Props = {
  color?: Color              // rgb(), #hex, ansi256(n), ansi:*
  backgroundColor?: Color
  bold?: boolean
  dim?: boolean              // Mutually exclusive with bold
  italic?: boolean
  underline?: boolean
  strikethrough?: boolean
  inverse?: boolean
  wrap?: Styles['textWrap']  // 'wrap' | 'truncate-*' | 'middle' | 'end'
  children?: ReactNode
}
```

**Memoized styles per wrap mode:**
```typescript
const memoizedStylesForWrap: Record<Styles['textWrap'], Styles> = {
  wrap: { flexGrow: 0, flexShrink: 1, flexDirection: 'row', textWrap: 'wrap' },
  'truncate-end': { flexGrow: 0, flexShrink: 1, textWrap: 'truncate-end' },
  // ... etc
}
```

**Type safety:** Bold and dim are mutually exclusive via union type:
```typescript
type WeightProps =
  | { bold?: never; dim?: never }
  | { bold: boolean; dim?: never }
  | { dim: boolean; bold?: never }
```

---

## 5. Line-by-Line Analysis of Critical Files

### `ink.tsx` — Main Render Loop

**Constructor setup (lines 180-250):**
```typescript
constructor(private readonly options: Options) {
  autoBind(this)
  
  // Patch console.log to write above Ink output (main-screen only)
  if (this.options.patchConsole) {
    this.restoreConsole = this.patchConsole()
    this.restoreStderr = this.patchStderr()
  }
  
  this.terminal = { stdout: options.stdout, stderr: options.stderr }
  this.terminalColumns = options.stdout.columns || 80
  this.terminalRows = options.stdout.rows || 24
  this.altScreenParkPatch = makeAltScreenParkPatch(this.terminalRows)
  
  // Initialize interning pools
  this.stylePool = new StylePool()
  this.charPool = new CharPool()
  this.hyperlinkPool = new HyperlinkPool()
  
  // Create front/back frames (double-buffering)
  this.frontFrame = emptyFrame(...)
  this.backFrame = emptyFrame(...)
  
  // LogUpdate manages main-screen output (console.log, Static)
  this.log = new LogUpdate({
    isTTY: options.stdout.isTTY,
    stylePool: this.stylePool,
    // ...
  })
  
  // Focus manager: tracks activeElement, tab order
  this.focusManager = new FocusManager(this.dispatchFocusEvent)
  
  // Create root DOM node for reconciler
  this.rootNode = dom.createNode('ink-root')
  this.rootNode.focusManager = this.focusManager
  
  // Renderer converts DOM → Output operations
  this.renderer = createRenderer(this.rootNode, this.stylePool)
  
  // React concurrent root
  this.container = reconciler.createContainer(
    this.rootNode,
    ConcurrentRoot,
    null,
    false,
    null,
    'id',
    throwOnUncaughtError,
    onRecoverableError
  )
  
  // Schedule render on animation frames
  this.scheduleRender = throttle(this.render, FRAME_INTERVAL_MS)
  
  // Setup input handling
  this.setupInputHandlers(options.stdin)
  
  // Handle resize, SIGCONT, SIGWINCH
  this.setupTerminalHandlers(options.stdout)
}
```

**The `render()` method (core loop):**
```typescript
private render = () => {
  if (this.isUnmounted) return
  
  const time = performance.now()
  const yogaStart = time
  
  // 1. Calculate layout (Yoga flexbox)
  this.rootNode.yogaNode!.calculateLayout(
    this.terminalColumns,
    undefined,
    LayoutDirection.LTR
  )
  recordYogaMs(performance.now() - yogaStart)
  
  // 2. Render to back frame
  const frame = this.renderer({
    frontFrame: this.frontFrame,
    backFrame: this.backFrame,
    isTTY: this.options.stdout.isTTY,
    terminalWidth: this.terminalColumns,
    terminalRows: this.terminalRows,
    altScreen: this.altScreenActive,
    prevFrameContaminated: this.prevFrameContaminated
  })
  
  // 3. Apply overlays (selection, search highlight)
  if (this.altScreenActive) {
    if (hasSelection(this.selection)) {
      const captured = captureScrolledRows(this.selection, frame.screen)
      applySelectionOverlay(frame.screen, this.selection, captured, this.stylePool)
    }
    if (this.searchHighlightQuery) {
      applySearchHighlight(frame.screen, this.searchHighlightQuery, this.stylePool)
    }
  }
  
  // 4. Diff front vs back frame, write patches to terminal
  const patches = this.diffAndWrite(frame)
  
  // 5. Swap frames
  this.frontFrame = frame
  this.prevFrameContaminated = false
  
  // 6. Emit frame event for instrumentation
  this.options.onFrame?.({
    type: 'frame',
    time: performance.now() - time,
    yogaMs: getLastYogaMs(),
    patches
  })
}
```

**Key observations:**

1. **Yoga layout runs first** — All computed positions (getComputedTop/Left) are valid before rendering
2. **Back frame rendering** — Output operations collected, then `get()` materializes Screen
3. **Overlay pass** — Selection/search highlight mutate screen AFTER normal render (z-order on top)
4. **Incremental diff** — Only changed cells between front/back frames emit ANSI
5. **Frame swap** — Back becomes front for next diff

---

### `output.ts` — Operation Collector

**Operation types:**
```typescript
type Operation =
  | WriteOperation      // Write text at (x,y)
  | ClipOperation       // Push clipping region
  | UnclipOperation     // Pop clipping region
  | BlitOperation       // Block transfer from source screen
  | ClearOperation      // Clear region (write empty cells)
  | NoSelectOperation   // Mark non-selectable region
  | ShiftOperation      // Scroll rows (DECSTBM optimization)
```

**The `get()` method — applying operations to Screen:**

```typescript
get(): Screen {
  const screen = this.screen
  
  // Pass 1: Expand damage to cover clear regions
  // Absolute-positioned node clears need special handling (see below)
  const absoluteClears: Rectangle[] = []
  for (const op of this.operations) {
    if (op.type !== 'clear') continue
    const rect = intersectWithScreen(op.region)
    screen.damage = unionRect(screen.damage, rect)
    if (op.fromAbsolute) absoluteClears.push(rect)
  }
  
  // Pass 2: Apply operations in tree order
  const clips: Clip[] = []
  for (const op of this.operations) {
    switch (op.type) {
      case 'clip':
        clips.push(intersectClip(clips.at(-1), op.clip))
        continue
      case 'unclip':
        clips.pop()
        continue
      case 'blit': {
        // Intersect with active clip
        const clip = clips.at(-1)
        const startX = max(regionX, clip?.x1 ?? 0)
        const maxY = min(regionY + regionHeight, clip?.y2 ?? Infinity)
        
        // Skip rows covered by absolute clears (prevent ghost artifacts)
        if (absoluteClears.length > 0) {
          // Split blit into segments around absolute clears
          blitInSegments()
        } else {
          blitRegion(screen, src, startX, startY, maxX, maxY)
        }
        continue
      }
      case 'write': {
        // Split into lines, apply clip, write character by character
        const lines = op.text.split('\n')
        let offsetY = 0
        for (const line of lines) {
          const lineY = op.y + offsetY
          if (lineY >= screenHeight) break
          const contentEnd = writeLineToScreen(screen, line, op.x, lineY, ...)
          // Record soft-wrap boundary if applicable
          if (op.softWrap) {
            screen.softWrap[lineY] = op.softWrap[offsetY] ? prevContentEnd : 0
          }
          offsetY++
        }
        continue
      }
    }
  }
  
  // Pass 3: noSelect applies LAST (wins over blit/write)
  for (const op of this.operations) {
    if (op.type === 'noSelect') {
      markNoSelectRegion(screen, op.region.x, op.region.y, ...)
    }
  }
  
  return screen
}
```

**Critical insight — Absolute clear handling:**

When an `position: absolute` node shrinks, its clear operation comes AFTER normal-flow siblings' blits (DOM order). Without special handling:
1. Sibling blits its clean subtree (copies from prevScreen)
2. Absolute node clears its old bounds
3. Next frame: sibling blits again, copying absolute node's stale paint from prevScreen

Solution: Track absolute clears, skip those rows during blit, force full re-render of affected region.

---

### `screen.ts` — Screen Buffer & Interning Pools

**CharPool — ASCII fast-path:**
```typescript
export class CharPool {
  private strings: string[] = [' ', '']  // 0=space, 1=empty (spacer)
  private stringMap = new Map<string, number>()
  private ascii: Int32Array = initCharAscii()  // code → index, -1=not interned
  
  intern(char: string): number {
    // ASCII fast-path: direct array lookup (no Map.get)
    if (char.length === 1) {
      const code = char.charCodeAt(0)
      if (code < 128) {
        const cached = this.ascii[code]!
        if (cached !== -1) return cached
        const index = this.strings.length
        this.strings.push(char)
        this.ascii[code] = index
        return index
      }
    }
    // Fallback: Map lookup/insert
    const existing = this.stringMap.get(char)
    if (existing !== undefined) return existing
    const index = this.strings.length
    this.strings.push(char)
    this.stringMap.set(char, index)
    return index
  }
}
```

**StylePool — Bit-0 visibility flag:**
```typescript
export class StylePool {
  private ids = new Map<string, number>()
  private styles: AnsiCode[][] = []
  private transitionCache = new Map<number, string>()
  readonly none: number  // Pre-interned empty style
  
  intern(styles: AnsiCode[]): number {
    const key = styles.length === 0 ? '' : styles.map(s => s.code).join('\0')
    let id = this.ids.get(key)
    if (id === undefined) {
      const rawId = this.styles.length
      this.styles.push(styles)
      // Bit 0 encodes visibility on space characters
      // Even = foreground-only (invisible on space)
      // Odd = has background/inverse/underline (visible on space)
      id = (rawId << 1) | (hasVisibleSpaceEffect(styles) ? 1 : 0)
      this.ids.set(key, id)
    }
    return id
  }
  
  get(id: number): AnsiCode[] {
    return this.styles[id >>> 1] ?? []  // Strip bit 0
  }
  
  transition(fromId: number, toId: number): string {
    if (fromId === toId) return ''
    const key = fromId * 0x100000 + toId
    let str = this.transitionCache.get(key)
    if (str === undefined) {
      str = ansiCodesToString(diffAnsiCodes(this.get(fromId), this.get(toId)))
      this.transitionCache.set(key, str)
    }
    return str
  }
  
  withInverse(baseId: number): number {
    // Cache for selection overlay (base + inverse)
    let id = this.inverseCache.get(baseId)
    if (id === undefined) {
      const baseCodes = this.get(baseId)
      const hasInverse = baseCodes.some(c => c.endCode === '\x1b[27m')
      id = hasInverse ? baseId : this.intern([...baseCodes, INVERSE_CODE])
      this.inverseCache.set(baseId, id)
    }
    return id
  }
  
  withCurrentMatch(baseId: number): number {
    // Cache for current search match (inverse + bold + yellow-bg)
    // Yellow via fg-then-inverse swap (terminal-dependent)
    let id = this.currentMatchCache.get(baseId)
    if (id === undefined) {
      const baseCodes = this.get(baseId)
      // Filter existing fg/bg to avoid color clashes
      const filtered = baseCodes.filter(c => !isFgOrBg(c))
      id = this.intern([
        ...filtered,
        YELLOW_FG_CODE,
        INVERSE_CODE,
        BOLD_CODE
      ])
      this.currentMatchCache.set(baseId, id)
    }
    return id
  }
}
```

**Cell packing — 4 bytes per cell:**
```typescript
// Packed cell word (Uint32)
// Bits 0-19:   charId (20 bits = 1M unique chars)
// Bits 20-30:  styleId (11 bits = 2K unique styles)
// Bits 31-32:  CellWidth (2 bits: Narrow=1, Wide=2, SpacerHead=3, SpacerTail=4)

function packCell(charId: number, styleId: number, width: CellWidth): number {
  return (charId & 0xFFFFF) | ((styleId & 0x7FF) << 20) | ((width & 0x3) << 31)
}

function unpackCell(word: number): { charId: number; styleId: number; width: number } {
  return {
    charId: word & 0xFFFFF,
    styleId: (word >>> 20) & 0x7FF,
    width: (word >>> 31) & 0x3
  }
}
```

---

### `selection.ts` — Fullscreen Text Selection

**Selection state structure:**
```typescript
type SelectionState = {
  anchor: Point | null       // Mouse-down position
  focus: Point | null        // Current drag position
  isDragging: boolean        // Between mouse-down/up
  anchorSpan: { lo: Point; hi: Point; kind: 'word' | 'line' } | null
  scrolledOffAbove: string[]     // Text scrolled out above viewport
  scrolledOffBelow: string[]     // Text scrolled out below viewport
  scrolledOffAboveSW: boolean[]  // Soft-wrap bits for above
  scrolledOffBelowSW: boolean[]  // Soft-wrap bits for below
  virtualAnchorRow?: number      // Pre-clamp anchor (scroll restore)
  virtualFocusRow?: number       // Pre-clamp focus
  lastPressHadAlt: boolean       // Alt modifier on mouse-down
}
```

**Word selection logic:**
```typescript
const WORD_CHAR = /[\p{L}\p{N}_/.\-+~\\]/u  // Unicode letters, digits, punctuation

function charClass(c: string): 0 | 1 | 2 {
  if (c === ' ' || c === '') return 0
  if (WORD_CHAR.test(c)) return 1
  return 2
}

function wordBoundsAt(screen: Screen, col: number, row: number) {
  // Find same-class character run at click position
  // Expand left/right until class changes or noSelect boundary
  // Handles wide chars (step over SpacerTail to head)
}

export function selectWordAt(s: SelectionState, screen: Screen, col: number, row: number) {
  const bounds = wordBoundsAt(screen, col, row)
  if (!bounds) return
  s.anchor = { col: bounds.lo, row }
  s.focus = { col: bounds.hi, row }
  s.anchorSpan = { lo: { col: bounds.lo, row }, hi: { col: bounds.hi, row }, kind: 'word' }
}
```

**Extending selection (shift+click, drag):**
```typescript
export function extendSelection(
  s: SelectionState,
  screen: Screen,
  col: number,
  row: number,
  mode: 'char' | 'word' | 'line'
) {
  if (mode === 'word') {
    // Find word bounds at current mouse position
    const bounds = wordBoundsAt(screen, col, row)
    if (!bounds) return
    // Extend from anchorSpan.lo to current word.hi (or vice versa)
    s.focus = { col: bounds.hi, row }
  } else if (mode === 'line') {
    // Full line at current row
    s.focus = { col: screen.width - 1, row }
  } else {
    // Character mode
    s.focus = { col, row }
  }
}
```

**Scroll-off accumulation:**
```typescript
export function captureScrolledRows(
  selection: SelectionState,
  screen: Screen
): { above: string[]; below: string[]; aboveSW: boolean[]; belowSW: boolean[] } {
  // When selection.anchor.row < visible top, text has scrolled off above
  // Extract those rows from the scrollback (stored separately)
  // Same for below when anchor.row > visible bottom
}

export function getSelectedText(
  s: SelectionState,
  screen: Screen,
  captured: CapturedRows
): string {
  // Normalize anchor/focus (swap if focus < anchor)
  // Collect rows from anchor.row to focus.row:
  // - Use scrolledOffAbove for rows above viewport
  // - Use screen for visible rows
  // - Use scrolledOffBelow for rows below viewport
  // - Join with soft-wrap bits (no \n if softWrap[i]=true)
}
```

**Selection overlay rendering:**
```typescript
export function applySelectionOverlay(
  screen: Screen,
  selection: SelectionState,
  captured: CapturedRows,
  stylePool: StylePool
) {
  if (!hasSelection(selection)) return
  
  const bounds = getSelectionBounds(selection)
  for (let row = bounds.startRow; row <= bounds.endRow; row++) {
    const startCol = row === bounds.startRow ? bounds.startCol : 0
    const endCol = row === bounds.endRow ? bounds.endCol : screen.width - 1
    
    for (let col = startCol; col <= endCol; col++) {
      const cell = cellAt(screen, col, row)
      if (!cell) continue
      
      // Inverse the style (selection highlight)
      const newStyleId = stylePool.withInverse(cell.styleId)
      setCellStyleId(screen, col, row, newStyleId)
    }
  }
}
```

---

## 6. Rendering Pipeline

### Complete Render Flow

```
┌──────────────────────────────────────────────────────────────────────┐
│ 1. React Component Render                                            │
│    - User updates state (setState, hooks)                            │
│    - React schedules re-render                                       │
│    - Component tree returns new React elements                       │
└──────────────────────────────────────────────────────────────────────┘
                                    │
                                    ▼
┌──────────────────────────────────────────────────────────────────────┐
│ 2. Reconciler Commit Phase                                           │
│    - render() → beginWork() → completeWork()                         │
│    - createInstance/updateInstance for each element                  │
│    - DOM nodes created/updated (ink-box, ink-text)                   │
│    - Styles applied to yogaNodes                                     │
│    - markDirty() on ancestors                                        │
└──────────────────────────────────────────────────────────────────────┘
                                    │
                                    ▼
┌──────────────────────────────────────────────────────────────────────┐
│ 3. Layout Calculation (Yoga)                                         │
│    - rootNode.yogaNode.calculateLayout(terminalWidth)                │
│    - Flexbox algorithm: main axis → cross axis                       │
│    - Computed positions available:                                   │
│      getComputedLeft(), getComputedTop(),                            │
│      getComputedWidth(), getComputedHeight()                         │
│    - Timing tracked via getLastYogaMs()                              │
└──────────────────────────────────────────────────────────────────────┘
                                    │
                                    ▼
┌──────────────────────────────────────────────────────────────────────┐
│ 4. Tree Walk (renderNodeToOutput)                                    │
│    - DFS from root node                                              │
│    - For each node:                                                  │
│      - Compute absolute position (x, y) from Yoga                    │
│      - Apply clip (overflow: hidden)                                 │
│      - Render children                                               │
│      - Emit operations:                                              │
│        - write() for text content                                    │
│        - blit() for sub-buffers (optimized repaint)                  │
│        - clear() for removed content                                 │
│        - clip()/unclip() for overflow                                │
│        - noSelect() for gutters                                      │
│        - shift() for scroll (DECSTBM optimization)                   │
└──────────────────────────────────────────────────────────────────────┘
                                    │
                                    ▼
┌──────────────────────────────────────────────────────────────────────┐
│ 5. Output Materialization                                            │
│    - output.get() applies operations to Screen                       │
│    - Pass 1: Expand damage for clears                                │
│    - Pass 2: Apply write/blit/clip in order                          │
│    - Pass 3: Apply noSelect (wins over all)                          │
│    - Character interning (charCache):                                │
│      - tokenize() → grapheme clustering → style interning            │
│      - Cached across frames (most lines unchanged)                   │
│    - Cell packing: 4 bytes (charId:20 + styleId:11 + width:2)        │
└──────────────────────────────────────────────────────────────────────┘
                                    │
                                    ▼
┌──────────────────────────────────────────────────────────────────────┐
│ 6. Overlay Pass                                                      │
│    - applySelectionOverlay(): inverse selected cells                 │
│    - applySearchHighlight(): invert matching cells                   │
│    - applyPositionedHighlight(): current search match (yellow+bold)  │
│    - Mutates screen in-place (z-order on top)                        │
└──────────────────────────────────────────────────────────────────────┘
                                    │
                                    ▼
┌──────────────────────────────────────────────────────────────────────┐
│ 7. Frame Diff                                                        │
│    - Compare backFrame.screen vs frontFrame.screen                   │
│    - Cell-by-cell comparison (packed word compare)                   │
│    - Generate patches:                                               │
│      - Style transitions: stylePool.transition(fromId, toId)         │
│      - Character writes: charPool.get(charId)                        │
│      - Cursor moves: cursorMove(dx, dy)                              │
│      - Hyperlinks: OSC 8 sequences                                   │
│    - Damage tracking: only diff damaged regions (optimization)       │
└──────────────────────────────────────────────────────────────────────┘
                                    │
                                    ▼
┌──────────────────────────────────────────────────────────────────────┐
│ 8. Terminal Write                                                    │
│    - Synchronized output (DEC 2026) if supported:                    │
│      - BSU (Begin Synchronized Update)                               │
│      - Write all patches                                             │
│      - ESU (End Synchronized Update)                                 │
│    - Patches grouped by type:                                        │
│      - stdout: cursor moves, text, SGR                               │
│      - stderr: separate stream                                       │
│    - Main-screen only: log-update manages scrollback                 │
│    - Alt-screen: direct cursor positioning                           │
└──────────────────────────────────────────────────────────────────────┘
                                    │
                                    ▼
┌──────────────────────────────────────────────────────────────────────┐
│ 9. Frame Swap                                                        │
│    - frontFrame = backFrame                                          │
│    - backFrame = recycled or new                                     │
│    - prevFrameContaminated = false (unless selection mutated)        │
│    - Clear node caches for next frame                                │
└──────────────────────────────────────────────────────────────────────┘
```

### DECSTBM Scroll Optimization

When a ScrollBox's scrollTop changes and nothing else moved:

```typescript
// renderNodeToOutput.ts
if (didScroll && !layoutShifted) {
  // Emit scroll hint instead of full re-render
  scrollHint = {
    top: viewportTop,
    bottom: viewportBottom,
    delta: newScrollTop - oldScrollTop
  }
  
  // log-update.ts can emit:
  // CSI top ; bottom r  (set scroll region)
  // CSI n S             (scroll n lines)
  // Instead of rewriting entire viewport
}
```

This reduces scroll from O(rows × cols) to O(1) escape sequences plus damaged lines at insertion/removal points.

---

## 7. Key Patterns

### Terminal-Specific Patterns

#### 1. Raw Mode Management

```typescript
// useInput.ts — raw mode tied to hook lifecycle
useLayoutEffect(() => {
  if (options.isActive === false) return
  setRawMode(true)
  return () => setRawMode(false)
}, [options.isActive, setRawMode])
```

**Why `useLayoutEffect`?** Raw mode must be enabled synchronously during commit phase, before render returns. Deferred effect (useEffect) leaves terminal in cooked mode — keystrokes echo, cursor visible.

#### 2. Alt-Screen Cursor Clamping

```typescript
// renderer.ts
cursor: {
  x: 0,
  y: altScreen
    ? Math.max(0, Math.min(screen.height, terminalRows) - 1)
    : screen.height,
  visible: !isTTY || screen.height === 0
}
```

**Why clamp?** Alt-screen cursor at `screen.height === terminalRows` would trigger log-update's cursor-restore LF, scrolling content off top of alt buffer. Clamp keeps cursor in viewport.

#### 3. Absolute Position Clear Handling

```typescript
// output.ts — Pass 1
const absoluteClears: Rectangle[] = []
for (const op of operations) {
  if (op.type === 'clear' && op.fromAbsolute) {
    absoluteClears.push(op.region)
  }
}

// Pass 2 — Skip blit rows covered by absolute clears
if (absoluteClears.length > 0) {
  blitInSegments(around: absoluteClears)
}
```

**Why?** Absolute nodes overlay normal-flow siblings. Stale paint from prevScreen would ghost through sibling blits without this fence.

#### 4. CharCache Retention Across Frames

```typescript
// output.ts
reset(width, height, screen): void {
  this.operations.length = 0  // Clear operations
  resetScreen(screen, width, height)
  if (this.charCache.size > 16384) this.charCache.clear()  // Cap growth
  // Otherwise retain — most lines unchanged between frames
}
```

**Benefit:** Tokenize + grapheme clustering is expensive. Caching unchanged lines saves CPU on steady-state frames (spinner tick, clock update).

---

### Input Handling Patterns

#### 1. CSI u (Kitty Keyboard Protocol) Parsing

```typescript
// parse-keypress.ts
const CSI_U_RE = /^\x1b\[(\d+)(?:;(\d+))?u/

// ESC[13;2u = Shift+Enter (codepoint=13, modifier=2)
// ESC[27u = Escape (codepoint=27, no modifier)
if (/^\[\d/.test(input) && input.endsWith('u')) {
  if (!keypress.name) {
    input = ''  // Unmapped keycode (Caps Lock 57358, F13-35)
  } else {
    input = keypress.name === 'space' ? ' '
          : keypress.name === 'escape' ? ''
          : keypress.name
  }
  processedAsSpecialSequence = true
}
```

**Why special handling?** CSI u carries Unicode codepoints, not legacy key codes. Space (32) must become `' '`, not `'space'`.

#### 2. Bracketed Paste Detection

```typescript
// parse-keypress.ts
const PASTE_START = '\x1b[200~'
const PASTE_END = '\x1b[201~'

function* tokenize(input: string): Generator<ParsedKey | TerminalResponse> {
  if (input.startsWith(PASTE_START)) {
    const endIndex = input.indexOf(PASTE_END)
    if (endIndex === -1) {
      // Buffer incomplete paste
      buffer += input
      return
    }
    const pasteContent = input.slice(PASTE_START.length, endIndex)
    yield createPasteKey(pasteContent)
  }
}
```

**Why special?** Pasted text may contain newlines, escapes that shouldn't be interpreted as keys.

---

### Dynamic Resizing Patterns

#### 1. Resize Handler

```typescript
// ink.tsx
private handleResize = () => {
  const columns = this.options.stdout.columns
  const rows = this.options.stdout.rows
  
  if (columns === this.terminalColumns && rows === this.terminalRows) {
    return  // Debounce: ignore if unchanged
  }
  
  this.needsEraseBeforePaint = true  // Prepend CSI 2J inside BSU/ESU
  this.terminalColumns = columns
  this.terminalRows = rows
  this.altScreenParkPatch = makeAltScreenParkPatch(rows)
  
  // Mark root dirty — layout will recompute
  markDirty(this.rootNode)
  
  // Schedule render (throttled)
  this.scheduleRender()
}
```

#### 2. Alt-Screen Resize Handling

```typescript
// ink.tsx — inside handleResize
if (this.altScreenActive) {
  // Alt-screen content is exactly rows tall
  // Resize = erase + repaint, no scrollback preservation
  this.frontFrame = emptyFrame(cols, rows, ...)
  this.backFrame = emptyFrame(cols, rows, ...)
  this.prevFrameContaminated = true  // Force full render
}
```

**Why full render?** Alt-screen buffer size changes — old frame dimensions invalid.

---

## 8. Integration Points

### `ink/` ↔ `components/`

**Components that depend on Ink internals:**

| Component | Ink Dependency |
|-----------|----------------|
| `<AlternateScreen>` | `instances.get()`, `setAltScreenActive()`, DEC escape sequences |
| `<ScrollBox>` | `scrollTop`, `pendingScrollDelta`, `scrollHint` (DECSTBM) |
| `<NoSelect>` | `output.noSelect()` operation |
| `<Link>` | `OSC 8` hyperlink sequences (termio/osc.ts) |
| `useInput` | `useStdin()`, raw mode, event emitter |
| `useSelection` | `Ink.selection` state, `hasSelection()`, `getSelectedText()` |

**How integration works:**
- Components access Ink instance via `instances.get(process.stdout)`
- Context providers (`TerminalSizeContext`, `TerminalWriteContext`) wired in `App.tsx`
- Event handlers (`onClick`, `onFocus`) dispatched via `FocusManager` and `Dispatcher`

---

### `ink/` ↔ `keybindings/`

**Event flow:**

```
Terminal stdin
     │
     ▼
parse-keypress.ts (tokenize escape sequences)
     │
     ▼
InputEvent (key: Key flags, input: string)
     │
     ▼
App.tsx (stdin listener)
     │
     ▼
dispatcher.emit('input', event)
     │
     ├─► useInput listeners (component handlers)
     └─► keybindings/ (global shortcuts)
```

**Keybindings integration:**
```typescript
// App.tsx
private handleKeydown = (event: KeyboardEvent) => {
  // 1. Dispatch to focused node first (DOM-like)
  if (this.focusManager.activeElement) {
    const handled = this.dispatchFocusEvent(
      this.focusManager.activeElement,
      event
    )
    if (handled) return
  }
  
  // 2. Global keybindings (if not handled)
  const binding = this.keybindings.match(event)
  if (binding) {
    binding.handler()
    event.preventDefault()
  }
}
```

---

### `ink/` ↔ `screens/`

**Screen types:**

| Screen Type | Ink Mode | Description |
|-------------|----------|-------------|
| `MainScreen` | Main (default) | Scrollback preserved, `log-update` manages output |
| `AlternateScreen` | Alt (DEC 1049) | Fullscreen, no scrollback, mouse tracking enabled |

**Integration points:**

1. **Main screen (`log-update.ts`):**
   - Console.log interception (patchConsole)
   - Static component output (append to scrollback)
   - Cursor-restore LF after output

2. **Alt screen (`Ink.selection`, `Ink.searchHighlight`):**
   - Text selection overlay
   - Search highlight overlay
   - Mouse tracking (SGR mode 1006)

**State sharing:**
```typescript
// App.tsx — passed to Ink constructor
const ink = new Ink({
  stdout: process.stdout,
  stdin: process.stdin,
  stderr: process.stderr,
  exitOnCtrlC: true,
  patchConsole: true,
  onFrame: (event) => {
    // Screens can subscribe to frame timing
  }
})
```

---

### `ink/` ↔ `termio/`

**ANSI sequence emission:**

```typescript
// terminal.ts
export function writeDiffToTerminal(
  diff: Diff,
  terminal: Terminal,
  patches: readonly Patch[]
) {
  const chunks: string[] = []
  
  // Synchronized output (DEC 2026) if supported
  if (diff.supportsSynchronizedOutput) {
    chunks.push(BSU)  // \x1b[?2026h
  }
  
  for (const patch of patches) {
    if (patch.type === 'cursor') {
      chunks.push(cursorMove(patch.dx, patch.dy))
    } else if (patch.type === 'style') {
      chunks.push(stylePool.transition(patch.fromId, patch.toId))
    } else if (patch.type === 'text') {
      chunks.push(charPool.get(patch.charId))
    } else if (patch.type === 'hyperlink') {
      chunks.push(link(patch.url))  // OSC 8;;url BEL
    }
  }
  
  if (diff.supportsSynchronizedOutput) {
    chunks.push(ESU)  // \x1b[?2026l
  }
  
  terminal.stdout.write(chunks.join(''))
}
```

**Terminal queries (XTVERSION, DA1, cursor position):**
```typescript
// terminal-querier.ts
class TerminalQuerier {
  async queryXTVERSION(): Promise<string> {
    // Send CSI > 0 q, wait for DCS > | name ST response
    const response = await this.sendQuery('\x1b[>0q', XTVERSION_RE)
    return response.name
  }
  
  async queryCursorPosition(): Promise<{ row: number; col: number }> {
    // Send CSI 6 n (DSR), wait for CSI ? row ; col R
    const response = await this.sendQuery('\x1b[6n', CURSOR_POSITION_RE)
    return { row: response.row, col: response.col }
  }
}
```

---

## 9. Debugging & Instrumentation

### Frame Timing

```typescript
// frame.ts
type FrameEvent = {
  type: 'frame'
  time: number        // Total frame time (ms)
  yogaMs: number      // Yoga layout time
  patches: {
    stdout: number    // stdout patch count
    stderr: number    // stderr patch count
  }
}

// ink.tsx — emitted every frame
this.options.onFrame?.({
  type: 'frame',
  time: performance.now() - frameStart,
  yogaMs: getLastYogaMs(),
  patches: { stdout: stdoutPatches.length, stderr: stderrPatches.length }
})
```

### Scroll Profiling

```typescript
// reconciler.ts
let _yogaMs = 0
let _commitMs = 0

export function recordYogaMs(ms: number) {
  _yogaMs = ms
}

export function getLastYogaMs(): number {
  return _yogaMs
}

export function getLastCommitMs(): number {
  return _commitMs
}
```

### Debug Repaints

```bash
# Enable debug logging
export CLAUDE_CODE_DEBUG_REPAINTS=1

# Logs owner chain for full resets
[render] Full reset at row 42 — owner: ToolUseLoader > Messages > REPL
```

---

## 10. Summary

The `ink/` module is a sophisticated terminal UI rendering engine that:

1. **Leverages React** — Uses react-reconciler to convert component trees to terminal UIs
2. **Implements Flexbox** — Yoga WASM engine for responsive layouts
3. **Optimizes Rendering** — Double-buffered frames, cell packing, char/style interning, blit caching
4. **Handles Terminal Quirks** — Kitty keyboard, SGR mouse, DEC modes, OSC hyperlinks, synchronized output
5. **Supports Selection** — Fullscreen text selection with word/line granularity, scroll-off accumulation
6. **Provides Hooks** — `useInput`, `useApp`, `useTerminalSize`, etc. for component authors

Key files for modification:
- **`ink.tsx`** — Main render loop, frame management
- **`renderer.ts`** — DOM → Output conversion
- **`output.ts`** — Operation collector, Screen mutator
- **`screen.ts`** — Cell buffer, interning pools
- **`selection.ts`** — Text selection logic
- **`components/Box.tsx`**, **`Text.tsx`** — Core UI primitives
- **`hooks/use-input.ts`** — Input handling

