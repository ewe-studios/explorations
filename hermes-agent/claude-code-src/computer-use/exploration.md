# Computer Use Deep Dive

**Source Directory:** `/home/darkvoid/Boxxed/@formulas/src.rust/src.llamacpp/src.ClaudOpen/claude-code-main/src/utils/computerUse/`

**Related Modules:** `@ant/computer-use-mcp` (npm package), `@ant/computer-use-input` (Rust/enigo), `@ant/computer-use-swift` (macOS native)

---

## Module Overview

The Computer Use module enables Claude to directly control the user's computer through native macOS APIs. This is implemented as an MCP (Model Context Protocol) server that provides tools for:

- **Screen capture** - High-fidelity screenshots via SCContentFilter
- **Mouse control** - Cursor movement, clicks, drags via enigo
- **Keyboard input** - Key presses, text input via enigo
- **Application management** - App launching, frontmost app detection via NSWorkspace
- **Clipboard operations** - Read/write via pbcopy/pbpaste

**Key Architecture Principles:**
1. **macOS Native** - Requires macOS with TCC (Transparency, Consent, Control) permissions
2. **Lock-Based Concurrency** - File-based lock prevents multiple sessions controlling simultaneously
3. **Permission Gating** - User must explicitly grant app access via dialog
4. **Terminal Surrogate** - Terminal.app exempted from hide/capture to prevent photobombing
5. **Runloop Drain** - All HID events processed through CFRunLoop for reliability

---

## Architecture Diagram

```
┌─────────────────┐     ┌──────────────────────────────┐     ┌─────────────────┐
│   Claude Code   │     │   @ant/computer-use-mcp      │     │   macOS Native  │
│   (Tool Call)   │────▶│   (NPM Package)              │────▶│   APIs          │
│                 │     │                              │     │                 │
│ wrapper.tsx     │     │ bindSessionContext()         │     │ - enigo (Rust)  │
│ - buildSession  │     │ - runPermissionDialog()      │     │ - SCContentFilter│
│ - dispatch      │     │ - checkCuLock()              │     │ - NSWorkspace   │
│ - call()        │     │ - acquireCuLock()            │     │ - CGEventTap    │
└─────────────────┘     └──────────────────────────────┘     └─────────────────┘
         │                          │                               │
         │                          │                               │
         ▼                          ▼                               ▼
┌─────────────────────────────────────────────────────────────────────────┐
│                         Computer Use Lock System                        │
│                                                                         │
│  checkCuLock() ──▶ read lock file ──▶ PID alive? ──▶ blocked/free     │
│  acquireCuLock() ──▶ O_EXCL create ──▶ register Esc hotkey            │
│  releaseCuLock() ──▶ unlink ──▶ unregister Esc hotkey                 │
└─────────────────────────────────────────────────────────────────────────┘
```

---

## Directory Structure

### Core Files

| File | Lines | Purpose |
|------|-------|---------|
| `wrapper.tsx` | ~340 | Main `.call()` override, session context binding |
| `executor.ts` | ~550 | CLI ComputerExecutor implementation |
| `toolRendering.tsx` | ~125 | Tool use/result rendering overrides |
| `computerUseLock.ts` | ~220 | File-based lock for concurrency |
| `gates.ts` | ~70 | GrowthBook feature gates |
| `hostAdapter.ts` | ~75 | Host platform adapter |
| `escHotkey.ts` | ~55 | Escape key abort registration |
| `cleanup.ts` | ~85 | Per-turn cleanup, lock release |
| `drainRunLoop.ts` | ~75 | CFRunLoop drain utility |
| `common.ts` | ~65 | Common constants, capabilities |
| `inputLoader.ts` | ~40 | Lazy input module loader |
| `swiftLoader.ts` | ~30 | Lazy Swift module loader |
| `appNames.ts` | ~180 | App bundle ID to display name mapping |
| `setup.ts` | ~55 | Computer Use setup wizard |

---

## Key Components

### 1. Session Context Binding (`wrapper.tsx`)

The `buildSessionContext()` function creates the context object that bridges Claude Code's `ToolUseContext` with the MCP package's `ComputerUseSessionContext`.

