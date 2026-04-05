# OpenSpace Skill Registry and Discovery System - Deep Dive

## Overview

OpenSpace's Skill Registry and Discovery System is a sophisticated two-stage retrieval pipeline that combines **BM25 lexical matching**, **embedding-based semantic search**, and **LLM-based selection** to discover, rank, and inject skills into agent context. The system supports both local skills (file-system based) and cloud skills (remote platform), with hybrid ranking and quality-aware filtering.

```
┌─────────────────────────────────────────────────────────────────────────┐
│                    SKILL DISCOVERY & INJECTION PIPELINE                 │
├─────────────────────────────────────────────────────────────────────────┤
│                                                                         │
│  ┌──────────────┐    ┌──────────────┐    ┌──────────────────────────┐  │
│  │   Local      │    │    Cloud     │    │    Quality Filter        │  │
│  │   Skills     │    │   Skills     │    │  (fallback rates, etc.)  │  │
│  │  (FS scan)   │    │  (API fetch) │    │                          │  │
│  └──────┬───────┘    └──────┬───────┘    └────────────┬─────────────┘  │
│         │                   │                          │                │
│         └───────────────────┼──────────────────────────┘                │
│                             │                                           │
│                    ┌────────▼────────┐                                  │
│                    │  Stage 1: BM25  │  (Lexical rough-rank)           │
│                    │  Rough-Rank     │                                  │
│                    └────────┬────────┘                                  │
│                             │                                           │
│                    ┌────────▼────────┐                                  │
│                    │  Stage 2:       │  (Semantic re-rank)             │
│                    │  Embedding      │                                  │
│                    │  Re-Rank        │                                  │
│                    └────────┬────────┘                                  │
│                             │                                           │
│                    ┌────────▼────────┐                                  │
│                    │  Stage 3:       │  (LLM plan-then-select)         │
│                    │  LLM Selection  │                                  │
│                    └────────┬────────┘                                  │
│                             │                                           │
│                    ┌────────▼────────┐                                  │
│                    │  Context        │  (Full skill content +          │
│                    │  Injection      │   skill directory path)         │
│                    └─────────────────┘                                  │
│                                                                         │
└─────────────────────────────────────────────────────────────────────────┘
```

---

## 1. Skill Registry Architecture

### 1.1 SkillMeta Structure

The fundamental unit of skill discovery is `SkillMeta`, a lightweight dataclass that captures essential skill identity:

```python
@dataclass
class SkillMeta:
    """Metadata for a discovered skill.

    ``skill_id`` is the globally unique identifier used throughout the
    system — LLM prompts, database, evolution, and selection all
    reference this field.
    """

    skill_id: str          # Unique — persisted in .skill_id sidecar
    name: str              # Human-readable name (from frontmatter or dirname)
    description: str
    path: Path             # Absolute path to SKILL.md
```

**Key Design Decisions:**

| Field | Purpose | Portability |
|-------|---------|-------------|
| `skill_id` | Global unique identifier | Survives directory moves, machine changes |
| `name` | Human-readable identifier | Can change across versions |
| `description` | One-line summary | Used in embedding text |
| `path` | Filesystem location | Points to `SKILL.md` |

### 1.2 Skill Directory Scanning

The registry scans configured directories to discover skills:

```python
def discover(self) -> List[SkillMeta]:
    """Scan all skill_dirs and populate the registry.

    Each skill is a sub-directory containing a ``SKILL.md`` file.
    The ``skill_id`` is read from the ``.skill_id`` sidecar (created
    automatically on first discovery). Two skills with the same
    ``name`` in different directories get different IDs and can
    coexist in the registry and database.
    """
    self._skills.clear()
    self._content_cache.clear()

    for skill_dir in self._skill_dirs:
        if not skill_dir.exists():
            logger.debug(f"Skill dir does not exist, skipping: {skill_dir}")
            continue

        for entry in sorted(skill_dir.iterdir()):
            if not entry.is_dir():
                continue
            skill_file = entry / "SKILL.md"
            if not skill_file.exists():
                continue

            try:
                content = skill_file.read_text(encoding="utf-8")

                # Safety check on skill content
                safety_flags = check_skill_safety(content)
                if not is_skill_safe(safety_flags):
                    logger.warning(
                        f"BLOCKED skill {entry.name}: "
                        f"safety flags {safety_flags}"
                    )
                    continue

                meta = self._parse_skill(entry.name, entry, skill_file, content)
                sid = meta.skill_id

                if sid in self._skills:
                    logger.debug(f"Skill '{sid}' already discovered, skipping {skill_file}")
                    continue

                self._skills[sid] = meta
                self._content_cache[sid] = content
```

**Discovery Flow:**

```
┌────────────────────────────────────────────────────────────────────┐
│                     DISCOVERY PIPELINE                              │
├────────────────────────────────────────────────────────────────────┤
│                                                                    │
│  1. Iterate skill_dirs (priority order)                            │
│     │                                                              │
│     ▼                                                              │
│  2. For each subdirectory:                                         │
│     ├── Check for SKILL.md                                         │
│     ├── Read content                                               │
│     ├── Safety check (block malware, suspicious keywords)          │
│     ├── Parse frontmatter (name, description)                      │
│     ├── Read/create .skill_id sidecar                              │
│     └── Add to registry (skill_id -> SkillMeta)                    │
│                                                                    │
│  3. Return list of discovered SkillMeta objects                    │
│                                                                    │
└────────────────────────────────────────────────────────────────────┘
```

### 1.3 .skill_id Sidecar Files

Skill identity is managed through a **sidecar file pattern**:

```python
SKILL_ID_FILENAME = ".skill_id"

def _read_or_create_skill_id(name: str, skill_dir: Path) -> str:
    """Read ``skill_id`` from ``.skill_id`` sidecar, or create one.

    The sidecar file is a single-line plain-text file containing only
    the ``skill_id`` string.  It lives alongside ``SKILL.md`` inside
    the skill directory.

    First call (no file): generates ``{name}__imp_{uuid8}`` and writes it.
    Subsequent calls: reads and returns the existing ID.
    """
    id_file = skill_dir / SKILL_ID_FILENAME
    if id_file.exists():
        try:
            existing = id_file.read_text(encoding="utf-8").strip()
            if existing:
                return existing
        except OSError:
            pass  # fall through to generate

    # Generate a new ID and persist
    new_id = f"{name}__imp_{uuid.uuid4().hex[:8]}"
    try:
        id_file.write_text(new_id + "\n", encoding="utf-8")
        logger.debug(f"Created .skill_id for '{name}': {new_id}")
    except OSError as e:
        logger.warning(f"Cannot write {id_file}: {e} — ID will not persist across restarts")
    return new_id
```

**Skill ID Naming Conventions:**

| Pattern | Meaning | Example |
|---------|---------|---------|
| `{name}__imp_{uuid8}` | Imported skill | `weather__imp_a1b2c3d4` |
| `{name}__v{gen}_{uuid8}` | Evolved skill (versioned) | `weather__v2_a1b2c3d4` |
| `{name}__clo_{uuid8}` | Cloud-uploaded skill | `weather__clo_a1b2c3d4` |

**Portability Guarantees:**

- **Survives directory moves**: ID is stored locally, not derived from path
- **Survives machine changes**: ID travels with the skill directory
- **Deterministic**: Once created, ID is stable across restarts
- **Collision-resistant**: UUID-based suffix prevents conflicts

### 1.4 Skill Identity and Portability

The sidecar pattern enables several critical features:

1. **Hot-Reload**: Skills can be registered at runtime without restarting
2. **Evolution Tracking**: New versions maintain lineage through skill_id
3. **Cloud Sync**: Local skills can be uploaded and matched to cloud records
4. **Deduplication**: Same skill in multiple directories gets unique IDs

```python
def register_skill_dir(self, skill_dir: Path) -> Optional[SkillMeta]:
    """Register a single skill directory (hot-reload).

    Safety: applies ``check_skill_safety`` / ``is_skill_safe`` filtering.

    Args:
        skill_dir: Path to a directory containing ``SKILL.md``.

    Returns:
        :class:`SkillMeta` if newly registered or already present,
        ``None`` if the directory is invalid or the skill fails safety checks.
    """
```

