---
source: /home/darkvoid/Boxxed/@dev/repo-expolorations/fframes/
explored_at: 2026-03-22
revised_at: 2026-03-22
workspace: fframes-workspace
---

# Rust Revision: FFrames Sub-Projects

## Overview

This document consolidates the FFrames sub-project explorations into implementation guidance for building programmatic video creation applications in Rust. The revision covers the core video rendering framework, GPU/CPU backends, encoding, and supporting libraries.

## Sub-Projects Covered

| Sub-Project | Purpose | Implementation Priority |
|-------------|---------|------------------------|
| fframes-core | Video trait, Scene, Frame | Critical |
| fframes-renderer | CPU/GPU rendering, FFmpeg | Critical |
| webvtt-parser | Subtitle parsing | High |
| rust-ffmpeg-sys | FFmpeg bindings | Critical |
| rust-skia | Skia graphics | Critical |
| resvg | SVG rendering | High |
| svgtypes | SVG types | Medium |
| remotion | React video reference | Medium |
| zlob | Glob matching | Low |
| fff.nvim | File finder | Low |

## Workspace Structure

```
fframes-workspace/
├── core/
│   ├── fframes-core/             # Core traits and types
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs            # Video trait, Scene trait
│   │       ├── frame.rs          # Frame context
│   │       ├── animation.rs      # Animation system
│   │       └── spring.rs         # Spring physics
│   └── fframes-renderer/         # Rendering backends
│       ├── Cargo.toml
│       └── src/
│           ├── lib.rs            # Renderer trait
│           ├── cpu/              # CPU rendering (tiny-skia)
│           ├── gpu/              # GPU rendering (Skia + Vulkan/Metal)
│           └── ffmpeg/           # FFmpeg encoder
├── parsers/
│   ├── webvtt-parser/            # Subtitle parsing
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs            # Zero-copy parsing
│   │       ├── cue.rs            # Cue timing
│   │       └── format.rs         # Text formatting
│   └── svgtypes/                 # SVG type definitions
│       ├── Cargo.toml
│       └── src/
│           ├── path.rs           # Path parsing
│           └── color.rs          # Color parsing
├── renderers/
│   ├── rust-skia/                # Skia bindings
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── surface.rs        # GPU surfaces
│   │       └── canvas.rs         # Drawing operations
│   └── resvg/                    # SVG renderer
│       ├── Cargo.toml
│       └── src/
│           ├── lib.rs            # SVG to raster
│           └── render.rs         # Rendering logic
├── encoding/
│   └── rust-ffmpeg-sys/          # FFmpeg FFI
│       ├── Cargo.toml
│       └── src/
│           ├── codec.rs          # AVCodec
│           ├── format.rs         # AVFormat
│           └── color.rs          # Color space conversion
└── examples/
    ├── basic-video/              # Simple video creation
    ├── subtitled-video/          # With WebVTT subtitles
    └── animated-presentation/    # Animated slides
```

## Core Implementation

### Video Trait

```rust
// core/fframes-core/src/lib.rs
use std::time::Duration;
use resvg::tiny_skia;

/// Core video trait that all videos must implement
pub trait Video: Sync + Send {
    /// Frames per second
    const FPS: usize;

    /// Frame width in pixels
    const WIDTH: usize;

    /// Frame height in pixels
    const HEIGHT: usize;

    /// Total duration of the video
    fn duration(&self) -> Duration;

    /// Audio tracks (optional)
    fn audio(&self) -> AudioMap {
        AudioMap::none()
    }

    /// Render a specific frame
    fn render_frame(&self, frame: Frame, ctx: &FFramesContext) -> Scene;
}

/// Frame number wrapper
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Frame(pub usize);

impl Frame {
    pub fn from_seconds(seconds: f64, fps: usize) -> Self {
        Frame((seconds * fps as f64) as usize)
    }

    pub fn to_seconds(&self, fps: usize) -> f64 {
        self.0 as f64 / fps as f64
    }

    pub fn to_duration(&self, fps: usize) -> Duration {
        Duration::from_secs_f64(self.to_seconds(fps))
    }
}

/// Rendering context
pub struct FFramesContext {
    pub assets: AssetManager,
    pub fonts: FontManager,
    pub config: RenderConfig,
}

/// Scene to be rendered
pub trait Scene {
    fn render(&self, canvas: &mut tiny_skia::PixmapMut);
    fn size(&self) -> (u32, u32);
}
```

