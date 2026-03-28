---
title: "Dependency Resolution Deep Dive: How Pavex Resolves and Caches Dependencies"
subtitle: "Understanding package graphs, feature resolution, and rustdoc JSON caching"
based_on: "pavexc crate - libs/pavexc/src/compiler/ and libs/pavexc/src/rustdoc/"
level: "Intermediate - Requires Rust and cargo fundamentals"
---

# Dependency Resolution Deep Dive

## Overview

This deep dive explores how Pavex resolves dependencies, builds package graphs, and caches rustdoc JSON. We'll examine the `guppy` crate integration, feature flag resolution, and the SQLite-based caching system that makes incremental builds fast.

---

## 1. Dependency Resolution Fundamentals

### 1.1 The Dependency Graph Problem

When you run `cargo build`, cargo must:

```
┌─────────────────────────────────────────────────────────┐
│              Cargo Dependency Resolution                 │
│                                                          │
│  1. Parse Cargo.toml files                              │
│     └── What dependencies exist?                        │
│                                                          │
│  2. Resolve versions                                     │
│     └── Which version of each crate?                    │
│                                                          │
│  3. Unify features                                       │
│     └── Which features are enabled?                     │
│                                                          │
│  4. Topological sort                                     │
│     └── What order to compile?                          │
│                                                          │
│  5. Build                                                │
│     └── Compile in order                                 │
└─────────────────────────────────────────────────────────┘
```

### 1.2 cargo metadata

Cargo exposes dependency info via `cargo metadata`:

```bash
cargo metadata --format-version 1
```

**Output structure (simplified):**

```json
{
  "packages": [
    {
      "name": "my_crate",
      "version": "0.1.0",
      "dependencies": [...],
      "targets": [...],
      "features": {...}
    }
  ],
  "workspace_members": ["my_crate 0.1.0 (path+file://...)"],
  "resolve": {
    "nodes": [
      {
        "id": "my_crate 0.1.0 ...",
        "dependencies": ["serde 1.0.193 ..."],
        "features": ["derive"]
      }
    ]
  },
  "target_directory": "/path/to/target",
  "workspace_root": "/path/to/workspace"
}
```

### 1.3 Why Pavex Needs Dependency Info

Pavex needs to understand dependencies for:

1. **rustdoc JSON generation** - Know which crates to document
2. **Type resolution** - Understand what `serde::Serialize` means
3. **Feature unification** - Ensure consistent feature flags
4. **Caching** - Cache rustdoc JSON per crate version

---

## 2. guppy Integration

### 2.1 What Is guppy?

