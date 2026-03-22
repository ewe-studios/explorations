---
name: typescript-flatbuffers
description: TypeScript implementation of FlatBuffers providing zero-copy serialization for JavaScript/TypeScript applications with full type safety
type: sub-project
source: /home/darkvoid/Boxxed/@formulas/src.rust/src.wasm/flatbuffers/ts/
---

# TypeScript FlatBuffers - Implementation

## Overview

The **TypeScript FlatBuffers** implementation brings Google's zero-copy serialization to JavaScript and TypeScript applications. It's particularly valuable for web applications, Node.js services, and any JavaScript environment that needs efficient binary data handling.

### Key Value Proposition

- **Zero-copy access** - Read data directly from ArrayBuffer without parsing
- **TypeScript-first** - Full type safety with generated `.d.ts` files
- **Cross-platform** - Works in browsers, Node.js, Deno, and other JS runtimes
- **Interoperable** - Compatible with all other FlatBuffers language implementations
- **FlexibleBuffers** - Schema-less variant for dynamic data structures

### Example Usage

```typescript
// Generated from schema.fbs using: flatc --ts schema.fbs
import * as MyGame from './monster_generated';

// Creating a buffer
const builder = new flatbuffers.Builder(1024);

// Build objects from innermost to outermost
const weaponName = builder.createString("Axe");
MyGame.Weapon.createWeapon(builder, weaponName, 10);

const monsterName = builder.createString("Orc");
MyGame.Monster.createMonster(builder, {
  name: monsterName,
  hp: 100,
  mana: 50,
  // ... other fields
});

// Finish and get the buffer
builder.finish(monsterOffset);
const bytes = builder.asUint8Array();

// Reading - zero copy!
const buffer = new flatbuffers.ByteBuffer(bytes);
const monster = MyGame.Monster.getRootAsMonster(buffer);
console.log(monster.name());  // Direct access
console.log(monster.hp());
```

## Directory Structure

```
/home/darkvoid/Boxxed/@formulas/src.rust/src.wasm/flatbuffers/ts/
├── package.json                    # NPM package configuration
├── tsconfig.json                   # TypeScript configuration
├── builder.ts                      # FlatBufferBuilder implementation
├── byte-buffer.ts                  # ByteBuffer wrapper for ArrayBuffer
├── constants.ts                    # Library constants
├── encoding.ts                     # Text encoding utilities
├── flatbuffers.ts                  # Main entry point
├── types.ts                        # Type definitions
├── utils.ts                        # Utility functions
├── flexbuffers/                    # Schema-less FlexBuffers implementation
│   ├── bit-width.ts               # Bit width enum
│   ├── bit-width-util.ts          # Bit width utilities
│   ├── builder.ts                 # FlexBuffer builder
│   ├── flexbuffers-util.ts        # FlexBuffer utilities
│   ├── reference.ts               # Reference type for navigation
│   ├── reference-util.ts          # Reference utilities
│   ├── stack-value.ts             # Stack value for building
│   ├── value-type.ts              # Value type enum
│   ├── value-type-util.ts         # Value type utilities
│   └── flexbuffers.ts             # FlexBuffers entry point
└── flexbuffers.ts                  # Re-export for flexbuffers
```

## Core Components

### 1. ByteBuffer (`byte-buffer.ts`)

Wrapper around ArrayBuffer for reading/writing binary data:

```typescript
export class ByteBuffer {
  private bytes_: Uint8Array;
  private position_: number = 0;

  constructor(size: number) {
    this.bytes_ = new Uint8Array(size);
  }

  static wrap(bytes: Uint8Array): ByteBuffer {
    const bb = new ByteBuffer(bytes.length);
    bb.bytes_ = bytes;
    return bb;
  }

  // Read operations
  readInt8(offset: number): number {
    return this.bytes_[offset];
  }

  readInt16(offset: number): number {
    let v = this.bytes_[offset] | (this.bytes_[offset + 1] << 8);
    return v | ((v & 0x8000) ? 0xFFFF0000 : 0);
  }

  readInt32(offset: number): number {
    return this.bytes_[offset] |
      (this.bytes_[offset + 1] << 8) |
      (this.bytes_[offset + 2] << 16) |
      (this.bytes_[offset + 3] << 24);
  }

  readFloat32(offset: number): number {
    const view = new DataView(this.bytes_.buffer, offset, 4);
    return view.getFloat32(0, true);  // Little endian
  }

  readFloat64(offset: number): number {
    const view = new DataView(this.bytes_.buffer, offset, 8);
    return view.getFloat64(0, true);
  }

  // Write operations
  writeInt8(offset: number, value: number): void {
    this.bytes_[offset] = value;
  }

  writeInt32(offset: number, value: number): void {
    this.bytes_[offset] = value;
    this.bytes_[offset + 1] = value >> 8;
    this.bytes_[offset + 2] = value >> 16;
    this.bytes_[offset + 3] = value >> 24;
  }

  // Position management
  setPosition(position: number): void {
    this.position_ = position;
  }

  getPosition(): number {
    return this.position_;
  }
}
```

### 2. Builder (`builder.ts`)

The FlatBufferBuilder for constructing buffers:

```typescript
export class Builder {
  private bb: ByteBuffer;
  private minkowskiBlock: Uint8Array;
  private space: number;
  private min_align: number = 1;
  private vtables: number[] = [];
  private num_vtables: number = 0;
  private vtable_in_use: number = 0;
  private isNested: boolean = false;

  constructor(opt_initial_size?: number) {
    let initial_size = opt_initial_size || 1024;
    this.bb = new ByteBuffer(initial_size);
    this.space = initial_size;
    this.minkowskiBlock = new Uint8Array(16);
  }

  // Alignment
  prep(size: number, additional_bytes: number): void {
    const align_size = size;
    let pt = this.space & ~(align_size - 1);
    const needed = align_size - (this.space - pt);

    this.space -= needed;
    this.bb.writeInt32(this.space, 0);  // Padding

    this.space -= additional_bytes;
  }

  // Write operations
  writeInt8(value: number): void {
    this.bb.writeInt8(this.space - 1, value);
    this.space -= 1;
  }

  writeInt32(value: number): void {
    this.bb.writeInt32(this.space - 4, value);
    this.space -= 4;
  }

  // String creation
  createString(s: string | Uint8Array): Offset {
    if (s instanceof Uint8Array) {
      return this.createByteVector(s);
    }

    const utf8 = new TextEncoder().encode(s);
    return this.createByteVector(utf8);
  }

  createByteVector(bytes: Uint8Array): Offset {
    this.prep(4, bytes.length + 4);
    this.bb.setPosition(this.space);

    for (let i = bytes.length - 1; i >= 0; i--) {
      this.bb.writeInt8(this.space - (bytes.length - i), bytes[i]);
    }

    this.bb.writeInt32(this.space - 4, bytes.length);
    return this.space;
  }

  // Vector creation
  startVector(elem_size: number, num_elems: number, alignment: number): number {
    this.nest();
    this.prep(4, num_elems * elem_size);
    this.prep(alignment, num_elems * elem_size);
    return this.space;
  }

  endVector(vector_start: number, num_elems: number): Offset {
    this.bb.writeInt32(this.space - 4, num_elems);
    this.unnest();
    return this.space;
  }

  // VTable management
  startObject(numfields: number): void {
    this.vtable_in_use = numfields;
    // Initialize vtable fields to 0
  }

  addInt8(field: number, value: number, default_value: number): void {
    // Add field to vtable
  }

  addInt32(field: number, value: number, default_value: number): void {
    // Add field to vtable
  }

  addOffset(field: number, value: Offset, default_value: Offset): void {
    // Add offset field
  }

  endObject(): Offset {
    // Write vtable and object
    // Return offset to object
  }

  // Finish and retrieve buffer
  finish(root_table: Offset, opt_file_identifier?: string): void {
    this.prep(this.min_align, 4);
    this.bb.writeInt32(this.space - 4, root_table);

    if (opt_file_identifier) {
      // Write 4-byte file identifier
    }
  }

  dataView(): DataView {
    return new DataView(this.bb.bytes().buffer, this.space);
  }

  asUint8Array(): Uint8Array {
    return this.bb.bytes().subarray(this.space);
  }
}
```

### 3. Table Base Class

