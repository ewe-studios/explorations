# Kumo Deep Dive: Every Component, Every Primitive, Source-Level

Comprehensive source-level documentation of every kumo component and primitive.

**Source:** `/home/darkvoid/Boxxed/@formulas/src.rust/src.AI/src.UIComponents/kumo/packages/kumo/src/`

## Source Map

| Directory | Files | Purpose |
|-----------|-------|---------|
| `src/components/` | ~100 | 35+ styled components |
| `src/primitives/` | ~30 | 30+ Base UI wrappers |
| `src/blocks/` | ~10 | Composite page patterns |
| `src/utils/` | ~10 | Utilities (cn, link-provider, portal-provider) |
| `src/registry/` | ~5 | Component metadata, AI schemas |
| `src/catalog/` | ~5 | Component catalog system |
| `src/code/` | ~5 | Syntax highlighting (Shiki) |
| `src/command-line/` | ~10 | CLI implementation |
| `src/styles/` | ~5 | CSS build outputs |

---

## Components

### Button (`src/components/button/`)

**Source:** `src/components/button/button.tsx`

#### Exports

| Export | Type |
|--------|------|
| `Button` | Main button component |
| `RefreshButton` | Button with refresh icon |
| `LinkButton` | Button that renders as a link |
| `buttonVariants` | Class generator function |
| `ButtonProps` | TypeScript type |
| `LinkButtonProps` | TypeScript type |

#### Variants

```typescript
// src/components/button/button.tsx:10-87
export const KUMO_BUTTON_VARIANTS = {
  shape: {
    base: { classes: "", description: "Default rectangular button" },
    square: { classes: "items-center justify-center p-0", description: "Square icon-only" },
    circle: { classes: "items-center justify-center p-0 rounded-full", description: "Circular icon-only" },
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

#### Default Variants

```typescript
export const KUMO_BUTTON_DEFAULT_VARIANTS = {
  shape: "base",
  size: "base",
  variant: "secondary",
} as const;
```

#### buttonVariants()

```typescript
// src/components/button/button.tsx:125-162
export function buttonVariants({
  variant = "secondary",
  size = "base",
  shape = "base",
}: KumoButtonVariantsProps = {}) {
  const isCompactShape = shape === "square" || shape === "circle";
  return cn(
    "group flex w-max shrink-0 items-center font-medium select-none",
    "border-0 shadow-xs",
    "focus:outline-none focus:ring-kumo-focus/50 focus-visible:ring-2",
    "cursor-pointer",
    "disabled:cursor-not-allowed disabled:text-kumo-subtle",
    resolveVariant(KUMO_BUTTON_VARIANTS.size, size, "base").classes,
    resolveVariant(KUMO_BUTTON_VARIANTS.shape, shape, "base").classes,
    isCompactShape && resolveVariant(KUMO_BUTTON_VARIANTS.compactSize, size, "base").classes,
    resolveVariant(KUMO_BUTTON_VARIANTS.variant, variant, "secondary").classes,
  );
}
```

#### Emphasis Token System

```typescript
// src/components/button/button.tsx:172-188
const getEmphasisToken = (variant: KumoButtonVariant) => {
  if (variant === "primary") return "var(--color-kumo-brand)";
  if (variant === "destructive") return "var(--color-kumo-danger)";
  return undefined;
};

const getEmphasisStyle = (variant: KumoButtonVariant) => {
  const token = getEmphasisToken(variant);
  if (!token) return undefined;
  return {
    "--kumo-button-emphasis-ring": `color-mix(in oklch, ${token}, black 10%)`,
    "--kumo-button-emphasis-bg": `color-mix(in oklch, ${token}, white 30%)`,
    "--kumo-button-emphasis-gradient-start": `color-mix(in oklch, ${token}, white 15%)`,
    "--kumo-button-emphasis-gradient-end": token,
  };
};
```

**Aha:** Primary and destructive buttons use CSS `color-mix()` for gradient backgrounds and ring colors. The emphasis style is computed as CSS custom properties and applied inline, so the button automatically adapts to theme changes without needing separate dark variant classes.

#### Button Component

```typescript
type ButtonBaseProps = React.ButtonHTMLAttributes<HTMLButtonElement> & {
  children?: React.ReactNode;
  className?: string;
  icon?: Icon | React.ReactNode;    // @phosphor-icons/react or React element
  loading?: boolean;                // Shows spinner, disables interaction
  title?: React.ReactNode;          // Wraps in Tooltip when set
};

