# Zero to FFF.nvim Engineer

**Location:** `/home/darkvoid/Boxxed/@formulas/src.rust/src.llamacpp/src.ClaudOpen/fff.nvim`

## Introduction

**fff.nvim** (Freakin Fast File Finder) is a high-performance file search tool for Neovim and AI agents (via MCP - Model Context Protocol). It's designed to be significantly faster than alternatives like ripgrep and fzf, with built-in memory (frecency) for intelligent result ranking.

### What Makes FFF Special?

1. **Performance**: 10-50x faster than ripgrep for content search, 10x faster than fzf for file search
2. **Frecency Ranking**: Tracks file access patterns to boost frequently/recently used files
3. **Typo-Resistant**: Fuzzy matching powered by `neo_frizbee` handles typos gracefully
4. **Git-Aware**: Built-in git status detection with visual indicators
5. **Dual-Purpose**: Works as both a Neovim plugin AND an MCP server for AI agents
6. **Memory-Efficient**: Uses memory mapping (mmap) and efficient data structures

---

## Quick Start

### Installation (Neovim)

#### Using lazy.nvim

```lua
{
  'dmtrKovalenko/fff.nvim',
  build = function()
    require("fff.download").download_or_build_binary()
  end,
  opts = {
    debug = {
      enabled = true,
      show_scores = true,
    },
  },
  lazy = false,  -- Auto-lazy initializes
  keys = {
    { "ff", function() require('fff').find_files() end, desc = 'FFFind files' },
    { "fg", function() require('fff').live_grep() end, desc = 'LiFFFe grep' },
  }
}
```

#### Build from Source

```bash
cd fff.nvim
cargo build --release --features zlob
```

### Installation (MCP for AI Agents)

```bash
curl -L https://dmtrkovalenko.dev/install-fff-mcp.sh | bash
```

Then add to your AI config:
```
For any file search or grep in the current git indexed directory use fff tools
```

---

## Core Concepts

### 1. FFFMode: Neovim vs AI

The plugin operates in two modes, configured via `FFFMode`:

```rust
pub enum FFFMode {
    Neovim,  // Human user - slower decay, more history
    Ai,      // AI agent - faster decay, burst-aware
}
```

- **Neovim Mode**: Tracks user patterns over weeks, optimized for interactive use
- **AI Mode**: Compressed time windows (seconds/minutes), optimized for rapid agent sessions

### 2. The File Index

FFF maintains an in-memory index of all files in your project:

```rust
pub struct FileItem {
    pub path: PathBuf,              // Absolute path
    pub relative_path: String,      // Relative to base_path
    pub file_name: String,          // Just the filename
    pub size: u64,                  // File size in bytes
    pub modified: u64,              // Unix timestamp
    pub access_frecency_score: i32, // Access-based ranking
    pub modification_frecency_score: i32, // Modification-based ranking
    pub total_frecency_score: i32,  // Combined score
    pub git_status: Option<git2::Status>, // Git state
    pub is_binary: bool,            // Binary file flag
    pub is_deleted: bool,           // Tombstone for deleted files
    content: OnceLock<FileContent>, // Lazy-loaded content
}
```

### 3. Frecency (Frequency + Recency)

Files are ranked using a decay-based scoring system:

```rust
// Human mode: 10-day half-life
const DECAY_CONSTANT: f64 = 0.0693; // ln(2)/10
const MAX_HISTORY_DAYS: f64 = 30.0;

// AI mode: 3-day half-life (faster decay)
const AI_DECAY_CONSTANT: f64 = 0.231; // ln(2)/3
const AI_MAX_HISTORY_DAYS: f64 = 7.0;
```

The frecency database uses **LMDB** (Lightning Memory-Mapped Database) for persistence.

---

## Basic Usage

### File Search

```lua
-- Open file picker
:FFFFind
-- or
<leader>ff

-- Search with query
:FFFFind some_file.rs

-- Search in specific directory
:FFFFind /path/to/dir

-- Programmatically
require('fff').find_files()
require('fff').find_files_in_dir("/path")
```

### Live Grep

```lua
-- Basic grep
:lua require('fff').live_grep()

-- With initial query
:lua require('fff').live_grep({ query = "search term" })

-- With fuzzy mode
:lua require('fff').live_grep({
  grep = { modes = { 'fuzzy', 'plain' } }
})
```

