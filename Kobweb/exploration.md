---
location: /home/darkvoid/Boxxed/@formulas/src.Kobweb
repository: https://github.com/varabyte/kobweb
explored_at: 2026-03-20T00:00:00Z
language: Kotlin
---

# Project Exploration: Kobweb - Kotlin Web Framework

## Overview

Kobweb is an opinionated Kotlin web development framework built on top of [Compose HTML](https://github.com/JetBrains/compose-multiplatform#compose-html) and inspired by [Next.js](https://nextjs.org) and [Chakra UI](https://chakra-ui.com). It enables developers to create websites and web applications using pure Kotlin, leveraging the Compose runtime for declarative UI construction.

The framework provides:
- **File-based routing** - Pages are automatically generated from Kotlin files in a `pages/` directory
- **Server-side rendering (SSR) export** - Static site generation with hydration support
- **Full-stack capabilities** - Built-in API route definitions and websocket streams
- **Silk UI library** - A Chakra UI-inspired component library
- **Markdown support** - Convert Markdown files to pages with Kotlin component interpolation
- **Live reloading** - Development server with hot module replacement
- **Web Workers** - First-class support for background threads

## Directory Structure

```
kobweb/                          # Main Kobweb framework
├── backend/                     # Server-side components
│   ├── kobweb-api/              # API route definitions (@Api annotation)
│   ├── server/                  # Ktor-based web server
│   └── server-plugin/           # Server plugin API
├── common/                      # Shared code between frontend/backend
│   ├── kobweb-common/           # Common types and utilities
│   └── client-server-internal/  # Internal client-server communication
├── frontend/                    # Client-side components
│   ├── browser-ext/             # Browser DOM extensions
│   ├── compose-html-ext/        # Compose HTML extensions (CSS APIs)
│   ├── kobweb-compose/          # Compose integration layer
│   ├── kobweb-core/             # Core framework (Page, Router, App)
│   ├── kobweb-silk/             # Silk UI framework integration
│   ├── kobweb-worker/           # Web Worker support
│   ├── kobweb-worker-interface/ # Worker communication interface
│   ├── kobwebx-markdown/        # Markdown processing
│   ├── silk-foundation/         # Silk CSS foundation
│   ├── silk-icons-fa/           # Font Awesome icons
│   ├── silk-icons-mdi/          # Material Design icons
│   ├── silk-widgets/           # Silk UI widgets
│   └── silk-widgets-kobweb/    # Silk widgets for Kobweb
├── tools/                       # Build tooling
│   ├── gradle-plugins/          # Gradle plugins (core, library, application)
│   ├── ksp/                     # Kotlin Symbol Processing
│   └── processor-common/        # Common processor utilities
├── playground/                  # Example/test site
└── templates/                   # Project templates (submodule)

kobweb-cli/                      # CLI binary (thin wrapper around Gradle)
kobweb-site/                     # Documentation site

kotter/                          # Terminal UI library (separate but related)
├── kotter/                      # Core Kotter library
├── kotterx/                     # Extensions
└── examples/                    # Terminal UI examples

truthish/                        # Multiplatform testing library
```

## Architecture

### High-Level Diagram

```mermaid
graph TB
    subgraph "Developer Experience"
        CLI[Kobweb CLI]
        Gradle[Gradle Plugins]
        KSP[KSP Processors]
    end

    subgraph "Frontend (Kotlin/JS)"
        Core[kobweb-core<br/>Page @ Composable]
        Router[Router<br/>RouteTree]
        ComposeExt[compose-html-ext<br/>CSS APIs]
        BrowserExt[browser-ext<br/>DOM APIs]
        Silk[Silk UI Library]
        Worker[Web Workers]
    end

    subgraph "Backend (Kotlin/JVM)"
        Server[Ktor Server]
        API[kobweb-api<br/>@Api routes]
        Streams[API Streams<br/>Websockets]
        Plugins[Server Plugins]
    end

    subgraph "Build Process"
        Compile[Kotlin Compile<br/>to JS + JVM]
        Process[Page/API Processing]
        Generate[Generate Routes<br/>HTML Export]
    end

    subgraph "Runtime"
        Browser[Browser<br/>Wasm/JS]
        SSR[SSR Export<br/>Static HTML]
        FullStack[Full-Stack<br/>Live Server]
    end

    CLI --> Gradle
    Gradle --> KSP
    KSP --> Compile
    Compile --> Process
    Process --> Generate

    Core --> Router
    ComposeExt --> Core
    BrowserExt --> Core
    Silk --> Core

    API --> Server
    Streams --> Server
    Plugins --> Server

    Generate --> SSR
    Generate --> FullStack
    FullStack --> Browser
    SSR --> Browser
```

## Kobweb Framework

### Page Model

Pages are defined using the `@Page` annotation on Composable functions:

```kotlin
@Page
@Composable
fun HomePage() {
    Column(
        Modifier.fillMaxWidth().whiteSpace(WhiteSpace.PreWrap).textAlign(TextAlign.Center),
        horizontalAlignment = Alignment.CenterHorizontally
    ) {
        var colorMode by ColorMode.currentState
        Button(
            onClick = { colorMode = colorMode.opposite }
        ) {
            Text("Toggle Theme")
        }
        H1 { Text("Welcome to Kobweb!") }
    }
}
```

**Route Generation:**
- File path determines route: `pages/account/Profile.kt` → `/account/profile`
- Filename converted to kebab-case: `WelcomeIntro.kt` → `/welcome-intro`
- `Index.kt` is special: `pages/blog/Index.kt` → `/blog`
- Dynamic routes: `pages/users/[userId]/Profile.kt` → `/users/{userId}`

**Route Override:**
```kotlin
@Page("custom-slug")           // Override slug
@Page("index")                 // Make default for path
@Page("/absolute/path/")       // Absolute path override
@Page("{}")                    // Dynamic segment from filename
@Page("{slug}")                // Named dynamic segment
```

### Component Model

Kobweb uses the Compose HTML component model:

1. **Composable Functions** - UI building blocks using `@Composable`
2. **Modifiers** - Chainable style/configuration objects
3. **State** - `mutableStateOf`, `remember`, etc. from Compose Runtime
4. **Layout** - `Row`, `Column`, `Box` primitives (Flexbox under the hood)

```kotlin
@Composable
fun MyWidget(data: String) {
    var count by remember { mutableStateOf(0) }

    Row(Modifier.padding(16.px)) {
        Text("Count: $count")
        Button(onClick = { count++ }) {
            Text("Increment")
        }
    }
}
```

### Layout System

Kobweb supports nested layouts similar to Next.js:

```kotlin
@Layout
@Composable
fun DefaultLayout(ctx: LayoutContext, content: @Composable () -> Unit) {
    Column {
        Header()
        Box(Modifier.weight(1f)) { content() }
        Footer()
    }
}
```

Layouts wrap pages and can be nested. Each page inherits ancestor layouts.

### Routing Architecture

The `Router` class manages client-side navigation:

```kotlin
class Router {
    fun navigateTo(path: String, updateHistoryMode: UpdateHistoryMode = PUSH)
    fun tryRoutingTo(path: String): Boolean
    fun register(route: String, pageMethod: PageMethod)
    fun addRouteInterceptor(interceptor: RouteInterceptorScope.() -> Unit)
}
```

**RouteTree** - Internal trie structure for efficient route matching:
- Static routes stored as literal path segments
- Dynamic routes stored with parameter names
- Catch-all routes supported with `{...}`

### Data Flow

```
┌─────────────────────────────────────────────────────────────┐
│  @Page Composable                                           │
│    └── Uses PageContext                                     │
│         ├── route.params (dynamic segments)                 │
│         ├── route.queryParams                               │
│         └── route.fragment                                  │
└─────────────────────────────────────────────────────────────┘
                          │
                          ▼
┌─────────────────────────────────────────────────────────────┐
│  InitRoute Methods (@InitRoute)                             │
│    - Called when page first loads                           │
│    - Can fetch data, set up PageContext.data                │
└─────────────────────────────────────────────────────────────┘
```

## Kotlin to Web Compilation

### Build Pipeline

1. **Kotlin Multiplatform Project** - Kobweb uses Kotlin/JS target
2. **KSP Processors** - Process `@Page`, `@Api`, `@Layout` annotations
3. **Code Generation** - Generate route registration code
4. **Kotlin/JS Compilation** - Compile to JavaScript
5. **Webpack Bundling** - Bundle JS with dependencies
6. **HTML Generation** (export) - Pre-render pages to static HTML

### Gradle Plugin Configuration

```kotlin
plugins {
    id("com.varabyte.kobweb.application") version "..."
}

kobweb {
    pagesPackage.set(".pages")
    apiPackage.set(".api")
    publicPath.set("public")
}
```

### KSP Processing

The KSP processor scans for annotations and generates:

1. **Page entries** - Maps file paths to composable functions
2. **API entries** - Maps routes to API handlers
3. **Route registration** - Code to register all routes at runtime

Generated code example:
```kotlin
// Generated by KSP
router.register("/users/{user}", layoutId = "default") { ctx ->
    UserProfilePage(ctx)
}
```

### Output Artifacts

```
build/dist/js/
├── site.js              # Bundled application
├── site.js.map          # Source maps
├── index.html           # Entry HTML
├── static/              # Public resources
└── kobweb/              # Framework internals
```

## SSR vs CSR Support

### Static Export (SSR-like)

Kobweb supports **static site export** which pre-renders pages:

```bash
kobweb export -PkobwebExportLayout=STATIC
```

**How it works:**
1. Start development server
2. Crawl all routes
3. For each route, render to HTML using headless browser
4. Save static `.html` files
5. Export includes hydrated state for client-side interactivity

**Limitations:**
- Dynamic routes must be pre-configured with known values
- Server APIs not available in static mode
- Uses `PageContext.isExporting` to conditionally skip client-only code

### Full-Stack Mode

```bash
kobweb run  # Development
kobweb export -PkobwebExportLayout=FULLSTACK  # Production
```

**Features:**
- Ktor server runs backend
- API routes respond to HTTP requests
- API Streams for websocket connections
- Dynamic data fetching at runtime

### Choosing Layout

| Feature | Static | Full-Stack |
|---------|--------|------------|
| Hosting | Any static host | Requires JVM server |
| APIs    | Not available | Full support |
| Dynamic Routes | Pre-defined only | Runtime |
| SEO | Excellent (pre-rendered) | Good (SSR) |
| Cost | Low (CDN) | Higher (server) |

## Backend (Server API)

### API Routes

```kotlin
@Api
suspend fun fetchUser(ctx: ApiContext) {
    val userId = ctx.pathParams["userId"]
    val user = database.getUser(userId)
    ctx.respond(user)
}
```

**Features:**
- HTTP methods: `@Api(method = HttpMethod.POST)`
- Path params, query params, body parsing
- Interceptors for auth/logging
- Response serialization (JSON by default)

### API Streams (Websockets)

```kotlin
@Api
val chatStream = object : ApiStream {
    override fun onOpen(ctx: ApiStreamContext) { ... }
    override fun onMessage(ctx: ApiStreamContext, msg: String) { ... }
    override fun onClose(ctx: ApiStreamContext) { ... }
}
```

**Client usage:**
```kotlin
val stream = ApiStream.connect("/api/chat")
stream.send("Hello")
stream.onMessage { msg -> println(msg) }
```

### Server Plugins

```kotlin
class KobwebServerPlugin : com.varabyte.kobweb.server.plugin.KobwebServerPlugin {
    override fun Application.install() {
        // Install Ktor features
        install(ContentNegotiation) { ... }
        install(CORS) { ... }
    }
}
```

## CLI Tooling

The `kobweb` CLI is a thin wrapper around Gradle:

```
kobweb create app          # Create new project
kobweb run                 # Start dev server
kobweb stop                # Stop server
kobweb export              # Export static site
kobweb list                # List templates
```

**Under the hood:**
- `kobweb run` → `gradle kobwebStart -t` (continuous mode)
- `kobweb export` → `gradle kobwebExport`
- Uses Gradle for all build operations

## Kotter Terminal UI

Kotter is a **separate but related** library for terminal UI applications.

### Architecture

```
session {
    var count by liveVarOf(0)

    section {
        textLine("Count: $count")
    }.run {
        addTimer(1.seconds) { count++ }
    }
}
```

**Key Concepts:**
- `session` - Top-level application scope
- `section` - Render block (can rerender multiple times)
- `run` - Background logic block
- `liveVarOf` - Reactive state (triggers rerender on change)
- `input()` - Interactive text input
- `renderAnimOf` / `textAnimOf` - Animations

**State Management:**
```kotlin
// LiveVar - Auto rerenders on change
var value by liveVarOf(initial)

// LiveList - Auto rerenders on add/remove
val items = liveListOf<String>()

// Signal/wait pattern
runUntilSignal {
    doWork()
    signal()  // Triggers completion
}
```

**Examples include:**
- Game of Life implementation
- Snake game
- Wordle clone
- Mandelbrot renderer
- ChatGPT-style streaming

### Kotter vs Kobweb

| Aspect | Kotter | Kobweb |
|--------|--------|--------|
| Target | Terminal | Web Browser |
| Output | ANSI text | HTML/CSS/JS |
| Input | Keyboard | Mouse + Keyboard |
| Platform | JVM/Native | JS/Wasm |

## Truthish Testing Library

Truthish is a **multiplatform testing assertion library** inspired by Google Truth:

```kotlin
@Test
fun testExample() {
    assertThat(listOf(1, 2, 3).sum()).isEqualTo(6)
    assertThat(isEven(4)).isTrue()

    assertAll {
        that(person.name).isEqualTo("Alice")
        that(person.age).isEqualTo(30)
    }

    val ex = assertThrows<ArithmeticException> {
        10 / 0
    }
    assertThat(ex.message).isEqualTo("/ by zero")
}
```

**Multiplatform targets:** JVM, JS, Native (Win/Linux/Mac/iOS/Android/Wasm)

## Integration with Kotlin Ecosystem

### Compose HTML

Kobweb builds on JetBrains Compose HTML:
- Same `@Composable` functions
- Same `Modifier` pattern
- Same state management (`remember`, `mutableStateOf`)
- CSS APIs extended in `compose-html-ext`

### Kotlin/JS

- Uses IR compiler backend
- Interoperates with JavaScript via `js()` function
- npm dependencies via Gradle
- Webpack bundling

### Ktor Server

Backend runs on Ktor:
- Same routing DSL concepts
- Same plugin architecture
- Serialization plugins (JSON, etc.)

### Gradle Version Catalogs

Kobweb templates use `libs.versions.toml`:
```toml
[versions]
kobweb = "0.23.1"
kotlin = "2.2.10"
jetbrains-compose = "1.8.0"

[libraries]
kobweb = { module = "com.varabyte.kobweb:kobweb-core", version.ref = "kobweb" }
```

## Key Insights

1. **File-based routing** - Similar to Next.js, pages are discovered from file paths, not manually registered

2. **KSP over reflection** - Route registration happens at compile time via KSP, not runtime reflection (better performance)

3. **Escape hatches** - While Kobweb provides abstractions, you can always drop down to:
   - Raw Compose HTML APIs
   - Kotlin/JS browser APIs
   - Direct DOM manipulation

4. **Hybrid SSR/CSR** - Export generates static HTML that hydrates on client, giving SEO benefits with interactivity

5. **Gradle-centric** - Everything flows through Gradle; the CLI is intentionally thin

6. **Silk UI** - Provides Chakra UI-like components (Button, Input, Card, etc.) with theme support

7. **Opinionated but extensible** - Strong conventions for project structure, but can be overridden

8. **Compose Multiplatform distinction** - Kobweb uses Compose HTML (DOM-based), NOT Compose Multiplatform for Web (canvas-based). This means:
   - Better SEO (real HTML elements)
   - Browser DevTools support
   - Smaller download size
   - Access to native HTML features (links, forms, etc.)

## Related Projects

- **kobweb-cli** - Separate repository for CLI binary
- **kobweb-site** - Documentation site (itself built with Kobweb)
- **kotter** - Terminal UI library (multiplatform)
- **truthish** - Testing library (multiplatform)
- **scoop-varabyte** - Windows package manager definition
