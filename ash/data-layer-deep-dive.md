# Data Layer Deep Dive

## Overview

This deep dive covers Ash's data layer implementations:
- **ash_postgres**: PostgreSQL data layer (v2.6.9)
- **ash_mysql**: MySQL data layer
- **ash_sqlite**: SQLite data layer
- **ash_archival**: Soft-delete/archival extension

---

## Part 1: AshPostgres (`ash_postgres` v2.6.9)

### Architecture Overview

AshPostgres is the most feature-complete data layer for Ash. It translates Ash queries into efficient Ecto/PostgreSQL queries.

```
┌─────────────────────────────────────────────────────────────┐
│                    Ash Query                               │
│  filter(Post, author_id == ^id and status == :published)   │
└─────────────────────────────────────────────────────────────┘
                            │
                            ▼
┌─────────────────────────────────────────────────────────────┐
│              AshPostgres Data Layer                        │
│  ┌─────────────────────────────────────────────────────┐   │
│  │  Expression Translator                               │   │
│  │  - Ash expressions → Ecto.Query                    │   │
│  │  - Custom functions → PostgreSQL functions         │   │
│  └─────────────────────────────────────────────────────┘   │
│  ┌─────────────────────────────────────────────────────┐   │
│  │  Query Builder                                       │   │
│  │  - SELECT, WHERE, ORDER BY, LIMIT                 │   │
│  │  - JOINs for relationships                          │   │
│  │  - Subqueries for aggregates                        │   │
│  └─────────────────────────────────────────────────────┘   │
│  ┌─────────────────────────────────────────────────────┐   │
│  │  Migration Generator                                 │   │
│  │  - Schema introspection                             │   │
│  │  - Migration file generation                        │   │
│  └─────────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────────┘
                            │
                            ▼
┌─────────────────────────────────────────────────────────────┐
│                    Ecto.Repo                               │
│  query(Postgres, query, params, options)                  │
└─────────────────────────────────────────────────────────────┘
```

### Resource Configuration

```elixir
defmodule MyApp.Blog.Post do
  use Ash.Resource,
    domain: MyApp.Blog,
    data_layer: AshPostgres.DataLayer

  postgres do
    table "posts"
    repo MyApp.Repo
    # Optional: schema for multi-tenant setups
    # schema "tenant_schema"
  end

  attributes do
    uuid_primary_key :id
    attribute :title, :string, allow_nil?: false
    attribute :body, :string
    attribute :status, :atom, constraints: [one_of: [:draft, :published]]
    timestamps()
  end

  relationships do
    belongs_to :author, MyApp.Accounts.User
    has_many :comments, MyApp.Blog.Comment
  end
end
```

### Expression Translation

AshPostgres translates Ash expressions to PostgreSQL:

| Ash Expression | PostgreSQL |
|----------------|------------|
| `field == ^value` | `WHERE field = $1` |
| `field != ^value` | `WHERE field != $1` |
| `field in ^list` | `WHERE field = ANY($1)` |
| `field like ^pattern` | `WHERE field LIKE $1` |
| `field ilike ^pattern` | `WHERE field ILIKE $1` |
| `string_contains(field, ^text)` | `WHERE field LIKE '%' || $1 || '%'` |
| `fragment("lower(??)", field)` | Raw SQL fragment |

### Custom PostgreSQL Functions

AshPostgres supports custom PostgreSQL functions:

```elixir
defmodule MyApp.Functions.TrigramSimilarity do
  use AshPostgres.Function,
    name: "similarity",
    args: [:text, :text],
    return_type: :float

  def transform(args, _context) do
    # Transform Ash expression to PostgreSQL function call
    fragment("similarity(?, ?)", Enum.at(args, 0), Enum.at(args, 1))
  end
end
```

### Aggregates

AshPostgres supports various aggregate types:

```elixir
defmodule MyApp.Blog.Post do
  use Ash.Resource

  aggregates do
    count :comment_count, :comments

    sum :total_views, :comments, :views

    avg :average_rating, :ratings, :value

    # Custom aggregate with filter
    count :published_comments, :comments do
      filter_expr(status == :published)
    end
  end
end
```

### Migration Generation

AshPostgres can generate migrations from resource definitions:

```bash
# Generate migrations
mix ash_postgres.generate_migrations --auto-name

# Check migrations are up to date
mix ash_postgres.generate_migrations --check

# Apply migrations
mix ash_postgres.migrate

# Rollback migrations
mix ash_postgres.rollback
```

