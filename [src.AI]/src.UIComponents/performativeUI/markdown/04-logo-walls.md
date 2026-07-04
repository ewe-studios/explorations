# Logo Walls Deep Dive

Company logo display components: LogoMarquee and LogoRow.

## LogoMarquee

Source: `src/components/LogoMarquee.tsx` — 74 lines.

Infinite-scrolling logo row. Duplicates logos 3× for seamless loop.

```tsx
interface LogoMarqueeProps {
  items: MarqueeItem[];
  speed?: number;       // px/second scroll speed
  gap?: number;         // Gap between logos
}

interface MarqueeItem {
  label: string;
  logo: ReactNode;
}
```

### Seamless Loop

```tsx
// Clone items 3× to fill viewport + scroll buffer
const tripled = [...items, ...items, ...items];
```

CSS animation scrolls the entire row leftward. At 1/3 of the total width, it snaps back to position 0 — invisible to the user because items 0 and N are identical.

```css
@keyframes marquee-scroll {
  from { transform: translateX(0); }
  to { transform: translateX(-33.333%); }
}
```

## LogoRow

Source: `src/components/LogoRow.tsx` — 40 lines.

Static logo grid with grayscale-to-color on hover.

```tsx
interface LogoRowProps {
  items: LogoRowItem[];
  columns?: number;     // Grid columns
}

interface LogoRowItem {
  label: string;
  logo: ReactNode;
  href?: string;
}
```

### Grayscale Filter

```css
.pui-logo-item img {
  filter: grayscale(1) opacity(0.5);
  transition: filter 0.2s ease;
}
.pui-logo-item:hover img {
  filter: grayscale(0) opacity(1);
}
```

Aha: Both components accept an array of logo sources. The marquee clones 3× to ensure seamless infinite scroll — clones fill the viewport even on wide screens.

## Cross-References

- [Overview](00-overview.md) — All components
- [Interactive Components](03-interactive-components.md) — User interactions
- [Build System](05-build-system.md) — Library build, docs site

[Next → Build System](05-build-system.md)
