# PlayCanvas Physics System Deep Dive

## Overview

PlayCanvas integrates with the Ammo.js physics engine, which is a WebAssembly port of the Bullet Physics library. The physics system provides rigid body dynamics, collision detection, constraints, and raycasting.

---

## Architecture Diagram

```
┌─────────────────────────────────────────────────────────────────┐
│                    PlayCanvas Engine                             │
│  ┌───────────────────────────────────────────────────────────┐ │
│  │              Collision Component System                    │ │
│  │  ┌─────────────┐  ┌─────────────┐  ┌─────────────────┐   │ │
│  │  │  Collision  │  │   Trigger   │  │     Shapes      │   │ │
│  │  │  Component  │  │   Volume    │  │ (Box, Sphere)   │   │ │
│  │  └─────────────┘  └─────────────┘  └─────────────────┘   │ │
│  └───────────────────────────────────────────────────────────┘ │
│  ┌───────────────────────────────────────────────────────────┐ │
│  │            RigidBody Component System                      │ │
│  │  ┌─────────────┐  ┌─────────────┐  ┌─────────────────┐   │ │
│  │  │  RigidBody  │  │   Contact   │  │  RaycastResult  │   │ │
│  │  │  Component  │  │   Events    │  │                 │   │ │
│  │  └─────────────┘  └─────────────┘  └─────────────────┘   │ │
│  └───────────────────────────────────────────────────────────┘ │
├─────────────────────────────────────────────────────────────────┤
│                    Ammo.js (WASM)                                │
│  ┌───────────────────────────────────────────────────────────┐ │
│  │              Bullet Physics Engine                         │ │
│  │  ┌─────────────┐  ┌─────────────┐  ┌─────────────────┐   │ │
│  │  │  Dynamics   │  │  Collision  │  │    Constraint   │   │ │
│  │  │  World      │  │  Dispatch   │  │    Solver       │   │ │
│  │  └─────────────┘  └─────────────┘  └─────────────────┘   │ │
│  └───────────────────────────────────────────────────────────┘ │
└─────────────────────────────────────────────────────────────────┘
```

---

## RigidBody Component

### RigidBodyComponent

**File:** `src/framework/components/rigid-body/component.js`

```javascript
class RigidBodyComponent extends Component {
    constructor(system, entity) {
        super(system, entity);

        // Body type
        this.type = BODYTYPE_STATIC;  // STATIC | DYNAMIC | KINEMATIC

        // Mass (only for dynamic bodies)
        this.mass = 0;

        // Linear damping (air resistance)
        this.linearDamping = 0.0;

        // Angular damping (rotational resistance)
        this.angularDamping = 0.05;

        // Initial velocity
        this.linearVelocity = new Vec3(0, 0, 0);
        this.angularVelocity = new Vec3(0, 0, 0);

        // Gravity
        this.useGravity = true;

        // Physics flags
        this.enabled = true;
        this.isTrigger = false;

        // Ammo.js body reference
        this.body = null;

        // Contact events
        this.onContact = new EventHandler();
        this.onCollisionStart = new EventHandler();
        this.onCollisionEnd = new EventHandler();
    }

    // Apply force to the body
    applyForce(force, point) {
        if (!this.body) return;

        const ammoForce = new Ammo.btVector3(force.x, force.y, force.z);
        const ammoPoint = point ?
            new Ammo.btVector3(point.x, point.y, point.z) :
            null;

        if (ammoPoint) {
            this.body.applyForce(ammoForce, ammoPoint);
        } else {
            this.body.applyCentralForce(ammoForce);
        }

        Ammo.destroy(ammoForce);
        if (ammoPoint) Ammo.destroy(ammoPoint);
    }

    // Apply impulse (instant force)
    applyImpulse(impulse, point) {
        if (!this.body) return;

        const ammoImpulse = new Ammo.btVector3(impulse.x, impulse.y, impulse.z);
        const ammoPoint = point ?
            new Ammo.btVector3(point.x, point.y, point.z) :
            null;

        if (ammoPoint) {
            this.body.applyImpulse(ammoImpulse, ammoPoint);
        } else {
            this.body.applyCentralImpulse(ammoImpulse);
        }

        Ammo.destroy(ammoImpulse);
        if (ammoPoint) Ammo.destroy(ammoPoint);
    }

    // Apply torque (rotational force)
    applyTorque(torque) {
        if (!this.body) return;

        const ammoTorque = new Ammo.btVector3(torque.x, torque.y, torque.z);
        this.body.applyTorque(ammoTorque);
        Ammo.destroy(ammoTorque);
    }

    // Activate/deactivate body
    activate() {
        if (this.body) {
            this.body.activate();
        }
    }

    deactivate() {
        if (this.body) {
            this.body.deactivate();
        }
    }

    // Teleport body to new position
    teleport(position, rotation) {
        if (!this.body) return;

        const transform = this.body.getWorldTransform();
        const origin = transform.getOrigin();
        const quaternion = transform.getRotation();

        origin.setValue(position.x, position.y, position.z);

        if (rotation) {
            quaternion.setValue(rotation.x, rotation.y, rotation.z, rotation.w);
        }

        transform.setOrigin(origin);
        transform.setRotation(quaternion);
        this.body.setWorldTransform(transform);

        // Update entity transform
        this.entity.setPosition(position);
        if (rotation) {
            this.entity.setRotation(rotation);
        }
    }

    // Get velocity
    getLinearVelocity() {
        if (!this.body) return new Vec3(0, 0, 0);

        const velocity = this.body.getLinearVelocity();
        return new Vec3(velocity.x(), velocity.y(), velocity.z());
    }

    // Set velocity
    setLinearVelocity(velocity) {
        if (!this.body) return;

        const ammoVelocity = new Ammo.btVector3(velocity.x, velocity.y, velocity.z);
        this.body.setLinearVelocity(ammoVelocity);
        Ammo.destroy(ammoVelocity);
    }
}
```

