# Versioned Storage Deep Dive

## Overview

This document explains how Dolt stores versioned tables at the storage layer. Understanding this is crucial for anyone wanting to reproduce Dolt's functionality in Rust or another language.

## Storage Stack

```
┌─────────────────────────────────────┐
│         SQL Layer (go-mysql-server) │
├─────────────────────────────────────┤
│      Version Control (doltcore)     │
├─────────────────────────────────────┤
│     Prolly Trees (B+ Tree)          │
├─────────────────────────────────────┤
│    NBS (Noms Block Store)           │
└─────────────────────────────────────┘
```

## 1. NBS - Noms Block Store

### Content-Addressed Storage

NBS is a content-addressed object store where:
- Each chunk is addressed by its SHA-1 hash
- Chunks are immutable (append-only)
- Duplication is automatically detected

```go
// Chunk identification
type hash [20]byte  // SHA-1 hash

type Chunk interface {
    Hash() hash.Hash
    Data() []byte
}
```

### Storage Format

NBS stores data in table files:

```
Table File Structure:
┌─────────────────────┐
│  Chunk Block        │  -- Compressed chunk data
├─────────────────────┤
│  Index Block        │  -- Hash → offset mappings
├─────────────────────┤
│  Footer             │  -- Metadata, checksums
└─────────────────────┘
```

### Key Properties

1. **Immutability** - Once written, chunks never change
2. **Deduplication** - Same content = same hash = same chunk
3. **Concurrency** - Multiple writers with optimistic locking
4. **Garbage Collection** - Unreferenced chunks can be removed

### Backends

NBS supports two backends:

**Local Disk:**
```
.dolt/noms/
├── manifests            -- Repository manifest
├── root_abc123          -- Root hash file
└── table_*.nbs          -- Table files
```

**AWS (S3 + DynamoDB):**
- Chunks stored in S3
- Manifest in DynamoDB
- "Effectively CA" consistency

## 2. Prolly Trees

### What is a Prolly Tree?

Prolly trees are probabilistically balanced B+ trees optimized for:
- Range queries
- Point lookups
- Three-way diff/merge
- Efficient serialization

### Tree Structure

```
Prolly Tree:
                    ┌───────────────┐
                    │    Root Node  │
                    │  [entries...] │
                    └───────┬───────┘
                            │
         ┌──────────────────┼──────────────────┐
         ▼                  ▼                  ▼
┌───────────────┐  ┌───────────────┐  ┌───────────────┐
│ Internal Node │  │ Internal Node │  │ Internal Node │
└───────┬───────┘  └───────┬───────┘  └───────┬───────┘
        │                  │                  │
    ┌───┴───┐          ┌───┴───┐          ┌───┴───┐
    ▼       ▼          ▼       ▼          ▼       ▼
┌───────┐ ┌───────┐ ┌───────┐ ┌───────┐ ┌───────┐ ┌───────┐
│ Leaf  │ │ Leaf  │ │ Leaf  │ │ Leaf  │ │ Leaf  │ │ Leaf  │
│ [k,v] │ │ [k,v] │ │ [k,v] │ │ [k,v] │ │ [k,v] │ │ [k,v] │
└───────┘ └───────┘ └───────┘ └───────┘ └───────┘ └───────┘
```

### Node Structure

Each node contains:
- **Entries** - Key-value pairs (internal) or key-data (leaf)
- **Count** - Number of entries
- **Level** - Distance from leaves

```go
type Node struct {
    entries []entry
    level   uint8
}

type entry struct {
    key   Item    // Sortable key
    value Item    // Value (or child pointer)
}
```

### Chunking Algorithm

Prolly trees use content-defined chunking:

```
Input: sorted key-value pairs
Output: tree of chunks

Algorithm:
1. Sort entries by key
2. Roll hash over entries
3. Create chunk boundary when hash meets threshold
4. Recursively build tree
```

Benefits:
- Similar data → similar chunk boundaries
- Efficient diffs (only changed chunks differ)
- Natural merge optimization

## 3. Map Implementation

### Map Types

Dolt uses several map types:

```go
// Immutable map
type Map struct {
    tuples  tree.StaticMap
    keyDesc *val.TupleDesc
    valDesc *val.TupleDesc
}

// Mutable map
type MutableMap struct {
    base    Map
    edits   *buffer
}
```

### Key-Value Storage

Maps store typed tuples:

```
Key Tuple: (column1, column2, ...)
Value Tuple: (column3, column4, ...)

Example (Primary Key: id, name):
Key:   (1, "Alice")
Value: (25, "Engineering", 50000)
```

