---
location: /home/darkvoid/Boxxed/@formulas/src.rust/src.wasm/src.fastcrdt/articulated
repository: https://github.com/mweidner037/articulated
explored_at: 2026-03-22
language: TypeScript
---

# Project Exploration: articulated (fastcrdt)

## Overview

**articulated** is a TypeScript library for managing stable element identifiers in mutable lists. It provides a CRDT-inspired approach to tracking elements that maintain their identity even as their positions change due to insertions and deletions. While not a full CRDT itself, it provides the foundational data structure needed for collaborative editing and similar applications.

### Key Value Proposition

- **Stable identifiers** - Elements keep their identity even as indices change
- **Efficient storage** - Optimized compression for sequential IDs (10-20 ElementIds per leaf)
- **Collaborative-ready** - Supports concurrent operations from multiple sources
- **Tombstone support** - Deleted elements remain addressable for future operations
- **Persistent data structure** - Immutable updates with memory sharing for easy rollbacks
- **TypeScript-first** - Full type safety and excellent IDE integration

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

## Repository Structure

```
/home/darkvoid/Boxxed/@formulas/src.rust/src.wasm/src.fastcrdt/articulated/
├── src/
│   ├── index.ts                      # Main exports
│   ├── id.ts                         # ElementId interface and utilities
│   ├── id_list.ts                    # Core IdList implementation (B+Tree)
│   ├── saved_id_list.ts              # JSON save format types
│   └── internal/
│       ├── leaf_map.ts               # Leaf node mapping
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
│       ├── real_text_trace_edits.json
│       └── util.ts
├── benchmark_results.md              # Published benchmark results
├── package.json
├── tsconfig.json
└── README.md
```

## Core Concepts

### 1. ElementId

An `ElementId` is a globally unique identifier for a list element:

```typescript
interface ElementId {
  readonly bunchId: string;   // UUID or similar globally unique ID
  readonly counter: number;    // Distinguishes elements in same bunch
}
```

**Key insight:** When generating IDs sequentially (like typing characters left-to-right), use the same `bunchId` with sequential counters for optimal compression:

```typescript
// IDs that compress well (same bunchId, sequential counters)
const id1 = { bunchId: "abc123", counter: 0 };
const id2 = { bunchId: "abc123", counter: 1 };
const id3 = { bunchId: "abc123", counter: 2 };

// These will be stored as a single compressed object:
// { bunchId: "abc123", startCounter: 0, count: 3 }
```

### 2. Stable Identifier Problem

Traditional array indices are unstable:

```
Initial:  ["A", "B", "C"]
           0    1    2

Insert "X" at position 0:
["X", "A", "B", "C"]
 0    1    2    3

// Problem: "A" was at index 0, now at index 1
// How do we reference "A" stably?
```

With ElementIds:

```
Initial:  [{id: A}, {id: B}, {id: C}]

Insert {id: X} after null:
[{id: X}, {id: A}, {id: B}, {id: C}]

// Element A still has the same ID regardless of position
```

### 3. Tombstone Model

Deleted elements remain known as "tombstones":

```typescript
list = list.insertAfter(null, { bunchId: "u1", counter: 0 }); // "A"
list = list.insertAfter(null, { bunchId: "u1", counter: 1 }); // "B"
list = list.delete({ bunchId: "u1", counter: 0 });            // Delete "A"

// "A" is deleted but still KNOWN
list.has({ bunchId: "u1", counter: 0 });     // false (not present)
list.isKnown({ bunchId: "u1", counter: 0 }); // true (tombstone exists)

// Can still insert relative to deleted element
list = list.insertAfter(
  { bunchId: "u1", counter: 0 },
  { bunchId: "u2", counter: 0 }
);
```

**Why tombstones?** In collaborative editing, concurrent operations may reference elements you've deleted locally:

```
Device 1: Deletes element A
Device 2: Inserts X after A (concurrent operation)

// Without tombstones: Device 1 can't process Device 2's operation
// With tombstones: "A" is still known, insertion works correctly
```

### 4. Persistent (Immutable) Data Structure

All mutating operations return a new IdList:

```typescript
let list1 = IdList.new();
let list2 = list1.insertAfter(null, id1);  // list1 unchanged
let list3 = list2.delete(id1);             // list2 unchanged

// Memory is shared between versions where possible
```

**Benefits:**
- Easy rollbacks (keep reference to old version)
- Safe for server reconciliation architectures
- Time-travel debugging
- No accidental mutations