### Constraints

Combine constraints in your query:

```
# Find modified Rust files in src/
git:modified src/ *.rs

# Exclude tests
!test/ !*.spec.ts

# Complex glob
./**/*.{rs,lua}

# Combined
git:modified src/**/*.rs !src/**/mod.rs user controller
```

---

## Architecture Overview

```
+----------------------------------------------------------+
|                    Neovim / MCP Client                    |
+----------------------------+-----------------------------+
                             |
                    Lua FFI Layer (mlua)
                             |
+----------------------------v-----------------------------+
|                    fff-nvim (Rust)                       |
|  - init_db, init_file_picker                             |
|  - fuzzy_search_files, live_grep                         |
|  - track_access, track_query_completion                  |
+----------------------------+-----------------------------+
                             |
+----------------------------v-----------------------------+
|                   fff-core (Search Engine)               |
|  +----------------+  +----------------+  +-------------+ |
|  |  FilePicker    |  |  Frecency      |  |  GitStatus  | |
|  |  - Indexing    |  |  Tracker       |  |  Cache      | |
|  |  - Watching    |  |  - LMDB        |  |             | |
|  |  - Searching   |  +----------------+  +-------------+ |
|  +----------------+                                      |
|                                                          |
|  +----------------+  +----------------+                  |
|  |  Grep Engine   |  |  Query         |                  |
|  |  - Plain/Regex |  |  Tracker       |                  |
|  |  - Fuzzy       |  |  - History     |                  |
|  +----------------+  +----------------+                  |
+----------------------------------------------------------+
                             |
+----------------------------v-----------------------------+
|              Supporting Crates                           |
|  - fff-query-parser  (Query syntax parsing)              |
|  - fff-grep          (SIMD grep primitives)              |
|  - fff-c             (C FFI for other languages)         |
+----------------------------------------------------------+
```

---

## Key Components

### 1. FilePicker (fff-core/src/file_picker.rs)

The central orchestrator:
- Spawns background indexing thread
- Maintains sorted file list
- Handles filesystem watching via `notify`
- Provides fuzzy search and grep APIs

```rust
// Key operations
FilePicker::new_with_shared_state(...)  // Initialize
picker.trigger_rescan(&FRECENCY)        // Force rescan
picker.grep(&parsed, &options)          // Grep search
FilePicker::fuzzy_search(...)           // Fuzzy search
```

### 2. FrecencyTracker (fff-core/src/frecency.rs)

LMDB-backed access tracking:

```rust
// Track file access
frecency.track_access(&file_path)?;

// Score calculation
let score = calculate_frecency_score(timestamps, now);
```

### 3. QueryTracker (fff-core/src/query_tracker.rs)

Tracks query history for "combo boost":

```rust
// Records: (project, query) -> [(file, count, timestamp)]
tracker.track_query_completion(query, project_path, file_path)?;

// Retrieve history
let prev_query = tracker.get_historical_query(project, offset)?;
```

### 4. BackgroundWatcher (fff-core/src/background_watcher.rs)

Real-time filesystem monitoring:

```rust
// Watches non-ignored directories
// Debounces events (250ms)
// Handles: Create, Modify, Delete, Rename
```

### 5. Grep Engine (fff-core/src/grep.rs)

Multi-mode content search:
- **Plain**: Literal text search (fastest)
- **Regex**: Full regex support
- **Fuzzy**: Smith-Waterman scoring

```rust
// SIMD-optimized line matching
// Aho-Corasick for multi-pattern
// Memory-mapped file access
```

---

## Data Flow

### Initialization Flow

```
1. plugin/fff.lua fires on UIEnter
2. lua/fff/core.lua:ensure_initialized()
3. lua/fff/rust/init.lua loads native library
4. Rust: init_db() - Opens LMDB databases
5. Rust: init_file_picker() - Spawns background indexer
6. Background thread scans directory tree
7. File list populated, watcher started
```

### Search Flow

```
1. User types query in picker
2. Lua calls fuzzy_search_files(query, ...)
3. QueryParser parses constraints + fuzzy parts
4. Constraints filter candidate files
5. neo_frizbee matches fuzzy parts
6. score.rs calculates final scores:
   - Base fuzzy match score
   - Frecency boost
   - Git status boost
   - Combo boost (history)
   - Distance penalty
7. Results sorted and returned
```

