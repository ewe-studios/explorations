# Tiny Skies -- Rust Equivalents

This document maps Tiny Skies' TypeScript/Three.js patterns to Rust equivalents, identifying crates and architectural choices for a Rust-based implementation.

## Rendering

### Three.js → wgpu/bevy

| Three.js | Rust Equivalent | Crate |
|----------|----------------|-------|
| WebGLRenderer | `wgpu` | `wgpu` |
| Scene/Object3D | `bevy_ecs` entities | `bevy` |
| Mesh | `Mesh` in bevy | `bevy_render` |
| ShaderMaterial | `Shader` + `Handle<Shader>` | `bevy_pbr` |
| InstancedMesh | `GpuInstances` | `bevy_render` |
| Points | `PointList` rendering | `wgpu` |
| onBeforeCompile | Custom `ShaderDef` | `bevy_pbr` |

```rust
// wgpu: Initialize rendering context
let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::default());
let adapter = instance.request_adapter(&wgpu::RequestAdapterOptions::default()).await?;
let (device, queue) = adapter.request_device(&wgpu::DeviceDescriptor::default()).await?;

// bevy: ECS-based scene graph
App::new()
    .add_plugins(DefaultPlugins)
    .add_systems(Update, (update_terrain, update_vehicles, update_particles))
    .run();
```

### Globe Sphere → Bevy Mesh

```rust
// Procedural globe with vertex displacement
fn build_globe_mesh(
    noise: &SimplexNoise,
    preset: &TerrainPreset,
    segments: usize,
) -> Mesh {
    let mut positions = Vec::new();
    let mut normals = Vec::new();
    let mut colors = Vec::new();

    for phi in 0..segments {
        for theta in 0..segments {
            // Convert spherical to cartesian
            let x = f32::sin(phi as f32) * f32::cos(theta as f32);
            let y = f32::cos(phi as f32);
            let z = f32::sin(phi as f32) * f32::sin(theta as f32);

            // Apply displacement
            let displaced = surface_displacement_at(x, y, z, noise, preset);
            positions.push(displaced);

            // Vertex color based on terrain zone
            colors.push(terrain_color(x, y, z, noise, preset));
        }
    }

    Mesh::new(
        bevy_render::mesh::PrimitiveTopology::TriangleList,
        VertexBufferLayout {
            positions,
            normals,
            colors,
        },
    )
}
```

## Procedural Noise

### Simplex Noise → `noise` crate

```rust
use noise::{NoiseFn, Simplex, Seedable};

struct SimplexNoiseGenerator {
    simplex: Simplex,
}

impl SimplexNoiseGenerator {
    fn new(seed: u32) -> Self {
        Self {
            simplex: Simplex::new(seed),
        }
    }

    fn noise_3d(&self, x: f64, y: f64, z: f64) -> f64 {
        self.simplex.get([x, y, z])
    }

    fn multi_octave(
        &self,
        x: f64, y: f64, z: f64,
        octaves: usize,
        lacunarity: f64,
        persistence: f64,
        scale: f64,
    ) -> f64 {
        let mut value = 0.0;
        let mut amplitude = 1.0;
        let mut frequency = 1.0;
        let mut max_amplitude = 0.0;

        for _ in 0..octaves {
            value += self.noise_3d(
                x * scale * frequency,
                y * scale * frequency,
                z * scale * frequency,
            ) * amplitude;
            max_amplitude += amplitude;
            amplitude *= persistence;
            frequency *= lacunarity;
        }
        value / max_amplitude
    }
}
```

### Park-Miller LCG → `rand` crate

```rust
use rand::{Rng, SeedableRng};
use rand::rngs::StdRng;

fn seeded_random_surface_position(seed: u64) -> (f64, f64, f64) {
    let mut rng = StdRng::seed_from_u64(seed);
    // Marsaglia method for uniform sphere sampling
    let x: f64 = rng.gen_range(-1.0..1.0);
    let y: f64 = rng.gen_range(-1.0..1.0);
    let z: f64 = rng.gen_range(-1.0..1.0);
    let len = (x*x + y*y + z*z).sqrt();
    (x/len, y/len, z/len)
}
```

## Spherical Math

### Quaternions → `nalgebra` or `glam`

```rust
use glam::Quat;
use nalgebra::Vector3;

fn move_on_sphere(
    position: Quat,
    heading: f32,
    arc_angle: f32,
) -> Quat {
    // Convert heading + arc_angle to quaternion rotation
    let rotation = Quat::from_axis_angle(
        Vector3::y_axis(),
        heading,
    );
    let arc_rotation = Quat::from_axis_angle(
        Vector3::z_axis(),
        arc_angle,
    );
    rotation * arc_rotation * position
}

fn cartesian_from_spherical(
    position: Quat,
    altitude: f32,
    globe_radius: f32,
) -> Vector3<f32> {
    let up = position * Vector3::new(0.0, 1.0, 0.0);
    up * (globe_radius + altitude)
}
```

