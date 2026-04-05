---
location: /home/darkvoid/Boxxed/@formulas/src.rust/src.llamacpp/src.ClaudOpen/claude-code-system-prompts
repository: git@github.com:Leonxlnx/claude-code-system-prompts.git
explored_at: 2026-04-02T22:00:00Z
language: Markdown (documentation/research)
---

# Zero to Prompt Architecture Engineer: Claude Code System Prompts

## What This Project Is

This repository is a reverse-engineered documentation of the prompt architecture that powers Claude Code -- Anthropic's agentic CLI coding assistant. It contains 30 reconstructed prompt documents covering every observable subsystem: the master system prompt, sub-agent prompts, security classifiers, memory hierarchies, context compaction strategies, skill systems, and tool descriptions.

This is not source code. It is a behavioral research project that reconstructs the prompt-level architecture through observation, output analysis, and community knowledge. The actual implementation may differ, but the patterns documented here are representative of production-grade agentic prompt systems.

## Why This Matters

Understanding how a production agentic coding assistant assembles and orchestrates its prompts is foundational knowledge for anyone building AI-powered developer tools. The patterns here -- dynamic prompt assembly, multi-agent coordination, security classification, context window management -- are the same patterns you will need to implement in any serious agentic system.

---

## Part 1: Fundamentals of Agentic Prompt Architecture

### What is a System Prompt?

A system prompt is the instruction set injected at the beginning of every LLM conversation. Unlike user messages, system prompts are invisible to the end user and define the model's persona, capabilities, constraints, and behavioral rules.

In simple chatbots, the system prompt is a static string. In production agentic systems like Claude Code, the system prompt is **dynamically assembled at runtime** from dozens of modular components.

### The Prompt Assembly Pipeline

Claude Code's system prompt is not one document. It is a **pipeline** that concatenates sections based on:

- Feature flags (which experimental features are enabled)
- Environment context (OS, shell, working directory, git state)
- Permission mode (what the user has authorized)
- Available tools (which tools are registered)
- Active MCP servers (external tool providers)
- Memory files (CLAUDE.md, project rules, user preferences)
- Session state (is this a sub-agent? a coordinator? simple mode?)

The pipeline has two critical zones:

```
Cacheable Prefix (stable across turns)
  |-- Identity and safety instructions
  |-- Permission and hook configuration
  |-- Code style and error handling rules
  |-- Tool preferences and usage patterns
  |-- Tone, style, and output rules
  |
  Cache Boundary
  |
Dynamic Suffix (changes per turn)
  |-- Available agents and skills
  |-- Memory file contents
  |-- Environment context (OS, directory, git state)
  |-- Language and output preferences
  |-- Active MCP server instructions
  |-- Context window management directives
```

The **cache boundary** is a critical optimization. Anthropic's API supports prompt caching -- if the prefix of the system prompt is identical across turns, the cached tokens are reused at reduced cost. By placing stable instructions before the boundary and dynamic context after it, Claude Code maximizes cache hits.

### Why Dynamic Assembly?

A static system prompt would need to cover every possible scenario. Dynamic assembly means:

1. **Smaller token footprint** -- only include what's relevant
2. **Feature gating** -- experimental features can be toggled without rewriting prompts
3. **Context sensitivity** -- the prompt adapts to the user's environment
4. **Security layering** -- different permission modes inject different safety constraints

---

## Part 2: The 30 Prompt Patterns

### Core Identity Layer (Prompts 01-04)

#### 01 - Main System Prompt
The master prompt assembled from modular sections. Key sections include:
- **Intro**: Identity declaration ("You are an interactive agent...")
- **System**: Tool execution rules, tag semantics, hook feedback
- **Doing Tasks**: Task execution philosophy -- read before modify, don't gold-plate, no speculative abstractions
- **Actions**: Reversibility and blast radius assessment -- categorizes actions by risk level
- **Using Tools**: Tool delegation rules -- use dedicated tools (Read, Edit, Write, Glob, Grep) instead of Bash for file operations
- **Proactive Safety**: When to check with the user vs. proceed autonomously

The Anthropic-internal build includes additional bullets about code comments, verification, and outcome reporting that represent a higher-fidelity engineering standard.

#### 02 - Simple Mode
A stripped-down prompt variant for lightweight operations. Removes advanced agent orchestration, reduces tool descriptions, and focuses on direct task completion. Used when the full system is overkill.

#### 03 - Default Agent Prompt
The base prompt inherited by all sub-agents. Establishes shared context about tool availability, communication protocols, and output formatting. Sub-agents overlay their specialized instructions on top of this base.

#### 04 - Cyber Risk Instruction
The security boundary definition. Separates authorized security activities (pentesting, CTFs, educational contexts) from prohibited ones (DoS, supply chain compromise, detection evasion for malicious purposes). This is the prompt-level complement to the model's built-in safety training.

### Orchestration Layer (Prompts 05-06)

