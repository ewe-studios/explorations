---
source: /home/darkvoid/Boxxed/@dev/repo-expolorations/jj-vcs/
repository: https://github.com/jj-vcs/jj
revised_at: 2026-03-23
---

# Rust Revision: Jujutsu (jj) VCS

## Overview

This document provides production-level guidance for understanding and reproducing the core VCS functionality of Jujutsu (jj) in Rust. jj is an experimental version control system that rethinks how version control works while using Git as its default storage backend. The key innovations are: a stable ChangeId that survives commit rewrites, an operation log that records every mutation as an immutable DAG enabling full undo, and first-class conflict tracking using algebraic merge types.

This revision focuses specifically on how jj stores file changes, its compression and deduplication strategy, its diff and merge algorithms, and the complete data flow from working copy to persistent storage.

## Package Structure

```
jj/
├── Cargo.toml                    # Workspace root (resolver = "3")
├── lib/                          # jj-lib: core VCS library
│   ├── Cargo.toml
│   ├── src/
│   │   ├── lib.rs                # Re-exports all modules
│   │   ├── backend.rs            # Backend trait + all core types
│   │   ├── store.rs              # Caching wrapper (LRU) around Backend
│   │   ├── git_backend.rs        # Git ODB storage implementation
│   │   ├── simple_backend.rs     # Native protobuf storage (testing)
│   │   ├── secret_backend.rs     # Access-restricted wrapper
│   │   ├── repo.rs               # ReadonlyRepo, MutableRepo, RepoLoader
│   │   ├── transaction.rs        # Atomic mutation via Transaction
│   │   ├── operation.rs          # Operation wrapper
│   │   ├── op_store.rs           # OpStore trait + View/Operation types
│   │   ├── simple_op_store.rs    # Protobuf-based OpStore
│   │   ├── op_heads_store.rs     # Current operation head tracking
│   │   ├── view.rs               # Snapshot of all refs at a point
│   │   ├── workspace.rs          # Ties repo + working copy
│   │   ├── working_copy.rs       # WorkingCopy trait
│   │   ├── local_working_copy.rs # Filesystem working copy
│   │   ├── merge.rs              # Merge<T> algebraic conflict type
│   │   ├── merged_tree.rs        # Lazily merged tree sets
│   │   ├── diff.rs               # Histogram-like diff algorithm
│   │   ├── conflicts.rs          # Conflict materialization/parsing
│   │   ├── files.rs              # File-level merge operations
│   │   ├── rewrite.rs            # Commit rewriting (rebase, squash)
│   │   ├── revset.rs             # Commit selection DSL
│   │   ├── content_hash.rs       # BLAKE2b-512 hashing trait
│   │   ├── stacked_table.rs      # Persistent append-only key-value store
│   │   ├── default_index/        # On-disk commit graph index
│   │   │   ├── composite.rs      # Stacked readonly segments
│   │   │   ├── entry.rs          # IndexPosition, IndexEntry types
│   │   │   ├── mutable.rs        # In-memory index building
│   │   │   ├── readonly.rs       # Memory-mapped on-disk segments
│   │   │   ├── store.rs          # DefaultIndexStore
│   │   │   ├── rev_walk.rs       # DAG walking via index
│   │   │   └── revset_engine.rs  # Revset evaluation
│   │   ├── object_id.rs          # ObjectId trait, id_type! macro
│   │   ├── dag_walk.rs           # Generic DAG algorithms
│   │   └── protos/               # Protobuf definitions
│   │       ├── op_store.proto
│   │       ├── simple_store.proto
│   │       ├── git_store.proto
│   │       └── working_copy.proto
│   ├── proc-macros/              # ContentHash derive macro
│   ├── gen-protos/               # Protobuf code generation
│   └── testutils/                # Test infrastructure
├── cli/                          # jj-cli: command-line interface
│   ├── Cargo.toml
│   ├── src/
│   │   ├── main.rs               # Binary entry point
│   │   ├── cli_util.rs           # Workspace loading, TX management
│   │   ├── commands/             # ~40+ command modules
│   │   ├── template_builder.rs   # Template language compiler
│   │   ├── revset_util.rs        # Revset CLI utilities
│   │   └── diff_util.rs          # Diff display
│   └── tests/                    # Snapshot integration tests
└── docs/                         # MkDocs documentation
```

## Crate Breakdown

### jj-lib (Core Library)

The heart of jj. All VCS logic lives here. External consumers use `jj_lib::*`.

**Key traits:**
- `Backend` -- abstract object storage (files, trees, commits)
- `OpStore` -- operation log storage
- `IndexStore` -- commit graph index
- `WorkingCopy` -- filesystem interaction

**Key types:**
- `CommitId` (20 bytes, SHA-1 for Git backend) -- content-addressed commit hash
- `ChangeId` (16 bytes, random) -- stable identifier surviving rewrites
- `Merge<T>` -- algebraic conflict representation
- `ReadonlyRepo` / `MutableRepo` -- immutable/mutable repository snapshots
- `Transaction` -- atomic mutation wrapper
- `Operation` / `View` -- operation log entries

### jj-cli (Binary)

Thin layer over jj-lib using clap for argument parsing, pest-based template/revset languages for output formatting and commit selection, and sapling-renderdag for graph visualization.

