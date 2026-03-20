---
location: /home/darkvoid/Boxxed/@formulas/src.UIFrameworks/src.animations/theatre
repository: https://github.com/theatre-js/theatre
explored_at: 2026-03-20
---

# Theatre.js Deep Exploration

## Project Overview

Theatre.js is a motion design library for high-fidelity animations on the web. Unlike typical animation libraries that focus on simple transitions, Theatre.js provides a **studio-grade animation editor** with a visual timeline interface, keyframe editing, and bezier curve manipulation - similar to professional tools like After Effects but running in the browser.

### Key Differentiators

1. **Dual Interface**: Programmatic API + Visual Studio Editor
2. **Keyframe-based Animation**: Timeline-driven with precise control
3. **Bezier Curve Easing**: Visual curve editor for custom easing functions
4. **Multi-target Support**: DOM, React, Three.js, and arbitrary JS variables
5. **Reactive Data System**: Built on `@theatre/dataverse` - a reactive atom/prism system
6. **Production Workflow**: Export state for production, use studio only during development

---

## Architecture Breakdown

```mermaid
graph TB
    subgraph "Core Bundle @theatre/core"
        Core[Core Exports]
        Project[Project]
        Sheet[Sheet]
        SheetObject[SheetObject]
        Sequence[Sequence]
        Dafaverse[Dataverse Integration]
    end

    subgraph "Studio Bundle @theatre/studio"
        StudioUI[UI Components]
        SequenceEditor[Sequence Editor Panel]
        DetailPanel[Detail Panel]
        OutlinePanel[Outline Panel]
        Store[StudioStore]
        Sync[SyncServer]
    end

    subgraph "Dataverse @theatre/dataverse"
        Atom[Atom - State Container]
        Prism[Prism - Reactive Computation]
        Ticker[Ticker - RAF Scheduler]
        Pointer[Pointer - Path Navigation]
    end

    subgraph "React Integration"
        R3F[@theatre/r3f - Three.js]
        React[@theatre/react]
    end

    Core --> Dafaverse
    StudioUI --> Store
    Store --> Core
    Dafaverse --> Atom
    Dafaverse --> Prism
    Dafaverse --> Ticker
    R3F --> Core
    React --> Core
    SequenceEditor --> Sequence
    DetailPanel --> SheetObject
```

### Monorepo Structure

```
theatre/
├── packages/
│   ├── core/           # Runtime animation engine (Apache License)
│   ├── studio/         # Visual editor UI (AGPL License)
│   ├── dataverse/      # Reactive state system
│   ├── react/          # React hooks and components
│   ├── r3f/            # React Three Fiber integration
│   ├── sync-server/    # Backend for studio sync
│   ├── app/            # Next.js app for studio hosting
│   ├── theatric/       # Alternative API surface
│   └── utils/          # Shared utilities
├── examples/           # Usage examples
├── playground/         # Development playground
└── devEnv/             # Build tools and CLI
```

---

## Core Animation System

### 1. Ticker System (Heartbeat)

The `Ticker` class in `@theatre/dataverse` is the heartbeat of all animations:

```typescript
// Ticker.ts - Core scheduling mechanism
export const EMPTY_TICKS_BEFORE_GOING_DORMANT = 60 * 3 // 3 seconds at 60fps

class Ticker {
  private _scheduledForThisOrNextTick: Set<ICallback>
  private _scheduledForNextTick: Set<ICallback>
  private _ticking: boolean = false
  private _dormant: boolean = true

  tick(t: number = performance.now()) {
    // Go dormant when no callbacks - saves battery
    if (this._scheduledForNextTick.size === 0 &&
        this._scheduledForThisOrNextTick.size === 0) {
      this._numberOfDormantTicks++
      if (this._numberOfDormantTicks >= EMPTY_TICKS_BEFORE_GOING_DORMANT) {
        this._goDormant()
        return
      }
    }

    // Process callbacks with recursive tick support
    this._ticking = true
    this._timeAtCurrentTick = t
    for (const v of this._scheduledForNextTick) {
      this._scheduledForThisOrNextTick.add(v)
    }
    this._scheduledForNextTick.clear()
    this._tick(0)
    this._ticking = false
  }

  private _tick(iterationNumber: number): void {
    const time = this.time
    const oldSet = this._scheduledForThisOrNextTick
    this._scheduledForThisOrNextTick = new Set()
    for (const fn of oldSet) {
      fn(time)
    }
    // Recursive tick if new callbacks were scheduled during execution
    if (this._scheduledForThisOrNextTick.size > 0) {
      return this._tick(iterationNumber + 1)
    }
  }
}
```