### RigidBody Types

```javascript
// Static - doesn't move, infinite mass
const BODYTYPE_STATIC = 'static';

// Dynamic - fully simulated, affected by forces
const BODYTYPE_DYNAMIC = 'dynamic';

// Kinematic - moved by setting velocity/position, affects dynamic bodies
const BODYTYPE_KINEMATIC = 'kinematic';
```

---

## Collision Component

### CollisionComponent

**File:** `src/framework/components/collision/component.js`

```javascript
class CollisionComponent extends Component {
    constructor(system, entity) {
        super(system, entity);

        // Collision shape type
        this.type = 'box';  // box | sphere | capsule | cylinder | mesh | compound

        // Shape dimensions
        this.halfExtents = new Vec3(0.5, 0.5, 0.5);  // For box
        this.radius = 0.5;                            // For sphere/capsule/cylinder
        this.height = 2;                              // For capsule/cylinder
        this.axis = 1;                                // Axis for cylinder/capsule (0=X, 1=Y, 2=Z)

        // Asset for mesh collision
        this.asset = null;

        // Material properties
        this.material = new PhysicsMaterial();

        // Trigger mode (no physical response)
        this.isTrigger = false;

        // Contact offset for optimization
        this.contactOffset = 0.04;

        // Ammo.js collision shape
        this.shape = null;
    }

    // Create collision shape based on type
    _createShape() {
        switch (this.type) {
            case 'box':
                return this._createBoxShape();
            case 'sphere':
                return this._createSphereShape();
            case 'capsule':
                return this._createCapsuleShape();
            case 'cylinder':
                return this._createCylinderShape();
            case 'mesh':
                return this._createMeshShape();
            case 'compound':
                return this._createCompoundShape();
        }
    }

    _createBoxShape() {
        const halfExtents = this.halfExtents;
        return new Ammo.btBoxShape(
            new Ammo.btVector3(halfExtents.x, halfExtents.y, halfExtents.z)
        );
    }

    _createSphereShape() {
        return new Ammo.btSphereShape(this.radius);
    }

    _createCapsuleShape() {
        const radius = this.radius;
        const height = this.height;

        // Ammo capsule is aligned differently, needs adjustment
        let shape;
        switch (this.axis) {
            case 0: shape = new Ammo.btCapsuleShapeX(radius, height); break;
            case 2: shape = new Ammo.btCapsuleShapeZ(radius, height); break;
            default: shape = new Ammo.btCapsuleShape(radius, height);
        }
        return shape;
    }

    _createCylinderShape() {
        const halfExtents = new Ammo.btVector3(
            this.axis === 0 ? this.height / 2 : this.radius,
            this.axis === 1 ? this.height / 2 : this.radius,
            this.axis === 2 ? this.height / 2 : this.radius
        );
        return new Ammo.btCylinderShape(halfExtents);
    }

    _createMeshShape() {
        if (!this.asset) return null;

        const mesh = this.asset.resource;
        const positions = mesh.getPositions();
        const indices = mesh.getIndices();

        // Create triangle mesh shape
        const triangleMesh = new Ammo.btTriangleMesh();

        for (let i = 0; i < indices.length; i += 3) {
            const i0 = indices[i];
            const i1 = indices[i + 1];
            const i2 = indices[i + 2];

            const v0 = new Ammo.btVector3(
                positions[i0 * 3],
                positions[i0 * 3 + 1],
                positions[i0 * 3 + 2]
            );
            const v1 = new Ammo.btVector3(
                positions[i1 * 3],
                positions[i1 * 3 + 1],
                positions[i1 * 3 + 2]
            );
            const v2 = new Ammo.btVector3(
                positions[i2 * 3],
                positions[i2 * 3 + 1],
                positions[i2 * 3 + 2]
            );

            triangleMesh.addTriangle(v0, v1, v2, true);
        }

        // Use GImpact mesh for concave collision
        const shape = new Ammo.btGImpactMeshShape(triangleMesh);
        shape.updateGimpact();

        return shape;
    }
}
```

