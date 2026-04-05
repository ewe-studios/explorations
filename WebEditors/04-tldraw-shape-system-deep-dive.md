---
location: /home/darkvoid/Boxxed/@dev/repo-expolorations/WebEditors
repository: https://github.com/tldraw/tldraw
explored_at: 2026-04-05
language: TypeScript
---

# tldraw Shape System: Complete Deep-Dive

This document provides an exhaustive technical exploration of tldraw's Shape System - the core abstraction that defines every visual element on the canvas. From simple rectangles to complex connectors, every shape follows the ShapeUtil pattern, enabling unlimited extensibility. This is the definitive guide for creating custom shapes.

## Table of Contents

1. [Shape Architecture](#1-shape-architecture)
2. [ShapeUtil Base Class](#2-shapeutil-base-class)
3. [Default Shapes](#3-default-shapes)
4. [Custom Shapes](#4-custom-shapes)
5. [Shape State Management](#5-shape-state-management)
6. [Shape Interactions](#6-shape-interactions)
7. [Connectors and Bindings](#7-connectors-and-bindings)
8. [Shape Utilities](#8-shape-utilities)

---

## 1. Shape Architecture

### 1.1 TLShape Interface

Every shape in tldraw implements the `TLShape` interface, which defines the core properties all shapes share:

```typescript
// packages/tldraw/src/lib/shapes/TLShape.ts
interface TLShape extends TLRecord {
  id: TLShapeId
  typeName: 'shape'
  parentId: TLShapeId | TLPageId
  index: IndexType
  type: string  // Shape type identifier (e.g., 'geo', 'arrow', 'text')
  
  // Transform properties
  x: number
  y: number
  rotation: number
  
  // Opacity (0-1)
  opacity: number
  
  // Group ID if shape is part of a group
  groupId: TLShapeId | null
  
  // Shape-specific props (defined by each ShapeUtil)
  props: object
  
  // Meta properties (user-defined arbitrary data)
  meta: Record<string, unknown>
}
```

**Key Properties Explained:**

| Property | Purpose | Notes |
|----------|---------|-------|
| `id` | Unique shape identifier | Generated via `createShapeId()` |
| `parentId` | Parent shape or page ID | Defines hierarchy |
| `index` | Z-order within siblings | Lexicographic index (e.g., "a1", "a2", "b1") |
| `type` | Shape type discriminator | Matches a registered ShapeUtil |
| `x`, `y` | Position in parent space | Relative to parent, not page |
| `rotation` | Rotation in radians | Positive = clockwise |
| `opacity` | Opacity multiplier | Applied to entire shape tree |
| `props` | Shape-specific data | Defined by ShapeUtil's `props` schema |
| `meta` | Arbitrary metadata | Persisted, not used by engine |

### 1.2 Shape Records and Schema

Shapes are stored in the centralized `TLStore` using a schema-driven approach:

```typescript
// packages/tldraw/src/lib/store/TLStore.ts
const shapeSchema = defineSchema({
  shape: defineStore({
    validations: {
      id: 'shapeId',
      parentId: 'string',
      index: 'string',
      type: 'string',
      x: 'number',
      y: 'number',
      rotation: 'number',
      opacity: 'number',
      groupId: 'nullable(shapeId)',
      props: 'object',
      meta: 'object',
    },
    // Migration support
    migrations: {
      // Shape migrations are versioned
      '1.0.0': (shape) => shape,
      '2.0.0': (shape) => ({ ...shape, opacity: shape.opacity ?? 1 }),
    },
  }),
})
```

**Store Operations:**

```typescript
// Create shapes
editor.createShapes([{
  type: 'geo',
  x: 100,
  y: 100,
  props: { geo: 'rectangle', w: 200, h: 100 },
}])

// Update shapes
editor.updateShapes([{
  id: shapeId,
  x: 150,
  props: { w: 250 },
}])

// Delete shapes
editor.deleteShapes([shapeId])

// Get shape
const shape = editor.getShape(shapeId)

// Get all shapes in current page
const shapes = editor.getCurrentPageShapes()
```

### 1.3 Shape Props

Each shape type defines its own `props` schema. Props are validated and migrated by the ShapeUtil:

```typescript
// Example: GeoShape props definition
interface TLGeoShapeProps {
  geo: 'rectangle' | 'ellipse' | 'triangle' | 'diamond' | 'star' | 'polygon'
  w: number
  h: number
  fill: FillStyle
  fillStyle: FillStyleType
  color: ColorKey
  dash: DashStyle
  size: SizeStyle
  text: string  // For labeled shapes
  font: FontFamily
  align: TextAlign
  verticalAlign: VerticalAlignStyle
}

// Props are validated on creation/update
const geoValidator: Validator<TLGeoShapeProps> = object({
  geo: oneOf('rectangle', 'ellipse', 'triangle', 'diamond', 'star', 'polygon'),
  w: number,
  h: number,
  fill: fillValidator,
  color: colorValidator,
  dash: dashValidator,
  size: sizeValidator,
  text: string,
  font: fontValidator,
  align: textAlignValidator,
  verticalAlign: verticalAlignValidator,
})
```

### 1.4 Meta Properties

The `meta` field stores arbitrary user data that persists with the shape:

```typescript
interface TLShape {
  meta: Record<string, unknown>
}

// Example: Store custom metadata
editor.updateShapes([{
  id: shapeId,
  meta: {
    createdBy: 'user-123',
    tags: ['important', 'review'],
    customData: { any: 'value' },
  },
}])

// Meta is never used by the engine internally
// It's purely for application-specific needs
```

### 1.5 Shape Hierarchy

Shapes form a tree structure via `parentId`:

```typescript
// Shape hierarchy example
Page
├── Frame (id: frame1)
│   ├── Rectangle (id: rect1, parentId: frame1)
│   ├── Text (id: text1, parentId: frame1)
│   └── Group (id: group1, parentId: frame1)
│       ├── Circle (id: circle1, parentId: group1)
│       └── Arrow (id: arrow1, parentId: group1)
└── Arrow (id: arrow2, parentId: pageId)

// Get parent shape
const parent = editor.getShape(shape.parentId)

// Get children
const children = editor.getShapeChildren(shape.id)

// Get all descendants (recursive)
const descendants = editor.getShapeAndDescendantIds([shape.id])

// Check if shape is descendant of another
const isDescendant = editor.isShapeDescendant(childId, parentId)
```

**Page Transform vs Local Transform:**

```typescript
// Local transform (relative to parent)
const localTransform = Mat.Compose(
  Mat.Translate(shape.x, shape.y),
  Mat.Rotate(shape.rotation)
)

// Page transform (absolute, includes all ancestors)
const pageTransform = editor.getShapePageTransform(shape.id)
// This recursively composes all parent transforms

// Convert local point to page space
const localPoint = { x: 50, y: 50 }
const pagePoint = pageTransform.applyToPoint(localPoint)

// Convert page point to local space
const pagePoint = { x: 200, y: 150 }
const localPoint = pageTransform.clone().invert().applyToPoint(pagePoint)
```

---

## 2. ShapeUtil Base Class

### 2.1 Shape Definition

Every shape type has a corresponding `ShapeUtil` class that defines its behavior:

```typescript
// packages/tldraw/src/lib/shapes/ShapeUtil.ts
abstract class ShapeUtil<T extends TLShape = TLShape> {
  // Static properties
  static type: string  // Shape type identifier
  static props?: object  // Props schema definition
  static snapshot?: object  // Snapshot for undo/redo

  // Editor reference
  readonly editor: Editor

  constructor(editor: Editor) {
    this.editor = editor
  }

  // ============ REQUIRED METHODS ============

  // Create a new shape instance
  abstract create(props: Partial<T>): T

  // Return the shape's geometry for hit-testing and bounds
  abstract geometry(shape: T): Geometry2d

  // ============ RENDERING (SVG) ============

  // Main SVG component
  component(shape: T): JSX.Element

  // Optional background component (rendered below other shapes)
  backgroundComponent?(shape: T): JSX.Element

  // ============ INTERACTION ============

  // Handle pointer events
  onPointerDown?(shape: T, info: TLPointerEventInfo): void
  onPointerMove?(shape: T, info: TLPointerEventInfo): void
  onPointerUp?(shape: T, info: TLPointerEventInfo): void

  // Handle keyboard events
  onKeyDown?(shape: T, info: TLKeyboardEventInfo): void
  onKeyUp?(shape: T, info: TLKeyboardEventInfo): void

  // Handle drag operations
  onDragStart?(shape: T, info: TLDragEventInfo): void
  onDrag?(shape: T, info: TLDragEventInfo): void
  onDragEnd?(shape: T, info: TLDragEventInfo): void

  // ============ HANDLES ============

  // Return interactive handles for the shape
  getHandles?(shape: T): TLHandle[]

  // Handle handle dragging
  onHandleDrag?(shape: T, info: TLHandleDragEventInfo): T

  // ============ BINDINGS ============

  // Return binding points for connectors
  getBindingPoints?(shape: T): TLBindingPoint[]

  // ============ MIGRATION ============

  // Migrate shape from older versions
  migrate?(shape: TLShape): T

  // Upgrade props during migration
  upgrade?(shape: TLShape): Partial<T>
}
```

### 2.2 Geometry (Bounds, Outline, Snap Points)

The `geometry()` method returns a `Geometry2d` object that defines the shape's spatial properties:

```typescript
// packages/tldraw/src/lib/geometry/Geometry2d.ts
abstract class Geometry2d {
  // Bounding box in shape's local space
  abstract readonly bounds: Box

  // Outline path for hit-testing
  abstract readonly outline: Vec[]

  // Snap points for alignment
  abstract readonly snapPoints: SnapPoint[]

  // ============ METHODS ============

  // Check if point is inside shape
  containsPoint(point: VecLike, tolerance?: number): boolean

  // Check if shape intersects another
  intersects(geometry: Geometry2d): boolean

  // Get nearest point on outline
  nearestPoint(point: VecLike): Vec

  // Get distance from point to shape
  distanceToPoint(point: VecLike): number

  // Clip a line segment to the shape boundary
  clipSegment(segment: Vec[]): Vec[]
}
```

**Example: Rectangle Geometry:**

```typescript
class RectangleGeometry extends Geometry2d {
  constructor(private w: number, private h: number) {
    super()
  }

  readonly bounds = new Box(0, 0, this.w, this.h)

  readonly outline = [
    new Vec(0, 0),
    new Vec(this.w, 0),
    new Vec(this.w, this.h),
    new Vec(0, this.h),
    new Vec(0, 0),  // Closed path
  ]

  readonly snapPoints = [
    // Corners
    { id: 'tl', x: 0, y: 0 },
    { id: 'tr', x: this.w, y: 0 },
    { id: 'br', x: this.w, y: this.h },
    { id: 'bl', x: 0, y: this.h },
    // Edge midpoints
    { id: 't', x: this.w / 2, y: 0 },
    { id: 'r', x: this.w, y: this.h / 2 },
    { id: 'b', x: this.w / 2, y: this.h },
    { id: 'l', x: 0, y: this.h / 2 },
    // Center
    { id: 'c', x: this.w / 2, y: this.h / 2 },
  ]

  containsPoint(point: VecLike, tolerance = 0): boolean {
    return (
      point.x >= -tolerance &&
      point.x <= this.w + tolerance &&
      point.y >= -tolerance &&
      point.y <= this.h + tolerance
    )
  }
}
```

**Example: Ellipse Geometry:**

```typescript
class EllipseGeometry extends Geometry2d {
  constructor(private w: number, private h: number) {
    super()
  }

  readonly bounds = new Box(0, 0, this.w, this.h)

  readonly outline = this.computeEllipsePoints()

  private computeEllipsePoints(): Vec[] {
    const points: Vec[] = []
    const rx = this.w / 2
    const ry = this.h / 2
    const cx = rx
    const cy = ry

    // Generate ellipse points
    for (let i = 0; i <= 64; i++) {
      const t = (i / 64) * Math.PI * 2
      points.push(
        new Vec(
          cx + rx * Math.cos(t),
          cy + ry * Math.sin(t)
        )
      )
    }

    return points
  }

  readonly snapPoints = [
    // Cardinal points
    { id: 'n', x: this.w / 2, y: 0 },
    { id: 'e', x: this.w, y: this.h / 2 },
    { id: 's', x: this.w / 2, y: this.h },
    { id: 'w', x: 0, y: this.h / 2 },
    // Center
    { id: 'c', x: this.w / 2, y: this.h / 2 },
  ]

  containsPoint(point: VecLike, tolerance = 0): boolean {
    const rx = this.w / 2
    const ry = this.h / 2
    const cx = rx
    const cy = ry

    // Ellipse equation: (x-cx)^2/rx^2 + (y-cy)^2/ry^2 <= 1
    const value =
      Math.pow(point.x - cx, 2) / Math.pow(rx, 2) +
      Math.pow(point.y - cy, 2) / Math.pow(ry, 2)

    return value <= 1 + tolerance
  }
}
```

### 2.3 Rendering (SVG, HTML)

**SVG Rendering:**

```typescript
// BaseBoxShapeUtil - Common base for box-shaped shapes
abstract class BaseBoxShapeUtil<T extends TLShape & { w: number; h: number }> 
  extends ShapeUtil<T> 
{
  component(shape: T): JSX.Element {
    const { w, h } = shape
    
    return (
      <svg width={w} height={h} overflow="visible">
        {/* Shape outline */}
        {this.renderOutline(shape)}
        
        {/* Fill */}
        {this.renderFill(shape)}
        
        {/* Text label if present */}
        {this.renderText(shape)}
        
        {/* Custom decorations */}
        {this.renderDecorations(shape)}
      </svg>
    )
  }

  protected renderOutline(shape: T): JSX.Element | null {
    const { w, h, dash } = shape.props
    const strokeWidth = this.getStrokeWidth(shape.props.size)
    
    switch (shape.props.geo) {
      case 'rectangle':
        return (
          <rect
            x={strokeWidth / 2}
            y={strokeWidth / 2}
            width={w - strokeWidth}
            height={h - strokeWidth}
            fill="none"
            stroke={shape.props.color}
            strokeWidth={strokeWidth}
            strokeDasharray={this.getDashPattern(dash)}
          />
        )
      case 'ellipse':
        return (
          <ellipse
            cx={w / 2}
            cy={h / 2}
            rx={w / 2 - strokeWidth / 2}
            ry={h / 2 - strokeWidth / 2}
            fill="none"
            stroke={shape.props.color}
            strokeWidth={strokeWidth}
            strokeDasharray={this.getDashPattern(dash)}
          />
        )
      // ... other shapes
    }
  }

  protected renderFill(shape: T): JSX.Element | null {
    if (!shape.props.fill || shape.props.fill === 'none') return null
    
    const { w, h } = shape
    // Similar to outline but with fill attribute
  }

  protected renderText(shape: T): JSX.Element | null {
    if (!shape.props.text) return null
    
    const { w, h } = shape
    const { text, font, align, verticalAlign } = shape.props
    
    // Calculate text position based on alignment
    const x = this.getTextX(w, align)
    const y = this.getTextY(h, verticalAlign)
    
    return (
      <text
        x={x}
        y={y}
        fontFamily={font}
        textAnchor={align}
        dominantBaseline={verticalAlign}
        style={{ userSelect: 'none' }}
      >
        {text}
      </text>
    )
  }
}
```

**HTML Rendering (for interactive elements):**

```typescript
// Text shape uses HTML for editing
class TextShapeUtil extends ShapeUtil<TLTextShape> {
  component(shape: TLTextShape): JSX.Element {
    return (
      <foreignObject width={shape.w} height={shape.h}>
        <div
          className="tl-text-shape"
          contentEditable={this.editor.getEditingShapeId() === shape.id}
          suppressContentEditableWarning
          style={{
            width: '100%',
            height: '100%',
            fontFamily: shape.props.font,
            fontSize: shape.props.size,
            textAlign: shape.props.align,
          }}
          onInput={(e) => this.handleTextChange(shape.id, e.currentTarget.textContent)}
        >
          {shape.props.text}
        </div>
      </foreignObject>
    )
  }

  // HTML overlay for precise text editing
  htmlComponent?(shape: TLTextShape): JSX.Element {
    if (this.editor.getEditingShapeId() !== shape.id) return null
    
    // Full HTML editor overlay when shape is being edited
    return (
      <textarea
        className="tl-text-editor-overlay"
        value={shape.props.text}
        onChange={(e) => this.handleTextChange(shape.id, e.target.value)}
        onBlur={() => this.editor.setEditingShape(null)}
        autoFocus
      />
    )
  }
}
```

### 2.4 Events (Pointer, Keyboard, Drag)

**Pointer Events:**

```typescript
class SelectableShapeUtil extends ShapeUtil<TLShape> {
  onPointerDown(shape: T, info: TLPointerEventInfo): void {
    const { point, shiftKey, altKey, ctrlKey } = info
    
    // Check if clicking on a handle
    const handle = this.getHandleAtPoint(shape, point)
    if (handle) {
      this.editor.dispatch({
        type: 'handle',
        target: 'handle',
        name: 'pointer_down',
        shapeId: shape.id,
        handleId: handle.id,
        ...info,
      })
      return
    }
    
    // Check for selection
    const isSelected = this.editor.getSelectedShapeIds().includes(shape.id)
    
    if (!isSelected) {
      // Select on click
      if (shiftKey) {
        // Add to selection
        this.editor.select(...this.editor.getSelectedShapeIds(), shape.id)
      } else {
        // Replace selection
        this.editor.select(shape.id)
      }
    }
    
    // Start drag operation
    this.editor.dispatch({
      type: 'shape',
      target: 'shape',
      name: 'pointer_down',
      shapeId: shape.id,
      ...info,
    })
  }

  onPointerMove(shape: T, info: TLPointerEventInfo): void {
    // Update hover state
    this.editor.setHoveredShape(shape.id)
    
    // Handle ongoing drag
    if (this.editor.inputs.isDragging) {
      this.onDrag?.(shape, info as TLDragEventInfo)
    }
  }

  onPointerUp(shape: T, info: TLPointerEventInfo): void {
    // Complete any drag operation
    this.onDragEnd?.(shape, info as TLDragEventInfo)
  }
}
```

**Keyboard Events:**

```typescript
class EditableShapeUtil extends ShapeUtil<TLShape> {
  onKeyDown(shape: T, info: TLKeyboardEventInfo): void {
    const { code, key } = info
    
    switch (code) {
      case 'Enter':
      case 'F2':
        // Enter edit mode
        this.editor.setEditingShape(shape.id)
        break
        
      case 'Delete':
      case 'Backspace':
        // Delete shape
        if (this.editor.getSelectedShapeIds().includes(shape.id)) {
          this.editor.deleteShapes([shape.id])
        }
        break
        
      case 'Escape':
        // Cancel editing
        this.editor.setEditingShape(null)
        break
        
      // Arrow keys for nudging
      case 'ArrowUp':
      case 'ArrowDown':
      case 'ArrowLeft':
      case 'ArrowRight':
        this.handleNudge(shape, code, info.shiftKey)
        break
    }
  }

  private handleNudge(shape: T, code: string, isShift: boolean): void {
    const delta = isShift ? 10 : 1  // Shift = large nudge
    
    const updates: Partial<TLShape> = { id: shape.id }
    
    switch (code) {
      case 'ArrowUp':
        updates.y = shape.y - delta
        break
      case 'ArrowDown':
        updates.y = shape.y + delta
        break
      case 'ArrowLeft':
        updates.x = shape.x - delta
        break
      case 'ArrowRight':
        updates.x = shape.x + delta
        break
    }
    
    this.editor.updateShapes([updates])
  }
}
```

**Drag Events:**

```typescript
class DraggableShapeUtil extends ShapeUtil<TLShape> {
  onDragStart(shape: T, info: TLDragEventInfo): void {
    // Store initial state for potential rollback
    this._initialState = {
      x: shape.x,
      y: shape.y,
    }
    
    // Snap preview if snapping enabled
    if (this.editor.getSnappingOptions().isSnapMode) {
      this.showSnapPreview(shape, info)
    }
  }

  onDrag(shape: T, info: TLDragEventInfo): void {
    const delta = Vec.Sub(info.point, info.originalPoint)
    
    // Handle multi-shape selection drag
    const selectedShapeIds = this.editor.getSelectedShapeIds()
    
    if (selectedShapeIds.length > 1 && selectedShapeIds.includes(shape.id)) {
      // Drag all selected shapes
      this.dragMultipleShapes(delta)
    } else {
      // Drag single shape
      this.editor.updateShapes([{
        id: shape.id,
        x: this._initialState.x + delta.x,
        y: this._initialState.y + delta.y,
      }])
    }
    
    // Update snap preview
    if (this.editor.getSnappingOptions().isSnapMode) {
      this.updateSnapPreview(shape, info)
    }
  }

  onDragEnd(shape: T, info: TLDragEventInfo): void {
    // Hide snap preview
    this.hideSnapPreview()
    
    // Snap to grid if enabled
    if (this.editor.getGridOptions().size > 0) {
      this.snapToGrid(shape)
    }
    
    // Complete the operation
    this.editor.completeDragOperation()
  }

  private dragMultipleShapes(delta: VecLike): void {
    const selectedShapeIds = this.editor.getSelectedShapeIds()
    
    this.editor.updateShapes(
      selectedShapeIds.map((id) => ({
        id,
        x: this.editor.getShape(id)!.x + delta.x,
        y: this.editor.getShape(id)!.y + delta.y,
      }))
    )
  }

  private snapToGrid(shape: T): void {
    const gridSize = this.editor.getGridOptions().size
    const currentShape = this.editor.getShape(shape.id)!
    
    this.editor.updateShapes([{
      id: shape.id,
      x: Math.round(currentShape.x / gridSize) * gridSize,
      y: Math.round(currentShape.y / gridSize) * gridSize,
    }])
  }
}
```

### 2.5 Migration (Versions, Updates)

Shape migrations handle schema evolution across versions:

```typescript
// packages/tldraw/src/lib/shapes/geo/GeoShapeUtil.ts
class GeoShapeUtil extends BaseBoxShapeUtil<TLGeoShape> {
  static override type = 'geo' as const

  // Migration function
  migrate(shape: TLShape): TLGeoShape {
    // Handle shape migration from older versions
    let migrated = { ...shape } as TLGeoShape
    
    // Migration: Add opacity if missing (v1 -> v2)
    if (migrated.opacity === undefined) {
      migrated.opacity = 1
    }
    
    // Migration: Normalize geo type (v2 -> v3)
    if ((migrated.props as any).type) {
      migrated.props.geo = this.normalizeGeoType((migrated.props as any).type)
      delete (migrated.props as any).type
    }
    
    // Migration: Add default text (v3 -> v4)
    if (!migrated.props.text) {
      migrated.props.text = ''
    }
    
    return migrated
  }

  // Props upgrade during schema changes
  upgrade(shape: TLShape): Partial<TLGeoShape> {
    const upgrades: Partial<TLGeoShape> = {}
    
    // Example: Convert old color format to new
    if ((shape.props as any).color?.startsWith('#')) {
      upgrades.props = {
        ...shape.props,
        color: this.migrateColor((shape.props as any).color),
      }
    }
    
    return upgrades
  }

  // Snapshot for undo/redo
  snapshot(shape: TLGeoShape): TLGeoShape {
    return {
      ...shape,
      // Deep copy props to avoid mutation issues
      props: { ...shape.props },
    }
  }

  private normalizeGeoType(type: string): TLGeoShape['geo'] {
    const mapping: Record<string, TLGeoShape['geo']> = {
      'rect': 'rectangle',
      'circle': 'ellipse',
      'triangle': 'triangle',
      // ...
    }
    return mapping[type] || 'rectangle'
  }

  private migrateColor(oldColor: string): string {
    // Convert hex to design token color
    return this.colorMap[oldColor.toLowerCase()] || 'black'
  }
}
```

**Schema Version Migration:**

```typescript
// Store-level migrations
const storeMigrations = {
  '0.0.0': (store) => store,  // Initial version
  
  '1.0.0': (store) => ({
    ...store,
    // Add shape records
    shape: defineStore({ ... }),
  }),
  
  '2.0.0': (store) => {
    // Migrate all shapes to new schema
    const shapes = Object.values(store.records.shape)
    const migratedShapes = shapes.map(shape => ({
      ...shape,
      opacity: shape.opacity ?? 1,  // Add default opacity
    }))
    
    return {
      ...store,
      records: {
        ...store.records,
        shape: Object.fromEntries(
          migratedShapes.map(s => [s.id, s])
        ),
      },
    }
  },
  
  '3.0.0': (store) => {
    // Add meta field to all shapes
    const shapes = Object.values(store.records.shape)
    const migratedShapes = shapes.map(shape => ({
      ...shape,
      meta: shape.meta ?? {},
    }))
    
    return {
      ...store,
      records: {
        ...store.records,
        shape: Object.fromEntries(
          migratedShapes.map(s => [s.id, s])
        ),
      },
    }
  },
}
```

---

## 3. Default Shapes

### 3.1 Geometric Shapes (Rectangle, Ellipse, Triangle, Diamond, Star, Polygon)

**Rectangle:**

```typescript
class RectangleShapeUtil extends BaseBoxShapeUtil<TLGeoShape> {
  static override type = 'geo' as const

  create(props: Partial<TLGeoShape>): TLGeoShape {
    return {
      id: createShapeId(),
      type: 'geo',
      x: props.x ?? 0,
      y: props.y ?? 0,
      w: props.w ?? 100,
      h: props.h ?? 100,
      rotation: props.rotation ?? 0,
      opacity: props.opacity ?? 1,
      parentId: this.editor.getCurrentPageId(),
      index: getIndexAbove(this.editor.getMaxIndex()),
      groupId: null,
      props: {
        geo: 'rectangle',
        w: props.w ?? 100,
        h: props.h ?? 100,
        fill: props.fill ?? 'semi',
        fillStyle: props.fillStyle ?? 'solid',
        color: props.color ?? 'black',
        dash: props.dash ?? 'solid',
        size: props.size ?? 'm',
        text: props.text ?? '',
        font: props.font ?? 'sans',
        align: props.align ?? 'middle',
        verticalAlign: props.verticalAlign ?? 'middle',
      },
      meta: {},
    }
  }

  geometry(shape: TLGeoShape): Geometry2d {
    return new RectangleGeometry(shape.w, shape.h)
  }

  component(shape: TLGeoShape): JSX.Element {
    const { w, h } = shape
    const strokeWidth = this.getStrokeWidth(shape.props.size)
    
    return (
      <svg width={w} height={h}>
        {shape.props.fill !== 'none' && (
          <rect
            x={0}
            y={0}
            width={w}
            height={h}
            fill={this.getColor(shape.props.color)}
            fillOpacity={this.getFillOpacity(shape.props.fill)}
          />
        )}
        <rect
          x={strokeWidth / 2}
          y={strokeWidth / 2}
          width={w - strokeWidth}
          height={h - strokeWidth}
          fill="none"
          stroke={this.getColor(shape.props.color)}
          strokeWidth={strokeWidth}
          strokeDasharray={this.getDashPattern(shape.props.dash)}
        />
        {shape.props.text && (
          <TextInShape shape={shape} />
        )}
      </svg>
    )
  }
}
```

**Ellipse:**

```typescript
class EllipseShapeUtil extends BaseBoxShapeUtil<TLGeoShape> {
  geometry(shape: TLGeoShape): Geometry2d {
    return new EllipseGeometry(shape.w, shape.h)
  }

  component(shape: TLGeoShape): JSX.Element {
    const { w, h } = shape
    const strokeWidth = this.getStrokeWidth(shape.props.size)
    
    return (
      <svg width={w} height={h}>
        {shape.props.fill !== 'none' && (
          <ellipse
            cx={w / 2}
            cy={h / 2}
            rx={w / 2}
            ry={h / 2}
            fill={this.getColor(shape.props.color)}
            fillOpacity={this.getFillOpacity(shape.props.fill)}
          />
        )}
        <ellipse
          cx={w / 2}
          cy={h / 2}
          rx={w / 2 - strokeWidth / 2}
          ry={h / 2 - strokeWidth / 2}
          fill="none"
          stroke={this.getColor(shape.props.color)}
          strokeWidth={strokeWidth}
          strokeDasharray={this.getDashPattern(shape.props.dash)}
        />
        {shape.props.text && (
          <TextInShape shape={shape} />
        )}
      </svg>
    )
  }
}
```

**Triangle:**

```typescript
class TriangleShapeUtil extends BaseBoxShapeUtil<TLGeoShape> {
  geometry(shape: TLGeoShape): Geometry2d {
    return new TriangleGeometry(shape.w, shape.h)
  }

  component(shape: TLGeoShape): JSX.Element {
    const { w, h } = shape
    const strokeWidth = this.getStrokeWidth(shape.props.size)
    
    // Triangle points: top, bottom-right, bottom-left
    const points = `
      ${w / 2},0
      ${w},${h}
      0,${h}
    `
    
    return (
      <svg width={w} height={h}>
        {shape.props.fill !== 'none' && (
          <polygon
            points={points}
            fill={this.getColor(shape.props.color)}
            fillOpacity={this.getFillOpacity(shape.props.fill)}
          />
        )}
        <polygon
          points={points}
          fill="none"
          stroke={this.getColor(shape.props.color)}
          strokeWidth={strokeWidth}
          strokeDasharray={this.getDashPattern(shape.props.dash)}
        />
        {shape.props.text && (
          <TextInShape shape={shape} alignment={{ x: 0.5, y: 0.6 }} />
        )}
      </svg>
    )
  }
}

class TriangleGeometry extends Geometry2d {
  constructor(private w: number, private h: number) {
    super()
  }

  readonly bounds = new Box(0, 0, this.w, this.h)

  readonly outline = [
    new Vec(this.w / 2, 0),
    new Vec(this.w, this.h),
    new Vec(0, this.h),
    new Vec(this.w / 2, 0),
  ]

  readonly snapPoints = [
    { id: 'top', x: this.w / 2, y: 0 },
    { id: 'br', x: this.w, y: this.h },
    { id: 'bl', x: 0, y: this.h },
    { id: 'center', x: this.w / 2, y: this.h * 0.6 },  // Centroid approx
  ]

  containsPoint(point: VecLike, tolerance = 0): boolean {
    // Barycentric coordinate test
    const [A, B, C] = this.outline
    return pointInTriangle(point, A, B, C, tolerance)
  }
}
```

**Diamond:**

```typescript
class DiamondShapeUtil extends BaseBoxShapeUtil<TLGeoShape> {
  geometry(shape: TLGeoShape): Geometry2d {
    return new DiamondGeometry(shape.w, shape.h)
  }

  component(shape: TLGeoShape): JSX.Element {
    const { w, h } = shape
    const strokeWidth = this.getStrokeWidth(shape.props.size)
    
    // Diamond points: top, right, bottom, left
    const points = `
      ${w / 2},0
      ${w},${h / 2}
      ${w / 2},${h}
      0,${h / 2}
    `
    
    return (
      <svg width={w} height={h}>
        {shape.props.fill !== 'none' && (
          <polygon
            points={points}
            fill={this.getColor(shape.props.color)}
            fillOpacity={this.getFillOpacity(shape.props.fill)}
          />
        )}
        <polygon
          points={points}
          fill="none"
          stroke={this.getColor(shape.props.color)}
          strokeWidth={strokeWidth}
          strokeDasharray={this.getDashPattern(shape.props.dash)}
        />
      </svg>
    )
  }
}
```

**Star:**

```typescript
class StarShapeUtil extends BaseBoxShapeUtil<TLGeoShape> {
  geometry(shape: TLGeoShape): Geometry2d {
    return new StarGeometry(shape.w, shape.h, shape.props.sides ?? 5)
  }

  component(shape: TLGeoShape): JSX.Element {
    const { w, h } = shape
    const sides = shape.props.sides ?? 5
    const strokeWidth = this.getStrokeWidth(shape.props.size)
    
    const points = this.computeStarPoints(w, h, sides)
    
    return (
      <svg width={w} height={h}>
        {shape.props.fill !== 'none' && (
          <polygon
            points={points.join(' ')}
            fill={this.getColor(shape.props.color)}
            fillOpacity={this.getFillOpacity(shape.props.fill)}
          />
        )}
        <polygon
          points={points.join(' ')}
          fill="none"
          stroke={this.getColor(shape.props.color)}
          strokeWidth={strokeWidth}
          strokeDasharray={this.getDashPattern(shape.props.dash)}
        />
      </svg>
    )
  }

  private computeStarPoints(w: number, h: number, sides: number): string[] {
    const points: string[] = []
    const cx = w / 2
    const cy = h / 2
    const outerR = Math.min(w, h) / 2
    const innerR = outerR * 0.4  // Inner radius ratio
    
    for (let i = 0; i < sides * 2; i++) {
      const angle = (i * Math.PI) / sides - Math.PI / 2
      const r = i % 2 === 0 ? outerR : innerR
      const x = cx + r * Math.cos(angle)
      const y = cy + r * Math.sin(angle)
      points.push(`${x},${y}`)
    }
    
    return points
  }
}
```

**Polygon:**

```typescript
class PolygonShapeUtil extends BaseBoxShapeUtil<TLGeoShape> {
  geometry(shape: TLGeoShape): Geometry2d {
    return new PolygonGeometry(shape.w, shape.h, shape.props.sides ?? 3)
  }

  component(shape: TLGeoShape): JSX.Element {
    const { w, h, sides = 3 } = shape
    const strokeWidth = this.getStrokeWidth(shape.props.size)
    
    const points = this.computePolygonPoints(w, h, sides)
    
    return (
      <svg width={w} height={h}>
        {shape.props.fill !== 'none' && (
          <polygon
            points={points.join(' ')}
            fill={this.getColor(shape.props.color)}
            fillOpacity={this.getFillOpacity(shape.props.fill)}
          />
        )}
        <polygon
          points={points.join(' ')}
          fill="none"
          stroke={this.getColor(shape.props.color)}
          strokeWidth={strokeWidth}
          strokeDasharray={this.getDashPattern(shape.props.dash)}
        />
      </svg>
    )
  }

  private computePolygonPoints(w: number, h: number, sides: number): string[] {
    const points: string[] = []
    const cx = w / 2
    const cy = h / 2
    const r = Math.min(w, h) / 2
    
    for (let i = 0; i < sides; i++) {
      const angle = (i * 2 * Math.PI) / sides - Math.PI / 2
      const x = cx + r * Math.cos(angle)
      const y = cy + r * Math.sin(angle)
      points.push(`${x},${y}`)
    }
    
    return points
  }
}
```

### 3.2 Drawing Tools (Draw, Pencil, Highlighter, Eraser)

**Draw Shape:**

```typescript
interface TLDrawShape extends TLShape {
  type: 'draw'
  props: {
    segments: DrawSegment[]
    isComplete: boolean
    color: ColorKey
    size: SizeStyle
    dash: DashStyle
  }
}

interface DrawSegment {
  type: 'free' | 'straight'
  points: Vec[]
}

class DrawShapeUtil extends ShapeUtil<TLDrawShape> {
  static override type = 'draw' as const

  create(props: Partial<TLDrawShape>): TLDrawShape {
    return {
      id: createShapeId(),
      type: 'draw',
      x: props.x ?? 0,
      y: props.y ?? 0,
      rotation: 0,
      opacity: 1,
      parentId: this.editor.getCurrentPageId(),
      index: getIndexAbove(this.editor.getMaxIndex()),
      groupId: null,
      props: {
        segments: props.segments ?? [],
        isComplete: props.isComplete ?? false,
        color: props.color ?? 'black',
        size: props.size ?? 'm',
        dash: props.dash ?? 'solid',
      },
      meta: {},
    }
  }

  geometry(shape: TLDrawShape): Geometry2d {
    return new DrawGeometry(shape.props.segments)
  }

  component(shape: TLDrawShape): JSX.Element {
    const strokeWidth = this.getStrokeWidth(shape.props.size)
    const color = this.getColor(shape.props.color)
    const dashArray = this.getDashPattern(shape.props.dash)
    
    const paths = shape.props.segments.map((segment) => {
      if (segment.type === 'free') {
        // Smooth freehand path using Catmull-Rom spline
        const d = this.smoothPath(segment.points)
        return (
          <path
            key={segment.points.join()}
            d={d}
            fill="none"
            stroke={color}
            strokeWidth={strokeWidth}
            strokeDasharray={dashArray}
            strokeLinecap="round"
            strokeLinejoin="round"
          />
        )
      } else {
        // Straight line segments
        const d = this.linePath(segment.points)
        return (
          <path
            key={segment.points.join()}
            d={d}
            fill="none"
            stroke={color}
            strokeWidth={strokeWidth}
            strokeDasharray={dashArray}
            strokeLinecap="round"
            strokeLinejoin="round"
          />
        )
      }
    })
    
    return <svg>{paths}</svg>
  }

  private smoothPath(points: Vec[]): string {
    if (points.length < 2) return ''
    
    // Catmull-Rom spline interpolation
    let d = `M ${points[0].x} ${points[0].y}`
    
    for (let i = 0; i < points.length - 1; i++) {
      const p0 = points[Math.max(0, i - 1)]
      const p1 = points[i]
      const p2 = points[i + 1]
      const p3 = points[Math.min(i + 2, points.length - 1)]
      
      // Calculate control points
      const cp1x = p1.x + (p2.x - p0.x) / 6
      const cp1y = p1.y + (p2.y - p0.y) / 6
      const cp2x = p2.x - (p3.x - p1.x) / 6
      const cp2y = p2.y - (p3.y - p1.y) / 6
      
      d += ` C ${cp1x},${cp1y} ${cp2x},${cp2y} ${p2.x},${p2.y}`
    }
    
    return d
  }

  private linePath(points: Vec[]): string {
    return `M ${points.map((p) => `${p.x} ${p.y}`).join(' L ')}`
  }
}
```

**Pencil Shape:**

```typescript
// Pencil is similar to draw but with different smoothing
class PencilShapeUtil extends ShapeUtil<TLDrawShape> {
  static override type = 'pencil' as const

  // Shares implementation with DrawShapeUtil but uses different smoothing
  private smoothPath(points: Vec[]): string {
    // Lighter smoothing for pencil effect
    return this.lighterSmoothPath(points)
  }
}
```

**Highlighter Shape:**

```typescript
class HighlighterShapeUtil extends ShapeUtil<TLDrawShape> {
  static override type = 'highlighter' as const

  component(shape: TLDrawShape): JSX.Element {
    const strokeWidth = this.getStrokeWidth(shape.props.size) * 2  // Thicker
    const color = this.getColor(shape.props.color)
    
    return (
      <svg>
        {shape.props.segments.map((segment) => (
          <path
            key={segment.points.join()}
            d={this.smoothPath(segment.points)}
            fill="none"
            stroke={color}
            strokeWidth={strokeWidth}
            strokeOpacity={0.3}  // Transparent highlight
            strokeLinecap="round"
            strokeLinejoin="round"
            style={{ mixBlendMode: 'multiply' }}
          />
        ))}
      </svg>
    )
  }
}
```

**Eraser:**

```typescript
// Eraser doesn't create shapes - it deletes them
class EraserTool extends StateNode {
  static override id = 'eraser' as const

  override onPointerDown = (info: TLPointerEventInfo) => {
    const shape = this.editor.getShapeAtPoint(info.point)
    
    if (shape) {
      // Mark for erasure
      this.editor.setErasingShapes([shape.id])
      this.parent.transition('erasing', info)
    }
  }

  override onPointerMove = (info: TLPointerEventInfo) => {
    const shape = this.editor.getShapeAtPoint(info.point)
    
    if (shape && !this.editor.getErasingShapes().includes(shape.id)) {
      this.editor.setErasingShapes([...this.editor.getErasingShapes(), shape.id])
    }
  }

  override onPointerUp = () => {
    // Delete all marked shapes
    const erasingShapeIds = this.editor.getErasingShapes()
    if (erasingShapeIds.length > 0) {
      this.editor.deleteShapes(erasingShapeIds)
    }
    this.editor.setErasingShapes([])
    this.parent.transition('idle')
  }
}
```

### 3.3 Text Shapes (Text, Sticky)

**Text Shape:**

```typescript
interface TLTextShape extends TLShape {
  type: 'text'
  props: {
    text: string
    font: FontFamily
    size: number
    color: ColorKey
    align: TextAlign
    w: number
    h: number
  }
}

class TextShapeUtil extends ShapeUtil<TLTextShape> {
  static override type = 'text' as const

  create(props: Partial<TLTextShape>): TLTextShape {
    // Measure text to get dimensions
    const metrics = this.measureText(props.text ?? '', props)
    
    return {
      id: createShapeId(),
      type: 'text',
      x: props.x ?? 0,
      y: props.y ?? 0,
      rotation: 0,
      opacity: 1,
      parentId: this.editor.getCurrentPageId(),
      index: getIndexAbove(this.editor.getMaxIndex()),
      groupId: null,
      props: {
        text: props.text ?? '',
        font: props.font ?? 'sans',
        size: props.size ?? 20,
        color: props.color ?? 'black',
        align: props.align ?? 'start',
        w: metrics.width,
        h: metrics.height,
      },
      meta: {},
    }
  }

  geometry(shape: TLTextShape): Geometry2d {
    return new RectangleGeometry(shape.props.w, shape.props.h)
  }

  component(shape: TLTextShape): JSX.Element {
    const { text, font, size, color, align } = shape.props
    
    return (
      <foreignObject width={shape.props.w} height={shape.props.h}>
        <div
          className="tl-text-shape"
          style={{
            fontFamily: font,
            fontSize: size,
            color: this.getColor(color),
            textAlign: align,
            whiteSpace: 'pre-wrap',
            userSelect: 'none',
          }}
        >
          {text}
        </div>
      </foreignObject>
    )
  }

  // HTML component for editing
  htmlComponent?(shape: TLTextShape): JSX.Element | null {
    if (this.editor.getEditingShapeId() !== shape.id) return null
    
    return (
      <textarea
        className="tl-text-editor"
        value={shape.props.text}
        onChange={(e) => {
          const text = e.target.value
          const metrics = this.measureText(text, shape.props)
          this.editor.updateShapes([{
            id: shape.id,
            props: {
              text,
              w: metrics.width,
              h: metrics.height,
            },
          }])
        }}
        onBlur={() => this.editor.setEditingShape(null)}
        autoFocus
        style={{
          fontFamily: shape.props.font,
          fontSize: shape.props.size,
          resize: 'none',
        }}
      />
    )
  }

  private measureText(text: string, props: TLTextShape['props']): { width: number; height: number } {
    const canvas = document.createElement('canvas')
    const ctx = canvas.getContext('2d')!
    ctx.font = `${props.size}px ${props.font}`
    
    const lines = text.split('\n')
    const maxWidth = Math.max(...lines.map((line) => ctx.measureText(line).width))
    const height = lines.length * props.size * 1.2  // line-height
    
    return { width: maxWidth + 4, height }  // padding
  }
}
```

**Sticky Note Shape:**

```typescript
interface TLNoteShape extends TLShape {
  type: 'note'
  props: {
    text: string
    color: ColorKey
    font: FontFamily
    size: SizeStyle
  }
}

class NoteShapeUtil extends BaseBoxShapeUtil<TLNoteShape> {
  static override type = 'note' as const

  create(props: Partial<TLNoteShape>): TLNoteShape {
    return {
      id: createShapeId(),
      type: 'note',
      x: props.x ?? 0,
      y: props.y ?? 0,
      w: props.w ?? 200,
      h: props.h ?? 200,
      rotation: 0,
      opacity: 1,
      parentId: this.editor.getCurrentPageId(),
      index: getIndexAbove(this.editor.getMaxIndex()),
      groupId: null,
      props: {
        text: props.text ?? '',
        color: props.color ?? 'yellow',
        font: props.font ?? 'hand',
        size: props.size ?? 'm',
      },
      meta: {},
    }
  }

  geometry(shape: TLNoteShape): Geometry2d {
    return new RectangleGeometry(shape.w, shape.h)
  }

  component(shape: TLNoteShape): JSX.Element {
    const { w, h } = shape
    const { color, text, font } = shape.props
    
    return (
      <svg width={w} height={h}>
        {/* Background with rounded corners */}
        <rect
          x={0}
          y={0}
          width={w}
          height={h}
          rx={8}
          ry={8}
          fill={this.getColor(color)}
        />
        
        {/* Shadow effect */}
        <rect
          x={4}
          y={4}
          width={w}
          height={h}
          rx={8}
          fill="none"
          stroke="rgba(0,0,0,0.1)"
          strokeWidth={1}
        />
        
        {/* Text content */}
        {text && (
          <foreignObject x={20} y={20} width={w - 40} height={h - 40}>
            <div
              style={{
                fontFamily: font,
                fontSize: 16,
                whiteSpace: 'pre-wrap',
                userSelect: 'none',
              }}
            >
              {text}
            </div>
          </foreignObject>
        )}
      </svg>
    )
  }
}
```

### 3.4 Media Shapes (Image, Video, Bookmark, Embed)

**Image Shape:**

```typescript
interface TLImageShape extends TLShape {
  type: 'image'
  props: {
    w: number
    h: number
    assetId: TLAssetId | null
    cropping: Box | null
    playing: boolean
  }
}

class ImageShapeUtil extends BaseBoxShapeUtil<TLImageShape> {
  static override type = 'image' as const

  geometry(shape: TLImageShape): Geometry2d {
    return new RectangleGeometry(shape.w, shape.h)
  }

  component(shape: TLImageShape): JSX.Element {
    const { w, h, assetId, cropping } = shape
    
    if (!assetId) {
      return (
        <svg width={w} height={h}>
          <rect width={w} height={h} fill="#f0f0f0" />
          <text x={w/2} y={h/2} textAnchor="middle" fill="#999">
            Drop image here
          </text>
        </svg>
      )
    }
    
    const asset = this.editor.getAsset(assetId)
    if (!asset) return <svg width={w} height={h} />
    
    const cropStyle = cropping ? {
      clipPath: `inset(${cropping.y}px ${w - cropping.x - cropping.w}px ${h - cropping.y - cropping.h}px ${cropping.x}px)`
    } : {}
    
    return (
      <svg width={w} height={h}>
        <image
          href={asset.src}
          width={w}
          height={h}
          preserveAspectRatio="xMidYMid slice"
          style={cropStyle}
        />
      </svg>
    )
  }

  // Handle image upload
  onDrop?(shape: TLImageShape, info: TLDropEventInfo): void {
    const file = info.files[0]
    if (!file?.type.startsWith('image/')) return
    
    const reader = new FileReader()
    reader.onload = (e) => {
      const asset = this.editor.createAsset({
        type: 'image',
        src: e.target?.result as string,
        w: info.imageWidth,
        h: info.imageHeight,
      })
      
      this.editor.updateShapes([{
        id: shape.id,
        props: { assetId: asset.id },
      }])
    }
    reader.readAsDataURL(file)
  }
}
```

**Video Shape:**

```typescript
interface TLVideoShape extends TLShape {
  type: 'video'
  props: {
    w: number
    h: number
    assetId: TLAssetId | null
    playing: boolean
    time: number
  }
}

class VideoShapeUtil extends BaseBoxShapeUtil<TLVideoShape> {
  static override type = 'video' as const

  component(shape: TLVideoShape): JSX.Element {
    const { w, h, assetId, playing } = shape
    
    if (!assetId) {
      return (
        <svg width={w} height={h}>
          <rect width={w} height={h} fill="#333" />
          <text x={w/2} y={h/2} textAnchor="middle" fill="#999">
            Drop video here
          </text>
        </svg>
      )
    }
    
    const asset = this.editor.getAsset(assetId)
    
    return (
      <foreignObject width={w} height={h}>
        <video
          src={asset?.src}
          width={w}
          height={h}
          autoPlay={playing ? 'autoplay' : undefined}
          controls
          style={{ objectFit: 'cover' }}
        />
      </foreignObject>
    )
  }
}
```

**Bookmark Shape:**

```typescript
interface TLBookmarkShape extends TLShape {
  type: 'bookmark'
  props: {
    url: string
    title: string
    description: string
    image: string | null
  }
}

class BookmarkShapeUtil extends BaseBoxShapeUtil<TLBookmarkShape> {
  static override type = 'bookmark' as const

  create(props: Partial<TLBookmarkShape>): TLBookmarkShape {
    return {
      id: createShapeId(),
      type: 'bookmark',
      x: props.x ?? 0,
      y: props.y ?? 0,
      w: props.w ?? 320,
      h: props.h ?? 160,
      rotation: 0,
      opacity: 1,
      parentId: this.editor.getCurrentPageId(),
      index: getIndexAbove(this.editor.getMaxIndex()),
      groupId: null,
      props: {
        url: props.url ?? '',
        title: props.title ?? '',
        description: props.description ?? '',
        image: props.image ?? null,
      },
      meta: {},
    }
  }

  component(shape: TLBookmarkShape): JSX.Element {
    const { w, h } = shape
    const { url, title, description, image } = shape.props
    
    return (
      <svg width={w} height={h}>
        <rect width={w} height={h} fill="white" stroke="#ccc" />
        
        {image && (
          <image
            href={image}
            x={16}
            y={16}
            width={h - 32}
            height={h - 32}
            preserveAspectRatio="xMidYMid slice"
          />
        )}
        
        <foreignObject
          x={image ? h : 16}
          y={16}
          width={w - (image ? h : 16) - 16}
          height={h - 32}
        >
          <div style={{ padding: '8px' }}>
            <a
              href={url}
              target="_blank"
              rel="noopener noreferrer"
              style={{
                fontWeight: 'bold',
                color: '#1a73e8',
                textDecoration: 'none',
              }}
            >
              {title || url}
            </a>
            {description && (
              <p style={{ 
                fontSize: 12, 
                color: '#666',
                margin: '4px 0 0',
                overflow: 'hidden',
                textOverflow: 'ellipsis',
              }}>
                {description}
              </p>
            )}
          </div>
        </foreignObject>
      </svg>
    )
  }
}
```

### 3.5 Connectors (Arrow, Line)

**Arrow Shape:**

```typescript
interface TLArrowShape extends TLShape {
  type: 'arrow'
  props: {
    start: { x: number; y: number }
    end: { x: number; y: number }
    bend: number
    size: SizeStyle
    dash: DashStyle
    start: ArrowheadStyle
    end: ArrowheadStyle
    color: ColorKey
  }
  bindings?: {
    start?: TLBindingId
    end?: TLBindingId
  }
}

class ArrowShapeUtil extends ShapeUtil<TLArrowShape> {
  static override type = 'arrow' as const

  create(props: Partial<TLArrowShape>): TLArrowShape {
    return {
      id: createShapeId(),
      type: 'arrow',
      x: props.x ?? 0,
      y: props.y ?? 0,
      rotation: 0,
      opacity: 1,
      parentId: this.editor.getCurrentPageId(),
      index: getIndexAbove(this.editor.getMaxIndex()),
      groupId: null,
      props: {
        start: { x: 0, y: 0 },
        end: { x: 100, y: 100 },
        bend: 0,
        size: props.size ?? 'm',
        dash: props.dash ?? 'solid',
        start: props.start ?? 'none',
        end: props.end ?? 'arrow',
        color: props.color ?? 'black',
      },
      meta: {},
    }
  }

  geometry(shape: TLArrowShape): Geometry2d {
    const path = this.getPath(shape)
    return new PolylineGeometry(path)
  }

  component(shape: TLArrowShape): JSX.Element {
    const path = this.getPath(shape)
    const strokeWidth = this.getStrokeWidth(shape.props.size)
    const color = this.getColor(shape.props.color)
    
    return (
      <svg>
        {/* Arrow path */}
        <path
          d={this.pathToSvg(path)}
          fill="none"
          stroke={color}
          strokeWidth={strokeWidth}
          strokeDasharray={this.getDashPattern(shape.props.dash)}
          strokeLinecap="round"
        />
        
        {/* Start arrowhead */}
        {shape.props.start !== 'none' && this.renderArrowhead(path[0], path[1], shape.props.start, color)}
        
        {/* End arrowhead */}
        {shape.props.end !== 'none' && this.renderArrowhead(path[path.length - 1], path[path.length - 2], shape.props.end, color)}
      </svg>
    )
  }

  getPath(shape: TLArrowShape): Vec[] {
    const { start, end, bend } = shape.props
    
    if (bend === 0) {
      // Straight line
      return [new Vec(start.x, start.y), new Vec(end.x, end.y)]
    }
    
    // Quadratic bezier with bend
    const midX = (start.x + end.x) / 2
    const midY = (start.y + end.y) / 2
    
    // Perpendicular offset for bend
    const dx = end.x - start.x
    const dy = end.y - start.y
    const len = Math.sqrt(dx * dx + dy * dy)
    const nx = -dy / len
    const ny = dx / len
    
    const controlX = midX + nx * bend
    const controlY = midY + ny * bend
    
    // Sample bezier curve
    return this.sampleBezier(
      new Vec(start.x, start.y),
      new Vec(controlX, controlY),
      new Vec(end.x, end.y)
    )
  }

  private sampleBezier(p0: Vec, p1: Vec, p2: Vec, samples = 16): Vec[] {
    const points: Vec[] = []
    for (let i = 0; i <= samples; i++) {
      const t = i / samples
      const u = 1 - t
      const x = u * u * p0.x + 2 * u * t * p1.x + t * t * p2.x
      const y = u * u * p0.y + 2 * u * t * p1.y + t * t * p2.y
      points.push(new Vec(x, y))
    }
    return points
  }

  private renderArrowhead(point: Vec, prev: Vec, type: string, color: string): JSX.Element {
    const angle = Math.atan2(point.y - prev.y, point.x - prev.x)
    const size = this.getArrowheadSize(type)
    
    return (
      <g transform={`translate(${point.x}, ${point.y}) rotate(${angle * 180 / Math.PI})`}>
        {type === 'arrow' && (
          <path
            d={`M 0 0 L -${size} -${size/2} L -${size} ${size/2} Z`}
            fill={color}
          />
        )}
        {type === 'dot' && (
          <circle cx={0} cy={0} r={size/2} fill={color} />
        )}
        {type === 'bar' && (
          <rect
            x={0}
            y={-size/2}
            width={4}
            height={size}
            fill={color}
          />
        )}
      </g>
    )
  }

  getHandles(shape: TLArrowShape): TLHandle[] {
    return [
      { id: 'start', type: 'vertex', x: shape.props.start.x, y: shape.props.start.y },
      { id: 'end', type: 'vertex', x: shape.props.end.x, y: shape.props.end.y },
      { id: 'bend', type: 'virtual', x: this.getBendPoint(shape).x, y: this.getBendPoint(shape).y },
    ]
  }

  onHandleDrag(shape: TLArrowShape, info: TLHandleDragEventInfo): TLArrowShape {
    switch (info.handle.id) {
      case 'start':
        return { ...shape, props: { ...shape.props, start: info.point } }
      case 'end':
        return { ...shape, props: { ...shape.props, end: info.point } }
      case 'bend':
        const bend = this.calculateBend(shape, info.point)
        return { ...shape, props: { ...shape.props, bend } }
    }
    return shape
  }

  private getBendPoint(shape: TLArrowShape): Vec {
    const { start, end, bend } = shape.props
    const midX = (start.x + start.x) / 2
    const midY = (start.y + end.y) / 2
    
    const dx = end.x - start.x
    const dy = end.y - start.y
    const len = Math.sqrt(dx * dx + dy * dy)
    
    return new Vec(
      midX - (dy / len) * bend,
      midY + (dx / len) * bend
    )
  }

  private calculateBend(shape: TLArrowShape, point: Vec): number {
    const midX = (shape.props.start.x + shape.props.end.x) / 2
    const midY = (shape.props.start.y + shape.props.end.y) / 2
    
    const dx = shape.props.end.x - shape.props.start.x
    const dy = shape.props.end.y - shape.props.start.y
    const len = Math.sqrt(dx * dx + dy * dy)
    
    // Project point onto perpendicular
    const nx = -dy / len
    const ny = dx / len
    const dot = (point.x - midX) * nx + (point.y - midY) * ny
    
    return dot
  }
}
```

### 3.6 Frames and Groups

**Frame Shape:**

```typescript
interface TLFrameShape extends TLShape {
  type: 'frame'
  props: {
    w: number
    h: number
    name: string
    color: ColorKey
  }
}

class FrameShapeUtil extends BaseBoxShapeUtil<TLFrameShape> {
  static override type = 'frame' as const

  geometry(shape: TLFrameShape): Geometry2d {
    return new RectangleGeometry(shape.w, shape.h)
  }

  component(shape: TLFrameShape): JSX.Element {
    const { w, h, name, color } = shape
    
    return (
      <>
        {/* Frame border */}
        <svg width={w} height={h}>
          <rect
            x={0}
            y={0}
            width={w}
            height={h}
            fill="none"
            stroke={this.getColor(color)}
            strokeWidth={2}
            strokeDasharray="8,4"
            rx={8}
          />
        </svg>
        
        {/* Name label */}
        <div
          className="tl-frame-name"
          style={{
            position: 'absolute',
            top: -24,
            left: 0,
            fontSize: 12,
            color: this.getColor(color),
            userSelect: 'none',
          }}
        >
          {name}
        </div>
      </>
    )
  }

  // Frames can contain children
  canBindChildren = true
}
```

**Group Shape:**

```typescript
interface TLGroupShape extends TLShape {
  type: 'group'
  props: {
    w: number
    h: number
  }
}

class GroupShapeUtil extends BaseBoxShapeUtil<TLGroupShape> {
  static override type = 'group' as const

  // Groups have no visual representation
  component(shape: TLGroupShape): JSX.Element {
    return <svg width={shape.w} height={shape.h} />
  }

  // Groups are invisible containers
  // Selection and transforms apply to all children
}
```

---

## 4. Custom Shapes

### 4.1 Creating Custom Shape Types

To create a custom shape, extend `ShapeUtil` and implement the required methods:

```typescript
// CustomCardShape.ts
import { ShapeUtil, TLShape, createShapeId, Geometry2d, RectangleGeometry } from '@tldraw/tldraw'

// Define shape interface
interface TLCustomCardShape extends TLShape {
  type: 'custom-card'
  props: {
    title: string
    description: string
    color: string
    w: number
    h: number
  }
}

// Implement ShapeUtil
export class CustomCardShapeUtil extends ShapeUtil<TLCustomCardShape> {
  static override type = 'custom-card' as const

  // ============ REQUIRED: Create ============
  create(props: Partial<TLCustomCardShape>): TLCustomCardShape {
    return {
      id: props.id ?? createShapeId(),
      type: 'custom-card',
      x: props.x ?? 0,
      y: props.y ?? 0,
      rotation: props.rotation ?? 0,
      opacity: props.opacity ?? 1,
      parentId: props.parentId ?? this.editor.getCurrentPageId(),
      index: props.index ?? this.editor.getMaxIndex(),
      groupId: null,
      props: {
        title: props.title ?? 'Card Title',
        description: props.description ?? 'Card description goes here',
        color: props.color ?? '#ffffff',
        w: props.w ?? 240,
        h: props.h ?? 160,
      },
      meta: props.meta ?? {},
    }
  }

  // ============ REQUIRED: Geometry ============
  geometry(shape: TLCustomCardShape): Geometry2d {
    return new RectangleGeometry(shape.props.w, shape.props.h)
  }

  // ============ REQUIRED: Component ============
  component(shape: TLCustomCardShape): JSX.Element {
    const { w, h } = shape.props
    
    return (
      <svg width={w} height={h}>
        {/* Card background with shadow */}
        <defs>
          <filter id={`shadow-${shape.id}`} x="-50%" y="-50%" width="200%" height="200%">
            <feDropShadow dx={0} dy={4} stdDeviation={8} floodOpacity={0.15} />
          </filter>
        </defs>
        
        <rect
          x={0}
          y={0}
          width={w}
          height={h}
          rx={12}
          ry={12}
          fill={shape.props.color}
          filter={`url(#shadow-${shape.id})`}
          stroke="#e0e0e0"
          strokeWidth={1}
        />
        
        {/* Title bar */}
        <rect
          x={0}
          y={0}
          width={w}
          height={48}
          rx={12}
          ry={12}
          fill="rgba(0,0,0,0.05)"
          clipPath="inset(0 0 50% 0 round 12px 12px 0 0)"
        />
        
        {/* Title text */}
        <foreignObject x={16} y={12} width={w - 32} height={24}>
          <div
            style={{
              fontWeight: 'bold',
              fontSize: 14,
              overflow: 'hidden',
              textOverflow: 'ellipsis',
              whiteSpace: 'nowrap',
            }}
          >
            {shape.props.title}
          </div>
        </foreignObject>
        
        {/* Description */}
        <foreignObject x={16} y={56} width={w - 32} height={h - 72}>
          <div
            style={{
              fontSize: 12,
              color: '#666',
              lineHeight: 1.5,
              overflow: 'hidden',
            }}
          >
            {shape.props.description}
          </div>
        </foreignObject>
      </svg>
    )
  }
}
```

### 4.2 Shape Props Definition

Define props with validation using the schema system:

```typescript
// CustomChartShape.ts
interface TLCustomChartShape extends TLShape {
  type: 'custom-chart'
  props: {
    data: number[]
    chartType: 'bar' | 'line' | 'pie'
    colors: string[]
    w: number
    h: number
    showLabels: boolean
    showGrid: boolean
  }
}

// Props validator
const chartPropsValidator = object({
  data: array(number),
  chartType: oneOf('bar', 'line', 'pie'),
  colors: array(string),
  w: positiveNumber,
  h: positiveNumber,
  showLabels: boolean,
  showGrid: boolean,
})

export class CustomChartShapeUtil extends ShapeUtil<TLCustomChartShape> {
  static override type = 'custom-chart' as const

  create(props: Partial<TLCustomChartShape>): TLCustomChartShape {
    return {
      id: createShapeId(),
      type: 'custom-chart',
      x: 0,
      y: 0,
      parentId: this.editor.getCurrentPageId(),
      index: getIndexAbove(this.editor.getMaxIndex()),
      groupId: null,
      rotation: 0,
      opacity: 1,
      props: {
        data: props.data ?? [10, 20, 30, 40, 50],
        chartType: props.chartType ?? 'bar',
        colors: props.colors ?? ['#4a90d9', '#67b26f', '#f5a623', '#d0021b', '#9013fe'],
        w: props.w ?? 300,
        h: props.h ?? 200,
        showLabels: props.showLabels ?? true,
        showGrid: props.showGrid ?? true,
      },
      meta: {},
    }
  }

  geometry(shape: TLCustomChartShape): Geometry2d {
    return new RectangleGeometry(shape.props.w, shape.props.h)
  }

  component(shape: TLCustomChartShape): JSX.Element {
    const { w, h, data, chartType, colors, showLabels, showGrid } = shape.props
    
    return (
      <svg width={w} height={h}>
        {/* Grid lines */}
        {showGrid && this.renderGrid(w, h)}
        
        {/* Chart based on type */}
        {chartType === 'bar' && this.renderBarChart(data, colors, w, h)}
        {chartType === 'line' && this.renderLineChart(data, colors, w, h)}
        {chartType === 'pie' && this.renderPieChart(data, colors, w, h)}
        
        {/* Labels */}
        {showLabels && this.renderLabels(data, w, h)}
      </svg>
    )
  }

  private renderGrid(w: number, h: number): JSX.Element {
    const lines = []
    const gridCount = 5
    
    for (let i = 0; i <= gridCount; i++) {
      const y = (i / gridCount) * h
      lines.push(
        <line
          key={i}
          x1={0}
          y1={y}
          x2={w}
          y2={y}
          stroke="#e0e0e0"
          strokeDasharray="4,4"
        />
      )
    }
    
    return <>{lines}</>
  }

  private renderBarChart(data: number[], colors: string[], w: number, h: number): JSX.Element {
    const maxValue = Math.max(...data)
    const barWidth = (w - 40) / data.length
    const padding = 20
    
    return (
      <g>
        {data.map((value, i) => {
          const barHeight = (value / maxValue) * (h - 40)
          return (
            <rect
              key={i}
              x={padding + i * barWidth}
              y={h - padding - barHeight}
              width={barWidth - 4}
              height={barHeight}
              fill={colors[i % colors.length]}
              rx={2}
            />
          )
        })}
      </g>
    )
  }

  private renderLineChart(data: number[], colors: string[], w: number, h: number): JSX.Element {
    const maxValue = Math.max(...data)
    const stepX = (w - 40) / (data.length - 1)
    const padding = 20
    
    const points = data.map((value, i) => ({
      x: padding + i * stepX,
      y: h - padding - (value / maxValue) * (h - 40),
    }))
    
    const pathD = `M ${points.map((p) => `${p.x} ${p.y}`).join(' L ')}`
    
    return (
      <g>
        {/* Fill area */}
        <path
          d={`${pathD} L ${points[points.length - 1].x} ${h - padding} L ${points[0].x} ${h - padding} Z`}
          fill={colors[0]}
          fillOpacity={0.1}
        />
        
        {/* Line */}
        <path
          d={pathD}
          fill="none"
          stroke={colors[0]}
          strokeWidth={2}
          strokeLinecap="round"
          strokeLinejoin="round"
        />
        
        {/* Points */}
        {points.map((p, i) => (
          <circle
            key={i}
            cx={p.x}
            cy={p.y}
            r={4}
            fill={colors[i % colors.length]}
            stroke="white"
            strokeWidth={2}
          />
        ))}
      </g>
    )
  }

  private renderPieChart(data: number[], colors: string[], w: number, h: number): JSX.Element {
    const total = data.reduce((a, b) => a + b, 0)
    const cx = w / 2
    const cy = h / 2
    const r = Math.min(w, h) / 2 - 10
    
    let startAngle = 0
    const segments = data.map((value, i) => {
      const angle = (value / total) * Math.PI * 2
      const endAngle = startAngle + angle
      
      // Calculate arc path
      const x1 = cx + r * Math.cos(startAngle)
      const y1 = cy + r * Math.sin(startAngle)
      const x2 = cx + r * Math.cos(endAngle)
      const y2 = cy + r * Math.sin(endAngle)
      
      const largeArc = angle > Math.PI ? 1 : 0
      
      const pathD = `M ${cx} ${cy} L ${x1} ${y1} A ${r} ${r} 0 ${largeArc} 1 ${x2} ${y2} Z`
      
      startAngle = endAngle
      
      return (
        <path
          key={i}
          d={pathD}
          fill={colors[i % colors.length]}
          stroke="white"
          strokeWidth={1}
        />
      )
    })
    
    return <>{segments}</>
  }

  private renderLabels(data: number[], w: number, h: number): JSX.Element {
    return (
      <foreignObject x={0} y={h - 20} width={w} height={20}>
        <div style={{ fontSize: 10, color: '#666', textAlign: 'center' }}>
          {data.join(' | ')}
        </div>
      </foreignObject>
    )
  }

  // ============ Custom Handles ============
  getHandles(shape: TLCustomChartShape): TLHandle[] {
    return [
      // Resize handles (inherited from BaseBoxShapeUtil pattern)
      { id: 'tl', type: 'vertex', x: 0, y: 0 },
      { id: 'tr', type: 'vertex', x: shape.props.w, y: 0 },
      { id: 'br', type: 'vertex', x: shape.props.w, y: shape.props.h },
      { id: 'bl', type: 'vertex', x: 0, y: shape.props.h },
    ]
  }

  // ============ Pointer Events ============
  onPointerDown(shape: TLCustomChartShape, info: TLPointerEventInfo): void {
    // Double-click to edit data
    if (info.ctrlKey || info.metaKey) {
      this.openDataEditor(shape)
      return
    }
    
    // Default: select shape
    this.editor.select(shape.id)
  }

  private openDataEditor(shape: TLCustomChartShape): void {
    // Open custom modal/editor
    const newData = prompt('Enter data values (comma-separated):', shape.props.data.join(','))
    if (newData) {
      const data = newData.split(',').map((s) => parseFloat(s.trim())).filter((n) => !isNaN(n))
      if (data.length > 0) {
        this.editor.updateShapes([{
          id: shape.id,
          props: { data },
        }])
      }
    }
  }
}
```

### 4.3 Rendering Implementation

**Complex Rendering with Multiple Layers:**

```typescript
// CustomNodeShape.ts - Node with ports for connections
interface TLCustomNodeShape extends TLShape {
  type: 'custom-node'
  props: {
    label: string
    inputPorts: number
    outputPorts: number
    color: string
    w: number
    h: number
  }
}

export class CustomNodeShapeUtil extends ShapeUtil<TLCustomNodeShape> {
  static override type = 'custom-node' as const

  geometry(shape: TLCustomNodeShape): Geometry2d {
    return new RectangleGeometry(shape.props.w, shape.props.h)
  }

  component(shape: TLCustomNodeShape): JSX.Element {
    const { w, h, label, inputPorts, outputPorts, color } = shape.props
    const portSize = 12
    const headerHeight = 36
    
    return (
      <svg width={w} height={h}>
        <defs>
          {/* Gradient for node header */}
          <linearGradient id={`header-gradient-${shape.id}`} x1="0%" y1="0%" x2="0%" y2="100%">
            <stop offset="0%" stopColor={color} stopOpacity={0.9} />
            <stop offset="100%" stopColor={color} stopOpacity={0.7} />
          </linearGradient>
        </defs>
        
        {/* Main body with shadow */}
        <rect
          x={0}
          y={0}
          width={w}
          height={h}
          rx={8}
          fill="white"
          stroke="#ccc"
          strokeWidth={1}
          filter="drop-shadow(0 2px 8px rgba(0,0,0,0.15))"
        />
        
        {/* Header */}
        <rect
          x={0}
          y={0}
          width={w}
          height={headerHeight}
          rx={8}
          fill={`url(#header-gradient-${shape.id})`}
          clipPath="inset(0 0 50% 0 round 8px 8px 0 0)"
        />
        
        {/* Label */}
        <foreignObject x={12} y={8} width={w - 24} height={headerHeight - 16}>
          <div
            style={{
              fontWeight: 'bold',
              fontSize: 13,
              color: 'white',
              textShadow: '0 1px 2px rgba(0,0,0,0.3)',
              overflow: 'hidden',
              textOverflow: 'ellipsis',
              whiteSpace: 'nowrap',
            }}
          >
            {label}
          </div>
        </foreignObject>
        
        {/* Input ports (left side) */}
        {Array.from({ length: inputPorts }).map((_, i) => {
          const y = headerHeight + ((h - headerHeight) / (inputPorts + 1)) * (i + 1)
          return (
            <g key={`in-${i}`} data-port-type="input" data-port-index={i}>
              <circle
                cx={0}
                cy={y}
                r={portSize / 2}
                fill="#666"
                stroke="white"
                strokeWidth={2}
              />
            </g>
          )
        })}
        
        {/* Output ports (right side) */}
        {Array.from({ length: outputPorts }).map((_, i) => {
          const y = headerHeight + ((h - headerHeight) / (outputPorts + 1)) * (i + 1)
          return (
            <g key={`out-${i}`} data-port-type="output" data-port-index={i}>
              <circle
                cx={w}
                cy={y}
                r={portSize / 2}
                fill={color}
                stroke="white"
                strokeWidth={2}
              />
            </g>
          )
        })}
      </svg>
    )
  }

  // ============ Binding Points for Connections ============
  getBindingPoints(shape: TLCustomNodeShape): TLBindingPoint[] {
    const { w, h, inputPorts, outputPorts } = shape.props
    const headerHeight = 36
    const points: TLBindingPoint[] = []
    
    // Input binding points
    for (let i = 0; i < inputPorts; i++) {
      const y = headerHeight + ((h - headerHeight) / (inputPorts + 1)) * (i + 1)
      points.push({
        id: `in-${i}`,
        x: 0,
        y,
        normal: new Vec(-1, 0),
        type: 'input',
      })
    }
    
    // Output binding points
    for (let i = 0; i < outputPorts; i++) {
      const y = headerHeight + ((h - headerHeight) / (outputPorts + 1)) * (i + 1)
      points.push({
        id: `out-${i}`,
        x: w,
        y,
        normal: new Vec(1, 0),
        type: 'output',
      })
    }
    
    return points
  }
}
```

### 4.4 Interaction Handling

**Advanced Pointer and Drag Handling:**

```typescript
// CustomInteractiveShape.ts
interface TLCustomInteractiveShape extends TLShape {
  type: 'custom-interactive'
  props: {
    items: { id: string; label: string; x: number; y: number }[]
    selectedItem: string | null
  }
}

export class CustomInteractiveShapeUtil extends ShapeUtil<TLCustomInteractiveShape> {
  static override type = 'custom-interactive' as const

  component(shape: TLCustomInteractiveShape): JSX.Element {
    return (
      <svg width={300} height={200}>
        <rect width={300} height={200} fill="#f9f9f9" stroke="#ddd" />
        
        {shape.props.items.map((item) => (
          <g
            key={item.id}
            onClick={(e) => {
              e.stopPropagation()
              this.handleItemClick(shape, item.id)
            }}
            style={{ cursor: 'pointer' }}
          >
            <rect
              x={item.x}
              y={item.y}
              width={80}
              height={40}
              rx={4}
              fill={shape.props.selectedItem === item.id ? '#4a90d9' : 'white'}
              stroke={shape.props.selectedItem === item.id ? '#2d5a8a' : '#ccc'}
            />
            <text
              x={item.x + 40}
              y={item.y + 20}
              textAnchor="middle"
              dominantBaseline="middle"
              fill={shape.props.selectedItem === item.id ? 'white' : '#333'}
              fontSize={12}
            >
              {item.label}
            </text>
          </g>
        ))}
      </svg>
    )
  }

  onPointerDown(shape: TLCustomInteractiveShape, info: TLPointerEventInfo): void {
    // Click on background - deselect
    this.editor.updateShapes([{
      id: shape.id,
      props: { selectedItem: null },
    }])
  }

  private handleItemClick(shape: TLCustomInteractiveShape, itemId: string): void {
    this.editor.updateShapes([{
      id: shape.id,
      props: { selectedItem: itemId },
    }])
  }

  // ============ Drag to reorder items ============
  private _draggingItemId: string | null = null

  onDragStart(shape: TLCustomInteractiveShape, info: TLDragEventInfo): void {
    const hitItem = this.getItemAtPoint(shape, info.point)
    if (hitItem) {
      this._draggingItemId = hitItem.id
    }
  }

  onDrag(shape: TLCustomInteractiveShape, info: TLDragEventInfo): void {
    if (!this._draggingItemId) return
    
    const items = shape.props.items.map((item) => {
      if (item.id === this._draggingItemId) {
        return {
          ...item,
          x: info.point.x - 40,  // Center on pointer
          y: info.point.y - 20,
        }
      }
      return item
    })
    
    this.editor.updateShapes([{
      id: shape.id,
      props: { items },
    }])
  }

  onDragEnd(): void {
    this._draggingItemId = null
  }

  private getItemAtPoint(shape: TLCustomInteractiveShape, point: VecLike): typeof shape.props.items[0] | null {
    return shape.props.items.find((item) => {
      return (
        point.x >= item.x &&
        point.x <= item.x + 80 &&
        point.y >= item.y &&
        point.y <= item.y + 40
      )
    }) ?? null
  }
}
```

### 4.5 Migration Strategies

**Shape Migration Example:**

```typescript
// CustomShapeMigration.ts
export class CustomMigratableShapeUtil extends ShapeUtil<TLCustomShape> {
  static override type = 'custom-migratable' as const

  // Migrate from older schema versions
  migrate(shape: TLShape): TLCustomShape {
    let migrated = { ...shape } as TLCustomShape
    
    // Version 1 -> 2: Add color property
    if ((migrated.props as any).color === undefined) {
      migrated = {
        ...migrated,
        props: {
          ...(migrated.props as any),
          color: '#ffffff',
        },
      }
    }
    
    // Version 2 -> 3: Rename 'title' to 'label'
    if ((migrated.props as any).title !== undefined) {
      migrated.props.label = (migrated.props as any).title
      delete (migrated.props as any).title
    }
    
    // Version 3 -> 4: Add opacity
    if (migrated.opacity === undefined) {
      migrated.opacity = 1
    }
    
    return migrated
  }

  // Snapshot for undo/redo
  snapshot(shape: TLCustomShape): TLCustomShape {
    return {
      ...shape,
      props: { ...shape.props },
    }
  }

  // Upgrade props during schema changes
  upgrade(shape: TLShape): Partial<TLCustomShape> {
    const props = shape.props as Record<string, unknown>
    const upgrades: Partial<TLCustomShape> = {}
    
    // Example: Convert hex colors to design tokens
    if (typeof props.color === 'string' && props.color.startsWith('#')) {
      upgrades.props = {
        ...shape.props,
        color: this.migrateColor(props.color),
      }
    }
    
    return upgrades
  }

  private migrateColor(hex: string): string {
    const colorMap: Record<string, string> = {
      '#ffffff': 'white',
      '#000000': 'black',
      '#ff0000': 'red',
      '#00ff00': 'green',
      '#0000ff': 'blue',
    }
    return colorMap[hex.toLowerCase()] || hex
  }
}
```

---

## 5. Shape State Management

### 5.1 Shape Updates

Shape updates flow through the Editor and into the Store:

```typescript
// Editor.ts - Shape update pipeline
updateShapes<T extends TLShape>(partials: Partial<T>[]): void {
  return this.run(
    () => {
      const updates: TLShape[] = []
      
      for (const partial of partials) {
        const shape = this.getShape(partial.id)
        if (!shape) continue
        
        const util = this.getShapeUtil(shape)
        
        // Apply update
        const updated = { ...shape, ...partial } as TLShape
        
        // Validate and normalize through util
        const normalized = util.validate?.(updated) ?? updated
        
        updates.push(normalized)
      }
      
      // Batch update to store
      this.store.put(updates)
    },
    { history: 'record' }
  )
}

// Transaction batching for atomic updates
batchUpdate() {
  this.editor.batch(() => {
    this.editor.updateShapes([{
      id: shape1.id,
      x: 100,
      y: 100,
    }])
    this.editor.updateShapes([{
      id: shape2.id,
      x: 200,
      y: 200,
    }])
    // Both updates are in single undo entry
  })
}
```

### 5.2 Transform Operations

Transform operations use matrices for efficient composition:

```typescript
// Apply transform to shape
applyTransform(shape: TLShape, transform: Mat): TLShape {
  const { x, y, rotation } = transform.decompose()
  
  return {
    ...shape,
    x: shape.x + x,
    y: shape.y + y,
    rotation: shape.rotation + rotation,
  }
}

// Get composed transform including all ancestors
getShapePageTransform(shapeId: TLShapeId): Mat | null {
  const shape = this.getShape(shapeId)
  if (!shape) return null
  
  const localTransform = Mat.Compose(
    Mat.Translate(shape.x, shape.y),
    Mat.Rotate(shape.rotation)
  )
  
  // Recursively compose parent transforms
  const parentTransform = shape.parentId && !isPageId(shape.parentId)
    ? this.getShapePageTransform(shape.parentId)
    : Mat.Identity()
  
  if (!parentTransform) return localTransform
  
  return Mat.Compose(parentTransform, localTransform)
}
```

### 5.3 Rotation

```typescript
// Rotate shape around its center
rotateShape(shape: TLShape, rotationDelta: number): TLShape {
  return {
    ...shape,
    rotation: shape.rotation + rotationDelta,
  }
}

// Rotate shape around arbitrary point
rotateShapeAround(shape: TLShape, center: VecLike, rotationDelta: number): TLShape {
  const currentTransform = this.getShapePageTransform(shape.id)!
  const shapeCenter = currentTransform.applyToPoint(new Vec(shape.w / 2, shape.h / 2))
  
  // Rotate center point
  const rotatedCenter = shapeCenter.rotWith(center, rotationDelta)
  const delta = Vec.Sub(rotatedCenter, shapeCenter)
  
  return {
    ...shape,
    x: shape.x + delta.x,
    y: shape.y + delta.y,
    rotation: shape.rotation + rotationDelta,
  }
}
```

### 5.4 Scale

```typescript
// Scale shape
scaleShape(shape: TLBoxShape, scaleX: number, scaleY: number): TLBoxShape {
  return {
    ...shape,
    w: Math.max(1, shape.w * scaleX),
    h: Math.max(1, shape.h * scaleY),
  }
}

// Scale from specific corner
scaleFromCorner(shape: TLBoxShape, corner: 'tl' | 'tr' | 'br' | 'bl', scale: VecLike): TLBoxShape {
  const newW = Math.max(1, shape.w * scale.x)
  const newH = Math.max(1, shape.h * scale.y)
  
  let x = shape.x
  let y = shape.y
  
  // Adjust position based on which corner is being dragged
  if (corner === 'tr' || corner === 'br') {
    x = shape.x + (shape.w - newW)
  }
  if (corner === 'bl' || corner === 'br') {
    y = shape.y + (shape.h - newH)
  }
  
  return {
    ...shape,
    x,
    y,
    w: newW,
    h: newH,
  }
}
```

### 5.5 Position

```typescript
// Move shape to absolute position
moveShapeTo(shape: TLShape, x: number, y: number): TLShape {
  return { ...shape, x, y }
}

// Move shape by delta
moveShapeBy(shape: TLShape, dx: number, dy: number): TLShape {
  return {
    ...shape,
    x: shape.x + dx,
    y: shape.y + dy,
  }
}

// Snap position to grid
snapToGrid(shape: TLShape, gridSize: number): TLShape {
  return {
    ...shape,
    x: Math.round(shape.x / gridSize) * gridSize,
    y: Math.round(shape.y / gridSize) * gridSize,
  }
}
```

---

## 6. Shape Interactions

### 6.1 Selection

```typescript
// Select single shape
editor.select(shapeId)

// Select multiple shapes
editor.select(shapeId1, shapeId2, shapeId3)

// Add to selection
editor.select(...editor.getSelectedShapeIds(), newShapeId)

// Deselect all
editor.selectNone()

// Toggle selection
const isSelected = editor.getSelectedShapeIds().includes(shapeId)
if (isSelected) {
  editor.select(...editor.getSelectedShapeIds().filter(id => id !== shapeId))
} else {
  editor.select(...editor.getSelectedShapeIds(), shapeId)
}

// Get selected shapes
const selectedShapes = editor.getSelectedShapes()
```

### 6.2 Transformation

```typescript
// Translate selection
translateSelection(delta: VecLike) {
  const selectedShapes = this.editor.getSelectedShapes()
  
  this.editor.updateShapes(
    selectedShapes.map((shape) => ({
      id: shape.id,
      x: shape.x + delta.x,
      y: shape.y + delta.y,
    }))
  )
}

// Rotate selection
rotateSelection(rotation: number) {
  const selectedShapes = this.editor.getSelectedShapes()
  const center = this.editor.getSelectionPageBounds()?.center
  
  if (!center) return
  
  this.editor.updateShapes(
    selectedShapes.map((shape) => 
      this.rotateShapeAround(shape, center, rotation)
    )
  )
}

// Scale selection
scaleSelection(scaleFactor: number) {
  const selectedShapes = this.editor.getSelectedShapes()
  
  this.editor.updateShapes(
    selectedShapes.map((shape) => ({
      id: shape.id,
      w: shape.w * scaleFactor,
      h: shape.h * scaleFactor,
    }))
  )
}
```

### 6.3 Resizing

```typescript
// Resize with handle
resizeWithHandle(shape: TLBoxShape, handleId: string, point: VecLike): TLBoxShape {
  switch (handleId) {
    case 'tl':  // Top-left
      return {
        ...shape,
        x: point.x,
        y: point.y,
        w: shape.w + (shape.x - point.x),
        h: shape.h + (shape.y - point.y),
      }
    case 'tr':  // Top-right
      return {
        ...shape,
        y: point.y,
        w: point.x - shape.x,
        h: shape.h + (shape.y - point.y),
      }
    case 'br':  // Bottom-right
      return {
        ...shape,
        w: point.x - shape.x,
        h: point.y - shape.y,
      }
    case 'bl':  // Bottom-left
      return {
        ...shape,
        x: point.x,
        w: shape.w + (shape.x - point.x),
        h: point.y - shape.y,
      }
    case 't':  // Top edge
      return {
        ...shape,
        y: point.y,
        h: shape.h + (shape.y - point.y),
      }
    case 'b':  // Bottom edge
      return {
        ...shape,
        h: point.y - shape.y,
      }
    case 'l':  // Left edge
      return {
        ...shape,
        x: point.x,
        w: shape.w + (shape.x - point.x),
      }
    case 'r':  // Right edge
      return {
        ...shape,
        w: point.x - shape.x,
      }
  }
  return shape
}

// Resize with aspect ratio lock
resizeWithAspectRatio(shape: TLBoxShape, handleId: string, point: VecLike): TLBoxShape {
  const aspectRatio = shape.w / shape.h
  const resized = this.resizeWithHandle(shape, handleId, point)
  
  // Corner handles preserve aspect ratio
  if (['tl', 'tr', 'br', 'bl'].includes(handleId)) {
    if (resized.w / resized.h > aspectRatio) {
      resized.h = resized.w / aspectRatio
    } else {
      resized.w = resized.h * aspectRatio
    }
  }
  
  return resized
}
```

### 6.4 Rotation

```typescript
// Start rotation
beginRotation(shape: TLShape, initialAngle: number): void {
  this._rotationState = {
    shapeId: shape.id,
    initialRotation: shape.rotation,
    startAngle: initialAngle,
  }
}

// Update rotation during drag
updateRotation(currentAngle: number): void {
  if (!this._rotationState) return
  
  const angleDelta = currentAngle - this._rotationState.startAngle
  
  this.editor.updateShapes([{
    id: this._rotationState.shapeId,
    rotation: this._rotationState.initialRotation + angleDelta,
  }])
}

// End rotation
endRotation(): void {
  this._rotationState = null
}
```

### 6.5 Duplication

```typescript
// Duplicate shape
duplicateShape(shapeId: TLShapeId): TLShapeId {
  const shape = this.editor.getShape(shapeId)
  if (!shape) return shapeId
  
  const newShape = {
    ...shape,
    id: createShapeId(),
    x: shape.x + 20,  // Offset duplicate
    y: shape.y + 20,
  }
  
  this.editor.createShapes([newShape])
  return newShape.id
}

// Duplicate selection
duplicateSelection(): void {
  const selectedShapeIds = this.editor.getSelectedShapeIds()
  
  const duplicates = selectedShapeIds.map((id) => {
    const shape = this.editor.getShape(id)!
    return {
      ...shape,
      id: createShapeId(),
      x: shape.x + 20,
      y: shape.y + 20,
    }
  })
  
  this.editor.createShapes(duplicates)
  this.editor.select(...duplicates.map((s) => s.id))
}
```

### 6.6 Deletion

```typescript
// Delete single shape
deleteShape(shapeId: TLShapeId): void {
  this.editor.deleteShapes([shapeId])
}

// Delete selection
deleteSelection(): void {
  const selectedShapeIds = this.editor.getSelectedShapeIds()
  this.editor.deleteShapes(selectedShapeIds)
}

// Delete with bindings cleanup
deleteShapeWithBindings(shapeId: TLShapeId): void {
  const shape = this.editor.getShape(shapeId)
  if (!shape) return
  
  // Find and delete associated bindings
  const bindings = this.editor.getBindingsForShape(shapeId)
  const bindingIds = bindings.map((b) => b.id)
  
  this.editor.deleteShapes([shapeId])
  this.editor.deleteBindings(bindingIds)
}
```

---

## 7. Connectors and Bindings

### 7.1 Arrow Shapes

See section 3.5 for complete arrow implementation.

### 7.2 Binding Points

```typescript
// TLBindingPoint interface
interface TLBindingPoint {
  id: string
  x: number
  y: number
  normal: Vec  // Outward normal for connection angle
  type?: 'input' | 'output' | 'both'
}

// Get binding points for a shape
getShapeBindingPoints(shapeId: TLShapeId): TLBindingPoint[] {
  const shape = this.getShape(shapeId)
  if (!shape) return []
  
  const util = this.getShapeUtil(shape)
  return util.getBindingPoints?.(shape) ?? []
}

// Find nearest binding point to a target
findNearestBindingPoint(
  shapeId: TLShapeId,
  target: VecLike
): TLBindingPoint | null {
  const points = this.getShapeBindingPoints(shapeId)
  if (points.length === 0) return null
  
  let nearest: TLBindingPoint | null = null
  let minDist = Infinity
  
  for (const point of points) {
    const dist = Vec.Dist(point, target)
    if (dist < minDist) {
      minDist = dist
      nearest = point
    }
  }
  
  return nearest
}
```

### 7.3 Connection Logic

```typescript
// Create binding between arrow and shape
createBinding(arrowId: TLArrowId, shapeId: TLShapeId, handle: 'start' | 'end'): TLBinding {
  const arrow = this.getShape(arrowId) as TLArrowShape
  const shape = this.getShape(shapeId)
  
  // Find nearest binding point
  const bindingPoint = this.findNearestBindingPoint(
    shapeId,
    handle === 'start' ? arrow.props.start : arrow.props.end
  )
  
  if (!bindingPoint) {
    // Create free binding (no shape)
    return null
  }
  
  const binding: TLBinding = {
    id: createBindingId(),
    typeName: 'binding',
    fromId: arrowId,
    toId: shapeId,
    props: {
      handle,
      normalizedAnchor: {
        x: bindingPoint.x / shape.w,
        y: bindingPoint.y / shape.h,
      },
    },
  }
  
  this.store.put([binding])
  return binding
}

// Update arrow when bound shape moves
updateBoundArrow(binding: TLBinding): void {
  const arrow = this.getShape(binding.fromId) as TLArrowShape
  const shape = this.getShape(binding.toId)
  
  if (!arrow || !shape) return
  
  const bindingPoints = this.getShapeBindingPoints(shape.id)
  const point = bindingPoints.find((p) => p.id === binding.props.normalizedAnchor?.id)
  
  if (!point) return
  
  const pageTransform = this.getShapePageTransform(shape.id)!
  const pagePoint = pageTransform.applyToPoint(point)
  
  // Update arrow endpoint
  this.updateShapes([{
    id: arrow.id,
    props: {
      [binding.props.handle]: { x: pagePoint.x, y: pagePoint.y },
    },
  }])
}
```

### 7.4 Path Calculation

```typescript
// Calculate arrow path with obstacle avoidance
calculateArrowPath(start: VecLike, end: VecLike, obstacles: Box[]): Vec[] {
  // Check for direct path
  if (!this.pathIntersectsObstacles(start, end, obstacles)) {
    return [start, end]
  }
  
  // Find path around obstacles using A*
  return this.findPathAroundObstacles(start, end, obstacles)
}

// Check if line segment intersects any obstacle
pathIntersectsObstacles(start: VecLike, end: VecLike, obstacles: Box[]): boolean {
  for (const obstacle of obstacles) {
    if (lineIntersectsBox(start, end, obstacle)) {
      return true
    }
  }
  return false
}

// Elbow path (right-angle connector)
calculateElbowPath(start: VecLike, end: VecLike, orientation: 'horizontal' | 'vertical'): Vec[] {
  if (orientation === 'horizontal') {
    const midX = (start.x + end.x) / 2
    return [
      start,
      new Vec(midX, start.y),
      new Vec(midX, end.y),
      end,
    ]
  } else {
    const midY = (start.y + end.y) / 2
    return [
      start,
      new Vec(start.x, midY),
      new Vec(end.x, midY),
      end,
    ]
  }
}

// Smooth elbow with rounded corners
calculateRoundedElbowPath(start: VecLike, end: VecLike, radius: number): Vec[] {
  const elbow = this.calculateElbowPath(start, end, 'horizontal')
  
  // Round the corner
  const corner = elbow[1]
  const beforeCorner = elbow[0]
  const afterCorner = elbow[2]
  
  // Calculate tangent points for arc
  const t1 = corner.clone().add(Vec.Sub(beforeCorner, corner).norm().mul(radius))
  const t2 = corner.clone().add(Vec.Sub(afterCorner, corner).norm().mul(radius))
  
  return [
    beforeCorner,
    t1,
    // Arc from t1 to t2
    ...this.arcPoints(t1, corner, t2, radius),
    t2,
    afterCorner,
  ]
}
```

### 7.5 Elbow Arrows

```typescript
// ElbowArrowShapeUtil - Right-angle arrow
class ElbowArrowShapeUtil extends ShapeUtil<TLElbowArrowShape> {
  static override type = 'elbow-arrow' as const

  geometry(shape: TLElbowArrowShape): Geometry2d {
    const path = this.getPath(shape)
    return new PolylineGeometry(path)
  }

  component(shape: TLElbowArrowShape): JSX.Element {
    const path = this.getPath(shape)
    const strokeWidth = this.getStrokeWidth(shape.props.size)
    
    return (
      <svg>
        <path
          d={this.elbowPathToSvg(path)}
          fill="none"
          stroke={this.getColor(shape.props.color)}
          strokeWidth={strokeWidth}
          strokeLinecap="round"
          strokeLinejoin="round"
        />
        {/* Arrowhead */}
        {this.renderArrowhead(path[path.length - 1], path[path.length - 2])}
      </svg>
    )
  }

  getPath(shape: TLElbowArrowShape): Vec[] {
    const { start, end } = shape.props
    
    // Determine elbow orientation based on relative positions
    const dx = Math.abs(end.x - start.x)
    const dy = Math.abs(end.y - start.y)
    
    if (dx > dy) {
      // Horizontal-first elbow
      const midX = (start.x + end.x) / 2
      return [
        new Vec(start.x, start.y),
        new Vec(midX, start.y),
        new Vec(midX, end.y),
        new Vec(end.x, end.y),
      ]
    } else {
      // Vertical-first elbow
      const midY = (start.y + end.y) / 2
      return [
        new Vec(start.x, start.y),
        new Vec(start.x, midY),
        new Vec(end.x, midY),
        new Vec(end.x, end.y),
      ]
    }
  }

  getHandles(shape: TLElbowArrowShape): TLHandle[] {
    const path = this.getPath(shape)
    return [
      { id: 'start', type: 'vertex', ...path[0] },
      { id: 'end', type: 'vertex', ...path[path.length - 1] },
      { id: 'elbow', type: 'virtual', ...path[1] },
    ]
  }

  onHandleDrag(shape: TLElbowArrowShape, info: TLHandleDragEventInfo): TLElbowArrowShape {
    switch (info.handle.id) {
      case 'start':
        return { ...shape, props: { ...shape.props, start: info.point } }
      case 'end':
        return { ...shape, props: { ...shape.props, end: info.point } }
      case 'elbow':
        // Adjust elbow point
        return this.adjustElbow(shape, info.point)
    }
    return shape
  }

  private adjustElbow(shape: TLElbowArrowShape, point: VecLike): TLElbowArrowShape {
    // Recalculate elbow based on handle drag
    const { start } = shape.props
    const path = this.getPath(shape)
    
    // Maintain elbow geometry while allowing adjustment
    return {
      ...shape,
      props: {
        ...shape.props,
        bend: Vec.Dist(point, path[1]),
      },
    }
  }
}
```

---

## 8. Shape Utilities

### 8.1 Geometry Calculations

```typescript
// Vec - 2D Vector Utilities
class Vec {
  constructor(public x: number, public y: number) {}

  // Length
  len(): number {
    return Math.sqrt(this.x * this.x + this.y * this.y)
  }

  // Distance to another point
  dist(v: VecLike): number {
    return Math.sqrt((v.x - this.x) ** 2 + (v.y - this.y) ** 2)
  }

  // Normalize
  norm(): Vec {
    const len = this.len()
    return len === 0 ? new Vec(0, 0) : new Vec(this.x / len, this.y / len)
  }

  // Add
  add(v: VecLike): Vec {
    return new Vec(this.x + v.x, this.y + v.y)
  }

  // Subtract
  sub(v: VecLike): Vec {
    return new Vec(this.x - v.x, this.y - v.y)
  }

  // Scale
  mul(t: number): Vec {
    return new Vec(this.x * t, this.y * t)
  }

  // Dot product
  dot(v: VecLike): number {
    return this.x * v.x + this.y * v.y
  }

  // Cross product (scalar in 2D)
  cross(v: VecLike): number {
    return this.x * v.y - this.y * v.x
  }

  // Rotate around origin
  rot(r: number): Vec {
    const cos = Math.cos(r)
    const sin = Math.sin(r)
    return new Vec(
      this.x * cos - this.y * sin,
      this.x * sin + this.y * cos
    )
  }

  // Rotate around point
  rotWith(center: VecLike, r: number): Vec {
    return this.sub(center).rot(r).add(center)
  }

  // Lerp
  lerp(v: VecLike, t: number): Vec {
    return new Vec(
      this.x + (v.x - this.x) * t,
      this.y + (v.y - this.y) * t
    )
  }

  // Angle to another point
  angle(v: VecLike): number {
    return Math.atan2(v.y - this.y, v.x - this.x)
  }

  // Static helpers
  static Dist(a: VecLike, b: VecLike): number {
    return Math.sqrt((b.x - a.x) ** 2 + (b.y - a.y) ** 2)
  }

  static Lrp(a: VecLike, b: VecLike, t: number): Vec {
    return new Vec(
      a.x + (b.x - a.x) * t,
      a.y + (b.y - a.y) * t
    )
  }

  static Angle(a: VecLike, b: VecLike): number {
    return Math.atan2(b.y - a.y, b.x - a.x)
  }

  static Cross(a: VecLike, b: VecLike): number {
    return a.x * b.y - a.y * b.x
  }

  static Dot(a: VecLike, b: VecLike): number {
    return a.x * b.x + a.y * b.y
  }
}

// Line intersection
function lineIntersect(a: VecLike, b: VecLike, c: VecLike, d: VecLike): Vec | null {
  const denom = (d.y - c.y) * (b.x - a.x) - (d.x - c.x) * (b.y - a.y)
  
  if (denom === 0) return null  // Parallel lines
  
  const ua = ((d.x - c.x) * (a.y - c.y) - (d.y - c.y) * (a.x - c.x)) / denom
  const ub = ((b.x - a.x) * (a.y - c.y) - (b.y - a.y) * (a.x - c.x)) / denom
  
  if (ua >= 0 && ua <= 1 && ub >= 0 && ub <= 1) {
    return new Vec(a.x + ua * (b.x - a.x), a.y + ua * (b.y - a.y))
  }
  
  return null
}

// Point in triangle
function pointInTriangle(p: VecLike, a: VecLike, b: VecLike, c: VecLike, tolerance = 0): boolean {
  const v0 = c.sub(a)
  const v1 = b.sub(a)
  const v2 = p.sub(a)
  
  const dot00 = v0.dot(v0)
  const dot01 = v0.dot(v1)
  const dot02 = v0.dot(v2)
  const dot11 = v1.dot(v1)
  const dot12 = v1.dot(v2)
  
  const invDenom = 1 / (dot00 * dot11 - dot01 * dot01)
  const u = (dot11 * dot02 - dot01 * dot12) * invDenom
  const v = (dot00 * dot12 - dot01 * dot02) * invDenom
  
  return (u >= -tolerance) && (v >= -tolerance) && (u + v <= 1 + tolerance)
}

// Line intersects box
function lineIntersectsBox(a: VecLike, b: VecLike, box: Box): boolean {
  const corners = box.corners
  
  // Check each edge of box
  for (let i = 0; i < 4; i++) {
    const p1 = corners[i]
    const p2 = corners[(i + 1) % 4]
    
    if (lineIntersect(a, b, p1, p2)) {
      return true
    }
  }
  
  // Also check if line is entirely inside box
  return box.containsPoint(a) || box.containsPoint(b)
}
```

### 8.2 Bounds Computation

```typescript
// Box - Axis-Aligned Bounding Box
class Box {
  constructor(public x: number, public y: number, public w: number, public h: number) {}

  get minX(): number { return this.x }
  get maxX(): number { return this.x + this.w }
  get minY(): number { return this.y }
  get maxY(): number { return this.y + this.h }
  get center(): Vec { return new Vec(this.x + this.w / 2, this.y + this.h / 2) }
  
  get corners(): Vec[] {
    return [
      new Vec(this.x, this.y),
      new Vec(this.maxX, this.y),
      new Vec(this.maxX, this.maxY),
      new Vec(this.x, this.maxY),
    ]
  }

  containsPoint(p: VecLike, tolerance = 0): boolean {
    return (
      p.x >= this.minX - tolerance &&
      p.x <= this.maxX + tolerance &&
      p.y >= this.minY - tolerance &&
      p.y <= this.maxY + tolerance
    )
  }

  intersects(other: Box): boolean {
    return (
      this.maxX >= other.minX &&
      this.minX <= other.maxX &&
      this.maxY >= other.minY &&
      this.minY <= other.maxY
    )
  }

  expand(n: number): Box {
    return new Box(this.x - n, this.y - n, this.w + n * 2, this.h + n * 2)
  }

  translate(delta: VecLike): Box {
    return new Box(this.x + delta.x, this.y + delta.y, this.w, this.h)
  }

  // Common bounds of multiple boxes
  static Common(boxes: Box[]): Box {
    if (boxes.length === 0) return new Box(0, 0, 0, 0)
    
    let minX = Infinity
    let minY = Infinity
    let maxX = -Infinity
    let maxY = -Infinity
    
    for (const box of boxes) {
      minX = Math.min(minX, box.minX)
      minY = Math.min(minY, box.minY)
      maxX = Math.max(maxX, box.maxX)
      maxY = Math.max(maxY, box.maxY)
    }
    
    return new Box(minX, minY, maxX - minX, maxY - minY)
  }

  // Bounds from points
  static FromPoints(points: VecLike[]): Box {
    if (points.length === 0) return new Box(0, 0, 0, 0)
    
    let minX = Infinity
    let minY = Infinity
    let maxX = -Infinity
    let maxY = -Infinity
    
    for (const p of points) {
      minX = Math.min(minX, p.x)
      minY = Math.min(minY, p.y)
      maxX = Math.max(maxX, p.x)
      maxY = Math.max(maxY, p.y)
    }
    
    return new Box(minX, minY, maxX - minX, maxY - minY)
  }
}
```

### 8.3 Intersection Testing

```typescript
// Shape-Shape intersection
function shapesIntersect(shapeA: TLShape, shapeB: TLShape): boolean {
  const geomA = getShapeGeometry(shapeA)
  const geomB = getShapeGeometry(shapeB)
  
  // Quick bounds check first
  if (!geomA.bounds.intersects(geomB.bounds)) {
    return false
  }
  
  // Precise geometry intersection
  return geomA.intersects(geomB)
}

// Point-Shape intersection
function pointInShape(point: VecLike, shape: TLShape, tolerance = 0): boolean {
  const geom = getShapeGeometry(shape)
  return geom.containsPoint(point, tolerance)
}

// Line-Shape intersection
function lineIntersectShape(a: VecLike, b: VecLike, shape: TLShape): Vec[] {
  const geom = getShapeGeometry(shape)
  const outline = geom.outline
  
  const intersections: Vec[] = []
  
  for (let i = 0; i < outline.length - 1; i++) {
    const p1 = outline[i]
    const p2 = outline[i + 1]
    
    const intersection = lineIntersect(a, b, p1, p2)
    if (intersection) {
      intersections.push(intersection)
    }
  }
  
  return intersections
}

// Ray casting for point-in-polygon
function pointInPolygon(point: VecLike, polygon: Vec[]): boolean {
  let inside = false
  
  for (let i = 0, j = polygon.length - 1; i < polygon.length; j = i++) {
    const xi = polygon[i].x, yi = polygon[i].y
    const xj = polygon[j].x, yj = polygon[j].y
    
    const intersect = ((yi > point.y) !== (yj > point.y)) &&
      (point.x < (xj - xi) * (point.y - yi) / (yj - yi) + xi)
    
    if (intersect) inside = !inside
  }
  
  return inside
}
```

### 8.4 Snapping

```typescript
// Snap Manager
class SnapManager {
  private editor: Editor

  constructor(editor: Editor) {
    this.editor = editor
  }

  // Find snap lines for a shape
  findSnapLines(shape: TLShape, tolerance = 8): SnapLine[] {
    const shapeBounds = this.editor.getShapePageBounds(shape.id)
    if (!shapeBounds) return []
    
    const snapLines: SnapLine[] = []
    const otherShapes = this.editor.getCurrentPageShapes()
      .filter((s) => s.id !== shape.id)
    
    for (const other of otherShapes) {
      const otherBounds = this.editor.getShapePageBounds(other.id)
      if (!otherBounds) continue
      
      // Check horizontal alignments
      this.checkHorizontalSnap(shapeBounds, otherBounds, tolerance, snapLines)
      
      // Check vertical alignments
      this.checkVerticalSnap(shapeBounds, otherBounds, tolerance, snapLines)
      
      // Check center alignments
      this.checkCenterSnap(shapeBounds, otherBounds, tolerance, snapLines)
    }
    
    return snapLines
  }

  private checkHorizontalSnap(a: Box, b: Box, tolerance: number, lines: SnapLine[]): void {
    // Top edge
    if (Math.abs(a.minY - b.minY) <= tolerance) {
      lines.push({ type: 'horizontal', y: a.minY, offset: a.minY - b.minY })
    }
    // Bottom edge
    if (Math.abs(a.maxY - b.maxY) <= tolerance) {
      lines.push({ type: 'horizontal', y: a.maxY, offset: a.maxY - b.maxY })
    }
    // Top to bottom
    if (Math.abs(a.minY - b.maxY) <= tolerance) {
      lines.push({ type: 'horizontal', y: a.minY, offset: a.minY - b.maxY })
    }
    // Bottom to top
    if (Math.abs(a.maxY - b.minY) <= tolerance) {
      lines.push({ type: 'horizontal', y: a.maxY, offset: a.maxY - b.minY })
    }
  }

  private checkVerticalSnap(a: Box, b: Box, tolerance: number, lines: SnapLine[]): void {
    // Left edge
    if (Math.abs(a.minX - b.minX) <= tolerance) {
      lines.push({ type: 'vertical', x: a.minX, offset: a.minX - b.minX })
    }
    // Right edge
    if (Math.abs(a.maxX - b.maxX) <= tolerance) {
      lines.push({ type: 'vertical', x: a.maxX, offset: a.maxX - b.maxX })
    }
    // Left to right
    if (Math.abs(a.minX - b.maxX) <= tolerance) {
      lines.push({ type: 'vertical', x: a.minX, offset: a.minX - b.maxX })
    }
    // Right to left
    if (Math.abs(a.maxX - b.minX) <= tolerance) {
      lines.push({ type: 'vertical', x: a.maxX, offset: a.maxX - b.minX })
    }
  }

  private checkCenterSnap(a: Box, b: Box, tolerance: number, lines: SnapLine[]): void {
    const aCenter = a.center
    const bCenter = b.center
    
    // Horizontal center
    if (Math.abs(aCenter.x - bCenter.x) <= tolerance) {
      lines.push({ type: 'vertical', x: aCenter.x, offset: aCenter.x - bCenter.x })
    }
    
    // Vertical center
    if (Math.abs(aCenter.y - bCenter.y) <= tolerance) {
      lines.push({ type: 'horizontal', y: aCenter.y, offset: aCenter.y - bCenter.y })
    }
  }

  // Snap point to grid
  snapToGrid(point: VecLike, gridSize: number): Vec {
    return new Vec(
      Math.round(point.x / gridSize) * gridSize,
      Math.round(point.y / gridSize) * gridSize
    )
  }
}

interface SnapLine {
  type: 'horizontal' | 'vertical'
  x?: number
  y?: number
  offset: number
}
```

### 8.5 Alignment

```typescript
// Align shapes
function alignShapes(
  editor: Editor,
  alignment: 'left' | 'right' | 'center' | 'top' | 'bottom' | 'middle'
): void {
  const selectedShapes = editor.getSelectedShapes()
  if (selectedShapes.length < 2) return
  
  const bounds = editor.getSelectionPageBounds()
  if (!bounds) return
  
  const updates: Partial<TLShape>[] = []
  
  for (const shape of selectedShapes) {
    const shapeBounds = editor.getShapePageBounds(shape.id)!
    let newX = shape.x
    let newY = shape.y
    
    switch (alignment) {
      case 'left':
        newX = bounds.x + (shape.x - shapeBounds.x)
        break
      case 'right':
        newX = bounds.maxX - shapeBounds.w + (shape.x - shapeBounds.x)
        break
      case 'center':
        newX = bounds.x + bounds.w / 2 - shapeBounds.w / 2 + (shape.x - shapeBounds.x)
        break
      case 'top':
        newY = bounds.y + (shape.y - shapeBounds.y)
        break
      case 'bottom':
        newY = bounds.maxY - shapeBounds.h + (shape.y - shapeBounds.y)
        break
      case 'middle':
        newY = bounds.y + bounds.h / 2 - shapeBounds.h / 2 + (shape.y - shapeBounds.y)
        break
    }
    
    updates.push({ id: shape.id, x: newX, y: newY })
  }
  
  editor.updateShapes(updates)
}

// Distribute shapes evenly
function distributeShapes(
  editor: Editor,
  direction: 'horizontal' | 'vertical'
): void {
  const selectedShapes = editor.getSelectedShapes()
  if (selectedShapes.length < 3) return
  
  const bounds = editor.getSelectionPageBounds()!
  const sortedShapes = selectedShapes.sort((a, b) => {
    const aBounds = editor.getShapePageBounds(a.id)!
    const bBounds = editor.getShapePageBounds(b.id)!
    return direction === 'horizontal'
      ? aBounds.x - bBounds.x
      : aBounds.y - bBounds.y
  })
  
  // First and last shapes stay in place
  const first = sortedShapes[0]
  const last = sortedShapes[sortedShapes.length - 1]
  
  const firstBounds = editor.getShapePageBounds(first.id)!
  const lastBounds = editor.getShapePageBounds(last.id)!
  
  const totalSpace = direction === 'horizontal'
    ? lastBounds.maxX - firstBounds.x
    : lastBounds.maxY - firstBounds.y
  
  const shapeSizes = sortedShapes.map((s) => 
    direction === 'horizontal'
      ? editor.getShapePageBounds(s.id)!.w
      : editor.getShapePageBounds(s.id)!.h
  )
  
  const totalShapeSize = shapeSizes.reduce((a, b) => a + b, 0)
  const gapSize = (totalSpace - totalShapeSize) / (sortedShapes.length - 1)
  
  const updates: Partial<TLShape>[] = []
  let currentPosition = direction === 'horizontal' ? firstBounds.x : firstBounds.y
  
  for (let i = 1; i < sortedShapes.length - 1; i++) {
    const shape = sortedShapes[i]
    const shapeSize = shapeSizes[i]
    
    currentPosition += shapeSizes[i - 1] + gapSize
    
    if (direction === 'horizontal') {
      updates.push({ 
        id: shape.id, 
        x: currentPosition + (shape.x - editor.getShapePageBounds(shape.id)!.x)
      })
    } else {
      updates.push({ 
        id: shape.id, 
        y: currentPosition + (shape.y - editor.getShapePageBounds(shape.id)!.y)
      })
    }
    
    currentPosition += shapeSize
  }
  
  editor.updateShapes(updates)
}
```

---

## ShapeUtil API Reference

### Core Methods

| Method | Required | Description |
|--------|----------|-------------|
| `create(props)` | Yes | Create a new shape instance |
| `geometry(shape)` | Yes | Return Geometry2d for bounds/hit-testing |
| `component(shape)` | Yes | Return SVG JSX for rendering |
| `backgroundComponent?(shape)` | No | Return background JSX (rendered below) |
| `migrate?(shape)` | No | Migrate shape from older versions |
| `upgrade?(shape)` | No | Upgrade props during schema changes |
| `snapshot?(shape)` | No | Return snapshot for undo/redo |

### Event Handlers

| Method | Description |
|--------|-------------|
| `onPointerDown?(shape, info)` | Handle pointer down on shape |
| `onPointerMove?(shape, info)` | Handle pointer move on shape |
| `onPointerUp?(shape, info)` | Handle pointer up on shape |
| `onKeyDown?(shape, info)` | Handle key down on shape |
| `onKeyUp?(shape, info)` | Handle key up on shape |
| `onDragStart?(shape, info)` | Handle drag start |
| `onDrag?(shape, info)` | Handle ongoing drag |
| `onDragEnd?(shape, info)` | Handle drag end |
| `onHandleDrag?(shape, info)` | Handle handle dragging |

### Handle & Binding Methods

| Method | Description |
|--------|-------------|
| `getHandles?(shape)` | Return interactive handles |
| `getBindingPoints?(shape)` | Return connection points for arrows |

---

## Sources

- tldraw GitHub Repository: https://github.com/tldraw/tldraw
- tldraw Documentation: https://tldraw.dev/docs
- ShapeUtil API: https://tldraw.dev/docs/api#ShapeUtil
- Geometry Primitives: `packages/editor/src/lib/geometry/`
- Default Shapes: `packages/tldraw/src/lib/shapes/`
