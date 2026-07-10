---
title: Deterministic Search — Resolution, Retrieval, and Evaluation
---

# Deterministic Search — Resolution, Retrieval, and Evaluation

RAC provides three search modes: exact ID resolution (`rac resolve`), substring search across all artifact fields (`rac find`), and live decision retrieval (`rac find --decisions`). All three are pure, deterministic, and offline — no embeddings, no LLM calls, no external index. The same query against the same corpus always returns the same results in the same order.

Source: `/home/darkvoid/Boxxed/@formulas/src.rust/src.AI/src.Wayfinder/rac-core/` — `rac/services/resolve.py`, `rac/services/eval.py`

**Aha: Search is tiered by evidence strength.** The search algorithm doesn't return a flat list of matches. Instead, it classifies each match into tiers based on where the query matched: exact ID match (strongest), filename match, title match, then body text match (weakest). This means the most relevant results always appear first, and the `--explain` flag shows exactly why each result matched — the tier, the field, and the terms.

**Aha: `find_decisions` is structurally restricted to live decisions only.** Unlike the general `find_artifacts` which searches all types, `find_decisions` filters to only Accepted, non-retired decisions. This is the deterministic "what did we decide about X" query — an agent consulting this tool gets the team's settled decisions, not every document that happens to mention the topic.

## Repository Index

Source: `rac-core/src/rac/services/index.py` — `build_repository_index()`

All search operations start by building a repository index — a single pass over the corpus that extracts each artifact's identity, type, title, and path:

```python
def build_repository_index(root: str, *, recursive: bool = True) -> RepositoryIndex:
    """Walk the corpus, building an index of all artifacts."""
    entries = list(walk_corpus(root, recursive=recursive))
    return index_from_corpus(root, entries, recursive=recursive)

def index_from_corpus(
    root: str, entries: list[CorpusEntry], *, recursive: bool = True
) -> RepositoryIndex:
    artifacts = []
    for entry in entries:
        spec = spec_for_classified(entry.product)
        artifact_id = artifact_identifier(entry.product, spec, entry.path)
        # Also collect legacy identifiers for migration compatibility
        all_ids = artifact_identifiers(entry.product, spec, entry.path)
        artifacts.append(IndexEntry(
            id=artifact_id,
            all_ids=all_ids,
            type=classify(entry.product).type,
            title=entry.product.title,
            path=entry.path,
        ))
    return RepositoryIndex(artifacts=artifacts)
```

The index is built fresh on every call — no persistent index, no file watcher, no incremental updates. This means the index always reflects the current state of the repository.

## ID Resolution

Source: `rac-core/src/rac/services/resolve.py` — `resolve_in_index()`

ID resolution is an exact match against the index, respecting both canonical and legacy identifiers:

```python
def resolve_in_index(
    entries: list[IndexEntry], artifact_id: str
) -> ResolutionResult:
    """Resolve artifact_id against the index. Returns resolved, not-found, or duplicate."""
    matches = [e for e in entries if artifact_id in e.all_ids]
    if len(matches) == 1:
        return ResolutionResult(outcome=OUTCOME_RESOLVED, artifact=matches[0])
    if len(matches) == 0:
        return ResolutionResult(outcome=OUTCOME_NOT_FOUND, query=artifact_id)
    return ResolutionResult(
        outcome=OUTCOME_DUPLICATE,
        query=artifact_id,
        duplicate_paths=[m.path for m in matches],
    )
```

Three outcomes are possible:
- **Resolved:** Exactly one artifact matches the ID.
- **Not Found:** No artifact matches the ID.
- **Duplicate:** Multiple artifacts share the same ID (a corpus integrity issue).

## Substring Search

Source: `rac-core/src/rac/services/resolve.py` — `search_index()`

Substring search operates on multiple tiers of evidence:

```python
def search_index(
    entries: list[IndexEntry],
    query: str,
    artifact_type: str | None = None,
) -> SearchResult:
    """Search artifacts by substring match across multiple fields."""
    query_lower = query.lower()
    results: list[SearchMatch] = []
    for entry in entries:
        if artifact_type and entry.type != artifact_type:
            continue
        # Tier 1: exact ID match (case-insensitive)
        if any(query_lower == aid.lower() for aid in entry.all_ids):
            results.append(SearchMatch(entry=entry, tier="id"))
        # Tier 2: filename contains query
        elif query_lower in Path(entry.path).stem.lower():
            results.append(SearchMatch(entry=entry, tier="filename"))
        # Tier 3: title contains query
        elif entry.title and query_lower in entry.title.lower():
            results.append(SearchMatch(entry=entry, tier="title"))
        # Tier 4: body text contains query (search_sections)
        elif any(query_lower in line.lower()
                 for ss in entry.search_sections
                 for line in ss.lines):
            results.append(SearchMatch(entry=entry, tier="body"))
    return SearchResult(matches=results, query=query)
```

