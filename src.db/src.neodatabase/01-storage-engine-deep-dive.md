---
title: "Neodatabase Storage Engine Deep Dive"
subtitle: "Native graph storage, index-free adjacency, and Neo4j internals"
location: /home/darkvoid/Boxxed/@dev/repo-expolorations/src.db/src.neodatabase
related: 00-zero-to-graph-engineer.md
---

# 01 - Storage Engine Deep Dive: Neodatabase

## Overview

This document covers native graph storage internals - how Neo4j stores nodes and relationships on disk, index-free adjacency for O(1) traversals, and the Neo4j storage architecture.

## Part 1: Native Graph Storage

### Index-Free Adjacency

```
Index-Free Adjacency Explained:

Traditional RDBMS with Indexes:
┌─────────────────────────────────────────────────────────┐
│ Query: Find all friends of Alice                        │
│                                                          │
│ Step 1: Lookup Alice in Users index                     │
│   └─> B-Tree lookup: O(log n)                          │
│                                                          │
│ Step 2: Lookup friendships in Friendships index         │
│   └─> B-Tree lookup: O(log n)                          │
│                                                          │
│ Step 3: JOIN Users table for friend details             │
│   └─> Hash/merge join: O(n log n)                      │
│                                                          │
│ Total: O(log n) per traversal hop                       │
└───────────────────────────────────────────────────────────┘

Neo4j with Index-Free Adjacency:
┌─────────────────────────────────────────────────────────┐
│ Query: Find all friends of Alice                        │
│                                                          │
│ Step 1: Find Alice node (one-time index lookup)         │
│   └─> O(log n) - only at query start                    │
│                                                          │
│ Step 2: Follow relationship pointers                    │
│   └─> Direct pointer dereference: O(1)                 │
│                                                          │
│ Step 3: Access connected node                           │
│   └─> Direct pointer dereference: O(1)                 │
│                                                          │
│ Total: O(1) per traversal hop                           │
└───────────────────────────────────────────────────────────┘

Physical Storage:
- Nodes store direct pointers to relationships
- Relationships store pointers to start/end nodes
- No JOIN computation needed at query time
- Traversal = pointer following
```

### Node Record Structure

```
Neo4j Node Record (Fixed Size):

┌─────────────────────────────────────────────────────────┐
│ Node Record Layout (34 bytes minimum)                   │
├─────────────────────────────────────────────────────────┤
│ Offset  │ Size │ Description                            │
├─────────────────────────────────────────────────────────┤
│ 0       │ 1    │ Record in use flag (1 byte)            │
│ 1       │ 4    │ Next property ID (4 bytes)             │
│ 5       │ 4    │ First relationship ID (4 bytes)        │
│ 9       │ 4    │ Label field (compact label storage)    │
│ 13      │ 8    │ Property chain pointer (8 bytes)       │
│ 21      │ 8    │ Relationship chain pointer (8 bytes)   │
│ 29      │ 4    │ Label count (4 bytes)                  │
│ 33      │ 1    │ Number of labels (1 byte)              │
├─────────────────────────────────────────────────────────┤
│ Total: 34 bytes per node (minimum)                      │
│ + dynamic records for properties and extra labels       │
└───────────────────────────────────────────────────────────┘

Node ID Space:
- 42-bit node IDs: up to 4.4 trillion nodes
- IDs are NOT sequential (gaps from deleted nodes)
- Internal IDs not exposed to applications (best practice)

Label Storage:
- First label stored inline in node record
- Additional labels stored in label scan store
- Compact format: label IDs as 32-bit integers
```

