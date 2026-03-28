---
title: "Zero Virtual Deep Dive"
subtitle: "Complete guide to Zero's virtual scrolling implementation for large lists"
---

# Zero Virtual Deep Dive

## 1. Overview

This document provides a comprehensive deep dive into `zero-virtual`, Zero's infinite virtual scrolling library built on top of TanStack Virtual.

### Features

- **Bidirectional infinite scrolling** - Load more items at top or bottom
- **Permalink support** - Jump to and highlight a specific item by ID
- **State persistence** - Restore scroll position across navigation
- **Dynamic page sizing** - Adjust page size based on viewport
- **Settled state detection** - Optimize queries when user stops scrolling

### Architecture

```
┌─────────────────────────────────────────────────────────┐
│                   React Component                        │
│              (Your App's List Component)                 │
└─────────────────────────────────────────────────────────┘
                            │
                            ▼
┌─────────────────────────────────────────────────────────┐
│              useZeroVirtualizer Hook                     │
│  ┌─────────────────────────────────────────────────┐    │
│  │           Paging State Management               │    │
│  │  - Anchor tracking (position, direction)        │    │
│  │  - Forward/backward pagination                  │    │
│  │  - Scroll state persistence                     │    │
│  └─────────────────────────────────────────────────┘    │
│  ┌─────────────────────────────────────────────────┐    │
│  │            Row Data Management                  │    │
│  │  - useRows hook integration                     │    │
│  │  - Row caching                                  │    │
│  │  - Loading states                               │    │
│  └─────────────────────────────────────────────────┘    │
│  ┌─────────────────────────────────────────────────┐    │
│  │         TanStack Virtual Integration            │    │
│  │  - Virtualizer instance                         │    │
│  │  - Item sizing                                  │    │
│  │  - Scroll handling                              │    │
│  └─────────────────────────────────────────────────┘    │
└─────────────────────────────────────────────────────────┘
                            │
                            ▼
┌─────────────────────────────────────────────────────────┐
│                    Zero Queries                         │
│  ┌─────────────────┐      ┌─────────────────┐         │
│  │  getPageQuery   │      │  getSingleQuery │         │
│  │  (Fetch pages)  │      │ (Permalink lookup)│        │
│  └─────────────────┘      └─────────────────┘         │
└─────────────────────────────────────────────────────────┘
```

## 2. Core Hook: useZeroVirtualizer

### 2.1 Hook Signature

```typescript
function useZeroVirtualizer<
  TScrollElement extends Element,
  TItemElement extends Element,
  TListContextParams,
  TRow,
  TStartRow
>(options: UseZeroVirtualizerOptions<
  TScrollElement,
  TItemElement,
  TListContextParams,
  TRow,
  TStartRow
>): ZeroVirtualizerResult<TScrollElement, TItemElement, TRow>
```

### 2.2 Options

```typescript
interface UseZeroVirtualizerOptions {
  // TanStack Virtual options
  estimateSize: (index: number) => number;
  getScrollElement: () => TScrollElement | null;
  overscan?: number;
  paddingStart?: number;
  paddingEnd?: number;

  // Zero-specific options
  listContextParams: TListContextParams;
  getPageQuery: GetPageQuery<TRow, TStartRow>;
  getSingleQuery: GetSingleQuery<TRow>;
  toStartRow: (row: TRow) => TStartRow;

  // Permalink support
  permalinkID?: string | null;

  // Scroll settling
  settleTime?: number; // Default: 2000ms
  onSettled?: () => void;

  // State persistence
  scrollState?: ScrollHistoryState<TStartRow> | null;
  onScrollStateChange?: (state: ScrollHistoryState<TStartRow>) => void;
}
```

### 2.3 Result

```typescript
interface ZeroVirtualizerResult {
  // TanStack Virtual virtualizer
  virtualizer: Virtualizer<TScrollElement, TItemElement>;

  // Row accessor
  rowAt: (index: number) => TRow | undefined;

  // Loading states
  complete: boolean;       // All initial data loaded
  rowsEmpty: boolean;      // No rows match query
  permalinkNotFound: boolean;

  // Total counts
  estimatedTotal: number;  // Approximate count
  total: number | undefined;  // Exact count (when known)

  // Settled state
  settled: boolean;  // User hasn't scrolled for settleTime ms
}
```

## 3. Paging State Management

### 3.1 Anchor Types

```typescript
type Anchor<TStartRow> =
  | {
      // Start from beginning
      kind: 'forward';
      index: number;
      startRow: TStartRow | undefined;
    }
  | {
      // Load backward from a position
      kind: 'backward';
      index: number;
      startRow: TStartRow;
    }
  | {
      // Jump to specific item (permalink)
      kind: 'permalink';
      id: string;
      index: number;
    };
```

