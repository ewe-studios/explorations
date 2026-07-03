# Main Process

## Source Files

| File | Purpose |
|------|---------|
| `main/index.ts` | Entry point, window creation |
| `main/app-state.ts` | Application state management |
| `main/timer-state.ts` | Timer state (idle/running/paused/stopped) |
| `main/timer-actions.ts` | Timer start/stop/pause/resume |
| `main/ipc-handlers.ts` | IPC handler registration |
| `main/storage.ts` | SQLite storage layer |
| `main/event-cache.ts` | Event caching for performance |
| `main/event-processing.ts` | Event aggregation and processing |
| `main/window-actions.ts` | Window management (show/hide/menu bar) |

## Preload

`preload/index.ts` — Context bridge exposing safe APIs to the renderer.

## Shared Types

`shared/types.ts` — TypeScript types shared between main and renderer.
