# FFF Grep Engine Deep Dive

**Source:** `/home/darkvoid/Boxxed/@formulas/src.rust/src.llamacpp/src.ClaudOpen/fff.nvim/crates/fff-core/src/grep.rs`

---

## Overview

The FFF grep engine is a high-performance content search system supporting three modes:
- **Plain Text**: Literal case-insensitive search (fastest)
- **Regex**: Full regular expression support
- **Fuzzy**: Smith-Waterman fuzzy matching

Key features:
- SIMD-optimized plain text matching
- Aho-Corasick multi-pattern matching
- Memory-mapped file access
- Definition detection and classification
- Parallel execution via rayon

---

## Architecture

```
grep.rs (81KB)
├── GrepMode enum           # Search mode selection
├── GrepSearchOptions       # Configuration struct
├── GrepResult              # Result structure
├── grep_search()           # Main entry point
├── search_file()           # Per-file search
├── MatchSink               # Match collection
├── is_definition_line()    # Definition detection
├── is_import_line()        # Import detection
└── classify_match()        # Match classification
```

### Core Types

```rust
/// Grep search mode
pub enum GrepMode {
    PlainText,   // Case-insensitive literal match
    Regex,       // Full regex via grep-regex
    Fuzzy,       // Smith-Waterman scoring
}

/// Search configuration
pub struct GrepSearchOptions {
    pub max_file_size: u64,         // Skip files > this size
    pub max_matches_per_file: usize, // Limit matches per file
    pub smart_case: bool,           // Case-sensitive if query has uppercase
    pub file_offset: usize,         // For pagination
    pub page_limit: usize,          // Max files to return
    pub mode: GrepMode,             // Search mode
    pub time_budget_ms: u64,        // Max search time (0 = unlimited)
    pub before_context: usize,      // Lines before match
    pub after_context: usize,       // Lines after match
    pub classify_definitions: bool, // Tag definition matches
}

/// Single match within a file
pub struct GrepMatch {
    pub line_number: usize,
    pub column: usize,
    pub line: String,
    pub matched_text: String,
    pub match_type: MatchType,  // Definition, Usage, Import, etc.
    pub is_definition: bool,
}

/// All matches in a single file
pub struct GrepFileMatch {
    pub path: String,
    pub matches: Vec<GrepMatch>,
    pub is_definition_file: bool,  // File contains definition
}

/// Complete grep result
pub struct GrepResult {
    pub files: Vec<GrepFileMatch>,
    pub total_matched: usize,
    pub total_files: usize,
    pub timed_out: bool,
}
```

---

## Search Flow

### High-Level Flow

```
1. Parse query (constraints + search text)
2. Filter files by constraints
3. Build matcher based on mode
4. Search files in parallel (rayon)
5. Collect and sort results by frecency
6. Apply pagination
7. Return GrepResult
```

### Implementation

```rust
pub fn grep_search(
    files: &[FileItem],
    options: &GrepSearchOptions,
    frecency: Option<&FrecencyTracker>,
) -> GrepResult {
    // 1. Build matcher based on mode
    let matcher = match options.mode {
        GrepMode::PlainText => {
            // Case-insensitive memmem (SIMD optimized)
            Box::new(CaseInsensitiveMatcher::new(&options.pattern))
        }
        GrepMode::Regex => {
            // grep-regex matcher
            Box::new(RegexMatcher::new(&options.pattern, options.smart_case))
        }
        GrepMode::Fuzzy => {
            // Fuzzy matcher using neo_frizbee
            Box::new(FuzzyMatcher::new(&options.pattern))
        }
    };

    // 2. Build searcher with context lines
    let searcher = SearcherBuilder::new()
        .before_context(options.before_context)
        .after_context(options.after_context)
        .build();

    // 3. Time budget tracking
    let start_time = std::time::Instant::now();
    let timeout = Duration::from_millis(options.time_budget_ms);

    // 4. Search in parallel
    let mut results: Vec<GrepFileMatch> = files
        .par_iter()  // Parallel iterator
        .filter_map(|file| {
            // Check timeout
            if options.time_budget_ms > 0 && start_time.elapsed() > timeout {
                return None;
            }

            // Skip binary/large files
            if file.is_binary || file.size > options.max_file_size {
                return None;
            }

            // Get content (lazy mmap)
            let content = file.get_content(budget)?;

            // Search file
            let mut sink = MatchSink::default();
            searcher.search_slice(&matcher, &content, &mut sink).ok()?;

            if sink.matches.is_empty() {
                return None;
            }

            Some(GrepFileMatch {
                path: file.relative_path.clone(),
                matches: sink.matches,
                is_definition_file: sink.has_definition,
            })
        })
        .collect();

    // 5. Sort by frecency
    if let Some(frecency) = frecency {
        results.par_sort_by(|a, b| {
            let score_a = frecency.get_frecency_score(&a.path).unwrap_or(0);
            let score_b = frecency.get_frecency_score(&b.path).unwrap_or(0);
            score_b.cmp(&score_a)  // Descending
        });
    }

    // 6. Apply pagination
    let total_matched = results.iter().map(|f| f.matches.len()).sum();
    let total_files = results.len();

    results.truncate(options.file_offset + options.page_limit);

    GrepResult {
        files: results,
        total_matched,
        total_files,
        timed_out: start_time.elapsed() > timeout,
    }
}
```