```
Property Storage (Dynamic Records):

┌─────────────────────────────────────────────────────────┐
│ Property Chain Structure                                │
│                                                          │
│ Node Record ──> Property Record 1 ──> Property Record 2 │
│                     │                      │            │
│              ┌──────┴──────┐        ┌──────┴──────┐     │
│              │ Key ID      │        │ Key ID      │     │
│              │ Value (inl) │        │ Value ID    │     │
│              │ Next Prop   │        │ Next Prop   │     │
│              └─────────────┘        └─────────────┘     │
│                                                          │
│ Property Record Layout (41 bytes):                       │
│ ┌─────────────────────────────────────────────────────┐ │
│ │ 1 byte  │ In-use flag                               │ │
│ │ 4 bytes │ Next property ID (or -1 if last)          │ │
│ │ 4 bytes │ Property key ID                           │ │
│ │ 8 bytes │ Value (if fits) or value block ID         │ │
│ │ 1 byte  │ Type tag (string, int, float, bool, etc)  │ │
│ └─────────────────────────────────────────────────────┘ │
│                                                          │
│ Inline values (fit in 8 bytes):                         │
│ - Booleans, bytes, shorts, ints, longs, floats, doubles│
│                                                          │
│ Dynamic values (stored separately):                     │
│ - Strings, arrays, large numbers                        │
│ - Stored in dynamic string/number stores                │
└───────────────────────────────────────────────────────────┘
```

### Relationship Record Structure

```
Relationship Record (Fixed Size):

┌─────────────────────────────────────────────────────────┐
│ Relationship Record Layout (33 bytes)                   │
├─────────────────────────────────────────────────────────┤
│ Offset  │ Size │ Description                            │
├─────────────────────────────────────────────────────────┤
│ 0       │ 1    │ Record in use flag (1 byte)            │
│ 1       │ 4    │ Next relationship ID in chain (4B)     │
│ 5       │ 4    │ Previous relationship ID (4 bytes)     │
│ 9       │ 4    │ Start node ID (4 bytes)                │
│ 13      │ 4    │ End node ID (4 bytes)                  │
│ 17      │ 4    │ Relationship type ID (4 bytes)         │
│ 21      │ 4    │ First property ID (4 bytes)            │
│ 25      │ 1    │ Direction flag (1 byte)                │
│ 26      │ 4    │ Next rel in start node chain (4 bytes) │
│ 30      │ 4    │ Next rel in end node chain (4 bytes)   │
├─────────────────────────────────────────────────────────┤
│ Total: 33 bytes per relationship (minimum)              │
│ + dynamic records for properties                        │
└───────────────────────────────────────────────────────────┘

Doubly-Linked Relationship Chains:

Each node maintains TWO relationship chains:
1. Outgoing relationships (from this node)
2. Incoming relationships (to this node)

Node A -[:FRIENDS_WITH]-> Node B

From Node A's perspective (outgoing):
  A.first_out_rel ──> [Rel: A->B] ──> [Rel: A->C] ──> null

From Node B's perspective (incoming):
  B.first_in_rel ──> [Rel: A->B] ──> [Rel: D->B] ──> null

Benefits:
- O(1) traversal in either direction
- No need to scan all relationships
- Efficient bidirectional pattern matching
```

```
Relationship Group (Optimization):

For nodes with many relationships, Neo4j uses relationship groups:

┌─────────────────────────────────────────────────────────┐
│ High-Degree Node Relationship Storage                   │
│                                                          │
│ Node with 10,000 relationships:                         │
│                                                          │
│ Node Record                                              │
│     │                                                   │
│     v                                                   │
│ ┌──────────────────────────────────────────────────┐   │
│ │ Relationship Group 1 (type: FRIENDS_WITH)        │   │
│ │   first_rel ──> rel1 ──> rel2 ──> ... ──> rel100│   │
│ │   next_group ──────────────────────────────┐     │   │
│ └──────────────────────────────────────────────┼─────┘   │
│                                                │         │
│ ┌──────────────────────────────────────────────┼─────┐   │
│ │ Relationship Group 2 (type: WORKS_AT)        │◄────┘   │
│ │   first_rel ──> rel101 ──> ... ──> rel200   │         │
│ │   next_group ──┐                             │         │
│ └────────────────┼─────────────────────────────┘         │
│                  │                                       │
│ ... (more groups for each relationship type)             │
│                                                          │
│ Benefits:                                                │
│ - Group by relationship type for efficient filtering    │
│ - Avoid scanning irrelevant relationship types          │
│ - Better cache locality for same-type traversals        │
└───────────────────────────────────────────────────────────┘
```

