# use-stick-to-bottom: Chat Auto-Scroll Hook

Lightweight React Hook for smoothly sticking to the bottom of scrollable containers — ideal for AI chat applications.

**Source:** `/home/darkvoid/Boxxed/@formulas/src.rust/src.AI/src.UIComponents/use-stick-to-bottom/`

## API

```typescript
import { useStickToBottom } from 'use-stick-to-bottom'

function Chat() {
  const { containerRef, contentRef, isSticky } = useStickToBottom()
  return (
    <div ref={containerRef} style={{ overflow: 'auto' }}>
      <div ref={contentRef}>{/* messages */}</div>
    </div>
  )
}
```

## Exports

| Export | Type |
|--------|------|
| `useStickToBottom` | Hook |
| `StickToBottom` | Component wrapper |

## Source Files

| File | Purpose |
|------|---------|
| `src/useStickToBottom.ts` | Main hook implementation |
| `src/StickToBottom.tsx` | Component wrapper |
| `src/index.ts` | Barrel export |

## Options

| Option | Type | Default | Description |
|--------|------|---------|-------------|
| `smooth` | `boolean` | `true` | Use smooth scrolling |
| `threshold` | `number` | `50` | Pixel threshold for sticky detection |
