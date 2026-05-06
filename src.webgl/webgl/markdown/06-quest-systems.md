# Tiny Skies -- Quest Systems

Tiny Skies features multiple quest systems: package delivery quests, time trial races, hot-potato flag capture, paintball combat, and landmark selfie quests. Each quest system has its own UI, VFX, XP rewards, and server synchronization.

Source: `tinyskies/client/src/game/PackageQuest.ts` — delivery quests
Source: `tinyskies/client/src/game/RaceManager.ts` — time trial races
Source: `tinyskies/client/src/game/FlagSystem.ts` — flag capture visuals
Source: `tinyskies/client/src/game/PaintballSystem.ts` — combat projectiles
Source: `tinyskies/client/src/game/SkyGremlins.ts` — enemy AI (~1500 lines)
Source: `tinyskies/client/src/game/CarpetLandmarkSelfieQuest.ts` — selfie quests

## Package Delivery Quests

```typescript
// PackageQuest.ts
const PACKAGE_DELIVERIES_PER_WORLD = 3;

class PackageQuestManager {
  private activeOffers: PackageOffer[] = [];  // 3 concurrent offers

  generateOffers(): PackageOffer[] {
    // Pick 3 origin/destination village pairs
    // Each offer has a pickup point (origin) and delivery point (destination)
  }
}

interface PackageOffer {
  originVillage: string;
  destinationVillage: string;
  accepted: boolean;
  pickedUp: boolean;
  completed: boolean;
}
```

### Visual Design

**Pickup beam**: Golden beam shader with gradient (bright at bottom, fading at top) and pulsing animation. Marks the origin village where the player picks up the package.

**Delivery beam**: Blue beam shader, same gradient pattern. Marks the destination village.

**Package model**: Box with straps, swinging physics (pendulum with gravity/damping/inertia). The package is displayed attached to the underside of the vehicle, swinging as the vehicle turns.

```typescript
// Package physics — pendulum model
class PackageModel {
  private angle = 0;        // Pendulum angle
  private velocity = 0;     // Angular velocity
  private gravity = 9.8;
  private damping = 0.95;

  update(delta: number, vehicleAcceleration: Vector3): void {
    // Pendulum acceleration from vehicle movement
    this.velocity += (-this.gravity * Math.sin(this.angle) + inertia) * delta;
    this.velocity *= this.damping;
    this.angle += this.velocity * delta;
  }
}
```

**Destination arrow**: Down-pointing chevron, gentle bob animation, points toward the delivery location.

### Fill and Decay

```typescript
// Fill rate: 1 unit per 1.5 seconds of being near destination
// Decay rate: 0.3 units per second when away from destination
if (nearDestination) {
  progress += (1 / 1.5) * delta;
} else {
  progress -= 0.3 * delta;
}
progress = clamp(progress, 0, 1);

if (progress >= 1) {
  this.completeDelivery();
  awardXP(DELIVERY_XP);  // 50 XP base
}
```

### Dialogue Generation

NPC dialogue is generated with randomized text templates, using the village names and delivery context:

```typescript
function generateDialogue(offer: PackageOffer, isNpcMale: boolean): string {
  const templates = isNpcMale ? [
    "Take this to {destination}. Tell {village} I said hello.",
    "My cousin in {destination} needs these supplies.",
  ] : [
    "Could you deliver this to {destination}?",
    "My friend at {destination} has been waiting for this.",
  ];
  return templates[Math.floor(Math.random() * templates.length)]
    .replace("{destination}", offer.destinationVillage)
    .replace("{village}", offer.originVillage);
}
```

## Race Manager

```typescript
// RaceManager.ts
const RACE_TIME_LIMIT = 45;        // 45 seconds for plane
const CARPET_RACE_TIME_LIMIT = 40; // 40 seconds for carpet
const CHECKPOINT_COUNT = 12;       // 12 checkpoint rings + FINISH

class RaceManager {
  private checkpoints: CheckpointRing[] = [];
  private currentCheckpoint = 0;
  private raceTimer: RaceTimerUI | null = null;
  private countdownActive = false;

  // S-curve path generation with chained heading offsets
  generateRacePath(startPos: Quaternion): void {
    const headings = [0, 30, -30, 45, -45, 20, -20, 0];
    let pos = startPos;

    for (const headingOffset of headings) {
      pos = moveOnSphere(pos, pos.heading + headingOffset, arcDistance);
      this.checkpoints.push({
        position: pos,
        checkpointIndex: this.checkpoints.length,
        collected: false,
        hasBonusDiamond: Math.random() < 0.3,
      });
    }
  }
}
```

