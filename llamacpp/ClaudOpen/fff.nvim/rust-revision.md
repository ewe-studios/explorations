# FFF.nvim Rust Revision

**Repository:** `/home/darkvoid/Boxxed/@formulas/src.rust/src.llamacpp/src.ClaudOpen/fff.nvim`

This document covers Rust-specific patterns, idioms, and implementation details used throughout the fff.nvim codebase.

---

## Workspace Configuration

### Cargo.toml Structure

```toml
[workspace]
members = [
  "crates/fff-c",
  "crates/fff-core",
  "crates/fff-mcp",
  "crates/fff-nvim",
  "crates/fff-query-parser",
  "crates/fff-grep",
]
resolver = "2"  # Use Rust 2021 edition resolver

[workspace.dependencies]
# Shared dependencies with versions
ahash = "0.8"
bindet = "0.3"
blake3 = "1.8.2"
chrono = { version = "0.4", features = ["serde"] }
git2 = { version = "0.20.2", default-features = false, features = ["vendored-libgit2"] }
heed = "0.22.0"
mlua = { version = "0.11.1", features = ["module", "luajit"] }
rayon = "1.8.0"
thiserror = "2.0.10"
tracing = "0.1"

# Release profile - maximum optimization
[profile.release]
opt-level = 3
lto = "fat"              # Full Link-Time Optimization
codegen-units = 1        # Single codegen unit for better optimization
strip = true             # Strip debug symbols
```

### Feature Flags

```toml
[features]
default = []
zlob = ["fff/zlob"]  # Optional glob support via zlob crate
```

Usage:
```bash
cargo build --release --features zlob
```

---

## Error Handling

### Custom Error Type (error.rs)

```rust
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Database error: {0}")]
    Db(#[from] heed::Error),

    #[error("Failed to start read transaction: {0}")]
    DbStartReadTxn(heed::Error),

    #[error("Failed to start write transaction: {0}")]
    DbStartWriteTxn(heed::Error),

    #[error("Git error: {0}")]
    Git(#[from] git2::Error),

    #[error("File picker not initialized")]
    FilePickerMissing,

    #[error("Invalid path: {0}")]
    InvalidPath(PathBuf),
}

pub type Result<T, E = Error> = std::result::Result<T, E>;
```

### Error Conversion for Lua

```rust
// Convert Error to LuaError
pub fn to_lua_error(err: Error) -> LuaError {
    LuaError::RuntimeError(err.to_string())
}

// Extension trait for ergonomic conversion
pub trait IntoLuaResult<T> {
    fn into_lua_result(self) -> LuaResult<T>;
}

impl<T, E: std::fmt::Display> IntoLuaResult<T> for Result<T, E> {
    fn into_lua_result(self) -> LuaResult<T> {
        self.map_err(|e| LuaError::RuntimeError(e.to_string()))
    }
}
```

---

## Shared State Patterns

### RwLock Wrapped Option

```rust
/// Thread-safe shared handle for FilePicker
pub struct SharedPicker(
    Arc<RwLock<Option<FilePicker>>>
);

impl SharedPicker {
    pub fn read(&self) -> Result<ArcRwLockReadGuard<Option<FilePicker>>> {
        self.0.read().map_err(Error::from)
    }

    pub fn write(&self) -> Result<ArcRwLockWriteGuard<Option<FilePicker>>> {
        self.0.write().map_err(Error::from)
    }
}

impl Default for SharedPicker {
    fn default() -> Self {
        Self(Arc::new(RwLock::new(None)))
    }
}
```

### Usage Pattern

```rust
// Read access (multiple readers allowed)
let guard = FILE_PICKER.read()?;
let picker = guard.as_ref().ok_or(Error::FilePickerMissing)?;
let files = picker.get_files();

// Write access (exclusive)
let mut guard = FILE_PICKER.write()?;
let picker = guard.as_mut().ok_or(Error::FilePickerMissing)?;
picker.trigger_rescan(&FRECENCY)?;
```

### Avoiding Deadlocks

