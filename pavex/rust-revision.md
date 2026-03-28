---
title: "Rust Revision: Pavex Design Patterns"
subtitle: "Note: Pavex is already Rust - This document covers key design patterns used"
source: /home/darkvoid/Boxxed/@formulas/src.rust/src.BuildTooling/pavex
revised_at: 2026-03-27
---

# Rust Revision: Pavex Design Patterns

## Overview

**Note:** Pavex is already written in Rust. This document doesn't translate from another language but instead documents the **key Rust design patterns** used throughout the Pavex codebase that you can learn from and replicate.

---

## 1. Error Handling with miette

### 1.1 Custom Error Types

```rust
// libs/pavexc/src/diagnostic/kind.rs
use miette::{Diagnostic, SourceSpan};
use thiserror::Error;

#[derive(Debug, Error, Diagnostic)]
pub enum CompilerError {
    #[error("Failed to parse blueprint")]
    #[diagnostic(code(pavex::blueprint::parse_error))]
    BlueprintParseError {
        #[source_code]
        source: String,
        #[label("error here")]
        span: SourceSpan,
    },

    #[error("Type '{0}' is not constructible")]
    #[diagnostic(
        code(pavex::di::not_constructible),
        help("Register a constructor for this type")
    )]
    TypeNotConstructible(String),

    #[error("Circular dependency detected")]
    #[diagnostic(
        code(pavex::di::circular_dependency),
        help("Break the cycle by restructuring dependencies")
    )]
    CircularDependency {
        #[label("depends on")]
        spans: Vec<SourceSpan>,
    },
}
```

### 1.2 Error Context Pattern

```rust
// libs/pavexc/src/rustdoc/compute/mod.rs
fn compute_crate_docs<I>(
    package_ids: I,
) -> Result<HashMap<PackageId, Crate>, anyhow::Error>
where
    I: Iterator<Item = PackageId>,
{
    let result = try_compute(package_ids)
        .context("Failed to compute rustdoc JSON for crates")?;

    Ok(result)
}

fn try_compute<I>(package_ids: I) -> anyhow::Result<HashMap<PackageId, Crate>> {
    // Implementation that may fail
}
```

### 1.3 Diagnostic Builder Pattern

```rust
// libs/pavexc/src/diagnostic/sink.rs
pub struct DiagnosticSink {
    errors: Vec<Diagnostic>,
    warnings: Vec<Diagnostic>,
}

impl DiagnosticSink {
    pub fn error(&mut self, msg: impl Into<String>) -> DiagnosticBuilder {
        DiagnosticBuilder::new(DiagnosticLevel::Error, msg.into())
    }

    pub fn warning(&mut self, msg: impl Into<String>) -> DiagnosticBuilder {
        DiagnosticBuilder::new(DiagnosticLevel::Warning, msg.into())
    }

    pub fn emit(&mut self, diag: Diagnostic) {
        match diag.level {
            DiagnosticLevel::Error => self.errors.push(diag),
            DiagnosticLevel::Warning => self.warnings.push(diag),
        }
    }
}

pub struct DiagnosticBuilder {
    level: DiagnosticLevel,
    message: String,
    labels: Vec<Label>,
    help: Option<String>,
}

impl DiagnosticBuilder {
    pub fn label(mut self, span: SourceSpan, msg: impl Into<String>) -> Self {
        self.labels.push(Label { span, msg: msg.into() });
        self
    }

    pub fn help(mut self, msg: impl Into<String>) -> Self {
        self.help = Some(msg.into());
        self
    }

    pub fn emit(self, sink: &mut DiagnosticSink) {
        sink.emit(self.build());
    }
}
```

---

## 2. Newtype Pattern for Type Safety

### 2.1 PackageId Wrapper

```rust
// libs/pavexc/src/language/krate_name.rs
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct PackageName(Box<str>);

impl PackageName {
    pub fn new(name: impl Into<Box<str>>) -> Self {
        Self(name.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for PackageName {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}
```

### 2.2 ResolvedType for Type Paths

```rust
// libs/pavexc/src/language/resolved_type.rs
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ResolvedType {
    pub package_id: PackageId,
    pub base_type: Vec<String>,  // e.g., ["std", "vec", "Vec"]
    pub generic_arguments: Vec<ResolvedType>,
}

impl ResolvedType {
    /// Check if this type is Option<T>
    pub fn is_option(&self) -> bool {
        self.base_type.last() == Some(&"Option".to_string())
            && self.generic_arguments.len() == 1
    }

    /// Check if this type is Result<T, E>
    pub fn is_result(&self) -> bool {
        self.base_type.last() == Some(&"Result".to_string())
            && self.generic_arguments.len() == 2
    }

    /// Get the Ok type if this is Result<T, E>
    pub fn ok_type(&self) -> Option<&ResolvedType> {
        if self.is_result() {
            self.generic_arguments.first()
        } else {
            None
        }
    }
}
```