### Race Countdown

The race starts with a 3-2-1 countdown displayed as a circular progress ring. The player must wait for "GO!" before crossing the first checkpoint — early starts don't count.

### Race Banners

Race checkpoints are marked by rings carried by hot-air balloons. The final checkpoint is a FINISH banner (canvas texture on a floating rectangle).

### Time Limit

If the player doesn't finish within the time limit, the race fails. The UI shows a countdown timer in the top center of the screen, turning red in the last 10 seconds.

## Paintball Combat

```typescript
// PaintballSystem.ts
class PaintballSystem {
  private projectiles: PaintballProjectile[] = [];

  // Client fires locally with optimistic spawn
  tryLocalFire(playerState: PlayerState): boolean {
    const now = Date.now();
    if (now - this.lastFireMs < PAINTBALL_COOLDOWN_MS) {
      // Check for double-tap burst
      if (now - this.lastFireMs < PAINTBALL_BURST_WINDOW_MS) {
        // Burst fire: fire second projectile immediately
        this.fireProjectile(playerState);
      }
      return false;  // Cooldown
    }
    this.lastFireMs = now;
    this.fireProjectile(playerState);
    return true;
  }

  private fireProjectile(playerState: PlayerState): void {
    // Calculate ray from player nose (aligned with server hit test)
    const ray = paintballRayFromPlaneState(
      playerState.position,
      playerState.heading,
      playerState.pitch,
    );

    // Spawn projectile at player nose position
    const projectile = {
      position: ray.origin.clone(),
      direction: ray.direction.clone(),
      speed: PAINTBALL_SPEED,
      color: pickRandomColor(),
      distanceTraveled: 0,
      maxRange: playerState.speed * PAINTBALL_RANGE_FACTOR,
      // Spherical arc tracking: r0, rHat, wHat
    };

    // Optimistic: add to scene immediately
    this.projectiles.push(projectile);

    // Notify server
    this.socketClient.emitPaintballFire({
      direction: playerState.heading,
      color: projectile.color,
      upgrades: this.currentUpgrades,
    });
  }
}
```

### Projectile Movement

Projectiles travel along great-circle arcs using spherical math:

```typescript
// Projectile update
update(delta: number): void {
  const r0 = this.startPosition;    // Starting quaternion
  const rHat = this.directionVector; // Great circle normal
  const wHat = cross(r0, rHat);      // Perpendicular

  // Advance along arc
  const angle = this.speed * delta;
  this.position = r0.clone().multiplyScalar(Math.cos(angle))
    .add(wHat.clone().multiplyScalar(Math.sin(angle)));

  this.distanceTraveled += this.speed * delta;
}
```

### Splatter Decals

```typescript
// Splatter system — DecalGeometry
function createSplatterDecal(hitPoint: Vector3, hitNormal: Vector3, color: number): DecalMesh {
  // Raycaster hit test on meshes marked paintSplatterSurface
  const decalGeo = new DecalGeometry(
    hitMesh,
    hitPoint,
    hitNormal,
    splatterSize
  );

  // Strip back-facing triangles from decal geometry
  // (optimization: only render the front face)
  const indices = decalGeo.getIndex();
  const frontFaces = stripBackFaces(indices, hitNormal);
  decalGeo.setIndex(frontFaces);

  // Fade over SPLATTER_LIFETIME_SEC (14s) with smooth ease-out
  return decalMesh;
}
```

### Balloon Hit Detection

