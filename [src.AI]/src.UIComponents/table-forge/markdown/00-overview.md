# table-forge: MDX Table Showcase

10 table variations built with zero dependencies beyond React + Tailwind CSS. Each documented via MDX with live preview and one-click code copy.

**Source:** `/home/darkvoid/Boxxed/@formulas/src.rust/src.AI/src.UIComponents/table-forge/`

## Table Variations

| Table | File | Feature |
|-------|------|---------|
| Basic Bordered | `BasicBorderedTable.tsx` | Simple bordered table with striped header |
| Sortable | `SortableTable.tsx` | Column sorting with asc/desc toggle |
| Filterable | `FilterableTable.tsx` | Row filtering via text input |
| Paginated | `PaginatedTable.tsx` | Pagination with page controls |
| Striped Rows | `StripedRowsTable.tsx` | Alternating row colors |
| Compact | `CompactTable.tsx` | Dense layout |
| With Actions | `WithActionsTable.tsx` | Action buttons per row |
| Responsive Cards | `ResponsiveCardsOnMobile.tsx` | Mobile card layout, desktop table |
| Minimalist | `MinimalistUI.tsx` | Minimal design |
| Top Controls | `TopControlsSortableTable.tsx` | Sort + search controls |

## SortableTable Implementation

```typescript
// src/components/tables/SortableTable.tsx
type Person = { name: string; title: string; email: string; role: string }
type SortKey = keyof Person
type SortDirection = "asc" | "desc"

function SortableTable() {
  const [sortKey, setSortKey] = useState<SortKey>("name")
  const [sortDirection, setSortDirection] = useState<SortDirection>("asc")

  function onHeaderClick(key: SortKey) {
    if (key === sortKey) {
      setSortDirection((d) => (d === "asc" ? "desc" : "asc"))
    } else {
      setSortKey(key)
      setSortDirection("asc")
    }
  }

  const sorted = useMemo(() => {
    const data = [...initialData]  // Spread to avoid mutation
    data.sort((a, b) => {
      const aVal = String(a[sortKey]).toLowerCase()
      const bVal = String(b[sortKey]).toLowerCase()
      return sortDirection === "asc" ? aVal.localeCompare(bVal) : bVal.localeCompare(aVal)
    })
    return data
  }, [sortKey, sortDirection])
  // ...render
}
```

**Aha:** Data is spread (`[...initialData]`) before sorting to avoid mutating the original — a common React mistake that causes re-render loops. Sorting uses `String().toLowerCase()` for case-insensitive comparison and `localeCompare` for proper alphabetical ordering.

## MDX Pattern

```tsx
// src/examples/tables/BasicBordered.mdx
import ExampleCard from "@/components/ExampleCard"
import BasicBorderedTable from "@/components/tables/BasicBorderedTable"

export const meta = {
  id: "basic-bordered",
  title: "Basic Bordered",
  description: "Simple table with borders and striped header.",
  group: "Basic",
  codePath: "src/components/tables/BasicBorderedTable.tsx",
}

export default function Example() {
  return (
    <ExampleCard title={meta.title} description={meta.description} codePath={meta.codePath}>
      <BasicBorderedTable />
    </ExampleCard>
  )
}
```

## Example Index

```typescript
// src/examples/index.ts
export const examples: ExampleItem[] = [
  { ...validateMeta(metaMinimalist, "Minimalist"), Component: Minimalist, codePath: ... },
  { ...validateMeta(metaBasicBordered, "BasicBordered"), Component: BasicBorderedTable, ... },
  // ... 10 total
]
```

`validateMeta` ensures every MDX file has `id`, `title`, `description`, `group` — fails at build time if missing.

## Zero Dependencies

**Aha:** All 10 tables use only React + Tailwind CSS. No table library, no data grid package, no sorting/filtering utility. Each table is hand-coded with React state (`useState`, `useMemo`) and Tailwind classes. This means full control, no library overhead, and easy copy-paste into any project.
