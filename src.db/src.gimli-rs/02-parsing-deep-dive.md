---
title: "Gimli-rs Parsing Deep Dive"
subtitle: "Zero-copy parsing, lazy evaluation, and iteration patterns"
location: /home/darkvoid/Boxxed/@dev/repo-expolorations/src.db/src.gimli-rs
related: exploration.md, 00-zero-to-debug-engineer.md, 01-storage-engine-deep-dive.md
---

# 02 - Parsing Deep Dive: Gimli-rs

## Overview

This document covers Gimli's parsing architecture - zero-copy design, lazy evaluation patterns, Reader trait abstraction, and efficient iteration over DWARF data.

## Part 1: Reader Trait Architecture

### Abstraction Over Data Source

```
Gimli's Reader Trait:

Problem: Debug data can come from many sources:
- Mmap'd file (direct memory access)
- Vec<u8> (loaded into memory)
- &[u8] slice (borrowed data)
- Network stream (remote debugging)
- Compressed (needs decompression)

Solution: Reader trait abstracts over data source

Reader trait definition (simplified):
```rust
pub trait Reader: Clone {
    type Endian: Endianity;
    type Offset: Offset;
    type OffsetUsize: OffsetUsize;

    /// Get the endian-ness of the data
    fn endian(&self) -> Self::Endian;

    /// Read a single byte
    fn read_u8(&mut self) -> Result<u8>;

    /// Read multiple bytes into a slice
    fn read_slice(&mut self, buf: &mut [u8]) -> Result<()>;

    /// Skip bytes without reading
    fn skip(&mut self, len: usize) -> Result<()>;

    /// Read a ULEB128 encoded value
    fn read_uleb128(&mut self) -> Result<u64>;

    /// Read a SLEB128 encoded value
    fn read_sleb128(&mut self) -> Result<i64>;

    /// Read an address (4 or 8 bytes based on address size)
    fn read_address(&mut self, size: u8) -> Result<u64>;

    /// Read a string (null-terminated)
    fn read_string(&mut self) -> Result<EndianStr<Self>>;

    /// Read a fixed-size buffer
    fn read_bytes(&mut self, len: usize) -> Result<EndianBuf<Self>>;

    /// Find a byte offset
    fn find(&self, byte: u8) -> Option<Self::Offset>;

    /// Convert offset to usize
    fn offset_from(&self, other: &Self) -> Result<usize>;
}
```

Implementations:
```rust
// Borrowed slice (most common, zero-copy)
impl<'a> Reader for EndianSlice<'a, Endian> {
    // Direct memory access, no allocation
}

// Owned buffer (when data must be copied)
impl<Endian: Endianity> Reader for EndianVec<Endian> {
    // Heap-allocated buffer
}

// Array-backed reader (for small fixed-size data)
impl<'a, const N: usize> Reader for ArrayWrapper<'a, [u8; N]> {
    // Stack-allocated, no heap
}
```

Usage example:
```rust
// Zero-copy parsing from mmap
let mmap = unsafe { MmapOptions::new().map(&file).unwrap() };
let debug_info = DebugInfo::new(
    EndianSlice::new(&mmap[info_range], LittleEndian)
);

// Or from owned data (if mmap not possible)
let data = std::fs::read("program").unwrap();
let debug_info = DebugInfo::new(
    EndianVec::new(data, LittleEndian)
);

// Same API, different storage!
```
```

### Endianity Abstraction

```
Gimli handles both little-endian and big-endian:

Endianity trait:
```rust
pub trait Endianity: Clone + Copy + Debug + Eq + PartialEq {
    /// True if this is little-endian
    fn is_little_endian() -> bool;

    /// Read a 16-bit value
    fn read_u16(data: &[u8]) -> u16;

    /// Read a 32-bit value
    fn read_u32(data: &[u8]) -> u32;

    /// Read a 64-bit value
    fn read_u64(data: &[u8]) -> u64;

