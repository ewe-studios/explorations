# PlayCanvas ECS Architecture Deep Dive

## Overview

PlayCanvas uses a variant of the Entity-Component-System (ECS) pattern. While not a pure ECS implementation, it provides a flexible component-based architecture that allows for clean separation of data and behavior.

---

## Architecture Diagram

```
┌─────────────────────────────────────────────────────────────────┐
│                        Application                               │
│                         (AppBase)                               │
├─────────────────────────────────────────────────────────────────┤
│  ┌─────────────────────────────────────────────────────────┐   │
│  │                    Entity                                │   │
│  │  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐  │   │
│  │  │  GraphNode   │  │  Component 1 │  │  Component 2 │  │   │
│  │  │  (Transform) │  │   (Model)    │  │  (Script)    │  │   │
│  │  └──────────────┘  └──────────────┘  └──────────────┘  │   │
│  └─────────────────────────────────────────────────────────┘   │
│                              │                                   │
│              ┌───────────────┼───────────────┐                  │
│              ▼               ▼               ▼                  │
│  ┌──────────────────┬──────────────────┬──────────────────┐    │
│  │ ComponentSystem  │ ComponentSystem  │ ComponentSystem  │    │
│  │     (Model)      │   (Render)       │    (Script)      │    │
│  └──────────────────┴──────────────────┴──────────────────┘    │
│                              │                                   │
│              ┌───────────────┴───────────────┐                  │
│              ▼                               ▼                  │
│  ┌──────────────────────┐      ┌──────────────────────┐        │
│  │   Scene/Renderer     │      │   Platform Layer     │        │
│  │   (Draw Calls)       │      │   (WebGL/WebGPU)     │        │
│  └──────────────────────┘      └──────────────────────┘        │
└─────────────────────────────────────────────────────────────────┘
```

---

## Core Classes

### 1. GraphNode - The Transform Hierarchy

**File:** `src/scene/graph-node.js`

Every entity in PlayCanvas inherits from `GraphNode`, which provides:

- Hierarchical transform (position, rotation, scale)
- Parent-child relationships
- Local and world space transformations
- Tag system for grouping

```javascript
class GraphNode extends EventHandler {
    name;                          // Node name
    tags = new Tags(this);         // Tag interface

    // Local transform (relative to parent)
    localPosition = new Vec3();
    localRotation = new Quat();
    localScale = new Vec3(1, 1, 1);
    localEulerAngles = new Vec3();

    // World transform (computed automatically)
    position = new Vec3();
    rotation = new Quat();
    eulerAngles = new Vec3();

    localTransform = new Mat4();   // Local transformation matrix

    // Children and parent
    _children = [];
    _parent = null;
}
```

**Key Features:**

- **Automatic Sync**: The engine automatically synchronizes the hierarchy each frame
- **Dirty Flag System**: Transforms are only recalculated when changed
- **Freeze Support**: Nodes can be frozen to skip hierarchy sync (optimization)

### 2. Entity - The Component Container

**File:** `src/framework/entity.js`

`Entity` extends `GraphNode` and adds component support:

```javascript
class Entity extends GraphNode {
    // Component properties (auto-populated)
    anim;              // AnimComponent
    camera;            // CameraComponent
    collision;         // CollisionComponent
    light;             // LightComponent
    model;             // ModelComponent
    render;            // RenderComponent
    rigidbody;         // RigidBodyComponent
    script;            // ScriptComponent
    // ... and more

    // Component collection
    c = {};

    // Add a component
    addComponent(type, options) {
        return this.app.systems[type].addComponent(this, options);
    }

    // Remove a component
    removeComponent(type) {
        this.app.systems[type].removeComponent(this);
    }

    // Find components in children
    findComponent(type) {
        // ... recursive search
    }
}
```

### 3. Component - The Data Container

**File:** `src/framework/components/component.js`

Components hold data and delegate behavior to their systems:

```javascript
class Component extends EventHandler {
    static order = 0;     // Enable order
    system;               // Reference to owning system
    entity;               // Reference to owning entity

    constructor(system, entity) {
        super();
        this.system = system;
        this.entity = entity;
        // Auto-build property accessors from schema
        if (this.system.schema && !this._accessorsBuilt) {
            this.buildAccessors(this.system.schema);
        }
    }

    // Access component data directly
    get data() {
        const record = this.system.store[this.entity.getGuid()];
        return record ? record.data : null;
    }

    // Lifecycle methods
    onEnable() {}
    onDisable() {}
    onPostStateChange() {}
}
```

### 4. ComponentSystem - The Behavior

**File:** `src/framework/components/system.js`

Systems manage all components of a particular type:

```javascript
class ComponentSystem extends EventHandler {
    id;                    // System identifier (e.g., 'model', 'render')
    app;                   // Application reference
    store = {};            // Component data storage
    schema = [];           // Property schema

    constructor(app) {
        super();
        this.app = app;
    }

    // Create component on entity
    addComponent(entity, data = {}) {
        const component = new this.ComponentType(this, entity);
        const componentData = new this.DataType();

        this.store[entity.getGuid()] = {
            entity: entity,
            data: componentData
        };

        entity[this.id] = component;
        entity.c[this.id] = component;

        this.initializeComponentData(component, data, []);
        this.fire('add', entity, component);

        return component;
    }

    // Remove component from entity
    removeComponent(entity) {
        const record = this.store[entity.getGuid()];
        const component = entity.c[this.id];

        component.fire('beforeremove');
        this.fire('beforeremove', entity, component);

        delete this.store[entity.getGuid()];
        delete entity[this.id];
        delete entity.c[this.id];

        this.fire('remove', entity, record.data);
    }
}
```

### 5. ComponentSystemRegistry - The System Manager

**File:** `src/framework/components/registry.js`

Manages all component systems:

```javascript
class ComponentSystemRegistry extends EventHandler {
    list = [];        // Array of all systems

    // Individual system references
    anim;
    camera;
    collision;
    light;
    model;
    render;
    rigidbody;
    script;
    // ... more systems

    add(system) {
        const id = system.id;
        if (this[id]) {
            throw new Error(`ComponentSystem name '${id}' already registered`);
        }
        this[id] = system;
        this.list.push(system);
    }

    remove(system) {
        const id = system.id;
        delete this[id];
        const index = this.list.indexOf(this[id]);
        if (index !== -1) {
            this.list.splice(index, 1);
        }
    }
}
```

---

## Component Types

PlayCanvas includes these built-in component systems:

| Component | System ID | Description |
|-----------|-----------|-------------|
| Anim | `anim` | State-based animation controller |
| Animation | `animation` | Legacy animation system |
| AudioListener | `audiolistener` | 3D audio listener |
| Button | `button` | UI button interaction |
| Camera | `camera` | Camera rendering |
| Collision | `collision` | Physics collision shapes |
| Element | `element` | UI text and images |
| GSplat | `gsplat` | Gaussian splatting rendering |
| LayoutChild | `layoutchild` | UI layout child |
| LayoutGroup | `layoutgroup` | UI layout manager |
| Light | `light` | Lighting (directional, point, spot) |
| Model | `model` | 3D model rendering |
| ParticleSystem | `particlesystem` | Particle effects |
| Render | `render` | Low-level renderable |
| RigidBody | `rigidbody` | Physics body |
| Screen | `screen` | UI screen container |
| Script | `script` | Custom behavior scripts |
| Scrollbar | `scrollbar` | UI scrollbar |
| ScrollView | `scrollview` | UI scrollable area |
| Sound | `sound` | Sound emitter |
| Sprite | `sprite` | 2D sprite rendering |
| Zone | `zone` | Lightmapping zone |

---

## Component Lifecycle

```
┌─────────────────────────────────────────────────────────────────┐
│                    Component Lifecycle                          │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  1. CREATE                                                      │
│     Entity.addComponent(type, options)                          │
│          │                                                      │
│          ▼                                                      │
│     ComponentSystem.addComponent()                              │
│          │                                                      │
│          ▼                                                      │
│     new ComponentType(system, entity)                           │
│          │                                                      │
│          ▼                                                      │
│     initializeComponentData()                                   │
│          │                                                      │
│          ▼                                                      │
│     fire('add', entity, component)                              │
│                                                                 │
│  2. ENABLE (when entity enabled)                                │
│     component.onEnable()                                        │
│                                                                 │
│  3. UPDATE (every frame)                                        │
│     ComponentSystem.onUpdate(dt)                                │
│          │                                                      │
│          ▼                                                      │
│     Iterate all components in store                             │
│                                                                 │
│  4. DISABLE (when entity disabled)                              │
│     component.onDisable()                                       │
│                                                                 │
│  5. DESTROY                                                     │
│     Entity.removeComponent(type)                                │
│          │                                                      │
│          ▼                                                      │
│     fire('beforeremove')                                        │
│          │                                                      │
│          ▼                                                      │
│     delete from store                                           │
│          │                                                      │
│          ▼                                                      │
│     fire('remove')                                              │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

---

## Usage Example

```javascript
// Create an entity
const player = new pc.Entity('player');

// Add components
player.addComponent('model', {
    type: 'box'
});

player.addComponent('collision', {
    type: 'box',
    halfExtents: new pc.Vec3(0.5, 0.5, 0.5)
});

player.addComponent('rigidbody', {
    type: 'dynamic',
    mass: 10
});

player.addComponent('script');
player.script.create('playerController', {
    speed: 5,
    jumpForce: 10
});

// Add to scene
app.root.addChild(player);

// Access component data
player.model.type = 'asset';
player.model.asset = characterAsset;

