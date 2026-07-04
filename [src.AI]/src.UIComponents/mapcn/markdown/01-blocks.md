# mapcn Blocks: Registry-Based Map Templates

7 pre-built map blocks installable via shadcn-style registry. Each block is a complete page with data, components, and layout, defined in `registry.json`.

Source: `src/lib/blocks.ts`, `src/lib/get-block-file-source.ts`, `registry.json`.

## Registry Schema

Source: `src/lib/blocks.ts`

```typescript
interface RegistrySchema {
  items: RegistryBlockItem[];
}

interface RegistryBlockItem {
  name: string
  type: string                    // "registry:block"
  title?: string
  description?: string
  files?: Array<{ path: string; target?: string }>
  registryDependencies?: string[]
  categories?: string[]
  meta?: { iframeHeight?: string }
}
```

Each block item declares:
- **`name`**: Unique identifier
- **`files`**: Source files with optional target path
- **`registryDependencies`**: Other registry items this block depends on
- **`categories`**: Grouping for the docs UI

## getAllBlocks()

```typescript
export function getAllBlocks(): RegistryBlockItem[] {
  return typedRegistry.items
    .filter((item) => item.type === "registry:block")
    .map((item) => ({
      name: item.name,
      type: item.type,
      title: item.title ?? item.name,
      description: item.description,
      files: (item.files ?? []).map((file) => ({
        ...file,
        target: displayTarget(file.target),
      })),
      registryDependencies: item.registryDependencies ?? [],
      categories: item.categories ?? [],
      meta: item.meta,
    }))
}
```

Filters to `type === "registry:block"` and normalizes target paths.

## Target Path Resolution

```typescript
const TARGET_PLACEHOLDERS: Record<string, string> = {
  "@components/": "components/",
  "@ui/": "components/ui/",
  "@lib/": "lib/",
  "@hooks/": "hooks/",
}

function displayTarget(target?: string): string | undefined {
  if (!target) return target
  for (const [prefix, replacement] of Object.entries(TARGET_PLACEHOLDERS)) {
    if (target.startsWith(prefix)) {
      return replacement + target.slice(prefix.length)
    }
  }
  return target
}
```

Aha: The shadcn target placeholders (`@lib/`, `@components/`, etc.) are resolved to friendly folder names for the docs file tree display. At install time, the actual path alias resolution happens in the consumer's project.

## File Tree Builder

```typescript
export function createFileTreeForRegistryItemFiles(
  files: Array<{ path: string; target?: string }>,
): FileTree[] {
  const root: FileTree[] = []

  for (const file of files) {
    const filePath = file.target ?? file.path
    const parts = filePath.split("/")
    let currentLevel = root

    for (let i = 0; i < parts.length; i++) {
      const part = parts[i]
      const isFile = i === parts.length - 1
      const existingNode = currentLevel.find((node) => node.name === part)

      if (existingNode) {
        if (isFile) existingNode.path = filePath
        else currentLevel = existingNode.children!
      } else {
        const newNode: FileTree = { name: part, path: isFile ? filePath : undefined }
        if (!isFile) newNode.children = []
        currentLevel.push(newNode)
        currentLevel = newNode.children ?? []
      }
    }
  }

  return root
}
```

Builds a hierarchical file tree from flat file paths. Shared parent directories become tree nodes with children.

## Block File Source

Source: `src/lib/get-block-file-source.ts`

```typescript
const SRC_DIR = process.cwd()

export function getBlockFileSource(registryPath: string): string {
  const filePath = path.join(SRC_DIR, registryPath)
  const source = fs.readFileSync(filePath, "utf-8")
  return source.replace(/@\/registry\/map/g, "@/components/ui/map")
}
```

Reads the raw source file from disk and replaces `@/registry/map` imports with `@/components/ui/map` — the path where installed components live in the consumer's project.

## Cross-References

- [Overview](00-overview.md) — Architecture summary
- [Components](02-components.md) — UI components, layout, search
- [App Structure](03-app-structure.md) — Next.js layout, CSS

[Next → Components](02-components.md)