---

## 2. SKILL.md Format

### 2.1 YAML Frontmatter Structure

Skills follow the official SKILL.md format with strict frontmatter requirements:

```markdown
---
name: Weather Forecast Guide
description: Step-by-step workflow for getting weather forecasts using wttr.in
---

# Weather Forecast Guide

## Overview

This skill provides instructions for obtaining weather forecasts...

## Procedure

1. Use curl to query wttr.in...
```

**Frontmatter Rules:**

```python
_FRONTMATTER_RE = re.compile(r"^---\n(.*?)\n---", re.DOTALL)

def parse_frontmatter(content: str) -> Dict[str, Any]:
    """Parse YAML frontmatter into a flat dict.

    Simple line-by-line parser (no PyYAML dependency).
    Handles both quoted and unquoted values.
    Returns ``{}`` if no valid frontmatter is found.
    """
    if not content.startswith("---"):
        return {}
    match = _FRONTMATTER_RE.match(content)
    if not match:
        return {}
    fm: Dict[str, Any] = {}
    for line in match.group(1).split("\n"):
        if ":" in line:
            key, value = line.split(":", 1)
            key = key.strip()
            if key:
                fm[key] = _yaml_unquote(value.strip())
    return fm
```

**Required Fields:**

| Field | Type | Description |
|-------|------|-------------|
| `name` | string | Human-readable skill name |
| `description` | string | One-line summary for search/selection |

### 2.2 Markdown Body

The body contains skill instructions in plain markdown format. Common sections include:

- **Overview**: What the skill does
- **Prerequisites**: Required tools/APIs
- **Procedure**: Step-by-step instructions
- **Examples**: Usage examples
- **Troubleshooting**: Common issues and fixes

### 2.3 Safety Checking

Skills are validated against safety rules before loading:

```python
_SAFETY_RULES = [
    ("blocked.malware",         re.compile(r"(ClawdAuthenticatorTool)", re.IGNORECASE)),
    ("suspicious.keyword",      re.compile(r"(malware|stealer|phish|phishing|keylogger)", re.IGNORECASE)),
    ("suspicious.secrets",      re.compile(r"(api[-_ ]?key|token|password|private key|secret)", re.IGNORECASE)),
    ("suspicious.crypto",       re.compile(r"(wallet|seed phrase|mnemonic|crypto)", re.IGNORECASE)),
    ("suspicious.webhook",      re.compile(r"(discord\.gg|webhook|hooks\.slack)", re.IGNORECASE)),
    ("suspicious.script",       re.compile(r"(curl[^\n]+\|\s*(sh|bash))", re.IGNORECASE)),
    ("suspicious.url_shortener", re.compile(r"(bit\.ly|tinyurl\.com|t\.co|goo\.gl|is\.gd)", re.IGNORECASE)),
]

_BLOCKING_FLAGS = frozenset({"blocked.malware"})

def check_skill_safety(text: str) -> List[str]:
    """Check *text* against safety rules, return list of triggered flag names.

    Returns an empty list if no rules match (= safe).
    """
    return [flag for flag, pat in _SAFETY_RULES if pat.search(text)]

def is_skill_safe(flags: List[str]) -> bool:
    """Return True if *flags* contain no blocking flag.

    ``suspicious.*`` flags are informational (logged / attached to search
    results) but do NOT block.  Only ``blocked.*`` flags cause rejection.
    """
    return not any(f in _BLOCKING_FLAGS for f in flags)
```

**Safety Flag Categories:**

| Category | Flags | Action |
|----------|-------|--------|
| `blocked.*` | `blocked.malware` | **REJECT** skill |
| `suspicious.*` | `suspicious.keyword`, `suspicious.secrets`, etc. | Log warning, allow |

### 2.4 Parsing Implementation

```python
def get_frontmatter_field(content: str, field_name: str) -> Optional[str]:
    """Extract a single field value from YAML frontmatter.

    Returns ``None`` if the field is absent or content has no frontmatter.
    """
    if not content.startswith("---"):
        return None
    match = _FRONTMATTER_RE.match(content)
    if not match:
        return None
    for line in match.group(1).split("\n"):
        if ":" in line:
            key, value = line.split(":", 1)
            if key.strip() == field_name:
                return _yaml_unquote(value.strip())
    return None


def set_frontmatter_field(content: str, field_name: str, value: str) -> str:
    """Set (or insert) a field in YAML frontmatter.

    Values containing YAML special characters (``:``, ``#``, etc.) are
    automatically double-quoted to produce valid YAML.

    If *content* has no frontmatter, a new one is prepended.
    """
    quoted = _yaml_quote(value)
    if not content.startswith("---"):
        return f"---\n{field_name}: {quoted}\n---\n{content}"

    match = _FRONTMATTER_RE.match(content)
    if not match:
        return content

    fm_text = match.group(1)
    new_line = f"{field_name}: {quoted}"
    found = False
    new_lines = []
    for line in fm_text.split("\n"):
        if ":" in line and line.split(":", 1)[0].strip() == field_name:
            new_lines.append(new_line)
            found = True
        else:
            new_lines.append(line)
    if not found:
        new_lines.append(new_line)

    new_fm = "\n".join(new_lines)
    return f"---\n{new_fm}\n---{content[match.end():]}"
```

---

## 3. Skill Discovery Pipeline

### 3.1 Directory Scanning and Loading

```python
class SkillRegistry:
    def __init__(self, skill_dirs: Optional[List[Path]] = None) -> None:
        self._skill_dirs: List[Path] = skill_dirs or []
        self._skills: Dict[str, SkillMeta] = {}     # skill_id -> SkillMeta
        self._content_cache: Dict[str, str] = {}     # skill_id -> raw SKILL.md content
        self._discovered = False
        self._ranker: Optional[SkillRanker] = None   # lazy-init on first use
```

**Directory Priority:**

- Earlier entries have higher priority
- A skill in the first dir shadows one with the same name in later dirs
- All internal maps are keyed by `skill_id`, not `name`

### 3.2 Prefilter with Embedding Similarity

When local skills exceed the threshold (`PREFILTER_THRESHOLD = 10`), a hybrid pre-filter narrows candidates before LLM selection:

```python
PREFILTER_THRESHOLD = 10
BM25_CANDIDATES_MULTIPLIER = 3  # top_k * 3

def _prefilter_skills(
    self,
    task: str,
    available: List[SkillMeta],
    max_skills: int,
) -> List[SkillMeta]:
    """Narrow the candidate set using BM25 + embedding hybrid ranking.

    Keeps at most ``max(15, max_skills * 5)`` candidates for the LLM
    selection prompt.
    """
    prefilter_top_k = max(15, max_skills * 5)

    # Build SkillCandidate list
    candidates: List[SkillCandidate] = []
    for s in available:
        body = ""
        raw = self._content_cache.get(s.skill_id, "")
        if raw:
            body = strip_frontmatter(raw)

        candidates.append(SkillCandidate(
            skill_id=s.skill_id,
            name=s.name,
            description=s.description,
            body=body,
        ))

    ranked = self.ranker.hybrid_rank(task, candidates, top_k=prefilter_top_k)

    # Map back to SkillMeta
    ranked_ids = {c.skill_id for c in ranked}
    result = [s for s in available if s.skill_id in ranked_ids]
```

### 3.3 BM25 Keyword Matching

```python
def _bm25_rank(
    self,
    query: str,
    candidates: List[SkillCandidate],
    top_k: int,
) -> List[SkillCandidate]:
    """Rank candidates using BM25."""
    if not candidates:
        return []

    try:
        from rank_bm25 import BM25Okapi  # type: ignore
    except ImportError:
        BM25Okapi = None

    # Build corpus: name + description + truncated body for richer matching
    corpus_tokens = []
    for c in candidates:
        text = f"{c.name} {c.description}"
        if c.body:
            text += f" {c.body[:2000]}"  # include body for BM25 but cap length
        corpus_tokens.append(self._tokenize(text))

    query_tokens = self._tokenize(query)

    if BM25Okapi and corpus_tokens:
        bm25 = BM25Okapi(corpus_tokens)
        scores = bm25.get_scores(query_tokens)
        for c, s in zip(candidates, scores):
            c.bm25_score = float(s)
    else:
        # Fallback: simple token overlap
        q_set = set(query_tokens)
        for c, toks in zip(candidates, corpus_tokens):
            if not toks or not q_set:
                c.bm25_score = 0.0
            else:
                overlap = q_set.intersection(toks)
                c.bm25_score = len(overlap) / len(q_set)

    # Sort and filter
    ranked = sorted(candidates, key=lambda c: c.bm25_score, reverse=True)

    # If all scores are 0 (no match), return all candidates (let embedding decide)
    if all(c.bm25_score == 0.0 for c in ranked):
        logger.debug("BM25 found no matches, passing all candidates to embedding stage")
        return candidates[:top_k]

    return ranked[:top_k]
```

