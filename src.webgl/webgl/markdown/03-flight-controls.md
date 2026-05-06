# Tiny Skies -- Flight Controls and Physics

Flight controls handle WASD/arrow input on desktop and virtual joystick on mobile, translating player input into vehicle-specific movement on a spherical surface. All physics operates in spherical space using quaternion-based rotation math.

Source: `tinyskies/client/src/game/FlightControls.ts` — desktop input
Source: `tinyskies/client/src/game/TouchControls.ts` — mobile virtual joystick
Source: `tinyskies/client/src/game/CameraRig.ts` — chase camera
Source: `tinyskies/client/src/game/SphericalMath.ts` — spherical geometry primitives

## Spherical Math Primitives

All movement, orientation, and collision detection use these core functions:

```typescript
// SphericalMath.ts

// Map +Y direction to a surface normal direction
function quaternionFromSurfaceNormal(nx: number, ny: number, nz: number): Quaternion;

// Uniform random position on sphere surface + random heading
function randomSpawnQuaternionAndHeading(seed: number): { q: Quaternion, heading: number };

// Returns {up, north, east} tangent vectors at a surface position
function tangentFrame(qPosition: Quaternion): { up: Vector3, north: Vector3, east: Vector3 };

// Move along a great-circle arc
function moveOnSphere(qPosition: Quaternion, heading: number, arcAngle: number): Quaternion;

// Convert spherical position to world XYZ
function cartesianFromSpherical(qPosition: Quaternion, altitude: number, globeRadius: number): Vector3;

// Build world ray from plane nose (aligned with server hit test)
function paintballRayFromPlaneState(position: Quaternion, heading: number, pitch: number, ...): { origin: Vector3, direction: Vector3 };

// Full 4x4 world matrix for vehicle
function buildPlaneMatrix(qPosition: Quaternion, heading: number, pitch: number, bankAngle: number, globeRadius: number): Matrix4;

// Build flat-plane matrix for cosmic void (u/v coordinates)
function buildCarpetMatrixVoidPlane(u: number, v: number, heading: number): Matrix4;

// Build boat matrix (minimal pitch/roll)
function buildBoatMatrix(qPosition: Quaternion, heading: number, globeRadius: number): Matrix4;

// Shortest-path angle interpolation with wrapping
function lerpAngle(a: number, b: number, t: number): number;

// Interpolate between player states
function slerpPlayerState(a: PlayerState, b: PlayerState, t: number): PlayerState;

// Predict player position forward by elapsed time
function deadReckon(state: PlayerState, elapsed: number, globeRadius: number): PlayerState;
```

## Flight Controls (Desktop)

```typescript
// FlightControls.ts
interface InputState {
  forward: number;   // W/S: -1 to 1
  turn: number;      // A/D: -1 to 1
  elevate: number;   // ArrowUp/ArrowDown: -1 to 1
  interact: boolean; // F key
  fire: boolean;     // Space
}

class FlightControls {
  private forwardPressed = 0;
  private turnPressed = 0;
  private elevating = false;

  update(delta: number): InputState {
    // One-shot consumption: fire and interact are true only on the frame pressed
    // Forward and turn are continuous values (-1 to 1)
    // This prevents double-firing on key hold
    return {
      forward: this.forwardPressed,
      turn: this.turnPressed,
      elevate: this.elevating ? 1 : 0,
      interact: this.consumeInteract(),
      fire: this.consumeFire(),
    };
  }
}
```

**One-shot consumption** is critical for the fire and interact keys. The player presses Space once, and the fire event is consumed on the next frame — holding Space does not fire continuously. This matches the server-side paintball cooldown model.

## Touch Controls (Mobile)

```typescript
// TouchControls.ts
class TouchControls {
  private leftJoystick: { x: number, y: number, touchId: number };
  private elevateButton: { pressed: boolean, touchId: number };
  private fireButton: { pressed: boolean, touchId: number };

  // Virtual joystick on left side of screen
  // Action + elevate buttons on right side
  // Touch ID tracking for multi-touch (move + fire simultaneously)

  handleTouchStart(event: TouchEvent): void {
    for (const touch of event.changedTouches) {
      if (touch.clientX < window.innerWidth / 2) {
        // Left side: joystick tracking
        this.leftJoystick.touchId = touch.identifier;
        this.leftJoystick.startX = touch.clientX;
        this.leftJoystick.startY = touch.clientY;
      } else {
        // Right side: button hit testing
        this.hitTestButtons(touch);
      }
    }
  }
}
```

The touch layout adapts to the current vehicle (carpet shows portal buttons, boat hides elevate). Cosmic void mode hides action buttons since combat is disabled.

## Vehicle Physics

### Plane Physics

```typescript
// Plane.ts
const CRUISE_SPEED = 1.725;
const MAX_SPEED = 0.8;
const BOOST_MULTIPLIER = 1.3;

update(delta: number, input: InputState): void {
  // Speed control
  let speed = CRUISE_SPEED + input.forward * (MAX_SPEED - CRUISE_SPEED);
  if (this.boosting) speed *= BOOST_MULTIPLIER;

  // Heading change: turn input * bank angle
  this.heading += input.turn * this.bankRate * delta;

  // Elevation blend: climb/cruise/descend
  const targetAlt = this.elevating ? this.climbAlt : this.cruiseAlt;
  this.altitude = lerp(this.altitude, targetAlt, this.elevationBlendSpeed * delta);

  // Barrel roll during boost (visual only)
  if (this.boosting) {
    this.rollAngle += this.rollSpeed * delta;
  }

  // Gremlin slow debuff (reduces speed after hit)
  if (this.slowDebuffTimer > 0) {
    speed *= 0.6;
  }

  // Move on sphere
  this.position = moveOnSphere(this.position, this.heading, speed * delta);
}
```

