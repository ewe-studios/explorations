# FFF Query Parser Deep Dive

**Source:** `/home/darkvoid/Boxxed/@formulas/src.rust/src.llamacpp/src.ClaudOpen/fff.nvim/crates/fff-query-parser`

---

## Overview

The FFF Query Parser transforms user input into structured queries with constraints, fuzzy patterns, and location information. It supports a rich syntax for file filtering and search refinement.

### Features

- **Constraint parsing**: Git status, globs, extensions, path segments
- **Negation support**: Exclude results with `!` prefix
- **Location parsing**: Line/column jumps (`:10:5`)
- **Multiple fuzzy parts**: Space-separated search terms
- **Glob support**: Optional zlob integration for fast globbing

---

## Query Syntax

### Basic Queries

```
# Simple text search
main.rs           # Find files containing "main.rs"
src/lib           # Find files with "src" and "lib"
test utils        # Match both "test" AND "utils"
```

### Constraints

```
# Extension filter
*.rs              # Rust files only
*.{ts,tsx}        # TypeScript files
*.md              # Markdown files

# Directory filter
src/              # Files in src/ (or children)
lib/utils/        # Files in lib/utils/

# Git status
git:modified      # Modified unstaged files
git:staged        # Staged files
git:deleted       # Deleted files
git:renamed       # Renamed files
git:untracked     # Untracked files
git:ignored       # Ignored files

# Filename filter
main.rs           # Files named exactly "main.rs"
schema.json       # Files named exactly "schema.json"

# Glob patterns (with zlob feature)
**/*.rs           # Any .rs file
src/**/*.ts       # .ts files under src/
**/test_*.rs      # Test files

# Negation (exclude)
!test/            # Exclude test directories
!*.spec.ts        # Exclude spec files
!git:ignored      # Exclude ignored files

# Size filter
size:>1mb         # Files larger than 1MB
size:<100kb       # Files smaller than 100KB
size:>1kb,<1mb    # Between 1KB and 1MB

# Modification time
modified:<1h      # Modified in last hour
modified:<1d      # Modified in last day
modified:>7d      # Modified more than 7 days ago
```

### Combined Queries

```
# Multiple constraints + search terms
git:modified src/**/*.rs !test/ user controller

# Breakdown:
# - git:modified    -> Only modified files
# - src/**/*.rs     -> In src/ with .rs extension
# - !test/          -> Exclude test/ directories
# - user controller -> Must match BOTH "user" AND "controller"
```

### Location Syntax

```
# Line number
file.rs:10          # Jump to line 10

# Line and column
file.rs:10:5        # Jump to line 10, column 5

# With constraints
src/main.rs:20:3    # File in src/, jump to 20:3
```

---

## Architecture

```
fff-query-parser/
├── src/
│   ├── lib.rs              # Public API
│   ├── parser.rs           # Main parsing logic
│   ├── constraint.rs       # Constraint types
│   ├── fuzzy_query.rs      # Fuzzy query types
│   ├── location.rs         # Location parsing
│   ├── size_constraint.rs  # Size filter parsing
│   └── modified_constraint.rs  # Time filter parsing
└── Cargo.toml
```

### Core Types

```rust
/// Parsed query structure
pub struct FFFQuery<'a> {
    pub constraints: Vec<Constraint<'a>>,
    pub fuzzy_query: FuzzyQuery<'a>,
    pub location: Option<Location>,
}

/// Constraint types for filtering
pub enum Constraint<'a> {
    Extension(&'a str),      // *.rs
    Glob(&'a str),           // **/*.rs
    PathSegment(&'a str),    // src/
    Filename(&'a str),       // main.rs
    GitStatus(GitStatusFilter), // git:modified
    Size(SizeConstraint),    // size:>1mb
    Modified(ModifiedConstraint), // modified:<1h
}

/// Fuzzy query component
pub enum FuzzyQuery<'a> {
    Text(&'a str),           // Single text query
    Parts(Vec<&'a str>),     // Multiple parts (space-separated)
}

/// Location for jumping
pub struct Location {
    pub line: u32,
    pub column: Option<u32>,
}

/// Git status filters
pub enum GitStatusFilter {
    Modified,
    Staged,
    Deleted,
    Renamed,
    Untracked,
    Ignored,
}

/// Size constraint
pub struct SizeConstraint {
    pub min: Option<u64>,  // Minimum bytes
    pub max: Option<u64>,  // Maximum bytes
}

/// Modification time constraint
pub struct ModifiedConstraint {
    pub max_age_secs: Option<u64>,  // Maximum age
    pub min_age_secs: Option<u64>,  // Minimum age
}
```

