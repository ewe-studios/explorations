---
title: Knowledge Graph — Artifact Relationships and Traversal
---

# Knowledge Graph — Artifact Relationships and Traversal

The RAC knowledge graph is a typed, directed graph built from Markdown artifact files. Each artifact is a node, and explicit relationship sections (`## Related Decisions`, `## Supersedes`, etc.) form directed edges. The graph powers `rac relationships`, `rac doctor`, `rac portfolio`, `rac export --graph`, and the MCP `get_related` tool.

Source: `/home/darkvoid/Boxxed/@formulas/src.rust/src.AI/src.Wayfinder/rac-core/` — primarily `rac/services/relationships.py`, `rac/core/identity.py`, `rac/core/corpus.py`

**Aha: Relationships are explicit, not inferred.** Unlike systems that use embeddings or LLM-based semantic matching to "discover" connections between documents, RAC only recognizes relationships that the author explicitly writes as `##` sections. This means the graph is deterministic and auditable — every edge has a human author and a specific section in a specific file. There are no false positives from over-eager semantic matching.

**Aha: The relationship graph is built from a single corpus walk.** Functions like `relationships_from_corpus()` walk the directory once, parse each file, extract relationship sections, and build the complete graph in memory. There is no incremental indexing or caching — the graph lives and dies within the call (ADR-032). This means the graph always reflects the current state of the repository, but at the cost of re-parsing on every request.

## Graph Construction

Source: `rac-core/src/rac/services/relationships.py` — `relationships_from_corpus()`

```python
def relationships_from_corpus(entries: list[CorpusEntry]) -> Relationships:
    """Build the complete relationship graph from a list of corpus entries."""
    rels = Relationships()
    for entry in entries:
        product = parse(entry.path)  # parse each file
        for section_name, body in product.sections.items():
            if section_name in RELATIONSHIP_SECTIONS:
                # Extract referenced artifact IDs from section body
                refs = extract_references(body)
                for ref in refs:
                    rels.add_edge(
                        source=entry.path,
                        target_id=ref,
                        relationship=section_name,
                    )
    return rels
```

The `RELATIONSHIP_SECTIONS` set includes: `related requirements`, `related decisions`, `related roadmaps`, `related prompts`, `related designs`, `related tickets`, `verified by`, and `supersedes`.

## Corpus Walking

Source: `rac-core/src/rac/core/corpus.py` — `walk_corpus()`

```python
def walk_corpus(root: str, *, recursive: bool = True) -> Iterator[CorpusEntry]:
    """Walk a directory, yielding CorpusEntry for each parseable Markdown file."""
    root_path = Path(root)
    patterns = ["*.md", "*.markdown"]
    for pattern in patterns:
        glob_method = root_path.rglob if recursive else root_path.glob
        for path in sorted(glob_method(pattern)):
            if _should_skip(path):
                continue
            product = parse_file(str(path))
            if product is not None:
                yield CorpusEntry(path=str(path), product=product)
```

The walker skips files in hidden directories (`.git`, `.rac`, `node_modules`), files that fail to parse, and files with unrecognized types. Each yielded `CorpusEntry` carries the parsed `Product` AST, so downstream consumers don't re-parse.

## Relationship Resolution

Source: `rac-core/src/rac/services/relationships.py` — `outgoing_references()`, `incoming_references()`

The graph supports two query directions:

- **Outgoing:** Given a file path, return all artifacts it references (by section).
- **Incoming:** Given a file path, return all artifacts that reference it (reverse lookup).

```python
def outgoing_references(rels: Relationships, source_path: str) -> SectionRefs:
    """Return all outgoing references from source_path, grouped by section."""
    # ...

def incoming_references(
    rels: Relationships,
    identity_by_path: dict[str, tuple[str, str, str]],
    target_path: str,
) -> RefItems:
    """Return all incoming references to target_path, with resolved identities."""
    # ...
```

The `incoming_references` function requires an `identity_by_path` mapping because edges store target *IDs*, but the caller needs the resolved *identity* (type, title, path) of the target artifact. This mapping comes from `build_repository_index()`.

## Multi-Hop Neighborhood

Source: `rac-core/src/rac/services/relationships.py` — `neighborhood()`

The `neighborhood()` function performs a bounded BFS traversal from a starting artifact:

```python
def neighborhood(
    rels: Relationships,
    identity_by_path: dict[str, tuple[str, str, str]],
    start_path: str,
    depth: int = 2,
) -> Neighborhood:
    """Bounded BFS from start_path, returning artifacts at each hop distance."""
    visited = {start_path}
    queue = [(start_path, 0)]
    nodes: list[NeighborhoodNode] = []
    truncated = False
    while queue:
        path, hops = queue.pop(0)
        if hops >= depth:
            continue
        # Find both incoming and outgoing neighbors
        for neighbor_path in get_neighbors(path, rels, identity_by_path):
            if neighbor_path not in visited:
                visited.add(neighbor_path)
                nodes.append(NeighborhoodNode(
                    id=..., type=..., title=..., path=neighbor_path, hops=hops + 1,
                ))
                queue.append((neighbor_path, hops + 1))
                if len(nodes) >= MAX_TRAVERSAL_DEPTH:
                    truncated = True
                    break
    return Neighborhood(nodes=nodes, truncated=truncated)
```

