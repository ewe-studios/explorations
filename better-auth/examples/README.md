# Better Auth WASM Examples

This directory contains working examples of the Better Auth WASM implementation.

## Examples

### 1. CloudFlare Worker Example

A complete CloudFlare Worker deployment with D2, R2, and KV bindings.

**Location:** `examples/cloudflare-worker/`

### 2. Node.js Example

Running the WASM module in Node.js with local SQLite.

**Location:** `examples/nodejs/`

### 3. Standalone WASM Example

Minimal example showing direct WASM module usage.

**Location:** `examples/standalone/`

---

## Example 1: CloudFlare Worker

### File Structure

```
examples/cloudflare-worker/
├── wrangler.toml
├── package.json
├── src/
│   ├── worker.ts
│   └── types.ts
├── migrations/
│   └── 001_initial.sql
└── dist/
    └── better_auth_wasm.{js,wasm}
```

### wrangler.toml

```toml
name = "better-auth-demo"
main = "src/worker.ts"
compatibility_date = "2024-01-01"
compatibility_flags = ["nodejs_compat"]

[vars]
ENVIRONMENT = "development"
BETTER_AUTH_URL = "http://localhost:8787"

[[d2_databases]]
binding = "DB"
database_name = "better-auth-db"
database_id = "your-d2-database-id"

[[r2_buckets]]
binding = "BUCKET"
bucket_name = "better-auth-storage"

[[kv_namespaces]]
binding = "CACHE"
id = "your-kv-namespace-id"

[[wasm_modules]]
BETTER_AUTH_WASM = "dist/better_auth_wasm_bg.wasm"
```

### src/worker.ts

```typescript
import { AuthEngine } from '../dist/better_auth_wasm';
import type { Env, AuthResponse } from './types';

export default {
  async fetch(
    request: Request,
    env: Env,
    ctx: ExecutionContext
  ): Promise<Response> {
    const url = new URL(request.url);
    const path = url.pathname;

    // Initialize auth engine (singleton pattern recommended)
    const auth = new AuthEngine(
      env.BETTER_AUTH_SECRET,
      env.DB as unknown as JsValue,
      env.CACHE as unknown as JsValue,
      env.BUCKET as unknown as JsValue
    );

    // CORS preflight
    if (request.method === 'OPTIONS') {
      return new Response(null, {
        headers: corsHeaders(),
      });
    }

    // Routes
    try {
      if (path === '/auth/sign-up' && request.method === 'POST') {
        const { email, password } = await request.json();
        const user = await auth.signUp(email, password);
        return json({ user }, { headers: corsHeaders() });
      }

      if (path === '/auth/sign-in' && request.method === 'POST') {
        const { email, password } = await request.json();
        const response: AuthResponse = await auth.signIn(email, password);

        // Set session cookie
        const headers = new Headers({
          'Content-Type': 'application/json',
          'Set-Cookie': `session=${response.token}; Path=/; HttpOnly; Secure; SameSite=Lax; Max-Age=604800`,
          ...corsHeaders(),
        });

        return new Response(JSON.stringify(response), { headers });
      }

      if (path === '/auth/sign-out' && request.method === 'POST') {
        const cookie = request.headers.get('cookie') || '';
        const sessionToken = getSessionToken(cookie);

        if (sessionToken) {
          await auth.revokeSession(sessionToken);
        }

        return json({ success: true }, {
          headers: {
            'Set-Cookie': 'session=; Path=/; Expires=Thu, 01 Jan 1970 00:00:00 GMT',
            ...corsHeaders(),
          },
        });
      }

      if (path === '/auth/session' && request.method === 'GET') {
        const cookie = request.headers.get('cookie') || '';
        const sessionToken = getSessionToken(cookie);

        if (!sessionToken) {
          return json({ error: 'No session' }, { status: 401, headers: corsHeaders() });
        }

        try {
          const session = await auth.verifySession(sessionToken);
          return json({ session }, { headers: corsHeaders() });
        } catch {
          return json({ error: 'Invalid session' }, { status: 401, headers: corsHeaders() });
        }
      }

      if (path === '/auth/magic-link/request' && request.method === 'POST') {
        const { email } = await request.json();
        const result = await auth.requestMagicLink(email, env.BETTER_AUTH_URL);

        // In production, send this email via your preferred provider
        console.log(`Magic link for ${email}: ${result.magicLink}`);

        return json({ success: true }, { headers: corsHeaders() });
      }

      if (path === '/auth/magic-link/verify' && request.method === 'GET') {
        const params = url.searchParams;
        const token = params.get('token');
        const email = params.get('email');

        if (!token || !email) {
          return redirect('/?error=invalid_magic_link');
        }

        const user = await auth.verifyMagicLink(token, email);
        const sessionToken = user.token;

        // Redirect to app with session cookie
        return redirect('/?success=magic_link_verified', {
          headers: {
            'Set-Cookie': `session=${sessionToken}; Path=/; HttpOnly; Secure; SameSite=Lax; Max-Age=604800`,
          },
        });
      }

      // Health check
      if (path === '/health') {
        return json({
          status: 'ok',
          timestamp: Date.now(),
          environment: env.ENVIRONMENT,
        }, { headers: corsHeaders() });
      }

      // Not found
      return json({ error: 'Not found' }, { status: 404, headers: corsHeaders() });

    } catch (error) {
      console.error('Auth error:', error);
      return json(
        { error: error instanceof Error ? error.message : 'Unknown error' },
        { status: 500, headers: corsHeaders() }
      );
    }
  },
};

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

function corsHeaders(): Record<string, string> {
  return {
    'Access-Control-Allow-Origin': '*',
    'Access-Control-Allow-Methods': 'GET, POST, PUT, DELETE, OPTIONS',
    'Access-Control-Allow-Headers': 'Content-Type, Authorization',
  };
}

function getSessionToken(cookie: string): string | null {
  const match = cookie.match(/session=([^;]+)/);
  return match?.[1] || null;
}
```

