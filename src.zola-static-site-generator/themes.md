# Zola Theme System

## Table of Contents

1. [Theme Architecture](#theme-architecture)
2. [Theme Structure](#theme-structure)
3. [Included Themes Analysis](#included-themes-analysis)
4. [Creating Themes](#creating-themes)
5. [Theme Configuration](#theme-configuration)
6. [Theme Inheritance](#theme-inheritance)
7. [Best Practices](#best-practices)

---

## Theme Architecture

Zola's theme system allows complete separation of design from content. Themes are directories containing templates, static assets, and configuration that can be easily swapped.

### Theme Resolution Order

```
┌─────────────────────────────────────────────────────────────┐
│                    TEMPLATE RESOLUTION                       │
│                                                              │
│  1. Project templates (highest priority)                    │
│     /templates/*.html                                        │
│                                                              │
│  2. Theme templates                                          │
│     /themes/<theme-name>/templates/*.html                    │
│                                                              │
│  3. Built-in templates (lowest priority)                    │
│     __zola_builtins/*                                        │
└─────────────────────────────────────────────────────────────┘
```

### How Themes Work

```rust
// From components/templates/src/lib.rs
pub fn load_tera(path: &Path, config: &Config) -> Result<Tera> {
    // Load project templates first
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
        tera.extend(&tera_theme)?;  // Theme templates can be overridden
    }

    // Extend with built-in templates
    tera.extend(&ZOLA_TERA)?;
    tera.build_inheritance_chains()?;

    Ok(tera)
}
```

---

## Theme Structure

### Complete Theme Directory

```
themes/my-theme/
├── theme.toml              # Theme metadata
├── templates/
│   ├── base.html           # Base template
│   ├── index.html          # Homepage
│   ├── page.html           # Page template
│   ├── section.html        # Section listing
│   ├── 404.html            # 404 page
│   ├── shortcodes/
│   │   ├── youtube.html
│   │   └── figure.html
│   ├── partials/
│   │   ├── header.html
│   │   ├── footer.html
│   │   ├── nav.html
│   │   └── sidebar.html
│   ├── macros/
│   │   ├── post.html
│   │   └── pagination.html
│   └── tags/
│       ├── list.html       # Tag list page
│       └── single.html     # Single tag page
├── static/
│   ├── css/
│   │   └── style.css
│   ├── js/
│   │   └── theme.js
│   └── images/
│       └── logo.png
├── sass/
│   └── style.scss
└── content/                # Optional example content
    └── _index.md
```

### theme.toml Format

```toml
name = "my-theme"
description = "A clean, minimal theme"
license = "MIT"
homepage = "https://github.com/user/my-theme"
min_version = "0.17.0"  # Minimum Zola version
demo = "https://my-theme-demo.netlify.app"

[author]
name = "Your Name"
homepage = "https://your-website.com"

[original]
# If this is a port/adaptation of another theme
author = "Original Author"
homepage = "https://original-theme.com"
repo = "https://github.com/original/theme"

[extra]
# Default values for config.extra
menu = [
    { url = "$BASE_URL", name = "Home" },
    { url = "$BASE_URL/blog", name = "Blog" },
]
```

---

## Included Themes Analysis

### After Dark

**Location:** `/home/darkvoid/Boxxed/@formulas/src.rust/src.zola-static-site-generator/after-dark/`

**Features:**
- Dark color scheme
- Search functionality
- Syntax highlighting (one-dark theme)
- LaTeX/MathJax support
- Responsive design

**Config:**
```toml
base_url = "https://getzola.github.io/after-dark/"
compile_sass = true
title = "after-dark theme"
description = "A robust, elegant dark theme"
generate_feeds = true
build_search_index = true

taxonomies = [
    {name = "categories", feed = true},
    {name = "tags", feed = true},
]

[markdown]
highlight_code = true
highlight_theme = "one-dark"

[extra]
author = "John Doe"
after_dark_menu = [
    {url = "$BASE_URL", name = "Home"},
    {url = "$BASE_URL/categories", name = "Categories"},
    {url = "$BASE_URL/tags", name = "Tags"},
]
codeblock = true  # Copy to clipboard
latex = true      # MathJax
enable_search = true
```

**Template Structure:**
```
after-dark/templates/
├── index.html
├── page.html
├── 404.html
├── post_macros.html
├── partials/
│   ├── latex.html
│   └── search.html
├── shortcodes/
│   ├── audio.html
│   ├── gif.html
│   ├── note.html
│   ├── responsive.html
│   └── youtube.html
└── tags/
    ├── list.html
    └── single.html
```

---

### Even

**Location:** `/home/darkvoid/Boxxed/@formulas/src.rust/src.zola-static-site-generator/even/`

**Features:**
- Clean, responsive design
- Mobile-friendly navigation
- Table of contents
- Pagination support

**Config:**
```toml
base_url = "https://getzola.github.io/even/"
compile_sass = true
title = "even theme"
description = "A clean, responsive theme"
generate_feed = true

taxonomies = [
    {name = "categories", feed = true},
    {name = "tags", feed = true},
]

[markdown]
highlight_code = true
highlight_theme = "css"

[extra]
author = "Vincent"
even_menu = [
    {url = "$BASE_URL", name = "Home"},
    {url = "$BASE_URL/categories", name = "Categories"},
    {url = "$BASE_URL/tags", name = "Tags"},
    {url = "$BASE_URL/about", name = "About"},
]
even_title = "Even"
```

**Template Analysis (index.html):**
```html+tera
{% extends "index.html" %}
{% import "post_macros.html" as post_macros %}

<!DOCTYPE html>
<html lang="en">
<head>
    {# Meta tags #}
    {% if page.description %}
        <meta name="description" content="{{ page.description }}" />
    {% endif %}

    {# RSS/Atom feeds #}
    {% if config.generate_feeds %}
        {% for feed in config.feed_filenames %}
        <link rel="alternate" type="{% if feed == "atom.xml" %}"application/atom+xml"{% else %}"application/rss+xml"{% endif %}"
              title="RSS" href="{{ get_url(path=feed) }}">
        {% endfor %}
    {% endif %}

    {# CSS/JS blocks #}
    {% block css %}
        <link rel="stylesheet" href="{{ get_url(path="site.css") }}">
    {% endblock %}
</head>

<body>
    {# Mobile navigation #}
    <nav id="mobile-menu" class="mobile-menu">
        {% for item in config.extra.even_menu %}
            <li><a href="{{ item.url | replace(from="$BASE_URL", to=config.base_url) }}">{{ item.name }}</a></li>
        {% endfor %}
    </nav>

    {# Main content with pagination #}
    <main>
        {% for page in paginator.pages %}
            {{ post_macros::title(page=page) }}
            <div class="post__summary">{{ page.summary | safe }}</div>
        {% endfor %}

        <nav class="pagination">
            {% if paginator.previous %}
                <a href="{{ paginator.previous }}">‹ Previous</a>
            {% endif %}
            {% if paginator.next %}
                <a href="{{ paginator.next }}">Next ›</a>
            {% endif %}
        </nav>
    </main>
</body>
</html>
```

---

### Book

**Location:** `/home/darkvoid/Boxxed/@formulas/src.rust/src.zola-static-site-generator/book/`

**Features:**
- Documentation/book layout
- Chapter navigation
- Search integration
- Clean typography

**Config:**
```toml
base_url = "https://getzola.github.io/book/"
compile_sass = true
title = "book theme"
description = "A book theme"
build_search_index = true

[markdown]
highlight_code = true
highlight_theme = "css"

[extra]
book_number_chapters = true
book_only_current_section_pages = false
```

**Template Structure:**
```
book/templates/
├── index.html
├── page.html
└── section.html
```

**Use Case:** Technical documentation, books, tutorials

---

### Hyde

**Location:** `/home/darkvoid/Boxxed/@formulas/src.rust/src.zola-static-site-generator/hyde/`

**Features:**
- Minimalist sidebar design
- Clean typography
- Single column layout

**Templates:**
```
hyde/templates/
├── 404.html
├── index.html
└── page.html
```

---

### Giallo

**Location:** `/home/darkvoid/Boxxed/@formulas/src.rust/src.zola-static-site-generator/giallo/`

**Features:**
- Yellow color scheme
- Includes syntax highlighting library

**Structure:**
```
giallo/
├── Cargo.toml          # Syntax highlighting as a crate
├── src/
│   ├── highlight.rs
│   ├── languages.rs
│   └── themes.rs
└── examples/
    └── highlight.rs
```

---

## Creating Themes

### Step 1: Create Theme Directory

```bash
mkdir -p themes/my-theme/{templates,static,sass}
```

### Step 2: Create theme.toml

```toml
name = "my-theme"
description = "A minimal blog theme"
license = "MIT"
min_version = "0.17.0"

[author]
name = "Your Name"
homepage = "https://your-site.com"
```

### Step 3: Create Base Template

```html+tera
{# templates/base.html #}
<!DOCTYPE html>
<html lang="{{ lang }}">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>{% block title %}{{ config.title }}{% endblock %}</title>

    {% block head %}
        <link rel="stylesheet" href="{{ get_url(path="style.css") }}">
    {% endblock %}
</head>
<body>
    {% block header %}
    <header>
        <h1><a href="{{ config.base_url }}">{{ config.title }}</a></h1>
        {% if config.description %}
            <p>{{ config.description }}</p>
        {% endif %}
    </header>
    {% endblock %}

    <main>
        {% block content %}{% endblock %}
    </main>

    {% block footer %}
    <footer>
        <p>&copy; {{ now() | date(format="%Y") }} {{ config.extra.author }}</p>
    </footer>
    {% endblock %}
</body>
</html>
```

### Step 4: Create Page Template

```html+tera
{# templates/page.html #}
{% extends "base.html" %}

{% block title %}{{ page.title }} - {{ config.title }}{% endblock %}

{% block content %}
<article class="page">
    <h1>{{ page.title }}</h1>

    {% if page.date %}
        <time>{{ page.date | date(format="%Y-%m-%d") }}</time>
    {% endif %}

    <div class="content">
        {{ page.content | safe }}
    </div>

    {% if page.taxonomies.tags %}
        <div class="tags">
            {% for tag in page.taxonomies.tags %}
                <a href="{{ get_taxonomy_url(kind="tags", name=tag) }}">#{{ tag }}</a>
            {% endfor %}
        </div>
    {% endif %}
</article>
{% endblock %}
```

### Step 5: Create Section Template

```html+tera
{# templates/section.html #}
{% extends "base.html" %}

{% block content %}
<section>
    {% if section.title %}
        <h1>{{ section.title }}</h1>
    {% endif %}

    {% if section.description %}
        <p>{{ section.description }}</p>
    {% endif %}

    {% for page in paginator.pages %}
        <article>
            <h2><a href="{{ page.permalink }}">{{ page.title }}</a></h2>
            <time>{{ page.date | date(format="%Y-%m-%d") }}</time>
            {% if page.summary %}
                <div class="summary">{{ page.summary | safe }}</div>
            {% endif %}
        </article>
    {% endfor %}

    {% if paginator %}
        <nav class="pagination">
            {% if paginator.previous %}
                <a href="{{ paginator.previous }}">Previous</a>
            {% endif %}
            {% if paginator.next %}
                <a href="{{ paginator.next }}">Next</a>
            {% endif %}
        </nav>
    {% endif %}
</section>
{% endblock %}
```

### Step 6: Add Styles

```scss
// sass/style.scss
$primary-color: #3498db;
$text-color: #333;
$bg-color: #fff;

body {
    font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, sans-serif;
    line-height: 1.6;
    color: $text-color;
    background: $bg-color;
    max-width: 800px;
    margin: 0 auto;
    padding: 20px;
}

header {
    border-bottom: 2px solid $primary-color;
    margin-bottom: 40px;
}

a {
    color: $primary-color;
    text-decoration: none;

    &:hover {
        text-decoration: underline;
    }
}

article {
    margin-bottom: 40px;
}

.tags {
    a {
        margin-right: 10px;
        color: #666;
    }
}
```

---

## Theme Configuration

### Using a Theme

```toml
# config.toml
base_url = "https://example.com"
theme = "my-theme"  # Name of theme in /themes/

[extra]
# Theme-specific options
menu = [
    { url = "$BASE_URL", name = "Home" },
    { url = "$BASE_URL/blog", name = "Blog" },
]
```

### Theme Extra Configuration

```toml
# In theme.toml
[extra]
show_authors = true
show_dates = true
social_links = [
    { platform = "twitter", url = "https://twitter.com/..." },
    { platform = "github", url = "https://github.com/..." },
]

# In project config.toml
[extra]
# Override theme defaults
show_authors = false
custom_css = ["custom.css"]
```

---

## Theme Inheritance

### Base Theme + Child Theme

```
themes/
├── parent-theme/        # Base theme
│   ├── templates/
│   │   └── base.html
│   └── static/
└── child-theme/         # Extends parent
    ├── templates/
    │   └── page.html    # Overrides parent
    └── static/
```

### Overriding Templates

Project templates always take precedence:

```
project/
├── themes/
│   └── my-theme/
│       └── templates/
│           └── page.html    # Theme's page.html
└── templates/
    └── page.html            # Overrides theme's page.html
```

---

## Best Practices

### 1. Use Template Blocks Extensively

```html+tera
{# Good: Many override points #}
{% block head %}{% endblock %}
{% block extra_head %}{% endblock %}
{% block header %}{% endblock %}
{% block content %}{% endblock %}
{% block footer %}{% endblock %}
{% block js %}{% endblock %}
```

### 2. Provide Sensible Defaults

```toml
# theme.toml
[extra]
# Provide defaults that users can override
menu = [{ url = "$BASE_URL", name = "Home" }]
show_sidebar = true
```

### 3. Use $BASE_URL Placeholder

```html+tera
{# Good: Works with any base URL #}
<a href="{{ item.url | replace(from="$BASE_URL", to=config.base_url) }}">

{# Bad: Hardcoded #}
<a href="https://mysite.com/{{ item.url }}">
```

### 4. Include All Required Templates

```
templates/
├── index.html      # Required for homepage
├── page.html       # Required for pages
├── section.html    # Required for sections
├── 404.html        # Required for 404
├── anchor-link.html # For toc
└── taxonomy/
    ├── list.html   # Required for taxonomy lists
    └── single.html # Required for taxonomy terms
```

### 5. Document Configuration Options

```toml
# theme.toml
[extra]
# Available configuration options:
# - menu: Array of menu items {url, name}
# - show_dates: Boolean, show publication dates
# - show_authors: Boolean, show author names
# - social_links: Array of social media links
```

### 6. Make Styles Customizable

```scss
// Provide CSS variables for easy customization
:root {
    --primary-color: #3498db;
    --text-color: #333;
    --bg-color: #fff;
}
```

### 7. Support Dark Mode

```css
@media (prefers-color-scheme: dark) {
    :root {
        --text-color: #eee;
        --bg-color: #1a1a1a;
    }
}
```

### 8. Include Example Content

```
themes/my-theme/
├── example/
│   ├── config.toml
│   └── content/
│       └── _index.md
└── README.md  # With setup instructions
```

### 9. Test with Various Configurations

- With and without pagination
- With and without taxonomies
- Multiple languages
- Different content types

### 10. Provide Shortcode Templates

```html+tera
{# templates/shortcodes/figure.html #}
<figure>
    <img src="{{ src }}" alt="{{ alt }}">
    {% if caption %}
        <figcaption>{{ caption | markdown | safe }}</figcaption>
    {% endif %}
</figure>
```

---

## Theme Distribution

### Publishing to Zola Themes

1. Create theme following structure above
2. Add to zola-themes repository as submodule
3. Submit pull request

### Self-Hosting

```bash
# Add theme as git submodule
git submodule add https://github.com/user/theme themes/my-theme

# Or clone directly
git clone https://github.com/user/theme themes/my-theme
```

### NPM/Cargo Distribution

Some themes are distributed as packages:

```bash
# Install theme
npm install zola-theme-even

# Copy to project
cp -r node_modules/zola-theme-even themes/even
```
