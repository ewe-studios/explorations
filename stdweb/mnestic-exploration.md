---
location: /home/darkvoid/Boxxed/@formulas/src.rust/src.wasm/src.stdweb/mnestic
repository: https://github.com/inanna-malick/mnestic
explored_at: 2026-03-23
language: Rust, JavaScript
---

# Sub-Project Exploration: mnestic (mnemnos)

## Overview

**mnestic** (internally named **mnemnos**) is a markdown-based page management application built as a full-stack Rust project. It uses Yew for the frontend (compiled to WASM), Cloudflare Workers for the backend, and Cloudflare R2 for persistent storage. Users can create, edit, and remove markdown pages through a web interface, with the pages rendered to HTML via the `comrak` markdown library.

The project is a workspace of three crates that share common types, demonstrating a modern Rust WASM stack that has moved beyond stdweb to use `wasm-bindgen` and Yew.

## Architecture

```mermaid
graph TD
    subgraph "Frontend (mnemnos-wasm)"
        Browser[Browser] --> YewApp[Yew App Component]
        YewApp --> LoadAndRun[LoadAndRun Component]
        LoadAndRun --> HeaderInput[HeaderInput Component]
        LoadAndRun --> PageView[PageView Component]
        LoadAndRun --> |"GET /api/state"| APIClient[gloo-net HTTP]
        LoadAndRun --> |"POST /api/state"| APIClient
        LoadAndRun --> LocalStorage[gloo LocalStorage]
        LoadAndRun --> UseReducer[Yew useReducer - AppState]
    end

    subgraph "Backend (mnemnos-worker)"
        Wrangler[Cloudflare Workers] --> AxumRouter[Axum Router]
        AxumRouter --> GetState[GET /api/state]
        AxumRouter --> SetState[POST /api/state]
        AxumRouter --> Assets[GET /assets/*path]
        GetState --> R2Bucket[Cloudflare R2 Bucket]
        SetState --> R2Bucket
        Assets --> R2Bucket
        Assets --> Render[AppState::render - Markdown to HTML]
    end

    subgraph "Shared (mnemnos-types)"
        AppState[AppState struct]
        Page[Page struct]
        PageName[PageName newtype]
        Action[Action enum - Reducible]
        RenderFn[render() - comrak markdown_to_html]
    end

    APIClient -.-> AxumRouter
    LoadAndRun --> AppState
    GetState --> AppState
    SetState --> AppState
```

## Directory Structure

```
mnestic/
├── Cargo.toml                     # Workspace: mnemnos-types, mnemnos-wasm, mnemnos-worker
├── Cargo.lock
├── wrangler.toml                  # Cloudflare Workers configuration
├── wrangler-build.sh              # Build script for wrangler
├── mnemnos-types/                 # Shared types crate
│   ├── Cargo.toml
│   └── src/
│       └── lib.rs                 # AppState, Page, PageName, Action, Reducible impl
├── mnemnos-wasm/                  # Frontend WASM crate (Yew)
│   ├── Cargo.toml
│   ├── src/
│   │   ├── main.rs                # Yew app entry, App + LoadAndRun components
│   │   ├── components.rs          # Module declarations
│   │   ├── hooks.rs               # Custom Yew hooks
│   │   └── components/
│   │       ├── header_input.rs    # Page creation input component
│   │       └── page.rs            # Individual page view component
│   └── README.md
├── mnemnos-worker/                # Backend Cloudflare Worker crate
│   ├── Cargo.toml
│   ├── src/
│   │   └── lib.rs                 # Axum router, R2 storage handlers
│   └── dist/
│       ├── shim.js                # Worker shim
│       └── README.md
└── .gitignore
```

## Key Components

### mnemnos-types (Shared Types)

Core domain types shared between frontend and backend:

- **`PageName(String)`** - Newtype wrapper for page identifiers
- **`Page { markdown: String }`** - A single page with markdown content
- **`AppState { pages: HashMap<PageName, Page> }`** - Full application state
- **`Action`** - Reducer actions: `AddPage`, `RemovePage`, `EditTemplate`
- **`Reducible` impl for AppState** - Yew reducer pattern for state management
- **`render()`** - Converts markdown to full HTML page using `comrak`

### mnemnos-wasm (Frontend)

A Yew CSR (Client-Side Rendered) application:

- **`App`** - Root component with `Suspense` for async loading
- **`LoadAndRun`** - Main component that:
  - Fetches initial state from `/api/state` using `use_future`
  - Manages state via `use_reducer` with `AppState`
  - Persists to `LocalStorage` via `use_effect_with`
  - Provides callbacks for add/remove/edit actions
  - Renders page list with `PageView` components
  - Has a "save state to R2" button that POSTs state to the API
- **`HeaderInput`** - Component for creating new pages
- **`PageView`** - Component for viewing/editing individual pages

### mnemnos-worker (Backend)

A Cloudflare Worker using Axum router:

- **Routes:**
  - `POST /api/state` - Save entire AppState to R2
  - `GET /api/state` - Load AppState from R2
  - `GET /assets/*path` - Render a specific page's markdown as HTML
- **Storage:** Cloudflare R2 bucket (`mnemnos-state`) with a single key `mnestic`
- **Error handling:** `MnemnosError` enum with `Internal(anyhow::Error)` and `AssetNotFound` variants
- Uses `worker::send::SendFuture` for R2 operations

### Wrangler Configuration

```toml
name = "mnemnos-worker"
main = "mnemnos-worker/build/worker/shim.mjs"
[assets]
directory = "mnemnos-wasm/dist"
[[r2_buckets]]
binding = 'STORAGE'
bucket_name = 'mnemnos-state'
```

## Dependencies

### mnemnos-wasm
| Dependency | Version | Purpose |
|------------|---------|---------|
| yew | 0.21 | Frontend framework (CSR mode) |
| gloo | 0.11 | Browser API bindings |
| gloo-net | 0.6 | HTTP client with JSON support |
| web-sys | 0.3 | Raw Web API bindings |
| serde/serde_json | 1.0 | Serialization |

### mnemnos-worker
| Dependency | Version | Purpose |
|------------|---------|---------|
| worker | 0.4.2 | Cloudflare Workers runtime (HTTP + Axum features) |
| axum | 0.7 | HTTP routing framework |
| tower-service | 0.3.2 | Service trait |
| anyhow | 1.0 | Error handling |
| futures | 0.3 | Async stream handling |
| console_error_panic_hook | 0.1.1 | Better WASM panic messages |

### mnemnos-types
| Dependency | Purpose |
|------------|---------|
| comrak | CommonMark markdown to HTML conversion |
| serde | Serialization for API transport |
| yew | Reducible trait for state management |
| anyhow | Error handling for render function |

## Key Insights

- This project has moved far from stdweb: it uses wasm-bindgen, Yew, gloo, and web-sys -- the modern Rust WASM stack
- The shared types crate pattern (mnemnos-types) is a best practice for full-stack Rust, ensuring type safety across the WASM/Worker boundary
- The entire application state is stored as a single JSON blob in R2, which is simple but limits scalability
- The `Reducible` trait from Yew enables Redux-like state management on the frontend
- The Cloudflare Workers + R2 combination provides a serverless deployment model
- The project includes `reqwasm` and `reqwest_wasi` in worker dependencies, suggesting some exploration of alternative HTTP clients
- The "save state to R2" button implies manual persistence rather than automatic sync, a deliberate UX choice