// Discriminated union: icon-only buttons require aria-label
type ButtonWithIconProps = ButtonBaseProps & {
  shape: "square" | "circle";
  "aria-label": string;             // Required for accessibility
  icon: Icon | React.ReactNode;
};
```

#### RefreshButton

A Button with a built-in `ArrowsClockwise` icon from `@phosphor-icons/react`:

```typescript
export function RefreshButton(props: ButtonProps) {
  return <Button icon={ArrowsClockwise} {...props} />;
}
```

#### LinkButton

A Button that renders as an anchor tag, using `useLinkComponent` for custom link routing:

```typescript
export function LinkButton({ href, ...props }: LinkButtonProps) {
  const LinkComponent = useLinkComponent();
  return <Button as={LinkComponent} href={href} {...props} />;
}
```

### Badge (`src/components/badge/`)

**Source:** `src/components/badge/badge.tsx`

Renders a small label/tag. Variants: default, success, warning, error, info. Sizes: sm, base, lg.

### Banner (`src/components/banner/`)

**Source:** `src/components/banner/banner.tsx`

Renders a full-width notification banner. Variants: info, warning, error, success.

### Checkbox (`src/components/checkbox/`)

**Source:** `src/components/checkbox/checkbox.tsx`

Wraps the Base UI Checkbox primitive. Props: `checked`, `onChange`, `disabled`, `indeterminate`. Includes legend support via `CheckboxLegendProps`.

### Collapsible (`src/components/collapsible/`)

**Source:** `src/components/collapsible/collapsible.tsx`

Wraps Base UI Collapsible. Compound component pattern with `CollapsibleRoot`, `CollapsibleTrigger`, `CollapsiblePanel`.

### Combobox (`src/components/combobox/`)

**Source:** `src/components/combobox/combobox.tsx`

Combines input with dropdown list. Supports search, async loading, multi-select. Built on Base UI Combobox primitive.

### Date Picker (`src/components/date-picker/`)

**Source:** `src/components/date-picker/date-picker.tsx`

Built on `react-day-picker ^9.13.2`. Supports single date, date range, and disabled date patterns.

### Date Range Picker (`src/components/date-range-picker/`)

**Source:** `src/components/date-range-picker/date-range-picker.tsx`

**Deprecated** — use `DatePicker` with `mode="range"` instead.

### Dialog (`src/components/dialog/`)

**Source:** `src/components/dialog/dialog.tsx`

Compound component: `DialogRoot`, `DialogTrigger`, `DialogTitle`, `DialogDescription`, `DialogClose`. Uses Base UI Dialog primitive.

### Dropdown (`src/components/dropdown/`)

**Source:** `src/components/dropdown/dropdown.tsx`

Dropdown menu built on Base UI Menu primitive. Supports nested submenus, icons, keyboard navigation.

### Field (`src/components/field/`)

**Source:** `src/components/field/field.tsx`

Form field wrapper with label, description, error message. Variants: default, inline.

### Grid (`src/components/grid/`)

**Source:** `src/components/grid/grid.tsx`

CSS grid layout component. Variants: auto (responsive), fixed, mixed. Supports gap, columns, breakpoints.

### Input (`src/components/input/`)

**Source:** `src/components/input/input.tsx`

Text input with variants. Includes `InputArea`/`Textarea` for multi-line input.

### Input Group (`src/components/input-group/`)

**Source:** `src/components/input-group/input-group.tsx`

Groups input with addon/suffix/button. Compound component: `InputGroupRoot`, `InputGroupAddon`, `InputGroupSuffix`, `InputGroupInput`, `InputGroupButton`.

### Label (`src/components/label/`)

**Source:** `src/components/label/label.tsx`

Form label with variants. Wraps Base UI Label primitive.

### Layer Card (`src/components/layer-card/`)

**Source:** `src/components/layer-card/layer-card.tsx`

Card component for layered content. **Replaces the deprecated `Surface` component.**

### Link (`src/components/link/`)

**Source:** `src/components/link/link.tsx`

Styled link component. Uses `useLinkComponent` for custom routing. Variants: default, subtle, brand.

### Loader (`src/components/loader/`)

**Source:** `src/components/loader/loader.tsx`

Loading spinner with variants. Includes `SkeletonLine` for content placeholders.

### Menu Bar (`src/components/menubar/`)

**Source:** `src/components/menubar/menubar.tsx`

Horizontal menu bar. Uses `useMenuNavigation` hook for keyboard navigation.

### Meter (`src/components/meter/`)

**Source:** `src/components/meter/meter.tsx`

Progress/level indicator. Variants: bar, circular.

### Pagination (`src/components/pagination/`)

**Source:** `src/components/pagination/pagination.tsx`

Page navigation with prev/next, page numbers, and ellipsis for large page counts.

### Popover (`src/components/popover/`)

**Source:** `src/components/popover/popover.tsx`

Floating panel. Compound component: `PopoverRoot`, `PopoverTrigger`, `PopoverContent`, `PopoverTitle`, `PopoverDescription`, `PopoverClose`.

### Radio (`src/components/radio/`)

**Source:** `src/components/radio/radio.tsx`

Radio group with variants. Supports legend, descriptions, and custom appearances.

### Select (`src/components/select/`)

**Source:** `src/components/select/select.tsx`

Dropdown select built on Base UI Select primitive. Supports search, multi-select, async loading.

### Sensitive Input (`src/components/sensitive-input/`)

**Source:** `src/components/sensitive-input/sensitive-input.tsx`

Password-style input with show/hide toggle. For API keys, passwords, tokens.

### Sidebar (`src/components/sidebar/`)

**Source:** `src/components/sidebar/sidebar.tsx`

Full sidebar component with collapsible groups, resize handle, sliding views. Compound component with 20+ sub-components: `SidebarProvider`, `SidebarRoot`, `SidebarHeader`, `SidebarContent`, `SidebarFooter`, `SidebarGroup`, `SidebarMenu`, `SidebarMenuButton`, `SidebarMenuSub`, `SidebarSeparator`, `SidebarTrigger`, `SidebarRail`, `SidebarResizeHandle`, `SidebarCollapsible`, etc.

**Aha:** The Sidebar is one of kumo's most complex components at 300+ lines. It supports:
- Two variants: floating, inset
- Two sides: left, right
- Collapsible with stored state
- Resizable with drag handle
- Sliding views for nested navigation
- Menu groups with chevron expand/collapse

### Switch (`src/components/switch/`)

**Source:** `src/components/switch/switch.tsx`

Toggle switch with legend support. Wraps Base UI Switch primitive.

### Table (`src/components/table/`)

**Source:** `src/components/table/table.tsx`

Data table with sortable columns, row selection, and pagination support.

### Table of Contents (`src/components/table-of-contents/`)

**Source:** `src/components/table-of-contents/table-of-contents.tsx`

Sticky TOC with active item tracking. Variants: default, compact.

### Tabs (`src/components/tabs/`)

**Source:** `src/components/tabs/tabs.tsx`

Tab navigation. Wraps Base UI Tabs primitive. Variants: default, underline.

### Text (`src/components/text/`)

**Source:** `src/components/text/text.tsx`

Typography component. Variants: heading, body, caption, code. Sizes: xs through 2xl.

### Toast (`src/components/toast/`)

**Source:** `src/components/toast/toast.tsx`

Toast notification system. Exports: `Toasty` (component), `ToastProvider`, `Toast`, `useKumoToastManager`, `createKumoToastManager`.

### Toolbar (`src/components/toolbar/`)

**Source:** `src/components/toolbar/toolbar.tsx`

Horizontal toolbar with buttons, inputs, separators. Variants: default, compact. Includes `ToolbarInputGroup` for grouped inputs.

### Tooltip (`src/components/tooltip/`)

**Source:** `src/components/tooltip/tooltip.tsx`

Tooltip wrapper. Wraps Base UI Tooltip primitive. Use `TooltipProvider` at app root.

### Autocomplete (`src/components/autocomplete/`)

**Source:** `src/components/autocomplete/autocomplete.tsx`

Autocomplete input with async search. Supports keyboard navigation, multi-select, custom rendering.

### Breadcrumbs (`src/components/breadcrumbs/`)

**Source:** `src/components/breadcrumbs/breadcrumbs.tsx`

Breadcrumb navigation. Supports custom separators, overflow truncation.

### Clipboard Text (`src/components/clipboard-text/`)

**Source:** `src/components/clipboard-text/clipboard-text.tsx`

Clickable text that copies to clipboard. Shows "copied" feedback.

### Cloudflare Logo (`src/components/cloudflare-logo/`)

**Source:** `src/components/cloudflare-logo/cloudflare-logo.tsx`

Cloudflare logo component with variants. Exports: `CloudflareLogo`, `PoweredByCloudflare`, `generateCloudflareLogoSvg`.

### Code (`src/components/code/`)

**Source:** `src/components/code/code.tsx`

Code display with syntax highlighting (Shiki). Exports: `Code` (inline), `CodeBlock` (block).

### Command Palette (`src/components/command-palette/`)

**Source:** `src/components/command-palette/command-palette.tsx`

Command palette (⌘K dialog). Supports groups, keyboard navigation, fuzzy search.

### Empty (`src/components/empty/`)

**Source:** `src/components/empty/empty.tsx`

Empty state placeholder. For showing "no results" or "nothing here yet" states.

### Flow (`src/components/flow/`)

**Source:** `src/components/flow/flow.tsx`

Flow/step indicator component. Variants: horizontal, vertical.

### Chart (`src/components/chart/`)

**Source:** `src/components/chart/chart.tsx`

Chart components built on ECharts. Exports: `Chart`, `SankeyChart`, `TimeseriesChart`, `ChartLegend`, `BubbleMap`. Requires `echarts ^6.0.0` peer dependency.

---

## Primitives

Primitives are generated by `scripts/generate-primitives.ts` from Base UI. They provide accessibility without opinions.

### Generated Primitives

| Primitive | Base UI Component | Purpose |
|-----------|------------------|---------|
| `Accordion` | `Accordion` | Expandable sections |
| `AlertDialog` | `AlertDialog` | Confirmation dialogs |
| `Autocomplete` | `Autocomplete` | Autocomplete input |
| `Avatar` | `Avatar` | User avatar |
| `Button` | `Button` | Base button |
| `Checkbox` | `Checkbox` | Checkbox input |
| `CheckboxGroup` | `CheckboxGroup` | Checkbox group |
| `Collapsible` | `Collapsible` | Expand/collapse |
| `Combobox` | `Combobox` | Combobox input |
| `ContextMenu` | `ContextMenu` | Right-click menu |
| `CspProvider` | — | Content Security Policy provider |
| `Dialog` | `Dialog` | Modal dialog |
| `DirectionProvider` | — | RTL/LTR direction provider |
| `Drawer` | `Drawer` | Slide-out panel |
| `Field` | `Field` | Form field |
| `Fieldset` | `Fieldset` | Form fieldset |
| `Form` | `Form` | Form wrapper |
| `Input` | `Input` | Text input |
| `Menu` | `Menu` | Dropdown menu |
| `Menubar` | `Menubar` | Menu bar |
| `Meter` | `Meter` | Progress indicator |
| `NavigationMenu` | `NavigationMenu` | Nav menu |
| `NumberField` | `NumberField` | Number input |
| `OtpField` | `OtpField` | One-time password input |
| `Popover` | `Popover` | Floating panel |
| `PreviewCard` | — | Preview card |
| `Progress` | `Progress` | Progress bar |
| `Radio` | `Radio` | Radio input |
| `RadioGroup` | `RadioGroup` | Radio group |
| `ScrollArea` | `ScrollArea` | Custom scroll area |
| `Select` | `Select` | Select dropdown |
| `Separator` | `Separator` | Divider |
| `Slider` | `Slider` | Range slider |
| `Switch` | `Switch` | Toggle switch |
| `Tabs` | `Tabs` | Tab navigation |
| `Toast` | `Toast` | Toast notification |
| `Toggle` | `Toggle` | Toggle button |
| `ToggleGroup` | `ToggleGroup` | Toggle group |
| `Toolbar` | `Toolbar` | Toolbar |
| `Tooltip` | `Tooltip` | Tooltip |

---

## Blocks

Blocks are composite page-level components installed via CLI (`npx @cloudflare/kumo add <block>`):

| Block | Purpose |
|-------|---------|
| `delete-resource` | Confirmation dialog for resource deletion |
| `page-header` | Page header with title, actions, breadcrumbs |
| `resource-list` | List of resources with selection and actions |

---

## Utilities

### cn()

```typescript
// src/utils/cn.ts
import { clsx } from "clsx";
import { twMerge } from "tailwind-merge";

