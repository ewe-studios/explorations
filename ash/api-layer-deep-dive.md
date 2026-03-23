# API Layer Deep Dive

## Overview

This deep dive covers Ash's API layer extensions:
- **ash_graphql**: GraphQL API builder (v1.7.17)
- **ash_json_api**: JSON:API specification builder
- **ash_hq**: Admin interface and tooling

---

## Part 1: AshGraphql (`ash_graphql` v1.7.17)

### Architecture Overview

AshGraphql automatically generates a complete GraphQL API from your Ash resource definitions. It uses Absinthe under the hood.

```
┌─────────────────────────────────────────────────────────────┐
│                   Ash Resources                            │
│  - Attributes (types, nullability)                         │
│  - Actions (queries, mutations)                            │
│  - Relationships (associations)                            │
│  - Calculations (computed fields)                          │
└─────────────────────────────────────────────────────────────┘
                            │
                            ▼
┌─────────────────────────────────────────────────────────────┐
│                AshGraphql Extension                        │
│  ┌─────────────────────────────────────────────────────┐   │
│  │  Schema Generator                                     │   │
│  │  - GraphQL types from Ash types                     │   │
│  │  - Query fields from read actions                   │   │
│  │  - Mutation fields from CUD actions                 │   │
│  └─────────────────────────────────────────────────────┘   │
│  ┌─────────────────────────────────────────────────────┐   │
│  │  Resolver                                           │   │
│  │  - Translate GraphQL args to Ash queries           │   │
│  │  - Handle authorization                            │   │
│  │  - Error transformation                            │   │
│  └─────────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────────┘
                            │
                            ▼
┌─────────────────────────────────────────────────────────────┐
│                   Absinthe                                 │
│  - Schema compilation                                       │
│  - Query execution                                         │
│  - Middleware pipeline                                     │
└─────────────────────────────────────────────────────────────┘
```

### Basic Configuration

**Domain Configuration:**

```elixir
defmodule MyApp.Blog do
  use Ash.Domain,
    extensions: [AshGraphql.Domain]

  resources do
    resource MyApp.Blog.Post do
      # Define which actions to expose
      graphql do
        queries [:read]
        mutations [:create, :update, :destroy]
      end
    end
  end

  graphql do
    queries do
      # Expose read actions as queries
      action MyApp.Blog.Post, :read, :posts

      # Custom query with arguments
      query :post_by_id, :post do
        arg :id, :uuid
        resolve fn args, _ ->
          Ash.get(MyApp.Blog.Post, args.id)
        end
      end
    end

    mutations do
      # Expose CUD actions as mutations
      mutation MyApp.Blog.Post, :create, :create_post
      mutation MyApp.Blog.Post, :update, :update_post
      mutation MyApp.Blog.Post, :destroy, :destroy_post
    end
  end
end
```

**Resource Configuration:**

```elixir
defmodule MyApp.Blog.Post do
  use Ash.Resource,
    domain: MyApp.Blog,
    data_layer: AshPostgres.DataLayer,
    extensions: [AshGraphql.Resource]

  attributes do
    uuid_primary_key :id
    attribute :title, :string, allow_nil?: false, public?: true
    attribute :body, :string, public?: true
    attribute :status, :atom, constraints: [one_of: [:draft, :published]], default: :draft, public?: true
    timestamps()
  end

  relationships do
    belongs_to :author, MyApp.Accounts.User, public?: true
    has_many :comments, MyApp.Blog.Comment, public?: true
  end

  actions do
    defaults [:read, :destroy]

    create :create_post do
      accept [:title, :body, :author_id]
    end

    update :update_post do
      accept [:title, :body, :status]
    end
  end

  graphql do
    type :post

    # Customize field names
    field :title, :title
    field :body, :content

    # Hide fields
    hide_fields [:status]

    # Custom resolvers
    field :author, :user do
      resolve fn post, _, _ ->
        Ash.load!(post, [:author])
      end
    end
  end
end
```

### Generated GraphQL Schema

From the above configuration, AshGraphql generates:

```graphql
# Queries
type Query {
  # List posts with filtering, sorting, pagination
  posts(
    filter: PostFilter
    sort: [PostSort]
    limit: Int
    offset: Int
  ): [Post!]!

  # Get single post by ID
  postById(id: UUID!): Post
}

# Mutations
type Mutation {
  createPost(input: CreatePostInput!): CreatePostResult!
  updatePost(id: UUID!, input: UpdatePostInput!): UpdatePostResult!
  destroyPost(id: UUID!): DestroyPostResult!
}

# Types
type Post {
  id: UUID!
  title: String!
  content: String
  author: User
  comments: [Comment!]!
  insertedAt: DateTime!
  updatedAt: DateTime!
}

input CreatePostInput {
  title: String!
  content: String
  authorId: UUID
}

input UpdatePostInput {
  title: String
  content: String
  status: PostStatus
}

enum PostStatus {
  draft
  published
}

# Filter input
input PostFilter {
  id: UUIDFilter
  title: StringFilter
  status: PostStatusFilter
  and: [PostFilter]
  or: [PostFilter]
}

input UUIDFilter {
  eq: UUID
  in: [UUID]
}

input StringFilter {
  eq: String
  contains: String
  starts_with: String
  ends_with: String
}
```

### Advanced Features

#### 1. Custom Queries and Mutations

```elixir
graphql do
  queries do
    # Simple query
    query :recent_posts, [:post] do
      arg :limit, :integer, default: 10
      resolve fn args, _ ->
        MyApp.Blog.Post
        |> Ash.Query.sort(inserted_at: :desc)
        |> Ash.Query.limit(args.limit)
        |> Ash.read!()
      end
    end
  end

  mutations do
    # Custom mutation
    mutation :publish_post, :post do
      arg :id, :uuid

      resolve fn args, _ ->
        post = Ash.get!(MyApp.Blog.Post, args.id)
        Ash.update!(post, %{status: :published})
      end
    end
  end
end
```

#### 2. Relay Connections (Cursor-based Pagination)

```elixir
graphql do
  type :post do
    connection :comments do
      # Custom edge fields
      edge_field :highlight, :string
    end
  end
end

# Generates:
# postComments(after: String, before: String, first: Int, last: Int): PostCommentConnection
```

#### 3. Subscriptions

```elixir
defmodule MyApp.Blog do
  use Ash.Domain,
    extensions: [AshGraphql.Domain]

  graphql do
    subscriptions do
      subscription :post_created, :post do
        # Trigger on create action
        topic fn _ ->
          # Subscribe to all post creation events
          ["posts:created"]
        end
      end

      subscription :post_updated, :post do
        topic fn args ->
          # Subscribe to specific post updates
          ["posts:#{args.id}"]
        end
      end
    end
  end
end

# In resource, notify on actions:
actions do
  create :create do
    accept [:title, :body]
    change notify_topic("posts:created")
  end
end
```

#### 4. Generic Actions

Generic (non-CRUD) actions can be exposed:

```elixir
# Resource action
defmodule MyApp.Blog.Post do
  actions do
    action :analyze_sentiment, :map do
      argument :post_id, :uuid
      run fn input, _ ->
        # Analyze sentiment
        {:ok, %{sentiment: "positive", score: 0.95}}
      end
    end
  end
end

# Domain GraphQL config
graphql do
  queries do
    action MyApp.Blog.Post, :analyze_sentiment, :analyze_post_sentiment
  end
end

# GraphQL:
# query AnalyzePostSentiment($postId: UUID!) {
#   analyzePostSentiment(postId: $postId) {
#     sentiment
#     score
#   }
# }
```

#### 5. Error Handling

```elixir
graphql do
  # Custom error types
  errors do
    error :not_found, "Resource not found"
    error :unauthorized, "Access denied"
    error :validation, "Validation failed"
  end

  # Error handler
  error_handler MyApp.Graphql.ErrorHandler
end

# Error handler module
defmodule MyApp.Graphql.ErrorHandler do
  def handle_error(%Ash.Error.NotFound{}), do: {:error, :not_found}
  def handle_error(%Ash.Error.Forbidden{}), do: {:error, :unauthorized}
  def handle_error(%Ash.Error.Changes.InvalidChanges{}), do: {:error, :validation}
end
```

#### 6. Custom Types and Scalars

```elixir
graphql do
  # Custom scalar
  scalar :email, :string do
    parse fn value, _ ->
      if String.contains?(value, "@") do
        {:ok, value}
      else
        :error
      end
    end

    serialize fn value ->
      value
    end
  end

  # Custom enum
  enum :role do
    values [:admin, :user, :guest]
  end
end
```

### Authorization with GraphQL

AshGraphql integrates with Ash's policy system:

```elixir
defmodule MyApp.Blog.Post do
  use Ash.Resource

  policies do
    # Public read access
    policy always() do
      authorize_if always()
    end

    # Only authors can update their posts
    policy always() do
      authorize_if relates_to_actor_via(:author)
    end

    # Admins can do anything
    policy actor_attribute(:role, :admin) do
      authorize_if always()
    end
  end
end
```

