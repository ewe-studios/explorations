# Nordcraft Editor Architecture Deep Dive

## Overview

The Nordcraft editor is a sophisticated visual development environment that combines real-time canvas rendering, element tree management, and multi-panel data configuration. This deep-dive examines the technical architecture behind the editor's core systems.

## Editor Layout Architecture

The editor consists of four main sections working in concert:

```
┌─────────────────────────────────────────────────────────────┐
│                     Top Bar (AI, Preview)                    │
├──────────┬────────────────────────────────────┬──────────────┤
│          │                                    │              │
│  Left    │             Canvas                 │    Right     │
│  Panel   │                                    │    Panel     │
│          │                                    │              │
│ - Tree   │  - Visual element rendering        │ - Data Panel │
│ - Files  │  - Drag & drop manipulation        │ - Element    │
│ - Pkgs   │  - Responsive testing              │   Panel      │
│ - Issues │                                    │              │
├──────────┴────────────────────────────────────┴──────────────┤
│                    Bottom Bar                                 │
│  (Undo/Redo, Viewport, AI Assistant)                         │
└─────────────────────────────────────────────────────────────┘
```

### Canvas Rendering System

The canvas is the central workspace where elements are visually rendered and manipulated.

#### Rendering Pipeline

1. **Component Tree to DOM**: The editor maintains an in-memory representation of the component tree using `NodeModel` objects
2. **Virtual DOM Applier**: Changes to the tree are batched and applied efficiently
3. **Highlight Overlay**: A semi-transparent overlay system highlights elements on hover
4. **Selection Bounds**: Visual bounds (borders, handles) render around selected elements

```typescript
// Simplified canvas rendering flow
interface CanvasState {
  selectedNodeId: string | null
  hoveredNodeId: string | null
  viewport: { width: number; height: number }
  mode: 'design' | 'test'
}

// Node models from component definition
interface NodeModel {
  id: string
  type: 'text' | 'element' | 'component' | 'slot'
  tag?: string
  attrs: Record<string, Formula>
  style: NodeStyleModel
  children: string[]
  condition?: Formula
  repeat?: Formula
}
```

#### Element Highlighting and Selection

The highlighting system uses a layer-based approach:

```typescript
// Highlight calculation
function getHighlightBounds(nodeId: string, root: Document): DOMRect {
  const element = root.querySelector(`[data-node-id="${nodeId}"]`)
  if (!element) return null
  
  return element.getBoundingClientRect()
}

// Selection management
function selectNode(nodeId: string, state: EditorState): EditorState {
  return {
    ...state,
    selectedNodeId: nodeId,
    // Auto-expand element tree to show selected node
    expandedTreeNodes: expandToNode(state.expandedTreeNodes, nodeId)
  }
}
```

### Element Tree Management

The element tree (left panel) provides a hierarchical view of the component structure.

#### Tree Data Structure

```typescript
interface TreeNode {
  id: string
  name: string
  type: 'element' | 'component' | 'text' | 'slot'
  children: TreeNode[]
  depth: number
  isExpanded: boolean
  isVisible: boolean  // Based on condition formula
}

// Build tree from node models
function buildElementTree(
  nodes: Record<string, NodeModel>,
  rootId: string
): TreeNode {
  const node = nodes[rootId]
  
  return {
    id: node.id,
    name: getDisplayName(node),
    type: node.type,
    depth: 0,
    isExpanded: true,
    isVisible: evaluateCondition(node.condition),
    children: node.children.map(childId => {
      const child = buildElementTree(nodes, childId)
      child.depth = 1
      return child
    })
  }
}
```

#### Tree Synchronization

The element tree stays synchronized with the canvas through:

1. **Observer Pattern**: Changes to the component trigger tree rebuilds
2. **Virtual Scrolling**: Only visible tree nodes render for performance
3. **Selection Sync**: Canvas selection highlights corresponding tree node

### Panel Systems

The editor uses context-sensitive panels that adapt based on selection state.

#### Left Panel Navigation