**Key Design Decisions:**
- **Dormancy**: Goes dormant after 3 seconds of inactivity to save battery
- **Recursive Ticking**: Handles callbacks scheduled during tick execution (up to 100 iterations)
- **Two-phase Scheduling**: `onThisOrNextTick` vs `onNextTick` for precise ordering

### 2. RafDriver (Animation Frame Driver)

Custom RAF drivers allow Theatre to sync with other animation systems:

```typescript
// rafDrivers.ts
function createBasicRafDriver(): IRafDriver {
  let rafId: number | null = null

  const start = (): void => {
    if (typeof window !== 'undefined') {
      const onAnimationFrame = (t: number) => {
        driver.tick(t)
        rafId = window.requestAnimationFrame(onAnimationFrame)
      }
      rafId = window.requestAnimationFrame(onAnimationFrame)
    }
  }

  const stop = (): void => {
    if (rafId !== null) {
      window.cancelAnimationFrame(rafId)
    }
  }

  const driver = createRafDriver({name: 'DefaultCoreRafDriver', start, stop})
  return driver
}
```

**Use Cases:**
- Sync with Three.js `requestAnimationFrame`
- XR frame loops via `xrSession.requestAnimationFrame`
- Manual stepping for benchmarking/recording

---

## Sequence and Keyframe System

### Sequence Class - Timeline Controller

```typescript
// Sequence.ts - Timeline and playback management
class Sequence implements PointerToPrismProvider {
  private _playbackControllerBox: Atom<IPlaybackController>
  private _positionD: Prism<number>
  private _lengthD: Prism<number>

  // Playback with configuration
  async play(conf: Partial<{
    iterationCount: number      // How many times to loop
    range: IPlaybackRange       // [start, end] in seconds
    rate: number               // Playback speed multiplier
    direction: IPlaybackDirection // 'normal' | 'reverse' | 'alternate' | 'alternateReverse'
  }>, ticker: Ticker): Promise<boolean>

  // Set position manually
  set position(requestedPosition: number) {
    this.pause()
    if (position > this.length) position = this.length
    this._playbackControllerBox.get().gotoPosition(position)
  }

  // Snap to grid for UI alignment
  closestGridPosition = (posInUnitSpace: number): number => {
    const subUnitsPerUnit = this.subUnitsPerUnit
    const gridLength = 1 / subUnitsPerUnit
    return parseFloat(
      (Math.round(posInUnitSpace / gridLength) * gridLength).toFixed(3)
    )
  }
}
```

### Playback Directions

| Direction | Behavior |
|-----------|----------|
| `normal` | Forward from start to end |
| `reverse` | Backward from end to start |
| `alternate` | Forward then backward (ping-pong) |
| `alternateReverse` | Backward then forward |

### Time-Based Position Formatter

