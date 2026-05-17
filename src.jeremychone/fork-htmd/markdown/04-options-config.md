# fork-htmd — Options and Configuration

**Source:** `fork-htmd/src/options.rs` (96 lines), `lib.rs` builder methods (lines 146-215).

The `Options` struct controls every aspect of the Markdown output format — heading style, code fence type, link format, list markers, and more. Combined with the builder pattern and custom handler API, fork-htmd supports full customization without modifying the core engine.

## Options Struct

```rust
// options.rs:2-18
pub struct Options {
    pub heading_style: HeadingStyle,          // How headings are rendered
    pub hr_style: HrStyle,                    // Horizontal rule style
    pub br_style: BrStyle,                    // Line break style
    pub link_style: LinkStyle,                // Link format
    pub link_reference_style: LinkReferenceStyle,  // Reference link format
    pub code_block_style: CodeBlockStyle,     // Code block format
    pub code_block_fence: CodeBlockFence,     // Code fence character
    pub bullet_list_marker: BulletListMarker, // Unordered list bullet character
    pub ul_bullet_spacing: u8,                // Spaces after bullet
    pub ol_number_spacing: u8,                // Spaces after number
    pub preformatted_code: bool,              // Inline code whitespace handling
}
```

## Default Values

```rust
// options.rs:20-36
impl Default for Options {
    fn default() -> Self {
        Self {
            heading_style: HeadingStyle::Atx,          // # Heading
            hr_style: HrStyle::Asterisks,              // * * *
            br_style: BrStyle::TwoSpaces,              // "  \n"
            link_style: LinkStyle::Inlined,            // [text](url)
            link_reference_style: LinkReferenceStyle::Full,  // [text][1]
            code_block_style: CodeBlockStyle::Fenced,  // ```code```
            code_block_fence: CodeBlockFence::Backticks,    // ```
            bullet_list_marker: BulletListMarker::Asterisk, // * item
            ul_bullet_spacing: 3,                      // "*   text"
            ol_number_spacing: 2,                      // "1.  text"
            preformatted_code: false,                  // trim whitespace in inline code
        }
    }
}
```

## All Option Enums

### HeadingStyle

| Variant | h1 Output | h2 Output | h3 Output |
|---------|-----------|-----------|-----------|
| `Atx` | `# Heading` | `## Heading` | `### Heading` |
| `Setext` | `Heading\n=====` | `Heading\n-----` | Falls back to ATX |

```rust
// options.rs:39-43
pub enum HeadingStyle { Atx, Setext }
```

**Aha:** Setext only supports h1 and h2 because the Setext heading syntax in CommonMark only defines two levels (`===` and `---`). For h3–h6, the converter falls back to ATX style even when `HeadingStyle::Setext` is configured.

### HrStyle

| Variant | Output |
|---------|--------|
| `Dashes` | `- - -` |
| `Asterisks` | `* * *` |
| `Underscores` | `_ _ _` |

```rust
// options.rs:45-52
pub enum HrStyle { Dashes, Asterisks, Underscores }
```

### BrStyle

| Variant | Output |
|---------|--------|
| `TwoSpaces` | `  \n` (two trailing spaces) |
| `Backslash` | `\\\n` (backslash + newline) |

### LinkStyle

| Variant | Example | When to Use |
|---------|---------|-------------|
| `Inlined` | `[Google](https://google.com)` | Default — readable, self-contained |
| `InlinedPreferAutolinks` | `<https://google.com>` | When text equals URL — produces cleaner output |
| `Referenced` | `[Google][1]` + `[1]: https://google.com` | For documents with repeated URLs |

```rust
// options.rs:83-89
pub enum LinkStyle { Inlined, InlinedPreferAutolinks, Referenced }
```

When `InlinedPreferAutolinks` is used and the link text exactly matches the URL, it produces a CommonMark autolink (`<url>`) instead of a full anchor:

```rust
// anchor.rs:95-97
if prefer_autolinks && content == link {
    return format!("<{link}>");
}
```

### LinkReferenceStyle

| Variant | Inline Format | Reference Format |
|---------|--------------|-----------------|
| `Full` | `[text][1]` | `[1]: url "title"` |
| `Collapsed` | `[text][]` | `[text]: url "title"` |
| `Shortcut` | `[text]` | `[text]: url "title"` |

```rust
// options.rs:91-96
pub enum LinkReferenceStyle { Full, Collapsed, Shortcut }
```

The reference definitions are collected during traversal via `thread_local!` storage in `AnchorElementHandler` and appended at the end of the document.

### CodeBlockStyle

| Variant | Output |
|---------|--------|
| `Fenced` | `` ```rust\ncode\n``` `` |
| `Indented` | `    code` (4-space indent per line) |

### CodeBlockFence

| Variant | Fence |
|---------|-------|
| `Backticks` | `` ``` `` |
| `Tildes` | `~~~` |

Only used when `code_block_style` is `Fenced`.

### BulletListMarker

| Variant | Output |
|---------|--------|
| `Asterisk` | `* item` |
| `Dash` | `- item` |

```rust
// options.rs:75-80
pub enum BulletListMarker { Asterisk, Dash }
```

## Builder Pattern

```rust
// lib.rs:146-215
pub struct HtmlToMarkdownBuilder {
    options: Options,
    handlers: ElementHandlers,
    scripting_enabled: bool,
}

