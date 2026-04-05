---
source: /home/darkvoid/Boxxed/@formulas/src.pheonixLiveView/phoenix
repository: github.com:phoenixframework/phoenix
explored_at: 2026-04-05
focus: HEEx templating engine, template compilation, change tracking, efficient rendering
---

# Deep Dive: HEEx Templating Engine

## Overview

This deep dive examines Phoenix's HEEx (HTML+EEx) templating engine - how templates are compiled, how change tracking works, and how efficient HTML diffs are generated for LiveView updates.

## Architecture

```mermaid
flowchart TB
    subgraph Template Source
        HEEx[.heex file] --> Parser
    end
    
    subgraph Compilation
        Parser --> AST[Template AST]
        AST --> Optimizer[Tree Optimizer]
        Optimizer --> CodeGen[Code Generator]
        CodeGen -> BEAM[BEAM Bytecode]
    end
    
    subgraph Runtime
        BEAM --> Assigns[Socket Assigns]
        Assigns --> Render[Render Function]
        Render --> Changed[Change Tracker]
        Changed --> Diff[HTML Diff]
    end
    
    subgraph Output
        Diff --> Push[Push to Client]
    end
```

## HEEx Template Syntax

### Basic Template Structure

```elixir
# lib/my_app_web/live/user_live.ex

defmodule MyAppWeb.UserLive do
  use Phoenix.LiveView
  
  def render(assigns) do
    ~H"""
    <div class="user-list">
      <h1>Users</h1>
      
      <%= for user <- @users do %>
        <div class={"user", active: user.active}>
          <img src={user.avatar_url} alt={user.name} />
          <h2>{user.name}</h2>
          <p>{user.email}</p>
          
          <%= if user.active do %>
            <span class="status">Online</span>
          <% else %>
            <span class="status">Offline</span>
          <% end %>
          
          <button phx-click="select_user" phx-value-id={user.id}>
            Select
          </button>
        </div>
      <% end %>
    </div>
    """
  end
end
```

### Template Compilation

```elixir
# lib/phoenix_live_view/engine.ex

defmodule Phoenix.LiveView.Engine do
  @moduledoc """
  HEEx template compiler
  
  Compiles .heex templates into efficient render functions
  with change tracking support
  """
  
  @doc """
  Compile HEEx template to render function
  """
  def compile(template_string, module) do
    # Parse HTML/EEx
    ast = parse_heex(template_string)
    
    # Optimize AST
    optimized = optimize_ast(ast)
    
    # Generate render function
    quote do
      def render(var!(assigns)) do
        unquote(generate_code(optimized))
      end
    end
  end
  
  defp parse_heex(template) do
    # Tokenize
    tokens = tokenize(template)
    
    # Parse to AST
    tokens |> to_ast()
  end
  
  defp tokenize(template) do
    # HTML tags
    # EEx expressions: <%= ... %>
    # Interpolations: {...}
    # Text content
    
    ~r/(<%=?.*?%>)|(\{.*?\})|([^<{}]+)/s
    |> Regex.scan(template)
    |> Enum.map(&extract_token/1)
  end
  
  defp to_ast(tokens) do
    tokens
    |> Enum.map(&token_to_node/1)
    |> build_tree()
  end
end
```

### Generated Code Example

```elixir
# What HEEx compiles to:

defmodule MyAppWeb.UserLive do
  use Phoenix.LiveView
  
  # Original template:
  # ~H"""
  # <div class="user-list">
  #   <h1>{@title}</h1>
  #   <%= for user <- @users do %>
  #     <div class={"user", active: user.active}>{user.name}</div>
  #   <% end %>
  # </div>
  # """
  
  # Compiled render function:
  def render(assigns) do
    import Phoenix.Component
    
    # Track changed keys
    changed = Map.get(assigns, :__changed__, %{})
    
    # Build IO list (efficient concatenation)
    [
      "<div class=\"user-list\">",
      render_title(assigns, changed),
      render_users(assigns, changed),
      "</div>"
    ]
  end
  
  # Static content - no re-render needed
  defp render_title(assigns, %{:title => true}) do
    ["<h1>", to_string(assigns.title), "</h1>"]
  end
  
  defp render_title(assigns, _) do
    # Return cached/static version
    static_title()
  end
  
  # Dynamic comprehension
  defp render_users(assigns, changed) do
    Enum.map(assigns.users, fn user ->
      user_assigns = Map.put(assigns, :user, user)
      render_user_div(user_assigns, changed)
    end)
  end
  
  defp render_user_div(assigns, _changed) do
    class = dynamic_class(["user", active: assigns.user.active])
    
    [
      "<div class=\"", class, "\">",
      to_string(assigns.user.name),
      "</div>"
    ]
  end
end
```

## Change Tracking

### Changed Assigns System

```elixir
# lib/phoenix/component.ex

defmodule Phoenix.Component do
  @moduledoc """
  Change tracking for efficient re-rendering
  """
  
  @doc """
  Mark assigns as changed
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
  Check if key changed
  """
  def changed?(assigns, key) do
    case assigns do
      %{__changed__: %{^key => true}} -> true
      %{__changed__: %{}} -> false
      _ -> true  # Initial render
    end
  end
end
```

### Conditional Rendering Optimization