### Animation System

```rust
// core/fframes-core/src/animation.rs
use std::time::Duration;

/// Animation trait
pub trait Animation: Send + Sync {
    /// Get value at progress (0.0 to 1.0)
    fn value_at(&self, progress: f64) -> f64;

    /// Duration of animation
    fn duration(&self) -> Duration;

    /// Easing function
    fn easing(&self) -> Easing {
        Easing::Linear
    }
}

/// Easing functions
#[derive(Clone, Copy)]
pub enum Easing {
    Linear,
    EaseIn,
    EaseOut,
    EaseInOut,
    Spring {
        stiffness: f64,
        damping: f64,
        mass: f64,
    },
}

impl Easing {
    pub fn apply(&self, t: f64) -> f64 {
        match self {
            Easing::Linear => t,
            Easing::EaseIn => t * t * t,
            Easing::EaseOut => 1.0 - (1.0 - t).powi(3),
            Easing::EaseInOut => {
                if t < 0.5 {
                    4.0 * t * t * t
                } else {
                    1.0 - (-2.0 * t + 2.0).powi(3) / 2.0
                }
            }
            Easing::Spring { stiffness, damping, mass } => {
                // Spring physics calculation
                let omega_n = (stiffness / mass).sqrt();
                let zeta = damping / (2.0 * (stiffness * mass).sqrt());

                if zeta < 1.0 {
                    // Underdamped
                    let omega_d = omega_n * (1.0 - zeta * zeta).sqrt();
                    (-zeta * omega_n * t) * (omega_d * t).sin()
                } else {
                    // Overdamped - simplify to linear
                    t
                }
            }
        }
    }
}

/// Keyframe animation
pub struct KeyframeAnimation {
    keyframes: Vec<(f64, f64)>,  // (time, value)
    easing: Easing,
}

impl KeyframeAnimation {
    pub fn new(keyframes: Vec<(f64, f64)>) -> Self {
        Self {
            keyframes,
            easing: Easing::Linear,
        }
    }

    pub fn with_easing(mut self, easing: Easing) -> Self {
        self.easing = easing;
        self
    }
}

impl Animation for KeyframeAnimation {
    fn value_at(&self, progress: f64) -> f64 {
        // Find surrounding keyframes
        for i in 0..self.keyframes.len() - 1 {
            let (t0, v0) = self.keyframes[i];
            let (t1, v1) = self.keyframes[i + 1];

            if progress >= t0 && progress <= t1 {
                let local_progress = (progress - t0) / (t1 - t0);
                let eased = self.easing.apply(local_progress);
                return v0 + (v1 - v0) * eased;
            }
        }

        // Outside keyframe range
        if progress < self.keyframes[0].0 {
            self.keyframes[0].1
        } else {
            *self.keyframes.last().unwrap()
        }
    }

    fn duration(&self) -> Duration {
        Duration::from_secs_f64(self.keyframes.last().unwrap().0)
    }
}
```

### Renderer Implementation

