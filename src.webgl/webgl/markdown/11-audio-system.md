# Tiny Skies -- Audio System

The audio system manages three music tracks (day, evening, night) with smooth crossfading, plus a large library of sound effects for gameplay events (collectibles, combat, UI, ambient loops).

Source: `tinyskies/client/src/audio/AudioManager.ts` — audio manager
Source: `tinyskies/client/src/game/Game.ts` — audio constants and calls

## Audio Manager

```typescript
// AudioManager.ts
const FADE_SPEED = 2.0;  // Volume crossfade rate

class AudioManager {
  private ctx: AudioContext;
  private masterGain: GainNode;
  private musicDay: AudioBufferSourceNode;
  private musicEvening: AudioBufferSourceNode;
  private musicNight: AudioBufferSourceNode;

  private targetVolumes = { day: 0, evening: 0, night: 0 };
  private currentVolumes = { day: 0, evening: 0, night: 0 };

  // Music crossfading: update called every frame
  update(weights: { day: number, evening: number, night: number }, delta: number): void {
    this.targetVolumes = weights;  // Weights always sum to 1.0

    // Smooth crossfade
    for (const track of ["day", "evening", "night"]) {
      const target = this.targetVolumes[track];
      const current = this.currentVolumes[track];
      const diff = target - current;

      // Exponential approach: current += diff * fadeSpeed * delta
      this.currentVolumes[track] += diff * FADE_SPEED * delta;

      // Apply volume
      this.musicNodes[track].gain.value = this.currentVolumes[track];
    }
  }
}
```

**Crossfading** uses exponential approach (`current += diff * rate * dt`) rather than instant volume changes. This ensures smooth transitions between music tracks when the day/night cycle shifts. The fade speed of 2.0 means the volume reaches 95% of its target in about 1.5 seconds.

## Sound Effect System

```typescript
// SFX playback with pitch variation
playSFX(name: string, options?: {
  playbackRate?: number;
  endFade?: number;   // Fraction of duration to fade out
  loop?: boolean;
}): void {
  const buffer = this.buffers[name];
  const source = this.ctx.createBufferSource();
  source.buffer = buffer;
  source.playbackRate.value = options.playbackRate ?? 1.0;

  // Fade-out: ramp gain down over last N% of duration
  if (options.endFade) {
    const fadeStart = buffer.duration * (1 - options.endFade);
    const gainNode = this.ctx.createGain();
    gainNode.gain.setValueAtTime(1, this.ctx.currentTime);
    gainNode.gain.linearRampToValueAtTime(0, this.ctx.currentTime + buffer.duration);
    source.connect(gainNode);
  }

  source.start();
}
```

## Looping SFX with Fade-Out

```typescript
// Looping ambient sounds (rain, crickets, ocean waves)
startLoopingSFX(name: string, maxVolume: number): void {
  const source = this.ctx.createBufferSource();
  source.buffer = this.buffers[name];
  source.loop = true;

  const gainNode = this.ctx.createGain();
  gainNode.gain.value = 0;
  this.currentLoops.set(name, { source, gainNode, maxVolume });

  source.connect(gainNode);
  gainNode.connect(this.masterGain);
  source.start();

  // Fade in to maxVolume
  gainNode.gain.linearRampToValueAtTime(maxVolume, this.ctx.currentTime + 1.0);
}

stopLoopingSFX(name: string): void {
  const loop = this.currentLoops.get(name);
  if (loop) {
    // Fade out first, then stop
    loop.gainNode.gain.linearRampToValueAtTime(0, this.ctx.currentTime + 0.5);
    setTimeout(() => loop.source.stop(), 500);
  }
}
```

## Audio Context Auto-Resume

```typescript
// UI click auto-resumes suspended audio context
// Browsers suspend AudioContext until user interaction
playUIClick(): void {
  if (this.ctx.state === "suspended") {
    this.ctx.resume();
  }
  this.playSFX("ui_click");
}
```

Browsers require user interaction to start audio playback. The first UI click (button press, menu interaction) automatically resumes the suspended AudioContext.

