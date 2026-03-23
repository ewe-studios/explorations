# Dolt Exploration

## Overview

Dolt is the flagship project from Dolthub - a MySQL-compatible SQL database with Git-like version control. It allows you to fork, clone, branch, merge, push, and pull database changes just like a Git repository.

**Key Properties:**
- MySQL wire protocol compatible
- Git-style CLI (`dolt add`, `dolt commit`, `dolt merge`, etc.)
- SQL-accessible version control via system tables
- Branch and merge at the table level
- Conflict detection and resolution

## Repository Structure

```
dolt/
├── go/
│   ├── cmd/dolt/                    # CLI implementation
│   │   ├── cli/                     # Command-line interface
│   │   └── commands/                # Commands (add, commit, merge, etc.)
│   ├── libraries/
│   │   └── doltcore/                # Core Dolt library
│   │       ├── doltdb/              # Database abstraction
│   │       ├── merge/               # Merge algorithms
│   │       ├── diff/                # Diff computation
│   │       ├── table/               # Table operations
│   │       ├── schema/              # Schema handling
│   │       ├── row/                 # Row operations
│   │       ├── ref/                 # Branch/tag references
│   │       ├── env/                 # Environment/repo state
│   │       ├── remotesrv/           # Remote server
│   │       └── sqle/                # SQL engine integration
│   ├── store/
│   │   ├── nbs/                     # Noms Block Store
│   │   ├── prolly/                  # Prolly trees
│   │   ├── chunks/                  # Chunk storage
│   │   └── types/                   # Type system
│   ├── gen/fb/serial/               # FlatBuffers generated code
│   └── proto/                       # Protocol buffers
├── integration-tests/               # Bats test suite
└── docker/                          # Docker configurations
```

## Core Concepts

### 1. Version Control Model

Dolt versions **tables** (not files like Git):

| Git Concept | Dolt Equivalent |
|-------------|-----------------|
| Repository | Dolt database |
| File | Table |
| Directory | Database |
| Commit | Database commit |
| Branch | Database branch |
| Merge | Table merge |
| Diff | Row/schema diff |
| Index | Table index |

### 2. CLI Commands

```bash
# Repository operations
dolt init                    # Initialize new database
dolt clone <remote>          # Clone remote database
dolt status                  # Show working state
dolt log                     # View commit history

# Branching
dolt branch                  # List branches
dolt branch <name>           # Create branch
dolt checkout <branch>       # Switch branch
dolt merge <branch>          # Merge branches

# Staging and committing
dolt add <table>             # Stage table changes
dolt diff                    # Show changes
dolt commit -m "msg"         # Commit changes

# Remote operations
dolt push <remote> <branch>  # Push to remote
dolt pull <remote>           # Pull from remote
dolt fetch <remote>          # Fetch from remote
```

### 3. SQL Interface

Version control via SQL:

```sql
-- View status
SELECT * FROM dolt_status;

-- View log
SELECT * FROM dolt_log;

-- View branches
SELECT * FROM dolt_branches;

-- Stage and commit
CALL dolt_add('table_name');
CALL dolt_commit('-m', 'commit message');

-- View diffs
SELECT * FROM dolt_diff_table_name;

-- Merge
CALL dolt_merge('branch_name');

-- View conflicts
SELECT * FROM dolt_conflicts_table_name;
```

## Architecture Layers

### 1. CLI Layer (`cmd/dolt/`)

Command implementations for all Git-like operations:

```
commands/
├── add.go                   # dolt add
├── branch.go                # dolt branch
├── checkout.go              # dolt checkout
├── commit.go                # dolt commit
├── diff.go                  # dolt diff
├── merge.go                 # dolt merge
├── push.go                  # dolt push
├── pull.go                  # dolt pull
├── sql.go                   # dolt sql
├── sql-server.go            # dolt sql-server
└── ...                      # 50+ commands
```

### 2. Core Library (`libraries/doltcore/`)

#### doltdb/ - Database Abstraction

Key types:
- `DoltDB` - Main database handle
- `RootValue` - Database state at a point in time
- `Commit` - A commit object
- `Table` - Table handle
- `WorkingSet` - Current working state

