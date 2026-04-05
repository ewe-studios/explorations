---
source: /home/darkvoid/Boxxed/@formulas/src.pheonixLiveView/phoenix
repository: github.com:phoenixframework/phoenix
explored_at: 2026-04-05
focus: PubSub broadcast, topic subscription, presence, distributed messaging
---

# Deep Dive: PubSub and Real-Time Broadcasting

## Overview

This deep dive examines Phoenix PubSub - the publish/subscribe system that powers LiveView's real-time capabilities. We explore topic subscription, broadcast mechanisms, Presence for tracking online users, and distributed messaging patterns.

## Architecture

```mermaid
flowchart TB
    subgraph LiveView1["LiveView Process 1"]
        LV1[LiveView] --> Subscribe1[Phoenix.PubSub.subscribe]
    end
    
    subgraph LiveView2["LiveView Process 2"]
        LV2[LiveView] --> Subscribe2[Phoenix.PubSub.subscribe]
    end
    
    subgraph LiveView3["LiveView Process 3"]
        LV3[LiveView] --> Subscribe3[Phoenix.PubSub.subscribe]
    end
    
    Subscribe1 --> Topic[Topic: "room:lobby"]
    Subscribe2 --> Topic
    Subscribe3 --> Topic
    
    Broadcast[Phoenix.PubSub.broadcast] --> Topic
    Topic --> Deliver1[Deliver to LV1]
    Topic --> Deliver2[Deliver to LV2]
    Topic --> Deliver3[Deliver to LV3]
    
    subgraph PubSub System
        Topic
        Broadcast
    end
    
    subgraph Adapters
        Local[Local Node]
        Distributed[PG2/Redis]
    end
    
    Topic --> Local
    Topic --> Distributed
```

## PubSub Basics

### Subscription and Broadcast

```elixir
# lib/my_app_web/live/chat_live.ex

defmodule MyAppWeb.ChatLive do
  use Phoenix.LiveView
  
  def mount(_params, _session, socket) do
    # Subscribe to room updates
    room_id = socket.assigns.room_id
    
    # Subscribe to topic
    Phoenix.PubSub.subscribe(MyApp.PubSub, "room:#{room_id}")
    
    # Subscribe to user presence
    Phoenix.PubSub.subscribe(MyApp.PubSub, "room:#{room_id}:presence")
    
    {:ok, socket}
  end
  
  def render(assigns) do
    ~H"""
    <div class="chat-room">
      <h1>Room {@room_id}</h1>
      
      <div class="messages">
        <%= for message <- @messages do %>
          <div class="message">
            <strong>{message.user}</strong>
            <p>{message.body}</p>
          </div>
        <% end %>
      </div>
      
      <div class="online-users">
        <h3>Online Users</h3>
        <%= for user <- @online_users do %>
          <span class="user">{user.name}</span>
        <% end %>
      </div>
      
      <form phx-submit="send_message">
        <input name="body" placeholder="Type a message..." />
        <button type="submit">Send</button>
      </form>
    </div>
    """
  end
  
  # Handle form submission
  def handle_event("send_message", %{"body" => body}, socket) do
    message = %{
      id: UUID.uuid4(),
      user: socket.assigns.current_user.name,
      body: body,
      timestamp: DateTime.utc_now()
    }
    
    # Save to database
    MyApp.Chat.create_message(socket.assigns.room_id, message)
    
    # Broadcast to all subscribers
    Phoenix.PubSub.broadcast(
      MyApp.PubSub,
      "room:#{socket.assigns.room_id}",
      {:new_message, message}
    )
    
    {:noreply, socket}
  end
  
  # Handle broadcast message
  def handle_info({:new_message, message}, socket) do
    # Append message to list
    {:noreply, update(socket, :messages, &(&1 ++ [message]))}
  end
  
  # Handle presence updates
  def handle_info({:presence_update, joins, leaves}, socket) do
    online_users = update_presence(socket.assigns.online_users, joins, leaves)
    {:noreply, assign(socket, online_users: online_users)}
  end
end
```

### PubSub Implementation

