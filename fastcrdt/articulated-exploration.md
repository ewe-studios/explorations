---
name: articulated
description: CRDT-inspired TypeScript library for stable element identifiers in mutable lists using B+Tree compression and tombstone-based deletion
type: sub-project
source: /home/darkvoid/Boxxed/@formulas/src.rust/src.wasm/src.fastcrdt/articulated/
---

# articulated - Stable Identifiers for Mutable Lists

## Overview

**articulated** is a TypeScript library that provides a CRDT-inspired approach to managing stable element identifiers in mutable lists. It uses a modified B+Tree data structure with compression techniques to efficiently track elements that maintain their identity even as their positions change due to insertions and deletions.

### Key Value Proposition

- **Stable identifiers** - Elements keep their identity regardless of index changes
- **Efficient storage** - Compressed representation (10-20 ElementIds per leaf node)
- **Collaborative-ready** - Supports concurrent operations from multiple sources
- **Tombstone support** - Deleted elements remain addressable for future operations
- **Persistent data structure** - Immutable updates with memory sharing
- **TypeScript-first** - Full type safety with excellent IDE integration

### Example Usage

```typescript
import { IdList } from "articulated";

// Create an empty list
let list = IdList.new();

// Insert elements with stable IDs
list = list.insertAfter(null, { bunchId: "user1", counter: 0 });
list = list.insertAfter(
  { bunchId: "user1", counter: 0 },
  { bunchId: "user1", counter: 1 }
);

// Delete an element (marks as tombstone, remains known)
list = list.delete({ bunchId: "user1", counter: 0 });

// Check status
console.log(list.has({ bunchId: "user1", counter: 0 }));     // false (deleted)
console.log(list.isKnown({ bunchId: "user1", counter: 0 })); // true (known)

// Insert relative to deleted element (works because it's still known)
list = list.insertAfter(
  { bunchId: "user1", counter: 0 },
  { bunchId: "user2", counter: 0 }
);

// Save and load state
const savedState = list.save();
const newList = IdList.load(savedState);
```

## Directory Structure

```
/home/darkvoid/Boxxed/@formulas/src.rust/src.wasm/src.fastcrdt/articulated/
├── src/
│   ├── index.ts                      # Main exports
│   ├── id.ts                         # ElementId interface and utilities
│   ├── id_list.ts                    # Core IdList implementation (B+Tree)
│   ├── saved_id_list.ts              # JSON save format types
│   └── internal/
│       ├── leaf_map.ts               # Leaf node mapping (Red-Black Tree)
│       └── seq_map.ts                # Sequence number mapping
├── test/
│   ├── basic.test.ts                 # Basic functionality tests
│   ├── basic_fuzz.test.ts            # Fuzz testing
│   ├── btree_fuzz.test.ts            # B+Tree structure fuzzing
│   ├── btree_implementation.test.ts  # B+Tree implementation tests
│   ├── btree_structure_and_edge_cases.test.ts
│   ├── fuzzer.ts                     # Fuzzing infrastructure
│   ├── id_list_simple.ts             # Simple reference implementation
│   └── persistence.test.ts           # Persistence tests
├── benchmarks/
│   ├── main.ts                       # Benchmark runner
│   ├── insert_after_custom.ts        # Custom encoding benchmarks
│   ├── insert_after_json.ts          # JSON encoding benchmarks
│   └── internal/
│       ├── real_text_trace_edits.json # Real edit trace
│       └── util.ts                   # Benchmark utilities
├── benchmark_results.md              # Published benchmark results
├── package.json
├── tsconfig.json
└── README.md
```

## Core Concepts

### 1. ElementId - Stable Identifiers

