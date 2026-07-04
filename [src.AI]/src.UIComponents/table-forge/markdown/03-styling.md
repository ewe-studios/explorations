# Styling

Tailwind CSS v4 with shadcn/ui component primitives.

## Tailwind Configuration

Uses Tailwind v4's `@theme inline` syntax in `src/app/globals.css` for custom design tokens.

## shadcn/ui Components

| Component | Source | Lines | Usage |
|-----------|--------|-------|-------|
| `Button` | `ui/button.tsx` | 59 | Table actions, pagination controls |
| `Card` | `ui/card.tsx` | 92 | Example cards, table containers |
| `Tabs` | `ui/tabs.tsx` | 66 | Example navigation |

## Responsive Design

### Mobile Cards

`ResponsiveCardsOnMobile.tsx` switches between table and card layout:

```tsx
// Desktop: <table> element
// Mobile: CSS @media hides table, shows card grid
```

### Tailwind Responsive Utilities

All tables use responsive classes:
- `hidden md:table-cell` — Hide on mobile, show on desktop
- `md:w-32` — Column widths at medium breakpoint
- `text-sm md:text-base` — Font size scaling

## Theme

Source: `src/components/theme/ThemeProvider.tsx` (20 lines), `ThemeToggle.tsx` (28 lines).

Uses `next-themes` for system-aware dark/light mode. The theme toggle is in the navbar.

## Cross-References

- [Overview](00-overview.md) — Package summary
- [Table Components](01-table-components.md) — All 10 tables
- [Data Handling](02-data-handling.md) — Sort, filter, paginate
- [App Structure](04-app-structure.md) — Next.js layout, SEO
