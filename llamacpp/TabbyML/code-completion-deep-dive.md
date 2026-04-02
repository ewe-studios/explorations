---
source: /home/darkvoid/Boxxed/@formulas/src.rust/src.llamacpp/src.AICoders/src.TabbyML/tabby
explored_at: 2026-04-02
---

# TabbyML Code Completion Deep Dive

## Overview

This document explores TabbyML's code completion system in depth, covering:
- Fill-In-the-Middle (FIM) inference
- Context building and retrieval
- Post-processing filters
- Caching strategies
- Performance optimization

## Table of Contents

1. [FIM Inference Fundamentals](#1-fim-inference-fundamentals)
2. [Context Building Architecture](#2-context-building-architecture)
3. [Post-processing Pipeline](#3-post-processing-pipeline)
4. [Caching System](#4-caching-system)
5. [Performance Profiling](#5-performance-profiling)
6. [Rust Implementation Guide](#6-rust-implementation-guide)

---

## 1. FIM Inference Fundamentals

### What is Fill-In-the-Middle?

FIM is a training paradigm where models learn to complete code given both prefix and suffix context. Unlike standard completion (which only sees prefix), FIM enables:
- Better understanding of code structure
- More relevant completions
- Ability to complete mid-function code

### Prompt Template Format

Tabby uses a special token format:

```
<PRE>{prefix}<SUF>{suffix}<MID>{completion}
```

**Example:**

```python
# User's code (cursor at # CURSOR)
def fibonacci(n):
    if n <= 1:
        return n
    # CURSOR
    print(fibonacci(10))

# FIM Prompt sent to model:
<PRE>def fibonacci(n):
    if n <= 1:
        return n
<SUF>
    print(fibonacci(10))
<MID>
```

### Model Training

FIM models are trained with:
1. **Prefix-Suffix-Middle** data augmentation
2. **Random cursor position** simulation
3. **Masked language modeling** objective

### Implementation in Tabby

```rust
// From crates/tabby-common/src/config.rs

pub struct FIMTemplate {
    pub prefix: String,   // "<PRE>"
    pub suffix: String,   // "<SUF>"
    pub middle: String,   // "<MID>"
}

impl FIMTemplate {
    pub fn format(&self, prefix: &str, suffix: &str) -> String {
        format!(
            "{}{}{}{}{}",
            self.prefix, prefix, self.suffix, suffix, self.middle
        )
    }
}
```

### Stop Conditions

The model generates until hitting a stop condition:

```rust
// From crates/tabby-inference/src/decoding.rs

pub struct StopConditionFactory {
    stop_trie_cache: DashMap<String, Trie<u8>>,
    stop_words_from_model_config: Vec<String>,
}

impl StopConditionFactory {
    pub fn create(&self, text: &str, language: Option<&Language>) -> StopCondition {
        let mut stop_words = language.map(|l| l.get_stop_words()).unwrap_or_default();
        stop_words.extend(self.stop_words_from_model_config.clone());

        StopCondition::new(
            self.get_trie(stop_words),
            text
        )
    }
}

pub struct StopCondition<'a> {
    stop_trie: Option<Trie<u8>>,
    reversed_text: String,
    num_decoded: usize,
}

impl StopCondition {
    pub fn should_stop(&mut self, new_text: &str) -> (bool, usize) {
        self.num_decoded += 1;

        if !new_text.is_empty() {
            self.reversed_text = reverse(new_text) + &self.reversed_text;

            if let Some(trie) = &self.stop_trie {
                let matches = trie.common_prefix_search(&self.reversed_text);
                if let Some(max_length) = matches.into_iter().map(|m| m.len()).max() {
                    return (true, max_length);
                }
            }
        }

        (false, 0)
    }
}
```

### Language-Specific Stop Words

```rust
// From tabby-common/src/languages.rs

impl Language {
    pub fn get_stop_words(&self) -> Vec<String> {
        match self {
            Language::Python => vec![
                "\n\nclass ".to_owned(),
                "\n\ndef ".to_owned(),
                "\n\nif ".to_owned(),
                "\n\nprint".to_owned(),
            ],
            Language::Rust => vec![
                "\n\nfn ".to_owned(),
                "\n\nimpl ".to_owned(),
                "\n\nstruct ".to_owned(),
                "\n}".to_owned(),
            ],
            Language::TypeScript => vec![
                "\n\nfunction ".to_owned(),
                "\n\nconst ".to_owned(),
                "\n\nexport ".to_owned(),
                "\n}".to_owned(),
            ],
            _ => vec![],
        }
    }
}
```

---

## 2. Context Building Architecture

### Context Components

The completion context gathers multiple signals:

```typescript
// From clients/tabby-agent/src/codeCompletion/contexts.ts

interface CompletionContext {
    // Primary document
    filepath: string;
    language: string;
    content: string;
    position: number;

    // Split content
    prefix: string;    // Before cursor
    suffix: string;    // After cursor

    // Repository context
    gitRemote?: string;
    declarations?: DeclarationSnippet[];
    recentlyModified?: CodeSnippet[];
    visibleRanges?: VisibleRange[];

    // User context
    clipboard?: string;
}
```

### Building the Request

```typescript
// From clients/tabby-agent/src/codeCompletion/buildRequest.ts

function buildRequest(context: CompletionContext, config: Config) {
    const { prefix, suffix, language, filepath } = context;

    // Build FIM prompt
    const prompt = buildFIMPrompt(prefix, suffix, language);

    // Gather additional context
    const extraContext = {
        declarations: context.declarations?.slice(0, 5),
        recentlyModified: context.recentlyModified?.slice(0, 3),
        filepath,
        language,
    };

    return {
        model: config.model,
        prompt,
        max_tokens: config.maxTokens,
        temperature: config.temperature,
        n: config.numChoices,
        extra_body: {
            input_patches: extraContext,
        },
    };
}
```

### Declaration Snippets

Using LSP for type definitions:

```typescript
// From clients/tabby-agent/src/contextProviders/declarationSnippets.ts

export class DeclarationSnippetsProvider {
    async getSnippets(position: Position): Promise<DeclarationSnippet[]> {
        // Query LSP for symbol at position
        const symbol = await this.lsp.getSymbolAtPosition(position);

        // Find declaration
        const declaration = await this.lsp.getDeclaration(position);

        // Fetch declaration content
        const content = await this.getDocumentContent(declaration.uri);

        return [{
            symbol: symbol.name,
            content: content,
            range: declaration.range,
        }];
    }
}
```

### Recently Changed Code

Tracking user edits:

```typescript
// From clients/tabby-agent/src/contextProviders/recentlyChangedCodeSearch.ts

export class RecentlyChangedCodeSearch {
    private editHistory: EditEntry[] = [];

    recordEdit(document: TextDocument, range: Range, content: string) {
        this.editHistory.push({
            timestamp: Date.now(),
            filepath: document.uri.fsPath,
            range,
            content,
        });

        // Keep last 100 edits
        this.editHistory = this.editHistory.slice(-100);
    }

    async search(query: string, limit = 5): Promise<CodeSnippet[]> {
        // Find recent edits matching query
        const matches = this.editHistory
            .filter(entry => entry.content.includes(query))
            .sort((a, b) => b.timestamp - a.timestamp)
            .slice(0, limit);

        return matches.map(entry => ({
            filepath: entry.filepath,
            content: entry.content,
            range: entry.range,
        }));
    }
}
```

### Visible Range Tracking

Tracking what code is visible on screen:

```typescript
// From clients/tabby-agent/src/contextProviders/editorVisibleRanges.ts

export class EditorVisibleRangesTracker {
    private visibleRanges = new Map<string, Range[]>();

    onDidChangeTextEditorVisibleRanges(event: TextEditorVisibleRangesEvent) {
        this.visibleRanges.set(
            event.textEditor.document.uri.toString(),
            event.visibleRanges
        );
    }

    getVisibleContent(): string[] {
        const contents: string[] = [];

        for (const [uri, ranges] of this.visibleRanges) {
            for (const range of ranges) {
                const content = this.getDocumentContent(uri, range);
                contents.push(content);
            }
        }

        return contents;
    }
}
```

---

## 3. Post-processing Pipeline

### Pipeline Architecture

Completions go through multiple filters:

```typescript
// From clients/tabby-agent/src/codeCompletion/postprocess/index.ts

export async function postCacheProcess(
    items: CompletionResultItem[],
    context: CompletionContext,
    config: ConfigData["postprocess"],
): Promise<CompletionResultItem[]> {
    const pipeline = Promise.resolve({ items, context })
        .then(applyFilter(removeRepetitiveBlocks))
        .then(applyFilter(removeRepetitiveLines))
        .then(applyFilter(limitScope))
        .then(applyFilter(removeDuplicatedBlockClosingLine))
        .then(applyFilter(formatIndentation))
        .then(applyFilter(normalizeIndentation))
        .then(applyFilter(dropDuplicated))
        .then(applyFilter(trimSpace))
        .then(applyFilter(removeDuplicateSuffixLines))
        .then(applyFilter(dropMinimum));

    const result = await pipeline;
    return result.items;
}
```

### Filter 1: Remove Repetitive Blocks

Detects and removes repetitive code blocks:

```typescript
// From postprocess/removeRepetitiveBlocks.ts

export function removeRepetitiveBlocks(
    item: CompletionResultItem,
): CompletionResultItem {
    const { text } = item;
    const lines = text.split('\n');

    // Find repeated patterns
    for (let patternLength = 3; patternLength <= lines.length / 2; patternLength++) {
        for (let start = 0; start < lines.length - patternLength * 2; start++) {
            const pattern = lines.slice(start, start + patternLength);
            const nextPattern = lines.slice(start + patternLength, start + patternLength * 2);

            if (arraysEqual(pattern, nextPattern)) {
                // Found repetition, truncate
                return {
                    ...item,
                    text: lines.slice(0, start + patternLength).join('\n'),
                };
            }
        }
    }

    return item;
}
```

### Filter 2: Limit Scope

Ensures completion doesn't exceed current scope:

```typescript
// From postprocess/limitScope.ts

export function limitScope(
    item: CompletionResultItem,
    context: CompletionContext,
): CompletionResultItem {
    const { prefix, suffix } = context;

    // Count opening and closing braces
    let braceBalance = 0;
    let result = '';

    for (const char of item.text) {
        if (char === '{') braceBalance++;
        if (char === '}') braceBalance--;

        // Stop if we've closed more than we opened
        if (braceBalance < 0) break;

        result += char;
    }

    return { ...item, text: result };
}
```

### Filter 3: Format Indentation

Fixes indentation issues:

```typescript
// From postprocess/formatIndentation.ts

export function formatIndentation(
    item: CompletionResultItem,
    context: CompletionContext,
): CompletionResultItem {
    const { prefix } = context;

    // Get indentation of last line in prefix
    const lastLine = prefix.split('\n').pop() || '';
    const expectedIndent = lastLine.match(/^\s*/)?.[0] || '';

    // Fix indentation in completion
    const lines = item.text.split('\n');
    const fixedLines = lines.map((line, index) => {
        if (index === 0) return line; // First line stays as-is
        return expectedIndent + line.trimStart();
    });

    return { ...item, text: fixedLines.join('\n') };
}
```

### Filter 4: Drop Duplicated

Removes completions that duplicate existing code:

```typescript
// From postprocess/dropDuplicated.ts

export function dropDuplicated(
    item: CompletionResultItem,
    context: CompletionContext,
): CompletionResultItem {
    const { suffix } = context;

    // Check if completion start matches suffix
    const overlapLength = findOverlap(item.text, suffix);

    if (overlapLength > 0) {
        // Remove duplicated portion from suffix
        return {
            ...item,
            text: item.text + suffix.slice(overlapLength),
        };
    }

    return item;
}

function findOverlap(completion: string, suffix: string): number {
    for (let len = Math.min(completion.length, suffix.length); len > 0; len--) {
        if (completion.endsWith(suffix.slice(0, len))) {
            return len;
        }
    }
    return 0;
}
```

### Filter 5: Trim Space

Removes leading/trailing whitespace:

```typescript
// From postprocess/trimSpace.ts

export function trimSpace(
    item: CompletionResultItem,
): CompletionResultItem {
    // Only trim if it doesn't start with whitespace
    // (we want to preserve intentional indentation)
    if (!item.text.match(/^\s/)) {
        return { ...item, text: item.text.trimEnd() };
    }

    return item;
}
```

### Filter 6: Drop Minimum

Drops completions that are too short:

```typescript
// From postprocess/dropMinimum.ts

export function dropMinimum(
    item: CompletionResultItem,
): CompletionResultItem | null {
    const minLines = 1;
    const minChars = 3;

    const lines = item.text.split('\n');

    if (lines.length < minLines || item.text.length < minChars) {
        return null; // Drop this completion
    }

    return item;
}
```

---

## 4. Caching System

### Cache Key Generation

```typescript
// From clients/tabby-agent/src/codeCompletion/cache.ts

export function calculateCompletionContextHash(context: CompletionContext): string {
    const hash = crypto.createHash('sha256');

    hash.update(context.filepath);
    hash.update(context.language);
    hash.update(context.prefix.slice(-500)); // Last 500 chars
    hash.update(context.suffix.slice(0, 100)); // First 100 chars

    // Include relevant context
    if (context.declarations) {
        for (const decl of context.declarations) {
            hash.update(decl.symbol);
            hash.update(decl.content);
        }
    }

    return hash.digest('hex');
}
```

### Cache Structure

```typescript
// From clients/tabby-agent/src/codeCompletion/cache.ts

interface CacheEntry {
    items: CompletionResultItem[];
    timestamp: number;
    accessCount: number;
}

class CompletionCache {
    private cache = new Map<string, CacheEntry>();
    private maxSize = 1000;
    private ttl = 3600 * 1000; // 1 hour

    async get(context: CompletionContext): Promise<CompletionResultItem[] | null> {
        const key = calculateCompletionContextHash(context);
        const entry = this.cache.get(key);

        if (!entry) return null;
        if (Date.now() - entry.timestamp > this.ttl) {
            this.cache.delete(key);
            return null;
        }

        entry.accessCount++;
        return entry.items;
    }

    async set(context: CompletionContext, items: CompletionResultItem[]) {
        // Evict old entries if needed
        if (this.cache.size >= this.maxSize) {
            this.evictOldest();
        }

        const key = calculateCompletionContextHash(context);
        this.cache.set(key, {
            items,
            timestamp: Date.now(),
            accessCount: 0,
        });
    }

    private evictOldest() {
        let oldestKey: string | null = null;
        let oldestTime = Infinity;

        for (const [key, entry] of this.cache.entries()) {
            if (entry.timestamp < oldestTime) {
                oldestTime = entry.timestamp;
                oldestKey = key;
            }
        }

        if (oldestKey) {
            this.cache.delete(oldestKey);
        }
    }
}
```

### Forwarding Contexts

Generating alternative cache keys:

```typescript
// From clients/tabby-agent/src/codeCompletion/cache.ts

export function generateForwardingContexts(
    context: CompletionContext,
): CompletionContext[] {
    const contexts: CompletionContext[] = [];

    // Generate contexts with varying prefix lengths
    const prefixLengths = [100, 200, 500, 1000];

    for (const length of prefixLengths) {
        contexts.push({
            ...context,
            prefix: context.prefix.slice(-length),
        });
    }

    return contexts;
}
```

---

## 5. Performance Profiling

### Latency Tracking

```typescript
// From clients/tabby-agent/src/codeCompletion/latencyTracker.ts

class LatencyTracker {
    private latencies: number[] = [];
    private timeouts = 0;
    private totalRequests = 0;

    recordLatency(latency: number, timedOut: boolean) {
        this.latencies.push(latency);
        this.totalRequests++;

        if (timedOut) {
            this.timeouts++;
        }

        // Keep last 100 measurements
        if (this.latencies.length > 100) {
            this.latencies.shift();
        }
    }

    calculateLatencyStatistics(): LatencyStats {
        if (this.latencies.length === 0) {
            return { p50: 0, p75: 0, p95: 0, timeoutRate: 0 };
        }

        const sorted = [...this.latencies].sort((a, b) => a - b);

        return {
            p50: percentile(sorted, 50),
            p75: percentile(sorted, 75),
            p95: percentile(sorted, 95),
            timeoutRate: this.timeouts / this.totalRequests,
        };
    }
}
```

### Statistics Tracking

```typescript
// From clients/tabby-agent/src/codeCompletion/statistics.ts

interface CompletionStatistics {
    totalRequests: number;
    acceptedCompletions: number;
    rejectedCompletions: number;
    averageAcceptanceRate: number;
}

class CompletionStatisticsTracker {
    private stats: Map<string, CompletionStatisticsEntry> = new Map();
    private totalAccepted = 0;
    private totalRejected = 0;

    recordRequest(completionId: string) {
        this.stats.set(completionId, {
            id: completionId,
            timestamp: Date.now(),
            shown: true,
            accepted: false,
        });
    }

    recordAccept(completionId: string) {
        const entry = this.stats.get(completionId);
        if (entry) {
            entry.accepted = true;
            this.totalAccepted++;
        }
    }

    recordReject(completionId: string) {
        const entry = this.stats.get(completionId);
        if (entry) {
            entry.accepted = false;
            this.totalRejected++;
        }
    }

    getAcceptanceRate(): number {
        const total = this.totalAccepted + this.totalRejected;
        if (total === 0) return 0;
        return this.totalAccepted / total;
    }
}
```

### Debouncing

```typescript
// From clients/tabby-agent/src/codeCompletion/debouncer.ts

class CompletionDebouncer {
    private pendingTimers = new Map<string, NodeJS.Timeout>();
    private debounceMs = 100;

    async debounce(
        context: CompletionContext,
        trigger: () => Promise<void>,
    ): Promise<boolean> {
        const key = calculateCompletionContextHash(context);

        // Cancel existing timer
        const existingTimer = this.pendingTimers.get(key);
        if (existingTimer) {
            clearTimeout(existingTimer);
        }

        // Set new timer
        return new Promise(resolve => {
            const timer = setTimeout(() => {
                this.pendingTimers.delete(key);
                trigger();
                resolve(true);
            }, this.debounceMs);

            this.pendingTimers.set(key, timer);
        });
    }
}
```

---

## 6. Rust Implementation Guide

### Core Traits

```rust
// From crates/tabby-inference/src/lib.rs

use async_trait::async_trait;
use futures::stream::BoxStream;

#[async_trait]
pub trait CompletionStream: Sync + Send {
    /// Generate a completion in streaming mode
    async fn generate(
        &self,
        prompt: &str,
        options: CompletionOptions,
    ) -> BoxStream<'life0, String>;

    /// Generate a completion in non-streaming mode
    async fn generate_sync(&self, prompt: &str, options: CompletionOptions) -> String {
        let mut stream = self.generate(prompt, options).await;
        let mut result = String::new();
        while let Some(chunk) = stream.next().await {
            result.push_str(&chunk);
        }
        result
    }
}
```

### CodeGeneration Wrapper

```rust
// From crates/tabby-inference/src/code.rs

use derive_builder::Builder;

#[derive(Builder, Debug)]
pub struct CodeGenerationOptions {
    #[builder(default = "1024")]
    pub max_input_length: usize,

    #[builder(default = "256")]
    pub max_decoding_tokens: i32,

    #[builder(default = "0.1")]
    pub sampling_temperature: f32,

    #[builder(default = "crate::default_seed()")]
    pub seed: u64,

    #[builder(default = "None")]
    pub language: Option<&'static Language>,

    #[builder(default = "\"standard\".to_string()")]
    pub mode: String,
}

pub struct CodeGeneration {
    imp: Arc<dyn CompletionStream>,
    stop_condition_factory: StopConditionFactory,
}

impl CodeGeneration {
    pub fn new(imp: Arc<dyn CompletionStream>, config: Option<ModelConfig>) -> Self {
        let additional_stop_words = match config {
            Some(ModelConfig::Local(config)) => config.additional_stop_words.unwrap_or_default(),
            Some(ModelConfig::Http(config)) => config.additional_stop_words.unwrap_or_default(),
            _ => vec![],
        };
        let stop_condition_factory = StopConditionFactory::with_stop_words(additional_stop_words);

        Self {
            imp,
            stop_condition_factory,
        }
    }

    pub async fn generate(&self, prompt: &str, options: CodeGenerationOptions) -> String {
        let prompt = if options.max_input_length > 0 {
            clip_prompt(prompt, options.max_input_length)
        } else {
            prompt
        };

        let completion_options = CompletionOptionsBuilder::default()
            .max_decoding_tokens(options.max_decoding_tokens)
            .sampling_temperature(options.sampling_temperature)
            .seed(options.seed)
            .build()
            .unwrap();

        if options.mode == "next_edit_suggestion" {
            return self.imp.generate_sync(prompt, completion_options).await;
        }

        // Streaming with stop conditions
        let mut text = String::new();
        let mut stop_condition = self.stop_condition_factory.create(prompt, options.language);

        let mut stream = self.imp.generate(prompt, completion_options).await;
        while let Some(new_text) = stream.next().await {
            let (should_stop, stop_length) = stop_condition.should_stop(&new_text);
            text += &new_text;

            if should_stop {
                let new_text_length = text.len().checked_sub(stop_length).unwrap_or(0);
                text.truncate(new_text_length);
                break;
            }
        }

        text
    }
}
```

### llama.cpp Integration

```rust
// From crates/llama-cpp-server/src/supervisor.rs

pub struct LlamaCppSupervisor {
    name: &'static str,
    port: u16,
    handle: JoinHandle<()>,
}

impl LlamaCppSupervisor {
    pub fn new(
        name: &'static str,
        num_gpu_layers: u16,
        embedding: bool,
        model_path: &str,
        parallelism: u8,
        chat_template: Option<String>,
        enable_fast_attention: bool,
        context_size: usize,
    ) -> Self {
        let binary_name = find_binary_name()
            .expect("Failed to locate llama-server binary");

        let model_path = model_path.to_owned();
        let port = get_available_port();

        let handle = tokio::spawn(async move {
            loop {
                let mut command = tokio::process::Command::new(binary_name.clone());

                command
                    .arg("-m")
                    .arg(&model_path)
                    .arg("--cont-batching")
                    .arg("--port")
                    .arg(port.to_string())
                    .arg("-np")
                    .arg(parallelism.to_string())
                    .arg("--ctx-size")
                    .arg(context_size.to_string())
                    .kill_on_drop(true);

                if num_gpu_layers > 0 {
                    command.arg("-ngl").arg(num_gpu_layers.to_string());
                }

                if embedding {
                    command.arg("--embedding");
                }

                if let Some(template) = &chat_template {
                    command.arg("--chat-template").arg(template);
                }

                if enable_fast_attention {
                    command.arg("-fa");
                }

                let mut process = command.spawn().unwrap();
                let status = process.wait().await;

                // Restart on failure
                if status.code() != Some(0) {
                    tokio::time::sleep(Duration::from_secs(1)).await;
                }
            }
        });

        Self { name, handle, port }
    }

    pub async fn start(&self) {
        let client = reqwest::Client::new();

        loop {
            if let Ok(resp) = client
                .get(format!("http://127.0.0.1:{}/health", self.port))
                .timeout(Duration::from_secs(1))
                .send()
                .await
            {
                if resp.status().is_success() {
                    return;
                }
            }

            tokio::time::sleep(Duration::from_secs(1)).await;
        }
    }
}
```

---

## Conclusion

TabbyML's code completion system is a sophisticated pipeline combining:
- FIM-trained models for context-aware completions
- Multi-source context gathering (LSP, recent edits, visible ranges)
- Extensive post-processing for quality
- Multi-layer caching for performance
- Comprehensive telemetry for optimization

The key insight is that **good completions require more than just a good model** - they need careful context building, intelligent filtering, and performance optimization.
