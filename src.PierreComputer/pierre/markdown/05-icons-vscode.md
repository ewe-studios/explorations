---
title: Icons and VS Code Extension
prev: 04-pierre-monorepo.md
---

# Icons and VS Code Extension

300+ React icon components and VS Code extension.

## Icons Package

**Location:** `src.PierreComputer/icons/`

### Structure

```
icons/
├── src/
│   ├── icons/          # 300+ icon components
│   │   ├── ArrowRight.tsx
│   │   ├── GitBranch.tsx
│   │   ├── FileCode.tsx
│   │   └── ...
│   ├── index.ts        # Main export
│   └── types.ts        # TypeScript definitions
├── svg/                # SVG source files
└── scripts/            # Build scripts
    ├── build-icons.ts
    └── build-sprite.ts
```

### Icon Component

```typescript
// src/icons/ArrowRight.tsx
import { IconProps } from '../types';

export function ArrowRight({ size = 24, color = 'currentColor' }: IconProps) {
  return (
    <svg
      width={size}
      height={size}
      viewBox="0 0 24 24"
      fill="none"
      stroke={color}
    >
      <path d="M5 12h14M12 5l7 7-7 7" />
    </svg>
  );
}
```

### SVG Sprite

**Aha:** Sprite generation reduces HTTP requests:

```typescript
// scripts/build-sprite.ts
import { glob } from 'glob';
import { readFileSync, writeFileSync } from 'fs';

const svgs = await glob('svg/*.svg');
const symbols = svgs.map(file => {
  const content = readFileSync(file, 'utf-8');
  const name = basename(file, '.svg');
  return `<symbol id="${name}">${content}</symbol>`;
});

const sprite = `<svg xmlns="http://www.w3.org/2000/svg">${symbols.join('')}</svg>`;
writeFileSync('dist/sprite.svg', sprite);
```

## VS Code Extension

**Location:** `src.PierreComputer/vscode-icons/`

### Manifest

```json
// package.json
{
  "name": "pierre-icons",
  "displayName": "Pierre Icons",
  "version": "1.0.0",
  "engines": {
    "vscode": "^1.74.0"
  },
  "contributes": {
    "iconThemes": [
      {
        "id": "pierre",
        "label": "Pierre Icons",
        "path": "./icons.json"
      }
    ]
  }
}
```

### Icon Theme

```json
// icons.json
{
  "iconDefinitions": {
    "fileCode": {
      "iconPath": "./icons/file-code.svg"
    },
    "folderSrc": {
      "iconPath": "./icons/folder-src.svg"
    }
  },
  "file": "fileCode",
  "folder": "folderSrc",
  "folderNames": {
    "src": "folderSrc",
    "source": "folderSrc"
  }
}
```

## Build Process

```bash
# Build all icons
bun run build

# Generate sprite
bun run build:sprite

# Build VS Code extension
bun run build:vscode
```

## Usage

### React

```tsx
import { ArrowRight, GitBranch } from '@pierre/icons';

function MyComponent() {
  return <ArrowRight size={16} />;
}
```

### SVG Sprite

```html
<svg>
  <use href="sprite.svg#arrow-right" />
</svg>
```

## Icon Categories

| Category | Count | Examples |
|----------|-------|----------|
| Navigation | 45 | Arrow, Menu, Home |
| Git | 30 | Branch, Commit, Merge |
| Files | 60 | Document, Folder, Code |
| Actions | 80 | Add, Delete, Edit |
| Status | 40 | Success, Warning, Error |
| Misc | 45 | Settings, User, Search |

## Next Steps

- Explore `src/icons/` for full icon set
- See `vscode-icons/` for extension details
- Use `@pierre/icons` in your projects