```typescript
class TimeBasedPositionFormatter {
  constructor(private readonly _fps: number) {}

  formatForPlayhead(posInUnitSpace: number): string {
    let p = posInUnitSpace
    let s = ''

    if (p >= hour) {
      const hours = Math.floor(p / hour)
      s += padStart(hours.toString(), 2, '0') + 'h'
      p = p % hour
    }
    if (p >= minute) {
      const minutes = Math.floor(p / minute)
      s += padStart(minutes.toString(), 2, '0') + 'm'
      p = p % minute
    }
    if (p >= second) {
      const seconds = Math.floor(p / second)
      s += padStart(seconds.toString(), 2, '0') + 's'
      p = p % second
    }

    // Convert to frames based on FPS
    const frameLength = 1 / this._fps
    if (p >= frameLength) {
      const frames = Math.round(p / frameLength)
      s += padStart(frames.toString(), 2, '0') + 'f'
    }
    return s
  }
}

const second = 1
const minute = second * 60
const hour = minute * 60
```

**Example Output:** `01h23m45s12f` (1 hour, 23 minutes, 45 seconds, 12 frames)

---

## Sheet Object System - Layered Value Composition

Theatre uses a **4-layer composition system** for computing final property values:

```typescript
// SheetObject.ts - Value layering
getValues(): Prism<Pointer<SheetObjectPropsValue>> {
  return prism(() => {
    // Layer 1: Default values from prop type config (rarely changes)
    const defaults = val(this.template.getDefaultValues())

    // Layer 2: Initial value set by user via sheetObject.initialValue
    const initial = val(this._initialValue.pointer)
    const withInitial = deepMergeWithCache(defaults, initial, withInitialCache)

    // Layer 3: Static overrides (same across all instances)
    const statics = val(this.template.getStaticValues())
    const withStatics = deepMergeWithCache(withInitial, statics, withStaticsCache)

    // Layer 4: Sequenced values (change every frame when playing)
    const sequenced = val(val(pointerToSequencedValuesD))
    const final = deepMergeWithCache(withStatics, sequenced, withSeqsCache)

    return valToAtom('finalAtom', final).pointer
  })
}
```

### Performance Optimizations

1. **Layered Caching**: Each merge level has its own `WeakMap` cache
2. **Stable WeakMaps**: Uses `prism.memo()` to maintain cache identity
3. **Sorted by Volatility**: Least-volatile layers computed first
4. **Prism Caching**: Result is cached per SheetObject

---

## Dataverse - Reactive Data System

### Atom - State Container with Path-based Subscription

```typescript
// Atom.ts - Fine-grained reactivity
class Atom<State> {
  private _currentState: State
  private _rootScope: Scope  // Tree structure for path subscriptions

  set(newState: State) {
    const oldState = this._currentState
    this._currentState = newState
    this._checkUpdates(this._rootScope, oldState, newState)
  }

  // Subscribe to changes at a specific path
  onChangeByPointer<S>(
    pointerOrFn: Pointer<S> | ((p: Pointer<State>) => Pointer<S>),
    cb: (v: S) => void,
  ): () => void {
    const {path} = getPointerParts(pointer)
    const scope = this._getOrCreateScopeForPath(path)
    scope.identityChangeListeners.add(cb)
    return () => scope.identityChangeListeners.delete(cb)
  }

  // Internal: Propagate changes up the scope tree
  private _checkUpdates(scope: Scope, oldState: unknown, newState: unknown) {
    if (oldState === newState) return

    // Notify listeners at this level
    for (const cb of scope.identityChangeListeners) {
      cb(newState)
    }

    // Recurse to children if object/array
    for (const [childKey, childScope] of scope.children) {
      const oldChildVal = getKeyOfValue(oldState, childKey)
      const newChildVal = getKeyOfValue(newState, childKey)
      this._checkUpdates(childScope, oldChildVal, newChildVal)
    }
  }
}
```

### Scope Tree Structure

```
Atom({ a: { b: { c: 1 } } })

Scope Tree:
rootScope
└── "a"
    └── "b"
        └── "c" ← onChangeByPointer(pointer.a.b.c, cb)
```

### Prism - Composable Reactive Computations

