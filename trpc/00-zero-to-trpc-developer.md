---
location: /home/darkvoid/Boxxed/@dev/repo-expolorations/trpc
source: github.com/trpc/trpc
explored_at: 2026-04-05
prerequisites: TypeScript knowledge, React familiarity, Basic HTTP API concepts
---

# Zero to tRPC Developer - Complete Fundamentals

## Table of Contents

1. [What is tRPC?](#what-is-trpc)
2. [Core Concepts](#core-concepts)
3. [Getting Started](#getting-started)
4. [Your First tRPC API](#your-first-trpc-api)
5. [Procedures and Routers](#procedures-and-routers)
6. [Input Validation](#input-validation)
7. [Middleware](#middleware)
8. [React Integration](#react-integration)
9. [Client Configuration](#client-configuration)
10. [Error Handling](#error-handling)
11. [Server Adapters](#server-adapters)
12. [Deployment](#deployment)

## What is tRPC?

**tRPC** (TypeScript Remote Procedure Call) is a library that enables fully typesafe APIs without schemas or code generation. Types are inferred directly from server-side code and shared automatically with the client.

### The Problem tRPC Solves

Traditional API development:
```
1. Define API schema (OpenAPI, GraphQL schema)
2. Generate client types
3. Implement server handlers
4. Implement client calls
5. Keep schema in sync with implementation
6. Manual type updates when API changes
```

tRPC approach:
```
1. Define server procedures with TypeScript
2. Types automatically inferred on client
3. No schema files, no code generation
4. Full autocompletion and type safety
5. Change server → client updates automatically
```

### Key Features

| Feature | Description |
|---------|-------------|
| **Zero Code Generation** | Types inferred at build time, no generation step |
| **Full Type Safety** | Inputs, outputs, and errors all typed |
| **Autocompletion** | IDE knows all procedures and their types |
| **Framework Agnostic** | Works with Express, Next.js, Fastify, standalone |
| **React Query Integration** | Built-in hooks with caching and invalidation |
| **Small Bundle** | Client only imports types, not implementation |

### tRPC vs Alternatives

| Approach | Type Safety | Code Gen | Runtime | Learning Curve |
|----------|-------------|----------|---------|----------------|
| **tRPC** | ⚡⚡⚡ Full | None | TypeScript | Low |
| **REST + OpenAPI** | ⚡ Partial | Required | Any | Medium |
| **GraphQL** | ⚡⚡ Schema | Optional | Any | High |
| **gRPC** | ⚡⚡ Proto | Required | Multi | High |
| **Raw Fetch** | ❌ None | None | Any | Low |

## Core Concepts

### 1. Procedures

Procedures are the building blocks of tRPC APIs:
```typescript
// Server-side procedure
const getUser = publicProcedure
  .input(z.object({ id: z.string() }))
  .query(({ input }) => {
    return { id: input.id, name: 'Alice' }
  })
```

### 2. Routers

Routers organize procedures into a hierarchical structure:
```typescript
const appRouter = t.router({
  users: t.router({
    get: getUser,
    list: listUsers,
  }),
  posts: t.router({
    get: getPost,
    create: createPost,
  }),
})
```

### 3. Context

Context is request-scoped data available to all procedures:
```typescript
export type Context = {
  user: { id: number; role: string } | null
  db: Database
}
```

### 4. Links

Links are client-side middleware for requests:
```typescript
const client = createTRPCClient({
  links: [
    httpBatchLink({ url: '/trpc' }),
    loggerLink(),
  ],
})
```

### 5. Type Inference

Types flow from server to client automatically:
```typescript
// Server defines
const router = t.router({
  greeting: publicProcedure
    .input(z.object({ name: z.string() }))
    .query(({ input }) => ({ text: `Hello ${input.name}` }))
})

// Client infers
const result = await client.greeting.query({ name: 'Alice' })
// result: { text: string }
```

## Getting Started

### Installation

```bash
# Server and client packages
npm install @trpc/server @trpc/client @trpc/react-query

# React Query (required for React integration)
npm install @tanstack/react-query

# Zod for validation (recommended)
npm install zod

# For Next.js
npm install @trpc/next
```

### Project Structure

```
my-app/
├── server/
│   ├── trpc/
│   │   ├── trpc.ts         # tRPC initialization
│   │   ├── router.ts       # App router
│   │   └── procedures/     # Procedure definitions
│   ├── server.ts           # Server entry
│   └── package.json
├── client/
│   ├── trpc/
│   │   ├── client.ts       # Client configuration
│   │   └── react.tsx       # React hooks
│   ├── App.tsx
│   └── package.json
└── shared/
    └── types.ts            # Shared types (optional)
```

## Your First tRPC API

### Step 1: Initialize tRPC

```typescript
// server/trpc/trpc.ts
import { initTRPC } from '@trpc/server'
import { z } from 'zod'

// Initialize tRPC
const t = initTRPC.create()

// Base procedures
export const publicProcedure = t.procedure
export const router = t.router
```

### Step 2: Create Router

```typescript
// server/trpc/router.ts
import { publicProcedure, router } from './trpc'

export const appRouter = router({
  // Simple query
  greeting: publicProcedure.query(() => {
    return 'Hello from tRPC!'
  }),
  
  // Query with input
  user: publicProcedure
    .input(z.object({ id: z.string() }))
    .query(({ input }) => {
      return { id: input.id, name: 'Alice' }
    }),
  
  // Mutation
  createUser: publicProcedure
    .input(z.object({ name: z.string() }))
    .mutation(({ input }) => {
      return { id: '1', name: input.name }
    }),
})

export type AppRouter = typeof appRouter
```

### Step 3: Create Server

```typescript
// server/server.ts
import { createHTTPServer } from '@trpc/server/adapters/node-http'
import { appRouter } from './trpc/router'

const server = createHTTPServer({
  router: appRouter,
})

server.listen(3000, () => {
  console.log('Server running on http://localhost:3000')
})
```

### Step 4: Create Client

```typescript
// client/trpc/client.ts
import { createTRPCClient, httpBatchLink } from '@trpc/client'
import type { AppRouter } from '../../server/trpc/router'

export const client = createTRPCClient<AppRouter>({
  links: [
    httpBatchLink({
      url: 'http://localhost:3000',
    }),
  ],
})
```

### Step 5: Use in Application

```typescript
// client/app.ts
async function main() {
  // Call greeting
  const greeting = await client.greeting.query()
  console.log(greeting)  // "Hello from tRPC!"
  
  // Call user with input
  const user = await client.user.query({ id: '123' })
  console.log(user)  // { id: "123", name: "Alice" }
  
  // Create user
  const newUser = await client.createUser.mutate({ name: 'Bob' })
  console.log(newUser)  // { id: "1", name: "Bob" }
}

main()
```

## Procedures and Routers

### Procedure Types

```typescript
// Public procedure (no auth required)
const publicProc = t.procedure

// Protected procedure (requires auth)
const protectedProc = t.procedure.use(isAuthenticatedMiddleware)

// Admin procedure (requires admin role)
const adminProc = t.procedure.use(isAdminMiddleware)
```

### Query Procedures

```typescript
// Basic query
t.procedure.query(() => {
  return { message: 'Hello' }
})

// Query with input
t.procedure
  .input(z.object({ id: z.string() }))
  .query(({ input }) => {
    return { id: input.id }
  })

// Query with context
t.procedure
  .query(({ ctx }) => {
    return { user: ctx.user }
  })
```

### Mutation Procedures

```typescript
// Basic mutation
t.procedure
  .input(z.object({ name: z.string() }))
  .mutation(({ input }) => {
    return { id: 1, name: input.name }
  })

// Mutation with side effects
t.procedure
  .input(z.object({ email: z.string().email() }))
  .mutation(async ({ input, ctx }) => {
    await ctx.db.user.create({ email: input.email })
    return { success: true }
  })
```

### Subscription Procedures

```typescript
// Real-time subscription
t.procedure
  .input(z.object({ channelId: z.string() }))
  .subscription(async function* ({ input, ctx }) {
    for await (const message of ctx.subscribeToChannel(input.channelId)) {
      yield message
    }
  })
```

### Router Composition

```typescript
// Nested routers
const userRouter = t.router({
  get: getUser,
  list: listUsers,
  create: createUser,
})

const postRouter = t.router({
  get: getPost,
  list: listPosts,
  create: createPost,
})

export const appRouter = t.router({
  users: userRouter,
  posts: postRouter,
})

// Usage: client.users.get.query(), client.posts.list.query()
```

## Input Validation

### Zod Integration

```typescript
import { z } from 'zod'

// String validation
t.procedure
  .input(z.object({
    name: z.string().min(1).max(100),
    email: z.string().email(),
  }))
  .query(({ input }) => {})

// Number validation
t.procedure
  .input(z.object({
    page: z.number().int().positive(),
    limit: z.number().int().min(1).max(100),
  }))
  .query(({ input }) => {})

// Optional fields
t.procedure
  .input(z.object({
    search: z.string().optional(),
    filter: z.enum(['active', 'inactive']).optional(),
  }))
  .query(({ input }) => {})

// Arrays
t.procedure
  .input(z.object({
    ids: z.array(z.string()).min(1),
  }))
  .query(({ input }) => {})
```

### Custom Validation

```typescript
// Custom transform
t.procedure
  .input(z.object({
    date: z.string().transform(s => new Date(s)),
  }))
  .query(({ input }) => {
    // input.date is now a Date object
  })

// Refinement
t.procedure
  .input(z.object({
    password: z.string().refine(p => p.length >= 8, 'Password too short'),
  }))
  .mutation(({ input }) => {})

// Preprocess
t.procedure
  .input(z.object({
    email: z.string().email().preprocess(s => s.toLowerCase()),
  }))
  .mutation(({ input }) => {})
```

## Middleware

### Basic Middleware

```typescript
// Logger middleware
const loggerMiddleware = t.middleware(async ({ path, type, next, ctx }) => {
  const start = Date.now()
  const result = await next()
  const duration = Date.now() - start
  
  console.log(`${type} ${path} - ${duration}ms`)
  
  return result
})

// Auth middleware
const authMiddleware = t.middleware(async ({ ctx, next }) => {
  const user = await getUserFromToken(ctx.token)
  
  return next({
    ctx: {
      ...ctx,
      user,
    },
  })
})
```

### Middleware Chains

```typescript
// Combine middleware
const protectedProcedure = t.procedure
  .use(loggerMiddleware)
  .use(authMiddleware)

// Admin-only procedure
const adminProcedure = protectedProcedure.use(async ({ ctx, next }) => {
  if (ctx.user?.role !== 'admin') {
    throw new TRPCError({ code: 'FORBIDDEN' })
  }
  
  return next({
    ctx: {
      ...ctx,
      user: ctx.user,
    },
  })
})
```

### Context Extension

```typescript
// Middleware that extends context
const withDb = t.middleware(async ({ ctx, next }) => {
  const db = new Database()
  
  const result = await next({
    ctx: {
      ...ctx,
      db,
    },
  })
  
  await db.close()
  return result
})
```

## React Integration

### Provider Setup

```typescript
// client/trpc/react.tsx
import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import { httpBatchLink } from '@trpc/client'
import { createTRPCReact } from '@trpc/react-query'
import type { AppRouter } from '../../server/trpc/router'

export const trpc = createTRPCReact<AppRouter>()

const queryClient = new QueryClient()

export function TRPCProvider({ children }: { children: React.ReactNode }) {
  const [client] = useState(() =>
    createTRPCClient<AppRouter>({
      links: [
        httpBatchLink({
          url: 'http://localhost:3000',
        }),
      ],
    })
  )
  
  return (
    <trpc.Provider client={client} queryClient={queryClient}>
      <QueryClientProvider client={queryClient}>
        {children}
      </QueryClientProvider>
    </trpc.Provider>
  )
}
```

### Using Hooks

```typescript
// Query hook
function UserProfile({ userId }: { userId: string }) {
  const { data, isLoading, error } = trpc.user.query({ id: userId })
  
  if (isLoading) return <div>Loading...</div>
  if (error) return <div>Error: {error.message}</div>
  
  return <div>{data.name}</div>
}

// Mutation hook
function CreateUser() {
  const utils = trpc.useContext()
  const createUser = trpc.createUser.useMutation({
    onSuccess: () => {
      utils.user.invalidate()  // Refetch users
    },
  })
  
  const handleSubmit = (e: FormEvent) => {
    createUser.mutate({ name: 'New User' })
  }
  
  return <button onClick={handleSubmit}>Create</button>
}

// Subscription hook
function ChatMessages({ channelId }: { channelId: string }) {
  const messages = trpc.messages.subscribe.useSubscription(
    { channelId },
    {
      onData: (message) => {
        console.log('New message:', message)
      },
    }
  )
  
  return <div>{/* Render messages */}</div>
}
```

## Client Configuration

### Links

```typescript
import { createTRPCClient, httpBatchLink, loggerLink } from '@trpc/client'

const client = createTRPCClient<AppRouter>({
  links: [
    // Logger (shows request/response in console)
    loggerLink({
      enabled: () => true,
    }),
    
    // Batch HTTP requests
    httpBatchLink({
      url: 'http://localhost:3000',
      maxURLLength: 2083,  // Browser limit
    }),
  ],
})
```

### HTTP Link Options

```typescript
httpBatchLink({
  url: '/api/trpc',
  
  // Headers
  headers() {
    const token = getAuthToken()
    return {
      Authorization: token ? `Bearer ${token}` : undefined,
    }
  },
  
  // Fetch options
  fetch(url, options) {
    return fetch(url, {
      ...options,
      credentials: 'include',
    })
  },
})
```

## Error Handling

### Error Types

```typescript
import { TRPCError } from '@trpc/server'

// Bad request
throw new TRPCError({
  code: 'BAD_REQUEST',
  message: 'Invalid input',
})

// Unauthorized
throw new TRPCError({
  code: 'UNAUTHORIZED',
  message: 'Please log in',
})

// Forbidden
throw new TRPCError({
  code: 'FORBIDDEN',
  message: 'Admin access required',
})

// Not found
throw new TRPCError({
  code: 'NOT_FOUND',
  message: 'User not found',
})

// Internal error
throw new TRPCError({
  code: 'INTERNAL_SERVER_ERROR',
  message: 'Something went wrong',
})
```

### Client-Side Error Handling

```typescript
try {
  await client.user.query({ id: '123' })
} catch (err) {
  if (err instanceof TRPCClientError) {
    console.log('tRPC error:', err.message)
    console.log('Error code:', err.shape?.code)
  }
}
```

## Server Adapters

### Express

```typescript
import express from 'express'
import { createExpressMiddleware } from '@trpc/server/adapters/express'
import { appRouter } from './trpc/router'

const app = express()

app.use('/trpc', createExpressMiddleware({
  router: appRouter,
  createContext: ({ req, res }) => ({ user: req.user }),
}))

app.listen(3000)
```

### Next.js

```typescript
// pages/api/trpc/[trpc].ts
import { createNextApiHandler } from '@trpc/server/adapters/next'
import { appRouter } from '../../../server/trpc/router'

export default createNextApiHandler({
  router: appRouter,
  createContext: ({ req, res }) => ({ user: req.user }),
})
```

### Fastify

```typescript
import fastify from 'fastify'
import { fastifyTRPCPlugin } from '@trpc/server/adapters/fastify'
import { appRouter } from './trpc/router'

const server = fastify()

server.register(fastifyTRPCPlugin, {
  prefix: '/trpc',
  trpcOptions: {
    router: appRouter,
    createContext: ({ req, res }) => ({ user: req.user }),
  },
})

server.listen({ port: 3000 })
```

### Standalone HTTP

```typescript
import { createHTTPServer } from '@trpc/server/adapters/node-http'
import { appRouter } from './trpc/router'

createHTTPServer({
  router: appRouter,
  createContext: () => ({}),
}).listen(3000)
```

## Deployment

### Vercel (Next.js)

```typescript
// vercel.json
{
  "buildCommand": "npm run build",
  "outputDirectory": ".next",
  "framework": "nextjs"
}
```

### Docker

```dockerfile
FROM node:20-alpine

WORKDIR /app

COPY package*.json ./
RUN npm ci --only=production

COPY . .

EXPOSE 3000

CMD ["node", "dist/server.js"]
```

### Environment Variables

```bash
# .env
DATABASE_URL=postgresql://user:pass@localhost:5432/app
JWT_SECRET=your-secret-key
NODE_ENV=production
```

## Conclusion

tRPC provides:

1. **End-to-End Type Safety**: No schema duplication, types inferred automatically
2. **Zero Code Generation**: Types flow from server to client at build time
3. **React Integration**: Built-in hooks with React Query caching
4. **Framework Agnostic**: Works with Express, Next.js, Fastify, standalone
5. **Small Bundle**: Client only imports types, not implementation
6. **Great DX**: Full autocompletion, instant feedback on API changes

## Next Steps

- [exploration.md](./exploration.md) - Full architecture deep dive
- [01-type-inference-deep-dive.md](./01-type-inference-deep-dive.md) - Type system internals
- [02-procedure-builder-deep-dive.md](./02-procedure-builder-deep-dive.md) - Builder pattern
- [rust-revision.md](./rust-revision.md) - tRPC patterns in Rust
- [production-grade.md](./production-grade.md) - Production deployment patterns
