# Bud Framework Deep Dive

## Overview

Bud is a full-stack web framework for Go that generates boilerplate code automatically. It feels like using a modern JavaScript framework but with Go's type safety and performance. Bud's core philosophy is that framework code should be "boring" - predictable, readable, and generatable.

## Architecture

Bud's architecture centers around code generation:

```
┌─────────────────────────────────────────────────────────────┐
│                    Bud Framework                             │
├─────────────────────────────────────────────────────────────┤
│  CLI Layer                                                   │
│  ┌──────────┐  ┌──────────┐  ┌──────────┐  ┌──────────┐    │
│  │ bud run  │  │ bud build│  │ bud gen  │  │bud create│    │
│  └──────────┘  └──────────┘  └──────────┘  └──────────┘    │
├─────────────────────────────────────────────────────────────┤
│  Code Generation Layer                                       │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐      │
│  │ Controllers  │  │   Views      │  │   Public     │      │
│  │  Generator   │  │  Generator   │  │   Embed      │      │
│  └──────────────┘  └──────────────┘  └──────────────┘      │
├─────────────────────────────────────────────────────────────┤
│  Runtime Layer                                               │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐      │
│  │  HTTP Server │  │ Live Reload  │  │  Hot Module  │      │
│  │              │  │   Server     │  │ Replacement  │      │
│  └──────────────┘  └──────────────┘  └──────────────┘      │
├─────────────────────────────────────────────────────────────┤
│  Transformation Layer                                        │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐      │
│  │  ESBuild     │  │   Svelte     │  │    CSS       │      │
│  │  (Bundler)   │  │  Compiler    │  │ Preprocessor │      │
│  └──────────────┘  └──────────────┘  └──────────────┘      │
└─────────────────────────────────────────────────────────────┘
```

## Project Structure

Bud enforces a convention-based directory structure:

```
myapp/
├── app/
│   ├── controller/         # HTTP controllers
│   │   └── hello/
│   │       └── hello.go
│   ├── view/               # View templates (Svelte, etc.)
│   │   └── hello/
│   │       └── index.svelte
│   ├── public/             # Static assets
│   │   ├── favicon.ico
│   │   └── styles/
│   └── lib/                # Shared libraries
├── bud/                    # Generated code (gitignored)
│   ├── bud.go              # Generated framework code
│   ├── controller/         # Generated controller code
│   ├── view/               # Compiled view bundles
│   └── build/              # Production binaries
├── go.mod
└── main.go
```

## Controller System

### Controller Definition

Controllers in Bud are simple Go structs with methods that map to HTTP routes:

```go
// app/controller/hello/hello.go
package hello

import (
    "context"
    "github.com/livebud/bud/framework/controller"
)

type Controller struct{}

// GET /hello
func (c *Controller) Get() string {
    return "Hello, World!"
}

// GET /hello/:name
func (c *Controller) GetShow(name string) string {
    return "Hello, " + name + "!"
}

// POST /hello
func (c *Controller) Post(name string) string {
    return "Created: " + name
}
```

### Route Mapping Convention

Bud uses a convention-based routing system that maps method names to HTTP routes:

| Method Pattern | HTTP Verb | Route Pattern | Description |
|---------------|-----------|---------------|-------------|
| `Index()` | GET | `/` | List resources |
| `New()` | GET | `/new` | Form to create resource |
| `Create()` | POST | `/` | Create resource |
| `Show(id)` | GET | `/:id` | Show single resource |
| `Edit(id)` | GET | `/:id/edit` | Form to edit resource |
| `Update(id)` | PATCH | `/:id` | Update resource |
| `Delete(id)` | DELETE | `/:id` | Delete resource |

### Generated Controller Code

Bud generates the HTTP handler layer from your controller:

```go
// bud/controller/controller.go (generated)
package controller

import (
    "net/http"
    "github.com/matthewmueller/hello/app/controller/hello"
)

func New(helloCtrl *hello.Controller) *Controller {
    return &Controller{hello: helloCtrl}
}

type Controller struct {
    hello *hello.Controller
}

func (c *Controller) Mount(router http.Handler) {
    router.HandleFunc("/hello", func(w http.ResponseWriter, r *http.Request) {
        switch r.Method {
        case "GET":
            result := c.hello.Get()
            json.NewEncoder(w).Encode(result)
        case "POST":
            name := r.FormValue("name")
            result := c.hello.Post(name)
            json.NewEncoder(w).Encode(result)
        }
    })
    
    router.HandleFunc("/hello/{name}", func(w http.ResponseWriter, r *http.Request) {
        name := mux.Vars(r)["name"]
        result := c.hello.GetShow(name)
        json.NewEncoder(w).Encode(result)
    })
}
```

### Controller Dependencies

Controllers use dependency injection:

```go
// app/controller/hello/hello.go
package hello

import (
    "context"
    "github.com/matthewmueller/hackernews"
)

func New(hn *hackernews.Client) *Controller {
    return &Controller{hn}
}

type Controller struct {
    hn *hackernews.Client
}

func (c *Controller) Index(ctx context.Context) ([]*hackernews.Story, error) {
    return c.hn.FrontPage(ctx)
}
```

