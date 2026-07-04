# Background Effects

Full-page animated background components.

## Aurora (177 lines)

Aurora borealis gradient animation using CSS gradients and animation.

Creates flowing, color-sh gradient bands that mimic the northern lights effect.

## BigBack (126 lines)

Large animated background element. Uses CSS transforms and opacity animations for a subtle parallax-like effect.

## Goldeneye (187 lines)

Golden ratio spiral animation. Draws a Fibonacci spiral that rotates continuously.

Aha: The spiral uses CSS `conic-gradient` rotated via `@keyframes`, not canvas. This means it's GPU-accelerated and compositable.

## NodeGraphBackground (270 lines)

Animated particle graph with connected nodes — dots connected by lines when within proximity.

```tsx
// Uses requestAnimationFrame loop
// Particles move with velocity
// Lines drawn between nearby particles
// Mouse interaction: particles repel from cursor
```

### Particle System

- **Count**: Configurable particle count
- **Speed**: Velocity magnitude
- **Connection distance**: Lines drawn when particles are within threshold
- **Mouse interaction**: Particles push away from cursor

Aha: The particle system uses a simple O(n²) proximity check for connections. For the typical count (~50 particles), this is 2,500 checks per frame — well within 60fps budget. For higher counts, a spatial hash or quadtree would be needed.

## FloatingSparkles (78 lines)

Floating sparkle particles with random positions, sizes, and animation delays.

## Sparkle (34 lines)

Single sparkle particle component.

## WibblingSpinner (155 lines)

Spinning animation with a "wibble" effect — the rotation isn't perfectly smooth but has a slight oscillation.

## useCounter Hook (39 lines)

```tsx
export function useCounter(target: number, duration: number = 2000) {
  const [count, setCount] = useState(0)
  // Animate from 0 to target over duration ms
}
```

## Cross-References

- [Overview](00-overview.md) — All components
- [Hero Components](01-hero-components.md) — Hero sections
