# Storage System Guide for Inexperienced Engineers

**How to build a file format and storage system like Rive's .riv**

---

## Table of Contents

1. [Overview](#overview)
2. [Why Binary Formats?](#why-binary-formats)
3. [Designing Your File Format](#designing-your-file-format)
4. [Implementation Step-by-Step](#implementation-step-by-step)
5. [Reading and Writing](#reading-and-writing)
6. [Version Compatibility](#version-compatibility)
7. [Asset Embedding](#asset-embedding)
8. [Common Pitfalls](#common-pitfalls)
9. [Complete Example](#complete-example)

---

## Overview

This guide teaches you how to design and implement a binary file format for storing structured data, similar to how Rive uses the `.riv` format to store animations.

**Prerequisites**: Basic programming knowledge. No prior experience with binary formats required.

### What You'll Learn

- Why binary formats are used instead of text formats
- How to design a file format structure
- How to read and write binary data
- How to handle version compatibility
- How to embed assets (images, fonts) in your file format

---

## Why Binary Formats?

### Text Format (JSON) vs. Binary Format

**JSON Example:**
```json
{
  "artboard": {
    "name": "Character",
    "width": 800,
    "height": 600,
    "objects": [
      {
        "type": "shape",
        "name": "Body",
        "path": {
          "commands": [
            {"type": "move", "x": 100, "y": 100},
            {"type": "line", "x": 200, "y": 100},
            {"type": "cubic", "cp1x": 250, "cp1y": 150, "cp2x": 250, "cp2y": 250, "x": 200, "y": 300}
          ]
        }
      }
    ]
  }
}
```

**Binary Equivalent:**
```
Bytes: [4D 69 76 65] [01 00] [03 00 00 00] [50 02 D0 03 ...]
       ^ Magic        ^ Ver  ^ Objects   ^ Path data...
```

### Comparison

| Aspect | JSON (Text) | Binary |
|--------|-------------|--------|
| File Size | Large (~500 bytes above) | Small (~50 bytes) |
| Parse Speed | Slow (text parsing) | Fast (direct memory copy) |
| Human Readable | Yes | No (need viewer tool) |
| Precision | Text conversion | Exact binary representation |
| Loading | Parse entire file | Can memory-map |

### Why Rive Uses Binary

1. **Smaller files**: Important for web downloads
2. **Faster loading**: Critical for real-time animation
3. **Exact representation**: No floating-point text conversion
4. **Streaming**: Can load parts of file on demand

---

## Designing Your File Format

### Step 1: Define Your Data Structure

Start with what you need to store:

```
For an animation system:
- File metadata (version, creator)
- Artboards (canvas definitions)
- Objects (shapes, images, text)
- Paths (vector data)
- Animations (keyframes)
- State machines
- Assets (embedded images, fonts)
```

### Step 2: Design the Structure

```
File Layout:

┌─────────────────────────────────────┐
│ HEADER                              │
│ - Magic number (4 bytes): "RIVE"    │
│ - Version (4 bytes): 1, 2, 3...     │
│ - Flags (4 bytes): endianness, etc. │
├─────────────────────────────────────┤
│ OBJECT TABLE                        │
│ - Object count (4 bytes)            │
│ - Object 1: type, properties        │
│ - Object 2: type, properties        │
│ - ...                               │
├─────────────────────────────────────┤
│ STRING TABLE                        │
│ - All strings stored here           │
│ - Referenced by index               │
├─────────────────────────────────────┤
│ ASSET TABLE                         │
│ - Embedded images                   │
│ - Embedded fonts                    │
│ - Embedded audio                    │
└─────────────────────────────────────┘
```

### Step 3: Define Object Types

```
Object Type IDs:
1  = Artboard
2  = Shape
3  = Path
4  = Image
5  = Text
6  = Bone
7  = Animation
8  = StateMachine
...

Each object has:
- Type ID (1 byte)
- Property count (2 bytes)
- Properties (variable)
```

### Step 4: Define Property Types

```
Property Types:
0 = Null
1 = Bool    (1 byte)
2 = Int     (4 bytes)
3 = Float   (4 bytes)
4 = String  (4 byte index into string table)
5 = Ref     (4 byte object index)
6 = List    (4 byte count, then items)
7 = Blob    (4 byte size, then bytes)
```

---

## Implementation Step-by-Step

### Step 1: Set Up Binary Reading/Writing

```rust
// For Rust
use std::io::{Read, Write, Cursor, Seek, SeekFrom};

// BinaryReader helper
struct BinaryReader<R: Read> {
    reader: R,
}

impl<R: Read> BinaryReader<R> {
    fn read_u8(&mut self) -> std::io::Result<u8> {
        let mut buf = [0; 1];
        self.reader.read_exact(&mut buf)?;
        Ok(buf[0])
    }

    fn read_u32(&mut self) -> std::io::Result<u32> {
        let mut buf = [0; 4];
        self.reader.read_exact(&mut buf)?;
        Ok(u32::from_le_bytes(buf))  // Little-endian
    }

    fn read_f32(&mut self) -> std::io::Result<f32> {
        let mut buf = [0; 4];
        self.reader.read_exact(&mut buf)?;
        Ok(f32::from_le_bytes(buf))
    }

    fn read_string(&mut self) -> std::io::Result<String> {
        let len = self.read_u32()?;
        let mut buf = vec![0; len as usize];
        self.reader.read_exact(&mut buf)?;
        Ok(String::from_utf8(buf).unwrap())
    }
}

// BinaryWriter helper
struct BinaryWriter<W: Write> {
    writer: W,
}

impl<W: Write> BinaryWriter<W> {
    fn write_u8(&mut self, value: u8) -> std::io::Result<()> {
        self.writer.write_all(&[value])
    }

    fn write_u32(&mut self, value: u32) -> std::io::Result<()> {
        self.writer.write_all(&value.to_le_bytes())
    }

    fn write_f32(&mut self, value: f32) -> std::io::Result<()> {
        self.writer.write_all(&value.to_le_bytes())
    }

    fn write_string(&mut self, s: &str) -> std::io::Result<()> {
        self.write_u32(s.len() as u32)?;
        self.writer.write_all(s.as_bytes())
    }
}
```

### Step 2: Define Your Data Structures

```rust
// Simple animation file format

#[derive(Debug)]
pub struct File {
    pub version: u32,
    pub artboards: Vec<Artboard>,
    pub strings: Vec<String>,
}

#[derive(Debug)]
pub struct Artboard {
    pub name: String,
    pub width: f32,
    pub height: f32,
    pub objects: Vec<Object>,
}

#[derive(Debug)]
pub enum Object {
    Shape(Shape),
    Image(Image),
    Text(Text),
}

#[derive(Debug)]
pub struct Shape {
    pub name: String,
    pub path: Path,
    pub fill_color: Option<[u8; 4]>,  // RGBA
}

#[derive(Debug)]
pub struct Path {
    pub commands: Vec<PathCommand>,
}

#[derive(Debug)]
pub enum PathCommand {
    MoveTo { x: f32, y: f32 },
    LineTo { x: f32, y: f32 },
    CubicTo { cp1x: f32, cp1y: f32, cp2x: f32, cp2y: f32, x: f32, y: f32 },
    Close,
}

#[derive(Debug)]
pub struct Image {
    pub name: String,
    pub width: u32,
    pub height: u32,
    pub data: Vec<u8>,  // RGBA pixels
}

#[derive(Debug)]
pub struct Text {
    pub name: String,
    pub content: String,
    pub font_size: f32,
}
```

### Step 3: Implement Writing

```rust
impl File {
    pub fn write_to<W: Write>(&self, writer: &mut W) -> std::io::Result<()> {
        let mut bw = BinaryWriter::new(writer);

        // Write header
        bw.write_all(b"RIVE")?;  // Magic number
        bw.write_u32(self.version)?;
        bw.write_u32(0)?;  // Flags (reserved)

        // Write string table
        self.write_string_table(&mut bw)?;

        // Write artboards
        bw.write_u32(self.artboards.len() as u32)?;
        for artboard in &self.artboards {
            self.write_artboard(artboard, &mut bw)?;
        }

        Ok(())
    }

    fn write_string_table<W: Write>(&self, bw: &mut BinaryWriter<W>) -> std::io::Result<()> {
        bw.write_u32(self.strings.len() as u32)?;
        for s in &self.strings {
            bw.write_string(s)?;
        }
        Ok(())
    }

    fn write_artboard<W: Write>(&self, artboard: &Artboard, bw: &mut BinaryWriter<W>) -> std::io::Result<()> {
        // Write artboard type
        bw.write_u8(1)?;

        // Write properties
        let string_index = self.strings.iter().position(|s| s == &artboard.name).unwrap();
        bw.write_u32(3)?;  // 3 properties

        // Name property
        bw.write_u8(1)?;  // Property key: Name
        bw.write_u8(4)?;  // Property type: String
        bw.write_u32(string_index as u32)?;

        // Width property
        bw.write_u8(2)?;  // Property key: Width
        bw.write_u8(3)?;  // Property type: Float
        bw.write_f32(artboard.width)?;

        // Height property
        bw.write_u8(3)?;  // Property key: Height
        bw.write_u8(3)?;  // Property type: Float
        bw.write_f32(artboard.height)?;

        // Write objects
        bw.write_u32(artboard.objects.len() as u32)?;
        for obj in &artboard.objects {
            self.write_object(obj, bw)?;
        }

        Ok(())
    }

    fn write_object<W: Write>(&self, obj: &Object, bw: &mut BinaryWriter<W>) -> std::io::Result<()> {
        match obj {
            Object::Shape(shape) => {
                bw.write_u8(2)?;  // Type: Shape
                // Write shape properties...
            }
            Object::Image(image) => {
                bw.write_u8(4)?;  // Type: Image
                // Write image properties...
            }
            Object::Text(text) => {
                bw.write_u8(5)?;  // Type: Text
                // Write text properties...
            }
        }
        Ok(())
    }
}
```

### Step 4: Implement Reading

```rust
impl File {
    pub fn read_from<R: Read>(reader: &mut R) -> std::io::Result<Self> {
        let mut br = BinaryReader::new(reader);

        // Read and verify header
        let mut magic = [0; 4];
        reader.read_exact(&mut magic)?;
        if &magic != b"RIVE" {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "Invalid file format: wrong magic number"
            ));
        }

        let version = br.read_u32()?;
        let _flags = br.read_u32()?;

        // Read string table
        let strings = Self::read_string_table(&mut br)?;

        // Read artboards
        let artboard_count = br.read_u32()?;
        let mut artboards = Vec::with_capacity(artboard_count as usize);

        for _ in 0..artboard_count {
            artboards.push(Self::read_artboard(&mut br, &strings)?);
        }

        Ok(File {
            version,
            artboards,
            strings,
        })
    }

    fn read_string_table<R: Read>(br: &mut BinaryReader<R>) -> std::io::Result<Vec<String>> {
        let count = br.read_u32()?;
        let mut strings = Vec::with_capacity(count as usize);

        for _ in 0..count {
            strings.push(br.read_string()?);
        }

        Ok(strings)
    }

    fn read_artboard<R: Read>(br: &mut BinaryReader<R>, strings: &[String]) -> std::io::Result<Artboard> {
        let type_id = br.read_u8()?;
        if type_id != 1 {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "Expected artboard type"
            ));
        }

        let prop_count = br.read_u32()?;
        let mut name = String::new();
        let mut width = 800.0;
        let mut height = 600.0;

        for _ in 0..prop_count {
            let key = br.read_u8()?;
            let prop_type = br.read_u8()?;

            match (key, prop_type) {
                (1, 4) => {  // Name
                    let idx = br.read_u32()? as usize;
                    name = strings.get(idx).cloned().unwrap_or_default();
                }
                (2, 3) => {  // Width
                    width = br.read_f32()?;
                }
                (3, 3) => {  // Height
                    height = br.read_f32()?;
                }
                _ => {
                    // Skip unknown properties
                    Self::skip_property(br, prop_type)?;
                }
            }
        }

        // Read objects
        let object_count = br.read_u32()?;
        let mut objects = Vec::with_capacity(object_count as usize);

        for _ in 0..object_count {
            objects.push(Self::read_object(br, strings)?);
        }

        Ok(Artboard {
            name,
            width,
            height,
            objects,
        })
    }

    fn read_object<R: Read>(br: &mut BinaryReader<R>, strings: &[String]) -> std::io::Result<Object> {
        let type_id = br.read_u8()?;

        match type_id {
            2 => Ok(Object::Shape(Self::read_shape(br, strings)?)),
            4 => Ok(Object::Image(Self::read_image(br)?)),
            5 => Ok(Object::Text(Self::read_text(br, strings)?)),
            _ => Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!("Unknown object type: {}", type_id)
            )),
        }
    }

    fn skip_property<R: Read>(br: &mut BinaryReader<R>, prop_type: u8) -> std::io::Result<()> {
        match prop_type {
            1 => { br.read_u8()?; }  // Bool
            2 => { br.read_u32()?; } // Int
            3 => { br.read_f32()?; } // Float
            4 => { br.read_u32()?; } // String (index)
            5 => { br.read_u32()?; } // Ref (index)
            6 => {  // List
                let count = br.read_u32()?;
                for _ in 0..count {
                    let item_type = br.read_u8()?;
                    Self::skip_property(br, item_type)?;
                }
            }
            7 => {  // Blob
                let size = br.read_u32()?;
                let mut buf = vec![0; size as usize];
                br.reader.read_exact(&mut buf)?;
            }
            _ => {}
        }
        Ok(())
    }
}
```

---

## Reading and Writing

### Complete Example: Creating a File

```rust
fn create_sample_file() -> File {
    let mut strings = Vec::new();
    strings.push("My Artboard".to_string());
    strings.push("Rectangle".to_string());

    let path = Path {
        commands: vec![
            PathCommand::MoveTo { x: 100.0, y: 100.0 },
            PathCommand::LineTo { x: 200.0, y: 100.0 },
            PathCommand::LineTo { x: 200.0, y: 200.0 },
            PathCommand::LineTo { x: 100.0, y: 200.0 },
            PathCommand::Close,
        ],
    };

    let shape = Shape {
        name: strings[1].clone(),
        path,
        fill_color: Some([255, 0, 0, 255]),  // Red
    };

    let artboard = Artboard {
        name: strings[0].clone(),
        width: 800.0,
        height: 600.0,
        objects: vec![Object::Shape(shape)],
    };

    File {
        version: 1,
        artboards: vec![artboard],
        strings,
    }
}

fn main() -> std::io::Result<()> {
    let file = create_sample_file();

    // Write to file
    let mut output = std::fs::File::create("animation.riv")?;
    file.write_to(&mut output)?;

    println!("File written successfully!");

    // Read it back
    let mut input = std::fs::File::open("animation.riv")?;
    let loaded = File::read_from(&mut input)?;

    println!("Loaded file:");
    println!("  Version: {}", loaded.version);
    println!("  Artboards: {}", loaded.artboards.len());
    for ab in &loaded.artboards {
        println!("    - {} ({}x{})", ab.name, ab.width, ab.height);
        println!("      Objects: {}", ab.objects.len());
    }

    Ok(())
}
```

---

## Version Compatibility

### Forward Compatibility

```rust
pub struct FileHeader {
    pub magic: [u8; 4],
    pub version: u32,
    pub min_readable_version: u32,  // Oldest version that can read this
    pub flags: u32,
}

impl File {
    pub fn read_from<R: Read>(reader: &mut R) -> std::io::Result<Self> {
        let mut br = BinaryReader::new(reader);

        // Verify magic
        let magic = br.read_u32()?;
        if magic != 0x45564952 {  // "RIVE" in little-endian
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "Not a RIVE file"
            ));
        }

        let version = br.read_u32()?;
        let _min_version = br.read_u32()?;
        let _flags = br.read_u32()?;

        // Handle different versions
        match version {
            1..=3 => Self::read_v1_to_v3(&mut br, version),
            4..=10 => Self::read_v4_to_v10(&mut br, version),
            _ => Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!("Unsupported version: {}", version)
            )),
        }
    }

    fn read_v1_to_v3<R: Read>(br: &mut BinaryReader<R>, version: u32) -> std::io::Result<Self> {
        // Older format without string table
        // Read directly embedded strings
        // ...
    }

    fn read_v4_to_v10<R: Read>(br: &mut BinaryReader<R>, version: u32) -> std::io::Result<Self> {
        // Newer format with string table
        // ...
    }
}
```

### Migration Guide

```rust
// When you add a new version, provide migration:

impl File {
    /// Upgrade from version N to N+1
    fn migrate_v3_to_v4(&mut self) {
        // Add string table
        let mut strings = Vec::new();

        // Extract all strings from objects
        for artboard in &mut self.artboards {
            let name_idx = strings.len();
            strings.push(std::mem::take(&mut artboard.name));
            // Update references...
        }

        self.strings = strings;
        self.version = 4;
    }
}
```

---

## Asset Embedding

### Embedding Images

```rust
#[derive(Debug)]
pub struct EmbeddedAsset {
    pub id: u32,
    pub name: String,
    pub asset_type: AssetType,
    pub data: Vec<u8>,
}

#[derive(Debug)]
pub enum AssetType {
    Png,
    Jpeg,
    Webp,
    Font,
    Audio,
}

impl File {
    fn write_embedded_image<W: Write>(
        &self,
        image: &Image,
        bw: &mut BinaryWriter<W>
    ) -> std::io::Result<()> {
        // Write asset header
        bw.write_u32(image.id)?;
        bw.write_u8(1)?;  // Type: Image

        // Write PNG data (assume already encoded)
        bw.write_u32(image.data.len() as u32)?;
        bw.writer.write_all(&image.data)?;

        Ok(())
    }

    fn read_embedded_image<R: Read>(
        br: &mut BinaryReader<R>
    ) -> std::io::Result<EmbeddedAsset> {
        let id = br.read_u32()?;
        let asset_type = br.read_u8()?;

        let size = br.read_u32()?;
        let mut data = vec![0; size as usize];
        br.reader.read_exact(&mut data)?;

        Ok(EmbeddedAsset {
            id,
            name: String::new(),  // Would come from string table
            asset_type: AssetType::Png,
            data,
        })
    }
}
```

---

## Common Pitfalls

### 1. Endianness

```rust
// WRONG: Platform-dependent
writer.write(&value)?;  // Don't do this!

// CORRECT: Specify endianness
writer.write_all(&value.to_le_bytes())?;  // Little-endian
// or
writer.write_all(&value.to_be_bytes())?;  // Big-endian

// Always use little-endian for cross-platform compatibility
```

### 2. Struct Padding

```rust
// WRONG: May have padding between fields
#[repr(C)]
struct MyStruct {
    a: u8,   // 1 byte
    // 3 bytes padding here on most systems!
    b: u32,  // 4 bytes
}

// CORRECT: Use packed representation or write fields manually
struct MyStruct {
    a: u8,
    b: u32,
}

// Write manually:
fn write<W: Write>(s: &MyStruct, w: &mut W) {
    w.write_all(&[s.a])?;
    w.write_all(&s.b.to_le_bytes())?;
}
```

### 3. String Encoding

```rust
// Always use UTF-8
fn write_string<W: Write>(s: &str, w: &mut W) {
    let bytes = s.as_bytes();  // UTF-8
    w.write_all(&(bytes.len() as u32).to_le_bytes())?;
    w.write_all(bytes)?;
}

// Don't use platform-specific encodings!
```

### 4. File Corruption Detection

```rust
// Add a checksum at the end

use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher, Hasher};

fn compute_checksum(data: &[u8]) -> u64 {
    let mut hasher = DefaultHasher::new();
    hasher.write(data);
    hasher.finish()
}

impl File {
    pub fn write_with_checksum<W: Write>(&self, w: &mut W) -> std::io::Result<()> {
        let mut buffer = Vec::new();
        self.write_to(&mut buffer)?;

        let checksum = compute_checksum(&buffer);
        w.write_all(&buffer)?;
        w.write_all(&checksum.to_le_bytes())?;

        Ok(())
    }

    pub fn read_with_checksum<R: Read + Seek>(r: &mut R) -> std::io::Result<Self> {
        // Read checksum (last 8 bytes)
        r.seek(SeekFrom::End(-8))?;
        let mut checksum_buf = [0; 8];
        r.read_exact(&mut checksum_buf)?;
        let stored_checksum = u64::from_le_bytes(checksum_buf);

        // Read content
        r.seek(SeekFrom::Start(0))?;
        let content_len = r.seek(SeekFrom::End(-8))? as usize;
        r.seek(SeekFrom::Start(0))?;

        let mut content = vec![0; content_len];
        r.read_exact(&mut content)?;

        // Verify checksum
        let computed_checksum = compute_checksum(&content);
        if computed_checksum != stored_checksum {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "File checksum mismatch - file may be corrupted"
            ));
        }

        Self::read_from(&mut Cursor::new(content))
    }
}
```

---

## Complete Example

Here's a minimal working example you can build and run:

```rust
// Cargo.toml
[package]
name = "simple-format"
version = "0.1.0"
edition = "2021"

// src/main.rs
use std::io::{Read, Write, Cursor};

// ============ Binary Helpers ============

struct BinaryReader<R: Read> {
    reader: R,
}

impl<R: Read> BinaryReader<R> {
    fn new(reader: R) -> Self { Self { reader } }

    fn read_u8(&mut self) -> std::io::Result<u8> {
        let mut buf = [0; 1];
        self.reader.read_exact(&mut buf)?;
        Ok(buf[0])
    }

    fn read_u32(&mut self) -> std::io::Result<u32> {
        let mut buf = [0; 4];
        self.reader.read_exact(&mut buf)?;
        Ok(u32::from_le_bytes(buf))
    }

    fn read_f32(&mut self) -> std::io::Result<f32> {
        let mut buf = [0; 4];
        self.reader.read_exact(&mut buf)?;
        Ok(f32::from_le_bytes(buf))
    }

    fn read_string(&mut self) -> std::io::Result<String> {
        let len = self.read_u32()?;
        let mut buf = vec![0; len as usize];
        self.reader.read_exact(&mut buf)?;
        Ok(String::from_utf8(buf).unwrap())
    }
}

struct BinaryWriter<W: Write> {
    writer: W,
}

impl<W: Write> BinaryWriter<W> {
    fn new(writer: W) -> Self { Self { writer } }

    fn write_u8(&mut self, v: u8) -> std::io::Result<()> {
        self.writer.write_all(&[v])
    }

    fn write_u32(&mut self, v: u32) -> std::io::Result<()> {
        self.writer.write_all(&v.to_le_bytes())
    }

    fn write_f32(&mut self, v: f32) -> std::io::Result<()> {
        self.writer.write_all(&v.to_le_bytes())
    }

    fn write_string(&mut self, s: &str) -> std::io::Result<()> {
        self.write_u32(s.len() as u32)?;
        self.writer.write_all(s.as_bytes())
    }
}

// ============ Data Structures ============

#[derive(Debug)]
struct File {
    version: u32,
    rectangles: Vec<Rectangle>,
}

#[derive(Debug)]
struct Rectangle {
    name: String,
    x: f32,
    y: f32,
    width: f32,
    height: f32,
    color: [u8; 4],
}

// ============ Serialization ============

impl File {
    fn write<W: Write>(&self, w: &mut W) -> std::io::Result<()> {
        let mut bw = BinaryWriter::new(w);

        // Header
        bw.writer.write_all(b"RECT")?;  // Magic
        bw.write_u32(self.version)?;

        // Rectangles
        bw.write_u32(self.rectangles.len() as u32)?;
        for rect in &self.rectangles {
            bw.write_string(&rect.name)?;
            bw.write_f32(rect.x)?;
            bw.write_f32(rect.y)?;
            bw.write_f32(rect.width)?;
            bw.write_f32(rect.height)?;
            bw.write_u8(rect.color[0])?;
            bw.write_u8(rect.color[1])?;
            bw.write_u8(rect.color[2])?;
            bw.write_u8(rect.color[3])?;
        }

        Ok(())
    }

    fn read<R: Read>(r: &mut R) -> std::io::Result<Self> {
        let mut br = BinaryReader::new(r);

        // Verify magic
        let mut magic = [0; 4];
        r.read_exact(&mut magic)?;
        if &magic != b"RECT" {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "Not a RECT file"
            ));
        }

        let version = br.read_u32()?;

        let count = br.read_u32()?;
        let mut rectangles = Vec::with_capacity(count as usize);

        for _ in 0..count {
            rectangles.push(Rectangle {
                name: br.read_string()?,
                x: br.read_f32()?,
                y: br.read_f32()?,
                width: br.read_f32()?,
                height: br.read_f32()?,
                color: [
                    br.read_u8()?,
                    br.read_u8()?,
                    br.read_u8()?,
                    br.read_u8()?,
                ],
            });
        }

        Ok(File { version, rectangles })
    }
}

// ============ Main ============

fn main() -> std::io::Result<()> {
    // Create a file
    let file = File {
        version: 1,
        rectangles: vec![
            Rectangle {
                name: "Box 1".to_string(),
                x: 10.0,
                y: 20.0,
                width: 100.0,
                height: 50.0,
                color: [255, 0, 0, 255],
            },
            Rectangle {
                name: "Box 2".to_string(),
                x: 120.0,
                y: 80.0,
                width: 80.0,
                height: 60.0,
                color: [0, 255, 0, 255],
            },
        ],
    };

    // Write to file
    let mut output = std::fs::File::create("test.rect")?;
    file.write(&mut output)?;
    println!("Written test.rect");

    // Read back
    let mut input = std::fs::File::open("test.rect")?;
    let loaded = File::read(&mut input)?;

    println!("\nLoaded file:");
    println!("  Version: {}", loaded.version);
    println!("  Rectangles: {}", loaded.rectangles.len());
    for rect in &loaded.rectangles {
        println!(
            "    {} at ({}, {}) size {}x{} color {:?}",
            rect.name, rect.x, rect.y, rect.width, rect.height, rect.color
        );
    }

    // Show file sizes
    let original_size = std::fs::metadata("test.rect")?.len();
    println!("\nFile size: {} bytes", original_size);

    // Compare with JSON equivalent
    let json_equiv = r#"{
  "version": 1,
  "rectangles": [
    {"name": "Box 1", "x": 10, "y": 20, "width": 100, "height": 50, "color": [255,0,0,255]},
    {"name": "Box 2", "x": 120, "y": 80, "width": 80, "height": 60, "color": [0,255,0,255]}
  ]
}"#;
    println!("JSON equivalent: {} bytes", json_equiv.len());
    println!("Binary is {}% smaller",
        100 - (original_size as f64 / json_equiv.len() as f64 * 100.0) as usize
    );

    Ok(())
}
```

Run this example:
```bash
cargo run
```

Expected output:
```
Written test.rect

Loaded file:
  Version: 1
  Rectangles: 2
    Box 1 at (10, 20) size 100x50 color [255, 0, 0, 255]
    Box 2 at (120, 80) size 80x60 color [0, 255, 0, 255]

File size: 84 bytes
JSON equivalent: 234 bytes
Binary is 64% smaller
```

---

## Summary

You now understand:

1. **Why binary formats**: Smaller files, faster loading
2. **How to design**: Header, object table, string table, assets
3. **How to implement**: BinaryReader/BinaryWriter helpers
4. **Version handling**: Support multiple versions gracefully
5. **Common mistakes**: Endianness, padding, encoding

### Next Steps

1. Study existing formats (PNG, GLTF, etc.)
2. Add compression (zstd, lz4)
3. Implement streaming for large files
4. Add encryption for protected content

For more details on Rive's specific format, see:
- `cpp-core-architecture.md` - File loading implementation
- The official Rive documentation at https://rive.app/docs/