[guppy](https://docs.rs/guppy) is a library for parsing and analyzing Cargo metadata.

**Why guppy over direct JSON parsing?**

| Approach | Pros | Cons |
|----------|------|------|
| Parse JSON directly | Full control | Manual graph construction, error-prone |
| guppy | Type-safe, graph APIs | Another dependency |

### 2.2 guppy Basic Usage

```rust
use guppy::graph::PackageGraph;

// Parse cargo metadata
let metadata = cargo_metadata::MetadataCommand::new()
    .exec()
    .unwrap();

// Build package graph
let graph = PackageGraph::from_metadata(&metadata).unwrap();

// Query the graph
for package in graph.workspace().iter() {
    println!("Workspace member: {} {}", package.name(), package.version());
}
```

### 2.3 Pavex's Package Graph Usage

```rust
// libs/pavexc/src/compiler/mod.rs (simplified)
use guppy::graph::PackageGraph;

fn analyze_workspace(
    package_graph: &PackageGraph,
) -> Result<Analysis, Error> {
    // Get workspace members
    let workspace = package_graph.workspace();

    // For each workspace member, find handlers and constructors
    for package_id in workspace.iter() {
        let metadata = package_graph.metadata(&package_id).unwrap();

        // Extract targets (lib, bin, etc.)
        for target in metadata.targets() {
            if target.is_lib() {
                // This is a library - extract rustdoc info
                analyze_library(target, metadata)?;
            }
        }
    }

    Ok(analysis)
}
```

### 2.4 Feature Resolution

Pavex must understand which features are enabled:

```rust
use guppy::graph::feature::FeatureSet;

// Get the default feature set
let feature_set = package_metadata
    .to_feature_set(guppy::graph::feature::StandardFeatures::Default);

// Get enabled features for a package
let features = feature_set
    .features_for(&package_id)
    .unwrap();

// Check if a feature is enabled
let has_derive = features.has_feature("derive");

// Get all enabled named features
let named_features: Vec<&str> = features.named_features().collect();
```

**Why features matter for caching:**

```
serde v1.0.193 + features:[derive]  --> different rustdoc JSON -->
serde v1.0.193 + features:[alloc]   --> different rustdoc JSON -->

Cache key must include features!
```

---

## 3. rustdoc JSON Computation

### 3.1 The rustdoc Command

Pavex generates rustdoc JSON via:

```rust
// libs/pavexc/src/rustdoc/compute/mod.rs
fn _compute_single_crate_docs(
    toolchain_name: &str,           // e.g., "nightly-2025-03-26"
    package_id_spec: &PackageIdSpecification,
    current_dir: &Path,
) -> Result<(), anyhow::Error> {
    let mut cmd = std::process::Command::new("rustup");
    cmd.arg("run")
        .current_dir(current_dir)
        .arg(toolchain_name)
        .arg("cargo")
        .arg("rustdoc")
        .arg("-q")
        .arg("--lib")
        .arg("-p")
        .arg(package_id_spec.to_string())
        .arg("-Zunstable-options")
        .arg("--output-format")
        .arg("json")
        .arg("--")
        .arg("--document-private-items")
        .arg("--document-hidden-items");

    let status = cmd.status()?;
    if !status.success() {
        anyhow::bail!("cargo rustdoc failed");
    }
    Ok(())
}
```

### 3.2 rustdoc Options

The flags Pavex uses affect the cache key:

```rust
pub fn rustdoc_options() -> [&'static str; 4] {
    [
        "--document-private-items",   // Include private types
        "-Zunstable-options",         // Enable JSON output
        "-wjson",                     // Write JSON format
        "--document-hidden-items",    // Include #[doc(hidden)] items
    ]
}
```

**Why these options matter:**

- `--document-private-items`: Pavex needs to see private constructors
- `--document-hidden-items`: Some frameworks use `#[doc(hidden)]` internally
- Changing options = different output = cache miss

### 3.3 Chunking Strategy

Cargo can only generate docs for one crate name at a time:

```rust
// libs/pavexc/src/rustdoc/compute/mod.rs
fn compute_crate_docs<I>(
    package_graph: &PackageGraph,
    package_ids: I,
) -> Result<HashMap<PackageId, Crate>, Error>
where
    I: Iterator<Item = PackageId>,
{
    // Group crates by name (avoid duplicates in same batch)
    let chunks = chunk_by_crate_name(package_ids, package_graph);

    for (i, chunk) in chunks.into_iter().enumerate() {
        if i > 0 {
            // Clean target directory for crates with same name
            for (_, spec) in &chunk {
                let _ = fs_err::remove_file(json_doc_location(spec, target_dir));
            }
        }

        // Generate rustdoc JSON for this chunk
        _compute_crate_docs(toolchain_name, chunk.iter().map(|(_, s)| s), current_dir)?;

        // Parse JSON files in parallel
        for (package_id, krate) in chunk
            .into_par_iter()  // rayon parallel iterator
            .map(|(package_id, spec)| {
                let krate = load_json_docs(target_dir, &spec);
                (package_id, krate)
            })
            .collect::<Vec<_>>()
        {
            results.insert(package_id, krate?);
        }
    }

    Ok(results)
}
```

### 3.4 Toolchain Crates

Some crates come from the Rust toolchain:

```rust
// Standard library crates that can't be documented locally
pub const TOOLCHAIN_CRATES: &[&str] = &[
    "std", "core", "alloc", "proc_macro",
    "test", "rustc_std_workspace_core",
    // ...
];

fn compute_crate_docs(
    package_id: &PackageId,
) -> Result<Crate, Error> {
    if TOOLCHAIN_CRATES.contains(&package_id.repr()) {
        // Fetch from pre-computed toolchain docs
        return get_toolchain_crate_docs(package_id.repr(), toolchain_name);
    }

    // Generate rustdoc JSON normally
    _compute_crate_docs(...)
}
```

---

## 4. SQLite Caching System

### 4.1 Cache Database Schema

```sql
-- Table for third-party crate docs
CREATE TABLE rustdoc_3d_party_crates_cache (
    crate_name TEXT NOT NULL,
    crate_source TEXT NOT NULL,          -- "crates.io" or path
    crate_version TEXT NOT NULL,
    crate_hash TEXT NOT NULL,            -- BLAKE3 hash (path deps only)
    cargo_fingerprint TEXT NOT NULL,     -- Toolchain version
    rustdoc_options TEXT NOT NULL,       -- Flags used
    default_feature_is_enabled INTEGER NOT NULL,
    active_named_features TEXT NOT NULL, -- Comma-separated list
    root_item_id INTEGER NOT NULL,
    external_crates BLOB NOT NULL,       -- Serialized external crate info
    paths BLOB NOT NULL,                 -- Serialized path mappings
    format_version INTEGER NOT NULL,
    items BLOB NOT NULL,                 -- Serialized item index
    item_id2delimiters BLOB NOT NULL,    -- For lazy deserialization
    import_index BLOB NOT NULL,
    import_path2id BLOB NOT NULL,
    re_exports BLOB NOT NULL,
    PRIMARY KEY (
        crate_name, crate_source, crate_version, crate_hash,
        cargo_fingerprint, rustdoc_options, default_feature_is_enabled,
        active_named_features
    )
);

-- Table for toolchain crate docs
CREATE TABLE rustdoc_toolchain_crates_cache (
    name TEXT NOT NULL,
    cargo_fingerprint TEXT NOT NULL,
    root_item_id INTEGER NOT NULL,
    external_crates BLOB NOT NULL,
    paths BLOB NOT NULL,
    format_version INTEGER NOT NULL,
    items BLOB NOT NULL,
    item_id2delimiters BLOB NOT NULL,
    import_index BLOB NOT NULL,
    import_path2id BLOB NOT NULL,
    re_exports BLOB NOT NULL,
    PRIMARY KEY (name, cargo_fingerprint)
);

-- Project access log (for cleanup)
CREATE TABLE project2package_id_access_log (
    project_fingerprint TEXT PRIMARY KEY,
    package_ids BLOB NOT NULL  -- Serialized Vec<PackageId>
);
```

### 4.2 Cache Key Construction

```rust
// libs/pavexc/src/rustdoc/compute/cache.rs
pub struct ThirdPartyCrateCacheKey<'a> {
    pub crate_name: &'a str,
    pub crate_source: Cow<'a, str>,    -- "crates.io" or path
    pub crate_version: String,
    pub crate_hash: Option<String>,    -- BLAKE3 for path deps
    pub cargo_fingerprint: &'a str,    -- Toolchain version
    pub rustdoc_options: String,       -- Serialized flags
    pub default_feature_is_enabled: bool,
    pub active_named_features: String, -- Sorted, space-separated
}

impl<'a> ThirdPartyCrateCacheKey<'a> {
    pub fn build(
        package_graph: &PackageGraph,
        package_metadata: &PackageMetadata,
        cargo_fingerprint: &str,
        cache_workspace_packages: bool,
    ) -> Option<Self> {
        // Determine source
        let source = match package_metadata.source() {
            PackageSource::Workspace(p) => {
                if !cache_workspace_packages {
                    return None;  // Don't cache workspace packages
                }
                // ...
            }
            PackageSource::Path(p) => {
                // Compute BLAKE3 hash for path dependencies
                let hash = checksum_crate(&package_path)?;
                Some(hash.to_string())
            }
            PackageSource::External(e) => {
                None  // No hash needed for external crates
            }
        };

        // Get feature info
        let features = package_metadata
            .to_feature_set(StandardFeatures::Default)
            .features_for(package_metadata.id())
            .unwrap();

        let (default_enabled, mut named_features) = match features {
            Some(f) => (f.has_base(), f.named_features().collect()),
            None => (false, vec![]),
        };
        named_features.sort();  // Ensure consistent ordering

        Some(Self {
            crate_name: package_metadata.name(),
            crate_source: source.into(),
            crate_version: package_metadata.version().to_string(),
            crate_hash: source.and_then(|s| s.path_hash),
            cargo_fingerprint,
            rustdoc_options: rustdoc_options().join(" "),
            default_feature_is_enabled: default_enabled,
            active_named_features: named_features.join(" "),
        })
    }
}
```

### 4.3 BLAKE3 Checksumming

```rust
// libs/pavexc/src/rustdoc/compute/checksum.rs
use blake3::Hasher;

pub fn checksum_crate(package_path: &Path) -> Result<String, Error> {
    let mut hasher = Hasher::new();

    // Hash all .rs files in the crate
    for entry in globwalk::glob_walk(package_path, "*.rs")? {
        let content = fs_err::read(entry.path())?;
        hasher.update(&content);
    }

    // Hash Cargo.toml
    let cargo_toml = package_path.join("Cargo.toml");
    if cargo_toml.exists() {
        let content = fs_err::read(&cargo_toml)?;
        hasher.update(&content);
    }

    Ok(hasher.finalize().to_hex().to_string())
}
```

### 4.4 Lazy Deserialization

Storing and loading large JSON efficiently:

```rust
// Serialized format
pub struct CachedData<'a> {
    root_item_id: u32,
    external_crates: Cow<'a, [u8]>,   // bincode serialized
    paths: Cow<'a, [u8]>,             // bincode serialized
    format_version: i64,
    items: Cow<'a, [u8]>,             // JSON as bytes
    item_id2delimiters: HashMap<u32, (usize, usize)>,
    import_index: Cow<'a, [u8]>,
    import_path2id: Cow<'a, [u8]>,
    re_exports: Cow<'a, [u8]>,
}

impl<'a> CachedData<'a> {
    pub fn new(krate: &Crate) -> Result<Self, Error> {
        // Serialize items individually with delimiters
        let mut items = Vec::new();
        let mut item_id2delimiters = HashMap::new();

        for (item_id, item) in &index.index {
            let start = items.len();
            serde_json::to_writer(&mut items, item)?;
            let end = items.len();
            item_id2delimiters.insert(item_id.0, (start, end));
        }

        // Serialize other fields with bincode (faster)
        let external_crates = bincode::encode_to_vec(&crate_data.external_crates, BINCODE_CONFIG)?;
        let paths = bincode::encode_to_vec(&crate_data.paths, BINCODE_CONFIG)?;

        Ok(Self {
            root_item_id: crate_data.root_item_id.0,
            external_crates: Cow::Owned(external_crates),
            paths: Cow::Owned(paths),
            format_version: crate_data.format_version as i64,
            items: Cow::Owned(items),
            item_id2delimiters: Cow::Owned(bincode::encode_to_vec(&item_id2delimiters, BINCODE_CONFIG)?),
            // ...
        })
    }

    pub fn hydrate(self, package_id: PackageId) -> Result<Crate, Error> {
        // Deserialize delimiters
        let item_id2delimiters = bincode::decode_from_slice(&self.item_id2delimiters, BINCODE_CONFIG)?.0;

        // Create lazy index (don't deserialize items yet)
        let crate_data = CrateData {
            root_item_id: rustdoc_types::Id(self.root_item_id),
            external_crates: bincode::decode_from_slice(&self.external_crates, BINCODE_CONFIG)?.0,
            paths: bincode::decode_from_slice(&self.paths, BINCODE_CONFIG)?.0,
            format_version: self.format_version.try_into()?,
            index: CrateItemIndex::Lazy(LazyCrateItemIndex {
                items: self.items.into_owned(),
                item_id2delimiters,
            }),
        };

        // ... continue hydration
        Ok(Crate { core: CrateCore { package_id, krate: crate_data }, ... })
    }
}
```

### 4.5 Cache Hit/Miss Logic

```rust
// libs/pavexc/src/rustdoc/compute/cache.rs
pub fn get(
    &self,
    cache_key: &RustdocCacheKey,
    package_graph: &PackageGraph,
) -> Result<Option<Crate>, Error> {
    let connection = self.connection_pool.get()?;

    match cache_key {
        RustdocCacheKey::ThirdPartyCrate(metadata) => {
            // Query with full cache key
            let mut stmt = connection.prepare_cached(
                "SELECT root_item_id, external_crates, paths, format_version, items, ...
                 FROM rustdoc_3d_party_crates_cache
                 WHERE crate_name = ? AND crate_source = ? AND crate_version = ?
                   AND crate_hash = ? AND cargo_fingerprint = ? AND rustdoc_options = ?
                   AND default_feature_is_enabled = ? AND active_named_features = ?",
            )?;

            let mut rows = stmt.query(params![
                cache_key.crate_name,
                cache_key.crate_source,
                cache_key.crate_version,
                cache_key.crate_hash.unwrap_or_default(),
                cache_key.cargo_fingerprint,
                cache_key.rustdoc_options,
                cache_key.default_feature_is_enabled,
                cache_key.active_named_features,
            ])?;

            let Some(row) = rows.next()? else {
                return Ok(None);  // Cache miss
            };

            // Deserialize and hydrate
            let krate = CachedData { /* extract from row */ }
                .hydrate(metadata.id().to_owned())?;

            Ok(Some(krate))  // Cache hit!
        }
        RustdocCacheKey::ToolchainCrate(name) => {
            // Similar logic for toolchain crates
        }
    }
}
```

---

## 5. Cargo Fingerprint

### 5.1 What Is the Cargo Fingerprint?

The cargo fingerprint identifies the toolchain version:

```rust
// libs/pavexc/src/rustdoc/compute/cache.rs
pub fn cargo_fingerprint(toolchain_name: &str) -> Result<String, Error> {
    let mut cmd = std::process::Command::new("rustup");
    cmd.arg("run")
        .arg(toolchain_name)
        .arg("cargo")
        .arg("--verbose")
        .arg("--version");

    let output = cmd.output()
        .context(format!("Failed to run cargo --version for {toolchain_name}"))?;

    if !output.status.success() {
        anyhow::bail!("cargo --version failed");
    }

    let output = String::from_utf8(output.stdout)
        .context("cargo --version returned non-UTF8")?;

    Ok(output)  // e.g., "cargo 1.78.0-nightly (...) ..."
}
```

### 5.2 Why It Matters

```
nightly-2025-03-26 + cargo 1.78.0  --> cache entry A
nightly-2025-04-01 + cargo 1.79.0  --> cache entry B

Different toolchains may produce different rustdoc JSON!
```

---

## 6. Project Access Log

### 6.1 Tracking Package Usage

```rust
// libs/pavexc/src/rustdoc/compute/cache.rs
pub fn persist_access_log(
    &self,
    package_ids: &BTreeSet<PackageId>,
    project_fingerprint: &str,
) -> Result<(), Error> {
    let connection = self.connection_pool.get()?;

    let mut stmt = connection.prepare_cached(
        "INSERT INTO project2package_id_access_log
         (project_fingerprint, package_ids)
         VALUES (?, ?)
         ON CONFLICT(project_fingerprint) DO UPDATE SET package_ids=excluded.package_ids",
    )?;

    stmt.execute(params![
        project_fingerprint,
        bincode::encode_to_vec(
            package_ids.iter().map(|s| s.repr()).collect_vec(),
            BINCODE_CONFIG
        )?,
    ])?;

    Ok(())
}

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
        return Ok(BTreeSet::new());
    };

    let package_ids: Vec<&str> =
        bincode::borrow_decode_from_slice(row.get_ref_unwrap(0).as_bytes()?, BINCODE_CONFIG)?.0;

    Ok(package_ids.into_iter().map(PackageId::new).collect())
}
```

### 6.2 Why Track Access?

1. **Cache cleanup** - Remove entries no longer used
2. **Incremental rebuilds** - Know which docs to re-fetch
3. **Debugging** - Understand dependency usage

---

## 7. Database Setup

### 7.1 Database Location

```rust
fn setup_database() -> Result<r2d2::Pool<SqliteConnectionManager>, Error> {
    let pavex_fingerprint = concat!(
        env!("CARGO_PKG_VERSION"), '-', env!("VERGEN_GIT_DESCRIBE")
    );

    let cache_dir = xdg_home::home_dir()
        .ok_or_else(|| anyhow::anyhow!("Failed to get home directory"))?
        .join(".pavex/rustdoc/cache");

    fs_err::create_dir_all(&cache_dir)?;

    // Different DB per Pavex version (avoids schema migrations)
    let cache_path = cache_dir.join(format!("{}.db", pavex_fingerprint));

    let manager = SqliteConnectionManager::file(&cache_path);
    let pool = r2d2::Pool::builder()
        .max_size(num_cpus::get() as u32)  // Parallel connections
        .build(manager)?;

    // Create tables
    let connection = pool.get()?;
    connection.execute(
        "CREATE TABLE IF NOT EXISTS rustdoc_3d_party_crates_cache (...)",
        [],
    )?;
    connection.execute(
        "CREATE TABLE IF NOT EXISTS rustdoc_toolchain_crates_cache (...)",
        [],
    )?;
    connection.execute(
        "CREATE TABLE IF NOT EXISTS project2package_id_access_log (...)",
        [],
    )?;

    Ok(pool)
}
```

### 7.2 Connection Pooling

```rust
use r2d2::Pool;
use r2d2_sqlite::SqliteConnectionManager;

// Create pool with multiple connections
let pool = Pool::builder()
    .max_size(num_cpus::get() as u32)  // One connection per CPU
    .build(manager)?;

// Get connection from pool
let conn = pool.get()?;

// Connections are returned to pool when dropped
```

---

## Key Takeaways

1. **guppy provides type-safe cargo metadata** - Use it instead of parsing JSON directly
2. **Feature flags affect rustdoc output** - Cache key must include enabled features
3. **BLAKE3 for path dependency hashing** - Detect local crate changes
4. **SQLite for global caching** - Share cache across all projects
5. **Lazy deserialization** - Store JSON as bytes, deserialize on demand
6. **Per-version databases** - Avoid schema migrations
7. **Connection pooling** - Parallel rustdoc generation

---

## Related Files

- **rustdoc computation**: `/home/darkvoid/Boxxed/@formulas/src.rust/src.BuildTooling/pavex/libs/pavexc/src/rustdoc/compute/mod.rs`
- **Cache implementation**: `/home/darkvoid/Boxxed/@formulas/src.rust/src.BuildTooling/pavex/libs/pavexc/src/rustdoc/compute/cache.rs`
- **Checksum computation**: `/home/darkvoid/Boxxed/@formulas/src.rust/src.BuildTooling/pavex/libs/pavexc/src/rustdoc/compute/checksum.rs`

---

*Next: [03-incremental-builds-deep-dive.md](03-incremental-builds-deep-dive.md)*
