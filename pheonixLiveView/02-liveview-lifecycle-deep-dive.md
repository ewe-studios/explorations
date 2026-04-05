---
source: /home/darkvoid/Boxxed/@formulas/src.pheonixLiveView/phoenix
repository: github.com:phoenixframework/phoenix
explored_at: 2026-04-05
focus: LiveView lifecycle, state management, process architecture, socket handling
---

# Deep Dive: LiveView Lifecycle and State Management

## Overview

This deep dive examines Phoenix LiveView's lifecycle, state management patterns, and process architecture. We explore how LiveView processes are spawned, managed, and how state flows through the system.

## Architecture

```mermaid
flowchart TB
    subgraph Client
        Browser[Browser - LiveView.js] --> WS[WebSocket Connection]
    end
    
    subgraph Phoenix Endpoint
        WS --> Router[Phoenix Router]
        Router --> Socket[LiveSocket Handler]
    end
    
    subgraph LiveView Process
        Socket --> Mount[mount/3 Callback]
        Mount --> State[Socket State]
        State --> Render[render/1]
        Render --> Diff[HTML Diff Engine]
        Diff --> Push[Push to Client]
    end
    
    subgraph Event Handling
        Browser --"phx-click"--> WS
        WS --> HandleEvent[handle_event/3]
        HandleEvent --> State
    end
    
    subgraph PubSub
        HandleEvent --> Broadcast[Phoenix.PubSub]
        Broadcast --> OtherLV[Other LiveViews]
    end
    
    subgraph Cleanup
        Browser --"disconnect"--> Stop[terminate/2]
        Stop --> Cleanup[Resource Cleanup]
    end
```

## LiveView Process Architecture

### Process Lifecycle

```elixir
# lib/phoenix_live_view.ex

defmodule Phoenix.LiveView do
  @moduledoc """
  LiveView creates a separate BEAM process for each connected client.
  
  Process lifecycle:
  1. mount/3 - Initialize state
  2. render/1 - Generate HTML
  3. handle_event/3 - Handle user events
  4. handle_info/2 - Handle async messages
  5. terminate/2 - Cleanup on disconnect
  """
  
  ## Lifecycle Callbacks
  
  @doc """
  Mount callback - called when LiveView starts
  
  Params:
  - params: URL parameters from connect
  - session: session data from plug
  - socket: LiveView socket
  
  Returns:
  - {:ok, socket} - Normal mount
  - {:ok, socket, options} - Mount with options
  """
  @callback mount(params :: map(), session :: map(), socket :: Socket.t()) ::
              {:ok, Socket.t()} | {:ok, Socket.t(), keyword()}
  
  @doc """
  Render callback - converts state to HTML
  
  Uses HEEx templates for efficient diffing
  """
  @callback render(assigns :: map()) :: Phoenix.LiveView.Rendered.t()
  
  @doc """
  Handle event - processes user interactions
  
  Event types:
  - phx-click: Button clicks
  - phx-change: Form changes
  - phx-submit: Form submissions
  - custom events
  """
  @callback handle_event(event :: binary(), params :: map(), socket :: Socket.t()) ::
              {:noreply, Socket.t()} | {:reply, reply :: map(), Socket.t()}
  
  @doc """
  Handle info - processes async messages
  
  Used for:
  - PubSub messages
  - Process messages (send/2)
  - Timer events (:timer.send_interval)
  """
  @callback handle_info(msg :: term(), socket :: Socket.t()) ::
              {:noreply, Socket.t()}
  
  @doc """
  Terminate - cleanup on disconnect
  """
  @callback terminate(reason, socket :: Socket.t()) :: term()
            when reason: :normal | :shutdown | {:shutdown, term} | term
end
```

### Socket Structure

```elixir
# lib/phoenix_live_view/socket.ex

defmodule Phoenix.LiveView.Socket do
  @moduledoc """
  LiveView socket contains all state for a connected client
  """
  
  defstruct [
    # Assigns - user-defined state
    assigns: %{__changed__: %{}},
    
    # Endpoint module
    endpoint: nil,
    
    # ID for this LiveView
    id: nil,
    
    # View module
    view: nil,
    
    # Parent PID (for live components)
    parent_pid: nil,
    
    # Root PID (for nested LiveViews)
    root_pid: nil,
    
    # Private socket state
    private: %{
      changed: %{},
      fetch_live_info: false,
      upload_configs: %{},
      upload_names: [],
    },
    
    # Transport PID
    transport_pid: nil,
  ]
  
  @type t :: %__MODULE__{
    assigns: map(),
    endpoint: module(),
    id: binary() | nil,
    view: module(),
    parent_pid: pid() | nil,
    root_pid: pid(),
    private: map(),
    transport_pid: pid() | nil,
  }
end
```

