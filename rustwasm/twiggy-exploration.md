---
name: Twiggy
description: WebAssembly code size profiler for analyzing and optimizing WASM binaries
type: sub-project
source: /home/darkvoid/Boxxed/@formulas/src.rust/src.wasm/src.rustwasm/twiggy/
---

# Twiggy - WebAssembly Code Size Profiler

## Overview

Twiggy is a **code size profiler for WebAssembly** that analyzes WASM binaries to identify what functions, types, and code paths are contributing most to the final binary size. It helps developers optimize their WASM output by providing detailed breakdowns and actionable insights.

Key features:
- **Function analysis** - Size breakdown by function
- **Call graph visualization** - Understand call relationships
- **Diff comparison** - Compare two WASM files
- **Garbage detection** - Find dead code
- **Multiple output formats** - Terminal, JSON, DOT graphs
- **Source mapping** - Map back to Rust source

## Directory Structure

```
twiggy/
├── crates/
│   ├── twiggy/                 # Main library
│   ├── twiggy-cli/             # Command-line interface
│   ├── twiggy-opt/             # Optimization suggestions
│   └── twiggy-gc/              # Garbage collection analysis
├── src/
│   ├── dominators.rs           # Dominator tree analysis
│   ├── ir.rs                   # Intermediate representation
│   ├── parser.rs               # WASM parsing
│   ├── top.rs                  # Top functions report
│   ├── paths.rs                # Call path analysis
│   ├── monos.rs                # Monomorphization analysis
│   └── diff.rs                 # Diff comparison
├── tests/
├── Cargo.toml
└── README.md
```

## Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                      Twiggy Architecture                        │
└─────────────────────────────────────────────────────────────────┘
                            │
                            ▼
┌─────────────────────────────────────────────────────────────────┐
│  1. Parse WASM Binary                                           │
│     - Read sections (type, function, code, export, etc.)        │
│     - Build function map                                        │
│     - Extract debug info (if available)                         │
└─────────────────────────────────────────────────────────────────┘
                            │
                            ▼
┌─────────────────────────────────────────────────────────────────┐
│  2. Build Call Graph                                            │
│     - Analyze call instructions                                 │
│     - Build dominator tree                                      │
│     - Identify unreachable code                                 │
└─────────────────────────────────────────────────────────────────┘
                            │
                            ▼
┌─────────────────────────────────────────────────────────────────┐
│  3. Size Analysis                                               │
│     - Calculate function sizes                                  │
│     - Attribute size to source locations                        │
│     - Identify monomorphization bloat                           │
└─────────────────────────────────────────────────────────────────┘
                            │
                            ▼
┌─────────────────────────────────────────────────────────────────┐
│  4. Generate Reports                                            │
│     - Top functions by size                                     │
│     - Call path analysis                                        │
│     - Diff between versions                                     │
│     - Optimization suggestions                                  │
└─────────────────────────────────────────────────────────────────┘
```

## Command-Line Interface

### Top Functions

```bash
# Show top functions by size
twiggy top my_module.wasm

# Show top 20 functions
twiggy top -n 20 my_module.wasm

# Include byte percentages
twiggy top --percentages my_module.wasm

# Output as JSON
twiggy top --format json my_module.wasm

# Output as CSV
twiggy top --format csv my_module.wasm
```

Example output:
```
 Shallow Bytes │ Shallow % │ Name
───────────────┼───────────┼──────────────────────────────────────
          1024 │     5.12% │ my_crate::expensive_function
           512 │     2.56% │ <my_crate::Type as Trait>::impl
           256 │     1.28% │ core::fmt::write
           128 │     0.64% │ wasm_bindgen::__wbindgen_malloc
           100 │     0.50% │ [others]
         17980 │    89.90% │ [...]
         20000 │   100.00% │ Σ [150 Total]
```

### Call Paths

```bash
# Show call paths to functions
twiggy paths my_module.wasm

# Show paths to specific function
twiggy paths -g "expensive_function" my_module.wasm

# Limit path depth
twiggy paths --max-depth 5 my_module.wasm
```

Example output:
```
 Bytes │     % │ Path
