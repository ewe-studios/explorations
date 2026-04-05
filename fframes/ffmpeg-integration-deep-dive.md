---
location: /home/darkvoid/Boxxed/@formulas/src.rust/src.fframes/
explored_at: 2026-04-02
focus: FFmpeg Integration Strategy
---

# FFrames FFmpeg Integration - Deep Dive

## Executive Summary

**FFrames uses direct library linking via FFI bindings, NOT command-line execution.**

The framework links directly to FFmpeg libraries at compile-time using `ffmpeg-sys-next` (a maintained fork of ffmpeg-sys), which generates FFI bindings via `bindgen`. This provides:

- **Zero-copy frame handling** - Direct memory access to AVFrame/AVPacket structures
- **Fine-grained control** - Per-frame encoding control, custom codec parameters
- **Better error handling** - Catch errors at the API level, not exit codes
- **Streaming integration** - Seamless integration with Rust's async/runtime model
- **Performance** - No process spawn overhead, no pipe serialization

---

## Architecture Overview

```
┌─────────────────────────────────────────────────────────────────┐
│                    FFrames User Code                            │
│                 (impl Video trait, Scene trait)                 │
└─────────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│                   fframes-renderer                              │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────────────────┐ │
│  │ cpu.rs      │  │ encoder.rs  │  │ concatenator.rs         │ │
│  │ (SVG→RGBA)  │  │ (libav)     │  │ (chunk merge)           │ │
│  └─────────────┘  └─────────────┘  └─────────────────────────┘ │
└─────────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│                  ffmpeg-sys-next (FFI)                          │
│  ┌──────────────┐ ┌──────────────┐ ┌──────────────┐            │
│  │ libavcodec   │ │ libavformat  │ │ libavutil    │            │
│  │ (encoding)   │ │ (muxing)     │ │ (utilities)  │            │
│  └──────────────┘ └──────────────┘ └──────────────┘            │
│  ┌──────────────┐ ┌──────────────┐                              │
│  │ libswscale   │ │ libswresample│                              │
│  │ (conversion) │ │ (audio)      │                              │
│  └──────────────┘ └──────────────┘                              │
└─────────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│              System FFmpeg Libraries (.so/.a/.dylib)            │
│         /usr/lib/x86_64-linux-gnu/libavcodec.so.59              │
│         /usr/lib/x86_64-linux-gnu/libavformat.so.59             │
│         ...                                                      │
└─────────────────────────────────────────────────────────────────┘
```

---

## Approach 1: Direct Library Linking (FFrames Uses This)

### How It Works

FFrames uses the `ffmpeg-sys-next` crate which:

1. **Builds FFI bindings at compile-time** using `bindgen`
2. **Links to system FFmpeg libraries** via `pkg-config`
3. **Generates Rust bindings** that directly call C functions

### Cargo.toml Configuration

```toml
[dependencies]
ffmpeg-sys-fframes = { package = "ffmpeg-sys-next", version = "8.0.1", features = [
    "build",
    "static",
] }
```

### Key Features

| Feature | Description |
|---------|-------------|
| `build` | Compile FFmpeg from source if system libs not found |
| `static` | Use static linking instead of dynamic |
| `build-lib-x264` | Include H.264 encoder |
| `build-lib-x265` | Include H.265/HEVC encoder |
| `build-license-gpl` | Accept GPL license for x264 |

### Build Script (build.rs)

The build script does several things:

```rust
// Simplified from ffmpeg-sys-next/build.rs

fn main() {
    // 1. Detect FFmpeg via pkg-config
    pkg_config::Config::new()
        .atleast_version("4.0")
        .probe("libavcodec")
        .unwrap();
    
    // 2. Generate FFI bindings via bindgen
    let bindings = bindgen::Builder::default()
        .header("wrapper.h")
        .clang_arg("-I/usr/include/ffmpeg")
        .generate()
        .unwrap();
    
    // 3. Write bindings to OUT_DIR
    bindings.write_to_file(out_path.join("bindings.rs"));
}
```

### Core FFI Usage Pattern

