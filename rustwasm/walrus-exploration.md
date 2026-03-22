---
name: Walrus
description: WebAssembly manipulation library for transforming and analyzing WASM modules
type: sub-project
source: /home/darkvoid/Boxxed/@formulas/src.rust/src.wasm/src.rustwasm/walrus/
---

# Walrus - WebAssembly Manipulation Library

## Overview

Walrus is a **high-level WebAssembly manipulation library** that provides an easy-to-use API for parsing, transforming, and emitting WASM modules. Unlike low-level binary parsers, Walrus works with a structured intermediate representation (IR) that makes it easy to analyze and modify WASM programs.

Key features:
- **High-level IR** - Structured representation of WASM modules
- **Easy transformation** - Modify functions, types, exports
- **Debug info preservation** - Maintain source maps through transforms
- **Module linking** - Combine multiple WASM modules
- **Analysis utilities** - Call graphs, type inference
- **Code generation** - Emit valid WASM after modifications

## Directory Structure

```
walrus/
├── src/
│   ├── ir/                     # Intermediate representation
│   │   ├── mod.rs              # Core IR types
│   │   ├── function.rs         # Function representation
│   │   ├── module.rs           # Module structure
│   │   ├── types.rs            # Type definitions
│   │   └── expr.rs             # Expression trees
│   ├── parse/                  # WASM parsing
│   ├── emit/                   # WASM emission
│   ├── transform/              # Transformation utilities
│   │   ├── gc.rs               # Dead code elimination
│   │   ├── inlining.rs         # Function inlining
│   │   └── optimize.rs         # Optimization passes
│   ├── module.rs               # Module API
│   ├── func.rs                 # Function handling
│   └── lib.rs                  # Crate root
├── tests/
├── Cargo.toml
└── README.md
```

## Core Concepts

### Module IR

```rust
use walrus::{Module, ModuleConfig};

// Parse a WASM file
let module = ModuleConfig::new()
    .generate_dwarf(true)  // Preserve debug info
    .parse("input.wasm")?;

// Module contains:
// - types: TypeSection (function signatures)
// - imports: ImportSection
// - functions: FunctionSection
// - tables: TableSection
// - memories: MemorySection
// - globals: GlobalSection
// - exports: ExportSection
// - start: Option<FunctionId>
// - data: DataSection
// - elements: ElementSection
```

### Function IR

```rust
use walrus::{Function, FunctionKind, Locals, Instr};

// Get a function
let func = module.functions.get(function_id);

// Function structure
struct Function {
    kind: FunctionKind,  // Local or Imported
    name: Option<String>,
    ty: TypeId,          // Function type
    locals: Locals,      // Local variables
    body: Option<Expr>,  // Function body (expression tree)
}

// Iterate over function instructions
for (_block, instr) in func.body.iter() {
    match instr {
        Instr::I32Const { val } => { /* ... */ }
        Instr::Call { func } => { /* ... */ }
        Instr::Binary { op, left, right } => { /* ... */ }
        _ => {}
    }
}
```

### Expression Trees

```rust
use walrus::ir::*;

// Expression tree representation
enum Expr {
    // Constants
    I32Const(i32),
    I64Const(i64),
    F32Const(f32),
    F64Const(f64),

    // Local variables
    LocalGet(LocalId),
    LocalSet(LocalId, Box<Expr>),
    LocalTee(LocalId, Box<Expr>),

    // Operations
    Binary(BinaryOp, Box<Expr>, Box<Expr>),
    Unary(UnaryOp, Box<Expr>),

    // Control flow
    Block(Block),
    If(If),
    Loop(Loop),
    Br(LabelId),
    BrIf(LabelId),
    Return(Option<Box<Expr>>),

    // Calls
    Call(FunctionId, Vec<Expr>),
    CallIndirect(TypeId, Vec<Expr>),

    // Memory
    Load(Load, Box<Expr>),
    Store(Store, Box<Expr>, Box<Expr>),
}
```

## Common Transformations

### Remove Unused Functions

```rust
use walrus::{Module, FunctionId};
use std::collections::HashSet;

fn remove_unused_functions(module: &mut Module) {
    // Find all used functions
    let mut used = HashSet::new();

    // Mark exported functions
    for export in module.exports.iter() {
        if let walrus::ExportItem::Function(id) = export.item {
            used.insert(id);
        }
    }

    // Mark start function
    if let Some(start) = module.start {
        used.insert(start);
    }

    // Mark functions called from other functions
    for func in module.functions.iter() {
        for (_block, instr) in func.body.iter() {
            if let walrus::ir::Instr::Call { func: id } = instr {
                used.insert(*id);
            }
        }
    }

    // Remove unused
    let to_remove: Vec<_> = module
        .functions
        .iter()
        .filter(|f| !used.contains(&f.id()))
        .map(|f| f.id())
        .collect();

    for id in to_remove {
        module.functions.delete(id);
    }
}
```