---

## Parser Implementation

### Main Parser

```rust
pub struct QueryParser {
    config: FileSearchConfig,  // or GrepConfig
}

impl QueryParser {
    pub fn new(config: FileSearchConfig) -> Self {
        Self { config }
    }

    pub fn parse<'a>(&self, query: &'a str) -> FFFQuery<'a> {
        let mut constraints = Vec::new();
        let mut fuzzy_parts = Vec::new();
        let mut location = None;

        // Split by whitespace
        let parts = query.split_whitespace();

        for part in parts {
            // Check for negation
            let (negated, token) = if let Some(stripped) = part.strip_prefix('!') {
                (true, stripped)
            } else {
                (false, part)
            };

            // Try to parse as constraint
            if let Some(constraint) = self.parse_constraint(token) {
                if negated {
                    // Wrap in negation (handled by constraint filter)
                    constraints.push(Constraint::Not(Box::new(constraint)));
                } else {
                    constraints.push(constraint);
                }
            } else if let Some(loc) = parse_location(token) {
                location = Some(loc);
            } else {
                // Regular fuzzy part
                fuzzy_parts.push(token);
            }
        }

        // Build fuzzy query
        let fuzzy_query = match fuzzy_parts.len() {
            0 => FuzzyQuery::Text(""),
            1 => FuzzyQuery::Text(fuzzy_parts[0]),
            _ => FuzzyQuery::Parts(fuzzy_parts),
        };

        FFFQuery {
            constraints,
            fuzzy_query,
            location,
        }
    }

    fn parse_constraint<'a>(&self, token: &'a str) -> Option<Constraint<'a>> {
        // Git status
        if let Some(status) = token.strip_prefix("git:") {
            return Some(Constraint::GitStatus(parse_git_status(status)?));
        }

        // Size filter
        if let Some(size_str) = token.strip_prefix("size:") {
            return Some(Constraint::Size(parse_size_constraint(size_str)?));
        }

        // Modification time
        if let Some(mod_str) = token.strip_prefix("modified:") {
            return Some(Constraint::Modified(parse_modified_constraint(mod_str)?));
        }

        // Extension
        if token.starts_with("*.") || token.starts_with("*.*") {
            return Some(Constraint::Extension(extract_extension(token)?));
        }

        // Glob (contains **)
        if token.contains("**") {
            return Some(Constraint::Glob(token));
        }

        // Path segment (ends with /)
        if token.ends_with('/') {
            return Some(Constraint::PathSegment(token));
        }

        // Filename (contains . and no /)
        if token.contains('.') && !token.contains('/') {
            return Some(Constraint::Filename(token));
        }

        None
    }
}
```

### Constraint Parsing

