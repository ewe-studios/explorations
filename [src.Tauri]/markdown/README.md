---
title: "Tauri Ecosystem — Documentation Index"
---

# Tauri Ecosystem Documentation

Comprehensive documentation for the Tauri desktop application ecosystem. Six major Rust crates plus a plugin system, covering the full stack from application framework to WebView rendering to window management.

## Foundation

- [Overview](00-overview.md) — What Tauri is, architecture at a glance, crate dependency graph

## Core Deep Dives

- [Tauri Core](01-tauri-core.md) — `App<R>`, `Builder`, `AppManager`, `WindowManager`, `WebviewManager`, plugin system, state management, `Runtime` trait, `generate_context!` macro
- [IPC and ACL](02-tauri-ipc-acl.md) — `#[tauri::command]` macro, capability files, scoped permissions, IPC transport layers, event system, invoke handler composition
- [Build and CLI](03-tauri-build-cli.md) — `tauri dev`, `tauri build`, `tauri-build` code generation, `tauri-bundler` pipeline (DMG/MSI/NSIS/AppImage/deb/rpm), icon generation

## Platform Libraries

- [Wry WebView](04-wry-webview.md) — `WebViewBuilder` (50+ fields), 4 platform backends (WKWebView, WebView2, WebKitGTK, Android WebView), protocol handlers, initialization scripts
- [Tao Windowing](05-tao-windowing.md) — `EventLoop`, `Window` (60+ methods), `ControlFlow`, 5 platform backends, raw window handles, DPI awareness, multi-monitor

## UI and Layout

- [Taffy Layout](06-taffy-layout.md) — `TaffyTree`, Flexbox/Grid/Block algorithms, 9-slot measure cache, `no_std` support, `LayoutTree` trait
- [Dioxus UI](07-dioxus-ui.md) — `VirtualDom`, `rsx!` macro, `WriteMutations` trait, signals, hooks, desktop renderer (tao+wry), cross-platform rendering

## Extensions

- [Plugins](08-plugins.md) — shell, filesystem, dialog, HTTP, SQL (sqlx), WebSocket (tokio-tungstenite), custom plugin development
- [Native and Mobile](09-native-mobile.md) — iOS (WKWebView + Xcode), Android (WebView + Gradle), lifecycle, permissions, safe area

## Cross-Cutting

- [Security, Performance, Patterns](10-cross-cutting.md) — CSP, ACL defense-in-depth, binary size, memory usage, IPC throughput, testing strategies, design patterns, common pitfalls, CI/CD

## Source Repositories

| Crate | Version | GitHub |
|-------|---------|--------|
| `tauri` | — | https://github.com/tauri-apps/tauri |
| `wry` | 0.55.1 | https://github.com/tauri-apps/wry |
| `tao` | 0.34.5 | https://github.com/tauri-apps/tao |
| `taffy` | 0.5.2 | https://github.com/dioxuslabs/taffy |
| `dioxus` | 0.6 | https://github.com/dioxuslabs/dioxus |
