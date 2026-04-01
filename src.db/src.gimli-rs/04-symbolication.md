---
title: "Gimli-rs Symbolication"
subtitle: "Stack unwinding, frame resolution, and crash reporting"
location: /home/darkvoid/Boxxed/@dev/repo-expolorations/src.db/src.gimli-rs
related: exploration.md, 00-zero-to-debug-engineer.md
---

# 04 - Symbolication: Gimli-rs

## Overview

This document covers stack symbolication using Gimli - converting raw instruction pointers to function names, file locations, and building crash reports.

## Part 1: Stack Unwinding Fundamentals

### Unwinding Methods

```
Three approaches to get stack frames:

1. Frame Pointer Unwinding (traditional)
┌─────────────────────────────────────────────────────────┐
│ Each function saves old frame pointer:                  │
│                                                         │
│ prologue:                                               │
│   push rbp              ; Save caller's frame pointer   │
│   mov rbp, rsp          ; Set up new frame              │
│   sub rsp, 0x20         ; Allocate locals               │
│                                                         │
│ epilogue:                                               │
│   leave                 ; Restore rsp, pop rbp          │
│   ret                                                   │
│                                                         │
│ Unwinding:                                              │
│   caller_rbp = [rbp]                                    │
│   return_addr = [rbp + 8]                               │
│   rbp = caller_rbp (repeat)                             │
│                                                         │
│ Pros: Simple, fast                                      │
│ Cons: Requires -fno-omit-framepointer                   │
└─────────────────────────────────────────────────────────┘

2. DWARF CFI Unwinding (modern default)
┌─────────────────────────────────────────────────────────┐
│ Uses .debug_frame / .eh_frame sections:                 │
│                                                         │
│ FDE (Frame Description Entry):                          │
│   Range: 0x1000-0x1050 (function bounds)                │
│   Rules:                                                │
│     CFA = RSP + 8                                       │
│     RBP at [CFA - 16]                                   │
│     ReturnAddr at [CFA - 8]                             │
│                                                         │
│ Unwinding:                                              │
│   1. Find FDE for current PC                            │
│   2. Evaluate CFI rules at current PC                   │
│   3. Compute CFA (Canonical Frame Address)              │
│   4. Restore registers from CFA                         │
│   5. Repeat until root                                  │
│                                                         │
│ Pros: Works with optimized code, no frame pointers      │
│ Cons: Complex, requires DWARF parsing (Gimli!)          │
└─────────────────────────────────────────────────────────┘

3. Platform-Specific (libunwind, DbgHelp)
┌─────────────────────────────────────────────────────────┐
│ Windows: DbgHelp API (CaptureStackBackTrace)            │
│ macOS: libunwind (system library)                       │
│ Linux: libunwind or DWARF via libgcc                    │
│                                                         │
│ Pros: System-provided, maintained                       │
│ Cons: Platform-specific, less control                   │
└─────────────────────────────────────────────────────────┘

Rust backtrace crate uses:
- DWARF CFI on Linux/macOS (via gimli)
- DbgHelp on Windows
```

### Unwinding Implementation

