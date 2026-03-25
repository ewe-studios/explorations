# PlayCanvas Animation System Deep Dive

## Overview

PlayCanvas features a powerful animation system supporting skeletal animation, morph targets, animation state machines, and blend trees. The system is designed for both character animation and general property animation.

---

## Architecture Diagram

```
┌─────────────────────────────────────────────────────────────────┐
│                    Animation Components                          │
│  ┌─────────────────┐  ┌─────────────────┐  ┌─────────────────┐ │
│  │ AnimComponent   │  │AnimationComponent│ │ MorphComponent  │ │
│  │ (State Machine) │  │  (Legacy)       │  │ (Blend Shapes)  │ │
│  └─────────────────┘  └─────────────────┘  └─────────────────┘ │
├─────────────────────────────────────────────────────────────────┤
│                    Animation Controller                          │
│  ┌───────────────────────────────────────────────────────────┐ │
│  │                    AnimController                          │ │
│  │  ┌─────────────┐  ┌─────────────┐  ┌─────────────────┐   │ │
│  │  │   States    │  │ Transitions │  │  Blend Trees    │   │ │
│  │  │  (Nodes)    │  │  (Conditions)│  │  (Mixing)       │   │ │
│  │  └─────────────┘  └─────────────┘  └─────────────────┘   │ │
│  └───────────────────────────────────────────────────────────┘ │
├─────────────────────────────────────────────────────────────────┤
│                    Animation Evaluator                           │
│  ┌───────────────────────────────────────────────────────────┐ │
│  │                    AnimEvaluator                           │ │
│  │  ┌─────────────┐  ┌─────────────┐  ┌─────────────────┐   │ │
│  │  │   Clips     │  │   Curves    │  │    Targets      │   │ │
│  │  └─────────────┘  └─────────────┘  └─────────────────┘   │ │
│  └───────────────────────────────────────────────────────────┘ │
├─────────────────────────────────────────────────────────────────┤
│                    Animation Binder                              │
│  ┌───────────────────────────────────────────────────────────┐ │
│  │  Resolves paths to actual properties (bones, materials)   │ │
│  └───────────────────────────────────────────────────────────┘ │
└─────────────────────────────────────────────────────────────────┘
```

---

## Animation Component (Anim)

### AnimComponent

**File:** `src/framework/components/anim/component.js`

The AnimComponent provides state-machine based animation control:

```javascript
class AnimComponent extends Component {
    constructor(system, entity) {
        super(system, entity);

        // State machine
        this.stateGraph = null;
        this.stateGraphAsset = null;

        // Animation clips
        this.assets = [];

        // Animation layers
        this.layers = [];

        // Root bone for skeletal animation
        this.rootBone = null;

        // Playback controls
        this.playing = false;
        this.speed = 1.0;

        // Evaluator (does the actual animation work)
        this.evaluator = null;

        // Controller (manages state machine)
        this.controllers = [];
    }

    // Create a new animation layer
    addLayer(layer) {
        const c = new AnimController(
            this.evaluator,
            layer.states,
            layer.transitions,
            layer.blendTrees,
            layer.enabled,
            layer.blendType,
            this.eventHandler
        );
        this.controllers.push(c);
        this.layers.push(layer);
    }

    // Assign animation clip to a state
    assignAnimation(stateName, asset) {
        const state = this.findState(stateName);
        if (state && asset) {
            state.clips = [asset.resource];
        }
    }

    // Play a specific animation
    play(stateName, layer = 0) {
        if (this.controllers[layer]) {
            this.controllers[layer].state = stateName;
            this.playing = true;
        }
    }

    // Update animation (called every frame)
    update(dt) {
        if (!this.playing) return;

        for (let i = 0; i < this.controllers.length; i++) {
            this.controllers[i].update(dt);
        }

        this.evaluator.evaluate(dt);
        this.evaluator.apply();
    }
}
```

### AnimStateGraph

**File:** `src/framework/anim/state-graph/anim-state-graph.js`

Defines the state machine structure:

```javascript
class AnimStateGraph {
    constructor() {
        this.states = [];
        this.transitions = [];
        this.parameters = [];
        this.events = [];
    }

    // Add a state
    addState(name, options = {}) {
        const state = {
            name: name,
            speed: options.speed ?? 1.0,
            loop: options.loop ?? true,
            blendTree: options.blendTree ?? null,
            clips: [],
            events: []
        };
        this.states.push(state);
        return state;
    }

    // Add transition between states
    addTransition(from, to, options = {}) {
        const transition = {
            from: from,
            to: to,
            duration: options.duration ?? 0.25,
            offset: options.offset ?? 0,
            restart: options.restart ?? false,
            conditions: options.conditions ?? [],
            interruptionSource: options.interruptionSource ?? ANIM_INTERRUPTION_NONE
        };
        this.transitions.push(transition);
        return transition;
    }

    // Add parameter (used in transition conditions)
    addParameter(name, type, defaultValue) {
        this.parameters.push({
            name: name,
            type: type,  // 'float', 'int', 'bool', 'trigger'
            value: defaultValue
        });
    }

    // Set parameter value (triggers transitions)
    setParameter(name, value) {
        const param = this.parameters.find(p => p.name === name);
        if (param) {
            param.value = value;
        }
    }
}
```

### AnimController

**File:** `src/framework/anim/controller/anim-controller.js`

Manages the animation state machine:

```javascript
class AnimController {
    constructor(animEvaluator, states, transitions, activate, eventHandler, findParameter, consumeTrigger) {
        this._animEvaluator = animEvaluator;
        this._eventHandler = eventHandler;
        this._findParameter = findParameter;
        this._consumeTrigger = consumeTrigger;

        // Build state map
        this._states = {};
        this._stateNames = [];
        for (const state of states) {
            this._states[state.name] = new AnimState(
                this,
                state.name,
                state.speed,
                state.loop,
                state.blendTree
            );
            this._stateNames.push(state.name);
        }

        // Build transitions
        this._transitions = transitions.map(t => new AnimTransition(t));

        // Initial state
        this._activeStateName = ANIM_STATE_START;
        this._playing = false;
        this._isTransitioning = false;
    }

    // Update state machine
    update(dt) {
        if (!this._playing) return;

        const activeState = this._states[this._activeStateName];

        // Update current state time
        this._timeInState += dt * activeState.speed;

        // Check for state exit
        if (activeState.loop) {
            this._timeInState = this._timeInState % activeState.duration;
        } else if (this._timeInState >= activeState.duration) {
            this._timeInState = activeState.duration;
            this._tryTransitionTo(ANIM_STATE_END);
        }

        // Check transition conditions
        this._checkTransitions();

        // Update evaluator with current state
        if (this._isTransitioning) {
            this._updateTransition(dt);
        } else {
            activeState.update(this._animEvaluator, this._timeInState);
        }
    }

    // Check if any transition conditions are met
    _checkTransitions() {
        if (this._isTransitioning) return;

        const activeState = this._activeStateName;
        const transitions = this._findTransitionsFromState(activeState);

        for (const transition of transitions) {
            if (this._checkConditions(transition.conditions)) {
                this._startTransition(transition);
                return;
            }
        }
    }

    // Check if all conditions for a transition are met
    _checkConditions(conditions) {
        for (const condition of conditions) {
            const param = this._findParameter(condition.parameterName);
            if (!this._evaluateCondition(condition, param)) {
                return false;
            }
        }
        return true;
    }

    // Evaluate a single condition
    _evaluateCondition(condition, parameter) {
        switch (condition.predicate) {
            case ANIM_GREATER_THAN:
                return parameter.value > condition.value;
            case ANIM_LESS_THAN:
                return parameter.value < condition.value;
            case ANIM_EQUAL_TO:
                return parameter.value === condition.value;
            case ANIM_GREATER_THAN_EQUAL_TO:
                return parameter.value >= condition.value;
            case ANIM_LESS_THAN_EQUAL_TO:
                return parameter.value <= condition.value;
            case ANIM_NOT_EQUAL_TO:
                return parameter.value !== condition.value;
        }
        return false;
    }

    // Start transitioning to a new state
    _startTransition(transition) {
        this._isTransitioning = true;
        this._transitionTime = 0;
        this._totalTransitionTime = transition.duration;
        this._previousStateName = this._activeStateName;
        this._activeStateName = transition.to;

        // Handle interruption
        this._transitionInterruptionSource = transition.interruptionSource;
    }

    // Update during transition
    _updateTransition(dt) {
        this._transitionTime += dt;

        const alpha = Math.min(this._transitionTime / this._totalTransitionTime, 1);

        const prevState = this._states[this._previousStateName];
        const newState = this._states[this._activeStateName];

        // Blend between states
        prevState.update(this._animEvaluator, this._timeInStateBefore, 1 - alpha);
        newState.update(this._animEvaluator, 0, alpha);

        if (alpha >= 1) {
            this._isTransitioning = false;
            this._timeInState = 0;
        }
    }
}
```

