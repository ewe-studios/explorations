# PlayCanvas Rendering Engine Deep Dive

## Overview

PlayCanvas features a modern rendering engine supporting both WebGL 2.0 and WebGPU backends. The rendering architecture is designed for performance, flexibility, and cross-browser compatibility.

---

## Rendering Architecture Diagram

```
┌─────────────────────────────────────────────────────────────────────────┐
│                         Application Layer                               │
│                           (Application)                                 │
├─────────────────────────────────────────────────────────────────────────┤
│                          Scene Layer                                    │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐   │
│  │   Camera    │  │    Light    │  │    Model    │  │   Material  │   │
│  │  Component  │  │  Component  │  │  Component  │  │   System    │   │
│  └─────────────┘  └─────────────┘  └─────────────┘  └─────────────┘   │
├─────────────────────────────────────────────────────────────────────────┤
│                         Renderer Layer                                  │
│  ┌─────────────────────────────────────────────────────────────────┐   │
│  │                    ForwardRenderer                               │   │
│  │  ┌──────────────┐  ┌──────────────┐  ┌──────────────────────┐  │   │
│  │  │ Layer        │  │ MeshInstance │  │ Material             │  │   │
│  │  │ Composition  │  │ Culling      │  │ Shader Generation    │  │   │
│  │  └──────────────┘  └──────────────┘  └──────────────────────┘  │   │
│  └─────────────────────────────────────────────────────────────────┘   │
├─────────────────────────────────────────────────────────────────────────┤
│                      Graphics Device Layer                              │
│  ┌─────────────────────────┐  ┌─────────────────────────┐             │
│  │   WebGLGraphicsDevice   │  │   WebGPUGraphicsDevice  │             │
│  │   (webgl-graphics-)     │  │   (webgpu-graphics-)    │             │
│  │        device.js        │  │        device.js        │             │
│  └─────────────────────────┘  └─────────────────────────┘             │
├─────────────────────────────────────────────────────────────────────────┤
│                         Platform Layer                                  │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐   │
│  │   Shader    │  │   Texture   │  │   Buffer    │  │  Render     │   │
│  │             │  │             │  │  (VB/IB)    │  │   Target    │   │
│  └─────────────┘  └─────────────┘  └─────────────┘  └─────────────┘   │
└─────────────────────────────────────────────────────────────────────────┘
```

---

## Graphics Device Abstraction

### GraphicsDevice Base Class

**File:** `src/platform/graphics/graphics-device.js`

The `GraphicsDevice` class provides an abstraction layer over WebGL and WebGPU:

```javascript
class GraphicsDevice extends EventHandler {
    // Canvas element
    canvas;

    // Device capabilities
    maxTextureSize;
    maxCubeMapSize;
    maxVolumeSize;
    maxAnisotropy;
    maxColorAttachments;

    // Device type flags
    isWebGPU = false;
    isWebGL2 = false;
    isNull = false;
    isHdr = false;

    // Scope namespace for shader variables
    scope;

    // Core methods
    setViewport(x, y, w, h);
    setScissor(x, y, w, h);
    setRenderTarget(renderTarget);
    clear(options);

    // Drawing
    draw(primitiveType, numIndices, offset);
    drawIndexed(primitiveType, numIndices, offset);

    // State management
    setBlendState(state);
    setDepthState(state);
    setStencilState(state);
    setRasterState(state);

    // Resource creation
    createShader(shaderDesc);
    createTexture(textureDesc);
    createVertexBuffer(desc);
    createIndexBuffer(desc);
    createRenderTarget(desc);
}
```

### WebGL Backend

**File:** `src/platform/graphics/webgl/webgl-graphics-device.js`

Key features of the WebGL implementation:

```javascript
class WebGLGraphicsDevice extends GraphicsDevice {
    constructor(canvas, options) {
        super();
        this.gl = canvas.getContext('webgl2', options);

        // Extensions
        this.extColorFloat = gl.getExtension('EXT_color_buffer_float');
        this.extTextureFloat = gl.getExtension('OES_texture_float');
        this.extTextureFloatLinear = gl.getExtension('OES_texture_float_linear');
        this.extDrawBuffers = gl.getExtension('WEBGL_draw_buffers');

        // Capability queries
        this.maxTextureSize = gl.getParameter(gl.MAX_TEXTURE_SIZE);
        this.maxCubeMapSize = gl.getParameter(gl.MAX_CUBE_MAP_TEXTURE_SIZE);
        this.maxAnisotropy = gl.getExtension('EXT_texture_filter_anisotropic')?.[
            gl.getExtension('EXT_texture_filter_anisotropic').MAX_TEXTURE_MAX_ANISOTROPY_EXT
        ] || 1;

        // Shader precision
        const vertexHigh = gl.getShaderPrecisionFormat(gl.VERTEX_SHADER, gl.HIGH_FLOAT);
        this.precision = vertexHigh.precision !== 0 ? 'highp' : 'mediump';
    }

    // Shader creation
    createShader(shaderDesc) {
        const gl = this.gl;
        const program = gl.createProgram();

        // Compile vertex and fragment shaders
        const vertexShader = this._compileShader(
            shaderDesc.vshader,
            gl.VERTEX_SHADER,
            shaderDesc.name
        );
        const fragmentShader = this._compileShader(
            shaderDesc.fshader,
            gl.FRAGMENT_SHADER,
            shaderDesc.name
        );

        // Attach and link
        gl.attachShader(program, vertexShader);
        gl.attachShader(program, fragmentShader);

        // Bind attribute locations
        if (shaderDesc.attributes) {
            for (let i = 0; i < shaderDesc.attributes.length; i++) {
                gl.bindAttribLocation(program, i, shaderDesc.attributes[i]);
            }
        }

        gl.linkProgram(program);

        // Check link status
        if (!gl.getProgramParameter(program, gl.LINK_STATUS)) {
            throw new Error(gl.getProgramInfoLog(program));
        }

        return program;
    }

    // Texture creation
    createTexture(textureDesc) {
        const gl = this.gl;
        const texture = gl.createTexture();

        gl.bindTexture(gl.TEXTURE_2D, texture);

        // Set parameters
        gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_S, textureDesc.addressU);
        gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_T, textureDesc.addressV);
        gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MIN_FILTER, textureDesc.minFilter);
        gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MAG_FILTER, textureDesc.magFilter);

        // Upload data
        gl.texImage2D(
            gl.TEXTURE_2D,
            0,
            textureDesc.format,
            textureDesc.width,
            textureDesc.height,
            0,
            textureDesc.format,
            textureDesc.type,
            textureDesc.data
        );

        return texture;
    }
}
```

### WebGPU Backend

**File:** `src/platform/graphics/webgpu/webgpu-graphics-device.js`

WebGPU implementation with modern features:

```javascript
class WebGPUGraphicsDevice extends GraphicsDevice {
    constructor(canvas, options) {
        super();
        this.isWebGPU = true;

        // Request adapter and device
        this.adapter = navigator.gpu.requestAdapter(options.adapterOptions);
        this.device = this.adapter.requestDevice(options.deviceOptions);

        // Pipeline caches
        this.renderPipelines = new Map();
        this.computePipelines = new Map();
        this.bindGroupLayouts = new Map();
        this.pipelineLayouts = new Map();
    }

    // Render pipeline creation
    createRenderPipeline(descriptor) {
        const pipeline = this.device.createRenderPipeline({
            layout: descriptor.layout,
            vertex: {
                module: descriptor.shader.vertex,
                entryPoint: descriptor.vertexEntryPoint || 'main',
                buffers: descriptor.vertexBuffers
            },
            fragment: {
                module: descriptor.shader.fragment,
                entryPoint: descriptor.fragmentEntryPoint || 'main',
                targets: descriptor.colorTargets
            },
            primitive: {
                topology: descriptor.topology,
                cullMode: descriptor.cullMode,
                frontFace: descriptor.frontFace
            },
            depthStencil: descriptor.depthStencil
        });

        return pipeline;
    }

    // Bind group for resource binding
    createBindGroup(layout, entries) {
        return this.device.createBindGroup({
            layout: layout,
            entries: entries
        });
    }

    // Compute pass
    dispatchCompute(pipeline, bindGroup, workgroupCount) {
        const commandEncoder = this.device.createCommandEncoder();
        const computePass = commandEncoder.beginComputePass();

        computePass.setPipeline(pipeline);
        computePass.setBindGroup(0, bindGroup);
        computePass.dispatchWorkgroups(...workgroupCount);
        computePass.end();

        this.queue.submit([commandEncoder.finish()]);
    }
}
```

---

## Shader System

### Shader Generation

PlayCanvas uses a shader generation system that creates shaders on-the-fly based on material and scene properties:

```javascript
// src/scene/shader-lib/programs/standard.js

class ProgramLibrary {
    constructor(device) {
        this.device = device;
        this.cache = new Map();
    }

    // Get or create shader from options
    getShader(material, scene, options) {
        const key = this._generateKey(options);

        let shader = this.cache.get(key);
        if (!shader) {
            shader = this._createShader(options);
            this.cache.set(key, shader);
        }

        return shader;
    }

    _generateKey(options) {
        // Generate unique key based on enabled features
        let key = '';
        for (const [name, value] of Object.entries(options)) {
            if (value) {
                key += `${name}_`;
            }
        }
        return key;
    }
}
```

### Shader Chunks

The shader system uses a chunk-based approach for code reuse:

```wgsl
// src/scene/shader-lib/wgsl/chunks/common/frag/lighting/main.wgsl

// Base lighting chunk
fn calculateLighting(
    normal: vec3<f32>,
    viewDir: vec3<f32>,
    uv: vec2<f32>
) -> vec3<f32> {
    var result = vec3<f32>(0.0);

    // Diffuse lighting
    result += calculateDiffuse(normal);

    // Specular lighting
    #if USE_SPECULAR
        result += calculateSpecular(normal, viewDir);
    #endif

    // Ambient lighting
    result += calculateAmbient();

    return result;
}

// Normal mapping chunk
fn calculateNormal(uv: vec2<f32>) -> vec3<f32> {
    #if USE_NORMAL_MAP
        let normalTex = textureSample(normalMap, normalSampler, uv);
        return normalize(normalTex.rgb * 2.0 - 1.0);
    #else
        return vec3<f32>(0.0, 0.0, 1.0);
    #endif
}
```

### Shader Pass System

Different rendering passes use different shader variants:

```javascript
// src/scene/shader-pass.js

class ShaderPass {
    constructor(name, options) {
        this.name = name;
        this.options = options;
    }
}

// Built-in passes
const SHADER_FORWARD = 'forward';        // Main color pass
const SHADER_DEPTH = 'depth';            // Depth-only pass
const SHADER_SHADOW = 'shadow';          // Shadow map pass
const SHADER_PICK = 'pick';              // Picking/pass
const SHADER_PREPASS = 'prepass';        // G-buffer pass (for deferred)
```

---

## Material System

### StandardMaterial

**File:** `src/scene/materials/standard-material.js`

The main material type with PBR support:

```javascript
class StandardMaterial extends Material {
    // Diffuse properties
    diffuse = new Color(1, 1, 1);
    diffuseMap = null;
    diffuseMapUv = 0;
    diffuseMapTiling = new Vec2(1, 1);
    diffuseMapOffset = new Vec2(0, 0);
    diffuseVertexColor = false;

    // Specular properties
    specular = new Color(1, 1, 1);
    specularMap = null;
    specularityFactor = 1.0;
    specularityFactorMap = null;

    // PBR properties
    metalness = 0.0;
    metalnessMap = null;
    roughness = 0.5;
    roughnessMap = null;

    // Normal mapping
    normalMap = null;
    normalMapFactor = 1.0;

    // Emissive
    emissive = new Color(0, 0, 0);
    emissiveMap = null;
    emissiveIntensity = 1.0;

    // Clear coat
    clearCoat = 0.0;
    clearCoatMap = null;
    clearCoatGlossiness = 1.0;

    // Transparency
    opacity = 1.0;
    opacityMap = null;
    blendType = BLEND_NONE;

    // Environment
    envMap = null;
    envMapIntensity = 1.0;

    constructor() {
        super();
        this.updateShader = this.onUpdateShader.bind(this);
    }

    // Generate shader options based on material state
    onUpdateShader(options) {
        options.litOptions.useDiffuseMap = !!this.diffuseMap;
        options.litOptions.useSpecularMap = !!this.specularMap;
        options.litOptions.useMetalnessMap = !!this.metalnessMap;
        options.litOptions.useRoughnessMap = !!this.roughnessMap;
        options.litOptions.useNormalMap = !!this.normalMap;
        options.litOptions.useEmissive = this.emissive.lengthSq() > 0;
        options.litOptions.useClearCoat = this.clearCoat > 0;
        options.litOptions.useOpacityMap = !!this.opacityMap;

        options.litOptions.blendType = this.blendType;
        options.litOptions.shadingModel = this.shadingModel;

        return options;
    }

    // Update the shader when material changes
    update() {
        // Trigger shader regeneration
        if (this.updateShader) {
            const options = new StandardMaterialOptions();
            this.updateShader(options);
            this.shader = this._getShader(options);
        }
    }
}
```

### Material Options

