# Hero Components Deep Dive

The hero section components: AsciiHero (procedural ASCII art), MockIDE (code editor mock), Prompt (AI input), and PromptHero.

## AsciiHero

Source: `src/components/AsciiHero.tsx` — 101 lines. Wraps `useAsciiField` hook to render procedural ASCII art on `<canvas>`.

```tsx
export const AsciiHero = forwardRef<HTMLDivElement, AsciiHeroProps>(
  ({ className, ...rest }, ref) => {
    const hostRef = useRef<HTMLDivElement>(null);
    const canvasRef = useRef<HTMLCanvasElement>(null);
    useAsciiField(canvasRef, hostRef, { /* options */ });
    return (
      <div ref={ref} className={cn('pui-ascii-hero', className)}>
        <div ref={hostRef} className="pui-ascii-host">
          <canvas ref={canvasRef} className="pui-ascii-canvas" />
        </div>
      </div>
    );
  }
);
```

### useAsciiField Hook

Source: `src/hooks/useAsciiField.ts` — 265 lines.

```ts
const DEFAULT_RAMP = " .`'\",:;Il!i><~+_-?][}{1)(|/tfjrxnuvczXYUJCLQ0OZmwqpdbkhao*#MW&8%B@$";
const DEFAULT_PALETTE = ["#a78bfa", "#ec4899", "#67e8f9", "#fbbf24"];
```

Options:
| Option | Default | Purpose |
|--------|---------|---------|
| `fontSize` | 11 | Character size in px |
| `fontFamily` | JetBrains Mono | Must be monospace |
| `charRamp` | 72 chars | Sparsest→densest |
| `colorful` | false | Enable aurora palette |
| `baseOpacity` | 1 | Base alpha |
| `reactive` | true | Cursor effects |
| `rippleStrength` | 1.4 | Ripple amplitude |
| `rippleRadius` | 6 | Ripple falloff in cells |
| `spotlightRadius` | 8 | Cursor spotlight in cells |
| `frameMs` | 50 | Throttle (20fps) |

Grid is computed from container size ÷ font metrics:
```ts
cols = Math.ceil(rect.width / cellW);
rows = Math.ceil(rect.height / cellH);
baseField = new Float32Array(cols * rows);
```

Procedural field uses sine waves:
```ts
const nx = (x / cols) * 2 - 1;
const ny = (y / rows) * 2 - 1;
const r = Math.sqrt(nx * nx + ny * ny);
const stripes = 0.5 + 0.5 * Math.sin(nx * 6 + ny * 2);
baseField[y * cols + x] = stripes * (1 - r * 0.3);
```

Aha: The charRamp has 72 characters ordered from sparse (space) to dense (@, $, #). This maps field values 0-1 to ASCII characters for smooth density gradients. Most ASCII art uses 10 chars; 72 gives finer resolution.

Cursor effects: ripple wave propagates outward from mouse position, spotlight increases opacity near cursor.

## MockIDE

Source: `src/components/MockIDE.tsx` — 170 lines.

```tsx
interface MockIDEProps {
  filename?: string;
  tokens?: IdeToken[];
  loop?: boolean;          // default true
  charMs?: [number, number];  // default [14, 42]
  thinkingLabel?: ReactNode | false;
}

interface IdeToken {
  c: string;          // Raw text including whitespace
  cls?: IdeTokenClass; // 'key' | 'str' | 'num' | 'com' | 'fn' | ''
}
```

Typing speed randomized: `charMs[0] + Math.random() * (charMs[1] - charMs[0])` → 14-42ms per character.

Aha: IDE enforces `data-theme="dark"` regardless of page theme — code editors are universally dark.

## Prompt

Source: `src/components/Prompt.tsx` — 268 lines.

Multi-line AI prompt with model dropdown, context button, voice button, send arrow.

```tsx
interface PromptProps {
  value?: string;
  defaultValue?: string;
  onChange?: (value: string) => void;
  onSubmit?: (value: string, ctx: { model?: string }) => void;
  models?: string[];   // default: ["GPT-5 Turbo Vision", "Claude Opus 4.7", "Gemini 3 Pro"]
  rows?: number;       // default 3
  submitOnCmdEnter?: boolean;  // default true
  hideAddContext?: boolean;
  hideModel?: boolean;
  hideVoice?: boolean;
  hideSend?: boolean;
  toolbarExtras?: ReactNode;
}
```

Controlled/uncontrolled pattern for both text value and model selection. `submitOnCmdEnter` captures Cmd/Ctrl+Enter in the textarea.

## PromptHero

Source: `src/components/PromptHero.tsx` — 91 lines. Wraps Prompt with centered hero layout, optional title/subtitle, gradient background.

## Cross-References

- [Overview](00-overview.md) — All components
- [Background Effects](02-background-effects.md) — Canvas backgrounds
- [Interactive Components](03-interactive-components.md) — User interactions

[Next → Background Effects](02-background-effects.md)