### 3.2 Paging Reducer

```typescript
// paging-reducer.ts

type PagingState<TStartRow> = {
  anchor: Anchor<TStartRow>;
  forwardLoaded: TStartRow | undefined;
  backwardLoaded: TStartRow | undefined;
  hasReachedStart: boolean;
  hasReachedEnd: boolean;
};

type PagingAction<TStartRow> =
  | { type: 'LOAD_FORWARD'; startRow: TStartRow }
  | { type: 'LOAD_BACKWARD'; startRow: TStartRow }
  | { type: 'FORWARD_COMPLETE'; hasMore: boolean }
  | { type: 'BACKWARD_COMPLETE'; hasMore: boolean }
  | { type: 'SET_PERMALINK'; id: string; index: number }
  | { type: 'RESTORE'; state: PagingState<TStartRow> };

function pagingReducer<TStartRow>(
  state: PagingState<TStartRow>,
  action: PagingAction<TStartRow>
): PagingState<TStartRow> {
  switch (action.type) {
    case 'LOAD_FORWARD':
      return {
        ...state,
        anchor: {
          kind: 'forward',
          index: state.anchor.index,
          startRow: action.startRow,
        },
      };

    case 'LOAD_BACKWARD':
      return {
        ...state,
        anchor: {
          kind: 'backward',
          index: state.anchor.index,
          startRow: action.startRow,
        },
      };

    case 'FORWARD_COMPLETE':
      return {
        ...state,
        forwardLoaded: action.hasMore
          ? state.anchor.startRow
          : undefined,
        hasReachedEnd: !action.hasMore,
      };

    case 'BACKWARD_COMPLETE':
      return {
        ...state,
        backwardLoaded: action.hasMore
          ? state.anchor.startRow
          : undefined,
        hasReachedStart: !action.hasMore,
      };

    case 'SET_PERMALINK':
      return {
        ...state,
        anchor: {
          kind: 'permalink',
          id: action.id,
          index: action.index,
        },
      };

    case 'RESTORE':
      return action.state;

    default:
      return state;
  }
}
```

### 3.3 Scroll State Persistence

```typescript
// useHistoryScroll State hook

interface ScrollHistoryState<TStartRow> {
  anchor: Anchor<TStartRow>;
  scrollTop: number;
  estimatedTotal: number;
  hasReachedStart: boolean;
  hasReachedEnd: boolean;
  listContextParams: unknown;
}

function useHistoryPermalinkState<TStartRow>(key: string = 'default') {
  const [state, setState] = useState<ScrollHistoryState<TStartRow> | null>(null);

  // Read from history.state on mount
  useEffect(() => {
    const historyState = window.history.state;
    const savedState = historyState?.[`zero-virtual-${key}`];

    if (savedState) {
      setState(savedState);
    }
  }, [key]);

  // Write to history.state when state changes
  const updateState = useCallback((newState: ScrollHistoryState<TStartRow>) => {
    setState(newState);
    window.history.replaceState(
      {
        ...window.history.state,
        [`zero-virtual-${key}`]: newState,
      },
      ''
    );
  }, [key]);

  return [state, updateState] as const;
}
```

## 4. Row Data Management

### 4.1 useRows Hook

```typescript
// use-rows.ts

function useRows<TRow, TStartRow>(options: UseRowsOptions<TRow, TStartRow>) {
  const {
    anchor,
    getPageQuery,
    getSingleQuery,
    toStartRow,
  } = options;

  const [rows, setRows] = useState<Map<number, TRow>>(new Map());
  const [loading, setLoading] = useState(false);
  const [complete, setComplete] = useState(false);

  // Fetch page of rows
  const fetchPage = useCallback(async (
    startRow: TStartRow | null,
    direction: 'forward' | 'backward',
    limit: number
  ) => {
    setLoading(true);

    const query = getPageQuery({
      limit,
      start: startRow,
      dir: direction,
      settled: false,
    });

    const result = await useQuery(query);
    const newRows = result.data;

    setRows(prev => {
      const next = new Map(prev);
      for (const row of newRows) {
        // Insert at appropriate index
        const index = direction === 'forward'
          ? prev.size
          : 0;
        next.set(index, row);
      }
      return next;
    });

    setLoading(false);
    return { rows: newRows, hasMore: newRows.length === limit };
  }, [getPageQuery]);

  // Fetch single row by ID (for permalinks)
  const fetchSingle = useCallback(async (id: string) => {
    const query = getSingleQuery({ id, settled: false });
    const result = await useQuery(query);
    return result.data;
  }, [getSingleQuery]);

  // Access row at index
  const rowAt = useCallback((index: number): TRow | undefined => {
    return rows.get(index);
  }, [rows]);

  return {
    rows,
    rowAt,
    loading,
    complete,
    fetchPage,
    fetchSingle,
  };
}
```

