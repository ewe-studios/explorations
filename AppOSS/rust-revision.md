---
source: /home/darkvoid/Boxxed/@formulas/src.AppOSS
related_projects: Skia, Penpot, OpenPencil, Rive, tldraw
created_at: 2026-04-02
tags: rust, wasm, graphics, rendering, architecture
---

# AppOSS in Rust: Complete Revision Guide

## Overview

This document provides a comprehensive guide to building Rust equivalents of the AppOSS applications. We cover architecture decisions, crate recommendations, and implementation strategies.

---

## Part 1: Project Structure

### 1.1 Monorepo Layout

```
my-app/
├── Cargo.toml                    # Workspace root
├── crates/
│   ├── core/                     # Shared core logic
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── document.rs       # Document model
│   │       ├── shape.rs          # Shape definitions
│   │       └── transform.rs      # Transform math
│   │
│   ├── renderer/                 # Rendering engine
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── surface.rs        # Render surface
│   │       ├── paint.rs          # Paint/fill handling
│   │       └── rasterizer.rs     # Software rasterizer
│   │
│   ├── wasm/                     # WebAssembly bindings
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       └── bindings.rs       # wasm-bindgen exports
│   │
│   ├── server/                   # Backend API
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── main.rs
│   │       ├── api.rs            # HTTP handlers
│   │       ├── db.rs             # Database layer
│   │       └── websocket.rs      # Real-time sync
│   │
│   └── cli/                      # Command-line tool
│       ├── Cargo.toml
│       └── src/
│           └── main.rs
│
├── frontend/                     # Optional: Tauri or web frontend
│   ├── src/
│   ├── package.json
│   └── vite.config.ts
│
└── tests/
    ├── integration/
    └── e2e/
```

### 1.2 Root Cargo.toml

```toml
[workspace]
resolver = "2"
members = [
    "crates/core",
    "crates/renderer",
    "crates/wasm",
    "crates/server",
    "crates/cli",
]

[workspace.dependencies]
# Internal
core = { path = "crates/core" }
renderer = { path = "crates/renderer" }

# Graphics
skia-safe = "0.70"
vello = "0.2"
kurbo = "0.11"

# Async
tokio = { version = "1", features = ["full"] }
async-trait = "0.1"

# Web
wasm-bindgen = "0.2"
wasm-bindgen-futures = "0.4"
js-sys = "0.3"
web-sys = { version = "0.3", features = ["CanvasRenderingContext2d", "ImageData"] }

# Serialization
serde = { version = "1", features = ["derive"] }
serde_json = "1"

# Database
sqlx = { version = "0.7", features = ["runtime-tokio-rustls", "postgres", "sqlite"] }

# HTTP
axum = "0.7"
tower = "0.4"
tower-http = { version = "0.5", features = ["cors", "trace"] }

# Error handling
thiserror = "1"
anyhow = "1"

# Logging
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter"] }
```

---

## Part 2: Core Data Structures

### 2.1 Document Model

```rust
// crates/core/src/lib.rs
use serde::{Deserialize, Serialize};
use uuid::Uuid;

pub type DocumentId = Uuid;
pub type LayerId = Uuid;
pub type UserId = Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Document {
    pub id: DocumentId,
    pub name: String,
    pub pages: Vec<Page>,
    pub components: Vec<Component>,
    pub variables: VariableCollection,
    pub metadata: DocumentMetadata,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Page {
    pub id: LayerId,
    pub name: String,
    pub layers: Vec<Layer>,
    pub width: f32,
    pub height: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Layer {
    pub id: LayerId,
    pub name: String,
    pub kind: LayerType,
    pub transform: Transform,
    pub style: LayerStyle,
    pub children: Vec<Layer>,
    pub visible: bool,
    pub locked: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LayerType {
    Frame(FrameData),
    Shape(ShapeData),
    Text(TextData),
    Image(ImageData),
    Component(ComponentInstance),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ShapeData {
    Rect { width: f32, height: f32, corner_radius: CornerRadii },
    Ellipse { radius_x: f32, radius_y: f32 },
    Path { path: PathData },
    Line { start: [f32; 2], end: [f32; 2] },
    Polygon { points: Vec<[f32; 2]> },
    Star { inner_radius: f32, outer_radius: f32, points: u32 },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PathData {
    pub commands: Vec<PathCommand>,
    pub winding: WindingRule,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PathCommand {
    MoveTo([f32; 2]),
    LineTo([f32; 2]),
    QuadTo([f32; 2], [f32; 2]),
    CubicTo([f32; 2], [f32; 2], [f32; 2]),
    ArcTo([f32; 2], [f32; 2], f32, bool, bool),
    Close,
}
```

