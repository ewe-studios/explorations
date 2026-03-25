# PlayCanvas Asset Pipeline Deep Dive

## Overview

PlayCanvas features a comprehensive asset pipeline supporting asynchronous loading, compression, and streaming. The system handles models, textures, animations, audio, scripts, and more.

---

## Architecture Diagram

```
┌─────────────────────────────────────────────────────────────────┐
│                      Asset Pipeline                              │
├─────────────────────────────────────────────────────────────────┤
│  ┌───────────────────────────────────────────────────────────┐ │
│  │                    AssetRegistry                           │ │
│  │  - Asset lookup by ID, name, URL                           │ │
│  │  - Asset lifecycle management                              │ │
│  │  - Event system (load, add, remove, error)                 │ │
│  └───────────────────────────────────────────────────────────┘ │
├─────────────────────────────────────────────────────────────────┤
│  ┌───────────────────────────────────────────────────────────┐ │
│  │                   ResourceLoader                           │ │
│  │  - Handles actual loading of resources                     │ │
│  │  - Manages bundle dependencies                             │ │
│  │  - Handles caching                                         │ │
│  └───────────────────────────────────────────────────────────┘ │
├─────────────────────────────────────────────────────────────────┤
│  ┌───────────────────────────────────────────────────────────┐ │
│  │                   ResourceHandlers                         │ │
│  │  ┌─────────┐ ┌─────────┐ ┌─────────┐ ┌─────────────────┐  │ │
│  │  │ Texture │  │  Model  │  │ Animation│  │   Container    │  │ │
│  │  │ Handler │  │ Handler │  │ Handler │  │    Handler     │  │ │
│  │  └─────────┘ └─────────┘ └─────────┘ └─────────────────┘  │ │
│  │  ┌─────────┐ ┌─────────┐ ┌─────────┐ ┌─────────────────┐  │ │
│  │  │  Audio  │  │ Script │  │  Font   │  │   Material     │  │ │
│  │  │ Handler │  │ Handler │  │ Handler │  │    Handler     │  │ │
│  │  └─────────┘ └─────────┘ └─────────┘ └─────────────────┘  │ │
│  └───────────────────────────────────────────────────────────┘ │
├─────────────────────────────────────────────────────────────────┤
│  ┌───────────────────────────────────────────────────────────┐ │
│  │                 Compression/Decompression                  │ │
│  │  - Basis Universal (texture compression)                   │ │
│  │  - Draco (geometry compression)                            │ │
│  │  - gzip/deflate (general compression)                      │ │
│  └───────────────────────────────────────────────────────────┘ │
└─────────────────────────────────────────────────────────────────┘
```

---

## Asset Registry

### AssetRegistry

**File:** `src/framework/asset/asset-registry.js`

