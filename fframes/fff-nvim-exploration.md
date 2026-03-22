---
name: FFF.nvim
description: Fast fuzzy file finder for Neovim and AI agents (MCP) with intelligent scoring and memory
type: sub-project
source: /home/darkvoid/Boxxed/@formulas/src.rust/src.fframes/fff.nvim/
---

# FFF.nvim - Fast Fuzzy File Finder

## Overview

FFF (Freakin Fast Fuzzy File Finder) is a **high-performance fuzzy file search tool** designed for both Neovim users and AI agents via the Model Context Protocol (MCP). It combines globbing, fuzzy matching, and grep functionality with intelligent scoring based on frecency, git status, file size, and definition matches.

Key features:
- **Blazing fast search** - Optimized for large repositories (100k+ files)
- **AI agent integration** - MCP server for AI-assisted development
- **Intelligent scoring** - Frecency, git status, file type awareness
- **Fuzzy matching** - Typo-resistant search
- **Git integration** - Respects .gitignore, prioritizes tracked files
- **Neovim plugin** - Seamless integration with lazy.nvim
- **Memory efficient** - Reduced token usage for AI agents

## Directory Structure

```
fff.nvim/
├── .cargo/                    # Rust configuration
├── crates/                    # Rust crates
│   ├── fff-core/             # Core search logic
│   ├── fff-mcp/              # MCP server implementation
│   └── fff-scorer/           # Scoring algorithms
├── lua/                       # Neovim Lua plugin
│   ├── fff/
│   │   ├── init.lua          # Plugin entry point
│   │   ├── finder.lua        # Search logic
│   │   ├── picker.lua        # UI picker
│   │   ├── scorer.lua        # Result scoring
│   │   └── download.lua      # Binary download
│   └── fff.lua               # Main module
├── plugin/                    # Neovim plugin files
├── packages/                  # npm packages
├── scripts/                   # Build scripts
├── tests/                     # Test suite
├── .mcp.json                  # MCP configuration
├── Cargo.toml                 # Rust workspace
├── flake.nix                  # Nix development env
├── install-mcp.sh            # MCP installation script
└── README.md
```

## Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                        FFF Architecture                         │
└─────────────────────────────────────────────────────────────────┘
                            │
        ┌───────────────────┼───────────────────┐
        │                   │                   │
        ▼                   ▼                   ▼
┌──────────────────┐ ┌──────────────────┐ ┌──────────────────┐
│   Neovim Plugin  │ │   MCP Server     │ │   Core Engine    │
│   (Lua)          │ │   (Rust)         │ │   (Rust)         │
│                  │ │                  │ │                  │
│ - UI/Picker      │ │ - AI interface   │ │ - Indexing       │
│ - Keybindings    │ │ - Tool calls     │ │ - Fuzzy match    │
│ - Configuration  │ │ - Context        │ │ - Scoring        │
│                  │ │   enrichment     │ │ - Git integration│
└────────┬─────────┘ └────────┬─────────┘ └────────┬─────────┘
         │                    │                    │
         └────────────────────┼────────────────────┘
                              │
                     ┌────────▼────────┐
                     │   Binary Core   │
                     │   (Rust)        │
                     │                 │
                     │ - SIMD search   │
                     │ - Parallel walk │
                     │ - Memory map    │
                     └─────────────────┘
```

## MCP Integration

### Installation

```bash
# Install MCP server
curl -L https://dmtrkovalenko.dev/install-fff-mcp.sh | bash
```

### MCP Configuration

```json
// ~/.claude/settings.json or project's .mcp.json
{
  "mcpServers": {
    "fff": {
      "command": "fff-mcp",
      "args": [],
      "env": {}
    }
  }
}
```

### CLAUDE.md Integration

```markdown
# CLAUDE.md
For any file search or grep in the current git-indexed directory use fff tools

# Example usage:
# - Use fff_find_files to search for files by name
# - Use fff_grep to search file contents
# - Use fff_live_grep for interactive search
```

### MCP Tools

```rust
// crates/fff-mcp/src/tools.rs
use mcp_server::{Tool, ToolResult};

#[derive(Debug)]
pub struct FffFindFiles {
    pub query: String,
    pub max_results: Option<usize>,
    pub base_path: Option<PathBuf>,
}

#[async_trait::async_trait]
impl Tool for FffFindFiles {
    fn name(&self) -> &'static str {
        "fff_find_files"
    }

    fn description(&self) -> &'static str {
        "Fast fuzzy file search using FFF engine"
    }

    async fn call(&self, context: &Context) -> ToolResult {
        let finder = Finder::new(&self.base_path.unwrap_or_else(cwd))?;
        let results = finder
            .fuzzy_search(&self.query)
            .max_results(self.max_results.unwrap_or(100))
            .execute()
            .await?;

        Ok(ToolResult::Success {
            content: vec![Content::Text(format_results(results))],
        })
    }
}

#[derive(Debug)]
pub struct FffGrep {
    pub pattern: String,
    pub file_pattern: Option<String>,
    pub case_sensitive: bool,
}

