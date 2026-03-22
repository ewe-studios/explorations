---
name: FFrames Renderer
description: CPU and GPU rendering backends with FFmpeg encoding for video output
type: sub-project
source: /home/darkvoid/Boxxed/@formulas/src.rust/src.fframes/fframes-renderer/
---

# FFFrames Renderer - Video Rendering Backend

## Overview

FFrames Renderer is the **production rendering engine** that converts SVG frame definitions into encoded video files. It supports multiple rendering backends (CPU, Skia GPU, Lyon/WGPU) and uses FFmpeg for professional-grade video encoding.

## Directory Structure

```
fframes-renderer/
├── src/
│   ├── lib.rs                 # Main entry point
│   ├── cpu.rs                 # CPU-based SVG rasterization
│   ├── encoder.rs             # FFmpeg encoder wrapper
│   ├── encoder_frame.rs       # Frame encoding logic
│   ├── concatenator.rs        # Multi-threaded chunk concatenation
│   ├── stream.rs              # AV stream management
│   ├── options.rs             # Encoder configuration
│   └── transcoding.c          # C bindings for transcoding
├── Cargo.toml
└── README.md
```

## Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                    FFrames Renderer                             │
│                                                                 │
│  ┌─────────────────┐     ┌─────────────────┐                   │
│  │  Video Trait    │────▶│  Timeline       │                   │
│  │  (User Code)    │     │  Resolution     │                   │
│  └─────────────────┘     └────────┬────────┘                   │
│                                   │                             │
│                          ┌────────▼────────┐                   │
│                          │  Frame Iterator │                   │
│                          └────────┬────────┘                   │
│                                   │                             │
│  ┌────────────────────────────────┼────────────────────────┐   │
│  │                    Render Backend                      │   │
│  │  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐  │   │
│  │  │ CPU (svgr/   │  │ Skia GPU     │  │ Lyon + WGPU  │  │   │
│  │  │ tiny-skia)   │  │ (Vulkan/     │  │ (POC)        │  │   │
│  │  │              │  │  Metal/D3D)  │  │              │  │   │
│  │  └──────┬───────┘  └──────┬───────┘  └──────┬───────┘  │   │
│  └─────────┼─────────────────┼─────────────────┼──────────┘   │
│            │                 │                 │               │
│            └─────────────────┼─────────────────┘               │
│                              │                                 │
│                     ┌────────▼────────┐                       │
│                     │  RGBA Frames    │                       │
│                     └────────┬────────┘                       │
│                              │                                 │
│                     ┌────────▼────────┐                       │
│                     │  FFmpeg Encoder │                       │
│                     │  (libx264/      │                       │
│                     │   libx265)      │                       │
│                     └────────┬────────┘                       │
│                              │                                 │
│                     ┌────────▼────────┐                       │
│                     │  Chunk Files    │                       │
│                     └────────┬────────┘                       │
│                              │                                 │
│                     ┌────────▼────────┐                       │
│                     │  Concatenator   │                       │
│                     │  (Concat Demux) │                       │
│                     └────────┬────────┘                       │
│                              │                                 │
│                     ┌────────▼────────┐                       │
│                     │  Final MP4/MKV  │                       │
│                     └─────────────────┘                       │
└─────────────────────────────────────────────────────────────────┘
```

## Core Components

### Renderer Trait

```rust
pub trait Renderer: Send + Sync {
    /// Render a single frame to RGBA buffer
    fn render_frame(
        &self,
        svg: &Svgr,
        width: usize,
        height: usize,
    ) -> Vec<u8>;

    /// Get renderer name for debugging
    fn name(&self) -> &'static str;
}
```

### CPU Renderer (svgr/tiny-skia)

```rust
// From fframes-renderer/src/cpu.rs
pub struct CpuRenderer {
    /// Font database for text rendering
    font_db: FontDb,

    /// SVG to tree converter
    svg_converter: SvgConverter,