**Tokenization:**

```python
@staticmethod
def _tokenize(text: str) -> List[str]:
    """Tokenize text for BM25."""
    tokens = re.split(r"[^\w]+", text.lower())
    return [t for t in tokens if t]
```

### 3.4 LLM-based Final Selection

After pre-filtering, an LLM performs final selection using a **plan-then-select** pattern:

```python
async def select_skills_with_llm(
    self,
    task_description: str,
    llm_client: "LLMClient",
    max_skills: int = 2,
    model: Optional[str] = None,
    skill_quality: Optional[Dict[str, Dict[str, Any]]] = None,
) -> tuple[List[SkillMeta], Optional[Dict[str, Any]]]:
    """Use an LLM to select the most relevant skills.

    When the local registry has more than ``PREFILTER_THRESHOLD`` skills,
    a **BM25 → embedding** pre-filter narrows the candidate set before
    sending to the LLM.  This avoids stuffing an overly long catalog
    into the prompt.

    Progressive disclosure: the LLM only sees skill *headers*
    (skill_id + description + quality stats), not the full SKILL.md
    content.  Full content is loaded only after selection.
    """
```

**Skill Selection Prompt:**

```python
@staticmethod
def _build_skill_selection_prompt(
    task: str,
    skills_catalog: str,
    max_skills: int,
) -> str:
    """Build the prompt for LLM skill selection.

    Uses a plan-then-select pattern: the LLM first writes a brief
    execution plan, then selects skills that match the plan.
    """
    return f"""You are a skill selector for an autonomous agent.

# Task

{{task}}

# Available Skills

{{skills_catalog}}

# Instructions

Follow these steps:

**Step 1 — Plan**: Think about how you would accomplish this task. What are the key deliverables? What file formats are needed (PDF, DOCX, XLSX, etc.)? What tools or libraries would you use?

**Step 2 — Match**: Check which skills directly teach workflows for the deliverables or file formats identified in your plan. A skill is relevant ONLY if it provides a tested procedure for a core part of your plan. Skills that only share vague topical overlap (e.g. a "PDF checklist" skill for a task that just happens to involve PDFs) add noise and should be excluded.

**Step 3 — Quality check**: Among matching skills, prefer ones with higher success rates. Avoid skills marked as "never succeeded" or with very low success rates — they waste iterations and actively hurt performance.

**Step 4 — Decide**: Select at most {{max_skills}} skill(s). If no skill closely matches your plan, you MUST return an empty list. Selecting an irrelevant or low-quality skill is **worse than selecting none** — it forces the agent down an unproductive path and wastes the entire iteration budget. When in doubt, leave it out.

Return a JSON object:
{{"brief_plan": "1-2 sentence plan for this task", "skills": ["skill_id_1", "skill_id_2"]}}

If no skill applies:
{{"brief_plan": "1-2 sentence plan", "skills": []}}

IMPORTANT: Use the **exact skill_id** from the list above."""
```

### 3.5 Hybrid Ranking Flow

```
┌─────────────────────────────────────────────────────────────────────────┐
│                        HYBRID RANKING PIPELINE                           │
├─────────────────────────────────────────────────────────────────────────┤
│                                                                         │
│  Input: Query + All Skills                                              │
│       │                                                                 │
│       ▼                                                                 │
│  ┌─────────────────────────────────────────────────────────────┐       │
│  │ STAGE 1: BM25 Rough-Rank                                     │       │
│  │ - Tokenize query + skill text (name + desc + body[:2000])   │       │
│  │ - Compute BM25 scores using rank_bm25                       │       │
│  │ - Fallback to token overlap if library unavailable          │       │
│  │ - Return top_k * 3 candidates                               │       │
│  └─────────────────────────────────────────────────────────────┘       │
│       │                                                                 │
│       ▼                                                                 │
│  ┌─────────────────────────────────────────────────────────────┐       │
│  │ STAGE 2: Embedding Re-Rank                                   │       │
│  │ - Generate query embedding (text-embedding-3-small)         │       │
│  │ - Generate/lookup candidate embeddings (cached)             │       │
│  │ - Compute cosine similarity scores                          │       │
│  │ - Return top_k candidates                                   │       │
│  └─────────────────────────────────────────────────────────────┘       │
│       │                                                                 │
│       ▼                                                                 │
│  ┌─────────────────────────────────────────────────────────────┐       │
│  │ STAGE 3: LLM Plan-then-Select                                │       │
│  │ - Build skills catalog (skill_id + description + quality)   │       │
│  │ - LLM writes brief execution plan                           │       │
│  │ - LLM selects skills matching the plan                      │       │
│  │ - Parse JSON response, validate skill_ids                   │       │
│  └─────────────────────────────────────────────────────────────┘       │
│       │                                                                 │
│       ▼                                                                 │
│  Output: Selected Skills (max_skills)                                   │
│                                                                         │
└─────────────────────────────────────────────────────────────────────────┘
```

---

## 4. Skill Ranking

### 4.1 SkillRanker Implementation

```python
@dataclass
class SkillCandidate:
    """Lightweight skill representation for ranking."""
    skill_id: str
    name: str
    description: str
    body: str = ""             # SKILL.md body (frontmatter stripped)
    source: str = "local"      # "local" | "cloud"
    # Internal ranking fields
    embedding: Optional[List[float]] = None
    embedding_text: str = ""   # text used to compute embedding
    score: float = 0.0
    bm25_score: float = 0.0
    vector_score: float = 0.0
    # Pass-through metadata (for MCP search results)
    metadata: Dict[str, Any] = field(default_factory=dict)


class SkillRanker:
    """Hybrid BM25 + embedding ranker for skills.

    Usage::

        ranker = SkillRanker()
        candidates = [SkillCandidate(skill_id=..., name=..., description=..., body=...)]
        ranked = ranker.hybrid_rank(query, candidates, top_k=10)
    """

    def __init__(
        self,
        *,
        cache_dir: Optional[Path] = None,
        enable_cache: bool = True,
    ) -> None:
        # Embedding cache: skill_id → List[float]
        self._embedding_cache: Dict[str, List[float]] = {}
        self._enable_cache = enable_cache

        if cache_dir is None:
            try:
                from openspace.config.constants import PROJECT_ROOT
                cache_dir = PROJECT_ROOT / ".openspace" / "skill_embedding_cache"
            except Exception:
                cache_dir = Path(".openspace") / "skill_embedding_cache"
        self._cache_dir = Path(cache_dir)

        if self._enable_cache:
            self._load_cache()
```

### 4.2 Score Calculation

The hybrid ranking uses a **two-stage approach**:

```python
def hybrid_rank(
    self,
    query: str,
    candidates: List[SkillCandidate],
    top_k: int = 10,
) -> List[SkillCandidate]:
    """BM25 rough-rank → embedding re-rank → return top_k.

    Falls back gracefully:
      - No BM25 lib → simple token overlap
      - No embedding API key → BM25-only
      - Both fail → return first top_k candidates
    """
    if not candidates or not query.strip():
        return candidates[:top_k]

    # Stage 1: BM25 rough-rank
    bm25_top = self._bm25_rank(query, candidates, top_k * BM25_CANDIDATES_MULTIPLIER)
    if not bm25_top:
        # BM25 found nothing — try embedding on all candidates
        emb_results = self._embedding_rank(query, candidates, top_k)
        return emb_results if emb_results else candidates[:top_k]

    # Stage 2: Embedding re-rank on BM25 candidates
    emb_results = self._embedding_rank(query, bm25_top, top_k)
    if emb_results:
        return emb_results

    # Embedding unavailable — return BM25 results
    logger.debug("Embedding unavailable, using BM25-only results")
    return bm25_top[:top_k]
```

