---
location: /home/darkvoid/Boxxed/@formulas/src.interval
repository: github:interval/interval-node (Node.js), github:interval/interval-py (Python)
explored_at: 2026-03-20T00:00:00.000Z
language: TypeScript, Python
---

# Project Exploration: Interval - Internal Tools Platform

## Overview

Interval is a platform for building internal web applications (admin panels, customer support tools, operational dashboards) by writing backend code only. Instead of building frontend UIs, developers write asynchronous functions that use Interval's I/O methods to request input and display output. The platform automatically generates the web UI.

**Core Philosophy:** "Node/Python code > no-code" - Building UIs for internal tools should happen in your codebase with version control, not in a drag-and-drop web builder.

## Directory Structure

```
/home/darkvoid/Boxxed/@formulas/src.interval/
├── server/                    # Self-hosted Interval Server (open-source)
│   ├── src/
│   │   ├── components/        # React UI components
│   │   ├── pages/             # Dashboard pages (tsx, mdx)
│   │   ├── server/            # Express server routes
│   │   ├── wss/               # WebSocket server
│   │   ├── emails/            # Email templates
│   │   └── entry.ts           # CLI entry point
│   ├── prisma/                # Database schema
│   └── package.json
├── interval-node/             # Node.js/TypeScript SDK
│   ├── src/
│   │   ├── classes/           # Core SDK classes
│   │   │   ├── IntervalClient.ts   # Main connection handler
│   │   │   ├── IOClient.ts         # I/O handling per transaction
│   │   │   ├── IOPromise.ts        # Promise wrapper for IO
│   │   │   ├── IOComponent.ts      # Component definitions
│   │   │   ├── ISocket.ts          # WebSocket wrapper
│   │   │   ├── DuplexRPCClient.ts  # RPC communication
│   │   │   ├── Action.ts           # Action definition
│   │   │   ├── Page.ts             # Page/router definition
│   │   │   └── Layout.ts           # Page layout
│   │   ├── components/        # IO component definitions
│   │   ├── examples/          # Example implementations
│   │   └── index.ts           # Main export
│   └── package.json
├── interval-py/               # Python SDK
│   ├── src/
│   │   └── interval_sdk/
│   │       ├── classes/       # Python equivalents of Node classes
│   │       │   ├── io.py
│   │       │   ├── io_client.py
│   │       │   ├── io_promise.py
│   │       │   ├── action.py
│   │       │   └── isocket.py
│   │       ├── components/    # Table, grid components
│   │       └── superjson/     # Cross-language serialization
│   └── pyproject.toml
├── interval-examples/         # Example applications
│   ├── basic/                 # Simple starter examples
│   ├── refund-charges/        # Payment refund tool
│   ├── github-issue-editor/   # GitHub integration
│   ├── user-settings/         # User management
│   └── ... (more examples)
├── viewtube/                  # Demo video streaming app for examples
├── ai-portraits/              # AI image generation demo app
└── llm-bench/                 # LLM benchmarking tools
```

## Architecture

### High-Level Diagram

```
┌─────────────────────────────────────────────────────────────────┐
│                     Interval Cloud (or Self-Hosted)             │
│  ┌─────────────┐  ┌──────────────┐  ┌────────────────────────┐ │
│  │   Dashboard │  │   Action     │  │   WebSocket Server     │ │
│  │     UI      │  │   Runner     │  │   (wss://websocket)    │ │
│  └─────────────┘  └──────────────┘  └────────────────────────┘ │
│                          │                       │              │
└──────────────────────────┼───────────────────────┼──────────────┘
                           │                       │
                    HTTPS  │                       │ WebSocket
                    (REST) │                       │ (RPC)
                           │                       │
┌──────────────────────────┼───────────────────────┼──────────────┐
│                  Your Backend Application         │              │
│  ┌─────────────────────────────────────────────┐  │              │
│  │           Interval SDK (Node.js/Python)     │  │              │
│  │  ┌─────────────┐  ┌──────────────┐         │  │              │
│  │  │   Interval  │  │  IOClient    │         │  │              │
│  │  │   Client    │  │  (per tx)    │         │  │              │
│  │  └─────────────┘  └──────────────┘         │  │              │
│  │         │                │                  │  │              │
│  │    declareHost      renderComponents        │  │              │
│  └─────────────────────────────────────────────┘  │              │
│                          │                        │              │
│  ┌─────────────────────────────────────────────┐  │              │
│  │           Your Actions (handlers)           │  │              │
│  │  async function editUser(io, ctx) {         │  │              │
│  │    const email = await io.input.text(...)   │◄─┘              │
│  │    await io.display.table(...)              │                 │
│  │    return { success: true }                 │                 │
│  └─────────────────────────────────────────────┘                 │
└──────────────────────────────────────────────────────────────────┘
```