```typescript
// prism.ts - Derived state
const prism = {
  // Create a derived computation
  source: <T>(subscribe: () => () => void, getValue: () => T): Prism<T> => {...},

  // Memoize within a prism context
  memo: <T>(key: string, factory: () => T, deps: any[]): T => {...},

  // Side effects with cleanup
  effect: <T>(key: string, compute: () => T, deps: any[]): void => {...}
}

// Example usage
const interpolatedValue = prism(() => {
  const triple = val(sequencePositionPrism)
  const left = deserializeAndSanitize(triple.left)
  const right = deserializeAndSanitize(triple.right)
  return interpolate(left, right, triple.progression)
})
```

---

## Studio Editor - Visual Animation Interface

### Sequence Editor Panel Architecture

```
SequenceEditorPanel/
├── DopeSheet/
│   ├── Left/           # Property list sidebar
│   │   ├── SheetObjectRow.tsx
│   │   ├── PropWithChildrenRow.tsx
│   │   └── PrimitivePropRow.tsx
│   └── Right/          # Timeline visualization
│       ├── BasicKeyframedTrack/
│       │   └── KeyframeEditor/
│       │       └── CurveEditorPopover/
│       │           ├── CurveEditorPopover.tsx
│       │           └── CurveSegmentEditor.tsx
│       ├── AggregatedKeyframeTrack/
│       └── DopeSheetBackground.tsx
├── GraphEditor/        # Function curve view
└── FrameGrid/          # Time grid visualization
```

### Curve Editor - Bezier Handle Manipulation

The curve editor allows visual editing of cubic bezier easing curves:

```typescript
// CurveSegmentEditor.tsx - SVG-based bezier visualization
const curvePathDAttrValue = (connection) =>
  `M0 ${toExtremumSpace(1)}
   C${connection.left.handles[2]} ${toExtremumSpace(1 - connection.left.handles[3])}
    ${connection.right.handles[0]} ${toExtremumSpace(1 - connection.right.handles[1])}
    1 ${toExtremumSpace(0)}`

// Cubic bezier: M start C cp1 cp2 end
// Where handles are [x1, y1, x2, y2] in normalized 0-1 space
```

### Bezier Handle Structure

```typescript
type CubicBezierHandles = [
  number, // x1 (0-1) - First control point X
  number, // y1 (0-1) - First control point Y
  number, // x2 (0-1) - Second control point X
  number  // y2 (0-1) - Second control point Y
]

// CSS representation: cubic-bezier(x1, y1, x2, y2)
// Example: cubic-bezier(0.25, 0.1, 0.25, 1) = "ease"
```

### Extremum Space Scaling

The editor dynamically scales its viewBox to keep handles visible:

```typescript
const minY = Math.min(0, 1 - right.handles[1], 1 - left.handles[3])
const maxY = Math.max(1, 1 - right.handles[1], 1 - left.handles[3])
const h = Math.max(1, maxY - minY)
const toExtremumSpace = (y: number) => (y - minY) / h
```

This creates a "stretching space" effect when dragging handles outside 0-1 bounds.

### Easing Presets System

```typescript
// shared.ts - Built-in easing presets
const EASING_PRESETS = [
  { label: 'Linear', value: 'cubic-bezier(0, 0, 1, 1)' },
  { label: 'Ease', value: 'cubic-bezier(0.25, 0.1, 0.25, 1)' },
  { label: 'Ease-In', value: 'cubic-bezier(0.42, 0, 1, 1)' },
  { label: 'Ease-Out', value: 'cubic-bezier(0, 0, 0.58, 1)' },
  { label: 'Ease-In-Out', value: 'cubic-bezier(0.42, 0, 0.58, 1)' },
  // ... 50+ more presets
]

// Search with fuzzy matching
const displayedPresets = useMemo(() => {
  if (/^[A-Za-z]/.test(inputValue)) {
    return fuzzy.filter(inputValue, EASING_PRESETS, {extract: el => el.label})
  }
  return EASING_PRESETS
}, [inputValue])
```

### Keyframe Types

