---
name: Wasm Tools
description: Command-line utilities and libraries for WebAssembly binary manipulation and analysis
type: sub-project
source: /home/darkvoid/Boxxed/@formulas/src.rust/src.wasm/src.wasmtime/wasm-tools/
---

# Wasm Tools - WebAssembly Binary Tooling

## Overview

Wasm Tools is a **comprehensive suite of command-line utilities and libraries** for inspecting, manipulating, and transforming WebAssembly binaries. It provides essential tooling for:

- Binary inspection and debugging
- Text format conversion (WAT ↔ Wasm)
- Component model support
- WASM shrinking and optimization
- Validation and linting
- Stack switching support
- DWARF debugging information

The tools are built as both a CLI and reusable Rust crates.

## Directory Structure

```
wasm-tools/
├── src/
│   ├── bin/
│   │   └── wasm-tools/      # CLI entry point
│   ├── lib.rs               # Library root
│   ├── parser.rs            # Wasm parsing
│   ├── print.rs             # Binary printing
│   ├── validate.rs          # Validation
│   ├── component/           # Component model tools
│   ├── shrink.rs            # Binary shrinking
│   ├── strip.rs             # Section stripping
│   ├── demangle.rs          # Symbol demangling
│   └── dump.rs              # Section dumping
├── crates/
│   ├── wasmparser/          # Streaming WebAssembly parser
│   ├── wasm-encoder/        # WebAssembly bytecode builder
│   ├── wast/                # WAT text format parser
│   ├── wat/                 # WAT → Wasm converter
│   ├── wasm-mutate/         # Mutation testing
│   ├── wasm-shrink/         # Binary shrinking
│   └── wasm-component-ld/   # Component linker
├── Cargo.toml
├── README.md
└── fuzz/                    # Fuzzing infrastructure
```

## Command-Line Interface

### Main Commands

```bash
# Parse and validate
wasm-tools validate module.wasm

# Print binary in WAT format
wasm-tools print module.wasm

# Parse WAT to binary
wasm-tools parse module.wat

# Demangle Rust symbols
wasm-tools demangle module.wasm

# Strip debug sections
wasm-tools strip module.wasm -o module.stripped.wasm

# Shrink binary size
wasm-tools shrink module.wasm

# Dump section contents
wasm-tools dump module.wasm

# Validate component
wasm-tools component validate component.wasm

# Print component structure
wasm-tools component print component.wasm
```

## Core Libraries

### Wasmparser (Streaming Parser)

```rust
use wasmparser::{Parser, Payload, Result};

let wasm = std::fs::read("module.wasm")?;

// Streaming parser
for payload in Parser::new(0).parse_all(&wasm) {
    match payload? {
        Payload::Version { num, encoding, range } => {
            println!("WASM version: {}", num);
        }
        Payload::TypeSection(section) => {
            for ty in section {
                println!("Type: {:?}", ty?);
            }
        }
        Payload::FunctionSection(section) => {
            for func in section {
                println!("Function type: {}", func?);
            }
        }
        Payload::ExportSection(section) => {
            for export in section {
                let exp = export?;
                println!("Export: {} -> {:?}", exp.name, exp.kind);
            }
        }
        Payload::CodeSectionEntry(body) => {
            println!("Function body: {} bytes", body.len());
        }
        _ => {}
    }
}
```

### Wasm-Encoder (Bytecode Builder)

```rust
use wasm_encoder::{
    Module, TypeSection, FunctionType, FuncType,
    FunctionSection, CodeSection, Function,
    ExportSection, ExportKind,
};

let mut module = Module::new();

// Type section
let mut types = TypeSection::new();
types.function().func_type(FuncType {
    params: vec![wasm_encoder::ValType::I32; 2],
    results: vec![wasm_encoder::ValType::I32],
});
module.section(&types);

// Function section
let mut funcs = FunctionSection::new();
funcs.function(0); // Type index 0
module.section(&funcs);

// Export section
let mut exports = ExportSection::new();
exports.export("add", ExportKind::Func, 0);
module.section(&exports);

// Code section
let mut code = CodeSection::new();
let mut func = Function::new(vec![wasm_encoder::ValType::I32; 2]);
func.instruction(&wasm_encoder::Instruction::LocalGet(0));
func.instruction(&wasm_encoder::Instruction::LocalGet(1));
func.instruction(&wasm_encoder::Instruction::I32Add);
func.instruction(&wasm_encoder::Instruction::End);
code.function(&func);
module.section(&code);

// Write binary
let wasm_bytes = module.finish();
std::fs::write("module.wasm", wasm_bytes)?;
```