#### 05 - Coordinator System Prompt
The multi-agent orchestration prompt. Defines how the coordinator agent manages worker agents through phased workflows. The coordinator:
- Decomposes tasks into parallelizable subtasks
- Assigns workers with specific tool subsets
- Monitors progress and handles failures
- Synthesizes results from multiple workers

#### 06 - Teammate Prompt Addendum
Communication protocols for multi-agent collaboration. Defines how agents share context, avoid duplicate work, and coordinate on shared resources (files, git state).

### Specialized Agent Layer (Prompts 07-10)

#### 07 - Verification Agent
An adversarial testing agent. After another agent implements a feature, the verification agent:
- Runs tests and inspects outputs
- Attempts edge cases the implementer might have missed
- Reports failures with specific evidence
- Does NOT fix issues -- only verifies and reports

#### 08 - Explore Agent
A read-only codebase exploration agent. Explicitly constrained to never modify files. Used for understanding codebases, answering questions about architecture, and mapping dependencies.

#### 09 - Agent Creation Architect
A meta-agent that generates new agent configurations from natural language requirements. It interviews the user about the agent's purpose, then produces a complete agent specification.

#### 10 - Status Line Setup Agent
A narrow-scope agent that configures terminal status lines across different shell environments (bash, zsh, fish, PowerShell).

### Security Layer (Prompts 11-12)

#### 11 - Permission Explainer
Before the user approves a tool call, this prompt generates a risk assessment explaining what the tool will do, what could go wrong, and what alternatives exist.

#### 12 - Auto Mode Classifier (YOLO Mode)
The multi-stage security classifier for autonomous tool execution. This is one of the most architecturally interesting prompts. It uses:

1. **Fast path**: A predefined rule set that immediately approves safe operations (reading files, running tests) and immediately blocks dangerous ones (rm -rf, force push)
2. **Extended reasoning**: For ambiguous cases, a slower analysis that considers the full context
3. **User overrides**: Configurable rules that extend or restrict the defaults