    /// Read a signed 32-bit value
    fn read_i32(data: &[u8]) -> i32;

    /// Read a signed 64-bit value
    fn read_i64(data: &[u8]) -> i64;
}

// Pre-defined implementations
pub struct LittleEndian;
pub struct BigEndian;
```

DW_TAG_compile_unit {
            unit_header: Some(header),
            debug_abbrev: Some(abbrevs),
        }),
        None => Err("No compilation units found"),
    }
}
```
```

## Part 3: DIE Iteration

### Entry Iterator

```
Iterating over Debug Information Entries:

Basic iteration:
```rust
let mut units = debug_info.units();
while let Ok(Some(header)) = units.next() {
    let abbrevs = header.abbreviations(&debug_abbrev)?;
    let mut entries = header.entries(&abbrevs);

    while let Ok(Some((depth, entry))) = entries.next() {
        println!("Tag: {:?}", entry.tag());
        println!("Depth: {}", depth);

        // Read attributes
        let mut attrs = entry.attrs();
        while let Ok(Some(attr)) = attrs.next() {
            println!("  Attribute: {:?}", attr.name());
            println!("  Value: {:?}", attr.value());
        }
    }
}
```

Tree iteration (handles parent/child relationships):
```rust
let mut tree = header.entries_tree(&abbrevs, None)?;
let root = tree.root()?;

// Recursive traversal
fn walk_die(die: Die, depth: usize) {
    let indent = "  ".repeat(depth);
    println!("{}Tag: {:?}", indent, die.tag());

    // Process attributes
    let mut attrs = die.attrs();
    while let Ok(Some(attr)) = attrs.next() {
        println!("{}  {:?} = {:?}", indent, attr.name(), attr.value());
    }

    // Recurse into children
    let mut children = die.children();
    while let Ok(Some(child)) = children.next() {
        walk_die(child, depth + 1);
    }
}

walk_die(root, 0);
```

Filtered iteration (skip uninteresting entries):
```rust
// Find all subprograms (functions)
let mut units = debug_info.units();
while let Ok(Some(header)) = units.next() {
    let abbrevs = header.abbreviations(&debug_abbrev)?;
    let mut entries = header.entries(&abbrevs);

    while let Ok(Some((depth, entry))) = entries.next() {
        if entry.tag() == DW_TAG_subprogram {
            // Found a function!
            let name = entry.attr(DW_AT_name)?
                .and_then(|attr| attr.string_value(&debug_str))
                .map(|s| String::from_utf8_lossy(&s).to_string());

            println!("Function: {:?}", name);
        }
    }
}
```
```

### Attribute Value Parsing

```
Reading DIE attributes:

Attribute values are strongly typed:
```rust
enum AttributeValue<R: Reader> {
    Addr(u64),
    Block(EndianBuf<R>),
    Constant(i64),
    ConstantClass(EndianBuf<R>),
    ConstantFlags(u64),
    ConstantUint(u64),
    DebugStrRef(EndianBuf<R>),
    Exprloc(Expr<R>),
    Flag(bool),
    LinePtr(EndianBuf<R>),
    LocListRef(u64),
    MacinfoRef(EndianBuf<R>),
    RangeListRef(u64),
    Sdata(i64),
    SecOffset(u64),
    String(EndianStr<R>),
    UnitRef(DebugInfoOffset),
    DebugInfoRef(EndianBuf<R>),
}
```

