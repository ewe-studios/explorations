# Tiny Skies -- Atmospheric VFX

Atmospheric visual effects create the mood and ambiance of Tiny Skies. The day/night cycle drives sky presets, music crossfades, rain intensity, aurora borealis, and star visibility. Additional atmospheric effects include god rays, meteor showers, and sky jellyfish.

Source: `tinyskies/client/src/game/DayNightCycle.ts` — 195-second cycle
Source: `tinyskies/client/src/game/SkyPresets.ts` — sky color presets
Source: `tinyskies/client/src/game/GodRays.ts` — volumetric god rays
Source: `tinyskies/client/src/game/Aurora.ts` — aurora borealis
Source: `tinyskies/client/src/game/Starfield.ts` — night sky stars
Source: `tinyskies/client/src/game/MeteorShower.ts` — meteor impacts
Source: `tinyskies/client/src/game/MoonThreat.ts` — approaching moon

## Day/Night Cycle

```typescript
// DayNightCycle.ts — 195-second full cycle
const PHASES = {
  day: 60_000,           // 60 seconds of full daylight
  dayToEvening: 15_000,  // 15-second smooth transition
  evening: 30_000,       // 30 seconds of golden hour
  eveningToNight: 15_000, // 15-second smooth transition
  night: 60_000,         // 60 seconds of nighttime
  nightToDay: 15_000,    // 15-second smooth transition
};
// Total: 195 seconds (3 minutes 15 seconds)
```

### Cycle Timing

The cycle uses wall-clock time with a world-seed offset for client synchronization:

```typescript
class DayNightCycle {
  private cycleStartTime: number;
  private worldSeedOffset: number;

  update(delta: number): DayNightState {
    const elapsed = (Date.now() - this.cycleStartTime + this.worldSeedOffset) % CYCLE_TOTAL_MS;

    // Determine current phase
    if (elapsed < 60_000) {
      return this.getDayState(elapsed);
    } else if (elapsed < 75_000) {
      return this.getTransitionState(elapsed, dayPreset, eveningPreset);
    } else if (elapsed < 105_000) {
      return this.getEveningState(elapsed);
    } else if (elapsed < 120_000) {
      return this.getTransitionState(elapsed, eveningPreset, nightPreset);
    } else if (elapsed < 180_000) {
      return this.getNightState(elapsed);
    } else {
      return this.getTransitionState(elapsed, nightPreset, dayPreset);
    }
  }

  private getTransitionState(elapsed: number, from: SkyPreset, to: SkyPreset): DayNightState {
    const progress = (elapsed - phaseStart) / phaseDuration;
    const t = smoothstep(0, 1, progress);  // Smooth easing

    return {
      skyColor: lerpColor(from.skyColor, to.skyColor, t),
      sunDirection: lerpVector(from.sunDirection, to.sunDirection, t),
      ambientIntensity: lerp(from.ambientIntensity, to.ambientIntensity, t),
      musicWeights: { day: w1, evening: w2, night: w3 },  // Always sum to 1.0
      rainWeight: this.computeRainWeight(elapsed),
      nightBlend: t,  // 0 = day, 1 = night
    };
  }
}
```

### Sky Presets

