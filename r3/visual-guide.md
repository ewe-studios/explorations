# R3 Reactive System - Visual Guide

## System Architecture Overview

```
┌─────────────────────────────────────────────────────────────────┐
│                        R3 REACTIVE SYSTEM                        │
├─────────────────────────────────────────────────────────────────┤
│                                                                  │
│  ┌──────────────┐         ┌──────────────┐                      │
│  │   SIGNALS    │         │   COMPUTED   │                      │
│  │  (Sources)   │         │  (Workers)   │                      │
│  │              │         │              │                      │
│  │  value: T    │         │  fn: () => T │                      │
│  │  subs: Link  │◄───────►│  deps: Link  │                      │
│  │  subsTail    │   Link  │  depsTail    │                      │
│  └──────────────┘         └──────────────┘                      │
│         │                        │                               │
│         │                        │                               │
│         ▼                        ▼                               │
│  ┌─────────────────────────────────────────────────┐            │
│  │              DEPENDENCY GRAPH                    │            │
│  │                                                  │            │
│  │    s (height 0)                                  │            │
│  │    │                                             │            │
│  │    ├────► a (height 1) ────┐                     │            │
│  │    │                       │                     │            │
│  │    └────► b (height 1) ────┼────► c (height 2)   │            │
│  │                            │                     │            │
│  └────────────────────────────┼─────────────────────┘            │
│                               │                                  │
│                               ▼                                  │
│  ┌─────────────────────────────────────────────────┐            │
│  │              DIRTY HEAP (Bucket Queue)           │            │
│  │                                                  │            │
│  │  [0] [1] [2] [3] ... [maxDirty]                 │            │
│  │   │   │   │                                      │            │
│  │   │   │   └─► c.prevHeap ──► c.nextHeap         │            │
│  │   │   │                                          │            │
│  │   │   └─────► b.prevHeap ──► b.nextHeap         │            │
│  │   │                                              │            │
│  │   └─────────► processing pointer (minDirty)      │            │
│  │                                                  │            │
│  └─────────────────────────────────────────────────┘            │
│                                                                  │
└─────────────────────────────────────────────────────────────────┘
```

---

## Link Structure - Bidirectional Connections

```
    ┌─────────────────────────────────────────────────┐
    │              SIGNAL "a"                          │
    │  ┌──────────────────────────────────────────┐   │
    │  │ subs ──────────────────────────────┐     │   │
    │  │ subsTail ◄─────────────────────┐   │     │   │
    │  └────────────────────────────────┼───┼─────┘   │
    │                                  │   │          │
    │                                  ▼   │          │
    │                            ┌─────────┴──────┐  │
    │                            │    LINK #1     │  │
    │                            │  dep: signal a │  │
    │                            │  sub: comp b   │  │
    │                            │  nextDep: ──┐  │  │
    │                            │  prevSub: null│  │
    │                            │  nextSub: null│  │
    │                            └───────────────┘  │
    │                                  │            │
    │                                  │            │
    │                                  ▼            │
    │  ┌──────────────────────────────────────────┐ │
    │  │         COMPUTED "b"                      │ │
    │  │  ┌────────────────────────────────────┐  │ │
    │  │  │ deps ──────────────────────────┐   │  │ │
    │  │  │ depsTail ◄─────────────────┐   │   │  │ │
    │  │  └────────────────────────────┼───┼───┘  │ │
    │  └───────────────────────────────┼───┼──────┘ │
    └──────────────────────────────────┼───┼────────┘
                                       │   │
                                       │   └────► Points back to LINK
                                       │
                                       └────────► Points forward to LINK
```

---

## The stabilize() Loop - Step by Step

