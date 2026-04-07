# Repomix Deep Dive Exploration

**Project:** Repomix  
**Source:** `/home/darkvoid/Boxxed/@formulas/src.rust/src.llamacpp/src.ClaudOpen/repomix`  
**Created:** 2026-04-07  
**Lines:** ~15,000+ across 126 TypeScript files

---

## Executive Summary

Repomix is a **repository packing tool** designed to bundle the contents of a codebase into a single, AI-friendly output file. It transforms entire repositories into formats optimized for LLM analysis, making it easier for AI systems to understand and process codebases.

**Key Value Propositions:**
1. **AI-Optimized Output** — Formats designed specifically for LLM consumption
2. **Security Screening** — Automatic detection and exclusion of sensitive files
3. **Flexible Output** — XML, Markdown, Plain text, and JSON formats
4. **Git Integration** — Include git diffs and commit history
5. **Token Counting** — Accurate token estimation for LLM context limits
6. **Multi-Root Support** — Pack multiple directories into one output
7. **Skill Generation** — Generate Claude Code skills from repositories

**Quick Start:**
```bash
npm install -g repomix
repomix                          # Pack current directory
repomix /path/to/repo            # Pack specific directory
repomix --remote https://github.com/user/repo  # Pack remote repo
```

---

## Architecture Overview

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                          Repomix Architecture                                 │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│  ┌─────────────────────────────────────────────────────────────────────┐   │
│  │                    CLI Layer (cli/)                                  │   │
│  │  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐              │   │
│  │  │  cliRun.ts   │  │  cliSpinner  │  │  cliReport   │              │   │
│  │  │              │  │              │  │              │              │   │
│  │  │  - Argument  │  │  - Progress  │  │  - Output    │              │   │
│  │  │    parsing   │  │    display   │  │    reporting │              │   │
│  │  └──────────────┘  └──────────────┘  └──────────────┘              │   │
│  │  ┌──────────────────────────────────────────────────────────────┐   │   │
│  │  │                    Actions                                    │   │   │
│  │  │  defaultAction | initAction | remoteAction | mcpAction       │   │   │
│  │  └──────────────────────────────────────────────────────────────┘   │   │
│  └─────────────────────────────────────────────────────────────────────┘   │
│                              │                                              │
│                              ▼                                              │
│  ┌─────────────────────────────────────────────────────────────────────┐   │
│  │                    Core Layer (core/)                                │   │
│  │                                                                      │   │
│  │  ┌──────────────────────────────────────────────────────────────┐   │   │
│  │  │  packager.ts — Main orchestration                            │   │   │
│  │  │  ┌────────┐ ┌────────┐ ┌────────┐ ┌────────┐ ┌────────┐    │   │   │
│  │  │  │ Search │→│Collect │→│Security│→│Process │→│ Output │    │   │   │
│  │  │  │ Files  │  │ Files  │  │ Check  │  │ Files  │  │ Gen   │    │   │   │
│  │  │  └────────┘ └────────┘ └────────┘ └────────┘ └────────┘    │   │   │
│  │  └──────────────────────────────────────────────────────────────┘   │   │
│  │                                                                      │   │
│  │  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐                 │   │
│  │  │   file/     │  │   metrics/  │  │   output/   │                 │   │
│  │  │             │  │             │  │             │                 │   │
│  │  │ - search    │  │ - Token     │  │ - XML       │                 │   │
│  │  │ - collect   │  │   counting  │  │ - Markdown  │                 │   │
│  │  │ - process   │  │ - Worker    │  │ - Plain     │                 │   │
│  │  │ - tree      │  │   pools     │  │ - JSON      │                 │   │
│  │  └─────────────┘  └─────────────┘  └─────────────┘                 │   │
│  │                                                                      │   │
│  │  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐                 │   │
│  │  │   git/      │  │   security/ │  │tree-sitter/ │                 │   │
│  │  │             │  │             │  │             │                 │   │
│  │  │ - Diff      │  │ - Secret    │  │ - AST       │                 │   │
│  │  │ - Log       │  │   detection │  │   parsing   │                 │   │
│  │  │ - Remote    │  │ - Pattern   │  │ - Language  │                 │   │
│  │  │   parsing   │  │   matching  │  │   loaders   │                 │   │
│  │  └─────────────┘  └─────────────┘  └─────────────┘                 │   │
│  └─────────────────────────────────────────────────────────────────────┘   │
│                              │                                              │
│                              ▼                                              │
│  ┌─────────────────────────────────────────────────────────────────────┐   │
│  │                    Config Layer (config/)                            │   │
│  │  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐              │   │
│  │  │ configLoad   │  │configSchema  │  │defaultIgnore │              │   │
│  │  │              │  │              │  │              │              │   │
│  │  │ - Load from  │  │ - Zod schema │  │ - Default    │              │   │
│  │  │   file       │  │ - Validation │  │   patterns   │              │   │
│  │  │ - Merge cfgs │  │ - Types      │  │ - .gitignore │              │   │
│  │  └──────────────┘  └──────────────┘  └──────────────┘              │   │
│  └─────────────────────────────────────────────────────────────────────┘   │
│                              │                                              │
│                              ▼                                              │
│              ┌───────────────────────────────┐                             │
│              │   Shared Utilities (shared/)  │                             │
│              │  - Logger (picocolors)        │                             │
│              │  - Error handling             │                             │
│              │  - Process concurrency        │                             │
│              │  - Unified workers            │                             │
│              └───────────────────────────────┘                             │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘
```

---

## Directory Structure

```
repomix/
├── src/
│   ├── cli/                          # CLI application layer
│   │   ├── actions/
│   │   │   ├── defaultAction.ts      # Default packing action
│   │   │   ├── initAction.ts         # Config initialization
│   │   │   ├── remoteAction.ts       # Remote repo packing
│   │   │   ├── mcpAction.ts          # MCP server action
│   │   │   ├── migrationAction.ts    # Config migration
│   │   │   └── versionAction.ts      # Version display
│   │   ├── prompts/
│   │   │   └── skillPrompts.ts       # Skill generation prompts
│   │   ├── reporters/
│   │   │   └── tokenCountTreeReporter.ts  # Token tree reporting
│   │   ├── cliRun.ts                 # Main CLI entry point
│   │   ├── cliReport.ts              # CLI output reporting
│   │   ├── cliSpinner.ts             # Progress spinners
│   │   ├── types.ts                  # CLI types
│   │   └── cliSpinner.ts             # Progress indicators
│   │
│   ├── config/                       # Configuration management
│   │   ├── configLoad.ts             # Load and merge configs
│   │   ├── configSchema.ts           # Zod schemas
│   │   ├── defaultIgnore.ts          # Default ignore patterns
│   │   └── globalDirectory.ts        # Global config directory
│   │
│   ├── core/                         # Core packing logic
│   │   ├── file/                     # File operations
│   │   │   ├── fileCollect.ts        # Collect file contents
│   │   │   ├── fileSearch.ts         # Find files via globby
│   │   │   ├── fileProcess.ts        # Process file contents
│   │   │   ├── fileProcessContent.ts # Remove comments, etc.
│   │   │   ├── filePathSort.ts       # Sort file paths
│   │   │   ├── fileRead.ts           # Read file contents
│   │   │   ├── fileStdin.ts          # STDIN handling
│   │   │   ├── fileTreeGenerate.ts   # Generate tree structure
│   │   │   ├── fileTypes.ts          # File type definitions
│   │   │   ├── packageJsonParse.ts   # Parse package.json
│   │   │   ├── permissionCheck.ts    # Check directory permissions
│   │   │   └── truncateBase64.ts     # Truncate large base64
│   │   │
│   │   ├── git/                      # Git integration
│   │   │   ├── gitDiffHandle.ts      # Handle git diffs
│   │   │   ├── gitLogHandle.ts       # Handle git logs
│   │   │   └── gitRemoteParse.ts     # Parse remote URLs
│   │   │
│   │   ├── metrics/                  # Metrics calculation
│   │   │   ├── TokenCounter.ts       # Token counting
│   │   │   ├── calculateMetrics.ts   # Calculate all metrics
│   │   │   ├── calculateOutputMetrics.ts  # Output metrics
│   │   │   └── workers/              # Worker thread handling
│   │   │
│   │   ├── output/                   # Output generation
│   │   │   ├── outputGenerator.ts    # Main output generator
│   │   │   ├── outputGeneratorTypes.ts  # Type definitions
│   │   │   ├── outputStyleDecorate.ts   # Header/footer generation
│   │   │   ├── outputStyles/         # Output style templates
│   │   │   │   ├── xmlStyle.ts       # XML template
│   │   │   │   ├── markdownStyle.ts  # Markdown template
│   │   │   │   └── plainStyle.ts     # Plain text template
│   │   │   └── outputSort.ts         # Sort output files
│   │   │
│   │   ├── packager/                 # Packaging orchestration
│   │   │   ├── copyToClipboardIfEnabled.ts
│   │   │   ├── produceOutput.ts      # Produce final output
│   │   │   └── writeOutputToDisk.ts  # Write to filesystem
│   │   │
│   │   ├── security/                 # Security checks
│   │   │   ├── securityCheck.ts      # Run security check
│   │   │   ├── validateFileSafety.ts # Validate safety results
│   │   │   └── workers/
│   │   │       └── securityCheckWorker.ts  # Security worker
│   │   │
│   │   ├── skill/                    # Skill generation
│   │   │   └── packSkill.ts          # Generate Claude Code skill
│   │   │
│   │   ├── tokenCount/               # Token counting utilities
│   │   │   └── TokenCounterFactory.ts
│   │   │
│   │   └── treeSitter/               # AST parsing
│   │       ├── parseFile.ts          # Parse files with tree-sitter
│   │       └── loadLanguage.ts       # Load language grammars
│   │
│   ├── mcp/                          # MCP server support
│   │   └── server.ts                 # MCP server implementation
│   │
│   └── shared/                       # Shared utilities
│       ├── errorHandle.ts            # Error handling
│       ├── logger.ts                 # Logging (picocolors)
│       ├── memoryUtils.ts            # Memory usage logging
│       ├── processConcurrency.ts     # Worker thread management
│       ├── types.ts                  # Shared types
│       └── unifiedWorker.ts          # Unified worker handling
│
├── tests/                            # Test files (mirrors src/)
├── website/                          # Documentation website
│   ├── client/                       # VitePress documentation
│   └── server/                       # Server-side API
│
├── package.json
├── repomix.config.json               # Default configuration
├── repomix-instruction.md            # Project structure docs
└── tsconfig.json
```

---

## Core Concepts

### 1. Packing Pipeline

The main packing flow in `core/packager.ts`:

```typescript
// src/core/packager.ts (lines 57-240)
export const pack = async (
  rootDirs: string[],
  config: RepomixConfigMerged,
  progressCallback: RepomixProgressCallback = () => {},
  overrideDeps: Partial<typeof defaultDeps> = {},
  explicitFiles?: string[],
  options: PackOptions = {},
): Promise<PackResult> => {
  const deps = { ...defaultDeps, ...overrideDeps };

  logMemoryUsage('Pack - Start');

  // 1. Search for files
  progressCallback('Searching for files...');
  const searchResultsByDir = await searchFiles(rootDirs, config, explicitFiles);

  // 2. Sort file paths
  progressCallback('Sorting files...');
  const sortedFilePaths = sortPaths(allFilePaths);

  // 3. Warm up metrics worker pool (overlap with subsequent stages)
  const { taskRunner: metricsTaskRunner, warmupPromise } = 
    createMetricsTaskRunner(fileCount, config.tokenCount.encoding);

  try {
    // 4. Collect files and git data in parallel
    progressCallback('Collecting files...');
    const [collectResults, gitDiffResult, gitLogResult] = await Promise.all([
      collectFiles(sortedFilePaths, config),
      getGitDiffs(rootDirs, config),
      getGitLogs(rootDirs, config),
    ]);

    // 5. Security check and file processing in parallel
    const [validationResult, allProcessedFiles] = await Promise.all([
      validateFileSafety(rawFiles, config, gitDiffResult, gitLogResult),
      processFiles(rawFiles, config),
    ]);

    // 6. Filter out suspicious files
    const processedFiles = filterSuspiciousFiles(allProcessedFiles, validationResult);

    // 7. Generate output (with concurrent metrics calculation)
    progressCallback('Generating output...');
    const outputPromise = produceOutput(rootDirs, config, processedFiles, ...);
    const metricsPromise = calculateMetrics(processedFiles, outputPromise, ...);

    const [{ outputFiles }, metrics] = await Promise.all([outputPromise, metricsPromise]);

    // 8. Return result
    return {
      ...metrics,
      outputFiles,
      suspiciousFilesResults: validationResult.suspiciousFilesResults,
      processedFiles,
      safeFilePaths: validationResult.safeFilePaths,
      skippedFiles: collectResults.skippedFiles,
    };
  } finally {
    await metricsTaskRunner.cleanup();
  }
};
```

### 2. File Search

```typescript
// src/core/file/fileSearch.ts (lines 96-250+)
export const searchFiles = async (
  rootDir: string,
  config: RepomixConfigMerged,
  explicitFiles?: string[],
): Promise<FileSearchResult> => {
  // 1. Validate path exists and is a directory
  const pathStats = await fs.stat(rootDir);
  if (!pathStats.isDirectory()) {
    throw new RepomixError(`Target path is not a directory: ${rootDir}`);
  }

  // 2. Check permissions
  const permissionCheck = await checkDirectoryPermissions(rootDir);

  // 3. Prepare ignore patterns (combine .gitignore, .repomixignore, default)
  const { adjustedIgnorePatterns, ignoreFilePatterns } = 
    await prepareIgnoreContext(rootDir, config);

  // 4. Build glob patterns from include config
  const globPatterns = config.include.map(pattern => 
    escapeGlobPattern(normalizeGlobPattern(pattern))
  );

  // 5. Execute search with globby
  const filePaths = await globby(globPatterns, {
    cwd: rootDir,
    ignore: adjustedIgnorePatterns,
    onlyFiles: true,
    absolute: false,
    dot: true,
  });

  // 6. Find empty directories if configured
  const emptyDirPaths = config.output.includeEmptyDirectories
    ? await findEmptyDirectories(rootDir, directories, ignorePatterns)
    : [];

  return { filePaths, emptyDirPaths };
};
```

### 3. Security Check

```typescript
// src/core/security/securityCheck.ts (lines 30-136)
const BATCH_SIZE = 50;