```typescript
export function buildSessionContext(): ComputerUseSessionContext {
  return {
    // ── Read state fresh via the per-call ref ─────────────────────────────
    getAllowedApps: () => 
      tuc().getAppState().computerUseMcpState?.allowedApps ?? [],
    
    getGrantFlags: () => 
      tuc().getAppState().computerUseMcpState?.grantFlags ?? DEFAULT_GRANT_FLAGS,
    
    // cc-2 has no Settings page for user-denied apps yet.
    getUserDeniedBundleIds: () => [],
    
    getSelectedDisplayId: () => 
      tuc().getAppState().computerUseMcpState?.selectedDisplayId,
    
    getDisplayPinnedByModel: () => 
      tuc().getAppState().computerUseMcpState?.displayPinnedByModel ?? false,
    
    getDisplayResolvedForApps: () => 
      tuc().getAppState().computerUseMcpState?.displayResolvedForApps,
    
    getLastScreenshotDims: (): ScreenshotDims | undefined => {
      const d = tuc().getAppState().computerUseMcpState?.lastScreenshotDims;
      return d ? {
        ...d,
        displayId: d.displayId ?? 0,
        originX: d.originX ?? 0,
        originY: d.originY ?? 0
      } : undefined;
    },
    
    // ── Write-backs ────────────────────────────────────────────────────────
    onPermissionRequest: (req, _dialogSignal) => runPermissionDialog(req),
    
    // Package does the merge (dedupe + truthy-only flags). We just persist.
    onAllowedAppsChanged: (apps, flags) => 
      tuc().setAppState(prev => {
        const cu = prev.computerUseMcpState;
        const prevApps = cu?.allowedApps;
        const prevFlags = cu?.grantFlags;
        
        const sameApps = prevApps?.length === apps.length && 
          apps.every((a, i) => prevApps[i]?.bundleId === a.bundleId);
        const sameFlags = prevFlags?.clipboardRead === flags.clipboardRead && 
          prevFlags?.clipboardWrite === flags.clipboardWrite && 
          prevFlags?.systemKeyCombos === flags.systemKeyCombos;
        
        return sameApps && sameFlags ? prev : {
          ...prev,
          computerUseMcpState: {
            ...cu,
            allowedApps: [...apps],
            grantFlags: flags
          }
        };
      }),
    
    onAppsHidden: ids => {
      if (ids.length === 0) return;
      tuc().setAppState(prev => {
        const cu = prev.computerUseMcpState;
        const existing = cu?.hiddenDuringTurn;
        if (existing && ids.every(id => existing.has(id))) return prev;
        return {
          ...prev,
          computerUseMcpState: {
            ...cu,
            hiddenDuringTurn: new Set([...(existing ?? []), ...ids])
          }
        };
      });
    },
    
    // Resolver writeback only fires under a pin when Swift fell back to main
    // (pinned display unplugged) — the pin is semantically dead, so clear it
    // and the app-set key so the chase chain runs next time.
    onResolvedDisplayUpdated: id => 
      tuc().setAppState(prev => {
        const cu = prev.computerUseMcpState;
        if (cu?.selectedDisplayId === id && 
            !cu.displayPinnedByModel && 
            cu.displayResolvedForApps === undefined) {
          return prev;
        }
        return {
          ...prev,
          computerUseMcpState: {
            ...cu,
            selectedDisplayId: id,
            displayPinnedByModel: false,
            displayResolvedForApps: undefined
          }
        };
      }),
    
    // switch_display(name) pins; switch_display("auto") unpins
    onDisplayPinned: id => 
      tuc().setAppState(prev => {
        const cu = prev.computerUseMcpState;
        const pinned = id !== undefined;
        const nextResolvedFor = pinned ? cu?.displayResolvedForApps : undefined;
        
        if (cu?.selectedDisplayId === id && 
            cu?.displayPinnedByModel === pinned && 
            cu?.displayResolvedForApps === nextResolvedFor) {
          return prev;
        }
        return {
          ...prev,
          computerUseMcpState: {
            ...cu,
            selectedDisplayId: id,
            displayPinnedByModel: pinned,
            displayResolvedForApps: nextResolvedFor
          }
        };
      }),
    
    onDisplayResolvedForApps: key => 
      tuc().setAppState(prev => {
        const cu = prev.computerUseMcpState;
        if (cu?.displayResolvedForApps === key) return prev;
        return {
          ...prev,
          computerUseMcpState: {
            ...cu,
            displayResolvedForApps: key
          }
        };
      }),
    
    onScreenshotCaptured: dims => 
      tuc().setAppState(prev => {
        const cu = prev.computerUseMcpState;
        const p = cu?.lastScreenshotDims;
        return p?.width === dims.width && p?.height === dims.height && 
               p?.displayWidth === dims.displayWidth && 
               p?.displayHeight === dims.displayHeight && 
               p?.displayId === dims.displayId && 
               p?.originX === dims.originX && 
               p?.originY === dims.originY
          ? prev
          : {
              ...prev,
              computerUseMcpState: {
                ...cu,
                lastScreenshotDims: dims
              }
            };
      }),
    
    // ── Lock — async, direct file-lock calls ───────────────────────────────
    checkCuLock: async () => {
      const c = await checkComputerUseLock();
      switch (c.kind) {
        case 'free':
          return { holder: undefined, isSelf: false };
        case 'held_by_self':
          return { holder: getSessionId(), isSelf: true };
        case 'blocked':
          return { holder: c.by, isSelf: false };
      }
    },
    
    acquireCuLock: async () => {
      const r = await tryAcquireComputerUseLock();
      if (r.kind === 'blocked') {
        throw new Error(formatLockHeld(r.by));
      }
      if (r.fresh) {
        // Global Escape → abort. Consumes the event (PI defense).
        const escRegistered = registerEscHotkey(() => {
          logForDebugging('[cu-esc] user escape, aborting turn');
          tuc().abortController.abort();
        });
        tuc().sendOSNotification?.({
          message: escRegistered 
            ? 'Claude is using your computer · press Esc to stop' 
            : 'Claude is using your computer · press Ctrl+C to stop',
          notificationType: 'computer_use_enter'
        });
      }
    },
    
    formatLockHeldMessage: formatLockHeld
  };
}
```