### jj-lib-proc-macros

Provides `#[derive(ContentHash)]` for automatic BLAKE2b-512 hash implementation on structs.

### gen-protos

Build-time protobuf code generation using prost-build.

### testutils

Shared test infrastructure: `TestRepo`, `TestWorkspace`, helpers for creating test repositories.

## How jj Stores File Changes

### The Core Insight: jj Does Not Reinvent Object Storage

jj's default (and production) backend is Git. It stores file contents, trees, and commits as **native Git objects** in a standard Git Object Database (ODB). This means:

1. **File content storage** uses Git's blob format: zlib-compressed content with a `blob <size>\0` header, content-addressed by SHA-1.
2. **Tree storage** uses Git's tree format: sorted entries of `<mode> <name>\0<20-byte-SHA-1>`.
3. **Commit storage** uses Git's commit format with jj-specific custom headers (`change-id`, `jj:trees`).
4. **Compression** is entirely delegated to Git's packfile mechanism (zlib for loose objects, delta compression in packfiles).
5. **Garbage collection** invokes `git gc` as a subprocess.

What jj adds on top of Git's storage is a metadata layer that tracks jj-specific concepts.

### Object Write Path (GitBackend)

When jj writes a file:

```rust
// GitBackend::write_file (simplified from git_backend.rs:976)
async fn write_file(
    &self,
    _path: &RepoPath,
    contents: &mut (dyn Read + Send),
) -> BackendResult<FileId> {
    let mut bytes = Vec::new();
    contents.read_to_end(&mut bytes).unwrap();
    let locked_repo = self.lock_git_repo();
    let oid = locked_repo.write_blob(bytes)?;
    Ok(FileId::new(oid.as_bytes().to_vec()))
}
```

This calls `gix::Repository::write_blob()`, which:
1. Prepends the Git blob header (`blob <size>\0`)
2. Computes SHA-1 of the header + content
3. Compresses the entire blob with zlib (deflate)
4. Writes to `.git/objects/<first-2-hex>/<remaining-38-hex>`
5. Returns the 20-byte SHA-1 as the `FileId`

**There is no delta/patch compression at write time.** Each file version is stored as a complete, independently compressed blob. Delta compression only happens during `git gc` / `git repack`, which builds packfiles using a sliding-window delta algorithm.

### Tree Write Path

Trees map filenames to object IDs. jj's `Tree` type is a `BTreeMap<RepoPathComponentBuf, TreeValue>`:

```rust
// From backend.rs
#[derive(ContentHash, Default, PartialEq, Eq, Debug, Clone)]
pub struct Tree {
    entries: BTreeMap<RepoPathComponentBuf, TreeValue>,
}

#[derive(ContentHash, Debug, PartialEq, Eq, Clone, Hash)]
pub enum TreeValue {
    File { id: FileId, executable: bool },
    Symlink(SymlinkId),
    Tree(TreeId),
    GitSubmodule(CommitId),
    Conflict(ConflictId),
}
```

When written to Git, each `TreeValue` variant maps to a Git tree entry mode:
- `File { executable: false }` -> `100644` (blob)
- `File { executable: true }` -> `100755` (executable blob)
- `Symlink` -> `120000` (symlink)
- `Tree` -> `040000` (subtree)
- `Conflict` -> stored as a blob with `.jjconflict` suffix in legacy format

The tree is serialized to Git's tree format and content-addressed by SHA-1, same as any Git tree.

### Commit Write Path

The commit is the most interesting object because jj extends it with custom headers:

```rust
// From git_backend.rs:1215 (simplified)
async fn write_commit(
    &self,
    mut contents: Commit,
    mut sign_with: Option<&mut SigningFn>,
) -> BackendResult<(CommitId, Commit)> {
    let locked_repo = self.lock_git_repo();

    // Build extra headers for jj metadata
    let mut extra_headers = vec![];
    if let MergedTreeId::Merge(tree_ids) = &contents.root_tree {
        if !tree_ids.is_resolved() {
            // Store conflicted tree IDs in a custom header
            let value = tree_ids.iter().map(|id| id.hex()).join(" ");
            extra_headers.push(("jj:trees", value));
        }
    }
    if self.write_change_id_header {
        extra_headers.push(("change-id", contents.change_id.reverse_hex()));
    }

    // Serialize jj extras (change_id, predecessors, tree conflict format)
    let extras = serialize_extras(&contents);

    // Lock the metadata table to prevent races
    let (table, table_lock) = self.read_extra_metadata_table_locked()?;

    // Write the Git commit, adjusting timestamp if hash collision on metadata
    let id = loop {
        let commit = gix::objs::Commit { /* ... */ };
        let git_id = locked_repo.write_object(&commit)?;
        match table.get_value(git_id.as_bytes()) {
            Some(existing_extras) if existing_extras != extras => {
                // Hash collision on different metadata -- adjust timestamp
                committer.time.seconds -= 1;
            }
            _ => break CommitId::from_bytes(git_id.as_bytes()),
        }
    };

    // Create no-gc ref to prevent Git from collecting this commit
    locked_repo.edit_reference(to_no_gc_ref_update(&id))?;

    // Store jj metadata in the sidecar stacked table
    let mut mut_table = table.start_mutation();
    mut_table.add_entry(id.to_bytes(), extras);
    self.save_extra_metadata_table(mut_table, &table_lock)?;

    Ok((id, contents))
}
```