```rust
// WRONG: Holding read lock while waiting for write lock
let guard = FILE_PICKER.read()?;
let picker = guard.as_ref().unwrap();
picker.wait_for_scan(); // Scan thread needs write lock - DEADLOCK!

// CORRECT: Extract needed data, release lock, then wait
let scan_signal = {
    let guard = FILE_PICKER.read()?;
    let picker = guard.as_ref().unwrap();
    picker.scan_signal()  // Clone Arc<AtomicBool>
};  // Read lock released here

// Now safe to wait (scan thread can acquire write lock)
while scan_signal.load(Ordering::Relaxed) {
    std::thread::sleep(Duration::from_millis(10));
}
```

---

## Lazy Initialization

### OnceLock for File Content

```rust
pub struct FileItem {
    // ... other fields
    content: OnceLock<FileContent>,
}

impl FileItem {
    pub fn get_content(&self, budget: &ContentCacheBudget) -> Option<FileContentRef> {
        // Fast path: already initialized (lock-free read)
        if let Some(cached) = self.content.get() {
            return Some(FileContentRef::Cached(cached));
        }

        // Slow path: initialize
        let content = create_mmap(&self.path)?;

        // Check if we can cache it
        if !budget.is_over() {
            self.content.set(content).ok()?;  // First writer wins
            budget.increment();
            // Need to re-get to return borrowed reference
            Some(FileContentRef::Cached(self.content.get().unwrap()))
        } else {
            // Return temporary (will be unmapped on drop)
            Some(FileContentRef::Temp(content))
        }
    }

    pub fn invalidate_mmap(&mut self, budget: &ContentCacheBudget) {
        if self.content.get().is_some() {
            budget.cached_count.fetch_sub(1, Ordering::Relaxed);
            budget.cached_bytes.fetch_sub(self.size, Ordering::Relaxed);
        }
        // Reset to uninitialized state
        self.content = OnceLock::new();
    }
}
```

---

## Parallel Processing with Rayon

### Basic Parallel Iteration

```rust
use rayon::prelude::*;

// Parallel grep search
let results: Vec<GrepFileMatch> = files
    .par_iter()  // Parallel iterator
    .filter_map(|file| {
        // Skip binary/large files
        if file.is_binary || file.size > max_size {
            return None;
        }

        // Search file content
        let content = file.get_content()?;
        let mut sink = MatchSink::default();
        searcher.search_slice(&matcher, &content, &mut sink).ok()?;

        Some(GrepFileMatch {
            path: file.relative_path.clone(),
            matches: sink.matches,
        })
    })
    .collect();
```

### Thread Pool Configuration

```rust
// Control thread count via search options
pub struct FuzzySearchOptions {
    pub max_threads: usize,  // 0 = use all available
}

// Pass to frizbee
let matches = neo_frizbee::match_list_parallel(
    query,
    &haystack,
    &config,
    options.max_threads,
);
```

### Parallel Constraint Checking

```rust
// Use parallel iteration for large datasets
const PAR_THRESHOLD: usize = 10_000;

fn apply_constraints<'a>(
    files: &'a [FileItem],
    constraints: &[Constraint],
) -> Vec<&'a FileItem> {
    if files.len() < PAR_THRESHOLD {
        // Sequential for small datasets (avoids thread pool overhead)
        files.iter()
            .filter(|file| matches_all_constraints(file, constraints))
            .collect()
    } else {
        // Parallel for large datasets
        files.par_iter()
            .filter(|file| matches_all_constraints(file, constraints))
            .collect()
    }
}
```

---

## Memory Management

### Mmap vs Buffer

```rust
/// File contents - mmap on Unix, heap buffer on Windows
#[derive(Debug)]
pub enum FileContent {
    #[cfg(not(target_os = "windows"))]
    Mmap(memmap2::Mmap),  // Zero-copy, kernel-managed
    Buffer(Vec<u8>),      // Heap-allocated (Windows)
}

impl Deref for FileContent {
    type Target = [u8];

    fn deref(&self) -> &[u8] {
        match self {
            #[cfg(not(target_os = "windows"))]
            FileContent::Mmap(m) => m,
            FileContent::Buffer(b) => b,
        }
    }
}
```

