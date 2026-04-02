---
source: /home/darkvoid/Boxxed/@formulas/src.rust/src.llamacpp/src.AICoders/src.TabbyML/tabby
explored_at: 2026-04-02
---

# TabbyML Indexing and Search Deep Dive

## Overview

This document explores TabbyML's indexing and search architecture, covering:
- Tantivy search engine fundamentals
- Code indexing with tree-sitter
- Structured document indexing
- RAG (Retrieval-Augmented Generation)
- Performance optimization

## Table of Contents

1. [Tantivy Search Engine](#1-tantivy-search-engine)
2. [Code Indexing Pipeline](#2-code-indexing-pipeline)
3. [Structured Document Indexing](#3-structured-document-indexing)
4. [RAG Implementation](#4-rag-implementation)
5. [Index Optimization](#5-index-optimization)
6. [Rust Implementation Guide](#6-rust-implementation-guide)

---

## 1. Tantivy Search Engine

### What is Tantivy?

Tantivy is a full-text search engine library for Rust, inspired by Apache Lucene. It provides:
- Inverted index for fast text search
- BM25 scoring for relevance
- Faceted search capabilities
- Document storage

### Index Schema

```rust
// From crates/tabby-common/src/index/schema.rs

pub struct IndexSchema {
    pub schema: Schema,

    // Core fields
    pub field_id: Field,              // Document ID (e.g., "file://path/to/file.rs")
    pub field_source_id: Field,       // Source identifier (e.g., "git://github.com/org/repo")
    pub field_corpus: Field,          // Document type ("code", "issue", "doc")
    pub field_chunk_id: Field,        // Chunk identifier within document
    pub field_attributes: Field,      // JSON attributes (language, symbols, etc.)
    pub field_chunk_attributes: Field,// Chunk-specific JSON attributes
    pub field_chunk_tokens: Field,    // Tokenized text content
    pub field_updated_at: Field,      // Last update timestamp (DateTime)
    pub field_failed_chunks_count: Field, // Count of failed chunk embeddings
}

impl IndexSchema {
    pub fn instance() -> &'static Self {
        static SCHEMA: OnceLock<IndexSchema> = OnceLock::new();
        SCHEMA.get_or_init(|| {
            let mut builder = Schema::builder();

            let field_id = builder.add_text_field("id", TEXT | STORED | FAST);
            let field_source_id = builder.add_text_field("source_id", TEXT | STORED | FAST);
            let field_corpus = builder.add_text_field("corpus", TEXT | STORED | FAST);
            let field_chunk_id = builder.add_text_field("chunk_id", TEXT | STORED);
            let field_attributes = builder.add_json_field("attributes", JSON_OPTIONS);
            let field_chunk_attributes = builder.add_json_field("chunk_attributes", JSON_OPTIONS);
            let field_chunk_tokens = builder.add_text_field("chunk_tokens", TEXT);
            let field_updated_at = builder.add_date_field("updated_at", DATE_OPTIONS);
            let field_failed_chunks_count = builder.add_u64_field("failed_chunks_count", INDEXED);

            IndexSchema {
                schema: builder.build(),
                field_id,
                field_source_id,
                field_corpus,
                field_chunk_id,
                field_attributes,
                field_chunk_attributes,
                field_chunk_tokens,
                field_updated_at,
                field_failed_chunks_count,
            }
        })
    }

    // Query helpers
    pub fn doc_query(&self, corpus: &str, id: &str) -> Box<dyn Query> {
        let mut query = BooleanQuery::builder();
        query.add(Required(TermQuery::new(
            Term::from_field_text(self.field_corpus, corpus),
            IndexRecordOption::Basic,
        )));
        query.add(Required(TermQuery::new(
            Term::from_field_text(self.field_id, id),
            IndexRecordOption::Basic,
        )));
        Box::new(query.build())
    }

    pub fn doc_with_attribute_field(
        &self,
        corpus: &str,
        source_id: &str,
        kvs: &Vec<(&str, &str)>,
    ) -> Box<dyn Query> {
        // Build query with attribute filters
        ...
    }
}
```

### Index Structure

```
~/.tabby/index/
├── meta.json           # Index metadata (segment list, schema)
├── segments/           # Tantivy segments
│   ├── segment1.meta   # Segment metadata
│   ├── segment1.term   # Term dictionary
│   ├── segment1.postings # Inverted index
│   └── segment1.store  # Document store
└── commitlog/          # Write-ahead log for durability
```

### Search Process

```rust
// From crates/tabby-index/src/indexer.rs

pub struct Indexer {
    corpus: String,
    searcher: Searcher,
    writer: IndexWriter,
}

impl Indexer {
    pub fn new(corpus: &str) -> Self {
        let schema = IndexSchema::instance();
        let (_, index) = open_or_create_index(&schema.schema, &path::index_dir());

        let writer = index.writer(150_000_000) // 150MB budget
            .expect("Failed to create index writer");
        let reader = index.reader()
            .expect("Failed to create index reader");

        Self {
            corpus: corpus.to_owned(),
            searcher: reader.searcher(),
            writer,
        }
    }

    pub async fn add(&self, document: TantivyDocument) {
        self.writer.add_document(document)
            .expect("Failed to add document");
    }

    pub fn is_indexed(&self, id: &str) -> bool {
        let schema = IndexSchema::instance();
        let query = schema.doc_query(&self.corpus, id);

        match self.searcher.search(&query, &TopDocs::with_limit(1)) {
            Ok(docs) => !docs.is_empty(),
            Err(_) => false,
        }
    }

    pub fn commit(mut self) {
        self.writer.commit()
            .expect("Failed to commit changes");
        self.writer.wait_merging_threads()
            .expect("Failed to wait for merging threads");
    }
}
```

### BM25 Scoring

Tantivy uses BM25 for relevance scoring:

```
score(q, d) = Σ [IDF(qi) * (f(qi, d) * (k1 + 1)) / (f(qi, d) + k1 * (1 - b + b * |d|/avgdl))]

Where:
- qi = query term i
- d = document
- f(qi, d) = term frequency in document
- |d| = document length
- avgdl = average document length
- k1, b = tuning parameters (typically k1=1.2, b=0.75)
```

---

## 2. Code Indexing Pipeline

### Overview

```
┌─────────────────────────────────────────────────────────────────┐
│                    Code Indexing Pipeline                        │
│                                                                  │
│  ┌──────────┐   ┌──────────┐   ┌──────────┐   ┌──────────┐     │
│  │   Git    │──▶│   Tree   │──▶│  Symbol  │──▶│  Chunk   │     │
│  │  Reader  │   │  Sitter  │   │Extractor │   │ Builder  │     │
│  └──────────┘   └──────────┘   └──────────┘   └──────────┘     │
│                                              │                  │
│                                              ▼                  │
│  ┌──────────┐   ┌──────────┐   ┌──────────┐   ┌──────────┐     │
│  │  Tantivy │◀──│  Embed   │◀──│  Filter  │◀──│  Tokenize│     │
│  │  Index   │   │  (opt)   │   │          │   │          │     │
│  └──────────┘   └──────────┘   └──────────┘   └──────────┘     │
└─────────────────────────────────────────────────────────────────┘
```

### Git Repository Reading

```rust
// From crates/tabby-index/src/code/repository.rs

use git2::{Repository, RepositoryOpenFlags};

pub struct GitRepository {
    repo: Repository,
    root: PathBuf,
}

impl GitRepository {
    pub fn open(path: &Path) -> Result<Self> {
        let repo = Repository::open(path)?;
        let root = repo.workdir().unwrap_or(path).to_path_buf();

        Ok(Self { repo, root })
    }

    pub fn iter_files(&self) -> impl Iterator<Item = CodeFile> {
        let mut walker = self.repo.revwalk().unwrap();
        walker.push_head().unwrap();

        let mut files = Vec::new();

        for oid in walker {
            let commit = self.repo.find_commit(oid.unwrap()).unwrap();
            let tree = commit.tree().unwrap();

            tree.walk(TreeWalkMode::PreOrder, |path, entry| {
                if entry.kind() == Some(git2::ObjectType::Blob) {
                    if let Some(file) = self.process_entry(path, entry) {
                        files.push(file);
                    }
                }
                TreeWalkResult::Ok
            }).unwrap();
        }

        files.into_iter()
    }

    fn process_entry(&self, path: &str, entry: &git2::TreeEntry) -> Option<CodeFile> {
        let blob = entry.to_object().as_blob()?;
        let content = std::str::from_utf8(blob.content()).ok()?;

        // Skip binary files, large files, etc.
        if should_skip(path, content) {
            return None;
        }

        let language = detect_language(path, content);

        Some(CodeFile {
            path: path.to_string(),
            content: content.to_string(),
            language,
            size: blob.size(),
        })
    }
}
```

### Tree-sitter Parsing

```rust
// From crates/tabby-index/src/code/intelligence.rs

use tree_sitter::{Parser, Tree, Node};

pub struct CodeIntelligence {
    parser: Parser,
    language: tree_sitter::Language,
}

impl CodeIntelligence {
    pub fn new(language_name: &str) -> Option<Self> {
        let language = get_tree_sitter_language(language_name)?;
        let mut parser = Parser::new();
        parser.set_language(&language).ok()?;

        Some(Self { parser, language })
    }

    pub fn parse(&mut self, content: &str) -> Tree {
        self.parser.parse(content, None).unwrap()
    }

    pub fn extract_symbols(&self, tree: &Tree, content: &str) -> Vec<Symbol> {
        let mut symbols = Vec::new();
        self.extract_symbols_recursive(tree.root_node(), content, &mut symbols, 0);
        symbols
    }

    fn extract_symbols_recursive(
        &self,
        node: Node,
        content: &str,
        symbols: &mut Vec<Symbol>,
        depth: usize,
    ) {
        // Check if this node is a definition
        if let Some(symbol_type) = self.get_symbol_type(node) {
            let name = self.get_symbol_name(node, content);
            let range = node.byte_range();

            symbols.push(Symbol {
                name: name.to_string(),
                symbol_type: symbol_type.to_string(),
                range,
                depth,
                content: content[range].to_string(),
            });
        }

        // Recurse into children
        for child in node.children(&mut node.walk()) {
            self.extract_symbols_recursive(child, content, symbols, depth + 1);
        }
    }

    fn get_symbol_type(&self, node: Node) -> Option<&str> {
        match node.kind() {
            "function_definition" | "function_item" => Some("function"),
            "struct_definition" | "struct_item" => Some("struct"),
            "impl_item" => Some("impl"),
            "trait_definition" => Some("trait"),
            "enum_definition" => Some("enum"),
            "class_definition" => Some("class"),
            _ => None,
        }
    }
}
```

### Symbol Extraction

```rust
// From crates/tabby-index/src/code/intelligence.rs

pub struct Symbol {
    pub name: String,
    pub symbol_type: String,
    pub range: Range<usize>,
    pub depth: usize,
    pub content: String,
}

impl Symbol {
    /// Generate a searchable representation
    pub fn to_indexable_text(&self) -> String {
        format!(
            "{} {} {}",
            self.symbol_type,
            self.name,
            self.content
        )
    }
}

// Example extracted symbols from Rust code:
/*
Input:
    pub fn fibonacci(n: u32) -> u32 {
        if n <= 1 {
            return n;
        }
        fibonacci(n - 1) + fibonacci(n - 2)
    }

Output:
    Symbol {
        name: "fibonacci",
        symbol_type: "function",
        range: 4..108,
        depth: 0,
        content: "pub fn fibonacci(n: u32) -> u32 { ... }"
    }
*/
```

### Chunk Building

```rust
// From crates/tabby-index/src/code/index.rs

pub struct ChunkBuilder {
    max_chunk_size: usize,
    overlap_size: usize,
}

impl ChunkBuilder {
    pub fn new(max_chunk_size: usize, overlap_size: usize) -> Self {
        Self {
            max_chunk_size,
            overlap_size,
        }
    }

    pub fn build_chunks(&self, content: &str, symbols: &[Symbol]) -> Vec<Chunk> {
        let mut chunks = Vec::new();

        // Strategy 1: Chunk by symbols
        for symbol in symbols {
            if symbol.content.len() <= self.max_chunk_size {
                chunks.push(Chunk {
                    content: symbol.content.clone(),
                    metadata: ChunkMetadata {
                        symbol_name: Some(symbol.name.clone()),
                        symbol_type: Some(symbol.symbol_type.clone()),
                        ..Default::default()
                    },
                });
            }
        }

        // Strategy 2: Chunk by size for remaining content
        let covered_ranges = symbols.iter()
            .map(|s| s.range.clone())
            .collect::<Vec<_>>();

        let remaining = self.get_uncovered_content(content, &covered_ranges);

        for chunk_text in self.chunk_by_size(&remaining) {
            chunks.push(Chunk {
                content: chunk_text,
                metadata: ChunkMetadata::default(),
            });
        }

        chunks
    }

    fn chunk_by_size(&self, content: &str) -> Vec<String> {
        let mut chunks = Vec::new();
        let mut current = String::new();

        for line in content.lines() {
            if current.len() + line.len() > self.max_chunk_size {
                // Add overlap
                if !current.is_empty() {
                    chunks.push(current.clone());
                    current = current.lines()
                        .rev()
                        .take(self.overlap_size)
                        .collect::<Vec<_>>()
                        .iter()
                        .rev()
                        .cloned()
                        .collect::<Vec<_>>()
                        .join("\n");
                }
            }
            current.push_str(line);
            current.push('\n');
        }

        if !current.is_empty() {
            chunks.push(current);
        }

        chunks
    }
}
```

### Document Building

```rust
// From crates/tabby-index/src/indexer.rs

pub struct TantivyDocBuilder<T> {
    corpus: &'static str,
    builder: Box<dyn IndexAttributeBuilder<T>>,
}

#[async_trait::async_trait]
pub trait IndexAttributeBuilder<T>: Send + Sync {
    async fn build_attributes(&self, document: &T) -> serde_json::Value;

    async fn build_chunk_attributes<'a>(
        &self,
        document: &'a T,
    ) -> BoxStream<'a, JoinHandle<Result<(Vec<String>, serde_json::Value)>>>;
}

impl<T: ToIndexId> TantivyDocBuilder<T> {
    pub async fn build(
        &self,
        document: T,
    ) -> (String, impl Stream<Item = JoinHandle<Option<TantivyDocument>>>) {
        let schema = IndexSchema::instance();
        let IndexId { source_id, id } = document.to_index_id();

        let now = tantivy::time::OffsetDateTime::now_utc();
        let updated_at = tantivy::DateTime::from_utc(now);

        let doc_attributes = self.builder.build_attributes(&document).await;

        let s = stream! {
            for await chunk_doc in self.build_chunks(id.clone(), source_id.clone(), updated_at, document).await {
                yield tokio::spawn(async move {
                    match chunk_doc.await {
                        Ok(Ok(doc)) => Some(doc),
                        _ => None,
                    }
                });
            }
        };

        (id, s)
    }
}
```

---

## 3. Structured Document Indexing

### Document Types

Tabby indexes multiple document types beyond code:

```rust
// From crates/tabby-index/src/structured_doc/public.rs

pub enum StructuredDoc {
    Web(WebDocument),
    Issue(IssueDocument),
    PullRequest(PullRequestDocument),
    Page(PageDocument),
    Commit(CommitDocument),
}

pub struct WebDocument {
    pub id: String,
    pub url: String,
    pub title: String,
    pub content: String,
    pub crawled_at: DateTime,
}

pub struct IssueDocument {
    pub id: String,
    pub repository: String,
    pub issue_number: u32,
    pub title: String,
    pub body: String,
    pub comments: Vec<String>,
    pub labels: Vec<String>,
    pub state: IssueState,
    pub created_at: DateTime,
    pub updated_at: DateTime,
}

pub struct PullRequestDocument {
    pub id: String,
    pub repository: String,
    pub pr_number: u32,
    pub title: String,
    pub body: String,
    pub diff: String,
    pub comments: Vec<String>,
    pub reviews: Vec<Review>,
    pub state: PullRequestState,
    pub merged: bool,
}

pub struct CommitDocument {
    pub id: String,
    pub repository: String,
    pub sha: String,
    pub message: String,
    pub diff: String,
    pub author: String,
    pub committed_at: DateTime,
}
```

### Indexing Issues

```rust
// From crates/tabby-index/src/structured_doc/types/issue.rs

pub struct StructuredDocIssueFields {
    pub repository: String,
    pub issue_number: u32,
    pub title: String,
    pub body: String,
    pub comments: Vec<String>,
    pub labels: Vec<String>,
}

impl ToIndexId for IssueDocument {
    fn to_index_id(&self) -> IndexId {
        IndexId {
            source_id: format!("git://{}", self.repository),
            id: format!("issue://{}", self.issue_number),
        }
    }
}

#[async_trait]
impl IndexAttributeBuilder<IssueDocument> for IssueAttributeBuilder {
    async fn build_attributes(&self, doc: &IssueDocument) -> serde_json::Value {
        json!({
            "kind": "issue",
            "repository": doc.repository,
            "issue_number": doc.issue_number,
            "title": doc.title,
            "labels": doc.labels,
            "state": doc.state.to_string(),
            "created_at": doc.created_at,
            "updated_at": doc.updated_at,
        })
    }

    async fn build_chunk_attributes(
        &self,
        doc: &IssueDocument,
    ) -> BoxStream<'_, JoinHandle<Result<(Vec<String>, serde_json::Value)>>> {
        stream! {
            // Index title
            yield tokio::spawn(async move {
                Ok((
                    tokenize(&doc.title),
                    json!({"section": "title"})
                ))
            });

            // Index body
            yield tokio::spawn(async move {
                Ok((
                    tokenize(&doc.body),
                    json!({"section": "body"})
                ))
            });

            // Index comments
            for (i, comment) in doc.comments.iter().enumerate() {
                yield tokio::spawn(async move {
                    Ok((
                        tokenize(comment),
                        json!({"section": "comment", "index": i})
                    ))
                });
            }
        }.boxed()
    }
}
```

### Indexing Pull Requests

```rust
// From crates/tabby-index/src/structured_doc/types/pull.rs

pub struct StructuredDocPullDocumentFields {
    pub repository: String,
    pub pr_number: u32,
    pub title: String,
    pub body: String,
    pub diff: String,
    pub reviews: Vec<Review>,
}

impl IndexAttributeBuilder<PullRequestDocument> for PullRequestAttributeBuilder {
    async fn build_chunk_attributes(
        &self,
        doc: &PullRequestDocument,
    ) -> BoxStream<'_, JoinHandle<Result<(Vec<String>, serde_json::Value)>>> {
        stream! {
            // Index title and body
            yield tokio::spawn(async move {
                Ok((
                    tokenize(&format!("{} {}", doc.title, doc.body)),
                    json!({"section": "description"})
                ))
            });

            // Index diff (code changes)
            yield tokio::spawn(async move {
                let changed_files = extract_changed_files(&doc.diff);
                let tokens = tokenize_code_changes(&changed_files);
                Ok((tokens, json!({"section": "diff"})))
            });

            // Index reviews
            for review in &doc.reviews {
                yield tokio::spawn(async move {
                    Ok((
                        tokenize(&review.body),
                        json!({"section": "review", "author": review.author})
                    ))
                });
            }
        }.boxed()
    }
}
```

---

## 4. RAG Implementation

### RAG Pipeline

```
┌─────────────────────────────────────────────────────────────────┐
│                    RAG Pipeline                                  │
│                                                                  │
│  User Query                                                      │
│     │                                                            │
│     ▼                                                            │
│  ┌──────────────┐                                               │
│  │ Query Analysis│ - Parse intent, extract keywords             │
│  └──────────────┘                                               │
│     │                                                            │
│     ▼                                                            │
│  ┌──────────────┐                                               │
│  │  Retrieval   │ - BM25 search                                │
│  │  (Tantivy)   │ - Semantic search (optional)                 │
│  └──────────────┘                                               │
│     │                                                            │
│     ▼                                                            │
│  ┌──────────────┐                                               │
│  │   Rerank     │ - Score by relevance                         │
│  └──────────────┘                                               │
│     │                                                            │
│     ▼                                                            │
│  ┌──────────────┐                                               │
│  │  Generation  │ - LLM generates answer with context          │
│  └──────────────┘                                               │
│     │                                                            │
│     ▼                                                            │
│  ┌──────────────┐                                               │
│  │  Citation    │ - Link back to source documents              │
│  └──────────────┘                                               │
└─────────────────────────────────────────────────────────────────┘
```

### Query Analysis

```rust
// From crates/tabby/src/api/answer.rs

pub struct QueryAnalysis {
    pub intent: QueryIntent,
    pub keywords: Vec<String>,
    pub filters: QueryFilters,
}

pub enum QueryIntent {
    CodeSearch,      // "find the fibonacci function"
    Documentation,   // "how to use the API"
    IssueSearch,     // "show me open bugs"
    General,         // "what is this project about"
}

pub struct QueryFilters {
    pub language: Option<String>,
    pub repository: Option<String>,
    pub doc_type: Option<String>,
}

fn analyze_query(query: &str) -> QueryAnalysis {
    // Simple heuristic analysis
    let intent = if query.contains("function") || query.contains("class") {
        QueryIntent::CodeSearch
    } else if query.contains("issue") || query.contains("bug") {
        QueryIntent::IssueSearch
    } else if query.contains("how") || query.contains("what") {
        QueryIntent::Documentation
    } else {
        QueryIntent::General
    };

    // Extract keywords
    let keywords = extract_keywords(query);

    // Extract filters
    let filters = QueryFilters {
        language: extract_language_filter(query),
        repository: extract_repository_filter(query),
        doc_type: extract_doctype_filter(query),
    };

    QueryAnalysis {
        intent,
        keywords,
        filters,
    }
}
```

### Retrieval

```rust
// From crates/tabby/src/api/answer.rs

pub async fn retrieve(
    index: &Indexer,
    query: &str,
    filters: &QueryFilters,
    limit: usize,
) -> Result<Vec<SearchResult>> {
    let schema = IndexSchema::instance();
    let searcher = &index.searcher;

    // Build query
    let mut boolean_query = BooleanQuery::builder();

    // Add term queries for keywords
    for keyword in extract_keywords(query) {
        let term_query = TermQuery::new(
            Term::from_field_text(schema.field_chunk_tokens, &keyword),
            IndexRecordOption::WithFreqs,
        );
        boolean_query.add(Occur::Should, Box::new(term_query));
    }

    // Add filters
    if let Some(language) = &filters.language {
        let language_query = TermQuery::new(
            Term::from_field_json_value(
                schema.field_attributes,
                json!({"language": language}),
            ),
            IndexRecordOption::Basic,
        );
        boolean_query.add(Occur::Must, Box::new(language_query));
    }

    if let Some(repo) = &filters.repository {
        let repo_query = TermQuery::new(
            Term::from_field_text(schema.field_source_id, repo),
            IndexRecordOption::Basic,
        );
        boolean_query.add(Occur::Must, Box::new(repo_query));
    }

    // Execute search
    let collector = TopDocs::with_limit(limit)
        .tweak_score(move |segment_reader| {
            // Custom scoring can go here
            move |_doc: DocId, original_score: Score| {
                original_score
            }
        });

    let query = boolean_query.build();
    let results = searcher.search(&query, &collector)?;

    // Fetch documents
    let mut docs = Vec::new();
    for (_score, doc_address) in results {
        let doc = searcher.doc::<TantivyDocument>(doc_address)?;
        docs.push(SearchResult::from_tantivy_doc(&doc));
    }

    Ok(docs)
}
```

### Generation with Context

```rust
// From crates/tabby/src/api/answer.rs

pub async fn generate_answer(
    query: &str,
    context: &[SearchResult],
    chat_model: &dyn ChatCompletionStream,
) -> Result<String> {
    // Build prompt with context
    let context_text = context
        .iter()
        .map(|r| r.content.as_str())
        .collect::<Vec<_>>()
        .join("\n\n---\n\n");

    let prompt = format!(
        r#"You are a helpful coding assistant. Answer the user's question using the provided context.

<context>
{context}
</context>

User question: {query}

Answer:""#,
        context = context_text,
        query = query
    );

    // Generate response
    let options = CompletionOptionsBuilder::default()
        .max_decoding_tokens(512)
        .sampling_temperature(0.1)
        .build()
        .unwrap();

    let response = chat_model.generate_sync(&prompt, options).await;

    Ok(response)
}
```

### Citation Generation

```rust
// From crates/tabby/src/api/answer.rs

pub struct Citation {
    pub source_id: String,
    pub filepath: Option<String>,
    pub line_range: Option<Range<usize>>,
    pub url: Option<String>,
}

impl SearchResult {
    pub fn to_citation(&self) -> Citation {
        Citation {
            source_id: self.source_id.clone(),
            filepath: self.filepath.clone(),
            line_range: self.line_range.clone(),
            url: self.generate_url(),
        }
    }

    fn generate_url(&self) -> Option<String> {
        if let Some(repo) = self.repository {
            if let Some(path) = &self.filepath {
                return Some(format!(
                    "https://github.com/{}/blob/main/{}",
                    repo, path
                ));
            }
        }
        None
    }
}
```

---

## 5. Index Optimization

### Garbage Collection

```rust
// From crates/tabby-index/src/indexer.rs

pub struct IndexGarbageCollector {
    searcher: Searcher,
    writer: IndexWriter,
}

impl IndexGarbageCollector {
    pub fn garbage_collect(&self, active_source_ids: &[String]) -> Result<()> {
        let source_ids: HashSet<_> = active_source_ids.iter().collect();

        // Count documents per source_id using aggregation
        let count_aggregation: Aggregation = serde_json::from_value(json!({
            "terms": {
                "field": FIELD_SOURCE_ID,
            }
        })).unwrap();

        let collector = AggregationCollector::from_aggs(
            vec![("count".to_owned(), count_aggregation)]
                .into_iter()
                .collect(),
            Default::default(),
        );

        let res = self.searcher.search(&AllQuery, &collector)?;

        // Find inactive source_ids
        if let Some(AggregationResult::BucketResult(BucketResult::Terms { buckets, .. })) =
            res.0.get("count")
        {
            for bucket in buckets {
                if let Key::Str(source_id) = &bucket.key {
                    if !source_ids.contains(source_id) {
                        debug!("Deleting {} documents for source_id: {}", bucket.doc_count, source_id);
                        self.delete_by_source_id(source_id);
                    }
                }
            }
        }

        Ok(())
    }

    fn delete_by_source_id(&self, source_id: &str) {
        let schema = IndexSchema::instance();
        let query = schema.source_id_query(source_id);
        let _ = self.writer.delete_query(Box::new(query));
    }
}
```

### Segment Merging

```rust
// Tantivy handles segment merging automatically, but you can tune it:

let index_writer = index.writer(150_000_000) // 150MB budget
    .expect("Failed to create writer");

// Merge policy: merge segments smaller than 10% of max size
let merge_policy = LogMergePolicy::default()
    .set_min_num_segments(8)
    .set_max_merge_size(10_000_000);

index_writer.set_merge_policy(Box::new(merge_policy));
```

### Memory Mapping

```rust
// Tantivy uses memory mapping for efficient access:

let index_reader = index.reader_builder()
    .reload_policy(ReloadPolicy::OnCommitWithDelay)
    .try_into()
    .expect("Failed to create reader");

// The searcher holds a snapshot of the index at a point in time
// New searchers see committed changes
```

### Caching

```rust
// From crates/tabby/src/api/search.rs

use moka::future::Cache;

pub struct SearchCache {
    cache: Cache<String, Vec<SearchResult>>,
}

impl SearchCache {
    pub fn new() -> Self {
        Self {
            cache: Cache::builder()
                .max_capacity(1000)
                .time_to_live(Duration::from_secs(3600))
                .build(),
        }
    }

    pub async fn get(&self, query: &str) -> Option<Vec<SearchResult>> {
        self.cache.get(query).await
    }

    pub async fn set(&self, query: &str, results: Vec<SearchResult>) {
        self.cache.insert(query.to_string(), results).await;
    }
}
```

---

## 6. Rust Implementation Guide

### Building a Code Indexer

```rust
// Example: Building a custom code indexer

use tabby_index::{
    indexer::{Indexer, IndexAttributeBuilder, ToIndexId, IndexId},
    code::{CodeIntelligence, ChunkBuilder},
};
use tantivy::doc;

pub struct CodeDocument {
    pub path: String,
    pub content: String,
    pub language: String,
    pub repository: String,
}

impl ToIndexId for CodeDocument {
    fn to_index_id(&self) -> IndexId {
        IndexId {
            source_id: format!("git://{}", self.repository),
            id: format!("file://{}", self.path),
        }
    }
}

pub struct CodeAttributeBuilder {
    intelligence: CodeIntelligence,
    chunk_builder: ChunkBuilder,
}

#[async_trait]
impl IndexAttributeBuilder<CodeDocument> for CodeAttributeBuilder {
    async fn build_attributes(&self, doc: &CodeDocument) -> serde_json::Value {
        let tree = self.intelligence.parse(&doc.content);
        let symbols = self.intelligence.extract_symbols(&tree, &doc.content);

        json!({
            "kind": "code",
            "language": doc.language,
            "path": doc.path,
            "symbols": symbols.iter()
                .map(|s| json!({
                    "name": s.name,
                    "type": s.symbol_type,
                }))
                .collect::<Vec<_>>(),
        })
    }

    async fn build_chunk_attributes(
        &self,
        doc: &CodeDocument,
    ) -> BoxStream<'_, JoinHandle<Result<(Vec<String>, serde_json::Value)>>> {
        let content = doc.content.clone();
        let tree = self.intelligence.parse(&content);
        let symbols = self.intelligence.extract_symbols(&tree, &content);
        let chunks = self.chunk_builder.build_chunks(&content, &symbols);

        futures::stream::iter(chunks.into_iter().map(|chunk| {
            tokio::spawn(async move {
                let tokens = tokenize(&chunk.content);
                let metadata = json!({
                    "chunk_type": "code",
                    "content": chunk.content,
                    "symbol": chunk.metadata.symbol_name,
                });
                Ok((tokens, metadata))
            })
        })).boxed()
    }
}

// Usage
async fn index_code() -> Result<()> {
    let indexer = Indexer::new("code");
    let builder = CodeAttributeBuilder {
        intelligence: CodeIntelligence::new("rust").unwrap(),
        chunk_builder: ChunkBuilder::new(1024, 64),
    };

    let doc = CodeDocument {
        path: "src/lib.rs".to_string(),
        content: std::fs::read_to_string("src/lib.rs")?,
        language: "rust".to_string(),
        repository: "my-repo".to_string(),
    };

    let (id, doc_stream) = TantivyDocBuilder::new("code", builder).build(doc).await;

    for await task in doc_stream {
        if let Ok(Some(doc)) = task.await {
            indexer.add(doc).await;
        }
    }

    indexer.commit();
    Ok(())
}
```

### Custom Search Queries

```rust
// Example: Building custom search queries

use tantivy::{
    query::{Query, BooleanQuery, TermQuery, RegexQuery},
    schema::IndexRecordOption,
    Term,
};

pub struct SearchQueryBuilder {
    corpus: String,
    source_id: Option<String>,
    language: Option<String>,
    keywords: Vec<String>,
}

impl SearchQueryBuilder {
    pub fn new(corpus: &str) -> Self {
        Self {
            corpus: corpus.to_string(),
            source_id: None,
            language: None,
            keywords: Vec::new(),
        }
    }

    pub fn source_id(mut self, source_id: String) -> Self {
        self.source_id = Some(source_id);
        self
    }

    pub fn language(mut self, language: String) -> Self {
        self.language = Some(language);
        self
    }

    pub fn keyword(mut self, keyword: String) -> Self {
        self.keywords.push(keyword);
        self
    }

    pub fn build(self) -> Box<dyn Query> {
        let schema = IndexSchema::instance();
        let mut boolean_query = BooleanQuery::builder();

        // Corpus filter (required)
        boolean_query.add(
            Occur::Must,
            Box::new(TermQuery::new(
                Term::from_field_text(schema.field_corpus, &self.corpus),
                IndexRecordOption::Basic,
            )),
        );

        // Source ID filter (optional)
        if let Some(source_id) = &self.source_id {
            boolean_query.add(
                Occur::Must,
                Box::new(TermQuery::new(
                    Term::from_field_text(schema.field_source_id, source_id),
                    IndexRecordOption::Basic,
                )),
            );
        }

        // Language filter (optional)
        if let Some(language) = &self.language {
            boolean_query.add(
                Occur::Must,
                Box::new(TermQuery::new(
                    Term::from_field_json_value(
                        schema.field_attributes,
                        serde_json::json!({"language": language}),
                    ),
                    IndexRecordOption::Basic,
                )),
            );
        }

        // Keyword search
        for keyword in &self.keywords {
            boolean_query.add(
                Occur::Should,
                Box::new(TermQuery::new(
                    Term::from_field_text(schema.field_chunk_tokens, keyword),
                    IndexRecordOption::WithFreqs,
                )),
            );
        }

        Box::new(boolean_query.build())
    }
}
```

---

## Conclusion

TabbyML's indexing and search system is built on:
- **Tantivy** for fast full-text search with BM25 scoring
- **Tree-sitter** for intelligent code parsing and symbol extraction
- **Chunking strategies** for efficient indexing
- **Structured documents** for multi-type indexing (code, issues, PRs)
- **RAG pipeline** for context-aware answers

The key insight is that **good search requires both smart indexing and smart retrieval** - TabbyML excels at both by understanding code structure and providing flexible query capabilities.