```rust
/// Parse git status filter
fn parse_git_status(status: &str) -> Option<GitStatusFilter> {
    match status {
        "modified" => Some(GitStatusFilter::Modified),
        "staged" => Some(GitStatusFilter::Staged),
        "deleted" => Some(GitStatusFilter::Deleted),
        "renamed" => Some(GitStatusFilter::Renamed),
        "untracked" => Some(GitStatusFilter::Untracked),
        "ignored" => Some(GitStatusFilter::Ignored),
        _ => None,
    }
}

/// Parse size constraint
fn parse_size_constraint(s: &str) -> Option<SizeConstraint> {
    let mut min = None;
    let mut max = None;

    for part in s.split(',') {
        if let Some(size_str) = part.strip_prefix('>') {
            min = Some(parse_size_value(size_str)?);
        } else if let Some(size_str) = part.strip_prefix('<') {
            max = Some(parse_size_value(size_str)?);
        }
    }

    Some(SizeConstraint { min, max })
}

/// Parse size value (1kb, 1mb, 1gb)
fn parse_size_value(s: &str) -> Option<u64> {
    let s = s.trim();
    let (num, unit) = s.split_at(s.find(|c: char| !c.is_ascii_digit())?);
    let num: u64 = num.parse().ok()?;

    match unit.to_lowercase().as_str() {
        "b" => Some(num),
        "kb" | "k" => Some(num * 1024),
        "mb" | "m" => Some(num * 1024 * 1024),
        "gb" | "g" => Some(num * 1024 * 1024 * 1024),
        _ => None,
    }
}

/// Parse modification time constraint
fn parse_modified_constraint(s: &str) -> Option<ModifiedConstraint> {
    let mut max_age_secs = None;
    let mut min_age_secs = None;

    for part in s.split(',') {
        if let Some(time_str) = part.strip_prefix('<') {
            max_age_secs = Some(parse_time_value(time_str)?);
        } else if let Some(time_str) = part.strip_prefix('>') {
            min_age_secs = Some(parse_time_value(time_str)?);
        }
    }

    Some(ModifiedConstraint { max_age_secs, min_age_secs })
}

/// Parse time value (1h, 30m, 7d)
fn parse_time_value(s: &str) -> Option<u64> {
    let s = s.trim();
    let num_end = s.find(|c: char| !c.is_ascii_digit())?;
    let (num_str, unit) = s.split_at(num_end);
    let num: u64 = num_str.parse().ok()?;

    match unit.to_lowercase().as_str() {
        "s" | "sec" | "secs" | "second" | "seconds" => Some(num),
        "m" | "min" | "mins" | "minute" | "minutes" => Some(num * 60),
        "h" | "hr" | "hrs" | "hour" | "hours" => Some(num * 60 * 60),
        "d" | "day" | "days" => Some(num * 60 * 60 * 24),
        "w" | "week" | "weeks" => Some(num * 60 * 60 * 24 * 7),
        _ => None,
    }
}
```

### Location Parsing

```rust
/// Parse location (line:column)
fn parse_location(token: &str) -> Option<Location> {
    // Must contain at least one colon
    if !token.contains(':') {
        return None;
    }

    let parts: Vec<&str> = token.split(':').collect();

    match parts.len() {
        2 => {
            // line or line:column
            let line: u32 = parts[0].parse().ok()?;
            let column: Option<u32> = parts[1].parse().ok();
            Some(Location { line, column })
        }
        3 => {
            // line:column (3 parts means empty first or last)
            if parts[0].is_empty() {
                let line: u32 = parts[1].parse().ok()?;
                let column: u32 = parts[2].parse().ok()?;
                Some(Location { line, column: Some(column) })
            } else {
                None
            }
        }
        _ => None,
    }
}
```

---

## Constraint Application

### Filter Implementation

```rust
/// Check if file matches all constraints
pub fn matches_all_constraints<T: Constrainable>(
    file: &T,
    constraints: &[Constraint],
) -> bool {
    constraints.iter().all(|constraint| {
        match constraint {
            Constraint::Extension(ext) => {
                file_has_extension(file.file_name(), ext)
            }
            Constraint::PathSegment(segment) => {
                path_contains_segment(file.relative_path(), segment)
            }
            Constraint::Filename(name) => {
                file.file_name().eq_ignore_ascii_case(name)
            }
            Constraint::GitStatus(filter) => {
                matches_git_status(file.git_status(), filter)
            }
            Constraint::Glob(pattern) => {
                #[cfg(feature = "zlob")]
                {
                    glob_matches(file.relative_path(), pattern)
                }
                #[cfg(not(feature = "zlob"))]
                {
                    // Fallback to simple glob
                    simple_glob(file.relative_path(), pattern)
                }
            }
            Constraint::Size(size) => {
                matches_size_constraint(file.size(), size)
            }
            Constraint::Modified(modified) => {
                matches_modified_constraint(file.modified(), modified)
            }
            Constraint::Not(inner) => {
                !matches_all_constraints(file, &[inner.as_ref()])
            }
        }
    })
}
```

