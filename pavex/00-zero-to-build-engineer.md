---
title: "Zero to Build Engineer: A First-Principles Journey Through Pavex"
subtitle: "Complete textbook-style guide from build system fundamentals to Rust code generation"
based_on: "pavex - Build-time transpiled web framework"
level: "Beginner to Intermediate - No prior build system knowledge assumed"
---

# Zero to Build Engineer: First-Principles Guide

## Table of Contents

1. [What Are Build Systems?](#1-what-are-build-systems)
2. [Compilation Fundamentals](#2-compilation-fundamentals)
3. [Code Generation at Build Time](#3-code-generation-at-build-time)
4. [rustdoc JSON as Reflection](#4-rustdoc-json-as-reflection)
5. [Incremental Compilation Basics](#5-incremental-compilation-basics)
6. [Your Learning Path](#6-your-learning-path)

---

## 1. What Are Build Systems?

### 1.1 The Fundamental Question

**What is a build system?**

A build system is a tool that automates the process of converting **source code** into **executable artifacts**.

```
┌─────────────────────────────────────────────────────────┐
│                   Build System                           │
│  ┌──────────┐    ┌──────────┐    ┌──────────┐          │
│  │  Source  │ -> │  Build   │ -> │ Artifact │          │
│  │   Code   │    │  System  │    │ (binary) │          │
│  │ (.rs)    │    │ (cargo)  │    │   (.exe) │          │
│  └──────────┘    └──────────┘    └──────────┘          │
└─────────────────────────────────────────────────────────┘
```

**Real-world analogy:** A manufacturing assembly line

| Aspect | Factory | Build System |
|--------|---------|--------------|
| Raw materials | Steel, plastic | Source code files |
| Assembly line | Robots, conveyor belts | Compiler, linker |
| Quality control | Inspectors | Type checker, linter |
| Finished product | Car | Executable binary |

### 1.2 Why Build Systems Exist

**Without a build system:**

```bash
# Manual compilation (impractical for large projects)
rustc src/main.rs
rustc src/lib.rs
rustc src/utils/helper.rs
# ... imagine 100+ files
rustc --link main.o lib.o helper.o -o myapp
```

**Problems:**
- Manual dependency tracking (which files depend on which?)
- No incremental builds (recompile everything every time)
- No dependency management (where do external crates come from?)
- No configuration management (debug vs release builds)

**With a build system (cargo):**

```bash
cargo build
# Cargo automatically:
# 1. Downloads dependencies from crates.io
# 2. Figures out compilation order
# 3. Only recompiles changed files
# 4. Links everything together
```

### 1.3 Types of Build Systems

| Type | Example | Characteristics |
|------|---------|-----------------|
| **Task-based** | Make, Ninja | Explicit rules: "to build A, first build B" |
| **Declarative** | Cargo, Maven | Describe structure, system figures out build |
| **Graph-based** | Bazel, Buck | Dependency graph with content-addressed caching |
| **Transpiler-based** | Pavex, protobuf | Generate source code as build step |

### 1.4 Where Pavex Fits

Pavex is a **transpiler-based build system**:

```
┌─────────────────────────────────────────────────────────┐
│                  Pavex Build Flow                        │
│                                                          │
│  User Code           Pavex CLI         Generated Code   │
│  ┌────────┐         ┌────────┐         ┌────────┐       │
│  │        │         │        │         │        │       │
│  │blueprint│  --->  │ pavexc │  --->   │server_ │       │
│  │  .rs   │         │ trans- │         │ sdk/   │       │
│  │        │         │  piler │         │        │       │
│  └────────┘         └────────┘         └────────┘       │
│                                                          │
│  (Declarative)     (Analysis +       (Ready-to-compile  │
│                    Codegen)           Rust source)       │
└─────────────────────────────────────────────────────────┘
```

---

## 2. Compilation Fundamentals

### 2.1 The Compilation Pipeline

```
Source Code (.rs)
       │
       ▼
┌─────────────────┐
│  Lexical Analysis│  (Tokens: keywords, identifiers, operators)
└────────┬────────┘
         │
         ▼
┌─────────────────┐
│    Parsing      │  (AST: Abstract Syntax Tree)
└────────┬────────┘
         │
         ▼
┌─────────────────┐
│  Name Resolution│  (Link identifiers to definitions)
└────────┬────────┘
         │
         ▼
┌─────────────────┐
│  Type Checking  │  (Verify type correctness)
└────────┬────────┘
         │
         ▼
┌─────────────────┐
│  MIR/Lowering   │  (Mid-level IR for optimizations)
└────────┬────────┘
         │
         ▼
┌─────────────────┐
│  LLVM IR        │  (Low-level IR for codegen)
└────────┬────────┘
         │
         ▼
┌─────────────────┐
│  Code Generation│  (Machine code)
└────────┬────────┘
         │
         ▼
    Object File (.o)
```

### 2.2 AST: Abstract Syntax Tree

The AST is a tree representation of your code:

```rust
// Source code
fn add(a: i32, b: i32) -> i32 {
    a + b
}
```

```
AST:
FnItem
├── name: "add"
├── generics: []
├── parameters:
│   ├── Param { name: "a", type: i32 }
│   └── Param { name: "b", type: i32 }
├── return_type: i32
└── body:
    └── BinExpr (+)
        ├── Left: Ident("a")
        └── Right: Ident("b")
```

### 2.3 Why rustdoc JSON Matters

`rustdoc` traverses the AST and produces JSON documentation:

```bash
cargo +nightly rustdoc -p my_crate -- --output-format json
```

**Output structure:**

```json
{
  "root": 123,
  "crate_version": "0.1.0",
  "includes_private": true,
  "index": {
    "456": {
      "id": 456,
      "name": "add",
      "docs": "Adds two integers",
      "visibility": "public",
      "inner": {
        "function": {
          "sig": {
            "inputs": [
              {"name": "a", "type": {"primitive": "i32"}},
              {"name": "b", "type": {"primitive": "i32"}}
            ],
            "output": {"primitive": "i32"}
          }
        }
      }
    }
  },
  "paths": {...}
}
```

**Pavex uses this for:**
1. Extracting function signatures (inputs, outputs)
2. Resolving type paths (what does `my_crate::MyType` mean?)
3. Building dependency graphs

---

## 3. Code Generation at Build Time

### 3.1 Why Generate Code?

**Problem:** You want to write less boilerplate while maintaining type safety.

**Without codegen:**

```rust
// Manual DI registration (error-prone, repetitive)
struct AppContainer {
    http_client: HttpClient,
    logger: Logger,
    db_pool: DbPool,
}

impl AppContainer {
    fn new() -> Self {
        let config = load_config();
        let http_client = HttpClient::new(&config);
        let logger = Logger::new(&config);
        let db_pool = DbPool::new(&config);
        Self { http_client, logger, db_pool }
    }
}
```

**With codegen (what Pavex generates):**

```rust
// You write
#[pavex::constructor(Lifecycle::Singleton)]
fn http_client() -> HttpClient { HttpClient::new() }

// Pavex generates the container for you
struct ApplicationState {
    s0: HttpClient,  // s0 = singleton 0
}

fn build_application_state() -> ApplicationState {
    ApplicationState {
        s0: http_client(),
    }
}
```

### 3.2 Code Generation Patterns

#### Pattern 1: Derive Macros

```rust
// Input to macro
#[derive(Debug, Clone, Serialize)]
struct User {
    name: String,
    age: u32,
}

// Generated code (expanded)
impl Debug for User {
    fn fmt(&self, f: &mut Formatter) -> Result {
        f.debug_struct("User")
            .field("name", &self.name)
            .field("age", &self.age)
            .finish()
    }
}
// ... Clone, Serialize implementations
```

#### Pattern 2: Attribute Macros

```rust
// Input
#[pavex::constructor(Lifecycle::Singleton)]
fn http_client() -> HttpClient { ... }

// Generated registration
// (conceptually)
blueprint.register_constructor(
    "crate::http_client",
    Lifecycle::Singleton,
    http_client as fn() -> HttpClient
);
```

#### Pattern 3: Build Scripts (build.rs)

```rust
// build.rs - Runs BEFORE compilation
fn main() {
    // Generate code
    let generated = generate_code();
    fs::write("out/generated.rs", generated);

    // Tell cargo where to find it
    println!("cargo:rerun-if-changed=build.rs");
}
```

**Limitation:** Cargo doesn't allow `cargo` commands from build.rs (coarse locking).

Pavex works around this using `cargo-px`.

### 3.3 The f! Macro

Pavex provides a special macro for referencing functions:

```rust
use pavex::f;

bp.constructor(f!(crate::http_client), Lifecycle::Singleton);
```

**What `f!` does:**

1. Captures the function path as a string (for runtime)
2. Provides IDE hints (go-to-definition, completions)
3. Validates the path exists (compile-time check)

**Implementation concept:**

```rust
#[macro_export]
macro_rules! f {
    ($path:path) => {{
        // IDE hint via cfg(pavex_ide_hint)
        #[cfg(pavex_ide_hint)]
        {
            let _ = $path; // Forces IDE to resolve the path
        }
        // Runtime: just the path string
        stringify!($path)
    }};
}
```

---

## 4. rustdoc JSON as Reflection

### 4.1 What Is Reflection?

**Reflection** is the ability to inspect types at runtime (or compile-time).

| Language | Reflection Approach |
|----------|-------------------|
| Java | `Class<?>`, `Method`, `Field` at runtime |
| Python | `type(obj)`, `dir(obj)`, `inspect` module |
| C# | `System.Reflection` namespace |
| Rust | No built-in reflection (use rustdoc JSON or macros) |

### 4.2 How Pavex Uses rustdoc

**Step 1: Generate rustdoc JSON**

```bash
cargo +nightly rustdoc -p my_app --lib -- \
    -Zunstable-options \
    --output-format json \
    --document-private-items
```

**Step 2: Parse the JSON**

```rust
// Simplified parsing
#[derive(Deserialize)]
struct Crate {
    root: ItemId,
    index: HashMap<ItemId, Item>,
    paths: HashMap<ItemId, ItemPath>,
}

#[derive(Deserialize)]
struct Item {
    name: String,
    docs: String,
    visibility: Visibility,
    inner: ItemEnum,
}
```

**Step 3: Extract function signatures**

```rust
fn extract_function_signature(item: &Item) -> FunctionSig {
    match &item.inner {
        ItemEnum::Function(func) => FunctionSig {
            name: item.name.clone(),
            inputs: func.sig.inputs.clone(),
            output: func.sig.output.clone(),
        },
        _ => panic!("Not a function"),
    }
}
```

### 4.3 Limitations of rustdoc JSON

| Limitation | Impact | Pavex's Solution |
|------------|--------|-----------------|
| Requires nightly | Breaking changes possible | Pin to specific nightly version |
| Slow for large crates | Long build times | SQLite caching |
| Format changes | Parser breaks | Version detection, graceful errors |
| No private items by default | Can't inspect internals | `--document-private-items` flag |

### 4.4 rustdoc JSON Caching

Pavex caches rustdoc JSON in SQLite:

```
~/.pavex/rustdoc/cache/
└── 0.1.80-<git-hash>.db  (SQLite database)

Tables:
├── rustdoc_toolchain_crates_cache  (std, core, alloc)
├── rustdoc_3d_party_crates_cache   (serde, tokio, etc.)
└── project2package_id_access_log   (per-project tracking)
```

**Cache key for third-party crates:**

```
(crate_name, crate_version, crate_source, crate_hash,
 cargo_fingerprint, rustdoc_options, features)
```

**Why so many fields?** Any of these can change the rustdoc output.

---

## 5. Incremental Compilation Basics

### 5.1 What Is Incremental Compilation?

**Incremental compilation** means only rebuilding what changed.

```
First build:
  src/main.rs --> compile --> main.o
  src/lib.rs  --> compile --> lib.o
  src/util.rs --> compile --> util.o
  link all .o files --> myapp

Second build (util.rs changed):
  src/util.rs --> compile --> util.o  (only this!)
  link all .o files --> myapp
```

### 5.2 cargo's Incremental Strategy

Cargo uses **fingerprinting**:

```
target/debug/incremental/
├── my_crate-<hash>/
│   ├── dep-graph.bin      (dependency graph)
│   ├── query-cache.bin    (query results)
│   └── work-products.bin  (compiled artifacts)
```

**Fingerprint includes:**
- Source file hash
- Dependency hashes
- Compiler flags
- Environment variables

### 5.3 Pavex's Incremental Strategy

Pavex adds another layer:

```
┌─────────────────────────────────────────────────────────┐
│              Pavex Incremental Flow                      │
│                                                          │
│  Blueprint Changed? --> Regenerate SDK                 │
│  Dependencies Changed? --> Re-fetch rustdoc JSON       │
│  Handler Changed? --> Re-analyze call graph            │
│                                                          │
│  Cache Storage: SQLite (~/.pavex/rustdoc/cache/)      │
│  Per-Project: .pavex/ directory                        │
└─────────────────────────────────────────────────────────┘
```

### 5.4 Cache Invalidation

**When to invalidate cache:**

1. **Source changed** - BLAKE3 hash mismatch
2. **Features changed** - Different feature flags enabled
3. **Toolchain changed** - Different nightly version
4. **rustdoc options changed** - Different flags passed

**Pavex's cache invalidation:**

```rust
// Simplified
fn should_use_cache(
    cached: &CachedEntry,
    current: &CurrentState,
) -> bool {
    cached.crate_hash == current.crate_hash
        && cached.features == current.features
        && cached.cargo_fingerprint == current.cargo_fingerprint
        && cached.rustdoc_options == current.rustdoc_options
}
```

### 5.5 Project Access Log

Pavex tracks which packages each project uses:

```sql
-- In SQLite
CREATE TABLE project2package_id_access_log (
    project_fingerprint TEXT PRIMARY KEY,
    package_ids BLOB  -- Serialized Vec<PackageId>
);
```

**Why?** To clean up unused cache entries and optimize rebuilds.

---

## 6. Your Learning Path

### 6.1 How to Use This Exploration

This document is part of a larger exploration:

```
pavex/
├── 00-zero-to-build-engineer.md    <- You are here (foundations)
├── 01-macro-codegen-deep-dive.md
├── 02-dependency-resolution-deep-dive.md
├── 03-incremental-builds-deep-dive.md
├── 04-framework-integration-deep-dive.md
├── rust-revision.md
├── production-grade.md
└── 05-valtron-integration.md
```

### 6.2 Recommended Reading Order

**For complete beginners:**

1. **This document (00-zero-to-build-engineer.md)** - Build system foundations
2. **01-macro-codegen-deep-dive.md** - Procedural macros and codegen
3. **02-dependency-resolution-deep-dive.md** - How dependencies work
4. **03-incremental-builds-deep-dive.md** - Caching strategies

**For experienced Rust developers:**

1. Skim this document for context
2. Jump to 01-macro-codegen-deep-dive.md
3. Study pavexc source directly (libs/pavexc/src/)

### 6.3 Practice Exercises

**Exercise 1: Generate rustdoc JSON**

```bash
# Create a simple crate
cargo new my_test
cd my_test

# Generate rustdoc JSON
cargo +nightly rustdoc --lib -- \
    -Zunstable-options \
    --output-format json \
    --document-private-items

# Find the output
ls target/doc/my_test.json
```

**Exercise 2: Parse rustdoc JSON**

```rust
// Add to Cargo.toml
// [dependencies]
// rustdoc-types = "0.25"
// serde_json = "1.0"

use rustdoc_types::Crate;
use std::fs;

fn main() {
    let json = fs::read_to_string("target/doc/my_test.json").unwrap();
    let krate: Crate = serde_json::from_str(&json).unwrap();
    println!("Root item: {:?}", krate.root);
}
```

**Exercise 3: Create a simple build script**

```rust
// build.rs
fn main() {
    println!("cargo:warning=Build script running!");
    println!("cargo:rerun-if-changed=build.rs");
}
```

### 6.4 Next Steps After Completion

**After finishing this exploration:**

1. **Read the pavex ARCHITECTURE.md** - Official deep dive
2. **Study the pavexc source** - libs/pavexc/src/compiler/
3. **Build a mini-pavex** - Generate simple DI code from rustdoc
4. **Explore guppy** - Cargo metadata library Pavex uses

### 6.5 Key Resources

| Resource | Purpose |
|----------|---------|
| [rustdoc JSON format](https://github.com/rust-lang/rust/blob/master/src/librustdoc/json/types.rs) | rustdoc types definition |
| [guppy documentation](https://docs.rs/guppy) | Cargo metadata library |
| [syn crate](https://docs.rs/syn) | Rust parsing for macros |
| [prettyplease](https://docs.rs/prettyplease) | Rust code formatter |

---

## Appendix A: Build System Comparison

| Feature | cargo | Bazel | Pavex |
|---------|-------|-------|-------|
| Language | Rust | Multi | Rust |
| Paradigm | Declarative | Graph-based | Transpiler |
| Caching | Per-project | Global, content-addressed | SQLite global |
| Incremental | File-based | Target-based | Analysis-based |
| Extensibility | build.rs | Starlark rules | Custom CLI |

## Appendix B: rustdoc JSON Schema

```typescript
interface Crate {
  root: number;                    // Root module ID
  crate_version: string;           // Version from Cargo.toml
  includes_private: boolean;       // Whether private items included
  format_version: number;          // rustdoc format version
  index: Map<number, Item>;        // All items by ID
  paths: Map<number, ItemPath>;    // Path information
  external_crates: Map<string, ExternalCrate>;
}

interface Item {
  id: number;
  name: string;
  docs?: string;
  visibility: "public" | "crate" | "private";
  inner: ItemEnum;
}

type ItemEnum =
  | { function: Function }
  | { struct: Struct }
  | { enum: Enum }
  | { trait: Trait }
  | { type: TypeAlias }
  | { module: Module }
  | { impl: Impl }
  | ...;

interface Function {
  sig: Signature;
  generics: Generics;
  header: FnHeader;
}

interface Signature {
  inputs: Param[];
  output?: Type;
}
```

## Appendix C: Key Terminology

| Term | Definition |
|------|------------|
| **Transpiler** | Converts source code to source code (not binary) |
| **AST** | Abstract Syntax Tree - structured code representation |
| **rustdoc JSON** | rustdoc's JSON output describing crate contents |
| **Fingerprint** | Hash used to detect changes for incremental builds |
| **Singleton** | Constructed once, reused for all requests |
| **Request-scoped** | Constructed once per request |
| **Transient** | Constructed fresh every time needed |

---

*This document is a living textbook. Revisit sections as concepts become clearer through implementation. Next: [01-macro-codegen-deep-dive.md](01-macro-codegen-deep-dive.md)*
