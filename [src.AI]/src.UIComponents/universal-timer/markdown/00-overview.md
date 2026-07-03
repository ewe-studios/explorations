# What Is universal-timer?

A lightweight menu bar time tracker for macOS built with Electron.

**Source:** `/home/darkvoid/Boxxed/@formulas/src.rust/src.AI/src.UIComponents/universal-timer/`

## Architecture

```
Main Process (Node.js)          Renderer (Browser)
├── index.ts (entry)            ├── index.html
├── app-state.ts                └── IPC → Main
├── timer-state.ts
├── timer-actions.ts
├── ipc-handlers.ts
├── storage.ts (SQLite)
├── event-cache.ts
├── event-processing.ts
└── window-actions.ts
```

## Timer State Machine

`idle` → `running` → `paused` → `stopped`

## Storage

SQLite-backed event storage with caching.
