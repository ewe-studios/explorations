---
location: /home/darkvoid/Boxxed/@formulas/src.UIFrameworks/src.animations/mojs/src
explored_at: 2026-03-20
---

# mo.js Forward/Backward Playback Tracking and Replay - Deep Dive

**Scope:** Animation direction tracking, yoyo playback, reverse playback, replay mechanics, speed control, state management, thenable chains for sequential playback

---

## Table of Contents

1. [Playback Architecture Overview](#1-playback-architecture-overview)
2. [Direction Detection System](#2-direction-detection-system)
3. [Yoyo Playback Mechanism](#3-yoyo-playback-mechanism)
4. [Reverse Playback System](#4-reverse-playback-system)
5. [Speed Control and Time Mapping](#5-speed-control-and-time-mapping)
6. [State Machine and Transitions](#6-state-machine-and-transitions)
7. [Thenable Chain Replay](#7-thenable-chain-replay)
8. [Timeline Child Replay](#8-timeline-child-replay)
9. [Callback Dispatch on Direction Changes](#9-callback-dispatch-on-direction-changes)
10. [Progress Tracking and Caching](#10-progress-tracking-and-caching)
11. [Pause/Resume State Preservation](#11-pauseresume-state-preservation)
12. [Visibility-Aware Playback](#12-visibility-aware-playback)

---

## 1. Playback Architecture Overview

### 1.1 Playback Component Hierarchy

```
┌─────────────────────────────────────────────────────────────────────┐
│                   mo.js PLAYBACK SYSTEM                              │
├─────────────────────────────────────────────────────────────────────┤
│                                                                      │
│  ┌──────────────────────────────────────────────────────────────┐   │
│  │                    Tween Class                                │   │
│  │  (tween/tween.babel.js - 1,276 lines)                        │   │
│  │                                                               │   │
│  │  ┌─────────────────┐  ┌─────────────────┐  ┌──────────────┐  │   │
│  │  │  Period Detector│  │ Direction Tracker│  │  Yoyo State  │  │   │
│  │  │  _getPeriod()   │  │  isForward       │  │  isYoyo      │  │   │
│  │  │  T = dTime/TTime│  │  time >= prevTime│  │  T % 2 === 1 │  │   │
│  │  └─────────────────┘  └─────────────────┘  └──────────────┘  │   │
│  │                                                               │   │
│  │  ┌─────────────────┐  ┌─────────────────┐  ┌──────────────┐  │   │
│  │  │  Speed Mapper   │  │  State Machine  │  │  Progress    │  │   │
│  │  │  speed * delta  │  │  play/pause/    │  │  Caching     │  │   │
│  │  │  time mapping   │  │  stop/reverse   │  │  _prevTime   │  │   │
│  │  └─────────────────┘  └─────────────────┘  └──────────────┘  │   │
│  └──────────────────────────────────────────────────────────────┘   │
│                              │                                       │
│                              ▼                                       │
│  ┌──────────────────────────────────────────────────────────────┐   │
│  │                    Timeline Class                             │   │
│  │  (tween/timeline.babel.js - 317 lines)                       │   │
│  │                                                               │   │
│  │  ┌─────────────────┐  ┌─────────────────┐                    │   │
│  │  │  Child Updater  │  │  Direction      │                    │   │
│  │  │  _updateChildren│  │  Propagation    │                    │   │
│  │  │  forward/back   │  │  isYoyo pass    │                    │   │
│  │  └─────────────────┘  └─────────────────┘                    │   │
│  └──────────────────────────────────────────────────────────────┘   │
│                              │                                       │
│                              ▼                                       │
│  ┌──────────────────────────────────────────────────────────────┐   │
│  │                    Tweener Singleton                          │   │
│  │  (tween/tweener.babel.js - 154 lines)                        │   │
│  │                                                               │   │
│  │  ┌─────────────────┐  ┌─────────────────┐                    │   │
│  │  │  RAF Loop       │  │  Visibility     │                    │   │
│  │  │  _loop()        │  │  Handler        │                    │   │
│  │  │  _update(time)  │  │  pause/resume   │                    │   │
│  │  └─────────────────┘  └─────────────────┘                    │   │
│  └──────────────────────────────────────────────────────────────┘   │
│                                                                      │
└─────────────────────────────────────────────────────────────────────┘
```

### 1.2 Playback Flow

```
user.play() / user.reverse() / user.playBackward()
        │
        ▼
┌─────────────────────────┐
│  Tween.play()           │
│  Set state = 'play'     │
│  _wasStarted = true     │
│  _timeShift = 0         │
└───────────┬─────────────┘
            │
            ▼
┌─────────────────────────┐
│  Tweener.add(tween)     │
│  Add to tweens array    │
│  Start RAF loop         │
└───────────┬─────────────┘
            │
            ▼
┌─────────────────────────┐
│  RAF: _loop()           │
│  _update(now)           │
└───────────┬─────────────┘
            │
            ▼
┌─────────────────────────┐
│  tween._update(time)    │
│  Apply speed mapping    │
│  Handle reverse         │
└───────────┬─────────────┘
            │
            ▼
┌─────────────────────────┐
│  _getPeriod(time)       │
│  Detect period number   │
│  Detect delay gap       │
└───────────┬─────────────┘
            │
            ▼
┌─────────────────────────┐
│  _getIsYoyo(time)       │
│  T % 2 === 1 check      │
└───────────┬─────────────┘
            │
            ▼
┌─────────────────────────┐
│  isForward = time >=    │
│            _prevTime    │
└───────────┬─────────────┘
            │
            ▼
┌─────────────────────────┐
│  _setProgress()         │
│  Apply easing based    │
│  on direction           │
└───────────┬─────────────┘
            │
            ▼
┌─────────────────────────┐
│  Callbacks dispatched   │
│  with direction info    │
└─────────────────────────┘
```

### 1.3 Key Playback State Variables

| Variable | Location | Purpose |
|----------|----------|---------|
| `_state` | `Tween` | Current state: 'play', 'pause', 'stop' |
| `_prevTime` | `Tween` | Previous frame time for direction detection |
| `_progressTime` | `Tween` | Time spent in active area (for pause) |
| `_playTime` | `Tween` | Time when play started (for speed) |
| `_wasStarted` | `Tween` | Whether animation was started |
| `_wasReversed` | `Tween` | Whether reverse was ever called |
| `_prevPeriod` | `Tween` | Previous period number |
| `_prevYoyo` | `Tween` | Previous yoyo state |
| `_prevEasedProgress` | `Tween` | Previous eased progress |
| `_wasUpdated` | `Tween` | Whether onUpdate fired |
| `_wasCompleted` | `Tween` | Whether onComplete fired |

---

## 2. Direction Detection System

### 2.1 Forward Detection

```javascript
// tween.babel.js - Line ~504
_update(time, timelinePrevTime, wasYoyo, onEdge) {
  var p = this._props;
  var isForward = time >= this._prevTime;

  // Use isForward for callback dispatch
  if (isForward) {
    // Forward playback callbacks
  } else {
    // Backward playback callbacks
  }
}
```

**Key Insight:** Direction is determined by comparing current time to previous time, NOT by any internal flag.

### 2.2 Direction Detection Timing

```
Frame N-1: time = 1000ms,  _prevTime = 900ms  → isForward = true  (1000 >= 900)
Frame N:   time = 1100ms,  _prevTime = 1000ms → isForward = true  (1100 >= 1000)

Reverse playback:
Frame N+1: time = 1050ms,  _prevTime = 1100ms → isForward = false (1050 < 1100)
Frame N+2: time = 1000ms,  _prevTime = 1050ms → isForward = false (1000 < 1050)
```

### 2.3 Direction-Aware Progress Setting

```javascript
// tween.babel.js - Lines 964-996
_setProgress(proc, time, isYoyo) {
  var p = this._props;
  var isYoyoChanged = p.wasYoyo !== isYoyo;
  var isForward = time > this._prevTime;

  this.progress = proc;

  // Get easing for forward direction
  if ((isForward && !isYoyo) || (!isForward && isYoyo)) {
    this.easedProgress = p.easing(proc);
  }
  // Get easing for backward direction
  else if ((!isForward && !isYoyo) || (isForward && isYoyo)) {
    var easing = (p.backwardEasing != null)
      ? p.backwardEasing : p.easing;
    this.easedProgress = easing(proc);
  }

  // Call onUpdate if eased progress changed
  if (p.prevEasedProgress !== this.easedProgress || isYoyoChanged) {
    if (p.onUpdate != null && typeof p.onUpdate === 'function') {
      p.onUpdate.call(
        p.callbacksContext || this,
        this.easedProgress, this.progress,
        isForward, isYoyo,
      );
    }
  }

  p.prevEasedProgress = this.easedProgress;
  p.wasYoyo = isYoyo;
  return this;
}
```

**Direction Logic Table:**

| isForward | isYoyo | Easing Used |
|-----------|--------|-------------|
| true | false | `easing` (forward) |
| false | false | `backwardEasing` or `easing` |
| true | true | `backwardEasing` or `easing` |
| false | true | `easing` (forward) |

### 2.4 Direction Visualization

```
Forward Playback:
time:      0 ──────▶ 500 ──────▶ 1000
prevTime:  ?         0           500
isForward: true      true        true

Backward Playback:
time:      1000 ◀───── 500 ◀───── 0
prevTime:  ?         1000        500
isForward: false     false       false

Yoyo Playback:
Period 0 (forward):  isForward=true,  isYoyo=false → forward easing
Period 1 (back):     isForward=false, isYoyo=true  → forward easing
Period 2 (forward):  isForward=true,  isYoyo=false → forward easing
Period 3 (back):     isForward=false, isYoyo=true  → forward easing
```

---

## 3. Yoyo Playback Mechanism

### 3.1 Yoyo Period Detection

```javascript
// tween.babel.js - Lines ~890-895
_getPeriod(time) {
  var p = this._props;
  var TTime = p.delay + p.duration;
  var dTime = p.delay + time - p.startTime;
  var T = dTime / TTime;
  var elapsed = (time < p.endTime) ? dTime % TTime : 0;

  // ... delay handling ...

  return Math.floor(T);  // Return period number
}

// tween.babel.js - Lines ~698-700
var isYoyo = props.isYoyo && (T % 2 === 1);
var isYoyoPrev = props.isYoyo && (prevT % 2 === 1);
```

**Key Formula:**
```
isYoyo = isYoyo && (periodNumber % 2 === 1)
```

### 3.2 Yoyo Progress Flipping

```javascript
// tween.babel.js - Lines 954-960
if (isYoyo) {
  proc = 1 - proc;  // Flip progress
  isReverse = !isReverse;
}
```

**Yoyo Progress Mapping:**

| Period | isYoyo | Raw Progress | Flipped Progress |
|--------|--------|--------------|------------------|
| 0 | false | 0 → 1 | 0 → 1 (forward) |
| 1 | true | 0 → 1 | 1 → 0 (backward) |
| 2 | false | 0 → 1 | 0 → 1 (forward) |
| 3 | true | 0 → 1 | 1 → 0 (backward) |

### 3.3 Yoyo Callback Dispatch

```javascript
// tween.babel.js - Lines 540-565
_updateInActiveArea(time) {
  var p = this._props;
  var T = this._getPeriod(time);
  var prevT = this._getPeriod(this._prevTime);

  var isYoyo = p.isYoyo && (T % 2 === 1);
  var isYoyoPrev = p.isYoyo && (prevT % 2 === 1);
  var yoyoZero = isYoyo ? 1 : 0;

  // Handle yoyo state change
  if (isYoyo !== isYoyoPrev) {
    // Yoyo direction changed - fire appropriate callbacks
    if (isYoyo && !isYoyoPrev) {
      // Transitioning to backward period
      this._callback('onRepeatComplete', [true, false]);
    } else {
      // Transitioning to forward period
      this._callback('onRepeatStart', [true, true]);
    }
  }

  // Continue with normal update
  this._setProgress(proc, time, isYoyo);
}
```

### 3.4 Yoyo Visualization

```
Yoyo Animation (repeat: 3, isYoyo: true, duration: 1000ms):

Timeline:
0ms          1000ms       2000ms       3000ms       4000ms
│────────────│────────────│────────────│────────────│
│  Period 0  │  Period 1  │  Period 2  │  Period 3  │
│  forward   │  backward  │  forward   │  backward  │
│  isYoyo=F  │  isYoyo=T  │  isYoyo=F  │  isYoyo=T  │
│            │            │            │            │
│  progress: │  progress: │  progress: │  progress: │
│  0 → 1     │  1 → 0     │  0 → 1     │  1 → 0     │
│            │            │            │            │
│  callbacks:│  callbacks:│  callbacks:│  callbacks:│
│  onStart   │  onRepeat  │  onRepeat  │  onRepeat  │
│  onUpdate  │  onUpdate  │  onUpdate  │  onUpdate  │
│            │  Complete  │  Start     │  Complete  │
│            │            │            │  onComplete│
```

---

## 4. Reverse Playback System

### 4.1 Reverse Method

```javascript
// tween.babel.js - Lines 354-376
playBackward(o) {
  if (!this._wasReversed) {
    this._wasReversed = true;
    this._props.isReversed = !this._props.isReversed;
  }

  // Set initial progress time for reverse
  var p = this._props;
  this._progressTime = (p.endTime - p.startTime) - this._progressTime;

  this._state = 'play';
  this._playTime = this._ currentTime;
  Tweener.add(this);
  this._callback('onPlaybackStart');
  return this;
}
```

### 4.2 Reverse Time Calculation

```javascript
// tween.babel.js - Line ~525
if (p.isReversed) {
  time = p.endTime - this._progressTime;
}
```

**Reverse Time Mapping:**

```
Normal Forward:
startTime ─────────────────▶ endTime
time = startTime + progressTime

Reverse:
endTime ◀────────────────── startTime
time = endTime - progressTime
```

### 4.3 Reverse State Variables

```javascript
// Initialization in _extendDefaults
this._wasReversed = false;  // Track if reverse was ever called

// In playBackward()
this._wasReversed = true;   // Mark as reversed
this._props.isReversed = !this._props.isReversed;  // Toggle flag
```

### 4.4 Reverse with Yoyo

```
Reverse + Yoyo Interaction:

Normal Yoyo:
Period 0: forward (0 → 1)
Period 1: backward (1 → 0)  ← yoyo
Period 2: forward (0 → 1)

Reverse Yoyo:
Period 2: forward (0 → 1)  ← start from end
Period 1: backward (1 → 0)  ← yoyo
Period 0: forward (0 → 1)
```

---

## 5. Speed Control and Time Mapping

### 5.1 Speed Variable

```javascript
// tween.babel.js - Line 99
speed: 1,  // Default speed multiplier
```

### 5.2 Speed Time Mapping

```javascript
// tween.babel.js - Lines ~507-510
if (p.speed && this._playTime) {
  time = this._playTime + (p.speed * (time - this._playTime));
}
```

**Speed Formula:**
```
mappedTime = playTime + speed × (realTime - playTime)
```

### 5.3 Speed Visualization

```
Normal Speed (speed: 1.0):
realTime:    0    500   1000  1500  2000
             │─────│─────│─────│─────│
mappedTime:  0    500   1000  1500  2000
animation:   0%   50%   100%

Half Speed (speed: 0.5):
realTime:    0    500   1000  1500  2000
             │─────│─────│─────│─────│
mappedTime:  0    250   500   750   1000
animation:   0%   25%   50%   75%   100%
             └──────── needs 2000ms real for 100% ────────┘

Double Speed (speed: 2.0):
realTime:    0    500   1000
             │─────│─────│
mappedTime:  0    1000  2000
animation:   0%   50%   100%
             └── completes in 500ms real ──┘
```

### 5.4 Play Time Tracking

```javascript
// tween.babel.js - play() method
play() {
  this._state = 'play';
  this._wasStarted = true;
  this._timeShift = 0;
  this._playTime = this._currentTime;  // Track when play started
  Tweener.add(this);
  this._callback('onPlaybackStart');
}
```

**Play Time Usage:**
- Used as reference point for speed calculations
- Allows speed changes mid-animation
- Enables accurate pause/resume

---

## 6. State Machine and Transitions

### 6.1 State Variables

```javascript
// tween.babel.js
this._state = 'stop';     // Current state: 'play', 'pause', 'stop'
this._wasStarted = false; // Whether animation ever started
this._wasCompleted = false; // Whether animation completed
```

### 6.2 Play Method

```javascript
// tween.babel.js - Lines 326-341
play() {
  if (this._state === 'play') { return this; }  // Already playing

  this._state = 'play';
  this._wasStarted = true;
  this._timeShift = 0;
  this._playTime = this._currentTime;
  Tweener.add(this);
  this._callback('onPlaybackStart');
  return this;
}
```

### 6.3 Pause Method

```javascript
// tween.babel.js - Lines 348-362
pause() {
  if (this._state !== 'play') { return this; }  // Not playing

  this._state = 'pause';
  this._pauseTime = this._currentTime;  // Save for resume

  Tweener.remove(this);
  this._callback('onPlaybackPause');
  return this;
}
```

### 6.4 Stop Method

```javascript
// tween.babel.js - Lines 369-386
stop(progress) {
  this._state = 'stop';
  this._wasStarted = false;

  Tweener.remove(this);

  // Set progress if provided
  if (progress != null) {
    this._setProgress(progress);
  }

  this._callback('onPlaybackStop');
  return this;
}
```

### 6.5 State Transition Diagram

```
                    ┌─────────────┐
              ┌────▶│   STOPPED   │◀────┐
              │     └──────┬──────┘     │
         stop()│            │play()     │stop()
              │            │            │
              │            ▼            │
              │     ┌─────────────┐    │
              │     │   PLAYING   │────┘
              │     └──────┬──────┘
              │            │
         pause()│            │reverse()
              │            │
              │            ▼            │
              │     ┌─────────────┐    │
              └─────│   PAUSED    │◀───┘
                    └─────────────┘
                         │
                    stop()│
                          ▼
                    ┌─────────────┐
                    │   REVERSE   │
                    └─────────────┘
```

### 6.6 State Callbacks

| State Change | Callback Fired |
|--------------|----------------|
| stop → play | `onPlaybackStart` |
| play → pause | `onPlaybackPause` |
| any → stop | `onPlaybackStop` |
| playing complete | `onPlaybackComplete` |

---

## 7. Thenable Chain Replay

### 7.1 Then Chain Structure

```javascript
// thenable.babel.js
then(o) {
  if ((o == null) || !Object.keys(o).length) { return 1; }

  const ModuleClass = this.constructor;
  const newModule = new ModuleClass(this._mergeThen(o));

  this._modules.push(newModule);
  newModule._o.prevChainModule = this._lastModule || this;
  this._lastModule = newModule;

  return newModule;
}
```

### 7.2 Chain Linking

```javascript
// Each module in chain has:
this._o.prevChainModule = previousModule;
```

**Chain Structure:**
```
Module 1 ──▶ Module 2 ──▶ Module 3 ──▶ Module 4
  │           │           │           │
  │           │           │           └─▶ onComplete
  │           │           └─▶ onComplete
  │           └─▶ onComplete
  └─▶ onComplete
```

### 7.3 Chained Delta Timeline

```javascript
// deltas.babel.js - Lines 87-95
_createDeltas(o) {
  // If chained - link to previous module's timeline
  if (this._o.isChained) {
    this.timeline = this._o.prevChainModule.timeline;
  } else {
    this.timeline = new Timeline();
  }

  // Add delta to timeline
  this.timeline.add(this._delta);
}
```

### 7.4 Chain Replay Flow

```javascript
// When playing a then chain:
const shape = new mojs.Shape({ radius: 50 })
  .then({ radius: 100, duration: 500 })
  .then({ radius: 50, duration: 500 })
  .then({ scale: 0, duration: 300 });

shape.play();  // Plays ALL modules sequentially
```

**Timeline:**
```
time:   0        500ms      1000ms     1300ms
        │────────│──────────│──────────│
Module: │ Mod 1  │  Mod 2   │  Mod 3   │
        │        │          │          │
Radius: 50 ──▶  100 ──────▶ 50 ──────▶ scale: 0
```

---

## 8. Timeline Child Replay

### 8.1 Timeline Child Update

```javascript
// timeline.babel.js - Lines 182-203
_updateChildren(p, time, isYoyo) {
  // Determine update direction
  var coef = (time > this._prevTime) ? -1 : 1;
  if (this._props.isYoyo && isYoyo) { coef *= -1; }

  var timeToTimelines = this._props.startTime + p * this._props.duration;
  var prevTimeToTimelines = timeToTimelines + coef;
  var len = this._timelines.length;

  for (var i = 0; i < len; i++) {
    // Determine iteration direction based on time direction
    var j = (timeToTimelines > prevTimeToTimelines)
      ? i : (len - 1) - i;

    this._timelines[j]._update(
      timeToTimelines,
      prevTimeToTimelines,
      this._prevYoyo,
      this._onEdge,
    );
  }
  this._prevYoyo = isYoyo;
}
```

### 8.2 Reverse Iteration

```
Forward Playback:
  i = 0, 1, 2, 3, 4  (normal order)

Backward Playback:
  i = 4, 3, 2, 1, 0  (reverse order)

This prevents visual glitches when animations reverse!
```

### 8.3 Timeline Direction Propagation

```javascript
// Child receives parent's yoyo state
this._timelines[j]._update(
  timeToTimelines,
  prevTimeToTimelines,
  this._prevYoyo,  // Parent's yoyo state passed to child
  this._onEdge,
);
```

---

## 9. Callback Dispatch on Direction Changes

### 9.1 Callback Types

```javascript
// tween.babel.js - _declareDefaults
_onProgress: null,      // Every frame (before others)
_onUpdate: null,        // Every frame in active area
_onStart: null,         // First frame of first period
_onComplete: null,      // Final period end
_onRepeatStart: null,   // Start of each repeat period
_onRepeatComplete: null,// End of each repeat period
_onFirstUpdate: null,   // First update in active area
```

### 9.2 Direction-Aware Callbacks

```javascript
// All direction callbacks receive (isForward, isYoyo)
_callback('onStart', [isForward, isYoyo]);
_callback('onUpdate', [easedProgress, progress, isForward, isYoyo]);
_callback('onComplete', [isForward, isYoyo]);
_callback('onRepeatStart', [isForward, isYoyo]);
_callback('onRepeatComplete', [isForward, isYoyo]);
```

### 9.3 Callback Dispatch Logic

```javascript
// tween.babel.js - Lines 540-590
_updateInActiveArea(time) {
  var p = this._props;
  var T = this._getPeriod(time);
  var prevT = this._getPeriod(this._prevTime);

  var isYoyo = p.isYoyo && (T % 2 === 1);
  var isYoyoPrev = p.isYoyo && (prevT % 2 === 1);
  var isForward = time > this._prevTime;

  // Period changed
  if (T !== prevT) {
    if (T !== 'delay' && prevT !== 'delay') {
      // Previous period completed
      this._callback('onRepeatComplete', [isForward, isYoyoPrev]);

      // New period started
      if (T !== Math.round(this._repeatT)) {
        this._callback('onRepeatStart', [isForward, isYoyo]);
      }
    }
  }

  // First period start
  if (T === 0 && !this._wasStarted) {
    this._callback('onStart', [isForward, isYoyo]);
  }

  // Set progress (triggers onUpdate)
  this._setProgress(proc, time, isYoyo);

  // Complete
  if (time >= p.endTime && !this._wasCompleted) {
    this._wasCompleted = true;
    this._callback('onComplete', [isForward, isYoyo]);
    this._callback('onPlaybackComplete');
  }
}
```

### 9.4 Callback Flow Example

```
Normal Playback (no yoyo, repeat: 1):

0ms: onStart(true, false)
     onFirstUpdate(true, false)
     onUpdate(ep, p, true, false) ← every frame

1000ms: onRepeatComplete(true, false)
        onRepeatStart(true, false)
        onUpdate(ep, p, true, false) ← every frame

2000ms: onComplete(true, false)
        onPlaybackComplete
```

---

## 10. Progress Tracking and Caching

### 10.1 Cached State Variables

```javascript
// tween.babel.js - State caching
this._prevTime = undefined;           // Previous frame time
this._prevPeriod = undefined;         // Previous period number
this._prevYoyo = undefined;           // Previous yoyo state
this._prevEasedProgress = undefined;  // Previous eased progress
this._wasUpdated = false;             // First update flag
this._wasCompleted = false;           // Completion flag
```

### 10.2 Progress State Update

```javascript
// At end of _update
this._prevTime = time;
this._prevPeriod = period;
this._prevYoyo = isYoyo;
```

### 10.3 Edge Progress Caching

```javascript
// When progress hits edges (0 or 1)
if (proc === 1 || proc === 0) {
  this.progress = proc;
}
```

### 10.4 Eased Progress Caching

```javascript
// _setProgress
p.prevEasedProgress = this.easedProgress;

// Only fire onUpdate if eased progress changed
if (p.prevEasedProgress !== this.easedProgress) {
  p.onUpdate.call(ctx, easedProgress, progress, isForward, isYoyo);
}
```

---

## 11. Pause/Resume State Preservation

### 11.1 Pause State Saving

```javascript
// pause() method
pause() {
  this._state = 'pause';
  this._pauseTime = this._currentTime;  // Save current time
  Tweener.remove(this);
  this._callback('onPlaybackPause');
}
```

### 11.2 Resume State Restoration

```javascript
// resume() method
resume() {
  if (this._state !== 'pause') { return this; }

  this._state = 'play';
  this._timeShift += this._currentTime - this._pauseTime;  // Adjust for gap
  Tweener.add(this);
  this._callback('onPlaybackStart');
}
```

### 11.3 Time Shift Calculation

```
Pause/Resume Timeline:

0ms          1000ms       2000ms       3000ms       4000ms
│────────────│────────────│────────────│────────────│
│  playing   │  paused    │  paused    │  playing   │
│            │◀──────── gap (1000ms) ────────▶│
│            │            │            │            │
_progressTime: 1000ms     1000ms      1000ms      1000ms

On resume:
_timeShift += currentTime - pauseTime
_timeShift += 3000 - 2000 = 1000ms

This shifts the animation start time to account for pause gap!
```

---

## 12. Visibility-Aware Playback

### 12.2 Visibility Handler

```javascript
// tweener.babel.js - Lines 125-148
_onVisibilityChange() {
  if (document[this._visibilityHidden]) {
    // Tab hidden - save and pause
    this._savePlayingTweens();
    for (var t of this._savedTweens) {
      t.pause();
    }
  } else {
    // Tab visible - restore and resume
    for (var t of this._savedTweens) {
      t.resume();
    }
    this._savedTweens = [];
  }
}
```

### 12.3 Save Playing Tweens

```javascript
// tweener.babel.js - Lines 109-118
_savePlayingTweens() {
  this._savedTweens = [];
  var i = this.tweens.length;
  while (i--) {
    var tween = this.tweens[i];
    if (tween._state === 'play') {
      this._savedTweens.push(tween);
    }
  }
}
```

### 12.4 Visibility Flow

```
Tab Visible → Tab Hidden → Tab Visible
     │           │            │
     │           │            │
     ▼           ▼            ▼
 playing    _savePlaying   resume all
            pause all      _savedTweens = []
```

---

## Appendix: Complete Playback State Diagram

```
┌─────────────────────────────────────────────────────────────────────┐
│                    PLAYBACK STATE MACHINE                             │
├─────────────────────────────────────────────────────────────────────┤
│                                                                      │
│                         ┌─────────────┐                             │
│                   ┌────▶│   STOPPED   │◀────┐                       │
│                   │     └──────┬──────┘     │                       │
│                   │            │            │                       │
│             stop()│     play() │            │stop()                 │
│                   │            │            │                       │
│                   │            ▼            │                       │
│                   │     ┌─────────────┐    │                       │
│                   │     │   PLAYING   │────┘                       │
│                   │     └──────┬──────┘                            │
│                   │            │                                   │
│                   │     pause()│            │                      │
│                   │            │            │reverse()             │
│                   │            ▼            │                      │
│                   │     ┌─────────────┐    │                       │
│                   └─────│   PAUSED    │◀───┘                       │
│                         └─────────────┘                            │
│                                                                      │
│  Direction Detection (every frame):                                 │
│  ┌─────────────────────────────────────────────────────────────┐    │
│  │  isForward = time >= _prevTime                               │    │
│  │  isYoyo = isYoyo && (period % 2 === 1)                      │    │
│  │                                                              │    │
│  │  if (isForward && !isYoyo) use forward easing               │    │
│  │  if (!isForward) use backwardEasing or easing               │    │
│  └─────────────────────────────────────────────────────────────┘    │
│                                                                      │
│  State Variables:                                                    │
│  ┌──────────────┬──────────────┬──────────────┬──────────────┐     │
│  │ _state       │ _prevTime    │ _progressTime│ _playTime    │     │
│  │ 'play/pause/ │ last frame   │ time in      │ when play    │     │
│  │  stop'       │ time         │ active area  │ started      │     │
│  └──────────────┴──────────────┴──────────────┴──────────────┘     │
│  ┌──────────────┬──────────────┬──────────────┬──────────────┐     │
│  │ _prevPeriod  │ _prevYoyo    │ _wasStarted  │ _wasCompleted│     │
│  │ period num   │ yoyo state   │ ever started │ ever complete│     │
│  └──────────────┴──────────────┴──────────────┴──────────────┘     │
│                                                                      │
└─────────────────────────────────────────────────────────────────────┘
```

---

## Summary

mo.js implements a sophisticated playback tracking system:

1. **Direction Detection:** Based on time comparison, not flags
2. **Yoyo System:** Period-based with automatic progress flipping
3. **Reverse Playback:** Time mapping with state preservation
4. **Speed Control:** Time scaling with play reference point
5. **State Machine:** Clean transitions between play/pause/stop
6. **Then Chains:** Sequential replay through linked timelines
7. **Timeline Children:** Direction-aware child updates
8. **Callbacks:** Full direction context in every callback
9. **Caching:** Extensive state caching for performance
10. **Visibility:** Automatic pause on tab hide

The key insight is that **direction is derived from time flow**, not stored state. This makes the system robust and eliminates edge cases.
