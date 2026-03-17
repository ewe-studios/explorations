# R3 Reactive System - Deep Dive

## 5-Minute Distillation: The Core Ideas

### The Problem R3 Solves

Traditional reactive systems face a fundamental tradeoff:

| Approach | Pros | Cons |
|----------|------|------|
| **Push** | Fast propagation | Wastes work on unused branches, needs batching |
| **Pull** | Simple, no batching needed | Re-traverses graph constantly |
| **Push-Pull Hybrid** | Best of both | Complex implementation |

R3 implements a **hybrid push-pull system with topological ordering** to achieve:
1. **No wasted computations** - only runs what's actually needed
2. **No glitches** - consistent state through height ordering
3. **Dynamic dependency tracking** - dependencies can change at runtime
4. **O(1) selector support** - can mark only subsets of children

---

## The Three Pillars of R3

### 1. Height-Based Topological Ordering

Every computed node has a `height` value:
- **Signals at the root**: height = 0
- **Computed nodes**: height = max(dep heights) + 1

This creates a "layered cake" structure where changes always flow **from low to high**.

```
     height 0:  signal(a), signal(b)
        │
     height 1:  computed(c) ← depends on a, b
        │
     height 2:  computed(d) ← depends on c
```

**Why this matters**: When processing changes, you can iterate from height 0 upward and guarantee you never process a node before its dependencies.

---

### 2. The Dirty Heap (Bucket Queue)

Instead of a simple queue, R3 uses a **bucket queue** organized by height:

```javascript
dirtyHeap[height] = [computed nodes at that height]
```

When a signal changes:
1. Mark affected computeds as `Dirty`
2. Insert them into `dirtyHeap` at their height
3. `stabilize()` processes height 0 → max, ensuring correct order

This is the key to **glitch-free** updates.

---

### 3. Three-Color Marking (Check/Dirty/None)

R3 uses a variant of tri-color marking:

| Flag | Meaning |
|------|---------|
| `None` | Clean, up to date |
| `Check` | May need recomputation, check deps first |
| `Dirty` | Definitely needs recomputation |

**The propagation algorithm**:
```
When a computed's value changes:
  1. Mark itself Dirty
  2. Mark all children Check (not Dirty!)
```

**Why Check vs Dirty distinction matters**:
- `Dirty` = "I know my inputs changed, I must recompute"
- `Check` = "Something upstream changed, but I need to verify it affects me"

This enables **short-circuit evaluation**:
```javascript
const c = computed(() => {
  if (!read(a)) return "early exit";  // a changed but result same
  return read(b) + read(c);           // only check b, c if needed
});
```

---

## Core Data Structures

### Signal<T> - The Source

```typescript
interface Signal<T> {
  value: T;           // Current value
  subs: Link | null;  // Head of subscription list
  subsTail: Link | null; // Tail for O(1) append
}
```

**Firewall Signals** (for ownership):
```typescript
interface FirewallSignal<T> extends Signal<T> {
  owner: Computed;     // Parent computed that owns this signal
  nextChild: FirewallSignal | null; // Sibling signals
}
```

---

### Computed<T> - The Worker

```typescript
interface Computed<T> {
  // Inherited from Signal
  value: T;
  subs: Link | null;
  subsTail: Link | null;

  // Dependency tracking
  deps: Link | null;      // What I depend on
  depsTail: Link | null;  // For O(1) append

  // Scheduling
  flags: ReactiveFlags;   // Check/Dirty/InHeap
  height: number;         // Topological level
  nextHeap: Computed;     // Linked list in dirtyHeap
  prevHeap: Computed;

  // Execution
  fn: () => T;            // The computation
  disposal: Disposable;   // Cleanup callbacks

  // Firewall children
  child: FirewallSignal | null;
}
```

---

### Link - The Glue

```typescript
interface Link {
  dep: Signal | Computed;  // The dependency
  sub: Computed;           // The subscriber
  nextDep: Link | null;    // Next dep in subscriber's list
  prevSub: Link | null;    // Prev sub in dependency's list
  nextSub: Link | null;    // Next sub in dependency's list
}
```

