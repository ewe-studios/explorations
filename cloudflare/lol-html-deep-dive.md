---
source: /home/darkvoid/Boxxed/@formulas/src.rust/src.cloudflare/Others/lol-html
repository: https://github.com/cloudflare/lol-html
revised_at: 2026-03-19
---

# lol-html Deep Dive: Streaming HTML Rewriter

## Overview

lol-html (**L**ow **O**utput **L**atency HTML) is a streaming HTML rewriter/parser with CSS selector-based API. It can process arbitrarily large HTML documents with constant memory usage, making it ideal for edge computing scenarios.

## Key Characteristics

| Feature | Description |
|---------|-------------|
| Streaming | Processes HTML in chunks, no full document in memory |
| Constant Memory | Memory-bounded regardless of document size |
| CSS Selectors | Familiar CSS selector syntax for element matching |
| Encoding Support | Handles all WHATWG-compatible encodings via `encoding_rs` |
| Zero-Copy | Minimizes allocations where possible |

## Architecture

### Crate Structure

```
lol-html/
├── src/
│   ├── lib.rs                    # Public API, HtmlRewriter, rewrite_str
│   ├── parser/
│   │   ├── mod.rs                # Parser module exports
│   │   ├── state_machine/
│   │   │   └── mod.rs            # 9-state HTML parsing state machine
│   │   ├── lexer/
│   │   │   ├── mod.rs            # Token lexer
│   │   │   └── lexeme/
│   │   │       └── mod.rs        # Lexeme representation
│   │   └── tree_builder_simulator/
│   │       └── mod.rs            # Simulated DOM tree builder
│   ├── rewriter.rs               # Core rewriting logic
│   ├── html.rs                   # HTML element representations
│   ├── selectors_vm.rs           # CSS selector matching VM
│   ├── transform_stream.rs       # Streaming transformation
│   ├── rewritable_units/         # Mutable HTML units
│   │   ├── element.rs            # Element manipulation
│   │   ├── attribute.rs          # Attribute manipulation
│   │   ├── text_chunk.rs         # Text content manipulation
│   │   ├── comment.rs            # Comment manipulation
│   │   ├── start_tag.rs          # Start tag manipulation
│   │   ├── end_tag.rs            # End tag manipulation
│   │   └── doctype.rs            # DOCTYPE manipulation
│   └── memory.rs                 # Memory limiting
├── Cargo.toml
└── README.md
```

### Key Dependencies

```toml
[dependencies]
cssparser = "0.31"       # CSS selector parsing
encoding_rs = "0.8"      # Character encoding handling
cfg-if = "1"             # Conditional compilation
```

## Core API

### HtmlRewriter: Streaming API

```rust
use lol_html::{HtmlRewriter, Settings, ElementContentHandlers, element};

let mut output = Vec::new();

let mut rewriter = HtmlRewriter::new(
    Settings {
        element_content_handlers: vec![
            // CSS selector + handler
            ("script", ElementContentHandlers::new()
                .element(|el| {
                    // Remove all script tags
                    el.remove();
                    Ok(())
                })
            ),
            ("img", ElementContentHandlers::new()
                .element(|el| {
                    // Add loading=lazy to all images
                    el.set_attribute("loading", "lazy")?;
                    Ok(())
                })
            ),
        ],
        ..Settings::default()
    },
    |c: &[u8]| output.extend_from_slice(c),  // Output sink
);

// Feed HTML chunks
rewriter.write(b"<html><head><script>alert('x')</script></head>")?;
rewriter.write(b"<body><img src='img.jpg'><p>Content</p></body></html>")?;

// Finalize
rewriter.end()?;
```

### rewrite_str: One-off Rewriting

```rust
use lol_html::{rewrite_str, Settings, ElementContentHandlers, element};

let html = r#"<div class="container"><p>Hello</p></div>"#;

let result = rewrite_str(
    html,
    Settings {
        element_content_handlers: vec![
            ("p", ElementContentHandlers::new()
                .element(|el| {
                    el.set_inner_content("Modified!");
                    Ok(())
                })
            ),
        ],
        ..Settings::default()
    },
)?;

assert_eq!(result, r#"<div class="container"><p>Modified!</p></div>"#);
```