```typescript
type LeftPanelView = 
  | 'element-tree'
  | 'project-sidebar'
  | 'packages'
  | 'issues'

interface LeftPanelState {
  currentView: LeftPanelView
  expandedKeys: Set<string>  // For tree expansion
  filterText: string
}
```

#### Right Panel Context Switching

The right panel switches between Data Panel and Element Panel based on selection:

```typescript
// Panel selection logic
function getRightPanelContent(selection: SelectionState): PanelContent {
  if (selection.selectedNodeId) {
    return {
      type: 'element-panel',
      tabs: ['style', 'attributes', 'events'],
      node: nodes[selection.selectedNodeId]
    }
  }
  
  if (selection.selectedComponent) {
    return {
      type: 'data-panel',
      sections: [
        'attributes', 'variables', 'apis',
        'events', 'formulas', 'workflows',
        'contexts', 'lifecycle'
      ]
    }
  }
  
  return { type: 'data-panel' }
}
```

## Drag and Drop System

The canvas supports sophisticated drag-and-drop operations for element manipulation.

### Drag Modes

```typescript
interface DragState {
  isDragging: boolean
  draggedNodeId: string
  dragPosition: { x: number; y: number }
  mode: 'reorder' | 'insert' | 'duplicate'
  dropTarget: {
    nodeId: string
    position: 'before' | 'after' | 'inside'
  } | null
}

// Keyboard modifier handling
function getDragMode(event: DragEvent): DragMode {
  if (event.ctrlKey || event.metaKey) {
    return 'insert'  // Force insertion mode
  }
  if (event.altKey) {
    return 'duplicate'  // Duplicate on drop
  }
  return 'reorder'  // Default reorder
}
```

### Insertion Mode Algorithm

When dragging outside a container or holding Cmd/Ctrl, insertion mode activates:

```typescript
function calculateInsertionPoint(
  dragPosition: Point,
  container: Element
): InsertionResult {
  const children = Array.from(container.children)
  
  // Find the sibling closest to the drag position
  for (let i = 0; i < children.length; i++) {
    const sibling = children[i]
    const siblingCenter = getCenterY(sibling)
    
    if (dragPosition.y < siblingCenter) {
      return {
        parentId: container.id,
        index: i,
        mode: 'insert'
      }
    }
  }
  
  // Append to end
  return {
    parentId: container.id,
    index: children.length,
    mode: 'insert'
  }
}
```

## Responsive Testing System

The canvas includes viewport resizing handles for responsive testing.

### Breakpoint Management

```typescript
interface Breakpoint {
  name: 'small' | 'medium' | 'large'
  minWidth?: number
  maxWidth?: number
}

const DEFAULT_BREAKPOINTS: Breakpoint[] = [
  { name: 'small', maxWidth: 640 },
  { name: 'medium', minWidth: 641, maxWidth: 1024 },
  { name: 'large', minWidth: 1025 }
]

// Viewport resize handling
function handleViewportResize(
  width: number,
  height: number
): void {
  const activeBreakpoint = getActiveBreakpoint(width, DEFAULT_BREAKPOINTS)
  
  // Update canvas dimensions
  canvasElement.style.width = `${width}px`
  canvasElement.style.height = `${height}px`
  
  // Trigger media query recalculation
  recalculateMediaQueries(activeBreakpoint)
  
  // Store in URL for shareability
  updateUrlParams({
    'canvas-width': width.toString(),
    'canvas-height': height.toString()
  })
}
```

## Keyboard Shortcut System

The editor implements a comprehensive keyboard shortcut system:

```typescript
interface ShortcutHandler {
  keys: string[]  // e.g., ['Cmd', 'C']
  handler: (event: KeyboardEvent) => void
  context: 'global' | 'canvas' | 'tree' | 'panel'
}

// Core shortcuts
const SHORTCUTS: ShortcutHandler[] = [
  {
    keys: ['Escape'],
    handler: () => deselectAll(),
    context: 'global'
  },
  {
    keys: ['Space'],
    handler: (e) => enablePanMode(e),
    context: 'canvas'
  },
  {
    keys: ['Cmd', 'K'],
    handler: () => toggleProjectSidebar(),
    context: 'global'
  }
]
```

