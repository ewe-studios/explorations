# Ash Core Deep Dive

## Overview

This deep dive covers the core components of Ash Framework:
- **ash**: The main framework
- **ash_events**: Event publishing system
- **igniter**: Code generation and scaffolding
- **spark**: DSL framework

---

## Part 1: Ash Core (`ash` v3.5.25)

### Architecture Overview

Ash core is built around several key abstractions that work together to provide a declarative, extensible framework:

```
┌─────────────────────────────────────────────────────────────┐
│                      Ash Domain                            │
│  ┌─────────────────────────────────────────────────────┐   │
│  │                   Resources                          │   │
│  │  ┌──────────┐  ┌──────────┐  ┌──────────────────┐  │   │
│  │  │Attributes│  │Actions   │  │Relationships     │  │   │
│  │  └──────────┘  └──────────┘  └──────────────────┘  │   │
│  │  ┌──────────┐  ┌──────────┐  ┌──────────────────┐  │   │
│  │  │Validations│ │Changes   │  │Calculations      │  │   │
│  │  └──────────┘  └──────────┘  └──────────────────┘  │   │
│  └─────────────────────────────────────────────────────┘   │
│                            │                                 │
│                    ┌───────▼───────┐                        │
│                    │  Data Layer   │                        │
│                    │  (Behavior)   │                        │
│                    └───────┬───────┘                        │
└────────────────────────────┼────────────────────────────────┘
                             │
         ┌───────────────────┼───────────────────┐
         │                   │                   │
  ┌──────▼──────┐    ┌──────▼──────┐    ┌──────▼──────┐
  │  Postgres   │    │   Sqlite    │    │    ETS      │
  └─────────────┘    └─────────────┘    └─────────────┘
```

### Key Modules

#### 1. Ash.Resource

The fundamental building block. A Resource is defined using Spark DSL:

```elixir
defmodule MyApp.Accounts.User do
  use Ash.Resource,
    domain: MyApp.Accounts,
    data_layer: AshPostgres.DataLayer

  # DSL sections:
  attributes do
    uuid_primary_key :id
    attribute :email, :ci_string, allow_nil?: false, public?: true
    attribute :role, :atom, constraints: [one_of: [:admin, :user]], default: :user
    timestamps()
  end

  relationships do
    has_one :profile, MyApp.Accounts.Profile
    has_many :posts, MyApp.Blog.Post
  end

  actions do
    defaults [:read, :destroy]

    create :register do
      accept [:email, :password]
      change hash_password(:password)
    end

    update :promote_to_admin do
      change set_attribute(:role, :admin)
    end
  end

  validations do
    validate present(:email)
    validate unique(:email)
  end
end
```

**Key Implementation Details:**

- Resources are **Elixir modules** that `use Ash.Resource`
- Under the hood, they use `Spark.Dsl` for the declarative syntax
- Resources compile into structs with known fields
- The `@persist` attribute stores DSL configuration for runtime introspection

#### 2. Ash.Domain

Domains group related resources and provide the API surface:

```elixir
defmodule MyApp.Accounts do
  use Ash.Domain

  resources do
    resource MyApp.Accounts.User do
      define :register, args: [:email, :password]
      define :get_user, args: [:id]
    end
  end
end

# Usage
{:ok, user} = MyApp.Accounts.register("user@example.com", "password")
```

**Domain Responsibilities:**
- Resource registry
- Transaction boundaries
- Shared configuration
- Code interface generation

#### 3. Action System

Actions are the core unit of work in Ash. Every action follows this lifecycle:

```
┌─────────────┐     ┌─────────────┐     ┌─────────────┐
│   Input     │────▶│  Changeset  │────▶│  Validate   │
│  Validation │     │   Build     │     │             │
└─────────────┘     └─────────────┘     └─────────────┘
                                               │
         ┌─────────────────────────────────────┘
         ▼
┌─────────────┐     ┌─────────────┐     ┌─────────────┐
│   Notify    │◀────│   Commit    │◀────│   Run       │
│             │     │             │     │  Changes    │
└─────────────┘     └─────────────┘     └─────────────┘
```

**Action Types:**

