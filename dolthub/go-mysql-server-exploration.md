# go-mysql-server Exploration

## Overview

go-mysql-server is a SQL database engine framework written in Go. It provides the core SQL processing capabilities that power Dolt and can be used as a foundation for building custom database backends.

**Key Features:**
- SQL parsing via Vitess
- Query planning and optimization
- Extensible execution engine
- MySQL wire protocol server
- Comprehensive test harness

## Architecture

```
go-mysql-server/
├── sql/                       # Core SQL package
│   ├── analyzer/              # Query analyzer
│   ├── expression/            # SQL expressions
│   │   ├── function/          # Built-in functions
│   │   └── aggregation/       # Aggregation functions
│   ├── plan/                  # Execution plan nodes
│   ├── parse/                 # SQL parsing
│   └── types/                 # SQL type system
├── server/                    # MySQL protocol server
├── auth/                      # Authentication system
├── enginetest/                # Test harness
├── memory/                    # In-memory backend example
└── _example/                  # Usage examples
```

## Core Components

### 1. SQL Package (`sql/`)

The heart of go-mysql-server, containing:

#### Interfaces

```go
// Database - provides tables from a data source
type Database interface {
    GetTable(ctx *Context, name string) (Table, bool, error)
    GetTableNames(ctx *Context) ([]string, error)
}

// Table - provides rows from a data source
type Table interface {
    Name() string
    String() string
    Schema() Schema
    Partitions(ctx *Context) (PartitionIter, error)
    PartitionRows(ctx *Context, partition *Partition) (RowIter, error)
}

// DatabaseProvider - finds available databases
type DatabaseProvider interface {
    Database(ctx *Context, db string) (Database, error)
}
```

#### Analyzer (`sql/analyzer/`)

The analyzer transforms parsed SQL into an execution plan:

1. **Parse** - Convert SQL text to AST using Vitess
2. **Analyze** - Transform AST through rule-based phases:
   - Resolve tables, columns, databases
   - Apply optimizations
   - Transform to execution plan

Key files:
- `rules.go` - All analysis rules and phases
- `analyzer.go` - Main analyzer logic

```go
// Analysis phases (from ARCHITECTURE.md):
// 1. Resolve databases
// 2. Resolve tables
// 3. Resolve columns
// 4. Apply optimizations
// 5. Transform to execution plan
```

#### Execution Plan Nodes (`sql/plan/`)

SQL queries become trees of nodes:

```
SELECT foo FROM bar

Project(foo)
 └── Table(bar)
```

Common nodes:
- `Project` - Select columns
- `Filter` - WHERE clause
- `JoinNode` - JOIN operations
- `Aggregate` - GROUP BY
- `Sort` - ORDER BY
- `Limit` - LIMIT/OFFSET

#### Expressions (`sql/expression/`)

Expression types:
- Arithmetic: `+`, `-`, `*`, `/`
- Comparison: `=`, `!=`, `<`, `>`
- Logic: `AND`, `OR`, `NOT`
- Functions: Built-in and user-defined
- Aggregations: `COUNT`, `SUM`, `AVG`, etc.

### 2. Server Package (`server/`)

Provides MySQL wire protocol implementation:

- Connection handling
- Query parsing and execution
- Result serialization
- Authentication

### 3. Authentication (`auth/`)

Two authentication methods:
- **None** - No authentication required
- **Native** - User/password with JSON config

Permission levels:
- Read
- Write
- All

### 4. Test Harness (`enginetest/`)

Comprehensive testing framework:

```go
// Example test query structure
queries.go contains:
- Query: SQL to execute
- Expected: Expected results
- ExpectedErr: Expected errors (if any)
```

Run tests on custom backend:
```bash
# Run engine tests
cd enginetest
go test -v
```

## Backend Integration

### Required Interfaces

To create a custom backend, implement:

1. **`sql.DatabaseProvider`**
   - Optional: `MutableDatabaseProvider` (create/drop databases)
   - Optional: `CollatedDatabaseProvider` (collations)

2. **`sql.Database`**
   - Optional: `TableCreator`, `TableDropper`, `TableRenamer`
   - Optional: `ViewCreator`, `ViewDropper`

