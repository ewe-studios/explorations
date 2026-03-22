---
name: FFrames Core
description: Core video generation framework with SVG-based scene composition and timeline management
type: sub-project
source: /home/darkvoid/Boxxed/@formulas/src.rust/src.fframes/fframes/
---

# FFrames Core - Video Generation Framework

## Overview

FFrames Core is the **foundational crate** of the FFrames video rendering ecosystem. It provides the abstractions and traits for defining videos programmatically with SVG-based rendering, timeline management, and audio synchronization.

## Key Responsibilities

1. **Video Trait**: Main interface for defining video properties
2. **Scene Trait**: Per-scene rendering with temporal awareness
3. **Frame Context**: Temporal helpers for animation and rendering
4. **Timeline Resolution**: Computing duration and scene ranges
5. **Audio Map**: Audio timeline composition and mixing
6. **Animation System**: Spring physics and keyframe animations
7. **Media Provider**: Abstracting media resolution

## Directory Structure

```
fframes/
├── src/
│   ├── animation/            # Animation system
│   │   ├── mod.rs
│   │   ├── spring.rs         # Spring physics simulation
│   │   ├── keyframe.rs       # Keyframe interpolation
│   │   ├── easing.rs         # Easing functions
│   │   └── timeline.rs       # Animation timeline macro
│   ├── frame.rs              # Frame abstraction
│   ├── scenes.rs             # Scene trait and composition
│   ├── video.rs              # Video trait (main interface)
│   ├── audio_map.rs          # Audio timeline and mixing
│   ├── media_provider.rs     # Media resolution abstraction
│   ├── fframes_context.rs    # Rendering context
│   ├── text.rs               # Text shaping and line breaking
│   ├── svgr.rs               # SVG tree conversion
│   ├── duration.rs           # Flexible duration specification
│   └── lib.rs                # Crate root
├── Cargo.toml
└── README.md
```

## Core Abstractions

### Video Trait

The `Video` trait is the primary interface users implement:

```rust
pub trait Video: Sync + Send {
    /// Frames per second (constant)
    const FPS: usize;

    /// Video width in pixels (constant)
    const WIDTH: usize;

    /// Video height in pixels (constant)
    const HEIGHT: usize;

    /// Total duration of the video
    fn duration(&self) -> Duration;

    /// Audio timeline for the video
    fn audio(&self) -> AudioMap {
        AudioMap::none()
    }

    /// Render a single frame
    fn render_frame(&self, frame: Frame, ctx: &FFramesContext) -> Svgr;

    /// Optional: Scene-based rendering
    fn scenes(&self) -> Vec<Box<dyn Scene>> {
        Vec::new()
    }
}
```

### Frame Context

```rust
pub struct Frame {
    /// Current frame index (0-based)
    pub index: usize,

    /// Frames per second
    pub fps: usize,

    /// Current time in seconds
    pub current_second: f32,

    /// Current progress (0.0 to 1.0)
    pub progress: f32,

    /// Reference to scene info (if scene-based)
    pub scene_info: Option<SceneInfo>,
}

impl Frame {
    /// Get current time as Duration
    pub fn as_duration(&self) -> Duration {
        Duration::from_secs_f32(self.current_second)
    }

    /// Animate value using spring physics
    pub fn animate<T: Interpolate>(
        &self,
        from: T,
        to: T,
        easing: Easing,
    ) -> T {
        match easing {
            Easing::Linear(duration) => {
                let t = (self.current_second / duration).min(1.0);
                T::interpolate(from, to, t)
            }
            Easing::Spring { mass, stiffness, damping } => {
                self.spring_animate(from, to, mass, stiffness, damping)
            }
        }
    }

    /// Get subtitle phrase for current frame
    pub fn get_subtitle_phrase<'a>(
        &self,
        subtitles: &'a Vtt<'a>,
    ) -> Option<&'a str> {
        subtitles
            .cues
            .iter()
            .find(|cue| {
                let cue_time_ms = cue.start;
                let frame_time_ms = self.current_second * 1000.0;
                frame_time_ms >= cue_time_ms
                    && frame_time_ms < cue.end
            })
            .map(|cue| cue.text)
    }

    /// Text wrapping with caching
    pub fn text_break_lines_structure(
        &self,
        ctx: &FFramesContext,
        text: &str,
        opts: BreakLinesOpts,
    ) -> WrappedText {
        // Uses LRU cache for performance
        ctx.get_wrapped_text(text, opts)
    }

    /// Audio visualization helper
    pub fn visualize_audio_frame(
        &self,
        input: VisualizeFrameInput,
    ) -> Vec<f32> {
        // FFT-based audio visualization
        microfft::fft(&input.audio_samples)
    }
}
```

