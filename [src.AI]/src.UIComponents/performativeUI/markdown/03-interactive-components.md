# Interactive Components Deep Dive

Temperature selector, BeforeAfter slider, Button, Popover, text animations, and hooks.

## Temperature (451 lines)

Source: `src/components/Temperature.tsx` — the largest single component.

Reasoning effort selector with 3 animated treatments: glow, rainbow, ludicrous.

### Default Ladder

```tsx
const DEFAULT_OPTIONS = [
  { key: "low", label: "low", color: "var(--pui-temp-low)" },
  { key: "medium", label: "medium", color: "var(--pui-temp-medium)" },
  { key: "high", label: "high", color: "var(--pui-temp-high)" },
  { key: "xhigh", label: "xhigh", color: "glow" },
  { key: "max", label: "max", color: "rainbow" },
  { key: "ludicrous", label: "ludicrous", color: "ludicrous" },
];
```

### Ludicrous Ripple

Full-component canvas rendering a violet radial cosine wave:

```ts
const RIPPLE_WAVELENGTH = 20;
const RIPPLE_TRAVEL_PER_FRAME = 80 * 0.03;  // 2.4 units/frame
const RIPPLE_FRAME_MS = 80;
const RIPPLE_ROW_WEIGHT = 2;   // Terminal cells ~2× taller than wide
const RIPPLE_CELL_PX = 9;
```

Distance weighted 2× vertically, cosine standing-wave on 8-step violet gradient:

```ts
const dx = (col - selectedCol) * RIPPLE_CELL_PX;
const dy = (row - baselineRow) * RIPPLE_CELL_PX * RIPPLE_ROW_WEIGHT;
const dist = Math.sqrt(dx * dx + dy * dy);
const lag = dist - travelDistance;
const phase = ((lag % RIPPLE_WAVELENGTH) + RIPPLE_WAVELENGTH) % RIPPLE_WAVELENGTH;
```

Aha: Mimics actual terminal behavior where the top reasoning tier floods the WHOLE menu rectangle with violet wave — not just the selected row. Canvas sized to entire component, layered behind text.

## BeforeAfter (110 lines)

Before/after slider with draggable divider.

```tsx
interface BeforeAfterProps {
  before: ReactNode;
  after: ReactNode;
  defaultValue?: number;  // 0-100 split position
}
```

## Button (93 lines)

```tsx
type ButtonVariant = "primary" | "secondary" | "ghost";
type ButtonSize = "sm" | "md" | "lg";
```

## Popover (169 lines)

```tsx
interface PopoverProps {
  trigger: ReactNode;
  content: ReactNode;
  placement?: "top" | "bottom" | "left" | "right";
  open?: boolean;
  onOpenChange?: (open: boolean) => void;
}
```

Uses `createPortal` for DOM-out-of-tree rendering.

## Text Animations

| Component | Lines | Technique |
|-----------|-------|-----------|
| `WordRoll` | 102 | CSS transform, slot-machine cycling |
| `Rotator` | 76 | CSS perspective + rotateX 3D carousel |
| `SlippyWords` | 149 | Staggered CSS slide-in |
| `GradientText` | 36 | background-clip: text + gradient animation |
| `QuestText` | 123 | Character-by-character RPG reveal |
| `TokenStream` | 59 | React state, streaming tokens |

## Hooks

| Hook | Lines | Purpose |
|------|-------|---------|
| `useTypewriter` | 92 | Char-by-char text reveal at `speed` ms intervals |
| `useCounter` | 39 | rAF easeOutQuad from 0 to target in `duration` ms |
| `useTokenStream` | 83 | Simulated AI token streaming with burst patterns |

### useTypewriter

```ts
function useTypewriter(text: string, speed = 50): string {
  const [displayed, setDisplayed] = useState('');
  useEffect(() => {
    let i = 0;
    const interval = setInterval(() => {
      setDisplayed(text.slice(0, ++i));
      if (i >= text.length) clearInterval(interval);
    }, speed);
    return () => clearInterval(interval);
  }, [text, speed]);
  return displayed;
}
```

### useCounter

```ts
function useCounter(target: number, duration = 2000): number {
  const [count, setCount] = useState(0);
  useEffect(() => {
    const start = performance.now();
    const tick = (now: number) => {
      const progress = Math.min((now - start) / duration, 1);
      const eased = 1 - (1 - progress) * (1 - progress);  // easeOutQuad
      setCount(Math.round(eased * target));
      if (progress < 1) requestAnimationFrame(tick);
    };
    requestAnimationFrame(tick);
  }, [target, duration]);
  return count;
}
```

## Cross-References

- [Overview](00-overview.md) — All components
- [Hero Components](01-hero-components.md) — Hero sections
- [Background Effects](02-background-effects.md) — Canvas backgrounds
- [Logo Walls](04-logo-walls.md) — Logo displays

[Next → Logo Walls](04-logo-walls.md)
