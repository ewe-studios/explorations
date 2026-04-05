# FFF.nvim Architecture Exploration

**Repository:** `/home/darkvoid/Boxxed/@formulas/src.rust/src.llamacpp/src.ClaudOpen/fff.nvim`

## Repository Structure

```
fff.nvim/
├── Cargo.toml              # Workspace root (6 crates)
├── lua/
│   ├── fff.lua             # Entry point
│   └── fff/
│       ├── main.lua        # Main API (find_files, live_grep)
│       ├── core.lua        # Initialization logic
│       ├── conf.lua        # Configuration management
│       ├── picker_ui.lua   # UI rendering (96KB!)
│       ├── file_renderer.lua
│       ├── list_renderer.lua
│       ├── combo_renderer.lua
│       ├── scrollbar.lua
│       ├── location_utils.lua
│       ├── git_utils.lua
│       ├── treesitter_hl.lua
│       ├── download.lua    # Binary download logic
│       ├── health.lua
│       ├── fuzzy.lua       # Fuzzy helper
│       ├── utils.lua
│       ├── rust/           # FFI loading
│       ├── file_picker/    # Preview, actions
│       └── grep/           # Grep renderer
├── crates/
│   ├── fff-c/              # C FFI layer
│   ├── fff-core/           # Core search engine (fff-search)
│   ├── fff-grep/           # Grep primitives
│   ├── fff-mcp/            # MCP server binary
│   ├── fff-nvim/           # Neovim Lua module
│   └── fff-query-parser/   # Query syntax parsing
├── plugin/
│   └── fff.lua             # Plugin bootstrap
├── doc/
│   └── fff.txt             # Vim help docs
├── packages/               # npm packages (Bun, Node)
└── tests/                  # Lua test specs
```

---

## Workspace Crates

### Crate Dependency Graph

```
                    fff-nvim (Neovim module)
                    fff-mcp (MCP server binary)
                    fff-c (C FFI library)
                           │
                    ┌──────┴──────┐
                    │             │
              fff-core      fff-query-parser
              (fff-search)   (optional: zlob)
                    │
                    │
               fff-grep
           (standalone lib)
```

### Crate Details

| Crate | Purpose | Key Features |
|-------|---------|--------------|
| **fff-grep** | Grep primitives | SIMD line matching, Aho-Corasick |
| **fff-query-parser** | Query syntax | Constraints, globs, git filters |
| **fff-core** | Search engine | FilePicker, Frecency, Grep, Git |
| **fff-nvim** | Neovim module | mlua FFI, mimalloc |
| **fff-mcp** | MCP server | rmcp, Tokio, CLI |
| **fff-c** | C FFI | cbindgen, serde_json |

---

## Core Architecture: fff-core

### Module Breakdown

```
fff-core/src/
├── lib.rs                 # Public API, re-exports
├── file_picker.rs         # Main indexing/search engine (54KB)
├── frecency.rs            # LMDB access tracking (20KB)
├── query_tracker.rs       # Query history (14KB)
├── grep.rs                # Content search (81KB)
├── score.rs               # Scoring system (30KB)
├── constraints.rs         # Constraint filtering (15KB)
├── background_watcher.rs  # Filesystem watching (22KB)
├── git.rs                 # Git status cache (5KB)
├── case_insensitive_memmem.rs  # SIMD substring search (25KB)
├── sort_buffer.rs         # Efficient sorting (5KB)
├── path_utils.rs          # Path helpers (4KB)
├── types.rs               # Core types (33KB)
├── error.rs               # Error types
├── log.rs                 # Tracing setup
├── shared.rs              # Thread-safe wrappers
└── db_healthcheck.rs      # DB health API
```

---

## FilePicker: The Heart of FFF

### Architecture

```rust
pub struct FilePicker {
    // Shared state handles
    shared_picker: SharedPicker,
    shared_frecency: SharedFrecency,

    // Background components
    background_watcher: Option<BackgroundWatcher>,
    scan_signal: Arc<AtomicBool>,

    // Configuration
    mode: FFFMode,
    base_path: PathBuf,
}
```

### File Index Structure

The file index uses a **dual-segment** design:

```rust
struct FileSync {
    files: Vec<FileItem>,    // All files
    base_count: usize,       // Sorted portion size
    git_workdir: Option<PathBuf>,
}
```

