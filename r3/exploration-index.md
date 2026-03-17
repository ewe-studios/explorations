# R3 Reactivity System - Exploration Index

This exploration provides a complete understanding of the r3 reactive system located at:
`/home/darkvoid/Boxxed/@formulas/src.UIFrameworks/src.reactivity/r3/`

---

## Documents

### 1. [exploration.md](./exploration.md) - 5-Minute Distillation
**Start here** - Core concepts explained in under 5 minutes:
- The three pillars of R3
- Height-based topological ordering
- The dirty heap (bucket queue)
- Three-color marking (Check/Dirty/None)
- Core data structures visualized
- Push-pull-push vs R3's approach
- Performance characteristics

### 2. [visual-guide.md](./visual-guide.md) - Visual Diagrams
Visual representations of:
- System architecture overview
- Link structure (bidirectional connections)
- The stabilize() loop step-by-step
- Three-color marking flow
- Dynamic dependency switching
- Firewall signals and component boundaries
- Height adjustment mechanics
- Memory layout and linked lists
- Execution traces

### 3. [function-dive.md](./function-dive.md) - Source Code Deep Dive
Line-by-line analysis of every function in `src/index.ts`:
- Type definitions (Lines 1-44)
- Global state management
- Heap operations (insert/delete)
- Core API (signal, computed, read, setSignal)
- Recomputation engine
- Dependency linking/unlinking
- Marking system
- Stabilization algorithm
- Cleanup callbacks

### 4. [test-walkthrough.md](./test-walkthrough.md) - Test Case Traces
Annotated walkthrough of every test in `test/basic.test.ts`:
- Execution traces showing internal state
- Dependency graph changes
- Heap operations during stabilize()
- Edge cases explained

---

## Quick Reference

### Core Concepts

```
┌─────────────────────────────────────────────────────────────┐
│                    R3 IN ONE PAGE                            │
├─────────────────────────────────────────────────────────────┤
│                                                              │
│  SIGNALS: Sources of truth, hold values                     │
│  COMPUTED: Derived values, track dependencies               │
│  LINKS: Bidirectional edges in dependency graph             │
│                                                              │
│  HEIGHT: Topological level (parent + 1)                     │
│  DIRTY HEAP: Bucket queue organized by height               │
│  FLAGS: None → Check → Dirty → (recompute) → None          │
│                                                              │
│  read():   Track dependency, pull updates if needed         │
│  set():    Update value, mark subscribers                   │
│  stabilize(): Process heap from height 0 to max            │
│                                                              │
│  link():   Subscribe to dependency                          │
│  unlink(): Unsubscribe from dependency                      │
│                                                              │
│  Key insight: Height ordering prevents glitches            │
│  Key optimization: Check vs Dirty enables short-circuit    │
└─────────────────────────────────────────────────────────────┘
```

### Data Flow

```
Signal Change
     │
     ▼
setSignal() → Update value
     │
     ▼
For each subscriber: insertIntoHeap()
     │
     ▼
stabilize() processes heap by height:
     │
     ├─► recompute() each node
     │    │
     │    ├─► Run disposal (cleanup)
     │    ├─► Execute fn() → tracks new deps
     │    ├─► Cleanup stale deps
     │    └─► If value changed: mark children
     │
     └─► Continue until heap empty
```

### Key Files

| File | Purpose |
|------|---------|
| `src/index.ts` | Core implementation (~420 lines) |
| `test/basic.test.ts` | Test suite |
| `README.md` | Project overview |
| `package.json` | Dependencies (minimal!) |

---

## Blog Post Distillation: Push-Pull-Push Reactivity

From https://milomg.dev/2022-12-01/reactivity

### The Problem

Traditional reactive systems must choose between:
- **Push**: Fast but wastes work on unused branches
- **Pull**: Simple but re-traverses constantly
- **Hybrid**: Complex but best of both

### Push-Pull-Push Algorithm

```
Phase 1 (Push): Mark all descendants dirty
Phase 2 (Pull): When reading, update if dirty
Phase 3 (Push): Propagate results

Problem: Phase 1 is O(n) - marks everything!
```