### Sendable Rewriter

For multi-threaded contexts:

```rust
use lol_html::send::{HtmlRewriter, Settings, ElementHandler};

pub type ElementHandler<'h> = ElementHandlerSend<'h>;

pub struct MyHandler;

impl ElementHandler<'_> for MyHandler {
    fn element(&self, el: &mut Element) -> Result<(), Box<dyn std::error::Error>> {
        el.set_attribute("data-processed", "true")?;
        Ok(())
    }
}
```

## Parser State Machine

The HTML parser implements a state machine with 9 state groups:

### State Groups

```rust
// In parser/state_machine/mod.rs

// 1. Data State Group
//    - Handles normal text content
//    - Transitions on '<' to tag states

// 2. CDATA Section State Group
//    - <![CDATA[ ... ]]> sections
//    - Raw text until ]]>

// 3. Plaintext State Group
//    - <plaintext> element content
//    - Raw text until EOF

// 4. Rawtext State Group
//    - <style>, <script>, <title> content
//    - Raw text until </tagname>

// 5. RCDATA State Group
//    - <textarea>, <title> content
//    - Character references + raw text

// 6. Script Data State Group
//    - <script> element content
//    - Special handling for </script>

// 7. Tag State Group
//    - Tag name parsing
//    - Attribute parsing
//    - Self-closing detection

// 8. Attributes State Group
//    - Attribute name/value parsing
//    - Quote handling

// 9. Comment State Group
//    - <!-- ... --> comments
//    - DOCTYPE comments
```

### State Transitions

```
                    ┌─────────────┐
                    │   DATA      │
                    └──────┬──────┘
                           │ '<'
         ┌─────────────────┼─────────────────┐
         │                 │                 │
         ▼                 ▼                 ▼
   ┌──────────┐    ┌──────────────┐   ┌──────────┐
   │   TAG    │    │   COMMENT    │   │  CDATA   │
   └────┬─────┘    └──────────────┘   └──────────┘
        │
        │ Tag name parsed
        │
        ▼
   ┌──────────┐
   │ ATTRIBUTES│
   └────┬─────┘
        │
        │ '>' or '/>'
        │
        ▼
   ┌──────────┐
   │  ELEMENT │
   │  CONTENT │
   └──────────┘
```

## Memory Management

### Memory Limits

```rust
pub struct MemorySettings {
    /// Maximum memory for element handlers
    pub max_allowed_memory_usage: usize,

    /// Threshold for preallocating buffers
    pub prealloc_threshold: f32,
}

impl Default for MemorySettings {
    fn default() -> Self {
        Self {
            max_allowed_memory_usage: 100 * 1024 * 1024,  // 100MB
            prealloc_threshold: 0.8,
        }
    }
}
```

### Memory Limiting Trait

```rust
pub trait MemoryLimiter: Clone + Send + Sync {
    fn try_increase(&self, amount: usize) -> Result<(), MemoryLimitExceededError>;
    fn decrease(&self, amount: usize);
}

pub struct SharedMemoryLimiter {
    usage: AtomicUsize,
    limit: usize,
}
```

### Buffer Preallocation

```rust
const DEFAULT_BUFFER_CAPACITY: usize = 1024;

fn prealloc_if_needed(needed: usize, current: &mut Vec<u8>) {
    if current.capacity() < needed {
        current.reserve(needed - current.capacity());
    }
}
```

## Selector VM

### Selector Compilation

```rust
use cssparser::{Parser, SelectorList, parse_author_origin_selector};

pub struct Selector {
    compiled: cssparser::SelectorList<'static>,
}

impl Selector {
    pub fn parse(selector_text: &str) -> Result<Self, SelectorError> {
        let mut parser_input = cssparser::ParserInput::new(selector_text);
        let mut parser = Parser::new(&mut parser_input);

        let list = parse_author_origin_selector(&mut parser)?;
        Ok(Selector {
            compiled: SelectorList::from(list),
        })
    }
}
```

### Selector Matching