```javascript
// src/scene/materials/standard-material-options.js

class StandardMaterialOptions {
    // Lighting options
    useDiffuseMap = false;
    useSpecularMap = false;
    useMetalnessMap = false;
    useRoughnessMap = false;
    useNormalMap = false;
    useEmissive = false;
    useClearCoat = false;
    useOpacityMap = false;

    // Shading model
    shadingModel = 'standard';  // 'standard' | 'blinn' | 'phong' | 'unlit'

    // Lighting model
    lightingModel = 'lambert';  // 'lambert' | 'phong' | 'blinn' | 'ggx'

    // Shadow options
    receiveShadow = true;
    castShadow = true;

    // Fog
    fog = 'none';  // 'none' | 'linear' | 'exp' | 'exp2'

    // Gamma correction
    gammaCorrect = true;
    toneMap = 'aces2';  // 'linear' | 'aces2' | 'aces' | 'filmic' | 'hejl'
}
```

---

## Rendering Pipeline

### Forward Rendering

```javascript
// src/scene/renderer/forward-renderer.js

class ForwardRenderer {
    constructor(scene) {
        this.scene = scene;
        this.layerComposer = scene.layers;
    }

    render(camera, target) {
        const device = camera._device;
        const layers = this.layerComposer.layerList;

        // Setup render target
        device.setRenderTarget(target);
        device.updateBegin();

        // Render each layer
        for (let i = 0; i < layers.length; i++) {
            const layer = layers[i];
            if (!layer.enabled) continue;

            this.renderLayer(camera, layer);
        }

        device.updateEnd();
    }

    renderLayer(camera, layer) {
        const instances = layer.instances;
        const lights = layer._lights;

        // Cull visible mesh instances
        const visible = this.cull(camera, instances);

        // Sort by material and distance
        this.sortByMaterial(visible);

        // Render opaque objects
        for (const meshInstance of visible.opaque) {
            this.renderMeshInstance(camera, meshInstance, lights);
        }

        // Render transparent objects (back-to-front)
        visible.transparent.sort((a, b) => b.zDistance - a.zDistance);
        for (const meshInstance of visible.transparent) {
            this.renderMeshInstance(camera, meshInstance, lights);
        }
    }

    renderMeshInstance(camera, meshInstance, lights) {
        const device = camera._device;
        const material = meshInstance.material;
        const mesh = meshInstance.mesh;

        // Set shader parameters
        material.setParameters(device, meshInstance, camera, lights);

        // Bind geometry
        device.setVertexBuffer(mesh.vertexBuffer);
        if (mesh.indexBuffer) {
            device.setIndexBuffer(mesh.indexBuffer);
        }

        // Draw
        device.setShader(material.shader);
        device.draw(mesh.primitive[0]);
    }
}
```

### Layer Composition

```javascript
// src/scene/composition/layer.js

class Layer {
    constructor(options) {
        this.name = options.name;
        this.enabled = true;
        this opaqueSortMode = OPAQUE_SORT_NONE;
        this.transparentSortMode = SORT_NONE;

        // Render mesh instances
        this.instances = [];

        // Lights in this layer
        this._lights = [];

        // Render passes
        this.renderPasses = [];

        // Culling
        this.cullingMask = 0xFFFFFFFF;
    }

    addMeshInstances(meshInstances, skipWorldBoundsCheck = false) {
        for (const instance of meshInstances) {
            this.instances.push(instance);
            instance.layer = this;
        }
    }

    removeMeshInstances(meshInstances) {
        for (const instance of meshInstances) {
            const idx = this.instances.indexOf(instance);
            if (idx !== -1) {
                this.instances.splice(idx, 1);
            }
        }
    }

    addLights(lights) {
        this._lights.push(...lights);
    }
}

// Default layers
const WORLD_LAYER = new Layer({ name: 'World' });
const UI_LAYER = new Layer({ name: 'UI', opaqueSortMode: SORT_FRONT_TO_BACK });
const IMMEDIATE_LAYER = new Layer({ name: 'Immediate' });
```

---

## Lighting System

### Light Types

**File:** `src/scene/light.js`

