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

# SSE merge fragments (Datastar's primary pattern)
datastar merge-fragments "<div id='count'>42</div>"

# SSE merge signals
datastar merge-signals {count: 42}

# SSE execute script
datastar execute-script "console.log('hello')"

# SSE remove fragments
datastar remove-fragments "#old-element"
```

Returns SSE-formatted strings for Datastar's reactive DOM patching.

### html module

```nushell
use html

# Escape HTML entities
html escape "<script>alert('xss')</script>"
# → &lt;script&gt;alert(&#39;xss&#39;)&lt;/script&gt;

# Render markdown to HTML
html md "# Hello\n\nWorld"

# Syntax highlight code
html highlight "fn main() {}" --lang rust
```

### http module

```nushell
use http

# Set cookie
http set-cookie "session" "abc123" --secure --httponly --max-age 3600

# Redirect
http redirect "/dashboard" --status 302
```

### router module

```nushell
use router

# Define routes with parameters
router match $req [
    ["GET", "/users/:id", {|req| {body: $"User ($req.params.id)"}}]
    ["POST", "/users", {|req| {body: "Created"}}]
    ["GET", "/files/*path", {|req| {body: $"File: ($req.params.path)"}}]
]
```

Supports:
- `:param` — Named parameters
- `*param` — Wildcard (catches rest of path)
- Exact string matching

## Custom Commands

**File**: `src/commands.rs`

When `--store` is enabled, these commands are available:

| Command | Purpose |
|---------|---------|
| `.cat` | Read frames from the xs store |
| `.append` | Append a frame to the store |
| `.cas` | Retrieve CAS content |
| `.last` | Get most recent frame(s) |
| `.get` | Get frame by ID |
| `.remove` | Remove a frame |

These are the same commands as in xs, enabling the HTTP handler to interact with the event store directly.

## SSE Streaming (Datastar Pattern)

```nushell
# serve.nu -- Counter with Datastar
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
            # Return SSE stream
            use datastar
            let count = (.last "counter" | get meta.value | default 0) + 1
            .append "counter" --meta {value: $count} --ttl "last:1"
            {
                headers: {"content-type": "text/event-stream"}
                body: (datastar merge-signals {count: $count})
            }
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
