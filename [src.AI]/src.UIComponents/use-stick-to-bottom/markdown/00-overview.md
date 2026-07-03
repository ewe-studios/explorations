# What Is use-stick-to-bottom?

A lightweight React Hook for smoothly sticking to the bottom of scrollable containers — ideal for AI chat applications.

**Source:** `/home/darkvoid/Boxxed/@formulas/src.rust/src.AI/src.UIComponents/use-stick-to-bottom/`

## Installation

```bash
npm install use-stick-to-bottom
```

## Usage

```tsx
import { useStickToBottom } from 'use-stick-to-bottom';

function Chat() {
  const { containerRef, contentRef, isSticky } = useStickToBottom();
  return (
    <div ref={containerRef} style={{ overflow: 'auto' }}>
      <div ref={contentRef}>{/* messages */}</div>
    </div>
  );
}
```

## API

| Export | Type |
|--------|------|
| `useStickToBottom` | Hook |
| `StickToBottom` | Component wrapper |

## Source Files

| File | Purpose |
|------|---------|
| `src/useStickToBottom.ts` | Main hook |
| `src/StickToBottom.tsx` | Component wrapper |
| `src/index.ts` | Barrel export |
