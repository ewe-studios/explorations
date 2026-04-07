# Utils Module — Comprehensive Deep-Dive Exploration

**Module:** `utils/`  
**Location:** `/home/darkvoid/Boxxed/@formulas/src.rust/src.llamacpp/src.ClaudOpen/claude-code-main/src/utils/`  
**Files:** 564 TypeScript/TSX files (~330 in root, ~234 in subdirectories)  
**Total Lines:** ~180,000+ lines of code  

**Primary Purpose:** Shared utility functions, helpers, and infrastructure that support all other modules in Claude Code. This is the "standard library" of the codebase — providing file operations, string utilities, JSON handling, process management, auth, settings, plugins, MCP, session management, model selection, and more.

---

## 1. Module Overview

The `utils/` module is the largest and most foundational module in Claude Code. It serves as the shared infrastructure layer that every other module depends on. Unlike domain-specific modules (tools, services, commands), utils contains reusable primitives and cross-cutting concerns.

### Architectural Principles

**Layering:**
- Utils is at the bottom of the dependency hierarchy — it should not import from higher-level modules (tools, services, commands)
- Exceptions exist for bootstrap/state.js which holds global application state
- Circular dependencies are broken via lazy requires (`require()` inside functions)

**Organization:**
- **Root files** (`utils/*.ts`): Core utilities used everywhere (file, path, json, errors, format, log, etc.)
- **Subdirectories** (`utils/*/`): Domain-specific utilities with their own internal structure
- **Shared patterns**: Consistent error handling, caching, memoization across all utilities

**Key Design Patterns:**
1. **FsOperations abstraction**: Filesystem interface for testability and platform abstraction
2. **Memoization with TTL**: Time-based cache invalidation with background refresh
3. **LRU caching**: Bounded caches to prevent memory leaks
4. **Error type guards**: `isENOENT()`, `isAbortError()`, `isFsInaccessible()`
5. **Safe parsing**: `safeParseJSON()` with BOM stripping and error caching

---

## 2. Complete File Inventory (564 Files)

### 2.1 Root Directory Files (~330 files)

