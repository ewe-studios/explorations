# ActiveRecord -- Project Spec

## Source Codebase

**Location:** `/Users/darkvoid/.gems/gems/activerecord-8.1.4/`
**Language:** Ruby (Rails framework gem)
**Version:** 8.1.4
**Part of:** Ruby on Rails. The Object-Relational Mapper (ORM): a model is data
plus behavior mapped to a database table, and a lazy immutable `Relation`
chains into SQL through the Arel AST.

## What ActiveRecord Is

Two halves. The model: `ActiveRecord::Base` reads the table's columns into typed
attributes, generates getters/setters, and attaches macros (`has_many`,
`validates`, `before_save`). The query: `Model.where(...).order(...)` returns a
lazy `Relation`; nothing touches the database until you iterate or count, and
Arel turns the chained nodes into parameterized SQL. Connection adapters and a
pool reach the database; GlobalID links models to jobs.

## Documentation Structure

```
ruby/activerecord/
├── spec.md                          ← THIS FILE
└── markdown/
    ├── README.md                    ← index
    ├── 00-overview.md
    ├── 01-connection-and-adapters.md
    ├── 02-sql-generation-arel.md
    ├── 03-the-model.md
    ├── 04-query-interface.md
    ├── 05-persistence.md
    ├── 06-associations.md
    ├── 07-callbacks-and-validations.md
    ├── 08-migrations-and-schema.md
    ├── 09-attribute-methods-and-types.md
    ├── 10-scoping-and-eager-loading.md
    ├── 11-query-cache-and-pool.md
    ├── 12-the-nice-ruby-experience.md
    └── 13-globalid-and-activejob-integration.md
```

## Quality Requirements (Iron Rules)

1. At least two Mermaid diagrams, three code snippets, one `## The Aha Moment`
   (opening with a bolded `**Aha:**`), and two cross-references per document.
2. H1 first line `# NN -- Title: subtitle`, no YAML frontmatter.
3. Footer chain 00→…→13→README; no orphan pages.
4. ASD-STE100 Simplified Technical English; abbreviations spelled out at first use.
5. Real source code snippets with path + approximate line.
6. HTML via `/Users/darkvoid/Dev/explorations/build.py activerecord`.

## Verification

Authored by agents reading the full gem source; verified by independent agents
(fresh-reader review + grandfather review against the gem source). See the
`ajdc` spec for the same process applied to the sibling exploration.
