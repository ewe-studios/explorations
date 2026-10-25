# ActiveRecord: a deep technical exploration

A fourteen-document walk through the ActiveRecord 8.1 source. Each document stands alone and links forward and backward, so the series can be read in order or by topic.

- [00-overview.md](./00-overview.md) — The whole map: what Active Record is and the layers it is built from.
- [01-connection-and-adapters.md](./01-connection-and-adapters.md) — How connections are configured and how adapters speak each database dialect.
- [02-sql-generation-arel.md](./02-sql-generation-arel.md) — How queries become Structured Query Language through the Arel abstract syntax tree.
- [03-the-model.md](./03-the-model.md) — How a Ruby class binds to a table and becomes a model.
- [04-query-interface.md](./04-query-interface.md) — How the relation builds, chains, and executes queries.
- [05-persistence.md](./05-persistence.md) — How records are created, updated, and destroyed.
- [06-associations.md](./06-associations.md) — How models connect with `belongs_to`, `has_many`, and the other association macros.
- [07-callbacks-and-validations.md](./07-callbacks-and-validations.md) — The model life cycle: when code runs and when a save is refused.
- [08-migrations-and-schema.md](./08-migrations-and-schema.md) — Evolving the database in Ruby, with reversibility and `schema.rb`.
- [09-attribute-methods-and-types.md](./09-attribute-methods-and-types.md) — From column to Ruby object: types, `serialize`, `store`, `enum`, and dirty tracking.
- [10-scoping-and-eager-loading.md](./10-scoping-and-eager-loading.md) — Narrowing queries with scopes and avoiding N+1 with eager loading.
- [11-query-cache-and-pool.md](./11-query-cache-and-pool.md) — Sharing results and sockets: the query cache, the connection pool, and async queries.
- [12-the-nice-ruby-experience.md](./12-the-nice-ruby-experience.md) — A synthesis of how the layers combine into a DSL that reads like English.
- [13-globalid-and-activejob-integration.md](./13-globalid-and-activejob-integration.md) — Passing records to background jobs with GlobalID and Active Job.
