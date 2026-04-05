# Zero to Pretext: Complete Guide

**Last Updated:** 2026-04-05

---

## Table of Contents

1. [Introduction](#introduction)
2. [The Problem](#the-problem)
3. [What is Pretext?](#what-is-pretext)
4. [Installation](#installation)
5. [Quick Start](#quick-start)
6. [API Reference](#api-reference)
7. [Use Cases](#use-cases)
8. [Internationalization](#internationalization)
9. [Performance](#performance)
10. [Limitations](#limitations)

---

## Introduction

**Pretext** is a pure JavaScript/TypeScript library for multiline text measurement and layout. It provides fast, accurate text dimensions without triggering browser layout reflows.

```bash
npm install @chenglou/pretext
```

### Why Pretext?

Traditional DOM-based text measurement:
- Triggers synchronous layout reflow
- Costs 30ms+ per frame for 500 text blocks
- Blocks main thread during rendering

Pretext solution:
- Pure arithmetic after one-time preparation
- Costs ~0.09ms for 500 texts
- Zero DOM reads during resize/scroll

---

## The Problem

### DOM Measurement Interleaving

When components independently measure text:

```typescript
// Anti-pattern: DOM measurement triggers reflow
const heights = elements.map(el => {
  return el.getBoundingClientRect().height  // Forces layout!
})
```

Each read forces **synchronous layout**:

```
JavaScript → Style → Layout → Paint → Composite
                   ↑
             getBoundingClientRect()
             forces full document reflow
```

### Real-World Impact

```
Virtual list with 500 items:
- DOM measurement: 30-50ms per frame (janky)
- Pretext: ~0.1ms per frame (smooth 60fps)
```

---

## What is Pretext?

### Two-Phase Architecture

```
┌─────────────────────────────────────────────────────────┐
│                    Pretext Architecture                  │
│                                                          │
│  Phase 1: prepare(text, font)                           │
│  - Segment text via Intl.Segmenter                      │
│  - Measure segments with canvas.measureText()           │
│  - Cache widths per font                                │
│  - Time: ~19ms for 500 texts                            │
│                                                          │
│  Phase 2: layout(prepared, maxWidth, lineHeight)        │
│  - Walk cached widths with pure arithmetic              │
│  - No DOM reads, no canvas calls                        │
│  - Time: ~0.09ms for 500 texts                          │
└─────────────────────────────────────────────────────────┘
```

### Key Features

| Feature | Description |
|---------|-------------|
| **i18n Support** | CJK, Arabic, Thai, emoji, mixed bidi |
| **Canvas Measurement** | Uses browser's font engine directly |
| **Emoji Correction** | Auto-detects canvas/DOM discrepancy |
| **whitespace: pre-wrap** | Preserves spaces, tabs, newlines |
| **Soft Hyphens** | Invisible until break point |
| **Kinsoku Shori** | Japanese line-breaking rules |

---

## Installation

### npm/yarn

```bash
npm install @chenglou/pretext
# or
yarn add @chenglou/pretext
# or
pnpm add @chenglou/pretext
```

### CDN

```html
<script type="module">
  import { prepare, layout } from 'https://esm.sh/@chenglou/pretext'
</script>
```

### Development Setup

```bash
# Clone repository
git clone https://github.com/chenglou/pretext.git
cd pretext

# Install dependencies
bun install

# Start dev server
bun start

# Open demos at http://localhost:3210/demos
```

---

## Quick Start

### Basic Height Measurement

```typescript
import { prepare, layout } from '@chenglou/pretext'

// Phase 1: Prepare (do once when text changes)
const prepared = prepare(
  'AGI 春天到了。Began the journey 🚀',
  '16px Inter'
)

// Phase 2: Layout (call on every resize)
const { height, lineCount } = layout(prepared, 320, 20)

console.log(`Text is ${height}px tall across ${lineCount} lines`)
```

### React Integration

```typescript
import { prepare, layout } from '@chenglou/pretext'
import { useMemo, useState, useEffect } from 'react'

function TextBlock({ text, maxWidth, fontSize }) {
  // Prepare once when text/font changes
  const prepared = useMemo(
    () => prepare(text, `${fontSize}px Inter`),
    [text, fontSize]
  )
  
  // Layout on resize (pure arithmetic)
  const [dimensions, setDimensions] = useState(() => 
    layout(prepared, maxWidth, fontSize * 1.5)
  )
  
  useEffect(() => {
    const handleResize = () => {
      // Recalculate on container resize
      setDimensions(layout(prepared, maxWidth, fontSize * 1.5))
    }
    
    window.addEventListener('resize', handleResize)
    return () => window.removeEventListener('resize', handleResize)
  }, [prepared, maxWidth, fontSize])
  
  return (
    <div style={{ height: dimensions.height }}>
      {text}
    </div>
  )
}
```

### Virtual List Example

```typescript
import { prepare, layout } from '@chenglou/pretext'

function VirtualList({ items }) {
  // Pre-measure all items
  const measurements = items.map(item => {
    const prepared = prepare(item.text, '16px Inter')
    return layout(prepared, 300, 24)
  })
  
  // Calculate total height
  const totalHeight = measurements.reduce(
    (sum, m) => sum + m.height, 0
  )
  
  // Render only visible items
  return (
    <div style={{ height: totalHeight, overflow: 'auto' }}>
      {items.map((item, i) => (
        <div 
          key={i}
          style={{ 
            height: measurements[i].height,
            position: 'absolute',
            top: measurements.slice(0, i).reduce((s, m) => s + m.height, 0)
          }}
        >
          {item.text}
        </div>
      ))}
    </div>
  )
}
```

---

## API Reference

### prepare()

```typescript
function prepare(
  text: string,
  font: string,
  options?: {
    whiteSpace?: 'normal' | 'pre-wrap'
  }
): PreparedText

// Examples
prepare('Hello world', '16px Inter')
prepare('Hello\nworld', '16px Inter', { whiteSpace: 'pre-wrap' })
prepare('Tabs\there', '14px monospace', { whiteSpace: 'pre-wrap' })
```

### layout()

```typescript
function layout(
  prepared: PreparedText,
  maxWidth: number,
  lineHeight: number
): {
  height: number
  lineCount: number
}

// Example
const prepared = prepare('Long text...', '16px Inter')
const { height, lineCount } = layout(prepared, 320, 24)
```

### prepareWithSegments()

```typescript
function prepareWithSegments(
  text: string,
  font: string,
  options?: { whiteSpace?: 'normal' | 'pre-wrap' }
): PreparedTextWithSegments

// Returns richer structure for manual layout
const prepared = prepareWithSegments('Hello world', '16px Inter')
```

### layoutWithLines()

```typescript
function layoutWithLines(
  prepared: PreparedTextWithSegments,
  maxWidth: number,
  lineHeight: number
): {
  height: number
  lineCount: number
  lines: LayoutLine[]
}

// Example
const prepared = prepareWithSegments('Hello world', '16px Inter')
const { lines } = layoutWithLines(prepared, 320, 24)

// lines: [
//   { text: 'Hello', width: 42.5, start: {...}, end: {...} },
//   { text: 'world', width: 37.2, start: {...}, end: {...} }
// ]
```

### walkLineRanges()

```typescript
function walkLineRanges(
  prepared: PreparedTextWithSegments,
  maxWidth: number,
  onLine: (line: LayoutLineRange) => void
): number

// Non-materializing walker (no string building)
let maxW = 0
walkLineRanges(prepared, 320, line => {
  if (line.width > maxW) maxW = line.width
})
// maxW = tightest container width
```

### layoutNextLine()

```typescript
function layoutNextLine(
  prepared: PreparedTextWithSegments,
  start: LayoutCursor,
  maxWidth: number
): LayoutLine | null

// Iterator for custom layout logic
let cursor = { segmentIndex: 0, graphemeIndex: 0 }
let y = 0

while (true) {
  const line = layoutNextLine(prepared, cursor, getWidthAtY(y))
  if (line === null) break
  
  ctx.fillText(line.text, 0, y)
  cursor = line.end
  y += 24
}
```

### Helper Functions

```typescript
// Clear caches (free memory)
clearCache()

// Set locale for segmentation
setLocale('ja-JP')

// Measure without building lines
measureLineGeometry(prepared, maxWidth): { lineCount, maxLineWidth }

// Get intrinsic width
measureNaturalWidth(prepared): number
```

---

## Use Cases

### Virtual Scrolling

```typescript
// Measure all items once
const itemHeights = items.map(item => {
  const prepared = prepare(item.text, '16px system-ui')
  return layout(prepared, containerWidth, 24).height
})

// Calculate scroll height
const totalHeight = itemHeights.reduce((a, b) => a + b, 0)

// Render only visible window
const visibleItems = getVisibleItems(scrollTop, viewportHeight)
```

### Masonry Layout

```typescript
// Measure items to distribute across columns
const columns = Array.from({ length: 3 }, () => ({ height: 0, items: [] }))

items.forEach(item => {
  const prepared = prepare(item.text, '14px Inter')
  const height = layout(prepared, columnWidth, 20).height
  
  // Add to shortest column
  const shortest = columns.reduce((min, col) => 
    col.height < min.height ? col : min
  )
  shortest.height += height
  shortest.items.push(item)
})
```

### Textarea Auto-Height

```typescript
function AutoResizeTextarea({ value }) {
  const prepared = useMemo(
    () => prepare(value, '14px monospace', { whiteSpace: 'pre-wrap' }),
    [value]
  )
  
  const height = layout(prepared, textareaWidth, 21).height
  
  return (
    <textarea 
      value={value}
      style={{ height: height + 20 }}  // + padding
    />
  )
}
```

### SVG Text Layout

```typescript
// Layout text on SVG path
const prepared = prepareWithSegments(longText, '16px Georgia')
const { lines } = layoutWithLines(prepared, pathLength, 20)

lines.forEach((line, i) => {
  const x = path.getPointAtLength(i * lineSpacing).x
  const y = path.getPointAtLength(i * lineSpacing).y
  
  svg.appendChild(createTextElement(line.text, x, y))
})
```

### Canvas Rendering

```typescript
// Custom text layout engine
function renderText(ctx, text, x, y, maxWidth, lineHeight) {
  const prepared = prepareWithSegments(text, '16px Inter')
  let cursor = { segmentIndex: 0, graphemeIndex: 0 }
  
  while (true) {
    const line = layoutNextLine(prepared, cursor, maxWidth)
    if (!line) break
    
    ctx.fillText(line.text, x, y)
    cursor = line.end
    y += lineHeight
  }
}
```

---

## Internationalization

### CJK Support

```typescript
// Chinese
prepare('春天到了，万物复苏', '18px PingFang SC')

// Japanese
prepare('羅生門', '18px Hiragino Mincho')

// Korean
prepare('운수 좋은 날', '18px Apple SD Gothic Neo')
```

### Arabic Support

```typescript
// Arabic with proper punctuation handling
prepare('بدأت الرحلة 🚀', '16px Arial')

// Right-to-left text
prepare('فيقول:وعليك السلام', '16px Arial')
```

### Mixed Script

```typescript
// Mixed English, Chinese, Emoji
prepare('AGI 春天到了。Began the journey 🚀', '16px Inter')

// Mixed Latin, Arabic, Emoji
prepare('Hello مرحبا 🌍 World', '16px Inter')
```

---

## Performance

### Benchmarks

```
500 texts, mixed content (Chrome M1):

prepare():
- Analysis: ~5ms
- Measurement: ~14ms
- Total: ~19ms
- Per text: ~0.038ms

layout():
- Total: ~0.09ms
- Per text: ~0.0002ms

DOM comparison:
- getBoundingClientRect x500: ~30-50ms
- Each forces full document reflow
```

### Memory

```
Segment cache per font:
- ~100KB for typical app
- Shared across all texts
- Clear with clearCache() when needed
```

---

## Limitations

### Supported CSS

Pretext targets common text configuration:

```css
white-space: normal (default)
white-space: pre-wrap (with option)
word-break: normal
overflow-wrap: break-word
line-break: auto
```

### Unsupported Features

- Complex CSS layouts (flexbox, grid inside text)
- Inline images
- Multi-column layout
- Custom line-break values
- `system-ui` font (use named fonts for accuracy)

### Known Browser Quirks

```typescript
// macOS: system-ui resolves differently in canvas vs DOM
// Chrome/Firefox: canvas measures emoji wider than DOM
// Safari: line-fit tolerance differs (1/64 vs 0.005)

// Solution: Use named fonts
prepare(text, '16px Inter')  // ✓
prepare(text, '16px system-ui')  // ✗ (unsafe for accuracy)
```

---

## Related Documents

- [Text Rendering Engine Deep Dive](./01-text-rendering-engine-deep-dive.md)
- [Cross-Browser Measurement Deep Dive](./02-cross-browser-measurement-deep-dive.md)
- [Internationalization Deep Dive](./03-i18n-text-layout-deep-dive.md)
- [Rust Revision](./rust-revision.md)
- [Production Guide](./production-grade.md)