// Component events
player.model.on('set', (name, oldValue, newValue) => {
    console.log(`Model.${name} changed from ${oldValue} to ${newValue}`);
});
```

---

## Custom Component Systems

Creating a custom component system:

```javascript
// 1. Create the component data class
class HealthData {
    constructor() {
        this.maxHealth = 100;
        this.currentHealth = 100;
        this.alive = true;
    }
}

// 2. Create the component class
class HealthComponent extends pc.Component {
    damage(amount) {
        this.currentHealth = Math.max(0, this.currentHealth - amount);
        if (this.currentHealth === 0) {
            this.alive = false;
            this.entity.fire('death');
        }
    }

    heal(amount) {
        this.currentHealth = Math.min(this.maxHealth, this.currentHealth + amount);
    }
}

// 3. Create the system class
class HealthSystem extends pc.ComponentSystem {
    constructor(app) {
        super(app);

        this.id = 'health';
        this.ComponentType = HealthComponent;
        this.DataType = HealthData;

        this.schema = [
            'maxHealth',
            'currentHealth',
            'alive'
        ];
    }

    update(dt) {
        // Regenerate health over time
        for (const id in this.store) {
            const data = this.store[id].data;
            if (data.alive && data.currentHealth < data.maxHealth) {
                data.currentHealth += dt * 5; // 5 HP per second
            }
        }
    }
}

// 4. Register the system
app.systems.add(new HealthSystem(app));
```

---

## Performance Optimizations

### 1. Object Pooling

Component systems use stores to avoid lookups:

```javascript
// Store structure
this.store = {
    'entity-guid-1': { entity: entity1, data: data1 },
    'entity-guid-2': { entity: entity2, data: data2 }
};
```

### 2. Schema-Based Property Access

Properties defined in schema get automatic getters/setters:

```javascript
// Schema definition
this.schema = ['maxHealth', 'currentHealth', 'alive'];

// Auto-generated accessor
Object.defineProperty(component, 'maxHealth', {
    get: function() {
        return this.data.maxHealth;
    },
    set: function(value) {
        const oldValue = this.data.maxHealth;
        this.data.maxHealth = value;
        this.fire('set', 'maxHealth', oldValue, value);
    }
});
```

### 3. Enable Order

Components have static `order` property for deterministic enable order:

```javascript
class RenderComponent extends Component {
    static order = 10;  // Enable after transforms (order 0)
}
```

### 4. Frozen Objects

Nodes can be frozen to skip hierarchy sync:

```javascript
// Mark node as static
node._frozen = true;

// Engine skips sync for frozen nodes and their children
```

---

## Entity Queries

Finding entities in the scene:

```javascript
// Find by name
const player = app.root.findByName('player');

// Find by path
const weapon = app.root.findByPath('player/hand/weapon');

// Find by tag
const enemies = app.root.findByTag('enemy');

// Find with custom predicate
const renderables = app.root.findBy((node) => node.model);

// Find all components of type
const cameras = app.root.findComponents('camera');

// Find by tag recursively
const allEnemies = app.root.findAllByTag('enemy');
```

---

## Script Component - Custom Behavior

The script component allows custom JavaScript/TypeScript behaviors:

```javascript
// script/player-controller.js
class PlayerController extends pc.ScriptType {
    // Define attributes (visible in editor)
    static attributes = {
        speed: {
            type: 'number',
            default: 5,
            title: 'Movement Speed'
        },
        jumpForce: {
            type: 'number',
            default: 10
        }
    };

    initialize() {
        // Called once when script starts
        this.velocity = new pc.Vec3();
        this.onGround = false;
    }

    update(dt) {
        // Called every frame
        const input = this.app.keyboard;

        if (input.isPressed(pc.KEY_W)) {
            this.velocity.z = this.speed;
        }

        if (input.isPressed(pc.KEY_SPACE) && this.onGround) {
            this.velocity.y = this.jumpForce;
            this.onGround = false;
        }

        // Apply movement
        this.entity.translate(this.velocity.x * dt, this.velocity.y * dt, this.velocity.z * dt);
    }
}

// Register the script
pc.registerScript(PlayerController, 'playerController');
```

---

## Architecture Best Practices

### DO:

- Use components for separate concerns (rendering, physics, audio)
- Keep component data simple and serializable
- Use the event system for cross-component communication
- Leverage the hierarchy for logical grouping

### DON'T:

- Store references to other entities directly in component data
- Access other components' internal data directly
- Create circular dependencies between components
- Perform heavy operations in update loops

---

## Comparison with Pure ECS

| Aspect | PlayCanvas | Pure ECS |
|--------|------------|----------|
| Entity | Object with components | Just an ID |
| Component | Object with data + reference to system | Just data (struct) |
| System | Processes components, has update loop | Query-based, data-only |
| Hierarchy | Built into GraphNode | Usually external |
| Flexibility | High - easy to mix patterns | Strict - data-only components |

PlayCanvas uses a "component-based" approach rather than pure ECS, trading some performance for flexibility and ease of use.