---

## Animation Evaluator

### AnimEvaluator

**File:** `src/framework/anim/evaluator/anim-evaluator.js`

The evaluator blends multiple animation clips and applies them to targets:

```javascript
class AnimEvaluator {
    constructor(binder) {
        this._binder = binder;
        this._clips = [];
        this._inputs = [];
        this._outputs = [];
        this._targets = {};
    }

    // Add animation clip
    addClip(clip) {
        const targets = this._targets;
        const binder = this._binder;

        // Get curves from clip
        const curves = clip.track.curves;
        const snapshot = clip.snapshot;

        const inputs = [];
        const outputs = [];

        for (let i = 0; i < curves.length; i++) {
            const curve = curves[i];
            const paths = curve.paths;

            for (let j = 0; j < paths.length; j++) {
                const path = paths[j];
                const resolved = binder.resolve(path);

                // Create target if doesn't exist
                let target = targets[resolved?.targetPath || null];
                if (!target && resolved) {
                    target = {
                        target: resolved,
                        value: [],
                        curves: 0,
                        blendCounter: 0
                    };

                    // Initialize value array based on component count
                    for (let k = 0; k < resolved.components; k++) {
                        target.value.push(0);
                    }

                    targets[resolved.targetPath] = target;
                }

                if (target) {
                    target.curves++;
                    inputs.push(snapshot._results[i]);
                    outputs.push(target);
                }
            }
        }

        this._clips.push(clip);
        this._inputs.push(inputs);
        this._outputs.push(outputs);
    }

    // Evaluate all clips at current time
    evaluate(dt) {
        // Reset blend counters
        for (const key in this._targets) {
            this._targets[key].blendCounter = 0;
        }

        // Evaluate each clip
        for (let i = 0; i < this._clips.length; i++) {
            const clip = this._clips[i];
            const inputs = this._inputs[i];
            const outputs = this._outputs[i];

            if (!clip.playing) continue;

            // Sample curves at current time
            clip.track.evaluate(clip.time, inputs);

            // Apply to outputs
            for (let j = 0; j < inputs.length; j++) {
                const input = inputs[j];
                const output = outputs[j];

                if (input === null) continue;

                // First blend for this target this frame - set value
                if (output.blendCounter === 0) {
                    for (let k = 0; k < input.length; k++) {
                        output.value[k] = input[k];
                    }
                } else {
                    // Subsequent blend - lerp with existing value
                    const alpha = 1 / (output.blendCounter + 1);
                    for (let k = 0; k < input.length; k++) {
                        output.value[k] = output.value[k] * (1 - alpha) + input[k] * alpha;
                    }
                }

                output.blendCounter++;
            }

            // Update clip time
            clip.time += dt * clip.speed;
            if (clip.loop) {
                clip.time = clip.time % clip.duration;
            }
        }
    }

    // Apply evaluated values to targets
    apply() {
        for (const key in this._targets) {
            const target = this._targets[key];
            if (target.target) {
                target.target.setValue(target.value);
            }
        }
    }

    // Remove clip
    removeClip(clip) {
        const index = this._clips.indexOf(clip);
        if (index !== -1) {
            this._clips.splice(index, 1);
            this._inputs.splice(index, 1);
            this._outputs.splice(index, 1);
        }
    }
}
```