### File Access Tracking Flow

```
1. User opens file in Neovim
2. BufEnter autocmd fires
3. track_access(real_path) called
4. LMDB: Append timestamp to file's entry
5. Update file's frecency score in index
6. Background GC periodically purges stale entries
```

---

## Configuration Reference

```lua
require('fff').setup({
    -- Base directory
    base_path = vim.fn.getcwd(),

    -- UI appearance
    prompt = '🪿 ',
    title = 'FFFiles',
    max_results = 100,

    -- Performance
    max_threads = 4,
    lazy_sync = true,  -- Defer indexing until picker opened

    -- Layout
    layout = {
      height = 0.8,
      width = 0.8,
      prompt_position = 'bottom',
      preview_position = 'right',
      preview_size = 0.5,
      path_shorten_strategy = 'middle_number',
    },

    -- Preview settings
    preview = {
      enabled = true,
      max_size = 10 * 1024 * 1024,  -- 10MB
      chunk_size = 8192,
      line_numbers = false,
      wrap_lines = false,
    },

    -- Frecency (file access memory)
    frecency = {
      enabled = true,
      db_path = vim.fn.stdpath('cache') .. '/fff_nvim',
    },

    -- Query history
    history = {
      enabled = true,
      db_path = vim.fn.stdpath('data') .. '/fff_queries',
      min_combo_count = 3,
      combo_boost_score_multiplier = 100,
    },

    -- Git integration
    git = {
      status_text_color = false,  -- Color filenames by git status
    },

    -- Grep settings
    grep = {
      max_file_size = 10 * 1024 * 1024,
      max_matches_per_file = 100,
      smart_case = true,
      time_budget_ms = 150,
      modes = { 'plain', 'regex', 'fuzzy' },
    },

    -- Debug
    debug = {
      enabled = false,
      show_scores = false,
    },

    -- Logging
    logging = {
      enabled = true,
      log_file = vim.fn.stdpath('log') .. '/fff.log',
      log_level = 'info',
    },
})
```

---

## Keymaps

```lua
keymaps = {
  close = '<Esc>',
  select = '<CR>',
  select_split = '<C-s>',
  select_vsplit = '<C-v>',
  select_tab = '<C-t>',
  move_up = { '<Up>', '<C-p>' },
  move_down = { '<Down>', '<C-n>' },
  preview_scroll_up = '<C-u>',
  preview_scroll_down = '<C-d>',
  toggle_debug = '<F2>',
  cycle_grep_modes = '<S-Tab>',
  cycle_previous_query = '<C-Up>',
  toggle_select = '<Tab>',       -- Multi-select
  send_to_quickfix = '<C-q>',    -- Send to quickfix
  focus_list = '<leader>l',
  focus_preview = '<leader>p',
}
```

---

## Commands

| Command | Description |
|---------|-------------|
| `:FFFFind [path/query]` | Open file picker |
| `:FFFScan` | Force rescan files |
| `:FFFRefreshGit` | Refresh git status |
| `:FFFClearCache [all\|frecency\|files]` | Clear caches |
| `:FFFHealth` | Health check |
| `:FFFDebug [on\|off\|toggle]` | Toggle debug scores |
| `:FFFOpenLog` | Open log file |

---

## Next Steps

After mastering the basics:

1. **[01-fff-exploration.md](./01-fff-exploration.md)** - Deep dive into architecture
2. **[rust-revision.md](./rust-revision.md)** - Rust patterns and implementation details
3. **[production-grade.md](./production-grade.md)** - Production deployment guide

---

## Troubleshooting

### Common Issues

**Binary not found:**
```lua
:lua require("fff.download").download_or_build_binary()
```

**Slow initial scan:**
- Set `lazy_sync = true` to defer indexing
- Use `.ignore` to exclude large directories

**Frecency not working:**
- Check `frecency.enabled = true`
- Verify database path is writable

**Git status not showing:**
- Ensure you're in a git repository
- Check `git2` dependencies are installed

### Health Check

```vim
:FFFHealth
```

This checks:
- File picker initialization
- Git/libgit2 availability
- Database connectivity
- Optional dependencies (image preview tools)