---

## 3. Builder Pattern for Complex Types

### 3.1 Blueprint Builder

```rust
// libs/pavex/src/blueprint/mod.rs
pub struct Blueprint {
    routes: Vec<Route>,
    constructors: Vec<Constructor>,
    middleware: Vec<Middleware>,
    error_handlers: Vec<ErrorHandler>,
}

impl Blueprint {
    pub fn new() -> Self {
        Self {
            routes: Vec::new(),
            constructors: Vec::new(),
            middleware: Vec::new(),
            error_handlers: Vec::new(),
        }
    }

    pub fn route(
        &mut self,
        method: Method,
        path: &str,
        handler: &'static str,
    ) -> &mut Self {
        self.routes.push(Route {
            method,
            path: path.to_string(),
            handler: handler.to_string(),
        });
        self
    }

    pub fn constructor(
        &mut self,
        callable: &'static str,
        lifecycle: Lifecycle,
    ) -> &mut Self {
        self.constructors.push(Constructor {
            callable: callable.to_string(),
            lifecycle,
        });
        self
    }

    pub fn merge(&mut self, other: Blueprint) -> &mut Self {
        self.routes.extend(other.routes);
        self.constructors.extend(other.constructors);
        self.middleware.extend(other.middleware);
        self.error_handlers.extend(other.error_handlers);
        self
    }
}
```

### 3.2 Fluent API for Route Configuration

```rust
// libs/pavex/src/blueprint/route.rs
pub struct RouteBuilder {
    path: String,
    method: Method,
    handler: String,
    middleware: Vec<String>,
    guards: Vec<Guard>,
}

impl RouteBuilder {
    pub fn new(method: Method, path: impl Into<String>) -> Self {
        Self {
            path: path.into(),
            method,
            handler: String::new(),
            middleware: Vec::new(),
            guards: Vec::new(),
        }
    }

    pub fn handled_by(mut self, handler: impl Into<String>) -> Self {
        self.handler = handler.into();
        self
    }

    pub fn with_middleware(mut self, middleware: impl Into<String>) -> Self {
        self.middleware.push(middleware.into());
        self
    }

    pub fn guarded_by(mut self, guard: Guard) -> Self {
        self.guards.push(guard);
        self
    }

    pub fn build(self) -> Route {
        Route {
            path: self.path,
            method: self.method,
            handler: self.handler,
            middleware: self.middleware,
            guards: self.guards,
        }
    }
}
```

---

## 4. Arena Allocation for Performance

### 4.1 String Interning

```rust
// libs/pavexc/src/compiler/interner.rs
use std::collections::HashMap;

/// Interns strings to avoid duplicates and enable fast comparison.
pub struct StringInterner {
    strings: Vec<Box<str>>,
    map: HashMap<Box<str>, usize>,
}

impl StringInterner {
    pub fn new() -> Self {
        Self {
            strings: Vec::new(),
            map: HashMap::new(),
        }
    }

    pub fn intern(&mut self, s: &str) -> InternedString {
        if let Some(&idx) = self.map.get(s) {
            return InternedString(idx);
        }

        let boxed: Box<str> = s.into();
        let idx = self.strings.len();
        self.strings.push(boxed);
        self.map.insert(s.into(), idx);
        InternedString(idx)
    }

    pub fn resolve(&self, interned: InternedString) -> &str {
        &self.strings[interned.0]
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct InternedString(usize);
```

### 4.2 Index-Based Storage

```rust
// Using la-arena crate for typed indices
use la_arena::{Arena, Idx};

pub struct ComponentArena {
    components: Arena<Component>,
}

pub type ComponentId = Idx<Component>;

impl ComponentArena {
    pub fn alloc(&mut self, component: Component) -> ComponentId {
        self.components.alloc(component)
    }

    pub fn get(&self, id: ComponentId) -> &Component {
        &self.components[id]
    }

    pub fn get_mut(&mut self, id: ComponentId) -> &mut Component {
        &mut self.components[id]
    }

    pub fn iter(&self) -> impl Iterator<Item = (ComponentId, &Component)> {
        self.components.iter()
    }
}
```

---

## 5. Graph Algorithms with petgraph

### 5.1 Dependency Graph