---

## Physics World

### RigidBodyComponentSystem

**File:** `src/framework/components/rigid-body/system.js`

```javascript
class RigidBodyComponentSystem extends ComponentSystem {
    constructor(app) {
        super(app);

        this.id = 'rigidbody';
        this.ComponentType = RigidBodyComponent;
        this.DataType = RigidBodyComponentData;

        // Physics world
        this._world = null;
        this._dispatcher = null;
        this._overlappingPairCache = null;
        this._solver = null;

        // Gravity
        this.gravity = new Vec3(0, -9.81, 0);

        // Contact events
        this.contactResult = [];

        // Raycast results
        this._raycastResult = [];

        // Physics material
        this.defaultContactMaterial = null;

        // Time step
        this.fixedTimeStep = 1 / 60;
        this.maxSubSteps = 10;
    }

    // Initialize physics world
    onLibraryLoaded() {
        // Create collision dispatcher
        this._dispatcher = new Ammo.btDefaultCollisionConfiguration();
        this._collisionDispatcher = new Ammo.btCollisionDispatcher(this._dispatcher);

        // Create broadphase (sweep and prune for better performance)
        this._overlappingPairCache = new Ammo.btDbvtBroadphase();

        // Create constraint solver
        this._solver = new Ammo.btSequentialImpulseConstraintSolver();

        // Create dynamics world
        this._world = new Ammo.btDiscreteDynamicsWorld(
            this._collisionDispatcher,
            this._overlappingPairCache,
            this._solver,
            this._dispatcher
        );

        // Set gravity
        const gravity = new Ammo.btVector3(
            this.gravity.x,
            this.gravity.y,
            this.gravity.z
        );
        this._world.setGravity(gravity);
        Ammo.destroy(gravity);

        // Contact result callback
        this._contactResult = new ContactResult();
    }

    // Update physics simulation
    update(dt) {
        if (!this._world) return;

        // Step the simulation
        this._world.stepSimulation(dt, this.maxSubSteps, this.fixedTimeStep);

        // Sync rigid bodies with their entities
        for (const id in this.store) {
            const entity = this.store[id].entity;
            const body = entity.rigidbody.body;

            if (body && entity.enabled && entity.rigidbody.enabled) {
                // Get new transform from physics
                const transform = body.getWorldTransform();
                const origin = transform.getOrigin();
                const quaternion = transform.getRotation();

                // Update entity position
                entity.setPosition(
                    origin.x(),
                    origin.y(),
                    origin.z()
                );

                // Update entity rotation
                entity.setRotation(
                    quaternion.x(),
                    quaternion.y(),
                    quaternion.z(),
                    quaternion.w()
                );
            }
        }

        // Fire contact events
        this._fireContactEvents();
    }

    // Add rigid body to world
    addBody(body) {
        if (!this._world) return;

        this._world.addRigidBody(body.body, body.collisionFilterGroup, body.collisionFilterMask);
    }

    // Remove rigid body from world
    removeBody(body) {
        if (!this._world) return;

        this._world.removeRigidBody(body.body);
    }
}
```