Generated migration example:

```elixir
defmodule MyApp.Repo.Migrations.CreatePosts do
  use Ecto.Migration

  def change do
    create table(:posts, primary_key: false) do
      add :id, :uuid, null: false, default: fragment("gen_random_uuid()")
      add :title, :text, null: false
      add :body, :text
      add :status, :text, null: false, default: "draft"
      add :inserted_at, :utc_datetime_usec, null: false, default: fragment("now()")
      add :updated_at, :utc_datetime_usec, null: false, default: fragment("now()")

      timestamps(type: :utc_datetime_usec)
    end

    create index(:posts, [:status])
    create index(:posts, [:inserted_at])
  end
end
```

### Advanced Features

#### 1. Lateral Joins

For efficient loading of related data with limits:

```elixir
# Load the 5 most recent comments per post
Ash.load!(posts, comments: [limit: 5, sort: [inserted_at: :desc]])
```

Generates efficient SQL using LATERAL JOIN:

```sql
SELECT p.*, c.*
FROM posts p
LEFT JOIN LATERAL (
  SELECT * FROM comments
  WHERE comments.post_id = p.id
  ORDER BY inserted_at DESC
  LIMIT 5
) c ON true
```

#### 2. Polymorphic Relationships

```elixir
defmodule MyApp.Content.Comment do
  use Ash.Resource

  relationships do
    belongs_to :commentable, {MyApp.Blog.Post, MyApp.Support.Ticket}, polymorphic?: true
  end
end
```

Stored in database with type discriminator:

| id | commentable_id | commentable_type |
|----|----------------|------------------|
| 1  | uuid-1         | MyApp.Blog.Post  |
| 2  | uuid-2         | MyApp.Support.Ticket |

#### 3. Full-Text Search

```elixir
defmodule MyApp.Blog.Post do
  use Ash.Resource

  postgres do
    table "posts"
    # Define full-text search index
    index [:title, :body], using: :gin, name: :posts_search_vector
  end

  calculations do
    # Calculate tsvector
    calculate :search_vector, :tsvector, expr(fragment("setweight(to_tsvector('english', ?), 'A') || setweight(to_tsvector('english', ?), 'B')", title, body))
  end
end

# Search query
Ash.Query.filter(Post, fragment("search_vector @@ plainto_tsquery('english', ?)", ^query))
```

#### 4. Multi-Tenancy

Schema-based multi-tenancy:

```elixir
defmodule MyApp.Repo do
  use AshPostgres.Repo,
    otp_app: :my_app,
    installed_extensions: ["uuid-ossp", "pgcrypto"]

  def tenant_prefix(tenant), do: "tenant_#{tenant}"
end

# Usage
Ash.Query.filter(Post, status == :published)
|> Ash.Query.set_tenant("acme")
|> Ash.read!()
```

#### 5. Custom Types

AshPostgres supports custom PostgreSQL types:

```elixir
defmodule MyApp.Ltree do
  use AshPostgres.Type,
    source: :ltree,
    extended: true

  def cast(value), do: {:ok, value}
  def load(value), do: {:ok, value}
  def dump(value), do: {:ok, value}
end

# Usage in resource
attribute :category_path, MyApp.Ltree
```

### Performance Optimizations

#### 1. Query Pushdown

AshPostgres pushes as much computation as possible to the database:

```elixir
# This entire query is executed in PostgreSQL
Post
|> Ash.Query.filter(status == :published)
|> Ash.Query.filter(fragment("created_at > now() - interval '30 days'"))
|> Ash.Query.load(:author)
|> Ash.Query.select([:id, :title, :author_id])
|> Ash.Query.sort(inserted_at: :desc)
|> Ash.Query.limit(10)
|> Ash.read!()
```

#### 2. Bulk Operations

```elixir
# Bulk create (batch insert)
Ash.BulkResult.create!(inputs, Post, :create, batch_size: 1000)

# Bulk update with query
Post
|> Ash.Query.filter(status == :draft)
|> Ash.Query.bulk_update(set: [status: :archived])

# Bulk destroy
Post
|> Ash.Query.filter(status == :deleted)
|> Ash.Query.bulk_destroy()
```

#### 3. Prepared Statements

Ecto automatically prepares and caches frequently used queries.

### Monitoring Integration

AshPostgres integrates with Ash's telemetry:

```elixir
# Telemetry events
[:ash, :postgres, :query, :start]
[:ash, :postgres, :query, :stop]

# Attach handler
:telemetry.attach(
  "my-app-pg-handler",
  [:ash, :postgres, :query, :stop],
  &MyApp.Metrics.handle_pg_query/3,
  nil
)
```

---

## Part 2: AshSqlite

### Overview

AshSqlite provides SQLite support for Ash resources. It's useful for:
- Local development
- Embedded applications
- Testing
- Small-scale deployments

### Configuration

```elixir
defmodule MyApp.Repo do
  use AshSqlite.Repo,
    otp_app: :my_app,
    database: "priv/my_app.sqlite"
end

defmodule MyApp.Post do
  use Ash.Resource,
    domain: MyApp.Blog,
    data_layer: AshSqlite.DataLayer

  sqlite do
    table "posts"
    repo MyApp.Repo
  end

  # ... rest of resource definition
end
```

### Limitations vs PostgreSQL

| Feature | PostgreSQL | SQLite |
|---------|------------|--------|
| Full-text search | Advanced (tsvector) | Basic (FTS5) |
| JSON support | JSONB | JSON |
| Custom types | Extensive | Limited |
| Concurrency | High | Moderate |
| Window functions | Full support | Limited (3.25+) |

---

## Part 3: AshMysql

### Overview

AshMysql provides MySQL/MariaDB support for Ash resources.

### Configuration

```elixir
defmodule MyApp.Repo do
  use AshMysql.Repo,
    otp_app: :my_app,
    database: "my_app_dev",
    username: "root",
    password: "password",
    hostname: "localhost"
end

defmodule MyApp.Post do
  use Ash.Resource,
    domain: MyApp.Blog,
    data_layer: AshMysql.DataLayer

  mysql do
    table "posts"
    repo MyApp.Repo
  end

  # ... rest of resource definition
end
```

### MySQL-Specific Features

#### 1. Generated Columns

```elixir
defmodule MyApp.Post do
  use Ash.Resource

  attributes do
    attribute :title, :string
    attribute :title_lower, :string do
      mysql do
        generated ":lower(`title`)"
        stored true
      end
    end
  end
end
```

#### 2. Spatial Types

```elixir
defmodule MyApp.Location do
  use Ash.Resource

  attributes do
    attribute :coordinates, :map do
      mysql do
        type :point
      end
    end
  end
end
```

---

## Part 4: AshArchival (`ash_archival`)

### Overview

AshArchival provides soft-delete functionality for Ash resources. Instead of permanently deleting records, they are moved to an archive.

### Installation

```elixir
def deps do
  [
    {:ash_archival, "~> 2.0"}
  ]
end
```

### Configuration

```elixir
defmodule MyApp.Blog.Post do
  use Ash.Resource,
    domain: MyApp.Blog,
    data_layer: AshPostgres.DataLayer,
    extensions: [AshArchival.Resource]

  archive do
    # Use the default archived_at field
    # or customize:
    archived_field :archived_at
    archived_value true
  end

  actions do
    defaults [:read, :create, :update]

    # Archive action (soft delete)
    destroy :archive do
      soft? true
    end

    # Hard destroy (permanent deletion)
    destroy :destroy do
      soft? false
    end
  end
end
```

### Generated Actions

AshArchival automatically adds:

1. `:archive` - Soft delete (sets `archived_at` timestamp)
2. `:restore` - Restore archived record
3. `:destroy` - Permanent deletion (opt-in)

### Query Filtering

Archived records are automatically filtered from reads:

```elixir
# Only returns non-archived posts
Ash.read!(MyApp.Blog.Post)

# Only archived posts
Ash.Query.filter(MyApp.Blog.Post, archived_at != nil)
|> Ash.read!()

# Include archived
Ash.Query.with_archived(MyApp.Blog.Post)
|> Ash.read!()
```

### Migration

```elixir
defmodule MyApp.Repo.Migrations.AddArchivedAtToPosts do
  use Ecto.Migration

  def change do
    alter table(:posts) do
      add :archived_at, :utc_datetime
    end

    create index(:posts, [:archived_at])
  end
end
```

### Archival with Relationships

```elixir
defmodule MyApp.Blog.Post do
  use Ash.Resource,
    extensions: [AshArchival.Resource]

  archive do
    # Cascade archive related comments
    archive_related [:comments]
  end

  relationships do
    has_many :comments, MyApp.Blog.Comment
  end
end
```

