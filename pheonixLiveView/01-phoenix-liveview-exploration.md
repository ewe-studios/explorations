---
location: /home/darkvoid/Boxxed/@formulas/src.pheonixLiveView/phoenix
repository: git@github.com:phoenixframework/phoenix.git
explored_at: 2026-03-29
language: Elixir
framework: Phoenix Framework
category: Real-time Web, LiveView
---

# Phoenix LiveView - Exploration

## Overview

Phoenix LiveView is a **real-time web framework** that enables rich, interactive web applications without writing JavaScript. It uses WebSockets to push HTML diffs to the browser, achieving SPA-like interactivity with server-side rendering simplicity.

### Key Value Proposition

- **No JavaScript Required**: Write interactive UIs in Elixir/HTML
- **Real-time by Default**: WebSocket connections for live updates
- **Server-Side State**: Single source of truth on server
- **SEO Friendly**: Full HTML server rendering
- **Performance**: Binary WebSocket diffs, minimal payload
- **Resilience**: Built on OTP fault tolerance

## Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                    Phoenix LiveView Stack                        │
│                                                                 │
│  ┌─────────────────┐  ┌─────────────────┐  ┌─────────────────┐ │
│  │   Browser       │  │   Browser       │  │   Browser       │ │
│  │   (LiveView.js) │  │   (LiveView.js) │  │   (LiveView.js) │ │
│  └────────┬────────┘  └────────┬────────┘  └────────┬────────┘ │
│           │                    │                    │           │
│           └────────────────────┼────────────────────┘           │
│                      WebSocket │                                │
│                                │                                │
│                    ┌───────────▼───────────┐                   │
│                    │   Phoenix Server      │                   │
│                    │   - WebSocket Handler │                   │
│                    │   - Channel Router    │                   │
│                    │   - LiveView Process  │                   │
│                    └───────────┬───────────┘                   │
│                                │                                │
│                    ┌───────────▼───────────┐                   │
│                    │   LiveView Module     │                   │
│                    │   - mount/3           │                   │
│                    │   - render/1          │                   │
│                    │   - handle_event/3    │                   │
│                    │   - handle_info/2     │                   │
│                    └───────────┬───────────┘                   │
│                                │                                │
│                    ┌───────────▼───────────┐                   │
│                    │   Phoenix PubSub      │                   │
│                    │   - Broadcast events  │                   │
│                    │   - Subscribe topics  │                   │
│                    └───────────┬───────────┘                   │
│                                │                                │
│                    ┌───────────▼───────────┐                   │
│                    │   Database / Ecto     │                   │
│                    │   - Data persistence  │                   │
│                    └───────────────────────┘                   │
└─────────────────────────────────────────────────────────────────┘
```

## Project Structure

```
phoenix/
├── lib/
│   ├── phoenix/              # Core Phoenix framework
│   │   ├── controller.ex     # HTTP controllers
│   │   ├── router.ex         # Request routing
│   │   ├── endpoint.ex       # WebSocket endpoint
│   │   ├── channel.ex        # WebSocket channels
│   │   ├── socket.ex         # Socket abstraction
│   │   └── pubsub.ex         # Publish/subscribe
│   │
│   ├── phoenix_live_view/    # LiveView implementation
│   │   ├── live_view.ex      # Main LiveView behavior
│   │   ├── engine.ex         # HEEx template engine
│   │   ├── diff.ex           # HTML diff algorithm
│   │   ├── render.ex         # Rendering pipeline
│   │   ├── socket.ex         # LiveView socket
│   │   ├── upload.ex         # File uploads
│   │   ├── hook.ex           # JavaScript hooks
│   │   └── test/             # Testing utilities
│   │
│   └── phoenix_html/         # HTML helpers
│
├── installer/                # mix phx.new generator
│   └── lib/
│       └── mix/
│           └── tasks/
│               └── phx.new.ex
│
├── assets/                   # JavaScript client
│   └── js/
│       └── phoenix/
│           ├── live_view.js  # Main LiveView client
│           ├── socket.js     # WebSocket client
│           └── browser.js    # Browser utilities
│
├── guides/                   # Documentation guides
│   ├── introduction/
│   ├── installation/
│   ├── routing/
│   ├── live_view/
│   ├── channels/
│   └── testing/
│
└── test/                     # Test suite
    ├── phoenix/
    └── phoenix_live_view/
