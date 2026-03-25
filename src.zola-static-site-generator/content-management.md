# Content Management in Zola

## Table of Contents

1. [Front Matter](#front-matter)
2. [Sections](#sections)
3. [Pages](#pages)
4. [Taxonomies](#taxonomies)
5. [Page Collections](#page-collections)
6. [Multilingual Content](#multilingual-content)
7. [Asset Co-location](#asset-co-location)
8. [Content Organization Patterns](#content-organization-patterns)

---

## Front Matter

Front matter is metadata at the beginning of content files. Zola supports both TOML and YAML front matter.

### TOML Front Matter

```toml
+++
title = "My Blog Post"
description = "A brief description of the post"
date = 2026-03-26
updated = 2026-03-27
draft = false
slug = "my-blog-post"
path = "/custom/path"
template = "blog-post.html"
weight = 1

[taxonomies]
tags = ["rust", "ssg", "web"]
categories = ["Programming"]

[extra]
author = "John Doe"
thumbnail = "thumb.jpg"
+++

Content starts here...
```

### YAML Front Matter

```yaml
---
title: "My Blog Post"
description: "A brief description of the post"
date: 2026-03-26
updated: 2026-03-27
draft: false
slug: "my-blog-post"
taxonomies:
  tags: ["rust", "ssg", "web"]
  categories: ["Programming"]
extra:
  author: "John Doe"
  thumbnail: "thumb.jpg"
---

Content starts here...
```

### Front Matter Fields

| Field | Type | Description |
|-------|------|-------------|
| `title` | String | Page title (used in `<title>` and headings) |
| `description` | String | Meta description for SEO |
| `date` | DateTime | Publication date (for sorting) |
| `updated` | DateTime | Last update date |
| `draft` | Boolean | If true, page is excluded from build |
| `slug` | String | URL slug (overrides filename) |
| `path` | String | Full URL path (overrides slug) |
| `template` | String | Custom template to use |
| `weight` | Integer | Sort order (lower = first) |
| `aliases` | Array | URL redirects |
| `taxonomies` | Object | Tags, categories, etc. |
| `authors` | Array | Page authors |
| `in_search_index` | Boolean | Include in search (default: true) |
| `extra` | Object | Custom fields |

### Date Formats

```toml
# Full datetime
date = 2026-03-26T10:30:00Z
date = 2026-03-26T10:30:00+02:00

# Just date (assumes midnight UTC)
date = 2026-03-26

# In filename (automatically parsed)
2026-03-26-my-post.md
```

### Custom Extra Fields

```toml
[extra]
# Custom fields accessible via page.extra
author = "John Doe"
cover_image = "cover.jpg"
series = "Rust Programming"
series_order = 3
toc = true
mathjax = true
custom_css = ["custom.css"]

# Nested fields
[extra.social]
twitter = "@username"
github = "username"
```

---

## Sections

Sections organize content into logical groups. Each section has an `_index.md` file.

### Section Structure

```
content/
├── _index.md              # Root section (homepage)
├── blog/
│   ├── _index.md          # Blog section
│   ├── post1.md
│   └── post2.md
└── docs/
    ├── _index.md          # Docs section
    ├── intro/
    │   ├── _index.md      # Intro subsection
    │   └── getting-started.md
    └── advanced/
        ├── _index.md      # Advanced subsection
        └── optimization.md
```

### Section Front Matter

```toml
+++
title = "Blog"
description = "All blog posts"
template = "blog-section.html"
page_template = "blog-post.html"
sort_by = "date"
order_by = "descending"
generate_feeds = true
paginate_by = 10
paginate_path = "page"
insert_anchor_links = "right"
extra.header_image = "blog-header.jpg"
+++

Optional section content (rendered above pages)...
```

### Section Fields

| Field | Type | Description |
|-------|------|-------------|
| `title` | String | Section title |
| `description` | String | Section description |
| `template` | String | Section template (default: `section.html`) |
| `page_template` | String | Template for child pages |
| `sort_by` | String | Sort field: `date`, `title`, `weight`, etc. |
| `order_by` | String | `ascending` or `descending` |
| `generate_feeds` | Boolean | Generate RSS for this section |
| `paginate_by` | Integer | Items per page |
| `paginate_path` | String | Pagination URL path |
| `insert_anchor_links` | String | Anchor link position |
| `cascade` | Object | Values to cascade to child pages |
| `extra` | Object | Custom fields |

### Cascade Fields

```toml
+++
title = "Documentation"
cascade = { extra.documentation = true }
+++
```

All pages in this section (and subsections) will have:
```tera
{% if page.extra.documentation %}
    <span class="doc-badge">Documentation</span>
{% endif %}
```

### Accessing Section Data

```tera
{# Get current section #}
{% set current_section = get_section(path=section.path ~ "_index.md") %}

{# Access section data from page #}
{% set parent_section = get_section(path=page.ancestors.0 ~ "/_index.md") %}

{# Iterate subsections #}
{% for subsection_path in section.subsections %}
    {% set subsection = get_section(path=subsection_path) %}
    <h2>{{ subsection.title }}</h2>
{% endfor %}
```

---

## Pages

### Page File Locations

```
# Regular page in section
content/blog/my-post.md
→ /blog/my-post/

# Page with custom slug
content/blog/my-post.md (slug = "custom-url")
→ /blog/custom-url/

# Page with custom path
content/blog/my-post.md (path = "/articles/special")
→ /articles/special/

# Colocated assets page
content/blog/my-post/index.md
→ /blog/my-post/
content/blog/my-post/image.png
→ /blog/my-post/image.png
```

### Dated Pages

Files can start with a date for automatic date parsing:

```
content/blog/
├── 2026-03-26-my-first-post.md
├── 2026-03-27-second-post.md
└── 2026-03-28T10-30-00-scheduled-post.md
```

The date is automatically extracted and used for sorting.

### Page Sorting

Pages are sorted within sections:

```toml
# Sort by date, newest first
sort_by = "date"
order_by = "descending"

# Sort by weight, lowest first
sort_by = "weight"
order_by = "ascending"

# Sort by title, alphabetical
sort_by = "title"
order_by = "ascending"

# Sort by last updated
sort_by = "updated"
order_by = "descending"
```

### Navigation Between Pages

```tera
{# Previous/next page (by sort order) #}
{% if page.lower %}
    <a href="{{ page.lower.permalink }}">Previous: {{ page.lower.title }}</a>
{% endif %}

{% if page.higher %}
    <a href="{{ page.higher.permalink }}">Next: {{ page.higher.title }}</a>
{% endif %}

{# Access sibling pages #}
{% set section = get_section(path=page.ancestors | last ~ "/_index.md") %}
{% for p in section.pages %}
    <a href="{{ p.permalink }}" class="{% if p.path == page.path %}active{% endif %}">
        {{ p.title }}
    </a>
{% endfor %}
```

### Page Aliases (Redirects)

```toml
+++
title = "New URL Structure"
aliases = ["/old-url/", "/legacy/path/"]
+++
```

Zola generates redirect pages at the alias paths.

---

## Taxonomies

Taxonomies classify content (tags, categories, etc.).

### Configuring Taxonomies

```toml
# config.toml
taxonomies = [
    { name = "tags", feed = true },
    { name = "categories", feed = true },
    { name = "series", paginate_by = 5 },
]

# Per-language taxonomies
[languages.fr]
title = "Mon Site"

[languages.fr.taxonomies]
name = "étiquettes"  # French name for tags
```

### Using Taxonomies in Pages

```toml
+++
title = "My Post"
taxonomies.tags = ["rust", "web", "performance"]
taxonomies.categories = ["Programming", "Rust"]
taxonomies.series = ["Rust Fundamentals"]
+++
```

### Listing Taxonomy Terms

```tera
{# tags/list.html #}
{% extends "base.html" %}

{% block content %}
<h1>Tags</h1>
<ul>
    {% for term in terms %}
        <li>
            <a href="{{ term.permalink }}">
                {{ term.name }} ({{ term.page_count }})
            </a>
        </li>
    {% endfor %}
</ul>
{% endblock %}
```

### Single Taxonomy Term Page

```tera
{# tags/single.html #}
{% extends "base.html" %}

{% block content %}
<h1>Posts tagged with "{{ term.name }}"</h1>

{% for page in term.pages %}
    <article>
        <h2><a href="{{ page.permalink }}">{{ page.title }}</a></h2>
        <time>{{ page.date | date(format="%Y-%m-%d") }}</time>
    </article>
{% endfor %}
{% endblock %}
```

### Getting Taxonomy Data

```tera
{# All taxonomies #}
{% for taxonomy in config.taxonomies %}
    <h2>{{ taxonomy.name }}</h2>
{% endfor %}

{# Specific taxonomy #}
{% set tags = get_taxonomy(kind="tags") %}
{% for term in tags %}
    <a href="{{ term.permalink }}">{{ term.name }}</a>
{% endfor %}

{# Specific term #}
{% set rust_tag = get_taxonomy_term(kind="tags", name="rust") %}
<p>{{ rust_tag.page_count }} posts about Rust</p>

{# Generate taxonomy URL #}
{{ get_taxonomy_url(kind="tags", name="rust") }}
```

### Taxonomy Feeds

```toml
taxonomies = [
    { name = "tags", feed = true },
]
```

Generates: `/tags/atom.xml`, `/tags/rss.xml`

---

## Page Collections

### Getting Pages

```tera
{# All pages in section #}
{% set blog = get_section(path="blog/_index.md") %}
{% for page in blog.pages %}
    {{ page.title }}
{% endfor %}

{# Only metadata (faster) #}
{% set blog = get_section(path="blog/_index.md", metadata_only=true) %}

{# Specific page #}
{% set about = get_page(path="pages/about.md") %}

{# Pages from another section #}
{% set docs = get_section(path="docs/_index.md") %}
```

### Filtering Pages

```tera
{# Filter by taxonomy #}
{% set blog = get_section(path="blog/_index.md") %}
{% for page in blog.pages %}
    {% if page.taxonomies.tags is containing("rust") %}
        {{ page.title }}
    {% endif %}
{% endfor %}

{# Filter by date #}
{% set blog = get_section(path="blog/_index.md") %}
{% for page in blog.pages %}
    {% if page.date < now() %}
        {{ page.title }}
    {% endif %}
{% endfor %}

{# Filter with multiple conditions #}
{% set all_posts = get_section(path="blog/_index.md").pages %}
{% set published = all_posts | filter(attribute="draft", value=false) %}
{% set rust_posts = published | filter(attribute="taxonomies.tags", value="rust") %}
```

### Custom Collections

```tera
{# Create custom collection #}
{% set all_pages = get_section(path="blog/_index.md").pages %}
{% set featured = all_pages | filter(attribute="extra.featured", value=true) %}

{# Sort custom collection #}
{% set sorted = featured | sort(attribute="date") | reverse %}
```

### Related Content

```tera
{# Pages with same tags #}
{% set current_tags = page.taxonomies.tags | default(value=[]) %}
{% set related = [] %}

{% for p in section.pages %}
    {% if p.path != page.path %}
        {% set common_tags = p.taxonomies.tags | intersect(current_tags) %}
        {% if common_tags | length > 0 %}
            {% set_global related = related | concat(with=p) %}
        {% endif %}
    {% endif %}
{% endfor %}

{% if related | length > 0 %}
    <h3>Related Posts</h3>
    {% for p in related | slice(end=3) %}
        <a href="{{ p.permalink }}">{{ p.title }}</a>
    {% endfor %}
{% endif %}
```

---

## Multilingual Content

### Configuration

```toml
# config.toml
default_language = "en"
languages = [
    { code = "fr", title = "Mon Site", feed = true },
    { code = "de", title = "Meine Seite", feed = true },
]

# Translation strings
[translations]
read_more = "Read more"

[translations.fr]
read_more = "Lire la suite"

[translations.de]
read_more = "Weiterlesen"
```

### Multilingual Content Files

```
content/
├── _index.md              # English (default)
├── _index.fr.md           # French
├── _index.de.md           # German
├── about.md               # English
├── about.fr.md            # French
├── about.de.md            # German
└── blog/
    ├── _index.md
    ├── _index.fr.md
    └── posts/
        ├── my-post.md
        └── my-post.fr.md
```

### Linking Translations

```tera
{# Get translation of current page #}
{% if page.translations %}
    <div class="translations">
        {% for t in page.translations %}
            <a href="{{ t.permalink }}" lang="{{ t.lang }}">
                {{ t.lang | upper }}
            </a>
        {% endfor %}
    </div>
{% endif %}

{# Get translation string #}
{{ trans(key="read_more", lang=lang) }}

{# Check current language #}
{% if lang == "fr" %}
    <p>Contenu en français</p>
{% endif %}
```

### Language-Specific Config

```toml
default_language = "en"

[languages]
en = { title = "My Site", generate_feeds = true }
fr = { title = "Mon Site", generate_feeds = true }
de = { title = "Meine Seite" }

[languages.fr.taxonomies]
tags = "étiquettes"
categories = "catégories"
```

---

## Asset Co-location

### Page with Assets

```
content/posts/my-project/
├── index.md              # Page content
├── screenshot.png        # Co-located asset
├── demo.gif            # Co-located asset
└── data.csv            # Data file
```

### Accessing Co-located Assets

```markdown
<!-- In index.md -->

![Screenshot](screenshot.png)

[Download data](data.csv)
```

In templates:

```tera
{% for asset in page.serialized_assets %}
    <a href="{{ get_url(path=asset) }}">{{ asset }}</a>
{% endfor %}
```

### Section Assets

```
content/blog/
├── _index.md
├── banner.jpg          # Section-level asset
└── post.md
```

```tera
{# In section template #}
<img src="{{ section.serialized_assets.0 }}" alt="Banner">
```

### Image Processing on Co-located Assets

```tera
{% set hero = resize_image(
    path=page.serialized_assets.0,
    width=1200,
    height=630,
    op="fill"
) %}
<meta property="og:image" content="{{ hero.permalink }}">
```

---

## Content Organization Patterns

### Blog Pattern

```
content/
├── _index.md
├── blog/
│   ├── _index.md       # Blog section with pagination
│   ├── 2026-03-26-post-1.md
│   └── 2026-03-27-post-2.md
└── about.md
```

### Documentation Pattern

```
content/
├── docs/
│   ├── _index.md       # Docs overview
│   ├── getting-started/
│   │   ├── _index.md
│   │   ├── installation.md
│   │   └── configuration.md
│   ├── guides/
│   │   ├── _index.md
│   │   ├── basic.md
│   │   └── advanced.md
│   └── reference/
│       ├── _index.md
│       ├── api.md
│       └── cli.md
└── _index.md
```

### Documentation Navigation

```tera
{# docs-sidebar.html #}
{% set docs = get_section(path="docs/_index.md") %}

<nav class="docs-nav">
    {% for subsection_path in docs.subsections %}
        {% set subsection = get_section(path=subsection_path) %}
        <div class="nav-group">
            <h3>{{ subsection.title }}</h3>
            <ul>
                {% for page in subsection.pages %}
                    <li class="{% if page.path == current_path %}active{% endif %}">
                        <a href="{{ page.permalink }}">{{ page.title }}</a>
                    </li>
                {% endfor %}
            </ul>
        </div>
    {% endfor %}
</nav>
```

### Portfolio Pattern

```
content/
├── projects/
│   ├── _index.md
│   ├── project-1/
│   │   ├── index.md
│   │   └── images/
│   ├── project-2/
│   │   ├── index.md
│   │   └── images/
│   └── project-3/
│       ├── index.md
│       └── images/
└── _index.md
```

### Knowledge Base Pattern

```
content/
├── kb/
│   ├── _index.md
│   ├── articles/
│   │   ├── _index.md
│   │   └── *.md
│   ├── faq/
│   │   ├── _index.md
│   │   └── *.md
│   └── tutorials/
│       ├── _index.md
│       └── *.md
└── _index.md
```

### Pagination Setup

```toml
# In section _index.md
+++
paginate_by = 10
paginate_path = "page"
+++
```

```tera
{# In section template #}
{% for page in paginator.pages %}
    <article>{{ page.title }}</article>
{% endfor %}

<nav class="pagination">
    {% if paginator.previous %}
        <a href="{{ paginator.previous }}">← Previous</a>
    {% endif %}

    <span>Page {{ paginator.current_index }} of {{ paginator.number_pagers }}</span>

    {% if paginator.next %}
        <a href="{{ paginator.next }}">Next →</a>
    {% endif %}
</nav>
```

---

## Content Query Examples

### Recent Posts

```tera
{% set blog = get_section(path="blog/_index.md") %}
{% set recent = blog.pages | slice(end=5) %}

<h3>Recent Posts</h3>
{% for post in recent %}
    <a href="{{ post.permalink }}">{{ post.title }}</a>
{% endfor %}
```

### Featured Content

```tera
{% set all = get_section(path="blog/_index.md").pages %}
{% set featured = all | filter(attribute="extra.featured", value=true) %}

{% for post in featured %}
    <article class="featured">
        {{ post.title }}
    </article>
{% endfor %}
```

### Content by Tag

```tera
{% set rust_posts = get_taxonomy_term(kind="tags", name="rust") %}

<h2>Rust Posts</h2>
{% for post in rust_posts.pages %}
    {{ post.title }}
{% endfor %}
```

### Series/Sequence

```toml
# In page front matter
+++
extra.series = "Rust Fundamentals"
extra.series_order = 3
+++
```

```tera
{% set all_posts = get_section(path="blog/_index.md").pages %}
{% set same_series = all_posts | filter(attribute="extra.series", value="Rust Fundamentals") %}
{% set sorted = same_series | sort(attribute="extra.series_order") %}

<nav class="series-nav">
    {% for post in sorted %}
        <a href="{{ post.permalink }}" class="{% if post.path == page.path %}current{% endif %}">
            {{ post.extra.series_order }}. {{ post.title }}
        </a>
    {% endfor %}
</nav>
```
