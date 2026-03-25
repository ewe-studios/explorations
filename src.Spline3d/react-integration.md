# React Integration for Spline 3D

**Source:** `/home/darkvoid/Boxxed/@formulas/src.rust/src.Spline3d/`

This document covers React integration patterns for Spline 3D scenes, including `react-spline` and `r3f-spline` packages.

---

## Table of Contents

1. [Overview](#overview)
2. [react-spline Package](#react-spline-package)
3. [r3f-spline Package](#r3f-spline-package)
4. [Component Comparison](#component-comparison)
5. [Advanced Patterns](#advanced-patterns)
6. [Next.js Integration](#nextjs-integration)
7. [State Management](#state-management)
8. [Performance Optimization](#performance-optimization)
9. [TypeScript Support](#typescript-support)

---

## Overview

### Package Ecosystem

```
┌─────────────────────────────────────────────────────────────────┐
│                  Spline React Ecosystem                          │
│                                                                  │
│  ┌───────────────────────────────────────────────────────────┐  │
│  │              @splinetool/react-spline                      │  │
│  │                                                            │  │
│  │  - Direct canvas rendering                                 │  │
│  │  - Simple API                                              │  │
│  │  - Event handling built-in                                 │  │
│  │  - SSR support (via /next)                                 │  │
│  └───────────────────────────────────────────────────────────┘  │
│                                                                  │
│  ┌───────────────────────────────────────────────────────────┐  │
│  │              @splinetool/r3f-spline                        │  │
│  │                                                            │  │
│  │  - React Three Fiber integration                           │  │
│  │  - Full Three.js scene graph access                        │  │
│  │  - Composable with other R3F components                    │  │
│  │  - Hook-based API                                          │  │
│  └───────────────────────────────────────────────────────────┘  │
│                                                                  │
│  ┌───────────────────────────────────────────────────────────┐  │
│  │              @splinetool/loader                            │  │
│  │                                                            │  │
│  │  - Low-level scene loader                                  │  │
│  │  - Custom integrations                                     │  │
│  │  - Framework agnostic                                      │  │
│  └───────────────────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────────────────┘
```

### Installation

```bash
# For react-spline (direct rendering)
npm install @splinetool/react-spline @splinetool/runtime
# or
yarn add @splinetool/react-spline @splinetool/runtime

# For r3f-spline (React Three Fiber)
npm install @splinetool/r3f-spline @splinetool/loader @react-three/fiber three
# or
yarn add @splinetool/r3f-spline @splinetool/loader @react-three/fiber three

# For Next.js SSR
npm install @splinetool/react-spline @splinetool/runtime
# Import from @splinetool/react-spline/next
```

---

## react-spline Package

### Basic Usage

```jsx
import Spline from '@splinetool/react-spline';

function App() {
  return (
    <Spline scene="https://prod.spline.design/6Wq1Q7YGyM-iab9i/scene.splinecode" />
  );
}
```

### Component Props

```jsx
<Spline
  // Required
  scene="https://prod.spline.design/xxx/scene.splinecode"

  // Optional props
  onLoad={(spline) => {
    // Called when scene is loaded
    console.log('Scene loaded', spline);
  }}

  renderOnDemand={true}  // Only render when needed (performance)
  className="my-spline"  // CSS class for container
  id="spline-canvas"     // Canvas ID
  wasmPath="/wasm/"      // Path to WASM files

  // Style props
  style={{
    width: '100%',
    height: '100vh',
  }}

  // Event listeners
  onSplineMouseDown={(e) => console.log('Mouse down', e)}
  onSplineMouseUp={(e) => console.log('Mouse up', e)}
  onSplineMouseHover={(e) => console.log('Mouse hover', e)}
  onSplineKeyDown={(e) => console.log('Key down', e)}
  onSplineKeyUp={(e) => console.log('Key up', e)}
  onSplineStart={(e) => console.log('Start', e)}
  onSplineLookAt={(e) => console.log('Look at', e)}
  onSplineFollow={(e) => console.log('Follow', e)}
  onSplineScroll={(e) => console.log('Scroll', e)}
/>
```

### Object Interaction

```jsx
import { useRef } from 'react';
import Spline from '@splinetool/react-spline';
import anime from 'animejs';

function InteractiveScene() {
  const cubeRef = useRef(null);
  const splineAppRef = useRef(null);

  function onLoad(spline) {
    splineAppRef.current = spline;

    // Find object by name
    cubeRef.current = spline.findObjectByName('Cube');

    // Or find by ID (from Spline editor Develop panel)
    const sphere = spline.findObjectById('8E8C2DDD-18B6-4C54-861D-7ED2519DE20E');

    // Access object properties
    console.log('Cube position:', cubeRef.current.position);
    console.log('Cube rotation:', cubeRef.current.rotation);
    console.log('Cube scale:', cubeRef.current.scale);
  }

  function moveCube() {
    if (!cubeRef.current) return;

    // Animate position
    anime({
      targets: cubeRef.current.position,
      x: cubeRef.current.position.x + 100,
      duration: 1000,
      easing: 'easeInOutQuad',
    });
  }

  function changeColor() {
    if (!cubeRef.current) return;

    // Modify material
    cubeRef.current.material.color = { r: 1, g: 0, b: 0, a: 1 };
  }

  return (
    <>
      <button onClick={moveCube}>Move Cube</button>
      <button onClick={changeColor}>Change Color</button>
      <Spline
        scene="https://prod.spline.design/xxx/scene.splinecode"
        onLoad={onLoad}
      />
    </>
  );
}
```

### Event Handling

```jsx
function EventScene() {
  function onSplineMouseDown(e) {
    // e.target contains the clicked object
    console.log('Clicked:', e.target.name);
    console.log('Intersection point:', e.intersectionPoint);
  }

  function onSplineMouseHover(e) {
    // Highlight hovered object
    e.target.material.emissive = { r: 0.5, g: 0.5, b: 0.5 };
  }

  return (
    <Spline
      scene="url"
      onSplineMouseDown={onSplineMouseDown}
      onSplineMouseHover={onSplineMouseHover}
    />
  );
}
```

---

## r3f-spline Package

### Basic Usage

```jsx
import { Canvas } from '@react-three/fiber';
import useSpline from '@splinetool/r3f-spline';

function Scene() {
  const { nodes, materials } = useSpline(
    'https://prod.spline.design/xxx/scene.spline'
  );

  return (
    <group dispose={null}>
      <mesh geometry={nodes.Cube.geometry} material={materials.Cube} />
      <mesh geometry={nodes.Sphere.geometry} material={materials.Sphere} />
    </group>
  );
}

function App() {
  return (
    <Canvas>
      <ambientLight intensity={0.5} />
      <directionalLight position={[1, 1, 1]} />
      <Scene />
    </Canvas>
  );
}
```

### Full Example with Controls

```jsx
import { Suspense, useRef } from 'react';
import { Canvas, useFrame } from '@react-three/fiber';
import { OrbitControls, PerspectiveCamera, Environment } from '@react-three/drei';
import useSpline from '@splinetool/r3f-spline';

function SplineScene(props) {
  const { nodes, materials } = useSpline(
    'https://prod.spline.design/JOgoI55ADZgorWoy/scene.spline'
  );

  const groupRef = useRef();

  // Rotate scene every frame
  useFrame((state, delta) => {
    if (groupRef.current) {
      groupRef.current.rotation.y += delta * 0.1;
    }
  });

  return (
    <group ref={groupRef} {...props} dispose={null}>
      <mesh
        castShadow
        receiveShadow
        geometry={nodes.Torus.geometry}
        material={materials['Torus Material']}
      />
      <mesh
        castShadow
        receiveShadow
        geometry={nodes.Rectangle.geometry}
        material={materials['Rectangle Material']}
        position={[-1, 0, 0]}
      />
    </group>
  );
}

function App() {
  return (
    <Canvas shadows flat linear>
      <Suspense fallback={null}>
        <OrbitControls />
        <PerspectiveCamera makeDefault position={[5, 5, 5]} />
        <Environment preset="city" />
        <SplineScene />
      </Suspense>
    </Canvas>
  );
}
```

### Combining with Three.js Objects

```jsx
function MixedScene() {
  const { nodes, materials } = useSpline('url/scene.spline');

  return (
    <>
      {/* Spline objects */}
      <mesh geometry={nodes.Cube.geometry} material={materials.Cube} />

      {/* Native Three.js objects */}
      <mesh position={[2, 0, 0]}>
        <boxGeometry args={[1, 1, 1]} />
        <meshStandardMaterial color="hotpink" />
      </mesh>

      {/* Custom shader object */}
      <mesh position={[-2, 0, 0]}>
        <sphereGeometry args={[0.5, 32, 32]} />
        <shaderMaterial
          vertexShader={customVertexShader}
          fragmentShader={customFragmentShader}
        />
      </mesh>
    </>
  );
}
```

---

## Component Comparison

### Feature Matrix

| Feature | react-spline | r3f-spline |
|---------|-------------|------------|
| Setup Complexity | Low | Medium |
| Three.js Access | Limited | Full |
| Custom Shaders | No | Yes |
| R3F Ecosystem | No | Yes |
| SSR Support | Yes (/next) | Yes (SSR) |
| Event System | Built-in | Custom |
| Performance | Optimized | Depends on setup |
| Bundle Size | ~500KB | ~1MB+ |

### When to Use Each

**Use react-spline when:**
- You want simple integration
- No custom Three.js needed
- Quick prototyping
- Smaller bundle size preferred

**Use r3f-spline when:**
- You need full Three.js control
- Mixing with other R3F components
- Custom shaders/materials
- Advanced R3F features (drei, fiber)

---

## Advanced Patterns

### Lazy Loading

```jsx
import React, { Suspense } from 'react';

const Spline = React.lazy(() => import('@splinetool/react-spline'));

function App() {
  return (
    <Suspense fallback={<div>Loading 3D...</div>}>
      <Spline scene="url" />
    </Suspense>
  );
}
```

### Progressive Loading

```jsx
function LoadingSpline({ scene }) {
  const [progress, setProgress] = useState(0);
  const [loaded, setLoaded] = useState(false);

  return (
    <>
      {!loaded && (
        <div className="loading-overlay">
          <div className="progress-bar" style={{ width: `${progress * 100}%` }} />
          <span>{Math.round(progress * 100)}%</span>
        </div>
      )}
      <Spline
        scene={scene}
        onLoad={(spline) => setLoaded(true)}
      />
    </>
  );
}
```

### Multiple Scenes

```jsx
function MultiScene() {
  const scenes = [
    { url: 'scene1.splinecode', position: [0, 0, 0] },
    { url: 'scene2.splinecode', position: [10, 0, 0] },
    { url: 'scene3.splinecode', position: [-10, 0, 0] },
  ];

  return (
    <Canvas>
      {scenes.map((scene, i) => (
        <SplineScene key={i} url={scene.url} position={scene.position} />
      ))}
    </Canvas>
  );
}
```

---

## Next.js Integration

### Server-Side Rendering

```jsx
// app/page.js or pages/index.js
import Spline from '@splinetool/react-spline/next';

export default function Home() {
  return (
    <main>
      <Spline scene="https://prod.spline.design/xxx/scene.splinecode" />
    </main>
  );
}
```

### Dynamic Import (Client-Side Only)

```jsx
import dynamic from 'next/dynamic';

const Spline = dynamic(() => import('@splinetool/react-spline'), {
  ssr: false,  // Disable SSR
  loading: () => <p>Loading 3D...</p>,
});

export default function Page() {
  return <Spline scene="url" />;
}
```

### Next.js 13+ App Router

```jsx
// With Suspense boundary
import { Suspense } from 'react';
import Spline from '@splinetool/react-spline/next';

function SplineLoader() {
  return <div>Loading...</div>;
}

export default function Page() {
  return (
    <Suspense fallback={<SplineLoader />}>
      <Spline scene="url" />
    </Suspense>
  );
}
```

---

## State Management

### Zustand Integration

```jsx
import { create } from 'zustand';

// Store
const useSceneStore = create((set) => ({
  selectedObject: null,
  setSelectedObject: (obj) => set({ selectedObject: obj }),
  hoveredObject: null,
  setHoveredObject: (obj) => set({ hoveredObject: obj }),
  animationPlaying: false,
  toggleAnimation: () => set((state) => ({ animationPlaying: !state.animationPlaying })),
}));

// Component
function InteractiveSpline() {
  const setSelectedObject = useSceneStore((state) => state.setSelectedObject);
  const selectedObject = useSceneStore((state) => state.selectedObject);

  function onLoad(spline) {
    const cube = spline.findObjectByName('Cube');

    spline.addEventListener('mouseDown', (e) => {
      setSelectedObject(e.target);
    });
  }

  return (
    <>
      {selectedObject && (
        <div className="info-panel">
          Selected: {selectedObject.name}
        </div>
      )}
      <Spline scene="url" onLoad={onLoad} />
    </>
  );
}
```

### Context API

```jsx
// SplineContext.js
import { createContext, useContext, useRef } from 'react';

const SplineContext = createContext(null);

export function SplineProvider({ children }) {
  const splineRef = useRef(null);

  const setSpline = (spline) => {
    splineRef.current = spline;
  };

  const findObject = (name) => {
    return splineRef.current?.findObjectByName(name);
  };

  return (
    <SplineContext.Provider value={{ splineRef, setSpline, findObject }}>
      {children}
    </SplineContext.Provider>
  );
}

export function useSplineContext() {
  return useContext(SplineContext);
}

// Usage
function App() {
  return (
    <SplineProvider>
      <SplineScene />
      <Controls />
    </SplineProvider>
  );
}

function Controls() {
  const { findObject } = useSplineContext();

  function moveObject() {
    const obj = findObject('Cube');
    if (obj) {
      obj.position.x += 10;
    }
  }

  return <button onClick={moveObject}>Move</button>;
}
```

---

## Performance Optimization

### Render on Demand

```jsx
// Only render when scene changes
<Spline
  scene="url"
  renderOnDemand={true}  // Default: true
/>
```

### React.memo for Static Scenes

```jsx
const MemoizedSpline = React.memo(({ scene }) => (
  <Spline scene={scene} />
));

// Only re-render when scene prop changes
```

### Debounced Updates

```jsx
function useDebouncedScene(scene, delay = 300) {
  const [debouncedScene, setDebouncedScene] = useState(scene);

  useEffect(() => {
    const handler = setTimeout(() => {
      setDebouncedScene(scene);
    }, delay);

    return () => clearTimeout(handler);
  }, [scene, delay]);

  return debouncedScene;
}

function OptimizedSpline({ scene }) {
  const debouncedScene = useDebouncedScene(scene);
  return <Spline scene={debouncedScene} />;
}
```

### Resource Cleanup

```jsx
function SplineWithCleanup({ scene }) {
  const [spline, setSpline] = useState(null);

  useEffect(() => {
    return () => {
      // Cleanup on unmount
      if (spline) {
        spline.dispose();
      }
    };
  }, [spline]);

  return (
    <Spline
      scene={scene}
      onLoad={(s) => setSpline(s)}
    />
  );
}
```

---

## TypeScript Support

### Type Definitions

```typescript
// Import types
import type { Application, SPEObject, SplineEvent } from '@splinetool/runtime';
import Spline from '@splinetool/react-spline';

// Typed component
function TypedSpline() {
  const onLoad = (spline: Application) => {
    const cube: SPEObject | null = spline.findObjectByName('Cube');

    if (cube) {
      console.log(cube.position.x);
      console.log(cube.rotation.y);
    }
  };

  const onSplineMouseDown = (e: SplineEvent) => {
    console.log('Target:', e.target.name);
    console.log('Point:', e.intersectionPoint);
  };

  return (
    <Spline
      scene="url"
      onLoad={onLoad}
      onSplineMouseDown={onSplineMouseDown}
    />
  );
}
```

### Custom Hook Types

```typescript
// r3f-spline with types
import useSpline from '@splinetool/r3f-spline';
import type { Object3D, Material } from 'three';

interface SplineNodes {
  Cube: Object3D;
  Sphere: Object3D;
  [key: string]: Object3D;
}

interface SplineMaterials {
  'Cube Material': Material;
  'Sphere Material': Material;
  [key: string]: Material;
}

function TypedScene() {
  const { nodes, materials } = useSpline('url') as {
    nodes: SplineNodes;
    materials: SplineMaterials;
  };

  return (
    <group dispose={null}>
      <mesh geometry={nodes.Cube.geometry} material={materials['Cube Material']} />
    </group>
  );
}
```

---

## References

1. **react-spline Documentation** - https://github.com/splinetool/react-spline
2. **React Three Fiber** - https://github.com/pmndrs/react-three-fiber
3. **Drei Helpers** - https://github.com/pmndrs/drei
4. **Spline Documentation** - https://docs.spline.design/
