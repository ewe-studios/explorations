---
name: Makepad Showcase Applications
description: Demo applications (Taobao, WeChat, Wonderous clones) and tutorial series (image_viewer) built with Makepad to demonstrate UI capabilities
type: sub-project
source: /home/darkvoid/Boxxed/@formulas/src.rust/src.Makerpad/
repository: N/A - various local projects
explored_at: 2026-03-23
language: Rust
---

# Makepad Showcase Applications

## Overview

The Makepad ecosystem includes several showcase applications that demonstrate the toolkit's ability to replicate complex, production-grade UIs. These include clones of popular apps (Taobao, WeChat, Wonderous) and a comprehensive step-by-step tutorial series (image_viewer). All are built entirely with Makepad widgets and serve as both marketing demos and learning resources.

---

## 1. makepad_taobao

**Source:** `/home/darkvoid/Boxxed/@formulas/src.rust/src.Makerpad/makepad_taobao/`

A Makepad recreation of Taobao (Alibaba's e-commerce app), demonstrating complex scrolling lists, product cards, image-heavy layouts, and tab navigation.

### Structure

```
makepad_taobao/
├── Cargo.toml              # depends on makepad-widgets (git, branch: rik)
├── android.sh              # Android build script
├── resources/              # Images, icons, assets
└── src/                    # Application source
```

### Key Details
- Depends on makepad-widgets via git (branch `rik`)
- Includes Android build script for mobile deployment
- Uses `profile.small` for optimized WASM builds (opt-level z, LTO, strip)
- Showcases: complex scroll views, image loading, product grid layouts, search bar

---

## 2. makepad_wechat

**Source:** `/home/darkvoid/Boxxed/@formulas/src.rust/src.Makerpad/makepad_wechat/`

A Makepad recreation of WeChat (Tencent's messaging app), demonstrating chat interfaces, contact lists, and tab-based navigation.

### Structure

```
makepad_wechat/
├── Cargo.toml              # depends on makepad-widgets (git, branch: rik)
├── android.sh              # Android build script
├── resources/              # Images, icons, assets
└── src/                    # Application source
```

### Key Details
- Same build configuration as makepad_taobao
- Demonstrates: message bubbles, conversation lists, avatar rendering, tab navigation
- Shows Makepad's ability to handle messaging UI patterns

---

## 3. makepad_wonderous

**Source:** `/home/darkvoid/Boxxed/@formulas/src.rust/src.Makerpad/makepad_wonderous/`

A Makepad port of the Flutter "Wonderous" demo app, showcasing animations, parallax effects, and rich visual storytelling. This is particularly significant as a cross-framework comparison piece.

### Structure

```
makepad_wonderous/
├── Cargo.toml              # depends on makepad-widgets (git, branch: rik)
├── android.sh              # Android build script
├── resources/              # Images, icons, assets
└── src/                    # Application source
```

### Key Details
- Includes `osiris` packaging metadata for macOS distribution
- Demonstrates: animations, parallax scrolling, image transitions, custom gestures
- Direct comparison target with Flutter's showcase demo
- Uses the same optimized build profiles

---

## 4. image_viewer (Tutorial Series)

**Source:** `/home/darkvoid/Boxxed/@formulas/src.rust/src.Makerpad/image_viewer/`

A step-by-step tutorial building an image viewer application in 17 incremental steps, teaching Makepad fundamentals from scratch.

### Structure

```
image_viewer/
├── Cargo.toml              # Workspace with 17 step members
├── images/                 # Sample images for the tutorial
├── step_1/                 # Hello World / empty window
│   ├── Cargo.toml
│   └── src/
├── step_2/                 # Adding a widget
├── step_3/                 # Layout basics
├── step_4/                 # ...
├── ...
└── step_17/                # Complete image viewer
```

### Key Details
- Workspace with `resolver = "3"` (Rust edition 2024)
- Each step is a standalone crate that builds independently
- Progressive introduction of Makepad concepts: windows, widgets, layout, events, images, scrolling
- Serves as the primary learning resource for new Makepad developers

---

## 5. ai_snake

**Source:** `/home/darkvoid/Boxxed/@formulas/src.rust/src.Makerpad/ai_snake/`

An AI-generated snake game built with Makepad, demonstrating how AI coding assistants can produce Makepad applications.

### Structure

```
ai_snake/
├── Cargo.toml              # makepad-experiment-ai-snake v0.6.0
├── ai/                     # AI-related files (prompts/context)
└── src/
    ├── main.rs             # Entry point
    ├── lib.rs              # Library root
    └── app.rs              # Game application logic
```

---

## 6. html_experiment

**Source:** `/home/darkvoid/Boxxed/@formulas/src.rust/src.Makerpad/html_experiment/`

A test application for HTML/image rendering in Makepad, specifically testing PNG loading edge cases.

### Structure

```
html_experiment/
├── Cargo.toml              # depends on makepad-widgets (local path)
├── resources/              # Test images (PNGs with loading issues)
└── src/
    ├── main.rs
    ├── lib.rs
    └── app.rs
```

---

## Common Patterns

All showcase apps share:
- **makepad-widgets** as sole dependency (via git or local path)
- **Android build scripts** (`android.sh`) for mobile deployment
- **Optimized WASM profile**: `opt-level = 'z'`, `lto = true`, `codegen-units = 1`, `panic = 'abort'`, `strip = true`
- **`src/app.rs`** as the main application module with `live_design!` blocks

## Key Insights

- The Taobao/WeChat/Wonderous demos serve as "proof by implementation" that Makepad can handle complex production UIs
- All showcase apps target the `rik` branch of Makepad, suggesting they track a development branch with latest features
- The image_viewer tutorial is the most structured learning resource in the ecosystem (17 steps)
- The ai_snake project is meta-interesting: it demonstrates that Makepad apps can be generated by AI
- The WASM optimization profile is standardized across all showcase apps, showing the importance of web deployment