### AnimClip

**File:** `src/framework/anim/evaluator/anim-clip.js`

Represents a single animation clip:

```javascript
class AnimClip {
    constructor(name, duration, tracks) {
        this.name = name;
        this.duration = duration;
        this.tracks = tracks;

        this.time = 0;
        this.speed = 1.0;
        this.playing = false;
        this.loop = true;

        this.snapshot = new AnimSnapshot(tracks);
    }

    play() {
        this.playing = true;
    }

    pause() {
        this.playing = false;
    }

    stop() {
        this.playing = false;
        this.time = 0;
    }
}

class AnimSnapshot {
    constructor(tracks) {
        this._results = [];
        for (let i = 0; i < tracks.length; i++) {
            this._results.push(null);
        }
    }
}
```

### AnimTrack

**File:** `src/framework/anim/evaluator/anim-track.js`

A track contains animation curves:

```javascript
class AnimTrack {
    constructor(name, duration, curves) {
        this.name = name;
        this.duration = duration;
        this.curves = curves;
    }

    // Evaluate all curves at given time
    evaluate(time, results) {
        for (let i = 0; i < this.curves.length; i++) {
            const curve = this.curves[i];
            results[i] = curve.valueAt(time);
        }
    }
}

class AnimCurve {
    constructor(type, name, paths, times, values, tangents) {
        this.type = type;       // 'value' | 'cubic' | 'step' | 'linear'
        this.name = name;       // Property name (e.g., 'localPosition')
        this.paths = paths;     // Paths to targets (e.g., ['Root/Hips'])
        this.times = times;     // Key times
        this.values = values;   // Key values
        this.tangents = tangents; // Tangents for cubic interpolation
    }

    // Get value at specific time
    valueAt(time) {
        // Find surrounding keys
        let i = 0;
        while (i < this.times.length && this.times[i] < time) {
            i++;
        }

        if (i === 0) {
            return this._getValue(0);
        }
        if (i >= this.times.length) {
            return this._getValue(this.times.length - 1);
        }

        const prev = i - 1;
        const next = i;
        const t = (time - this.times[prev]) / (this.times[next] - this.times[prev]);

        // Interpolate based on curve type
        switch (this.type) {
            case 'linear':
                return this._lerp(this._getValue(prev), this._getValue(next), t);
            case 'cubic':
                return this._cubic(prev, next, t);
            case 'step':
                return this._getValue(prev);
            default:
                return this._lerp(this._getValue(prev), this._getValue(next), t);
        }
    }

    _getValue(index) {
        const offset = index * this._components;
        return this.values.slice(offset, offset + this._components);
    }
}
```

### AnimBinder

**File:** `src/framework/anim/binder/anim-binder.js`

Resolves animation paths to actual properties:

```javascript
class AnimBinder {
    constructor(node) {
        this._node = node;
        this._cache = new Map();
    }

    // Resolve a path to a target
    resolve(path) {
        // Check cache
        if (this._cache.has(path)) {
            return this._cache.get(path);
        }

        // Parse path (e.g., "Root/Hips/leg_L")
        const parts = path.split('/');
        let node = this._node;

        for (const part of parts) {
            node = node?.children.find(c => c.name === part);
        }

        if (!node) {
            return null;
        }

        // Create target
        const target = {
            node: node,
            targetPath: path,
            components: 3, // Position/rotation/scale have 3 components
            setValue: (value) => {
                node.localPosition.set(value[0], value[1], value[2]);
            }
        };

        this._cache.set(path, target);
        return target;
    }
}
```

---

## Skeletal Animation

### Skin and SkinInstance

**File:** `src/scene/skin.js` and `src/scene/skin-instance.js`

