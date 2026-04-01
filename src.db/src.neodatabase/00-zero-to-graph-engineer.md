---
title: "Zero to Graph Engineer: Neo4j & Graph Databases"
subtitle: "Graph database fundamentals, property graph model, and Cypher query language"
location: /home/darkvoid/Boxxed/@dev/repo-expolorations/src.db/src.neodatabase
related: exploration.md
---

# 00 - Zero to Graph Engineer: Neo4j & Graph Databases

## Overview

This document covers graph database fundamentals - why graphs for connected data, the property graph model, and Cypher query language basics.

## Part 1: Why Graph Databases?

### The Connected Data Problem

```
Traditional RDBMS for Connected Data:

Query: "Find friends of friends who work at the same company"

SQL (4+ JOINs):
SELECT DISTINCT f2.name
FROM users u
JOIN friendships f ON f.user_id = u.id
JOIN users f1 ON f1.id = f.friend_id
JOIN friendships f2 ON f2.user_id = f1.id
JOIN users f2u ON f2u.id = f2.friend_id
JOIN companies c ON c.id = f2u.company_id
WHERE u.name = 'Alice'
  AND c.name = 'Acme'
  AND f2.friend_id != u.id;

Problems:
- JOIN explosion as data grows
- Fixed schema limits relationship types
- Recursive queries require CTEs or application logic
- Index efficiency degrades with multiple JOINs
```

```
Graph Database Solution:

Query: "Find friends of friends who work at the same company"

Cypher (pattern matching):
MATCH (alice:Person {name: "Alice"})-[:FRIENDS_OF*2]->(fof:Person)
WHERE (fof)-[:WORKS_AT]->(c:Company {name: "Acme"})
RETURN fof.name

Benefits:
- Relationships are first-class citizens
- Constant-time traversal (index-free adjacency)
- Natural expression of recursive patterns
- Schema flexibility for evolving relationships
```

### When to Use Graph Databases

```
Ideal Use Cases for Graph Databases:

┌─────────────────────────────────────────────────────────┐
│ Use Case              │ Why Graph                      │
├─────────────────────────────────────────────────────────┤
│ Social Networks       │ Friend graphs, recommendations │
│ Fraud Detection       │ Pattern detection, rings       │
│ Knowledge Graphs      │ Entity relationships, ontologies│
│ Recommendation Engines│ Collaborative filtering        │
│ Network/IT Ops        │ Dependency mapping, impact     │
│ Supply Chain          │ Multi-tier supplier tracking   │
│ Identity & Access     │ Permission inheritance         │
│ Master Data Management│ Entity resolution              │
└───────────────────────────────────────────────────────────┘

Good Fit Signals:
- Relationships are as important as entities
- Deep hierarchical queries (3+ levels)
- Recursive patterns (trees, graphs)
- Highly connected data with many-to-many relationships
- Evolving schema with new relationship types

Poor Fit Signals:
- Simple CRUD operations
- Heavily structured, tabular data
- No relationship traversal needs
- Bulk aggregation queries over entire dataset
```

### Performance Comparison

```
JOIN Performance: Depth vs Latency

Relational (JOIN-heavy):
Depth 1: ████ 10ms
Depth 2: ████████ 50ms
Depth 3: ████████████████ 150ms
Depth 4: ████████████████████████ 300ms
Depth 5: ████████████████████████████████ 500ms
(Degradation: O(n²) or worse)

Graph Database:
Depth 1: ████ 10ms
Depth 2: █████ 12ms
Depth 3: ██████ 15ms
Depth 4: ███████ 18ms
Depth 5: ████████ 22ms
(Constant-time traversal: O(1) per hop)

Why Graph is Faster for Deep Traversals:
- Index-free adjacency: relationships are direct pointers
- No JOIN computation at query time
- Relationships stored physically adjacent to nodes
```

## Part 2: Property Graph Model

### Core Concepts

```
Property Graph Elements:

1. Nodes (Vertices)
   - Represent entities/objects
   - Can have multiple labels (types)
   - Store properties as key-value pairs

2. Relationships (Edges)
   - Connect exactly two nodes (source → target)
   - Have a single type (directional)
   - Can store properties as key-value pairs
   - Always have direction (but queries can traverse either way)

3. Properties
   - Key-value pairs on nodes or relationships
   - Values can be: strings, numbers, booleans, arrays
   - Typed values (unlike JSON documents)

4. Labels
   - Group nodes into categories
   - A node can have multiple labels
   - Used for indexing and constraints
```

