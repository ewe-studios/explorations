# Rive Animation System: Deep Dive

**Source:** `/home/darkvoid/Boxxed/@formulas/src.rust/src.rive/rive-runtime/src/animation/`

---

## Table of Contents

1. [Overview](#overview)
2. [Animation Architecture](#animation-architecture)
3. [Linear Animation System](#linear-animation-system)
4. [State Machine System](#state-machine-system)
5. [Interpolation Algorithms](#interpolation-algorithms)
6. [Timeline and Keyframe Systems](#timeline-and-keyframe-systems)
7. [Event System](#event-system)
8. [Animation Authoring and Playback](#animation-authoring-and-playback)

---

## Overview

The Rive animation system provides two complementary animation paradigms:

1. **Linear Animations**: Traditional timeline-based keyframe animations
2. **State Machines**: Interactive, state-driven animations with blend trees

### Key Files

| File | Lines | Purpose |
|------|-------|---------|
| `linear_animation.cpp` | ~450 | Linear animation definition |
| `linear_animation_instance.cpp` | ~1,100 | Runtime animation playback |
| `state_machine.cpp` | ~160 | State machine definition |
| `state_machine_instance.cpp` | ~2,100 | Runtime state execution |
| `state_transition.cpp` | ~190 | Transition logic |
| `transition_viewmodel_condition.cpp` | ~950 | Transition conditions |
| `keyed_object.cpp` | ~80 | Keyed object base |
| `keyed_property.cpp` | ~140 | Property keyframes |
| `keyframe_interpolator.cpp` | ~50 | Interpolation base |
| `blend_animation_*.cpp` | ~200 | Blend tree animations |
| `nested_state_machine.cpp` | ~150 | Hierarchical state machines |

### Animation Data Flow

```
┌───────────────────────────────────────────────────────────────────┐
│                     .riv File (Authored)                          │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────────────────┐   │
│  │  Keyframes  │  │   States    │  │    Blend Trees          │   │
│  │  Timelines  │  │ Transitions │  │    Conditions           │   │
│  └─────────────┘  └─────────────┘  └─────────────────────────┘   │
└───────────────────────────────────────────────────────────────────┘
                              │
                              ▼ (loaded)
┌───────────────────────────────────────────────────────────────────┐
│                   Runtime Objects (C++)                           │
│  ┌─────────────────┐  ┌─────────────────────────────────────┐    │
│  │ LinearAnimation │  │ StateMachine                        │    │
│  │ - KeyedObjects  │  │ - Layers[]                          │    │
│  │ - Duration      │  │ - Inputs[] (Bool, Number, Trigger)  │    │
│  │ - FPS           │  │ - Transitions[]                     │    │
│  └─────────────────┘  └─────────────────────────────────────┘    │
└───────────────────────────────────────────────────────────────────┘
                              │
                              ▼ (instantiated)
┌───────────────────────────────────────────────────────────────────┐
│                  Runtime Instances                                │
│  ┌─────────────────────────┐  ┌─────────────────────────────┐    │
│  │ LinearAnimationInstance │  │ StateMachineInstance        │    │
│  │ - currentTime           │  │ - activeState               │    │
│  │ - timeDirection         │  │ - inputValueCache           │    │
│  │ - loopCount             │  │ - transitionProgress        │    │
│  └─────────────────────────┘  └─────────────────────────────┘    │
└───────────────────────────────────────────────────────────────────┘
                              │
                              ▼ (applied)
┌───────────────────────────────────────────────────────────────────┐
│                    Artboard (Modified)                            │
│  - Transform values updated                                       │
│  - Property values changed                                        │
│  - Events triggered                                               │
└───────────────────────────────────────────────────────────────────┘
```

---

## Animation Architecture

### Core Concepts

**Artboard**: The root container holding all animatable objects

**Component**: Base class for all objects in the hierarchy

**KeyedObject**: Object that can be animated via keyframes

**StateMachine**: Container for states, inputs, and transitions

### Class Hierarchy

```
Core
 └── Component
      ├── TransformComponent
      │    └── Node
      │         ├── Artboard
      │         ├── Shape
      │         ├── Bone
      │         └── ...
      ├── KeyedObject
      │    └── KeyedProperty<T>
      ├── LinearAnimation
      └── StateMachine
```

### Update Cycle

```cpp
// Simplified animation update cycle
void Artboard::advance(float elapsedSeconds) {
    // 1. Update animations
    for (auto& animation : m_animationInstances) {
        animation->update(elapsedSeconds);  // Advances time
        animation->apply(this);             // Applies values
    }

    // 2. Update state machines
    for (auto& sm : m_stateMachineInstances) {
        sm->update(elapsedSeconds);
    }

    // 3. Dirty propagation and transform updates
    updateTransforms();
}
```

---

## Linear Animation System

### LinearAnimation Class

```cpp
// From linear_animation.cpp
class LinearAnimation : public Animation {
    float m_Duration;          // Duration in frames
    float m_FPS;               // Frames per second
    Loop m_Loop;               // oneShot, loop, pingPong
    bool m_Quantize;           // Snap to frame boundaries

    std::vector<KeyedObject*> m_KeyedObjects;

    void apply(Artboard* artboard, float time, float mix) const {
        if (quantize()) {
            // Snap to nearest frame
            time = std::floor(time * fps()) / fps();
        }
        for (const auto& object : m_KeyedObjects) {
            object->apply(artboard, time, mix);
        }
    }
};
```

### Loop Modes

```
oneShot:
    0 ──────────────────────────────► duration
    (plays once, stops at end)

loop:
    0 ──────► duration 0 ──────► duration
    (wraps to start)
    └────────┘

pingPong:
    0 ──────► duration ◄────── 0
    (bounces back and forth)
    └────────┘
```

### Time Calculation

```cpp
// From linear_animation.cpp
float LinearAnimation::globalToLocalSeconds(float seconds) const {
    switch (loop()) {
        case Loop::oneShot:
            return seconds + startTime();

        case Loop::loop:
            // Positive modulus for wrapping
            return positiveMod(seconds, durationSeconds()) + startTime();

        case Loop::pingPong:
            float localTime = positiveMod(seconds, durationSeconds());
            int direction = ((int)(seconds / durationSeconds())) % 2;
            return direction == 0
                ? localTime + startTime()     // Forward
                : endTime() - localTime;      // Backward
    }
}

// Positive modulus (Dart-style)
static float positiveMod(float value, float range) {
    float result = fmod(value, range);
    return (result < 0) ? result + range : result;
}
```

### LinearAnimationInstance

Runtime instance that tracks playback state:

```cpp
class LinearAnimationInstance {
    LinearAnimation* m_Animation;
    float m_Time;              // Current time in seconds
    float m_TimeDirection;     // 1.0 or -1.0 for pingPong
    int m_LoopCount;           // Number of loops completed

    bool update(float elapsedSeconds) {
        m_Time += elapsedSeconds * m_Animation->speed() * m_TimeDirection;

        // Check for loop completion
        if (m_Time >= m_Animation->endTime()) {
            handleLoopCompletion();
            return false; // Animation ended
        }
        return true; // Continue playing
    }

    void apply(Artboard* artboard, float mix = 1.0f) {
        m_Animation->apply(artboard, m_Time, mix);
    }
};
```

---

## State Machine System

### State Machine Architecture

```
┌─────────────────────────────────────────────────────────┐
│                   StateMachine                          │
│  ┌──────────────────────────────────────────────────┐   │
│  │ Layer 0 (Base)                                   │   │
│  │  ┌──────────┐    ┌──────────┐    ┌──────────┐    │   │
│  │  │  State A │───►│  State B │───►│  State C │    │   │
│  │  │          │◄───│          │    │          │    │   │
│  │  └──────────┘    └──────────┘    └──────────┘    │   │
│  └──────────────────────────────────────────────────┘   │
│  ┌──────────────────────────────────────────────────┐   │
│  │ Layer 1 (Overlay)                                │   │
│  │  ┌──────────┐    ┌──────────┐                    │   │
│  │  │  Idle    │───►│  Active  │                    │   │
│  │  └──────────┘    └──────────┘                    │   │
│  └──────────────────────────────────────────────────┘   │
│                                                         │
│  Inputs: [isRunning: bool, speed: number, jump: trigger]│
└─────────────────────────────────────────────────────────┘
```

### StateMachine Definition

```cpp
// From state_machine.cpp
class StateMachine {
    std::vector<StateMachineLayer*> m_Layers;
    std::vector<StateMachineInput*> m_Inputs;
    std::vector<StateMachineListener*> m_Listeners;
    std::vector<DataBind*> m_dataBinds;

    // Access methods
    const StateMachineInput* input(std::string name) const;
    const StateMachineLayer* layer(std::string name) const;
};
```

### State Machine Instance

The runtime execution context:

```cpp
// From state_machine_instance.cpp (simplified)
class StateMachineInstance {
    StateMachine* m_StateMachine;
    Artboard* m_Artboard;

    // Runtime state
    std::vector<StateInstance*> m_States;
    std::vector<float> m_InputValueCache;
    std::vector<bool> m_ListenerFired;

    bool update(float elapsedSeconds) {
        // 1. Update all active states
        for (auto& state : m_States) {
            if (state->active()) {
                state->update(elapsedSeconds);
            }
        }

        // 2. Check transition conditions
        for (auto& state : m_States) {
            if (state->active()) {
                auto* transition = state->checkTransitions(*this);
                if (transition != nullptr) {
                    startTransition(transition);
                    break;
                }
            }
        }

        // 3. Update active transitions
        updateTransitions(elapsedSeconds);

        return hasActiveAnimation();
    }
};
```

### State Transitions

```cpp
// From state_transition.cpp
class StateTransition {
    State* m_From;
    State* m_To;
    float m_Duration;
    BlendAnimation* m_BlendAnimation;
    std::vector<TransitionCondition*> m_Conditions;

    bool canTransition(const StateMachineInstance& instance) const {
        // All conditions must be true
        for (const auto& condition : m_Conditions) {
            if (!condition->evaluate(instance)) {
                return false;
            }
        }
        return true;
    }
};
```

### Transition Conditions

```cpp
// From transition_viewmodel_condition.cpp (excerpt)
class TransitionViewModelCondition : public TransitionCondition {
    ViewModelPropertyComparator* m_Comparator;

    bool evaluate(const StateMachineInstance& instance) override {
        auto* viewModel = instance.viewModel();
        if (viewModel == nullptr) return false;

        auto value = viewModel->getProperty(m_PropertyPath);
        return m_Comparator->compare(value);
    }
};

// Condition types:
// - NumberCondition: value == target, value > target, etc.
// - BoolCondition: value == true/false
// - TriggerCondition: trigger was fired
```

### Input Types

```cpp
// Input types supported:
enum class InputType {
    Bool,       // True/false toggle
    Number,     // Floating-point value
    Trigger,    // One-shot event
    // Also: String, Color (for data binding)
};

// Setting inputs from runtime:
void StateMachineInstance::setInputBool(const std::string& name, bool value) {
    auto index = inputIndex(name, InputType::Bool);
    if (index != INVALID_INDEX) {
        m_InputValueCache[index] = value ? 1.0f : 0.0f;
        markInputDirty(index);
    }
}
```

---

## Interpolation Algorithms

### Keyframe Interpolation

Rive supports multiple interpolation methods:

### 1. Linear Interpolation

```cpp
// From keyframe_interpolator.cpp
template<typename T>
T lerp(const T& a, const T& b, float t) {
    return a + (b - a) * t;
}

// Usage for numbers:
float interpolateNumber(float from, float to, float t) {
    return from + (to - from) * t;
}

// Usage for transforms:
Vec2D interpolateVec2D(const Vec2D& from, const Vec2D& to, float t) {
    return Vec2D(
        from.x + (to.x - from.x) * t,
        from.y + (to.y - from.y) * t
    );
}
```

### 2. Cubic Bezier Interpolation (Custom Easing)

```
Cubic Bezier Curve:
    y
    │         ● (1, 1)
    │       ╱
    │     ● P2 (control point 2)
    │   ╱
    │ ● P1 (control point 1)
    │╱
    └─────────────── x
   (0, 0)

Formula:
    B(t) = (1-t)³P₀ + 3(1-t)²tP₁ + 3(1-t)t²P₂ + t³P₃

    where:
    - P₀ = (0, 0) - start
    - P₃ = (1, 1) - end
    - P₁, P₂ = control points (define the curve shape)
```

```cpp
// From cubic_interpolator_solver.cpp
class CubicInterpolatorSolver {
    float m_P1x, m_P1y;  // First control point
    float m_P2x, m_P2y;  // Second control point

    // Solve for t given x value
    float solveT(float x) const {
        // Newton-Raphson iteration
        float t = x;  // Initial guess
        for (int i = 0; i < 8; i++) {
            float xT = bezierX(t);
            float xDeriv = bezierXDerivative(t);
            if (std::abs(xDeriv) < 1e-6) break;
            t -= (xT - x) / xDeriv;
        }
        return clamp(t, 0.0f, 1.0f);
    }

    float evaluate(float x) const {
        float t = solveT(x);
        return bezierY(t);
    }
};
```

### 3. Elastic Easing

```cpp
// From elastic_ease.cpp
class ElasticEase {
    float m_Oscillations;   // Number of bounces
    float m_Damping;        // Decay rate

    float evaluate(float t) const {
        if (t <= 0 || t >= 1) return t;

        float decay = std::pow(2.0f, -10.0f * t);
        float oscillation = std::sin((t * m_Oscillations - 0.1f) * math::PI * 2);

        return decay * oscillation + 1.0f;
    }
};

// Visual representation:
//
//     1.0 ┼      ╱╲    ╱╲
//         │     ╱  ╲  ╱  ╲   ╱
//         │    ╱    ╲╱    ╲ ╱
//     0.5 ┼   ╱              ╲
//         │  ╱
//     0.0 ┼─╱────────────────────────
//         0  0.25  0.5  0.75  1.0
```

### 4. Step Interpolation (Discrete)

```cpp
// No interpolation - instant change at keyframe
template<typename T>
T stepInterpolate(const T& from, const T& to, float t) {
    return (t >= 1.0f) ? to : from;
}
```

### Keyframe Types

```cpp
// From keyframe_*.cpp files:

class KeyframeDouble : public KeyframeInterpolator<float> {
    // Interpolates floating-point values
    float interpolate(float time) const;
};

class KeyframeColor : public KeyframeInterpolator<Color> {
    // Interpolates RGBA colors
    Color interpolate(float time) const {
        return Color(
            lerp(m_from.r, m_to.r, t),
            lerp(m_from.g, m_to.g, t),
            lerp(m_from.b, m_to.b, t),
            lerp(m_from.a, m_to.a, t)
        );
    }
};

class KeyframeBool : public Keyframe<bool> {
    // Boolean values don't interpolate - they snap
    bool interpolate(float time) const {
        return (time >= m_time) ? m_value : m_previousValue;
    }
};
```

---

## Timeline and Keyframe Systems

### Keyframe Structure

```
Timeline for "rotation" property:

Time (seconds)
0.0    0.5    1.0    1.5    2.0
│      │      │      │      │
●──────●──────●──────●──────●
0°    90°    180°   270°   360°

Each keyframe contains:
- Time: When the keyframe occurs
- Value: The property value
- Interpolation: How to reach the next keyframe
```

### KeyedObject System

```cpp
// From keyed_object.cpp
class KeyedObject {
    std::vector<KeyedPropertyBase*> m_Properties;

    void apply(Artboard* artboard, float time, float mix) const {
        for (const auto& prop : m_Properties) {
            // Each property interpolates its value
            prop->apply(artboard, time, mix);
        }
    }
};

class KeyedPropertyBase {
    std::vector<KeyframeBase*> m_Keyframes;

    void apply(Artboard* artboard, float time, float mix) const {
        // Find surrounding keyframes
        auto* before = findKeyframeBefore(time);
        auto* after = findKeyframeAfter(time);

        if (before == nullptr || after == nullptr) {
            return; // No interpolation possible
        }

        // Calculate interpolation factor
        float t = (time - before->time()) / (after->time() - before->time());

        // Apply custom easing if present
        if (before->interpolator()) {
            t = before->interpolator()->evaluate(t);
        }

        // Interpolate and apply
        auto value = before->interpolate(*after, t);
        applyToArtboard(artboard, value, mix);
    }
};
```

### Work Area

```
Full Timeline:
├────────────────────────────────────────┤
0                                       duration

Work Area (subset for looping):
├──────────┬────────────────┬──────────┤
0        workStart        workEnd    duration
           ▲────────────────▲
           This region loops
```

```cpp
// From linear_animation.cpp
float LinearAnimation::startSeconds() const {
    return (enableWorkArea() ? (float)workStart() : 0.0f) / (float)fps();
}

float LinearAnimation::durationSeconds() const {
    return std::abs(endSeconds() - startSeconds());
}
```

---

## Event System

### Listener System

Rive provides an event-driven system for runtime interaction:

```cpp
// From listener_*.cpp
class ListenerNumberChange : public StateMachineListener {
    void onInputChanged(StateMachineInstance* instance,
                        StateMachineInput* input) override {
        // Called when a number input changes
        float value = input->asNumber()->value();
        triggerAction(instance, value);
    }
};

class ListenerTriggerChange : public StateMachineListener {
    void onInputChanged(StateMachineInstance* instance,
                        StateMachineInput* input) override {
        // Called when a trigger fires
        if (input->asTrigger()->wasFired()) {
            triggerAction(instance);
        }
    }
};
```

### Fire Events

```cpp
// From animation_reset.cpp
class AnimationResetAction {
    void fire(Artboard* artboard) {
        // Reset animation to start
        for (auto& instance : artboard->animationInstances()) {
            instance->time(0.0f);
        }
    }
};
```

### Event Propagation

```
Event Flow:

User Input (click, touch, key)
       │
       ▼
┌─────────────────┐
│ Input Listener  │
└─────────────────┘
       │
       ▼
┌─────────────────┐
│ State Machine   │
│ Input Change    │
└─────────────────┘
       │
       ▼
┌─────────────────┐
│   Transition    │
│   Condition     │
└─────────────────┘
       │
       ▼
┌─────────────────┐
│  State Change   │
│  (Animation)    │
└─────────────────┘
```

---

## Animation Authoring and Playback

### Authoring Workflow (Rive Editor)

1. **Create Artboard**: Set up the scene hierarchy
2. **Add Shapes/Objects**: Create visual elements
3. **Create Animation**: Add a LinearAnimation or StateMachine
4. **Set Keyframes**: Record property values at specific times
5. **Configure Interpolation**: Choose easing curves
6. **Add State Logic** (for state machines):
   - Define states
   - Create transitions
   - Set up conditions
7. **Export**: Generate .riv file

### Playback (Runtime)

```cpp
// Typical runtime usage pattern

// 1. Load file
auto* file = loadRiveFile("animation.riv");

// 2. Get artboard
auto* artboard = file->artboard("Artboard");

// 3. Get animation or state machine
auto* animation = artboard->animation("Walk Cycle");
auto* instance = animation->createInstance();

// 4. Main loop
float lastTime = currentTimeSeconds();

while (running) {
    float now = currentTimeSeconds();
    float elapsed = now - lastTime;
    lastTime = now;

    // Update animation
    instance->update(elapsed);
    instance->apply(artboard);

    // Render
    artboard->draw(renderer);

    // Check for completion
    if (!instance->isPlaying()) {
        break; // Animation finished
    }
}
```

### State Machine Runtime Usage

```cpp
// State machine interaction pattern

auto* smInstance = artboard->stateMachine("Player");

// Set inputs based on game state
smInstance->setInputBool("isRunning", player.isRunning);
smInstance->setInputNumber("speed", player.speed);

if (player.jumped) {
    smInstance->fireInputTrigger("jump");
}

// Update and apply
smInstance->update(elapsedTime);
smInstance->apply(artboard);
```

### Blend Trees

```
Blend Tree for locomotion:

                    ┌──────────┐
                    │  Blend2D │
                    │  (x, y)  │
                    └────┬─────┘
                         │
         ┌───────────────┼───────────────┐
         │               │               │
    ┌────▼────┐    ┌────▼────┐    ┌────▼────┐
    │  Idle   │    │  Walk   │    │  Run    │
    │ (0, 0)  │    │ (0.5, 0)│    │ (1, 0)  │
    └─────────┘    └─────────┘    └─────────┘

Input: velocity.x, velocity.y
Output: Blended animation pose
```

```cpp
// From blend_animation_direct.cpp
class BlendAnimationDirect : public BlendAnimation {
    std::vector<Animation*> m_Animations;
    std::vector<float> m_Weights;

    void apply(Artboard* artboard, float mix) {
        for (size_t i = 0; i < m_Animations.size(); i++) {
            if (m_Weights[i] > 0.0f) {
                m_Animations[i]->apply(artboard, m_Weights[i] * mix);
            }
        }
    }
};
```

---

## Summary

The Rive animation system provides:

1. **Linear Animations**: Keyframe-based with multiple interpolation modes
2. **State Machines**: Hierarchical states with conditional transitions
3. **Blend Trees**: Smooth blending between multiple animations
4. **Event System**: Listeners and triggers for interactivity
5. **Data Binding**: Reactive property connections

For related topics:
- `rendering-engine-deep-dive.md` - How animations are rendered
- `cpp-core-architecture.md` - Core object system
- `rust-revision.md` - How to replicate in Rust
