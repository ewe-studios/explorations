# Tiny Skies -- Server Architecture

The server is an Express.js application with Socket.IO for real-time communication, Prisma ORM for PostgreSQL, and REST API routes for world management, event tracking, and the admin dashboard.

Source: `tinyskies/server/src/index.ts` — Express app, Socket.IO setup
Source: `tinyskies/server/src/rooms/RoomManager.ts` — room lifecycle
Source: `tinyskies/server/src/rooms/Room.ts` — per-world room state
Source: `tinyskies/server/src/routes/` — REST API routes
Source: `tinyskies/server/src/paintball/` — server hit testing

## Server Entry Point

```typescript
// server/src/index.ts
import express from "express";
import { createServer } from "http";
import { Server } from "socket.io";
import { PrismaClient } from "@prisma/client";

const app = express();
const httpServer = createServer(app);

const io = new Server(httpServer, {
  cors: {
    origin: VERCEL_URLS,  // Allowlist of Vercel deployment URLs
    methods: ["GET", "POST"],
  },
  transports: ["websocket"],  // WebSocket only (no polling)
});

const prisma = new PrismaClient();

// Mount REST routes
app.use("/api/worlds", createWorldsRoute(prisma));
app.use("/api/lanterns", createLanternsRoute(prisma));
app.use("/api/save-feed", createSaveFeedRoute(prisma));
app.use("/api/events", createEventsRoute(prisma));
app.use("/api/dashboard/worlds", createDashboardRoute(prisma));

// Admin dashboard HTML
app.get("/dashboard", (req, res) => {
  res.send(adminDashboardHTML);
});

// Socket.IO connection handling
io.on("connection", (socket) => {
  socket.on("world:join", async (data) => {
    const { worldSlug } = data;
    let slug = worldSlug;

    if (!slug) {
      // Auto-join: find best available world
      slug = await roomManager.findBestWorld();
    }

    const room = roomManager.getOrCreateRoom(slug);
    room.joinRoom(socket, slug);
  });

  socket.on("disconnect", () => {
    roomManager.handleDisconnect(socket);
  });
});

// Auto-seed system worlds on startup
await seedSystemWorlds(prisma);

httpServer.listen(PORT);
```

### CORS Configuration

```typescript
const VERCEL_URLS = [
  "https://tinyskies.vercel.app",
  `https://${process.env.VERCEL_URL}`,  // Dynamic preview URL
];
```

The CORS allowlist includes both the production Vercel URL and the dynamic preview URL (`VERCEL_URL` env var), allowing Vercel preview deployments to connect to the production server.

## REST API Routes

### Worlds Route (`/api/worlds`)

```typescript
// routes/worlds.ts
router.post("/auto-join", async (req, res) => {
  // Serialized lock to prevent race condition on world creation
  const lock = await acquireWorldCreationLock();

  try {
    // Find best world (non-full, seeded)
    const world = await roomManager.findBestWorld(prisma);
    res.json({ world });
  } finally {
    lock.release();
  }
});

router.post("/create", async (req, res) => {
  const { name, terrainType, seed } = req.body;

  // Validate inputs
  if (!name || name.length > 50) {
    return res.status(400).json({ error: "Invalid name" });
  }

  // Generate slug from name
  const slug = slugify(name);

  // Create world in database
  const world = await prisma.world.create({
    data: { slug, name, terrainType, seed, globeRadius: 5.0 },
  });

  res.json({ world });
});
```

The **serialized lock** prevents multiple simultaneous world creation requests from creating duplicate worlds. Only one world-creation operation can proceed at a time.

### Lantern Route (`/api/lanterns`)

```typescript
// routes/lanterns.ts
router.post("/log", async (req, res) => {
  const { worldSlug, playerId, count } = req.body;

  await prisma.lanternLedger.create({
    data: { worldSlug, playerId, count, createdAt: new Date() },
  });

  res.json({ ok: true });
});
```

The lantern ledger tracks how many lantern clusters each player has collected per world, used for the lantern leaderboard.

### Save Feed Route (`/api/save-feed`)

```typescript
// routes/saveFeed.ts
router.get("/recent", async (req, res) => {
  const entries = await prisma.saveFeedEntry.findMany({
    orderBy: { createdAt: "desc" },
    take: 20,
  });
  res.json({ entries });
});

router.post("/entry", async (req, res) => {
  const { worldSlug, vehicle, level, xp } = req.body;

  await prisma.saveFeedEntry.create({
    data: { worldSlug, vehicle, level, xp, createdAt: new Date() },
  });

  res.json({ ok: true });
});
```

### Events Route (`/api/events`)

```typescript
// routes/events.ts
router.post("/", async (req, res) => {
  const { type, worldSlug, vehicle, data } = req.body;

  await prisma.gameEvent.create({
    data: { type, worldSlug, vehicle, data, createdAt: new Date() },
  });

  res.json({ ok: true });
});