## Server Platform

The Interval Server is the central hub that:

1. **Hosts the Dashboard UI** - React-based web interface for running actions
2. **Manages WebSocket Connections** - Persistent connections to SDK hosts
3. **Handles Authentication** - User sessions, teams, permissions
4. **Runs Actions** - Executes action handlers via RPC
5. **Renders I/O Components** - Displays forms, tables, and collects input

### Key Server Technologies

- **Framework:** Express.js + Vite (React frontend)
- **Database:** PostgreSQL with Prisma ORM
- **Real-time:** WebSocket server on `/websocket`
- **RPC Protocol:** Custom duplex RPC over WebSocket
- **UI:** React with Tailwind CSS, Ariakit, TipTap editor

### Server Entry Point (`server/src/entry.ts`)

The server CLI provides:
- `interval-server start` - Starts the server on port 3000
- `interval-server db-init` - Initializes the PostgreSQL database

## Action Definition and Execution Model

### Defining Actions (Node.js)

Actions are async functions that receive `io` and `ctx` parameters:

```typescript
import Interval from '@interval/sdk'

const interval = new Interval({
  apiKey: process.env.INTERVAL_KEY,
  routes: {
    edit_user: async (io, ctx) => {
      // Request input
      const email = await io.input.email('User email', {
        defaultValue: ctx.params.email,
      })

      // Display data
      await io.display.table('User data', {
        data: [{ email, status: 'active' }],
      })

      // Return result
      return { success: true }
    },
  },
})

interval.listen()
```

### Defining Actions (Python)

```python
from interval_sdk import Interval, IO

interval = Interval(api_key="your_key")

@interval.action
async def edit_user(io: IO):
    email = await io.input.email('User email')
    await io.display.table('User data', data=[{'email': email}])
    return {'success': True}

interval.listen()
```

### Action Configuration Options

| Property | Type | Description |
|----------|------|-------------|
| `handler` | Function | The async action function |
| `name` | string | Display name in dashboard |
| `description` | string | Help text for users |
| `backgroundable` | boolean | Can run in background |
| `unlisted` | boolean | Hidden from navigation |
| `warnOnClose` | boolean | Warn before closing during execution |
| `access` | object | Team/role-based permissions |

### Action Execution Flow

1. User clicks action in dashboard
2. Server sends `START_TRANSACTION` RPC to host
3. Host creates `IOClient` for the transaction
4. Action handler executes:
   - `await io.input.*()` pauses execution, sends render instruction to server
   - Server displays form, waits for user input
   - Server sends `IO_RESPONSE` with values
   - Action resumes with input values
5. Action returns result
6. Server sends `MARK_TRANSACTION_COMPLETE`

## I/O Methods (UI Builder)

Interval provides a comprehensive set of I/O methods that automatically generate UI:

### Input Methods (`io.input.*`)

| Method | Returns | Description |
|--------|---------|-------------|
| `text()` | string | Single or multiline text |
| `email()` | string | Validated email |
| `number()` | number | Integer or decimal |
| `slider()` | number | Range slider input |
| `boolean()` | boolean | Checkbox toggle |
| `richText()` | string | WYSIWYG HTML editor |
| `date()` | Date | Date picker |
| `time()` | Time | Time picker |
| `datetime()` | DateTime | Combined date/time |
| `url()` | URL | Validated URL |
| `file()` | IntervalFile | File upload |

### Selection Methods (`io.select.*`)

| Method | Returns | Description |
|--------|---------|-------------|
| `single()` | option | Dropdown/radio selection |
| `multiple()` | option[] | Multi-select checkboxes |
| `table()` | row[] | Select from tabular data |