### Scene Trait

```rust
pub trait Scene: Debug + Sync + Send {
    /// Duration of this scene
    fn duration(&self) -> Duration;

    /// Render a frame within this scene
    fn render_frame(
        &self,
        frame: Frame,
        ctx: &FFramesContext
    ) -> Svgr;

    /// Overlap with adjacent scenes (for transitions)
    fn overlap(&self) -> Overlap {
        Overlap::None
    }

    /// Audio specific to this scene
    fn audio(&self) -> AudioMap {
        AudioMap::none()
    }

    /// Scene name for debugging
    fn name(&self) -> &'static str {
        std::any::type_name::<Self>()
    }
}

pub enum Overlap {
    None,
    PreviousAndNext {
        previous: f32, // seconds
        next: f32,     // seconds
    },
}
```

### Duration System

```rust
pub enum Duration {
    /// Exact seconds
    Seconds(f32),

    /// Exact frames
    Frames(usize),

    /// Infer from audio file duration
    FromAudio(&'static str),

    /// Infer from scenes
    Auto,

    /// Until end of audio track
    UntilAudioEnd(&'static str),
}

impl Duration {
    pub fn resolve(
        &self,
        get_audio_duration: impl Fn(&str) -> Option<f32>,
        scenes: &[Box<dyn Scene>],
    ) -> Option<f32> {
        match self {
            Duration::Seconds(s) => Some(*s),
            Duration::Frames(f) => Some(*f as f32 / 60.0),
            Duration::FromAudio(path) => get_audio_duration(path),
            Duration::Auto => {
                // Sum of scene durations
                scenes.iter()
                    .map(|s| s.duration().resolve(get_audio_duration, scenes))
                    .sum()
            }
            Duration::UntilAudioEnd(path) => get_audio_duration(path),
        }
    }
}
```

## Animation System

### Spring Physics

```rust
pub struct SpringState {
    pub position: f32,
    pub velocity: f32,
}

impl SpringState {
    /// Simulate spring physics
    pub fn update(
        &mut self,
        target: f32,
        mass: f32,
        stiffness: f32,
        damping: f32,
        dt: f32,
    ) {
        // Hooke's law: F = -kx
        let displacement = target - self.position;
        let spring_force = displacement * stiffness;

        // Damping: F = -bv
        let damping_force = -self.velocity * damping;

        // F = ma
        let acceleration = (spring_force + damping_force) / mass;

        // Integrate (semi-implicit Euler)
        self.velocity += acceleration * dt;
        self.position += self.velocity * dt;
    }
}

// Usage in Frame
impl Frame {
    pub fn spring_animate<T: Interpolate>(
        &self,
        from: T,
        to: T,
        mass: f32,
        stiffness: f32,
        damping: f32,
    ) -> T {
        let mut state = SpringState {
            position: 0.0,
            velocity: 0.0,
        };

        // Simulate spring over time
        let elapsed = self.current_second;
        let mut t = 0.0;
        let dt = 1.0 / 1000.0; // 1ms steps

        while t < elapsed {
            state.update(1.0, mass, stiffness, damping, dt);
            t += dt;
        }

        T::interpolate(from, to, state.position)
    }
}
```

### Easing Functions

```rust
pub enum Easing {
    Linear(f32), // duration
    Spring {
        mass: f32,
        stiffness: f32,
        damping: f32,
    },
    EaseInQuad(f32),
    EaseOutQuad(f32),
    EaseInOutQuad(f32),
    // ... more easing functions
}

fn ease_in_quad(t: f32) -> f32 {
    t * t
}

fn ease_out_quad(t: f32) -> f32 {
    t * (2.0 - t)
}

fn ease_in_out_quad(t: f32) -> f32 {
    if t < 0.5 {
        2.0 * t * t
    } else {
        -1.0 + (4.0 - 2.0 * t) * t
    }
}
```

### Timeline Macro

```rust
// Declarative animation timeline
#[macro_export]
macro_rules! timeline {
    (
        on $time:expr,
        val $from:expr => $to:expr,
        $easing:expr
    ) => {
        AnimationSpec {
            start_time: $time,
            from: $from,
            to: $to,
            easing: $easing,
        }
    };

    (
        $(
            on $time:expr,
            val $from:expr => $to:expr,
            $easing:expr
        ),* $(,)?
    ) => {
        vec![
            $(timeline!(on $time, val $from => $to, $easing)),*
        ]
    };
}

// Usage
frame.animate!(timeline!(
    on 0.0, val 0.0 => 100.0, Easing::Spring { mass: 1.0, stiffness: 100.0, damping: 15.0 },
    on 1.0, val 100.0 => 50.0, Easing::EaseOutQuad(0.5),
));
```