Reading specific attribute types:
```rust
// Read a string attribute
fn read_string<R: Reader>(
    entry: &DebugInformationEntry<R>,
    debug_str: &DebugStr<R>,
) -> Result<String> {
    match entry.attr_value(DW_AT_name)? {
        Some(AttributeValue::String(s)) => {
            Ok(String::from_utf8_lossy(&s).to_string())
        }
        Some(AttributeValue::DebugStrRef(offset)) => {
            let string = debug_str.get_str(offset)?;
            Ok(String::from_utf8_lossy(&string).to_string())
        }
        _ => Err("Expected string attribute"),
    }
}

// Read an address attribute
fn read_address<R: Reader>(
    entry: &DebugInformationEntry<R>,
) -> Result<u64> {
    match entry.attr_value(DW_AT_low_pc)? {
        Some(AttributeValue::Addr(addr)) => Ok(addr),
        _ => Err("Expected address attribute"),
    }
}

// Read a reference to another DIE
fn read_type_ref<R: Reader>(
    entry: &DebugInformationEntry<R>,
    unit: &CompilationUnitHeader<R>,
) -> Result<DebugInformationEntry<R>> {
    match entry.attr_value(DW_AT_type)? {
        Some(AttributeValue::UnitRef(offset)) => {
            // Get referenced entry
            let mut entries = unit.entries_at_offset(offset)?;
            entries.next()?; // Skip to offset
            Ok(entries.current().clone())
        }
        _ => Err("Expected reference attribute"),
    }
}

// Read a location expression
fn read_location<R: Reader>(
    entry: &DebugInformationEntry<R>,
) -> Result<Expr<R>> {
    match entry.attr_value(DW_AT_location)? {
        Some(AttributeValue::Exprloc(expr)) => Ok(expr),
        Some(AttributeValue::LocListRef(offset)) => {
            // Location list (complex, multiple locations)
            todo!("Handle location lists")
        }
        _ => Err("Expected location attribute"),
    }
}
```
```

## Part 4: Line Number Parsing

### Line Program Execution

```rust
use gimli::{DebugLine, DebugLineStr, EndianSlice, LittleEndian, LineRow};

/// Execute line number program to build line table
pub fn build_line_table<'input>(
    debug_line: &DebugLine<EndianSlice<'input, LittleEndian>>,
    debug_str: &DebugLineStr<EndianSlice<'input, LittleEndian>>,
) -> Result<LineTable> {
    let mut table = LineTable::new();

    // Iterate over all line number programs
    let mut headers = debug_line.headers();
    while let Some(header) = headers.next()? {
        // Get the program for this compilation unit
        let program = header.program();

        // Execute line number program
        let mut rows = program.rows();
        while let Some((_, row)) = rows.next_row()? {
            if row.end_sequence() {
                continue; // End of sequence, not a real row
            }

            // Get file information
            let file = row.file(header);
            let path = if let Some(dir) = file.directory(header) {
                let dir_str = dir.string_value(debug_str)?;
                let file_str = file.path_name();
                format!("{}/{}", dir_str, file_str)
            } else {
                String::from_utf8_lossy(file.path_name()).to_string()
            };

            // Add row to table
            table.rows.push(LineTableRow {
                address: row.address(),
                file: path,
                line: row.line().unwrap_or(0),
                column: row.column().unwrap_or(Column::LeftEdge),
                is_statement: row.is_stmt(),
                basic_block: row.basic_block(),
                end_sequence: row.end_sequence(),
            });
        }
    }

    Ok(table)
}

/// Line table for address-to-line lookup
pub struct LineTable {
    rows: Vec<LineTableRow>,
}

impl LineTable {
    /// Find line number for an address
    pub fn lookup(&self, address: u64) -> Option<&LineTableRow> {
        // Binary search for address
        let idx = self.rows
            .binary_search_by_key(&address, |r| r.address)
            .ok()?;

        Some(&self.rows[idx])
    }

    /// Find all addresses for a line
    pub fn find_line(&self, file: &str, line: u64) -> Vec<u64> {
        self.rows
            .iter()
            .filter(|r| r.file == file && r.line == line)
            .map(|r| r.address)
            .collect()
    }
}
```

### Line Number Optimization

```
Special Opcodes - Compact encoding:

Most lines advance by 1, addresses advance steadily
Special opcodes encode both in a single byte!

Special opcode formula:
```
opcode = (line_delta - line_base) + (address_delta * line_range) + opcode_base
```