### src/types.ts

```typescript
export interface Env {
  DB: D1Database;
  BUCKET: R2Bucket;
  CACHE: KVNamespace;
  BETTER_AUTH_SECRET: string;
  BETTER_AUTH_URL: string;
  ENVIRONMENT: string;
}

export interface User {
  id: string;
  email: string;
  username?: string;
  emailVerified: boolean;
  createdAt: string;
  metadata?: Record<string, unknown>;
}

export interface Session {
  id: string;
  userId: string;
  expiresAt: string;
  createdAt: string;
  ipAddress?: string;
  userAgent?: string;
}

export interface AuthResponse {
  user: User;
  session: Session;
  token: string;
}

export interface MagicLinkResult {
  email: string;
  magicLink: string;
  expiresAt: string;
}
```

### migrations/001_initial.sql

```sql
-- Users table
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

-- Sessions table
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

-- Verification tokens
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
CREATE INDEX IF NOT EXISTS idx_verification_tokens_token ON verification_tokens(token);
```

---

## Example 2: Node.js with SQLite

### File Structure

```
examples/nodejs/
├── package.json
├── tsconfig.json
├── src/
│   ├── index.ts
│   ├── server.ts
│   └── database.ts
└── dist/
    └── better_auth_wasm.{js,wasm}
```

### package.json

```json
{
  "name": "better-auth-nodejs-example",
  "version": "1.0.0",
  "type": "module",
  "scripts": {
    "dev": "tsx src/server.ts",
    "build": "tsc && cp ../dist/better_auth_wasm_bg.wasm dist/"
  },
  "dependencies": {
    "better-auth-wasm": "file:../dist",
    "express": "^4.18.0",
    "better-sqlite3": "^9.0.0",
    "cookie-parser": "^1.4.6"
  },
  "devDependencies": {
    "@types/express": "^4.17.0",
    "@types/cookie-parser": "^1.4.0",
    "tsx": "^4.0.0",
    "typescript": "^5.0.0"
  }
}
```

### src/database.ts

```typescript
import Database from 'better-sqlite3';

export function createDatabase(path: string = ':memory:'): Database.Database {
  const db = new Database(path);

  // Enable WAL mode for better concurrency
  db.pragma('journal_mode = WAL');

  // Create tables
  db.exec(`
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

    CREATE TABLE IF NOT EXISTS verification_tokens (
      id TEXT PRIMARY KEY,
      identifier TEXT NOT NULL,
      token TEXT UNIQUE NOT NULL,
      type TEXT NOT NULL,
      expires_at INTEGER NOT NULL,
      created_at INTEGER NOT NULL,
      consumed_at INTEGER
    );

    CREATE INDEX IF NOT EXISTS idx_sessions_user_id ON sessions(user_id);
    CREATE INDEX IF NOT EXISTS idx_sessions_token ON sessions(token);
  `);

  return db;
}
```

### src/server.ts