```rust
pub struct SelectorVm {
    selectors: Vec<(Selector, HandlerId)>,
    active_handlers: BitSet,
}

impl SelectorVm {
    pub fn on_element_start(&mut self, element: &ElementData) -> Vec<HandlerId> {
        self.active_handlers.clear();

        for (selector, handler_id) in &self.selectors {
            if selector.matches(element) {
                self.active_handlers.insert(*handler_id);
            }
        }

        self.active_handlers.iter().collect()
    }
}
```

## Rewritable Units

### Element Handler

```rust
pub trait ElementHandler {
    fn element(&self, el: &mut Element) -> Result<(), Box<dyn std::error::Error>> {
        Ok(())
    }
}

// Element manipulation API
pub struct Element<'r, 't> {
    // Internal state
}

impl<'r, 't> Element<'r, 't> {
    /// Get tag name
    pub fn tag_name(&self) -> Cow<'_, str>;

    /// Set tag name
    pub fn set_tag_name(&mut self, name: &str) -> Result<(), TagNameError>;

    /// Get attribute
    pub fn get_attribute(&self, name: &str) -> Option<String>;

    /// Set attribute
    pub fn set_attribute(&mut self, name: &str, value: &str) -> Result<(), AttributeNameError>;

    /// Remove attribute
    pub fn remove_attribute(&mut self, name: &str);

    /// Has attribute
    pub fn has_attribute(&self, name: &str) -> bool;

    /// Get all attributes
    pub fn attributes(&self) -> Vec<Attribute>;

    /// Set inner content
    pub fn set_inner_content(&mut self, content: &str);

    /// Prepend content
    pub fn prepend(&mut self, content: &str);

    /// Append content
    pub fn append(&mut self, content: &str);

    /// Remove element entirely
    pub fn remove(&mut self);

    /// On end tag handler
    pub fn on_end_tag(&mut self, handler: impl FnOnce(&mut EndTag) + 'static);
}
```

### Text Handler

```rust
pub trait TextHandler {
    fn text(&self, text: &mut TextChunk) -> Result<(), Box<dyn std::error::Error>>;
}

pub struct TextChunk<'r, 't> {
    // Internal state
}

impl<'r, 't> TextChunk<'r, 't> {
    /// Get text content
    pub fn as_str(&self) -> &str;

    /// Check if last text chunk in element
    pub fn last_in_element(&self) -> bool;

    /// Replace text
    pub fn replace(&mut self, text: &str);

    /// Append text
    pub fn after(&mut self, text: &str);
}
```

### Comment Handler

```rust
pub trait CommentHandler {
    fn comment(&self, comment: &mut Comment) -> Result<(), Box<dyn std::error::Error>>;
}

pub struct Comment<'r, 't> {
    // Internal state
}

impl<'r, 't> Comment<'r, 't> {
    /// Get comment text
    pub fn text(&self) -> &str;

    /// Set comment text
    pub fn set_text(&mut self, text: &str) -> Result<(), CommentTextError>;
}
```

### End Tag Handler

```rust
pub trait EndTagHandler {
    fn end_tag(&self, tag: &mut EndTag) -> Result<(), Box<dyn std::error::Error>>;
}

pub struct EndTag<'r, 't> {
    // Internal state
}

impl<'r, 't> EndTag<'r, 't> {
    /// Get tag name
    pub fn tag_name(&self) -> &str;

    /// Set tag name
    pub fn set_tag_name(&mut self, name: &str) -> Result<(), TagNameError>;

    /// Remove end tag
    pub fn remove(&mut self);
}
```

### Document-Level Handlers

```rust
pub struct DocumentContentHandlers {
    doctype: Option<Box<dyn DoctypeHandler>>,
    comment: Option<Box<dyn CommentHandler>>,
    text: Option<Box<dyn TextHandler>>,
    end: Option<Box<dyn EndHandler>>,
}

pub trait DoctypeHandler {
    fn doctype(&self, doctype: &mut Doctype) -> Result<(), Box<dyn std::error::Error>>;
}

pub struct Doctype<'r, 't> {
    // DOCTYPE attributes
}

impl<'r, 't> Doctype<'r, 't> {
    pub fn name(&self) -> Option<&str>;
    pub fn public_id(&self) -> Option<&str>;
    pub fn system_id(&self) -> Option<&str>;
}

pub trait EndHandler {
    fn end(&self, doc_end: &mut DocumentEnd) -> Result<(), Box<dyn std::error::Error>>;
}

pub struct DocumentEnd<'r, 't> {
    // End of document marker
}

impl<'r, 't> DocumentEnd<'r, 't> {
    pub fn after(&mut self, content: &str);
}
```

