---
location: /home/darkvoid/Boxxed/@dev/ewe_platform/backends/foundation_wasm
repository: Internal (ewe_platform)
explored_at: 2026-03-26
language: Rust
---

# Foundation WASM WebGPU Integration Guide

## Overview

This document provides a comprehensive, step-by-step guide on how to use the `foundation_wasm` binding generator to wrap and use WebGPU in Rust+WASM projects **without depending on the `wasm-bindgen` project**. Instead, we use the custom `foundation_wasm` runtime that provides JavaScript interoperability and memory management.

## Table of Contents

1. [Understanding foundation_wasm Architecture](#understanding-foundation_wasm-architecture)
2. [WebGPU API Surface](#webgpu-api-surface)
3. [Step-by-Step Integration Plan](#step-by-step-integration-plan)
4. [Implementation Examples](#implementation-examples)
5. [Complete WebGPU Wrapper Example](#complete-webgpu-wrapper-example)
6. [Memory Management Considerations](#memory-management-considerations)
7. [Error Handling Patterns](#error-handling-patterns)
8. [Performance Considerations](#performance-considerations)

---

## Understanding foundation_wasm Architecture

### Core Components

The `foundation_wasm` crate provides:

1. **Memory Management** (`mem.rs`):
   - `MemoryAllocation`: Thread-safe handle to underlying `Vec<u8>` memory
   - `MemoryAllocations`: Registry managing all memory slots with free-list reuse
   - `MemoryId`: Generation-based memory identifier for safety

2. **JavaScript API Bridge** (`jsapi.rs`):
   - `DoTask`: Trait for defining host function calls
   - `FnDoTask`: Function wrapper for task execution
   - `InternalReferenceRegistry`: Callback registration for async operations
   - `ReturnTypeHints`: Type hints for return value parsing

3. **Scheduling** (`schedule.rs`, `intervals.rs`, `frames.rs`):
   - `ScheduleRegistry`: setTimeout-style scheduling
   - `IntervalRegistry`: setInterval-style recurring callbacks
   - `FrameCallbackList`: requestAnimationFrame-style frame callbacks

4. **Type Encoding** (`ops.rs`):
   - `Params`: Union type for JavaScript parameter passing
   - `ToBinary`/`FromBinary`: Serialization traits
   - `BatchEncodable`: Batch operation encoding

### Key Architecture Decisions

```rust
// No-std compatible - works in WASM environment
#![no_std]
extern crate alloc;

// Platform-aware synchronization
#[cfg(any(target_arch = "wasm32", target_arch = "wasm64"))]
pub struct WrappedItem<T>(pub alloc::rc::Rc<T>);

#[cfg(all(not(target_arch = "wasm32"), not(target_arch = "wasm64")))]
pub struct WrappedItem<T>(pub alloc::sync::Arc<foundation_nostd::comp::basic::Mutex<T>>);
```

This means:
- On WASM: Uses `Rc` (single-threaded, no Mutex needed)
- On native: Uses `Arc<Mutex<T>>` for thread safety

---

## WebGPU API Surface

### Core WebGPU Interfaces to Wrap

| Interface | Description | foundation_wasm Integration Approach |
|-----------|-------------|-------------------------------------|
| `navigator.gpu` | Entry point | `js_call("navigator.gpu")` |
| `GPUAdapter` | Hardware adapter | Store as ExternalReference |
| `GPUDevice` | GPU device context | Store as ExternalReference |
| `GPUQueue` | Command queue | Store as ExternalReference |
| `GPUBuffer` | Buffer resource | Store as ExternalReference |
| `GPUTexture` | Texture resource | Store as ExternalReference |
| `GPUSampler` | Texture sampler | Store as ExternalReference |
| `GPUBindGroupLayout` | Bind group layout | Store as ExternalReference |
| `GPUPipelineLayout` | Pipeline layout | Store as ExternalReference |
| `GPUComputePipeline` | Compute pipeline | Store as ExternalReference |
| `GPURenderPipeline` | Render pipeline | Store as ExternalReference |
| `GPUShaderModule` | Compiled shader | Store as ExternalReference |
| `GPUCommandEncoder` | Command encoder | Store as ExternalReference |
| `GPURenderPassEncoder` | Render pass | Store as ExternalReference |
| `GPUComputePassEncoder` | Compute pass | Store as ExternalReference |

### WebGPU Method Categories

1. **Async Methods** (require callback registry):
   - `adapter.requestDevice()`
   - `device.createShaderModule()` (with compilation info)
   - `buffer.mapAsync()`

2. **Sync Methods** (direct return):
   - `device.createBuffer()`
   - `device.createTexture()`
   - `device.createBindGroup()`
   - `encoder.beginRenderPass()`

3. **Property Access**:
   - `device.limits`
   - `adapter.features`
   - `buffer.size`

---

## Step-by-Step Integration Plan

### Step 1: Create WebGPU Module Structure

Create a new module in your project:

```
src/
  webgpu/
    mod.rs           # Module root, re-exports
    adapter.rs       # GPUAdapter wrapper
    device.rs        # GPUDevice wrapper
    buffer.rs        # GPUBuffer wrapper
    texture.rs       # GPUTexture wrapper
    shader.rs        # GPUShaderModule wrapper
    pipeline.rs      # Pipeline wrappers
    bindgroup.rs     # BindGroup wrappers
    encoder.rs       # CommandEncoder wrappers
    pass.rs          # Render/Compute pass wrappers
    queue.rs         # GPUQueue wrapper
    error.rs         # WebGPU-specific errors
    ffi.rs           # FFI bindings to host JS
```

### Step 2: Define External Reference Types

```rust
// src/webgpu/mod.rs

use foundation_wasm::{ExternalPointer, InternalPointer, ReturnTypeHints, Returns, TaskResult};

/// GPU external reference wrapper
#[derive(Clone)]
pub struct GPUHandle {
    ptr: ExternalPointer,
}

impl GPUHandle {
    pub fn new(ptr: ExternalPointer) -> Self {
        Self { ptr }
    }

    pub fn as_ptr(&self) -> ExternalPointer {
        self.ptr
    }
}

/// Type-safe GPU resource handles
pub struct GPUAdapter(GPUHandle);
pub struct GPUDevice(GPUHandle);
pub struct GPUBuffer(GPUHandle);
pub struct GPUTexture(GPUHandle);
pub struct GPUSampler(GPUHandle);
pub struct GPUShaderModule(GPUHandle);
pub struct GPUComputePipeline(GPUHandle);
pub struct GPURenderPipeline(GPUHandle);
pub struct GPUBindGroupLayout(GPUHandle);
pub struct GPUBindGroup(GPUHandle);
pub struct GPUPipelineLayout(GPUHandle);
pub struct GPUCommandEncoder(GPUHandle);
pub struct GPURenderPassEncoder(GPUHandle);
pub struct GPUComputePassEncoder(GPUHandle);
pub struct GPUQueue(GPUHandle);
```

### Step 3: Define Host Function Call Interface

```rust
// src/webgpu/ffi.rs

use foundation_wasm::{
    DoTask, FnDoTask, Params, ReturnTypeHints, MemoryAllocation,
    MemoryId, Instructions, create_instructions, parse_callback_replies,
};
use crate::webgpu::{GPUHandle, GPUAdapter, GPUDevice};

/// WebGPU FFI bridge to JavaScript host
pub struct WebGPUHost;

impl WebGPUHost {
    /// Request GPU adapter from navigator.gpu
    pub async fn request_adapter() -> Result<GPUAdapter, WebGPUError> {
        // This will be called via foundation_wasm's task system
        let instructions = create_instructions(256, 256);

        // Encode: navigator.gpu.requestAdapter()
        // Will be implemented in host JS
        let task = FnDoTask::new(
            "navigator.gpu.requestAdapter",
            Params::None,
            ReturnTypeHints::One(ThreeState::One(ReturnTypeId::ExternalReference))
        );

        // Execute and wait for callback
        let result = execute_async_task(task).await?;

        match result {
            Returns::One(ReturnValues::ExternalReference(ptr)) => {
                Ok(GPUAdapter(GPUHandle::new(ptr)))
            }
            _ => Err(WebGPUError::UnexpectedReturn),
        }
    }

    /// Request device from adapter
    pub async fn request_device(adapter: &GPUAdapter) -> Result<GPUDevice, WebGPUError> {
        let task = FnDoTask::new(
            "GPUAdapter.requestDevice",
            Params::ExternalReference(adapter.0.as_ptr()),
            ReturnTypeHints::One(ThreeState::One(ReturnTypeId::ExternalReference))
        );

        let result = execute_async_task(task).await?;

        match result {
            Returns::One(ReturnValues::ExternalReference(ptr)) => {
                Ok(GPUDevice(GPUHandle::new(ptr)))
            }
            _ => Err(WebGPUError::UnexpectedReturn),
        }
    }
}
```

### Step 4: Implement Async Task Execution

```rust
// src/webgpu/async.rs

use foundation_wasm::{
    InternalReferenceRegistry, InternalPointer, ReturnTypeHints,
    Returns, TaskResult, FnCallback, ScheduleRegistry,
};
use alloc::sync::Arc;
use foundation_nostd::comp::basic::Mutex;

/// Pending async task result
pub struct PendingTask {
    pub return_hints: ReturnTypeHints,
    pub callback: FnCallback,
}

/// Async executor for WebGPU tasks
pub struct WebGPUExecutor {
    pending: Mutex<BTreeMap<InternalPointer, PendingTask>>,
}

impl WebGPUExecutor {
    pub const fn new() -> Self {
        Self {
            pending: Mutex::new(BTreeMap::new()),
        }
    }

    /// Register a pending async task
    pub fn register<F>(&self, return_hints: ReturnTypeHints, callback: F) -> InternalPointer
    where
        F: Fn(TaskResult<Returns>) + 'static,
    {
        let ptr = InternalPointer::new(self.next_id());
        let mut pending = self.pending.lock().unwrap();
        pending.insert(
            ptr,
            PendingTask {
                return_hints,
                callback: FnCallback::from(callback),
            },
        );
        ptr
    }

    /// Receive callback from host
    pub fn on_task_complete(&self, id: InternalPointer, result: TaskResult<Returns>) {
        let mut pending = self.pending.lock().unwrap();
        if let Some(task) = pending.remove(&id) {
            task.callback.receive(result);
        }
    }

    fn next_id(&self) -> u64 {
        // Atomic ID generation
        static COUNTER: AtomicU64 = AtomicU64::new(0);
        COUNTER.fetch_add(1, Ordering::Relaxed)
    }
}

/// Future wrapper for async WebGPU operations
pub struct WebGPUFuture<T> {
    receiver: alloc::sync::Arc<Mutex<Option<Result<T, WebGPUError>>>>,
}

impl<T> core::future::Future for WebGPUFuture<T> {
    type Output = Result<T, WebGPUError>;

    fn poll(
        self: core::pin::Pin<&mut Self>,
        cx: &mut core::task::Context<'_>,
    ) -> core::task::Poll<Self::Output> {
        let mut inner = self.receiver.lock().unwrap();
        match inner.take() {
            Some(result) => core::task::Poll::Ready(result),
            None => {
                // Register waker for later
                cx.waker().wake_by_ref();
                core::task::Poll::Pending
            }
        }
    }
}
```

### Step 5: Implement WebGPU Adapter Wrapper

```rust
// src/webgpu/adapter.rs

use foundation_wasm::{ExternalPointer, Params, ReturnTypeHints};
use crate::webgpu::{
    device::GPUDevice,
    error::WebGPUError,
    ffi::WebGPUHost,
    async::WebGPUFuture,
};

/// GPUAdapter wrapper
#[derive(Clone)]
pub struct GPUAdapter {
    handle: ExternalPointer,
}

impl GPUAdapter {
    pub fn from_ptr(ptr: ExternalPointer) -> Self {
        Self { handle: ptr }
    }

    pub fn as_ptr(&self) -> ExternalPointer {
        self.handle
    }

    /// Request a GPU device from this adapter
    pub async fn request_device(
        &self,
        descriptor: &GPUDeviceDescriptor,
    ) -> WebGPUFuture<GPUDevice> {
        // Encode device descriptor
        let desc_params = descriptor.to_params();

        // Call adapter.requestDevice(descriptor)
        WebGPUHost::call_method(
            self.handle,
            "requestDevice",
            desc_params,
            ReturnTypeHints::external_ref(),
        )
        .await
    }

    /// Get adapter features
    pub fn features(&self) -> GPUSupportedFeatures {
        WebGPUHost::get_property(self.handle, "features")
    }

    /// Get adapter limits
    pub fn limits(&self) -> GPUSupportedLimits {
        WebGPUHost::get_property(self.handle, "limits")
    }
}

/// GPUDeviceDescriptor
pub struct GPUDeviceDescriptor {
    pub label: Option<alloc::string::String>,
    pub required_features: alloc::vec::Vec<alloc::string::String>,
    pub required_limits: Option<GPULimits>,
    pub default_queue: GPUQueueDescriptor,
}

impl GPUDeviceDescriptor {
    pub fn to_params(&self) -> Params {
        // Convert to foundation_wasm Params
        // This will be serialized and sent to JS
        Params::ExternalReference(/* encoded descriptor */)
    }
}
```

### Step 6: Implement WebGPU Device Wrapper

```rust
// src/webgpu/device.rs

use crate::webgpu::{
    buffer::GPUBuffer,
    texture::GPUTexture,
    shader::GPUShaderModule,
    pipeline::GPUComputePipeline,
    bindgroup::GPUBindGroupLayout,
    error::WebGPUError,
};

/// GPUDevice wrapper
#[derive(Clone)]
pub struct GPUDevice {
    handle: ExternalPointer,
    queue: GPUQueue,
}

impl GPUDevice {
    pub fn from_ptr(ptr: ExternalPointer, queue: GPUQueue) -> Self {
        Self { handle: ptr, queue }
    }

    /// Create a GPU buffer
    pub fn create_buffer(&self, descriptor: &GPUBufferDescriptor) -> GPUBuffer {
        let ptr = WebGPUHost::call_method_sync(
            self.handle,
            "createBuffer",
            descriptor.to_params(),
        );
        GPUBuffer::from_ptr(ptr, descriptor.size, descriptor.usage)
    }

    /// Create a texture
    pub fn create_texture(&self, descriptor: &GPUTextureDescriptor) -> GPUTexture {
        let ptr = WebGPUHost::call_method_sync(
            self.handle,
            "createTexture",
            descriptor.to_params(),
        );
        GPUTexture::from_ptr(ptr)
    }

    /// Create a shader module from WGSL
    pub fn create_shader_module(&self, descriptor: &GPUShaderModuleDescriptor) -> GPUShaderModule {
        let ptr = WebGPUHost::call_method_sync(
            self.handle,
            "createShaderModule",
            descriptor.to_params(),
        );
        GPUShaderModule::from_ptr(ptr)
    }

    /// Create a compute pipeline
    pub fn create_compute_pipeline(
        &self,
        descriptor: &GPUComputePipelineDescriptor,
    ) -> GPUComputePipeline {
        let ptr = WebGPUHost::call_method_sync(
            self.handle,
            "createComputePipeline",
            descriptor.to_params(),
        );
        GPUComputePipeline::from_ptr(ptr)
    }

    /// Create a bind group layout
    pub fn create_bind_group_layout(
        &self,
        descriptor: &GPUBindGroupLayoutDescriptor,
    ) -> GPUBindGroupLayout {
        let ptr = WebGPUHost::call_method_sync(
            self.handle,
            "createBindGroupLayout",
            descriptor.to_params(),
        );
        GPUBindGroupLayout::from_ptr(ptr)
    }

    /// Get the device queue
    pub fn queue(&self) -> &GPUQueue {
        &self.queue
    }
}
```

### Step 7: Implement Buffer Operations

```rust
// src/webgpu/buffer.rs

use foundation_wasm::{MemoryAllocation, ExternalPointer};
use crate::webgpu::{error::WebGPUError, device::GPUDevice};

bitflags::bitflags! {
    pub struct BufferUsage: u32 {
        const MAP_READ = 1 << 0;
        const MAP_WRITE = 1 << 1;
        const COPY_SRC = 1 << 2;
        const COPY_DST = 1 << 3;
        const INDEX = 1 << 4;
        const VERTEX = 1 << 5;
        const UNIFORM = 1 << 6;
        const STORAGE = 1 << 7;
        const INDIRECT = 1 << 8;
    }
}

/// GPUBuffer wrapper
#[derive(Clone)]
pub struct GPUBuffer {
    handle: ExternalPointer,
    size: u64,
    usage: BufferUsage,
    mapped: bool,
}

impl GPUBuffer {
    pub fn from_ptr(ptr: ExternalPointer, size: u64, usage: BufferUsage) -> Self {
        Self {
            handle: ptr,
            size,
            usage,
            mapped: false,
        }
    }

    /// Get buffer size
    pub fn size(&self) -> u64 {
        self.size
    }

    /// Get buffer usage flags
    pub fn usage(&self) -> BufferUsage {
        self.usage
    }

    /// Map the buffer for reading
    pub async fn map_async(&self, mode: MapMode) -> Result<(), WebGPUError> {
        if self.mapped {
            return Err(WebGPUError::BufferAlreadyMapped);
        }

        WebGPUHost::call_method_async(
            self.handle,
            "mapAsync",
            Params::Uint64(mode.bits() as u64),
            ReturnTypeHints::None,
        )
        .await?;

        self.mapped = true;
        Ok(())
    }

    /// Get mapped range as MemoryAllocation
    pub fn get_mapped_range(&self, offset: u64, size: Option<u64>) -> MappedBufferRange {
        let ptr = WebGPUHost::call_method_sync(
            self.handle,
            "getMappedRange",
            Params::Multi(alloc::vec![
                Params::Uint64(offset),
                size.map(Params::Uint64).unwrap_or(Params::Undefined),
            ]),
        );
        MappedBufferRange::from_ptr(ptr, size.unwrap_or(self.size - offset))
    }

    /// Unmap the buffer
    pub fn unmap(&self) {
        WebGPUHost::call_method_sync(self.handle, "unmap", Params::None);
        self.mapped = false;
    }

    /// Write data to buffer (creates staging copy)
    pub fn write_buffer(&self, device: &GPUDevice, offset: u64, data: &[u8]) {
        // Create staging buffer
        let staging = device.create_buffer(&GPUBufferDescriptor {
            label: Some("staging".into()),
            size: data.len() as u64,
            usage: BufferUsage::MAP_WRITE | BufferUsage::COPY_SRC,
            mapped_at_creation: true,
        });

        // Write data
        staging.map_async(MapMode::Write);
        let mut range = staging.get_mapped_range(0, None);
        range.write(data);
        staging.unmap();

        // Copy to destination
        let mut encoder = device.create_command_encoder(&GPUCommandEncoderDescriptor::default());
        encoder.copy_buffer_to_buffer(&staging, 0, self, offset, data.len() as u64);
        device.queue().submit(alloc::vec![encoder.finish()]);
    }
}

/// Mapped buffer range for safe access
pub struct MappedBufferRange {
    handle: ExternalPointer,
    size: u64,
    memory: MemoryAllocation,
}

impl MappedBufferRange {
    pub fn from_ptr(ptr: ExternalPointer, size: u64) -> Self {
        let memory = MemoryAllocation::new(alloc::vec![0; size as usize]);
        Self { handle: ptr, size, memory }
    }

    /// Write bytes to mapped range
    pub fn write(&mut self, data: &[u8]) {
        let len = core::cmp::min(data.len() as u64, self.size);
        self.memory.apply(|mem| {
            mem[..len as usize].copy_from_slice(&data[..len as usize]);
        });
        WebGPUHost::sync_memory_to_js(self.handle, &self.memory);
    }

    /// Read bytes from mapped range
    pub fn read(&self) -> alloc::vec::Vec<u8> {
        WebGPUHost::read_memory_from_js(self.handle, self.size as usize)
    }
}
```

### Step 8: Implement Command Encoder

```rust
// src/webgpu/encoder.rs

use crate::webgpu::{
    buffer::GPUBuffer,
    texture::GPUTexture,
    pass::{GPURenderPassEncoder, GPUComputePassEncoder},
};

/// GPUCommandEncoder wrapper
#[derive(Clone)]
pub struct GPUCommandEncoder {
    handle: ExternalPointer,
}

impl GPUCommandEncoder {
    pub fn from_ptr(ptr: ExternalPointer) -> Self {
        Self { handle: ptr }
    }

    /// Begin a render pass
    pub fn begin_render_pass(&self, descriptor: &GPURenderPassDescriptor) -> GPURenderPassEncoder {
        let ptr = WebGPUHost::call_method_sync(
            self.handle,
            "beginRenderPass",
            descriptor.to_params(),
        );
        GPURenderPassEncoder::from_ptr(ptr)
    }

    /// Begin a compute pass
    pub fn begin_compute_pass(
        &self,
        descriptor: &GPUComputePassDescriptor,
    ) -> GPUComputePassEncoder {
        let ptr = WebGPUHost::call_method_sync(
            self.handle,
            "beginComputePass",
            descriptor.to_params(),
        );
        GPUComputePassEncoder::from_ptr(ptr)
    }

    /// Copy buffer to buffer
    pub fn copy_buffer_to_buffer(
        &self,
        source: &GPUBuffer,
        source_offset: u64,
        destination: &GPUBuffer,
        destination_offset: u64,
        size: u64,
    ) {
        WebGPUHost::call_method_sync(
            self.handle,
            "copyBufferToBuffer",
            Params::Multi(alloc::vec![
                Params::ExternalReference(source.as_ptr()),
                Params::Uint64(source_offset),
                Params::ExternalReference(destination.as_ptr()),
                Params::Uint64(destination_offset),
                Params::Uint64(size),
            ]),
        );
    }

    /// Copy buffer to texture
    pub fn copy_buffer_to_texture(
        &self,
        source: &GPUImageCopyBuffer,
        destination: &GPUImageCopyTexture,
        copy_size: GPUExtent3D,
    ) {
        WebGPUHost::call_method_sync(
            self.handle,
            "copyBufferToTexture",
            Params::Multi(alloc::vec![
                source.to_params(),
                destination.to_params(),
                copy_size.to_params(),
            ]),
        );
    }

    /// Copy texture to buffer
    pub fn copy_texture_to_buffer(
        &self,
        source: &GPUImageCopyTexture,
        destination: &GPUImageCopyBuffer,
        copy_size: GPUExtent3D,
    ) {
        WebGPUHost::call_method_sync(
            self.handle,
            "copyTextureToBuffer",
            Params::Multi(alloc::vec![
                source.to_params(),
                destination.to_params(),
                copy_size.to_params(),
            ]),
        );
    }

    /// Finish encoding and return GPUCommandBuffer
    pub fn finish(self) -> GPUCommandBuffer {
        let ptr = WebGPUHost::call_method_sync(
            self.handle,
            "finish",
            Params::None,
        );
        GPUCommandBuffer::from_ptr(ptr)
    }
}
```

### Step 9: Create JavaScript Host Bridge

Create the JavaScript side that will handle the WebGPU calls:

```javascript
// webgpu_host.js

// This runs on the JavaScript/host side and communicates with Rust WASM

const gpuDeviceRegistry = new Map();
const gpuBufferRegistry = new Map();
const gpuTextureRegistry = new Map();
const callbackRegistry = new Map();

let nextId = 1;

function registerGPUObject(obj, type) {
    const id = nextId++;
    const registry = type === 'device' ? gpuDeviceRegistry
        : type === 'buffer' ? gpuBufferRegistry
        : gpuTextureRegistry;
    registry.set(id, obj);
    return id;
}

function getGPUObject(id, type) {
    const registry = type === 'device' ? gpuDeviceRegistry
        : type === 'buffer' ? gpuBufferRegistry
        : gpuTextureRegistry;
    return registry.get(id);
}

// Handle tasks from Rust
globalThis.handleWebGPUTask = async function(taskId, methodName, params, callbackId) {
    try {
        let result;

        if (methodName === 'navigator.gpu.requestAdapter') {
            const adapter = await navigator.gpu.requestAdapter();
            result = registerGPUObject(adapter, 'adapter');
        }
        else if (methodName === 'GPUAdapter.requestDevice') {
            const adapter = getGPUObject(params[0], 'adapter');
            const device = await adapter.requestDevice(params[1]);
            result = registerGPUObject(device, 'device');
        }
        else if (methodName === 'GPUDevice.createBuffer') {
            const device = getGPUObject(params[0], 'device');
            const buffer = device.createBuffer(params[1]);
            result = registerGPUObject(buffer, 'buffer');
        }
        // ... handle all other methods

        // Send result back to Rust
        globalThis.onTaskComplete(callbackId, result);
    } catch (error) {
        globalThis.onTaskError(callbackId, error.message);
    }
};

// Async callback handler
globalThis.onTaskComplete = function(callbackId, result) {
    // Use foundation_wasm's callback mechanism
    const rustCallback = callbackRegistry.get(callbackId);
    if (rustCallback) {
        rustCallback(null, result);
        callbackRegistry.delete(callbackId);
    }
};
```

---

## Implementation Examples

### Example 1: Basic Triangle Rendering

```rust
use foundation_wasm;
use crate::webgpu::{
    adapter::GPUAdapter,
    device::GPUDevice,
    buffer::GPUBuffer,
    pipeline::GPUComputePipeline,
};

async fn init_webgpu() -> Result<WebGPUContext, WebGPUError> {
    // 1. Request adapter
    let adapter = GPUAdapter::request().await?;

    // 2. Request device
    let device = adapter.request_device(&GPUDeviceDescriptor {
        label: Some("Main Device".into()),
        required_features: vec![],
        required_limits: None,
        default_queue: GPUQueueDescriptor::default(),
    }).await?;

    // 3. Create shader module
    let shader = device.create_shader_module(&GPUShaderModuleDescriptor {
        label: Some("Triangle Shader".into()),
        code: r#"
            @vertex
            fn vs_main(@builtin(vertex_index) vertex_index: u32) -> @builtin(position) vec4f {
                var positions = array<vec2f, 3>(
                    vec2f(0.0, 0.5),
                    vec2f(-0.5, -0.5),
                    vec2f(0.5, -0.5)
                );
                return vec4f(positions[vertex_index], 0.0, 1.0);
            }

            @fragment
            fn fs_main() -> @location(0) vec4f {
                return vec4f(1.0, 0.0, 0.0, 1.0);
            }
        "#,
    });

    // 4. Create render pipeline
    let pipeline = device.create_render_pipeline(&GPURenderPipelineDescriptor {
        label: Some("Triangle Pipeline".into()),
        layout: PipelineLayout::Auto,
        vertex: GPUVertexState {
            module: &shader,
            entry_point: "vs_main",
            buffers: &[],
        },
        fragment: Some(GPUFragmentState {
            module: &shader,
            entry_point: "fs_main",
            targets: &[ColorTargetState {
                format: TextureFormat::Bgra8UnormSrgb,
                blend: None,
                write_mask: ColorWrites::ALL,
            }],
        }),
        primitive: PrimitiveState {
            topology: PrimitiveTopology::TriangleList,
            ..Default::default()
        },
        depth_stencil: None,
        multisample: MultisampleState::default(),
    });

    Ok(WebGPUContext { adapter, device, shader, pipeline })
}
```

### Example 2: Compute Shader

```rust
async fn run_compute_example(device: &GPUDevice) -> Result<(), WebGPUError> {
    // Create compute shader
    let shader = device.create_shader_module(&GPUShaderModuleDescriptor {
        label: Some("Compute Shader".into()),
        code: r#"
            @group(0) @binding(0)
            var<storage, read_write> output: array<u32>;

            @compute @workgroup_size(64)
            fn main(@builtin(global_invocation_id) id: vec3u) {
                output[id.x] = id.x * 2;
            }
        "#,
    });

    // Create pipeline
    let pipeline = device.create_compute_pipeline(&GPUComputePipelineDescriptor {
        label: Some("Compute Pipeline".into()),
        layout: PipelineLayout::Auto,
        module: &shader,
        entry_point: "main",
    });

    // Create output buffer
    let output_buffer = device.create_buffer(&GPUBufferDescriptor {
        label: Some("Output Buffer".into()),
        size: 1024 * 4, // 1024 u32 values
        usage: BufferUsage::STORAGE | BufferUsage::COPY_SRC | BufferUsage::MAP_READ,
        mapped_at_creation: false,
    });

    // Create bind group
    let bind_group_layout = pipeline.get_bind_group_layout(0);
    let bind_group = device.create_bind_group(&GPUBindGroupDescriptor {
        label: Some("Compute Bind Group".into()),
        layout: &bind_group_layout,
        entries: &[BindGroupEntry {
            binding: 0,
            resource: BindingResource::Buffer(output_buffer.as_entire_buffer_binding()),
        }],
    });

    // Encode commands
    let mut encoder = device.create_command_encoder(&GPUCommandEncoderDescriptor::default());

    {
        let mut compute_pass = encoder.begin_compute_pass(&GPUComputePassDescriptor::default());
        compute_pass.set_pipeline(&pipeline);
        compute_pass.set_bind_group(0, &bind_group, &[]);
        compute_pass.dispatch_workgroups(16, 1, 1);
    }

    let command_buffer = encoder.finish();
    device.queue().submit(vec![command_buffer]);

    // Read results
    output_buffer.map_async(MapMode::Read).await?;
    let data = output_buffer.get_mapped_range(0, None).read();
    output_buffer.unmap();

    println!("First 10 values: {:?}", &data[..10]);
    // Expected: [0, 2, 4, 6, 8, 10, 12, 14, 16, 18]

    Ok(())
}
```

---

## Memory Management Considerations

### Memory Allocation Strategy

1. **Use foundation_wasm's MemoryAllocations**:
   - Pre-allocate common sizes (256, 1024, 4096 bytes)
   - Reuse freed slots via free-list
   - Generation IDs prevent use-after-free

2. **GPU Resource Lifetime**:
   - GPU resources are owned by JS garbage collector
   - Rust holds external references (not owning)
   - Implement explicit destroy methods for deterministic cleanup

```rust
impl GPUBuffer {
    pub fn destroy(self) {
        WebGPUHost::call_method_sync(self.handle, "destroy", Params::None);
        // External reference is now invalid
    }
}
```

3. **Mapped Buffer Memory**:
   - Allocate staging memory via `MemoryAllocation`
   - Sync to/from JS when mapping/unmapping
   - Clear memory after unmap to prevent stale data

---

## Error Handling Patterns

### WebGPU Error Types

```rust
// src/webgpu/error.rs

#[derive(Debug)]
pub enum WebGPUError {
    // Request errors
    NoGPUAvailable,
    AdapterRequestFailed,
    DeviceRequestFailed,

    // Resource errors
    BufferCreationFailed,
    TextureCreationFailed,
    ShaderCompilationFailed(alloc::string::String),

    // Operation errors
    BufferAlreadyMapped,
    InvalidMappingMode,
    BufferNotMapped,

    // Validation errors
    ValidationError(alloc::string::String),
    OutOfMemory,

    // Internal errors
    UnexpectedReturn,
    CallbackTimeout,
}

impl From<MemoryAllocationError> for WebGPUError {
    fn from(err: MemoryAllocationError) -> Self {
        match err {
            MemoryAllocationError::NoMemoryAllocation => WebGPUError::OutOfMemory,
            _ => WebGPUError::OutOfMemory,
        }
    }
}
```

---

## Performance Considerations

### Minimizing FFI Overhead

1. **Batch Operations**:
   - Use `BatchEncodable` to encode multiple operations
   - Single FFI call per batch instead of per-operation

2. **Memory Reuse**:
   - Reuse `MemoryAllocation` slots
   - Pre-allocate common descriptor sizes

3. **Async Pattern**:
   - Use callback registry for async operations
   - Avoid blocking/polling in WASM

4. **Descriptor Caching**:
   - Cache frequently used descriptors
   - Use string interning for labels

---

## Next Steps

1. Implement the full WebGPU API surface in `foundation_webgpu` crate
2. Create JavaScript host bridge with full method dispatch
3. Add integration tests comparing with wasm-bindgen web-sys
4. Benchmark performance vs wasm-bindgen
5. Create example applications (triangle, compute, 3D scene)

---

## Appendix: Complete File List

```
foundation_webgpu/
  Cargo.toml
  src/
    lib.rs           # Root exports
    mod.rs           # Module structure
    error.rs         # Error types
    async.rs         # Async execution
    ffi.rs           # FFI bridge
    adapter.rs       # GPUAdapter
    device.rs        # GPUDevice
    queue.rs         # GPUQueue
    buffer.rs        # GPUBuffer
    texture.rs       # GPUTexture
    sampler.rs       # GPUSampler
    shader.rs        # GPUShaderModule
    bindgroup.rs     # BindGroup, BindGroupLayout
    pipeline.rs      # ComputePipeline, RenderPipeline
    encoder.rs       # CommandEncoder
    pass.rs          # RenderPass, ComputePass
    types.rs         # WebGPU type definitions
```
