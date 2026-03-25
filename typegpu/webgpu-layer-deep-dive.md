# WebGPU Layer Deep Dive

## Overview

TypeGPU sits on top of WebGPU, providing a type-safe abstraction layer. This document details exactly how TypeGPU accesses WebGPU APIs.

## WebGPU Initialization

### Device Acquisition

**File**: `src/core/root/init.ts`

```typescript
// init() - Request adapter and device
export async function init(
  descriptor?: GPURequestAdapterOptions & {
    device?: GPUDeviceDescriptor;
  }
): Promise<TgpuRoot> {
  const adapter = await navigator.gpu?.requestAdapter(descriptor);
  const device = await adapter?.requestDevice(descriptor?.device);
  return new TgpuRootImpl(device);
}

// initFromDevice() - Use existing device
export function initFromDevice(device: GPUDevice): TgpuRoot {
  return new TgpuRootImpl(device);
}
```

**Exact WebGPU Calls**:
1. `navigator.gpu.requestAdapter()` - Get GPU adapter
2. `adapter.requestDevice()` - Get logical device

### TgpuRootImpl - Root Context

```typescript
// src/core/root/init.ts
class TgpuRootImpl implements TgpuRoot {
  constructor(
    public readonly device: GPUDevice,
    private readonly _ownDevice = true
  ) {}

  // Command encoder management
  private _commandEncoder: GPUCommandEncoder | undefined;

  get commandEncoder(): GPUCommandEncoder {
    if (!this._commandEncoder) {
      this._commandEncoder = this.device.createCommandEncoder();
    }
    return this._commandEncoder;
  }

  // Submit commands
  submit(): void {
    if (this._commandEncoder) {
      this.device.queue.submit([this._commandEncoder.finish()]);
      this._commandEncoder = undefined;
    }
  }
}
```

**Key Pattern**: Lazy command encoder creation - encoder only created when first needed.

## Buffer Creation

### Basic Buffer

**File**: `src/core/buffer/buffer.ts`

```typescript
private _ensureBuffer(device: GPUDevice): GPUBuffer {
  if (!this._buffer) {
    const initialData = this._initialOrBuffer as Infer<TData> | undefined;
    const mappedData = initialData
      ? getInitialData(this._schema, initialData)
      : undefined;

    this._buffer = device.createBuffer({
      label: this._label,
      size: mappedData?.byteLength ?? getSize(this._schema),
      usage: GPUBufferUsage.COPY_DST | GPUBufferUsage.COPY_SRC,
      mappedAtCreation: !!mappedData,
    });

    if (mappedData) {
      new Uint8Array(this._buffer.getMappedRange()).set(mappedData);
      this._buffer.unmap();
    }
  }
  return this._buffer;
}
```

**Exact WebGPU Calls**:
1. `device.createBuffer()` - Create GPU buffer
2. `buffer.getMappedRange()` - Get CPU-accessible memory (if mappedAtCreation)
3. `buffer.unmap()` - Commit initial data

### Buffer Usage Flags

```typescript
// Different usage patterns create different buffer configurations

// Uniform buffer
const uniformBuffer = device.createBuffer({
  size: 64,  // Uniform buffers must be aligned
  usage: GPUBufferUsage.UNIFORM | GPUBufferUsage.COPY_DST,
});

// Storage buffer
const storageBuffer = device.createBuffer({
  size: 1024,
  usage: GPUBufferUsage.STORAGE | GPUBufferUsage.COPY_DST | GPUBufferUsage.COPY_SRC,
});

// Vertex buffer
const vertexBuffer = device.createBuffer({
  size: 512,
  usage: GPUBufferUsage.VERTEX | GPUBufferUsage.COPY_DST,
});
```

## Buffer Operations

### Write Operation

