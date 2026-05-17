# fork-htmd — Architecture

**Source:** `fork-htmd/src/` — 17 Rust files, ~2,700 lines.

fork-htmd is structured as a three-layer system: DOM parsing (html5ever), tree walking (dom_walker), and element conversion (element_handler registry). The text utility layer handles HTML entity decoding, whitespace compression, and Markdown syntax escaping throughout.

## Module Structure

```
fork-htmd/src/
├── lib.rs                          # Public API: convert(), HtmlToMarkdown, Element
├── dom_walker.rs                   # DOM traversal + text escaping + whitespace handling
├── node_util.rs                    # Node helpers: tag name, parent, children, content
├── text_util.rs                    # Whitespace compression, Markdown escaping, concat macro
├── options.rs                      # Options struct + all config enums
│
└── element_handler/
    ├── mod.rs                      # ElementHandler trait + ElementHandlers registry
    ├── anchor.rs                   # <a> → [text](url) with thread-local reference links
    ├── code.rs                     # <code> inline/block with fence adaptation
    ├── table.rs                    # <table> → pipe-markdown table
    ├── list.rs                     # <ul>/<ol> wrapper
    ├── li.rs                       # <li> item with numbering and indentation
    ├── headings.rs                 # <h1>-<h6> → ATX or Setext
    ├── blockquote.rs               # <blockquote> → > prefix
    ├── emphasis.rs                 # <strong>/<em> → **bold**/_italic_
    ├── img.rs                      # <img> → ![alt](src)
    ├── br.rs                       # <br> → "  \n" or "\\\n"
    └── hr.rs                       # <hr> → "* * *" or "- - -" or "_ _ _"
```

## Layer Architecture

```mermaid
flowchart TB
    subgraph "Layer 1: DOM Parsing"
        HTML["HTML string"]
        HTML5["html5ever::parse_document"]
        DOM["RcDom tree\n(Rc<Node>)"]
    end

    subgraph "Layer 2: DOM Walking"
        WALK["walk_node()\ndepth-first traversal"]
        TEXT["append_text()\nentity decode + ws compress + escape"]
        VISIT["visit_element()\nrecurse children then call handler"]
        BUFFER["buffer: Vec<String>\nraw child text collected"]
        JOIN["join_contents()\nintelligent newline merging"]
    end

    subgraph "Layer 3: Element Conversion"
        REGISTRY["ElementHandlers\nhandler registry (tag → handler)"]
        TRAIT["ElementHandler trait\non_visit() -> Option<String>"]
        BUILTINS["13+ built-in handlers\nanchor, code, table, list, etc."]
    end

    subgraph "Utilities"
        NODE["node_util.rs\ntag name, parent, children"]
        TEXTUTIL["text_util.rs\ncompress_ws, escape, indent"]
    end

    HTML --> HTML5
    HTML5 --> DOM
    DOM --> WALK
    WALK --> TEXT
    WALK --> VISIT
    VISIT --> BUFFER
    VISIT --> REGISTRY
    REGISTRY --> TRAIT
    TRAIT --> BUILTINS
    BUILTINS --> NODE
    BUILTINS --> TEXTUTIL
    BUFFER --> JOIN
```

## Conversion Pipeline: Full Data Flow

```mermaid
sequenceDiagram
    participant User
    participant API as HtmlToMarkdown
    participant Parse as html5ever
    participant Walker as dom_walker
    participant Handler as ElementHandlers
    participant H as Specific Handler
    participant Join as join_contents

    User->>API: convert(html)
    API->>Parse: parse_document(html)
    Parse-->>API: RcDom tree

    API->>Walker: walk_node(document)
    
    loop Each DOM node
        alt Text node
            Walker->>Walker: append_text()
            Walker->>Walker: html_escape::decode_html_entities
            Walker->>Walker: compress_whitespace
            Walker->>Walker: escape_if_needed (markdown chars)
            Walker->>Walker: push to buffer
        else Element node
            Walker->>Walker: walk_children (recursive)
            Walker->>Handler: on_visit(node, tag, attrs, joined_children)
            Handler->>Handler: find handler by tag (reverse rules order)
            alt Has handler
                Handler->>H: call specific handler
                H-->>Handler: Option<String> (Markdown)
            else No handler
                Handler-->>Walker: Some(content) (pass-through)
            end
            Handler-->>Walker: Markdown string
            Walker->>Walker: push to buffer
        end
    end

    Walker-->>API: buffer collected
    
    API->>Join: join_contents(buffer)
    Join-->>API: merged Markdown

    API->>API: collect append() from handlers (reference links)
    API->>API: trim trailing newlines
    API-->>User: Final Markdown string
```

## Core Types and Relationships

