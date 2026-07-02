---
title: "Tauri Ecosystem — Specification"
---

# Tauri Ecosystem — Documentation Spec

## Source Codebase

- **Location:** `/home/darkvoid/Boxxed/@formulas/src.rust/src.Tauri/src.tauri/`
- **Sub-projects:**
  - `tauri/` — Core Tauri framework (Rust)
  - `wry/` — Cross-platform WebView library (v0.55.1)
  - `tao/` — Cross-platform windowing library (v0.34.5)
  - `taffy/` — Flexbox/Grid layout engine (v0.5.2)
  - `dioxus/` — Rust UI framework
  - `plugins-workspace/` — Official Tauri plugins
- **Language:** Rust 2021 edition
- **License:** MIT / Apache-2.0
- **GitHub:**
  - https://github.com/tauri-apps/tauri
  - https://github.com/tauri-apps/wry
  - https://github.com/tauri-apps/tao
  - https://github.com/dioxuslabs/dioxus
  - https://github.com/dioxuslabs/taffy

## What This Is

Tauri is a framework for building desktop applications using web technologies. The frontend is HTML/CSS/JS rendered in a native WebView. The backend is Rust. Unlike Electron, which bundles Chromium and Node.js (150-200 MB binaries), Tauri uses the OS-provided WebView (WKWebView on macOS, WebView2 on Windows, WebKitGTK on Linux), resulting in ~2-10 MB binaries. The ecosystem spans six major crates plus a plugin system.

## Documentation Goal

A reader should understand:
1. How Tauri's `App<R>`, `AppManager`, and plugin system orchestrate a desktop application
2. How IPC between JavaScript and Rust works, including the ACL permission system
3. How wry abstracts 4 platform WebView backends behind a single `WebViewBuilder` API
4. How tao abstracts 5 platform windowing backends behind a single `EventLoop` API
5. How taffy implements Flexbox/Grid/Block layout algorithms with `no_std` support
6. How dioxus provides a React-like `rsx!` macro and VirtualDom for Rust UI
7. How official plugins (shell, fs, http, sql, websocket) extend Tauri's capabilities
8. How the CLI and bundler produce DMG, MSI, NSIS, AppImage, deb, and rpm packages
9. Cross-cutting concerns: security, performance, platform differences, build pipeline

## Documentation Structure

```
[src.Tauri]/
├── spec.md                     ← This file
├── markdown/
│   ├── README.md               ← Index
│   ├── 00-overview.md          ← What Tauri is, architecture at a glance
│   ├── 01-tauri-core.md        ← App<R>, Builder, managers, plugin system
│   ├── 02-tauri-ipc-acl.md     ← IPC channel, ACL permission system, command macros
│   ├── 03-tauri-build-cli.md   ← tauri-cli, tauri-build, bundler
│   ├── 04-wry-webview.md       ← WebViewBuilder, 4 platform backends, protocol handlers
│   ├── 05-tao-windowing.md     ← EventLoop, Window, 5 platform backends
│   ├── 06-taffy-layout.md      ← TaffyTree, Flexbox/Grid/Block algorithms
│   ├── 07-dioxus-ui.md         ← VirtualDom, rsx!, WriteMutations, signals
│   ├── 08-plugins.md           ← shell, fs, dialog, http, sql, websocket
│   ├── 09-native-mobile.md     ← iOS/Android support, native APIs
│   └── 10-cross-cutting.md     ← Security, performance, testing, patterns
├── html/                       ← Generated HTML
└── build.py                    ← Shared build script
```

## Tasks

| #  | Document             | Status |
|----|----------------------|--------|
| 00 | spec.md              | DONE   |
| 01 | README.md (index)    | DONE   |
| 02 | 00-overview.md       | DONE   |
| 03 | 01-tauri-core.md     | DONE   |
| 04 | 02-tauri-ipc-acl.md  | DONE   |
| 05 | 03-tauri-build-cli.md| DONE   |
| 06 | 04-wry-webview.md    | DONE   |
| 07 | 05-tao-windowing.md  | DONE   |
| 08 | 06-taffy-layout.md   | DONE   |
| 09 | 07-dioxus-ui.md      | DONE   |
| 10 | 08-plugins.md        | DONE   |
| 11 | 09-native-mobile.md  | DONE   |
| 12 | 10-cross-cutting.md  | DONE   |
| 13 | build.py (copy)      | DONE   |
| 14 | Grandfather review   | DONE   |

## Build System

```bash
cd "[src.Tauri]" && python3 ../build.py [src.Tauri]
```

Zero dependencies. Python 3.12+ stdlib only.

## Quality Requirements

All ten Iron Rules from `markdown_engineering/documentation_directive.md` apply:
1. Detailed sections with code snippets
2. Teach key facts quickly
3. Clear articulation
4. Minimum 2 mermaid diagrams per document
5. Good visual assets (tables, code blocks)
6. Generated HTML with navigation
7. Cross-references between documents
8. Source path references
9. Aha moments (minimum 1 per document)
10. Navigation: index + prev/next buttons

## Expected Outcome

After reading, a developer can:
- Architect a Tauri application from scratch
- Understand the IPC/ACL permission model
- Extend Tauri with custom plugins
- Choose the right WebView configuration for their platform
- Debug cross-platform WebView issues
- Build and bundle for all target platforms

## Resume Point

If interrupted, check the Tasks table above and continue from the next TODO item.
