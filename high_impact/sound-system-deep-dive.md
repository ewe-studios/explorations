---
location: /home/darkvoid/Boxxed/@formulas/src.Gaming/src.Impactjs/high_impact/src/
repository: https://github.com/phoboslab/high_impact
explored_at: 2026-03-20
language: C
parent: exploration.md
---

# High Impact Sound System - Deep Dive

**Source Files:** `sound.h`, `sound.c`

**Dependencies:** `pl_synth.h` (procedural audio synthesis)

---

## Table of Contents

1. [Sound Architecture Overview](#1-sound-architecture-overview)
2. [Source/Node Pattern](#2-sourcenode-pattern)
3. [QOA Audio Format](#3-qoa-audio-format)
4. [Sound Loading](#4-sound-loading)
5. [Node Control](#5-node-control)
6. [Audio Mixing](#6-audio-mixing)
7. [Procedural Audio (pl_synth)](#7-procedural-audio-pl_synth)
8. [Memory Management](#8-memory-management)
9. [Platform Integration](#9-platform-integration)

---

## 1. Sound Architecture Overview

### 1.1 System Design

The sound system uses a **source/instance pattern**:

```
┌────────────────────────────────────────────────────────────┐
│                     SOUND SYSTEM                            │
├────────────────────────────────────────────────────────────┤
│                                                             │
│  ┌─────────────────────────────────────────────────────┐   │
│  │              Sound Sources (Loaded Data)            │   │
│  │  ┌──────────┐  ┌──────────┐  ┌──────────┐          │   │
│  │  │ Source 0 │  │ Source 1 │  │ Source N │          │   │
│  │  │ (jump.qoa)│ │ (shoot.qoa)│ │ (music) │          │   │
│  │  │ samples[]│  │ samples[]│  │ samples[]│          │   │
│  │  └────┬─────┘  └────┬─────┘  └────┬─────┘          │   │
│  │       │             │             │                 │   │
│  └───────┼─────────────┼─────────────┼─────────────────┘   │
│          │             │             │                      │
│          ▼             ▼             ▼                      │
│  ┌─────────────────────────────────────────────────────┐   │
│  │              Sound Nodes (Playing)                  │   │
│  │  ┌──────────┐  ┌──────────┐  ┌──────────┐          │   │
│  │  │ Node 0   │  │ Node 1   │  │ Node N   │          │   │
│  │  │ source=0 │  │ source=1 │  │ source=0 │          │   │
│  │  │ pos=0.5s │  │ pos=0.0s │  │ pos=1.2s │          │   │
│  │  │ vol=1.0  │  │ vol=0.8  │  │ vol=0.5  │          │   │
│  │  │ loop=false│ │ loop=true│ │ pitch=1.5│          │   │
│  │  └──────────┘  └──────────┘  └──────────┘          │   │
│  └─────────────────────────────────────────────────────┘   │
│                              │                               │
│                              ▼                               │
│                   ┌──────────────────┐                      │
│                   │  Audio Mixer     │                      │
│                   │  (32 channels)   │                      │
│                   └────────┬─────────┘                      │
│                            │                                 │
│                            ▼                                 │
│                   ┌──────────────────┐                      │
│                   │  Platform Output │                      │
│                   │  (SDL/sokol)     │                      │
│                   └──────────────────┘                      │
│                                                             │
└────────────────────────────────────────────────────────────┘
```

### 1.2 Key Structures

```c
// Sound source - loaded audio data
typedef struct sound_source_t {
    char path[256];           // File path (for caching)
    int16_t *samples;         // PCM samples (-32768 to 32767)
    uint32_t sample_count;    // Number of samples
    uint32_t channels;        // 1 = mono, 2 = stereo
    uint32_t samplerate;      // Usually 44100 or 48000
    bool is_compressed;       // Uses on-demand decompression
    void *compressed_data;    // QOA compressed data
} sound_source_t;

// Sound node - playing instance
typedef struct {
    uint16_t id;              // Unique identifier
    uint16_t source_index;    // Index in sources array
    uint32_t position;        // Current sample position
    float volume;             // 0.0 - 1.0
    float pan;                // -1.0 (left) to 1.0 (right)
    float pitch;              // 0.5 - 2.0 (playback speed)
    bool loop;                // Loop at end
    bool paused;              // Paused state
    bool active;              // Currently playing
} sound_node_t;

// Handle types
typedef struct { uint16_t id; uint16_t index; } sound_t;      // Node handle
typedef struct { uint32_t index; } sound_mark_t;              // Memory mark
```

### 1.3 Configuration

```c
// Maximum limits
#define SOUND_MAX_UNCOMPRESSED_SAMPLES (64 * 1024)  // 64KB
#define SOUND_MAX_SOURCES 128
#define SOUND_MAX_NODES 32

// Default samplerate
#define SOUND_SAMPLERATE 44100
```

---

## 2. Source/Node Pattern

### 2.1 Sources Are Shared

Sources are **loaded once and shared** across multiple playbacks:

```c
// Load once
sound_source_t *jump_snd = sound_source("assets/sfx/jump.qoa");

// Play multiple times simultaneously
sound_play(jump_snd);  // Node 1
sound_play(jump_snd);  // Node 2
sound_play(jump_snd);  // Node 3
```

### 2.2 Nodes Are Instances

Each `sound_play()` creates a new **node** (instance):

```c
// Get a node (paused)
sound_t s = sound(jump_snd);

// Configure before playing
sound_set_volume(s, 0.5f);
sound_set_pitch(s, 1.2f);
sound_unpause(s);  // Start playback
```

### 2.3 Node Lifecycle

```
sound_source("jump.qoa")
    │
    └─> Returns cached source (or loads new)

sound_play(source)
    │
    ├─> Find free node slot
    ├─> Associate with source
    ├─> Reset position to 0
    ├─> Set active = true
    └─> Will be mixed into output

    During playback:
    │
    ├─> Each mix: read samples from source
    ├─> Apply volume, pan, pitch
    ├─> Advance position
    └─> When done: auto-dispose (if not looping)

sound_dispose(node)
    │
    └─> Mark node as free for reuse
```

---

## 3. QOA Audio Format

### 3.1 Format Overview

**QOA (Quick Object Audio)** is a simple audio format by phoboslab:

- **Lossy compression** (~4:1 ratio)
- **Very fast decode** (designed for games)
- **Simple specification** (single header file)
- **Sample rates:** 44100, 48000 Hz
- **Channels:** Mono or Stereo

### 3.2 QOA File Structure

```
┌─────────────────────────────────────┐
│           QOA Header (8 bytes)      │
│  "qoafmt" magic (4 bytes)           │
│  samples count (4 bytes)            │
├─────────────────────────────────────┤
│         QOA Audio Data              │
│  Variable length, encoded frames    │
├─────────────────────────────────────┤
│           QOA Footer (8 bytes)      │
│  "qoaend" magic (4 bytes)           │
│  total size (4 bytes)               │
└─────────────────────────────────────┘
```

### 3.3 Loading QOA

```c
sound_source_t *sound_source(char *path) {
    // Check cache first
    for (int i = 0; i < source_count; i++) {
        if (strcmp(sources[i].path, path) == 0) {
            return &sources[i];
        }
    }

    // Load file
    uint8_t *data = platform_load_asset(path, &file_size);

    // Decode QOA
    qoa_desc desc;
    int16_t *samples = qoa_decode(data, file_size, &desc);

    // Create source
    sound_source_t *src = &sources[source_count++];
    strncpy(src->path, path, sizeof(src->path));
    src->samples = samples;
    src->sample_count = desc.samples;
    src->channels = desc.channels;
    src->samplerate = desc.samplerate;

    // Decide if we keep decompressed
    if (src->sample_count > SOUND_MAX_UNCOMPRESSED_SAMPLES) {
        // Too large - keep compressed, decompress on demand
        src->is_compressed = true;
        src->compressed_data = data;
        src->samples = NULL;
    } else {
        temp_free(data);
    }

    return src;
}
```

---

## 4. Sound Loading

### 4.1 Loading from File

```c
// Load sound effect
sound_source_t *laser = sound_source("assets/sfx/laser.qoa");

// Load music
sound_source_t *theme = sound_source("assets/music/theme.qoa");
```

### 4.2 Loading from Samples

```c
// Generate procedurally
#define SAMPLE_RATE 44100
#define DURATION_SEC 1.0f
uint32_t samples = SAMPLE_RATE * DURATION_SEC;
int16_t *buffer = malloc(samples * sizeof(int16_t));

// Fill with synthesized audio
for (uint32_t i = 0; i < samples; i++) {
    float t = (float)i / SAMPLE_RATE;
    buffer[i] = sinf(t * 440.0f * 2 * M_PI) * 16384;  // 440 Hz sine
}

// Create source (doesn't take ownership)
sound_source_t *sine = sound_source_with_samples(
    buffer, samples, 1, SAMPLE_RATE
);
```

### 4.3 Loading from pl_synth

```c
// Define a sound effect
pl_synth_sound_t explosion_sound = {
    .type = PL_SYNTH_SOUND_NOISE,
    .duration = 0.5f,
    .volume = 0.8f,
    .pitch_start = 200.0f,
    .pitch_end = 50.0f,  // Pitch drop for explosion effect
};

// Create source from definition
sound_source_t *explosion = sound_source_synth_sound(&explosion_sound);
```

---

## 5. Node Control

### 5.1 Basic Playback

```c
// One-shot playback (auto-disposes when done)
sound_play(source);

// With parameters
sound_play_ex(source,
    0.5f,   // volume
    0.0f,   // pan (center)
    1.0f    // pitch (normal speed)
);
```

### 5.2 Node Lifecycle Control

```c
// Get node (paused)
sound_t s = sound(source);

// Configure
sound_set_volume(s, 0.7f);
sound_set_pan(s, -0.5f);   // Pan left
sound_set_pitch(s, 1.5f);  // Faster playback
sound_set_loop(s, true);   // Loop indefinitely

// Start
sound_unpause(s);

// Later...
sound_pause(s);    // Pause
sound_unpause(s);  // Resume
sound_stop(s);     // Stop and rewind
sound_dispose(s);  // Release slot (continues playing)
```

### 5.3 Position Control

```c
// Get duration (seconds)
float duration = sound_duration(s);

// Get current position
float pos = sound_time(s);

// Seek
sound_set_time(s, 2.5f);  // Jump to 2.5 seconds
```

### 5.4 State Queries

```c
bool is_looping = sound_loop(s);
float volume = sound_volume(s);
float pan = sound_pan(s);
float pitch = sound_pitch(s);
float time = sound_time(s);
float duration = sound_duration(s);
```

---

## 6. Audio Mixing

### 6.2 Mix Callback

```c
void sound_mix_stereo(float *dest_samples, uint32_t dest_len) {
    // Clear buffer
    memset(dest_samples, 0, dest_len * sizeof(float) * 2);

    // Mix all active nodes
    for (int i = 0; i < SOUND_MAX_NODES; i++) {
        sound_node_t *node = &nodes[i];
        if (!node->active || node->paused) continue;

        sound_source_t *src = &sources[node->source_index];

        // Get samples from source
        int16_t *src_samples = get_source_samples(src);

        // Mix into buffer
        for (uint32_t j = 0; j < dest_len; j++) {
            if (node->position >= src->sample_count) {
                if (node->loop) {
                    node->position = 0;
                } else {
                    node->active = false;
                    break;
                }
            }

            // Read sample with pitch adjustment
            uint32_t src_pos = node->position;
            node->position += (uint32_t)node->pitch;

            float sample = src_samples[src_pos] / 32768.0f;

            // Apply volume
            sample *= node->volume;

            // Apply pan
            float left_pan = node->pan < 0 ? 1.0f : 1.0f - node->pan;
            float right_pan = node->pan > 0 ? 1.0f : 1.0f + node->pan;

            // Mix to stereo
            dest_samples[j * 2] += sample * left_pan * 0.5f;
            dest_samples[j * 2 + 1] += sample * right_pan * 0.5f;
        }
    }
}
```

### 6.3 Global Volume

```c
static float global_volume = 1.0f;

float sound_global_volume(void) {
    return global_volume;
}

void sound_set_global_volume(float volume) {
    global_volume = clampf(volume, 0.0f, 1.0f);
}

// Applied during mix
sample *= global_volume;
```

### 6.4 Halt/Resume

```c
// Pause all sounds (e.g., pause screen)
void sound_halt(void) {
    for (int i = 0; i < SOUND_MAX_NODES; i++) {
        if (nodes[i].active) {
            nodes[i].paused = true;
        }
    }
}

// Resume all sounds
void sound_resume(void) {
    for (int i = 0; i < SOUND_MAX_NODES; i++) {
        if (nodes[i].active && nodes[i].paused) {
            nodes[i].paused = false;
        }
    }
}
```

---

## 7. Procedural Audio (pl_synth)

### 7.1 Sound Definitions

```c
// Sine wave
pl_synth_sound_t sine = {
    .type = PL_SYNTH_SOUND_SINE,
    .frequency = 440.0f,  // Hz
    .duration = 1.0f,
    .volume = 0.5f,
};

// Noise (for explosions, etc.)
pl_synth_sound_t noise = {
    .type = PL_SYNTH_SOUND_NOISE,
    .duration = 0.5f,
    .volume = 0.8f,
};

// Sweep (for jumps, powerups)
pl_synth_sound_t sweep = {
    .type = PL_SYNTH_SOUND_SWEEP,
    .frequency_start = 200.0f,
    .frequency_end = 800.0f,
    .duration = 0.3f,
    .volume = 0.6f,
};
```

### 7.2 Song Definitions

```c
// Simple sequence
pl_synth_song_t theme = {
    .bpm = 120,
    .tracks = {
        {
            .sound = &sine_sound,
            .notes = {
                { .note = 60, .duration = 0.5f },  // C4
                { .note = 64, .duration = 0.5f },  // E4
                { .note = 67, .duration = 1.0f },  // G4
            },
            .note_count = 3,
        },
    },
    .track_count = 1,
};

// Create source from song
sound_source_t *music = sound_source_synth_song(&theme);
sound_set_loop(music, true);
sound_play(music);
```

### 7.3 Synth Initialization

```c
void sound_init_synth(void) {
    pl_synth_init(SOUND_SAMPLERATE);
}
```

---

## 8. Memory Management

### 8.1 Mark/Reset Pattern

```c
// Save current state
sound_mark_t mark = sound_mark();

// Load level sounds
sound_source("level1_jump.qoa");
sound_source("level1_coin.qoa");
sound_source("level1_enemy.qoa");

// On level change:
sound_reset(mark);  // Free all sounds loaded after mark
```

### 8.2 Implementation

```c
sound_mark_t sound_mark(void) {
    return (sound_mark_t){.index = source_count};
}

void sound_reset(sound_mark_t mark) {
    // Free sources since mark
    for (int i = mark.index; i < source_count; i++) {
        sound_source_t *src = &sources[i];
        if (src->samples && !src->is_compressed) {
            free(src->samples);
        }
        if (src->compressed_data) {
            temp_free(src->compressed_data);
        }
    }
    source_count = mark.index;

    // Stop all nodes using freed sources
    for (int i = 0; i < SOUND_MAX_NODES; i++) {
        if (nodes[i].source_index >= mark.index) {
            nodes[i].active = false;
        }
    }
}
```

---

## 9. Platform Integration

### 9.1 SDL Audio

```c
void platform_init_audio(void) {
    SDL_AudioSpec want = {0};
    want.freq = SOUND_SAMPLERATE;
    want.format = AUDIO_F32;
    want.channels = 2;
    want.samples = 1024;
    want.callback = sdl_audio_callback;
    want.userdata = NULL;

    audio_device = SDL_OpenAudioDevice(NULL, 0, &want, NULL, 0);
    SDL_PauseAudioDevice(audio_device, 0);
}

void sdl_audio_callback(void *userdata, Uint8 *stream, int len) {
    float *buffer = (float *)stream;
    sound_mix_stereo(buffer, len / sizeof(float) / 2);
}
```

### 9.2 Sokol Audio

```c
void sokol_audio_callback(const saudio_frame_desc *desc) {
    float *buffer = desc->buffer;
    int frames = desc->num_frames;
    sound_mix_stereo(buffer, frames);
}

sapp_desc sokol_main(int argc, char *argv[]) {
    return (sapp_desc){
        // ...
        .audio = (saudio_desc){
            .sample_rate = SOUND_SAMPLERATE,
            .num_channels = 2,
            .buffer_size = 2048,
            .stream_cb = sokol_audio_callback,
        },
    };
}
```

---

## Related Documents

- **[Main Exploration](./exploration.md)** - Overall architecture
- **[Platform Layer Deep Dive](./platform-layer-deep-dive.md)** - Audio backend setup
- **[Memory Management](./memory-management-deep-dive.md)** - Arena allocation