```typescript
/**
 * A unique and immutable id for a list element.
 *
 * ElementIds are conceptually the same as UUIDs (or nanoids).
 * When a single thread generates a series of ElementIds, you can
 * generate a single `bunchId` and use that with varying `counter`.
 */
export interface ElementId {
  /**
   * A UUID or similar globally unique ID.
   * Must be unique even if another part of your application creates
   * ElementIds concurrently (possibly on a different device).
   */
  readonly bunchId: string;

  /**
   * An integer used to distinguish ElementIds in the same bunch.
   *
   * Typically assigned sequentially: 0, 1, 2, ... for elements
   * inserted in left-to-right order.
   */
  readonly counter: number;
}

// Equals function
export function equalsId(a: ElementId, b: ElementId): boolean {
  return a.counter === b.counter && a.bunchId === b.bunchId;
}

// Expand sequential IDs
export function expandIds(startId: ElementId, count: number): ElementId[] {
  const ans: ElementId[] = [];
  for (let i = 0; i < count; i++) {
    ans.push({ bunchId: startId.bunchId, counter: startId.counter + i });
  }
  return ans;
}
```

**BunchId Optimization Pattern:**

```typescript
// User types "Hello" - generate one bunchId for all characters
const bunchId = generateUUID();  // e.g., "abc123"

// Assign sequential counters:
// { bunchId: "abc123", counter: 0 }  // 'H'
// { bunchId: "abc123", counter: 1 }  // 'e'
// { bunchId: "abc123", counter: 2 }  // 'l'
// { bunchId: "abc123", counter: 3 }  // 'l'
// { bunchId: "abc123", counter: 4 }  // 'o'

// IdList compresses these into a single entry:
// { bunchId: "abc123", startCounter: 0, count: 5, isDeleted: false }
```

### 2. IdList - B+Tree Implementation

The core data structure is a modified B+Tree with no keys, only values:

```typescript
// B+Tree branching factor (max children per node)
// Chosen for cache efficiency: 64 byte cache line / 8 byte pointer = 8
export const M = 8;

/**
 * Leaf node in the B+Tree.
 *
 * Each leaf represents a compressed group of ElementIds with:
 * - Same bunchId
 * - Sequential counters
 * - Optional deletion tracking via `present` SparseIndices
 */
export interface LeafNode {
  readonly bunchId: string;
  readonly startCounter: number;
  readonly count: number;
  /**
   * SparseIndices tracks which counters are present (not deleted).
   * Indexed by counter, not by (counter - startCounter).
   */
  readonly present: SparseIndices;
}

/**
 * Inner node with inner-node children.
 */
export class InnerNodeInner {
  constructor(
    readonly seq: number,           // Unique sequence number
    readonly children: readonly InnerNode[],
    parentSeqsMut: MutableSeqMap | null  // Updates parent map
  ) {
    // Aggregate statistics
    this.size = children.reduce((sum, c) => sum + c.size, 0);
    this.knownSize = children.reduce((sum, c) => sum + c.knownSize, 0);
  }

  readonly size: number;      // Present (non-deleted) ids
  readonly knownSize: number; // Total known ids (including deleted)
}

/**
 * Inner node with leaf children.
 */
export class InnerNodeLeaf {
  constructor(
    readonly seq: number,
    readonly children: readonly LeafNode[],
    leafMapMut: MutableLeafMap | null  // Updates leaf map
  ) {
    this.size = children.reduce((sum, c) => sum + c.present.count(), 0);
    this.knownSize = children.reduce((sum, c) => sum + c.count, 0);
  }

  readonly size: number;
  readonly knownSize: number;
}

export type InnerNode = InnerNodeInner | InnerNodeLeaf;
```

### 3. Persistent Maps for Navigation

**LeafMap - Maps leaves to parent sequence numbers:**