```rust
// core/fframes-renderer/src/lib.rs
use fframes_core::{Video, Frame, FFramesContext, Scene};
use std::path::Path;

/// Renderer trait
pub trait Renderer: Send + Sync {
    /// Render a frame to pixmap
    fn render_frame(&self, scene: &dyn Scene) -> tiny_skia::Pixmap;

    /// Render multiple frames
    fn render_frames(&self, scenes: &[&dyn Scene]) -> Vec<tiny_skia::Pixmap> {
        scenes.iter().map(|s| self.render_frame(s)).collect()
    }
}

/// Video encoder trait
pub trait VideoEncoder: Send + Sync {
    /// Initialize encoder
    fn init(&mut self, width: u32, height: u32, fps: u32) -> Result<(), EncodeError>;

    /// Write a frame
    fn write_frame(&mut self, pixmap: &tiny_skia::Pixmap) -> Result<(), EncodeError>;

    /// Finish encoding
    fn finish(&mut self) -> Result<(), EncodeError>;
}

/// Main video creator
pub struct VideoCreator<R: Renderer, E: VideoEncoder> {
    renderer: R,
    encoder: E,
    context: FFramesContext,
}

impl<R: Renderer, E: VideoEncoder> VideoCreator<R, E> {
    pub fn new(renderer: R, encoder: E, context: FFramesContext) -> Self {
        Self {
            renderer,
            encoder,
            context,
        }
    }

    pub fn create_video<V: Video>(&mut self, video: &V, output: &Path) -> Result<(), CreateError> {
        // Initialize encoder
        self.encoder.init(
            V::WIDTH as u32,
            V::HEIGHT as u32,
            V::FPS as u32,
        )?;

        let total_frames = video.duration().as_secs_f64() * V::FPS as f64;

        for frame_num in 0..total_frames as usize {
            let frame = Frame(frame_num);

            // Render frame
            let scene = video.render_frame(frame, &self.context);
            let pixmap = self.renderer.render_frame(&scene);

            // Encode frame
            self.encoder.write_frame(&pixmap)?;

            // Progress
            if frame_num % 30 == 0 {
                println!("Rendered {}/{} frames", frame_num, total_frames as usize);
            }
        }

        // Finish encoding
        self.encoder.finish()?;

        Ok(())
    }
}
```

### CPU Renderer (tiny-skia)

```rust
// core/fframes-renderer/src/cpu/mod.rs
use crate::Renderer;
use fframes_core::Scene;
use tiny_skia::{Pixmap, PixmapMut, Paint, Color};

pub struct CpuRenderer {
    default_bg: Color,
}

impl CpuRenderer {
    pub fn new(bg_color: Color) -> Self {
        Self {
            default_bg: bg_color,
        }
    }
}

impl Renderer for CpuRenderer {
    fn render_frame(&self, scene: &dyn Scene) -> Pixmap {
        let (width, height) = scene.size();
        let mut pixmap = Pixmap::new(width, height).unwrap();

        // Fill background
        let mut pixmap_mut = pixmap.as_mut();
        pixmap_mut.fill(self.default_bg);

        // Render scene
        scene.render(&mut pixmap_mut);

        pixmap
    }
}

/// CPU-based scene implementation
pub struct CpuScene {
    width: u32,
    height: u32,
    draw_commands: Vec<DrawCommand>,
}

enum DrawCommand {
    FillRect {
        x: f32,
        y: f32,
        width: f32,
        height: f32,
        color: Color,
    },
    StrokeRect {
        x: f32,
        y: f32,
        width: f32,
        height: f32,
        stroke_width: f32,
        color: Color,
    },
    FillCircle {
        cx: f32,
        cy: f32,
        radius: f32,
        color: Color,
    },
    DrawText {
        x: f32,
        y: f32,
        text: String,
        font_size: f32,
        color: Color,
    },
}

impl Scene for CpuScene {
    fn render(&self, canvas: &mut PixmapMut) {
        for command in &self.draw_commands {
            match command {
                DrawCommand::FillRect { x, y, width, height, color } => {
                    let rect = tiny_skia::Rect::from_xywh(*x, *y, *width, *height).unwrap();
                    let mut paint = Paint::default();
                    paint.set_color(*color);
                    canvas.fill_rect(rect, &paint, tiny_skia::Transform::default(), None);
                }
                DrawCommand::FillCircle { cx, cy, radius, color } => {
                    let circle = tiny_skia::PathBuilder::new()
                        .push_circle(*cx, *cy, *radius)
                        .finish()
                        .unwrap();
                    let mut paint = Paint::default();
                    paint.set_color(*color);
                    canvas.fill_path(&circle, &paint, tiny_skia::FillRule::Winding, tiny_skia::Transform::default(), None);
                }
                _ => {}
            }
        }
    }

    fn size(&self) -> (u32, u32) {
        (self.width, self.height)
    }
}
```

