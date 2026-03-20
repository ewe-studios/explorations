---
location: /home/darkvoid/Boxxed/@formulas/src.UIFrameworks/src.reactivity/r3/src/index.ts
repository: N/A - not a git repository (local copy)
explored_at: 2026-03-17
language: TypeScript
parent: exploration.md
---

# R3 Source Code - Function-by-Function Deep Dive

This document connects each function in `/home/darkvoid/Boxxed/@formulas/src.UIFrameworks/src.reactivity/r3/src/index.ts` to the reactive concepts it implements.

---

## Type Definitions (Lines 1-44)

### `Disposable` (Line 1-3)

```typescript
export interface Disposable {
  (): void;
}
```

**Purpose**: Cleanup callback type for `onCleanup()`.

**Connection**: When a computed recomputes, it runs disposal callbacks to clean up side effects (like unsubscribing from external resources).

---

### `ReactiveFlags` enum (Lines 5-11)

```typescript
export const enum ReactiveFlags {
  None = 0,
  Check = 1 << 0,      // 1 - May need recomputation
  Dirty = 1 << 1,      // 2 - Must recompute
  RecomputingDeps = 1 << 2,  // 4 - Currently tracking deps
  InHeap = 1 << 3,     // 8 - Currently in dirtyHeap
}
```

**Purpose**: Bit flags for computed node state.

**Key insight**: These are bit flags (powers of 2) so they can be combined:
- `Check | Dirty = 3` (both bits set)
- Check: `(flags & ReactiveFlags.Check) !== 0`

**State machine**:
```
None ──► Check ──► Dirty ──► (recompute) ──► None
              ▲                              │
              └──────────────────────────────┘
```

---

### `Link` (Lines 13-19)

```typescript
export interface Link {
  dep: Signal<unknown> | Computed<unknown>;  // The dependency
  sub: Computed<unknown>;                     // The subscriber
  nextDep: Link | null;                       // Next in sub's dep list
  prevSub: Link | null;                       // Prev in dep's sub list
  nextSub: Link | null;                       // Next in dep's sub list
}
```

**Purpose**: Bidirectional edge in the dependency graph.

**Visual**:
```
    Signal/Computed "dep"
    ┌─────────────────────┐
    │ subs ───────────────┼───►
    │ subsTail ◄──────────┼───
    └─────────────────────┘
              ▲
              │
         ┌────┴────┐
         │  LINK   │
         │ dep: ▲  │
         │ sub: │  │
         │ nextDep: ──► next link in sub's dep list
         │ prevSub: ──► prev link in dep's sub list
         │ nextSub: ──► next link in dep's sub list
         └────┬────┘
              │
              │
    Computed "sub"
    ┌─────────────────────┐
    │ deps ───────────────┼───►
    │ depsTail ◄──────────┼───
    └─────────────────────┘
```

---

### `RawSignal<T>` (Lines 21-25)

```typescript
export interface RawSignal<T> {
  subs: Link | null;
  subsTail: Link | null;
  value: T;
}
```

**Purpose**: Base signal structure - a value with subscribers.

---

### `FirewallSignal<T>` (Lines 27-30)

```typescript
interface FirewallSignal<T> extends RawSignal<T> {
  owner: Computed<unknown>;
  nextChild: FirewallSignal<unknown> | null;
}
```

**Purpose**: Signal owned by a computed (component-scoped reactivity).

**Use case**: When a component's selector recomputes, all its owned signals' subscribers need invalidation.

---

### `Signal<T>` (Line 32)

```typescript
export type Signal<T> = RawSignal<T> | FirewallSignal<T>;
```

**Purpose**: Union type - can be plain signal or firewall signal.

---

### `Computed<T>` (Lines 34-44)

```typescript
export interface Computed<T> extends RawSignal<T> {
  deps: Link | null;              // My dependencies
  depsTail: Link | null;          // Tail for O(1) append
  flags: ReactiveFlags;           // State flags
  height: number;                 // Topological level
  nextHeap: Computed<unknown>;    // Next in dirtyHeap bucket
  prevHeap: Computed<unknown>;    // Prev in dirtyHeap bucket
  disposal: Disposable | Disposable[] | null;  // Cleanup callbacks
  fn: () => T;                    // The computation function
  child: FirewallSignal<unknown> | null;       // Owned firewall signals
}
```

