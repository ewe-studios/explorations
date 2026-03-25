# Zola Static Site Generator - Comprehensive Exploration

**Source:** `/home/darkvoid/Boxxed/@formulas/src.rust/src.zola-static-site-generator/`

**Date:** 2026-03-26

---

## Table of Contents

1. [Overview](#overview)
2. [Project Structure](#project-structure)
3. [Architecture Summary](#architecture-summary)
4. [Key Components](#key-components)
5. [Themes Analyzed](#themes-analyzed)
6. [Related Documents](#related-documents)

---

## Overview

Zola is a **fast static site generator (SSG)** written in Rust. It's designed as a single binary with everything built-in, requiring no external dependencies. The project was created by Vincent Prouillet as a reaction against Hugo's Go template engine.

### Key Features

- **Single binary** - No dependencies to install
- **Syntax highlighting** - Built-in via syntect
- **Sass compilation** - Native support
- **Asset co-location** - Pages and assets together
- **Multilingual support** - i18n ready
- **Image processing** - Resize, crop, optimize
- **Themes** - Extensible theme system
- **Shortcodes** - Custom content components
- **Live reload** - Development server with hot reload
- **Search** - Client-side search without servers
- **RSS/Atom feeds** - Automatic generation
- **Sitemap** - SEO-friendly

### Performance Characteristics

- Uses **Rayon** for parallel processing
- **Incremental builds** in serve mode
- **LTO (Link Time Optimization)** enabled in release builds
- Zero-cost abstractions from Rust

---

## Project Structure

```
src.zola-static-site-generator/
├── zola/                          # Main Zola SSG source code
│   ├── src/                       # CLI entry point
│   │   ├── main.rs                # Application entry
│   │   ├── cli.rs                 # CLI argument parsing
│   │   └── cmd/                   # Commands (build, serve, init, check)
│   ├── components/                # Core library components
│   │   ├── config/                # Configuration parsing
│   │   ├── content/               # Page/Section/Taxonomy handling
│   │   ├── templates/             # Tera template integration
│   │   ├── markdown/              # Markdown processing
│   │   ├── site/                  # Site building logic
│   │   ├── imageproc/             # Image processing
│   │   ├── search/                # Search index generation
│   │   ├── link_checker/          # Link validation
│   │   ├── utils/                 # Utilities
│   │   ├── libs/                  # Shared dependencies
│   │   ├── console/               # CLI output
│   │   └── errors/                # Error handling
│   └── docs/                      # Official documentation
│
├── themes/                        # Community themes collection
├── after-dark/                    # Dark theme example
├── book/                          # Book documentation theme
├── even/                          # Clean, responsive theme
├── hyde/                          # Minimalist theme
├── giallo/                        # Yellow theme + syntax highlighting lib
└── zola/                          # Main source (git submodule)
```

---

## Architecture Summary

```
┌─────────────────────────────────────────────────────────────────┐
│                        Zola CLI (main.rs)                       │
│                    init | build | serve | check                 │
└─────────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│                      Site (components/site)                      │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────────────────┐  │
│  │   Load      │  │   Build     │  │      Serve              │  │
│  │  Content    │─▶│   Site      │─▶│   (Live Reload)         │  │
│  └─────────────┘  └─────────────┘  └─────────────────────────┘  │
└─────────────────────────────────────────────────────────────────┘
         │                │                  │
         ▼                ▼                  ▼
┌─────────────────┐ ┌─────────────────┐ ┌─────────────────┐
│    Content      │ │   Templates     │ │     Output      │
│  (pages, secs)  │ │    (Tera)       │ │  (HTML, CSS)    │
└─────────────────┘ └─────────────────┘ └─────────────────┘
         │                │
         ▼                ▼
┌─────────────────┐ ┌─────────────────┐
│   Markdown      │ │  Global Fns     │
│   (pulldown)    │ │  (load_data)    │
└─────────────────┘ └─────────────────┘
```

---

## Key Components

### 1. Config Component (`components/config/`)
- Parses `config.toml` configuration
- Handles multilingual settings
- Manages taxonomies, slugification, search config
- Markdown rendering options

### 2. Content Component (`components/content/`)
- **Page**: Individual markdown pages with front matter
- **Section**: Content sections with `_index.md`
- **Library**: Collection of all pages/sections
- **Taxonomy**: Tags, categories management
- **Pagination**: Page splitting for large sections

### 3. Templates Component (`components/templates/`)
- Tera template engine integration
- Global functions: `get_page`, `get_section`, `load_data`
- Filters: `markdown`, `base64_encode`, `regex_replace`
- Shortcode processing

### 4. Markdown Component (`components/markdown/`)
- pulldown-cmark parsing
- Syntax highlighting via syntect
- Shortcode injection
- Table of contents generation
- Link resolution

### 5. Site Component (`components/site/`)
- Main site building logic
- Parallel page rendering with Rayon
- Sass compilation
- Feed generation (RSS/Atom)
- Sitemap generation
- Link checking

### 6. Image Processing (`components/imageproc/`)
- Image resizing, cropping
- Format conversion
- Thumbnail generation
- Cached processing

---

## Themes Analyzed

### After Dark
- Dark theme with search support
- Syntax highlighting (one-dark theme)
- LaTeX/MathJax support
- Menu configuration via extra config

### Book
- Documentation/book layout
- Chapter-based navigation
- Search integration
- Clean, readable typography

### Even
- Responsive design
- Mobile-friendly navigation
- Pagination support
- Taxonomy feeds
- Table of contents per page

### Hyde
- Minimalist design
- Sidebar navigation
- Clean typography

### Giallo
- Includes syntax highlighting library
- Example configurations

---

## Related Documents

| Document | Description |
|----------|-------------|
| [`ssg-fundamentals.md`](./ssg-fundamentals.md) | Static site generation concepts |
| [`zola-architecture.md`](./zola-architecture.md) | How Zola works internally |
| [`tera-templating.md`](./tera-templating.md) | Tera template engine guide |
| [`content-management.md`](./content-management.md) | Front matter, taxonomies, sections |
| [`themes.md`](./themes.md) | Theme system analysis |
| [`rust-revision.md`](./rust-revision.md) | Building similar SSG in Rust |
| [`production-grade.md`](./production-grade.md) | Production deployment |

---

## File References

**Main Source Files:**
- `/home/darkvoid/Boxxed/@formulas/src.rust/src.zola-static-site-generator/zola/src/main.rs`
- `/home/darkvoid/Boxxed/@formulas/src.rust/src.zola-static-site-generator/zola/components/site/src/lib.rs`
- `/home/darkvoid/Boxxed/@formulas/src.rust/src.zola-static-site-generator/zola/components/content/src/page.rs`
- `/home/darkvoid/Boxxed/@formulas/src.rust/src.zola-static-site-generator/zola/components/content/src/section.rs`

**Template Examples:**
- `/home/darkvoid/Boxxed/@formulas/src.rust/src.zola-static-site-generator/even/templates/index.html`
- `/home/darkvoid/Boxxed/@formulas/src.rust/src.zola-static-site-generator/even/templates/page.html`

**Configuration Examples:**
- `/home/darkvoid/Boxxed/@formulas/src.rust/src.zola-static-site-generator/even/config.toml`
- `/home/darkvoid/Boxxed/@formulas/src.rust/src.zola-static-site-generator/after-dark/config.toml`
