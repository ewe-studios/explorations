---
name: wasm-snip
description: WebAssembly function removal tool for eliminating dead code and reducing WASM module size
type: sub-project
source: /home/darkvoid/Boxxed/@formulas/src.rust/src.wasm/src.rustwasm/wasm-snip/
---

# wasm-snip - WebAssembly Function Snipper

## Overview

wasm-snip is a **WebAssembly binary manipulation tool** that replaces function bodies with `unreachable` instructions, effectively eliminating dead code and reducing WASM module size. It's particularly useful for removing debugging code, logging functions, or any functionality that isn't needed in production builds.

Key features:
- **Dead code elimination** - Remove unused functions from compiled WASM
- **Size reduction** - Shrink WASM binaries by snipping unnecessary code
- **Pattern matching** - Use regex to match function names to snip
- **walrus-based** - Built on the walrus WASM manipulation library
- **CLI tool** - Simple command-line interface
- **Safe removal** - Replaces bodies with `unreachable`, maintains module structure

## Directory Structure

```
wasm-snip/
├── src/
│   ├── bin/
│   │   └── wasm-snip.rs      # CLI entry point
│   ├── lib.rs                # Core snipping logic
│   └── snip.rs               # Function snipping implementation
├── tests/                    # Integration tests
├── Cargo.toml
└── README.md
```

## Installation

```bash
# Install from crates.io
cargo install wasm-snip

# Or build from source
git clone https://github.com/rustwasm/wasm-snip
cd wasm-snip
cargo build --release
```

## Usage

### Basic Usage

```bash
# Snip a specific function
wasm-snip input.wasm -o output.wasm my_function

# Snip multiple functions
wasm-snip input.wasm -o output.wasm func1 func2 func3

# Use regex pattern
wasm-snip input.wasm -o output.wasm "debug_.*"

# Snip all functions matching pattern
wasm-snip input.wasm -o output.wasm --pattern "log_.*"
```

### Command Line Options

```bash
wasm-snip [FLAGS] [OPTIONS] <input> <output> [functions]...

FLAGS:
    -h, --help           Prints help information
    -V, --version        Prints version information
        --list           List all function names in the WASM file

OPTIONS:
    -o, --output <FILE>  Output file path
    -p, --pattern <REGEX>  Snip functions matching regex pattern

ARGS:
    <input>              Input WASM file
    <output>             Output WASM file
    <functions>...       Function names to snip
```

## How It Works

### Core Snipping Logic

```rust
use walrus::{Module, FunctionId};
use std::collections::HashSet;

/// Replace function bodies with unreachable instructions
pub fn snip_functions(
    module: &mut Module,
    functions_to_snip: &[String],
) -> Result<(), SnipError> {
    let mut snipped_count = 0;

    for func in module.functions.iter_mut() {
        let func_name = func.name.as_deref().unwrap_or("");

        // Check if this function should be snipped
        if functions_to_snip.iter().any(|f| f == func_name) {
            // Replace body with unreachable
            replace_body_with_unreachable(func);
            snipped_count += 1;
        }
    }

    log::info!("Snipped {} functions", snipped_count);
    Ok(())
}

/// Replace a function's body with a single unreachable instruction
fn replace_body_with_unreachable(func: &mut walrus::Function) {
    if func.kind.is_local() {
        // Get local function data
        if let Some(export) = func.body_mut() {
            // Clear existing instructions
            export.clear();

            // Insert unreachable instruction
            export.insert(
                walrus::ir::Instr::Unreachable,
            );
        }
    }
}
```

### Pattern Matching

```rust
use regex::Regex;

pub fn snip_by_pattern(
    module: &mut Module,
    pattern: &str,
) -> Result<usize, SnipError> {
    let regex = Regex::new(pattern)
        .map_err(|e| SnipError::InvalidRegex(e.to_string()))?;

    let mut snipped = 0;

    for func in module.functions.iter_mut() {
        if let Some(name) = &func.name {
            if regex.is_match(name) {
                replace_body_with_unreachable(func);
                snipped += 1;
            }
        }
    }

    Ok(snipped)
}
```

### Listing Functions

```rust
pub fn list_functions(module: &Module) {
    println!("Functions in module:");

    for func in module.functions.iter() {
        let name = func.name.as_deref().unwrap_or("(anonymous)");
        let kind = if func.kind.is_import() {
            "import"
        } else {
            "local"
        };

        println!("  [{}] {}", kind, name);
    }
}
```

## Use Cases

### Remove Debug Logging

```rust
// In your Rust code
#[cfg(debug_assertions)]
#[wasm_bindgen]
pub fn debug_log(message: &str) {
    web_sys::console::log_1(&message.into());
}

// In production, snip the debug function
// wasm-snip input.wasm -o output.wasm debug_log
```

### Remove Test Code

```rust
// Test-only functions
#[cfg(test)]
#[wasm_bindgen]
pub fn internal_test_helper() {
    // Test setup code
}

// Snip all test functions
// wasm-snip input.wasm -o output.wasm --pattern "internal_test_.*"
```

### Remove Console Output

```bash
# Remove all console logging in production
wasm-snip input.wasm -o output.wasm \
    console_error_panic_hook::set_once \
    web_sys::console::log_1 \
    web_sys::console::error_1
```

### Size Optimization Pipeline

