# BetterContext VS Code Extension - Deep Dive

## Overview

`bettercontextoai` is a VS Code extension that helps developers escape the context limitations of VS Code AI extensions by generating comprehensive context documents that can be pasted into web-based AI chats.

---

## Problem Statement

### The Problem
- VS Code AI extensions often have **restrictive context limitations**
- Web-based AI chats (ChatGPT, Claude, Gemini, Google AI Studio) offer **100K-1M+ token context windows**
- But using web AI requires **tedious file-by-file copy-pasting**
- Users end up choosing between **convenience (limited AI)** or **capability (manual work)**

### The Solution
Select multiple files/folders in VS Code, generate one comprehensive document (`FILE_CONTENT_MAP.md`), then paste everything into your preferred web AI chat in a single action.

---

## Extension Structure

```
bettercontextoai/
├── src/
│   ├── extension.ts         # Extension entry point
│   ├── fileSelectorView.ts  # File selector tree view
│   ├── contextGenerator.ts  # Context document generation
│   └── gitignore.ts         # .gitignore management
├── package.json             # Extension manifest
├── images/                  # Extension icons
└── test/                    # Extension tests
```

---

## Core Features

### 1. File Selector View

Interactive tree view in the Explorer pane:

```typescript
// File selector registration
const fileSelectorView = window.createTreeView('betterContextToAI.fileSelector', {
  treeDataProvider: new FileSelectorProvider()
});

// Toggle file selection
async function toggleFileSelection(file: Uri) {
  const isSelected = selectedFiles.has(file.fsPath);
  if (isSelected) {
    selectedFiles.delete(file.fsPath);
  } else {
    selectedFiles.add(file.fsPath);
  }
  refreshTreeView();
}
```

### 2. Context Document Generation

Generates `FILE_CONTENT_MAP.md`:

```typescript
// Context generation
async function generateContextDocument() {
  let content = '# Context\n\n';

  for (const filePath of selectedFiles) {
    const relativePath = workspace.asRelativePath(filePath);
    const fileContent = await workspace.fs.readFile(filePath);

    content += `## ${relativePath}\n\n`;
    content += '```' + getLanguageForFile(filePath) + '\n';
    content += fileContent.toString() + '\n';
    content += '```\n\n';
  }

  // Write to FILE_CONTENT_MAP.md
  const mapUri = Uri.joinPath(workspace.workspaceFolders![0].uri, 'FILE_CONTENT_MAP.md');
  await workspace.fs.writeFile(mapUri, new TextEncoder().encode(content));
}
```

### 3. Smart Filtering

- Files over 50KB automatically omitted
- Binary files and images skipped
- Nested/duplicate paths avoided
- `.gitignore` respected

### 4. Automatic .gitignore Management

```typescript
// Auto-add to .gitignore
async function addToGitignore() {
  const gitignorePath = Uri.joinPath(workspace.workspaceFolders![0].uri, '.gitignore');

  try {
    const content = await workspace.fs.readFile(gitignorePath);
    const lines = content.toString().split('\n');

    if (!lines.includes('FILE_CONTENT_MAP.md')) {
      lines.push('FILE_CONTENT_MAP.md');
      await workspace.fs.writeFile(
        gitignorePath,
        new TextEncoder().encode(lines.join('\n'))
      );
    }
  } catch {
    // Create new .gitignore
    await workspace.fs.writeFile(
      gitignorePath,
      new TextEncoder().encode('FILE_CONTENT_MAP.md\n')
    );
  }
}
```

---

## Usage Workflow

### Method 1: File Selector View

1. Open the **File Selector** view in Explorer pane
2. Click files/folders to toggle selection
3. Click **Generate** (⚡) icon to create `FILE_CONTENT_MAP.md`
4. Copy content and paste into AI chat

### Method 2: Explorer Context Menu

1. Right-click files in Explorer view
2. Select **"Select/Unselect for AI Context"**
3. Use Generate icon or command palette

### Method 3: Command Palette

```
Ctrl+Shift+P > Better Context to AI: Generate File Content Map
```

---

## Extension Commands

```json
{
  "commands": [
    {
      "command": "betterContextToAI.generate",
      "title": "Generate File Content Map"
    },
    {
      "command": "betterContextToAI.refresh",
      "title": "Refresh File Tree"
    },
    {
      "command": "betterContextToAI.selectFile",
      "title": "Select for AI Context"
    }
  ]
}
```

---

## Configuration

### Settings

```json
{
  "betterContextToAI.excludePatterns": [
    "**/node_modules/**",
    "**/dist/**",
    "**/*.min.js",
    "**/*.map"
  ],
  "betterContextToAI.maxFileSize": 51200,
  "betterContextToAI.outputFileName": "FILE_CONTENT_MAP.md"
}
```

---

## Visual Indicators

### Tree View Decoration

```typescript
// Show checkmark for selected files
const selectedDecoration = window.createFileDecoration2('✓', 'Selected', new ThemeColor('terminal.foreground'));

function getDecoration(file: Uri) {
  if (selectedFiles.has(file.fsPath)) {
    return selectedDecoration;
  }
  return undefined;
}
```

---

## Output Format

### FILE_CONTENT_MAP.md Structure

```markdown
# Context

## File: src/components/Button.tsx

```tsx
// File contents here with line numbers
```

## File: src/lib/utils.ts

```typescript
// File contents here
```

## Summary

- Total files: 5
- Total lines: 423
- Total tokens (est): ~600
```

---

## Release History

### 1.3.0 (Latest)
- Added "Select/Unselect for AI Context" in Explorer context menu
- Added Settings to Exclude specific file patterns, folders, extensions

### 1.2.9
- Visual indicators (✓) in Explorer view
- Automatic .gitignore management
- Hide `FILE_CONTENT_MAP.md` from File Selector
- Generate (⚡) and Refresh (🔃) icons in title bar

### 1.1.1
- MIT License added

---

## Supported Languages

All programming languages and file types supported by VS Code are fully supported. The extension:
- Detects file extension for syntax highlighting
- Handles text-based files only
- Skips binary files automatically

---

## Why Use This Extension?

### Benefits

| Benefit | Description |
|---------|-------------|
| Escape context limitations | Leverage web AI models with 100K+ token windows |
| Save money | Web AI chats are often free or cheaper than premium VS Code extensions |
| End copy-paste hell | No more manually copying dozens of files |
| Best of both worlds | Keep VS Code workflow while accessing powerful AI models |

---

## Rust Implementation Considerations

For a Rust-based VS Code extension (using `zed` or custom LSP):

### Architecture

```
bettercontext-rs/
├── src/
│   ├── extension.rs      # Extension entry
│   ├── file_selector.rs  # File selection tree
│   ├── generator.rs      # Context generation
│   └── gitignore.rs      # .gitignore handling
├── Cargo.toml
└── package.json          # VS Code extension manifest
```

### Key Crates

- `tower-lsp` - LSP server for VS Code integration
- `tree-sitter` - Syntax highlighting detection
- `ignore` - .gitignore parsing
- `tokio` - Async file operations

### Performance Optimizations

- Use `memmap2` for efficient file reading
- Parallel file processing with `rayon`
- Incremental updates (only changed files)
- Token counting with `tiktoken-rs`
