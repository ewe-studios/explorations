# Dolthub Project Exploration

## Overview

Dolthub is the organization behind **Dolt** - "Git for Data" - a SQL database that you can fork, clone, branch, merge, push and pull just like a Git repository. The project provides version control for structured data at the table level.

**Key Products:**
- **Dolt** - MySQL-compatible versioned database
- **DoltgreSQL** - PostgreSQL-compatible versioned database (Beta)
- **go-mysql-server** - SQL database engine framework
- **DoltHub** - Hosting platform for Dolt databases
- **DoltLab** - Enterprise self-hosted DoltHub

## Repository Structure

```
src.dolthub/
├── dolt/                    # Main Dolt database project
│   ├── go/                  # Go source code
│   │   ├── cmd/dolt/        # CLI commands
│   │   ├── libraries/       # Core libraries
│   │   │   └── doltcore/    # Core Dolt functionality
│   │   ├── store/           # Storage layer (NBS, prolly)
│   │   └── gen/             # Generated code (FlatBuffers)
│   └── integration-tests/   # Bats integration tests
├── go-mysql-server/         # SQL engine framework
│   ├── sql/                 # Core SQL types and interfaces
│   ├── server/              # MySQL wire protocol server
│   ├── enginetest/          # Test harness
│   └── memory/              # In-memory backend example
├── doltgresql/              # PostgreSQL-compatible Dolt
│   ├── postgres/            # Postgres parser
│   ├── server/              # Server implementation
│   └── core/                # Core functionality
├── CommitGraph/             # React component for commit visualization
├── driver/                  # Database drivers
├── dolt-workbench/          # Web UI for Dolt
├── dolt-mcp/                # Model Context Protocol integration
├── lambdabats/              # Lambda test infrastructure
└── eventsapi_schema/        # Event API schemas
```

## Core Concepts

### Version Control for Tables

Dolt treats tables like Git treats files:
- **Commits** - Snapshot of all tables at a point in time
- **Branches** - Independent lines of development
- **Merges** - Combine changes from different branches
- **Diffs** - Show changes between commits or branches

### Storage Architecture

Dolt uses a content-addressed storage model:
1. **NBS (Noms Block Store)** - Low-level storage backend
2. **Prolly Trees** - Probabilistic B+ trees for indexes
3. **FlatBuffers** - Serialization format for row data

### Key Features

1. **Git-style CLI** - All familiar Git commands work with Dolt
2. **SQL Interface** - Version control via system tables and procedures
3. **MySQL Compatibility** - Works with existing MySQL clients
4. **Branching/Merging** - Three-way merge with conflict detection
5. **Data Lineage** - Full history of all data changes

## Sub-Projects

### go-mysql-server

The SQL database engine that powers Dolt. Provides:
- SQL parsing (via Vitess)
- Query planning and optimization
- Execution engine
- MySQL wire protocol server

**See:** [go-mysql-server-exploration.md](./go-mysql-server-exploration.md)

### dolt

The main Dolt database implementation:
- Version control logic
- Storage format implementation
- Merge algorithms
- CLI and SQL server

**See:** [dolt-exploration.md](./dolt-exploration.md)

### doltgresql

PostgreSQL-compatible version of Dolt:
- Postgres SQL parser
- Postgres wire protocol
- Same version control backend as Dolt

**See:** [doltgresql-exploration.md](./doltgresql-exploration.md)

### CommitGraph

React component for visualizing commit histories:
- Interactive commit graph
- Infinite scroll support
- Used by DoltHub for repository views

**See:** [CommitGraph/](../../src.dolthub/CommitGraph/)

## Storage Deep Dive

Dolt's storage is built on several key innovations:

### NBS (Noms Block Store)

A content-addressed object store:
- Chunks are addressed by their SHA-1 hash
- Optimized for immutable data with append-only semantics
- Supports both local disk and AWS (S3+DynamoDB) backends
- Multiprocess concurrency with optimistic locking

### Prolly Trees

The modern storage format for Dolt data:
- Probabilistically balanced B+ trees
- Key-value storage with ordered keys
- Efficient for range queries and point lookups
- Supports three-way diff and merge operations

### RootValue

The entry point for database state:
- Each commit points to a RootValue
- RootValue contains all tables and metadata
- Stored as FlatBuffers messages
- Feature versioned for compatibility

**See:** [versioned-storage-deep-dive.md](./versioned-storage-deep-dive.md)

## Version Control Algorithm

### Three-Way Merge

Dolt uses standard three-way merge:
1. Find common ancestor (merge base)
2. Compute diffs: ancestor→ours, ancestor→theirs
3. Apply non-conflicting changes
4. Detect and record conflicts

### Merge Conflicts

Conflicts are tracked in system tables:
- `dolt_conflicts_<table>` - Conflict details
- `dolt_constraint_violations_<table>` - FK/unique violations
- Manual resolution via SQL or CLI

## Rust Reproduction Potential

Reproducing Dolt's functionality in Rust is feasible:

### Components to Implement

1. **Storage Layer**
   - Content-addressed chunk store
   - Prolly tree B+ tree implementation
   - FlatBuffers serialization

2. **SQL Engine**
   - SQL parser (use sqlparser-rs or similar)
   - Query planner and executor
   - Type system and expressions

3. **Version Control**
   - Commit graph management
   - Three-way diff algorithm
   - Merge conflict detection

4. **Wire Protocol**
   - MySQL/Postgres protocol implementation
   - Authentication and sessions

### Existing Rust Projects

Consider leveraging:
- `sqlparser-rs` - SQL parsing
- `bytes` - Buffer management
- `tokio` - Async runtime
- `flatbuffers` - Serialization
- `rocksdb` / `sled` - Optional storage backend

**See:** [rust-revision.md](./rust-revision.md)

## References

- [Dolt Documentation](https://docs.dolthub.com/)
- [DoltHub](https://www.dolthub.com/)
- [DoltgreSQL](https://www.doltgres.com/)
- [Architecture Overview](https://docs.dolthub.com/architecture/architecture)