### FFmpeg Encoder

```rust
// core/fframes-renderer/src/ffmpeg/mod.rs
use crate::VideoEncoder;
use tiny_skia::Pixmap;
use ffmpeg_sys::*;
use std::path::Path;

pub struct FFmpegEncoder {
    format_ctx: *mut AVFormatContext,
    codec_ctx: *mut AVCodecContext,
    frame: *mut AVFrame,
    packet: *mut AVPacket,
    sws_ctx: *mut SwsContext,
    width: u32,
    height: u32,
}

unsafe impl Send for FFmpegEncoder {}
unsafe impl Sync for FFmpegEncoder {}

impl FFmpegEncoder {
    pub fn new(output_path: &Path) -> Result<Self, EncodeError> {
        unsafe {
            let mut format_ctx = std::ptr::null_mut();

            // Create output context
            avformat_alloc_output_context2(
                &mut format_ctx,
                std::ptr::null(),
                std::ptr::null(),
                output_path.to_str().unwrap().as_ptr() as *const i8,
            );

            if format_ctx.is_null() {
                return Err(EncodeError::FormatError);
            }

            Ok(Self {
                format_ctx,
                codec_ctx: std::ptr::null_mut(),
                frame: std::ptr::null_mut(),
                packet: std::ptr::null_mut(),
                sws_ctx: std::ptr::null_mut(),
                width: 0,
                height: 0,
            })
        }
    }

    fn init_codec(&mut self, width: u32, height: u32, fps: u32) -> Result<(), EncodeError> {
        unsafe {
            // Find encoder
            let codec = avcodec_find_encoder(AVCodecID::AV_CODEC_ID_H264);
            if codec.is_null() {
                return Err(EncodeError::CodecNotFound);
            }

            // Create codec context
            self.codec_ctx = avcodec_alloc_context3(codec);

            // Configure codec
            (*self.codec_ctx).bit_rate = 4_000_000;
            (*self.codec_ctx).width = width as i32;
            (*self.codec_ctx).height = height as i32;
            (*self.codec_ctx).time_base = AVRational { num: 1, den: fps as i32 };
            (*self.codec_ctx).framerate = AVRational { num: fps as i32, den: 1 };
            (*self.codec_ctx).pix_fmt = AVPixelFormat::AV_PIX_FMT_YUV420P;

            // Open codec
            if avcodec_open2(self.codec_ctx, codec, std::ptr::null_mut()) < 0 {
                return Err(EncodeError::CodecOpenError);
            }

            // Create frame
            self.frame = av_frame_alloc();
            (*self.frame).format = AVPixelFormat::AV_PIX_FMT_YUV420P as i32;
            (*self.frame).width = width as i32;
            (*self.frame).height = height as i32;
            av_frame_get_buffer(self.frame, 0);

            // Create packet
            self.packet = av_packet_alloc();

            // Create SWS context for RGB to YUV conversion
            self.sws_ctx = sws_getContext(
                width as i32,
                height as i32,
                AVPixelFormat::AV_PIX_FMT_RGB24,
                width as i32,
                height as i32,
                AVPixelFormat::AV_PIX_FMT_YUV420P,
                SWS_BILINEAR,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                std::ptr::null_mut(),
            );

            self.width = width;
            self.height = height;

            Ok(())
        }
    }
}

impl VideoEncoder for FFmpegEncoder {
    fn init(&mut self, width: u32, height: u32, fps: u32) -> Result<(), EncodeError> {
        self.init_codec(width, height, fps)
    }

    fn write_frame(&mut self, pixmap: &Pixmap) -> Result<(), EncodeError> {
        unsafe {
            // Convert RGB to YUV
            let mut src_data = [pixmap.data_mut().as_mut_ptr()];
            let mut src_linesize = [pixmap.width() as i32 * 4];

            sws_scale(
                self.sws_ctx,
                &src_data as *const _ as *const *const u8,
                &src_linesize as *const _,
                0,
                self.height as i32,
                (*self.frame).data.as_mut_ptr(),
                (*self.frame).linesize.as_mut_ptr(),
            );

            // Send frame to encoder
            avcodec_send_frame(self.codec_ctx, self.frame);

            // Get packet
            while avcodec_receive_packet(self.codec_ctx, self.packet) == 0 {
                av_write_frame(self.format_ctx, self.packet);
                av_packet_unref(self.packet);
            }

            Ok(())
        }
    }

    fn finish(&mut self) -> Result<(), EncodeError> {
        unsafe {
            // Flush encoder
            avcodec_send_frame(self.codec_ctx, std::ptr::null_mut());

            while avcodec_receive_packet(self.codec_ctx, self.packet) == 0 {
                av_write_frame(self.format_ctx, self.packet);
                av_packet_unref(self.packet);
            }

            // Write trailer
            av_write_trailer(self.format_ctx);

            // Cleanup
            avcodec_free_context(&mut self.codec_ctx);
            av_frame_free(&mut self.frame);
            av_packet_free(&mut self.packet);
            sws_freeContext(self.sws_ctx);
            avformat_free_context(self.format_ctx);

            Ok(())
        }
    }
}
```

