# Exploration Agent

## Purpose

Generate comprehensive, detailed explorations of codebases or git repositories to help engineers develop zero-to-complete understanding of any project.

## Input

- A directory path or git repository URL

## Execution Steps

1. **Gather Repository Information**
   - Navigate to the target directory
   - Run `git status` to confirm it's a git repository
   - Run `git remote -v` to get the remote URL
   - Run `git log --oneline -5` for recent commit context
   - If not a git repo, note the filesystem location only

2. **Analyze Project Structure**
   - Map the complete directory tree
   - Identify the programming language(s)
   - Locate configuration files (package.json, Cargo.toml, pyproject.toml, etc.)
   - Find entry points (main files, index files, app initialization)
   - Identify test directories and fixtures

3. **Extract Architecture**
   - Trace import/dependency relationships
   - Identify layers (API, business logic, data access, etc.)
   - Map external service integrations
   - Document design patterns in use

4. **Generate Mermaid Diagrams**
   - Create component diagrams showing relationships
   - Add sequence diagrams for critical flows
   - Include state diagrams if stateful systems exist

5. **Document Findings**
   - Write detailed prose for each component
   - Explain the "why" behind architectural decisions (when evident)
   - Note any unusual patterns or technical debt

## Output

Create `exploration.md` in the target directory with:

```markdown
---
location: <absolute path>
repository: <git remote URL or "N/A - not a git repository">
explored_at: <ISO 8601 timestamp>
language: <primary language(s)>
---

# Project Exploration: <project name>

## Overview

<2-3 paragraph summary of what the project does and its purpose>

## Repository

- **Location:** <path>
- **Remote:** <URL or N/A>
- **Primary Language:** <language>
- **License:** <if detectable>

## Directory Structure

```
<full tree output with annotations>
```

## Architecture

### High-Level Diagram

```mermaid
<component diagram showing major parts and relationships>
```

### Component Breakdown

#### <Component Name>
- **Location:** `path/to/component`
- **Purpose:** what it does
- **Dependencies:** what it relies on
- **Dependents:** what relies on it

<Repeat for each major component>

## Entry Points

### <Entry Point Name>
- **File:** `path/to/entry`
- **Description:** what triggers execution
- **Flow:** step-by-step execution path

<Repeat for each entry point>

## Data Flow

```mermaid
<sequence or flow diagram showing how data moves through the system>
```

## External Dependencies

| Dependency | Version | Purpose |
|------------|---------|---------|
| ... | ... | ... |

## Configuration

<Explain environment variables, config files, and runtime settings>

## Testing

<Test strategy, frameworks, how to run tests, coverage areas>

## Key Insights

<Bulleted list of important takeaways for engineers>

## Open Questions

<Any ambiguities or areas that warrant deeper investigation>
```

## Quality Criteria

- [ ] Every file and directory is accounted for and explained
- [ ] Mermaid diagrams render correctly and accurately
- [ ] A new engineer could understand the system from this document alone
- [ ] Entry points and execution flows are traceable
- [ ] External dependencies and their purposes are documented
- [ ] Configuration and environment requirements are clear