```javascript
class Skin {
    constructor(device, boneNames, inverseBindMatrices) {
        this.device = device;
        this.boneNames = boneNames;
        this.inverseBindMatrices = inverseBindMatrices;

        // Root bone of the skeleton
        this.skeleton = null;
    }
}

class SkinInstance {
    constructor(skin) {
        this.skin = skin;
        this.device = skin.device;

        // Current bone matrices
        this.matrices = new Array(skin.boneNames.length);
        for (let i = 0; i < this.matrices.length; i++) {
            this.matrices[i] = new Mat4();
        }

        // GPU buffer for bone matrices
        this.boneTexture = null;
        this.boneTextureSize = 0;
    }

    // Update bone matrices from skeleton
    updateMatrices() {
        const skin = this.skin;
        const nodes = this._boneNodes;

        for (let i = 0; i < skin.boneNames.length; i++) {
            if (nodes[i]) {
                // World matrix * inverse bind matrix
                this.matrices[i].mul2(nodes[i].getWorldTransform(), skin.inverseBindMatrices[i]);
            }
        }

        this._dirty = true;
    }

    // Upload bone matrices to GPU
    updateBoneTexture() {
        if (!this._dirty) return;

        // Pack 4x4 matrices into texture
        const size = this.boneTextureSize;
        const data = new Float32Array(size * size * 4);

        for (let i = 0; i < this.matrices.length; i++) {
            const matrix = this.matrices[i];
            const offset = i * 16;

            // Upload as 4 RGBA pixels (4 components * 4 rows)
            data[offset * 4 + 0] = matrix.data[0];
            data[offset * 4 + 1] = matrix.data[1];
            data[offset * 4 + 2] = matrix.data[2];
            data[offset * 4 + 3] = matrix.data[3];
            // ... etc for all 16 elements
        }

        if (!this.boneTexture) {
            this.boneTexture = new Texture(this.device, {
                width: size,
                height: size,
                format: PIXELFORMAT_RGBA32F,
                mipmaps: false,
                minFilter: FILTER_NEAREST,
                magFilter: FILTER_NEAREST
            });
        }

        this.boneTexture.setData(data);
        this._dirty = false;
    }
}
```

### Skeleton Hierarchy

```javascript
// Build skeleton from hierarchy
function buildSkeleton(rootNode, boneNames) {
    const boneNodes = [];

    // Find each bone node by name
    for (const boneName of boneNames) {
        const boneNode = rootNode.findByName(boneName);
        if (boneNode) {
            boneNodes.push(boneNode);
        } else {
            console.warn(`Bone ${boneName} not found`);
            boneNodes.push(null);
        }
    }

    return boneNodes;
}
```

---

## Morph Targets

### Morph System

**File:** `src/scene/morph.js` and `src/scene/morph-instance.js`

```javascript
class Morph {
    constructor(device, targets) {
        this.device = device;
        this.targets = targets;

        // GPU resources
        this._vertexBuffer = null;
        this._textures = [];
    }
}

class MorphTarget {
    constructor(name, options) {
        this.name = name;

        // Delta data for each attribute
        this.deltaPositions = options.deltaPositions; // Float32Array
        this.deltaNormals = options.deltaNormals;     // Optional
        this.deltaTangents = options.deltaTangents;   // Optional

        // AABB for this morph
        this.aabb = options.aabb;
    }
}

class MorphInstance {
    constructor(morph) {
        this.morph = morph;
        this.device = morph.device;

        // Weights for each morph target
        this.weights = new Float32Array(morph.targets.length);

        // Max weights to use (for GPU optimization)
        this.maxWeights = 4;
    }

    // Set weight for a morph target
    setWeight(index, weight) {
        if (index >= 0 && index < this.weights.length) {
            this.weights[index] = weight;
        }
    }

    // Get weight for a morph target
    getWeight(index) {
        return this.weights[index] || 0;
    }

    // Find morph target by name
    findTarget(name) {
        return this.morph.targets.findIndex(t => t.name === name);
    }
}
```

### Morph Shader Integration

