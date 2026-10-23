# 13 -- Delta Replication vs CRDTs (Automerge)

chord's delta system is often compared to a CRDT. It is not one. It is single-writer operation-log replication. Automerge is a multi-writer CRDT. Both answer the same question, "how do I sync a shared value efficiently?", with opposite answers. This document explains the algorithm, where the two diverge, where chord's choice is optimal, and where it trades away what a CRDT gives.

Jargon first. A CRDT (conflict-free replicated data type) is a data structure where concurrent edits merge deterministically, and every replica converges to the same value no matter the delivery order. An RGA (replicated growable array) is the sequence CRDT Automerge uses for text. A Lamport clock is a counter that orders events causally. An op ID is a unique identity plus a Lamport timestamp plus an actor ID. A tombstone is a marker that an element was deleted, kept so other replicas converge.

## The Two Answers

The problem: two sides hold a copy of one JSON value. One side changes it. The other side must learn the change without resending the whole value.

Automerge's answer: let any replica edit, give every change an identity and a timestamp, and make merge a commutative join. Convergence falls out of the algebra, not the delivery.

chord's answer: forbid concurrent editors. One writer produces a linear sequence of operations. Replicas are read-only views that apply the operations in order. Convergence falls out of having one writer.

The Aha is that chord did not implement a simpler CRDT. It removed the problem a CRDT exists to solve. There is nothing to merge, because there is one author.

## What chord Actually Does

This is the algorithm, from the code.

### The Operation Vocabulary

A mutation is turned into a delta: a list of operations. Each operation is a tuple.

```ts
// packages/chord/src/delta/index.ts -- the op type (verbatim)
export type Op =
  | readonly ["r", JsonValue]
  | readonly ["s", NonEmptyPath, JsonValue]
  | readonly ["d", NonEmptyPath]
  | readonly ["a", NonEmptyPath, string]
  | readonly ["t", NonEmptyPath, number]
  | readonly ["p", Path, number, number, JsonValue[]]
  | readonly ["m", Path, number[]];
```

| Op | Meaning |
|----|---------|
| `r` | Replace the whole value. The only op that does. |
| `s` | Set a field at a path. |
| `d` | Delete a field at a path. |
| `a` | Append a string to the end of a string field. |
| `t` | Drop the first `count` characters of a string field (keep the tail). |
| `p` | Splice an array: `splice(index, deleteCount, ...items)`. |
| `m` | Reorder an array by a permutation. |

`s`, `d`, `a`, `t` never target the root. Only `r`, `p`, and `m` may. `isBase` uses this: a batch whose first op is `r` is a full replacement.

### Apply Is a Left Fold

`apply` is a plain loop over the ops.

```ts
// packages/chord/src/delta/index.ts -- apply (abridged)
export function apply<T>(target: T | undefined, ops: readonly Op[]): T {
  return applyOps(target, ops);
}

function applyOps<T>(target: T | undefined, ops: readonly Op[]): T {
  let root = target as unknown as JsonValue;
  for (const op of ops) {
    assertValidOp(op);
    if (op[0] === "r") {
      root = op[1];            // adopted, not copied
      continue;
    }
    const path = op[1];
    assertSafePath(path);
    if (op[0] === "p") {
      const target_ = path.length === 0 ? root : resolve(root, path);
      target_.splice(op[2], op[3]);                       // delete
      const chunkSize = 10_000;
      for (let offset = 0; offset < op[4].length; offset += chunkSize) {
        target_.splice(op[2] + offset, 0, ...op[4].slice(offset, offset + chunkSize));
      }
      continue;
    }
    if (op[0] === "m") {
      const target_ = path.length === 0 ? root : resolve(root, path);
      const previous = target_.slice();
      for (let index = 0; index < op[2].length; index++) target_[index] = previous[op[2][index]!]!;
      continue;
    }
    const parent = resolve(root, path.slice(0, -1));
    const key = path[path.length - 1]!;
    // defineProperty, not assignment: an inherited setter would otherwise run on write
    const write = (value) => Object.defineProperty(parent, key, { value, writable: true, enumerable: true, configurable: true });
    const read = () => (Object.hasOwn(parent, key) ? parent[key] : undefined);
    switch (op[0]) {
      case "s": write(op[2]); break;
      case "d": Array.isArray(parent) ? parent.splice(key, 1) : delete parent[key]; break;
      case "a": { const current = read(); write(`${current}${op[2]}`); break; }  // append
      case "t": { const current = read(); write(current.slice(op[2])); break; }  // drop the first N chars
    }
  }
  return root;
}
```