───────┼───────┼─────────────────────────────────────────────────
   256 │ 1.28% │ <my_crate::App as wasm_bindgen::convert::FromWasmAbi>::from_abi
       │       │   └── my_crate::expensive_function
       │       │       └── core::fmt::write
   128 │ 0.64% │ my_crate::main
       │       │   └── my_crate::expensive_function
       │       │       └── core::fmt::write
```

### Monomorphization Analysis

```bash
# Show monomorphized generics
twiggy monos my_module.wasm

# Group by original function
twiggy monos --group-by my_module.wasm
```

Example output:
```
 Bytes │     % │ Name
───────┼───────┼─────────────────────────────────────────────────
  1024 │ 5.12% │ my_crate::generic_fn<T>
       │       │   ├── my_crate::generic_fn<my_crate::TypeA> (512 bytes)
       │       │   └── my_crate::generic_fn<my_crate::TypeB> (512 bytes)
   512 │ 2.56% │ core::default::default_for<T>
       │       │   ├── core::default::default_for<String> (256 bytes)
       │       │   └── core::default::default_for<Vec<u8>> (256 bytes)
```

### Diff Comparison

```bash
# Compare two WASM files
twiggy diff old.wasm new.wasm

# Show only added functions
twiggy diff --added-only old.wasm new.wasm

# Show only removed functions
twiggy diff --removed-only old.wasm new.wasm

# Output as JSON
twiggy diff --format json old.wasm new.wasm
```

Example output:
```
┌──────┬───────────────┬───────────────┬──────────────┐
│      │ Old Size      │ New Size      │ Change       │
├──────┼───────────────┼───────────────┼──────────────┤
│ +128 │ -             │ 128 bytes     │ +128 bytes   │
│      │               │ new_function  │              │
├──────┼───────────────┼───────────────┼──────────────┤
│ -64  │ 64 bytes      │ -             │ -64 bytes    │
│      │ old_function  │               │              │
├──────┼───────────────┼───────────────┼──────────────┤
│ ±0   │ 1024 bytes    │ 1024 bytes    │ 0 bytes      │
│      │ unchanged_fn  │ unchanged_fn  │              │
└──────┴───────────────┴───────────────┴──────────────┘
```

### Garbage Detection

```bash
# Find unreachable code
twiggy garbage my_module.wasm

# Show garbage with size threshold
twiggy garbage --min-bytes 100 my_module.wasm
```

## Programmatic API

### Basic Usage

```rust
use twiggy::{analyze, AnalysisOptions};

// Analyze WASM file
let options = AnalysisOptions::default();
let report = analyze::file("my_module.wasm", &options)?;

// Get top functions
for item in report.top().items() {
    println!("{}: {} bytes ({}%)",
        item.name(),
        item.size(),
        item.percentage()
    );
}

// Get dominator tree
let dominators = report.dominators();
for (dominator, size) in dominators {
    println!("{} dominates {} bytes", dominator.name(), size);
}
```

### Custom Analysis

```rust
use twiggy::{ir, parser, dominators};

// Parse WASM
let mut items = ir::Items::default();
parser::parse_wasm_file("my_module.wasm", &mut items)?;

// Build dominator tree
let dom_tree = dominators::build(&items);

// Find largest dominators
let mut dominators: Vec<_> = dom_tree.iter().collect();
dominators.sort_by(|a, b| b.size().cmp(&a.size()));

for dom in dominators.iter().take(10) {
    println!("Dominator: {} ({} bytes)", dom.name(), dom.size());

    // Print children
    for child in dom.children() {
        println!("  └── {} ({} bytes)", child.name(), child.size());
    }
}
```

### Monomorphization Detection

```rust
use twiggy::monos;

let options = monos::Options::default();
let report = monos::analyze("my_module.wasm", &options)?;

// Group by generic function
let mut groups: std::collections::HashMap<_, Vec<_>> =
    std::collections::HashMap::new();

for item in report.items() {
    let generic = item.generic_name();
    groups.entry(generic).or_default().push(item);
}