Base class for all generated table types:

```typescript
export class Table {
  protected bb: ByteBuffer;
  protected i: number;

  protected _wrap(i: number, bb: ByteBuffer): void {
    this.i = i;
    this.bb = bb;
  }

  protected __offset(field: number): number {
    const vtable = this.i - this.bb.readInt32(this.i);
    const vtable_offset = this.bb.readInt16(vtable + field);
    return vtable_offset !== 0 ? vtable_offset : 0;
  }

  protected __vector(field: number): number {
    const offset = this.__offset(field);
    return offset !== 0 ? this.i + offset : 0;
  }

  protected __union(field: number): number {
    const offset = this.__offset(field);
    return offset !== 0 ? this.i + offset : 0;
  }

  protected __field(field: number, type: FieldType, default_value: any): any {
    const offset = this.__offset(field);
    if (offset === 0) return default_value;

    switch (type) {
      case FieldType.INT8:
        return this.bb.readInt8(this.i + offset);
      case FieldType.INT32:
        return this.bb.readInt32(this.i + offset);
      case FieldType.FLOAT32:
        return this.bb.readFloat32(this.i + offset);
      // ... other types
    }
  }
}
```

### 4. Generated Code Example

```typescript
// monster_generated.ts
export class Monster extends Table {
  static getRootAsMonster(bb: ByteBuffer, obj?: Monster): Monster {
    return (obj || new Monster())._wrap(
      bb.readInt32(bb.position()) + bb.position(),
      bb
    );
  }

  name(optionalEncoding?: flatbuffers.Encoding): string | null {
    const offset = this.__offset(4);
    return offset ? this.__string(this.i + offset) : null;
  }

  hp(): number {
    const offset = this.__offset(6);
    return offset ? this.bb.readInt32(this.i + offset) : 100;
  }

  mana(): number {
    const offset = this.__offset(8);
    return offset ? this.bb.readInt32(this.i + offset) : 50;
  }

  pos(obj?: Vec3): Vec3 | null {
    const offset = this.__offset(10);
    return offset ?
      (obj || new Vec3())._wrap(this.i + offset, this.bb) :
      null;
  }

  weapons(index: number, obj?: Weapon): Weapon | null {
    const offset = this.__offset(12);
    return offset ?
      (obj || new Weapon())._wrap(
        this.__vector(offset) + index * 4,
        this.bb
      ) :
      null;
  }

  weaponsLength(): number {
    const offset = this.__offset(12);
    return offset ? this.bb.readInt32(this.i + offset) : 0;
  }
}

export class Vec3 {
  x(): number { return this.bb.readFloat32(this.i); }
  y(): number { return this.bb.readFloat32(this.i + 4); }
  z(): number { return this.bb.readFloat32(this.i + 8); }
}
```

### 5. FlexBuffers Implementation

Schema-less variant for dynamic data:

```typescript
// flexbuffers/builder.ts
export class Builder {
  private stack: StackValue[] = [];
  private buffer: Uint8Array;
  private finished: boolean = false;

  pushInt(value: number): void {
    this.stack.push(StackValue.int(value));
  }

  pushFloat(value: number): void {
    this.stack.push(StackValue.float(value));
  }

  pushString(value: string): void {
    this.stack.push(StackValue.string(value));
  }

  startVector(): void {
    this.stack.push(StackValue.vectorMarker());
  }

  endVector(): void {
    // Pop values until marker, create vector
  }

  startMap(): void {
    this.stack.push(StackValue.mapMarker());
  }

  endMap(): void {
    // Pop values until marker, create map with sorted keys
  }

  finish(): Uint8Array {
    // Finalize and return buffer
  }
}

// flexbuffers/reference.ts
export class Reference {
  private buffer: Uint8Array;
  private offset: number;
  private parent_width: number;
  private cached_type: ValueType | null = null;

  static create(buffer: Uint8Array, offset: number, width: number): Reference {
    return new Reference(buffer, offset, width);
  }

  private get subtype(): ValueType {
    if (!this.cached_type) {
      const bit_width = this.read_bit_width();
      const type_byte = this.read_type_byte(bit_width);
      this.cached_type = type_byte & 0x0F;
    }
    return this.cached_type;
  }

  asInt(): number {
    return this.read_int();
  }

  asFloat(): number {
    return this.read_float();
  }

  asString(): string {
    const str_offset = this.read_offset();
    const str_len = this.read_length();
    return this.read_string(str_offset, str_len);
  }

  asVector(): VectorReference {
    return new VectorReference(this.buffer, this.offset, this.parent_width);
  }

  asMap(): MapReference {
    return new MapReference(this.buffer, this.offset, this.parent_width);
  }
}
```