```typescript
import createRBTree, { Tree } from "../vendor/functional-red-black-tree";

/**
 * A persistent sorted map from each LeafNode to its parent's seq.
 *
 * Leaves are sorted by their first ElementId (bunchId, startCounter).
 * This enables quick lookup of the LeafNode containing an ElementId.
 */
export class LeafMap {
  private constructor(private readonly tree: Tree<LeafNode, number>) {}

  static new() {
    return new this(createRBTree(compareLeaves));
  }

  /**
   * Returns the greatest leaf whose first id is <= the given id,
   * or undefined if none exists. Also returns the associated seq.
   *
   * The returned leaf might not actually contain the given id
   * (you need to check counter range).
   */
  getLeaf(bunchId: string, counter: number): [LeafNode | undefined, number] {
    const iter = this.tree.le({ bunchId, startCounter: counter } as LeafNode);
    return [iter.key, iter.value ?? -1];
  }

  set(leaf: LeafNode, seq: number): LeafMap {
    return new LeafMap(this.tree.set(leaf, seq));
  }

  delete(leaf: LeafNode): LeafMap {
    return new LeafMap(this.tree.remove(leaf));
  }
}

function compareLeaves(a: LeafNode, b: LeafNode): number {
  if (a.bunchId === b.bunchId) {
    return a.startCounter - b.startCounter;
  }
  return a.bunchId > b.bunchId ? 1 : -1;
}
```

**SeqMap - Maps inner node sequences to parent sequences:**

```typescript
/**
 * A persistent map from an InnerNode's seq to its parent's seq
 * (or 0 for the root).
 *
 * Sequence numbers start at 1 and increment each time you call set().
 */
export class SeqMap {
  constructor(
    private readonly tree: Tree<number, number>,
    private readonly nextSeq: number
  ) {}

  static new(): SeqMap {
    return new this(createRBTree((a, b) => a - b), 1);
  }

  bumpNextSeq(): SeqMap {
    return new SeqMap(this.tree, this.nextSeq + 1);
  }

  get(seq: number): number {
    return this.tree.get(seq)!;
  }

  set(seq: number, value: number): SeqMap {
    return new SeqMap(this.tree.set(seq, value), this.nextSeq);
  }
}
```

### 4. IdList API

```typescript
export class IdList {
  /**
   * Constructs an empty list.
   */
  static new(): IdList { }

  /**
   * Constructs a list with the given known ids and their isDeleted status.
   */
  static from(knownIds: Iterable<{ id: ElementId; isDeleted: boolean }>): IdList { }

  /**
   * Constructs a list with the given present ids.
   */
  static fromIds(ids: Iterable<ElementId>): IdList { }

  /**
   * Inserts `newId` immediately after `before`.
   *
   * @param before May be null to insert at the beginning.
   *               May be deleted (tombstone).
   * @param count Bulk-insert count sequential IDs with same bunchId.
   * @returns New IdList (immutable - old list unchanged).
   */
  insertAfter(before: ElementId | null, newId: ElementId, count?: number): IdList { }

  /**
   * Inserts `newId` immediately before `after`.
   */
  insertBefore(after: ElementId | null, newId: ElementId, count?: number): IdList { }

  /**
   * Marks an id as deleted (tombstone).
   * Deleted ids remain known and addressable.
   */
  delete(id: ElementId): IdList { }

  /**
   * Returns the number of present (non-deleted) elements.
   */
  length(): number { }

  /**
   * Returns true if the id is present (not deleted).
   */
  has(id: ElementId): boolean { }

  /**
   * Returns true if the id is known (present or deleted).
   */
  isKnown(id: ElementId): boolean { }

  /**
   * Returns the id at the given present index.
   */
  getIdAtIndex(index: number): ElementId | null { }

  /**
   * Returns the present index of the given id.
   */
  indexOf(id: ElementId): number { }

  /**
   * Iterates over present (non-deleted) ids in list order.
   */
  [Symbol.iterator](): Iterator<ElementId> { }

  /**
   * Saves the list state to JSON format.
   */
  save(): SavedIdList { }

  /**
   * Loads a list from saved state.
   */
  static load(saved: SavedIdList): IdList { }

  /**
   * Returns the maximum counter for a given bunchId.
   * Useful for generating the next counter.
   */
  maxCounter(bunchId: string): number { }
}
```

### 5. Save Format (SavedIdList)