---

## Collision Detection

### Contact Results

**File:** `src/framework/components/rigid-body/system.js`

```javascript
// Contact point data
class ContactResult {
    constructor() {
        this.pointA = new Vec3();
        this.pointB = new Vec3();
        this.normalA = new Vec3();
        this.normalB = new Vec3();
        this.impulse = 0;
    }
}

// Contact callback
class ContactCallback {
    constructor() {
        this.add = [];      // Contacts added this frame
        this.persist = [];  // Contacts persisting from last frame
        this.remove = [];   // Contacts removed this frame
    }
}

// Contact result for single contact
class SingleContactResult {
    constructor(a, b, contact) {
        this.a = a;  // Entity A
        this.b = b;  // Entity B
        this.impulse = contact.impulse;
        this.localPointA = contact.localPointA;
        this.localPointB = contact.localPointB;
        this.pointA = contact.pointA;
        this.pointB = contact.pointB;
        this.normal = contact.normal;
    }
}

// Contact callback for collision events
class CollisionCallback {
    constructor() {
        this.other = null;           // Other entity
        this.point = new Vec3();     // Contact point
        this.normal = new Vec3();    // Contact normal
        this.relativeVelocity = new Vec3();
    }
}
```

### Contact Event Handling

```javascript
class RigidBodyComponent {
    // Contact event fired when in contact with another body
    onContact(other, contactResult) {
        // other: Other rigid body
        // contactResult: Array of contact points
    }

    // Collision start - first frame of contact
    onCollisionStart(other) {
        // Triggered when collision begins
    }

    // Collision end - contact broken
    onCollisionEnd(other) {
        // Triggered when collision ends
    }
}

// Usage example
class CollisionHandler extends pc.ScriptType {
    initialize() {
        this.entity.rigidbody.on('contact', this.onContact, this);
        this.entity.rigidbody.on('collisionstart', this.onCollisionStart, this);
        this.entity.rigidbody.on('collisionend', this.onCollisionEnd, this);
    }

    onContact(result) {
        console.log('Contact with:', result.other.name);
        console.log('Contact point:', result.point);
        console.log('Impulse:', result.impulse);
    }

    onCollisionStart(result) {
        console.log('Collision started with:', result.other.name);
    }

    onCollisionEnd(result) {
        console.log('Collision ended with:', result.other.name);
    }
}
```

---

## Raycasting

### RaycastResult

**File:** `src/framework/components/rigid-body/system.js`