```rust
use ffmpeg_sys_fframes::*;
use std::ffi::CString;

// 1. Create output context
unsafe {
    let mut oc: *mut AVFormatContext = std::ptr::null_mut();
    let c_filename = CString::new("output.mp4").unwrap();
    
    avformat_alloc_output_context2(
        &mut oc,
        std::ptr::null_mut(),
        std::ptr::null(),
        c_filename.as_ptr(),
    );
    
    // 2. Create video stream
    let video_stream = avformat_new_stream(oc, std::ptr::null());
    
    // 3. Find and open codec
    let codec = avcodec_find_encoder(AV_CODEC_ID_H264);
    let codec_context = avcodec_alloc_context3(codec);
    
    // 4. Configure codec
    (*codec_context).width = 1920;
    (*codec_context).height = 1080;
    (*codec_context).time_base = AVRational { num: 1, den: 60 };
    (*codec_context).pix_fmt = AVPixelFormat::AV_PIX_FMT_YUV420P;
    
    // 5. Set codec-specific options
    av_opt_set(
        (*codec_context).priv_data,
        b"preset\0".as_ptr() as *const _,
        b"ultrafast\0".as_ptr() as *const _,
        0,
    );
    av_opt_set(
        (*codec_context).priv_data,
        b"crf\0".as_ptr() as *const _,
        b"23\0".as_ptr() as *const _,
        0,
    );
    
    avcodec_open2(codec_context, codec, std::ptr::null_mut());
}
```

### FFrames Encoder Wrapper

From `fframes-renderer/src/encoder.rs`:

```rust
pub struct Encoder {
    pub(crate) video_stream: stream::Stream,
    pub(crate) audio_stream: Option<stream::Stream>,
    pub(crate) b_frames_count: i32,
    pub(crate) oc: *mut AVFormatContext,
}

impl Encoder {
    pub unsafe fn with_output<T, F: FnMut(&mut Encoder) -> RenderEncodingResult<T>>(
        width: i32,
        height: i32,
        fps: i32,
        filename: &str,
        encoder_options: &EncoderOptions,
        logger: &Arc<dyn FFramesLogger>,
        inner_fn: &mut F,
    ) -> RenderEncodingResult<T> {
        // 1. Allocate output context
        avformat_alloc_output_context2(&mut oc, ...);
        
        // 2. Create video stream with options
        let video_stream = stream::Stream::make_video(
            width, height, fps, oc, encoder_options
        )?;
        
        // 3. Open file for writing
        avio_open(&mut (*oc).pb, c_filename.as_ptr(), 2);
        avformat_write_header(oc, std::ptr::null_mut());
        
        // 4. Run user-provided encoding function
        let res = inner_fn(&mut encoder);
        
        // 5. Flush and cleanup
        avcodec_send_frame(video_stream.enc, std::ptr::null_mut());
        av_write_trailer(oc);
        avio_closep(&mut (*oc).pb);
        avformat_free_context(oc);
        
        res
    }
}
```

### Frame Encoding Flow

```rust
// From fframes-renderer/src/encoder.rs

pub unsafe fn send_frame(
    &mut self,
    stream: &stream::Stream,
    frame: &EncoderFrame,
) -> RenderEncodingResult<()> {
    self.send_customizable_frame_packet(stream, frame, |packet| {
        // Rescale timestamps to stream timebase
        av_packet_rescale_ts(packet, (*stream.enc).time_base, (*stream.st).time_base);
        
        (*packet).stream_index = (*stream.st).index;
        
        // Write interleaved (audio/video sync)
        av_interleaved_write_frame(oc, packet)
    })
}

// Inside send_customizable_frame_packet:
let status = avcodec_send_frame(stream.enc, *frame);

// Signal end of stream
if status < 0 {
    return Err(CantWriteFrame(...));
}

let packet = av_packet_alloc();
while status >= 0 {
    status = avcodec_receive_packet(stream.enc, packet);
    
    match status {
        AVERROR_EOF => break,           // Encoding complete
        AVERROR(EAGAIN) => break,       // Need more frames (B-frames)
        _ => customize_frame(packet),   // Write packet
    }
}
```

---

## Approach 2: Command-Line Execution (NOT Used by FFrames)

### How It Would Work

```rust
use std::process::Command;

// Spawn ffmpeg as subprocess
let output = Command::new("ffmpeg")
    .args([
        "-f", "rawvideo",
        "-pix_fmt", "rgba",
        "-s", "1920x1080",
        "-r", "60",
        "-i", "pipe:0",  // Read from stdin
        "-c:v", "libx264",
        "-crf", "23",
        "-preset", "ultrafast",
        "output.mp4",
    ])
    .stdin(Stdio::piped())
    .spawn()?;

// Write raw frames to stdin
let mut stdin = output.stdin.unwrap();
for frame in frames {
    stdin.write_all(&frame.rgba_data)?;
}
```