export const runSecurityCheck = async (
  rawFiles: RawFile[],
  progressCallback: RepomixProgressCallback = () => {},
  gitDiffResult?: GitDiffResult,
  gitLogResult?: GitLogResult,
): Promise<SuspiciousFileResult[]> => {
  // 1. Prepare items (files + git diffs + git logs)
  const fileItems = rawFiles.map(file => ({
    filePath: file.path,
    content: file.content,
    type: 'file' as const,
  }));

  const gitDiffItems = gitDiffResult ? [...] : [];
  const gitLogItems = gitLogResult ? [...] : [];

  const allItems = [...fileItems, ...gitDiffItems, ...gitLogItems];

  // 2. Cap workers at 2 to reduce contention with metrics
  const maxSecurityWorkers = Math.min(2, os.cpus().length);

  // 3. Initialize task runner with worker threads
  const taskRunner = initTaskRunner<SecurityCheckTask, SuspiciousFileResult[]>({
    numOfTasks: allItems.length,
    workerType: 'securityCheck',
    runtime: 'worker_threads',
    maxWorkerThreads: maxSecurityWorkers,
  });

  // 4. Split into batches to reduce IPC overhead
  const batches = chunk(allItems, BATCH_SIZE);

  // 5. Process in parallel
  const batchResults = await Promise.all(
    batches.map(batch => taskRunner.run({ items: batch }))
  );

  // 6. Flatten and filter results
  return batchResults.flat().filter(result => result !== null);
};
```

**Security Check Patterns:**
- API keys (AWS, GitHub, Google, etc.)
- Private keys (RSA, SSH, PGP)
- Password files
- .env files with secrets
- Credential files
- Token files

### 4. Output Generation

```typescript
// src/core/output/outputGenerate.ts (lines 79-106)
export const createRenderContext = (
  outputGeneratorContext: OutputGeneratorContext
): RenderContext => {
  return {
    generationHeader: generateHeader(config, generationDate),
    summaryPurpose: generateSummaryPurpose(config),
    summaryFileFormat: generateSummaryFileFormat(),
    summaryUsageGuidelines: generateSummaryUsageGuidelines(config, instruction),
    summaryNotes: generateSummaryNotes(config),
    headerText: config.output.headerText,
    instruction: outputGeneratorContext.instruction,
    treeString: outputGeneratorContext.treeString,
    processedFiles: outputGeneratorContext.processedFiles,
    fileLineCounts: calculateFileLineCounts(processedFiles),
    fileSummaryEnabled: config.output.fileSummary,
    directoryStructureEnabled: config.output.directoryStructure,
    filesEnabled: config.output.files,
    escapeFileContent: config.output.parsableStyle,
    markdownCodeBlockDelimiter: calculateMarkdownDelimiter(processedFiles),
    gitDiffEnabled: config.output.git?.includeDiffs,
    gitDiffWorkTree: gitDiffResult?.workTreeDiffContent,
    gitDiffStaged: gitDiffResult?.stagedDiffContent,
    gitLogEnabled: config.output.git?.includeLogs,
    gitLogContent: gitLogResult?.logContent,
    gitLogCommits: gitLogResult?.commits,
  };
};
```

---

## Configuration System

### Config Schema

```typescript
// src/config/configSchema.ts (lines 17-169)
export const repomixConfigBaseSchema = z.object({
  input: z.object({
    maxFileSize: z.number().optional(),  // Default: 50MB
  }).optional(),
  
  output: z.object({
    filePath: z.string().optional(),
    style: z.enum(['xml', 'markdown', 'plain', 'json']).optional(),
    parsableStyle: z.boolean().optional(),
    headerText: z.string().optional(),
    instructionFilePath: z.string().optional(),
    fileSummary: z.boolean().default(true),
    directoryStructure: z.boolean().default(true),
    files: z.boolean().default(true),
    removeComments: z.boolean().default(false),
    removeEmptyLines: z.boolean().default(false),
    compress: z.boolean().default(false),
    topFilesLength: z.number().default(5),
    showLineNumbers: z.boolean().default(false),
    truncateBase64: z.boolean().default(false),
    copyToClipboard: z.boolean().default(false),
    includeEmptyDirectories: z.boolean().optional(),
    splitOutput: z.number().int().min(1).optional(),
    tokenCountTree: z.union([z.boolean(), z.number()]).default(false),
    git: z.object({
      sortByChanges: z.boolean().default(true),
      sortByChangesMaxCommits: z.number().default(100),
      includeDiffs: z.boolean().default(false),
      includeLogs: z.boolean().default(false),
      includeLogsCount: z.number().default(50),
    }).optional(),
  }).optional(),
  
  include: z.array(z.string()).default([]),
  
  ignore: z.object({
    useGitignore: z.boolean().default(true),
    useDotIgnore: z.boolean().default(true),
    useDefaultPatterns: z.boolean().default(true),
    customPatterns: z.array(z.string()).default([]),
  }).optional(),
  
  security: z.object({
    enableSecurityCheck: z.boolean().default(true),
  }).optional(),
  
  tokenCount: z.object({
    encoding: z.enum(['o200k_base', 'cl100k_base', 'p50k_base', 'r50k_base']).default('o200k_base'),
  }).optional(),
});
```

### Example Configuration

```json
{
  "$schema": "https://repomix.com/schemas/latest/schema.json",
  "input": {
    "maxFileSize": 50000000
  },
  "output": {
    "filePath": "repomix-output.xml",
    "style": "xml",
    "parsableStyle": false,
    "headerText": "Custom header for AI context",
    "instructionFilePath": "AI_INSTRUCTIONS.md",
    "fileSummary": true,
    "directoryStructure": true,
    "files": true,
    "removeComments": false,
    "removeEmptyLines": false,
    "topFilesLength": 5,
    "showLineNumbers": false,
    "truncateBase64": true,
    "includeEmptyDirectories": true,
    "tokenCountTree": 50000,
    "git": {
      "sortByChanges": true,
      "sortByChangesMaxCommits": 100,
      "includeDiffs": true,
      "includeLogs": true,
      "includeLogsCount": 50
    }
  },
  "include": ["**/*.ts", "**/*.tsx", "**/*.js"],
  "ignore": {
    "useGitignore": true,
    "useDefaultPatterns": true,
    "customPatterns": ["**/*.test.ts", "**/node_modules/**"]
  },
  "security": {
    "enableSecurityCheck": true
  },
  "tokenCount": {
    "encoding": "o200k_base"
  }
}
```

---

## Output Styles

### XML Output (Default)

```xml
<!-- 
================================================================
File Summary
================================================================