**Embedding Text Construction:**

```python
SKILL_EMBEDDING_MAX_CHARS = 12_000

@staticmethod
def _build_embedding_text(candidate: SkillCandidate) -> str:
    """Build text for embedding, consistent with MCP search_skills."""
    if candidate.embedding_text:
        return candidate.embedding_text
    header = "\n".join(filter(None, [candidate.name, candidate.description]))
    raw = "\n\n".join(filter(None, [header, candidate.body]))
    if len(raw) > SKILL_EMBEDDING_MAX_CHARS:
        raw = raw[:SKILL_EMBEDDING_MAX_CHARS]
    candidate.embedding_text = raw
    return raw
```

**Cosine Similarity:**

```python
def _cosine_similarity(a: List[float], b: List[float]) -> float:
    """Compute cosine similarity between two vectors."""
    if len(a) != len(b) or not a:
        return 0.0
    dot = sum(x * y for x, y in zip(a, b))
    norm_a = math.sqrt(sum(x * x for x in a))
    norm_b = math.sqrt(sum(x * x for x in b))
    if norm_a == 0 or norm_b == 0:
        return 0.0
    return dot / (norm_a * norm_b)
```

### 4.3 Threshold Filtering

```python
# Pre-filter threshold: when local skills exceed this count, BM25 pre-filter
# is activated before LLM selection.  Below this, all skills go directly to LLM.
PREFILTER_THRESHOLD = 10

# How many candidates to keep after BM25 rough-rank (before embedding re-rank)
BM25_CANDIDATES_MULTIPLIER = 3  # top_k * 3
```

**Quality-Based Filtering:**

```python
# Quality-based filtering: remove skills that consistently fail
filtered_out: List[str] = []
if skill_quality:
    kept: List[SkillMeta] = []
    for s in available:
        q = skill_quality.get(s.skill_id)
        if q:
            selections = q.get("total_selections", 0)
            applied = q.get("total_applied", 0)
            completions = q.get("total_completions", 0)
            fallbacks = q.get("total_fallbacks", 0)
            # Filter 1: selected multiple times but never completed
            if selections >= 2 and completions == 0:
                filtered_out.append(s.skill_id)
                continue
            # Filter 2: high fallback rate when applied
            if applied >= 2 and fallbacks / applied > 0.5:
                filtered_out.append(s.skill_id)
                continue
        kept.append(s)
    available = kept
```

### 4.4 Multi-Skill Selection

The LLM selects multiple skills with a **strict relevance policy**:

```python
@staticmethod
def _parse_skill_selection_response(content: str) -> tuple[List[str], str]:
    """Parse the LLM response and extract selected skill IDs + plan.

    Returns:
        (skill_ids, brief_plan)
    """
    # Handle markdown code blocks
    code_block = re.search(r"```(?:json)?\s*\n?(.*?)\n?```", content, re.DOTALL)
    if code_block:
        content = code_block.group(1).strip()
    else:
        # Try to find a raw JSON object
        json_match = re.search(r"\{.*\}", content, re.DOTALL)
        if json_match:
            content = json_match.group()

    try:
        data = json.loads(content)
    except json.JSONDecodeError:
        logger.warning(f"Failed to parse LLM skill selection JSON: {content[:200]}")
        return [], ""

    brief_plan = data.get("brief_plan", "")
    if brief_plan:
        logger.info(f"Skill selection plan: {brief_plan}")

    ids = data.get("skills", [])
    if not isinstance(ids, list):
        return [], brief_plan
    return [str(n).strip() for n in ids if n], brief_plan
```

---

## 5. Skill Injection

### 5.1 Context Preparation

After selection, skills are injected into the agent context:

```python
def build_context_injection(
    self,
    skills: List[SkillMeta],
    backends: Optional[List[str]] = None,
) -> str:
    """Build a prompt fragment with the full content of *skills*.

    Injected as a system message into the agent's messages before the
    user instruction so the LLM reads skill guidance first.

    Args:
        skills: Skills to inject.
        backends: Active backend names (e.g. ``["shell", "mcp"]``).  Used to
            tailor the guidance so only actually available backends are
            mentioned.  ``None`` falls back to mentioning all backends.
    """
    parts: List[str] = []
    for skill in skills:
        content = self.load_skill_content(skill.skill_id)
        if content:
            # Resolve {baseDir} placeholder to the skill directory
            skill_dir = str(skill.path.parent)
            content = content.replace("{baseDir}", skill_dir)

            part = (
                f"### Skill: {skill.skill_id}\n"
                f"**Skill directory**: `{skill_dir}`\n\n"
                f"{content}"
            )
            parts.append(part)

    if not parts:
        return ""
```

### 5.2 Skill Instruction Loading

```python
def load_skill_content(self, skill_id: str) -> Optional[str]:
    """Return the SKILL.md content (with frontmatter stripped) for *skill_id*."""
    self._ensure_discovered()
    raw = self._content_cache.get(skill_id)
    if raw is None:
        return None
    return self._strip_frontmatter(raw)
```

### 5.3 Token Optimization

The system uses **progressive disclosure** to optimize token usage:

1. **Pre-filter stage**: Only skill_id + description visible
2. **LLM selection**: Catalog with descriptions + quality stats
3. **Final injection**: Full SKILL.md content loaded only for selected skills

**Header Format in Catalog:**

```python
catalog_lines: List[str] = []
for s in available:
    q = skill_quality.get(s.skill_id) if skill_quality else None
    if q:
        selections = q.get("total_selections", 0)
        applied = q.get("total_applied", 0)
        completions = q.get("total_completions", 0)
        if applied > 0:
            rate = completions / applied
            catalog_lines.append(
                f"- **{s.skill_id}**: {s.description}  "
                f"(success {completions}/{applied} = {rate:.0%})"
            )
        elif selections > 0:
            catalog_lines.append(
                f"- **{s.skill_id}**: {s.description}  "
                f"(selected {selections}x, never succeeded)"
            )
        else:
            catalog_lines.append(f"- **{s.skill_id}**: {s.description}  (new)")
    else:
        catalog_lines.append(f"- **{s.skill_id}**: {s.description}")
skills_catalog = "\n".join(catalog_lines)
```

### 5.4 Multi-Skill Composition

When multiple skills are selected, they're composed into a unified context:

```python
# Resource access tips — mention shell_agent only when shell is available
has_shell = "shell" in scope
resource_tip = (
    "Use `read_file` / `list_dir` / `write_file` for file operations"
    + (" and `shell_agent` for running scripts" if has_shell else "")
    + ". Paths in skill instructions are relative to the skill "
    "directory listed under each skill heading.\n\n"
)

header = (
    "# Active Skills\n\n"
    "The following skills provide **domain knowledge and tested procedures** "
    "relevant to this task.\n\n"
    "**How to use skills:**\n"
    "- If a skill contains **step-by-step procedures or commands**, follow them — "
    "they are verified workflows.\n"
    "- If a skill provides **reference information, best practices, or tool guides**, "
    "use it as context to inform your decisions.\n"
    f"- Skills supplement your available tools — you may use **any** tool "
    f"({tool_hint}) alongside skill guidance. "
    "Choose the best tool for each sub-step.\n\n"
    "**Resource access**: Each skill may include bundled resources "
    "(scripts, references, assets) in its skill directory. "
    + resource_tip
)
return header + "\n\n---\n\n".join(parts)
```

**Injected Context Format:**

