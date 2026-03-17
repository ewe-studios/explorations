---
type: guide
created: 2026-03-17
---

# CloudFlare Workers Deployment Guide

Complete guide for deploying Better Auth WASM to CloudFlare Workers with D2, R2, and KV.

## Prerequisites

- CloudFlare account (free tier works)
- Wrangler CLI installed (`npm install -g wrangler`)
- Node.js 18+
- Rust + wasm-pack installed

## Project Setup

### 1. Enable CloudFlare Services

```bash
# Login to CloudFlare
wrangler login

# List your account
wrangler whoami
```

### 2. Create D2 Database

```bash
# Create database
wrangler d2 create better-auth-db

# Output:
# ✅ Created database 'better-auth-db' (uuid: xxxxxxxx-xxxx-xxxx-xxxx-xxxxxxxxxxxx)
```

### 3. Create R2 Bucket

```bash
# Create bucket
wrangler r2 bucket create better-auth-storage

# Output:
# ✅ Created bucket 'better-auth-storage'
```

### 4. Create KV Namespace

```bash
# Create namespace
wrangler kv:namespace create CACHE

# Output:
# ✅ Created namespace for CACHE
# id = "yyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyy"
```

### 5. Configure wrangler.toml

```toml
name = "better-auth-worker"
main = "src/worker.ts"
compatibility_date = "2024-01-01"
compatibility_flags = ["nodejs_compat"]

# Environment variables
[vars]
ENVIRONMENT = "production"
BETTER_AUTH_URL = "https://auth.yourdomain.com"
LOG_LEVEL = "info"

# D2 Database
[[d2_databases]]
binding = "DB"
database_name = "better-auth-db"
database_id = "xxxxxxxx-xxxx-xxxx-xxxx-xxxxxxxxxxxx"

# R2 Bucket
[[r2_buckets]]
binding = "BUCKET"
bucket_name = "better-auth-storage"

# KV Namespace
[[kv_namespaces]]
binding = "CACHE"
id = "yyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyy"

# WASM Module
[[wasm_modules]]
BETTER_AUTH_WASM = "dist/better_auth_wasm_bg.wasm"

# Staging environment
[env.staging]
name = "better-auth-worker-staging"

[env.staging.vars]
ENVIRONMENT = "staging"
BETTER_AUTH_URL = "https://auth-staging.yourdomain.com"

[[env.staging.d2_databases]]
binding = "DB"
database_name = "better-auth-db-staging"
database_id = "staging-database-id"
```

## Database Setup

### Run Migrations

```bash
# Create initial schema
wrangler d2 execute better-auth-db --file=migrations/001_initial.sql

# Or run programmatically in worker startup
```

### Migration File (migrations/001_initial.sql)

```sql
-- Core tables
CREATE TABLE IF NOT EXISTS users (
    id TEXT PRIMARY KEY,
    email TEXT UNIQUE NOT NULL,
    username TEXT UNIQUE,
    password_hash TEXT,
    email_verified INTEGER DEFAULT 0,
    email_verified_at INTEGER,
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL,
    metadata TEXT
);

CREATE TABLE IF NOT EXISTS sessions (
    id TEXT PRIMARY KEY,
    user_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    token TEXT UNIQUE NOT NULL,
    expires_at INTEGER NOT NULL,
    created_at INTEGER NOT NULL,
    ip_address TEXT,
    user_agent TEXT,
    metadata TEXT
);

CREATE TABLE IF NOT EXISTS accounts (
    id TEXT PRIMARY KEY,
    user_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    provider_id TEXT NOT NULL,
    provider_account_id TEXT NOT NULL,
    access_token TEXT,
    refresh_token TEXT,
    expires_at INTEGER,
    scope TEXT,
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL,
    UNIQUE(provider_id, provider_account_id)
);

CREATE TABLE IF NOT EXISTS verification_tokens (
    id TEXT PRIMARY KEY,
    identifier TEXT NOT NULL,
    token TEXT UNIQUE NOT NULL,
    type TEXT NOT NULL,
    expires_at INTEGER NOT NULL,
    created_at INTEGER NOT NULL,
    consumed_at INTEGER
);

-- Indexes
CREATE INDEX IF NOT EXISTS idx_sessions_user_id ON sessions(user_id);
CREATE INDEX IF NOT EXISTS idx_sessions_token ON sessions(token);
CREATE INDEX IF NOT EXISTS idx_sessions_expires ON sessions(expires_at);
CREATE INDEX IF NOT EXISTS idx_accounts_user ON accounts(user_id);
CREATE INDEX IF NOT EXISTS idx_accounts_provider ON accounts(provider_id, provider_account_id);
CREATE INDEX IF NOT EXISTS idx_verification_tokens_token ON verification_tokens(token);

-- Migrations tracking
CREATE TABLE IF NOT EXISTS _migrations (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    applied_at INTEGER NOT NULL DEFAULT (strftime('%s', 'now') * 1000)
);

INSERT INTO _migrations (id, name) VALUES ('001', 'initial_schema');
```