| File | Lines | Key Exports | Description |
|------|-------|-------------|-------------|
| `abortController.ts` | 99 | `CombinedAbortSignal` | AbortController utilities and combined signal creation |
| `activityManager.ts` | 164 | `ActivityManager` | Session activity tracking and management |
| `advisor.ts` | 145 | `isAdvisorBlock()`, `AdvisorBlock` | Advisor message type guards |
| `agentContext.ts` | 178 | `AgentContext` | Agent context management |
| `agenticSessionSearch.ts` | 307 | `searchAgenticSessions()` | Session search functionality |
| `agentId.ts` | 99 | `AgentId`, `asAgentId()` | Agent ID type utilities |
| `agentSwarmsEnabled.ts` | 44 | `isAgentSwarmsEnabled()` | Feature gate for agent swarms |
| `analyzeContext.ts` | 1382 | `analyzeContext()` | Context analysis for prompts |
| `ansiToPng.ts` | 334 | `ansiToPng()` | ANSI terminal to PNG image conversion |
| `ansiToSvg.ts` | 272 | `ansiToSvg()` | ANSI terminal to SVG conversion |
| `apiPreconnect.ts` | 71 | `preconnect()` | API connection pre-warming |
| `api.ts` | 718 | `normalizeToolInput()`, `APIError` | API request/response utilities |
| `appleTerminalBackup.ts` | 124 | `backupAppleTerminal()` | macOS Terminal backup utilities |
| `argumentSubstitution.ts` | 145 | `substituteArguments()` | Command argument substitution |
| `array.ts` | 13 | `uniq()`, `count()` | Array utility functions |
| `asciicast.ts` | 239 | `recordAsciicast()` | Terminal recording (asciicast format) |
| `attachments.ts` | 3997 | `Attachment`, `createAttachmentMessage()` | Attachment handling for messages |
| `attribution.ts` | 393 | `AttributionState` | Code attribution tracking |
| `auth.ts` | 2002+ | `getAuthToken()`, `isAnthropicAuthEnabled()` | Authentication core (OAuth, API keys) |
| `authFileDescriptor.ts` | 196 | `getOAuthTokenFromFileDescriptor()` | FD-based auth token passing |
| `authPortable.ts` | 19 | `normalizeApiKeyForConfig()` | Cross-platform auth utilities |
| `autoModeDenials.ts` | 25 | `AutoModeDenial` | Auto-mode denial tracking |
| `autoRunIssue.tsx` | 121 | `AutoRunIssue` | Auto-run permission issues |
| `autoUpdater.ts` | 561 | `AutoUpdater` | Automatic update management |
| `aws.ts` | 74 | `checkStsCallerIdentity()` | AWS authentication utilities |
| `awsAuthStatusManager.ts` | 81 | `AwsAuthStatusManager` | AWS auth status management |
| `backgroundHousekeeping.ts` | 94 | `runBackgroundHousekeeping()` | Background cleanup tasks |
| `betas.ts` | 434 | `BetaFeature` | Beta feature management |
| `billing.ts` | 78 | `BillingType` | Billing/subscription types |
| `binaryCheck.ts` | 53 | `checkBinary()` | Binary availability checks |
| `browser.ts` | 68 | `openBrowser()` | Browser launching utilities |
| `bufferedWriter.ts` | 100 | `BufferedWriter` | Buffered file writing |
| `bundledMode.ts` | 22 | `isInBundledMode()` | Bundled/native mode detection |
| `caCerts.ts` | 115 | `loadCACertificates()` | CA certificate loading |
| `caCertsConfig.ts` | 88 | `CaCertsConfig` | CA certificate configuration |
| `cachePaths.ts` | 38 | `CACHE_PATHS` | Cache directory paths |
| `CircularBuffer.ts` | 84 | `CircularBuffer<T>` | Fixed-size circular buffer |
| `classifierApprovals.ts` | 88 | `ClassifierApproval` | Permission classifier approvals |
| `classifierApprovalsHook.ts` | 17 | `registerClassifierApprovalsHook()` | Hook for classifier approvals |
| `claudeCodeHints.ts` | 193 | `ClaudeCodeHint` | In-context hints/suggestions |
| `claudeDesktop.ts` | 151 | `ClaudeDesktopConfig` | Claude Desktop integration |
| `claudemd.ts` | 1479 | `loadClaudeMd()`, `ClaudeMdConfig` | AGENTS.md/CLAUDE.md loading |
| `cleanup.ts` | 602 | `cleanupRegistry` | Resource cleanup management |
| `cleanupRegistry.ts` | 25 | `registerCleanup()` | Cleanup callback registry |
| `cliArgs.ts` | 60 | `CliArgs` | CLI argument parsing |
| `cliHighlight.ts` | 54 | `highlightCli()` | CLI syntax highlighting |
| `codeIndexing.ts` | 206 | `indexCode()` | Code indexing for search |
| `collapseBackgroundBashNotifications.ts` | 84 | `collapseBackgroundBash()` | Bash output collapsing |
| `collapseHookSummaries.ts` | 59 | `collapseHookSummaries()` | Hook summary collapsing |
| `collapseReadSearch.ts` | 1109 | `collapseReadSearch()` | Read/search result collapsing |
| `collapseTeammateShutdowns.ts` | 55 | `collapseTeammateShutdowns()` | Teammate shutdown collapsing |
| `combinedAbortSignal.ts` | 47 | `createCombinedAbortSignal()` | Combine multiple abort signals |
| `commandLifecycle.ts` | 21 | `CommandLifecycle` | Command lifecycle hooks |
| `commitAttribution.ts` | 961 | `AttributionState` | Git commit attribution |
| `completionCache.ts` | 166 | `CompletionCache` | Completion caching |
| `concurrentSessions.ts` | 204 | `ConcurrentSessionManager` | Multi-session management |
| `config.ts` | 1817+ | `GlobalConfig`, `ProjectConfig`, `getGlobalConfig()` | Configuration loading/saving |
| `configConstants.ts` | 21 | `EDITOR_MODES`, `NOTIFICATION_CHANNELS` | Config constants |
| `context.ts` | 221 | `has1mContext()` | Context window utilities |
| `contextAnalysis.ts` | 272 | `analyzeContext()` | Context analysis |
| `contextSuggestions.ts` | 235 | `ContextSuggestion` | Context suggestions |
| `controlMessageCompat.ts` | 32 | `ControlMessage` | Control message compatibility |
| `conversationRecovery.ts` | 597 | `recoverConversation()` | Conversation recovery |
| `cron.ts` | 308 | `CronJob` | Cron scheduling |
| `cronJitterConfig.ts` | 75 | `CronJitterConfig` | Cron jitter configuration |
| `cronScheduler.ts` | 565 | `CronScheduler` | Cron job scheduler |
| `cronTasks.ts` | 458 | `CronTask` | Cron task management |
| `cronTasksLock.ts` | 195 | `CronTasksLock` | Cron task locking |
| `crossProjectResume.ts` | 76 | `resumeCrossProject()` | Cross-project session resume |
| `crypto.ts` | 13 | `hash()`, `randomBytes()` | Crypto utilities |
| `Cursor.ts` | 1530 | `Cursor` | Text cursor/selection management |
| `cwd.ts` | 32 | `getCwd()`, `setCwd()` | Current working directory |
| `debug.ts` | 268 | `logForDebugging()`, `logAntError()` | Debug logging utilities |
| `debugFilter.ts` | 157 | `DebugFilter` | Debug output filtering |
| `desktopDeepLink.ts` | 236 | `handleDesktopDeepLink()` | Desktop app deep links |
| `detectRepository.ts` | 178 | `detectRepository()` | Repository detection |
| `diagLogs.ts` | 94 | `logForDiagnosticsNoPII()` | Diagnostic logging (no PII) |
| `diff.ts` | 177 | `createDiff()` | Diff generation utilities |
| `directMemberMessage.ts` | 69 | `DirectMemberMessage` | Direct team member messages |
| `displayTags.ts` | 51 | `stripDisplayTags()` | Display tag stripping |
| `doctorContextWarnings.ts` | 265 | `DoctorContextWarnings` | Doctor warning system |
| `doctorDiagnostic.ts` | 625 | `DoctorDiagnostic` | Doctor diagnostic checks |
| `earlyInput.ts` | 191 | `EarlyInput` | Early input capture |
| `editor.ts` | 183 | `openEditor()` | External editor integration |
| `effort.ts` | 329 | `EffortEstimate` | Effort estimation |
| `embeddedTools.ts` | 29 | `hasEmbeddedSearchTools()` | Embedded tool detection |
| `env.ts` | 347 | `getGlobalClaudeFile()` | Environment utilities |
| `envDynamic.ts` | 151 | `DynamicEnvVars` | Dynamic environment variables |
| `envUtils.ts` | 183 | `isEnvTruthy()`, `getClaudeConfigHomeDir()` | Environment variable utilities |
| `envValidation.ts` | 38 | `validateEnvVar()` | Environment validation |
| `errorLogSink.ts` | 238 | `ErrorLogSink` | Error logging sink interface |
| `errors.ts` | 238 | `ClaudeError`, `isAbortError()`, `isENOENT()` | Error classes and type guards |
| `exampleCommands.ts` | 184 | `ExampleCommand` | Example command suggestions |
| `execFileNoThrow.ts` | 150 | `execFileNoThrow()` | execFile without throwing |
| `execFileNoThrowPortable.ts` | 89 | `execFileNoThrowPortable()` | Portable execFile wrapper |
| `execSyncWrapper.ts` | 38 | `execSyncWrapper()` | execSync wrapper |
| `exportRenderer.tsx` | 97 | `ExportRenderer` | Session export rendering |
| `extraUsage.ts` | 23 | `ExtraUsage` | Extra usage tracking |
| `fastMode.ts` | 532 | `FastMode` | Fast/low-latency mode |
| `file.ts` | 584 | `writeTextContent()`, `readFileSafe()`, `pathExists()` | File operations |
| `fileHistory.ts` | 1115 | `FileHistorySnapshot` | File edit history tracking |
| `fileOperationAnalytics.ts` | 71 | `trackFileOperation()` | File operation analytics |
| `fileRead.ts` | 102 | `readFileSync()`, `detectEncodingForResolvedPath()` | File reading utilities |
| `fileReadCache.ts` | 96 | `fileReadCache` | File read caching |
| `fileStateCache.ts` | 142 | `FileStateCache` | File state caching |
| `findExecutable.ts` | 17 | `findExecutable()` | Find executable in PATH |
| `fingerprint.ts` | 76 | `generateFingerprint()` | Session fingerprinting |
| `forkedAgent.ts` | 689 | `ForkedAgent` | Forked agent process management |
| `format.ts` | 200+ | `formatDuration()`, `formatFileSize()`, `formatTokens()` | Formatting utilities |
| `formatBriefTimestamp.ts` | 81 | `formatBriefTimestamp()` | Brief timestamp formatting |
| `frontmatterParser.ts` | 370 | `parseFrontmatter()` | YAML frontmatter parsing |
| `fsOperations.ts` | 770 | `FsOperations`, `safeResolvePath()` | Filesystem abstraction layer |
| `fullscreen.ts` | 202 | `Fullscreen` | Fullscreen terminal mode |
| `generatedFiles.ts` | 136 | `GeneratedFiles` | Generated file tracking |
| `generators.ts` | 88 | `all()` | Generator utilities |
| `genericProcessUtils.ts` | 184 | `GenericProcessUtils` | Process utilities |
| `getWorktreePaths.ts` | 70 | `getWorktreePaths()` | Git worktree paths |
| `getWorktreePathsPortable.ts` | 27 | `getWorktreePathsPortable()` | Portable worktree paths |
| `ghPrStatus.ts` | 106 | `GitHubPRStatus` | GitHub PR status |
| `git.ts` | 926 | `findCanonicalGitRoot()`, `getBranch()`, `gitExe()` | Git operations |
| `gitDiff.ts` | 532 | `GitDiff` | Git diff utilities |
| `gitSettings.ts` | 18 | `GitSettings` | Git configuration |
| `glob.ts` | 130 | `glob()` | Glob pattern matching via ripgrep |
| `gracefulShutdown.ts` | 529 | `gracefulShutdown()`, `registerShutdownHook()` | Graceful shutdown handling |
| `groupToolUses.ts` | 198 | `groupToolUses()` | Tool use grouping |
| `handlePromptSubmit.ts` | 610 | `handlePromptSubmit()` | Prompt submission handling |
| `hash.ts` | 46 | `hash()` | Hashing utilities |
| `headlessProfiler.ts` | 178 | `HeadlessProfiler` | Headless mode profiling |
| `heapDumpService.ts` | 303 | `HeapDumpService` | Heap dump debugging |
| `heatmap.ts` | 198 | `Heatmap` | Usage heatmap |
| `highlightMatch.tsx` | 27 | `HighlightMatch` | Match highlighting |
| `horizontalScroll.ts` | 430 | `HorizontalScroll` | Horizontal scroll handling |
| `http.ts` | 492 | `HttpClient` | HTTP client utilities |
| `hyperlink.ts` | 146 | `createHyperlink()` | Terminal hyperlink creation |
| `ide.ts` | 4658 | `IDE` | IDE integration (JetBrains, VS Code, etc.) |
| `idePathConversion.ts` | 29 | `convertIdePath()` | IDE path conversion |
| `idleTimeout.ts` | 157 | `IdleTimeout` | Session idle timeout |
| `imagePaste.ts` | 1450 | `ImagePaste` | Image paste handling |
| `imageResizer.ts` | 2669 | `compressImageBlock()`, `ImageResizer` | Image resizing/compression |
| `imageStore.ts` | 432 | `ImageStore` | Image storage |
| `imageValidation.ts` | 361 | `validateImagesForAPI()` | Image validation |
| `immediateCommand.ts` | 54 | `ImmediateCommand` | Immediate command execution |
| `ink.ts` | 29 | `Ink` | Ink rendering utilities |
| `inProcessTeammateHelpers.ts` | 298 | `InProcessTeammateHelpers` | In-process teammate helpers |
| `intl.ts` | 282 | `getRelativeTimeFormat()`, `getTimeZone()` | Internationalization |
| `iTermBackup.ts` | 160 | `backupIterm()` | iTerm backup |
| `jetbrains.ts` | 580 | `JetBrains` | JetBrains IDE integration |
| `json.ts` | 914 | `safeParseJSON()`, `parseJSONL()` | JSON parsing/utilities |
| `jsonRead.ts` | 56 | `stripBOM()` | BOM stripping for JSON |
| `keyboardShortcuts.ts` | 56 | `KeyboardShortcuts` | Keyboard shortcut handling |
| `lazySchema.ts` | 29 | `lazySchema()` | Lazy Zod schema loading |
| `listSessionsImpl.ts` | 1507 | `listSessions()` | Session listing implementation |
| `localInstaller.ts` | 481 | `LocalInstaller` | Local installation |
| `lockfile.ts` | 133 | `lockfile` | File locking utilities |
| `logoV2Utils.ts` | 983 | `LogoV2` | Logo rendering |
| `log.ts` | 502 | `logError()`, `logEvent()` | Logging utilities |
| `mailbox.ts` | 159 | `Mailbox` | Message mailbox |
| `managedEnv.ts` | 790 | `ManagedEnv` | Managed environment variables |
| `managedEnvConstants.ts` | 681 | `ManagedEnvConstants` | Managed env constants |
| `markdown.ts` | 1185 | `Markdown` | Markdown parsing/rendering |
| `markdownConfigLoader.ts` | 2131 | `MarkdownConfigLoader` | Markdown config loading |
| `mcpInstructionsDelta.ts` | 475 | `McpInstructionsDelta` | MCP instruction deltas |
| `mcpOutputStorage.ts` | 708 | `McpOutputStorage` | MCP output storage |
| `mcpValidation.ts` | 630 | `McpValidation` | MCP validation |
| `mcpWebSocketTransport.ts` | 605 | `McpWebSocketTransport` | MCP WebSocket transport |
| `memoize.ts` | 861 | `memoizeWithTTL()`, `memoizeWithLRU()` | Memoization utilities |
| `memoryFileDetection.ts` | 1021 | `MemoryFileDetection` | Memory file detection |
| `messagePredicates.ts` | 42 | `MessagePredicates` | Message type guards |
| `messageQueueManager.ts` | 1656 | `MessageQueueManager` | Message queue management |
| `messages.ts` | 4436+ | `Message`, `extractTextContent()`, `isAssistantMessage()` | Message utilities |
| `modelCost.ts` | 752 | `formatModelPricing()` | Model cost calculations |
| `modifiers.ts` | 111 | `Modifiers` | Key modifier handling |
| `mtls.ts` | 465 | `MTLS` | mTLS configuration |
| `notebook.ts` | 636 | `Notebook` | Jupyter notebook support |
| `objectGroupBy.ts` | 51 | `objectGroupBy()` | Object grouping utility |
| `pasteStore.ts` | 295 | `PasteStore` | Clipboard paste storage |
| `path.ts` | 570 | `expandPath()`, `toRelativePath()`, `sanitizePath()` | Path utilities |
| `pdf.ts` | 814 | `Pdf` | PDF processing |
| `pdfUtils.ts` | 219 | `PdfUtils` | PDF utilities |
| `peerAddress.ts` | 98 | `PeerAddress` | Peer address utilities |
| `planModeV2.ts` | 305 | `PlanModeV2` | Plan mode v2 |
| `plans.ts` | 1239 | `Plans` | Plan management |
| `platform.ts` | 380 | `getPlatform()` | Platform detection |
| `preflightChecks.tsx` | 1930 | `PreflightChecks` | Startup preflight checks |
| `privacyLevel.ts` | 188 | `PrivacyLevel` | Privacy level settings |
| `process.ts` | 68 | `writeToStdout()`, `exitWithError()` | Process utilities |
| `profilerBase.ts` | 157 | `ProfilerBase` | Profiling base class |
| `promptCategory.ts` | 150 | `PromptCategory` | Prompt categorization |
| `promptEditor.ts` | 566 | `PromptEditor` | Prompt editing |
| `promptShellExecution.ts` | 701 | `PromptShellExecution` | Shell execution for prompts |
| `proxy.ts` | 1354 | `Proxy` | Proxy configuration |
| `queryContext.ts` | 593 | `QueryContext` | Query context |
| `QueryGuard.ts` | 121 | `QueryGuard` | Query guard |
| `queryHelpers.ts` | 1973 | `QueryHelpers` | Query helpers |
| `queryProfiler.ts` | 889 | `QueryProfiler` | Query profiling |
| `queueProcessor.ts` | 317 | `QueueProcessor` | Queue processing |
| `readEditContext.ts` | 722 | `ReadEditContext` | Read/edit context |
| `readFileInRange.ts` | 1225 | `readFileInRange()` | Read file within range |
| `releaseNotes.ts` | 1181 | `ReleaseNotes` | Release notes |
| `renderOptions.ts` | 227 | `RenderOptions` | Render options |
| `ripgrep.ts` | 2118 | `ripGrep()`, `ripgrepCommand()` | Ripgrep search |
| `sanitization.ts` | 400 | `Sanitization` | Input sanitization |
| `screenshotClipboard.ts` | 370 | `ScreenshotClipboard` | Screenshot from clipboard |
| `sdkEventQueue.ts` | 407 | `SdkEventQueue` | SDK event queue |
| `semanticBoolean.ts` | 116 | `SemanticBoolean` | Semantic boolean parsing |
| `semanticNumber.ts` | 148 | `SemanticNumber` | Semantic number parsing |
| `semver.ts` | 171 | `Semver` | Semantic versioning |
| `sequential.ts` | 164 | `Sequential` | Sequential execution |
| `sessionActivity.ts` | 408 | `SessionActivity` | Session activity |
| `sessionEnvironment.ts` | 505 | `SessionEnvironment` | Session environment |
| `sessionEnvVars.ts` | 59 | `SessionEnvVars` | Session environment variables |
| `sessionFileAccessHooks.ts` | 813 | `SessionFileAccessHooks` | Session file access hooks |
| `sessionIngressAuth.ts` | 473 | `SessionIngressAuth` | Session ingress auth |
| `sessionRestore.ts` | 2040 | `restoreSessionState()` | Session state restoration |
| `sessionStart.ts` | 815 | `SessionStart` | Session start |
| `sessionState.ts` | 530 | `SessionState` | Session state |
| `sessionStorage.ts` | 18062 | `saveMessage()`, `loadTranscriptFile()` | Session storage (JSONL) |
| `sessionStoragePortable.ts` | 2544 | `readHeadAndTail()`, `SKIP_FIRST_PROMPT_PATTERN` | Portable session storage |
| `sessionTitle.ts` | 475 | `SessionTitle` | Session title generation |
| `sessionUrl.ts` | 167 | `SessionUrl` | Session URL |
| `set.ts` | 103 | `Set` | Set utilities |
| `Shell.ts` | 474 | `Shell`, `setCwd()` | Shell management |
| `ShellCommand.ts` | 465 | `ShellCommand` | Shell command execution |
| `shellConfig.ts` | 473 | `ShellConfig` | Shell configuration |
| `sideQuery.ts` | 818 | `SideQuery` | Side query |
| `sideQuestion.ts` | 613 | `SideQuestion` | Side question |
| `signal.ts` | 144 | `Signal` | Signal handling |
| `sinks.ts` | 60 | `Sinks` | Log sinks |
| `slashCommandParsing.ts` | 143 | `SlashCommandParsing` | Slash command parsing |
| `sleep.ts` | 54 | `sleep()`, `withTimeout()` | Sleep and timeout utilities |
| `sliceAnsi.ts` | 334 | `sliceAnsi()` | ANSI-aware string slicing |
| `slowOperations.ts` | 905 | `jsonStringify()`, `slowLogging` | Slow operation tracking |
| `standaloneAgent.ts` | 80 | `StandaloneAgent` | Standalone agent |
| `startupProfiler.ts` | 607 | `StartupProfiler` | Startup profiling |
| `staticRender.tsx` | 1223 | `StaticRender` | Static rendering |
| `stats.ts` | 3379 | `Stats` | Statistics |
| `statsCache.ts` | 1389 | `StatsCache` | Statistics caching |
| `status.tsx` | 4863 | `Status` | Status display |
| `statusNoticeDefinitions.tsx` | 3063 | `StatusNoticeDefinitions` | Status notice definitions |
| `statusNoticeHelpers.ts` | 67 | `StatusNoticeHelpers` | Status notice helpers |
| `stream.ts` | 192 | `Stream` | Stream utilities |
| `streamJsonStdoutGuard.ts` | 398 | `StreamJsonStdoutGuard` | JSON stdout guard |
| `streamlinedTransform.ts` | 594 | `StreamlinedTransform` | Streamlined transform |
| `stringUtils.ts` | 659 | `escapeRegExp()`, `capitalize()`, `plural()` | String utilities |
| `subprocessEnv.ts` | 397 | `SubprocessEnv` | Subprocess environment |
| `swarm/` | (see subdirectory) | | Agent swarm utilities |
| `systemDirectories.ts` | 212 | `SystemDirectories` | System directory paths |
| `systemPrompt.ts` | 486 | `SystemPrompt` | System prompt generation |
| `systemPromptType.ts` | 38 | `SystemPromptType` | System prompt types |
| `systemTheme.ts` | 423 | `SystemTheme` | System theme detection |
| `taggedId.ts` | 157 | `TaggedId` | Tagged ID utilities |
| `tasks.ts` | 2635 | `Tasks` | Task management |
| `teamDiscovery.ts` | 232 | `TeamDiscovery` | Team discovery |
| `teammate.ts` | 920 | `Teammate` | Teammate management |
| `teammateContext.ts` | 316 | `TeammateContext` | Teammate context |
| `teammateMailbox.ts` | 3342 | `TeammateMailbox` | Teammate mailbox |
| `teamMemoryOps.ts` | 247 | `TeamMemoryOps` | Team memory operations |
| `telemetryAttributes.ts` | 209 | `TelemetryAttributes` | Telemetry attributes |
| `teleport.tsx` | 17577 | `Teleport` | Teleport feature |
| `tempfile.ts` | 117 | `Tempfile` | Temporary file utilities |
| `terminal.ts` | 437 | `Terminal` | Terminal utilities |
| `terminalPanel.ts` | 601 | `TerminalPanel` | Terminal panel |
| `textHighlighting.ts` | 454 | `TextHighlighting` | Text highlighting |
| `theme.ts` | 2683 | `Theme` | Theme management |
| `thinking.ts` | 551 | `Thinking` | Thinking block handling |
| `timeouts.ts` | 141 | `Timeouts` | Timeout management |
| `tmuxSocket.ts` | 1369 | `TmuxSocket` | tmux socket communication |
| `tokenBudget.ts` | 267 | `TokenBudget` | Token budget management |
| `tokens.ts` | 960 | `Tokens` | Token counting |
| `toolErrors.ts` | 401 | `ToolErrors` | Tool error handling |
| `toolPool.ts` | 313 | `ToolPool` | Tool pooling |
| `toolResultStorage.ts` | 3838 | `ToolResultStorage` | Tool result storage |
| `toolSchemaCache.ts` | 106 | `ToolSchemaCache` | Tool schema caching |
| `toolSearch.ts` | 2660 | `ToolSearch` | Tool search |
| `transcriptSearch.ts` | 803 | `TranscriptSearch` | Transcript search |
| `treeify.ts` | 503 | `Treeify` | Tree formatting |
| `truncate.ts` | 571 | `Truncate` | Text truncation |
| `unaryLogging.ts` | 125 | `UnaryLogging` | Unary logging |
| `undercover.ts` | 368 | `Undercover` | Undercover mode |
| `userAgent.ts` | 28 | `UserAgent` | User agent |
| `userPromptKeywords.ts` | 92 | `UserPromptKeywords` | User prompt keywords |
| `user.ts` | 571 | `User` | User utilities |
| `uuid.ts` | 88 | `validateUuid()` | UUID utilities |
| `warningHandler.ts` | 448 | `WarningHandler` | Warning handling |
| `which.ts` | 239 | `Which` | Which command |
| `windowsPaths.ts` | 600 | `WindowsPaths` | Windows path handling |
| `withResolvers.ts` | 44 | `WithResolvers` | Promise withResolvers |
| `words.ts` | 1096 | `Words` | Word utilities |
| `workloadContext.ts` | 233 | `WorkloadContext` | Workload context |
| `worktree.ts` | 4999 | `Worktree` | Git worktree management |
| `worktreeModeEnabled.ts` | 41 | `WorktreeModeEnabled` | Worktree mode detection |
| `xdg.ts` | 187 | `Xdg` | XDG directories |
| `xml.ts` | 62 | `Xml` | XML utilities |
| `yaml.ts` | 52 | `Yaml` | YAML parsing |
| `zodToJsonSchema.ts` | 76 | `ZodToJsonSchema` | Zod to JSON schema |

### 2.2 Subdirectory Files

#### `bash/` (14 files, ~4500 lines)
| File | Lines | Description |
|------|-------|-------------|
| `ast.ts` | 2679 | Bash AST parsing |
| `bashParser.ts` | 4436 | Bash command parser |
| `bashPipeCommand.ts` | 294 | Bash pipe command handling |
| `commands.ts` | 1339 | Bash command registry |
| `heredoc.ts` | 733 | Heredoc parsing |
| `ParsedCommand.ts` | 318 | Parsed command class |
| `parser.ts` | 230 | Parser utilities |
| `prefix.ts` | 204 | Command prefix handling |
| `registry.ts` | 53 | Command registry |
| `shellCompletion.ts` | 259 | Shell completion |
| `shellPrefix.ts` | 28 | Shell prefix |
| `shellQuote.ts` | 304 | Shell quoting |
| `shellQuoting.ts` | 128 | Shell quoting utilities |
| `ShellSnapshot.ts` | 582 | Shell snapshot |
| `treeSitterAnalysis.ts` | 506 | Tree-sitter analysis |
| `specs/*.ts` | 6 files | Command specifications |