### Borrowed Content Reference

```rust
/// Borrows from FileItem's cache or owns temporary mmap
pub enum FileContentRef<'a> {
    Cached(&'a [u8]),   // Borrowed from OnceLock cache
    Temp(FileContent),  // Owned temporary mmap
}

impl Deref for FileContentRef<'_> {
    type Target = [u8];

    fn deref(&self) -> &[u8] {
        match self {
            FileContentRef::Cached(s) => s,
            FileContentRef::Temp(c) => c,
        }
    }
}

// Usage - transparent to caller
fn search_file(file: &FileItem) {
    let content = file.get_content(budget)?;  // FileContentRef
    let matches = find_matches(&content);     // Deref to &[u8]
}  // Temp variant unmapped here
```

### Content Cache Budget

```rust
pub struct ContentCacheBudget {
    pub max_files: usize,
    pub max_bytes: usize,
    pub cached_count: AtomicUsize,
    pub cached_bytes: AtomicUsize,
}

impl ContentCacheBudget {
    pub fn is_over(&self) -> bool {
        self.cached_count.load(Ordering::Relaxed) >= self.max_files
            || self.cached_bytes.load(Ordering::Relaxed) >= self.max_bytes
    }

    pub fn increment(&self) {
        self.cached_count.fetch_add(1, Ordering::Relaxed);
    }
}

// Default budget: 30,000 files or 1GB
impl Default for ContentCacheBudget {
    fn default() -> Self {
        Self {
            max_files: 30_000,
            max_bytes: 1024 * 1024 * 1024,
            cached_count: AtomicUsize::new(0),
            cached_bytes: AtomicUsize::new(0),
        }
    }
}
```

---

## LMDB Database Patterns

### Database Initialization

```rust
pub struct FrecencyTracker {
    env: Env,
    db: Database<Bytes, SerdeBincode<VecDeque<u64>>>,
}

impl FrecencyTracker {
    pub fn new(db_path: &Path, use_unsafe_no_lock: bool) -> Result<Self> {
        fs::create_dir_all(db_path).map_err(Error::CreateDir)?;

        // Unsafe flags for performance (no locking, no sync)
        // Safe because: single-process access, crash recovery via purge
        let env = unsafe {
            let mut opts = EnvOpenOptions::new();
            opts.map_size(24 * 1024 * 1024);  // 24 MB
            if use_unsafe_no_lock {
                opts.flags(EnvFlags::NO_LOCK | EnvFlags::NO_SYNC | EnvFlags::NO_META_SYNC);
            }
            opts.open(db_path).map_err(Error::EnvOpen)?
        };

        env.clear_stale_readers()
            .map_err(Error::DbClearStaleReaders)?;

        let mut wtxn = env.write_txn().map_err(Error::DbStartWriteTxn)?;
        let db = env
            .create_database(&mut wtxn, None)
            .map_err(Error::DbCreate)?;
        wtxn.commit().map_err(Error::DbCommit)?;

        Ok(FrecencyTracker { env, db })
    }
}
```

### Key Generation with Blake3

```rust
fn create_file_key(path: &Path) -> Result<[u8; 32]> {
    let path_str = path.to_str()
        .ok_or_else(|| Error::InvalidPath(path.to_path_buf()))?;

    Ok(*blake3::hash(path_str.as_bytes()).as_bytes())
}

// For composite keys (project + query)
fn create_query_key(project_path: &Path, query: &str) -> Result<[u8; 32]> {
    let project_str = project_path.to_str()
        .ok_or_else(|| Error::InvalidPath(project_path.to_path_buf()))?;

    let mut hasher = blake3::Hasher::default();
    hasher.update(project_str.as_bytes());
    hasher.update(b"::");  // Separator
    hasher.update(query.as_bytes());

    Ok(*hasher.finalize().as_bytes())
}
```

### Transaction Pattern