## Build and Deploy

### Build WASM Module

```bash
# Install wasm-pack if not installed
curl https://rustwasm.github.io/wasm-pack/installer/init.sh -sSf | sh

# Build WASM for web target
cd crates/better-auth-bindings
wasm-pack build --target web --out-dir ../../dist

# Copy WASM to worker dist
cp dist/better_auth_wasm_bg.wasm ../examples/cloudflare-worker/dist/
```

### Deploy Worker

```bash
cd examples/cloudflare-worker

# Install dependencies
npm install

# Deploy to production
wrangler deploy

# Deploy to staging
wrangler deploy --env staging
```

### Local Development

```bash
# Run locally with emulated services
wrangler dev --local

# Run with remote services (uses real D2/R2/KV)
wrangler dev
```

## Worker Implementation

### src/worker.ts

```typescript
import init, { AuthEngine } from '../dist/better_auth_wasm';
import type { Env } from './types';

export default {
  async fetch(
    request: Request,
    env: Env,
    ctx: ExecutionContext
  ): Promise<Response> {
    const url = new URL(request.url);
    const path = url.pathname;

    // Initialize WASM (singleton pattern recommended)
    await init(env.BETTER_AUTH_WASM);
    const auth = new AuthEngine(
      env.BETTER_AUTH_SECRET,
      env.DB as unknown as JsValue,
      env.CACHE as unknown as JsValue,
      env.BUCKET as unknown as JsValue
    );

    // Route handling
    const routes: Record<string, RouteHandler> = {
      'POST /auth/sign-up': handleSignUp,
      'POST /auth/sign-in': handleSignIn,
      'POST /auth/sign-out': handleSignOut,
      'GET /auth/session': handleGetSession,
      'POST /auth/refresh': handleRefresh,
      'POST /auth/magic-link/request': handleMagicLinkRequest,
      'GET /auth/magic-link/verify': handleMagicLinkVerify,
      'GET /auth/oauth/:provider': handleOAuthStart,
      'GET /auth/oauth/:provider/callback': handleOAuthCallback,
      'GET /health': handleHealth,
    };

    const routeKey = `${request.method} ${path}`;
    const handler = routes[routeKey] || routes['GET /health'];

    try {
      return await handler(request, auth, env);
    } catch (error) {
      console.error('Handler error:', error);
      return json(
        { error: error.message || 'Internal server error' },
        { status: 500 }
      );
    }
  },
};

type RouteHandler = (
  request: Request,
  auth: AuthEngine,
  env: Env
) => Promise<Response>;

const handleSignUp: RouteHandler = async (request, auth, env) => {
  const { email, password } = await request.json();
  const user = await auth.signUp(email, password);
  return json({ user });
};

const handleSignIn: RouteHandler = async (request, auth, env) => {
  const { email, password, rememberMe } = await request.json();
  const response = await auth.signIn(email, password);

  const headers = new Headers({ 'Content-Type': 'application/json' });
  headers.append(
    'Set-Cookie',
    `session=${response.token}; Path=/; HttpOnly; Secure; SameSite=Lax; Max-Age=604800`
  );

  return new Response(JSON.stringify(response), { headers });
};

const handleSignOut: RouteHandler = async (request, auth, env) => {
  const cookie = request.headers.get('cookie') || '';
  const sessionToken = getSessionToken(cookie);

  if (sessionToken) {
    await auth.revokeSession(sessionToken);
  }

  return json(
    { success: true },
    {
      headers: {
        'Set-Cookie': 'session=; Path=/; Expires=Thu, 01 Jan 1970 00:00:00 GMT',
      },
    }
  );
};

const handleGetSession: RouteHandler = async (request, auth, env) => {
  const cookie = request.headers.get('cookie') || '';
  const sessionToken = getSessionToken(cookie);

  if (!sessionToken) {
    return json({ error: 'No session' }, { status: 401 });
  }

  const session = await auth.verifySession(sessionToken);
  return json({ session });
};

const handleRefresh: RouteHandler = async (request, auth, env) => {
  const cookie = request.headers.get('cookie') || '';
  const sessionToken = getSessionToken(cookie);

  if (!sessionToken) {
    return json({ error: 'No session' }, { status: 401 });
  }

  const newToken = await auth.refreshSession(sessionToken);

  return json(
    { token: newToken },
    {
      headers: {
        'Set-Cookie': `session=${newToken}; Path=/; HttpOnly; Secure; SameSite=Lax; Max-Age=604800`,
      },
    }
  );
};

const handleMagicLinkRequest: RouteHandler = async (request, auth, env) => {
  const { email } = await request.json();
  const result = await auth.requestMagicLink(email, env.BETTER_AUTH_URL);

  // TODO: Send email with magic link
  console.log(`Magic link for ${email}: ${result.magicLink}`);

  return json({ success: true });
};

const handleMagicLinkVerify: RouteHandler = async (request, auth, env) => {
  const params = url.searchParams;
  const token = params.get('token');
  const email = params.get('email');

  if (!token || !email) {
    return redirect('/?error=invalid_magic_link');
  }

  const user = await auth.verifyMagicLink(token, email);

  return redirect('/?success=1', {
    headers: {
      'Set-Cookie': `session=${user.token}; Path=/; HttpOnly; Secure; SameSite=Lax; Max-Age=604800`,
    },
  });
};

const handleOAuthStart: RouteHandler = async (request, auth, env) => {
  const url = new URL(request.url);
  const provider = url.pathname.split('/').pop();

  if (!provider) {
    return json({ error: 'Provider required' }, { status: 400 });
  }

  const authUrl = await auth.createOAuthUrl(provider, env.BETTER_AUTH_URL);
  return redirect(authUrl);
};

const handleOAuthCallback: RouteHandler = async (request, auth, env) => {
  const url = new URL(request.url);
  const parts = url.pathname.split('/');
  const provider = parts[parts.length - 2];
  const code = url.searchParams.get('code');
  const state = url.searchParams.get('state');

  if (!code || !provider) {
    return redirect('/?error=oauth_failed');
  }

  const user = await auth.handleOAuthCallback(provider, code, state);

  return redirect('/?success=1', {
    headers: {
      'Set-Cookie': `session=${user.token}; Path=/; HttpOnly; Secure; SameSite=Lax; Max-Age=604800`,
    },
  });
};

const handleHealth: RouteHandler = async (request, auth, env) => {
  // Check database connectivity
  try {
    await env.DB.prepare('SELECT 1').first();
  } catch (error) {
    return json({ status: 'error', error: 'Database connection failed' }, { status: 500 });
  }

  return json({
    status: 'ok',
    timestamp: Date.now(),
    environment: env.ENVIRONMENT,
  });
};

// Utilities
function json(data: unknown, init?: ResponseInit): Response {
  return new Response(JSON.stringify(data), {
    ...init,
    headers: { 'Content-Type': 'application/json', ...init?.headers },
  });
}

function redirect(url: string, init?: ResponseInit): Response {
  return new Response(null, {
    ...init,
    status: 302,
    headers: { 'Location': url, ...init?.headers },
  });
}

function getSessionToken(cookie: string): string | null {
  const match = cookie.match(/session=([^;]+)/);
  return match?.[1] || null;
}
```