### 4.2 Query Result Type

```typescript
type QueryResult<TReturn> = {
  query: Query<TReturn>;
  options?: UseQueryOptions;
};

type GetPageQueryOptions<TStartRow> = {
  limit: number;
  start: TStartRow | null;
  dir: 'forward' | 'backward';
  settled: boolean;
};

type GetSingleQueryOptions = {
  id: string;
  settled: boolean;
};

// Query function types
type GetPageQuery<TRow, TStartRow> = (
  options: GetPageQueryOptions<TStartRow>
) => QueryResult<TRow[]>;

type GetSingleQuery<TRow> = (
  options: GetSingleQueryOptions
) => QueryResult<TRow | null>;
```

## 5. Scroll Settling

### 5.1 Settled Detection

```typescript
function useSettledDetection(
  scrollElement: Element | null,
  settleTime: number,
  onSettled?: () => void
): boolean {
  const [settled, setSettled] = useState(true);
  const timerRef = useRef<NodeJS.Timeout | null>(null);

  useEffect(() => {
    if (!scrollElement) return;

    const handleScroll = () => {
      // User is scrolling, mark as not settled
      setSettled(false);

      // Clear existing timer
      if (timerRef.current) {
        clearTimeout(timerRef.current);
      }

      // Start new timer
      timerRef.current = setTimeout(() => {
        setSettled(true);
        onSettled?.();
      }, settleTime);
    };

    scrollElement.addEventListener('scroll', handleScroll);

    return () => {
      if (timerRef.current) {
        clearTimeout(timerRef.current);
      }
      scrollElement.removeEventListener('scroll', handleScroll);
    };
  }, [scrollElement, settleTime, onSettled]);

  return settled;
}
```

### 5.2 Settled Query Optimization

```typescript
// When settled, use longer TTL for queries
const getPageQuery = useCallback(
  ({ limit, start, dir, settled }) => ({
    query: queries.items.getPageQuery({ limit, start, dir }),
    options: {
      // Short TTL while scrolling (fresh data)
      // Long TTL when settled (reduce server load)
      ttl: settled ? '5m' : '10s',
    },
  }),
  []
);
```

## 6. Permalink Support

### 6.1 Permalink Resolution Flow

```typescript
async function resolvePermalink(
  id: string,
  getSingleQuery: GetSingleQuery,
  getPageQuery: GetPageQuery
): Promise<PermalinkResult> {
  // 1. Fetch the single row by ID
  const singleResult = await getSingleQuery({ id, settled: false });

  if (!singleResult.data) {
    return { found: false, reason: 'not_found' };
  }

  const targetRow = singleResult.data;

  // 2. Determine position in ordered list
  // Query for rows after the target to count
  const afterQuery = getPageQuery({
    start: toStartRow(targetRow),
    dir: 'forward',
    limit: 1,
    settled: false,
  });
  const afterResult = await useQuery(afterQuery);

  // 3. Calculate approximate index
  // (This is simplified; actual implementation uses binary search)
  const approximateIndex = afterResult.data.length;

  return {
    found: true,
    row: targetRow,
    index: approximateIndex,
  };
}
```

### 6.2 Permalink Highlighting

```typescript
function PermalinkedList({ permalinkID, rows, virtualItems }) {
  return (
    <div>
      {virtualItems.map(virtualRow => {
        const row = rows.get(virtualRow.index);
        const isPermalink = row?.id === permalinkID;

        return (
          <div
            key={virtualRow.key}
            data-index={virtualRow.index}
            style={{
              position: 'absolute',
              transform: `translateY(${virtualRow.start}px)`,
            }}
            className={isPermalink ? 'highlight' : ''}
          >
            {row ? row.title : 'Loading...'}
          </div>
        );
      })}
    </div>
  );
}
```

## 7. Complete Example

### 7.1 Query Definitions

```typescript
// queries.ts
import { defineQuery } from '@rocicorp/zero';
import { zql } from './schema';

export type ItemStart = Pick<Item, 'id' | 'created'>;

export const queries = defineQueries({
  item: {
    // Fetch single item by ID
    getSingleQuery: defineQuery(({ args: { id } }) =>
      zql.item.where('id', id).one()
    ),

    // Fetch page of items
    getPageQuery: defineQuery(({
      args: { limit, start, dir },
    }: {
      args: {
        limit: number;
        start: ItemStart | null;
        dir: 'forward' | 'backward';
      };
    }) => {
      let q = zql.item
        .limit(limit)
        .orderBy('created', dir === 'forward' ? 'desc' : 'asc');

      if (start) {
        q = q.start(start, { inclusive: false });
      }

      return q;
    }),
  },
});
```

