# Aipack -- Support Utilities

Aipack includes a collection of support modules for file operations, document processing, time handling, and general utilities.

Source: `aipack/src/support/` — all support modules

## File Operations

```rust
// support/files.rs
pub fn current_dir() -> Result<SPath> {
    Ok(SPath::new(std::env::current_dir()?.to_str().ok_or("...")?))
}

pub fn home_dir() -> SPath {
    dirs::home_dir()
        .map(SPath::from)
        .unwrap_or_else(|| SPath::new("/"))  // Fallback for containerized environments
}

pub fn safer_trash_dir(path: &SPath, check: Option<DeleteCheck>) -> Result<bool> {
    // Move to trash instead of deleting
    // If DeleteCheck is provided, verify path is within expected directory
}

pub fn safer_trash_file(path: &SPath, check: Option<DeleteCheck>) -> Result<bool> {
    // Same for files
}
```

### DeleteCheck Safety

```rust
pub enum DeleteCheck {
    CONTAINS_AIPACK,       // Path must contain ".aipack" component
    CONTAINS_AIPACK_BASE,  // Path must contain ".aipack-base" component
}

fn check_delete_safety(path: &SPath, check: DeleteCheck) -> Result<()> {
    let s = path.as_str();
    match check {
        CONTAINS_AIPACK if !s.contains(".aipack") => Err("..."),
        CONTAINS_AIPACK_BASE if !s.contains(".aipack-base") => Err("..."),
        _ => Ok(()),
    }
}
```

The `DeleteCheck` prevents accidental trashing of files outside the aipack directory tree. If a path doesn't contain the expected marker, the operation is refused.

## ZIP Operations

```rust
// support/zip.rs
pub fn zip_dir(dir: &SPath, dest: &SPath) -> Result<()> {
    // Walk directory, add to zip archive
    // Uses zip crate with filetime preservation
}

pub fn unzip_file(zip_path: &SPath, dest_dir: &SPath) -> Result<()> {
    // Extract zip, creating destination directories as needed
}
```

## Time Utilities

```rust
// support/time.rs
pub fn now_micro() -> i64 {
    // Current time as microseconds since epoch
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_micros() as i64
}

pub fn format_duration_us(micros: i64) -> String {
    if micros >= 1_000_000 {
        format!("{:.2}s", micros as f64 / 1_000_000.0)
    } else if micros >= 1_000 {
        format!("{:.0}ms", micros as f64 / 1_000.0)
    } else {
        format!("{micros}μs")
    }
}
```

All timestamps in aipack are stored as microseconds since Unix epoch (`EpochUs`). This provides sufficient precision for sub-millisecond timing while fitting in a 64-bit integer.

## Markdown Block Iterator

```rust
// support/md_block_iter.rs
pub struct MdBlockIter<'a> {
    content: &'a str,
    pos: usize,
}

impl<'a> Iterator for MdBlockIter<'a> {
    type Item = MdBlock<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        // Find next markdown block (code fence, heading, etc.)
        // Parse block type and content
    }
}

pub struct MdBlock<'a> {
    pub kind: BlockKind,  // CodeFence, Heading, Text, etc.
    pub content: &'a str,
    pub language: Option<&'a str>,  // For code fences
}
```

Used by the AgentDoc parser to identify code blocks within `.aip` files.

## Path Utilities (AsStrsExt)

```rust
// support/mod.rs
pub trait AsStrsExt {
    fn x_as_strs(&self) -> Vec<&str>;
}

impl AsStrsExt for Vec<SPath> {
    fn x_as_strs(&self) -> Vec<&str> {
        self.iter().map(|p| p.as_str()).collect()
    }
}
```

Convenience trait for converting `Vec<SPath>` to `Vec<&str>` for functions that need string paths.

## AsStrsExt

Used in initialization code:
```rust
// From init_base.rs
let custom_pack_file_paths = init_assets::extract_base_pack_custom_file_paths()?;
assets::update_files("base", &base_dir, &custom_pack_file_paths.x_as_strs(), force_update).await?;
```

See [Initialization](16-initialization.md) for asset extraction.
See [Agent System](02-agent-system.md) for markdown block parsing.