Results are ordered by tier: ID matches first, then filename, then title, then body. Within a tier, results are in corpus walk order (sorted by path).

The `--explain` flag adds evidence details to each match:

```python
def render_find_json(result: SearchResult, *, explain: bool = False) -> str:
    matches = []
    for m in result.matches:
        match = {"id": m.entry.id, "type": m.entry.type, "title": m.entry.title, "path": m.entry.path}
        if explain:
            match["evidence"] = {"tier": m.tier, "field": m.field, "terms": m.terms}
        matches.append(match)
    return json.dumps({"matches": matches, "query": result.query})
```

## Live Decision Retrieval

Source: `rac-core/src/rac/services/resolve.py` — `find_decisions()`

The `find_decisions` function is a specialized search restricted to live decisions:

```python
def find_decisions(
    root: str, topic: str, *, recursive: bool = True
) -> DecisionSearchResult:
    """Find live (Accepted, non-retired) decisions about a topic."""
    entries = list(walk_corpus(root, recursive=recursive))
    index = index_from_corpus(root, entries, recursive=recursive)
    # Filter to decisions only
    decisions = [e for e in index.artifacts if e.type == "decision"]
    # Further filter to live (Accepted, not Superseded/Deprecated)
    live = [d for d in decisions if is_live_decision(d)]
    # Search within live decisions
    matches = search_index(live, topic)
    return DecisionSearchResult(matches=matches, topic=topic, filter="live-decisions")
```

A decision is "live" when its status is "Accepted" (or absent, which defaults to Accepted). Superseded and Deprecated decisions are excluded because they represent retired knowledge.

**Aha: The `--decisions` flag in `rac find` is mutually exclusive with `--type`.** This is enforced at the argument parser level because `--decisions` implies both the type filter (decision) and the status filter (live). Passing both would be redundant and potentially contradictory.

Source: `rac-core/src/rac/cli.py:2012-2025`

```python
find_scope = p_find.add_mutually_exclusive_group()
find_scope.add_argument("--type", help="Only match artifacts of this type...")
find_scope.add_argument(
    "--decisions", action="store_true",
    help="Only live decisions (Accepted, non-retired) — the 'what did we "
         "decide about X / is X ruled out' query (ADR-067)."
)
```

## Evaluation Benchmark

Source: `rac-core/src/rac/services/eval.py` — `run_eval()`

The `rac eval` command runs a retrieval benchmark against a fixture corpus:

```python
def run_eval(root: str, queries_path: str) -> Scorecard:
    """Score retrieval against the fixture benchmark."""
    queries = load_queries(queries_path)
    metrics = []
    for query in queries:
        result = search_index(build_repository_index(root).artifacts, query.text)
        metric = compute_metric(result, query.expected)
        metrics.append(metric)
    return Scorecard(metrics=metrics)
```

The eval has three modes:
1. **Default:** Report the scorecard (exit 0).
2. **`--check`:** Gate CI — exit 1 if any metric is below its floor or below baseline minus tolerance.
3. **`--update-baseline`:** Overwrite the baseline with current metrics (human-only).

Source: `rac-core/src/rac/cli.py:949-982` — `cmd_eval()`

```python
def cmd_eval(args: argparse.Namespace) -> int:
    scorecard = eval_service.run_eval(args.root, args.queries)
    if args.update_baseline:
        Path(args.baseline).write_text(
            eval_service.render_metrics_json(scorecard.metrics) + "\n"
        )
        return EXIT_OK
    if args.check:
        baseline = eval_service.load_baseline(args.baseline)
        config = eval_service.load_config(args.config)
        failures = eval_service.evaluate_gate(scorecard.metrics, baseline, config)
        if failures:
            for failure in failures:
                print(failure.render())
            return EXIT_VALIDATION_FAILED
        print("rac eval: gate PASS")
        return EXIT_OK
    # Default: report
    ...
```

The gate config defines floors (minimum acceptable values) and tolerances (how far below baseline is acceptable before failing):

```json
{
  "floors": {"precision@1": 0.8, "recall@5": 0.9},
  "tolerance": 0.05
}
```

## Cross-References

- [RAC Architecture](01-architecture.md) — layer diagram, identity resolution
- [Knowledge Graph](07-knowledge-graph.md) — corpus walking, index building
- [Data Flow End-to-End](09-data-flow.md) — how search queries flow through the system
- [Error Handling](10-error-handling.md) — not-found, duplicate, and gate failure handling

## Next Document

Continue to [Data Flow End-to-End](09-data-flow.md) for the complete request lifecycle from CLI invocation to output rendering.