### 7.2 List Component

```typescript
// ItemList.tsx

import {
  useZeroVirtualizer,
  useHistoryPermalinkState,
} from '@rocicorp/zero-virtual/react';
import { useCallback, useRef } from 'react';

function getRowKey(item: Item) {
  return item.id;
}

function toStartRow(item: Item): ItemStart {
  return { id: item.id, created: item.created };
}

export function ItemList({ permalinkID }: { permalinkID?: string }) {
  const parentRef = useRef<HTMLDivElement>(null);

  const [permalinkState, setPermalinkState] =
    useHistoryPermalinkState<ItemStart>('item-list');

  const {
    virtualizer,
    rowAt,
    complete,
    rowsEmpty,
    permalinkNotFound,
    estimatedTotal,
    total,
    settled,
  } = useZeroVirtualizer({
    listContextParams: {},
    getScrollElement: useCallback(() => parentRef.current, []),
    estimateSize: useCallback(() => 48, []), // 48px per row
    getRowKey,
    toStartRow,
    getPageQuery: useCallback(
      ({ limit, start, dir, settled }) => ({
        query: queries.item.getPageQuery({ limit, start, dir }),
        options: { ttl: settled ? '5m' : '10s' },
      }),
      []
    ),
    getSingleQuery: useCallback(
      ({ id }) => ({
        query: queries.item.getSingleQuery({ id }),
      }),
      []
    ),
    permalinkID,
    scrollState: permalinkState,
    onScrollStateChange: setPermalinkState,
    settleTime: 2000,
    onSettled: useCallback(() => {
      // Sync scroll state to URL when settled
      console.log('List settled, can update URL');
    }, []),
  });

  const virtualItems = virtualizer.getVirtualItems();

  if (rowsEmpty) {
    return <div>No items found</div>;
  }

  if (permalinkID && permalinkNotFound) {
    return <div>Item not found</div>;
  }

  return (
    <div ref={parentRef} style={{ overflow: 'auto', height: '100vh' }}>
      <div style={{ height: virtualizer.getTotalSize(), position: 'relative' }}>
        {virtualItems.map(virtualRow => {
          const row = rowAt(virtualRow.index);
          const isHighlighted = row?.id === permalinkID;

          return (
            <div
              key={virtualRow.key}
              data-index={virtualRow.index}
              style={{
                position: 'absolute',
                top: 0,
                left: 0,
                width: '100%',
                transform: `translateY(${virtualRow.start}px)`,
                height: `${virtualRow.size}px`,
              }}
              className={isHighlighted ? 'highlight' : ''}
            >
              {row ? (
                <ItemComponent item={row} />
              ) : (
                <div className="loading-skeleton">Loading...</div>
              )}
            </div>
          );
        })}
      </div>
    </div>
  );
}
```

### 7.3 Styling

```css
/* ItemList.css */

.highlight {
  background-color: #fff3cd;
  animation: pulse 2s ease-in-out;
}

@keyframes pulse {
  0%, 100% {
    background-color: #fff3cd;
  }
  50% {
    background-color: #ffc107;
  }
}

.loading-skeleton {
  background: linear-gradient(
    90deg,
    #f0f0f0 25%,
    #e0e0e0 50%,
    #f0f0f0 75%
  );
  background-size: 200% 100%;
  animation: loading 1.5s infinite;
}

@keyframes loading {
  0% {
    background-position: 200% 0;
  }
  100% {
    background-position: -200% 0;
  }
}
```

## 8. Integration with Zero

### 8.1 Zero Query Integration

```typescript
// The useZeroVirtualizer hook integrates with Zero's query system

// Zero provides reactive queries that automatically update
const query = zero.query.item
  .orderBy('created', 'desc')
  .limit(100);

// Virtualizer uses Zero queries for pagination
const getPageQuery = ({ limit, start, dir }) => ({
  query: zero.query.item
    .orderBy('created', dir === 'forward' ? 'desc' : 'asc')
    .limit(limit)
    .start(start, { inclusive: false }),
});

// Changes are automatically pushed to the UI
// No manual cache invalidation needed
```

### 8.2 Performance Considerations

| Aspect | Recommendation |
|--------|----------------|
| Page size | 50-100 rows per page |
| Row height | Fixed height for best performance |
| Overscan | 5-10 rows beyond viewport |
| Settle time | 1-2 seconds for most apps |
| TTL while scrolling | 10-30 seconds |
| TTL when settled | 5-15 minutes |

---

*Next: [06-fractional-indexing-deep-dive.md](06-fractional-indexing-deep-dive.md)*