This file contains the packed contents of the repository.
Purpose: AI codebase analysis
================================================================
-->
<repomix>
  <file_summary>
    <purpose>AI codebase analysis</purpose>
    <file_format>
      1. This XML header
      2. Directory structure
      3. Repository files
    </file_format>
    <usage_guidelines>...</usage_guidelines>
  </file_summary>
  
  <directory_structure>
src/
├── cli/
│   ├── actions/
│   └── cliRun.ts
├── config/
│   └── configSchema.ts
└── core/
    └── packager.ts
  </directory_structure>
  
  <files>
    <file path="src/core/packager.ts">
import type { RepomixConfigMerged } from '../config/configSchema.js';

export const pack = async (...) => {
  // Implementation
};
    </file>
    
    <file path="src/config/configSchema.ts">
import { z } from 'zod';

export const repomixConfigBaseSchema = z.object({...});
    </file>
  </files>
  
  <git_diffs>
    <git_diff_work_tree>...</git_diff_work_tree>
    <git_diff_staged>...</git_diff_staged>
  </git_diffs>
  
  <git_logs>
    <git_log_commit>
      <date>2026-04-07</date>
      <message>Fix bug in packager</message>
      <files>
        <file>src/core/packager.ts</file>
      </files>
    </git_log_commit>
  </git_logs>
