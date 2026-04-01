---
title: "Gimli-rs Storage Engine Deep Dive"
subtitle: "DWARF section format, encoding, and compression"
location: /home/darkvoid/Boxxed/@dev/repo-expolorations/src.db/src.gimli-rs
related: exploration.md, 00-zero-to-debug-engineer.md
---

# 01 - Storage Engine Deep Dive: Gimli-rs

## Overview

This document covers DWARF storage format - section layout, encoding schemes, compression methods, and how Gimli handles different object file formats.

## Part 1: DWARF Section Layout

### ELF Object File Structure

```
ELF File with DWARF Debug Info:

┌─────────────────────────────────────────────────────────┐
│ ELF Header (64 bytes)                                   │
│ ┌─────────────────────────────────────────────────────┐ │
│ │ Magic: 0x7F ELF                                     │ │
│ │ Class: ELF64                                        │ │
│ │ Entry Point: 0x401000                               │ │
│ │ Section Header Offset: 0x10000                      │ │
│ └─────────────────────────────────────────────────────┘ │
├─────────────────────────────────────────────────────────┤
│ Program Headers                                         │
│ ┌─────────────────────────────────────────────────────┐ │
│ │ LOAD: 0x400000-0x401000 (code)                      │ │
│ │ LOAD: 0x600000-0x601000 (data)                      │ │
│ └─────────────────────────────────────────────────────┘ │
├─────────────────────────────────────────────────────────┤
│ Code Sections (.text)                                   │
│ ┌─────────────────────────────────────────────────────┐ │
│ │ main:    push rbp; mov rbp, rsp; ...                │ │
│ │ helper:  push rbp; mov rbp, rsp; ...                │ │
│ └─────────────────────────────────────────────────────┘ │
├─────────────────────────────────────────────────────────┤
│ DWARF Debug Sections                                    │
│ ┌─────────────────────────────────────────────────────┐ │
│ │ .debug_info      - Debug information entries        │ │
│ │ .debug_abbrev    - Abbreviation declarations        │ │
│ │ .debug_line      - Line number program              │ │
│ │ .debug_str       - String table                     │ │
│ │ .debug_aranges   - Address ranges (fast lookup)     │ │
│ │ .debug_frame     - Call frame information           │ │
│ │ .debug_loc       - Location expressions             │ │
│ │ .debug_rnglists  - Range lists (DWARF 5)            │ │
│ │ .debug_loclists  - Location lists (DWARF 5)         │ │
│ │ .debug_pubnames  - Public symbols (optional)        │ │
│ │ .debug_pubtypes  - Public types (optional)          │ │
│ │ .debug_names     - Name index (DWARF 5)             │ │
│ │ .debug_addr      - Address table (DWARF 5)          │ │
│ │ .debug_str_offsets - String offsets (DWARF 5)       │ │
│ └─────────────────────────────────────────────────────┘ │
├─────────────────────────────────────────────────────────┤
│ Section Headers                                         │
│ ┌─────────────────────────────────────────────────────┐ │
│ │ .debug_info: offset=0x5000, size=0x2000             │ │
│ │ .debug_abbrev: offset=0x7000, size=0x500            │ │
│ │ ...                                                 │ │
│ └─────────────────────────────────────────────────────┘ │
└─────────────────────────────────────────────────────────┘

Debug section sizes (typical Rust binary):
- .debug_info: 2-5 MB
- .debug_abbrev: 50-200 KB
- .debug_line: 500 KB - 2 MB
- .debug_str: 200-500 KB
- .debug_frame: 100-300 KB
- Total: 3-8 MB (for 1 MB binary)
```

### Section Header Format