#### `computerUse/` (15 files, ~2000 lines)
| File | Lines | Description |
|------|-------|-------------|
| `appNames.ts` | 196 | Application names for computer use |
| `cleanup.ts` | 86 | Computer use cleanup |
| `common.ts` | 61 | Common utilities |
| `computerUseLock.ts` | 215 | Computer use locking |
| `drainRunLoop.ts` | 79 | Drain run loop |
| `escHotkey.ts` | 54 | ESC hotkey handling |
| `executor.ts` | 658 | Computer use executor |
| `gates.ts` | 62 | Feature gates |
| `hostAdapter.ts` | 27 | Host adapter |
| `inputLoader.ts` | 30 | Input loader |
| `mcpServer.ts` | 41 | MCP server |
| `setup.ts` | 20 | Setup utilities |
| `swiftLoader.ts` | 23 | Swift loader (macOS) |
| `toolRendering.tsx` | 925 | Tool rendering |
| `wrapper.tsx` | 335 | Computer use wrapper |

#### `hooks/` (17 files, ~2000 lines)
| File | Lines | Description |
|------|-------|-------------|
| `apiQueryHookHelper.ts` | 438 | API query hook helper |
| `AsyncHookRegistry.ts` | 309 | Async hook registry |
| `execAgentHook.ts` | 1248 | Execute agent hook |
| `execHttpHook.ts` | 887 | Execute HTTP hook |
| `execPromptHook.ts` | 682 | Execute prompt hook |
| `fileChangedWatcher.ts` | 530 | File change watcher |
| `hookEvents.ts` | 449 | Hook events |
| `hookHelpers.ts` | 252 | Hook helpers |
| `hooksConfigManager.ts` | 1749 | Hooks config manager |
| `hooksConfigSnapshot.ts` | 506 | Hooks config snapshot |
| `hooksSettings.ts` | 850 | Hooks settings |
| `postSamplingHooks.ts` | 199 | Post-sampling hooks |
| `registerFrontmatterHooks.ts` | 227 | Frontmatter hook registration |
| `registerSkillHooks.ts` | 205 | Skill hook registration |
| `sessionHooks.ts` | 1213 | Session hooks |
| `skillImprovement.ts` | 836 | Skill improvement |
| `ssrfGuard.ts` | 873 | SSRF guard |

#### `mcp/` (3 files, ~1400 lines)
| File | Lines | Description |
|------|-------|-------------|
| `dateTimeParser.ts` | 432 | Date/time parsing for MCP |
| `elicitationValidation.ts` | 938 | MCP elicitation validation |

#### `memory/` (2 files, ~600 lines)
| File | Lines | Description |
|------|-------|-------------|
| `types.ts` | 26 | Memory types |
| `versions.ts` | 30 | Memory versions |

#### `messages/` (2 files, ~1300 lines)
| File | Lines | Description |
|------|-------|-------------|
| `mappers.ts` | 903 | Message mappers |
| `systemInit.ts` | 374 | System initialization messages |

#### `model/` (16 files, ~4000 lines)
| File | Lines | Description |
|------|-------|-------------|
| `agent.ts` | 557 | Agent model utilities |
| `aliases.ts` | 79 | Model aliases |
| `antModels.ts` | 179 | Ant-specific models |
| `bedrock.ts` | 919 | AWS Bedrock models |
| `check1mAccess.ts` | 221 | 1M context access check |
| `configs.ts` | 428 | Model configurations |
| `contextWindowUpgradeCheck.ts` | 128 | Context window upgrade check |
| `deprecation.ts` | 253 | Model deprecation |
| `modelAllowlist.ts` | 603 | Model allowlist |
| `modelCapabilities.ts` | 409 | Model capabilities |
| `modelOptions.ts` | 1833 | Model options |
| `modelStrings.ts` | 523 | Model string constants |
| `modelSupportOverrides.ts` | 153 | Model support overrides |
| `model.ts` | 2140 | Core model utilities |
| `providers.ts` | 134 | Model providers |
| `validateModel.ts` | 463 | Model validation |

#### `nativeInstaller/` (5 files, ~9000 lines)
| File | Lines | Description |
|------|-------|-------------|
| `download.ts` | 1519 | Native installer download |
| `index.ts` | 45 | Index exports |
| `installer.ts` | 5475 | Native installer |
| `packageManagers.ts` | 896 | Package manager detection |
| `pidLock.ts` | 1193 | PID-based locking |

#### `permissions/` (22 files, ~28000 lines)
| File | Lines | Description |
|------|-------|-------------|
| `autoModeState.ts` | 109 | Auto-mode state |
| `bashClassifier.ts` | 144 | Bash command classifier |
| `bypassPermissionsKillswitch.ts` | 458 | Permissions bypass killswitch |
| `classifierDecision.ts` | 458 | Classifier decision |
| `classifierShared.ts` | 117 | Classifier shared |
| `dangerousPatterns.ts` | 247 | Dangerous patterns |
| `denialTracking.ts` | 110 | Denial tracking |
| `filesystem.ts` | 6225 | Filesystem permissions |
| `getNextPermissionMode.ts` | 330 | Get next permission mode |
| `pathValidation.ts` | 1624 | Path validation |
| `permissionExplainer.ts` | 760 | Permission explainer |
| `PermissionMode.ts` | 348 | Permission mode |
| `PermissionPromptToolResultSchema.ts` | 410 | Permission prompt schema |
| `PermissionResult.ts` | 87 | Permission result |
| `permissionRuleParser.ts` | 727 | Permission rule parser |
| `PermissionRule.ts` | 117 | Permission rule |
| `permissionSetup.ts` | 5343 | Permission setup |
| `permissions.ts` | 5219 | Core permissions |
| `permissionsLoader.ts` | 874 | Permissions loader |
| `PermissionUpdate.ts` | 1191 | Permission updates |
| `PermissionUpdateSchema.ts` | 240 | Permission update schema |
| `shadowedRuleDetection.ts` | 805 | Shadowed rule detection |
| `shellRuleMatching.ts` | 640 | Shell rule matching |
| `yoloClassifier.ts` | 5216 | YOLO classifier |

#### `plugins/` (40 files, ~50000 lines)
| File | Lines | Description |
|------|-------|-------------|
| `addDirPluginSettings.ts` | 232 | Add-dir plugin settings |
| `cacheUtils.ts` | 665 | Plugin cache utilities |
| `dependencyResolver.ts` | 1167 | Plugin dependency resolver |
| `fetchTelemetry.ts` | 492 | Plugin fetch telemetry |
| `gitAvailability.ts` | 227 | Git availability check |
| `headlessPluginInstall.ts` | 677 | Headless plugin install |
| `hintRecommendation.ts` | 543 | Plugin hint recommendation |
| `installCounts.ts` | 831 | Plugin install counts |
| `installedPluginsManager.ts` | 4141 | Installed plugins manager |
| `loadPluginAgents.ts` | 1248 | Load plugin agents |
| `loadPluginCommands.ts` | 3054 | Load plugin commands |
| `loadPluginHooks.ts` | 1006 | Load plugin hooks |
| `loadPluginOutputStyles.ts` | 567 | Load plugin output styles |
| `lspPluginIntegration.ts` | 1241 | LSP plugin integration |
| `lspRecommendation.ts` | 1069 | LSP recommendation |
| `managedPlugins.ts` | 87 | Managed plugins |
| `marketplaceHelpers.ts` | 1821 | Marketplace helpers |
| `marketplaceManager.ts` | 9327 | Marketplace manager |
| `mcpbHandler.ts` | 3128 | MCPB handler |
| `mcpPluginIntegration.ts` | 2011 | MCP plugin integration |
| `officialMarketplace.ts` | 83 | Official marketplace |
| `officialMarketplaceGcs.ts` | 933 | Official marketplace GCS |
| `officialMarketplaceStartupCheck.ts` | 1519 | Official marketplace startup check |
| `orphanedPluginFilter.ts` | 398 | Orphaned plugin filter |
| `parseMarketplaceInput.ts` | 607 | Parse marketplace input |
| `performStartupChecks.tsx` | 959 | Plugin startup checks |
| `pluginAutoupdate.ts` | 947 | Plugin autoupdate |
| `pluginBlocklist.ts` | 436 | Plugin blocklist |
| `pluginDirectories.ts` | 666 | Plugin directories |
| `pluginFlagging.ts` | 562 | Plugin flagging |
| `pluginIdentifier.ts` | 392 | Plugin identifier |
| `pluginInstallationHelpers.ts` | 2062 | Plugin installation helpers |
| `pluginLoader.ts` | 11026 | Plugin loader |
| `pluginOptionsStorage.ts` | 1530 | Plugin options storage |
| `pluginPolicy.ts` | 82 | Plugin policy |
| `pluginStartupCheck.ts` | 1109 | Plugin startup check |
| `pluginVersioning.ts` | 534 | Plugin versioning |
| `reconciler.ts` | 827 | Plugin reconciler |
| `refresh.ts` | 853 | Plugin refresh |
| `schemas.ts` | 5891 | Plugin schemas |
| `validatePlugin.ts` | 2836 | Plugin validation |
| `walkPluginMarkdown.ts` | 222 | Walk plugin markdown |
| `zipCacheAdapters.ts` | 531 | Zip cache adapters |
| `zipCache.ts` | 1316 | Zip cache |

#### `powershell/` (3 files, ~8500 lines)
| File | Lines | Description |
|------|-------|-------------|
| `dangerousCmdlets.ts` | 615 | Dangerous PowerShell cmdlets |
| `parser.ts` | 6664 | PowerShell parser |
| `staticPrefix.ts` | 1227 | PowerShell static prefix |

#### `processUserInput/` (4 files, ~19000 lines)
| File | Lines | Description |
|------|-------|-------------|
| `processBashCommand.tsx` | 2227 | Process bash command |
| `processSlashCommand.tsx` | 14480 | Process slash command |
| `processTextPrompt.ts` | 327 | Process text prompt |
| `processUserInput.ts` | 1950 | Process user input |

#### `secureStorage/` (6 files, ~2800 lines)
| File | Lines | Description |
|------|-------|-------------|
| `fallbackStorage.ts` | 236 | Fallback storage |
| `index.ts` | 55 | Index exports |
| `keychainPrefetch.ts` | 475 | Keychain prefetch |
| `macOsKeychainHelpers.ts` | 579 | macOS Keychain helpers |
| `macOsKeychainStorage.ts` | 827 | macOS Keychain storage |
| `plainTextStorage.ts` | 243 | Plain text storage |

#### `settings/` (16 files, ~14000 lines)
| File | Lines | Description |
|------|-------|-------------|
| `allErrors.ts` | 125 | All settings errors |
| `applySettingsChange.ts` | 365 | Apply settings change |
| `changeDetector.ts` | 1638 | Settings change detector |
| `constants.ts` | 562 | Settings constants |
| `internalWrites.ts` | 109 | Internal writes |
| `managedPath.ts` | 109 | Managed settings path |
| `mdm/settings.ts` | - | MDM settings |
| `permissionValidation.ts` | 865 | Permission validation |
| `pluginOnlyPolicy.ts` | 240 | Plugin-only policy |
| `schemaOutput.ts` | 31 | Schema output |
| `settings.ts` | 3218 | Settings loading |
| `settingsCache.ts` | 241 | Settings caching |
| `toolValidationConfig.ts` | 310 | Tool validation config |
| `types.ts` | 4278 | Settings types |
| `validateEditTool.ts` | 169 | Edit tool validation |
| `validation.ts` | 795 | Settings validation |
| `validationTips.ts` | 546 | Validation tips |

#### `shell/` (9 files, ~10000 lines)
| File | Lines | Description |
|------|-------|-------------|
| `bashProvider.ts` | 1100 | Bash provider |
| `outputLimits.ts` | 41 | Output limits |
| `powershellDetection.ts` | 371 | PowerShell detection |
| `powershellProvider.ts` | 577 | PowerShell provider |
| `prefix.ts` | 1121 | Shell prefix |
| `readOnlyCommandValidation.ts` | 6829 | Read-only command validation |
| `resolveDefaultShell.ts` | 49 | Resolve default shell |
| `shellProvider.ts` | 95 | Shell provider |
| `shellToolUtils.ts` | 103 | Shell tool utilities |
| `specPrefix.ts` | 790 | Spec prefix |

#### `suggestions/` (5 files, ~3600 lines)
| File | Lines | Description |
|------|-------|-------------|
| `commandSuggestions.ts` | 1855 | Command suggestions |
| `directoryCompletion.ts` | 709 | Directory completion |
| `shellHistoryCompletion.ts` | 345 | Shell history completion |
| `skillUsageTracking.ts` | 194 | Skill usage tracking |
| `slackChannelSuggestions.ts` | 639 | Slack channel suggestions |

#### `swarm/` (14 files, ~17000 lines)
| File | Lines | Description |
|------|-------|-------------|
| `backends/` | - | Swarm backends |
| `constants.ts` | 133 | Swarm constants |
| `inProcessRunner.ts` | 5356 | In-process runner |
| `It2SetupPrompt.tsx` | 4272 | IT2 setup prompt |
| `leaderPermissionBridge.ts` | 173 | Leader permission bridge |
| `permissionSync.ts` | 2647 | Permission synchronization |
| `reconnection.ts` | 340 | Reconnection |
| `spawnInProcess.ts` | 1024 | Spawn in-process |
| `spawnUtils.ts` | 520 | Spawn utilities |
| `teamHelpers.ts` | 2138 | Team helpers |
| `teammateInit.ts` | 428 | Teammate initialization |
| `teammateLayoutManager.ts` | 326 | Teammate layout manager |
| `teammateModel.ts` | 46 | Teammate model |
| `teammatePromptAddendum.ts` | 77 | Teammate prompt addendum |

#### `task/` (5 files, ~3800 lines)
| File | Lines | Description |
|------|-------|-------------|
| `diskOutput.ts` | 1356 | Disk output |
| `framework.ts` | 986 | Task framework |
| `outputFormatting.ts` | 118 | Output formatting |
| `sdkProgress.ts` | 115 | SDK progress |
| `TaskOutput.ts` | 1247 | Task output |

#### `telemetry/` (9 files, ~12000 lines)
| File | Lines | Description |
|------|-------|-------------|
| `betaSessionTracing.ts` | 1584 | Beta session tracing |
| `bigqueryExporter.ts` | 780 | BigQuery exporter |
| `events.ts` | 228 | Telemetry events |
| `instrumentation.ts` | 2674 | Telemetry instrumentation |
| `logger.ts` | 74 | Telemetry logger |
| `perfettoTracing.ts` | 2979 | Perfetto tracing |
| `pluginTelemetry.ts` | 1049 | Plugin telemetry |
| `sessionTracing.ts` | 2799 | Session tracing |
| `skillLoadedEvent.ts` | 142 | Skill loaded event |

#### `teleport/` (4 files, ~2900 lines)
| File | Lines | Description |
|------|-------|-------------|
| `api.ts` | 1332 | Teleport API |
| `environmentSelection.ts` | 269 | Environment selection |
| `environments.ts` | 347 | Environments |
| `gitBundle.ts` | 982 | Git bundle |

#### `todo/` (1 file)
| File | Lines | Description |
|------|-------|-------------|
| `types.ts` | 60 | Todo types |

#### `ultraplan/` (2 files, ~1700 lines)
| File | Lines | Description |
|------|-------|-------------|
| `ccrSession.ts` | 1298 | CCR session |
| `keyword.ts` | 469 | Keyword |

---

## 3. Utility Categories by Function

### 3.1 File/Path Utilities

**Core Files:** `file.ts`, `path.ts`, `fsOperations.ts`, `glob.ts`, `fileRead.ts`, `fileReadCache.ts`

