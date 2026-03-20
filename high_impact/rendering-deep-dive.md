---
location: /home/darkvoid/Boxxed/@formulas/src.Gaming/src.Impactjs/high_impact/src/
repository: https://github.com/phoboslab/high_impact
explored_at: 2026-03-20
language: C
parent: exploration.md
---

# High Impact Rendering System - Deep Dive

**Source Files:** `render.h`, `render_gl.c`, `render_metal.m`, `render_software.c`

---

## Table of Contents

1. [Renderer Architecture](#1-renderer-architecture)
2. [Logical vs. Real Pixels](#2-logical-vs-real-pixels)
3. [Transform Stack](#3-transform-stack)
4. [OpenGL Backend](#4-opengl-backend)
5. [Metal Backend](#5-metal-backend)
6. [Software Renderer](#6-software-renderer)
7. [Texture Management](#7-texture-management)
8. [Draw Call Pipeline](#8-draw-call-pipeline)
9. [Post-Processing Effects](#9-post-processing-effects)

---

## 1. Renderer Architecture

### 1.1 Backend Abstraction

High Impact supports **multiple render backends** through a common interface:

```c
// Required backend functions
void render_backend_init(void);
void render_backend_cleanup(void);
void render_set_screen(vec2i_t size);
void render_set_blend_mode(render_blend_mode_t mode);
void render_set_post_effect(render_post_effect_t post);
void render_frame_prepare(void);
void render_frame_end(void);
void render_draw_quad(quadverts_t *quad, texture_t texture_handle);
texture_mark_t textures_mark(void);
void textures_reset(texture_mark_t mark);
texture_t texture_create(vec2i_t size, rgba_t *pixels);
void texture_replace_pixels(texture_t texture, vec2i_t size, rgba_t *pixels);
```

### 1.2 Available Backends

| Backend | File | Platform | Features |
|---------|------|----------|----------|
| OpenGL | `render_gl.c` | Linux, Windows | Shaders, blending |
| Metal | `render_metal.m` | macOS, iOS | Shaders, blending |
| Software | `render_software.c` | All | Basic drawing |

### 1.3 Renderer State

```c
typedef struct {
    vec2i_t screen_size;      // Real pixel size
    vec2i_t logical_size;     // RENDER_WIDTH/HEIGHT
    vec2_t scale;             // screen / logical

    render_blend_mode_t blend_mode;
    render_post_effect_t post_effect;

    mat3_t transform_stack[RENDER_TRANSFORM_STACK_SIZE];
    int transform_stack_depth;

    uint32_t draw_calls;      // Frame counter
} render_state_t;
```

---

## 2. Logical vs. Real Pixels

### 2.1 Configuration

```c
#define RENDER_WIDTH 1280
#define RENDER_HEIGHT 720

#define RENDER_SCALE_MODE RENDER_SCALE_DISCRETE
#define RENDER_RESIZE_MODE RENDER_RESIZE_ANY
```

### 2.2 Scale Modes

```c
typedef enum {
    RENDER_SCALE_NONE,      // 1:1 pixel mapping
    RENDER_SCALE_DISCRETE,  // Integer scaling (pixel perfect)
    RENDER_SCALE_EXACT      // Stretch to fit
} render_scale_mode_t;
```

**DISCRETE scaling example:**
```
Logical:  320x240
Window:   1280x960
Scale:    4x (integer)
Result:   Each logical pixel = 4x4 real pixels
```

### 2.3 Resize Modes

```c
typedef enum {
    RENDER_RESIZE_NONE,    // Fixed logical size
    RENDER_RESIZE_WIDTH,   // Width adapts, height fixed
    RENDER_RESIZE_HEIGHT,  // Height adapts, width fixed
    RENDER_RESIZE_ANY      // Both adapt (fills window)
} render_resize_mode_t;
```

### 2.4 Resize Calculation

```c
void render_resize(vec2i_t available_size) {
    vec2i_t logical = vec2i(RENDER_WIDTH, RENDER_HEIGHT);
    vec2i_t screen = available_size;

    switch (RENDER_RESIZE_MODE) {
        case RENDER_RESIZE_NONE:
            logical = vec2i(RENDER_WIDTH, RENDER_HEIGHT);
            break;

        case RENDER_RESIZE_WIDTH:
            logical.x = screen.x;
            logical.y = RENDER_HEIGHT;
            break;

        case RENDER_RESIZE_HEIGHT:
            logical.x = RENDER_WIDTH;
            logical.y = screen.y;
            break;

        case RENDER_RESIZE_ANY:
            logical = screen;
            break;
    }

    // Apply scale mode
    switch (RENDER_SCALE_MODE) {
        case RENDER_SCALE_DISCRETE: {
            float scale_x = floorf(screen.x / (float)logical.x);
            float scale_y = floorf(screen.y / (float)logical.y);
            float scale = fminf(scale_x, scale_y);
            if (scale < 1) scale = 1;

            logical.x = screen.x / scale;
            logical.y = screen.y / scale;
            break;
        }

        case RENDER_SCALE_EXACT:
            // logical = screen (already set)
            break;
    }

    render_state.logical_size = logical;
    render_state.screen_size = screen;
    render_state.scale = vec2(
        screen.x / (float)logical.x,
        screen.y / (float)logical.y
    );

    render_set_screen(screen);
}
```

### 2.5 Pixel Snapping

```c
vec2_t render_snap_px(vec2_t pos) {
    // Convert logical to real pixels, snap, convert back
    float sx = render_state.scale.x;
    float sy = render_state.scale.y;

    return vec2(
        roundf(pos.x * sx) / sx,
        roundf(pos.y * sy) / sy
    );
}
```

---

## 3. Transform Stack

### 3.1 Stack Operations

```c
#define RENDER_TRANSFORM_STACK_SIZE 16

static mat3_t transform_stack[RENDER_TRANSFORM_STACK_SIZE];
static int transform_depth = 0;

void render_push(void) {
    if (transform_depth >= RENDER_TRANSFORM_STACK_SIZE) {
        return;  // Overflow protection
    }
    transform_stack[transform_depth++] = current_transform;
}

void render_pop(void) {
    if (transform_depth > 0) {
        current_transform = transform_stack[--transform_depth];
    }
}
```

### 3.2 Transform Operations

```c
void render_translate(vec2_t t) {
    mat3_translate(&current_transform, t);
}

void render_scale(vec2_t s) {
    mat3_scale(&current_transform, s);
}

void render_rotate(float r) {
    mat3_rotate(&current_transform, r);
}
```

### 3.3 Matrix Math

```c
static mat3_t current_transform = {1, 0, 0, 1, 0, 0};  // Identity

mat3_t *mat3_translate(mat3_t *m, vec2_t t) {
    m->tx += m->a * t.x + m->c * t.y;
    m->ty += m->b * t.x + m->d * t.y;
    return m;
}

mat3_t *mat3_scale(mat3_t *m, vec2_t r) {
    m->a *= r.x;
    m->b *= r.x;
    m->c *= r.y;
    m->d *= r.y;
    return m;
}

mat3_t *mat3_rotate(mat3_t *m, float r) {
    float sin = sinf(r);
    float cos = cosf(r);
    float a = m->a, b = m->b;
    float c = m->c, d = m->d;

    m->a = a * cos + c * sin;
    m->b = b * cos + d * sin;
    m->c = c * cos - a * sin;
    m->d = d * cos - b * sin;
    return m;
}

vec2_t vec2_transform(vec2_t v, mat3_t *m) {
    return vec2(
        m->a * v.x + m->b * v.y + m->tx,
        m->c * v.x + m->d * v.y + m->ty
    );
}
```

### 3.4 Usage Pattern

```c
// Draw rotated sprite
render_push();
render_translate(entity->pos);
render_rotate(entity->angle);
render_translate(vec2(-entity->size.x/2, -entity->size.y/2));
anim_draw(entity->anim, vec2(0, 0));
render_pop();
```

---

## 4. OpenGL Backend

### 4.1 Initialization

```c
// render_gl.c
void render_backend_init(void) {
    // Load OpenGL functions via glad
    gladLoadGL();

    // Create vertex array
    glGenVertexArrays(1, &vao);
    glBindVertexArray(vao);

    // Create vertex buffer
    glGenBuffers(1, &vbo);
    glBindBuffer(GL_ARRAY_BUFFER, vbo);
    glBufferData(GL_ARRAY_BUFFER, sizeof(vertex_t) * 4, NULL, GL_DYNAMIC_DRAW);

    // Setup vertex attributes
    glVertexAttribPointer(0, 2, GL_FLOAT, GL_FALSE, sizeof(vertex_t),
                          (void*)offsetof(vertex_t, pos));
    glEnableVertexAttribArray(0);

    glVertexAttribPointer(1, 2, GL_FLOAT, GL_FALSE, sizeof(vertex_t),
                          (void*)offsetof(vertex_t, uv));
    glEnableVertexAttribArray(1);

    glVertexAttribPointer(2, 4, GL_UNSIGNED_BYTE, GL_TRUE, sizeof(vertex_t),
                          (void*)offsetof(vertex_t, color));
    glEnableVertexAttribArray(2);

    // Create shader program
    GLuint vs = glCreateShader(GL_VERTEX_SHADER);
    glShaderSource(vs, 1, &vertex_shader_src, NULL);
    glCompileShader(vs);

    GLuint fs = glCreateShader(GL_FRAGMENT_SHADER);
    glShaderSource(fs, 1, &fragment_shader_src, NULL);
    glCompileShader(fs);

    shader_program = glCreateProgram();
    glAttachShader(shader_program, vs);
    glAttachShader(shader_program, fs);
    glLinkProgram(shader_program);
}
```

### 4.2 Shaders

```glsl
// Vertex Shader
#version 330 core
layout(location = 0) in vec2 in_pos;
layout(location = 1) in vec2 in_uv;
layout(location = 2) in vec4 in_color;

out vec2 v_uv;
out vec4 v_color;

uniform mat3 u_transform;
uniform vec2 u_screen_size;

void main() {
    vec3 transformed = u_transform * vec3(in_pos, 1.0);
    // Convert to clip space (-1 to 1)
    vec2 clip_pos = (transformed.xy / u_screen_size) * 2.0 - 1.0;
    gl_Position = vec4(clip_pos * vec2(1, -1), 0.0, 1.0);
    v_uv = in_uv;
    v_color = in_color;
}

// Fragment Shader
#version 330 core
in vec2 v_uv;
in vec4 v_color;
out vec4 frag_color;

uniform sampler2D u_texture;
uniform int u_use_texture;

void main() {
    vec4 tex_color = u_use_texture ? texture(u_texture, v_uv) : vec4(1.0);
    frag_color = tex_color * v_color;
}
```

### 4.3 Draw Quad

```c
void render_draw_quad(quadverts_t *quad, texture_t texture_handle) {
    glUseProgram(shader_program);

    // Update vertex buffer
    glBindBuffer(GL_ARRAY_BUFFER, vbo);
    glBufferSubData(GL_ARRAY_BUFFER, 0, sizeof(vertex_t) * 4, quad->vertices);

    // Bind texture
    if (texture_handle.index != RENDER_NO_TEXTURE.index) {
        glBindTexture(GL_TEXTURE_2D, texture_handle.index);
        glUniform1i(glGetUniformLocation(shader_program, "u_use_texture"), 1);
    } else {
        glUniform1i(glGetUniformLocation(shader_program, "u_use_texture"), 0);
    }

    // Set transform
    glUniformMatrix3fv(glGetUniformLocation(shader_program, "u_transform"),
                       1, GL_FALSE, (float*)&current_transform);
    glUniform2f(glGetUniformLocation(shader_program, "u_screen_size"),
                render_state.screen_size.x, render_state.screen_size.y);

    // Draw
    glDrawArrays(GL_TRIANGLE_FAN, 0, 4);
    render_state.draw_calls++;
}
```

### 4.4 Blending

```c
void render_set_blend_mode(render_blend_mode_t mode) {
    switch (mode) {
        case RENDER_BLEND_NORMAL:
            glEnable(GL_BLEND);
            glBlendFunc(GL_SRC_ALPHA, GL_ONE_MINUS_SRC_ALPHA);
            break;
        case RENDER_BLEND_LIGHTER:
            glEnable(GL_BLEND);
            glBlendFunc(GL_SRC_ALPHA, GL_ONE);
            break;
    }
}
```

---

## 5. Metal Backend

### 5.1 Layer Integration

```objc
// render_metal.m
void *platform_get_metal_layer(void) {
    NSWindow *window = [NSApp keyWindow];
    return [window.contentView layer];
}

void render_backend_init(void) {
    id<MTLDevice> device = MTLCreateSystemDefaultDevice();

    CAMetalLayer *layer = (__bridge CAMetalLayer *)platform_get_metal_layer();
    layer.device = device;
    layer.pixelFormat = MTLPixelFormatBGRA8Unorm;
    layer.framebufferOnly = YES;

    // Create pipeline state
    NSString *shader_src = [NSString stringWithContentsOfFile:@"shader.metallib"
                                                       encoding:NSUTF8StringEncoding
                                                          error:nil];
    id<MTLLibrary> library = [device newLibraryWithSource:shader_src
                                                  options:nil
                                                    error:nil];

    MTLRenderPipelineDescriptor *desc = [[MTLRenderPipelineDescriptor alloc] init];
    desc.vertexFunction = [library newFunctionWithName:@"vertex_main"];
    desc.fragmentFunction = [library newFunctionWithName:@"fragment_main"];
    desc.colorAttachments[0].pixelFormat = MTLPixelFormatBGRA8Unorm;
    desc.colorAttachments[0].blendingEnabled = YES;

    pipeline_state = [device newRenderPipelineStateWithPipelineDescriptor:desc
                                                                    error:nil];
}
```

### 5.2 Metal Shaders

```metal
// shader.metal
#include <metal_stdlib>
using namespace metal;

struct VertexIn {
    float2 pos [[attribute(0)]];
    float2 uv [[attribute(1)]];
    uchar4 color [[attribute(2)]];
};

struct VertexOut {
    float4 pos [[position]];
    float2 uv;
    float4 color;
};

vertex VertexOut vertex_main(
    VertexIn in [[stage_in]],
    constant float3x3 &transform [[buffer(1)]],
    constant float2 &screen_size [[buffer(2)]]
) {
    VertexOut out;

    float3 transformed = transform * float3(in.pos, 1.0);
    float2 clip_pos = (transformed.xy / screen_size) * 2.0 - 1.0;
    out.pos = float4(clip_pos * float2(1, -1), 0.0, 1.0);
    out.uv = in.uv;
    out.color = float4(in.color) / 255.0;

    return out;
}

fragment float4 fragment_main(
    VertexOut in [[stage_in]],
    texture2d<float> tex [[texture(0)]],
    constant int &use_texture [[buffer(0)]]
) {
    constexpr sampler s(mag_filter::linear, min_filter::linear);
    float4 tex_color = use_texture ? tex.sample(s, in.uv) : float4(1.0);
    return tex_color * in.color;
}
```

### 5.3 Draw Quad (Metal)

```objc
void render_draw_quad(quadverts_t *quad, texture_t texture_handle) {
    id<MTLRenderCommandEncoder> encoder = current_encoder;
    [encoder setRenderPipelineState:pipeline_state];
    [encoder setVertexBytes:quad->vertices
                     length:sizeof(vertex_t) * 4
                    atIndex:0];
    [encoder setVertexBytes:&current_transform
                     length:sizeof(mat3_t)
                    atIndex:1];
    [encoder setVertexBytes:&render_state.screen_size
                     length:sizeof(vec2i_t)
                    atIndex:2];

    if (texture_handle.index != RENDER_NO_TEXTURE.index) {
        [encoder setFragmentTexture:texture_handle.mtlTexture atIndex:0];
        [encoder setFragmentBytes:&use_texture_yes length:sizeof(int) atIndex:0];
    } else {
        [encoder setFragmentBytes:&use_texture_no length:sizeof(int) atIndex:0];
    }

    [encoder drawPrimitives:MTLPrimitiveTypeTriangleFan vertexStart:0 vertexCount:4];
    render_state.draw_calls++;
}
```

---

## 6. Software Renderer

### 6.1 Framebuffer Setup

```c
// render_software.c
static rgba_t *framebuffer = NULL;
static vec2i_t framebuffer_size;

void render_backend_init(void) {
    framebuffer_size = vec2i(RENDER_WIDTH, RENDER_HEIGHT);
    framebuffer = malloc(sizeof(rgba_t) * framebuffer_size.x * framebuffer_size.y);
}

rgba_t *platform_get_screenbuffer(int32_t *pitch) {
    *pitch = framebuffer_size.x;
    return framebuffer;
}
```

### 6.2 Draw Quad (Software)

```c
void render_draw_quad(quadverts_t *quad, texture_t texture_handle) {
    // Simple software rasterization
    // (This is a simplified version)

    texture_t *tex = texture_handle.index != RENDER_NO_TEXTURE.index
                     ? &textures[texture_handle.index]
                     : NULL;

    for (int y = 0; y < quad->vertices[0].pos.y; y++) {
        for (int x = 0; x < quad->vertices[0].pos.x; x++) {
            float u = (float)x / quad->vertices[0].pos.x;
            float v = (float)y / quad->vertices[0].pos.y;

            rgba_t color = quad->vertices[0].color;

            if (tex) {
                int tx = u * tex->width;
                int ty = v * tex->height;
                color = tex->pixels[ty * tex->width + tx];
            }

            int px = x + quad->vertices[0].pos.x;
            int py = y + quad->vertices[0].pos.y;

            if (px >= 0 && px < framebuffer_size.x &&
                py >= 0 && py < framebuffer_size.y) {
                framebuffer[py * framebuffer_size.x + px] = color;
            }
        }
    }
}
```

---

## 7. Texture Management

### 7.1 Texture Storage

```c
typedef struct {
    vec2i_t size;
    void *backend_data;  // GL texture ID, MTLTexture, etc.
    bool dirty;          // Needs upload
} texture_t;

static texture_t textures[RENDER_TEXTURES_MAX];
static int texture_count = 0;
```

### 7.2 Mark/Reset Pattern

```c
texture_mark_t textures_mark(void) {
    return (texture_mark_t){.index = texture_count};
}

void textures_reset(texture_mark_t mark) {
    for (int i = mark.index; i < texture_count; i++) {
        // Backend cleanup
        glDeleteTextures(1, &textures[i].backend_data);
    }
    texture_count = mark.index;
}
```

### 7.3 Create Texture

```c
texture_t texture_create(vec2i_t size, rgba_t *pixels) {
    if (texture_count >= RENDER_TEXTURES_MAX) {
        return RENDER_NO_TEXTURE;
    }

    texture_t *tex = &textures[texture_count++];
    tex->size = size;
    tex->dirty = false;

    // OpenGL
    glGenTextures(1, &tex->backend_data);
    glBindTexture(GL_TEXTURE_2D, tex->backend_data);
    glTexImage2D(GL_TEXTURE_2D, 0, GL_RGBA, size.x, size.y, 0,
                 GL_RGBA, GL_UNSIGNED_BYTE, pixels);
    glTexParameteri(GL_TEXTURE_2D, GL_TEXTURE_MIN_FILTER, GL_NEAREST);
    glTexParameteri(GL_TEXTURE_2D, GL_TEXTURE_MAG_FILTER, GL_NEAREST);

    return *tex;
}
```

### 7.4 Image Integration

```c
// image.c
image_t *image(char *path) {
    // Check cache first
    for (int i = 0; i < image_count; i++) {
        if (strcmp(images[i].path, path) == 0) {
            return &images[i];
        }
    }

    // Load QOI file
    uint8_t *data = platform_load_asset(path, &file_size);
    qoi_desc desc;
    rgba_t *pixels = qoi_decode(data, file_size, &desc, 4);

    // Create texture
    image_t *img = &images[image_count++];
    img->texture = texture_create(vec2i(desc.width, desc.height), pixels);
    img->size = vec2i(desc.width, desc.height);

    free(pixels);
    temp_free(data);

    return img;
}
```

---

## 8. Draw Call Pipeline

### 8.1 Frame Flow

```
render_frame_prepare()
    │
    ├─> Clear screen
    ├─> Reset draw call counter
    └─> Begin render pass (Metal) / Clear errors (GL)

For each drawable:
    │
    ├─> render_push()           [optional]
    ├─> render_translate()      [optional]
    ├─> render_scale()          [optional]
    ├─> render_rotate()         [optional]
    ├─> render_draw_quad()      [required]
    └─> render_pop()            [optional]

render_frame_end()
    │
    ├─> End render pass (Metal) / glFlush (GL)
    ├─> Swap buffers
    └─> Reset transform stack
```

### 8.2 Batching

Current implementation: **No batching** - each quad is a separate draw call.

**Optimization opportunity:**
```c
typedef struct {
    quadverts_t quads[MAX_BATCH_QUADS];
    texture_t texture;
    int count;
} batch_t;

void render_draw_quad_batched(quadverts_t *quad, texture_t tex) {
    if (current_batch.texture.index != tex.index ||
        current_batch.count >= MAX_BATCH_QUADS) {
        render_flush_batch();
    }
    current_batch.quads[current_batch.count++] = *quad;
    current_batch.texture = tex;
}
```

---

## 9. Post-Processing Effects

### 9.1 Available Effects

```c
typedef enum {
    RENDER_POST_NONE,
    RENDER_POST_CRT,      // CRT scanline simulation
    RENDER_POST_MAX,
} render_post_effect_t;
```

### 9.2 CRT Effect

```glsl
// Fragment shader with CRT effect
uniform int u_post_effect;

float scanline(float y, float time) {
    return 0.95 + 0.05 * sin(y * 3.14159 * 2.0 + time);
}

void main() {
    vec4 tex_color = texture(u_texture, v_uv);

    if (u_post_effect == RENDER_POST_CRT) {
        // Scanlines
        float scan = scanline(v_uv.y, u_time);
        tex_color.rgb *= scan;

        // Chromatic aberration
        float r = texture(u_texture, v_uv + vec2(0.002, 0)).r;
        float b = texture(u_texture, v_uv - vec2(0.002, 0)).b;
        tex_color.r = r;
        tex_color.b = b;

        // Vignette
        float dist = length(v_uv - 0.5) * 2.0;
        tex_color.rgb *= 1.0 - dist * 0.3;
    }

    frag_color = tex_color * v_color;
}
```

---

## Related Documents

- **[Main Exploration](./exploration.md)** - Overall architecture
- **[Image System](./image-system-deep-dive.md)** - QOI loading, texture creation
- **[Animation System](./animation-system-deep-dive.md)** - Sprite rendering