```typescript
// Segment-to-sphere test for paintball hitting hot-air balloons
function testBalloonHit(segmentStart: Vector3, segmentEnd: Vector3,
                        balloonCenter: Vector3, balloonRadius: number): boolean {
  const closestPoint = closestPointOnSegmentToSphere(segmentStart, segmentEnd, balloonCenter);
  return closestPoint.distanceTo(balloonCenter) < balloonRadius;
}
```

## Sky Gremlins

```typescript
// SkyGremlins.ts
const GREMLIN_COUNT = 3;           // Base gremlins
const MAX_ACTIVE_GREMLINS = 5;     // All active at moon phase >= 0.75
const GREMLIN_TAKEDOWNS_FOR_KING = 7;  // King spawns after 7 kills

class SkyGremlin {
  hp: number;        // 3 HP normal, 10 HP king
  state: "alive" | "falling" | "respawning" | "dormant";
  aiState: "orbit" | "chase" | "retreat";
  shootCooldown: number;
  position: Vector3;

  updateAI(delta: number, player: Plane): void {
    const distToPlayer = this.position.distanceTo(player.position);

    switch (this.aiState) {
      case "orbit":
        // Orbit player at standoff distance
        this.orbitAroundPlayer(delta, player);
        if (distToPlayer > orbitRadius + threshold) {
          this.aiState = "chase";
        }
        break;

      case "chase":
        // Approach player
        this.moveToward(player.position, delta);
        if (distToPlayer < chaseRange) {
          this.aiState = "retreat";
        }
        // Shoot at player
        if (distToPlayer < shootRange && this.shootCooldown <= 0) {
          this.fireAt(player.position);
          this.shootCooldown = randomBetween(1.85, 2.95);
        }
        break;

      case "retreat":
        // Move away from player
        this.moveAway(player.position, delta);
        if (distToPlayer > retreatDistance) {
          this.aiState = "orbit";
        }
        break;
    }

    // Retaliate mode: after non-lethal hit, prefer long-range for 5.5s
    if (this.retaliateTimer > 0) {
      this.retaliateTimer -= delta;
      this.preferLongRange = true;
    }
  }
}
```

### Gremlin King

The Gremlin King spawns after 7 regular gremlin takedowns:
- 10 HP (vs 3 HP for regular)
- 2x size with crown and trident visual
- Slower fire rate (2.5-3.85s cooldown vs 1.85-2.95s)
- Trident ember particles (44-point particle system)
- Upon defeat, triggers eternal flame reward (one brazier burns forever)

### HP Bars

```typescript
// HP bar: rounded box geometry above gremlin, camera-facing
const hpBarGeo = new RoundedBoxGeometry(width, height, depth, segments, radius);
const hpBar = new Mesh(hpBarGeo, new MeshBasicMaterial({ color }));

update(delta: number): void {
  // Billboard: face camera
  this.hpBar.lookAt(camera.position);
  // Width proportional to remaining HP
  this.hpBar.scale.x = this.hp / this.maxHp;
}
```

## Hot-Potato Flag System

See [Multiplayer Networking](05-multiplayer-networking.md) for the server-side flag state machine. The client visuals include:

- **Free flag**: Golden beam (same shader as package quests), bob animation, spinning flag cloth with vertex wave shader, sparkles
- **Held flag**: Attached to local player group, point light indicator
- **Remote players**: Show flag marker above the carrier's vehicle
- **Capture UI**: `CircularProgressRing` with flag icon, HUD carrier warning

## Landmark Selfie Quest (Carpet)

When flying the magic carpet, players can take selfies at landmarks:

```typescript
// CarpetLandmarkSelfieQuest.ts
const LANDMARK_SELFIE_XP = 25;

update(player: Carpet): void {
  // Check if carpet is near a landmark
  const playerNormal = tangentFrame(player.position).up;
  const refUp = new Vector3(0, 1, 0);
  const alignment = dot(playerNormal, refUp);

  if (alignment > selfieThreshold && nearLandmark) {
    this.triggerSelfieSequence();
    awardXP(LANDMARK_SELFIE_XP);
  }
}
```

See [Progression](07-progression-upgrades.md) for XP rewards and upgrade cards.
See [Atmospheric VFX](08-atmospheric-vfx.md) for moon phase effects on gremlins.