```typescript
// SkyPresets.ts
interface SkyPreset {
  skyColor: Color;
  sunColor: Color;
  sunDirection: Vector3;
  ambientIntensity: number;
  fogColor: Color;
  fogDensity: number;
  musicDayWeight: number;
  musicEveningWeight: number;
  musicNightWeight: number;
  auroraWeight: number;      // 0-1, aurora visibility
  starOpacity: number;       // 0-1, star field visibility
}

const DAY_PRESET: SkyPreset = {
  skyColor: new Color(0x87CEEB),    // Sky blue
  sunColor: new Color(0xFFF5E1),    // Warm white
  sunDirection: new Vector3(0.5, 0.8, 0.3),
  ambientIntensity: 1.0,
  fogColor: new Color(0xC8E6F0),
  fogDensity: 0.005,
  musicDayWeight: 1.0,
  musicEveningWeight: 0,
  musicNightWeight: 0,
  auroraWeight: 0,
  starOpacity: 0,
};

const EVENING_PRESET: SkyPreset = {
  skyColor: new Color(0xFF6B35),    // Orange
  sunColor: new Color(0xFF4500),    // Orange-red
  sunDirection: new Vector3(0.8, 0.1, 0.5),
  ambientIntensity: 0.5,
  musicDayWeight: 0,
  musicEveningWeight: 1.0,
  musicNightWeight: 0,
  auroraWeight: 0.2,
  starOpacity: 0.3,
};

const NIGHT_PRESET: SkyPreset = {
  skyColor: new Color(0x0C1445),    // Dark blue
  sunColor: new Color(0x4466AA),    // Cool blue
  sunDirection: new Vector3(-0.3, -0.5, -0.1),
  ambientIntensity: 0.15,
  musicDayWeight: 0,
  musicEveningWeight: 0,
  musicNightWeight: 1.0,
  auroraWeight: 1.0,
  starOpacity: 1.0,
};
```

### Moon Progress Override

When the moon threat approaches (moon progress >= 0.75), the day/night cycle is forced to night blend regardless of the current phase:

```typescript
if (moonProgress >= 0.75) {
  state.nightBlend = Math.max(state.nightBlend, smoothstep(0.75, 1.0, moonProgress));
}
```

This ensures the endgame feels appropriately threatening — the sky darkens as the moon approaches, even if the normal cycle would be in daytime.

### Rain System

Rain weight is computed from two slow sine waves with irrational frequency ratios, seeded for client sync:

```typescript
computeRainWeight(elapsed: number): number {
  const seed = this.worldSeed;
  // Two sine waves with irrational ratios prevent periodic rain patterns
  const wave1 = Math.sin(elapsed * 0.0003 * seed + seed);
  const wave2 = Math.sin(elapsed * 0.0007 * seed + seed * 1.5);
  return Math.max(0, (wave1 + wave2) / 2);  // Normalized to [0, 1]
}
```

The irrational ratios (0.0003 and 0.0007 are approximations of ratios involving sqrt(2)) ensure that rain doesn't follow a repeating pattern — it feels organic and unpredictable.

## God Rays

```typescript
// GodRays.ts
class GodRays {
  // Volumetric light shafts from sun through clouds
  // Implemented as a full-screen quad with raymarching
  // Light source: sun position from DayNightCycle
  // Occlusion: Globe mesh (blocks light)

  update(delta: number): void {
    this.material.uniforms.sunPosition.value = this.sunDirection;
    this.material.uniforms.cameraPosition.value = this.camera.position;
    this.material.uniforms.intensity.value = this.getGodRayIntensity();
  }

  private getGodRayIntensity(): number {
    // Stronger at sunrise/sunset (low sun angle)
    // Weaker at noon (high sun angle)
    // Zero at night
    return Math.max(0, Math.cos(sunAngle)) * dayNight.sunWeight;
  }
}
```

## Aurora Borealis

```typescript
// Aurora.ts
class Aurora {
  // Ribbon geometry: curved strips in the northern sky
  // Shader: flowing, color-shifting bands (green/purple/teal)
  // Visibility: controlled by DayNightCycle.auroraWeight

  update(delta: number): void {
    const time = performance.now() * 0.001;
    this.material.uniforms.time.value = time;
    this.material.uniforms.auroraWeight.value = this.dayNightState.auroraWeight;
    // Flowing animation: offset UVs over time
    this.material.uniforms.flowOffset.value = time * 0.1;
  }
}
```

The aurora is only visible at night (auroraWeight = 1.0 during night phase). It appears as flowing, color-shifting ribbons in the northern sky, implemented as a custom shader on curved ribbon geometry.

## Starfield