### Extension Matching

```rust
/// Check if file extension matches (no allocation)
#[inline]
pub fn file_has_extension(file_name: &str, ext: &str) -> bool {
    // ext is like "rs" (without the dot)
    if file_name.len() <= ext.len() + 1 {
        return false;  // Too short for ".ext"
    }
    let start = file_name.len() - ext.len() - 1;
    file_name.as_bytes().get(start) == Some(&b'.')
        && file_name[start + 1..].eq_ignore_ascii_case(ext)
}
```

### Path Segment Matching

```rust
/// Check if path contains segment (no allocation)
#[inline]
pub fn path_contains_segment(path: &str, segment: &str) -> bool {
    // Remove trailing slash for comparison
    let segment = segment.strip_suffix('/').unwrap_or(segment);

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

### Git Status Matching

```rust
/// Check if file matches git status filter
fn matches_git_status(
    file_status: Option<git2::Status>,
    filter: &GitStatusFilter,
) -> bool {
    let Some(status) = file_status else {
        return false;
    };

    match filter {
        GitStatusFilter::Modified => {
            status.intersects(git2::Status::INDEX_MODIFIED | git2::Status::WT_MODIFIED)
        }
        GitStatusFilter::Staged => {
            status.intersects(git2::Status::INDEX_NEW | git2::Status::INDEX_MODIFIED)
        }
        GitStatusFilter::Deleted => {
            status.intersects(git2::Status::INDEX_DELETED | git2::Status::WT_DELETED)
        }
        GitStatusFilter::Renamed => {
            status.intersects(git2::Status::INDEX_RENAMED)
        }
        GitStatusFilter::Untracked => {
            status.intersects(git2::Status::WT_NEW)
        }
        GitStatusFilter::Ignored => {
            status.intersects(git2::Status::IGNORED)
        }
    }
}
```

### Size Constraint Matching

```rust
fn matches_size_constraint(file_size: u64, constraint: &SizeConstraint) -> bool {
    if let Some(min) = constraint.min {
        if file_size < min {
            return false;
        }
    }
    if let Some(max) = constraint.max {
        if file_size > max {
            return false;
        }
    }
    true
}
```

### Modified Constraint Matching

```rust
fn matches_modified_constraint(
    file_modified: u64,  // Unix timestamp
    constraint: &ModifiedConstraint,
) -> bool {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();

    let file_age_secs = now.saturating_sub(file_modified);

    if let Some(max_age) = constraint.max_age_secs {
        if file_age_secs > max_age {
            return false;
        }
    }
    if let Some(min_age) = constraint.min_age_secs {
        if file_age_secs < min_age {
            return false;
        }
    }
    true
}
```

---

## Glob Support (zlob)

### Feature Flag

```toml
# Cargo.toml
[features]
default = []
zlob = ["dep:zlob"]

[dependencies]
zlob = { workspace = true, optional = true }
```

### Glob Implementation

```rust
#[cfg(feature = "zlob")]
pub fn glob_matches(path: &str, pattern: &str) -> bool {
    use zlob::Pattern;

    // Compile pattern (cached in production)
    let Ok(pattern) = Pattern::new(pattern) else {
        return false;
    };

    // Match against path
    pattern.matches(path)
}

#[cfg(not(feature = "zlob"))]
pub fn glob_matches(path: &str, pattern: &str) -> bool {
    // Fallback: simple glob without ** support
    simple_glob(path, pattern)
}

