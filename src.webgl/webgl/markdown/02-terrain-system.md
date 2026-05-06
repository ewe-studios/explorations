# Tiny Skies -- Terrain System

The terrain system generates a procedurally-created spherical world using 3D simplex noise, multiple octaves of fractal noise, and ocean backbone masking. The entire globe is a 256-segment sphere with vertex displacement, vertex colors, and instanced props (trees, rocks, buildings) placed deterministically from the world seed.

Source: `tinyskies/client/src/game/SimplexNoise.ts` — seeded 3D noise
Source: `tinyskies/client/src/game/TerrainPresets.ts` — 4 terrain presets
Source: `tinyskies/client/src/game/TerrainSurface.ts` — displacement algorithm
Source: `tinyskies/client/src/game/Globe.ts` — globe mesh construction (~5800 lines)

## Simplex Noise Implementation

```typescript
// SimplexNoise.ts — Park-Miller LCG for seeding
function seededRandom(seed: number): () => number {
  let s = seed;
  return () => {
    s = (s * 48271) % 2147483647;
    return s / 2147483647;
  };
}

// Permutation table generation from seed
class SimplexNoise {
  private perm: Uint8Array;

  constructor(seed: number) {
    const rng = seededRandom(seed);
    const p = new Uint8Array(256);
    for (let i = 0; i < 256; i++) p[i] = i;
    // Fisher-Yates shuffle
    for (let i = 255; i > 0; i--) {
      const j = Math.floor(rng() * (i + 1));
      [p[i], p[j]] = [p[j], p[i]];
    }
    this.perm = new Uint8Array(512);
    this.perm.set(p);
    this.perm.set(p, 256);
  }

  noise3D(x: number, y: number, z: number): number {
    // Stefan Gustavson 3D simplex noise
    // Skew input, find simplex cell, calculate contributions
    // from 4 corners weighted by gradient vectors
    return (contrib0 + contrib1 + contrib2 + contrib3) * 32;
  }
}
```

The **Park-Miller LCG** (`s = s * 48271 mod 2147483647`) is a well-tested pseudo-random number generator with a full period of 2^31 - 2. It ensures the same seed always produces the same permutation table, which means the same terrain.

**3D simplex noise** is preferred over Perlin noise because:
- Lower computational complexity (4 corners vs 8 for trilinear interpolation)
- Better frequency spectrum (no directional artifacts)
- Continuous gradients (important for smooth terrain)

## Multi-Octave Fractal Sum

```typescript
// TerrainSurface.ts — noise evaluation
function noiseAt(x: number, y: number, z: number,
                octaves: number, lacunarity: number,
                persistence: number, scale: number): number {
  let value = 0;
  let amplitude = 1;
  let frequency = 1;
  let maxAmplitude = 0;

  for (let i = 0; i < octaves; i++) {
    value += simplex.noise3D(
      x * scale * frequency,
      y * scale * frequency,
      z * scale * frequency
    ) * amplitude;
    maxAmplitude += amplitude;
    amplitude *= persistence;
    frequency *= lacunarity;
  }
  return value / maxAmplitude;  // Normalize to [-1, 1]
}
```

- **Octaves**: Number of noise layers (more = more detail)
- **Lacunarity**: Frequency multiplier between octaves (2.0 means each octave is 2x finer)
- **Persistence**: Amplitude multiplier between octaves (0.5 means each octave contributes half as much)
- **Scale**: Base frequency of the noise (higher = smaller features)

## Terrain Presets

```typescript
// TerrainPresets.ts
const PRESETS: Record<string, TerrainPreset> = {
  default: {
    scale: 0.8, octaves: 6, lacunarity: 2.0, persistence: 0.5,
    threshold: 0.05, oceanBackboneWidth: 0.25, oceanBackboneStrength: 0.3,
  },
  archipelago: {
    scale: 0.6, octaves: 5, lacunarity: 2.2, persistence: 0.45,
    threshold: 0.15, oceanBackboneWidth: 0.4, oceanBackboneStrength: 0.5,
  },
  pangaea: {
    scale: 0.5, octaves: 7, lacunarity: 1.8, persistence: 0.55,
    threshold: -0.05, oceanBackboneWidth: 0.15, oceanBackboneStrength: 0.15,
  },
  waterworld: {
    scale: 0.7, octaves: 4, lacunarity: 2.0, persistence: 0.4,
    threshold: 0.25, oceanBackboneWidth: 0.5, oceanBackboneStrength: 0.6,
  },
};
```