</repomix>
```

### Markdown Output

```markdown
<!-- Repomix Generated: 2026-04-07 -->

# File Summary

## Purpose
AI codebase analysis

## File Format
1. This markdown header
2. Directory structure  
3. Repository files

## Usage Guidelines
...

---

# Directory Structure

```
src/
├── cli/
│   ├── actions/
│   └── cliRun.ts
└── core/
    └── packager.ts
```

---

# Files

## src/core/packager.ts

\```typescript
import type { RepomixConfigMerged } from '../config/configSchema.js';

export const pack = async (...) => {
  // Implementation
};
\```

## src/config/configSchema.ts

\```typescript
import { z } from 'zod';

export const repomixConfigBaseSchema = z.object({...});
\```
```

---

## File Processing

### Process Flow

```typescript
// src/core/file/fileProcess.ts
export const processFiles = async (
  rawFiles: RawFile[],
  config: RepomixConfigMerged,
  progressCallback: RepomixProgressCallback = () => {},
): Promise<ProcessedFile[]> => {
  const processedFiles: ProcessedFile[] = [];

  for (const rawFile of rawFiles) {
    const content = await processFileContent(rawFile.content, rawFile.path, config);
    processedFiles.push({
      path: rawFile.path,
      content,
    });
    
    progressCallback(`Processing file: ${rawFile.path}`);
  }

  return processedFiles;
};
```

