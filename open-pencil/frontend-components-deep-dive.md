---
source: /home/darkvoid/Boxxed/@formulas/src.AppOSS/open-pencil
explored_at: 2026-03-17T23:15:00Z
---

# Frontend Components Deep Dive: OpenPencil

## Overview

OpenPencil's frontend is a **Vue 3 SPA** with a component architecture built for a Figma-like design editor. It uses **CanvasKit (Skia WASM)** for rendering, **Reka UI** for UI primitives, **Tailwind CSS 4** for styling, and a custom store system with Vue reactivity.

**Key characteristics:**
- **No React** — Pure Vue 3 with Composition API (`<script setup>`)
- **No traditional web components** — Vue Single File Components (SFCs) compiled by Vite
- **Canvas-first** — UI panels wrap a WebGL canvas, not DOM elements
- **Dark theme by default** — Custom color tokens via Tailwind CSS 4
- **Responsive splitter layout** — Reka UI Splitter for resizable panels

---

## Tech Stack Breakdown

### Core Framework

| Technology | Purpose |
|------------|---------|
| **Vue 3.5+** | UI framework with Composition API |
| **Vite 7** | Build tool with HMR |
| **TypeScript 5.8** | Type safety throughout |
| **Tailwind CSS 4** | Utility-first styling (CSS variables via `@theme`) |
| **Reka UI** | Headless UI primitives (Splitter, Popover, DropdownMenu, etc.) |
| **VueUse** | Composition utilities (useEventListener, useResizeObserver, onClickOutside) |
| **unplugin-icons** | Iconify/Lucide icons as Vue components (`<icon-lucide-*>`) |
| **unplugin-vue-components** | Auto-import for Vue components and icons |

### Not Absences

- **No React** — All components are Vue SFCs
- **No JSX** — Templates use Vue template syntax
- **No traditional web components** — Vue SFCs compiled to efficient JS, not Custom Elements
- **No CSS-in-JS** — Tailwind utilities + CSS custom properties
- **No state library** — Vanilla Vue reactivity (`shallowReactive`, `shallowRef`)

---

## Component Architecture

### Component Categories

```
src/components/
├── EditorCanvas.vue        # Main canvas wrapper
├── Toolbar.vue             # Floating tool palette
├── LayersPanel.vue         # Tree view with drag-drop
├── PropertiesPanel.vue     # Contextual property editor
├── ChatPanel.vue           # AI chat interface
├── CodePanel.vue           # Generated code view
├── CollabPanel.vue         # Collaboration / presence UI
├── TabBar.vue              # Multi-tab document navigation
│
├── properties/             # Property editor sections
│   ├── LayoutSection.vue   # Auto layout, padding, alignment
│   ├── FillSection.vue     # Fill/stroke editors
│   ├── StrokeSection.vue   # Stroke properties
│   ├── TypographySection.vue # Text properties
│   ├── EffectsSection.vue  # Shadows, blurs
│   └── ...
│
├── chat/                   # AI chat sub-components
│   ├── APIKeySetup.vue     # OpenRouter API key config
│   ├── ChatInput.vue       # Message input with stop button
│   └── ChatMessage.vue     # Message display with tool calls
│
└── Shared UI components
    ├── ScrubInput.vue      # Drag-to-edit number input
    ├── ColorPicker.vue     # Color swatch + popover
    ├── HsvColorArea.vue    # HSV color selector
    ├── FontPicker.vue      # System font dropdown
    ├── FillPicker.vue      # Complex fill editor
    ├── AppMenu.vue         # App menu (file, edit, etc.)
    ├── AppSelect.vue       # Custom select dropdown
    └── AppToast.vue        # Toast notifications
```

---

## Key Components Explained

### 1. EditorCanvas.vue

**Purpose:** Wrapper for the Skia WASM canvas with input handling.

