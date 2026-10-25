# Awesome Opus 5.5 Video -- Documentation Index

A deep documentation of `awesome-opus-5.5-video`, a curated catalog of 2,094
videos, motion graphics, 3D scenes and games made with the Claude 5.5 model
family. Each work is an X (formerly Twitter) post with 5,000 or more views.
The catalog ships two surfaces: human-readable Markdown (two README files plus
eight per-category lists in English and Chinese) and one machine-readable file
(`cases.json`) that holds every work as a structured record. This index covers
all nine documents, from the inclusion rule to the daily sync pipeline.

## The Catalog

- [Overview](00-overview.md) — what the catalog is, its purpose, its audience, and the two surfaces
- [Inclusion Criteria](01-inclusion-criteria.md) — the 5,000-views threshold, the native-video rule, the creator's-post rule, and the exclusions
- [Model Family](02-model-family.md) — Opus 5.5, Sonnet 5.5, Haiku 5.5, Fable 5.5, and how their counts are attributed

## The Data

- [Schema](03-schema.md) — `cases.json` field-by-field: the eight top-level keys and the eighteen case fields
- [Categories](04-categories.md) — the eight categories, their counts, and how `cases/` is organized
- [Prompts](05-prompts.md) — full versus brief prompts, the four prompt sources, and the verbatim rule

## The Languages and the Machine

- [Bilingual Mirror](06-bilingual-mirror.md) — the English / Chinese file pairs and the `language` field
- [Maintenance](07-maintenance.md) — the daily GitHub Actions sync, the upstream repository, and the 500-work guard
- [Contributing](08-contributing.md) — how to submit a work, correct attribution, or remove a work

## Source Repository

| Item | Value |
|---|---|
| Local source | `/Users/darkvoid/GitSources/src.Designer/awesome-opus-5.5-video` |
| Upstream data source | `zhuyansen/jasonzhu.ai` |
| Curator | JasonZhu.AI (https://jasonzhu.ai) |
| Online browse (English) | https://jasonzhu.ai/en/prompts/claude-opus-5-5 |
| Online browse (Chinese) | https://jasonzhu.ai/zh/prompts/claude-opus-5-5 |