// Dashboard stats endpoint
router.get("/dashboard", async (req, res) => {
  const [recentWorldSaves, recentQuestCompletions, totals, todayTotals, vehicleCounts] =
    await Promise.all([
      prisma.gameEvent.findMany({
        where: { type: "world_saved" },
        orderBy: { createdAt: "desc" },
        take: 10,
      }),
      prisma.gameEvent.findMany({
        where: { type: "quest_completed" },
        orderBy: { createdAt: "desc" },
        take: 10,
      }),
      prisma.gameEvent.groupBy({
        by: ["type"],
        _count: true,
        _sum: { durationSec: true },
      }),
      // ... today's totals, vehicle counts
    ]);

  res.json({
    recentWorldSaves: recentWorldSaves.map(eventToJson),
    recentQuestCompletions: recentQuestCompletions.map(eventToJson),
    totals,
    todayTotals,
    vehicleCounts,
  });
});
```

## Paintball Server Logic

### Hit Test Algorithm

```typescript
// paintball/hitTest.ts
function computePaintballShot(
  shooterPos: Quaternion,
  shooterHeading: number,
  shooterPitch: number,
  victims: PlayerState[],
  maxRange: number
): { hitPlayerId: string, distance: number } | null {
  // Ray from shooter's nose along pitched forward direction
  const ray = paintballRayFromPlaneState(shooterPos, shooterHeading, shooterPitch);

  let closestHit: { playerId: string, distance: number } | null = null;

  for (const victim of victims) {
    // Closest distance from ray to victim's globe position point
    const dist = closestDistanceRayToPoint(ray, victim.position);

    if (dist < PAINTBALL_HIT_RADIUS && dist < maxRange) {
      if (!closestHit || dist < closestHit.distance) {
        closestHit = { playerId: victim.id, distance: dist };
      }
    }
  }

  return closestHit;
}
```

The server uses a **ray-point distance test** rather than bounding-box collision. This is simpler, faster, and works correctly with spherical geometry. The hit radius (`PAINTBALL_HIT_RADIUS = 0.14`) is tuned so that shots need to be reasonably accurate.

### Server-Side Cooldown Enforcement

```typescript
// Room.ts
processPaintballFire(playerId: string, data: PaintballFireData): void {
  const player = this.players.get(playerId);
  const now = Date.now();
  const elapsed = now - player.lastPaintballFireMs;

  // Single shot cooldown
  if (elapsed < PAINTBALL_COOLDOWN_MS) {
    // Double-tap burst check
    if (elapsed < PAINTBALL_BURST_WINDOW_MS) {
      // Burst fire allowed
    } else {
      return;  // Too early — ignore
    }
  }

  player.lastPaintballFireMs = now;
  this.broadcast("paintball:fired", { shooterId: playerId, ...data });
}
```

### Upgrade Clamping

```typescript
// Clamp paintball upgrades to prevent cheating
clampUpgrades(upgrades: PaintballUpgrades): PaintballUpgrades {
  return {
    speed: clamp(upgrades.speed, 0, maxSpeedUpgrade),
    range: clamp(upgrades.range, 0, maxRangeUpgrade),
    doubleTap: Boolean(upgrades.doubleTap),  // Don't allow arbitrary values
  };
}
```

## World Name Generation

```typescript
// utils/worldNames.ts
const ADJECTIVES = [
  "Whispering", "Golden", "Misty", "Verdant", "Crystal",
  "Sunlit", "Moonlit", "Starfall", "Dewdrop", "Thunder",
  // ... 30 total
];

const NOUNS = [
  "Haven", "Cove", "Hollow", "Meadow", "Summit",
  "Grove", "Bay", "Crest", "Dell", "Ridge",
  // ... 30 total
];

// 30 * 30 = 900 possible world names
function generateWorldName(): string {
  return `${pickRandom(ADJECTIVES)} ${pickRandom(NOUNS)}`;
}
```

## Flag Constants (Server)

```typescript
// flagConstants.ts
export const FLAG_COLLECT_RADIUS = 0.8;       // World units
export const FLAG_CAPTURE_RADIUS = 1.0;       // World units
export const FLAG_HOVER_ALTITUDE = 0.05;      // Globe units above surface
export const FLAG_CAPTURE_DURATION_MS = 3000;  // 3 seconds to capture
export const FLAG_IMMUNITY_MS = 10000;         // 10 seconds after capture
export const FLAG_AUTO_RESPAWN_MS = 45000;     // 45 seconds before auto-respawn
export const FLAG_SPAWN_DELAY_MS = 5000;       // 5 seconds before initial spawn
export const FLAG_CAPTURE_GRACE_MS = 300;      // 300ms grace period
```

These constants are duplicated between server and shared — the server uses its own copy for authoritative enforcement.

See [Multiplayer Networking](05-multiplayer-networking.md) for room management and state sync.
See [Database Schema](13-database-schema.md) for Prisma models.