### Rename Functions

```rust
use walrus::{Module, FunctionId};

fn demangle_rust_functions(module: &mut Module) {
    for func in module.functions.iter_mut() {
        if let Some(name) = &func.name {
            // Demangle Rust symbol
            if let Ok(demangled) = rustc_demangle::try_demangle(name) {
                func.name = Some(demangled.to_string());
            }
        }
    }
}

fn minify_function_names(module: &mut Module) {
    let mut counter = 0;

    for func in module.functions.iter_mut() {
        if !is_exported(module, func.id()) {
            func.name = Some(format!("f{}", counter));
            counter += 1;
        }
    }
}
```

### Inject Logging

```rust
use walrus::{Module, FunctionId, ir::*};
use walrus::ir::instr::*;

fn inject_logging(module: &mut Module) {
    let log_import = add_log_import(module);

    for func in module.functions.iter_mut() {
        if func.kind.is_import() {
            continue;
        }

        // Add log at function entry
        let body = func.body.as_mut().unwrap();
        let mut new_body = Vec::new();

        // Log function name
        if let Some(name) = &func.name {
            new_body.push(Instr::MemoryString(name.clone()));
            new_body.push(Instr::Call { func: log_import });
        }

        // Original body
        new_body.extend(body.iter().cloned());

        *body = new_body;
    }
}

fn add_log_import(module: &mut Module) -> FunctionId {
    // Add import for host logging function
    let ty = module.types.add(&[], &[]);
    module.funcs.add_import("env", "log", ty)
}
```

### Add Wrappers

```rust
use walrus::{Module, ModuleConfig, TypeId};

// Wrap a function to add error handling
fn add_error_wrapper(module: &mut Module, target: FunctionId) -> FunctionId {
    let target_func = module.functions.get(target);
    let ty = target_func.ty;

    // Get type signature
    let (params, results) = get_type_signature(module, ty);

    // Create wrapper function
    let mut wrapper = walrus::FunctionBuilder::new(&mut module.types, &params, &results);

    // Try block
    let mut body = wrapper.func_body();

    // Call original function
    for (i, _) in params.iter().enumerate() {
        body.local_get(i as u32);
    }
    body.call(target);

    // Return result
    // (results are automatically returned)

    wrapper.finish(vec![], &mut module.funcs)
}
```

## Module Analysis

### Call Graph

```rust
use walrus::{Module, FunctionId};
use std::collections::{HashMap, HashSet};

struct CallGraph {
    edges: HashMap<FunctionId, HashSet<FunctionId>>,
}

impl CallGraph {
    fn build(module: &Module) -> Self {
        let mut edges = HashMap::new();

        for func in module.functions.iter() {
            let callers = edges.entry(func.id()).or_insert_with(HashSet::new);

            for (_block, instr) in func.body.iter() {
                if let walrus::ir::Instr::Call { func: callee } = instr {
                    callers.insert(*callee);
                }
            }
        }

        CallGraph { edges }
    }

    fn callers(&self, func: FunctionId) -> &HashSet<FunctionId> {
        self.edges.get(&func).unwrap()
    }

    fn callees(&self, func: FunctionId) -> Vec<FunctionId> {
        self.edges
            .iter()
            .filter(|(_, callers)| callers.contains(&func))
            .map(|(caller, _)| *caller)
            .collect()
    }
}
```

### Type Inference

```rust
use walrus::{Module, TypeId, ValType};

fn infer_types(module: &Module) -> HashMap<FunctionId, Vec<ValType>> {
    let mut types = HashMap::new();

    for func in module.functions.iter() {
        let ty = module.types.get(func.ty);
        let params: Vec<ValType> = ty.params().iter().copied().collect();
        types.insert(func.id(), params);
    }

    types
}
```

### Size Analysis

```rust
use walrus::{Module};

struct SizeAnalysis {
    total_code_size: usize,
    function_sizes: HashMap<String, usize>,
}

impl SizeAnalysis {
    fn analyze(module: &Module) -> Self {
        let mut total = 0;
        let mut sizes = HashMap::new();

        for func in module.functions.iter() {
            if let Some(body) = &func.body {
                let size = estimate_size(body);
                total += size;

                let name = func.name.clone().unwrap_or_else(|| "anonymous".to_string());
                sizes.insert(name, size);
            }
        }

        SizeAnalysis {
            total_code_size: total,
            function_sizes: sizes,
        }
    }
}

fn estimate_size(body: &walrus::ir::Expr) -> usize {
    // Rough estimate based on instruction count
    body.iter().count() * 2  // ~2 bytes per instruction
}
```

