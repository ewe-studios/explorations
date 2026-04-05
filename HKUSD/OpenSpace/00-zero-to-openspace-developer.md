# Zero to OpenSpace Developer

A comprehensive guide to understanding, installing, and developing with OpenSpace — the self-evolving agent skill engine.

---

## Table of Contents

1. [Project Overview](#1-project-overview)
2. [Core Concepts](#2-core-concepts)
3. [Architecture](#3-architecture)
4. [Getting Started](#4-getting-started)
5. [Skill System](#5-skill-system)
6. [Evolution Modes](#6-evolution-modes)
7. [Quality Monitoring](#7-quality-monitoring)
8. [Cloud Community](#8-cloud-community)
9. [Development Guide](#9-development-guide)

---

## 1. Project Overview

### What is OpenSpace?

**OpenSpace** is a self-evolving agent skill engine that transforms AI agents from static tools into adaptive, learning systems. It plugs into any agent (Claude Code, OpenClaw, nanobot, Codex, Cursor) and provides:

- **Skills**: Reusable execution patterns that agents apply to tasks
- **Self-Evolution**: Skills that automatically fix, improve, and adapt
- **Cloud Community**: A shared registry where agents contribute and benefit from collective intelligence

### The Problem: Agents Don't Learn, Adapt, Evolve, Share

Current AI agents have critical weaknesses:

| Problem | Consequence |
|---------|-------------|
| **No Learning** | Agents repeat the same mistakes across sessions |
| **No Adaptation** | Skills degrade silently as tools and APIs evolve |
| **No Evolution** | Failed patterns are never converted into improvements |
| **No Sharing** | Knowledge remains trapped in individual agents |

This leads to:
- **Massive Token Waste**: Reasoning from scratch every time instead of reusing patterns
- **Repeated Costly Failures**: Same errors across agents, no collective learning
- **Poor Reliability**: Skills break as external tools change, no automatic repair

### Three Superpowers

#### 1. Self-Evolution (Growth Mindset)

Skills automatically improve through three mechanisms:

- **AUTO-FIX**: When a skill breaks (API changes, tool error patterns), it repairs itself
- **AUTO-IMPROVE**: Successful patterns spawn enhanced versions (DERIVED skills)
- **AUTO-LEARN**: Novel successful workflows are extracted as new skills (CAPTURED)
- **Quality Monitoring**: Tracks skill metrics (applied rate, completion rate, fallback rate) and triggers evolution for underperformers

**Result**: Skills that continuously improve — turning every failure into a fix, every success into optimization.

#### 2. Collective Intelligence (Network Effects)

The cloud community transforms individual progress into collective capability:

- **Shared Evolution**: One agent's improvement becomes available to all
- **Network Effects**: More agents → richer data → faster evolution for everyone
- **Easy Sharing**: Upload/download evolved skills with one command
- **Access Control**: Choose public, private, or group-only visibility

**Result**: One agent learns, all agents benefit — collective intelligence at scale.

#### 3. Token Efficiency (Economic Value)

Smarter agents dramatically reduce costs:

- **Stop Repeating Work**: Reuse successful solutions instead of starting from zero
- **Tasks Get Cheaper**: As skills improve, similar work costs less
- **Small Updates Only**: Fix what's broken, don't rebuild everything
- **Proven Results**: 46% fewer tokens on GDPVal benchmark tasks

### GDPVal Benchmark Results

OpenSpace was evaluated on [GDPVal](https://huggingface.co/datasets/openai/gdpval) — 50 professional tasks across 6 industries:

| Metric | OpenSpace | Baseline (ClawWork) | Improvement |
|--------|-----------|---------------------|-------------|
| **Income Captured** | $11,484 | $2,736 | **4.2x higher** |
| **Value Capture Rate** | 72.8% | ~20% | **+52pp** |
| **Average Quality** | 70.8% | 40.8% | **+30pp** |
| **Token Usage (Phase 2)** | 45.9% of Phase 1 | N/A | **54% reduction** |

**Real-World Tasks**:
- Building payroll calculators from complex union contracts
- Preparing tax returns from 15 scattered PDF documents
- Drafting legal memoranda on California privacy regulations
- Creating compliance forms and engineering specifications

**By Category**:

| Category | Income Δ | Token Δ | Why |
|----------|----------|---------|-----|
| Documents & Correspondence | +3.3pp | -56% | `document-gen-fallback` evolved 13 versions |
| Compliance & Forms | +18.5pp | -51% | PDF skill chain evolves once, reused everywhere |
| Media Production | +5.8pp | -46% | ffmpeg flags and codec fallbacks encoded |
| Engineering | +8.7pp | -43% | Multi-file orchestration transfers universally |
| Spreadsheets | +7.3pp | -37% | Patterns (formulas, merged cells) identical across domains |
| Strategy & Analysis | +1.0pp | -32% | Already high quality; savings from structure reuse |

---

## 2. Core Concepts

### Skills (SKILL.md Format)

A **skill** is a reusable execution pattern stored as a directory with a `SKILL.md` file:

```
my-skill/
├── SKILL.md           # Skill definition (YAML frontmatter + instructions)
└── .skill_id          # Persistent unique identifier (auto-generated)
```

**SKILL.md Structure**:

```markdown
---
name: skill-name
description: One-line description of what the skill does
---

# Skill Name

## When to Use

Circumstances that trigger this skill.

## Core Technique

The main approach or workflow.

## Step-by-Step Workflow

1. Step one with code examples
2. Step two with code examples
3. Step three with verification

## Complete Example

Full working example showing the skill in action.

## Troubleshooting

Common issues and how to resolve them.
```

**Example** (`document-gen-fallback/SKILL.md`):

```markdown
---
name: document-gen-fallback
description: Fallback workflow for multi-format document generation when shell_agent encounters errors
---

# Document Generation Fallback Workflow

## When to Use

Use this skill when `shell_agent` returns unknown or unclear errors on complex document generation tasks.

## Core Technique

Instead of delegating the entire document generation to `shell_agent`, manually split the workflow into discrete, observable steps:
1. **Content creation** → Use `write_file` to create source document (Markdown)
2. **Format conversion** → Use `run_shell` with `pandoc` for each target format
3. **Verification** → Check output files exist and are valid

## Step-by-Step Workflow

### Step 1: Create Source Content with write_file

```
write_file
path: /tmp/document_source.md
content: |
  # Document Title
  
  ## Section 1
  Content here...
```

### Step 2: Convert to Target Formats with run_shell

```
run_shell
command: pandoc /tmp/document_source.md -o output.docx
```
```

### Skill Evolution Types

Skills evolve through three modes:

| Type | Purpose | Result |
|------|---------|--------|
| **FIX** | Repair broken/outdated instructions | Same skill, new version (in-place update) |
| **DERIVED** | Create enhanced version from parent | New skill directory, coexists with parent |
| **CAPTURED** | Extract novel pattern from execution | Brand new skill, no parent |

**Example Evolution Chain**:

```
document-gen-fallback (imported)
├── document-gen-fallback-enhanced (DERIVED, gen 1)
│   ├── document-gen-fallback-enhanced-enhanced (DERIVED, gen 2)
│   │   └── document-gen-fallback-enhanced-enhanced-2794b4 (DERIVED, gen 3)
│   └── document-gen-fallback-merged (DERIVED, gen 2)
└── document-gen-fallback-enhanced-9f3b1f (DERIVED, gen 1)
```

### Evolution Triggers

Three independent triggers ensure skills stay healthy:

#### 1. Post-Execution Analysis

After every task, the `ExecutionAnalyzer` reviews the full recording:

- Which skills were applied?
- Did they succeed or fail?
- What patterns could be extracted?
- What needs fixing?

**Suggestion Format**:

```python
EvolutionSuggestion(
    evolution_type=EvolutionType.DERIVED,
    target_skill_ids=["skill-abc123"],
    reason="Successful pattern worth generalizing",
    priority=0.85,
)
```

#### 2. Tool Degradation Monitor

When tool success rates drop:

1. `ToolQualityManager` flags the problematic tool
2. All skills depending on that tool are identified
3. Batch evolution is triggered to add fallbacks

**Anti-Loop**: Skills evolved for a specific tool degradation are tracked. If the tool recovers and degrades again, re-evaluation is allowed.

#### 3. Metric Monitor

Periodic scan of skill health metrics:

| Metric | Threshold | Action |
|--------|-----------|--------|
| Fallback Rate | > 0.4 | FIX evolution |
| Completion Rate | < 0.35 | FIX evolution |
| Applied Rate | > 0.4 + Low Effective | DERIVED evolution |
| Total Selections | < min_selections | Skip (not enough data) |

### Cloud Skill Community

**open-space.cloud** is the central registry:

- **Upload**: Share evolved skills (public, private, group-only)
- **Download**: Discover and auto-import skills from other agents
- **Search**: Hybrid search (BM25 + embeddings + LLM ranking)
- **Lineage Tracking**: Full version history and diff visualization

**CLI Tools**:

```bash
# Download a skill from the cloud
openspace-download-skill <skill_id>

# Upload a skill to the cloud
openspace-upload-skill /path/to/skill/dir --visibility public
```

---

## 3. Architecture

### System Overview

```
┌─────────────────────────────────────────────────────────────────┐
│                        Agent (Host)                              │
│  (Claude Code / OpenClaw / nanobot / Cursor / Codex)            │
│                          │                                        │
│                          │ MCP Protocol                          │
└──────────────────────────┼────────────────────────────────────────┘
                           │
                           ▼
┌─────────────────────────────────────────────────────────────────┐
│                    OpenSpace MCP Server                          │
│  ┌──────────────────────────────────────────────────────────┐   │
│  │  4 Tools: execute_task, search_skills, fix_skill, upload  │   │
│  └──────────────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────────────┘
                           │
                           ▼
┌─────────────────────────────────────────────────────────────────┐
│                      OpenSpace Engine                            │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐          │
│  │ SkillRegistry│  │GroundingAgent│  │SkillEvolver  │          │
│  │ (discovery)  │  │ (execution)  │  │ (evolution)  │          │
│  └──────────────┘  └──────────────┘  └──────────────┘          │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐          │
│  │ SkillStore   │  │ Execution    │  │ ToolQuality  │          │
│  │ (SQLite DB)  │  │ Analyzer     │  │ Manager      │          │
│  └──────────────┘  └──────────────┘  └──────────────┘          │
└─────────────────────────────────────────────────────────────────┘
                           │
          ┌────────────────┼────────────────┐
          ▼                ▼                ▼
┌─────────────────┐ ┌──────────────┐ ┌──────────────┐
│  Cloud Client   │ │  Backends    │ │  Recording   │
│  (HTTP API)     │ │ (shell/gui/  │ │  Manager     │
│                 │ │  mcp/web)    │ │              │
└─────────────────┘ └──────────────┘ └──────────────┘
```

### OpenSpace Main Class (`tool_layer.py`)

The `OpenSpace` class is the main entry point:

```python
from openspace import OpenSpace, OpenSpaceConfig

config = OpenSpaceConfig(
    llm_model="openrouter/anthropic/claude-sonnet-4.5",
    llm_kwargs={"api_key": "sk-xxx"},
    workspace_dir="/path/to/workspace",
    grounding_max_iterations=20,
    enable_recording=True,
    recording_log_dir="./logs/recordings",
)

async with OpenSpace(config=config) as cs:
    result = await cs.execute("Build a monitoring dashboard")
    print(result["response"])
    
    for skill in result.get("evolved_skills", []):
        print(f"Evolved: {skill['name']} ({skill['origin']})")
```

**Key Components Initialized**:

```python
class OpenSpace:
    def __init__(self, config: OpenSpaceConfig):
        self._llm_client: LLMClient           # LiteLLM wrapper
        self._grounding_client: GroundingClient  # Backend orchestration
        self._grounding_agent: GroundingAgent    # Task execution
        self._skill_registry: SkillRegistry      # Skill discovery/ranking
        self._skill_store: SkillStore            # SQLite persistence
        self._execution_analyzer: ExecutionAnalyzer  # Post-task analysis
        self._skill_evolver: SkillEvolver        # Evolution execution
        self._recording_manager: RecordingManager    # Screenshots/video
```

### MCP Server (`mcp_server.py`)

Exposes 4 tools to agent clients:

#### 1. `execute_task`

Delegate a task to OpenSpace:

```python
@mcp.tool()
async def execute_task(
    task: str,
    search_scope: str = "all",
    max_iterations: int = 20,
) -> dict:
    """Execute a task with skill search, execution, and evolution."""
    
    openspace = await _get_openspace()
    result = await openspace.execute(
        instruction=task,
        search_scope=search_scope,
        max_iterations=max_iterations,
    )
    
    return {
        "status": result["status"],
        "response": result["response"],
        "evolved_skills": result.get("evolved_skills", []),
    }
```

#### 2. `search_skills`

Search local and cloud skills:

```python
@mcp.tool()
async def search_skills(
    query: str,
    source: str = "all",
    limit: int = 20,
    auto_import: bool = True,
) -> list:
    """Search for skills matching the query."""
    
    # Local search (BM25 + embedding + LLM)
    local_results = await search_local_skills(query, limit)
    
    # Cloud search (if API key configured)
    cloud_results = []
    if source == "all" and has_api_key():
        cloud_results = await search_cloud_skills(query, limit)
        if auto_import and cloud_results:
            await auto_import_top_skill(cloud_results[0])
    
    return local_results + cloud_results
```

#### 3. `fix_skill`

Manually fix a broken skill:

```python
@mcp.tool()
async def fix_skill(
    skill_dir: str,
    direction: str,
) -> dict:
    """Fix a skill with explicit instructions."""
    
    openspace = await _get_openspace()
    result = await openspace.evolver.fix_skill_sync(
        skill_dir=Path(skill_dir),
        fix_direction=direction,
    )
    
    _write_upload_meta(result.skill_dir, {
        "origin": "fix",
        "change_summary": direction,
    })
    
    return {
        "status": "success",
        "skill_dir": str(result.skill_dir),
        "upload_ready": True,
    }
```

#### 4. `upload_skill`

Upload a skill to the cloud:

```python
@mcp.tool()
async def upload_skill(
    skill_dir: str,
    visibility: str = "public",
    origin: Optional[str] = None,
    tags: Optional[List[str]] = None,
    change_summary: Optional[str] = None,
) -> dict:
    """Upload a skill to the cloud community."""
    
    client = _get_cloud_client()
    skill_id = await client.upload_skill(
        skill_dir=Path(skill_dir),
        visibility=visibility,
        metadata={...},  # Read from .upload_meta.json or defaults
    )
    
    return {"status": "success", "skill_id": skill_id}
```

### GroundingAgent (`grounding_agent.py`)

The execution agent that runs tasks with tool calling:

```python
class GroundingAgent(BaseAgent):
    async def process(self, context: dict) -> dict:
        """Execute a task with iterative tool calling."""
        
        messages = [
            {"role": "system", "content": self._system_prompt},
            {"role": "user", "content": context["instruction"]},
        ]
        
        # Inject skill context if available
        if self._skill_context:
            messages[0]["content"] += "\n\n" + self._skill_context
        
        for iteration in range(self._max_iterations):
            # LLM decides next action
            response = await self._llm_client.chat(messages)
            
            # Parse tool calls
            if response.tool_calls:
                for tool_call in response.tool_calls:
                    result = await self._execute_tool(tool_call)
                    messages.append({"role": "tool", "content": result})
            else:
                # Final response
                return {"status": "success", "response": response.content}
        
        return {"status": "max_iterations_reached", "response": messages[-1]}
```

**Key Features**:
- **Skill Context Injection**: Relevant skills are prepended to the system prompt
- **Message Truncation**: Caps oversized tool results to prevent context overflow
- **Backend Scope**: Controls which backends (shell, gui, mcp, web) are available
- **Visual Analysis**: Optional screenshot analysis for GUI tasks

### SkillEngine Components

#### SkillRegistry (`registry.py`)

Discovers and ranks skills:

```python
class SkillRegistry:
    def discover(self) -> List[SkillMeta]:
        """Scan skill directories and populate registry."""
        for skill_dir in self._skill_dirs:
            for entry in skill_dir.iterdir():
                skill_file = entry / "SKILL.md"
                if skill_file.exists():
                    content = skill_file.read_text()
                    meta = self._parse_skill(entry.name, entry, content)
                    self._skills[meta.skill_id] = meta
    
    async def select_relevant_skills(self, task: str) -> List[str]:
        """Hybrid search: BM25 prefilter → embedding → LLM selection."""
        
        # Step 1: BM25 keyword prefilter
        candidates = self._ranker.prefilter(task, threshold=0.3)
        
        # Step 2: Embedding similarity ranking
        ranked = await self._ranker.rank_with_embeddings(task, candidates)
        
        # Step 3: LLM final selection
        selected = await self._llm_select(task, ranked[:20])
        
        return selected
```

#### SkillStore (`store.py`)

SQLite persistence for skill quality tracking:

```python
class SkillStore:
    """Tables:
    - skill_records: SkillRecord main table
    - skill_lineage_parents: Parent-child relationships
    - execution_analyses: Per-task analysis records
    - skill_judgments: Per-skill judgments within analysis
    - skill_tool_deps: Tool dependencies
    - skill_tags: Auxiliary tags
    """
    
    def save_record(self, record: SkillRecord) -> None:
        """Upsert a skill record with metrics."""
        
        self._conn.execute("""
            INSERT INTO skill_records (...)
            VALUES (?, ?, ?, ...)
            ON CONFLICT(skill_id) DO UPDATE SET
                total_selections = total_selections + ?,
                total_applied = total_applied + ?,
                last_updated = ?
        """, [...])
```

#### SkillEvolver (`evolver.py`)

Executes evolution with an agent loop:

```python
class SkillEvolver:
    async def evolve(self, ctx: EvolutionContext) -> Optional[SkillRecord]:
        """Execute one evolution action."""
        
        evo_type = ctx.suggestion.evolution_type
        
        if evo_type == EvolutionType.FIX:
            return await self._evolve_fix(ctx)
        elif evo_type == EvolutionType.DERIVED:
            return await self._evolve_derived(ctx)
        elif evo_type == EvolutionType.CAPTURED:
            return await self._evolve_captured(ctx)
    
    async def _evolve_fix(self, ctx: EvolutionContext) -> SkillRecord:
        """Fix a skill in-place."""
        
        # Build agent prompt with skill content and fix direction
        prompt = self._build_fix_prompt(ctx)
        
        # Agent loop: read, analyze, edit, validate
        async with self._semaphore:  # Concurrency limit
            for attempt in range(_MAX_EVOLUTION_ATTEMPTS):
                result = await self._agent_loop(
                    prompt=prompt,
                    tools=ctx.available_tools,
                    max_iterations=_MAX_EVOLUTION_ITERATIONS,
                )
                
                if result.success:
                    # Validate new skill
                    if await self._validate_skill(result.skill_dir):
                        # Persist to store
                        return await self._store.save_fix(result)
```

### Cloud Client (`cloud/client.py`)

HTTP client for cloud API:

```python
class OpenSpaceClient:
    def __init__(self, auth_headers: dict, api_base: str):
        self._headers = auth_headers
        self._base = api_base
    
    def upload_skill(self, skill_dir: Path) -> str:
        """Full workflow: stage → diff → create record."""
        
        # Step 1: Upload files (get artifact_id)
        artifact_id, file_count = self.stage_artifact(skill_dir)
        
        # Step 2: Compute diff vs parent (if any)
        diff = self._compute_diff(skill_dir)
        
        # Step 3: Create record
        record = self.create_record(
            artifact_id=artifact_id,
            skill_id=skill_id,
            metadata={...},
            diff=diff,
        )
        
        return record["record_id"]
    
    def search_record_embeddings(
        self,
        query: str,
        limit: int = 300,
        level: Optional[str] = None,
        tags: Optional[List[str]] = None,
    ) -> List[dict]:
        """Server-side embedding search."""
        
        response = self._request(
            "POST",
            "/records/embeddings/search",
            body={"query": query, "limit": limit, "level": level, "tags": tags},
        )
        
        return response["results"]
```

---

## 4. Getting Started

### Installation

```bash
# Clone the repository
git clone https://github.com/HKUDS/OpenSpace.git
cd OpenSpace

# Or lightweight clone (skip assets)
git clone --filter=blob:none --sparse https://github.com/HKUDS/OpenSpace.git
cd OpenSpace
git sparse-checkout set '/*' '!assets/'

# Install
pip install -e .

# Verify
openspace-mcp --help
```

### Configuration

#### 1. Create `.env` File

Copy from `openspace/.env.example`:

```bash
# LLM API Keys (at least one required)
OPENROUTER_API_KEY=sk-xxx  # For openrouter/* models
ANTHROPIC_API_KEY=sk-ant-xxx  # For anthropic/* models
OPENAI_API_KEY=sk-xxx  # For openai/* models

# OpenSpace Cloud (optional)
OPENSPACE_API_KEY=sk_xxxxxxxxxxxxxxxx

# Optional: Embedding API (uses local BAAI/bge-small-en-v1.5 by default)
# EMBEDDING_BASE_URL=https://api.openai.com
# EMBEDDING_API_KEY=sk-xxx
# EMBEDDING_MODEL=text-embedding-3-small

# Optional: E2B Sandbox (if sandbox mode enabled)
# E2B_API_KEY=xxx
```

#### 2. Configure Grounding Backends

Edit `openspace/config/config_grounding.json`:

```json
{
  "enabled_backends": [
    {"name": "shell", "config": {...}},
    {"name": "gui", "config": {...}},
    {"name": "mcp", "config": {...}},
    {"name": "web", "config": {...}}
  ]
}
```

#### 3. Agent Integration (MCP Config)

For Claude Code, OpenClaw, nanobot, etc.:

```json
{
  "mcpServers": {
    "openspace": {
      "command": "openspace-mcp",
      "toolTimeout": 600,
      "env": {
        "OPENSPACE_HOST_SKILL_DIRS": "/path/to/your/agent/skills",
        "OPENSPACE_WORKSPACE": "/path/to/OpenSpace",
        "OPENSPACE_API_KEY": "sk-xxx"
      }
    }
  }
}
```

#### 4. Copy Host Skills

```bash
# These skills teach your agent when and how to use OpenSpace
cp -r OpenSpace/openspace/host_skills/delegate-task/ /path/to/your/agent/skills/
cp -r OpenSpace/openspace/host_skills/skill-discovery/ /path/to/your/agent/skills/
```

### Usage Modes

#### Mode 1: As Agent MCP Server (Recommended)

Your agent has access to 4 tools:

```
# Agent conversation
User: "Monitor my Docker containers and restart the highest memory one"

Agent: (checks available skills via search_skills)
Agent: (finds relevant skill, uses execute_task)
Agent: execute_task(task="Monitor Docker containers...")

# OpenSpace executes with skills
# Returns result + any evolved skills
```

#### Mode 2: Direct CLI Usage

```bash
# Interactive mode
openspace

# Single task execution
openspace --model "anthropic/claude-sonnet-4-5" \
    --query "Create a monitoring dashboard for my Docker containers"
```

#### Mode 3: Python API

```python
import asyncio
from openspace import OpenSpace, OpenSpaceConfig

async def main():
    config = OpenSpaceConfig(
        llm_model="openrouter/anthropic/claude-sonnet-4.5",
        workspace_dir="/path/to/workspace",
    )
    
    async with OpenSpace(config=config) as cs:
        result = await cs.execute("Analyze GitHub trending repos")
        print(result["response"])
        
        for skill in result.get("evolved_skills", []):
            print(f"Evolved: {skill['name']} ({skill['origin']})")

asyncio.run(main())
```

### Local Dashboard

Browse skills, track lineage, compare diffs:

```bash
# Terminal 1: Start backend API
openspace-dashboard --port 7788

# Terminal 2: Start frontend dev server
cd frontend
npm install
npm run dev
```

Open http://localhost:5173 to view the dashboard.

---

## 5. Skill System

### SKILL.md Format

Every skill directory contains:

```
my-skill/
├── SKILL.md           # Definition (required)
├── .skill_id          # Persistent ID (auto-generated)
└── .upload_meta.json  # Cloud upload metadata (after evolution)
```

**SKILL.md Template**:

```markdown
---
name: skill-name
description: One-line description
---

# Skill Name

## When to Use

- Condition 1
- Condition 2

## Core Technique

Main approach explanation.

## Step-by-Step Workflow

### Step 1: First Step

```
tool_call
parameter: value
```

### Step 2: Second Step

```
another_tool
arg: value
```

## Complete Example

Full working example.

## Troubleshooting

Common issues and solutions.
```

### Skill Discovery Pipeline

Three-stage hybrid search:

#### Stage 1: BM25 Prefilter

```python
def prefilter(self, query: str, threshold: float = 0.3) -> List[SkillMeta]:
    """BM25 keyword matching."""
    
    # Tokenize query
    query_tokens = self._tokenize(query)
    
    # Score each skill
    scores = {}
    for skill_id, skill in self._skills.items():
        score = self._bm25.score(query_tokens, skill.name, skill.description)
        if score >= threshold:
            scores[skill_id] = score
    
    return sorted(scores.items(), key=lambda x: -x[1])
```

#### Stage 2: Embedding Ranking

```python
async def rank_with_embeddings(
    self,
    query: str,
    candidates: List[Tuple[str, float]],
) -> List[Tuple[str, float]]:
    """Embedding similarity re-ranking."""
    
    # Generate query embedding
    query_emb = self._embedding_client.embed(query)
    
    # Load candidate embeddings (cached)
    candidate_embs = self._load_embeddings(candidates)
    
    # Cosine similarity
    scores = cosine_similarity(query_emb, candidate_embs)
    
    # Hybrid fusion: BM25 score * 0.3 + embedding score * 0.7
    fused = fuse_scores(candidates, scores, bm25_weight=0.3)
    
    return sorted(fused, key=lambda x: -x[1])
```

#### Stage 3: LLM Selection

```python
async def _llm_select(
    self,
    task: str,
    ranked: List[Tuple[str, float]],
) -> List[str]:
    """LLM makes final selection based on task context."""
    
    prompt = f"""
Task: {task}

Available skills:
{self._format_candidates(ranked[:20])}

Select skills that are most relevant. Return skill_ids as JSON array.
"""
    
    response = await self._llm_client.complete(prompt)
    return json.loads(response)
```

### Skill Registry and Ranking

**Priority Order**:

1. User-configured skill dirs (highest)
2. Config-file skill dirs
3. Built-in skills (lowest)

**Shadowing**: Skills with the same name in earlier dirs shadow later ones.

**Skill Identity**:

- `.skill_id` sidecar file stores persistent ID
- Format: `{name}__imp_{uuid[:8]}` for imported skills
- Format: `{name}__v{gen}_{uuid[:8]}` for evolved skills

### .skill_id Sidecar Files

```python
def _read_or_create_skill_id(name: str, skill_dir: Path) -> str:
    """Read or create .skill_id sidecar."""
    
    id_file = skill_dir / ".skill_id"
    
    if id_file.exists():
        return id_file.read_text().strip()
    
    # Generate new ID
    new_id = f"{name}__imp_{uuid.uuid4().hex[:8]}"
    id_file.write_text(new_id + "\n")
    
    return new_id
```

**Benefits**:
- Portable (survives directory moves)
- Deterministic (consistent across restarts)
- Unique (UUID-based)

---

## 6. Evolution Modes

### FIX — Repair Broken Skills

**Purpose**: Fix skills that have broken due to API changes, tool errors, or degraded instructions.

**Trigger**:
- Post-execution analysis detects failure
- Tool degradation monitor flags dependent skills
- Metric monitor finds high fallback rate

**Process**:

```python
async def _evolve_fix(self, ctx: EvolutionContext) -> SkillRecord:
    """Fix a skill in-place."""
    
    # 1. Read current skill content
    skill_content = ctx.skill_dirs[0] / "SKILL.md"
    content = skill_content.read_text()
    
    # 2. Build prompt with fix direction
    prompt = f"""
Current skill:
{content}

Problem: {ctx.suggestion.reason}

Fix the skill instructions to address this issue.
"""
    
    # 3. Agent loop: analyze, edit, validate
    result = await self._agent_loop(prompt, tools=ctx.available_tools)
    
    # 4. Apply diff with retry
    for attempt in range(_MAX_EVOLUTION_ATTEMPTS):
        success = await apply_fix(result.edits)
        if success:
            break
    
    # 5. Validate new skill
    if not await self._validate_skill(ctx.skill_dirs[0]):
        raise EvolutionError("Skill validation failed")
    
    # 6. Persist (same skill_id, new version)
    return await self._store.save_fix(result)
```

**Example**:

```
# Before: API endpoint v1
run_shell
command: curl https://api.example.com/v1/weather

# After FIX evolution: API endpoint v2
run_shell
command: curl https://api.example.com/v2/weather?units=metric
```

### DERIVED — Create Enhanced Versions

**Purpose**: Create improved or specialized versions from successful parent skills.

**Trigger**:
- Post-execution analysis finds successful pattern worth generalizing
- Metric monitor finds high applied rate + moderate effective rate

**Process**:

```python
async def _evolve_derived(self, ctx: EvolutionContext) -> SkillRecord:
    """Create enhanced version from parent skill."""
    
    # 1. Copy parent skill to new directory
    parent_dir = ctx.skill_dirs[0]
    new_name = _sanitize_skill_name(f"{ctx.parent_name}-enhanced")
    new_dir = parent_dir.parent / new_name
    shutil.copytree(parent_dir, new_dir)
    
    # 2. Generate new skill_id
    new_id = f"{new_name}__v{generation}_{uuid.uuid4().hex[:8]}"
    write_skill_id(new_dir, new_id)
    
    # 3. Agent loop: enhance instructions
    prompt = f"""
Parent skill: {parent_dir}/SKILL.md

Enhancement direction: {ctx.suggestion.reason}

Create an enhanced version that improves upon the parent.
"""
    
    result = await self._agent_loop(prompt, tools=ctx.available_tools)
    
    # 4. Apply enhancements
    await apply_derive(new_dir, result.edits)
    
    # 5. Record lineage (parent → child)
    lineage = SkillLineage(
        origin=EvolutionOrigin.DERIVED,
        generation=parent_generation + 1,
        parent_skill_ids=[parent_skill_id],
    )
    
    return await self._store.save_derived(new_dir, lineage)
```

**Example Evolution Chain**:

```
document-gen-fallback (imported, gen 0)
└── document-gen-fallback-enhanced (DERIVED, gen 1)
    └── document-gen-fallback-enhanced-enhanced (DERIVED, gen 2)
        └── document-gen-fallback-enhanced-enhanced-2794b4 (DERIVED, gen 3)
```

### CAPTURED — Extract New Patterns

**Purpose**: Capture novel reusable patterns from successful executions.

**Trigger**:
- Post-execution analysis finds new pattern not covered by existing skills

**Process**:

```python
async def _evolve_captured(self, ctx: EvolutionContext) -> SkillRecord:
    """Extract new skill from execution recording."""
    
    # 1. Create new skill directory
    new_name = _sanitize_skill_name(ctx.suggestion.skill_name)
    new_dir = get_skill_base_dir() / new_name
    new_dir.mkdir(parents=True, exist_ok=True)
    
    # 2. Generate skill_id
    new_id = f"{new_name}__imp_{uuid.uuid4().hex[:8]}"
    write_skill_id(new_dir, new_id)
    
    # 3. Agent loop: extract pattern from recording
    prompt = f"""
Execution recording:
{ctx.execution_recording}

Extract a reusable skill from this successful workflow.
Create SKILL.md with when-to-use, steps, and examples.
"""
    
    result = await self._agent_loop(prompt, tools=ctx.available_tools)
    
    # 4. Write SKILL.md
    skill_file = new_dir / "SKILL.md"
    skill_file.write_text(result.skill_content)
    
    # 5. Record lineage (no parent)
    lineage = SkillLineage(
        origin=EvolutionOrigin.CAPTURED,
        generation=1,
        parent_skill_ids=[],
        source_task_id=ctx.source_task_id,
    )
    
    return await self._store.save_captured(new_dir, lineage)
```

**Example**:

From a successful Docker monitoring task:

```markdown
---
name: docker-container-monitor-restart
description: Monitor Docker containers, find highest memory usage, restart gracefully
---

# Docker Container Monitor & Restart

## When to Use

- User asks to monitor Docker containers
- Need to identify and restart problematic containers

## Workflow

### Step 1: List containers with memory usage

```
run_shell
command: docker stats --no-stream --format "table {{.Name}}\t{{.MemUsage}}"
```

### Step 2: Parse and find highest

```python
import subprocess
output = subprocess.check_output(["docker", "stats", "--no-stream"])
containers = parse_docker_stats(output)
highest = max(containers, key=lambda c: c.memory_percent)
```

### Step 3: Graceful restart

```
run_shell
command: docker restart {container_name}
```
```

---

## 7. Quality Monitoring

### Skill Metrics

Tracked in `skill_records` table:

| Metric | Description | Calculation |
|--------|-------------|-------------|
| `total_selections` | Times skill was considered | Incremented on selection |
| `total_applied` | Times skill was actually used | Incremented on apply |
| `total_completions` | Times skill led to success | Incremented on success |
| `total_fallbacks` | Times skill failed | Incremented on failure |

**Derived Metrics**:

```python
applied_rate = total_applied / max(total_selections, 1)
completion_rate = total_completions / max(total_applied, 1)
fallback_rate = total_fallbacks / max(total_applied, 1)
effective_rate = completion_rate * applied_rate
```

### Thresholds for Evolution

| Condition | Threshold | Evolution Type |
|-----------|-----------|----------------|
| High fallback rate | > 0.4 | FIX |
| Low completion rate | < 0.35 | FIX |
| High applied + low effective | applied > 0.4, effective < 0.55 | DERIVED |
| Novel successful pattern | N/A (LLM judgment) | CAPTURED |

### Tool Quality Tracking

`ToolQualityManager` monitors tool health:

```python
class ToolQualityManager:
    def record_outcome(self, tool_key: str, success: bool, latency_ms: float):
        """Record tool execution outcome."""
        
        self._records[tool_key].append(
            ToolQualityRecord(
                tool_key=tool_key,
                success=success,
                latency_ms=latency_ms,
                timestamp=datetime.now(),
            )
        )
    
    def get_problematic_tools(
        self,
        success_threshold: float = 0.7,
        min_samples: int = 5,
    ) -> List[str]:
        """Find tools with degraded performance."""
        
        problematic = []
        for tool_key, records in self._records.items():
            if len(records) < min_samples:
                continue
            
            success_rate = sum(r.success for r in records) / len(records)
            if success_rate < success_threshold:
                problematic.append(tool_key)
        
        return problematic
```

### Cascade Evolution

When a tool degrades:

```python
async def process_tool_degradation(self, tool_key: str):
    """Evolve all skills depending on problematic tool."""
    
    # 1. Find dependent skills
    dependent_skills = self._store.get_skills_using_tool(tool_key)
    
    # 2. Skip already-addressed skills
    already_fixed = self._addressed_degradations.get(tool_key, set())
    to_evolve = [s for s in dependent_skills if s.skill_id not in already_fixed]
    
    # 3. Batch evolve
    tasks = []
    for skill in to_evolve:
        ctx = EvolutionContext(
            trigger=EvolutionTrigger.TOOL_DEGRADATION,
            suggestion=EvolutionSuggestion(
                evolution_type=EvolutionType.FIX,
                target_skill_ids=[skill.skill_id],
                reason=f"Tool {tool_key} degraded, add fallback",
            ),
        )
        tasks.append(self.evolver.evolve(ctx))
    
    # 4. Wait for completion (with semaphore limit)
    results = await asyncio.gather(*tasks)
    
    # 5. Track addressed skills
    for result in results:
        if result:
            already_fixed.add(result.skill_id)
    
    self._addressed_degradations[tool_key] = already_fixed
```

---

## 8. Cloud Community

### open-space.cloud Platform

**Features**:
- Skill browsing and search
- Version lineage visualization
- Diff comparison between versions
- Group management for team sharing
- Upload/download CLI tools

### Skill Sharing Modes

| Visibility | Description |
|------------|-------------|
| `public` | Visible to all users, appears in search |
| `private` | Only visible to uploader |
| `group` | Visible to group members only |

### CLI Tools

#### Download Skill

```bash
openspace-download-skill <skill_id> [--output-dir /path/to/skills]
```

**Process**:
1. Fetch record metadata
2. Download artifact zip
3. Extract to output directory
4. Register in local skill registry

#### Upload Skill

```bash
openspace-upload-skill /path/to/skill/dir \
    --visibility public \
    --tags "docker,monitoring,restart"
```

**Metadata** (auto-saved to `.upload_meta.json` after evolution):

```json
{
  "origin": "derived",
  "parent_skill_ids": ["parent-skill-abc123"],
  "change_summary": "Added graceful restart with health check",
  "created_by": "claude-code-agent",
  "tags": ["docker", "monitoring"]
}
```

**Upload Process**:

```python
async def upload_skill(
    self,
    skill_dir: Path,
    visibility: str = "public",
) -> str:
    """Full upload workflow."""
    
    # 1. Read metadata (from .upload_meta.json or defaults)
    metadata = _read_upload_meta(skill_dir)
    
    # 2. Stage artifact (upload files)
    artifact_id, file_count = self.stage_artifact(skill_dir)
    
    # 3. Compute diff vs parent
    if metadata.get("parent_skill_ids"):
        parent = self.fetch_record(metadata["parent_skill_ids"][0])
        diff = compute_unified_diff(parent, skill_dir)
    else:
        diff = ""
    
    # 4. Create record
    record = self.create_record(
        artifact_id=artifact_id,
        skill_id=_read_skill_id(skill_dir),
        metadata={**metadata, "visibility": visibility},
        diff=diff,
    )
    
    return record["record_id"]
```

### Hybrid Search (Cloud)

```python
def search_record_embeddings(
    self,
    query: str,
    limit: int = 300,
    level: Optional[str] = None,
    tags: Optional[List[str]] = None,
) -> List[dict]:
    """Server-side embedding search."""
    
    # 1. Generate query embedding (client-side)
    query_emb = self._embedding_client.embed(query)
    
    # 2. Server searches record embeddings
    response = self._request(
        "POST",
        "/records/embeddings/search",
        body={
            "query": query,
            "query_embedding": query_emb,
            "limit": limit,
            "level": level,
            "tags": tags,
        },
    )
    
    # 3. Server returns ranked results
    return response["results"]
```

---

## 9. Development Guide

### Project Structure

```
OpenSpace/
├── openspace/
│   ├── tool_layer.py           # Main class & config
│   ├── mcp_server.py           # MCP server (4 tools)
│   ├── __main__.py             # CLI entry point
│   ├── dashboard_server.py     # Web dashboard API
│   │
│   ├── agents/
│   │   ├── base.py             # Base agent class
│   │   └── grounding_agent.py  # Execution agent
│   │
│   ├── grounding/
│   │   ├── core/
│   │   │   ├── grounding_client.py    # Backend orchestration
│   │   │   ├── search_tools.py        # Tool RAG
│   │   │   ├── quality/               # Tool quality tracking
│   │   │   ├── security/              # Policies, sandboxing
│   │   │   └── tool/                  # Tool abstraction
│   │   └── backends/
│   │       ├── shell/          # Shell commands
│   │       ├── gui/            # Anthropic Computer Use
│   │       ├── mcp/            # Model Context Protocol
│   │       └── web/            # Web search
│   │
│   ├── skill_engine/
│   │   ├── registry.py         # Discovery & ranking
│   │   ├── analyzer.py         # Post-execution analysis
│   │   ├── evolver.py          # FIX/DERIVED/CAPTURED
│   │   ├── patch.py            # Diff application
│   │   ├── store.py            # SQLite persistence
│   │   └── types.py            # Type definitions
│   │
│   ├── cloud/
│   │   ├── client.py           # HTTP client
│   │   ├── search.py           # Hybrid search
│   │   ├── embedding.py        # Embedding generation
│   │   └── cli/                # CLI tools
│   │
│   ├── config/                 # Configuration
│   ├── llm/                    # LiteLLM wrapper
│   ├── prompts/                # Prompt templates
│   └── utils/                  # Logging, telemetry
│
├── frontend/                   # Dashboard UI (React)
├── gdpval_bench/               # Benchmark experiments
└── showcase/                   # My Daily Monitor demo
```

### Adding a New Skill

```bash
# Create skill directory
mkdir -p /path/to/skills/my-new-skill

# Create SKILL.md
cat > /path/to/skills/my-new-skill/SKILL.md << 'EOF'
---
name: my-new-skill
description: What this skill does
---

# My New Skill

## When to Use

- Condition 1
- Condition 2

## Workflow

Step-by-step instructions.
EOF

# Register with OpenSpace (auto-discovered on next run)
```

### Debugging Evolution

```python
# Enable debug logging
export OPENSPACE_DEBUG=true

# Watch evolution logs
tail -f logs/mcp_server.log
tail -f logs/openspace.log

# Inspect skill database
sqlite3 .openspace/openspace.db "SELECT * FROM skill_records;"
```

### Testing Evolution

```python
import asyncio
from pathlib import Path
from openspace.tool_layer import OpenSpace, OpenSpaceConfig
from openspace.skill_engine.evolver import EvolutionContext, EvolutionSuggestion, EvolutionType

async def test_evolution():
    config = OpenSpaceConfig(llm_model="test-model")
    
    async with OpenSpace(config=config) as cs:
        # Trigger FIX evolution
        ctx = EvolutionContext(
            trigger=EvolutionTrigger.ANALYSIS,
            suggestion=EvolutionSuggestion(
                evolution_type=EvolutionType.FIX,
                target_skill_ids=["my-skill-abc123"],
                reason="Test fix direction",
            ),
        )
        
        result = await cs._skill_evolver.evolve(ctx)
        print(f"Evolved: {result}")

asyncio.run(test_evolution())
```

### Running Benchmarks

```bash
# Run GDPVal benchmark
python -m gdpval_bench

# View results
cat gdpval_bench/results.json
```

---

## Quick Reference

### Environment Variables

| Variable | Description | Default |
|----------|-------------|---------|
| `OPENSPACE_API_KEY` | Cloud API key | (none) |
| `OPENSPACE_HOST_SKILL_DIRS` | Skill directories | (none) |
| `OPENSPACE_WORKSPACE` | Workspace root | (project root) |
| `OPENSPACE_MODEL` | LLM model | openrouter/claude-sonnet-4.5 |
| `OPENSPACE_MAX_ITERATIONS` | Max agent iterations | 20 |
| `OPENSPACE_ENABLE_RECORDING` | Enable recording | true |
| `OPENSPACE_BACKEND_SCOPE` | Backend scope | all backends |

### MCP Tools

| Tool | Description |
|------|-------------|
| `execute_task` | Delegate task to OpenSpace |
| `search_skills` | Search local + cloud skills |
| `fix_skill` | Manually fix a skill |
| `upload_skill` | Upload skill to cloud |

### Evolution Types

| Type | Purpose | Result |
|------|---------|--------|
| FIX | Repair broken skills | In-place update |
| DERIVED | Enhanced version | New skill directory |
| CAPTURED | Extract new pattern | Brand new skill |

### Skill Metrics

| Metric | Good | Bad |
|--------|------|-----|
| Applied Rate | > 0.5 | < 0.3 |
| Completion Rate | > 0.7 | < 0.5 |
| Fallback Rate | < 0.2 | > 0.4 |

---

## Further Reading

- [SKILL.md Format Reference](openspace/skills/README.md)
- [Configuration Guide](openspace/config/README.md)
- [Cloud API Documentation](https://open-space.cloud/docs)
- [GDPVal Benchmark Results](gdpval_bench/README.md)
- [My Daily Monitor Showcase](showcase/README.md)