| Type | Description | Visual |
|------|-------------|--------|
| `bezier` | Smooth curve with handles | Curved line |
| `hold` | Instant step, no interpolation | Step function |
| `linear` | Straight line between keys | Straight line |

---

## Easing Mathematics

### Cubic Bezier Formula

The core easing calculation uses cubic bezier curves:

```coffeescript
# From bezier-easing implementation
A = (aA1, aA2) -> 1.0 - 3.0 * aA2 + 3.0 * aA1
B = (aA1, aA2) -> 3.0 * aA2 - 6.0 * aA1
C = (aA1) -> 3.0 * aA1

calcBezier = (aT, aA1, aA2) ->
  ((A(aA1, aA2) * aT + B(aA1, aA2)) * aT + C(aA1)) * aT

# For Y value given t:
y(t) = (1-3t2+3t1) * t³ + (3t2-6t1) * t² + (3t1) * t
```

### Newton-Raphson Iteration

To find `t` given `x` (inverse of bezier X):

```typescript
const NEWTON_ITERATIONS = 4
const NEWTON_MIN_SLOPE = 0.001

function newtonRaphsonIterate(aX: number, aGuessT: number): number {
  for (let i = 0; i < NEWTON_ITERATIONS; ++i) {
    const currentSlope = getSlope(aGuessT, mX1, mX2)
    if (currentSlope === 0) return aGuessT
    const currentX = calcBezier(aGuessT, mX1, mX2) - aX
    aGuessT -= currentX / currentSlope
  }
  return aGuessT
}
```

### Binary Search Optimization

Pre-compute 11 sample values for binary search before Newton refinement:

```typescript
const kSplineTableSize = 11
const kSampleStepSize = 1.0 / (kSplineTableSize - 1.0)

// Pre-compute: samples[i] = calcBezier(i * step, mX1, mX2)
// Binary search to find interval
// Newton-Raphson for precise t value
```

---

## Keyframe Interpolation System

### Interpolation Triple

At any sequence position, the system computes:

```typescript
type InterpolationTriple = {
  left: BasicKeyframe      // Keyframe before current position
  right: BasicKeyframe     // Keyframe after current position
  progression: number      // 0-1 progress between left and right
}

// From interpolationTripleAtPosition.ts
function interpolationTripleAtPosition(
  trackPrism: Prism<SequenceTrack>,
  timePrism: Prism<number>
): Prism<InterpolationTriple | undefined> {
  return prism(() => {
    const track = val(trackPrism)
    const time = val(timePrism)
    const keyframes = getSortedKeyframes(track.keyframes)

    // Find surrounding keyframes
    const leftIndex = keyframes.findIndex(k => k.position > time) - 1
    const left = keyframes[leftIndex]
    const right = keyframes[leftIndex + 1]

    if (!left || !right) return undefined

    const progression = (time - left.position) / (right.position - left.position)
    return {left, right, progression}
  })
}
```

### Keyframe Structure

```typescript
type BasicKeyframe = {
  id: KeyframeId
  position: number        // Time in seconds
  type: 'bezier' | 'hold' | 'linear'
  value: SerializableValue

  // Bezier handles (4 points for cubic bezier)
  handles: [
    number, number,  // Left keyframe: inX, inY
    number, number,  // Left keyframe: outX, outY
    number, number,  // Right keyframe: inX, inY
    number, number   // Right keyframe: outX, outY
  ]
}
```

---

## Transaction System - State Editing

### Studio Store Transactions

All studio edits go through transactions for undo/redo support:

```typescript
// CurveEditorPopover.tsx - Transaction example
function transactionSetCubicBezier(
  keyframeConnections: Array<KeyframeConnectionWithAddress>,
  handles: CubicBezierHandles,
): CommitOrDiscardOrRecapture {
  return getStudio().tempTransaction(({stateEditors}) => {
    const {setHandlesForKeyframe, setKeyframeType} =
      stateEditors.coreByProject.historic.sheetsById.sequence

    for (const {projectId, sheetId, objectKey, trackId, left, right}
         of keyframeConnections) {
      setHandlesForKeyframe({
        projectId, sheetId, objectKey, trackId,
        keyframeId: left.id,
        start: [handles[0], handles[1]]
      })
      setHandlesForKeyframe({
        projectId, sheetId, objectKey, trackId,
        keyframeId: right.id,
        end: [handles[2], handles[3]]
      })
      setKeyframeType({
        projectId, sheetId, objectKey, trackId,
        keyframeId: left.id,
        keyframeType: 'bezier'
      })
    }
  })
}

// Usage with discard on escape
const tempTransaction = useRef<CommitOrDiscardOrRecapture | null>(null)
useEffect(() => {
  return () => {
    // Commit on unmount (popover close)
    tempTransaction.current?.commit()
  }
}, [])

// Discard on escape key
const onEscape = () => {
  tempTransaction.current?.discard()
}
```

---

## Project State Management

### State Structure

```typescript
type OnDiskState = {
  definitionVersion: string  // '0.4.0' - for migration detection
  sheetsById: {
    [sheetId: string]: {
      sequence: {
        length: number              // Duration in seconds
        subUnitsPerUnit: number     // Grid resolution (default: 30)
        tracksByObject: {
          [objectKey: string]: {
            trackIdByPropPath: {[propPath: string]: SequenceTrackId}
            trackData: {
              [trackId: string]: {
                keyframes: BasicKeyframe[]
              }
            }
          }
        }
      }
    }
  }
  revisionHistory: Revision[]
}
```

### Project Attachment Flow

```typescript
// Project.ts - Studio attachment
class Project {
  attachToStudio(studio: Studio) {
    studio.initialized.then(async () => {
      await initialiseProjectState(studio, this, this.config.state)

      // Redirect pointers to studio's atom
      this._pointerProxies.historic.setPointer(
        studio.atomP.historic.coreByProject[this.address.projectId]
      )

      this._pointerProxies.ephemeral.setPointer(
        studio.ephemeralAtom.pointer.coreByProject[this.address.projectId]
      )

      // Initialize asset storage
      await studio.createAssetStorage(this, this.config.assets?.baseUrl)
    })
  }
}
```

---

## React Integration Patterns

### useSequenceFor Playback

```typescript
import {useSequence} from '@theatre/react'

function MyComponent() {
  const sequence = useSequence(sheet, 'default')

  const playAnimation = async () => {
    await sequence.play({
      iterationCount: Infinity,
      range: [0, sequence.length],
      rate: 1,
      direction: 'normal'
    })
  }

  return <button onClick={playAnimation}>Play</button>
}
```

### useValue for Reactive Values

```typescript
import {useValue} from '@theatre/react'

function AnimatedObject() {
  const values = useValue(sheetObject.props)

  return (
    <div style={{
      transform: `translateX(${values.x}px) rotate(${values.rotation}deg)`
    }} />
  )
}
```

---

## Performance Considerations

### 1. Prism Memoization

```typescript
const expensiveValue = prism(() => {
  const config = val(configPrism)
  const data = val(dataPrism)

  // Cache result within this prism's lifetime
  const cached = prism.memo('expensive', () => compute(config, data), [config, data])
  return cached
})
```

### 2. WeakMap Caching for Deep Merges

```typescript
const withInitialCache = prism.memo(
  'withInitialCache',
  () => new WeakMap(),
  [],
)
const withInitial = deepMergeWithCache(defaults, initial, withInitialCache)
```

### 3. Dormant Ticker

When no animations are playing, the ticker goes dormant after 3 seconds, preventing unnecessary RAF calls.

---

## File Structure Deep Dive

### Core Package Entry Points