There are no op IDs, no timestamps, no actor IDs. There is no merge step. The fold is it.

The Aha is that `a` and `t` are positional and relative to the current value in one history. `a` appends to whatever the string is now. `t` drops the first `count` characters. They are a sliding window, not a concurrent-insertion record.

### Immutable Apply Is Copy-on-Write

`applyImmutable` produces a new value without mutating the old one.

```ts
// packages/chord/src/delta/index.ts -- applyImmutable (abridged)
export function applyImmutableBatches<T>(target: T | undefined, batches: Iterable<readonly Op[]>): T {
  let root = target as unknown as JsonValue;
  const owned = new WeakSet<object>();
  for (const ops of batches) {
    for (const op of ops) {
      if (op[0] === "r") { root = op[1]; continue; }
      if (op[0] === "p") {
        const appended = appendCopy(root, op, owned);   // concat fast path
        if (appended !== undefined) { root = appended; continue; }
      }
      root = copyContainers(root, op[0] === "p" || op[0] === "m" ? op[1] : op[1].slice(0, -1), owned);
      root = applyOps(root, [op]);
    }
  }
  return root;
}
```

`copyContainers` copies only the containers on the touched path. For `p` and `m` it copies the array's path; for `s`/`d`/`a`/`t` it copies the parent's path, because the last segment is a property write, not a container. The old value and the new value share every untouched subtree. A pure append to an array uses `concat`, one copy, instead of a copy that the splice then grows.

The Aha is structural sharing. A tiny edit deep in a large value copies only the spine to that edit. The rest is shared by reference.

### The Wire Form

Ops cross a boundary compressed. Two compressions apply.

1. Path interning: a path is assigned an id on its second use. A `["#", id, path]` definition is emitted once; later ops reference the id.
2. Arity omission: an op drops its path when it matches the previous op's path.

```ts
// packages/chord/src/delta/index.ts -- intern on second use (verbatim comment)
// Intern on SECOND use. A definition costs more than the path it replaces, so
// interning on first use loses on the many paths written exactly once.
```

A `r` batch clears the interning dictionary, so a reader replays from the last base with a fresh decoder.

### The 4096-Op Cap

A delta is capped. Past `MAX_DELTA_OPERATIONS`, the tracker falls back to a root replace.

```ts
// packages/chord/src/delta/tracker.ts -- the cap (verbatim)
const MAX_DELTA_OPERATIONS = 4_096;
```

The Aha is why. A giant change is cheaper as one full value than as thousands of ops. The cap bounds both write size and apply time.

### Storage: Base Plus Deltas

durable stores a document as a base plus a tail of deltas.

```ts
// packages/durable/src/types.ts -- document content (concept)
type DocumentContent =
  | { kind: "base"; version: number; value: JsonObject }
  | { kind: "delta"; version: number; ops: readonly Op[] };
```

Reading materializes: load the base, then `apply` each delta in order. A document may checkpoint with `checkpointWhen`, which stores a full base instead of a delta. This bounds the read tail. See [05-durable-storage.md](./05-durable-storage.md).

### Replication: Monotonic Cursor, No Gaps

A replica attaches to a source with a snapshot and a cursor.

```ts
// packages/chord/src/state.ts -- the cursor contract (verbatim comment)
// Monotonic source cursor. The first frame after a snapshot must be `snapshot.cursor + 1`.
```

A gap is an error.

```ts
// packages/chord/src/state.ts -- the gap guard (verbatim)
if (frame.cursor !== expected) {
  throw new Error(`Replicated state source cursor has a gap: expected ${expected}, received ${frame.cursor}`);
}
```

The Aha is that order is not just assumed. It is enforced. A lost frame throws, which forces a resync from a new snapshot. A CRDT tolerates loss and reordering; chord refuses them.

A slow subscriber is bounded: at most 100 deliveries wait behind the running callback.

