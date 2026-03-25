# iOS Implementation - Spline Runtime

**Source:** `/home/darkvoid/Boxxed/@formulas/src.rust/src.Spline3d/spline-ios/`

This document covers the iOS/macOS implementation of Spline runtime using Metal and native frameworks.

---

## Table of Contents

1. [Overview](#overview)
2. [Framework Structure](#framework-structure)
3. [Metal Rendering](#metal-rendering)
4. [Touch Interactions](#touch-interactions)
5. [Swift Integration](#swift-integration)
6. [Performance Optimization](#performance-optimization)
7. [Rust Mobile Rendering](#rust-mobile-rendering)

---

## Overview

### SplineRuntime.xcframework

The Spline iOS runtime is distributed as an XCFramework supporting multiple platforms:

```
SplineRuntime.xcframework/
├── ios-arm64/                    # iOS devices (iPhone, iPad)
│   └── SplineRuntime.framework/
│       ├── Headers/
│       ├── Modules/
│       ├── PrivateHeaders/
│       └── SplineRuntime (binary)
│
├── ios-arm64-simulator/          # iOS Simulator (Apple Silicon)
│   └── SplineRuntime.framework/
│
├── ios-arm64_x86_64-maccatalyst/ # Mac Catalyst
│   └── SplineRuntime.framework/
│
├── macos-arm64_x86_64/           # macOS (Intel + Apple Silicon)
│   └── SplineRuntime.framework/
│
├── xros-arm64/                   # visionOS (Apple Vision Pro)
│   └── SplineRuntime.framework/
│
└── xros-arm64-simulator/         # visionOS Simulator
    └── SplineRuntime.framework/
```

### Platform Requirements

| Platform | Minimum Version | Architecture |
|----------|-----------------|--------------|
| iOS | 16.0 | arm64 |
| macOS | 13.0 | arm64, x86_64 |
| Mac Catalyst | 16.0 | arm64, x86_64 |
| visionOS | 1.0 | arm64 |

### Package.swift

```swift
// swift-tools-version:5.9
import PackageDescription

let package = Package(
    name: "SplineRuntime",
    platforms: [
        .iOS("16.0"),
        .macCatalyst("16.0"),
        .macOS("13.0"),
        .visionOS("1.0")
    ],
    products: [
        .library(
            name: "SplineRuntime",
            targets: ["SplineRuntime"]
        ),
    ],
    targets: [
        .binaryTarget(
            name: "SplineRuntime",
            path: "SplineRuntime.xcframework"
        ),
    ]
)
```

---

## Framework Structure

### Public Headers

```objc
// SplineRuntime-Swift.h (Auto-generated)
#import <Foundation/Foundation.h>

//! Project version number for SplineRuntime.
FOUNDATE_EXPORT double SplineRuntimeVersionNumber;

//! Project version string for SplineRuntime.
FOUNDATE_EXPORT const unsigned char SplineRuntimeVersionString[];

// Swift module header
#import <SplineRuntime/SplineRuntime-Swift.h>
```

### Internal Structure (Inferred)

```
SplineRuntime (Binary)
├── Scene Loading
│   ├── .splinecode parser
│   ├── Binary decoder
│   └── Asset extraction
│
├── Rendering Engine
│   ├── Metal pipeline
│   ├── Shader library
│   ├── Texture management
│   └── Geometry processing
│
├── Interaction Layer
│   ├── Touch handling
│   ├── Gesture recognizers
│   ├── Ray casting
│   └── Hit testing
│
├── Animation System
│   ├── Keyframe interpolation
│   ├── Timeline management
│   └── Easing functions
│
└── Platform Abstraction
    ├── iOS/UIKit
    ├── macOS/AppKit
    └── visionOS/RealityKit
```

---

## Metal Rendering

### Metal Pipeline Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                    Metal Rendering Pipeline                      │
│                                                                  │
│  ┌──────────────────────────────────────────────────────────┐   │
│  │                   Application Layer                       │   │
│  │  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐       │   │
│  │  │   Spline    │  │   Scene     │  │   Gesture   │       │   │
│  │  │   Runtime   │  │   Graph     │  │   Handler   │       │   │
│  │  └─────────────┘  └─────────────┘  └─────────────┘       │   │
│  └──────────────────────────────────────────────────────────┘   │
│                              │                                   │
│                              ▼                                   │
│  ┌──────────────────────────────────────────────────────────┐   │
│  │                   Metal Layer                             │   │
│  │  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐       │   │
│  │  │  MTLDevice  │  │  MTLCommand  │  │  MTLRender  │       │   │
│  │  │  (GPU)      │  │  Queue      │  │  Pass       │       │   │
│  │  └─────────────┘  └─────────────┘  └─────────────┘       │   │
│  └──────────────────────────────────────────────────────────┘   │
│                              │                                   │
│                              ▼                                   │
│  ┌──────────────────────────────────────────────────────────┐   │
│  │                  Pipeline States                          │   │
│  │  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐       │   │
│  │  │  Vertex     │  │  Fragment   │  │  Depth/     │       │   │
│  │  │  Pipeline   │  │  Pipeline   │  │  Stencil    │       │   │
│  │  └─────────────┘  └─────────────┘  └─────────────┘       │   │
│  └──────────────────────────────────────────────────────────┘   │
│                              │                                   │
│                              ▼                                   │
│  ┌──────────────────────────────────────────────────────────┐   │
│  │                   Resources                               │   │
│  │  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐       │   │
│  │  │  Buffers    │  │  Textures   │  │  Samplers   │       │   │
│  │  │  (Vertex/   │  │  (Diffuse/  │  │  (Filter/   │       │   │
│  │  │   Uniform)  │  │   Normal)   │  │   Wrap)     │       │   │
│  │  └─────────────┘  └─────────────┘  └─────────────┘       │   │
│  └──────────────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────────────┘
```

### Metal Rendering Setup

```swift
import Metal
import MetalKit

class SplineRenderer: NSObject, MTKViewDelegate {
    let device: MTLDevice
    let commandQueue: MTLCommandQueue
    let pipelineState: MTLRenderPipelineState
    let depthState: MTLDepthStencilState

    var scene: SplineScene?
    var vertexBuffer: MTLBuffer?
    var uniformBuffer: MTLBuffer!

    init?(metalView: MTKView, scene: SplineScene) {
        self.scene = scene

        guard let device = metalView.device,
              let commandQueue = device.makeCommandQueue() else {
            return nil
        }

        self.device = device
        self.commandQueue = commandQueue

        // Create pipeline
        let library = device.makeDefaultLibrary()!
        let pipelineDescriptor = MTLRenderPipelineDescriptor()
        pipelineDescriptor.vertexFunction = library.makeFunction(name: "vertexShader")
        pipelineDescriptor.fragmentFunction = library.makeFunction(name: "fragmentShader")
        pipelineDescriptor.colorAttachments[0].pixelFormat = metalView.colorPixelFormat
        pipelineDescriptor.depthAttachmentPixelFormat = .depth32Float

        self.pipelineState = try! device.makeRenderPipelineState(descriptor: pipelineDescriptor)

        // Create depth state
        let depthDescriptor = MTLDepthStencilDescriptor()
        depthDescriptor.depthCompareFunction = .less
        depthDescriptor.isDepthWriteEnabled = true
        self.depthState = device.makeDepthStencilState(descriptor: depthDescriptor)

        // Create uniform buffer
        let uniformBufferSize = MemoryLayout<Uniforms>.stride
        self.uniformBuffer = device.makeBuffer(length: uniformBufferSize, options: .storageModeShared)

        super.init()
        metalView.delegate = self
    }

    func mtkView(_ view: MTKView, drawableSizeWillChange size: CGSize) {
        // Handle resize
    }

    func draw(in view: MTKView) {
        guard let drawable = view.currentDrawable,
              let descriptor = view.currentRenderPassDescriptor,
              let commandBuffer = commandQueue.makeCommandBuffer() else {
            return
        }

        // Update uniforms
        var uniforms = Uniforms(
            modelMatrix: scene?.transform ?? matrix_identity_float4x4,
            viewMatrix: camera.viewMatrix,
            projectionMatrix: camera.projectionMatrix
        )
        memcpy(uniformBuffer.contents(), &uniforms, MemoryLayout<Uniforms>.stride)

        // Render
        guard let renderEncoder = commandBuffer.makeRenderCommandEncoder(descriptor: descriptor) else {
            return
        }

        renderEncoder.setRenderPipelineState(pipelineState)
        renderEncoder.setDepthStencilState(depthState)
        renderEncoder.setVertexBytes(&uniforms, length: MemoryLayout<Uniforms>.stride, index: 1)

        if let vertexBuffer = vertexBuffer {
            renderEncoder.setVertexBuffer(vertexBuffer, offset: 0, index: 0)
            renderEncoder.drawPrimitives(type: .triangle, vertexStart: 0, vertexCount: vertexCount)
        }

        renderEncoder.endEncoding()
        commandBuffer.present(drawable)
        commandBuffer.commit()
    }
}

struct Uniforms {
    var modelMatrix: matrix_float4x4
    var viewMatrix: matrix_float4x4
    var projectionMatrix: matrix_float4x4
}
```

### Metal Shaders

```metal
// SplineShaders.metal
#include <metal_stdlib>
using namespace metal;

struct VertexIn {
    float3 position [[attribute(0)]];
    float3 normal [[attribute(1)]];
    float2 texCoord [[attribute(2)]];
};

struct VertexOut {
    float4 position [[position]];
    float3 worldPosition;
    float3 normal;
    float2 texCoord;
};

struct Uniforms {
    float4x4 modelMatrix;
    float4x4 viewMatrix;
    float4x4 projectionMatrix;
};

vertex VertexOut vertexShader(
    const VertexIn in [[stage_in]],
    constant Uniforms& uniforms [[buffer(1)]]
) {
    VertexOut out;

    float4 worldPos = uniforms.modelMatrix * float4(in.position, 1.0);
    out.worldPosition = worldPos.xyz;
    out.position = uniforms.projectionMatrix * uniforms.viewMatrix * worldPos;

    // Transform normal
    float3x3 normalMatrix = float3x3(uniforms.modelMatrix[0].xyz,
                                      uniforms.modelMatrix[1].xyz,
                                      uniforms.modelMatrix[2].xyz);
    out.normal = normalize(normalMatrix * in.normal);
    out.texCoord = in.texCoord;

    return out;
}

fragment float4 fragmentShader(
    VertexOut in [[stage_in]],
    texture2d<float> diffuseTexture [[texture(0)]],
    sampler textureSampler [[sampler(0)]]
) {
    // PBR lighting
    float3 N = normalize(in.normal);
    float3 L = normalize(float3(1.0, 1.0, 1.0)); // Light direction
    float3 V = normalize(-in.worldPosition); // View direction (assuming camera at origin)

    // Diffuse
    float diff = max(dot(N, L), 0.0);

    // Specular (Blinn-Phong)
    float3 H = normalize(L + V);
    float spec = pow(max(dot(N, H), 0.0), 32.0);

    // Sample texture
    float4 texColor = diffuseTexture.sample(textureSampler, in.texCoord);

    // Final color
    float3 ambient = 0.1 * texColor.rgb;
    float3 diffuse = diff * texColor.rgb;
    float3 specular = spec * float3(0.3);

    return float4(ambient + diffuse + specular, texColor.a);
}
```

---

## Touch Interactions

### Gesture Handling

```swift
import UIKit
import MetalKit

class SplineView: MTKView {
    var splineScene: SplineScene?
    var lastTouchLocation: CGPoint?

    // MARK: - Touch Handling

    override func touchesBegan(_ touches: Set<UITouch>, with event: UIEvent?) {
        guard let touch = touches.first else { return }
        let location = touch.location(in: self)
        lastTouchLocation = location

        // Raycast to find hit objects
        if let hitObject = hitTest(location) {
            splineScene?.selectedObject = hitObject
            delegate?.splineView?(self, didSelectObject: hitObject)
        }
    }

    override func touchesMoved(_ touches: Set<UITouch>, with event: UIEvent?) {
        guard let touch = touches.first else { return }
        let location = touch.location(in: self)
        let previousLocation = lastTouchLocation ?? location

        let deltaX = location.x - previousLocation.x
        let deltaY = location.y - previousLocation.y

        // Rotate camera based on drag
        if let selectedObject = splineScene?.selectedObject {
            rotateCamera(deltaX: deltaX, deltaY: deltaY)
        } else {
            // Orbit camera
            orbitCamera(deltaX: deltaX, deltaY: deltaY)
        }

        lastTouchLocation = location
    }

    override func touchesEnded(_ touches: Set<UITouch>, with event: UIEvent?) {
        lastTouchLocation = nil
    }

    // MARK: - Gesture Recognizers

    @objc func handlePinch(_ gesture: UIPinchGestureRecognizer) {
        // Zoom camera
        camera.zoom(scale: gesture.scale)
        gesture.scale = 1.0
    }

    @objc func handleRotation(_ gesture: UIRotationGestureRecognizer) {
        // Rotate scene
        splineScene?.rotation.z += gesture.rotation
        gesture.rotation = 0.0
    }

    @objc func handleTap(_ gesture: UITapGestureRecognizer) {
        let location = gesture.location(in: self)

        if let hitObject = hitTest(location) {
            // Trigger tap event
            splineScene?.emitEvent(.tap, on: hitObject)
        }
    }

    // MARK: - Hit Testing

    func hitTest(_ point: CGPoint) -> SplineObject? {
        guard let scene = splineScene else { return nil }

        // Convert screen point to ray
        let ray = camera.ray(from: point, viewport: bounds.size)

        // Test against all objects
        for object in scene.objects {
            if let intersection = ray.intersects(object.bounds) {
                return object
            }
        }

        return nil
    }

    // MARK: - Setup

    private func setupGestures() {
        let pinchGesture = UIPinchGestureRecognizer(target: self, action: #selector(handlePinch))
        addGestureRecognizer(pinchGesture)

        let rotationGesture = UIRotationGestureRecognizer(target: self, action: #selector(handleRotation))
        addGestureRecognizer(rotationGesture)

        let tapGesture = UITapGestureRecognizer(target: self, action: #selector(handleTap))
        addGestureRecognizer(tapGesture)
    }
}
```

### Ray Casting

```swift
import simd

struct Ray {
    var origin: SIMD3<Float>
    var direction: SIMD3<Float>
}

extension Ray {
    func intersects(_ sphere: Sphere) -> Bool {
        let oc = origin - sphere.center
        let a = dot(direction, direction)
        let b = dot(oc, direction)
        let c = dot(oc, oc) - sphere.radius * sphere.radius

        let discriminant = b * b - a * c
        return discriminant > 0
    }

    func intersects(_ box: BoundingBox) -> Bool {
        let invDir = SIMD3<Float>(1.0) / direction

        let t1 = (box.min.x - origin.x) * invDir.x
        let t2 = (box.max.x - origin.x) * invDir.x
        let t3 = (box.min.y - origin.y) * invDir.y
        let t4 = (box.max.y - origin.y) * invDir.y
        let t5 = (box.min.z - origin.z) * invDir.z
        let t6 = (box.max.z - origin.z) * invDir.z

        let tmin = max(max(min(t1, t2), min(t3, t4)), min(t5, t6))
        let tmax = min(min(max(t1, t2), max(t3, t4)), max(t5, t6))

        return tmax >= max(tmin, 0)
    }
}

// Möller–Trumbore triangle intersection
func rayTriangleIntersection(
    ray: Ray,
    v0: SIMD3<Float>,
    v1: SIMD3<Float>,
    v2: SIMD3<Float>
) -> (hit: Bool, t: Float, u: Float, v: Float) {

    let edge1 = v1 - v0
    let edge2 = v2 - v0
    let h = cross(ray.direction, edge2)
    let a = dot(edge1, h)

    if abs(a) < 0.0001 {
        return (false, 0, 0, 0)
    }

    let f = 1.0 / a
    let s = ray.origin - v0
    let u = f * dot(s, h)

    if u < 0 || u > 1 {
        return (false, 0, 0, 0)
    }

    let q = cross(s, edge1)
    let v = f * dot(ray.direction, q)

    if v < 0 || u + v > 1 {
        return (false, 0, 0, 0)
    }

    let t = f * dot(edge2, q)

    if t > 0.0001 {
        return (true, t, u, v)
    }

    return (false, 0, 0, 0)
}
```

---

## Swift Integration

### Basic Usage

```swift
import UIKit
import SplineRuntime

class ViewController: UIViewController {
    var splineView: SplineView!

    override func viewDidLoad() {
        super.viewDidLoad()

        // Create spline view
        splineView = SplineView(frame: view.bounds)
        splineView.autoresizingMask = [.flexibleWidth, .flexibleHeight]
        view.addSubview(splineView)

        // Load scene
        splineView.loadScene(url: URL(string: "https://prod.spline.design/xxx/scene.splinecode")!) { result in
            switch result {
            case .success:
                print("Scene loaded successfully")
            case .failure(let error):
                print("Error loading scene: \(error)")
            }
        }

        // Set delegate
        splineView.delegate = self
    }
}

extension ViewController: SplineViewDelegate {
    func splineView(_ splineView: SplineView, didSelectObject object: SplineObject) {
        print("Selected: \(object.name)")
    }

    func splineView(_ splineView: SplineView, didTriggerEvent event: SplineEvent) {
        print("Event triggered: \(event.type)")
    }
}
```

### SwiftUI Integration

```swift
import SwiftUI
import SplineRuntime

struct SplineSceneView: UIViewRepresentable {
    let sceneURL: URL

    func makeUIView(context: Context) -> SplineView {
        let splineView = SplineView()
        splineView.loadScene(url: sceneURL)
        return splineView
    }

    func updateUIView(_ uiView: SplineView, context: Context) {
        // Update scene if needed
    }
}

// Usage
struct ContentView: View {
    var body: some View {
        SplineSceneView(
            sceneURL: URL(string: "https://prod.spline.design/xxx/scene.splinecode")!
        )
    }
}
```

### Object Manipulation

```swift
// Find object
if let cube = splineView.findObject(named: "Cube") {
    // Modify position
    cube.position = SIMD3<Float>(10, 0, 0)

    // Modify rotation
    cube.rotation = SIMD3<Float>(0, .pi / 4, 0)

    // Modify scale
    cube.scale = SIMD3<Float>(2, 2, 2)

    // Modify material
    cube.material.color = .red
    cube.material.roughness = 0.5
    cube.material.metalness = 0.8
}

// Animate
UIView.animate(withDuration: 1.0) {
    cube.position.x += 5
    splineView.setNeedsDisplay()
}
```

---

## Performance Optimization

### Metal Best Practices

```swift
// 1. Reuse pipeline states
class PipelineCache {
    static let shared = PipelineCache()
    private var pipelines: [String: MTLRenderPipelineState] = [:]

    func getPipeline(
        device: MTLDevice,
        descriptor: MTLRenderPipelineDescriptor
    ) -> MTLRenderPipelineState {
        let key = descriptor.hashValue.description

        if let cached = pipelines[key] {
            return cached
        }

        let pipeline = try! device.makeRenderPipelineState(descriptor: descriptor)
        pipelines[key] = pipeline
        return pipeline
    }
}

// 2. Use buffer recycling
class BufferPool {
    let device: MTLDevice
    var availableBuffers: [MTLBuffer] = []

    init(device: MTLDevice, count: Int, length: Int) {
        self.device = device
        for _ in 0..<count {
            availableBuffers.append(device.makeBuffer(length: length)!)
        }
    }

    func acquire() -> MTLBuffer? {
        return availableBuffers.popLast()
    }

    func release(_ buffer: MTLBuffer) {
        availableBuffers.append(buffer)
    }
}

// 3. Batch draw calls
func render(scene: SplineScene, commandBuffer: MTLCommandBuffer) {
    // Group objects by material
    let grouped = Dictionary(grouping: scene.objects) { $0.material }

    for (material, objects) in grouped {
        // Set material properties once
        encoder.setFragmentBytes(&material.uniforms, length: ...)

        // Draw all objects with same material
        for object in objects {
            encoder.setVertexBuffer(object.vertexBuffer, offset: 0, index: 0)
            encoder.drawPrimitives(type: .triangle, vertexStart: 0, vertexCount: object.vertexCount)
        }
    }
}

// 4. Use LOD (Level of Detail)
class LODMesh {
    let highDetail: MTLBuffer
    let mediumDetail: MTLBuffer
    let lowDetail: MTLBuffer
    let thresholds: [Float]

    func getBuffer(for distance: Float) -> MTLBuffer {
        switch distance {
        case 0..<thresholds[0]: return highDetail
        case thresholds[0]..<thresholds[1]: return mediumDetail
        default: return lowDetail
        }
    }
}
```

### Memory Management

```swift
// Texture compression
class TextureLoader {
    func loadCompressed(url: URL) -> MTLTexture? {
        // Use ASTC or PVRTC for iOS
        let textureLoader = MTKTextureLoader(device: device)

        return try? textureLoader.newTexture(
            URL: url,
            options: [
                .textureStorageMode: MTLStorageMode.private.rawValue,
                .textureUsage: MTLTextureUsage.shaderRead.rawValue,
            ]
        )
    }
}

// Geometry compression
func compressGeometry(_ vertices: [Vertex]) -> Data {
    // Use quantization for vertex positions
    var compressed = Data()

    for vertex in vertices {
        // Quantize positions to 16-bit
        let x = Int16(vertex.position.x * 32767)
        let y = Int16(vertex.position.y * 32767)
        let z = Int16(vertex.position.z * 32767)

        // Pack normals to 4 bytes
        let packedNormal = packNormal(vertex.normal)

        compressed.append(contentsOf: withUnsafeBytes(of: x) { Data($0) })
        compressed.append(contentsOf: withUnsafeBytes(of: y) { Data($0) })
        compressed.append(contentsOf: withUnsafeBytes(of: z) { Data($0) })
        compressed.append(packedNormal)
    }

    return compressed
}
```

---

## Rust Mobile Rendering

### wgpu for Cross-Platform Mobile

```rust
use wgpu::*;
use raw_window_handle::{HasWindowHandle, HasDisplayHandle};

pub struct MobileRenderer<W: HasWindowHandle + HasDisplayHandle> {
    instance: Instance,
    surface: Option<Surface<'static>>,
    device: Device,
    queue: Queue,
    config: SurfaceConfiguration,
    pipeline: RenderPipeline,
    _window: W,
}

impl<W: HasWindowHandle + HasDisplayHandle> MobileRenderer<W> {
    pub async fn new(window: W) -> Result<Self, Box<dyn std::error::Error>> {
        let instance = Instance::new(InstanceDescriptor {
            backends: Backends::METAL, // iOS/macOS only
            ..Default::default()
        });

        let surface = instance.create_surface(window.clone())?;

        let adapter = instance
            .request_adapter(&RequestAdapterOptions {
                power_preference: PowerPreference::LowPower, // Mobile optimization
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            })
            .await?;

        let (device, queue) = adapter
            .request_device(
                &DeviceDescriptor {
                    required_features: Features::empty(),
                    required_limits: Limits::downlevel_webgl2_defaults(),
                    ..Default::default()
                },
                None,
            )
            .await?;

        let caps = surface.get_capabilities(&adapter);
        let format = caps.formats[0];

        let size = window.window_handle()?.window_size();
        let config = SurfaceConfiguration {
            usage: TextureUsages::RENDER_ATTACHMENT,
            format,
            width: size.width,
            height: size.height,
            present_mode: PresentMode::Fifo, // VSync for mobile
            alpha_mode: caps.alpha_modes[0],
            view_formats: vec![],
        };

        // Create pipeline
        let shader = device.create_shader_module(ShaderModuleDescriptor {
            label: Some("Mobile Shader"),
            source: ShaderSource::Wgsl(include_str!("mobile_shader.wgsl").into()),
        });

        let pipeline = device.create_render_pipeline(&RenderPipelineDescriptor {
            label: Some("Mobile Pipeline"),
            layout: None,
            vertex: VertexState {
                module: &shader,
                entry_point: "vs_main",
                buffers: &[/* ... */],
            },
            fragment: Some(FragmentState {
                module: &shader,
                entry_point: "fs_main",
                targets: &[/* ... */],
            }),
            primitive: PrimitiveState {
                topology: PrimitiveTopology::TriangleList,
                ..Default::default()
            },
            depth_stencil: None,
            multisample: MultisampleState::default(),
            multiview: None,
            cache: None,
        });

        Ok(MobileRenderer {
            instance,
            surface: Some(surface),
            device,
            queue,
            config,
            pipeline,
            _window: window,
        })
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        self.config.width = width.max(1);
        self.config.height = height.max(1);
        if let Some(surface) = &self.surface {
            surface.configure(&self.device, &self.config);
        }
    }

    pub fn render(&mut self) -> Result<(), SurfaceError> {
        let surface = self.surface.as_ref().unwrap();
        let output = surface.get_current_texture()?;
        let view = output.texture.create_view(&TextureViewDescriptor::default());

        let mut encoder = self.device.create_command_encoder(&CommandEncoderDescriptor {
            label: Some("Render Encoder"),
        });

        {
            let mut render_pass = encoder.begin_render_pass(&RenderPassDescriptor {
                label: Some("Render Pass"),
                color_attachments: &[Some(RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: Operations {
                        load: LoadOp::Clear(Color {
                            r: 0.1,
                            g: 0.1,
                            b: 0.1,
                            a: 1.0,
                        }),
                        store: StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
            });

            render_pass.set_pipeline(&self.pipeline);
            // Draw calls...
        }

        self.queue.submit(std::iter::once(encoder.finish()));
        output.present();

        Ok(())
    }
}
```

### WGSL Mobile Shader

```wgsl
// Optimized for mobile GPUs
struct VertexOutput {
    @builtin(position) position: vec4f,
    @location(0) color: vec3f,
    @location(1) uv: vec2f,
};

@vertex
fn vs_main(
    @location(0) position: vec3f,
    @location(1) normal: vec3f,
    @location(2) uv: vec2f,
) -> VertexOutput {
    var output: VertexOutput;
    output.position = vec4f(position, 1.0);
    output.color = normal * 0.5 + 0.5; // Normal visualization
    output.uv = uv;
    return output;
}

@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4f {
    // Simple lighting - optimized for mobile
    let light_dir = normalize(vec3f(0.5, 1.0, 0.3));
    let diff = max(dot(input.color, light_dir), 0.3);
    return vec4f(input.color * diff, 1.0);
}
```

---

## References

1. **Metal Documentation** - https://developer.apple.com/metal/
2. **Metal Shading Language** - Apple MSL spec
3. **Spline Documentation** - https://docs.spline.design/native-3d-embeds-for-ios
4. **wgpu Examples** - https://github.com/gfx-rs/wgpu
