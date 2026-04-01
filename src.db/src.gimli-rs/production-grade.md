---
title: "Gimli-rs Production Usage"
subtitle: "Patterns for debug tools, symbolication, and analysis"
location: /home/darkvoid/Boxxed/@dev/repo-expolorations/src.db/src.gimli-rs
related: exploration.md, rust-revision.md
---

# Production-Grade Gimli-rs

## Overview

This document covers production usage patterns for Gimli - building debug tools, stack symbolication, coverage analysis, and performance optimization.

## Part 1: Building Debug Tools

### addr2line Clone

```rust
/// Minimal addr2line implementation using Gimli
///
/// Usage: my-addr2line -e program 0x1234 0x5678

use gimli::*;
use object::{Object, ObjectSection};
use std::collections::HashMap;
use std::path::Path;

pub struct Addr2Line {
    context: Context<Vec<u8>>,
}

struct Context<R: Reader> {
    debug_info: DebugInfo<R>,
    debug_abbrev: DebugAbbrev<R>,
    debug_line: DebugLine<R>,
    debug_str: DebugStr<R>,
    units: Vec<Unit<R>>,
}

struct Unit<R: Reader> {
    dw_unit: gimli::CompilationUnitHeader<R>,
    abbrevs: gimli::Abbreviations,
    lines: Option<LineTable<R>>,
    functions: Functions,
}

impl Addr2Line {
    pub fn new<P: AsRef<Path>>(path: P) -> Result<Self, Box<dyn Error>> {
        let data = std::fs::read(path)?;
        let file = object::File::parse(data.as_slice())?;

        // Extract DWARF sections
        let endian = gimli::RunTimeEndian::Little;

        let debug_info_data = file
            .section_by_name(".debug_info")
            .ok_or("Missing .debug_info")?
            .data()?;

        let debug_abbrev_data = file
            .section_by_name(".debug_abbrev")
            .ok_or("Missing .debug_abbrev")?
            .data()?;

        let debug_line_data = file
            .section_by_name(".debug_line")
            .ok_or("Missing .debug_line")?
            .data()?;

        let debug_str_data = file
            .section_by_name(".debug_str")
            .ok_or("Missing .debug_str")?
            .data()?;

        // Create parsers
        let debug_info = DebugInfo::new(debug_info_data, endian);
        let debug_abbrev = DebugAbbrev::new(debug_abbrev_data, endian);
        let debug_line = DebugLine::new(debug_line_data, endian);
        let debug_str = DebugStr::new(debug_str_data, endian);

        // Build context
        let mut units = Vec::new();

        let mut headers = debug_info.units();
        while let Some(header) = headers.next()? {
            let abbrevs = header.abbreviations(&debug_abbrev)?;

            // Parse line table
            let lines = if let Some(header) = debug_line.header(header.line_program_offset())? {
                let program = header.program();
                let mut table = LineTable::new();

                let mut rows = program.rows();
                while let Some((_, row)) = rows.next_row()? {
                    if !row.end_sequence() {
                        if let Some(line) = row.line() {
                            let file = row.file(&header);
                            table.rows.push(LineRow {
                                address: row.address(),
                                line: line.get(),
                                file_index: file.file_index(),
                            });
                        }
                    }
                }

                Some(table)
            } else {
                None
            };

            // Parse function names
            let functions = parse_functions(&header, &abbrevs, &debug_str)?;

            units.push(Unit {
                dw_unit: header,
                abbrevs,
                lines,
                functions,
            });
        }

        let context = Context {
            debug_info,
            debug_abbrev,
            debug_line,
            debug_str,
            units,
        };

        Ok(Self { context })
    }

    pub fn lookup(&self, addr: u64) -> Option<Frame> {
        for unit in &self.context.units {
            // Check if address is in this unit
            if let Some(lines) = &unit.lines {
                if let Some(row) = lines.lookup(addr) {
                    // Found line number
                    let file = unit.get_file(row.file_index);
                    let func = unit.functions.get(addr);

                    return Some(Frame {
                        function: func.map(|f| f.name.clone()),
                        file,
                        line: row.line,
                    });
                }
            }
        }

        None
    }
}

pub struct Frame {
    pub function: Option<String>,
    pub file: Option<String>,
    pub line: Option<u64>,
}
```

### Stack Trace Symbolicator

```rust
/// Symbolicate raw stack traces
use backtrace::{Backtrace, BacktraceFrame, SymbolName};
use std::sync::OnceLock;

static SYMBOLICATOR: OnceLock<Addr2Line> = OnceLock::new();

pub fn init_symbolicator() {
    if let Ok(exe) = std::env::current_exe() {
        let _ = SYMBOLICATOR.set(Addr2Line::new(exe).unwrap());
    }
}

pub fn symbolicate_backtrace(bt: &Backtrace) -> Vec<SymbolicatedFrame> {
    let symbolicator = SYMBOLICATOR.get();

    bt.frames()
        .iter()
        .map(|frame| {
            let mut result = SymbolicatedFrame {
                address: frame.ip() as u64,
                function: None,
                file: None,
                line: None,
            };

            // Try addr2line first
            if let Some(sym) = symbolicator {
                if let Some(frame) = sym.lookup(frame.ip() as u64) {
                    result.function = frame.function;
                    result.file = frame.file;
                    result.line = frame.line;
                }
            }

            // Fallback to backtrace symbols
            if result.function.is_none() {
                if let Some(symbol) = frame.symbols().first() {
                    result.function = symbol.name().map(|n| n.to_string());
                }
            }

            result
        })
        .collect()
}

pub struct SymbolicatedFrame {
    pub address: u64,
    pub function: Option<String>,
    pub file: Option<String>,
    pub line: Option<u64>,
}

impl std::fmt::Display for SymbolicatedFrame {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Some(func) = &self.function {
            write!(f, "{}", func)?;
        } else {
            write!(f, "??")?;
        }

        if let (Some(file), Some(line)) = (&self.file, self.line) {
            write!(f, " ({}:{})", file, line)?;
        }

        Ok(())
    }
}
```