- **Base files** (`files[..base_count]`): Sorted by path, used for binary search and bigram indexing
- **Overflow files** (`files[base_count..]`): Recently added, unsorted
- **Tombstones**: Deleted files marked `is_deleted = true` to preserve bigram indices

### Indexing Flow

```rust
// 1. Background scan spawns
std::thread::spawn(|| {
    // Walk directory tree (ignore .gitignore)
    for entry in WalkDir::new(base_path) {
        let file = FileItem::new(path, &base_path, git_status);
        files.push(file);
    }

    // Sort by path for binary search
    sort_by_path(&mut files);

    // Build bigram index for overlay filtering
    let bigram_index = BigramIndexBuilder::build(&files);

    // Store in shared state
    *shared_picker.write() = Some(FilePicker { ... });
    scan_signal.store(false);  // Signal completion
});
```

### Filesystem Watching

The `BackgroundWatcher` uses `notify-debouncer-full`:

```rust
// Selective watching strategy:
// 1. Watch root non-recursively (catches new dirs)
// 2. Watch each non-ignored dir recursively

fn create_debouncer(...) -> Debouncer {
    let config = Config::default().with_follow_symlinks(false);

    // Avoid watching gitignored dirs (target/, node_modules/)
    let watch_dirs = collect_non_ignored_dirs(&base_path);

    if watch_dirs.len() > MAX_SELECTIVE_WATCH_DIRS {
        // Too many dirs, fall back to root-only
        debouncer.watch(base_path, RecursiveMode::Recursive)?;
    } else {
        debouncer.watch(base_path, RecursiveMode::NonRecursive)?;
        for dir in &watch_dirs {
            debouncer.watch(dir, RecursiveMode::Recursive)?;
        }
    }
}
```

### Event Handling

```rust
fn handle_debounced_events(events, ...) {
    for event in events {
        match event.kind {
            EventKind::Create(_) => {
                // Insert new file in sorted position
                insert_file_sorted(file);
            }
            EventKind::Modify(ModifyKind::Data(_)) => {
                // Invalidate cached content
                file.invalidate_mmap();
                // Update modification frecency
                frecency.track_modification(&path);
            }
            EventKind::Remove(_) => {
                // Mark as tombstone (don't remove from index)
                file.is_deleted = true;
            }
            EventKind::Rename(_) => {
                // Handle as delete + create
            }
        }
    }
}
```

---

## Frecency System

### Database Schema

Uses LMDB with a single database:
- **Key**:Blake3 hash of file path (32 bytes)
- **Value**: `VecDeque<u64>` of Unix timestamps

### Score Calculation

```rust
// Exponential decay with half-life
fn calculate_frecency_score(timestamps: &[u64], now: u64) -> i32 {
    let mut score = 0f64;
    let now_secs = now as f64;

    for &ts in timestamps {
        let age_days = (now_secs - ts as f64) / SECONDS_PER_DAY;
        if age_days > MAX_HISTORY_DAYS {
            continue;  // Too old
        }
        // Exponential decay
        let weight = (-DECAY_CONSTANT * age_days).exp();
        score += weight;
    }

    // Log-scale to prevent extreme scores
    (score.ln_1p() * 100.0) as i32
}
```

### Modification Boost

Files modified recently get a boost:

```rust
const MODIFICATION_THRESHOLDS: [(i64, u64); 5] = [
    (16, 60 * 2),      // 16x if modified < 2 min ago
    (8, 60 * 15),      // 8x if < 15 min
    (4, 60 * 60),      // 4x if < 1 hour
    (2, 60 * 60 * 24), // 2x if < 1 day
    (1, 60 * 60 * 24 * 7), // 1x if < 1 week
];
```

### Background GC

```rust
pub fn spawn_gc(shared: SharedFrecency, db_path: String, ...) {
    std::thread::spawn(move || {
        // Phase 1: Purge stale entries (> 30 days)
        let (deleted, pruned) = tracker.purge_stale_entries();

        // Phase 2: Compact database if needed
        if deleted > 0 || pruned > 0 || file_size > threshold {
            compact_database();
        }
    });
}
```

---

## Query Tracker (Combo Boost)

### Purpose

Tracks which files users open for specific queries, providing "combo boost" for repeated patterns.

### Database Schema