### Why FFrames Does NOT Use This

| Issue | Description |
|-------|-------------|
| **Pipe overhead** | Serializing frames through pipes is slow |
| **No fine control** | Can't control B-frames, GOP structure mid-stream |
| **Error handling** | Only exit codes, no structured errors |
| **Process spawn** | Spawning for each chunk is expensive |
| **Audio sync** | Harder to manage A/V synchronization |
| **Custom muxing** | Can't easily do custom container operations |

### When Command-Line IS Appropriate

- Quick scripts/prototypes
- When you need a specific FFmpeg filter not available via libavfilter API
- When static linking is too large for your deployment
- When you want to leverage FFmpeg's automatic filter graphs

---

## Detailed Comparison: Linking vs CLI

| Aspect | Direct Linking | CLI Execution |
|--------|----------------|---------------|
| **Performance** | Zero-copy, no serialization | Pipe overhead, copying |
| **Control** | Per-frame, per-packet | Limited to CLI options |
| **Error Handling** | Structured errors | Exit codes, stderr parsing |
| **Integration** | Native Rust types | Process management |
| **Binary Size** | +50-200MB (static) | External dependency |
| **Deployment** | Self-contained | Needs ffmpeg installed |
| **Flexibility** | Full libav API access | CLI-exposed features only |
| **Debugging** | Stack traces, symbols | Limited visibility |

---

## Can You Compile FFmpeg Directly Into Rust?

**Yes, with the `build` feature.** FFrames supports two modes:

### Option 1: System Libraries (Default)

```toml
[dependencies]
ffmpeg-sys-next = "8.0.1"  # No features = use system libs
```

**Pros:**
- Smaller Rust binary
- Share system FFmpeg updates
- Faster compile times

**Cons:**
- Requires FFmpeg installed
- Version mismatches possible

**System dependencies (Debian/Ubuntu):**
```bash
apt install \
    libavcodec-dev \
    libavformat-dev \
    libavutil-dev \
    libswscale-dev \
    libswresample-dev \
    libavfilter-dev \
    pkg-config \
    clang
```

### Option 2: Static Build from Source

```toml
[dependencies]
ffmpeg-sys-next = { version = "8.0.1", features = [
    "build",
    "static",
    "build-lib-x264",
    "build-lib-x265",
    "build-license-gpl",
]}
```

**Pros:**
- Self-contained binary
- No external dependencies
- Guaranteed version compatibility

**Cons:**
- +50-200MB binary size
- Longer compile times (FFmpeg builds in ~5-15 min)
- GPL licensing implications

**Build output:**
```
target/debug/build/ffmpeg-sys-next-xxxx/out/
├── libavcodec.a
├── libavformat.a
├── libavutil.a
├── libswscale.a
├── libswresample.a
└── ...
```

### Option 3: Hybrid (Dynamic with Fallback)

```toml
[dependencies]
ffmpeg-sys-next = { version = "8.0.1", features = ["build"] }
```

This tries system libs first, builds from source if not found.

---

## Integration Guide for Your Use Cases

### Use Case 1: Simple Video Generation (Like FFrames)