The classifier must balance autonomy (users want speed) with safety (users don't want catastrophic accidents).

### Tool Description Layer (Prompt 13)

Defines how each tool describes itself to the model. Tools include Bash, Read, Edit, Write, Glob, Grep, Agent, WebFetch, WebSearch, NotebookEdit, and MCP tools. Each description includes:
- When to use the tool vs. alternatives
- Parameter schemas
- Usage constraints and best practices
- Examples

### Utility Patterns (Prompts 14-17, 20, 29-30)

- **14 - Tool Use Summary**: Generates concise labels for completed tool batches (shown in the UI)
- **15 - Session Search**: Semantic search across past conversation sessions
- **16 - Memory Selection**: Selects which memory files to inject based on the current query
- **17 - Auto Mode Critique**: Reviews user-written classifier rules for correctness
- **20 - Session Title**: Lightweight title generation for session management
- **29 - Agent Summary**: Background progress updates for running sub-agents
- **30 - Prompt Suggestion**: Predicts likely follow-up commands

### Context Window Management (Prompts 21-22)

#### 21 - Compact Service
When the conversation approaches context limits, the compact service summarizes earlier messages. This is critical -- without compaction, long sessions would fail. The strategy must preserve:
- Key decisions and their rationale
- File paths and code references
- Unresolved questions and pending tasks
- Tool results that inform future actions

#### 22 - Away Summary
When a user returns to a session, this generates a brief recap of what happened while they were away.

### Skill Patterns (Prompts 19, 25-28)

Skills are reusable prompt templates that encapsulate specialized workflows:
- **19 - Simplify**: Multi-agent parallel code review
- **25 - Skillify**: Interview-based skill creation
- **26 - Stuck**: Session diagnostic and recovery
- **27 - Remember**: Memory organization and promotion
- **28 - Update Config**: Configuration management

---

## Part 3: Architectural Patterns Worth Studying

### Pattern 1: Hierarchical Memory with Override Semantics

Memory loads in priority order (lowest to highest):
1. Enterprise/managed configuration
2. User global preferences
3. Project-level instructions (shared, committed)
4. Project rules directory
5. Local overrides (private, not committed)

Later layers override earlier ones. This mirrors CSS cascading and is a proven pattern for multi-tenant configuration.

### Pattern 2: Cache-Aware Prompt Segmentation

Splitting prompts into cacheable prefix + dynamic suffix is a cost optimization technique specific to LLM APIs that support prompt caching. The key insight is that the identity, safety, and tool instruction sections rarely change between turns, so they can be cached. The memory, environment, and session-specific sections change frequently and go after the cache boundary.

### Pattern 3: Multi-Stage Security Classification

The auto-mode classifier is a defense-in-depth pattern:
- **Static rules** (fast, deterministic) handle the clear cases
- **LLM reasoning** (slow, probabilistic) handles the ambiguous cases
- **User overrides** provide escape hatches

This three-tier approach is directly applicable to any agentic system that needs to autonomously execute actions.

### Pattern 4: Adversarial Verification

The verification agent pattern (Prompt 07) embodies a fundamental software engineering principle: the person who builds something should not be the only one who tests it. By delegating verification to a separate agent with different instructions, the system gets genuinely independent testing.

---

## Part 4: What This Looks Like in Rust

### Prompt Assembly in Rust

```rust
use std::collections::BTreeMap;

/// A single section of the system prompt.
struct PromptSection {
    key: &'static str,
    content: String,
    cacheable: bool,
    priority: u32,
}

/// Assembles the system prompt from modular sections.
struct PromptBuilder {
    sections: Vec<PromptSection>,
    feature_flags: BTreeMap<String, bool>,
    env: EnvironmentContext,
}

impl PromptBuilder {
    fn build(&self) -> AssembledPrompt {
        let mut cacheable = Vec::new();
        let mut dynamic = Vec::new();

        for section in &self.sections {
            if self.should_include(section) {
                if section.cacheable {
                    cacheable.push(section.content.as_str());
                } else {
                    dynamic.push(section.content.as_str());
                }
            }
        }

        AssembledPrompt {
            cacheable_prefix: cacheable.join("\n\n"),
            dynamic_suffix: dynamic.join("\n\n"),
        }
    }

    fn should_include(&self, section: &PromptSection) -> bool {
        // Feature flag gating, environment checks, etc.
        true
    }
}

struct AssembledPrompt {
    cacheable_prefix: String,
    dynamic_suffix: String,
}
```

### Security Classifier in Rust

```rust
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
enum ClassificationResult {
    Allow,
    Deny { reason: String },
    NeedsReview { context: String },
}

/// Multi-stage security classifier.
struct SecurityClassifier {
    static_rules: Vec<StaticRule>,
    user_overrides: Vec<UserOverride>,
}

impl SecurityClassifier {
    fn classify(&self, tool_call: &ToolCall) -> ClassificationResult {
        // Stage 1: Check static rules (fast path)
        for rule in &self.static_rules {
            if rule.matches(tool_call) {
                return rule.result.clone();
            }
        }

        // Stage 2: Check user overrides
        for override_rule in &self.user_overrides {
            if override_rule.matches(tool_call) {
                return override_rule.result.clone();
            }
        }

        // Stage 3: Defer to LLM reasoning (slow path)
        ClassificationResult::NeedsReview {
            context: format!("Tool {} requires manual review", tool_call.name),
        }
    }
}
```

### Production-Grade Considerations

A production Rust implementation would need:

1. **Async prompt assembly** -- memory files and environment context may require filesystem or network I/O
2. **Token counting** -- each section should track its token cost, and the builder should respect context window limits
3. **Cache key computation** -- hash the cacheable prefix to detect when the cache is invalidated
4. **Hot-reloading** -- memory files (CLAUDE.md) may change between turns; the builder should detect changes
5. **Template interpolation** -- some sections contain placeholders (tool names, model capabilities) that need runtime substitution
6. **Audit logging** -- for security-sensitive deployments, log which prompt sections were included and why

---

## Part 5: Building Your Own Prompt Architecture

### Step 1: Define Your Sections

Start with the minimum viable set:
- Identity (who is the agent?)
- Safety (what must it never do?)
- Tasks (what is its job?)
- Tools (what can it use?)
- Context (what does it know about the environment?)

### Step 2: Implement Dynamic Assembly

Build a section registry where each section can:
- Declare its dependencies (e.g., "only include if tools are available")
- Declare its priority (for ordering)
- Declare whether it's cacheable

### Step 3: Add Memory Hierarchy

Implement a cascading configuration system:
- Global defaults
- Project-level overrides
- Session-level overrides
- Turn-level context

### Step 4: Add Security Classification

For any agentic system that executes actions:
- Define a static allowlist/denylist
- Implement context-aware reasoning for edge cases
- Provide user override mechanisms
- Log all decisions for audit

### Step 5: Add Context Compaction

For long-running sessions:
- Track token usage per turn
- Trigger compaction when approaching limits
- Preserve critical context (decisions, file paths, pending tasks)
- Test that compacted sessions produce equivalent behavior

---

## Key Insights

- The system prompt is not a single document but a **dynamically assembled pipeline** of 30+ modular sections
- **Cache-aware segmentation** splits stable instructions from dynamic context to optimize API costs
- **Multi-stage security classification** balances autonomy with safety through static rules, user overrides, and LLM reasoning
- **Hierarchical memory** with override semantics allows multi-tenant configuration (enterprise > user > project > local)
- **Adversarial verification** uses a separate agent to independently test another agent's work
- **Context compaction** is essential for long sessions and must preserve decision context, not just content
- The Anthropic-internal build includes higher-fidelity engineering standards (verification, outcome reporting) that represent best practices for agentic systems