### 2.2 Transform System

```rust
// crates/core/src/transform.rs
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, Default)]
pub struct Transform {
    pub translation: [f32; 2],
    pub rotation: f32,      // radians
    pub scale: [f32; 2],
    pub skew: [f32; 2],
}

impl Transform {
    pub fn identity() -> Self {
        Self {
            translation: [0.0, 0.0],
            rotation: 0.0,
            scale: [1.0, 1.0],
            skew: [0.0, 0.0],
        }
    }
    
    pub fn to_matrix(&self) -> [f32; 6] {
        let cos = self.rotation.cos();
        let sin = self.rotation.sin();
        
        [
            self.scale[0] * cos,
            self.scale[0] * sin,
            self.scale[1] * -sin,
            self.scale[1] * cos,
            self.translation[0],
            self.translation[1],
        ]
    }
    
    pub fn compose(&self, other: &Transform) -> Transform {
        Transform {
            translation: [
                self.translation[0] + other.translation[0],
                self.translation[1] + other.translation[1],
            ],
            rotation: self.rotation + other.rotation,
            scale: [
                self.scale[0] * other.scale[0],
                self.scale[1] * other.scale[1],
            ],
            skew: [
                self.skew[0] + other.skew[0],
                self.skew[1] + other.skew[1],
            ],
        }
    }
    
    pub fn transform_point(&self, point: [f32; 2]) -> [f32; 2] {
        let m = self.to_matrix();
        [
            m[0] * point[0] + m[2] * point[1] + m[4],
            m[1] * point[0] + m[3] * point[1] + m[5],
        ]
    }
}
```

### 2.3 Style System

```rust
// crates/core/src/style.rs
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LayerStyle {
    pub fill: Vec<Fill>,
    pub stroke: Vec<Stroke>,
    pub effects: Vec<Effect>,
    pub opacity: f32,
    pub blend_mode: BlendMode,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Fill {
    Solid(Color),
    Gradient(Gradient),
    Image { id: String, transform: Transform },
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Color {
    pub r: f32,
    pub g: f32,
    pub b: f32,
    pub a: f32,
}

impl Color {
    pub fn from_rgb(r: u8, g: u8, b: u8) -> Self {
        Self {
            r: r as f32 / 255.0,
            g: g as f32 / 255.0,
            b: b as f32 / 255.0,
            a: 1.0,
        }
    }
    
    pub fn from_rgba(r: u8, g: u8, b: u8, a: u8) -> Self {
        Self {
            r: r as f32 / 255.0,
            g: g as f32 / 255.0,
            b: b as f32 / 255.0,
            a: a as f32 / 255.0,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Gradient {
    Linear {
        start: [f32; 2],
        end: [f32; 2],
        stops: Vec<GradientStop>,
    },
    Radial {
        center: [f32; 2],
        radius: f32,
        stops: Vec<GradientStop>,
    },
    Angular {
        center: [f32; 2],
        angle: f32,
        stops: Vec<GradientStop>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GradientStop {
    pub position: f32,
    pub color: Color,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Stroke {
    pub color: Color,
    pub width: f32,
    pub align: StrokeAlign,
    pub cap: StrokeCap,
    pub join: StrokeJoin,
    pub dash_pattern: Option<Vec<f32>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum StrokeAlign {
    Center,
    Inside,
    Outside,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum StrokeCap {
    None,
    Round,
    Square,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum StrokeJoin {
    Miter,
    Round,
    Bevel,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Effect {
    DropShadow { color: Color, offset: [f32; 2], blur: f32, spread: f32 },
    InnerShadow { color: Color, offset: [f32; 2], blur: f32, spread: f32 },
    Blur { radius: f32 },
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum BlendMode {
    Normal,
    Multiply,
    Screen,
    Overlay,
    HardLight,
    SoftLight,
    Difference,
    Exclusion,
    ColorDodge,
    ColorBurn,
}
```