**Module-Level State (Deliberate Exception):**
```typescript
// Cached binding — built on first `.call()`, reused for process lifetime.
// The dispatcher's closure-held screenshot blob persists across calls.
let binding: Binding | undefined;
let currentToolUseContext: ToolUseContext | undefined;

function tuc(): ToolUseContext {
  return currentToolUseContext!;
}
```

This is a **deliberate exception** to the "no module-scope state" rule (src/CLAUDE.md). The dispatcher closure must persist across calls so its internal screenshot blob survives, but `ToolUseContext` is per-call.

---

### 2. Tool Call Override (`wrapper.tsx`)

The `.call()` override dispatches through the cached binder.

```typescript
type CallOverride = Pick<Tool, 'call'>['call'];

export function getComputerUseMCPToolOverrides(toolName: string): ComputerUseMCPToolOverrides {
  const call: CallOverride = async (args, context: ToolUseContext) => {
    currentToolUseContext = context;
    const { dispatch } = getOrBind();
    
    const { telemetry, ...result } = await dispatch(toolName, args);
    
    if (telemetry?.error_kind) {
      logForDebugging(
        `[Computer Use MCP] ${toolName} error_kind=${telemetry.error_kind}`
      );
    }

    // MCP content blocks → Anthropic API blocks.
    // CU only produces text and pre-sized JPEG.
    const data = Array.isArray(result.content)
      ? result.content.map(item => 
          item.type === 'image' 
            ? {
                type: 'image' as const,
                source: {
                  type: 'base64' as const,
                  media_type: item.mimeType ?? 'image/jpeg',
                  data: item.data
                }
              }
            : {
                type: 'text' as const,
                text: item.type === 'text' ? item.text : ''
              }
        )
      : result.content;
    
    return { data };
  };
  
  return {
    ...getComputerUseMCPRenderingOverrides(toolName),
    call
  };
}
```

---

### 3. Permission Dialog (`wrapper.tsx`)

Renders approval dialog mid-call via `setToolJSX`.

```typescript
async function runPermissionDialog(
  req: CuPermissionRequest
): Promise<CuPermissionResponse> {
  const context = tuc();
  const setToolJSX = context.setToolJSX;
  
  if (!setToolJSX) {
    // Shouldn't happen — main.tsx gate excludes non-interactive.
    return {
      granted: [],
      denied: [],
      flags: DEFAULT_GRANT_FLAGS
    };
  }
  
  try {
    return await new Promise<CuPermissionResponse>((resolve, reject) => {
      const signal = context.abortController.signal;
      
      // If already aborted, addEventListener won't fire — reject now.
      if (signal.aborted) {
        reject(new Error('Computer Use permission dialog aborted'));
        return;
      }
      
      const onAbort = (): void => {
        signal.removeEventListener('abort', onAbort);
        reject(new Error('Computer Use permission dialog aborted'));
      };
      
      signal.addEventListener('abort', onAbort);
      
      setToolJSX({
        jsx: React.createElement(ComputerUseApproval, {
          request: req,
          onDone: (resp: CuPermissionResponse) => {
            signal.removeEventListener('abort', onAbort);
            resolve(resp);
          }
        }),
        shouldHidePromptInput: true
      });
    });
  } finally {
    setToolJSX(null);
  }
}
```