### Display Methods (`io.display.*`)

| Method | Description |
|--------|-------------|
| `heading()` | Section heading with menu items |
| `markdown()` | Rendered markdown text |
| `html()` | Sanitized HTML |
| `code()` | Syntax-highlighted code |
| `table()` | Sortable/filterable data table |
| `grid()` | Card grid layout |
| `metadata()` | Key-value pairs (grid/list/card) |
| `object()` | JSON object tree |
| `link()` | Button-styled action link |
| `image()` | Image display |
| `video()` | Video player |

### Exclusive Methods

| Method | Description |
|--------|-------------|
| `io.confirm()` | Full-screen confirmation dialog |
| `io.confirmIdentity()` | MFA/password re-authentication |
| `io.search()` | Searchable result selector |

### Grouping I/O

Multiple inputs can be grouped into a single form:

```typescript
const [name, email, role] = await io.group([
  io.input.text('Name'),
  io.input.email('Email'),
  io.select.single('Role', { options: ['admin', 'user'] }),
])

// Or with object syntax for named returns
const { name, email, role } = await io.group({
  name: io.input.text('Name'),
  email: io.input.email('Email'),
  role: io.select.single('Role', { options: ['admin', 'user'] }),
})
```

## SDK Integration

### Node.js SDK Architecture

**Core Classes:**

1. **`Interval`** - Main class, manages connection and route registration
2. **`IntervalClient`** - WebSocket connection, RPC communication
3. **`IOClient`** - Per-transaction I/O handling
4. **`IOPromise`** - Promise wrapper for async I/O
5. **`IOComponent`** - Component definition and state management
6. **`ISocket`** - WebSocket wrapper with ACK protocol
7. **`DuplexRPCClient`** - Type-safe RPC over WebSocket

**Connection Flow:**

```typescript
// 1. Create Interval instance
const interval = new Interval({
  apiKey: 'your_key',
  endpoint: 'wss://app.interval.com/websocket',
  routes: { ... }
})

// 2. Establish connection
await interval.listen()

// Internally:
// - Creates ISocket WebSocket connection
// - Creates DuplexRPCClient for RPC
// - Sends INITIALIZE_HOST with action definitions
// - Receives organization/environment info
```

### Python SDK Architecture

The Python SDK mirrors the Node.js architecture:

**Core Modules:**

- `interval_sdk/classes/io.py` - IO class with all I/O methods
- `interval_sdk/classes/io_client.py` - IOClient for transaction handling
- `interval_sdk/classes/io_promise.py` - IOPromise wrapper
- `interval_sdk/classes/action.py` - Action decorator/decorator
- `interval_sdk/superjson/` - Cross-language serialization

**Usage Pattern:**

```python
from interval_sdk import Interval, IO, ctx_var

interval = Interval(api_key="key")

@interval.action
async def my_action(io: IO):
    ctx = ctx_var.get()  # Access context
    value = await io.input.text('Enter value')
    return {'value': value}

# Sync listen (blocking)
interval.listen()

# Or async with event loop
await interval.listen_async()
```

### SuperJSON Serialization

Interval uses a custom serialization format to handle complex types across languages:

- Dates, Maps, Sets
- BigInt, Typed Arrays
- Preserves type information between backend and frontend

## Pages and Routing

### File-System Routing

Actions can be organized using a routes directory:

```
routes/
├── users/
│   ├── edit.ts      # -> /users/edit
│   └── delete.ts    # -> /users/delete
├── reports/
│   └── generate.ts  # -> /reports/generate
└── dashboard.ts     # -> /dashboard
```

```typescript
const interval = new Interval({
  apiKey: process.env.INTERVAL_KEY,
  routesDirectory: path.resolve(__dirname, 'routes'),
})
```

### Programmatic Routing

```typescript
import { Page, Action } from '@interval/sdk'

const usersPage = new Page({
  name: 'Users',
  handler: async (display, ctx) => {
    return new Layout({
      title: 'User Management',
      children: [
        display.table('All users', { data: users }),
      ],
    })
  },
  routes: {
    edit: async (io, ctx) => { /* edit action */ },
    delete: async (io, ctx) => { /* delete action */ },
  },
})

const interval = new Interval({
  routes: {
    users: usersPage,
  },
})
```