**Visual representation**:
```
    Signal A                    Computed B
    ┌─────────┐                 ┌─────────┐
    │ subs ───┼───────────────► │ deps    │
    │ subsTail◄────────────────┼─ depsTail│
    └─────────┘                 └─────────┘
         ▲                           │
         │         Link              │
         └────── {dep: A, sub: B} ◄──┘
```

---

## The Algorithm Step-by-Step

### Phase 1: Reading a Signal (`read()`)

```typescript
function read<T>(el: Signal<T> | Computed<T>): T {
  if (context) {           // If inside a computed
    link(el, context);     // 1. Track dependency

    // 2. If dep is a computed, ensure it's up-to-date
    if ("fn" in owner) {
      if (height >= context.height)
        context.height = height + 1;  // Adjust height if needed

      if (needsUpdate)
        updateIfNecessary(owner);     // Pull-based update
    }
  }
  return el.value;
}
```

**Key insight**: Reading creates the edge in the dependency graph.

---

### Phase 2: Linking Dependencies (`link()`)

```typescript
function link(dep, sub) {
  // Fast path: already subscribed
  if (prevDep.dep === dep) return;

  // Create bidirectional link
  const newLink = { dep, sub, nextDep, prevSub, nextSub: null };

  // Add to sub's dep list (tail insert)
  sub.depsTail = newLink;

  // Add to dep's sub list (tail insert)
  dep.subsTail = newLink;
}
```

**Bidirectional** so:
- Dep → Sub: "notify children when I change"
- Sub → Dep: "check if I need to update"

---

### Phase 3: Setting a Signal (`setSignal()`)

```typescript
function setSignal(el: Signal, v) {
  if (el.value === v) return;  // Early exit!
  el.value = v;

  // Mark all direct subscribers
  for (let link = el.subs; link; link = link.nextSub) {
    insertIntoHeap(link.sub);  // Adds to dirtyHeap
  }
}
```

---

### Phase 4: Stabilization (`stabilize()`)

```typescript
function stabilize() {
  // Process from lowest height to highest
  for (minDirty = 0; minDirty <= maxDirty; minDirty++) {
    let el = dirtyHeap[minDirty];
    dirtyHeap[minDirty] = undefined;

    while (el !== undefined) {
      const next = el.nextHeap;
      recompute(el, false);
      el = next;
    }
  }
}
```

**This is the "push" phase** - it processes the bucket queue in order.

---

### Phase 5: Recomputation (`recompute()`)

```typescript
function recompute(el: Computed, del: boolean) {
  runDisposal(el);        // Run cleanup callbacks
  context = el;           // Set global context
  el.flags = RecomputingDeps;

  const value = el.fn();  // 1. Re-run the function

  el.flags = None;
  context = oldContext;

  // 2. Remove old dependencies that weren't re-subscribed
  cleanupUnusedDeps(el);

  // 3. If value changed, propagate to children
  if (value !== el.value) {
    el.value = value;
    for (let s = el.subs; s; s = s.nextSub) {
      markNode(s.sub, Dirty);  // Mark children
      insertIntoHeap(s.sub);
    }
  }
}
```

---

## Edge Cases & How R3 Handles Them

### 1. Dynamic Dependencies

```javascript
const cond = signal(true);
const a = signal(1);
const b = signal(2);

const c = computed(() => {
  return read(cond) ? read(a) : read(b);
});
```

**What happens**:
1. First run: c depends on `cond` and `a`
2. If `cond` becomes false: c now depends on `cond` and `b`
3. Old dependency `a` is unlinked via `unlinkSubs()`

**How**: During recompute, R3 tracks which deps were actually read. Any dep not re-subscribed is removed.

---

### 2. Diamond Dependencies

```
     s
    / \
   a   b
    \ /
     c
```

Without height ordering, `c` might run twice. With height ordering:
- `s` at height 0
- `a`, `b` at height 1
- `c` at height 2

`stabilize()` processes height 1 before height 2, so `c` sees consistent state.

---

### 3. Disappearing Dependencies

```javascript
let done = false;
const c = computed(() => {
  if (done) return 0;
  const v = read(s);
  if (v > 2) done = true;
  return v;
});
```

