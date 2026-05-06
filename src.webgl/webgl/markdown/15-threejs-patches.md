# Tiny Skies -- Three.js Patches

Three.js's built-in materials are extensively patched via `onBeforeCompile` to inject custom GLSL shaders. Additionally, post-build JavaScript patches modify the compiled output for performance and visual enhancements that would be cumbersome to implement in the TypeScript source.

Source: `tinyskies/client/src/game/Globe.ts` — ocean/atmosphere shader patches
Source: `tinyskies/patch.js` — SFX randomization, camera shake
Source: `tinyskies/patch2.js` — Meteor shader materials
Source: `tinyskies/patch3.js` — Meteor complete rewrite

## onBeforeCompile Pattern

Three.js materials support an `onBeforeCompile` hook that intercepts the shader compilation process:

```typescript
const material = new MeshPhongMaterial({ color: 0xffffff });

material.onBeforeCompile = (shader: Shader) => {
  // Modify GLSL source before compilation
  shader.vertexShader = shader.vertexShader.replace(
    "#include <begin_vertex>",
    `#include <begin_vertex>
     // Custom vertex displacement
     transformed.y += sin(position.x * 10.0) * 0.1;`
  );

  shader.fragmentShader = shader.fragmentShader.replace(
    "#include <color_fragment>",
    `#include <color_fragment>
     // Custom color blending
     diffuseColor.rgb *= rimLight;`
  );

  // Add custom uniforms
  shader.uniforms.time = { value: 0 };
  shader.uniforms.rimColor = { value: new Color(0xff8800) };

  // Store reference for per-frame updates
  this.shader = shader;
};
```

This pattern allows custom effects to be layered on top of Three.js's standard material pipeline (lighting, shadows, fog) without writing a full custom shader from scratch.

## Ocean Shader Patch

The ocean mesh uses `MeshPhongMaterial` patched with custom effects:

```glsl
// Patches applied to ocean fragment shader:

// 1. Animated foam lines
// Scrolling white lines at the coastline
float foam = step(foamThreshold, sin(position.x * foamFrequency + time * foamSpeed));

// 2. Coastline contour foam
// Depth-based scrolling lines near shore
float depthFoam = smoothstep(shallowDepth, deepDepth, -position.y);
foam += depthFoam * contourFoam;

// 3. Sparkle highlights
// Moving bright spots on water surface
float sparkle = pow(max(0.0, dot(normal, lightDir)), sparklePower);
sparkle *= step(sparkleThreshold, sin(time * sparkleSpeed + position.x * 50.0));

// 4. Rim lighting
// Edge glow for ocean surface
float rim = 1.0 - max(0.0, dot(normal, viewDir));
rimColor += pow(rim, rimPower) * rimIntensity;
```

The ocean shader produces:
- **Foam lines**: Animated white lines that scroll across the water surface, denser near shorelines
- **Sparkle highlights**: Moving bright spots that simulate sunlight reflecting off waves
- **Rim lighting**: Edge glow that makes the ocean surface visible from the side

## Atmosphere Shader Patch

```glsl
// Rim-fresnel glow for atmosphere
float fresnel = pow(1.0 - max(0.0, dot(normal, viewDir)), atmospherePower);
atmosphereColor = mix(skyColor, atmosphereColor, fresnel * atmosphereIntensity);
```

The atmosphere uses a fresnel-based rim glow — the atmosphere appears more opaque at the edges (where the view angle is grazing) and more transparent when looking directly down.

## Tree Sway Vertex Shader

```glsl
// Tree sway — displacement in vertex shader
uniform float time;
uniform float windSpeed;

// Read instance matrix to get world position
vec4 worldPos = instanceMatrix * vec4(position, 1.0);
float heightFactor = smoothstep(0.0, 5.0, worldPos.y);

// Sway proportional to height
float sway = sin(time * windSpeed + worldPos.x * 0.5) * heightFactor * 0.15;
worldPos.x += sway;

// Transform back to local space
transformed = (inverse(instanceMatrix) * worldPos).xyz;
```

Trees sway in the wind, with taller trees swaying more than shorter ones. The wind speed and direction are driven by the `time` uniform, which is updated every frame.

## Moon Shader Patch

```glsl
// Molten crack shader — patches appear as moon approaches
uniform float moonProgress;  // 0.0 → 1.0

// Crack pattern: noise-based threshold
float crack = step(crackThreshold, noise(position * crackScale));
crack *= smoothstep(0.0, 0.5, moonProgress);  // More cracks as moon approaches

// Molten glow
vec3 moltenColor = vec3(1.0, 0.3, 0.0);  // Orange-red
color = mix(baseColor, moltenColor, crack * moonProgress);
```

The moon's surface develops glowing cracks as it approaches, creating a threatening visual. The crack pattern is driven by noise, with the threshold decreasing as moon progress increases.

## Post-Build Patches

### patch.js — SFX Randomization and Shake

```javascript
// patch.js
// Post-build patch to Game.ts:
// 1. Randomize explosion SFX playback rate
// 2. Increase camera shake intensity

const gameTs = readFileSync("dist/Game.js", "utf8");

// Replace hardcoded explosion playback rate
let patched = gameTs.replace(
  'playSFX("explosion_1", { playbackRate: 1.0 })',
  'playSFX("explosion_1", { playbackRate: 0.7 + Math.random() * 0.5 })'
);

// Increase camera shake intensity multiplier
patched = patched.replace(
  "shakeIntensity * 0.5",
  "shakeIntensity * 1.2"
);

writeFileSync("dist/Game.js", patched);
```

### patch2.js — Meteor Shader Materials

```javascript
// patch2.js
// Replaces MeteorShower.ts MeshBasicMaterials with custom ShaderMaterials
// for trail, flash, and shockwave using GLSL noise

const meteorJs = readFileSync("dist/MeteorShower.js", "utf8");

// Replace MeshBasicMaterial for trail with custom shader
patched = meteorJs.replace(
  "new MeshBasicMaterial({ color: 0xff6600 })",
  `new ShaderMaterial({
    uniforms: { time: { value: 0 } },
    vertexShader: trailVertexShader,
    fragmentShader: trailFragmentShader,
  })`
);

// Similar replacements for flash and shockwave materials
```

### patch3.js — Meteor Complete Rewrite

```javascript
// patch3.js
// Complete rewrite of MeteorShower.ts:
// - Dodecahedron head shader with lava cracks
// - 300-point particle spark system
// - Fire dome shader
// - Shockwave ring shader

// This patch replaces the entire MeteorShower update loop
// with a more complex particle and shader system
```

The meteor patches are the most aggressive — patch3.js essentially replaces the entire MeteorShower module with a more visually impressive version that uses custom shaders for every component (head, trail, spark, shockwave, fire dome).

**Why post-build patches?** These changes modify Three.js internals or generated code that would be cumbersome to express in TypeScript source. The patches allow rapid visual iteration without changing the core game logic.

See [Atmospheric VFX](08-atmospheric-vfx.md) for the visual effects these patches create.
See [Particle Systems](09-particle-systems.md) for meteor spark particles.