fn simple_glob(path: &str, pattern: &str) -> bool {
    // Convert glob to regex-like matching
    let pattern = pattern.replace('*', ".*");
    let regex = format!("^{}$", pattern);

    // Simple regex match (no full regex engine)
    path.contains(&pattern.replace(".*", ""))
}
```

### Pattern Examples

```
**/*.rs         # Any .rs file anywhere
src/**/*.ts     # .ts files under src/
**/test_*.rs    # Test files in any directory
*.{ts,tsx}      # .ts or .tsx files
```

---

## Config Types

### File Search Config

```rust
pub struct FileSearchConfig;

impl ConstraintParser for FileSearchConfig {
    fn parse_constraint<'a>(&self, token: &'a str) -> Option<Constraint<'a>> {
        // File search supports all constraints
        parse_any_constraint(token)
    }
}
```

### Grep Config

```rust
pub struct GrepConfig;

impl ConstraintParser for GrepConfig {
    fn parse_constraint<'a>(&self, token: &'a str) -> Option<Constraint<'a>> {
        // Grep only supports path-based constraints
        // (no size/modified filters for content search)
        match token {
            t if t.starts_with("*.") => Some(Constraint::Extension(extract_extension(t)?)),
            t if t.contains("**") => Some(Constraint::Glob(t)),
            t if t.ends_with('/') => Some(Constraint::PathSegment(t)),
            t if t.contains('.') && !t.contains('/') => Some(Constraint::Filename(t)),
            _ => None,
        }
    }
}
```

---

## Usage Examples

### File Search

```rust
use fff_query_parser::{QueryParser, FileSearchConfig};

let parser = QueryParser::new(FileSearchConfig);

// Simple search
let query = parser.parse("main.rs");
// FuzzyQuery::Text("main.rs"), constraints: []

// With constraints
let query = parser.parse("git:modified src/**/*.rs !test/");
// constraints: [GitStatus(Modified), Glob("src/**/*.rs"), Not(PathSegment("test/"))]
// fuzzy_query: Text("")

// With location
let query = parser.parse("src/main.rs:10:5");
// constraints: [PathSegment("src/"), Filename("main.rs")]
// location: Some(Location { line: 10, column: Some(5) })
```

### Grep Search

```rust
use fff_query_parser::{QueryParser, GrepConfig};

let parser = QueryParser::new(GrepConfig);

// Grep with path constraints
let query = parser.parse("*.rs src/ TODO");
// constraints: [Extension("rs"), PathSegment("src/")]
// fuzzy_query: Text("TODO")
```

### Multi-Part Search

```rust
let query = parser.parse("user controller auth");
// fuzzy_query: Parts(["user", "controller", "auth"])
// All three parts must match
```

---

## Performance

### Parsing Benchmarks

```rust
#[cfg(test)]
mod benches {
    use criterion::*;

    fn bench_parse_simple(c: &mut Criterion) {
        let parser = QueryParser::new(FileSearchConfig);
        c.bench_function("parse_simple", |b| {
            b.iter(|| parser.parse("main.rs"))
        });
    }

    fn bench_parse_constraints(c: &mut Criterion) {
        let parser = QueryParser::new(FileSearchConfig);
        c.bench_function("parse_constraints", |b| {
            b.iter(|| parser.parse("git:modified src/**/*.rs !test/"))
        });
    }