## Audio Map System

```rust
pub struct AudioMap(
    Vec<(
        &'static str, // Audio file path
        AudioTimestamp, // Start
        AudioTimestamp, // End
    )>
);

pub enum AudioTimestamp {
    Second(f32),
    Frame(usize),
    Eof, // End of file
}

impl AudioMap {
    /// Create from iterator
    pub fn from_iter<I>(iter: I) -> Self
    where
        I: IntoIterator<Item = (&'static str, Range<AudioTimestamp>)>,
    {
        AudioMap(
            iter.into_iter()
                .map(|(path, range)| {
                    (path, range.start, range.end)
                })
                .collect(),
        )
    }

    /// Get audio ranges for a specific time
    pub fn get_active_at(&self, time: f32) -> Vec<(&str, f32, f32)> {
        self.0
            .iter()
            .filter_map(|(path, start, end)| {
                let start_sec = match start {
                    AudioTimestamp::Second(s) => *s,
                    AudioTimestamp::Frame(f) => *f as f32 / 60.0,
                    AudioTimestamp::Eof => return None,
                };
                let end_sec = match end {
                    AudioTimestamp::Second(s) => *s,
                    AudioTimestamp::Frame(f) => *f as f32 / 60.0,
                    AudioTimestamp::Eof => f32::INFINITY,
                };

                if time >= start_sec && time < end_sec {
                    Some((*path, time - start_sec, end_sec - start_sec))
                } else {
                    None
                }
            })
            .collect()
    }

    /// Mix multiple audio tracks
    pub fn mix_audio(
        &self,
        time: f32,
        resolve_audio: impl Fn(&str) -> Vec<f32>,
    ) -> Vec<f32> {
        let mut mixed = Vec::new();
        let active = self.get_active_at(time);

        for (path, offset, _duration) in active {
            let samples = resolve_audio(path);
            // Mix with normalization: a + b - (a * b)
            for (i, sample) in samples.iter().enumerate() {
                if i >= mixed.len() {
                    mixed.push(*sample);
                } else {
                    mixed[i] = mixed[i] + sample - (mixed[i] * sample);
                }
            }
        }

        mixed
    }
}
```

## Media Provider

```rust
pub trait MediaProvider: Sync + Send {
    /// Resolve audio data
    fn resolve_audio(&self, path: &str) -> AudioData;

    /// Resolve video data
    fn resolve_video(&self, path: &str) -> VideoData;

    /// Resolve image data
    fn resolve_image(&self, path: &str) -> ImageData;

    /// Resolve font data
    fn resolve_font(&self, family: &str) -> FontData;

    /// Resolve subtitles
    fn resolve_subtitles(&self, path: &str) -> Vtt;
}

// Default implementation for renderer
pub struct DefaultMediaProvider {
    media_dir: PathBuf,
    audio_cache: LRUCache<String, AudioData>,
    image_cache: LRUCache<String, ImageData>,
}

impl MediaProvider for DefaultMediaProvider {
    fn resolve_audio(&self, path: &str) -> AudioData {
        if let Some(cached) = self.audio_cache.get(path) {
            return cached.clone();
        }

        let full_path = self.media_dir.join(path);
        let data = load_audio_ffmpeg(&full_path);

        self.audio_cache.put(path.to_string(), data.clone());
        data
    }

    // ... other methods
}
```

## Text Handling

```rust
pub struct TextShaper {
    font_db: FontDb,
    cache: LRUCache<String, ShapedText>,
}

impl TextShaper {
    /// Shape text for rendering
    pub fn shape(
        &mut self,
        text: &str,
        font_family: &str,
        font_size: f32,
    ) -> ShapedText {
        let key = format!("{}|{}|{}", text, font_family, font_size);

        if let Some(cached) = self.cache.get(&key) {
            return cached.clone();
        }

        let font = self.font_db.get(font_family);
        let shaped = ttf_parser::shape(font, text, font_size);

        self.cache.insert(key, shaped.clone());
        shaped
    }

    /// Break text into lines
    pub fn break_lines(
        &mut self,
        text: &str,
        font_family: &str,
        font_size: f32,
        max_width: f32,
    ) -> Vec<String> {
        let words: Vec<_> = text.split_whitespace().collect();
        let mut lines = Vec::new();
        let mut current_line = String::new();

        for word in words {
            let test_line = if current_line.is_empty() {
                word.to_string()
            } else {
                format!("{} {}", current_line, word)
            };

            let shaped = self.shape(&test_line, font_family, font_size);
            if shaped.width <= max_width {
                current_line = test_line;
            } else {
                if !current_line.is_empty() {
                    lines.push(current_line);
                }
                current_line = word.to_string();
            }
        }

        if !current_line.is_empty() {
            lines.push(current_line);
        }

        lines
    }
}
```