impl HtmlToMarkdownBuilder {
    pub fn new() -> Self { ... }
    pub fn options(mut self, options: Options) -> Self { ... }
    pub fn skip_tags(self, tags: Vec<&str>) -> Self { ... }
    pub fn add_handler<Handler>(mut self, tags: Vec<&str>, handler: Handler) -> Self { ... }
    pub fn scripting_enabled(mut self, enabled: bool) -> Self { ... }
    pub fn build(self) -> HtmlToMarkdown { ... }
}
```

## Usage Examples

### Skip Tags

```rust
let converter = HtmlToMarkdown::builder()
    .skip_tags(vec!["img", "script"])
    .build();
let md = converter.convert("<img src='x.png'><h1>Hi</h1>").unwrap();
// Output: "# Hi"  (img is skipped)
```

`skip_tags` is just sugar for adding a handler that returns `None`:

```rust
// lib.rs:175-177
pub fn skip_tags(self, tags: Vec<&str>) -> Self {
    self.add_handler(tags, |_: Element| None)
}
```

### Custom Handler

```rust
let converter = HtmlToMarkdown::builder()
    .add_handler(vec!["video"], |element: Element| {
        let src = element.attrs.iter()
            .find(|a| a.name.local == "src")
            .map(|a| a.value.to_string())
            .unwrap_or_default();
        Some(format!("[Video: {src}]"))
    })
    .build();
```

### Override Built-in Handler

Custom handlers override built-in ones because the registry searches rules in **reverse order** (custom handlers are added last):

```rust
// Override the default heading style
let converter = HtmlToMarkdown::builder()
    .add_handler(vec!["h1"], |element: Element| {
        // Custom: always use Setext for h1, regardless of option
        Some(format!("\n\n{}\n{}\n\n", element.content, "=".repeat(element.content.len())))
    })
    .build();
```

### Struct Update Syntax for Options

```rust
let converter = HtmlToMarkdown::builder()
    .options(Options {
        heading_style: HeadingStyle::Setext,
        code_block_fence: CodeBlockFence::Tildes,
        bullet_list_marker: BulletListMarker::Dash,
        ..Default::default()
    })
    .build();
```

## Scripting Flag

Controls `<noscript>` tag handling:

```rust
// lib.rs:204-209
pub fn scripting_enabled(mut self, enabled: bool) -> Self {
    self.scripting_enabled = enabled;
    self
}
```

| Value | `<noscript>` Content |
|-------|---------------------|
| `true` (default) | Treated as raw text — not parsed as DOM |
| `false` | Parsed as normal DOM — child elements are converted |

This maps to `html5ever`'s `TreeBuilderOpts.scripting_enabled`:

```rust
// lib.rs:107-109
ParseOpts {
    tree_builder: TreeBuilderOpts {
        scripting_enabled: self.scripting_enabled,
        ..Default::default()
    },
}
```

## Code Preformatting

The `preformatted_code` option controls inline code whitespace handling:

```rust
// options.rs:17
pub preformatted_code: bool,  // default: false
```

| Value | Behavior |
|-------|----------|
| `false` (default) | `trim_ascii_whitespace()` — leading/trailing whitespace is stripped |
| `true` | `handle_preformatted_code()` — newlines become spaces, extra space at start/end |

The preformat mode converts newlines to spaces and adds an extra space at the start/end if the code began or ended with a newline:

```rust
// code.rs:125-146
fn handle_preformatted_code(code: &str) -> String {
    for ch in code.chars() {
        if ch == '\n' {
            result.push(' ');
            is_prev_ch_new_line = true;
        } else {
            if is_prev_ch_new_line && !in_middle { result.push(' '); }
            result.push(ch);
            // ...
        }
    }
}
```

This ensures that multi-line inline code like `code\nhere` becomes `` `code here` `` rather than `` `code\nhere` `` which would break inline rendering.

## Spacing Options

### ul_bullet_spacing

Controls spaces between the bullet character and content:

| Value | Output |
|-------|--------|
| `0` | `*item` |
| `3` (default) | `*   item` |
| `4` | `*    item` |

Used in `li.rs`:

```rust
// li.rs:20
let spacing = " ".repeat(element.options.ul_bullet_spacing.into());
// Output: "\n*{spacing}{content}\n"
```

### ol_number_spacing

Controls spaces between the number and content:

| Value | Output |
|-------|--------|
| `0` | `1.item` |
| `2` (default) | `1.  item` |

Used in `li.rs`:

```rust
// li.rs:25
let spacing = " ".repeat(element.options.ol_number_spacing.into());
// Output: "\n{index}.{spacing}{content}\n"
```

## What to Read Next

- [Element Handlers](03-element-handlers.md) for how options are used by each handler
- [DOM Walker](02-dom-walker.md) for the traversal algorithm
- [Overview](00-overview.md) for the complete API surface
