# MoltHub Deep Dive - API and Implementation Details

## Convex Backend Architecture

### Schema Design Patterns

The MoltHub schema uses several key patterns for scalability and maintainability:

#### 1. Soft Delete Pattern
```typescript
const skills = defineTable({
  // ... fields
  softDeletedAt: v.optional(v.number()),
  moderationStatus: v.optional(v.union(
    v.literal('active'),
    v.literal('hidden'),
    v.literal('removed'),
  )),
});
```

All deletions are soft deletes, preserving referential integrity and audit trails.

#### 2. Tag-Based Versioning
```typescript
tags: v.record(v.string(), v.id('skillVersions')),
```

Tags are user-defined pointers to versions (e.g., `latest`, `stable`, `v1`).

#### 3. Badge System
```typescript
badges: v.object({
  redactionApproved: v.optional(v.object({
    byUserId: v.id('users'),
    at: v.number(),
  })),
  highlighted: v.optional(v.object({
    byUserId: v.id('users'),
    at: v.number(),
  })),
  official: v.optional(v.object({
    byUserId: v.id('users'),
    at: v.number(),
  })),
  deprecated: v.optional(v.object({
    byUserId: v.id('users'),
    at: v.number(),
  })),
}),
```

Badges track who approved and when, for full auditability.

## Key Mutations

### Publishing Skills

```typescript
// convex/skills.ts
export const publishSkill = mutation({
  args: {
    displayName: v.string(),
    slug: v.string(),
    version: v.string(),
    changelog: v.string(),
    files: v.array(v.object({
      path: v.string(),
      storageId: v.id('_storage'),
      sha256: v.string(),
    })),
    tags: v.optional(v.record(v.string())),
  },
  handler: async (ctx, args) => {
    const identity = await ctx.auth.getUserIdentity();
    if (!identity) throw new Error("Unauthorized");

    const user = await getUserByAuth(ctx, identity);

    // Check slug uniqueness
    const existing = await ctx.db
      .query('skills')
      .withIndex('by_slug', (q) => q.eq('slug', args.slug))
      .unique();

    if (existing && existing.ownerUserId !== user._id) {
      throw new Error("Slug already taken");
    }

    // Parse frontmatter from SKILL.md
    const skillFile = await getFileContent(ctx, args.files[0].storageId);
    const parsed = parseSkillFrontmatter(skillFile.content);

    // Create or update skill
    const skillId = existing ? existing._id : await ctx.db.insert('skills', {
      slug: args.slug,
      displayName: args.displayName,
      ownerUserId: user._id,
      tags: {},
      badges: { redactionApproved: undefined, highlighted: undefined, official: undefined, deprecated: undefined },
      stats: { downloads: 0, stars: 0, versions: 0, comments: 0 },
      createdAt: Date.now(),
      updatedAt: Date.now(),
    });

    // Create version
    const versionId = await ctx.db.insert('skillVersions', {
      skillId,
      version: args.version,
      changelog: args.changelog,
      files: args.files,
      parsed,
      createdBy: user._id,
      createdAt: Date.now(),
    });

    // Update skill's latestVersionId and tags
    await ctx.db.patch(skillId, {
      latestVersionId: versionId,
      tags: { ...existing?.tags, latest: versionId, ...args.tags },
      updatedAt: Date.now(),
    });

    // Generate embeddings for search
    await generateEmbeddings(ctx, skillId, versionId, parsed);

    return { skillId, versionId };
  },
});
```

### Vector Search

```typescript
// convex/search.ts
export const searchSkills = query({
  args: {
    query: v.string(),
    filters: v.optional(v.object({
      tag: v.optional(v.string()),
      ownerUserId: v.optional(v.id('users')),
      minStars: v.optional(v.number()),
      redactionApprovedOnly: v.optional(v.boolean()),
    })),
    limit: v.optional(v.number()),
  },
  handler: async (ctx, args) => {
    const embedding = await generateEmbedding(args.query);

    const results = await ctx.db
      .query('skillEmbeddings')
      .withSearchIndex('by_embedding', (q) => q
        .search('embedding', embedding)
        .filter('visibility', 'public')
      )
      .take(args.limit ?? 20);

    // Enrich with skill metadata
    const enriched = await Promise.all(
      results.map(async (result) => {
        const skill = await ctx.db.get(result.skillId);
        return { skill, score: result.score };
      })
    );

    return enriched;
  },
});
```

## HTTP API Implementation

### API Router Setup

```typescript
// convex/http.ts
import { httpRouter } from 'convex/server';
import { auth } from './auth';

const http = httpRouter();

auth.addHttpRoutes(http);

// Skill routes
http.route({
  path: '/api/skills',
  method: 'GET',
  handler: listSkillsV1Http,
});

http.route({
  path: '/api/skills/:slug',
  method: 'GET',
  handler: getSkillV1Http,
});

http.route({
  path: '/api/skills',
  method: 'POST',
  handler: publishSkillV1Http,
});

// Search
http.route({
  path: '/api/search',
  method: 'GET',
  handler: searchSkillsV1Http,
});

// Download
http.route({
  path: '/api/download/:skillId/:version',
  method: 'GET',
  handler: downloadZip,
});

export default http;
```