| Preset | Result |
|--------|--------|
| **default** | Balanced island with beaches, hills, and mountains |
| **archipelago** | Multiple small islands separated by deep ocean |
| **pangaea** | One massive continent surrounded by ocean |
| **waterworld** | Mostly ocean with scattered small islands |

## Ocean Backbone Mask

The ocean backbone creates natural ocean basins by carving troughs along random great circles:

```typescript
// Create 3 random axes as ocean trough directions
const axes = [
  randomUnitVector(rng),
  randomUnitVector(rng),
  randomUnitVector(rng),
];

// For each vertex, check distance to nearest axis
function getOceanBackbone(x: number, y: number, z: number): number {
  let minDist = 1;
  for (const axis of axes) {
    const dist = Math.abs(dot(normalize([x,y,z]), axis));
    minDist = Math.min(minDist, dist);
  }
  // Smooth mask: 1.0 at axis, 0.0 outside width
  return smoothstep(width, 0, minDist);
}
```

This produces realistic ocean basins — the noise alone would create random hills and valleys everywhere, but the backbone mask pushes down vertices near the random axes to create ocean trenches.

## Ocean Region Identification

A 96x48 grid is evaluated to identify the main ocean component using BFS flood-fill:

```typescript
// Identify the largest connected ocean region
const grid = buildHeightGrid(resolution);
const visited = new Set();
let largestRegion = [];

for (const cell of grid) {
  if (cell.isOcean && !visited.has(cell)) {
    const region = bfsFloodFill(cell, visited, (c) => c.isOcean);
    if (region.size > largestRegion.size) {
      largestRegion = region;
    }
  }
}
```

This identifies which ocean basin is the "main" ocean, used for prop placement (boats spawn on the main ocean, not isolated lakes).

## Surface Displacement

```typescript
// TerrainSurface.ts
const LAND_HEIGHT = 0.0;
const OCEAN_DEPTH = 0.15;
const MOUNTAIN_HEIGHT = 0.35;
const PROP_TERRAIN_SINK = 0.018;

function surfaceDisplacementAt(x: number, y: number, z: number): Vector3 {
  const terrainVal = noiseAt(x, y, z, preset);

  if (terrainVal > threshold) {
    // Land: elevated above sphere surface
    const elevation = terrainVal - threshold;
    const jagged = 1.0 + ruggedNoise(x, y, z) * 0.3;
    const height = LAND_HEIGHT + elevation * MOUNTAIN_HEIGHT * jagged;
    return normalize([x, y, z]).multiplyScalar(globeRadius + height);
  } else {
    // Ocean: pushed below sphere surface
    const depth = (threshold - terrainVal) / threshold;
    return normalize([x, y, z]).multiplyScalar(globeRadius - OCEAN_DEPTH * depth);
  }
}
```

The displacement is applied to the 256-segment sphere vertices. Land vertices are pushed outward, ocean vertices are pushed inward. The `ruggedNoise` adds micro-variation to mountain peaks, making them look more jagged.

## Vertex Coloring

The terrain uses a 4-color scheme with smooth transitions:

| Zone | Color | Condition |
|------|-------|-----------|
| **Deep ocean** | Dark blue | `terrainVal < threshold - 0.3` |
| **Shallow ocean** | Light blue | `terrainVal < threshold` |
| **Lowland** | Green | `terrainVal > threshold, elevation < 0.5` |
| **Highland** | Brown | `elevation > 0.5` |
| **Snow** | White | `elevation > 0.85` |
| **Warm patch** | Warm tint | Rugged noise overlay on lowland green |

## Globe.ts Construction

The Globe class builds the entire 3D scene (~5800 lines):