```javascript
class AssetRegistry extends EventHandler {
    constructor(loader) {
        super();

        this.loader = loader;
        this._assets = [];
        this._assetsById = {};
        this._assetsByUid = {};
        this._assetsByName = {};
        this._tagsIndex = new TagsCache('_id');

        // Bundle support
        this._bundleRegistry = null;
    }

    // Add asset to registry
    add(asset) {
        if (this._assetsById[asset.id]) {
            throw new Error(`Asset with ID ${asset.id} already exists`);
        }

        this._assets.push(asset);
        this._assetsById[asset.id] = asset;

        if (asset.name) {
            if (!this._assetsByName[asset.name]) {
                this._assetsByName[asset.name] = [];
            }
            this._assetsByName[asset.name].push(asset);
        }

        if (asset.file && asset.file.url) {
            asset.on('change', this._onAssetChange, this);
            asset.on('remove', this._onAssetRemove, this);
        }

        this._tagsIndex.add(asset);

        this.fire('add', asset);
        this.fire('add:' + asset.id, asset);
    }

    // Get asset by ID
    get(id) {
        if (typeof id === 'number') {
            return this._assetsById[id];
        }
        return null;
    }

    // Get asset by name
    getByName(name) {
        const assets = this._assetsByName[name];
        return assets ? assets[0] : null;
    }

    // Get all assets by name
        getAllByName(name) {
        const assets = this._assetsByName[name];
        return assets ? assets.slice() : [];
    }

    // Find assets by tag
    findByTag(...tags) {
        return this._tagsIndex.findByTag(...tags);
    }

    // Filter assets by custom predicate
    filter(callback) {
        return this._assets.filter(callback);
    }

    // Load asset
    load(asset) {
        if (typeof asset === 'number') {
            asset = this._assetsById[asset];
        }

        if (!asset || asset.loaded) return Promise.resolve(asset.resource);

        return new Promise((resolve, reject) => {
            this.load(asset, (err, resource) => {
                if (err) {
                    reject(err);
                } else {
                    resolve(resource);
                }
            });
        });
    }

    // Load from URL
    loadFromUrl(url, type, callback) {
        // Check if already loaded
        const existing = this._assetsByUrl[url];
        if (existing) {
            if (existing.loaded) {
                callback(null, existing);
            } else {
                existing.once('load', () => callback(null, existing));
                existing.once('error', (err) => callback(err, null));
            }
            return;
        }

        // Create new asset
        const asset = new Asset(url, type, { url: url });
        this.add(asset);

        asset.once('load', () => callback(null, asset));
        asset.once('error', (err) => callback(err, null));

        this.load(asset);
    }

    // Unload asset
    unload(asset) {
        if (typeof asset === 'number') {
            asset = this._assetsById[asset];
        }

        if (!asset) return;

        if (asset.resource && asset.resource.destroy) {
            asset.resource.destroy();
        }

        asset.loaded = false;
        asset.resource = null;
    }

    // Remove asset from registry
    remove(asset) {
        if (typeof asset === 'number') {
            asset = this._assetsById[asset];
        }

        if (!asset) return;

        this.unload(asset);

        const idx = this._assets.indexOf(asset);
        if (idx !== -1) {
            this._assets.splice(idx, 1);
        }

        delete this._assetsById[asset.id];

        if (asset.name) {
            const assets = this._assetsByName[asset.name];
            if (assets) {
                const nameIdx = assets.indexOf(asset);
                if (nameIdx !== -1) {
                    assets.splice(nameIdx, 1);
                }
                if (assets.length === 0) {
                    delete this._assetsByName[asset.name];
                }
            }
        }

        this._tagsIndex.remove(asset);

        this.fire('remove', asset);
        this.fire('remove:' + asset.id, asset);
    }
}
```

### Asset Class

**File:** `src/framework/asset/asset.js`

```javascript
class Asset extends EventHandler {
    constructor(name, type, file, data) {
        super();

        this._id = null;
        this._name = name;
        this._type = type;
        this._file = file || null;
        this._data = data || {};

        this._loaded = false;
        this._loading = false;
        this._resource = null;
        this._resources = null;  // For multi-resource assets

        this._preload = false;
        this._priority = 0;

        // Localized asset support
        this._i18nKey = null;

        // Reference count for auto-unload
        this._refCount = 0;
    }

    // Load the asset
    load() {
        if (this._loaded || this._loading) return;

        this._loading = true;
        this.fire('load:start', this);

        this.registry.loader.load(this, (err, resource) => {
            this._loading = false;

            if (err) {
                this._error = err;
                this.fire('error', err, this);
                this.fire('load:error', err, this);
            } else {
                this._loaded = true;
                this._resource = resource;
                this.fire('load', this);
                this.fire('load:' + this.id, this);
            }
        });
    }

    // Get resource (auto-load if needed)
    get resource() {
        if (!this._loaded && this._preload) {
            this.load();
        }
        return this._resource;
    }

    // Reference counting
    reference() {
        this._refCount++;
    }

    unreference() {
        this._refCount = Math.max(0, this._refCount - 1);

        // Auto-unload if no references and unloadable
        if (this._refCount === 0 && this.unloadable) {
            this.unload();
        }
    }
}
```