```rust
// Cargo.toml
[dependencies]
ffmpeg-sys-next = { version = "8.0.1", features = ["build", "static"] }
svgr = "0.44.1"  # For SVG rendering
tiny-skia = "0.11"

// main.rs
use ffmpeg_sys_next::*;
use std::ffi::CString;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    unsafe {
        // 1. Setup output
        let mut oc: *mut AVFormatContext = std::ptr::null_mut();
        let filename = CString::new("output.mp4")?;
        
        avformat_alloc_output_context2(
            &mut oc,
            std::ptr::null_mut(),
            std::ptr::null(),
            filename.as_ptr(),
        );
        
        // 2. Create H.264 encoder
        let codec = avcodec_find_encoder(AV_CODEC_ID_H264)?;
        let ctx = avcodec_alloc_context3(codec);
        
        (*ctx).width = 1920;
        (*ctx).height = 1080;
        (*ctx).time_base = AVRational { num: 1, den: 60 };
        (*ctx).framerate = AVRational { num: 60, den: 1 };
        (*ctx).pix_fmt = AVPixelFormat::AV_PIX_FMT_YUV420P;
        (*ctx).bit_rate = 5_000_000;
        
        avcodec_open2(ctx, codec, std::ptr::null_mut());
        
        // 3. Create stream
        let stream = avformat_new_stream(oc, std::ptr::null());
        avcodec_parameters_to_stream(stream, ctx);
        
        // 4. Write header
        avio_open(&mut (*oc).pb, filename.as_ptr(), 2);
        avformat_write_header(oc, std::ptr::null_mut());
        
        // 5. Encode frames
        for frame_num in 0..1800 {  // 30 seconds at 60fps
            // Render your frame (SVG→RGBA→YUV)
            let rgba_data = render_your_frame(frame_num);
            
            // Convert RGBA→YUV420P
            let mut yuv_frame = av_frame_alloc();
            (*yuv_frame).width = 1920;
            (*yuv_frame).height = 1080;
            (*yuv_frame).format = AVPixelFormat::AV_PIX_FMT_YUV420P;
            av_frame_get_buffer(yuv_frame, 32);
            
            // Fill YUV data from RGBA
            convert_rgba_to_yuv420p(
                &rgba_data,
                &mut (*yuv_frame).data,
                &mut (*yuv_frame).linesize,
                1920, 1080,
            );
            
            (*yuv_frame).pts = frame_num;
            
            // Send to encoder
            avcodec_send_frame(ctx, yuv_frame);
            
            // Receive and write packets
            let packet = av_packet_alloc();
            while avcodec_receive_packet(ctx, packet) == 0 {
                av_packet_rescale_ts(packet, (*ctx).time_base, (*stream).time_base);
                (*packet).stream_index = (*stream).index;
                av_interleaved_write_frame(oc, packet);
                av_packet_unref(packet);
            }
            
            av_frame_free(&mut yuv_frame);
        }
        
        // 6. Flush encoder
        avcodec_send_frame(ctx, std::ptr::null_mut());
        let packet = av_packet_alloc();
        while avcodec_receive_packet(ctx, packet) == 0 {
            av_interleaved_write_frame(oc, packet);
            av_packet_unref(packet);
        }
        
        // 7. Cleanup
        av_write_trailer(oc);
        avio_closep(&mut (*oc).pb);
        avformat_free_context(oc);
        avcodec_free_context(&mut ctx);
    }
    
    Ok(())
}

fn convert_rgba_to_yuv420p(
    rgba: &[u8],
    yuv_data: &mut [*mut u8; 8],
    yuv_linesize: &mut [i32; 8],
    width: usize,
    height: usize,
) {
    // Simple RGB→YUV conversion
    // For production, use libswscale (sws_scale)
    for y in 0..height {
        for x in 0..width {
            let rgba_offset = (y * width + x) * 4;
            let r = rgba[rgba_offset] as f32 / 255.0;
            let g = rgba[rgba_offset + 1] as f32 / 255.0;
            let b = rgba[rgba_offset + 2] as f32 / 255.0;
            
            // YUV420P: Y plane
            let y_val = (0.299 * r + 0.587 * g + 0.114 * b) * 255.0;
            yuv_data[0][y * yuv_linesize[0] as usize + x] = y_val as u8;
            
            // U/V planes (subsampled)
            if y % 2 == 0 && x % 2 == 0 {
                let u_val = ((-0.169 * r - 0.331 * g + 0.5 * b + 0.5) * 255.0) as u8;
                let v_val = ((0.5 * r - 0.419 * g - 0.081 * b + 0.5) * 255.0) as u8;
                let uv_x = x / 2;
                let uv_y = y / 2;
                yuv_data[1][uv_y * yuv_linesize[1] as usize + uv_x] = u_val;
                yuv_data[2][uv_y * yuv_linesize[2] as usize + uv_x] = v_val;
            }
        }
    }
}
```

### Use Case 2: GPU-Accelerated Rendering

```toml
[dependencies]
ffmpeg-sys-next = { version = "8.0.1", features = ["build-nvenc"] }
skia-safe = { version = "0.80", features = ["gpu"] }
```

