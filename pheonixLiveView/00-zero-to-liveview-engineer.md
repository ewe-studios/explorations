---
location: /home/darkvoid/Boxxed/@dev/repo-expolorations/pheonixLiveView
explored_at: 2026-03-29
prerequisites: Basic programming knowledge, Web fundamentals (HTTP, HTML)
---

# Zero to Phoenix LiveView Engineer - Complete Fundamentals

## Table of Contents

1. [What is Phoenix LiveView?](#what-is-phoenix-liveview)
2. [Why LiveView?](#why-liveview)
3. [Installation](#installation)
4. [Your First LiveView](#your-first-liveview)
5. [State Management](#state-management)
6. [Event Handling](#event-handling)
7. [Realtime Updates](#realtime-updates)
8. [Forms and Validation](#forms-and-validation)
9. [Live Components](#live-components)

## What is Phoenix LiveView?

Phoenix LiveView is a **real-time web framework** that lets you build interactive web applications without writing JavaScript. It uses WebSockets to push HTML updates to the browser, achieving Single Page App (SPA) interactivity with server-side rendering simplicity.

### The Problem LiveView Solves

**Traditional Web Apps:**
```
Backend (Ruby/Python/Elixir) → Renders HTML
              ↓
Frontend (React/Vue) → Separate codebase
              ↓
API (REST/GraphQL) → Connects both
              ↓
Complexity: Two codebases, state sync issues, API versioning
```

**LiveView Approach:**
```
Phoenix + LiveView → Renders HTML + Handles Events
              ↓
WebSocket → Pushes HTML diffs
              ↓
Simplicity: One codebase, server state is source of truth
```

### Key Concepts

| Term | Definition |
|------|------------|
| **LiveView** | Server process that renders HTML and handles events |
| **HEEx** | HTML+EEx template language |
| **Socket** | WebSocket connection between browser and server |
| **PubSub** | Publish/Subscribe for broadcasting updates |
| **Hook** | JavaScript for client-side interactivity |

## Why LiveView?

### Benefits

1. **No JavaScript Framework**: Write interactive UIs in Elixir/HTML
2. **Real-time by Default**: WebSocket connection for live updates
3. **Server-Side State**: Single source of truth
4. **SEO Friendly**: Full HTML rendered on server
5. **Fast Development**: No API design, no state sync
6. **Resilient**: Built on OTP fault tolerance

### When to Use LiveView

**Good fit:**
- Dashboards with live data
- Collaborative apps (chat, editing)
- Forms with validation
- Real-time notifications
- Admin interfaces

**Not recommended:**
- Mobile apps (use API)
- Offline-first apps
- Very high-frequency updates (use canvas/WebGL)

## Installation

### Prerequisites

1. **Install Elixir** (1.14+):
```bash
# macOS
brew install elixir

# Ubuntu
wget https://packages.erlang-solutions.com/erlang-solutions_2.0_all.deb
sudo dpkg -i erlang-solutions_2.0_all.deb
sudo apt-get update
sudo apt-get install elixir
```

2. **Install Hex** (package manager):
```bash
mix local.hex
```

3. **Install Phoenix generator**:
```bash
mix archive.install hex phx_new
```

### Create New Project

```bash
mix phx.new myapp

# Options:
# --live    Include LiveView
# --ecto    Include database (Ecto)
# --app     App name (myapp)
# --module  Module name (MyApp)

cd myapp
mix setup
```

### Start the Server

```bash
mix phx.server

# Visit http://localhost:4000
```

## Your First LiveView

### Create LiveView Module

```elixir
# lib/my_app_web/live/counter_live.ex
defmodule MyAppWeb.CounterLive do
  use Phoenix.LiveView

  # Mount: Called when LiveView starts
  def mount(_params, _session, socket) do
    # Initialize state
    {:ok, assign(socket, count: 0)}
  end

  # Render: Convert state to HTML
  def render(assigns) do
    ~H"""
    <div>
      <h1>Count: {@count}</h1>
      <button phx-click="increment">+</button>
      <button phx-click="decrement">-</button>
    </div>
    """
  end

  # Handle events
  def handle_event("increment", _params, socket) do
    {:noreply, assign(socket, count: socket.assigns.count + 1)}
  end

  def handle_event("decrement", _params, socket) do
    {:noreply, assign(socket, count: socket.assigns.count - 1)}
  end
end
```

### Add Route

```elixir
# lib/my_app_web/router.ex
defmodule MyAppWeb.Router do
  use Phoenix.Router

  live "/", CounterLive
end
```

### Result

Visit `http://localhost:4000` and click buttons - count updates without page reload!

## State Management

### Assigning State

```elixir
# Single value
socket = assign(socket, :count, 0)

# Multiple values
socket = assign(socket, count: 0, user: user, messages: [])

# In mount
def mount(_params, _session, socket) do
  {:ok, assign(socket, %{count: 0, timer: 0})}
end
```

### Accessing State

```elixir
# In functions
socket.assigns.count
@count  # In templates (shorthand)

# Pattern matching
%{assigns: %{count: count}} = socket
```

### Updating State

```elixir
# Replace entire state
{:noreply, assign(socket, count: new_count)}

# Update specific key
{:noreply, update(socket, :count, fn c -> c + 1 end)}

# Multiple updates
{:noreply, socket |> assign(:count, 1) |> assign(:timer, 0)}
```

### Async State Updates

```elixir
def mount(_params, _session, socket) do
  # Schedule periodic update
  :timer.send_interval(1000, self(), :tick)

  {:ok, assign(socket, timer: 0)}
end

def handle_info(:tick, socket) do
  {:noreply, assign(socket, timer: socket.assigns.timer + 1)}
end
```

## Event Handling

### Button Clicks

```heex
<button phx-click="save">Save</button>

def handle_event("save", _params, socket) do
  # Save logic
  {:noreply, socket}
end
```

### Form Submission

```heex
<form phx-submit="submit">
  <input name="email" />
  <button type="submit">Submit</button>
</form>

def handle_event("submit", %{"email" => email}, socket) do
  # Process email
  {:noreply, socket}
end
```

### Input Changes

```heex
<input
  name="search"
  value={@search}
  phx-change="search"
  phx-debounce="500"
/>

def handle_event("search", %{"search" => search}, socket) do
  {:noreply, assign(socket, search: search)}
end
```

**Debounce/Throttle:**
- `phx-debounce="500"` - Wait 500ms after last input
- `phx-throttle="500"` - Wait 500ms between inputs

### Key Events

```heex
<input
  phx-keydown="keydown"
  phx-key="Enter"
  placeholder="Press Enter"
/>

def handle_event("keydown", %{"key" => "Enter"}, socket) do
  # Handle Enter key
  {:noreply, socket}
end
```

### Focus Events

```heex
<input
  phx-focus="focus"
  phx-blur="blur"
/>

def handle_event("focus", _params, socket) do
  {:noreply, assign(socket, focused: true)}
end

def handle_event("blur", _params, socket) do
  {:noreply, assign(socket, focused: false)}
end
```

## Realtime Updates

### PubSub Broadcasting

```elixir
defmodule MyAppWeb.RoomLive do
  use Phoenix.LiveView

  def mount(_params, _session, socket) do
    # Subscribe to topic
    Phoenix.PubSub.subscribe(MyApp.PubSub, "room:lobby")

    {:ok, assign(socket, messages: [], users: [])}
  end

  def handle_event("send_message", %{"text" => text}, socket) do
    # Broadcast to all subscribers
    Phoenix.PubSub.broadcast(MyApp.PubSub, "room:lobby",
      {:new_message, text}
    )

    {:noreply, socket}
  end

  def handle_info({:new_message, text}, socket) do
    # Update state when message received
    {:noreply, assign(socket, messages: [text | socket.assigns.messages])}
  end
end
```

### Multiple Users

```elixir
# User 1 sends message
# → Server broadcasts to "room:lobby"
# → All connected users receive update
# → HTML diff pushed to each browser
```

### Presence (Who's Online)

```elixir
defmodule MyAppWeb.RoomLive do
  use Phoenix.LiveView
  import Phoenix.Channel, only: [track_presence: 2]

  def mount(_params, _session, socket) do
    # Track user presence
    track_presence(socket, socket.assigns.user_id)

    # Get online users
    online_users = MyApp.Presence.list("room:lobby")

    {:ok, assign(socket, online_users: online_users)}
  end
end
```

## Forms and Validation

### Basic Form

```heex
<form phx-submit="submit" phx-change="validate">
  <.input
    field={@form[:email]}
    type="email"
    label="Email"
    phx-debounce="blur"
  />

  <.input
    field={@form[:password]}
    type="password"
    label="Password"
  />

  <button type="submit">Sign Up</button>

  {for error <- @form.errors do}
    <p class="error">{error_message(error)}</p>
  {end}
</form>
```

### Server-Side Validation

```elixir
def mount(_params, _session, socket) do
  form = to_form(%{}, as: "user")
  {:ok, assign(socket, form: form)}
end

def handle_event("validate", %{"user" => params}, socket) do
  changeset = User.changeset(%User{}, params)

  form =
    params
    |> to_form(as: "user")
    |> Map.put(:errors, translate_errors(changeset))

  {:noreply, assign(socket, form: form)}
end

def handle_event("submit", %{"user" => params}, socket) do
  case MyApp.create_user(params) do
    {:ok, user} ->
      {:noreply, redirect(socket, to: "/dashboard")}

    {:error, changeset} ->
      form = to_form(params, as: "user", errors: translate_errors(changeset))
      {:noreply, assign(socket, form: form)}
  end
end
```

### Ecto Integration

```elixir
defmodule MyApp.User do
  use Ecto.Schema
  import Ecto.Changeset

  schema "users" do
    field :email, :string
    field :password, :string
  end

  def changeset(user, attrs) do
    user
    |> cast(attrs, [:email, :password])
    |> validate_required([:email, :password])
    |> validate_format(:email, ~r/@/)
    |> validate_length(:password, min: 8)
  end
end
```

## Live Components

### Define Component

```elixir
defmodule MyAppWeb.UserComponent do
  use Phoenix.LiveComponent

  def render(assigns) do
    ~H"""
    <div>
      <h2>{@user.name}</h2>
      <p>{@user.email}</p>
      <button phx-click="edit" phx-target={@myself}>Edit</button>
    </div>
    """
  end

  def handle_event("edit", _params, socket) do
    # Component-specific logic
    {:noreply, socket}
  end
end
```

### Use in LiveView

```heex
<.live_component
  module={UserComponent}
  id={user.id}
  user={user}
/>

{# In a loop }
{for user <- @users do}
  <.live_component
    module={UserComponent}
    id={user.id}
    user={user}
  />
{end}
```

### Component State

```elixir
defmodule MyAppWeb.TodoComponent do
  use Phoenix.LiveComponent

  def render(assigns) do
    ~H"""
    <div>
      <input
        type="checkbox"
        checked={@todo.completed}
        phx-click="toggle"
        phx-target={@myself}
      />
      {@todo.text}
    </div>
    """
  end

  def handle_event("toggle", _params, socket) do
    # Toggle component state
    new_todo = %{socket.assigns.todo | completed: !socket.assigns.todo.completed}

    # Notify parent
    send(self(), {:todo_updated, new_todo})

    {:noreply, assign(socket, todo: new_todo)}
  end
end
```

---

**Next Steps:**
- [01-phoenix-liveview-exploration.md](./01-phoenix-liveview-exploration.md) - Full architecture
- [01-phoenix-channel-deep-dive.md](./01-phoenix-channel-deep-dive.md) - WebSocket channels
- [02-liveview-protocol-deep-dive.md](./02-liveview-protocol-deep-dive.md) - Protocol details
