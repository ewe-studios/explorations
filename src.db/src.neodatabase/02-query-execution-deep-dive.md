---
title: "Neodatabase Query Execution Deep Dive"
subtitle: "Cypher compilation, pattern matching, and graph traversal algorithms"
location: /home/darkvoid/Boxxed/@dev/repo-expolorations/src.db/src.neodatabase
related: 01-storage-engine-deep-dive.md
---

# 02 - Query Execution Deep Dive: Neodatabase

## Overview

This document covers Cypher query execution - how Neo4j compiles and executes graph queries, pattern matching algorithms, and traversal optimization.

## Part 1: Cypher Compilation

### Query Pipeline

```
Cypher Query Execution Pipeline:

┌─────────────────────────────────────────────────────────┐
│ 1. Parsing                                               │
│    Cypher text ──> AST (Abstract Syntax Tree)           │
│                                                          │
│    MATCH (p:Person {name: "Alice"})                      │
│    RETURN p.name, p.age                                 │
│                                                          │
│    AST:                                                  │
│    └─ Match                                             │
│       └─ Pattern: (p:Person {name: "Alice"})            │
│       └─ Return: p.name, p.age                          │
└───────────────────────────────────────────────────────────┘
                         │
                         ▼
┌─────────────────────────────────────────────────────────┐
│ 2. Semantic Analysis                                     │
│    AST ──> Resolved Query (symbol table, type check)    │
│                                                          │
│    - Resolve labels, property keys, relationship types  │
│    - Type checking (property types, function args)      │
│    - Variable scoping                                   │
└───────────────────────────────────────────────────────────┘
                         │
                         ▼
┌─────────────────────────────────────────────────────────┐
│ 3. Logical Planning                                      │
│    Resolved Query ──> Logical Plan (operator tree)      │
│                                                          │
│    Logical Plan:                                         │
│    +ProduceResults                                      │
│    │                                                    │
│    +Project                                             │
│    │                                                    │
│    +NodeIndexSeek                                       │
│      Index: person_name_idx                             │
│      Lookup: p:Person(name = "Alice")                   │
└───────────────────────────────────────────────────────────┘
                         │
                         ▼
┌─────────────────────────────────────────────────────────┐
│ 4. Logical Optimization (Cost-Based)                     │
│    Logical Plan ──> Optimized Logical Plan              │
│                                                          │
│    Optimizations:                                        │
│    - Predicate pushdown                                 │
│    - Join reordering                                    │
│    - Path pruning                                       │
│    - Redundant operator elimination                     │
│                                                          │
│    Example: Move filter before expand                   │
│    Before: Expand ──> Filter                            │
│    After:  Filter ──> Expand (fewer rows to expand)     │
└───────────────────────────────────────────────────────────┘
                         │
                         ▼
┌─────────────────────────────────────────────────────────┐
│ 5. Physical Planning                                     │
│    Logical Plan ──> Physical Plan (executable ops)      │
│                                                          │
│    Physical operators:                                  │
│    - NodeIndexSeek (uses index)                         │
│    - NodeLabelScan (scans all nodes with label)         │
│    - Expand (traverse relationships)                    │
│    - Filter (apply predicates)                          │
│    - HashJoin, NestedLoopJoin                           │
│    - Aggregate, Sort, Limit                             │
└───────────────────────────────────────────────────────────┘
                         │
                         ▼
┌─────────────────────────────────────────────────────────┐
│ 6. Execution                                               │
│    Physical Plan ──> Result                             │
│                                                          │
│    - Pipelined execution (operators pull from children) │
│    - Vectorized processing (batches of rows)            │
│    - Parallel execution (when possible)                 │
└───────────────────────────────────────────────────────────┘
```

### Query Plan Operators