```

## Core Concepts

### 1. LiveView Lifecycle

```elixir
defmodule MyAppWeb.CounterLive do
  use Phoenix.LiveView

  # 1. Mount: Called when LiveView starts
  def mount(_params, _session, socket) do
    # Assign initial state
    {:ok, assign(socket, count: 0, timer: 0)}
  end

  # 2. Render: Convert state to HTML (HEEx templates)
  def render(assigns) do
    ~H"""
    <div>
      <h1>Count: {@count}</h1>
      <button phx-click="increment">+</button>
      <button phx-click="decrement">-</button>
      <p>Timer: {@timer}</p>
    </div>
    """
  end

  # 3. Handle Event: Process user interactions
  def handle_event("increment", _params, socket) do
    {:noreply, assign(socket, count: socket.assigns.count + 1)}
  end

  def handle_event("decrement", _params, socket) do
    {:noreply, assign(socket, count: socket.assigns.count - 1)}
  end

  # 4. Handle Info: Process async messages
  def handle_info(:tick, socket) do
    {:noreply, assign(socket, timer: socket.assigns.timer + 1)}
  end
end
```

### 2. HEEx Templates

HEEx (HTML + EEx) is Phoenix's template format:

```heex
{# Basic interpolation }
<h1>{@user.name}</h1>

{# Conditionals }
{if @logged_in? do}
  <p>Welcome!</p>
{else}
  <p>Please log in</p>
{end}

{# Loops }
<ul>
  {for user <- @users do}
    <li>{user.name}</li>
  {end}
</ul>

{# Event handlers }
<button phx-click="save">Save</button>

{# Form with validation }
<form phx-submit="submit" phx-change="validate">
  <input name="email" value={@email} />
  {for error <- @errors[:email] do}
    <span class="error">{error}</span>
  {end}
</form>

{# File upload }
<form phx-submit="upload" phx-change="validate">
  {live_file_upload @uploads.avatar}
</form>

{# Components }
<MyAppWeb.CoreComponents.button>
  Click me
</MyAppWeb.CoreComponents.button>
```

### 3. State Management

```elixir
# Assign state
socket = assign(socket, :count, 0)
socket = assign(socket, count: 0, user: user)

# Access state
socket.assigns.count
@count  # In templates

# Update state
{:noreply, assign(socket, count: new_count)}

# Async updates with send_self
send(self(), {:update_data, new_data})

def handle_info({:update_data, data}, socket) do
  {:noreply, assign(socket, data: data)}
end
```

### 4. Event Handling

```elixir
# Button click
<button phx-click="delete" phx-value-id={@id}>Delete</button>

def handle_event("delete", %{"id" => id}, socket) do
  # Delete logic
  {:noreply, socket}
end

# Form submission
<form phx-submit="save">
  <input name="name" />
</form>

def handle_event("save", %{"name" => name}, socket) do
  # Save logic
  {:noreply, socket}
end

# Debounced input
<input name="search" phx-change="search" phx-debounce="500" />

# Throttled input
<input name="search" phx-change="search" phx-throttle="500" />

# Keydown events
<input phx-keydown="keydown" phx-key="Enter" />
```

### 5. PubSub Integration

```elixir
# Subscribe to topic
def mount(_params, _session, socket) do
  Phoenix.PubSub.subscribe(MyApp.PubSub, "room:lobby")

  {:ok, socket}
end

# Broadcast to subscribers
def handle_event("message", %{"text" => text}, socket) do
  Phoenix.PubSub.broadcast(MyApp.PubSub, "room:lobby",
    {:message, text}
  )

  {:noreply, socket}
end

# Receive broadcast
def handle_info({:message, text}, socket) do
  {:noreply, assign(socket, messages: [text | socket.assigns.messages])}
end
```

### 6. Live Components

Stateful components within LiveView:

```elixir
defmodule MyAppWeb.UserComponent do
  use Phoenix.LiveComponent

  def render(assigns) do
    ~H"""
    <div>
      <h2>{@user.name}</h2>
      <button phx-click="edit" phx-target={@myself}>Edit</button>
    </div>
    """
  end

  def handle_event("edit", _params, socket) do
    # Component-specific logic
    {:noreply, socket}
  end
end

# Usage in LiveView
<.live_component module={UserComponent} id={user.id} user={user} />
```

## JavaScript Client

### LiveView.js

```javascript
import { LiveSocket } from "phoenix_live_view"
import { Socket } from "phoenix"

let liveSocket = new LiveSocket("/live", Socket, {
  params: { _csrf_token: csrfToken },
  hooks: {
    // Custom JavaScript hooks
    MapView: {
      mounted() {
        this.handleEvent("update_markers", markers => {
          // Update map markers
        })
      }
    }
  }
})

liveSocket.connect()
```

### JavaScript Hooks

```javascript
// Define hook
let Hooks = {}

Hooks.MapView = {
  mounted() {
    // Initialize on mount
    this.map = new MapLib(this.el)

    // Listen for server events
    this.handleEvent("update_markers", ({ markers }) => {
      this.map.updateMarkers(markers)
    })
  },

  updated() {
    // Called when LiveView updates
  },

  destroyed() {
    // Cleanup on destroy
  }
}

// Use in template
<div id="map" phx-hook="MapView"></div>
```

## HTML Diff Algorithm

LiveView sends minimal HTML diffs:

```elixir
# Initial render (full HTML)
%{
  0 => "<div>",
  1 => "<h1>Count: 0</h1>",
  2 => "<button>+</button>",
  3 => "</div>"
}

# After increment (diff)
%{
  d => %{  # d = dynamic
    0 => "1"  # Only changed text
  }
}
```

**Diff process:**
1. Server renders full template
2. Compares with previous render
3. Identifies changed nodes
4. Sends minimal diff to client
5. Client patches DOM

## Routing

```elixir
# router.ex
defmodule MyAppWeb.Router do
  use Phoenix.Router

  # LiveView routes
  live "/", DashboardLive
  live "/users", UserListLive
  live "/users/:id", UserShowLive

  # Nested routes
  live "/users/:user_id/posts", UserPostLive

  # With layout
  live "/admin", AdminLive, layout: {MyAppWeb.Layouts, :admin}
end
```

## Testing

```elixir
defmodule MyAppWeb.CounterLiveTest do
  use MyAppWeb.ConnCase

  import Phoenix.LiveViewTest

  test "increment counter", %{conn: conn} do
    {:ok, view, _html} = live(conn, "/counter")

    assert render(view) =~ "Count: 0"

    assert view
           |> element("button", "+")
           |> render_click()

    assert render(view) =~ "Count: 1"
  end

  test "broadcasts updates", %{conn: conn} do
    {:ok, view1, _} = live(conn, "/counter")
    {:ok, view2, _} = live(conn, "/counter")

    render_click(view1, "increment")

    # Both views should see update
    assert render(view1) =~ "Count: 1"
    assert render(view2) =~ "Count: 1"
  end
end
```

## Production Considerations

### Scaling

```elixir
# Configure Phoenix endpoint
config :my_app, MyAppWeb.Endpoint,
  http: [port: 4000],
  url: [host: "example.com"],
  server: true,
  pubsub: [
    name: MyApp.PubSub,
    adapter: Phoenix.PubSub.PG2  # Distributed PG2
  ]
```

### Security

```elixir
# CSRF protection
config :my_app, MyAppWeb.Endpoint,
  csrf_token: true

# Rate limiting
def handle_event("submit", params, socket) do
  if rate_limit_exceeded?(socket.assigns.user) do
    {:noreply, put_flash(socket, :error, "Too many requests")}
  else
    # Process
  end
end
```

### Monitoring

```elixir
# Telemetry hooks
:telemetry.attach(
  "my-app",
  [:phoenix, :live_view, :mount, :stop],
  &MyApp.Metrics.handle_event/4,
  nil
)
```

---

## Related Deep Dives

- [00-zero-to-liveview-engineer.md](./00-zero-to-liveview-engineer.md) - Fundamentals
- [01-phoenix-channel-deep-dive.md](./01-phoenix-channel-deep-dive.md) - WebSocket channels
- [02-liveview-protocol-deep-dive.md](./02-liveview-protocol-deep-dive.md) - Protocol details
- [03-javascript-client-deep-dive.md](./03-javascript-client-deep-dive.md) - Client implementation
- [04-realtime-patterns-deep-dive.md](./04-realtime-patterns-deep-dive.md) - Realtime patterns
- [rust-revision.md](./rust-revision.md) - Rust implementation considerations
- [production-grade.md](./production-grade.md) - Production deployment guide
