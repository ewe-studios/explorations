---
title: "Zero to Debug Engineer: Gimli-rs"
subtitle: "Understanding DWARF debugging information and stack unwinding"
location: /home/darkvoid/Boxxed/@dev/repo-expolorations/src.db/src.gimli-rs
related: exploration.md
---

# 00 - Zero to Debug Engineer: Gimli-rs

## Overview

This document explains DWARF debugging fundamentals - how compilers encode debug information, how tools like gdb/addr2line use it, and how Gimli provides safe zero-copy parsing.

## Part 1: Why DWARF Exists

### The Debugging Problem

```
Without Debug Info:
┌─────────────────────────────────────────────────────────┐
│  Compiled Binary (no debug info)                        │
│                                                         │
│  0x00401000: push rbp                                   │
│  0x00401001: mov rbp, rsp                               │
│  0x00401004: sub rsp, 0x10                              │
│  0x00401008: mov dword [rbp-4], edi                     │
│  ...                                                    │
│                                                         │
│  Crash at 0x00401020:                                   │
│  "Segmentation fault at address 0x00401020"             │
│                                                         │
│  Problems:                                              │
│  - No function names                                    │
│  - No file/line information                             │
│  - No variable names or types                           │
│  - No type information                                  │
│  - Impossible to debug                                  │
└─────────────────────────────────────────────────────────┘

With DWARF Debug Info:
┌─────────────────────────────────────────────────────────┐
│  Compiled Binary + .debug_* sections                    │
│                                                         │
│  Crash at 0x00401020:                                   │
│  "Segmentation fault in main() at main.c:42"           │
│                                                         │
│  Stack trace:                                           │
│  #0 main() at main.c:42                                 │
│  #1 helper_function(x=5) at helper.c:15                 │
│  #2 __libc_start_main at libc.c:100                     │
│                                                         │
│  Variables:                                             │
│  - x: 5 (int)                                           │
│  - ptr: 0x7ffd... (char*)                               │
│                                                         │
│  Benefits:                                              │
│  ✓ Source-level debugging                               │
│  ✓ Stack unwinding                                      │
│  ✓ Variable inspection                                  │
│  ✓ Type information                                     │
└─────────────────────────────────────────────────────────┘
```

### What is DWARF?

```
DWARF = Debug With Attributed Record Formats

500+ page specification defining:
- How to encode source file/line mappings
- How to encode function/subroutine info
- How to encode variable names and types
- How to encode optimized code info
- How to unwind stack frames

Versions:
- DWARF 2: Widely supported (GCC default for years)
- DWARF 3: Added C++ support
- DWARF 4: Compression, improved optimization support
- DWARF 5: Modern features, better performance (current)

Debug Info Size:
- Typical: 2-5x larger than code section
- Optimized builds: Less debug info
- Unoptimized (-g0): No debug info
- Full debug (-g3): Maximum detail
```

## Part 2: DWARF Data Structures

### Compilation Units