---

## Part 3: Rendering Engine

### 3.1 Using Vello (GPU Renderer)

```rust
// crates/renderer/src/lib.rs
use vello::{
    kurbo::{Rect, Shape},
    peniko::{Brush, Color, Fill, Stroke},
    AaConfig, Renderer, RendererOptions, Scene,
};
use wgpu::{Device, Queue, Surface};

pub struct RenderEngine {
    renderer: Renderer,
    device: Arc<Device>,
    queue: Arc<Queue>,
}

impl RenderEngine {
    pub async fn new(surface: Surface) -> Result<Self, Error> {
        let adapter = wgpu::Adapter::request(
            &wgpu::RequestAdapterOptions::default(),
            wgpu::BackendBit::PRIMARY,
        ).await.unwrap();
        
        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor::default(), None)
            .await?;
        
        let device = Arc::new(device);
        let queue = Arc::new(queue);
        
        let renderer = Renderer::new(
            &device,
            RendererOptions {
                surface_format: Some(surface.get_preferred_format(&adapter).unwrap()),
                use_cpu: false,
                antialiasing_support: AaConfig::Msaa16,
            },
        ).await?;
        
        Ok(Self { renderer, device, queue })
    }
    
    pub fn render(&mut self, scene_data: &SceneData, surface: &mut Surface) {
        let mut scene = Scene::new();
        
        // Render layers
        for layer in &scene_data.layers {
            self.render_layer(&mut scene, layer);
        }
        
        // Render to surface
        let surface_texture = surface.get_current_texture().unwrap();
        self.renderer
            .render_to_surface(&self.device, &self.queue, &scene, &surface_texture, &vello::RenderParams::default())
            .unwrap();
        
        surface_texture.present();
    }
    
    fn render_layer(&self, scene: &mut Scene, layer: &Layer) {
        if !layer.visible {
            return;
        }
        
        let transform = layer.transform.to_kurbo();
        scene.push_transform(&transform);
        
        match &layer.kind {
            LayerType::Shape(shape) => {
                self.render_shape(scene, shape, &layer.style);
            }
            LayerType::Text(text) => {
                self.render_text(scene, text, &layer.style);
            }
            LayerType::Frame(frame) => {
                for child in &frame.children {
                    self.render_layer(scene, child);
                }
            }
            _ => {}
        }
        
        scene.pop_transform();
    }
    
    fn render_shape(&self, scene: &mut Scene, shape: &ShapeData, style: &LayerStyle) {
        let path = shape.to_bez_path();
        
        // Fill
        for fill in &style.fill {
            if let Fill::Solid(color) = fill {
                let brush = Brush::Solid(Color::rgba8(
                    (color.r * 255.0) as u8,
                    (color.g * 255.0) as u8,
                    (color.b * 255.0) as u8,
                    (color.a * 255.0) as u8,
                ));
                scene.fill(Fill::NonZero, &path, &brush, None, None);
            }
        }
        
        // Stroke
        for stroke in &style.stroke {
            let brush = Brush::Solid(Color::rgba8(
                (stroke.color.r * 255.0) as u8,
                (stroke.color.g * 255.0) as u8,
                (stroke.color.b * 255.0) as u8,
                (stroke.color.a * 255.0) as u8,
            ));
            
            let stroke_style = Stroke::new(stroke.width);
            scene.stroke(&stroke_style, &path, &brush, None, None);
        }
    }
}
```

### 3.2 Using Skia (CPU/GPU)

