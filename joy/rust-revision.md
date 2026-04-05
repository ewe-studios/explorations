# Joy Compiler - Rust Revision

## Overview

This document translates the Joy ecosystem concepts (Compiler, Bud Framework, X-ray, LLM) to Rust implementations. The goal is to build similar tools using Rust's type system, performance characteristics, and ecosystem.

## Architecture Comparison

### Go (Joy) vs Rust Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                    Joy (Go) Architecture                         │
├─────────────────────────────────────────────────────────────────┤
│  Go Source → parser → index → graph → translate → assemble → JS  │
│                                                                  │
│  - Runtime reflection for some operations                       │
│  - Garbage collected memory                                     │
│  - Concurrent with goroutines                                   │
│  - Dynamic interface assertions                                 │
└─────────────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────────────┐
│                    Rust Implementation                            │
├─────────────────────────────────────────────────────────────────┤
│  Rust Source → syn → index → graph → translate → quote → JS/TS   │
│                                                                  │
│  - Zero-cost abstractions                                       │
│  - Compile-time type checking                                   │
│  - Memory safe without GC                                       │
│  - Trait-based polymorphism                                     │
└─────────────────────────────────────────────────────────────────┘
```

## Core Data Structures in Rust

### Definition Trait

```rust
// src/compiler/definition.rs
use std::rc::Rc;
use std::cell::RefCell;

/// Unique identifier for a definition
pub type DefId = String;

/// Kind of definition
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum DefKind {
    Function,
    Struct,
    Method,
    Interface,
    Type,
    Const,
    Var,
}

/// A definition in the source code
pub trait Definition: std::fmt::Debug {
    /// Unique identifier
    fn id(&self) -> DefId;
    
    /// Full path (e.g., "package/subpkg.Name")
    fn path(&self) -> &str;
    
    /// Kind of definition
    fn kind(&self) -> DefKind;
    
    /// Simple name
    fn name(&self) -> &str;
    
    /// Whether it's exported/public
    fn exported(&self) -> bool;
    
    /// Whether it should be omitted from output
    fn omitted(&self) -> bool {
        false
    }
    
    /// Dependencies of this definition
    fn dependencies(&self) -> Result<Vec<Rc<dyn Definition>>, DefError>;
    
    /// Imports required by this definition
    fn imports(&self) -> std::collections::HashMap<String, String>;
}

#[derive(Debug)]
pub enum DefError {
    NotFound(String),
    Cycle(Vec<DefId>),
    TypeMismatch(String),
}
```

### Index System

```rust
// src/compiler/index.rs
use std::collections::HashMap;
use std::rc::Rc;

/// Index of all definitions
pub struct Index {
    definitions: HashMap<DefId, Rc<dyn Definition>>,
    packages: HashMap<String, Package>,
    mains: Vec<Rc<dyn Definition>>,
}

struct Package {
    name: String,
    path: String,
    exports: Vec<DefId>,
}

impl Index {
    pub fn new() -> Self {
        Index {
            definitions: HashMap::new(),
            packages: HashMap::new(),
            mains: Vec::new(),
        }
    }
    
    pub fn add(&mut self, def: Rc<dyn Definition>) {
        let id = def.id();
        self.definitions.insert(id, def);
    }
    
    pub fn find(&self, id: &str) -> Option<Rc<dyn Definition>> {
        self.definitions.get(id).cloned()
    }
    
    pub fn mains(&self) -> &[Rc<dyn Definition>] {
        &self.mains
    }
    
    pub fn by_package(&self, pkg: &str) -> Vec<Rc<dyn Definition>> {
        self.definitions
            .values()
            .filter(|d| d.path().starts_with(pkg))
            .cloned()
            .collect()
    }
}
```

### Dependency Graph

```rust
// src/compiler/graph.rs
use petgraph::graph::DiGraph;
use petgraph::algo::toposort;
use std::rc::Rc;

/// Dependency graph of definitions
pub struct Graph {
    graph: DiGraph<Rc<dyn Definition>, ()>,
    node_map: HashMap<DefId, petgraph::graph::NodeIndex>,
}

impl Graph {
    pub fn new() -> Self {
        Graph {
            graph: DiGraph::new(),
            node_map: HashMap::new(),
        }
    }
    