---

### 4. Computer Use Lock (`computerUseLock.ts`)

File-based lock prevents multiple sessions from controlling simultaneously.

```typescript
const LOCK_FILENAME = 'computer-use.lock';

type ComputerUseLock = {
  readonly sessionId: string;
  readonly pid: number;
  readonly acquiredAt: number;
};

export type AcquireResult =
  | { readonly kind: 'acquired'; readonly fresh: boolean }
  | { readonly kind: 'blocked'; readonly by: string };

export type CheckResult =
  | { readonly kind: 'free' }
  | { readonly kind: 'held_by_self' }
  | { readonly kind: 'blocked'; readonly by: string };

/**
 * Check lock state without acquiring.
 * Does stale-PID recovery (unlinks) so a dead session's lock doesn't block.
 */
export async function checkComputerUseLock(): Promise<CheckResult> {
  const existing = await readLock();
  if (!existing) return { kind: 'free' };
  if (existing.sessionId === getSessionId()) return { kind: 'held_by_self' };
  if (isProcessRunning(existing.pid)) {
    return { kind: 'blocked', by: existing.sessionId };
  }
  
  logForDebugging(
    `Recovering stale computer-use lock from session ${existing.sessionId} (PID ${existing.pid})`
  );
  await unlink(getLockPath()).catch(() => {});
  return { kind: 'free' };
}

/**
 * Zero-syscall check: does THIS process believe it holds the lock?
 */
export function isLockHeldLocally(): boolean {
  return unregisterCleanup !== undefined;
}

/**
 * Try to acquire the computer-use lock for the current session.
 * Uses O_EXCL (open 'wx') for atomic test-and-set.
 */
export async function tryAcquireComputerUseLock(): Promise<AcquireResult> {
  const sessionId = getSessionId();
  const lock: ComputerUseLock = {
    sessionId,
    pid: process.pid,
    acquiredAt: Date.now(),
  };

  await mkdir(getClaudeConfigHomeDir(), { recursive: true });

  // Fresh acquisition.
  if (await tryCreateExclusive(lock)) {
    registerLockCleanup();
    return FRESH;
  }

  const existing = await readLock();

  // Corrupt/unparseable — treat as stale.
  if (!existing) {
    await unlink(getLockPath()).catch(() => {});
    if (await tryCreateExclusive(lock)) {
      registerLockCleanup();
      return FRESH;
    }
    return { kind: 'blocked', by: (await readLock())?.sessionId ?? 'unknown' };
  }

  // Already held by this session.
  if (existing.sessionId === sessionId) return REENTRANT;

  // Another live session holds it — blocked.
  if (isProcessRunning(existing.pid)) {
    return { kind: 'blocked', by: existing.sessionId };
  }

  // Stale lock — recover. Unlink then retry.
  logForDebugging(
    `Recovering stale computer-use lock from session ${existing.sessionId} (PID ${existing.pid})`
  );
  await unlink(getLockPath()).catch(() => {});
  if (await tryCreateExclusive(lock)) {
    registerLockCleanup();
    return FRESH;
  }
  return { kind: 'blocked', by: (await readLock())?.sessionId ?? 'unknown' };
}

/**
 * Release the computer-use lock if the current session owns it.
 */
export async function releaseComputerUseLock(): Promise<boolean> {
  // ... unlink implementation
}
```

**Lock States:**
| State | Description | Next Action |
|-------|-------------|-------------|
| `free` | No lock file | Acquire |
| `held_by_self` | Current session holds | Re-entrant call |
| `blocked` | Another session holds | Error, wait |

---

### 5. CLI Executor (`executor.ts`)

Implements the `ComputerExecutor` interface for CLI.