```rust
// libs/pavexc/src/compiler/analyses/call_graph/dependency_graph.rs
use petgraph::graphmap::DiGraphMap;
use petgraph::visit::{Dfs, VisitMap, Visitable};

pub struct DependencyGraph {
    graph: DiGraphMap<ResolvedType, EdgeType>,
}

impl DependencyGraph {
    pub fn new() -> Self {
        Self {
            graph: DiGraphMap::new(),
        }
    }

    pub fn add_dependency(&mut self, from: ResolvedType, to: ResolvedType) {
        self.graph.add_edge(from, to, EdgeType::Needs);
    }

    /// Detect cycles in the dependency graph
    pub fn has_cycle(&self) -> Option<Vec<ResolvedType>> {
        use petgraph::algo::tarjan_scc;

        let sccs = tarjan_scc(&self.graph);
        for scc in sccs {
            if scc.len() > 1 {
                return Some(scc);  // Cycle found
            }
        }
        None
    }

    /// Get topological order (if no cycles)
    pub fn topological_sort(&self) -> Result<Vec<ResolvedType>, CycleError> {
        use petgraph::algo::toposort;

        let sorted = toposort(&self.graph, None)
            .map_err(|cycle| CycleError { node: cycle.node_id() })?;

        Ok(sorted)
    }
}
```

### 5.2 Call Graph Construction

```rust
// libs/pavexc/src/compiler/analyses/call_graph/mod.rs
use fixedbitset::FixedBitSet;

pub struct CallGraph {
    /// Nodes: callables (handlers, constructors)
    nodes: Vec<Callable>,
    /// Edges: A needs B to be called
    edges: Vec<(usize, usize)>,
    /// Precomputed reachability
    reachability: Vec<FixedBitSet>,
}

impl CallGraph {
    /// Build call graph from dependency graph
    pub fn from_dependency_graph(
        dep_graph: &DependencyGraph,
        lifecycles: &LifecycleMap,
    ) -> Result<Self, BuildError> {
        // ...
    }

    /// Get all callables needed to call this one
    pub fn dependencies(&self, callable: usize) -> Vec<usize> {
        self.edges
            .iter()
            .filter(|(from, _)| *from == callable)
            .map(|(_, to)| *to)
            .collect()
    }

    /// Check if A transitively depends on B
    pub fn depends_on(&self, a: usize, b: usize) -> bool {
        self.reachability[a].contains(b)
    }
}
```

---

## 6. Borrow Checker at Build Time

### 6.1 Lifetime Validation

```rust
// libs/pavexc/src/compiler/analyses/call_graph/borrow_checker/mod.rs
pub struct BorrowChecker {
    lifetimes: HashMap<CallableId, Lifecycle>,
}

impl BorrowChecker {
    /// Validate that borrow relationships are sound
    pub fn validate(&self, call_graph: &CallGraph) -> Result<(), BorrowError> {
        for (caller, callee) in call_graph.edges() {
            self.validate_edge(caller, callee)?;
        }
        Ok(())
    }

    fn validate_edge(
        &self,
        caller: CallableId,
        callee: CallableId,
    ) -> Result<(), BorrowError> {
        let caller_lifecycle = self.lifetimes.get(&caller).unwrap();
        let callee_lifecycle = self.lifetimes.get(&callee).unwrap();

        // Rule: A longer-lived thing cannot borrow from a shorter-lived thing
        if caller_lifecycle.outlives(callee_lifecycle) {
            return Err(BorrowError::DanglingBorrow {
                caller,
                callee,
                caller_lifecycle: *caller_lifecycle,
                callee_lifecycle: *callee_lifecycle,
            });
        }

        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Lifecycle {
    Transient,      // Shortest
    RequestScoped,  // Medium
    Singleton,      // Longest
}

impl Lifecycle {
    pub fn outlives(&self, other: Lifecycle) -> bool {
        // Singleton outlives RequestScoped, etc.
        *self > other
    }
}
```

---

## 7. Serialization for Caching

### 7.1 Bincode for Structured Data

```rust
// libs/pavexc/src/rustdoc/compute/cache.rs
use bincode::{config::Configuration, encode_to_vec, decode_from_slice};

const BINCODE_CONFIG: Configuration = bincode::config::standard();

#[derive(Serialize, Deserialize)]
pub struct CachedMetadata {
    pub root_item_id: u32,
    pub external_crates: Vec<u8>,
    pub paths: HashMap<u32, ItemPath>,
}

impl CachedMetadata {
    pub fn serialize(&self) -> Result<Vec<u8>, bincode::error::EncodeError> {
        encode_to_vec(self, BINCODE_CONFIG)
    }

    pub fn deserialize(bytes: &[u8]) -> Result<Self, bincode::error::DecodeError> {
        let (decoded, _): (Self, usize) = decode_from_slice(bytes, BINCODE_CONFIG)?;
        Ok(decoded)
    }
}
```

### 7.2 Lazy Deserialization