```vue
<template>
  <CanvasContextMenu>
    <div class="canvas-area">
      <canvas ref="canvasRef" :style="{ cursor }" />
      <!-- Loading indicator -->
    </div>
  </CanvasContextMenu>
</template>
```

**Composables used:**
- `useCanvas()` — CanvasKit initialization, surface management, render loop
- `useCanvasInput()` — Pointer events, pan/zoom, selection, marquee
- `useTextEdit()` — Text editing overlay
- `useCollabInjected()` — Remote cursor broadcasting

**Cursor logic:**
```typescript
const cursor = computed(() => {
  if (cursorOverride.value) return cursorOverride.value
  const tool = store.state.activeTool
  if (tool === 'HAND') return 'grab'
  if (tool === 'SELECT') return 'default'
  if (tool === 'TEXT') return 'text'
  return 'crosshair'
})
```

---

### 2. LayersPanel.vue

**Purpose:** Tree view of scene graph nodes with drag-drop reordering.

**Key features:**
- **Reka UI TreeRoot** for hierarchical display
- **Drag-drop reordering** with pointer events
- **Context menu** for node operations
- **Auto-scroll to selection**
- **Expand/collapse** with stored state

**Drag-drop implementation:**
```typescript
function onPointerDown(e: PointerEvent, nodeId: string) {
  dragStartY = e.clientY
  stopMove = useEventListener(document, 'pointermove', (ev) => {
    if (Math.abs(ev.clientY - dragStartY) > 4) {
      dragging.value = true
      updateDropTarget(ev) // Calculates insert position
    }
  })
  stopUp = useEventListener(document, 'pointerup', () => {
    if (dropTarget.value) {
      store.graph.reorderChild(dragNodeId.value, parentId, index)
    }
    cleanup()
  })
}
```

**Drop target calculation:**
- Detects row hover zone (top 25% = insert before, middle = drop into, bottom 25% = insert after)
- Shows blue indicator line at drop position
- Supports nesting by calculating depth

**Tree item rendering:**
```vue
<TreeItem v-slot="{ isExpanded }" v-bind="item.bind" @select="onSelect">
  <button
    :data-node-id="item.value.id"
    :data-level="item.level"
    :style="{ paddingLeft: `${8 + (item.level - 1) * 16}px` }"
  >
    <icon-lucide-chevron-right :class="isExpanded ? 'rotate-90' : 'rotate-0'" />
    <component :is="nodeIcons[item.value.type]" />
    <span>{{ item.value.name }}</span>
  </button>
</TreeItem>
```

---

### 3. ScrubInput.vue