```typescript
import type {
  ComputerExecutor,
  DisplayGeometry,
  FrontmostApp,
  InstalledApp,
  ResolvePrepareCaptureResult,
  RunningApp,
  ScreenshotResult,
} from '@ant/computer-use-mcp';

import { API_RESIZE_PARAMS, targetImageSize } from '@ant/computer-use-mcp';
import { execFileNoThrow } from '../execFileNoThrow.js';
import { requireComputerUseInput } from './inputLoader.js';
import { requireComputerUseSwift } from './swiftLoader.js';

const SCREENSHOT_JPEG_QUALITY = 0.75;

/** Logical → physical → API target dims. */
function computeTargetDims(
  logicalW: number,
  logicalH: number,
  scaleFactor: number,
): [number, number] {
  const physW = Math.round(logicalW * scaleFactor);
  const physH = Math.round(logicalH * scaleFactor);
  return targetImageSize(physW, physH, API_RESIZE_PARAMS);
}

async function readClipboardViaPbpaste(): Promise<string> {
  const { stdout, code } = await execFileNoThrow('pbpaste', [], {
    useCwd: false,
  });
  if (code !== 0) {
    throw new Error(`pbpaste exited with code ${code}`);
  }
  return stdout;
}

async function writeClipboardViaPbcopy(text: string): Promise<void> {
  const { code } = await execFileNoThrow('pbcopy', [], {
    input: text,
    useCwd: false,
  });
  if (code !== 0) {
    throw new Error(`pbcopy exited with code ${code}`);
  }
}

/**
 * Instant move, then 50ms — an input→HID→AppKit→NSEvent round-trip.
 */
const MOVE_SETTLE_MS = 50;

async function moveAndSettle(
  input: Input,
  x: number,
  y: number,
): Promise<void> {
  await input.moveMouse(x, y, false);
  await sleep(MOVE_SETTLE_MS);
}

/**
 * Release `pressed` in reverse (last pressed = first released).
 */
async function releasePressed(input: Input, pressed: string[]): Promise<void> {
  let k: string | undefined;
  while ((k = pressed.pop()) !== undefined) {
    try {
      await input.key(k, 'release');
    } catch {
      // Swallow — best-effort release.
    }
  }
}

/**
 * Bracket `fn()` with modifier press/release.
 */
async function withModifiers<T>(
  input: Input,
  mods: string[],
  fn: () => Promise<T>,
): Promise<T> {
  const pressed: string[] = [];
  try {
    for (const m of mods) {
      await input.key(m, 'press');
      pressed.push(m);
    }
    return await fn();
  } finally {
    await releasePressed(input, pressed);
  }
}

/**
 * Port of Cowork's `typeViaClipboard`.
 * Sequence: save → write → verify → Cmd+V → sleep → restore
 */
async function typeViaClipboard(input: Input, text: string): Promise<void> {
  let saved: string | undefined;
  try {
    saved = await readClipboardViaPbpaste();
  } catch {
    logForDebugging(
      '[computer-use] pbpaste before paste failed; proceeding without restore'
    );
  }

  try {
    await writeClipboardViaPbcopy(text);
    if ((await readClipboardViaPbpaste()) !== text) {
      throw new Error('Clipboard write did not round-trip.');
    }
    await input.keys(['command', 'v']);
    await sleep(100);
  } finally {
    if (typeof saved === 'string') {
      try {
        await writeClipboardViaPbcopy(saved);
      } catch {
        logForDebugging('[computer-use] clipboard restore after paste failed');
      }
    }
  }
}

/**
 * Ease-out-cubic at 60fps; distance-proportional duration at 2000 px/sec.
 */
async function animatedMove(
  input: Input,
  targetX: number,
  targetY: number,
  mouseAnimationEnabled: boolean,
): Promise<void> {
  if (!mouseAnimationEnabled) {
    await moveAndSettle(input, targetX, targetY);
    return;
  }
  const start = await input.mouseLocation();
  const deltaX = targetX - start.x;
  const deltaY = targetY - start.y;
  const distance = Math.hypot(deltaX, deltaY);
  if (distance < 1) return;
  
  const durationSec = Math.min(distance / 2000, 0.5);
  if (durationSec < 0.03) {
    await moveAndSettle(input, targetX, targetY);
    return;
  }
  
  const frameRate = 60;
  const frameIntervalMs = 1000 / frameRate;
  const totalFrames = Math.floor(durationSec * frameRate);
  
  for (let frame = 1; frame <= totalFrames; frame++) {
    const t = frame / totalFrames;
    const eased = 1 - Math.pow(1 - t, 3);
    await input.moveMouse(
      Math.round(start.x + deltaX * eased),
      Math.round(start.y + deltaY * eased),
      false,
    );
    if (frame < totalFrames) {
      await sleep(frameIntervalMs);
    }
  }
  await sleep(MOVE_SETTLE_MS);
}
```