```rust
pub enum LazyOrEager<T> {
    Lazy { bytes: Vec<u8> },
    Eager(T),
}

impl<T: DeserializeOwned> LazyOrEager<T> {
    pub fn force_eager(&mut self) -> Result<(), Error> {
        if let LazyOrEager::Lazy { bytes } = self {
            let value: T = serde_json::from_slice(bytes)?;
            *self = LazyOrEager::Eager(value);
        }
        Ok(())
    }

    pub fn as_ref(&self) -> Result<&T, Error> {
        match self {
            LazyOrEager::Eager(v) => Ok(v),
            LazyOrEager::Lazy { bytes } => Err(Error::NotDeserialized),
        }
    }
}
```

---

## 8. Parallel Processing with rayon

### 8.1 Parallel Deserialization

```rust
use rayon::prelude::*;

fn deserialize_in_parallel(
    json_files: Vec<(PackageId, PathBuf)>,
) -> HashMap<PackageId, Crate> {
    json_files
        .into_par_iter()
        .map(|(package_id, path)| {
            let json = fs_err::read_to_string(&path)?;
            let krate: Crate = serde_json::from_str(&json)?;
            Ok((package_id, krate))
        })
        .collect::<Result<_, Error>>()
        .unwrap()
}
```

### 8.2 Parallel Graph Processing

```rust
use rayon::prelude::*;

fn process_components_parallel(
    components: &[Component],
) -> Vec<ProcessedComponent> {
    components
        .par_iter()
        .map(|component| {
            // Each component processed independently
            process_single_component(component)
        })
        .collect()
}
```

---

## 9. Tracing for Observability

### 9.1 Instrumented Functions

```rust
use tracing::{instrument, Span};
use tracing_log_error::log_error;

#[instrument(
    name = "Generate SDK",
    skip_all,
    fields(
        blueprint_path = %blueprint_path.display(),
        output_path = %output_path.display(),
    )
)]
pub fn generate_sdk(
    blueprint_path: &Path,
    output_path: &Path,
) -> Result<(), Error> {
    let blueprint = load_blueprint(blueprint_path)
        .inspect_err(|e| log_error!(e, "Failed to load blueprint"))?;

    let analysis = analyze(&blueprint)
        .inspect_err(|e| log_error!(e, "Failed to analyze blueprint"))?;

    generate_code(&analysis, output_path)
        .inspect_err(|e| log_error!(e, "Failed to generate code"))?;

    Ok(())
}
```

### 9.2 Span Field Updates

```rust
#[instrument(name = "Cache lookup", skip_all, fields(hit = tracing::field::Empty))]
fn get_from_cache(key: &CacheKey) -> Result<Option<CachedValue>, Error> {
    let result = _get_impl(key)?;

    // Record whether it was a hit or miss
    Span::current().record("hit", result.is_some());

    Ok(result)
}
```

---

## 10. Workspace Hack Pattern

### 10.1 px_workspace_hack

```toml
# libs/px_workspace_hack/Cargo.toml
[package]
name = "px_workspace_hack"
version = "0.1.80"
edition = "2024"

[dependencies]
# Common dependencies used across workspace
tracing = { workspace = true }
anyhow = { workspace = true }
thiserror = { workspace = true }
miette = { workspace = true }
serde = { workspace = true, features = ["derive"] }
serde_json = { workspace = true }
```

```toml
# Every other crate includes:
[dependencies]
px_workspace_hack = { version = "0.1", path = "../px_workspace_hack" }
```

**Purpose:**
- Force consistent dependency versions
- Centralize feature flag configuration
- Reduce compilation time (shared dependencies)

---

## Key Takeaways

1. **miette for diagnostics** - Rich error reporting with source spans
2. **Newtypes for type safety** - Wrap primitives in semantic types
3. **Builder pattern** - Fluent APIs for complex construction
4. **Arena allocation** - Interned strings, index-based storage
5. **petgraph for analysis** - Dependency graphs, cycle detection
6. **Lifetime validation** - Borrow checking at build time
7. **bincode for caching** - Efficient serialization
8. **rayon for parallelism** - Parallel deserialization and processing
9. **tracing for observability** - Instrumented functions with spans
10. **Workspace hack** - Shared dependencies across crates

---

## Related Files

- **Diagnostic types**: `/home/darkvoid/Boxxed/@formulas/src.rust/src.BuildTooling/pavex/libs/pavexc/src/diagnostic/kind.rs`
- **ResolvedType**: `/home/darkvoid/Boxxed/@formulas/src.rust/src.BuildTooling/pavex/libs/pavexc/src/language/resolved_type.rs`
- **Borrow checker**: `/home/darkvoid/Boxxed/@formulas/src.rust/src.BuildTooling/pavex/libs/pavexc/src/compiler/analyses/call_graph/borrow_checker/mod.rs`
- **Cache implementation**: `/home/darkvoid/Boxxed/@formulas/src.rust/src.BuildTooling/pavex/libs/pavexc/src/rustdoc/compute/cache.rs`

---

*Next: [production-grade.md](production-grade.md)*