| Type | Description | Transaction |
|------|-------------|-------------|
| `read` | Query resources | No (default) |
| `create` | Create new records | Yes |
| `update` | Modify existing records | Yes |
| `destroy` | Remove records | Yes |
| `action` | Generic operations | Configurable |

**Action Implementation (from `lib/ash/actions/action.ex`):**

```elixir
def run(domain, input, opts) do
  # 1. Set context and extract options
  {input, opts} = Ash.Actions.Helpers.set_context_and_get_opts(domain, input, opts)

  # 2. Build context for the action
  context = %Ash.Resource.Actions.Implementation.Context{
    actor: opts[:actor],
    tenant: opts[:tenant],
    authorize?: opts[:authorize?]
  }

  # 3. Run in transaction if configured
  if input.action.transaction? do
    Ash.DataLayer.transaction(resources, fn ->
      # Authorize
      case authorize(domain, opts[:actor], input) do
        :ok -> call_run_function(module, input, run_opts, context, true)
        {:error, error} -> Ash.DataLayer.rollback(resources, error)
      end
    end)
  else
    # Run without transaction
    case authorize(domain, opts[:actor], input) do
      :ok -> call_run_function(module, input, run_opts, context, false)
      {:error, error} -> {:error, error}
    end
  end
end
```

#### 4. Changeset System

Changesets are the intermediate representation between input and persisted data:

```elixir
defstruct [
  :action_type,      # :create, :update, :destroy, :read
  :action,           # The action struct
  :resource,         # The resource module
  :data,             # Existing data (for updates)
  :attributes,       # Raw attributes
  :casted_attributes,# Type-casted attributes
  :arguments,        # Action arguments
  :relationships,    # Relationship changes
  :validations,      # Validation results
  :errors,           # Accumulated errors
  :context,          # Free-form context map
  :valid?            # Overall validity flag
]
```

**Changeset Lifecycle Hooks:**

```elixir
def change(changeset, _opts, _context) do
  changeset
  |> Ash.Changeset.before_transaction(fn changeset ->
    # Before transaction starts
    changeset
  end)
  |> Ash.Changeset.around_transaction(fn changeset, callback ->
    # Around transaction
    callback.(changeset)
  end)
  |> Ash.Changeset.before_action(fn changeset ->
    # Before data layer call
    changeset
  end)
  |> Ash.Changeset.after_action(fn changeset, result ->
    # After successful data layer call
    {:ok, result}
  end)
  |> Ash.Changeset.after_transaction(fn changeset, result ->
    # After transaction completes (success or error)
    result
  end)
end
```

#### 5. Query System

Ash queries are composable, type-safe query builders:

```elixir
# Basic query
Ash.Query.filter(Post, author_id == ^author_id and status == :published)

# Complex query with joins
Ash.Query.filter(Post, author.name == "John")
|> Ash.Query.load(:comments)
|> Ash.Query.sort(published_at: :desc)
|> Ash.Query.limit(10)
|> Ash.Query.offset(20)

# With calculations
Ash.Query.select(Post, [:id, :title, full_name: fragment("?", author.first_name <> " " <> author.last_name)])
```

**Query Pipeline:**

```
┌─────────────┐     ┌─────────────┐     ┌─────────────┐
│    Parse    │────▶│  Validate   │────▶│   Compile   │
│  Expression │     │   Filters   │     │   to SQL    │
└─────────────┘     └─────────────┘     └─────────────┘
                                               │
         ┌─────────────────────────────────────┘
         ▼
┌─────────────┐     ┌─────────────┐
│   Execute   │◀────│   Optimize  │
│  in Data    │     │   Query     │
│   Layer     │     │             │
└─────────────┘     └─────────────┘
```

### Data Layer Interface

The data layer behavior (`Ash.DataLayer`) defines the contract for persistence:

```elixir
@callback resource_to_query(Ash.Resource.t(), Ash.Domain.t()) :: data_layer_query()
@callback run_query(data_layer_query(), Ash.Resource.t()) :: {:ok, list} | {:error, term}
@callback filter(data_layer_query(), Ash.Filter.t(), resource :: Ash.Resource.t()) :: {:ok, query} | {:error, term}
@callback sort(data_layer_query(), Ash.Sort.t(), resource :: Ash.Resource.t()) :: {:ok, query} | {:error, term}
@callback limit(data_layer_query(), limit :: integer(), resource :: Ash.Resource.t()) :: {:ok, query} | {:error, term}
@callback create(data_layer_query(), changes :: map(), create_opts :: map()) :: {:ok, struct} | {:error, term}
@callback update(data_layer_query(), record :: struct(), changes :: map(), update_opts :: map()) :: {:ok, struct} | {:error, term}
@callback destroy(data_layer_query(), record :: struct(), destroy_opts :: map()) :: :ok | {:error, term}
```

---

## Part 2: Ash Events (`ash_events`)

### Overview

Ash Events provides pub/sub capabilities for Ash resources, allowing you to publish and consume events based on resource changes.

### Key Concepts

1. **Event Publications**: Define what events to publish when actions succeed
2. **Subscriptions**: Subscribe to events using patterns
3. **Handlers**: Process events asynchronously

### Example Configuration

```elixir
defmodule MyApp.Accounts.User do
  use Ash.Resource,
    domain: MyApp.Accounts,
    data_layer: AshPostgres.DataLayer,
    extensions: [AshEvents.Resource]

  events do
    publish :user_registered, :create do
      # Publish after successful create action
      entity fn changeset, _ ->
        %{
          user_id: changeset.data.id,
          email: changeset.attributes.email
        }
      end
    end
  end
end
```

---

## Part 3: Igniter (`igniter` v0.6+)

### Overview

Igniter is Ash's code generation and project scaffolding system. It provides utilities for:
- Generating new resources
- Creating migrations
- Bootstrapping projects
- Adding dependencies

### Key Features

**1. Project Generators:**

```elixir
Mix.Tasks.Ash.New.main(["my_app", "--postgres"])
```

**2. Resource Generators:**

```elixir
Mix.Tasks.Ash.Generate.Resource.main(["User", "--domain", "Accounts"])
```

**3. Migration Generators:**

```elixir
Mix.Tasks.Ash.Postgres.GenerateMigrations.main(["--auto-name"])
```

### Architecture

Igniter uses a composable pipeline approach:

```elixir
defmodule Igniter.Pipeline do
  defstruct [
    :project,      # Project configuration
    :files,        # Files to create/modify
    :dependencies, # Dependencies to add
    :steps         # Pipeline steps
  ]
end
```

---

## Part 4: Spark (`spark` v2.2+)

### Overview

Spark is the DSL (Domain Specific Language) framework that powers Ash. It provides:
- DSL definition utilities
- Extension system
- Configuration validation
- Compile-time verification

### DSL Definition

```elixir
defmodule MyApp.Dsl do
  use Spark.Dsl,
    default_extensions: [MyApp.Dsl.Extension],
    sections: [
      resources: [
        entities: [
          resource: [
            name: :resource,
            schema: [
              module: [type: :module, required: true],
              name: [type: :atom]
            ]
          ]
        ]
      ]
    ]
end
```

### Key Components

#### 1. Extension System

Extensions allow runtime modification of DSL behavior:

```elixir
defmodule AshGraphql.Resource do
  use Spark.Dsl.Extension

  def transform(dsl_state) do
    # Modify the resource DSL for GraphQL support
    # Add GraphQL-specific configuration
    # Validate GraphQL compatibility
  end

  def verify(dsl_state) do
    # Verify the DSL state is valid
  end
end
```

#### 2. DSL Sections and Entities

```elixir
section :actions do
  entity :create do
    field :name, :atom, required?: true
    field :accept, {:list, :atom}, default: []
    field :transaction?, :boolean, default: true

    sections: [
      changes: [
        entities: [
          change: [
            name: :change,
            schema: [module: [type: :module]]
          ]
        ]
      ]
    ]
  end
end
```

#### 3. Persisted Configuration

Spark persists DSL configuration using module attributes:

```elixir
@persist {:actions, actions_list}
@persist {:attributes, attributes_list}
@persist {:relationships, relationships_list}
```