---

## Plain Text Matching

### Case-Insensitive Memmem

The plain text matcher uses a custom SIMD-optimized case-insensitive substring search:

```rust
/// Case-insensitive ASCII substring search
pub struct CaseInsensitiveMatcher {
    needle: Vec<u8>,  // Lowercase pattern
}

impl CaseInsensitiveMatcher {
    pub fn new(pattern: &str) -> Self {
        Self {
            needle: pattern.as_bytes().to_ascii_lowercase(),
        }
    }

    /// Find all matches in haystack
    pub fn find_iter<'a>(&self, haystack: &'a [u8]) -> Vec<Match> {
        let mut matches = Vec::new();
        let needle = &self.needle;

        if needle.is_empty() {
            return matches;
        }

        let first = needle[0];
        for i in 0..=haystack.len().saturating_sub(needle.len()) {
            // Fast first-byte check
            if haystack[i].to_ascii_lowercase() == first {
                // Check rest of pattern
                if haystack[i..i + needle.len()]
                    .iter()
                    .zip(needle)
                    .all(|(a, b)| a.to_ascii_lowercase() == *b)
                {
                    matches.push(Match {
                        start: i,
                        end: i + needle.len(),
                    });
                }
            }
        }

        matches
    }
}
```

### SIMD Optimization

For longer patterns, uses SIMD for parallel byte comparison:

```rust
#[cfg(target_feature = "sse2")]
#[target_feature(enable = "sse2")]
unsafe fn find_simd(haystack: &[u8], needle: &[u8]) -> Vec<Match> {
    use std::arch::x86_64::*;

    // Process 16 bytes at a time using SSE2
    // ... SIMD implementation
}
```

---

## Regex Matching

### grep-regex Integration

```rust
use grep_regex::RegexMatcherBuilder;

pub struct RegexMatcher {
    matcher: grep_regex::RegexMatcher,
}

impl RegexMatcher {
    pub fn new(pattern: &str, smart_case: bool) -> Result<Self> {
        let mut builder = RegexMatcherBuilder::new();

        // Smart case: case-insensitive unless pattern has uppercase
        if smart_case && !pattern.chars().any(|c| c.is_uppercase()) {
            builder.case_insensitive(true);
        }

        // Enable line numbers
        builder.line_number(true);

        let matcher = builder.build(pattern)?;

        Ok(Self { matcher })
    }
}
```

### Regex Features Supported

- Character classes: `[a-z]`, `[^0-9]`
- Quantifiers: `*`, `+`, `?`, `{n}`, `{n,}`, `{n,m}`
- Alternation: `foo|bar`
- Anchors: `^`, `$`, `\b`
- Groups: `(...)`, `(?:...)`
- Word boundaries: `\b`, `\B`

---

## Fuzzy Matching

### Smith-Waterman Scoring

```rust
use neo_frizbee::{Config, match_list};

pub struct FuzzyMatcher {
    pattern: String,
    config: Config,
}

impl FuzzyMatcher {
    pub fn new(pattern: &str) -> Self {
        Self {
            pattern: pattern.to_string(),
            config: Config {
                max_typos: Some(2),
                // ... other config
            },
        }
    }

    pub fn find_in_lines(&self, lines: &[&str]) -> Vec<FuzzyMatch> {
        let matches = match_list(&self.pattern, lines, &self.config);

        matches.into_iter()
            .map(|m| FuzzyMatch {
                line_index: m.index,
                score: m.score,
                start: m.start,
                end: m.end,
            })
            .collect()
    }
}
```

### Quality Threshold

To avoid overly fuzzy matches:

```rust
const FUZZY_QUALITY_THRESHOLD: u16 = 50;  // Minimum score

fn filter_low_quality(matches: Vec<FuzzyMatch>) -> Vec<FuzzyMatch> {
    matches.into_iter()
        .filter(|m| m.score >= FUZZY_QUALITY_THRESHOLD)
        .collect()
}
```