Example: line_base=-5, line_range=14, opcode_base=13
- Advance line by +1, address by +1: opcode = (1 - (-5)) + (1 * 14) + 13 = 33
- Advance line by +2, address by +0: opcode = (2 - (-5)) + (0 * 14) + 13 = 20
- Advance line by -3, address by +2: opcode = (-3 - (-5)) + (2 * 14) + 13 = 43

Gimli decodes special opcodes:
```rust
fn decode_special_opcode(
    opcode: u8,
    line_base: i8,
    line_range: u8,
    opcode_base: u8,
) -> (i8, u8) {
    let adjusted = opcode as i16 - opcode_base as i16;
    let line_delta = line_base as i16 + (adjusted % line_range as i16);
    let address_delta = adjusted / line_range as i16;

    (line_delta as i8, address_delta as u8)
}
```

This is why DWARF line tables are compact:
- Common case (line+1, addr+1): 1 byte
- Standard opcodes (DW_LNS_copy, etc.): 1-2 bytes
- Only large jumps need extended opcodes
```

## Part 5: Error Handling

### Gimli Error Types

```rust
use gimli::read::{Error, Result};

/// Gimli's error type
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    /// An I/O error occurred
    Io,

    /// A DWARF entry was not found
    EntryNotFound,

    /// A DWARF unit was not found
    UnitNotFound,

    /// An abbreviation was not found
    AbbreviationNotFound,

    /// A string was not found
    StringNotFound,

    /// A line number row was not found
    LineTableRowNotFound,

    /// The DWARF data is too large
    TooBig,

    /// An overflow occurred during parsing
    Overflow,

    /// An unexpected EOF was encountered
    UnexpectedEof,

    /// An unknown DWARF tag was encountered
    UnknownTag(u16),

    /// An unknown DWARF attribute was encountered
    UnknownAttribute(u16),

    /// An unknown DWARF form was encountered
    UnknownForm(u16),

    /// An unknown DWARF opcode was encountered
    UnknownOpcode(u8),

    /// The DWARF version is unsupported
    UnsupportedVersion(u16),

    /// The DWARF format is unsupported
    UnsupportedFormat,

    /// The encoding is unsupported
    UnsupportedEncoding,

    /// The operation is unsupported
    UnsupportedOperation,

    /// The address size is unsupported
    UnsupportedAddressSize(u8),

    /// The byte order is unsupported
    UnsupportedEndianity(u8),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::Io => write!(f, "I/O error"),
            Error::EntryNotFound => write!(f, "DWARF entry not found"),
            Error::Overflow => write!(f, "Integer overflow"),
            Error::UnexpectedEof => write!(f, "Unexpected end of data"),
            Error::UnknownTag(tag) => write!(f, "Unknown DWARF tag: {}", tag),
            // ... etc
        }
    }
}

impl std::error::Error for Error {}
```

Error handling patterns:
```rust
// Convert gimli errors to your own type
#[derive(Debug)]
enum MyError {
    Gimli(gimli::read::Error),
    Utf8(std::str::Utf8Error),
    Io(std::io::Error),
}

impl From<gimli::read::Error> for MyError {
    fn from(err: gimli::read::Error) -> Self {
        MyError::Gimli(err)
    }
}

// Graceful error recovery
fn parse_with_fallback(data: &[u8]) -> Result<DebugInfo, MyError> {
    // Try DWARF 5 first
    match DebugInfo::new(data, LittleEndian) {
        Ok(info) => Ok(info),
        Err(gimli::read::Error::UnsupportedVersion(_)) => {
            // Fall back to DWARF 4
            eprintln!("DWARF 5 not supported, trying DWARF 4...");
            DebugInfo::new(data, BigEndian).map_err(MyError::from)
        }
        Err(e) => Err(MyError::from(e)),
    }
}
```

---

*This document is part of the Gimli-rs exploration series. See [exploration.md](./exploration.md) for the complete index.*
