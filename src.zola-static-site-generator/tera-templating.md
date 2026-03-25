# Tera Templating Guide

## Table of Contents

1. [Introduction to Tera](#introduction-to-tera)
2. [Template Syntax](#template-syntax)
3. [Template Inheritance](#template-inheritance)
4. [Filters](#filters)
5. [Tests](#tests)
6. [Functions](#functions)
7. [Macros](#macros)
8. [Zola-Specific Extensions](#zola-specific-extensions)
9. [Best Practices](#best-practices)

---

## Introduction to Tera

**Tera** is a template engine for Rust based on Jinja2 and the Django template language. Zola uses Tera as its templating engine, providing a powerful and familiar syntax for generating HTML.

### Key Features

- **Template inheritance** - Base templates with overrideable blocks
- **Macros** - Reusable template components with arguments
- **Filters** - Transform values (like `| upper`)
- **Tests** - Conditional checks (like `is defined`)
- **Functions** - Global functions for data loading
- **Auto-escaping** - HTML escaping by default for safety
- **Whitespace control** - Trim whitespace with `-` markers

---

## Template Syntax

### Variables

Variables are accessed with `{{ }}`:

```html+tera
<h1>{{ page.title }}</h1>
<p>{{ config.description }}</p>
```

### Variable Access

```html+tera
{# Dot notation for nested access #}
{{ page.meta.title }}
{{ config.extra.author.name }}

{# Array indexing #}
{{ page.ancestors.0 }}

{# Array iteration #}
{% for tag in page.taxonomies.tags %}
    <span>{{ tag }}</span>
{% endfor %}
```

### String Literals and Operators

```html+tera
{% set greeting = "Hello, World!" %}
{% set number = 42 %}
{% set float = 3.14 %}
{% set boolean = true %}

{# Arithmetic #}
{% set sum = number + 10 %}
{% set product = number * 2 %}

{# Comparison #}
{% if number > 10 %}
    <p>Large number</p>
{% endif %}

{# String concatenation #}
{% set full_name = first_name ~ " " ~ last_name %}
```

### Comments

```html+tera
{# This is a comment #}
{#
    Multi-line
    comment
#}
```

---

## Template Inheritance

### Base Template

```html+tera
{# templates/base.html #}
<!DOCTYPE html>
<html lang="{{ lang }}">
<head>
    <meta charset="UTF-8">
    <title>{% block title %}{{ config.title }}{% endblock title %}</title>

    {% block head %}
        <link rel="stylesheet" href="{{ get_url(path="style.css") }}">
    {% endblock head %}

    {% block extra_head %}{% endblock extra_head %}
</head>
<body>
    <header>
        {% block header %}
            <nav>
                {% for item in config.extra.menu %}
                    <a href="{{ item.url }}">{{ item.name }}</a>
                {% endfor %}
            </nav>
        {% endblock header %}
    </header>

    <main>
        {% block content %}{% endblock content %}
    </main>

    <footer>
        {% block footer %}
            <p>&copy; {{ config.extra.author }}</p>
        {% endblock footer %}
    </footer>

    {% block js %}{% endblock js %}
</body>
</html>
```

### Child Template

```html+tera
{# templates/page.html #}
{% extends "base.html" %}

{% block title %}{{ page.title }} - {{ config.title }}{% endblock title %}

{% block extra_head %}
    <meta name="description" content="{{ page.description }}">
{% endblock extra_head %}

{% block content %}
<article class="page">
    <h1>{{ page.title }}</h1>
    <div class="content">
        {{ page.content | safe }}
    </div>
</article>
{% endblock content %}
```

### Extending Further

```html+tera
{# templates/blog-post.html #}
{% extends "page.html" %}

{% block extra_head %}
    {{ super() }}
    <meta property="article:published_time" content="{{ page.date }}">
{% endblock extra_head %}

{% block content %}
    {{ super() }}

    <footer class="post-footer">
        {% if page.taxonomies.tags %}
            <div class="tags">
                {% for tag in page.taxonomies.tags %}
                    <a href="{{ get_taxonomy_url(kind="tags", name=tag) }}">
                        #{{ tag }}
                    </a>
                {% endfor %}
            </div>
        {% endif %}
    </footer>
{% endblock content %}
```

---

## Filters

### Built-in Filters

```html+tera
{# String filters #}
{{ name | upper }}
{{ name | lower }}
{{ title | capitalize }}
{{ text | truncate(length=100) }}
{{ text | replace(from="old", to="new") }}
{{ text | split(pat=",") }}
{{ text | trim }}

{# Number filters #}
{{ price | round(precision=2) }}
{{ count | pluralize(left="item", right="items") }}

{# Array filters #}
{{ items | first }}
{{ items | last }}
{{ items | length }}
{{ items | reverse }}
{{ items | sort(attribute="title") }}
{{ items | filter(attribute="draft", value=false) }}
{{ items | map(attribute="title") }}

{# Object filters #}
{{ data | get(key="name") }}
{{ object | get(key="nested.value", default="N/A") }}

{# Date filters #}
{{ page.date | date(format="%Y-%m-%d") }}
{{ page.date | date(format="%B %d, %Y") }}

{# Encoding #}
{{ secret | base64_encode }}
{{ encoded | base64_decode }}

{# Markdown #}
{{ markdown_text | markdown }}
{{ inline_md | markdown(inline=true) }}
```

### Zola-Specific Filters

```html+tera
{# Regex replace #}
{{ "John Doe" | regex_replace(pattern=r"(\w+), (\w+)", rep="$2 $1") }}

{# Number formatting #}
{{ 1000000 | num_format(locale="en") }}
{{ 1000000 | num_format(locale="fr") }}
```

### Custom Filters in Zola

Zola registers these filters automatically:

```rust
// From components/templates/src/filters.rs
tera.register_filter("base64_encode", filters::base64_encode);
tera.register_filter("base64_decode", filters::base64_decode);
tera.register_filter("regex_replace", filters::RegexReplaceFilter::new());
tera.register_filter("markdown", filters::MarkdownFilter::new(...));
```

### Filter Chaining

```html+tera
{{ page.title | lower | replace(from=" ", to="-") | truncate(length=50) }}
```

---

## Tests

Tests are used in conditionals to check values:

```html+tera
{# Type checks #}
{% if value is defined %}
    <p>Value exists</p>
{% endif %}

{% if value is undefined %}
    <p>Value missing</p>
{% endif %}

{% if value is none %}
    <p>Value is null</p>
{% endif %}

{% if items is iterable %}
    {% for item in items %}...{% endfor %}
{% endif %}

{% if data is object %}
    <p>It's an object</p>
{% endif %}

{% if flag is starting_with("enable") %}
    <p>Enabled feature</p>
{% endif %}

{% if name is ending_with(".md") %}
    <p>Markdown file</p>
{% endif %}

{% if path is containing("blog") %}
    <p>Blog content</p>
{% endif %}

{% if num is divisibleby(divisor=2) %}
    <p>Even number</p>
{% endif %}

{% if arr is containing("target") %}
    <p>Found target</p>
{% endif %}
```

### Negating Tests

```html+tera
{% if value is not defined %}
    <p>Missing value</p>
{% endif %}
```

---

## Functions

### Global Functions

```html+tera
{# Get a page by path #}
{% set about = get_page(path="pages/about.md") %}
{{ about.title }}

{# Get a section #}
{% set blog = get_section(path="blog/_index.md") %}
{% for page in blog.pages %}
    <a href="{{ page.permalink }}">{{ page.title }}</a>
{% endfor %}

{# Get all pages in a section #}
{% set all_blog = get_section(path="blog/_index.md", metadata_only=true) %}

{# Get taxonomy terms #}
{% set tags = get_taxonomy(kind="tags") %}
{% for term in tags %}
    <a href="{{ term.permalink }}">{{ term.name }} ({{ term.page_count }})</a>
{% endfor %}

{# Get specific taxonomy term #}
{% set rust_tag = get_taxonomy_term(kind="tags", name="rust") %}

{# Generate URL #}
{% set css_url = get_url(path="style.css", trailing_slash=false) %}
{% set post_url = get_url(path="blog/post.md") %}

{# Process image #}
{% set hero = get_image(path="hero.jpg") %}
<img src="{{ hero.url }}" width="{{ hero.width }}" height="{{ hero.height }}">

{# Resize image #}
{% set thumb = resize_image(path="photo.jpg", width=300, height=200) %}

{# Load external data #}
{% set config_data = load_data(path="config.yaml", format="yaml") %}
{% set api_data = load_data(url="https://api.example.com/data", format="json") %}

{# Translation #}
{{ trans(key="read_more", lang=lang) }}
```

### Image Processing Functions

```html+tera
{# Resize #}
{% set resized = resize_image(
    path="image.jpg",
    width=800,
    height=600,
    op="fill",
    format="webp"
) %}

{# Operations: fit, fill, scale #}
{% set thumbnail = resize_image(path="pic.png", width=150, op="scale") %}

{# Get image metadata #}
{% set img = get_image(path="photo.jpg") %}
<p>Dimensions: {{ img.width }}x{{ img.height }}</p>
```

### Load Data Options

```html+tera
{# JSON #}
{% set data = load_data(path="data.json", format="json") %}

{# YAML #}
{% set config = load_data(path="config.yml", format="yaml") %}

{# TOML #}
{% set settings = load_data(path="settings.toml", format="toml") %}

{# CSV #}
{% set rows = load_data(path="data.csv", format="csv") %}

{# Plain text #}
{% set text = load_data(path="file.txt", format="plain") %}

{# Bibtex #}
{% set citations = load_data(path="papers.bib", format="bibtex") %}

{# XML #}
{% set feed = load_data(url="https://example.com/feed.xml", format="xml") %}
```

---

## Macros

### Defining Macros

```html+tera
{# templates/macros.html #}
{% macro render_post(page) %}
<article class="post">
    <h2>
        <a href="{{ page.permalink }}">{{ page.title }}</a>
    </h2>
    <time>{{ page.date | date(format="%Y-%m-%d") }}</time>
    <div class="summary">
        {{ page.summary | default(value=page.description) | safe }}
    </div>
</article>
{% endmacro render_post %}

{% macro render_tag(tag, kind="tags") %}
<a href="{{ get_taxonomy_url(kind=kind, name=tag) }}" class="tag">
    #{{ tag }}
</a>
{% endmacro render_tag %}

{% macro render_image(path, alt, caption) %}
<figure>
    <img src="{{ get_url(path=path) }}" alt="{{ alt }}">
    {% if caption %}
        <figcaption>{{ caption }}</figcaption>
    {% endif %}
</figure>
{% endmacro render_image %}

{% macro pagination(paginator) %}
<nav class="pagination">
    {% if paginator.previous %}
        <a href="{{ paginator.previous }}" class="previous">Previous</a>
    {% endif %}

    <span class="page-info">
        Page {{ paginator.current_index }} of {{ paginator.number_pagers }}
    </span>

    {% if paginator.next %}
        <a href="{{ paginator.next }}" class="next">Next</a>
    {% endif %}
</nav>
{% endmacro pagination %}
```

### Using Macros

```html+tera
{# Import macros #}
{% import "macros.html" as macros %}

{# Use macros #}
{% for page in paginator.pages %}
    {{ macros::render_post(page=page) }}
{% endfor %}

{{ macros::pagination(paginator=paginator) }}

{# Import specific macro #}
{% from "macros.html" import render_tag %}
{{ render_tag(tag="rust") }}

{# Import with namespace #}
{% import "macros.html" as m %}
{{ m::render_tag(tag="zola") }}
```

### Macro Files per Theme

```
themes/after-dark/
├── templates/
│   ├── index.html
│   ├── page.html
│   └── post_macros.html    ← Theme-specific macros
```

---

## Zola-Specific Extensions

### The `config` Object

```html+tera
{{ config.base_url }}
{{ config.title }}
{{ config.description }}
{{ config.generate_feeds }}
{{ config.build_search_index }}
{{ config.default_language }}
{{ config.markdown.highlight_code }}
{{ config.extra.author }}
{{ config.feed_filenames }}
```

### The `page` Object

```html+tera
{{ page.title }}
{{ page.description }}
{{ page.content }}
{{ page.summary }}
{{ page.permalink }}
{{ page.path }}
{{ page.slug }}
{{ page.date }}
{{ page.updated }}
{{ page.word_count }}
{{ page.reading_time }}
{{ page.toc }}
{{ page.taxonomies }}
{{ page.ancestors }}
{{ page.lower.permalink }}      {# Previous page #}
{{ page.higher.permalink }}     {# Next page #}
{{ page.extra.custom_field }}
```

### The `section` Object

```html+tera
{{ section.title }}
{{ section.content }}
{{ section.permalink }}
{{ section.pages }}
{{ section.subsections }}
{{ section.ancestors }}
{{ section.toc }}
{{ section.extra.custom_field }}
```

### The `paginator` Object

```html+tera
{% if paginator %}
    {{ paginator.pages }}          {# Current page items #}
    {{ paginator.first }}          {# First page URL #}
    {{ paginator.previous }}       {# Previous page URL #}
    {{ paginator.next }}           {# Next page URL #}
    {{ paginator.current_index }}  {# Current page number #}
    {{ paginator.number_pagers }}  {# Total pages #}
    {{ paginator.total_pages }}    {# Total items #}
{% endif %}
```

### The `taxonomy` Object

```html+tera
{# In taxonomy list template #}
{{ taxonomy.kind.name }}
{{ taxonomy.kind.feed }}
{{ taxonomy.items }}  {# List of terms #}

{# In term template #}
{{ term.name }}
{{ term.slug }}
{{ term.permalink }}
{{ term.pages }}
{{ term.page_count }}
```

### Control Flow with Zola Data

```html+tera
{# Iterate over pages with sorting #}
{% for page in section.pages %}
    <article>
        <h2>{{ page.title }}</h2>
        {{ page.summary | safe }}
    </article>
{% endfor %}

{# Check for taxonomy terms #}
{% if page.taxonomies.tags %}
    <div class="tags">
        {% for tag in page.taxonomies.tags %}
            <span>{{ tag }}</span>
        {% endfor %}
    </div>
{% endif %}

{# Table of contents #}
{% if page.toc %}
    <nav class="toc">
        <ul>
            {% for h1 in page.toc %}
                <li>
                    <a href="{{ h1.permalink }}">{{ h1.title }}</a>
                    {% if h1.children %}
                        <ul>
                            {% for h2 in h1.children %}
                                <li><a href="{{ h2.permalink }}">{{ h2.title }}</a></li>
                            {% endfor %}
                        </ul>
                    {% endif %}
                </li>
            {% endfor %}
        </ul>
    </nav>
{% endif %}
```

### Shortcodes in Templates

While shortcodes are primarily used in markdown, their output appears in templates:

```html+tera
{# Shortcode output in page.content #}
<div class="content">
    {{ page.content | safe }}
</div>
```

---

## Best Practices

### 1. Use Template Inheritance

```html+tera
{# Good: Extends base #}
{% extends "base.html" %}

{% block title %}Page Title{% endblock %}
{% block content %}...{% endblock %}

{# Bad: Duplicates HTML structure #}
<!DOCTYPE html>
<html>
...
</html>
```

### 2. Create Reusable Macros

```html+tera
{# Define once, use everywhere #}
{% macro card(title, image, content) %}
<div class="card">
    <img src="{{ image }}" alt="{{ title }}">
    <h3>{{ title }}</h3>
    <p>{{ content }}</p>
</div>
{% endmacro %}
```

### 3. Use `safe` Filter Judiciously

```html+tera
{# Safe: You trust the content #}
{{ page.content | safe }}

{# Unsafe: User input should be escaped #}
{{ page.title }}  {# Auto-escaped #}
```

### 4. Leverage Default Values

```html+tera
{# Provide fallbacks #}
{{ page.description | default(value=config.description) }}
{{ page.extra.author | default(value=config.extra.author) }}
```

### 5. Use `metadata_only` for Performance

```html+tera
{# Only load metadata, not full content #}
{% set section = get_section(path="blog/_index.md", metadata_only=true) %}
```

### 6. Organize Templates Logically

```
templates/
├── base.html           # Base template
├── index.html          # Homepage
├── page.html           # Generic page
├── section.html        # Section listing
├── macros/
│   ├── navigation.html
│   ├── pagination.html
│   └── components.html
└── partials/
    ├── header.html
    ├── footer.html
    └── sidebar.html
```

### 7. Include Partials

```html+tera
{% include "partials/header.html" %}

{# With context #}
{% include "partials/nav.html" %}

{# Without context (slightly faster) #}
{% include "partials/nav.html" ignore missing %}
```

### 8. Whitespace Control

```html+tera
{# Trim whitespace #}
{% if condition -%}
    <p>No leading whitespace</p>
{%- endif %}

{# Compact loops #}
{% for item in items -%}
    <span>{{ item }}</span>
{%- endfor %}
```

### 9. Use set for Complex Expressions

```html+tera
{% set full_url = config.base_url ~ page.path %}
{% set is_draft = page.draft or page.extra.draft %}

{% if is_draft %}
    <meta name="robots" content="noindex">
{% endif %}
```

### 10. Handle Missing Data Gracefully

```html+tera
{% if page is defined and page.title %}
    <h1>{{ page.title }}</h1>
{% else %}
    <h1>Untitled</h1>
{% endif %}

{# Or use default #}
<h1>{{ page.title | default(value="Untitled") }}</h1>
```

---

## Example: Complete Blog Template

```html+tera
{# templates/blog.html #}
{% extends "base.html" %}
{% import "macros/pagination.html" as pagination %}

{% block title %}Blog - {{ config.title }}{% endblock %}

{% block content %}
<section class="blog">
    <header>
        <h1>Blog</h1>
        {% if section.description %}
            <p class="description">{{ section.description }}</p>
        {% endif %}
    </header>

    {% for page in paginator.pages %}
        <article class="post-preview">
            <h2>
                <a href="{{ page.permalink }}">{{ page.title }}</a>
            </h2>

            <div class="meta">
                <time datetime="{{ page.date }}">
                    {{ page.date | date(format="%B %d, %Y") }}
                </time>

                {% if page.authors %}
                    <span class="author">by {{ page.authors | join(sep=", ") }}</span>
                {% endif %}
            </div>

            {% if page.summary %}
                <div class="summary">
                    {{ page.summary | safe }}
                </div>
            {% endif %}

            {% if page.taxonomies.tags %}
                <div class="tags">
                    {% for tag in page.taxonomies.tags %}
                        <a href="{{ get_taxonomy_url(kind="tags", name=tag) }}">
                            #{{ tag }}
                        </a>
                    {% endfor %}
                </div>
            {% endif %}

            <a href="{{ page.permalink }}" class="read-more">Read more →</a>
        </article>
    {% endfor %}

    {{ pagination::render(paginator=paginator) }}
</section>
{% endblock %}
```