```
Common Physical Operators:

┌─────────────────────────────────────────────────────────┐
│ Operator              │ Description                     │
├─────────────────────────────────────────────────────────┤
│ NodeIndexSeek        │ Lookup nodes using index         │
│ NodeLabelScan        │ Scan all nodes with label        │
│ Expand(All)          │ Traverse all relationships       │
│ Expand(Into)         │ Traverse to specific node        │
│ Expand(Variable)     │ Variable-length traversal        │
│ Filter               │ Apply WHERE predicates           │
│ HashJoin             │ Hash-based join                  │
│ NestedLoopJoin       │ Nested loop join                 │
│ Apply                │ Correlated subquery              │
│ LeftApply            │ Correlated optional match        │
│ Aggregate            │ GROUP BY aggregation             │
│ Sort                 │ ORDER BY sorting                 │
│ Limit                │ LIMIT/SKIP                       │
│ Distinct             │ DISTINCT deduplication           │
│ Union                │ UNION (with dedup)               │
│ UnionAll             │ UNION ALL (no dedup)             │
└───────────────────────────────────────────────────────────┘

Example Query Plan:

MATCH (p:Person {name: "Alice"})-[:FRIENDS_OF]->(friend)
WHERE friend.age > 25
RETURN friend.name
ORDER BY friend.name
LIMIT 10

Plan:
┌─────────────────────────────────────────────────────────┐
│ +Limit(10)                                              │
│ │                                                       │
│ +Sort                                                 │
│ │ orderBy: friend.name ASC                              │
│ │                                                       │
│ +Project                                                │
│ │ friend.name                                           │
│ │                                                       │
│ +Filter                                                 │
│ │ friend.age > 25                                       │
│ │                                                       │
│ +Expand(All)                                            │
│ │ :FRIENDS_OF                                           │
│ │                                                       │
│ +NodeUniqueIndexSeek                                    │
│   :Person(name = "Alice")                               │
└───────────────────────────────────────────────────────────┘
```

## Part 2: Pattern Matching

### Single-Hop Pattern

```
Simple Relationship Traversal:

Query:
MATCH (p:Person {name: "Alice"})-[:FRIENDS_OF]->(friend)
RETURN friend.name

Execution Steps:
┌─────────────────────────────────────────────────────────┐
│ Step 1: Index Lookup                                    │
│   - Look up "Alice" in person_name_idx                  │
│   - O(log n) using B-tree                               │
│   - Result: Node ID 42                                  │
└───────────────────────────────────────────────────────────┘
                         │
                         ▼
┌─────────────────────────────────────────────────────────┐
│ Step 2: Relationship Traversal                          │
│   - Load node record for ID 42                          │
│   - Read first_out_rel pointer                          │
│   - Follow relationship chain:                          │
│     ┌──────────┐    ┌──────────┐    ┌──────────┐       │
│     │ Rel: 100 │───>│ Rel: 101 │───>│ Rel: 102 │───>   │
│     │ type=FO  │    │ type=FO  │    │ type=WA  │        │
│     │ end=50   │    │ end=75   │    │ end=200  │        │
│     └──────────┘    └──────────┘    └──────────┘       │
│   - Filter by relationship type (:FRIENDS_OF)           │
│   - Collect end node IDs: [50, 75]                      │
│   - O(k) where k = number of relationships              │
└───────────────────────────────────────────────────────────┘
                         │
                         ▼
┌─────────────────────────────────────────────────────────┐
│ Step 3: Load Friend Nodes                               │
│   - Load node records for IDs 50, 75                    │
│   - Read name property from each                        │
│   - Result: ["Bob", "Carol"]                            │
└───────────────────────────────────────────────────────────┘

Pointer Chasing (O(1) per hop):
- Each relationship is a direct pointer
- No JOIN computation
- Cache locality matters (prefetching helps)
```

### Multi-Hop Pattern

```
Friends of Friends (2 hops):

Query:
MATCH (p:Person {name: "Alice"})-[:FRIENDS_OF*2]->(fof)
RETURN DISTINCT fof.name

Execution:
┌─────────────────────────────────────────────────────────┐
│ Iteration 1: Direct Friends                             │
│   - Start: Alice (ID: 42)                               │
│   - Traverse FRIENDS_OF relationships                   │
│   - Found: Bob (50), Carol (75), Dave (100)             │
│   - Frontier: [50, 75, 100]                             │
└───────────────────────────────────────────────────────────┘
                         │
                         ▼
┌─────────────────────────────────────────────────────────┐
│ Iteration 2: Friends of Friends                         │
│   - For each node in frontier:                          │
│     - Bob (50): Friends → Carol (75), Eve (125)         │
│     - Carol (75): Friends → Alice (42), Frank (150)     │
│     - Dave (100): Friends → Bob (50), Grace (175)       │
│   - All FOFs: [75, 125, 42, 150, 50, 175]               │
│   - Exclude start (Alice): [75, 125, 150, 50, 175]      │
│   - Distinct: [Bob, Carol, Eve, Frank, Grace]           │
└───────────────────────────────────────────────────────────┘

Breadth-First Search (BFS):
- Neo4j uses BFS for variable-length patterns
- Ensures shortest path found first
- Visited set prevents cycles

Complexity: O(b^d) where b=branching factor, d=depth
```

