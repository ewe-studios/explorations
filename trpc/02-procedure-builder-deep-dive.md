---
source: /home/darkvoid/Boxxed/@formulas/src.trpc
repository: github.com/trpc/trpc
explored_at: 2026-04-05
focus: Builder pattern for procedures, middleware chains, context extension, procedural type construction
---

# Deep Dive: Procedure Builder Pattern

## Overview

This deep dive examines tRPC's procedure builder pattern - a fluent API for constructing procedures with input validation, middleware chains, and context extension. The builder pattern enables composable, type-safe procedure definitions.

## Builder Pattern Fundamentals

```typescript
// @trpc/server/src/core/procedure.ts

// The builder interface
export interface ProcedureBuilder<TDef extends ProcedureDefinition> {
  _def: TDef
  
  // Chainable methods return new builder with updated types
  input<$Input>(
    input: Parser<TDef['_input_in'], $Input>
  ): ProcedureBuilder<{
    _input_in: $Input
    _input_out: $Input
    _output_out: TDef['_output_out']
    _meta: TDef['_meta']
  }>
  
  use<$MiddlewareArgs>(
    fn: MiddlewareFunction<TDef, $MiddlewareArgs>
  ): ProcedureBuilder<{
    _input_in: TDef['_input_in']
    _input_out: TDef['_input_out']
    _output_out: TDef['_output_out']
    _meta: TDef['_meta']
  }>
  
  output<$OutputOut>(
    output: Parser<TDef['_output_out'], $OutputOut>
  ): ProcedureBuilder<{
    _input_in: TDef['_input_in']
    _input_out: TDef['_input_out']
    _output_out: $OutputOut
    _meta: TDef['_meta']
  }>
  
  // Terminal methods
  query(fn: QueryResolver<TDef['_input_out'], TDef['_output_out']>): QueryProcedure<TDef>
  mutation(fn: MutationResolver<TDef['_input_out'], TDef['_output_out']>): MutationProcedure<TDef>
  subscription(fn: SubscriptionResolver<...>): SubscriptionProcedure<TDef>
}

// Base definition
interface ProcedureDefinition {
  _input_in: any
  _input_out: any
  _output_out: any
  _meta: any
}
```

## Building a Procedure

```typescript
// Initialize tRPC
const t = initTRPC.create()

// Start with base procedure
const baseProcedure = t.procedure

// Add input validation
const withInput = baseProcedure.input(z.object({
  id: z.string(),
}))

// Add middleware
const withAuth = withInput.use(async ({ ctx, next }) => {
  const user = await getUser(ctx.token)
  return next({ ctx: { ...ctx, user } })
})

// Define the handler
const getUserProcedure = withAuth.query(({ input, ctx }) => {
  return {
    id: input.id,
    user: ctx.user,
  }
})

// Fluent chain (typical usage)
const userProc = t.procedure
  .input(z.object({ id: z.string() }))
  .use(authMiddleware)
  .use(loggingMiddleware)
  .query(({ input, ctx }) => {
    return db.user.findUnique({ where: { id: input.id } })
  })
```

## Middleware Chains

```typescript
// @trpc/server/src/core/middleware.ts

// Middleware function type
type MiddlewareFunction<TDef, TArgs> = (opts: {
  ctx: inferRouterContext<any>
  input: TDef['_input_out']
  next: {
    (args?: { ctx: Partial<inferRouterContext<any>> }): Promise<any>
  }
  path: string
  type: 'query' | 'mutation' | 'subscription'
}) => Promise<any>

// Logger middleware
const loggerMiddleware = t.middleware(async ({ path, type, next, ctx }) => {
  const start = Date.now()
  
  console.log(`→ ${type} ${path} started`)
  
  const result = await next()
  
  const duration = Date.now() - start
  console.log(`← ${type} ${path} completed in ${duration}ms`)
  
  return result
})

// Auth middleware
const authMiddleware = t.middleware(async ({ ctx, next }) => {
  const user = await getUserFromToken(ctx.token)
  
  if (!user) {
    throw new TRPCError({
      code: 'UNAUTHORIZED',
      message: 'Please log in',
    })
  }
  
  return next({
    ctx: {
      ...ctx,
      user,
    },
  })
})

// Admin middleware (builds on auth)
const adminMiddleware = t.middleware(async ({ ctx, next }) => {
  if (ctx.user?.role !== 'admin') {
    throw new TRPCError({
      code: 'FORBIDDEN',
      message: 'Admin access required',
    })
  }
  
  return next({
    ctx: {
      ...ctx,
      user: ctx.user,
    },
  })
})

// Rate limit middleware
const rateLimitMiddleware = t.middleware(async ({ ctx, path, next }) => {
  const key = `rate-limit:${ctx.ip}:${path}`
  const count = await redis.incr(key)
  
  if (count === 1) {
    await redis.expire(key, 60)  // 1 minute window
  }
  
  if (count > 100) {
    throw new TRPCError({
      code: 'TOO_MANY_REQUESTS',
      message: 'Rate limit exceeded',
    })
  }
  
  return next()
})

// Combine middleware into procedure types
const publicProcedure = t.procedure
  .use(loggerMiddleware)
  .use(rateLimitMiddleware)

const protectedProcedure = publicProcedure
  .use(authMiddleware)

const adminProcedure = protectedProcedure
  .use(adminMiddleware)

// Usage
const router = t.router({
  public: publicProcedure.query(() => 'Anyone can see'),
  protected: protectedProcedure.query(({ ctx }) => ctx.user),
  admin: adminProcedure.query(() => 'Admins only'),
})
```