for (generic, instances) in groups {
    let total: usize = instances.iter().map(|i| i.size()).sum();
    println!("{}: {} bytes ({} instances)",
        generic,
        total,
        instances.len()
    );
}
```

## Optimization Strategies

### Identifying Bloat

```bash
# Find largest functions
twiggy top my_module.wasm | head -20

# Find most monomorphized generics
twiggy monos my_module.wasm

# Find dead code
twiggy garbage my_module.wasm
```

### Common Issues

#### 1. Panicking Overhead

```bash
# Check panic-related code
twiggy top my_module.wasm | grep -i panic
```

Fix in Cargo.toml:
```toml
[profile.release]
panic = "abort"  # Instead of "unwind"
```

#### 2. Debug Symbols

```bash
# Check debug info size
twiggy top my_module.wasm | grep -E "(debug|__debug)"
```

Fix:
```toml
[profile.release]
debug = false  # Remove debug info
```

Or strip after build:
```bash
wasm-strip my_module.wasm
```

#### 3. Monomorphization Bloat

```bash
# Check generic bloat
twiggy monos my_module.wasm
```

Fix in Rust code:
```rust
// Bad: Generic bloat
fn process<T: Processable>(item: T) {
    // ... lots of code ...
}

// Good: Dynamic dispatch
fn process(item: &dyn Processable) {
    // ... same code, single instance ...
}
```

#### 4. Unused Features

```bash
# Check what features add
twiggy diff without-feature.wasm with-feature.wasm
```

Fix in Cargo.toml:
```toml
[dependencies]
# Bad: All features
serde = "1.0"

# Good: Only needed features
serde = { version = "1.0", features = ["derive"], default-features = false }
```

### Size Optimization Pipeline

```bash
# 1. Build with optimizations
cargo build --release --target wasm32-unknown-unknown

# 2. Analyze size
twiggy top target/wasm32-unknown-unknown/release/my_module.wasm

# 3. Strip debug info
wasm-strip target/wasm32-unknown-unknown/release/my_module.wasm

# 4. Optimize with wasm-opt
wasm-opt -Oz target/wasm32-unknown-unknown/release/my_module.wasm -o optimized.wasm

# 5. Verify optimization
twiggy diff target/wasm32-unknown-unknown/release/my_module.wasm optimized.wasm
```

## Integration with CI/CD

### Size Budget Check

```yaml
# .github/workflows/size-check.yml
name: WASM Size Check

on: [pull_request]

jobs:
  size-check:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3

      - name: Install Rust
        uses: dtolnay/rust-action@stable
        with:
          targets: wasm32-unknown-unknown

      - name: Install twiggy
        run: cargo install twiggy

      - name: Build WASM
        run: cargo build --release --target wasm32-unknown-unknown

      - name: Check size
        run: |
          twiggy top target/wasm32-unknown-unknown/release/my_module.wasm > size_report.txt
          SIZE=$(wc -c < target/wasm32-unknown-unknown/release/my_module.wasm)
          if [ $SIZE -gt 100000 ]; then
            echo "WASM size ($SIZE bytes) exceeds budget (100KB)"
            exit 1
          fi
```

### Size Trend Tracking

```yaml
- name: Track size trend
  run: |
    twiggy top target/wasm32-unknown-unknown/release/my_module.wasm \
      --format json > size_data.json

- name: Upload size data
  uses: actions/upload-artifact@v3
  with:
    name: wasm-size
    path: size_data.json
```

## Related Documents

- [wasm-pack](./wasm-pack-exploration.md) - Build tooling
- [wasm-tools](../wasmtime/wasm-tools-exploration.md) - WASM optimization
- [wasm-bindgen](./wasm-bindgen-exploration.md) - Rust/JS bindings

## Sources

- Source: `/home/darkvoid/Boxxed/@formulas/src.rust/src.wasm/src.rustwasm/twiggy/`
- Twiggy Documentation: https://rustwasm.github.io/twiggy/
- Twiggy GitHub: https://github.com/rustwasm/twiggy