## WebVTT Integration

```rust
// parsers/webvtt-parser/src/lib.rs
use std::time::Duration;

/// WebVTT subtitle file
pub struct WebVttFile {
    pub cues: Vec<Cue>,
}

/// Single subtitle cue
pub struct Cue {
    pub start: Duration,
    pub end: Duration,
    pub text: String,
    pub styles: Vec<Style>,
}

/// Cue text style
#[derive(Debug, Clone)]
pub enum Style {
    Bold,
    Italic,
    Underline,
    Color(String),
    Position(f32, f32),
}

/// Zero-copy WebVTT parser
pub struct WebVttParser<'a> {
    input: &'a str,
    position: usize,
}

impl<'a> WebVttParser<'a> {
    pub fn new(input: &'a str) -> Self {
        Self { input, position: 0 }
    }

    pub fn parse(mut self) -> Result<WebVttFile, ParseError> {
        // Skip WEBVTT header
        self.skip_header()?;

        let mut cues = Vec::new();

        while !self.is_at_end() {
            if let Some(cue) = self.parse_cue()? {
                cues.push(cue);
            }
        }

        Ok(WebVttFile { cues })
    }

    fn skip_header(&mut self) -> Result<(), ParseError> {
        // Skip "WEBVTT" and any header metadata
        while self.position < self.input.len() {
            if self.peek_line().is_empty() {
                self.skip_line();  // Empty line marks end of header
                self.skip_line();  // Skip the empty line itself
                return Ok(());
            }
            self.skip_line();
        }
        Ok(())
    }

    fn parse_cue(&mut self) -> Result<Option<Cue>, ParseError> {
        self.skip_empty_lines();

        if self.is_at_end() {
            return Ok(None);
        }

        // Parse timing line
        let timing_line = self.read_line();
        let (start, end) = self.parse_timing(&timing_line)?;

        // Parse text
        let mut text = String::new();
        while !self.is_at_end() && !self.peek_line().is_empty() {
            if !text.is_empty() {
                text.push('\n');
            }
            text.push_str(&self.read_line());
        }

        Ok(Some(Cue {
            start,
            end,
            text,
            styles: Vec::new(),
        }))
    }

    fn parse_timing(&self, line: &str) -> Result<(Duration, Duration), ParseError> {
        // Format: 00:00:01.000 --> 00:00:04.000
        let parts: Vec<&str> = line.split(" --> ").collect();
        if parts.len() != 2 {
            return Err(ParseError::InvalidTiming(line.to_string()));
        }

        let start = parse_timestamp(parts[0])?;
        let end = parse_timestamp(parts[1])?;

        Ok((start, end))
    }
}

fn parse_timestamp(s: &str) -> Result<Duration, ParseError> {
    // Format: HH:MM:SS.mmm
    let parts: Vec<&str> = s.split(':').collect();
    if parts.len() != 3 {
        return Err(ParseError::InvalidTimestamp(s.to_string()));
    }

    let hours: u64 = parts[0].parse()?;
    let minutes: u64 = parts[1].parse()?;
    let seconds: f64 = parts[2].parse()?;

    Ok(Duration::from_secs_f64(
        hours as f64 * 3600.0 + minutes as f64 * 60.0 + seconds
    ))
}
```

