---
title: SQLite
prev: 03-wal.md
next: 05-network.md
---

# SQLite Integration

SQLite as Raft state machine.

## Configuration

### PRAGMA Settings

```rust
// hiqlite/src/sqlite/config.rs
pub const SQLITE_PRAGMAS: &[(&str, &str)] = &[
    ("journal_mode", "WAL"),
    ("synchronous", "OFF"),
    ("page_size", "4096"),
    ("journal_size_limit", "16384"),
    ("wal_autocheckpoint", "4000"),
    ("auto_vacuum", "INCREMENTAL"),
    ("foreign_keys", "ON"),
    ("optimize", "0x10002"),
];
```

| PRAGMA | Value | Why |
|--------|-------|-----|
| `journal_mode` | `WAL` | Better concurrency |
| `synchronous` | `OFF` | ~18% speed boost (safe with Raft) |
| `page_size` | `4096` | Modern default |
| `journal_size_limit` | `16MB` | Larger WAL |
| `wal_autocheckpoint` | `4000` | Match 16MB WAL |
| `auto_vacuum` | `INCREMENTAL` | Reduce fragmentation |
| `foreign_keys` | `ON` | Referential integrity |
| `optimize` | `0x10002` | Query optimization |

**Aha:** `synchronous=OFF` is safe because Raft logs provide durability.

## Connection

### Single Writer

```rust
// hiqlite/src/sqlite/connection.rs
pub struct SqliteWriter {
    conn: Connection,
}

impl SqliteWriter {
    pub fn new(data_dir: &Path) -> Result<Self, Error> {
        let path = data_dir.join("hiqlite.db");
        let conn = Connection::open(&path)?;
        
        // Apply PRAGMAs
        for (name, value) in SQLITE_PRAGMAS {
            conn.execute(&format!("PRAGMA {} = {}", name, value), [])?;
        }
        
        Ok(Self { conn })
    }
    
    pub fn execute(&mut self, sql: &str) -> Result<usize, Error> {
        self.conn.execute(sql, [])?
    }
}
```

### Multiple Readers

```rust
// hiqlite/src/sqlite/reader.rs
pub struct SqliteReader {
    conn: Connection,
}

impl SqliteReader {
    pub fn new(data_dir: &Path) -> Result<Self, Error> {
        let path = data_dir.join("hiqlite.db");
        // Read-only connection
        let conn = Connection::open_with_flags(
            &path,
            OpenFlags::SQLITE_OPEN_READ_ONLY,
        )?;
        
        Ok(Self { conn })
    }
    
    pub fn query<T: FromRow>(&self, sql: &str) -> Result<Vec<T>, Error> {
        let mut stmt = self.conn.prepare(sql)?;
        let rows = stmt.query_map([], |row| T::from_row(row))?;
        rows.collect()
    }
}
```

## State Machine

### Applying Log Entries

```rust
// hiqlite/src/state_machine.rs
pub struct SqliteStateMachine {
    writer: SqliteWriter,
    last_applied: u64,
}

impl StateMachine for SqliteStateMachine {
    fn apply(&mut self, entry: &LogEntry) -> Result<(), Error> {
        match entry {
            LogEntry::Execute(sql) => {
                self.writer.execute(sql)?;
            }
            LogEntry::Transaction(sqls) => {
                let tx = self.writer.transaction()?;
                for sql in sqls {
                    tx.execute(sql, [])?;
                }
                tx.commit()?;
            }
        }
        
        self.last_applied = entry.index;
        Ok(())
    }
    
    fn last_applied_index(&self) -> u64 {
        self.last_applied
    }
}
```

## Migrations

### Automatic Migrations

```rust
// hiqlite/src/migration.rs
pub async fn run_migrations(node: &HiqliteNode) -> Result<(), Error> {
    let migrations = load_migrations("./migrations")?;
    
    for migration in migrations {
        if !is_applied(&node, &migration.id)? {
            info!("Applying migration {}", migration.id);
            
            // Execute via Raft (replicated)
            node.execute(&migration.sql).await?;
            
            mark_applied(&node, &migration.id)?;
        }
    }
    
    Ok(())
}
```

### Migration File

```sql
-- migrations/001_initial.sql
-- Migration: 001_initial
-- Date: 2025-01-15

CREATE TABLE users (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    email TEXT UNIQUE NOT NULL,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX idx_users_email ON users(email);
```

## Snapshots

### Creating Snapshots

```rust
// hiqlite/src/snapshot.rs
pub async fn create_snapshot(node: &HiqliteNode) -> Result<PathBuf, Error> {
    let snapshot_id = generate_snapshot_id();
    let snapshot_path = node.data_dir().join(format!("snapshot-{}.db", snapshot_id));
    
    // Backup SQLite
    node.execute(&format!(
        "VACUUM INTO '{}'",
        snapshot_path.to_string_lossy()
    )).await?;
    
    // Compress
    compress(&snapshot_path)?;
    
    // Encrypt with cryptr
    encrypt_file(&snapshot_path, &node.encryption_key())?;
    
    Ok(snapshot_path)
}
```

### Restoring Snapshots

```rust
pub async fn restore_snapshot(
    node: &HiqliteNode,
    snapshot_path: &Path,
) -> Result<(), Error> {
    // Decrypt
    decrypt_file(snapshot_path, &node.encryption_key())?;
    
    // Decompress
    decompress(snapshot_path)?;
    
    // Replace database
    let db_path = node.data_dir().join("hiqlite.db");
    fs::copy(snapshot_path, db_path)?;
    
    // Rebuild from Raft logs after snapshot
    let last_index = get_snapshot_last_index(snapshot_path)?;
    replay_logs(node, last_index).await?;
    
    Ok(())
}
```

## Backup

### Online Backup

```rust
// hiqlite/src/backup.rs
pub async fn backup_to_s3(
    node: &HiqliteNode,
    s3_url: &str,
) -> Result<(), Error> {
    // Create temporary snapshot
    let snapshot = create_snapshot(node).await?;
    
    // Upload to S3
    upload_to_s3(&snapshot, s3_url, &node.encryption_key()).await?;
    
    // Cleanup
    fs::remove_file(&snapshot)?;
    
    Ok(())
}
```

## Recovery

### Crash Recovery

```rust
// hiqlite/src/recovery.rs
pub async fn recover(node: &HiqliteNode) -> Result<(), Error> {
    let lock_path = node.data_dir().join(".db-lock");
    
    if lock_path.exists() {
        // Crash detected
        warn!("Database crash detected, recovering...");
        
        // Delete corrupted database
        let db_path = node.data_dir().join("hiqlite.db");
        fs::remove_file(&db_path)?;
        
        // Find latest snapshot
        let snapshot = find_latest_snapshot(node.data_dir())?;
        
        // Restore from snapshot
        restore_snapshot(node, &snapshot).await?;
        
        info!("Recovery complete");
    }
    
    Ok(())
}
```

**Aha:** With Raft, we can safely use `synchronous=OFF` and recover from crashes.

## Next Steps

Continue to [Network →](05-network.html) for WebSocket networking.