```typescript
// src/core/buffer/buffer.ts
write(data: Infer<TData> | InferGPU<TData>): this {
  const device = this._getDevice();
  const gpuBuffer = this._ensureBuffer(device);

  // Serialize to binary
  const writer = new BufferWriter(getSize(this._schema));
  writeData(writer, this._schema, data);
  const arrayBuffer = writer.finish();

  // Write to GPU queue
  device.queue.writeBuffer(
    gpuBuffer,    // destination buffer
    0,            // byte offset
    arrayBuffer,  // source data
    0,            // source offset
    arrayBuffer.byteLength  // size
  );

  return this;
}
```

**Exact WebGPU Calls**:
1. `device.queue.writeBuffer()` - Queue write command

### Read Operation (Staging Buffer Pattern)

```typescript
// src/core/buffer/buffer.ts
async read(): Promise<Infer<TData>> {
  const device = this._getDevice();
  const gpuBuffer = this._ensureBuffer(device);

  // Step 1: Create staging buffer (MAP_READ usage)
  const stagingBuffer = device.createBuffer({
    size: getSize(this._schema),
    usage: GPUBufferUsage.COPY_DST | GPUBufferUsage.MAP_READ,
  });

  // Step 2: Copy from GPU buffer to staging
  const commandEncoder = device.createCommandEncoder();
  commandEncoder.copyBufferToBuffer(
    gpuBuffer,      // source
    0,              // source offset
    stagingBuffer,  // destination
    0,              // destination offset
    getSize(this._schema)  // size
  );
  const commandBuffer = commandEncoder.finish();

  // Step 3: Submit copy command
  device.queue.submit([commandBuffer]);

  // Step 4: Map staging buffer for reading
  await stagingBuffer.mapAsync(GPUMapMode.READ);

  // Step 5: Read data
  const arrayBuffer = stagingBuffer.getMappedRange();
  const reader = new BufferReader(new Uint8Array(arrayBuffer));
  const data = readData(reader, this._schema);

  // Step 6: Cleanup
  stagingBuffer.unmap();
  stagingBuffer.destroy();

  return data;
}
```

**Exact WebGPU Calls**:
1. `device.createBuffer()` - Create staging buffer
2. `device.createCommandEncoder()` - Create command encoder
3. `commandEncoder.copyBufferToBuffer()` - Record copy command
4. `commandEncoder.finish()` - Finish command buffer
5. `device.queue.submit()` - Submit to GPU
6. `buffer.mapAsync()` - Map for CPU access
7. `buffer.getMappedRange()` - Get mapped memory
8. `buffer.unmap()` - Unmap after reading
9. `buffer.destroy()` - Clean up staging buffer

## Pipeline Creation

### Compute Pipeline

**File**: `src/core/pipeline/computePipeline.ts`

```typescript
// src/core/pipeline/computePipeline.ts
class ComputePipelineCore {
  unwrap(device: GPUDevice): GPUComputePipeline {
    if (!this._pipeline) {
      // Step 1: Create shader module
      const shaderModule = device.createShaderModule({
        label: this._label,
        code: this._shaderCode,
      });

      // Step 2: Create pipeline layout
      const layout = device.createPipelineLayout({
        bindGroupLayouts: this._bindGroupLayouts.map(layout =>
          layout.unwrap(device)
        ),
      });

      // Step 3: Create compute pipeline
      this._pipeline = device.createComputePipeline({
        layout,
        compute: {
          module: shaderModule,
          entryPoint: this._entryPoint,
        },
      });
    }
    return this._pipeline;
  }
}
```

**Exact WebGPU Calls**:
1. `device.createShaderModule()` - Compile WGSL shader
2. `device.createPipelineLayout()` - Create pipeline layout
3. `device.createComputePipeline()` - Create compute pipeline

### Render Pipeline