## Encoding Handling

### ASCII-Compatible Encodings

lol-html supports all ASCII-compatible encodings via `encoding_rs`:

```rust
pub static ASCII_COMPATIBLE_ENCODINGS: [&Encoding; 36] = [
    BIG5, EUC_JP, EUC_KR, GB18030, GBK, IBM866,
    ISO_8859_2, ISO_8859_3, ISO_8859_4, ISO_8859_5,
    ISO_8859_6, ISO_8859_7, ISO_8859_8, ISO_8859_8_I,
    ISO_8859_10, ISO_8859_13, ISO_8859_14, ISO_8859_15,
    ISO_8859_16, KOI8_R, KOI8_U, MACINTOSH, SHIFT_JIS,
    UTF_8, WINDOWS_874, WINDOWS_1250, WINDOWS_1251,
    WINDOWS_1252, WINDOWS_1253, WINDOWS_1254, WINDOWS_1255,
    WINDOWS_1256, WINDOWS_1257, WINDOWS_1258,
    X_MAC_CYRILLIC, X_USER_DEFINED,
];
```

### Non-ASCII-Compatible Encodings

```rust
pub static NON_ASCII_COMPATIBLE_ENCODINGS: [&Encoding; 4] =
    [UTF_16BE, UTF_16LE, ISO_2022_JP, REPLACEMENT];
```

These require special handling and are not fully supported.

### Encoding Detection

```rust
pub trait AsciiCompatibleEncoding {
    fn from_html_encoding_attribute(value: &str) -> Option<Self>;
    fn from_bom(bytes: &[u8]) -> Option<Self>;
    fn from_content_type_header(header: &str) -> Option<Self>;
}
```

## Tree Builder Simulator

The tree builder simulator tracks the implicit DOM structure without building an actual tree:

```rust
const DEFAULT_NS_STACK_CAPACITY: usize = 256;

pub struct TreeBuilderSimulator {
    // Namespace stack for SVG/MathML
    ns_stack: Vec<Namespace>,
    // Open element stack
    open_elements: Vec<LocalNameHash>,
    // Current insertion mode
    insertion_mode: InsertionMode,
}

impl TreeBuilderSimulator {
    pub fn on_element_start(&mut self, name: &LocalName, namespace: Namespace) {
        self.open_elements.push(name.hash());
        self.ns_stack.push(namespace);
    }

    pub fn on_element_end(&mut self, name: &LocalName) {
        // Pop matching element
        while let Some(top) = self.open_elements.pop() {
            if top.matches(name) {
                break;
            }
        }
    }

    pub fn current_ns(&self) -> Namespace {
        self.ns_stack.last().copied().unwrap_or(Namespace::Html)
    }
}
```

## Transform Stream API

For advanced use cases with full control over token transformation:

```rust
pub trait TransformController {
    type OutputSink: OutputSink;

    fn on_start_tag(&mut self, tag: StartTag) -> StartTagHandlingResult<Self::OutputSink>;
    fn on_end_tag(&mut self, tag: EndTag);
    fn on_comment(&mut self, comment: Comment);
    fn on_doctype(&mut self, doctype: Doctype);
    fn on_text(&mut self, text: &[u8]);
    fn on_eof(&mut self);
}

pub struct TransformStream<Controller> {
    controller: Controller,
    // Internal state
}
```

## Error Types