```markdown
# Active Skills

The following skills provide **domain knowledge and tested procedures** 
relevant to this task.

**How to use skills:**
- If a skill contains **step-by-step procedures or commands**, follow them — 
  they are verified workflows.
- If a skill provides **reference information, best practices, or tool guides**, 
  use it as context to inform your decisions.
- Skills supplement your available tools — you may use **any** tool 
  (MCP, shell) alongside skill guidance. Choose the best tool for each sub-step.

**Resource access**: Each skill may include bundled resources 
(scripts, references, assets) in its skill directory. 
Use `read_file` / `list_dir` / `write_file` for file operations 
and `shell_agent` for running scripts. Paths in skill instructions 
are relative to the skill directory listed under each skill heading.

---

### Skill: weather__imp_a1b2c3d4
**Skill directory**: `/path/to/skills/weather`

# Weather Forecast Guide

## Overview
This skill provides...

---

### Skill: pdf__imp_b2c3d4e5
**Skill directory**: `/path/to/skills/pdf`

# PDF Generation Guide

## Overview
...
```

---

## 6. Cloud Integration

### 6.1 Local vs Cloud Skills

OpenSpace supports a **hybrid skill model** where local and cloud skills coexist:

| Aspect | Local Skills | Cloud Skills |
|--------|-------------|--------------|
| Storage | Filesystem directories | Remote platform (PostgreSQL + S3) |
| Discovery | Directory scanning | API fetch (`/records/metadata`) |
| Search | BM25 + embedding (local) | Server-side embedding search |
| Identity | `.skill_id` sidecar | `record_id` in database |
| Visibility | Always available | Controlled by access level |

### 6.2 Cloud Search Architecture (Hybrid: BM25 + Embedding)

The cloud uses a **server-side embedding search** with hybrid ranking:

```python
async def hybrid_search_skills(
    query: str,
    local_skills: list = None,
    store: Any = None,
    source: str = "all",
    limit: int = 20,
) -> List[Dict[str, Any]]:
    """Shared cloud+local skill search with graceful fallback.

    Builds candidates, generates embeddings, runs ``SkillSearchEngine``.
    Cloud is attempted when *source* includes it; failures are silently
    skipped so the caller always gets local results at minimum.
    """
    from openspace.cloud.embedding import generate_embedding

    normalized_query = query.strip()
    if not normalized_query:
        return []

    candidates: List[Dict[str, Any]] = []

    if source in ("all", "local") and local_skills:
        candidates.extend(build_local_candidates(local_skills, store))

    if source in ("all", "cloud"):
        try:
            from openspace.cloud.auth import get_openspace_auth
            from openspace.cloud.client import OpenSpaceClient

            auth_headers, api_base = get_openspace_auth()
            if auth_headers:
                cloud_client = OpenSpaceClient(auth_headers, api_base)
                cloud_result_limit = limit if source == "cloud" else CLOUD_EMBEDDING_SEARCH_MAX_LIMIT
                cloud_search_items = await asyncio.to_thread(
                    cloud_client.search_record_embeddings,
                    query=normalized_query,
                    limit=cloud_result_limit,
                )
                if source == "cloud":
                    return build_cloud_results(cloud_search_items, limit=limit)
                candidates.extend(build_cloud_candidates(cloud_search_items))
        except Exception as e:
            logger.warning(f"hybrid_search_skills: cloud unavailable: {e}")
```

**Cloud Search Flow:**

```
┌─────────────────────────────────────────────────────────────────────────┐
│                      CLOUD SEARCH ARCHITECTURE                           │
├─────────────────────────────────────────────────────────────────────────┤
│                                                                         │
│  Client Request (query + limit)                                         │
│       │                                                                 │
│       ▼                                                                 │
│  ┌─────────────────────────────────────────────────────────────┐       │
│  │ POST /records/embeddings/search                              │       │
│  │ - Server computes query embedding                            │       │
│  │ - PostgreSQL pgvector: cosine similarity search             │       │
│  │ - Returns ranked results with embeddings                    │       │
│  └─────────────────────────────────────────────────────────────┘       │
│       │                                                                 │
│       ▼                                                                 │
│  ┌─────────────────────────────────────────────────────────────┐       │
│  │ Client-Side Hybrid Fusion                                    │       │
│  │ - Merge cloud + local candidates                            │       │
│  │ - Re-rank with lexical boost                                │       │
│  │ - Deduplicate by name                                       │       │
│  └─────────────────────────────────────────────────────────────┘       │
│       │                                                                 │
│       ▼                                                                 │
│  Final Ranked Results                                                   │
│                                                                         │
└─────────────────────────────────────────────────────────────────────────┘
```

**Server-Side Search (Cloud Platform):**

```python
def search_record_embeddings(
    self,
    *,
    query: str,
    limit: int = RECORD_EMBEDDING_SEARCH_MAX_LIMIT,
    level: Optional[str] = None,
    tags: Optional[List[str]] = None,
) -> List[Dict[str, Any]]:
    """POST /records/embeddings/search — fetch server-ranked embedding rows."""
    search_request_payload: Dict[str, Any] = {
        "query": query,
        "limit": limit,
    }
    if level:
        search_request_payload["level"] = level
    if tags:
        search_request_payload["tags"] = tags

    _, response_body = self._request(
        "POST",
        "/records/embeddings/search",
        body=json.dumps(search_request_payload).encode("utf-8"),
        extra_headers={"Content-Type": "application/json"},
        timeout=30,
    )
    return json.loads(response_body.decode("utf-8"))
```

### 6.3 Download and Import

Cloud skills can be downloaded and imported locally:

```python
def import_skill(
    self,
    skill_id: str,
    target_dir: Path,
) -> Dict[str, Any]:
    """Download a cloud skill and extract to a local directory.

    Returns a result dict with status, local_path, files, etc.
    """
    # 1. Fetch metadata
    logger.info(f"import_skill: fetching metadata for {skill_id}")
    record_data = self.fetch_record(skill_id)
    skill_name = record_data.get("name", skill_id)

    if "/" in skill_name or "\\" in skill_name or skill_name.startswith("."):
        skill_name = skill_id
    skill_dir = (target_dir / skill_name).resolve()
    if not skill_dir.is_relative_to(target_dir.resolve()):
        raise CloudError(f"Skill name {skill_name!r} escapes target directory")

    # Check if already exists locally
    if skill_dir.exists() and (skill_dir / SKILL_FILENAME).exists():
        return {
            "status": "already_exists",
            "skill_id": skill_id,
            "name": skill_name,
            "local_path": str(skill_dir),
        }

    # 2. Download artifact
    logger.info(f"import_skill: downloading artifact for {skill_id}")
    zip_data = self.download_artifact(skill_id)

    # 3. Extract
    skill_dir.mkdir(parents=True, exist_ok=True)
    extracted = self._extract_zip(zip_data, skill_dir)

    # 4. Write .skill_id sidecar
    (skill_dir / SKILL_ID_FILENAME).write_text(skill_id + "\n", encoding="utf-8")

    return {
        "status": "success",
        "skill_id": skill_id,
        "name": skill_name,
        "description": record_data.get("description", ""),
        "local_path": str(skill_dir),
        "files": extracted,
    }
```

**Import Flow:**

```python
async def _do_import_cloud_skill(skill_id: str, target_dir: Optional[str] = None) -> Dict[str, Any]:
    """Download a cloud skill and register it locally."""
    client = _get_cloud_client()

    if target_dir:
        base_dir = Path(target_dir)
    else:
        # Determine base directory from environment or config
        host_ws = (
            os.environ.get("NANOBOT_WORKSPACE")
            or os.environ.get("OPENCLAW_STATE_DIR")
        )
        if host_ws:
            base_dir = Path(host_ws) / "skills"
            base_dir.mkdir(parents=True, exist_ok=True)
        else:
            # Fallback to package skills directory
            base_dir = Path(__file__).resolve().parent / "skills"

    result = await asyncio.to_thread(client.import_skill, skill_id, base_dir)

    skill_dir = Path(result.get("local_path", ""))
    if skill_dir.exists():
        openspace = await _get_openspace()
        registry = openspace._skill_registry
        if registry:
            meta = registry.register_skill_dir(skill_dir)
            if meta:
                store = _get_store()
                await store.sync_from_registry([meta])
                result["registered"] = True

    result.setdefault("registered", False)
    return result
```

### 6.4 Access Control (Public, Private, Group)

Cloud skills have visibility controls:

```python
class SkillVisibility(str, Enum):
    """Cloud visibility of a skill."""

    PRIVATE = "private"  # Only visible to the creator
    PUBLIC  = "public"   # Visible to all users on the cloud
```

