# Rive C++ Core Architecture

**Source:** `/home/darkvoid/Boxxed/@formulas/src.rust/src.rive/rive-runtime/src/`, `rive-runtime/include/`

---

## Table of Contents

1. [Overview](#overview)
2. [Core Architecture](#core-architecture)
3. [Memory Model](#memory-model)
4. [Object System](#object-system)
5. [Component Hierarchy](#component-hierarchy)
6. [Threading and Concurrency](#threading-and-concurrency)
7. [Platform Abstraction Layer](#platform-abstraction-layer)
8. [File Loading and Parsing](#file-loading-and-parsing)

---

## Overview

The Rive C++ runtime is the core engine that powers all Rive applications across platforms. It provides object management, animation playback, and rendering abstraction.

### Key Statistics

| Metric | Value |
|--------|-------|
| Total Source Files | ~200 |
| Lines of Code | ~50,000+ |
| Core Classes | ~150 |
| Supported Platforms | Windows, macOS, Linux, iOS, Android, Web |

### Core Directories

```
rive-runtime/
├── src/                      # Core implementation
│   ├── animation/            # Animation system
│   ├── shapes/               # Vector shapes
│   ├── math/                 # Math utilities
│   ├── core/                 # Core object system
│   ├── input/                # Input handling
│   ├── layout/               # Layout system
│   ├── text/                 # Text rendering
│   ├── bones/                # Bone/skin system
│   ├── constraints/          # IK constraints
│   ├── data_bind/            # Data binding
│   ├── viewmodel/            # ViewModel system
│   └── scripted/             # Scripting support
├── renderer/                 # GPU rendering
├── include/                  # Public headers
├── tests/                    # Unit tests
└── dependencies/             # Third-party libs
```

---

## Core Architecture

### System Overview

```
┌─────────────────────────────────────────────────────────────────┐
│                        Application                               │
└─────────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│                      File Loader                                 │
│  - Binary .riv parsing                                          │
│  - Object instantiation                                         │
│  - Asset loading                                                │
└─────────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│                     Core Object System                           │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────────────────┐ │
│  │    Core     │──│  Component  │──│   TransformComponent    │ │
│  │  (RefCnt)   │  │  (Hierarchy)│  │   (Matrices)            │ │
│  └─────────────┘  └─────────────┘  └─────────────────────────┘ │
└─────────────────────────────────────────────────────────────────┘
                              │
              ┌───────────────┼───────────────┐
              ▼               ▼               ▼
     ┌──────────────┐ ┌──────────────┐ ┌──────────────┐
     │  Artboard    │ │  Animation   │ │ StateMachine │
     │  (Container) │ │  (Keyframes) │ │  (Logic)     │
     └──────────────┘ └──────────────┘ └──────────────┘
                              │
                              ▼
     ┌────────────────────────────────────────────────────────────┐
     │                    Renderer Abstraction                     │
     │  - drawPath(), clipPath(), transform()                      │
     │  - Vulkan │ Metal │ D3D │ OpenGL │ WebGL                   │
     └────────────────────────────────────────────────────────────┘
```

### Main Classes

| Class | File | Purpose |
|-------|------|---------|
| `Core` | `src/core/core.cpp` | Base class with ref counting |
| `Component` | `src/component.cpp` | Hierarchical object base |
| `Artboard` | `src/artboard.cpp` | Root container (~2,000 lines) |
| `File` | `src/file.cpp` | .riv file representation |
| `Renderer` | `src/renderer.cpp` | Renderer interface |

---

## Memory Model

### Reference Counting

Rive uses reference counting (`rcp<>`) instead of garbage collection:

```cpp
// From include/rive/core/ref_counted.hpp

template<typename T>
class rcp {
    T* m_Ptr;
    RefCounted* m_RefCount;

public:
    rcp(T* ptr) : m_Ptr(ptr) {
        if (m_Ptr) {
            m_RefCount = m_Ptr->addRef();
        }
    }

    ~rcp() {
        if (m_Ptr) {
            m_RefCount->release();
        }
    }

    // Copy constructor
    rcp(const rcp& other) : m_Ptr(other.m_Ptr), m_RefCount(other.m_RefCount) {
        if (m_RefCount) {
            m_RefCount->addRef();
        }
    }

    // Move constructor
    rcp(rcp&& other) noexcept : m_Ptr(other.m_Ptr), m_RefCount(other.m_RefCount) {
        other.m_Ptr = nullptr;
        other.m_RefCount = nullptr;
    }

    T* get() const { return m_Ptr; }
    T* operator->() const { return m_Ptr; }
};

// Usage example:
rcp<Artboard> artboard = file->artboard();  // Reference counted
Artboard* raw = artboard.get();             // Get raw pointer (don't delete)
```

### Core Reference Counting

```cpp
// From src/core/core.cpp
class Core {
    int m_RefCount;

public:
    Core() : m_RefCount(1) {}

    virtual ~Core() = default;

    RefCounted* addRef() {
        m_RefCount++;
        return this;
    }

    void release() {
        if (--m_RefCount == 0) {
            delete this;
        }
    }
};
```

### Arena Allocation

For batch allocations, Rive uses arena allocators:

```cpp
// From renderer/src/trivial_block_allocator.hpp

class TrivialBlockAllocator {
    static constexpr size_t kDefaultBlockSize = 16 * 1024;

    struct Block {
        Block* next;
        uint8_t data[kDefaultBlockSize];
    };

    Block* m_CurrentBlock;
    size_t m_Offset;

public:
    void* allocate(size_t size, size_t alignment) {
        // Align offset
        size_t alignedOffset = (m_Offset + alignment - 1) & ~(alignment - 1);

        // Check if current block has space
        if (alignedOffset + size > kDefaultBlockSize) {
            // Allocate new block
            Block* newBlock = (Block*)malloc(sizeof(Block));
            newBlock->next = m_CurrentBlock;
            m_CurrentBlock = newBlock;
            m_Offset = 0;
            alignedOffset = 0;
        }

        void* ptr = m_CurrentBlock->data + alignedOffset;
        m_Offset = alignedOffset + size;
        return ptr;
    }

    void reset() {
        // Free all blocks at once
        Block* block = m_CurrentBlock;
        while (block) {
            Block* next = block->next;
            free(block);
            block = next;
        }
        m_CurrentBlock = nullptr;
        m_Offset = 0;
    }
};
```

### Memory Layout Example

```
Artboard Memory Layout:

Artboard object (heap):
┌────────────────────────────────────┐
│ Core base (ref count)              │  8 bytes
├────────────────────────────────────┤
│ Component base (parent, children)  │ 24 bytes
├────────────────────────────────────┤
│ width, height                      │  8 bytes
├────────────────────────────────────┤
│ m_Objects[] (pointers)             │  8 bytes
├────────────────────────────────────┤
│ m_Animations[] (pointers)          │  8 bytes
├────────────────────────────────────┤
│ m_StateMachines[] (pointers)       │  8 bytes
└────────────────────────────────────┘

Object array (contiguous heap allocation):
┌─────────┬─────────┬─────────┬─────────┐
│ Shape*  │ Bone*   │ Text*   │  ...    │
└─────────┴─────────┴─────────┴─────────┘
```

---

## Object System

### RTTI (Run-Time Type Information)

Rive implements custom RTTI for type-safe casting:

```cpp
// From include/rive/core/rtti.hpp

class Core {
public:
    virtual uint32_t coreType() const = 0;
    virtual bool is(uint32_t type) const { return coreType() == type; }

    template<typename T>
    T* as() {
        return is(T::typeKey) ? static_cast<T*>(this) : nullptr;
    }

    template<typename T>
    const T* as() const {
        return is(T::typeKey) ? static_cast<const T*>(this) : nullptr;
    }

    template<typename T>
    bool is() const {
        return is(T::typeKey);
    }
};

// Type key constants
enum TypeKey : uint32_t {
    ArtboardType = 1,
    ShapeType = 2,
    PathType = 3,
    // ...
};

// Usage:
Core* obj = getObject();

if (obj->is<Shape>()) {
    Shape* shape = obj->as<Shape>();
    // Safe to use shape
}

// C++ style casting:
Shape* shape = rcp_cast<Shape*>(obj);
```

### Object Import System

Objects are deserialized from .riv files using importers:

```cpp
// From src/importers/importer.hpp

class Importer {
public:
    virtual StatusCode import(ImportStack& stack) = 0;
};

class ImportStack {
    std::vector<Importer*> m_Stack;

public:
    void push(Importer* importer) {
        m_Stack.push_back(importer);
    }

    void pop() {
        m_Stack.pop_back();
    }

    template<typename T>
    T* latest() {
        for (auto it = m_Stack.rbegin(); it != m_Stack.rend(); ++it) {
            if (auto* result = dynamic_cast<T>(*it)) {
                return result;
            }
        }
        return nullptr;
    }
};

// Example importer:
class PathImporter : public Importer {
    Path* m_Path;

    StatusCode import(ImportStack& stack) override {
        // Get parent shape importer
        auto* shapeImporter = stack.latest<ShapeImporter>(Shape::typeKey);
        if (shapeImporter == nullptr) {
            return StatusCode::MissingObject;
        }

        shapeImporter->addPath(m_Path);
        return StatusCode::Ok;
    }
};
```

---

## Component Hierarchy

### Component Base Class

```cpp
// From src/component.cpp

class Component : public Core {
    Component* m_Parent;
    std::vector<Component*> m_Children;
    Mat2D m_Transform;
    Mat2D m_WorldTransform;

    // Dirt tracking for efficient updates
    uint32_t m_Dirt;
    uint32_t m_ComponentDirt;

public:
    void setParent(Component* parent) {
        if (m_Parent) {
            m_Parent->removeChild(this);
        }
        m_Parent = parent;
        if (parent) {
            parent->addChild(this);
        }
    }

    const Mat2D& worldTransform() const {
        return m_WorldTransform;
    }

    void updateWorldTransform() {
        if (m_Parent) {
            m_WorldTransform = m_Parent->m_WorldTransform * m_Transform;
        } else {
            m_WorldTransform = m_Transform;
        }

        for (auto* child : m_Children) {
            child->updateWorldTransform();
        }
    }
};
```

### Dirt System (Dirty Flag Pattern)

```cpp
// From include/rive/component_dirt.hpp

enum class ComponentDirt : uint32_t {
    None             = 0,
    Transform        = 1 << 0,
    WorldTransform   = 1 << 1,
    Path             = 1 << 2,
    Vertex           = 1 << 3,
    Paint            = 1 << 4,
    // ...
};

// From src/component.cpp
class Component {
    void addDirt(ComponentDirt dirt) {
        m_ComponentDirt |= static_cast<uint32_t>(dirt);

        // Propagate dirt to parent
        if (m_Parent) {
            m_Parent->onChildDirt(dirt);
        }
    }

    virtual void onDirty(ComponentDirt dirt) {
        // Handle dirt - can defer or process immediately
    }

    virtual void update(ComponentDirt dirt) {
        // Update based on dirt type
        if (hasDirt(dirt, ComponentDirt::Transform)) {
            updateTransform();
        }
        if (hasDirt(dirt, ComponentDirt::Path)) {
            updatePath();
        }
    }

    bool hasDirt(ComponentDirt check, ComponentDirt flag) const {
        return (static_cast<uint32_t>(check) & static_cast<uint32_t>(flag)) != 0;
    }
};

// Usage example:
void Path::markPathDirty() {
    addDirt(ComponentDirt::Path);

    // Notify shape that path changed
    if (m_Shape) {
        m_Shape->pathChanged();
    }
}
```

### Update Cycle

```cpp
// From src/artboard.cpp

void Artboard::advance(float elapsedSeconds) {
    RIVE_PROF_SCOPE()

    // 1. Update animations
    for (auto* animation : m_AnimationInstances) {
        if (animation->isPlaying()) {
            animation->update(elapsedSeconds);
            animation->apply(this);
        }
    }

    // 2. Update state machines
    for (auto* sm : m_StateMachineInstances) {
        sm->update(elapsedSeconds);
    }

    // 3. Update hierarchy (transforms, paths, etc.)
    updateHierarchy();
}

void Artboard::updateHierarchy() {
    // Collect dirty components
    std::vector<Component*> dirtyComponents;
    collectDirtyComponents(this, dirtyComponents);

    // Sort by dependency (children before parents)
    sortComponentsByDependency(dirtyComponents);

    // Update each component
    for (auto* component : dirtyComponents) {
        component->update(component->m_ComponentDirt);
        component->m_ComponentDirt = 0;  // Clear dirt
    }

    // Update world transforms
    updateWorldTransforms();
}
```

---

## Threading and Concurrency

### Thread Safety Model

Rive's core is **not thread-safe** by design. The model is:

```
Thread Model:

Main Thread (UI/Render):
├── Load files
├── Update animations
└── Render artboards

Worker Thread (Optional):
├── File decoding (parallel)
├── Asset loading
└── Texture decoding

Communication:
- Load on worker → Transfer to main
- Never share Artboard between threads
```

### File Loading on Worker Thread

```cpp
// Worker thread pattern

// 1. Load file on worker thread
std::unique_ptr<File> loadFileOnWorker(const std::string& path) {
    return std::unique_ptr<File>(File::load(path));
}

// 2. Transfer to main thread
void onFileLoaded(std::unique_ptr<File> file) {
    // Take ownership on main thread
    m_File = std::move(file);

    // Now safe to use from main thread
    auto* artboard = m_File->artboard();
}
```

### Render Thread Separation

```cpp
// Game engine integration pattern

class RiveComponent {
    // Game thread
    void update(float deltaTime) {
        // Update animation state
        m_StateMachine->setInput("isRunning", player.isRunning);
        m_StateMachine->advance(deltaTime);
    }

    // Render thread
    void render(Renderer* renderer) {
        // Read-only access to animation state
        m_Artboard->draw(renderer);
    }
};
```

---

## Platform Abstraction Layer

### File I/O Abstraction

```cpp
// From include/rive/file_reader.hpp

class FileReader {
public:
    virtual bool read(void* destination, size_t length) = 0;
    virtual bool seek(int offset) = 0;
    virtual size_t position() const = 0;
    virtual size_t size() const = 0;
};

// Platform implementations:

class PosixFileReader : public FileReader {
    FILE* m_File;
    size_t m_Size;

public:
    bool read(void* dst, size_t len) override {
        return fread(dst, 1, len, m_File) == len;
    }

    bool seek(int offset) override {
        return fseek(m_File, offset, SEEK_SET) == 0;
    }
};

class WASMFileReader : public FileReader {
    const uint8_t* m_Data;
    size_t m_Size;
    size_t m_Pos;

public:
    bool read(void* dst, size_t len) override {
        if (m_Pos + len > m_Size) return false;
        memcpy(dst, m_Data + m_Pos, len);
        m_Pos += len;
        return true;
    }
};
```

### Thread Abstraction

```cpp
// From include/rive/threading.hpp

class Mutex {
public:
    virtual void lock() = 0;
    virtual void unlock() = 0;
};

class Thread {
public:
    virtual void start() = 0;
    virtual void join() = 0;
};

// Platform-specific implementations
#ifdef _WIN32
    class WindowsMutex : public Mutex { /* ... */ };
#elif __APPLE__
    class PosixMutex : public Mutex { /* ... */ };
#else
    class PthreadMutex : public Mutex { /* ... */ };
#endif
```

---

## File Loading and Parsing

### .riv File Structure

```
.riv Binary Format:

┌─────────────────────────────────────┐
│  Header                             │
│  - Magic: "RIVE"                    │
│  - Version                          │
│  - Endianness marker                │
├─────────────────────────────────────┤
│  Object Table                       │
│  - Object count                     │
│  - Object type IDs                  │
│  - Property data                    │
├─────────────────────────────────────┤
│  String Table                       │
│  - All string data                  │
├─────────────────────────────────────┤
│  Asset Table                        │
│  - Images                           │
│  - Fonts                            │
│  - Audio                            │
└─────────────────────────────────────┘
```

### File Loader Implementation

```cpp
// From src/file.cpp

class File {
    std::unique_ptr<FileReader> m_Reader;
    std::vector<Core*> m_Objects;
    std::vector<Artboard*> m_Artboards;

    static File* load(const std::string& path) {
        auto reader = openFile(path);
        if (!reader) return nullptr;

        File* file = new File();
        file->m_Reader = std::move(reader);

        // Read header
        if (!file->readHeader()) {
            delete file;
            return nullptr;
        }

        // Read objects
        if (!file->readObjects()) {
            delete file;
            return nullptr;
        }

        // Initialize objects
        if (!file->initializeObjects()) {
            delete file;
            return nullptr;
        }

        return file;
    }

    bool readHeader() {
        char magic[4];
        m_Reader->read(magic, 4);

        if (memcmp(magic, "RIVE", 4) != 0) {
            return false;  // Invalid file
        }

        uint32_t version;
        m_Reader->read(&version, sizeof(version));

        if (version > kMaxSupportedVersion) {
            return false;  // Unsupported version
        }

        return true;
    }

    bool readObjects() {
        uint32_t objectCount;
        m_Reader->read(&objectCount, sizeof(objectCount));

        m_Objects.resize(objectCount);

        ImportStack importStack;

        for (uint32_t i = 0; i < objectCount; i++) {
            uint32_t typeId;
            m_Reader->read(&typeId, sizeof(typeId));

            Core* obj = factory().createObject(typeId);
            if (!obj) {
                // Unknown type - skip or error
                continue;
            }

            m_Objects[i] = obj;

            // Read properties
            Importer* importer = obj->as<Importer>();
            if (importer) {
                importStack.push(importer);

                uint32_t propertyCount;
                m_Reader->read(&propertyCount, sizeof(propertyCount));

                for (uint32_t j = 0; j < propertyCount; j++) {
                    uint32_t propertyKey;
                    m_Reader->read(&propertyKey, sizeof(propertyKey));

                    // Read property value based on type
                    readProperty(importer, propertyKey);
                }

                importStack.pop();
            }
        }

        return true;
    }
};
```

### Property Reading

```cpp
// From src/generated/property_reader.cpp

void File::readProperty(Importer* importer, uint32_t propertyKey) {
    uint8_t propertyType;
    m_Reader->read(&propertyType, sizeof(propertyType));

    switch (propertyType) {
        case PropertyType::Bool: {
            bool value;
            m_Reader->read(&value, sizeof(value));
            importer->importBool(propertyKey, value);
            break;
        }
        case PropertyType::Int: {
            int32_t value;
            m_Reader->read(&value, sizeof(value));
            importer->importInt(propertyKey, value);
            break;
        }
        case PropertyType::Float: {
            float value;
            m_Reader->read(&value, sizeof(value));
            importer->importFloat(propertyKey, value);
            break;
        }
        case PropertyType::String: {
            uint32_t stringIndex;
            m_Reader->read(&stringIndex, sizeof(stringIndex));
            const char* str = getString(stringIndex);
            importer->importString(propertyKey, str);
            break;
        }
        case PropertyType::Ref: {
            uint32_t objectIndex;
            m_Reader->read(&objectIndex, sizeof(objectIndex));
            Core* ref = m_Objects[objectIndex];
            importer->importRef(propertyKey, ref);
            break;
        }
        case PropertyType::List: {
            uint32_t count;
            m_Reader->read(&count, sizeof(count));
            importer->importList(propertyKey, count);
            for (uint32_t i = 0; i < count; i++) {
                readProperty(importer, propertyKey);
            }
            break;
        }
    }
}
```

---

## Summary

The Rive C++ core provides:

1. **Object System**: Reference-counted, RTTI-enabled base classes
2. **Component Hierarchy**: Parent-child relationships with transforms
3. **Dirt System**: Efficient dirty flag propagation for updates
4. **Memory Management**: Reference counting with arena allocators
5. **Platform Abstraction**: File I/O and threading across platforms
6. **File Loading**: Binary deserialization with validation

For related topics:
- `rendering-engine-deep-dive.md` - GPU rendering
- `animation-system-deep-dive.md` - Animation playback
- `rust-revision.md` - Rust implementation approach