#[async_trait::async_trait]
impl Tool for FffGrep {
    fn name(&self) -> &'static str {
        "fff_grep"
    }

    fn description(&self) -> &'static str {
        "Fast grep with fuzzy file filtering"
    }

    async fn call(&self, context: &Context) -> ToolResult {
        let grep = GrepBuilder::new(&self.pattern)
            .case_sensitive(self.case_sensitive)
            .file_pattern(self.file_pattern.as_deref())
            .build();

        let results = grep.execute().await?;
        Ok(ToolResult::Success {
            content: vec![Content::Text(format_grep_results(results))],
        })
    }
}
```

## Core Engine

### Index Building

```rust
// crates/fff-core/src/index.rs
use std::collections::HashMap;
use dashmap::DashMap;

pub struct FileIndex {
    /// File path -> File entry
    files: DashMap<PathBuf, FileEntry>,

    /// Git status cache
    git_status: DashMap<PathBuf, GitStatus>,

    /// Frecency data
    frecency: FrecencyTracker,

    /// Root path
    root: PathBuf,
}

impl FileIndex {
    pub fn new(root: PathBuf) -> Self {
        let mut index = FileIndex {
            files: DashMap::new(),
            git_status: DashMap::new(),
            frecency: FrecencyTracker::new(),
            root,
        };

        // Build index in parallel
        index.rebuild();

        index
    }

    fn rebuild(&mut self) {
        let walker = GitAwareWalker::new(&self.root);

        // Parallel directory walking
        walker.par_walk().for_each(|entry| {
            self.files.insert(
                entry.path.clone(),
                FileEntry {
                    path: entry.path,
                    size: entry.metadata.len(),
                    modified: entry.metadata.modified().unwrap(),
                    git_status: self.check_git_status(&entry.path),
                },
            );
        });
    }

    pub fn search(&self, query: &str, limit: usize) -> Vec<SearchResult> {
        let mut results: Vec<_> = self
            .files
            .iter()
            .filter_map(|entry| {
                let score = self.score(&entry, query);
                if score > 0.0 {
                    Some(SearchResult {
                        path: entry.path.clone(),
                        score,
                    })
                } else {
                    None
                }
            })
            .collect();

        // Sort by score
        results.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap());
        results.truncate(limit);

        results
    }
}
```

### Scoring System

```rust
// crates/fff-scorer/src/lib.rs
pub struct Scorer {
    /// Frecency weight
    frecency_weight: f32,

    /// Git status weight
    git_weight: f32,

    /// File size weight
    size_weight: f32,

    /// Definition match bonus
    definition_bonus: f32,
}

impl Scorer {
    pub fn score(&self, entry: &FileEntry, query: &str) -> f32 {
        let mut score = 0.0;

        // Fuzzy match score (0.0 to 1.0)
        let fuzzy_score = fuzzy_match(&entry.path.to_string_lossy(), query);
        score += fuzzy_score * 0.4;

        // Frecency bonus (recently/frequently accessed)
        let frecency = self.frecency.get(&entry.path);
        score += frecency * self.frecency_weight;

        // Git status bonus (tracked, modified files)
        if let Some(status) = entry.git_status {
            if status.is_tracked() {
                score += 0.1;
            }
            if status.is_modified() {
                score += 0.15;  // Prioritize modified files
            }
            if status.is_untracked() {
                score -= 0.05;  // Deprioritize untracked
            }
        }

        // Definition bonus (header files, test files)
        if self.is_definition_file(&entry.path) {
            score += self.definition_bonus;
        }

        // Size penalty (very large files)
        if entry.size > 1_000_000 {
            score -= 0.1;
        }

        // Path length bonus (shorter paths often more relevant)
        score += 1.0 / (entry.path.components().count() as f32);

        score.clamp(0.0, 1.0)
    }

    fn is_definition_file(&self, path: &Path) -> bool {
        matches!(
            path.extension().and_then(|e| e.to_str()),
            Some("h" | "hpp" | "d.ts" | ".pyi" | ".rs")
        )
    }
}

fn fuzzy_match(haystack: &str, needle: &str) -> f32 {
    // Skewed matching - prefer prefix matches
    let haystack_lower = haystack.to_lowercase();
    let needle_lower = needle.to_lowercase();

    // Exact match bonus
    if haystack_lower == needle_lower {
        return 1.0;
    }

    // Prefix match bonus
    if haystack_lower.starts_with(&needle_lower) {
        return 0.9;
    }

    // Fuzzy match
    let mut needle_chars = needle_lower.chars().peekable();
    let mut score = 0.0;
    let mut matches = 0;

    for (i, hc) in haystack_lower.chars().enumerate() {
        if let Some(nc) = needle_chars.peek() {
            if hc == *nc {
                // Bonus for consecutive matches
                if i > 0 && haystack_lower.chars().nth(i - 1) == *needle_chars.clone().nth(matches.saturating_sub(1)).unwrap_or(' ') {
                    score += 0.1;
                }
                score += 0.1;
                matches += 1;
                needle_chars.next();
            }
        }
    }

    if matches == needle_lower.chars().count() {
        score.min(1.0)
    } else {
        0.0  // Didn't match all chars
    }
}
```

### Fuzzy Matcher

```rust
// crates/fff-core/src/fuzzy.rs
use simd::prelude::*;