## Context Extension

```typescript
// @trpc/server/src/core/context.ts

// Define base context
interface BaseContext {
  token?: string
  ip: string
}

// Middleware extends context
const withDbMiddleware = t.middleware(async ({ ctx, next }) => {
  const db = new Database(process.env.DATABASE_URL!)
  
  try {
    const result = await next({
      ctx: {
        ...ctx,
        db,
      },
    })
    return result
  } finally {
    await db.close()
  }
})

// Context typing with module augmentation
declare module './trpc' {
  interface Context {
    user?: { id: string; role: string }
    db: Database
  }
}

// Multiple context extensions
const withCacheMiddleware = t.middleware(async ({ ctx, next }) => {
  const cache = new RedisCache(process.env.REDIS_URL!)
  
  return next({
    ctx: {
      ...ctx,
      cache,
    },
  })
})

// Chain context extensions
const enrichedProcedure = t.procedure
  .use(withDbMiddleware)
  .use(withCacheMiddleware)
  .use(authMiddleware)

// All context properties available in handler
enrichedProcedure.query(({ ctx, input }) => {
  // ctx has: token, ip, db, cache, user
  const cached = await ctx.cache.get('key')
  const data = await ctx.db.query('SELECT * FROM ...')
  return { cached, data, user: ctx.user }
})
```

## Output Transformation

```typescript
// @trpc/server/src/core/procedure.ts

// Transform output before sending to client
const sanitizedProcedure = t.procedure
  .input(z.object({ id: z.string() }))
  .output(z.object({
    id: z.string(),
    email: z.string(),
    // password field excluded from output schema
  }))
  .query(async ({ input }) => {
    const user = await db.user.findUnique({
      where: { id: input.id },
    })
    
    // Full user from DB includes password
    // Output transformer strips it
    return {
      id: user.id,
      email: user.email,
    }
  })

// Date serialization
const dateProcedure = t.procedure
  .output(z.object({
    createdAt: z.string().datetime(),
    updatedAt: z.string().datetime(),
  }))
  .query(async () => {
    const record = await db.record.findFirst()
    return {
      createdAt: record.createdAt.toISOString(),
      updatedAt: record.updatedAt.toISOString(),
    }
  })
```

## Builder Composition Patterns

```typescript
// Reusable procedure configurations

// Pattern 1: Factory function
function createProcedure(prefix: string) {
  return t.procedure
    .use(loggerMiddleware)
    .use((opts) => {
      console.log(`[${prefix}] ${opts.path}`)
      return opts.next()
    })
}

const userProcedure = createProcedure('users')
const postProcedure = createProcedure('posts')

// Pattern 2: Inheritance via spread
const baseOptions = {
  maxDuration: 30000,
  retry: { limit: 3 },
}

const procedureWithOptions = t.procedure
  .input(z.object({ id: z.string() }))
  .use(loggingMiddleware)

// Pattern 3: Conditional middleware
const withOptionalAuth = t.middleware(async ({ ctx, next, type }) => {
  // Only require auth for mutations
  if (type === 'mutation') {
    const user = await getUser(ctx.token)
    if (!user) {
      throw new TRPCError({ code: 'UNAUTHORIZED' })
    }
    return next({ ctx: { ...ctx, user } })
  }
  return next()
})

// Pattern 4: Scoped procedures
const userScopedProcedure = t.procedure
  .input(z.object({ userId: z.string() }))
  .use(authMiddleware)
  .use(async ({ ctx, input, next }) => {
    // Ensure user can only access their own data
    if (ctx.user.id !== input.userId) {
      throw new TRPCError({ code: 'FORBIDDEN' })
    }
    return next()
  })
```

## Advanced Builder Patterns