```javascript
class RaycastResult {
    constructor(entity, point, normal, hitFraction) {
        this.entity = entity;           // Hit entity
        this.point = point;             // Hit point in world space
        this.normal = normal;           // Surface normal at hit point
        this.hitFraction = hitFraction; // Distance along ray (0-1)
    }
}

class RigidBodyComponentSystem {
    // Cast ray and return first hit
    raycastFirst(from, to, options) {
        if (!this._world) return null;

        const start = new Ammo.btVector3(from.x, from.y, from.z);
        const end = new Ammo.btVector3(to.x, to.y, to.z);

        const rayCallback = new Ammo.ClosestRayResultCallback(start, end);

        // Filter options
        if (options) {
            if (options.filterTags) {
                rayCallback.set_m_collisionFilterGroup(0);
                rayCallback.set_m_collisionFilterMask(0);
            }
        }

        this._world.rayTest(start, end, rayCallback);

        let result = null;

        if (rayCallback.hasHit()) {
            const body = Ammo.castObject(rayCallback.get_m_collisionObject(), Ammo.btRigidBody);
            const point = rayCallback.get_m_hitPointA();
            const normal = rayCallback.get_m_hitNormalA();

            // Find entity from body
            const entity = this._getBodyEntity(body);

            result = new RaycastResult(
                entity,
                new Vec3(point.x(), point.y(), point.z()),
                new Vec3(normal.x(), normal.y(), normal.z()),
                rayCallback.get_m_closestHitFraction()
            );
        }

        Ammo.destroy(start);
        Ammo.destroy(end);
        Ammo.destroy(rayCallback);

        return result;
    }

    // Cast ray and return all hits
    raycastAll(from, to, options) {
        if (!this._world) return [];

        const start = new Ammo.btVector3(from.x, from.y, from.z);
        const end = new Ammo.btVector3(to.x, to.y, to.z);

        const results = [];
        const callback = new Ammo.AllHitsRayResultCallback(start, end);

        this._world.rayTest(start, end, callback);

        if (callback.hasHit()) {
            const bodies = callback.get_m_collisionObjects();
            const points = callback.get_m_hitPointWorld();
            const normals = callback.get_m_hitNormalWorld();
            const fractions = callback.get_m_hitRayFraction();

            for (let i = 0; i < bodies.size(); i++) {
                const body = Ammo.castObject(bodies.at(i), Ammo.btRigidBody);
                const entity = this._getBodyEntity(body);

                results.push(new RaycastResult(
                    entity,
                    new Vec3(points.get(i).x(), points.get(i).y(), points.get(i).z()),
                    new Vec3(normals.get(i).x(), normals.get(i).y(), normals.get(i).z()),
                    fractions.get(i)
                ));
            }
        }

        Ammo.destroy(start);
        Ammo.destroy(end);
        Ammo.destroy(callback);

        return results;
    }
}
```

### Raycast Usage

```javascript
class RaycastExample extends pc.ScriptType {
    update(dt) {
        const from = this.entity.getPosition();
        const to = from.clone().add(pc.Vec3.FORWARD);
        to.y -= 1;

        const result = this.app.systems.rigidbody.raycastFirst(from, to);

        if (result) {
            console.log('Hit:', result.entity.name);
            console.log('Distance:', result.hitFraction * from.distance(to));
            console.log('Normal:', result.normal);
        }
    }
}

// Sphere casting (swept sphere)
const sphereCast = this.app.systems.rigidbody.sphereCastFirst(
    from, to, radius, options
);
```

---

## Constraints/Joints

### Constraint System

```javascript
// Point-to-point constraint (ball joint)
class PointToPointConstraint {
    constructor(bodyA, pivotA, bodyB, pivotB) {
        this.constraint = new Ammo.btPoint2PointConstraint(
            bodyA.body,
            bodyB.body,
            new Ammo.btVector3(pivotA.x, pivotA.y, pivotA.z),
            new Ammo.btVector3(pivotB.x, pivotB.y, pivotB.z)
        );
    }

    enable() {
        this.app.systems.rigidbody._world.addConstraint(this.constraint);
    }

    disable() {
        this.app.systems.rigidbody._world.removeConstraint(this.constraint);
    }
}

// Hinge constraint (door hinge)
class HingeConstraint {
    constructor(bodyA, bodyB, pivot, axis) {
        this.constraint = new Ammo.btHingeConstraint(
            bodyA.body,
            bodyB.body,
            new Ammo.btVector3(pivot.x, pivot.y, pivot.z),
            new Ammo.btVector3(axis.x, axis.y, axis.z),
            true, true
        );
    }
}

// Fixed constraint (weld joint)
class FixedConstraint {
    constructor(bodyA, bodyB, transform) {
        this.constraint = new Ammo.btFixedConstraint(
            bodyA.body,
            bodyB.body,
            transform
        );
    }
}

// Slider constraint (linear slide)
class SliderConstraint {
    constructor(bodyA, bodyB, pivot, axis) {
        this.constraint = new Ammo.btSliderConstraint(
            bodyA.body,
            bodyB.body,
            new Ammo.btVector3(pivot.x, pivot.y, pivot.z),
            new Ammo.btVector3(axis.x, axis.y, axis.z),
            true
        );
    }

    setLimits(lower, upper) {
        this.constraint.setLowerLinLimit(lower);
        this.constraint.setUpperLinLimit(upper);
    }
}

// Cone twist constraint (shoulder joint)
class ConeTwistConstraint {
    constructor(bodyA, bodyB, pivot, axis) {
        this.constraint = new Ammo.btConeTwistConstraint(
            bodyA.body,
            bodyB.body,
            new Ammo.btVector3(pivot.x, pivot.y, pivot.z),
            new Ammo.btVector3(axis.x, axis.y, axis.z)
        );
    }

    setLimit(swingSpan1, swingSpan2, twistSpan) {
        this.constraint.setLimit(swingSpan1, swingSpan2, twistSpan);
    }
}
```

