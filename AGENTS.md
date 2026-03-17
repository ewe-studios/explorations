# Agents Framework

This repository uses custom agents to drive systematic project explorations and transformations.

## Available Agents

### 1. Exploration Agent

**Location:** `.agents/exploration-agent.md`

**Purpose:** Generate comprehensive, detailed explorations of codebases or git repositories.

**Output Format:**
- Creates `exploration.md` in the target project directory
- Frontmatter includes:
  - `location:` - filesystem path
  - `repository:` - git remote URL (if applicable)
  - `explored_at:` - timestamp
- Body contains:
  - Project overview and purpose
  - Architecture breakdown with Mermaid diagrams
  - File/folder structure explanation
  - Key components and their relationships
  - Entry points and execution flow
  - Dependencies and external integrations
  - Configuration and environment details
  - Testing strategy (if present)

**Usage:**
```
/agents explore <directory_or_repo>
```

### 2. Rust Revision Agent

**Location:** `.agents/rust-revision-agent.md`

**Purpose:** Translate explored projects into Rust, providing idiomatic implementations.

**Output Format:**
- Creates `rust-revision.md` in the target project directory
- Frontmatter includes:
  - `source:` - original project path
  - `repository:` - git remote URL (if applicable)
  - `revised_at:` - timestamp
- Body contains:
  - Crate breakdown and package structure
  - Key Rust-specific design decisions
  - Dependency recommendations (crates.io)
  - Type system considerations
  - Error handling strategy
  - Concurrency/async considerations
  - Edge cases and safety guarantees
  - Performance considerations
  - Code examples for critical components

**Usage:**
```
/agents rust-revision <directory_or_repo>
```

## Project Structure Convention

Explorations are written to this directory using the required structure below (not the target project directory):

```
[project-name]/
  exploration.md          # Exploration agent output
  rust-revision.md        # Rust revision agent output (if applicable)
  examples/
    example-1.md
    example-2.md
    ...
```

## Workflow

1. **Load this AGENTS.md** at the start of every interaction
2. **Select appropriate agent** based on task:
   - Understanding a codebase → Exploration Agent
   - Rust translation → Rust Revision Agent (after exploration)
3. **Execute agent** with target directory/repository
4. **Review and iterate** on outputs as needed

## Agent Creation

New agents are created in `.agents/` with:
- Clear purpose and scope
- Defined input/output format
- Step-by-step execution instructions
- Quality criteria for outputs
