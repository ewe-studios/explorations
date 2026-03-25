# PlayCanvas Editor Architecture Deep Dive

## Overview

The PlayCanvas Editor is a web-based integrated development environment (IDE) for creating and editing PlayCanvas projects. It provides real-time collaboration, visual editing tools, and seamless integration with the cloud backend.

---

## Architecture Diagram

```
┌─────────────────────────────────────────────────────────────────────────┐
│                         PlayCanvas Editor                                │
├─────────────────────────────────────────────────────────────────────────┤
│                            UI Layer                                      │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐   │
│  │   Hierarchy │  │ Inspector   │  │   Assets    │  │    Scene    │   │
│  │    Panel    │  │    Panel    │  │    Panel    │  │   Viewport  │   │
│  └─────────────┘  └─────────────┘  └─────────────┘  └─────────────┘   │
├─────────────────────────────────────────────────────────────────────────┤
│                         PCUI Component Library                           │
│  ┌─────────────────────────────────────────────────────────────────┐   │
│  │  Panels, Controls, Graphs, Selectors, Pickers, Dialogs, etc.   │   │
│  └─────────────────────────────────────────────────────────────────┘   │
├─────────────────────────────────────────────────────────────────────────┤
│                        Editor Core                                       │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐   │
│  │   Entities  │  │   Assets    │  │   History   │  │   Realtime  │   │
│  │  Manager    │  │   Manager   │  │   (Undo)    │  │    Sync     │   │
│  └─────────────┘  └─────────────┘  └─────────────┘  └─────────────┘   │
├─────────────────────────────────────────────────────────────────────────┤
│                      Engine Integration                                  │
│  ┌─────────────────────────────────────────────────────────────────┐   │
│  │              PlayCanvas Engine (Modified for Editor)            │   │
│  │  - Scene rendering                                              │   │
│  │  - Entity/component system                                      │   │
│  │  - Asset loading                                                │   │
│  └─────────────────────────────────────────────────────────────────┘   │
├─────────────────────────────────────────────────────────────────────────┤
│                       Backend API                                        │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐   │
│  │   Project   │  │    Asset    │  │   Scene     │  │   Realtime  │   │
│  │     API     │  │     API     │  │    API      │  │   Service   │   │
│  └─────────────┘  └─────────────┘  └─────────────┘  └─────────────┘   │
└─────────────────────────────────────────────────────────────────────────┘
```

---

## Editor Structure

### Main Editor Entry Point

**File:** `src/editor/index.ts`

```typescript
// Editor entry point
import { Application } from 'playcanvas';
import { Scene } from './editor/scene';
import { Entities } from './editor/entities';
import { Assets } from './editor/assets';
import { History } from './editor/history';
import { Realtime } from './editor/realtime';
import { Inspector } from './editor/inspector';
import { Hierarchy } from './editor/hierarchy';
import { AssetsPanel } from './editor/assets';
import { Viewport } from './editor/viewport';

interface EditorOptions {
    canvas: HTMLCanvasElement;
    projectId: string;
    userId: string;
    authToken: string;
}

class Editor {
    app: Application;
    scene: Scene;
    entities: Entities;
    assets: Assets;
    history: History;
    realtime: Realtime;

    panels: {
        hierarchy: Hierarchy;
        inspector: Inspector;
        assets: AssetsPanel;
        viewport: Viewport;
    };

    constructor(options: EditorOptions) {
        // Initialize engine
        this.app = new Application(options.canvas);

        // Initialize editor systems
        this.scene = new Scene(this);
        this.entities = new Entities(this);
        this.assets = new Assets(this);
        this.history = new History(this);
        this.realtime = new Realtime(this);

        // Initialize panels
        this.panels = {
            hierarchy: new Hierarchy(this),
            inspector: new Inspector(this),
            assets: new AssetsPanel(this),
            viewport: new Viewport(this)
        };
    }

    async load(projectId: string) {
        // Load project data
        await this.assets.loadProject(projectId);

        // Load scene
        await this.scene.load();

        // Start realtime sync
        this.realtime.connect();

        // Start editor loop
        this.start();
    }

    start() {
        this.app.start();
        this.editorUpdate();
    }

    editorUpdate = () => {
        // Update editor systems
        this.entities.update();
        this.realtime.update();

        // Update panels
        this.panels.hierarchy.update();
        this.panels.inspector.update();

        requestAnimationFrame(this.editorUpdate);
    };
}
```