---

## Resource Handlers

### Handler Base Class

**File:** `src/framework/handlers/handler.js`

```javascript
class ResourceHandler {
    constructor(registry, loader) {
        this.registry = registry;
        this.loader = loader;
    }

    // Load resource from URL
    load(url, callback, asset) {
        // To be implemented by subclass
    }

    // Open loaded data (parse/decode)
    open(asset, data) {
        // To be implemented by subclass
    }

    // Patch loaded asset data
    patch(asset, assetData) {
        // To be implemented by subclass
    }

    // Clean up resource
    destroy(resource) {
        if (resource && typeof resource.destroy === 'function') {
            resource.destroy();
        }
    }
}
```

### Texture Handler

**File:** `src/framework/handlers/texture.js`

```javascript
class TextureHandler extends ResourceHandler {
    constructor(registry, loader) {
        super(registry, loader);
        this.retry = false;
    }

    load(url, callback, asset) {
        const options = {
            cache: true,
            withCredentials: false,
            retry: this.retry
        };

        // Check for Basis compression
        if (asset.file && asset.file.filename?.endsWith('.ktx2')) {
            this._loadBasis(url, callback, asset);
            return;
        }

        // Load regular texture
        this.loader.loadImage(url, options, (err, img) => {
            if (err) {
                callback(err);
                return;
            }

            // Create texture
            const texture = new Texture(asset.registry._app.graphicsDevice, {
                name: asset.name,
                width: img.width,
                height: img.height,
                format: PIXELFORMAT_R8_G8_B8_A8,
                mipmaps: true,
                minFilter: FILTER_LINEAR_MIPMAP_LINEAR,
                magFilter: FILTER_LINEAR,
                addressU: ADDRESS_REPEAT,
                addressV: ADDRESS_REPEAT
            });

            texture.setName(asset.name);
            texture.setSource(img);

            callback(null, texture);
        });
    }

    _loadBasis(url, callback, asset) {
        // Load Basis compressed texture
        this.loader.loadArrayBuffer(url, (err, arrayBuffer) => {
            if (err) {
                callback(err);
                return;
            }

            // Transcode Basis to GPU format
            const basisTranscoded = transcodeBasis(arrayBuffer);

            const texture = new Texture(asset.registry._app.graphicsDevice, {
                name: asset.name,
                width: basisTranscoded.width,
                height: basisTranscoded.height,
                format: basisTranscoded.format,
                mipmaps: basisTranscoded.mipmaps,
                compression: true
            });

            texture.setSource(basisTranscoded.data);
            callback(null, texture);
        });
    }

    open(asset, data) {
        return data;  // Texture is already created in load
    }

    patch(asset, assetData) {
        const resource = asset.resource;
        if (!resource) return;

        // Apply asset data properties to texture
        if (assetData.mipmap !== undefined) {
            resource.mipmaps = assetData.mipmap;
        }
        if (assetData.minFilter !== undefined) {
            resource.minFilter = assetData.minFilter;
        }
        if (assetData.magFilter !== undefined) {
            resource.magFilter = assetData.magFilter;
        }
        if (assetData.addressU !== undefined) {
            resource.addressU = assetData.addressU;
        }
        if (assetData.addressV !== undefined) {
            resource.addressV = assetData.addressV;
        }

        resource.update();
    }
}
```

### Container Handler (glTF/GLB)

**File:** `src/framework/handlers/container.js`