The traversal is bounded by `MAX_TRAVERSAL_DEPTH` (constant in `rac.core.limits`) to prevent runaway graph walks on large corpora. When the cap is hit, `truncated=True` is returned so the caller knows the result is incomplete.

**Aha: Neighborhood traversal explores both incoming and outgoing edges.** Unlike a simple "who references me" query, the neighborhood function follows edges in both directions, so it discovers the full transitive context around an artifact. An artifact that references ADR-001, which in turn references REQ-005, shows both hops — making it possible to trace the full chain from requirement to decision to design.

## Edge Types

The knowledge graph has 8 distinct edge types, each with a specific semantic meaning:

| Edge Type | Direction | Meaning | Target Scope |
|-----------|-----------|---------|-------------|
| `related requirements` | Outgoing | This artifact depends on these requirements | In-corpus artifacts |
| `related decisions` | Outgoing | This artifact references these decisions | In-corpus artifacts |
| `related roadmaps` | Outgoing | This artifact contributes to these roadmaps | In-corpus artifacts |
| `related prompts` | Outgoing | This artifact references these prompts | In-corpus artifacts |
| `related designs` | Outgoing | This artifact references these designs | In-corpus artifacts |
| `related tickets` | Outgoing | This artifact traces to external tickets | External (GitHub, Jira, etc.) |
| `verified by` | Outgoing | This artifact is verified by these tests | External file paths |
| `supersedes` | Outgoing | This artifact replaces the target | In-corpus artifacts |

## Graph Export

Source: `rac-core/src/rac/services/export.py` — `build_graph_export()`

The `rac export --graph` command produces a typed JSON graph for external graph backends:

```python
def build_graph_export(root: str) -> GraphExport:
    entries = list(walk_corpus(root, recursive=True))
    relationships = relationships_from_corpus(entries)
    index = index_from_corpus(root, entries, recursive=True)

    nodes = [
        GraphNode(id=e.id, type=e.type, title=e.title, path=e.path)
        for e in index.artifacts
    ]
    edges = [
        GraphEdge(
            source=rel.source_path,
            target=rel.target_id,
            relationship=rel.section,
            direction="outgoing",
        )
        for rel in relationships.all()
    ]
    return GraphExport(nodes=nodes, edges=edges)
```

The output is a JSON document with `nodes` (id, type, title, path) and `edges` (source, target, relationship kind, direction). This is consumed by graph databases, GraphRAG backends, and the proofkeeper coverage system (which uses it to identify capabilities needing verification).

## Validation

Source: `rac-core/src/rac/services/relationships.py` — `validate_relationships()`

Relationship validation resolves every reference against the live corpus and reports:

- **Broken:** Target artifact not found in the corpus.
- **Ambiguous:** Multiple artifacts match the target ID.
- **Self-reference:** An artifact references itself.
- **Duplicate:** The same target is referenced multiple times in one section.

```python
def validate_relationships(root: str, *, recursive: bool = True) -> ValidationReport:
    entries = list(walk_corpus(root, recursive=recursive))
    relationships = relationships_from_corpus(entries)
    index = index_from_corpus(root, entries, recursive=recursive)
    # For each relationship edge, resolve the target ID against the index
    # Report broken, ambiguous, self-referencing, and duplicate findings
    ...
```

Validation uses the same identity resolution as the rest of the system (`resolve_in_index()`), so it respects canonical frontmatter IDs, legacy `## ID` sections, and filename-derived identifiers.

## Hub Detection

Source: `rac-core/src/rac/services/doctor.py` — `diagnose()`

The `rac doctor` command includes hub detection: artifacts with more than `DEFAULT_HUB_THRESHOLD` (configurable via `--hub-threshold`) resolved relationship edges are flagged as high-fan-out hubs. These are informational warnings (exit 0), not errors, because hubs are a natural consequence of important artifacts that many others reference.

```python
# In doctor.diagnose():
hub_threshold = args.hub_threshold  # default: DEFAULT_HUB_THRESHOLD
for path, count in relationship_counts.items():
    if count > hub_threshold:
        findings.append(Finding(
            severity="warning",
            code="high-fan-out",
            message=f"{path} has {count} relationship edges (threshold: {hub_threshold})",
        ))
```

## Cross-References

- [RAC Architecture](01-architecture.md) — layer diagram, module map, Product AST
- [Deterministic Search](08-deterministic-search.md) — resolution, find, eval algorithms
- [Data Flow End-to-End](09-data-flow.md) — how relationships flow through the system
- [Error Handling](10-error-handling.md) — validation errors, broken references

## Next Document

Continue to [Deterministic Search](08-deterministic-search.md) for the search and resolution algorithms that power `rac find`, `rac resolve`, and `rac eval`.