**Purpose**: A computed value that tracks dependencies and propagates changes.

**Key fields explained**:
- `deps`/`depsTail`: doubly-linked list of what I depend on
- `subs`/`subsTail`: doubly-linked list of who depends on me
- `height`: topological level (parent height + 1)
- `nextHeap`/`prevHeap`: doubly-linked list within dirtyHeap bucket
- `disposal`: cleanup callbacks from `onCleanup()`
- `fn`: the function that computes my value
- `child`: head of linked list of firewall signals I own

---

## Global State (Lines 46-56)

### Module-level variables (Lines 46-51)

```typescript
let markedHeap = false;      // Has heap been marked this batch?
let context: Computed<unknown> | null = null;  // Current executing computed

let minDirty = 0;            // Current processing height
let maxDirty = 0;            // Highest dirty height
const dirtyHeap: (Computed<unknown> | undefined)[] = new Array(2000);  // Bucket queue
```

**Purpose**: Global scheduler state.

**context**: When a computed's `fn()` runs, it sets `context = self`. Any `read()` calls check `context` to know what to subscribe to.

**dirtyHeap**: Array where `dirtyHeap[h]` = head of linked list of dirty computeds at height `h`.

---

### `increaseHeapSize()` (Lines 52-56)

```typescript
export function increaseHeapSize(n: number) {
  if (n > dirtyHeap.length) {
    dirtyHeap.length = n;
  }
}
```

**Purpose**: Pre-allocate heap for deep dependency chains.

**Use case**: If you know you have computeds at height > 2000, call this first.

---

## Heap Management (Lines 58-97)

### `insertIntoHeap()` (Lines 58-75)

```typescript
function insertIntoHeap(n: Computed<unknown>) {
  const flags = n.flags;
  if (flags & ReactiveFlags.InHeap) return;  // Already in heap
  n.flags = flags | ReactiveFlags.InHeap;    // Mark as in-heap

  const height = n.height;
  const heapAtHeight = dirtyHeap[height];

  if (heapAtHeight === undefined) {
    dirtyHeap[height] = n;  // First node at this height
  } else {
    // Append to end of linked list at this height
    const tail = heapAtHeight.prevHeap;
    tail.nextHeap = n;
    n.prevHeap = tail;
    heapAtHeight.prevHeap = n;
  }

  if (height > maxDirty) {
    maxDirty = height;  // Update max height
  }
}
```

**Purpose**: Add a computed to the dirty queue at its height level.

**Algorithm**:
1. Check `InHeap` flag - don't duplicate
2. Set `InHeap` flag
3. Get the head of the bucket at this height
4. If bucket empty: this becomes the head
5. If bucket has nodes: append to end (O(1) with prevHeap pointer)
6. Update `maxDirty` if needed

**Time complexity**: O(1)

**Why tail insert**: Maintains stable ordering - nodes inserted earlier are processed first within the same height.

---

### `deleteFromHeap()` (Lines 77-97)

```typescript
function deleteFromHeap(n: Computed<unknown>) {
  const flags = n.flags;
  if (!(flags & ReactiveFlags.InHeap)) return;  // Not in heap
  n.flags = flags & ~ReactiveFlags.InHeap;      // Clear flag

  const height = n.height;
  if (n.prevHeap === n) {
    // Special case: self-referential (head marker)
    dirtyHeap[height] = undefined;
  } else {
    const next = n.nextHeap;
    const dhh = dirtyHeap[height]!;
    const end = next ?? dhh;

    if (n === dhh) {
      // Removing the head
      dirtyHeap[height] = next;
    } else {
      // Removing from middle/end
      n.prevHeap.nextHeap = next;
    }
    end.prevHeap = n.prevHeap;
  }

  // Reset to self-referential state
  n.prevHeap = n;
  n.nextHeap = undefined;
}
```

**Purpose**: Remove a computed from the dirty queue.

