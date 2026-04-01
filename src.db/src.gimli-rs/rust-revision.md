---
title: "Gimli-rs Rust Revision"
subtitle: "Already Rust! Design patterns and usage guide"
location: /home/darkvoid/Boxxed/@dev/repo-expolorations/src.db/src.gimli-rs
related: exploration.md
---

# Rust Revision: Gimli-rs

## Overview

Gimli is already written in safe Rust! This document covers Gimli's design patterns, how to use it effectively, and how related crates (addr2line, backtrace) build on top of it.

## Part 1: Gimli Design Patterns

### Zero-Copy Architecture

```rust
/// Gimli's core design: borrow everything
///
/// All major structs have lifetime parameters:
/// - DebugInfo<'input>
/// - DebugLine<'input>
/// - DebugAbbrev<'input>
///
/// The 'input lifetime is the borrowed data source

use gimli::{DebugInfo, DebugAbbrev, DebugLine, EndianSlice, LittleEndian};

/// Example: Stack-allocated parser
/// No heap allocation during parsing!
pub struct DebugParser<'input> {
    pub debug_info: DebugInfo<EndianSlice<'input, LittleEndian>>,
    pub debug_abbrev: DebugAbbrev<EndianSlice<'input, LittleEndian>>,
    pub debug_line: DebugLine<EndianSlice<'input, LittleEndian>>,
    pub debug_str: gimli::DebugStr<EndianSlice<'input, LittleEndian>>,
}

impl<'input> DebugParser<'input> {
    /// Create from mmap'd data - zero copies!
    pub fn new(mmap: &'input [u8]) -> Self {
        // Find section ranges (using object crate)
        let file = object::File::parse(mmap).unwrap();

        let info_range = file.section_by_name(".debug_info").unwrap().offset()..;
        let abbrev_range = file.section_by_name(".debug_abbrev").unwrap().offset()..;
        // etc...

        Self {
            debug_info: DebugInfo::new(
                EndianSlice::new(&mmap[info_range], LittleEndian)
            ),
            debug_abbrev: DebugAbbrev::new(
                EndianSlice::new(&mmap[abbrev_range], LittleEndian)
            ),
            // etc...
        }
    }
}
```

### Reader Trait Pattern

```rust
/// Abstract over data source with Reader trait
///
/// This allows the same parser to work with:
/// - Borrowed slices (&[u8])
/// - Owned vectors (Vec<u8>)
/// - Memory-mapped files
/// - Network streams

use gimli::read::{Reader, Endianity, LittleEndian};

/// Generic function works with any Reader
pub fn parse_debug_info<R: Reader<Endian = LittleEndian>>(
    data: R,
) -> gimli::read::Result<DebugInfo<R>> {
    DebugInfo::new(data, LittleEndian)
}

/// Use with borrowed data (zero-copy)
pub fn from_slice(data: &[u8]) -> DebugInfo<EndianSlice<LittleEndian>> {
    parse_debug_info(EndianSlice::new(data, LittleEndian))
}

/// Use with owned data (when necessary)
pub fn from_vec(data: Vec<u8>) -> DebugInfo<EndianVec<LittleEndian>> {
    parse_debug_info(EndianVec::new(data, LittleEndian))
}
```

## Part 2: addr2line Patterns

### Building on Gimli

```rust
/// addr2line builds on Gimli for symbol resolution
///
/// Gimli provides: Raw DWARF parsing
/// addr2line provides: High-level API

use addr2line::{Context, LookupResult, FallibleIterator};
use object::Object;

/// Simple addr2line usage
pub fn symbolicate(path: &str, address: u64) -> Result<(), Box<dyn Error>> {
    // Load file
    let data = std::fs::read(path)?;
    let file = object::File::parse(data.as_slice())?;

    // Create context (parses all debug info)
    let ctx = Context::new(&file)?;

    // Lookup address
    let frames = ctx.find_frames(address)?;

    // Iterate stack frames
    let mut frames = frames?;
    while let Some(frame) = frames.next()? {
        if let Some(location) = frame.location {
            println!(
                "{}:{} ({})",
                location.file.unwrap_or("unknown"),
                location.line.unwrap_or(0),
                frame.function.as_ref().map(|f| f.demangle()).unwrap_or("?")
            );
        }
    }

    Ok(())
}

/// Lazy frame lookup (only parse what's needed)
pub fn lazy_symbolicate<R: object::read::Object>(
    file: &R,
    address: u64,
) -> LookupResult<Option<Frame>> {
    let ctx = Context::new(file)?;

    // find_frames returns LookupResult for lazy evaluation
    ctx.find_frames(address)
}
```

