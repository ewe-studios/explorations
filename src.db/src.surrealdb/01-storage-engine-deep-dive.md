# SurrealDB: Storage Engine Deep Dive

## Overview

This document explores SurrealDB's storage architecture:
- Pluggable storage backends
- Key-value storage layer
- Document and graph storage
- Index implementations

---

## 1. Pluggable Storage Architecture

### Storage Traits

```rust
/// Core storage trait
pub trait Datastore: Send + Sync {
    /// Begin a transaction
    fn transaction(&self, write: bool, lock: bool) -> Transaction;

    /// Check if datastore supports transactions
    fn supports_transactions(&self) -> bool;

    /// Check if datastore supports locking
    fn supports_locking(&self) -> bool;
}

/// Transaction trait
pub trait Transaction: Send + Sync {
    /// Get a key
    async fn get(&mut self, key: Key) -> Result<Option<Value>>;

    /// Put a key-value pair
    async fn put(&mut self, key: Key, value: Value) -> Result<()>;

    /// Delete a key
    async fn del(&mut self, key: Key) -> Result<()>;

    /// Scan a range of keys
    async fn scan(&mut self, range: Range<Key>) -> Result<Vec<(Key, Value)>>;

    /// Commit transaction
    async fn commit(&mut self) -> Result<()>;

    /// Rollback transaction
    async fn rollback(&mut self) -> Result<()>;
}
```

### Storage Backends

```rust
// Memory storage
pub struct MemStore {
    data: Arc<Mutex<BTreeMap<Vec<u8>, Vec<u8>>>>,
}

// RocksDB storage
pub struct RocksDBStore {
    db: Arc<rocksdb::DB>,
}

// TiKV storage
pub struct TiKVStore {
    client: Arc<tikv_client::TransactionClient>,
}

// FoundationDB storage
pub struct FdbStore {
    database: Arc<foundationdb::Database>,
}
```

---

## 2. Key-Value Storage Layer

### Key Encoding

```rust
/// Key structure for storage
pub struct Key {
    /// Namespace
    pub ns: Vec<u8>,

    /// Database
    pub db: Vec<u8>,

    /// Table
    pub tb: Vec<u8>,

    /// Record ID (optional)
    pub id: Option<Vec<u8>>,

    /// Key type prefix
    pub key_type: KeyType,
}

#[derive(Clone, Copy)]
pub enum KeyType {
    All,        ///: All records
    Doc,        ///: Document data
    Index,      ///: Index data
    Edge,       ///: Edge data
    Seq,        ///: Sequence counter
}

impl Key {
    /// Encode key to bytes
    pub fn encode(&self) -> Vec<u8> {
        let mut bytes = Vec::new();

        // Prefix
        bytes.push(self.key_type as u8);

        // Namespace
        bytes.extend_from_slice(&self.ns);
        bytes.push(0x00);

        // Database
        bytes.extend_from_slice(&self.db);
        bytes.push(0x00);

        // Table
        bytes.extend_from_slice(&self.tb);
        bytes.push(0x00);

        // ID if present
        if let Some(id) = &self.id {
            bytes.extend_from_slice(id);
        }

        bytes
    }

    /// Decode key from bytes
    pub fn decode(bytes: &[u8]) -> Result<Self> {
        let mut cursor = 0;

        // Read prefix
        let key_type = KeyType::from_u8(bytes[cursor])?;
        cursor += 1;

        // Parse components...
        // (implementation details)

        Ok(Key { ns, db, tb, id, key_type })
    }
}
```

### Value Encoding

```rust
/// Value encoding for storage
pub enum Value {
    /// Document data
    Document {
        id: RecordId,
        data: Map<String, Value>,
    },

    /// Edge data
    Edge {
        id: RecordId,
        from: RecordId,
        to: RecordId,
        data: Map<String, Value>,
    },

    /// Index entry
    Index {
        key: DbValue,
        rid: RecordId,
    },

    /// Sequence counter
    Seq(u64),
}

impl Value {
    /// Serialize value to bytes
    pub fn encode(&self) -> Vec<u8> {
        match self {
            Value::Document { id, data } => {
                let mut bytes = vec![0x01];  // Document prefix
                bytes.extend_from_slice(&id.encode());
                bytes.extend_from_slice(&data.encode());
                bytes
            }
            Value::Edge { id, from, to, data } => {
                let mut bytes = vec![0x02];  // Edge prefix
                bytes.extend_from_slice(&id.encode());
                bytes.extend_from_slice(&from.encode());
                bytes.extend_from_slice(&to.encode());
                bytes.extend_from_slice(&data.encode());
                bytes
            }
            _ => unimplemented!(),
        }
    }
}
```

---

## 3. Document Storage

### Document Layout

```
Document Storage Format:
┌─────────────────────────────────────────────────────┐
│ Header (8 bytes)                                    │
│ - Version (1 byte)                                  │
│ - Flags (1 byte)                                    │
│ - Data length (6 bytes)                             │
├─────────────────────────────────────────────────────┤
│ Record ID (variable)                                │
│ - Table name (length-prefixed)                      │
│ - ID value (type + data)                            │
├─────────────────────────────────────────────────────┤
│ Field Data (variable)                               │
│ - Field 1: name + type + value                      │
│ - Field 2: name + type + value                      │
│ - ...                                               │
└─────────────────────────────────────────────────────┘
```

### Document Operations