```
packages/core/src/
├── index.ts              # Main exports, studio connection
├── CoreBundle.ts         # Bundle registration
├── coreTicker.ts         # Get/set core's RAF driver
├── rafDrivers.ts         # createRafDriver()
├── globals.ts            # Version and global variable names
├── projects/
│   ├── Project.ts        # Project class, state management
│   └── TheatreProject.ts # Public API wrapper
├── sheets/
│   ├── Sheet.ts          # Sheet class, object factory
│   └── TheatreSheet.ts   # Public API wrapper
├── sheetObjects/
│   ├── SheetObject.ts    # Object with layered values
│   └── TheatreSheetObject.ts # Public API wrapper
├── sequences/
│   ├── Sequence.ts       # Playback controller
│   ├── TheatreSequence.ts # Public API wrapper
│   └── interpolationTripleAtPosition.ts
├── propTypes/
│   ├── index.ts          # Prop type definitions
│   └── utils.ts          # getPropConfigByPath()
└── utils/
    ├── keyframeUtils.ts  # Keyframe sorting/caching
    ├── instanceTypes.ts  # Type guards
    └── notify.ts         # User notifications
```

### Studio Package UI Structure

```
packages/studio/src/
├── StudioStore/
│   ├── StudioStore.ts    # Main store, transactions
│   └── createTransactionPrivateApi.ts
├── SyncStore/
│   ├── SyncServerLink.ts # WebSocket sync
│   └── AppLink.ts        # Local app communication
├── panels/
│   ├── SequenceEditorPanel/
│   │   ├── DopeSheet/    # Timeline view
│   │   ├── GraphEditor/  # Function curves
│   │   └── FrameGrid/    # Time grid
│   ├── DetailPanel/      # Property inspector
│   ├── OutlinePanel/     # Object hierarchy
│   └── BasePanel/        # Shared panel UI
└── UI/
    ├── UIRoot.tsx        # Theme, pointer capture
    └── PanelsRoot.tsx    # Panel layout
```

---

## Key Insights

### 1. Separation of Concerns

- **Core** = Runtime (Apache License) - Ships to production
- **Studio** = Editor (AGPL License) - Development only

This means you design in Studio, export state, and only Core goes to users.

### 2. Dataverse Abstraction

The reactive system (Atom/Prism/Ticker) is decoupled from animation logic. This enables:
- Tree-shakable computations
- Automatic dependency tracking
- Coordinated updates via Ticker

### 3. Value Layering Architecture

The 4-layer composition (defaults → initial → static → sequenced) allows:
- Non-destructive editing
- Instance overrides (initialValue)
- Shared static config across instances
- Runtime sequence playback

### 4. Transaction-Based Editing

All studio edits use transactions enabling:
- Undo/Redo stack
- Live preview with discard
- Batch operations
- Collaborative sync

### 5. SVG-Based Curve Editor

The curve editor uses SVG paths with:
- Dynamic viewBox scaling (extremum space)
- CSS gradient fills for visual feedback
- Hit zones for handle dragging
- Background curves for multi-value comparison

---

## Open Questions / Areas for Deeper Dive

1. **Sync Server Protocol**: How does the WebSocket sync work for collaborative editing?
2. **Asset Storage**: How are blob assets (images, sounds) stored and referenced?
3. **Action System**: The `actions` parameter in `createObject` - how does the callback system work?
4. **Three.js Integration**: How does `@theatre/r3f` handle Three.js object property mapping?
5. **Performance at Scale**: How does the system handle 1000+ animated properties?

---

## Summary

Theatre.js is a sophisticated animation system combining:

- **Reactive Foundation**: Dataverse atoms/prisms for fine-grained reactivity
- **Timeline Engine**: Sequence-based keyframe animation with bezier interpolation
- **Visual Editor**: Professional-grade curve editor and dope sheet
- **Production Ready**: State export, typed APIs, and SSR compatibility

The architecture prioritizes:
1. **Precision**: Frame-accurate timing with sub-unit resolution
2. **Composability**: Prism-based computations that chain and cache
3. **Flexibility**: Custom RAF drivers, multiple playback directions
4. **Developer Experience**: Visual editing with programmatic fallback