    /// Rasterization cache
    cache: LRUCache<String, Pixmap>,
}

impl CpuRenderer {
    pub fn new() -> Self {
        let mut font_db = FontDb::new();
        font_db.load_system_fonts();

        CpuRenderer {
            font_db,
            svg_converter: SvgConverter::default(),
            cache: LRUCache::new(1000),
        }
    }
}

impl Renderer for CpuRenderer {
    fn render_frame(
        &self,
        svg: &Svgr,
        width: usize,
        height: usize,
    ) -> Vec<u8> {
        // Convert Svgr to Resvg tree
        let opt = usvg::Options::default();
        let rtree = self.svg_converter.convert(svg, &opt);

        // Check cache
        let svg_hash = hash_svg(&rtree);
        if let Some(cached) = self.cache.get(&svg_hash) {
            return cached.data().to_vec();
        }

        // Rasterize with tiny-skia
        let mut pixmap = Pixmap::new(width as u32, height as u32).unwrap();
        svgr::render(
            &rtree,
            svgr::Transform::default(),
            &mut pixmap,
        );

        // Cache result
        self.cache.insert(svg_hash, pixmap.clone());

        pixmap.data().to_vec()
    }
}
```

### FFmpeg Encoder

```rust
// From fframes-renderer/src/encoder.rs
use ffmpeg_sys_fframes::*;

pub struct Encoder {
    /// Output format context
    oc: *mut AVFormatContext,

    /// Video stream
    video_stream: *mut AVStream,

    /// Audio stream (optional)
    audio_stream: *mut AVStream,

    /// Video codec context
    video_codec: *mut AVCodecContext,

    /// Audio codec context
    audio_codec: *mut AVCodecContext,

    /// Frame counter
    frame_count: usize,

    /// Output path
    output_path: PathBuf,
}

impl Encoder {
    /// Create new encoder
    pub fn new(
        output_path: &str,
        options: &EncoderOptions,
    ) -> Result<Self, EncoderError> {
        unsafe {
            // Allocate output context
            let mut oc = std::ptr::null_mut();
            avformat_alloc_output_context2(
                &mut oc,
                std::ptr::null(),
                options.format,
                output_path,
            );

            // Create video stream
            let codec = avcodec_find_encoder(AV_CODEC_ID_H264)
                .ok_or(EncoderError::CodecNotFound)?;

            let video_stream = avformat_new_stream(oc, codec);
            let video_codec = avcodec_alloc_context3(codec);

            // Configure video codec
            (*video_codec).width = options.width as i32;
            (*video_codec).height = options.height as i32;
            (*video_codec).time_base = AVRational {
                num: 1,
                den: options.fps as i32,
            };
            (*video_codec).pix_fmt = options.pixel_format;
            (*video_codec).bit_rate = options.video_bitrate.unwrap_or(5_000_000);

            // Set codec parameters
            if let Some(params) = options.codec_params {
                for (key, value) in params {
                    av_opt_set(
                        (*video_codec).priv_data,
                        key.as_ptr() as *const _,
                        value.as_ptr() as *const _,
                        0,
                    );
                }
            }

            // Open codec
            avcodec_open2(video_codec, codec, std::ptr::null_mut());

            // Copy codec params to stream
            avcodec_parameters_from_context(
                (*video_stream).codecpar,
                video_codec,
            );

            Ok(Encoder {
                oc,
                video_stream,
                audio_stream: std::ptr::null_mut(),
                video_codec,
                audio_codec: std::ptr::null_mut(),
                frame_count: 0,
                output_path: PathBuf::from(output_path),
            })
        }
    }