### Operations

```go
// Get value by key
func (m Map) Get(ctx, key) (val.Tuple, bool)

// Iterate over range
func (m Map) IterRange(ctx, rng Range) MapIter

// Diff two maps
func DiffMaps(ctx, from, to Map, cb DiffFn)
```

## 4. Table Storage

### Table Structure

Each table is stored as:

```
Table
├── schema: Schema            -- Column definitions
├── row_data: ProllyMap       -- Primary index (keyed by PK)
├── indexes: []ProllyMap      -- Secondary indexes
└── artifacts: ProllyMap      -- Conflicts, violations
```

### Schema Storage

Schemas are stored as FlatBuffers:

```
Schema (FlatBuffers)
├── name: string
├── columns: []Column
│   ├── name: string
│   ├── type: Type
│   ├── nullable: bool
│   └── tags: []uint64
├── primary_key: []uint64    -- Column indexes
└── collation: Collation
```

### Row Format

Rows stored as serialized tuples:

```
Row Message (FlatBuffers)
├── tuple_type: byte
├── data: []byte             -- Column values
└── offsets: []uint32        -- Variable-length offsets
```

## 5. Version Control

### RootValue

The root of database state:

```
RootValue (FlatBuffers)
├── tables: map<Name, Hash>   -- Table name → table hash
├── foreign_keys: fkcHash     -- Foreign key collection
├── feature_version: int64    -- Compatibility version
├── collation: Collation      -- Database collation
└── root_objects: map         -- Additional objects
```

### Commit Structure

```
Commit
├── root: RootValue Hash      -- Points to RootValue
├── parents: []Commit Hash    -- Parent commits
├── metadata: CommitMeta
│   ├── name: string          -- Committer name
│   ├── email: string         -- Committer email
│   ├── message: string       -- Commit message
│   └── timestamp: int64      -- Commit time
└── height: uint64            -- Tree height
```

### Branch References

Branches are lightweight pointers:

```
.dolt/refs/heads/main → commit_hash
.dolt/refs/heads/dev  → commit_hash
```

## 6. Diff Algorithm

### Three-Way Diff

Dolt's diff algorithm:

```
Given:
- ancestor (common base)
- ours (current branch)
- theirs (branch to merge)

For each key in the maps:
1. Compare ancestor→ours
2. Compare ancestor→theirs
3. Classify change:
   - No change: same in all three
   - Simple change: changed in one branch
   - Conflict: changed differently in both
```

### Diff Types

```go
type DiffType int
const (
    Added    DiffType = iota  // Key only in 'to'
    Removed                   // Key only in 'from'
    Modified                  // Key in both, value changed
)

type Diff struct {
    Type      DiffType
    Key       Tuple
    FromValue Tuple  // nil if added
    ToValue   Tuple  // nil if removed
}
```

### Efficient Diffing

Prolly trees enable efficient diffs:

1. **Hash Comparison** - If node hashes equal, subtrees equal
2. **Skip Unchanged** - Skip entire unchanged subtrees
3. **Range Diffs** - Diff only specified key ranges

```go
// Skip unchanged subtrees
if from.Hash() == to.Hash() {
    return nil  // No changes in this subtree
}
```

## 7. Merge Algorithm

### Three-Way Merge

```go
func MergeRoots(ctx, ourRoot, theirRoot, ancRoot RootValue) (*Result, error)
```

Process:

1. **Find changed tables**
   ```go
   // Compare table hashes
   ourTableHash := ourRoot.GetTableHash("users")
   ancTableHash := ancRoot.GetTableHash("users")
   theirTableHash := theirRoot.GetTableHash("users")
   ```

2. **For each changed table:**
   ```go
   // Get row data
   ourRows := ourTable.GetRowData(ctx)
   theirRows := theirTable.GetRowData(ctx)
   ancRows := ancTable.GetRowData(ctx)

   // Compute diffs
   DiffMaps(ancRows, ourRows, ourDiff)
   DiffMaps(ancRows, theirRows, theirDiff)
   ```

3. **Apply non-conflicting changes**
   ```go
   // Our change only
   if ourDiff && !theirDiff {
       apply(ourDiff)
   }
   // Their change only
   if !ourDiff && theirDiff {
       apply(theirDiff)
   }
   // Both changed - conflict!
   if ourDiff && theirDiff && ourDiff != theirDiff {
       recordConflict(diff)
   }
   ```

4. **Merge schemas**
   ```go
   mergedSchema, conflicts := mergeSchemas(ourSchema, theirSchema, ancSchema)
   ```

