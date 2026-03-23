# Extensions Deep Dive

## Overview

This deep dive covers Ash's extension ecosystem:
- **ash_ai**: AI/ML integrations
- **ash_cloak**: Encryption at rest
- **ash_paper_trail**: Audit logging
- **ash_rate_limiter**: Rate limiting
- **ash_slug**: URL-friendly slugs

---

## Part 1: AshAI (`ash_ai`)

### Overview

AshAI provides integrations for AI/ML capabilities including:
- Structured outputs from LLMs
- Vector embeddings and search
- MCP (Model Context Protocol) support

### Key Features

#### 1. Structured Outputs

```elixir
defmodule MyApp.Content.Article do
  use Ash.Resource,
    extensions: [AshAI.Resource]

  ai do
    # Generate content with structured output
    generate :summarize do
      prompt fn article, _ ->
        "Summarize this article in 3 bullet points:\n\n#{article.body}"
      end

      output do
        attribute :bullet_points, {:array, :string}
      end
    end
  end
end

# Usage
{:ok, summary} = MyApp.Content.Article.summarize(article)
# => %{bullet_points: ["Point 1", "Point 2", "Point 3"]}
```

#### 2. Vector Search

```elixir
defmodule MyApp.Content.Document do
  use Ash.Resource,
    extensions: [AshAI.Resource]

  attributes do
    attribute :content, :string
    attribute :embedding, AshAI.Vector, dimensions: 1536
  end

  ai do
    # Generate embedding automatically
    embed :content, :embedding do
      model "text-embedding-ada-002"
    end
  end

  # Vector similarity search
  calculations do
    calculate :similarity, :float, expr(AshAI.vector_cosine_distance(embedding, ^query_embedding))
  end
end
```

#### 3. MCP Integration

```elixir
defmodule MyApp.MCP.Server do
  use AshAI.MCP.Server

  resource MyApp.Content.Document do
    # Expose resource as MCP tool
    tool :search_documents do
      description "Search documents by content"
      argument :query, :string
    end
  end
end
```

---

## Part 2: AshCloak (`ash_cloak`)

### Overview

AshCloak provides encryption at rest for Ash resource attributes using the Cloak library.

### Configuration

```elixir
defmodule MyApp.Vault do
  use Cloak.Vault,
    otp_app: :my_app
end

# In config/config.exs
config :my_app, MyApp.Vault,
  cipher: {Cloak.Ciphers.AES.GCM,
   key: Base.decode64!(System.get_env("ENCRYPTION_KEY")),
   tag: "AES.GCM.V1"}

# Start vault in application supervisor
children = [
  MyApp.Vault
]
```

### Resource Configuration

```elixir
defmodule MyApp.Accounts.User do
  use Ash.Resource,
    domain: MyApp.Accounts,
    data_layer: AshPostgres.DataLayer,
    extensions: [AshCloak.Resource]

  cloak do
    vault MyApp.Vault
    # Encrypt these attributes
    attributes [:email, :ssn, :password_hash]

    # Optional: Encrypt with additional context
    encrypted_attribute :secret do
      # Use attribute value as part of encryption context
      context fn user ->
        %{user_id: user.id}
      end
    end
  end

  attributes do
    uuid_primary_key :id
    attribute :email, :string, sensitive?: true
    attribute :ssn, :string, sensitive?: true
    attribute :password_hash, :string, sensitive?: true
    attribute :secret, :string, sensitive?: true
  end
end
```

### Migration

```elixir
defmodule MyApp.Repo.Migrations.EncryptUserData do
  use Ecto.Migration

  def change do
    # Ensure columns are binary for encrypted data
    alter table(:users) do
      modify :email, :binary
      modify :ssn, :binary
      modify :password_hash, :binary
    end
  end
end
```

### Encryption Context

Using context adds an extra layer of security:

```elixir
cloak do
  vault MyApp.Vault

  encrypted_attribute :api_key do
    # Each user's API key is encrypted with their user_id as context
    context fn user ->
      %{user_id: user.id, resource: "api_key"}
    end
  end
end
```

### Searching Encrypted Data

Encrypted fields cannot be searched directly. Options:

```elixir
# Option 1: Blind index (hash for equality checks)
cloak do
  encrypted_attribute :email do
    # Create hash for equality searches
    blind_index true
  end
end

# Now you can filter by exact email
Ash.Query.filter(User, email == ^"user@example.com")

# Option 2: Store searchable hash separately
attributes do
  attribute :email_hash, :string
  attribute :email, :string, sensitive?: true
end

changes do
  change fn changeset, _ ->
    if email = Ash.Changeset.get_change(changeset, :email) do
      Ash.Changeset.put_change(changeset, :email_hash, :crypto.hash(:sha256, email))
    end
  end
end
```

### Decryption on Load

Cloak automatically decrypts attributes when loading records:

```elixir
user = Ash.get!(MyApp.Accounts.User, id)
# email, ssn, etc. are automatically decrypted

# Access decrypted value
IO.inspect(user.email)
```

---

## Part 3: AshPaperTrail (`ash_paper_trail`)

### Overview

AshPaperTrail keeps a complete history of changes to resources, enabling:
- Audit logging
- Point-in-time recovery
- Change visualization

### Configuration

```elixir
defmodule MyApp.Audit.Version do
  use Ash.Resource,
    domain: MyApp.Audit,
    data_layer: AshPostgres.DataLayer

  attributes do
    uuid_primary_key :id
    attribute :item_type, :string
    attribute :item_id, :uuid
    attribute :event, :atom
    attribute :whodunnit, :string
    attribute :object, :map
    attribute :object_changes, :map
    attribute :created_at, :utc_datetime
  end

  indices do
    index [:item_type, :item_id]
    index [:created_at]
  end
end

defmodule MyApp.Blog.Post do
  use Ash.Resource,
    domain: MyApp.Blog,
    data_layer: AshPostgres.DataLayer,
    extensions: [AshPaperTrail.Resource]

  paper_trail do
    # Track this resource
    track true

    # Version model
    version MyApp.Audit.Version

    # Who made the change
    actor_name fn changeset ->
      changeset.context[:current_user]&.email
    end

    # Optional: Only track specific actions
    only [:create, :update, :destroy]

    # Optional: Ignore specific fields
    ignore_fields [:updated_at, :view_count]
  end
end
```

### Version Tracking

Every change creates a version record:

```elixir
# Create
post = Ash.create!(MyApp.Blog.Post, %{title: "Hello", body: "World"})
# Creates version with event: :create

# Update
Ash.update!(post, %{title: "Hello World"})
# Creates version with event: :update, object_changes shows diff

# Destroy
Ash.destroy!(post)
# Creates version with event: :destroy
```

### Querying Versions

```elixir
# Get all versions of a record
versions = AshPaperTrail.versions(post)

# Get version at point in time
version = AshPaperTrail.version_at(post, ~U[2024-01-01 12:00:00Z])

# Get the actual record at point in time
record = AshPaperTrail.reify_at(post, ~U[2024-01-01 12:00:00Z])

# Who made changes
changes = AshPaperTrail.versions(post)
|> Ash.Query.filter.whodunnit == "admin@example.com"
```

### Reverting Changes

```elixir
# Revert to previous version
previous_version = AshPaperTrail.versions(post) |> Enum.at(1)
AshPaperTrail.reify(post, previous_version)

# Revert to point in time
AshPaperTrail.revert_to(post, ~U[2024-01-01 00:00:00Z])
```

### Custom Metadata

```elixir
paper_trail do
  # Add custom metadata to versions
  meta fn changeset ->
    %{
      ip_address: changeset.context[:ip_address],
      user_agent: changeset.context[:user_agent]
    }
  end
end
```