```rust
/// Simple DWARF CFI unwinder using Gimli
use gimli::*;
use std::collections::HashMap;

pub struct Unwinder {
    /// CFI information from .debug_frame
    cie_fdes: HashMap<u64, FrameDescriptionEntry>,
}

pub struct FrameDescriptionEntry {
    /// Start address of function
    pub initial_address: u64,
    /// Function length
    pub address_range: u64,
    /// CFI rules (simplified)
    pub cfa_rule: CfaRule,
    pub return_addr_rule: RegRule,
}

pub enum CfaRule {
    /// CFA = register + offset
    RegPlusOffset(u16, i64),
}

pub enum RegRule {
    /// Register saved at [CFA + offset]
    AtCfaPlus(i64),
    /// Register in same location as before
    Same,
    /// Register not saved (caller-saved)
    NotSaved,
}

impl Unwinder {
    pub fn new<R: Reader>(debug_frame: &DebugFrame<R>) -> Result<Self> {
        let mut cie_fdes = HashMap::new();

        // Parse FDEs
        let mut entries = debug_frame.entries();
        while let Some(entry) = entries.next()? {
            if let CieOrFde::Fde(partial_fde) = entry {
                let fde = partial_fde.parse(|_, _, _| Ok(None))?;

                // Store for lookup
                cie_fdes.insert(
                    fde.initial_address(),
                    FrameDescriptionEntry {
                        initial_address: fde.initial_address(),
                        address_range: fde.len(),
                        cfa_rule: extract_cfa_rule(&fde)?,
                        return_addr_rule: extract_return_addr_rule(&fde)?,
                    },
                );
            }
        }

        Ok(Self { cie_fdes })
    }

    /// Unwind one frame
    pub fn unwind_frame(&self, state: &MachineState) -> Option<Frame> {
        // Find FDE for current PC
        let fde = self.find_fde(state.pc)?;

        // Evaluate CFI at current PC
        let cfa = self.evaluate_cfa(&fde.cfa_rule, state)?;

        // Get return address
        let return_addr = self.evaluate_reg_rule(
            &fde.return_addr_rule,
            state,
            cfa,
        )?;

        Some(Frame {
            pc: state.pc,
            sp: cfa,
            return_addr,
        })
    }

    fn find_fde(&self, pc: u64) -> Option<&FrameDescriptionEntry> {
        // Binary search for FDE containing pc
        self.cie_fdes.values().find(|fde| {
            fde.initial_address <= pc
                && pc < fde.initial_address + fde.address_range
        })
    }

    fn evaluate_cfa(&self, rule: &CfaRule, state: &MachineState) -> Option<u64> {
        match rule {
            CfaRule::RegPlusOffset(reg, offset) => {
                let reg_value = state.get_register(*reg)?;
                Some(reg_value.wrapping_add(*offset as u64))
            }
        }
    }

    fn evaluate_reg_rule(
        &self,
        rule: &RegRule,
        state: &MachineState,
        cfa: u64,
    ) -> Option<u64> {
        match rule {
            RegRule::AtCfaPlus(offset) => {
                let addr = cfa.wrapping_add(*offset as u64);
                state.read_memory(addr)
            }
            RegRule::Same => Some(state.get_register(0)?), // RAX
            RegRule::NotSaved => None,
        }
    }
}

pub struct MachineState {
    pub pc: u64,
    pub registers: [u64; 16],
    // Memory read callback
    pub read_memory: Box<dyn Fn(u64) -> Option<u64>>,
}

impl MachineState {
    fn get_register(&self, reg: u16) -> Option<u64> {
        self.registers.get(reg).copied()
    }
}

pub struct Frame {
    pub pc: u64,
    pub sp: u64,
    pub return_addr: u64,
}

/// Full stack unwinding
pub fn unwind_stack(
    unwinder: &Unwinder,
    initial_state: MachineState,
) -> Vec<Frame> {
    let mut frames = Vec::new();
    let mut state = initial_state;

    loop {
        if let Some(frame) = unwinder.unwind_frame(&state) {
            frames.push(frame.clone());

            // Move to caller
            state.pc = frame.return_addr;
            state.registers[0] = frame.sp; // RSP = CFA
        } else {
            break;
        }
    }

    frames
}
```

## Part 2: Symbolication Pipeline

### Complete Symbolication

