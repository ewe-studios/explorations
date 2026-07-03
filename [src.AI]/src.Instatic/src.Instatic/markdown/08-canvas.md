# Canvas Deep Dive: Rendering, DnD, Composition, Breakpoint Frames

The visual editor canvas is the core of Instatic's editing experience. It renders pages live, supports drag-and-drop reordering, inline editing, and breakpoint-aware previews.

**Source:** `/home/darkvoid/Boxxed/@formulas/src.rust/src.AI/src.Instatic/Instatic/src/admin/pages/site/canvas/`

## File Map

| File | Purpose |
|------|---------|
| `CanvasLiveSurface.tsx` | Main canvas surface with live rendering |
| `CanvasComposedTree.tsx` | Composed tree rendering (modules + VCs) |
| `canvasComposition.ts` | Composition logic for rendering nodes |
| `canvasNodeClassName.ts` | Class name resolution for canvas nodes |
| `canvasDnd.ts` | Drag-and-drop integration (@dnd-kit/core) |
| `canvasDomGeometry.ts` | DOM geometry calculations |
| `canvasDomReadyReplay.ts` | DOM ready state replay |
| `canvasFormPreview.ts` | Form preview in canvas |
| `BreakpointFrame.tsx` | Breakpoint-aware rendering frame |
| `BreakpointSelectionOverlay.tsx` | Breakpoint selection overlay |
| `CanvasContextSelector.tsx` | Context selection UI |
| `CanvasContexts.ts` | React contexts for canvas |
| `CanvasInsertModuleButton.tsx` | Module insertion button |
| `CanvasLayerContextMenu.tsx` | Layer context menu |
| `CanvasModeToggle.tsx` | Mode toggle (edit/preview) |
| `canvasClassCss.ts` | Canvas-specific CSS class handling |

## CanvasLiveSurface

The `CanvasLiveSurface` is the main canvas component:

- Renders the page tree as a live preview
- Supports inline editing (click text to edit)
- Handles drag-and-drop for reordering nodes
- Manages breakpoint frames for responsive preview
- Integrates with the editor store for state

## CanvasComposedTree

The `CanvasComposedTree` renders the composed tree:

- Walks the page tree node by node
- For each node, renders the corresponding module preview
- Handles Visual Component refs by inlining the VC tree
- Handles loops by rendering the current iteration

## Canvas Composition

The `canvasComposition` module handles:

- Resolving module definitions for each node
- Building the React component tree from the page tree
- Handling specialized node types (VC refs, loops)
- Passing render configuration to modules

## Drag and Drop

The `canvasDnd` module integrates with `@dnd-kit/core`:

- Defines drop zones for each node
- Handles drag start, drag over, and drop events
- Updates the page tree via the editor store
- Manages visual feedback during drag operations

## DOM Geometry

The `canvasDomGeometry` module handles:

- Calculating node positions and sizes
- Handling viewport scaling
- Managing selection outlines
- Computing insertion indicators

## Breakpoint Frames

The `BreakpointFrame` component:

- Renders the page at a specific viewport size
- Shows breakpoint boundaries
- Allows switching between breakpoints
- Maintains layout consistency across breakpoints

**Aha:** The canvas renders pages using the **same rendering pipeline** as the publisher, but in preview mode. This means what you see in the canvas is exactly what gets published — no separate preview renderer. The canvas uses `canvasComposition.ts` which follows the same bottom-up walk as `renderNode.ts`, but produces React components instead of HTML strings.

## DOM Ready Replay

The `canvasDomReadyReplay` module:

- Tracks DOM ready state during page load
- Replays DOM mutations for accurate preview
- Handles timing-dependent modules (animations, lazy loading)

## Canvas Contexts

The `CanvasContexts.ts` file defines React contexts for:

- Current page reference
- Editor store access
- Breakpoint context
- Selection context
- Mode context (edit vs preview)

## Next

Continue to [10-ai-agent](10-ai-agent.md) for the site agent deep dive.