```typescript
// src/core/pipeline/renderPipeline.ts
class RenderPipelineCore {
  unwrap(device: GPUDevice): GPURenderPipeline {
    const shaderModule = device.createShaderModule({
      code: this._shaderCode,
    });

    return device.createRenderPipeline({
      layout: device.createPipelineLayout({
        bindGroupLayouts: this._bindGroupLayouts.map(l => l.unwrap(device)),
      }),
      vertex: {
        module: shaderModule,
        entryPoint: this._vertexEntryPoint,
        buffers: this._vertexBuffers.map(vb => ({
          arrayStride: vb.arrayStride,
          attributes: vb.attributes.map(attr => ({
            shaderLocation: attr.shaderLocation,
            offset: attr.offset,
            format: attr.format,  // e.g., 'float32x3'
          })),
        })),
      },
      fragment: {
        module: shaderModule,
        entryPoint: this._fragmentEntryPoint,
        targets: this._colorTargets.map(target => ({
          format: target.format,
          blend: target.blend,
        })),
      },
      primitive: {
        topology: this._topology,  // 'triangle-list', etc.
      },
    });
  }
}
```

**Exact WebGPU Calls**:
1. `device.createShaderModule()` - Compile shader
2. `device.createPipelineLayout()` - Create layout
3. `device.createRenderPipeline()` - Create render pipeline

## Bind Group Creation

### Bind Group Layout

**File**: `src/tgpuBindGroupLayout.ts`

```typescript
// src/tgpuBindGroupLayout.ts
class TgpuBindGroupLayoutImpl implements TgpuBindGroupLayout {
  unwrap(device: GPUDevice): GPUBindGroupLayout {
    if (!this._layout) {
      this._layout = device.createBindGroupLayout({
        label: this._label,
        entries: Object.entries(this._entries).map(([binding, entry]) => ({
          binding: parseInt(binding),
          visibility: this._getVisibility(entry),
          buffer: entry.buffer,
          texture: entry.texture,
          sampler: entry.sampler,
          storageTexture: entry.storageTexture,
        })),
      });
    }
    return this._layout;
  }

  private _getVisibility(entry: TgpuLayoutEntry): GPUShaderStageFlags {
    // Determine visibility based on entry type
    if (entry.buffer?.type === 'uniform') {
      return GPUShaderStage.VERTEX | GPUShaderStage.FRAGMENT | GPUShaderStage.COMPUTE;
    }
    // ... other cases
  }
}
```

**Exact WebGPU Calls**:
1. `device.createBindGroupLayout()` - Create bind group layout

### Bind Group

```typescript
// src/tgpuBindGroupLayout.ts
class TgpuBindGroupImpl implements TgpuBindGroup {
  unwrap(device: GPUDevice): GPUBindGroup {
    if (!this._bindGroup) {
      this._bindGroup = device.createBindGroup({
        label: this._label,
        layout: this._layout.unwrap(device),
        entries: Object.entries(this._entries).map(([binding, entry]) => ({
          binding: parseInt(binding),
          resource: this._getResource(entry),
        })),
      });
    }
    return this._bindGroup;
  }

  private _getResource(entry: TgpuLayoutEntry): GPUBindingResource {
    if (entry.buffer) {
      return entry.buffer.unwrap(device);
    }
    if (entry.texture) {
      return entry.texture.unwrap(device).createView();
    }
    if (entry.sampler) {
      return entry.sampler.unwrap(device);
    }
  }
}
```

**Exact WebGPU Calls**:
1. `device.createBindGroup()` - Create bind group

## Shader Module Creation

```typescript
// src/core/pipeline/computePipeline.ts
const shaderModule = device.createShaderModule({
  label: 'MyShader',
  code: `
    @group(0) @binding(0)
    var<uniform> config: Config;

    @compute @workgroup_size(64)
    fn main(@builtin(global_invocation_id) id: vec3<u32>) {
      // Shader code
    }
  `,
});
```

**Exact WebGPU Calls**:
1. `device.createShaderModule()` - Compile WGSL

## Command Recording

### Compute Pass