```
DWARF organizes debug info into Compilation Units (CUs):

Each CU represents one compilation (one .c/.rs file):
┌─────────────────────────────────────────────────────────┐
│  Compilation Unit Header                                │
│  ┌───────────────────────────────────────────────────┐  │
│  │ Length: u32 (total CU size)                       │  │
│  │ Version: u16 (DWARF version)                      │  │
│  │ Abbrev Offset: u32 (pointer to abbrev table)      │  │
│  │ Address Size: u8 (pointer size, usually 8)        │  │
│  └───────────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────────┘
       │
       ▼
┌─────────────────────────────────────────────────────────┐
│  Debug Information Entries (DIEs)                       │
│                                                         │
│  DIE 0: Compile Unit                                   │
│    Tag: DW_TAG_compile_unit                            │
│    Attributes:                                         │
│      DW_AT_name: "main.c"                              │
│      DW_AT_language: DW_LANG_C99                       │
│      DW_AT_comp_dir: "/home/user/project"              │
│      DW_AT_producer: "GCC 11.2.0"                      │
│                                                         │
│  DIE 1: Subprogram (function)                          │
│    Tag: DW_TAG_subprogram                              │
│    Attributes:                                         │
│      DW_AT_name: "main"                                │
│      DW_AT_decl_file: 0 (index into file table)        │
│      DW_AT_decl_line: 1                                │
│      DW_AT_low_pc: 0x00401000                          │
│      DW_AT_high_pc: 0x00401050                         │
│                                                         │
│  DIE 2: Formal Parameter                               │
│    Tag: DW_TAG_formal_parameter                        │
│    Attributes:                                         │
│      DW_AT_name: "argc"                                │
│      DW_AT_type: reference to DIE 5 (int type)         │
│      DW_AT_location: DW_OP_fbreg + 4                   │
│                                                         │
│  DIE 3: Variable                                       │
│    Tag: DW_TAG_variable                                │
│    Attributes:                                         │
│      DW_AT_name: "result"                              │
│      DW_AT_type: reference to DIE 5 (int type)         │
│      DW_AT_location: DW_OP_fbreg - 8                   │
│                                                         │
│  DIE 4: Lexical Block (scope)                          │
│    Tag: DW_TAG_lexical_block                           │
│    Attributes:                                         │
│      DW_AT_low_pc: 0x00401020                          │
│      DW_AT_high_pc: 0x00401040                         │
│                                                         │
│  DIE 5: Base Type                                      │
│    Tag: DW_TAG_base_type                               │
│    Attributes:                                         │
│      DW_AT_name: "int"                                 │
│      DW_AT_encoding: DW_ATE_signed                     │
│      DW_AT_byte_size: 4                                │
│                                                         │
│  DIE Tree Structure:                                   │
│    [0] Compile Unit                                    │
│      [1] Subprogram "main"                             │
│        [2] Formal Parameter "argc"                     │
│        [3] Variable "result"                           │
│        [4] Lexical Block                               │
│          ... (nested scope content)                    │
│      [5] Base Type "int"                               │
│                                                         │
│  Children are nested, siblings are linked              │
└─────────────────────────────────────────────────────────┘
```

### Abbreviation Tables

```
Abbreviation Table - Compact DIE templates:

Problem: DIEs have repetitive structure
Solution: Define abbreviations (templates) once

Abbrev Table:
┌─────────────────────────────────────────────────────────┐
│ Abbrev #1:                                             │
│   Tag: DW_TAG_compile_unit                             │
│   Children: yes                                        │
│   Attributes:                                          │
│     [DW_AT_name]       form: DW_FORM_string            │
│     [DW_AT_language]   form: DW_FORM_data1             │
│     [DW_AT_comp_dir]   form: DW_FORM_string            │
│     [DW_AT_producer]   form: DW_FORM_string            │
│                                                         │
│ Abbrev #2:                                             │
│   Tag: DW_TAG_subprogram                               │
│   Children: yes                                        │
│   Attributes:                                          │
│     [DW_AT_name]       form: DW_FORM_string            │
│     [DW_AT_decl_file]  form: DW_FORM_data1             │
│     [DW_AT_decl_line]  form: DW_FORM_data1             │
│     [DW_AT_low_pc]     form: DW_FORM_addr              │
│     [DW_AT_high_pc]    form: DW_FORM_addr              │
│                                                         │
│ Abbrev #3:                                             │
│   Tag: DW_TAG_base_type                                │
│   Children: no                                         │
│   Attributes:                                          │
│     [DW_AT_name]       form: DW_FORM_string            │
│     [DW_AT_encoding]   form: DW_FORM_data1             │
│     [DW_AT_byte_size]  form: DW_FORM_data1             │
│                                                         │
│ Using abbreviations:                                   │
│                                                         │
│ DIE in .debug_info references abbrev #1:               │
│   [abbrev_num: 1]                                      │
│   [DW_AT_name: "main.c"]                               │
│   [DW_AT_language: 0x01]                               │
│   [DW_AT_comp_dir: "/home/user/project"]               │
│   [DW_AT_producer: "GCC 11.2.0"]                       │
│                                                         │
│ Size savings: ~50% reduction                           │
└─────────────────────────────────────────────────────────┘
```