```glsl
// Vertex shader morph targets (GLSL)
#if defined(USE_MORPH_TARGETS)

uniform vec4 morph_weights[MORPH_TARGET_COUNT];
uniform sampler2D morph_position_texture;
uniform vec2 morph_texture_size;

void applyMorphTargets(inout vec3 position) {
    vec2 uv = gl_VertexID / morph_texture_size;

    for (int i = 0; i < MORPH_TARGET_COUNT; i++) {
        if (morph_weights[i].x == 0.0) continue;

        vec4 delta = texture(morph_position_texture, uv + vec2(float(i) * morph_texture_size.x, 0.0));
        position += delta.rgb * morph_weights[i].x;
    }
}

#endif
```

---

## Blend Trees

### Blend Tree Types

**File:** `src/framework/anim/controller/anim-node.js`

```javascript
class AnimBlendTree {
    constructor(type, parameters, children) {
        this.type = type;  // '1D' | '2D' | 'direct'
        this.parameter = parameters.parameter;
        this.parameter2 = parameters.parameter2;
        this.children = children;
    }
}

class AnimBlendTreeNode {
    constructor(name, animation, time, blendValue, position) {
        this.name = name;
        this.animation = animation;  // Animation clip
        this.time = time;            // Local time within clip
        this.blendValue = blendValue; // Blend weight
        this.position = position;    // Position in blend space
    }
}
```

### 1D Blend Tree

```javascript
// Example: Walk-to-run blend based on speed parameter
const blendTree = new AnimBlendTree('1D', { parameter: 'speed' }, [
    new AnimBlendTreeNode('walk', walkClip, 0, 0, 0),      // Blend at speed=0
    new AnimBlendTreeNode('jog', jogClip, 0, 0.5, 0.5),    // Blend at speed=0.5
    new AnimBlendTreeNode('run', runClip, 0, 1, 1)         // Blend at speed=1
]);

// When speed=0.3, result is ~60% walk + 40% jog
```

### 2D Blend Tree

```javascript
// Example: Directional movement blend
const blendTree = new AnimBlendTree('2D', {
    parameter: 'velocityX',
    parameter2: 'velocityY'
}, [
    new AnimBlendTreeNode('idle', idleClip, 0, 0, [0, 0]),
    new AnimBlendTreeNode('forward', walkFClip, 0, 0, [0, 1]),
    new AnimBlendTreeNode('backward', walkBClip, 0, 0, [0, -1]),
    new AnimBlendTreeNode('left', walkLClip, 0, 0, [-1, 0]),
    new AnimBlendTreeNode('right', walkRClip, 0, 0, [1, 0])
]);

// Blends based on 2D velocity vector
```

### Direct Blend Tree

```javascript
// Example: Layered animation (base + additive)
const blendTree = new AnimBlendTree('direct', {}, [
    new AnimBlendTreeNode('base', walkClip, 0, 1.0, 0),
    new AnimBlendTreeNode('additive', waveClip, 0, 0.5, 1)  // Additive at 50%
]);
```

---

## Animation Events

### Event System

```javascript
class AnimEvent {
    constructor(time, functionName, parameters) {
        this.time = time;           // Time in clip to fire event
        this.functionName = functionName;  // Function to call
        this.parameters = parameters;  // Parameters to pass
    }
}

// Animation clip with events
const clip = new AnimClip('attack', 1.5, tracks);
clip.events = [
    new AnimEvent(0.3, 'onAttackStart', []),
    new AnimEvent(0.5, 'onAttackHit', [damage]),
    new AnimEvent(0.8, 'onAttackEnd', [])
];

// Event handler in component
class AttackHandler extends pc.ScriptType {
    initialize() {
        this.entity.anim.on('animEvent', (event) => {
            if (event.functionName === 'onAttackHit') {
                this.dealDamage(event.parameters[0]);
            }
        });
    }
}
```

---

## Usage Examples

### Basic Character Animation