    /// Encode a video frame
    pub fn encode_frame(
        &mut self,
        rgba_data: &[u8],
        width: usize,
        height: usize,
    ) -> Result<(), EncoderError> {
        unsafe {
            // Create frame
            let frame = av_frame_alloc();
            (*frame).width = width as i32;
            (*frame).height = height as i32;
            (*frame).format = AVPixelFormat::AV_PIX_FMT_YUV420P;

            // Allocate frame buffer
            av_frame_get_buffer(frame, 0);

            // Convert RGBA to YUV420P
            let mut sws = sws_getContext(
                width as i32,
                height as i32,
                AVPixelFormat::AV_PIX_FMT_RGBA,
                width as i32,
                height as i32,
                AVPixelFormat::AV_PIX_FMT_YUV420P,
                SWS_BILINEAR,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                std::ptr::null_mut(),
            );

            sws_scale(
                sws,
                &rgba_data.as_ptr() as *const _,
                &((width * 4) as i32) as *const _,
                0,
                height as i32,
                &(*frame).data,
                &(*frame).linesize,
            );

            // Set presentation timestamp
            (*frame).pts = self.frame_count as i64;
            self.frame_count += 1;

            // Send frame to encoder
            avcodec_send_frame(self.video_codec, frame);

            // Receive and write packets
            loop {
                let packet = av_packet_alloc();
                match avcodec_receive_packet(self.video_codec, packet) {
                    0 => {
                        // Rescale PTS
                        packet.pts = av_rescale_q_rnd(
                            packet.pts,
                            (*self.video_codec).time_base,
                            (*self.video_stream).time_base,
                            AVRounding::AV_ROUND_NEAR_INF | AVRounding::AV_ROUND_PASS_MINMAX,
                        );
                        packet.dts = packet.pts;
                        packet.duration = (*self.video_codec).time_base.den as i32 / (*self.video_codec).time_base.num as i32;
                        packet.stream_index = (*self.video_stream).index;

                        // Write packet
                        av_write_frame(self.oc, packet);
                    }
                    AVERROR_EOF | AVERROR(EAGAIN) => break,
                    e => return Err(EncoderError::EncodingFailed(e)),
                }
            }

            av_frame_free(&mut frame);
        }

        Ok(())
    }

    /// Finish encoding and write trailer
    pub fn finish(mut self) -> Result<(), EncoderError> {
        unsafe {
            // Flush encoder
            avcodec_send_frame(self.video_codec, std::ptr::null_mut());

            loop {
                let packet = av_packet_alloc();
                match avcodec_receive_packet(self.video_codec, packet) {
                    0 => {
                        packet.pts = av_rescale_q_rnd(
                            packet.pts,
                            (*self.video_codec).time_base,
                            (*self.video_stream).time_base,
                            AVRounding::AV_ROUND_NEAR_INF | AVRounding::AV_ROUND_PASS_MINMAX,
                        );
                        packet.dts = packet.pts;
                        packet.stream_index = (*self.video_stream).index;
                        av_write_frame(self.oc, packet);
                    }
                    _ => break,
                }
            }

            // Write trailer
            av_write_trailer(self.oc);

            // Free resources
            avcodec_free_context(&mut self.video_codec);
            avformat_free_context(self.oc);
        }

        Ok(())
    }
}
```

### Encoder Options

```rust
pub struct EncoderOptions {
    /// Preferred video codec (e.g., "libx264", "libx265")
    pub preferred_video_codec: Option<&'static str>,

    /// Preferred audio codec (e.g., "aac", "mp3")
    pub preferred_audio_codec: Option<&'static str>,

    /// Pixel format (default: YUV420P)
    pub pixel_format: AVPixelFormat,

    /// Sample format (default: FLTP)
    pub sample_format: AVSampleFormat,

    /// Video bitrate in bits/second
    pub video_bitrate: Option<i64>,

    /// Audio bitrate in bits/second
    pub audio_bitrate: Option<i64>,

    /// Additional codec parameters
    pub codec_params: Option<&'static [(&'static str, &'static str)]>,

    /// GOP size (Group of Pictures)
    pub gop_size: i32,

    /// Audio sample rate
    pub sample_rate: usize,

    /// Video width
    pub width: usize,

    /// Video height
    pub height: usize,

    /// Frames per second
    pub fps: usize,
}