    /// Add a definition node
    pub fn add_node(&mut self, def: Rc<dyn Definition>) -> petgraph::graph::NodeIndex {
        if let Some(&idx) = self.node_map.get(&def.id()) {
            return idx;
        }
        let idx = self.graph.add_node(def.clone());
        self.node_map.insert(def.id(), idx);
        idx
    }
    
    /// Add dependency edge (from depends on to)
    pub fn add_edge(&mut self, from: &Rc<dyn Definition>, to: &Rc<dyn Definition>) {
        let from_idx = self.add_node(from.clone());
        let to_idx = self.add_node(to.clone());
        self.graph.add_edge(from_idx, to_idx, ());
    }
    
    /// Topological sort - dependencies first
    pub fn toposort(&self, root: &Rc<dyn Definition>) -> Result<Vec<DefId>, CycleError> {
        let root_idx = *self.node_map.get(&root.id())
            .ok_or_else(|| CycleError::NotFound(root.id()))?;
        
        // Get all nodes reachable from root
        let mut nodes = Vec::new();
        self.collect_deps(root_idx, &mut nodes);
        
        // Topological sort
        let sorted = toposort(&self.graph, None)
            .map_err(|e| CycleError::Detected(format!("{:?}", e)))?;
        
        // Filter to only include nodes we care about
        Ok(sorted
            .into_iter()
            .filter(|idx| nodes.contains(idx))
            .map(|idx| self.graph[*idx].id())
            .collect())
    }
    
    fn collect_deps(
        &self,
        node: petgraph::graph::NodeIndex,
        collected: &mut Vec<petgraph::graph::NodeIndex>,
    ) {
        for dep in self.graph.neighbors(node) {
            if !collected.contains(&dep) {
                self.collect_deps(dep, collected);
            }
        }
        collected.push(node);
    }
}

#[derive(Debug)]
pub enum CycleError {
    NotFound(String),
    Detected(String),
}
```

## JavaScript AST in Rust

```rust
// src/jsast/mod.rs
use serde::Serialize;

/// Position in source code
#[derive(Debug, Clone, Serialize)]
pub struct Position {
    pub line: u32,
    pub column: u32,
}

/// Source location
#[derive(Debug, Clone, Serialize)]
pub struct SourceLocation {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
    pub start: Position,
    pub end: Position,
}

/// Base node trait
pub trait Node: std::fmt::Debug {
    fn node_type(&self) -> &'static str;
    fn loc(&self) -> Option<&SourceLocation> {
        None
    }
}

/// Expression trait
pub trait Expression: Node {
    fn as_expression(&self) -> &dyn Expression;
}

/// Statement trait
pub trait Statement: Node {
    fn as_statement(&self) -> &dyn Statement;
}

/// Identifier node
#[derive(Debug, Clone, Serialize)]
pub struct Identifier {
    #[serde(rename = "type")]
    pub node_type: &'static str,
    pub name: String,
}

impl Node for Identifier {
    fn node_type(&self) -> &'static str {
        "Identifier"
    }
}

/// Literal value
#[derive(Debug, Clone, Serialize)]
#[serde(untagged)]
pub enum LiteralValue {
    String(String),
    Number(f64),
    Bool(bool),
    Null,
}

/// Literal node
#[derive(Debug, Clone, Serialize)]
pub struct Literal {
    #[serde(rename = "type")]
    pub node_type: &'static str,
    pub value: LiteralValue,
}

impl Node for Literal {
    fn node_type(&self) -> &'static str {
        "Literal"
    }
}

/// Function expression
#[derive(Debug, Clone, Serialize)]
pub struct FunctionExpression {
    #[serde(rename = "type")]
    pub node_type: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<Identifier>,
    pub params: Vec<Pattern>,
    pub body: FunctionBody,
    #[serde(default)]
    pub async: bool,
    #[serde(default)]
    pub generator: bool,
}

/// Call expression
#[derive(Debug, Clone, Serialize)]
pub struct CallExpression {
    #[serde(rename = "type")]
    pub node_type: &'static str,
    pub callee: Box<dyn Expression>,
    pub arguments: Vec<Box<dyn Expression>>,
}