---

### 6. Terminal as Surrogate Host (`executor.ts`)

Terminal.app is exempted from hide/capture to prevent photobombing.

```typescript
const CLI_HOST_BUNDLE_ID = 'com.anthropic.claude-code-cli';

function getTerminalBundleId(): string | null {
  // Detect common terminal emulators
  const terminals = [
    'com.apple.Terminal',
    'com.googlecode.iterm2',
    'org.alacritty',
    'net.kovidgoyal.kitty',
    'com.microsoft.VSCode', // Integrated terminal
  ];
  
  // Check if any terminal is frontmost
  // ... detection logic
  
  return detectedTerminal || null;
}

export function createCliExecutor(opts: {
  getMouseAnimationEnabled: () => boolean;
  getHideBeforeActionEnabled: () => boolean;
}): ComputerExecutor {
  if (process.platform !== 'darwin') {
    throw new Error(
      `createCliExecutor called on ${process.platform}. Computer control is macOS-only.`
    );
  }

  const cu = requireComputerUseSwift();
  const terminalBundleId = getTerminalBundleId();
  const surrogateHost = terminalBundleId ?? CLI_HOST_BUNDLE_ID;
  
  // Swift 0.2.1's captureExcluding takes an ALLOW list.
  // Terminal isn't in user's grants so naturally excluded, but strip it
  // so terminal never photobombs a screenshot.
  const withoutTerminal = (allowed: readonly string[]): string[] =>
    terminalBundleId === null
      ? [...allowed]
      : allowed.filter(id => id !== terminalBundleId);

  logForDebugging(
    terminalBundleId
      ? `[computer-use] terminal ${terminalBundleId} → surrogate host (hide-exempt, activate-skip, screenshot-excluded)`
      : '[computer-use] terminal not detected; falling back to sentinel host'
  );

  return {
    capabilities: {
      ...CLI_CU_CAPABILITIES,
      hostBundleId: CLI_HOST_BUNDLE_ID,
    },
    // ... executor methods
  };
}
```

---

### 7. Escape Hotkey (`escHotkey.ts`)

Registers global Escape key to abort computer use.

```typescript
import { CGEventTap, CFRunLoopSource } from '@ant/computer-use-swift';

let tap: CGEventTap | undefined;
let runLoopSource: CFRunLoopSource | undefined;

export function registerEscHotkey(onEscape: () => void): boolean {
  try {
    tap = new CGEventTap({
      events: ['keyDown'],
      callback: (event) => {
        if (event.keyCode === 53) { // Escape
          onEscape();
          return null; // Consume event (PI defense)
        }
        return event;
      }
    });
    
    runLoopSource = tap.createRunLoopSource();
    CFRunLoop.addSource(runLoopSource, CFRunLoop.defaultMode());
    
    return true;
  } catch (e) {
    logForDebugging('[escHotkey] failed to register Escape hotkey', e);
    return false;
  }
}

export function unregisterEscHotkey(): void {
  if (runLoopSource) {
    CFRunLoop.removeSource(runLoopSource, CFRunLoop.defaultMode());
    runLoopSource = undefined;
  }
  if (tap) {
    tap.disable();
    tap = undefined;
  }
}

export function notifyExpectedEscape(): void {
  // Inform user that Escape will abort
  tuc().sendOSNotification?.({
    message: 'Press Esc to stop Claude',
    notificationType: 'computer_use_escape_hint'
  });
}
```

**Escape Key Properties:**
- **Event Consumption** - Escape event consumed, not propagated (prompt injection defense)
- **CGEventTap** - Low-level event tap, works even when other apps focused
- **CFRunLoop** - Runloop source processed by `drainRunLoop()` pump

---

### 8. Tool Rendering (`toolRendering.tsx`)

Custom rendering for Computer Use tool messages.