---

## Entity Management

### Entities Manager

```typescript
// src/editor/entities/index.ts

import { Entity, Vec3, Quat } from 'playcanvas';

interface EntityData {
    id: string;
    name: string;
    parentId: string | null;
    position: [number, number, number];
    rotation: [number, number, number, number];
    scale: [number, number, number];
    components: ComponentData[];
}

interface ComponentData {
    type: string;
    enabled: boolean;
    data: Record<string, any>;
}

class Entities {
    editor: Editor;

    // Entity map
    private entities: Map<string, Entity>;
    private entityData: Map<string, EntityData>;

    // Selection
    private selected: Set<string>;

    constructor(editor: Editor) {
        this.editor = editor;
        this.entities = new Map();
        this.entityData = new Map();
        this.selected = new Set();
    }

    // Create new entity
    create(name: string, parent?: Entity): Entity {
        const entity = new Entity(name);

        // Add default components
        entity.addComponent('render');
        entity.addComponent('script');

        // Add to hierarchy
        if (parent) {
            parent.addChild(entity);
        } else {
            this.editor.scene.root.addChild(entity);
        }

        // Track entity
        this.entities.set(entity.getGuid(), entity);

        // Record creation for history
        this.editor.history.record({
            type: 'entity:create',
            entityId: entity.getGuid(),
            data: this.serialize(entity)
        });

        // Notify realtime
        this.editor.realtime.broadcast('entity:create', {
            entityId: entity.getGuid(),
            data: this.serialize(entity)
        });

        return entity;
    }

    // Delete entity
    delete(entity: Entity) {
        const id = entity.getGuid();

        // Record for history
        this.editor.history.record({
            type: 'entity:delete',
            entityId: id,
            data: this.serialize(entity)
        });

        // Remove from tracking
        this.entities.delete(id);
        this.selected.delete(id);

        // Remove from parent
        entity.destroy();

        // Notify realtime
        this.editor.realtime.broadcast('entity:delete', { entityId: id });
    }

    // Set entity position
    setPosition(entity: Entity, position: Vec3) {
        const oldPosition = entity.getPosition().clone();

        entity.setPosition(position);

        this.editor.history.record({
            type: 'entity:set',
            entityId: entity.getGuid(),
            property: 'position',
            oldValue: oldPosition,
            newValue: position
        });

        this.editor.realtime.broadcast('entity:set', {
            entityId: entity.getGuid(),
            property: 'position',
            value: position
        });
    }

    // Serialize entity to JSON
    serialize(entity: Entity): EntityData {
        return {
            id: entity.getGuid(),
            name: entity.name,
            parentId: entity.parent?.getGuid() ?? null,
            position: entity.localPosition.toArray(),
            rotation: entity.localRotation.toArray(),
            scale: entity.localScale.toArray(),
            components: this.serializeComponents(entity)
        };
    }

    serializeComponents(entity: Entity): ComponentData[] {
        const components: ComponentData[] = [];

        for (const type of Object.keys(entity.c)) {
            const component = entity[type];
            components.push({
                type,
                enabled: component.enabled,
                data: { ...component.data }
            });
        }

        return components;
    }

    // Deserialize and create entity
    deserialize(data: EntityData, parent?: Entity): Entity {
        const entity = new Entity(data.name);

        entity.setPosition(data.position[0], data.position[1], data.position[2]);
        entity.setRotation(data.rotation[0], data.rotation[1], data.rotation[2], data.rotation[3]);
        entity.setScale(data.scale[0], data.scale[1], data.scale[2]);

        // Add components
        for (const component of data.components) {
            if (entity[component.type]) {
                Object.assign(entity[component.type].data, component.data);
            }
        }

        if (parent) {
            parent.addChild(entity);
        }

        this.entities.set(data.id, entity);

        return entity;
    }

    // Select entity
    select(entity: Entity) {
        this.selected.clear();
        this.selected.add(entity.getGuid());
        this.editor.emit('selection:changed', [entity]);
    }

    // Get selected entities
    getSelection(): Entity[] {
        return Array.from(this.selected).map(id => this.entities.get(id));
    }
}
```