**What happens**:
1. `s` changes → `c` recomputes → `done` becomes true
2. Next `s` change: `c` doesn't read `s` anymore (early return)
3. `unlinkSubs()` removes `c` from `s`'s subscription list
4. `c` will never run again

---

### 4. Untracked Inner Computeds

```javascript
computed(function outer() {
  read(a);
  computed(function inner() {
    read(b);
  });
});
```

**Problem**: If `inner` is created conditionally, it shouldn't run when `outer`'s other deps change.

**Solution**: `updateIfNecessary()` short-circuits:
```typescript
function updateIfNecessary(el: Computed) {
  // First check if any dep is Dirty
  for (let d = el.deps; d; d = d.nextDep) {
    if ("fn" in d.dep) updateIfNecessary(d.dep);
    if (el.flags & Dirty) break;  // Early exit!
  }

  if (el.flags & Dirty) recompute(el);
}
```

---

## The Push-Pull-Push Algorithm Explained

### Traditional Push-Pull-Push

1. **Push**: Mark all descendants dirty (eager)
2. **Pull**: When reading, update if dirty
3. **Push**: Propagate changes

**Problem**: Step 1 marks too much - O(n) marking even for selective updates.

---

### R3's Approach: Height-Ordered Push with Lazy Pull

1. **Push**: Insert into height-ordered bucket queue
2. **Pull**: `updateIfNecessary()` traverses deps lazily
3. **Push**: Process bucket queue in order

**Key difference**: R3 doesn't eagerly mark all descendants. It uses the heap for ordering and only marks what's actually in the propagation path.

---

## Firewall Signals (Component Ownership)

Firewall signals allow **component-scoped reactivity**:

```typescript
const selector = computed(() => {
  const prev = read(selected);
  // ... update ownership
});

// These signals are "owned" by selector
const a = signal(true, selector);  // FirewallSignal
const b = signal(false, selector);
```

**Structure**:
```
Computed (selector)
    │
    └─ child → FirewallSignal A
                ├─ owner: selector
                └─ nextChild → FirewallSignal B
```

**When selector recomputes**:
1. All its firewall children's subscribers are marked `Check`
2. This propagates invalidation through component boundaries

---

## Performance Characteristics

| Operation | Complexity |
|-----------|------------|
| `read()` | O(1) + dependency link |
| `setSignal()` | O(children) to mark |
| `stabilize()` | O(n) where n = affected nodes |
| `link()` | O(1) with tail pointers |
| `unlink()` | O(1) per link |

**Key optimizations**:
- Tail pointers for O(1) append
- Height-based bucket queue (no sorting needed)
- Early exit on unchanged values
- Lazy dependency cleanup

---

## Comparison with Other Approaches

| System | Ordering | Marking | Dynamic Deps |
|--------|----------|---------|--------------|
| R3 | Height buckets | Lazy Check/Dirty | Yes |
| Alien Signals | Height buckets | Tri-color | Yes |
| SolidJS | Queue + generations | Sync/Dirty | Yes |
| Vue Reactivity | Queue | Dirty flag | Yes |
| MobX | DFS traversal | Observable | Yes |

---

## Summary: The Mental Model

Think of R3 as a **waterfall with buckets**:

1. **Signals** are the source of water at the top
2. **Computed nodes** are buckets at different heights
3. When a signal changes, it **opens valves** (marks Dirty)
4. Water flows **one height at a time** (stabilize loop)
5. Each bucket only fills if water **actually reaches it** (value change check)
6. Buckets can **change which pipes feed them** (dynamic deps)

The beauty is in the **lazy evaluation**:
- Don't mark everything dirty upfront
- Only process what's actually affected
- Use height ordering to prevent glitches

---

## Code Reading Guide

When reading r3 code, follow this flow:

1. **Entry points**: `signal()`, `computed()`, `read()`, `setSignal()`
2. **Core loop**: `stabilize()` → `recompute()`
3. **Dependency management**: `link()`, `unlinkSubs()`
4. **Scheduling**: `insertIntoHeap()`, `deleteFromHeap()`
5. **Marking**: `markNode()`, `markHeap()`

Key invariants to verify:
- Heights always increase along dependency edges
- Heap processing is always low-to-high
- Links are bidirectional and consistent
