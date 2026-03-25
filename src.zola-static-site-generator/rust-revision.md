# Building a Zola-like SSG in Rust

## Table of Contents

1. [Architecture Overview](#architecture-overview)
2. [Crate Recommendations](#crate-recommendations)
3. [Core Components](#core-components)
4. [Implementation Details](#implementation-details)
5. [Performance Considerations](#performance-considerations)
6. [Example Implementation](#example-implementation)

---

## Architecture Overview

### System Design

```
┌─────────────────────────────────────────────────────────────────┐
│                         CLI Interface                            │
│                    (clap + subcommands)                          │
└────────────────────────────┬────────────────────────────────────┘
                             │
         ┌───────────────────┼───────────────────┐
         │                   │                   │
         ▼                   ▼                   ▼
┌─────────────────┐ ┌─────────────────┐ ┌─────────────────┐
│   init cmd      │ │   build cmd     │ │   serve cmd     │
│   (scaffold)    │ │   (compile)     │ │   (dev server)  │
└─────────────────┘ └────────┬────────┘ └────────┬────────┘
                             │                   │
                             ▼                   ▼
                    ┌─────────────────────────────────┐
                    │      Site Builder Core          │
                    │  ┌─────────────────────────┐    │
                    │  │   Content Loader        │    │
                    │  │   - Front matter parse  │    │
                    │  │   - Markdown processing │    │
                    │  └───────────┬─────────────┘    │
                    │              │                  │
                    │  ┌───────────▼─────────────┐    │
                    │  │   Template Engine       │    │
                    │  │   - Tera/Askama         │    │
                    │  │   - Macro system        │    │
                    │  └───────────┬─────────────┘    │
                    │              │                  │
                    │  ┌───────────▼─────────────┐    │
                    │  │   Output Generator      │    │
                    │  │   - HTML rendering      │    │
                    │  │   - Asset processing    │    │
                    │  └─────────────────────────┘    │
                    └─────────────────────────────────┘
```

### Data Flow

```
Content Files ──▶ Parser ──▶ IR ──▶ Renderer ──▶ HTML
     │              │        │         │
     │              │        │         └──▶ Templates
     │              │        │
     │              │        └──▶ Type-safe intermediate repr
     │              │
     │              └──▶ TOML/YAML front matter
     │
Static Assets ──────────────────────────▶ Copy/Process
```

---

## Crate Recommendations

### Core Dependencies

```toml
[dependencies]
# CLI
clap = { version = "4", features = ["derive"] }

# Serialization
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
toml = "0.8"
serde_yaml = "0.9"

# Markdown
pulldown-cmark = "0.9"
syntect = "5"           # Syntax highlighting

# Templating
tera = "1.19"           # Jinja2-like templates
# OR
askama = "0.12"         # Compile-time templates

# Filesystem
walkdir = "2.4"
notify = "6"            # File watching
globset = "0.4"         # Glob patterns

# HTTP (for serve mode)
hyper = { version = "1", features = ["full"] }
tokio = { version = "1", features = ["full"] }
ws = "0.9"              # WebSocket for live reload

# Image processing
image = "0.24"
rayon = "1.8"           # Parallel processing

# Utilities
anyhow = "1.0"          # Error handling
thiserror = "1.0"       # Custom errors
chrono = "0.4"          # Date/time
regex = "1.10"          # Pattern matching
once_cell = "1.19"      # Lazy initialization
uuid = { version = "1", features = ["v4"] }

# CSS/Sass
grass = "0.13"          # Pure Rust Sass compiler
lightningcss = "1.0"    # CSS parser/minifier
```

### Complete Cargo.toml

```toml
[package]
name = "rust-ssg"
version = "0.1.0"
edition = "2021"
description = "A fast static site generator in Rust"
license = "MIT"

[dependencies]
# CLI
clap = { version = "4.4", features = ["derive"] }

# Serialization
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
toml = "0.8"
serde_yaml = "0.9"

# Markdown & Syntax Highlighting
pulldown-cmark = { version = "0.9", default-features = false }
syntect = "5.1"

# Templating
tera = "1.19"

# Filesystem
walkdir = "2.4"
notify = "6.1"
globset = "0.4"
pathdiff = "0.2"

# Async & HTTP
tokio = { version = "1.35", features = ["full"] }
hyper = { version = "1.1", features = ["full"] }
hyper-util = { version = "0.1", features = ["full"] }
http-body-util = "0.1"
ws = "0.9"

# Image Processing
image = { version = "0.24", default-features = false, features = ["png", "jpeg", "webp"] }
rayon = "1.8"

# Utilities
anyhow = "1.0"
thiserror = "1.0"
chrono = "0.4"
regex = "1.10"
once_cell = "1.19"
mime_guess = "2.0"
mime = "0.3"
percent-encoding = "2.3"

# CSS
grass = "0.13"

[profile.release]
lto = true
codegen-units = 1
strip = true
```

---

## Core Components

### Project Structure

```
rust-ssg/
├── Cargo.toml
├── src/
│   ├── main.rs              # CLI entry point
│   ├── lib.rs               # Library root
│   │
│   ├── cli/
│   │   ├── mod.rs           # CLI module
│   │   ├── commands.rs      # Command definitions
│   │   └── args.rs          # CLI arguments
│   │
│   ├── config/
│   │   ├── mod.rs           # Config loading
│   │   └── site.rs          # Site configuration
│   │
│   ├── content/
│   │   ├── mod.rs           # Content module
│   │   ├── page.rs          # Page struct
│   │   ├── section.rs       # Section struct
│   │   ├── front_matter.rs  # Front matter parsing
│   │   └── taxonomy.rs      # Taxonomy handling
│   │
│   ├── markdown/
│   │   ├── mod.rs           # Markdown module
│   │   ├── renderer.rs      # HTML renderer
│   │   ├── highlight.rs     # Syntax highlighting
│   │   └── shortcodes.rs    # Shortcode processing
│   │
│   ├── templates/
│   │   ├── mod.rs           # Template module
│   │   ├── engine.rs        # Template engine wrapper
│   │   └── functions.rs     # Template functions
│   │
│   ├── build/
│   │   ├── mod.rs           # Build module
│   │   ├── site.rs          # Site builder
│   │   └── output.rs        # Output generation
│   │
│   ├── serve/
│   │   ├── mod.rs           # Serve module
│   │   ├── server.rs        # HTTP server
│   │   └── watch.rs         # File watcher
│   │
│   └── error.rs             # Error types
│
└── tests/
    ├── integration/
    └── fixtures/
```

---

## Implementation Details

### Front Matter Parsing

```rust
// src/content/front_matter.rs
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PageFrontMatter {
    pub title: Option<String>,
    pub description: Option<String>,
    pub date: Option<chrono::DateTime<chrono::Utc>>,
    pub draft: bool,
    pub slug: Option<String>,
    pub path: Option<String>,
    pub template: Option<String>,
    pub weight: Option<usize>,
    pub taxonomies: HashMap<String, Vec<String>>,
    pub extra: serde_json::Map<String, serde_json::Value>,
}

impl PageFrontMatter {
    pub fn parse(content: &str) -> Result<(Self, &str), anyhow::Error> {
        // Split front matter from content
        let (front_matter, content) = split_front_matter(content)?;

        // Parse based on delimiter
        let matter = if front_matter.starts_with("+++") {
            toml::from_str(&front_matter.trim_matches('+'))?
        } else if front_matter.starts_with("---") {
            serde_yaml::from_str(&front_matter.trim_matches('-'))?
        } else {
            return Err(anyhow::anyhow!("Invalid front matter delimiter"));
        };

        Ok((matter, content))
    }
}

fn split_front_matter(content: &str) -> Result<(&str, &str), anyhow::Error> {
    let bytes = content.as_bytes();

    // Find TOML front matter (+++)
    if bytes.starts_with(b"+++") {
        if let Some(end) = content[3..].find("+++") {
            return Ok((&content[0..end + 3], &content[end + 6..]));
        }
    }

    // Find YAML front matter (---)
    if bytes.starts_with(b"---") {
        if let Some(end) = content[3..].find("---") {
            return Ok((&content[0..end + 3], &content[end + 6..]));
        }
    }

    // No front matter
    Ok(("", content))
}
```

### Page Struct

```rust
// src/content/page.rs
use std::path::{Path, PathBuf};

use crate::content::front_matter::PageFrontMatter;
use crate::config::Config;

#[derive(Debug, Clone)]
pub struct Page {
    pub file_path: PathBuf,
    pub relative_path: String,
    pub front_matter: PageFrontMatter,
    pub raw_content: String,
    pub content: String,  // Rendered HTML
    pub slug: String,
    pub path: String,
    pub permalink: String,
    pub summary: Option<String>,
    pub word_count: usize,
    pub reading_time: usize,
    pub toc: Vec<Heading>,
    pub assets: Vec<PathBuf>,
}

#[derive(Debug, Clone)]
pub struct Heading {
    pub level: u8,
    pub title: String,
    pub id: String,
    pub permalink: String,
    pub children: Vec<Heading>,
}

impl Page {
    pub fn from_file(path: &Path, config: &Config) -> Result<Self, anyhow::Error> {
        let content = std::fs::read_to_string(path)?;
        let (front_matter, body) = PageFrontMatter::parse(&content)?;

        // Generate slug from filename if not specified
        let slug = front_matter.slug.clone()
            .unwrap_or_else(|| generate_slug(path.file_stem().unwrap().to_str().unwrap()));

        // Generate path
        let path = generate_path(&slug, &front_matter);

        // Generate permalink
        let permalink = format!("{}{}", config.base_url, path);

        // Count words and estimate reading time
        let word_count = body.split_whitespace().count();
        let reading_time = (word_count as f32 / 200.0).ceil() as usize;

        // Extract summary (content before <!-- more -->)
        let summary = body.split("<!-- more -->").next().map(String::from);

        Ok(Page {
            file_path: path.to_path_buf(),
            relative_path: path.to_string(),
            front_matter,
            raw_content: body.to_string(),
            content: String::new(),  // Will be rendered later
            slug,
            path,
            permalink,
            summary,
            word_count,
            reading_time,
            toc: Vec::new(),
            assets: Vec::new(),
        })
    }

    pub fn render(&mut self, config: &Config) -> Result<(), anyhow::Error> {
        // Render markdown to HTML
        let rendered = crate::markdown::render(&self.raw_content, config)?;
        self.content = rendered.html;
        self.toc = rendered.toc;
        Ok(())
    }
}
```

### Markdown Rendering

```rust
// src/markdown/renderer.rs
use pulldown_cmark::{html, Parser, Options, Event, Tag, HeadingLevel};
use crate::content::Heading;

pub struct RenderResult {
    pub html: String,
    pub toc: Vec<Heading>,
}

pub fn render(content: &str, config: &Config) -> Result<RenderResult, anyhow::Error> {
    let mut options = Options::empty();
    options.insert(Options::ENABLE_TABLES);
    options.insert(Options::ENABLE_FOOTNOTES);
    options.insert(Options::ENABLE_STRIKETHROUGH);
    options.insert(Options::ENABLE_TASKLISTS);

    let parser = Parser::new_ext(content, options);

    // Collect headings for TOC
    let mut toc = Vec::new();
    let mut events: Vec<Event> = Vec::new();

    for event in parser {
        match event {
            Event::Start(Tag::Heading(level, id, _)) => {
                let heading = Heading {
                    level: level as u8,
                    title: String::new(),
                    id: id.unwrap_or_default(),
                    permalink: String::new(),
                    children: Vec::new(),
                };
                toc.push(heading);
                events.push(Event::Start(Tag::Heading(level, id, None)));
            }
            Event::Text(text) => {
                // Add text to current heading
                if let Some(heading) = toc.last_mut() {
                    heading.title.push_str(&text);
                }
                events.push(Event::Text(text));
            }
            _ => events.push(event),
        }
    }

    let html = html::render_html(&events, &html::Options::default());

    Ok(RenderResult { html, toc })
}
```

### Syntax Highlighting

```rust
// src/markdown/highlight.rs
use syntect::easy::HighlightLines;
use syntect::highlighting::{ThemeSet, Style};
use syntect::parsing::SyntaxSet;
use syntect::html::{styled_line_to_highlighted_html, IncludeBackground};

pub struct Highlighter {
    syntax_set: SyntaxSet,
    theme_set: ThemeSet,
}

impl Highlighter {
    pub fn new() -> Self {
        Highlighter {
            syntax_set: SyntaxSet::load_defaults_newlines(),
            theme_set: ThemeSet::load_defaults(),
        }
    }

    pub fn highlight(&self, code: &str, language: &str) -> String {
        let syntax = self.syntax_set
            .find_syntax_by_token(language)
            .unwrap_or_else(|| self.syntax_set.find_syntax_plain_text());

        let theme = self.theme_set.themes.get("base16-ocean.dark")
            .unwrap_or(&self.theme_set.themes.values().next().unwrap());

        let mut highlighter = HighlightLines::new(syntax, theme);
        let regions = highlighter.highlight_line(code, &self.syntax_set).unwrap();

        styled_line_to_highlighted_html(&regions, IncludeBackground::No).unwrap()
    }
}
```

### Template Engine

```rust
// src/templates/engine.rs
use tera::{Tera, Context, to_value};
use std::collections::HashMap;

pub struct TemplateEngine {
    tera: Tera,
}

impl TemplateEngine {
    pub fn new(templates_dir: &std::path::Path) -> Result<Self, anyhow::Error> {
        let mut tera = Tera::default();

        // Load templates
        let pattern = format!("{}/**/*.html", templates_dir.display());
        tera.auto_reload_on_patch(&pattern)?;

        // Register filters
        tera.register_filter("date", filters::date_filter);
        tera.register_filter("markdown", filters::markdown_filter);

        // Register global functions
        tera.register_function("get_page", functions::GetPage::new());
        tera.register_function("get_section", functions::GetSection::new());
        tera.register_function("get_url", functions::GetUrl::new());

        Ok(TemplateEngine { tera })
    }

    pub fn render_page(
        &self,
        template: &str,
        page: &crate::content::Page,
        config: &crate::config::Config,
    ) -> Result<String, anyhow::Error> {
        let mut context = Context::new();
        context.insert("page", &page);
        context.insert("config", &config);

        Ok(self.tera.render(template, &context)?)
    }
}

// Custom filter example
mod filters {
    use tera::{to_value, Filter, Result, Value};
    use std::collections::HashMap;
    use chrono::{DateTime, Utc};

    pub fn date_filter(value: &Value, args: &HashMap<String, Value>) -> Result<Value> {
        let date = value.as_str().ok_or("date filter expects a string")?;
        let format = args.get("format")
            .and_then(|v| v.as_str())
            .unwrap_or("%Y-%m-%d");

        let dt: DateTime<Utc> = date.parse().map_err(|e| format!("Invalid date: {}", e))?;
        Ok(to_value(dt.format(format).to_string()).unwrap())
    }
}
```

### Site Builder

```rust
// src/build/site.rs
use std::path::{Path, PathBuf};
use std::sync::Arc;
use rayon::prelude::*;

use crate::config::Config;
use crate::content::{Page, Section, Library};
use crate::templates::TemplateEngine;

pub struct SiteBuilder {
    config: Config,
    templates: TemplateEngine,
    library: Library,
}

impl SiteBuilder {
    pub fn new(config: Config, templates: TemplateEngine) -> Self {
        SiteBuilder {
            config,
            templates,
            library: Library::default(),
        }
    }

    pub fn load_content(&mut self, content_dir: &Path) -> Result<(), anyhow::Error> {
        // Walk directory and load all pages/sections
        for entry in walkdir::WalkDir::new(content_dir) {
            let entry = entry?;
            let path = entry.path();

            if path.extension().map_or(false, |e| e == "md") {
                if path.file_name().map_or(false, |n| n == "_index.md") {
                    let section = Section::from_file(path, &self.config)?;
                    self.library.add_section(section);
                } else {
                    let page = Page::from_file(path, &self.config)?;
                    self.library.add_page(page);
                }
            }
        }
        Ok(())
    }

    pub fn build(&self, output_dir: &Path) -> Result<(), anyhow::Error> {
        // Clean output directory
        if output_dir.exists() {
            std::fs::remove_dir_all(output_dir)?;
        }
        std::fs::create_dir_all(output_dir)?;

        // Render pages in parallel
        let results: Vec<Result<(), anyhow::Error>> = self.library.pages()
            .par_iter()
            .map(|page| self.render_page(page, output_dir))
            .collect();

        // Check for errors
        for result in results {
            result?;
        }

        // Copy static assets
        self.copy_static_assets(output_dir)?;

        // Generate feeds, sitemap, etc.
        self.generate_feeds(output_dir)?;
        self.generate_sitemap(output_dir)?;

        Ok(())
    }

    fn render_page(&self, page: &Page, output_dir: &Path) -> Result<(), anyhow::Error> {
        let template = page.front_matter.template.as_deref().unwrap_or("page.html");
        let html = self.templates.render_page(template, page, &self.config)?;

        let output_path = output_dir.join(&page.path[1..]).join("index.html");
        if let Some(parent) = output_path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(output_path, html)?;

        Ok(())
    }
}
```

### File Watcher (Serve Mode)

```rust
// src/serve/watch.rs
use notify::{Event, RecommendedWatcher, RecursiveMode, Watcher};
use std::path::Path;
use std::sync::mpsc::channel;
use std::time::Duration;

pub struct FileWatcher {
    watcher: RecommendedWatcher,
}

impl FileWatcher {
    pub fn new<F>(callback: F) -> Result<Self, anyhow::Error>
    where
        F: Fn(Event) + Send + 'static,
    {
        let (tx, rx) = channel();

        let watcher = RecommendedWatcher::new(move |res| {
            if let Ok(event) = res {
                let _ = tx.send(event);
            }
        })?;

        // Spawn thread to handle events
        std::thread::spawn(move || {
            while let Ok(event) = rx.recv() {
                // Debounce events
                std::thread::sleep(Duration::from_millis(100));
                callback(event);
            }
        });

        Ok(FileWatcher { watcher })
    }

    pub fn watch(&mut self, path: &Path) -> Result<(), anyhow::Error> {
        self.watcher.watch(path, RecursiveMode::Recursive)?;
        Ok(())
    }
}
```

---

## Performance Considerations

### 1. Parallel Processing

```rust
use rayon::prelude::*;

// Parallel page rendering
let results: Vec<_> = pages.par_iter()
    .map(|page| render_page(page))
    .collect();

// Parallel image processing
let images: Vec<_> = image_paths.par_iter()
    .map(|path| process_image(path))
    .collect();
```

### 2. Incremental Builds

```rust
use std::collections::HashMap;
use std::fs::FileTime;

pub struct BuildCache {
    file_times: HashMap<PathBuf, FileTime>,
    rendered_pages: HashMap<PathBuf, String>,
}

impl BuildCache {
    pub fn needs_rebuild(&self, path: &Path) -> bool {
        let current_time = FileTime::from_last_modification_time(&std::fs::metadata(path).unwrap());
        match self.file_times.get(path) {
            Some(cached) => cached != &current_time,
            None => true,
        }
    }
}
```

### 3. Memory-Efficient Streaming

```rust
use std::io::Write;

// Stream large files instead of loading entirely into memory
pub fn copy_large_file(src: &Path, dst: &Path) -> Result<(), anyhow::Error> {
    let mut src_file = std::fs::File::open(src)?;
    let mut dst_file = std::fs::File::create(dst)?;
    std::io::copy(&mut src_file, &mut dst_file)?;
    Ok(())
}
```

### 4. Template Caching

```rust
use once_cell::sync::Lazy;
use std::sync::Arc;

static TEMPLATE_CACHE: Lazy<Arc<Tera>> = Lazy::new(|| {
    Arc::new(Tera::new("templates/**/*.html").unwrap())
});
```

### 5. Efficient String Handling

```rust
// Use Cow for borrowed/owned string optimization
use std::borrow::Cow;

pub fn process_content<'a>(content: &'a str) -> Cow<'a, str> {
    if content.contains("{{") {
        Cow::Owned(process_template(content))
    } else {
        Cow::Borrowed(content)
    }
}
```

---

## Example Implementation

### Minimal SSG Binary

```rust
// src/main.rs
use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "rust-ssg")]
#[command(about = "A fast static site generator")]
struct Cli {
    #[command(subcommand)]
    command: Commands,

    #[arg(short, long, default_value = ".")]
    root: PathBuf,
}

#[derive(Subcommand)]
enum Commands {
    /// Create a new site
    Init {
        #[arg(default_value = ".")]
        name: String,
    },
    /// Build the site
    Build {
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
    /// Serve the site with live reload
    Serve {
        #[arg(short, long, default_value_t = 1111)]
        port: u16,
    },
}

fn main() -> Result<(), anyhow::Error> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Init { name } => {
            cmd_init(&name)?;
        }
        Commands::Build { output } => {
            cmd_build(&cli.root, output)?;
        }
        Commands::Serve { port } => {
            cmd_serve(&cli.root, port)?;
        }
    }

    Ok(())
}

fn cmd_init(name: &str) -> Result<(), anyhow::Error> {
    println!("Creating new site: {}", name);
    std::fs::create_dir_all(format!("{}/content", name))?;
    std::fs::create_dir_all(format!("{}/templates", name))?;
    std::fs::create_dir_all(format!("{}/static", name))?;

    // Create config.toml
    let config = r#"
base_url = "https://example.com"
title = "My Site"
description = "A site built with rust-ssg"
"#;
    std::fs::write(format!("{}/config.toml", name), config)?;

    // Create _index.md
    let index = r#"+++
title = "Home"
+++

Welcome to your new site!
"#;
    std::fs::write(format!("{}/content/_index.md", name), index)?;

    println!("Site created successfully!");
    Ok(())
}

fn cmd_build(root: &PathBuf, output: Option<PathBuf>) -> Result<(), anyhow::Error> {
    // Load config
    let config = crate::config::Config::from_file(&root.join("config.toml"))?;

    // Initialize templates
    let templates = crate::templates::TemplateEngine::new(&root.join("templates"))?;

    // Build site
    let mut builder = crate::build::SiteBuilder::new(config, templates);
    builder.load_content(&root.join("content"))?;

    let output_dir = output.unwrap_or_else(|| root.join("public"));
    builder.build(&output_dir)?;

    println!("Build complete! Output in {:?}", output_dir);
    Ok(())
}

fn cmd_serve(root: &PathBuf, port: u16) -> Result<(), anyhow::Error> {
    use tokio::runtime::Runtime;

    let rt = Runtime::new()?;
    rt.block_on(async {
        crate::serve::run_server(root, port).await?;
        Ok::<(), anyhow::Error>(())
    })?;

    Ok(())
}
```

### Build Optimization

```rust
// Cargo.toml
[profile.release]
lto = true           # Link-time optimization
codegen-units = 1    # Single codegen unit
strip = true         # Strip debug symbols

[profile.dev]
debug = 0            # Faster builds during development

# Optimize dependencies even in dev
[profile.dev.package."*"]
opt-level = 3
```

---

## Testing Strategy

### Unit Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_toml_front_matter() {
        let content = r#"
+++
title = "Test"
+++
Body content
"#;
        let (matter, body) = PageFrontMatter::parse(content).unwrap();
        assert_eq!(matter.title, Some("Test".to_string()));
        assert_eq!(body.trim(), "Body content");
    }

    #[test]
    fn test_generate_slug() {
        assert_eq!(generate_slug("Hello World"), "hello-world");
        assert_eq!(generate_slug("Rust & SSG"), "rust-ssg");
    }
}
```

### Integration Tests

```rust
// tests/integration/build_test.rs
#[test]
fn test_full_build() {
    let temp_dir = tempfile::tempdir().unwrap();
    let site_path = temp_dir.path();

    // Create test site structure
    create_test_site(site_path);

    // Build site
    let output = site_path.join("public");
    cmd_build(&site_path.to_path_buf(), Some(output.clone())).unwrap();

    // Verify output
    assert!(output.join("index.html").exists());
}
```

---

## Next Steps

1. **Implement all template functions** (`get_page`, `get_section`, etc.)
2. **Add taxonomy support** (tags, categories)
3. **Implement pagination**
4. **Add search index generation**
5. **Implement Sass compilation**
6. **Add image processing pipeline**
7. **Create live reload WebSocket server**
8. **Add link checking**