---

## History System (Undo/Redo)

### History Manager

```typescript
// src/editor/history/index.ts

interface HistoryAction {
    type: string;
    entityId?: string;
    property?: string;
    oldValue?: any;
    newValue?: any;
    data?: any;
}

class History {
    editor: Editor;

    private undoStack: HistoryAction[];
    private redoStack: HistoryAction[];

    private maxHistorySize: number = 100;
    private batchMode: boolean = false;
    private batchActions: HistoryAction[] = [];

    constructor(editor: Editor) {
        this.editor = editor;
        this.undoStack = [];
        this.redoStack = [];
    }

    // Record an action
    record(action: HistoryAction) {
        if (this.batchMode) {
            this.batchActions.push(action);
            return;
        }

        this.undoStack.push(action);

        // Limit history size
        if (this.undoStack.length > this.maxHistorySize) {
            this.undoStack.shift();
        }

        // Clear redo stack on new action
        this.redoStack = [];

        this.editor.emit('history:changed', {
            canUndo: this.canUndo(),
            canRedo: this.canRedo()
        });
    }

    // Begin batch recording
    beginBatch() {
        this.batchMode = true;
        this.batchActions = [];
    }

    // End batch recording
    endBatch() {
        this.batchMode = false;

        if (this.batchActions.length > 0) {
            this.undoStack.push({
                type: 'batch',
                actions: this.batchActions
            });
        }

        this.redoStack = [];
    }

    // Undo last action
    undo() {
        if (!this.canUndo()) return;

        const action = this.undoStack.pop();
        this.executeUndo(action);
        this.redoStack.push(action);

        this.editor.emit('history:changed', {
            canUndo: this.canUndo(),
            canRedo: this.canRedo()
        });
    }

    // Redo last undone action
    redo() {
        if (!this.canRedo()) return;

        const action = this.redoStack.pop();
        this.executeRedo(action);
        this.undoStack.push(action);

        this.editor.emit('history:changed', {
            canUndo: this.canUndo(),
            canRedo: this.canRedo()
        });
    }

    private executeUndo(action: HistoryAction) {
        const entity = this.editor.entities.getEntity(action.entityId);

        switch (action.type) {
            case 'entity:create':
                this.editor.entities.delete(entity);
                break;

            case 'entity:delete':
                this.editor.entities.deserialize(action.data);
                break;

            case 'entity:set':
                if (entity && action.property) {
                    entity[action.property] = action.oldValue;
                }
                break;

            case 'batch':
                // Undo batch actions in reverse
                for (let i = action.actions.length - 1; i >= 0; i--) {
                    this.executeUndo(action.actions[i]);
                }
                break;
        }
    }

    private executeRedo(action: HistoryAction) {
        const entity = this.editor.entities.getEntity(action.entityId);

        switch (action.type) {
            case 'entity:create':
                this.editor.entities.deserialize(action.data);
                break;

            case 'entity:delete':
                this.editor.entities.delete(entity);
                break;

            case 'entity:set':
                if (entity && action.property) {
                    entity[action.property] = action.newValue;
                }
                break;

            case 'batch':
                for (const subAction of action.actions) {
                    this.executeRedo(subAction);
                }
                break;
        }
    }

    canUndo(): boolean {
        return this.undoStack.length > 0;
    }

    canRedo(): boolean {
        return this.redoStack.length > 0;
    }

    clear() {
        this.undoStack = [];
        this.redoStack = [];
    }
}
```

