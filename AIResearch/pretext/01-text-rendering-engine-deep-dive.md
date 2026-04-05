# AIResearch Pretext: Text Rendering Engine Deep Dive

**Last Updated:** 2026-04-05

---

## Table of Contents

1. [Overview](#overview)
2. [The Problem: DOM Measurement Interleaving](#the-problem-dom-measurement-interleaving)
3. [Two-Phase Architecture](#two-phase-architecture)
4. [Text Segmentation](#text-segmentation)
5. [Canvas Measurement](#canvas-measurement)
6. [Line Breaking Algorithm](#line-breaking-algorithm)
7. [Internationalization](#internationalization)
8. [Browser Quirks](#browser-quirks)
9. [Performance Characteristics](#performance-characteristics)

---

## Overview

Pretext is a **pure JavaScript/TypeScript text measurement library** that solves the problem of DOM-based text measurement triggering expensive layout reflows. It implements a two-phase architecture:

1. **prepare()** - Segment text, measure with canvas, cache widths
2. **layout()** - Walk cached widths with pure arithmetic

```typescript
import { prepare, layout } from '@chenglou/pretext'

// Phase 1: One-time work (~19ms for 500 texts)
const prepared = prepare('AGI 春天到了。Began the journey 🚀', '16px Inter')

// Phase 2: Pure arithmetic (~0.09ms for 500 texts)
const { height, lineCount } = layout(prepared, 320, 20)
```

---

## The Problem: DOM Measurement Interleaving

### Layout Reflow Cost

When UI components measure text with DOM APIs:

```typescript
// Expensive: Forces synchronous layout
const height = element.getBoundingClientRect().height
const width = element.offsetWidth
```

Each read can trigger a **full document reflow**:

```
┌─────────────────────────────────────────────────────────┐
│                  Browser Rendering Pipeline              │
│                                                          │
│  JavaScript ──► Style ──► Layout ──► Paint ──► Composite │
│                   ↑          ↑                           │
│                   │          │                           │
│              getBoundingClientRect() forces              │
│              synchronous Layout (reflow)                 │
└─────────────────────────────────────────────────────────┘
```

### Cost at Scale

For 500 text blocks:
- **DOM measurement**: 30ms+ per frame (layout thrashing)
- **Pretext**: ~0.0002ms per text (cached arithmetic)

---

## Two-Phase Architecture

### Phase 1: prepare()

```typescript
// Source: src/layout.ts

export function prepare(text: string, font: string, options?: PrepareOptions): PreparedText {
  const startTime = performance.now()
  
  // 1. Normalize whitespace
  const normalized = normalizeWhitespace(text, options?.whiteSpace)
  
  // 2. Segment text via Intl.Segmenter
  const analysis = analyzeText(normalized, font)
  
  // 3. Measure segments with canvas
  const measured = measureAnalysis(analysis, font, false)
  
  // 4. Return opaque handle
  return measured as unknown as PreparedText
}
```

### Phase 2: layout()

```typescript
// Source: src/layout.ts

export function layout(prepared: PreparedText, maxWidth: number, lineHeight: number): LayoutResult {
  // Pure arithmetic - no DOM reads, no canvas calls
  const internal = prepared as unknown as InternalPreparedText
  
  // Walk cached widths
  let lineCount = 0
  let currentLineWidth = 0
  let totalHeight = 0
  
  for (let i = 0; i < internal.widths.length; i++) {
    const width = internal.widths[i]
    const kind = internal.kinds[i]
    
    if (kind === 'hard-break') {
      lineCount++
      totalHeight += lineHeight
      currentLineWidth = 0
      continue
    }
    
    if (currentLineWidth + width > maxWidth) {
      lineCount++
      totalHeight += lineHeight
      currentLineWidth = width
    } else {
      currentLineWidth += width
    }
  }
  
  // Account for last line
  if (currentLineWidth > 0) {
    lineCount++
    totalHeight += lineHeight
  }
  
  return { lineCount, height: totalHeight }
}
```

---

## Text Segmentation

### Intl.Segmenter

Pretext uses the [Intl.Segmenter](https://developer.mozilla.org/en-US/docs/Web/JavaScript/Reference/Global_Objects/Intl/Segmenter) API for i18n-aware segmentation:

```typescript
// Source: src/analysis.ts

const sharedWordSegmenter = new Intl.Segmenter(undefined, { granularity: 'word' })

function segmentText(text: string): SegmentationPiece[] {
  const segments = []
  
  for (const { segment, index } of sharedWordSegmenter.segment(text)) {
    segments.push({
      text: segment,
      isWordLike: /* computed */,
      kind: classifySegment(segment),
      start: index
    })
  }
  
  return segments
}
```

### Segment Break Kinds

```typescript
// Source: src/analysis.ts

export type SegmentBreakKind =
  | 'text'              // Regular word/text
  | 'space'             // Collapsible whitespace
  | 'preserved-space'   // Preserved space (pre-wrap)
  | 'tab'               // Tab character
  | 'glue'              // Non-breaking glue (NBSP)
  | 'zero-width-break'  // ZWSP break opportunity
  | 'soft-hyphen'       // Soft hyphen (SHY)
  | 'hard-break'        // Newline character
```

### Punctuation Merging

Punctuation merges with preceding words:

```typescript
// Source: src/analysis.ts

export const leftStickyPunctuation = new Set([
  '.', ',', '!', '?', ':', ';',
  '\u060C',  // Arabic comma
  '\u061B',  // Arabic semicolon
  '\u0964',  // Devanagari danda
  ')', ']', '}',
  '%',
  '"', '"', "'", ''',
])

function isLeftStickyPunctuationSegment(segment: string): boolean {
  for (const ch of segment) {
    if (!leftStickyPunctuation.has(ch)) return false
  }
  return segment.length > 0
}
```

---

## Canvas Measurement

### measureText API

```typescript
// Source: src/measurement.ts

const measureContext: CanvasRenderingContext2D | null = null

function getMeasureContext(): CanvasRenderingContext2D {
  if (measureContext !== null) return measureContext
  
  if (typeof OffscreenCanvas !== 'undefined') {
    measureContext = new OffscreenCanvas(1, 1).getContext('2d')!
    return measureContext
  }
  
  if (typeof document !== 'undefined') {
    measureContext = document.createElement('canvas').getContext('2d')!
    return measureContext
  }
  
  throw new Error('Text measurement requires OffscreenCanvas or DOM canvas')
}

function measureSegment(seg: string, font: string): number {
  const ctx = getMeasureContext()
  ctx.font = font
  return ctx.measureText(seg).width
}
```

### Segment Metrics Cache

```typescript
// Source: src/measurement.ts

const segmentMetricCaches = new Map<string, Map<string, SegmentMetrics>>()

export function getSegmentMetricCache(font: string): Map<string, SegmentMetrics> {
  let cache = segmentMetricCaches.get(font)
  if (!cache) {
    cache = new Map()
    segmentMetricCaches.set(font, cache)
  }
  return cache
}

export function getSegmentMetrics(seg: string, cache: Map<string, SegmentMetrics>): SegmentMetrics {
  let metrics = cache.get(seg)
  if (metrics === undefined) {
    const ctx = getMeasureContext()
    metrics = {
      width: ctx.measureText(seg).width,
      containsCJK: isCJK(seg)
    }
    cache.set(seg, metrics)
  }
  return metrics
}
```

### Emoji Correction

Chrome/Firefox canvas measures emoji wider than DOM:

```typescript
// Source: src/measurement.ts

const emojiCorrectionCache = new Map<string, number>()

function getEmojiCorrection(font: string, fontSize: number): number {
  let correction = emojiCorrectionCache.get(font)
  if (correction !== undefined) return correction
  
  const ctx = getMeasureContext()
  ctx.font = font
  const canvasW = ctx.measureText('\u{1F600}').width  // Grinning face
  
  correction = 0
  if (canvasW > fontSize + 0.5 && typeof document !== 'undefined') {
    const span = document.createElement('span')
    span.style.font = font
    span.style.display = 'inline-block'
    span.style.visibility = 'hidden'
    span.style.position = 'absolute'
    span.textContent = '\u{1F600}'
    document.body.appendChild(span)
    
    const domW = span.getBoundingClientRect().width
    document.body.removeChild(span)
    
    if (canvasW - domW > 0.5) {
      correction = canvasW - domW
    }
  }
  
  emojiCorrectionCache.set(font, correction)
  return correction
}
```

---

## Line Breaking Algorithm

### Greedy Line Breaker

```typescript
// Source: src/line-break.ts

function walkPreparedLines(
  prepared: InternalPreparedText,
  maxWidth: number,
  onLine: (line: InternalLayoutLine) => void
): number {
  let currentX = 0
  let lineStart = 0
  let lineText = ''
  
  for (let i = 0; i < prepared.widths.length; i++) {
    const width = prepared.widths[i]
    const kind = prepared.kinds[i]
    
    if (kind === 'hard-break') {
      // Force line break
      onLine({ text: lineText, width: currentX })
      currentX = 0
      lineStart = i + 1
      lineText = ''
      continue
    }
    
    if (currentX + width > maxWidth) {
      // Wrap to next line
      onLine({ text: lineText, width: currentX })
      currentX = width
      lineStart = i
      lineText = prepared.segments[i] || ''
    } else {
      currentX += width
      lineText += prepared.segments[i] || ''
    }
  }
  
  // Final line
  if (currentX > 0) {
    onLine({ text: lineText, width: currentX })
  }
  
  return 0
}
```

### Line-Fit Tolerance

Browser-specific tolerance for width matching:

```typescript
// Source: src/measurement.ts

export function getEngineProfile(): EngineProfile {
  if (typeof navigator === 'undefined') {
    return {
      lineFitEpsilon: 0.005
    }
  }
  
  const ua = navigator.userAgent
  const vendor = navigator.vendor
  
  const isSafari = vendor === 'Apple Computer, Inc.' && ua.includes('Safari/') && !ua.includes('Chrome/')
  
  return {
    // Safari uses 1/64, Chromium/Gecko use 0.005
    lineFitEpsilon: isSafari ? 1 / 64 : 0.005
  }
}
```

---

## Internationalization

### CJK Support

```typescript
// Source: src/analysis.ts

export function isCJK(s: string): boolean {
  for (const ch of s) {
    const c = ch.codePointAt(0)!
    if (
      (c >= 0x4E00 && c <= 0x9FFF) ||      // CJK Unified Ideographs
      (c >= 0x3400 && c <= 0x4DBF) ||      // CJK Extension A
      (c >= 0x20000 && c <= 0x2A6DF) ||    // CJK Extension B
      (c >= 0x30000 && c <= 0x3134F) ||    // CJK Extension G
      (c >= 0x3040 && c <= 0x309F) ||      // Hiragana
      (c >= 0x30A0 && c <= 0x30FF) ||      // Katakana
      (c >= 0xAC00 && c <= 0xD7AF)         // Hangul Syllables
    ) {
      return true
    }
  }
  return false
}
```

### Kinsoku Shori (Japanese Line Breaking)

```typescript
// Source: src/analysis.ts

export const kinsokuStart = new Set([
  '\uFF0C',  // Ideographic comma
  '\uFF0E',  // Fullwidth full stop
  '\u3001',  // Ideographic comma
  '\u3002',  // Ideographic full stop
  '\u30FB',  // Katakana middle dot
  '\uFF09',  // Fullwidth right parenthesis
])

export const kinsokuEnd = new Set([
  '"', '(', '[', '{',
  '"', ''', '«', '‹',
])

// Kinsoku rules: certain characters cannot start/end lines
function applyKinsokuRules(lines: LayoutLine[]): LayoutLine[] {
  // Implementation applies kinsoku merging
  return lines
}
```

### Arabic Support

```typescript
// Source: src/analysis.ts

const arabicNoSpaceTrailingPunctuation = new Set([
  ':',
  '.',
  '\u060C',  // Arabic comma
  '\u061B',  // Arabic semicolon
])

function endsWithArabicNoSpacePunctuation(segment: string): boolean {
  if (!containsArabicScript(segment) || segment.length === 0) return false
  return arabicNoSpaceTrailingPunctuation.has(segment[segment.length - 1]!)
}

// Arabic punctuation merges with preceding word (no-space punctuation)
function mergeArabicPunctuation(segments: SegmentationPiece[]): SegmentationPiece[] {
  // Implementation merges Arabic punctuation clusters
  return mergedSegments
}
```

---

## Browser Quirks

### system-ui Font Mismatch

```typescript
// Source: RESEARCH.md

// Discovery: system-ui resolves to different optical variants on macOS
// Canvas uses SF Pro Display, DOM uses SF Pro Text at different thresholds

// Mismatches cluster at: 10-12px, 14px, 26px
// Exact matches at: 13px, 15-25px, 27-28px

// Recommendation: Use named fonts for accuracy
// "16px Inter" instead of "16px system-ui"
```

### Safari Canvas/DOM Discrepancy

Safari canvas and DOM agree on emoji width (unlike Chrome/Firefox):

```typescript
// Source: src/measurement.ts

// Safari: canvas W === DOM W (emoji correction = 0)
// Chrome/Firefox: canvas W > DOM W (emoji correction > 0)

function getEmojiCorrection(font: string, fontSize: number): number {
  // ... detection logic ...
  // Returns 0 for Safari, positive value for Chrome/Firefox
}
```

---

## Performance Characteristics

### prepare() Performance

```
500 texts, mixed content:
- Analysis phase: ~5ms
- Measurement phase: ~14ms
- Total: ~19ms

Per-text average: ~0.038ms
```

### layout() Performance

```
500 texts, mixed content:
- Total: ~0.09ms

Per-text average: ~0.0002ms
```

### Memory Usage

```
Segment metrics cache per font:
- Map<segment, metrics>
- ~100KB for typical app fonts
- Clear with clearCache() when needed
```

---

## Related Documents

- [Pretext Zero-to Guide](./00-zero-to-pretext.md)
- [Cross-Browser Measurement Deep Dive](./02-cross-browser-measurement-deep-dive.md)
- [Internationalization Deep Dive](./03-i18n-text-layout-deep-dive.md)
- [Rust Revision](./rust-revision.md)
