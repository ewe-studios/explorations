---
title: Pierre Monorepo
prev: 03-just-bash.md
next: 05-icons-vscode.md
---

# Pierre Monorepo

UI components for code visualization and file tree navigation.

## Architecture

**Location:** `src.PierreComputer/pierre/`

Bun monorepo with packages:

```mermaid
flowchart TB
    subgraph Monorepo["pierre Monorepo"]
        DIFFS[@pierre/diffs]
        TREES[@pierre/trees]
        PATHSTORE[@pierre/path-store]
        ELEMENTS[@pierre/storage-elements]
        ELEMENTS2[@pierre/storage-elements-next]
        TRUNC[@pierre/truncate]
    end

    subgraph Apps["Applications"]
        DEMO[demo]
        DOCS[docs]
    end

    DIFFS --> DEMO
    TREES --> DEMO
    PATHSTORE --> TREES
    ELEMENTS --> DOCS
    ELEMENTS2 --> DOCS
```

## Package Overview

| Package | Purpose | Location |
|---------|---------|----------|
| `@pierre/diffs` | Diff rendering | `packages/diffs/` |
| `@pierre/trees` | File tree component | `packages/trees/` |
| `@pierre/path-store` | File tree state | `packages/path-store/` |
| `@pierre/storage-elements` | Storage UI | `packages/storage-elements/` |
| `@pierre/storage-elements-next` | Next.js storage | `packages/storage-elements-next/` |
| `@pierre/truncate` | Text truncation | `packages/truncate/` |

## @pierre/diffs

**Location:** `src.PierreComputer/pierre/packages/diffs/`

Diff rendering with shadow DOM isolation:

```typescript
// src/components/DiffView.tsx
import { useShadowDOM } from '@pierre/diffs';

export function DiffView({ diff }) {
  const shadowRef = useShadowDOM({
    styles: [diffStyles, syntaxHighlighting],
  });

  return (
    <div ref={shadowRef}>
      <DiffRenderer diff={diff} />
    </div>
  );
}
```

**Aha:** Shadow DOM prevents CSS leakage between diffs and host application.

## @pierre/trees

**Location:** `src.PierreComputer/pierre/packages/trees/`

File tree component with virtualization:

```typescript
// src/Tree.tsx
interface TreeProps {
  data: FileTreeNode[];
  onSelect: (path: string) => void;
  virtualized?: boolean;
}

export function Tree({ data, onSelect, virtualized = true }) {
  // Virtual scrolling for large trees
  // Collapsible folders
  // Search/filter support
}
```

## @pierre/path-store

**Location:** `src.PierreComputer/pierre/packages/path-store/`

State management for file trees:

```typescript
// src/store.ts
export interface PathStore {
  // Current selection
  selectedPath: string | null;

  // Expansion state
  expandedPaths: Set<string>;

  // Operations
  select(path: string): void;
  expand(path: string): void;
  collapse(path: string): void;
  toggle(path: string): void;
}

export function createPathStore(): PathStore {
  // Zustand-based implementation
}
```

## @pierre/storage-elements

**Location:** `src.PierreComputer/pierre/packages/storage-elements/`

Storage UI components:

| Component | Purpose |
|-----------|---------|
| FileTree | File browser |
| DiffViewer | Side-by-side diff |
| BranchSelector | Branch picker |
| CommitList | Commit history |

## Next Steps

Continue to [Icons & VS Code →](05-icons-vscode.html) for icon systems.
