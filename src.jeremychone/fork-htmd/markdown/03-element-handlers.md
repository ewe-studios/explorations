# fork-htmd — Element Handlers

**Source:** `fork-htmd/src/element_handler/` — 12 files, ~600 lines.

Each element handler implements the `ElementHandler` trait, converting a specific HTML element type to its Markdown equivalent. The trait-based design allows users to register custom handlers that override built-in behavior.

## ElementHandler Trait

```rust
// element_handler/mod.rs:32-45
pub trait ElementHandler: Send + Sync {
    fn append(&self) -> Option<String> { None }  // deferred content (reference links)
    
    fn on_visit(
        &self,
        node: &Rc<Node>,
        tag: &str,
        attrs: &[Attribute],
        content: &str,
        options: &Options,
    ) -> Option<String>;
}
```

The trait has two methods:
- `on_visit()` — the main conversion method, called after children are processed. Returns `Some(markdown)` to contribute content or `None` to skip this element.
- `append()` — returns deferred content to be appended at the end of the document. Used only by the anchor handler for reference-style links.

A blanket impl allows any `Fn(Element) -> Option<String>` to be an `ElementHandler`:

```rust
// element_handler/mod.rs:52-72
impl<F> ElementHandler for F
where
    F: (Fn(Element) -> Option<String>) + Send + Sync,
{
    fn on_visit(&self, node, tag, attrs, content, options) -> Option<String> {
        self(Element { node, tag, attrs, content, options })
    }
}
```

## Handler Registry and Lookup

```rust
// element_handler/mod.rs:74-143
pub(crate) struct ElementHandlers {
    pub(crate) rules: Vec<HandlerRule>,
}

struct HandlerRule {
    tags: HashSet<String>,
    pub(crate) handler: Box<dyn ElementHandler>,
}
```

Lookup iterates rules in **reverse order** (so custom handlers override built-in ones):

```rust
// element_handler/mod.rs:145-159
impl ElementHandler for ElementHandlers {
    fn on_visit(&self, node, tag, attrs, content, options) -> Option<String> {
        match self.rules.iter().rev().find(|rule| rule.tags.contains(tag)) {
            Some(rule) => rule.handler.on_visit(node, tag, attrs, content, options),
            None => Some(content.to_string()),  // fallback: pass-through
        }
    }
}
```

Unregistered tags pass through as-is — their child content is preserved but no wrapper is added.

## Handler: Headings (headings.rs)

Converts `h1`–`h6` to ATX (`#`) or Setext (`===`/`---`) style:

```rust
// headings.rs:3-23
pub(super) fn headings_handler(element: Element) -> Option<String> {
    let level = element.tag.chars().nth(1).unwrap() as u32 - '0' as u32;  // "h3" → 3
    let content = element.content.trim_ascii_whitespace();
    
    // Setext only for h1/h2
    if (level == 1 || level == 2) && element.options.heading_style == HeadingStyle::Setex {
        let ch = if level == 1 { "=" } else { "-" };
        Some(format!("\n\n{content}\n{}\n\n", ch.repeat(content.chars().count())))
    } else {
        Some(format!("\n\n{}{content}\n\n", "#".repeat(level as usize)))
    }
}
```

**Aha:** Setext style only applies to h1 and h2 because Markdown's Setext syntax only supports two heading levels (underlined with `=` and `-`). Headings h3–h6 always use ATX style regardless of the `heading_style` option.

## Handler: Code (code.rs)

Handles both inline `<code>` and block `<pre><code>`:

```rust
// code.rs:13-24
pub(super) fn code_handler(element: Element) -> Option<String> {
    let parent = get_parent_node(element.node);
    let is_code_block = parent.as_ref()
        .map(|p| get_node_tag_name(p).is_some_and(|t| t == "pre"))
        .unwrap_or(false);
    
    if is_code_block { handle_code_block(element, &parent.unwrap()) }
    else { handle_inline_code(element) }
}
```

