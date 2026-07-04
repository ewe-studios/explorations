# mapcn: UI Components and Command Search

mapcn has a rich set of UI components built on shadcn/ui primitives with MapLibre integration, including a command palette search, theme toggle, and navigation.

Source: `src/components/`, `src/lib/site-navigation.ts`.

## CommandSearch

Source: `src/components/command-search.tsx` — 272 lines.

A `cmdk`-based command palette for jumping to pages, components, and docs.

```tsx
"use client"

export function CommandSearch({ className }: { className?: string }) {
  const [open, setOpen] = React.useState(false)
  const router = useRouter()

  React.useEffect(() => {
    function handleKeyDown(e: KeyboardEvent) {
      if ((e.metaKey || e.ctrlKey) && e.key === "k") {
        e.preventDefault()
        setOpen((open) => !open)
      }
    }
    document.addEventListener("keydown", handleKeyDown)
    return () => document.removeEventListener("keydown", handleKeyDown)
  }, [])

  function handleSelect(href: string) {
    setOpen(false)
    router.push(href)
  }
```

### Keyboard Shortcut

`Cmd+K` / `Ctrl+K` toggles the command dialog. The event listener is added to `document` and cleaned up on unmount.

### Search Items

Populated from `siteNavigation`:

```typescript
import { siteNavigation } from "@/lib/site-navigation"
```

The search includes pages, components, and docs. Results are grouped by category in the `CommandList`.

### Search Button

```tsx
<Button variant="ghost" size="sm" onClick={() => setOpen(true)}>
  <SearchIcon className="size-3.5" />
  <span>Search...</span>
  <Kbd className="ml-auto bg-transparent">⌘K</Kbd>
</Button>
```

Hidden on mobile (`hidden md:flex`), shown as a search icon button on mobile.

## Theme Toggle

Source: `src/components/theme-toggle.tsx` — 28 lines.

```tsx
"use client"

export function ThemeToggle() {
  const { theme, setTheme } = useTheme()
  return (
    <Button variant="ghost" size="sm"
      onClick={() => setTheme(theme === "light" ? "dark" : "light")}>
      {theme === "light" ? <MoonIcon /> : <SunIcon />}
    </Button>
  )
}
```

Uses `next-themes` for system-aware theme switching.

## Header

Source: `src/components/header.tsx` — 46 lines.

Top navigation bar with logo, main nav links, search, theme toggle, and GitHub button.

## MainNav / MobileNav

Source: `src/components/main-nav.tsx`, `src/components/mobile-nav.tsx`

Desktop: horizontal nav links. Mobile: hamburger menu with vertical links.

## PageHeader

Source: `src/components/page-header.tsx` — 56 lines.

Page-level header with title, description, and breadcrumb.

## Logo

Source: `src/components/logo.tsx` — 28 lines.

Brand logo component.

## GitHub Button

Source: `src/components/github-button.tsx` — 24 lines.

GitHub star button linking to the repository.

## Footer

Source: `src/components/footer.tsx` — 18 lines.

Simple footer with links.

## shadcn/ui Components

| Component | Source | Purpose |
|-----------|--------|---------|
| `Button` | `ui/button.tsx` (59 lines) | Button with variants (default, destructive, outline, secondary, ghost, link) |
| `Badge` | `ui/badge.tsx` (46 lines) | Status/category badges (default, secondary, destructive, outline) |
| `Card` | `ui/card.tsx` (92 lines) | Card container (Card, CardHeader, CardTitle, CardDescription, CardContent, CardFooter) |
| `Command` | `ui/command.tsx` (177 lines) | cmdk-based command palette (CommandDialog, CommandInput, CommandList, CommandItem, etc.) |
| `Dialog` | `ui/dialog.tsx` (135 lines) | Radix-based modal dialogs |
| `DropdownMenu` | `ui/dropdown-menu.tsx` (90 lines) | Radix-based dropdown menus |
| `Input` | `ui/input.tsx` (21 lines) | Text input |
| `Kbd` | `ui/kbd.tsx` | Keyboard shortcut display |
| `Separator` | `ui/separator.tsx` (28 lines) | Visual separators (Radix) |
| `Tabs` | `ui/tabs.tsx` (66 lines) | Tab navigation (Radix) |
| `Collapsible` | `ui/collapsible.tsx` | Collapsible sections (Radix) |
| `Popover` | `ui/popover.tsx` (48 lines) | Popover containers (Radix) |
| `ButtonGroup` | `ui/button-group.tsx` | Grouped buttons |
| `Chart` | `ui/chart.tsx` | Chart components |

## Cross-References

- [Overview](00-overview.md) — Architecture summary
- [Blocks](01-blocks.md) — Map block components
- [App Structure](03-app-structure.md) — Next.js layout, CSS