```mermaid
classDiagram
    class HtmlToMarkdown {
        -options: Options
        -handlers: ElementHandlers
        -scripting_enabled: bool
        +new()
        +builder() HtmlToMarkdownBuilder
        +convert(html) Result~String~
    }

    class HtmlToMarkdownBuilder {
        -options: Options
        -handlers: ElementHandlers
        -scripting_enabled: bool
        +new()
        +options(Options) Self
        +skip_tags(Vec&lt;str&gt;) Self
        +add_handler(tags, Handler) Self
        +scripting_enabled(bool) Self
        +build() HtmlToMarkdown
    }

    class Element {
        +node: &amp;Rc~Node~
        +tag: &amp;str
        +attrs: &amp;[Attribute]
        +content: &amp;str
        +options: &amp;Options
    }

    class ElementHandler {
        <<trait>>
        +append() Option~String~
        +on_visit(node, tag, attrs, content, options) Option~String~
    }

    class ElementHandlers {
        -rules: Vec~HandlerRule~
        +new()
        +add_handler(tags, Handler)
    }

    class HandlerRule {
        -tags: HashSet~String~
        +handler: Box~dyn ElementHandler~
    }

    class Options {
        +heading_style: HeadingStyle
        +hr_style: HrStyle
        +br_style: BrStyle
        +link_style: LinkStyle
        +link_reference_style: LinkReferenceStyle
        +code_block_style: CodeBlockStyle
        +code_block_fence: CodeBlockFence
        +bullet_list_marker: BulletListMarker
        +ul_bullet_spacing: u8
        +ol_number_spacing: u8
        +preformatted_code: bool
    }

    HtmlToMarkdownBuilder --> HtmlToMarkdown : builds
    HtmlToMarkdown --> ElementHandlers : uses
    HtmlToMarkdown --> Options : uses
    ElementHandlers o-- HandlerRule
    HandlerRule --> ElementHandler : delegates
    Element --> Options : carries
    Element --> ElementHandler : passed to
```

## Handler Registration

Built-in handlers are registered in `ElementHandlers::new()` in reverse order of specificity:

```rust
// element_handler/mod.rs:80-129
pub fn new() -> Self {
    let mut handlers = Self { rules: Vec::new() };
    
    handlers.add_handler(vec!["img"], img_handler);           // specific
    handlers.add_handler(vec!["a"], AnchorElementHandler::new());
    handlers.add_handler(vec!["ol", "ul"], list_handler);
    handlers.add_handler(vec!["li"], list_item_handler);
    handlers.add_handler(vec!["blockquote"], blockquote_handler);
    handlers.add_handler(vec!["code"], code_handler);
    handlers.add_handler(vec!["strong", "b"], bold_handler);
    handlers.add_handler(vec!["i", "em"], italic_handler);
    handlers.add_handler(vec!["h1".."h6"], headings_handler);
    handlers.add_handler(vec!["br"], br_handler);
    handlers.add_handler(vec!["hr"], hr_handler);
    handlers.add_handler(vec!["table"], table_handler);
    handlers.add_handler(vec!["p", "pre", "div", ...], block_handler); // catch-all
    
    handlers
}
```

Handler lookup uses **reverse iteration** through the rules:

```rust
// element_handler/mod.rs:154
match self.rules.iter().rev().find(|rule| rule.tags.contains(tag)) {
    Some(rule) => rule.handler.on_visit(...),
    None => Some(content.to_string()),  // pass-through for unregistered tags
}
```

**Aha:** Custom handlers are added via `add_handler()` which appends to the end of the rules vector. Since lookup is in reverse order, custom handlers take precedence over built-in handlers. This means you can override any built-in handler by adding your own for the same tag — a simple but effective override mechanism.

## Text Processing Pipeline

Every text node passes through a multi-stage pipeline:

```mermaid
flowchart LR
    RAW["Raw HTML text\nfrom DOM tree"] --> ESCAPE1["html_escape::decode_html_entities()"]
    ESCAPE1 --> ESCAPE2["escape_if_needed()\nmarkdown syntax chars"]
    ESCAPE2 --> COMPRESS["compress_whitespace()\nruns → single space"]
    COMPRESS --> TRIM["trim leading/trailing\nbased on context"]
    TRIM --> BUFFER["Push to buffer"]
```

The `escape_if_needed()` function handles Markdown syntax conflicts:

```rust
// dom_walker.rs:236-289
fn escape_if_needed(text: Cow<str>) -> Cow<'_, str> {
    // First char triggers: = ~ > - + # 0-9
    // Any char triggers: \ * _ ` [ ]
    // Then escape specific chars with backslash
}
```

## Security/Safety Model

The `scripting_enabled` flag controls how `<noscript>` content is handled:

- `scripting_enabled: true` (default) — `<noscript>` content is treated as raw text (not parsed as DOM)
- `scripting_enabled: false` — `<noscript>` content is parsed as normal DOM elements

This is passed through to `html5ever`'s `TreeBuilderOpts`:

```rust
// lib.rs:106-110
ParseOpts {
    tree_builder: TreeBuilderOpts {
        scripting_enabled: self.scripting_enabled,
        ..Default::default()
    },
}
```

## What to Read Next

- [DOM Walker](02-dom-walker.md) for the traversal algorithm and text escaping
- [Element Handlers](03-element-handlers.md) for each handler's conversion logic
- [Options](04-options-config.md) for all configuration options
