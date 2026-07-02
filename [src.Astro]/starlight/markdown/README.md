---
title: "Starlight Documentation Index"
---

# Starlight Documentation

**Starlight** (`@astrojs/starlight` v0.41.1) is a documentation theme for Astro that transforms Markdown/MDX content into polished, production-ready documentation sites. Built by the Astro team, it provides i18n, navigation, search, syntax highlighting, and responsive design out of the box.

**Source:** https://github.com/withastro/starlight

## Foundation

- [Overview](00-overview.html) -- What Starlight is, why it exists, quick architecture
- [Architecture](01-architecture.html) -- Module dependency graph, layer diagram, package structure
- [Configuration Schema](02-config-schema.html) -- Every config option, Zod validation, defaults, transforms

## Integration & Build

- [Integration Lifecycle](03-integration.html) -- How Starlight hooks into Astro's lifecycle
- [Virtual Modules](04-virtual-modules.html) -- How `virtual:starlight/*` exposes config at build time
- [Content Loaders](07-content-loaders.html) -- `docsLoader`, `i18nLoader`, and content collection integration

## Features

- [Internationalization (i18n)](05-i18n.html) -- Locales, translations, fallback content, BCP-47
- [Sidebar & Navigation](06-sidebar-nav.html) -- Sidebar schema, autogeneration, prev/next links
- [Components](08-components.html) -- Page frame, user components, override system
- [Plugins](09-plugins.html) -- Plugin API, hooks, `config:setup`, `i18n:setup`
- [Markdoc & DocSearch](10-markdoc-docsearch.html) -- Markdoc content support and Algolia DocSearch

## Cross-Cutting

- [Cross-Cutting Concerns](11-cross-cutting.html) -- Routing, route data, CSS layers, Expressive Code, Pagefind
