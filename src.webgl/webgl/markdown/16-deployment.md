# Tiny Skies -- Deployment

Tiny Skies is deployed across two platforms: the client as a Vercel SPA and the server as a Railway container with PostgreSQL.

Source: `tinyskies/vercel.json` — Vercel configuration
Source: `tinyskies/railway.toml` — Railway configuration
Source: `tinyskies/server/Dockerfile` — server container
Source: `tinyskies/docker-compose.yml` — local development
Source: `tinyskies/api/server-url.js` — Vercel serverless function
Source: `tinyskies/.github/workflows/vercel-deploy.yml` — CI workflow

## Client Deployment (Vercel)

### vercel.json

```json
{
  "buildCommand": "npm run build",
  "outputDirectory": "client/dist",
  "rewrites": [
    { "source": "/(.*)", "destination": "/index.html" }
  ]
}
```

The **SPA rewrite** routes all paths to `index.html`, allowing the client to handle routing internally. This is necessary for Vite's HTML5 history API fallback — without it, refreshing on a deep link would return a 404.

### Vercel Serverless Function

```javascript
// api/server-url.js
// Returns the backend server URL from environment variables
// Allows client to discover Railway URL dynamically

export default function handler(req, res) {
  const serverUrl = process.env.SERVER_URL || "http://localhost:3001";
  res.status(200).json({ url: serverUrl });
}
```

The client fetches `/api/server-url` at startup to discover the WebSocket server URL. This allows the same client build to connect to different server environments (production, staging, local) based on Vercel environment variables.

### Environment Variables

| Variable | Purpose | Example |
|----------|---------|---------|
| `SERVER_URL` | Railway server URL | `https://tinyskies-production.up.railway.app` |
| `VERCEL_URL` | Dynamic preview URL | `tinyskies-abc123.vercel.app` |

## Server Deployment (Railway)

### railway.toml

```toml
[build]
dockerfilePath = "server/Dockerfile"
```

Railway uses the Dockerfile to build and deploy the server container.

### Dockerfile

```dockerfile
FROM node:20-alpine

WORKDIR /app

# Copy package files
COPY package*.json ./
COPY server/package*.json ./server/
COPY shared/package*.json ./shared/

# Install dependencies
RUN npm install

# Copy source code
COPY server/ ./server/
COPY shared/ ./shared/

# Generate Prisma client
RUN cd server && npx prisma generate

# Build TypeScript
RUN npm run build

# Run migrations and start server
CMD cd server && npx prisma migrate deploy && node dist/index.js
```

The Dockerfile:
1. Installs dependencies for all workspaces (shared, server)
2. Generates Prisma client (creates TypeScript types from schema)
3. Builds TypeScript to JavaScript
4. Runs database migrations on startup (`prisma migrate deploy`)
5. Starts the Express server

### PostgreSQL Database

Railway provides a managed PostgreSQL instance. The `DATABASE_URL` environment variable is automatically set by Railway.

```
DATABASE_URL=postgresql://user:password@postgres.railway.internal:5432/db
```

## Local Development

### docker-compose.yml

```yaml
version: "3.8"
services:
  postgres:
    image: postgres:16-alpine
    environment:
      POSTGRES_USER: tinyskies
      POSTGRES_PASSWORD: password
      POSTGRES_DB: tinyskies_dev
    ports:
      - "5432:5432"
    volumes:
      - postgres_data:/var/lib/postgresql/data

volumes:
  postgres_data:
```

For local development, Docker Compose runs a PostgreSQL 16 instance. The `.env` file configures `DATABASE_URL=postgresql://tinyskies:password@localhost:5432/tinyskies_dev`.

### npm Workspaces

```json
// root package.json
{
  "workspaces": ["shared", "client", "server"]
}
```

The monorepo uses npm workspaces to manage shared dependencies:
- `shared/` — types and constants used by both client and server
- `client/` — browser game (Vite build)
- `server/` — Express app (tsc build)

### Build Scripts

```json
// root package.json
{
  "scripts": {
    "dev": "npm run dev:client",
    "dev:client": "cd client && vite",
    "dev:server": "cd server && nodemon src/index.ts",
    "build": "cd client && vite build",
    "build:server": "cd server && tsc",
    "postbuild": "node patch.js && node patch2.js && node patch3.js"
  }
}
```

The `postbuild` script runs the Three.js patches after the Vite build completes.

### CI Workflow

```yaml
# .github/workflows/vercel-deploy.yml
name: Vercel Deploy
on:
  push:
    branches: [main]

jobs:
  deploy:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      - uses: actions/setup-node@v3
        with:
          node-version: "20"
      - run: npm install
      - run: npm run build
      - uses: amondnet/vercel-action@v20
        with:
          vercel-token: ${{ secrets.VERCEL_TOKEN }}
          vercel-org-id: ${{ secrets.ORG_ID }}
          vercel-project-id: ${{ secrets.PROJECT_ID }}
```

The workflow builds the client and deploys to Vercel on every push to `main`.

## .dockerignore

```
client/
node_modules/
dist/
.git/
.env
```

Excludes client code, dependencies, build output, git history, and secrets from the server Docker image to reduce image size and prevent leaks.

## .railwayignore

```
client/
node_modules/
.git/
```

Same as `.dockerignore` but for Railway's native build (non-Docker mode).

See [Server Architecture](12-server-architecture.md) for the Express/Socket.IO setup.
See [Database Schema](13-database-schema.md) for Prisma configuration.
