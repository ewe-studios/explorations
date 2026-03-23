# Agent-Browser - Deep Dive Exploration

## Overview

**Agent-Browser** is a headless browser automation CLI specifically designed for AI agents. It provides a fast Rust CLI that communicates with a Node.js daemon running Playwright.

**Source:** `/home/darkvoid/Boxxed/@formulas/src.rust/src.llamacpp/src.AICoders/src.VarcelLabs/agent-browser`

---

## Architecture

### Client-Daemon Model

```
┌─────────────────┐     IPC (JSON)     ┌─────────────────┐
│   Rust CLI      │ ←─────────────────→ │  Node.js Daemon │
│   (Fast CLI)    │                     │  (Playwright)   │
└─────────────────┘                     └────────┬────────┘
                                                  │
                                                  ↓
                                         ┌───────────────┐
                                         │   Chromium    │
                                         │   (Headless)  │
                                         └───────────────┘
```

### Component Breakdown

**1. Rust CLI (`cli/src/main.rs`)**
- Command parsing with custom flag handling
- Session management via PID files
- IPC communication with daemon
- JSON output mode for agents

**2. Node.js Daemon (`src/daemon.ts`)**
- Playwright browser management
- Command execution via `src/actions.ts`
- Session isolation
- CDP connection support

**3. Browser Manager (`src/browser.ts`)**
- `BrowserManager` class wraps Playwright
- Multi-tab/window support
- CDP session management
- Screencast streaming

**4. Snapshot System (`src/snapshot.ts`)**
- ARIA tree parsing
- Ref generation for elements
- Interactive-only filtering
- Compact mode for reduced output

---

## Key Features

### 1. Ref-Based Element Selection

Instead of fragile CSS selectors, agent-browser generates deterministic refs:

```bash
# Get snapshot with refs
$ agent-browser snapshot
- heading "Example Domain" [ref=e1] [level=1]
- button "Submit" [ref=e2]
- textbox "Email" [ref=e3]

# Use refs to interact
$ agent-browser click @e2
$ agent-browser fill @e3 "test@example.com"
```

**How Refs Work (`src/snapshot.ts`):**

```typescript
export interface RefMap {
  [ref: string]: {
    selector: string;      // getByRole('button', { name: "Submit", exact: true })
    role: string;          // 'button'
    name?: string;         // 'Submit'
    nth?: number;          // Index for duplicates
  };
}

// Generate refs from ARIA tree
export async function getEnhancedSnapshot(
  page: Page,
  options: SnapshotOptions = {}
): Promise<EnhancedSnapshot> {
  resetRefs();
  const refs: RefMap = {};
  const ariaTree = await page.locator(':root').ariaSnapshot();
  const enhancedTree = processAriaTree(ariaTree, refs, options);
  return { tree: enhancedTree, refs };
}
```

**Ref Categories:**
- `INTERACTIVE_ROLES` - buttons, links, inputs (get refs)
- `CONTENT_ROLES` - headings, cells, list items (get refs if named)
- `STRUCTURAL_ROLES` - divs, groups, lists (filtered in compact mode)

### 2. Session Isolation

```bash
# Different isolated sessions
$ agent-browser --session agent1 open site-a.com
$ agent-browser --session agent2 open site-b.com

# List sessions
$ agent-browser session list
Active sessions:
→ default
  agent1
  agent2
```

**Implementation (`cli/src/main.rs`):**

```rust
fn run_session(args: &[String], session: &str, json_mode: bool) {
    let tmp = env::temp_dir();
    // Look for socket files: agent-browser-{session}.pid
    if let Ok(entries) = fs::read_dir(&tmp) {
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().to_string();
            if name.starts_with("agent-browser-") && name.ends_with(".pid") {
                // Check if process is running
                let pid_str = fs::read_to_string(&pid_path)?;
                let pid = pid_str.trim().parse::<u32>()?;
                #[cfg(unix)]
                let running = unsafe { libc::kill(pid as i32, 0) == 0 };
            }
        }
    }
}
```