Key points:
- The Git commit object contains the standard tree, parents, author, committer, message fields
- jj adds `change-id` and `jj:trees` as custom Git commit headers (these survive `git push`/`pull`)
- A protobuf-encoded "extras" blob (change_id, predecessors, tree conflict format flag) is stored in a sidecar stacked table at `.jj/repo/store/extra/`
- A `refs/jj/keep/<commit-hex>` ref is created to prevent Git GC from collecting the commit

### Sidecar Metadata: Stacked Tables

The stacked table (`stacked_table.rs`) is jj's custom persistent key-value store used by `GitBackend` to map Git commit SHA-1s to jj metadata (ChangeId, predecessors, tree conflict info).

**Format (on disk):**
```
[parent_filename_length: u32]
[parent_filename: bytes]           # if parent_filename_length > 0
[num_entries: u32]
[index: sorted entries of (key: [u8; key_size], value_offset: u32)]
[values: concatenated variable-size values]
```

**Design:**
- Fixed-size keys (20 bytes for Git SHA-1), variable-size values
- Keys stored in sorted order for binary search (O(log n) lookup)
- Each table segment can have a parent segment, forming a chain
- New entries are written as a new segment; the chain is compacted periodically
- Content-addressed by BLAKE2b-512 for deduplication
- File-locked during writes for concurrency safety

```rust
// From stacked_table.rs
pub struct ReadonlyTable {
    key_size: usize,
    parent_file: Option<Arc<ReadonlyTable>>,
    name: String,
    num_local_entries: usize,
    index: Vec<u8>,    // sorted keys + offsets
    values: Vec<u8>,   // concatenated values
}

pub struct MutableTable {
    key_size: usize,
    parent_file: Option<Arc<ReadonlyTable>>,
    entries: BTreeMap<Vec<u8>, Vec<u8>>,
}
```

Lookup traverses from child to parent segments:
```rust
fn get_value<'a>(&'a self, key: &[u8]) -> Option<&'a [u8]> {
    self.segment_get_value(key)
        .or_else(|| self.segment_parent_file()?.get_value(key))
}
```

### The SimpleBackend (Native Storage)

The `SimpleBackend` is a non-Git storage implementation used for testing. It shows what a pure-jj storage layer looks like:

- Objects are serialized using protobuf (`simple_store.proto`)
- Content-addressed by BLAKE2b-512 (64-byte hashes)
- Each object type stored in its own directory
- No delta compression -- every version is a complete snapshot
- No packfile equivalent

```rust
// SimpleBackend object IDs
const COMMIT_ID_LENGTH: usize = 64;  // BLAKE2b-512
const CHANGE_ID_LENGTH: usize = 16;  // Random
```

### How Git Compression Works Under jj

Since jj delegates to Git for storage, the compression pipeline is:

1. **Loose objects:** Each blob/tree/commit is individually zlib-compressed. This is purely whole-file compression with no delta encoding. A 1MB file stored as a loose object produces a ~1MB compressed blob (compression ratio depends on content).

2. **Packfiles (during `git gc` / `git repack`):** Git's packfile format uses delta compression:
   - Objects are sorted by type and size
   - A sliding window algorithm finds similar objects
   - Delta chains are created: `base_object + delta_1 + delta_2 + ...`
   - The delta format is a custom binary format with copy-from-base and insert-new-data instructions
   - Maximum delta chain depth is configurable (default 50)
   - The pack index provides O(1) lookup by SHA-1

3. **jj triggers GC** via the `gc()` method on `Backend`:
   ```rust
   fn gc(&self, index: &dyn Index, keep_newer: SystemTime) -> BackendResult<()> {
       recreate_no_gc_refs(git_repo, heads, keep_newer)?;
       run_git_gc(&self.git_executable, git_dir)?;
       Ok(())
   }
   ```
   This first updates the `refs/jj/keep/` namespace to protect reachable objects, then runs `git gc`.

### Change Tracking: ChangeId vs CommitId

The dual-ID system is central to jj's model:

```rust
// From backend.rs
id_type!(pub CommitId { hex() });      // 20 bytes, SHA-1 (Git backend)
id_type!(pub ChangeId { reverse_hex() }); // 16 bytes, random

pub struct Commit {
    pub parents: Vec<CommitId>,
    pub predecessors: Vec<CommitId>,  // Previous versions of this change
    pub root_tree: MergedTreeId,
    pub change_id: ChangeId,          // Stable across rewrites
    pub description: String,
    pub author: Signature,
    pub committer: Signature,
    pub secure_sig: Option<SecureSig>,
}
```

- `CommitId` changes whenever commit content changes (amend, rebase, squash)
- `ChangeId` stays the same across rewrites -- it is a random 16-byte identifier generated once
- `predecessors` tracks the evolution chain: when commit A is amended to produce commit B, B's predecessors list contains A's CommitId
- For Git-imported commits without a `change-id` header, jj derives ChangeId by reversing and bit-flipping bytes from the CommitId:
  ```rust
  ChangeId::new(
      id.as_bytes()[4..HASH_LENGTH]
          .iter()
          .rev()
          .map(|b| b.reverse_bits())
          .collect(),
  )
  ```

