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

Each project's exploration lives in a subdirectory named after the project itself within the parent exploration directory. The source directory name determines the nested path:

```
[src.datastar]/                    ← parent exploration directory
  datastar/                        ← project subdirectory (matches source name)
    exploration.md
    rust-revision.md
    spec.md
    markdown/
      README.md
      00-overview.md
      ...
    html/
      index.html
      *.html
  other_project/                   ← another project in the same parent
    its_own_exploration.md
    ...

[src.orbitinghail]/
  orbitinghail/
    exploration.md
    spec.md
    markdown/
    html/

[src.ui]/
  ui/
    exploration.md
    spec.md
    markdown/
    html/
```

When you are provided a directory of directories (i.e rather than a git directory but a directory that contains other projects and their own specific git repositories), dont just create the exploration in the root, rather create a directory for all the exploration and then create the individual directory explorations per directory in there for organisation.

The parent directory contains a central `build.py` and generates a central `html/index.html` that links to all project subdirectories.

## Workflow

1. **Load this AGENTS.md** at the start of every interaction
2. **Select appropriate agent** based on task:
   - Understanding a codebase → Exploration Agent
   - Rust translation → Rust Revision Agent (after exploration)
3. **Execute agent** with target directory/repository
4. **Review and iterate** on outputs as needed
5. **Commit and push** changes to this repository
   - Use conventional commits: `ADD:`, `FIX:`, `UPDATE:`, `REFACTOR:`
   - Do NOT include Claude attribution in commit messages
   - Do NOT use `--no-verify` or skip hooks
   - Push to remote after committing

## Agent Creation

New agents are created in `.agents/` with:
- Clear purpose and scope
- Defined input/output format
- Step-by-step execution instructions
- Quality criteria for outputs
