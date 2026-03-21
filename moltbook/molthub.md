# MoltHub - Skill Registry Deep Dive

## Overview

MoltHub is the public skill registry for the Moltbot/Clawdbot ecosystem. It serves as the central repository for publishing, versioning, and discovering text-based agent skills (`SKILL.md`) and system personas (`SOUL.md`).

**Live Sites:**
- MoltHub: `https://molthub.com`
- onlycrabs.ai (SOUL.md registry): `https://onlycrabs.ai`

## Core Capabilities

- **Browse skills** with rendered `SKILL.md` documentation
- **Publish skill versions** with changelogs and tags (including `latest`)
- **Browse souls** with rendered `SOUL.md` documentation
- **Vector search** via OpenAI embeddings (`text-embedding-3-small`)
- **Star and comment** on skills/souls
- **Moderation system** with admin approval and badges
- **Install telemetry** tracking (opt-out via `MOLTHUB_DISABLE_TELEMETRY=1`)

## Technology Stack

| Layer | Technology |
|-------|------------|
| **Frontend** | TanStack Start (React, Vite/Nitro) |
| **Backend** | Convex (DB + File Storage + HTTP Actions) |
| **Auth** | Convex Auth with GitHub OAuth |
| **Search** | Convex Vector Search + OpenAI Embeddings |
| **Package Manager** | Bun |
| **Linting** | Biome + Oxlint (type-aware) |
| **Testing** | Vitest 4 + jsdom |
| **Schema** | `packages/schema` (molthub-schema) |

## Repository Structure

```
molthub/
├── src/                          # TanStack Start application
│   ├── components/               # React components
│   ├── pages/                    # Route handlers
│   ├── layouts/                  # Page layouts
│   └── data/                     # Static data (showcase, testimonials)
├── convex/                       # Backend (Convex)
│   ├── schema.ts                 # Database schema
│   ├── auth.ts                   # Authentication configuration
│   ├── skills.ts                 # Skill operations
│   ├── souls.ts                  # Soul operations
│   ├── search.ts                 # Vector search
│   ├── http.ts                   # HTTP API routes
│   ├── lib/                      # Shared utilities
│   │   ├── embeddings.ts         # OpenAI embedding generation
│   │   ├── githubBackup.ts       # GitHub sync utilities
│   │   ├── moderation.ts         # Content moderation
│   │   ├── skillPublish.ts       # Skill publication logic
│   │   └── tokens.ts             # API token management
│   └── _generated/               # Generated Convex API types
├── packages/schema/              # Shared API types
├── docs/                         # Documentation
│   ├── spec.md                   # Product specification
│   └── telemetry.md              # Telemetry documentation
└── public/                       # Static assets
```

## Database Schema

### Core Tables

#### Users
- GitHub OAuth authentication
- Handles, display names, bios
- Role-based access (`admin`, `moderator`, `user`)

#### Skills
- Unique slug identifiers
- Owner references
- Version tracking (`latestVersionId`, `tags` map)
- Badge system (official, highlighted, deprecated, redactionApproved)
- Moderation status and flags
- Statistics (downloads, stars, installs)

#### SkillVersions
- Semver versioning
- File storage with SHA256 checksums
- Changelog tracking
- Parsed frontmatter metadata

#### Souls
- Similar structure to Skills
- SOUL.md only bundles
- Separate namespace (onlycrabs.ai host-based routing)

#### Vector Embeddings
- Convex vector index for similarity search
- 1536 dimensions (OpenAI `text-embedding-3-small`)
- Visibility filtering

### Schema Highlights

```typescript
const skills = defineTable({
  slug: v.string(),
  displayName: v.string(),
  ownerUserId: v.id('users'),
  latestVersionId: v.optional(v.id('skillVersions')),
  tags: v.record(v.string(), v.id('skillVersions')),
  badges: v.object({
    redactionApproved: v.optional(/* ... */),
    highlighted: v.optional(/* ... */),
    official: v.optional(/* ... */),
    deprecated: v.optional(/* ... */),
  }),
  moderationStatus: v.optional(v.union(
    v.literal('active'),
    v.literal('hidden'),
    v.literal('removed')
  )),
  stats: v.object({
    downloads: v.number(),
    installsCurrent: v.optional(v.number()),
    installsAllTime: v.optional(v.number()),
    stars: v.number(),
    versions: v.number(),
    comments: v.number(),
  }),
})
  .index('by_slug', ['slug'])
  .index('by_owner', ['ownerUserId'])
  .index('by_updated', ['updatedAt'])
  .index('by_stats_installs_current', ['statsInstallsCurrent', 'updatedAt'])
```

## API Endpoints

### HTTP Routes (Convex)

| Endpoint | Method | Description |
|----------|--------|-------------|
| `/api/skills` | GET | List all skills with filters |
| `/api/skills/:slug` | GET | Get skill by slug |
| `/api/skills/:slug/:version` | GET | Get specific version |
| `/api/skills` | POST | Publish new skill |
| `/api/skills/:id` | DELETE | Delete skill (admin) |
| `/api/souls` | GET | List all souls |
| `/api/souls/:slug` | GET | Get soul by slug |
| `/api/souls` | POST | Publish new soul |
| `/api/search` | GET | Vector search |
| `/api/download/:skillId/:version` | GET | Download ZIP |
| `/api/whoami` | GET | Current user info |
| `/api/stars/:skillId` | POST/DELETE | Star/unstar |

### CLI API

The schema package (`molthub-schema`) provides shared types for the CLI:

```typescript
// packages/schema/src/routes.ts
export const ApiRoutes = {
  skills: '/api/skills',
  souls: '/api/souls',
  search: '/api/search',
  download: '/api/download',
  whoami: '/api/whoami',
  stars: '/api/stars',
} as const;
```