```javascript
// Initialize animation component
const entity = new pc.Entity('player');
entity.addComponent('anim', {
    activate: true,
    rootBone: entity.findByName('Root')
});

// Load and assign animations
app.assets.loadFromUrl('idle.json', 'anim-state-graph', (err, asset) => {
    entity.anim.stateGraph = asset.resource;

    app.assets.loadFromUrl('idle.glb', 'animation', (err, animAsset) => {
        entity.anim.assignAnimation('idle', animAsset);
    });

    app.assets.loadFromUrl('run.glb', 'animation', (err, animAsset) => {
        entity.anim.assignAnimation('run', animAsset);
    });
});

// Control animation from script
class PlayerAnimation extends pc.ScriptType {
    initialize() {
        this.speed = 0;
    }

    update(dt) {
        // Update blend parameter
        this.entity.anim.setFloat('speed', this.speed);

        // Trigger jump
        if (this.jumpPressed) {
            this.entity.anim.trigger('jump');
        }
    }
}
```

### Procedural Animation

```javascript
// Animate property directly
class ProceduralAnimation extends pc.ScriptType {
    initialize() {
        this.time = 0;

        // Create animation clip for custom property
        const track = new pc.AnimTrack('rotation', 1, [
            new pc.AnimCurve(
                'linear',
                'localEulerAngles',
                [''],
                [0, 1],
                [0, 0, 0, 360, 0, 0],
                null
            )
        ]);

        this.clip = new pc.AnimClip('spin', 1, [track]);
        this.clip.play();

        this.entity.anim.evaluator.addClip(this.clip);
    }

    update(dt) {
        this.entity.anim.evaluator.evaluate(dt);
        this.entity.anim.evaluator.apply();
    }
}
```

### Animation Blending

```javascript
// Layer multiple animations
class AnimationBlending extends pc.ScriptType {
    initialize() {
        // Base layer - locomotion
        this.entity.anim.addLayer({
            name: 'Locomotion',
            blendType: pc.ANIM_BLEND_ADDITIVE
        });

        // Upper body layer - actions
        this.entity.anim.addLayer({
            name: 'Actions',
            blendType: pc.ANIM_BLEND_ADDITIVE,
            mask: this.createUpperBodyMask()
        });
    }

    createUpperBodyMask() {
        // Only animate bones from spine up
        const mask = {};
        const bones = ['Spine', 'Spine1', 'Spine2', 'Neck', 'Head', 'Arm_L', 'Arm_R'];
        for (const bone of bones) {
            mask[bone] = true;
        }
        return mask;
    }
}
```

---

## Performance Optimizations

### 1. Animation Culling

```javascript
// Skip animation update when entity is not visible
class AnimComponent {
    update(dt) {
        if (!this.entity.enabled || !this.playing) return;

        // Check if entity is in view
        if (this.cullAnimation && !this.isVisible()) {
            return;
        }

        // ... normal update
    }
}
```

### 2. Bone Texture Optimization

```javascript
// Use bone textures instead of uniform arrays for many bones
class SkinInstance {
    updateBoneTexture() {
        // Determine optimal texture size
        const boneCount = this.matrices.length;
        const pixelsPerBone = 4; // 4x4 matrix as 4 RGBA pixels
        const textureWidth = Math.ceil(Math.sqrt(boneCount * pixelsPerBone));

        this.boneTextureSize = textureWidth;

        // Only update if changed
        if (!this._dirty) return;
        // ... update texture
    }
}
```

### 3. Layer Masking

```javascript
// Only update bones that are affected by a layer
class AnimLayer {
    constructor(options) {
        this.mask = options.mask || {};
        this._affectedBones = new Set();

        // Pre-calculate affected bones
        for (const boneName in this.mask) {
            if (this.mask[boneName]) {
                this._affectedBones.add(boneName);
            }
        }
    }

    update(dt) {
        // Only update masked bones
        for (const bone of this._affectedBones) {
            // ... update bone
        }
    }
}
```

---

## Summary

The PlayCanvas animation system provides:

1. **State Machine**: Powerful animation state machines with transitions and conditions
2. **Blend Trees**: 1D, 2D, and direct blend trees for smooth animation blending
3. **Skeletal Animation**: Full skeletal animation with GPU skinning via bone textures
4. **Morph Targets**: Vertex morphing for facial animation and detail
5. **Events**: Timeline events for syncing game logic with animation
6. **Layering**: Multiple animation layers with masking for complex behaviors
7. **Performance**: Optimizations including culling, bone textures, and layer masking