**Key Functions:**
```typescript
// file.ts
export function pathExists(path: string): Promise<boolean>
export function readFileSafe(filepath: string): string | null
export function getFileModificationTime(filePath: string): number
export function writeTextContent(filePath: string, content: string, ...): void
export function detectFileEncoding(filePath: string): BufferEncoding
export function detectLineEndings(filePath: string): LineEndingType

// path.ts
export function expandPath(path: string, baseDir?: string): string
export function toRelativePath(absolutePath: string): string
export function getDirectoryForPath(path: string): string
export function containsPathTraversal(path: string): boolean
export function sanitizePath(path: string): string
export function normalizePathForConfigKey(path: string): string

// fsOperations.ts
export type FsOperations = { /* Node.js fs abstraction */ }
export function safeResolvePath(fs: FsOperations, filePath: string): {
  resolvedPath: string
  isSymlink: boolean
  isCanonical: boolean
}
export function isDuplicatePath(fs: FsOperations, filePath: string, loadedPaths: Set<string>): boolean
export function getFsImplementation(): FsOperations

// glob.ts
export function glob(filePattern: string, cwd: string, { limit, offset }, abortSignal, toolPermissionContext): Promise<{ files: string[], truncated: boolean }>
```

**Architecture:**
- `FsOperations` interface provides testability and platform abstraction
- `safeResolvePath()` handles symlinks, UNC paths, FIFOs, and permission errors
- `expandPath()` handles tilde expansion, POSIX-to-Windows conversion, null byte rejection
- `glob()` uses ripgrep for memory-efficient file matching with .gitignore support

### 3.2 String/Text Utilities

**Core Files:** `stringUtils.ts`, `format.ts`, `truncate.ts`, `sliceAnsi.ts`, `words.ts`, `frontmatterParser.ts`

**Key Functions:**
```typescript
// stringUtils.ts
export function escapeRegExp(str: string): string
export function capitalize(str: string): string
export function plural(n: number, word: string, pluralWord?: string): string
export function firstLineOf(s: string): string
export function countCharInString(str: { indexOf(...) }, char: string): number
export function normalizeFullWidthDigits(input: string): string
export function normalizeFullWidthSpace(input: string): string
export function safeJoinLines(lines: string[], delimiter?: string, maxSize?: number): string

// format.ts
export function formatFileSize(sizeInBytes: number): string
export function formatSecondsShort(ms: number): string
export function formatDuration(ms: number, options?: { hideTrailingZeros?: boolean, mostSignificantOnly?: boolean }): string
export function formatNumber(number: number): string
export function formatTokens(count: number): string
export function formatRelativeTime(date: Date, options?: { style?: 'long'|'short'|'narrow', numeric?: 'always'|'auto' }): string

// truncate.ts
export function truncateWithWidth(str: string, maxWidth: number): string
export function truncateMiddle(str: string, maxLength: number): string

// frontmatterParser.ts
export function parseFrontmatter(content: string): { frontmatter: Record<string, unknown>, content: string }
```

### 3.3 JSON/Data Utilities

**Core Files:** `json.ts`, `jsonRead.ts`, `yaml.ts`, `xml.ts`, `treeify.ts`

**Key Functions:**
```typescript
// json.ts
export function safeParseJSON(json: string | null | undefined, shouldLogError?: boolean): unknown
export function safeParseJSONC(json: string | null | undefined): unknown  // JSON with comments
export function parseJSONL<T>(content: string): T[]  // JSON Lines
export function addToArrayInJsonFile(filePath: string, newItem: unknown): void

// jsonRead.ts
export function stripBOM(str: string): string

// treeify.ts
export function treeify(obj: Record<string, unknown>, replacer?: (key: string, value: unknown) => unknown): string
```

**Performance Notes:**
- `safeParseJSON()` uses LRU cache (50 entries, 8KB max key size) with identity-guarded stale refresh
- BOM stripping for cross-platform compatibility
- JSONL parsing with Bun native acceleration when available

### 3.4 Process/Execution Utilities

**Core Files:** `process.ts`, `Shell.ts`, `ShellCommand.ts`, `genericProcessUtils.ts`, `execFileNoThrow.ts`

**Key Functions:**
```typescript
// process.ts
export function registerProcessOutputErrorHandlers(): void
export function writeToStdout(data: string): void
export function writeToStderr(data: string): void
export function exitWithError(message: string): never
export function peekForStdinData(stream: NodeJS.EventEmitter, ms: number): Promise<boolean>

// Shell.ts
export class Shell { /* Shell session management */ }
export function setCwd(path: string): void
export function getCwd(): string

// ShellCommand.ts
export class ShellCommand { /* Command execution with streaming output */ }
export function wrapSpawn(spawnFn: typeof spawn): typeof spawn

// execFileNoThrow.ts
export function execFileNoThrow(file: string, args?: string[], options?: ExecFileOptions): Promise<{ stdout: string, stderr: string, code: number | null }>
```

### 3.5 Auth Utilities

**Core Files:** `auth.ts`, `authFileDescriptor.ts`, `authPortable.ts`

**Key Functions:**
```typescript
// auth.ts
export function isAnthropicAuthEnabled(): boolean
export function getAuthTokenSource(): { source: string, hasToken: boolean }
export function getAnthropicApiKeyWithSource(options?: SkipOptions): { apiKey: string | null, source: string }
export function getClaudeAIOAuthTokens(): OAuthTokens | null
export function clearAuthTokens(): void
export function isClaudeAISubscriber(): boolean
export function isMaxSubscriber(): boolean
export function isProSubscriber(): boolean
export function getSubscriptionType(): SubscriptionType | null
```

**Auth Sources (precedence order):**
1. `CLAUDE_CODE_OAUTH_TOKEN` (managed OAuth context)
2. `CLAUDE_CODE_OAUTH_TOKEN_FILE_DESCRIPTOR` (FD-based token passing)
3. `CCR_OAUTH_TOKEN_FILE` (CCR disk fallback)
4. `apiKeyHelper` (user-configured command)
5. `ANTHROPIC_AUTH_TOKEN` (legacy env var)
6. Keychain/secure storage

### 3.6 Settings Utilities

**Core Files:** `settings/settings.ts`, `settings/types.ts`, `settings/validation.ts`, `config.ts`

**Key Functions:**
```typescript
// settings/settings.ts
export function getSettings_DEPRECATED(): SettingsJson | null
export function getSettingsForSource(source: SettingSource): SettingsWithErrors
export function updateSettingsForSource(source: EditableSettingSource, update: (prev: SettingsJson) => SettingsJson): void
export function parseSettingsFile(path: string): { settings: SettingsJson | null, errors: ValidationError[] }
export function loadManagedFileSettings(): { settings: SettingsJson | null, errors: ValidationError[] }

// config.ts
export function getGlobalConfig(): GlobalConfig
export function saveGlobalConfig(config: GlobalConfig): void
export function getProjectConfig(projectPath: string): ProjectConfig
export function saveProjectConfig(projectPath: string, config: ProjectConfig): void
```

**Settings Sources (precedence order):**
1. Session settings (SDK/flags)
2. Plugin settings
3. Managed settings (MDM/HKCU/file drop-ins)
4. Project settings (.claude/settings.json)
5. Global settings (~/.claude/settings.json)

### 3.7 Plugin Utilities

**Core Files:** `plugins/pluginLoader.ts`, `plugins/marketplaceManager.ts`, `plugins/schemas.ts`, `plugins/installedPluginsManager.ts`

**Key Functions:**
```typescript
// pluginLoader.ts
export function loadAllPlugins(options: LoadOptions): Promise<PluginLoadResult>
export function getPluginCachePath(): string
export function getVersionedCachePath(pluginId: string, version: string): string
export function probeSeedCache(pluginId: string, version: string): Promise<string | null>

// marketplaceManager.ts
export function getMarketplacePlugins(marketplace: string): Promise<PluginMarketplaceEntry[]>
export function getPluginById(pluginId: string): Promise<PluginMarketplaceEntry | null>
export function loadKnownMarketplacesConfigSafe(): KnownMarketplacesConfig
```

**Plugin Cache Structure:**
```
~/.claude/plugins/
├── cache/
│   └── {marketplace}/{plugin}/{version}/
│       ├── plugin.json
│       ├── commands/
│       ├── agents/
│       └── hooks/
└── (symlinks to active versions)
```

### 3.8 MCP Utilities

**Core Files:** `mcpValidation.ts`, `mcpOutputStorage.ts`, `mcpInstructionsDelta.ts`, `mcpWebSocketTransport.ts`, `mcp/`

**Key Functions:**
```typescript
// mcpValidation.ts
export function getMaxMcpOutputTokens(): number
export function getContentSizeEstimate(content: MCPToolResult): number
export function truncateMcpContent(content: MCPToolResult, abortSignal: AbortSignal): Promise<MCPToolResult>

// mcpOutputStorage.ts
export class McpOutputStorage { /* Store/retrieve MCP tool results */ }

// mcp/elicitationValidation.ts
export function validateElicitationRequest(request: unknown): ElicitationRequest
export function validateElicitationResponse(response: unknown): ElicitationResponse
```

### 3.9 Session Utilities

**Core Files:** `sessionStorage.ts`, `sessionStoragePortable.ts`, `sessionRestore.ts`, `sessionState.ts`, `sessionEnvironment.ts`

**Key Functions:**
```typescript
// sessionStorage.ts
export function saveMessage(message: Message, options?: SaveOptions): void
export function loadTranscriptFile(sessionId: string): Promise<Message[]>
export function getTranscriptPathForSession(sessionId: string): string
export function adoptResumedSessionFile(sessionId: string): void
export function recordContentReplacement(record: ContentReplacementRecord): void

// sessionRestore.ts
export function restoreSessionStateFromLog(result: ResumeResult, setAppState: (f: AppState) => void): void
export function computeRestoredAttributionState(result: ResumeResult): AttributionState | undefined
export function restoreAgentFromSession(agentName: string | undefined, agentColor: string | undefined, cliAgentOverride?: string): { definition: AgentDefinition, agentType: string } | undefined

// sessionStoragePortable.ts
export function readHeadAndTail(filePath: string, headBytes: number, tailBytes: number): { head: string, tail: string }
export function readTranscriptForLoad(filePath: string, maxBytes?: number): Entry[]
export const SKIP_FIRST_PROMPT_PATTERN: RegExp
```

**Session Storage Format (JSONL):**
```jsonl
{"type":"user","uuid":"...","parentUuid":null,"message":{"content":"hello"}}
{"type":"assistant","uuid":"...","parentUuid":"...","message":{"content":"hi","content":[{"type":"text","text":"..."}]}}
{"type":"system","dataType":"compact_boundary","direction":"start","model":"..."}
```

### 3.10 Model Utilities

**Core Files:** `model/model.ts`, `model/modelOptions.ts`, `model/modelStrings.ts`, `model/providers.ts`, `model/modelAllowlist.ts`

**Key Functions:**
```typescript
// model/model.ts
export function parseUserSpecifiedModel(model: string): ModelName
export function getDefaultOpusModel(): ModelName
export function getDefaultSonnetModel(): ModelName
export function getDefaultHaikuModel(): ModelName
export function getDefaultMainLoopModelSetting(): ModelName

// model/providers.ts
export function getAPIProvider(): 'firstParty' | 'bedrock' | 'vertex' | 'foundry'

// model/modelAllowlist.ts
export function isModelAllowed(model: string): boolean
export function getAvailableModels(): ModelOption[]
```

**Model Selection Priority:**
1. `/model` command override
2. `--model` CLI flag
3. `ANTHROPIC_MODEL` env var
4. Settings `model` key
5. Built-in default (Opus for Max, Sonnet for others)

### 3.11 Other Utilities

| Category | Files | Description |
|----------|-------|-------------|
| **Error Handling** | `errors.ts`, `errorLogSink.ts` | Error classes, type guards, logging |
| **Logging** | `log.ts`, `debug.ts`, `diagLogs.ts` | Debug, error, diagnostic logging |
| **Caching** | `memoize.ts`, `completionCache.ts`, `statsCache.ts` | TTL memoization, LRU caching |
| **Git** | `git.ts`, `gitDiff.ts`, `git/` | Git operations, diff generation |
| **Hooks** | `hooks.ts`, `hooks/` | Hook execution, registry, events |
| **Messages** | `messages.ts`, `messages/` | Message utilities, type guards |
| **Permissions** | `permissions/` | Permission rules, classifiers, validation |
| **Telemetry** | `telemetry/` | Tracing, events, exporters |
| **Image** | `imageResizer.ts`, `imageValidation.ts`, `imageStore.ts` | Image compression, validation |
| **Diff** | `diff.ts`, `gitDiff.ts` | Diff generation and rendering |

---

## 4. Line-by-Line Analysis of Critical Utilities

### 4.1 `memoize.ts` - Caching Infrastructure

**Purpose:** Provides TTL-based and LRU memoization with background refresh.

**Key Implementation:**
```typescript
// Lines 40-107: memoizeWithTTL
export function memoizeWithTTL<Args extends unknown[], Result>(
  f: (...args: Args) => Result,
  cacheLifetimeMs: number = 5 * 60 * 1000,
): MemoizedFunction<Args, Result> {
  const cache = new Map<string, CacheEntry<Result>>()
  
  const memoized = (...args: Args): Result => {
    const key = jsonStringify(args)
    const cached = cache.get(key)
    const now = Date.now()
    
    if (!cached) {
      const value = f(...args)
      cache.set(key, { value, timestamp: now, refreshing: false })
      return value
    }
    
    // Stale-while-revalidate pattern
    if (now - cached.timestamp > cacheLifetimeMs && !cached.refreshing) {
      cached.refreshing = true
      Promise.resolve()
        .then(() => {
          const newValue = f(...args)
          if (cache.get(key) === cached) {
            cache.set(key, { value: newValue, timestamp: Date.now(), refreshing: false })
          }
        })
        .catch(e => {
          logError(e)
          if (cache.get(key) === cached) {
            cache.delete(key)
          }
        })
      return cached.value
    }
    
    return cached.value
  }
}
```

**Key Patterns:**
1. **Identity guard**: `if (cache.get(key) === cached)` prevents stale refresh from overwriting newer entries after `cache.clear()`
2. **Stale-while-revalidate**: Returns stale cache immediately, refreshes in background
3. **Deduplication**: `refreshing` flag prevents concurrent refreshes for same key
4. **LRU variant**: `memoizeWithLRU()` uses `LRUCache` from npm with bounded size

### 4.2 `errors.ts` - Error Type Guards

**Purpose:** Centralized error classes and type guards for safe error handling.

```typescript
// Lines 27-33: isAbortError
export function isAbortError(e: unknown): boolean {
  return (
    e instanceof AbortError ||
    e instanceof APIUserAbortError ||
    (e instanceof Error && e.name === 'AbortError')
  )
}

// Lines 139-141: isENOENT
export function isENOENT(e: unknown): boolean {
  return getErrnoCode(e) === 'ENOENT'
}

// Lines 128-133: getErrnoCode
export function getErrnoCode(e: unknown): string | undefined {
  if (e && typeof e === 'object' && 'code' in e && typeof e.code === 'string') {
    return e.code
  }
  return undefined
}
```

**Why this matters:**
- Avoids unsafe type assertions like `(e as NodeJS.ErrnoException).code`
- Handles minified builds where class names are mangled
- Single source of truth for error detection

### 4.3 `json.ts` - Safe JSON Parsing