### Operation Log

Every mutation to the repository is recorded as an `Operation`:

```rust
// From op_store.rs
pub struct Operation {
    pub view_id: ViewId,
    pub parents: Vec<OperationId>,
    pub metadata: OperationMetadata,
}

pub struct View {
    pub head_ids: HashSet<CommitId>,
    pub local_bookmarks: BTreeMap<RefNameBuf, RefTarget>,
    pub tags: BTreeMap<RefNameBuf, RefTarget>,
    pub remote_views: BTreeMap<RemoteNameBuf, RemoteView>,
    pub git_refs: BTreeMap<GitRefNameBuf, RefTarget>,
    pub git_head: RefTarget,
    pub wc_commit_ids: HashMap<WorkspaceNameBuf, CommitId>,
}
```

Storage path:
- Operations: `.jj/repo/op_store/operations/<hex-id>` (protobuf)
- Views: `.jj/repo/op_store/views/<hex-id>` (protobuf)
- Both content-addressed by BLAKE2b-512

The operation DAG supports concurrent operations (multiple workspaces can create operations in parallel, producing a DAG that is later merged).

## Diff Algorithm

jj uses a histogram-based diff algorithm operating at word and line granularity. The implementation in `diff.rs` is a custom algorithm similar to Git's histogram diff / patience diff family.

### Architecture

```rust
// Core abstraction: CompareBytes trait for configurable comparison
pub trait CompareBytes {
    fn eq(&self, left: &[u8], right: &[u8]) -> bool;
    fn hash<H: Hasher>(&self, text: &[u8], state: &mut H);
}

// Implementations:
// - CompareBytesExactly: literal byte comparison
// - CompareBytesIgnoreAllWhitespace: ignores all whitespace
// - CompareBytesIgnoreWhitespaceAmount: normalizes whitespace runs
```

### Algorithm Steps

1. **Tokenization:** Input is split into ranges (words, lines, or individual non-word characters) using `find_word_ranges()`, `find_line_ranges()`, or `find_nonword_ranges()`.

2. **Hash pre-computation:** Each token range is hashed using the `WordComparator`, producing a `Vec<u64>` for each side. This enables O(1) hash lookups later.

3. **Histogram construction:** A `HashTable<HistogramEntry>` maps hashed words to their positions in the source text, capped at a `max_occurrences` threshold to ignore very common tokens.

4. **Patience-like matching:** Unique or low-frequency tokens are matched first (lowest count entries), establishing anchor points. The algorithm then recursively finds matches in the gaps between anchors.

5. **Result:** A sequence of `DiffHunk` values -- `Matching(ranges)`, `Different(slices)` -- representing the edit script.

```rust
// SmallVec optimization: most words appear at 1-2 positions
type HistogramEntry<'input> = (HashedWord<'input>, SmallVec<[LocalWordPosition; 2]>);
```

### File-Level Merge

File merges use the diff algorithm to produce a 3-way merge:

```rust
// From files.rs (conceptual)
pub enum MergeResult {
    Resolved(BString),
    Conflict(Vec<MergeHunk>),
}

pub enum MergeHunk {
    Resolved(BString),
    Conflict(Merge<BString>),
}
```

The merge works by:
1. Diffing each side against the base
2. Interleaving the diffs
3. Detecting overlapping changes as conflicts
4. Representing conflicts as `Merge<T>` values

## First-Class Conflicts: `Merge<T>`

The `Merge<T>` type is the algebraic foundation of jj's conflict system:

```rust
// From merge.rs
pub struct Merge<T> {
    /// Alternates between positive and negative terms, starting with positive.
    /// A resolved value has length 1 (just one add).
    /// A 3-way conflict has length 3: [add0, remove, add1].
    /// A 5-way conflict has length 5: [add0, remove0, add1, remove1, add2].
    values: SmallVec<[T; 1]>,
}
```

- `Merge::resolved(v)` -> `[v]` (no conflict)
- 3-way conflict: `[side_a, base, side_b]` (add, remove, add)
- N-way merges nest: merging two conflicts produces a wider conflict

The trivial merge resolver:
```rust
pub fn trivial_merge<T>(values: &[T]) -> Option<&T>
where T: Eq + Hash
{
    // Optimized for common cases
    if let [add] = values { return Some(add); }
    if let [add0, remove, add1] = values {
        if add0 == add1 { return Some(add0); }     // Both sides agree
        if add0 == remove { return Some(add1); }    // Only right changed
        if add1 == remove { return Some(add0); }    // Only left changed
        return None;                                 // True conflict
    }
    // General case: count occurrences, cancel matching adds/removes
    // ...
}
```

This permeates the entire codebase: `MergedTreeId`, `RefTarget`, `MergedTree`, `MergedTreeValue` all use `Merge<T>`. Conflicts are not textual markers -- they are structured data that can be rebased, merged, and resolved programmatically.

## Commit Graph Index

The `default_index` module provides an on-disk index for efficient commit graph queries.

### Format