```javascript
class Light extends EventHandler {
    constructor() {
        super();

        // Light type
        this._type = LIGHTTYPE_DIRECTIONAL;  // DIRECTIONAL | POINT | SPOT

        // Color and intensity
        this._color = new Color(1, 1, 1);
        this._intensity = 1.0;

        // Shadow properties
        this.castShadows = false;
        this.shadowResolution = 1024;
        this.shadowBias = 0.0005;
        this.shadowDistance = 40;

        // Spot light specific
        this._spotAngle = 45;
        this._spotBlur = 0.5;

        // Range (for point/spot lights)
        this._range = 10;

        // Masking
        this.mask = 1;
    }

    // Shadow generation
    updateShadowTexture() {
        if (this.castShadows && !this._shadowMap) {
            const size = this.shadowResolution;
            this._shadowMap = new Texture(this.device, {
                width: size,
                height: size,
                format: PIXELFORMAT_DEPTH,
                mipmaps: false,
                minFilter: FILTER_NEAREST,
                magFilter: FILTER_NEAREST,
                addressU: ADDRESS_CLAMP_TO_EDGE,
                addressV: ADDRESS_CLAMP_TO_EDGE
            });

            this._shadowRenderTarget = new RenderTarget({
                colorBuffer: null,
                depthBuffer: this._shadowMap
            });
        }
    }
}
```

### Lighting Buffer

**File:** `src/scene/lighting/lights-buffer.js`

Clustered lighting for efficient many-light rendering:

```javascript
class LightsBuffer {
    constructor() {
        // Light data stored in structured buffer
        this.lightData = new Float32Array(MAX_LIGHTS * LIGHT_DATA_SIZE);

        // Light indices per cluster
        this.clusterLightIndices = new Uint16Array(MAX_CLUSTERS * MAX_LIGHTS_PER_CLUSTER);
    }

    // Pack light data into buffer
    packLight(light, index) {
        const offset = index * LIGHT_DATA_SIZE;

        // Position/Direction (vec4)
        if (light.type === LIGHTTYPE_DIRECTIONAL) {
            this.lightData[offset + 0] = light.direction.x;
            this.lightData[offset + 1] = light.direction.y;
            this.lightData[offset + 2] = light.direction.z;
            this.lightData[offset + 3] = 0; // w = 0 for directional
        } else {
            this.lightData[offset + 0] = light.position.x;
            this.lightData[offset + 1] = light.position.y;
            this.lightData[offset + 2] = light.position.z;
            this.lightData[offset + 3] = 1; // w = 1 for point/spot
        }

        // Color (vec3)
        this.lightData[offset + 4] = light.color.r;
        this.lightData[offset + 5] = light.color.g;
        this.lightData[offset + 6] = light.color.b;

        // Intensity and type
        this.lightData[offset + 7] = light.intensity;
        this.lightData[offset + 8] = light.type;

        // Spot angles
        if (light.type === LIGHTTYPE_SPOT) {
            this.lightData[offset + 9] = Math.cos(light.spotAngle * 0.5 * DEG_TO_RAD);
            this.lightData[offset + 10] = Math.cos(light.spotAngle * 0.5 * DEG_TO_RAD * (1 - light.spotBlur));
        }

        // Range
        if (light.type !== LIGHTTYPE_DIRECTIONAL) {
            this.lightData[offset + 11] = 1.0 / light.range;
            this.lightData[offset + 12] = light.range * light.range;
        }
    }
}
```

### World Clusters

**File:** `src/scene/lighting/world-clusters.js`

```javascript
class WorldClusters {
    constructor() {
        // Cluster grid dimensions
        this.numClustersX = 16;
        this.numClustersY = 16;
        this.numClustersZ = 16;

        // Cluster bounds
        this.boundsMin = new Vec3(-50, -50, -50);
        this.boundsMax = new Vec3(50, 50, 50);

        // Cluster texture (3D texture storing light indices)
        this.clusterTexture = null;
    }

    // Build clusters from light positions
    build(lights, viewProjectionMatrix) {
        // Clear cluster data
        this.clearClusters();

        // Assign lights to clusters
        for (const light of lights) {
            if (light.type === LIGHTTYPE_DIRECTIONAL) {
                // Directional lights affect all clusters
                this.assignLightToAllClusters(light);
            } else {
                // Point/spot lights affect specific clusters based on range
                const affectedClusters = this.getAffectedClusters(light);
                for (const cluster of affectedClusters) {
                    this.addLightToCluster(cluster, light);
                }
            }
        }

        // Upload to GPU
        this.updateClusterTexture();
    }

    getAffectedClusters(light) {
        const affected = [];
        const lightSphere = light.getBoundingSphere();

        for (let z = 0; z < this.numClustersZ; z++) {
            for (let y = 0; y < this.numClustersY; y++) {
                for (let x = 0; x < this.numClustersX; x++) {
                    const clusterBounds = this.getClusterBounds(x, y, z);
                    if (this.intersectSphereAABB(lightSphere, clusterBounds)) {
                        affected.push({ x, y, z });
                    }
                }
            }
        }

        return affected;
    }
}
```