### Surface Mesh
```typescript
const surfaceGeo = new SphereGeometry(globeRadius, 256, 256);
const positions = surfaceGeo.getAttribute("position");

for (let i = 0; i < positions.count; i++) {
  const x = positions.getX(i);
  const y = positions.getY(i);
  const z = positions.getZ(i);
  const displaced = surfaceDisplacementAt(x, y, z, noise, preset);
  positions.setXYZ(i, displaced.x, displaced.y, displaced.z);
  // Set vertex color based on terrain zone
  colors.setXYZ(i, r, g, b);
}
surfaceGeo.computeVertexNormals();
```

### Ocean Shader
```typescript
const oceanMat = new MeshPhongMaterial({
  color: 0x1a5276,
  transparent: true,
  opacity: 0.85,
});

// Patch the shader via onBeforeCompile
oceanMat.onBeforeCompile = (shader) => {
  shader.fragmentShader = shader.fragmentShader
    .replace("#include <color_fragment>", /* foam + sparkle GLSL */)
    .replace("#include <lights_phong_fragment>", /* rim light GLSL */);

  // Animated foam lines
  // Coastline contour foam via depth-based scrolling lines
  // Sparkle highlights that move with time
  // Rim lighting for edge glow

  shader.uniforms.time = { value: 0 };
  this.oceanShader = shader;
};
```

### Instanced Props

| Prop | Count | Technique |
|------|-------|-----------|
| Trees | 10,000 | `InstancedMesh` with teardrop geometry + vertex sway |
| Coconut trees | 270 clusters | Trunk (cylinder) + fronds (cones) + coconuts (spheres) |
| Rocks | 400 | `InstancedMesh` with dodecahedron geometry |
| Houses | 160-320 | Procedural stone walls + wood beams + thatch roof |
| Steam particles | Per hot spring | `InstancedMesh` with upward animation |
| Mushrooms | Per cluster | `InstancedMesh` with scale variation |
| Butterflies | Per cluster | `InstancedMesh` with orbital paths |
| Debris (impact) | 350 | `InstancedMesh` with camera rocks + upward velocity |

**Tree sway** is implemented in the vertex shader:

```glsl
// Tree sway — time-driven displacement
uniform float time;
uniform float windSpeed;

// Read instance matrix to get world position
vec4 worldPos = instanceMatrix * vec4(position, 1.0);
float heightFactor = smoothstep(0.0, 5.0, worldPos.y);
float sway = sin(time * windSpeed + worldPos.x * 0.5) * heightFactor * 0.15;
worldPos.x += sway;
```

### Prop Placement Algorithm

Props are placed by iterating over the sphere surface at random positions seeded from the world seed:

```typescript
// Place trees at land positions above a certain height threshold
for (let i = 0; i < targetTreeCount; i++) {
  const q = seededRandomSurfacePosition(rng);
  const altitude = sampleTerrainHeightAt(q);

  if (altitude > landThreshold && altitude < mountainThreshold) {
    // Place tree at this position
    const matrix = buildInstanceMatrix(q, scale, rotation);
    treeInstances.setMatrixAt(treeIndex++, matrix);
  }
}
```

This ensures trees grow on hills and plains but not on peaks or in the ocean.

## Moonstone Ruins

Two giant stone ring halves are placed at inland lowland sites. They have a raise/float/lower cycle triggered by player proximity:

```typescript
// Moonstone animation cycle
enum RuinState { "raise", "float", "lower" }
const MOONSTONE_RAISE_MS = 5000;
const MOONSTONE_FLOAT_MS = 15000;
const MOONSTONE_LOWER_MS = 5000;
```

The animation includes wobble (sinusoidal rotation around globe normal), dust particles (points that rise from the base), and rim-lit materials that glow more brightly as the ruins rise.

## Lighthouses, Windmills, Observatories

| Structure | Count | Animation |
|-----------|-------|-----------|
| Lighthouses | 3 | Rotating beam shader (cone of light sweeping around) |
| Windmills | 5 | Spinning blade pivots (rotation around local axis) |
| Observatories | 3 | Dome slit rotation + telescope finder animation |

See [Architecture](01-architecture.md) for the globe in the scene hierarchy.
See [Vehicles](04-vehicles.md) for how boats navigate the ocean.