```rust
pub fn track_access(&self, file_path: &Path) -> Result<()> {
    let file_key = create_file_key(file_path)?;
    let now = self.get_now();

    let mut wtxn = self.env.write_txn().map_err(Error::DbStartWriteTxn)?;

    // Get existing timestamps
    let mut timestamps = self.db
        .get(&mut wtxn, &file_key)
        .map_err(Error::DbRead)?
        .unwrap_or_default();

    // Append new timestamp
    timestamps.push_back(now);

    // Limit history size
    while timestamps.len() > MAX_HISTORY_ENTRIES {
        timestamps.pop_front();
    }

    // Write back
    self.db.put(&mut wtxn, &file_key, &timestamps)
        .map_err(Error::DbWrite)?;

    wtxn.commit().map_err(Error::DbCommit)?;
    Ok(())
}
```

### Read Transaction Pattern

```rust
pub fn get_frecency_score(&self, file_path: &Path) -> Result<i32> {
    let file_key = create_file_key(file_path)?;
    let rtxn = self.env.read_txn().map_err(Error::DbStartReadTxn)?;

    let timestamps = self.db
        .get(&rtxn, &file_key)
        .map_err(Error::DbRead)?;

    rtxn.commit().map_err(Error::DbCommit)?;

    Ok(match timestamps {
        Some(ts) => calculate_frecency_score(&ts, self.get_now()),
        None => 0,
    })
}
```

### Background GC

```rust
pub fn spawn_gc(shared: SharedFrecency, db_path: String, use_unsafe_no_lock: bool) {
    std::thread::Builder::new()
        .name("fff-frecency-gc".into())
        .spawn(move || {
            Self::run_frecency_gc(shared, db_path, use_unsafe_no_lock);
        })
        .expect("Failed to spawn GC thread");
}

fn run_frecency_gc(shared: SharedFrecency, db_path: String, use_unsafe_no_lock: bool) {
    // Phase 1: Purge stale entries (read lock sufficient)
    let (deleted, pruned) = {
        let guard = match shared.read() {
            Ok(g) => g,
            Err(e) => {
                tracing::debug!("Failed to acquire read lock: {e}");
                return;
            }
        };
        let Some(ref tracker) = *guard else { return };

        match tracker.purge_stale_entries() {
            Ok(result) => result,
            Err(e) => {
                tracing::debug!("Purge failed: {e}");
                return;
            }
        }
    };

    if deleted > 0 || pruned > 0 {
        tracing::info!(deleted, pruned, "Frecency GC purged entries");
    }

    // Phase 2: Compact (write lock required)
    let mut guard = match shared.write() {
        Ok(g) => g,
        Err(e) => {
            tracing::debug!("Failed to acquire write lock: {e}");
            return;
        }
    };

    let Some(ref mut tracker) = *guard else { return };

    // Force compaction by closing and reopening env
    drop(tracker);  // Drop tracker to release env
    *guard = Some(FrecencyTracker::new(&db_path, use_unsafe_no_lock).unwrap());

    tracing::info!("Frecency GC completed");
}
```

---

## Filesystem Watching

### Debouncer Setup

```rust
type Debouncer = notify_debouncer_full::Debouncer<notify::RecommendedWatcher, NoCache>;

fn create_debouncer(
    base_path: PathBuf,
    git_workdir: Option<PathBuf>,
    shared_picker: SharedPicker,
    shared_frecency: SharedFrecency,
    mode: FFFMode,
) -> Result<Debouncer> {
    let config = Config::default().with_follow_symlinks(false);

    let git_workdir_for_handler = git_workdir.clone();
    let mut debouncer = new_debouncer_opt(
        DEBOUNCE_TIMEOUT,  // 250ms
        Some(DEBOUNCE_TIMEOUT / 2),  // Tick rate
        move |result: DebounceEventResult| {
            match result {
                Ok(events) => handle_debounced_events(
                    events,
                    &git_workdir_for_handler,
                    &shared_picker,
                    &shared_frecency,
                    mode,
                ),
                Err(errors) => error!("File watcher errors: {:?}", errors),
            }
        },
        NoCache::new(),  // Custom cache impl
        config,
    )?;

    // Selective watching (only non-ignored dirs)
    let watch_dirs = collect_non_ignored_dirs(&base_path);

    if watch_dirs.len() > MAX_SELECTIVE_WATCH_DIRS {
        debouncer.watch(&base_path, RecursiveMode::Recursive)?;
    } else {
        debouncer.watch(&base_path, RecursiveMode::NonRecursive)?;
        for dir in &watch_dirs {
            debouncer.watch(dir, RecursiveMode::Recursive)?;
        }
    }

    Ok(debouncer)
}
```