## State Management Patterns

### Assign Operations

```elixir
# lib/phoenix_live_view.ex

defmodule Phoenix.LiveView do
  import Phoenix.Component
  
  ## Assign Operations
  
  @doc """
  Assign a single key-value pair
  
  Marks the key as changed for efficient diffing
  """
  def assign(socket, key, value) do
    new_assigns = Map.put(socket.assigns, key, value)
    new_changed = Map.put(socket.assigns.__changed__, key, true)
    
    %Phoenix.LiveView.Socket{
      socket |
      assigns: %{new_assigns | __changed__: new_changed}
    }
  end
  
  @doc """
  Assign multiple key-value pairs
  """
  def assign(socket, attrs) when is_map(attrs) or is_list(attrs) do
    Enum.reduce(attrs, socket, fn {key, val}, acc ->
      assign(acc, key, val)
    end)
  end
  
  @doc """
  Assign new key only if it doesn't exist
  
  Useful for one-time initialization in mount/3
  """
  def assign_new(socket, key, fun) when is_function(fun, 0) do
    case socket.assigns do
      %{^key => _} -> socket
      _ -> assign(socket, key, fun.())
    end
  end
  
  @doc """
  Update existing value with function
  
  Useful for incrementing counters, appending to lists
  """
  def update(socket, key, fun) when is_function(fun, 1) do
    case socket.assigns do
      %{^key => current} ->
        assign(socket, key, fun.(current))
      
      _ ->
        raise ArgumentError, "could not find key #{inspect(key)} in socket assigns"
    end
  end
  
  ## Example: Counter with multiple patterns
  
  defmodule CounterLive do
    use Phoenix.LiveView
    
    def mount(_params, _session, socket) do
      socket =
        socket
        |> assign_new(:count, fn -> 0 end)  # Initialize if not set
        |> assign(count: 0, last_updated: DateTime.utc_now())
      
      {:ok, socket}
    end
    
    def handle_event("increment", _params, socket) do
      # Update with function
      {:noreply, update(socket, :count, &(&1 + 1))}
    end
    
    def handle_event("reset", _params, socket) do
      # Direct assignment
      {:noreply, assign(socket, count: 0)}
    end
    
    def handle_info({:tick, time}, socket) do
      # Update multiple assigns
      {:noreply, assign(socket, last_updated: time)}
    end
  end
end
```

### State Optimization

```elixir
# lib/phoenix_live_view/diff.ex

defmodule Phoenix.LiveView.Diff do
  @moduledoc """
  Efficient HTML diffing based on changed assigns
  
  Only re-renders template sections where assigns changed
  """
  
  @doc """
  Render with diff tracking
  
  Tracks which assigns changed and only renders affected DOM
  """
  def render_with_diff(socket, view_module) do
    # Get changed keys
    changed = get_changed_keys(socket)
    
    # Render template with changed tracking
    rendered = view_module.render(socket.assigns)
    
    # Generate diff
    diff = generate_diff(rendered, changed)
    
    # Clear changed flags
    clear_changed(socket)
    
    {diff, rendered}
  end
  
  defp get_changed_keys(socket) do
    socket.assigns
    |> Map.get(:__changed__, %{})
    |> Map.keys()
  end
  
  defp generate_diff(rendered, changed) do
    # Generate minimal diff for transport
    %{
      d: rendered.html,  # Full HTML for initial render
      c: changed,        # Changed components
      p: rendered.parts  # Template parts for re-use
    }
  end
end
```

## Event Handling

### Client to Server Events