```rust
// crates/renderer/src/skia_renderer.rs
use skia_safe::{
    canvas::Canvas,
    gpu::{self, surfaces::wrap_backend_render_target},
    paint::Paint,
    path::Path,
    surfaces::raster_n32_premul,
    Color, Font, Surface, TextBlob,
};

pub struct SkiaRenderer {
    surface: Option<Surface>,
    paint: Paint,
}

impl SkiaRenderer {
    pub fn new(width: i32, height: i32) -> Self {
        let surface = raster_n32_premul((width, height))
            .expect("Failed to create surface");
        
        SkiaRenderer {
            surface: Some(surface),
            paint: Paint::default(),
        }
    }
    
    pub fn canvas(&mut self) -> Option<&mut Canvas> {
        self.surface.as_mut().map(|s| s.canvas())
    }
    
    pub fn draw_rect(&mut self, rect: Rect, color: Color) {
        if let Some(canvas) = self.canvas() {
            self.paint.set_color(color);
            canvas.draw_rect(rect, &self.paint);
        }
    }
    
    pub fn draw_path(&mut self, path: &Path, color: Color, style: PaintStyle) {
        if let Some(canvas) = self.canvas() {
            self.paint.set_color(color);
            self.paint.set_style(style);
            canvas.draw_path(path, &self.paint);
        }
    }
    
    pub fn draw_text(&mut self, text: &str, x: f32, y: f32, font: &Font, color: Color) {
        if let Some(canvas) = self.canvas() {
            let blob = TextBlob::from_str(text, font).unwrap();
            self.paint.set_color(color);
            canvas.draw_text_blob(blob, (x, y), &self.paint);
        }
    }
    
    pub fn flush(&mut self) -> Vec<u8> {
        if let Some(surface) = &self.surface {
            surface.flush();
            let image_info = surface.image_info();
            let pixels = surface.peek_pixels().unwrap();
            pixels.read_pixels().unwrap()
        } else {
            vec![]
        }
    }
}
```

### 3.3 Software Rasterizer (for WASM)

```rust
// crates/renderer/src/software.rs
pub struct SoftwareRasterizer {
    width: u32,
    height: u32,
    pixels: Vec<u32>, // RGBA
}

impl SoftwareRasterizer {
    pub fn new(width: u32, height: u32) -> Self {
        Self {
            width,
            height,
            pixels: vec![0; (width * height) as usize],
        }
    }
    
    pub fn clear(&mut self, color: Color) {
        let rgba = color.to_u32();
        self.pixels.fill(rgba);
    }
    
    pub fn draw_pixel(&mut self, x: u32, y: u32, color: Color) {
        if x < self.width && y < self.height {
            let idx = (y * self.width + x) as usize;
            self.pixels[idx] = color.to_u32();
        }
    }
    
    pub fn draw_line(&mut self, x0: i32, y0: i32, x1: i32, y1: i32, color: Color) {
        let mut dx = (x1 - x0).abs();
        let mut dy = (y1 - y0).abs();
        let sx = if x0 < x1 { 1 } else { -1 };
        let sy = if y0 < y1 { 1 } else { -1 };
        let mut err = (if dx > dy { dx } else { dy }) / 2;
        
        let mut x = x0;
        let mut y = y0;
        
        loop {
            if x >= 0 && x < self.width as i32 && y >= 0 && y < self.height as i32 {
                self.draw_pixel(x as u32, y as u32, color);
            }
            
            if x == x1 && y == y1 {
                break;
            }
            
            let e2 = err;
            if e2 >= -dx {
                err -= dy;
                x += sx;
            }
            if e2 <= dy {
                err += dx;
                y += sy;
            }
        }
    }
    
    pub fn get_pixels(&self) -> &[u32] {
        &self.pixels
    }
}

impl Color {
    fn to_u32(&self) -> u32 {
        ((self.a as u32) << 24) | ((self.r as u32) << 16) | ((self.g as u32) << 8) | (self.b as u32)
    }
}
```

---

## Part 4: Backend Server

### 4.1 Axum API Server