## Example: Basic Video

```rust
// examples/basic-video/src/main.rs
use fframes_core::{Video, Frame, FFramesContext, Scene};
use fframes_renderer::{VideoCreator, CpuRenderer, FFmpegEncoder};
use std::time::Duration;
use tiny_skia::{Color, Paint, Rect, PixmapMut};

struct BasicVideo {
    title: String,
}

impl Video for BasicVideo {
    const FPS: usize = 30;
    const WIDTH: usize = 1920;
    const HEIGHT: usize = 1080;

    fn duration(&self) -> Duration {
        Duration::from_secs(5)
    }

    fn render_frame(&self, frame: Frame, _ctx: &FFramesContext) -> Scene {
        Box::new(BasicScene {
            frame: frame.0,
            title: self.title.clone(),
        })
    }
}

struct BasicScene {
    frame: usize,
    title: String,
}

impl Scene for BasicScene {
    fn render(&self, canvas: &mut PixmapMut) {
        // Fade in effect
        let alpha = (self.frame as f32 / 30.0).min(1.0);

        // Background gradient
        let mut paint = Paint::default();
        paint.set_color(Color::from_rgba(0, 0, 50, (alpha * 255.0) as u8));
        canvas.fill_rect(
            Rect::from_xywh(0.0, 0.0, 1920.0, 1080.0).unwrap(),
            &paint,
            tiny_skia::Transform::default(),
            None,
        );

        // Title text (would use actual text rendering in production)
        // For now, just a colored rectangle as placeholder
        paint.set_color(Color::from_rgba(255, 255, 255, (alpha * 255.0) as u8));
        canvas.fill_rect(
            Rect::from_xywh(760.0, 490.0, 400.0, 100.0).unwrap(),
            &paint,
            tiny_skia::Transform::default(),
            None,
        );
    }

    fn size(&self) -> (u32, u32) {
        (1920, 1080)
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let video = BasicVideo {
        title: "My Video".to_string(),
    };

    let renderer = CpuRenderer::new(Color::from_rgba(0, 0, 0, 255));
    let encoder = FFmpegEncoder::new(std::path::Path::new("output.mp4"))?;

    let context = FFramesContext {
        assets: AssetManager::new(),
        fonts: FontManager::new(),
        config: RenderConfig::default(),
    };

    let mut creator = VideoCreator::new(renderer, encoder, context);
    creator.create_video(&video, std::path::Path::new("output.mp4"))?;

    Ok(())
}
```

## Testing

```rust
#[cfg(test)]
mod tests {
    use fframes_core::{Frame, Video};
    use crate::BasicVideo;

    #[test]
    fn test_video_duration() {
        let video = BasicVideo { title: "Test".to_string() };
        assert_eq!(video.duration().as_secs(), 5);
    }

    #[test]
    fn test_frame_count() {
        let video = BasicVideo { title: "Test".to_string() };
        let expected_frames = video.duration().as_secs_f64() * BasicVideo::FPS as f64;
        assert_eq!(expected_frames as usize, 150);
    }
}
```

## Related Documents

- [FFrames Core](./fframes-core-exploration.md) - Core architecture
- [FFrames Renderer](./fframes-renderer-exploration.md) - Rendering backends
- [WebVTT Parser](./webvtt-parser-exploration.md) - Subtitle parsing
- [Rust FFmpeg](./rust-ffmpeg-sys-exploration.md) - Encoding
- [Rust Skia](./rust-skia-exploration.md) - Graphics rendering

## Sources

- FFmpeg Documentation: https://ffmpeg.org/documentation.html
- Skia Documentation: https://skia.org/
- WebVTT Spec: https://w3c.github.io/webvtt/