---

## Mesh and Geometry

### Mesh Class

**File:** `src/scene/mesh.js`

```javascript
class Mesh {
    constructor(device) {
        this.device = device;

        // Geometry
        this.vertexBuffer = null;
        this.indexBuffer = null;

        // Primitive info
        this.primitive = [{
            type: PRIMITIVE_TRIANGLES,
            base: 0,
            count: 0,
            indexed: true
        }];

        // AABB for culling
        this.aabb = new BoundingBox();

        // Skin for animation
        this.skin = null;

        // Morph targets
        this.morph = null;

        // Caching
        this._geometryData = new GeometryData();
    }

    // Simple API for setting positions
    setPositions(positions) {
        this._geometryData.positions = positions;
        this._changeVertexCount(positions.length / 3, 'position');
    }

    // Set normals
    setNormals(normals) {
        this._geometryData.normals = normals;
    }

    // Set UV coordinates
    setUvs(channel, uvs) {
        this._geometryData.uvs = this._geometryData.uvs || [];
        this._geometryData.uvs[channel] = uvs;
    }

    // Set indices
    setIndices(indices) {
        this._geometryData.indices = indices;
        this.indexCount = indices.length;
    }

    // Build the mesh
    update() {
        const data = this._geometryData;
        const device = this.device;

        // Build vertex format
        const formatDesc = [];
        if (data.positions) {
            formatDesc.push({
                semantic: SEMANTIC_POSITION,
                components: 3,
                type: TYPE_FLOAT32
            });
        }
        if (data.normals) {
            formatDesc.push({
                semantic: SEMANTIC_NORMAL,
                components: 3,
                type: TYPE_FLOAT32
            });
        }
        if (data.uvs && data.uvs[0]) {
            formatDesc.push({
                semantic: SEMANTIC_TEXCOORD0,
                components: 2,
                type: TYPE_FLOAT32
            });
        }

        // Create vertex format
        const vertexFormat = new VertexFormat(device, formatDesc);

        // Create vertex buffer
        this.vertexBuffer?.destroy();
        this.vertexBuffer = new VertexBuffer(
            device,
            vertexFormat,
            this.vertexCount,
            { usage: data.verticesUsage }
        );

        // Lock and fill vertex buffer
        const dst = this.vertexBuffer.lock();
        let offset = 0;

        if (data.positions) {
            for (let i = 0; i < this.vertexCount; i++) {
                dst[offset++] = data.positions[i * 3 + 0];
                dst[offset++] = data.positions[i * 3 + 1];
                dst[offset++] = data.positions[i * 3 + 2];
            }
        }

        // ... fill other attributes

        this.vertexBuffer.unlock();

        // Create index buffer if needed
        if (data.indices) {
            this.indexBuffer?.destroy();
            this.indexBuffer = new IndexBuffer(
                device,
                INDEXFORMAT_UINT16,
                data.indices,
                data.indicesUsage
            );

            this.primitive[0].count = data.indices.length;
            this.primitive[0].indexed = true;
        }

        // Calculate AABB
        this._calculateAabb();
    }
}
```

### MeshInstance

**File:** `src/scene/mesh-instance.js`

```javascript
class MeshInstance {
    constructor(mesh, material, node) {
        this.mesh = mesh;
        this.material = material;
        this.node = node;

        // Rendering options
        this.castShadow = true;
        this.receiveShadow = true;
        this.cull = true;

        // Rendering mask
        this.mask = 1;

        // Render style
        this.renderStyle = RENDERSTYLE_SOLID;

        // Parameters (uniform overrides)
        this.parameters = {};

        // Draw order override
        this.drawOrder = null;

        // Skin instance for animated meshes
        this.skinInstance = null;

        // Morph instance for morph targets
        this.morphInstance = null;
    }

    // Get world space AABB
    getAabb() {
        if (this.mesh.aabb) {
            const nodeMat = this.node.getWorldTransform();
            return this.mesh.aabb.clone().transform(nodeMat);
        }
        return null;
    }

    // Set custom shader parameter
    setParameter(name, value) {
        this.parameters[name] = {
            value: value,
            scopeId: null  // Resolved at render time
        };
    }
}
```

---

## Post-Processing

### Render Target

**File:** `src/platform/graphics/render-target.js`

