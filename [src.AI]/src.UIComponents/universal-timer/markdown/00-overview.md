# universal-timer: macOS Menu Bar Time Tracker

Lightweight menu bar time tracker for macOS built with Electron.

**Source:** `/home/darkvoid/Boxxed/@formulas/src.rust/src.AI/src.UIComponents/universal-timer/`

## Architecture

```
src/
├── main/                    ← Electron main process
│   ├── index.ts             ← Entry point, window creation
│   ├── app-state.ts         ← Application state
│   ├── timer-state.ts       ← Timer state (idle/running/paused/stopped)
│   ├── timer-actions.ts     ← Start/stop/pause/resume
│   ├── ipc-handlers.ts      ← IPC handler registration
│   ├── storage.ts           ← SQLite storage
│   └── event-processing.ts  ← Event aggregation
├── preload/
│   └── index.ts             ← Context bridge
└── shared/
    └── types.ts             ← Shared TypeScript types
```

## Timer State Machine

```
idle → running → paused → stopped
  ↑_______________|
```

## Storage

SQLite-backed event storage with caching.
