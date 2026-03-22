---
name: Rust FFmpeg Sys
description: FFmpeg FFI bindings for Rust video/audio encoding and decoding
type: sub-project
source: /home/darkvoid/Boxxed/@formulas/src.rust/src.fframes/rust-ffmpeg-sys/
---

# Rust FFmpeg Sys - FFmpeg FFI Bindings

## Overview

Rust FFmpeg Sys provides **low-level FFI bindings to FFmpeg libraries**, enabling Rust applications to leverage FFmpeg's comprehensive audio/video codec support. This is the foundation for video encoding in FFrames and many other Rust multimedia projects.

Key features:
- **Complete FFmpeg API** - libavcodec, libavformat, libavutil, libswscale, libswresample
- **Zero-cost abstractions** - Direct FFI bindings with minimal overhead
- **Codec support** - H.264, H.265, VP9, AV1, AAC, MP3, Opus, and more
- **Format support** - MP4, MKV, WebM, MOV, AVI, and hundreds more
- **Hardware acceleration** - VAAPI, NVENC, VideoToolbox support

## Directory Structure

```
rust-ffmpeg-sys/
├── .github/                    # CI/CD workflows
├── src/
│   ├── lib.rs                  # Main module with FFI declarations
│   ├── avcodec.rs              # libavcodec bindings
│   ├── avformat.rs             # libavformat bindings
│   ├── avutil.rs               # libavutil bindings
│   ├── swscale.rs              # libswscale bindings
│   └── swresample.rs           # libswresample bindings
├── build.rs                    # Build script for FFmpeg detection
├── Cargo.toml
├── channel_layout_fixed.h      # Compatibility header
└── README.md
```

## FFmpeg Libraries Bound

### Core Libraries

| Library | Purpose | Rust Module |
|---------|---------|-------------|
| libavcodec | Codec encoding/decoding | `avcodec` |
| libavformat | Container muxing/demuxing | `avformat` |
| libavutil | Utility functions | `avutil` |
| libswscale | Image scaling/conversion | `swscale` |
| libswresample | Audio resampling | `swresample` |
| libavfilter | Video/audio filtering | `avfilter` |
| libavdevice | Input/output devices | `avdevice` |

## Build Configuration

### Build Script

```rust
// build.rs
use std::env;
use std::path::PathBuf;

fn main() {
    // Detect FFmpeg via pkg-config
    pkg_config::Config::new()
        .atleast_version("4.0")
        .probe("libavcodec")
        .unwrap();

    pkg_config::Config::new()
        .atleast_version("4.0")
        .probe("libavformat")
        .unwrap();

    pkg_config::Config::new()
        .atleast_version("4.0")
        .probe("libavutil")
        .unwrap();

    // Generate bindings via bindgen
    let bindings = bindgen::Builder::default()
        .header("wrapper.h")
        .clang_arg("-I/usr/include/ffmpeg")
        .generate()
        .unwrap();

    let out_path = PathBuf::from(env::var("OUT_DIR").unwrap());
    bindings.write_to_file(out_path.join("bindings.rs")).unwrap();
}
```

### Cargo.toml Dependencies

```toml
[package]
name = "ffmpeg-sys-fframes"
version = "0.1.0"
edition = "2021"
links = "ffmpeg"

[dependencies]
libc = "0.2"

[build-dependencies]
pkg-config = "0.3"
bindgen = "0.69"

[features]
default = ["codec-h264", "codec-aac"]
codec-h264 = []
codec-h265 = []
codec-vp9 = []
codec-av1 = []
libav-agree-gpl = []  # Accept GPL for certain codecs
```

## Core FFI Bindings

### AVCodec Context

