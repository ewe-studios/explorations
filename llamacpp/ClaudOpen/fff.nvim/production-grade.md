# FFF.nvim Production Guide

**Repository:** `/home/darkvoid/Boxxed/@formulas/src.rust/src.llamacpp/src.ClaudOpen/fff.nvim`

This guide covers production deployment, optimization, troubleshooting, and best practices for fff.nvim.

---

## Table of Contents

1. [Installation](#installation)
2. [Configuration](#configuration)
3. [Performance Tuning](#performance-tuning)
4. [MCP Server Deployment](#mcp-server-deployment)
5. [Monitoring and Debugging](#monitoring-and-debugging)
6. [Troubleshooting](#troubleshooting)
7. [Best Practices](#best-practices)

---

## Installation

### Prerequisites

- **Neovim**: 0.10.0 or higher
- **Rust**: 1.85+ (for building from source)
- **git**: For git integration features

### Pre-built Binary Installation (Recommended)

#### Using lazy.nvim

```lua
{
  'dmtrKovalenko/fff.nvim',
  build = function()
    require("fff.download").download_or_build_binary()
  end,
  opts = {
    debug = {
      enabled = false,
      show_scores = false,
    },
  },
  lazy = false,
  keys = {
    { "ff", function() require('fff').find_files() end, desc = 'Find files' },
    { "fg", function() require('fff').live_grep() end, desc = 'Live grep' },
  }
}
```

The `download_or_build_binary()` function:
1. Checks for pre-built binary matching your platform
2. Downloads from GitHub releases if available
3. Falls back to `cargo build --release` if needed

#### Manual Binary Installation

```bash
# Download pre-built binary
wget https://github.com/dmtrKovalenko/fff.nvim/releases/latest/download/x86_64-unknown-linux-gnu.so

# Place in plugin directory
mkdir -p ~/.local/share/nvim/lazy/fff.nvim/build
mv x86_64-unknown-linux-gnu.so ~/.local/share/nvim/lazy/fff.nvim/build/fff_nvim.so
```

### Building from Source

```bash
cd ~/.local/share/nvim/lazy/fff.nvim

# Full build with all features
cargo build --release --features zlob

# Binary location
# Linux: target/release/libfff_nvim.so
# macOS: target/release/libfff_nvim.dylib
# Windows: target/release/fff_nvim.dll
```

### Platform-Specific Installation

#### Linux (glibc 2.17+)

```bash
# Download
wget https://github.com/dmtrKovalenko/fff.nvim/releases/latest/download/x86_64-unknown-linux-gnu.2.17.so
mv x86_64-unknown-linux-gnu.2.17.so ~/.local/share/nvim/lazy/fff.nvim/build/fff_nvim.so
```

#### Linux (musl/static)

```bash
# For Alpine Linux or static builds
wget https://github.com/dmtrKovalenko/fff.nvim/releases/latest/download/x86_64-unknown-linux-musl.so
```

#### macOS (Apple Silicon)

```bash
wget https://github.com/dmtrKovalenko/fff.nvim/releases/latest/download/aarch64-apple-darwin.dylib
codesign --force --sign - aarch64-apple-darwin.dylib  # Ad-hoc sign
```

#### macOS (Intel)

```bash
wget https://github.com/dmtrKovalenko/fff.nvim/releases/latest/download/x86_64-apple-darwin.dylib
```

#### Windows (MSVC)

```powershell
# Download DLL
Invoke-WebRequest -Uri "https://github.com/dmtrKovalenko/fff.nvim/releases/latest/download/x86_64-pc-windows-msvc.dll" -OutFile "fff_nvim.dll"

# Place in plugin build directory
```

#### Android (Termux)

```bash
# Install dependencies
pkg install rust libgit2

# Build from source
cargo build --release --target aarch64-linux-android --features zlob
```

---

## Configuration

### Minimal Configuration

```lua
require('fff').setup({
  frecency = {
    enabled = true,
    db_path = vim.fn.stdpath('cache') .. '/fff_nvim',
  },
  history = {
    enabled = true,
    db_path = vim.fn.stdpath('data') .. '/fff_queries',
  },
  logging = {
    enabled = true,
    log_file = vim.fn.stdpath('log') .. '/fff.log',
    log_level = 'warn',  -- Only log warnings and errors
  },
})
```

### Production Configuration

```lua
require('fff').setup({
  -- Performance
  max_results = 100,       -- Limit results
  max_threads = 4,         -- Cap thread count
  lazy_sync = true,        -- Defer indexing until picker opened

  -- Layout
  layout = {
    height = 0.8,
    width = 0.8,
    prompt_position = 'bottom',
    preview_position = 'right',
    preview_size = 0.5,
    show_scrollbar = true,
    path_shorten_strategy = 'middle_number',
  },

  -- Preview (disable for large files)
  preview = {
    enabled = true,
    max_size = 10 * 1024 * 1024,  -- 10MB limit
    chunk_size = 8192,
    binary_file_threshold = 1024,
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
    status_text_color = false,  -- Only sign column
  },

  -- Grep settings
  grep = {
    max_file_size = 10 * 1024 * 1024,  -- 10MB limit
    max_matches_per_file = 100,
    smart_case = true,
    time_budget_ms = 150,  -- Prevent UI freeze
    modes = { 'plain', 'regex', 'fuzzy' },
  },

  -- Debug (disable in production)
  debug = {
    enabled = false,
    show_scores = false,
  },

  -- Logging
  logging = {
    enabled = true,
    log_file = vim.fn.stdpath('log') .. '/fff.log',
    log_level = 'warn',
  },

  -- File picker
  file_picker = {
    current_file_label = '(current)',
  },
})
```

### Enterprise Configuration (Large Repos)

```lua
require('fff').setup({
  -- Reduce initial scan time
  lazy_sync = true,  -- Critical for large repos

  -- Limit resource usage
  max_threads = math.min(vim.uv.available_parallelism(), 4),
  max_results = 50,

  -- Aggressive filtering
  grep = {
    max_file_size = 5 * 1024 * 1024,  -- 5MB
    max_matches_per_file = 50,
    time_budget_ms = 100,
  },

  -- Exclude heavy directories
  -- Create .ignore in project root:
  -- target/
  -- node_modules/
  -- .git/
  -- vendor/
  -- build/
})
```

---

## Performance Tuning

### Initial Scan Optimization

For large repositories (100k+ files):

1. **Enable lazy_sync**: Defer indexing until picker is opened
2. **Use .ignore file**: Exclude gitignored directories
3. **Reduce max_threads**: Prevent CPU saturation

```lua
-- .ignore in project root
# Build artifacts
target/
build/
dist/
out/

# Dependencies
node_modules/
vendor/
.venv/
Cargo.lock

# Generated files
**/*.min.js
**/*.gen.go
**/*_generated.rs
```

### Grep Performance

```lua
grep = {
  -- Skip large files
  max_file_size = 10 * 1024 * 1024,

  -- Limit matches per file
  max_matches_per_file = 100,

  -- Time budget prevents UI freeze
  time_budget_ms = 150,

  -- Use plain text when possible (faster than regex)
  modes = { 'plain', 'regex' },  -- Disable fuzzy if not needed
}
```

### Memory Usage

FFF uses memory mapping for file content:

- **Default cache budget**: 30,000 files or 1GB
- **Per-file mmap**: Released after use if over budget
- **Frecency DB**: 24MB LMDB map
- **Query history DB**: 10MB LMDB map

To reduce memory:

```lua
-- In rust code (requires rebuild)
// crates/fff-nvim/src/lib.rs
impl Default for ContentCacheBudget {
    fn default() -> Self {
        Self {
            max_files: 10_000,  // Reduced from 30,000
            max_bytes: 512 * 1024 * 1024,  // 512MB
            ...
        }
    }
}
```

### Frecency Optimization

For AI agent usage (MCP mode):

```rust
// Faster decay for AI sessions
const AI_DECAY_CONSTANT: f64 = 0.231;  // 3-day half-life
const AI_MAX_HISTORY_DAYS: f64 = 7.0;
const AI_MODE_COOLDOWN_SECS: u64 = 5 * 60;  // 5 min between tracks
```

For human usage (Neovim):

```rust
// Slower decay for human patterns
const DECAY_CONSTANT: f64 = 0.0693;  // 10-day half-life
const MAX_HISTORY_DAYS: f64 = 30.0;
```

---

## MCP Server Deployment

### Installation

```bash
curl -L https://dmtrkovalenko.dev/install-fff-mcp.sh | bash
```

This installs:
- `fff-mcp` binary to `~/.local/bin/`
- Configuration for Claude Code, Codex, OpenCode

### Manual Installation

```bash
# Download pre-built binary
wget https://github.com/dmtrKovalenko/fff.nvim/releases/latest/download/fff-mcp-x86_64-unknown-linux-gnu

# Make executable
chmod +x fff-mcp-x86_64-unknown-linux-gnu
mv fff-mcp-x86_64-unknown-linux-gnu ~/.local/bin/fff-mcp

# Verify
fff-mcp --version
```

### Configuration

#### Claude Code

Add to `~/.claude/settings.json`:

```json
{
  "mcpServers": {
    "fff": {
      "command": "fff-mcp",
      "args": ["--log-file", "/tmp/fff-mcp.log", "--log-level", "info"]
    }
  }
}
```

#### Cursor

Add to `~/.cursor/mcp.json`:

```json
{
  "mcpServers": {
    "fff": {
      "command": "fff-mcp",
      "cwd": "${workspaceFolder}"
    }
  }
}
```

### MCP Arguments

```bash
fff-mcp --help

Options:
  [PATH]                    Base directory to index
  --frecency-db <PATH>      Frecency database path
  --history-db <PATH>       Query history database path
  --log-file <PATH>         Log file path
  --log-level <LEVEL>       Log level (trace, debug, info, warn, error)
  --no-update-check         Disable update checks
  --no-warmup               Disable eager mmap warmup
  --max-cached-files <N>    Max files in persistent cache
  --healthcheck             Run health check and exit
```

### MCP Best Practices

1. **Set appropriate base path**: Use git root for best results
2. **Enable logging**: Debug issues with `--log-level debug`
3. **Configure AI instructions**: Add to CLAUDE.md:

```markdown
# CLAUDE.md
For any file search or grep in the current git indexed directory use fff tools:
- grep: Search file CONTENTS for definitions, usage, patterns
- find_files: Find files by name when exploring
- multi_grep: Search for multiple patterns at once (OR logic)

Rules:
1. Search BARE IDENTIFIERS only (e.g., 'ActorAuth' not 'struct ActorAuth')
2. Stop after 2 greps - READ the code
3. Use multi_grep for multiple name variants
```

---

## Monitoring and Debugging

### Log Analysis

```lua
-- Open log file
:FFFOpenLog

-- Check for errors
grep ERROR ~/.local/state/nvim/log/fff.log
```

### Health Check

```vim
:FFFHealth
```

Expected output:
```
fff.nvim Report
  Version: 0.5.1
  File Picker
    Initialized: true
    Base Path: /path/to/project
    Indexed Files: 12345
    Is Scanning: false
  Frecency DB
    Initialized: true
    DB Path: /path/to/cache
    Entries: 5678
  Query Tracker
    Initialized: true
    DB Path: /path/to/data
    Entries: 234
  Git
    Available: true
    libgit2: 1.8.0
```

### Debug Mode

```lua
-- Enable in config
debug = {
  enabled = true,
  show_scores = true,
}

-- Or toggle with command
:FFFDebug toggle

-- Or keybind in picker (F2)
```

Shows:
- Base fuzzy match score
- Frecency boost
- Git status boost
- Combo boost
- Distance penalty
- Final total score

### Profiling

```bash
# Build with profiling symbols
cargo build --release --features zlob

# Run grep profiler
./target/release/grep_profiler

# Run search profiler
./target/release/search_profiler
```

### Performance Benchmarks

```bash
# Run benchmarks
cargo bench -p fff-core

# Benchmarks included:
# - Bigram overlay
# - memmem search
# - Query parsing
```

---

## Troubleshooting

### Common Issues

#### Binary Not Found

```
Failed to load fff rust backend.
Searched paths: [...]
```

**Solution:**
```lua
:lua require("fff.download").download_or_build_binary()
```

Or build manually:
```bash
cd ~/.local/share/nvim/lazy/fff.nvim
cargo build --release --features zlob
```

#### Slow Initial Scan

**Symptoms**: Picker takes >5 seconds to open

**Causes:**
1. Large repository (100k+ files)
2. No .ignore file
3. lazy_sync = false

**Solutions:**
```lua
-- Enable lazy indexing
lazy_sync = true

-- Create .ignore file
echo "target/" >> .ignore
echo "node_modules/" >> .ignore
```

#### Frecency Not Working

**Symptoms**: Recently opened files not boosted

**Checks:**
```lua
-- Verify frecency is enabled
debug = { show_scores = true }  -- Check scores in UI

-- Check database
:lua print(vim.fn.stdpath('cache') .. '/fff_nvim')
-- Verify directory exists and is writable
```

**Solution:**
```lua
frecency = {
  enabled = true,
  db_path = vim.fn.stdpath('cache') .. '/fff_nvim',
}
```

#### Git Status Not Showing

**Symptoms**: No git indicators in sign column

**Checks:**
```vim
:FFFHealth  -- Verify git2 is available
```

**Solutions:**
1. Ensure you're in a git repository
2. Install libgit2 dependencies
3. Rebuild with git2 feature

#### Preview Not Working

**Symptoms**: Empty preview window

**Checks:**
```lua
-- Verify preview is enabled
preview = { enabled = true }

-- Check file size limit
preview = { max_size = 10 * 1024 * 1024 }
```

#### High Memory Usage

**Symptoms**: Neovim uses >500MB RAM

**Solutions:**
1. Reduce max_cached_files (requires rebuild)
2. Lower grep.max_file_size
3. Clear frecency cache: `:FFFClearCache frecency`

### Database Corruption

If LMDB databases become corrupted:

```vim
:FFFClearCache all
:FFFScan  -- Force rescan
```

Or manually:
```bash
rm -rf ~/.local/state/nvim/fff_nvim
rm -rf ~/.local/share/nvim/fff_queries
```

### Windows-Specific Issues

#### DLL Loading Failed

**Error**: `The specified module could not be found`

**Solution:**
1. Install Visual C++ Redistributable
2. Ensure DLL is in PATH or plugin directory

#### File Locking

**Issue**: Files can't be saved while in cache

**Cause**: Windows holds file handles open for mmap

**Solution:** Uses heap buffers instead of mmap on Windows (automatic)

### macOS-Specific Issues

#### Code Signing

**Error**: `Library not loaded: code signature invalid`

**Solution:**
```bash
codesign --force --sign - ~/.local/share/nvim/lazy/fff.nvim/build/fff_nvim.dylib
```

#### Gatekeeper

**Error**: Binary can't be opened

**Solution:**
```bash
xattr -cr ~/.local/share/nvim/lazy/fff.nvim/build/fff_nvim.dylib
```

---

## Best Practices

### Query Usage

**Effective queries:**
```
# File search
main.rs           # Exact filename
lib.rs            # Partial match
src/main          # Path + filename
*.toml            # Extension filter

# Grep
ActorAuth         # Bare identifier
prepare_upload    # Function name
TODO              # Comment marker
```

**Ineffective queries:**
```
struct ActorAuth    # Too specific
ctx.data::<T>       # Code syntax
load.*metadata      # Complex regex
```

### Constraint Usage

```
# Good constraints
git:modified        # Only modified files
*.rs                # Rust files only
src/                # In src directory
!test/              # Exclude tests

# Combined
git:modified src/**/*.rs !src/**/mod.rs
```

### Keybind Strategy

```lua
keys = {
  -- Core functionality
  { "ff", function() require('fff').find_files() end, desc = 'Find files' },
  { "fg", function() require('fff').live_grep() end, desc = 'Live grep' },

  -- Advanced grep modes
  { "fz", function() require('fff').live_grep({
      grep = { modes = { 'fuzzy', 'plain' } }
    }) end, desc = 'Fuzzy grep' },
  { "fc", function() require('fff').live_grep({
      query = vim.fn.expand("<cword>")
    }) end, desc = 'Search word under cursor' },

  -- Quick actions
  { "<leader>ff", function() require('fff').find_files() end, desc = 'Find files' },
  { "<leader>fg", function() require('fff').live_grep() end, desc = 'Live grep' },
}
```

### Multi-Select Workflow

```lua
-- In picker:
-- 1. Navigate to files
-- 2. Press <Tab> to toggle selection (shows border ▊)
-- 3. Press <C-q> to send to quickfix
-- 4. Navigate quickfix with :cnext, :cprev

-- Useful for:
-- - Opening related files
-- - Reviewing changes
-- - Batch operations
```

### History Navigation

```lua
-- In picker:
-- Press <C-Up> to cycle through previous queries

-- Query history is per-project
-- Stored in ~/.local/share/nvim/fff_queries
```

### Integration with Other Tools

#### With Telescope

```lua
-- Use fff for file search, Telescope for fuzzy finding
keys = {
  { "ff", function() require('fff').find_files() end, desc = 'FFF files' },
  { "<leader>ft", function() require('telescope.builtin').find_files() end, desc = 'Telescope files' },
}
```

#### With LSP

```lua
-- Use fff for file search, LSP for symbol search
keys = {
  { "ff", function() require('fff').find_files() end, desc = 'Find files' },
  { "<leader>fs", function() vim.lsp.buf.workspace_symbol() end, desc = 'Workspace symbols' },
}
```

#### With Neo-tree

```lua
-- Use Neo-tree for exploration, fff for search
require('neo-tree').setup({
  source_selector = {
    sources = {
      { source = 'filesystem', display_name = ' Files ' },
      { source = 'buffers', display_name = ' Buffers ' },
    },
  },
})

keys = {
  { "<leader>e", function() require('neo-tree.command').execute() end, desc = 'Neo-tree' },
  { "<leader>ff", function() require('fff').find_files() end, desc = 'FFF find' },
}
```

### Cache Management

```vim
" Clear all caches
:FFFClearCache all

" Clear frecency only
:FFFClearCache frecency

" Clear file cache only
:FFFClearCache files

" Force rescan
:FFFScan
```

### Update Strategy

```lua
-- Check for updates (automatic on startup)
-- Disable if needed:
-- Add to MCP: --no-update-check

-- Manual update with lazy.nvim
:Lazy sync fff.nvim

-- Rebuild after update
:Lazy build fff.nvim
```

---

## Security Considerations

### Binary Verification

Download binaries are signed via GitHub Actions. To verify:

```bash
# Download checksum
wget https://github.com/dmtrKovalenko/fff.nvim/releases/latest/download/x86_64-unknown-linux-gnu.so.sha256
wget https://github.com/dmtrKovalenko/fff.nvim/releases/latest/download/x86_64-unknown-linux-gnu.so

# Verify
sha256sum -c x86_64-unknown-linux-gnu.so.sha256
```

### Database Permissions

Ensure database directories have restricted permissions:

```bash
chmod 700 ~/.local/state/nvim/fff_nvim
chmod 700 ~/.local/share/nvim/fff_queries
```

### Network Access

MCP server makes outbound requests for:
- Update checks (GitHub API)
- Can be disabled with `--no-update-check`

---

## Contributing

### Development Setup

```bash
git clone https://github.com/dmtrKovalenko/fff.nvim
cd fff.nvim

# Install dependencies
rustup install stable
cargo install cargo-edit

# Build
make build

# Test
make test

# Lint
make lint
```

### Code Style

```bash
# Format all code
make format

# Individual formatters
cargo fmt --all      # Rust
stylua .             # Lua
bun format           # TypeScript
```

### Running Tests

```bash
# All tests
make test

# Rust only
cargo test --workspace --features zlob

# Lua only
make test-lua

# Specific test
cargo test -p fff-core grep
```

---

## Support

### Resources

- **GitHub Issues**: https://github.com/dmtrKovalenko/fff.nvim/issues
- **Documentation**: https://docs.rs/crate/fff-search/latest
- **Installation Script**: https://github.com/dmtrKovalenko/fff.nvim/blob/main/install-mcp.sh

### Reporting Bugs

Include:
1. `:FFFHealth` output
2. Relevant log excerpts (`:FFFOpenLog`)
3. Neovim version: `:version`
4. Platform: OS, architecture
5. Steps to reproduce

---

## Appendix: Version History

### v0.5.x (Current)

- Frecency tracking with LMDB
- Query history with combo boost
- Multi-mode grep (plain/regex/fuzzy)
- Git status integration
- Real-time filesystem watching

### Planned Features

- [ ] LSP integration for symbol search
- [ ] Bookmark system
- [ ] Project switching
- [ ] Enhanced preview (images, PDFs)
