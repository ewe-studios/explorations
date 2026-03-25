# Zola Architecture Deep Dive

## Table of Contents

1. [Overall Architecture](#overall-architecture)
2. [Content Loading](#content-loading)
3. [Markdown Processing](#markdown-processing)
4. [Template Rendering](#template-rendering)
5. [Output Generation](#output-generation)
6. [Live Reload System](#live-reload-system)
7. [Component Interactions](#component-interactions)

---

## Overall Architecture

Zola follows a modular component architecture where each component is a separate Rust crate:

```
                    ┌─────────────────────────────────────┐
                    │           Zola Binary                │
                    │         (src/main.rs)                │
                    └─────────────────┬───────────────────┘
                                      │
                    ┌─────────────────┼───────────────────┐
                    │                 │                   │
                    ▼                 ▼                   ▼
           ┌─────────────┐   ┌─────────────┐    ┌─────────────┐
           │   init      │   │    build    │    │    serve    │
           │  command    │   │   command   │    │   command   │
           └─────────────┘   └──────┬──────┘    └──────┬──────┘
                                    │                  │
                                    ▼                  ▼
                          ┌─────────────────────────────────┐
                          │      Site (components/site)      │
                          │  - load()                        │
                          │  - build()                       │
                          │  - render_*()                    │
                          └─────────────────────────────────┘
```

### Component Dependency Graph

```
┌──────────────────────────────────────────────────────────────────┐
│                         zola (main)                               │
└──────────────────────────────────────────────────────────────────┘
    │         │         │         │         │
    ▼         ▼         ▼         ▼         ▼
┌───────┐ ┌───────┐ ┌───────┐ ┌───────┐ ┌───────┐
│ site  │ │console│ │ errors│ │ utils │ │ libs  │
└───┬───┘ └───────┘ └───────┘ └───┬───┘ └───┬───┘
    │                              │         │
    ▼                              ▼         ▼
┌───────────┐              ┌───────────┐ ┌───────┐
│ templates │              │  config   │ │markdown│
└─────┬─────┘              └─────┬─────┘ └───┬───┘
      │                          │           │
      ▼                          ▼           ▼
┌───────────┐              ┌─────────────────────┐
│  content  │              │   imageproc, etc.   │
└───────────┘              └─────────────────────┘
```

---

## Content Loading

### Content Discovery Process

Zola walks the `content/` directory to discover sections and pages:

```rust
// From components/site/src/lib.rs
let mut dir_walker = WalkDir::new(self.base_path.join("content"))
    .follow_links(true)
    .into_iter();

// Process sections (_index.md) before pages
if path.is_dir() {
    // Find all _index.{lang}.md files
    let section = Section::from_file(index_file.path(), &self.config, &self.base_path)?;
    self.add_section(section, false)?;
} else {
    let page = Page::from_file(path, &self.config, &self.base_path)?;
    pages.push(page);
}
```

### Section Processing

```
content/
├── _index.md              → Root section
├── blog/
│   ├── _index.md          → Blog section
│   └── post.md            → Blog post
└── docs/
    ├── _index.md          → Docs section
    └── guide.md           → Guide page
```

**Section Structure (`components/content/src/section.rs`):**

```rust
pub struct Section {
    pub file: FileInfo,           // File metadata
    pub meta: SectionFrontMatter, // Front matter
    pub path: String,             // URL path
    pub components: Vec<String>,  // Path components
    pub permalink: String,        // Full URL
    pub raw_content: String,      // Markdown content
    pub content: String,          // Rendered HTML
    pub assets: Vec<PathBuf>,     // Co-located assets
    pub pages: Vec<PathBuf>,      // Direct child pages
    pub subsections: Vec<PathBuf>, // Child sections
    pub toc: Vec<Heading>,        // Table of contents
    pub lang: String,             // Language code
}
```

### Page Processing

**Page Structure (`components/content/src/page.rs`):**

```rust
pub struct Page {
    pub file: FileInfo,
    pub meta: PageFrontMatter,
    pub ancestors: Vec<String>,   // Parent sections
    pub raw_content: String,
    pub assets: Vec<PathBuf>,
    pub content: String,
    pub slug: String,             // URL slug
    pub path: String,
    pub components: Vec<String>,
    pub permalink: String,
    pub summary: Option<String>,  // <!-- more --> cutoff
    pub lower: Option<PathBuf>,   // Previous page (sorted)
    pub higher: Option<PathBuf>,  // Next page (sorted)
    pub toc: Vec<Heading>,
    pub word_count: Option<usize>,
    pub reading_time: Option<usize>,
    pub lang: String,
    pub translations: Vec<PathBuf>,
}
```

### Front Matter Parsing

```rust
// From components/content/src/front_matter/page.rs
pub struct PageFrontMatter {
    pub title: Option<String>,
    pub description: Option<String>,
    pub date: Option<String>,
    pub datetime: Option<OffsetDateTime>,
    pub draft: bool,
    pub slug: Option<String>,
    pub path: Option<String>,
    pub taxonomies: HashMap<String, Vec<String>>,
    pub weight: Option<usize>,
    pub aliases: Vec<String>,
    pub template: Option<String>,
    pub extra: Map<String, Value>,
}
```

---

## Markdown Processing

### Processing Pipeline

```
┌─────────────────────────────────────────────────────────────┐
│                  MARKDOWN PROCESSING                         │
│                                                              │
│  Markdown Text                                               │
│       │                                                      │
│       ▼                                                      │
│  ┌─────────────────┐                                        │
│  │ pulldown-cmark  │  →  AST (Abstract Syntax Tree)         │
│  │    Parser       │                                        │
│  └────────┬────────┘                                        │
│           │                                                  │
│           ▼                                                  │
│  ┌─────────────────┐                                        │
│  │  Code Block     │  →  Syntax highlighting (syntect)      │
│  │  Handler        │                                        │
│  └────────┬────────┘                                        │
│           │                                                  │
│           ▼                                                  │
│  ┌─────────────────┐                                        │
│  │   Shortcode     │  →  Template rendering                 │
│  │   Parser        │                                        │
│  └────────┬────────┘                                        │
│           │                                                  │
│           ▼                                                  │
│  ┌─────────────────┐                                        │
│  │  Link Resolver  │  →  Internal link resolution           │
│  └────────┬────────┘                                        │
│           │                                                  │
│           ▼                                                  │
│  ┌─────────────────┐                                        │
│  │  Table of       │  →  Heading extraction                 │
│  │  Contents       │                                        │
│  └────────┬────────┘                                        │
│           │                                                  │
│           ▼                                                  │
│       HTML Output                                            │
└─────────────────────────────────────────────────────────────┘
```

### Syntax Highlighting

Zola uses **syntect** for syntax highlighting:

```rust
// From components/markdown/src/codeblock/highlight.rs
pub(crate) struct ClassHighlighter<'config> {
    syntax_set: &'config SyntaxSet,
    parse_state: ParseState,
    scope_stack: ScopeStack,
}

impl<'config> ClassHighlighter<'config> {
    pub fn highlight_line(&mut self, line: &str) -> String {
        let parsed_line = self.parse_state.parse_line(line, self.syntax_set);
        // Generate class-based spans
        let (formatted, _) = line_tokens_to_classed_spans(...);
        formatted
    }
}
```

**Output format:**

```html
<pre style="background:#2b303b;">
  <span class="source rust">
    <span class="keyword.control.rust">fn</span>
    <span class="entity.name.function.rust">main</span>() {
      <span class="constant.language.rust">println!</span>(<span class="string.quoted.double.rust">"Hello"</span>);
    }
  </span>
</pre>
```

### Shortcode Processing

Shortcodes are template-based components:

```rust
// Shortcode definition extraction
pub fn get_shortcodes(tera: &Tera) -> HashMap<String, ShortcodeDefinition> {
    // Find all shortcodes/*.html templates
    // Extract required/optional arguments
}
```

**Usage in markdown:**

```markdown
{{ youtube(id="dQw4w9WgXcQ", class="video") }}

{{ figure(
    src="image.png",
    alt="Description",
    caption="A nice image"
) }}
```

---

## Template Rendering

### Tera Integration

Zola uses **Tera**, a Jinja2-inspired template engine:

```rust
// From components/templates/src/lib.rs
pub static ZOLA_TERA: Lazy<Tera> = Lazy::new(|| {
    let mut tera = Tera::default();
    tera.add_raw_templates(vec![
        ("__zola_builtins/404.html", include_str!("builtins/404.html")),
        ("__zola_builtins/atom.xml", include_str!("builtins/atom.xml")),
        ("__zola_builtins/rss.xml", include_str!("builtins/rss.xml")),
        ("__zola_builtins/sitemap.xml", include_str!("builtins/sitemap.xml")),
    ])
    .unwrap();
    tera.register_filter("base64_encode", filters::base64_encode);
    tera.register_filter("base64_decode", filters::base64_decode);
    tera.register_filter("regex_replace", filters::RegexReplaceFilter::new());
    tera
});
```

### Template Loading with Themes

```rust
pub fn load_tera(path: &Path, config: &Config) -> Result<Tera> {
    // Load project templates
    let mut tera = Tera::parse(&tpl_glob)?;

    // Extend with theme templates if theme is set
    if let Some(ref theme) = config.theme {
        let theme_tpl_glob = format!(
            "{}/themes/{}/templates/**/*.{{*ml,md}}",
            path.to_string_lossy(),
            theme
        );
        let mut tera_theme = Tera::parse(&theme_tpl_glob)?;
        rewrite_theme_paths(&mut tera_theme, theme);
        tera.extend(&tera_theme)?;
    }

    // Extend with built-in templates
    tera.extend(&ZOLA_TERA)?;
    tera.build_inheritance_chains()?;

    Ok(tera)
}
```

### Template Inheritance

```
base.html
    │
    ├─→ index.html (homepage)
    │
    ├─→ section.html (section listing)
    │   │
    │   └─→ taxonomy terms
    │
    └─→ page.html (individual page)
        │
        └─→ extends base, imports macros
```

**Example:**

```html+tera
{# page.html #}
{% extends "base.html" %}
{% import "post_macros.html" as post_macros %}

{% block title %}{{ config.title }} - {{ page.title }}{% endblock %}

{% block content %}
<article class="post">
    {{ post_macros::title(page=page) }}
    <div class="content">
        {{ page.content | safe }}
    </div>
</article>
{% endblock %}
```

### Global Functions

Zola provides powerful global functions in templates:

| Function | Description |
|----------|-------------|
| `get_page(path)` | Load a page by path |
| `get_section(path)` | Load a section |
| `get_taxonomy(kind)` | Get taxonomy terms |
| `get_url(path)` | Generate absolute URL |
| `load_data(path/url)` | Load external data |
| `get_image(path)` | Process images |
| `trans(key)` | Translation lookup |

---

## Output Generation

### Build Process

```rust
// From components/site/src/lib.rs - simplified
pub fn build(&self) -> Result<()> {
    // 1. Clean output directory
    clean_site_output_folder(&self.output_path)?;

    // 2. Copy static assets
    copy_directory(&self.static_path, &self.output_path)?;

    // 3. Copy co-located assets
    for page in self.library.pages() {
        copy_assets(&page.assets, &self.output_path)?;
    }

    // 4. Compile Sass
    if self.config.compile_sass {
        compile_sass(&self.sass_path, &self.output_path)?;
    }

    // 5. Render pages in parallel
    self.library.par_render_pages(&self.tera, &self.config)?;

    // 6. Generate feeds
    if self.config.generate_feeds {
        self.render_feeds()?;
    }

    // 7. Generate sitemap
    if self.config.generate_sitemap {
        self.render_sitemap()?;
    }

    // 8. Generate search index
    if self.config.build_search_index {
        self.render_search_index()?;
    }

    Ok(())
}
```

### Parallel Rendering

```rust
use libs::rayon::prelude::*;

// Pages are rendered in parallel using Rayon
let results: Vec<Result<_>> = pages
    .par_iter()
    .map(|page| {
        let html = page.render_html(&self.tera, &self.config, &self.library)?;
        write_file(&output_path, &html)?;
        Ok(())
    })
    .collect();
```

### Output Structure

```
public/
├── index.html              # Homepage
├── blog/
│   ├── index.html          # Blog section
│   ├── page/1/             # Pagination
│   └── my-post/
│       └── index.html      # Blog post
├── tags/
│   ├── index.html          # Tags list
│   └── rust/
│       └── index.html      # Tag page
├── sitemap.xml             # SEO sitemap
├── atom.xml                # RSS feed
├── search_index.en.json    # Search index
└── static/                 # Copied assets
    ├── css/
    └── images/
```

---

## Live Reload System

### Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                    LIVE RELOAD SYSTEM                        │
│                                                              │
│  ┌─────────────┐    ┌─────────────┐    ┌─────────────┐      │
│  │  File       │    │  WebSocket  │    │  Browser    │      │
│  │  Watcher    │───▶│   Server    │───▶│   Client    │      │
│  │ (notify)    │    │   (ws)      │    │ (injected)  │      │
│  └─────────────┘    └─────────────┘    └─────────────┘      │
│         │                  │                  │              │
│         │                  │                  │              │
│         ▼                  ▼                  ▼              │
│  Detect changes     Send reload        Refresh page         │
│  (debounced)        signal              + scroll pos         │
└─────────────────────────────────────────────────────────────┘
```

### File Watching

```rust
// From src/cmd/serve.rs
let mut debouncer = new_debouncer(
    Duration::from_millis(100),
    move |res: Result<Event>| {
        let events = res.unwrap().events;
        // Filter and process events
        let (path, event_kind) = filter_events(events);

        match event_kind {
            ChangeKind::Content => {
                // Rebuild affected pages
                site.reload()?;
            }
            ChangeKind::Templates => {
                // Reload templates and rebuild
                site.reload_templates()?;
            }
            ChangeKind::Static => {
                // Copy static files
                copy_file(&path, &output_path)?;
            }
        }

        // Notify browser via WebSocket
        ws_sender.send(Message::text("reload"))?;
    }
)?;
```

### WebSocket Communication

```javascript
// Injected livereload.js (embedded in binary)
(function() {
    var ws = new WebSocket('ws://localhost:1111/livereload');
    ws.onmessage = function(event) {
        if (event.data === 'reload') {
            window.location.reload();
        }
    };
})();
```

### Serve Mode Build Modes

```rust
pub enum BuildMode {
    Disk,      // Full build to filesystem
    Memory,    // In-memory for content only
    Both,      // Both (fast rebuilds)
}
```

In `fast` mode, only the minimum is rebuilt:
- Changed page re-rendered
- Templates cached
- Sass not recompiled unless changed

---

## Component Interactions

### Data Flow Diagram

```
┌──────────────────────────────────────────────────────────────┐
│                        CONFIG LOADING                         │
│  config.toml ──▶ Config::from_file() ──▶ Config struct       │
└──────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌──────────────────────────────────────────────────────────────┐
│                        CONTENT LOADING                        │
│  WalkDir ──▶ Section::from_file() / Page::from_file()        │
│            ──▶ Library (all content)                          │
└──────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌──────────────────────────────────────────────────────────────┐
│                     MARKDOWN RENDERING                        │
│  raw_content ──▶ render_content() ──▶ HTML + TOC + Links     │
└──────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌──────────────────────────────────────────────────────────────┐
│                     TEMPLATE RENDERING                        │
│  page.content + templates ──▶ tera.render() ──▶ Final HTML   │
└──────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌──────────────────────────────────────────────────────────────┐
│                      OUTPUT WRITING                           │
│  HTML ──▶ write_file(public/path/index.html)                 │
└──────────────────────────────────────────────────────────────┘
```

### Key Struct Relationships

```
Site
├── Config
├── Tera
├── Library
│   ├── Vec<Section>
│   └── HashMap<PathBuf, Page>
├── taxonomies: Vec<Taxonomy>
│   └── Vec<TaxonomyTerm>
│       └── Vec<Page>
└── permalinks: HashMap<String, String>

Page
├── FileInfo
├── PageFrontMatter
├── content: String (HTML)
├── toc: Vec<Heading>
└── ancestors: Vec<String>
```

### Error Handling

All components use a unified error type:

```rust
// components/errors/src/lib.rs
pub type Result<T> = std::result::Result<T, Error>;

pub struct Error {
    kind: ErrorKind,
    message: String,
    source: Option<Box<dyn std::error::Error>>,
}
```

Errors propagate with context:

```rust
section.render_markdown(...)
    .with_context(|| format!("Failed to render section '{}'", section.path))?;
```

---

## Performance Optimizations

### 1. Parallel Processing

```rust
use rayon::prelude::*;

// Parallel page rendering
pages.par_iter().map(|page| page.render()).collect();
```

### 2. Incremental Builds

```rust
// In serve mode, only rebuild what changed
if event.path.ends_with(".md") {
    site.reload_section(&event.path)?;
} else if event.path.ends_with(".html") {
    site.reload_templates()?;
}
```

### 3. Caching

- Template compilation cached
- Syntax highlighting themes cached
- Regex patterns cached (e.g., in filters)
- Image processing results cached

### 4. Memory Efficiency

```rust
// Shared content storage in serve mode
pub static SITE_CONTENT: Lazy<Arc<RwLock<HashMap<RelativePathBuf, String>>>> =
    Lazy::new(|| Arc::new(RwLock::new(HashMap::new())));
```

### 5. Release Build Optimizations

```toml
[profile.release]
lto = true           # Link Time Optimization
codegen-units = 1    # Single codegen unit for better optimization
strip = true         # Strip debug symbols
```