### Owner Thread Pattern

```rust
pub struct BackgroundWatcher {
    stop_signal: Arc<AtomicBool>,
    owner_thread: Option<std::thread::JoinHandle<()>>,
}

impl BackgroundWatcher {
    pub fn new(...) -> Result<Self> {
        let debouncer = create_debouncer(...)?;
        let stop_signal = Arc::new(AtomicBool::new(false));
        let stop_clone = Arc::clone(&stop_signal);

        // Owner thread keeps debouncer alive
        let owner_thread = std::thread::Builder::new()
            .name("fff-watcher-owner".into())
            .spawn(move || {
                while !stop_clone.load(Ordering::Acquire) {
                    std::thread::park_timeout(Duration::from_secs(1));
                }
                // Proper cleanup: join debouncer thread
                debouncer.stop();

                // Windows workaround: give OS time to reclaim I/O thread
                #[cfg(windows)]
                std::thread::sleep(Duration::from_millis(250));
            })?;

        Ok(Self {
            stop_signal,
            owner_thread: Some(owner_thread),
        })
    }

    pub fn stop(&mut self) {
        // Signal stop
        self.stop_signal.store(true, Ordering::Release);

        // Unpark and join
        if let Some(handle) = self.owner_thread.take() {
            handle.thread().unpark();
            handle.join().expect("Watcher thread panicked");
        }
    }
}

impl Drop for BackgroundWatcher {
    fn drop(&mut self) {
        self.stop();
    }
}
```

---

## Sorting and Buffering

### Sort Buffer Pattern

```rust
/// Pre-allocated buffer for efficient sorting
pub fn sort_by_score_with_buffer(items: &mut [FileItem], scores: &[Score]) {
    // Pre-allocate buffer to avoid reallocations
    let mut buffer = Vec::with_capacity(items.len());

    // Glidesort: stable, fast, low allocation
    glidesort::sort_by_key(items, |item| {
        // Find corresponding score
        let idx = items.iter().position(|i| i.path == item.path).unwrap();
        scores[idx].total
    }, &mut buffer);
}
```

### Custom Sort Keys

```rust
/// Sort GrepResult by frecency
fn sort_by_frecency(results: &mut [GrepFileMatch], frecency: &FrecencyTracker) {
    let mut buffer = Vec::with_capacity(results.len());

    glidesort::sort_by_cached_key(results, |file| {
        frecency.get_frecency_score(&file.path).unwrap_or(0)
    }, &mut buffer);
}
```

---

## Trait Patterns

### Constrainable Trait

```rust
/// Trait for items that can be filtered by constraints
pub trait Constrainable {
    fn relative_path(&self) -> &str;
    fn file_name(&self) -> &str;
    fn git_status(&self) -> Option<git2::Status>;
}

impl Constrainable for FileItem {
    fn relative_path(&self) -> &str { &self.relative_path }
    fn file_name(&self) -> &str { &self.file_name }
    fn git_status(&self) -> Option<git2::Status> { self.git_status }
}

// Generic constraint application
pub fn apply_constraints<'a, T: Constrainable>(
    items: &'a [T],
    constraints: &[Constraint],
) -> Vec<&'a T> {
    items.iter()
        .filter(|item| {
            constraints.iter().all(|c| item_matches_constraint(item, c))
        })
        .collect()
}
```

### DbHealthChecker Trait