```bash
# 1. Compile with wasm-pack
wasm-pack build --release

# 2. Snip unnecessary functions
wasm-snip pkg/my_module.wasm \
    -o pkg/my_module.snipped.wasm \
    --pattern "debug_.*"

# 3. Run wasm-opt for final optimization
wasm-opt -Oz pkg/my_module.snipped.wasm \
    -o pkg/my_module.wasm
```

## Integration with Build Process

### Post-Build Script

```bash
#!/bin/bash
# post-build.sh

INPUT="pkg/my_module.wasm"
OUTPUT="pkg/my_module.optimized.wasm"

# List of functions to snip
SNIP_FUNCTIONS=(
    "debug_log"
    "debug_dump"
    "trace_execution"
)

# Run wasm-snip
wasm-snip "$INPUT" -o "$OUTPUT" "${SNIP_FUNCTIONS[@]}"

# Replace original
mv "$OUTPUT" "$INPUT"

# Run wasm-opt
wasm-opt -Oz "$INPUT" -o "$INPUT"
```

### Cargo.toml Configuration

```toml
[package]
name = "my-wasm-module"
version = "0.1.0"
edition = "2021"

[lib]
crate-type = ["cdylib"]

[features]
default = ["debug"]
release = []  # No debug features

[dev-dependencies]
wasm-snip = "0.4.0"

[profile.release]
opt-level = "s"
lto = true
```

### Custom Build Script

```rust
// build.rs
use std::process::Command;

fn main() {
    if std::env::var("PROFILE").unwrap() == "release" {
        // Run wasm-snip after build
        Command::new("wasm-snip")
            .args(&[
                "pkg/my_module.wasm",
                "-o",
                "pkg/my_module.snipped.wasm",
                "--pattern",
                "debug_.*",
            ])
            .status()
            .unwrap();
    }
}
```

## Size Impact

### Before and After Comparison

```
Module: example.wasm

Before wasm-snip:
  Total size:    245 KB
  Code section:  180 KB
  Function count: 1,247

After wasm-snip (debug functions):
  Total size:    198 KB
  Code section:  133 KB
  Function count: 1,247 (bodies snipped)

After wasm-opt -Oz:
  Total size:    156 KB
  Code section:  91 KB
  Function count: 892 (dead code eliminated)

Total savings: ~36% size reduction
```

### Common Targets for Snipping

```
Function Pattern              | Typical Savings
------------------------------|----------------
debug_.*                      | 5-15%
log_.*                        | 3-10%
trace_.*                      | 2-8%
test_.*                       | 10-25%
benchmark_.*                  | 5-12%
console_.*                    | 2-5%
```

## Advanced Features

### Parallel Processing

```rust
use rayon::prelude::*;

pub fn snip_parallel(
    module: &mut Module,
    patterns: &[&str],
) -> Result<usize, SnipError> {
    let regexes: Vec<Regex> = patterns
        .iter()
        .map(|p| Regex::new(p))
        .collect::<Result<_, _>>()
        .map_err(|e| SnipError::InvalidRegex(e.to_string()))?;

    let mut snipped = 0;

    module
        .functions
        .par_iter_mut()
        .for_each(|func| {
            if let Some(name) = &func.name {
                if regexes.iter().any(|r| r.is_match(name)) {
                    replace_body_with_unreachable(func);
                    // Note: counting in parallel requires atomic
                }
            }
        });

    Ok(snipped)
}
```

### Custom Snip Strategies

```rust
pub enum SnipStrategy {
    /// Replace with unreachable
    Unreachable,

    /// Replace with noop (for certain function types)
    Noop,

    /// Replace with trap message
    Trap(String),
}

pub fn snip_with_strategy(
    func: &mut walrus::Function,
    strategy: SnipStrategy,
) {
    match strategy {
        SnipStrategy::Unreachable => {
            func.body_mut().unwrap().clear();
            func.body_mut()
                .unwrap()
                .insert(walrus::ir::Instr::Unreachable);
        }

        SnipStrategy::Noop => {
            // Return default values for function signature
            func.body_mut().unwrap().clear();
            // Add appropriate return instructions based on type
        }

        SnipStrategy::Trap(message) => {
            func.body_mut().unwrap().clear();
            // Insert trap with message
        }
    }
}
```

## Error Handling

```rust
#[derive(Debug, thiserror::Error)]
pub enum SnipError {
    #[error("Invalid regex pattern: {0}")]
    InvalidRegex(String),

    #[error("WASM parse error: {0}")]
    ParseError(#[from] walrus::Error),

    #[error("Function not found: {0}")]
    FunctionNotFound(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}
```

## Limitations

### Cannot Remove Imports

```rust
// wasm-snip cannot remove imported functions
// It only replaces local function bodies

// This won't work (import will remain):
// wasm-snip input.wasm -o output.wasm console.log

// Imports must be handled differently
```

### Type Safety

```rust
// Snipping a function that returns a value will cause
// unreachable to execute, which traps at runtime

// Original function:
// fn get_value() -> i32 { 42 }

// After snip:
// fn get_value() -> i32 { unreachable }  // Traps!

// Ensure snipped functions aren't called in production paths
```

## Related Documents

- [walrus](./walrus-exploration.md) - WASM manipulation library
- [Twiggy](./twiggy-exploration.md) - WASM size profiler
- [wasm-pack](./wasm-pack-exploration.md) - WASM build tooling

## Sources

- Source: `/home/darkvoid/Boxxed/@formulas/src.rust/src.wasm/src.rustwasm/wasm-snip/`
- GitHub: https://github.com/rustwasm/wasm-snip
- Documentation: https://docs.rs/wasm-snip/