```typescript
/**
 * JSON saved state for an IdList.
 *
 * Describes all known ElementIds in list order, with compression:
 * Sequential ElementIds with the same bunchId, isDeleted status,
 * and sequential counters are combined into a single object.
 */
export type SavedIdList = Array<{
  readonly bunchId: string;
  readonly startCounter: number;
  readonly count: number;
  readonly isDeleted: boolean;
}>;

// Example saved state:
const saved: SavedIdList = [
  { bunchId: "abc123", startCounter: 0, count: 5, isDeleted: false },  // 'H','e','l','l','o'
  { bunchId: "abc123", startCounter: 5, count: 1, isDeleted: true },   // deleted ' '
  { bunchId: "abc123", startCounter: 6, count: 5, isDeleted: false },  // 'W','o','r','l','d'
];

// Save/load
const list = IdList.load(saved);
const json = JSON.stringify(list.save());
const restored = IdList.load(JSON.parse(json));
```

### 6. SparseIndices for Deletion Tracking

```typescript
import { SparseIndices } from "sparse-array-rled";

/**
 * SparseIndices efficiently tracks which counters are present.
 *
 * Uses run-length encoding for compression.
 */
interface LeafNode {
  readonly present: SparseIndices;
}

// Example:
const present = SparseIndices.new();
present.set(0, 5);   // counters 0,1,2,3,4 are present
present.set(7, 3);   // counters 7,8,9 are present
// counters 5,6 are deleted (tombstones)

console.log(present.count());  // 8 present ids
console.log(present.has(3));   // true
console.log(present.has(5));   // false (deleted)
```

## B+Tree Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                    IdList B+Tree Structure                       │
│                                                                 │
│                     Root (InnerNodeInner)                       │
│                    seq: 5, size: 100, knownSize: 120            │
│                          /           \                          │
│                         /             \                         │
│                        /               \                        │
│         ┌─────────────┴──────┐   ┌──────┴──────────────┐       │
│         │ InnerNodeLeaf      │   │ InnerNodeLeaf       │       │
│         │ seq: 3             │   │ seq: 4              │       │
│         │ size: 50           │   │ size: 50            │       │
│         │ knownSize: 60      │   │ knownSize: 60       │       │
│         └─────┬──────┬──────┘   └──────┬──────┬────────┘       │
│               /      \                  \      \                │
│              /        \                  \      \               │
│    ┌────────┴─┐  ┌────┴────────┐  ┌─────┴───┐  └────────┐     │
│    │ Leaf 1   │  │ Leaf 2      │  │ Leaf 3  │  │ Leaf 4 │     │
│    │ bunchId: │  │ bunchId:    │  │ bunchId:│  │ bunchId:│    │
│    │ "abc"    │  │ "abc"       │  │ "def"   │  │ "def"  │     │
│    │ start: 0 │  │ start: 20   │  │ start:0 │  │ start: │     │
│    │ count:20 │  │ count:20    │  │ count:  │  │ count: │     │
│    │present:  │  │ present:    │  │         │  │        │     │
│    │ [all]    │  │ [some del]  │  │         │  │        │     │
│    └──────────┘  └─────────────┘  └─────────┘  └────────┘     │
│                                                                 │
│  Statistics:                                                    │
│  - size = count of present (non-deleted) ids                   │
│  - knownSize = total known ids (including tombstones)          │
│                                                                 │
│  Navigation:                                                    │
│  - parentSeqs: InnerNode.seq → parent.seq                      │
│  - leafMap: LeafNode → parent.seq (sorted by bunchId,counter)  │
└─────────────────────────────────────────────────────────────────┘
```

## Insert Operation Flow

```typescript
// Insert "X" after element with id {bunchId: "abc", counter: 10}

list.insertAfter({ bunchId: "abc", counter: 10 }, { bunchId: "xyz", counter: 0 });