```
Each DWARF section has a standard header:

.debug_info Section Header:
┌─────────────────────────────────────────────────────────┐
│ Unit Length: u32 (or u64 for DWARF64)                   │
│   - Total bytes in this CU (including header)           │
│   - 0xFFFFFFF0 indicates DWARF64 format                 │
├─────────────────────────────────────────────────────────┤
│ Version: u16                                            │
│   - DWARF version (2, 3, 4, or 5)                       │
├─────────────────────────────────────────────────────────┤
│ Debug Abbrev Offset: u32                                │
│   - Byte offset into .debug_abbrev section              │
│   - Where this CU's abbreviations start                 │
├─────────────────────────────────────────────────────────┤
│ Address Size: u8                                        │
│   - Size of addresses in bytes (4 or 8)                 │
├─────────────────────────────────────────────────────────┤
│ Segment Selector Size: u8 (DWARF 5 only)                │
│   - Size of segment selector (usually 0)                │
├─────────────────────────────────────────────────────────┤
│ Debug Information Entries (DIEs)                        │
│   - Encoded compilation unit content                    │
└─────────────────────────────────────────────────────────┘

.debug_line Section Header:
┌─────────────────────────────────────────────────────────┐
│ Unit Length: u32                                        │
├─────────────────────────────────────────────────────────┤
│ Version: u16                                            │
├─────────────────────────────────────────────────────────┤
│ Header Length: u32                                      │
│   - Bytes from here to start of line number program     │
├─────────────────────────────────────────────────────────┤
│ Minimum Instruction Length: u8                          │
├─────────────────────────────────────────────────────────┤
│ Maximum Operations Per Instruction: u8                  │
├─────────────────────────────────────────────────────────┤
│ Default Is Stmt: u8                                     │
│   - Initial value of is_stmt register                   │
├─────────────────────────────────────────────────────────┤
│ Line Base: i8                                           │
│   - Signed base for special opcodes                     │
├─────────────────────────────────────────────────────────┤
│ Line Range: u8                                          │
│   - Range for special opcodes                           │
├─────────────────────────────────────────────────────────┤
│ Opcode Base: u8                                         │
│   - First special opcode                                │
├─────────────────────────────────────────────────────────┤
│ Standard Opcode Lengths: u8[opcode_base - 1]            │
│   - Argument count for each standard opcode             │
├─────────────────────────────────────────────────────────┤
│ Include Directories: null-terminated strings            │
├─────────────────────────────────────────────────────────┤
│ File Names: null-terminated strings                     │
└─────────────────────────────────────────────────────────┘
  │
  ▼
Line Number Program (bytecode)
```

## Part 2: LEB128 Encoding

### Variable-Length Integer Encoding

```
DWARF uses LEB128 (Little Endian Base 128) for compact encoding:

Problem: Most debug values are small (line numbers, tags, etc.)
Fixed-size u32 wastes 3 bytes for small values
Solution: LEB128 encodes small values in fewer bytes

ULEB128 (Unsigned LEB128):
┌─────────────────────────────────────────────────────────┐
│ Encoding algorithm:                                     │
│                                                         │
│ while value > 0x7F:                                     │
│   output (value & 0x7F) | 0x80  // 7 bits + continue   │
│   value >>= 7                                           │
│ output value  // final byte, no continuation            │
│                                                         │
│ Decoding:                                               │
│                                                         │
│ result = 0                                              │
│ shift = 0                                               │
│ repeat:                                                 │
│   byte = next()                                         │
│   result |= (byte & 0x7F) << shift                      │
│   shift += 7                                            │
│   if (byte & 0x80) == 0: break  // no continuation     │
│                                                         │
│ Examples:                                                │
│   0         -> 0x00          (1 byte)                   │
│   1         -> 0x01          (1 byte)                   │
│   127       -> 0x7F          (1 byte)                   │
│   128       -> 0x80 0x01     (2 bytes)                  │
│   129       -> 0x81 0x01     (2 bytes)                  │
│   16383     -> 0xFF 0x7F     (2 bytes)                  │
│   16384     -> 0x80 0x80 0x01 (3 bytes)                 │
│   1000000   -> 0xC0 0xC4 0x3B (3 bytes)                 │
│   2^32-1    -> 0xFF 0xFF 0xFF 0xFF 0x0F (5 bytes)       │
│                                                         │
│ Space savings: ~50% for typical debug info              │
└─────────────────────────────────────────────────────────┘

SLEB128 (Signed LEB128):
┌─────────────────────────────────────────────────────────┐
│ For signed integers (line deltas, offsets)              │
│                                                         │
│ Similar to ULEB128 but:                                 │
│ - Uses sign extension                                   │
│ - Continues until sign bit is correct                   │
│                                                         │
│ Examples:                                                │
│   0         -> 0x00                                     │
│   1         -> 0x01                                     │
│   -1        -> 0x7F                                     │
│   2         -> 0x02                                     │
│   -2        -> 0x7E                                     │
│   127       -> 0x7F                                     │
│   -128      -> 0x80 0x7F                                │
│                                                         │
│ Key insight: Small positive and negative values         │
│ both encode efficiently                                 │
└─────────────────────────────────────────────────────────┘

Gimli Implementation:
```rust
use gimli::read::{EndianBuf, Endianity, Error, Result};

