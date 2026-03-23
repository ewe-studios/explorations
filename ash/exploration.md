# Ash Framework Exploration

## Overview

Ash Framework is a **declarative, extensible application framework** for Elixir that brings a "batteries-included" experience similar to Django, but built for the BEAM (Erlang VM). It is designed for building web applications, APIs, and services with a focus on **modeling your domain** and **deriving everything else automatically**.

**Version**: 3.5.25 (core ash package)
**Language**: Elixir (1721+ source files in core alone)
**Total Files**: 2561+ source files across all sub-projects

## Design Philosophy

Ash is built on several core principles:

### 1. Anything, Not Everything
Ash builds a framework capable of doing *anything*, not one that already does *everything*. It provides extension points at every level, allowing you to build custom behavior while leveraging prebuilt extensions.

### 2. Declarative, Introspectable, Derivable
All behavior is driven by **explicit, static declarations**. A Resource is essentially a configuration file that is provided to code that reads that configuration and acts accordingly. This enables:
- **Introspection**: Other tools can discover what actions exist
- **Derivation**: Extensions automatically understand and build on top of declarations
- **Type Safety**: Arguments and return types are known at compile time

### 3. Configuration over Convention
Ash prefers explicit configuration over implicit conventions. This means:
- No magic based on file names or locations
- All behavior is explicitly declared
- More discoverable and maintainable code

### 4. Pragmatism First
Focus on what current users need, with clean upgrade paths and rare breaking changes.

## Core Abstractions

### Resources and Actions

The fundamental abstractions in Ash are:

**Resources**: Static definitions of entities in your system (similar to Ecto schemas but richer). Resources define:
- Attributes (with types, constraints, validations)
- Relationships (belongs_to, has_one, has_many, many_to_many)
- Calculations (computed fields)
- Aggregates (summary data)
- Identities (unique constraints)

**Actions**: The operations you can perform - the core business logic:
- `read` - Query resources
- `create` - Create new records
- `update` - Modify existing records
- `destroy` - Remove records
- `action` - Generic operations that don't fit CRUD

```elixir
defmodule MyBlog.Blog.Post do
  use Ash.Resource,
    domain: MyBlog.Blog,
    data_layer: AshPostgres.DataLayer

  attributes do
    uuid_primary_key :id
    attribute :title, :string, allow_nil?: false, public?: true
    attribute :content, :string, public?: true
    attribute :status, :atom, constraints: [one_of: [:draft, :published]], default: :draft
    timestamps()
  end

  actions do
    defaults [:read, :destroy]
    create :create_post do
      accept [:title, :content]
    end
    update :publish do
      accept [:title, :content]
      change set_attribute(:status, :published)
    end
  end
end
```

### Domains

Domains group related resources together and provide the module through which you interact with resources:

```elixir
defmodule MyBlog.Blog do
  use Ash.Domain

  resources do
    resource MyBlog.Blog.Post
    resource MyBlog.Blog.Author
  end
end

# Usage
{:ok, post} = MyBlog.Blog.create_post(%{title: "Hello", content: "World"})
```

## Architecture Layers

### 1. Core (ash)
The main Ash framework providing:
- Resource DSL and behavior
- Action execution engine
- Query system
- Changeset API
- Policy-based authorization
- Data layer behavior

### 2. Spark DSL System
Ash uses the **Spark** library for its declarative DSL system. Spark provides:
- DSL definition utilities
- Extension system
- Configuration validation
- Compile-time verification

### 3. Data Layers
Ash supports multiple data layers through a plugin architecture:
- **AshPostgres**: PostgreSQL with full query support
- **AshSqlite**: SQLite support
- **AshMysql**: MySQL support
- **AshCubdb**: CubDB embedded database
- **ETS/Mnesia**: In-memory Erlang term storage

### 4. API Extensions
- **AshJsonApi**: JSON:API specification builder
- **AshGraphql**: GraphQL API builder

### 5. Resource Extensions
- **AshStateMachine**: State machine support
- **AshArchival**: Soft-delete with archival
- **AshPaperTrail**: Audit logging/history
- **AshCloak**: Encryption at rest
- **AshOban**: Background job integration

### 6. Tooling
- **Igniter**: Code generation and project scaffolding
- **AshAdmin**: Auto-generated admin UI
- **Evals**: Evaluation and testing utilities

### 7. Reactor (Workflow Orchestration)
- **Reactor**: Dependency-resolving saga orchestrator
- **ReactorFile**: File-based reactor definitions
- **Splode**: Error handling utilities

## Key Features

### 1. Rich Query System
Ash provides a powerful query system that translates Elixir expressions to database queries:

```elixir
Ash.Query.filter(Post, author_id == ^author_id and status == :published)
|> Ash.Query.sort(:published_at, :desc)
|> Ash.Query.limit(10)
|> Ash.read!()
```

### 2. Policy-Based Authorization
Fine-grained authorization using policies:

```elixir
policies do
  policy actor_attribute(:admin, true) do
    authorize_if always()
  end

  policy acting() do
    authorize_if relates_to_actor_via(:author)
  end
end
```

### 3. Manual Actions
For complex operations that don't fit standard CRUD:

```elixir
action :analyze_text, :map do
  argument :text, :string, allow_nil?: false
  run fn input, _context ->
    {:ok, %{word_count: String.length(input.arguments.text)}}
  end
end
```

### 4. Extensions Working Together
Multiple extensions can enhance the same resource:

```elixir
defmodule Post do
  use Ash.Resource,
    extensions: [
      AshStateMachine,
      AshCloak.Resource,
      AshGraphql.Resource
    ]

  # All extensions work together seamlessly
end
```

## Ecosystem Projects

| Project | Description | Version |
|---------|-------------|---------|
| ash | Core framework | 3.5.25 |
| ash_postgres | PostgreSQL data layer | 2.6.9 |
| ash_graphql | GraphQL API builder | 1.7.17 |
| ash_json_api | JSON:API builder | - |
| ash_phoenix | Phoenix integrations | - |
| reactor | Workflow orchestrator | 0.11+ |
| spark | DSL framework | 2.2+ |
| igniter | Code generator | 0.6+ |
| splode | Error utilities | 0.2.6+ |

## Comparison with Other Frameworks

### vs Phoenix
Ash is **not** a web framework - it complements Phoenix. Ash handles the application layer (domain modeling, business logic, data access), while Phoenix handles the web layer (HTTP, WebSockets, LiveView).

### vs Django
Ash is often compared to Django for its "batteries-included" approach:
- Ash Resource ≈ Django Model
- Ash Domain ≈ Django App
- AshJsonApi ≈ Django REST Framework
- AshAdmin ≈ Django Admin
- AshAuthentication ≈ Django Allauth

### vs Ecto
Ash builds on Ecto (uses Ecto schemas under the hood) but adds:
- Action abstraction layer
- Built-in authorization
- Declarative resource definitions
- Extension ecosystem

## File Structure

```
src.Ash/
├── ash/                    # Core framework
│   ├── lib/ash/
│   │   ├── actions/        # Action implementations
│   │   ├── changeset/      # Changeset API
│   │   ├── data_layer/     # Data layer behaviors
│   │   ├── domain/         # Domain DSL
│   │   ├── query/          # Query system
│   │   ├── policy/         # Authorization
│   │   ├── resource/       # Resource DSL
│   │   └── type/           # Type system
│   └── documentation/      # Comprehensive docs
├── spark/                  # DSL framework
├── ash_postgres/           # PostgreSQL adapter
├── ash_graphql/            # GraphQL extension
├── ash_json_api/           # JSON:API extension
├── ash_phoenix/            # Phoenix integration
├── reactor/                # Workflow engine
├── igniter/                # Code generation
└── splode/                 # Error handling
```

## Getting Started Pattern

The typical progression when using Ash:

1. **Start with behavior**: Define actions without state
2. **Add persistence**: Connect to a data layer
3. **Add API**: Enable GraphQL or JSON:API
4. **Add extensions**: Encryption, state machines, archival

```elixir
# Step 1: Pure behavior
action :analyze_text, :map do
  argument :text, :string
  run fn input, _ -> {:ok, %{count: String.length(input.arguments.text)}} end
end

# Step 2: Add data layer
use Ash.Resource, data_layer: AshPostgres.DataLayer

# Step 3: Add GraphQL
extensions: [AshGraphql.Resource]

# Step 4: Add encryption
extensions: [AshCloak.Resource]
```

## Community and Support

- **Discord**: Active community server
- **ElixirForum**: Dedicated forum section
- **GitHub**: Open source with active development
- **HexDocs**: Comprehensive documentation

## Conclusion

Ash Framework represents a new approach to Elixir application development - one that emphasizes:
- **Declarative configuration** over imperative code
- **Domain modeling** as the single source of truth
- **Extension over replacement** - works with existing Elixir ecosystem
- **Pragmatic flexibility** - escape hatches at every level

The framework is particularly well-suited for:
- Complex business domains requiring rich modeling
- Applications needing multiple API interfaces
- Teams wanting consistency and reduced boilerplate
- Long-term maintainability focus