---

## Realtime Collaboration

### Realtime Sync

```typescript
// src/editor/realtime/index.ts

import { WebSocket } from './relay/websocket';

interface Operation {
    type: string;
    userId: string;
    timestamp: number;
    data: any;
}

class Realtime {
    editor: Editor;

    private ws: WebSocket;
    private connected: boolean = false;
    private userId: string;

    // Local operation tracking
    private pendingOperations: Operation[] = [];
    private lastSyncTime: number = 0;

    // Other users
    private users: Map<string, UserInfo> = new Map();

    constructor(editor: Editor) {
        this.editor = editor;
        this.userId = editor.options.userId;
    }

    connect() {
        this.ws = new WebSocket(this.editor.options.relayUrl);

        this.ws.on('open', () => {
            this.connected = true;
            this.editor.emit('realtime:connected');
        });

        this.ws.on('message', (data) => {
            this.handleMessage(data);
        });

        this.ws.on('close', () => {
            this.connected = false;
            this.editor.emit('realtime:disconnected');
        });
    }

    broadcast(type: string, data: any) {
        if (!this.connected) return;

        const operation: Operation = {
            type,
            userId: this.userId,
            timestamp: Date.now(),
            data
        };

        this.ws.send(JSON.stringify(operation));

        // Apply locally
        this.applyOperation(operation);
    }

    private handleMessage(data: string) {
        const operation: Operation = JSON.parse(data);

        // Ignore own operations
        if (operation.userId === this.userId) return;

        // Apply remote operation
        this.applyOperation(operation);
    }

    private applyOperation(operation: Operation) {
        switch (operation.type) {
            case 'entity:create':
                this.handleEntityCreate(operation);
                break;

            case 'entity:delete':
                this.handleEntityDelete(operation);
                break;

            case 'entity:set':
                this.handleEntitySet(operation);
                break;

            case 'user:joined':
                this.handleUserJoined(operation);
                break;

            case 'user:left':
                this.handleUserLeft(operation);
                break;
        }
    }

    private handleEntityCreate(operation: Operation) {
        const { entityId, data } = operation.data;

        // Check if entity already exists (we created it)
        if (this.editor.entities.getEntity(entityId)) return;

        // Create entity from data
        this.editor.entities.deserialize(data);
    }

    private handleEntityDelete(operation: Operation) {
        const { entityId } = operation.data;
        const entity = this.editor.entities.getEntity(entityId);

        if (entity) {
            entity.destroy();
        }
    }

    private handleEntitySet(operation: Operation) {
        const { entityId, property, value } = operation.data;
        const entity = this.editor.entities.getEntity(entityId);

        if (entity && entity[property] !== undefined) {
            entity[property] = value;
        }
    }

    private handleUserJoined(operation: Operation) {
        const { user } = operation.data;
        this.users.set(user.id, user);
        this.editor.emit('user:joined', user);
    }

    private handleUserLeft(operation: Operation) {
        const { userId } = operation.data;
        this.users.delete(userId);
        this.editor.emit('user:left', userId);
    }

    update() {
        // Sync pending operations
        if (!this.connected) return;

        const now = Date.now();
        if (now - this.lastSyncTime > 100) {
            this.lastSyncTime = now;

            // Send heartbeat
            this.ws.send(JSON.stringify({
                type: 'heartbeat',
                userId: this.userId,
                timestamp: now
            }));
        }
    }

    // Get current users
    getUsers(): UserInfo[] {
        return Array.from(this.users.values());
    }
}
```

---

## Asset Management

### Asset Panel

