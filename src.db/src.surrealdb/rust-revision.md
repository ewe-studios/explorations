# SurrealDB: Rust Revision - Translation Guide

**Source:** `/home/darkvoid/Boxxed/@formulas/src.rust/src.db/src.surrealdb/surrealdb`
**Target:** Rust with valtron executor (no async/await, no tokio)

---

## 1. Overview

SurrealDB is a multi-model database with:
- SurrealQL query language
- Pluggable storage backends
- Graph traversal capabilities
- Document storage

---

## 2. Core Types Translation

### Database Value

```rust
#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    None,
    Null,
    Bool(bool),
    Int(i64),
    Float(f64),
    String(String),
    Array(Vec<Value>),
    Object(Map<String, Value>),
    Geometry(Geometry),
    Duration(Duration),
    Datetime(Datetime),
}
```

### Record ID

```rust
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct RecordId {
    pub table: String,
    pub id: Id,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Id {
    Number(u64),
    String(String),
    Uuid(Uuid),
}
```

---

## 3. Storage Translation

### Sync Transaction Trait

```rust
/// Synchronous transaction (no async)
pub trait Transaction: Send + Sync {
    /// Get value
    fn get(&mut self, key: &[u8]) -> Result<Option<Vec<u8>>>;

    /// Put value
    fn put(&mut self, key: &[u8], value: Vec<u8>) -> Result<()>;

    /// Delete key
    fn del(&mut self, key: &[u8]) -> Result<()>;

    /// Scan range
    fn scan(&mut self, start: &[u8], end: &[u8]) -> Result<Vec<(Vec<u8>, Vec<u8>)>>;

    /// Commit
    fn commit(&mut self) -> Result<()>;

    /// Rollback
    fn rollback(&mut self) -> Result<()>;
}
```

### Memory Storage Implementation

```rust
use std::collections::BTreeMap;
use std::cell::RefCell;

pub struct MemTransaction {
    data: Rc<RefCell<BTreeMap<Vec<u8>, Vec<u8>>>>,
    snapshot: BTreeMap<Vec<u8>, Vec<u8>>,
    writes: BTreeMap<Vec<u8>, Option<Vec<u8>>>,
}

impl MemTransaction {
    pub fn new(data: Rc<RefCell<BTreeMap<Vec<u8>, Vec<u8>>>>) -> Self {
        let snapshot = data.borrow().clone();
        Self { data, snapshot, writes: BTreeMap::new() }
    }
}

impl Transaction for MemTransaction {
    fn get(&mut self, key: &[u8]) -> Result<Option<Vec<u8>>> {
        // Check writes first
        if let Some(value) = self.writes.get(key) {
            return Ok(value.clone());
        }
        // Then snapshot
        Ok(self.snapshot.get(key).cloned())
    }

    fn put(&mut self, key: &[u8], value: Vec<u8>) -> Result<()> {
        self.writes.insert(key.to_vec(), Some(value));
        Ok(())
    }

    fn del(&mut self, key: &[u8]) -> Result<()> {
        self.writes.insert(key.to_vec(), None);
        Ok(())
    }

    fn scan(&mut self, start: &[u8], end: &[u8]) -> Result<Vec<(Vec<u8>, Vec<u8>)>> {
        let mut results = Vec::new();

        // Combine snapshot and writes
        let all_keys: BTreeSet<_> = self.snapshot.keys()
            .chain(self.writes.keys())
            .collect();

        for key in all_keys {
            if key >= start && key < end {
                if let Some(value) = self.get(key) ? {
                    results.push((key.clone(), value));
                }
            }
        }

        Ok(results)
    }

    fn commit(&mut self) -> Result<()> {
        let mut data = self.data.borrow_mut();
        for (key, value) in self.writes.drain() {
            match value {
                Some(v) => data.insert(key, v),
                None => data.remove(&key),
            };
        }
        Ok(())
    }

    fn rollback(&mut self) -> Result<()> {
        self.writes.clear();
        Ok(())
    }
}
```

---

## 4. Query Execution without Async

### TaskIterator Pattern

```rust
pub trait TaskIterator {
    type Ready;
    type Pending;
    type Spawner;
    type Error;

    fn next(&mut self) -> Option<TaskStatus<Self::Ready, Self::Pending, Self::Spawner, Self::Error>>;
}

pub enum TaskStatus<Ready, Pending, Spawner, Error> {
    Ready(Result<Ready, Error>),
    Pending(Pending),
    Spawned(Spawner),
    Done,
}
```

### Query as TaskIterator

```rust
pub struct QueryTask {
    plan: LogicalPlan,
    state: QueryState,
}

enum QueryState {
    Initial,
    Scanning { scan_iter: ScanIterator },
    Filtering { rows: Vec<Row>, current: usize },
    Complete,
}

impl TaskIterator for QueryTask {
    type Ready = Vec<Row>;
    type Pending = ();
    type Spawner = ();
    type Error = QueryError;

    fn next(&mut self) -> Option<TaskStatus<Self::Ready, Self::Pending, Self::Spawner, Self::Error>> {
        match &mut self.state {
            QueryState::Initial => {
                // Start scanning
                self.state = QueryState::Scanning {
                    scan_iter: ScanIterator::new(&self.plan),
                };
                self.next()
            }

            QueryState::Scanning { scan_iter } => {
                let mut rows = Vec::new();

                // Scan batch of rows
                for _ in 0..100 {
                    match scan_iter.next() {
                        Some(row) => rows.push(row),
                        None => {
                            self.state = QueryState::Complete;
                            return Some(TaskStatus::Ready(Ok(rows)));
                        }
                    }
                }

                Some(TaskStatus::Pending(()))
            }

            QueryState::Complete => Some(TaskStatus::Done),
        }
    }
}
```

---

## Document History

| Date | Change |
|------|--------|
| 2026-03-27 | Initial Rust revision guide created |

---

*This exploration is a living document.*