## B+Tree Implementation

### Tree Structure

The IdList uses a modified B+Tree where:

1. **No keys** - Order is determined by insertion order (insertAfter/insertBefore)
2. **Compressed leaves** - Each leaf represents multiple ElementIds with same bunchId and sequential counters
3. **Subtree statistics** - Each node tracks `size` (present IDs) and `knownSize` (all known IDs including tombstones)

```
┌─────────────────────────────────────────────────────────────────┐
│                    IdList B+Tree Structure                       │
│                                                                  │
│                    ┌─────────────────┐                          │
│                    │   Root Node     │                          │
│                    │   (InnerNode)   │                          │
│                    │   seq: 1        │                          │
│                    │   size: 100     │                          │
│                    │   knownSize:120 │                          │
│                    └────────┬────────┘                          │
│                             │                                   │
│           ┌─────────────────┼─────────────────┐                │
│           ▼                 ▼                 ▼                │
│    ┌─────────────┐   ┌─────────────┐   ┌─────────────┐         │
│    │ InnerNode   │   │ InnerNode   │   │ InnerNode   │         │
│    │ seq: 2      │   │ seq: 3      │   │ seq: 4      │         │
│    └──────┬──────┘   └──────┬──────┘   └──────┬──────┘         │
│           │                 │                 │                 │
│    ┌──────┴──────┐   ┌──────┴──────┐   ┌──────┴──────┐         │
│    ▼      ▼      ▼   ▼      ▼      ▼   ▼      ▼      ▼         │
│  ┌────┐ ┌────┐ ┌────┐ ┌────┐ ┌────┐ ┌────┐ ┌────┐ ┌────┐      │
│  │Leaf│ │Leaf│ │Leaf│ │Leaf│ │Leaf│ │Leaf│ │Leaf│ │Leaf│      │
│  │ 10 │ │ 15 │ │ 8  │ │ 12 │ │ 20 │ │ 5  │ │ 18 │ │ 12 │      │
│  │ids │ │ids │ │ids │ │ids │ │ids │ │ids │ │ids │ │ids │      │
│  └────┘ └────┘ └────┘ └────┘ └────┘ └────┘ └────┘ └────┘      │
│                                                                  │
│  Each leaf stores: { bunchId, startCounter, count, present }   │
│  where `present` is a SparseIndices of non-deleted counters    │
└─────────────────────────────────────────────────────────────────┘
```

### Leaf Node Structure

```typescript
interface LeafNode {
  readonly bunchId: string;      // Common bunchId for all IDs in leaf
  readonly startCounter: number; // First counter value
  readonly count: number;        // Number of sequential IDs
  readonly present: SparseIndices; // Which counters are not deleted
}
```

**Compression example:**

```typescript
// Instead of storing 10 separate objects:
[
  { bunchId: "abc", counter: 0, present: true },
  { bunchId: "abc", counter: 1, present: true },
  { bunchId: "abc", counter: 2, present: false }, // deleted
  { bunchId: "abc", counter: 3, present: true },
  // ... etc
]

// Store as single leaf:
{
  bunchId: "abc",
  startCounter: 0,
  count: 10,
  present: SparseIndices.fromSet([0, 1, 3, 4, 5, 7, 8, 9]) // deleted: 2, 6
}
```

### Bottom-Up Tree for Fast Lookups

To quickly find the leaf containing an ElementId, articulated maintains a "bottom-up" mapping:

```typescript
// Each inner node has a unique sequence number (seq)
// Two maps maintained:

// 1. leafMap: LeafNode -> parent's seq
//    Sorted by (bunchId, startCounter) for binary search

// 2. parentSeqs: seq -> parent's seq
//    Maps each node to its parent

// Lookup algorithm for ElementId { bunchId, counter }:
// 1. Binary search leafMap for matching bunchId
// 2. Find leaf where startCounter <= counter < startCounter + count
// 3. O(log L) time where L = number of leaves
```

### Node Types

```typescript
// Inner node with inner-node children
class InnerNodeInner {
  readonly seq: number;          // Unique node identifier
  readonly children: InnerNode[];
  readonly size: number;         // Present IDs in subtree
  readonly knownSize: number;    // All known IDs in subtree
}

// Inner node with leaf children
class InnerNodeLeaf {
  readonly seq: number;
  readonly children: LeafNode[];
  readonly size: number;
  readonly knownSize: number;
}

// B+Tree branching factor
const M = 8; // Max children per node
```

## API Reference