```rust
use ffmpeg_sys::*;
use std::ptr;

// Find encoder
unsafe {
    let codec = avcodec_find_encoder(AV_CODEC_ID_H264)
        .as_ref()
        .unwrap();

    // Create codec context
    let context = avcodec_alloc_context3(codec);
    if context.is_null() {
        panic!("Failed to allocate codec context");
    }

    // Configure codec
    (*context).width = 1920;
    (*context).height = 1080;
    (*context).time_base = AVRational { num: 1, den: 60 };
    (*context).framerate = AVRational { num: 60, den: 1 };
    (*context).pix_fmt = AVPixelFormat::AV_PIX_FMT_YUV420P;
    (*context).bit_rate = 5_000_000;

    // H.264 specific options
    av_opt_set(
        (*context).priv_data,
        b"preset\0".as_ptr() as *const _,
        b"ultrafast\0".as_ptr() as *const _,
        0,
    );
    av_opt_set(
        (*context).priv_data,
        b"crf\0".as_ptr() as *const _,
        b"23\0".as_ptr() as *const _,
        0,
    );

    // Open codec
    if avcodec_open2(context, codec, ptr::null_mut()) < 0 {
        panic!("Failed to open codec");
    }
}
```

### AVFormat Context

```rust
use ffmpeg_sys::*;
use std::ffi::CString;
use std::ptr;

unsafe {
    // Allocate output context
    let mut oc: *mut AVFormatContext = ptr::null_mut();
    let filename = CString::new("output.mp4").unwrap();

    avformat_alloc_output_context2(
        &mut oc,
        ptr::null_mut(),
        ptr::null(),  // Auto-detect format from filename
        filename.as_ptr(),
    );

    if oc.is_null() {
        panic!("Failed to create output context");
    }

    // Create video stream
    let video_stream = avformat_new_stream(oc, ptr::null());
    if video_stream.is_null() {
        panic!("Failed to create stream");
    }

    // Copy codec params to stream
    let codec_params = (*video_stream).codecpar;
    (*codec_params).codec_type = AVMediaType::AVMEDIA_TYPE_VIDEO;
    (*codec_params).codec_id = AVCodecId::AV_CODEC_ID_H264;
    (*codec_params).width = 1920;
    (*codec_params).height = 1080;
    (*codec_params).format = AVPixelFormat::AV_PIX_FMT_YUV420P as i32;
    (*codec_params).codec_tag = 0;

    // Set timebase
    (*video_stream).time_base = AVRational { num: 1, den: 60 };

    // Write file header
    let mut filename_out = CString::new("output.mp4").unwrap();
    if avio_open(&mut (*oc).pb, filename_out.as_ptr(), AVIO_FLAG_WRITE) < 0 {
        panic!("Failed to open output file");
    }

    if avformat_write_header(oc, ptr::null_mut()) < 0 {
        panic!("Failed to write header");
    }
}
```

### Frame Allocation and Conversion

```rust
use ffmpeg_sys::*;
use std::ptr;

unsafe {
    // Allocate frame
    let frame = av_frame_alloc();
    if frame.is_null() {
        panic!("Failed to allocate frame");
    }

    (*frame).width = 1920;
    (*frame).height = 1080;
    (*frame).format = AVPixelFormat::AV_PIX_FMT_YUV420P;

    // Allocate frame buffers
    if av_frame_get_buffer(frame, 32) < 0 {
        panic!("Failed to allocate frame buffers");
    }

    // Make frame writable
    if av_frame_make_writable(frame) < 0 {
        panic!("Failed to make frame writable");
    }

    // Fill with dummy data (YUV420P)
    for y in 0..1080 {
        for x in 0..1920 {
            (*frame).data[0][y * (*frame).linesize[0] + x] = (x + y) as u8;
        }
    }

    // Set presentation timestamp
    (*frame).pts = 0;
}
```

### Packet Management

```rust
use ffmpeg_sys::*;
use std::ptr;

unsafe {
    // Allocate packet
    let packet = av_packet_alloc();

    // Send frame to encoder
    if avcodec_send_frame(codec_context, frame) < 0 {
        panic!("Failed to send frame");
    }

    // Receive encoded packet
    loop {
        match avcodec_receive_packet(codec_context, packet) {
            0 => {
                // Successfully received packet
                println!(
                    "Packet: size={}, pts={}, dts={}",
                    (*packet).size,
                    (*packet).pts,
                    (*packet).dts,
                );

                // Write packet to file
                if av_write_frame(format_context, packet) < 0 {
                    panic!("Failed to write packet");
                }

                av_packet_unref(packet);
            }
            AVERROR_EAGAIN => break, // Need more frames
            AVERROR_EOF => break,    // Encoding complete
            e => panic!("Error receiving packet: {}", e),
        }
    }

    av_packet_free(&mut packet);
}
```