## Part 2: Neo4j Storage Files

### File Structure

```
Neo4j Database Directory:

data/databases/neo4j/
├── neostore.nodestore.db          # Node records
├── neostore.nodestore.db.labels   # Label scan store
├── neostore.relationshipstore.db  # Relationship records
├── neostore.propertystore.db      # Property records
├── neostore.propertystore.db.strings    # String values
├── neostore.propertystore.db.arrays     # Array values
├── neostore.relationshipgrouystore.db   # Relationship groups
├── neostore.schemastore.db        # Schema (indexes, constraints)
├── tx_log.*                       # Transaction log (WAL)
└── meta.db                        # Database metadata

File Formats:
- Fixed-size record stores: Dense arrays of fixed-size records
- Dynamic stores: Variable-length data (strings, arrays)
- All stores are memory-mapped for efficient access
```

```
Record Store Header:

Every Neo4j store file has a header:

┌─────────────────────────────────────────────────────────┐
│ Store File Header (128 bytes)                           │
├─────────────────────────────────────────────────────────┤
│ Offset  │ Size │ Description                            │
├─────────────────────────────────────────────────────────┤
│ 0       │ 9    │ Magic identifier ("NeoStore")          │
│ 9       │ 1    │ Version byte                          │
│ 10      │ 8    │ Creation timestamp (long)              │
│ 18      │ 8    │ Upgrade time (long)                    │
│ 26      │ 8    │ Store version (long)                   │
│ 34      │ 8    │ Log version (long)                     │
│ 42      │ 8    │ Random identifier (long)               │
│ 50      │ 8    │ Current transaction ID (long)          │
│ 58      │ 8    │ Time of current transaction (long)     │
│ 66      │ 62   │ Reserved for future use                │
├─────────────────────────────────────────────────────────┤
│ Total Header: 128 bytes                                 │
└───────────────────────────────────────────────────────────┘

Record Access:
- Header is read once at startup
- Records accessed via memory-mapped I/O
- Record at offset N: header_size + (N * record_size)
```

### Transaction Log (WAL)

```
Transaction Log Format:

┌─────────────────────────────────────────────────────────┐
│ Transaction Log Entry Structure                         │
├─────────────────────────────────────────────────────────┤
│ ┌─────────────────────────────────────────────────────┐ │
│ │ TX Log Entry Header (16 bytes)                      │ │
│ │ - 4 bytes: Magic (0xBEEF)                          │ │
│ │ - 4 bytes: Entry size                               │ │
│ │ - 8 bytes: Transaction ID                           │ │
│ └─────────────────────────────────────────────────────┘ │
│ ┌─────────────────────────────────────────────────────┐ │
│ │ Transaction Commands (variable)                     │ │
│ │ - Command type (1 byte)                             │ │
│ │ - Command-specific data                             │ │
│ │   - Node CREATE: node ID, label IDs, property data  │ │
│ │   - Rel CREATE: rel ID, type, start/end node IDs   │ │
│ │   - Property SET: prop ID, key, value, type        │ │
│ └─────────────────────────────────────────────────────┘ │
│ ┌─────────────────────────────────────────────────────┐ │
│ │ Checksum (8 bytes)                                  │ │
│ │ - CRC32C of entire entry                            │ │
│ └─────────────────────────────────────────────────────┘ │
└───────────────────────────────────────────────────────────┘

Transaction Commands:
- CREATE_NODE: Allocate node ID, set initial properties
- CREATE_REL: Allocate rel ID, link to nodes
- SET_PROPERTY: Create/update property record
- REMOVE_PROPERTY: Mark property as deleted
- ADD_LABEL: Add label to node
- REMOVE_LABEL: Remove label from node
- DELETE_NODE: Mark node record as free
- DELETE_REL: Mark rel record as free

Write-Ahead Protocol:
1. Write transaction to WAL first
2. Sync WAL to disk (fsync)
3. Apply changes to store files
4. Mark transaction as committed
```