## Context Object (`ctx`)

The context object provides action runtime information:

```typescript
interface ActionCtx {
  user: {
    email: string
    firstName: string | null
    lastName: string | null
    role: 'ADMIN' | 'DEVELOPER' | 'VIEWER'
    teams: string[]
  }
  params: SerializableRecord  // Query string params
  environment: 'development' | 'test' | 'production'
  organization: {
    name: string
    slug: string
  }
  action: {
    slug: string
    url: string
  }
  loading: TransactionLoadingState  // Loading indicators
  log: (...args: any[]) => void     // Logging to dashboard
  notify: (config: NotifyConfig) => void  // Send notifications
  redirect: (config: RedirectConfig) => void  // Navigate
}
```

## Loading States

Long-running actions can show progress:

```typescript
await ctx.loading.start({
  label: 'Migrating users...',
  description: 'This may take a few minutes',
  itemsInQueue: 1000,
})

for (const user of users) {
  await migrateUser(user)
  await ctx.loading.completeOne()
}

await ctx.loading.update('Finalizing...')
```

## Notifications

Actions can send notifications via email or Slack:

```typescript
await ctx.notify({
  title: 'Refund Processed',
  message: `Refunded $${amount} to ${customer.email}`,
  delivery: [
    { to: '#finance-alerts', method: 'SLACK' },
    { to: 'manager@example.com', method: 'EMAIL' },
  ],
})
```

## Queued Actions

Actions can be enqueued for later execution:

```typescript
// Enqueue an action
const job = await interval.enqueue('send_report', {
  assignee: 'user@example.com',
  params: { reportId: 123 },
})

// Later, dequeue and run
const { params } = await interval.dequeue(job.id)
```

## Self-Hosted Server

Interval Server can be self-hosted:

```bash
# Install globally
npm i -g @interval/server

# Initialize database
interval-server db-init

# Start server
interval-server start
```

### Required Environment Variables

```
DATABASE_URL=postgresql://user:pass@localhost/interval
SECRET=your_encryption_secret
APP_URL=http://localhost:3000
AUTH_COOKIE_SECRET=32+char_secret
WSS_API_SECRET=websocket_api_secret
```

### Optional Integrations

- **Postmark** - Email sending
- **WorkOS** - SSO, directory sync
- **Slack** - Slack notifications
- **S3** - File upload storage

## Example Applications

### ai-portraits

A full generative AI application with:
- Custom model training via Dreambooth
- Image generation with Stable Diffusion
- Gallery page for viewing results

Key patterns demonstrated:
- `Page` with multiple actions
- File uploads for training data
- Loading states for long operations
- Dynamic grid display

### viewtube

A demo video streaming platform with internal tools showing:
- User management
- Content moderation
- Analytics dashboards

## Key Insights

1. **Architecture Philosophy**: Interval inverts the traditional full-stack model - UI is generated from backend I/O calls rather than frontend code making API calls.

2. **State Management**: Each transaction has its own `IOClient` that manages the render loop. Components can update their state dynamically, triggering re-renders.

3. **Resilience**: The SDK handles reconnection, resending pending I/O calls, and maintaining transaction state across network interruptions.

4. **Type Safety**: The Node.js SDK uses extensive TypeScript generics to provide type-safe I/O methods with proper return types.

5. **Cross-Language**: The Python SDK mirrors the Node.js architecture, using SuperJSON for cross-language type preservation.

6. **Extensibility**: Pages can have dynamic handlers that return different layouts based on query params, enabling complex multi-step workflows.

7. **Security Model**: Access control is defined per-action with team-based and role-based permissions. Identity confirmation available for sensitive operations.

8. **Demo Mode**: The SDK supports a demo mode (`getClientHandlers`) for local development without connecting to the cloud.

## References

- [Interval Documentation](https://interval.com/docs)
- [Node.js SDK](https://github.com/interval/interval-node)
- [Python SDK](https://github.com/interval/interval-py)
- [Self-Hosted Server](https://github.com/interval/interval-node/tree/main/server)