pub fn read_uleb128<R: Reader>(input: &mut R) -> Result<u64> {
    let mut result = 0u64;
    let mut shift = 0;

    loop {
        let byte = input.read_u8()?;
        result |= ((byte & 0x7F) as u64) << shift;
        shift += 7;

        if (byte & 0x80) == 0 {
            if shift > 63 && byte > 1 {
                return Err(Error::Overflow);
            }
            break;
        }

        if shift > 63 {
            return Err(Error::Overflow);
        }
    }

    Ok(result)
}

pub fn read_sleb128<R: Reader>(input: &mut R) -> Result<i64> {
    let mut result = 0i64;
    let mut shift = 0;
    let mut byte;

    loop {
        byte = input.read_u8()?;
        result |= ((byte & 0x7F) as i64) << shift;
        shift += 7;

        if (byte & 0x80) == 0 {
            // Sign extend if negative
            if shift < 64 && (byte & 0x40) != 0 {
                result |= -(1 << shift);
            }
            break;
        }

        if shift > 63 {
            return Err(Error::Overflow);
        }
    }

    Ok(result)
}
```
```

## Part 3: DWARF Compression

### ZLIB Compression (.zdebug_* sections)

```
Compressed DWARF Sections:

Standard sections (uncompressed):
- .debug_info, .debug_line, etc.

Compressed sections (zlib):
- .zdebug_info, .zdebug_line, etc.

Compression header (12 bytes):
┌─────────────────────────────────────────────────────────┐
│ Magic: "ZLIB" (4 bytes)                                 │
├─────────────────────────────────────────────────────────┤
│ Uncompressed Size: u64 (big-endian, 8 bytes)            │
└─────────────────────────────────────────────────────────┘
       │
       ▼
Zlib-compressed DWARF data

Decompression:
```rust
use flate2::read::ZlibDecoder;
use std::io::Read;

fn decompress_section(compressed: &[u8]) -> Vec<u8> {
    // Skip 4-byte magic ("ZLIB")
    assert_eq!(&compressed[0..4], b"ZLIB");

    // Read uncompressed size (big-endian u64)
    let size = u64::from_be_bytes(compressed[4..12].try_into().unwrap());

    // Decompress
    let mut decoder = ZlibDecoder::new(&compressed[12..]);
    let mut decompressed = Vec::with_capacity(size as usize);
    decoder.read_to_end(&mut decompressed).unwrap();

    assert_eq!(decompressed.len(), size as usize);
    decompressed
}
```

Compression ratios:
- .debug_info: 60-70% reduction
- .debug_line: 70-80% reduction
- .debug_str: 20-30% reduction (already strings)
- Overall: ~65% size reduction

Trade-offs:
✓ Much smaller binaries (faster download, less disk)
✓ Less I/O for remote debugging
✗ CPU cost to decompress
✗ Cannot mmap directly (must decompress first)

Gimli handles both:
```rust
// Check if section is compressed
if section_name.starts_with(".zdebug") {
    let decompressed = decompress_section(&section_data);
    DebugInfo::new(&decompressed, endian)
} else {
    DebugInfo::new(&section_data, endian)
}
```
```

### GNU Debuglink

```
Separate Debug Files (GNU debuglink):

Problem: Debug info makes binaries huge
Solution: Split debug info into separate file

Original binary (stripped):
┌─────────────────────────────────────────────────────────┐
│ Code and data only                                      │
│                                                         │
│ .text:    Executable code                              │
│ .data:    Initialized data                             │
│ .rodata:  Read-only data                               │
│                                                         │
│ .gnu_debuglink section:                                │
│   - Filename: "program.debug"                          │
│   - CRC32: 0x12345678 (checksum of debug file)         │
│                                                         │
│ Size: 1 MB (stripped)                                  │
└─────────────────────────────────────────────────────────┘

Separate debug file (program.debug):
┌─────────────────────────────────────────────────────────┐
│ Full DWARF debug info                                   │
│                                                         │
│ .debug_info:    Debug information                      │
│ .debug_line:    Line numbers                           │
│ .debug_abbrev:  Abbreviations                          │
│ .debug_str:     Strings                                │
│ .debug_frame:   Call frame info                        │
│ ...                                                     │
│                                                         │
│ Size: 5 MB (debug info only)                           │
└─────────────────────────────────────────────────────────┘

Debug file locations (searched in order):
1. Same directory as binary
2. .debug/ subdirectory
3. System debug directories (/usr/lib/debug)

Build ID method (modern alternative):
┌─────────────────────────────────────────────────────────┐
│ .note.gnu.build-id section:                             │
│   - Build ID: SHA1 hash of binary (20 bytes)            │
│                                                         │
│ Debug file location:                                    │
│   /usr/lib/debug/.build-id/ab/cdef1234...debug         │
│                          ^^ ^^^^^^^^^^^^                │
│                          first 2 hex, rest of hash      │
│                                                         │
│ More reliable than filename matching                    │
└─────────────────────────────────────────────────────────┘
```