### Node Structure

```
Node Anatomy:

(:Person {
  id: 12345,
  name: "Alice Johnson",
  email: "alice@example.com",
  age: 30,
  interests: ["hiking", "photography", "cooking"]
})

Labels: :Person (can have multiple: :Person:Employee:Manager)
Properties: id, name, email, age, interests
Property Types:
  - id: Integer
  - name: String
  - email: String
  - age: Integer
  - interests: Array<String>

Node Identity:
- Internal ID: Neo4j assigns unique node ID (implementation detail)
- Business Key: Use properties like `id` or `email` for application logic
```

### Relationship Structure

```
Relationship Anatomy:

(Alice:Person)-[:FRIENDS_WITH {
  since: date("2020-01-15"),
  closeness: 0.95,
  met_at: "Stanford University"
}]->(Bob:Person)

Relationship Type: :FRIENDS_WITH (singular, past tense convention)
Direction: Alice → Bob (Alice initiated, or chronological)
Properties:
  - since: Date relationship started
  - closeness: Float score 0.0-1.0
  - met_at: String location

Direction Matters:
- (A)-[:PARENT_OF]->(B) means A is parent of B
- (B)<-[:PARENT_OF]-(A) is equivalent (query can go either way)
- But semantics matter: choose direction consistently
```

### Common Relationship Patterns

```
Social Network Schema:

Nodes:
- (:Person {id, name, email, age})
- (:Group {id, name, description, created_at})
- (:Post {id, content, created_at, likes})
- (:Comment {id, content, created_at})

Relationships:
- (:Person)-[:FRIENDS_WITH {since}]->(:Person)
- (:Person)-[:FOLLOWS]->(:Person)  (asymmetric, like Twitter)
- (:Person)-[:MEMBER_OF {role, joined_at}]->(:Group)
- (:Person)-[:CREATED]->(:Post)
- (:Person)-[:LIKED {rated_at}]->(:Post)
- (:Person)-[:COMMENTED {comment_text}]->(:Post)
- (:Post)-[:HAS_COMMENT]->(:Comment)
- (:Comment)-[:WRITTEN_BY]->(:Person)

Example Graph:
┌─────────────────────────────────────────────────────────┐
│                                                          │
│  (Alice) -[:FRIENDS_WITH]-> (Bob) -[:FRIENDS_WITH]-> (Carol)
│    |                       |                              │
│  [:CREATED]            [:FOLLOWS]                        │
│    |                       |                              │
│    v                       v                              │
│  (Post1)               (Dave) -[:CREATED]-> (Post2)      │
│    |                                                  │
│  [:HAS_COMMENT]                                          │
│    |                                                  │
│    v                                                  │
│  (Comment1) -[:WRITTEN_BY]-> (Carol)                    │
│                                                          │
└───────────────────────────────────────────────────────────┘
```

### Label Conventions

```
Label Naming Best Practices:

Singular nouns (not plural):
✓ :Person, :Company, :Product
✗ :Persons, :Companies, :Products

CamelCase for multi-word labels:
✓ :CreditCard, :UserProfile, :OrderItem
✗ :Credit_card, :userprofile, :ORDER-ITEM

Multiple labels per node:
(Alice:Person:Employee:Manager {
  id: 123,
  name: "Alice",
  hire_date: "2020-01-15",
  level: "L5"
})

Label hierarchy (optional convention):
- Base type: :Person
- Role: :Employee
- Seniority: :Manager, :Senior, :Principal

Query by multiple labels:
MATCH (e:Person:Employee:Manager)
WHERE e.level = "L5"
RETURN e.name
```

## Part 3: Cypher Query Language

### Basic Syntax

```
Cypher Reading Order (ASCII art style):

Node: (variable:Label {property: value})
      ╰─┬─╯ ╰──┬──╯ ╰────┬────╯
        │      │         │
      var   label    properties

Relationship: -[:TYPE {prop: val}]->
              ╰────┬────╯ ╰───┬───╯
                    │         │
                  type   properties
              ╰───────────╯
               direction

Direction:
- [a]-[b]  : Undirected (either direction)
- [a]->[b] : Directed (a to b)
- [a]<-[b] : Directed (b to a)
```

### CRUD Operations