// Internal flow:
//
// 1. Locate the leaf containing counter 10
//    - Use leafMap to find candidate leaf
//    - Verify counter is in range [startCounter, startCounter + count)
//
// 2. Check if newId can extend the leaf:
//    if (leaf.bunchId === newId.bunchId &&
//        leaf.startCounter + leaf.count === newId.counter) {
//      // Extend leaf forward
//      leaf.count++;
//      leaf.present.set(newId.counter);
//    }
//
// 3. Otherwise, may need to split leaf:
//    - Create left leaf with ids before insertion point
//    - Create new leaf for inserted id
//    Create right leaf with ids after insertion point
//
// 4. Update parent statistics:
//    - Recalculate size and knownSize up the tree
//
// 5. Return new IdList (persistent - shares unchanged nodes)
```

## Benchmark Results

Based on automerge-perf 260k edit text trace:

### JSON Encoding

| Metric | Value |
|--------|-------|
| Sender time (ms) | 2229 |
| Avg update size (bytes) | 147.3 |
| Receiver time (ms) | 2214 |
| Save time (ms) | 14 |
| Save size (bytes) | 1,177,551 |
| Save size GZIP'd | 65,897 |
| Load time (ms) | 28 |
| Memory used (MB) | 2.7 |

### Custom Encoding

| Metric | Value |
|--------|-------|
| Sender time (ms) | 1943 |
| Avg update size (bytes) | 45.6 |
| Receiver time (ms) | 3237 |
| Save time (ms) | 13 |
| Save size (bytes) | 1,177,551 |
| Save size GZIP'd | 65,889 |
| Load time (ms) | 19 |
| Memory used (MB) | 2.7 |

**Note:** Final text is 104,852 bytes (27,556 bytes GZIP'd) - ~15 pages of text.

## Use Cases

### 1. Collaborative Text Editing

```typescript
// Each character gets a stable ElementId
const doc = IdList.new();

// User A types "Hello"
let bunchIdA = uuid();
for (let i = 0; i < 5; i++) {
  doc = doc.insertAfter(null, { bunchId: bunchIdA, counter: i });
}

// User B concurrently types "World" at end
let bunchIdB = uuid();
for (let i = 0; i < 5; i++) {
  doc = doc.insertAfter(
    { bunchId: bunchIdA, counter: 4 },
    { bunchId: bunchIdB, counter: i }
  );
}

// User A deletes 'H'
doc = doc.delete({ bunchId: bunchIdA, counter: 0 });

// 'H' remains known - can still insert relative to it
doc = doc.insertAfter(
  { bunchId: bunchIdA, counter: 0 },  // Deleted but known!
  { bunchId: bunchIdA, counter: 100 } // New character
);
```

### 2. Todo List with Stable IDs

```typescript
// Each todo item gets a stable ElementId
const todos = IdList.new();

// Add todos
const session1 = uuid();
todos = todos.insertAfter(null, { bunchId: session1, counter: 0 });  // Todo 1
todos = todos.insertAfter(null, { bunchId: session1, counter: 1 });  // Todo 2

// Reorder: move Todo 2 before Todo 1
// (In practice, you'd track element-to-todo mapping separately)

// Delete Todo 1
todos = todos.delete({ bunchId: session1, counter: 0 });

// Todo 1's id remains known for undo/reorder operations
```

### 3. Server Reconciliation Architecture

```typescript
// Optimistic UI updates with server reconciliation
let pendingList = IdList.new();
let confirmedList = IdList.new();

// User adds item - optimistically update UI
const newId = { bunchId: uuid(), counter: 0 };
pendingList = pendingList.insertAfter(null, newId);

// When server confirms:
confirmedList = confirmedList.insertAfter(null, newId);
pendingList = confirmedList;  // Reconcile
```

## Key Insights

1. **Tombstones enable collaboration** - Deleted ids remain known for concurrent operations
2. **BunchId compression is key** - 10-20 ElementIds per leaf node in typical use
3. **Persistent data structures** - Immutable updates enable easy undo/redo
4. **No keys in B+Tree** - Order determined "by fiat" via insertAfter/insertBefore
5. **Statistics at each node** - size and knownSize enable indexed access in O(log n)

## Open Questions

- How does this compare to full CRDTs like Yjs or Automerge?
- What's the memory overhead of the B+Tree structure?
- How to handle very large documents (millions of elements)?
- What concurrency model pairs best with this approach?