## Part 4: Object File Support

### Multi-Format Support

```
Gimli supports multiple object file formats:

┌──────────────────────────────────────────────────────────┐
│ Format    │ OS          │ Extension │ Reader Crate      │
├──────────────────────────────────────────────────────────┤
│ ELF       │ Linux, BSD  │ (none)    │ object, goblin    │
│ Mach-O    │ macOS, iOS  │ (none)    │ object, mach      │
│ PE/COFF   │ Windows     │ .exe, .dll│ object, goblin    │
│ WASM      │ Web         │ .wasm     │ wasmparser        │
│ XCOFF     │ AIX         │ (none)    │ object            │
└──────────────────────────────────────────────────────────┘

Gimli is object-file agnostic:
- Works with any format that provides section data
- Doesn't parse ELF/Mach-O/PE headers itself
- Relies on object crate for file loading
```

Example: Loading from different formats
```rust
use gimli::{DebugInfo, LittleEndian, EndianSlice};
use object::{File, Object, ObjectSection};

fn load_debug_info(path: &str) -> Result<DebugInfo<EndianSlice<LittleEndian>>> {
    // Read file
    let data = std::fs::read(path)?;

    // Parse object file (auto-detect format)
    let file = File::parse(data.as_slice())?;

    // Extract .debug_info section
    let debug_info_data = file
        .section_by_name(".debug_info")
        .ok_or("No debug_info section")?
        .data()?;

    // Create Gimli parser
    let endian = LittleEndian;
    let debug_info = DebugInfo::new(debug_info_data, endian);

    Ok(debug_info)
}

// Works for:
// - Linux ELF binaries
// - macOS Mach-O executables
// - Windows PE executables
// - WASM modules
// - Object files (.o, .obj)
```

### WASM Debug Info

```
WASM Debug Info (custom sections):

WASM module structure:
┌─────────────────────────────────────────────────────────┐
│ Magic: 0x00 asm                                         │
│ Version: 1                                               │
├─────────────────────────────────────────────────────────┤
│ Type Section    - Function signatures                   │
│ Import Section  - Imported functions/memory             │
│ Function Section - Function indices                     │
│ Memory Section  - Memory declarations                   │
│ Export Section  - Exported functions/memory             │
│ Start Section   - Start function                        │
│ Element Section - Function table init                   │
│ Code Section    - Function bodies                       │
│ Data Section    - Memory init                           │
├─────────────────────────────────────────────────────────┤
│ Custom Sections:                                        │
│   .debug_info   - DWARF debug info                      │
│   .debug_line   - Line number info                      │
│   .debug_abbrev - Abbreviations                         │
│   .debug_str    - String table                          │
│   .debug_loc    - Location expressions                  │
│   .debug_ranges - Address ranges                        │
│   name          - Function names (non-DWARF)            │
│   sourceMap     - Source map (non-DWARF)                │
└─────────────────────────────────────────────────────────┘

WASM-specific considerations:
- Addresses are bytecode offsets, not memory addresses
- Code section contains bytecode, not machine code
- DWARF must be interpreted relative to WASM runtime

Gimli handles WASM:
```rust
use wasmparser::Parser;

fn extract_wasm_debug_info(wasm_data: &[u8]) -> Result<()> {
    let parser = Parser::new(0);

    for payload in parser.parse_all(wasm_data) {
        if let wasmparser::Payload::CustomSection(section) = payload? {
            match section.name() {
                ".debug_info" => {
                    let debug_info = DebugInfo::new(section.data(), LittleEndian);
                    // Process debug info...
                }
                ".debug_line" => {
                    let debug_line = DebugLine::new(section.data(), LittleEndian);
                    // Process line info...
                }
                _ => {}
            }
        }
    }

    Ok(())
}
```
```

---

*This document is part of the Gimli-rs exploration series. See [exploration.md](./exploration.md) for the complete index.*