```
CREATE - Creating Nodes and Relationships:

-- Create a single node
CREATE (alice:Person {name: "Alice", age: 30})

-- Create multiple nodes
CREATE (alice:Person {name: "Alice", age: 30}),
       (bob:Person {name: "Bob", age: 25}),
       (acme:Company {name: "Acme", founded: 1990})

-- Create node with relationship
MATCH (alice:Person {name: "Alice"})
MATCH (bob:Person {name: "Bob"})
CREATE (alice)-[:FRIENDS_WITH {since: date()}]->(bob)

-- Create and return
CREATE (p:Person {name: "Charlie"})
RETURN p.id, p.name, labels(p)

MERGE - Create if not exists (upsert):

-- Merge node (find or create)
MERGE (alice:Person {name: "Alice"})
RETURN alice

-- Merge relationship
MATCH (alice:Person {name: "Alice"})
MATCH (bob:Person {name: "Bob"})
MERGE (alice)-[r:FRIENDS_WITH]->(bob)
RETURN r

-- Merge with ON CREATE / ON MATCH
MERGE (p:Person {name: "Alice"})
ON CREATE SET p.created_at = datetime()
ON MATCH SET p.last_seen = datetime()
RETURN p
```

```
READ - MATCH and Pattern Matching:

-- Find all nodes with label
MATCH (p:Person)
RETURN p.name, p.age

-- Find with property filter
MATCH (p:Person {name: "Alice"})
RETURN p

-- Pattern: node-relationship-node
MATCH (p:Person)-[:WORKS_AT]->(c:Company)
RETURN p.name, c.name

-- Multiple patterns
MATCH (p:Person)-[:WORKS_AT]->(c:Company {name: "Acme"})
MATCH (p)-[:FRIENDS_WITH]->(friend)
RETURN p.name, friend.name

-- Optional match (like LEFT JOIN)
MATCH (p:Person {name: "Alice"})
OPTIONAL MATCH (p)-[:WORKS_AT]->(c:Company)
RETURN p.name, c.name  -- c is null if no match

-- Variable-length relationships (recursive)
MATCH (p:Person)-[:FRIENDS_WITH*1..3]->(fof)
WHERE p.name = "Alice"
RETURN DISTINCT fof.name

-- Shortest path
MATCH path = shortestPath(
  (alice:Person {name: "Alice"})-[:FRIENDS_WITH*]-(bob:Person {name: "Bob"})
)
RETURN path
```

```
UPDATE - Modifying Nodes and Relationships:

-- Set properties
MATCH (p:Person {name: "Alice"})
SET p.age = 31, p.email = "alice@new.com"
RETURN p

-- Set from parameter
MATCH (p:Person {name: "Alice"})
SET p += {age: 31, email: "alice@new.com"}
RETURN p

-- Remove properties
MATCH (p:Person {name: "Alice"})
REMOVE p.email, p.age
RETURN p

-- Remove label
MATCH (p:Person {name: "Alice"})
REMOVE p:Employee
RETURN p

-- Add label
MATCH (p:Person {name: "Alice"})
SET p:Manager
RETURN p, labels(p)

-- Update relationship property
MATCH (p:Person)-[r:FRIENDS_WITH]->(f:Person)
WHERE p.name = "Alice" AND f.name = "Bob"
SET r.closeness = 0.95
RETURN r
```

```
DELETE - Removing Nodes and Relationships:

-- Delete relationship only
MATCH (p:Person)-[r:FRIENDS_WITH]->(f:Person)
WHERE p.name = "Alice" AND f.name = "Bob"
DELETE r

-- Delete node (must have no relationships)
MATCH (p:Person {name: "Charlie"})
DELETE p

-- Delete node with all relationships
MATCH (p:Person {name: "Charlie"})
DETACH DELETE p

-- Delete all nodes with specific label (DANGER!)
MATCH (n:TemporaryData)
DETACH DELETE n

-- Delete pattern
MATCH (p:Person {name: "Alice"})-[r:FRIENDS_WITH]->(f:Person)
WHERE f.name = "Bob"
DELETE r
```

### Filtering with WHERE

```
WHERE Clause:

-- Comparison operators
MATCH (p:Person)
WHERE p.age > 25 AND p.age < 40
RETURN p.name

-- String matching
MATCH (p:Person)
WHERE p.name STARTS WITH "A"
RETURN p.name

MATCH (p:Person)
WHERE p.name ENDS WITH "e"
RETURN p.name

MATCH (p:Person)
WHERE p.name CONTAINS "li"
RETURN p.name

-- Regex
MATCH (p:Person)
WHERE p.name =~ "A.*e"
RETURN p.name

-- IN list
MATCH (p:Person)
WHERE p.age IN [25, 30, 35]
RETURN p.name, p.age

-- EXISTS pattern
MATCH (p:Person)
WHERE EXISTS((p)-[:WORKS_AT]->(:Company))
RETURN p.name

-- Type checking
MATCH (n)
WHERE n:Person
RETURN n.name

-- NULL handling
MATCH (p:Person)
WHERE p.email IS NOT NULL
RETURN p.name

MATCH (p:Person)
WHERE p.phone IS NULL
RETURN p.name
```