```
Variable-Length with Path Filtering:

Query:
MATCH path = (p:Person {name: "Alice"})-[:FRIENDS_OF*1..3]->(target)
WHERE ALL(r IN relationships(path) WHERE r.since > date("2020-01-01"))
RETURN target.name, length(path) AS degrees

Execution with Path Tracking:
┌─────────────────────────────────────────────────────────┐
│ BFS with Path Accumulation                              │
│                                                          │
│ Queue: [(Alice, path=[], depth=0)]                      │
│                                                          │
│ While queue not empty:                                  │
│   (current, path, depth) = dequeue()                    │
│                                                          │
│   For each relationship r from current:                 │
│     If r.type == FRIENDS_OF AND r.since > 2020-01-01:   │
│       new_path = path + [r]                             │
│       next_node = r.other_end(current)                  │
│       If depth < 3:                                     │
│         enqueue(next_node, new_path, depth+1)           │
│       If depth >= 1:                                    │
│         emit(next_node, new_path, depth)                │
└───────────────────────────────────────────────────────────┘

Path Representation:
- Store as list of relationship IDs (compact)
- Node IDs can be derived from rel endpoints
- functions(path) and nodes(path) reconstruct on demand
```

### Bidirectional Search

```
Bidirectional Pattern Matching:

Query: Find connection between Alice and Bob
MATCH path = shortestPath(
  (alice:Person {name: "Alice"})-[:FRIENDS_OF*]-(bob:Person {name: "Bob"})
)
RETURN path

Bidirectional BFS:
┌─────────────────────────────────────────────────────────┐
│ Forward Search (from Alice)    │ Backward Search (Bob)  │
│ ─────────────────────────────  │ ─────────────────────  │
│ Frontier: [Alice]              │ Frontier: [Bob]        │
│ Visited: {Alice}               │ Visited: {Bob}         │
│                                                          │
│ Iteration 1:                   │ Iteration 1:           │
│   Expand Alice → [Bob, Carol]  │   Expand Bob → [Dave]  │
│   Visited: {A, B, C}           │   Visited: {B, D}      │
│   ─────────────────────────────┼──────────────────────  │
│              │                 │         │               │
│              └────────┬────────┘                         │
│                       ▼                                  │
│              Intersection Found!                         │
│              Path: Alice → Bob                           │
└───────────────────────────────────────────────────────────┘

Benefits:
- Reduces search space from O(b^d) to O(b^(d/2))
- Meet-in-the-middle optimization
- Especially effective for long paths

Neo4j automatically uses bidirectional search for:
- shortestPath() queries
- Undirected variable-length patterns
```

## Part 3: Join Strategies

### Hash Join

```
Hash Join for Pattern Matching:

Query:
MATCH (p:Person)-[:WORKS_AT]->(c:Company)
MATCH (p)-[:LIVES_IN]->(city:City)
WHERE c.name = "Acme"
RETURN p.name, city.name

Hash Join Execution:
┌─────────────────────────────────────────────────────────┐
│ Step 1: Build Side (smaller result set)                 │
│   - Find companies named "Acme"                         │
│   - For each, find employees                            │
│   - Build hash table: { person_id -> company }          │
│                                                          │
│   Hash Table:                                            │
│   ┌─────────────────────────────────────────────────┐   │
│   │ Person ID │ Company                             │   │
│   ├─────────────────────────────────────────────────┤   │
│   │ 42 (Alice)│ Acme (ID: 10)                       │   │
│   │ 50 (Bob)  │ Acme (ID: 10)                       │   │
│   │ 75 (Carol)│ Acme (ID: 10)                       │   │
│   └─────────────────────────────────────────────────┘   │
└───────────────────────────────────────────────────────────┘
                         │
                         ▼
┌─────────────────────────────────────────────────────────┐
│ Step 2: Probe Side                                      │
│   - Find all LIVES_IN relationships                     │
│   - For each, check if person in hash table             │
│   - Emit matches                                        │
│                                                          │
│   Probe:                                                 │
│   Alice (42) lives in NYC ──> Match! Emit (Alice, NYC)  │
│   Bob (50) lives in SF  ──> Match! Emit (Bob, SF)       │
│   Dave (100) lives in LA ──> No match (not in hash)     │
└───────────────────────────────────────────────────────────┘

When Hash Join is Used:
- One side is significantly smaller
- Equi-join conditions (equality predicates)
- Sufficient memory for hash table
```