### Line Number Programs

```
DWARF Line Number Table - Maps addresses to source lines:

Problem: Machine code has no concept of "line 42"
Solution: DWARF line number program

Line Number Table Format:
┌─────────────────────────────────────────────────────────┐
│ Line Number Program Header                             │
│ ┌───────────────────────────────────────────────────┐  │
│ │ Minimum instruction length: 1                     │  │
│ │ Maximum operations per instruction: 1             │  │
│ │ Default discriminator size: 0                     │  │
│ │ Code base: 1, Line base: -5, Range: 14            │  │
│ │ Opcode base: 13                                   │  │
│ │ Include directories: ["/usr/include", ...]        │  │
│ │ File names: ["main.c", "helper.h", ...]           │  │
│ └───────────────────────────────────────────────────┘  │
├─────────────────────────────────────────────────────────┤
│ Line Number Program (bytecode for state machine)       │
│                                                         │
│ State Machine Registers:                               │
│   address: Current address (starts at low_pc)          │
│   line: Current line number (starts at 1)              │
│   column: Current column (starts at 0)                 │
│   file: Current file index (starts at 1)               │
│   discriminator: Distinguishes same line/col           │
│   is_stmt: Is statement boundary (for debugger)        │
│   basic_block: Is basic block start                    │
│   end_sequence: Is end of sequence                     │
│                                                         │
│ Example Program:                                       │
│   DW_LNS_extended_op (length=3, opcode=0x01)          │
│     -> Set address to 0x00401000                       │
│   DW_LNS_copy                                          │
│     -> Emit row: (address=0x00401000, file=1, line=1) │
│   DW_LNS_advance_line (line_delta=41)                  │
│     -> line = 1 + 41 = 42                              │
│   DW_LNS_const_add_pc (special opcode)                 │
│     -> address += (opcode - base) / range = 0x20       │
│   DW_LNS_copy                                          │
│     -> Emit row: (address=0x00401020, file=1, line=42)│
│   DW_LNS_advance_line (line_delta=5)                   │
│     -> line = 42 + 5 = 47                              │
│   DW_LNS_const_add_pc                                  │
│     -> address += 0x20 = 0x00401040                    │
│   DW_LNS_copy                                          │
│     -> Emit row: (address=0x00401040, file=1, line=47)│
│   DW_LNS_extended_op (opcode=0x04)                     │
│     -> End sequence                                    │
│                                                         │
│ Generated Line Table:                                  │
│ ┌──────────────┬──────────┬──────────┬──────────┐     │
│ │ Address      │ File     │ Line     │ Column   │     │
│ ├──────────────┼──────────┼──────────┼──────────┤     │
│ │ 0x00401000   │ main.c   │ 1        │ 0        │     │
│ │ 0x00401020   │ main.c   │ 42       │ 0        │     │
│ │ 0x00401040   │ main.c   │ 47       │ 0        │     │
│ └──────────────┴──────────┴──────────┴──────────┘     │
│                                                         │
│ Debugger usage:                                        │
│ - PC = 0x00401025 -> line 42 (binary search table)     │
│ - Show "main.c:42" to user                             │
└─────────────────────────────────────────────────────────┘

Special Opcodes (optimized for common case):
Most lines advance by 1-10, addresses advance steadily
Special opcode = (line_delta - line_base) + (address_delta * range) + opcode_base
Single byte encodes both line and address advance!
```

## Part 3: Stack Unwinding

### Call Frame Information