/// Binary expression
#[derive(Debug, Clone, Serialize)]
pub struct BinaryExpression {
    #[serde(rename = "type")]
    pub node_type: &'static str,
    pub operator: BinaryOperator,
    pub left: Box<dyn Expression>,
    pub right: Box<dyn Expression>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum BinaryOperator {
    Eq,
    Neq,
    StrictEq,
    StrictNeq,
    Lt,
    Lte,
    Gt,
    Gte,
    Plus,
    Minus,
    Multiply,
    Divide,
    Modulo,
    And,
    Or,
    // ... more operators
}

/// Program (root node)
#[derive(Debug, Clone, Serialize)]
pub struct Program {
    #[serde(rename = "type")]
    pub node_type: &'static str,
    pub body: Vec<Statement>,
}
```

## Code Generation with quote!

```rust
// src/jsast/codegen.rs
use quote::{quote, ToTokens};
use proc_macro2::TokenStream;

/// Generate JavaScript code from AST
pub struct CodeGenerator {
    indent: usize,
}

impl CodeGenerator {
    pub fn new() -> Self {
        CodeGenerator { indent: 0 }
    }
    
    pub fn generate(&mut self, program: &Program) -> String {
        let tokens = self.generate_program(program);
        tokens.to_string()
    }
    
    fn generate_program(&mut self, program: &Program) -> TokenStream {
        let body: Vec<TokenStream> = program
            .body
            .iter()
            .map(|stmt| self.generate_statement(stmt))
            .collect();
        
        quote! {
            #(#body);*;
        }
    }
    
    fn generate_statement(&mut self, stmt: &dyn Statement) -> TokenStream {
        // Match on statement type and generate
        // This would be implemented based on concrete types
        todo!()
    }
}

/// Alternative: Direct string generation (simpler for JS)
pub struct JsWriter {
    output: String,
    indent: usize,
}

impl JsWriter {
    pub fn new() -> Self {
        JsWriter {
            output: String::new(),
            indent: 0,
        }
    }
    
    pub fn write(&mut self, s: &str) {
        self.output.push_str(s);
    }
    
    pub fn write_indent(&mut self, s: &str) {
        for _ in 0..self.indent {
            self.output.push_str("  ");
        }
        self.output.push_str(s);
    }
    
    pub fn newline(&mut self) {
        self.output.push('\n');
    }
    
    pub fn finish(self) -> String {
        self.output
    }
}
```

## Bud Framework - Rust Implementation

### Controller Trait

```rust
// src/framework/controller.rs
use axum::{extract::State, response::IntoResponse, Json};
use serde::{Deserialize, Serialize};
use std::future::Future;

/// HTTP response trait
pub trait Response: IntoResponse {}

/// Controller trait for handling HTTP requests
pub trait Controller: Send + Sync {
    /// The state type this controller needs
    type State: Send + Sync;
    
    /// Error type for this controller
    type Error: std::fmt::Debug;
}

/// RESTful controller with CRUD operations
pub trait ResourceController: Controller {
    type Resource: Serialize + for<'de> Deserialize<'de> + Send + Sync;
    type CreateRequest: for<'de> Deserialize<'de> + Send + Sync;
    type UpdateRequest: for<'de> Deserialize<'de> + Send + Sync;

    /// GET / - List all resources
    fn index(
        &self,
        state: Self::State,
    ) -> impl Future<Output = Result<Json<Vec<Self::Resource>>, Self::Error>> + Send;

    /// GET /:id - Show single resource
    fn show(
        &self,
        state: Self::State,
        id: String,
    ) -> impl Future<Output = Result<Json<Self::Resource>, Self::Error>> + Send;

    /// POST / - Create resource
    fn create(
        &self,
        state: Self::State,
        req: Self::CreateRequest,
    ) -> impl Future<Output = Result<Json<Self::Resource>, Self::Error>> + Send;

    /// PATCH /:id - Update resource
    fn update(
        &self,
        state: Self::State,
        id: String,
        req: Self::UpdateRequest,
    ) -> impl Future<Output = Result<Json<Self::Resource>, Self::Error>> + Send;