**Upload with Visibility:**

```python
def upload_skill(
    self,
    skill_dir: Path,
    *,
    visibility: str = "public",
    origin: str = "imported",
    parent_skill_ids: Optional[List[str]] = None,
    tags: Optional[List[str]] = None,
    created_by: str = "",
    change_summary: str = "",
) -> Dict[str, Any]:
    """Upload a local skill to the cloud (stage → diff → create record).

    Returns a result dict with status, record_id, etc.
    """
    from openspace.skill_engine.skill_utils import parse_frontmatter

    skill_path = Path(skill_dir)
    skill_file = skill_path / SKILL_FILENAME
    if not skill_file.exists():
        raise CloudError(f"SKILL.md not found in {skill_dir}")

    content = skill_file.read_text(encoding="utf-8")
    fm = parse_frontmatter(content)
    name = fm.get("name", skill_path.name)
    description = fm.get("description", "")

    if not name:
        raise CloudError("SKILL.md frontmatter missing 'name' field")

    parents = parent_skill_ids or []
    self._validate_origin_parents(origin, parents)

    # Convert visibility: "private" → "group_only", "public" → "public"
    api_visibility = "group_only" if visibility == "private" else "public"

    # Step 1: Stage files
    artifact_id, file_count = self.stage_artifact(skill_path)

    # Step 2: Content diff
    content_diff = self._compute_content_diff(skill_path, api_visibility, parents)

    # Step 3: Create record
    record_id = f"{name}__clo_{uuid.uuid4().hex[:8]}"
    payload: Dict[str, Any] = {
        "record_id": record_id,
        "artifact_id": artifact_id,
        "origin": origin,
        "visibility": api_visibility,
        "parent_skill_ids": parents,
        "tags": tags or [],
        "level": "workflow",
    }
    if created_by:
        payload["created_by"] = created_by
    if change_summary:
        payload["change_summary"] = change_summary
    if content_diff is not None:
        payload["content_diff"] = content_diff

    record_data, status_code = self.create_record(payload)
```

**Visibility Filtering in Search:**

```python
public_cloud_hits = [
    cloud_result for cloud_result in cloud_search_results
    if cloud_result.get("visibility", "public") == "public"
    and cloud_result.get("record_id")
][:limit]
```

**Origin/Parent Validation:**

```python
@staticmethod
def _validate_origin_parents(origin: str, parents: List[str]) -> None:
    if origin in ("imported", "captured") and parents:
        raise CloudError(f"origin='{origin}' must not have parent_skill_ids")
    if origin == "derived" and not parents:
        raise CloudError("origin='derived' requires at least 1 parent_skill_id")
    if origin == "fixed" and len(parents) != 1:
        raise CloudError("origin='fixed' requires exactly 1 parent_skill_id")
```

---

## 7. Skill Utils

### 7.1 Frontmatter Parsing

```python
_YAML_NEEDS_QUOTE_RE = re.compile(r"[:\#\[\]{}&*!|>'\"%@`]")

def _yaml_quote(value: str) -> str:
    """Quote a YAML scalar value if it contains special characters."""
    if not value or not _YAML_NEEDS_QUOTE_RE.search(value):
        return value
    escaped = value.replace("\\", "\\\\").replace('"', '\\"')
    return f'"{escaped}"'

def _yaml_unquote(value: str) -> str:
    """Strip surrounding quotes and unescape a YAML scalar value."""
    if len(value) >= 2:
        if (value[0] == '"' and value[-1] == '"') or \
           (value[0] == "'" and value[-1] == "'"):
            inner = value[1:-1]
            if value[0] == '"':
                inner = inner.replace('\\"', '"').replace("\\\\", "\\")
            return inner
    return value
```

### 7.2 Strip Frontmatter

```python
def strip_frontmatter(content: str) -> str:
    """Remove YAML frontmatter from markdown content."""
    if content.startswith("---"):
        match = re.match(r"^---\n.*?\n---\n?", content, re.DOTALL)
        if match:
            return content[match.end():].strip()
    return content
```

### 7.3 Safety Checking

```python
_SAFETY_RULES = [
    ("blocked.malware",         re.compile(r"(ClawdAuthenticatorTool)", re.IGNORECASE)),
    ("suspicious.keyword",      re.compile(r"(malware|stealer|phish|phishing|keylogger)", re.IGNORECASE)),
    ("suspicious.secrets",      re.compile(r"(api[-_ ]?key|token|password|private key|secret)", re.IGNORECASE)),
    ("suspicious.crypto",       re.compile(r"(wallet|seed phrase|mnemonic|crypto)", re.IGNORECASE)),
    ("suspicious.webhook",      re.compile(r"(discord\.gg|webhook|hooks\.slack)", re.IGNORECASE)),
    ("suspicious.script",       re.compile(r"(curl[^\n]+\|\s*(sh|bash))", re.IGNORECASE)),
    ("suspicious.url_shortener", re.compile(r"(bit\.ly|tinyurl\.com|t\.co|goo\.gl|is\.gd)", re.IGNORECASE)),
]

_BLOCKING_FLAGS = frozenset({"blocked.malware"})

def check_skill_safety(text: str) -> List[str]:
    """Check *text* against safety rules, return list of triggered flag names."""
    return [flag for flag, pat in _SAFETY_RULES if pat.search(text)]

def is_skill_safe(flags: List[str]) -> bool:
    """Return True if *flags* contain no blocking flag."""
    return not any(f in _BLOCKING_FLAGS for f in flags)
```

### 7.4 Formatting

```python
def normalize_frontmatter(content: str) -> str:
    """Re-serialize frontmatter with proper YAML quoting.

    Parses the existing frontmatter, then re-writes each value through
    :func:`_yaml_quote` so that colons, hashes, and other special
    characters are safely double-quoted.  The body after ``---`` is
    preserved verbatim.

    Returns *content* unchanged if no frontmatter is found.
    """
    if not content.startswith("---"):
        return content
    match = _FRONTMATTER_RE.match(content)
    if not match:
        return content

    fm = parse_frontmatter(content)
    if not fm:
        return content

    safe_lines = [f"{k}: {_yaml_quote(v)}" for k, v in fm.items()]
    new_fm = "\n".join(safe_lines)
    return f"---\n{new_fm}\n---{content[match.end():]}"
```

### 7.5 Markdown Fence Stripping

```python
def strip_markdown_fences(text: str) -> str:
    """Remove surrounding markdown code fences if present.

    Handles common LLM wrapping patterns:
      - ````` ```markdown ```, ````` ```md ```, ````` ``` ```, ````` ```text `````
      - Nested triple-backtick pairs (outermost only)
      - Leading/trailing whitespace around fences
    """
    text = text.strip()

    # Pattern: opening ``` with optional language tag, content, closing ```
    m = re.match(
        r"^```(?:markdown|md|text|yaml|diff|patch)?\s*\n(.*?)\n```\s*$",
        text,
        re.DOTALL,
    )
    if m:
        return m.group(1).strip()

    # Some LLMs emit ``````` (4+ backticks) as outer fence
    m = re.match(
        r"^`{3,}(?:\w+)?\s*\n(.*?)\n`{3,}\s*$",
        text,
        re.DOTALL,
    )
    if m:
        return m.group(1).strip()

    return text
```

---

## 8. Fuzzy Matching

### 8.1 Skill Name Matching

The fuzzy matching system provides graceful degradation for skill name matching:

```python
from openspace.skill_engine.fuzzy_match import fuzzy_find_match

# Used when LLM selects a skill by approximate name
def get_skill_by_name(self, name: str) -> Optional[SkillMeta]:
    """Get a skill by ``name`` (first match).  Use ``get_skill`` when possible."""
    self._ensure_discovered()
    for meta in self._skills.values():
        if meta.name == name:
            return meta
    return None
```

### 8.2 Fallback Strategies

The fuzzy match chain degrades through multiple levels:

```python
REPLACER_CHAIN: list = [
    ("simple", simple_replacer),
    ("line_trimmed", line_trimmed_replacer),
    ("block_anchor", block_anchor_replacer),
    ("whitespace_normalized", whitespace_normalized_replacer),
    ("indentation_flexible", indentation_flexible_replacer),
    ("trimmed_boundary", trimmed_boundary_replacer),
]

