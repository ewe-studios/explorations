# kumo: Cloudflare's Component Library

Cloudflare's production React component library built on Base UI and Tailwind CSS v4. 44+ components, 30+ primitives, blocks, CLI, AI registry, Figma sync plugin.

**Source:** `/home/darkvoid/Boxxed/@formulas/src.rust/src.AI/src.UIComponents/kumo/packages/kumo/`

## Package Info

| Attribute | Value |
|-----------|-------|
| Package | `@cloudflare/kumo` |
| Version | 2.6.0 |
| Language | TypeScript/React 19 |
| Base UI | `@base-ui/react ^1.6.0` |
| Files | 364 (227 source) |
| Bundle | Tree-shakable ESM |

## Dependencies

| Type | Package | Purpose |
|------|---------|---------|
| Runtime | `@base-ui/react ^1.6.0` | Accessible component primitives |
| Runtime | `clsx ^2.1.1` | Class merging |
| Runtime | `tailwind-merge ^3.4.0` | Tailwind conflict resolution |
| Runtime | `motion ^12.34.1` | Animations |
| Runtime | `react-day-picker ^9.13.2` | Date picker |
| Runtime | `shiki ^4.0.0` | Syntax highlighting |
| Peer | `@phosphor-icons/react ^2.1.10` | Icon library |
| Peer | `echarts ^6.0.0` | Charts (optional) |
| Peer | `zod ^4.0.0` | Validation (optional) |

## Installation

```bash
npm install @cloudflare/kumo @phosphor-icons/react react react-dom
```

## Import Patterns

```tsx
// Granular import (tree-shakable)
import { Button } from "@cloudflare/kumo/components/button";
import "@cloudflare/kumo/styles";

// Barrel import (larger bundle)
import { Button, Dialog, Sidebar } from "@cloudflare/kumo";
```

## Component Categories

### Action (4)
Button, RefreshButton, LinkButton, ClipboardText

### Display (8)
Badge, Breadcrumbs, Code, CodeBlock, Empty, LayerCard, Meter, Surface (deprecated → LayerCard), Text

### Feedback (3)
Banner, Loader, SkeletonLine, Toast (Toasty)

### Input (10)
Checkbox, Collapsible, Combobox, DatePicker, DateRangePicker (deprecated), Field, Input, InputGroup, InputArea, Textarea, Label, Radio, RadioGroup, Select, SensitiveInput, Switch

### Layout (2)
Grid, GridItem, Surface

### Navigation (5)
CommandPalette, MenuBar, Pagination, Tabs, TableOfContents

### Overlay (4)
Dialog, DropdownMenu, Popover, Tooltip

### Data (2)
Table, Chart (SankeyChart, TimeseriesChart, BubbleMap, ChartLegend, BubbleMap)

### Complex (3)
Sidebar (20+ sub-components), Autocomplete, Flow

### Blocks (installed via CLI)
DeleteResource, PageHeader, ResourceList

## Key Rules

1. **Semantic tokens only** — `bg-kumo-base`, `text-kumo-default` — never raw Tailwind colors
2. **No `dark:` variant** — CSS `light-dark()` handles themes automatically
3. **Use `cn()`** for class merging
4. **Import styles** — `import "@cloudflare/kumo/styles"`

## Button Component: Source Detail

**Source:** `src/components/button/button.tsx`

### Variants

```typescript
export const KUMO_BUTTON_VARIANTS = {
  shape: {
    base: { classes: "" },
    square: { classes: "items-center justify-center p-0" },
    circle: { classes: "items-center justify-center p-0 rounded-full" },
  },
  size: {
    xs: { classes: "h-5 gap-1 rounded-sm px-1.5 text-xs" },
    sm: { classes: "h-6.5 gap-1 rounded-md px-2 text-xs" },
    base: { classes: "h-9 gap-1.5 rounded-lg px-3 text-base" },
    lg: { classes: "h-10 gap-2 rounded-lg px-4 text-base" },
  },
  variant: {
    primary: { classes: "bg-(--kumo-button-emphasis-bg) !text-white ring ring-(--kumo-button-emphasis-ring)..." },
    secondary: { classes: "bg-kumo-base !text-kumo-default ring not-disabled:hover:bg-kumo-tint..." },
    ghost: { classes: "text-kumo-default hover:bg-kumo-tint shadow-none bg-inherit" },
    destructive: { classes: "bg-(--kumo-button-emphasis-bg) !text-white ring..." },
    "secondary-destructive": { classes: "bg-kumo-base !text-kumo-danger ring..." },
    outline: { classes: "bg-transparent text-kumo-default ring ring-kumo-line..." },
  },
} as const;
```

### Emphasis Token System

```typescript
const getEmphasisStyle = (variant) => {
  const token = variant === "primary" ? "var(--color-kumo-brand)"
    : variant === "destructive" ? "var(--color-kumo-danger)" : undefined;
  return token ? {
    "--kumo-button-emphasis-ring": `color-mix(in oklch, ${token}, black 10%)`,
    "--kumo-button-emphasis-bg": `color-mix(in oklch, ${token}, white 30%)`,
  } : undefined;
};
```

### Discriminated Union (Accessibility)

```typescript
type ButtonWithIconProps = ButtonBaseProps & {
  shape: "square" | "circle";  // Required
  "aria-label": string;         // Required
  icon: Icon | React.ReactNode;
};
```

Icon-only buttons require `aria-label` — enforced by TypeScript.

## Sidebar Component: 20+ Exports

**Source:** `src/components/sidebar/sidebar.tsx`

Sidebar, SidebarProvider, SidebarRoot, SidebarHeader, SidebarContent, SidebarFooter, SidebarGroup, SidebarGroupLabel, SidebarMenu, SidebarMenuItem, SidebarMenuButton, SidebarMenuSub, SidebarMenuSubItem, SidebarMenuSubButton, SidebarSeparator, SidebarTrigger, SidebarRail, SidebarResizeHandle, SidebarMenuChevron, SidebarCollapsible, SidebarCollapsibleTrigger, SidebarCollapsibleContent, SidebarSlidingViews, SidebarSlidingView, useSidebar

Variants: floating, inset. Sides: left, right. Features: resizable, collapsible.

## Primitives: Generated

30+ Base UI wrappers generated by `scripts/generate-primitives.ts`:
Accordion, AlertDialog, Autocomplete, Avatar, Button, Checkbox, CheckboxGroup, Collapsible, Combobox, ContextMenu, Dialog, DirectionProvider, Drawer, Field, Fieldset, Form, Input, Menu, Menubar, Meter, NavigationMenu, NumberField, OtpField, Popover, PreviewCard, Progress, Radio, RadioGroup, ScrollArea, Select, Separator, Slider, Switch, Tabs, Toast, Toggle, ToggleGroup, Toolbar, Tooltip

## AI Registry

```bash
npx @cloudflare/kumo ls          # List components
npx @cloudflare/kumo doc Button  # Component docs
npx @cloudflare/kumo docs        # All docs
```

Registry at `ai/component-registry.json` — generated from TypeScript types.

## Next

Continue to [01-theming](01-theming.md) for the CSS theming system.