    criterion_group!(benches, bench_parse_simple, bench_parse_constraints);
    criterion_main!(benches);
}
```

### Optimization Techniques

1. **Zero-copy parsing**: Uses `&str` slices instead of `String`
2. **Pre-allocated vectors**: `Vec::with_capacity()` for known sizes
3. **Early exit**: Returns `None` quickly for non-constraints
4. **Feature flags**: Optional zlob for minimal binary size

---

## Testing

### Unit Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple() {
        let parser = QueryParser::new(FileSearchConfig);
        let query = parser.parse("main.rs");

        assert!(query.constraints.is_empty());
        assert!(matches!(query.fuzzy_query, FuzzyQuery::Text("main.rs")));
    }

    #[test]
    fn test_parse_git_status() {
        let parser = QueryParser::new(FileSearchConfig);
        let query = parser.parse("git:modified git:staged");

        assert_eq!(query.constraints.len(), 2);
        assert!(matches!(
            query.constraints[0],
            Constraint::GitStatus(GitStatusFilter::Modified)
        ));
    }

    #[test]
    fn test_parse_negation() {
        let parser = QueryParser::new(FileSearchConfig);
        let query = parser.parse("!test/ !*.spec.ts");

        assert_eq!(query.constraints.len(), 2);
        // Check constraints are negated
    }

    #[test]
    fn test_parse_location() {
        let parser = QueryParser::new(FileSearchConfig);
        let query = parser.parse("main.rs:10:5");

        assert!(query.location.is_some());
        assert_eq!(query.location.unwrap().line, 10);
    }

    #[test]
    fn test_parse_size() {
        let parser = QueryParser::new(FileSearchConfig);
        let query = parser.parse("size:>1mb,<10mb");

        assert_eq!(query.constraints.len(), 1);
        if let Constraint::Size(size) = &query.constraints[0] {
            assert_eq!(size.min, Some(1024 * 1024));
            assert_eq!(size.max, Some(10 * 1024 * 1024));
        } else {
            panic!("Expected Size constraint");
        }
    }

    #[test]
    fn test_parse_modified() {
        let parser = QueryParser::new(FileSearchConfig);
        let query = parser.parse("modified:<1h");

        assert_eq!(query.constraints.len(), 1);
        if let Constraint::Modified(modified) = &query.constraints[0] {
            assert_eq!(modified.max_age_secs, Some(3600));
        } else {
            panic!("Expected Modified constraint");
        }
    }

    #[test]
    fn test_parse_multi_part() {
        let parser = QueryParser::new(FileSearchConfig);
        let query = parser.parse("user controller auth");

        assert!(matches!(
            query.fuzzy_query,
            FuzzyQuery::Parts(parts) if parts.len() == 3
        ));
    }
}
```

### Integration Tests

```rust
#[test]
fn test_full_query_flow() {
    let parser = QueryParser::new(FileSearchConfig);
    let query = parser.parse("git:modified src/**/*.rs !test/ main");

    // Apply constraints to mock files
    let files = vec![
        MockFile::new("src/main.rs", Some(git2::Status::WT_MODIFIED)),
        MockFile::new("test/main.rs", Some(git2::Status::WT_MODIFIED)),
        MockFile::new("src/lib.rs", None),
    ];

    let filtered = apply_constraints(&files, &query.constraints);

    // Should only match src/main.rs
    assert_eq!(filtered.len(), 1);
    assert_eq!(filtered[0].path, "src/main.rs");
}
```

---

## Error Handling

### Parse Errors

```rust
/// Parse with error reporting
pub fn parse_with_errors(query: &str) -> Result<FFFQuery, ParseError> {
    // Validate syntax
    if let Some(invalid) = find_invalid_syntax(query) {
        return Err(ParseError::InvalidSyntax {
            position: invalid.position,
            token: invalid.token.to_string(),
            message: invalid.message,
        });
    }

    Ok(parse(query))
}

#[derive(Debug)]
pub enum ParseError {
    InvalidSyntax {
        position: usize,
        token: String,
        message: String,
    },
    InvalidSizeFormat(String),
    InvalidTimeFormat(String),
    InvalidGitStatus(String),
}
```

---

## Next Steps

- See [01-fff-exploration.md](./01-fff-exploration.md) for full architecture
- See [grep-engine-deep-dive.md](./grep-engine-deep-dive.md) for grep implementation
- See [rust-revision.md](./rust-revision.md) for Rust patterns