```go
// RootValue interface (root_val.go)
type RootValue interface {
    GetTable(ctx, tableName) (*Table, bool, error)
    PutTable(ctx, tableName, *Table) (RootValue, error)
    RemoveTables(ctx, ...tableName) (RootValue, error)
    GetForeignKeyCollection() (*ForeignKeyCollection, error)
    // ... 30+ methods
}
```

#### merge/ - Merge Algorithms

Three-way merge implementation:

```go
// merge.go - Main merge entry point
func MergeCommits(ctx, commit, mergeCommit *Commit) (*Result, error)
func MergeRoots(ctx, ourRoot, theirRoot, ancRoot RootValue) (*Result, error)

// Result contains:
// - Root: merged root value
// - SchemaConflicts: schema-level conflicts
// - Stats: per-table merge stats
// - CommitVerificationErr: any verification errors
```

Key merge files:
- `merge.go` - Main merge orchestration
- `merge_prolly_rows.go` - Row-level merge (76KB!)
- `merge_schema.go` - Schema merging
- `merge_rows.go` - Row merging
- `merge_prolly_indexes.go` - Index merging

#### diff/ - Diff Computation

```
diff/
├── diff.go                  # Core diff types
├── diffsplitter.go          # Split diff into changes
├── table_deltas.go          # Table-level changes
├── schema_diff.go           # Schema changes
└── diff_stat.go             # Diff statistics
```

#### sqle/ - SQL Engine Integration

Integration with go-mysql-server:

```
sqle/
├── database.go              # SQL Database implementation
├── database_provider.go     # Database provider
├── tables.go                # SQL Table implementations
├── dtables/                 # System tables (dolt_*)
├── dprocedures/             # Stored procedures
├── dsess/                   # SQL sessions
└── writer/                  # Write operations
```

### 3. Storage Layer (`store/`)

#### NBS - Noms Block Store

Content-addressed storage:

```
nbs/
├── table.go                 # Table file format
├── manifest.go              # Repository manifest
├── store.go                 # Main store implementation
└── chunk_store.go           # Chunk-level operations
```

Key properties:
- Chunks addressed by SHA-1 hash
- Append-only semantics
- Multiprocess concurrency
- Local disk or AWS (S3+DynamoDB) backend

#### Prolly Trees

Modern B+ tree storage:

```
prolly/
├── tuple_map.go             # Key-value map
├── mutable_map_*.go         # Mutable map variants
├── tree/*.go                # Tree operations
└── message/*.go             # Serialization
```

Key operations:
- `Get`, `Put`, `Delete`
- Range queries
- Three-way diff
- Patch-based merging

## Data Model

### RootValue Structure

RootValue is persisted with each commit:

```
RootValue (FlatBuffers)
├── tables: map<TableName, TableHash>
├── foreign_key: ForeignKeyCollection
├── feature_ver: FeatureVersion
├── root_collation_key: Collation
└── root_objects: map<Name, ObjectHash>
```

### Table Structure

```
Table
├── schema: Schema           # Column definitions
├── row_data: ProllyMap      # Primary index
├── indexes: []ProllyMap     # Secondary indexes
└── artifacts: ProllyMap     # Conflicts/violations
```

### Row Storage

Rows stored as FlatBuffers messages:

```
Row Message
├── key: Tuple               # Primary key columns
├── value: Tuple             # Other columns
└── metadata: bytes          # Type info, etc.
```

## Merge Algorithm

### Three-Way Merge Process

```
1. Find merge base (common ancestor)
       ↓
2. Get root values:
   - ours (current branch)
   - theirs (branch to merge)
   - ancestor (merge base)
       ↓
3. For each table:
   a. Compute schema diff
   b. Compute row diff
   c. Apply non-conflicting changes
   d. Record conflicts
       ↓
4. Resolve foreign keys
       ↓
5. Return merged RootValue
```

### Conflict Types

1. **Schema Conflicts**
   - Column added/modified in both branches
   - Incompatible type changes
   - Constraint conflicts