def fuzzy_find_match(content: str, find: str) -> Tuple[str, int]:
    """Locate *find* in *content* using the replacer chain.

    Returns ``(matched_text, position)`` where *matched_text* is the
    actual substring of *content*, and *position* is its character offset.
    Returns ``("", -1)`` when no match is found.
    """
    for name, replacer in REPLACER_CHAIN:
        for candidate in replacer(content, find):
            pos = content.find(candidate)
            if pos == -1:
                continue
            if name != "simple":
                logger.debug(
                    "fuzzy_find_match: matched via '%s' at position %d",
                    name, pos,
                )
            return candidate, pos

    return "", -1
```

### 8.3 Levenshtein-Based Matching

```python
def levenshtein(a: str, b: str) -> int:
    """Compute the Levenshtein edit distance between two strings."""
    if not a or not b:
        return max(len(a), len(b))
    rows = len(a) + 1
    cols = len(b) + 1
    matrix = [[0] * cols for _ in range(rows)]
    for i in range(rows):
        matrix[i][0] = i
    for j in range(cols):
        matrix[0][j] = j
    for i in range(1, rows):
        for j in range(1, cols):
            cost = 0 if a[i - 1] == b[j - 1] else 1
            matrix[i][j] = min(
                matrix[i - 1][j] + 1,
                matrix[i][j - 1] + 1,
                matrix[i - 1][j - 1] + cost,
            )
    return matrix[len(a)][len(b)]
```

**Block Anchor Matching with Levenshtein:**

```python
def block_anchor_replacer(content: str, find: str) -> Replacer:
    """Anchor on first/last lines (trimmed) and use Levenshtein on middles."""
    original_lines = content.split("\n")
    search_lines = find.split("\n")

    if len(search_lines) < 3:
        return
    if search_lines and search_lines[-1] == "":
        search_lines.pop()
    if len(search_lines) < 3:
        return

    first_search = search_lines[0].strip()
    last_search = search_lines[-1].strip()
    search_block_size = len(search_lines)

    candidates: List[Tuple[int, int]] = []
    for i, line in enumerate(original_lines):
        if line.strip() != first_search:
            continue
        for j in range(i + 2, len(original_lines)):
            if original_lines[j].strip() == last_search:
                candidates.append((i, j))
                break

    if not candidates:
        return

    def _extract_block(start_line: int, end_line: int) -> str:
        s = sum(len(original_lines[k]) + 1 for k in range(start_line))
        e = s
        for k in range(start_line, end_line + 1):
            e += len(original_lines[k])
            if k < end_line:
                e += 1
        return content[s:e]

    if len(candidates) == 1:
        start_line, end_line = candidates[0]
        actual_size = end_line - start_line + 1
        lines_to_check = min(search_block_size - 2, actual_size - 2)

        if lines_to_check > 0:
            similarity = 0.0
            for j in range(1, min(search_block_size - 1, actual_size - 1)):
                orig_line = original_lines[start_line + j].strip()
                srch_line = search_lines[j].strip()
                max_len = max(len(orig_line), len(srch_line))
                if max_len == 0:
                    continue
                dist = levenshtein(orig_line, srch_line)
                similarity += (1 - dist / max_len) / lines_to_check
                if similarity >= SINGLE_CANDIDATE_SIMILARITY_THRESHOLD:
                    break
        else:
            similarity = 1.0

        if similarity >= SINGLE_CANDIDATE_SIMILARITY_THRESHOLD:
            yield _extract_block(start_line, end_line)
        return

    # Multiple candidates: pick the best
    best_match: Optional[Tuple[int, int]] = None
    max_similarity = -1.0

    for start_line, end_line in candidates:
        actual_size = end_line - start_line + 1
        lines_to_check = min(search_block_size - 2, actual_size - 2)

        if lines_to_check > 0:
            raw_sim = 0.0
            for j in range(1, min(search_block_size - 1, actual_size - 1)):
                orig_line = original_lines[start_line + j].strip()
                srch_line = search_lines[j].strip()
                max_len = max(len(orig_line), len(srch_line))
                if max_len == 0:
                    continue
                dist = levenshtein(orig_line, srch_line)
                raw_sim += 1 - dist / max_len
            similarity = raw_sim / lines_to_check
        else:
            similarity = 1.0

        if similarity > max_similarity:
            max_similarity = similarity
            best_match = (start_line, end_line)

    if max_similarity >= MULTIPLE_CANDIDATES_SIMILARITY_THRESHOLD and best_match:
        yield _extract_block(best_match[0], best_match[1])
```

### 8.4 Stem-Based Matching

```python
def _tokenize(value: str) -> list[str]:
    """Tokenize text for lexical matching."""
    return _WORD_RE.findall(value.lower()) if value else []

_WORD_RE = re.compile(r"[a-z0-9]+")

def _lexical_boost(query_tokens: list[str], name: str, slug: str) -> float:
    """Compute lexical boost score based on exact/prefix token matching."""
    slug_tokens = _tokenize(slug)
    name_tokens = _tokenize(name)
    boost = 0.0

    # Slug exact / prefix
    if slug_tokens and all(
        any(ct == qt for ct in slug_tokens) for qt in query_tokens
    ):
        boost += 1.4
    elif slug_tokens and all(
        any(ct.startswith(qt) for ct in slug_tokens) for qt in query_tokens
    ):
        boost += 0.8

    # Name exact / prefix
    if name_tokens and all(
        any(ct == qt for ct in name_tokens) for qt in query_tokens
    ):
        boost += 1.1
    elif name_tokens and all(
        any(ct.startswith(qt) for ct in name_tokens) for qt in query_tokens
    ):
        boost += 0.6

    return boost
```

---

## Appendix: Complete Source Code

### registry.py

```python
"""SkillRegistry — discover, load, match, and inject skills.

Skills follow the official SKILL.md format:
  - YAML frontmatter with only ``name`` and ``description``
  - Markdown body with instructions (loaded only after selection)

Skills are discovered from user-configured directories and matched to
tasks via LLM-based selection (with keyword fallback).

Skill identity:
  Every skill directory may contain a ``.skill_id`` sidecar file that
  stores the persistent unique identifier.  On **first discovery**
  (no ``.skill_id`` file present), an ID is generated and written to
  the file.  On subsequent runs the ID is **read** from the file —
  this makes the ID portable (survives directory moves, machine changes)
  and deterministic (never regenerated).

  Imported skills: ``{name}__imp_{uuid_hex[:8]}``
  Evolved skills:  ``{name}__v{gen}_{uuid_hex[:8]}``  (written by evolver)
"""

from __future__ import annotations

import json
import re
import uuid
from dataclasses import dataclass
from pathlib import Path
from typing import Any, Dict, List, Optional, TYPE_CHECKING

from openspace.utils.logging import Logger
from .skill_utils import parse_frontmatter, strip_frontmatter, check_skill_safety, is_skill_safe
from .skill_ranker import SkillRanker, SkillCandidate, PREFILTER_THRESHOLD

if TYPE_CHECKING:
    from openspace.llm import LLMClient

logger = Logger.get_logger(__name__)

# Sidecar filename that stores the persistent skill_id
SKILL_ID_FILENAME = ".skill_id"


def _read_or_create_skill_id(name: str, skill_dir: Path) -> str:
    """Read ``skill_id`` from ``.skill_id`` sidecar, or create one.

    The sidecar file is a single-line plain-text file containing only
    the ``skill_id`` string.  It lives alongside ``SKILL.md`` inside
    the skill directory.

    First call (no file): generates ``{name}__imp_{uuid8}`` and writes it.
    Subsequent calls: reads and returns the existing ID.
    """
    id_file = skill_dir / SKILL_ID_FILENAME
    if id_file.exists():
        try:
            existing = id_file.read_text(encoding="utf-8").strip()
            if existing:
                return existing
        except OSError:
            pass  # fall through to generate

    # Generate a new ID and persist
    new_id = f"{name}__imp_{uuid.uuid4().hex[:8]}"
    try:
        id_file.write_text(new_id + "\n", encoding="utf-8")
        logger.debug(f"Created .skill_id for '{name}': {new_id}")
    except OSError as e:
        logger.warning(f"Cannot write {id_file}: {e} — ID will not persist across restarts")
    return new_id