```javascript
class RenderTarget {
    constructor(options) {
        this.colorBuffers = [];
        this.depthBuffer = options.depthBuffer;

        if (options.colorBuffer) {
            this.colorBuffers.push(options.colorBuffer);
        } else if (options.colorBuffers) {
            this.colorBuffers = options.colorBuffers;
        }

        this.width = this.colorBuffers[0]?.width || options.width;
        this.height = this.colorBuffers[0]?.height || options.height;
    }

    destroy() {
        for (const colorBuffer of this.colorBuffers) {
            colorBuffer.destroy();
        }
        if (this.depthBuffer) {
            this.depthBuffer.destroy();
        }
    }
}
```

### Post-Processing Effect

```javascript
// Example post-processing effect

class PostEffect {
    constructor(device, vertexShader, fragmentShader) {
        this.device = device;

        // Create fullscreen quad vertex buffer
        this.vertexBuffer = this._createFullscreenQuad();

        // Create shader
        this.shader = device.createShader({
            vshader: vertexShader,
            fshader: fragmentShader,
            name: 'PostEffect'
        });
    }

    _createFullscreenQuad() {
        const format = new VertexFormat(this.device, [
            { semantic: SEMANTIC_POSITION, components: 2, type: TYPE_FLOAT32 }
        ]);

        const vertices = new Float32Array([
            -1, -1,
             1, -1,
            -1,  1,
             1,  1
        ]);

        const vb = new VertexBuffer(this.device, format, 4, {
            usage: BUFFER_STATIC
        });

        const dst = vb.lock();
        dst.set(vertices);
        vb.unlock();

        return vb;
    }

    render(inputTexture, outputRenderTarget) {
        const device = this.device;

        device.setRenderTarget(outputRenderTarget);
        device.updateBegin();

        device.setViewport(0, 0, outputRenderTarget.width, outputRenderTarget.height);
        device.setScissor(0, 0, outputRenderTarget.width, outputRenderTarget.height);

        device.setVertexBuffer(this.vertexBuffer);
        device.setShader(this.shader);

        // Set input texture
        const scopeId = device.scope.resolve('uInputTexture');
        scopeId.setValue(inputTexture);

        device.draw(PRIMITIVE_TRIFAN, 4);

        device.updateEnd();
    }
}

// Usage - Bloom effect
const bloomEffect = new PostEffect(device, bloomVS, bloomFS);
bloomEffect.setParameter('threshold', 0.8);
bloomEffect.setParameter('intensity', 1.5);
bloomEffect.render(sceneTexture, outputRenderTarget);
```

---

## Performance Optimizations

### 1. Batching

```javascript
// src/scene/batching/batch-manager.js

class BatchManager {
    // Group mesh instances with same material
    groupByMaterial(meshInstances) {
        const groups = new Map();

        for (const mi of meshInstances) {
            if (!mi.castShadow && mi.renderStyle === RENDERSTYLE_SOLID) {
                const key = mi.material._id;
                if (!groups.has(key)) {
                    groups.set(key, []);
                }
                groups.get(key).push(mi);
            }
        }

        return groups;
    }

    // Merge geometries
    mergeMeshes(meshInstances) {
        // Combine vertex buffers
        // Combine index buffers with offset
        // Create new merged mesh
        // Return batch mesh instance
    }
}
```

### 2. Instancing

```javascript
class Instancing {
    constructor(meshInstance, instances) {
        this.meshInstance = meshInstance;
        this.instances = instances;

        // Create instance buffer
        this.instanceData = new Float32Array(instances.length * 16); // 4x4 matrices
        this.instanceBuffer = null;
    }

    updateMatrices() {
        let offset = 0;
        for (const instance of this.instances) {
            const matrix = instance.node.getWorldTransform().data;
            for (let i = 0; i < 16; i++) {
                this.instanceData[offset++] = matrix[i];
            }
        }

        // Upload to GPU
        if (!this.instanceBuffer) {
            this.instanceBuffer = new VertexBuffer(
                this.device,
                instanceFormat,
                this.instances.length,
                { usage: BUFFER_DYNAMIC }
            );
        }

        const dst = this.instanceBuffer.lock();
        dst.set(this.instanceData);
        this.instanceBuffer.unlock();
    }
}
```

### 3. Frustum Culling