---

## Match Classification

### Definition Detection

```rust
/// Detect if a line looks like a code definition
pub fn is_definition_line(line: &str) -> bool {
    let s = line.trim_start().as_bytes();
    let s = skip_modifiers(s);
    is_definition_keyword(s)
}

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
        // Handle pub(...) visibility
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
```

### Import Detection

```rust
pub fn is_import_line(line: &str) -> bool {
    let s = line.trim_start().as_bytes();
    s.starts_with(b"import ")
        || s.starts_with(b"import\t")
        || (s.starts_with(b"from ") && s.get(5).is_some_and(|&b| b == b'\'' || b == b'"'))
        || s.starts_with(b"use ")
        || s.starts_with(b"use\t")
        || starts_with_require(s)
        || starts_with_include(s)
}

fn starts_with_require(s: &[u8]) -> bool {
    if !s.starts_with(b"require") {
        return false;
    }
    let rest = &s[b"require".len()..];
    rest.first() == Some(&b'(') || (rest.first() == Some(&b' ') && rest.get(1) == Some(&b'('))
}

fn starts_with_include(s: &[u8]) -> bool {
    if s.first() != Some(&b'#') {
        return false;
    }
    let rest = skip_ws(&s[1..]);
    rest.starts_with(b"include ") || rest.starts_with(b"include\t")
}
```

### Match Type Classification

```rust
pub enum MatchType {
    Definition,      // struct/func/class definition
    Usage,           // Regular usage
    Import,          // import/use statement
    Comment,         // Match in comment
    String,          // Match in string literal
}

pub fn classify_match(line: &str, match_start: usize) -> MatchType {
    // Check if definition
    if is_definition_line(line) {
        return MatchType::Definition;
    }

    // Check if import
    if is_import_line(line) {
        return MatchType::Import;
    }

    // Check if in comment (simple heuristic)
    let before_match = &line[..match_start];
    if before_match.contains("//") || before_match.contains("/*") {
        return MatchType::Comment;
    }

    // Check if in string (simple heuristic)
    let quote_count = before_match.chars().filter(|&c| c == '"').count();
    if quote_count % 2 == 1 {
        return MatchType::String;
    }

    MatchType::Usage
}
```

---

## Match Sink

### Collecting Matches

```rust
use grep_searcher::{Sink, SinkMatch};

#[derive(Default)]
pub struct MatchSink {
    pub matches: Vec<GrepMatch>,
    pub has_definition: bool,
    max_matches: usize,
}

impl MatchSink {
    pub fn new(max_matches: usize) -> Self {
        Self {
            matches: Vec::with_capacity(max_matches.min(100)),
            has_definition: false,
            max_matches,
        }
    }
}

impl Sink for MatchSink {
    type Error = std::io::Error;

    fn matched(&mut self, _path: &Path, line: &SinkMatch) -> Result<bool, Self::Error> {
        if self.max_matches > 0 && self.matches.len() >= self.max_matches {
            return Ok(false);  // Stop searching
        }

        let line_str = std::str::from_utf8(line.lines().next().unwrap_or(&[])).unwrap_or("");
        let match_start = line.absolute_byte_offset() as usize;

        // Classify the match
        let match_type = classify_match(line_str, match_start);
        let is_definition = matches!(match_type, MatchType::Definition);

        if is_definition {
            self.has_definition = true;
        }

        self.matches.push(GrepMatch {
            line_number: line.line_number() as usize,
            column: match_start,
            line: line_str.to_string(),
            matched_text: line.bytes().to_vec(),
            match_type,
            is_definition,
        });

        Ok(true)  // Continue searching
    }
}
```

---

## Aho-Corasick Multi-Pattern

For searching multiple patterns simultaneously:

```rust
use aho_corasick::{AhoCorasick, AhoCorasickKind};

pub struct MultiPatternMatcher {
    ac: AhoCorasick,
}

impl MultiPatternMatcher {
    pub fn new(patterns: &[&str]) -> Self {
        Self {
            ac: AhoCorasick::builder()
                .kind(Some(AhoCorasickKind::DFA))
                .build(patterns)
                .unwrap(),
        }
    }

    pub fn find_all(&self, haystack: &str) -> Vec<Match> {
        self.ac.find_iter(haystack)
            .map(|m| Match {
                start: m.start(),
                end: m.end(),
                pattern_index: m.pattern().as_usize(),
            })
            .collect()
    }
}
```

Usage for multi_grep:
```rust
let patterns = vec!["ActorAuth", "PopulatedActorAuth", "actor_auth"];
let matcher = MultiPatternMatcher::new(&patterns);
let matches = matcher.find_all(&content);
```