Each session has:
- Independent browser instance
- Separate cookies/storage
- Isolated navigation history

### 3. CDP Mode

Connect to existing browsers via Chrome DevTools Protocol:

```bash
# Connect to Electron app
$ agent-browser --cdp 9222 snapshot

# Connect to Chrome with remote debugging
$ google-chrome --remote-debugging-port=9222
$ agent-browser --cdp 9222 open about:blank
```

**Implementation (`src/browser.ts`):**

```typescript
private async connectViaCDP(cdpPort: number): Promise<void> {
  const browser = await chromium.connectOverCDP(`http://localhost:${cdpPort}`);

  const contexts = browser.contexts();
  const allPages = contexts.flatMap(c => c.pages());

  this.browser = browser;
  this.cdpPort = cdpPort;

  for (const context of contexts) {
    this.contexts.push(context);
    this.setupContextTracking(context);
  }
}
```

### 4. Screencast Streaming

Stream viewport via WebSocket for live preview:

```bash
$ AGENT_BROWSER_STREAM_PORT=9223 agent-browser open example.com
```

**WebSocket Protocol:**

```json
// Receive frames
{
  "type": "frame",
  "data": "<base64-jpeg>",
  "metadata": {
    "deviceWidth": 1280,
    "deviceHeight": 720,
    "pageScaleFactor": 1,
    "scrollOffsetX": 0,
    "scrollOffsetY": 0
  }
}

// Send mouse events
{
  "type": "input_mouse",
  "eventType": "mousePressed",
  "x": 100,
  "y": 200,
  "button": "left"
}
```

**Implementation (`src/browser.ts`):**

```typescript
async startScreencast(
  callback: (frame: ScreencastFrame) => void,
  options?: ScreencastOptions
): Promise<void> {
  const cdp = await this.getCDPSession();
  this.frameCallback = callback;
  this.screencastActive = true;

  this.screencastFrameHandler = async (params: any) => {
    const frame: ScreencastFrame = {
      data: params.data,
      metadata: params.metadata,
      sessionId: params.sessionId,
    };
    await cdp.send('Page.screencastFrameAck', { sessionId: params.sessionId });
    if (this.frameCallback) this.frameCallback(frame);
  };

  cdp.on('Page.screencastFrame', this.screencastFrameHandler);
  await cdp.send('Page.startScreencast', {
    format: options?.format ?? 'jpeg',
    quality: options?.quality ?? 80,
  });
}
```

### 5. Scoped HTTP Headers

Set headers for specific origins (not leaked to other domains):

```bash
$ agent-browser open api.example.com \
  --headers '{"Authorization": "Bearer <token>"}'
```

**Implementation (`src/browser.ts`):**

```typescript
async setScopedHeaders(origin: string, headers: Record<string, string>): Promise<void> {
  const page = this.getPage();

  // Build URL pattern: "**://api.example.com/**"
  let urlPattern: string;
  try {
    const url = new URL(origin.startsWith('http') ? origin : `https://${origin}`);
    urlPattern = `**://${url.host}/**`;
  } catch {
    urlPattern = `**://${origin}/**`;
  }

  const handler = async (route: Route) => {
    const requestHeaders = route.request().headers();
    await route.continue({
      headers: { ...requestHeaders, ...headers },
    });
  };

  this.scopedHeaderRoutes.set(urlPattern, handler);
  await page.route(urlPattern, handler);
}
```

---

## Command Reference

### Core Commands

| Command | Description | Example |
|---------|-------------|---------|
| `open <url>` | Navigate to URL | `open example.com` |
| `snapshot` | Get accessibility tree with refs | `snapshot -i -c` |
| `click <sel>` | Click element | `click @e2` |
| `fill <sel> <text>` | Fill input | `fill @e3 "test"` |
| `type <sel> <text>` | Type into element | `type @e3 "test"` |
| `press <key>` | Press keyboard key | `press Enter` |
| `screenshot [path]` | Take screenshot | `screenshot page.png --full` |
| `close` | Close browser | `close` |

### Find Commands (Semantic Locators)

```bash
agent-browser find role button click --name "Submit"
agent-browser find text "Sign In" click
agent-browser find label "Email" fill "test@test.com"
agent-browser find first ".item" click
agent-browser find nth 2 "a" text
```

### Get Commands

```bash
agent-browser get text @e1        # Get text content
agent-browser get html @e2        # Get innerHTML
agent-browser get value @e3       # Get input value
agent-browser get attr @e4 href   # Get attribute
agent-browser get title           # Get page title
agent-browser get url             # Get current URL
agent-browser get count ".item"   # Count elements
agent-browser get box @e1         # Get bounding box
```

### Wait Commands

```bash
agent-browser wait @e2                # Wait for element
agent-browser wait 1000               # Wait 1 second
agent-browser wait --text "Welcome"   # Wait for text
agent-browser wait --url "**/dash"    # Wait for URL
agent-browser wait --load networkidle # Wait for load state
agent-browser wait --fn "window.ready" # Wait for JS condition
```

### Mouse/Keyboard

```bash
agent-browser mouse move 100 200
agent-browser mouse down left
agent-browser mouse up left
agent-browser mouse wheel 100       # Scroll down