```rust
pub mod errors {
    pub use super::memory::MemoryLimitExceededError;
    pub use super::parser::ParsingAmbiguityError;
    pub use super::rewritable_units::{
        AttributeNameError,
        CommentTextError,
        TagNameError,
    };
    pub use super::rewriter::RewritingError;
    pub use super::selectors_vm::SelectorError;
}

#[derive(Debug, thiserror::Error)]
pub enum RewritingError {
    #[error("Memory limit exceeded")]
    MemoryLimitExceeded(#[from] MemoryLimitExceededError),

    #[error("Parsing ambiguity: {0}")]
    ParsingAmbiguity(#[from] ParsingAmbiguityError),

    #[error("Invalid attribute name: {0}")]
    AttributeName(#[from] AttributeNameError),

    #[error("Invalid tag name: {0}")]
    TagName(#[from] TagNameError),

    #[error("Invalid comment text: {0}")]
    CommentText(#[from] CommentTextError),

    #[error("Invalid selector: {0}")]
    SelectorError(#[from] SelectorError),
}
```

## Performance Optimizations

### 1. Streaming Processing

No full document buffering:
```rust
// Process chunk immediately
pub fn write(&mut self, chunk: &[u8]) -> Result<()> {
    self.parser.feed(chunk);
    // Processed tokens emitted to sink immediately
    Ok(())
}
```

### 2. Zero-Copy Token Emission

Tokens reference input buffer where possible:
```rust
pub struct Lexeme<'i, T> {
    input: Bytes<'i>,  // Borrowed from input
    token_outline: T,
    raw_range: Range,
}
```

### 3. Selector Caching

Compiled selectors are cached:
```rust
pub struct SelectorCache {
    compiled: HashMap<String, Selector>,
}

impl SelectorCache {
    pub fn get_or_insert(&mut self, text: &str) -> Result<&Selector> {
        // Use precompiled selector if available
    }
}
```

### 4. Buffer Reuse

Output buffers are reused across chunks:
```rust
struct Rewriter {
    output_buffer: Vec<u8>,
    // ...
}

fn emit(&mut self, bytes: &[u8]) {
    self.output_buffer.clear();
    self.output_buffer.extend_from_slice(bytes);
    (self.sink)(&self.output_buffer);
}
```

## Use Cases

### 1. Content Security Policy

```rust
("script", ElementContentHandlers::new()
    .element(|el| {
        let src = el.get_attribute("src");
        if let Some(s) = src {
            if !is_trusted_source(&s) {
                el.remove();
            }
        }
        Ok(())
    })
)
```

### 2. Image Optimization

```rust
("img", ElementContentHandlers::new()
    .element(|el| {
        el.set_attribute("loading", "lazy")?;
        el.set_attribute("decoding", "async")?;

        // Add srcset for responsive images
        if let Some(src) = el.get_attribute("src") {
            let srcset = generate_srcset(&src);
            el.set_attribute("srcset", &srcset)?;
        }
        Ok(())
    })
)
```

### 3. HTML Minification

```rust
Settings {
    element_content_handlers: vec![
        // Remove comments
        ("*", DocumentContentHandlers::new()
            .comment(|c| {
                c.remove();
                Ok(())
            })
        ),
        // Remove whitespace text nodes
        ("*", DocumentContentHandlers::new()
            .text(|t| {
                if t.as_str().trim().is_empty() {
                    t.replace("");
                }
                Ok(())
            })
        ),
    ],
    ..Settings::default()
}
```

### 4. A/B Testing

```rust
("body", ElementContentHandlers::new()
    .element(|el| {
        if should_show_variant_a() {
            el.append(r#"<div class="experiment-banner">Variant A!</div>"#);
        }
        Ok(())
    })
)
```

## Testing

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_element_removal() {
        let html = r#"<div><script>alert('x')</script><p>Hello</p></div>"#;

        let result = rewrite_str(
            html,
            Settings {
                element_content_handlers: vec![
                    ("script", ElementContentHandlers::new()
                        .element(|el| { el.remove(); Ok(()) })
                    ),
                ],
                ..Settings::default()
            },
        ).unwrap();

        assert_eq!(result, r#"<div><p>Hello</p></div>"#);
    }
}
```

## References

- [lol-html GitHub](https://github.com/cloudflare/lol-html)
- [cssparser Documentation](https://docs.rs/cssparser)
- [encoding_rs Documentation](https://docs.rs/encoding_rs)
- [HTML Living Standard](https://html.spec.whatwg.org/)