### Color Space Conversion (SwsContext)

```rust
use ffmpeg_sys::*;
use std::ptr;

unsafe {
    // Create scaling context
    let sws = sws_getContext(
        1920,  // Source width
        1080,  // Source height
        AVPixelFormat::AV_PIX_FMT_RGBA,  // Source format
        1920,  // Destination width
        1080,  // Destination height
        AVPixelFormat::AV_PIX_FMT_YUV420P,  // Destination format
        SWS_BILINEAR,  // Scaling algorithm
        ptr::null_mut(),
        ptr::null_mut(),
        ptr::null_mut(),
    );

    // Convert RGBA to YUV
    let src_data: [*mut u8; 4] = [rgba_data, ptr::null_mut(), ptr::null_mut(), ptr::null_mut()];
    let src_linesize: [i32; 4] = [(1920 * 4) as i32, 0, 0, 0];

    sws_scale(
        sws,
        &src_data[0] as *const *mut u8,
        &src_linesize[0] as *const i32,
        0,
        1080,
        &(*dst_frame).data,
        &(*dst_frame).linesize,
    );

    sws_freeContext(sws);
}
```

### Audio Encoding (AAC)

```rust
use ffmpeg_sys::*;
use std::ptr;

unsafe {
    // Find AAC encoder
    let codec = avcodec_find_encoder(AV_CODEC_ID_AAC).unwrap();
    let context = avcodec_alloc_context3(codec);

    // Configure audio codec
    (*context).sample_rate = 44100;
    (*context).channels = 2;
    (*context).channel_layout = AV_CH_LAYOUT_STEREO;
    (*context).sample_fmt = AVSampleFormat::AV_SAMPLE_FMT_FLTP;
    (*context).bit_rate = 128_000;

    avcodec_open2(context, codec, ptr::null_mut())?;

    // Allocate audio frame
    let frame = av_frame_alloc();
    (*frame).nb_samples = (*context).frame_size;
    (*frame).format = AVSampleFormat::AV_SAMPLE_FMT_FLTP;
    (*frame).channel_layout = AV_CH_LAYOUT_STEREO;

    av_frame_get_buffer(frame, 0);

    // Fill with audio samples (FLTP format)
    for ch in 0..2 {
        let data = (*frame).data[ch] as *mut f32;
        for i in 0..(*context).frame_size as usize {
            *data.add(i) = (i as f32 * 0.01).sin(); // Sine wave
        }
    }

    // Encode audio
    avcodec_send_frame(context, frame)?;

    let packet = av_packet_alloc();
    while avcodec_receive_packet(context, packet) == 0 {
        // Write audio packet
        av_write_frame(format_context, packet);
        av_packet_unref(packet);
    }
}
```

## Safe Wrapper Pattern

```rust
// Example of safe wrapper around FFI
pub struct Encoder {
    context: *mut AVCodecContext,
    format_context: *mut AVFormatContext,
}

impl Encoder {
    pub fn new(width: i32, height: i32, fps: i32) -> Result<Self, EncoderError> {
        unsafe {
            let codec = avcodec_find_encoder(AV_CODEC_ID_H264)
                .ok_or(EncoderError::CodecNotFound)?;

            let context = avcodec_alloc_context3(codec);
            if context.is_null() {
                return Err(EncoderError::AllocationFailed);
            }

            (*context).width = width;
            (*context).height = height;
            (*context).time_base = AVRational { num: 1, den: fps };
            (*context).pix_fmt = AVPixelFormat::AV_PIX_FMT_YUV420P;

            avcodec_open2(context, codec, ptr::null_mut())?;

            Ok(Encoder { context, format_context: ptr::null_mut() })
        }
    }

    pub fn encode(&mut self, frame: &AVFrame) -> Result<Vec<u8>, EncoderError> {
        unsafe {
            avcodec_send_frame(self.context, frame)?;

            let packet = av_packet_alloc();
            let mut data = Vec::new();

            while avcodec_receive_packet(self.context, packet) == 0 {
                let slice = std::slice::from_raw_parts(
                    (*packet).data,
                    (*packet).size as usize,
                );
                data.extend_from_slice(slice);
                av_packet_unref(packet);
            }

            av_packet_free(&mut packet);
            Ok(data)
        }
    }
}

impl Drop for Encoder {
    fn drop(&mut self) {
        unsafe {
            avcodec_free_context(&mut self.context);
            if !self.format_context.is_null() {
                avformat_free_context(self.format_context);
            }
        }
    }
}
```

