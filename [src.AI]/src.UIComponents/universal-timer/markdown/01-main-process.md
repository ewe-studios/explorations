# Main Process

The Electron main process manages the menu bar tray, window lifecycle, timer events, IPC communication, and settings persistence.

Source: `src/main/` — 8 files, ~1,800 lines total.

## Entry Point: `index.ts`

Source: `src/main/index.ts` — 175 lines.

### App Initialization

```ts
// src/main/index.ts:100-175
app.whenReady().then(() => {
  registerIpcHandlers()       // Register all IPC handlers
  loadSettings()              // Load settings from disk
  createWindow()              // Create the frameless BrowserWindow
  createTrayIcon()            // Create the system tray icon
  syncTrayTitleInterval()     // Periodically update tray title
  registerGlobalHotkeys()     // Register keyboard shortcuts
  rebuildContextMenu()        // Build tray context menu
})
```

### Window Configuration

```ts
// src/main/index.ts:28-38
const WINDOW_WIDTH = 360
const WINDOW_HEIGHT = 520

function createWindow(): void {
  mainWindow = new BrowserWindow({
    width: WINDOW_WIDTH, height: WINDOW_HEIGHT,
    show: false, frame: false, resizable: false,
    skipTaskbar: true, alwaysOnTop: true,
    webPreferences: {
      preload: path.join(__dirname, '../preload/index.js'),
      nodeIntegration: false, contextIsolation: true
    }
  })
}
```

Key choices:
- **`frame: false`** — No title bar (menu bar style)
- **`skipTaskbar: true`** — Doesn't appear in Dock
- **`alwaysOnTop: true`** — Stays above other windows
- **`nodeIntegration: false` + `contextIsolation: true`** — Security: renderer can't access Node.js directly

### Auto-Hide Behavior

```ts
mainWindow.on('blur', () => {
  if (!mainWindow!.webContents.isDevToolsOpened() && !isPinned()) {
    mainWindow!.hide()
  }
})

mainWindow.on('close', (event) => {
  if (!app.isQuitting) {
    event.preventDefault()
    mainWindow!.hide()
  }
})
```

Window hides on blur (unless pinned or DevTools open). Close button hides instead of quitting.

### Window Positioning

```ts
function positionWindow(trayBounds: Electron.Rectangle): void {
  // Center below tray icon, clamp to screen bounds
  // If below screen, position above tray instead
}
```

Positions the window centered below the tray icon, clamping to screen edges. If the window would go below the screen (tray at bottom), it appears above the tray.

## App State

Source: `src/main/app-state.ts` — 16 lines.

Simple module-level globals:

```ts
let mainWindow: BrowserWindow | null = null
let tray: Tray | null = null
let pinned = false

export function setMainWindow(w: BrowserWindow) { mainWindow = w }
export function setTray(t: Tray) { tray = t }
export function isPinned(): boolean { return pinned }
```

**Aha:** Using module-level globals instead of a state manager is intentional — the main process is single-threaded and single-instance. There's no risk of concurrent access or stale state. The tradeoff is testability, but for a menu bar app, simplicity wins.

## IPC Handlers

Source: `src/main/ipc-handlers.ts` — 319 lines.

Registers all `ipcMain.handle` endpoints:

| Channel | Handler | Description |
|---------|---------|-------------|
| `timer:start` | `(timerName) =>` | Start a timer |
| `timer:pause` | `(timerName) =>` | Pause a timer |
| `timer:pauseAll` | `() =>` | Pause all timers |
| `timer:rename` | `(name, newName) =>` | Rename a timer |
| `timer:delete` | `(name) =>` | Delete a timer |
| `timer:deleteSegment` | `(timer, start, end) =>` | Delete a time segment |
| `timer:getAll` | `() =>` | Get all timer info |
| `timer:getRunning` | `() =>` | Get currently running timer |
| `timer:getState` | `() =>` | Get full TimerState |
| `timer:getTimeline` | `(dateTs?) =>` | Get timeline for date |
| `timer:getMonthlyCalendar` | `(year, month) =>` | Get monthly calendar |
| `settings:get` | `() =>` | Get current settings |
| `settings:update` | `(partial) =>` | Update settings |
| `settings:updateTimerOrder` | `(order) =>` | Update timer display order |
| `data:export` | `() =>` | Export all data as JSON |
| `data:import` | `() =>` | Import data from JSON |
| `data:getEventsPath` | `() =>` | Get event log file path |
| `data:setEventsPath` | `() =>` | Set custom event log path |
| `data:resetEventsPath` | `() =>` | Reset to default event log path |
| `window:togglePin` | `() =>` | Toggle window pin |
| `window:getPinned` | `() =>` | Get pin state |
| `app:quit` | `() =>` | Quit the app |

After handling timer operations, the main process emits events to the renderer:

```ts
mainWindow?.webContents.send('timer:updated', computeState())
mainWindow?.webContents.send('settings:updated', settings)
mainWindow?.webContents.send('app:error', { message })
```

## Cross-References

- [Overview](00-overview.md) — Architecture summary
- [Renderer Process](02-renderer-process.md) — UI that calls these IPC handlers
- [Timer Engine](03-timer-engine.md) — How timer state is computed