### Nested Loop Join

```
Nested Loop Join:

Query: Same as above, but small data

Nested Loop Execution:
┌─────────────────────────────────────────────────────────┐
│ For each person who works at Acme:                      │
│   └─> For each city they live in:                       │
│         └─> Emit (person, city)                         │
│                                                          │
│ Pseudo-code:                                             │
│ persons = find_persons_by_company("Acme")               │
│ for p in persons:                                       │
│   cities = find_cities_for_person(p.id)                 │
│   for city in cities:                                   │
│     emit(p.name, city.name)                             │
└───────────────────────────────────────────────────────────┘

When Nested Loop is Used:
- Small result sets
- No suitable index for hash join
- Correlated subqueries (Apply operator)
- When index exists on inner side

Optimization: Index Nested Loop
- If index exists on inner side, use index lookup
- Converts O(n*m) to O(n * log m)
```

### Join Ordering

```
Join Ordering Optimization:

Query with multiple joins:
MATCH (p:Person)-[:WORKS_AT]->(c:Company)
MATCH (p)-[:FRIENDS_OF]->(friend)
MATCH (friend)-[:WORKS_AT]->(other_company)
WHERE c.name = "Acme"
RETURN p.name, friend.name, other_company.name

Cardinality Estimation:
┌─────────────────────────────────────────────────────────┐
│ Statistics Used:                                        │
│ - Label counts: count(:Person), count(:Company)         │
│ - Relationship counts: count(:WORKS_AT)                 │
│ - Property value cardinality: distinct values of c.name │
│ - Value distribution: histogram for c.name              │
└───────────────────────────────────────────────────────────┘

Join Order Options:
┌─────────────────────────────────────────────────────────┐
│ Option A (filter first):                                │
│   1. Filter Company by name = "Acme" (~1 row)           │
│   2. Expand WORKS_AT (incoming) → ~100 persons          │
│   3. Expand FRIENDS_OF → ~500 friends                   │
│   4. Expand WORKS_AT (outgoing) → ~50 companies         │
│   Estimated cost: 1 + 100 + 500 + 50 = 651 ops          │
│                                                          │
│ Option B (start from Person):                           │
│   1. Scan all Persons (~1,000,000 rows)                 │
│   2. Expand WORKS_AT → ~500,000 relationships           │
│   3. Filter Company name = "Acme"                       │
│   ...                                                   │
│   Estimated cost: 1,000,000+ ops                        │
│                                                          │
│ Optimizer chooses Option A (lower cost)                 │
└───────────────────────────────────────────────────────────┘

Cost Model:
- I/O cost: Page reads from disk
- CPU cost: Comparisons, hash computations
- Memory cost: Hash table size, sort buffers
- Network cost: (in cluster mode) data transfer
```

## Part 4: Aggregation and Sorting

### Hash Aggregation

```
GROUP BY with Hash Aggregation:

Query:
MATCH (p:Person)-[:WORKS_AT]->(c:Company)
RETURN c.name, count(p) AS employee_count, avg(p.age) AS avg_age
ORDER BY employee_count DESC

Hash Aggregation Execution:
┌─────────────────────────────────────────────────────────┐
│ Streaming Aggregation (single pass):                    │
│                                                          │
│ Hash Table (key = company_id):                          │
│ ┌──────────────────────────────────────────────────┐    │
│ │ Key     │ Count │ Age Sum │ Name                │    │
│ ├──────────────────────────────────────────────────┤    │
│ │ Acme(10)│ 100   │ 3500    │ "Acme Corp"         │    │
│ │ Globex  │ 50    │ 1600    │ "Globex Inc"        │    │
│ │ Initech │ 75    │ 2400    │ "Initech LLC"       │    │
│ └──────────────────────────────────────────────────┘    │
│                                                          │
│ For each (person, company) pair:                        │
│   agg = hash_table.get_or_insert(company.id)            │
│   agg.count += 1                                        │
│   agg.age_sum += person.age                             │
│   agg.name = company.name                               │
│                                                          │
│ After scan:                                             │
│   For each agg in hash_table:                           │
│     avg_age = agg.age_sum / agg.count                   │
│     emit(agg.name, agg.count, avg_age)                  │
└───────────────────────────────────────────────────────────┘

Memory Management:
- If hash table exceeds memory: Spill to disk
- Sort-based aggregation as fallback
```

### Sort Operations