Three databases:
1. **query_file_associations**: `(project_hash :: query_hash) -> QueryMatchEntry`
2. **query_history**: `project_hash -> VecDeque<HistoryEntry>`
3. **grep_query_history**: `project_hash -> VecDeque<HistoryEntry>`

### Combo Boost Logic

```rust
// In score.rs
if query_tracker.is_some() && min_combo_count >= 3 {
    let history = query_tracker.get_matches(query, project_path);
    for (file_path, open_count) in history {
        if open_count >= min_combo_count {
            // Apply 100x multiplier to base score
            score += base_score * combo_boost_score_multiplier;
        }
    }
}
```

---

## Scoring System (score.rs)

### Score Components

```rust
pub struct Score {
    pub total: i32,
    pub base_score: i32,           // Fuzzy match quality
    pub filename_bonus: i32,       // Filename match bonus
    pub special_filename_bonus: i32, // Cargo.toml, etc.
    pub frecency_boost: i32,       // From frecency DB
    pub git_status_boost: i32,     // Modified/untracked bonus
    pub distance_penalty: i32,     // Path depth penalty
    pub current_file_penalty: i32, // Penalty for current file
    pub combo_match_boost: i32,    // Query history bonus
    pub exact_match: bool,
    pub match_type: &'static str,
}
```

### Scoring Flow

```rust
pub fn match_and_score_files(files: &[FileItem], context: &ScoringContext) {
    // 1. Apply constraints (git:modified, *.rs, !test/)
    let working_files = apply_constraints(files, &parsed.constraints);

    // 2. Fuzzy match with neo_frizbee
    let matches = neo_frizbee::match_list_parallel(
        query,
        &haystack,
        &config,
        max_threads
    );

    // 3. Calculate scores for matches
    for (idx, m) in matches {
        let file = &files[idx];
        let mut score = Score::default();

        // Base score from frizbee
        score.base_score = m.score as i32;

        // Filename bonus (exact filename match)
        if file.file_name.contains(query) {
            score.filename_bonus = 50;
        }

        // Special files (Cargo.toml, package.json, etc.)
        if is_special_file(&file.file_name) {
            score.special_filename_bonus = 30;
        }

        // Frecency boost
        score.frecency_boost = file.total_frecency_score;

        // Git status boost (modified = +20, untracked = +10)
        if let Some(status) = file.git_status {
            score.git_status_boost = calculate_git_boost(status);
        }

        // Distance penalty (deeper paths penalized)
        score.distance_penalty = calculate_distance_penalty(&file.relative_path);

        // Combo boost from history
        if let Some(history) = query_tracker.get_matches(query) {
            score.combo_match_boost = calculate_combo_boost(file, history);
        }

        scores.push(score);
    }

    // 4. Sort by total score
    sort_by_score_with_buffer(&mut results, &scores);
}
```

---

## Grep Engine

### Architecture

```
grep.rs (81KB total)
├── grep_search()           # Main entry point
├── search_file()           # Per-file search
├── SinkMatch impl          # Match collection
├── is_definition_line()    # Definition detection
├── is_import_line()        # Import detection
└── classify_match()        # Match classification
```

### Search Modes

```rust
pub enum GrepMode {
    PlainText,   // Literal search (fastest)
    Regex,       // Full regex (grep-searcher)
    Fuzzy,       // Smith-Waterman scoring
}
```

### Search Flow

```rust
pub fn grep_search(files: &[FileItem], options: &GrepSearchOptions) -> GrepResult {
    // 1. Build matcher based on mode
    let matcher = match options.mode {
        GrepMode::PlainText => {
            // Case-insensitive memmem (SIMD optimized)
            CaseInsensitiveMatcher::new(&options.pattern)
        }
        GrepMode::Regex => {
            // grep-regex matcher
            RegexMatcher::new(&options.pattern)
        }
        GrepMode::Fuzzy => {
            // neo_frizbee for fuzzy
            FuzzyMatcher::new(&options.pattern)
        }
    };

    // 2. Build searcher
    let searcher = SearcherBuilder::new()
        .before_context(options.before_context)
        .after_context(options.after_context)
        .build();

    // 3. Search in parallel (rayon)
    let results = files.par_iter()
        .filter_map(|file| {
            // Skip binary/large files
            if file.is_binary || file.size > options.max_file_size {
                return None;
            }

            // Get content (lazy mmap)
            let content = file.get_content()?;

            // Search
            let mut sink = MatchSink::default();
            searcher.search_slice(&matcher, &content, &mut sink).ok()?;

            Some(GrepFileMatch {
                path: file.relative_path.clone(),
                matches: sink.matches,
            })
        })
        .collect();

    // 4. Sort by frecency
    sort_by_frecency(&mut results, frecency);

    GrepResult { files: results, ... }
}
```

