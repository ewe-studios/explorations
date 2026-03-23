# DoltgreSQL Exploration

## Overview

DoltgreSQL is the PostgreSQL-compatible version of Dolt. It combines Dolt's version control features with PostgreSQL's SQL dialect and wire protocol.

**Status:** Beta (as of 2025)

**Key Features:**
- PostgreSQL wire protocol compatible
- Git-style version control for tables
- SQL-accessible version operations
- Same storage backend as Dolt

## Repository Structure

```
doltgresql/
├── cmd/                       # Command entry points
├── core/                      # Core functionality
├── postgres/                  # PostgreSQL integration
│   └── parser/                # Postgres SQL parser
├── server/                    # Server implementation
├── servercfg/                 # Server configuration
├── utils/                     # Utilities
├── testing/                   # Test utilities
├── flatbuffers/               # Serialization
└── scripts/                   # Build scripts
```

## Architecture

DoltgreSQL shares architecture with Dolt:

```
┌─────────────────────────────────────────────────────────┐
│                    Client (psql, etc.)                   │
└─────────────────────────────────────────────────────────┘
                            │
                            ▼
┌─────────────────────────────────────────────────────────┐
│              Postgres Wire Protocol Server               │
└─────────────────────────────────────────────────────────┘
                            │
                            ▼
┌─────────────────────────────────────────────────────────┐
│              PostgreSQL SQL Parser (AST)                 │
└─────────────────────────────────────────────────────────┘
                            │
                            ▼
┌─────────────────────────────────────────────────────────┐
│           AST → Dolt Query Plan Conversion               │
└─────────────────────────────────────────────────────────┘
                            │
                            ▼
┌─────────────────────────────────────────────────────────┐
│              Dolt SQL Engine (go-mysql-server)           │
│   • Query analyzer                                       │
│   • Execution engine                                     │
│   • Version control operations                           │
└─────────────────────────────────────────────────────────┘
                            │
                            ▼
┌─────────────────────────────────────────────────────────┐
│              Storage Layer (NBS + Prolly)                │
│   • Same as Dolt                                         │
└─────────────────────────────────────────────────────────┘
```

## Key Differences from Dolt

| Feature | Dolt | DoltgreSQL |
|---------|------|------------|
| Wire Protocol | MySQL | PostgreSQL |
| SQL Dialect | MySQL | PostgreSQL |
| Parser | Vitess | Custom Postgres |
| System Tables | `dolt_*` | `dolt_*` |
| CLI | `dolt` | `doltgres` |
| Version Interface | CLI + SQL | SQL only |

## Installation

```bash
# Linux/Mac
sudo bash -c 'curl -L https://github.com/dolthub/doltgresql/releases/latest/download/install.sh | bash'

# Docker
docker run -e DOLTGRES_PASSWORD=myPassword -p 5432:5432 dolthub/doltgresql:latest

# From source
./scripts/build.sh
```

## Getting Started

```bash
# Start server
doltgres

# Connect with psql
PGPASSWORD=password psql -h localhost -U postgres

# Create database
CREATE DATABASE getting_started;
\c getting_started;

# Create tables
CREATE TABLE employees (
    id int8,
    last_name text,
    first_name text,
    PRIMARY KEY(id)
);

-- Make a commit
SELECT * FROM dolt.status;
SELECT dolt_add('employees');
SELECT dolt_commit('-m', 'Created initial schema');

-- View log
SELECT * FROM dolt.log;
```

## Version Control Interface

DoltgreSQL exposes version control **only via SQL** (no Git-style CLI):

### System Tables

```sql
-- Status
SELECT * FROM dolt.status;

-- Log
SELECT * FROM dolt.log;

-- Branches
SELECT * FROM dolt.branches;

-- Diffs
SELECT * FROM dolt.diff_employees;

-- Conflicts
SELECT * FROM dolt.conflicts_employees;
```

### Stored Procedures

```sql
-- Stage changes
SELECT dolt_add('employees');

-- Commit
SELECT dolt_commit('-m', 'commit message');

-- Branch operations
SELECT dolt_branch('feature-branch');
SELECT dolt_checkout('feature-branch');
SELECT dolt_merge('feature-branch');
```

## SQL Dialect Differences

### PostgreSQL Types Supported

- `int8`, `int4`, `int2` - Integer types
- `text`, `varchar` - String types
- `bool` - Boolean
- `timestamp`, `date`, `time` - Temporal types
- `json`, `jsonb` - JSON types
- Arrays and more...

### MySQL vs Postgres Syntax