```rust
pub trait DbHealthChecker {
    fn get_env(&self) -> &heed::Env;

    fn count_entries(&self) -> Result<Vec<(&'static str, u64)>>;

    fn get_health(&self) -> Result<DbHealth> {
        let rtxn = self.get_env().read_txn().map_err(Error::DbStartReadTxn)?;
        let disk_size = self.get_env().info().map_err(Error::DbRead)?.map_size;
        let entry_counts = self.count_entries()?;

        Ok(DbHealth {
            path: self.get_env().path().unwrap().to_string_lossy().to_string(),
            disk_size,
            entry_counts: entry_counts
                .into_iter()
                .map(|(k, v)| (k.to_string(), v))
                .collect(),
        })
    }
}

impl DbHealthChecker for FrecencyTracker {
    fn get_env(&self) -> &heed::Env { &self.env }

    fn count_entries(&self) -> Result<Vec<(&'static str, u64)>> {
        let rtxn = self.env.read_txn().map_err(Error::DbStartReadTxn)?;
        let count = self.db.len(&rtxn).map_err(Error::DbRead)?;
        Ok(vec![("absolute_frecency_entries", count)])
    }
}
```

---

## Allocation-Free Path Matching

### Extension Check

```rust
/// Check if file extension matches (no allocation)
#[inline]
pub fn file_has_extension(file_name: &str, ext: &str) -> bool {
    if file_name.len() <= ext.len() + 1 {
        return false;  // Too short for ".ext"
    }
    let start = file_name.len() - ext.len() - 1;
    file_name.as_bytes().get(start) == Some(&b'.')
        && file_name[start + 1..].eq_ignore_ascii_case(ext)
}
```

### Path Segment Check

```rust
/// Check if path contains segment (no allocation)
#[inline]
pub fn path_contains_segment(path: &str, segment: &str) -> bool {
    // Check segment/ at start
    if path.len() > segment.len()
        && path.as_bytes()[segment.len()] == b'/'
        && path[..segment.len()].eq_ignore_ascii_case(segment)
    {
        return true;
    }

    // Scan for /segment/ using byte scanning
    for i in 0..path.len().saturating_sub(segment.len() + 1) {
        if path.as_bytes()[i] == b'/' {
            let start = i + 1;
            let end = start + segment.len();
            if end < path.len()
                && path.as_bytes()[end] == b'/'
                && path[start..end].eq_ignore_ascii_case(segment)
            {
                return true;
            }
        }
    }
    false
}
```

### Case-Insensitive Substring

```rust
/// Case-insensitive ASCII substring search (no allocation)
#[inline]
fn contains_ascii_ci(haystack: &str, needle: &str) -> bool {
    let h = haystack.as_bytes();
    let n = needle.as_bytes();

    if n.len() > h.len() {
        return false;
    }
    if n.is_empty() {
        return true;
    }

    let first = n[0];
    for i in 0..=(h.len() - n.len()) {
        if h[i].to_ascii_lowercase() == first
            && h[i..i + n.len()]
                .iter()
                .zip(n)
                .all(|(a, b)| a.to_ascii_lowercase() == *b)
        {
            return true;
        }
    }
    false
}
```

---

## Definition Detection

### Keyword Scanning

```rust
/// Modifier keywords that can precede a definition
const MODIFIERS: &[&[u8]] = &[
    b"pub", b"export", b"default", b"async", b"abstract",
    b"unsafe", b"static", b"protected", b"private", b"public",
];

/// Definition keywords
const DEF_KEYWORDS: &[&[u8]] = &[
    b"struct", b"fn", b"enum", b"trait", b"impl",
    b"class", b"interface", b"function", b"def", b"func",
];

fn skip_modifiers(mut s: &[u8]) -> &[u8] {
    loop {
        // Handle pub(crate) style visibility
        if s.starts_with(b"pub(")
            && let Some(end) = s.iter().position(|&b| b == b')')
        {
            s = skip_ws(&s[end + 1..]);
            continue;
        }

        // Try each modifier
        let mut matched = false;
        for &kw in MODIFIERS {
            if s.starts_with(kw) {
                let rest = &s[kw.len()..];
                if rest.first().is_some_and(|b| b.is_ascii_whitespace()) {
                    s = skip_ws(rest);
                    matched = true;
                    break;
                }
            }
        }
        if !matched {
            return s;
        }
    }
}

fn is_definition_keyword(s: &[u8]) -> bool {
    for &kw in DEF_KEYWORDS {
        if s.starts_with(kw) {
            let after = s.get(kw.len());
            // Word boundary check
            if after.is_none_or(|b| !b.is_ascii_alphanumeric() && *b != b'_') {
                return true;
            }
        }
    }
    false
}

pub fn is_definition_line(line: &str) -> bool {
    let s = skip_modifiers(line.trim_start().as_bytes());
    is_definition_keyword(s)
}
```