### Conflict Types

```go
type Conflict struct {
    Table     TableName
    Kind      ConflictKind
    Ancestor  Row
    Ours      Row
    Theirs    Row
}

type ConflictKind int
const (
    CellConflict      // Same cell modified differently
    RowConflict       // Row modified in incompatible ways
    SchemaConflict    // Schema changes incompatible
)
```

### Conflict Storage

Conflicts stored in artifact maps:

```
Artifact Key:   (table_id, conflict_id, type)
Artifact Value: (ancestor_hash, ours_hash, theirs_hash)
```

Query via system table:
```sql
SELECT * FROM dolt_conflicts_users;
```

## 8. Garbage Collection

### Reference Tracing

GC finds unreachable chunks:

1. **Start from roots** - All branch heads, working sets
2. **Trace references** - Follow all chunk hashes
3. **Mark reachable** - Build set of reachable chunks
4. **Sweep unreferenced** - Remove chunks not in set

### GC Process

```go
func GarbageCollect(ctx *DoltDB,保留Period time.Duration) error {
    // 1. Find all reachable chunks
    reachable := traceReferences(ctx)

    // 2. Find old unreferenced table files
    oldFiles := findOldTableFiles(reachable)

    // 3. Delete unreferenced files
    for _, file := range oldFiles {
        deleteTableFile(file)
    }
}
```

## 9. Performance Optimizations

### 1. Chunk Caching

```go
type ChunkCache interface {
    Get(hash.Hash) (Chunk, bool)
    Put(Chunk)
}
```

LRU cache for frequently accessed chunks.

### 2. Batched Writes

Buffer edits before flushing:
```go
editor := map.Editor()
for _, edit := range edits {
    editor.Set(edit.key, edit.value)
}
map, err := editor.Flush(ctx)
```

### 3. Incremental Flush

For large operations, flush incrementally:
```go
for batch := range batches {
    chunk := writeChunk(batch)
    if chunk.Size() > threshold {
        flushToStorage(chunk)
    }
}
```

### 4. Parallel Operations

```go
// Parallel table loading
for _, tableName := range tables {
    go func(name string) {
        table, _ := root.GetTable(ctx, name)
        results <- table
    }(tableName)
}
```

## 10. Serialization Format

### FlatBuffers

Dolt uses FlatBuffers for:
- Schema storage
- Row serialization
- RootValue structure
- Commit metadata

Benefits:
- Zero-copy deserialization
- Fast random access
- Compact storage

### Example Schema

```
// FlatBuffers schema
table Column {
    name: string;
    type: string;
    nullable: bool;
    tags: [ulong];
}

table Schema {
    name: string;
    columns: [Column];
    primary_key: [ulong];
    collation: Collation;
}
```

## 11. Feature Versioning

To ensure compatibility:

```go
// Feature version in each RootValue
const DoltFeatureVersion = 7

// Clients reject newer versions
if persistedVersion > clientVersion {
    return fmt.Errorf("incompatible version: %d > %d",
                      persistedVersion, clientVersion)
}
```

## Key Files for Implementation

| File | Purpose | Size |
|------|---------|------|
| `store/nbs/*.go` | NBS storage | ~50 files |
| `store/prolly/*.go` | Prolly trees | ~30 files |
| `doltdb/root_val.go` | Root value | 44KB |
| `doltdb/table.go` | Table operations | 13KB |
| `merge/merge.go` | Merge orchestration | 14KB |
| `merge/merge_prolly_rows.go` | Row merging | 76KB |

## Rust Implementation Checklist

### Storage Layer
- [ ] Content-addressed chunk store
- [ ] SHA-1 hashing
- [ ] Table file format
- [ ] Manifest tracking

### Prolly Trees
- [ ] B+ tree implementation
- [ ] Content-defined chunking
- [ ] Range queries
- [ ] Diff algorithm

### Serialization
- [ ] FlatBuffers integration
- [ ] Tuple encoding
- [ ] Schema storage

### Version Control
- [ ] Commit graph
- [ ] Branch references
- [ ] Three-way merge
- [ ] Conflict detection

### SQL Integration
- [ ] System tables
- [ ] Row iteration
- [ ] Schema enforcement

## References

- [NBS README](../../src.dolthub/dolt/go/store/nbs/README.md)
- [Prolly Trees](../../src.dolthub/dolt/go/store/prolly/)
- [Feature Versioning](../../src.dolthub/dolt/go/libraries/doltcore/doltdb/feature_version.md)
