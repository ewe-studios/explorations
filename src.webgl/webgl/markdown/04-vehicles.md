# Tiny Skies -- Vehicle Systems

Tiny Skies features three vehicles — biplane, magic carpet, and boat — each with distinct physics, visual meshes, capabilities, and control feel. A fourth "void" variant of the carpet operates in the cosmic void flat-plane dimension.

Source: `tinyskies/client/src/game/Plane.ts` — biplane vehicle
Source: `tinyskies/client/src/game/Boat.ts` — boat vehicle
Source: `tinyskies/client/src/game/Carpet.ts` — magic carpet vehicle
Source: `tinyskies/shared/vehicleCapabilities.ts` — per-vehicle feature flags

## Vehicle Capabilities

```typescript
// vehicleCapabilities.ts
interface VehicleGameFeatures {
  collectibleDiamonds: boolean;
  xpProgressionUI: boolean;
  speedLines: boolean;
  contrails: boolean;
  wakeTrail: boolean;
  carpetTrail: boolean;
  cameraTiltScale: number;
  cameraFollowDistance: number;
  cameraFollowHeight: number;
  cameraSpeedZoom: number;
  cameraFovBoost: number;
  packageQuests: boolean;
  fishingMiniGame: boolean;
}
```

| Feature | Plane | Carpet | Boat |
|---------|-------|--------|------|
| Speed | 1.725 cruise | 0.6 cruise | 0.22 cruise |
| Max boost | +80% (1.3x) | +78% | +42% |
| Diamonds | 15 | 15 | 24 |
| Contrails | Yes | No | No |
| Wake trail | Yes | Yes (carpet trail) | Yes (water wake) |
| Package quests | Yes | Yes | No |
| Fishing | No | No | Yes |
| Portals | No | Yes (2 portals) | No |
| Cosmic void | No | Yes | No |

## Biplane Mesh

```typescript
// BiplaneMesh.ts
class BiplaneMesh extends Group {
  // Upper wing: box geometry, wider span
  // Lower wing: box geometry, narrower span
  // Wing struts: thin cylinders connecting wings
  // Fuselage: tapered cylinder
  // Propeller: spinning box blades
  // Tail: horizontal + vertical stabilizers
  // Landing gear: wheels + struts
}
```

The biplane has a distinctive two-wing configuration with struts, a spinning propeller, and landing gear. The entire mesh is built from Three.js primitives — no imported GLB models.

## Plane Physics

```typescript
// Plane.ts
class Plane {
  private speed = CRUISE_SPEED;  // 1.725
  private maxSpeed = MAX_SPEED;   // 0.8 additional
  private altitude = cruiseAlt;
  private elevationBlendSpeed = 2.5;
  private climbAlt = 0.45;
  private cruiseAlt = 0.15;

  update(delta: number, input: InputState): void {
    // Speed control
    let targetSpeed = CRUISE_SPEED + input.forward * this.maxSpeed;
    if (this.boosting) targetSpeed *= BOOST_MULTIPLIER;
    this.speed = lerp(this.speed, targetSpeed, 3.0 * delta);

    // Heading
    this.heading += input.turn * this.bankRate * delta;

    // Elevation
    const targetAlt = input.elevate ? this.climbAlt : this.cruiseAlt;
    this.altitude = lerp(this.altitude, targetAlt, this.elevationBlendSpeed * delta);

    // Barrel roll during boost (visual spin)
    if (this.boosting) {
      this.rollAngle += this.rollSpeed * delta;
    }

    // Gremlin hit: paintball wobble
    if (this.wobbleTimer > 0) {
      this.heading += Math.sin(this.wobbleTimer * 30) * 0.05;
    }

    // Gremlin slow debuff
    if (this.slowDebuffTimer > 0) {
      this.speed *= 0.6;
    }

    // Gremlin HP
    this.hp = Math.max(0, this.hp - gremlinDamage * delta);
    if (this.hp <= 0) {
      this.explode();
    }

    // Apply upgrade multipliers
    this.speed *= this.upgradeMultipliers.speed;
  }
}
```

**Gremlin HP system**: The plane has hit points (base 14 HP, modified by upgrade). Gremlin attacks deal damage per second. When HP reaches 0, the plane explodes (triggering the explosion SFX and a cinematic fall).

