# Table Components

10 table implementations, each demonstrating a different pattern.

Source: `src/components/tables/` — 10 files, ~800 lines total.

## 1. BasicBorderedTable (34 lines)

The simplest table: a bordered HTML table with header row and data rows.

```tsx
// tables/BasicBorderedTable.tsx
<table className="w-full border-collapse border border-border">
  <thead><tr className="bg-muted"><th>...</th></tr></thead>
  <tbody><tr><td>...</td></tr></tbody>
</table>
```

## 2. StripedRowsTable (52 lines)

Alternating row colors using Tailwind's `even:bg-muted`:

```tsx
<tr className="even:bg-muted odd:bg-background">
```

## 3. CompactTable (42 lines)

Dense layout with reduced padding:

```tsx
<th className="px-2 py-1 text-xs">
<td className="px-2 py-1 text-xs">
```

## 4. MinimalistUI (44 lines)

Borderless design using only spacing and typography for visual hierarchy.

## 5. SortableTable (168 lines)

Column sorting with indicators:

```tsx
const [sortColumn, setSortColumn] = useState<string | null>(null)
const [sortDirection, setSortDirection] = useState<'asc' | 'desc'>('asc')

const sorted = useMemo(() => {
  if (!sortColumn) return data
  return [...data].sort((a, b) => {
    const aVal = a[sortColumn]
    const bVal = b[sortColumn]
    return sortDirection === 'asc' ? aVal > bVal ? 1 : -1 : bVal > aVal ? 1 : -1
  })
}, [data, sortColumn, sortDirection])
```

Clicking a header toggles sort direction. Arrow indicators show current sort state.

## 6. FilterableTable (100 lines)

Text filter with real-time search:

```tsx
const [filter, setFilter] = useState('')
const filtered = data.filter(row =>
  Object.values(row).some(val =>
    String(val).toLowerCase().includes(filter.toLowerCase())
  )
)
```

Searches all columns. Case-insensitive.

## 7. PaginatedTable (185 lines)

Full pagination with page controls:

```tsx
const [currentPage, setCurrentPage] = useState(1)
const pageSize = 10
const totalPages = Math.ceil(data.length / pageSize)
const paginated = data.slice((currentPage - 1) * pageSize, currentPage * pageSize)
```

Page navigation with first/prev/next/last buttons and page number display.

## 8. WithActionsTable (56 lines)

Action buttons per row (edit, delete):

```tsx
<td>
  <Button variant="ghost" size="sm" onClick={() => onEdit(row)}>Edit</Button>
  <Button variant="ghost" size="sm" onClick={() => onDelete(row)}>Delete</Button>
</td>
```

## 9. ResponsiveCardsOnMobile (57 lines)

Uses CSS media queries to switch between table layout (desktop) and card layout (mobile):

```tsx
// Desktop: <table> element
// Mobile: <div> cards with label: value pairs
```

## 10. TopControlsSortableTable (129 lines)

Combines sorting, filtering, and pagination in one component:

```tsx
// State: sortColumn, sortDirection, filter, currentPage
// Pipeline: filter → sort → paginate
```

## Example Data

Source: `src/examples/index.ts` — 100 lines.

Sample data generators for users, products, and other common table content.

## Cross-References

- [Overview](00-overview.md) — Package summary
- [Data Handling](02-data-handling.md) — Sort, filter, paginate logic
- [Styling](03-styling.md) — Tailwind, responsive design
- [App Structure](04-app-structure.md) — Next.js layout, SEO