## State Management Architecture

The editor uses a centralized state management system with isolated slices:

```typescript
interface EditorState {
  // Component/Page being edited
  project: string
  branch: string
  component: Component | null
  
  // Selection state
  selection: SelectionState
  
  // Panel state
  leftPanel: LeftPanelState
  rightPanel: RightPanelState
  
  // Canvas state
  canvas: CanvasState
  
  // Undo/redo history
  history: {
    past: EditorState[]
    future: EditorState[]
  }
  
  // UI preferences
  preferences: {
    theme: 'light' | 'dark'
    fontSize: number
  }
}
```

### State Updates and Reactions

```typescript
// State update with automatic reactions
function updateEditorState<K extends keyof EditorState>(
  key: K,
  value: EditorState[K]
): void {
  const previousState = currentState
  
  // Update state
  currentState = {
    ...currentState,
    [key]: value
  }
  
  // Trigger reactions based on what changed
  triggerReactions(previousState, currentState)
}

// Example reaction
function triggerReactions(
  previous: EditorState,
  current: EditorState
): void {
  // If selection changed, update right panel
  if (previous.selection !== current.selection) {
    updateRightPanelContent(current.selection)
  }
  
  // If canvas mode changed, re-render
  if (previous.canvas.mode !== current.canvas.mode) {
    rerenderCanvas(current.canvas)
  }
}
```

## Performance Optimizations

The editor implements several performance optimizations:

### 1. Virtual Tree Rendering

Only visible tree nodes render:

```typescript
function useVirtualTree(
  nodes: TreeNode[],
  containerHeight: number
): VirtualTreeResult {
  const rowHeight = 28
  const visibleCount = Math.ceil(containerHeight / rowHeight)
  
  // Calculate visible range
  const startIndex = Math.max(0, scrollTop / rowHeight)
  const endIndex = Math.min(nodes.length, startIndex + visibleCount)
  
  return {
    visibleNodes: nodes.slice(startIndex, endIndex),
    offsetY: startIndex * rowHeight
  }
}
```

### 2. Debounced Formula Evaluation

Formula evaluation debounces to prevent excessive recalculations:

```typescript
const debouncedEvaluate = debounce(
  (formula: Formula, env: FormulaEnv) => {
    return evaluateFormula(formula, env)
  },
  150  // 150ms debounce
)
```

### 3. Canvas Render Batching

DOM updates batch together:

```typescript
// Batch DOM updates
const batchedUpdates = new Map<string, Update>()

function scheduleUpdate(nodeId: string, update: Update): void {
  batchedUpdates.set(nodeId, update)
  
  requestAnimationFrame(() => {
    applyBatchedUpdates(batchedUpdates)
    batchedUpdates.clear()
  })
}
```

## Editor Extension Architecture

The editor supports extensions through a plugin system:

```typescript
interface EditorExtension {
  id: string
  name: string
  
  // Lifecycle hooks
  onMount?(editor: EditorAPI): void
  onUnmount?(): void
  
  // Panel contributions
  panels?: {
    position: 'left' | 'right' | 'bottom'
    icon: React.ComponentType
    content: React.ComponentType
  }[]
  
  // Keyboard shortcuts
  shortcuts?: ShortcutHandler[]
  
  // Menu contributions
  menus?: MenuContribution[]
}
```

## Summary

The Nordcraft editor architecture combines:

1. **Real-time canvas rendering** with virtual DOM and efficient updates
2. **Hierarchical element tree** with synchronization to canvas selection
3. **Context-sensitive panels** that adapt based on selection state
4. **Sophisticated drag-and-drop** with insertion/reorder/duplicate modes
5. **Responsive testing** with viewport controls and breakpoint management
6. **Centralized state management** with automatic reactions to changes
7. **Performance optimizations** including virtualization, debouncing, and batching

This architecture enables the visual development experience while maintaining performance even with complex component trees.
