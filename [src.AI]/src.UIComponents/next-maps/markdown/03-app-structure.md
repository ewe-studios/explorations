# App Structure and Styling

Next.js 15.2.3 with App Router, Tailwind CSS v4, and shadcn/ui component primitives.

## Root Layout

Source: `src/app/layout.tsx` — 42 lines.

```tsx
export default function RootLayout({ children }) {
  return (
    <html lang="en" suppressHydrationWarning>
      <body className={`${geistSans.variable} ${geistMono.variable} antialiased`}>
        <ThemeProvider attribute="class" defaultTheme="system"
          enableSystem disableTransitionOnChange>
          <main className="min-h-screen">{children}</main>
        </ThemeProvider>
      </body>
    </html>
  );
}
```

Key choices:
- **`suppressHydrationWarning`** — Prevents hydration mismatch from the theme class (set by `next-themes` client-side).
- **Geist Sans + Geist Mono** — Vercel's font family, loaded via `next/font/google`. CSS variables `--font-geist-sans` and `--font-geist-mono` are set on `<body>`.
- **`ThemeProvider`** — Uses `class` attribute strategy (adds `.dark` class), defaults to system preference.

## Home Page

Source: `src/app/page.tsx` — 35 lines.

```tsx
"use client";

export default function Home() {
  const mapContainerRef = useRef<HTMLDivElement | null>(null);

  return (
    <div className="relative w-screen h-screen">
      <div id="map-container" ref={mapContainerRef} className="absolute inset-0" />
      <MapProvider mapContainerRef={mapContainerRef}
        initialViewState={{ longitude: -122.4194, latitude: 37.7749, zoom: 10 }}>
        <MapSearch />
        <MapCotrols />
        <MapStyles />
      </MapProvider>
    </div>
  );
}
```

A single `"use client"` component — the entire app is client-side rendered because Mapbox requires DOM access.

## Styling: Tailwind CSS v4

Source: `src/app/globals.css` — ~160 lines.

Tailwind v4 uses `@import "tailwindcss"` and `@theme inline` for custom tokens instead of `tailwind.config.js`.

### Theme Tokens

All design tokens use the oklch color space:

```css
:root {
  --background: oklch(1 0 0);          /* white */
  --foreground: oklch(0.145 0 0);       /* near-black */
  --primary: oklch(0.205 0 0);
  --radius: 0.625rem;
  /* ... 40+ tokens */
}

.dark {
  --background: oklch(0.145 0 0);       /* near-black */
  --foreground: oklch(0.985 0 0);       /* white */
  /* ... inverted */
}
```

### Mapbox Popup Styling

Custom CSS overrides for Mapbox popup appearance, with dark mode support:

```css
.mapboxgl-custom-popup .mapboxgl-popup-content {
  @apply bg-card text-card-foreground p-5 rounded-lg;
}
```

Popup tip colors are set per-anchor (top, bottom, left, right) to match `--card`, with a separate dark mode override.

### Base Layer

```css
@layer base {
  * { @apply border-border outline-ring/50; }
  button, [role="button"] { cursor: pointer; }
  body { @apply bg-background text-foreground; }
}
```

## shadcn/ui Components

The app uses 8 shadcn/ui primitives, all in `src/components/ui/`:

| Component | Source | Lines | Purpose |
|-----------|--------|-------|---------|
| `Badge` | `badge.tsx` | 46 | Status/category tags in LocationPopup |
| `Button` | `button.tsx` | 59 | Zoom controls, search triggers |
| `Command` | `command.tsx` | 177 | Search palette (cmdk-based) |
| `Dialog` | `dialog.tsx` | 135 | Modal dialogs (Radix) |
| `Input` | `input.tsx` | 21 | Search input |
| `Popover` | `popover.tsx` | 48 | Popover containers (Radix) |
| `Separator` | `separator.tsx` | 28 | Visual dividers (Radix) |
| `Tabs` | `tabs.tsx` | 66 | Map style switcher (Radix) |

All use `class-variance-authority` (cva) for variant definitions and `cn()` for class merging.

## Next.js Config

Source: `next.config.ts` — 5 lines.

```ts
const nextConfig: NextConfig = {};
export default nextConfig;
```

Default configuration. No custom rewrites, redirects, or headers. Turbopack is enabled via `--turbopack` flag in `package.json`.

## Cross-References

- [Overview](00-overview.md) — Architecture summary
- [Map Context](01-map-context.md) — Provider, API, constants
- [Components](02-components.md) — All React components