In GraphQL context, the actor is passed through:

```elixir
# Phoenix socket configuration
defmodule MyApp.UserSocket do
  use Phoenix.Socket

  connect fn params, socket ->
    user = get_user_from_token(params["token"])
    {:ok, assign(socket, :actor, user)}
  end
end
```

### Performance Optimizations

#### 1. DataLoader Integration

```elixir
# Use Absinthe DataLoader for batched loading
defmodule MyApp.Graphql.Resolver do
  use Absinthe.Resolution

  alias MyApp.Repo

  def resolve_author(post, _, %{context: context}) do
    loader = context[:loader]
    DataLoader.load(loader, MyApp.DataLoader, {MyApp.Accounts.User, post.author_id})
  end
end
```

#### 2. Select Optimization

```elixir
graphql do
  type :post do
    # Only select fields that are requested
    select_fields [:id, :title, :body]
  end
end
```

#### 3. N+1 Prevention

```elixir
# Bad: N+1 queries
def resolve_posts(_, _, _) do
  posts = Ash.read!(MyApp.Blog.Post)
  Enum.map(posts, fn post ->
    # Triggers query for each post
    %{post | author: Ash.get!(MyApp.Accounts.User, post.author_id)}
  end)
end

# Good: Batch load
def resolve_posts(_, _, _) do
  Ash.read!(MyApp.Blog.Post, load: [:author])
end
```

### SDL File Generation

AshGraphql can generate a complete SDL schema file:

```elixir
# In config/dev.exs
config :ash_graphql, :generate_schema, true
config :ash_graphql, :schema_file, "priv/graphql/schema.graphql"

# Generate
mix ash_graphql.generate_schema
```

Generated `schema.graphql`:

```graphql
type Query {
  posts(filter: PostFilter, sort: [PostSort], limit: Int, offset: Int): [Post!]!
  postById(id: UUID!): Post
}

type Mutation {
  createPost(input: CreatePostInput!): CreatePostResult!
  updatePost(id: UUID!, input: UpdatePostInput!): UpdatePostResult!
  destroyPost(id: UUID!): DestroyPostResult!
}

type Post {
  id: UUID!
  title: String!
  body: String
  author: User
  comments: [Comment!]!
}
```

---

## Part 2: AshJsonApi

### Overview

AshJsonApi generates JSON:API compliant REST endpoints from Ash resources.

### Configuration

```elixir
defmodule MyApp.Blog do
  use Ash.Domain,
    extensions: [AshJsonApi.Domain]

  resources do
    resource MyApp.Blog.Post do
      json_api do
        base_route "/posts"
      end
    end
  end

  json_api do
    # Global configuration
    base_route "/api"
    type_prefix "v1"
  end
end

defmodule MyApp.Blog.Post do
  use Ash.Resource,
    domain: MyApp.Blog,
    extensions: [AshJsonApi.Resource]

  attributes do
    uuid_primary_key :id
    attribute :title, :string, public?: true
    attribute :body, :string, public?: true
  end

  actions do
    defaults [:read, :create, :update, :destroy]
  end
end
```

### Generated Endpoints

| Method | Endpoint | Action |
|--------|----------|--------|
| GET | `/api/v1/posts` | List posts |
| GET | `/api/v1/posts/:id` | Get post |
| POST | `/api/v1/posts` | Create post |
| PATCH | `/api/v1/posts/:id` | Update post |
| DELETE | `/api/v1/posts/:id` | Destroy post |

### Request Examples

**List with filtering:**
```
GET /api/v1/posts?filter[status]=published&sort=-inserted_at&page[offset]=0&page[limit]=10
```

**Create:**
```json
POST /api/v1/posts
Content-Type: application/vnd.api+json

{
  "data": {
    "type": "post",
    "attributes": {
      "title": "Hello World",
      "body": "Content here"
    },
    "relationships": {
      "author": {
        "data": { "type": "user", "id": "uuid-here" }
      }
    }
  }
}
```

**Response:**
```json
{
  "data": {
    "id": "uuid-here",
    "type": "post",
    "attributes": {
      "title": "Hello World",
      "body": "Content here",
      "inserted-at": "2024-01-01T00:00:00Z"
    },
    "relationships": {
      "author": {
        "data": { "type": "user", "id": "uuid-here" }
      }
    }
  },
  "included": [
    {
      "id": "uuid-here",
      "type": "user",
      "attributes": {
        "name": "Author Name"
      }
    }
  ]
}
```

