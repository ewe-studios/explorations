# mapcn: UI Components

mapcn has a rich set of UI components built on shadcn/ui primitives with MapLibre integration.

**Source:** `/home/darkvoid/Boxxed/@formulas/src.rust/src.AI/src.UIComponents/mapcn/`

## Layout Components

| Component | Source | Purpose |
|-----------|--------|---------|
| `Header` | `src/components/header.tsx` | Top navigation bar with logo and nav links |
| `MainNav` | `src/components/main-nav.tsx` | Primary navigation links |
| `MobileNav` | `src/components/mobile-nav.tsx` | Mobile-responsive navigation |
| `Footer` | `src/components/footer.tsx` | Page footer |
| `Logo` | `src/components/logo.tsx` | Brand logo component |
| `GitHubButton` | `src/components/github-button.tsx` | GitHub star button |
| `PageHeader` | `src/components/page-header.tsx` | Page-level header with breadcrumb |

## Theme Components

| Component | Source | Purpose |
|-----------|--------|---------|
| `ThemeProvider` | `src/components/theme-provider.tsx` | next-themes wrapper |
| `ThemeToggle` | `src/components/theme-toggle.tsx` | Dark/light toggle button |

## Search & Navigation

| Component | Source | Purpose |
|-----------|--------|---------|
| `CommandSearch` | `src/components/command-search.tsx` | Command palette search (cmdk-based) |

## UI Primitives (shadcn/ui)

| Component | Source | Purpose |
|-----------|--------|---------|
| `Button` | `src/components/ui/button.tsx` | Button with variants |
| `ButtonGroup` | `src/components/ui/button-group.tsx` | Grouped buttons |
| `Badge` | `src/components/ui/badge.tsx` | Status/category badges |
| `Card` | `src/components/ui/card.tsx` | Card container |
| `Chart` | `src/components/ui/chart.tsx` | Chart components |
| `Collapsible` | `src/components/ui/collapsible.tsx` | Collapsible sections |
| `Command` | `src/components/ui/command.tsx` | Command palette (cmdk) |
| `Dialog` | `src/components/ui/dialog.tsx` | Modal dialogs |
| `DropdownMenu` | `src/components/ui/dropdown-menu.tsx` | Dropdown menus |
| `Input` | `src/components/ui/input.tsx` | Text input |
| `Kbd` | `src/components/ui/kbd.tsx` | Keyboard shortcut display |
| `Separator` | `src/components/ui/separator.tsx` | Visual separators |

## Cross-References

- [Overview](00-overview.md) — Architecture summary
- [Blocks](01-blocks.md) — MapLibre block components