```elixir
# lib/phoenix/pubsub.ex

defmodule Phoenix.PubSub do
  @moduledoc """
  Distributed PubSub system
  
  Supports multiple adapters:
  - Phoenix.PubSub (local, uses PG2)
  - Phoenix.PubSub.Redis (distributed)
  - Phoenix.PubSub.PG (distributed, Erlang PG)
  """
  
  @doc """
  Subscribe to a topic
  """
  def subscribe(pubsub, topic, opts \\ []) do
    adapter(pubsub).subscribe(node_name(pubsub), topic, opts)
  end
  
  @doc """
  Unsubscribe from a topic
  """
  def unsubscribe(pubsub, topic) do
    adapter(pubsub).unsubscribe(node_name(pubsub), topic)
  end
  
  @doc """
  Broadcast message to topic subscribers
  """
  def broadcast(pubsub, topic, message, opts \\ []) do
    adapter(pubsub).broadcast(node_name(pubsub), topic, message, opts)
  end
  
  @doc """
  Broadcast to local subscribers only
  """
  def broadcast_local(pubsub, topic, message, opts \\ []) do
    adapter(pubsub).broadcast_local(node_name(pubsub), topic, message, opts)
  end
  
  @doc """
  Broadcast from current node to remote nodes
  """
  def broadcast_remote(pubsub, topic, message, opts \\ []) do
    adapter(pubsub).broadcast_remote(node_name(pubsub), topic, message, opts)
  end
  
  defp adapter(module) do
    # Get configured adapter
    Application.get_env(:phoenix, :pubsub_adapter, Phoenix.PubSub)
  end
  
  defp node_name(module) do
    Application.get_env(:phoenix, :pubsub_node_name, to_string(node()))
  end
end
```

## Presence System

### Presence Tracking

```elixir
# lib/my_app_web/channels/presence.ex

defmodule MyAppWeb.Presence do
  @moduledoc """
  Presence tracking for LiveView
  
  Tracks which users are online in which rooms
  """
  
  use Phoenix.Presence,
    otp_app: :my_app,
    pubsub_server: MyApp.PubSub
  
  @doc """
  Track user in room
  """
  def track_user(room_id, user) do
    track(self(), "room:#{room_id}", user.id, %{
      name: user.name,
      online_at: inspect(System.system_time(:second)),
      meta: %{
        avatar_url: user.avatar_url,
        status: "online"
      }
    })
  end
  
  @doc """
  Fetch presence for room
  """
  def get_room_presence(room_id) do
    list("room:#{room_id}")
  end
  
  @doc """
  Handle presence diff (joins/leaves)
  """
  def fetch_diff(topic, joins, leaves) do
    # Convert to format for LiveView
    join_list = extract_users(joins)
    leave_list = extract_users(leaves)
    
    # Broadcast to subscribers
    Phoenix.PubSub.broadcast(
      MyApp.PubSub,
      "#{topic}:presence",
      {:presence_update, join_list, leave_list}
    )
  end
  
  defp extract_users(presence_map) do
    presence_map
    |> Enum.flat_map(fn {_, presence} ->
      Enum.map(presence.metas, fn meta ->
        %{id: presence.key, name: meta.name, avatar_url: meta.avatar_url}
      end)
    end)
  end
end
```

### Using Presence in LiveView

```elixir
# lib/my_app_web/live/room_live.ex

defmodule MyAppWeb.RoomLive do
  use Phoenix.LiveView
  
  alias MyAppWeb.Presence
  
  def mount(_params, _session, socket) do
    room_id = socket.assigns.room_id
    user = socket.assigns.current_user
    
    # Subscribe to presence updates
    Phoenix.PubSub.subscribe(MyApp.PubSub, "room:#{room_id}:presence")
    
    # Track this user
    Presence.track_user(room_id, user)
    
    # Get initial presence
    online_users = get_online_users(room_id)
    
    {:ok, assign(socket, online_users: online_users)}
  end
  
  def render(assigns) do
    ~H"""
    <div class="room">
      <div class="user-list">
        <h3>Online (<%= length(@online_users) %>)</h3>
        <%= for user <- @online_users do %>
          <div class="user">
            <img src={user.avatar_url} alt={user.name} class="avatar" />
            <span>{user.name}</span>
            <span class="status-dot"></span>
          </div>
        <% end %>
      </div>
    </div>
    """
  end
  
  def handle_info({:presence_update, joins, leaves}, socket) do
    # Update online users
    online_users = 
      socket.assigns.online_users
      |> add_users(joins)
      |> remove_users(leaves)
    
    {:noreply, assign(socket, online_users: online_users)}
  end
  
  def handle_info({:presence_diff, joins, leaves}, socket) do
    # Handle presence diff from Presence module
    online_users = 
      socket.assigns.online_users
      |> add_users(extract_users(joins))
      |> remove_users(extract_users(leaves))
    
    {:noreply, assign(socket, online_users: online_users)}
  end
  
  defp add_users(users, new_users) do
    Enum.reduce(new_users, users, fn user, acc ->
      if Enum.any?(acc, &(&1.id == user.id)) do
        acc
      else
        [user | acc]
      end
    end)
  end
  
  defp remove_users(users, left_users) do
    Enum.reject(users, fn user ->
      Enum.any?(left_users, &(&1.id == user.id))
    end)
  end
  
  defp get_online_users(room_id) do
    Presence.get_room_presence(room_id)
    |> extract_users()
  end
  
  defp extract_users(presence_list) do
    Enum.flat_map(presence_list, fn presence ->
      Enum.map(presence.metas, fn meta ->
        %{
          id: presence.key,
          name: meta.name,
          avatar_url: meta.avatar_url,
          online_at: meta.online_at
        }
      end)
    end)
  end
end
```