    /// DELETE /:id - Delete resource
    fn delete(
        &self,
        state: Self::State,
        id: String,
    ) -> impl Future<Output = Result<(), Self::Error>> + Send;
}
```

### Router Generation

```rust
// src/framework/router.rs
use axum::{routing::*, Router};
use std::sync::Arc;

/// Route builder
pub struct RouteBuilder<S> {
    prefix: String,
    routes: Vec<Route<S>>,
}

enum Route<S> {
    Get { path: String, handler: Arc<dyn Handler<S>> },
    Post { path: String, handler: Arc<dyn Handler<S>> },
    // ... more methods
}

trait Handler<S>: Send + Sync {
    fn call(&self, state: S) -> impl Future<Output = axum::response::Response>;
}

impl<S: Clone + Send + Sync + 'static> RouteBuilder<S> {
    pub fn new(prefix: impl Into<String>) -> Self {
        RouteBuilder {
            prefix: prefix.into(),
            routes: Vec::new(),
        }
    }
    
    pub fn get(mut self, path: impl Into<String>, handler: impl Handler<S> + 'static) -> Self {
        self.routes.push(Route::Get {
            path: path.into(),
            handler: Arc::new(handler),
        });
        self
    }
    
    pub fn build(self) -> Router<S> {
        let mut router = Router::new();
        
        for route in self.routes {
            router = match route {
                Route::Get { path, handler } => {
                    router.route(&format!("{}{}", self.prefix, path), get(move |state| handler.call(state)))
                }
                // ... handle other methods
            };
        }
        
        router
    }
}
```

### View System with Tera Templates

```rust
// src/framework/view.rs
use tera::{Tera, Context};
use serde::Serialize;

/// View renderer
pub struct ViewEngine {
    tera: Tera,
}

impl ViewEngine {
    pub fn new(template_dir: &str) -> Result<Self, tera::Error> {
        let tera = Tera::new(&format!("{}/**/*.html", template_dir))?;
        Ok(ViewEngine { tera })
    }
    
    pub fn render<T: Serialize>(&self, template: &str, context: &T) -> Result<String, tera::Error> {
        let ctx = Context::from_serialize(context)?;
        self.tera.render(template, &ctx)
    }
}

/// View context derive macro
/// #[derive(ViewContext)]
/// struct HomeContext {
///     title: String,
///     user: Option<User>,
/// }
```

## X-ray Scraper - Rust Implementation

```rust
// src/scraper/mod.rs
use scraper::{Html, Selector};
use reqwest::Client;
use serde::{Serialize, Deserialize};

/// Selector for extracting data
#[derive(Debug, Clone)]
pub struct Selector {
    pub css: String,
    pub attribute: Option<String>,
    pub filters: Vec<Filter>,
}

/// Filter for transforming extracted values
#[derive(Debug, Clone)]
pub enum Filter {
    Trim,
    Uppercase,
    Lowercase,
    Regex(String),
    Custom(Box<dyn Fn(&str) -> String + Send + Sync>),
}

