# Assistant Module Exploration

## File Inventory

| File | Lines | Key Exports | Description |
|------|-------|-------------|-------------|
| `sessionHistory.ts` | 88 | `HISTORY_PAGE_SIZE`, `HistoryPage`, `HistoryAuthCtx`, `fetchLatestEvents()`, `fetchOlderEvents()` | Session event pagination with cursor-based navigation |

**Total Source Lines:** 88

---

## Module Overview

### Purpose

The `assistant/` module provides **session history pagination** for retrieving Claude Code conversation events from the remote API. It implements a bi-directional cursor-based pagination system that allows users to navigate through conversation history efficiently.

### Responsibilities

1. **Event Pagination**: Fetch session events in pages of 100 (configurable)
2. **Cursor Management**: Track oldest event ID for fetching older pages
3. **Auth Context Reuse**: Prepare authentication headers once, reuse across requests
4. **Error Resilience**: Graceful failure with null returns on HTTP errors

### Architecture

```
sessionHistory.ts
│
├── Auth Context Layer
│   └── createHistoryAuthCtx() → { baseUrl, headers }
│
├── Fetch Layer
│   ├── fetchPage() [internal]
│   ├── fetchLatestEvents() → newest page
│   └── fetchOlderEvents() → older page via before_id cursor
│
└── Types
    ├── HistoryPage (events, firstId, hasMore)
    └── HistoryAuthCtx (baseUrl, headers)
```

---

## Key Exports

### Constants

```typescript
export const HISTORY_PAGE_SIZE = 100
```

Default page size for event pagination. Used when no explicit limit is provided.

---

### Type Definitions

#### `HistoryPage`

```typescript
export type HistoryPage = {
  /** Chronological order within the page. */
  events: SDKMessage[]
  /** Oldest event ID in this page → before_id cursor for next-older page. */
  firstId: string | null
  /** true = older events exist. */
  hasMore: boolean
}
```

**Purpose**: Standardized page response type for history pagination.

**Fields**:
- `events`: Array of SDK messages in chronological order
- `firstId`: ID of the oldest event in the page (used as cursor for fetching older)
- `hasMore`: Boolean flag indicating if older events exist

---

#### `HistoryAuthCtx`

```typescript
export type HistoryAuthCtx = {
  baseUrl: string
  headers: Record<string, string>
}
```

**Purpose**: Pre-authenticated context object for efficient repeated requests.

**Fields**:
- `baseUrl`: Full API endpoint URL for session events
- `headers`: Pre-computed authentication and beta headers

---

### Functions

#### `createHistoryAuthCtx()`

```typescript
export async function createHistoryAuthCtx(
  sessionId: string,
): Promise<HistoryAuthCtx>
```

**Purpose**: Prepare authentication context once, reuse across multiple page fetches.

**Returns**:
```typescript
{
  baseUrl: `${BASE_API_URL}/v1/sessions/${sessionId}/events`,
  headers: {
    ...getOAuthHeaders(accessToken),
    'anthropic-beta': 'ccr-byoc-2025-07-29',
    'x-organization-uuid': orgUUID,
  }
}
```

**Key Implementation Details**:
- Calls `prepareApiRequest()` for OAuth token and org UUID
- Includes `anthropic-beta` header for CCR/BYOC feature access
- Single initialization reduces redundant auth computations

---

#### `fetchLatestEvents()`

```typescript
export async function fetchLatestEvents(
  ctx: HistoryAuthCtx,
  limit = HISTORY_PAGE_SIZE,
): Promise<HistoryPage | null>
```

**Purpose**: Fetch the newest page of events (most recent first).

**Parameters**:
- `ctx`: Pre-computed auth context
- `limit`: Number of events to fetch (default: 100)

**Returns**: `HistoryPage` with newest events, or `null` on error

**Key Implementation Details**:
- Uses `anchor_to_latest: true` parameter for API
- Returns events in chronological order (oldest to newest within page)
- `hasMore` indicates if older events exist beyond this page

---

#### `fetchOlderEvents()`