### Definition Detection

```rust
pub fn is_definition_line(line: &str) -> bool {
    // Strip modifiers: pub, async, export, default, etc.
    let s = skip_modifiers(line.trim_start().as_bytes());

    // Check for definition keywords
    is_definition_keyword(s)  // struct, fn, enum, class, interface, etc.
}

const DEF_KEYWORDS: &[&[u8]] = &[
    b"struct", b"fn", b"enum", b"trait", b"impl",
    b"class", b"interface", b"function", b"def",
];
```

### Match Classification

```rust
pub enum MatchType {
    Definition,      // struct/func/class definition
    Usage,           // Regular usage
    Import,          // import/use statement
    Comment,         // Match in comment
    String,          // Match in string literal
}
```

---

## Constraint System

### Constraint Types

```rust
pub enum Constraint<'a> {
    Extension(&'a str),    // *.rs, *.{ts,tsx}
    Glob(&'a str),         // **/*.rs, src/**/*.ts
    PathSegment(&'a str),  // src/, tests/
    Filename(&'a str),     // main.rs, schema.json
    GitStatus(GitStatusFilter), // git:modified, git:staged
    Size(SizeConstraint),  // size:>1mb
    Modified(ModifiedConstraint), // modified:<1h
}
```

### Constraint Parsing

Parsed by `fff-query-parser`:

```
Query: "git:modified src/**/*.rs !test/ user controller"

Parsed:
  constraints: [
    GitStatus(Modified),
    Glob("src/**/*.rs"),
    Not(PathSegment("test/")),
  ]
  fuzzy_query: Text("user controller")
```

### Filtering Algorithm

```rust
pub fn apply_constraints<'a>(
    files: &'a [FileItem],
    constraints: &[Constraint],
) -> Option<Vec<&'a FileItem>> {
    // 1. Pre-compute glob matches (expensive, do once)
    let glob_results: Vec<(bool, AHashSet<usize>)> = constraints
        .iter()
        .filter_map(|c| match c {
            Constraint::Glob(pattern) => {
                Some(run_glob(pattern, files))
            }
            _ => None,
        })
        .collect();

    // 2. Filter files
    let mut glob_idx = 0;
    let filtered: Vec<&FileItem> = files
        .iter()
        .enumerate()
        .filter(|(idx, file)| {
            constraints.iter().all(|constraint| {
                let matches = item_matches_constraint_at_index(
                    file, *idx, constraint, &glob_results, &mut glob_idx
                );
                matches  // All constraints must match (AND logic)
            })
        })
        .map(|(_, file)| file)
        .collect();

    if filtered.is_empty() {
        None  // Signal no matches
    } else {
        Some(filtered)
    }
}
```

### Allocation-Free Path Matching

```rust
// Check if path contains segment without allocation
#[inline]
pub fn path_contains_segment(path: &str, segment: &str) -> bool {
    // Check segment/ at start
    if path.starts_with(&format!("{}/", segment)) {
        return true;
    }

    // Scan for /segment/ using byte matching
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

---

## Lua Integration Layer

### Module Structure

```
lua/fff/
├── main.lua (8KB)         # Public API
├── core.lua (4KB)         # Initialization
├── conf.lua (12KB)        # Config management
├── picker_ui.lua (96KB)   # UI rendering
├── file_renderer.lua (8KB) # File formatting
├── list_renderer.lua (12KB) # List management
├── combo_renderer.lua (9KB) # Combo display
├── scrollbar.lua (4KB)    # Scrollbar rendering
├── location_utils.lua (12KB) # Jump handling
├── git_utils.lua (7KB)    # Git highlights
├── treesitter_hl.lua (4KB) # TS highlighting
├── download.lua (10KB)    # Binary download
├── health.lua (11KB)      # Health checks
├── fuzzy.lua (2KB)        # Fuzzy helpers
├── utils.lua (3KB)        # Utilities
└── rust/init.lua (3KB)    # FFI loading
```

### FFI Binding (lua/fff/rust/init.lua)

```lua
-- Find and load the native library
local paths = {
  download.get_binary_cpath_component(),
  base_path .. '../../../target/release/lib?' .. get_lib_extension(),
}