impl Default for EncoderOptions {
    fn default() -> Self {
        Self {
            preferred_video_codec: Some("libx264"),
            preferred_audio_codec: Some("aac"),
            pixel_format: AVPixelFormat::AV_PIX_FMT_YUV420P,
            sample_format: AVSampleFormat::AV_SAMPLE_FMT_FLTP,
            video_bitrate: Some(5_000_000), // 5 Mbps
            audio_bitrate: Some(128_000),   // 128 kbps
            codec_params: Some(&[
                ("crf", "23"),
                ("preset", "ultrafast"),
            ]),
            gop_size: 24, // 24 frames between keyframes
            sample_rate: 44100,
            width: 1920,
            height: 1080,
            fps: 60,
        }
    }
}
```

## Multi-threaded Rendering

### Chunk-Based Rendering

```rust
// From fframes-renderer/src/lib.rs
pub struct RenderChunk {
    pub start_frame: usize,
    pub end_frame: usize,
    pub output_path: String,
}

pub fn split_video_chunks(
    total_frames: usize,
    concurrency: usize,
    gop_size: i32,
) -> Vec<RenderChunk> {
    let frames_per_chunk = total_frames / concurrency;
    let mut chunks = Vec::new();

    for i in 0..concurrency {
        let start = i * frames_per_chunk;
        let mut end = if i == concurrency - 1 {
            total_frames
        } else {
            (i + 1) * frames_per_chunk
        };

        // Align to GOP boundaries
        end = (end / gop_size as usize) * gop_size as usize;

        chunks.push(RenderChunk {
            start_frame: start,
            end_frame,
            output_path: format!("/tmp/chunk_{}.mp4", i),
        });
    }

    chunks
}

pub fn render_chunks_parallel<V: Video>(
    video: &V,
    chunks: &[RenderChunk],
    options: &EncoderOptions,
    concurrency: usize,
) -> Result<(), RenderError> {
    use rayon::prelude::*;

    chunks
        .par_iter()
        .enumerate()
        .map(|(thread_id, chunk)| {
            // Each thread creates its own encoder
            let mut encoder = Encoder::new(&chunk.output_path, options)?;

            // Create rendering context
            let mut ctx = FFramesContext::new();

            for frame_idx in chunk.start_frame..chunk.end_frame {
                // Calculate frame time
                let current_second = frame_idx as f32 / V::FPS as f32;

                // Create frame context
                let frame = Frame {
                    index: frame_idx,
                    fps: V::FPS,
                    current_second,
                    progress: frame_idx as f32 / total_frames as f32,
                    scene_info: None,
                };

                // Render frame
                let svg = video.render_frame(frame, &mut ctx);

                // Rasterize
                let rgba = cpu_renderer.render_frame(&svg, V::WIDTH, V::HEIGHT);

                // Encode
                encoder.encode_frame(&rgba, V::WIDTH, V::HEIGHT)?;
            }

            encoder.finish()?;
            Ok(())
        })
        .collect::<Result<Vec<()>, RenderError>>()?;

    Ok(())
}
```

### Chunk Concatenation

```rust
// From fframes-renderer/src/concatenator.rs
pub fn concat_video_files(
    chunk_files: &[String],
    output_path: &str,
) -> Result<(), ConcatError> {
    // Create concat demuxer input file
    let concat_file = "/tmp/concat_list.txt";
    let mut file = File::create(concat_file)?;

    for chunk in chunk_files {
        writeln!(file, "file '{}'", chunk)?;
    }

    // Use ffmpeg CLI for concat demuxing
    let output = Command::new("ffmpeg")
        .args([
            "-f", "concat",
            "-safe", "0",
            "-i", concat_file,
            "-c", "copy",
            "-y",
            output_path,
        ])
        .output()?;

    if !output.status.success() {
        return Err(ConcatError::FFmpegError(
            String::from_utf8_lossy(&output.stderr).to_string()
        ));
    }

    Ok(())
}

