---
source: /home/darkvoid/Boxxed/@dev/repo-expolorations/llamacpp/TabbyML
repository: https://github.com/TabbyML/tabby
revised_at: 2026-04-02
workspace: tabby-rust-workspace
---

# Rust Revision: TabbyML in Rust

## Overview

This document provides guidance on implementing a TabbyML-like system in Rust. While TabbyML is already primarily written in Rust, this guide focuses on:
- Building custom components from scratch
- Understanding the core algorithms
- Creating a minimal implementation
- Production-ready patterns

## Table of Contents

1. [Core Architecture](#1-core-architecture)
2. [Inference Engine Integration](#2-inference-engine-integration)
3. [Code Completion Pipeline](#3-code-completion-pipeline)
4. [Search Index Implementation](#4-search-index-implementation)
5. [HTTP API Server](#5-http-api-server)
6. [IDE Integration Library](#6-ide-integration-library)

---

## 1. Core Architecture

### Workspace Structure

```toml
# Cargo.toml
[workspace]
members = [
    "crates/tabby-core",
    "crates/tabby-inference",
    "crates/tabby-index",
    "crates/tabby-api",
    "crates/tabby-client",
]

[workspace.dependencies]
tokio = { version = "1", features = ["full"] }
async-trait = "0.1"
serde = { version = "1", features = ["derive"] }
serde_json = "1"
tantivy = "0.21"
tree-sitter = "0.20"
llama-cpp-2 = "0.1"
axum = "0.7"
```

### Core Traits

```rust
// crates/tabby-core/src/lib.rs

use async_trait::async_trait;
use futures::stream::BoxStream;

/// Core completion trait
#[async_trait]
pub trait CompletionEngine: Send + Sync {
    async fn complete(&self, prompt: &str, options: CompletionOptions) -> Completion;
    async fn complete_stream(
        &self,
        prompt: &str,
        options: CompletionOptions,
    ) -> BoxStream<'_, String>;
}

/// Completion options
#[derive(Debug, Clone)]
pub struct CompletionOptions {
    pub max_tokens: u32,
    pub temperature: f32,
    pub seed: u64,
    pub stop_words: Vec<String>,
}

/// Completion result
#[derive(Debug, Clone)]
pub struct Completion {
    pub id: String,
    pub text: String,
    pub finish_reason: FinishReason,
    pub usage: CompletionUsage,
}

#[derive(Debug, Clone, PartialEq)]
pub enum FinishReason {
    Stop,
    Length,
    Error,
}

#[derive(Debug, Clone, Default)]
pub struct CompletionUsage {
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
    pub total_tokens: u32,
}

/// Error types
#[derive(Debug, thiserror::Error)]
pub enum TabbyError {
    #[error("Model error: {0}")]
    Model(String),

    #[error("Index error: {0}")]
    Index(#[from] tantivy::TantivyError),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("API error: {0}")]
    Api(String),
}
```

---

## 2. Inference Engine Integration

### llama.cpp Wrapper

```rust
// crates/tabby-inference/src/llama.rs

use llama_cpp_2::{
    context::LlamaContext,
    model::LlamaModel,
    backend::LlamaBackend,
};
use std::sync::Arc;

pub struct LlamaCompletionEngine {
    model: Arc<LlamaModel>,
    context: Arc<LlamaContext>,
}

impl LlamaCompletionEngine {
    pub fn new(model_path: &str, n_gpu_layers: u32) -> Result<Self, TabbyError> {
        let backend = LlamaBackend::init().unwrap();
        let model = backend
            .load_model_from_file(model_path)
            .map_err(|e| TabbyError::Model(e.to_string()))?;

        let context = model
            .new_context(&backend, 4096)
            .map_err(|e| TabbyError::Model(e.to_string()))?;

        Ok(Self {
            model: Arc::new(model),
            context: Arc::new(context),
        })
    }

    fn format_prompt(&self, prefix: &str, suffix: &str) -> String {
        format!("<PRE>{}<SUF>{}<MID>", prefix, suffix)
    }
}

#[async_trait]
impl CompletionEngine for LlamaCompletionEngine {
    async fn complete(&self, prompt: &str, options: CompletionOptions) -> Completion {
        let mut ctx = self.context.clone();
        let tokens = ctx.tokenizer().encode(prompt, false).unwrap();

        ctx.decode(tokens.as_slice(), false).unwrap();

        let mut output_tokens = Vec::new();
        let mut rng = rand::thread_rng();

        for _ in 0..options.max_tokens {
            let logits = ctx.candidates();
            let token_id = sample_token(logits, options.temperature, &mut rng);

            if is_stop_token(token_id, &options.stop_words) {
                break;
            }

            output_tokens.push(token_id);
            ctx.eval(&[token_id]).unwrap();
        }

        let text = ctx.tokenizer().decode(&output_tokens, false).unwrap();

        Completion {
            id: format!("cmpl-{}", uuid::Uuid::new_v4()),
            text,
            finish_reason: FinishReason::Stop,
            usage: CompletionUsage {
                prompt_tokens: tokens.len() as u32,
                completion_tokens: output_tokens.len() as u32,
                total_tokens: (tokens.len() + output_tokens.len()) as u32,
            },
        }
    }

    async fn complete_stream(
        &self,
        prompt: &str,
        options: CompletionOptions,
    ) -> BoxStream<'_, String> {
        use futures::stream;

        let model = Arc::clone(&self.model);
        let context = Arc::clone(&self.context);
        let prompt = prompt.to_string();

        let stream = stream::unfold(
            (context, prompt, options, Vec::new(), false),
            |(ctx, prompt, opts, mut tokens, done)| async move {
                if done {
                    return None;
                }

                // Generate one token
                let new_token = generate_one_token(&ctx, &prompt, &opts).await;

                if new_token.is_none() {
                    return None;
                }

                let token_text = ctx.tokenizer().decode(&[new_token.unwrap()], false).unwrap();
                tokens.push(new_token.unwrap());

                Some((
                    token_text,
                    (ctx, prompt, opts, tokens, true),
                ))
            },
        );

        Box::pin(stream)
    }
}

fn sample_token(logits: &[f32], temperature: f32, rng: &mut impl rand::Rng) -> u32 {
    if temperature == 0.0 {
        // Greedy sampling
        logits
            .iter()
            .enumerate()
            .max_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap())
            .map(|(i, _)| i as u32)
            .unwrap()
    } else {
        // Temperature sampling
        let exp_logits: Vec<f32> = logits
            .iter()
            .map(|&l| (l / temperature).exp())
            .collect();

        let sum: f32 = exp_logits.iter().sum();
        let probs: Vec<f32> = exp_logits.iter().map(|&e| e / sum).collect();

        // Sample from distribution
        let r = rng.gen::<f32>();
        let mut cumsum = 0.0;

        for (i, &prob) in probs.iter().enumerate() {
            cumsum += prob;
            if cumsum >= r {
                return i as u32;
            }
        }

        probs.len() as u32 - 1
    }
}
```

### Stop Condition Implementation

```rust
// crates/tabby-inference/src/stop_condition.rs

use dashmap::DashMap;
use trie_rs::{Trie, TrieBuilder};

pub struct StopConditionFactory {
    cache: DashMap<String, Trie<u8>>,
    stop_words: Vec<String>,
}

impl StopConditionFactory {
    pub fn new(stop_words: Vec<String>) -> Self {
        Self {
            cache: DashMap::new(),
            stop_words,
        }
    }

    pub fn create(&self, language: &str) -> StopCondition {
        let trie = self.get_trie(language);

        StopCondition {
            trie,
            reversed_text: String::new(),
            num_decoded: 0,
        }
    }

    fn get_trie(&self, language: &str) -> Option<Trie<u8>> {
        if let Some(cached) = self.cache.get(language) {
            return Some(cached.clone());
        }

        let words = self.get_stop_words_for_language(language);
        if words.is_empty() {
            return None;
        }

        let mut builder = TrieBuilder::new();
        for word in words {
            builder.push(word.chars().rev().collect::<String>());
        }
        let trie = builder.build();

        self.cache.insert(language.to_string(), trie.clone());
        Some(trie)
    }

    fn get_stop_words_for_language(&self, language: &str) -> Vec<String> {
        match language {
            "rust" => vec![
                "\n\nfn ".to_string(),
                "\n\nimpl ".to_string(),
                "\n}".to_string(),
            ],
            "python" => vec![
                "\n\nclass ".to_string(),
                "\n\ndef ".to_string(),
                "\n\nif ".to_string(),
            ],
            _ => self.stop_words.clone(),
        }
    }
}

pub struct StopCondition {
    trie: Option<Trie<u8>>,
    reversed_text: String,
    num_decoded: usize,
}

impl StopCondition {
    pub fn should_stop(&mut self, new_text: &str) -> (bool, usize) {
        self.num_decoded += 1;

        if !new_text.is_empty() {
            self.reversed_text = new_text.chars().rev().collect::<String>() + &self.reversed_text;

            if let Some(trie) = &self.trie {
                let matches = trie.common_prefix_search(&self.reversed_text);
                if let Some(max_len) = matches.iter().map(|m| m.len()).max() {
                    return (true, max_len);
                }
            }
        }

        (false, 0)
    }
}
```

---

## 3. Code Completion Pipeline

### Context Builder

```rust
// crates/tabby-core/src/context.rs

use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct CompletionContext {
    pub filepath: PathBuf,
    pub language: String,
    pub content: String,
    pub position: usize,
    pub prefix: String,
    pub suffix: String,
    pub declarations: Vec<DeclarationSnippet>,
    pub recently_changed: Vec<CodeSnippet>,
}

#[derive(Debug, Clone)]
pub struct DeclarationSnippet {
    pub symbol: String,
    pub content: String,
    pub filepath: PathBuf,
}

#[derive(Debug, Clone)]
pub struct CodeSnippet {
    pub content: String,
    pub filepath: PathBuf,
    pub timestamp: u64,
}

pub struct ContextBuilder {
    max_prefix_length: usize,
    max_suffix_length: usize,
    max_declarations: usize,
}

impl ContextBuilder {
    pub fn new() -> Self {
        Self {
            max_prefix_length: 1024,
            max_suffix_length: 256,
            max_declarations: 5,
        }
    }

    pub fn build(&self, content: &str, position: usize) -> CompletionContext {
        let prefix = if position > self.max_prefix_length {
            content[(position - self.max_prefix_length)..position].to_string()
        } else {
            content[..position].to_string()
        };

        let suffix = content[position..].chars().take(self.max_suffix_length).collect();

        CompletionContext {
            filepath: PathBuf::new(),
            language: "unknown".to_string(),
            content: content.to_string(),
            position,
            prefix,
            suffix,
            declarations: Vec::new(),
            recently_changed: Vec::new(),
        }
    }

    pub fn with_declarations(mut self, declarations: Vec<DeclarationSnippet>) -> Self {
        self.max_declarations = declarations.len();
        CompletionContext {
            declarations,
            ..Default::default()
        }
    }
}

impl Default for CompletionContext {
    fn default() -> Self {
        Self {
            filepath: PathBuf::new(),
            language: "unknown".to_string(),
            content: String::new(),
            position: 0,
            prefix: String::new(),
            suffix: String::new(),
            declarations: Vec::new(),
            recently_changed: Vec::new(),
        }
    }
}
```

### Post-processing Pipeline

```rust
// crates/tabby-core/src/postprocess.rs

pub trait PostProcessor: Send + Sync {
    fn process(&self, text: &str, context: &CompletionContext) -> Option<String>;
}

pub struct PostProcessPipeline {
    processors: Vec<Box<dyn PostProcessor>>,
}

impl PostProcessPipeline {
    pub fn new() -> Self {
        Self {
            processors: vec![
                Box::new(RepetitiveBlocksRemover),
                Box::new(RepetitiveLinesRemover),
                Box::new(ScopeLimiter),
                Box::new(IndentationFormatter),
                Box::new(DuplicatedRemover),
                Box::new(SpaceTrimmer),
                Box::new(MinimumChecker),
            ],
        }
    }

    pub fn process(&self, text: &str, context: &CompletionContext) -> Option<String> {
        let mut current = text.to_string();

        for processor in &self.processors {
            match processor.process(&current, context) {
                Some(processed) => current = processed,
                None => return None, // Processor filtered out the completion
            }
        }

        Some(current)
    }
}

// Remove repetitive blocks
pub struct RepetitiveBlocksRemover;

impl PostProcessor for RepetitiveBlocksRemover {
    fn process(&self, text: &str, _context: &CompletionContext) -> Option<String> {
        let lines: Vec<&str> = text.lines().collect();

        for pattern_len in 3..=(lines.len() / 2) {
            for start in 0..(lines.len() - pattern_len * 2) {
                let pattern = &lines[start..start + pattern_len];
                let next_pattern = &lines[start + pattern_len..start + pattern_len * 2];

                if pattern == next_pattern {
                    return Some(lines[..start + pattern_len].join("\n"));
                }
            }
        }

        Some(text.to_string())
    }
}

// Limit to current scope
pub struct ScopeLimiter;

impl PostProcessor for ScopeLimiter {
    fn process(&self, text: &str, context: &CompletionContext) -> Option<String> {
        let mut brace_balance = 0;
        let mut result = String::new();

        for ch in text.chars() {
            if ch == '{' {
                brace_balance += 1;
            } else if ch == '}' {
                brace_balance -= 1;
            }

            if brace_balance < 0 {
                break;
            }

            result.push(ch);
        }

        Some(result)
    }
}

// Format indentation
pub struct IndentationFormatter;

impl PostProcessor for IndentationFormatter {
    fn process(&self, text: &str, context: &CompletionContext) -> Option<String> {
        let prefix_last_line = context.prefix.lines().last().unwrap_or("");
        let expected_indent = prefix_last_line.chars().take_while(|c| c.is_whitespace()).collect::<String>();

        let lines: Vec<String> = text.lines().enumerate().map(|(i, line)| {
            if i == 0 {
                line.to_string()
            } else {
                format!("{}{}", expected_indent, line.trim_start())
            }
        }).collect();

        Some(lines.join("\n"))
    }
}

// Drop minimum length
pub struct MinimumChecker;

impl PostProcessor for MinimumChecker {
    fn process(&self, text: &str, _context: &CompletionContext) -> Option<String> {
        if text.trim().len() < 3 {
            None
        } else {
            Some(text.to_string())
        }
    }
}
```

---

## 4. Search Index Implementation

```rust
// crates/tabby-index/src/lib.rs

use tantivy::{
    doc,
    schema::{Schema, TEXT, STORED, FAST, Field},
    Index, IndexWriter, Searcher, Term,
    query::{Query, TermQuery, BooleanQuery},
    collector::TopDocs,
};
use std::sync::Arc;

pub struct CodeIndex {
    schema: IndexSchema,
    index: Index,
    writer: IndexWriter,
    searcher: Searcher,
}

struct IndexSchema {
    schema: Schema,
    id: Field,
    source_id: Field,
    corpus: Field,
    chunk_id: Field,
    attributes: Field,
    tokens: Field,
}

impl CodeIndex {
    pub fn open_or_create(path: &str) -> tantivy::Result<Self> {
        let schema = Self::build_schema();

        let index = Index::open_or_create(tantivy::directory::MmapDirectory::open(path)?, schema.schema.clone())?;
        let writer = index.writer(150_000_000)?;
        let reader = index.reader()?;

        Ok(Self {
            schema,
            index,
            writer,
            searcher: reader.searcher(),
        })
    }

    fn build_schema() -> IndexSchema {
        let mut builder = Schema::builder();

        let id = builder.add_text_field("id", TEXT | STORED | FAST);
        let source_id = builder.add_text_field("source_id", TEXT | STORED);
        let corpus = builder.add_text_field("corpus", TEXT | STORED | FAST);
        let chunk_id = builder.add_text_field("chunk_id", TEXT | STORED);
        let attributes = builder.add_json_field("attributes", Default::default());
        let tokens = builder.add_text_field("tokens", TEXT);

        IndexSchema {
            schema: builder.build(),
            id,
            source_id,
            corpus,
            chunk_id,
            attributes,
            tokens,
        }
    }

    pub fn add_document(&mut self, doc: CodeDocument) -> tantivy::Result<()> {
        let tantivy_doc = doc! {
            self.schema.id => doc.id,
            self.schema.source_id => doc.source_id,
            self.schema.corpus => "code",
            self.schema.chunk_id => format!("{}-0", doc.id),
            self.schema.attributes => doc.attributes,
        };

        // Add tokenized content
        let mut indexed_doc = tantivy_doc;
        for token in tokenize(&doc.content) {
            indexed_doc.add_text(self.schema.tokens, token);
        }

        self.writer.add_document(indexed_doc)?;
        Ok(())
    }

    pub fn search(&self, query: &str, limit: usize) -> tantivy::Result<Vec<SearchResult>> {
        let tokens = tokenize(query);

        let mut boolean_query = BooleanQuery::builder();
        for token in tokens {
            let term_query = TermQuery::new(
                Term::from_field_text(self.schema.tokens, &token),
                tantivy::schema::IndexRecordOption::WithFreqs,
            );
            boolean_query.add(tantivy::query::Occur::Should, Box::new(term_query));
        }

        let collector = TopDocs::with_limit(limit);
        let results = self.searcher.search(&boolean_query.build(), &collector)?;

        let mut search_results = Vec::new();
        for (_score, doc_address) in results {
            let doc = self.searcher.doc::<tantivy::TantivyDocument>(doc_address)?;
            search_results.push(SearchResult::from_tantivy_doc(&doc));
        }

        Ok(search_results)
    }

    pub fn commit(&mut self) -> tantivy::Result<()> {
        self.writer.commit()?;
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct CodeDocument {
    pub id: String,
    pub source_id: String,
    pub content: String,
    pub attributes: serde_json::Value,
}

#[derive(Debug, Clone)]
pub struct SearchResult {
    pub id: String,
    pub source_id: String,
    pub content: String,
    pub score: f32,
}

fn tokenize(text: &str) -> Vec<String> {
    text.split(|c: char| !c.is_alphanumeric())
        .filter(|s| !s.is_empty())
        .map(|s| s.to_lowercase())
        .collect()
}
```

---

## 5. HTTP API Server

```rust
// crates/tabby-api/src/server.rs

use axum::{
    extract::State,
    json::Json,
    routing::post,
    Router,
};
use tower_http::cors::{CorsLayer, Any};

pub struct TabbyServer {
    engine: Arc<dyn CompletionEngine>,
    index: Arc<CodeIndex>,
}

impl TabbyServer {
    pub fn new(engine: Arc<dyn CompletionEngine>, index: Arc<CodeIndex>) -> Self {
        Self { engine, index }
    }

    pub fn into_router(self) -> Router {
        let state = Arc::new(ServerState {
            engine: self.engine,
            index: self.index,
        });

        Router::new()
            .route("/v1/completions", post(completions_handler))
            .route("/v1/chat/completions", post(chat_completions_handler))
            .route("/v1/search", post(search_handler))
            .route("/v1/health", post(health_handler))
            .layer(CorsLayer::new().allow_origin(Any).allow_methods(Any).allow_headers(Any))
            .with_state(state)
    }
}

struct ServerState {
    engine: Arc<dyn CompletionEngine>,
    index: Arc<CodeIndex>,
}

#[derive(serde::Deserialize)]
struct CompletionRequest {
    model: String,
    prompt: String,
    suffix: Option<String>,
    max_tokens: Option<u32>,
    temperature: Option<f32>,
}

#[derive(serde::Serialize)]
struct CompletionResponse {
    id: String,
    choices: Vec<CompletionChoice>,
    usage: CompletionUsage,
}

#[derive(serde::Serialize)]
struct CompletionChoice {
    index: u32,
    text: String,
    finish_reason: String,
}

async fn completions_handler(
    State(state): State<Arc<ServerState>>,
    Json(req): Json<CompletionRequest>,
) -> Json<CompletionResponse> {
    let options = CompletionOptions {
        max_tokens: req.max_tokens.unwrap_or(256),
        temperature: req.temperature.unwrap_or(0.1),
        seed: 0,
        stop_words: vec![],
    };

    let prompt = if let Some(suffix) = &req.suffix {
        format!("<PRE>{}<SUF>{}<MID>", req.prompt, suffix)
    } else {
        req.prompt.clone()
    };

    let completion = state.engine.complete(&prompt, options).await;

    Json(CompletionResponse {
        id: completion.id,
        choices: vec![CompletionChoice {
            index: 0,
            text: completion.text,
            finish_reason: match completion.finish_reason {
                FinishReason::Stop => "stop".to_string(),
                FinishReason::Length => "length".to_string(),
                FinishReason::Error => "error".to_string(),
            },
        }],
        usage: completion.usage,
    })
}

async fn health_handler() -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "status": "ok",
        "model": "tabby-rust",
    }))
}
```

---

## 6. IDE Integration Library

### LSP Server

```rust
// crates/tabby-client/src/lsp.rs

use lsp_server::{Connection, Message, Request, Response};
use lsp_types::*;

pub struct TabbyLspServer {
    connection: Connection,
    engine: Arc<dyn CompletionEngine>,
    documents: DashMap<String, String>,
}

impl TabbyLspServer {
    pub fn new(engine: Arc<dyn CompletionEngine>) -> Self {
        let connection = Connection::stdio();

        Self {
            connection,
            engine,
            documents: DashMap::new(),
        }
    }

    pub fn run(&self) -> Result<(), Box<dyn std::error::Error>> {
        // Initialize
        let initialize_params: InitializeParams = serde_json::from_value(
            self.connection.initialize_start()?
        )?;

        let capabilities = ServerCapabilities {
            text_document_sync: Some(TextDocumentSyncCapability::Kind(
                TextDocumentSyncKind::INCREMENTAL,
            )),
            completion_provider: Some(CompletionOptions {
                trigger_characters: Some(vec![".".to_string(), "(".to_string()]),
                ..Default::default()
            }),
            ..Default::default()
        };

        self.connection.initialize_finish(serde_json::to_value(capabilities)?)?;

        // Main loop
        for message in &self.connection.receiver {
            match message {
                Message::Request(req) => {
                    self.handle_request(&req)?;
                }
                Message::Notification(notif) => {
                    self.handle_notification(&notif)?;
                }
                _ => {}
            }
        }

        Ok(())
    }

    fn handle_request(&self, req: &Request) -> Result<(), Box<dyn std::error::Error>> {
        match req.method.as_str() {
            "textDocument/completion" => {
                let params: CompletionParams = serde_json::from_value(req.params.clone())?;
                let completions = self.provide_completions(&params)?;

                let result = serde_json::to_value(&completions)?;
                self.connection.sender.send(Message::Response(Response {
                    id: req.id.clone(),
                    result: Some(result),
                    error: None,
                }))?;
            }
            _ => {}
        }

        Ok(())
    }

    fn handle_notification(&self, notif: &lsp_server::Notification) -> Result<(), Box<dyn std::error::Error>> {
        match notif.method.as_str() {
            "textDocument/didOpen" | "textDocument/didChange" => {
                let params: DidChangeTextDocumentParams = serde_json::from_value(notif.params.clone())?;
                let content = params.content_changes[0].text.clone();
                let uri = params.text_document.uri.to_string();
                self.documents.insert(uri, content);
            }
            _ => {}
        }

        Ok(())
    }

    fn provide_completions(&self, params: &CompletionParams) -> Result<CompletionList, Box<dyn std::error::Error>> {
        let uri = params.text_document_position.text_document.uri.to_string();
        let position = params.text_document_position.position;

        let content = self.documents.get(&uri)
            .map(|c| c.value().clone())
            .unwrap_or_default();

        let offset = position_to_offset(&content, position);
        let prefix = &content[..offset];
        let suffix = &content[offset..];

        let prompt = format!("<PRE>{}<SUF>{}<MID>", prefix, suffix);

        let completion = futures::executor::block_on(
            self.engine.complete(&prompt, CompletionOptions::default())
        );

        let items = vec![CompletionItem {
            label: completion.text.clone(),
            kind: Some(CompletionItemKind::TEXT),
            insert_text: Some(completion.text),
            ..Default::default()
        }];

        Ok(CompletionList {
            is_incomplete: false,
            items,
        })
    }
}

fn position_to_offset(content: &str, position: Position) -> usize {
    let mut offset = 0;
    for (line_num, line) in content.lines().enumerate() {
        if line_num == position.line as usize {
            return offset + position.character as usize;
        }
        offset += line.len() + 1; // +1 for newline
    }
    offset
}
```

---

## Conclusion

This Rust revision demonstrates:
- Core trait design for completion engines
- llama.cpp integration for inference
- Post-processing pipeline with trait objects
- Tantivy-based search indexing
- Axum-based HTTP API
- LSP server implementation

The key insight is that **Rust's type system and ownership model enable safe, efficient implementations** of all TabbyML's core components.