```javascript
class ContainerHandler extends ResourceHandler {
    constructor(registry, loader) {
        super(registry, loader);
        this.parser = new GlbContainerParser(registry._app);
    }

    load(url, callback, asset) {
        // Load GLB file as array buffer
        this.loader.loadArrayBuffer(url, (err, arrayBuffer) => {
            if (err) {
                callback(err);
                return;
            }

            // Parse GLB
            this.parser.parse(arrayBuffer, asset, (err, result) => {
                if (err) {
                    callback(err);
                    return;
                }

                // Create container resource
                const container = new ContainerResource(result);
                callback(null, container);
            });
        });
    }

    open(asset, data) {
        return data;
    }
}

// GLB Container Parser
class GlbContainerParser {
    constructor(app) {
        this.app = app;
        this.assetRegistry = app.assets;
    }

    parse(arrayBuffer, asset, callback) {
        const result = {
            model: null,
            renders: [],
            materials: [],
            textures: [],
            animations: [],
            entities: []
        };

        // Parse GLB using glTF parser
        const parser = new GLTFParser(this.app.graphicsDevice);

        parser.parse(arrayBuffer, (err, gltfData) => {
            if (err) {
                callback(err);
                return;
            }

            // Create renders from meshes
            for (const mesh of gltfData.meshes) {
                const render = new Render(asset.registry._app.graphicsDevice);
                render.meshes = mesh.meshes;

                const renderAsset = new Asset(
                    mesh.name || 'render',
                    'render',
                    null,
                    { render: render }
                );
                renderAsset.loaded = true;
                renderAsset.resource = render;

                result.renders.push(renderAsset);
            }

            // Create materials
            for (const material of gltfData.materials) {
                const matAsset = new Asset(
                    material.name || 'material',
                    'material',
                    null,
                    { material: material }
                );
                matAsset.loaded = true;
                matAsset.resource = material;

                result.materials.push(matAsset);
            }

            // Create animations
            for (const anim of gltfData.animations) {
                const animAsset = new Asset(
                    anim.name || 'animation',
                    'animation',
                    null,
                    { animation: anim }
                );
                animAsset.loaded = true;
                animAsset.resource = anim;

                result.animations.push(animAsset);
            }

            // Create model entity hierarchy
            const rootEntity = parser.createEntityHierarchy(gltfData);
            result.entities.push(rootEntity);

            callback(null, result);
        });
    }
}

// Container Resource
class ContainerResource {
    constructor(data) {
        this.renders = data.renders;
        this.materials = data.materials;
        this.textures = data.textures;
        this.animations = data.animations;
        this.entities = data.entities;
    }

    // Instantiate as model entity
    instantiateModelEntity(options = {}) {
        const entity = this.entities[0]?.clone();

        if (options.type) {
            const modelComponent = entity?.model;
            if (modelComponent) {
                modelComponent.type = options.type;
            }
        }

        return entity;
    }

    // Instantiate as render entity
    instantiateRenderEntity(options = {}) {
        const entity = this.entities[0]?.clone();

        if (options.castShadows !== undefined) {
            entity?.findComponents('render').forEach(r => {
                r.castShadows = options.castShadows;
            });
        }

        return entity;
    }

    // Get material variants
    getMaterialVariants() {
        return this._materialVariants || [];
    }

    // Apply material variant
    applyMaterialVariant(entity, variantName) {
        // Find and apply material variant
        const variant = this._materialVariants?.find(v => v.name === variantName);
        if (variant) {
            entity.findComponents('render').forEach(render => {
                render.meshInstances.forEach(mi => {
                    const material = variant.materials.find(m => m.node === mi.node);
                    if (material) {
                        mi.material = material;
                    }
                });
            });
        }
    }
}
```

### Animation Handler

**File:** `src/framework/handlers/animation.js`

```javascript
class AnimationHandler extends ResourceHandler {
    load(url, callback, asset) {
        this.loader.loadJson(url, (err, data) => {
            if (err) {
                callback(err);
                return;
            }

            const animation = this._parseAnimation(data);
            callback(null, animation);
        });
    }

    _parseAnimation(data) {
        const tracks = [];

        for (const trackData of data.tracks) {
            const paths = trackData.paths || [trackData.path];
            const curve = new AnimCurve(
                trackData.type || 'linear',
                trackData.name,
                paths,
                new Float32Array(trackData.times),
                new Float32Array(trackData.values),
                trackData.tangents ? new Float32Array(trackData.tangents) : null
            );
            tracks.push(curve);
        }

        return new AnimTrack(data.name, data.duration, tracks);
    }
}
```