### Content Processing Options

| Option | Description | Default |
|--------|-------------|---------|
| `removeComments` | Remove code comments | false |
| `removeEmptyLines` | Remove empty lines | false |
| `compress` | Full compression | false |
| `truncateBase64` | Truncate large base64 strings | true |
| `showLineNumbers` | Add line numbers | false |

---

## Security Check

### Detection Patterns

The security check worker (`src/core/security/workers/securityCheckWorker.ts`) detects:

| Pattern Type | Examples |
|--------------|----------|
| **API Keys** | AWS Access Key, GitHub Token, Google API Key |
| **Private Keys** | RSA Private Key, SSH Key, PGP Key |
| **Passwords** | password=, passwd=, pwd= |
| **Environment Files** | .env, .env.local, .env.production |
| **Credential Files** | credentials, secrets, tokens |
| **Config Files** | config with sensitive data |

### Security Check Result

```typescript
export interface SuspiciousFileResult {
  filePath: string;
  messages: string[];        // Why it was flagged
  type: 'file' | 'gitDiff' | 'gitLog';
}

// Example result:
{
  filePath: '.env',
  messages: [
    'Potential secret detected: AWS_SECRET_ACCESS_KEY pattern found',
    'Environment file with sensitive values detected'
  ],
  type: 'file'
}
```