## Module Linking

### Combine Modules

```rust
use walrus::{Module, ModuleConfig};

fn link_modules(modules: Vec<Module>) -> Module {
    let mut result = modules.into_iter().next().unwrap();

    for module in modules.into_iter().skip(1) {
        // Merge types
        for ty in module.types.iter() {
            result.types.add(ty);
        }

        // Merge functions
        for func in module.functions.iter() {
            if func.kind.is_import() {
                // Re-import
                result.funcs.add_import(
                    func.module.as_deref().unwrap_or(""),
                    func.name.as_deref().unwrap_or(""),
                    func.ty,
                );
            } else {
                // Add local function
                result.funcs.add_local(func.ty, func.body.clone());
            }
        }

        // Merge exports
        for export in module.exports.iter() {
            result.exports.add(export.name.clone(), export.item);
        }

        // Merge memories
        for memory in module.memories.iter() {
            result.memories.add_local(memory.kind, memory.shared);
        }
    }

    result
}
```

### Import Resolution

```rust
use walrus::{Module, Import, ImportKind};

fn resolve_imports(module: &mut Module, imports: &Module) {
    for import in module.imports.iter_mut() {
        if let Some(resolved) = find_import(imports, &import.module, &import.name) {
            import.item = resolved.item;
        }
    }
}

fn find_import(module: &Module, mod_name: &str, name: &str) -> Option<Import> {
    module.imports.iter().find(|i| {
        i.module.as_deref() == Some(mod_name) && i.name.as_deref() == Some(name)
    }).cloned()
}
```

## Code Generation

### Emit WASM

```rust
use walrus::{Module, ModuleConfig};

// After transformations, emit the module
fn emit_wasm(module: &Module) -> Vec<u8> {
    let mut wasm_bytes = Vec::new();
    module.emit_wasm(&mut wasm_bytes).unwrap();
    wasm_bytes
}

// Emit with options
fn emit_optimized(module: &Module) -> Vec<u8> {
    ModuleConfig::new()
        .generate_name_section(false)
        .generate_dwarf(false)
        .emit(module)
}
```

### Generate Source Maps

```rust
use walrus::{Module, source_map::SourceMap};

fn generate_source_map(module: &Module) -> SourceMap {
    let mut source_map = SourceMap::new();

    for func in module.functions.iter() {
        if let Some(name) = &func.name {
            // Map function positions
            for (offset, instr) in func.body.iter() {
                source_map.add_mapping(*offset, name, instr.line_number());
            }
        }
    }

    source_map
}
```

## Use Cases

### wasm-bindgen Integration

```rust
use walrus::Module;

// wasm-bindgen uses walrus internally for:
// - Adding JS glue code
// - Modifying exports
// - Adding imports for JS interop
// - Transforming types

fn process_for_wasm_bindgen(module: &mut Module) {
    // Add externref table
    module.tables.add_local(TableKind::ExternRef, false, 0, None);

    // Add wrapper functions for type conversion
    add_type_conversions(module);

    // Modify exports to handle JS types
    wrap_exports_for_js(module);
}
```

### Custom Sections

```rust
use walrus::{Module, CustomSection, CustomSectionId};

// Add custom section
fn add_metadata(module: &mut Module, metadata: &str) {
    let id = module.customs.add(CustomSection {
        name: "metadata".to_string(),
        data: metadata.as_bytes().to_vec(),
    });
}

// Read custom section
fn read_metadata(module: &Module) -> Option<String> {
    module.customs
        .iter()
        .find(|(_, s)| s.name() == "metadata")
        .map(|(_, s)| String::from_utf8_lossy(s.data()).to_string())
}
```

## Related Documents

- [wasm-tools](../wasmtime/wasm-tools-exploration.md) - WASM tooling
- [twiggy](./twiggy-exploration.md) - Size analysis
- [wasm-bindgen](./wasm-bindgen-exploration.md) - Rust/JS bindings

## Sources

- Source: `/home/darkvoid/Boxxed/@formulas/src.rust/src.wasm/src.rustwasm/walrus/`
- Walrus Documentation: https://docs.rs/walrus/
- Walrus GitHub: https://github.com/rustwasm/walrus
