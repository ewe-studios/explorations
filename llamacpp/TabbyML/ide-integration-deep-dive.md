---
source: /home/darkvoid/Boxxed/@formulas/src.rust/src.llamacpp/src.AICoders/src.TabbyML/tabby
explored_at: 2026-04-02
---

# TabbyML IDE Integration Deep Dive

## Overview

This document explores TabbyML's IDE integration architecture, covering:
- Language Server Protocol (LSP) integration
- VSCode extension architecture
- Chat panel implementation
- Context providers
- Cross-IDE compatibility

## Table of Contents

1. [Extension Architecture](#1-extension-architecture)
2. [Language Server Protocol](#2-language-server-protocol)
3. [Completion Provider](#3-completion-provider)
4. [Chat Implementation](#4-chat-implementation)
5. [Context Providers](#5-context-providers)
6. [Cross-IDE Patterns](#6-cross-ide-patterns)

---

## 1. Extension Architecture

### VSCode Extension Structure

```typescript
// From clients/vscode/src/extension.ts

import * as vscode from 'vscode';
import { TabbyAgent } from './tabby-agent';
import { ChatViewProvider } from './chat/ChatViewProvider';
import { CompletionProvider } from './completion/CompletionProvider';

export async function activate(context: vscode.ExtensionContext) {
    // Initialize agent
    const agent = new TabbyAgent(context);
    await agent.initialize();

    // Register completion provider
    const completionProvider = new CompletionProvider(agent);
    context.subscriptions.push(
        vscode.languages.registerInlineCompletionItemProvider(
            { pattern: '**' },
            completionProvider
        )
    );

    // Register chat provider
    const chatProvider = new ChatViewProvider(context, agent);
    context.subscriptions.push(
        vscode.window.registerWebviewViewProvider(
            'tabby-chat',
            chatProvider
        )
    );

    // Register commands
    context.subscriptions.push(
        vscode.commands.registerCommand('tabby.acceptCompletion', () => {
            completionProvider.acceptCompletion();
        })
    );

    context.subscriptions.push(
        vscode.commands.registerCommand('tabby.triggerCompletion', () => {
            completionProvider.triggerCompletion();
        })
    );

    context.subscriptions.push(
        vscode.commands.registerCommand('tabby.openSettings', () => {
            vscode.commands.executeCommand('workbench.action.openSettings', 'Tabby');
        })
    );
}

export function deactivate() {
    // Cleanup
}
```

### Extension Manifest (package.json)

```json
{
    "name": "vscode-tabby",
    "displayName": "Tabby",
    "publisher": "TabbyML",
    "version": "1.20.0",
    "engines": {
        "vscode": "^1.85.0"
    },
    "activationEvents": [
        "onStartupFinished"
    ],
    "main": "./dist/extension.js",
    "contributes": {
        "viewsContainers": {
            "activitybar": [
                {
                    "id": "tabby",
                    "title": "Tabby",
                    "icon": "resources/icons/tabby-icon.svg"
                }
            ]
        },
        "views": {
            "tabby": [
                {
                    "type": "webview",
                    "id": "tabby-chat",
                    "name": "Chat",
                    "icon": "resources/icons/chat-icon.svg"
                }
            ]
        },
        "commands": [
            {
                "command": "tabby.acceptCompletion",
                "title": "Accept Completion",
                "category": "Tabby"
            },
            {
                "command": "tabby.triggerCompletion",
                "title": "Trigger Completion",
                "category": "Tabby"
            }
        ],
        "keybindings": [
            {
                "command": "tabby.acceptCompletion",
                "key": "Tab",
                "when": "tabbyCompletionVisible"
            },
            {
                "command": "tabby.triggerCompletion",
                "key": "Ctrl+\\",
                "when": "editorTextFocus"
            }
        ],
        "configuration": {
            "title": "Tabby",
            "properties": {
                "tabby.server.endpoint": {
                    "type": "string",
                    "default": "http://localhost:8080",
                    "description": "Tabby server endpoint URL"
                },
                "tabby.completion.triggerMode": {
                    "type": "string",
                    "enum": ["auto", "manual"],
                    "default": "auto",
                    "description": "When to trigger completions"
                },
                "tabby.chat.enabled": {
                    "type": "boolean",
                    "default": true,
                    "description": "Enable chat functionality"
                }
            }
        }
    }
}
```

---

## 2. Language Server Protocol

### LSP Integration

```typescript
// From clients/tabby-agent/src/server.ts

import {
    Connection,
    createConnection,
    TextDocuments,
    TextDocument,
    CompletionParams,
    CompletionList,
    InitializeParams,
    InitializeResult,
} from 'vscode-languageserver';

export class TabbyLanguageServer {
    private connection: Connection;
    private documents: TextDocuments<TextDocument>;
    private completionProvider: CompletionProvider;

    constructor() {
        this.connection = createConnection();
        this.documents = new TextDocuments(TextDocument);

        this.setupHandlers();
        this.documents.listen(this.connection);
    }

    private setupHandlers() {
        // Initialize handler
        this.connection.onInitialize((params: InitializeParams): InitializeResult => {
            const capabilities: InitializeResult['capabilities'] = {
                textDocumentSync: TextDocumentSyncKind.Incremental,
                completionProvider: {
                    resolveProvider: false,
                    triggerCharacters: ['.', '(', '{', '['],
                },
            };

            return { capabilities };
        });

        // Completion handler
        this.connection.onCompletion(
            async (params: CompletionParams): Promise<CompletionList> => {
                const document = this.documents.get(params.textDocument.uri);
                if (!document) return { items: [] };

                const completions = await this.completionProvider.provideCompletions(
                    document,
                    params.position
                );

                return {
                    items: completions.map((c, i) => ({
                        label: c.text,
                        kind: CompletionItemKind.Text,
                        data: { completionId: c.id, index: i },
                    })),
                };
            }
        );

        // Text change handler
        this.documents.onDidChangeContent((change) => {
            this.completionProvider.invalidateCache(change.document);
        });
    }

    public listen() {
        this.connection.listen();
    }
}
```

### Custom LSP Extensions

```typescript
// From clients/tabby-agent/src/protocol.ts

// Custom notification for telemetry
export namespace TelemetryEventNotification {
    export const type = new NotificationType<EventParams>('tabby/telemetry/event');
}

export interface EventParams {
    type: 'view' | 'select';
    completion_id: string;
    elapsed?: number;
}

// Custom request for server health
export namespace HealthCheckRequest {
    export const type = new RequestType<void, HealthCheckResponse, void>('tabby/health');
}

export interface HealthCheckResponse {
    model: string;
    device: string;
    latency_ms?: number;
}
```

---

## 3. Completion Provider

### Core Provider Implementation

```typescript
// From clients/tabby-agent/src/codeCompletion/index.ts

export class CompletionProvider extends EventEmitter {
    private readonly cache = new CompletionCache();
    private readonly debouncer = new CompletionDebouncer();
    private readonly statistics = new CompletionStatisticsTracker();
    private readonly latencyTracker = new LatencyTracker();

    constructor(
        private readonly config: Configurations,
        private readonly apiClient: TabbyApiClient,
        private readonly documents: TextDocuments,
    ) {
        super();
    }

    async provideInlineCompletionItems(
        document: TextDocument,
        position: Position,
        context: InlineCompletionContext,
    ): Promise<InlineCompletionList> {
        // 1. Check if available
        if (!this.isAvailable()) {
            return { items: [] };
        }

        // 2. Build completion context
        const completionContext = await buildCompletionContext({
            document,
            position,
            extraContext: await this.gatherExtraContext(document, position),
        });

        // 3. Check cache
        const cached = await this.cache.get(completionContext);
        if (cached) {
            return cached;
        }

        // 4. Debounce request
        await this.debouncer.debounce(completionContext, async () => {
            this.fetchingCompletion = true;

            try {
                // 5. Build API request
                const request = buildRequest(completionContext, this.config);

                // 6. Fetch from server
                const startTime = Date.now();
                const response = await this.apiClient.completion(request);
                const latency = Date.now() - startTime;

                this.latencyTracker.recordLatency(latency, false);

                // 7. Post-process
                const processed = await postCacheProcess(
                    response.choices,
                    completionContext,
                    this.config.postprocess
                );

                // 8. Cache and return
                await this.cache.set(completionContext, processed);
                this.statistics.recordRequest(response.id);

                this.emit('completionReceived', {
                    document: document.uri,
                    position,
                    items: processed.length,
                });

            } catch (error) {
                this.latencyTracker.recordLatency(0, true);
                this.handleError(error);
            } finally {
                this.fetchingCompletion = false;
            }
        });

        return { items: [] }; // Async completions come later
    }

    private async gatherExtraContext(
        document: TextDocument,
        position: Position,
    ): Promise<CompletionExtraContexts> {
        return {
            declarations: await this.declarationProvider.getSnippets(document, position),
            recentlyChanged: await this.recentlyChangedProvider.search(document.getText()),
            visibleRanges: this.visibleRangesTracker.getVisibleContent(document),
        };
    }

    private handleError(error: Error) {
        if (isRateLimitExceededError(error)) {
            this.rateLimitExceeded = true;
            this.emit('rateLimitExceeded');
        } else if (isCanceledError(error)) {
            // Ignore cancellation
        } else {
            this.logger.error('Completion error:', error);
            this.emit('error', error);
        }
    }
}
```

### Inline Completion Items

```typescript
// From clients/tabby-agent/src/codeCompletion/solution.ts

export interface CompletionResultItem {
    id: string;
    text: string;
    range?: Range;
    replacementCommonLength?: number;
}

export class CompletionSolution {
    private items: CompletionResultItem[];
    private selectedIndex?: number;

    constructor(items: CompletionResultItem[]) {
        this.items = items;
    }

    toInlineCompletions(context: CompletionContext): vscode.InlineCompletionItem[] {
        return this.items.map((item) => {
            const insertionRange = this.calculateInsertionRange(item, context);
            const insertText = this.calculateInsertText(item, context);

            return new vscode.InlineCompletionItem(
                insertText,
                insertionRange,
                {
                    title: 'Record Telemetry',
                    command: 'tabby.recordTelemetry',
                    arguments: [item.id, 'view'],
                }
            );
        });
    }

    private calculateInsertionRange(
        item: CompletionResultItem,
        context: CompletionContext,
    ): vscode.Range {
        const { prefix, suffix } = context;

        // Find overlap with suffix
        const overlap = findOverlap(item.text, suffix);

        return new vscode.Range(
            context.position,
            context.position.translate(0, overlap)
        );
    }

    private calculateInsertText(
        item: CompletionResultItem,
        context: CompletionContext,
    ): string {
        const { suffix } = context;
        const overlap = findOverlap(item.text, suffix);

        // Remove overlapping portion
        return item.text.slice(0, -overlap) || item.text;
    }
}
```

### Accept/Reject Tracking

```typescript
// From clients/tabby-agent/src/codeCompletion/statistics.ts

export class CompletionStatisticsTracker {
    private pendingCompletions = new Map<string, CompletionEntry>();

    recordRequest(completionId: string, items: CompletionResultItem[]) {
        for (const item of items) {
            this.pendingCompletions.set(item.id, {
                completionId,
                itemId: item.id,
                timestamp: Date.now(),
                viewed: true,
                accepted: false,
            });
        }
    }

    recordAccept(itemId: string) {
        const entry = this.pendingCompletions.get(itemId);
        if (entry) {
            entry.accepted = true;
            entry.elapsed = Date.now() - entry.timestamp;

            // Send telemetry
            this.sendTelemetry({
                type: 'select',
                completion_id: entry.completionId,
                elapsed: entry.elapsed,
            });

            this.pendingCompletions.delete(itemId);
        }
    }

    recordReject(completionId: string) {
        // Mark all items from this completion as rejected
        for (const [itemId, entry] of this.pendingCompletions.entries()) {
            if (entry.completionId === completionId) {
                this.pendingCompletions.delete(itemId);
            }
        }
    }

    private sendTelemetry(event: EventParams) {
        this.connection.sendNotification(TelemetryEventNotification.type, event);
    }

    getAcceptanceRate(): number {
        const stats = this.getStatistics();
        if (stats.total === 0) return 0;
        return stats.accepted / stats.total;
    }
}
```

---

## 4. Chat Implementation

### Chat View Provider

```typescript
// From clients/vscode/src/chat/ChatViewProvider.ts

export class ChatViewProvider implements vscode.WebviewViewProvider {
    public static readonly viewType = 'tabby-chat';

    constructor(
        private readonly context: vscode.ExtensionContext,
        private readonly agent: TabbyAgent,
    ) {}

    resolveWebviewView(
        webviewView: vscode.WebviewView,
        context: vscode.WebviewViewResolveContext,
        token: vscode.CancellationToken,
    ): void {
        // Set webview options
        webviewView.webview.options = {
            enableScripts: true,
            localResourceRoots: [this.context.extensionUri],
        };

        // Set HTML content
        webviewView.webview.html = this.getHtmlContent(webviewView.webview);

        // Setup message handling
        webviewView.webview.onDidReceiveMessage(async (message) => {
            switch (message.type) {
                case 'sendMessage':
                    await this.handleSendMessage(message.content);
                    break;
                case 'compactHistory':
                    await this.handleCompactHistory();
                    break;
                case 'selectContext':
                    await this.handleSelectContext(message.context);
                    break;
            }
        });

        // Store for later use
        this.webviewView = webviewView;
    }

    private async handleSendMessage(content: string) {
        // Get conversation history
        const history = this.getHistory();

        // Send to server
        const response = await this.agent.chat.sendMessage({
            messages: [...history, { role: 'user', content }],
            model: this.agent.config.chatModel,
        });

        // Stream response
        for await (const chunk of response.stream) {
            this.webviewView.webview.postMessage({
                type: 'streamChunk',
                content: chunk,
            });
        }

        // Add to history
        this.addToHistory({ role: 'user', content });
        this.addToHistory({ role: 'assistant', content: response.content });
    }

    private getHtmlContent(webview: vscode.Webview): string {
        const styleUri = webview.asWebviewUri(
            vscode.Uri.joinPath(this.context.extensionUri, 'dist', 'chat.css')
        );

        const scriptUri = webview.asWebviewUri(
            vscode.Uri.joinPath(this.context.extensionUri, 'dist', 'chat.js')
        );

        return `
            <!DOCTYPE html>
            <html>
            <head>
                <link rel="stylesheet" href="${styleUri}">
            </head>
            <body>
                <div id="chat-container">
                    <div id="messages"></div>
                    <div id="input-container">
                        <textarea id="message-input" placeholder="Ask Tabby..."></textarea>
                        <button id="send-button">Send</button>
                    </div>
                </div>
                <script src="${scriptUri}"></script>
            </body>
            </html>
        `;
    }
}
```

### Chat Panel Protocol

```typescript
// From clients/tabby-chat-panel/src/index.ts

export interface ChatPanelApi {
    sendMessage(message: ChatMessage): void;
    updateHistory(history: ChatMessage[]): void;
    compactHistory(): void;
    selectContext(context: ChatContext): void;
}

export interface ChatMessage {
    role: 'system' | 'user' | 'assistant';
    content: string;
    timestamp?: number;
}

export interface ChatContext {
    filepath?: string;
    range?: Range;
    content: string;
}

// Events from webview
export interface WebviewEvent {
    type: 'sendMessage' | 'compactHistory' | 'selectContext';
    payload: unknown;
}
```

---

## 5. Context Providers

### Document Context

```typescript
// From clients/tabby-agent/src/contextProviders/documentContexts.ts

export class TextDocumentReader {
    private documents = new Map<string, TextDocument>();

    onDidOpenTextDocument(event: TextDocument) {
        this.documents.set(event.uri.toString(), event);
    }

    onDidChangeTextDocument(event: TextDocumentChangeEvent) {
        this.documents.set(event.document.uri.toString(), event.document);
    }

    onDidCloseTextDocument(event: TextDocument) {
        this.documents.delete(event.uri.toString());
    }

    getDocument(uri: string): TextDocument | undefined {
        return this.documents.get(uri);
    }

    getOpenDocuments(): TextDocument[] {
        return Array.from(this.documents.values());
    }
}
```

### Workspace Context

```typescript
// From clients/tabby-agent/src/contextProviders/workspace.ts

export class WorkspaceContextProvider {
    constructor(private workspaceFolder: vscode.WorkspaceFolder) {}

    async getRelatedFiles(query: string, limit = 10): Promise<FileMatch[]> {
        // Use VSCode's built-in search
        const results = await vscode.workspace.findFiles(
            new vscode.RelativePattern(this.workspaceFolder, '**/*'),
            '**/node_modules/**',
            limit
        );

        // Score by relevance to query
        const scored = results.map(file => ({
            uri: file,
            score: this.calculateRelevance(file.fsPath, query),
        }));

        return scored
            .sort((a, b) => b.score - a.score)
            .slice(0, limit);
    }

    private calculateRelevance(filepath: string, query: string): number {
        const filename = path.basename(filepath);
        const score = {
            exactMatch: filename === query ? 100 : 0,
            containsQuery: filename.includes(query) ? 50 : 0,
            isSourceFile: isSourceFile(filepath) ? 20 : 0,
            recentlyOpened: this.isRecentlyOpened(filepath) ? 10 : 0,
        };

        return Object.values(score).reduce((a, b) => a + b, 0);
    }
}
```

### Git Context

```typescript
// From clients/tabby-agent/src/contextProviders/git.ts

export class GitContextProvider {
    private repositories = new Map<string, GitRepository>();

    async getRepository(uri: vscode.Uri): Promise<GitRepository | undefined> {
        const workspaceFolder = vscode.workspace.getWorkspaceFolder(uri);
        if (!workspaceFolder) return undefined;

        // Check cache
        const cached = this.repositories.get(workspaceFolder.uri.toString());
        if (cached) return cached;

        // Open repository
        try {
            const repo = await git.openRepository(workspaceFolder.uri);
            if (repo) {
                this.repositories.set(workspaceFolder.uri.toString(), repo);
                return repo;
            }
        } catch (error) {
            // Not a git repository
        }

        return undefined;
    }

    async getRemoteUrl(uri: vscode.Uri): Promise<string | undefined> {
        const repo = await this.getRepository(uri);
        if (!repo) return undefined;

        const remotes = await repo.getRemotes();
        const origin = remotes.find(r => r.name === 'origin');

        return origin?.fetchUrl;
    }

    async getCurrentBranch(uri: vscode.Uri): Promise<string | undefined> {
        const repo = await this.getRepository(uri);
        if (!repo) return undefined;

        return repo.getCurrentBranch();
    }
}
```

### Declaration Snippets

```typescript
// From clients/tabby-agent/src/contextProviders/declarationSnippets.ts

export class DeclarationSnippetsProvider {
    constructor(
        private lspClient: LspClient,
        private maxSnippets = 5,
        private maxSnippetLength = 500,
    ) {}

    async getSnippets(
        document: TextDocument,
        position: Position,
    ): Promise<DeclarationSnippet[]> {
        // Get symbol at position
        const symbol = await this.lspClient.getSymbolAtPosition(
            document.uri,
            position
        );

        if (!symbol) return [];

        // Get declaration location
        const locations = await this.lspClient.getDeclaration(
            document.uri,
            position
        );

        if (!locations) return [];

        // Fetch declaration content
        const snippets: DeclarationSnippet[] = [];

        for (const location of locations.slice(0, this.maxSnippets)) {
            const declDoc = await this.getDocument(location.uri);
            if (!declDoc) continue;

            const range = location.range || this.findSymbolRange(declDoc, symbol.name);
            const content = this.truncateContent(
                declDoc.getText(range),
                this.maxSnippetLength
            );

            snippets.push({
                symbol: symbol.name,
                kind: symbol.kind,
                filepath: declDoc.uri.fsPath,
                range,
                content,
            });
        }

        return snippets;
    }

    private truncateContent(content: string, maxLength: number): string {
        if (content.length <= maxLength) return content;

        // Try to cut at a line boundary
        const truncated = content.slice(0, maxLength);
        const lastNewline = truncated.lastIndexOf('\n');

        if (lastNewline > maxLength * 0.8) {
            return truncated.slice(0, lastNewline) + '\n...';
        }

        return truncated + '...';
    }
}
```

### Recently Changed Code

```typescript
// From clients/tabby-agent/src/contextProviders/recentlyChangedCodeSearch.ts

interface EditEntry {
    timestamp: number;
    filepath: string;
    content: string;
    range: Range;
}

export class RecentlyChangedCodeSearch {
    private history: EditEntry[] = [];
    private maxHistory = 100;

    recordEdit(document: TextDocument, range: Range, content: string) {
        this.history.push({
            timestamp: Date.now(),
            filepath: document.uri.fsPath,
            content,
            range,
        });

        // Trim history
        if (this.history.length > this.maxHistory) {
            this.history.shift();
        }
    }

    async search(query: string, limit = 5): Promise<CodeSnippet[]> {
        // Simple text matching
        const matches = this.history
            .filter(entry => entry.content.includes(query))
            .sort((a, b) => b.timestamp - a.timestamp)
            .slice(0, limit);

        return matches.map(entry => ({
            filepath: entry.filepath,
            content: entry.content,
            range: entry.range,
            timestamp: entry.timestamp,
        }));
    }

    getRecentEdits(timeWindow: number = 60000): EditEntry[] {
        // Get edits from last N milliseconds
        const cutoff = Date.now() - timeWindow;
        return this.history.filter(entry => entry.timestamp > cutoff);
    }
}
```

---

## 6. Cross-IDE Patterns

### IntelliJ Plugin

```kotlin
// From clients/intellij/src/main/kotlin/com/tabbyml/intellijtabby/completion/CompletionProvider.kt

class CompletionProvider : InlineCompletionProvider {
    private val agent = TabbyAgent.getInstance()
    private val cache = CompletionCache()

    override fun provideInlineCompletions(
        editor: Editor,
        file: PsiFile,
        context: InlineCompletionContext,
        callback: InlineCompletionCallback
    ) {
        val position = editor.caretModel.offset
        val document = editor.document

        // Build context
        val prefix = document.text.substring(0, position)
        val suffix = document.text.substring(position)

        // Check cache
        val cached = cache.get(prefix, suffix)
        if (cached != null) {
            callback.accept(cached)
            return
        }

        // Request from agent
        agent.requestCompletions(prefix, suffix) { completions ->
            val processed = postProcess(completions, prefix, suffix)
            cache.set(prefix, suffix, processed)

            callback.accept(processed.map { completion ->
                InlineCompletionItem(
                    completion.text,
                    completion.range?.let { Range(it.start, it.end) }
                )
            })
        }
    }

    private fun postProcess(
        completions: List<Completion>,
        prefix: String,
        suffix: String
    ): List<Completion> {
        return completions
            .map { removeRepetitiveBlocks(it) }
            .map { limitScope(it, prefix, suffix) }
            .filter { it.text.isNotBlank() }
    }
}
```

### Vim Plugin

```vim
" From clients/vim/plugin/tabby.vim

function! s:TriggerCompletion() abort
    " Get current buffer content
    let l:lines = getline(1, '$')
    let l:content = join(l:lines, "\n")

    " Get cursor position
    let l:line = line('.')
    let l:col = col('.')

    " Calculate byte offset
    let l:prefix = join(l:lines[:l:line-1], "\n") . "\n" . l:lines[l:line-1][:l:col-2]
    let l:suffix = l:lines[l:line-1][l:col-1:] . "\n" . join(l:lines[l:line:], "\n")

    " Call Python client
    python3 << EOF
import sys
import vim
import requests

prefix = vim.eval('l:prefix')
suffix = vim.eval('l:suffix')

response = requests.post(
    'http://localhost:8080/v1/completions',
    json={
        'prompt': prefix,
        'suffix': suffix,
        'max_tokens': 128,
    }
)

completions = response.json()['choices']
if completions:
    vim.command(f'let l:completion = "{completions[0]["text"]}"')
else:
    vim.command('let l:completion = ""')
EOF

    " Insert completion
    if exists('l:completion') && !empty(l:completion)
        execute 'normal! a' . l:completion
    endif
endfunction

nnoremap <silent> <Tab> :call <SID>TriggerCompletion()<CR>
```

### Common Patterns

```typescript
// Shared completion logic across IDEs

interface IDEAdapter {
    // Document access
    getDocumentContent(): string;
    getCursorPosition(): Position;
    getVisibleRanges(): Range[];

    // Completion insertion
    insertCompletion(text: string, range?: Range): void;
    showCompletionPopup(items: CompletionItem[], selectedIndex: number): void;

    // Event handling
    onDocumentChange(callback: () => void): Disposable;
    onCursorMove(callback: () => void): Disposable;
}

// Base completion provider works with any IDE adapter
class BaseCompletionProvider {
    constructor(private adapter: IDEAdapter) {}

    async triggerCompletion() {
        const content = this.adapter.getDocumentContent();
        const position = this.adapter.getCursorPosition();

        const prefix = content.slice(0, position);
        const suffix = content.slice(position);

        const response = await this.apiClient.completion({
            prompt: prefix,
            suffix: suffix,
        });

        if (response.choices.length > 0) {
            const completion = response.choices[0].text;
            this.adapter.insertCompletion(completion);
        }
    }
}
```

---

## Conclusion

TabbyML's IDE integration demonstrates:
- **LSP-based architecture** for broad IDE support
- **Modular context providers** for flexible context gathering
- **Streaming chat UI** for interactive assistance
- **Telemetry tracking** for measuring effectiveness
- **Cross-IDE patterns** for consistent behavior

The key insight is that **good IDE integration requires understanding both the editor's APIs and the user's workflow** - TabbyML provides both inline completions and chat-based assistance to cover different interaction patterns.