```
Initial State: Signal "s" changes from 1 → 2

┌─────────────────────────────────────────────────────────────┐
│ STEP 1: setSignal(s, 2)                                     │
├─────────────────────────────────────────────────────────────┤
│                                                             │
│   s.value = 2                                               │
│   for each subscriber:                                      │
│     └─► insertIntoHeap(a)  // a goes in bucket [1]         │
│                                                             │
│   dirtyHeap:                                                │
│   [0]: undefined                                            │
│   [1]: a ─► (prevHeap=a, nextHeap=undefined)               │
│   [2]: undefined                                            │
│   maxDirty = 1                                              │
│                                                             │
└─────────────────────────────────────────────────────────────┘
                          │
                          ▼
┌─────────────────────────────────────────────────────────────┐
│ STEP 2: stabilize() begins                                  │
├─────────────────────────────────────────────────────────────┤
│                                                             │
│   minDirty = 0, maxDirty = 1                               │
│                                                             │
│   ITERATION minDirty=0:                                     │
│   ─────────────────────                                     │
│   dirtyHeap[0] = undefined  →  skip                        │
│                                                             │
│   ITERATION minDirty=1:                                     │
│   ─────────────────────                                     │
│   el = dirtyHeap[1] = a                                     │
│   dirtyHeap[1] = undefined                                  │
│                                                             │
│   while (el !== undefined):                                 │
│     recompute(a, false)                                     │
│     ────────────────────                                    │
│     1. runDisposal(a)                                       │
│     2. context = a                                          │
│     3. a.value = a.fn()  →  reads s, gets 3                │
│     4. a.value changed (2→3)!                               │
│     5. for each sub of a:                                   │
│        └─► insertIntoHeap(b)  // b goes in bucket [2]      │
│     6. el = next (= undefined)                              │
│                                                             │
│   minDirty++ → 2, loop ends                                 │
│                                                             │
└─────────────────────────────────────────────────────────────┘
                          │
                          ▼
┌─────────────────────────────────────────────────────────────┐
│ STEP 3: stabilize() continues (b was added mid-loop)        │
├─────────────────────────────────────────────────────────────┤
│                                                             │
│   Note: maxDirty was updated to 2 during recompute         │
│   So we continue...                                         │
│                                                             │
│   ITERATION minDirty=2:                                     │
│   ─────────────────────                                     │
│   el = dirtyHeap[2] = b                                     │
│   dirtyHeap[2] = undefined                                  │
│                                                             │
│   recompute(b, false):                                      │
│   ───────────────────────                                   │
│   1. b.value = b.fn()  →  reads a, gets 4                  │
│   2. b.value changed (3→4)!                                 │
│   3. notify b's subscribers...                              │
│                                                             │
└─────────────────────────────────────────────────────────────┘

Final State:
  s.value = 2, a.value = 3, b.value = 4
  dirtyHeap: all undefined
  minDirty = 3, maxDirty = 2 (loop exits)
```

---

## Three-Color Marking Flow

```
Legend:
  ○ = None (clean)
  ◐ = Check (may need update)
  ● = Dirty (must update)

Initial State (all clean):
    ○ s
    │
    ○ a    ○ b
     \    /
       ○ c

After setSignal(s, newValue):
    ● s  (value changed)
    │
    ◐ a  (marked Check, inserted in heap)
    ◐ b  (marked Check, inserted in heap)
     \  /
      ○ c

During stabilize() - processing a:
    ○ s  (signals don't have flags)
    │
    ● a  (recomputed, value changed)
    ◐ b
     \  /
      ◐ c  (marked Check by a)

During stabilize() - processing b:
    ○ s
    │
    ○ a  (flags reset after processing)
    ● b  (recomputed, value changed)
     \  /
      ◐ c  (still Check, may become Dirty if b's change affects it)

Final state after c processed:
    All nodes return to None (clean)
```

---

## Dynamic Dependency Switching

```javascript
const mode = signal('A');
const dataA = signal(10);
const dataB = signal(20);

const result = computed(() => {
  if (read(mode) === 'A') {
    return read(dataA) * 2;
  } else {
    return read(dataB) + 5;
  }
});
```

State Diagram:

```
BEFORE: mode='A', result depends on [mode, dataA]

    mode ─────┐
              ├──► result
    dataA ────┘

    dataB (not connected - no subscription)


ACTION: setSignal(mode, 'B')

1. result is marked Dirty, inserted in heap
2. stabilize() processes result
3. result.fn() runs:
   - reads mode → 'B'
   - takes ELSE branch
   - reads dataB (NEW subscription!)
   - does NOT read dataA


AFTER: mode='B', result depends on [mode, dataB]

    mode ─────┐
              ├──► result
    dataA     │
              │
    dataB ────┘

    dataA is unlinked via unlinkSubs()
    - result removed from dataA.subs
    - dataA removed from result.deps
```

---

## Firewall Signals - Component Boundaries

```
Global Context
    │
    └─► selector (computed)
         │
         │ Reads: selected
         │
         └─► child: [firewallSignalA, firewallSignalB]
              │
              │ Each firewall signal has:
              │ - owner: selector
              │ - nextChild: sibling
              │ - subs: external subscribers
              │
              ├─► firewallSignalA
              │    └─► effect1 (subscribed to A)
              │
              └─► firewallSignalB
                   └─► effect2 (subscribed to B)


When selector recomputes:
1. selector marks all firewall children
2. markNode() propagates through firewall signals
3. All subscribers of all firewall signals marked Check
4. This ensures component-wide invalidation
```

---

## Height Adjustment - Preventing Glitches

```
Scenario: Reading a signal that's "higher" than current context

    s1 (h=0)     s2 (h=0)
      │            │
      │            │
    c1 (h=1) ◄────┘
      │    ▲
      │    │ reads s2 out-of-order!
      │    │
      └────┘
         c2 (h=?)

Without height adjustment:
  c2 would get wrong height

With height adjustment in read():
  if (height >= context.height) {
    context.height = height + 1;
  }

Result:
  c2 adjusts its height to be > max(dep heights)
  This maintains topological ordering
```