```
Checkpoint Mechanism:

Neo4j creates checkpoints to limit recovery time:

┌─────────────────────────────────────────────────────────┐
│ Checkpoint Process                                      │
│                                                          │
│ 1. Flush all dirty pages from page cache                │
│ 2. Write checkpoint record to WAL                       │
│ 3. Record checkpoint position in metadata               │
│                                                          │
│ Checkpoint Record Format:                                │
│ ┌─────────────────────────────────────────────────────┐ │
│ │ CHECKPOINT                                          │ │
│ │   - Timestamp                                       │ │
│ │   - WAL position (log version, offset)              │ │
│ │   - Oldest transaction ID still active              │ │
│ └─────────────────────────────────────────────────────┘ │
│                                                          │
│ Recovery after crash:                                   │
│ 1. Find last checkpoint                                 │
│ 2. Scan WAL from checkpoint position                    │
│ 3. Redo committed transactions                          │
│ 4. Undo uncommitted transactions                        │
│                                                          │
│ Checkpoint frequency: Configurable (default: 15 min)    │
│ Recovery time: Proportional to WAL size since checkpoint│
└───────────────────────────────────────────────────────────┘
```

### Page Cache

```
Neo4j Page Cache:

┌─────────────────────────────────────────────────────────┐
│ Page Cache Architecture                                 │
│                                                          │
│ ┌─────────────────────────────────────────────────────┐ │
│ │ Page Cache (configurable size, e.g., 8GB)           │ │
│ │ ┌─────────┬─────────┬─────────┬─────────┐           │ │
│ │ │ Page 0  │ Page 1  │ Page 2  │ ...     │           │ │
│ │ │ 8192 B  │ 8192 B  │ 8192 B  │         │           │ │
│ │ └─────────┴─────────┴─────────┴─────────┘           │ │
│ └─────────────────────────────────────────────────────┘ │
│                         │                               │
│              ┌──────────┴──────────┐                    │
│              ▼                     ▼                    │
│       ┌────────────┐       ┌────────────┐              │
│       │ Node Store │       │ Rel Store  │              │
│       │ (mmap'd)   │       │ (mmap'd)   │              │
│       └────────────┘       └────────────┘              │
│                                                          │
│ Page Size: 8192 bytes (8KB)                             │
│ Eviction Policy: Clock algorithm (approximate LRU)      │
│ Flushing: Background writer thread + checkpoint flush   │
│                                                          │
│ Configuration:                                           │
│ - dbms.memory.pagecache.size: Total cache size          │
│ - dbms.memory.pagecache.warmup.enable: true/false       │
└───────────────────────────────────────────────────────────┘

Page Structure (8KB):
┌─────────────────────────────────────────────────────────┐
│ Page Header (32 bytes)                                  │
│ - Flags, LSN, checksum                                  │
├─────────────────────────────────────────────────────────┤
│ Record Data (variable)                                  │
│ - Fixed-size records packed densely                     │
│ - Free space bitmap for dynamic allocation              │
├─────────────────────────────────────────────────────────┤
│ Page Footer (32 bytes)                                  │
│ - Checksum, next page pointer (for overflow)            │
└───────────────────────────────────────────────────────────┘
```

## Part 3: Indexing

### Label Scan Store