### Rate Limiting

```typescript
// convex/rateLimits.ts
export const checkRateLimit = async (
  ctx: MutationCtx | QueryCtx,
  key: string,
  limit: number,
  windowMs: number,
) => {
  const now = Date.now();
  const windowStart = now - windowMs;

  const existing = await ctx.db
    .query('rateLimits')
    .withIndex('by_key_window', (q) =>
      q.eq('key', key).eq('windowStart', windowStart)
    )
    .unique();

  if (existing && existing.count >= limit) {
    throw new Error(`Rate limit exceeded: ${limit} requests per ${windowMs}ms`);
  }

  if (existing) {
    await ctx.db.patch(existing._id, { count: existing.count + 1 });
  } else {
    await ctx.db.insert('rateLimits', {
      key,
      windowStart,
      count: 1,
      limit,
      updatedAt: now,
    });
  }
};
```

## Frontmatter Parsing

### SKILL.md Structure

```yaml
---
name: my-skill
description: A useful skill
homepage: https://github.com/user/my-skill
metadata:
  moltbot:
    always: true
    skillKey: "my-skill"
    primaryEnv: "MY_API_KEY"
    requires:
      bins: ["node", "git"]
      env: ["MY_API_KEY"]
    install:
      - "npm install -g my-skill"
    nix:
      plugin: "github:user/nix-my-skill"
      systems: ["aarch64-darwin", "x86_64-linux"]
    config:
      requiredEnv: ["MY_API_KEY"]
      stateDirs: [".config/my-skill"]
      example: "config = { env = { MY_API_KEY = \"xxx\"; }; };"
    cliHelp: "my-skill --help\nUsage: my-skill [OPTIONS]"
---

# My Skill

## Installation

...

## Usage

...
```

### Parser Implementation

```typescript
// convex/lib/skillPublish.ts
import yaml from 'yaml';
import { parseMarkdown } from './markdown';

export function parseSkillFrontmatter(content: string) {
  const match = content.match(/^---\s*\n([\s\S]*?)\n---\s*\n/);
  if (!match) {
    throw new Error("No frontmatter found");
  }

  const frontmatter = yaml.parse(match[1]);
  const body = content.slice(match[0].length);

  return {
    name: frontmatter.name,
    description: frontmatter.description,
    homepage: frontmatter.homepage,
    website: frontmatter.website,
    emoji: frontmatter.emoji,
    metadata: frontmatter.metadata ?? {},
    moltbot: frontmatter.metadata?.moltbot ?? frontmatter.metadata?.moltbot ?? {},
    cliHelp: frontmatter.metadata?.moltbot?.cliHelp,
    body,
    bodyHtml: parseMarkdown(body),
  };
}
```

## Embedding Generation

### Batch Processing

```typescript
// convex/lib/embeddings.ts
export async function generateEmbeddings(
  ctx: MutationCtx,
  skillId: Id<'skills'>,
  versionId: Id<'skillVersions'>,
  parsed: ParsedSkill,
) {
  // Combine all searchable text
  const searchText = [
    parsed.name,
    parsed.description,
    parsed.body,
    JSON.stringify(parsed.moltbot),
  ].join('\n\n');

  const embedding = await generateEmbedding(searchText);

  // Delete old embeddings for this version
  const existing = await ctx.db
    .query('skillEmbeddings')
    .withIndex('by_version', (q) => q.eq('versionId', versionId))
    .collect();

  for (const emb of existing) {
    await ctx.db.delete(emb._id);
  }

  // Insert new embedding
  await ctx.db.insert('skillEmbeddings', {
    skillId,
    versionId,
    ownerId: parsed.ownerUserId,
    embedding,
    isLatest: true,
    isApproved: true,
    visibility: 'public',
    updatedAt: Date.now(),
  });
}
```

## GitHub OAuth Integration

### Auth Configuration

```typescript
// convex/auth.ts
import { GitHub } from '@convex-dev/auth/providers/GitHub';

export const authConfig = {
  providers: [
    GitHub({
      clientId: process.env.AUTH_GITHUB_ID!,
      clientSecret: process.env.AUTH_GITHUB_SECRET!,
    }),
  ],
};
```

### User Creation Flow

```typescript
// convex/auth.ts
export const onBeforeCreateUser = async (ctx: ActionCtx, identity: Identity) => {
  // Bootstrap first user as admin
  const users = await ctx.db.query('users').collect();

  return {
    handle: identity.profile.login,
    displayName: identity.profile.name ?? identity.profile.login,
    bio: identity.profile.bio,
    image: identity.profile.avatar_url,
    role: users.length === 0 ? 'admin' : 'user',
    createdAt: Date.now(),
    updatedAt: Date.now(),
  };
};
```

## Download ZIP Generation

### HTTP Handler