```typescript
// Lines 29-58: safeParseJSON with LRU cache
const PARSE_CACHE_MAX_KEY_BYTES = 8 * 1024

function parseJSONUncached(json: string, shouldLogError: boolean): CachedParse {
  try {
    return { ok: true, value: JSON.parse(stripBOM(json)) }
  } catch (e) {
    if (shouldLogError) logError(e)
    return { ok: false }
  }
}

const parseJSONCached = memoizeWithLRU(parseJSONUncached, json => json, 50)

export const safeParseJSON = Object.assign(
  function safeParseJSON(json: string | null | undefined, shouldLogError: boolean = true): unknown {
    if (!json) return null
    const result = json.length > PARSE_CACHE_MAX_KEY_BYTES
      ? parseJSONUncached(json, shouldLogError)
      : parseJSONCached(json, shouldLogError)
    return result.ok ? result.value : null
  },
  { cache: parseJSONCached.cache }
)
```

**Design decisions:**
1. **Invalid JSON cached too**: Prevents repeated parse failures on bad configs
2. **Size limit**: 8KB max key size prevents LRU from pinning memory with large files
3. **LRU bounded to 50 entries**: Prevents unbounded memory growth
4. **BOM stripping**: Handles PowerShell UTF-8 output

### 4.4 `fsOperations.ts` - Filesystem Abstraction

```typescript
// Lines 138-178: safeResolvePath
export function safeResolvePath(
  fs: FsOperations,
  filePath: string,
): { resolvedPath: string, isSymlink: boolean, isCanonical: boolean } {
  // Block UNC paths before filesystem access
  if (filePath.startsWith('\\\\') || filePath.startsWith('//')) {
    return { resolvedPath: filePath, isSymlink: false, isCanonical: false }
  }
  
  try {
    // Check for special file types before realpathSync
    const stats = fs.lstatSync(filePath)
    if (stats.isFIFO() || stats.isSocket() || 
        stats.isCharacterDevice() || stats.isBlockDevice()) {
      return { resolvedPath: filePath, isSymlink: false, isCanonical: false }
    }
    
    const resolvedPath = fs.realpathSync(filePath)
    return { resolvedPath, isSymlink: resolvedPath !== filePath, isCanonical: true }
  } catch (_error) {
    // ENOENT, broken symlink, EACCES, ELOOP -> return original path
    return { resolvedPath: filePath, isSymlink: false, isCanonical: false }
  }
}
```

**Error handling strategy:**
- UNC paths blocked before network access (prevents SMB/DNS requests)
- FIFOs/sockets/devices skipped (realpathSync blocks on FIFOs)
- Any error returns original path (allows file creation operations)

### 4.5 `path.ts` - Path Expansion

```typescript
// Lines 32-85: expandPath
export function expandPath(path: string, baseDir?: string): string {
  const actualBaseDir = baseDir ?? getCwd() ?? getFsImplementation().cwd()
  
  // Input validation
  if (typeof path !== 'string') {
    throw new TypeError(`Path must be a string, received ${typeof path}`)
  }
  
  // Security: null bytes
  if (path.includes('\0') || actualBaseDir.includes('\0')) {
    throw new Error('Path contains null bytes')
  }
  
  // Handle empty/whitespace
  const trimmedPath = path.trim()
  if (!trimmedPath) {
    return normalize(actualBaseDir).normalize('NFC')
  }
  
  // Tilde expansion
  if (trimmedPath === '~') return homedir().normalize('NFC')
  if (trimmedPath.startsWith('~/')) {
    return join(homedir(), trimmedPath.slice(2)).normalize('NFC')
  }
  
  // POSIX-to-Windows conversion on Windows
  let processedPath = trimmedPath
  if (getPlatform() === 'windows' && trimmedPath.match(/^\/[a-z]\//i)) {
    try {
      processedPath = posixPathToWindowsPath(trimmedPath)
    } catch {
      processedPath = trimmedPath
    }
  }
  
  // Absolute vs relative
  if (isAbsolute(processedPath)) {
    return normalize(processedPath).normalize('NFC')
  }
  
  return resolve(actualBaseDir, processedPath).normalize('NFC')
}
```

**Security features:**
- Null byte rejection (prevents path truncation attacks)
- Type validation (prevents prototype pollution via non-string paths)
- NFC normalization (Unicode consistency)

### 4.6 `sessionStoragePortable.ts` - Efficient Session Loading

```typescript
// Read head + tail only, avoiding full file read for large sessions
export function readHeadAndTail(
  filePath: string,
  headBytes: number,
  tailBytes: number,
): { head: string, tail: string } {
  const fd = openSync(filePath, 'r')
  try {
    const stats = fstatSync(fd)
    const fileSize = stats.size
    
    const headBuffer = Buffer.alloc(Math.min(headBytes, fileSize))
    readSync(fd, headBuffer, 0, headBuffer.length, 0)
    const head = headBuffer.toString('utf8')
    
    if (fileSize <= headBytes + tailBytes) {
      return { head, tail: '' }
    }
    
    const tailBuffer = Buffer.alloc(tailBytes)
    readSync(fd, tailBuffer, 0, tailBuffer.length, fileSize - tailBytes)
    const tail = tailBuffer.toString('utf8')
    
    return { head, tail }
  } finally {
    closeSync(fd)
  }
}
```

**Performance optimization:**
- Reads only first/last N bytes instead of full multi-GB files
- Used for session listing (shows first prompt + last message)
- Single file descriptor, no memory allocation for full file

### 4.7 `sleep.ts` - Abort-Responsive Sleep

```typescript
// Lines 14-54: sleep with abort handling
export function sleep(
  ms: number,
  signal?: AbortSignal,
  opts?: { throwOnAbort?: boolean, abortError?: () => Error, unref?: boolean },
): Promise<void> {
  return new Promise((resolve, reject) => {
    // Check aborted BEFORE timer setup (TDZ safety)
    if (signal?.aborted) {
      if (opts?.throwOnAbort || opts?.abortError) {
        reject(opts.abortError?.() ?? new Error('aborted'))
      } else {
        resolve()
      }
      return
    }
    
    const timer = setTimeout((signal, onAbort, resolve) => {
      signal?.removeEventListener('abort', onAbort)
      resolve()
    }, ms, signal, onAbort, resolve)
    
    function onAbort(): void {
      clearTimeout(timer)
      if (opts?.throwOnAbort || opts?.abortError) {
        reject(opts.abortError?.() ?? new Error('aborted'))
      } else {
        resolve()
      }
    }
    
    signal?.addEventListener('abort', onAbort, { once: true })
    if (opts?.unref) timer.unref()
  })
}
```

