---
title: "Incremental Builds Deep Dive: Caching Strategies and Fingerprinting"
subtitle: "Understanding how Pavex achieves fast rebuilds through intelligent caching"
based_on: "pavexc crate - libs/pavexc/src/rustdoc/compute/ and libs/pavexc/src/compiler/"
level: "Intermediate - Requires understanding of build systems"
---

# Incremental Builds Deep Dive

## Overview

This deep dive explores how Pavex achieves fast incremental builds through intelligent caching strategies, fingerprinting, and cache invalidation. We'll examine the SQLite cache system, BLAKE3 checksumming, and the project access log that enables efficient cleanup.

---

## 1. Incremental Build Fundamentals

### 1.1 The Incremental Build Problem

**Full rebuild:**
```
Change one line in src/lib.rs
         │
         ▼
┌─────────────────┐
│ Recompile EVERY │
│ - All crates    │
│ - All files     │
│ - All analysis  │
└─────────────────┘
         │
         ▼
    10+ minutes
```

**Incremental rebuild:**
```
Change one line in src/lib.rs
         │
         ▼
┌─────────────────┐
│ Recompile ONLY  │
│ - Changed crate │
│ - Dependents    │
│ - Cached rest   │
└─────────────────┘
         │
         ▼
    30 seconds
```

### 1.2 Levels of Incrementality

| Level | What's Cached | Invalidation Trigger |
|-------|---------------|---------------------|
| **File** | Compiled objects (.o) | File modification time |
| **Crate** | rlib files | Crate source changed |
| **Analysis** | rustdoc JSON, dependency graph | Source or features changed |
| **Generated** | Generated SDK code | Blueprint changed |

Pavex operates at **analysis** and **generated** levels.

---

## 2. Caching Architecture

### 2.1 Cache Layers

```
┌─────────────────────────────────────────────────────────┐
│              Pavex Cache Hierarchy                       │
│                                                          │
│  ┌─────────────────────────────────────────────────┐    │
│  │  L1: In-Memory (during single run)              │    │
│  │  - Interned strings                             │    │
│  │  - Computed call graphs                         │    │
│  └─────────────────────────────────────────────────┘    │
│                          │                               │
│                          ▼                               │
│  ┌─────────────────────────────────────────────────┐    │
│  │  L2: Per-Project (.pavex/)                      │    │
│  │  - Generated SDK                                │    │
│  │  - Blueprint fingerprint                        │    │
│  │  - Last build metadata                          │    │
│  └─────────────────────────────────────────────────┘    │
│                          │                               │
│                          ▼                               │
│  ┌─────────────────────────────────────────────────┐    │
│  │  L3: Global (~/.pavex/rustdoc/cache/)           │    │
│  │  - rustdoc JSON (SQLite)                        │    │
│  │  - Third-party crate docs                       │    │
│  │  - Toolchain crate docs                         │    │
│  └─────────────────────────────────────────────────┘    │
└─────────────────────────────────────────────────────────┘
```

### 2.2 Cache Storage Comparison

| Storage | Access Time | Persistence | Use Case |
|---------|-------------|-------------|----------|
| Memory (HashMap) | ~100ns | Process lifetime | Intermediate results |
| File (JSON) | ~1ms | Permanent | Generated code |
| SQLite (BLOB) | ~500μs | Permanent | rustdoc JSON |
| Memory-mapped | ~10μs | Permanent | Large binary blobs |

