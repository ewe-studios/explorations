# Tiny Skies -- Particle Systems

Particle systems provide visual feedback for nearly every game action: contrails behind the plane, wake trails behind vehicles, drift smoke from the carpet, ring collection VFX, paintball splashes, and more. All particle systems use instanced rendering or point sprites for performance.

Source: `tinyskies/client/src/game/Contrails.ts` — plane contrails
Source: `tinyskies/client/src/game/WakeTrail.ts` — vehicle wake trails
Source: `tinyskies/client/src/game/CarpetTrail.ts` — carpet trail particles
Source: `tinyskies/client/src/game/CarpetWake.ts` — carpet water wake
Source: `tinyskies/client/src/game/CarpetDriftSmoke.ts` — drift smoke
Source: `tinyskies/client/src/game/RingCollectVFX.ts` — ring collection effects
Source: `tinyskies/client/src/game/PaintballSplash.ts` — paintball impact particles
Source: `tinyskies/client/src/game/CarpetLeaves.ts` — carpet leaf particles

## Contrails

```typescript
// Contrails.ts
class Contrails {
  // White smoke trails behind plane wingtips
  // Activated during boost/roll maneuvers
  // Implemented as a trail renderer using a ribbon of connected quads

  private segments: ContrailSegment[] = [];
  private maxAge = 3.0;  // Seconds before fading out

  update(player: Plane, delta: number): void {
    // Emit new segments when boosting or rolling
    if (player.boosting || Math.abs(player.rollAngle) > 0.3) {
      this.emitSegment(player.wingtipPosition);
    }

    // Update existing segments
    for (const seg of this.segments) {
      seg.age += delta;
      seg.opacity = 1 - smoothstep(0, this.maxAge, seg.age);
    }

    // Remove expired segments
    this.segments = this.segments.filter(s => s.age < this.maxAge);
  }
}
```

Contrails are rendered as a ribbon — a series of connected quad segments that follow the wingtip positions. Each segment fades out over 3 seconds, creating a dissipating smoke effect.

## Wake Trail

```typescript
// WakeTrail.ts
class WakeTrail {
  // Water wake behind vehicle (plane on water, boat)
  // Foam particles that spread outward and fade

  update(player: Plane | Boat, delta: number): void {
    if (player.altitude < wakeThreshold) {
      this.emitFoam(player.position, player.speed);
    }

    // Foam particles: rise, spread, fade
    for (const particle of this.foamParticles) {
      particle.position.y += particle.riseSpeed * delta;
      particle.position.x += particle.spreadX * delta;
      particle.opacity -= particle.fadeRate * delta;
    }
  }
}
```

## Carpet Trail

```typescript
// CarpetTrail.ts
class CarpetTrail {
  // Magical trail particles behind carpet
  // Shimmering, color-shifting particles
  // Uses Points with custom shader for glow effect

  update(carpet: Carpet, delta: number): void {
    // Emit particles at carpet trailing edge
    if (carpet.speed > 0.1) {
      this.emitParticle(carpet.trailPosition);
    }

    // Shader: shimmering rainbow effect
    this.material.uniforms.time.value = performance.now() * 0.001;
  }
}
```

## Carpet Drift Smoke

```typescript
// CarpetDriftSmoke.ts
class CarpetDriftSmoke {
  // Smoke puffs emitted during carpet drift (sharp turn at speed)
  // Brownish-grey particles that rise and dissipate

  update(carpet: Carpet, delta: number): void {
    if (Math.abs(carpet.driftVelocity) > driftSmokeThreshold) {
      this.emitSmoke(carpet.cornerPosition);
    }

    // Smoke: rise with turbulence, fade
    for (const puff of this.puffs) {
      puff.position.y += puff.riseSpeed * delta;
      puff.position.x += Math.sin(puff.phase + puff.speed * delta) * 0.01;
      puff.opacity = 1 - smoothstep(0, smokeLifetime, puff.age);
    }
  }
}
```

## Ring Collection VFX

```typescript
// RingCollectVFX.ts
class RingCollectVFX {
  // Flash and sparkle when player collects a diamond
  // Expanding ring + burst of points

  onCollect(position: Vector3): void {
    // Expanding ring: starts small, grows and fades
    this.ringScale = 0;
    this.ringOpacity = 1;

    // Burst points: 20 particles in random directions
    for (let i = 0; i < 20; i++) {
      this.particles.push({
        position: position.clone(),
        velocity: randomDirection().multiplyScalar(burstSpeed),
        lifetime: 0.5,
        color: goldColor,
      });
    }
  }
}
```