## Networking

### Socket.IO → `tokio-tungstenite`

```rust
use tokio_tungstenite::connect_async;
use futures::StreamExt;

async fn connect_to_server(url: &str) -> Result<(), Error> {
    let (ws_stream, _) = connect_async(url).await?;
    let (write, mut read) = ws_stream.split();

    // Send player state
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_millis(50));
        loop {
            interval.tick().await;
            let state = get_player_state();
            write.send(Message::Text(serde_json::to_string(&state)?)).await?;
        }
    });

    // Receive updates
    while let Some(msg) = read.next().await {
        if let Message::Text(text) = msg? {
            let update: PlayerUpdate = serde_json::from_str(&text)?;
            apply_remote_update(update);
        }
    }
    Ok(())
}
```

### Server → `axum` + `tokio`

```rust
use axum::{Router, extract::ws::WebSocketUpgrade, response::IntoResponse};

async fn websocket_handler(ws: WebSocketUpgrade) -> impl IntoResponse {
    ws.on_upgrade(handle_socket)
}

async fn handle_socket(socket: WebSocket) {
    let (sender, mut receiver) = socket.split();
    let broadcast = Arc::new(tokio::sync::RwLock::new(Vec::new()));

    while let Some(Ok(msg)) = receiver.next().await {
        let state: PlayerState = serde_json::from_str(&msg.to_string())?;
        // Broadcast to all other players in room
        for sender in broadcast.read().await.iter() {
            sender.send(serde_json::to_string(&state)?).await?;
        }
    }
}
```

## Physics

### Vehicle Physics → Custom

```rust
struct Plane {
    position: Quat,
    heading: f32,
    altitude: f32,
    speed: f32,
    bank_angle: f32,
}

impl Plane {
    fn update(&mut self, delta: f32, input: &InputState) {
        // Speed control
        let target_speed = CRUISE_SPEED + input.forward * MAX_SPEED;
        self.speed += (target_speed - self.speed) * 3.0 * delta;

        // Heading
        self.heading += input.turn * self.bank_rate * delta;

        // Elevation
        let target_alt = if input.elevate { CLIMB_ALT } else { CRUISE_ALT };
        self.altitude += (target_alt - self.altitude) * self.elevation_blend_speed * delta;

        // Move on sphere
        self.position = move_on_sphere(self.position, self.heading, self.speed * delta);
    }
}
```

## Database

### Prisma → `sqlx`

```rust
use sqlx::PgPool;

async fn create_world(
    pool: &PgPool,
    slug: &str,
    name: &str,
    seed: i32,
    terrain_type: &str,
) -> Result<World, sqlx::Error> {
    sqlx::query_as!(
        World,
        "INSERT INTO worlds (slug, name, seed, terrain_type) VALUES ($1, $2, $3, $4)
         RETURNING id, slug, name, globe_radius, seed, terrain_type, created_at",
        slug, name, seed, terrain_type,
    )
    .fetch_one(pool)
    .await
}
```

## Audio

### Web Audio API → `cpal` + `rodio`

```rust
use rodio::{Decoder, Sink, Source};
use std::fs::File;

struct AudioManager {
    sink: Sink,
    day_music: rodio::decoder::Decoder<File>,
    evening_music: rodio::decoder::Decoder<File>,
    night_music: rodio::decoder::Decoder<File>,
}

impl AudioManager {
    fn crossfade(&mut self, weights: MusicWeights) {
        // Adjust volume of each track
        self.day_music.set_volume(weights.day);
        self.evening_music.set_volume(weights.evening);
        self.night_music.set_volume(weights.night);
    }
}
```

## Key Rust Crate Recommendations

| Purpose | Crate | Notes |
|---------|-------|-------|
| Rendering | `bevy` | ECS-based, built on wgpu |
| GPU compute | `wgpu` | Low-level GPU API |
| Math | `nalgebra`, `glam` | Quaternions, matrices, vectors |
| Noise | `noise` | Simplex, Perlin, value noise |
| Networking | `tokio-tungstenite` | WebSocket client |
| Server | `axum` | REST + WebSocket server |
| Database | `sqlx` | Compile-time checked SQL |
| Audio | `rodio` | Audio playback |
| Serialization | `serde`, `serde_json` | JSON serialization |
| Async runtime | `tokio` | Async I/O, timers, channels |
| Random | `rand` | PRNG, seeded RNG |

See [Particle Systems](09-particle-systems.md) for GPU particle patterns.
See [Deployment](16-deployment.md) for container deployment.
