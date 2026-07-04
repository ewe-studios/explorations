# Logo Walls

Company logo display components.

## LogoMarquee (74 lines)

Scrolling logo marquee with infinite horizontal scroll animation.

```tsx
// Duplicates logos for seamless loop
// CSS animation for continuous scroll
// Pause on hover
```

## LogoRow (40 lines)

Static logo row/grid with grayscale-to-color on hover.

Aha: Both components accept an array of logo sources (URLs or React nodes). The marquee clones the array 3× to ensure seamless infinite scroll — the clones fill the viewport even on wide screens.

## Cross-References

- [Overview](00-overview.md) — All components
- [Interactive Components](03-interactive-components.md) — User interactions
- [Build System](05-build-system.md) — Library build, docs