agent-browser keydown Control
agent-browser keyup Control
```

### Storage

```bash
agent-browser cookies               # Get all cookies
agent-browser cookies set name val  # Set cookie
agent-browser cookies clear         # Clear cookies

agent-browser storage local         # Get localStorage
agent-browser storage local set k v # Set value
agent-browser storage local clear   # Clear all
```

---

## Rust Implementation Details

### CLI Main Loop (`cli/src/main.rs`)

```rust
fn main() {
    let args: Vec<String> = env::args().skip(1).collect();
    let flags = parse_flags(&args);
    let clean = clean_args(&args);

    // Handle special commands
    if clean.get(0) == Some("install") {
        run_install(flags.with_deps);
        return;
    }

    if clean.get(0) == Some("session") {
        run_session(&clean, &flags.session, flags.json);
        return;
    }

    // Parse command
    let cmd = match parse_command(&clean, &flags) {
        Ok(c) => c,
        Err(e) => { /* error handling */ exit(1); }
    };

    // Ensure daemon is running
    let daemon_result = ensure_daemon(&flags.session, flags.headed, ...)?;

    // Send command to daemon
    match send_command(cmd, &flags.session) {
        Ok(resp) => {
            print_response(&resp, flags.json);
            if !resp.success { exit(1); }
        }
        Err(e) => { /* error */ exit(1); }
    }
}
```

### Command Parsing (`cli/src/commands.rs`)

```rust
pub fn parse_command(args: &[String], flags: &Flags) -> Result<serde_json::Value, ParseError> {
    match args.get(0).map(|s| s.as_str()) {
        Some("open") | Some("goto") | Some("navigate") => {
            let url = args.get(1).ok_or(ParseError::MissingArguments { ... })?;
            Ok(json!({ "action": "navigate", "url": url }))
        }
        Some("click") => {
            let selector = args.get(1).ok_or(...)?;
            Ok(json!({ "action": "click", "selector": selector }))
        }
        Some("snapshot") => {
            let options = parse_snapshot_flags(args)?;
            Ok(json!({ "action": "snapshot", ...options }))
        }
        // ... more commands
        _ => Err(ParseError::UnknownCommand { command: ... })
    }
}
```

### Daemon Communication (`cli/src/connection.rs`)

```rust
pub fn ensure_daemon(session: &str, headed: bool, ...) -> Result<DaemonResult> {
    let pid_path = temp_dir().join(format!("agent-browser-{}.pid", session));

    // Check if daemon already running
    if let Ok(pid_str) = fs::read_to_string(&pid_path) {
        if let Ok(pid) = pid_str.trim().parse::<u32>() {
            #[cfg(unix)]
            if unsafe { libc::kill(pid as i32, 0) == 0 } {
                return Ok(DaemonResult { already_running: true, pid });
            }
        }
    }

    // Start daemon
    let daemon_path = get_daemon_path()?;
    let mut cmd = Command::new("node");
    cmd.arg(&daemon_path);
    cmd.env("AGENT_BROWSER_SESSION", session);
    if headed { cmd.env("AGENT_BROWSER_HEADED", "1"); }

    let child = cmd.spawn()?;
    fs::write(&pid_path, child.id().to_string())?;

    // Wait for daemon to be ready
    wait_for_socket(session)?;

    Ok(DaemonResult { already_running: false, pid: child.id() })
}
```

---

## Optimal AI Agent Workflow

```bash
# 1. Navigate
agent-browser open https://example.com