## FlexBuffers Memory Layout

```
┌─────────────────────────────────────────────────────────────────┐
│                    FlexBuffer Layout                             │
│                                                                 │
│  ┌───────────────────────────────────────────────────────────┐  │
│  │                   Data Section                             │  │
│  │   ┌──────┐ ┌──────┐ ┌──────┐ ┌──────────────────┐        │  │
│  │   │ Int  │ │ Float│ │ String │   Vector Data    │  ...   │  │
│  │   │  42  │ │ 3.14 │ │ "hello"│   [1, 2, 3, 4]   │        │  │
│  │   └──────┘ └──────┘ └────────┴──────────────────┘        │  │
│  └───────────────────────────────────────────────────────────┘  │
│                                                                 │
│  ┌───────────────────────────────────────────────────────────┐  │
│  │                  Control Section                           │  │
│  │   ┌─────────┐ ┌─────────┐ ┌─────────────────────────┐    │  │
│  │   │  Type   │ │ Length  │    Parent Bit Width       │    │  │
│  │   │ (4 bit) │ │(varies) │      (2 bits)             │    │  │
│  │   └─────────┘ └─────────┘ └─────────────────────────┘    │  │
│  └───────────────────────────────────────────────────────────┘  │
│                                                                 │
│  End →  └───────────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────────────────┘
```

## Performance Comparison

| Operation | TypeScript FlatBuffers | JSON.parse |
|-----------|----------------------|------------|
| Deserialize | O(1) - no copy | O(n) - full parse |
| Field access | O(1) | O(1) after parse |
| Memory | 1x buffer size | 2-3x JSON size |
| Serialization | O(n) | O(n) |

## Browser Usage

```typescript
// Fetch a FlatBuffer from network
async function loadMonster(url: string): Promise<Monster> {
  const response = await fetch(url);
  const arrayBuffer = await response.arrayBuffer();
  const bytes = new Uint8Array(arrayBuffer);

  const buffer = new flatbuffers.ByteBuffer(bytes);
  return Monster.getRootAsMonster(buffer);
}

// Use in React/Vue/Angular
function MonsterCard({ url }: { url: string }) {
  const [monster, setMonster] = useState<Monster | null>(null);

  useEffect(() => {
    loadMonster(url).then(setMonster);
  }, [url]);

  if (!monster) return <div>Loading...</div>;

  return (
    <div>
      <h2>{monster.name()}</h2>
      <p>HP: {monster.hp()}</p>
      <p>Mana: {monster.mana()}</p>
    </div>
  );
}
```

## Node.js Usage

```typescript
import * as fs from 'fs';
import * as flatbuffers from 'flatbuffers';
import { Monster } from './monster_generated';

// Read from file
const buffer = fs.readFileSync('monster.bin');
const bb = new flatbuffers.ByteBuffer(new Uint8Array(buffer));
const monster = Monster.getRootAsMonster(bb);

// Write to file
const builder = new flatbuffers.Builder(1024);
// ... build monster
const data = builder.asUint8Array();
fs.writeFileSync('monster.bin', data);
```

## Key Insights

1. **TypeScript provides type safety** - Generated `.d.ts` files give full IDE support
2. **ByteBuffer abstraction** - Clean interface over ArrayBuffer
3. **FlexBuffers for dynamic data** - Schema-less option when needed
4. **Compatible with all FlatBuffers** - Interoperable with Rust, Go, Java, etc.
5. **Zero-copy in JavaScript** - Unusual but powerful pattern for JS

## Open Questions

- How does TypeScript performance compare to Rust for large buffers?
- What are the limitations of FlexBuffers vs regular FlatBuffers?
- How to handle schema evolution in TypeScript specifically?