### Wast (Text Format Parser)

```rust
use wast::{parser, QuoteWat, Wast, WastDirective, Wat};

let wat = r#"
    (module
        (func $add (param i32 i32) (result i32)
            local.get 0
            local.get 1
            i32.add
        )
        (export "add" (func $add))
    )
"#;

// Parse WAT
let wat = Wat::parse_str(wat)?;
let wasm_bytes = wat.encode()?;

// Parse full WAST with assertions
let wast = Wast::parse_str(wat)?;
for directive in wast.directives {
    match directive {
        WastDirective::Wat(QuoteWat::Wat(module)) => {
            let bytes = module.encode()?;
            // Run tests or assertions
        }
        WastDirective::AssertTrap { .. } => {
            // Expected trap
        }
        WastDirective::AssertReturn { .. } => {
            // Expected return value
        }
        _ => {}
    }
}
```

## Component Model Tools

### Component Creation

```rust
use wasm_encoder::{
    Component, ComponentTypeSection, ComponentFuncType,
    ComponentBuilder,
};

let mut component = Component::new();

// Type section
let mut types = ComponentTypeSection::new();
types.function(ComponentFuncType {
    params: vec![("x".into(), wasm_encoder::ValType::I32)],
    results: vec![wasm_encoder::ValType::I32],
});
component.section(&types);

// Core module section
let mut builder = ComponentBuilder::new();
builder.section(&wasm_encoder::CoreModuleSection(&core_module));
component.section(&builder.sections().next().unwrap());

// Write component
let bytes = component.finish();
std::fs::write("component.wasm", bytes)?;
```

### Component Inspection

```rust
use wasm_component_loader::Component;

let component = Component::from_file("component.wasm")?;

// Inspect imports
for import in component.imports() {
    println!("Import: {}.{}", import.module, import.name);
}

// Inspect exports
for export in component.exports() {
    println!("Export: {} -> {:?}", export.name, export.type_info());
}

// Get type information
for ty in component.types() {
    println!("Type: {:?}", ty);
}
```

## Binary Shrinking

```rust
use wasm_shrink::{WasmShrink, ShrinkOptions};

let wasm = std::fs::read("large.wasm")?;

let mut shrink = WasmShrink::default();
shrink.passes(100);  // Number of shrink passes
shrink.interesting_script("test.sh");  // Test script

let result = shrink.run(&wasm)?;

println!("Original size: {} bytes", wasm.len());
println!("Shrunk size: {} bytes", result.len());
println!("Reduction: {:.1}%",
    100.0 * (1.0 - result.len() as f64 / wasm.len() as f64));

std::fs::write("small.wasm", result)?;
```

## Mutation Testing

```rust
use wasm_mutate::{WasmMutate, MutationStrategy};

let wasm = std::fs::read("module.wasm")?;

let mut mutator = WasmMutate::default();
mutator.seed(42);
mutator.strategy(MutationStrategy::RemoveFunc);

for mutation in mutator.run(&wasm)? {
    let mutated_wasm = mutation?;

    // Validate mutated module
    wasmparser::Validator::new().validate(&mutated_wasm)?;

    // Run test suite to find bugs
    // ...
}
```

## DWARF Debugging

```rust
use wasmparser::{Parser, Payload};

let wasm = std::fs::read("debug.wasm")?;

for payload in Parser::new(0).parse_all(&wasm) {
    match payload? {
        Payload::CustomSection(reader) => {
            if reader.name() == ".debug_info" {
                println!("Found DWARF debug info");
                // Parse DWARF using gimli or similar
            }
            if reader.name() == ".debug_line" {
                println!("Found DWARF line table");
            }
        }
        _ => {}
    }
}
```