pub fn concat_video_files_with_audio(
    chunk_files: &[String],
    output_path: &str,
    audio_map: &AudioMap,
    encoder_options: &EncoderOptions,
) -> Result<(), ConcatError> {
    // More complex: re-encode with audio sync
    // Uses FFmpeg filter complex
    let filter_complex = format!(
        "[0:v] [1:a] concat=n={}:v=1:a=1 [v] [a]",
        chunk_files.len()
    );

    let mut args = vec![
        "-f".to_string(), "concat".to_string(),
        "-safe".to_string(), "0".to_string(),
    ];

    // Add inputs
    for chunk in chunk_files {
        args.push("-i".to_string());
        args.push(chunk.clone());
    }

    // Add audio input
    if let Some(audio_path) = audio_map.get_primary_audio() {
        args.push("-i".to_string());
        args.push(audio_path.to_string());
    }

    args.extend([
        "-filter_complex".to_string(), filter_complex,
        "-map".to_string(), "[v]".to_string(),
        "-map".to_string(), "[a]".to_string(),
        "-c:v".to_string(), "libx264".to_string(),
        "-c:a".to_string(), "aac".to_string(),
        "-y".to_string(),
        output_path.to_string(),
    ]);

    let output = Command::new("ffmpeg")
        .args(&args)
        .output()?;

    if !output.status.success() {
        return Err(ConcatError::FFmpegError(
            String::from_utf8_lossy(&output.stderr).to_string()
        ));
    }

    Ok(())
}
```

## Skia GPU Renderer

```rust
// From fframes-skia-renderer/src/lib.rs
use skia_safe::{
    gpu::{
        vulkan::{BackendContext, GetProcOf},
        DirectContext,
    },
    canvas::Canvas,
    surface::Surface,
    ColorType, PixelGeometry, SurfaceProps,
};

pub struct SkiaGpuRenderer {
    /// GPU backend context
    gpu_context: DirectContext,

    /// Surface for rendering
    surface: Surface,

    /// Canvas for drawing
    canvas: Canvas,
}

impl SkiaGpuRenderer {
    pub fn new_vulkan(
        width: usize,
        height: usize,
        get_proc: impl GetProcOf,
    ) -> Result<Self, RendererError> {
        // Create Vulkan backend
        let backend = BackendContext::new(get_proc);

        // Create direct context
        let mut gpu_context = DirectContext::new_vulkan(&backend, None)
            .ok_or(RendererError::VulkanInitFailed)?;

        // Create surface
        let mut surface = Surface::new_renderable(
            &mut gpu_context,
            width as i32,
            height as i32,
            None,
            ColorType::RGBA8888,
            None,
            &SurfaceProps::new(PixelGeometry::RgbH),
            false,
        )
        .ok_or(RendererError::SurfaceCreationFailed)?;

        let canvas = surface.canvas().clone();

        Ok(SkiaGpuRenderer {
            gpu_context,
            surface,
            canvas,
        })
    }