---

## Physics Materials

### PhysicsMaterial

```javascript
class PhysicsMaterial {
    constructor() {
        // Friction coefficient (0 = no friction, 1 = high friction)
        this.friction = 0.5;

        // Restitution (bounciness: 0 = no bounce, 1 = full bounce)
        this.restitution = 0;

        // Linear damping override
        this.linearDamping = null;

        // Angular damping override
        this.angularDamping = null;
    }
}

// Usage
const iceMaterial = new PhysicsMaterial();
iceMaterial.friction = 0.1;  // Slippery
iceMaterial.restitution = 0; // No bounce

const bouncyMaterial = new PhysicsMaterial();
bouncyMaterial.friction = 0.3;
bouncyMaterial.restitution = 0.9;  // Very bouncy
```

---

## Collision Filters

### Collision Groups and Masks

```javascript
// Define collision groups
const COLLISION_GROUP_DEFAULT = 0x0001;
const COLLISION_GROUP_PLAYER = 0x0002;
const COLLISION_GROUP_ENEMY = 0x0004;
const COLLISION_GROUP_TRIGGER = 0x0008;
const COLLISION_GROUP_PROJECTILE = 0x0010;

// Define collision masks (what each group collides with)
const COLLISION_MASK_DEFAULT = COLLISION_GROUP_DEFAULT | COLLISION_GROUP_PLAYER | COLLISION_GROUP_ENEMY;
const COLLISION_MASK_PLAYER = COLLISION_GROUP_DEFAULT | COLLISION_GROUP_ENEMY | COLLISION_GROUP_TRIGGER;
const COLLISION_MASK_ENEMY = COLLISION_GROUP_DEFAULT | COLLISION_GROUP_PLAYER;
const COLLISION_MASK_TRIGGER = COLLISION_GROUP_PLAYER;

// Apply to rigid body
const playerBody = entity.rigidbody;
playerBody.collisionFilterGroup = COLLISION_GROUP_PLAYER;
playerBody.collisionFilterMask = COLLISION_MASK_PLAYER;

// Enemy won't collide with other enemies
const enemyBody = enemyEntity.rigidbody;
enemyBody.collisionFilterGroup = COLLISION_GROUP_ENEMY;
enemyBody.collisionFilterMask = COLLISION_MASK_ENEMY;
```

---

## Vehicle Physics

### RaycastVehicle