## Secrets Management

```bash
# Set production secrets
wrangler secret put BETTER_AUTH_SECRET
wrangler secret put HMAC_SECRET

# Set staging secrets
wrangler secret put BETTER_AUTH_SECRET --env staging
wrangler secret put HMAC_SECRET --env staging
```

## Monitoring

### Configure Logging

```typescript
// Add to worker.ts
export interface Env {
  LOG_LEVEL: string;  // 'debug' | 'info' | 'warn' | 'error'
}

function log(level: string, message: string, data?: Record<string, unknown>) {
  const logLevel = env.LOG_LEVEL || 'info';
  const levels = ['debug', 'info', 'warn', 'error'];

  if (levels.indexOf(level) >= levels.indexOf(logLevel)) {
    console[level](`[${level.toUpperCase()}] ${message}`, data);
  }
}
```

### Error Tracking

```typescript
// Add error reporting
export default {
  async fetch(request, env, ctx) {
    try {
      return await handler(request, env, ctx);
    } catch (error) {
      // Log to CloudFlare Analytics
      console.error('Unhandled error:', error);

      // Send to external error tracking (optional)
      ctx.waitUntil(
        fetch('https://sentry.io/api/...', {
          method: 'POST',
          body: JSON.stringify({
            message: error.message,
            stack: error.stack,
            url: request.url,
          }),
        })
      );

      return json({ error: 'Internal server error' }, { status: 500 });
    }
  },
};
```