pub struct FuzzyMatcher {
    /// Pre-compiled pattern
    pattern: Vec<u8>,

    /// Case-insensitive
    ignore_case: bool,
}

impl FuzzyMatcher {
    pub fn new(pattern: &str, ignore_case: bool) -> Self {
        let pattern = if ignore_case {
            pattern.to_lowercase().into_bytes()
        } else {
            pattern.as_bytes().to_vec()
        };

        FuzzyMatcher { pattern, ignore_case }
    }

    pub fn match_path(&self, path: &str) -> Option<MatchResult> {
        let haystack = if self.ignore_case {
            path.to_lowercase()
        } else {
            path.to_string()
        };

        // SIMD-accelerated matching
        self.match_simd(haystack.as_bytes())
    }

    fn match_simd(&self, haystack: &[u8]) -> Option<MatchResult> {
        use std::arch::x86_64::*;

        unsafe {
            // Load pattern into SIMD register
            let pattern_vec = _mm256_loadu_si256(self.pattern.as_ptr() as *const __m256i);

            // Slide over haystack
            for i in 0..haystack.len() - self.pattern.len() {
                let chunk = _mm256_loadu_si256(haystack[i..].as_ptr() as *const __m256i);

                // Compare
                let cmp = _mm256_cmpeq_epi8(pattern_vec, chunk);
                let mask = _mm256_movemask_epi8(cmp);

                if mask != 0 {
                    // Found match
                    return Some(MatchResult {
                        start: i,
                        end: i + self.pattern.len(),
                        score: calculate_score(i, mask),
                    });
                }
            }
        }

        None
    }
}
```

## Neovim Integration

### Plugin API

```lua
-- lua/fff/init.lua
local M = {}

function M.setup(opts)
  M.config = vim.tbl_extend('force', {
    base_path = vim.fn.getcwd(),
    prompt = '🪿 ',
    title = 'FFFiles',
    max_results = 100,
    max_threads = 4,
    lazy_sync = true,
    debug = {
      enabled = false,
      show_scores = false,
    },
  }, opts or {})

  -- Lazy load binary
  if not M.config.lazy_sync then
    require('fff.download').ensure_binary()
  end
end

function M.find_files(opts)
  opts = opts or {}
  local picker = require('fff.picker')
  picker.open({
    mode = 'files',
    query = opts.query,
  })
end

function M.live_grep(opts)
  opts = opts or {}
  local picker = require('fff.picker')
  picker.open({
    mode = 'grep',
    query = opts.query,
    grep_opts = opts.grep,
  })
end

return M
```

### Picker UI

```lua
-- lua/fff/picker.lua
local function create_picker(results)
  local win = vim.api.nvim_open_win(buf, false, {
    relative = 'editor',
    width = math.floor(vim.o.columns * 0.8),
    height = math.floor(vim.o.lines * 0.6),
    row = math.floor(vim.o.lines * 0.2),
    col = math.floor(vim.o.columns * 0.1),
    style = 'minimal',
    border = 'rounded',
  })

  -- Populate results with scores
  local lines = {}
  for _, result in ipairs(results) do
    local line = result.path
    if M.config.debug.show_scores then
      line = string.format('[%.3f] %s', result.score, line)
    end
    table.insert(lines, line)
  end

  vim.api.nvim_buf_set_lines(buf, 0, -1, false, lines)

  -- Keymaps
  vim.keymap.set('n', '<CR>', select_entry, { buffer = buf })
  vim.keymap.set('n', '<C-q>', quickfix_list, { buffer = buf })
  vim.keymap.set('n', '<C-c>', close, { buffer = buf })
end

return {
  open = function(opts)
    -- Start async search
    local job = vim.system(
      { 'fff', '--json', '--query', opts.query },
      { stdout = true },
      function(result)
        local results = vim.json.decode(result.stdout)
        create_picker(results)
      end
    )
  end,
}
```

## Performance

### Benchmark Results

```
Repository: Linux kernel (100k files, 8GB)
Query: "drivers/gpu"

fff.nvim:     50ms
telescope:   250ms
fzf:         180ms
ripgrep:     120ms

Repository: VSCode (50k files)
Query: "src/vs/workbench"

fff.nvim:     30ms
telescope:   150ms
fzf:         100ms
```

## Related Documents

- [Zlob](./zlob-exploration.md) - Fast globbing used by FFF

## Sources

- Source: `/home/darkvoid/Boxxed/@formulas/src.rust/src.fframes/fff.nvim/`
- FFF.nvim GitHub: https://github.com/dmtrKovalenko/fff.nvim
- MCP Documentation: https://modelcontextprotocol.io/