3. **`sql.Table`**
   - Optional: `InsertableTable`, `UpdateableTable`, `DeletableTable`
   - Optional: `AlterableTable`
   - Optional: `IndexedTable`
   - Optional: `ProjectedTable`, `FilteredTable`

### Sessions and Transactions

- **`sql.BaseSession`** - Default session implementation
- **`sql.Session`** - Custom session for backend-specific data
- **`sql.TransactionSession`** - For transactional backends

### Native Indexes

Implement `sql.IndexedTable` to support indexes:

```go
type IndexedTable interface {
    GetIndexes(ctx *Context) ([]Index, error)
    LookupPartitions(ctx *Context, lookup IndexLookup) (PartitionIter, error)
}
```

## Query Execution Flow

```
1. Client sends SQL query
       ↓
2. Server receives via MySQL protocol
       ↓
3. Parse SQL with Vitess → AST
       ↓
4. Analyzer transforms AST:
   - Resolve tables/columns
   - Apply optimizations
   - Create execution plan
       ↓
5. Execute plan recursively:
   - Top node requests rows
   - Child nodes produce rows
   - Results flow back up
       ↓
6. Results sent to client
```

## Key Design Patterns

### 1. Rule-Based Analysis

Analyzer uses atomic rules executed in phases:

```go
// Each rule:
// - Takes a Node
// - Returns transformed Node
// - Should be as small/atomic as possible
```

### 2. Node/Expression Tree

All plan nodes implement:
- `sql.Node` - Tree nodes
- `sql.Expression` - Expressions within nodes

Utilities:
- `Inspect` - Examine tree
- `Walk` - Traverse tree

### 3. Iterator Pattern

Row production uses iterators:
- `PartitionIter` - Iterate over partitions
- `RowIter` - Iterate over rows
- Support early termination via `Close()`

## Example Usage

```go
import (
    "github.com/dolthub/go-mysql-server"
    "github.com/dolthub/go-mysql-server/memory"
    "github.com/dolthub/go-mysql-server/server"
    "github.com/dolthub/go-mysql-server/sql"
)

// Create in-memory database
db := memory.NewDatabase("mydb")

// Create provider
provider := memory.NewDBProvider(db)

// Create engine
engine := sqle.NewDefault(provider)

// Create server
server, err := server.NewServer(ctx, &server.Config{
    Port: 3306,
}, engine, nil)
```

## Performance Considerations

1. **Index Usage** - Analyzer transforms queries to use indexes
2. **Projection Pushdown** - `ProjectedTable` returns only needed columns
3. **Filter Pushdown** - `FilteredTable` filters at storage level
4. **Parallel Execution** - Some operations support parallelism

## Extensibility

### Custom Functions

Register custom SQL functions:
```go
// Add to function registry
sql.RegisterFunction("my_func", myFunc)
```

### Custom Index Drivers

Implement `sql.IndexDriver` for external index storage:
```go
type IndexDriver interface {
    ID() string
    CreateIndex(ctx *Context, db string, table Table, exprs []Expression) Index
}
```

## Integration with Dolt

Dolt extends go-mysql-server by:

1. **Custom Database Provider** - Versioned databases
2. **Custom Tables** - Versioned table implementations
3. **Custom Plan Nodes** - Dolt-specific operations
4. **System Tables** - `dolt_*` tables for version control
5. **Stored Procedures** - `dolt_add`, `dolt_commit`, etc.

## Testing

### Engine Tests

Located in `enginetest/`:
- `queries.go` - Test queries and expected results
- `engine_test.go` - Test runner

### Integration Tests

Client compatibility tests:
```bash
make TEST=mysql integration
```

## Files of Interest

| File | Purpose |
|------|---------|
| `engine.go` | Main engine implementation |
| `sql/analyzer/rules.go` | Analysis rules definition |
| `sql/plan/*.go` | Execution plan nodes |
| `sql/expression/*.go` | Expression implementations |
| `server/server.go` | MySQL protocol server |
| `memory/database.go` | Example backend |

## References

- [Architecture Overview](../../src.dolthub/go-mysql-server/ARCHITECTURE.md)
- [Backend Integration Guide](../../src.dolthub/go-mysql-server/BACKEND.md)
- [Supported Clients](../../src.dolthub/go-mysql-server/SUPPORTED_CLIENTS.md)
