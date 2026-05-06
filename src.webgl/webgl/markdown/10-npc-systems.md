# Tiny Skies -- NPC Systems

Non-player characters in Tiny Skies include NPC planes, boats, sky jellyfish, sky gremlins, and various ground NPCs (villagers, shopkeepers). Each NPC type has distinct AI behaviors, visual models, and interaction patterns.

Source: `tinyskies/client/src/game/NpcPlanes.ts` — NPC biplanes
Source: `tinyskies/client/src/game/NpcBoats.ts` — NPC boats
Source: `tinyskies/client/src/game/SkyJellyfish.ts` — collectible jellyfish
Source: `tinyskies/client/src/game/SkyGremlins.ts` — enemy gremlins (~1500 lines)
Source: `tinyskies/client/src/game/VoidMoths.ts` — cosmic void moths
Source: `tinyskies/client/src/game/BirdFlock.ts` — flock formations
Source: `tinyskies/client/src/game/GremlinHearts.ts` — heart particles

## NPC Planes

```typescript
// NpcPlanes.ts
class NpcPlanes {
  // NPC biplanes that fly around the world
  // Used for visual ambiance — not interactive
  // Follow predetermined patrol routes

  private planes: NpcPlane[] = [];

  update(delta: number): void {
    for (const plane of this.planes) {
      // Follow patrol route: waypoints on sphere surface
      const waypoint = this.getNextWaypoint(plane);
      plane.heading = headingToward(plane.position, waypoint);
      plane.position = moveOnSphere(plane.position, plane.heading, plane.speed * delta);

      // Biplane mesh rendering
      plane.updateMesh();
    }
  }
}
```

NPC planes fly along preset routes, adding life to the sky. They are non-interactive — players cannot shoot them or interact with them.

## NPC Boats

```typescript
// NpcBoats.ts
class NpcBoats {
  // NPC boats that sail on the main ocean
  // Constrained to ocean region (identified by BFS flood-fill)

  private boats: NpcBoat[] = [];

  update(delta: number): void {
    for (const boat of this.boats) {
      // Follow coastal route
      boat.position = moveOnSphere(boat.position, boat.heading, boat.speed * delta);

      // Ensure boat is on ocean
      if (terrainHeightAt(boat.position) > 0) {
        // Push back to ocean
        boat.heading = headingTowardOcean(boat.position);
      }

      // Bob animation (same as player boat)
      boat.updateBob(delta);
    }
  }
}
```

## Sky Jellyfish

```typescript
// SkyJellyfish.ts
const JELLY_COUNT = 24;
const JELLY_CAPTURE_XP = 25;

class SkyJellyfish {
  private jellyfish: Jellyfish[] = [];

  // Sky jellyfish: floating translucent creatures
  // Collectible for XP — like diamonds but rarer
  // Bioluminescent glow shader

  update(delta: number, player: Plane | Carpet): void {
    for (const jelly of this.jellyfish) {
      // Float in sky: gentle up/down bob
      jelly.bobHeight = Math.sin(time * jelly.bobSpeed) * jelly.bobAmplitude;

      // Check player proximity for collection
      if (distanceToPlayer < collectionRadius) {
        this.captureJellyfish(jelly);
        awardXP(JELLY_CAPTURE_XP);
      }
    }
  }
}
```

### Jellyfish Mesh

```typescript
// SkyJellyfishMesh.ts
class SkyJellyfishMesh extends Group {
  // Bell: hemisphere geometry with translucent material
  // Tentacles: curve tubes hanging down
  // Bioluminescent glow: shader-based pulsing light

  // The bell uses a custom shader with:
  // - Translucency (semi-transparent material)
  // - Pulsing glow (time-based intensity)
  // - Color cycling (blue → purple → teal)
}
```

### Jellyfish NPC Speaker

One specific jellyfish serves as an NPC with dialogue:

```typescript
// From PackageDialogue.ts
const JELLYFISH_NPC_SPEAKER = "jellyfish";

function getJellyfishCaptureLine(): string {
  return randomFrom([
    "Thank you for setting me free...",
    "The sky was getting lonely.",
    "I'll find my way back to the sea.",
  ]);
}
```

## Void Moths

