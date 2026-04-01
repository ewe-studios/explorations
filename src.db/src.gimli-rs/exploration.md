---
title: "Gimli-rs: Complete Exploration"
subtitle: "DWARF debugging information parser"
location: /home/darkvoid/Boxxed/@dev/repo-expolorations/src.db/src.gimli-rs
repository: https://github.com/gimli-rs/gimli
explored_at: 2026-03-28
status: COMPLETE
---

# Gimli-rs: Complete Exploration

## Overview

**Gimli** is a Rust library for parsing DWARF debugging information:
- **Zero-copy parsing** - Borrows data, no allocation
- **Safe Rust** - No unsafe code in parsing
- **Cross-platform** - ELF, Mach-O, PE, WASM support
- **Used by** - addr2line, backtrace, cargo-symbolicate

### Key Characteristics

| Aspect | Gimli |
|--------|-------|
| **Core** | DWARF debug info parser |
| **Language** | Rust |
| **License** | MIT/Apache-2.0 |
| **Design** | Zero-copy, lazy evaluation |
| **Backed by** | Mozilla, FitBench |

### Documents

| Document | Description |
|----------|-------------|
| [exploration.md](./exploration.md) | Overview |
| [00-zero-to-debug-engineer.md](./00-zero-to-debug-engineer.md) | DWARF fundamentals |
| [01-storage-engine-deep-dive.md](./01-storage-engine-deep-dive.md) | DWARF sections, encoding |
| [02-parsing-deep-dive.md](./02-parsing-deep-dive.md) | Zero-copy parsing |
| [rust-revision.md](./rust-revision.md) | Already in Rust! |
| [production-grade.md](./production-grade.md) | Usage patterns |
| [04-symbolication.md](./04-symbolication.md) | Stack unwinding |

---

## DWARF Debugging Format

```
DWARF Sections (in ELF/Mach-O):
├── .debug_info      - Compilation units, DIEs
├── .debug_abbrev    - Abbreviation declarations
├── .debug_line      - Line number program
├── .debug_str       - String table
├── .debug_addr      - Address table
├── .debug_ranges    - Address ranges
├── .debug_rnglists  - Range lists (DWARF 5)
├── .debug_loclists  - Location lists (DWARF 5)
├── .debug_frame     - Call frame info
├── .eh_frame        - Exception handling
└── .gnu_debuglink   - Separate debug file link
```

---

## Quick Start

```rust
use gimli::{DebugInfo, DebugAbbrev, DebugLine, EndianSlice, LittleEndian};

// Load object file
let data = std::fs::read("program").unwrap();
let file = object::File::parse(data.as_slice()).unwrap();

// Extract DWARF sections
let debug_info_data = file.section_data(".debug_info").unwrap();
let debug_abbrev_data = file.section_data(".debug_abbrev").unwrap();

// Parse DWARF
let endian = LittleEndian;
let debug_info = DebugInfo::new(debug_info_data, endian);
let debug_abbrev = DebugAbbrev::new(debug_abbrev_data, endian);

// Iterate compilation units
let mut units = debug_info.units();
while let Some(header) = units.next().unwrap() {
    let abbrev = debug_abbrev.abbreviations(&header).unwrap();

    // Read DIEs
    let mut entries = header.entries(&abbrev);
    while let Some((depth, entry)) = entries.next().unwrap() {
        println!("Entry: {:?}", entry.tag());
    }
}
```

---

## Document History

| Date | Change |
|------|--------|
| 2026-03-28 | Full exploration completed |
| 2026-03-28 | Added 00-zero-to-debug-engineer.md |
| 2026-03-28 | Added 01-storage-engine-deep-dive.md |
| 2026-03-28 | Added 02-parsing-deep-dive.md |
| 2026-03-28 | Added rust-revision.md |
| 2026-03-28 | Added production-grade.md |
| 2026-03-28 | Added 04-symbolication.md |