The index uses stacked binary segments, where each segment has:
- A parent segment reference (forming a chain)
- Sorted entries containing: CommitId, ChangeId, generation number, parent positions
- O(1) positional lookup (entries addressed by `IndexPosition(u32)`)

```rust
// From entry.rs
pub struct IndexPosition(pub(super) u32);  // Global position

struct MutableGraphEntry {
    commit_id: CommitId,
    change_id: ChangeId,
    generation_number: u32,
    parent_positions: SmallIndexPositionsVec,  // SmallVec<[IndexPosition; 4]>
}
```

### Operations

- **Commit lookup by ID:** Binary search on sorted CommitId entries -> O(log n)
- **Ancestor queries:** Walk parent_positions using generation numbers as bounds -> efficient pruning
- **Common ancestor:** BFS with generation-based priority queue
- **Heads computation:** Filter set to only commits not reachable from others
- **Revset evaluation:** Compiled revset expressions evaluated directly against the index

### SmallVec Optimization

The index uses `SmallVec<[IndexPosition; 4]>` for parent positions. Since `IndexPosition` is 4 bytes and `SmallVec` inlines up to 16 bytes on 64-bit platforms, this avoids heap allocation for commits with up to 4 parents (covers essentially all real-world cases).

## Content Hashing

jj uses BLAKE2b-512 for all internal content addressing (operations, views, simple backend objects, stacked table names):

```rust
// From content_hash.rs
pub trait ContentHash {
    fn hash(&self, state: &mut impl DigestUpdate);
}

pub fn blake2b_hash(x: &(impl ContentHash + ?Sized)) -> digest::Output<Blake2b512> {
    let mut hasher = Blake2b512::default();
    x.hash(&mut hasher);
    hasher.finalize()
}
```

The `#[derive(ContentHash)]` proc macro generates stable, portable hash implementations:
- Integers use little-endian encoding
- Variable-length sequences hash their length (u64 LE) before elements
- Enums hash their variant ordinal (u32 LE) before fields
- Unordered containers sort by `Ord` before hashing

The Git backend uses SHA-1 for object IDs (Git compatibility) but BLAKE2b-512 for its own metadata (stacked tables, operation store).

## Reproducing jj's VCS Functionality in Rust

### Workspace Layout

```toml
# Cargo.toml (workspace)
[workspace]
resolver = "3"
members = ["lib", "lib/proc-macros", "cli"]

[workspace.package]
version = "0.1.0"
edition = "2021"
rust-version = "1.84"
```

### Dependency Recommendations

```toml
# Core storage
gix = { version = "0.71", default-features = false, features = [
    "attributes", "blob-diff", "index", "max-performance-safe", "zlib-rs"
] }

# Serialization
prost = "0.13"
prost-build = "0.13"  # build dependency

# Hashing
blake2 = "0.10"
digest = "0.10"

# Data structures
smallvec = "1.14"
hashbrown = { version = "0.15", features = ["inline-more"] }
indexmap = { version = "2.9", features = ["serde"] }
clru = "0.6"           # LRU cache

# Async bridge
async-trait = "0.1"
futures = "0.3"
pollster = "0.4"        # block_on for sync wrappers

# Error handling
thiserror = "2.0"

# Utilities
itertools = "0.14"
bstr = "1.11"           # Byte string handling
tempfile = "3.19"       # Atomic file writes
rand = "0.8"            # ChangeId generation

# CLI (if building one)
clap = { version = "4.5", features = ["derive"] }
pest = "2.8"
pest_derive = "2.8"
tracing = "0.1"
```

### Type System Design

#### ID Types

Use a macro to define strongly-typed IDs:

```rust
/// Macro for generating strongly-typed content-addressed identifiers.
macro_rules! id_type {
    (pub $name:ident) => {
        #[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
        pub struct $name(Vec<u8>);

        impl $name {
            pub fn new(bytes: Vec<u8>) -> Self { Self(bytes) }
            pub fn from_bytes(bytes: &[u8]) -> Self { Self(bytes.to_vec()) }
            pub fn as_bytes(&self) -> &[u8] { &self.0 }
            pub fn to_bytes(&self) -> Vec<u8> { self.0.clone() }
            pub fn hex(&self) -> String { hex::encode(&self.0) }
            pub fn from_hex(hex: &str) -> Self {
                Self(hex::decode(hex).expect("valid hex"))
            }
        }

        impl std::fmt::Debug for $name {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                write!(f, "{}({})", stringify!($name), self.hex())
            }
        }
    };
}

id_type!(pub CommitId);
id_type!(pub ChangeId);
id_type!(pub TreeId);
id_type!(pub FileId);
id_type!(pub OperationId);
id_type!(pub ViewId);
```

#### The Backend Trait