### Inline Code

Handles the tricky case of code containing backticks:

```rust
// code.rs:87-122
fn handle_inline_code(element: Element) -> Option<String> {
    // Case 1: `code` contains a lone backtick → use double backticks: ``code ` here``
    // Case 2: code starts with backtick → add space: `` `start ``
    // Default: wrap with single backticks: `code`
    
    let content = if element.options.preformatted_code {
        handle_preformatted_code(content)  // newlines → spaces
    } else {
        content.trim_ascii_whitespace()
    };
}
```

The backtick detection scans for any backtick not adjacent to another backtick:

```rust
for (idx, c) in chars.iter().enumerate() {
    if c == &'`' {
        let prev = if idx > 0 { chars[idx-1] } else { '\0' };
        let next = if idx < len-1 { chars[idx+1] } else { '\0' };
        if prev != '`' && next != '`' { use_double_backticks = true; break; }
    }
}
```

### Code Block

```rust
// code.rs:26-57
fn handle_code_block(element: Element, parent: &Rc<Node>) -> Option<String> {
    let content = content.strip_suffix('\n').unwrap_or(content);
    
    if options.code_block_style == CodeBlockStyle::Fenced {
        let fence = get_code_fence_marker("`" or "~", content);
        let language = find_language_from_attrs(element.attrs)  // class="language-rust"
            .or_else(|| find_language_from_attrs(parent attrs)); // or from <pre>
        
        Some(format!("{fence}{lang}\n{content}\n{fence}"))
    } else {
        // Indented: 4 spaces per line
        Some(content.lines().map(|l| format!("    {l}")).join("\n"))
    }
}
```

**Aha:** The fence marker adapts to content. If the code contains ```` ``` ```` , it uses ````` ````` ``, and if that also appears, it uses 5. This prevents broken Markdown output when documenting Markdown itself:

```rust
// code.rs:60-72
fn get_code_fence_marker(symbol: &str, content: &str) -> String {
    if content.contains("```") {
        if content.contains("````") { symbol.repeat(5) }
        else { symbol.repeat(4) }
    } else { symbol.repeat(3) }
}
```

Language detection looks for `class="language-xxx"` on both `<code>` and `<pre>`:

```rust
// code.rs:74-85
fn find_language_from_attrs(attrs: &[Attribute]) -> Option<String> {
    attrs.iter()
        .find(|a| &a.name.local == "class")
        .and_then(|a| a.value.split(' ').find(|c| c.starts_with("language-")))
        .map(|lang| lang.split('-').skip(1).join("-"))
}
```

## Handler: Anchor (anchor.rs)

The most complex handler — handles links with three styles and deferred reference links:

```rust
// anchor.rs:13-81
pub(super) struct AnchorElementHandler {}