```ts
// packages/chord/src/state.ts -- the overflow rule (verbatim comment)
// At most 100 deliveries wait behind the running callback; overflow keeps
// only the newest pending value/context/delivery, so update sequences may skip.
```

Overflow coalesces to the newest value. Intermediate values are skipped. This is read-side coalescing, not a merge.

## What Automerge Does

Automerge is a JSON CRDT. These are its well-known properties.

1. **Any replica edits.** There is no single writer. Concurrent edits from different devices are the normal case.
2. **Every change is immutable and addressed.** A change has a hash. Changes form a hash graph of causal dependencies.
3. **Maps are last-writer-wins registers.** Each property is a register with a Lamport timestamp and an actor ID. A concurrent write to the same key is resolved by the timestamp, then by the actor ID.
4. **Text and lists are an RGA.** Each element has an op ID. An insertion references its predecessor element. Two insertions at the same position are ordered by op ID, deterministically on every replica.
5. **Deletion is a tombstone.** A deleted text element is marked, not removed, so a concurrent edit to it still converges. Garbage collection removes tombstones later.
6. **Merge is a join.** Merging is commutative, associative, and idempotent. The result does not depend on delivery order or duplicates.

The Aha is the difference in guarantees. Automerge's merge is safe under arbitrary order and duplication. chord's apply is not. chord avoids needing that safety by having one writer.

## Where They Diverge

| Axis | chord | Automerge |
|------|-------|-----------|
| Writers | One | Many, concurrent |
| Order | Total, enforced by cursor | Partial, causal |
| Operation identity | None; op is a tuple | Op ID, Lamport clock, actor ID |
| Convergence | From single writer + in-order apply | From commutative, idempotent merge |
| Text edits | `a` append, `t` drop-prefix (positional) | RGA insert by predecessor ID (intent-preserving) |
| Deletion | `d` removes now | Tombstone, retained for convergence |
| Reordering | `m` permutation (O(n)) | Move op (references an element) |
| Delivery loss | Gap is an error | Loss is tolerated (anti-entropy) |
| Duplicate delivery | Corrupts (re-applies) | Harmless (idempotent) |

The deepest divergence is the first row. Everything else follows from it. Because chord has one writer, its ops need no identity. Because they have no identity, they cannot be safely reordered or re-applied. Because they cannot, the cursor must be gap-checked.

## Where It Is Optimal

chord's choice is optimal for its actual job. The job is a server-owned conversation where one process writes and many watchers read.

1. **Simplicity.** No clocks, no actor IDs, no causal graph, no merge routine. An op is a bare tuple. Storage is a base plus an ordered delta tail. The whole thing is a few hundred lines.
2. **Compact deltas for the common edits.** Streaming tool output is an append (`a`). A sliding context window is a drop-prefix plus append (`t` then `a`). These are the most frequent edits, and each is one small op.
3. **Determinism by construction.** There is no conflict resolution to get wrong, because there is no conflict. Replay is a pure fold.
4. **Cheap reads.** A document is a base plus a short tail, bounded by `checkpointWhen`. The immutable apply shares untouched subtrees.
5. **Debuggability.** A linear log is easy to audit and replay. "What changed" is a sequence, not a graph.
6. **Bandwidth.** Path interning, arity omission, and deflate cut the wire size without losing the order that makes them safe.

The Aha is that the "single writer" is not a limitation here. It is the source of every other win. An agent harness owns its conversation. No second device edits it. So the system gives up nothing it needs.

## The Trade-Offs and Issues

These are the honest costs. Each is a direct consequence of removing the CRDT's merge machinery.

1. **No offline or multi-device concurrent editing.** Two clients cannot edit the same document and merge later. If the product ever needs that, chord's model does not extend to it; you would layer a real CRDT on top or change the ownership model.

2. **Re-application is not idempotent.** Applying a batch twice appends twice and truncates twice. Automerge's merge is idempotent, so duplicate delivery is free. chord must guarantee exactly-once by the cursor and the single-writer log. If a replica's cursor drifts, it diverges. The gap guard turns that drift into an error rather than silent corruption, which is the safe choice, but it is a recovery event, not a normal path.