```rust
// crates/server/src/main.rs
use axum::{
    extract::{Path, State},
    http::StatusCode,
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use std::sync::Arc;

#[derive(Clone)]
struct AppState {
    db: PgPool,
}

#[derive(Serialize, Deserialize)]
struct CreateDocumentRequest {
    name: String,
    width: f32,
    height: f32,
}

#[derive(Serialize, Deserialize)]
struct DocumentResponse {
    id: String,
    name: String,
    created_at: chrono::DateTime<chrono::Utc>,
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();
    
    let database_url = std::env::var("DATABASE_URL")
        .expect("DATABASE_URL must be set");
    
    let pool = PgPool::connect(&database_url)
        .await
        .expect("Failed to connect to database");
    
    let state = AppState { db: pool };
    
    let app = Router::new()
        .route("/api/documents", post(create_document))
        .route("/api/documents/:id", get(get_document))
        .route("/api/documents/:id", put(update_document))
        .route("/api/documents/:id", delete(delete_document))
        .with_state(state);
    
    let addr = "0.0.0.0:3000";
    tracing::info!("Listening on {}", addr);
    
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

async fn create_document(
    State(state): State<AppState>,
    Json(req): Json<CreateDocumentRequest>,
) -> Result<Json<DocumentResponse>, StatusCode> {
    let id = uuid::Uuid::new_v4().to_string();
    
    sqlx::query!(
        r#"
        INSERT INTO documents (id, name, width, height)
        VALUES ($1, $2, $3, $4)
        "#,
        id,
        req.name,
        req.width,
        req.height,
    )
    .execute(&state.db)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    
    Ok(Json(DocumentResponse {
        id,
        name: req.name,
        created_at: chrono::Utc::now(),
    }))
}

async fn get_document(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<DocumentResponse>, StatusCode> {
    let doc = sqlx::query_as!(
        DocumentResponse,
        r#"
        SELECT id, name, created_at
        FROM documents
        WHERE id = $1
        "#,
        id,
    )
    .fetch_one(&state.db)
    .await
    .map_err(|_| StatusCode::NOT_FOUND)?;
    
    Ok(Json(doc))
}
```

### 4.2 WebSocket for Real-time Collaboration

```rust
// crates/server/src/websocket.rs
use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        State,
    },
    response::IntoResponse,
};
use futures::{sink::SinkExt, stream::StreamExt};
use tokio::sync::broadcast;

pub struct CollaborationState {
    sender: broadcast::Sender<Operation>,
}

#[derive(Clone, serde::Serialize, serde::Deserialize)]
pub enum Operation {
    CreateLayer { id: String, parent: String, data: LayerData },
    UpdateLayer { id: String, changes: LayerChanges },
    DeleteLayer { id: String },
    SelectLayer { user_id: String, layer_id: Option<String> },
    CursorMove { user_id: String, x: f32, y: f32 },
}

pub async fn ws_handler(
    ws: WebSocketUpgrade,
    State(state): State<Arc<CollaborationState>>,
) -> impl IntoResponse {
    ws.on_upgrade(|socket| handle_socket(socket, state))
}

async fn handle_socket(socket: WebSocket, state: Arc<CollaborationState>) {
    let (mut sender, mut receiver) = socket.split();
    
    // Subscribe to broadcasts
    let mut rx = state.sender.subscribe();
    
    // Spawn task to receive from broadcast
    let send_task = tokio::spawn(async move {
        while let Ok(msg) = rx.recv().await {
            let json = serde_json::to_string(&msg).unwrap();
            if sender.send(Message::Text(json.into())).await.is_err() {
                break;
            }
        }
    });
    
    // Receive from socket and broadcast
    let broadcast_tx = state.sender.clone();
    while let Some(Ok(msg)) = receiver.next().await {
        if let Message::Text(text) = msg {
            if let Ok(op) = serde_json::from_str::<Operation>(&text) {
                let _ = broadcast_tx.send(op);
            }
        }
    }
    
    send_task.abort();
}
```

---

## Part 5: WASM Bindings

### 5.1 wasm-bindgen Setup

```rust
// crates/wasm/src/lib.rs
use wasm_bindgen::prelude::*;
use core::{Document, Layer, Transform};
use renderer::RenderEngine;

#[wasm_bindgen]
pub struct WasmApp {
    document: Document,
    renderer: RenderEngine,
}

#[wasm_bindgen]
impl WasmApp {
    #[wasm_bindgen(constructor)]
    pub fn new(width: u32, height: u32) -> Result<WasmApp, JsValue> {
        console_error_panic_hook::set_once();
        
        let document = Document::default();
        let renderer = RenderEngine::new(width, height)
            .map_err(|e| JsValue::from_str(&e.to_string()))?;
        
        Ok(WasmApp { document, renderer })
    }
    
    pub fn create_rect(&mut self, x: f32, y: f32, width: f32, height: f32) -> String {
        let id = uuid::Uuid::new_v4().to_string();
        
        let layer = Layer::new_rect(x, y, width, height);
        self.document.add_layer(layer);
        
        id
    }
    
    pub fn render(&mut self) -> Vec<u8> {
        self.renderer.render(&self.document)
    }
    
    pub fn apply_operation(&mut self, json: &str) -> Result<(), JsValue> {
        let op: Operation = serde_json::from_str(json)
            .map_err(|e| JsValue::from_str(&e.to_string()))?;
        
        self.document.apply(op);
        Ok(())
    }
}

#[wasm_bindgen(start)]
pub fn main() -> Result<(), JsValue> {
    Ok(())
}
```