## Cost Estimation

### Free Tier Limits

| Resource | Free Tier | Typical Usage (10K users) |
|----------|-----------|---------------------------|
| Worker Requests | 100K/day | ~30K/day (sign-ins, session checks) |
| D2 Read | 5M/month | ~500K/month |
| D2 Write | 100K/month | ~30K/month (new sessions) |
| D2 Storage | 10 GB | ~100 MB |
| R2 Storage | 10 GB | ~1 GB (avatars) |
| KV Read | 100K/day | ~50K/day (session cache) |
| KV Write | 1K/day | ~500/day (session creation) |

### Estimated Monthly Cost

With 10K users and typical usage patterns:
- **Worker requests**: Within free tier
- **D2 operations**: ~$1-2/month
- **R2 storage**: ~$0.02/month
- **KV operations**: ~$1-2/month

**Total: ~$5/month or less**

## Troubleshooting

### Common Issues

1. **"Module not found" for WASM**
   ```bash
   # Ensure WASM file exists and is referenced correctly
   ls -la dist/better_auth_wasm_bg.wasm

   # Check wrangler.toml wasm_modules section
   ```

2. **"D2 database not found"**
   ```bash
   # Verify database ID in wrangler.toml
   wrangler d2 list
   ```

3. **"WASM memory limit exceeded"**
   ```toml
   # Increase memory limit in wrangler.toml
   [vars]
   WASM_MEMORY_LIMIT = 256
   ```

4. **"Cold start too slow"**
   - Use `wasm-opt` to optimize WASM binary
   - Enable Wrangler's minification
   - Consider lazy-loading non-critical code

## CI/CD with GitHub Actions

```yaml
# .github/workflows/deploy.yml
name: Deploy Worker

on:
  push:
    branches: [main]

jobs:
  deploy:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4

      - name: Setup Node.js
        uses: actions/setup-node@v4
        with:
          node-version: '20'

      - name: Setup Rust
        uses: actions-rs/toolchain@v1
        with:
          toolchain: stable
          target: wasm32-unknown-unknown

      - name: Install wasm-pack
        run: curl https://rustwasm.github.io/wasm-pack/installer/init.sh -sSf | sh

      - name: Build WASM
        run: |
          cd crates/better-auth-bindings
          wasm-pack build --target web --out-dir ../../dist

      - name: Install dependencies
        run: cd examples/cloudflare-worker && npm ci

      - name: Deploy to CloudFlare
        run: cd examples/cloudflare-worker && npx wrangler deploy
        env:
          CLOUDFLARE_API_TOKEN: ${{ secrets.CLOUDFLARE_API_TOKEN }}
```