---

## Git Integration

### Git Diff Support

```typescript
// src/core/git/gitDiffHandle.ts
export interface GitDiffResult {
  workTreeDiffContent?: string;    // Uncommitted changes
  stagedDiffContent?: string;      // Staged changes
}

export const getGitDiffs = async (
  rootDirs: string[],
  config: RepomixConfigMerged,
): Promise<GitDiffResult> => {
  if (!config.output.git?.includeDiffs) {
    return {};
  }

  // Get work tree diff
  const workTreeDiff = await execGit('diff', { cwd: rootDirs[0] });
  
  // Get staged diff
  const stagedDiff = await execGit('diff --cached', { cwd: rootDirs[0] });

  return {
    workTreeDiffContent: workTreeDiff || undefined,
    stagedDiffContent: stagedDiff || undefined,
  };
};
```

### Git Log Support

```typescript
// src/core/git/gitLogHandle.ts
export interface GitLogResult {
  logContent?: string;
  commits: Array<{
    date: string;
    message: string;
    files: string[];
  }>;
}
```

---

## Token Counting

### TokenCounter

```typescript
// src/core/metrics/TokenCounter.ts
import { encodingForModel } from 'js-tiktoken';

export class TokenCounter {
  private encoding: Tiktoken;

  constructor(encoding: 'o200k_base' | 'cl100k_base' | 'p50k_base' | 'r50k_base') {
    this.encoding = encodingForModel(encoding);
  }

  count(text: string): number {
    return this.encoding.encode(text).length;
  }

  cleanup(): void {
    this.encoding.free();
  }
}
```

### Supported Encodings

| Encoding | Models |
|----------|--------|
| `o200k_base` | Claude 3.5/3.7, GPT-4o |
| `cl100k_base` | Claude 2/3, GPT-4 |
| `p50k_base` | GPT-3, Codex |
| `r50k_base` | GPT-3, early models |

---

## CLI Commands

### Basic Usage