2. **Data Conflicts**
   - Same row modified differently
   - Insert/delete conflicts
   - Unique constraint violations

3. **Constraint Violations**
   - Foreign key violations
   - Unique key violations
   - Check constraint violations

### Conflict Resolution

```bash
# View conflicts
dolt conflicts cat <table>

# Resolve conflicts
dolt conflicts resolve --ours <table>    # Take our version
dolt conflicts resolve --theirs <table>  # Take their version
```

## Version Control Flow

### Commit Creation

```go
// doltdb/doltdb.go
func (ddb *DoltDB) Commit(ctx, root RootValue, parents []*Commit) (*Commit, error)
```

Process:
1. Write RootValue to storage
2. Create commit object with metadata
3. Update branch reference

### Branch Operations

```go
// ref/branch_ref.go
type branchRef struct {
    name: string
    commit: *Commit
}
```

Branches stored in `.dolt/refs/heads/`

## Key Files

| File | Purpose | Size |
|------|---------|------|
| `doltdb/doltdb.go` | Main database operations | 74KB |
| `doltdb/root_val.go` | Root value operations | 44KB |
| `sqle/database.go` | SQL database impl | 103KB |
| `sqle/tables.go` | SQL tables impl | 96KB |
| `merge/merge_prolly_rows.go` | Row merge logic | 76KB |
| `merge/merge_schema.go` | Schema merge logic | 46KB |
| `sqle/database_provider.go` | Database provider | 58KB |

## System Tables

Dolt exposes version control via system tables:

| Table | Purpose |
|-------|---------|
| `dolt_status` | Uncommitted changes |
| `dolt_log` | Commit history |
| `dolt_branches` | Branch info |
| `dolt_remotes` | Remote repositories |
| `dolt_conflicts_*` | Merge conflicts |
| `dolt_diff_*` | Table diffs |
| `dolt_schema_diff` | Schema changes |
| `dolt_statistics` | Table statistics |

## Stored Procedures

| Procedure | Purpose |
|-----------|---------|
| `dolt_add()` | Stage changes |
| `dolt_commit()` | Create commit |
| `dolt_merge()` | Merge branches |
| `dolt_checkout()` | Switch branch |
| `dolt_branch()` | Create/delete branch |
| `dolt_push()` | Push to remote |
| `dolt_pull()` | Pull from remote |
| `dolt_reset()` | Reset state |

## Performance

### Benchmarks (from documentation)

Dolt is ~1.1X slower than MySQL for standard operations.

Key optimization areas:
1. **Prolly trees** - Faster than old format
2. **Chunk caching** - Reduce I/O
3. **Parallel merge** - Multi-threaded operations
4. **Incremental GC** - Background cleanup

## Testing

### Integration Tests

Located in `integration-tests/bats/`:
- SQL correctness tests
- CLI behavior tests
- Version control workflow tests

### Unit Tests

Go tests throughout codebase:
```bash
# Run tests in package
cd go/libraries/doltcore/merge
go test -v
```

## Rust Implementation Considerations

### What to Reuse

1. **Storage Format** - Prolly tree design is well-documented
2. **Merge Algorithm** - Three-way merge is language-agnostic
3. **System Table Design** - SQL interface patterns

### What to Implement

1. **Chunk Store** - Content-addressed storage in Rust
2. **B+ Tree** - Prolly tree equivalent
3. **SQL Integration** - Use sqlparser-rs + custom engine
4. **Wire Protocol** - MySQL protocol crate

### Complexity Estimates

| Component | LOC (Go) | Rust Equivalent |
|-----------|----------|-----------------|
| Core dolt | ~100K | ~80K |
| Storage | ~50K | ~40K |
| Merge | ~20K | ~15K |
| SQL layer | ~50K | ~30K (with sqlparser) |
| **Total** | **~220K** | **~165K** |

## References

- [Dolt README](../../src.dolthub/dolt/README.md)
- [Dolt Documentation](https://docs.dolthub.com/)
- [Architecture](https://docs.dolthub.com/architecture/architecture)
- [AGENT.md](../../src.dolthub/dolt/go/libraries/doltcore/doltdb/AGENT.md)