```rust
use ffmpeg_sys_next::*;

// Use NVENC for hardware encoding
unsafe {
    let codec = avcodec_find_encoder_by_name(b"h264_nvenc\0".as_ptr() as *const _);
    
    // Configure for GPU
    (*ctx).pix_fmt = AVPixelFormat::AV_PIX_FMT_CUDA;
    av_opt_set(
        (*ctx).priv_data,
        b"preset\0".as_ptr() as *const _,
        b"p1\0".as_ptr() as *const _,  // Fastest
        0,
    );
    av_opt_set(
        (*ctx).priv_data,
        b"tuning\0".as_ptr() as *const _,
        b"ultra_low_latency\0".as_ptr() as *const _,
        0,
    );
}
```

### Use Case 3: Audio-Video Muxing

```rust
// From fframes-renderer/src/stream.rs

pub unsafe fn make_audio(
    sample_rate: i32,
    oc: *mut AVFormatContext,
    options: &EncoderOptions,
) -> Result<Stream, RenderEncodingError> {
    let codec = avcodec_find_encoder(AV_CODEC_ID_AAC)?;
    let ctx = avcodec_alloc_context3(codec);
    
    (*ctx).sample_rate = sample_rate;
    (*ctx).sample_fmt = AVSampleFormat::AV_SAMPLE_FMT_FLTP;
    (*ctx).channel_layout = AV_CH_LAYOUT_STEREO;
    (*ctx).channels = 2;
    (*ctx).bit_rate = options.audio_bitrate.unwrap_or(192_000);
    (*ctx).time_base = AVRational { num: 1, den: sample_rate };
    
    avcodec_open2(ctx, codec, std::ptr::null_mut());
    
    let stream = avformat_new_stream(oc, std::ptr::null());
    avcodec_parameters_to_stream(stream, ctx);
    
    Ok(Stream { st: stream, enc: ctx, variant: StreamVariant::Audio })
}

// Audio frame encoding
fn fill_audio_frame(
    frame: &mut EncoderFrame,
    pts: i64,
    audio_data: Vec<f32>,  // FLTP format
) {
    unsafe {
        (*frame.frame).pts = pts;
        
        // FLTP = Float Planar: separate buffer per channel
        let channel_size = audio_data.len() / 2;
        
        for ch in 0..2 {
            let data = (*frame.frame).data[ch] as *mut f32;
            for i in 0..channel_size {
                *data.add(i) = audio_data[ch * channel_size + i];
            }
        }
    }
}
```

---

## Best Practices for Your Integration

### 1. Choose Linking Strategy Based on Deployment

| Deployment | Recommended Approach |
|------------|---------------------|
| Desktop app | Static linking (self-contained) |
| Server/container | System libs (smaller image) |
| CLI tool | Static with fallback |
| Embedded | Static, minimal codecs |

### 2. Use Safe Wrappers

Don't use raw FFI directly. Create safe abstractions:

```rust
pub struct VideoEncoder {
    codec_context: *mut AVCodecContext,
    format_context: *mut AVFormatContext,
    video_stream: *mut AVStream,
}

impl VideoEncoder {
    pub fn new(width: i32, height: i32, fps: i32) -> Result<Self, EncoderError> {
        // Validate inputs
        if width <= 0 || height <= 0 || fps <= 0 {
            return Err(EncoderError::InvalidDimensions);
        }
        
        unsafe {
            // ... allocation with error handling
        }
    }
    
    pub fn encode(&mut self, rgba: &[u8], pts: i64) -> Result<(), EncoderError> {
        // Validate frame data
        if rgba.len() != (width * height * 4) as usize {
            return Err(EncoderError::InvalidFrameSize);
        }
        
        // ... encoding
    }
}

impl Drop for VideoEncoder {
    fn drop(&mut self) {
        unsafe {
            avcodec_free_context(&mut self.codec_context);
            avformat_free_context(self.format_context);
        }
    }
}
```

### 3. Handle B-Frames Correctly

```rust
// From fframes-renderer/src/encoder.rs

match status {
    AVERROR_EOF => break,           // Done
    FFMPEG_AVERROR(EAGAIN) => {
        // Encoder is buffering frames for B-frame reordering
        // This is NORMAL - don't treat as error
        self.b_frames_count += 1;
        break;
    }
    _ => {
        // Actually write the packet
        customize_frame(packet)
    }
}
```

### 4. Use Proper Timestamp Handling

```rust
// Always rescale timestamps to stream timebase
av_packet_rescale_ts(
    packet,
    (*codec_context).time_base,      // Codec timebase (e.g., 1/60)
    (*stream).time_base,             // Stream timebase (e.g., 1/90000)
);
```