```javascript
// src/scene/camera.js

class Camera {
    // Extract frustum planes
    extractFrustumPlanes(viewProjectionMatrix) {
        const planes = this.frustumPlanes;

        // Left plane
        planes[0].x = viewProjectionMatrix[3] + viewProjectionMatrix[0];
        planes[0].y = viewProjectionMatrix[7] + viewProjectionMatrix[4];
        planes[0].z = viewProjectionMatrix[11] + viewProjectionMatrix[8];
        planes[0].w = viewProjectionMatrix[15] + viewProjectionMatrix[12];

        // Right plane
        planes[1].x = viewProjectionMatrix[3] - viewProjectionMatrix[0];
        // ... etc for all 6 planes

        // Normalize
        for (let i = 0; i < 6; i++) {
            const len = planes[i].length();
            planes[i].divide(len);
        }
    }

    // Test AABB against frustum
    isVisible(aabb) {
        for (let i = 0; i < 6; i++) {
            // Get the corner of the box that is most in the negative direction of the normal
            const x = this.frustumPlanes[i].x < 0 ? aabb.min.x : aabb.max.x;
            const y = this.frustumPlanes[i].y < 0 ? aabb.min.y : aabb.max.y;
            const z = this.frustumPlanes[i].z < 0 ? aabb.min.z : aabb.max.z;

            if (this.frustumPlanes[i].x * x +
                this.frustumPlanes[i].y * y +
                this.frustumPlanes[i].z * z +
                this.frustumPlanes[i].w < 0) {
                return false;
            }
        }
        return true;
    }
}
```

---

## WebGPU-Specific Features

### Compute Shaders

```wgsl
// src/scene/shader-lib/wgsl/chunks/particle/comp/particle-update.wgsl

@group(0) @binding(0)
var<storage, read_write> positions: array<vec3<f32>>;

@group(0) @binding(1)
var<storage, read_write> velocities: array<vec3<f32>>;

@group(0) @binding(2)
var<uniform> params: Params {
    deltaTime: f32,
    gravity: vec3<f32>,
};

@compute @workgroup_size(64)
fn updateParticle(@builtin(global_invocation_id) id: vec3<u32>) {
    let index = id.x;
    if (index >= arrayLength(&positions)) {
        return;
    }

    var vel = velocities[index];
    vel += params.gravity * params.deltaTime;

    var pos = positions[index];
    pos += vel * params.deltaTime;

    positions[index] = pos;
    velocities[index] = vel;
}
```

### Indirect Rendering

```javascript
class IndirectRenderer {
    constructor(device) {
        this.device = device;
        this.indirectBuffer = device.createBuffer({
            usage: GPUBufferUsage.INDIRECT | GPUBufferUsage.COPY_DST,
            size: INDIRECT_DRAW_SIZE * MAX_DRAWS
        });
    }

    // Build indirect draw commands
    buildDrawCommands(meshInstances) {
        const commands = new Uint32Array(meshInstances.length * 5);
        let offset = 0;

        for (const mi of meshInstances) {
            commands[offset + 0] = mi.mesh.primitive[0].count;  // vertex count
            commands[offset + 1] = 1;                            // instance count
            commands[offset + 2] = mi.mesh.primitive[0].base;   // first vertex
            commands[offset + 3] = 0;                            // first instance
            commands[offset + 4] = 0;                            // base instance (WebGPU)
            offset += 5;
        }

        // Upload to GPU
        this.device.queue.writeBuffer(this.indirectBuffer, 0, commands);
    }

    // Execute indirect draws
    drawIndirect(renderPass, pipeline, count) {
        renderPass.setPipeline(pipeline);
        for (let i = 0; i < count; i++) {
            renderPass.drawIndirect(this.indirectBuffer, i * INDIRECT_DRAW_SIZE);
        }
    }
}
```

---

## Debug Tools

### Graphics Debugging

```javascript
class DebugGraphics {
    // Enable wireframe mode
    static setWireframe(enabled) {
        // Sets global wireframe flag
    }

    // Show bounding boxes
    static renderBounds(bounds, color = Color.RED) {
        // Draws wireframe box
    }

    // Show frustum
    static renderFrustum(camera) {
        // Draws camera frustum lines
    }

    // Show light volumes
    static renderLightVolume(light) {
        // Draws light influence volume
    }
}
```

---

## Summary

The PlayCanvas rendering engine provides:

1. **Multi-backend support**: WebGL 2 and WebGPU with shared high-level API
2. **PBR materials**: Standard material with physically-based rendering
3. **Advanced lighting**: Clustered forward rendering supporting many lights
4. **Shadow mapping**: Cascaded shadows for directional lights, shadows for spot/point lights
5. **Post-processing**: Render target-based post-processing effects
6. **Optimization**: Batching, instancing, frustum culling, occlusion culling
7. **Modern features**: Compute shaders, indirect rendering (WebGPU)