---

## Data Layer Comparison

| Feature | AshPostgres | AshSqlite | AshMysql |
|---------|-------------|-----------|----------|
| Primary Key Types | UUID, Integer | UUID, Integer | UUID, Integer |
| Composite Keys | Yes | Yes | Yes |
| Transactions | Yes | Yes | Yes |
| Row-Level Locking | Yes (FOR UPDATE) | Yes (BEGIN IMMEDIATE) | Yes (FOR UPDATE) |
| Bulk Operations | Yes | Yes | Yes |
| Custom Functions | Extensive | Limited | Moderate |
| JSON Support | JSONB (rich) | JSON (basic) | JSON (moderate) |
| Full-Text Search | Advanced | Basic | Moderate |
| Geographic Types | PostGIS | None | Spatial |
| Migration Tooling | Excellent | Good | Good |

---

## Query Translation Examples

### Simple Filter

**Ash Query:**
```elixir
Ash.Query.filter(Post, status == :published and author_id == ^author_id)
```

**PostgreSQL:**
```sql
SELECT * FROM posts
WHERE status = 'published' AND author_id = $1
```

### Aggregation

**Ash Query:**
```elixir
Ash.Query.aggregate(Post, :count, :comments)
```

**PostgreSQL:**
```sql
SELECT COUNT(*) as count
FROM comments
WHERE comments.post_id = posts.id
```

### Complex Join with Calculation

**Ash Query:**
```elixir
Post
|> Ash.Query.filter(status == :published)
|> Ash.Query.load(author: [calculations: [:full_name]])
|> Ash.Query.sort(inserted_at: :desc)
|> Ash.Query.limit(10)
```

**PostgreSQL:**
```sql
SELECT p.*, a.first_name || ' ' || a.last_name AS author_full_name
FROM posts p
LEFT JOIN authors a ON p.author_id = a.id
WHERE p.status = 'published'
ORDER BY p.inserted_at DESC
LIMIT 10
```

### Lateral Join for Limited Relationships

**Ash Query:**
```elixir
Post
|> Ash.Query.load(comments: [limit: 5, sort: [inserted_at: :desc]])
```

**PostgreSQL:**
```sql
SELECT p.*, c.*
FROM posts p
LEFT JOIN LATERAL (
  SELECT * FROM comments
  WHERE post_id = p.id
  ORDER BY inserted_at DESC
  LIMIT 5
) c ON true
```

---

## Best Practices

### 1. Use Appropriate Data Types

```elixir
# Good: Use appropriate types
attribute :email, :ci_string  # Case-insensitive string
attribute :status, :atom, constraints: [one_of: [:active, :inactive]]
attribute :metadata, :map, default: %{}

# Avoid: Generic types for everything
attribute :email, :string
attribute :status, :string
```

### 2. Leverage Database Constraints

```elixir
# Let the database enforce constraints too
postgres do
  table "posts"
  # Unique constraint
  index [:email], unique: true
  # Check constraint
  check constraint: "status IN ('draft', 'published', 'archived')"
end
```

### 3. Use Bulk Operations for Batch Processing

```elixir
# Instead of:
Enum.each(records, &Ash.update!/2)

# Use:
Ash.BulkResult.update!(records, Post, :update, batch_size: 1000)
```

### 4. Index Strategically

```elixir
postgres do
  # Index foreign keys
  index [:author_id]
  # Index frequently filtered columns
  index [:status, :created_at]
  # Composite indexes for common query patterns
  index [:author_id, :status, :created_at]
end
```

### 5. Handle N+1 Queries

```elixir
# Bad: N+1 query problem
posts = Ash.read!(Post)
Enum.each(posts, fn post ->
  # Triggers a query for each post
  IO.inspect(post.author)
end)

# Good: Preload relationships
posts = Ash.read!(Post, load: [:author])
```

---

## Conclusion

Ash's data layers provide:

1. **Consistent API**: Same Ash syntax regardless of database
2. **Database-Specific Optimizations**: Leverage unique features of each database
3. **Migration Support**: Generate and manage database schema
4. **Performance**: Efficient query translation and execution
5. **Extensibility**: Custom types, functions, and aggregates

The data layer is a critical component that translates Ash's declarative queries into efficient database operations while maintaining type safety and providing powerful abstractions.