```typescript
// Transaction support
const transactionProcedure = t.procedure
  .use(async ({ ctx, next }) => {
    const tx = await db.$transaction()
    
    try {
      const result = await next({
        ctx: { ...ctx, tx },
      })
      await tx.commit()
      return result
    } catch (error) {
      await tx.rollback()
      throw error
    }
  })

// Caching decorator
const cachedProcedure = (ttl: number) => 
  t.procedure.use(async ({ path, next, cache }) => {
    const cacheKey = `trpc:${path}:${JSON.stringify(input)}`
    
    const cached = await cache.get(cacheKey)
    if (cached) return cached
    
    const result = await next()
    await cache.set(cacheKey, result, ttl)
    return result
  })

// Pagination helper
const paginatedProcedure = t.procedure
  .input(z.object({
    cursor: z.string().optional(),
    limit: z.number().min(1).max(100),
  }))
  .output(z.object({
    items: z.array(z.any()),
    nextCursor: z.string().optional(),
    hasMore: z.boolean(),
  }))

// File upload handling
const uploadProcedure = t.procedure
  .input(z.object({
    file: z.instanceof(File),
  }))
  .use(async ({ input, next, storage }) => {
    const url = await storage.upload(input.file)
    return next({
      ctx: { fileUrl: url },
    })
  })
```

## Type Safety Through the Chain

```typescript
// Each step in the chain maintains type safety

// Step 1: Base procedure
const base = t.procedure
// Type: ProcedureBuilder<{ _input_in: void, _output_out: any }>

// Step 2: Add input
const withInput = base.input(z.object({ id: z.string() }))
// Type: ProcedureBuilder<{ _input_in: { id: string }, _output_out: any }>

// Step 3: Add middleware
const withMiddleware = withInput.use(authMiddleware)
// Type: ProcedureBuilder<{ _input_in: { id: string }, _output_out: any }>
// Context now includes user

// Step 4: Define handler
const handler = withMiddleware.query(({ input, ctx }) => {
  // input: { id: string }
  // ctx: { token?: string, ip: string, user: User }
  return { id: input.id }
})
// Type: QueryProcedure<{ _input_in: { id: string }, _output_out: { id: string } }>

// TypeScript enforces the chain:
base.query()  // ✓ Works (no input required)
withInput.query()  // ✓ Works
withInput.mutation()  // ✓ Works

// Type error if you try wrong input:
withInput.query(({ input }) => {
  input.email  // Error: Property 'email' does not exist
})
```

## Error Handling in Builders

```typescript
// Centralized error handling
const withErrorHandling = t.middleware(async ({ path, type, next }) => {
  try {
    return await next()
  } catch (error) {
    if (error instanceof TRPCError) {
      // Already a tRPC error, rethrow
      throw error
    }
    
    // Convert unknown errors to TRPC errors
    throw new TRPCError({
      code: 'INTERNAL_SERVER_ERROR',
      message: error.message,
      cause: error,
    })
  }
})

// Input validation errors
const withValidation = t.middleware(async ({ input, next, path }) => {
  try {
    return await next()
  } catch (error) {
    if (error instanceof z.ZodError) {
      throw new TRPCError({
        code: 'BAD_REQUEST',
        message: 'Invalid input',
        data: {
          zodError: error.errors,
        },
      })
    }
    throw error
  }
})

// Usage
const safeProcedure = t.procedure
  .use(withErrorHandling)
  .use(withValidation)
```

## Real-World Example: Full Procedure Stack

```typescript
// Complete procedure with all concerns handled

const productionProcedure = t.procedure
  // 1. Logging (first in chain)
  .use(loggerMiddleware)
  // 2. Rate limiting
  .use(rateLimitMiddleware)
  // 3. Error handling wrapper
  .use(withErrorHandling)
  // 4. Authentication
  .use(authMiddleware)
  // 5. Database connection
  .use(withDbMiddleware)
  // 6. Caching layer
  .use(cachedMiddleware)
  // 7. Input validation
  .input(z.object({
    id: z.string().uuid(),
    include: z.array(z.enum(['posts', 'comments'])).optional(),
  }))
  // 8. Output validation
  .output(z.object({
    id: z.string(),
    name: z.string(),
    email: z.string().email(),
  }))
  // 9. Handler
  .query(async ({ input, ctx }) => {
    const cacheKey = `user:${input.id}`
    
    const cached = await ctx.cache.get(cacheKey)
    if (cached) return cached
    
    const user = await ctx.db.user.findUnique({
      where: { id: input.id },
      include: {
        posts: input.include?.includes('posts'),
        comments: input.include?.includes('comments'),
      },
    })
    
    if (!user) {
      throw new TRPCError({ code: 'NOT_FOUND' })
    }
    
    const result = {
      id: user.id,
      name: user.name,
      email: user.email,
    }
    
    await ctx.cache.set(cacheKey, result, 300)  // 5 min cache
    
    return result
  })
```

## Conclusion

tRPC's procedure builder pattern provides:

1. **Fluent API**: Chainable methods for composing procedures
2. **Type Preservation**: Types flow through each step of the chain
3. **Middleware Composition**: Stackable middleware for cross-cutting concerns
4. **Context Extension**: Middleware can add to context type-safely
5. **Output Transformation**: Validate and transform output before sending
6. **Error Handling**: Centralized error handling in middleware
7. **Reusability**: Create procedure templates for common patterns

The builder pattern makes complex procedure composition ergonomic while maintaining full type safety throughout the chain.