```typescript
import express from 'express';
import cookieParser from 'cookie-parser';
import { AuthEngine } from 'better-auth-wasm';
import { createDatabase } from './database.js';
import { readFileSync } from 'fs';
import { fileURLToPath } from 'url';
import { dirname, join } from 'path';

const __dirname = dirname(fileURLToPath(import.meta.url));

const app = express();
const db = createDatabase('./auth.db');

// Load WASM module
const wasmBuffer = readFileSync(join(__dirname, 'better_auth_wasm_bg.wasm'));
const wasmModule = await WebAssembly.instantiate(wasmBuffer);

// Initialize auth engine
const auth = new AuthEngine(
  process.env.BETTER_AUTH_SECRET || 'dev-secret-min-32-characters-long!',
  db as unknown as JsValue,
  {} as JsValue,  // Cache (not used in Node.js example)
  {} as JsValue   // Storage (not used in Node.js example)
);

app.use(express.json());
app.use(cookieParser());

// Sign up
app.post('/auth/sign-up', async (req, res) => {
  const { email, password } = req.body;

  try {
    const user = await auth.signUp(email, password);
    res.json({ user });
  } catch (error) {
    res.status(400).json({ error: error.message });
  }
});

// Sign in
app.post('/auth/sign-in', async (req, res) => {
  const { email, password } = req.body;

  try {
    const response = await auth.signIn(email, password);

    res.cookie('session', response.token, {
      httpOnly: true,
      secure: process.env.NODE_ENV === 'production',
      sameSite: 'lax',
      maxAge: 7 * 24 * 60 * 60 * 1000,  // 7 days
    });

    res.json(response);
  } catch (error) {
    res.status(401).json({ error: error.message });
  }
});

// Sign out
app.post('/auth/sign-out', async (req, res) => {
  const sessionToken = req.cookies.session;

  if (sessionToken) {
    await auth.revokeSession(sessionToken);
  }

  res.clearCookie('session');
  res.json({ success: true });
});

// Get current session
app.get('/auth/session', async (req, res) => {
  const sessionToken = req.cookies.session;

  if (!sessionToken) {
    return res.status(401).json({ error: 'No session' });
  }

  try {
    const session = await auth.verifySession(sessionToken);
    res.json({ session });
  } catch {
    res.status(401).json({ error: 'Invalid session' });
  }
});

// Request magic link
app.post('/auth/magic-link/request', async (req, res) => {
  const { email } = req.body;

  try {
    const result = await auth.requestMagicLink(email, 'http://localhost:3000');
    console.log(`Magic link for ${email}: ${result.magicLink}`);
    res.json({ success: true });
  } catch (error) {
    res.status(400).json({ error: error.message });
  }
});

// Verify magic link
app.get('/auth/magic-link/verify', async (req, res) => {
  const { token, email } = req.query;

  if (!token || !email) {
    return res.status(400).json({ error: 'Invalid magic link' });
  }

  try {
    const user = await auth.verifyMagicLink(token as string, email as string);

    res.cookie('session', user.token, {
      httpOnly: true,
      secure: process.env.NODE_ENV === 'production',
      sameSite: 'lax',
      maxAge: 7 * 24 * 60 * 60 * 1000,
    });

    res.redirect('/?success=1');
  } catch (error) {
    res.redirect('/?error=' + encodeURIComponent(error.message));
  }
});

const PORT = process.env.PORT || 3000;
app.listen(PORT, () => {
  console.log(`Server running on http://localhost:${PORT}`);
});
```

---

## Example 3: Minimal WASM Usage

### index.html

```html
<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="UTF-8">
  <meta name="viewport" content="width=device-width, initial-scale=1.0">
  <title>Better Auth WASM Demo</title>
  <style>
    body { font-family: system-ui; max-width: 400px; margin: 2rem auto; }
    input { width: 100%; padding: 0.5rem; margin: 0.5rem 0; }
    button { width: 100%; padding: 0.75rem; background: #007bff; color: white; border: none; }
    .error { color: red; }
    .success { color: green; }
  </style>
</head>
<body>
  <h1>Better Auth WASM</h1>

  <div id="auth-form">
    <input type="email" id="email" placeholder="Email" required>
    <input type="password" id="password" placeholder="Password" required>
    <button onclick="signIn()">Sign In</button>
    <button onclick="signUp()">Sign Up</button>
  </div>

  <div id="message"></div>

  <script type="module">
    import init, { AuthEngine } from './better_auth_wasm.js';

    // Initialize WASM
    await init();

    // Create mock database (in browser, you'd use IndexedDB)
    const mockDb = {
      prepare: () => ({
        bind: () => ({})
      })
    };

    // Initialize auth engine
    const auth = new AuthEngine(
      'dev-secret-min-32-characters-long!',
      mockDb,
      {},
      {}
    );

    window.signIn = async () => {
      const email = document.getElementById('email').value;
      const password = document.getElementById('password').value;

      try {
        const response = await auth.signIn(email, password);
        showMessage('Signed in! Token: ' + response.token.substring(0, 20) + '...', 'success');
      } catch (error) {
        showMessage('Sign in failed: ' + error.message, 'error');
      }
    };

    window.signUp = async () => {
      const email = document.getElementById('email').value;
      const password = document.getElementById('password').value;

      try {
        const user = await auth.signUp(email, password);
        showMessage('User created: ' + user.email, 'success');
      } catch (error) {
        showMessage('Sign up failed: ' + error.message, 'error');
      }
    };

    function showMessage(text, type) {
      const el = document.getElementById('message');
      el.textContent = text;
      el.className = type;
    }
  </script>
</body>
</html>
```

---

## Running the Examples

### CloudFlare Worker

```bash
cd examples/cloudflare-worker
npm install
npm run dev
```

### Node.js

```bash
cd examples/nodejs
npm install
npm run dev
```

### Standalone

```bash
cd examples/standalone
python -m http.server 8000
# Open http://localhost:8000
```