### Script Handler

**File:** `src/framework/handlers/script.js`

```javascript
class ScriptHandler extends ResourceHandler {
    load(url, callback, asset) {
        // Load script as text
        this.loader.loadText(url, (err, code) => {
            if (err) {
                callback(err);
                return;
            }

            // Execute script in global context
            try {
                // Scripts register themselves via pc.registerScript()
                const script = new Function('exports', 'pc', code);
                script({}, pc);

                callback(null, null);  // Scripts don't have resources
            } catch (e) {
                callback(e);
            }
        });
    }
}
```

---

## Resource Loader

### ResourceLoader

**File:** `src/framework/handlers/loader.js`

```javascript
class ResourceLoader {
    constructor(registry) {
        this.registry = registry;
        this.handlers = {};

        // Loading cache
        this._loading = {};

        // Load prefix for URLs
        this._prefix = '';

        // Retry configuration
        this.maxRetries = 3;
        this.retryDelay = 1000;
    }

    // Register handler for asset type
    registerHandler(type, handler) {
        this.handlers[type] = handler;
    }

    // Load asset
    load(asset, callback) {
        const handler = this.handlers[asset.type];
        if (!handler) {
            callback(new Error(`No handler for asset type: ${asset.type}`));
            return;
        }

        // Check if already loading
        const url = asset.file?.url;
        if (url && this._loading[url]) {
            this._loading[url].push(callback);
            return;
        }

        // Start loading
        if (url) {
            this._loading[url] = [callback];
        }

        const onLoad = (err, resource) => {
            if (url) {
                const callbacks = this._loading[url];
                delete this._loading[url];

                // Call all pending callbacks
                if (callbacks) {
                    for (const cb of callbacks) {
                        cb(err, resource);
                    }
                }
            } else {
                callback(err, resource);
            }
        };

        // Load using handler
        handler.load(this._prefix + url, onLoad, asset);
    }

    // Load image
    loadImage(url, options, callback) {
        const img = new Image();

        img.onload = () => callback(null, img);
        img.onerror = () => callback(new Error(`Failed to load image: ${url}`));

        img.crossOrigin = options.crossOrigin || 'anonymous';
        img.src = url;
    }

    // Load array buffer
    loadArrayBuffer(url, callback) {
        const xhr = new XMLHttpRequest();
        xhr.open('GET', url, true);
        xhr.responseType = 'arraybuffer';

        xhr.onload = () => {
            if (xhr.status === 200) {
                callback(null, xhr.response);
            } else {
                callback(new Error(`HTTP ${xhr.status}: ${url}`));
            }
        };

        xhr.onerror = () => callback(new Error(`Failed to load: ${url}`));
        xhr.send();
    }

    // Load text
    loadText(url, callback) {
        const xhr = new XMLHttpRequest();
        xhr.open('GET', url, true);

        xhr.onload = () => {
            if (xhr.status === 200) {
                callback(null, xhr.responseText);
            } else {
                callback(new Error(`HTTP ${xhr.status}: ${url}`));
            }
        };

        xhr.onerror = () => callback(new Error(`Failed to load: ${url}`));
        xhr.send();
    }

    // Load JSON
    loadJson(url, callback) {
        this.loadText(url, (err, text) => {
            if (err) {
                callback(err);
                return;
            }

            try {
                const json = JSON.parse(text);
                callback(null, json);
            } catch (e) {
                callback(e);
            }
        });
    }
}
```

---

## Asset Compression

### Basis Universal Texture Compression

**File:** `basis_universal/`

Basis Universal is a codec for GPU-compressed textures:

```javascript
// Basis transcoder setup
import { initBasis, transcode } from './basis-transcoder.js';

async function transcodeBasis(arrayBuffer) {
    // Initialize Basis transcoder (WASM)
    await initBasis();

    // Parse Basis file
    const basisFile = new BasisFile(arrayBuffer);

    // Get texture info
    const width = basisFile.getWidth();
    const height = basisFile.getHeight();
    const mipmaps = basisFile.getNumLevels(0);

    // Transcode to target format (BC7, ASTC, ETC2, etc.)
    const format = selectBestFormat();
    const transcoded = transcode(basisFile, format);

    return {
        width,
        height,
        mipmaps,
        format,
        data: transcoded
    };
}

function selectBestFormat() {
    // Select best format based on GPU capabilities
    const gl = glContext;

    if (gl.getExtension('WEBGL_compressed_texture_s3tc')) {
        return BASIS_FORMAT_BC7;  // Desktop
    }
    if (gl.getExtension('WEBGL_compressed_texture_etc')) {
        return BASIS_FORMAT_ETC2;  // Mobile
    }
    if (gl.getExtension('WEBGL_compressed_texture_astc')) {
        return BASIS_FORMAT_ASTC;  // Modern mobile
    }

    return BASIS_FORMAT_RGBA8;  // Fallback
}
```

### Draco Geometry Compression

**File:** `src/framework/parsers/gltf/draco-parser.js`

```javascript
class DracoParser {
    constructor() {
        this.decoder = null;
        this.initialized = false;
    }

    async init() {
        if (this.initialized) return;

        // Load Draco decoder WASM
        const draco = await import('draco3d');
        this.decoder = new draco.Decoder();
        this.initialized = true;
    }

    async decode(dracoBuffer) {
        await this.init();

        // Decode Draco compressed geometry
        const array = new Int8Array(dracoBuffer);
        const buffer = new draco.DecoderBuffer();
        buffer.Init(array.length);
        buffer.PushBackArray(array);

        // Get geometry type
        const geometryType = this.decoder.GetEncodedGeometryType(buffer);

        // Decode
        let decoded;
        if (geometryType === draco.TRIANGULAR_MESH) {
            decoded = new draco.Mesh();
            this.decoder.DecodeBufferToMesh(buffer, decoded);
        }

        // Extract attributes
        const positions = this._extractAttribute(decoded, draco.POSITION);
        const normals = this._extractAttribute(decoded, draco.NORMAL);
        const uvs = this._extractAttribute(decoded, draco.TEX_COORD);

        // Extract indices
        const indices = this._extractIndices(decoded);

        return {
            positions,
            normals,
            uvs,
            indices
        };
    }

    _extractAttribute(mesh, attributeType) {
        const attribute = this.decoder.GetAttributeByUniqueId(mesh, attributeType);
        const count = attribute.num_components() * mesh.num_points();
        const data = new Float32Array(count);

        this.decoder.GetAttributeFloatForAllPoints(mesh, attribute, data);

        return data;
    }
}
```

---

## Bundle System

### Bundle Support

```javascript
class Bundle {
    constructor(asset) {
        this.asset = asset;
        this.assets = [];  // Assets contained in this bundle
        this.loaded = false;
        this.loading = false;
    }

    // Load entire bundle
    load(callback) {
        if (this.loaded || this.loading) return;

        this.loading = true;
        let loaded = 0;
        const total = this.assets.length;

        for (const asset of this.assets) {
            asset.once('load', () => {
                loaded++;
                if (loaded === total) {
                    this.loaded = true;
                    this.loading = false;
                    callback(null, this);
                }
            });

            asset.load();
        }
    }
}

class BundleRegistry {
    constructor() {
        this._bundles = {};
    }

    add(bundle) {
        this._bundles[bundle.asset.id] = bundle;
    }

    getBundleForAsset(asset) {
        for (const bundle of Object.values(this._bundles)) {
            if (bundle.assets.includes(asset)) {
                return bundle;
            }
        }
        return null;
    }
}
```

---