```typescript
const RESULT_SUMMARY: Readonly<Partial<Record<string, string>>> = {
  screenshot: 'Captured',
  zoom: 'Captured',
  request_access: 'Access updated',
  left_click: 'Clicked',
  right_click: 'Clicked',
  middle_click: 'Clicked',
  double_click: 'Clicked',
  triple_click: 'Clicked',
  type: 'Typed',
  key: 'Pressed',
  hold_key: 'Pressed',
  scroll: 'Scrolled',
  left_click_drag: 'Dragged',
  open_application: 'Opened'
};

export function getComputerUseMCPRenderingOverrides(toolName: string) {
  return {
    userFacingName() {
      return `Computer Use[${toolName}]`;
    },
    
    renderToolUseMessage(input: CuToolInput) {
      switch (toolName) {
        case 'screenshot':
        case 'left_mouse_down':
        case 'left_mouse_up':
        case 'cursor_position':
        case 'list_granted_applications':
        case 'read_clipboard':
          return ''; // Hide entire row
        
        case 'left_click':
        case 'right_click':
        case 'middle_click':
        case 'double_click':
        case 'triple_click':
        case 'mouse_move':
          return fmtCoord(input.coordinate);
        
        case 'left_click_drag':
          return input.start_coordinate 
            ? `${fmtCoord(input.start_coordinate)} → ${fmtCoord(input.coordinate)}` 
            : `to ${fmtCoord(input.coordinate)}`;
        
        case 'type':
          return typeof input.text === 'string' 
            ? `"${truncateToWidth(input.text, 40)}"` 
            : '';
        
        case 'key':
        case 'hold_key':
          return typeof input.text === 'string' ? input.text : '';
        
        case 'scroll':
          return [
            input.direction, 
            input.amount && `×${input.amount}`, 
            input.coordinate && `at ${fmtCoord(input.coordinate)}`
          ].filter(Boolean).join(' ');
        
        case 'zoom':
          const r = input.region;
          return Array.isArray(r) && r.length === 4 
            ? `[${r[0]}, ${r[1]}, ${r[2]}, ${r[3]}]` 
            : '';
        
        case 'wait':
          return typeof input.duration === 'number' ? `${input.duration}s` : '';
        
        case 'write_clipboard':
          return typeof input.text === 'string' 
            ? `"${truncateToWidth(input.text, 40)}"` 
            : '';
        
        case 'open_application':
          return typeof input.bundle_id === 'string' 
            ? String(input.bundle_id) 
            : '';
        
        case 'request_access':
          const apps = input.apps;
          if (!Array.isArray(apps)) return '';
          const names = apps
            .map(a => typeof a?.displayName === 'string' ? a.displayName : '')
            .filter(Boolean);
          return names.join(', ');
        
        case 'computer_batch':
          const actions = input.actions;
          return Array.isArray(actions) ? `${actions.length} actions` : '';
        
        default:
          return '';
      }
    },
    
    renderToolResultMessage(output, _progress, { verbose }) {
      if (verbose || typeof output !== 'object' || output === null) return null;
      
      const summary = RESULT_SUMMARY[toolName];
      if (!summary) return null;
      
      return (
        <MessageResponse height={1}>
          <Text dimColor>{summary}</Text>
        </MessageResponse>
      );
    }
  };
}
```

---

### 9. Feature Gates (`gates.ts`)

GrowthBook feature flags for Computer Use.

```typescript
export function checkComputerUseEnabled(): boolean {
  return checkGate_CACHED_OR_BLOCKING('tengu_malort_pedway');
}

export function getChicagoCoordinateMode(): 'chicago' | 'legacy' {
  // Chicago mode: coordinate system changes
  return checkGate_CACHED_OR_BLOCKING('chicago_coordinates') 
    ? 'chicago' 
    : 'legacy';
}

export function getHideBeforeActionEnabled(): boolean {
  return checkGate_CACHED_OR_BLOCKING('hide_before_action');
}

export function getMouseAnimationEnabled(): boolean {
  return checkGate_CACHED_OR_BLOCKING('mouse_animation');
}
```

**Feature Flags:**
| Flag | Description | Default |
|------|-------------|---------|
| `tengu_malort_pedway` | Master Computer Use gate | Varies |
| `chicago_coordinates` | New coordinate system | Varies |
| `hide_before_action` | Hide terminal before action | true |
| `mouse_animation` | Animated mouse movement | true |

---

## Tool Rendering Reference

| Tool | Display | Args Shown | Result |
|------|---------|------------|--------|
| `screenshot` | Computer Use[screenshot] | (hidden) | Captured |
| `left_click` | Computer Use[left_click] | (x, y) | Clicked |
| `type` | Computer Use[type] | "text..." | Typed |
| `key` | Computer Use[key] | command | Pressed |
| `scroll` | Computer Use[scroll] | down ×2 at (x, y) | Scrolled |
| `open_application` | Computer Use[open_application] | com.apple.Safari | Opened |
| `request_access` | Computer Use[request_access] | App1, App2 | Access updated |