**Key features:**
- Aborted check before timer (prevents scheduling if already aborted)
- `unref` option (doesn't block process exit)
- `throwOnAbort` for retry loops that need rejection
- `abortError` factory for custom error types

---

## 5. Key Patterns

### 5.1 Error Handling Pattern

```typescript
// Catch-site normalization
try {
  await someOperation()
} catch (e) {
  if (isAbortError(e)) return  // Silent abort
  if (isENOENT(e)) return null  // File doesn't exist
  if (isFsInaccessible(e)) {
    logForDebugging(`Failed for expected reason: ${e.code}`)
    return null
  }
  logError(e)
  throw e
}
```

### 5.2 Caching Pattern

```typescript
// LRU-bounded cache with identity guard
const cache = new Map<string, CachedValue>()

const cached = cache.get(key)
if (cached) {
  if (isStale(cached) && !cached.refreshing) {
    cached.refreshing = true
    refreshAsync().then(newValue => {
      if (cache.get(key) === cached) {  // Identity guard
        cache.set(key, { value: newValue, timestamp: Date.now() })
      }
    })
  }
  return cached.value
}
```

### 5.3 Memoize with TTL (Background Refresh)

```typescript
export function memoizeWithTTL(f, cacheLifetimeMs) {
  const cache = new Map()
  return (...args) => {
    const key = jsonStringify(args)
    const cached = cache.get(key)
    const now = Date.now()
    
    if (!cached) {
      const value = f(...args)
      cache.set(key, { value, timestamp: now, refreshing: false })
      return value
    }
    
    // Stale-while-revalidate
    if (now - cached.timestamp > cacheLifetimeMs && !cached.refreshing) {
      cached.refreshing = true
      Promise.resolve()
        .then(() => f(...args))
        .then(newValue => {
          if (cache.get(key) === cached) {
            cache.set(key, { value: newValue, timestamp: Date.now(), refreshing: false })
          }
        })
        .catch(e => {
          logError(e)
          if (cache.get(key) === cached) cache.delete(key)
        })
      return cached.value
    }
    
    return cached.value
  }
}
```

### 5.4 FsOperations Abstraction

```typescript
// Interface for testability
export type FsOperations = {
  cwd(): string
  existsSync(path: string): boolean
  stat(path: string): Promise<fs.Stats>
  readdir(path: string): Promise<fs.Dirent[]>
  readFile(path: string, options: { encoding: BufferEncoding }): Promise<string>
  // ... 40+ methods
}

// Default implementation
export function getFsImplementation(): FsOperations {
  return {
    cwd: () => process.cwd(),
    existsSync: (path) => fs.existsSync(path),
    // ...
  }
}
```

---

## 6. Integration Points

### 6.1 With `bootstrap/state.js`

Utils imports global state for:
- `getCwd()`, `getOriginalCwd()` - Working directory
- `getSessionId()` - Current session
- `getFeatureValue_CACHED_MAY_BE_STALE()` - GrowthBook flags
- `getStatsStore()` - Statistics

### 6.2 With `services/`

Utils provides utilities TO services:
- `logEvent()` from `services/analytics/`
- `refreshOAuthToken()` from `services/oauth/`
- `ripGrep()` used by search services

### 6.3 With `tools/`

Utils provides:
- `FsOperations` for file-based tools
- `parseFrontmatter()` for markdown tools
- `ImageResizer` for image handling
- `memoizeWithTTL()` for tool result caching

### 6.4 With `cli/`

Utils provides:
- `process.ts` for stdio handling
- `format.ts` for output formatting
- `settings/` for config loading
- `auth.ts` for authentication

### 6.5 With `bridge/`

Utils provides:
- `memoize.ts` for JWT caching
- `format.ts` for status formatting
- `sessionStorage.ts` for session persistence

---

## 7. Performance Considerations

### 7.1 Memory Management

1. **LRU caches bounded**: `memoizeWithLRU()` limited to 50 entries
2. **Large file streaming**: `readHeadAndTail()` avoids full file loads
3. **Buffer reuse**: `Buffer.alloc()` for fixed-size reads
4. **CircularBuffer**: Fixed-size FIFO for message replay

### 7.2 I/O Optimization

1. **ripgrep for glob**: Memory-efficient file matching
2. **Sync stat for hot paths**: `getFileModificationTime()` uses statSync
3. **Async for cold paths**: `getFileModificationTimeAsync()` for non-critical
4. **BOM stripping upfront**: Single pass vs repeated checks

### 7.3 Caching Strategy

| Cache Type | Size | TTL | Use Case |
|------------|------|-----|----------|
| `safeParseJSON` | 50 entries, 8KB max key | N/A (LRU) | Settings, configs |
| `memoizeWithTTL` | Unbounded | 5 min default | Theme, platform |
| `memoizeWithLRU` | Bounded (50) | N/A | File content |
| `fileReadCache` | LRU | N/A | Recently read files |
| `toolSchemaCache` | LRU | N/A | MCP tool schemas |

---

## 8. Testing Guidelines

### 8.1 Mocking FsOperations

```typescript
const mockFs: FsOperations = {
  cwd: () => '/test',
  existsSync: (path) => path === '/test/existing',
  readFile: async (path, opts) => 'content',
  // ...
}
```

### 8.2 Testing Error Handlers

```typescript
// Test isENOENT
expect(isENOENT(new Error('fail'))).toBe(false)
expect(isENOENT(Object.assign(new Error('ENOENT'), { code: 'ENOENT' }))).toBe(true)

// Test isAbortError
expect(isAbortError(new AbortError())).toBe(true)
expect(isAbortError(new APIUserAbortError())).toBe(true)
```

### 8.3 Testing Path Expansion

```typescript
// Tilde expansion
expect(expandPath('~')).toBe(homedir())
expect(expandPath('~/docs')).toBe(join(homedir(), 'docs'))

// Null byte rejection
expect(() => expandPath('test\0path')).toThrow('Path contains null bytes')
```

---

## 9. Common Pitfalls

1. **Don't call `getCwd()` at module load time** - Use `getOriginalCwd()` or call at runtime
2. **Don't mutate cached settings** - Clone before returning from cache
3. **Don't use `jsonStringify()` in hot paths** - It's wrapped with `slowLogging`
4. **Don't assume file exists after `pathExists()`** - Race condition possible
5. **Don't forget identity guards in cache refresh** - `if (cache.get(key) === cached)`

---

## 10. Module Dependencies

```
utils/
├── Depends on: (minimal)
│   ├── bootstrap/state.js (global state)
│   ├── services/analytics/ (logEvent)
│   ├── services/oauth/ (auth)
│   └── External: lodash-es, zod, lru-cache, execa
│
└── Depended on by: (everything)
    ├── cli/
    ├── tools/
    ├── services/
    ├── bridge/
    ├── commands/
    └── All other modules
```

---

This exploration document provides a comprehensive reference for the `utils/` module. The 564 files here form the foundation of Claude Code - understanding these utilities is essential for any modification or extension of the codebase.

---

## 33. Complete Implementation Reference

This section provides complete implementation code for all major utility functions, organized by category.

### 33.1 Array Utilities (`array.ts`)

```typescript
/**
 * Intersperse array elements with separator values.
 * @param as - Source array
 * @param separator - Function generating separator based on index
 * @returns New array with separators inserted between elements
 */
export function intersperse<A>(as: A[], separator: (index: number) => A): A[] {
  return as.flatMap((a, i) => (i ? [separator(i), a] : [a]))
}

/**
 * Count elements matching predicate.
 * More efficient than filter().length for large arrays.
 * @param arr - Source array  
 * @param pred - Predicate function
 * @returns Count of matching elements
 */
export function count<T>(arr: readonly T[], pred: (x: T) => unknown): number {
  let n = 0
  for (const x of arr) n += +!!pred(x)
  return n
}

/**
 * Return unique elements from iterable.
 * @param xs - Any iterable collection
 * @returns Array of unique elements
 */
export function uniq<T>(xs: Iterable<T>): T[] {
  return [...new Set(xs)]
}
```

### 33.2 String Utilities (`stringUtils.ts`)

```typescript
/**
 * Escape special regex characters for literal pattern matching.
 */
export function escapeRegExp(str: string): string {
  return str.replace(/[.*+?^${}()|[\]\\]/g, '\\$&')
}

/**
 * Uppercase first character, leave rest unchanged.
 * Unlike lodash capitalize, does NOT lowercase remaining characters.
 */
export function capitalize(str: string): string {
  return str.charAt(0).toUpperCase() + str.slice(1)
}

/**
 * Return singular or plural form based on count.
 */
export function plural(
  n: number,
  word: string,
  pluralWord = word + 's',
): string {
  return n === 1 ? word : pluralWord
}

/**
 * Get first line without allocating split array.
 */
export function firstLineOf(s: string): string {
  const nl = s.indexOf('\n')
  return nl === -1 ? s : s.slice(0, nl)
}

/**
 * Count character occurrences using indexOf jumps.
 * Structurally typed so Buffer works too.
 */
export function countCharInString(
  str: { indexOf(search: string, start?: number): number },
  char: string,
  start = 0,
): number {
  let count = 0
  let i = str.indexOf(char, start)
  while (i !== -1) {
    count++
    i = str.indexOf(char, i + 1)
  }
  return count
}

/**
 * Normalize full-width (zenkaku) digits to half-width.
 */
export function normalizeFullWidthDigits(input: string): string {
  return input.replace(/[0-9]/g, ch =>
    String.fromCharCode(ch.charCodeAt(0) - 0xfee0),
  )
}

/**
 * Normalize full-width space (U+3000) to half-width space (U+0020).
 */
export function normalizeFullWidthSpace(input: string): string {
  return input.replace(/\u3000/g, ' ')
}

/**
 * Safely join lines with truncation at max size.
 */
const MAX_STRING_LENGTH = 2 ** 25

export function safeJoinLines(
  lines: string[],
  delimiter: string = ',',
  maxSize: number = MAX_STRING_LENGTH,
): string {
  const truncationMarker = '...[truncated]'
  let result = ''

  for (const line of lines) {
    const delimiterToAdd = result ? delimiter : ''
    const fullAddition = delimiterToAdd + line

    if (result.length + fullAddition.length <= maxSize) {
      result += fullAddition
    } else {
      const remainingSpace =
        maxSize -
        result.length -
        delimiterToAdd.length -
        truncationMarker.length

      if (remainingSpace > 0) {
        result +=
          delimiterToAdd + line.slice(0, remainingSpace) + truncationMarker
      } else {
        result += truncationMarker
      }
      return result
    }
  }
  return result
}

/**
 * String accumulator that truncates from end when limit exceeded.
 * Prevents RangeError crashes from large shell outputs.
 */
export class EndTruncatingAccumulator {
  private content: string = ''
  private isTruncated = false
  private totalBytesReceived = 0

  constructor(private readonly maxSize: number = MAX_STRING_LENGTH) {}

  append(data: string | Buffer): void {
    const str = typeof data === 'string' ? data : data.toString()
    this.totalBytesReceived += str.length

    if (this.isTruncated && this.content.length >= this.maxSize) {
      return
    }

    if (this.content.length + str.length > this.maxSize) {
      const remainingSpace = this.maxSize - this.content.length
      if (remainingSpace > 0) {
        this.content += str.slice(0, remainingSpace)
      }
      this.isTruncated = true
    } else {
      this.content += str
    }
  }

  toString(): string {
    if (!this.isTruncated) return this.content
    const truncatedBytes = this.totalBytesReceived - this.maxSize
    const truncatedKB = Math.round(truncatedBytes / 1024)
    return this.content + `\n... [output truncated - ${truncatedKB}KB removed]`
  }

  clear(): void {
    this.content = ''
    this.isTruncated = false
    this.totalBytesReceived = 0
  }

  get length(): number { return this.content.length }
  get truncated(): boolean { return this.isTruncated }
  get totalBytes(): number { return this.totalBytesReceived }
}

/**
 * Truncate text to maximum lines with ellipsis.
 */
export function truncateToLines(text: string, maxLines: number): string {
  const lines = text.split('\n')
  if (lines.length <= maxLines) return text
  return lines.slice(0, maxLines).join('\n') + '…'
}
```

### 33.3 Object Utilities (`objectGroupBy.ts`)

```typescript
/**
 * ECMA262 Object.groupBy polyfill.
 * Groups items by key selector.
 */
export function objectGroupBy<T, K extends PropertyKey>(
  items: Iterable<T>,
  keySelector: (item: T, index: number) => K,
): Partial<Record<K, T[]>> {
  const result = Object.create(null) as Partial<Record<K, T[]>>
  let index = 0
  for (const item of items) {
    const key = keySelector(item, index++)
    if (result[key] === undefined) {
      result[key] = []
    }
    result[key].push(item)
  }
  return result
}
```

### 33.4 Set Utilities (`set.ts`)

```typescript
/**
 * Set difference: elements in A but not in B.
 * Optimized for hot paths.
 */
export function difference<A>(a: Set<A>, b: Set<A>): Set<A> {
  const result = new Set<A>()
  for (const item of a) {
    if (!b.has(item)) {
      result.add(item)
    }
  }
  return result
}

/**
 * Check if sets have any common elements.
 * Early exit if either set is empty.
 */
export function intersects<A>(a: Set<A>, b: Set<A>): boolean {
  if (a.size === 0 || b.size === 0) return false
  for (const item of a) {
    if (b.has(item)) return true
  }
  return false
}

/**
 * Subset check: every element in A exists in B.
 */
export function every<A>(a: ReadonlySet<A>, b: ReadonlySet<A>): boolean {
  for (const item of a) {
    if (!b.has(item)) return false
  }
  return true
}

/**
 * Set union: combine two sets.
 */
export function union<A>(a: Set<A>, b: Set<A>): Set<A> {
  const result = new Set<A>()
  for (const item of a) result.add(item)
  for (const item of b) result.add(item)
  return result
}
```

### 33.5 Async/Concurrency Utilities

#### sleep.ts

```typescript
/**
 * Abort-responsive sleep utility.
 * Resolves after ms or immediately when signal aborts.
 */
export function sleep(
  ms: number,
  signal?: AbortSignal,
  opts?: { throwOnAbort?: boolean; abortError?: () => Error; unref?: boolean },
): Promise<void> {
  return new Promise((resolve, reject) => {
    if (signal?.aborted) {
      if (opts?.throwOnAbort || opts?.abortError) {
        reject(opts.abortError?.() ?? new Error('aborted'))
      } else {
        resolve()
      }
      return
    }
    const timer = setTimeout(
      (signal, onAbort, resolve) => {
        signal?.removeEventListener('abort', onAbort)
        resolve()
      },
      ms,
      signal,
      onAbort,
      resolve,
    )
    function onAbort(): void {
      clearTimeout(timer)
      if (opts?.throwOnAbort || opts?.abortError) {
        reject(opts.abortError?.() ?? new Error('aborted'))
      } else {
        resolve()
      }
    }
    signal?.addEventListener('abort', onAbort, { once: true })
    if (opts?.unref) timer.unref()
  })
}

/**
 * Race promise against timeout.
 * Timer is cleared when promise settles.
 */
export function withTimeout<T>(
  promise: Promise<T>,
  ms: number,
  message: string,
): Promise<T> {
  let timer: ReturnType<typeof setTimeout> | undefined
  const timeoutPromise = new Promise<never>((_, reject) => {
    timer = setTimeout(rejectWithTimeout, ms, reject, message)
    if (typeof timer === 'object') timer.unref?.()
  })
  return Promise.race([promise, timeoutPromise]).finally(() => {
    if (timer !== undefined) clearTimeout(timer)
  })
}

function rejectWithTimeout(reject: (e: Error) => void, message: string): void {
  reject(new Error(message))
}
```

#### sequential.ts

```typescript
/**
 * Sequential execution wrapper for async functions.
 * Prevents race conditions by queuing calls.
 */
type QueueItem<T extends unknown[], R> = {
  args: T
  resolve: (value: R) => void
  reject: (reason?: unknown) => void
  context: unknown
}

export function sequential<T extends unknown[], R>(
  fn: (...args: T) => Promise<R>,
): (...args: T) => Promise<R> {
  const queue: QueueItem<T, R>[] = []
  let processing = false

  async function processQueue(): Promise<void> {
    if (processing || queue.length === 0) return
    processing = true

    while (queue.length > 0) {
      const { args, resolve, reject, context } = queue.shift()!
      try {
        const result = await fn.apply(context, args)
        resolve(result)
      } catch (error) {
        reject(error)
      }
    }

    processing = false
    if (queue.length > 0) void processQueue()
  }

  return function (this: unknown, ...args: T): Promise<R> {
    return new Promise((resolve, reject) => {
      queue.push({ args, resolve, reject, context: this })
      void processQueue()
    })
  }
}
```

### 33.6 File System Utilities (`fsOperations.ts`)

```typescript
/**
 * Filesystem operations interface for abstraction.
 */
export type FsOperations = {
  cwd(): string
  existsSync(path: string): boolean
  stat(path: string): Promise<fs.Stats>
  statSync(path: string): fs.Stats
  lstatSync(path: string): fs.Stats
  readdir(path: string): Promise<fs.Dirent[]>
  readdirSync(path: string): fs.Dirent[]
  readdirStringSync(path: string): string[]
  isDirEmptySync(path: string): boolean
  readFile(path: string, options: { encoding: BufferEncoding }): Promise<string>
  readFileSync(path: string, options: { encoding: BufferEncoding }): string
  readFileBytesSync(path: string): Buffer
  readFileBytes(path: string, maxBytes?: number): Promise<Buffer>
  readSync(path: string, options: { length: number }): { buffer: Buffer; bytesRead: number }
  writeFile(path: string, data: string | Buffer): Promise<void>
  writeFileSync(path: string, data: string | Buffer): void
  appendFileSync(path: string, data: string, options?: { mode?: number }): void
  mkdir(path: string, options?: { mode?: number }): Promise<void>
  mkdirSync(path: string, options?: { mode?: number }): void
  unlink(path: string): Promise<void>
  unlinkSync(path: string): void
  rmdir(path: string): Promise<void>
  rmdirSync(path: string): void
  rm(path: string, options?: { recursive?: boolean; force?: boolean }): Promise<void>
  rmSync(path: string, options?: { recursive?: boolean; force?: boolean }): void
  rename(oldPath: string, newPath: string): Promise<void>
  renameSync(oldPath: string, newPath: string): void
  copyFileSync(src: string, dest: string): void
  linkSync(target: string, path: string): void
  symlinkSync(target: string, path: string, type?: 'dir' | 'file' | 'junction'): void
  readlinkSync(path: string): string
  realpathSync(path: string): string
  createWriteStream(path: string): fs.WriteStream
}

/**
 * Safely resolve path handling symlinks and errors.
 */
export function safeResolvePath(
  fs: FsOperations,
  filePath: string,
): { resolvedPath: string; isSymlink: boolean; isCanonical: boolean } {
  if (filePath.startsWith('//') || filePath.startsWith('\\\\')) {
    return { resolvedPath: filePath, isSymlink: false, isCanonical: false }
  }
  try {
    const stats = fs.lstatSync(filePath)
    if (stats.isFIFO() || stats.isSocket() || 
        stats.isCharacterDevice() || stats.isBlockDevice()) {
      return { resolvedPath: filePath, isSymlink: false, isCanonical: false }
    }
    const resolvedPath = fs.realpathSync(filePath)
    return {
      resolvedPath,
      isSymlink: resolvedPath !== filePath,
      isCanonical: true,
    }
  } catch {
    return { resolvedPath: filePath, isSymlink: false, isCanonical: false }
  }
}

/**
 * Read file bytes from offset.
 */
export type ReadFileRangeResult = {
  content: string
  bytesRead: number
  bytesTotal: number
}

export async function readFileRange(
  path: string,
  offset: number,
  maxBytes: number,
): Promise<ReadFileRangeResult | null> {
  await using fh = await open(path, 'r')
  const size = (await fh.stat()).size
  if (size <= offset) return null
  const bytesToRead = Math.min(size - offset, maxBytes)
  const buffer = Buffer.allocUnsafe(bytesToRead)
  let totalRead = 0
  while (totalRead < bytesToRead) {
    const { bytesRead } = await fh.read(
      buffer, totalRead, bytesToRead - totalRead, offset + totalRead,
    )
    if (bytesRead === 0) break
    totalRead += bytesRead
  }
  return {
    content: buffer.toString('utf8', 0, totalRead),
    bytesRead: totalRead,
    bytesTotal: size,
  }
}

/**
 * Read last N bytes of file.
 */
export async function tailFile(
  path: string,
  maxBytes: number,
): Promise<ReadFileRangeResult> {
  await using fh = await open(path, 'r')
  const size = (await fh.stat()).size
  if (size === 0) return { content: '', bytesRead: 0, bytesTotal: 0 }
  const offset = Math.max(0, size - maxBytes)
  const bytesToRead = size - offset
  const buffer = Buffer.allocUnsafe(bytesToRead)
  let totalRead = 0
  while (totalRead < bytesToRead) {
    const { bytesRead } = await fh.read(
      buffer, totalRead, bytesToRead - totalRead, offset + totalRead,
    )
    if (bytesRead === 0) break
    totalRead += bytesRead
  }
  return {
    content: buffer.toString('utf8', 0, totalRead),
    bytesRead: totalRead,
    bytesTotal: size,
  }
}

/**
 * Async generator yielding lines in reverse order.
 */
export async function* readLinesReverse(path: string): AsyncGenerator<string> {
  const CHUNK_SIZE = 1024 * 4
  const fileHandle = await open(path, 'r')
  try {
    const stats = await fileHandle.stat()
    let position = stats.size
    let remainder = Buffer.alloc(0)
    const buffer = Buffer.alloc(CHUNK_SIZE)

    while (position > 0) {
      const currentChunkSize = Math.min(CHUNK_SIZE, position)
      position -= currentChunkSize
      await fileHandle.read(buffer, 0, currentChunkSize, position)
      const combined = Buffer.concat([buffer.subarray(0, currentChunkSize), remainder])
      const firstNewline = combined.indexOf(0x0a)
      if (firstNewline === -1) {
        remainder = combined
        continue
      }
      remainder = Buffer.from(combined.subarray(0, firstNewline))
      const lines = combined.toString('utf8', firstNewline + 1).split('\n')
      for (let i = lines.length - 1; i >= 0; i--) {
        if (lines[i]) yield lines[i]
      }
    }
    if (remainder.length > 0) yield remainder.toString('utf8')
  } finally {
    await fileHandle.close()
  }
}
```

### 33.7 Path Utilities (`path.ts`)

```typescript
/**
 * Expand tilde notation and resolve paths.
 */
export function expandPath(path: string, baseDir?: string): string {
  const actualBaseDir = baseDir ?? getCwd() ?? getFsImplementation().cwd()

  if (typeof path !== 'string') {
    throw new TypeError(`Path must be string, received ${typeof path}`)
  }
  if (typeof actualBaseDir !== 'string') {
    throw new TypeError(`Base directory must be string`)
  }
  if (path.includes('\0') || actualBaseDir.includes('\0')) {
    throw new Error('Path contains null bytes')
  }

  const trimmedPath = path.trim()
  if (!trimmedPath) return normalize(actualBaseDir).normalize('NFC')
  if (trimmedPath === '~') return homedir().normalize('NFC')
  if (trimmedPath.startsWith('~/')) {
    return join(homedir(), trimmedPath.slice(2)).normalize('NFC')
  }

  let processedPath = trimmedPath
  if (getPlatform() === 'windows' && trimmedPath.match(/^\/[a-z]\//i)) {
    try {
      processedPath = posixPathToWindowsPath(trimmedPath)
    } catch {
      processedPath = trimmedPath
    }
  }

  if (isAbsolute(processedPath)) {
    return normalize(processedPath).normalize('NFC')
  }

  return resolve(actualBaseDir, processedPath).normalize('NFC')
}

/**
 * Convert absolute path to relative from cwd.
 */
export function toRelativePath(absolutePath: string): string {
  const relativePath = relative(getCwd(), absolutePath)
  return relativePath.startsWith('..') ? absolutePath : relativePath
}

/**
 * Check for directory traversal patterns.
 */
export function containsPathTraversal(path: string): boolean {
  return /(?:^|[\\/])\.\.(?:[\\/]|$)/.test(path)
}

/**
 * Normalize path for JSON config key (Windows compatibility).
 */
export function normalizePathForConfigKey(path: string): string {
  const normalized = normalize(path)
  if (getPlatform() === 'windows') {
    return normalized.replace(/\\/g, '/')
  }
  return normalized
}
```

### 33.8 JSON Utilities (`json.ts`)

```typescript
const PARSE_CACHE_MAX_KEY_BYTES = 8 * 1024

type CachedParse = { ok: true; value: unknown } | { ok: false }

/**
 * Safe JSON parse with LRU caching.
 * Bounded to 50 entries, skips large inputs.
 */
export const safeParseJSON = Object.assign(
  function safeParseJSON(
    json: string | null | undefined,
    shouldLogError: boolean = true,
  ): unknown {
    if (!json) return null
    const result = json.length > PARSE_CACHE_MAX_KEY_BYTES
      ? parseJSONUncached(json, shouldLogError)
      : parseJSONCached(json, shouldLogError)
    return result.ok ? result.value : null
  },
  { cache: parseJSONCached.cache },
)

/**
 * Parse JSON with comments (jsonc).
 */
export function safeParseJSONC(json: string | null | undefined): unknown {
  if (!json) return null
  try {
    return parseJsonc(stripBOM(json))
  } catch (e) {
    logError(e)
    return null
  }
}

/**
 * Parse JSONL data.
 */
export function parseJSONL<T>(data: string | Buffer): T[] {
  if (bunJSONLParse) return parseJSONLBun<T>(data)
  if (typeof data === 'string') return parseJSONLString<T>(data)
  return parseJSONLBuffer<T>(data)
}

/**
 * Read JSONL file, at most last 100MB.
 */
const MAX_JSONL_READ_BYTES = 100 * 1024 * 1024

export async function readJSONLFile<T>(filePath: string): Promise<T[]> {
  const { size } = await stat(filePath)
  if (size <= MAX_JSONL_READ_BYTES) {
    return parseJSONL<T>(await readFile(filePath))
  }
  await using fd = await open(filePath, 'r')
  const buf = Buffer.allocUnsafe(MAX_JSONL_READ_BYTES)
  let totalRead = 0
  const fileOffset = size - MAX_JSONL_READ_BYTES
  while (totalRead < MAX_JSONL_READ_BYTES) {
    const { bytesRead } = await fd.read(
      buf, totalRead, MAX_JSONL_READ_BYTES - totalRead, fileOffset + totalRead,
    )
    if (bytesRead === 0) break
    totalRead += bytesRead
  }
  const newlineIndex = buf.indexOf(0x0a)
  if (newlineIndex !== -1 && newlineIndex < totalRead - 1) {
    return parseJSONL<T>(buf.subarray(newlineIndex + 1, totalRead))
  }
  return parseJSONL<T>(buf.subarray(0, totalRead))
}
```

### 33.9 Error Handling Utilities (`errors.ts`)

```typescript
/**
 * Check if error is abort-related.
 */
export function isAbortError(e: unknown): boolean {
  return (
    e instanceof AbortError ||
    e instanceof APIUserAbortError ||
    (e instanceof Error && e.name === 'AbortError')
  )
}

/**
 * Normalize unknown to Error.
 */
export function toError(e: unknown): Error {
  return e instanceof Error ? e : new Error(String(e))
}

/**
 * Extract error message from unknown.
 */
export function errorMessage(e: unknown): string {
  return e instanceof Error ? e.message : String(e)
}

/**
 * Extract errno code from error.
 */
export function getErrnoCode(e: unknown): string | undefined {
  if (e && typeof e === 'object' && 'code' in e && typeof e.code === 'string') {
    return e.code
  }
  return undefined
}

export function isENOENT(e: unknown): boolean {
  return getErrnoCode(e) === 'ENOENT'
}

/**
 * Extract errno path from error.
 */
export function getErrnoPath(e: unknown): string | undefined {
  if (e && typeof e === 'object' && 'path' in e && typeof e.path === 'string') {
    return e.path
  }
  return undefined
}

/**
 * Get error message + top N stack frames.
 */
export function shortErrorStack(e: unknown, maxFrames = 5): string {
  if (!(e instanceof Error)) return String(e)
  if (!e.stack) return e.message
  const lines = e.stack.split('\n')
  const header = lines[0] ?? e.message
  const frames = lines.slice(1).filter(l => l.trim().startsWith('at '))
  if (frames.length <= maxFrames) return e.stack
  return [header, ...frames.slice(0, maxFrames)].join('\n')
}

/**
 * Check if error is filesystem inaccessible.
 */
export function isFsInaccessible(e: unknown): e is NodeJS.ErrnoException {
  const code = getErrnoCode(e)
  return code === 'ENOENT' || code === 'EACCES' || code === 'EPERM' ||
         code === 'ENOTDIR' || code === 'ELOOP'
}
```

### 33.10 Memoization Utilities (`memoize.ts`)

```typescript
/**
 * Memoization with TTL and background refresh.
 */
export function memoizeWithTTL<Args extends unknown[], Result>(
  f: (...args: Args) => Result,
  cacheLifetimeMs: number = 5 * 60 * 1000,
): MemoizedFunction<Args, Result> {
  const cache = new Map<string, CacheEntry<Result>>()

  const memoized = (...args: Args): Result => {
    const key = jsonStringify(args)
    const cached = cache.get(key)
    const now = Date.now()

    if (!cached) {
      const value = f(...args)
      cache.set(key, { value, timestamp: now, refreshing: false })
      return value
    }

    if (cached && now - cached.timestamp > cacheLifetimeMs && !cached.refreshing) {
      cached.refreshing = true
      Promise.resolve().then(() => {
        const newValue = f(...args)
        if (cache.get(key) === cached) {
          cache.set(key, { value: newValue, timestamp: Date.now(), refreshing: false })
        }
      }).catch(e => {
        logError(e)
        if (cache.get(key) === cached) cache.delete(key)
      })
      return cached.value
    }

    return cache.get(key)!.value
  }

  memoized.cache = { clear: () => cache.clear() }
  return memoized
}

/**
 * Async memoization with deduplication.
 */
export function memoizeWithTTLAsync<Args extends unknown[], Result>(
  f: (...args: Args) => Promise<Result>,
  cacheLifetimeMs: number = 5 * 60 * 1000,
): ((...args: Args) => Promise<Result>) & { cache: { clear: () => void } } {
  const cache = new Map<string, CacheEntry<Result>>()
  const inFlight = new Map<string, Promise<Result>>()

  const memoized = async (...args: Args): Promise<Result> => {
    const key = jsonStringify(args)
    const cached = cache.get(key)
    const now = Date.now()

    if (!cached) {
      const pending = inFlight.get(key)
      if (pending) return pending
      const promise = f(...args)
      inFlight.set(key, promise)
      try {
        const result = await promise
        if (inFlight.get(key) === promise) {
          cache.set(key, { value: result, timestamp: now, refreshing: false })
        }
        return result
      } finally {
        if (inFlight.get(key) === promise) inFlight.delete(key)
      }
    }

    if (cached && now - cached.timestamp > cacheLifetimeMs && !cached.refreshing) {
      cached.refreshing = true
      const staleEntry = cached
      f(...args).then(newValue => {
        if (cache.get(key) === staleEntry) {
          cache.set(key, { value: newValue, timestamp: Date.now(), refreshing: false })
        }
      }).catch(e => {
        logError(e)
        if (cache.get(key) === staleEntry) cache.delete(key)
      })
      return cached.value
    }

    return cache.get(key)!.value
  }

  memoized.cache = { clear: () => { cache.clear(); inFlight.clear() } }
  return memoized
}

/**
 * LRU-bounded memoization.
 */
export function memoizeWithLRU<
  Args extends unknown[],
  Result extends NonNullable<unknown>,
>(
  f: (...args: Args) => Result,
  cacheFn: (...args: Args) => string,
  maxCacheSize: number = 100,
): LRUMemoizedFunction<Args, Result> {
  const cache = new LRUCache<string, Result>({ max: maxCacheSize })

  const memoized = (...args: Args): Result => {
    const key = cacheFn(...args)
    const cached = cache.get(key)
    if (cached !== undefined) return cached
    const result = f(...args)
    cache.set(key, result)
    return result
  }

  memoized.cache = {
    clear: () => cache.clear(),
    size: () => cache.size,
    delete: (key: string) => cache.delete(key),
    get: (key: string) => cache.peek(key),
    has: (key: string) => cache.has(key),
  }

  return memoized
}
```

### 33.11 AbortController Utilities (`abortController.ts`)

```typescript
/**
 * Create AbortController with listener limit.
 */
const DEFAULT_MAX_LISTENERS = 50

export function createAbortController(
  maxListeners: number = DEFAULT_MAX_LISTENERS,
): AbortController {
  const controller = new AbortController()
  setMaxListeners(maxListeners, controller.signal)
  return controller
}

/**
 * Create child controller that aborts when parent aborts.
 * Uses WeakRef for memory safety.
 */
export function createChildAbortController(
  parent: AbortController,
  maxListeners?: number,
): AbortController {
  const child = createAbortController(maxListeners)

  if (parent.signal.aborted) {
    child.abort(parent.signal.reason)
    return child
  }

  const weakChild = new WeakRef(child)
  const weakParent = new WeakRef(parent)
  
  const handler = function(this: WeakRef<AbortController>) {
    const p = this.deref()
    weakChild.deref()?.abort(p?.signal.reason)
  }.bind(weakParent)

  parent.signal.addEventListener('abort', handler, { once: true })

  const cleanupHandler = function(this: WeakRef<AbortController>) {
    const p = this.deref()
    if (p) p.signal.removeEventListener('abort', handler)
  }.bind(weakParent)

  child.signal.addEventListener('abort', cleanupHandler, { once: true })
  return child
}
```

### 33.12 CircularBuffer (`CircularBuffer.ts`)

```typescript
/**
 * Fixed-size circular buffer with automatic eviction.
 */
export class CircularBuffer<T> {
  private buffer: T[]
  private head = 0
  private size = 0

  constructor(private capacity: number) {
    this.buffer = new Array(capacity)
  }

  add(item: T): void {
    this.buffer[this.head] = item
    this.head = (this.head + 1) % this.capacity
    if (this.size < this.capacity) this.size++
  }

  addAll(items: T[]): void {
    for (const item of items) this.add(item)
  }

  getRecent(count: number): T[] {
    const result: T[] = []
    const start = this.size < this.capacity ? 0 : this.head
    const available = Math.min(count, this.size)
    for (let i = 0; i < available; i++) {
      const index = (start + this.size - available + i) % this.capacity
      result.push(this.buffer[index]!)
    }
    return result
  }

  toArray(): T[] {
    if (this.size === 0) return []
    const result: T[] = []
    const start = this.size < this.capacity ? 0 : this.head
    for (let i = 0; i < this.size; i++) {
      const index = (start + i) % this.capacity
      result.push(this.buffer[index]!)
    }
    return result
  }

  clear(): void {
    this.buffer.length = 0
    this.head = 0
    this.size = 0
  }

  length(): number { return this.size }
}
```

### 33.13 Platform Detection (`platform.ts`)

```typescript
export type Platform = 'macos' | 'windows' | 'wsl' | 'linux' | 'unknown'

export const getPlatform = memoize((): Platform => {
  try {
    if (process.platform === 'darwin') return 'macos'
    if (process.platform === 'win32') return 'windows'
    if (process.platform === 'linux') {
      try {
        const procVersion = getFsImplementation().readFileSync('/proc/version', { encoding: 'utf8' })
        if (procVersion.toLowerCase().includes('microsoft') ||
            procVersion.toLowerCase().includes('wsl')) {
          return 'wsl'
        }
      } catch {
        logError(error)
      }
      return 'linux'
    }
    return 'unknown'
  } catch {
    logError(error)
    return 'unknown'
  }
})

export const getWslVersion = memoize((): string | undefined => {
  if (process.platform !== 'linux') return undefined
  try {
    const procVersion = getFsImplementation().readFileSync('/proc/version', { encoding: 'utf8' })
    const match = procVersion.match(/WSL(\d+)/i)
    if (match) return match[1]
    if (procVersion.toLowerCase().includes('microsoft')) return '1'
    return undefined
  } catch {
    return undefined
  }
})

/**
 * Detect VCS in directory.
 */
const VCS_MARKERS: Array<[string, string]> = [
  ['.git', 'git'], ['.hg', 'mercurial'], ['.svn', 'svn'],
  ['.p4config', 'perforce'], ['$tf', 'tfs'], ['.tfvc', 'tfs'],
  ['.jj', 'jujutsu'], ['.sl', 'sapling'],
]

export async function detectVcs(dir?: string): Promise<string[]> {
  const detected = new Set<string>()
  if (process.env.P4PORT) detected.add('perforce')
  try {
    const targetDir = dir ?? getFsImplementation().cwd()
    const entries = new Set(await readdir(targetDir))
    for (const [marker, vcs] of VCS_MARKERS) {
      if (entries.has(marker)) detected.add(vcs)
    }
  } catch {}
  return [...detected]
}
```

### 33.14 Format Utilities (`format.ts`)

```typescript
/**
 * Format byte count to human-readable.
 */
export function formatFileSize(sizeInBytes: number): string {
  const kb = sizeInBytes / 1024
  if (kb < 1) return `${sizeInBytes} bytes`
  if (kb < 1024) return `${kb.toFixed(1).replace(/\.0$/, '')}KB`
  const mb = kb / 1024
  if (mb < 1024) return `${mb.toFixed(1).replace(/\.0$/, '')}MB`
  const gb = mb / 1024
  return `${gb.toFixed(1).replace(/\.0$/, '')}GB`
}

/**
 * Format duration with multiple units.
 */
export function formatDuration(
  ms: number,
  options?: { hideTrailingZeros?: boolean; mostSignificantOnly?: boolean },
): string {
  if (ms < 60000) {
    if (ms === 0) return '0s'
    if (ms < 1) return `${(ms / 1000).toFixed(1)}s`
    return `${Math.floor(ms / 1000)}s`
  }

  let days = Math.floor(ms / 86400000)
  let hours = Math.floor((ms % 86400000) / 3600000)
  let minutes = Math.floor((ms % 3600000) / 60000)
  let seconds = Math.round((ms % 60000) / 1000)

  // Handle carry-over
  if (seconds === 60) { seconds = 0; minutes++ }
  if (minutes === 60) { minutes = 0; hours++ }
  if (hours === 24) { hours = 0; days++ }

  const hide = options?.hideTrailingZeros
  if (options?.mostSignificantOnly) {
    if (days > 0) return `${days}d`
    if (hours > 0) return `${hours}h`
    if (minutes > 0) return `${minutes}m`
    return `${seconds}s`
  }

  if (days > 0) {
    if (hide && hours === 0 && minutes === 0) return `${days}d`
    if (hide && minutes === 0) return `${days}d ${hours}h`
    return `${days}d ${hours}h ${minutes}m`
  }
  if (hours > 0) {
    if (hide && minutes === 0 && seconds === 0) return `${hours}h`
    if (hide && seconds === 0) return `${hours}h ${minutes}m`
    return `${hours}h ${minutes}m ${seconds}s`
  }
  if (minutes > 0) {
    if (hide && seconds === 0) return `${minutes}m`
    return `${minutes}m ${seconds}s`
  }
  return `${seconds}s`
}

/**
 * Format number with compact notation.
 */
export function formatNumber(number: number): string {
  const shouldUseConsistentDecimals = number >= 1000
  return getNumberFormatter(shouldUseConsistentDecimals)
    .format(number)
    .toLowerCase()
}

export function formatTokens(count: number): string {
  return formatNumber(count).replace('.0', '')
}
```

### 33.15 Word Slug Utilities (`words.ts`)

```typescript
/**
 * Generate random word slug: "adjective-verb-noun"
 * Example: "gleaming-brewing-phoenix"
 */
export function generateWordSlug(): string {
  const adjective = pickRandom(ADJECTIVES)
  const verb = pickRandom(VERBS)
  const noun = pickRandom(NOUNS)
  return `${adjective}-${verb}-${noun}`
}

/**
 * Generate short word slug: "adjective-noun"
 * Example: "graceful-unicorn"
 */
export function generateShortWordSlug(): string {
  const adjective = pickRandom(ADJECTIVES)
  const noun = pickRandom(NOUNS)
  return `${adjective}-${noun}`
}

// Word lists (abbreviated - actual file has 500+ words each)
const ADJECTIVES = ['abundant', 'ancient', 'bright', 'cosmic', 'async', ...] as const
const NOUNS = ['aurora', 'blossom', 'phoenix', 'dragonfly', 'turing', ...] as const
const VERBS = ['baking', 'conjuring', 'purring', 'wandering', 'zooming', ...] as const

function randomInt(max: number): number {
  const bytes = randomBytes(4)
  return bytes.readUInt32BE(0) % max
}

function pickRandom<T>(array: readonly T[]): T {
  return array[randomInt(array.length)]!
}
```

### 33.16 Sanitization Utilities (`sanitization.ts`)

```typescript
/**
 * Unicode sanitization for hidden character attack mitigation.
 * Protects against ASCII smuggling and hidden prompt injection.
 */
export function partiallySanitizeUnicode(prompt: string): string {
  let current = prompt
  let previous = ''
  let iterations = 0
  const MAX_ITERATIONS = 10

  while (current !== previous && iterations < MAX_ITERATIONS) {
    previous = current
    current = current.normalize('NFKC')
    current = current.replace(/[\p{Cf}\p{Co}\p{Cn}]/gu, '')
    current = current
      .replace(/[\u200B-\u200F]/g, '')  // Zero-width spaces
      .replace(/[\u202A-\u202E]/g, '')  // Directional formatting
      .replace(/[\u2066-\u2069]/g, '')  // Directional isolates
      .replace(/[\uFEFF]/g, '')         // BOM
      .replace(/[\uE000-\uF8FF]/g, '')  // Private use area
    iterations++
  }

  if (iterations >= MAX_ITERATIONS) {
    throw new Error(`Unicode sanitization reached maximum iterations`)
  }

  return current
}

/**
 * Recursive sanitization for complex data structures.
 */
export function recursivelySanitizeUnicode(value: unknown): unknown {
  if (typeof value === 'string') return partiallySanitizeUnicode(value)
  if (Array.isArray(value)) return value.map(recursivelySanitizeUnicode)
  if (value !== null && typeof value === 'object') {
    const sanitized: Record<string, unknown> = {}
    for (const [key, val] of Object.entries(value)) {
      sanitized[recursivelySanitizeUnicode(key)] = recursivelySanitizeUnicode(val)
    }
    return sanitized
  }
  return value
}
```

### 33.17 Intl/Localization Utilities (`intl.ts`)

```typescript
/**
 * Cached segmenters for Unicode text processing.
 */
let graphemeSegmenter: Intl.Segmenter | null = null
let wordSegmenter: Intl.Segmenter | null = null

export function getGraphemeSegmenter(): Intl.Segmenter {
  if (!graphemeSegmenter) {
    graphemeSegmenter = new Intl.Segmenter(undefined, { granularity: 'grapheme' })
  }
  return graphemeSegmenter
}

export function getWordSegmenter(): Intl.Segmenter {
  if (!wordSegmenter) {
    wordSegmenter = new Intl.Segmenter(undefined, { granularity: 'word' })
  }
  return wordSegmenter
}

/**
 * Extract first/last grapheme cluster.
 */
export function firstGrapheme(text: string): string {
  if (!text) return ''
  const segments = getGraphemeSegmenter().segment(text)
  const first = segments[Symbol.iterator]().next().value
  return first?.segment ?? ''
}

export function lastGrapheme(text: string): string {
  if (!text) return ''
  let last = ''
  for (const { segment } of getGraphemeSegmenter().segment(text)) {
    last = segment
  }
  return last
}

/**
 * Cached RelativeTimeFormat.
 */
const rtfCache = new Map<string, Intl.RelativeTimeFormat>()

export function getRelativeTimeFormat(
  style: 'long' | 'short' | 'narrow',
  numeric: 'always' | 'auto',
): Intl.RelativeTimeFormat {
  const key = `${style}:${numeric}`
  let rtf = rtfCache.get(key)
  if (!rtf) {
    rtf = new Intl.RelativeTimeFormat('en', { style, numeric })
    rtfCache.set(key, rtf)
  }
  return rtf
}

/**
 * Cached timezone.
 */
let cachedTimeZone: string | null = null

export function getTimeZone(): string {
  if (!cachedTimeZone) {
    cachedTimeZone = Intl.DateTimeFormat().resolvedOptions().timeZone
  }
  return cachedTimeZone
}
```

### 33.18 TaggedID Utilities (`taggedId.ts`)

```typescript
const BASE_58_CHARS = '123456789ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz'
const VERSION = '01'
const ENCODED_LENGTH = 22

/**
 * Base58 encode 128-bit integer.
 */
function base58Encode(n: bigint): string {
  const base = BigInt(BASE_58_CHARS.length)
  const result = new Array<string>(ENCODED_LENGTH).fill(BASE_58_CHARS[0]!)
  let i = ENCODED_LENGTH - 1
  let value = n
  while (value > 0n) {
    const rem = Number(value % base)
    result[i] = BASE_58_CHARS[rem]!
    value = value / base
    i--
  }
  return result.join('')
}

/**
 * Parse UUID to bigint.
 */
function uuidToBigInt(uuid: string): bigint {
  const hex = uuid.replace(/-/g, '')
  if (hex.length !== 32) throw new Error(`Invalid UUID hex length: ${hex.length}`)
  return BigInt('0x' + hex)
}

/**
 * Convert UUID to tagged ID format.
 * Output: "user_01PaGUP2rbg1XDh7Z9W1CEpd"
 */
export function toTaggedId(tag: string, uuid: string): string {
  const n = uuidToBigInt(uuid)
  return `${tag}_${VERSION}${base58Encode(n)}`
}
```

### 33.19 Semver Utilities (`semver.ts`)

```typescript
/**
 * Semver comparison using Bun.semver or npm semver fallback.
 */
export function gt(a: string, b: string): boolean {
  if (typeof Bun !== 'undefined') return Bun.semver.order(a, b) === 1
  return getNpmSemver().gt(a, b, { loose: true })
}

export function gte(a: string, b: string): boolean {
  if (typeof Bun !== 'undefined') return Bun.semver.order(a, b) >= 0
  return getNpmSemver().gte(a, b, { loose: true })
}

export function lt(a: string, b: string): boolean {
  if (typeof Bun !== 'undefined') return Bun.semver.order(a, b) === -1
  return getNpmSemver().lt(a, b, { loose: true })
}

export function lte(a: string, b: string): boolean {
  if (typeof Bun !== 'undefined') return Bun.semver.order(a, b) <= 0
  return getNpmSemver().lte(a, b, { loose: true })
}

export function satisfies(version: string, range: string): boolean {
  if (typeof Bun !== 'undefined') return Bun.semver.satisfies(version, range)
  return getNpmSemver().satisfies(version, range, { loose: true })
}

export function order(a: string, b: string): -1 | 0 | 1 {
  if (typeof Bun !== 'undefined') return Bun.semver.order(a, b)
  return getNpmSemver().compare(a, b, { loose: true })
}
```

### 33.20 Truncate/Wrap Utilities (`truncate.ts`)

```typescript
/**
 * Truncate path in middle, preserving directory and filename.
 */
export function truncatePathMiddle(path: string, maxLength: number): string {
  if (stringWidth(path) <= maxLength) return path
  if (maxLength <= 0) return '…'
  if (maxLength < 5) return truncateToWidth(path, maxLength)

  const lastSlash = path.lastIndexOf('/')
  const filename = lastSlash >= 0 ? path.slice(lastSlash) : path
  const directory = lastSlash >= 0 ? path.slice(0, lastSlash) : ''
  const filenameWidth = stringWidth(filename)

  if (filenameWidth >= maxLength - 1) return truncateStartToWidth(path, maxLength)

  const availableForDir = maxLength - 1 - filenameWidth
  if (availableForDir <= 0) return truncateStartToWidth(filename, maxLength)

  const truncatedDir = truncateToWidthNoEllipsis(directory, availableForDir)
  return truncatedDir + '…' + filename
}

/**
 * Truncate string to width with ellipsis.
 */
export function truncateToWidth(text: string, maxWidth: number): string {
  if (stringWidth(text) <= maxWidth) return text
  if (maxWidth <= 1) return '…'
  let width = 0
  let result = ''
  for (const { segment } of getGraphemeSegmenter().segment(text)) {
    const segWidth = stringWidth(segment)
    if (width + segWidth > maxWidth - 1) break
    result += segment
    width += segWidth
  }
  return result + '…'
}

/**
 * Truncate from start, keeping tail.
 */
export function truncateStartToWidth(text: string, maxWidth: number): string {
  if (stringWidth(text) <= maxWidth) return text
  if (maxWidth <= 1) return '…'
  const segments = [...getGraphemeSegmenter().segment(text)]
  let width = 0
  let startIdx = segments.length
  for (let i = segments.length - 1; i >= 0; i--) {
    const segWidth = stringWidth(segments[i]!.segment)
    if (width + segWidth > maxWidth - 1) break
    width += segWidth
    startIdx = i
  }
  return '…' + segments.slice(startIdx).map(s => s.segment).join('')
}

/**
 * Wrap text to width.
 */
export function wrapText(text: string, width: number): string[] {
  const lines: string[] = []
  let currentLine = ''
  let currentWidth = 0
  for (const { segment } of getGraphemeSegmenter().segment(text)) {
    const segWidth = stringWidth(segment)
    if (currentWidth + segWidth <= width) {
      currentLine += segment
      currentWidth += segWidth
    } else {
      if (currentLine) lines.push(currentLine)
      currentLine = segment
      currentWidth = segWidth
    }
  }
  if (currentLine) lines.push(currentLine)
  return lines
}
```

---

## 34. Quick Reference - Import Paths

```typescript
// Arrays
import { uniq, count, intersperse } from './utils/array.js'

// Strings  
import { escapeRegExp, capitalize, plural, truncateToLines } from './utils/stringUtils.js'

// Objects
import { objectGroupBy } from './utils/objectGroupBy.js'

// Sets
import { difference, intersects, every, union } from './utils/set.js'

// Async
import { sleep, withTimeout, sequential } from './utils/sleep.js'

// FS
import { getFsImplementation, safeResolvePath, readLinesReverse } from './utils/fsOperations.js'

// Path
import { expandPath, toRelativePath, containsPathTraversal } from './utils/path.js'

// JSON
import { safeParseJSON, parseJSONL, readJSONLFile } from './utils/json.js'

// Errors
import { isAbortError, isENOENT, isFsInaccessible, shortErrorStack } from './utils/errors.js'

// Memoization
import { memoizeWithTTL, memoizeWithTTLAsync, memoizeWithLRU } from './utils/memoize.js'

// Platform
import { getPlatform, getWslVersion, detectVcs } from './utils/platform.js'

// Formatting
import { formatFileSize, formatDuration, formatRelativeTime } from './utils/format.js'

// UUID/ID
import { validateUuid, createAgentId } from './utils/uuid.js'
import { toTaggedId } from './utils/taggedId.js'

// Hash
import { djb2Hash, hashContent, hashPair } from './utils/hash.js'

// Sanitization
import { partiallySanitizeUnicode, recursivelySanitizeUnicode } from './utils/sanitization.js'

// Intl
import { getGraphemeSegmenter, firstGrapheme, lastGrapheme } from './utils/intl.js'

// Stream
import { Stream } from './utils/stream.js'

// Circular Buffer
import { CircularBuffer } from './utils/CircularBuffer.js'

// AbortController
import { createAbortController, createChildAbortController } from './utils/abortController.js'

// Word Slugs
import { generateWordSlug, generateShortWordSlug } from './utils/words.js'

// Semver
import { gt, gte, lt, lte, satisfies, order } from './utils/semver.js'

// Semantic Types
import { semanticBoolean } from './utils/semanticBoolean.js'

// Cursor
import { Cursor, pushToKillRing, canYankPop, yankPop } from './utils/Cursor.js'

// Truncate
import { truncate, truncateToWidth, wrapText, truncatePathMiddle } from './utils/truncate.js'

// XML
import { escapeXml, escapeXmlAttr } from './utils/xml.js'

// Log
import { logError, logMCPError, captureAPIRequest } from './utils/log.js'

// Debug
import { isDebugMode, enableDebugLogging, logForDebugging } from './utils/debug.js'
```

---

## 35. Architecture Notes

### Dependency Graph

```
utils/ (foundation layer)
  ↓
bootstrap/ (global state)
  ↓
services/ (API, telemetry, settings sync)
  ↓
commands/ (user-facing commands)
  ↓
tools/ (MCP, file operations, bash)
```

### Key Design Decisions

1. **FsOperations abstraction**: Allows mocking filesystem for tests and abstracting platform differences
2. **LRU cache bounds**: All caches have size limits to prevent memory leaks
3. **Background refresh**: TTL memoization refreshes stale data without blocking
4. **WeakRef for abort propagation**: Parent controllers don't retain dead children
5. **Grapheme-safe text operations**: All truncate/wrap operations respect Unicode grapheme boundaries
6. **Error type guards**: Centralized error classification replaces instanceof checks
7. **Bun optimization**: Uses Bun-native APIs when available (Bun.hash, Bun.semver, Bun.JSONL)

### Performance Considerations

- `set.ts` and `array.ts` functions are optimized for hot paths (explicit loops vs higher-order functions)
- `memoizeWithLRU` uses LRUCache package for O(1) get/set/delete
- `safeParseJSON` skips caching for inputs >8KB to prevent cache pollution
- `readLinesReverse` uses fixed 4KB chunks to avoid loading entire files into memory
- `getGraphemeSegmenter()` and other Intl constructors are cached to avoid ~0.05-0.1ms initialization cost per call

---

**Document Generated:** 2026-04-07  
**Source Version:** Commit 42fe8c6 (ADD: sources)  
**Total Lines Documented:** 6000+ lines across 329+ TypeScript files  
**Output Location:** `/home/darkvoid/Boxxed/@dev/repo-expolorations/hermes-agent/claude-code-src/utils/exploration.md`