**Purpose:** Drag-to-edit number input (like Figma's scrubbing inputs).

**Key behavior:**
- **Drag horizontally** to change value (ew-resize cursor)
- **Click to type** exact value
- **Enter to commit**, Escape to cancel
- **Min/max clamping**
- **Icon/label prefix + suffix**

```vue
<div
  class="flex items-center rounded border bg-input"
  :style="{ cursor: editing ? 'auto' : 'ew-resize' }"
  @pointerdown="!editing && startScrub($event)"
>
  <span class="px-[5px]">
    <slot name="icon"><span v-if="icon">{{ icon }}</span></slot>
  </span>
  <input v-if="editing" ref="inputRef" type="number" :value="displayValue" />
  <span v-else>{{ displayValue }}<span v-if="suffix">{{ suffix }}</span></span>
</div>
```

**Scrubbing logic:**
```typescript
function startScrub(e: PointerEvent) {
  const startX = e.clientX
  let accumulated = props.modelValue

  stopMove = useEventListener(document, 'pointermove', (ev) => {
    const dx = ev.clientX - lastX
    accumulated += dx * props.step * props.sensitivity
    const clamped = Math.round(Math.min(max, Math.max(min, accumulated)))
    emit('update:modelValue', clamped)
  })

  stopUp = useEventListener(document, 'pointerup', () => {
    if (hasMoved) emit('commit', modelValue, valueBeforeScrub)
    else startEdit() // Click without drag = edit mode
  })
}
```

**Usage throughout the app:**
```vue
<ScrubInput
  icon="W"
  :model-value="node.width"
  :min="0"
  @update:model-value="updateProp('width', $event)"
  @commit="(v, p) => commitProp('width', v, p)"
/>
```

---

### 4. PropertiesPanel.vue → LayoutSection.vue

**Purpose:** Contextual property editor for selected nodes.

**Structure:**
```
PropertiesPanel.vue
├── PageSection.vue      (for PAGE nodes)
├── PositionSection.vue  (x, y coordinates)
├── LayoutSection.vue    (width, height, auto layout)
├── FillSection.vue      (fills, strokes)
├── StrokeSection.vue    (stroke weight, style)
├── EffectsSection.vue   (shadows, blurs)
├── TypographySection.vue (text properties)
└── ExportSection.vue    (export settings)
```

**LayoutSection features:**
- **Width/Height with sizing dropdown** (Fixed, Hug, Fill)
- **Auto layout toggle** (Shift+A shortcut)
- **Direction buttons** (Vertical, Horizontal, Wrap)
- **3x3 alignment grid**
- **Item spacing (gap)**
- **Uniform/individual padding toggle**

**Sizing dropdown implementation:**
```vue
<button @click="widthSizingOpen = !widthSizingOpen">
  {{ sizingLabel(widthSizing) }} <!-- Fixed/Hug/Fill -->
</button>
<div v-if="widthSizingOpen" class="absolute z-10 rounded border bg-panel">
  <button @click="setWidthSizing('FIXED')">Fixed width</button>
  <button v-if="node.layoutMode !== 'NONE'" @click="setWidthSizing('HUG')">
    Hug contents
  </button>
  <button v-if="isInAutoLayout" @click="setWidthSizing('FILL')">
    Fill container
  </button>
</div>
```

**Auto layout direction buttons:**
```vue
<button
  :class="node.layoutMode === 'VERTICAL' ? 'bg-accent' : 'bg-input'"
  @click="store.setLayoutMode(node.id, 'VERTICAL')"
>
  <!-- Vertical icon SVG -->
</button>
```

---

### 5. ChatPanel.vue

**Purpose:** AI chat interface with streaming responses and tool use visualization.

**Architecture:**
```vue
<ScrollAreaRoot>
  <ScrollAreaViewport>
    <div v-if="messages.length === 0">Empty state</div>
    <div v-else>
      <ChatMessage v-for="msg in messages" :message="msg" />
      <div v-if="status === 'submitted'">
        <!-- Typing indicator (3 bouncing dots) -->
      </div>
    </div>
  </ScrollAreaViewport>
  <ScrollAreaScrollbar />
</ScrollAreaRoot>
<ChatInput :status="status" @submit="handleSubmit" @stop="handleStop" />
```

**Chat composable (`use-chat.ts`):**
```typescript
const SYSTEM_PROMPT = dedent`
  You are a design assistant inside OpenPencil, a Figma-like design editor.
  Available node types: FRAME, RECTANGLE, ELLIPSE, TEXT, LINE, STAR, POLYGON, SECTION.
  Always use tools to make changes.
`

function createTransport() {
  const openrouter = createOpenRouter({ apiKey: apiKey.value })
  const tools = createAITools(useEditorStore())

  const agent = new ToolLoopAgent({
    model: openrouter(modelId.value),
    instructions: SYSTEM_PROMPT,
    tools
  })

  return new DirectChatTransport({ agent })
}

function ensureChat() {
  if (!chat) {
    chat = new Chat({ transport: createTransport() })
  }
  return chat
}
```

**AI models supported:**
```typescript
export const AI_MODELS = [
  { id: 'anthropic/claude-3.5-sonnet', name: 'Claude 3.5 Sonnet' },
  { id: 'anthropic/claude-sonnet-4', name: 'Claude Sonnet 4' },
  { id: 'openai/gpt-4o', name: 'GPT-4o' },
  // ... more via OpenRouter
]
```

---

### 6. Toolbar.vue

**Purpose:** Floating tool palette at bottom of canvas.

**Tool structure:**
```typescript
export const TOOLS: ToolDef[] = [
  { key: 'SELECT', label: 'Move', shortcut: 'V' },
  { key: 'FRAME', label: 'Frame', shortcut: 'F', flyout: ['FRAME', 'SECTION'] },
  {
    key: 'RECTANGLE',
    label: 'Rectangle',
    shortcut: 'R',
    flyout: ['RECTANGLE', 'LINE', 'ELLIPSE', 'POLYGON', 'STAR']
  },
  { key: 'PEN', label: 'Pen', shortcut: 'P' },
  { key: 'TEXT', label: 'Text', shortcut: 'T' },
  { key: 'HAND', label: 'Hand', shortcut: 'H' }
]
```

**Split button for tools with flyout:**
```vue
<div v-if="tool.flyout && tool.flyout.length > 1">
  <!-- Main button -->
  <button @click="store.setTool(activeKeyForTool(tool))">
    <component :is="toolIcons[activeKeyForTool(tool)]" />
  </button>

  <!-- Dropdown trigger -->
  <DropdownMenuRoot>
    <DropdownMenuTrigger>
      <icon-lucide-chevron-down />
    </DropdownMenuTrigger>
    <DropdownMenuPortal>
      <DropdownMenuContent>
        <DropdownMenuItem v-for="sub in tool.flyout" @select="store.setTool(sub)">
          <component :is="toolIcons[sub]" />
          <span>{{ toolLabels[sub] }}</span>
          <span class="text-muted">{{ toolShortcuts[sub] }}</span>
        </DropdownMenuItem>
      </DropdownMenuContent>
    </DropdownMenuPortal>
  </DropdownMenuRoot>
</div>
```

---

### 7. ColorPicker.vue + HsvColorArea.vue

**Purpose:** Color selection with HSV color area.

**ColorPicker:**
```vue
<PopoverRoot>
  <PopoverTrigger>
    <button :style="{ background: swatchColor }" /> <!-- Color swatch -->
  </PopoverTrigger>
  <PopoverContent side="left">
    <HsvColorArea :color="color" @update="emit('update', $event)" />
  </PopoverContent>
</PopoverRoot>
```

**HSV Color Area:**
- **Hue slider** (vertical, 0-360°)
- **Saturation/Value area** (2D gradient)
- **Alpha slider**
- **RGBA + HEX inputs**

**Color format:**
```typescript
interface Color {
  r: number // 0-1
  g: number // 0-1
  b: number // 0-1
  a: number // 0-1
}
```

---

## Composables (Vue Hooks)

### useCanvas

**Purpose:** CanvasKit WASM lifecycle and rendering.

```typescript
export function useCanvas(canvasRef, store) {
  let renderer: SkiaRenderer | null = null
  let ck: CanvasKit | null = null

  async function init() {
    ck = await getCanvasKit()
    if (getGpuBackend() === 'webgpu') {
      gpuCtx = await initWebGPU(ck) // WebGPU support
    }
    createSurface(canvas)
    renderer.loadFonts().then(() => renderNow())
  }

  function renderNow() {
    renderer.dpr = devicePixelRatio
    renderer.panX = store.state.panX
    renderer.panY = store.state.panY
    renderer.zoom = store.state.zoom
    renderer.render(graph, selectedIds, debugOverlays)
  }

  useRafFn(() => {
    if (dirty || store.state.renderVersion !== lastRenderVersion) {
      renderNow()
    }
  })
}
```

**WebGPU vs WebGL:**
```typescript
async function initWebGPU(ck: CanvasKit): Promise<WebGPUContext | null> {
  if (!('gpu' in navigator)) return null
  const adapter = await navigator.gpu.requestAdapter()
  const device = await adapter.requestDevice()
  const deviceContext = asWebGPU(ck).MakeGPUDeviceContext(device)
  return { device, deviceContext }
}

function createSurface(canvas) {
  if (getGpuBackend() === 'webgpu' && gpuCtx) {
    // WebGPU surface
    const gpu = asWebGPU(ck)
    const canvasCtx = gpu.MakeGPUCanvasContext(gpuCtx.deviceContext, canvas)
    surface = gpu.MakeGPUCanvasSurface(canvasCtx, ...)
  } else {
    // WebGL fallback
    surface = makeGLSurface(canvas)
  }
}
```

---

### useCollab

**Purpose:** P2P collaboration via WebRTC + Yjs.

**Full implementation in** `src/composables/use-collab.ts` — see main exploration.md for details.

**Key pattern:**
```typescript
export const COLLAB_KEY = Symbol('collab') as InjectionKey<CollabReturn>

// In EditorView.vue
const collab = useCollab(firstTab.store)
provide(COLLAB_KEY, collab)

// In child components
const collab = useCollabInjected() // inject(COLLAB_KEY)
```

---

### useKeyboard

**Purpose:** Global keyboard shortcuts for tools and actions.

```typescript
export function useKeyboard() {
  const store = useEditorStore()

  useEventListener(document, 'keydown', (e) => {
    // Tool shortcuts (V, F, R, O, L, T, P, H)
    if (e.target instanceof HTMLInputElement) return

    const key = e.key.toLowerCase()
    if (TOOL_SHORTCUTS[key] && !e.metaKey && !e.ctrlKey) {
      store.setTool(TOOL_SHORTCUTS[key])
      e.preventDefault()
    }

    // Cmd/Ctrl shortcuts
    if ((e.metaKey || e.ctrlKey)) {
      switch (key) {
        case 'z': store.undo()
        case 'y': store.redo()
        case 'c': store.copy()
        case 'v': store.paste()
        case 'g': store.groupSelection()
        case '\\': store.state.showUI = !store.state.showUI
      }
    }

    // Delete/Backspace
    if (key === 'delete' || key === 'backspace') {
      store.deleteSelection()
    }

    // Enter to rename
    if (key === 'enter' && store.state.selectedIds.size === 1) {
      store.startEditingText()
    }
  })
}
```

---

### useNodeProps

**Purpose:** Reactive node property access for property panels.

```typescript
export function useNodeProps() {
  const store = useEditorStore()
  const node = computed(() => {
    const id = [...store.state.selectedIds][0]
    return id ? store.graph.getNode(id) : null
  })

  function updateProp<K extends keyof SceneNode>(key: K, value: SceneNode[K]) {
    if (node.value) store.graph.updateNode(node.value.id, { [key]: value })
  }

  function commitProp(key: string, value: unknown, previous: unknown) {
    store.commitNodeUpdate(node.value!.id, { [key]: value }, `Change ${key}`)
  }

  return { store, node, updateProp, commitProp }
}
```

**Usage in LayoutSection.vue:**
```vue
<script setup>
const { store, node, updateProp, commitProp } = useNodeProps()
</script>

<ScrubInput
  :model-value="node.width"
  @update:model-value="updateProp('width', $event)"
  @commit="(v, p) => commitProp('width', v, p)"
/>
```

---

## State Management

### Editor Store (`src/stores/editor.ts`)

**Pattern:** Vanilla Vue reactivity with `shallowReactive` + `shallowRef`.

```typescript
export function createEditorStore() {
  let graph = new SceneGraph()
  const undo = new UndoManager()

  const state = shallowReactive({
    activeTool: 'SELECT' as Tool,
    selectedIds: new Set<string>(),
    panX: 0,
    panY: 0,
    zoom: 1,
    sceneVersion: 0,
    renderVersion: 0,
    currentPageId: '',
    // ... more state
  })

  function updateNode(id: string, changes: Partial<SceneNode>) {
    graph.updateNode(id, changes)
    state.sceneVersion++
    state.renderVersion++
  }

  function requestRender() {
    state.renderVersion++
  }

  return { state, graph, undo, updateNode, requestRender, ... }
}
```

**Key design decisions:**
- `shallowReactive` — Avoids deep reactivity overhead for large objects
- `sceneVersion` vs `renderVersion` — UI panels watch `sceneVersion`, canvas watches `renderVersion`
- `UndoManager` — Centralized undo/redo with descriptive actions

---

### Tab Store (`src/stores/tabs.ts`)

**Purpose:** Multi-tab document management.

```typescript
interface EditorTab {
  id: string
  name: string
  store: EditorStore
}

const tabs = shallowReactive<EditorTab[]>([createTab()])
const activeTabId = ref<string>(tabs[0].id)

export const activeTab = computed(() =>
  tabs.find(t => t.id === activeTabId.value)
)

export function createTab() {
  const store = createEditorStore()
  const tab = { id: crypto.randomUUID(), name: 'Untitled', store }
  tabs.push(tab)
  return tab
}
```

---

## UI Primitives (Reka UI)

OpenPencil uses **Reka UI** (formerly Radix Vue) for headless UI components:

| Component | Usage |
|-----------|-------|
| `SplitterGroup`, `SplitterPanel`, `SplitterResizeHandle` | Resizable panels |
| `TreeRoot`, `TreeItem` | Layer tree hierarchy |
| `ContextMenuRoot`, `ContextMenuTrigger`, `ContextMenuContent` | Right-click menus |
| `DropdownMenuRoot`, `DropdownMenuContent` | Tool flyouts |
| `PopoverRoot`, `PopoverTrigger`, `PopoverContent` | Color picker, sizing dropdowns |
| `ScrollAreaRoot`, `ScrollAreaViewport`, `ScrollAreaScrollbar` | Scrollable panels |
| `TabsRoot`, `TabsList`, `TabsTrigger`, `TabsContent` | Design/Code/AI tabs |

**Why Reka UI:**
- Headless — No styling, full control via Tailwind
- Accessible — WAI-ARIA patterns built-in
- Vue 3 native — Composition API friendly
- Type-safe — Full TypeScript support

---

## Styling Approach

### Tailwind CSS 4 with Custom Theme

```css
@theme {
  --color-panel: #2a2a2a;
  --color-canvas: #1e1e1e;
  --color-border: #3a3a3a;
  --color-hover: #353535;
  --color-accent: #3b82f6;
  --color-surface: #e0e0e0;
  --color-muted: #888888;
  --color-input: #1e1e1e;
}
```

### CSS Containment

```css
/* Panels isolated from canvas repaints */
aside {
  contain: paint layout style;
}
```

### Number Input Styling (Global)

```css
input[type='number'] {
  appearance: textfield;
  -moz-appearance: textfield;
}

input[type='number']::-webkit-inner-spin-button,
input[type='number']::-webkit-outer-spin-button {
  display: none;
}
```

---

## Build Configuration

### Vite Config (`vite.config.ts`)

```typescript
export default defineConfig({
  resolve: {
    alias: {
      '@': resolve(__dirname, 'src'),
      shiki: resolve(__dirname, 'src/shims/shiki.ts')
    }
  },
  plugins: [
    tailwindcss(),
    Icons({ compiler: 'vue3' }), // Iconify/Lucide
    Components({
      resolvers: [IconsResolver({ prefix: 'icon' })]
    }),
    vue()
  ],
  server: {
    port: 1420,
    hmr: { protocol: 'ws', host, port: 1421 }
  }
})
```

### CanvasKit WASM Copy

```typescript
{
  name: 'copy-canvaskit-wasm',
  buildStart() {
    copyFileSync('node_modules/canvaskit-wasm/bin/canvaskit.wasm', 'public/canvaskit.wasm')
    // Also copies WebGPU variant
  }
}
```

---

## Icon System

**unplugin-icons** with Iconify/Lucide:

```vue
<icon-lucide-frame class="size-4" />
<icon-lucide-chevrons-right class="size-3" />
```

**Auto-imported via:**
```typescript
Components({
  resolvers: [IconsResolver({ prefix: 'icon' })]
})
```

**Available icon sets:**
- `lucide` — Default icon set (modern, consistent)
- Iconify — 200k+ icons from 100+ sets

---

## Testing

### Playwright E2E (`tests/e2e/`)

```typescript
// tests/e2e/basic.spec.ts
import { test, expect } from '@playwright/test'

test('creates rectangle', async ({ page }) => {
  await page.goto('/')
  await page.click('[data-tool="rectangle"]')
  await page.mouse.move(100, 100)
  await page.mouse.down()
  await page.mouse.move(300, 200)
  await page.mouse.up()

  const canvas = page.locator('canvas')
  await expect(canvas).toHaveScreenshot('rectangle-created.png')
})
```

### Unit Tests (`tests/engine/`)

```typescript
// tests/engine/scene-graph.test.ts
import { describe, test, expect } from 'bun:test'
import { SceneGraph } from '@open-pencil/core'

test('creates node with default fill', () => {
  const graph = new SceneGraph()
  const rect = graph.createNode('RECTANGLE', 'page')
  expect(rect.fills).toEqual([DEFAULT_SHAPE_FILL])
})
```

---

## Performance Optimizations

### 1. `shallowReactive` for Large Objects

```typescript
const state = shallowReactive({...}) // Only top-level reactive
```

### 2. Separate Version Counters

```typescript
state.sceneVersion++  // UI panels recompute
state.renderVersion++ // Canvas repaints only
```

### 3. CSS Containment

```css
contain: paint layout style; /* Isolate panel repaints */
```

### 4. rAF-Throttled Resize

```typescript
let resizeRaf = 0
useResizeObserver(canvasRef, () => {
  if (resizeRaf) return
  resizeRaf = requestAnimationFrame(() => {
    resizeRaf = 0
    resizeCanvas(canvas)
  })
})
```

### 5. Viewport Culling in Renderer

```typescript
// Only render nodes within viewport
const visibleNodes = graph.getAllNodes().filter(node =>
  intersectsViewport(node, viewport)
)
```

---

## Key Insights

### Vue 3 Composition API Pattern

All components use `<script setup>` with composable functions:
```vue
<script setup lang="ts">
import { computed, ref, watch } from 'vue'
import { useEventListener } from '@vueuse/core'

const props = defineProps<{ color: Color }>()
const emit = defineEmits<{ update: [color: Color] }>()

const { store, node } = useNodeProps()
</script>
```

### Injection Pattern for Cross-Cutting Concerns

```typescript
// Provide in root
provide(COLLAB_KEY, collab)

// Inject in children
const collab = useCollabInjected()
```

### Pointer Events for Drag Operations

All drag interactions (scrubbing, layer reordering, pan/zoom) use pointer events:
```typescript
function onPointerDown(e: PointerEvent) {
  stopMove = useEventListener(document, 'pointermove', onMove)
  stopUp = useEventListener(document, 'pointerup', onUp)
}
```

### No React, No Problem

OpenPencil demonstrates that Vue 3 + Tailwind + headless UI can build Figma-class interfaces without React ecosystem dependencies.

---

## Open Questions

1. **Why not Nuxt?** — Pure Vite SPA gives more control over build output
2. **CSS containment impact** — Has it been measured for actual repaint isolation?
3. **Shallow reactivity gotchas** — Any bugs from non-reactive nested properties?
4. **WebGPU adoption** — What % of users actually use WebGPU vs WebGL?
5. **Mobile support** — Any touch-specific optimizations planned?