## FFramesContext

```rust
pub struct FFramesContext<'media> {
    /// Current mode (editor vs renderer)
    pub mode: FFramesMode,

    /// Media provider
    pub media: Box<dyn MediaProvider + 'media>,

    /// Current scene info
    pub scene_info: Option<SceneInfo>,

    /// Text shaper
    pub text_shaper: TextShaper,

    /// Audio cache
    pub audio_cache: HashMap<String, AudioData>,

    /// Font database
    pub font_db: FontDb,
}

impl<'media> FFramesContext<'media> {
    /// Get audio data with caching
    pub fn get_audio(&mut self, path: &str) -> Option<&AudioData> {
        if !self.audio_cache.contains_key(path) {
            let data = self.media.resolve_audio(path);
            self.audio_cache.insert(path.to_string(), data);
        }
        self.audio_cache.get(path)
    }

    /// Get mixed audio for current time
    pub fn get_mixed_audio_at(
        &mut self,
        time: f32,
        audio_map: &AudioMap,
    ) -> Vec<f32> {
        audio_map.mix_audio(time, |p| {
            self.get_audio(p).map(|a| a.samples.clone()).unwrap_or_default()
        })
    }

    /// Get subtitle for current frame
    pub fn get_subtitle_at(
        &mut self,
        path: &str,
        time: f32,
    ) -> Option<&str> {
        let vtt = self.media.resolve_subtitles(path);
        vtt.cues
            .iter()
            .find(|cue| time * 1000.0 >= cue.start && time * 1000.0 < cue.end)
            .map(|cue| cue.text)
    }
}
```

## SVG Integration

```rust
/// Convert Svgr macro output to renderable tree
pub fn svgr_to_rtree(svg: Svgr, options: &RenderOptions) -> RTree {
    // Parse SVG string into Resvg tree
    let opt = usvg::Options::default();
    let rtree = usvg::Tree::from_str(&svg.to_string(), &opt).unwrap();
    rtree
}

// Svgr macro for declarative SVG
#[macro_export]
macro_rules! svgr {
    (<svg $($attrs:tt)*> $($children:tt)* </svg>) => {
        Svgr::Svg(SvgAttrs { $($attrs)* }, vec![$($children)*])
    };

    (<circle $($attrs:tt)* />) => {
        Svgr::Element("circle", parse_attrs!($($attrs)*))
    };

    // ... more element types
}
```

## Example Usage

```rust
use fframes::*;

struct IntroVideo<'a> {
    background_music: &'a str,
    title: &'a str,
}

impl<'a> Video for IntroVideo<'a> {
    const FPS: usize = 60;
    const WIDTH: usize = 1920;
    const HEIGHT: usize = 1080;

    fn duration(&self) -> Duration {
        Duration::Seconds(10.0)
    }

    fn audio(&self) -> AudioMap {
        AudioMap::from([(
            self.background_music,
            AudioTimestamp::Second(0.0)..AudioTimestamp::Second(10.0)
        )])
    }

    fn render_frame(&self, frame: Frame, ctx: &FFramesContext) -> Svgr {
        // Animate title position with spring
        let title_y = frame.animate!(timeline!(
            on 0.0, val -100.0 => 540.0,
            Easing::Spring { mass: 1.5, stiffness: 120.0, damping: 14.0 }
        ));

        // Get subtitle if any
        let subtitle = frame.get_subtitle_phrase(&ctx.subtitles);

        svgr!(
            <svg width="1920" height="1080">
                <rect width="100%" height="100%" fill="#1a1a2e" />

                <text
                    x="50%"
                    y={title_y.to_string()}
                    font-family="Roboto"
                    font-size="72"
                    fill="white"
                    text-anchor="middle"
                >
                    {self.title}
                </text>

                {subtitle.map(|s| svgr!(
                    <text
                        x="50%"
                        y="900"
                        font-family="Roboto"
                        font-size="36"
                        fill="#aaa"
                        text-anchor="middle"
                    >
                        {s}
                    </text>
                )).unwrap_or_else(|| svgr!(<g/>))}
            </svg>
        )
    }
}
```

## Related Documents

- [FFrames Renderer](./fframes-renderer-exploration.md) - CPU/GPU rendering backends
- [FFrames Editor](./fframes-editor-exploration.md) - Web-based video editor
- [WebVTT Parser](./webvtt-parser-exploration.md) - Subtitle parsing

## Sources

- Source: `/home/darkvoid/Boxxed/@formulas/src.rust/src.fframes/fframes/`
- FFrames Main Exploration: `../../fframes/exploration.md`