```typescript
// VoidMoths.ts
class VoidMothsManager {
  // Cosmic void enemies for carpet players
  // Moths with forewing/hindwing geometry (low-poly kite shapes)
  // Shared wing geometries between moths for performance

  private moths: VoidMoth[] = [];

  update(delta: number, carpet: Carpet): void {
    for (const moth of this.moths) {
      // Flight: jitter around flame position
      moth.position = moth.basePosition + jitter(delta);

      // Post-hit slow: 0.26x speed for 1 second
      if (moth.hitTimer > 0) {
        moth.speed *= 0.26;
      }

      // HP bars
      moth.updateHPBar();

      // Ribbon trails (orange-red gradient)
      // Elder moths use purple-to-teal gradient
    }
  }
}
```

### Mothwing Eldest

The boss moth has enhanced stats:
- 2.8x scale (much larger than normal moths)
- 9 HP (vs 3 HP for normal moths)
- Slower flight speed
- Purple-to-teal ribbon trail (vs orange-red for normal)

## Bird Flock Formations

```typescript
// BirdFlock.ts
const BIRD_FLOCK_COUNT = 5;
const FLOCK_FORMATION_XP = 15;

class BirdFlock {
  // Flocks of birds flying in formation
  // Collectible XP reward for flying through them

  update(delta: number, player: Plane | Carpet): void {
    // V-formation movement
    // Lead bird sets direction, followers maintain offset

    if (distanceToPlayer < collectionRadius) {
      awardXP(FLOCK_FORMATION_XP);
      this.playFlockSound();
    }
  }
}
```

## Gremlin Hearts

```typescript
// GremlinHearts.ts
class GremlinHearts {
  // Heart particles emitted when gremlin is hit
  // Visual feedback for damage

  onGremlinHit(position: Vector3): void {
    this.emitHeart(position);
    // Heart: pink particle that rises and fades
  }
}
```

## Ground NPCs (Globe.ts)

Ground NPCs are placed as sprite images at village locations:

```
public/npc/
├── auntie_rue.png
├── baker_finch.png
├── beekeeper_thyme.png
├── capatain_moss.png
├── clockmaster_gale.png
├── cobbler_pip.png
├── doctor_celeste.png
├── farmer_oats.png
├── fisherman_cork.png
├── granny_maple.png
├── jellyfish.png
├── librarian_sage.png
├── mayor_bramble.png
├── nana_clover.png
├── old_barnaby.png
├── postmaster_quill.png
```

Each NPC has a whimsical name (profession + nature-themed surname). They appear as 2D sprites in their respective villages.

## Balloon NPCs

Hot-air balloons carry NPCs who greet the player when they fly close:

```typescript
// Balloon greeting system
const BALLOON_GREET_DIST = 1.2;         // World units
const BALLOON_GREET_EXIT_DIST = 1.75;   // Exit distance
const BALLOON_GREET_COOLDOWN = 32;       // Seconds before same balloon greets again

function checkBalloonGreetings(player: Plane | Carpet): void {
  for (const balloon of this.balloons) {
    const dist = distanceToPlayer(balloon, player);

    if (dist < BALLOON_GREET_DIST && !balloon.greeted) {
      this.showDialogue(balloon.npc, pickBalloonGreeting(balloon.npc));
      balloon.greeted = true;
    } else if (dist > BALLOON_GREET_EXIT_DIST) {
      balloon.greeted = false;
      // Cooldown: same balloon can't greet again for 32 seconds
      balloon.cooldownUntil = Date.now() + BALLOON_GREET_COOLDOWN * 1000;
    }
  }
}
```

### Other NPC Proximity Interactions

| NPC Type | Trigger Distance | Exit Distance | Cooldown |
|----------|-----------------|---------------|----------|
| Hot-air balloon | 1.2 | 1.75 | 32s |
| Observatory | 1.6 | 2.2 | 40s |
| Stonehenge whisper | 1.8 | 2.4 | 45s |
| Brazier whisper | 1.6 | 2.2 | 60s |

See [Quest Systems](06-quest-systems.md) for package quest dialogue and gremlin AI.
See [Atmospheric VFX](08-atmospheric-vfx.md) for moon phase effects.