3. **Positional edits assume one history.** `p` splices by index and `m` reorders by permutation. Under any concurrency they corrupt silently. The design avoids this by forbidding concurrency. That is a constraint enforced outside the op algebra, not a property of the ops themselves.

4. **`r` adopts, not copies.** A root replace installs the batch's value by reference.

```ts
// packages/chord/src/delta/index.ts -- the ownership rule (verbatim comment)
// Adopted, not copied. The consumer owns the batch it was handed.
//
// Fanning one batch out to several consumers in-process therefore makes
// their replicas alias each other. That is an ownership rule, not a
// defect: copy the batch at the fan-out point, or let each consumer
// decode its own. A batch that crosses a real boundary is already
// distinct, because serialisation produces fresh objects.
```

Two in-process replicas that share one decoded batch will alias. The code documents this as an ownership rule, not a defect. The safe pattern is to copy at the fan-out point, or let each consumer decode its own. It is a hazard only if you ignore the rule.

5. **The 4096 cap loses the delta advantage exactly when the edit is large.** A big change becomes a full-value replace, O(n) bytes. That is correct and bounded, but it is the opposite of delta-efficient. A CRDT has no such cliff; a large change is many small changes.

6. **`m` is O(n) even for one move.** Reordering an array of n items carries an n-length permutation. Automerge's move references the element, O(1)-ish per move.

7. **Array growth is O(n) in the op.** `arr.length = n` is emitted as a splice of explicit nulls.

```ts
// packages/chord/src/delta/index.ts -- growth stays proportional (verbatim comment)
// Growth stays possible and stays proportional — the tracker already emits
// `arr.length = n` as a splice of explicit nulls, whose op size grows with the
// gap, so a large growth costs a large op rather than a small one.
```

This keeps growth honest, but it means a sparse array extension is as large as the gap.

8. **`t` truncation is irreversible.** Dropping the prefix discards those characters. A rewindable document recovers by keeping per-version bases, not by replaying the delta backward. A CRDT with tombstones keeps deleted text until garbage collection. chord's truncation is a real deletion.

9. **Overflow coalesces and skips.** A subscriber more than 100 deliveries behind loses intermediate values and keeps the newest. That is correct for a view, but it is not a merge; a subscriber that needs every step must not fall behind.

The Aha is that every one of these is the price of "one writer, ordered log." None is a bug. Each is the honest boundary of the model.

## A Worked Example: Concurrent Edits at One Position

Two users start with the string `"ab"`. Both insert at position 1.

Automerge: user A inserts `x` at 1 (after `a`), user B inserts `y` at 1 (after `a`). The inserts reference the same predecessor `a`. RGA orders the two by op ID, identically on every replica. Both converge to `"axyb"` or `"ayxb"`, the same on all replicas.

chord: the concept does not exist. There is one writer. A second editor is not an editor; it is a client that sends its intent to the writer, which serializes it. The string is never edited by two writers at once, so "concurrent insert at one position" is not a state the system can be in.

The Aha is that chord did not answer the concurrency question. It made the question un-askable.

## When to Choose Which

| Requirement | chord-style single-writer | Automerge-style CRDT |
|-------------|---------------------------|----------------------|
| One server owns the document | Fits | Fits |
| Many clients edit offline, then sync | Does not fit | Fits |
| Need to audit a linear history | Fits | Harder (a graph) |
| Need minimal implementation | Fits | Larger |
| Need to tolerate lost/duplicate messages | Must resync | Fits |
| Need streaming append of output | Fits, one small op | Fits but heavier per element |

## The Production Lesson

durable is single-leader replication: one writer, a log, ordered followers. The correct analogy is a primary-replica database or Raft, not a CRDT. For an agent harness that owns its conversations, that is the right model. It is simpler, faster to read, and easy to replay.

If the product later needs two devices editing one conversation offline, the change is architectural, not local. You would replace the delta-and-cursor layer with a real mergeable structure, or put the single writer behind a sync service and treat devices as clients, which is what chord's model already expects.

## Where to Go Next

The delta vocabulary itself is in [02-chord-concepts.md](./02-chord-concepts.md). The storage that persists base-plus-delta is in [05-durable-storage.md](./05-durable-storage.md). The cross-cutting production picture is in [12-cross-cutting.md](./12-cross-cutting.md).
