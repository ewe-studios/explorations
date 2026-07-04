# Features

## Global Hotkeys

Configurable per-timer hotkeys via `settings.hotkeys.timers`:

```ts
interface HotkeySettings {
  pauseAll: string      // Global pause-all hotkey
  timers: Record<string, string>  // Per-timer start/pause hotkeys
}
```

Registered via `globalShortcut.register()` in Electron. Hotkeys work even when the app window is hidden.

## Pin Behavior

The window can be "pinned" to stay visible:

```ts
// src/main/index.ts:55-56
mainWindow.on('blur', () => {
  if (!mainWindow!.webContents.isDevToolsOpened() && !isPinned()) {
    mainWindow!.hide()
  }
})
```

When pinned, the window stays visible even when focus is lost. Toggle via tray context menu or renderer UI.

## Data Export/Import

The full event log and settings can be exported/imported as JSON:

| IPC Channel | Description |
|-------------|-------------|
| `data:export` | Returns `{ events: TimerEvent[], settings: Settings }` |
| `data:import` | Loads events and settings from JSON |

Aha: Because the timer is event-sourced, export/import is a single JSON file. There's no database migration or schema versioning — just replay the events and the state computes itself.

## Event Log Path

The event log file can be relocated:

| IPC Channel | Description |
|-------------|-------------|
| `data:getEventsPath` | Returns current event log file path |
| `data:setEventsPath` | Opens file picker to choose new location |
| `data:resetEventsPath` | Resets to default app data path |

## Tray Icon

Dynamic tray icon based on timer state:
- **Idle** — Default icon
- **Running** — Active icon (may show task number)
- **Paused** — Paused icon

Configurable via settings:
- `useTaskNumberAsTrayIcon` — Show current task number
- `showActiveTaskInTray` — Show active timer name as tray title
- `showActiveTimeInTray` — Show elapsed time as tray title

## Timeline

Per-day view showing colored bars for each timer's active periods:

```
TimelineSegment {
  timer: string
  displayName: string
  start: number    // timestamp
  end: number      // timestamp
  duration: number // ms
}
```

Overlapping timers are detected and displayed.

## Monthly Calendar

Grid view showing daily totals:

```
CalendarDayData {
  date: Date
  totalMs: number
  perTimer: Map<string, number>
}
```

Each day cell shows the total tracked time and a breakdown by timer.

## Cross-References

- [Overview](00-overview.md) — Architecture summary
- [Main Process](01-main-process.md) — IPC handlers
- [Timer Engine](03-timer-engine.md) — Event sourcing
- [Build & Config](05-build-config.md) — Packaging