export function cn(...inputs: ClassValue[]) {
  return twMerge(clsx(inputs));
}
```

Merges Tailwind classes, resolving conflicts.

### link-provider

```typescript
// src/utils/link-provider.tsx
// Provides custom link component for routing
export { LinkProvider, useLinkComponent, type LinkComponentProps };
```

### portal-provider

```typescript
// src/utils/portal-provider.tsx
// Provides portal container for overlays
export { KumoPortalProvider, type PortalContainer };
```

### safeRandomId()

```typescript
// src/utils/cn.ts
export function safeRandomId(): string;
```

Generates a safe random ID for accessibility attributes.

---

## CLI

**Source:** `src/command-line/`

The kumo CLI provides:

```bash
npx @cloudflare/kumo ls          # List all components
npx @cloudflare/kumo doc Button  # Get component documentation
npx @cloudflare/kumo docs        # Get all component docs
```

### Commands

| Command | File | Purpose |
|---------|------|---------|
| `ls` | `commands/list.ts` | List all components |
| `doc` | `commands/doc.ts` | Get component documentation |
| `docs` | `commands/docs.ts` | Get all component documentation |

### AI Registry

The CLI reads from `ai/component-registry.json` which is generated from TypeScript types + demo examples. This provides machine-readable component metadata for AI tools.

**Source:** `ai/component-registry.json`, `scripts/component-registry/`.

---

## Next

Return to the [README](README.md) for the full index, or continue to [08-cross-cutting](08-cross-cutting.md) for shared patterns across all src.UIComponents projects.