Bud's DI container automatically resolves and injects dependencies.

## View System

### Svelte Integration

Bud compiles Svelte components to both server-side rendered HTML and client-side bundles:

```svelte
<!-- app/view/hello/index.svelte -->
<script>
    export let name = "World"
</script>

<h1>Hello, {name}!</h1>

<style>
    h1 {
        color: #ff3e00;
    }
</style>
```

### View Generator

The view generator compiles Svelte components:

```go
// bud/framework/view/view.go
package view

import (
    "github.com/livebud/bud/framework"
    "github.com/livebud/bud/package/genfs"
    "github.com/livebud/bud/package/gotemplate"
)

func Generate(state *State) ([]byte, error) {
    return generator.Generate(state)
}

type Generator struct {
    flag      *framework.Flag
    module    *gomod.Module
    transform *transformrt.Map
}

func (g *Generator) GenerateFile(fsys genfs.FS, file *genfs.File) error {
    state, err := Load(fsys, g.module, g.transform, g.flag)
    if err != nil {
        return err
    }
    code, err := Generate(state)
    file.Data = code
    return nil
}
```

### Viewer Interface

Views implement the Viewer interface for rendering:

```go
// bud/package/viewer/viewer.go
type Viewer interface {
    Mount(r *router.Router) error
    Render(ctx context.Context, key string, propMap PropMap) ([]byte, error)
    RenderError(ctx context.Context, key string, propMap PropMap, err error) []byte
    Bundle(ctx context.Context, embed virtual.Tree) error
}

type Page struct {
    *View
    Frames []*View   // Innermost to outermost views
    Layout *View     // Layout wrapper
    Error  *View     // Error page
    Route  string
}
```

### Server-Side Rendering

Bud renders views server-side for initial page loads:

```go
func StaticPropMap(page *Page, r *http.Request) (PropMap, error) {
    props := map[string]interface{}{}
    
    // Extract query params, form data, JSON body
    if err := request.Unmarshal(r, &props); err != nil {
        return nil, err
    }
    
    // Build prop map for all views in the page
    propMap := PropMap{}
    propMap[page.Key] = props
    if page.Layout != nil {
        propMap[page.Layout.Key] = props
    }
    for _, frame := range page.Frames {
        propMap[frame.Key] = props
    }
    
    return propMap, nil
}
```

## Build System

### Development Mode

```bash
bud run
```

In development mode, Bud:
1. Watches for file changes
2. Regenerates code on every request
3. Enables live reload
4. Serves source maps
5. Uses unminified bundles

### Production Build

```bash
bud build
```

Production builds:
1. Generate all code once
2. Minify JavaScript and CSS
3. Embed static assets
4. Build a single binary

```go
// bud/internal/cli/build.go
type Build struct {
    Flag *framework.Flag
}

func (c *CLI) Build(ctx context.Context, in *Build) error {
    return c.Generate(ctx, &Generate{Flag: in.Flag})
}
```

### Build Flags

```go
// bud/framework/framework.go
type Flag struct {
    Embed  EmbedFlags  // What to embed
    Minify MinifyFlags // What to minify
    Hot    bool        // Enable hot reload
}

type EmbedFlags struct {
    Views    bool
    Public   bool
    Assets   bool
}

type MinifyFlags struct {
    JS       bool
    CSS      bool
    HTML     bool
}
```

## Live Reload System

Bud implements live reload for instant browser updates:

```go
// Conceptual live reload flow
type LiveReload struct {
    clients map[string]*websocket.Conn
    mu      sync.Mutex
}

func (lr *LiveReload) Reload() {
    lr.mu.Lock()
    defer lr.mu.Unlock()
    
    for _, client := range lr.clients {
        client.WriteJSON(map[string]string{
            "type": "reload",
        })
    }
}

func (lr *LiveReload) Connect(w http.ResponseWriter, r *http.Request) {
    conn, _ := websocket.Accept(w, r)
    lr.clients[conn.ID] = conn
}
```

## Code Generation Philosophy

Bud's philosophy is that framework code should be boring - predictable and readable. All generated code lives in `bud/` and can be regenerated at any time.

### Generation Pipeline

```
┌─────────────┐     ┌─────────────┐     ┌─────────────┐
│   Source    │────▶│   Parser    │────▶│    State    │
│   (app/)    │     │             │     │             │
└─────────────┘     └─────────────┘     └─────────────┘
                                              │
                                              ▼
┌─────────────┐     ┌─────────────┐     ┌─────────────┐
│  Generated  │◀────│  Template   │◀────│  Generator  │
│   (bud/)    │     │  (gotext)   │     │             │
└─────────────┘     └─────────────┘     └─────────────┘
```

### Template System

Bud uses Go's `text/template` with embedded template strings:

```go
// bud/framework/controller/controller.go
//go:embed controller.gotext
var template string

var generator = gotemplate.MustParse("framework/controller/controller.gotext", template)

func Generate(state *State) ([]byte, error) {
    return generator.Generate(state)
}
```

## Dependency Injection