## Nix Plugin Support

MoltHub supports Nix-based skill distribution through frontmatter metadata:

```yaml
---
name: peekaboo
description: Capture and automate macOS UI with the Peekaboo CLI.
metadata:
  moltbot:
    nix:
      plugin: "github:moltbot/nix-steipete-tools?dir=tools/peekaboo"
      systems: ["aarch64-darwin"]
---
```

Install via nix-moltbot:
```nix
programs.moltbot.plugins = [
  { source = "github:moltbot/nix-steipete-tools?dir=tools/peekaboo"; }
];
```

## Authentication Flow

1. User clicks "Sign in with GitHub"
2. Convex Auth handles OAuth redirect
3. JWT tokens stored in httpOnly cookies
4. User profile created/updated in `users` table
5. Admin role bootstrap for first user (`steipete`)

```typescript
// convex/auth.config.ts
export default {
  providers: [
    {
      domain: process.env.CONVEX_SITE_URL,
      applicationID: "convex",
    },
  ],
};
```

## Vector Search Implementation

### Embedding Generation

```typescript
// convex/lib/embeddings.ts
export const EMBEDDING_DIMENSIONS = 1536; // text-embedding-3-small

export async function generateEmbedding(text: string) {
  const response = await fetch("https://api.openai.com/v1/embeddings", {
    method: "POST",
    headers: {
      "Authorization": `Bearer ${process.env.OPENAI_API_KEY}`,
      "Content-Type": "application/json",
    },
    body: JSON.stringify({
      model: "text-embedding-3-small",
      input: text,
    }),
  });
  const data = await response.json();
  return data.data[0].embedding;
}
```

### Vector Index Configuration

```typescript
const skillEmbeddings = defineTable({
  skillId: v.id('skills'),
  versionId: v.id('skillVersions'),
  ownerId: v.id('users'),
  embedding: v.array(v.number()),
  isLatest: v.boolean(),
  isApproved: v.boolean(),
  visibility: v.string(),
  updatedAt: v.number(),
})
  .index('by_skill', ['skillId'])
  .index('by_version', ['versionId'])
  .vectorIndex('by_embedding', {
    vectorField: 'embedding',
    dimensions: EMBEDDING_DIMENSIONS,
    filterFields: ['visibility'],
  });
```

## Moderation System

### Badge Types

| Badge | Purpose | Required Role |
|-------|---------|---------------|
| `official` | Admin-verified official skills | Admin |
| `highlighted` | Featured skills | Moderator |
| `redactionApproved` | Privacy redactions reviewed | Moderator |
| `deprecated` | Not recommended for new use | Admin |

### Moderation Actions

- Soft delete comments (preserves audit trail)
- Hide/remove skills from public view
- Flag automatic detection (spam patterns)
- Audit logging for all actions

## Upload Flow (50MB limit)

1. Client requests upload session
2. Convex generates signed upload URLs
3. Client uploads files directly to Convex storage
4. Client submits metadata with file storage IDs
5. Server validates:
   - Total size ≤ 50MB
   - SKILL.md/SOUL.md exists
   - Frontmatter parseable
   - Semver version unique
6. Files stored, metadata indexed, embeddings generated

## Development Setup

```bash
# Prerequisites: Bun, Convex CLI
bun install
cp .env.local.example .env.local

# Terminal A: Web app
bun run dev

# Terminal B: Convex backend
bunx convex dev
```

### Required Environment Variables

```bash
VITE_CONVEX_URL=https://<deployment>.convex.cloud
VITE_CONVEX_SITE_URL=https://<deployment>.convex.site
VITE_SOULHUB_SITE_URL=https://onlycrabs.ai
VITE_SOULHUB_HOST=onlycrabs.ai
AUTH_GITHUB_ID=<github-oauth-id>
AUTH_GITHUB_SECRET=<github-oauth-secret>
JWT_PRIVATE_KEY=<convex-auth-key>
JWKS=<jwks-json>
OPENAI_API_KEY=<openai-key>
```

## Testing

```bash
# Run tests
bun run test

# Coverage (80% threshold required)
bun run coverage

# Type-aware linting
bun run lint:oxlint

# E2E tests
bun run test:e2e
bun run test:pw
```

## CI/CD

GitHub Actions workflow (`.github/workflows/ci.yml`):
- TypeScript type checking
- Biome linting
- Oxlint type-aware checks
- Vitest test suite
- Convex deployment on main merge

## Performance Optimizations

1. **Convex Query Caching** - Automatic subscription caching
2. **Vector Search** - Cosine similarity on Convex vector index
3. **Pagination** - Cursor-based pagination for large result sets
4. **Optimistic UI** - TanStack Start mutations with rollback
5. **Prefetching** - Route-based data prefetching

## GitHub Integration (Phase 2)

Planned GitHub App sync:
- Backup skills to `moltbot/skills` repository
- Sync stars/issues with GitHub
- CI/CD integration for skill validation

## Telemetry

Install telemetry is tracked when running `molthub sync` while logged in:

- User ID (anonymized)
- Skill slug + version
- Timestamp
- Root identifier (for multi-root workspaces)

Disable with:
```bash
export MOLTHUB_DISABLE_TELEMETRY=1
```

## Related Projects

- **Moltbot** - Core agent that consumes skills
- **Lobster** - Workflow engine for deterministic pipelines
- **ClawdHub** - Public skill registry (sister project)
- **nix-moltbot** - Nix module for skill installation

---

*MoltHub deep dive - Part of Moltbook ecosystem exploration*
