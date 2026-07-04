# Background Effects Deep Dive

Canvas-rendered and CSS-driven background components.

## NodeGraphBackground

Source: `src/components/NodeGraphBackground.tsx` — 270 lines.

Canvas particle simulation: drifting dots connected by proximity lines, with cursor gravity and brightening.

### Props

```tsx
interface NodeGraphBackgroundProps {
  density?: number;         // Default 70
  speed?: number;           // Default 0.4 px/frame
  linkDistance?: number;    // Default 140px
  colors?: string[];        // Default: purple/pink/cyan
  linkColor?: string;       // Default: "#7c3aed"
  hoverDistance?: number;   // Default 200px cursor radius
  hoverGravity?: number;    // Default 0.005 pull strength
  hoverBrighten?: number;   // Default 0.8 opacity boost
  baseOpacity?: number;     // Default 0.45
  overscan?: number;        // Default 80px off-screen world
}
```

### Simulation

```ts
type Node = { x: number; y: number; vx: number; vy: number; r: number; color: string };
```

Nodes seeded across world (viewport + overscan), bounce off world bounds.

### Connection Drawing

O(n²) proximity: for each pair of nodes, draw line if distance < linkDistance. Opacity fades with distance: `alpha = (1 - dist/linkDistance) * baseOpacity`.

Aha: O(n²) is fine for 70 nodes (2,450 checks/frame) but would need spatial hashing for 500+. The default 70 stays within 60fps on mobile.

### Cursor Interaction

```ts
// Gravity: pull nodes toward cursor
node.vx += dx * hoverGravity * (1 - dist / hoverDistance);
node.vy += dy * hoverGravity * (1 - dist / hoverDistance);

// Brighten: increase opacity near cursor
const brighten = hoverBrighten * (1 - dist / hoverDistance);
alpha = baseOpacity + brighten;
```

### Overscan

World extends `overscan` pixels past viewport. Nodes drift in from off-screen, creating illusion of infinite field.

## Goldeneye

Source: `src/components/Goldeneye.tsx` — 187 lines.

Golden ratio (137.508°) spiral animation:

```ts
const goldenAngle = Math.PI * (3 - Math.sqrt(5));  // ≈ 2.39996 rad
for (let i = 0; i < dots; i++) {
  const angle = i * goldenAngle + rotation;
  const radius = Math.sqrt(i) * spacing;
  const x = cx + Math.cos(angle) * radius;
  const y = cy + Math.sin(angle) * radius;
}
```

Produces the sunflower spiral pattern.

## Aurora

Source: `src/components/Aurora.tsx` — 177 lines.

CSS gradient animation with `@keyframes` on `background-position`. Flowing gradient bands mimicking aurora borealis.

## BigBack

Source: `src/components/BigBack.tsx` — 126 lines.

Footer-style background with columns of links and animated gradient.

```tsx
interface BigBackProps {
  columns?: BigBackColumn[];
  speed?: number;
}
```

## FloatingSparkles

Source: `src/components/FloatingSparkles.tsx` — 78 lines.

CSS-animated sparkle particles. Each sparkle is a `<div>` with float-up-and-fade `@keyframes`.

```tsx
interface FloatingSparklesProps {
  count?: number;   // Default 12
  color?: string;   // Default white
  size?: number;    // Default 8px
}
```

## Sparkle

Source: `src/components/Sparkle.tsx` — 34 lines. Single sparkle particle.

## WibblingSpinner

Source: `src/components/WibblingSpinner.tsx` — 155 lines.

Spinning loader with "wibble" oscillation. Rotating verbs and glyphs.

```tsx
// Default verbs: ["thinking", "generating", "computing", "reasoning"]
// Default glyphs: ["✦", "◈", "⬡", "◎", "❋"]
```

## Cross-References

- [Overview](00-overview.md) — All components
- [Hero Components](01-hero-components.md) — Hero sections
- [Interactive Components](03-interactive-components.md) — User interactions

[Next → Interactive Components](03-interactive-components.md)