```
DWARF Call Frame Information (CFI) - Stack unwinding:

Problem: How to find caller's stack frame?
Solution: DWARF CFI describes how to restore registers

CFI describes how to compute:
- Caller's return address (where to continue execution)
- Caller's frame pointer (where is caller's local vars)
- Caller's callee-saved registers

Frame Description Entry (FDE):
┌─────────────────────────────────────────────────────────┐
│ FDE for function at 0x1000-0x1050                      │
│ ┌───────────────────────────────────────────────────┐  │
│ │ Initial state (function prologue complete):       │  │
│ │   CFA (Canonical Frame Address) = RSP + 16        │  │
│ │   RBP at [CFA - 16]                               │  │
│ │   Return address at [CFA - 8]                     │  │
│ │                                                   │  │
│ │ Instructions (row-by-row state changes):          │  │
│ │   offset=0: CFA = RSP + 8                         │  │
│ │   offset=5: RBP = [CFA - 16]                      │  │
│ │   offset=10: ReturnAddr = [CFA - 8]               │  │
│ │   offset=20: CFA = RBP, RSP = [CFA - 16]          │  │
│ │   offset=45: End (function epilogue)              │  │
│ └───────────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────────┘

Stack Unwinding Process:
┌─────────────────────────────────────────────────────────┐
│ Current state:                                          │
│   PC = 0x1025 (in function)                             │
│   SP = 0x7ffd1000                                       │
│   BP = 0x7ffd0fe0                                       │
│                                                         │
│ Step 1: Find FDE for PC=0x1025                          │
│   -> FDE covers 0x1000-0x1050                           │
│                                                         │
│ Step 2: Evaluate CFI at offset=0x25                     │
│   -> CFA = RBP                                          │
│   -> Return address at [CFA - 8] = [0x7ffd0fe0 - 8]     │
│   -> Read return address: 0x1055                        │
│                                                         │
│ Step 3: Restore caller's BP                             │
│   -> BP = [CFA - 16] = [0x7ffd0fe0 - 16] = 0x7ffd0fc0   │
│                                                         │
│ Step 4: Update to caller state                          │
│   -> New PC = 0x1055 (return address)                   │
│   -> New SP = CFA = 0x7ffd0fe0                          │
│   -> New BP = 0x7ffd0fc0                                │
│                                                         │
│ Step 5: Repeat until root (main, _start)                │
│                                                         │
│ Result: Complete stack trace!                           │
│   #0 func() at main.c:42                                │
│   #1 caller() at main.c:50                              │
│   #2 main() at main.c:10                                │
└─────────────────────────────────────────────────────────┘
```

### Unwinding Example

```
Complete Unwinding Example:

Source Code:
```c
void leaf() {
    int x = 42;  // line 5
    *(int*)0 = x; // line 6 - CRASH!
}

void middle() {
    leaf();  // line 10
}

void root() {
    middle();  // line 14
}

int main() {
    root();  // line 18
}
```

Crash State:
```
RIP = 0x1234  (in leaf, at *(int*)0 = x)
RSP = 0x7ffd0100
RBP = 0x7ffd00f0
```

Unwind Step 1 (leaf -> middle):
```
FDE for leaf (0x1200-0x1250):
  CFA = RBP
  ReturnAddr at [CFA - 8]

CFA = 0x7ffd00f0
ReturnAddr = [0x7ffd00f0 - 8] = 0x7ffd00e8 = 0x1260
New RBP = [0x7ffd00f0 - 16] = 0x7ffd00e0

Result:
  RIP = 0x1260 (in middle, at call leaf())
  RSP = 0x7ffd00f0
  RBP = 0x7ffd00e0
```

Unwind Step 2 (middle -> root):
```
FDE for middle (0x1260-0x1280):
  CFA = RBP + 16
  ReturnAddr at [CFA - 8]

CFA = 0x7ffd00e0 + 16 = 0x7ffd00f0
ReturnAddr = [0x7ffd00f0 - 8] = 0x7ffd00e8 = 0x1290

Result:
  RIP = 0x1290 (in root, at call middle())
  RSP = 0x7ffd00f0
  RBP = 0x7ffd00d0
```

Unwind Step 3 (root -> main):
```
Result:
  RIP = 0x12b0 (in main, at call root())
  RSP = 0x7ffd0100
  RBP = 0x7ffd00c0
```

Final Stack Trace:
```
#0 leaf() at crash.c:6
   RIP = 0x1234