---

## Part 4: AshRateLimiter (`ash_rate_limiter`)

### Overview

AshRateLimiter provides rate limiting for Ash actions.

### Configuration

```elixir
defmodule MyApp.Blog.Post do
  use Ash.Resource,
    domain: MyApp.Blog,
    data_layer: AshPostgres.DataLayer,
    extensions: [AshRateLimiter.Resource]

  rate_limiter do
    # Limit create action
    limit :create do
      # 5 creates per minute per actor
      max 5
      interval :timer.minutes(1)
      by :actor

      # What to do when limit exceeded
      on_limit_exceeded fn changeset ->
        {:error, "Rate limit exceeded. Try again in 1 minute."}
      end
    end

    # Limit update action
    limit :update do
      # 10 updates per hour per record
      max 10
      interval :timer.hours(1)
      by :record

      # Different limits for different roles
      with :role, :admin do
        max 100
      end
    end
  end
end
```

### Limiting Strategies

```elixir
rate_limiter do
  # By actor (user-specific)
  limit :create do
    max 5
    interval :timer.minutes(1)
    by :actor
  end

  # By IP address
  limit :search do
    max 100
    interval :timer.hours(1)
    by {:context, :ip_address}
  end

  # By resource (per-record limiting)
  limit :update do
    max 10
    interval :timer.hours(1)
    by :record
  end

  # Global limit
  limit :export do
    max 1000
    interval :timer.hours(1)
    by :global
  end
end
```

### Custom Limit Keys

```elixir
rate_limiter do
  limit :post_comment do
    max 3
    interval :timer.hours(1)

    # Custom key function
    key fn changeset ->
      # Limit per actor per post
      "#{changeset.context[:actor_id]}:#{changeset.data.post_id}"
    end
  end
end
```

### Integration with Context

```elixir
# Rate limit based on actor role
rate_limiter do
  limit :api_call do
    by {:context, :actor}

    # Premium users get higher limits
    with {:context, :plan}, :premium do
      max 1000
    end

    # Default limit
    max 100
    interval :timer.hours(1)
  end
end
```

---

## Part 5: AshSlug (`ash_slug`)

### Overview

AshSlug generates URL-friendly slugs for resources.

### Configuration

```elixir
defmodule MyApp.Blog.Post do
  use Ash.Resource,
    domain: MyApp.Blog,
    data_layer: AshPostgres.DataLayer,
    extensions: [AshSlug.Resource]

  attributes do
    uuid_primary_key :id
    attribute :title, :string
    attribute :slug, :string, public?: true
  end

  slug do
    # Generate slug from title
    from :title

    # Store in slug field
    into :slug

    # Ensure uniqueness
    unique true

    # Regenerate when source changes
    regenerate true
  end
end
```

### Slug Options

```elixir
slug do
  from :title
  into :slug

  # Custom separator (default: "-")
  separator "-"

  # Maximum length
  max_length 100

  # Only regenerate if slug is nil
  regenerate_if_nil true

  # Custom slug generation
  generate fn post ->
    # Custom logic
    Slugify.slugify(post.title, separator: "_")
  end
end
```

### Unique Slugs

```elixir
slug do
  from :title
  unique true

  # How to handle duplicates (default: append number)
  on_conflict :increment
  # or :error to raise on conflict
  # or :uuid to append UUID
end

# Results in:
# "hello-world"
# "hello-world-1"
# "hello-world-2"
```

### Usage

```elixir
# Create with auto-slug
post = Ash.create!(MyApp.Blog.Post, %{title: "Hello World!"})
# => %{title: "Hello World!", slug: "hello-world"}

# Find by slug
post = Ash.get_by!(MyApp.Blog.Post, %{slug: "hello-world"})

# Update title regenerates slug
Ash.update!(post, %{title: "Hello Beautiful World!"})
# => %{slug: "hello-beautiful-world"}
```

---

## Other Notable Extensions