```rust
#[async_trait]
pub trait Backend: Send + Sync + Debug {
    fn name(&self) -> &str;
    fn commit_id_length(&self) -> usize;
    fn change_id_length(&self) -> usize;
    fn root_commit_id(&self) -> &CommitId;
    fn root_change_id(&self) -> &ChangeId;
    fn empty_tree_id(&self) -> &TreeId;
    fn concurrency(&self) -> usize;

    async fn read_file(&self, path: &RepoPath, id: &FileId)
        -> BackendResult<Box<dyn Read>>;
    async fn write_file(&self, path: &RepoPath, contents: &mut (dyn Read + Send))
        -> BackendResult<FileId>;
    async fn read_tree(&self, path: &RepoPath, id: &TreeId)
        -> BackendResult<Tree>;
    async fn write_tree(&self, path: &RepoPath, contents: &Tree)
        -> BackendResult<TreeId>;
    async fn read_commit(&self, id: &CommitId)
        -> BackendResult<Commit>;
    async fn write_commit(&self, contents: Commit, sign_with: Option<&mut SigningFn>)
        -> BackendResult<(CommitId, Commit)>;
    fn gc(&self, index: &dyn Index, keep_newer: SystemTime)
        -> BackendResult<()>;
}
```

#### The Store Wrapper

```rust
pub struct Store {
    backend: Box<dyn Backend>,
    commit_cache: Mutex<CLruCache<CommitId, Arc<Commit>>>,  // 100 entries
    tree_cache: Mutex<CLruCache<(RepoPathBuf, TreeId), Arc<Tree>>>,  // 1000 entries
}
```

The cache sizes are deliberately small. jj's access patterns are sequential within a transaction, so a small LRU suffices. The `Mutex` is acceptable because there is typically no contention.

#### Conflict Representation

```rust
pub struct Merge<T> {
    values: SmallVec<[T; 1]>,
}

impl<T> Merge<T> {
    pub fn resolved(value: T) -> Self {
        Merge { values: smallvec![value] }
    }

    pub fn from_vec(values: SmallVec<[T; 1]>) -> Self {
        assert!(values.len() % 2 == 1, "must have odd number of terms");
        Merge { values }
    }

    pub fn is_resolved(&self) -> bool { self.values.len() == 1 }
    pub fn as_resolved(&self) -> Option<&T> {
        if self.values.len() == 1 { Some(&self.values[0]) } else { None }
    }
    pub fn adds(&self) -> impl Iterator<Item = &T> {
        self.values.iter().step_by(2)
    }
    pub fn removes(&self) -> impl Iterator<Item = &T> {
        self.values.iter().skip(1).step_by(2)
    }
}
```

### Error Handling

jj uses `thiserror` throughout with structured error enums:

```rust
#[derive(Debug, Error)]
pub enum BackendError {
    #[error("Object {hash} of type {object_type} not found")]
    ObjectNotFound {
        object_type: String,
        hash: String,
        source: Box<dyn std::error::Error + Send + Sync>,
    },
    #[error("Error when reading object {hash} of type {object_type}")]
    ReadObject {
        object_type: String,
        hash: String,
        source: Box<dyn std::error::Error + Send + Sync>,
    },
    #[error("Could not write object of type {object_type}")]
    WriteObject {
        object_type: &'static str,
        source: Box<dyn std::error::Error + Send + Sync>,
    },
    #[error(transparent)]
    Other(Box<dyn std::error::Error + Send + Sync>),
    #[error("{0}")]
    Unsupported(String),
}

pub type BackendResult<T> = Result<T, BackendError>;
```

Key patterns:
- Backend-specific errors (e.g., `GitBackendError`) implement `From<...> for BackendError`
- Init and load errors are separate types (`BackendInitError`, `BackendLoadError`) wrapping `Box<dyn Error>`
- Transaction errors aggregate index, op-store, and op-heads-store errors

### Concurrency Considerations

jj's concurrency model is deliberately simple:

1. **Single-writer per workspace:** The `Transaction` model ensures only one mutation happens at a time within a workspace. The operation log provides MVCC-like isolation.

2. **Multi-workspace concurrency:** Multiple workspaces can operate on the same repository concurrently because each workspace gets its own working copy and creates independent operations. The operation DAG merges these naturally.

3. **Backend locking:** `GitBackend` holds a `Mutex<gix::Repository>` for thread safety, but notes in the source that it is "most likely to be used in a single-threaded context."

4. **Stacked table locking:** File locks (`FileLock`) protect the metadata stacked table during writes.

5. **Async bridge:** Backend methods are `async` (via `async_trait`) to support future network backends, but are currently bridged to sync with `pollster::block_on()`.

6. **Op heads resolution:** When concurrent operations create divergent heads, `op_heads_store` detects this and performs automatic merge on next access.

### Implementing the Operation Log

```rust
pub trait OpStore: Send + Sync + Debug {
    fn name(&self) -> &str;
    fn root_operation_id(&self) -> &OperationId;
    fn read_view(&self, id: &ViewId) -> OpStoreResult<View>;
    fn write_view(&self, view: &View) -> OpStoreResult<ViewId>;
    fn read_operation(&self, id: &OperationId) -> OpStoreResult<Operation>;
    fn write_operation(&self, operation: &Operation) -> OpStoreResult<OperationId>;
}

// SimpleOpStore stores as protobuf files, content-addressed by BLAKE2b-512
// Operations: .jj/repo/op_store/operations/<hex>
// Views: .jj/repo/op_store/views/<hex>
```

The write path for operations:
1. Serialize `View` to protobuf, hash with BLAKE2b-512, write to `views/<hex>`
2. Serialize `Operation` (referencing the ViewId) to protobuf, hash, write to `operations/<hex>`
3. Update `op_heads_store` to point to the new operation (remove parent from heads, add new)