```typescript
// src/editor/assets/index.ts

import { Asset } from 'playcanvas';

class AssetsPanel {
    editor: Editor;

    private assets: Map<string, Asset>;
    private folders: Map<string, Folder>;

    private selected: Set<string>;
    private viewMode: 'grid' | 'list' = 'grid';

    constructor(editor: Editor) {
        this.editor = editor;
        this.assets = new Map();
        this.folders = new Map();
        this.selected = new Set();
    }

    async loadProject(projectId: string) {
        // Load asset hierarchy
        const response = await fetch(`/api/project/${projectId}/assets`);
        const data = await response.json();

        this.buildAssetTree(data.assets);
    }

    private buildAssetTree(assets: any[]) {
        for (const assetData of assets) {
            const asset = new Asset(
                assetData.name,
                assetData.type,
                assetData.file
            );

            asset.id = assetData.id;

            if (assetData.parent) {
                this.folders.get(assetData.parent)?.addAsset(asset);
            }

            this.assets.set(asset.id, asset);
        }
    }

    // Upload new asset
    async upload(file: File, folderId?: string): Promise<Asset> {
        const formData = new FormData();
        formData.append('file', file);
        if (folderId) {
            formData.append('folder', folderId);
        }

        const response = await fetch('/api/assets/upload', {
            method: 'POST',
            body: formData
        });

        const data = await response.json();
        const asset = new Asset(data.name, data.type, data.file);
        asset.id = data.id;

        this.assets.set(asset.id, asset);
        this.editor.emit('asset:created', asset);

        return asset;
    }

    // Delete asset
    delete(asset: Asset) {
        fetch(`/api/assets/${asset.id}`, {
            method: 'DELETE'
        });

        this.assets.delete(asset.id);
        this.editor.emit('asset:deleted', asset);
    }

    // Select asset
    select(asset: Asset) {
        this.selected.clear();
        this.selected.add(asset.id);
        this.editor.emit('asset:selected', asset);
    }

    // Create folder
    async createFolder(name: string, parentFolderId?: string): Promise<Folder> {
        const response = await fetch('/api/assets/folder', {
            method: 'POST',
            headers: { 'Content-Type': 'application/json' },
            body: JSON.stringify({ name, parent: parentFolderId })
        });

        const data = await response.json();
        const folder = new Folder(data.id, data.name);

        this.folders.set(folder.id, folder);

        return folder;
    }
}

class Folder {
    id: string;
    name: string;
    parentId: string | null;
    children: Folder[] = [];
    assets: Asset[] = [];

    constructor(id: string, name: string) {
        this.id = id;
        this.name = name;
    }

    addAsset(asset: Asset) {
        this.assets.push(asset);
    }

    addFolder(folder: Folder) {
        folder.parentId = this.id;
        this.children.push(folder);
    }
}
```

---

## Inspector Panel

### Inspector