```rust
/// Full symbolication: addresses -> source locations
use addr2line::Context;
use object::Object;

pub struct Symbolicator {
    context: Context<Vec<u8>>,
    names: HashMap<u64, String>,
}

impl Symbolicator {
    pub fn new<P: AsRef<Path>>(path: P) -> Result<Self, Box<dyn Error>> {
        let data = std::fs::read(path)?;
        let file = object::File::parse(data.as_slice())?;

        let context = Context::new(&file)?;

        // Pre-extract symbol names
        let mut names = HashMap::new();
        for symbol in file.symbols() {
            if let Some(name) = symbol.name() {
                names.insert(symbol.address(), name.to_string());
            }
        }

        Ok(Self { context, names })
    }

    /// Symbolicate a single address
    pub fn symbolicate(&self, addr: u64) -> SymbolicatedFrame {
        let mut result = SymbolicatedFrame {
            address: addr,
            function: None,
            demangled: None,
            file: None,
            line: None,
            column: None,
        };

        // Get function name from symbol table
        if let Some(name) = self.names.get(&addr) {
            result.function = Some(name.clone());
            result.demangled = Some(demangle(name));
        }

        // Get source location from DWARF
        if let Ok(mut frames) = self.context.find_frames(addr) {
            if let Ok(Some(frame)) = frames.next() {
                if result.function.is_none() {
                    if let Some(func) = &frame.function {
                        result.function = Some(func.raw_name().to_string());
                        result.demangled = Some(func.demangle());
                    }
                }

                if let Some(location) = frame.location {
                    result.file = location.file.map(|s| s.to_string());
                    result.line = location.line;
                    result.column = location.column.map(|c| c.get());
                }
            }
        }

        result
    }

    /// Symbolicate full stack trace
    pub fn symbolicate_stack(&self, addresses: &[u64]) -> Vec<SymbolicatedFrame> {
        addresses.iter().map(|&addr| self.symbolicate(addr)).collect()
    }
}

pub struct SymbolicatedFrame {
    pub address: u64,
    pub function: Option<String>,
    pub demangled: Option<String>,
    pub file: Option<String>,
    pub line: Option<u64>,
    pub column: Option<u64>,
}

impl std::fmt::Display for SymbolicatedFrame {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Some(func) = &self.demangled {
            write!(f, "{}", func)?;
        } else if let Some(func) = &self.function {
            write!(f, "{}", func)?;
        } else {
            write!(f, "??")?;
        }

        if let (Some(file), Some(line)) = (&self.file, self.line) {
            write!(f, " ({}:{})", file, line)?;
            if let Some(col) = self.column {
                write!(f, ":{}", col)?;
            }
        }

        Ok(())
    }
}

/// Demangle Rust/C++ symbols
fn demangle(name: &str) -> String {
    // Try Rust demangling
    if let Ok(sym) = rustc_demangle::try_demangle(name) {
        return sym.to_string();
    }

    // Try C++ demangling
    if let Ok(sym) = cpp_demangle::Symbol::new(name) {
        return sym.demangle().unwrap_or_else(|_| name.to_string());
    }

    name.to_string()
}
```

### Crash Report Generation

```rust
/// Generate crash report from backtrace
use backtrace::Backtrace;

pub struct CrashReport {
    pub timestamp: String,
    pub threads: Vec<ThreadReport>,
    pub binary_info: BinaryInfo,
}

pub struct ThreadReport {
    pub id: u64,
    pub name: String,
    pub frames: Vec<SymbolicatedFrame>,
    pub crashed: bool,
}

pub struct BinaryInfo {
    pub path: String,
    pub build_id: Option<String>,
}

impl CrashReport {
    pub fn generate() -> Result<Self, Box<dyn Error>> {
        let symbolicator = Symbolicator::new(std::env::current_exe()?)?;

        // Get current backtrace
        let bt = Backtrace::new();
        let addresses: Vec<u64> = bt.frames()
            .iter()
            .map(|f| f.ip() as u64)
            .collect();

        let frames = symbolicator.symbolicate_stack(&addresses);

        let report = CrashReport {
            timestamp: chrono::Utc::now().to_rfc3339(),
            threads: vec![ThreadReport {
                id: 0,
                name: "main".to_string(),
                frames,
                crashed: true,
            }],
            binary_info: BinaryInfo {
                path: std::env::current_exe()?.to_string_lossy().to_string(),
                build_id: read_build_id()?,
            },
        };

        Ok(report)
    }

    pub fn format(&self) -> String {
        let mut output = String::new();

        output.push_str("=== Crash Report ===\n\n");
        output.push_str(&format!("Timestamp: {}\n", self.timestamp));
        output.push_str(&format!("Binary: {}\n", self.binary_info.path));

        if let Some(build_id) = &self.binary_info.build_id {
            output.push_str(&format!("Build ID: {}\n", build_id));
        }

        output.push_str("\n=== Stack Trace ===\n\n");

        for thread in &self.threads {
            output.push_str(&format!("Thread {} ({})\n", thread.id, thread.name));

            if thread.crashed {
                output.push_str("  [CRASHED]\n");
            }

            for (i, frame) in thread.frames.iter().enumerate() {
                output.push_str(&format!("  #{:2} {}\n", i, frame));
            }

            output.push('\n');
        }

        output
    }

    pub fn save<P: AsRef<Path>>(&self, path: P) -> std::io::Result<()> {
        std::fs::write(path, self.format())
    }
}

fn read_build_id() -> Option<String> {
    // Read .note.gnu.build-id section
    // Implementation depends on platform
    None
}
```

---

*This document is part of the Gimli-rs exploration series. See [exploration.md](./exploration.md) for the complete index.*