**Banking** is a visual effect — the plane rolls into turns, making flight feel more natural. The roll angle is proportional to the turn input and fades back to zero when the player stops turning.

### Boat Physics

```typescript
// Boat.ts
const CRUISE_SPEED = 0.22;
const MAX_SPEED = 0.42;
const DIAMOND_BOOST_SPEED = 0.58;

update(delta: number, input: InputState): void {
  // Slower than plane
  let speed = CRUISE_SPEED + input.forward * (MAX_SPEED - CRUISE_SPEED);
  if (this.diamondBoosted) speed = DIAMOND_BOOST_SPEED;

  // Constrained to ocean: land collision blocks
  const terrainHeight = sampleTerrainHeightAt(this.position);
  if (terrainHeight > 0) {
    // Push boat back toward ocean
    speed *= 0.3;
    this.altitude = Math.max(this.altitude, 0.01);
  }

  // Bob animation: sinusoidal pitch/roll/height
  const time = performance.now() * 0.001;
  this.bobHeight = Math.sin(time * 1.2) * 0.02;
  this.bobPitch = Math.sin(time * 0.8) * 0.01;
  this.bobRoll = Math.sin(time * 1.5) * 0.008;

  // Turn input smoothing: boats turn slower than planes
  this.heading = lerpAngle(this.heading, this.targetHeading, 0.3 * delta);
}
```

### Carpet Physics

```typescript
// Carpet.ts
const CRUISE_SPEED = 0.6;
const MAX_SPEED = 0.78;

update(delta: number, input: InputState): void {
  // Hover altitude tracking: follow terrain height with offset
  const terrainHeight = sampleTerrainHeightAt(this.position);
  const targetAlt = Math.max(terrainHeight + this.hoverOffset, this.minAltitude);
  this.altitude = lerp(this.altitude, targetAlt, this.hoverBlendSpeed * delta);

  // Drift system: velocity heading decouples from facing heading
  // on sharp turns at speed
  if (Math.abs(input.turn) > 0.7 && this.speed > 0.5) {
    this.driftVelocity += input.turn * this.driftRate * delta;
    this.driftVelocity *= 0.95;  // Damping
  }
  const velocityHeading = this.heading + this.driftVelocity;

  // Cliff glide: when carpet is near terrain, increase altitude smoothly
  if (terrainHeight > this.altitude - 0.05) {
    this.altitude = lerp(this.altitude, terrainHeight + 0.1, 0.3 * delta);
  }

  // Barrel roll during boost
  // Tassel curl animation
  // Capybara bob

  // Cosmic void flat-plane mode (u/v coordinates)
  if (this.inCosmicVoid) {
    this.position = moveOnVoidPlane(this.u, this.v, this.heading, speed * delta);
  }
}
```

The **drift system** is the carpet's signature mechanic. When the player turns sharply at speed, the carpet's velocity vector decouples from its facing direction, creating a drifting/sliding effect. The drift velocity accumulates with turn input and damps over time, producing smooth drift curves.

## Camera Rig

```typescript
// CameraRig.ts
const POSITION_SMOOTH = 3.0;
const SHAKE_DECAY = 0.95;

update(delta: number, vehicle: Plane | Boat | Carpet): void {
  // Target: behind and above vehicle
  const targetPos = this.getChasePosition(vehicle);

  // Exponential smoothing
  this.position.lerp(targetPos, 1 - Math.exp(-POSITION_SMOOTH * delta));

  // Speed-based zoom: faster = further back
  const zoomFactor = lerp(1.0, cameraSpeedZoom, vehicle.speed / maxSpeed);
  this.position = vehicle.position + (this.position - vehicle.position) * zoomFactor;

  // FOV boost at speed
  this.camera.fov = lerp(defaultFOV, cameraFovBoost, vehicle.speed / maxSpeed);

  // Bank-induced tilt: camera rolls with plane banking
  this.camera.rotation.z = vehicle.bankAngle * cameraTiltScale;

  // Camera shake: instant (decay envelope) + persistent (trauma)
  if (this.shakeIntensity > 0) {
    this.position += randomVector() * this.shakeIntensity * SHAKE_DECAY;
    this.shakeIntensity *= SHAKE_DECAY;
  }
  if (this.shakeTrauma > 0) {
    this.position += randomVector() * sin(time * shakeFrequency) * this.shakeTrauma;
  }

  // Void flat-plane chase mode: higher, tighter framing
  if (vehicle.inCosmicVoid) {
    const voidTarget = this.getVoidChasePosition(vehicle);
    this.cameraOffset = lerp(this.cameraOffset, voidTarget, VOID_CAMERA_BLEND_SPEED * delta);
  }

  this.camera.lookAt(vehicle.position);
  this.camera.updateMatrixWorld();
}
```

**Exponential smoothing** (`1 - e^(-rate * dt)`) produces frame-rate-independent smoothing. At 60fps (dt=16ms) with rate=3.0, the smoothing factor is ~4.7%, which means the camera catches up to its target in about 0.3 seconds.

**Close-distance damping** prevents the camera from going underground when the vehicle is close to the surface. When the chase position would clip inside the terrain, the camera is pushed outward along the surface normal.

See [Vehicles](04-vehicles.md) for vehicle-specific physics parameters.
See [Multiplayer Networking](05-multiplayer-networking.md) for dead reckoning and interpolation.
