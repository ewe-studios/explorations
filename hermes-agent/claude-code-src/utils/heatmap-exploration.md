# heatmap.ts Deep Dive Exploration

**Source File:** `/home/darkvoid/Boxxed/@formulas/src.rust/src.llamacpp/src.ClaudOpen/claude-code-main/src/utils/heatmap.ts`  
**Lines:** 199  
**Status:** Complete

---

## Module Overview

The `heatmap.ts` module generates GitHub-style activity heatmaps for the terminal using chalk for colored output. It visualizes daily activity data (message counts) as a grid of colored characters, where intensity represents activity level.

**Purpose:** Visual representation of Claude Code usage patterns over time, similar to GitHub's contribution graph.

**Key Features:**
- Dynamic terminal width adaptation
- Percentile-based intensity calculation (adaptive to user's activity level)
- Month labels for temporal context
- Day-of-week labels (Mon, Wed, Fri)
- Claude orange color scheme (#da7756)
- Unicode block characters for intensity levels

---

## File Inventory

| File | Lines | Purpose |
|------|-------|---------|
| `heatmap.ts` | 199 | Heatmap generation with chalk terminal colors |

**Dependencies:**
- `chalk` — Terminal colorization (ANSI/hex colors)
- `./stats.js` — `DailyActivity` type
- `./statsCache.js` — `toDateString()` utility

---

## Key Components

### 1. Types and Interfaces

```typescript
export type HeatmapOptions = {
  terminalWidth?: number   // Terminal width in characters (default: 80)
  showMonthLabels?: boolean
}

type Percentiles = {
  p25: number  // 25th percentile (Q1)
  p50: number  // 50th percentile (median)
  p75: number  // 75th percentile (Q3)
}
```

**DailyActivity (imported):**
```typescript
// From ./stats.js
type DailyActivity = {
  date: string       // ISO date string "YYYY-MM-DD"
  messageCount: number
  // ... other fields
}
```

---

### 2. Percentile Calculation

```typescript
function calculatePercentiles(
  dailyActivity: DailyActivity[],
): Percentiles | null {
  const counts = dailyActivity
    .map(a => a.messageCount)
    .filter(c => c > 0)  // Exclude zero-activity days
    .sort((a, b) => a - b)

  if (counts.length === 0) return null

  return {
    p25: counts[Math.floor(counts.length * 0.25)]!,
    p50: counts[Math.floor(counts.length * 0.5)]!,
    p75: counts[Math.floor(counts.length * 0.75)]!,
  }
}
```

**Why Percentiles?**

Using percentiles instead of absolute thresholds makes the heatmap adaptive:
- Light users see variation in their lower activity range
- Heavy users need higher counts to reach max intensity
- Prevents "all max intensity" or "all min intensity" visuals

**Example:**
```
User A (light): [1, 2, 3, 5, 8, 12, 20] messages/day
  p25=2, p50=5, p75=12
  → 3 messages = intensity 2 (medium-low)

User B (heavy): [10, 25, 50, 80, 120, 200, 350] messages/day
  p25=25, p50=80, p75=200
  → 50 messages = intensity 2 (medium-low)
```

---

### 3. Main Heatmap Generator

```typescript
export function generateHeatmap(
  dailyActivity: DailyActivity[],
  options: HeatmapOptions = {},
): string {
  const { terminalWidth = 80, showMonthLabels = true } = options

  // Day labels take 4 characters ("Mon "), calculate weeks that fit
  // Cap at 52 weeks (1 year) to match GitHub style
  const dayLabelWidth = 4
  const availableWidth = terminalWidth - dayLabelWidth
  const width = Math.min(52, Math.max(10, availableWidth))
```

**Width Calculation:**
- Reserves 4 characters for day labels ("Mon ")
- Minimum 10 weeks, maximum 52 weeks (1 year)
- Adapts to terminal width dynamically

**Activity Map:**
```typescript
  // Build activity map by date for O(1) lookup
  const activityMap = new Map<string, DailyActivity>()
  for (const activity of dailyActivity) {
    activityMap.set(activity.date, activity)
  }

  // Pre-calculate percentiles once for all intensity lookups
  const percentiles = calculatePercentiles(dailyActivity)
```

**Date Range Calculation:**
```typescript
  // Calculate date range - end at today, go back N weeks
  const today = new Date()
  today.setHours(0, 0, 0, 0)

  // Find the Sunday of the current week (start of the week containing today)
  const currentWeekStart = new Date(today)
  currentWeekStart.setDate(today.getDate() - today.getDay())

  // Go back (width - 1) weeks from the current week start
  const startDate = new Date(currentWeekStart)
  startDate.setDate(startDate.getDate() - (width - 1) * 7)
```

**Grid Initialization:**
```typescript
  // Generate grid (7 rows for days of week, width columns for weeks)
  // Also track which week each month starts for labels
  const grid: string[][] = Array.from({ length: 7 }, () =>
    Array(width).fill(''),
  )
  const monthStarts: { month: number; week: number }[] = []
  let lastMonth = -1
```

**Grid Population:**
```typescript
  const currentDate = new Date(startDate)
  for (let week = 0; week < width; week++) {
    for (let day = 0; day < 7; day++) {
      // Don't show future dates
      if (currentDate > today) {
        grid[day]![week] = ' '
        currentDate.setDate(currentDate.getDate() + 1)
        continue
      }

      const dateStr = toDateString(currentDate)
      const activity = activityMap.get(dateStr)

      // Track month changes (on day 0 = Sunday of each week)
      if (day === 0) {
        const month = currentDate.getMonth()
        if (month !== lastMonth) {
          monthStarts.push({ month, week })
          lastMonth = month
        }
      }

      // Determine intensity level based on message count
      const intensity = getIntensity(activity?.messageCount || 0, percentiles)
      grid[day]![week] = getHeatmapChar(intensity)

      currentDate.setDate(currentDate.getDate() + 1)
    }
  }
```

**Grid Layout:**
```
Week 0    Week 1    Week 2    ...   Week 51
  │         │         │               │
  ▼         ▼         ▼               ▼
Sun [·][·][·][·]...[·][·][·][·]...[·][█][·][·]  Sat
Mon [·][·][·][·]...[·][·][·][·]...[·][█][·][·]  Fri
...
```

Note: The grid is stored as `grid[day][week]` but rendered as rows (days) across columns (weeks).

---

### 4. Month Labels

```typescript
  // Month labels - evenly spaced across the grid
  if (showMonthLabels) {
    const monthNames = [
      'Jan', 'Feb', 'Mar', 'Apr', 'May', 'Jun',
      'Jul', 'Aug', 'Sep', 'Oct', 'Nov', 'Dec',
    ]

    // Build label line with fixed-width month labels
    const uniqueMonths = monthStarts.map(m => m.month)
    const labelWidth = Math.floor(width / Math.max(uniqueMonths.length, 1))
    const monthLabels = uniqueMonths
      .map(month => monthNames[month]!.padEnd(labelWidth))
      .join('')

    // 4 spaces for day label column prefix
    lines.push('    ' + monthLabels)
  }
```

**Example Output:**
```
    Jan         Feb         Mar         Apr
```

---

### 5. Day Labels and Grid

```typescript
  // Day labels
  const dayLabels = ['Sun', 'Mon', 'Tue', 'Wed', 'Thu', 'Fri', 'Sat']

  // Grid - only show labels for Mon, Wed, Fri
  for (let day = 0; day < 7; day++) {
    const label = [1, 3, 5].includes(day) ? dayLabels[day]!.padEnd(3) : '   '
    const row = label + ' ' + grid[day]!.join('')
    lines.push(row)
  }
```

**Example Output:**
```
    Jan  Feb  Mar
       ··········
Mon  ···░░░▒▒▓▓██
       ··········
Wed  ···░░░▒▒▓▓██
       ··········
Fri  ···░░░▒▒▓▓██
       ··········
```

---

### 6. Legend

```typescript
  // Legend
  lines.push('')
  lines.push(
    '    Less ' +
      [
        claudeOrange('░'),
        claudeOrange('▒'),
        claudeOrange('▓'),
        claudeOrange('█'),
      ].join(' ') +
      ' More',
  )
```

**Example Output:**
```
    Less ░ ▒ ▓ █ More
```

---

### 7. Intensity Calculation

```typescript
function getIntensity(
  messageCount: number,
  percentiles: Percentiles | null,
): number {
  if (messageCount === 0 || !percentiles) return 0

  if (messageCount >= percentiles.p75) return 4
  if (messageCount >= percentiles.p50) return 3
  if (messageCount >= percentiles.p25) return 2
  return 1
}
```

**Intensity Levels:**
| Level | Threshold | Character | Description |
|-------|-----------|-----------|-------------|
| 0 | `messageCount === 0` | `·` (gray) | No activity |
| 1 | `< p25` | `░` (light) | Low activity |
| 2 | `>= p25, < p50` | `▒` (medium) | Medium-low activity |
| 3 | `>= p50, < p75` | `▓` (dark) | Medium-high activity |
| 4 | `>= p75` | `█` (solid) | High activity |

---

### 8. Color Functions

```typescript
// Claude orange color (hex #da7756)
const claudeOrange = chalk.hex('#da7756')

function getHeatmapChar(intensity: number): string {
  switch (intensity) {
    case 0:
      return chalk.gray('·')    // Middle dot
    case 1:
      return claudeOrange('░')  // Light shade
    case 2:
      return claudeOrange('▒')  // Medium shade
    case 3:
      return claudeOrange('▓')  // Dark shade
    case 4:
      return claudeOrange('█')  // Full block
    default:
      return chalk.gray('·')
  }
}
```

**Unicode Characters:**
| Char | Code | Name | Visual |
|------|------|------|--------|
| `·` | U+00B7 | Middle Dot | Small dot |
| `░` | U+2591 | Light Shade | ~25% fill |
| `▒` | U+2592 | Medium Shade | ~50% fill |
| `▓` | U+2593 | Dark Shade | ~75% fill |
| `█` | U+2588 | Full Block | 100% fill |

---

## Complete Output Example

```
    Jan         Feb         Mar         Apr
       ······································
Mon  ·······░░░░░▒▒▒▒▒▓▓▓▓▓█████████████████
       ······································
Wed  ·······░░░░░▒▒▒▒▒▓▓▓▓▓█████████████████
       ······································
Fri  ·······░░░░░▒▒▒▒▒▓▓▓▓▓█████████████████

    Less ░ ▒ ▓ █ More
```

---

## Algorithm Flow

```
generateHeatmap(dailyActivity, options)
         │
         ▼
  Calculate terminal width (min 10, max 52 weeks)
         │
         ▼
  Build activityMap (date → DailyActivity)
         │
         ▼
  Calculate percentiles (p25, p50, p75)
         │
         ▼
  Calculate date range (today - N weeks → today)
         │
         ▼
  For each week, for each day:
         │     │
         │     ├──► Skip future dates → ' '
         │     │
         │     ├──► Track month changes (for labels)
         │     │
         │     └──► Get intensity → getHeatmapChar() → grid[day][week]
         │
         ▼
  Build output lines:
         │
         ├──► Month labels (evenly spaced)
         │
         ├──► Day rows (Mon, Wed, Fri labeled)
         │
         └──► Legend (Less ░ ▒ ▓ █ More)
         │
         ▼
  Return joined string
```

---

## Design Decisions

### 1. Adaptive Thresholds (Percentiles)

**Problem:** Fixed thresholds (e.g., 10 messages = max intensity) don't scale:
- Light users: everything looks empty
- Heavy users: everything looks maxed

**Solution:** Use percentiles from the user's own data:
- Top 25% of activity days = max intensity
- Bottom 25% (non-zero) = min intensity
- Relative comparison within user's pattern

### 2. Week-Based Grid

**Problem:** Calendar months have varying lengths (28-31 days), making alignment complex.

**Solution:** Fixed 7-day weeks, like GitHub:
- Consistent column width
- Easy to count "weeks ago"
- Month labels as approximate markers

### 3. Terminal Width Adaptation

**Problem:** Users have different terminal sizes (80 cols, 120 cols, maximized).

**Solution:** Dynamic width calculation:
```typescript
const width = Math.min(52, Math.max(10, terminalWidth - 4))
```
- Minimum 10 weeks (visible pattern)
- Maximum 52 weeks (1 year, GitHub style)
- Subtracts 4 for day labels

### 4. Claude Orange Brand Color

**Decision:** Use `#da7756` (Claude's brand orange) instead of GitHub's green.

**Why:** Brand consistency, visual distinction from GitHub.

### 5. Sparse Day Labels

**Decision:** Only label Mon, Wed, Fri (not all 7 days).

**Why:** Reduces visual clutter while maintaining orientation.

---

## Integration Points

### With stats.ts

```typescript
// DailyActivity comes from stats module
import type { DailyActivity } from './stats.js'

// Typical usage:
const dailyActivity = computeDailyActivity(messages)
const heatmap = generateHeatmap(dailyActivity, { terminalWidth: 100 })
console.log(heatmap)
```

### With statsCache.ts

```typescript
import { toDateString } from './statsCache.js'

// Converts Date to "YYYY-MM-DD" string for Map lookup
const dateStr = toDateString(currentDate)  // "2026-04-07"
const activity = activityMap.get(dateStr)
```

---

## Related Files

**Module Documentation:**
- [utils/exploration.md](../utils/exploration.md) — Utils module overview

**Related Files:**
- `./stats.ts` — Activity computation
- `./statsCache.ts` — Date utilities, caching
- `./chalk` — Terminal colorization library

---

*Deep dive created: 2026-04-07*