```typescript
// src/core/root/init.ts
const commandEncoder = device.createCommandEncoder();

const computePass = commandEncoder.beginComputePass();
computePass.setPipeline(computePipeline);
computePass.setBindGroup(0, bindGroup);
computePass.dispatchWorkgroups(workgroupCountX, workgroupCountY, workgroupCountZ);
computePass.end();

const commandBuffer = commandEncoder.finish();
device.queue.submit([commandBuffer]);
```

**Exact WebGPU Calls**:
1. `device.createCommandEncoder()` - Create command encoder
2. `commandEncoder.beginComputePass()` - Begin compute pass
3. `computePass.setPipeline()` - Set compute pipeline
4. `computePass.setBindGroup()` - Set bind group
5. `computePass.dispatchWorkgroups()` - Dispatch compute shader
6. `computePass.end()` - End compute pass
7. `commandEncoder.finish()` - Finish command buffer
8. `device.queue.submit()` - Submit to GPU

### Render Pass

```typescript
const commandEncoder = device.createCommandEncoder();

const renderPass = commandEncoder.beginRenderPass({
  colorAttachments: [{
    view: texture.createView(),
    clearValue: { r: 0, g: 0, b: 0, a: 1 },
    loadOp: 'clear',
    storeOp: 'store',
  }],
});

renderPass.setPipeline(renderPipeline);
renderPass.setBindGroup(0, bindGroup);
renderPass.setVertexBuffer(0, vertexBuffer);
renderPass.draw(vertexCount, instanceCount);
renderPass.end();

const commandBuffer = commandEncoder.finish();
device.queue.submit([commandBuffer]);
```

**Exact WebGPU Calls**:
1. `device.createCommandEncoder()` - Create command encoder
2. `commandEncoder.beginRenderPass()` - Begin render pass
3. `renderPass.setPipeline()` - Set render pipeline
4. `renderPass.setBindGroup()` - Set bind group
5. `renderPass.setVertexBuffer()` - Set vertex buffer
6. `renderPass.draw()` - Draw primitives
7. `renderPass.end()` - End render pass
8. `commandEncoder.finish()` - Finish command buffer
9. `device.queue.submit()` - Submit to GPU

## Texture Creation

```typescript
// src/core/texture/texture.ts
class TgpuTextureImpl {
  unwrap(device: GPUDevice): GPUTexture {
    if (!this._texture) {
      this._texture = device.createTexture({
        label: this._label,
        size: this._size,  // [width, height, depth]
        format: this._format,  // 'rgba8unorm', etc.
        usage: this._usage,  // GPUTextureUsage flags
        mipLevelCount: this._mipLevelCount,
        sampleCount: this._sampleCount,
        dimension: this._dimension,  // '2d', '3d', 'cube'
      });
    }
    return this._texture;
  }

  createView(): GPUTextureView {
    return this.unwrap(device).createView({
      dimension: '2d',
      format: this._format,
    });
  }
}
```

**Exact WebGPU Calls**:
1. `device.createTexture()` - Create GPU texture
2. `texture.createView()` - Create texture view

## Sampler Creation

```typescript
// src/core/sampler/sampler.ts
class TgpuSamplerImpl {
  unwrap(device: GPUDevice): GPUSampler {
    if (!this._sampler) {
      this._sampler = device.createSampler({
        label: this._label,
        magFilter: this._magFilter,  // 'linear', 'nearest'
        minFilter: this._minFilter,
        mipmapFilter: this._mipmapFilter,
        addressModeU: this._addressModeU,  // 'clamp-to-edge', 'repeat'
        addressModeV: this._addressModeV,
        addressModeW: this._addressModeW,
      });
    }
    return this._sampler;
  }
}
```

**Exact WebGPU Calls**:
1. `device.createSampler()` - Create sampler

## GPU Memory Management

### Buffer Alignment

