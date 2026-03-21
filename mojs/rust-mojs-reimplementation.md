# Rust Reimplementation Guide: mojs Animation System

**Target:** Complete Rust/WASM reimplementation of mojs motion graphics library
**Source:** mojs v1.7.1 (@mojs/core)
**Based on:** 11 comprehensive deep-dive exploration documents

---

## Table of Contents

1. [Crate Structure & Dependencies](#1-crate-structure--dependencies)
2. [Module Organization](#2-module-organization)
3. [Core Tween Engine](#3-core-tween-engine)
4. [Delta System](#4-delta-system)
5. [Easing System](#5-easing-system)
6. [Timeline Composition](#6-timeline-composition)
7. [Rendering System](#7-rendering-system)
8. [MotionPath](#8-motionpath)
9. [Burst/Particle System](#9-burstparticle-system)
10. [Stagger Patterns](#10-stagger-patterns)
11. [Performance Patterns](#11-performance-patterns)
12. [WASM Integration](#12-wasm-integration)
13. [Complete Example](#13-complete-example)

---

## 1. Crate Structure & Dependencies

### 1.1 Cargo.toml

```toml
[package]
name = "mojs-rs"
version = "0.1.0"
edition = "2021"
authors = ["Your Name <your@email.com>"]
description = "Rust reimplementation of mojs motion graphics library"
license = "MIT"
repository = "https://github.com/your/repo"
keywords = ["animation", "motion-graphics", "wasm", "svg", "web"]
categories = ["graphics", "web-programming", "wasm"]

[lib]
crate-type = ["cdylib", "rlib"]

[dependencies]
# Core dependencies
wasm-bindgen = "0.2"
wasm-bindgen-futures = "0.4"
js-sys = "0.3"

# Web APIs
web-sys = { version = "0.3", features = [
    "Window",
    "Document",
    "Element",
    "HtmlElement",
    "SvgElement",
    "SvgPathElement",
    "SvgGraphicsElement",
    "Node",
    "NodeList",
    "CssStyleDeclaration",
    "DomTokenList",
    "EventTarget",
    "RequestAnimationFrameCallback",
    "VisibilityState",
    "Performance",
    "console",
] }

# Serialization
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
serde-wasm-bindgen = "0.6"

# Math utilities
nalgebra = { version = "0.33", features = ["serde-serialize"] }

# Color handling
palette = { version = "0.7", features = ["serializing"] }

# Error handling
thiserror = "1.0"
anyhow = "1.0"

# Logging
web-time = "0.2"  # Browser-compatible time
log = "0.4"
wasm-logger = "0.2"

# Optional: For headless/server-side rendering
[features]
default = ["web"]
web = []
headless = []

[dev-dependencies]
wasm-bindgen-test = "0.3"
console_error_panic_hook = "0.1"

[profile.release]
opt-level = "s"
lto = true
```

### 1.2 Directory Structure

```
mojs-rs/
├── Cargo.toml
├── src/
│   ├── lib.rs                 # Main entry point, WASM exports
│   ├── mojs.rs                # Main API facade
│   │
│   ├── tween/
│   │   ├── mod.rs
│   │   ├── tween.rs           # Core tween engine
│   │   ├── timeline.rs        # Timeline composition
│   │   ├── tweener.rs         # Global RAF loop
│   │   └── state.rs           # Playback state types
│   │
│   ├── delta/
│   │   ├── mod.rs
│   │   ├── delta.rs           # Single delta calculator
│   │   ├── deltas.rs          # Multiple deltas manager
│   │   ├── types.rs           # Delta type enums
│   │   └── interpolate.rs     # Interpolation traits
│   │
│   ├── easing/
│   │   ├── mod.rs
│   │   ├── bezier.rs          # Cubic bezier easing
│   │   ├── path.rs            # SVG path easing
│   │   ├── approximate.rs     # Function sampling
│   │   ├── registry.rs        # Easing registry
│   │   └── presets.rs         # Built-in easing presets
│   │
│   ├── render/
│   │   ├── mod.rs
│   │   ├── svg.rs             # SVG rendering
│   │   ├── html.rs            # HTML element animation
│   │   ├── shape.rs           # Shape types
│   │   └── cache.rs           # Attribute caching
│   │
│   ├── shape/
│   │   ├── mod.rs
│   │   ├── circle.rs
│   │   ├── rect.rs
│   │   ├── polygon.rs
│   │   ├── line.rs
│   │   ├── cross.rs
│   │   └── custom.rs
│   │
│   ├── path/
│   │   ├── mod.rs
│   │   ├── motion_path.rs     # Path-based animation
│   │   ├── parser.rs          # SVG path parsing
│   │   └── sampler.rs         # Path sampling
│   │
│   ├── burst/
│   │   ├── mod.rs
│   │   ├── burst.rs           # Burst system
│   │   └── pool.rs            # Object pooling
│   │
│   ├── stagger/
│   │   ├── mod.rs
│   │   └── patterns.rs        # Stagger patterns
│   │
│   ├── utils/
│   │   ├── mod.rs
│   │   ├── color.rs           # Color parsing/conversion
│   │   ├── unit.rs            # CSS unit handling
│   │   ├── math.rs            # Math utilities
│   │   └── dom.rs             # DOM helpers
│   │
│   └── types.rs               # Shared types
│
├── examples/
│   ├── basic_shape.rs
│   ├── burst_animation.rs
│   ├── timeline_composition.rs
│   └── motion_path.rs
│
├── tests/
│   ├── tween_tests.rs
│   ├── delta_tests.rs
│   └── easing_tests.rs
│
└── www/                       # WASM demo site
    ├── index.html
    ├── bootstrap.js
    └── index.js
```

---

## 2. Module Organization

### 2.1 Core Module Pattern (lib.rs)

```rust
use wasm_bindgen::prelude::*;
use web_sys::{Window, Document, Element};
use std::cell::RefCell;
use std::rc::Rc;

// Re-export main modules
pub mod tween;
pub mod delta;
pub mod easing;
pub mod render;
pub mod shape;
pub mod path;
pub mod burst;
pub mod stagger;
pub mod utils;
pub mod types;

// Main API facade
pub struct Mojs {
    tweener: Rc<RefCell<tween::Tweener>>,
}

#[wasm_bindgen]
impl Mojs {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        console_error_panic_hook::set_once();

        Self {
            tweener: Rc::new(RefCell::new(tween::Tweener::new())),
        }
    }

    // Export main classes
    #[wasm_bindgen(js_name = Shape)]
    pub fn create_shape(&self, options: JsValue) -> Result<shape::Shape, JsValue> {
        shape::Shape::new(options)
    }

    #[wasm_bindgen(js_name = Burst)]
    pub fn create_burst(&self, options: JsValue) -> Result<burst::Burst, JsValue> {
        burst::Burst::new(options)
    }

    #[wasm_bindgen(js_name = Timeline)]
    pub fn create_timeline(&self) -> Result<tween::Timeline, JsValue> {
        Ok(tween::Timeline::new())
    }

    #[wasm_bindgen(js_name = MotionPath)]
    pub fn create_motion_path(&self, options: JsValue) -> Result<path::MotionPath, JsValue> {
        path::MotionPath::new(options)
    }

    #[wasm_bindgen(js_name = Html)]
    pub fn create_html(&self, options: JsValue) -> Result<render::Html, JsValue> {
        render::Html::new(options)
    }

    // Global tweener access
    #[wasm_bindgen(js_name = "tweener")]
    pub fn tweener(&self) -> Rc<RefCell<tween::Tweener>> {
        self.tweener.clone()
    }
}

#[wasm_bindgen(start)]
pub fn main() {
    // Initialize logger
    wasm_logger::init(wasm_logger::Config::default());
    log::info!("mojs-rs initialized");
}
```

### 2.2 Shared Types (types.rs)

```rust
use wasm_bindgen::prelude::*;
use serde::{Deserialize, Serialize};
use web_sys::{Element, HtmlElement, SvgElement};

/// Common callback types
pub type OnProgressCallback = js_sys::Function;
pub type OnUpdateCallback = js_sys::Function;
pub type OnStartCallback = js_sys::Function;
pub type OnCompleteCallback = js_sys::Function;

/// Playback state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlaybackState {
    Playing,
    Paused,
    Stopped,
}

/// Playback direction
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlaybackDirection {
    Forward,
    Backward,
}

/// Progress value (0.0 - 1.0)
pub type Progress = f64;

/// Time in milliseconds
pub type TimeMs = f64;

/// Common options shared across modules
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CommonOptions {
    /// Animation duration in ms
    #[serde(default = "default_duration")]
    pub duration: TimeMs,

    /// Delay before starting
    #[serde(default)]
    pub delay: TimeMs,

    /// Number of repeats
    #[serde(default)]
    pub repeat: u32,

    /// Delay between repeats
    #[serde(default)]
    pub repeat_delay: TimeMs,

    /// Enable yoyo playback
    #[serde(default)]
    pub is_yoyo: bool,

    /// Start from end
    #[serde(default)]
    pub is_reverse: bool,

    /// Playback speed multiplier
    #[serde(default = "default_speed")]
    pub speed: f64,

    /// Easing function name or definition
    #[serde(default = "default_easing")]
    pub easing: String,

    /// Backward easing (for yoyo)
    #[serde(default)]
    pub backward_easing: Option<String>,

    /// Callbacks context
    #[serde(skip)]
    pub callbacks_context: Option<JsValue>,
}

fn default_duration() -> TimeMs { 1000.0 }
fn default_speed() -> f64 { 1.0 }
fn default_easing() -> String { "sin.out".to_string() }

impl Default for CommonOptions {
    fn default() -> Self {
        Self {
            duration: default_duration(),
            delay: 0.0,
            repeat: 0,
            repeat_delay: 0.0,
            is_yoyo: false,
            is_reverse: false,
            speed: default_speed(),
            easing: default_easing(),
            backward_easing: None,
            callbacks_context: None,
        }
    }
}
```

---

## 3. Core Tween Engine

### 3.1 Tween Structure (tween/tween.rs)

```rust
use crate::types::*;
use crate::easing::{Easing, EasingRegistry};
use crate::delta::DeltaCallback;
use wasm_bindgen::prelude::*;
use web_sys::Window;
use std::cell::RefCell;
use std::rc::Rc;

/// Core tween engine - handles timing, period detection, and callbacks
pub struct Tween {
    /// Internal state
    props: TweenProps,

    /// Callbacks
    callbacks: TweenCallbacks,

    /// Cached state for period detection
    prev_time: Option<TimeMs>,
    prev_period: Option<i32>,
    prev_yoyo: bool,
    prev_eased_progress: Progress,

    /// State flags
    was_started: bool,
    was_completed: bool,
    was_reversed: bool,
    was_updated: bool,

    /// Progress tracking
    progress: Progress,
    eased_progress: Progress,
    progress_time: TimeMs,
    play_time: Option<TimeMs>,
    pause_time: Option<TimeMs>,
    time_shift: TimeMs,

    /// Current state
    state: PlaybackState,
    is_running: bool,
}

struct TweenProps {
    /// Timing
    duration: TimeMs,
    delay: TimeMs,
    repeat: u32,
    repeat_delay: TimeMs,

    /// Calculated times
    start_time: TimeMs,
    end_time: TimeMs,
    repeat_time: TimeMs,  // Total time including all repeats

    /// Direction
    is_yoyo: bool,
    is_reverse: bool,

    /// Speed
    speed: f64,

    /// Easing functions
    easing: Rc<Easing>,
    backward_easing: Option<Rc<Easing>>,

    /// Shifts for timeline integration
    shift_time: TimeMs,
    negative_shift: TimeMs,
}

struct TweenCallbacks {
    on_progress: Option<OnProgressCallback>,
    on_update: Option<OnUpdateCallback>,
    on_start: Option<OnStartCallback>,
    on_complete: Option<OnCompleteCallback>,
    on_repeat_start: Option<OnUpdateCallback>,
    on_repeat_complete: Option<OnUpdateCallback>,
    on_first_update: Option<OnUpdateCallback>,
    on_playback_start: Option<js_sys::Function>,
    on_playback_pause: Option<js_sys::Function>,
    on_playback_stop: Option<js_sys::Function>,
    on_playback_complete: Option<js_sys::Function>,
}

impl Tween {
    pub fn new(options: TweenOptions) -> Self {
        let props = TweenProps {
            duration: options.duration,
            delay: options.delay,
            repeat: options.repeat,
            repeat_delay: options.repeat_delay,
            start_time: 0.0,  // Calculated on play
            end_time: 0.0,
            repeat_time: (options.duration + options.delay) * (options.repeat + 1) as f64,
            is_yoyo: options.is_yoyo,
            is_reverse: options.is_reverse,
            speed: options.speed,
            easing: EasingRegistry::parse(&options.easing),
            backward_easing: options.backward_easing
                .map(|e| EasingRegistry::parse(&e)),
            shift_time: 0.0,
            negative_shift: 0.0,
        };

        Self {
            props,
            callbacks: TweenCallbacks {
                on_progress: options.on_progress,
                on_update: options.on_update,
                on_start: options.on_start,
                on_complete: options.on_complete,
                on_repeat_start: options.on_repeat_start,
                on_repeat_complete: options.on_repeat_complete,
                on_first_update: options.on_first_update,
                on_playback_start: None,
                on_playback_pause: None,
                on_playback_stop: None,
                on_playback_complete: None,
            },
            prev_time: None,
            prev_period: None,
            prev_yoyo: false,
            prev_eased_progress: 0.0,
            was_started: false,
            was_completed: false,
            was_reversed: false,
            was_updated: false,
            progress: 0.0,
            eased_progress: 0.0,
            progress_time: 0.0,
            play_time: None,
            pause_time: None,
            time_shift: 0.0,
            state: PlaybackState::Stopped,
            is_running: false,
        }
    }

    /// Start playback
    pub fn play(&mut self) {
        if self.state == PlaybackState::Playing {
            return;
        }

        self.state = PlaybackState::Playing;
        self.was_started = true;
        self.time_shift = 0.0;
        self.play_time = Some(self.current_time());

        // Add to global tweener
        crate::tween::Tweener::add(Rc::new(RefCell::new(self.clone())));

        self.callback_playback_start();
    }

    /// Pause playback
    pub fn pause(&mut self) {
        if self.state != PlaybackState::Playing {
            return;
        }

        self.state = PlaybackState::Paused;
        self.pause_time = Some(self.current_time());

        // Remove from tweener
        crate::tween::Tweener::remove(/* self reference */);

        self.callback_playback_pause();
    }

    /// Stop and reset
    pub fn stop(&mut self, progress: Option<Progress>) {
        self.state = PlaybackState::Stopped;
        self.was_started = false;

        crate::tween::Tweener::remove(/* self reference */);

        if let Some(p) = progress {
            self.set_progress(p, None);
        }

        self.callback_playback_stop();
    }

    /// Play backward (reverse)
    pub fn play_backward(&mut self) {
        if !self.was_reversed {
            self.was_reversed = true;
            self.props.is_reverse = !self.props.is_reverse;
        }

        // Set initial progress time for reverse
        self.progress_time = (self.props.end_time - self.props.start_time) - self.progress_time;

        self.state = PlaybackState::Playing;
        self.play_time = Some(self.current_time());
        crate::tween::Tweener::add(Rc::new(RefCell::new(self.clone())));
        self.callback_playback_start();
    }

    /// Main update method called by Tweener
    pub fn update(&mut self, time: TimeMs) -> bool {
        let p = &mut self.props;

        // Apply speed mapping
        if p.speed > 0.0 && self.play_time.is_some() {
            let play_time = self.play_time.unwrap();
            time = play_time + (p.speed * (time - play_time));
        }

        // Handle reverse playback
        if p.is_reverse {
            time = p.end_time - self.progress_time;
        }

        // Skip first frame without prev_time
        if self.prev_time.is_none() {
            self.prev_time = Some(time);
            return false;
        }

        // Get period and progress
        let period = self.get_period(time);
        let progress = self.get_progress(time);

        // Determine yoyo state
        let is_yoyo = p.is_yoyo && (period % 2 == 1);
        let is_forward = time >= self.prev_time.unwrap_or(time);

        // Set progress with easing
        self.set_progress_with_easing(progress, time, is_yoyo, is_forward);

        // Dispatch callbacks based on period changes
        self.dispatch_callbacks(period, is_yoyo, is_forward);

        // Cache values
        self.prev_time = Some(time);
        self.prev_period = Some(period);
        self.prev_yoyo = is_yoyo;

        // Return true if complete
        time >= p.end_time || time <= p.start_time
    }

    /// Detect current period (which repeat we're in)
    fn get_period(&self, time: TimeMs) -> i32 {
        let p = &self.props;
        let t_time = p.delay + p.duration;
        let d_time = p.delay + time - p.start_time;

        let t = d_time / t_time;
        let elapsed = if time < p.end_time {
            d_time % t_time
        } else {
            0.0
        };

        // Handle delay gaps
        if elapsed > 0.0 && elapsed < p.delay {
            return -1;  // In delay gap
        }

        if time > p.end_time {
            ((p.end_time - p.start_time + p.delay) / t_time).round() as i32
        } else {
            t.floor() as i32
        }
    }

    /// Calculate raw progress (0-1) within current period
    fn get_progress(&self, time: TimeMs) -> Progress {
        let p = &self.props;
        let t_time = p.duration + p.delay;
        let d_time = time - p.start_time;

        let mut t = d_time / t_time;

        // Clamp to valid range
        if time < p.start_time {
            return 0.0;
        }
        if time > p.end_time {
            return 1.0;
        }

        // Get progress within current period
        let mut progress = (d_time % t_time - p.delay) / p.duration;

        // Clamp progress
        progress = progress.max(0.0).min(1.0);

        progress
    }

    /// Apply easing based on direction and set progress
    fn set_progress_with_easing(
        &mut self,
        proc: Progress,
        time: TimeMs,
        is_yoyo: bool,
        is_forward: bool,
    ) {
        self.progress = proc;

        // Determine which easing to use based on direction
        self.eased_progress = if (is_forward && !is_yoyo) || (!is_forward && is_yoyo) {
            self.props.easing.apply(proc)
        } else {
            let easing = self.props.backward_easing
                .as_ref()
                .unwrap_or(&self.props.easing);
            easing.apply(proc)
        };

        // Call onUpdate if eased progress changed
        if (self.eased_progress - self.prev_eased_progress).abs() > f64::EPSILON {
            if let Some(ref cb) = self.callbacks.on_update {
                let _ = cb.call4(
                    &JsValue::NULL,
                    &self.eased_progress.into(),
                    &self.progress.into(),
                    &is_forward.into(),
                    &is_yoyo.into(),
                );
            }
        }

        self.prev_eased_progress = self.eased_progress;
    }

    /// Set progress directly (for initialization)
    fn set_progress(&mut self, proc: Progress, time: Option<TimeMs>) {
        self.progress = proc;
        self.eased_progress = self.props.easing.apply(proc);
    }

    /// Dispatch callbacks based on period changes
    fn dispatch_callbacks(&mut self, period: i32, is_yoyo: bool, is_forward: bool) {
        let prev_period = self.prev_period;

        // Period changed
        if Some(period) != prev_period {
            if period != -1 && prev_period != Some(-1) {
                // Previous period completed
                if let Some(ref cb) = self.callbacks.on_repeat_complete {
                    let _ = cb.call4(&JsValue::NULL, &is_forward.into(), &is_yoyo.into(), &self.eased_progress.into(), &self.progress.into());
                }

                // New period started
                if period != ((self.props.repeat + 1) as i32) {
                    if let Some(ref cb) = self.callbacks.on_repeat_start {
                        let _ = cb.call4(&JsValue::NULL, &is_forward.into(), &is_yoyo.into(), &self.eased_progress.into(), &self.progress.into());
                    }
                }
            }
        }

        // First period start
        if period == 0 && !self.was_started {
            if let Some(ref cb) = self.callbacks.on_start {
                let _ = cb.call2(&JsValue::NULL, &is_forward.into(), &is_yoyo.into());
            }
        }

        // Completion
        if !self.was_completed && self.progress >= 1.0 {
            self.was_completed = true;
            if let Some(ref cb) = self.callbacks.on_complete {
                let _ = cb.call2(&JsValue::NULL, &is_forward.into(), &is_yoyo.into());
            }
            self.callback_playback_complete();
        }
    }

    fn current_time(&self) -> TimeMs {
        web_time::performance_now() / 1000.0
    }

    // Callback helper methods...
    fn callback_playback_start(&self) { /* ... */ }
    fn callback_playback_pause(&self) { /* ... */ }
    fn callback_playback_stop(&self) { /* ... */ }
    fn callback_playback_complete(&self) { /* ... */ }
}

/// Options for creating a Tween
#[derive(Debug, Clone)]
pub struct TweenOptions {
    pub duration: TimeMs,
    pub delay: TimeMs,
    pub repeat: u32,
    pub repeat_delay: TimeMs,
    pub is_yoyo: bool,
    pub is_reverse: bool,
    pub speed: f64,
    pub easing: String,
    pub backward_easing: Option<String>,

    // Callbacks
    pub on_progress: Option<OnProgressCallback>,
    pub on_update: Option<OnUpdateCallback>,
    pub on_start: Option<OnStartCallback>,
    pub on_complete: Option<OnCompleteCallback>,
    pub on_repeat_start: Option<OnUpdateCallback>,
    pub on_repeat_complete: Option<OnUpdateCallback>,
    pub on_first_update: Option<OnUpdateCallback>,
}

impl Default for TweenOptions {
    fn default() -> Self {
        Self {
            duration: 1000.0,
            delay: 0.0,
            repeat: 0,
            repeat_delay: 0.0,
            is_yoyo: false,
            is_reverse: false,
            speed: 1.0,
            easing: "sin.out".to_string(),
            backward_easing: None,
            on_progress: None,
            on_update: None,
            on_start: None,
            on_complete: None,
            on_repeat_start: None,
            on_repeat_complete: None,
            on_first_update: None,
        }
    }
}
```

### 3.2 Tweener - Global RAF Loop (tween/tweener.rs)

```rust
use crate::types::*;
use crate::tween::Tween;
use wasm_bindgen::prelude::*;
use web_sys::{Window, Document};
use std::cell::RefCell;
use std::rc::Rc;
use std::collections::HashSet;

/// Global animation loop manager - singleton
pub struct Tweener {
    tweens: Vec<Rc<RefCell<Tween>>>,
    is_running: bool,
    raf_id: Option<i32>,
    saved_tweens: Vec<Rc<RefCell<Tween>>>,

    // Visibility handling
    visibility_hidden: Option<String>,
}

thread_local! {
    static TWEENER: Rc<RefCell<Tweener>> = Rc::new(RefCell::new(Tweener::new()));
}

impl Tweener {
    pub fn new() -> Self {
        let visibility_hidden = Self::get_visibility_hidden();

        // Set up visibility change listener
        if let Some(_) = visibility_hidden {
            // Would need to set up event listener via WASM
        }

        Self {
            tweens: Vec::new(),
            is_running: false,
            raf_id: None,
            saved_tweens: Vec::new(),
            visibility_hidden,
        }
    }

    /// Add a tween to the global loop
    pub fn add(tween: Rc<RefCell<Tween>>) {
        TWEENER.with(|t| {
            let mut t = t.borrow_mut();

            // Check if already running
            let is_already_added = t.tweens.iter().any(|t| {
                Rc::ptr_eq(t, &tween)
            });

            if is_already_added {
                return;
            }

            // Mark as running
            tween.borrow_mut().is_running = true;
            t.tweens.push(tween);
            t.start_loop();
        })
    }

    /// Remove a tween from the global loop
    pub fn remove(tween: Rc<RefCell<Tween>>) {
        TWEENER.with(|t| {
            let mut t = t.borrow_mut();

            tween.borrow_mut().is_running = false;
            t.tweens.retain(|t| !Rc::ptr_eq(t, &tween));
        })
    }

    /// Start the RAF loop
    fn start_loop(&mut self) {
        if self.is_running {
            return;
        }

        self.is_running = true;
        let closure = Closure::wrap(Box::new(move |time: f64| {
            Tweener::loop_tick(time);
        }) as Box<dyn FnMut(f64)>);

        let raf_id = web_sys::window()
            .unwrap()
            .request_animation_frame(closure.as_ref().unchecked_ref())
            .unwrap();

        self.raf_id = Some(raf_id);
        closure.forget();  // Prevent dropping
    }

    /// Main loop tick
    fn loop_tick(time: f64) {
        TWEENER.with(|t| {
            let mut t = t.borrow_mut();

            if !t.is_running {
                return;
            }

            // Update all tweens
            t.update(time);

            // Stop if no tweens
            if t.tweens.is_empty() {
                t.is_running = false;
                return;
            }

            // Schedule next frame
            let closure = Closure::wrap(Box::new(move |time: f64| {
                Tweener::loop_tick(time);
            }) as Box<dyn FnMut(f64)>);

            let raf_id = web_sys::window()
                .unwrap()
                .request_animation_frame(closure.as_ref().unchecked_ref())
                .unwrap();

            t.raf_id = Some(raf_id);
            closure.forget();
        })
    }

    /// Update all tweens with current time
    fn update(&mut self, time: f64) {
        // Reverse iteration for efficient removal
        let mut i = self.tweens.len();
        while i > 0 {
            i -= 1;

            let tween = &self.tweens[i];
            let is_complete = {
                let mut t = tween.borrow_mut();
                t.update(time)
            };

            if is_complete {
                self.tweens.remove(i);

                // Call on tweener finish
                let mut t = tween.borrow_mut();
                t.on_tweener_finish();
                t.prev_time = None;
            }
        }
    }

    /// Handle visibility change
    fn on_visibility_change(&mut self) {
        let is_hidden = self.visibility_hidden
            .as_ref()
            .and_then(|prop| {
                web_sys::window()
                    .and_then(|w| w.document())
                    .and_then(|d| d.get(prop.as_str()))
            })
            .map(|v| v.as_bool().unwrap_or(false))
            .unwrap_or(false);

        if is_hidden {
            // Save and pause playing tweens
            self.save_playing_tweens();
            for tween in &self.saved_tweens {
                tween.borrow_mut().pause();
            }
        } else {
            // Restore and resume
            for tween in &self.saved_tweens {
                tween.borrow_mut().play();
            }
            self.saved_tweens.clear();
        }
    }

    fn save_playing_tweens(&mut self) {
        self.saved_tweens = self.tweens
            .iter()
            .filter(|t| t.borrow().state == PlaybackState::Playing)
            .cloned()
            .collect();
    }

    fn get_visibility_hidden() -> Option<String> {
        let document = web_sys::window()?.document()?;

        if document.hidden().is_ok() {
            Some("hidden".to_string())
        } else if js_sys::Reflect::has(&document, &"mozHidden".into()).is_ok() {
            Some("mozHidden".to_string())
        } else if js_sys::Reflect::has(&document, &"webkitHidden".into()).is_ok() {
            Some("webkitHidden".to_string())
        } else {
            None
        }
    }
}
```

---

## 4. Delta System

### 4.1 Delta Types (delta/types.rs)

```rust
use crate::utils::{Color, Unit};
use serde::{Deserialize, Serialize};

/// Types of deltas that can be interpolated
#[derive(Debug, Clone)]
pub enum DeltaType {
    /// Color interpolation (RGBA)
    Color(ColorDelta),

    /// Plain number interpolation
    Number(NumberDelta),

    /// Unit interpolation (px, %, em, etc.)
    Unit(UnitDelta),

    /// Array interpolation (stroke-dasharray)
    Array(ArrayDelta),
}

/// Color delta for RGBA interpolation
#[derive(Debug, Clone)]
pub struct ColorDelta {
    pub name: String,
    pub start: Color,      // RGBA struct
    pub end: Color,
    pub delta: Color,      // Pre-calculated difference
    pub curve: Option<Rc<Easing>>,  // Optional elasticity curve
}

/// Number delta for plain numeric interpolation
#[derive(Debug, Clone)]
pub struct NumberDelta {
    pub name: String,
    pub start: f64,
    pub end: f64,
    pub delta: f64,  // end - start
    pub curve: Option<Rc<Easing>>,
}

/// Unit delta for CSS unit interpolation
#[derive(Debug, Clone)]
pub struct UnitDelta {
    pub name: String,
    pub start: Unit,   // { value, unit, is_strict }
    pub end: Unit,
    pub delta: f64,    // end.value - start.value
    pub curve: Option<Rc<Easing>>,
}

/// Array delta for stroke-dasharray etc.
#[derive(Debug, Clone)]
pub struct ArrayDelta {
    pub name: String,
    pub start: Vec<Unit>,
    pub end: Vec<Unit>,
    pub delta: Vec<Unit>,  // Pre-calculated deltas
    pub curve: Option<Rc<Easing>>,
}

/// Result of delta interpolation
#[derive(Debug, Clone)]
pub enum DeltaValue {
    Color(String),     // "rgba(255,0,0,0.5)"
    Number(f64),       // 50.0
    Unit(String),      // "100px"
    Array(String),     // "5,10,5"
}
```

### 4.2 Delta Calculator (delta/delta.rs)

```rust
use crate::delta::types::*;
use crate::easing::Easing;
use crate::types::{Progress, TimeMs};
use std::rc::Rc;

/// Single delta calculator - interpolates one property
pub struct Delta {
    delta_type: DeltaType,
    props: Rc<RefCell<AnimationProps>>,
    tween: Option<Rc<RefCell<Tween>>>,
}

impl Delta {
    pub fn new(delta_type: DeltaType, props: Rc<RefCell<AnimationProps>>) -> Self {
        Self {
            delta_type,
            props,
            tween: None,
        }
    }

    /// Calculate current value based on progress
    pub fn calc_current(&self, eased_progress: Progress, raw_progress: Progress) -> DeltaValue {
        match &self.delta_type {
            DeltaType::Color(delta) => {
                self.calc_color(delta, eased_progress, raw_progress)
            }
            DeltaType::Number(delta) => {
                self.calc_number(delta, eased_progress, raw_progress)
            }
            DeltaType::Unit(delta) => {
                self.calc_unit(delta, eased_progress, raw_progress)
            }
            DeltaType::Array(delta) => {
                self.calc_array(delta, eased_progress, raw_progress)
            }
        }
    }

    /// Color interpolation
    fn calc_color(
        &self,
        delta: &ColorDelta,
        eased_progress: Progress,
        raw_progress: Progress,
    ) -> DeltaValue {
        let (r, g, b, a) = if let Some(ref curve) = delta.curve {
            // Curve-based (elasticity)
            let cp = curve.apply(raw_progress);
            (
                ((delta.start.r + raw_progress * delta.delta.r) * cp).round() as u8,
                ((delta.start.g + raw_progress * delta.delta.g) * cp).round() as u8,
                ((delta.start.b + raw_progress * delta.delta.b) * cp).round() as u8,
                (delta.start.a + raw_progress * delta.delta.a) * cp,
            )
        } else {
            // Linear with easing
            (
                (delta.start.r + eased_progress * delta.delta.r).round() as u8,
                (delta.start.g + eased_progress * delta.delta.g).round() as u8,
                (delta.start.b + eased_progress * delta.delta.b).round() as u8,
                delta.start.a + eased_progress * delta.delta.a,
            )
        };

        DeltaValue::Color(format!("rgba({},{},{},{})", r, g, b, a))
    }

    /// Number interpolation
    fn calc_number(
        &self,
        delta: &NumberDelta,
        eased_progress: Progress,
        raw_progress: Progress,
    ) -> DeltaValue {
        let value = if let Some(ref curve) = delta.curve {
            curve.apply(raw_progress) * (delta.start + raw_progress * delta.delta)
        } else {
            delta.start + eased_progress * delta.delta
        };

        DeltaValue::Number(value)
    }

    /// Unit interpolation
    fn calc_unit(
        &self,
        delta: &UnitDelta,
        eased_progress: Progress,
        raw_progress: Progress,
    ) -> DeltaValue {
        let value = if let Some(ref curve) = delta.curve {
            curve.apply(raw_progress) * (delta.start.value + raw_progress * delta.delta)
        } else {
            delta.start.value + eased_progress * delta.delta
        };

        DeltaValue::Unit(format!("{}{}", value, delta.end.unit))
    }

    /// Array interpolation (stroke-dasharray)
    fn calc_array(
        &self,
        delta: &ArrayDelta,
        eased_progress: Progress,
        raw_progress: Progress,
    ) -> DeltaValue {
        // Optimization: calculate curve once for all elements
        let proc = delta.curve.as_ref().map(|c| c.apply(raw_progress));

        let mut result = String::new();
        for i in 0..delta.delta.len() {
            let dash = if let Some(cp) = proc {
                cp * (delta.start[i].value + raw_progress * delta.delta[i].value)
            } else {
                delta.start[i].value + eased_progress * delta.delta[i].value
            };

            result.push_str(&format!("{}{} ", dash, delta.delta[i].unit));
        }

        DeltaValue::Array(result.trim().to_string())
    }
}
```

### 4.3 Deltas Manager (delta/deltas.rs)

```rust
use crate::delta::delta::*;
use crate::delta::types::*;
use crate::utils::{Color, Unit, parse_color, parse_unit};
use crate::tween::{Tween, Timeline};
use crate::easing::EasingRegistry;
use wasm_bindgen::prelude::*;
use std::rc::Rc;
use std::cell::RefCell;

/// Multiple deltas manager - handles all property interpolation
pub struct Deltas {
    main_deltas: Vec<Delta>,
    child_deltas: Vec<Delta>,
    timeline: Rc<Timeline>,
    props: Rc<RefCell<AnimationProps>>,
}

/// Property name to delta type mapping hints
pub struct PropertyHints {
    /// Properties that are arrays
    pub array_properties: HashSet<String>,
    /// Properties that are unitless numbers
    pub number_properties: HashSet<String>,
}

impl Deltas {
    pub fn new(options: DeltasOptions) -> Self {
        let props = Rc::new(RefCell::new(AnimationProps::new()));

        // Parse all deltas from options
        let (main_deltas, child_deltas, tween_options) =
            Self::parse_deltas(&options, &props, &options.hints);

        // Create timeline
        let timeline = Rc::new(Timeline::new());

        // Add main delta tween
        if !main_deltas.is_empty() {
            let main_delta = Delta::new(main_deltas[0].clone());
            timeline.add_delta(main_delta);
        }

        Self {
            main_deltas,
            child_deltas,
            timeline,
            props,
        }
    }

    /// Parse options into delta objects
    fn parse_deltas(
        options: &DeltasOptions,
        props: &Rc<RefCell<AnimationProps>>,
        hints: &PropertyHints,
    ) -> (Vec<Delta>, Vec<Delta>, TweenOptions) {
        let mut main_deltas = Vec::new();
        let mut child_deltas = Vec::new();
        let mut tween_options = TweenOptions::default();

        // Separate tween options from delta properties
        let (delta_props, tween_opts) = Self::split_tween_options(&options.raw);

        tween_options.duration = tween_opts.duration;
        tween_options.delay = tween_opts.delay;
        tween_options.easing = tween_opts.easing;
        // ... other tween options

        // Parse each delta property
        for (key, value) in delta_props {
            if let Some(delta) = Self::parse_delta(&key, value, props, hints) {
                main_deltas.push(delta);
            }
        }

        (main_deltas, child_deltas, tween_options)
    }

    /// Parse a single delta property
    fn parse_delta(
        key: &str,
        value: JsValue,
        props: &Rc<RefCell<AnimationProps>>,
        hints: &PropertyHints,
    ) -> Option<Delta> {
        // Extract start/end from value object like {0: 100}
        let (start, end, curve) = Self::preparse_delta(&value)?;

        // Detect delta type based on value and hints
        let delta_type = if Self::is_color(&start, &end) {
            Self::parse_color_delta(key, start, end, curve)
        } else if hints.array_properties.contains(key) {
            Self::parse_array_delta(key, start, end, curve)
        } else if hints.number_properties.contains(key) {
            Self::parse_number_delta(key, start, end, curve)
        } else {
            Self::parse_unit_delta(key, start, end, curve)
        };

        Some(Delta::new(delta_type, props.clone()))
    }

    /// Pre-parse delta to extract start, end, and optional curve
    fn preparse_delta(value: &JsValue) -> Option<(String, String, Option<String>)> {
        let obj = value.dyn_ref::<js_sys::Object>()?;

        // Get first key as start
        let keys = js_sys::Object::keys(obj);
        let start_key = keys.get(0).as_string()?;
        let start = js_sys::Reflect::get(obj, &start_key.into())
            .ok()?
            .as_string()?;

        // Curve is a special property
        let curve = js_sys::Reflect::get(obj, &"curve".into())
            .ok()
            .and_then(|v| v.as_string());

        // End is the value at start_key
        let end = start;  // Simplified - actual parsing gets end value

        Some((start, end, curve))
    }

    /// Check if values represent a color
    fn is_color(start: &str, end: &str) -> bool {
        // Not a number and not rand/stagger
        start.parse::<f64>().is_err()
            && !start.starts_with("rand(")
            && !start.starts_with("stagger(")
    }

    fn parse_color_delta(name: &str, start: String, end: String, curve: Option<String>) -> DeltaType {
        let start_color = parse_color(&start).unwrap_or(Color::TRANSPARENT);
        let end_color = parse_color(&end).unwrap_or(Color::TRANSPARENT);

        let delta = Color {
            r: end_color.r as f64 - start_color.r as f64,
            g: end_color.g as f64 - start_color.g as f64,
            b: end_color.b as f64 - start_color.b as f64,
            a: end_color.a - start_color.a,
        };

        DeltaType::Color(ColorDelta {
            name: name.to_string(),
            start: start_color,
            end: end_color,
            delta,
            curve: curve.map(|c| EasingRegistry::parse(&c)),
        })
    }

    fn parse_number_delta(name: &str, start: String, end: String, curve: Option<String>) -> DeltaType {
        let start_val = start.parse::<f64>().unwrap_or(0.0);
        let end_val = end.parse::<f64>().unwrap_or(0.0);

        DeltaType::Number(NumberDelta {
            name: name.to_string(),
            start: start_val,
            end: end_val,
            delta: end_val - start_val,
            curve: curve.map(|c| EasingRegistry::parse(&c)),
        })
    }

    fn parse_unit_delta(name: &str, start: String, end: String, curve: Option<String>) -> DeltaType {
        let start_unit = parse_unit(&start);
        let end_unit = parse_unit(&end);

        // Merge units if needed
        let merged = Self::merge_units(start_unit, end_unit, name);

        DeltaType::Unit(UnitDelta {
            name: name.to_string(),
            start: merged.0,
            end: merged.1,
            delta: merged.1.value - merged.0.value,
            curve: curve.map(|c| EasingRegistry::parse(&c)),
        })
    }

    fn parse_array_delta(name: &str, start: String, end: String, curve: Option<String>) -> DeltaType {
        let start_arr = Self::string_to_array(&start);
        let end_arr = Self::string_to_array(&end);

        // Normalize array lengths
        let (start_arr, end_arr) = Self::normalize_arrays(start_arr, end_arr);

        // Calculate deltas
        let delta: Vec<Unit> = start_arr.iter()
            .zip(end_arr.iter())
            .map(|(s, e)| Unit {
                value: e.value - s.value,
                unit: e.unit.clone(),
                is_strict: e.is_strict,
            })
            .collect();

        DeltaType::Array(ArrayDelta {
            name: name.to_string(),
            start: start_arr,
            end: end_arr,
            delta,
            curve: curve.map(|c| EasingRegistry::parse(&c)),
        })
    }

    fn string_to_array(s: &str) -> Vec<Unit> {
        s.split_whitespace()
            .map(|part| parse_unit(part.trim()))
            .collect()
    }

    fn normalize_arrays(mut arr1: Vec<Unit>, mut arr2: Vec<Unit>) -> (Vec<Unit>, Vec<Unit>) {
        if arr1.len() > arr2.len() {
            let diff = arr1.len() - arr2.len();
            for i in 0..diff {
                arr2.push(Unit {
                    value: 0.0,
                    unit: arr1[arr2.len()].unit.clone(),
                    is_strict: false,
                });
            }
        } else if arr2.len() > arr1.len() {
            let diff = arr2.len() - arr1.len();
            for i in 0..diff {
                arr1.push(Unit {
                    value: 0.0,
                    unit: arr2[arr1.len()].unit.clone(),
                    is_strict: false,
                });
            }
        }
        (arr1, arr2)
    }

    fn merge_units(mut start: Unit, mut end: Unit, key: &str) -> (Unit, Unit) {
        if !end.is_strict && start.is_strict {
            end.unit = start.unit.clone();
        } else if end.is_strict && !start.is_strict {
            start.unit = end.unit.clone();
        } else if end.is_strict && start.is_strict && end.unit != start.unit {
            // Warn about unit mismatch
            log::warn!("Unit mismatch on {}: {} vs {}", key, start.unit, end.unit);
            start.unit = end.unit.clone();
        }
        (start, end)
    }

    /// Split tween options from delta properties
    fn split_tween_options(raw: &JsValue) -> (js_sys::Object, TweenOptionsLite) {
        let obj = raw.dyn_ref::<js_sys::Object>().unwrap();
        let mut delta_obj = js_sys::Object::new();
        let mut tween_opts = TweenOptionsLite::default();

        let tween_keys = ["duration", "delay", "repeat", "repeatDelay", "isYoyo",
                          "isReverse", "speed", "easing", "backwardEasing"];

        for key in js_sys::Object::keys(obj).iter() {
            let key_str = key.as_string().unwrap();
            let value = js_sys::Reflect::get(obj, &key).unwrap();

            if tween_keys.contains(&key_str.as_str()) {
                // Copy to tween options
                match key_str.as_str() {
                    "duration" => tween_opts.duration = value.as_f64().unwrap_or(1000.0),
                    "delay" => tween_opts.delay = value.as_f64().unwrap_or(0.0),
                    "easing" => tween_opts.easing = value.as_string().unwrap_or("sin.out".to_string()),
                    // ... other tween options
                    _ => {}
                }
            } else {
                // Keep in delta object
                js_sys::Reflect::set(&delta_obj, &key, &value).unwrap();
            }
        }

        (delta_obj, tween_opts)
    }
}

struct DeltasOptions {
    raw: JsValue,
    hints: PropertyHints,
}

struct TweenOptionsLite {
    duration: TimeMs,
    delay: TimeMs,
    easing: String,
    // ... other fields
}

impl Default for TweenOptionsLite {
    fn default() -> Self {
        Self {
            duration: 1000.0,
            delay: 0.0,
            easing: "sin.out".to_string(),
        }
    }
}
```

### 4.4 Color and Unit Utilities (utils/color.rs, utils/unit.rs)

```rust
// utils/color.rs
use palette::Srgba;

#[derive(Debug, Clone, Copy)]
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: f64,
}

impl Color {
    pub const TRANSPARENT: Color = Color { r: 0, g: 0, b: 0, a: 0.0 };
    pub const WHITE: Color = Color { r: 255, g: 255, b: 255, a: 1.0 };
    pub const BLACK: Color = Color { r: 0, g: 0, b: 0, a: 1.0 };

    /// Parse color from string (hex, rgb, rgba, named)
    pub fn parse(s: &str) -> Option<Self> {
        let s = s.trim().to_lowercase();

        // Named colors
        if let Some(color) = Self::parse_named(&s) {
            return Some(color);
        }

        // Hex colors
        if s.starts_with('#') {
            return Self::parse_hex(&s);
        }

        // RGB/RGBA
        if s.starts_with("rgb") {
            return Self::parse_rgb(&s);
        }

        None
    }

    fn parse_named(s: &str) -> Option<Color> {
        match s {
            "transparent" => Some(Self::TRANSPARENT),
            "white" => Some(Self::WHITE),
            "black" => Some(Self::BLACK),
            "red" => Some(Color { r: 255, g: 0, b: 0, a: 1.0 }),
            "blue" => Some(Color { r: 0, g: 0, b: 255, a: 1.0 }),
            "green" => Some(Color { r: 0, g: 128, b: 0, a: 1.0 }),
            "yellow" => Some(Color { r: 255, g: 255, b: 0, a: 1.0 }),
            // ... more named colors
            _ => None,
        }
    }

    fn parse_hex(s: &str) -> Option<Color> {
        let hex = s.trim_start_matches('#');

        match hex.len() {
            3 => {
                // #RGB -> #RRGGBB
                let r = u8::from_str_radix(&hex[0..1].repeat(2), 16).ok()?;
                let g = u8::from_str_radix(&hex[1..2].repeat(2), 16).ok()?;
                let b = u8::from_str_radix(&hex[2..3].repeat(2), 16).ok()?;
                Some(Color { r, g, b, a: 1.0 })
            }
            4 => {
                // #RGBA -> #RRGGBBAA
                let r = u8::from_str_radix(&hex[0..1].repeat(2), 16).ok()?;
                let g = u8::from_str_radix(&hex[1..2].repeat(2), 16).ok()?;
                let b = u8::from_str_radix(&hex[2..3].repeat(2), 16).ok()?;
                let a = u8::from_str_radix(&hex[3..4].repeat(2), 16).ok()?;
                Some(Color { r, g, b, a: a as f64 / 255.0 })
            }
            6 => {
                // #RRGGBB
                let r = u8::from_str_radix(&hex[0..2], 16).ok()?;
                let g = u8::from_str_radix(&hex[2..4], 16).ok()?;
                let b = u8::from_str_radix(&hex[4..6], 16).ok()?;
                Some(Color { r, g, b, a: 1.0 })
            }
            8 => {
                // #RRGGBBAA
                let r = u8::from_str_radix(&hex[0..2], 16).ok()?;
                let g = u8::from_str_radix(&hex[2..4], 16).ok()?;
                let b = u8::from_str_radix(&hex[4..6], 16).ok()?;
                let a = u8::from_str_radix(&hex[6..8], 16).ok()?;
                Some(Color { r, g, b, a: a as f64 / 255.0 })
            }
            _ => None,
        }
    }

    fn parse_rgb(s: &str) -> Option<Color> {
        // rgba(255, 0, 0, 0.5) or rgb(255, 0, 0)
        let nums: Vec<&str> = s
            .trim_start_matches(|c: char| c != '(')
            .trim_start_matches('(')
            .trim_end_matches(')')
            .split(',')
            .collect();

        if nums.len() < 3 || nums.len() > 4 {
            return None;
        }

        let r = nums[0].trim().parse::<u8>().ok()?;
        let g = nums[1].trim().parse::<u8>().ok()?;
        let b = nums[2].trim().parse::<u8>().ok()?;
        let a = nums.get(3)
            .and_then(|s| s.trim().parse::<f64>().ok())
            .unwrap_or(1.0);

        Some(Color { r, g, b, a })
    }
}

// utils/unit.rs
#[derive(Debug, Clone)]
pub struct Unit {
    pub value: f64,
    pub unit: String,
    pub is_strict: bool,  // Whether unit was explicitly specified
}

impl Unit {
    pub fn new(value: f64, unit: &str, is_strict: bool) -> Self {
        Self {
            value,
            unit: unit.to_string(),
            is_strict,
        }
    }

    /// Parse unit from string
    pub fn parse(s: &str) -> Self {
        let s = s.trim();

        // Try to parse as plain number
        if let Ok(value) = s.parse::<f64>() {
            return Self {
                value,
                unit: "px".to_string(),
                is_strict: false,
            };
        }

        // Extract unit
        let unit_regex = regex::Regex::new(r"(px|%,|rem|em|ex|cm|ch|mm|in|pt|pc|vh|vw|vmin|deg|rad)").unwrap();
        let unit = unit_regex.find(s)
            .map(|m| m.as_str().to_string())
            .unwrap_or_else(|| "px".to_string());

        let value = s.parse::<f64>().unwrap_or(0.0);

        Self {
            value,
            unit,
            is_strict: true,
        }
    }
}

pub fn parse_unit(s: &str) -> Unit {
    Unit::parse(s)
}

pub fn parse_color(s: &str) -> Option<Color> {
    Color::parse(s)
}
```

---

## 5. Easing System

### 5.1 Easing Registry (easing/registry.rs)

```rust
use crate::easing::{bezier, path, approximate};
use std::collections::HashMap;
use std::sync::RwLock;

/// Global easing registry and parser
pub struct EasingRegistry {
    presets: HashMap<String, Easing>,
}

thread_local! {
    static REGISTRY: RwLock<EasingRegistry> = RwLock::new(EasingRegistry::new());
}

impl EasingRegistry {
    fn new() -> Self {
        let mut registry = Self {
            presets: HashMap::new(),
        };

        // Register all preset easings
        registry.register_presets();

        registry
    }

    /// Register all built-in easing presets
    fn register_presets(&mut self) {
        // Linear
        self.presets.insert("linear.none".to_string(), Easing::Linear);

        // Ease (CSS-like)
        self.presets.insert("ease.in".to_string(), Easing::Bezier(0.42, 0.0, 1.0, 1.0));
        self.presets.insert("ease.out".to_string(), Easing::Bezier(0.0, 0.0, 0.58, 1.0));
        self.presets.insert("ease.inout".to_string(), Easing::Bezier(0.42, 0.0, 0.58, 1.0));

        // Sin
        self.presets.insert("sin.in".to_string(), Easing::SinIn);
        self.presets.insert("sin.out".to_string(), Easing::SinOut);
        self.presets.insert("sin.inout".to_string(), Easing::SinInOut);

        // Quad
        self.presets.insert("quad.in".to_string(), Easing::QuadIn);
        self.presets.insert("quad.out".to_string(), Easing::QuadOut);
        self.presets.insert("quad.inout".to_string(), Easing::QuadInOut);

        // Cubic
        self.presets.insert("cubic.in".to_string(), Easing::CubicIn);
        self.presets.insert("cubic.out".to_string(), Easing::CubicOut);
        self.presets.insert("cubic.inout".to_string(), Easing::CubicInOut);

        // Quart
        self.presets.insert("quart.in".to_string(), Easing::QuartIn);
        self.presets.insert("quart.out".to_string(), Easing::QuartOut);
        self.presets.insert("quart.inout".to_string(), Easing::QuartInOut);

        // Quint
        self.presets.insert("quint.in".to_string(), Easing::QuintIn);
        self.presets.insert("quint.out".to_string(), Easing::QuintOut);
        self.presets.insert("quint.inout".to_string(), Easing::QuintInOut);

        // Expo
        self.presets.insert("expo.in".to_string(), Easing::ExpoIn);
        self.presets.insert("expo.out".to_string(), Easing::ExpoOut);
        self.presets.insert("expo.inout".to_string(), Easing::ExpoInOut);

        // Circ
        self.presets.insert("circ.in".to_string(), Easing::CircIn);
        self.presets.insert("circ.out".to_string(), Easing::CircOut);
        self.presets.insert("circ.inout".to_string(), Easing::CircInOut);

        // Back
        self.presets.insert("back.in".to_string(), Easing::BackIn);
        self.presets.insert("back.out".to_string(), Easing::BackOut);
        self.presets.insert("back.inout".to_string(), Easing::BackInOut);

        // Elastic
        self.presets.insert("elastic.in".to_string(), Easing::ElasticIn);
        self.presets.insert("elastic.out".to_string(), Easing::ElasticOut);
        self.presets.insert("elastic.inout".to_string(), Easing::ElasticInOut);

        // Bounce
        self.presets.insert("bounce.in".to_string(), Easing::BounceIn);
        self.presets.insert("bounce.out".to_string(), Easing::BounceOut);
        self.presets.insert("bounce.inout".to_string(), Easing::BounceInOut);
    }

    /// Parse easing from string or array
    pub fn parse(input: &str) -> Rc<Easing> {
        let input = input.trim();

        // Check for path easing (starts with 'm')
        if input.starts_with('m') || input.starts_with('M') {
            return Rc::new(Easing::Path(path::PathEasing::new(input)));
        }

        // Check for bezier array [x1, y1, x2, y2]
        // This would need JS interop for array parsing

        // Parse family.variant format
        let parts: Vec<&str> = input.split('.').collect();
        let (family, variant) = match parts.as_slice() {
            [family, variant] => (family.to_lowercase(), variant.to_lowercase()),
            [family] => (family.to_lowercase(), "none".to_string()),
            _ => ("linear".to_string(), "none".to_string()),
        };

        let key = format!("{}.{}", family, variant);

        REGISTRY.with(|registry| {
            let registry = registry.read().unwrap();

            if let Some(easing) = registry.presets.get(&key) {
                Rc::new(easing.clone())
            } else {
                log::warn!("Unknown easing: {}, falling back to linear.none", key);
                Rc::new(Easing::Linear)
            }
        })
    }
}

/// Easing function types
#[derive(Debug, Clone)]
pub enum Easing {
    /// Linear: f(t) = t
    Linear,

    /// Cubic Bezier curve
    Bezier(f64, f64, f64, f64),  // x1, y1, x2, y2

    /// Pre-defined easing functions
    SinIn, SinOut, SinInOut,
    QuadIn, QuadOut, QuadInOut,
    CubicIn, CubicOut, CubicInOut,
    QuartIn, QuartOut, QuartInOut,
    QuintIn, QuintOut, QuintInOut,
    ExpoIn, ExpoOut, ExpoInOut,
    CircIn, CircOut, CircInOut,
    BackIn, BackOut, BackInOut,
    ElasticIn, ElasticOut, ElasticInOut,
    BounceIn, BounceOut, BounceInOut,

    /// SVG path-based easing
    Path(path::PathEasing),

    /// Approximated/sampled function
    Sampled(Vec<f64>),

    /// Custom JS function
    Custom(js_sys::Function),
}

impl Easing {
    /// Apply easing to progress value
    pub fn apply(&self, progress: f64) -> f64 {
        match self {
            Easing::Linear => progress,

            Easing::Bezier(x1, y1, x2, y2) => {
                bezier::cubic_bezier(progress, *x1, *y1, *x2, *y2)
            }

            // Preset implementations
            Easing::SinIn => {
                const PI_2: f64 = std::f64::consts::PI / 2.0;
                1.0 - (progress * PI_2).cos()
            }
            Easing::SinOut => {
                const PI_2: f64 = std::f64::consts::PI / 2.0;
                (progress * PI_2).sin()
            }
            Easing::SinInOut => {
                const PI: f64 = std::f64::consts::PI;
                0.5 * (1.0 - (progress * PI).cos())
            }

            Easing::QuadIn => progress * progress,
            Easing::QuadOut => progress * (2.0 - progress),
            Easing::QuadInOut => {
                if progress < 0.5 {
                    2.0 * progress * progress
                } else {
                    -1.0 + (4.0 - 2.0 * progress) * progress
                }
            }

            Easing::CubicIn => progress * progress * progress,
            Easing::CubicOut => {
                let p = progress - 1.0;
                p * p * p + 1.0
            }
            Easing::CubicInOut => {
                if progress < 0.5 {
                    4.0 * progress * progress * progress
                } else {
                    let p = 2.0 * progress - 2.0;
                    0.5 * p * p * p + 1.0
                }
            }

            // ... other presets

            Easing::Path(path_easing) => path_easing.sample(progress),

            Easing::Sampled(samples) => {
                approximate::sampled_lookup(samples, progress)
            }

            Easing::Custom(func) => {
                func.call1(&JsValue::NULL, &progress.into())
                    .and_then(|v| v.as_f64())
                    .unwrap_or(progress)
            }
        }
    }
}
```

### 5.2 Bezier Easing (easing/bezier.rs)

```rust
use std::f64::consts;

/// Cubic Bezier easing implementation
/// Uses Newton-Raphson iteration with sample table optimization

const NEWTON_ITERATIONS: u32 = 4;
const NEWTON_MIN_SLOPE: f64 = 0.001;
const SUBDIVISION_PRECISION: f64 = 0.0000001;
const SUBDIVISION_MAX_ITERATIONS: u32 = 10;

const SAMPLE_TABLE_SIZE: usize = 11;
const SAMPLE_STEP_SIZE: f64 = 1.0 / (SAMPLE_TABLE_SIZE - 1) as f64;

/// Calculate cubic Bezier value
pub fn cubic_bezier(t: f64, x1: f64, y1: x2: f64, y2: f64) -> f64 {
    // Validate control points
    if !(0.0..=1.0).contains(&x1) || !(0.0..=1.0).contains(&x2) {
        log::warn!("Bezier x values must be in [0, 1]");
        return t;  // Fall back to linear
    }

    // Linear shortcut
    if x1 == y1 && x2 == y2 {
        return t;
    }

    // Precompute coefficients
    let cx = 3.0 * x1;
    let bx = 3.0 * (x2 - x1) - cx;
    let ax = 1.0 - cx - bx;

    let cy = 3.0 * y1;
    let by = 3.0 * (y2 - y1) - cy;
    let ay = 1.0 - cy - by;

    // Sample values for binary search optimization
    let mut sample_values = [0.0; SAMPLE_TABLE_SIZE];
    for i in 0..SAMPLE_TABLE_SIZE {
        let t = i as f64 * SAMPLE_STEP_SIZE;
        sample_values[i] = calc_bezier(t, ax, bx, cx);
    }

    // Get t for given x
    let t = if t == 0.0 || t == 1.0 {
        t
    } else {
        get_t_for_x(t, &sample_values, ax, bx, cx)
    };

    // Calculate y value
    calc_bezier(t, ay, by, cy)
}

/// Calculate Bezier value given t and coefficients
fn calc_bezier(t: f64, a: f64, b: f64, c: f64) -> f64 {
    ((a * t + b) * t + c) * t
}

/// Calculate slope (derivative) at t
fn get_slope(t: f64, a: f64, b: f64, c: f64) -> f64 {
    3.0 * a * t * t + 2.0 * b * t + c
}

/// Find t for given x using sample table + Newton-Raphson
fn get_t_for_x(
    x: f64,
    sample_values: &[f64; SAMPLE_TABLE_SIZE],
    ax: f64,
    bx: f64,
    cx: f64,
) -> f64 {
    // Binary search to find interval
    let mut interval_start = 0.0;
    let mut current_sample = 1;

    while current_sample < SAMPLE_TABLE_SIZE - 1
        && sample_values[current_sample] <= x
    {
        interval_start += SAMPLE_STEP_SIZE;
        current_sample += 1;
    }
    current_sample -= 1;

    // Linear interpolation for initial guess
    let delta = sample_values[current_sample + 1] - sample_values[current_sample];
    let dist = (x - sample_values[current_sample]) / delta;
    let guess = interval_start + dist * SAMPLE_STEP_SIZE;

    // Newton-Raphson refinement
    let slope = get_slope(guess, ax, bx, cx);

    if slope >= NEWTON_MIN_SLOPE {
        newton_raphson_iterate(x, guess, ax, bx, cx)
    } else if slope == 0.0 {
        guess
    } else {
        binary_subdivide(x, interval_start, interval_start + SAMPLE_STEP_SIZE, ax, bx, cx)
    }
}

/// Newton-Raphson iteration
fn newton_raphson_iterate(x: f64, mut t: f64, ax: f64, bx: f64, cx: f64) -> f64 {
    for _ in 0..NEWTON_ITERATIONS {
        let slope = get_slope(t, ax, bx, cx);
        if slope == 0.0 {
            return t;
        }
        let current_x = calc_bezier(t, ax, bx, cx) - x;
        t -= current_x / slope;
    }
    t
}

/// Binary subdivision fallback
fn binary_subdivide(
    x: f64,
    mut a: f64,
    mut b: f64,
    ax: f64,
    bx: f64,
    cx: f64,
) -> f64 {
    let mut current_x = 0.0;
    let mut current_t = 0.0;

    for _ in 0..SUBDIVISION_MAX_ITERATIONS {
        current_t = a + (b - a) / 2.0;
        current_x = calc_bezier(current_t, ax, bx, cx) - x;

        if current_x > 0.0 {
            b = current_t;
        } else {
            a = current_t;
        }

        if current_x.abs() <= SUBDIVISION_PRECISION {
            break;
        }
    }

    current_t
}
```

---

## 6. Timeline Composition

### 6.1 Timeline Structure (tween/timeline.rs)

```rust
use crate::tween::{Tween, TweenOptions};
use crate::delta::Delta;
use crate::types::*;
use std::rc::Rc;
use std::cell::RefCell;

/// Timeline for composing multiple tweens
pub struct Timeline {
    /// Child tweens/timelines
    children: Vec<Rc<RefCell<TimelineChild>>>,

    /// Timeline properties
    props: TimelineProps,

    /// State
    state: PlaybackState,
    is_running: bool,
}

enum TimelineChild {
    Tween(Rc<RefCell<Tween>>),
    Timeline(Rc<RefCell<Timeline>>),
    Delta(Delta),
}

struct TimelineProps {
    duration: TimeMs,
    delay: TimeMs,
    repeat: u32,
    is_yoyo: bool,
    is_reverse: bool,
    speed: f64,

    /// Calculated times
    start_time: TimeMs,
    end_time: TimeMs,

    /// Shifts
    shift_time: TimeMs,
    negative_shift: TimeMs,
}

impl Timeline {
    pub fn new() -> Self {
        Self {
            children: Vec::new(),
            props: TimelineProps {
                duration: 0.0,
                delay: 0.0,
                repeat: 0,
                is_yoyo: false,
                is_reverse: false,
                speed: 1.0,
                start_time: 0.0,
                end_time: 0.0,
                shift_time: 0.0,
                negative_shift: 0.0,
            },
            state: PlaybackState::Stopped,
            is_running: false,
        }
    }

    /// Add tweens to play in parallel
    pub fn add(&mut self, tweens: Vec<Rc<RefCell<Tween>>>) -> &mut Self {
        for tween in tweens {
            self.children.push(Rc::new(RefCell::new(TimelineChild::Tween(tween))));
        }
        self.recalc_dimensions();
        self
    }

    /// Append tweens sequentially
    pub fn append(&mut self, children: Vec<Rc<RefCell<TimelineChild>>>) -> &mut Self {
        for child in children {
            // Set shift time for sequential playback
            if let TimelineChild::Tween(ref tween) = *child.borrow() {
                let shift = self.props.duration;
                tween.borrow_mut().set_shift_time(shift);
            }

            self.children.push(child);
        }
        self.recalc_dimensions();
        self
    }

    /// Append array of tweens (play in parallel at end)
    pub fn append_array(&mut self, tweens: Vec<Rc<RefCell<Tween>>>) -> &mut Self {
        let shift = self.props.duration;

        for tween in tweens {
            tween.borrow_mut().set_shift_time(shift);
            self.children.push(Rc::new(RefCell::new(TimelineChild::Tween(tween))));
        }

        self.recalc_dimensions();
        self
    }

    /// Add delta animation
    pub fn add_delta(&mut self, delta: Delta) {
        self.children.push(Rc::new(RefCell::new(TimelineChild::Delta(delta))));
        self.recalc_dimensions();
    }

    /// Calculate timeline duration based on children
    fn recalc_dimensions(&mut self) {
        let mut max_duration = 0.0;

        for child in &self.children {
            let child_duration = match &*child.borrow() {
                TimelineChild::Tween(t) => t.borrow().get_total_time(),
                TimelineChild::Timeline(t) => t.borrow().props.duration,
                TimelineChild::Delta(_) => 0.0,
            };

            max_duration = max_duration.max(child_duration);
        }

        self.props.duration = max_duration;
        self.calculate_times();
    }

    fn calculate_times(&mut self) {
        let p = &mut self.props;
        p.repeat_time = (p.duration + p.delay) * (p.repeat + 1) as f64;
        p.end_time = p.start_time + p.repeat_time - p.delay;
    }

    /// Play timeline
    pub fn play(&mut self) {
        if self.state == PlaybackState::Playing {
            return;
        }

        self.state = PlaybackState::Playing;
        self.props.start_time = self.current_time();
        self.calculate_times();

        // Play all children
        for child in &self.children {
            match &*child.borrow() {
                TimelineChild::Tween(t) => t.borrow_mut().play(),
                TimelineChild::Timeline(t) => t.borrow_mut().play(),
                TimelineChild::Delta(_) => {}
            }
        }

        // Add to tweener
        crate::tween::Tweener::add_timeline(Rc::new(RefCell::new(self.clone())));
    }

    /// Update timeline and children
    pub fn update(&mut self, time: TimeMs) -> bool {
        let p = &self.props;

        // Calculate timeline progress
        let timeline_progress = if time < p.start_time {
            0.0
        } else if time > p.end_time {
            1.0
        } else {
            (time - p.start_time) / (p.end_time - p.start_time)
        };

        // Update children based on timeline progress
        self.update_children(timeline_progress, time);

        time >= p.end_time
    }

    /// Update children with proper direction handling
    fn update_children(&mut self, progress: Progress, time: TimeMs) {
        let is_forward = time > self.prev_time.unwrap_or(time);

        // Determine iteration direction
        let iter: Box<dyn Iterator<Item = usize>> = if is_forward {
            Box::new(0..self.children.len())
        } else {
            Box::new((0..self.children.len()).rev())
        };

        let time_to_children = self.props.start_time + progress * self.props.duration;

        for i in iter {
            if let Some(child) = self.children.get(i) {
                match &*child.borrow() {
                    TimelineChild::Tween(t) => {
                        let mut t = t.borrow_mut();
                        t.update(time_to_children);
                    }
                    TimelineChild::Timeline(t) => {
                        let mut t = t.borrow_mut();
                        t.update(time_to_children);
                    }
                    TimelineChild::Delta(_) => {}
                }
            }
        }
    }

    fn current_time(&self) -> TimeMs {
        web_time::performance_now() / 1000.0
    }
}
```

---

## 7. Rendering System

### 7.1 SVG Shape Rendering (render/svg.rs)

```rust
use wasm_bindgen::prelude::*;
use web_sys::{SvgElement, Element, Document};
use crate::types::*;
use crate::render::cache::AttributeCache;

/// Base SVG shape renderer
pub struct SvgShape {
    /// SVG element
    svg: SvgElement,

    /// Shape element (circle, rect, etc.)
    shape_el: Element,

    /// Parent container
    parent: Element,

    /// Attribute cache for performance
    cache: AttributeCache,

    /// Shape properties
    props: ShapeProps,
}

struct ShapeProps {
    /// Shape type
    shape_type: ShapeType,

    /// Size
    radius: f64,
    radius_x: Option<f64>,
    radius_y: Option<f64>,

    /// Stroke
    stroke: String,
    stroke_width: f64,
    stroke_opacity: f64,
    stroke_linecap: String,
    stroke_dasharray: String,
    stroke_dashoffset: f64,

    /// Fill
    fill: String,
    fill_opacity: f64,

    /// Canvas
    width: f64,
    height: f64,
}

#[derive(Debug, Clone)]
pub enum ShapeType {
    Circle,
    Rect,
    Line,
    Cross,
    Polygon,
    Equal,
    Zigzag,
    Custom(String),
}

impl SvgShape {
    pub fn new(parent: Element, shape_type: ShapeType) -> Result<Self, JsValue> {
        let document = parent.owner_document()
            .ok_or("No owner document")?;

        // Create SVG canvas
        let svg: SvgElement = document
            .create_element_ns(Some("http://www.w3.org/2000/svg"), "svg")?
            .dyn_into()?;

        // Create shape element
        let tag = Self::shape_tag(&shape_type);
        let shape_el = document
            .create_element_ns(Some("http://www.w3.org/2000/svg"), &tag)?;

        svg.append_child(&shape_el)?;
        parent.append_child(&svg)?;

        // Set initial styles
        let style = svg.unchecked_ref::<Element>().style();
        style.set_property("display", "block")?;
        style.set_property("width", "100%")?;
        style.set_property("height", "100%")?;

        Ok(Self {
            svg,
            shape_el,
            parent,
            cache: AttributeCache::new(),
            props: ShapeProps {
                shape_type,
                radius: 50.0,
                radius_x: None,
                radius_y: None,
                stroke: "transparent".to_string(),
                stroke_width: 2.0,
                stroke_opacity: 1.0,
                stroke_linecap: String::new(),
                stroke_dasharray: String::new(),
                stroke_dashoffset: 0.0,
                fill: "deeppink".to_string(),
                fill_opacity: 1.0,
                width: 0.0,
                height: 0.0,
            },
        })
    }

    fn shape_tag(shape_type: &ShapeType) -> &'static str {
        match shape_type {
            ShapeType::Circle => "ellipse",
            ShapeType::Rect => "rect",
            ShapeType::Line => "line",
            ShapeType::Cross => "g",  // Group with two lines
            ShapeType::Polygon => "polygon",
            ShapeType::Equal => "g",  // Group with two rects
            ShapeType::Zigzag => "polyline",
            ShapeType::Custom(tag) => tag,
        }
    }

    /// Draw current properties to DOM
    pub fn draw(&mut self) {
        let p = &self.props;

        // Draw based on shape type
        match &p.shape_type {
            ShapeType::Circle => self.draw_circle(),
            ShapeType::Rect => self.draw_rect(),
            ShapeType::Line => self.draw_line(),
            ShapeType::Polygon => self.draw_polygon(),
            _ => {}
        }

        // Set common attributes
        self.set_attr_if_changed("stroke", &p.stroke);
        self.set_attr_if_changed("stroke-width", &p.stroke_width.to_string());
        self.set_attr_if_changed("stroke-opacity", &p.stroke_opacity.to_string());
        self.set_attr_if_changed("stroke-dasharray", &p.stroke_dasharray);
        self.set_attr_if_changed("stroke-dashoffset", &p.stroke_dashoffset.to_string());
        self.set_attr_if_changed("fill", &p.fill);
        self.set_attr_if_changed("fill-opacity", &p.fill_opacity.to_string());
    }

    fn draw_circle(&mut self) {
        let p = &self.props;
        let rx = p.radius_x.unwrap_or(p.radius);
        let ry = p.radius_y.unwrap_or(p.radius);

        self.set_attr_if_changed("rx", &rx.to_string());
        self.set_attr_if_changed("ry", &ry.to_string());
        self.set_attr_if_changed("cx", &(p.width / 2.0).to_string());
        self.set_attr_if_changed("cy", &(p.height / 2.0).to_string());
    }

    fn draw_rect(&mut self) {
        let p = &self.props;
        let rx = p.radius_x.unwrap_or(p.radius);
        let ry = p.radius_y.unwrap_or(p.radius);

        self.set_attr_if_changed("x", &(-rx).to_string());
        self.set_attr_if_changed("y", &(-ry).to_string());
        self.set_attr_if_changed("width", &(rx * 2.0).to_string());
        self.set_attr_if_changed("height", &(ry * 2.0).to_string());
    }

    fn draw_line(&mut self) {
        let r = self.props.radius;

        self.set_attr_if_changed("x1", &(-r).to_string());
        self.set_attr_if_changed("y1", "0");
        self.set_attr_if_changed("x2", &r.to_string());
        self.set_attr_if_changed("y2", "0");
    }

    fn draw_polygon(&mut self) {
        let p = &self.props;
        let points = self.get_polygon_points(p.radius, 3);  // 3 = default points

        self.set_attr_if_changed("points", &points);
    }

    fn get_polygon_points(&self, radius: f64, points: u32) -> String {
        let mut result = String::new();
        let angle_step = (2.0 * std::f64::consts::PI) / points as f64;

        for i in 0..points {
            let angle = i as f64 * angle_step - std::f64::consts::PI / 2.0;
            let x = angle.cos() * radius;
            let y = angle.sin() * radius;

            if i > 0 {
                result.push(' ');
            }
            result.push_str(&format!("{},{}", x, y));
        }

        result
    }

    /// Set SVG attribute only if changed (performance optimization)
    fn set_attr_if_changed(&mut self, name: &str, value: &str) {
        if self.cache.get(name) != Some(value.to_string()) {
            let _ = self.shape_el.set_attribute(name, value);
            self.cache.set(name, value.to_string());
        }
    }
}
```

### 7.2 Attribute Cache (render/cache.rs)

```rust
use std::collections::HashMap;

/// Attribute cache to prevent redundant DOM updates
pub struct AttributeCache {
    cache: HashMap<String, String>,
}

impl AttributeCache {
    pub fn new() -> Self {
        Self {
            cache: HashMap::new(),
        }
    }

    pub fn get(&self, name: &str) -> Option<String> {
        self.cache.get(name).cloned()
    }

    pub fn set(&mut self, name: &str, value: String) {
        self.cache.insert(name.to_string(), value);
    }

    pub fn clear(&mut self) {
        self.cache.clear();
    }
}

/// Style cache for HTML elements
pub struct StyleCache {
    cache: HashMap<String, String>,
    vendor_prefix: Option<String>,
}

impl StyleCache {
    pub fn new() -> Self {
        let vendor_prefix = Self::detect_vendor_prefix();

        Self {
            cache: HashMap::new(),
            vendor_prefix,
        }
    }

    pub fn set_if_changed(&mut self, style: &web_sys::CssStyleDeclaration, name: &str, value: &str) -> Result<(), JsValue> {
        if self.cache.get(name) == Some(&value.to_string()) {
            return Ok(());  // Already set
        }

        style.set_property(name, value)?;

        // Apply vendor prefix if needed
        if let Some(ref prefix) = self.vendor_prefix {
            if Self::needs_prefix(name) {
                style.set_property(&format!("{}{}", prefix, name), value)?;
            }
        }

        self.cache.insert(name.to_string(), value.to_string());
        Ok(())
    }

    fn needs_prefix(name: &str) -> bool {
        matches!(name, "transform" | "transform-origin")
    }

    fn detect_vendor_prefix() -> Option<String> {
        // Could detect browser and return appropriate prefix
        // Modern browsers mostly don't need prefixes
        None
    }
}
```

### 7.3 HTML Element Animation (render/html.rs)

```rust
use wasm_bindgen::prelude::*;
use web_sys::{HtmlElement, CssStyleDeclaration};
use crate::types::*;
use crate::render::cache::StyleCache;

/// HTML element animator
pub struct Html {
    /// Target element
    el: HtmlElement,

    /// Style cache
    cache: StyleCache,

    /// Properties
    props: HtmlProps,
}

struct HtmlProps {
    // Transforms
    x: String,
    y: String,
    z: String,
    rotate_x: f64,
    rotate_y: f64,
    rotate_z: f64,
    skew_x: f64,
    skew_y: f64,
    scale: f64,
    scale_x: f64,
    scale_y: f64,

    // Behavior
    is_force_3d: bool,
    is_soft_hide: bool,

    // Other
    opacity: f64,
    transform_origin: String,
}

impl Default for HtmlProps {
    fn default() -> Self {
        Self {
            x: "0".to_string(),
            y: "0".to_string(),
            z: "0".to_string(),
            rotate_x: 0.0,
            rotate_y: 0.0,
            rotate_z: 0.0,
            skew_x: 0.0,
            skew_y: 0.0,
            scale: 1.0,
            scale_x: 1.0,
            scale_y: 1.0,
            is_force_3d: false,
            is_soft_hide: true,
            opacity: 1.0,
            transform_origin: "50% 50%".to_string(),
        }
    }
}

impl Html {
    pub fn new(el: HtmlElement) -> Self {
        Self {
            el,
            cache: StyleCache::new(),
            props: HtmlProps::default(),
        }
    }

    /// Draw current properties
    pub fn draw(&mut self) {
        let style = self.el.style();
        let p = &self.props;

        // Draw transforms
        self.draw_transform(&style);

        // Draw transform origin
        let _ = self.cache.set_if_changed(&style, "transform-origin", &p.transform_origin);

        // Draw opacity
        let _ = self.cache.set_if_changed(&style, "opacity", &p.opacity.to_string());

        // Custom draw callback (for custom properties)
        // Would need to support custom properties
    }

    fn draw_transform(&mut self, style: &CssStyleDeclaration) {
        let p = &self.props;

        let transform = if self.is_3d() {
            // 3D transform
            format!(
                "translate3d({}, {}, {}) rotateX({}deg) rotateY({}deg) rotateZ({}deg) skew({}deg, {}deg) scale({}, {})",
                p.x, p.y, p.z,
                p.rotate_x, p.rotate_y, p.rotate_z,
                p.skew_x, p.skew_y,
                p.scale_x, p.scale_y
            )
        } else {
            // 2D transform (faster)
            format!(
                "translate({}, {}) rotate({}deg) skew({}deg, {}deg) scale({}, {})",
                p.x, p.y,
                p.rotate_z,
                p.skew_x, p.skew_y,
                p.scale_x, p.scale_y
            )
        };

        let _ = self.cache.set_if_changed(style, "transform", &transform);
    }

    fn is_3d(&self) -> bool {
        self.props.is_force_3d
            || self.props.rotate_x != 0.0
            || self.props.rotate_y != 0.0
            || self.props.z != "0"
    }

    /// Hide element
    pub fn hide(&mut self) {
        let style = self.el.style();

        if self.props.is_soft_hide {
            let _ = style.set_property("opacity", "0");
        } else {
            let _ = style.set_property("display", "none");
        }
    }

    /// Show element
    pub fn show(&mut self) {
        let style = self.el.style();

        if self.props.is_soft_hide {
            let _ = style.set_property("opacity", "1");
        } else {
            let _ = style.remove_property("display");
        }
    }
}
```

---

## 8. MotionPath

### 8.1 Path Parsing (path/parser.rs)

```rust
use wasm_bindgen::prelude::*;
use web_sys::SvgPathElement;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum PathParseError {
    #[error("Invalid path string")]
    InvalidString,
    #[error("Element not found: {0}")]
    ElementNotFound(String),
    #[error("Not a path element")]
    NotPathElement,
}

/// SVG path parser
pub struct PathParser;

impl PathParser {
    /// Parse path from various input types
    pub fn parse(input: &JsValue) -> Result<SvgPathElement, PathParseError> {
        // Already SVGPathElement
        if let Some(path) = input.dyn_ref::<SvgPathElement>() {
            return Ok(path.clone());
        }

        // String input
        if let Some(s) = input.as_string() {
            // CSS selector
            if !s.starts_with('m') && !s.starts_with('M') {
                return Self::parse_selector(&s);
            }

            // SVG path string
            return Self::parse_path_string(&s);
        }

        // Object with x, y (arc shift)
        if input.is_object() {
            let x = js_sys::Reflect::get(input, &"x".into())
                .ok()
                .and_then(|v| v.as_f64())
                .unwrap_or(0.0);
            let y = js_sys::Reflect::get(input, &"y".into())
                .ok()
                .and_then(|v| v.as_f64())
                .unwrap_or(0.0);

            return Self::create_arc_path(x, y);
        }

        Err(PathParseError::InvalidString)
    }

    fn parse_selector(selector: &str) -> Result<SvgPathElement, PathParseError> {
        let document = web_sys::window()
            .ok_or(PathParseError::ElementNotFound("No window".to_string()))?
            .document()
            .ok_or(PathParseError::ElementNotFound("No document".to_string()))?;

        let element = document.query_selector(selector)
            .map_err(|_| PathParseError::ElementNotFound(selector.to_string()))?
            .ok_or(PathParseError::ElementNotFound(selector.to_string()))?;

        element.dyn_into::<SvgPathElement>()
            .map_err(|_| PathParseError::NotPathElement)
    }

    fn parse_path_string(path_string: &str) -> Result<SvgPathElement, PathParseError> {
        let document = web_sys::window()
            .unwrap()
            .document()
            .unwrap();

        let path: SvgPathElement = document
            .create_element_ns(Some("http://www.w3.org/2000/svg"), "path")?
            .dyn_into()
            .map_err(|_| PathParseError::InvalidString)?;

        path.set_attribute("d", path_string)?;
        Ok(path)
    }

    fn create_arc_path(x: f64, y: f64) -> Result<SvgPathElement, PathParseError> {
        // Create quadratic Bezier arc
        let document = web_sys::window().unwrap().document().unwrap();

        let path: SvgPathElement = document
            .create_element_ns(Some("http://www.w3.org/2000/svg"), "path")?
            .dyn_into()
            .map_err(|_| PathParseError::InvalidString)?;

        // M start Q control end
        let d = format!("M 0,0 Q {},{} {},{}", x * 0.75, y * 0.5, x, y);
        path.set_attribute("d", &d)?;

        Ok(path)
    }
}

/// Arc path builder
pub struct ArcBuilder {
    start: (f64, f64),
    shift: (f64, f64),
    curvature_x: String,
    curvature_y: String,
}

impl ArcBuilder {
    pub fn new() -> Self {
        Self {
            start: (0.0, 0.0),
            shift: (0.0, 0.0),
            curvature_x: "75%".to_string(),
            curvature_y: "50%".to_string(),
        }
    }

    pub fn start(mut self, x: f64, y: f64) -> Self {
        self.start = (x, y);
        self
    }

    pub fn shift(mut self, x: f64, y: f64) -> Self {
        self.shift = (x, y);
        self
    }

    pub fn curvature(mut self, x: &str, y: &str) -> Self {
        self.curvature_x = x.to_string();
        self.curvature_y = y.to_string();
        self
    }

    pub fn build(self) -> Result<SvgPathElement, PathParseError> {
        let (start_x, start_y) = self.start;
        let (shift_x, shift_y) = self.shift;

        let end_x = start_x + shift_x;
        let end_y = start_y + shift_y;

        // Calculate control point
        let distance = (shift_x * shift_x + shift_y * shift_y).sqrt();
        let percent = distance / 100.0;
        let rotation = (shift_y / shift_x).atan() * (180.0 / std::f64::consts::PI) + 90.0;

        // Parse curvature
        let cx = Self::parse_curvature(&self.curvature_x, percent);
        let cy = Self::parse_curvature(&self.curvature_y, percent);

        // Calculate control point using radial point
        let control = Self::get_radial_point(
            (start_x, start_y),
            cx,
            rotation,
        );

        let d = format!(
            "M {},{} Q {},{} {},{}",
            start_x, start_y,
            control.0, control.1,
            end_x, end_y
        );

        PathParser::parse_path_string(&d)
    }

    fn parse_curvature(s: &str, percent: f64) -> f64 {
        if s.ends_with('%') {
            s.trim_end_matches('%')
                .parse::<f64>()
                .unwrap_or(75.0) * percent
        } else {
            s.parse::<f64>().unwrap_or(0.0)
        }
    }

    fn get_radial_point(center: (f64, f64), radius: f64, rotation: f64) -> (f64, f64) {
        let rad = (rotation - 90.0) * std::f64::consts::PI / 180.0;
        (
            center.0 + rad.cos() * radius,
            center.1 + rad.sin() * radius,
        )
    }
}
```

### 8.2 MotionPath Animation (path/motion_path.rs)

```rust
use wasm_bindgen::prelude::*;
use web_sys::{SvgPathElement, HtmlElement};
use crate::tween::Tween;
use crate::types::*;
use super::parser::{PathParser, PathParseError};

/// Path-based animation
pub struct MotionPath {
    /// Target element
    el: HtmlElement,

    /// SVG path
    path: SvgPathElement,

    /// Path length
    path_length: f64,

    /// Current rotation
    rotation: f64,

    /// Properties
    props: MotionPathProps,

    /// Tween
    tween: Option<Rc<RefCell<Tween>>>,

    /// Previous coordinates (for motion blur)
    prev_coords: Option<(f64, f64)>,
}

struct MotionPathProps {
    /// Path bounds
    path_start: Progress,
    path_end: Progress,

    /// Offsets
    offset_x: f64,
    offset_y: f64,

    /// Rotation
    is_rotation: bool,
    rotation_offset: Option<f64>,

    /// Motion blur
    motion_blur: f64,

    /// Behavior
    is_reverse: bool,
    is_composite_layer: bool,
}

impl MotionPath {
    pub fn new(options: MotionPathOptions) -> Result<Self, PathParseError> {
        let el = options.el
            .dyn_into::<HtmlElement>()
            .map_err(|_| PathParseError::NotPathElement)?;

        let path = PathParser::parse(&options.path)?;
        let path_length = path.get_total_length();

        let props = MotionPathProps {
            path_start: options.path_start.unwrap_or(0.0),
            path_end: options.path_end.unwrap_or(1.0),
            offset_x: options.offset_x.unwrap_or(0.0),
            offset_y: options.offset_y.unwrap_or(0.0),
            is_rotation: options.is_rotation.unwrap_or(false),
            rotation_offset: options.rotation_offset,
            motion_blur: options.motion_blur.unwrap_or(0.0),
            is_reverse: options.is_reverse.unwrap_or(false),
            is_composite_layer: options.is_composite_layer.unwrap_or(true),
        };

        let mut motion_path = Self {
            el,
            path,
            path_length,
            rotation: 0.0,
            props,
            tween: None,
            prev_coords: None,
        };

        // Create tween
        motion_path.create_tween(options);

        // Set composite layer if enabled
        if motion_path.props.is_composite_layer {
            let _ = motion_path.el.style().set_property("transform", "translateZ(0)");
        }

        Ok(motion_path)
    }

    fn create_tween(&mut self, options: MotionPathOptions) {
        let tween_options = TweenOptions {
            duration: options.duration.unwrap_or(1000.0),
            delay: options.delay.unwrap_or(0.0),
            easing: options.easing.unwrap_or_else(|| "linear".to_string()),
            // ... other options
            ..Default::default()
        };

        let tween = Rc::new(RefCell::new(Tween::new(tween_options)));

        // Set up onUpdate callback
        let el_clone = self.el.clone();
        // Would need to set up callback properly

        self.tween = Some(tween);
    }

    /// Set progress along path
    pub fn set_progress(&mut self, progress: Progress) {
        let p = &self.props;

        // Calculate length along path
        let sliced_len = self.path_length * (p.path_end - p.path_start);
        let start_len = p.path_start * self.path_length;

        let len = start_len + if !p.is_reverse {
            progress * sliced_len
        } else {
            (1.0 - progress) * sliced_len
        };

        // Get point at length
        let point = self.path.get_point_at_length(len);

        // Apply offsets
        let x = point.x() + p.offset_x;
        let y = point.y() + p.offset_y;

        // Calculate rotation if enabled
        if p.is_rotation || p.rotation_offset.is_some() {
            self.calculate_rotation(len);
        }

        // Apply transform
        self.set_transform(x, y);

        // Apply motion blur if enabled
        if p.motion_blur > 0.0 {
            self.apply_motion_blur(x, y);
        }
    }

    fn calculate_rotation(&mut self, len: f64) {
        // Get previous point (1 unit back)
        let prev_point = self.path.get_point_at_length(len - 1.0);
        let point = self.path.get_point_at_length(len);

        // Calculate angle
        let dy = point.y() - prev_point.y();
        let dx = point.x() - prev_point.x();

        let mut atan = dy / dx;
        if !atan.is_finite() {
            atan = 0.0;
        }

        self.rotation = atan * (180.0 / std::f64::consts::PI);

        // Apply offset
        if let Some(offset) = self.props.rotation_offset {
            self.rotation += offset;
        }
    }

    fn set_transform(&self, x: f64, y: f64) {
        let transform = if self.rotation != 0.0 {
            format!("translate({}px, {}px) rotate({}deg)", x, y, self.rotation)
        } else {
            format!("translate({}px, {}px)", x, y)
        };

        let _ = self.el.style().set_property("transform", &transform);
    }

    fn apply_motion_blur(&mut self, x: f64, y: f64) {
        if let Some((prev_x, prev_y)) = self.prev_coords {
            let speed_x = (x - prev_x).abs();
            let speed_y = (y - prev_y).abs();

            // Calculate blur amounts
            let blur_x = (speed_x / 16.0 * self.props.motion_blur).min(1.0).max(0.0);
            let blur_y = (speed_y / 16.0 * self.props.motion_blur).min(1.0).max(0.0);

            // Apply blur via SVG filter (would need to create filter)
            // For now, simplified
        }

        self.prev_coords = Some((x, y));
    }
}

struct MotionPathOptions {
    el: JsValue,
    path: JsValue,
    duration: Option<TimeMs>,
    delay: Option<TimeMs>,
    easing: Option<String>,
    path_start: Option<Progress>,
    path_end: Option<Progress>,
    offset_x: Option<f64>,
    offset_y: Option<f64>,
    is_rotation: Option<bool>,
    rotation_offset: Option<f64>,
    motion_blur: Option<f64>,
    is_reverse: Option<bool>,
    is_composite_layer: Option<bool>,
}
```

---

## 9. Burst/Particle System

### 9.1 Burst Implementation (burst/burst.rs)

```rust
use wasm_bindgen::prelude::*;
use crate::shape_swirl::ShapeSwirl;
use crate::tween::Timeline;
use crate::types::*;
use std::rc::Rc;
use std::cell::RefCell;

/// Particle burst system
pub struct Burst {
    /// Child particles
    swirls: Vec<ShapeSwirl>,

    /// Timeline for all children
    timeline: Rc<RefCell<Timeline>>,

    /// Properties
    props: BurstProps,
}

struct BurstProps {
    count: u32,
    degree: f64,
    radius: f64,
    radius_x: Option<f64>,
    radius_y: Option<f64>,

    /// Child options
    children: JsValue,
}

impl Burst {
    pub fn new(options: BurstOptions) -> Self {
        let props = BurstProps {
            count: options.count.unwrap_or(5),
            degree: options.degree.unwrap_or(360.0),
            radius: options.radius.unwrap_or(50.0),
            radius_x: options.radius_x,
            radius_y: options.radius_y,
            children: options.children,
        };

        let mut burst = Self {
            swirls: Vec::new(),
            timeline: Rc::new(RefCell::new(Timeline::new())),
            props,
        };

        burst.create_children();
        burst
    }

    fn create_children(&mut self) {
        let p = &self.props;
        let degree_step = p.degree / p.count as f64;

        for i in 0..p.count {
            let degree_shift = degree_step * i as f64;

            // Calculate child position on radial path
            let rad_angle = (degree_shift - 90.0) * std::f64::consts::PI / 180.0;
            let x = rad_angle.cos() * p.radius;
            let y = rad_angle.sin() * p.radius;

            // Create child options
            let child_opts = self.get_child_options(i, degree_shift, x, y);

            // Create ShapeSwirl
            let swirl = ShapeSwirl::new(child_opts);
            self.swirls.push(swirl);
        }

        // Add all children to timeline
        for swirl in &self.swirls {
            // Add to timeline
        }
    }

    fn get_child_options(
        &self,
        index: u32,
        degree_shift: f64,
        x: f64,
        y: f64,
    ) -> ShapeSwirlOptions {
        // Clone children options and add burst-specific properties
        ShapeSwirlOptions {
            degree_shift: Some(degree_shift),
            x: Some(x.to_string()),
            y: Some(y.to_string()),
            // ... merge with children options
            ..Default::default()
        }
    }

    /// Play burst
    pub fn play(&mut self) {
        self.timeline.borrow_mut().play();
    }
}

struct BurstOptions {
    count: Option<u32>,
    degree: Option<f64>,
    radius: Option<f64>,
    radius_x: Option<f64>,
    radius_y: Option<f64>,
    children: JsValue,
}
```

---

## 10. Stagger Patterns

### 10.1 Stagger Implementation (stagger/patterns.rs)

```rust
use crate::types::TimeMs;

/// Stagger pattern types
pub enum StaggerPattern {
    /// Linear stagger from start
    From { amount: TimeMs, start: TimeMs },

    /// Stagger from center
    FromCenter { amount: TimeMs },

    /// Random stagger
    Rand { min: TimeMs, max: TimeMs },

    /// Repeat pattern
    Repeat { pattern: Vec<TimeMs> },
}

impl StaggerPattern {
    /// Calculate delay for index
    pub fn delay(&self, index: u32, total: u32) -> TimeMs {
        match self {
            StaggerPattern::From { amount, start } => {
                start + index as f64 * amount
            }

            StaggerPattern::FromCenter { amount } => {
                let center = (total / 2) as f64;
                (center - index as f64).abs() * amount
            }

            StaggerPattern::Rand { min, max } => {
                use rand::Rng;
                let mut rng = rand::thread_rng();
                rng.gen_range(*min..=*max)
            }

            StaggerPattern::Repeat { pattern } => {
                pattern[index as usize % pattern.len()]
            }
        }
    }
}

/// Stagger helper functions
pub mod stagger {
    use super::*;

    /// Linear stagger from start
    pub fn from(amount: TimeMs) -> StaggerPattern {
        StaggerPattern::From { amount, start: 0.0 }
    }

    /// Linear stagger with custom start
    pub fn from_with_start(amount: TimeMs, start: TimeMs) -> StaggerPattern {
        StaggerPattern::From { amount, start }
    }

    /// Stagger from center
    pub fn from_center(amount: TimeMs) -> StaggerPattern {
        StaggerPattern::FromCenter { amount }
    }

    /// Random stagger
    pub fn rand(min: TimeMs, max: TimeMs) -> StaggerPattern {
        StaggerPattern::Rand { min, max }
    }

    /// Repeat pattern
    pub fn repeat(pattern: Vec<TimeMs>) -> StaggerPattern {
        StaggerPattern::Repeat { pattern }
    }
}

// Usage example:
// let delays: Vec<TimeMs> = (0..5)
//     .map(|i| stagger::from(100.0).delay(i, 5))
//     .collect();
// // [0, 100, 200, 300, 400]
```

---

## 11. Performance Patterns

### 11.1 RequestAnimationFrame Loop Optimization

```rust
// In tween/tweener.rs - optimized RAF loop

const MAX_TWEENS_PER_FRAME: usize = 100;

impl Tweener {
    /// Optimized loop with batching
    fn update_optimized(&mut self, time: f64) {
        let len = self.tweens.len();

        // Process in batches to prevent frame drops
        let mut processed = 0;
        let mut i = len;

        while i > 0 && processed < MAX_TWEENS_PER_FRAME {
            i -= 1;
            processed += 1;

            let tween = &self.tweens[i];
            let is_complete = {
                let mut t = tween.borrow_mut();
                t.update(time)
            };

            if is_complete {
                self.tweens.remove(i);
            }
        }
    }
}
```

### 11.2 Float32Array for Sample Tables

```rust
// Use Box<[f64]> for sample storage (contiguous memory)
// In production, could use wasm-bindgen to access JS Float32Array

pub struct SampleTable {
    samples: Box<[f64; 11]>,  // Fixed size for bezier samples
}

impl SampleTable {
    pub fn new() -> Self {
        Self {
            samples: Box::new([0.0; 11]),
        }
    }

    pub fn compute(&mut self, x1: f64, x2: f64) {
        for i in 0..11 {
            let t = i as f64 * 0.1;
            self.samples[i] = Self::calc_bezier(t, x1, x2);
        }
    }
}
```

### 11.3 Object Pooling for Burst

```rust
use std::collections::VecDeque;

/// Object pool for expensive-to-create particles
pub struct ParticlePool<T> {
    pool: VecDeque<T>,
    factory: Box<dyn Fn() -> T>,
    max_size: usize,
}

impl<T> ParticlePool<T> {
    pub fn new(factory: impl Fn() -> T + 'static, max_size: usize) -> Self {
        Self {
            pool: VecDeque::with_capacity(max_size),
            factory: Box::new(factory),
            max_size,
        }
    }

    pub fn acquire(&mut self) -> T {
        self.pool.pop_front().unwrap_or_else(|| (self.factory)())
    }

    pub fn release(&mut self, particle: T) {
        if self.pool.len() < self.max_size {
            self.pool.push_back(particle);
        }
    }
}
```

---

## 12. WASM Integration

### 12.1 JavaScript Interop

```rust
// lib.rs - WASM exports

use wasm_bindgen::prelude::*;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = console)]
    fn log(s: &str);
}

/// Initialize mojs
#[wasm_bindgen(start)]
pub fn init() {
    console_error_panic_hook::set_once();
    log("mojs-rs initialized");
}

/// Create shape element
#[wasm_bindgen(js_name = "createShape")]
pub fn create_shape_js(options: JsValue) -> Result<JsValue, JsValue> {
    let shape = shape::Shape::new(options)?;
    Ok(JsValue::from(shape))
}

/// Create burst element
#[wasm_bindgen(js_name = "createBurst")]
pub fn create_burst_js(options: JsValue) -> Result<JsValue, JsValue> {
    let burst = burst::Burst::new(options)?;
    Ok(JsValue::from(burst))
}

/// Create timeline
#[wasm_bindgen(js_name = "createTimeline")]
pub fn create_timeline_js() -> JsValue {
    let timeline = tween::Timeline::new();
    JsValue::from(timeline)
}
```

### 12.2 Usage from JavaScript

```javascript
// After building with wasm-pack

import init, {
    Mojs,
    createShape,
    createBurst,
    createTimeline
} from './pkg/mojs_rs.js';

async function run() {
    await init();

    const mojs = new Mojs();

    // Create shape
    const shape = createShape({
        shape: 'circle',
        radius: { 0: 100 },
        fill: { 'red': 'blue' },
        duration: 1000,
        easing: 'sin.out'
    });

    shape.play();

    // Create burst
    const burst = createBurst({
        radius: { 0: 200 },
        count: 20,
        children: {
            shape: 'circle',
            radius: 'rand(10, 20)',
            fill: ['red', 'blue', 'yellow'],
            duration: 2000,
            easing: 'cubic.out'
        }
    });

    burst.play();

    // Create timeline
    const timeline = createTimeline()
        .add(shape)
        .append(burst);

    timeline.play();
}

run();
```

### 12.3 Build Configuration

```bash
# Install wasm-pack
cargo install wasm-pack

# Build for web
wasm-pack build --target web --release

# Build for bundlers
wasm-pack build --target bundler --release

# Run tests
wasm-pack test --headless --firefox

# Generate docs
cargo doc --no-deps --open
```

---

## 13. Complete Example

### 13.1 Full Animation Example

```rust
// examples/burst_animation.rs

use mojs_rs::{Mojs, shape, burst, tween};
use wasm_bindgen::prelude::*;
use web_sys::Document;

#[wasm_bindgen]
pub fn run_burst_demo() -> Result<(), JsValue> {
    let mojs = Mojs::new();

    // Create main burst
    let main_burst = burst::Burst::new(burst::BurstOptions {
        count: Some(15),
        degree: Some(360.0),
        radius: Some(150.0),
        children: js_sys::Object::new().into(),
    })?;

    // Create shape animation
    let shape = shape::Shape::new(shape::ShapeOptions {
        shape_type: Some(shape::ShapeType::Circle),
        radius: Some(js_value!({ "0": 50, "100": 0 })),
        fill: Some(js_value!({ "deeppink": "cyan" })),
        duration: Some(1500.0),
        easing: Some("cubic.out".to_string()),
        ..Default::default()
    })?;

    // Create timeline
    let mut timeline = tween::Timeline::new();
    timeline
        .add(vec![shape.rc()])
        .append(vec![main_burst.rc()]);

    timeline.play();

    Ok(())
}
```

### 13.2 HTML Integration

```html
<!DOCTYPE html>
<html>
<head>
    <meta charset="utf-8">
    <title>mojs-rs Demo</title>
    <style>
        body {
            margin: 0;
            display: flex;
            justify-content: center;
            align-items: center;
            height: 100vh;
            background: #1a1a2e;
        }
        .container {
            position: relative;
            width: 400px;
            height: 400px;
        }
    </style>
</head>
<body>
    <div class="container" id="container"></div>

    <script type="module">
        import init, { run_burst_demo } from './pkg/mojs_rs.js';

        async function main() {
            await init();
            run_burst_demo();
        }

        main();
    </script>
</body>
</html>
```

---

## Appendix: JS/CoffeeScript to Rust Mapping

| JavaScript/CoffeeScript | Rust Equivalent |
|------------------------|-----------------|
| `class Tween` | `pub struct Tween` |
| `constructor(o)` | `pub fn new(options: TweenOptions)` |
| `_defaults = {}` | `impl Default for TweenOptions` |
| `@_props = {}` | `props: TweenProps` |
| `->` (CoffeeScript arrow) | `FnMut` / closures |
| `typeof x` | `std::any::type_name` or JS interop |
| `null` / `undefined` | `Option<T>` |
| `Array` | `Vec<T>` |
| `Object` | `HashMap<K, V>` or `js_sys::Object` |
| `function` | `js_sys::Function` |
| `new Array(10)` | `vec![0.0; 10]` or `Box::new([0.0; 10])` |
| `requestAnimationFrame` | `web_sys::Window::request_animation_frame` |
| `document.createElement` | `Document::create_element` |
| `el.setAttribute` | `Element::set_attribute` |
| `el.style.prop` | `CssStyleDeclaration::set_property` |

---

## Performance Considerations

1. **Memory**: Rust's ownership model prevents memory leaks, but be careful with `Rc<RefCell<T>>` cycles
2. **GC**: No garbage collector pauses, but JS interop values need manual cleanup
3. **WASM Size**: Use `opt-level = "s"` and `lto = true` for smaller binaries
4. **DOM Access**: Minimize JS boundary crossings - batch DOM operations
5. **Float32**: Consider using `wasm-bindgen` to access JS `Float32Array` for sample tables

---

## Next Steps

1. **Implement core modules** in order: Tween → Delta → Easing → Timeline
2. **Add rendering layer**: SVG → HTML
3. **Build shape system**: Circle, Rect, Polygon, etc.
4. **Implement MotionPath**: Path parsing → sampling → animation
5. **Add Burst/Stagger**: Particle system → stagger patterns
6. **Create WASM bindings**: Export main API
7. **Write tests**: Unit tests for each module
8. **Build examples**: Demo animations showing all features
9. **Optimize**: Profile and optimize hot paths
10. **Document**: API documentation and usage guides

This guide provides the architecture and code structure for a complete Rust reimplementation of mojs. The type system and ownership model of Rust will catch many bugs at compile time that would be runtime errors in JavaScript, while WASM provides near-native performance for animation calculations.