impl AnchorElementHandler {
    thread_local! {
        static LINKS: RefCell<Vec<String>> = const { RefCell::new(vec![]) };
    }
}
```

**Aha:** The anchor handler uses `thread_local!` storage to collect reference link definitions during traversal, then flushes them in `append()` at the end of the document. This is necessary because reference links (`[text][1]`) need their definitions (`[1]: url`) appended after all content, but the links themselves appear inline.

### on_visit — Link Conversion

```rust
// anchor.rs:34-81
fn on_visit(&self, _node, _tag, attrs, content, options) -> Option<String> {
    // Extract href and title from attributes
    let Some(link) = link else { return Some(content.to_string()) };  // no href → pass-through
    
    let link = link.replace('(', "\\(").replace(')', "\\)");  // escape parens
    
    match options.link_style {
        LinkStyle::Inlined => build_inlined_anchor(content, link, title, false),
        LinkStyle::InlinedPreferAutolinks => build_inlined_anchor(content, link, title, true),
        LinkStyle::Referenced => build_referenced_anchor(content, link, title, &style),
    }
}
```

### Inlined Links

```rust
// anchor.rs:88-115
fn build_inlined_anchor(&self, content, link, title, prefer_autolinks) -> String {
    // Autolink shortcut: if text == URL → <url>
    if prefer_autolinks && content == link {
        return format!("<{link}>");
    }
    
    // Wrap URL in < > if it contains spaces
    // Format: [text](url "title") or [text](url)
}
```

### Reference Links

```rust
// anchor.rs:117-146
fn build_referenced_anchor(&self, content, link, title, style) -> String {
    AnchorElementHandler::LINKS.with(|links| {
        let (current, append) = match style {
            LinkReferenceStyle::Full => ("[text][1]", "[1]: url title"),       // numbered
            LinkReferenceStyle::Collapsed => ("[text][]", "[text]: url title"),  // text key
            LinkReferenceStyle::Shortcut => ("[text]", "[text]: url title"),     // bare
        };
        links.borrow_mut().push(append);
        current
    })
}
```

### append — Flush References

```rust
// anchor.rs:22-32
fn append(&self) -> Option<String> {
    AnchorElementHandler::LINKS.with(|links| {
        let mut links = links.borrow_mut();
        if links.is_empty() { return None; }
        let result = format!("\n\n{}\n\n", links.join("\n"));
        links.clear();
        Some(result)
    })
}
```

Called by the main converter after all elements are processed:

```rust
// lib.rs:131-138
for rule in &self.handlers.rules {
    let Some(append_content) = rule.handler.append() else { continue; };
    append.push_str(&append_content);
}
content.push_str(append.trim_end_matches('\n'));
```

## Handler: Table (table.rs)

Converts HTML tables to pipe-markdown:

```rust
// table.rs:15-136
pub(super) fn table_handler(element: Element) -> Option<String> {
    // Extract caption, thead (headers), tbody/tfoot rows
    // First <tr> without <thead> becomes headers
    
    // Build pipe table:
    // | Header1 | Header2 |
    // | ------- | ------- |
    // | Cell1   | Cell2   |
}
```

Cell content is normalized (newlines → spaces, trimmed):

```rust
// table.rs:155-160
fn normalize_cell_content(content: &str) -> String {
    content.replace('\n', " ").replace('\r', "").trim().to_string()
}
```

## Handler: List Item (li.rs)

Handles both ordered and unordered list items with configurable markers and indentation:

```rust
// li.rs:10-82
pub(super) fn list_item_handler(element: Element) -> Option<String> {
    let content = content.trim_start_ascii_whitespace();
    let content = indent_text_except_first_line(&content, 4, true);  // indent sub-content
    
    // Check parent: if <ol>, use numbered format
    // If <ul>, use bullet format
    
    // For ordered: respect `start` attribute
    let start = ol_attrs.find("start").parse().unwrap_or(1);
    let index = position_of_this_li_in_parent();
    format!("\n{}.{spacing}{content}\n", start + index)
}
```

Multi-level indentation: nested list content (beyond the first line) gets 4-space indent:

```rust
// text_util.rs:148-167
pub(crate) fn indent_text_except_first_line(text: &str, indent: usize, trim_line_end: bool) -> String {
    for (idx, line) in text.lines().enumerate() {
        if idx == 0 { result.push(line); }
        else { result.push(format!("{}{}", " ".repeat(indent), line)); }
    }
}
```

## Handler: Blockquote (blockquote.rs)

```rust
// blockquote.rs:6-14
pub(super) fn blockquote_handler(element: Element) -> Option<String> {
    let content = content.trim_start_matches('\n')
        .trim_end_ascii_whitespace()
        .lines()
        .map(|line| format!("> {line}"))
        .join("\n");
    Some(format!("\n\n{content}\n\n"))
}
```

Each line gets a `> ` prefix, including empty lines (for multi-paragraph blockquotes).

## Handler: Emphasis (emphasis.rs)

Shared handler for both bold (`**`) and italic (`_`):

```rust
// emphasis.rs:6-23
pub(super) fn emphasis_handler(element: Element, marker: &str) -> Option<String> {
    if content.is_empty() { return None; }
    
    let (content, leading_ws) = content.strip_leading_whitespace();
    let (content, trailing_ws) = content.strip_trailing_whitespace();
    if content.is_empty() { return None; }
    
    Some(format!("{leading_ws}{marker}{content}{marker}{trailing_ws}"))
}
```

Whitespace is preserved around the markers so `  **bold**  ` renders correctly.

Called as:
- `emphasis_handler(element, "**")` for `<strong>` and `<b>`
- `emphasis_handler(element, "_")` for `<i>` and `<em>`

## Handler: Image (img.rs)

```rust
// img.rs:6-55
pub(super) fn img_handler(element: Element) -> Option<String> {
    // Extract src/href, alt, title from attributes
    // Escape parens in URL
    // Format: ![alt](src "title")
    // Returns None if no src/href (skip the image)
}
```

## Handler: Horizontal Rule (hr.rs)

```rust
// hr.rs:3-9
pub(super) fn hr_handler(element: Element) -> Option<String> {
    match element.options.hr_style {
        HrStyle::Dashes => Some("\n\n- - -\n\n"),
        HrStyle::Asterisks => Some("\n\n* * *\n\n"),
        HrStyle::Underscores => Some("\n\n_ _ _\n\n"),
    }
}
```

## Handler: Line Break (br.rs)

```rust
// br.rs:3-8
pub(super) fn br_handler(element: Element) -> Option<String> {
    match element.options.br_style {
        BrStyle::TwoSpaces => Some("  \n"),
        BrStyle::Backslash => Some("\\\n"),
    }
}
```

## Handler: Block (mod.rs — catch-all)

The fallback for block-level elements (`p`, `div`, `section`, `article`, `pre`, etc.):

```rust
// mod.rs:161-163
fn block_handler(element: Element) -> Option<String> {
    Some(format!("\n\n{}\n\n", element.content))
}
```

Simply wraps content in double newlines to create paragraph breaks.

## Complete Tag-to-Handler Mapping

| Tag(s) | Handler | Output Format |
|--------|---------|---------------|
| `h1`–`h6` | `headings_handler` | `# Heading\n\n` or `Heading\n===\n\n` |
| `code` (inline) | `code_handler` → `handle_inline_code` | `` `code` `` |
| `pre > code` | `code_handler` → `handle_code_block` | `` ```lang\ncode\n``` `` |
| `a` | `AnchorElementHandler` | `[text](url)` or `[text][1]` + deferred |
| `img` | `img_handler` | `![alt](src)` |
| `strong`, `b` | `bold_handler` → `emphasis_handler(..."**")` | `**bold**` |
| `i`, `em` | `italic_handler` → `emphasis_handler(..."_")` | `_italic_` |
| `ul`, `ol` | `list_handler` | `\n\ncontent\n\n` (wrapper only) |
| `li` | `list_item_handler` | `\n* item\n` or `\n1. item\n` |
| `blockquote` | `blockquote_handler` | `> line\n` per line |
| `table` | `table_handler` | `\| col \| col \|` pipe table |
| `hr` | `hr_handler` | `* * *` or `- - -` or `_ _ _` |
| `br` | `br_handler` | `  \n` or `\\\n` |
| `p`, `div`, `section`, `article`, etc. | `block_handler` | `\n\ncontent\n\n` |
| `head`, `script`, `style`, `body` | `block_handler` | pass-through (content only) |
| Unknown tags | Pass-through | `content.to_string()` |

## What to Read Next

- [Options](04-options-config.md) for all configuration options
- [DOM Walker](02-dom-walker.md) for the traversal algorithm
- [Architecture](01-architecture.md) for the full module map