### Basic Operations

| Method | Description | Time Complexity |
|--------|-------------|-----------------|
| `insertAfter(before, newId, count?)` | Insert after element (or null for start) | O(log²L + F) |
| `insertBefore(after, newId, count?)` | Insert before element (or null for end) | O(log²L + F) |
| `delete(id)` | Mark as deleted (tombstone) | O(log²L + F) |
| `undelete(id)` | Restore deleted element | O(log²L + F) |
| `uninsert(id, count?)` | Completely remove (inverse of insert) | O(log²L + F) |

### Accessors

| Method | Description | Time Complexity |
|--------|-------------|-----------------|
| `at(index)` | Get ElementId at index | O(log L + F) |
| `indexOf(id, bias?)` | Get index of ElementId | O(log²L + F) |
| `has(id)` | Check if present (not deleted) | O(log L + F) |
| `isKnown(id)` | Check if known (including tombstones) | O(log L + F) |
| `length` | Count of present elements | O(1) |
| `maxCounter(bunchId)` | Get max counter for bunch | O(L) |

### Bulk Operations

```typescript
// Insert 5 sequential IDs at once
list = list.insertAfter(null, { bunchId: "user1", counter: 0 }, 5);
// Inserts: {bunchId:"user1",counter:0}, {bunchId:"user1",counter:1}, ...
```

### Save/Load

```typescript
// Save to JSON
const saved: SavedIdList = list.save();
// Format: Array<{ bunchId, startCounter, count, isDeleted }>

// Load from JSON
const newList = IdList.load(savedState);
```

### Iteration

```typescript
// Iterate over present IDs
for (const id of list) {
  console.log(id);
}

// Iterate over all known IDs (including deleted)
for (const { id, isDeleted } of list.valuesWithIsDeleted()) {
  console.log(id, isDeleted);
}

// View treating all known IDs as present
const knownView = list.knownIds;
console.log(knownView.length); // Includes tombstones
```

## Complexity Analysis

Let:
- `L` = Number of leaves
- `F` = Maximum fragmentation (alternations between deleted/present in a leaf)

| Operation | Complexity | Bottleneck |
|-----------|------------|------------|
| `insertAfter` | O(log²L + F) | Finding leaf via bottom-up tree |
| `insertBefore` | O(log²L + F) | Finding leaf via bottom-up tree |
| `delete` | O(log²L + F) | Finding leaf + updating SparseIndices |
| `undelete` | O(log²L + F) | Finding leaf + updating SparseIndices |
| `indexOf` | O(log²L + F) | Finding leaf via bottom-up tree |
| `at` | O(log L + F) | Simple B+Tree descent |
| `has`/`isKnown` | O(log L + F) | Binary search in leafMap |
| `length` | O(1) | Cached value |
| `save` | O(S + L) | S = serialized size |
| `load` | O(S × log S) | Building bottom-up tree |

**Fragmentation (F):** When elements are deleted, the `present` bitmap becomes fragmented. High fragmentation slows down leaf operations. Typical text editing has low F; adversarial deletion patterns can increase it.

## Benchmark Results

Using automerge-perf 260k edit text trace:

### Insert-After with JSON Encoding

| Metric | Value |
|--------|-------|
| Sender time | 2229 ms |
| Avg update size | 147.3 bytes |
| Receiver time | 2214 ms |
| Save time | 14 ms |
| Save size | 1,177,551 bytes |
| Save size (GZIP) | 65,897 bytes |
| Load time | 28 ms |
| Load time (GZIP) | 53 ms |
| Memory used | ~2.7 MB |

### Insert-After with Custom Encoding

| Metric | Value |
|--------|-------|
| Sender time | 1943 ms |
| Avg update size | 45.6 bytes |
| Receiver time | 3237 ms |
| Save time | 13 ms |
| Save size | 1,177,551 bytes |
| Save size (GZIP) | 65,889 bytes |
| Load time | 19 ms |
| Memory used | ~2.7 MB |

