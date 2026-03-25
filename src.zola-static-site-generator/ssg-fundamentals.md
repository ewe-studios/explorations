# Static Site Generation Fundamentals

## Table of Contents

1. [What is a Static Site Generator?](#what-is-a-static-site-generator)
2. [How SSGs Work](#how-ssgs-work)
3. [Benefits vs Dynamic Sites](#benefits-vs-dynamic-sites)
4. [SSG Architecture Diagram](#ssg-architecture-diagram)
5. [Key Concepts](#key-concepts)
6. [Comparison with Other Approaches](#comparison-with-other-approaches)

---

## What is a Static Site Generator?

A **Static Site Generator (SSG)** is a tool that generates complete HTML pages at **build time** rather than at **request time**. Instead of running server-side code for each visitor, an SSG pre-renders all pages and outputs static HTML, CSS, and JavaScript files that can be served by any web server.

### The Static Site Workflow

```
┌──────────────┐     ┌──────────────┐     ┌──────────────┐
│   Content    │     │    SSG       │     │   Static     │
│  (Markdown,  │ ──▶ │   (Build)    │ ──▶ │   Files      │
│   Data)      │     │              │     │  (HTML/CSS)  │
└──────────────┘     └──────────────┘     └──────────────┘
                                                  │
                                                  ▼
                                         ┌──────────────┐
                                         │     CDN/     │
                                         │   Web Server │
                                         └──────────────┘
```

---

## How SSGs Work

### Build Process Steps

1. **Content Loading**
   - Scan content directory for markdown/text files
   - Parse front matter (metadata)
   - Build content tree/structure

2. **Template Processing**
   - Load template files
   - Apply template inheritance
   - Register functions and filters

3. **Content Rendering**
   - Convert Markdown to HTML
   - Apply syntax highlighting
   - Process shortcodes/components

4. **Page Generation**
   - Combine content with templates
   - Generate URLs/permalinks
   - Create pagination if needed

5. **Asset Processing**
   - Compile Sass/SCSS to CSS
   - Process/minify JavaScript
   - Optimize images

6. **Output Generation**
   - Write HTML files to output directory
   - Copy static assets
   - Generate feeds, sitemaps, search index

### Detailed Flow Diagram

```
                          ┌─────────────────┐
                          │  config.toml    │
                          └────────┬────────┘
                                   │
                                   ▼
┌─────────────────────────────────────────────────────────────┐
│                      BUILD PROCESS                           │
│                                                              │
│  ┌──────────┐    ┌──────────┐    ┌──────────┐               │
│  │  Parse   │    │  Build   │    │  Render  │               │
│  │  Config  │───▶│  Content │───▶│ Templates│               │
│  └──────────┘    └────┬─────┘    └────┬─────┘               │
│                       │                │                     │
│                       ▼                ▼                     │
│                ┌──────────┐    ┌──────────┐                  │
│                │  Pages   │    │  Tera    │                  │
│                │  Sections│    │  Engine  │                  │
│                │  Taxonomy│    │          │                  │
│                └──────────┘    └──────────┘                  │
│                                                              │
└─────────────────────────────────────────────────────────────┘
                                   │
                                   ▼
                          ┌─────────────────┐
                          │   public/       │
                          │   (output)      │
                          └─────────────────┘
```

---

## Benefits vs Dynamic Sites

### Static Sites (SSG)

| Advantage | Description |
|-----------|-------------|
| **Performance** | Pre-rendered HTML served instantly, no server processing |
| **Security** | No database, no server-side code to exploit |
| **Scalability** | Serve from CDN, handle unlimited traffic |
| **Cost** | Cheap/free hosting (Netlify, Vercel, GitHub Pages) |
| **Version Control** | All content in git, easy rollbacks |
| **Simplicity** | No server maintenance, no updates needed |
| **SEO** | Full HTML content immediately available |

### Dynamic Sites (CMS, Frameworks)

| Advantage | Description |
|-----------|-------------|
| **Real-time Content** | Content can change per request |
| **User Personalization** | Different content per user |
| **Interactive Features** | Comments, user accounts, e-commerce |
| **Admin Interface** | Non-technical content management |

### When to Use an SSG

- Blogs and documentation
- Marketing/landing pages
- Portfolios
- Product documentation
- Any content-heavy site without user-specific content

### When NOT to Use an SSG

- Social networks
- E-commerce with dynamic pricing
- Real-time dashboards
- User-generated content platforms
- Sites requiring authentication

---

## SSG Architecture Diagram

```
┌─────────────────────────────────────────────────────────────────────┐
│                         CONTENT LAYER                                │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐                  │
│  │  Markdown   │  │   Front     │  │   Static    │                  │
│  │   Files     │  │   Matter    │  │   Assets    │                  │
│  └─────────────┘  └─────────────┘  └─────────────┘                  │
└─────────────────────────────────────────────────────────────────────┘
                                  │
                                  ▼
┌─────────────────────────────────────────────────────────────────────┐
│                        PROCESSING LAYER                              │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐                  │
│  │   Markdown  │  │    Tera     │  │     Sass    │                  │
│  │   Parser    │  │  Templates  │  │   Compiler  │                  │
│  └─────────────┘  └─────────────┘  └─────────────┘                  │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐                  │
│  │    Image    │  │    Search   │  │     Feed    │                  │
│  │  Processor  │  │    Index    │  │  Generator  │                  │
│  └─────────────┘  └─────────────┘  └─────────────┘                  │
└─────────────────────────────────────────────────────────────────────┘
                                  │
                                  ▼
┌─────────────────────────────────────────────────────────────────────┐
│                         OUTPUT LAYER                                 │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐                  │
│  │    HTML     │  │     CSS     │  │  JavaScript │                  │
│  │   Pages     │  │   Styles    │  │   (optional)│                  │
│  └─────────────┘  └─────────────┘  └─────────────┘                  │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐                  │
│  │   RSS/Atom  │  │   Sitemap   │  │   Search    │                  │
│  │    Feeds    │  │     XML     │  │    JSON     │                  │
│  └─────────────┘  └─────────────┘  └─────────────┘                  │
└─────────────────────────────────────────────────────────────────────┘
```

---

## Key Concepts

### 1. Front Matter

Metadata at the beginning of content files, typically in YAML or TOML format:

```toml
+++
title = "My Blog Post"
date = 2026-03-26
draft = false
taxonomies.tags = ["rust", "ssg"]
+++

Content starts here...
```

### 2. Templates

Reusable HTML structures with placeholders:

```html+tera
{% extends "base.html" %}

{% block content %}
  <h1>{{ page.title }}</h1>
  {{ page.content | safe }}
{% endblock %}
```

### 3. Sections

Content organization units, each with an `_index.md`:

```
content/
├── _index.md          # Root section
├── blog/
│   ├── _index.md      # Blog section
│   ├── post1.md
│   └── post2.md
└── docs/
    ├── _index.md      # Docs section
    └── guide.md
```

### 4. Taxonomies

Classification systems for content:

```toml
# config.toml
taxonomies = [
    { name = "tags", feed = true },
    { name = "categories", feed = true },
]
```

### 5. Shortcodes

Reusable components within content:

```
{{ youtube(id="dQw4w9WgXcQ") }}
{{ figure(src="image.jpg", alt="Description") }}
```

### 6. Asset Co-location

Assets stored with content:

```
content/posts/my-post/
├── index.md
├── image1.png
├── image2.png
└── data.csv
```

---

## Comparison with Other Approaches

### SSG vs Traditional CMS (WordPress)

```
┌──────────────────────────────────────────────────────────────┐
│                      TRADITIONAL CMS                          │
│                                                              │
│   User Request ──▶ Server ──▶ Database Query                │
│                         │                                    │
│                         ▼                                    │
│                   Template Engine                            │
│                         │                                    │
│                         ▼                                    │
│                   HTML Response                              │
│                                                              │
│   ⚡ Every request requires server processing               │
└──────────────────────────────────────────────────────────────┘

┌──────────────────────────────────────────────────────────────┐
│                    STATIC SITE GENERATOR                      │
│                                                              │
│   Build Time: Content + Templates ──▶ Static HTML           │
│                                                              │
│   User Request ──▶ CDN ──▶ Static HTML File                 │
│                                                              │
│   ⚡ No server processing at request time                   │
└──────────────────────────────────────────────────────────────┘
```

### SSG vs Server-Side Rendering (Next.js, etc.)

| Aspect | SSG | SSR |
|--------|-----|-----|
| Build Time | Pre-renders all pages | Builds on demand |
| First Contentful Paint | Instant | Waiting for server |
| Server Load | None (static files) | High (per-request) |
| Content Freshness | Build-time only | Real-time |
| Hosting Cost | Very low | Higher |
| Complexity | Simple | Complex |

### SSG vs Client-Side Rendering (React SPA)

| Aspect | SSG | CSR |
|--------|-----|-----|
| SEO | Excellent | Requires SSR/hydration |
| Initial Load | Full HTML | Empty HTML + JS |
| Interactivity | Limited | Full |
| JavaScript Required | No | Yes |
| Accessibility | Better | Depends on implementation |

---

## Popular SSGs

| SSG | Language | Notable Features |
|-----|----------|------------------|
| **Zola** | Rust | Single binary, fast, built-in features |
| **Hugo** | Go | Very fast, large ecosystem |
| **Jekyll** | Ruby | Original SSG, GitHub Pages |
| **Next.js** | JavaScript | Hybrid SSG/SSR |
| **Gatsby** | JavaScript | React-based, GraphQL |
| **Eleventy** | JavaScript | Flexible, simple |
| **Pelican** | Python | Django templates |

---

## Performance Comparison

Typical build times for a 1000-page site:

```
Zola:    ~1-2 seconds    ████████
Hugo:    ~2-3 seconds    ██████████
Jekyll:  ~30-60 seconds  ████████████████████████████████
Next.js: ~10-20 seconds  █████████████████
```

Zola achieves its speed through:
- Rust's performance
- Parallel processing with Rayon
- Efficient markdown parsing
- Minimal allocations