## Part 2: Performance Optimization

### Caching Strategies

```rust
/// Cache parsed debug info for faster startup
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

/// Global cache for parsed DWARF
static DWARF_CACHE: RwLock<HashMap<String, Arc<ParsedDwarf>>> = RwLock::new(HashMap::new());

pub struct ParsedDwarf {
    pub context: addr2line::Context<Vec<u8>>,
    pub load_time: Instant,
}

impl ParsedDwarf {
    /// Load or cached DWARF info
    pub fn load<P: AsRef<Path>>(path: P) -> Arc<Self> {
        let path_str = path.as_ref().to_string_lossy().to_string();

        // Check cache
        {
            let cache = DWARF_CACHE.read().unwrap();
            if let Some(cached) = cache.get(&path_str) {
                return Arc::clone(cached);
            }
        }

        // Load and parse
        let data = std::fs::read(path.as_ref()).unwrap();
        let file = object::File::parse(data.as_slice()).unwrap();
        let context = addr2line::Context::new(&file).unwrap();

        let parsed = Arc::new(ParsedDwarf {
            context,
            load_time: Instant::now(),
        });

        // Insert into cache
        let mut cache = DWARF_CACHE.write().unwrap();
        cache.insert(path_str, Arc::clone(&parsed));

        parsed
    }

    /// Clear old entries from cache
    pub fn cleanup(max_age: Duration) {
        let mut cache = DWARF_CACHE.write().unwrap();
        let now = Instant::now();

        cache.retain(|_, v| now.duration_since(v.load_time) < max_age);
    }
}
```

### Lazy Parsing

```rust
/// Parse DWARF lazily - only what's needed
pub struct LazyDwarf {
    data: Vec<u8>,
    parsed: OnceLock<ParsedDwarf>,
}

impl LazyDwarf {
    pub fn new(data: Vec<u8>) -> Self {
        Self {
            data,
            parsed: OnceLock::new(),
        }
    }

    fn parsed(&self) -> &ParsedDwarf {
        self.parsed.get_or_init(|| {
            let file = object::File::parse(self.data.as_slice()).unwrap();
            ParsedDwarf {
                context: addr2line::Context::new(&file).unwrap(),
            }
        })
    }

    pub fn lookup(&self, addr: u64) -> Option<Frame> {
        // Only parse when first lookup happens
        self.parsed().context.find_frames(addr).ok().flatten()
    }
}

/// Even lazier: parse per-unit
pub struct LazyUnitIterator<R: Reader> {
    headers: gimli::CompilationUnitHeaders<R>,
    debug_abbrev: DebugAbbrev<R>,
}

impl<R: Reader> Iterator for LazyUnitIterator<R> {
    type Item = gimli::read::Result<ParsedUnit<R>>;

    fn next(&mut self) -> Option<Self::Item> {
        let header = self.headers.next()?;
        let header = match header {
            Ok(h) => h,
            Err(e) => return Some(Err(e)),
        };

        // Only parse abbreviations when unit is accessed
        Some(Ok(ParsedUnit {
            header,
            debug_abbrev: self.debug_abbrev.clone(),
            abbrevs: OnceLock::new(),
        }))
    }
}
```

## Part 3: Testing

### Test Fixtures

```rust
/// Test DWARF parsing with known data
#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_binary() -> Vec<u8> {
        // Compile a small test program with debug info
        // Or use a pre-compiled test binary
        include_bytes!("../test-data/test.elf").to_vec()
    }

    #[test]
    fn test_parse_debug_info() {
        let data = create_test_binary();
        let file = object::File::parse(data.as_slice()).unwrap();

        let debug_info_data = file.section_by_name(".debug_info").unwrap().data().unwrap();
        let debug_info = DebugInfo::new(debug_info_data, gimli::RunTimeEndian::Little);

        let mut units = debug_info.units();
        assert!(units.next().is_some());
    }

    #[test]
    fn test_line_table() {
        let data = create_test_binary();
        let file = object::File::parse(data.as_slice()).unwrap();

        let debug_line_data = file.section_by_name(".debug_line").unwrap().data().unwrap();
        let debug_line = DebugLine::new(debug_line_data, gimli::RunTimeEndian::Little);

        let mut headers = debug_line.headers();
        let header = headers.next().unwrap().unwrap();
        let program = header.program();

        let mut rows = program.rows();
        let mut row_count = 0;
        while let Some((_, row)) = rows.next_row().unwrap() {
            if !row.end_sequence() {
                row_count += 1;
            }
        }

        assert!(row_count > 0);
    }

    #[test]
    fn test_addr2line() {
        let symbolicator = Addr2Line::new("test-data/test.elf").unwrap();

        // Known address from test binary
        let frame = symbolicator.lookup(0x1000).unwrap();
        assert_eq!(frame.line, Some(10));
        assert!(frame.function.unwrap().contains("main"));
    }
}
```

---

*This document is part of the Gimli-rs exploration series. See [exploration.md](./exploration.md) for the complete index.*