```typescript
export async function fetchOlderEvents(
  ctx: HistoryAuthCtx,
  beforeId: string,
  limit = HISTORY_PAGE_SIZE,
): Promise<HistoryPage | null>
```

**Purpose**: Fetch the page of events immediately older than a given cursor.

**Parameters**:
- `ctx`: Pre-computed auth context
- `beforeId`: Cursor ID from `firstId` of previous page
- `limit`: Number of events to fetch (default: 100)

**Returns**: `HistoryPage` with older events, or `null` on error

**Key Implementation Details**:
- Uses `before_id` parameter for cursor-based pagination
- Enables infinite-scroll style history navigation
- Returns `null` on HTTP error (caller handles retry/loading state)

---

### Internal Functions

#### `fetchPage()`

```typescript
async function fetchPage(
  ctx: HistoryAuthCtx,
  params: Record<string, string | number | boolean>,
  label: string,
): Promise<HistoryPage | null>
```

**Purpose**: Internal fetch wrapper with standardized error handling.

**Parameters**:
- `ctx`: Auth context
- `params`: Query parameters (limit, anchor_to_latest, before_id)
- `label`: Debug label ('fetchLatestEvents' or 'fetchOlderEvents')

**Implementation**:
```typescript
const resp = await axios
  .get<SessionEventsResponse>(ctx.baseUrl, {
    headers: ctx.headers,
    params,
    timeout: 15000,
    validateStatus: () => true, // Never throw on HTTP errors
  })
  .catch(() => null)

if (!resp || resp.status !== 200) {
  logForDebugging(`[${label}] HTTP ${resp?.status ?? 'error'}`)
  return null
}
```

**Key Design Choices**:
- `validateStatus: () => true` prevents axios from throwing
- Returns `null` on any error (caller decides retry strategy)
- 15-second timeout for slow network conditions
- Debug logging for troubleshooting

---

## Line-by-Line Analysis

### Response Type Transformation (Lines 62-66)

```typescript
return {
  events: Array.isArray(resp.data.data) ? resp.data.data : [],
  firstId: resp.data.first_id,
  hasMore: resp.data.has_more,
}
```

**Purpose**: Transform API response to internal `HistoryPage` type.

**Key Points**:
- `Array.isArray()` guard handles malformed API responses
- CamelCase → camelCase transformation (`first_id` → `firstId`)
- `has_more` boolean passed through directly
- Defensive: empty array fallback if `data` is not an array

---

### Auth Header Construction (Lines 35-42)

```typescript
return {
  baseUrl: `${getOauthConfig().BASE_API_URL}/v1/sessions/${sessionId}/events`,
  headers: {
    ...getOAuthHeaders(accessToken),
    'anthropic-beta': 'ccr-byoc-2025-07-29',
    'x-organization-uuid': orgUUID,
  },
}
```

**Purpose**: Build complete auth context for session events API.

**Key Headers**:
- OAuth headers from `getOAuthHeaders()` (Bearer token, etc.)
- `anthropic-beta`: Feature flag for CCR/BYOC (Claude Code Remote/BYOC)
- `x-organization-uuid`: Multi-tenant organization isolation

**Security Note**: Token refresh happens in `prepareApiRequest()` - this function just packages the result.

---

### Error Handling Pattern (Lines 50-60)

```typescript
const resp = await axios
  .get<SessionEventsResponse>(ctx.baseUrl, {
    headers: ctx.headers,
    params,
    timeout: 15000,
    validateStatus: () => true,
  })
  .catch(() => null)

if (!resp || resp.status !== 200) {
  logForDebugging(`[${label}] HTTP ${resp?.status ?? 'error'}`)
  return null
}
```

**Purpose**: Graceful error handling without throwing.

**Key Design Choices**:
1. `validateStatus: () => true` - Never throw on HTTP errors
2. `.catch(() => null)` - Network errors return null
3. Status check with optional chaining (`resp?.status`)
4. Debug logging for troubleshooting

**Why This Pattern?**:
- Callers expect `null` for "not available" (common case)
- No try/catch boilerplate in calling code
- Consistent with other fetch functions in codebase

---

## Integration Points

### Session History UI Components