## Integration with FFrames

```rust
// From fframes-renderer
use ffmpeg_sys::*;

pub struct FFmpegEncoder {
    codec_context: *mut AVCodecContext,
    format_context: *mut AVFormatContext,
    video_stream: *mut AVStream,
    sws_context: *mut SwsContext,
}

impl FFmpegEncoder {
    pub fn encode_frame(
        &mut self,
        rgba_data: &[u8],
        width: usize,
        height: usize,
        pts: i64,
    ) -> Result<(), EncoderError> {
        unsafe {
            // Create frame
            let frame = av_frame_alloc();
            (*frame).width = width as i32;
            (*frame).height = height as i32;
            (*frame).format = AVPixelFormat::AV_PIX_FMT_YUV420P;
            (*frame).pts = pts;

            av_frame_get_buffer(frame, 0);
            av_frame_make_writable(frame);

            // Convert RGBA to YUV420P
            let src_data: [*mut u8; 4] = [rgba_data.as_ptr() as *mut u8, ptr::null_mut(), ptr::null_mut(), ptr::null_mut()];
            let src_linesize: [i32; 4] = [(width * 4) as i32, 0, 0, 0];

            sws_scale(
                self.sws_context,
                &src_data[0] as *const *mut u8,
                &src_linesize[0] as *const i32,
                0,
                height as i32,
                &(*frame).data,
                &(*frame).linesize,
            );

            // Send to encoder
            avcodec_send_frame(self.codec_context, frame)?;

            // Receive and write packets
            let packet = av_packet_alloc();
            while avcodec_receive_packet(self.codec_context, packet) == 0 {
                packet.pts = av_rescale_q_rnd(
                    (*packet).pts,
                    (*self.codec_context).time_base,
                    (*self.video_stream).time_base,
                    AVRounding::AV_ROUND_NEAR_INF | AVRounding::AV_ROUND_PASS_MINMAX,
                );
                packet.dts = packet.pts;
                packet.stream_index = (*self.video_stream).index;

                av_write_frame(self.format_context, packet);
                av_packet_unref(packet);
            }

            av_frame_free(&mut frame);
            av_packet_free(&mut packet);
        }

        Ok(())
    }
}
```

## System Dependencies

### Linux (Debian/Ubuntu)

```bash
apt install \
    libavcodec-dev \
    libavformat-dev \
    libavutil-dev \
    libswscale-dev \
    libswresample-dev \
    libavfilter-dev \
    libavdevice-dev \
    pkg-config \
    clang
```

### Arch Linux

```bash
pacman -S \
    ffmpeg \
    pkg-config \
    clang
```

### macOS

```bash
brew install ffmpeg pkg-config
```

### Windows (vcpkg)

```bash
vcpkg install ffmpeg:x64-windows
set PKG_CONFIG_PATH=%VCPKG_ROOT%\installed\x64-windows\lib\pkgconfig
```

## Related Documents

- [FFrames Renderer](./fframes-renderer-exploration.md) - Video encoding usage
- [Rust Skia](./rust-skia-exploration.md) - GPU rendering

## Sources

- Source: `/home/darkvoid/Boxxed/@formulas/src.rust/src.fframes/rust-ffmpeg-sys/`
- FFmpeg Documentation: https://ffmpeg.org/documentation.html
- FFrames Main Exploration: `../../fframes/exploration.md`