```elixir
# lib/phoenix_live_view/diff.ex

defmodule Phoenix.LiveView.Diff do
  @moduledoc """
  Template diffing based on changed assigns
  """
  
  @doc """
  Render only changed parts of template
  """
  def render_with_diff(socket, view_module) do
    assigns = socket.assigns
    changed = Map.get(assigns, :__changed__, %{})
    
    # Call render with change tracking
    rendered = view_module.render(assigns)
    
    # Generate minimal diff
    diff = compute_diff(rendered, changed)
    
    {diff, rendered}
  end
  
  defp compute_diff(rendered, changed) do
    # For initial render, send full HTML
    if map_size(changed) == 0 do
      %{d: rendered.html}  # d = document (full HTML)
    else
      # For updates, send only changed parts
      %{
        c: extract_components(rendered, changed),
        p: extract_parts(rendered)
      }
    end
  end
end
```

## Components

### Function Components

```elixir
# lib/my_app_web/components/core_components.ex

defmodule MyAppWeb.CoreComponents do
  @moduledoc """
  Reusable function components
  """
  
  use Phoenix.Component
  
  attr :class, :string, default: ""
  attr :rest, :global, include: ~w(id title aria-label)
  slot :inner_block, required: true
  
  def card(assigns) do
    ~H"""
    <div class={"card #{@class}"} {@rest}>
      <div class="card-content">
        <%= render_slot(@inner_block) %>
      </div>
    </div>
    """
  end
  
  attr :user, :map, required: true
  attr :size, :string, default: "medium", values: ~w(small medium large)
  
  def avatar(assigns) do
    ~H"""
    <img 
      src={@user.avatar_url} 
      alt={@user.name}
      class={"avatar avatar-#{@size}"}
      data-user-id={@user.id}
    />
    """
  end
  
  attr :form, Phoenix.HTML.Form, required: true
  attr :field, Phoenix.HTML.FormField, required: true
  attr :label, :string, required: true
  attr :type, :string, default: "text"
  
  def input(assigns) do
    ~H"""
    <div class="form-group">
      <label for={@field.id}>{@label}</label>
      <input
        type={@type}
        id={@field.id}
        name={@field.name}
        value={Phoenix.HTML.Form.input_value(@form, @field)}
        class={if @field.errors != [], do: "error", else: ""}
      />
      <%= for error <- @field.errors do %>
        <span class="error-message">{error}</span>
      <% end %>
    </div>
    """
  end
  
  # Component with effects
  attr :items, :list, required: true
  attr :on_select, :any, required: true
  
  def select_list(assigns) do
    ~H"""
    <ul class="select-list">
      <%= for item <- @items do %>
        <li 
          phx-click={@on_select}
          phx-value-id={item.id}
          class="select-item"
        >
          {item.name}
        </li>
      <% end %>
    </ul>
    """
  end
end
```

### Using Components

```elixir
# In LiveView template:

defmodule MyAppWeb.UserLive do
  use Phoenix.LiveView
  import MyAppWeb.CoreComponents
  
  def render(assigns) do
    ~H"""
    <div class="user-page">
      <card class="profile-card">
        <h1>User Profile</h1>
        
        <div class="profile-header">
          <avatar user={@current_user} size="large" />
          <h2>{@current_user.name}</h2>
        </div>
        
        <form phx-change="validate" phx-submit="save">
          <input 
            form={@form}
            field={@form[:name]}
            label="Name"
          />
          
          <input 
            form={@form}
            field={@form[:email]}
            label="Email"
            type="email"
          />
          
          <button type="submit">Save</button>
        </form>
      </card>
      
      <card class="activity-card">
        <h3>Recent Activity</h3>
        <select_list 
          items={@activities}
          on_select="view_activity"
        />
      </card>
    </div>
    """
  end
end
```

## Slots and Render Slots

### Slot Types

```elixir
# lib/my_app_web/components/slots.ex

defmodule MyAppWeb.SlotComponents do
  use Phoenix.Component
  
  # Basic slot
  attr :rest, :global
  slot :inner_block
  
  def wrapper(assigns) do
    ~H"""
    <div class="wrapper" {@rest}>
      <%= render_slot(@inner_block) %>
    </div>
    """
  end
  
  # Named slots
  attr :title, :string, required: true
  slot :header
  slot :inner_block, required: true
  slot :footer
  
  def modal(assigns) do
    ~H"""
    <div class="modal-overlay">
      <div class="modal">
        <div class="modal-header">
          <%= render_slot(@header) %>
          <h2>{@title}</h2>
        </div>
        
        <div class="modal-body">
          <%= render_slot(@inner_block) %>
        </div>
        
        <div class="modal-footer">
          <%= render_slot(@footer) %>
        </div>
      </div>
    </div>
    """
  end
  
  # Slot with props
  attr :items, :list, required: true
  slot :item, required: true do
    attr :item, :map, required: true
    attr :index, :integer
  end
  
  def item_list(assigns) do
    ~H"""
    <ul class="item-list">
      <%= for {item, index} <- Enum.with_index(@items) do %>
        <%= render_slot(@item, item: item, index: index) %>
      <% end %>
    </ul>
    """
  end
end

# Usage:
# <modal title="Confirm">
#   <:header>
#     <icon name="warning" />
#   </:header>
#   
#   Are you sure?
#   
#   <:footer>
#     <button phx-click="confirm">Yes</button>
#     <button phx-click="cancel">No</button>
#   </:footer>
# </modal>
#
# <item_list items={@users}>
#   <:item :let={item: user, index: i}>
#     <li class={"user-#{i}"}>
#       {user.name} - {user.email}
#     </li>
#   </:item>
# </item_list>
```

## Conclusion

HEEx templating provides:

1. **Compile-Time Optimization**: Templates compiled to efficient BEAM code
2. **Change Tracking**: Only re-render changed assigns
3. **Function Components**: Reusable template functions
4. **Slots**: Flexible content composition
5. **HTML Safety**: Automatic escaping, XSS protection