```typescript
// Starfield.ts
class Starfield {
  // Points with varying brightness, twinkle animation
  // Visibility: controlled by DayNightCycle.starOpacity

  update(delta: number): void {
    const time = performance.now() * 0.001;
    this.material.uniforms.time.value = time;
    this.material.uniforms.opacity.value = this.dayNightState.starOpacity;
  }
}
```

Stars are implemented as Three.js Points with a custom shader that adds twinkle (sinusoidal brightness variation with randomized phase per star).

## Moon Threat

```typescript
// MoonThreat.ts
const MOON_START_DISTANCE = 35;  // Far away (invisible)
const MOON_END_DISTANCE = 7;     // Close (threatening)

class MoonThreat {
  private moonProgress = 0;  // 0.0 → 1.0
  private impactDuration: number;  // 5/7/10 min based on completed runs

  update(delta: number): void {
    this.moonProgress += delta / this.impactDuration;
    this.moonProgress = Math.min(this.moonProgress, 1.0);

    // Moon position: approaching from distance
    const dist = lerp(MOON_START_DISTANCE, MOON_END_DISTANCE, this.moonProgress);
    this.moon.position.copy(this.sunDirection).normalize().multiplyScalar(-dist);

    // Molten crack shader: patches of glowing cracks appear as moon gets closer
    this.material.uniforms.moonProgress.value = this.moonProgress;

    // Ember particles: 1500 points with fire trails
    this.emberSystem.update(delta, this.moonProgress);

    // Freeze approach when all five braziers lit
    if (this.allBraziersLit) {
      this.freezeApproachForever();
    }
  }

  getShakeTrauma(): number {
    // Camera shake intensity based on moon progress
    return this.moonProgress > 0.75 ? (this.moonProgress - 0.75) * 4 : 0;
  }
}
```

**Impact cinematic**: When moon progress reaches 1.0:
1. 3 shockwave rings expand outward (staggered timing)
2. 350 debris instanced meshes fly upward
3. 3 camera rocks tumble through the scene
4. VHS-style rewind effect (if enabled)

The **ember particle system** uses 1500 points with fire trails that intensify as the moon approaches. Each ember is a point sprite with a custom shader that simulates fire rising from the moon's surface.

## Meteor Shower

```typescript
// MeteorShower.ts — activated at moon progress 0.85-1.0
class MeteorShower {
  private meteors: Meteor[] = [];  // Pool of 4, max 3 active

  update(delta: number, playerPosition: Quaternion): void {
    if (moonProgress < 0.85) return;

    // Spawn new meteors relative to player position
    if (activeCount < 3 && Math.random() < spawnChance * delta) {
      this.spawnMeteor(playerPosition);
    }

    for (const meteor of this.meteors) {
      meteor.update(delta);
      if (meteor.hasImpacted) {
        this.onMeteorImpact(meteor.impactPosition);
      }
    }
  }
}

class Meteor {
  // Head: dodecahedron with lava crack shader
  // Trail: cone with fire/smoke shader
  // Target glow ring
  // Shockwave ring on impact
  // Impact dome
  // 300 spark points on impact

  update(delta: number): void {
    // Move toward target (globe surface point)
    this.position.lerp(this.targetPosition, this.speed * delta);

    if (this.position.distanceTo(this.targetPosition) < threshold) {
      this.impact();
    }
  }

  impact(): void {
    // Spawn shockwave ring
    // Spawn impact dome
    // Spawn 300 spark points
    // Trigger callback with distance to player
  }
}
```

**Post-build patches** (patch2.js, patch3.js) replace the basic MeshBasicMaterials with custom ShaderMaterials:

- **patch2.js**: Replaces meteor trail/flash/shockwave materials with custom GLSL shaders using noise
- **patch3.js**: Complete rewrite — adds dodecahedron head shader with lava cracks, 300-point particle spark system, fire dome shader, shockwave ring shader

See [Particle Systems](09-particle-systems.md) for ember, spark, and shockwave details.
See [NPC Systems](10-npc-systems.md) for balloon NPC interactions.