```sql
-- MySQL (Dolt)
SELECT * FROM table LIMIT 10;
AUTO_INCREMENT primary key

-- PostgreSQL (DoltgreSQL)
SELECT * FROM table LIMIT 10;
GENERATED ALWAYS AS IDENTITY primary key
```

## Limitations

### Current Limitations (Beta)

1. **No Git-style CLI** - Only SQL interface for version control
2. **Limited Push/Pull** - Can't push to DoltHub yet
3. **No Extensions** - PostgreSQL extensions not supported
4. **GSSAPI** - Not supported
5. **Backup/Replication** - Work in progress

### Syntax Not Yet Implemented

Some PostgreSQL syntax, functions, and features are still being implemented.

## Performance

### Benchmark Comparison (v0.50.0)

**Read Operations (multiple of Postgres):**
| Test | Postgres | DoltgreSQL | Multiple |
|------|----------|------------|----------|
| oltp_point_select | 0.14ms | 0.52ms | 3.7x |
| oltp_read_only | 2.48ms | 12.75ms | 5.1x |
| index_scan | 17.95ms | 130.13ms | 7.2x |
| **Read Average** | | | **6.3x** |

**Write Operations (multiple of Postgres):**
| Test | Postgres | DoltgreSQL | Multiple |
|------|----------|------------|----------|
| oltp_insert | 1.1ms | 3.68ms | 3.3x |
| oltp_read_write | 4.25ms | 20.37ms | 4.8x |
| oltp_update_index | 1.12ms | 3.55ms | 3.2x |
| **Write Average** | | | **3.6x** |

**Overall:** ~5.2x slower than PostgreSQL

### Correctness

SQLLogicTest results (v0.50.0):
- Total Tests: 5,691,305
- Passed: 5,188,604 (91.2%)
- Failed: 411,415
- Timeout: 16

## Storage Format

DoltgreSQL uses the **same storage format as Dolt**:

### RootValue

```
RootValue (FlatBuffers)
├── tables: map<TableName, TableHash>
├── foreign_keys: ForeignKeyCollection
├── feature_version: int64
└── collation: Collation
```

### Table Storage

```
Table
├── schema: Schema           -- PostgreSQL types
├── row_data: ProllyMap      -- Primary index
├── indexes: []ProllyMap     -- Secondary indexes
└── artifacts: ProllyMap     -- Conflicts
```

### Chunk Storage

Content-addressed via NBS:
- SHA-1 hashed chunks
- Append-only semantics
- Local disk or S3 backend

## Parser Architecture

DoltgreSQL includes a custom PostgreSQL parser:

```
postgres/parser/
├── parser.go                  -- Main parser
├── ast/                       -- AST node types
├── lex/                       -- Lexer
└── transform/                 -- AST transformation
```

The parser:
1. Tokenizes PostgreSQL SQL
2. Produces PostgreSQL AST
3. AST converted to Dolt plan
4. Executed by go-mysql-server

## Key Source Files

| File | Purpose |
|------|---------|
| `server/server.go` | Main server implementation |
| `postgres/parser/*.go` | SQL parsing |
| `core/context.go` | Session context |
| `servercfg/config.go` | Configuration |

## Rust Implementation Considerations

### What DoltgreSQL Teaches Us

1. **Protocol Separation** - Wire protocol is separate from storage
2. **Parser Flexibility** - Can swap MySQL parser for Postgres
3. **Shared Backend** - Same version control layer works for both

### Rust Implementation Path

For a Rust implementation:

```
1. Choose wire protocol:
   - tokio-postgres (for Postgres)
   - mysql_async (for MySQL)

2. Use sqlparser-rs for SQL parsing:
   - Supports both MySQL and Postgres dialects

3. Implement storage layer:
   - Content-addressed chunk store
   - Prolly tree B+ trees

4. Build version control:
   - Three-way merge
   - Branch/commit management
```

### Crates to Leverage

```toml
[dependencies]
# SQL
sqlparser = "0.43"        # SQL parsing
tokio-postgres = "0.7"    # Postgres protocol
mysql_async = "0.33"      # MySQL protocol

# Storage
bytes = "1.5"             # Buffer management
tokio = "1.35"            # Async runtime

# Serialization
flatbuffers = "23"        # Same as Dolt

# Optional storage backends
rocksdb = "0.22"          # Alternative to NBS
```

## Testing

DoltgreSQL uses the same test harness as Dolt:

```bash
# Run tests
cd doltgresql
go test ./...

# SQL logic tests
cd doltgresql/testing
```

## References

- [DoltgreSQL README](../../src.dolthub/doltgresql/README.md)
- [DoltgreSQL Documentation](https://docs.doltgres.com/)
- [Dolt Architecture](https://docs.dolthub.com/architecture/architecture)