### Custom Context

```rust
/// Build custom context for batch lookups
pub struct Symbolicator {
    context: addr2line::Context<Vec<u8>>,
}

impl Symbolicator {
    pub fn new(path: &str) -> Result<Self, Box<dyn Error>> {
        let data = std::fs::read(path)?;
        let file = object::File::parse(data.as_slice())?;
        let context = addr2line::Context::new(&file)?;

        Ok(Self { context })
    }

    /// Batch symbolicate multiple addresses
    pub fn symbolicate_batch(&self, addresses: &[u64]) -> Vec<Option<Frame>> {
        addresses.iter()
            .map(|&addr| {
                self.context.find_frames(addr)
                    .ok()
                    .and_then(|mut frames| frames.next().ok().flatten())
            })
            .collect()
    }
}
```

## Part 3: Practical Examples

### Stack Trace Formatter

```rust
/// Format backtrace addresses to source locations
use backtrace::Backtrace;
use addr2line::Context;

pub struct BacktraceFormatter {
    context: Option<Context<Vec<u8>>>,
}

impl BacktraceFormatter {
    pub fn new() -> Self {
        // Try to load debug info for current executable
        let context = std::env::current_exe()
            .ok()
            .and_then(|path| std::fs::read(&path).ok())
            .and_then(|data| {
                object::File::parse(data.as_slice())
                    .ok()
                    .and_then(|file| Context::new(&file).ok())
            });

        Self { context }
    }

    pub fn format(&self, backtrace: &Backtrace) -> String {
        let mut output = String::new();

        for (i, frame) in backtrace.frames().iter().enumerate() {
            output.push_str(&format!("Frame {}: ", i));

            // Try to symbolicate
            if let Some(ctx) = &self.context {
                let ip = frame.ip() as u64;
                if let Ok(mut frames) = ctx.find_frames(ip) {
                    if let Ok(Some(frame)) = frames.next() {
                        if let Some(func) = &frame.function {
                            output.push_str(&func.demangle());
                        }
                        if let Some(loc) = frame.location {
                            if let Some(file) = loc.file {
                                output.push_str(&format!(" at {}:{}", file, loc.line.unwrap_or(0)));
                            }
                        }
                    }
                }
            }

            // Fallback to raw address
            if let Some(symbol) = frame.symbols().first() {
                if let Some(name) = symbol.name() {
                    output.push_str(&format!(" ({})", name));
                }
            }

            output.push('\n');
        }

        output
    }
}
```

### Coverage Tool

```rust
/// Build coverage data from DWARF line info
use gimli::*;
use std::collections::BTreeMap;

pub struct CoverageMap {
    /// file -> line -> execution count
    data: BTreeMap<String, BTreeMap<u64, u64>>,
}

impl CoverageMap {
    pub fn from_dwarf(path: &str) -> Result<Self, Box<dyn Error>> {
        let data = std::fs::read(path)?;
        let file = object::File::parse(data.as_slice())?;

        // Extract sections
        let debug_info_data = file.section_by_name(".debug_info").unwrap().data()?;
        let debug_abbrev_data = file.section_by_name(".debug_abbrev").unwrap().data()?;
        let debug_line_data = file.section_by_name(".debug_line").unwrap().data()?;

        let endian = LittleEndian;
        let debug_info = DebugInfo::new(debug_info_data, endian);
        let debug_abbrev = DebugAbbrev::new(debug_abbrev_data, endian);
        let debug_line = DebugLine::new(debug_line_data, endian);

        let mut coverage = CoverageMap::new();

        // Process line tables
        let mut headers = debug_line.headers();
        while let Some(header) = headers.next()? {
            let program = header.program();
            let mut rows = program.rows();

            while let Some((_, row)) = rows.next_row()? {
                if row.end_sequence() {
                    continue;
                }

                let file = row.file(header);
                let path = file.path_name();
                let line = row.line().unwrap_or(0);

                coverage.record(path, line);
            }
        }

        Ok(coverage)
    }

    fn record(&mut self, file: &[u8], line: u64) {
        let path = String::from_utf8_lossy(file).to_string();
        *self.data
            .entry(path)
            .or_default()
            .entry(line)
            .or_insert(0) += 1;
    }

    pub fn merge(&mut self, execution_data: &[(u64, u64)]) {
        // Merge runtime execution data
        for &(addr, count) in execution_data {
            // Convert address to file:line using line table
            // ... implementation depends on runtime
        }
    }
}
```

---

*This document is part of the Gimli-rs exploration series. See [exploration.md](./exploration.md) for the complete index.*
