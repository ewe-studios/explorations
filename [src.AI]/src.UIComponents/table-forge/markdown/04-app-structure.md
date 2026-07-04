# App Structure

Next.js 15 with App Router, SEO metadata, and layout components.

## Root Layout

Source: `src/app/layout.tsx` — 92 lines.

```tsx
export default function RootLayout({ children }) {
  return (
    <html lang="en" suppressHydrationWarning>
      <body>
        <ThemeProvider>
          <Navbar />
          <main>{children}</main>
          <Footer />
        </ThemeProvider>
      </body>
    </html>
  )
}
```

Includes Google Fonts (Geist Sans + Geist Mono) and SEO metadata.

## Home Page

Source: `src/app/page.tsx` — 61 lines.

Renders `TablesGallery` — a card grid showing all 10 table implementations.

## SEO

Source: `src/lib/seo.ts` — 91 lines.

Generates:
- **Metadata** — title, description, OG image
- **robots.ts** — `robots.txt` generation
- **sitemap.ts** — `sitemap.xml` generation

## Cross-References

- [Overview](00-overview.md) — Package summary
- [Table Components](01-table-components.md) — All 10 tables
- [Data Handling](02-data-handling.md) — Sort, filter, paginate
- [Styling](03-styling.md) — Tailwind, responsive design