---

## Integration Points

### With MCP System
- `@ant/computer-use-mcp` - Core MCP package
- `bindSessionContext()` - Context binding
- `defersLockAcquire` - Lock acquisition deferral

### With AppState
- `computerUseMcpState` - State management
- `allowedApps` - Granted applications
- `grantFlags` - Permission flags (clipboardRead, clipboardWrite, systemKeyCombos)
- `selectedDisplayId` - Pinned display
- `lastScreenshotDims` - Last screenshot dimensions

### With macOS Native
- `@ant/computer-use-input` - Rust/enigo for HID
- `@ant/computer-use-swift` - SCContentFilter, NSWorkspace
- `CGEventTap` - Global event monitoring
- `CFRunLoop` - Event pump

---

## Error Handling

### Lock Errors
```typescript
function formatLockHeld(holder: string): string {
  return `Computer use is in use by another Claude session (${holder.slice(0, 8)}...). Wait for that session to finish or run /exit there.`;
}
```

### Permission Errors
- TCC permission denied
- Screen Recording not granted
- Accessibility not granted

### Executor Errors
- `pbpaste`/`pbcopy` failures
- App launch failures
- Display not found

---

## CLI Deltas from Cowork Reference

| Feature | Cowork | CLI |
|---------|--------|-----|
| Click-through | `setIgnoreMouseEvents(true)` | N/A (no window) |
| Clipboard | Electron `clipboard` module | `pbcopy`/`pbpaste` |
| Host bundle ID | `com.copilot.chat` | `com.anthropic.claude-code-cli` |
| Terminal surrogate | N/A | Detected terminal exempted |

---

## Related Files

**Module Documentation:**
- [mcpValidation.md](../mcpValidation.md) - MCP tool validation
- [cleanup.md](../cleanup.md) - Cleanup registry

**Related Modules:**
- [components/ComputerUseApproval.md](../components/ComputerUseApproval.md) - Permission dialog
- [services/analytics/growthbook.md](../services/analytics/growthbook.md) - Feature flags

**External Packages:**
- `@ant/computer-use-mcp` - Core MCP package
- `@ant/computer-use-input` - HID input (Rust/enigo)
- `@ant/computer-use-swift` - macOS native APIs

---

## Code Flow Summary

### Computer Use Turn Flow

```
1. Model calls: mcp__computer-use__left_click({coordinate: [100, 200]})
                │
2. getComputerUseMCPToolOverrides('left_click').call()
                │
3. currentToolUseContext = context
                │
4. getOrBind() ──┬── first call ──▶ bindSessionContext()
                │                  │
                │                  └──▶ buildSessionContext()
                │                     │
                │                     └──▶ checkCuLock()
                │                     │
                │                     └──▶ acquireCuLock()
                │                        │
                │                        ├──▶ O_EXCL create lock
                │                        │
                │                        └──▶ registerEscHotkey()
                │                           │
                │                           └──▶ sendOSNotification()
                │
5. dispatch('left_click', args)
                │
6. Executor.left_click([100, 200])
                │
                ├──▶ moveMouse(100, 200)
                │
                └──▶ mouseButton('left', 'press')
                │
                └──▶ mouseButton('left', 'release')
                │
7. Return { content: [{type: 'text', text: 'Clicked at (100, 200)'}] }
                │
8. MCP → Anthropic API block conversion
                │
9. Per-turn cleanup (cleanup.ts)
                │
                └──▶ releaseComputerUseLock()
```

### Permission Request Flow

```
1. Model requests access to app
                │
2. MCP package calls onPermissionRequest(req)
                │
3. runPermissionDialog(req)
                │
4. setToolJSX({jsx: <ComputerUseApproval request={req} />})
                │
5. User sees dialog: "Claude wants to control Safari"
                │
                ├──▶ Allow ──▶ onDone({granted: [Safari], denied: []})
                │              │
                │              └──▶ resolve(resp)
                │
                └──▶ Deny ──▶ onDone({granted: [], denied: [Safari]})
                               │
                               └──▶ resolve(resp)
                │
6. setToolJSX(null)
                │
7. onAllowedAppsChanged(apps, flags) → persist to AppState
```

---

*Deep dive created: 2026-04-07*
