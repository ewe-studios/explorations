# Timer Engine

The timer uses event sourcing: a log of start/pause events, not stored elapsed times. State is computed from the full history.

Source: `src/main/timer-state.ts` (332 lines), `src/main/event-cache.ts` (198 lines), `src/main/event-processing.ts` (167 lines).

## Event Types

Source: `src/shared/types.ts`

```ts
export interface StartEvent { event: 'start'; ts: number; timer: string }
export interface PauseEvent { event: 'pause'; ts: number; timer: string }
export interface PauseAllEvent { event: 'pause_all'; ts: number }
export type TimerEvent = StartEvent | PauseEvent | PauseAllEvent
```

Events are stored in chronological order by timestamp. The event log is the single source of truth.

## Event Cache

Source: `src/main/event-cache.ts` — 198 lines.

Maintains an in-memory cache of processed events to avoid re-parsing the log on every state computation.

```ts
export function getTimerState(): TimerState
export function getEvents(): TimerEvent[]
export function getProcessedEvents(): ProcessedEvents
export function getColorOrder(): string[]
```

## Timer State Computation

Source: `src/main/timer-state.ts` — 332 lines.

Computes `TimerState` from the event log:

```ts
export interface TimerState {
  timers: TimerInfo[]        // All timers with stats
  runningTimers: string[]    // Currently running timer names
  totalToday: number         // Total time tracked today (ms)
  weeklyTrend: number        // % change vs last week
  weeklyAvg?: number         // Average per day this week
  timerColors: Record<string, string>
}
```

### Color Assignment

```ts
const TIMER_COLORS: string[] = [
  '#4a9eff', '#4ade80', '#f472b6', '#fbbf24',
  '#a78bfa', '#22d3d3', '#fb923c', '#f87171',
]

function getTimerColor(timerName: string, timerOrder: string[]): string {
  const index = timerOrder.indexOf(timerName)
  return TIMER_COLORS[index % TIMER_COLORS.length]
}
```

8 colors, assigned by display order position.

## Event Processing

Source: `src/main/event-processing.ts` — 167 lines.

### Core Functions

| Function | Purpose |
|----------|---------|
| `calculateOverlap()` | Detects overlapping timer intervals |
| `getDayStart()` | Midnight of current day (or configured start) |
| `getDayEnd()` | Midnight of next day |
| `getDaysFromMonday()` | Days since Monday (0=Monday) |
| `sumIntervalsPerTimerForDay()` | Total ms per timer for a day |
| `computeTrend()` | Weekly trend % (this week vs last week) |

Aha: The event processing handles overlapping timers correctly. If two timers run simultaneously, the overlap is detected and the total is computed without double-counting. The `calculateOverlap` function identifies where timer intervals intersect and adjusts the per-timer totals accordingly.

## Timer Actions

Source: `src/main/timer-actions.ts` — 246 lines.

Handles side effects:

| Function | Purpose |
|----------|---------|
| `createTrayIcon()` | Creates system tray with dynamic icon |
| `updateTrayIcon()` | Updates tray icon based on timer state |
| `registerGlobalHotkeys()` | Registers keyboard shortcuts |
| `syncTrayTitleInterval()` | Periodically updates tray title |

### Tray Icon States

```ts
function getTrayIconIndex(): number
// 0 = idle, 1 = running, 2 = paused
```

The tray icon changes based on whether timers are running, idle, or paused.

### Sound Notifications

```ts
// Plays resources/sound-start.wav or sound-pause.wav
// Only when settings.playSounds is true
```

## Storage

Source: `src/main/storage.ts` — 258 lines.

Settings persistence:

```ts
export function loadSettings(): void
export function saveSettings(): void
export function normalizeTimerName(name: string): string
export function timerDisplayNames(name: string): string
export function getDisplayName(name: string): string
```

Settings are stored as JSON in the app's user data directory (`app.getPath('userData')`).

## Window Actions

Source: `src/main/window-actions.ts` — 58 lines.

Tray context menu management:

```ts
export function rebuildContextMenu(): void
export function popUpContextMenu(): void
```

The context menu shows:
- Start/pause buttons for each timer
- Pause All
- Pin/Unpin
- Quit

## Cross-References

- [Overview](00-overview.md) — Architecture summary
- [Main Process](01-main-process.md) — IPC handlers that trigger timer operations
- [Renderer Process](02-renderer-process.md) — UI that displays computed state
