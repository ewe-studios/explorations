# PlayCanvas Production-Grade Patterns

## Overview

This document covers production considerations when using or building a game engine like PlayCanvas, including performance optimization, memory management, debugging, and deployment strategies.

---

## Table of Contents

1. [Performance Optimization](#performance-optimization)
2. [Memory Management](#memory-management)
3. [Debugging Tools](#debugging-tools)
4. [Asset Optimization](#asset-optimization)
5. [Loading Optimization](#loading-optimization)
6. [Cross-Platform Considerations](#cross-platform-considerations)
7. [Testing Strategies](#testing-strategies)
8. [Deployment](#deployment)

---

## Performance Optimization

### 1. Render Batching

**Static Batching**: Combine meshes with same material at build time.

```javascript
class BatchManager {
    // Group by material and combine
    batch(meshInstances) {
        const groups = this.groupByMaterial(meshInstances);
        const batches = [];

        for (const [material, instances] of groups) {
            if (instances.length > 1) {
                batches.push(this.mergeMeshes(instances));
            }
        }

        return batches;
    }

    mergeMeshes(instances) {
        // Combine vertex buffers
        // Combine index buffers with offsets
        // Create single draw call
    }
}
```

**Dynamic Batching**: Combine small meshes at runtime (CPU overhead).

**GPU Instancing**: Draw same mesh multiple times with different transforms.

```javascript
class Instancer {
    constructor(mesh, material) {
        this.mesh = mesh;
        this.material = material;
        this.instances = [];
        this.instanceBuffer = null;
    }

    addInstance(node) {
        this.instances.push(node);
        this._updateInstanceBuffer();
    }

    render(device) {
        // Set instance buffer
        device.setVertexBuffer(this.instanceBuffer, 1);

        // Draw instanced
        device.drawInstanced(
            this.mesh.primitive[0],
            this.instances.length
        );
    }
}
```

### 2. Level of Detail (LOD)

```javascript
class LODGroup {
    constructor(entity, lodLevels) {
        this.entity = entity;
        this.lods = lodLevels;  // [{ distance, mesh, error }]
        this.currentLod = 0;
    }

    update(cameraPosition) {
        const distance = this.entity.getPosition().distance(cameraPosition);

        // Find appropriate LOD
        for (let i = this.lods.length - 1; i >= 0; i--) {
            if (distance >= this.lods[i].distance) {
                if (this.currentLod !== i) {
                    this.setLod(i);
                }
                break;
            }
        }
    }

    setLod(index) {
        const lod = this.lods[index];

        // Swap mesh
        this.entity.model.mesh = lod.mesh;
        this.currentLod = index;

        // Log LOD transition
        console.log(`LOD changed to ${index} (distance: ${lod.distance})`);
    }
}

// Usage
const lodGroup = new LODGroup(entity, [
    { distance: 0, mesh: highPolyMesh, error: 0 },
    { distance: 10, mesh: mediumPolyMesh, error: 0.01 },
    { distance: 50, mesh: lowPolyMesh, error: 0.05 }
]);
```

### 3. Occlusion Culling

```javascript
class OcclusionCuller {
    constructor(scene) {
        this.scene = scene;
        this.occlusionQuery = null;
        this.visibleObjects = [];
    }

    // Use hardware occlusion queries
    cull(camera) {
        const objects = this.scene.objects;

        for (const obj of objects) {
            if (!this.isInFrustum(camera, obj.bounds)) {
                continue;  // Frustum culled
            }

            if (this.isOccluded(obj)) {
                obj.visible = false;
            } else {
                obj.visible = true;
                this.visibleObjects.push(obj);
            }
        }
    }

    // Software occlusion (simplified)
    isOccluded(object) {
        // Cast rays from camera to object corners
        // If all rays hit something closer, object is occluded
        const rays = this.getCornerRays(object.bounds);

        for (const ray of rays) {
            const hit = this.raycast(ray);
            if (!hit || hit.distance >= ray.origin.distance(object.center)) {
                return false;  // At least one corner visible
            }
        }

        return true;  // All corners occluded
    }
}
```

### 4. Frustum Culling

```javascript
class Frustum {
    constructor() {
        this.planes = new Array(6).fill().map(() => new Vec4());
    }

    update(viewProjectionMatrix) {
        // Extract frustum planes from VP matrix
        // Left plane
        this.planes[0].set(
            viewProjectionMatrix[3] + viewProjectionMatrix[0],
            viewProjectionMatrix[7] + viewProjectionMatrix[4],
            viewProjectionMatrix[11] + viewProjectionMatrix[8],
            viewProjectionMatrix[15] + viewProjectionMatrix[12]
        );

        // Right plane
        this.planes[1].set(
            viewProjectionMatrix[3] - viewProjectionMatrix[0],
            viewProjectionMatrix[7] - viewProjectionMatrix[4],
            viewProjectionMatrix[11] - viewProjectionMatrix[8],
            viewProjectionMatrix[15] - viewProjectionMatrix[12]
        );

        // ... Top, Bottom, Near, Far

        // Normalize planes
        for (let i = 0; i < 6; i++) {
            const plane = this.planes[i];
            const len = Math.sqrt(plane.x**2 + plane.y**2 + plane.z**2);
            plane.divide(len);
        }
    }

    isVisible(aabb) {
        for (let i = 0; i < 6; i++) {
            const plane = this.planes[i];

            // Get corner most in negative plane direction
            const corner = new Vec3(
                plane.x < 0 ? aabb.min.x : aabb.max.x,
                plane.y < 0 ? aabb.min.y : aabb.max.y,
                plane.z < 0 ? aabb.min.z : aabb.max.z
            );

            if (plane.dot(corner) + plane.w < 0) {
                return false;  // Outside plane
            }
        }

        return true;
    }
}
```

### 5. Fixed Time Step

```javascript
class GameLoop {
    constructor() {
        this.fixedTimeStep = 1 / 60;  // 60 Hz physics
        this.accumulator = 0;
        this.lastTime = performance.now();
        this.maxFrameTime = 0.25;  // Prevent spiral of death
    }

    loop(currentTime) {
        let deltaTime = (currentTime - this.lastTime) / 1000;
        this.lastTime = currentTime;

        // Clamp delta time
        deltaTime = Math.min(deltaTime, this.maxFrameTime);

        this.accumulator += deltaTime;

        // Fixed step updates (physics, etc.)
        while (this.accumulator >= this.fixedTimeStep) {
            this.fixedUpdate(this.fixedTimeStep);
            this.accumulator -= this.fixedTimeStep;
        }

        // Variable step updates (rendering, input)
        const alpha = this.accumulator / this.fixedTimeStep;
        this.update(deltaTime);
        this.render(alpha);

        requestAnimationFrame((t) => this.loop(t));
    }

    fixedUpdate(dt) {
        // Physics simulation
        // Deterministic systems
    }

    update(dt) {
        // Game logic
        // Input handling
        // Animation
    }

    render(alpha) {
        // Interpolate for smooth rendering
        // Draw frame
    }
}
```

---

## Memory Management

### 1. Object Pooling

```javascript
class ObjectPool {
    constructor(createFn, resetFn, initialSize = 100) {
        this.createFn = createFn;
        this.resetFn = resetFn;
        this.pool = [];
        this.inUse = new Set();

        // Pre-allocate
        for (let i = 0; i < initialSize; i++) {
            this.pool.push(createFn());
        }
    }

    acquire() {
        let obj;

        if (this.pool.length > 0) {
            obj = this.pool.pop();
        } else {
            obj = this.createFn();
        }

        this.inUse.add(obj);
        return obj;
    }

    release(obj) {
        if (this.inUse.has(obj)) {
            this.resetFn(obj);
            this.inUse.delete(obj);
            this.pool.push(obj);
        }
    }

    releaseAll() {
        for (const obj of this.inUse) {
            this.resetFn(obj);
            this.pool.push(obj);
        }
        this.inUse.clear();
    }
}

// Example: Vector pool
const vec3Pool = new ObjectPool(
    () => new Vec3(),
    (v) => v.set(0, 0, 0)
);

// Usage
const temp = vec3Pool.acquire();
// ... use temp
vec3Pool.release(temp);
```

### 2. Memory Budget

```javascript
class MemoryManager {
    constructor() {
        this.budgets = {
            textures: 256 * 1024 * 1024,  // 256 MB
            meshes: 128 * 1024 * 1024,    // 128 MB
            audio: 64 * 1024 * 1024,      // 64 MB
            scripts: 32 * 1024 * 1024     // 32 MB
        };

        this.usage = {
            textures: 0,
            meshes: 0,
            audio: 0,
            scripts: 0
        };
    }

    canAllocate(type, size) {
        return this.usage[type] + size <= this.budgets[type];
    }

    allocate(type, size) {
        if (!this.canAllocate(type, size)) {
            // Trigger garbage collection or unload assets
            this.gc(type);
        }

        this.usage[type] += size;
    }

    deallocate(type, size) {
        this.usage[type] -= size;
    }

    gc(type) {
        // Unload least recently used assets
        console.warn(`Memory budget exceeded for ${type}, triggering GC`);
    }
}
```

### 3. Asset Unloading

```javascript
class AssetManager {
    constructor() {
        this.assets = new Map();
        this.refCounts = new Map();
        this.lruQueue = [];
    }

    load(asset) {
        if (this.assets.has(asset.id)) {
            this.refCount(asset.id);
            return this.assets.get(asset.id);
        }

        // Load asset
        const resource = await this.loader.load(asset);
        this.assets.set(asset.id, resource);
        this.refCounts.set(asset.id, 1);
        this.lruQueue.push(asset.id);

        return resource;
    }

    refCount(id) {
        const count = this.refCounts.get(id) || 0;
        this.refCounts.set(id, count + 1);

        // Move to end of LRU queue
        const idx = this.lruQueue.indexOf(id);
        if (idx !== -1) {
            this.lruQueue.splice(idx, 1);
            this.lruQueue.push(id);
        }
    }

    release(id) {
        const count = this.refCounts.get(id) || 0;
        if (count <= 1) {
            this.refCounts.delete(id);
            this.assets.delete(id);

            const idx = this.lruQueue.indexOf(id);
            if (idx !== -1) {
                this.lruQueue.splice(idx, 1);
            }

            console.log(`Unloaded asset ${id}`);
        } else {
            this.refCounts.set(id, count - 1);
        }
    }

    // Unload least recently used assets to free memory
    unloadLRU(count = 1) {
        for (let i = 0; i < count && this.lruQueue.length > 0; i++) {
            const id = this.lruQueue.shift();
            const count = this.refCounts.get(id) || 0;
            if (count === 0) {
                this.assets.delete(id);
            }
        }
    }
}
```

---

## Debugging Tools

### 1. Performance Profiler

```javascript
class Profiler {
    constructor() {
        this.frames = [];
        this.currentFrame = null;
        this.enabled = false;
    }

    begin() {
        if (!this.enabled) return;

        this.currentFrame = {
            startTime: performance.now(),
            markers: []
        };
    }

    end() {
        if (!this.enabled || !this.currentFrame) return;

        this.currentFrame.endTime = performance.now();
        this.currentFrame.duration = this.currentFrame.endTime - this.currentFrame.startTime;

        this.frames.push(this.currentFrame);

        // Keep last 60 frames
        if (this.frames.length > 60) {
            this.frames.shift();
        }

        this.currentFrame = null;
    }

    mark(name) {
        if (!this.enabled || !this.currentFrame) return;

        this.currentFrame.markers.push({
            name,
            time: performance.now() - this.currentFrame.startTime
        });
    }

    beginScope(name) {
        this.mark(`BEGIN ${name}`);
    }

    endScope(name) {
        this.mark(`END ${name}`);
    }

    // Get average frame time
    getAverageFrameTime() {
        if (this.frames.length === 0) return 0;
        const total = this.frames.reduce((sum, f) => sum + f.duration, 0);
        return total / this.frames.length;
    }

    // Get FPS
    getFPS() {
        const avgTime = this.getAverageFrameTime();
        return avgTime > 0 ? 1000 / avgTime : 0;
    }

    // Export profile data
    export() {
        return JSON.stringify({
            frames: this.frames,
            avgFrameTime: this.getAverageFrameTime(),
            fps: this.getFPS()
        });
    }
}

// Usage
const profiler = new Profiler();

function gameLoop() {
    profiler.begin();
    profiler.beginScope('update');

    update();

    profiler.endScope('update');
    profiler.beginScope('render');

    render();

    profiler.endScope('render');
    profiler.end();

    // Display stats
    console.log(`FPS: ${profiler.getFPS().toFixed(1)}`);

    requestAnimationFrame(gameLoop);
}
```

### 2. Debug Rendering

```javascript
class DebugRenderer {
    constructor() {
        this.lines = [];
        this.texts = [];
        this.shapes = [];
    }

    drawLine(start, end, color = Color.RED, duration = 0) {
        this.lines.push({ start, end, color, duration });
    }

    drawBox(center, halfExtents, color = Color.GREEN, duration = 0) {
        // Add 12 lines for box edges
        const e = halfExtents;
        const corners = [
            new Vec3(center.x - e.x, center.y - e.y, center.z - e.z),
            new Vec3(center.x + e.x, center.y - e.y, center.z - e.z),
            // ... all 8 corners
        ];

        // Add edges
        this.drawLine(corners[0], corners[1], color, duration);
        this.drawLine(corners[1], corners[2], color, duration);
        // ... all 12 edges
    }

    drawText(text, position, color = Color.WHITE, duration = 0) {
        this.texts.push({ text, position, color, duration });
    }

    drawSphere(center, radius, color = Color.BLUE, duration = 0) {
        // Draw sphere using latitude/longitude lines
        // ...
    }

    // Render all debug geometry
    render(camera) {
        for (const line of this.lines) {
            this._drawLineImmediate(line.start, line.end, line.color);
        }

        for (const text of this.texts) {
            this._drawTextImmediate(text.text, text.position, text.color);
        }

        // Update durations
        this.lines = this.lines.filter(l => l.duration === 0 || l.duration-- > 0);
        this.texts = this.texts.filter(t => t.duration === 0 || t.duration-- > 0);
    }

    clear() {
        this.lines = [];
        this.texts = [];
        this.shapes = [];
    }
}

// Usage in game
debug.drawLine(player.getPosition(), targetPosition, Color.RED);
debug.drawBox(enemyBounds, Color.YELLOW);
debug.drawText(`Health: ${player.health}`, player.getPosition().add(0, 2, 0));
```

### 3. Memory Stats

```javascript
class MemoryStats {
    constructor() {
        this.peakUsage = 0;
        this.allocations = 0;
        this.deallocations = 0;
    }

    update() {
        if (performance.memory) {
            const usedMB = performance.memory.usedJSHeapSize / 1048576;
            const totalMB = performance.memory.totalJSHeapSize / 1048576;

            this.peakUsage = Math.max(this.peakUsage, usedMB);

            console.log(`Memory: ${usedMB.toFixed(1)} MB / ${totalMB.toFixed(1)} MB (peak: ${this.peakUsage.toFixed(1)} MB)`);
        }
    }

    trackAllocation(size) {
        this.allocations++;
    }

    trackDeallocation(size) {
        this.deallocations++;
    }
}
```

---

## Asset Optimization

### 1. Texture Compression

```javascript
// Choose optimal compression format per platform
function selectTextureFormat(gl) {
    // Check for ASTC (best quality/compression)
    if (gl.getExtension('WEBGL_compressed_texture_astc')) {
        return { format: 'ASTC_4x4', extension: 'ktx2' };
    }

    // Check for ETC2 (good for mobile)
    if (gl.getExtension('WEBGL_compressed_texture_etc')) {
        return { format: 'ETC2_RGBA8', extension: 'ktx2' };
    }

    // Check for S3TC/DXT (desktop)
    if (gl.getExtension('WEBGL_compressed_texture_s3tc')) {
        return { format: 'DXT5', extension: 'ktx2' };
    }

    // Fallback to uncompressed
    return { format: 'RGBA8', extension: 'png' };
}

// Basis Universal compression workflow
async function compressTextureBasis(sourceImage) {
    // Use basisu command line tool or encoder library
    const encoder = new BasisEncoder();

    encoder.setInput(sourceImage);
    encoder.setOutputFormat('KTX2');
    encoder.setQuality(128);  // 0-255
    encoder.generateMipmaps();

    const compressed = await encoder.encode();

    return {
        data: compressed,
        format: 'BASIS',
        width: sourceImage.width,
        height: sourceImage.height,
        mipmaps: true
    };
}
```

### 2. Mesh Optimization

```javascript
// Mesh simplification for LOD generation
function generateLODMesh(mesh, targetReduction) {
    // Use edge collapse simplification
    const simplifier = new MeshSimplifier();

    simplifier.setInput(mesh);
    simplifier.setTargetReduction(targetReduction);  // 0.5 = 50% reduction
    simplifier.preserveBorders(true);
    simplifier.preserveUVs(true);

    return simplifier.simplify();
}

// Vertex cache optimization for GPU
function optimizeVertexCache(mesh) {
    // Use Fresnel or STRPLE cache optimization
    const optimizer = new VertexCacheOptimizer();

    optimizer.setInput(mesh);
    optimizer.setCacheSize(32);  // Typical GPU vertex cache size

    return optimizer.optimize();
}

// Vertex fetch optimization
function optimizeVertexFetch(mesh) {
    // Reorder vertices for better vertex shader utilization
    const optimizer = new VertexFetchOptimizer();
    return optimizer.optimize(mesh);
}
```

### 3. Audio Optimization

```javascript
class AudioOptimizer {
    // Compress audio for web
    async compressAudio(audioBuffer, options = {}) {
        const {
            targetBitrate = 128,  // kbps
            format = 'mp3',
            sampleRate = 44100
        } = options;

        // Use Web Audio API or external encoder
        const encoder = new AudioEncoder({
            format,
            bitrate: targetBitrate * 1000,
            sampleRate
        });

        const compressed = await encoder.encode(audioBuffer);

        return {
            data: compressed,
            format,
            bitrate: targetBitrate,
            sampleRate,
            duration: audioBuffer.duration
        };
    }

    // Generate impulse responses for reverb
    generateImpulseResponse(duration = 2.0, decay = 2.0) {
        const sampleRate = 44100;
        const length = sampleRate * duration;
        const impulse = new Float32Array(length);

        for (let i = 0; i < length; i++) {
            const t = i / sampleRate;
            impulse[i] = (Math.random() * 2 - 1) * Math.exp(-decay * t);
        }

        return impulse;
    }
}
```

---

## Loading Optimization

### 1. Asset Bundling

```javascript
class BundleManager {
    constructor() {
        this.bundles = new Map();
        this.bundleMap = new Map();  // asset -> bundle
    }

    // Define bundles
    defineBundles(bundles) {
        for (const bundle of bundles) {
            this.bundles.set(bundle.name, {
                assets: bundle.assets,
                loaded: false,
                loading: false,
                promise: null
            });

            for (const assetId of bundle.assets) {
                this.bundleMap.set(assetId, bundle.name);
            }
        }
    }

    // Load bundle on demand
    async loadBundle(bundleName) {
        const bundle = this.bundles.get(bundleName);

        if (bundle.loaded) {
            return;
        }

        if (bundle.loading) {
            return bundle.promise;
        }

        bundle.loading = true;

        bundle.promise = Promise.all(
            bundle.assets.map(id => app.assets.load(id))
        ).then(() => {
            bundle.loaded = true;
            bundle.loading = false;
        });

        return bundle.promise;
    }

    // Unload bundle to free memory
    unloadBundle(bundleName) {
        const bundle = this.bundles.get(bundleName);

        if (bundle.loaded) {
            for (const assetId of bundle.assets) {
                app.assets.unload(assetId);
            }
            bundle.loaded = false;
        }
    }
}

// Usage
bundleManager.defineBundles([
    {
        name: 'level1',
        assets: [level1Model, level1Textures, level1Audio]
    },
    {
        name: 'level2',
        assets: [level2Model, level2Textures, level2Audio]
    }
]);

// Preload next level while playing current
async function onLevelComplete() {
    // Start loading next level in background
    bundleManager.loadBundle('level2');
}
```

### 2. Progressive Loading

```javascript
class ProgressiveLoader {
    constructor() {
        this.queue = [];
        this.loading = false;
        this.onProgress = null;
    }

    add(priority, asset) {
        this.queue.push({ priority, asset });
        this.queue.sort((a, b) => b.priority - a.priority);
    }

    async loadAll() {
        this.loading = true;
        let loaded = 0;
        const total = this.queue.length;

        while (this.queue.length > 0) {
            const item = this.queue.shift();
            await this.loadAsset(item.asset);

            loaded++;
            if (this.onProgress) {
                this.onProgress(loaded / total);
            }
        }

        this.loading = false;
    }

    async loadAsset(asset) {
        return new Promise((resolve, reject) => {
            asset.load((err) => {
                if (err) {
                    reject(err);
                } else {
                    resolve(asset);
                }
            });
        });
    }
}

// Priority levels
const PRIORITY = {
    CRITICAL: 100,   // Must have for game to function
    HIGH: 75,        // Needed soon
    NORMAL: 50,      // Standard loading
    LOW: 25          // Background loading
};

// Usage
const loader = new ProgressiveLoader();

loader.add(PRIORITY.CRITICAL, playerModel);
loader.add(PRIORITY.CRITICAL, playerTextures);
loader.add(PRIORITY.HIGH, levelGeometry);
loader.add(PRIORITY.NORMAL, backgroundMusic);
loader.add(PRIORITY.LOW, ambientSounds);

loader.onProgress = (progress) => {
    updateLoadingScreen(progress * 100);
};

loader.loadAll();
```

### 3. Streaming

```javascript
class AssetStreamer {
    constructor() {
        this.loadedAssets = new Set();
        this.streamingRadius = 50;  // meters
        this.unloadRadius = 100;    // meters
    }

    update(playerPosition, assetPositions) {
        for (const [assetId, position] of assetPositions) {
            const distance = playerPosition.distance(position);

            if (distance < this.streamingRadius && !this.loadedAssets.has(assetId)) {
                // Load asset
                this.loadAsset(assetId);
                this.loadedAssets.add(assetId);
            } else if (distance > this.unloadRadius && this.loadedAssets.has(assetId)) {
                // Unload asset
                this.unloadAsset(assetId);
                this.loadedAssets.delete(assetId);
            }
        }
    }

    async loadAsset(assetId) {
        console.log(`Streaming in: ${assetId}`);
        await app.assets.load(assetId);
    }

    unloadAsset(assetId) {
        console.log(`Streaming out: ${assetId}`);
        app.assets.unload(assetId);
    }
}
```

---

## Cross-Platform Considerations

### 1. Input Abstraction

```javascript
class InputManager {
    constructor() {
        this.actions = new Map();
        this.bindings = {
            keyboard: {},
            gamepad: {},
            touch: {}
        };

        this.setupDefaultBindings();
    }

    // Define logical actions
    defineAction(name, bindings) {
        this.actions.set(name, {
            value: 0,
            pressed: false,
            released: false,
            bindings
        });
    }

    setupDefaultBindings() {
        // Movement
        this.defineAction('move_forward', {
            keyboard: ['KeyW', 'ArrowUp'],
            gamepad: ['ButtonSouth'],  // X button
            touch: 'forward'
        });

        this.defineAction('move_backward', {
            keyboard: ['KeyS', 'ArrowDown'],
            gamepad: ['ButtonNorth'],  // A button
            touch: 'backward'
        });

        // Jump
        this.defineAction('jump', {
            keyboard: ['Space'],
            gamepad: ['ButtonEast'],  // B button
            touch: 'jump'
        });
    }

    update() {
        // Reset state
        for (const action of this.actions.values()) {
            action.pressed = false;
            action.released = false;
        }

        // Check keyboard
        for (const [key, actions] of Object.entries(this.bindings.keyboard)) {
            if (keyboard.isPressed(key)) {
                for (const actionName of actions) {
                    const action = this.actions.get(actionName);
                    action.value = 1;
                    action.pressed = true;
                }
            }
        }

        // Check gamepad
        // Check touch
    }

    getAction(name) {
        return this.actions.get(name);
    }

    isPressed(name) {
        return this.actions.get(name)?.pressed || false;
    }

    getValue(name) {
        return this.actions.get(name)?.value || 0;
    }
}
```

### 2. Graphics Quality Settings

```javascript
class QualitySettings {
    constructor() {
        this.presets = {
            low: {
                shadowResolution: 512,
                textureQuality: 0.5,
                antiAliasing: false,
                postProcessing: false,
                drawDistance: 50
            },
            medium: {
                shadowResolution: 1024,
                textureQuality: 0.75,
                antiAliasing: 'fxaa',
                postProcessing: true,
                drawDistance: 100
            },
            high: {
                shadowResolution: 2048,
                textureQuality: 1.0,
                antiAliasing: 'msaa4x',
                postProcessing: true,
                drawDistance: 200
            }
        };

        this.current = 'medium';
    }

    autoDetect() {
        // Detect capabilities and choose preset
        const gl = app.graphicsDevice.gl;

        const maxTextureSize = gl.getParameter(gl.MAX_TEXTURE_SIZE);
        const maxAnisotropy = gl.getExtension('EXT_texture_filter_anisotropic')?.[
            gl.MAX_TEXTURE_MAX_ANISOTROPY_EXT
        ] || 1;

        // Simple heuristic based on renderer
        const renderer = gl.getParameter(gl.RENDERER).toLowerCase();

        if (renderer.includes('intel') || renderer.includes('swiftshader')) {
            return 'low';
        }

        if (maxTextureSize >= 16384 && maxAnisotropy >= 16) {
            return 'high';
        }

        return 'medium';
    }

    apply(preset) {
        const settings = this.presets[preset];

        // Apply shadow resolution
        app.systems.light.shadowResolution = settings.shadowResolution;

        // Apply texture quality
        setTextureQuality(settings.textureQuality);

        // Apply AA
        app.graphicsDevice.setAntialias(settings.antiAliasing !== false);

        // Apply draw distance
        setCullingDistance(settings.drawDistance);
    }
}
```

### 3. Resolution Scaling

```javascript
class ResolutionScaler {
    constructor() {
        this.scale = 1.0;
        this.targetFPS = 60;
        this.adaptive = true;
        this.minScale = 0.5;
        this.maxScale = 1.0;
    }

    update(currentFPS, deltaTime) {
        if (!this.adaptive) return;

        if (currentFPS < this.targetFPS - 5 && this.scale > this.minScale) {
            // Lower resolution to improve FPS
            this.scale -= 0.05 * deltaTime;
            this.scale = Math.max(this.minScale, this.scale);
            this.apply();
        } else if (currentFPS > this.targetFPS + 5 && this.scale < this.maxScale) {
            // Increase resolution if we have FPS to spare
            this.scale += 0.05 * deltaTime;
            this.scale = Math.min(this.maxScale, this.scale);
            this.apply();
        }
    }

    apply() {
        const canvas = app.canvas;
        const rect = canvas.getBoundingClientRect();

        canvas.width = rect.width * this.scale;
        canvas.height = rect.height * this.scale;

        app.resizeCanvas();
    }
}
```

---

## Testing Strategies

### 1. Unit Testing

```javascript
// test/math/vec3.test.mjs
import { describe, it } from 'mocha';
import { expect } from 'chai';
import { Vec3 } from '../../src/core/math/vec3.js';

describe('Vec3', () => {
    describe('#add()', () => {
        it('should add two vectors correctly', () => {
            const a = new Vec3(1, 2, 3);
            const b = new Vec3(4, 5, 6);
            const result = a.add(b);

            expect(result.x).to.equal(5);
            expect(result.y).to.equal(7);
            expect(result.z).to.equal(9);
        });
    });

    describe('#length()', () => {
        it('should calculate vector length correctly', () => {
            const v = new Vec3(3, 4, 0);
            expect(v.length()).to.equal(5);
        });
    });

    describe('#normalize()', () => {
        it('should normalize vector to unit length', () => {
            const v = new Vec3(3, 4, 0);
            const normalized = v.clone().normalize();

            expect(normalized.length()).to.be.closeTo(1, 0.0001);
        });
    });
});
```

### 2. Visual Regression Testing

```javascript
// test/visual/regression.mjs
import { ScreenshotComparator } from './utils/screenshot-comparator.js';

describe('Visual Regression', () => {
    const comparator = new ScreenshotComparator({
        tolerance: 0.05,  // 5% difference allowed
    });

    it('should render box correctly', async () => {
        await loadScene('box-scene');

        const screenshot = await takeScreenshot();
        const baseline = await loadBaseline('box-scene-baseline.png');

        const result = comparator.compare(screenshot, baseline);

        if (!result.match) {
            await saveDiff(screenshot, baseline, result.diff, 'box-scene-diff.png');
            throw new Error(`Visual regression detected: ${result.differencePercent}% difference`);
        }
    });
});
```

---

## Deployment

### 1. Build Optimization

```javascript
// rollup.config.mjs for production
import swc from '@rollup/plugin-swc';
import terser from '@rollup/plugin-terser';
import strip from '@rollup/plugin-strip';

export default {
    input: 'src/index.js',
    output: {
        file: 'build/playcanvas.js',
        format: 'esm',
        sourcemap: false  // Don't include sourcemaps in production
    },
    plugins: [
        // Transpile with SWC (faster than Babel)
        swc({
            jsc: {
                minify: false,
                parser: {
                    syntax: 'ecmascript'
                }
            }
        }),

        // Strip debug code
        strip({
            functions: ['console.log', 'debug.*', 'assert.*']
        }),

        // Minify
        terser({
            compress: {
                drop_console: true,
                drop_debugger: true,
                pure_funcs: ['Debug.log', 'Debug.warn']
            },
            mangle: {
                reserved: ['Application', 'Entity', 'Component']  // Don't mangle public API
            }
        })
    ]
};
```

### 2. CDN Deployment

```javascript
// CDN configuration for assets
const CDN_CONFIG = {
    // Use different CDN for different regions
    regions: {
        'us-east': 'https://us-east.cdn.playcanvas.com',
        'eu-west': 'https://eu-west.cdn.playcanvas.com',
        'ap-east': 'https://ap-east.cdn.playcanvas.com'
    },

    // Asset versioning for cache busting
    version: '1.2.3',

    // Get optimal CDN endpoint
    getEndpoint() {
        // Use GeoDNS or detect region
        const region = detectRegion();
        return this.regions[region] || this.regions['us-east'];
    },

    // Asset URL with version
    assetUrl(path) {
        return `${this.getEndpoint()}/v${this.version}/${path}`;
    }
};

// Service Worker for offline support
self.addEventListener('install', (event) => {
    event.waitUntil(
        caches.open('playcanvas-v1').then((cache) => {
            return cache.addAll([
                '/engine/playcanvas.js',
                '/game/manifest.json',
                '/game/index.html'
            ]);
        })
    );
});

self.addEventListener('fetch', (event) => {
    event.respondWith(
        caches.match(event.request).then((response) => {
            return response || fetch(event.request);
        })
    );
});
```

### 3. Error Reporting

```javascript
class ErrorReporter {
    constructor(options) {
        this.endpoint = options.endpoint;
        this.userId = options.userId;
        this.sessionId = generateSessionId();
    }

    report(error, context = {}) {
        const report = {
            error: {
                name: error.name,
                message: error.message,
                stack: error.stack
            },
            context: {
                ...context,
                url: window.location.href,
                userAgent: navigator.userAgent,
                language: navigator.language
            },
            session: {
                id: this.sessionId,
                userId: this.userId
            },
            game: {
                version: GAME_VERSION,
                build: BUILD_NUMBER
            },
            timestamp: Date.now()
        };

        // Send to error reporting service
        navigator.sendBeacon(this.endpoint, JSON.stringify(report));
    }
}

// Global error handler
window.addEventListener('error', (event) => {
    errorReporter.report(event.error, {
        type: 'uncaught_exception',
        filename: event.filename,
        lineno: event.lineno,
        colno: event.colno
    });
});

// Promise rejection handler
window.addEventListener('unhandledrejection', (event) => {
    errorReporter.report(event.reason, {
        type: 'unhandled_rejection'
    });
});
```

---

## Summary

Production-grade game development requires attention to:

1. **Performance**: Batching, LOD, culling, fixed timestep
2. **Memory**: Object pooling, budgets, asset lifecycle
3. **Debugging**: Profilers, debug rendering, stats
4. **Assets**: Compression, optimization, streaming
5. **Loading**: Bundles, progressive loading, prioritization
6. **Cross-platform**: Input abstraction, quality settings, resolution scaling
7. **Testing**: Unit tests, visual regression, integration tests
8. **Deployment**: Build optimization, CDN, error reporting