---

## mlua Integration

### Function Export Pattern

```rust
#[mlua::lua_module(skip_memory_check)]
fn fff_nvim(lua: &Lua) -> LuaResult<LuaTable> {
    create_exports(lua)
}

fn create_exports(lua: &Lua) -> LuaResult<LuaTable> {
    let exports = lua.create_table()?;

    exports.set("init_db", lua.create_function(init_db)?)?;
    exports.set("init_file_picker", lua.create_function(init_file_picker)?)?;
    exports.set("fuzzy_search_files", lua.create_function(fuzzy_search_files)?)?;
    exports.set("live_grep", lua.create_function(live_grep)?)?;
    exports.set("track_access", lua.create_function(track_access)?)?;
    // ... more exports

    Ok(exports)
}
```

### Lua Function Signature

```rust
#[allow(clippy::type_complexity)]
pub fn fuzzy_search_files(
    lua: &Lua,
    (
        query,
        max_threads,
        current_file,
        combo_boost_score_multiplier,
        min_combo_count,
        page_index,
        page_size,
    ): (
        String,
        usize,
        Option<String>,
        i32,
        Option<u32>,
        Option<usize>,
        Option<usize>,
    ),
) -> LuaResult<LuaValue> {
    // Implementation
}
```

### Custom Lua Types

```rust
/// Convert SearchResult to Lua table
pub struct SearchResultLua {
    pub items: LuaTable,
    pub scores: LuaTable,
    pub total_matched: usize,
    pub total_files: usize,
}

impl From<SearchResult<'_>> for SearchResultLua {
    fn from(result: SearchResult) -> Self {
        // Conversion logic
    }
}

impl<'lua> IntoLua<'lua> for SearchResultLua {
    fn into_lua(self, lua: &'lua Lua) -> LuaResult<LuaValue> {
        let table = lua.create_table()?;
        table.set("items", self.items)?;
        table.set("scores", self.scores)?;
        table.set("total_matched", self.total_matched)?;
        table.set("total_files", self.total_files)?;
        Ok(LuaValue::Table(table))
    }
}
```

---

## Global Allocator

### MiMalloc for Performance

```rust
use mimalloc::MiMalloc;

#[global_allocator]
static GLOBAL: MiMalloc = MiMalloc;
```

Benefits:
- Faster allocation/deallocation than system allocator
- Better cache locality
- Reduced fragmentation
- Thread-local caches reduce contention

---

## Tracing and Logging

### Init Function

```rust
pub fn init_tracing(log_file_path: &str, log_level: Option<&str>) -> Result<String> {
    let log_level = log_level.unwrap_or("info");

    // Parse log level
    let directive = log_level.parse::<tracing_subscriber::filter::LevelFilter>()?;

    // File appender
    let file_appender = tracing_appender::rolling::never(".", log_file_path);
    let (non_blocking, _guard) = tracing_appender::non_blocking(file_appender);

    // Install panic hook before tracing
    install_panic_hook();

    // Initialize subscriber
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::builder()
                .with_default_directive(directive.into())
                .from_env_lossy()
        )
        .with_writer(non_blocking)
        .with_ansi(false)
        .with_target(true)
        .with_thread_ids(true)
        .with_thread_names(true)
        .init();

    Ok(log_file_path.to_string())
}
```

### Panic Hook

```rust
pub fn install_panic_hook() {
    let default_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |panic_info| {
        // Log panic
        tracing::error!("Panic: {:?}", panic_info);

        // Call default hook (prints to stderr)
        default_hook(panic_info);
    }));
}
```

### Usage