/// Extraction schema
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Schema {
    pub fields: Vec<Field>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Field {
    pub name: String,
    pub selector: String,
    pub attribute: Option<String>,
    #[serde(default)]
    pub filters: Vec<String>,
}

/// Scraper builder
pub struct ScraperBuilder {
    client: Client,
    concurrency: usize,
    delay: std::time::Duration,
    max_pages: usize,
}

impl ScraperBuilder {
    pub fn new() -> Self {
        ScraperBuilder {
            client: Client::builder()
                .user_agent("Mozilla/5.0 (compatible; Xray/1.0)")
                .build()
                .unwrap(),
            concurrency: 3,
            delay: std::time::Duration::from_secs(2),
            max_pages: 10,
        }
    }
    
    pub fn concurrency(mut self, n: usize) -> Self {
        self.concurrency = n;
        self
    }
    
    pub fn delay(mut self, delay: std::time::Duration) -> Self {
        self.delay = delay;
        self
    }
    
    pub fn max_pages(mut self, n: usize) -> Self {
        self.max_pages = n;
        self
    }
    
    pub fn build(self) -> Scraper {
        Scraper {
            client: self.client,
            concurrency: self.concurrency,
            delay: self.delay,
            max_pages: self.max_pages,
        }
    }
}

/// Main scraper
pub struct Scraper {
    client: Client,
    concurrency: usize,
    delay: std::time::Duration,
    max_pages: usize,
}

impl Scraper {
    /// Scrape a single page
    pub async fn scrape<T: for<'de> Deserialize<'de>>(
        &self,
        url: &str,
        schema: &Schema,
    ) -> Result<T, ScraperError> {
        let html = self.fetch(url).await?;
        let document = Html::parse_document(&html);
        
        // Extract data based on schema
        // This would use a macro or codegen to map schema to struct
        todo!()
    }
    
    /// Scrape with pagination
    pub async fn scrape_paginated<T: for<'de> Deserialize<'de>>(
        &self,
        url: &str,
        schema: &Schema,
        next_selector: &str,
    ) -> Result<Vec<T>, ScraperError> {
        let mut results = Vec::new();
        let mut current_url = url.to_string();
        
        for _ in 0..self.max_pages {
            let items = self.scrape::<T>(&current_url, schema).await?;
            results.push(items);
            
            // Find next page
            let html = self.fetch(&current_url).await?;
            let document = Html::parse_document(&html);
            let next_selector = Selector::parse(next_selector).unwrap();
            
            if let Some(next_link) = document.select(&next_selector).next() {
                if let Some(href) = next_link.value().attr("href") {
                    current_url = self.resolve_url(&current_url, href);
                } else {
                    break;
                }
            } else {
                break;
            }
            
            tokio::time::sleep(self.delay).await;
        }
        
        Ok(results)
    }
    
    async fn fetch(&self, url: &str) -> Result<String, ScraperError> {
        let response = self.client.get(url).send().await?;
        Ok(response.text().await?)
    }
    
    fn resolve_url(&self, base: &str, href: &str) -> String {
        if href.starts_with("http") {
            href.to_string()
        } else {
            // Resolve relative URL
            todo!()
        }
    }
}

#[derive(Debug)]
pub enum ScraperError {
    Http(reqwest::Error),
    Parse(String),
    Timeout,
}
```

## LLM Agent Framework - Rust Implementation

```rust
// src/llm/mod.rs
use async_trait::async_trait;
use serde::{Serialize, Deserialize};
use std::pin::Pin;
use futures::stream::Stream;

/// Message role
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Role {
    System,
    User,
    Assistant,
    Tool,
}

/// Chat message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub role: Role,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_call_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_call: Option<ToolCall>,
}

/// Tool call from model
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCall {
    pub id: String,
    pub name: String,
    pub arguments: serde_json::Value,
}

/// Chat response chunk
#[derive(Debug, Clone)]
pub struct ChatResponse {
    pub role: Role,
    pub content: Option<String>,
    pub tool_call: Option<ToolCall>,
    pub usage: Option<Usage>,
    pub done: bool,
}

/// Token usage
#[derive(Debug, Clone, Default)]
pub struct Usage {
    pub input_tokens: usize,
    pub output_tokens: usize,
    pub total_tokens: usize,
}

/// Provider trait
#[async_trait]
pub trait Provider: Send + Sync {
    fn name(&self) -> &'static str;
    
    async fn models(&self) -> Result<Vec<Model>, ProviderError>;
    
    fn chat(
        &self,
        request: ChatRequest,
    ) -> Pin<Box<dyn Stream<Item = Result<ChatResponse, ProviderError>> + Send>>;
}

/// Chat request
#[derive(Debug, Clone)]
pub struct ChatRequest {
    pub model: String,
    pub messages: Vec<Message>,
    pub tools: Vec<ToolSchema>,
    pub thinking: ThinkingLevel,
}

/// Thinking/reasoning level
#[derive(Debug, Clone, Default)]
pub enum ThinkingLevel {
    None,
    Low,
    #[default]
    Medium,
    High,
}