local function try_load_library()
  for _, path_pattern in ipairs(paths) do
    local actual_path = resolve_path(path_pattern:gsub('%?', 'fff_nvim'))
    local stat = vim.uv.fs_stat(actual_path)
    if stat and stat.type == 'file' then
      local loader, err = package.loadlib(actual_path, 'luaopen_fff_nvim')
      if loader then return loader() end
    end
  end
end

local backend = try_load_library()
-- backend exposes: init_db, init_file_picker, fuzzy_search_files, etc.
```

### Key FFI Functions

```lua
-- Database initialization
rust.init_db(frecency_db_path, history_db_path, use_unsafe_no_lock)

-- File picker initialization
rust.init_file_picker(base_path)

-- Search
local results = rust.fuzzy_search_files(
  query,
  max_threads,
  current_file,
  combo_boost_score_multiplier,
  min_combo_count,
  page_index,
  page_size
)

-- Grep
local grep_results = rust.live_grep(
  query,
  file_offset,
  page_size,
  max_file_size,
  max_matches_per_file,
  smart_case,
  grep_mode,
  time_budget_ms
)

-- Tracking
rust.track_access(file_path)
rust.track_query_completion(query, file_path)
```

---

## UI Architecture (picker_ui.lua)

### Component Hierarchy

```
picker_ui.lua (96KB)
│
├── Window Layout
│   ├── Main window (file list)
│   ├── Preview window (file content)
│   ├── Prompt window (input field)
│   └── Border windows (connected borders)
│
├── Rendering
│   ├── file_renderer.lua (file formatting)
│   ├── list_renderer.lua (list management)
│   ├── combo_renderer.lua (combo display)
│   └── scrollbar.lua (pagination)
│
├── Input Handling
│   ├── Keymaps (close, select, move, etc.)
│   ├── Query input (with debouncing)
│   └── Mode cycling (grep modes)
│
└── Actions
    ├── File opening (e, split, vsplit, tab)
    ├── Quickfix integration
    └── Multi-select
```

### Layout Calculation

```lua
local function compute_layout(config)
  local terminal_width = vim.o.columns
  local terminal_height = vim.o.lines

  -- Calculate main window dimensions
  local width = math.floor(terminal_width * config.layout.width)
  local height = math.floor(terminal_height * config.layout.height)

  -- Handle flex layout (responsive)
  local preview_position = config.layout.preview_position
  if config.layout.flex and terminal_width < config.layout.flex.size then
    preview_position = config.layout.flex.wrap  -- Switch to top/bottom
  end

  -- Calculate preview window size
  local preview_size = math.floor(
    (preview_position == 'left' or preview_position == 'right')
      and width * config.layout.preview_size
      or height * config.layout.preview_size
  )

  return {
    main = { row, col, width, height },
    preview = { row, col, preview_size },
    prompt = { row, col, width },
  }
end
```

### Rendering Pipeline

```lua
function M.render(state, results)
  -- 1. Clear previous highlights
  clear_highlights(state.main_win)

  -- 2. Render file list
  local lines = {}
  for i, item in ipairs(results.items) do
    local line, highlights = file_renderer.format_file(
      item,
      results.scores[i],
      state.config
    )
    table.insert(lines, line)
    apply_highlights(state.main_win, i - 1, highlights)
  end

  -- 3. Update buffer
  vim.api.nvim_buf_set_lines(state.main_buf, 0, -1, false, lines)

  -- 4. Render scrollbar
  if state.config.layout.show_scrollbar then
    scrollbar.render(state, results.total_matched, state.page_index)
  end

  -- 5. Update preview
  if state.selected_index and results.items[state.selected_index] then
    preview.update(state, results.items[state.selected_index])
  end
end
```

### Input Debouncing

```lua
-- Debounced search (150ms)
local function schedule_search(state)
  if state.search_timer then
    state.search_timer:stop()
  end

  state.search_timer = vim.uv.new_timer()
  state.search_timer:start(
    150, 0,
    vim.schedule_wrap(function()
      perform_search(state)
    end)
  )
