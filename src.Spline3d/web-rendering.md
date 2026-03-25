# Web Rendering for 3D - Deep Dive

**Source:** `/home/darkvoid/Boxxed/@formulas/src.rust/src.Spline3d/`

This document covers web-based 3D rendering technologies including Three.js, WebGL, WebGPU, and WASM integration.

---

## Table of Contents

1. [Web Rendering Overview](#web-rendering-overview)
2. [WebGL Fundamentals](#webgl-fundamentals)
3. [Three.js Architecture](#threejs-architecture)
4. [React Three Fiber](#react-three-fiber)
5. [WebGPU](#webgpu)
6. [WASM for 3D Graphics](#wasm-for-3d-graphics)
7. [Spline Runtime Architecture](#spline-runtime-architecture)
8. [Performance Optimization](#performance-optimization)
9. [Rust Web Rendering](#rust-web-rendering)

---

## Web Rendering Overview

### Technology Stack Comparison

```
┌─────────────────────────────────────────────────────────────────┐
│                    Web 3D Rendering Stack                        │
│                                                                  │
│  ┌───────────────────────────────────────────────────────────┐  │
│  │                   Application Layer                        │  │
│  │  ┌─────────────┐  ┌─────────────┐  ┌─────────────────┐   │  │
│  │  │   Spline    │  │  Babylon.js │  │   PlayCanvas    │   │  │
│  │  │   Runtime   │  │             │  │                 │   │  │
│  │  └─────────────┘  └─────────────┘  └─────────────────┘   │  │
│  └───────────────────────────────────────────────────────────┘  │
│                              │                                   │
│  ┌───────────────────────────────────────────────────────────┐  │
│  │                    Engine Layer                            │  │
│  │  ┌─────────────┐  ┌─────────────┐  ┌─────────────────┐   │  │
│  │  │  Three.js   │  │   OGRE      │  │   Custom        │   │  │
│  │  │             │  │   (WASM)    │  │   Engine        │   │  │
│  │  └─────────────┘  └─────────────┘  └─────────────────┘   │  │
│  └───────────────────────────────────────────────────────────┘  │
│                              │                                   │
│  ┌───────────────────────────────────────────────────────────┐  │
│  │                   Graphics API Layer                       │  │
│  │  ┌─────────────┐  ┌─────────────┐                         │  │
│  │  │   WebGL     │  │   WebGPU    │                         │  │
│  │  │   1.0/2.0   │  │   (Modern)  │                         │  │
│  │  └─────────────┘  └─────────────┘                         │  │
│  └───────────────────────────────────────────────────────────┘  │
│                              │                                   │
│  ┌───────────────────────────────────────────────────────────┐  │
│  │                    System Layer                            │  │
│  │  ┌─────────────┐  ┌─────────────┐  ┌─────────────────┐   │  │
│  │  │   OpenGL    │  │   DirectX   │  │     Metal       │   │  │
│  │  │   ES        │  │   11/12     │  │                 │   │  │
│  │  └─────────────┘  └─────────────┘  └─────────────────┘   │  │
│  └───────────────────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────────────────┘
```

### Browser Support Matrix

| Technology | Chrome | Firefox | Safari | Edge |
|------------|--------|---------|--------|------|
| WebGL 1.0  | ✓ All  | ✓ All   | ✓ All  | ✓ All |
| WebGL 2.0  | ✓ 56+  | ✓ 51+   | ✓ 15+  | ✓ 79+ |
| WebGPU     | ✓ 113+ | ✓ 119+  | ✓ 17.4+| ✓ 113+ |
| WASM       | ✓ All  | ✓ All   | ✓ All  | ✓ All |

---

## WebGL Fundamentals

### WebGL Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                      WebGL Pipeline                              │
│                                                                  │
│  JavaScript              WebGL API              GPU Driver       │
│  ┌─────────┐            ┌─────────┐            ┌─────────┐     │
│  │         │            │         │            │         │     │
│  │  App    │ ────────>  │ WebGL   │ ────────>  │  GPU    │     │
│  │  Code   │  gl.*      │ Context │  Commands  │         │     │
│  │         │            │         │            │         │     │
│  └─────────┘            └─────────┘            └─────────┘     │
│                                                                  │
│  Data Flow:                                                     │
│  ┌──────────┐   ┌──────────┐   ┌──────────┐   ┌──────────┐    │
│  │ Vertices │──>│ Vertex   │──>│ Raster   │──>│ Fragment │    │
│  │  (JS)    │   │ Shader   │   │          │   │  Shader  │    │
│  └──────────┘   └──────────┘   └──────────┘   └──────────┘    │
│                                      │                │         │
│                                      ▼                ▼         │
│                               ┌──────────┐   ┌──────────┐      │
│                               │  Depth   │   │  Frame   │      │
│                               │  Test    │   │  Buffer  │      │
│                               └──────────┘   └──────────┘      │
└─────────────────────────────────────────────────────────────────┘
```

### WebGL Rendering Context

```javascript
// Get WebGL context
const canvas = document.getElementById('canvas');
const gl = canvas.getContext('webgl2') || canvas.getContext('webgl');

// Basic setup
gl.viewport(0, 0, canvas.width, canvas.height);
gl.clearColor(0.0, 0.0, 0.0, 1.0);
gl.clear(gl.COLOR_BUFFER_BIT | gl.DEPTH_BUFFER_BIT);

// Enable depth testing
gl.enable(gl.DEPTH_TEST);
gl.depthFunc(gl.LEQUAL);

// Enable backface culling
gl.enable(gl.CULL_FACE);
gl.cullFace(gl.BACK);
```

### Shader Programs

```javascript
// Vertex Shader (WGSL/GLSL)
const vertexShaderSource = `
  attribute vec4 a_position;
  attribute vec4 a_color;

  uniform mat4 u_matrix;

  varying vec4 v_color;

  void main() {
    gl_Position = u_matrix * a_position;
    v_color = a_color;
  }
`;

// Fragment Shader
const fragmentShaderSource = `
  precision mediump float;

  varying vec4 v_color;

  void main() {
    gl_FragColor = v_color;
  }
`;

// Compile shader
function createShader(gl, type, source) {
  const shader = gl.createShader(type);
  gl.shaderSource(shader, source);
  gl.compileShader(shader);

  if (!gl.getShaderParameter(shader, gl.COMPILE_STATUS)) {
    console.error(gl.getShaderInfoLog(shader));
    gl.deleteShader(shader);
    return null;
  }
  return shader;
}

// Create program
function createProgram(gl, vertexShader, fragmentShader) {
  const program = gl.createProgram();
  gl.attachShader(program, vertexShader);
  gl.attachShader(program, fragmentShader);
  gl.linkProgram(program);

  if (!gl.getProgramParameter(program, gl.LINK_STATUS)) {
    console.error(gl.getProgramInfoLog(program));
    return null;
  }
  return program;
}
```

### Buffer Management

```javascript
// Create vertex buffer
const positionBuffer = gl.createBuffer();
gl.bindBuffer(gl.ARRAY_BUFFER, positionBuffer);
gl.bufferData(
  gl.ARRAY_BUFFER,
  new Float32Array([
    // Triangle
    0.0,  0.5,  0.0,
   -0.5, -0.5,  0.0,
    0.5, -0.5,  0.0,
  ]),
  gl.STATIC_DRAW
);

// Get attribute location
const positionLocation = gl.getAttribLocation(program, 'a_position');

// Enable vertex attribute
gl.enableVertexAttribArray(positionLocation);
gl.vertexAttribPointer(
  positionLocation,
  3,          // components per vertex
  gl.FLOAT,   // data type
  false,      // normalize
  0,          // stride
  0           // offset
);

// Draw
gl.drawArrays(gl.TRIANGLES, 0, 3);
```

### Index Buffers

```javascript
// Index buffer for efficient mesh rendering
const indexBuffer = gl.createBuffer();
gl.bindBuffer(gl.ELEMENT_ARRAY_BUFFER, indexBuffer);
gl.bufferData(
  gl.ELEMENT_ARRAY_BUFFER,
  new Uint16Array([
    0, 1, 2,  // First triangle
    2, 3, 0,  // Second triangle (quad)
  ]),
  gl.STATIC_DRAW
);

// Draw with indices
gl.drawElements(gl.TRIANGLES, 6, gl.UNSIGNED_SHORT, 0);
```

### Uniforms and Transforms

```javascript
// Matrix utilities
const m4 = {
  perspective: (fov, aspect, near, far) => {
    const f = 1.0 / Math.tan(fov / 2);
    const rangeInv = 1 / (near - far);
    return [
      f / aspect, 0, 0, 0,
      0, f, 0, 0,
      0, 0, (near + far) * rangeInv, -1,
      0, 0, near * far * rangeInv * 2, 0
    ];
  },

  translation: (tx, ty, tz) => [
    1, 0, 0, 0,
    0, 1, 0, 0,
    0, 0, 1, 0,
    tx, ty, tz, 1
  ],

  rotationY: (angle) => {
    const c = Math.cos(angle);
    const s = Math.sin(angle);
    return [
      c, 0, -s, 0,
      0, 1, 0, 0,
      s, 0, c, 0,
      0, 0, 0, 1
    ];
  },

  multiply: (a, b) => {
    const result = new Array(16).fill(0);
    for (let i = 0; i < 4; i++) {
      for (let j = 0; j < 4; j++) {
        for (let k = 0; k < 4; k++) {
          result[i * 4 + j] += a[i * 4 + k] * b[k * 4 + j];
        }
      }
    }
    return result;
  }
};

// Set uniform
const matrixLocation = gl.getUniformLocation(program, 'u_matrix');
const projection = m4.perspective(Math.PI / 4, canvas.width / canvas.height, 0.1, 100);
const view = m4.translation(0, 0, -5);
const model = m4.rotationY(Date.now() * 0.001);

const matrix = m4.multiply(projection, m4.multiply(view, model));
gl.uniformMatrix4fv(matrixLocation, false, matrix);
```

---

## Three.js Architecture

### Scene Graph

```
┌─────────────────────────────────────────────────────────────────┐
│                    Three.js Scene Graph                          │
│                                                                  │
│                      ┌──────────────┐                           │
│                      │    Scene     │                           │
│                      └──────┬───────┘                           │
│                             │                                    │
│         ┌───────────────────┼───────────────────┐               │
│         │                   │                   │               │
│    ┌────▼────┐        ┌────▼────┐        ┌────▼────┐          │
│    │ Camera  │        │  Light  │        │  Group  │          │
│    │(Persp.) │        │(Dir.)   │        │         │          │
│    └─────────┘        └─────────┘        └────┬────┘          │
│                                               │                │
│                        ┌──────────────────────┼──────┐         │
│                        │                      │      │         │
│                   ┌────▼────┐           ┌────▼────┐ │         │
│                   │  Mesh   │           │  Mesh   │ │         │
│                   │  (Car)  │           │(Wheel)  │ │         │
│                   │         │           │         │ │         │
│                   └────┬────┘           └─────────┘ │         │
│                        │                            │         │
│              ┌─────────┴─────────┐                  │         │
│              │                   │                  │         │
│         ┌────▼────┐        ┌────▼────┐        ┌────▼────┐    │
│         │ Geometry│        │ Material│        │  Mesh   │    │
│         │(Box)    │        │(Lambert)│        │(Body)   │    │
│         └─────────┘        └─────────┘        └─────────┘    │
└─────────────────────────────────────────────────────────────────┘
```

### Core Classes

```javascript
import * as THREE from 'three';

// Scene
const scene = new THREE.Scene();
scene.background = new THREE.Color(0x000000);

// Camera
const camera = new THREE.PerspectiveCamera(
  75,                                    // FOV
  window.innerWidth / window.innerHeight, // Aspect
  0.1,                                   // Near
  1000                                   // Far
);
camera.position.z = 5;

// Renderer
const renderer = new THREE.WebGLRenderer({
  canvas: document.getElementById('canvas'),
  antialias: true,
  alpha: true,
});
renderer.setSize(window.innerWidth, window.innerHeight);
renderer.setPixelRatio(window.devicePixelRatio);

// Geometry
const geometry = new THREE.BoxGeometry(1, 1, 1);

// Material
const material = new THREE.MeshStandardMaterial({
  color: 0x00ff00,
  roughness: 0.5,
  metalness: 0.5,
});

// Mesh
const cube = new THREE.Mesh(geometry, material);
scene.add(cube);

// Light
const light = new THREE.DirectionalLight(0xffffff, 1);
light.position.set(1, 1, 1);
scene.add(light);

// Render loop
function animate() {
  requestAnimationFrame(animate);
  cube.rotation.x += 0.01;
  cube.rotation.y += 0.01;
  renderer.render(scene, camera);
}
animate();
```

### Material System

```javascript
// Basic material (no lighting)
const basicMat = new THREE.MeshBasicMaterial({
  color: 0xff0000,
  map: texture,
});

// Lambert material (diffuse only)
const lambertMat = new THREE.MeshLambertMaterial({
  color: 0x00ff00,
});

// Phong material (diffuse + specular)
const phongMat = new THREE.MeshPhongMaterial({
  color: 0x0000ff,
  shininess: 100,
  specular: 0x444444,
});

// Standard material (PBR - physically based)
const standardMat = new THREE.MeshStandardMaterial({
  color: 0xffffff,
  roughness: 0.5,    // 0 = glossy, 1 = matte
  metalness: 0.5,    // 0 = dielectric, 1 = metal
  normalMap: normalMap,
  displacementMap: displacementMap,
});

// Physical material (extended PBR)
const physicalMat = new THREE.MeshPhysicalMaterial({
  color: 0xffffff,
  metalness: 0.0,
  roughness: 0.5,
  transmission: 1.0,   // Glass-like
  thickness: 0.5,      // Volume thickness
  clearcoat: 1.0,      // Car paint effect
  clearcoatRoughness: 0.1,
});
```

### Texture Loading

```javascript
const textureLoader = new THREE.TextureLoader();

// Load texture
const diffuseMap = textureLoader.load('texture.jpg', (texture) => {
  console.log('Texture loaded');
}, undefined, (err) => {
  console.error('Error loading texture', err);
});

// Texture settings
diffuseMap.wrapS = THREE.RepeatWrapping;
diffuseMap.wrapT = THREE.RepeatWrapping;
diffuseMap.repeat.set(2, 2);
diffuseMap.minFilter = THREE.LinearMipMapLinearFilter;
diffuseMap.magFilter = THREE.LinearFilter;

// Environment map (for reflections)
const envMap = textureLoader.load('envmap.hdr');
envMap.mapping = THREE.EquirectangularReflectionMapping;
scene.environment = envMap;
```

### Geometry Types

```javascript
// Built-in geometries
const boxGeo = new THREE.BoxGeometry(1, 1, 1);
const sphereGeo = new THREE.SphereGeometry(1, 32, 32);
const cylinderGeo = new THREE.CylinderGeometry(1, 1, 2, 32);
const torusGeo = new THREE.TorusGeometry(1, 0.3, 16, 100);
const planeGeo = new THREE.PlaneGeometry(2, 2);

// Custom geometry
const customGeo = new THREE.BufferGeometry();
const vertices = new Float32Array([
  0, 0, 0,  // vertex 0
  1, 0, 0,  // vertex 1
  0, 1, 0,  // vertex 2
]);
const colors = new Float32Array([
  1, 0, 0,  // red
  0, 1, 0,  // green
  0, 0, 1,  // blue
]);

customGeo.setAttribute('position', new THREE.BufferAttribute(vertices, 3));
customGeo.setAttribute('color', new THREE.BufferAttribute(colors, 3));

// Compute normals
customGeo.computeVertexNormals();
```

---

## React Three Fiber

### Basic Usage

```jsx
import { Canvas, useFrame } from '@react-three/fiber';
import { OrbitControls, PerspectiveCamera } from '@react-three/drei';

function RotatingCube() {
  const meshRef = useRef();

  useFrame((state, delta) => {
    meshRef.current.rotation.x += delta * 0.5;
    meshRef.current.rotation.y += delta * 0.5;
  });

  return (
    <mesh ref={meshRef}>
      <boxGeometry args={[1, 1, 1]} />
      <meshStandardMaterial color="hotpink" />
    </mesh>
  );
}

function App() {
  return (
    <Canvas>
      <PerspectiveCamera makeDefault position={[0, 0, 5]} />
      <OrbitControls />
      <ambientLight intensity={0.5} />
      <directionalLight position={[1, 1, 1]} intensity={1} />
      <RotatingCube />
    </Canvas>
  );
}
```

### Custom Hooks

```jsx
import { useFrame, useThree, useLoader } from '@react-three/fiber';
import { GLTFLoader } from 'three/examples/jsm/loaders/GLTFLoader';

function Model({ url }) {
  const { scene } = useLoader(GLTFLoader, url);
  const { camera } = useThree();

  useFrame((state) => {
    // Animate model
    scene.rotation.y = state.clock.getElapsedTime() * 0.5;
  });

  return <primitive object={scene} />;
}

function LoadingModel({ url }) {
  return (
    <Suspense fallback={<Loader />}>
      <Model url={url} />
    </Suspense>
  );
}
```

### State Management

```jsx
import { create } from 'zustand';

// Store
const useStore = create((set) => ({
  selectedObject: null,
  setSelectedObject: (obj) => set({ selectedObject: obj }),
  hoveredObject: null,
  setHoveredObject: (obj) => set({ hoveredObject: obj }),
}));

// Component
function InteractiveMesh() {
  const setSelectedObject = useStore((state) => state.setSelectedObject);
  const setHoveredObject = useStore((state) => state.setHoveredObject);

  return (
    <mesh
      onClick={(e) => setSelectedObject(e.object)}
      onPointerOver={(e) => setHoveredObject(e.object)}
      onPointerOut={(e) => setHoveredObject(null)}
    >
      <boxGeometry />
      <meshStandardMaterial />
    </mesh>
  );
}
```

---

## WebGPU

### WebGPU vs WebGL

| Feature | WebGL 2.0 | WebGPU |
|---------|-----------|--------|
| API Style | OpenGL-like | Modern (Vulkan/Metal) |
| Compute Shaders | Limited | Full support |
| Multithreading | No | Yes |
| Bind Groups | No | Yes |
| Pipeline State | Monolithic | Granular |
| Validation | Runtime | Mostly offline |

### WebGPU Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                      WebGPU Architecture                         │
│                                                                  │
│  ┌──────────────────────────────────────────────────────────┐   │
│  │                    GPUDevice                              │   │
│  │  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐    │   │
│  │  │  Pipelines   │  │  Bind Groups │  │   Buffers    │    │   │
│  │  └──────────────┘  ┌──────────────┐  ┌──────────────┐    │   │
│  │  │  Samplers      │  │   Textures  │  │   QuerySets │    │   │
│  │  └──────────────┘  └──────────────┘  └──────────────┘    │   │
│  └──────────────────────────────────────────────────────────┘   │
│                              │                                   │
│                    ┌─────────▼─────────┐                        │
│                    │   GPUQueue        │                        │
│                    │   (Command Buffers)                        │
│                    └─────────┬─────────┘                        │
│                              │                                   │
│                    ┌─────────▼─────────┐                        │
│                    │   GPUSwapChain    │                        │
│                    │   (Presenting)    │                        │
│                    └───────────────────┘                        │
└─────────────────────────────────────────────────────────────────┘
```

### WebGPU Basic Setup

```javascript
// Request adapter and device
const adapter = await navigator.gpu.requestAdapter({
  powerPreference: 'high-performance',
});

const device = await adapter.requestDevice({
  requiredFeatures: [],
  requiredLimits: {},
});

// Create swap chain / context
const canvas = document.getElementById('canvas');
const context = canvas.getContext('webgpu');
const format = navigator.gpu.getPreferredCanvasFormat();

context.configure({
  device,
  format,
  alphaMode: 'premultiplied',
});

// Create shader module
const shaderCode = `
@vertex
fn vs_main(@location(0) position: vec3f) -> @builtin(position) vec4f {
  return vec4f(position, 1.0);
}

@fragment
fn fs_main() -> @location(0) vec4f {
  return vec4f(1.0, 0.0, 0.0, 1.0);
}
`;

const shaderModule = device.createShaderModule({
  code: shaderCode,
});

// Create render pipeline
const pipeline = device.createRenderPipeline({
  layout: 'auto',
  vertex: {
    module: shaderModule,
    entryPoint: 'vs_main',
    buffers: [{
      arrayStride: 12, // 3 floats * 4 bytes
      attributes: [{
        shaderLocation: 0,
        offset: 0,
        format: 'float32x3',
      }],
    }],
  },
  fragment: {
    module: shaderModule,
    entryPoint: 'fs_main',
    targets: [{ format }],
  },
  primitive: {
    topology: 'triangle-list',
  },
});

// Create vertex buffer
const vertices = new Float32Array([
  0.0,  0.5,  0.0,
 -0.5, -0.5,  0.0,
  0.5, -0.5,  0.0,
]);

const vertexBuffer = device.createBuffer({
  size: vertices.byteLength,
  usage: GPUBufferUsage.VERTEX,
  mappedAtCreation: true,
});
new Float32Array(vertexBuffer.getMappedRange()).set(vertices);
vertexBuffer.unmap();

// Render
function frame() {
  const commandEncoder = device.createCommandEncoder();
  const textureView = context.getCurrentTexture().createView();

  const renderPass = commandEncoder.beginRenderPass({
    colorAttachments: [{
      view: textureView,
      clearValue: { r: 0, g: 0, b: 0, a: 1 },
      loadOp: 'clear',
      storeOp: 'store',
    }],
  });

  renderPass.setPipeline(pipeline);
  renderPass.setVertexBuffer(0, vertexBuffer);
  renderPass.draw(3);
  renderPass.end();

  device.queue.submit([commandEncoder.finish()]);
  requestAnimationFrame(frame);
}
frame();
```

---

## WASM for 3D Graphics

### WASM Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                     WASM Graphics Architecture                   │
│                                                                  │
│  ┌──────────────────────────────────────────────────────────┐   │
│  │                  JavaScript (Main Thread)                 │   │
│  │  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐       │   │
│  │  │   Event     │  │   Scene     │  │   WebGL     │       │   │
│  │  │   Handler   │  │   Graph     │  │   Context   │       │   │
│  │  └─────────────┘  └─────────────┘  └─────────────┘       │   │
│  └──────────────────────────────────────────────────────────┘   │
│                              │                                   │
│                    ┌─────────▼─────────┐                        │
│                    │   wasm-bindgen    │                        │
│                    │   (JS Bindings)   │                        │
│                    └─────────┬─────────┘                        │
│                              │                                   │
│  ┌──────────────────────────────────────────────────────────┐   │
│  │                  WebAssembly Module                       │   │
│  │  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐       │   │
│  │  │  Geometry   │  │  Rasterizer │  │  Shaders    │       │   │
│  │  │  Pipeline   │  │  (Software) │  │  (GLSL→SPIR-V)│     │   │
│  │  └─────────────┘  └─────────────┘  └─────────────┘       │   │
│  │  ┌─────────────┐  ┌─────────────┐                        │   │
│  │  │  Physics    │  │   Spatial   │                        │   │
│  │  │  Engine     │  │   Index     │                        │   │
│  │  └─────────────┘  └─────────────┘                        │   │
│  └──────────────────────────────────────────────────────────┘   │
│                              │                                   │
│                    ┌─────────▼─────────┐                        │
│                    │   WebGL Context   │                        │
│                    │   (via JS)        │                        │
│                    └─────────┬─────────┘                        │
│                              │                                   │
│                    ┌─────────▼─────────┐                        │
│                    │       GPU         │                        │
│                    └───────────────────┘                        │
└─────────────────────────────────────────────────────────────────┘
```

### Rust + WASM Setup

```toml
# Cargo.toml
[package]
name = "wasm-3d-engine"
version = "0.1.0"
edition = "2021"

[lib]
crate-type = ["cdylib", "rlib"]

[dependencies]
wasm-bindgen = "0.2"
wasm-bindgen-futures = "0.4"
js-sys = "0.3"
web-sys = { version = "0.3", features = [
  "WebGlRenderingContext",
  "WebGlProgram",
  "WebGlShader",
  "WebGlBuffer",
  "CanvasElement",
  "Document",
  "Window",
] }
nalgebra = "0.32"
console_error_panic_hook = "0.1"

[dev-dependencies]
wasm-bindgen-test = "0.3"
```

### Rust WASM WebGL Example

```rust
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use web_sys::{WebGlRenderingContext, WebGlProgram, WebGlShader};
use nalgebra::{Matrix4, Vector3};

#[wasm_bindgen]
pub struct Renderer {
    gl: WebGlRenderingContext,
    program: WebGlProgram,
    position_buffer: web_sys::WebGlBuffer,
}

#[wasm_bindgen]
impl Renderer {
    #[wasm_bindgen(constructor)]
    pub fn new(canvas: &web_sys::HtmlCanvasElement) -> Result<Renderer, JsValue> {
        let gl = canvas
            .get_context("webgl")?
            .ok_or("WebGL not supported")?
            .dyn_into::<WebGlRenderingContext>()?;

        // Create shader program
        let program = create_program(&gl)?;

        // Create vertex buffer
        let position_buffer = gl.create_buffer().ok_or("Failed to create buffer")?;
        gl.bind_buffer(WebGlRenderingContext::ARRAY_BUFFER, Some(&position_buffer));

        let vertices: Vec<f32> = vec![
            0.0,  0.5,  0.0,
           -0.5, -0.5,  0.0,
            0.5, -0.5,  0.0,
        ];
        gl.buffer_data_with_array_buffer(
            WebGlRenderingContext::ARRAY_BUFFER,
            &wasm_bindgen::JsValue::from_slice(&vertices),
            WebGlRenderingContext::STATIC_DRAW,
        );

        Ok(Renderer {
            gl,
            program,
            position_buffer,
        })
    }

    pub fn render(&self) -> Result<(), JsValue> {
        let gl = &self.gl;

        // Clear
        gl.clear_color(0.0, 0.0, 0.0, 1.0);
        gl.clear(WebGlRenderingContext::COLOR_BUFFER_BIT);

        // Use program
        gl.use_program(Some(&self.program));

        // Bind vertex buffer
        gl.bind_buffer(WebGlRenderingContext::ARRAY_BUFFER, Some(&self.position_buffer));

        // Set up vertex attribute
        let position = gl.get_attrib_location(&self.program, "a_position") as u32;
        gl.vertex_attrib_pointer_with_i32(position, 3, WebGlRenderingContext::FLOAT, false, 0, 0);
        gl.enable_vertex_attrib_array(position);

        // Set uniform
        let matrix = Matrix4::new_rotation(0.5);
        let matrix_uniform = gl.get_uniform_location(&self.program, "u_matrix");

        // Note: WebGL in wasm-bindgen requires manual conversion
        let matrix_array: Vec<f32> = matrix.as_slice().to_vec();
        unsafe {
            let array = wasm_bindgen::JsValue::from_slice(&matrix_array);
            gl.uniform_matrix4fv_with_f32_array(Some(&matrix_uniform), false, &matrix_array);
        }

        // Draw
        gl.draw_arrays(WebGlRenderingContext::TRIANGLES, 0, 3);

        Ok(())
    }
}

fn create_shader(
    gl: &WebGlRenderingContext,
    shader_type: u32,
    source: &str,
) -> Result<WebGlShader, String> {
    let shader = gl.create_shader(shader_type).ok_or("Failed to create shader")?;
    gl.shader_source(&shader, source);
    gl.compile_shader(&shader);

    if !gl.get_shader_parameter(&shader, WebGlRenderingContext::COMPILE_STATUS)
        .as_bool()
        .unwrap_or(false)
    {
        return Err(gl.get_shader_info_log(&shader).unwrap_or("Unknown error".to_string()));
    }

    Ok(shader)
}

fn create_program(gl: &WebGlRenderingContext) -> Result<WebGlProgram, JsValue> {
    const VERTEX_SHADER: &str = r#"
        attribute vec3 a_position;
        uniform mat4 u_matrix;
        void main() {
            gl_Position = u_matrix * vec4(a_position, 1.0);
        }
    "#;

    const FRAGMENT_SHADER: &str = r#"
        precision mediump float;
        void main() {
            gl_FragColor = vec4(1.0, 0.0, 0.0, 1.0);
        }
    "#;

    let vertex_shader = create_shader(gl, WebGlRenderingContext::VERTEX_SHADER, VERTEX_SHADER)
        .map_err(JsValue::from_str)?;
    let fragment_shader = create_shader(gl, WebGlRenderingContext::FRAGMENT_SHADER, FRAGMENT_SHADER)
        .map_err(JsValue::from_str)?;

    let program = gl.create_program().ok_or("Failed to create program")?;
    gl.attach_shader(&program, &vertex_shader);
    gl.attach_shader(&program, &fragment_shader);
    gl.link_program(&program);

    if !gl.get_program_parameter(&program, WebGlRenderingContext::LINK_STATUS)
        .as_bool()
        .unwrap_or(false)
    {
        return Err(JsValue::from_str(
            &gl.get_program_info_log(&program).unwrap_or("Unknown error".to_string())
        ));
    }

    Ok(program)
}

#[wasm_bindgen(start)]
pub fn main() -> Result<(), JsValue> {
    console_error_panic_hook::set_once();

    let window = web_sys::window().ok_or("No window")?;
    let document = window.document().ok_or("No document")?;
    let canvas = document
        .get_element_by_id("canvas")
        .ok_or("No canvas")?
        .dyn_into::<web_sys::HtmlCanvasElement>()?;

    let renderer = Renderer::new(&canvas)?;

    // Render loop
    let f = Closure::wrap(Box::new(move || {
        renderer.render().unwrap();
    }) as Box<dyn FnMut()>);

    window.request_animation_frame(f.as_ref().unchecked_ref())?;
    f.forget();

    Ok(())
}
```

---

## Spline Runtime Architecture

### Module Structure

```
@splinetool/
├── runtime/           # Core runtime engine
│   ├── Application   # Main app class
│   ├── Scene         # Scene management
│   ├── Objects       # 3D object types
│   ├── Events        # Event system
│   └── Renderer      # WebGL/WebGPU renderer
│
├── loader/           # Scene file loader
│   ├── SplineLoader  # .spline parser
│   └── Decoder       # Binary/text decoder
│
└── react-spline/     # React integration
    ├── Spline        # React component
    └── next/         # Next.js SSR support
```

### Application Class

```javascript
import { Application } from '@splinetool/runtime';

// Initialize
const canvas = document.getElementById('canvas');
const app = new Application(canvas, {
  renderOnDemand: true,  // Only render when needed
  wasmPath: '/wasm/',    // Optional WASM path
});

// Load scene
await app.load('https://prod.spline.design/xxx/scene.splinecode');

// Query objects
const cube = app.findObjectByName('Cube');
const sphere = app.findObjectById('8E8C2DDD-18B6-4C54');

// Modify objects
cube.position.x += 10;
sphere.rotation.y = Math.PI / 4;

// Event listeners
app.addEventListener('mouseDown', (e) => {
  if (e.target.name === 'Cube') {
    console.log('Cube clicked!');
  }
});

// Trigger events programmatically
cube.emitEvent('mouseHover');

// Cleanup
app.dispose();
```

### Scene File Format

```
scene.splinecode (binary format):
├── Header
│   ├── Magic bytes
│   ├── Version
│   └── Metadata
│
├── Scene Graph
│   ├── Objects (hierarchical)
│   ├── Transforms
│   └── Properties
│
├── Geometry
│   ├── Vertices
│   ├── Normals
│   ├── UVs
│   └── Indices
│
├── Materials
│   ├── PBR parameters
│   ├── Textures (embedded/referenced)
│   └── Shaders
│
├── Animations
│   ├── Keyframes
│   ├── Timelines
│   └── Curves
│
└── Events
    ├── Triggers
    ├── Actions
    └── Targets
```

### React Integration

```jsx
// react-spline implementation
import { useEffect, useRef, useState } from 'react';
import { Application } from '@splinetool/runtime';

function Spline({ scene, onLoad, renderOnDemand = true }) {
  const canvasRef = useRef(null);
  const [app, setApp] = useState(null);

  useEffect(() => {
    if (!canvasRef.current) return;

    const application = new Application(canvasRef.current, {
      renderOnDemand,
    });

    application.load(scene)
      .then(() => {
        setApp(application);
        onLoad?.(application);
      })
      .catch(console.error);

    return () => {
      application.dispose();
    };
  }, [scene, renderOnDemand]);

  return <canvas ref={canvasRef} />;
}

export default Spline;
```

---

## Performance Optimization

### Rendering Optimization

```javascript
// 1. Instanced rendering
const mesh = new THREE.InstancedMesh(geometry, material, count);
for (let i = 0; i < count; i++) {
  mesh.setMatrixAt(i, matrix);
}
scene.add(mesh);  // Single draw call for all instances

// 2. Geometry merging
const mergedGeometry = mergeGeometries(geometries);
const mergedMesh = new THREE.Mesh(mergedGeometry, material);

// 3. Level of Detail (LOD)
const lod = new THREE.LOD();
lod.addLevel(highDetailMesh, 0);      // 0-50% screen
lod.addLevel(mediumDetailMesh, 0.5);  // 50-80% screen
lod.addLevel(lowDetailMesh, 0.8);     // 80-100% screen
scene.add(lod);

// 4. Frustum culling (automatic in Three.js)
mesh.frustumCulled = true;

// 5. Occlusion culling (manual or via plugin)
```

### WASM Optimization

```rust
// 1. Use release builds
// cargo build --release
// wasm-opt -O3 output.wasm -o output.optimized.wasm

// 2. Minimize JS/WASM boundary crossings
#[wasm_bindgen]
pub struct BatchRenderer {
    data: Vec<f32>,
}

#[wasm_bindgen]
impl BatchRenderer {
    // Batch operations to reduce crossings
    pub fn add_many(&mut self, vertices: &[f32]) {
        self.data.extend_from_slice(vertices);
    }
}

// 3. Use shared memory
// [dependencies]
// wasm-bindgen = { version = "0.2", features = ["enable-interning"] }

// 4. Pre-allocate buffers
let mut buffer = Vec::with_capacity(expected_size);
```

### Loading Optimization

```javascript
// 1. Lazy loading
const Spline = React.lazy(() => import('@splinetool/react-spline'));

function App() {
  return (
    <Suspense fallback={<Loading />}>
      <Spline scene="url" />
    </Suspense>
  );
}

// 2. Progressive loading
const app = new Application(canvas);
await app.load(scene, {
  onProgress: (progress) => {
    console.log(`Loading: ${progress * 100}%`);
  },
});

// 3. Texture compression
// Use KTX2/DDS formats with Basis Universal
const compressedTexture = await loadKTX2('texture.ktx2');

// 4. Draco compression for geometry
import { DRACOLoader } from 'three/examples/jsm/loaders/DRACOLoader';
const dracoLoader = new DRACOLoader();
dracoLoader.setDecoderPath('/draco/');
```

---

## Rust Web Rendering

### Recommended Crates

```toml
[dependencies]
# Core
wasm-bindgen = "0.2"
web-sys = "0.3"
js-sys = "0.3"
console_error_panic_hook = "0.1"

# Math
nalgebra = "0.32"
cgmath = "0.18"

# Rendering
wgpu = "0.19"         # Cross-platform GPU
winit = "0.29"        # Window management
pollster = "0.3"      # Async runtime

# Higher-level
nannou = "0.18"       # Creative coding
macroquad = "0.4"     # Simple game engine
```

### WGPU Example

```rust
use wgpu::*;
use winit::{
    application::ApplicationBuilder,
    event::*,
    event_loop::EventLoop,
    window::{Window, WindowBuilder},
};

struct GraphicsApp {
    device: Device,
    queue: Queue,
    surface: Surface<'static>,
    config: SurfaceConfiguration,
    pipeline: RenderPipeline,
    vertex_buffer: Buffer,
    size: winit::dpi::PhysicalSize<u32>,
}

impl GraphicsApp {
    async fn new(window: Window) -> Self {
        let size = window.inner_size();

        let instance = Instance::new(InstanceDescriptor {
            backends: Backends::all(),
            ..Default::default()
        });

        let surface = instance.create_surface(window).unwrap();

        let adapter = instance
            .request_adapter(&RequestAdapterOptions {
                power_preference: PowerPreference::HighPerformance,
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            })
            .await
            .unwrap();

        let (device, queue) = adapter
            .request_device(&DeviceDescriptor::default(), None)
            .await
            .unwrap();

        let caps = surface.get_capabilities(&adapter);
        let format = caps
            .formats
            .into_iter()
            .find(|f| f.is_srgb())
            .unwrap_or(caps.formats[0]);

        let config = SurfaceConfiguration {
            usage: TextureUsages::RENDER_ATTACHMENT,
            format,
            width: size.width,
            height: size.height,
            present_mode: PresentMode::Fifo,
            alpha_mode: caps.alpha_modes[0],
            view_formats: vec![],
        };
        surface.configure(&device, &config);

        // Create pipeline, buffers, etc.
        // ...

        GraphicsApp {
            device,
            queue,
            surface,
            config,
            pipeline,
            vertex_buffer,
            size,
        }
    }

    fn render(&mut self) -> Result<(), SurfaceError> {
        let output = self.surface.get_current_texture()?;
        let view = output
            .texture
            .create_view(&TextureViewDescriptor::default());

        let mut encoder = self
            .device
            .create_command_encoder(&CommandEncoderDescriptor {
                label: Some("Render Encoder"),
            });

        {
            let mut render_pass = encoder.begin_render_pass(&RenderPassDescriptor {
                label: Some("Render Pass"),
                color_attachments: &[Some(RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: Operations {
                        load: LoadOp::Clear(Color::BLACK),
                        store: StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
            });

            render_pass.set_pipeline(&self.pipeline);
            render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
            render_pass.draw(0..3, 0..1);
        }

        self.queue.submit(std::iter::once(encoder.finish()));
        output.present();

        Ok(())
    }

    fn resize(&mut self, size: winit::dpi::PhysicalSize<u32>) {
        self.size = size;
        self.config.width = size.width.max(1);
        self.config.height = size.height.max(1);
        self.surface.configure(&self.device, &self.config);
    }
}
```

---

## References

1. **Three.js Documentation** - https://threejs.org/docs/
2. **WebGPU Specification** - https://www.w3.org/TR/webgpu/
3. **WebGL Programming Guide** - Addison-Wesley
4. **wasm-bindgen Guide** - https://rustwasm.github.io/wasm-bindgen/
5. **Spline Documentation** - https://docs.spline.design/