---

## Memory Layout - Linked Lists

```
dirtyHeap as doubly-linked lists at each height:

dirtyHeap[0]: undefined
dirtyHeap[1]: a
              │
              ├─ a.prevHeap = a (self-loop = head marker)
              ├─ a.nextHeap = b
              │
              ▼
              b
              │
              ├─ b.prevHeap = a
              ├─ b.nextHeap = c
              │
              ▼
              c
              │
              ├─ c.prevHeap = b
              ├─ c.nextHeap = undefined (tail)

Insert at height 1 (new node d):
  1. d.prevHeap = current tail (c)
  2. c.nextHeap = d
  3. d.nextHeap = undefined

Delete from heap (remove b):
  1. a.nextHeap = c
  2. c.prevHeap = a
  3. b.prevHeap = b (self-loop marker)
  4. b.nextHeap = undefined
```

---

## Execution Trace - Diamond Dependency

```javascript
const s = signal(1);
const a = computed(() => read(s) + 1);  // h=1
const b = computed(() => read(s) + 2);  // h=1
const c = computed(() => read(a) * read(b));  // h=2
stabilize();

setSignal(s, 2);
stabilize();
```

Trace:

```
INITIAL COMPUTATION (stabilize #1):
═══════════════════════════════════════

context = null
a.fn() executes:
  context = a
  read(s) → links s→a, returns 1
  a.value = 2
  a.height = 0 (no context, so computed() sets 0)

context = null
b.fn() executes:
  context = b
  read(s) → links s→b, returns 1
  b.value = 3
  b.height = 0

context = null
c.fn() executes:
  context = c
  read(a) → links a→c
    a is computed, needs update? No (already has value)
    returns 2
  read(b) → links b→c
    b is computed, needs update? No
    returns 3
  c.value = 6
  c.height = 1 (context was c, but adjusted during reads)


AFTER setSignal(s, 2):
═══════════════════════════════════════

s.value = 2
s.subs = [link to a, link to b]

For each sub:
  insertIntoHeap(a) → dirtyHeap[0] = a
  insertIntoHeap(b) → dirtyHeap[0] = b → a


STABILIZE #2:
═══════════════════════════════════════

minDirty = 0
el = dirtyHeap[0] = b (process in heap order)

recompute(b):
  b.fn() executes:
    read(s) → s.value = 2
    b.value = 4 (changed from 3!)
    Mark c as Check
    insertIntoHeap(c) → dirtyHeap[1] = c

el = a (next in heap)

recompute(a):
  a.fn() executes:
    read(s) → s.value = 2
    a.value = 3 (changed from 2!)
    Mark c as Check (already marked)
    insertIntoHeap(c) → already in heap (InHeap flag)

minDirty = 1
el = dirtyHeap[1] = c

recompute(c):
  c.fn() executes:
    read(a) → 3
    read(b) → 4
    c.value = 12 (changed from 6!)
    No subscribers to notify

DONE: s=2, a=3, b=4, c=12
```

---

## Comparison: R3 vs Traditional Push-Pull

```
TRADITIONAL PUSH-PULL-PUSH:
═══════════════════════════

    s
    │
    a
   / \
  b   c
   \ /
    d

s changes:
1. PUSH: Mark a,b,c,d all as DIRTY (eager, O(n))
2. PULL: When reading d, check if dirty, update if needed
3. PUSH: Propagate results

Problem: b and c marked dirty even if d is never read!


R3 HEIGHT-ORDERED APPROACH:
═══════════════════════════

    s (h=0)
    │
    a (h=1)
   / \
  b   c (h=2)
   \ /
    d (h=3)

s changes:
1. PUSH: Insert a in heap[1] only
2. Process heap[1]: recompute a, insert b,c in heap[2]
3. Process heap[2]: recompute b,c, insert d in heap[3]
4. Process heap[3]: recompute d (only if d has subscribers!)

Advantage: d is only computed if someone cares!
```

---

## Key Invariants

```
1. TOPOLOGICAL ORDERING
   ─────────────────────
   For all edges (dep → sub):
     dep.height < sub.height

   Maintained by: read() height adjustment


2. HEAP ORDERING
   ──────────────
   dirtyHeap[h] contains all dirty nodes at height h
   Process from h=0 to maxDirty

   Maintained by: insertIntoHeap(), deleteFromHeap()


3. LINK CONSISTENCY
   ─────────────────
   If link in dep.subs:
     link.sub.deps must contain link

   If link not in dep.subs:
     link must be removed from link.sub.deps

   Maintained by: link(), unlinkSubs()


4. FLAG PROGRESSION
   ─────────────────
   None → Check → Dirty → (recompute) → None

   Never skip states backwards.

   Maintained by: markNode(), recompute()
```