end
```

---

## Build System

### Makefile Targets

```makefile
# Build release binary
make build  # cargo build --release --features zlob

# Run tests
make test   # Rust + Lua tests

# Generate C header
make header  # cbindgen

# Format code
make format  # cargo fmt + stylua + biome

# Lint code
make lint    # cargo clippy + luacheck

# Publish crates
make publish-crates V=0.5.1
```

### CI/CD (release.yaml)

Builds pre-built binaries for:
- **Linux**: x86_64/aarch64 (glibc 2.17, musl)
- **macOS**: x86_64/aarch64 (macOS 13+)
- **Windows**: x86_64/aarch64 (MSVC)
- **Android**: aarch64 (Termux)

Publishes:
1. GitHub Release with binaries
2. crates.io (fff-grep, fff-query-parser, fff-search)
3. npm (@ff-labs/fff-bun, @ff-labs/fff-node, platform packages)

---

## Performance Optimizations

### 1. Memory Mapping

Files are memory-mapped for zero-copy access:

```rust
#[cfg(not(target_os = "windows"))]
pub enum FileContent {
    Mmap(memmap2::Mmap),  // Zero-copy on Unix
    Buffer(Vec<u8>),      // Heap buffer on Windows
}
```

### 2. Lazy Content Loading

```rust
pub struct FileItem {
    content: OnceLock<FileContent>,  // Initialized on first access
}

pub fn get_content(&self, budget: &ContentCacheBudget) -> Option<FileContentRef> {
    // Check persistent cache first
    if let Some(cached) = self.content.get() {
        return Some(FileContentRef::Cached(cached));
    }

    // Check budget
    if budget.is_over() {
        // Temporary mmap (released after use)
        return Some(FileContentRef::Temp(create_temp_mmap()));
    }

    // Load into persistent cache
    let content = create_mmap(&self.path);
    self.content.set(content).ok()?;
    budget.increment();
}
```

### 3. Parallel Processing

```rust
// Rayon parallel iterators everywhere
files.par_iter()
    .filter_map(|file| search_file(file))
    .collect()
```

### 4. Bigram Overlay

For large result sets, uses bigram filtering:

```rust
// Extract bigrams from query
let bigrams = extract_bigrams("main.rs");  // ["ma", "ai", "in", "n.", ".r", "rs"]

// Filter candidates using bigram index
let candidates = bigram_index.get_candidates(&bigrams);
```

### 5. Sort Buffer

Efficient sorting with pre-allocated buffers:

```rust
pub fn sort_by_score_with_buffer(items: &mut [FileItem], scores: &[Score]) {
    // Pre-allocate sort buffer
    let mut buffer = Vec::with_capacity(items.len());

    // Glidesort (stable, fast)
    glidesort::sort_by_key(items, scores, &mut buffer);
}
```

---

## Testing

### Rust Tests

```bash
# Run all Rust tests
cargo test --workspace --features zlob

# Run specific crate tests
cargo test -p fff-core

# Run benchmarks
cargo bench -p fff-core
```

### Lua Tests

```bash
# Run Lua tests (requires plenary.nvim)
make test-lua
```

### Test Files

```
crates/fff-core/tests/
├── bigram_overlay_integration.rs
└── grep_integration.rs

tests/
├── fff_core_spec.lua
└── version_spec.lua
```

---

## Debugging

### Logging

```lua
-- Enable logging in config
logging = {
  enabled = true,
  log_file = vim.fn.stdpath('log') .. '/fff.log',
  log_level = 'debug',  -- trace, debug, info, warn, error
}

-- Open log file
:FFFOpenLog
```

### Debug Mode

```lua
-- Show scores in UI
debug = {
  enabled = true,
  show_scores = true,
}

-- Toggle with F2 in picker
-- Or use :FFFDebug toggle
```

### Health Check

```vim
:FFFHealth
```

Output:
```
fff.nvim Report
  Version: 0.5.1
  File Picker
    Initialized: true
    Base Path: /path/to/project
    Indexed Files: 12345
  Frecency DB
    Initialized: true
    Entries: 5678
  Git
    Available: true
    libgit2: 1.8.0
```

---

## Next Steps

- **[rust-revision.md](./rust-revision.md)** - Deep dive into Rust patterns
- **[production-grade.md](./production-grade.md)** - Production deployment guide