```typescript
// src/editor/inspector/index.ts

import { Entity, Component } from 'playcanvas';
import { Panel, Property, VectorInput, ColorInput } from 'pcui';

class Inspector {
    editor: Editor;

    private panel: Panel;
    private properties: Map<string, Property>;

    private inspected: any[] = [];

    constructor(editor: Editor) {
        this.editor = editor;
        this.properties = new Map();

        // Create inspector panel UI
        this.panel = new Panel({
            headerText: 'Inspector',
            resizable: 'right'
        });
    }

    // Set objects to inspect
    inspect(objects: any[]) {
        this.inspected = objects;

        // Clear existing properties
        this.clear();

        if (objects.length === 0) return;

        // Build property list
        if (objects[0] instanceof Entity) {
            this.buildEntityInspector(objects[0]);
        } else if (objects[0] instanceof Asset) {
            this.buildAssetInspector(objects[0]);
        }

        this.panel.dom.appendChild(this.propertiesContainer);
    }

    private buildEntityInspector(entity: Entity) {
        // Transform properties
        this.addSection('Transform');
        this.addVector3Property('Position', entity, 'position');
        this.addVector3Property('Rotation', entity, 'rotation');
        this.addVector3Property('Scale', entity, 'scale');

        // Component properties
        for (const type of Object.keys(entity.c)) {
            this.addSection(type);
            this.buildComponentProperties(type, entity[type]);
        }
    }

    private buildComponentProperties(type: string, component: Component) {
        const schema = component.system.schema;

        for (const descriptor of schema) {
            const name = typeof descriptor === 'object' ? descriptor.name : descriptor;
            const propType = typeof descriptor === 'object' ? descriptor.type : 'any';

            switch (propType) {
                case 'vec3':
                    this.addVector3Property(name, component, name);
                    break;
                case 'vec4':
                    this.addVector4Property(name, component, name);
                    break;
                case 'rgb':
                case 'rgba':
                    this.addColorProperty(name, component, name);
                    break;
                case 'boolean':
                    this.addBooleanProperty(name, component, name);
                    break;
                case 'number':
                    this.addNumberProperty(name, component, name);
                    break;
                case 'string':
                    this.addStringProperty(name, component, name);
                    break;
                case 'asset':
                    this.addAssetProperty(name, component, name);
                    break;
                case 'entity':
                    this.addEntityProperty(name, component, name);
                    break;
            }
        }
    }

    private addVector3Property(name: string, object: any, key: string) {
        const input = new VectorInput({
            dimensions: 3,
            value: [object[key].x, object[key].y, object[key].z],
            precision: 4
        });

        input.on('change', (value: number[]) => {
            object[key].set(value[0], value[1], value[2]);
        });

        this.addProperty(name, input);
    }

    private addColorProperty(name: string, object: any, key: string) {
        const input = new ColorInput({
            value: [object[key].r, object[key].g, object[key].b]
        });

        input.on('change', (value: number[]) => {
            object[key].set(value[0], value[1], value[2]);
        });

        this.addProperty(name, input);
    }

    private addBooleanProperty(name: string, object: any, key: string) {
        const input = new BooleanInput({
            value: object[key]
        });

        input.on('change', (value: boolean) => {
            object[key] = value;
        });

        this.addProperty(name, input);
    }

    private addNumberProperty(name: string, object: any, key: string) {
        const input = new NumberInput({
            value: object[key],
            precision: 4
        });

        input.on('change', (value: number) => {
            object[key] = value;
        });

        this.addProperty(name, input);
    }

    private addProperty(name: string, input: any) {
        const property = new Property({
            key: name,
            value: input
        });

        this.properties.set(name, property);
        this.propertiesContainer.appendChild(property.dom);
    }

    private clear() {
        this.properties.clear();
        this.propertiesContainer.innerHTML = '';
    }

    update() {
        // Update property values if inspected objects changed
        // (handled by input change events)
    }
}
```

---

## Scene Viewport

### Viewport