```javascript
class Vehicle {
    constructor(chassisBody, options) {
        this.vehicle = new Ammo.btRaycastVehicle(
            chassisBody.body,
            new Ammo.btDefaultVehicleTuning()
        );

        this.wheels = [];
    }

    addWheel(options) {
        const wheelInfo = new Ammo.btWheelInfoConstructionInfo();

        wheelInfo.set_m_chassisConnectionPointCS(
            new Ammo.btVector3(
                options.connectionPoint.x,
                options.connectionPoint.y,
                options.connectionPoint.z
            )
        );
        wheelInfo.set_m_wheelDirectionCS(
            new Ammo.btVector3(0, -1, 0)
        );
        wheelInfo.set_m_wheelAxleCS(
            new Ammo.btVector3(-1, 0, 0)
        );
        wheelInfo.set_m_suspensionRestLength(options.suspensionRestLength);
        wheelInfo.set_m_wheelsRadius(options.wheelRadius);
        wheelInfo.set_m_suspensionStiffness(options.suspensionStiffness);
        wheelInfo.set_m_wheelsDampingRelaxation(options.dampingRelaxation);
        wheelInfo.set_m_wheelsDampingCompression(options.dampingCompression);
        wheelInfo.set_m_maxSuspensionForce(options.maxSuspensionForce);
        wheelInfo.set_m_maxSuspensionTravelCm(options.maxSuspensionTravel);
        wheelInfo.set_m_rollInfluence(options.rollInfluence);
        wheelInfo.set_m_frictionSlip(options.frictionSlip);
        wheelInfo.set_m_isFrontWheel(options.isFrontWheel);

        this.vehicle.addWheel(wheelInfo);
        this.wheels.push(options);
    }

    applyEngineForce(force, wheelIndex) {
        this.vehicle.applyEngineForce(force, wheelIndex);
    }

    setSteeringValue(value, wheelIndex) {
        this.vehicle.setSteeringValue(value, wheelIndex);
    }

    setBrake(brakeForce, wheelIndex) {
        this.vehicle.setBrake(brakeForce, wheelIndex);
    }

    updateVehicle(dt) {
        this.vehicle.updateVehicle(dt);
    }
}
```

---

## Performance Optimizations

### 1. Sleeping

```javascript
// Bodies automatically sleep when at rest
class RigidBodyComponent {
    // Body will go to sleep after being still
    body.setActivationState(1); // DISABLE_DEACTIVATION - never sleep
    body.setActivationState(2); // ENABLE_DEACTIVATION - can sleep (default)
    body.setActivationState(3); // DISABLE_LATER - sleep next frame
    body.setActivationState(4); // WANTS_DEACTIVATION - wants to sleep
}
```

### 2. Collision Margins

```javascript
// Small collision margin improves performance
shape.setMargin(0.04);  // Default margin
shape.setMargin(0.01);  // Smaller margin for precision
```

### 3. Compound Shapes

```javascript
// Use compound shapes instead of mesh collision when possible
const compound = new Ammo.btCompoundShape();

// Add child shapes
const childShape1 = new Ammo.btBoxShape(new Ammo.btVector3(0.5, 0.5, 0.5));
const childTransform = new Ammo.btTransform();
childTransform.setIdentity();
childTransform.setOrigin(new Ammo.btVector3(0, 0, 0));
compound.addChildShape(childTransform, childShape1);

const childShape2 = new Ammo.btBoxShape(new Ammo.btVector3(0.3, 0.3, 0.3));
const childTransform2 = new Ammo.btTransform();
childTransform2.setIdentity();
childTransform2.setOrigin(new Ammo.btVector3(0, 1, 0));
compound.addChildShape(childTransform2, childShape2);
```

---

## Memory Management

### Ammo.js Memory Handling

```javascript
// Ammo.js objects must be manually destroyed to free WASM memory

// Correct pattern
function applyForce(body, force) {
    const ammoForce = new Ammo.btVector3(force.x, force.y, force.z);
    body.applyCentralForce(ammoForce);
    Ammo.destroy(ammoForce);  // Important!
}

// Reuse objects when possible
class PhysicsHelper {
    constructor() {
        // Cache commonly used objects
        this._tempVector = new Ammo.btVector3(0, 0, 0);
        this._tempTransform = new Ammo.btTransform();
    }

    applyForce(body, force) {
        this._tempVector.setValue(force.x, force.y, force.z);
        body.applyCentralForce(this._tempVector);
        // No destroy needed - we reuse the object
    }
}
```

---

## Summary

The PlayCanvas physics system provides:

1. **Rigid Body Dynamics**: Full 3D rigid body simulation with forces, impulses, and torques
2. **Collision Detection**: Multiple collision shapes (box, sphere, capsule, cylinder, mesh)
3. **Constraints**: Point-to-point, hinge, fixed, slider, and cone twist joints
4. **Raycasting**: Single and multiple raycast queries with filtering
5. **Contact Events**: Collision start/end and continuous contact callbacks
6. **Vehicles**: Raycast vehicle physics with suspension and steering
7. **Performance**: Collision filtering, sleeping, and optimization options
