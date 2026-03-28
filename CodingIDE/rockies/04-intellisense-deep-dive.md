---
title: "IntelliSense Deep Dive: Completions, Hover, and Navigation"
subtitle: "Building responsive code intelligence with symbol indexing and fast lookups"
based_on: "rockies neighbor tracking for O(1) reference resolution"
level: "Intermediate - Requires understanding of IDE fundamentals"
---

# IntelliSense Deep Dive

## Table of Contents

1. [IntelliSense Architecture](#1-intellisense-architecture)
2. [Completion Engines](#2-completion-engines)
3. [Hover Information](#3-hover-information)
4. [Go-to-Definition](#4-go-to-definition)
5. [Find All References](#5-find-all-references)
6. [Signature Help](#6-signature-help)
7. [Rockies Neighbor Tracking for References](#7-rockies-neighbor-tracking-for-references)
8. [Performance Optimization](#8-performance-optimization)

---

## 1. IntelliSense Architecture

### 1.1 What is IntelliSense?

IntelliSense is a collective term for code completion features:
- **Completions** - Suggest identifiers as you type
- **Hover** - Show type/info on mouse hover
- **Go-to-Definition** - Navigate to symbol definition
- **Find References** - Find all uses of a symbol
- **Signature Help** - Show function parameters

### 1.2 Architecture Overview

```
┌─────────────────────────────────────────────────────┐
│              IntelliSense Engine                    │
├─────────────────────────────────────────────────────┤
│  Request Router                                     │
│  - completion                                       │
│  - hover                                            │
│  - definition                                       │
│  - references                                       │
├─────────────────────────────────────────────────────┤
│  Symbol Index                                       │
│  - Fast symbol lookup by name                       │
│  - Location mapping                                 │
│  - Reference tracking                               │
├─────────────────────────────────────────────────────┤
│  Type Checker                                       │
│  - Type inference at position                       │
│  - Type lookup                                      │
├─────────────────────────────────────────────────────┤
│  Language Parser                                    │
│  - AST generation                                   │
│  - Incremental parsing                              │
└─────────────────────────────────────────────────────┘
```

### 1.3 Rockies Parallels

| Rockies | IntelliSense |
|---------|-------------|
| `Grid::get(x, y)` | `hover(line, column)` |
| `Grid::put(x, y, cell)` | `index_symbol(symbol)` |
| `neighbors` tracking | Reference tracking |
| `get_missing_grids()` | Lazy type loading |
| Grid serialization | Index persistence |

---

## 2. Completion Engines

### 2.1 Completion Architecture

```rust
use std::collections::{HashMap, BTreeMap};

/// Completion item for IntelliSense
#[derive(Debug, Clone)]
pub struct CompletionItem {
    pub label: String,
    pub kind: CompletionItemKind,
    pub detail: Option<String>,
    pub documentation: Option<String>,
    pub sort_text: Option<String>,
    pub filter_text: Option<String>,
    pub insert_text: Option<String>,
    pub text_edit: Option<TextEdit>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompletionItemKind {
    Text,
    Method,
    Function,
    Constructor,
    Field,
    Variable,
    Class,
    Interface,
    Module,
    Property,
    Unit,
    Value,
    Enum,
    Keyword,
    Snippet,
    Color,
    File,
    Reference,
    Folder,
    EnumMember,
    Constant,
    Struct,
    Event,
    Operator,
    TypeParameter,
}

#[derive(Debug, Clone)]
pub struct TextEdit {
    pub range: Range,
    pub new_text: String,
}

#[derive(Debug, Clone, Copy)]
pub struct Range {
    pub start: Position,
    pub end: Position,
}

#[derive(Debug, Clone, Copy)]
pub struct Position {
    pub line: usize,
    pub column: usize,
}
```

### 2.2 Completion Provider

```rust
pub struct CompletionEngine {
    /// Global symbol index
    index: Arc<SymbolIndex>,

    /// Language-specific providers
    providers: HashMap<String, Box<dyn CompletionProvider>>,

    /// Keyword completions by language
    keywords: HashMap<String, Vec<CompletionItem>>,

    /// Snippet completions
    snippets: Vec<CompletionItem>,
}

trait CompletionProvider: Send + Sync {
    fn provide_completions(
        &self,
        file: &Path,
        position: Position,
        content: &str,
    ) -> Vec<CompletionItem>;
}

impl CompletionEngine {
    pub fn new(index: Arc<SymbolIndex>) -> Self {
        let mut keywords = HashMap::new();

        // Rust keywords
        keywords.insert("rust".to_string(), vec![
            CompletionItem { label: "fn".into(), kind: CompletionItemKind::Keyword, ..Default::default() },
            CompletionItem { label: "let".into(), kind: CompletionItemKind::Keyword, ..Default::default() },
            CompletionItem { label: "mut".into(), kind: CompletionItemKind::Keyword, ..Default::default() },
            CompletionItem { label: "if".into(), kind: CompletionItemKind::Keyword, ..Default::default() },
            CompletionItem { label: "else".into(), kind: CompletionItemKind::Keyword, ..Default::default() },
            CompletionItem { label: "match".into(), kind: CompletionItemKind::Keyword, ..Default::default() },
            CompletionItem { label: "for".into(), kind: CompletionItemKind::Keyword, ..Default::default() },
            CompletionItem { label: "while".into(), kind: CompletionItemKind::Keyword, ..Default::default() },
            CompletionItem { label: "struct".into(), kind: CompletionItemKind::Keyword, ..Default::default() },
            CompletionItem { label: "impl".into(), kind: CompletionItemKind::Keyword, ..Default::default() },
            CompletionItem { label: "trait".into(), kind: CompletionItemKind::Keyword, ..Default::default() },
            CompletionItem { label: "enum".into(), kind: CompletionItemKind::Keyword, ..Default::default() },
            CompletionItem { label: "pub".into(), kind: CompletionItemKind::Keyword, ..Default::default() },
            CompletionItem { label: "use".into(), kind: CompletionItemKind::Keyword, ..Default::default() },
            CompletionItem { label: "mod".into(), kind: CompletionItemKind::Keyword, ..Default::default() },
        ]);

        // Snippets
        let snippets = vec![
            CompletionItem {
                label: "main".into(),
                kind: CompletionItemKind::Snippet,
                detail: Some("fn main() { ... }".into()),
                insert_text: Some("fn main() {\n    $0\n}".into()),
                ..Default::default()
            },
            CompletionItem {
                label: "if".into(),
                kind: CompletionItemKind::Snippet,
                detail: Some("if condition { ... }".into()),
                insert_text: Some("if $1 {\n    $0\n}".into()),
                ..Default::default()
            },
            CompletionItem {
                label: "fn".into(),
                kind: CompletionItemKind::Snippet,
                detail: Some("fn name() { ... }".into()),
                insert_text: Some("fn ${1:name}($2) -> ${3:Type} {\n    $0\n}".into()),
                ..Default::default()
            },
        ];

        Self {
            index,
            providers: HashMap::new(),
            keywords,
            snippets,
        }
    }

    /// Get completions at a position
    pub fn completions(
        &self,
        file: &Path,
        position: Position,
        content: &str,
    ) -> Vec<CompletionItem> {
        let mut results = Vec::new();

        // Get prefix being typed
        let prefix = self.get_prefix(content, position);

        // 1. Language keywords
        if let Some(lang) = self.detect_language(file) {
            if let Some(keywords) = self.keywords.get(lang) {
                for kw in keywords {
                    if kw.label.starts_with(&prefix) {
                        results.push(kw.clone());
                    }
                }
            }
        }

        // 2. Snippets
        for snippet in &self.snippets {
            if snippet.label.starts_with(&prefix) {
                results.push(snippet.clone());
            }
        }

        // 3. Symbols from index
        let symbol_completions = self.index.find_symbols_by_prefix(&prefix);
        for symbol in symbol_completions {
            results.push(CompletionItem {
                label: symbol.name.clone(),
                kind: symbol.kind.to_completion_kind(),
                detail: Some(symbol.type_str()),
                documentation: symbol.documentation.clone(),
                ..Default::default()
            });
        }

        // 4. Language-specific provider
        if let Some(lang) = self.detect_language(file) {
            if let Some(provider) = self.providers.get(lang) {
                let provider_completions = provider.provide_completions(file, position, content);
                results.extend(provider_completions);
            }
        }

        // 5. Local variables (from current scope)
        let locals = self.extract_locals(content, position);
        for local in locals {
            if local.starts_with(&prefix) {
                results.push(CompletionItem {
                    label: local.clone(),
                    kind: CompletionItemKind::Variable,
                    ..Default::default()
                });
            }
        }

        // Sort and deduplicate
        results.sort_by(|a, b| {
            a.sort_text.as_ref().unwrap_or(&a.label)
                .cmp(b.sort_text.as_ref().unwrap_or(&b.label))
        });
        results.dedup_by(|a, b| a.label == b.label);

        results
    }

    fn get_prefix(&self, content: &str, position: Position) -> String {
        let lines: Vec<&str> = content.lines().collect();
        if position.line >= lines.len() {
            return String::new();
        }

        let line = lines[position.line];
        let start = line[..position.column.min(line.len())]
            .rfind(|c: char| !c.is_alphanumeric() && c != '_')
            .map(|i| i + 1)
            .unwrap_or(0);

        line[start..position.column.min(line.len())].to_string()
    }

    fn detect_language(&self, file: &Path) -> Option<&str> {
        file.extension()
            .and_then(|ext| ext.to_str())
            .map(|ext| match ext {
                "rs" => "rust",
                "ts" | "tsx" => "typescript",
                "js" | "jsx" => "javascript",
                "py" => "python",
                _ => "unknown",
            })
    }

    fn extract_locals(&self, content: &str, position: Position) -> Vec<String> {
        // Simple local variable extraction
        // In a real implementation, use the AST
        let mut locals = Vec::new();

        for line in content.lines().take(position.line + 1) {
            // Match "let x = " or "var x = " or "const x = "
            if let Some(captures) = regex::Regex::new(r"(?:let|var|const)\s+(\w+)\s*=").ok() {
                for cap in captures.captures_iter(line) {
                    if let Some(name) = cap.get(1) {
                        locals.push(name.as_str().to_string());
                    }
                }
            }
        }

        locals
    }
}

impl Default for CompletionItem {
    fn default() -> Self {
        Self {
            label: String::new(),
            kind: CompletionItemKind::Text,
            detail: None,
            documentation: None,
            sort_text: None,
            filter_text: None,
            insert_text: None,
            text_edit: None,
        }
    }
}
```

---

## 3. Hover Information

### 3.1 Hover Structure

```rust
#[derive(Debug, Clone)]
pub struct HoverResult {
    pub contents: Vec<HoverContent>,
    pub range: Option<Range>,
}

#[derive(Debug, Clone)]
pub enum HoverContent {
    Markdown(String),
    CodeBlock { language: String, code: String },
    Text(String),
}

impl HoverResult {
    pub fn type_info(type_str: &str) -> Self {
        Self {
            contents: vec![HoverContent::CodeBlock {
                language: "rust".to_string(),
                code: type_str.to_string(),
            }],
            range: None,
        }
    }

    pub fn with_docs(mut self, docs: &str) -> Self {
        self.contents.push(HoverContent::Markdown(docs.to_string()));
        self
    }
}
```

### 3.2 Hover Provider

```rust
pub struct HoverProvider {
    index: Arc<SymbolIndex>,
}

impl HoverProvider {
    pub fn new(index: Arc<SymbolIndex>) -> Self {
        Self { index }
    }

    /// Get hover information at a position
    pub fn hover(&self, file: &Path, position: Position, content: &str) -> Option<HoverResult> {
        // Get the identifier at the position
        let identifier = self.get_identifier_at(content, position)?;

        // Look up in symbol index
        if let Some(symbol) = self.index.find_symbol_by_name(&identifier) {
            return Some(HoverResult {
                contents: vec![
                    HoverContent::CodeBlock {
                        language: "rust".to_string(),
                        code: symbol.signature(),
                    },
                    HoverContent::Markdown(symbol.documentation.clone().unwrap_or_default()),
                ],
                range: Some(Range {
                    start: Position { line: symbol.location.line, column: symbol.location.column },
                    end: Position { line: symbol.location.line, column: symbol.location.column + identifier.len() },
                }),
            });
        }

        // Fallback: try type inference
        if let Some(type_str) = self.infer_type_at(content, position) {
            return Some(HoverResult::type_info(&type_str));
        }

        None
    }

    fn get_identifier_at(&self, content: &str, position: Position) -> Option<String> {
        let lines: Vec<&str> = content.lines().collect();
        if position.line >= lines.len() {
            return None;
        }

        let line = lines[position.line];
        let chars: Vec<char> = line.chars().collect();

        if position.column >= chars.len() {
            return None;
        }

        // Check if we're on an alphanumeric or underscore
        if !chars[position.column].is_alphanumeric() && chars[position.column] != '_' {
            return None;
        }

        // Find start and end of identifier
        let start = (0..=position.column)
            .rev()
            .find(|&i| !chars[i].is_alphanumeric() && chars[i] != '_')
            .map(|i| i + 1)
            .unwrap_or(0);

        let end = (position.column..chars.len())
            .find(|&i| !chars[i].is_alphanumeric() && chars[i] != '_')
            .unwrap_or(chars.len());

        Some(chars[start..end].iter().collect())
    }

    fn infer_type_at(&self, content: &str, position: Position) -> Option<String> {
        // Simple type inference
        // In a real implementation, use a full type checker

        let lines: Vec<&str> = content.lines().collect();
        if position.line >= lines.len() {
            return None;
        }

        let line = lines[position.line];

        // Check for common patterns
        if let Some(idx) = line.find("let ") {
            if position.column > idx + 4 {
                // Try to extract type from annotation
                if let Some(type_start) = line[idx..].find(':') {
                    if let Some(type_end) = line[idx + type_start..].find('=') {
                        let type_str = line[idx + type_start + 1..idx + type_start + type_end]
                            .trim();
                        return Some(type_str.to_string());
                    }
                }
            }
        }

        None
    }
}
```

---

## 4. Go-to-Definition

### 4.1 Definition Provider

```rust
pub struct DefinitionProvider {
    index: Arc<SymbolIndex>,
}

impl DefinitionProvider {
    pub fn new(index: Arc<SymbolIndex>) -> Self {
        Self { index }
    }

    /// Find definition of symbol at position
    pub fn definition(&self, file: &Path, position: Position, content: &str) -> Vec<Location> {
        // Get identifier at position
        let identifier = match self.get_identifier_at(content, position) {
            Some(id) => id,
            None => return Vec::new(),
        };

        // Look up in symbol index
        let locations = self.index.find_definition(&identifier);

        locations
    }

    fn get_identifier_at(&self, content: &str, position: Position) -> Option<String> {
        // Same as hover provider
        None // Implementation omitted for brevity
    }
}

#[derive(Debug, Clone)]
pub struct Location {
    pub uri: String,
    pub range: Range,
}
```

---

## 5. Find All References

### 5.1 Reference Provider

```rust
pub struct ReferenceProvider {
    index: Arc<SymbolIndex>,
}

impl ReferenceProvider {
    pub fn new(index: Arc<SymbolIndex>) -> Self {
        Self { index }
    }

    /// Find all references to symbol at position
    pub fn references(
        &self,
        file: &Path,
        position: Position,
        content: &str,
        include_declaration: bool,
    ) -> Vec<Location> {
        // Get identifier at position
        let identifier = match self.get_identifier_at(content, position) {
            Some(id) => id,
            None => return Vec::new(),
        };

        // Find symbol and its references
        let mut locations = Vec::new();

        if include_declaration {
            if let Some(definition) = self.index.find_definition(&identifier) {
                locations.extend(definition);
            }
        }

        // Get all references
        locations.extend(self.index.find_references(&identifier));

        locations
    }
}
```

### 5.2 Reference Index Structure

```rust
/// Reference tracking in symbol index
impl SymbolIndex {
    /// Find all references to a symbol
    pub fn find_references(&self, symbol_name: &str) -> Vec<Location> {
        let mut locations = Vec::new();

        // Look up symbol
        if let Some(symbol_ids) = self.symbols_by_name.get(symbol_name) {
            for symbol_id in symbol_ids {
                if let Some(references) = self.references.get(symbol_id) {
                    for reference in references {
                        locations.push(Location {
                            uri: format!("file://{}", reference.path.display()),
                            range: Range {
                                start: Position { line: reference.line, column: reference.column },
                                end: Position { line: reference.line, column: reference.column + symbol_name.len() },
                            },
                        });
                    }
                }
            }
        }

        locations
    }
}
```

---

## 6. Signature Help

### 6.1 Signature Help Structure

```rust
#[derive(Debug, Clone)]
pub struct SignatureHelp {
    pub signatures: Vec<SignatureInformation>,
    pub active_signature: usize,
    pub active_parameter: usize,
}

#[derive(Debug, Clone)]
pub struct SignatureInformation {
    pub label: String,
    pub documentation: Option<String>,
    pub parameters: Vec<ParameterInformation>,
}

#[derive(Debug, Clone)]
pub struct ParameterInformation {
    pub label: ParameterLabel,
    pub documentation: Option<String>,
}

#[derive(Debug, Clone)]
pub enum ParameterLabel {
    Simple(String),
    LabelOffsets([u32; 2]),
}

impl SignatureHelp {
    pub fn new(label: &str) -> Self {
        Self {
            signatures: vec![SignatureInformation {
                label: label.to_string(),
                documentation: None,
                parameters: Vec::new(),
            }],
            active_signature: 0,
            active_parameter: 0,
        }
    }
}
```

### 6.2 Signature Help Provider

```rust
pub struct SignatureHelpProvider {
    index: Arc<SymbolIndex>,
}

impl SignatureHelpProvider {
    pub fn new(index: Arc<SymbolIndex>) -> Self {
        Self { index }
    }

    /// Get signature help at a position
    pub fn signature_help(&self, file: &Path, position: Position, content: &str) -> Option<SignatureHelp> {
        // Find function call at position
        let function_name = self.get_function_call(content, position)?;

        // Look up function signature
        if let Some(symbol) = self.index.find_symbol_by_name(&function_name) {
            if symbol.kind == SymbolKind::Function || symbol.kind == SymbolKind::Method {
                return Some(self.build_signature_help(&symbol));
            }
        }

        None
    }

    fn get_function_call(&self, content: &str, position: Position) -> Option<String> {
        // Look backwards from position to find function name before '('
        let lines: Vec<&str> = content.lines().collect();
        if position.line >= lines.len() {
            return None;
        }

        let line = &lines[position.line][..position.column.min(lines[position.line].len())];

        // Find the opening parenthesis
        if let Some(paren_pos) = line.rfind('(') {
            let before_paren = &line[..paren_pos];
            // Get the identifier before the parenthesis
            if let Some(name_start) = before_paren.rfind(|c: char| !c.is_alphanumeric() && c != '_') {
                return Some(before_paren[name_start + 1..].to_string());
            } else {
                return Some(before_paren.to_string());
            }
        }

        None
    }

    fn build_signature_help(&self, symbol: &Symbol) -> SignatureHelp {
        let mut help = SignatureHelp::new(&symbol.name);

        // Parse parameters from signature
        if let Some(params) = self.extract_parameters(&symbol.signature) {
            for param in params {
                help.signatures[0].parameters.push(ParameterInformation {
                    label: ParameterLabel::Simple(param),
                    documentation: None,
                });
            }
        }

        help
    }

    fn extract_parameters(&self, signature: &str) -> Option<Vec<String>> {
        // Simple parameter extraction from signature
        // e.g., "fn foo(x: i32, y: String)" -> ["x: i32", "y: String"]
        let start = signature.find('(')? + 1;
        let end = signature.find(')')?;
        let params_str = &signature[start..end];

        Some(params_str.split(',').map(|s| s.trim().to_string()).collect())
    }
}
```

---

## 7. Rockies Neighbor Tracking for References

### 7.1 Rockies Grid Neighbor Pre-computation

```rust
// Rockies: Grid pre-calculates neighbors
pub struct GridCell<T> {
    value: Vec<Rc<RefCell<T>>>,      // Items at this position
    neighbors: Vec<Rc<RefCell<T>>>,  // Items in adjacent cells
}

impl Grid<T> {
    pub fn put(&mut self, x: usize, y: usize, value: Rc<RefCell<T>>) {
        // Add to current cell
        self.grid[index(x, y)].value.push(value.clone());

        // Add to neighbors of surrounding cells
        for px in 0..3 {
            for py in 0..3 {
                self.grid[index(x + px, y + py)]
                    .add_neighbor(self.version, value.clone());
            }
        }
    }

    // O(1) neighbor lookup
    pub fn get(&self, x: usize, y: usize) -> GetResult<T> {
        GetResult {
            value: &self.grid[index(x, y)].value,
            neighbors: &self.grid[index(x, y)].neighbors,
        }
    }
}
```

### 7.2 Reference Tracking with Same Pattern

```rust
/// Symbol with pre-computed references (like neighbors)
pub struct SymbolEntry {
    pub symbol: Symbol,
    /// Pre-computed references (like neighbors in Rockies)
    pub references: Vec<Reference>,
    /// Incoming references (who calls this)
    pub incoming: Vec<CallSite>,
    /// Outgoing references (what this calls)
    pub outgoing: Vec<CallSite>,
}

pub struct ReferenceIndex {
    symbols: HashMap<SymbolId, SymbolEntry>,
    /// For fast lookup by name
    by_name: HashMap<String, SymbolId>,
}

impl ReferenceIndex {
    pub fn new() -> Self {
        Self {
            symbols: HashMap::new(),
            by_name: HashMap::new(),
        }
    }

    /// Add symbol and pre-compute references
    pub fn add_symbol(&mut self, symbol: Symbol, references: Vec<Reference>) {
        let symbol_id = symbol.id;

        let entry = SymbolEntry {
            symbol: symbol.clone(),
            references,
            incoming: Vec::new(),
            outgoing: Vec::new(),
        };

        self.symbols.insert(symbol_id, entry);
        self.by_name.insert(symbol.name.clone(), symbol_id);
    }

    /// O(1) reference lookup (like Grid::get)
    pub fn get_references(&self, symbol_id: SymbolId) -> Option<&Vec<Reference>> {
        self.symbols.get(&symbol_id).map(|e| &e.references)
    }

    /// O(1) incoming lookup
    pub fn get_incoming(&self, symbol_id: SymbolId) -> Option<&Vec<CallSite>> {
        self.symbols.get(&symbol_id).map(|e| &e.incoming)
    }

    /// O(1) outgoing lookup
    pub fn get_outgoing(&self, symbol_id: SymbolId) -> Option<&Vec<CallSite>> {
        self.symbols.get(&symbol_id).map(|e| &e.outgoing)
    }

    /// Fast "find all references" (like neighbor lookup)
    pub fn find_all_references(&self, symbol_name: &str) -> Vec<Location> {
        if let Some(symbol_id) = self.by_name.get(symbol_name) {
            if let Some(entry) = self.symbols.get(symbol_id) {
                return entry.references
                    .iter()
                    .map(|r| r.location.clone())
                    .collect();
            }
        }
        Vec::new()
    }
}
```

### 7.3 Incremental Reference Updates

```rust
impl ReferenceIndex {
    /// Update references when a file changes
    pub fn update_file(&mut self, file: &Path, old_symbols: &[SymbolId], new_content: &str) {
        // Remove old references from removed symbols
        for symbol_id in old_symbols {
            if let Some(entry) = self.symbols.get(symbol_id) {
                // Remove this symbol's references from other symbols' incoming
                for reference in &entry.references {
                    if let Some(target_id) = self.find_symbol_at(&reference.location) {
                        if let Some(target) = self.symbols.get_mut(&target_id) {
                            target.incoming.retain(|c| c.symbol_id != *symbol_id);
                        }
                    }
                }
            }
        }

        // Re-analyze file and update references
        // ... (parsing and analysis code)
    }
}
```

---

## 8. Performance Optimization

### 8.1 Caching Strategies

```rust
use lru::LruCache;
use std::num::NonZeroUsize;

/// Cached completion results
pub struct CompletionCache {
    /// Cache completions by (file, line, prefix)
    cache: LruCache<(String, usize, String), Vec<CompletionItem>>,
}

impl CompletionCache {
    pub fn new(max_entries: usize) -> Self {
        Self {
            cache: LruCache::new(NonZeroUsize::new(max_entries).unwrap()),
        }
    }

    pub fn get(&mut self, file: &str, line: usize, prefix: &str) -> Option<&Vec<CompletionItem>> {
        self.cache.get(&(file.to_string(), line, prefix.to_string()))
    }

    pub fn insert(&mut self, file: &str, line: usize, prefix: &str, items: Vec<CompletionItem>) {
        self.cache.insert((file.to_string(), line, prefix.to_string()), items);
    }

    pub fn invalidate(&mut self, file: &str) {
        // Remove all entries for this file
        self.cache.iter().filter(|((f, _, _), _)| f == file).count();
    }
}
```

### 8.2 Debouncing and Throttling

```rust
use std::time::{Duration, Instant};

/// Debounced completion trigger
pub struct Debouncer {
    last_trigger: Instant,
    delay: Duration,
}

impl Debouncer {
    pub fn new(delay_ms: u64) -> Self {
        Self {
            last_trigger: Instant::now() - delay,  // Allow immediate first trigger
            delay: Duration::from_millis(delay_ms),
        }
    }

    pub fn should_trigger(&mut self) -> bool {
        let now = Instant::now();
        if now.duration_since(self.last_trigger) >= self.delay {
            self.last_trigger = now;
            true
        } else {
            false
        }
    }
}

// Usage: 200ms debounce for completions
let mut debouncer = Debouncer::new(200);

if debouncer.should_trigger() {
    // Request completions
}
```

### 8.3 Parallel Indexing

```rust
use rayon::prelude::*;

impl SymbolIndex {
    /// Build index in parallel
    pub fn build_parallel(files: Vec<PathBuf>) -> Self {
        let mut index = Self::new();

        // Process files in parallel
        let results: Vec<(PathBuf, FileIndex)> = files
            .par_iter()
            .filter_map(|path| {
                if let Ok(content) = std::fs::read_to_string(path) {
                    let file_index = Self::analyze_file(path, &content);
                    Some((path.clone(), file_index))
                } else {
                    None
                }
            })
            .collect();

        // Merge results
        for (path, file_index) in results {
            index.add_file_index(path, file_index);
        }

        index
    }
}
```

---

*Next: [rust-revision.md](rust-revision.md)*