```
Label Scan Store:

Before Neo4j 5.x: Separate index per label
Neo4j 5.x+: Unified label scan store

┌─────────────────────────────────────────────────────────┐
│ Label Scan Store Structure                              │
│                                                          │
│ For each label, store node ID ranges:                   │
│                                                          │
│ Label:Person                                             │
│ ┌─────────────────────────────────────────────────────┐ │
│ │ Node ID Ranges:                                     │ │
│ │ [1-100], [150-200], [250-300], ...                  │ │
│ │ (compact representation of which nodes have label)  │ │
│ └─────────────────────────────────────────────────────┘ │
│                                                          │
│ Label:Company                                            │
│ ┌─────────────────────────────────────────────────────┐ │
│ │ Node ID Ranges:                                     │ │
│ │ [5-5], [50-55], [1000-1005], ...                    │ │
│ └─────────────────────────────────────────────────────┘ │
│                                                          │
│ Benefits:                                                │
│ - Efficient label-based scans                           │
│ - Compact storage (ranges, not individual IDs)          │
│ - Fast label membership checks                          │
└───────────────────────────────────────────────────────────┘
```

### Index Types

```
B-Tree Index (Neo4j 5.x):

┌─────────────────────────────────────────────────────────┐
│ B-Tree Index Structure                                  │
│                                                          │
│ Root Node                                                │
│ ┌─────────────────────────────────────────────────────┐ │
│ │ Keys: [100, 500, 1000]                              │ │
│ │ Pointers: [child1, child2, child3, child4]          │ │
│ └─────────────────────────────────────────────────────┘ │
│         │              │              │                 │
│         v              v              v                 │
│ ┌─────────────┐ ┌─────────────┐ ┌─────────────┐        │
│ │ Leaf: 1-99  │ │Leaf:100-499 │ │Leaf:500-999 │        │
│ │ ┌─────────┐ │ │ ┌─────────┐ │ │ ┌─────────┐ │        │
│ │ │key:ptr  │ │ │ │key:ptr  │ │ │ │key:ptr  │ │        │
│ │ │alice:1  │ │ │ │bob:100  │ │ │ │carol:500│ │        │
│ │ │...      │ │ │ │...      │ │ │ │...      │ │        │
│ │ └─────────┘ │ │ └─────────┘ │ │ └─────────┘ │        │
│ └─────────────┘ └─────────────┘ └─────────────┘        │
│                                                          │
│ Lookup Complexity: O(log n)                             │
│ Best for: Equality and range queries                    │
│ Supported types: Strings, numbers, booleans, temporal   │
└───────────────────────────────────────────────────────────┘

Range Index:
CREATE INDEX person_name_idx FOR (p:Person) ON (p.name)

Composite Index (multi-property):
CREATE INDEX person_name_age_idx FOR (p:Person) ON (p.name, p.age)
```

```
Full-Text Index:

┌─────────────────────────────────────────────────────────┐
│ Full-Text Index (Apache Lucene)                         │
│                                                          │
│ Inverted Index Structure:                               │
│                                                          │
│ Term: "alice" ──> [Node ID: 1, Node ID: 50, ...]        │
│ Term: "bob"   ──> [Node ID: 2, Node ID: 100, ...]       │
│ Term: "john"  ──> [Node ID: 5, Node ID: 75, ...]        │
│                                                          │
│ Features:                                                │
│ - Tokenization (split on whitespace, punctuation)       │
│ - Lowercasing                                           │
│ - Stop word removal                                     │
│ - Stemming (optional)                                   │
│ - Fuzzy matching (Levenshtein distance)                 │
│ - Prefix matching                                       │
│                                                          │
│ Create:                                                  │
│ CREATE FULLTEXT INDEX person_name_ft                    │
│ FOR (p:Person) ON EACH [p.name]                         │
│                                                          │
│ Query:                                                   │
│ CALL db.index.fulltext.queryNodes(                      │
│   "person_name_ft", "alice OR bob"                      │
│ ) YIELD node, score                                     │
│ RETURN node.name, score                                 │
└───────────────────────────────────────────────────────────┘

Vector Index (Neo4j 5.x+):
- For similarity search (embeddings, ML vectors)
- HNSW (Hierarchical Navigable Small World) algorithm
- Approximate nearest neighbor search

CREATE VECTOR INDEX product_embedding_idx
FOR (p:Product) ON (p.embedding)
OPTIONS {indexConfig: {
  `vector.dimensions`: 768,
  `vector.similarity_function`: 'cosine'
}}
```