---

## Context Lines

### Implementation

```rust
pub fn add_context_lines(
    lines: &[String],
    match_line: usize,
    before: usize,
    after: usize,
) -> Vec<String> {
    let start = match_line.saturating_sub(before);
    let end = (match_line + 1 + after).min(lines.len());

    lines[start..end].to_vec()
}

// In MatchSink
fn matched(&mut self, _path: &Path, line: &SinkMatch) -> Result<bool, Self::Error> {
    // ... collect match ...

    // Add context if configured
    if self.options.before_context > 0 || self.options.after_context > 0 {
        match.context_lines = add_context_lines(
            &self.file_lines,
            line.line_number() as usize,
            self.options.before_context,
            self.options.after_context,
        );
    }

    Ok(true)
}
```

---

## Performance Optimizations

### 1. Early Termination

```rust
fn matched(&mut self, _path: &Path, line: &SinkMatch) -> Result<bool, Self::Error> {
    if self.max_matches > 0 && self.matches.len() >= self.max_matches {
        return Ok(false);  // Stop searching this file
    }
    Ok(true)
}
```

### 2. File Size Filtering

```rust
// Skip binary/large files
if file.is_binary || file.size > options.max_file_size {
    return None;
}
```

### 3. Parallel Search

```rust
files.par_iter()
    .filter_map(|file| search_file(file))
    .collect()
```

### 4. Time Budget

```rust
let start_time = Instant::now();
let timeout = Duration::from_millis(options.time_budget_ms);

files.par_iter().filter_map(|file| {
    if options.time_budget_ms > 0 && start_time.elapsed() > timeout {
        return None;  // Timeout
    }
    search_file(file)
})
```

### 5. Content Caching

```rust
// File content cached in FileItem.content (OnceLock)
// Subsequent searches reuse cached mmap
let content = file.get_content(budget)?;
```

---

## Usage Examples

### Basic Grep

```rust
let options = GrepSearchOptions {
    mode: GrepMode::PlainText,
    max_file_size: 10 * 1024 * 1024,
    max_matches_per_file: 100,
    smart_case: true,
    time_budget_ms: 150,
    ..Default::default()
};

let result = grep_search(&files, &options, Some(&frecency));
```

### Regex Grep

```rust
let options = GrepSearchOptions {
    mode: GrepMode::Regex,
    ..Default::default()
};

let result = grep_search(&files, &options, None);
// Searches for regex pattern like "fn\s+\w+\("
```

### Fuzzy Grep

```rust
let options = GrepSearchOptions {
    mode: GrepMode::Fuzzy,
    ..Default::default()
};

let result = grep_search(&files, &options, Some(&frecency));
// Fuzzy matches like "mtxlk" -> "mutex_lock"
```

### With Context Lines

```rust
let options = GrepSearchOptions {
    mode: GrepMode::PlainText,
    before_context: 2,
    after_context: 3,
    ..Default::default()
};

let result = grep_search(&files, &options, None);
// Returns 2 lines before and 3 lines after each match
```

---

## Testing

### Unit Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_definition_line() {
        assert!(is_definition_line("pub struct Foo {"));
        assert!(is_definition_line("fn bar() {}"));
        assert!(is_definition_line("class Baz {"));
        assert!(!is_definition_line("let x = foo();"));
    }

    #[test]
    fn test_is_import_line() {
        assert!(is_import_line("use std::io;"));
        assert!(is_import_line("import { foo } from 'bar';"));
        assert!(is_import_line("#include <stdio.h>"));
        assert!(!is_import_line("let import = true;"));
    }

    #[test]
    fn test_case_insensitive_matcher() {
        let matcher = CaseInsensitiveMatcher::new("Hello");
        let matches = matcher.find_iter(b"hello WORLD, HELLO!");
        assert_eq!(matches.len(), 2);
    }
}
```

### Integration Tests

```rust
#[test]
fn test_grep_basic() {
    let temp_dir = tempfile::tempdir().unwrap();
    let file_path = temp_dir.path().join("test.rs");
    std::fs::write(&file_path, "fn main() { println!(\"hello\"); }").unwrap();

    let file = FileItem::new(file_path, temp_dir.path(), None);

    let options = GrepSearchOptions {
        mode: GrepMode::PlainText,
        ..Default::default()
    };

    let result = grep_search(&[file], &options, None);
    assert!(result.total_matched > 0);
}
```

---

## Next Steps

- See [01-fff-exploration.md](./01-fff-exploration.md) for full architecture
- See [rust-revision.md](./rust-revision.md) for Rust patterns
- See [production-grade.md](./production-grade.md) for deployment guide