```typescript
// Uniform buffers must be aligned to 256 bytes
const UNIFORM_ALIGNMENT = 256;

function getUniformSize(schema: AnyData): number {
  const size = getSize(schema);
  return Math.ceil(size / UNIFORM_ALIGNMENT) * UNIFORM_ALIGNMENT;
}
```

### Staging Buffer Pattern

```typescript
// For reading from GPU:
// 1. Create staging buffer with MAP_READ usage
// 2. Copy from GPU buffer to staging
// 3. Map staging buffer
// 4. Read data
// 5. Destroy staging buffer

const stagingBuffer = device.createBuffer({
  size: dataSize,
  usage: GPUBufferUsage.COPY_DST | GPUBufferUsage.MAP_READ,
});
```

### Resource Lifecycle

```typescript
// Proper cleanup pattern
class TgpuBufferImpl {
  destroy(): void {
    if (this._buffer) {
      this._buffer.destroy();
      this._buffer = undefined;
    }
  }
}

// Textures
class TgpuTextureImpl {
  destroy(): void {
    if (this._texture) {
      this._texture.destroy();
      this._texture = undefined;
    }
  }
}
```

## WebGPU API Call Summary

### Device Level
| Call | Purpose | TypeGPU Usage |
|------|---------|---------------|
| `navigator.gpu.requestAdapter()` | Get GPU adapter | init() |
| `adapter.requestDevice()` | Get logical device | init() |
| `device.createBuffer()` | Create buffer | All buffer operations |
| `device.createTexture()` | Create texture | Texture creation |
| `device.createSampler()` | Create sampler | Sampler creation |
| `device.createShaderModule()` | Compile WGSL | Pipeline creation |
| `device.createBindGroupLayout()` | Create bind group layout | Bind group system |
| `device.createBindGroup()` | Create bind group | Bind group system |
| `device.createPipelineLayout()` | Create pipeline layout | Pipeline creation |
| `device.createComputePipeline()` | Create compute pipeline | Compute functions |
| `device.createRenderPipeline()` | Create render pipeline | Render functions |
| `device.createCommandEncoder()` | Record commands | All GPU operations |

### Command Encoder Level
| Call | Purpose | TypeGPU Usage |
|------|---------|---------------|
| `encoder.copyBufferToBuffer()` | Copy between buffers | Buffer read operations |
| `encoder.beginComputePass()` | Start compute pass | Compute dispatch |
| `encoder.beginRenderPass()` | Start render pass | Rendering |
| `encoder.finish()` | Finish command buffer | Submit to queue |

### Queue Level
| Call | Purpose | TypeGPU Usage |
|------|---------|---------------|
| `device.queue.writeBuffer()` | Write to buffer | Buffer write operations |
| `device.queue.submit()` | Submit commands | All GPU operations |

### Buffer Level
| Call | Purpose | TypeGPU Usage |
|------|---------|---------------|
| `buffer.getMappedRange()` | Get CPU memory | Initial data, reads |
| `buffer.unmap()` | Commit/close mapping | After writes |
| `buffer.mapAsync()` | Async map for access | Buffer reads |
| `buffer.destroy()` | Destroy buffer | Cleanup |

### Texture Level
| Call | Purpose | TypeGPU Usage |
|------|---------|---------------|
| `texture.createView()` | Create texture view | Sampling, rendering |
| `texture.destroy()` | Destroy texture | Cleanup |

### Pass Level
| Call | Purpose | TypeGPU Usage |
|------|---------|---------------|
| `pass.setPipeline()` | Set pipeline | Compute/render |
| `pass.setBindGroup()` | Set bind group | Resource binding |
| `pass.setVertexBuffer()` | Set vertex buffer | Rendering |
| `pass.dispatchWorkgroups()` | Dispatch compute | Compute shaders |
| `pass.draw()` | Draw primitives | Rendering |
| `pass.end()` | End pass | Finalize commands |