**Note:** For perspective, the final text (104,852 bytes, 27,556 GZIP'd) represents ~15 pages of two-column LaTeX text. These benchmarks track ElementIds only, not actual text content.

## Use Cases

### 1. Collaborative Text Editing

```typescript
// Each character gets a stable ElementId
const charIds = text.split('').map((c, i) => ({
  bunchId: sessionId,
  counter: i
}));

// Insert new character
list = list.insertAfter(afterCharId, newCharId);

// Delete character
list = list.delete(charId);

// Map ElementIds to actual characters via separate content store
const content = new Map<ElementId, string>();
```

### 2. Todo List with Stable References

```typescript
interface Todo {
  id: ElementId;
  text: string;
  completed: boolean;
}

// Todos maintain their ID even when reordered
let todoList = IdList.new();
const todoId = { bunchId: userId, counter: 0 };

todoList = todoList.insertAfter(null, todoId);
// ... user moves todo, deletes, restores ...
// ID remains the same throughout
```

### 3. Server Reconciliation Architecture

```typescript
// Keep optimistic state (includes unconfirmed changes)
let optimisticState = IdList.new();

// Keep confirmed state from server
let confirmedState = IdList.new();

// When user makes change
optimisticState = optimisticState.insertAfter(...);

// If server rejects, rollback is easy
optimisticState = confirmedState; // Just use old reference!
```

## Comparison with Related Work

### articulated vs Full CRDTs

| Aspect | articulated | Full CRDT (Yjs, Automerge) |
|--------|-------------|---------------------------|
| Concurrency | Requires server reconciliation | Native concurrent support |
| Complexity | Simpler | More complex |
| Performance | Optimized for single-author batches | Optimized for concurrency |
| Use case | Collaborative with central authority | Fully peer-to-peer |

### articulated vs Array Indices

| Aspect | articulated | Array Indices |
|--------|-------------|---------------|
| Stability | IDs never change | Indices shift on insert/delete |
| Tombstones | Supported | Not supported |
| Collaboration | Ready | Requires OT/CRDT layer |
| Memory | Higher (tombstones) | Minimal |

## Simple Reference Implementation

The repository includes `IdListSimple` (~300 SLOC) for understanding and testing:

```typescript
export class IdListSimple {
  private state: ListElement[]; // Simple array
  private _length: number;

  static new() {
    return new this([], 0);
  }

  insertAfter(before: ElementId | null, newId: ElementId, count = 1): void {
    const index = before === null
      ? -1
      : this.state.findIndex(elt => equalsId(elt.id, before));

    this.state.splice(index + 1, 0, ...expandElements(newId, false, count));
    this._length += count;
  }

  delete(id: ElementId): void {
    const index = this.state.findIndex(elt => equalsId(elt.id, id));
    if (index !== -1 && !this.state[index].isDeleted) {
      this.state[index].isDeleted = true;
      this._length--;
    }
  }

  // ... etc (full implementation in test/id_list_simple.ts)
}
```

**Trade-offs:** O(n) operations, one object per ID, no compression. Used as reference for fuzz testing.

## Internal Utilities

### SparseIndices

```typescript
// Efficient storage for sets of integers
// Used by LeafNode.present to track non-deleted counters

import { SparseIndices } from "sparse-array-rled";

const present = SparseIndices.fromSet([0, 1, 3, 4, 7]);
present.count();     // Number of present elements
present.has(3);      // true
present.has(2);      // false (deleted)
```

### SeqMap and LeafMap

Persistent maps using functional-red-black-tree:

```typescript
// Maps seq number to parent seq
type SeqMap = FunctionalRedBlackTree<number, number>;

// Maps LeafNode to parent seq (sorted by bunchId, startCounter)
type LeafMap = FunctionalRedBlackTree<LeafNode, number>;
```

## Trade-offs

| Design Choice | Benefit | Cost |
|---------------|---------|-----|
| Tombstones | Supports concurrent refs to deleted items | Memory grows with deletions |
| Persistence | Easy rollbacks, time-travel | Allocation overhead |
| B+Tree | O(log L) operations | Complex implementation |
| Compression | 10-20x space savings | Decompression on access |
| No concurrency | Simpler, faster | Needs server reconciliation |

## Related Projects

- **automerge-perf** - Performance tracing for CRDTs (used in benchmarks)
- **crdt-benchmarks** - CRDT benchmarking framework
- **functional-red-black-tree** - Persistent balanced trees for maps
- **sparse-array-rled** - Run-length encoded sparse arrays

## References

- [Demos](https://github.com/mweidner037/articulated-demos)
- [Server Reconciliation Architecture](https://mattweidner.com/2024/06/04/server-architectures.html#1-server-reconciliation)
- Source code: [id_list.ts](https://github.com/mweidner037/articulated/blob/main/src/id_list.ts)
- Simple implementation: [id_list_simple.ts](https://github.com/mweidner037/articulated/blob/main/test/id_list_simple.ts)