# 2. Get interactive snapshot (JSON for agents)
agent-browser snapshot -i --json
# Returns:
# {
#   "success": true,
#   "data": {
#     "snapshot": "- button \"Submit\" [ref=e1]\n- link \"Login\" [ref=e2]",
#     "refs": {
#       "e1": { "role": "button", "name": "Submit" },
#       "e2": { "role": "link", "name": "Login" }
#     }
#   }
# }

# 3. AI parses snapshot and identifies target ref
# 4. Execute action using ref
agent-browser click @e1

# 5. Re-snapshot if page changed
agent-browser snapshot -i --json
```

---

## Error Handling

The CLI provides AI-friendly error messages:

```typescript
// src/actions.ts
export function toAIFriendlyError(error: unknown, selector: string): Error {
  const message = error instanceof Error ? error.message : String(error);

  // Strict mode violation
  if (message.includes('strict mode violation')) {
    const count = message.match(/resolved to (\d+) elements/)?.[1] ?? 'multiple';
    return new Error(
      `Selector "${selector}" matched ${count} elements. ` +
      `Run 'snapshot' to get updated refs.`
    );
  }

  // Element blocked by overlay
  if (message.includes('intercepts pointer events')) {
    return new Error(
      `Element "${selector}" is blocked by another element. ` +
      `Try dismissing modals/cookie banners first.`
    );
  }

  // Element not found
  if (message.includes('Timeout')) {
    return new Error(
      `Element "${selector}" not found. Run 'snapshot' to see current elements.`
    );
  }

  return error instanceof Error ? error : new Error(message);
}
```

---

## Platform Support

| Platform | Binary | Fallback |
|----------|--------|----------|
| macOS ARM64 | ✅ Native Rust | Node.js |
| macOS x64 | ✅ Native Rust | Node.js |
| Linux ARM64 | ✅ Native Rust | Node.js |
| Linux x64 | ✅ Native Rust | Node.js |
| Windows x64 | ✅ Native Rust | Node.js |

---

## Installation

```bash
# npm (recommended)
npm install -g agent-browser
agent-browser install  # Download Chromium

# From source
git clone https://github.com/vercel-labs/agent-browser
cd agent-browser
pnpm install
pnpm build
pnpm build:native   # Requires Rust
pnpm link --global
agent-browser install
```

---

## Files Reference

| File | Description |
|------|-------------|
| `cli/src/main.rs` | Rust CLI entry point |
| `cli/src/commands.rs` | Command parsing |
| `cli/src/flags.rs` | Flag parsing |
| `cli/src/connection.rs` | Daemon IPC |
| `cli/src/install.rs` | Chromium installation |
| `src/daemon.ts` | Node.js daemon |
| `src/browser.ts` | BrowserManager class |
| `src/snapshot.ts` | Enhanced snapshots |
| `src/actions.ts` | Command execution |
| `src/protocol.ts` | JSON protocol |
| `src/types.ts` | TypeScript types |

---

## See Also

- [Agent-Browser README](../../../@formulas/src.rust/src.llamacpp/src.AICoders/src.VarcelLabs/agent-browser/README.md)
- [Main Vercel Labs Exploration](./exploration.md)
- [Rust Revision Guide](./rust-revision.md)