```elixir
# lib/phoenix_live_view/handler.ex

defmodule Phoenix.LiveView.EventHandler do
  @moduledoc """
  Handles events from browser
  
  Event flow:
  1. User clicks button with phx-click
  2. LiveView.js sends event over WebSocket
  3. Phoenix router dispatches to LiveView process
  4. handle_event/3 callback invoked
  5. Socket updated and re-rendered
  6. HTML diff sent to browser
  """
  
  @doc """
  Handle event from client
  """
  def handle_event(socket, event, params) do
    view_module = socket.view
    
    # Call handle_event callback
    case view_module.handle_event(event, params, socket) do
      {:noreply, new_socket} ->
        # Render and push diff
        push_diff(new_socket)
      
      {:reply, reply, new_socket} ->
        # Send reply and render
        push_reply(new_socket, reply)
        push_diff(new_socket)
    end
  end
  
  defp push_diff(socket) do
    rendered = socket.view.render(socket.assigns)
    diff = Diff.generate_diff(rendered, get_changed_keys(socket))
    
    send_update(socket.transport_pid, {:diff, diff})
  end
  
  defp push_reply(socket, reply) do
    send(socket.transport_pid, {:reply, reply})
  end
end
```

### Async Message Handling

```elixir
# Example: LiveView with async messages

defmodule MyAppWeb.DashboardLive do
  use Phoenix.LiveView
  
  def mount(_params, _session, socket) do
    # Subscribe to PubSub topic
    Phoenix.PubSub.subscribe(MyApp.PubSub, "dashboard:updates")
    
    # Start periodic update timer
    :timer.send_interval(5000, :update_stats)
    
    {:ok, assign(socket, stats: load_stats(), last_update: DateTime.utc_now())}
  end
  
  def render(assigns) do
    ~H"""
    <div>
      <h1>Dashboard</h1>
      <p>Users: {@stats.users}</p>
      <p>Orders: {@stats.orders}</p>
      <p>Last update: {@last_update}</p>
    </div>
    """
  end
  
  # Handle timer message
  def handle_info(:update_stats, socket) do
    {:noreply, assign(socket, stats: load_stats(), last_update: DateTime.utc_now())}
  end
  
  # Handle PubSub broadcast
  def handle_info({:dashboard_updated, data}, socket) do
    {:noreply, assign(socket, stats: Map.merge(socket.assigns.stats, data))}
  end
  
  defp load_stats do
    %{
      users: MyApp.User.count(),
      orders: MyApp.Order.count()
    }
  end
end
```

## Process Management

### Supervisor Integration

```elixir
# lib/my_app/application.ex

defmodule MyApp.Application do
  use Application
  
  def start(_type, _args) do
    children = [
      # Phoenix endpoint
      MyAppWeb.Endpoint,
      
      # PubSub
      {Phoenix.PubSub, name: MyApp.PubSub},
      
      # Ecto repo
      MyApp.Repo,
      
      # Custom supervisor for background tasks
      MyApp.BackgroundSupervisor,
    ]
    
    opts = [strategy: :one_for_one, name: MyApp.Supervisor]
    Supervisor.start_link(children, opts)
  end
end
```

### Rate Limiting

```elixir
# lib/phoenix_live_view/throttle.ex

defmodule Phoenix.LiveView.Throttle do
  @moduledoc """
  Rate limit events from client
  """
  
  use GenServer
  
  def start_link(opts) do
    GenServer.start_link(__MODULE__, opts, name: __MODULE__)
  end
  
  def init(opts) do
    {:ok, %{
      limits: Keyword.get(opts, :limits, %{}),
      counters: %{}
    }}
  end
  
  def check_limit(event_type, socket) do
    GenServer.call(__MODULE__, {:check, event_type, socket})
  end
  
  def handle_call({:check, event_type, socket}, _from, state) do
    key = {event_type, socket.id}
    current = Map.get(state.counters, key, 0)
    limit = Map.get(state.limits, event_type, :infinity)
    
    if limit == :infinity or current < limit do
      new_counters = Map.update(state.counters, key, 1, &(&1 + 1))
      {:reply, :ok, %{state | counters: new_counters}}
    else
      {:reply, :rate_limited, state}
    end
  end
  
  # Reset counter after timeout
  def reset(event_type, socket_id) do
    GenServer.cast(__MODULE__, {:reset, event_type, socket_id})
  end
  
  def handle_cast({:reset, event_type, socket_id}, state) do
    key = {event_type, socket_id}
    new_counters = Map.delete(state.counters, key)
    {:noreply, %{state | counters: new_counters}}
  end
end
```

## Conclusion

LiveView state management provides:

1. **Process Isolation**: One BEAM process per client
2. **Efficient Diffing**: Only changed assigns trigger re-render
3. **Event Handling**: Synchronous events, async messages
4. **PubSub Integration**: Real-time broadcasting
5. **Lifecycle Hooks**: mount, render, handle_event, handle_info, terminate