## Stack Switching Support

```rust
// Stack switching enables coroutines and async
let wasm = r#"
    (module
        (import "stack" "switch" (func $switch (param i32)))
        (func (export "run")
            i32.const 1
            call $switch  ;; Yield control
            i32.const 2
            call $switch  ;; Yield again
        )
    )
"#;
```

## Validation

```rust
use wasmparser::Validator;

let wasm = std::fs::read("module.wasm")?;

// Validate with default settings
Validator::new().validate(&wasm)?;

// Validate with custom settings
let mut validator = Validator::new_with_features(
    wasmparser::WasmFeatures {
        reference_types: true,
        simd: true,
        threads: true,
        component_model: false,
        ..Default::default()
    }
);

validator.validate(&wasm)?;
```

## Section Manipulation

### Stripping Sections

```rust
use wasm_encoder::{Module, RawSection};
use wasmparser::{Parser, Payload};

let wasm = std::fs::read("module.wasm")?;
let mut new_module = Module::new();

for payload in Parser::new(0).parse_all(&wasm) {
    match payload? {
        Payload::CustomSection { .. } => {
            // Skip debug sections
            continue;
        }
        Payload::Section(section) => {
            new_module.section(&RawSection {
                id: section.id,
                data: section.data,
            });
        }
        _ => {}
    }
}

let stripped = new_module.finish();
std::fs::write("stripped.wasm", stripped)?;
```

### Adding Sections

```rust
use wasm_encoder::{Module, CustomSection};

let mut module = Module::new();

// Add custom section
module.section(&CustomSection {
    name: "my_metadata",
    data: b"version=1.0.0",
});

let wasm = module.finish();
```

## Symbol Demangling

```rust
use rustc_demangle::demangle;

let mangled = "_ZN4core3fmt5num72<$impl core..fmt..Display for i32>::3fmt17h1234567890abcdefE";
let demangled = demangle(mangled);

println!("Demangled: {}", demangled);
// Output: core::fmt::num::<impl core::fmt::Display for i32>::fmt::h1234567890abcdef
```

## Fuzzing Infrastructure

```rust
// Fuzz target for wasmparser
#![no_main]
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    for _ in wasmparser::Parser::new(0).parse_all(data) {
        // Just ensure parsing doesn't crash
    }
});
```

## Integration with Wasmtime

```rust
use wasmtime::{Engine, Module, Config};
use wasmparser::Validator;

// Pre-validate before compilation
let wasm = std::fs::read("module.wasm")?;
Validator::new().validate(&wasm)?;

// Compile with wasmtime
let engine = Engine::default();
let module = unsafe { Module::from_binary(&engine, &wasm) };
```

## Use Cases

### Size Optimization Pipeline

```bash
# 1. Strip debug info
wasm-tools strip input.wasm -o stripped.wasm

# 2. Shrink
wasm-tools shrink stripped.wasm -o shrunk.wasm

# 3. Validate
wasm-tools validate shrunk.wasm

# 4. Print final size
ls -l shrunk.wasm
```

### Debug Info Extraction

```bash
# Extract DWARF info
wasm-tools dump module.wasm > dump.txt

# Demangle symbols
wasm-tools demangle module.wasm > demangled.wat
```

### Component Workflow

```bash
# Create component from core module
wasm-tools component new core.wasm -o component.wasm

# Embed WASI adapter
wasm-tools component new core.wasm \
    --adapt wasi_snapshot_preview1=wasi_adapter.wasm \
    -o component.wasm

# Inspect component
wasm-tools component print component.wasm

# Validate component
wasm-tools component validate component.wasm
```

## Related Documents

- [Wasmtime](./wasmtime-runtime-exploration.md) - Runtime integration
- [Wit Bindgen](./wit-bindgen-exploration.md) - Interface bindings
- [WASI](./wasi-exploration.md) - System interface

## Sources

- Source: `/home/darkvoid/Boxxed/@formulas/src.rust/src.wasm/src.wasmtime/wasm-tools/`
- Wasm Tools GitHub: https://github.com/bytecodealliance/wasm-tools
- Bytecode Alliance: https://bytecodealliance.org/