### Aggregation

```
Aggregation Functions:

-- COUNT
MATCH (p:Person)
RETURN count(p) AS total_people

MATCH (p:Person)
RETURN count(DISTINCT p.age) AS unique_ages

-- SUM, AVG, MIN, MAX
MATCH (p:Person)
RETURN
  sum(p.age) AS total_age,
  avg(p.age) AS average_age,
  min(p.age) AS youngest,
  max(p.age) AS oldest

-- COLLECT (list aggregation)
MATCH (p:Person)
RETURN collect(p.name) AS names

MATCH (c:Company)
MATCH (p:Person)-[:WORKS_AT]->(c)
RETURN c.name, collect(p.name) AS employees

-- GROUP BY (implicit by non-aggregated columns)
MATCH (p:Person)
RETURN p.age, count(p) AS count
ORDER BY p.age

-- HAVING-like filtering (use WHERE after aggregation)
MATCH (p:Person)-[:WORKS_AT]->(c:Company)
WITH c, count(p) AS employee_count
WHERE employee_count > 100
RETURN c.name, employee_count
ORDER BY employee_count DESC
```

### Variable-Length Patterns

```
Recursive Traversals:

-- Exact length: exactly 2 hops
MATCH (p:Person)-[:FRIENDS_WITH*2]->(fof)
WHERE p.name = "Alice"
RETURN fof.name

-- Range: 1 to 3 hops
MATCH (p:Person)-[:FRIENDS_WITH*1..3]->(connection)
WHERE p.name = "Alice"
RETURN DISTINCT connection.name

-- Unbounded: any number of hops
MATCH (p:Person)-[:FRIENDS_WITH*]->(any_connection)
WHERE p.name = "Alice"
RETURN count(DISTINCT any_connection) AS network_size

-- With relationship filter
MATCH path = (alice:Person {name: "Alice"})-[:FRIENDS_WITH*..5]->(target)
WHERE ALL(r IN relationships(path) WHERE r.since > date("2020-01-01"))
RETURN target.name

-- BFS-style: find closest connection
MATCH path = shortestPath(
  (alice:Person {name: "Alice"})-[:FRIENDS_WITH*]-(bob:Person {name: "Bob"})
)
RETURN length(path) AS degrees_of_separation, path
```

### Query Examples

```
Example 1: Social Network Recommendations

-- "People you may know" (friends of friends)
MATCH (me:Person {name: "Alice"})-[:FRIENDS_WITH*2]->(fof:Person)
WHERE NOT (me)-[:FRIENDS_WITH]->(fof)
  AND fof <> me
RETURN fof.name, count(fof) AS mutual_friends
ORDER BY mutual_friends DESC
LIMIT 10

Example 2: Organizational Chart

-- Find all managers above someone
MATCH path = (emp:Person {name: "Charlie"})-[:REPORTS_TO*]->(manager)
RETURN [n IN nodes(path) | n.name] AS management_chain

-- Find all direct reports under a manager
MATCH (mgr:Person {name: "Alice"})<-[:REPORTS_TO*]-(report)
RETURN report.name, length(path) AS levels_below

Example 3: Fraud Detection

-- Detect circular money transfers (potential fraud ring)
MATCH path = (start:Account)-[:TRANSFERRED_TO*3..6]->(start)
WHERE start.account_type = "personal"
RETURN path, length(path) AS cycle_length

-- Detect rapid money movement (structuring)
MATCH (a1:Account)-[t1:TRANSFERRED_TO]->(a2:Account)
MATCH (a2)-[t2:TRANSFERRED_TO]->(a3:Account)
WHERE t1.timestamp < t2.timestamp - duration("PT1H")
  AND t1.amount > 9000
  AND t2.amount > 9000
RETURN a1.account_id, a2.account_id, a3.account_id,
       t1.amount + t2.amount AS total_amount
```

---

*This document is part of the Neodatabase exploration series. See [exploration.md](./exploration.md) for the complete index.*
