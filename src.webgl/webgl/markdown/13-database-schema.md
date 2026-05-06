# Tiny Skies -- Database Schema

The server uses PostgreSQL with Prisma ORM for persistent storage. The schema tracks world configurations, lantern collection ledgers, save feed entries (player activity), and game events (analytics).

Source: `tinyskies/server/prisma/schema.prisma` — Prisma schema
Source: `tinyskies/server/prisma/migrations/` — 5 migration sets
Source: `tinyskies/server/prisma/seed.ts` — system world seeding

## Prisma Schema

```prisma
// schema.prisma
generator client {
  provider = "prisma-client-js"
}

datasource db {
  provider = "postgresql"
  url      = env("DATABASE_URL")
}

model World {
  id          String   @id @default(cuid())
  slug        String   @unique
  name        String
  globeRadius Float    @default(5.0)
  texture     String?
  createdBy   String?
  seed        Int      // Terrain generation seed
  terrainType String?  // "default", "archipelago", "pangaea", "waterworld"
  createdAt   DateTime @default(now())

  @@map("worlds")
}

model LanternLedger {
  id        String   @id @default(cuid())
  worldSlug String
  playerId  String
  count     Int
  createdAt DateTime @default(now())

  @@index([worldSlug])
  @@index([createdAt])
  @@map("lantern_ledger")
}

model SaveFeedEntry {
  id        String   @id @default(cuid())
  worldSlug String
  vehicle   String   // "plane", "carpet", "boat"
  level     Int
  xp        Int
  createdAt DateTime @default(now())

  @@index([worldSlug, createdAt])
  @@map("save_feed_entries")
}

model GameEvent {
  id          String   @id @default(cuid())
  type        String   // "world_saved", "quest_completed", "flag_event", etc.
  worldSlug   String
  vehicle     String?
  data        Json?    // Arbitrary event data
  durationSec Int?     // Duration in seconds (for timed events)
  createdAt   DateTime @default(now())

  @@index([createdAt])
  @@index([type])
  @@index([worldSlug])
  @@map("game_events")
}
```

## Migrations

| Migration | Date | Changes |
|-----------|------|---------|
| `20260403043307_init` | 2026-04-03 | Initial schema: World, LanternLedger, SaveFeedEntry, GameEvent |
| `20260403072503_add_seed_and_terrain_type` | 2026-04-03 | Added `seed` (Int) and `terrainType` (String?) to World |
| `20260411070211_add_lantern_ledger` | 2026-04-11 | Created LanternLedger model |
| `20260425073632_add_save_feed_entry` | 2026-04-25 | Created SaveFeedEntry model |
| `20260428093700_add_game_event` | 2026-04-28 | Created GameEvent model with indexes |

## World Model

The World model stores the configuration for each game world:

| Field | Type | Purpose |
|-------|------|---------|
| `slug` | String (unique) | URL-safe identifier for room routing |
| `name` | String | Display name (generated from 900 name combinations) |
| `globeRadius` | Float | Radius of the sphere (default 5.0 world units) |
| `seed` | Int | Terrain generation seed (deterministic noise) |
| `terrainType` | String | Preset: "default", "archipelago", "pangaea", "waterworld" |

## Game Event Types

| Event Type | Description | Data Fields |
|-----------|-------------|-------------|
| `world_saved` | World completed (eternal flame) | { duration, vehicle, level } |
| `quest_completed` | Package delivery done | { deliveryIndex, xp } |
| `flag_event` | Flag capture/steal/drop | { eventType, playerId } |

## Database Seeding

```typescript
// prisma/seed.ts
const SYSTEM_WORLDS = [
  {
    slug: "whispering-haven",
    name: "Whispering Haven",
    seed: 12345,
    terrainType: "default",
    globeRadius: 5.0,
  },
  // ... 20 system worlds total
];

async function main() {
  for (const world of SYSTEM_WORLDS) {
    const existing = await prisma.world.findUnique({
      where: { slug: world.slug },
    });

    if (!existing) {
      await prisma.world.create({ data: world });
      console.log(`Seeded world: ${world.name}`);
    }
  }
}
```

The seed script runs on server startup (`prisma migrate deploy` in the Dockerfile). It creates 20 system worlds if they don't already exist. This ensures players always have worlds to join even if no custom worlds have been created.

## Index Strategy

| Model | Index | Purpose |
|-------|-------|---------|
| LanternLedger | `[worldSlug]` | Fast lookup of all lantern entries for a world |
| LanternLedger | `[createdAt]` | Time-range queries for leaderboard |
| SaveFeedEntry | `[worldSlug, createdAt]` | Composite: recent entries per world |
| GameEvent | `[createdAt]` | Time-range dashboard queries |
| GameEvent | `[type]` | Filter by event type |
| GameEvent | `[worldSlug]` | Filter by world |

The GameEvent table has the most indexes because it supports the most diverse queries: the dashboard needs to group by type, filter by world, and order by time.

## Number Handling

```typescript
// Server uses BigInt for numeric fields from PostgreSQL
// Client expects regular numbers
function numberFromDb(val: bigint | number): number {
  return typeof val === "bigint" ? Number(val) : val;
}
```

PostgreSQL returns `BIGINT` columns as BigInt in Node.js. The conversion function ensures consistent number types across the API.

See [Server Architecture](12-server-architecture.md) for API routes using these models.
See [Deployment](16-deployment.md) for Docker/Prisma setup.
