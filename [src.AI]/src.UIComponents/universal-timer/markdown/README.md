# universal-timer Documentation

A lightweight macOS menu bar time tracker built with Electron. Multiple named timers, event-sourced logging, weekly/monthly metrics, global hotkeys, and configurable settings.

| Attribute | Value |
|-----------|-------|
| Source files | ~16 (src/) |
| Framework | Electron + electron-vite |
| Target | macOS (menu bar app) |
| Build | electron-builder → .dmg |

## Navigation

| # | Document | Purpose |
|---|----------|---------|
| 00 | [Overview](00-overview.md) | Architecture, quick start, types |
| 01 | [Main Process](01-main-process.md) | Window, tray, IPC handlers, initialization |
| 02 | [Renderer Process](02-renderer-process.md) — Timers UI, metrics, settings, formatting |
| 03 | [Timer Engine](03-timer-engine.md) — Event sourcing, state computation, interval math |
| 04 | [Features](04-features.md) — Hotkeys, pin, sounds, data export/import |
| 05 | [Build & Config](05-build-config.md) — electron-vite, electron-builder, resources |