**Elevation blend**: The plane transitions smoothly between climb altitude (0.45) and cruise altitude (0.15). The blend speed of 2.5 means it takes about 0.4 seconds to transition, giving a responsive but smooth feel.

## Boat Physics

```typescript
// Boat.ts
class Boat {
  private speed = 0.22;
  private maxSpeed = 0.42;
  private diamondBoostSpeed = 0.58;
  private turnSmoothing = 0.3;

  update(delta: number, input: InputState): void {
    // Constrained to ocean
    const terrainHeight = sampleTerrainHeightAt(this.position);
    if (terrainHeight > 0) {
      // Land collision: slow down and push away
      this.speed *= 0.3;
    }

    // Bob animation
    const t = performance.now() * 0.001;
    this.bobPitch = Math.sin(t * 0.8) * 0.01;

    // Turn smoothing
    this.heading = lerpAngle(this.heading, this.targetHeading, this.turnSmoothing * delta);
  }
}
```

Boat movement is significantly slower than plane or carpet. The **land collision** prevents the boat from sailing onto land — it slows to a crawl and the player must turn back toward the ocean.

## Carpet Physics

```typescript
// Carpet.ts
class Carpet {
  private speed = 0.6;
  private maxSpeed = 0.78;
  private hoverOffset = 0.15;
  private driftVelocity = 0;
  private driftRate = 2.0;
  private inCosmicVoid = false;

  update(delta: number, input: InputState): void {
    // Hover altitude: follow terrain
    const terrainHeight = sampleTerrainHeightAt(this.position);
    const targetAlt = Math.max(terrainHeight + this.hoverOffset, this.minAltitude);
    this.altitude = lerp(this.altitude, targetAlt, this.hoverBlendSpeed * delta);

    // Drift: velocity heading decouples from facing heading
    if (Math.abs(input.turn) > 0.7 && this.speed > 0.5) {
      this.driftVelocity += input.turn * this.driftRate * delta;
    }
    this.driftVelocity *= 0.95;  // Damping
    const velocityHeading = this.heading + this.driftVelocity;

    // Cliff glide: near-terrain altitude boost
    if (terrainHeight > this.altitude - 0.05) {
      this.altitude = lerp(this.altitude, terrainHeight + 0.1, 0.3 * delta);
    }

    // Tassel curl animation
    // Capybara bob (passenger character)

    // Cosmic void mode
    if (this.inCosmicVoid) {
      this.updateVoidMode(delta, input);
    }
  }

  private updateVoidMode(delta: number, input: InputState): void {
    // Flat-plane coordinates (u, v) instead of spherical
    this.u += Math.cos(this.heading) * this.speed * delta;
    this.v += Math.sin(this.heading) * this.speed * delta;
  }
}
```

The carpet's **hover altitude tracking** makes it follow the terrain at a fixed offset. This means the carpet naturally follows hills and valleys without the player needing to manually adjust altitude — unlike the plane which has a fixed cruise altitude.

## Carpet Mesh

```typescript
// CarpetMesh.ts
class CarpetMesh extends Group {
  // Carpet body: flattened box with fringed edges
  // Tassels: small spheres at corners with spring animation
  // Capybara: passenger character (low-poly box character)
  // Flame shield: void-mode protective barrier
}
```

## Vehicle Colors

```typescript
// vehicleColors.ts
function pickRandomVehicleColor(): { hull: number, accent: number } {
  const palettes = [
    { hull: 0xe74c3c, accent: 0xf39c12 },  // Red/gold
    { hull: 0x3498db, accent: 0x2ecc71 },  // Blue/green
    { hull: 0x9b59b6, accent: 0xe67e22 },  // Purple/orange
    // ... more palettes
  ];
  return palettes[Math.floor(Math.random() * palettes.length)];
}
```

Each player's vehicle gets a random color palette at spawn, shown in the multiplayer lobby and on remote players.

See [Upgrade Manager](07-progression-upgrades.md) for how upgrades modify vehicle performance.
See [Multiplayer Networking](05-multiplayer-networking.md) for how vehicle state is synced.