```typescript
// src/editor/viewport/index.ts

import { Camera, Entity, Vec3, Mouse, Keyboard } from 'playcanvas';
import { Gizmo } from 'playcanvas/gizmo';

class Viewport {
    editor: Editor;

    private camera: Entity;
    private scene: Scene;

    // Viewport state
    private viewMode: 'perspective' | 'orthographic' = 'perspective';
    private gridEnabled: boolean = true;
    private gizmoEnabled: boolean = true;

    // Gizmos
    private translateGizmo: Gizmo;
    private rotateGizmo: Gizmo;
    private scaleGizmo: Gizmo;

    // Current tool
    private currentTool: 'select' | 'translate' | 'rotate' | 'scale' = 'select';

    constructor(editor: Editor) {
        this.editor = editor;

        // Create camera entity
        this.camera = new Entity('EditorCamera');
        this.camera.addComponent('camera', {
            clearColor: new Color(0.3, 0.3, 0.3, 1)
        });
        this.camera.setPosition(5, 5, 5);
        this.camera.lookAt(0, 0, 0);

        // Create gizmos
        this.createGizmos();

        // Setup input
        this.setupInput();
    }

    private createGizmos() {
        // Translate gizmo
        this.translateGizmo = new TranslateGizmo(this.editor.app, this.camera.camera);
        this.translateGizmo.on('transform:start', this.onTransformStart, this);
        this.translateGizmo.on('transform:move', this.onTransformMove, this);
        this.translateGizmo.on('transform:end', this.onTransformEnd, this);

        // Rotate gizmo
        this.rotateGizmo = new RotateGizmo(this.editor.app, this.camera.camera);
        this.rotateGizmo.on('transform:start', this.onTransformStart, this);
        this.rotateGizmo.on('transform:move', this.onTransformMove, this);
        this.rotateGizmo.on('transform:end', this.onTransformEnd, this);

        // Scale gizmo
        this.scaleGizmo = new ScaleGizmo(this.editor.app, this.camera.camera);
        this.scaleGizmo.on('transform:start', this.onTransformStart, this);
        this.scaleGizmo.on('transform:move', this.onTransformMove, this);
        this.scaleGizmo.on('transform:end', this.onTransformEnd, this);
    }

    private setupInput() {
        const mouse = this.editor.app.mouse;
        const keyboard = this.editor.app.keyboard;

        // Orbit camera with right mouse
        mouse.on('mousedown', (event) => {
            if (event.button === MOUSEBUTTON_RIGHT) {
                this.startOrbit(event.x, event.y);
            }
        });

        mouse.on('mousemove', (event) => {
            if (this.isOrbiting) {
                this.orbit(event.x, event.y);
            }
        });

        mouse.on('mouseup', () => {
            this.isOrbiting = false;
        });

        // Zoom with scroll
        mouse.on('wheel', (delta) => {
            this.zoom(delta);
        });

        // Keyboard shortcuts
        keyboard.on('keydown', (event) => {
            switch (event.key) {
                case 'w':
                    this.setTool('translate');
                    break;
                case 'e':
                    this.setTool('rotate');
                    break;
                case 'r':
                    this.setTool('scale');
                    break;
                case 'q':
                    this.setTool('select');
                    break;
                case 'f':
                    this.focusOnSelection();
                    break;
            }
        });
    }

    private setTool(tool: string) {
        this.currentTool = tool;

        // Hide all gizmos
        this.translateGizmo.visible = false;
        this.rotateGizmo.visible = false;
        this.scaleGizmo.visible = false;

        // Show selected gizmo
        const selection = this.editor.entities.getSelection();
        if (selection.length > 0 && tool !== 'select') {
            const gizmo = this.getGizmo(tool);
            gizmo.node = selection[0];
            gizmo.visible = true;
        }
    }

    private onTransformStart() {
        this.editor.history.beginBatch();
    }

    private onTransformMove(node: Entity) {
        // Live update during transform
    }

    private onTransformEnd() {
        this.editor.history.endBatch();
    }

    // Camera controls
    private isOrbiting: boolean = false;
    private orbitYaw: number = 0;
    private orbitPitch: number = 0;
    private orbitDistance: number = 10;
    private orbitTarget: Vec3 = new Vec3(0, 0, 0);

    private startOrbit(x: number, y: number) {
        this.isOrbiting = true;
        this.lastMouseX = x;
        this.lastMouseY = y;
    }

    private orbit(x: number, y: number) {
        const dx = x - this.lastMouseX;
        const dy = y - this.lastMouseY;

        this.orbitYaw -= dx * 0.5;
        this.orbitPitch -= dy * 0.5;
        this.orbitPitch = Math.max(-89, Math.min(89, this.orbitPitch));

        this.updateCameraPosition();

        this.lastMouseX = x;
        this.lastMouseY = y;
    }

    private zoom(delta: number) {
        this.orbitDistance *= 1 + delta * 0.01;
        this.updateCameraPosition();
    }

    private updateCameraPosition() {
        const x = this.orbitTarget.x + this.orbitDistance *
            Math.cos(this.orbitYaw * DEG_TO_RAD) *
            Math.cos(this.orbitPitch * DEG_TO_RAD);
        const y = this.orbitTarget.y + this.orbitDistance *
            Math.sin(this.orbitPitch * DEG_TO_RAD);
        const z = this.orbitTarget.z + this.orbitDistance *
            Math.sin(this.orbitYaw * DEG_TO_RAD) *
            Math.cos(this.orbitPitch * DEG_TO_RAD);

        this.camera.setPosition(x, y, z);
        this.camera.lookAt(this.orbitTarget);
    }

    private focusOnSelection() {
        const selection = this.editor.entities.getSelection();
        if (selection.length > 0) {
            const position = selection[0].getPosition();
            this.orbitTarget.copy(position);
            this.updateCameraPosition();
        }
    }

    private getGizmo(tool: string): Gizmo {
        switch (tool) {
            case 'translate': return this.translateGizmo;
            case 'rotate': return this.rotateGizmo;
            case 'scale': return this.scaleGizmo;
        }
    }
}
```