## Asset References

### AssetReference

**File:** `src/framework/asset/asset-reference.js`

```javascript
class AssetReference {
    constructor(key, component, property, registry, onLoad, onChange, onRemove) {
        this._key = key;
        this._component = component;
        this._property = property;
        this._registry = registry;
        this._onLoad = onLoad;
        this._onChange = onChange;
        this._onRemove = onRemove;

        this._asset = null;
        this._id = null;
    }

    // Get/set asset ID
    get id() {
        return this._id;
    }

    set id(value) {
        if (this._id === value) return;

        // Unsubscribe from old asset
        if (this._asset) {
            this._asset.off('load', this._onAssetLoad, this);
            this._asset.off('change', this._onAssetChange, this);
            this._asset.off('remove', this._onAssetRemove, this);
        }

        this._id = value;

        // Get new asset
        this._asset = this._registry.get(value);

        if (this._asset) {
            // Subscribe to new asset
            this._asset.on('load', this._onAssetLoad, this);
            this._asset.on('change', this._onAssetChange, this);
            this._asset.on('remove', this._onAssetRemove, this);

            // Load if needed
            if (!this._asset.loaded) {
                this._asset.load();
            } else {
                this._onAssetLoad(this._asset);
            }
        } else if (this._id !== null) {
            // Wait for asset to be added
            this._registry.on('add:' + this._id, this._onAssetAdded, this);
        }
    }

    // Get resource
    get resource() {
        return this._asset?.resource;
    }

    _onAssetAdded(asset) {
        this._registry.off('add:' + this._id, this._onAssetAdded, this);
        this.id = this._id;
    }

    _onAssetLoad(asset) {
        this._component[this._property] = asset.resource;
        if (this._onLoad) this._onLoad(asset.resource);
    }

    _onAssetChange(asset, key, value) {
        if (this._onChange) this._onChange(asset, key, value);
    }

    _onAssetRemove(asset) {
        if (this._onRemove) this._onRemove(asset);
    }
}
```

---

## Loading Screen

```javascript
class LoadingScreen {
    constructor(app) {
        this.app = app;
        this.progress = 0;
        this.total = 0;
        this.loaded = 0;
    }

    show() {
        this.element = document.createElement('div');
        this.element.className = 'loading-screen';
        this.element.innerHTML = `
            <div class="loading-bar">
                <div class="loading-progress"></div>
            </div>
            <div class="loading-text">0%</div>
        `;
        document.body.appendChild(this.element);

        this.progressElement = this.element.querySelector('.loading-progress');
        this.textElement = this.element.querySelector('.loading-text');
    }

    hide() {
        if (this.element) {
            this.element.remove();
            this.element = null;
        }
    }

    setTotal(total) {
        this.total = total;
    }

    addLoaded() {
        this.loaded++;
        this.update();
    }

    update() {
        const percent = Math.round((this.loaded / this.total) * 100);
        this.progressElement.style.width = percent + '%';
        this.textElement.textContent = percent + '%';

        if (this.loaded >= this.total) {
            setTimeout(() => this.hide(), 500);
        }
    }
}

// Usage
const loadingScreen = new LoadingScreen(app);
loadingScreen.show();

// Load all assets
const assets = app.assets.filter(a => a.preload);
loadingScreen.setTotal(assets.length);

assets.forEach(asset => {
    asset.once('load', () => loadingScreen.addLoaded());
    asset.load();
});
```

---

## Summary

The PlayCanvas asset pipeline provides:

1. **AssetRegistry**: Central asset management with lookup by ID, name, URL, and tags
2. **ResourceHandlers**: Type-specific loaders for textures, models, animations, etc.
3. **Asynchronous Loading**: Promise and callback-based loading APIs
4. **Compression**: Basis Universal textures and Draco geometry compression
5. **Reference Counting**: Automatic asset lifecycle management
6. **Bundles**: Group assets for efficient loading
7. **Localization**: Built-in support for localized assets