```rust
#[tracing::instrument(skip(shared), fields(db_path = %db_path))]
fn run_frecency_gc(shared: SharedFrecency, db_path: String, ...) {
    tracing::info!("Starting frecency GC");

    let (deleted, pruned) = tracker.purge_stale_entries()?;

    tracing::info!(
        deleted,
        pruned,
        elapsed = ?start.elapsed(),
        "Frecency GC purged entries"
    );
}
```

---

## Atomic Operations

### Scan Signal Pattern

```rust
pub struct FilePicker {
    scan_signal: Arc<AtomicBool>,  // true = scanning, false = complete
}

impl FilePicker {
    pub fn new(...) -> Self {
        let scan_signal = Arc::new(AtomicBool::new(true));

        // Spawn background scan
        let signal_clone = Arc::clone(&scan_signal);
        std::thread::spawn(move || {
            // ... scan files ...

            // Signal completion
            signal_clone.store(false, Ordering::Release);
        });

        Self { scan_signal, ... }
    }

    pub fn scan_signal(&self) -> Arc<AtomicBool> {
        Arc::clone(&self.scan_signal)
    }

    pub fn is_scan_active(&self) -> bool {
        self.scan_signal.load(Ordering::Acquire)
    }
}
```

### Progress Counters

```rust
pub struct ScanProgress {
    pub scanned_files_count: AtomicUsize,
    pub is_scanning: AtomicBool,
}

impl FilePicker {
    pub fn get_scan_progress(&self) -> ScanProgress {
        ScanProgress {
            scanned_files_count: self.progress.scanned_files_count.load(Ordering::Relaxed),
            is_scanning: self.progress.is_scanning.load(Ordering::Acquire),
        }
    }
}
```

---

## Platform-Specific Code

### Conditional Compilation

```rust
#[cfg(not(target_os = "windows"))]
pub enum FileContent {
    Mmap(memmap2::Mmap),
    Buffer(Vec<u8>),
}

#[cfg(target_os = "windows")]
pub enum FileContent {
    Buffer(Vec<u8>),  // Windows doesn't support mmap well
}
```

### Platform Detection

```rust
// In Lua (for FFI loading)
local is_windows = jit.os:lower() == 'windows'

local function get_lib_extension()
  if jit.os:lower() == 'mac' or jit.os:lower() == 'osx' then
    return '.dylib'
  end
  if is_windows then return '.dll' end
  return '.so'
end
```

---

## Testing Patterns

### Unit Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_file_has_extension() {
        assert!(file_has_extension("main.rs", "rs"));
        assert!(!file_has_extension("main.txt", "rs"));
    }

    #[test]
    fn test_path_contains_segment() {
        assert!(path_contains_segment("src/main.rs", "src"));
        assert!(path_contains_segment("src/main.rs", "src/main"));
        assert!(!path_contains_segment("xsrc/main.rs", "src"));
    }
}
```

### Integration Tests

```rust
// crates/fff-core/tests/grep_integration.rs
#[test]
fn test_grep_basic() {
    let temp_dir = tempfile::tempdir().unwrap();
    create_test_files(&temp_dir);

    let picker = FilePicker::new(FilePickerOptions {
        base_path: temp_dir.path().to_string_lossy().to_string(),
        ..Default::default()
    }).unwrap();

    let options = GrepSearchOptions {
        mode: GrepMode::PlainText,
        ..Default::default()
    };

    let result = picker.grep("test_query", &options);
    assert!(result.total_matched > 0);
}
```

### Benchmarks

```rust
// crates/fff-core/benches/memmem_bench.rs
use criterion::{black_box, criterion_group, criterion_main, Criterion};

fn bench_memmem(c: &mut Criterion) {
    let haystack = "the quick brown fox jumps over the lazy dog";
    let needle = "brown";

    c.bench_function("memmem_search", |b| {
        b.iter(|| {
            black_box(contains_ascii_ci(
                black_box(haystack),
                black_box(needle)
            ))
        })
    });
}

criterion_group!(benches, bench_memmem);
criterion_main!(benches);
```

---

## Next Steps

- **[production-grade.md](./production-grade.md)** - Production deployment and optimization