**Location**: `components/history/` (likely)

**Integration**:
```typescript
// Component fetches initial page
const ctx = await createHistoryAuthCtx(sessionId)
const page = await fetchLatestEvents(ctx)

// User scrolls up → fetch older
const olderPage = await fetchOlderEvents(ctx, page.firstId!)
```

---

### Session ID Flow

```
URL: /sessions/:sessionId
       ↓
Component extracts sessionId
       ↓
createHistoryAuthCtx(sessionId)
       ↓
fetchLatestEvents() / fetchOlderEvents()
       ↓
SDKMessage[] rendered in UI
```

---

### Auth System Integration

**Dependencies**:
- `prepareApiRequest()` - OAuth token + org UUID
- `getOAuthHeaders()` - Header construction
- `getOauthConfig()` - API base URL

**Auth Flow**:
```
fetchLatestEvents()
    ↓
createHistoryAuthCtx() (called once, reused)
    ↓
prepareApiRequest() → { accessToken, orgUUID }
    ↓
getOAuthHeaders(accessToken) → headers
    ↓
axios.get() with headers
```

---

### SDK Message Types

**Dependency**: `entrypoints/agentSdkTypes.ts`

**Type Import**:
```typescript
import type { SDKMessage } from '../entrypoints/agentSdkTypes.js'
```

**SDKMessage Structure** (probable):
```typescript
{
  type: 'user' | 'assistant' | 'system'
  message: {
    content: ContentBlock[]
    role: string
    usage?: TokenUsage
  }
}
```

---

### Cursor-Based Pagination Protocol

**API Contract**:
```
GET /v1/sessions/{sessionId}/events?limit=100&anchor_to_latest=true
→ { data: [...], first_id: "evt_123", has_more: true }

GET /v1/sessions/{sessionId}/events?limit=100&before_id=evt_123
→ { data: [...], first_id: "evt_456", has_more: true }
```

**Client-Side Cursor Management**:
```typescript
let page = await fetchLatestEvents(ctx)
display(page.events)

while (page.hasMore) {
  page = await fetchOlderEvents(ctx, page.firstId!)
  prepend(page.events) // Older events go at top
}
```

---

## Performance Considerations

### Auth Context Reuse

**Anti-pattern** (avoid):
```typescript
// BAD: Re-authenticates on every page
for (let i = 0; i < 10; i++) {
  const ctx = await createHistoryAuthCtx(sessionId)
  const page = await fetchOlderEvents(ctx, cursor)
}
```

**Recommended Pattern**:
```typescript
// GOOD: Auth once, reuse
const ctx = await createHistoryAuthCtx(sessionId)
let page = await fetchLatestEvents(ctx)

while (page.hasMore) {
  page = await fetchOlderEvents(ctx, page.firstId!)
}
```

**Benefit**: Single OAuth token fetch instead of N fetches.

---

### Memory Efficiency

**Design Choice**: Returns new `HistoryPage` array each call (no caching).

**Rationale**:
- History is append-only (old events don't change)
- Caller controls caching strategy
- Prevents stale data issues

**Recommended Caching** (caller-side):
```typescript
const cache = new Map<string, HistoryPage>()

async function getCachedPage(beforeId: string | null): Promise<HistoryPage> {
  const key = beforeId ?? 'latest'
  if (cache.has(key)) return cache.get(key)!
  const page = beforeId
    ? await fetchOlderEvents(ctx, beforeId)
    : await fetchLatestEvents(ctx)
  cache.set(key, page!)
  return page!
}
```

---

## Summary

The `assistant/sessionHistory.ts` module is a **minimal, focused pagination layer** for session event retrieval. Key characteristics:

1. **Single Responsibility**: Only handles pagination, not rendering or state management
2. **Cursor-Based**: Bi-directional pagination via `anchor_to_latest` and `before_id`
3. **Auth-Efficient**: Context reuse pattern avoids redundant token fetches
4. **Error-Resilient**: Returns `null` on errors, never throws
5. **Type-Safe**: Full TypeScript types for API response transformation

**88 lines** of source code implementing a complete pagination system.