/// Tool schema
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolSchema {
    #[serde(rename = "type")]
    pub schema_type: String,
    pub function: FunctionSchema,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionSchema {
    pub name: String,
    pub description: String,
    pub parameters: serde_json::Value,
}

/// Tool trait for user-defined tools
#[async_trait]
pub trait Tool: Send + Sync {
    fn schema(&self) -> ToolSchema;
    
    async fn run(&self, input: serde_json::Value) -> Result<serde_json::Value, ToolError>;
}

/// LLM Client
pub struct Client {
    providers: Vec<Box<dyn Provider>>,
}

impl Client {
    pub fn new() -> Self {
        Client {
            providers: Vec::new(),
        }
    }
    
    pub fn add_provider<P: Provider + 'static>(&mut self, provider: P) {
        self.providers.push(Box::new(provider));
    }
    
    pub async fn chat(
        &self,
        provider: &str,
        config: ChatConfig,
    ) -> Pin<Box<dyn Stream<Item = Result<ChatResponse, ProviderError>> + Send>> {
        let provider = self.providers
            .iter()
            .find(|p| p.name() == provider)
            .expect("Provider not found");
        
        provider.chat(ChatRequest {
            model: config.model,
            messages: config.messages,
            tools: config.tools.iter().map(|t| t.schema()).collect(),
            thinking: config.thinking,
        })
    }
}

/// Chat configuration
pub struct ChatConfig {
    pub model: String,
    pub messages: Vec<Message>,
    pub tools: Vec<Box<dyn Tool>>,
    pub thinking: ThinkingLevel,
    pub max_steps: usize,
}
```

### Tool System with Macros

```rust
// src/llm/tool_macro.rs
use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, ItemFn};

/// Derive macro for tool schema generation
/// #[tool(name = "add", description = "Add two numbers")]
/// fn add(a: i32, b: i32) -> i32 {
///     a + b
/// }
#[proc_macro_attribute]
pub fn tool(attr: TokenStream, item: TokenStream) -> TokenStream {
    let args = parse_macro_input!(attr as ToolArgs);
    let func = parse_macro_input!(item as ItemFn);
    
    let name = &args.name;
    let description = &args.description;
    let func_name = &func.sig.ident;
    
    // Generate schema
    let schema = generate_schema(&func);
    
    quote! {
        #func
        
        fn #func_name##_schema() -> ToolSchema {
            #schema
        }
    }.into()
}

struct ToolArgs {
    name: String,
    description: String,
}

fn generate_schema(func: &ItemFn) -> proc_macro2::TokenStream {
    let mut properties = Vec::new();
    let mut required = Vec::new();
    
    for input in &func.sig.inputs {
        if let syn::FnArg::Typed(pat) = input {
            let name = if let syn::Pat::Ident(ident) = &*pat.pat {
                &ident.ident
            } else {
                continue;
            };
            
            let ty = &pat.ty;
            required.push(quote! { stringify!(#name).to_string() });
            properties.push(quote! {
                props.insert(
                    stringify!(#name).to_string(),
                    json!({
                        "type": get_json_type::<#ty>(),
                        "description": ""
                    })
                );
            });
        }
    }
    
    quote! {
        use serde_json::json;
        use std::collections::HashMap;
        
        let mut props: HashMap<String, serde_json::Value> = HashMap::new();
        #(#properties)*
        
        ToolSchema {
            schema_type: "object".to_string(),
            function: FunctionSchema {
                name: stringify!(#func_name).replace("_tool", "").to_string(),
                description: "".to_string(),
                parameters: json!({
                    "type": "object",
                    "properties": props,
                    "required": vec![#(#required),*]
                }),
            },
        }
    }
}
```

## Summary

Key differences when implementing Joy-like tools in Rust:

1. **Type Safety** - Rust's type system catches errors at compile time
2. **Zero-Cost Abstractions** - No runtime overhead from framework abstractions
3. **Memory Safety** - No garbage collector needed
4. **Async Runtime** - tokio for concurrent operations
5. **Trait-based Polymorphism** - Replaces Go interfaces
6. **Macros** - Code generation at compile time
7. **Ownership** - Clear ownership semantics for AST nodes

Recommended crates:
- `syn` + `quote` - For parsing and code generation
- `petgraph` - For dependency graphs
- `axum` - For web framework (Bud equivalent)
- `scraper` + `reqwest` - For web scraping (X-ray equivalent)
- `tokio` - For async runtime
- `serde` - For JSON serialization
- `tera` - For template rendering
