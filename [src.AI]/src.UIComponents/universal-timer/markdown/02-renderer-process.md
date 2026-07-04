# Renderer Process

The renderer process renders the timer UI, metrics charts, and settings panel. Three main views switchable from the UI.

Source: `src/renderer/src/` — 5 files, ~1,500 lines total.

## Entry Point: `renderer.ts`

Source: `src/renderer/src/renderer.ts` — 106 lines.

The renderer initializes the app and manages view switching (timers, metrics, settings):

```ts
// renderer.ts:1-30
// Load view templates, set up event listeners, register IPC callbacks
```

The renderer subscribes to:
- `timer:updated` — Refreshes timer display
- `settings:updated` — Refreshes settings and timer names
- `app:error` — Shows error notifications

## Timers View: `timers.ts`

Source: `src/renderer/src/timers.ts` — 381 lines.

The main timer list showing:
- All timers with elapsed time (live-updating)
- Start/pause buttons
- Running indicator (pulsing dot)
- Weekly total and trend per timer
- Rename and delete controls

### Live Time Updates

```ts
// Updates elapsed time every second for running timers
// Uses setInterval to poll getTimers() from main process
```

### Timer Creation

When a timer is started, the renderer calls `api.startTimer(name)`, which triggers:
1. Main process appends `StartEvent` to the event log
2. Main process recomputes state and emits `timer:updated`
3. Renderer receives the update and refreshes

## Metrics View: `metrics.ts`

Source: `src/renderer/src/metrics.ts` — 634 lines (largest file).

### Timeline View

Shows a day's timeline as colored bars:

```ts
// TimelineData interface:
interface TimelineData {
  segments: TimelineSegment[]  // timer, start, end, duration
  totalPerTimer: Map<string, number>
  totalMs: number
}
```

### Weekly Chart

Bar chart showing per-timer time for each day of the week.

### Monthly Calendar

```ts
interface MonthlyCalendarData {
  days: CalendarDayData[]  // date, totalMs, perTimer: Map<string, number>
}
```

Calendar grid showing daily totals with per-timer breakdown.

### Trend Calculation

```ts
// computeTrend compares this week's total to last week's
// Returns positive/negative percentage
```

Aha: The metrics view reads the full event log and computes everything client-side. There's no server or database — just a local JSON event file. The `event-processing.ts` module in the main process handles all the interval math (overlap detection, per-timer sums, trend computation).

## Settings View: `settings.ts`

Source: `src/renderer/src/settings.ts` — 316 lines.

Configurable settings:

| Setting | Type | Default | Purpose |
|---------|------|---------|---------|
| `pauseOthersOnStart` | boolean | false | Pause other timers when starting one |
| `playSounds` | boolean | false | Play start/pause sounds |
| `dayStartHour` | number | 9 | Work day start |
| `dayStartMinute` | number | 0 | Work day start minute |
| `hotkeys.pauseAll` | string | "" | Global hotkey to pause all |
| `hotkeys.timers` | Record<string,string> | {} | Per-timer hotkeys |
| `timerOrder` | string[] | [] | Display order |
| `timerFriendlyNames` | Record<string,string> | {} | Display names |
| `timerIcons` | Record<string,string> | {} | Custom icons |
| `useTaskNumberAsTrayIcon` | boolean | false | Show task number in tray |
| `showActiveTaskInTray` | boolean | false | Show active timer name in tray |
| `showActiveTimeInTray` | boolean | false | Show elapsed time in tray |

## Formatting: `formatting.ts`

Source: `src/renderer/src/formatting.ts` — 46 lines.

```ts
export function formatDuration(ms: number): string
export function formatTime(timestamp: number): string
export function formatHours(decimalHours: number): string
```

Converts milliseconds to human-readable strings: `3661000` → `"1h 1m 1s"`.

## Cross-References

- [Overview](00-overview.md) — Architecture summary
- [Main Process](01-main-process.md) — IPC handlers the renderer calls
- [Timer Engine](03-timer-engine.md) — How state is computed from events
