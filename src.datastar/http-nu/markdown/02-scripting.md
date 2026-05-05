# http-nu -- Scripting

## Handler Closure

Your script must evaluate to a closure that takes one argument (the request record):

```nushell
# serve.nu
{|req|
    match [$req.method, $req.path] {
        ["GET", "/"] => {body: "<h1>Home</h1>"}
        ["POST", "/api/submit"] => {
            let data = ($req.body | from json)
            {body: ({ok: true, received: $data} | to json)}
        }
        _ => {status: 404, body: "Not found"}
    }
}
```

## Request Record

| Field | Type | Description |
|-------|------|-------------|
| `method` | string | HTTP method (GET, POST, etc.) |
| `path` | string | Request path (e.g., `/api/users`) |
| `query` | record | Parsed query parameters |
| `headers` | record | Request headers (lowercased keys) |
| `body` | string/binary | Request body |
| `params` | record | Route parameters (from router module) |
| `remote_addr` | string | Client IP:port |

## Response Record

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `status` | int | 200 | HTTP status code |
| `headers` | record | {} | Response headers |
| `body` | string/binary/stream | "" | Response body |

## Stdlib Modules

**File**: `src/stdlib/`

http-nu includes built-in Nushell modules available to all scripts:

### datastar module

```nushell
use datastar

# Patch HTML elements via SSE (primary pattern)
"<div id='count'>42</div>" | to datastar-patch-elements
"<div id='count'>42</div>" | to datastar-patch-elements --selector "#target" --mode inner

# Patch signals via SSE (JSON Merge Patch RFC 7386)
{count: 42} | to datastar-patch-signals
{count: 42} | to datastar-patch-signals --only-if-missing

# Execute JavaScript via SSE
"console.log('hello')" | to datastar-execute-script

# Redirect via SSE (JavaScript location change)
"/dashboard" | to datastar-redirect

# Parse signals from request (GET: query param, POST: body JSON)
$req.body | from datastar-signals $req

# CDN script tag for Datastar
SCRIPT-DATASTAR  # Returns {__html: '<script src="...datastar@1.0.1.js"></script>'}
```