**When called**:
1. When processing a node from the heap (it's being removed from queue)
2. When a computed's value doesn't change (no need to propagate)

**Algorithm**:
1. Check `InHeap` flag - nothing to do if not set
2. Clear `InHeap` flag
3. If self-referential (`prevHeap === n`): bucket becomes empty
4. Otherwise: standard doubly-linked list removal
   - If removing head: update bucket head to `next`
   - Otherwise: bypass `n` in the list
   - Update `end.prevHeap` to point to node before `n`
5. Reset `n`'s pointers to initial state

**Time complexity**: O(1)

---

## Core API (Lines 99-152)

### `computed()` (Lines 99-129)

```typescript
export function computed<T>(fn: () => T): Computed<T> {
  const self: Computed<T> = {
    disposal: null,
    fn: fn,
    value: undefined as T,
    height: 0,
    child: null,
    nextHeap: undefined,
    prevHeap: null as any,
    deps: null,
    depsTail: null,
    subs: null,
    subsTail: null,
    flags: ReactiveFlags.None,
  };
  self.prevHeap = self;  // Self-referential = head marker

  if (context) {
    // Created inside another computed
    if (context.depsTail === null) {
      // Parent is still initializing deps
      self.height = context.height;
      recompute(self, false);
    } else {
      // Parent has deps, so we're one level deeper
      self.height = context.height + 1;
      insertIntoHeap(self);
    }
    link(self, context);  // Subscribe to parent
  } else {
    // Created at top level
    recompute(self, false);
  }

  return self;
}
```

**Purpose**: Create a computed value that tracks its dependencies.

**Execution flow**:

```
1. Create the Computed object with default state
2. Set prevHeap = self (empty list marker)
3. Check if created inside another computed (context !== null)

   YES (nested):
   ──────────────────────────────────
   a. Check if parent (context) is still tracking deps
      - depsTail === null means parent hasn't read anything yet
   b. If yes: same height as parent, compute immediately
   c. If no: height = parent.height + 1, insert in heap
   d. Link self to context (becomes child of context)

   NO (top-level):
   ──────────────────────────────────
   Compute immediately with no dependencies
```

**Why height adjustment**: If created mid-execution of parent, the computed hasn't had a chance to track its own deps yet, so it inherits parent's height.

**Example**:
```javascript
const a = computed(() => {
  const b = computed(() => read(signal(1)));  // b created mid-execution
  return read(b);
});
```

Here, `b` is created while `a` is running, so `b.height = a.height` initially.

---

### `signal()` (Lines 131-152)

```typescript
// Overload 1: Signal with firewall (owner)
export function signal<T>(v: T, firewall: Computed<unknown>): FirewallSignal<T>;

// Overload 2: Plain signal
export function signal<T>(v: T): Signal<T>;

// Implementation
export function signal<T>(
  v: T,
  firewall: Computed<unknown> | null = null,
): Signal<T> {
  if (firewall !== null) {
    // Create firewall signal - owned by computed
    return (firewall.child = {
      value: v,
      subs: null,
      subsTail: null,
      owner: firewall,
      nextChild: firewall.child,  // Prepend to firewall list
    });
  } else {
    // Create plain signal
    return {
      value: v,
      subs: null,
      subsTail: null,
    };
  }
}
```

**Purpose**: Create a reactive signal (source of truth).

**Firewall signals**: When `firewall` is provided:
1. Creates a `FirewallSignal` with `owner` reference
2. Prepends to `firewall.child` list (owner's firewall signals)
3. Updates `firewall.child` to point to new signal

**Why prepend**: When owner recomputes, it can iterate all children via the `child` pointer.

**Example**:
```javascript
const selector = computed(() => {
  // ... selector logic
});

// These signals are "owned" by selector
const active = signal(true, selector);
const selected = signal('item1', selector);

// selector.child → active → selected → null
```

---

## Recomputation Engine (Lines 154-216)

### `recompute()` (Lines 154-196)

```typescript
function recompute(el: Computed<unknown>, del: boolean) {
  // Remove from heap or mark as no longer needing heap slot
  if (del) {
    deleteFromHeap(el);
  } else {
    el.nextHeap = undefined;
    el.prevHeap = el;
  }

  runDisposal(el);           // Run cleanup callbacks
  const oldcontext = context;
  context = el;              // Set self as current context
  el.depsTail = null;        // Reset deps tracking
  el.flags = ReactiveFlags.RecomputingDeps;

  const value = el.fn();     // EXECUTE THE COMPUTED FUNCTION

  el.flags = ReactiveFlags.None;
  context = oldcontext;      // Restore previous context

  // Clean up old dependencies
  const depsTail = el.depsTail as Link | null;
  let toRemove = depsTail !== null ? depsTail.nextDep : el.deps;
  if (toRemove !== null) {
    do {
      toRemove = unlinkSubs(toRemove);
    } while (toRemove !== null);
    if (depsTail !== null) {
      depsTail.nextDep = null;
    } else {
      el.deps = null;
    }
  }

  // Propagate if value changed
  if (value !== el.value) {
    el.value = value;

    for (let s = el.subs; s !== null; s = s.nextSub) {
      const o = s.sub;
      const flags = o.flags;
      if (flags & ReactiveFlags.Check) {
        o.flags = flags | ReactiveFlags.Dirty;  // Upgrade Check to Dirty
      }
      insertIntoHeap(o);  // Schedule for recompute
    }
  }
}
```

**Purpose**: Re-execute a computed and propagate changes.

**Step-by-step**:

```
┌─────────────────────────────────────────────────────────────┐
│ PHASE 1: PREPARATION                                        │
├─────────────────────────────────────────────────────────────┤
│ 1. Remove from heap (or reset heap pointers)               │
│ 2. Run disposal callbacks (cleanup from previous run)      │
│ 3. Save old context, set self as context                   │
│ 4. Reset depsTail (will track new deps as fn runs)         │
│ 5. Set RecomputingDeps flag                                │
└─────────────────────────────────────────────────────────────┘
                            │
                            ▼
┌─────────────────────────────────────────────────────────────┐
│ PHASE 2: EXECUTION                                          │
├─────────────────────────────────────────────────────────────┤
│ 6. Call el.fn()                                            │
│                                                            │
│    During fn() execution:                                  │
│    - read() calls will link() deps to el                   │
│    - el.depsTail will track the last dep added             │
└─────────────────────────────────────────────────────────────┘
                            │
                            ▼
┌─────────────────────────────────────────────────────────────┐
│ PHASE 3: CLEANUP                                            │
├─────────────────────────────────────────────────────────────┤
│ 7. Clear RecomputingDeps flag                              │
│ 8. Restore old context                                     │
│ 9. Remove stale dependencies (deps not re-subscribed)      │
│    - Any deps after depsTail were not re-read              │
│    - unlinkSubs() removes el from those deps' sub lists    │
└─────────────────────────────────────────────────────────────┘
                            │
                            ▼
┌─────────────────────────────────────────────────────────────┐
│ PHASE 4: PROPAGATION                                        │
├─────────────────────────────────────────────────────────────┤
│ 10. If value changed:                                       │
│     - Update el.value                                      │
│     - For each subscriber:                                 │
│       - Upgrade Check → Dirty                              │
│       - Insert into heap                                   │
└─────────────────────────────────────────────────────────────┘
```

**Key insight**: Dependency cleanup handles dynamic dependencies. If a computed stops reading a signal, that link is removed.

---

### `updateIfNecessary()` (Lines 198-216)

```typescript
function updateIfNecessary(el: Computed<unknown>): void {
  // If marked Check, verify by checking deps
  if (el.flags & ReactiveFlags.Check) {
    for (let d = el.deps; d; d = d.nextDep) {
      const dep = d.dep;
      if ("fn" in dep) {
        updateIfNecessary(dep);  // Recursive pull
      }
      // Early exit if we became Dirty
      if (el.flags & ReactiveFlags.Dirty) {
        break;
      }
    }
  }

  // If Dirty, recompute
  if (el.flags & ReactiveFlags.Dirty) {
    recompute(el, true);  // true = remove from heap
  }

  el.flags = ReactiveFlags.None;
}
```

**Purpose**: Pull-based update - check if computed needs refresh.

**When called**:
- `read()` calls this if computed might be stale
- During stabilization for height ordering

**Algorithm**:

```
1. If flagged Check:
   ──────────────────────────────────
   a. Iterate through all dependencies
   b. For computed deps: recursively updateIfNecessary
   c. This "pulls" updates from the top down
   d. If any dep marks us Dirty, short-circuit

2. If flagged Dirty:
   ──────────────────────────────────
   a. Recompute (with del=true to remove from heap)

3. Clear all flags
```

**Why recursive**: Pull-based systems traverse up the dependency tree to ensure ancestors are updated first.

**Short-circuit optimization**: Once we know we're Dirty, no need to check remaining deps.

---

## Dependency Unlinking (Lines 218-248)

### `unlinkSubs()` (Lines 218-238)

```typescript
function unlinkSubs(link: Link): Link | null {
  const dep = link.dep;
  const nextDep = link.nextDep;
  const nextSub = link.nextSub;
  const prevSub = link.prevSub;

  // Remove from dep's sub list
  if (nextSub !== null) {
    nextSub.prevSub = prevSub;
  } else {
    dep.subsTail = prevSub;  // Was tail, update tail
  }
  if (prevSub !== null) {
    prevSub.nextSub = nextSub;
  } else {
    dep.subs = nextSub;  // Was head, update head

    // If computed has no more subscribers, unwatch it
    if (nextSub === null && "fn" in dep) {
      unwatched(dep);
    }
  }

  return nextDep;  // Return next dep to process
}
```

**Purpose**: Remove a subscription link - called when cleaning up old dependencies.

**What it does**:
1. Extract all pointers from link
2. Remove link from `dep.subs` doubly-linked list
3. Update `subsTail` if link was tail
4. Update `subs` if link was head
5. If `dep` is a computed with no more subscribers: call `unwatched()`
6. Return `nextDep` for iteration

**Time complexity**: O(1)

---

### `unwatched()` (Lines 240-248)

```typescript
function unwatched(el: Computed<unknown>) {
  deleteFromHeap(el);    // Remove from dirty queue
  let dep = el.deps;
  while (dep !== null) {
    dep = unlinkSubs(dep);  // Unsubscribe from all deps
  }
  el.deps = null;
  runDisposal(el);         // Run cleanup callbacks
}
```

**Purpose**: Handle a computed that has no subscribers (garbage collection).

**When called**: When the last subscriber unsubscribes from a computed.

**What it does**:
1. Remove from heap
2. Iterate through all deps, unlinking each
3. Clear deps list
4. Run disposal (cleanup side effects)

**Why important**: Prevents memory leaks and wasted computation for unused values.

---

## Dependency Linking (Lines 250-315)

### `link()` (Lines 250-297)

```typescript
function link(
  dep: Signal<unknown> | Computed<unknown>,
  sub: Computed<unknown>,
) {
  const prevDep = sub.depsTail;

  // Fast path: same dep as last time
  if (prevDep !== null && prevDep.dep === dep) {
    return;
  }

  let nextDep: Link | null = null;
  const isRecomputing = sub.flags & ReactiveFlags.RecomputingDeps;

  if (isRecomputing) {
    // Check if we already have this dep next in line
    nextDep = prevDep !== null ? prevDep.nextDep : sub.deps;
    if (nextDep !== null && nextDep.dep === dep) {
      sub.depsTail = nextDep;  // Advance tail
      return;
    }
  }

  // Check if already subscribed from dep's side
  const prevSub = dep.subsTail;
  if (
    prevSub !== null &&
    prevSub.sub === sub &&
    (!isRecomputing || isValidLink(prevSub, sub))
  ) {
    return;
  }

  // Create new link
  const newLink =
    (sub.depsTail =
    dep.subsTail =
      {
        dep,
        sub,
        nextDep,
        prevSub,
        nextSub: null,
      });

  // Wire into dep list
  if (prevDep !== null) {
    prevDep.nextDep = newLink;
  } else {
    sub.deps = newLink;
  }

  // Wire into sub list
  if (prevSub !== null) {
    prevSub.nextSub = newLink;
  } else {
    dep.subs = newLink;
  }
}
```

**Purpose**: Subscribe `sub` to `dep` - called by `read()`.

**Fast paths** (early returns):

```
1. Same dep as last read:
   ──────────────────────────────────
   if (prevDep.dep === dep) return;

   Common case: reading same signal twice in a row

2. Dep already exists next in line:
   ──────────────────────────────────
   if (nextDep.dep === dep) {
     sub.depsTail = nextDep;
     return;
   }

   Common case: same dependency order as last execution

3. Already subscribed:
   ──────────────────────────────────
   if (prevSub.sub === sub && isValidLink(prevSub, sub)) return;

   Common case: another path already created this link
```

**Full path** (create new link):

```
1. Create Link object with:
   - dep, sub references
   - nextDep (if recomputing, for later cleanup)
   - prevSub (for doubly-linked list)
   - nextSub: null (will be tail)

2. Update sub.depsTail = newLink
3. Update dep.subsTail = newLink

4. Wire into lists:
   - If had prevDep: prevDep.nextDep = newLink
   - Else: sub.deps = newLink (new head)
   - If had prevSub: prevSub.nextSub = newLink
   - Else: dep.subs = newLink (new head)
```

---

### `isValidLink()` (Lines 300-315)

```typescript
function isValidLink(checkLink: Link, sub: Computed<unknown>): boolean {
  const depsTail = sub.depsTail;
  if (depsTail !== null) {
    let link = sub.deps!;
    do {
      if (link === checkLink) {
        return true;
      }
      if (link === depsTail) {
        break;
      }
      link = link.nextDep!;
    } while (link !== null);
  }
  return false;
}
```

**Purpose**: Verify a link is still in the dep list (not yet cleaned up).

**When called**: From `link()` fast path #3 to avoid duplicate subscriptions.

**Algorithm**: Iterate from `sub.deps` to `depsTail`, looking for `checkLink`.

**Time complexity**: O(n) where n = number of deps, but only called in specific cases.

---

## Reading and Writing (Lines 317-346)

### `read()` (Lines 317-337)

```typescript
export function read<T>(el: Signal<T> | Computed<T>): T {
  if (context) {
    // We're inside a computed - track dependency
    link(el, context);

    const owner = "owner" in el ? el.owner : el;
    if ("fn" in owner) {
      // Dependency is a computed - ensure it's fresh
      const height = owner.height;

      // Height adjustment: maintain topological order
      if (height >= context.height) {
        context.height = height + 1;
      }

      // If dep is at or above minDirty, or is dirty/check, update it
      if (
        height >= minDirty ||
        owner.flags & (ReactiveFlags.Dirty | ReactiveFlags.Check)
      ) {
        markHeap();
        updateIfNecessary(owner);
      }
    }
  }
  return el.value;
}
```

**Purpose**: Read a signal/computed value, tracking dependencies.

**Two modes**:

```
OUTSIDE computed (context === null):
──────────────────────────────────────
Just return el.value
No dependency tracking needed


INSIDE computed (context !== null):
──────────────────────────────────────
1. link(el, context) - Subscribe context to el

2. If el is a FirewallSignal, get owner
   - owner = the computed that owns this signal

3. If owner is a computed:
   a. Height adjustment:
      - If owner.height >= context.height:
        context.height = owner.height + 1
      - This maintains topological ordering

   b. Pull-based update:
      - If owner might be stale (height >= minDirty or has flags)
      - Call markHeap() then updateIfNecessary(owner)

4. Return el.value
```

**Height adjustment example**:
```javascript
const a = computed(() => {
  // a.height starts at 0
  const b = computed(() => read(s));  // b.height = 0 initially
  return read(b);  // read() sees b.height (0) >= a.height (0)
                   // So a.height becomes 1
});
```

---

### `setSignal()` (Lines 339-346)

```typescript
export function setSignal(el: Signal<unknown>, v: unknown) {
  if (el.value === v) return;  // Early exit - no change
  el.value = v;

  // Mark all direct subscribers
  for (let link = el.subs; link !== null; link = link.nextSub) {
    markedHeap = false;
    insertIntoHeap(link.sub);
  }
}
```

**Purpose**: Update a signal's value and propagate.

**Algorithm**:
1. Early exit if value unchanged (important optimization!)
2. Update value
3. For each subscriber:
   - Reset `markedHeap` flag
   - Insert subscriber into heap

**Note**: Only marks direct subscribers, not recursive descendants. Height ordering handles the rest.

---

## Marking System (Lines 348-376)

### `markNode()` (Lines 348-366)

```typescript
function markNode(el: Computed<unknown>, newState = ReactiveFlags.Dirty) {
  const flags = el.flags;
  // Don't downgrade: Check >= Dirty in bit values
  if ((flags & (ReactiveFlags.Check | ReactiveFlags.Dirty)) >= newState) return;
  el.flags = flags | newState;

  // Mark all children as Check (not Dirty!)
  for (let link = el.subs; link !== null; link = link.nextSub) {
    markNode(link.sub, ReactiveFlags.Check);
  }

  // Also mark through firewall signals
  if (el.child !== null) {
    for (
      let child: FirewallSignal<unknown> | null = el.child;
      child !== null;
      child = child.nextChild
    ) {
      for (let link = child.subs; link !== null; link = link.nextSub) {
        markNode(link.sub, ReactiveFlags.Check);
      }
    }
  }
}
```

**Purpose**: Mark a computed and its descendants as needing update.

**Key insight**: Uses `Check` for descendants, not `Dirty`. This enables short-circuit evaluation.

**Algorithm**:
1. If already at same or higher state, skip (idempotent)
2. Add `newState` to flags
3. Mark all direct subscribers as `Check`
4. If has firewall children, mark their subscribers as `Check` too

**Why Check not Dirty**:
```
Dirty = "I know my inputs changed, I must recompute"
Check = "Something changed upstream, verify if it affects me"

This allows computed functions to exit early:
  const c = computed(() => {
    if (!read(a)) return;  // a changed but result may not
    // Only read b, c if needed
  });
```

---

### `markHeap()` (Lines 368-376)

```typescript
function markHeap() {
  if (markedHeap) return;  // Already marked this batch
  markedHeap = true;

  for (let i = 0; i <= maxDirty; i++) {
    for (let el = dirtyHeap[i]; el !== undefined; el = el.nextHeap) {
      markNode(el);
    }
  }
}
```

**Purpose**: Mark all computeds currently in the heap.

**When called**: From `read()` when pulling updates for a computed.

**Why single-run flag**: `markedHeap` ensures we don't repeatedly mark the same batch.

---

## Stabilization (Lines 378-388)

### `stabilize()` (Lines 378-388)

```typescript
export function stabilize() {
  // Process from lowest height to highest
  for (minDirty = 0; minDirty <= maxDirty; minDirty++) {
    let el = dirtyHeap[minDirty];
    dirtyHeap[minDirty] = undefined;  // Clear bucket

    while (el !== undefined) {
      const next = el.nextHeap;
      recompute(el, false);  // false = don't call deleteFromHeap (already cleared)
      el = next;
    }
  }
}
```

**Purpose**: Process all pending updates in topological order.

**Algorithm**:
```
For each height level (0 to maxDirty):
  1. Get the bucket head
  2. Clear the bucket (we're processing it)
  3. For each node in the bucket's linked list:
     a. Save next pointer
     b. Recompute the node
     c. Move to next
```

**Why topological order**: Ensures we never process a computed before its dependencies are updated. This prevents glitches.

**Example trace**:
```
Before stabilize():
  dirtyHeap[0]: undefined
  dirtyHeap[1]: a → b
  dirtyHeap[2]: c
  minDirty=0, maxDirty=2

Iteration minDirty=0:
  - Empty, skip

Iteration minDirty=1:
  - el = a
  - recompute(a): a.value changes, marks c as Check
  - el = b
  - recompute(b): b.value changes, marks c as Dirty
  - el = undefined (end of list)

Iteration minDirty=2:
  - el = c (may have been added during iteration 1)
  - recompute(c): reads updated a and b
  - el = undefined

After stabilize():
  dirtyHeap: all undefined
  minDirty=3, maxDirty=2 (loop exits)
```

---

## Cleanup System (Lines 390-418)

### `onCleanup()` (Lines 390-403)

```typescript
export function onCleanup(fn: Disposable): Disposable {
  if (!context) return fn;  // Not in computed, just return fn

  const node = context;

  if (!node.disposal) {
    node.disposal = fn;  // First cleanup
  } else if (Array.isArray(node.disposal)) {
    node.disposal.push(fn);  // Append to array
  } else {
    node.disposal = [node.disposal, fn];  // Convert to array
  }
  return fn;
}
```

**Purpose**: Register a cleanup callback for when computed recomputes or is destroyed.

**Storage strategy**:
- No disposal: store single function
- One disposal: convert to array
- Multiple: push to array

**Use case**:
```javascript
const timer = computed(() => {
  const id = setInterval(() => {
    // do something
  }, 1000);

  onCleanup(() => {
    clearInterval(id);  // Clean up on recompute
  });

  return something;
});
```

---

### `runDisposal()` (Lines 405-418)

```typescript
function runDisposal(node: Computed<unknown>): void {
  if (!node.disposal) return;

  if (Array.isArray(node.disposal)) {
    for (let i = 0; i < node.disposal.length; i++) {
      const callable = node.disposal[i];
      callable.call(callable);
    }
  } else {
    node.disposal.call(node.disposal);
  }

  node.disposal = null;  // Clear after running
}
```

**Purpose**: Execute all cleanup callbacks for a computed.

**When called**:
1. Before `recompute()` (cleanup from previous run)
2. In `unwatched()` (computed is being garbage collected)

---

## Debugging (Lines 420-422)

### `getContext()` (Lines 420-422)

```typescript
export function getContext(): Computed<unknown> | null {
  return context;
}
```

**Purpose**: Get the currently executing computed (for debugging/introspection).

**Use case**: Check if code is running inside a reactive context.

---

## Summary: Execution Flow

```
┌─────────────────────────────────────────────────────────────────┐
│                    COMPLETE EXECUTION FLOW                       │
├─────────────────────────────────────────────────────────────────┤
│                                                                  │
│  USER CODE: const c = computed(() => read(a) + read(b))         │
│                                                                  │
│  1. computed() creates Computed object                          │
│  2. Sets context = c                                            │
│  3. Calls c.fn()                                                │
│                                                                  │
│  INSIDE fn():                                                   │
│  ├─► read(a) called                                            │
│  │   └─► link(a, c) - subscribe c to a                         │
│  │   └─► return a.value                                        │
│  │                                                              │
│  └─► read(b) called                                            │
│      └─► link(b, c) - subscribe c to b                         │
│      └─► return b.value                                        │
│                                                                  │
│  4. c.value = result                                            │
│  5. Clear context                                               │
│  6. c now depends on [a, b]                                     │
│                                                                  │
├─────────────────────────────────────────────────────────────────┤
│                                                                  │
│  USER CODE: setSignal(a, newValue)                              │
│                                                                  │
│  1. a.value = newValue                                          │
│  2. For each sub (c):                                           │
│     └─► insertIntoHeap(c)                                       │
│                                                                  │
│  USER CODE: stabilize()                                         │
│                                                                  │
│  1. minDirty = 0, maxDirty = c.height                           │
│  2. For each height:                                            │
│     └─► For each node at height:                                │
│         └─► recompute(node)                                     │
│             ├─► Run disposal                                    │
│             ├─► Set context = node                              │
│             ├─► Call node.fn() → reads deps, tracks them        │
│             ├─► Cleanup stale deps                              │
│             └─► If value changed, propagate to children         │
│                                                                  │
└─────────────────────────────────────────────────────────────────┘
```

---

## Key Design Patterns

### 1. Tail-Pointer Optimization
All linked lists use tail pointers for O(1) append:
- `subs`/`subsTail` in Signal
- `deps`/`depsTail` in Computed
- `dirtyHeap[h]` uses `prevHeap` as tail pointer

### 2. Doubly-Linked Lists
Enable O(1) removal without iteration:
- `prevSub`/`nextSub` in Link
- `prevHeap`/`nextHeap` in Computed

### 3. Self-Referential Head Marker
Empty lists use `node.prevHeap = node` as marker:
- Simplifies empty check
- Avoids separate `isEmpty` flag

### 4. Bit Flags
Compact state storage with bitwise operations:
- `flags & Check` to test
- `flags | Dirty` to set
- `flags & ~InHeap` to clear

### 5. Global Context Pattern
Single `context` variable for implicit dependency tracking:
- Avoids passing context through every call
- Works because execution is synchronous

### 6. Bucket Queue
Height-ordered scheduling without sorting:
- `dirtyHeap[h]` = bucket for height h
- Iterate 0 to maxDirty for topological order