## Music Track Layout

| Track | Buffer Path | Phase |
|-------|------------|-------|
| Day music | `/music/day.mp3` | Daytime (60s) |
| Evening music | `/music/evening.mp3` | Golden hour (30s) |
| Night music | `/music/night.mp3` | Nighttime (60s) |
| End-times track 1 | `/music/end1.mp3` | Moon approaching |
| End-times track 2 | `/music/end2.mp3` | Moon impact imminent |

The end-times tracks are layered in addition to the day/evening/night tracks. When moon progress exceeds 0.75, the end-times tracks gradually fade in while the normal music fades out.

## SFX Library

| SFX | Usage | Volume |
|-----|-------|--------|
| `diamond_collect_1/2/3` | Diamond collection (random) | 0.3 |
| `explosion_1` | Plane destruction | 0.48 |
| `gremlin_1/2/3/4` | Gremlin hit (random) | 0.5 |
| `levelup_1/2/3` | Level up (random) | 0.42 |
| `rain_loop` | Rain ambient | 0.58 |
| `ocean_waves_1` | Ocean ambient | 0.32 |
| `void_1` | Cosmic void ambient | 0.30 |
| `dialogue_1/2/3/4` | NPC dialogue bed | 0.28 |
| `cheer_1/2` | Race completion | 0.10 |
| `speed_boost_1/2/3` | Boost activation | 0.10 |
| `box_collect_1/2/3` | Package pickup | 0.52 |
| `flame_dialogue_1/2/3` | Eternal flame dialogue | 0.46 |
| `impact_energy_1` | Shield impact | 0.58 |
| `moth_1/2/3` | Void moth hit | 0.50 |

## Playback Rate Variation

Some SFX use randomized playback rate for variety:

```typescript
// Explosion SFX — randomized pitch
// From patch.js post-build patch
playSFX("explosion_1", {
  playbackRate: 0.7 + Math.random() * 0.5,  // 0.7 to 1.2
});

// Dialogue gender: lower playback rate = deeper "male" voice
playSFX("dialogue_1", {
  playbackRate: isMale ? DIALOGUE_MALE_PLAYBACK_RATE : 1.0,  // 0.88 for male
});
```

The explosion SFX playback rate varies by 50% each time, making each explosion sound slightly different. The dialogue system uses a lower playback rate (0.88) for "male" NPCs, creating a deeper voice from the same audio file.

## Game-Specific Audio Cues

```typescript
// Game.ts — audio triggers for specific events

// Rain starts/stops
if (rainWeight > 0.5 && !raining) {
  this.audio.startLoopingSFX("rain_loop", RAIN_LOOP_MAX_VOL);
  raining = true;
} else if (rainWeight < 0.3 && raining) {
  this.audio.stopLoopingSFX("rain_loop");
  raining = false;
}

// Night crickets
if (nightBlend > 0.5) {
  const cricketVolume = nightBlend * CRICKETS_LOOP_MAX_VOL;
  this.audio.setLoopVolume("crickets", cricketVolume);
}

// Bird ambient (daytime)
if (dayWeight > 0.5) {
  this.audio.setLoopVolume("birds_loop", dayWeight * BIRDS_LOOP_MAX_VOL);
}

// Moon rumble (approaching impact)
if (moonProgress >= 0.75) {
  const rumbleVol = smoothstep(0.75, 1.0, moonProgress) * RUMBLE_MAX_VOL;
  this.audio.setLoopVolume("rumbling_1", rumbleVol);
}

// Ocean waves (boat mode)
if (player.vehicleType === "boat") {
  this.audio.setLoopVolume("ocean_waves_1", OCEAN_WAVES_LOOP_VOL);
}

// Void ambient (cosmic void mode)
if (player.inCosmicVoid) {
  this.audio.startLoopingSFX("void_1", VOID_MUSIC_LOOP_MAX_VOL);
}
```

See [Atmospheric VFX](08-atmospheric-vfx.md) for day/night cycle music weights.
See [Vehicles](04-vehicles.md) for boat-specific ocean sounds.