Returns SSE-formatted records for piping to `to sse`. Follows the [Datastar SDK ADR](https://github.com/starfederation/datastar/blob/develop/sdk/ADR.md).

### html module

A full HTML DSL — capitalized tag functions that return `{__html: "..."}` records:

```nushell
use html *

# Build HTML with composable tag functions
DIV {class: "card"} (
  H1 "Hello World"
  P "Welcome to http-nu"
  A {href: "/about"} "Learn more"
)

# Void elements (self-closing)
IMG {src: "/logo.png" alt: "Logo"}
INPUT {type: "text" name: "email"}

# Attributes: records with special handling
DIV {class: ["btn" "primary"]}     # class lists joined with space
DIV {style: {color: "red" padding: "10px"}}  # style records → CSS
BUTTON {disabled: true}            # boolean attrs

# Jinja2 template control flow
_for {item: "items"} (LI (_var "item"))
_if "user.is_admin" (BUTTON "Delete")

# Full document
HTML (
  HEAD (TITLE "My App") (META {charset: "utf-8"})
  BODY (MAIN (H1 "Content"))
)
```

### http module

Cookie utilities with secure defaults (HttpOnly, SameSite=Lax, Secure in prod):

```nushell
use http *

# Parse cookies from request
let cookies = $req | cookie parse
# → {session: "abc123", theme: "dark"}

# Set cookies (threads pipeline value through, accumulates Set-Cookie headers)
"OK" | cookie set "session" "abc123" --max-age 86400
     | cookie set "theme" "dark"

# Delete cookie (sets Max-Age=0)
"OK" | cookie delete "session"

# Cookie options
"OK" | cookie set "token" "xyz" --no-httponly --same-site "Strict" --domain ".example.com"
```

### router module

Declarative routing with pattern matching and parameter extraction:

```nushell
use router *

{|req|
  dispatch $req [
    # Exact path match
    (route {path: "/health"} {|req ctx| "OK"})

    # Method + path
    (route {method: "POST", path: "/users"} {|req ctx| "Created"})

    # Path parameters via path-matches
    (route {path-matches: "/users/:id"} {|req ctx| $"User: ($ctx.id)"})

    # Multiple path params
    (route {path-matches: "/users/:userId/posts/:postId"} {|req ctx|
      $"User ($ctx.userId) Post ($ctx.postId)"
    })

    # Header matching
    (route {has-header: {accept: "application/json"}} {|req ctx| {status: "ok"} | to json})

    # Mount sub-handlers under prefix (strips prefix from path)
    (mount "/blog" {|req| dispatch $req $blog_routes})

    # Fallback (true always matches)
    (route true {|req ctx| "Not Found"})
  ]
}
```

Key functions:
- `route <test> <handler>` — Create route with test (record/closure/true) and handler
- `dispatch <req> <routes>` — Find first matching route, execute handler
- `path-matches <pattern>` — Extract `:param` segments, returns record or null
- `has-header <name> <value>` — Check header presence/value (case-insensitive)
- `href <path>` — Resolve path against mount prefix
- `mount <prefix> <handler>` — Mount sub-handler, strips prefix from path

## Custom Commands

**File**: `src/commands.rs`

Built-in commands always available:

| Command | Purpose |
|---------|---------|
| `to sse` | Convert records to SSE text/event-stream format |
| `.static` | Serve static files from a directory |
| `.reverse-proxy` | Proxy requests to another backend |
| `.mj` | Load MiniJinja template environment from directory |
| `.mj compile` | Compile a template string |
| `.mj render` | Render a template with data |
| `.highlight` | Syntax-highlight code (via syntect) |
| `.highlight theme` | List/set highlight themes |
| `.highlight lang` | List available languages |
| `.md` | Render markdown to HTML (via pulldown-cmark) |

When `--store` is enabled, cross.stream commands are also available:

| Command | Purpose |
|---------|---------|
| `.cat` | Read frames from the xs store |
| `.append` | Append a frame to the store |
| `.cas` | Retrieve CAS content |
| `.last` | Get most recent frame(s) |
| `.get` | Get frame by ID |
| `.remove` | Remove a frame |

## SSE Streaming (Datastar Pattern)

```nushell
# serve.nu -- Counter with Datastar
use datastar *

{|req|
    match [$req.method, $req.path] {
        ["GET", "/"] => {
            {body: '
                <div data-signals="{count: 0}">
                    <span data-text="$count"></span>
                    <button data-on-click="$$get(/increment)">+1</button>
                </div>
            '}
        }
        ["GET", "/increment"] => {
            # Return SSE stream via `to sse`
            let count = (.last "counter" | get meta.value | default 0) + 1
            .append "counter" --meta {value: $count} --ttl "last:1"
            {count: $count} | to datastar-patch-signals | to sse
        }
    }
}
```

## Templates (MiniJinja)

http-nu integrates MiniJinja for HTML templating:

```nushell
# With template files
{|req|
    let data = {title: "Hello", items: [1, 2, 3]}
    {body: (render "index.html" $data)}
}
```

Templates support auto-escaping, filters, loops, conditionals, and template inheritance.

## Plugin Loading

```bash
http-nu --plugin /usr/local/bin/nu_plugin_polars :3000 serve.nu
```

Plugins are loaded at startup. Their commands are available in the handler closure.

## Hot Reload Patterns

### File watching (`-w`)

```bash
http-nu -w :3000 serve.nu
# Edit serve.nu → server reloads automatically
```

Uses `notify` crate to watch the script's parent directory. Any file change triggers reload.

### Stdin reload (`-w` with `-`)

```bash
# Send null-terminated scripts for reload
printf 'new_script\0' | http-nu -w :3000 -
```

### Store topic (`--topic`)

```bash
http-nu --store ./store --topic "handler" -w :3000
# .append "handler" with new closure → server reloads
```

Load handler from xs store. Combined with `-w`, reloads when the topic updates.