### AshStateMachine

```elixir
defmodule MyApp.Order do
  use Ash.Resource,
    extensions: [AshStateMachine.Resource]

  state_machine do
    initial_states [:cart]
    default_initial_state :cart

    transitions do
      transition :checkout, from: :cart, to: :pending_payment
      transition :payment_received, from: :pending_payment, to: :processing
      transition :ship, from: :processing, to: :shipped
      transition :deliver, from: :shipped, to: :delivered
      transition :cancel, from: [:cart, :pending_payment], to: :cancelled
    end
  end

  actions do
    update :checkout do
      change transition_state(:pending_payment)
    end
  end
end
```

### AshOban

```elixir
defmodule MyApp.BackgroundJobs.ProcessOrder do
  use AshOban.Job,
    queue: :orders,
    max_attempts: 3

  resource MyApp.Order

  def perform(%{order_id: order_id}) do
    order = Ash.get!(MyApp.Order, order_id)
    # Process order
    :ok
  end
end

# In resource
defmodule MyApp.Order do
  use Ash.Resource,
    extensions: [AshOban.Resource]

  actions do
    update :process do
      change enqueue_job(MyApp.BackgroundJobs.ProcessOrder)
    end
  end
end
```

### AshMoney

```elixir
defmodule MyApp.Product do
  use Ash.Resource,
    extensions: [AshMoney.Resource]

  attributes do
    attribute :price, AshMoney.Type, currency: :USD
    attribute :discounted_price, AshMoney.Type, currency: :USD
  end

  calculations do
    calculate :savings, AshMoney.Type, expr(price - discounted_price)
  end
end
```

### AshDoubleEntry

```elixir
defmodule MyApp.Accounting.Ledger do
  use AshDoubleEntry.Ledger,
    otp_app: :my_app

  accounts do
    account :assets, type: :asset
    account :liabilities, type: :liability
    account :revenue, type: :revenue
  end
end

# Create transaction
AshDoubleEntry.transaction(ledger, fn ->
  debit(:assets, 10000)
  credit(:revenue, 10000)
end)
```

---

## Extension Best Practices

### 1. Compose Extensions

```elixir
defmodule MyApp.SecurePost do
  use Ash.Resource,
    extensions: [
      AshStateMachine,      # State management
      AshCloak.Resource,    # Encryption
      AshPaperTrail.Resource, # Audit trail
      AshSlug.Resource      # URL slugs
    ]
end
```

### 2. Order Matters

Some extensions must be loaded in specific orders:

```elixir
# Good order
extensions: [
  AshCloak.Resource,        # Encrypt first
  AshPaperTrail.Resource    # Then track changes
]

# PaperTrail will see encrypted values (correct)
```

### 3. Extension Configuration

```elixir
# Keep extension config organized
cloak do
  vault MyApp.Vault
  attributes [:email, :ssn]
end

paper_trail do
  version MyApp.Audit.Version
  actor_name & &1.context[:current_user]&.email
end

slug do
  from :title
  unique true
end
```

### 4. Testing Extensions

```elixir
test "encrypts email" do
  user = Ash.create!(MyApp.Accounts.User, %{email: "test@example.com"})

  # In database, email is encrypted
  raw_user = MyApp.Repo.one!(from u in MyApp.Accounts.User, where: u.id == ^user.id)
  assert raw_user.email != "test@example.com"

  # Loaded record is decrypted
  assert user.email == "test@example.com"
end
```

---

## Conclusion

Ash's extension ecosystem provides:

1. **Specialized Functionality**: Each extension adds specific capabilities
2. **Composability**: Multiple extensions work together seamlessly
3. **Consistency**: All extensions use the same DSL patterns
4. **Production Ready**: Battle-tested in production applications
5. **Extensible**: Build your own extensions using the same tools

The extension system is one of Ash's greatest strengths, allowing you to add complex functionality with minimal configuration.
