# OpenSpace Skill Evolution Engine - Deep Dive

A comprehensive technical deep-dive into OpenSpace's self-evolving skill engine - the system that transforms static AI agent patterns into adaptive, learning systems.

---

## Table of Contents

1. [Overview](#1-overview)
2. [Skill Evolution Architecture](#2-skill-evolution-architecture)
3. [Evolution Triggers](#3-evolution-triggers)
4. [Evolution Modes](#4-evolution-modes)
5. [Skill Analyzer](#5-skill-analyzer)
6. [Skill Evolver](#6-skill-evolver)
7. [Skill Store](#7-skill-store)
8. [Quality Monitoring](#8-quality-monitoring)
9. [Safety Mechanisms](#9-safety-mechanisms)
10. [Implementation Reference](#10-implementation-reference)

---

## 1. Overview

### What is the Skill Evolution Engine?

The Skill Evolution Engine is the core intelligence behind OpenSpace's self-improving capabilities. It continuously monitors skill performance, detects degradation and opportunities, and autonomously repairs or enhances skills without human intervention.

### Key Capabilities

| Capability | Description |
|------------|-------------|
| **Auto-Fix** | Repairs broken skills when APIs change or tools fail |
| **Auto-Improve** | Creates enhanced versions from successful patterns |
| **Auto-Learn** | Extracts new skills from novel successful workflows |
| **Quality Monitoring** | Tracks metrics and triggers evolution for underperformers |
| **Lineage Tracking** | Maintains full version history and parent-child relationships |

### Evolution in Action

```
Skill: document-gen-fallback (imported)
├── document-gen-fallback-enhanced (DERIVED, gen 1)
│   ├── document-gen-fallback-enhanced-enhanced (DERIVED, gen 2)
│   │   └── document-gen-fallback-enhanced-enhanced-2794b4 (DERIVED, gen 3)
│   └── document-gen-fallback-merged (DERIVED, gen 2)
└── document-gen-fallback-enhanced-9f3b1f (DERIVED, gen 1)
```

---

## 2. Skill Evolution Architecture

### System Components

The evolution engine consists of four primary components working in concert:

```
┌─────────────────────────────────────────────────────────────────┐
│                    Evolution Engine Architecture                 │
├─────────────────────────────────────────────────────────────────┤
│                                                                  │
│  ┌─────────────────┐     ┌─────────────────┐                    │
│  │ ExecutionAnalyzer│────▶│  SkillEvolver   │                    │
│  │                 │     │                 │                    │
│  │ - Post-execution│     │ - FIX evolution │                    │
│  │ - Pattern detect│     │ - DERIVED       │                    │
│  │ - Suggest evol. │     │ - CAPTURED      │                    │
│  └────────┬────────┘     └────────┬────────┘                    │
│           │                       │                              │
│           │                       ▼                              │
│           │            ┌─────────────────┐                       │
│           │            │   SkillStore    │                       │
│           │            │                 │                       │
│           │            │ - SQLite DB     │                       │
│           │            │ - Version DAG   │                       │
│           │            │ - Metrics       │                       │
│           │            └─────────────────┘                       │
│           │                                                      │
│  ┌────────▼────────┐     ┌─────────────────┐                    │
│  │ MetricMonitor   │────▶│ ToolQualityMgr  │                    │
│  │                 │     │                 │                    │
│  │ - Periodic scan │     │ - Success rates │                    │
│  │ - Threshold chk │     │ - Latency track │                    │
│  │ - Cascade evol. │     │ - Degradation   │                    │
│  └─────────────────┘     └─────────────────┘                    │
│                                                                  │
└─────────────────────────────────────────────────────────────────┘
```

### Core Component Classes

#### 1. SkillRegistry (`registry.py`)

Discovers and ranks skills for task relevance:

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

#### 2. SkillStore (`store.py`)

SQLite persistence for skill records and lineage:

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

#### 3. SkillEvolver (`evolver.py`)

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
```

#### 4. ExecutionAnalyzer (`analyzer.py`)

Post-execution analysis and evolution suggestion:

```python
class ExecutionAnalyzer:
    async def analyze(self, execution_recording: dict) -> List[EvolutionSuggestion]:
        """Analyze execution and suggest evolutions."""
        
        # Review applied skills
        # Detect success/failure patterns
        # Identify novel patterns worth capturing
        # Generate evolution suggestions
```

### Evolution Flow - End to End

```
┌─────────────────────────────────────────────────────────────────────┐
│                        Complete Evolution Flow                       │
└─────────────────────────────────────────────────────────────────────┘

     ┌──────────────┐
     │  Task Input  │
     └──────┬───────┘
            │
            ▼
     ┌─────────────────┐
     │ SkillRegistry   │
     │ select_relevant │
     └──────┬──────────┘
            │
            ▼
     ┌─────────────────┐
     │ GroundingAgent  │
     │ execute_task    │
     └──────┬──────────┘
            │
            ▼
     ┌─────────────────┐
     │ ExecutionRecord │
     │ (tool calls,    │
     │  outcomes,      │
     │  screenshots)   │
     └──────┬──────────┘
            │
    ┌───────┴────────┐
    │                │
    ▼                ▼
┌──────────────┐  ┌──────────────┐
│SkillAnalyzer │  │MetricMonitor │
│POST-EXECUTION│  │PERIODIC SCAN │
└──────┬───────┘  └──────┬───────┘
       │                 │
       │  ┌──────────────┘
       │  │
       ▼  ▼
┌─────────────────┐
│ EvolutionSuggestion │
│ - evolution_type    │
│ - target_skill_ids  │
│ - reason            │
│ - priority          │
└──────┬──────────┘
       │
       ▼
┌─────────────────┐
│ ConfirmationGate│
│ - Anti-loop chk │
│ - Safety check  │
└──────┬──────────┘
       │ APPROVED
       ▼
┌─────────────────┐
│  SkillEvolver   │
│  evolve(ctx)    │
└──────┬──────────┘
       │
       ├──────────────┬──────────────┐
       │              │              │
       ▼              ▼              ▼
┌───────────┐  ┌───────────┐  ┌───────────┐
│ FIX       │  │ DERIVED   │  │ CAPTURED  │
│ In-place  │  │ New ver.  │  │ New skill │
└─────┬─────┘  └─────┬─────┘  └─────┬─────┘
      │              │               │
      └──────────────┴───────────────┘
                     │
                     ▼
              ┌──────────────┐
              │  SkillStore  │
              │  persist()   │
              └──────────────┘
```

### Version DAG and Lineage Tracking

Every evolved skill maintains complete lineage information:

```python
class SkillLineage:
    origin: EvolutionOrigin  # FIX, DERIVED, CAPTURED
    generation: int          # Generation number (1, 2, 3...)
    parent_skill_ids: List[str]  # Parent skill IDs
    source_task_id: Optional[str]  # For CAPTURED skills
```

**Version DAG Example:**

```
document-gen-fallback__imp_abc123 (gen 0, imported)
│
├─ document-gen-fallback-enhanced__v1_def456 (gen 1, DERIVED)
│  │
│  ├─ document-gen-fallback-enhanced-enhanced__v2_ghi789 (gen 2, DERIVED)
│  │  │
│  │  └─ document-gen-fallback-enhanced-enhanced-2794b4__v3_jkl012 (gen 3, DERIVED)
│  │
│  └─ document-gen-fallback-merged__v2_mno345 (gen 2, DERIVED)
│
└─ document-gen-fallback-enhanced-9f3b1f__v1_pqr678 (gen 1, DERIVED)
```

**Database Schema for Lineage:**

```sql
CREATE TABLE skill_lineage_parents (
    skill_id TEXT PRIMARY KEY,
    parent_skill_ids TEXT,  -- JSON array
    origin TEXT,            -- 'fix', 'derived', 'captured'
    generation INTEGER,
    source_task_id TEXT,
    FOREIGN KEY (skill_id) REFERENCES skill_records(skill_id)
);

CREATE INDEX idx_lineage_parent ON skill_lineage_parents(parent_skill_ids);
```

**Lineage Query Example:**

```python
def get_full_lineage(self, skill_id: str) -> List[SkillLineage]:
    """Retrieve complete version history for a skill."""
    
    cursor = self._conn.execute("""
        WITH RECURSIVE lineage_cte AS (
            SELECT skill_id, parent_skill_ids, origin, generation, 0 as depth
            FROM skill_lineage_parents
            WHERE skill_id = ?
            
            UNION ALL
            
            SELECT p.skill_id, p.parent_skill_ids, p.origin, p.generation, c.depth + 1
            FROM skill_lineage_parents p
            JOIN lineage_cte c ON p.skill_id IN (SELECT value FROM json_each(c.parent_skill_ids))
        )
        SELECT * FROM lineage_cte ORDER BY depth DESC
    """, (skill_id,))
    
    return [SkillLineage(**row) for row in cursor.fetchall()]
```

---

## 3. Evolution Triggers

Three independent triggers ensure skills stay healthy and adaptive:

### 3.1 Post-Execution Analysis (After Every Task)

After every task execution, the `ExecutionAnalyzer` reviews the full recording:

**Analysis Scope:**
- Which skills were applied?
- Did they succeed or fail?
- What patterns could be extracted?
- What needs fixing?

**Process:**

```python
class ExecutionAnalyzer:
    async def analyze(self, execution_recording: dict) -> List[EvolutionSuggestion]:
        """Analyze full execution recording and suggest evolutions."""
        
        suggestions = []
        
        # 1. Extract skill judgments from recording
        for skill_id, judgment in execution_recording.get("skill_judgments", {}).items():
            if judgment["outcome"] == "failure":
                suggestions.append(EvolutionSuggestion(
                    evolution_type=EvolutionType.FIX,
                    target_skill_ids=[skill_id],
                    reason=f"Skill failed: {judgment['error_message']}",
                    priority=0.9,
                ))
            
            elif judgment["outcome"] == "success" and judgment.get("novel_pattern"):
                suggestions.append(EvolutionSuggestion(
                    evolution_type=EvolutionType.DERIVED,
                    target_skill_ids=[skill_id],
                    reason="Successful pattern worth generalizing",
                    priority=0.85,
                ))
        
        # 2. Check for novel workflows not covered by existing skills
        if execution_recording.get("uncaptured_pattern"):
            suggestions.append(EvolutionSuggestion(
                evolution_type=EvolutionType.CAPTURED,
                target_skill_ids=[],
                skill_name=generate_skill_name(execution_recording),
                reason="Novel successful workflow detected",
                priority=0.8,
            ))
        
        return suggestions
```

**Suggestion Format:**

```python
@dataclass
class EvolutionSuggestion:
    evolution_type: EvolutionType
    target_skill_ids: List[str]
    reason: str
    priority: float  # 0.0 to 1.0
    skill_name: Optional[str] = None  # For CAPTURED
    source_task_id: Optional[str] = None
```

### 3.2 Tool Degradation Monitoring

When tool success rates drop, affected skills are automatically evolved:

**ToolQualityManager:**

```python
class ToolQualityManager:
    def __init__(self, success_threshold: float = 0.7, min_samples: int = 5):
        self._records: Dict[str, List[ToolQualityRecord]] = defaultdict(list)
        self._success_threshold = success_threshold
        self._min_samples = min_samples
        self._addressed_degradations: Dict[str, Set[str]] = defaultdict(set)
    
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
    
    async def process_tool_degradation(self, tool_key: str, evolver: SkillEvolver):
        """Evolve all skills depending on problematic tool."""
        
        # 1. Find dependent skills
        dependent_skills = self._store.get_skills_using_tool(tool_key)
        
        # 2. Skip already-addressed skills (ANTI-LOOP)
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
            tasks.append(evolver.evolve(ctx))
        
        # 4. Wait for completion (with semaphore limit)
        results = await asyncio.gather(*tasks, return_exceptions=True)
        
        # 5. Track addressed skills (ANTI-LOOP: prevents infinite evolution cycles)
        for result in results:
            if result and not isinstance(result, Exception):
                already_fixed.add(result.skill_id)
        
        self._addressed_degradations[tool_key] = already_fixed
```

**Anti-Loop Guard:**

The `_addressed_degradations` tracking prevents infinite evolution cycles:

1. When a tool degrades, affected skills are evolved with fallbacks
2. These skills are marked as "addressed" for this specific tool
3. If the tool degrades again, already-addressed skills are skipped
4. If the tool recovers (success rate improves), the tracking is reset
5. Future degradation after recovery allows re-evaluation

### 3.3 Metric Monitor (Periodic Scans)

Periodic scan of skill health metrics triggers evolution for underperformers:

```python
class MetricMonitor:
    def __init__(self, store: SkillStore):
        self._store = store
    
    async def scan(self) -> List[EvolutionSuggestion]:
        """Scan all skills and suggest evolutions for underperformers."""
        
        suggestions = []
        records = self._store.get_all_records()
        
        for record in records:
            # Skip if not enough data
            if record.total_selections < 10:
                continue
            
            # Calculate metrics
            applied_rate = record.total_applied / max(record.total_selections, 1)
            completion_rate = record.total_completions / max(record.total_applied, 1)
            fallback_rate = record.total_fallbacks / max(record.total_applied, 1)
            effective_rate = completion_rate * applied_rate
            
            # Check thresholds
            if fallback_rate > 0.4:
                suggestions.append(EvolutionSuggestion(
                    evolution_type=EvolutionType.FIX,
                    target_skill_ids=[record.skill_id],
                    reason=f"High fallback rate: {fallback_rate:.2f}",
                    priority=0.9,
                ))
            
            elif completion_rate < 0.35:
                suggestions.append(EvolutionSuggestion(
                    evolution_type=EvolutionType.FIX,
                    target_skill_ids=[record.skill_id],
                    reason=f"Low completion rate: {completion_rate:.2f}",
                    priority=0.85,
                ))
            
            elif applied_rate > 0.4 and effective_rate < 0.55:
                suggestions.append(EvolutionSuggestion(
                    evolution_type=EvolutionType.DERIVED,
                    target_skill_ids=[record.skill_id],
                    reason=f"High applied ({applied_rate:.2f}) but low effective ({effective_rate:.2f})",
                    priority=0.75,
                ))
        
        return suggestions
```

**Thresholds Summary:**

| Metric | Threshold | Evolution Type | Priority |
|--------|-----------|----------------|----------|
| Fallback Rate | > 0.4 | FIX | 0.9 |
| Completion Rate | < 0.35 | FIX | 0.85 |
| Applied > 0.4 + Effective < 0.55 | DERIVED | 0.75 |
| Novel successful pattern | CAPTURED | 0.8 |

### 3.4 Confirmation Gates and Anti-Loop Guards

Before any evolution is executed, safety checks are performed:

```python
class ConfirmationGate:
    def __init__(self, store: SkillStore):
        self._store = store
        self._recent_evolutions: Dict[str, List[datetime]] = defaultdict(list)
    
    async def check(self, suggestion: EvolutionSuggestion) -> bool:
        """Run all confirmation checks. Return True if evolution should proceed."""
        
        # 1. Anti-loop: Check for rapid repeated evolution
        for skill_id in suggestion.target_skill_ids:
            recent = self._recent_evolutions[skill_id]
            # Filter to last hour
            recent = [t for t in recent if (datetime.now() - t).seconds < 3600]
            if len(recent) >= 3:
                log(f"Skipping {skill_id}: evolved 3 times in last hour")
                return False
            self._recent_evolutions[skill_id] = recent
        
        # 2. Safety: Check for prompt injection patterns in suggestion reason
        if self._contains_injection(suggestion.reason):
            log(f"Blocked evolution: potential prompt injection in reason")
            return False
        
        # 3. Validation: Ensure target skills exist
        for skill_id in suggestion.target_skill_ids:
            if not self._store.exists(skill_id):
                log(f"Target skill {skill_id} not found")
                return False
        
        return True
    
    def record_evolution(self, skill_id: str):
        """Record that an evolution occurred for anti-loop tracking."""
        self._recent_evolutions[skill_id].append(datetime.now())
    
    def _contains_injection(self, text: str) -> bool:
        """Check for common prompt injection patterns."""
        injection_patterns = [
            r"ignore previous",
            r"bypass.*constraint",
            r"override.*security",
            r"extract.*credential",
            r"system.*prompt",
        ]
        return any(re.search(p, text.lower()) for p in injection_patterns)
```

---

## 4. Evolution Modes

Three evolution modes handle different improvement scenarios:

### 4.1 FIX - In-Place Repair

**Purpose:** Repair broken skills due to API changes, tool errors, or degraded instructions.

**Triggers:**
- Post-execution analysis detects failure
- Tool degradation monitor flags dependent skills
- Metric monitor finds high fallback rate (> 0.4)

**Process:**

```python
class SkillEvolver:
    async def _evolve_fix(self, ctx: EvolutionContext) -> SkillRecord:
        """Fix a skill in-place."""
        
        # 1. Read current skill content
        skill_dir = ctx.skill_dirs[0]
        skill_file = skill_dir / "SKILL.md"
        content = skill_file.read_text()
        skill_id = read_skill_id(skill_dir)
        
        # 2. Build prompt with fix direction
        prompt = f"""
You are fixing a skill that has encountered issues.

Current skill (SKILL.md):
{content}

Problem detected: {ctx.suggestion.reason}

Your task:
1. Analyze the current skill and identify the root cause
2. Modify the SKILL.md to fix the issue
3. Ensure the fix is minimal and targeted
4. Preserve working sections unchanged

Return the complete fixed SKILL.md content.
"""
        
        # 3. Agent loop with retry
        async with self._semaphore:  # Concurrency limit
            for attempt in range(_MAX_EVOLUTION_ATTEMPTS):
                result = await self._agent_loop(
                    prompt=prompt,
                    tools=ctx.available_tools,
                    max_iterations=_MAX_EVOLUTION_ITERATIONS,
                )
                
                if result.success:
                    # 4. Apply diff with validation
                    diff_result = await self._apply_fix(
                        skill_dir=skill_dir,
                        new_content=result.content,
                    )
                    
                    if diff_result.success:
                        # 5. Validate new skill
                        if await self._validate_skill(skill_dir):
                            # 6. Persist (same skill_id, new version)
                            record = await self._store.save_fix(
                                skill_dir=skill_dir,
                                skill_id=skill_id,
                                change_summary=ctx.suggestion.reason,
                            )
                            return record
            
            raise EvolutionError(f"FIX evolution failed after {_MAX_EVOLUTION_ATTEMPTS} attempts")
```

**Diff Generation Options:**

```python
class DiffStrategy(Enum):
    FULL = "full"      # Replace entire file
    DIFF = "diff"      # Unified diff patch
    PATCH = "patch"    # Targeted section patches
```

**FIX Example:**

```markdown
# Before FIX (API v1)
## Step 2: Fetch Weather Data

```
run_shell
command: curl https://api.example.com/v1/weather?city=London
```

# After FIX (API v2)
## Step 2: Fetch Weather Data

```
run_shell
command: curl https://api.example.com/v2/weather?city=London&units=metric
```

Change: Updated from v1 to v2 API endpoint with units parameter
```

### 4.2 DERIVED - Create Enhanced Version

**Purpose:** Create improved or specialized versions from successful parent skills.

**Triggers:**
- Post-execution analysis finds successful pattern worth generalizing
- Metric monitor finds high applied rate + moderate effective rate

**Process:**

```python
class SkillEvolver:
    async def _evolve_derived(self, ctx: EvolutionContext) -> SkillRecord:
        """Create enhanced version from parent skill."""
        
        # 1. Copy parent skill to new directory
        parent_dir = ctx.skill_dirs[0]
        parent_name = parent_dir.name
        parent_id = read_skill_id(parent_dir)
        
        # Generate new name
        new_name = _sanitize_skill_name(f"{parent_name}-enhanced")
        new_dir = parent_dir.parent / new_name
        
        # Copy parent content
        shutil.copytree(parent_dir, new_dir)
        
        # 2. Generate new skill_id
        generation = self._store.get_generation(parent_id) + 1
        new_id = f"{new_name}__v{generation}_{uuid.uuid4().hex[:8]}"
        write_skill_id(new_dir, new_id)
        
        # 3. Build enhancement prompt
        prompt = f"""
You are creating an enhanced version of a skill.

Parent skill ({parent_dir}/SKILL.md):
{parent_dir.read_text()}

Enhancement direction: {ctx.suggestion.reason}

Your task:
1. Analyze the parent skill's strengths and weaknesses
2. Create improvements based on the enhancement direction
3. You may add new steps, clarify existing ones, or add troubleshooting
4. Maintain the same overall structure

Return the complete enhanced SKILL.md content.
"""
        
        # 4. Agent loop
        result = await self._agent_loop(prompt, tools=ctx.available_tools)
        
        # 5. Apply enhancements
        skill_file = new_dir / "SKILL.md"
        skill_file.write_text(result.content)
        
        # 6. Record lineage (parent → child)
        lineage = SkillLineage(
            origin=EvolutionOrigin.DERIVED,
            generation=generation,
            parent_skill_ids=[parent_id],
        )
        
        # 7. Persist
        record = await self._store.save_derived(
            skill_dir=new_dir,
            skill_id=new_id,
            lineage=lineage,
        )
        
        return record
```

**Example Evolution Chain:**

```
document-gen-fallback (imported, gen 0)
└── document-gen-fallback-enhanced (DERIVED, gen 1)
    └── document-gen-fallback-enhanced-enhanced (DERIVED, gen 2)
        └── document-gen-fallback-enhanced-enhanced-2794b4 (DERIVED, gen 3)
```

### 4.3 CAPTURED - Extract New Patterns

**Purpose:** Capture novel reusable patterns from successful executions that aren't covered by existing skills.

**Triggers:**
- Post-execution analysis finds new pattern not covered by existing skills
- LLM identifies novel workflow during execution review

**Process:**

```python
class SkillEvolver:
    async def _evolve_captured(self, ctx: EvolutionContext) -> SkillRecord:
        """Extract new skill from execution recording."""
        
        # 1. Create new skill directory
        new_name = _sanitize_skill_name(ctx.suggestion.skill_name)
        new_dir = get_skill_base_dir() / new_name
        new_dir.mkdir(parents=True, exist_ok=True)
        
        # 2. Generate skill_id (no version, first occurrence)
        new_id = f"{new_name}__imp_{uuid.uuid4().hex[:8]}"
        write_skill_id(new_dir, new_id)
        
        # 3. Build extraction prompt
        prompt = f"""
You are extracting a new reusable skill from a successful execution.

Execution Recording:
{json.dumps(ctx.execution_recording, indent=2)}

Your task:
1. Identify the core pattern that made this execution successful
2. Create a SKILL.md with:
   - name and description (YAML frontmatter)
   - "When to Use" section
   - "Core Technique" section  
   - "Step-by-Step Workflow" with code examples
   - "Complete Example" section
   - "Troubleshooting" section
3. Make the skill general enough to apply to similar tasks
4. Include specific tool calls and commands

Return the complete SKILL.md content.
"""
        
        # 4. Agent loop
        result = await self._agent_loop(prompt, tools=ctx.available_tools)
        
        # 5. Write SKILL.md
        skill_file = new_dir / "SKILL.md"
        skill_file.write_text(result.content)
        
        # 6. Record lineage (no parent for CAPTURED)
        lineage = SkillLineage(
            origin=EvolutionOrigin.CAPTURED,
            generation=1,
            parent_skill_ids=[],
            source_task_id=ctx.source_task_id,
        )
        
        # 7. Persist
        record = await self._store.save_captured(
            skill_dir=new_dir,
            skill_id=new_id,
            lineage=lineage,
        )
        
        return record
```

**Example Captured Skill:**

```markdown
---
name: docker-container-monitor-restart
description: Monitor Docker containers, find highest memory usage, restart gracefully
---

# Docker Container Monitor & Restart

## When to Use

- User asks to monitor Docker containers
- Need to identify and restart problematic containers
- Container health check is required

## Core Technique

Use `docker stats` for real-time metrics, parse output programmatically,
and perform graceful restart with health verification.

## Step-by-Step Workflow

### Step 1: List Containers with Memory Usage

```
run_shell
command: docker stats --no-stream --format "table {{.Name}}\t{{.MemUsage}}\t{{.MemPerc}}"
```

### Step 2: Parse and Find Highest Memory Consumer

```python
import subprocess
import re

def parse_docker_stats(output: str) -> List[dict]:
    lines = output.strip().split('\n')[1:]  # Skip header
    containers = []
    for line in lines:
        name, mem_usage, mem_perc = line.split('\t')
        containers.append({
            'name': name,
            'memory_usage': mem_usage,
            'memory_percent': float(mem_perc.rstrip('%')),
        })
    return containers

output = subprocess.check_output(
    ["docker", "stats", "--no-stream", "--format", 
     "table {{.Name}}\t{{.MemUsage}}\t{{.MemPerc}}"]
)
containers = parse_docker_stats(output)
highest = max(containers, key=lambda c: c['memory_percent'])
```

### Step 3: Graceful Restart with Health Check

```
run_shell
command: docker restart {container_name}
```

```
run_shell
command: docker inspect --format='{{.State.Health.Status}}' {container_name}
```

## Complete Example

```python
from openspace import OpenSpace, OpenSpaceConfig

async def main():
    config = OpenSpaceConfig(workspace_dir="/workspace")
    async with OpenSpace(config) as cs:
        result = await cs.execute(
            "Monitor Docker containers and restart the highest memory one"
        )
```

## Troubleshooting

### Container has no health check configured

Use `docker inspect --format='{{.State.Status}}'` instead.

### docker stats returns empty

Ensure containers are running: `docker ps --format '{{.Names}}'`
```

---

## 5. Skill Analyzer

The `ExecutionAnalyzer` performs post-execution analysis to detect patterns and suggest evolutions.

### 5.1 Recording Analysis

**Input Structure:**

```python
@dataclass
class ExecutionRecording:
    task_id: str
    instruction: str
    messages: List[Message]  # LLM conversation
    tool_calls: List[ToolCall]
    skill_judgments: Dict[str, SkillJudgment]
    screenshots: List[Screenshot]
    outcome: str  # success, failure, max_iterations
```

**Analysis Process:**

```python
class ExecutionAnalyzer:
    async def analyze(self, recording: ExecutionRecording) -> AnalysisResult:
        """Perform complete analysis of execution recording."""
        
        result = AnalysisResult()
        
        # 1. Analyze skill applications
        for skill_id, judgment in recording.skill_judgments.items():
            skill_analysis = await self._analyze_skill_application(judgment)
            result.skill_analyses[skill_id] = skill_analysis
        
        # 2. Detect patterns across tool calls
        pattern_result = await self._detect_patterns(recording.tool_calls)
        result.patterns = pattern_result
        
        # 3. Generate evolution suggestions
        result.suggestions = await self._generate_suggestions(
            skill_analyses=result.skill_analyses,
            patterns=result.patterns,
        )
        
        # 4. Persist analysis to store
        await self._store.save_analysis(recording.task_id, result)
        
        return result
```

### 5.2 Tool Call Examination

Analyzing tool call patterns for success/failure:

```python
class ExecutionAnalyzer:
    async def _analyze_skill_application(self, judgment: SkillJudgment) -> SkillAnalysis:
        """Analyze a single skill application."""
        
        analysis = SkillAnalysis(
            skill_id=judgment.skill_id,
            outcome=judgment.outcome,
            tool_calls_used=judgment.tool_calls,
        )
        
        # 1. Examine tool call success rates within this skill
        for tool_call in judgment.tool_calls:
            tool_result = await self._evaluate_tool_result(tool_call)
            analysis.tool_results.append(tool_result)
        
        # 2. Detect error patterns
        if judgment.outcome == "failure":
            error_analysis = await self._analyze_error(
                error_message=judgment.error_message,
                tool_calls=judgment.tool_calls,
            )
            analysis.error_analysis = error_analysis
        
        # 3. Check for novel patterns (successful but not in existing skill)
        if judgment.outcome == "success":
            novelty_score = await self._assess_novelty(
                tool_calls=judgment.tool_calls,
                skill_id=judgment.skill_id,
            )
            analysis.novelty_score = novelty_score
        
        return analysis
```

### 5.3 Success/Failure Pattern Detection

Using LLM to identify patterns:

```python
class ExecutionAnalyzer:
    async def _detect_patterns(self, tool_calls: List[ToolCall]) -> PatternResult:
        """Detect patterns in tool call sequences."""
        
        # Serialize tool calls for analysis
        serialized = self._serialize_tool_calls(tool_calls)
        
        prompt = f"""
Analyze this tool call sequence for patterns:

{serialized}

Questions to answer:
1. Are there repeated subsequences that could be abstracted?
2. Are there fallback patterns (retry with different approach)?
3. Are there tool combinations that work well together?
4. Is there a novel approach not covered by existing skills?

Return analysis as JSON.
"""
        
        response = await self._llm_client.complete(prompt)
        return PatternResult.from_json(response)
    
    async def _assess_novelty(
        self,
        tool_calls: List[ToolCall],
        skill_id: str,
    ) -> float:
        """Assess how novel this execution pattern is compared to existing skills."""
        
        # Get existing skill patterns
        skill_content = self._store.get_skill_content(skill_id)
        
        prompt = f"""
Compare this execution pattern to the existing skill:

Execution:
{self._serialize_tool_calls(tool_calls)}

Existing Skill:
{skill_content}

How novel is this pattern? (0.0 = identical, 1.0 = completely new)
Return a score between 0.0 and 1.0.
"""
        
        response = await self._llm_client.complete(prompt)
        return float(response.strip())
```

### 5.4 Evolution Suggestion Generation

Generating actionable evolution suggestions:

```python
class ExecutionAnalyzer:
    async def _generate_suggestions(
        self,
        skill_analyses: Dict[str, SkillAnalysis],
        patterns: PatternResult,
    ) -> List[EvolutionSuggestion]:
        """Generate evolution suggestions from analysis."""
        
        suggestions = []
        
        for skill_id, analysis in skill_analyses.items():
            # Failed skills -> FIX suggestion
            if analysis.outcome == "failure":
                suggestions.append(EvolutionSuggestion(
                    evolution_type=EvolutionType.FIX,
                    target_skill_ids=[skill_id],
                    reason=f"Skill failed: {analysis.error_analysis.summary}",
                    priority=0.9,
                ))
            
            # Successful with high novelty -> DERIVED suggestion
            elif analysis.novelty_score > 0.7:
                suggestions.append(EvolutionSuggestion(
                    evolution_type=EvolutionType.DERIVED,
                    target_skill_ids=[skill_id],
                    reason=f"Novel successful pattern (novelty: {analysis.novelty_score:.2f})",
                    priority=0.85,
                ))
        
        # Completely novel workflow -> CAPTURED suggestion
        if patterns.novel_workflow_detected:
            suggestions.append(EvolutionSuggestion(
                evolution_type=EvolutionType.CAPTURED,
                target_skill_ids=[],
                skill_name=patterns.suggested_name,
                reason="Novel workflow not covered by existing skills",
                priority=0.8,
            ))
        
        # Sort by priority
        suggestions.sort(key=lambda s: s.priority, reverse=True)
        
        return suggestions
```

---

## 6. Skill Evolver

The `SkillEvolver` executes evolution with an autonomous agent loop.

### 6.1 Autonomous Fix Exploration

The evolver uses an agent loop to explore and implement fixes:

```python
class SkillEvolver:
    def __init__(
        self,
        store: SkillStore,
        llm_client: LLMClient,
        max_attempts: int = 3,
        max_iterations: int = 15,
        concurrency_limit: int = 3,
    ):
        self._store = store
        self._llm_client = llm_client
        self._max_attempts = max_attempts
        self._max_iterations = max_iterations
        self._semaphore = asyncio.Semaphore(concurrency_limit)
    
    async def _agent_loop(
        self,
        prompt: str,
        tools: List[Tool],
        max_iterations: int,
    ) -> AgentResult:
        """Run autonomous agent loop for evolution."""
        
        messages = [
            {"role": "system", "content": self._system_prompt},
            {"role": "user", "content": prompt},
        ]
        
        for iteration in range(max_iterations):
            # LLM decides next action
            response = await self._llm_client.chat(messages, tools=tools)
            
            if response.tool_calls:
                for tool_call in response.tool_calls:
                    result = await self._execute_tool(tool_call)
                    messages.append({
                        "role": "assistant",
                        "tool_call": tool_call,
                    })
                    messages.append({
                        "role": "tool",
                        "tool_call_id": tool_call.id,
                        "content": result,
                    })
            else:
                # Final content response
                return AgentResult(
                    success=True,
                    content=response.content,
                    iterations=iteration,
                )
        
        return AgentResult(
            success=False,
            error="Max iterations reached",
            iterations=max_iterations,
        )
```

### 6.2 Root Cause Analysis

The evolver performs root cause analysis before applying fixes:

```python
class SkillEvolver:
    async def _analyze_root_cause(
        self,
        skill_content: str,
        error_info: dict,
    ) -> RootCauseAnalysis:
        """Perform root cause analysis for a skill failure."""
        
        prompt = f"""
Analyze the root cause of this skill failure.

Skill Content:
{skill_content}

Error Information:
- Error Type: {error_info.get('type')}
- Error Message: {error_info.get('message')}
- Tool Call: {error_info.get('tool_call')}
- Context: {error_info.get('context')}

Perform root cause analysis:
1. What specific instruction or assumption caused the failure?
2. Is this an API change, missing fallback, or incorrect guidance?
3. What is the minimal fix needed?

Return analysis as JSON with:
- root_cause: string
- category: 'api_change' | 'missing_fallback' | 'incorrect_guidance' | 'other'
- fix_description: string
- confidence: float (0.0-1.0)
"""
        
        response = await self._llm_client.complete(prompt)
        return RootCauseAnalysis.from_json(response)
```

### 6.3 Diff Generation (FULL/DIFF/PATCH)

Three strategies for applying skill updates:

```python
class DiffApplier:
    async def apply(
        self,
        skill_dir: Path,
        new_content: str,
        strategy: DiffStrategy,
    ) -> ApplyResult:
        """Apply evolution changes to skill."""
        
        skill_file = skill_dir / "SKILL.md"
        old_content = skill_file.read_text()
        
        if strategy == DiffStrategy.FULL:
            # Replace entire file
            return await self._apply_full(skill_file, new_content)
        
        elif strategy == DiffStrategy.DIFF:
            # Generate and apply unified diff
            return await self._apply_diff(skill_file, old_content, new_content)
        
        elif strategy == DiffStrategy.PATCH:
            # Apply targeted section patches
            return await self._apply_patch(skill_file, old_content, new_content)
    
    async def _apply_full(self, skill_file: Path, new_content: str) -> ApplyResult:
        """Full file replacement."""
        try:
            skill_file.write_text(new_content)
            return ApplyResult(success=True)
        except Exception as e:
            return ApplyResult(success=False, error=str(e))
    
    async def _apply_diff(
        self,
        skill_file: Path,
        old_content: str,
        new_content: str,
    ) -> ApplyResult:
        """Generate and apply unified diff."""
        
        # Generate diff
        diff = difflib.unified_diff(
            old_content.splitlines(keepends=True),
            new_content.splitlines(keepends=True),
            fromfile="a/SKILL.md",
            tofile="b/SKILL.md",
        )
        diff_text = "".join(diff)
        
        # Apply with patch command
        try:
            process = await asyncio.create_subprocess_exec(
                "patch",
                str(skill_file),
                stdin=asyncio.subprocess.PIPE,
                stdout=asyncio.subprocess.PIPE,
                stderr=asyncio.subprocess.PIPE,
            )
            stdout, stderr = await process.communicate(diff_text.encode())
            
            if process.returncode == 0:
                return ApplyResult(success=True, diff=diff_text)
            else:
                return ApplyResult(
                    success=False,
                    error=stderr.decode(),
                    diff=diff_text,
                )
        except Exception as e:
            return ApplyResult(success=False, error=str(e))
```

### 6.4 Retry and Validation

Validation ensures evolved skills are correct before replacement:

```python
class SkillEvolver:
    async def _validate_skill(self, skill_dir: Path) -> ValidationResult:
        """Validate an evolved skill."""
        
        skill_file = skill_dir / "SKILL.md"
        
        # 1. Check file exists
        if not skill_file.exists():
            return ValidationResult(
                success=False,
                errors=["SKILL.md not found"],
            )
        
        content = skill_file.read_text()
        
        # 2. Parse YAML frontmatter
        try:
            meta = self._parse_frontmatter(content)
        except Exception as e:
            return ValidationResult(
                success=False,
                errors=[f"Invalid YAML frontmatter: {e}"],
            )
        
        # 3. Validate required fields
        errors = []
        if not meta.get("name"):
            errors.append("Missing 'name' in frontmatter")
        if not meta.get("description"):
            errors.append("Missing 'description' in frontmatter")
        
        # 4. Validate markdown structure
        structure_errors = self._validate_structure(content)
        errors.extend(structure_errors)
        
        # 5. LLM quality check
        quality_result = await self._llm_quality_check(content)
        if not quality_result.passed:
            errors.append(quality_result.feedback)
        
        return ValidationResult(
            success=len(errors) == 0,
            errors=errors,
        )
    
    async def _llm_quality_check(self, content: str) -> QualityResult:
        """LLM-based quality validation."""
        
        prompt = f"""
Validate this skill content for quality:

{content}

Check for:
1. Clear, actionable instructions
2. Complete code examples
3. Logical step-by-step flow
4. Troubleshooting coverage
5. No placeholder text (TODO, FIXME, etc.)

Return JSON with:
- passed: boolean
- feedback: string (if failed)
"""
        
        response = await self._llm_client.complete(prompt)
        return QualityResult.from_json(response)
```

---

## 7. Skill Store

SQLite-backed persistence for skill records, lineage, and metrics.

### 7.1 SQLite Persistence Schema

**Complete Schema:**

```sql
-- Main skill records table
CREATE TABLE skill_records (
    skill_id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    description TEXT,
    skill_dir TEXT NOT NULL,
    
    -- Metrics
    total_selections INTEGER DEFAULT 0,
    total_applied INTEGER DEFAULT 0,
    total_completions INTEGER DEFAULT 0,
    total_fallbacks INTEGER DEFAULT 0,
    
    -- Timestamps
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    last_updated DATETIME DEFAULT CURRENT_TIMESTAMP,
    last_applied_at DATETIME,
    
    -- Status
    status TEXT DEFAULT 'active',  -- active, deprecated, archived
    
    -- Content hash for change detection
    content_hash TEXT
);

-- Lineage tracking (version DAG)
CREATE TABLE skill_lineage_parents (
    skill_id TEXT PRIMARY KEY,
    parent_skill_ids TEXT,  -- JSON array
    origin TEXT NOT NULL,   -- 'fix', 'derived', 'captured'
    generation INTEGER NOT NULL DEFAULT 1,
    source_task_id TEXT,
    
    FOREIGN KEY (skill_id) REFERENCES skill_records(skill_id)
);

CREATE INDEX idx_lineage_parent ON skill_lineage_parents(parent_skill_ids);

-- Per-task execution analyses
CREATE TABLE execution_analyses (
    analysis_id TEXT PRIMARY KEY,
    task_id TEXT NOT NULL,
    timestamp DATETIME DEFAULT CURRENT_TIMESTAMP,
    outcome TEXT,  -- success, failure
    
    -- Full analysis JSON
    analysis_json TEXT
);

-- Per-skill judgments within analyses
CREATE TABLE skill_judgments (
    judgment_id TEXT PRIMARY KEY,
    analysis_id TEXT NOT NULL,
    skill_id TEXT NOT NULL,
    outcome TEXT,  -- success, failure, partial
    error_message TEXT,
    tool_calls TEXT,  -- JSON array
    
    FOREIGN KEY (analysis_id) REFERENCES execution_analyses(analysis_id),
    FOREIGN KEY (skill_id) REFERENCES skill_records(skill_id)
);

CREATE INDEX idx_judgments_skill ON skill_judgments(skill_id);
CREATE INDEX idx_judgments_analysis ON skill_judgments(analysis_id);

-- Tool dependencies
CREATE TABLE skill_tool_deps (
    skill_id TEXT NOT NULL,
    tool_key TEXT NOT NULL,
    usage_count INTEGER DEFAULT 0,
    last_used DATETIME,
    
    PRIMARY KEY (skill_id, tool_key),
    FOREIGN KEY (skill_id) REFERENCES skill_records(skill_id)
);

CREATE INDEX idx_tool_deps_tool ON skill_tool_deps(tool_key);

-- Auxiliary tags
CREATE TABLE skill_tags (
    skill_id TEXT NOT NULL,
    tag TEXT NOT NULL,
    
    PRIMARY KEY (skill_id, tag),
    FOREIGN KEY (skill_id) REFERENCES skill_records(skill_id)
);

CREATE INDEX idx_tags ON skill_tags(tag);

-- Quality metrics history (for trend analysis)
CREATE TABLE skill_metrics_history (
    record_id INTEGER PRIMARY KEY AUTOINCREMENT,
    skill_id TEXT NOT NULL,
    timestamp DATETIME DEFAULT CURRENT_TIMESTAMP,
    
    -- Snapshot metrics
    total_selections INTEGER,
    total_applied INTEGER,
    total_completions INTEGER,
    total_fallbacks INTEGER,
    
    FOREIGN KEY (skill_id) REFERENCES skill_records(skill_id)
);

CREATE INDEX idx_metrics_history_skill ON skill_metrics_history(skill_id);
```

### 7.2 Version DAG Implementation

The version DAG tracks parent-child relationships:

```python
class SkillStore:
    def save_derived(
        self,
        skill_dir: Path,
        skill_id: str,
        lineage: SkillLineage,
    ) -> SkillRecord:
        """Save a DERIVED skill with lineage."""
        
        # 1. Insert skill record
        self._conn.execute("""
            INSERT INTO skill_records (skill_id, skill_dir, name, description, status)
            VALUES (?, ?, ?, ?, 'active')
        """, (skill_id, str(skill_dir), lineage.name, lineage.description))
        
        # 2. Insert lineage
        self._conn.execute("""
            INSERT INTO skill_lineage_parents 
            (skill_id, parent_skill_ids, origin, generation, source_task_id)
            VALUES (?, ?, 'derived', ?, ?)
        """, (
            skill_id,
            json.dumps(lineage.parent_skill_ids),
            lineage.generation,
            lineage.source_task_id,
        ))
        
        self._conn.commit()
        return self.get_record(skill_id)
    
    def get_children(self, skill_id: str) -> List[str]:
        """Get all direct children of a skill."""
        
        cursor = self._conn.execute("""
            SELECT skill_id FROM skill_lineage_parents
            WHERE parent_skill_ids LIKE ?
        """, (f'%{skill_id}%',))
        
        return [row[0] for row in cursor.fetchall()]
    
    def get_full_lineage(self, skill_id: str) -> List[dict]:
        """Get complete lineage tree using recursive CTE."""
        
        cursor = self._conn.execute("""
            WITH RECURSIVE lineage_tree AS (
                -- Base case: the skill itself
                SELECT 
                    skill_id,
                    parent_skill_ids,
                    origin,
                    generation,
                    0 as depth,
                    skill_id as root_id
                FROM skill_lineage_parents
                WHERE skill_id = ?
                
                UNION ALL
                
                -- Recursive case: parents
                SELECT 
                    p.skill_id,
                    p.parent_skill_ids,
                    p.origin,
                    p.generation,
                    t.depth + 1,
                    t.root_id
                FROM skill_lineage_parents p
                JOIN lineage_tree t ON p.skill_id IN (
                    SELECT value FROM json_each(t.parent_skill_ids)
                )
            )
            SELECT * FROM lineage_tree ORDER BY depth DESC, generation ASC
        """, (skill_id,))
        
        return [dict(row) for row in cursor.fetchall()]
```

**Visualizing the DAG:**

```python
def visualize_dag(self, skill_id: str) -> str:
    """Generate ASCII visualization of version DAG."""
    
    lineage = self.get_full_lineage(skill_id)
    records = {l['skill_id']: self.get_record(l['skill_id']) for l in lineage}
    
    lines = []
    for item in lineage:
        indent = "  " * item['depth']
        record = records.get(item['skill_id'])
        name = record.name if record else item['skill_id']
        lines.append(f"{indent}├─ {name} ({item['origin']}, gen {item['generation']})")
    
    return "\n".join(lines)
```

### 7.3 Quality Metrics Tracking

Continuous tracking of skill health:

```python
class SkillStore:
    def record_application(self, skill_id: str, outcome: str, tool_calls: List[dict]):
        """Record a skill application and update metrics."""
        
        # 1. Update main record
        if outcome == "success":
            self._conn.execute("""
                UPDATE skill_records 
                SET total_applied = total_applied + 1,
                    total_completions = total_completions + 1,
                    last_applied_at = CURRENT_TIMESTAMP,
                    last_updated = CURRENT_TIMESTAMP
                WHERE skill_id = ?
            """, (skill_id,))
        else:
            self._conn.execute("""
                UPDATE skill_records 
                SET total_applied = total_applied + 1,
                    total_fallbacks = total_fallbacks + 1,
                    last_applied_at = CURRENT_TIMESTAMP,
                    last_updated = CURRENT_TIMESTAMP
                WHERE skill_id = ?
            """, (skill_id,))
        
        # 2. Record tool usage
        for tool_call in tool_calls:
            self._conn.execute("""
                INSERT INTO skill_tool_deps (skill_id, tool_key, usage_count, last_used)
                VALUES (?, ?, 1, CURRENT_TIMESTAMP)
                ON CONFLICT(skill_id, tool_key) DO UPDATE SET
                    usage_count = usage_count + 1,
                    last_used = CURRENT_TIMESTAMP
            """, (skill_id, tool_call['tool_key']))
        
        self._conn.commit()
    
    def get_metrics(self, skill_id: str) -> SkillMetrics:
        """Get computed metrics for a skill."""
        
        record = self.get_record(skill_id)
        
        return SkillMetrics(
            applied_rate=record.total_applied / max(record.total_selections, 1),
            completion_rate=record.total_completions / max(record.total_applied, 1),
            fallback_rate=record.total_fallbacks / max(record.total_applied, 1),
            effective_rate=(
                (record.total_completions / max(record.total_applied, 1)) *
                (record.total_applied / max(record.total_selections, 1))
            ),
        )
    
    def snapshot_metrics(self):
        """Create historical snapshot of all metrics."""
        
        self._conn.execute("""
            INSERT INTO skill_metrics_history 
            (skill_id, total_selections, total_applied, total_completions, total_fallbacks)
            SELECT skill_id, total_selections, total_applied, total_completions, total_fallbacks
            FROM skill_records
        """)
        
        self._conn.commit()
```

### 7.4 Lineage Queries

Common lineage query patterns:

```python
class SkillStore:
    def get_evolution_chain(self, skill_id: str) -> List[SkillRecord]:
        """Get complete evolution chain from root to current."""
        
        cursor = self._conn.execute("""
            WITH RECURSIVE evolution_chain AS (
                SELECT skill_id, parent_skill_ids, generation
                FROM skill_lineage_parents
                WHERE skill_id = ?
                
                UNION ALL
                
                SELECT p.skill_id, p.parent_skill_ids, p.generation
                FROM skill_lineage_parents p
                JOIN evolution_chain e ON p.skill_id IN (
                    SELECT value FROM json_each(e.parent_skill_ids)
                )
            )
            SELECT skill_id FROM evolution_chain ORDER BY generation ASC
        """, (skill_id,))
        
        return [self.get_record(row[0]) for row in cursor.fetchall()]
    
    def get_latest_version(self, base_skill_id: str) -> Optional[str]:
        """Get the latest version in an evolution chain."""
        
        cursor = self._conn.execute("""
            WITH RECURSIVE all_descendants AS (
                SELECT skill_id, generation
                FROM skill_lineage_parents
                WHERE skill_id = ?
                
                UNION ALL
                
                SELECT p.skill_id, p.generation
                FROM skill_lineage_parents p
                JOIN all_descendants d ON p.parent_skill_ids LIKE ?
            )
            SELECT skill_id FROM all_descendants 
            ORDER BY generation DESC 
            LIMIT 1
        """, (base_skill_id, f'%{base_skill_id}%'))
        
        row = cursor.fetchone()
        return row[0] if row else None
    
    def find_skills_by_generation(
        self,
        base_name: str,
        generation: int,
    ) -> List[SkillRecord]:
        """Find all skills with a specific base name and generation."""
        
        cursor = self._conn.execute("""
            SELECT sr.skill_id FROM skill_records sr
            JOIN skill_lineage_parents slp ON sr.skill_id = slp.skill_id
            WHERE sr.name LIKE ? AND slp.generation = ?
        """, (f"{base_name}%", generation))
        
        return [self.get_record(row[0]) for row in cursor.fetchall()]
```

---

## 8. Quality Monitoring

Continuous monitoring of skill and tool health with automated triggers.

### 8.1 Skill Metrics

Core metrics tracked for every skill:

| Metric | Description | Calculation |
|--------|-------------|-------------|
| `total_selections` | Times skill was considered | Incremented on selection |
| `total_applied` | Times skill was actually used | Incremented on apply |
| `total_completions` | Times skill led to success | Incremented on success |
| `total_fallbacks` | Times skill failed | Incremented on failure |

**Derived Metrics:**

```python
@dataclass
class SkillMetrics:
    applied_rate: float      # total_applied / total_selections
    completion_rate: float   # total_completions / total_applied
    fallback_rate: float     # total_fallbacks / total_applied
    effective_rate: float    # completion_rate * applied_rate
    
    def is_healthy(self) -> bool:
        """Check if skill metrics are within healthy ranges."""
        return (
            self.fallback_rate < 0.4 and
            self.completion_rate > 0.35 and
            self.applied_rate > 0.1
        )
    
    def needs_evolution(self) -> Optional[EvolutionType]:
        """Determine if skill needs evolution and what type."""
        if self.fallback_rate > 0.4:
            return EvolutionType.FIX
        if self.completion_rate < 0.35:
            return EvolutionType.FIX
        if self.applied_rate > 0.4 and self.effective_rate < 0.55:
            return EvolutionType.DERIVED
        return None
```

### 8.2 Tool Call Metrics

Parallel tracking of tool health:

| Metric | Description | Calculation |
|--------|-------------|-------------|
| `success_rate` | Tool success percentage | successes / total_calls |
| `avg_latency_ms` | Average execution time | sum(latency) / count |
| `p95_latency_ms` | 95th percentile latency | percentile(latencies, 95) |
| `error_rate` | Error frequency | errors / total_calls |

```python
@dataclass
class ToolMetrics:
    tool_key: str
    success_rate: float
    avg_latency_ms: float
    p95_latency_ms: float
    error_rate: float
    call_count: int
    
    def is_degraded(
        self,
        success_threshold: float = 0.7,
        min_samples: int = 5,
    ) -> bool:
        """Check if tool is degraded."""
        if self.call_count < min_samples:
            return False
        return self.success_rate < success_threshold
```

### 8.3 Cascade Evolution Triggers

When one skill evolves, dependent skills may need updates:

```python
class CascadeManager:
    def __init__(self, store: SkillStore, evolver: SkillEvolver):
        self._store = store
        self._evolver = evolver
    
    async def process_evolution(self, evolved_skill_id: str):
        """Process cascade effects after a skill evolves."""
        
        # 1. Find children (skills derived from this one)
        children = self._store.get_children(evolved_skill_id)
        
        # 2. Check if children need sync updates
        for child_id in children:
            child_metrics = self._store.get_metrics(child_id)
            if not child_metrics.is_healthy():
                # Child may be out of sync with parent
                await self._trigger_sync_evolution(child_id, evolved_skill_id)
        
        # 3. Find skills using same tools
        tools = self._store.get_skill_tools(evolved_skill_id)
        for tool_key in tools:
            sibling_skills = self._store.get_skills_using_tool(tool_key)
            for sibling_id in sibling_skills:
                if sibling_id != evolved_skill_id:
                    await self._check_sibling_health(sibling_id, tool_key)
    
    async def _trigger_sync_evolution(
        self,
        child_id: str,
        parent_id: str,
    ):
        """Trigger sync evolution for out-of-sync child."""
        
        ctx = EvolutionContext(
            trigger=EvolutionTrigger.CASCADE,
            suggestion=EvolutionSuggestion(
                evolution_type=EvolutionType.FIX,
                target_skill_ids=[child_id],
                reason=f"Parent skill {parent_id} evolved, sync may be needed",
            ),
        )
        await self._evolver.evolve(ctx)
```

### 8.4 Health Scoring

Composite health score for prioritizing evolution:

```python
@dataclass
class HealthScore:
    skill_id: str
    overall_score: float      # 0.0 to 1.0
    component_scores: dict    # breakdown by category
    recommendation: str
    
    @classmethod
    def calculate(cls, record: SkillRecord, tool_metrics: dict) -> HealthScore:
        """Calculate composite health score."""
        
        # Component scores
        components = {}
        
        # 1. Effectiveness (40% weight)
        effectiveness = record.total_completions / max(record.total_applied, 1)
        components['effectiveness'] = effectiveness
        
        # 2. Adoption (25% weight)
        adoption = min(record.total_selections / 100, 1.0)  # Cap at 100 selections
        components['adoption'] = adoption
        
        # 3. Reliability (25% weight)
        reliability = 1.0 - (record.total_fallbacks / max(record.total_applied, 1))
        components['reliability'] = reliability
        
        # 4. Tool Health (10% weight)
        tool_scores = [
            m.success_rate for m in tool_metrics.values()
        ]
        avg_tool_health = sum(tool_scores) / len(tool_scores) if tool_scores else 1.0
        components['tool_health'] = avg_tool_health
        
        # Weighted average
        overall = (
            components['effectiveness'] * 0.40 +
            components['adoption'] * 0.25 +
            components['reliability'] * 0.25 +
            components['tool_health'] * 0.10
        )
        
        # Generate recommendation
        recommendation = cls._generate_recommendation(overall, components)
        
        return cls(
            skill_id=record.skill_id,
            overall_score=overall,
            component_scores=components,
            recommendation=recommendation,
        )
    
    @staticmethod
    def _generate_recommendation(overall: float, components: dict) -> str:
        """Generate human-readable recommendation."""
        
        if overall > 0.8:
            return "Skill is healthy - no action needed"
        
        # Find weakest component
        weakest = min(components.items(), key=lambda x: x[1])
        
        if weakest[0] == 'effectiveness':
            return "Low effectiveness - consider DERIVED evolution to improve"
        elif weakest[0] == 'reliability':
            return "Low reliability - FIX evolution recommended"
        elif weakest[0] == 'adoption':
            return "Low adoption - review skill discovery and naming"
        else:
            return "Tool health issues - monitor dependent tools"
```

**Quality Monitoring Dashboard Architecture:**

```
┌─────────────────────────────────────────────────────────────────┐
│                    Quality Monitoring Dashboard                   │
├─────────────────────────────────────────────────────────────────┤
│                                                                  │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐              │
│  │Skill Metrics│  │Tool Metrics │  │Health Scores│              │
│  │             │  │             │  │             │              │
│  │- Applied %  │  │- Success %  │  │- Overall    │              │
│  │- Complete % │  │- Latency    │  │- Components │              │
│  │- Fallback % │  │- Error rate │  │- Trend      │              │
│  └─────────────┘  └─────────────┘  └─────────────┘              │
│                                                                  │
│  ┌───────────────────────────────────────────────────────────┐  │
│  │                   Evolution Queue                          │  │
│  │                                                            │  │
│  │  Priority │ Skill │ Type  │ Reason │ Score │ Status       │  │
│  │  ────────┼───────┼───────┼────────┼───────┼───────        │  │
│  │  0.95    │ X     │ FIX   │ API    │ 0.32  │ Pending       │  │
│  │  0.87    │ Y     │ DERIVED│ Pattern│ 0.55  │ Running      │  │
│  └───────────────────────────────────────────────────────────┘  │
│                                                                  │
│  ┌───────────────────────────────────────────────────────────┐  │
│  │                   Version DAG View                         │  │
│  │                                                            │  │
│  │  root-skill (gen 0)                                        │  │
│  │    └─ enhanced (gen 1)                                     │  │
│  │       └─ enhanced-v2 (gen 2) ← Current                     │  │
│  └───────────────────────────────────────────────────────────┘  │
│                                                                  │
└─────────────────────────────────────────────────────────────────┘
```

---

## 9. Safety Mechanisms

Multiple safety layers prevent harmful or runaway evolution.

### 9.1 Confirmation Gates

Before any evolution executes, confirmation checks run:

```python
class ConfirmationGate:
    """Gates that must pass before evolution proceeds."""
    
    def __init__(self, store: SkillStore):
        self._store = store
        self._recent_evolutions: Dict[str, List[datetime]] = defaultdict(list)
        self._pending_confirmations: Dict[str, ConfirmationRequest] = {}
    
    async def check_all(self, suggestion: EvolutionSuggestion) -> GateResult:
        """Run all confirmation gates. Return result."""
        
        gates = [
            self._anti_loop_check,
            self._safety_check,
            self._validation_check,
            self._rate_limit_check,
        ]
        
        results = []
        for gate in gates:
            result = await gate(suggestion)
            results.append(result)
            if not result.passed:
                return GateResult(
                    passed=False,
                    failed_gate=gate.__name__,
                    reason=result.reason,
                )
        
        return GateResult(passed=True)
    
    async def _anti_loop_check(self, suggestion: EvolutionSuggestion) -> GateResult:
        """Prevent rapid repeated evolution of same skill."""
        
        for skill_id in suggestion.target_skill_ids:
            recent = self._recent_evolutions[skill_id]
            # Filter to last hour
            recent = [t for t in recent if (datetime.now() - t).seconds < 3600]
            
            if len(recent) >= 3:
                return GateResult(
                    passed=False,
                    reason=f"Skill {skill_id} evolved 3+ times in last hour",
                )
            
            self._recent_evolutions[skill_id] = recent
        
        return GateResult(passed=True)
    
    async def _safety_check(self, suggestion: EvolutionSuggestion) -> GateResult:
        """Check for prompt injection and credential exfiltration attempts."""
        
        # Check reason field
        if self._contains_injection(suggestion.reason):
            return GateResult(
                passed=False,
                reason="Potential prompt injection detected in suggestion reason",
            )
        
        # Check for credential patterns
        if self._contains_credential_patterns(suggestion.reason):
            return GateResult(
                passed=False,
                reason="Potential credential exfiltration attempt detected",
            )
        
        return GateResult(passed=True)
    
    def _contains_injection(self, text: str) -> bool:
        """Check for common prompt injection patterns."""
        
        injection_patterns = [
            r"ignore\s+previous",
            r"bypass.*constraint",
            r"override.*security",
            r"extract.*credential",
            r"system.*prompt",
            r"new\s+instructions",
            r"forget.*prior",
        ]
        
        return any(re.search(p, text.lower()) for p in injection_patterns)
    
    def _contains_credential_patterns(self, text: str) -> bool:
        """Check for credential-like patterns."""
        
        credential_patterns = [
            r"api[_-]?key\s*[=:]\s*['\"]?[a-zA-Z0-9]{20,}",
            r"password\s*[=:]\s*['\"]?\S+",
            r"secret\s*[=:]\s*['\"]?\S+",
            r"token\s*[=:]\s*['\"]?[a-zA-Z0-9]{20,}",
        ]
        
        return any(re.search(p, text.lower()) for p in credential_patterns)
    
    async def _validation_check(self, suggestion: EvolutionSuggestion) -> GateResult:
        """Validate target skills exist and are accessible."""
        
        for skill_id in suggestion.target_skill_ids:
            if not self._store.exists(skill_id):
                return GateResult(
                    passed=False,
                    reason=f"Target skill {skill_id} not found",
                )
        
        return GateResult(passed=True)
    
    async def _rate_limit_check(self, suggestion: EvolutionSuggestion) -> GateResult:
        """Check global evolution rate limits."""
        
        # Count evolutions in last 10 minutes
        ten_min_ago = datetime.now() - timedelta(minutes=10)
        recent_count = sum(
            len([t for t in times if t > ten_min_ago])
            for times in self._recent_evolutions.values()
        )
        
        if recent_count >= 20:
            return GateResult(
                passed=False,
                reason="Global evolution rate limit exceeded (20 per 10 min)",
            )
        
        return GateResult(passed=True)
    
    def record_evolution(self, skill_id: str):
        """Record that an evolution occurred for tracking."""
        self._recent_evolutions[skill_id].append(datetime.now())
```

### 9.2 Anti-Loop Guards

Preventing infinite evolution cycles:

```python
class AntiLoopGuard:
    """Prevents runaway evolution loops."""
    
    def __init__(self):
        self._evolution_history: Dict[str, List[EvolutionRecord]] = defaultdict(list)
        self._tool_degradation_addressed: Dict[str, Set[str]] = defaultdict(set)
    
    def should_allow_evolution(
        self,
        skill_id: str,
        evolution_type: EvolutionType,
        reason: str,
    ) -> bool:
        """Check if evolution should be allowed."""
        
        history = self._evolution_history[skill_id]
        
        # Check 1: Too many evolutions in short time
        one_hour_ago = datetime.now() - timedelta(hours=1)
        recent = [e for e in history if e.timestamp > one_hour_ago]
        
        if len(recent) >= 3:
            log(f"Blocked: {skill_id} evolved 3+ times in last hour")
            return False
        
        # Check 2: Same reason repeated
        if len(history) >= 2:
            last_two_reasons = [e.reason for e in history[-2:]]
            if all(r == reason for r in last_two_reasons):
                log(f"Blocked: {skill_id} same reason repeated: {reason}")
                return False
        
        # Check 3: FIX loop (fix -> fail -> fix -> fail)
        if evolution_type == EvolutionType.FIX:
            fix_history = [e for e in history if e.evolution_type == EvolutionType.FIX]
            if len(fix_history) >= 2:
                last_fix = fix_history[-1]
                if (datetime.now() - last_fix.timestamp).seconds < 300:
                    log(f"Blocked: {skill_id} FIX too soon after previous FIX")
                    return False
        
        return True
    
    def record_tool_degradation_addressed(
        self,
        tool_key: str,
        skill_id: str,
    ):
        """Record that a skill was evolved for tool degradation."""
        self._tool_degradation_addressed[tool_key].add(skill_id)
    
    def is_skill_addressed_for_tool(
        self,
        tool_key: str,
        skill_id: str,
    ) -> bool:
        """Check if skill was already evolved for this tool degradation."""
        return skill_id in self._tool_degradation_addressed[tool_key]
    
    def reset_tool_degradation(self, tool_key: str):
        """Reset tracking when tool recovers."""
        self._tool_degradation_addressed[tool_key].clear()
```

### 9.3 Safety Checks

Multiple safety layers protect against harmful content:

**Prompt Injection Detection:**

```python
class SafetyChecker:
    """Safety checks for evolution content."""
    
    INJECTION_PATTERNS = [
        r"ignore\s+(previous|prior|all)",
        r"bypass\s+(all\s+)?(constraint|rule|policy)",
        r"override\s+(security|safety|content)",
        r"new\s+instructions?\s*:",
        r"system\s*(message|prompt|instruction)",
        r"forget\s+(everything|all|your)",
        r"you\s+are\s+now\s+(instructed|commanded)",
    ]
    
    CREDENTIAL_PATTERNS = [
        r"(api[_-]?key|apikey)\s*[=:]\s*['\"]?[a-zA-Z0-9_-]{16,}",
        r"(password|passwd|pwd)\s*[=:]\s*['\"]?\S+",
        r"(secret|token)\s*[=:]\s*['\"]?[a-zA-Z0-9_-]{16,}",
        r"(aws[_-]?access|aws[_-]?secret)\s*[=:]\s*['\"]?[A-Z0-9]{16,}",
    ]
    
    DANGEROUS_COMMAND_PATTERNS = [
        r"rm\s+-rf\s+/",
        r"mkfs\.",
        r":\(\)\{\s*:\|:\s*&\s*\}\s*;",  # Fork bomb
        r"chmod\s+-R\s+777\s+/",
        r"dd\s+if=/dev/zero",
    ]
    
    async def check_content(self, content: str) -> SafetyResult:
        """Check content for safety issues."""
        
        issues = []
        
        # Check for prompt injection
        for pattern in self.INJECTION_PATTERNS:
            if re.search(pattern, content, re.IGNORECASE):
                issues.append(f"Prompt injection pattern detected: {pattern}")
        
        # Check for credential leaks
        for pattern in self.CREDENTIAL_PATTERNS:
            if re.search(pattern, content, re.IGNORECASE):
                issues.append(f"Potential credential pattern: {pattern}")
        
        # Check for dangerous commands
        for pattern in self.DANGEROUS_COMMAND_PATTERNS:
            if re.search(pattern, content, re.IGNORECASE):
                issues.append(f"Dangerous command pattern: {pattern}")
        
        return SafetyResult(
            passed=len(issues) == 0,
            issues=issues,
        )
```

### 9.4 Validation Before Replacement

Final validation before committing evolution:

```python
class EvolutionValidator:
    """Validates evolved skills before replacement."""
    
    async def validate_before_commit(
        self,
        skill_dir: Path,
        original_content: str,
        new_content: str,
    ) -> ValidationResult:
        """Complete validation before committing evolution."""
        
        errors = []
        
        # 1. Syntax validation
        syntax_result = self._validate_syntax(new_content)
        if not syntax_result.passed:
            errors.extend(syntax_result.errors)
        
        # 2. Structure validation
        structure_result = self._validate_structure(new_content)
        if not structure_result.passed:
            errors.extend(structure_result.errors)
        
        # 3. Safety check
        safety_result = await SafetyChecker().check_content(new_content)
        if not safety_result.passed:
            errors.extend(safety_result.issues)
        
        # 4. Diff analysis (ensure changes are reasonable)
        diff_analysis = self._analyze_diff(original_content, new_content)
        if diff_analysis.too_radical:
            errors.append("Changes too radical - evolution should be incremental")
        
        # 5. LLM quality check
        llm_result = await self._llm_quality_check(new_content)
        if not llm_result.passed:
            errors.append(llm_result.feedback)
        
        # 6. Test execution (if test framework available)
        if self._test_framework_available():
            test_result = await self._run_tests(skill_dir)
            if not test_result.passed:
                errors.extend(test_result.errors)
        
        return ValidationResult(
            passed=len(errors) == 0,
            errors=errors,
        )
    
    def _analyze_diff(self, original: str, new: str) -> DiffAnalysis:
        """Analyze the diff between original and new content."""
        
        diff = difflib.ndiff(original.splitlines(), new.splitlines())
        
        lines = list(diff)
        additions = sum(1 for l in lines if l.startswith('+'))
        deletions = sum(1 for l in lines if l.startswith('-'))
        total_original = len(original.splitlines())
        
        change_ratio = (additions + deletions) / max(total_original, 1)
        
        return DiffAnalysis(
            additions=additions,
            deletions=deletions,
            change_ratio=change_ratio,
            too_radical=change_ratio > 0.5,  # >50% changed
        )
```

---

## 10. Implementation Reference

### 10.1 Complete Type Definitions

```python
from enum import Enum
from dataclasses import dataclass, field
from typing import List, Dict, Optional, Set
from pathlib import Path
from datetime import datetime

class EvolutionType(Enum):
    FIX = "fix"
    DERIVED = "derived"
    CAPTURED = "captured"

class EvolutionOrigin(Enum):
    MANUAL = "manual"
    AUTO_FIX = "auto_fix"
    AUTO_DERIVED = "auto_derived"
    AUTO_CAPTURED = "auto_captured"

class EvolutionTrigger(Enum):
    POST_EXECUTION = "post_execution"
    TOOL_DEGRADATION = "tool_degradation"
    METRIC_MONITOR = "metric_monitor"
    CASCADE = "cascade"
    MANUAL = "manual"

@dataclass
class EvolutionSuggestion:
    evolution_type: EvolutionType
    target_skill_ids: List[str]
    reason: str
    priority: float  # 0.0 to 1.0
    skill_name: Optional[str] = None
    source_task_id: Optional[str] = None

@dataclass
class EvolutionContext:
    trigger: EvolutionTrigger
    suggestion: EvolutionSuggestion
    skill_dirs: List[Path]
    available_tools: List[dict]
    execution_recording: Optional[dict] = None
    source_task_id: Optional[str] = None

@dataclass
class SkillLineage:
    origin: EvolutionOrigin
    generation: int
    parent_skill_ids: List[str]
    source_task_id: Optional[str] = None

@dataclass
class SkillRecord:
    skill_id: str
    name: str
    description: str
    skill_dir: str
    total_selections: int = 0
    total_applied: int = 0
    total_completions: int = 0
    total_fallbacks: int = 0
    created_at: datetime = field(default_factory=datetime.now)
    last_updated: datetime = field(default_factory=datetime.now)
    status: str = "active"

@dataclass
class SkillMetrics:
    applied_rate: float
    completion_rate: float
    fallback_rate: float
    effective_rate: float

@dataclass
class EvolutionResult:
    success: bool
    skill_id: Optional[str]
    skill_dir: Path
    evolution_type: EvolutionType
    change_summary: str
    error: Optional[str] = None
```

### 10.2 Complete Evolution Flow Example

```python
import asyncio
from pathlib import Path
from openspace import OpenSpace, OpenSpaceConfig
from openspace.skill_engine.evolver import (
    EvolutionContext, 
    EvolutionSuggestion, 
    EvolutionType,
    EvolutionTrigger,
)

async def run_evolution_example():
    """Complete example of evolution flow."""
    
    # 1. Initialize OpenSpace
    config = OpenSpaceConfig(
        llm_model="openrouter/anthropic/claude-sonnet-4.5",
        workspace_dir="/workspace",
        enable_recording=True,
    )
    
    async with OpenSpace(config=config) as cs:
        # 2. Execute a task (triggers skill application)
        result = await cs.execute(
            "Monitor my Docker containers and restart the highest memory one"
        )
        
        # 3. Post-execution analysis (automatic)
        # ExecutionAnalyzer reviews the recording
        suggestions = await cs._execution_analyzer.analyze(
            result.recording
        )
        
        # 4. Process suggestions through confirmation gates
        for suggestion in suggestions:
            gate_result = await cs._confirmation_gate.check(suggestion)
            
            if gate_result.passed:
                # 5. Check anti-loop guards
                if cs._anti_loop_guard.should_allow_evolution(
                    skill_id=suggestion.target_skill_ids[0],
                    evolution_type=suggestion.evolution_type,
                    reason=suggestion.reason,
                ):
                    # 6. Create evolution context
                    ctx = EvolutionContext(
                        trigger=EvolutionTrigger.POST_EXECUTION,
                        suggestion=suggestion,
                        skill_dirs=[
                            cs._skill_registry.get_skill_dir(sid)
                            for sid in suggestion.target_skill_ids
                        ],
                        available_tools=cs.available_tools,
                        execution_recording=result.recording,
                    )
                    
                    # 7. Execute evolution
                    evolved_record = await cs._skill_evolver.evolve(ctx)
                    
                    if evolved_record:
                        print(f"Evolved: {evolved_record.name}")
                        print(f"Type: {suggestion.evolution_type}")
                        print(f"Reason: {suggestion.reason}")
            
            # 8. Record evolution for anti-loop tracking
            for skill_id in suggestion.target_skill_ids:
                cs._confirmation_gate.record_evolution(skill_id)

asyncio.run(run_evolution_example())
```

### 10.3 Testing Evolution

```python
async def test_evolution():
    """Test evolution with mock data."""
    
    from openspace.skill_engine.store import SkillStore
    from openspace.skill_engine.evolver import SkillEvolver
    from openspace.llm import MockLLMClient
    
    # Create test infrastructure
    store = SkillStore(":memory:")  # In-memory SQLite
    llm_client = MockLLMClient()
    evolver = SkillEvolver(store, llm_client)
    
    # Create test skill
    test_skill_dir = Path("/tmp/test-skills/test-skill")
    test_skill_dir.mkdir(parents=True, exist_ok=True)
    
    (test_skill_dir / "SKILL.md").write_text("""
---
name: test-skill
description: Test skill for evolution
---

# Test Skill

## Workflow

Step 1: Do something
""")
    
    write_skill_id(test_skill_dir, "test-skill__imp_test123")
    
    # Trigger FIX evolution
    ctx = EvolutionContext(
        trigger=EvolutionTrigger.MANUAL,
        suggestion=EvolutionSuggestion(
            evolution_type=EvolutionType.FIX,
            target_skill_ids=["test-skill__imp_test123"],
            reason="Test fix direction",
            priority=1.0,
        ),
        skill_dirs=[test_skill_dir],
        available_tools=[],
    )
    
    result = await evolver.evolve(ctx)
    print(f"Evolution result: {result}")

asyncio.run(test_evolution())
```

---

## Appendix A: Quick Reference Tables

### Evolution Triggers Summary

| Trigger | When | Priority Range | Typical Type |
|---------|------|----------------|--------------|
| Post-Execution Failure | Skill fails | 0.9-0.95 | FIX |
| Post-Execution Novel Pattern | Successful new pattern | 0.8-0.9 | DERIVED |
| Tool Degradation | Tool success < 70% | 0.85-0.9 | FIX |
| Metric Monitor (High Fallback) | Fallback > 40% | 0.85-0.9 | FIX |
| Metric Monitor (Low Completion) | Completion < 35% | 0.8-0.85 | FIX |
| Metric Monitor (Low Effective) | Applied > 40%, Effective < 55% | 0.7-0.8 | DERIVED |
| Novel Workflow Capture | New pattern detected | 0.75-0.85 | CAPTURED |

### Metric Thresholds

| Metric | Healthy | Warning | Critical |
|--------|---------|---------|----------|
| Fallback Rate | < 0.2 | 0.2-0.4 | > 0.4 |
| Completion Rate | > 0.7 | 0.35-0.7 | < 0.35 |
| Applied Rate | > 0.3 | 0.1-0.3 | < 0.1 |
| Effective Rate | > 0.55 | 0.3-0.55 | < 0.3 |

### Safety Check Summary

| Check | What It Blocks | Action |
|-------|----------------|--------|
| Anti-Loop | 3+ evolutions/hour | Skip evolution |
| Prompt Injection | "ignore previous", etc. | Block + alert |
| Credential Exfil | API keys, passwords | Block + alert |
| Dangerous Commands | rm -rf, fork bombs | Block + alert |
| Rate Limit | 20 evolutions/10min | Queue for later |
| Validation | Invalid YAML, structure | Retry or fail |

---

## Appendix B: File Structure

```
openspace/skill_engine/
├── __init__.py
├── types.py              # Type definitions (EvolutionType, SkillRecord, etc.)
├── store.py              # SQLite persistence (SkillStore)
├── registry.py           # Skill discovery and ranking (SkillRegistry)
├── analyzer.py           # Post-execution analysis (ExecutionAnalyzer)
├── evolver.py            # Evolution execution (SkillEvolver)
├── patch.py              # Diff application utilities
├── monitor.py            # Metric monitoring (MetricMonitor, ToolQualityManager)
├── safety.py             # Safety checks (ConfirmationGate, SafetyChecker)
└── validation.py         # Evolution validation (EvolutionValidator)
```

---

## Appendix C: Configuration

```python
@dataclass
class EvolutionConfig:
    # Agent loop limits
    max_evolution_attempts: int = 3
    max_evolution_iterations: int = 15
    
    # Concurrency
    evolution_concurrency: int = 3
    
    # Anti-loop thresholds
    max_evolutions_per_hour: int = 3
    global_rate_limit: int = 20  # per 10 minutes
    
    # Safety thresholds
    prompt_injection_threshold: float = 0.8
    credential_check_enabled: bool = True
    dangerous_command_check_enabled: bool = True
    
    # Validation
    require_llm_validation: bool = True
    require_test_validation: bool = False
    max_change_ratio: float = 0.5
    
    # Metrics thresholds
    fallback_rate_threshold: float = 0.4
    completion_rate_threshold: float = 0.35
    tool_success_threshold: float = 0.7
```
