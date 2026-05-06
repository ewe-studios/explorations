# Tiny Skies -- Production Patterns

Production patterns in Tiny Skies cover mobile optimization, performance considerations, cross-platform compatibility, and deployment best practices.

## Mobile Optimization

### Touch Controls

```typescript
// TouchControls.ts
class TouchControls {
  // Virtual joystick on left half of screen
  // Action buttons on right half
  // Touch ID tracking for multi-touch

  // Joystick: drag anywhere on left half to control
  // Buttons: tap fire/elevate on right half
  // Supports simultaneous joystick + button input
}
```

### Responsive Layout

```css
/* HUD.ts — responsive breakpoints */
@media (max-width: 768px) {
  .hud-container {
    font-size: 12px;  /* Smaller text on mobile */
  }
  .brazier-icons {
    width: 20px;  /* Smaller icons */
  }
}

@media (max-width: 480px) {
  .hud-container {
    font-size: 10px;  /* Even smaller on narrow screens */
  }
}
```

### Mobile Detection

```typescript
// isMobile.ts
function isMobile(): boolean {
  return /Android|iPhone|iPad|iPod|webOS|BlackBerry|IEMobile|Opera Mini/i.test(
    navigator.userAgent
  ) || (navigator.maxTouchPoints > 0 && window.innerWidth < 1024);
}
```

The game disables features like contrails and god rays on mobile devices to maintain framerate.

## Performance Optimizations

### Instanced Rendering

All high-count props use `InstancedMesh` for single draw calls:

| Prop | Instance Count | Draw Calls Without Instancing | With Instancing |
|------|---------------|-------------------------------|-----------------|
| Trees | 10,000 | 10,000 | 1 |
| Rocks | 400 | 400 | 1 |
| Steam particles | ~100 | ~100 | 1 |
| Debris | 350 | 350 | 1 |
| Ember particles | 1500 | 1500 | 1 |

```typescript
// InstancedMesh setup
const treeGeo = new TeardropGeometry();
const treeMat = new MeshPhongMaterial({ color: 0x228B22 });
const treeMesh = new InstancedMesh(treeGeo, treeMat, MAX_TREES);

for (let i = 0; i < treeCount; i++) {
  const matrix = buildInstanceMatrix(position, scale, rotation);
  treeMesh.setMatrixAt(i, matrix);
}
treeMesh.count = treeCount;
treeMesh.instanceMatrix.needsUpdate = true;
```

### Object Pooling

Projectiles, particles, and VFX objects are pooled to avoid garbage collection pauses:

```typescript
class ProjectilePool {
  private pool: PaintballProjectile[] = [];
  private active: PaintballProjectile[] = [];

  acquire(): PaintballProjectile {
    return this.pool.pop() ?? new PaintballProjectile();
  }

  release(projectile: PaintballProjectile): void {
    projectile.reset();
    this.pool.push(projectile);
  }
}
```

### Frame Rate Capping

```typescript
// Game.ts
const tick = (): void => {
  const delta = Math.min(this.clock.getDelta(), 0.1);  // Cap at 100ms

  // If the tab is backgrounded, delta can be huge (seconds)
  // Capping prevents physics explosions on tab switch-back
  ...
};
```

### Visibility API

```typescript
// Pause game logic when tab is hidden
document.addEventListener("visibilitychange", () => {
  if (document.hidden) {
    this.pause();
  } else {
    this.resume();
  }
});
```

### Shader Complexity Management

Complex shaders are disabled on lower-end devices:

```typescript
// features.ts
const isLowEndDevice = () => {
  const renderer = gl.getParameter(gl.RENDERER);
  return renderer.includes("Mali-400") || renderer.includes("Adreno 300");
};

if (isLowEndDevice()) {
  // Disable god rays, aurora, tree sway
  config.godRays = false;
  config.aurora = false;
  config.treeSway = false;
}
```

## Cross-Platform Compatibility

### Browser Support

- WebGL 2.0 required (Chrome 56+, Firefox 51+, Safari 15+)
- Fallback message shown for unsupported browsers
- Vite handles module bundling and browser compatibility

### Input Abstraction

```typescript
// Input is abstracted through FlightControls interface
// Both desktop (keyboard) and mobile (touch) implementations
interface InputProvider {
  update(delta: number): InputState;
}

class DesktopControls implements InputProvider { ... }
class MobileControls implements InputProvider { ... }

// Game uses whichever is appropriate
this.controls = isMobile() ? new MobileControls() : new DesktopControls();
```

## Networking Reliability

### Reconnection Handling

```typescript
// SocketClient.ts
const socket = io(SERVER_URL, {
  transports: ["websocket"],
  reconnection: true,
  reconnectionAttempts: 10,
  reconnectionDelay: 1000,
});

socket.on("reconnect", (attempt) => {
  console.log(`Reconnected after ${attempt} attempts`);
  // Re-join world
  this.emit("world:join", { worldSlug: this.worldSlug });
});

socket.on("reconnect_failed", () => {
  // Show "Connection lost" UI
  this.showReconnectFailed();
});
```

### State Reconciliation

When reconnecting, the client receives the full world state (all player positions, flags, etc.) and interpolates to the current state smoothly:

```typescript
socket.on("world:state", (state) => {
  // Snap local state to server state
  for (const player of state.players) {
    this.remotePlanes.updatePlayer(player);
  }
});
```

## Security

### Input Validation

Server validates all incoming data:

```typescript
// Paintball upgrades — clamp to prevent cheating
function clampUpgrades(upgrades: PaintballUpgrades): PaintballUpgrades {
  return {
    speed: Math.min(upgrades.speed, MAX_SPEED_UPGRADE),
    range: Math.min(upgrades.range, MAX_RANGE_UPGRADE),
  };
}

// World name — sanitize and length limit
function validateWorldName(name: string): string | null {
  if (name.length > 50) return null;
  return name.replace(/[<>]/g, "");  // Strip HTML
}
```

### CORS

```typescript
// Only allow known Vercel URLs
const cors = {
  origin: [
    "https://tinyskies.vercel.app",
    `https://${process.env.VERCEL_URL}`,
  ],
  methods: ["GET", "POST"],
};
```

## Monitoring

### Vercel Analytics

```typescript
// main.ts
import { inject } from "@vercel/analytics";

inject({
  mode: import.meta.env.PROD ? "production" : "development",
});
```

Tracks page views, custom events, and performance metrics in production.

### Server Dashboard

```typescript
// Admin dashboard at /dashboard
// Shows:
// - Recent world saves
// - Recent quest completions
// - Event totals (by type)
// - Today's totals
// - Vehicle usage counts
```

## Feature Flags

```typescript
// features.ts
export const CAMPSITE_HOME_ENABLED = false;  // Disabled feature flag
```

Feature flags allow gradual rollout and testing. The campsite home feature is implemented but disabled via flag — it can be enabled without a code change by updating the flag value.

## Build Pipeline

```
npm install
    ↓
npm run build (Vite)
    ↓
patch.js (SFX randomization)
    ↓
patch2.js (Meteor materials)
    ↓
patch3.js (Meteor rewrite)
    ↓
Vercel deployment
```

The post-build patches are the most unusual part of the pipeline. They modify the compiled JavaScript output, which means they must be updated whenever the TypeScript source they target changes.

See [Deployment](16-deployment.md) for Docker and CI configuration.
See [Three.js Patches](15-threejs-patches.md) for the patch mechanisms.
