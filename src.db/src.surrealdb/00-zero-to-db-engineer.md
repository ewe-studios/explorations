# SurrealDB: Zero to Database Engineer

## Table of Contents

1. [What is SurrealDB?](#1-what-is-surrealdb)
2. [Multi-Model Databases Explained](#2-multi-model-databases-explained)
3. [Document Databases](#3-document-databases)
4. [Graph Databases](#4-graph-databases)
5. [Relational Databases](#5-relational-databases)
6. [SurrealQL Basics](#6-surrealql-basics)

---

## 1. What is SurrealDB?

**SurrealDB** is a **multi-model database** that combines:
- **Document storage** (like MongoDB)
- **Graph relationships** (like Neo4j)
- **Relational queries** (like PostgreSQL)
- **Real-time subscriptions** (like Firebase)

### Why Multi-Model?

Traditional approach requires multiple databases:
```
Application -> API Server -> PostgreSQL (relational)
                          -> Neo4j (graph)
                          -> Redis (cache)
                          -> Elasticsearch (search)
```

SurrealDB approach:
```
Application -> SurrealDB (everything in one)
```

---

## 2. Multi-Model Databases Explained

### What Does "Multi-Model" Mean?

A **data model** is how data is organized and accessed:

| Model | Organization | Access Pattern |
|-------|--------------|----------------|
| Document | JSON-like documents | By ID, by field |
| Graph | Nodes and edges | Traversal |
| Relational | Tables, rows | SQL queries |
| Key-Value | Simple pairs | By key |

**Multi-model** means supporting multiple access patterns on the same data.

### Example: User Data

```javascript
// Document view
{
  id: "user:1",
  name: "Alice",
  email: "alice@example.com"
}

// Graph view
(Alice) -[FRIEND]-> (Bob)
(Alice) -[WORKS_AT]-> (Company)

// Relational view
| id | name  | email           |
|----|-------|-----------------|
| 1  | Alice | alice@example.com |
```

---

## 3. Document Databases

### What is a Document Database?

Stores data as **documents** (typically JSON-like):

```javascript
{
  "_id": "user:1",
  "name": "Alice",
  "age": 30,
  "address": {
    "city": "New York",
    "zip": "10001"
  },
  "tags": ["admin", "active"]
}
```

### Characteristics

| Aspect | Document DB |
|--------|-------------|
| Schema | Flexible (schemaless) |
| Nesting | Supports nested objects |
| Scaling | Horizontal (sharding) |
| Queries | By field, by ID |

### SurrealDB Document Operations

```sql
-- Create document
CREATE user:1 SET name = 'Alice', age = 30;

-- Get document
SELECT * FROM user:1;

-- Update document
UPDATE user:1 SET age = 31;

-- Delete document
DELETE user:1;
```

---

## 4. Graph Databases

### What is a Graph Database?

Stores data as **nodes** (entities) and **edges** (relationships):

```
(Alice) --[FRIEND]--> (Bob)
   |                    |
[WORKS_AT]         [LIVES_IN]
   |                    |
   v                    v
(Company) --[LOCATED_IN]--> (New York)
```

### Graph Traversals

```
"Find all friends of Alice's friends"

(Alice) -> [FRIEND] -> ? -> [FRIEND] -> ???
```

### SurrealDB Graph Operations

```sql
-- Create edge
CREATE friend RELATES user:1 TO user:2 SET since = 2024;

-- Traverse graph
SELECT ->friend->user FROM user:1;

-- Two-hop traversal
SELECT ->friend->friend->user FROM user:1;
```

---

## 5. Relational Databases

### What is a Relational Database?

Stores data in **tables** with rows and columns:

```sql
CREATE TABLE users (
  id INTEGER PRIMARY KEY,
  name TEXT NOT NULL,
  email TEXT UNIQUE
);

INSERT INTO users VALUES (1, 'Alice', 'alice@example.com');
```

### SQL Queries

```sql
-- Select with filter
SELECT * FROM users WHERE age > 25;

-- Join tables
SELECT u.name, o.total
FROM users u
JOIN orders o ON u.id = o.user_id;

-- Aggregate
SELECT COUNT(*), AVG(age) FROM users;
```

---

## 6. SurrealQL Basics

### Record IDs

Every record has a unique ID: `table:id`

```sql
user:1        -- Numeric ID
user:abc      -- String ID
user:uuid()   -- Generated UUID
post:$uuid    -- Variable reference
```

### Creating Records

```sql
-- Simple create
CREATE user SET name = 'Alice';

-- With specific ID
CREATE user:1 SET name = 'Alice';

-- Create multiple
CREATE user:[1,2,3] SET name = 'User';

-- Create from select
CREATE premium_user AS SELECT * FROM user WHERE premium = true;
```

### Querying Records

```sql
-- Select all
SELECT * FROM user;

-- Select with filter
SELECT name, email FROM user WHERE age > 25;

-- Order and limit
SELECT * FROM user ORDER BY name DESC LIMIT 10;

-- Graph traversal
SELECT ->follows->user FROM user:1;
```

### Updating Records

```sql
-- Full update
UPDATE user:1 SET name = 'Bob', age = 31;

-- Partial update
UPDATE user:1 MERGE { age: 31 };

-- Patch specific fields
UPDATE user:1 CONTENT { name: 'Bob' };
```

### Deleting Records

```sql
-- Delete specific
DELETE user:1;

-- Delete with condition
DELETE FROM user WHERE age < 18;
```

---

## Document History

| Date | Change |
|------|--------|
| 2026-03-27 | Initial zero-to-engineer guide created |

---

*This exploration is a living document. Revisit sections as concepts become clearer through implementation.*