### Implementing the Commit Index

The index provides O(1) positional access and efficient graph queries:

```rust
pub trait Index: Send + Sync {
    fn shortest_unique_commit_id_prefix_len(&self, commit_id: &CommitId) -> usize;
    fn resolve_commit_id_prefix(&self, prefix: &HexPrefix)
        -> PrefixResolution<CommitId>;
    fn has_id(&self, commit_id: &CommitId) -> bool;
    fn is_ancestor(&self, ancestor_id: &CommitId, descendant_id: &CommitId) -> bool;
    fn common_ancestors(&self, set1: &[CommitId], set2: &[CommitId]) -> Vec<CommitId>;
    fn all_heads_for_gc(&self)
        -> Result<Box<dyn Iterator<Item = CommitId>>, AllHeadsForGcUnsupported>;
    fn heads(&self, candidates: &mut dyn Iterator<Item = &CommitId>) -> Vec<CommitId>;
    fn evaluate_revset<'index>(&'index self, expression: &ResolvedExpression,
        store: &Arc<Store>) -> Result<Box<dyn Revset + 'index>, RevsetEvaluationError>;
}
```

The default index uses stacked binary segments with generation numbers for pruning ancestor walks. Each entry is a fixed-size record containing the commit ID, change ID, generation number, and parent positions (using overflow entries for commits with many parents).

## Performance Considerations

### Object Storage
- **Loose objects are fast to write** (just compress + write file) but slow to read at scale
- **Packfiles** (via `git gc`) provide excellent read performance and space efficiency for large repos
- jj creates `refs/jj/keep/` refs in batch using `edit_references()` to avoid per-ref overhead

### Caching
- Store uses small LRU caches (100 commits, 1000 trees) behind `Mutex`
- The stacked table caches its head in `cached_extra_metadata` to avoid re-reading on every lookup
- Index segments are read once and kept in memory

### Diff Algorithm
- Pre-computes hashes for all tokens, enabling O(1) equality checks
- Uses `HashTable` from `hashbrown` (Robin Hood hashing) for the histogram
- `SmallVec<[LocalWordPosition; 2]>` avoids heap allocation for unique words (the common case)
- `max_occurrences` threshold skips very common tokens, preventing quadratic behavior

### Index
- `SmallVec<[IndexPosition; 4]>` for parent positions: zero heap allocations for normal commits
- Binary search on sorted CommitId/ChangeId arrays for O(log n) lookups
- Generation numbers enable pruned ancestor walks (skip entire subtrees)
- `AncestorsBitSet` for O(1) reachability queries on bounded subgraphs

### Working Copy
- Uses file stat (mtime, size) to detect changes without re-hashing unchanged files
- Only hashes files whose stat has changed
- Parallelizable with `rayon` for large working copies

## Code Example: Complete Storage Backend Skeleton

```rust
use std::io::Read;
use std::sync::{Arc, Mutex};
use std::time::SystemTime;
use async_trait::async_trait;
use gix;

pub struct MyGitBackend {
    repo: Mutex<gix::Repository>,
    base_repo: gix::ThreadSafeRepository,
    metadata_store: StackedTableStore,
    root_commit_id: CommitId,
    root_change_id: ChangeId,
    empty_tree_id: TreeId,
}

#[async_trait]
impl Backend for MyGitBackend {
    fn name(&self) -> &str { "git" }
    fn commit_id_length(&self) -> usize { 20 }
    fn change_id_length(&self) -> usize { 16 }
    fn root_commit_id(&self) -> &CommitId { &self.root_commit_id }
    fn root_change_id(&self) -> &ChangeId { &self.root_change_id }
    fn empty_tree_id(&self) -> &TreeId { &self.empty_tree_id }
    fn concurrency(&self) -> usize { 1 }

    async fn write_file(
        &self,
        _path: &RepoPath,
        contents: &mut (dyn Read + Send),
    ) -> BackendResult<FileId> {
        let mut bytes = Vec::new();
        contents.read_to_end(&mut bytes)?;
        let repo = self.repo.lock().unwrap();
        let oid = repo.write_blob(bytes)?;
        Ok(FileId::from_bytes(oid.as_bytes()))
    }

    async fn write_commit(
        &self,
        mut contents: Commit,
        sign_with: Option<&mut SigningFn>,
    ) -> BackendResult<(CommitId, Commit)> {
        let repo = self.repo.lock().unwrap();

        // Build Git commit object
        let git_tree_id = to_gix_oid(&contents.root_tree.to_merge().first());
        let parents: Vec<_> = contents.parents.iter()
            .filter(|id| **id != self.root_commit_id)
            .map(|id| to_gix_oid(id))
            .collect();

        let mut extra_headers = Vec::new();
        extra_headers.push((
            b"change-id".into(),
            contents.change_id.reverse_hex().into(),
        ));

        let commit_obj = gix::objs::Commit {
            tree: git_tree_id,
            parents: parents.into(),
            author: to_git_sig(&contents.author).into(),
            committer: to_git_sig(&contents.committer).into(),
            message: contents.description.clone().into(),
            extra_headers,
            encoding: None,
        };

        let git_id = repo.write_object(&commit_obj)?;
        let commit_id = CommitId::from_bytes(git_id.as_bytes());

        // Store jj metadata in sidecar
        let extras = serialize_extras(&contents);
        let (table, lock) = self.metadata_store.get_head_locked()?;
        let mut mut_table = table.start_mutation();
        mut_table.add_entry(commit_id.to_bytes(), extras);
        self.metadata_store.save_table(mut_table)?;

        // Prevent GC
        repo.edit_reference(no_gc_ref_for(&commit_id))?;

        Ok((commit_id, contents))
    }

    fn gc(&self, index: &dyn Index, keep_newer: SystemTime) -> BackendResult<()> {
        let heads = index.all_heads_for_gc()?;
        // Update no-gc refs, then run `git gc`
        recreate_no_gc_refs(&self.base_repo.to_thread_local(), heads, keep_newer)?;
        std::process::Command::new("git")
            .arg("gc")
            .current_dir(self.base_repo.path())
            .status()?;
        Ok(())
    }

    // ... read methods follow the same pattern: lock repo, find object, deserialize
}
```