```typescript
// convex/downloads.ts
import { httpAction } from './_generated/actions';
import JSZip from 'jszip';

export const downloadZip = httpAction(async (ctx, request) => {
  const { skillId, version } = extractParams(request);

  // Get version files
  const skillVersion = await ctx.runQuery(query.getVersion, { skillId, version });
  if (!skillVersion) {
    return new Response('Not found', { status: 404 });
  }

  // Create ZIP
  const zip = new JSZip();

  for (const file of skillVersion.files) {
    const content = await ctx.runQuery(query.getFileContent, { storageId: file.storageId });
    zip.file(file.path, content);
  }

  // Add metadata
  zip.file('.molthub.json', JSON.stringify({
    skillId,
    version: skillVersion.version,
    publishedAt: skillVersion.createdAt,
  }));

  const blob = await zip.generateAsync({ type: 'blob' });

  return new Response(blob, {
    headers: {
      'Content-Type': 'application/zip',
      'Content-Disposition': `attachment; filename="${skillVersion.skillId}-${version}.zip"`,
    },
  });
});
```

## Install Telemetry

### Telemetry Sync

```typescript
// convex/httpApi.ts
export const cliTelemetrySyncHttp = httpAction(async (ctx, request) => {
  const body = await request.json();
  const { userId, skillId, version, rootId } = body;

  // Check for existing install
  const existing = await ctx.runQuery(query.getUserSkillRootInstalls, {
    userId,
    rootId,
    skillId,
  });

  if (existing.length === 0) {
    // New install
    await ctx.runMutation(mutations.createUserSkillRootInstalls, {
      userId,
      rootId,
      skillId,
      firstSeenAt: Date.now(),
      lastSeenAt: Date.now(),
      lastVersion: version,
    });

    // Update stats
    await ctx.runMutation(mutations.incrementInstallStats, { skillId });
  } else {
    // Update existing
    await ctx.runMutation(mutations.updateUserSkillRootInstalls, {
      installId: existing[0]._id,
      lastSeenAt: Date.now(),
      lastVersion: version,
    });
  }

  return new Response(JSON.stringify({ success: true }));
});
```

## Testing Strategy

### Unit Tests

```typescript
// convex/lib/skills.test.ts
import { describe, it, expect } from 'vitest';
import { parseSkillFrontmatter } from './skillPublish';

describe('parseSkillFrontmatter', () => {
  it('should parse valid frontmatter', () => {
    const content = `---
name: test-skill
description: A test skill
---

# Body content
`;
    const result = parseSkillFrontmatter(content);
    expect(result.name).toBe('test-skill');
    expect(result.description).toBe('A test skill');
    expect(result.body).toBe('# Body content\n');
  });

  it('should throw on missing frontmatter', () => {
    expect(() => parseSkillFrontmatter('No frontmatter here')).toThrow();
  });
});
```

### Integration Tests

```typescript
// convex/maintenance.test.ts
import { convexTest } from 'convex-test';
import { describe, it, expect } from 'vitest';

describe('skill publishing flow', () => {
  it('should publish a skill and create embeddings', async () => {
    const t = convexTest(schema, modules);

    // Login
    await t.run(async (ctx) => {
      await ctx.db.insert('users', { /* test user */ });
    });

    // Publish skill
    const result = await t.mutation(api.skills.publishSkill, {
      displayName: 'Test Skill',
      slug: 'test-skill',
      version: '1.0.0',
      changelog: 'Initial release',
      files: [/* test files */],
    });

    expect(result.skillId).toBeDefined();
    expect(result.versionId).toBeDefined();

    // Verify embedding was created
    const embeddings = await t.query(api.search.getEmbeddings, {
      skillId: result.skillId,
    });
    expect(embeddings.length).toBeGreaterThan(0);
  });
});
```

## Performance Optimizations

### Index Usage

```typescript
// Efficient queries using indexes
const skill = await ctx.db
  .query('skills')
  .withIndex('by_slug', (q) => q.eq('slug', slug))
  .unique();

const recentSkills = await ctx.db
  .query('skills')
  .withIndex('by_updated')
  .order('desc')
  .take(20);

const topSkills = await ctx.db
  .query('skills')
  .withIndex('by_stats_installs_current')
  .order('desc')
  .filter((q) => q.eq(q.field('softDeletedAt'), undefined))
  .take(50);
```

### Pagination

```typescript
export const listSkills = query({
  args: {
    cursor: v.optional(v.string()),
    limit: v.optional(v.number()),
  },
  handler: async (ctx, args) => {
    const limit = args.limit ?? 20;

    let query = ctx.db.query('skills').withIndex('by_updated').order('desc');

    if (args.cursor) {
      query = query.after(args.cursor);
    }

    const page = await query.take(limit + 1);
    const hasMore = page.length > limit;
    const items = hasMore ? page.slice(0, limit) : page;
    const nextCursor = hasMore ? items[items.length - 1]._id : undefined;

    return { items, nextCursor };
  },
});
```

---

*MoltHub API deep dive - Part of Moltbook ecosystem exploration*
