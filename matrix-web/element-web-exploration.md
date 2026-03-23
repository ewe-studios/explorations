---
location: /home/darkvoid/Boxxed/@formulas/src.rust/src.matrix-web/element-web
repository: https://github.com/element-hq/element-web
explored_at: 2026-03-23
language: TypeScript, React
---

# Sub-Project Exploration: Element Web

## Overview

Element Web is the flagship Matrix web client, built with TypeScript and React. It provides a full-featured communication interface including messaging, voice/video calls, end-to-end encryption, room management, spaces, and threads. Version 1.11.95, it serves as both a standalone web application and the embedded content for Element Desktop (Electron).

The application uses the matrix-js-sdk for Matrix protocol communication and matrix-react-sdk for reusable React UI components.

## Architecture

### High-Level Diagram

```mermaid
graph TD
    User[User Browser] --> EW[Element Web<br/>React SPA]
    EW --> JSSDK[matrix-js-sdk]
    EW --> ReactSDK[matrix-react-sdk]
    EW --> CompoundWeb[compound-web<br/>Design System]
    EW --> WASM[wysiwyg-wasm<br/>Rich Text Editor]
    JSSDK --> HS[Homeserver API]
    JSSDK --> Crypto[Crypto Module<br/>Olm/Megolm]
    EW --> IndexedDB[(IndexedDB<br/>Local Storage)]

    subgraph "Element Web Source"
        Components[src/components/]
        Actions[src/actions/]
        Stores[src/stores/]
        Contexts[src/contexts/]
        Audio[src/audio/]
        Autocomplete[src/autocomplete/]
    end
```

### Source Structure

```
element-web/
├── src/
│   ├── vector/                 # App entry point and initialization
│   ├── components/             # React components
│   ├── actions/                # Redux-style actions
│   ├── contexts/               # React contexts
│   ├── audio/                  # Audio playback/recording
│   ├── autocomplete/           # Message autocomplete
│   ├── accessibility/          # A11y utilities
│   ├── async-components/       # Code-split components
│   ├── customisations/         # White-label customization hooks
│   ├── stores/                 # State management
│   ├── i18n/                   # Internationalization
│   └── @types/                 # TypeScript type definitions
├── playwright/                 # E2E test suite
├── docker/                     # Docker deployment
├── debian/                     # Debian packaging
├── docs/                       # Developer documentation
├── config.sample.json          # Runtime configuration sample
└── package.json
```

## Key Components

### Entry Point (`src/vector/`)
- Initializes the Matrix client, loads configuration, sets up routing, renders the React app tree.

### Components Layer
- Room views, message timeline, room list, spaces, member list, settings panels, dialogs.
- Uses compound-web design system components as building blocks.

### State Management
- Combination of React contexts, custom stores, and matrix-js-sdk's built-in event emitters.
- IndexedDB for persistent local storage of sync state and crypto keys.

### Rich Text Composition
- Integrates the matrix-rich-text-editor WASM module for WYSIWYG message composition.

### E2E Testing
- Playwright test suite for end-to-end testing of user flows.

## Key Insights

- The codebase is mature (v1.11.95) with extensive feature coverage
- Configuration is runtime-loaded (config.json), enabling deployment-specific customization without rebuilds
- Supports white-labeling through a customization API
- Docker deployment and Debian packaging are first-class concerns
- The project references both the old `vector-im` and new `element-hq` GitHub organizations (ongoing migration)
- Playwright E2E tests provide comprehensive regression coverage
- Book.toml indicates mdBook documentation is available