### Index Selection

```
Index Usage Patterns:

┌─────────────────────────────────────────────────────────┐
│ Query Pattern              │ Index Used                 │
├─────────────────────────────────────────────────────────┤
│ p.name = "Alice"           │ B-tree on :Person(name)    │
│ p.age > 25 AND p.age < 40  │ Range index on :Person(age)│
│ p.name STARTS WITH "A"     │ B-tree (range scan)        │
│ p.name CONTAINS "li"       │ Full-text index            │
│ p.name =~ "A.*e"           │ Full-text or scan          │
│ p.age IN [25, 30, 35]      │ B-tree (multiple lookups)  │
│ (p)-[:FRIENDS_OF]->()      │ No index (pointer chase)   │
│ (p)-[*1..5]->()            │ No index (traversal)       │
└───────────────────────────────────────────────────────────┘

Query Plan Inspection:

EXPLAIN MATCH (p:Person {name: "Alice"})
RETURN p.name, p.age

Query Plan:
┌─────────────────────────────────────────────────────────┐
│ +ProduceResults                                         │
│ │                                                       │
│ +Project                                                │
│ │                                                       │
│ +NodeUniqueIndexSeek                                    │
│   │ Index: person_name_idx                              │
│   │ Lookup: p:Person(name = "Alice")                    │
│   │ Estimated rows: 1                                   │
└───────────────────────────────────────────────────────────┘

Without index (label scan):
┌─────────────────────────────────────────────────────────┐
│ +ProduceResults                                         │
│ │                                                       │
│ +Project                                                │
│ │                                                       │
│ +NodeByLabelScan                                        │
│   │ Label: Person                                       │
│   │ Estimated rows: 1000000 (all Person nodes)          │
└───────────────────────────────────────────────────────────┘
```

## Part 4: Storage Optimization

### Compression

```
Property Compression:

Neo4j compresses property values:

String Compression:
- Short strings (< 50 chars): Stored inline, no compression
- Long strings: ZSTD compression applied
- Common prefixes: Prefix compression in dynamic string store

Integer Compression:
- Small integers (1 byte): -128 to 127
- Medium integers (2 bytes): -32768 to 32767
- Large integers (4-8 bytes): Full range

Array Compression:
- Uniform arrays: Store element type once
- Delta encoding for sorted arrays
- Run-length encoding for repeated values

Configuration:
- dbms.memory.compression.zstd.level: 1-9 (default: 3)
```

### Defragmentation

```
Store Defragmentation:

Over time, deletes create gaps in record stores:

Before Defragmentation:
┌─────────────────────────────────────────────────────────┐
│ Node Store: [A][B][gap][C][gap][gap][D][E][gap]...     │
│                                                          │
│ Free space ratio: 33% (3 gaps out of 9 slots)           │
│ Scan efficiency: Must skip gaps                         │
└───────────────────────────────────────────────────────────┘

After Defragmentation:
┌─────────────────────────────────────────────────────────┐
│ Node Store: [A][B][C][D][E][gap][gap][gap][gap]...     │
│                                                          │
│ Free space ratio: Compacted to end                      │
│ Scan efficiency: Sequential until free space            │
└───────────────────────────────────────────────────────────┘

Manual Defragmentation:
CALL db.resampleIndex()  -- Update index statistics
CALL db.checkpoint()     -- Force checkpoint

Automatic:
- Neo4j reuses freed record IDs for new records
- No manual defragmentation typically needed
```

---

*This document is part of the Neodatabase exploration series. See [exploration.md](./exploration.md) for the complete index.*