#1 middle() at crash.c:10
   RIP = 0x1260
#2 root() at crash.c:14
   RIP = 0x1290
#3 main() at crash.c:18
   RIP = 0x12b0
#4 __libc_start_main()
#5 _start()
```

This is how addr2line, gdb, and backtrace-rs work!
```

## Part 4: Gimli Design Philosophy

### Zero-Copy Parsing

```
Gimli's Design Philosophy:

Traditional Parser (allocating):
```rust
struct DebugInfo {
    compilation_units: Vec<CompilationUnit>,
    strings: HashMap<u64, String>,
    // Lots of owned data
}

fn parse(data: &[u8]) -> DebugInfo {
    let mut info = DebugInfo::new();
    // Allocate, copy, convert everything
    info.strings.insert(offset, String::from(bytes));
    // ...
    info
}
```

Problems:
- Allocates memory for all debug info
- Copies strings into new allocations
- Slow startup time
- High memory usage

Gimli's Zero-Copy Approach:
```rust
struct DebugInfo<'input> {
    data: &'input [u8],  // Borrow input, no allocation
}

fn parse<'input>(data: &'input [u8]) -> DebugInfo<'input> {
    DebugInfo { data }  // Just store the slice
}

// Reading a string - returns a view, doesn't allocate
fn read_string(offset: usize) -> &'input [u8] {
    // Return slice of original data, no allocation
    &self.data[offset..]
}
```

Benefits:
- No allocation during parsing
- Strings are views into original data
- Mmap file directly, parse in-place
- Minimal memory overhead
- Fast startup (important for tools)

Usage pattern:
```rust
// Mmap the binary (zero-copy file access)
let file = File::open("program").unwrap();
let mmap = unsafe { MmapOptions::new().map(&file).unwrap() };

// Parse directly from mmap'd data
let debug_info = DebugInfo::new(&mmap[debug_info_range]);

// Reading attributes doesn't allocate
for unit in debug_info.units() {
    for (_, entry) in unit.entries() {
        for attr in entry.attrs() {
            // attr.value() returns enum with borrowed data
            match attr.value() {
                AttributeValue::String(s) => {
                    // s is &[u8], not String
                    println!("{}", std::str::from_utf8(s).unwrap());
                }
                _ => {}
            }
        }
    }
}
// mmap still in scope, debug_info borrows it
```
```

### Lazy Evaluation

```
Gimli uses lazy evaluation everywhere:

Eager parsing (traditional):
```
Read entire .debug_info section
  -> Parse all CUs
    -> Parse all DIEs
      -> Read all attributes
        -> Convert all values

Even if you only need the first function name!
```

Lazy parsing (Gimli):
```
Create parser for .debug_info section
  -> Iterate CUs (one at a time)
    -> Iterate DIEs (one at a time)
      -> Read attributes on demand
        -> Convert values only when accessed

Only pay for what you actually use!
```

Example: Find function containing address
```rust
// Find the CU containing address 0x1234
let mut units = debug_info.units();
while let Some(header) = units.next()? {
    // Only parse this CU's abbreviations when needed
    let abbrevs = header.abbreviations(&debug_abbrev)?;

    // Only read DIEs in this CU
    let mut entries = header.entries(&abbrevs);
    while let Some((depth, entry)) = entries.next()? {
        // Only read attributes we actually access
        if entry.tag() == DW_TAG_subprogram {
            let low_pc = entry.attr_value(DW_AT_low_pc)?;
            let high_pc = entry.attr_value(DW_AT_high_pc)?;

            // Check if address is in this function
            if low_pc <= 0x1234 && 0x1234 < high_pc {
                // NOW read the function name
                let name = entry.attr_string(DW_AT_name)?;
                println!("Found: {}", name);
                break;
            }
        }
    }
}
```

Key insight: Most debug info is never used!
- Typical program: millions of DIEs
- Stack trace: needs ~20 DIEs
- Lazy evaluation = 1000x less work
```

---

*This document is part of the Gimli-rs exploration series. See [exploration.md](./exploration.md) for the complete index.*
