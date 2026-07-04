# table-forge: MDX Table Showcase

10 table variations with zero dependencies beyond React + Tailwind CSS. Each documented via MDX with live preview and one-click code copy.

**Source:** `/home/darkvoid/Boxxed/@formulas/src.rust/src.AI/src.UIComponents/table-forge/`

## Table Variations

| Table | File | Feature |
|-------|------|---------|
| Basic Bordered | `BasicBorderedTable.tsx` | Simple bordered table |
| Sortable | `SortableTable.tsx` | Column sorting asc/desc |
| Filterable | `FilterableTable.tsx` | Row filtering via text input |
| Paginated | `PaginatedTable.tsx` | Pagination controls |
| Striped Rows | `StripedRowsTable.tsx` | Alternating row colors |
| Compact | `CompactTable.tsx` | Dense layout |
| With Actions | `WithActionsTable.tsx` | Action buttons per row |
| Responsive Cards | `ResponsiveCardsOnMobile.tsx` | Mobile card layout |
| Minimalist | `MinimalistUI.tsx` | Minimal design |
| Top Controls | `TopControlsSortableTable.tsx` | Sort + search controls |

## SortableTable Implementation

```typescript
// src/components/tables/SortableTable.tsx
function onHeaderClick(key: SortKey) {
  if (key === sortKey) setSortDirection(d => d === "asc" ? "desc" : "asc")
  else { setSortKey(key); setSortDirection("asc") }
}

const sorted = useMemo(() => {
  const data = [...initialData]  // Spread to avoid mutation
  data.sort((a, b) => String(a[sortKey]).toLowerCase().localeCompare(String(b[sortKey]).toLowerCase()))
  return data
}, [sortKey, sortDirection])
```

**Aha:** Data is spread (`[...initialData]`) before sorting to avoid mutation. Uses `localeCompare` for proper alphabetical ordering.

## MDX Pattern

```tsx
// src/examples/tables/BasicBordered.mdx
export const meta = {
  id: "basic-bordered", title: "Basic Bordered", description: "...", group: "Basic",
  codePath: "src/components/tables/BasicBorderedTable.tsx",
}
export default function Example() {
  return <ExampleCard title={meta.title} description={meta.description} codePath={meta.codePath}>
    <BasicBorderedTable />
  </ExampleCard>
}
```

## Example Index

```typescript
// src/examples/index.ts
export const examples: ExampleItem[] = [
  { ...validateMeta(metaMinimalist, "Minimalist"), Component: Minimalist, codePath },
  // ... 10 total
]
```

`validateMeta` ensures every MDX has `id`, `title`, `description`, `group` — fails at build time if missing.

## Zero Dependencies

**Aha:** All 10 tables use only React + Tailwind CSS. No table library, no data grid. Each is hand-coded with `useState`, `useMemo` — easy to copy into any project.