This configuration is accessible at runtime via `Ash.Resource.Info` functions.

### Spark DSL Internals

**Compilation Flow:**

```
┌─────────────┐     ┌─────────────┐     ┌─────────────┐
│  use Spark  │────▶│   init/1    │────▶│handle_opts/1│
│   .Dsl      │     │  Callback   │     │  Callback   │
└─────────────┘     └─────────────┘     └─────────────┘
                                               │
         ┌─────────────────────────────────────┘
         ▼
┌─────────────┐     ┌─────────────┐     ┌─────────────┐
│   Compile   │◀────│   verify/2  │◀────│  handle_    │
│   Module    │     │  Callback   │     │before_compile│
└─────────────┘     └─────────────┘     └─────────────┘
```

### Key Spark Modules

| Module | Purpose |
|--------|---------|
| `Spark.Dsl` | Main DSL behavior |
| `Spark.Dsl.Extension` | Extension behavior |
| `Spark.Dsl.Entity` | DSL entity definition |
| `Spark.Dsl.Section` | DSL section definition |
| `Spark.Options` | Option validation |
| `Spark.Docs` | Documentation generation |

---

## Design Patterns

### 1. Declarative Configuration

All Ash configuration is declarative:

```elixir
# Declarative (Ash way)
create :register do
  accept [:email, :password]
  change hash_password(:password)
  validate confirm_password(:password, :password_confirmation)
end

# Imperative (not Ash way)
def register(params) do
  if params.password == params.password_confirmation do
    hash = hash_password(params.password)
    create_user(%{email: params.email, password_hash: hash})
  end
end
```

**Benefits:**
- Introspectable at compile-time
- Composable extensions
- Automatic documentation
- Consistent validation

### 2. Extension Composition

Extensions can be composed and layered:

```elixir
defmodule Post do
  use Ash.Resource,
    extensions: [
      AshStateMachine,      # Adds state machine DSL
      AshCloak.Resource,    # Adds encryption DSL
      AshGraphql.Resource   # Adds GraphQL DSL
    ]
end
```

Each extension:
1. Adds DSL sections
2. Transforms the DSL state
3. Verifies compatibility
4. Injects compile-time code

### 3. Escape Hatches

Ash provides escape hatches at every level:

```elixir
# 1. Manual actions for custom logic
action :complex_operation, :map do
  run fn input, _ ->
    # Raw Elixir code
  end
end

# 2. Manual relationships for custom joins
manual_relationship :complex_relation do
  run fn records, _ ->
    # Custom query logic
  end
end

# 3. Direct data layer access
Ash.DataLayer.run_query(query, resource)
```

---

## Performance Considerations

### 1. Compile-Time Optimization

- DSL configuration is compiled into efficient data structures
- Validations run at compile-time where possible
- Protocol consolidation in production

### 2. Runtime Optimization

- Changesets are structs (efficient pattern matching)
- Queries are composable data structures
- Lazy loading of relationships

### 3. Database Optimization (AshPostgres)

- Query pushdown to database
- Efficient joins via lateral joins
- Bulk operations support

---

## Testing Utilities

### Ash.Test

```elixir
use Ash.Test

setup do
  # Setup test data
end

test "create user" do
  assert {:ok, user} = MyApp.Accounts.register("test@example.com", "password")
  assert user.email == "test@example.com"
end
```

### Ash.Generator

Property-based testing with StreamData integration:

```elixir
test "user email is always valid" do
  check all attrs <- Ash.Generator.attributes(MyApp.Accounts.User) do
    # Property test
  end
end
```

---

## Conclusion

Ash Core provides a robust, extensible foundation for building Elixir applications. The key takeaways are:

1. **Declarative DSL**: All configuration is declarative, enabling introspection and extension
2. **Action-Centric**: Actions are the primary abstraction for business logic
3. **Extensible**: Spark extension system allows deep customization
4. **Data Layer Agnostic**: Works with multiple persistence backends
5. **Production Ready**: Comprehensive monitoring, testing, and error handling

The core is designed to be "batteries-included" while maintaining flexibility through well-defined extension points.