```bash
# Pack current directory
repomix

# Pack specific directory
repomix /path/to/repo

# Pack with custom config
repomix --config repomix.custom.json

# Output to stdout
repomix --stdout > output.xml

# Pack remote repository
repomix --remote https://github.com/user/repo

# Pack specific branch
repomix --remote https://github.com/user/repo --remote-branch main

# Generate skill for Claude Code
repomix --skill-generate MySkill

# Include git diffs
repomix --git-include-diffs

# Output as markdown
repomix --style markdown
```

### CLI Options

| Option | Description |
|--------|-------------|
| `--output <file>` | Output file path |
| `--style <style>` | Output style (xml, markdown, plain, json) |
| `--config <path>` | Custom config file |
| `--remote <url>` | Remote repository URL |
| `--remote-branch <branch>` | Remote branch/tag |
| `--stdout` | Output to stdout |
| `--verbose` | Verbose logging |
| `--git-include-diffs` | Include git diffs |
| `--git-include-logs` | Include git logs |
| `--skill-generate <name>` | Generate Claude Code skill |
| `--truncate-base64` | Truncate base64 strings |
| `--no-security-check` | Disable security check |

---

## MCP Server Support

Repomix can run as an MCP server:

```typescript
// src/mcp/server.ts
import { Server } from '@modelcontextprotocol/sdk/server/index.js';
import { StdioServerTransport } from '@modelcontextprotocol/sdk/server/stdio.js';

export async function startMCPServer() {
  const server = new Server(
    { name: 'repomix', version: '1.0.0' },
    { capabilities: { resources: {}, tools: {} } }
  );

  // tools/pack - Pack a repository
  server.setRequestHandler('tools/call', async (request) => {
    const { name, arguments: args } = request.params;
    
    if (name === 'pack') {
      const result = await pack([args.path], config);
      return {
        content: [{ type: 'text', text: result.outputFiles?.[0] || 'No output' }],
      };
    }
  });

  const transport = new StdioServerTransport();
  await server.connect(transport);
}
```

---

## Skill Generation

Repomix can generate Claude Code skills:

```typescript
// src/core/skill/packSkill.ts
export const packSkill = async (
  rootDirs: string[],
  config: RepomixConfigMerged,
  options: PackOptions,
): Promise<PackResult> => {
  // Generate skill.json with metadata
  const skillJson = {
    name: options.skillName,
    version: '1.0.0',
    source: options.skillSourceUrl,
    description: `Packed from ${rootDirs.join(', ')}`,
  };

  // Generate skill content (packed repository)
  const skillContent = await generateSkillContent(rootDirs, config);

  // Write skill files
  await writeSkillFiles(skillJson, skillContent, options.skillDir);

  return { /* PackResult */ };
};
```

---

## Data Flow

### Complete Pack Flow

```
┌──────────────┐     ┌──────────────┐     ┌──────────────┐
│   CLI Run    │────▶│ Default      │────▶│   Pack       │
│   (cliRun)   │     │ Action       │     │   (Core)     │
└──────────────┘     └──────────────┘     └──────────────┘
                            │                    │
                            │                    ▼
                            │           ┌─────────────────┐
                            │           │ 1. Search Files │
                            │           │   (globby)      │
                            │           └─────────────────┘
                            │                    │
                            │                    ▼
                            │           ┌─────────────────┐
                            │           │ 2. Sort Paths   │
                            │           │   (topological) │
                            │           └─────────────────┘
                            │                    │
                            │                    ▼
                            │           ┌─────────────────┐
                            │           │ 3. Collect      │◀────┐
                            │           │   (read files)  │     │
                            │           └─────────────────┘     │
                            │                    │              │
                            │                    ▼              │
                            │           ┌─────────────────┐     │ Parallel
                            │           │ 4. Git Diff/Log │◀────┘
                            │           └─────────────────┘
                            │                    │
                            │                    ▼
                            │           ┌─────────────────┐
                            │           │ 5. Security     │
                            │           │   Check (workers)
                            │           └─────────────────┘
                            │                    │
                            │                    ▼
                            │           ┌─────────────────┐
                            │           │ 6. Process      │
                            │           │   (remove comments)
                            │           └─────────────────┘
                            │                    │
                            │                    ▼
                            │           ┌─────────────────┐
                            │           │ 7. Generate     │
                            │           │   Output        │
                            │           └─────────────────┘
                            │                    │
                            │                    ▼
                            │           ┌─────────────────┐
                            └──────────▶│ 8. Write Output │
                                        │   (file/clipboard)
                                        └─────────────────┘
```

