# miette Deep Dive: Diagnostic Error Reporting in Rust

**Source:** `/home/darkvoid/Boxxed/@formulas/src.rust/src.zkat/miette/`

**Version:** 7.6.0

---

## Table of Contents

1. [Introduction to Diagnostic Reporting](#introduction-to-diagnostic-reporting)
2. [miette Architecture](#miette-architecture)
3. [Core Traits and Types](#core-traits-and-types)
4. [Error Reporting Patterns](#error-reporting-patterns)
5. [Source Code Snippets](#source-code-snippets)
6. [Comparison to eyre/anyhow](#comparison-to-eyreanyhow)
7. [Advanced Features](#advanced-features)
8. [Custom Handlers and Theming](#custom-handlers-and-theming)

---

## Introduction to Diagnostic Reporting

### The Problem with Traditional Error Handling

Traditional Rust error handling often produces unhelpful messages:

```
Error: IoError: No such file or directory (os error 2)
```

This tells you *what* happened, but not:
- **Where** in your code/file/input the error occurred
- **Why** it matters
- **What** to do about it

### Enter Diagnostic Reporting

miette provides rich, contextual error reports:

```
Error: Configuration file not found

  × Failed to read configuration
   ╭─[config.toml:1:1]
   │
 1 │ [database]
   │ ─────┬────
   │      ╰── File referenced here
   │
   ╰────
  help: Create the file or specify a different path

  Error code: myapp::config::not_found
  See: https://myapp.dev/errors/config/not_found
```

### Design Philosophy

miette is inspired by:
- **Rust compiler diagnostics** - Clear, actionable, with source context
- **ariadne** - Beautiful terminal output
- **thiserror** - Library-friendly error definitions
- **anyhow/eyre** - Application-level error handling

---

## miette Architecture

### Component Diagram

```
┌─────────────────────────────────────────────────────────────────┐
│                     miette Architecture                          │
├─────────────────────────────────────────────────────────────────┤
│                                                                  │
│  ┌───────────────┐     ┌───────────────┐     ┌───────────────┐ │
│  │   Your Error  │────▶│  Diagnostic   │────▶│ ReportHandler │ │
│  │    Types      │     │    Trait      │     │  (Renderer)   │ │
│  └───────────────┘     └───────────────┘     └───────────────┘ │
│         │                     │                     │           │
│         │                     │                     │           │
│         ▼                     ▼                     ▼           │
│  ┌───────────────┐     ┌───────────────┐     ┌───────────────┐ │
│  │  thiserror    │     │ SourceCode    │     │MietteHandler  │ │
│  │   (derive)    │     │ SourceSpan    │     │ (default)     │ │
│  └───────────────┘     └───────────────┘     └───────────────┘ │
│                               │                     │           │
│                               │                     │           │
│                               ▼                     ▼           │
│                        ┌───────────────┐     ┌───────────────┐  │
│                        │ NamedSource   │     │ Fancy Printer │  │
│                        │ (file/string) │     │ (ANSI/Unicode)│  │
│                        └───────────────┘     └───────────────┘  │
│                                                                  │
└─────────────────────────────────────────────────────────────────┘
```

### Key Modules

```
miette/
├── lib.rs              # Core types and traits
├── diagnostic.rs       # Diagnostic trait
├── handler.rs          # ReportHandler implementations
├── source.rs           # SourceCode, SourceSpan, NamedSource
├── error.rs            # Error types and Report
├── handlers/           # Built-in handlers
│   ├── graphical.rs    # Fancy graphical output
│   └── narratable.rs   # Screen-reader friendly
├── highlighters/       # Syntax highlighting
│   └── syntect.rs      # syntect-based highlighting
└── derive/             # Procedural macros
    └── miette-derive/
```

---

## Core Traits and Types

### The Diagnostic Trait

The foundation of miette is the `Diagnostic` trait:

```rust
use miette::Diagnostic;
use std::error::Error;

/// Main diagnostic reporting trait
pub trait Diagnostic: Error {
    /// Unique error code for documentation linking
    fn code<'a>(&'a self) -> Option<Box<dyn Display + 'a>> {
        None
    }

    /// URL for more information about this error
    fn url<'a>(&'a self) -> Option<Box<dyn Display + 'a>> {
        None
    }

    /// Source code related to this error
    fn source_code(&self) -> Option<&dyn SourceCode> {
        None
    }

    /// Labels to highlight in the source
    fn labels(&self) -> Option<Box<dyn Iterator<Item = LabeledSpan> + '_>> {
        None
    }

    /// Help text for resolving the error
    fn help<'a>(&'a self) -> Option<Box<dyn Display + 'a>> {
        None
    }

    /// Related errors (for error chains)
    fn related<'a>(&'a self) -> Option<Box<dyn Iterator<Item = &'a dyn Diagnostic> + 'a>> {
        None
    }

    /// Diagnostic severity (Error, Warning, Advice)
    fn severity(&self) -> Option<Severity> {
        None
    }
}
```

### Deriving Diagnostic

The easiest way to implement `Diagnostic` is with the derive macro:

```rust
use miette::{Diagnostic, SourceSpan};
use thiserror::Error;

#[derive(Error, Debug, Diagnostic)]
#[error("unexpected token: {0}")]
#[diagnostic(
    code(parser::unexpected_token),
    url("https://docs.myapp.dev/errors/parser/unexpected-token"),
    help("try removing the extra syntax")
)]
struct UnexpectedTokenError {
    #[source_code]
    src: NamedSource<String>,

    #[label("unexpected token")]
    span: SourceSpan,
}
```

### SourceSpan and LabeledSpan

```rust
/// A span of source code (offset and length)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SourceSpan {
    offset: SourceOffset,
    length: usize,
}

impl SourceSpan {
    pub fn new(offset: usize, length: usize) -> Self {
        Self {
            offset: SourceOffset::from(offset),
            length,
        }
    }

    // Can be created from various types:
    // - (usize, usize) tuple
    // - Range<usize>
    // - SourceOffset
}

/// A labeled span for highlighting
#[derive(Debug, Clone)]
pub struct LabeledSpan {
    label: Option<String>,
    span: SourceSpan,
    primary: bool,
}

impl LabeledSpan {
    pub fn new(label: Option<String>, offset: usize, length: usize) -> Self {
        Self {
            label,
            span: SourceSpan::new(offset, length),
            primary: false,
        }
    }

    pub fn at(offset: usize, label: &str) -> Self {
        Self::new(Some(label.into()), offset, 1)
    }
}
```

### SourceCode Trait

```rust
/// Trait for types that can provide source code
pub trait SourceCode: Send + Sync {
    /// Read bytes from the source
    fn read_span<'a>(
        &'a self,
        span: &SourceSpan,
        context_lines_before: usize,
        context_lines_after: usize,
    ) -> Result<Box<dyn SpanContents<'a> + 'a>, MietteError>;
}

// Implementations for:
// - String
// - &str
// - Vec<u8>
// - &[u8]
// - NamedSource<T>
// - Custom types
```

### NamedSource

```rust
/// Source code with an associated name (filename)
#[derive(Clone, Debug)]
pub struct NamedSource<T: SourceCode> {
    source: T,
    name: String,
    language: Option<String>,
}

impl<T: SourceCode> NamedSource<T> {
    pub fn new(name: impl Into<String>, source: T) -> Self {
        Self {
            source,
            name: name.into(),
            language: None,
        }
    }

    /// Set the language for syntax highlighting
    pub fn with_language(mut self, language: impl Into<String>) -> Self {
        self.language = Some(language.into());
        self
    }
}

// Usage:
let source = NamedSource::new("config.toml", source_code)
    .with_language("toml");
```

---

## Error Reporting Patterns

### Pattern 1: Basic Error with Source

```rust
use miette::{Diagnostic, NamedSource, SourceSpan};
use thiserror::Error;

#[derive(Error, Diagnostic, Debug)]
#[error("syntax error")]
#[diagnostic(code(myapp::syntax))]
struct SyntaxError {
    #[source_code]
    src: NamedSource<String>,

    #[label("unexpected character")]
    span: SourceSpan,
}

fn parse_input(input: &str) -> Result<(), SyntaxError> {
    // Find error position
    let error_pos = 10;
    Err(SyntaxError {
        src: NamedSource::new("input.txt", input.to_string()),
        span: (error_pos, 1).into(),
    })
}
```

### Pattern 2: Multiple Labels

```rust
#[derive(Error, Diagnostic, Debug)]
#[error("type mismatch")]
#[diagnostic(code(myapp::types::mismatch))]
struct TypeMismatch {
    #[source_code]
    src: NamedSource<String>,

    #[label("expected type: {expected}")]
    expected_span: SourceSpan,

    #[label("got type: {actual}")]
    actual_span: SourceSpan,

    expected: String,
    actual: String,
}
```

### Pattern 3: Primary Label

```rust
#[derive(Error, Diagnostic, Debug)]
#[error("undefined variable")]
struct UndefinedVar {
    #[source_code]
    src: NamedSource<String>,

    #[label(primary, "variable not defined")]
    primary_span: SourceSpan,

    #[label("defined here with different name")]
    definition_span: SourceSpan,
}
```

### Pattern 4: Label Collection

```rust
#[derive(Error, Diagnostic, Debug)]
#[error("multiple unused variables")]
struct UnusedVars {
    #[source_code]
    src: NamedSource<String>,

    #[label(collection, "unused variable")]
    spans: Vec<SourceSpan>,
}

// Usage:
let error = UnusedVars {
    src: NamedSource::new("main.rs", source),
    spans: vec![(10, 5).into(), (50, 7).into(), (100, 4).into()],
};
```

### Pattern 5: Help Field

```rust
#[derive(Error, Diagnostic, Debug)]
#[error("file not found: {path}")]
struct FileNotFound {
    path: String,

    #[help]
    advice: Option<String>,
}

let err = FileNotFound {
    path: "/etc/myapp/config.toml".into(),
    advice: Some("Run `myapp init` to create default configuration".into()),
};
```

### Pattern 6: Related Errors

```rust
#[derive(Error, Diagnostic, Debug)]
#[error("batch processing failed")]
struct BatchError {
    #[related]
    errors: Vec<IndividualError>,
}

#[derive(Error, Diagnostic, Debug)]
#[error("item {id} failed: {reason}")]
struct IndividualError {
    id: usize,
    reason: String,
}
```

### Pattern 7: Diagnostic Source

```rust
#[derive(Error, Diagnostic, Debug)]
#[error("operation failed")]
struct WrapperError {
    #[source]
    #[diagnostic_source]
    inner: InnerError,
}

#[derive(Error, Diagnostic, Debug)]
#[error("inner error details")]
struct InnerError {
    #[label("problem here")]
    span: SourceSpan,
}
```

### Pattern 8: Severity Levels

```rust
use miette::Severity;

#[derive(Error, Diagnostic, Debug)]
#[error("deprecated function")]
#[diagnostic(severity(Warning))]
struct DeprecatedWarning {
    #[source_code]
    src: NamedSource<String>,
    #[label("this function is deprecated")]
    span: SourceSpan,
}

#[derive(Error, Diagnostic, Debug)]
#[error("suggestion")]
#[diagnostic(severity(Advice))]
struct Suggestion {
    message: String,
}
```

---

## Source Code Snippets

### Basic Snippet Output

```rust
use miette::{miette, NamedSource, SourceSpan};
use miette::LabeledSpan;

let report = miette!(
    labels = vec![
        LabeledSpan::at(12, "this should be 6"),
    ],
    help = "'*' has greater precedence than '+'",
    "Wrong answer"
).with_source_code(NamedSource::new("math.txt", "2 + 2 * 2 = 8"));

println!("{:?}", report);
```

Output:
```
Error: Wrong answer
   ╭─[math.txt:1:1]
   │
 1 │ 2 + 2 * 2 = 8
   │         ┬
   │         ╰── this should be 6
   │
   ╰────
  help: '*' has greater precedence than '+'
```

### Multi-line Snippets

```rust
#[derive(Error, Diagnostic, Debug)]
#[error("unterminated string")]
struct UnterminatedString {
    #[source_code]
    src: NamedSource<String>,

    #[label("string starts here")]
    start: SourceSpan,

    #[label("expected closing quote")]
    end: SourceSpan,
}

let src = r#"fn main() {
    let greeting = "Hello, world!;
    println!(greeting);
}"#;

let error = UnterminatedString {
    src: NamedSource::new("main.rs", src.to_string()),
    start: (17, 1).into(),  // Position of opening "
    end: (37, 0).into(),    // Position where " should be
};
```

Output:
```
Error: unterminated string
   ╭─[main.rs:1:1]
   │
 1 │ fn main() {
 2 │     let greeting = "Hello, world!;
   │                    ┬
   │                    ╰── string starts here
 3 │     println!(greeting);
   │                     ▲
   │                     ╰── expected closing quote
 4 │ }
   ╰────
```

### Context Lines

```rust
// Control context lines in handler
use miette::MietteHandlerOpts;

miette::set_hook(Box::new(|_| {
    Box::new(
        MietteHandlerOpts::new()
            .context_lines(3)  // Show 3 lines before/after
            .build(),
    )
}));
```

### Syntax Highlighting

```rust
// Enable with syntect-highlighter feature
use miette::{Diagnostic, NamedSource};

#[derive(Error, Diagnostic, Debug)]
#[error("parse error")]
struct ParseError {
    #[source_code]
    src: NamedSource<String>,
    #[label]
    span: SourceSpan,
}

// Set language for syntax highlighting
let src = NamedSource::new("config.rs", source_code)
    .with_language("Rust");
```

---

## Comparison to eyre/anyhow

### Feature Comparison

| Feature | miette | eyre/anyhow |
|---------|--------|-------------|
| Diagnostic trait | ✓ | ✗ |
| Source snippets | ✓ | ✗ |
| Error codes | ✓ | ✗ |
| Diagnostic URLs | ✓ | ✗ |
| Multiple labels | ✓ | ✗ |
| Syntax highlighting | ✓ | ✗ |
| Screen reader support | ✓ | ✗ |
| Context/ WrapErr | ✓ | ✓ |
| Report type | ✓ | ✓ |
| Library-friendly | ✓ | ✗ |
| Custom handlers | ✓ | Limited |
| Fancy output | ✓ | Basic |

### When to Use miette

**Use miette when:**
- You need source code snippets in errors
- You want rich diagnostic output
- You're building a compiler/interpreter/linter
- You need error codes and documentation links
- You care about accessibility (screen readers)
- You're writing application code (not libraries)

**Use anyhow when:**
- You just need simple error propagation
- You don't need source context
- You want minimal dependencies
- You're writing library code (libraries should use concrete error types)

### Migration Example

**Before (anyhow):**
```rust
use anyhow::{Context, Result};

fn parse_config(path: &str) -> Result<Config> {
    let content = std::fs::read_to_string(path)
        .with_context(|| format!("Failed to read {}", path))?;
    toml::from_str(&content)
        .context("Failed to parse TOML")
}
```

**After (miette):**
```rust
use miette::{Context, Result, NamedSource};

fn parse_config(path: &str) -> Result<Config> {
    let content = std::fs::read_to_string(path)
        .into_diagnostic()
        .wrap_err_with(|| format!("Failed to read {}", path))?;

    toml::from_str(&content)
        .map_err(|e: toml::de::Error| {
            miette!(
                labels = vec![LabeledSpan::at(e.column(), "parse error here")],
                "Failed to parse TOML"
            )
            .with_source_code(NamedSource::new(path, content))
        })
}
```

---

## Advanced Features

### Result and Report Types

```rust
use miette::{Result, Report};

// miette::Result<T> is an alias for Result<T, Report>
fn my_function() -> Result<()> {
    // Return Ok or Err
    Ok(())
}

// Report is the error type - it's already a boxed diagnostic
fn returns_report() -> Report {
    miette!("something went wrong")
}
```

### The miette! Macro

```rust
use miette::{miette, LabeledSpan};

// Basic error
let err = miette!("simple error message");

// With code
let err = miette!(
    code = "myapp::error::not_found",
    "file not found"
);

// With labels
let err = miette!(
    labels = vec![
        LabeledSpan::at(10, "error here"),
    ],
    "syntax error"
);

// With help
let err = miette!(
    help = "try running with --verbose",
    "operation failed"
);

// With severity
let err = miette!(
    severity = Severity::Warning,
    "deprecated usage"
);
```

### IntoDiagnostic Trait

```rust
use miette::IntoDiagnostic;

// Convert std::error::Error to miette::Result
fn read_file() -> miette::Result<String> {
    std::fs::read_to_string("file.txt").into_diagnostic()
}

// With context
fn read_config() -> miette::Result<Config> {
    let path = "config.toml";
    std::fs::read_to_string(path)
        .into_diagnostic()
        .wrap_err_with(|| format!("Failed to read config: {}", path))
}
```

### bail! and ensure! Macros

```rust
use miette::{bail, ensure, Result};

fn parse_number(s: &str) -> Result<i32> {
    // bail! for early error return
    if s.is_empty() {
        bail!("empty string is not a valid number");
    }

    // ensure! for condition checking
    let num: i32 = s.parse().ok().ok_or_else(|| {
        miette!("invalid number format: {}", s)
    })?;

    ensure!(num > 0, "number must be positive, got {}", num);

    Ok(num)
}
```

### Diagnostic Code URLs

```rust
#[derive(Error, Diagnostic, Debug)]
#[error("connection timeout")]
#[diagnostic(
    code(network::timeout),
    // Static URL
    url("https://docs.myapp.dev/errors/network/timeout")
)]
struct TimeoutError;

#[derive(Error, Diagnostic, Debug)]
#[error("invalid input")]
#[diagnostic(
    code(input::invalid),
    // Dynamic URL based on error code
    url("https://docs.myapp.dev/errors/{}", self.code().unwrap())
)]
struct InvalidInput;

#[derive(Error, Diagnostic, Debug)]
#[error("library error")]
#[diagnostic(
    code(my_lib::error),
    // Auto-link to docs.rs
    url(docsrs)
)]
struct LibraryError;
```

### Delayed Source Code

```rust
use miette::{Diagnostic, SourceSpan};
use thiserror::Error;

#[derive(Error, Diagnostic, Debug)]
#[error("validation error")]
struct ValidationError {
    #[label]
    span: SourceSpan,
    // Note: no #[source_code] here
}

fn validate(data: &str) -> miette::Result<()> {
    // Error without source initially
    let err = ValidationError {
        span: (10, 5).into(),
    };

    // Add source code later
    Err(err).map_err(|e| e.with_source_code(data.to_string()))
}
```

### Custom Highlighters

```rust
use miette::highlighters::{Highlighter, HighlighterState};

struct MyCustomHighlighter;

impl Highlighter for MyCustomHighlighter {
    fn highlight<'a>(
        &'a self,
        source: &'a str,
        language: Option<&str>,
    ) -> Box<dyn std::fmt::Display + 'a> {
        // Custom highlighting logic
        Box::new(format!("[HIGHLIGHTED] {}", source))
    }
}

// Use with handler
miette::set_hook(Box::new(|_| {
    Box::new(
        MietteHandlerOpts::new()
            .with_syntax_highlighting(Box::new(MyCustomHighlighter))
            .build(),
    )
}));
```

---

## Custom Handlers and Theming

### MietteHandlerOpts

```rust
use miette::MietteHandlerOpts;

miette::set_hook(Box::new(|_| {
    Box::new(
        MietteHandlerOpts::new()
            // Terminal features
            .terminal_links(true)      // Enable hyperlink support
            .unicode(true)             // Use unicode characters
            .color(true)               // Enable colors

            // Output formatting
            .context_lines(3)          // Lines of context
            .tab_width(4)              // Tab expansion
            .width(80)                 // Terminal width

            // Behavior
            .break_words(true)         // Break long lines
            .word_separator(textwrap::WordSeparator::AsciiSpace)
            .wrap_lines(true)          // Wrap long lines

            // Debugging
            .debug(false)              // Debug output

            // Syntax highlighting
            .with_syntax_highlighting(...)

            .build(),
    )
}));
```

### Narratable Handler (Accessibility)

```rust
// Automatically used when:
// - NO_COLOR is set
// - CLICOLOR=0
// - Running on CI
// - Terminal doesn't support colors

// The narratable printer outputs screen-reader friendly text:
/*
Error: Received some bad JSON from the source. Unable to parse.
    Caused by: missing field `foo` at line 1 column 1700

Begin snippet for https://api.nuget.org/v3/registration5-gz-semver2/json.net/index.json starting at line 1, column 1659

snippet line 1: gs&quot;:[&quot;json&quot;],&quot;title&quot;:&quot;&quot;,&quot;version&quot;:&quot;1.0.0&quot;},&quot;packageContent&quot;:&quot;https://api.nuget.o
    highlight starting at line 1, column 1699: last parsing location

diagnostic help: This is a bug. It might be in ruget, or it might be in the source you're using, but it's definitely a bug and should be reported.
diagnostic error code: ruget::api::bad_json
*/
```

### Completely Custom Handler

```rust
use miette::{ReportHandler, Diagnostic, Protocol};
use std::fmt;

struct MyCustomHandler;

impl ReportHandler for MyCustomHandler {
    fn debug(
        &self,
        diagnostic: &(dyn Diagnostic),
        f: &mut fmt::Formatter<'_>,
    ) -> fmt::Result {
        // Custom formatting logic
        if let Some(code) = diagnostic.code() {
            write!(f, "[{}] ", code)?;
        }

        write!(f, "{}", diagnostic)?;

        if let Some(help) = diagnostic.help() {
            write!(f, "\nHelp: {}", help)?;
        }

        Ok(())
    }
}

// Install custom handler
miette::set_hook(Box::new(|_| Box::new(MyCustomHandler)))?;
```

---

## Summary

miette is a powerful diagnostic reporting library that provides:

1. **Rich Diagnostics:** Source snippets, labels, and context
2. **Derive Macros:** Easy implementation via attributes
3. **Error Codes:** Unique identifiers with URL links
4. **Accessibility:** Screen reader support via narratable printer
5. **Syntax Highlighting:** Integration with syntect
6. **Customization:** Custom handlers and theming
7. **Compatibility:** Works with thiserror, anyhow-style context
8. **Production Ready:** Used in miette, cacache, orogene, and more

The library excels at providing actionable, helpful error messages that guide users toward solutions rather than just reporting failures.