def write_skill_id(skill_dir: Path, skill_id: str) -> None:
    """Write (or overwrite) the ``.skill_id`` sidecar in *skill_dir*.

    Called by ``SkillEvolver`` after FIX / DERIVED / CAPTURED to stamp
    the new ``skill_id`` into the skill directory so that the next
    ``discover()`` picks it up correctly.
    """
    id_file = skill_dir / SKILL_ID_FILENAME
    try:
        id_file.write_text(skill_id + "\n", encoding="utf-8")
    except OSError as e:
        logger.warning(f"Cannot write {id_file}: {e}")


@dataclass
class SkillMeta:
    """Metadata for a discovered skill.

    ``skill_id`` is the globally unique identifier used throughout the
    system — LLM prompts, database, evolution, and selection all
    reference this field.
    """

    skill_id: str          # Unique — persisted in .skill_id sidecar
    name: str              # Human-readable name (from frontmatter or dirname)
    description: str
    path: Path             # Absolute path to SKILL.md


class SkillRegistry:
    """Discover, load, select, and inject skills into agent context.

    Args:
        skill_dirs: Ordered list of directories to scan.  Earlier entries have higher
            priority — a skill in the first dir shadows one with the same name
            in later dirs.

    All internal maps are keyed by ``skill_id``, not ``name``.
    """

    def __init__(self, skill_dirs: Optional[List[Path]] = None) -> None:
        self._skill_dirs: List[Path] = skill_dirs or []
        self._skills: Dict[str, SkillMeta] = {}     # skill_id -> SkillMeta
        self._content_cache: Dict[str, str] = {}     # skill_id -> raw SKILL.md content
        self._discovered = False
        self._ranker: Optional[SkillRanker] = None   # lazy-init on first use
```

### skill_ranker.py

```python
"""SkillRanker — BM25 + embedding hybrid ranking for skills.

Provides a two-stage retrieval pipeline for skill selection:
  Stage 1 (BM25): Fast lexical rough-rank over all skills
  Stage 2 (Embedding): Semantic re-rank on BM25 candidates

Embedding strategy:
  - Text = ``name + description + SKILL.md body`` (consistent with MCP
    ``search_skills`` and the clawhub cloud platform)
  - Model: ``text-embedding-3-small`` via OpenAI API
  - Embeddings are cached in-memory keyed by ``skill_id`` and optionally
    persisted to a pickle file for cross-session reuse

Reused by:
  - ``SkillRegistry.select_skills_with_llm`` — pre-filter before LLM selection
  - ``mcp_server.search_skills`` — BM25 stage of the MCP search tool
"""

from __future__ import annotations

import json
import math
import os
import pickle
import re
from dataclasses import dataclass, field
from datetime import datetime
from pathlib import Path
from typing import Any, Dict, List, Optional, Tuple

from openspace.utils.logging import Logger

logger = Logger.get_logger(__name__)

# Embedding model — must match clawhub platform for vector-space compatibility
SKILL_EMBEDDING_MODEL = "openai/text-embedding-3-small"
SKILL_EMBEDDING_MAX_CHARS = 12_000

# Pre-filter threshold: when local skills exceed this count, BM25 pre-filter
# is activated before LLM selection.  Below this, all skills go directly to LLM.
PREFILTER_THRESHOLD = 10

# How many candidates to keep after BM25 rough-rank (before embedding re-rank)
BM25_CANDIDATES_MULTIPLIER = 3  # top_k * 3

# Cache version — increment when format changes
_CACHE_VERSION = 1


@dataclass
class SkillCandidate:
    """Lightweight skill representation for ranking."""
    skill_id: str
    name: str
    description: str
    body: str = ""             # SKILL.md body (frontmatter stripped)
    source: str = "local"      # "local" | "cloud"
    # Internal ranking fields
    embedding: Optional[List[float]] = None
    embedding_text: str = ""   # text used to compute embedding
    score: float = 0.0
    bm25_score: float = 0.0
    vector_score: float = 0.0
    # Pass-through metadata (for MCP search results)
    metadata: Dict[str, Any] = field(default_factory=dict)


class SkillRanker:
    """Hybrid BM25 + embedding ranker for skills.

    Usage::

        ranker = SkillRanker()
        candidates = [SkillCandidate(skill_id=..., name=..., description=..., body=...)]
        ranked = ranker.hybrid_rank(query, candidates, top_k=10)
    """

    def __init__(
        self,
        *,
        cache_dir: Optional[Path] = None,
        enable_cache: bool = True,
    ) -> None:
        # Embedding cache: skill_id → List[float]
        self._embedding_cache: Dict[str, List[float]] = {}
        self._enable_cache = enable_cache

        if cache_dir is None:
            try:
                from openspace.config.constants import PROJECT_ROOT
                cache_dir = PROJECT_ROOT / ".openspace" / "skill_embedding_cache"
            except Exception:
                cache_dir = Path(".openspace") / "skill_embedding_cache"
        self._cache_dir = Path(cache_dir)

        if self._enable_cache:
            self._load_cache()
```

### skill_utils.py

```python
"""Shared utility functions for the skill engine.

Provides:
  - YAML frontmatter parsing/manipulation (unified across registry, evolver, etc.)
  - LLM output cleaning (markdown fence stripping, change summary extraction)
  - Skill content safety checking (regex-based moderation)
  - Skill directory validation
  - Text truncation
"""

from __future__ import annotations

import re
from pathlib import Path
from typing import Any, Dict, List, Optional

from openspace.utils.logging import Logger

logger = Logger.get_logger(__name__)

SKILL_FILENAME = "SKILL.md"

_SAFETY_RULES = [
    ("blocked.malware",         re.compile(r"(ClawdAuthenticatorTool)", re.IGNORECASE)),
    ("suspicious.keyword",      re.compile(r"(malware|stealer|phish|phishing|keylogger)", re.IGNORECASE)),
    ("suspicious.secrets",      re.compile(r"(api[-_ ]?key|token|password|private key|secret)", re.IGNORECASE)),
    ("suspicious.crypto",       re.compile(r"(wallet|seed phrase|mnemonic|crypto)", re.IGNORECASE)),
    ("suspicious.webhook",      re.compile(r"(discord\.gg|webhook|hooks\.slack)", re.IGNORECASE)),
    ("suspicious.script",       re.compile(r"(curl[^\n]+\|\s*(sh|bash))", re.IGNORECASE)),
    ("suspicious.url_shortener", re.compile(r"(bit\.ly|tinyurl\.com|t\.co|goo\.gl|is\.gd)", re.IGNORECASE)),
]

_BLOCKING_FLAGS = frozenset({"blocked.malware"})


def check_skill_safety(text: str) -> List[str]:
    """Check *text* against safety rules, return list of triggered flag names.

    Returns an empty list if no rules match (= safe).
    """
    return [flag for flag, pat in _SAFETY_RULES if pat.search(text)]


def is_skill_safe(flags: List[str]) -> bool:
    """Return True if *flags* contain no blocking flag.

    ``suspicious.*`` flags are informational (logged / attached to search
    results) but do NOT block.  Only ``blocked.*`` flags cause rejection.
    """
    return not any(f in _BLOCKING_FLAGS for f in flags)
```

---

## Summary

The OpenSpace Skill Registry and Discovery System is a production-grade skill management platform featuring:

1. **Robust Identity Management**: `.skill_id` sidecar files enable portable, deterministic skill identity
2. **Hybrid Ranking**: BM25 + embedding + LLM selection provides accurate, context-aware skill discovery
3. **Quality-Aware Filtering**: Skills with high fallback rates are automatically filtered out
4. **Cloud Integration**: Seamless hybrid search across local and cloud skills with access control
5. **Token Optimization**: Progressive disclosure minimizes context window usage
6. **Safety First**: Regex-based moderation blocks malicious skill content
7. **Graceful Degradation**: Multiple fallback strategies ensure the system works even without embeddings or LLM access

The architecture follows production-ready patterns including caching, retry logic, lazy initialization, and comprehensive error handling.