---

## Key Design Decisions

### 1. Worker Thread Parallelism

Repomix uses worker threads for:
- **Security check** — Pattern matching in parallel
- **Token counting** — CPU-intensive encoding
- **File processing** — When compress/removeComments enabled

**Batching Strategy:**
```typescript
const BATCH_SIZE = 50;  // Files per worker message

// Split items into batches to reduce IPC overhead
const batches = chunk(allItems, BATCH_SIZE);

// Process in parallel across workers
const results = await Promise.all(
  batches.map(batch => taskRunner.run({ items: batch }))
);
```

### 2. Dependency Injection for Testability

All core functions use dependency injection:

```typescript
export const processFiles = async (
  rawFiles: RawFile[],
  config: RepomixConfigMerged,
  progressCallback: RepomixProgressCallback = () => {},
  deps = {
    processFileContent,
    parseFile,
  },
): Promise<ProcessedFile[]> => {
  // Use deps.processFileContent instead of direct call
};
```

### 3. Config Merging Strategy

Configs are merged in order:
1. **Default config** — Built-in defaults
2. **File config** — `repomix.config.json`
3. **CLI config** — Command-line flags
4. **Merged config** — Final result

```typescript
export const mergeConfigs = (
  defaultConfig: RepomixConfigDefault,
  fileConfig: RepomixConfigFile,
  cliConfig: RepomixConfigCli,
): RepomixConfigMerged => {
  return {
    ...defaultConfig,
    ...fileConfig,
    ...cliConfig,
    cwd: process.cwd(),
  };
};
```

### 4. Parsable Output Mode

`parsableStyle` ensures output can be parsed by XML/Markdown parsers:

```typescript
// XML: Properly escape special characters
if (config.output.parsableStyle && config.output.style === 'xml') {
  content = escapeXml(content);
}

// Markdown: Dynamic code block delimiter
const delimiter = calculateMarkdownDelimiter(files);  // Avoid conflicts
```

---

## File Inventory

### CLI Layer (cli/)

| File | Lines | Purpose |
|------|-------|---------|
| `cliRun.ts` | ~400 | Main CLI entry and argument parsing |
| `cliReport.ts` | ~200 | Output reporting |
| `cliSpinner.ts` | ~100 | Progress spinners |
| `actions/defaultAction.ts` | ~300 | Default pack action |
| `actions/remoteAction.ts` | ~250 | Remote repo packing |
| `actions/initAction.ts` | ~150 | Config initialization |
| `actions/mcpAction.ts` | ~100 | MCP server start |

### Core Layer (core/)

| File | Lines | Purpose |
|------|-------|---------|
| `packager.ts` | 241 | Main orchestration |
| `file/fileSearch.ts` | ~300 | File discovery |
| `file/fileCollect.ts` | ~150 | File collection |
| `file/fileProcess.ts` | ~200 | Content processing |
| `file/fileTreeGenerate.ts` | ~250 | Tree generation |
| `security/securityCheck.ts` | ~140 | Security orchestration |
| `security/workers/securityCheckWorker.ts` | ~200 | Security worker |
| `metrics/TokenCounter.ts` | ~100 | Token counting |
| `metrics/calculateMetrics.ts` | ~300 | Metrics calculation |
| `output/outputGenerator.ts` | ~400 | Output generation |
| `output/outputStyles/*.ts` | ~300 | Output templates |
| `git/gitDiffHandle.ts` | ~150 | Git diff handling |
| `git/gitLogHandle.ts` | ~150 | Git log handling |

### Config Layer (config/)

| File | Lines | Purpose |
|------|-------|---------|
| `configSchema.ts` | 173 | Zod schemas |
| `configLoad.ts` | ~200 | Config loading |
| `defaultIgnore.ts` | ~150 | Default patterns |

---

## Integration Points

### With AI Systems

Repomix output is optimized for:
- **Claude Code** — Via skill generation
- **Cursor** — Pack repos for context
- **Cline/Roo** — Repository analysis
- **Any LLM** — Structured codebase input

### With MCP

```json
// .mcp.json
{
  "mcpServers": {
    "repomix": {
      "command": "repomix",
      "args": ["mcp"]
    }
  }
}
```

---

## Related Files

**In This Repo:**
- None (Repomix is in a separate repository)

**External:**
- [Repomix Documentation](https://repomix.com/)
- [Zod Documentation](https://zod.dev/)
- [MCP Specification](https://modelcontextprotocol.io/)
- [Tree-sitter Documentation](https://tree-sitter.github.io/)

---

*Deep dive created: 2026-04-07*
