---
title: "Astro 7.0.4 Documentation Index"
---

# Astro 7.0.4 Documentation

Complete documentation for Astro 7.0.4, based on source code at [`packages/astro`](https://github.com/withastro/astro/tree/main/packages/astro). Astro is a content-focused web framework that ships zero JavaScript by default using islands architecture.

## Foundation

- [Overview](00-overview.html) — What Astro is, core philosophy, zero-JS-by-default principle
- [Architecture](01-architecture.html) — Monorepo structure, 23 packages, layer diagram, dependency graph

## Deep Dives

- [Configuration & Routing](02-config-routing.html) — AstroConfig schema, settings, file-based routing, dynamic params, redirects
- [Component Model](03-component-model.html) — `.astro` files, frontmatter, JSX rendering, virtual modules
- [Vite Plugins](04-vite-plugins.html) — 23 custom plugins: astro compiler, pages, routes, CSS, markdown, server
- [Build Pipeline](05-build-pipeline.html) — Three-environment sequential build: prerender, SSR, client
- [SSR Runtime](06-ssr-runtime.html) — App class, manifest, request handling, adapter integration
- [Integration API](07-integration-api.html) — Hooks system, plugin injection, renderer registration
- [Content Collections](08-content-collections.html) — defineCollection, schema validation, content layers, live collections
- [Islands Hydration](09-islands-hydration.html) — Islands architecture, client directives, astro-island custom element
- [Assets & Images](10-assets-images.html) — Image service pipeline, Sharp, font optimization, asset handling
- [Middleware & Actions](11-middleware-actions.html) — defineMiddleware, sequence, server actions, API context
- [Internationalization](12-i18n.html) — Locale resolution, pathname-prefix vs domain routing, fallback routes
- [Dev Toolbar](13-dev-toolbar.html) — Preferences system, audit plugins, overlay rendering

## Cross-Cutting

- [Cross-Cutting Concerns](14-cross-cutting.html) — CSP, caching, session management, errors, telemetry

---

Source: [withastro/astro](https://github.com/withastro/astro) v7.0.4 | [npm](https://www.npmjs.com/package/astro)