---

## PCUI Component Library

### PCUI Overview

The PlayCanvas Editor uses PCUI, a custom UI component library built on vanilla JavaScript:

```typescript
// PCUI component example
import { Panel, Label, Button, VectorInput } from 'pcui';

// Create panel
const panel = new Panel({
    headerText: 'My Panel',
    id: 'my-panel',
    resizable: 'bottom',
    collapsible: true,
    collapsed: false
});

// Add content
const label = new Label({
    text: 'Hello World',
    class: 'my-label'
});

const button = new Button({
    text: 'Click Me',
    class: 'my-button'
});

button.on('click', () => {
    console.log('Button clicked!');
});

panel.dom.appendChild(label.dom);
panel.dom.appendChild(button.dom);
document.body.appendChild(panel.dom);
```

---

## MCP Server Integration

### Editor MCP Server

**File:** `editor-mcp-server/src/index.ts`

The editor includes an MCP (Model Context Protocol) server for AI assistant integration:

```typescript
import { Server } from '@modelcontextprotocol/sdk/server/index.js';
import { StdioServerTransport } from '@modelcontextprotocol/sdk/server/stdio.js';

class EditorMCPServer {
    private server: Server;
    private editor: Editor;

    constructor(editor: Editor) {
        this.editor = editor;

        this.server = new Server(
            { name: 'playcanvas-editor', version: '1.0.0' },
            { capabilities: { resources: {}, tools: {} } }
        );

        this.setupTools();
        this.setupResources();

        const transport = new StdioServerTransport();
        this.server.connect(transport);
    }

    private setupTools() {
        // Create entity tool
        this.server.setRequestHandler(CallToolRequestSchema, async (request) => {
            switch (request.params.name) {
                case 'create_entity':
                    return this.createEntity(request.params.arguments);
                case 'delete_entity':
                    return this.deleteEntity(request.params.arguments);
                case 'set_component':
                    return this.setComponent(request.params.arguments);
                case 'get_scene_info':
                    return this.getSceneInfo();
            }
        });
    }

    private async createEntity(args: any) {
        const entity = this.editor.entities.create(args.name, args.parent);

        if (args.components) {
            for (const component of args.components) {
                Object.assign(entity[component.type].data, component.data);
            }
        }

        return {
            content: [{
                type: 'text',
                text: `Created entity "${args.name}" with ID ${entity.getGuid()}`
            }]
        };
    }
}
```

---

## Summary

The PlayCanvas Editor provides:

1. **Visual Editing**: Hierarchical entity editing with gizmos and inspector
2. **Real-time Collaboration**: Multiple users can edit simultaneously via WebSocket sync
3. **Undo/Redo**: Full history system with batch operations
4. **Asset Management**: Visual asset browser with folders and upload
5. **Component Editing**: Type-aware property inspectors for all components
6. **Extensibility**: MCP server for AI/automation integration
7. **PCUI**: Custom UI component library for editor panels