### 5. Memory Management

```rust
// Always free in reverse order of allocation
impl Drop for Encoder {
    fn drop(&mut self) {
        unsafe {
            // 1. Free packets
            av_packet_free(&mut self.packet);
            
            // 2. Free frames
            av_frame_free(&mut self.frame);
            
            // 3. Free codec context
            avcodec_free_context(&mut self.codec_context);
            
            // 4. Close IO
            avio_closep(&mut (*self.format_context).pb);
            
            // 5. Free format context
            avformat_free_context(self.format_context);
        }
    }
}
```

---

## Performance Optimization

### FFrames' Approach

1. **Multi-threaded chunk rendering:**
   - Split video into GOP-aligned chunks
   - Each thread renders its own chunk
   - Concatenate at the end

2. **SVG caching:**
   ```rust
   let mut svgr_cache = SvgrCache::new(self.cache_capacity);
   let break_lines_cache = BreaksLruCache::new(self.text_cache_capacity);
   ```

3. **GOP-aware splitting:**
   ```rust
   // Minimum chunk = 2 * GOP size
   let min_chunk = 2 * gop_size;  // Default GOP = 24 frames
   ```

### Codec Settings for Speed

```rust
EncoderOptions {
    codec_params: Some(&[
        ("crf", "23"),           // Quality (18-28 range)
        ("preset", "ultrafast"), // Speed preset
        ("tune", "animation"),   // Tune for animated content
        ("bframes", "5"),        // B-frames for compression
    ]),
    gop_size: 24,  // Keyframe interval
    ..Default::default()
}
```

---

## Common Pitfalls

### 1. Not Flushing the Encoder

```rust
// WRONG: Just sending frames
for frame in frames {
    avcodec_send_frame(ctx, frame);
    // Missing: receive_packet loop
}

// RIGHT: Flush with NULL frame
avcodec_send_frame(ctx, std::ptr::null_mut());
while avcodec_receive_packet(ctx, packet) == 0 {
    av_interleaved_write_frame(oc, packet);
}
```

### 2. Incorrect Timebase Rescaling

```rust
// WRONG: Using wrong timebase
(*packet).pts = frame_number;  // This will be way off

// RIGHT: Rescale properly
av_packet_rescale_ts(packet, codec_timebase, stream_timebase);
```

### 3. Not Handling EAGAIN

```rust
// WRONG: Treating EAGAIN as error
if avcodec_send_frame(ctx, frame) < 0 {
    panic!("Error sending frame");  // EAGAIN is normal!
}

// RIGHT: Check for EAGAIN
match avcodec_send_frame(ctx, frame) {
    AVERROR(EAGAIN) => {
        // Buffer full, need to receive packets first
    }
    e if e < 0 => {
        // Actual error
    }
    _ => {}
}
```

### 4. Memory Leaks

```rust
// WRONG: Not unreffing packets
avcodec_receive_packet(ctx, packet);
av_interleaved_write_frame(oc, packet);
// Missing: av_packet_unref(packet)!

// RIGHT: Always unref
while avcodec_receive_packet(ctx, packet) == 0 {
    av_interleaved_write_frame(oc, packet);
    av_packet_unref(packet);  // Critical!
}
```

---

## Sources

- `/home/darkvoid/Boxxed/@formulas/src.rust/src.fframes/rust-ffmpeg-sys/`
- `/home/darkvoid/Boxxed/@formulas/Others/src.code_to_video/fframes/fframes-renderer/src/`
- FFmpeg Documentation: https://ffmpeg.org/documentation.html
- ffmpeg-sys-next: https://github.com/zmwangx/rust-ffmpeg-sys

---

## Quick Reference: Your Integration Checklist

- [ ] **Choose linking strategy:** System libs vs static build
- [ ] **Add ffmpeg-sys-next to Cargo.toml** with needed features
- [ ] **Create safe wrapper structs** for encoder/format contexts
- [ ] **Implement proper timestamp handling** with av_rescale_q
- [ ] **Handle EAGAIN correctly** for B-frame buffering
- [ ] **Always flush encoder** with NULL frame at end
- [ ] **Free resources in correct order** (implement Drop)
- [ ] **Consider multi-threading** for parallel chunk rendering
- [ ] **Test with various codecs** (H.264, H.265, VP9, AV1)
- [ ] **Profile memory usage** for large videos