## Code Example: Operation Log Transaction

```rust
pub struct Transaction {
    mut_repo: MutableRepo,
    parent_ops: Vec<Operation>,
    metadata: OperationMetadata,
}

impl Transaction {
    pub fn new(mut_repo: MutableRepo, settings: &UserSettings) -> Self {
        let parent_ops = vec![mut_repo.base_repo().operation().clone()];
        Transaction {
            mut_repo,
            parent_ops,
            metadata: OperationMetadata::new(settings),
        }
    }

    pub fn commit(self, description: impl Into<String>) -> Result<Arc<ReadonlyRepo>, Error> {
        let mut metadata = self.metadata;
        metadata.description = description.into();
        metadata.end_time = Timestamp::now();

        // Snapshot the new view
        let view = self.mut_repo.take_view();
        let view_id = self.op_store().write_view(&view)?;

        // Create the operation
        let operation = Operation {
            view_id,
            parents: self.parent_ops.iter().map(|op| op.id().clone()).collect(),
            metadata,
        };
        let op_id = self.op_store().write_operation(&operation)?;

        // Update op heads (atomically: remove parents, add new)
        let op_heads_store = self.mut_repo.base_repo().op_heads_store();
        op_heads_store.update_op_heads(&self.parent_ops, &op_id)?;

        // Build index for the new operation
        let index = self.index_store().write_index(
            self.mut_repo.mutable_index(),
            &op_id,
        )?;

        Ok(ReadonlyRepo::new(/* ... */))
    }
}
```

## Code Example: Histogram Diff Core

```rust
struct Histogram<'input> {
    word_to_positions: HashTable<(HashedWord<'input>, SmallVec<[Position; 2]>)>,
}

impl<'input> Histogram<'input> {
    fn calculate(
        source: &DiffSource<'input>,
        comparator: &WordComparator,
        max_occurrences: usize,
    ) -> Self {
        let mut table = HashTable::new();
        for (i, word) in source.hashed_words().enumerate() {
            table.entry(
                word.hash,
                |(w, _)| comparator.eq(w.text, word.text),
                |(w, _)| w.hash,
            )
            .and_modify(|(_, positions)| {
                if positions.len() <= max_occurrences {
                    positions.push(Position(i));
                }
            })
            .or_insert_with(|| (word, smallvec![Position(i)]));
        }
        Histogram { word_to_positions: table }
    }
}

// The diff then:
// 1. Build histogram of right side
// 2. Find lowest-count matching tokens as anchors
// 3. Use anchors to divide into sub-problems
// 4. Recursively match sub-problems
// 5. Produce DiffHunk::Matching / DiffHunk::Different
```

## Summary of Key Design Decisions

| Decision | jj's Choice | Rationale |
|----------|-------------|-----------|
| Object storage | Git ODB (SHA-1, zlib, packfiles) | Full Git compatibility, proven at scale |
| Metadata storage | Protobuf + BLAKE2b-512 | Portable, fast, deterministic hashing |
| Sidecar metadata | Stacked tables (sorted keys, binary search) | Append-only, compactable, concurrent-safe |
| Content hashing | BLAKE2b-512 for internal, SHA-1 for Git objects | BLAKE2 is faster and more secure than SHA-1 |
| Conflict model | `Merge<T>` algebraic type (SmallVec-backed) | Composable, structured, survives rebases |
| Change tracking | Random 16-byte ChangeId + predecessors list | Stable across rewrites, decoupled from content |
| Diff algorithm | Histogram diff with hash pre-computation | Good performance on both structured code and prose |
| Index format | Stacked binary segments with generation numbers | O(1) positional access, efficient ancestor queries |
| Async model | `async_trait` + `pollster::block_on()` | Future-proofs for network backends |
| Caching | Small LRU (100 commits, 1000 trees) behind Mutex | Sufficient for sequential access patterns |
| GC strategy | `refs/jj/keep/` + `git gc` subprocess | Leverages Git's proven GC, simple implementation |
| Serialization | prost (protobuf) for structured data | Schema evolution, compact binary format |
| Error handling | `thiserror` with structured enums | Composable, context-rich errors |
| Concurrency | Transaction-per-workspace, operation DAG for cross-workspace | Simple model, safe concurrent multi-workspace |