### Filtering

AshJsonApi supports complex filtering:

```
# Simple filter
GET /api/v1/posts?filter[status]=published

# Multiple filters
GET /api/v1/posts?filter[status]=published&filter[author_id]=uuid

# Nested filters
GET /api/v1/posts?filter[author][name]=John

# Operators
GET /api/v1/posts?filter[title][contains]=tutorial
GET /api/v1/posts?filter[inserted_at][gt]=2024-01-01
```

### Sorting

```
# Single sort
GET /api/v1/posts?sort=inserted_at

# Descending
GET /api/v1/posts?sort=-inserted_at

# Multiple sorts
GET /api/v1/posts?sort=-inserted_at,title
```

### Pagination

```
# Offset pagination
GET /api/v1/posts?page[offset]=0&page[limit]=20

# Response includes pagination meta
{
  "meta": {
    "total": 100,
    "offset": 0,
    "limit": 20
  }
}
```

### Field Selection

```
# Sparse fieldsets
GET /api/v1/posts?fields[post]=title,body
```

### Including Relationships

```
# Include author
GET /api/v1/posts?include=author

# Nested include
GET /api/v1/posts?include=author,comments.author
```

---

## Part 3: AshHQ

### Overview

AshHQ is the admin interface and operational tooling for Ash applications.

### Features

1. **Admin UI**: Auto-generated admin interface
2. **Resource Browser**: View and manage resources
3. **Action Runner**: Execute actions manually
4. **Query Builder**: Build and test queries
5. **Policy Inspector**: Debug authorization policies

### Configuration

```elixir
defmodule MyAppWeb.AshAdmin do
  use AshAdmin,
    otp_app: :my_app,
    domains: [
      MyApp.Blog,
      MyApp.Accounts
    ]
end

# Add to router
scope "/admin", MyAppWeb do
  pipe_through [:browser, :authenticate_admin]
  AshAdmin.Router.route("/", AshAdmin)
end
```

### Admin Interface Features

**Resource Management:**
- List all resources in a domain
- Filter, sort, paginate records
- Create, update, destroy records
- View relationships

**Action Execution:**
- Run any action manually
- View action results
- Inspect errors

**Policy Debugging:**
- See which policies apply
- Test authorization with different actors
- View policy evaluation results

---

## API Comparison

| Feature | AshGraphql | AshJsonApi |
|---------|------------|------------|
| Protocol | GraphQL | REST (JSON:API) |
| Query Flexibility | High (client defines) | Medium (predefined) |
| Over-fetching | No | Possible |
| Under-fetching | No | Possible |
| Caching | Complex | Standard HTTP |
| Real-time | Native subscriptions | Polling/WebSocket |
| Tooling | GraphiQL, Playground | Standard REST tools |
| Learning Curve | Steeper | Gentler |

---

## Best Practices

### 1. Schema Design

```elixir
# Good: Clear naming
graphql do
  query :posts, [:post]
  mutation :create_post, :post
end

# Avoid: Redundant naming
graphql do
  query :get_all_posts, [:post]
  mutation :do_create_post, :post
end
```

### 2. Authorization

```elixir
# Always pass actor through context
def resolve(_, %{context: %{actor: actor}} = resolution) do
  Ash.read(MyApp.Blog.Post, actor: actor)
end
```

### 3. Error Handling

```elixir
# Transform Ash errors to GraphQL errors
def handle_error(%Ash.Error.Forbidden{}),
  do: {:error, Absinthe.Execution.Error.new("Unauthorized")}

def handle_error(%Ash.Error.Changes.InvalidChanges{errors: errors}),
  do: {:error, errors |> Enum.map(&to_graphql_error/1)}
```

### 4. Performance

```elixir
# Use DataLoader for relationships
def resolve_author(post, _, %{context: context}) do
  DataLoader.load(context[:loader], MyApp.DataLoader, post.author_id)
end

# Select only needed fields
def resolve_posts(_, _, _) do
  Ash.read!(MyApp.Blog.Post, select: [:id, :title])
end
```

---

## Conclusion

Ash's API layer extensions provide:

1. **Automatic Generation**: APIs generated from resource definitions
2. **Consistency**: Same patterns across GraphQL and REST
3. **Flexibility**: Escape hatches for custom logic
4. **Performance**: Built-in optimizations (DataLoader, batching)
5. **Security**: Integrated authorization

The API layer is designed to be production-ready out of the box while remaining customizable for specific needs.