```
External Merge Sort:

Query: ORDER BY employee_count DESC

In-Memory Sort (small result):
┌─────────────────────────────────────────────────────────┐
│ Results fit in memory:                                  │
│ - Load all rows into array                              │
│ - Quicksort / Timsort (O(n log n))                      │
│ - Return sorted results                                 │
└───────────────────────────────────────────────────────────┘

External Sort (large result):
┌─────────────────────────────────────────────────────────┐
│ Results exceed memory:                                  │
│                                                          │
│ Phase 1 - Create Runs:                                  │
│   - Read chunks that fit in memory                      │
│   - Sort each chunk in memory                           │
│   - Write sorted runs to disk                           │
│                                                          │
│   Memory (1GB)          Disk                            │
│   ┌──────────┐           ┌─────────────────┐           │
│   │ [chunk]  │ ──────>   │ run_0.sorted    │           │
│   │  sort    │           │ run_1.sorted    │           │
│   │  write   │           │ run_2.sorted    │           │
│   └──────────┘           │ ...             │           │
│                          └─────────────────┘           │
│                                                          │
│ Phase 2 - Merge Runs:                                   │
│   - K-way merge of sorted runs                          │
│   - Min-heap for efficient merge                        │
│   - Stream sorted output                                │
│                                                          │
│   ┌─────────────────┐                                    │
│   │ run_0.sorted ───┤                                   │
│   │ run_1.sorted ───┼──> K-way merge ──> Sorted output  │
│   │ run_2.sorted ───┤     (min-heap)                     │
│   └─────────────────┘                                    │
└───────────────────────────────────────────────────────────┘

Optimization: Top-N (LIMIT with ORDER BY)
- Don't sort entire result set
- Use heap of size N
- O(n log k) instead of O(n log n)
```

## Part 5: Query Optimization

### Predicate Pushdown

```
Pushing Filters Closer to Data:

Query:
MATCH (p:Person)-[:WORKS_AT]->(c:Company)
WHERE p.age > 30 AND c.name = "Acme"
RETURN p.name

Without Pushdown:
┌─────────────────────────────────────────────────────────┐
│ +Filter (p.age > 30 AND c.name = "Acme")               │
│   │                                                     │
│ +Expand                                                 │
│   │                                                     │
│ +NodeLabelScan :Person                                 │
│   (scans ALL Person nodes)                             │
└───────────────────────────────────────────────────────────┘

With Pushdown:
┌─────────────────────────────────────────────────────────┐
│ +Filter (p.age > 30)                                    │
│   │                                                     │
│ +Expand                                                 │
│   │                                                     │
│ +NodeIndexSeek :Company(name = "Acme")                 │
│   (starts from filtered Company)                        │
└───────────────────────────────────────────────────────────┘

Benefits:
- Fewer rows to expand
- Less data to process
- Better index utilization
```

### Path Pruning

```
Pruning Dead-End Paths:

Query:
MATCH (p:Person)-[:FRIENDS_OF*3]->(fof)
WHERE p.age > 50
RETURN fof.name

Path Pruning:
┌─────────────────────────────────────────────────────────┐
│ During traversal:                                       │
│                                                          │
│ Depth 0: Persons over 50 → [Alice(60), Bob(55)]         │
│ Depth 1: Their friends → [Carol, Dave, Eve]             │
│ Depth 2: Friends of friends → [Frank]                   │
│ Depth 3: FOFs → [] (Frank has no friends)               │
│                                                          │
│ Without pruning: Would continue exploring from Frank    │
│ With pruning: Stop when no matching relationships       │
└───────────────────────────────────────────────────────────┘

Subquery Pruning:
MATCH (p:Person)
WHERE EXISTS((p)-[:FRIENDS_OF]->(f {status: "active"}))
RETURN p.name

- Only check persons with FRIENDS_OF relationships
- Early exit on first match (EXISTS short-circuits)
```

### Index Hint

```
Forcing Index Usage:

Query with index hint:
MATCH (p:Person)
USING INDEX p:Person(name)
WHERE p.name STARTS WITH "A"
RETURN p

When to Use Hints:
- Statistics are stale
- Optimizer chooses wrong plan
- Testing/development

Available Hints:
- USING INDEX p:Label(property)
- USING JOIN ON p:Label
- USING PERIODIC COMMIT (for LOAD CSV)

Check Plan:
EXPLAIN MATCH ...
CYPLAN 5  -- Force specific planner version
```

---

*This document is part of the Neodatabase exploration series. See [exploration.md](./exploration.md) for the complete index.*