## Distributed PubSub

### Redis Adapter Configuration

```elixir
# config/config.exs

config :my_app, MyApp.PubSub,
  adapter: Phoenix.PubSub.Redis,
  redis_url: "redis://localhost:6379",
  redis_opts: [
    socket_opts: [reuseaddr: true],
    max_reconnect_attempts: 10,
    reconnect_sleep: :random
  ]

# For multi-node clusters
config :my_app, MyAppWeb.Endpoint,
  server: true,
  pubsub_server: MyApp.PubSub,
  pubsub_node_name: "#{System.get_env("NODE_NAME")}@#{System.get_env("POD_IP")}"
```

### PG Adapter (Built-in)

```elixir
# config/config.exs

# Phoenix 1.7+ uses PG adapter by default
config :my_app, MyApp.PubSub,
  adapter: Phoenix.PubSub.PG

# No external dependencies needed
# Uses Erlang's built-in pg module for distributed pubsub

# For cluster discovery:
config :libcluster,
  topologies: [
    k8s_example: [
      strategy: Cluster.Strategy.Kubernetes.DNS,
      config: [
        service: "my-app",
        application_name: "my_app",
        node_basename: "my_app"
      ]
    ]
  ]
```

## Advanced Patterns

### Rate-Limited Broadcasts

```elixir
# lib/my_app_web/live/throttled_broadcast.ex

defmodule MyAppWeb.ThrottledBroadcast do
  @moduledoc """
  Rate-limited broadcasts for high-frequency updates
  """
  
  use GenServer
  
  def start_link(opts) do
    GenServer.start_link(__MODULE__, opts, name: __MODULE__)
  end
  
  def init(opts) do
    {:ok, %{
      buffers: %{},
      timers: %{},
      interval: Keyword.get(opts, :interval, 100)
    }}
  end
  
  @doc """
  Queue broadcast with throttling
  """
  def broadcast(topic, message) do
    GenServer.cast(__MODULE__, {:broadcast, topic, message})
  end
  
  def handle_cast({:broadcast, topic, message}, state) do
    # Add to buffer
    buffers = Map.update(state.buffers, topic, [message], &[message | &1])
    
    # Start timer if not already running
    timers = case state.timers do
      %{^topic => _} -> state.timers
      _ ->
        timer = Process.send_after(self(), {:flush, topic}, state.interval)
        Map.put(state.timers, topic, timer)
    end
    
    {:noreply, %{state | buffers: buffers, timers: timers}}
  end
  
  def handle_info({:flush, topic}, state) do
    # Get buffered messages
    messages = Map.get(state.buffers, topic, [])
    
    # Broadcast merged update
    merged = merge_messages(messages)
    Phoenix.PubSub.broadcast(MyApp.PubSub, topic, merged)
    
    # Clear buffer and timer
    buffers = Map.delete(state.buffers, topic)
    timers = Map.delete(state.timers, topic)
    
    {:noreply, %{state | buffers: buffers, timers: timers}}
  end
  
  defp merge_messages(messages) do
    # Merge multiple messages into single update
    # Implementation depends on message type
    {:batch_update, Enum.reverse(messages)}
  end
end
```

### Conclusion

Phoenix PubSub provides:

1. **Local and Distributed**: Works single-node or multi-node
2. **Presence Tracking**: Know who's online where
3. **Fault Tolerant**: Built on OTP supervision
4. **Scalable**: Redis adapter for large deployments
5. **Simple API**: subscribe/2, broadcast/3