## Paintball Splash

```typescript
// PaintballSplash.ts
class PaintballSplash {
  // 8-color paint palette for splatter particles
  const COLORS = [
    0xff0000, 0x00ff00, 0x0000ff, 0xffff00,
    0xff00ff, 0x00ffff, 0xff8800, 0x8800ff,
  ];

  onImpact(position: Vector3, normal: Vector3, color: number): void {
    // Splash pool: drops of paint spreading on impact surface
    for (let i = 0; i < splashCount; i++) {
      this.splashes.push({
        position: position.clone(),
        velocity: tangentVector(normal).multiplyScalar(splashSpeed),
        color,
        size: splashSize,
        lifetime: SPLATTER_LIFETIME_SEC,  // 14 seconds
      });
    }

    // Smooth ease-out fade over lifetime
    // splashes fade gradually, then are removed
  }
}
```

## Void Carpet Trail

```typescript
// VoidCarpetTrail.ts
class VoidCarpetTrail {
  // Dark, ethereal trail in cosmic void mode
  // Purple/teal color scheme (matches void moths)
  // More intense than normal carpet trail

  update(carpet: Carpet, delta: number): void {
    if (carpet.inCosmicVoid && carpet.speed > 0.1) {
      this.emitVoidParticle(carpet.trailPosition);
    }
  }
}
```

## Carpet Leaves

```typescript
// CarpetLeaves.ts
class CarpetLeaves {
  // Falling leaf particles from carpet tassels
  // Seasonal color variation (green, yellow, orange, red)

  update(carpet: Carpet, delta: number): void {
    // Emit leaves at random intervals
    if (Math.random() < leafEmitRate * delta) {
      this.emitLeaf(carpet.tasselPosition);
    }

    // Leaves fall with wind drift
    for (const leaf of this.leaves) {
      leaf.position.y -= leaf.fallSpeed * delta;
      leaf.position.x += Math.sin(leaf.phase + time * windSpeed) * 0.02;
      leaf.rotation += leaf.spinSpeed * delta;
    }
  }
}
```

## Braziers — Ember Particles

```typescript
// Braziers.ts
const EMBER_COUNT = 36;  // Per brazier

class BrazierEmbers {
  // Ember particles rise through flame column
  // Seeded random phase/speed for each ember
  // Warm color gradient (red → orange → yellow → transparent)

  update(delta: number): void {
    for (let i = 0; i < EMBER_COUNT; i++) {
      const ember = this.embers[i];
      ember.y += ember.riseSpeed * delta;
      ember.x = baseX + Math.sin(ember.phase + ember.speed * ember.y) * 0.02;

      // Color gradient based on height
      const t = ember.y / flameHeight;
      ember.color = lerpColor(red, yellow, t);
      ember.opacity = 1 - t;  // Fade at top

      if (ember.y > flameHeight) {
        // Reset to bottom with new random phase
        ember.y = 0;
        ember.phase = Math.random() * Math.PI * 2;
      }
    }
  }
}
```

The flame uses a **cylindrical billboard** shader — horizontal rotation tracks the camera (always faces the player), while vertical orientation is locked to the globe normal (flame always points "up" relative to the globe surface).

## Moon Ember System

```typescript
// MoonThreat.ts
const EMBER_COUNT = 1500;

class MoonEmbers {
  // 1500 point sprites with fire trails
  // Intensity increases with moon progress

  update(delta: number, moonProgress: number): void {
    for (const ember of this.embers) {
      // Fire trail: point moves upward with slight horizontal drift
      ember.position.y += ember.speed * delta;
      ember.position.x += Math.sin(ember.phase + ember.speed * ember.y) * 0.005;

      // Reset when reaching top
      if (ember.position.y > maxRise) {
        ember.position.y = 0;
        ember.phase = Math.random() * Math.PI * 2;
      }

      // Color: orange → red as moon gets closer
      ember.color = lerpColor(orange, red, moonProgress);
    }
  }
}
```

See [Atmospheric VFX](08-atmospheric-vfx.md) for meteor showers and aurora particles.
See [NPC Systems](10-npc-systems.md) for gremlin ember particles.