```rust
impl Document {
    /// Create new document
    pub fn new(table: String, id: Id) -> Self {
        Self {
            rid: RecordId { table, id },
            data: Map::new(),
        }
    }

    /// Set field value
    pub fn set(&mut self, path: &str, value: Value) {
        self.data.insert(path.into(), value);
    }

    /// Get field value
    pub fn get(&self, path: &str) -> Option<&Value> {
        self.data.get(path)
    }

    /// Store document
    pub async fn store(&self, txn: &mut Transaction) -> Result<()> {
        let key = self.key_encode();
        let value = self.encode();
        txn.put(key, value).await
    }

    /// Load document
    pub async fn load(txn: &mut Transaction, rid: RecordId) -> Result<Option<Self>> {
        let key = rid.key_encode();
        match txn.get(key).await? {
            Some(bytes) => Ok(Some(Self::decode(&bytes)?)),
            None => Ok(None),
        }
    }
}
```

---

## 4. Graph Storage

### Edge Representation

```rust
/// Edge structure
pub struct Edge {
    /// Edge ID
    pub id: RecordId,

    /// Source record
    pub from: RecordId,

    /// Target record
    pub to: RecordId,

    /// Edge properties
    pub data: Map<String, Value>,
}

/// Edge key encoding
impl Edge {
    /// Encode edge key for storage
    pub fn key_encode(&self) -> Key {
        Key {
            ns: self.from.ns.clone(),
            db: self.from.db.clone(),
            tb: self.from.table.clone(),
            id: Some(self.encode_direction(Direction::Out)),
            key_type: KeyType::Edge,
        }
    }

    fn encode_direction(&self, dir: Direction) -> Vec<u8> {
        match dir {
            Direction::Out => {
                // Outgoing edge: from -> to
                let mut bytes = self.to.encode();
                bytes.insert(0, 0x00);  // Out prefix
                bytes
            }
            Direction::In => {
                // Incoming edge: from <- to
                let mut bytes = self.from.encode();
                bytes.insert(0, 0x01);  // In prefix
                bytes
            }
        }
    }
}
```

### Graph Traversal

```rust
impl Graph {
    /// Traverse outgoing edges
    pub async fn traverse_out(
        txn: &mut Transaction,
        from: RecordId,
        edge_type: Option<&str>,
    ) -> Result<Vec<Edge>> {
        let prefix = from.edge_prefix(Direction::Out);

        // Scan for outgoing edges
        let edges = txn.scan(prefix.range()).await?;

        // Filter by edge type if specified
        let result = edges
            .into_iter()
            .filter(|(key, _)| {
                edge_type.map_or(true, |t| key.matches_edge_type(t))
            })
            .map(|(_, value)| Edge::decode(&value))
            .collect();

        Ok(result)
    }

    /// Traverse incoming edges
    pub async fn traverse_in(
        txn: &mut Transaction,
        to: RecordId,
        edge_type: Option<&str>,
    ) -> Result<Vec<Edge>> {
        Self::traverse_out(txn, to, edge_type).await
    }

    /// Multi-hop traversal
    pub async fn traverse_hops(
        txn: &mut Transaction,
        start: RecordId,
        hops: usize,
        direction: Direction,
    ) -> Result<Vec<RecordId>> {
        let mut visited = HashSet::new();
        let mut current = vec![start];
        let mut results = Vec::new();

        for _ in 0..hops {
            let mut next = Vec::new();

            for rid in current {
                if visited.contains(&rid) {
                    continue;
                }
                visited.insert(rid);

                let edges = match direction {
                    Direction::Out => Self::traverse_out(txn, rid, None).await?,
                    Direction::In => Self::traverse_in(txn, rid, None).await?,
                };

                for edge in edges {
                    let next_rid = match direction {
                        Direction::Out => edge.to,
                        Direction::In => edge.from,
                    };
                    next.push(next_rid);
                }
            }

            results.extend(&next);
            current = next;
        }

        Ok(results)
    }
}
```

---

## 5. Index Implementations

### Index Structure

```rust
/// Index definition
pub struct Index {
    pub name: String,
    pub table: String,
    pub columns: Vec<String>,
    pub unique: bool,
    pub index_type: IndexType,
}

#[derive(Clone)]
pub enum IndexType {
    /// B-Tree index for range queries
    BTree,

    /// Hash index for equality
    Hash,

    /// Full-text index
    FullText {
        highlight: bool,
        snippet: bool,
    },
}
```

### B-Tree Index

```rust
/// B-Tree index implementation
pub struct BTreeIndex {
    /// Index entries: (key, record_id)
    entries: BTreeMap<Vec<u8>, Vec<RecordId>>,
}

impl BTreeIndex {
    /// Insert into index
    pub fn insert(&mut self, key: DbValue, rid: RecordId) {
        let key_bytes = key.encode();
        self.entries.entry(key_bytes).or_default().push(rid);
    }

    /// Remove from index
    pub fn remove(&mut self, key: DbValue, rid: RecordId) {
        let key_bytes = key.encode();
        if let Some(rids) = self.entries.get_mut(&key_bytes) {
            ridids.retain(|r| r != &rid);
            if rids.is_empty() {
                self.entries.remove(&key_bytes);
            }
        }
    }

    /// Point lookup
    pub fn lookup(&self, key: DbValue) -> Vec<RecordId> {
        let key_bytes = key.encode();
        self.entries.get(&key_bytes).cloned().unwrap_or_default()
    }

    /// Range scan
    pub fn range(&self, start: DbValue, end: DbValue) -> Vec<RecordId> {
        let start_bytes = start.encode();
        let end_bytes = end.encode();

        self.entries
            .range((Bound::Included(start_bytes), Bound::Excluded(end_bytes)))
            .flat_map(|(_, rids)| rids.iter())
            .cloned()
            .collect()
    }
}
```

---

## Document History

| Date | Change |
|------|--------|
| 2026-03-27 | Initial storage engine deep dive created |

---

*This exploration is a living document. Revisit sections as concepts become clearer through implementation.*