### R3's Innovation

R3 replaces Phase 1 with **height-ordered bucket queue**:

```
Instead of: Mark everything dirty eagerly
Use: Insert into heap at computed's height

Result: Only affected nodes are processed
        Height ordering prevents glitches
        O(1) selector support possible
```

### Tri-Color Marking

Traditional approach uses three colors:
- **White**: Not yet visited
- **Gray**: Visited, processing children
- **Black**: Fully processed

R3's variant:
- **None**: Clean
- **Check**: May need update (lazy)
- **Dirty**: Must update (eager)

The Check/Dirty distinction enables short-circuit evaluation.

---

## Related Projects

R3 references these related implementations:

| Project | Description |
|---------|-------------|
| [alien-signals](https://github.com/stackblitz/alien-signals) | Similar height-based approach |
| [reactively](https://github.com/milomg/reactively) | Fine-grained reactivity library |
| [r2](https://github.com/milomg/r2) | Predecessor to r3 |
| [incremental](https://github.com/janestreet/incremental) | OCaml incremental computation |

---

## Glossary

| Term | Definition |
|------|------------|
| **Signal** | A reactive source of truth (can be set externally) |
| **Computed** | A derived reactive value (computed from other signals) |
| **Link** | Bidirectional edge in dependency graph |
| **Height** | Topological level in dependency DAG |
| **Dirty Heap** | Bucket queue organizing pending recomputations |
| **Firewall Signal** | Signal owned by a computed (component boundary) |
| **Check Flag** | "May need recomputation, verify first" |
| **Dirty Flag** | "Must recompute, inputs definitely changed" |
| **stabilize()** | Process all pending updates in order |
| **Glitch** | Inconsistent state where some nodes updated, others stale |

---

## Reading Order

For someone new to the codebase:

1. **Start**: `r3-deep-dive.md` - Get the 5-minute overview
2. **Then**: `r3-visual-guide.md` - See diagrams of key concepts
3. **Then**: `r3-function-dive.md` - Read alongside `src/index.ts`
4. **Finally**: `r3-test-walkthrough.md` - See it all in action

For understanding a specific concept:

- **Height ordering**: See visual-guide "Height Adjustment" section
- **Three-color marking**: See deep-dive "Three Pillars" section
- **Dynamic dependencies**: See test-walkthrough "Dynamic Sources" test
- **Firewall signals**: See visual-guide "Firewall Signals" diagram

---

## Key Takeaways

1. **Topological ordering prevents glitches** - By processing nodes in height order, we guarantee dependencies are always updated first.

2. **Lazy marking is more efficient** - Instead of eagerly marking all descendants, R3 uses the heap to track only affected nodes.

3. **Check vs Dirty enables short-circuits** - A computed marked Check can exit early if its inputs didn't really change.

4. **Dynamic dependencies are first-class** - Dependencies can change at runtime; unlinkSubs() cleans up old subscriptions.

5. **Bidirectional links enable O(1) operations** - Both dep→sub and sub→dep directions allow efficient insertion and removal.

6. **Tail pointers are critical** - Every linked list uses tail pointers for O(1) append instead of O(n) traversal.

7. **Self-referential markers simplify empty checks** - Using `node.prevHeap = node` as empty marker avoids extra flags.

---

## Unanswered Questions / Areas for Further Exploration

- How does R3 handle circular dependencies? (Likely runtime error from infinite recursion)
- What's the maximum practical height? (dirtyHeap is fixed at 2000, can be increased)
- How does this compare to SolidJS's approach? (Similar height-based ordering)
- Can this be made concurrent? (Would require significant redesign)

---

## Appendix: Code Size

The entire r3 implementation:
- **~420 lines** of TypeScript
- **Zero dependencies**
- **~1KB gzipped** (estimated)

Key functions by line count:
- `recompute()`: 42 lines
- `link()`: 47 lines
- `deleteFromHeap()`: 20 lines
- `insertIntoHeap()`: 17 lines
- `markNode()`: 18 lines

This is an extremely compact, well-optimized implementation.