### 5.2 JavaScript API

```typescript
// Generated by wasm-pack
import init, { WasmApp } from './app_wasm.js';

export class App {
    private wasm: WasmApp;
    
    static async create(width: number, height: number): Promise<App> {
        await init();
        const wasm = new WasmApp(width, height);
        return new App(wasm);
    }
    
    constructor(wasm: WasmApp) {
        this.wasm = wasm;
    }
    
    createRect(x: number, y: number, width: number, height: number): string {
        return this.wasm.create_rect(x, y, width, height);
    }
    
    render(): ImageData {
        const pixels = this.wasm.render();
        return new ImageData(
            new Uint8ClampedArray(pixels),
            800,
            600
        );
    }
}
```

---

## Part 6: CLI Tool

```rust
// crates/cli/src/main.rs
use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "myapp")]
#[command(about = "Design tool CLI")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Create a new document
    New {
        #[arg(short, long)]
        name: String,
        #[arg(short, long, default_value = "800")]
        width: f32,
        #[arg(short, long, default_value = "600")]
        height: f32,
    },
    
    /// Export document to PNG
    Export {
        #[arg(short, long)]
        input: PathBuf,
        #[arg(short, long)]
        output: PathBuf,
        #[arg(short, long, default_value = "2")]
        scale: f32,
    },
    
    /// Inspect document
    Inspect {
        #[arg(short, long)]
        input: PathBuf,
    },
}

fn main() {
    let cli = Cli::parse();
    
    match cli.command {
        Commands::New { name, width, height } => {
            cmd_new(&name, width, height);
        }
        Commands::Export { input, output, scale } => {
            cmd_export(&input, &output, scale);
        }
        Commands::Inspect { input } => {
            cmd_inspect(&input);
        }
    }
}

fn cmd_new(name: &str, width: f32, height: f32) {
    let doc = Document::new(name, width, height);
    let json = serde_json::to_string_pretty(&doc).unwrap();
    println!("{}", json);
}

fn cmd_export(input: &PathBuf, output: &PathBuf, scale: f32) {
    // Load document
    let content = std::fs::read_to_string(input).unwrap();
    let doc: Document = serde_json::from_str(&content).unwrap();
    
    // Render
    let mut renderer = RenderEngine::new((width * scale) as u32, (height * scale) as u32);
    renderer.render(&doc);
    
    // Save PNG
    let image = renderer.get_image();
    image.save_png(output).unwrap();
}

fn cmd_inspect(input: &PathBuf) {
    let content = std::fs::read_to_string(input).unwrap();
    let doc: Document = serde_json::from_str(&content).unwrap();
    
    println!("Document: {}", doc.name);
    println!("Pages: {}", doc.pages.len());
    println!("Layers: {}", count_layers(&doc));
}

fn count_layers(doc: &Document) -> usize {
    doc.pages.iter()
        .flat_map(|p| count_layers_recursive(&p.layers))
        .sum()
}
```

---

## Summary

Key crates and libraries for building AppOSS in Rust:

| Category | Crate | Purpose |
|----------|-------|---------|
| Graphics | `vello` | GPU rendering |
| Graphics | `skia-safe` | Skia bindings |
| Geometry | `kurbo` | 2D geometry |
| Geometry | `lyon` | Path tessellation |
| WASM | `wasm-bindgen` | JS interop |
| WASM | `wasm-pack` | Build tool |
| Server | `axum` | HTTP framework |
| Server | `tokio` | Async runtime |
| Database | `sqlx` | Database access |
| CLI | `clap` | Argument parsing |
| Serialization | `serde` | JSON/serde |
| Errors | `thiserror` | Error types |

Building a production Rust equivalent requires:
1. Start with core data structures
2. Add rendering (Vello for GPU, Skia for compatibility)
3. Build server with Axum
4. Add WASM bindings for web
5. Create CLI for automation