Pavex uses **SQLite** for rustdoc JSON because:
- Efficient partial reads (don't deserialize entire cache)
- Transaction safety (concurrent builds)
- Structured queries (cache lookup by multiple keys)

---

## 3. Fingerprinting Strategies

### 3.1 What Is a Fingerprint?

A **fingerprint** is a hash that uniquely identifies the state of something.

```
Source Code + Features + Toolchain
      │           │          │
      ▼           ▼          ▼
  BLAKE3      Sorted     cargo --version
  Hash        List
      │           │          │
      └───────────┴──────────┘
              │
              ▼
       Combined Fingerprint
       (cache lookup key)
```

### 3.2 Fingerprint Components

Pavex fingerprints include:

| Component | Source | Why It Matters |
|-----------|--------|----------------|
| **crate_hash** | BLAKE3 of all .rs files | Detects source changes |
| **cargo_fingerprint** | `cargo --verbose --version` | Detects toolchain changes |
| **rustdoc_options** | Serialized flags | Different flags = different output |
| **active_named_features** | Sorted feature list | Features affect visible items |
| **default_feature_is_enabled** | Boolean | Base feature affects output |

### 3.3 BLAKE3 Checksumming

```rust
// libs/pavexc/src/rustdoc/compute/checksum.rs
use blake3::{Hasher, hash};
use std::path::Path;

/// Compute a BLAKE3 hash of all source files in a crate.
pub fn checksum_crate(package_path: &Path) -> Result<String, Error> {
    let mut hasher = Hasher::new();

    // Hash all Rust source files
    let rs_files = globwalk::glob_walk(package_path, "**/*.rs")?;
    for entry in rs_files {
        let content = fs_err::read(entry.path())
            .with_context(|| format!("Failed to read {}", entry.path().display()))?;

        // Include relative path in hash (file location matters)
        let rel_path = entry.path()
            .strip_prefix(package_path)?
            .to_string_lossy();
        hasher.update(rel_path.as_bytes());

        // Hash file contents
        hasher.update(&content);
    }

    // Hash Cargo.toml (dependencies affect type resolution)
    let cargo_toml = package_path.join("Cargo.toml");
    if cargo_toml.exists() {
        let content = fs_err::read(&cargo_toml)?;
        hasher.update(&content);
    }

    // Hash build.rs if present (affects compilation)
    let build_rs = package_path.join("build.rs");
    if build_rs.exists() {
        let content = fs_err::read(&build_rs)?;
        hasher.update(&content);
    }

    Ok(hasher.finalize().to_hex().to_string())
}
```

### 3.4 Why BLAKE3?

| Algorithm | Speed | Collision Resistance | Use Case |
|-----------|-------|---------------------|----------|
| **BLAKE3** | ~4 GB/s | 256-bit | Pavex crate hashing |
| SHA-256 | ~500 MB/s | 256-bit | Security-critical |
| xxHash | ~20 GB/s | 64-bit | Quick checksums |
| FNV-1a | ~10 GB/s | 64-bit | Hash maps |

BLAKE3 is:
- **Fast** - Faster than SHA-256, close to xxHash
- **Secure** - 256-bit output, cryptographically sound
- **Parallel** - Can hash large files in parallel

---

## 4. Cache Invalidation Logic

### 4.1 When to Invalidate

```rust
// libs/pavexc/src/rustdoc/compute/cache.rs
fn should_invalidate_cache(
    cached: &CachedEntry,
    current: &CurrentState,
) -> bool {
    // Any mismatch = cache miss
    cached.crate_name != current.crate_name
        || cached.crate_version != current.crate_version
        || cached.crate_source != current.crate_source
        || cached.crate_hash != current.crate_hash
        || cached.cargo_fingerprint != current.cargo_fingerprint
        || cached.rustdoc_options != current.rustdoc_options
        || cached.default_feature_is_enabled != current.default_feature_is_enabled
        || cached.active_named_features != current.active_named_features
}
```

### 4.2 Invalidation Scenarios

| Scenario | Cache Hit? | Why |
|----------|------------|-----|
| Source file changed | ❌ Miss | crate_hash differs |
| Feature flag added | ❌ Miss | active_named_features differs |
| Toolchain updated | ❌ Miss | cargo_fingerprint differs |
| Same crate, different project | ✅ Hit | All keys match |
| Path dependency moved | ❌ Miss | Path in crate_source differs |

### 4.3 Granular Invalidation

Pavex invalidates at the **crate level**, not project level:

```
Project A depends on: serde 1.0, tokio 1.0
Project B depends on: serde 1.0, reqwest 0.11

Change in Project A:
  - Invalidate: Project A's blueprint
  - Keep: serde 1.0 cache (still valid)
  - Keep: tokio 1.0 cache (still valid)
  - Unused: reqwest 0.11 cache (not touched)
```

---

## 5. Project Access Log

### 5.1 Purpose

The access log tracks which packages each project uses:

```sql
-- Schema
CREATE TABLE project2package_id_access_log (
    project_fingerprint TEXT PRIMARY KEY,  -- Hash of project path
    package_ids BLOB NOT NULL              -- bincode-encoded Vec<PackageId>
);
```

### 5.2 Usage

```rust
// libs/pavexc/src/rustdoc/compute/cache.rs

/// Record which packages were accessed during this build.
pub fn persist_access_log(
    &self,
    package_ids: &BTreeSet<PackageId>,
    project_fingerprint: &str,
) -> Result<(), Error> {
    let connection = self.connection_pool.get()?;

    // Upsert: insert or update on conflict
    let mut stmt = connection.prepare_cached(
        "INSERT INTO project2package_id_access_log
         (project_fingerprint, package_ids)
         VALUES (?, ?)
         ON CONFLICT(project_fingerprint) DO UPDATE
         SET package_ids = excluded.package_ids",
    )?;

    // Serialize package IDs as strings
    let package_ids_bytes = bincode::encode_to_vec(
        package_ids.iter().map(|id| id.repr()).collect_vec(),
        BINCODE_CONFIG,
    )?;

    stmt.execute(params![project_fingerprint, package_ids_bytes])?;
    Ok(())
}

/// Get packages accessed during last build.
pub fn get_access_log(
    &self,
    project_fingerprint: &str,
) -> Result<BTreeSet<PackageId>, Error> {
    let connection = self.connection_pool.get()?;

    let mut stmt = connection.prepare_cached(
        "SELECT package_ids FROM project2package_id_access_log
         WHERE project_fingerprint = ?",
    )?;

    let Some(row) = stmt.query(params![project_fingerprint])?.next()? else {
        return Ok(BTreeSet::new());  // First build
    };

    let package_ids: Vec<&str> =
        bincode::borrow_decode_from_slice(
            row.get_ref_unwrap(0).as_bytes()?,
            BINCODE_CONFIG,
        )?.0;

    Ok(package_ids.into_iter().map(PackageId::new).collect())
}
```

### 5.3 Use Cases

**1. Incremental rustdoc generation:**

```rust
fn compute_rustdoc_needed(
    current_packages: &BTreeSet<PackageId>,
    previous_packages: &BTreeSet<PackageId>,
) -> Vec<PackageId> {
    // Only generate rustdoc for new or changed packages
    current_packages
        .difference(previous_packages)
        .copied()
        .collect()
}
```

**2. Cache cleanup:**

```rust
fn cleanup_unused_cache_entries(
    all_projects: &[ProjectFingerprint],
) -> Result<(), Error> {
    // Find all packages still in use
    let mut in_use = BTreeSet::new();
    for project in all_projects {
        let accessed = get_access_log(project)?;
        in_use.extend(accessed);
    }

    // Remove cache entries not in any access log
    remove_unreferenced_cache_entries(&in_use)?;
    Ok(())
}
```

---

## 6. Parallel Processing

### 6.1 rayon for Parallel Deserialization

```rust
// libs/pavexc/src/rustdoc/compute/mod.rs
use rayon::prelude::*;

// After generating rustdoc JSON, deserialize in parallel
let results: HashMap<PackageId, Crate> = chunk
    .into_par_iter()  // Parallel iterator
    .map(|(package_id, spec)| {
        let krate = load_json_docs(target_dir, &spec);
        (package_id, krate)
    })
    .collect();
```

### 6.2 Connection Pooling

```rust
// libs/pavexc/src/rustdoc/compute/cache.rs
use r2d2::Pool;
use r2d2_sqlite::SqliteConnectionManager;

fn setup_database() -> Result<Pool<SqliteConnectionManager>, Error> {
    let manager = SqliteConnectionManager::file(&cache_path);

    // One connection per CPU core
    let pool = Pool::builder()
        .max_size(num_cpus::get() as u32)
        .build(manager)?;

    Ok(pool)
}

// Usage: connections are borrowed from pool
fn get_cached(&self, key: &CacheKey) -> Result<Option<Crate>, Error> {
    let conn = self.connection_pool.get()?;  // Borrow from pool
    // ... query database
    // conn returned to pool when dropped
}
```

---

## 7. Per-Version Databases

### 7.1 Design Decision

Pavex uses a **different SQLite database per version**:

```rust
fn setup_database() -> Result<Pool<SqliteConnectionManager>, Error> {
    // Include Pavex version in database filename
    let pavex_fingerprint = concat!(
        env!("CARGO_PKG_VERSION"),  // e.g., "0.1.80"
        '-',
        env!("VERGEN_GIT_DESCRIBE"), // e.g., "0.1.80-123-gabc123"
    );

    let cache_dir = xdg_home::home_dir()
        .unwrap()
        .join(".pavex/rustdoc/cache");

    // Database per version: 0.1.80-0.1.80-123-gabc123.db
    let cache_path = cache_dir.join(format!("{}.db", pavex_fingerprint));

    // ...
}
```

### 7.2 Trade-offs

| Approach | Pros | Cons |
|----------|------|------|
| **Per-version DB** | No schema migrations, clean separation | Duplicate data across versions |
| Single DB with migrations | Shared data, less disk space | Complex migration logic |
| Single DB with version column | Shared data, no migrations | More complex queries |

**Why Pavex chose per-version:**
- Simpler implementation (no migration code)
- Clean rollback (just use old DB)
- Disk space is cheap, developer time is expensive

---

## 8. Lazy Deserialization

### 8.1 The Problem

rustdoc JSON can be **large** (10MB+ for big crates):

```
serde:     ~5MB JSON
tokio:     ~3MB JSON
syn:       ~8MB JSON
```

Deserializing all of it upfront is wasteful if you only need a few items.

### 8.2 The Solution: Lazy Index

```rust
// libs/pavexc/src/rustdoc/queries.rs
pub enum CrateItemIndex {
    /// All items deserialized
    Eager(EagerCrateItemIndex),
    /// Items stored as raw JSON, deserialized on demand
    Lazy(LazyCrateItemIndex),
}

pub struct LazyCrateItemIndex {
    /// Raw JSON bytes (all items concatenated)
    pub items: Vec<u8>,
    /// (start, end) byte offsets for each item
    pub item_id2delimiters: HashMap<u32, (usize, usize)>,
}

impl LazyCrateItemIndex {
    /// Get a specific item by ID
    pub fn get(&self, id: u32) -> Option<Item> {
        let (start, end) = self.item_id2delimiters.get(&id)?;
        let item_json = &self.items[*start..*end];
        serde_json::from_slice(item_json).ok()
    }

    /// Iterate all items (deserializes each on demand)
    pub fn iter(&self) -> impl Iterator<Item = Item> + '_ {
        self.item_id2delimiters.keys()
            .sorted()
            .filter_map(|id| self.get(*id))
    }
}
```

### 8.3 Hydration from Cache

```rust
// libs/pavexc/src/rustdoc/compute/cache.rs
impl<'a> CachedData<'a> {
    pub fn hydrate(self, package_id: PackageId) -> Result<Crate, Error> {
        // Deserialize delimiters first
        let item_id2delimiters =
            bincode::decode_from_slice(&self.item_id2delimiters, BINCODE_CONFIG)?.0;

        // Create lazy index (items stay as raw JSON)
        let crate_data = CrateData {
            root_item_id: rustdoc_types::Id(self.root_item_id),
            external_crates: bincode::decode_from_slice(&self.external_crates, BINCODE_CONFIG)?.0,
            paths: bincode::decode_from_slice(&self.paths, BINCODE_CONFIG)?.0,
            format_version: self.format_version.try_into()?,
            index: CrateItemIndex::Lazy(LazyCrateItemIndex {
                items: self.items.into_owned(),  // Keep as bytes
                item_id2delimiters,
            }),
        };

        // Deserialize other fields
        let import_path2id = bincode::decode_from_slice(&self.import_path2id, BINCODE_CONFIG)?.0;
        let re_exports = bincode::decode_from_slice(&self.re_exports, BINCODE_CONFIG)?.0;
        let import_index = bincode::decode_from_slice(&self.import_index, BINCODE_CONFIG)?.0;

        Ok(Crate {
            core: CrateCore { package_id, krate: crate_data },
            import_path2id,
            re_exports,
            import_index,
        })
    }
}
```

---

## 9. Cache Performance

### 9.1 Typical Cache Hit Rates

| Scenario | Hit Rate | Notes |
|----------|----------|-------|
| First build | 0% | Everything is a miss |
| Second build (no changes) | 100% | Full cache hit |
| Source change | 80-95% | Only changed crate misses |
| Feature change | 50-80% | Affected crates miss |
| Toolchain update | 0% | Everything misses |

### 9.2 Measuring Cache Performance

```rust
// libs/pavexc/src/rustdoc/compute/cache.rs
#[instrument(
    name = "Retrieve cached docs",
    skip_all,
    fields(
        crate.id = %package_metadata.id(),
        cache_key = tracing::field::Empty,
        hit = tracing::field::Empty  // Tracing field for metrics
    )
)]
fn get(
    &self,
    package_metadata: &PackageMetadata,
    // ...
) -> Result<Option<Crate>, Error> {
    let outcome = _get_impl(...);

    // Record hit/miss for tracing
    match &outcome {
        Ok(Some(_)) => {
            tracing::Span::current().record("hit", true);
        }
        Ok(None) => {
            tracing::Span::current().record("hit", false);
        }
        _ => {}
    }

    outcome
}
```

---

## 10. Disk Space Management

### 10.1 Cache Size Over Time

```
~/.pavex/rustdoc/cache/
├── 0.1.78-abc.db    (50MB)  # Old version
├── 0.1.79-def.db    (55MB)  # Old version
├── 0.1.80-ghi.db    (60MB)  # Current version
└── ...

Total: ~500MB after several months
```

### 10.2 Cleanup Strategy

```rust
fn cleanup_old_databases(
    keep_versions: usize = 3,
) -> Result<(), Error> {
    let cache_dir = xdg_home::home_dir()
        .unwrap()
        .join(".pavex/rustdoc/cache");

    // List all .db files, sorted by modification time
    let mut dbs: Vec<_> = fs_err::read_dir(&cache_dir)?
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension() == Some("db".as_ref()))
        .collect();

    dbs.sort_by_key(|e| e.metadata().and_then(|m| m.modified()).unwrap());

    // Remove all but the N most recent
    if dbs.len() > keep_versions {
        for db in &dbs[..dbs.len() - keep_versions] {
            fs_err::remove_file(db.path())?;
        }
    }

    Ok(())
}
```

---

## Key Takeaways

1. **Multi-level caching** - Memory, per-project, and global caches
2. **BLAKE3 for fingerprinting** - Fast, secure hashing for crate contents
3. **SQLite for structured caching** - Efficient partial reads, concurrent access
4. **Granular invalidation** - Per-crate, not per-project
5. **Access log tracking** - Enables incremental rustdoc and cleanup
6. **Lazy deserialization** - Don't deserialize what you don't need
7. **Per-version databases** - Avoid schema migration complexity
8. **Connection pooling** - Parallel cache access

---

## Related Files

- **Cache implementation**: `/home/darkvoid/Boxxed/@formulas/src.rust/src.BuildTooling/pavex/libs/pavexc/src/rustdoc/compute/cache.rs`
- **Checksum computation**: `/home/darkvoid/Boxxed/@formulas/src.rust/src.BuildTooling/pavex/libs/pavexc/src/rustdoc/compute/checksum.rs`
- **rustdoc computation**: `/home/darkvoid/Boxxed/@formulas/src.rust/src.BuildTooling/pavex/libs/pavexc/src/rustdoc/compute/mod.rs`

---

*Next: [04-framework-integration-deep-dive.md](04-framework-integration-deep-dive.md)*