Bud includes a compile-time dependency injection system:

```go
// bud/package/di/di.go
type Injector struct {
    providers []Provider
    nodes     map[string]*Node
}

func (inj *Injector) Provide(fn interface{}) error {
    node, err := newNode(fn)
    if err != nil {
        return err
    }
    inj.nodes[node.key] = node
    return nil
}

func (inj *Injector) Resolve(typ reflect.Type) (interface{}, error) {
    node, ok := inj.nodes[typeKey(typ)]
    if !ok {
        return nil, fmt.Errorf("no provider for %v", typ)
    }
    return node.Resolve(inj)
}
```

## Request/Response Flow

```
┌─────────────────────────────────────────────────────────────┐
│                     Request Flow                              │
└─────────────────────────────────────────────────────────────┘

1. Client Request
         │
         ▼
2. HTTP Server (budsvr)
         │
         ▼
3. Middleware Stack
   - httpbuffer (buffer response)
   - methodoverride (REST methods)
         │
         ▼
4. Router
         │
         ▼
5. Controller (generated)
   - Parse route params
   - Call user controller
         │
         ▼
6. View Renderer (if applicable)
   - Server-side render
   - Inject props
         │
         ▼
7. Response
   - Set headers
   - Write body
   - Handle errors
```

## ESBuild Integration

Bud uses ESBuild for fast JavaScript bundling:

```go
// bud/package/es/es.go
type Bundler struct {
    entrypoints map[string]string
    options     BuildOptions
}

func (b *Bundler) Build(ctx context.Context) (*BuildResult, error) {
    // Configure ESBuild options
    options := api.BuildOptions{
        EntryPoints: maps.Keys(b.entrypoints),
        Outdir:      "/bud/view",
        Bundle:      true,
        Minify:      b.options.Minify,
        Target:      api.ES2020,
        Format:      api.FormatESModule,
    }
    
    result := api.Build(options)
    return &BuildResult{
        Outputs: result.OutputFiles,
    }, nil
}
```

## Key Patterns

### Convention Over Configuration

Bud uses conventions to reduce configuration:
- Directory structure defines routes
- Method names map to HTTP verbs
- File names become route segments

### Code Generation Over Runtime Magic

All framework "magic" is generated as readable Go code:
- No reflection at runtime
- Type-safe compile-time checks
- Debuggable generated code

### Progressive Enhancement

Views work without JavaScript but enhance with it:
- Server-rendered HTML by default
- Client-side hydration for interactivity
- Graceful degradation

## Example: Full CRUD Application

```go
// app/controller/posts/posts.go
package posts

import (
    "context"
    "github.com/matthewmueller/myapp/app/model"
)

type Controller struct {
    posts *model.PostStore
}

func New(posts *model.PostStore) *Controller {
    return &Controller{posts}
}

// GET /posts
func (c *Controller) Index(ctx context.Context) ([]*model.Post, error) {
    return c.posts.All(ctx)
}

// GET /posts/new
func (c *Controller) New(ctx context.Context) {}

// POST /posts
func (c *Controller) Create(ctx context.Context, title, body string) (*model.Post, error) {
    return c.posts.Create(ctx, title, body)
}

// GET /posts/:id
func (c *Controller) Show(ctx context.Context, id int) (*model.Post, error) {
    return c.posts.Find(ctx, id)
}

// GET /posts/:id/edit
func (c *Controller) Edit(ctx context.Context, id int) (*model.Post, error) {
    return c.posts.Find(ctx, id)
}

// PATCH /posts/:id
func (c *Controller) Update(ctx context.Context, id int, title, body string) (*model.Post, error) {
    return c.posts.Update(ctx, id, title, body)
}

// DELETE /posts/:id
func (c *Controller) Delete(ctx context.Context, id int) error {
    return c.posts.Delete(ctx, id)
}
```

```svelte
<!-- app/view/posts/index.svelte -->
<script>
    export let posts = []
</script>

<h1>All Posts</h1>

<a href="/posts/new">New Post</a>

<ul>
    {#each posts as post}
        <li>
            <a href="/posts/{post.id}">{post.title}</a>
        </li>
    {/each}
</ul>
```

## Performance Considerations

1. **Compile-time Code Generation**: No runtime overhead from framework abstraction
2. **Embedded Assets**: Static files embedded in binary for fast serving
3. **ESBuild**: Fastest JavaScript bundler (10-100x faster than webpack)
4. **Server-Side Rendering**: Fast initial page loads
5. **Hot Module Replacement**: Instant updates during development

## Summary

Bud Framework provides:

1. **Convention-based routing** - Method names map to routes
2. **Code generation** - All framework code is generated and readable
3. **Svelte integration** - Full SSR and client-side rendering
4. **Live reload** - Instant browser updates
5. **Dependency injection** - Compile-time DI container
6. **ESBuild bundling** - Fast JavaScript builds
7. **Production-ready** - Single binary deployment

The framework's core insight is that framework code should be "boring" - predictable, readable, and generatable. By generating boilerplate code, Bud lets developers focus on application logic while maintaining full type safety and performance.