    pub fn render_svg(&mut self, svg: &Svgr) -> Vec<u8> {
        // Clear canvas
        self.canvas.clear(Color::TRANSPARENT);

        // Convert SVG to Skia picture
        let picture = svg.to_skia_picture();

        // Draw picture
        self.canvas.draw_picture(&picture, None, None);

        // Flush and read pixels
        self.gpu_context.flush_and_submit();

        let info = ImageInfo::new(
            self.surface.width(),
            self.surface.height(),
            ColorType::RGBA8888,
            AlphaType::Premul,
            None,
        );

        let mut pixels = vec![0u8; info.min_row_bytes() * info.height() as usize];
        self.surface.read_pixels(&info, &mut pixels, info.min_row_bytes(), 0, 0);

        pixels
    }
}
```

## Audio Encoding

```rust
// From fframes-renderer/src/encoder.rs (audio methods)
impl Encoder {
    /// Initialize audio stream
    pub fn init_audio_stream(
        &mut self,
        sample_rate: usize,
        channels: u16,
    ) -> Result<(), EncoderError> {
        unsafe {
            let codec = avcodec_find_encoder(AV_CODEC_ID_AAC)
                .ok_or(EncoderError::CodecNotFound)?;

            self.audio_stream = avformat_new_stream(self.oc, codec);
            self.audio_codec = avcodec_alloc_context3(codec);

            (*self.audio_codec).sample_rate = sample_rate as i32;
            (*self.audio_codec).channels = channels as i32;
            (*self.audio_codec).sample_fmt = AVSampleFormat::AV_SAMPLE_FMT_FLTP;
            (*self.audio_codec).time_base = AVRational {
                num: 1,
                den: sample_rate as i32,
            };

            avcodec_open2(self.audio_codec, codec, std::ptr::null_mut());
            avcodec_parameters_from_context(
                (*self.audio_stream).codecpar,
                self.audio_codec,
            );
        }

        Ok(())
    }

    /// Encode audio samples
    pub fn encode_audio(
        &mut self,
        samples: &[f32],
        sample_rate: usize,
    ) -> Result<(), EncoderError> {
        unsafe {
            // Create audio frame
            let frame = av_frame_alloc();
            (*frame).nb_samples = samples.len() as i32;
            (*frame).format = AVSampleFormat::AV_SAMPLE_FMT_FLTP;
            (*frame).sample_rate = sample_rate as i32;

            av_frame_get_buffer(frame, 0);

            // Copy samples to frame
            let data_ptr = (*frame).data[0] as *mut f32;
            ptr::copy_nonoverlapping(
                samples.as_ptr(),
                data_ptr,
                samples.len(),
            );

            // Send to encoder
            avcodec_send_frame(self.audio_codec, frame);

            // Receive packets
            loop {
                let packet = av_packet_alloc();
                match avcodec_receive_packet(self.audio_codec, packet) {
                    0 => {
                        packet.stream_index = (*self.audio_stream).index;
                        av_write_frame(self.oc, packet);
                    }
                    _ => break,
                }
            }

            av_frame_free(&mut frame);
        }

        Ok(())
    }
}
```

## Performance Considerations

### Memory Management

```rust
// Frame buffer pooling
pub struct FrameBufferPool {
    buffers: Vec<Vec<u8>>,
    width: usize,
    height: usize,
}

impl FrameBufferPool {
    pub fn new(width: usize, height: usize, initial_size: usize) -> Self {
        let buffer_size = width * height * 4; // RGBA
        let buffers = (0..initial_size)
            .map(|_| vec![0u8; buffer_size])
            .collect();

        FrameBufferPool {
            buffers,
            width,
            height,
        }
    }

    pub fn acquire(&mut self) -> Vec<u8> {
        self.buffers.pop().unwrap_or_else(|| {
            vec![0u8; self.width * self.height * 4]
        })
    }

    pub fn release(&mut self, buffer: Vec<u8>) {
        if self.buffers.len() < 10 {
            self.buffers.push(buffer);
        }
    }
}
```

### GOP Alignment

```rust
/// Ensure frames are aligned to GOP boundaries for efficient seeking
pub fn align_to_gop(frame_count: usize, gop_size: i32) -> usize {
    let gop = gop_size as usize;
    ((frame_count + gop - 1) / gop) * gop
}

/// Minimum chunk size for parallel rendering
pub fn min_chunk_size(gop_size: i32) -> usize {
    (gop_size * 2) as usize // At least 2 GOPs per chunk
}
```

## Related Documents

- [FFrames Core](./fframes-core-exploration.md) - Core video trait and abstractions
- [FFrames Editor](./fframes-editor-exploration.md) - Web-based video editor

## Sources

- Source: `/home/darkvoid/Boxxed/@formulas/src.rust/src.fframes/fframes-renderer/`
- FFrames Main Exploration: `../../fframes/exploration.md`
