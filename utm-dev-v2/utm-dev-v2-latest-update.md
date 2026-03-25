---
location: /home/darkvoid/Boxxed/@formulas/src.rust/src.llamacpp/src.GedWeb/utm-dev
repository: git@github.com:joeblew999/utm-dev.git
explored_at: 2026-03-25T00:00:00Z
type: update
parent: exploration.md
previous_state: exploration.md (2026-03-24, assumed bash/TOML architecture)
---

# utm-dev-v2: Latest Update — What Changed and What to Keep in Mind

> **This document supersedes assumptions in `exploration.md` about bash-based tasks.**
> utm-dev has undergone a complete task rewrite from bash scripts to TypeScript/Bun.

---

## TL;DR — The Big Shifts

| What We Assumed (exploration.md) | What Actually Happened |
|---|---|
| Tasks are **bash scripts** with `#MISE` comments | Tasks are **TypeScript files** running on **Bun** with `//MISE` comments |
| 3 tasks under `vm/` (up, down, exec) | 6 tasks under `vm/` + 5 public tasks + 1 under `clean/` |
| Single Windows VM | **5 VM profiles**: windows-build, windows-test, linux-build, linux-test, linux-dev |
| Manual bootstrap | **Auto-bootstrap** with two modes: `full` (build tools) vs `ssh-only` (clean testing) |
| No MCP integration | **MCP servers** configured (context7 + mise) |
| No screenshot tooling | **WebDriver-based screenshot automation** |
| Bash logging helpers | **Structured TypeScript logging** via `_lib.ts` |
| `sshpass` + raw SSH | **Pure TypeScript WinRM SOAP client** + SSH helpers |
| Platform-agnostic setup | **Platform-aware setup** (macOS: SDKs, Linux: apt libs) |

---

## 1. The TypeScript/Bun Migration

### Why This Matters

The single most important change: **every task is now TypeScript running on Bun**. This is not a cosmetic rename — it fundamentally changes how the system works.

```
Before (exploration.md assumed):
  .mise/tasks/vm/up        ← bash script, #MISE metadata
  .mise/tasks/vm/down       ← bash script
  .mise/tasks/setup         ← bash script

After (actual current state):
  .mise/tasks/vm/up.ts      ← TypeScript, //MISE metadata
  .mise/tasks/vm/down.ts    ← TypeScript
  .mise/tasks/setup.ts      ← TypeScript
  .mise/tasks/_lib.ts       ← shared library (NOT a task)
  .mise/tasks/_utm.ts       ← shared library (NOT a task)
  .mise/tasks/_winrm.ts     ← shared library (NOT a task)
```

### What Bun Gives Them

- **Native `fetch()`** — WinRM SOAP client uses fetch, no curl/python
- **Native `Bun.spawn()`** — subprocess management with streaming stdout/stderr
- **TypeScript without transpilation** — Bun runs `.ts` directly
- **Top-level `await`** — async tasks without wrapping in main()
- **Fast startup** — ~10ms vs ~200ms for Node.js

### mise Task Metadata Syntax Change

```typescript
// TypeScript files use //MISE (double-slash) not #MISE (hash)
//MISE description="Start a VM — import + bootstrap on first run"
//MISE alias="vup"
//MISE depends=["setup"]
//MISE hide=true
//MISE
//MISE [args]
//MISE name = { description = "VM profile name", required = true }
```

> **Keep in mind:** The `#MISE` syntax from bash DOES NOT WORK in TypeScript files.
> This was a bug that was caught and fixed (commit `2734b9e`).

### File Naming Convention

Files starting with `_` are **internal libraries**, not tasks:

| File | Type | Purpose |
|---|---|---|
| `_lib.ts` | Library | VM profiles, SSH/SCP helpers, state management, logging |
| `_utm.ts` | Library | UTM operations (install, import box, configure network, start/stop) |
| `_winrm.ts` | Library | Pure SOAP WinRM client over HTTP (no Python dependency) |
| `_bootstrap.ts` | Library | Windows VM provisioning (OpenSSH, VS Build Tools, WebView2, mise) |
| `_bootstrapLinux.ts` | Library | Linux VM provisioning (build-essential, Rust, mise, Tauri deps) |
| `_screenshot.ts` | Library | WebDriver session management for automated screenshots |

mise ignores `_`-prefixed files as tasks — they're only imported by other tasks.

---

## 2. The Five VM Profiles

The system now manages **five distinct VMs**, not one. Each has a specific purpose, resource allocation, and bootstrap mode.

### Profile Table

```mermaid
flowchart TB
    subgraph WINDOWS["Windows ARM64"]
        direction LR
        wb["windows-build<br/>12GB RAM · 4 CPU<br/>SSH:2222 · RDP:3389 · WinRM:5985<br/>Bootstrap: full<br/>(VS Build Tools + Rust + mise)"]
        wt["windows-test<br/>4GB RAM · 2 CPU<br/>SSH:2322 · RDP:3489 · WinRM:6985<br/>Bootstrap: ssh-only<br/>(clean Windows)"]
    end

    subgraph LINUX["Linux ARM64"]
        direction LR
        lb["linux-build<br/>ubuntu-24.04 headless<br/>4GB RAM · 4 CPU<br/>SSH:2422<br/>Bootstrap: full"]
        lt["linux-test<br/>ubuntu-24.04 headless<br/>2GB RAM · 2 CPU<br/>SSH:2522<br/>Bootstrap: ssh-only"]
        ld["linux-dev<br/>debian-12 GNOME desktop<br/>6GB RAM · 4 CPU<br/>SSH:2622<br/>Bootstrap: full"]
    end

    style wb fill:#0f3460,stroke:#533483,color:#e0e0e0
    style wt fill:#264653,stroke:#2a9d8f,color:#e0e0e0
    style lb fill:#2d6a4f,stroke:#1b4332,color:#fff
    style lt fill:#264653,stroke:#2a9d8f,color:#e0e0e0
    style ld fill:#40916c,stroke:#2d6a4f,color:#fff
```

### Bootstrap Modes

```mermaid
flowchart TD
    up["vm:up <profile>"] --> check{"Bootstrap<br/>mode?"}
    check -->|"full"| full["Install everything:<br/>- OpenSSH server<br/>- VS Build Tools (Windows)<br/>- build-essential (Linux)<br/>- WebView2 Runtime<br/>- Rust toolchain<br/>- mise + mise install"]
    check -->|"ssh-only"| ssh["Just enable SSH<br/>(clean OS for testing)"]
    check -->|"false"| skip["No bootstrap<br/>(pre-configured box)"]

    full --> stop["Stop VM"]
    ssh --> stop
    skip --> stop
    stop --> save["Save state to<br/>.mise/state/vm-{name}.env"]
```

> **Keep in mind:**
> - `windows-build` needs **12GB RAM** — VS Build Tools crashes at 8GB on ARM64
> - Linux VMs take **5–10 minutes** to boot (cloud-init + SSH key generation)
> - Boot timeout is **600s for Linux**, **300s for Windows** — a hard-learned bug fix
> - All VMs use `vagrant:vagrant` credentials from Vagrant Cloud boxes

---

## 3. The Complete Task Map

### Current Directory Structure

```
.mise/tasks/
├── _lib.ts                 # VM profiles, SSH/SCP, state, logging
├── _winrm.ts               # Pure WinRM SOAP client (fetch-based)
├── _utm.ts                 # UTM install, import, network, start/stop
├── _bootstrap.ts           # Windows VM provisioning
├── _bootstrapLinux.ts      # Linux VM provisioning
├── _screenshot.ts          # WebDriver session management
│
├── init.ts                 # Add utm-dev [tools]+[env] to project's mise.toml
├── setup.ts                # Platform-aware: macOS SDKs / Linux apt libs
├── doctor.ts               # Health check: what's installed/missing
├── mcp.ts                  # Configure MCP servers (.mcp.json)
├── screenshot.ts           # WebDriver screenshot automation
├── clean/
│   └── disk.ts             # System-wide disk cleanup (safe + --deep)
│
└── vm/                     # ALL HIDDEN (hide=true) — internal plumbing
    ├── up.ts               # Import + bootstrap + start (idempotent)
    ├── down.ts             # Stop VM
    ├── exec.ts             # SSH command in VM
    ├── build.ts            # Sync → install → build → pull artifacts
    ├── delete.ts           # Delete VM/UTM (preserves box cache)
    └── package.ts          # Export as Vagrant .box
```

### Task Dependency Flow

```mermaid
flowchart TD
    subgraph PUBLIC["Public Tasks (user-facing)"]
        init["init"]
        setup["setup"]
        doctor["doctor"]
        mcp["mcp"]
        screenshot["screenshot"]
        clean_disk["clean:disk"]
    end

    subgraph HIDDEN["Hidden Tasks (vm plumbing)"]
        vm_up["vm:up"]
        vm_down["vm:down"]
        vm_exec["vm:exec"]
        vm_build["vm:build"]
        vm_delete["vm:delete"]
        vm_package["vm:package"]
    end

    subgraph CONSUMER["Consumer Tasks (in example mise.toml)"]
        mac_build["mac:build"]
        ios_build["ios:build"]
        android_build["android:build"]
        windows_build["windows:build"]
        linux_build["linux:build"]
        all_build["all:build"]
    end

    init --> setup
    setup --> doctor
    setup --> vm_up
    vm_up --> vm_exec
    vm_up --> vm_build
    vm_build --> vm_down

    windows_build -->|"mise run vm:build windows-build"| vm_build
    linux_build -->|"mise run vm:build linux-build"| vm_build
    all_build --> mac_build & ios_build & android_build & windows_build & linux_build

    vm_build -->|"auto-starts if<br/>SSH unreachable"| vm_up

    style PUBLIC fill:#2d6a4f,stroke:#1b4332,color:#fff
    style HIDDEN fill:#533483,stroke:#e94560,color:#e0e0e0
    style CONSUMER fill:#0f3460,stroke:#533483,color:#e0e0e0
```

> **Keep in mind:** Users never run `vm:*` tasks directly. They use consumer-level tasks like `windows:build` or `linux:build` which delegate to `vm:build` internally.

---

## 4. Platform-Aware Setup

`setup.ts` now detects the host OS and runs completely different installation paths.

### macOS Path (7 stages)

```mermaid
flowchart TD
    setup["mise run setup<br/>(macOS detected)"] --> s1["Stage 1: Host Tools<br/>Rust (rustup), Xcode CLI"]
    s1 --> s2["Stage 2: Java<br/>mise use java@temurin-17"]
    s2 --> s3["Stage 3: Android SDK<br/>cmdline-tools download<br/>sdkmanager accept-licenses"]
    s3 --> s4["Stage 4: SDK Components<br/>platform android-35<br/>build-tools 35.0.0<br/>NDK 27.2.12479018<br/>emulator + system-image"]
    s4 --> s5["Stage 5: Android AVD<br/>avdmanager create 'utm-dev'<br/>pixel_6 device"]
    s5 --> s6["Stage 6: Rust Targets<br/>aarch64-linux-android<br/>armv7-linux-androideabi<br/>i686-linux-android<br/>x86_64-linux-android"]
    s6 --> s7["Stage 7: iOS Deps<br/>gem install cocoapods"]
```

### Linux Path (2 stages)

```mermaid
flowchart TD
    setup["mise run setup<br/>(Linux detected)"] --> s1["Stage 1: System Libraries<br/>apt-get install:<br/>build-essential, libwebkit2gtk-4.1-dev<br/>libgtk-3-dev, libsoup-3.0-dev<br/>libjavascriptcoregtk-4.1-dev<br/>librsvg2-dev, libssl-dev<br/>libxdo-dev, patchelf"]
    s1 --> s2["Stage 2: Verify mise tools<br/>Rust, bun, cargo-tauri<br/>(already installed by mise)"]
```

> **Keep in mind:** mise handles Rust, Bun, Java, Ruby, cargo-tauri (things it CAN install). `setup.ts` only handles what mise CANNOT: OS-level C libraries (apt), Apple SDKs, CocoaPods. This split is intentional and important.

---

## 5. The WinRM SOAP Client

One of the most notable pieces of engineering: a **pure TypeScript WinRM client** using only `fetch()`. No Python, no external dependencies.

### How It Works

```mermaid
sequenceDiagram
    participant Task as vm/up.ts
    participant WinRM as _winrm.ts
    participant VM as Windows VM :5985

    Task->>WinRM: runElevated("Install-VSBuildTools.ps1")
    WinRM->>WinRM: Encode PS script as UTF-16LE base64
    WinRM->>VM: SOAP/XML: Create scheduled task as SYSTEM
    VM-->>WinRM: Task created
    WinRM->>VM: SOAP/XML: Start scheduled task
    Note over VM: VS Build Tools installing...<br/>(10-15 min, WinRM may drop)
    loop Poll every 10s
        WinRM->>VM: SOAP/XML: Query task status
        VM-->>WinRM: Still running / Complete
    end
    VM-->>WinRM: Exit code 0
    WinRM->>WinRM: Read stdout from C:\Windows\Temp\task-output.txt
    WinRM-->>Task: { stdout, exitCode }
```

### API Surface

```typescript
class WinRMClient {
  async runCmd(command: string)           // cmd.exe /c
  async runPS(script: string)             // PowerShell -EncodedCommand (UTF-16LE)
  async runElevated(psCode, timeout?)     // Scheduled task as SYSTEM (bypasses UAC)
  async ping(timeoutMs?)                  // HTTP probe to /wsman
}
```

> **Keep in mind:**
> - `runElevated()` is the **only reliable method** for installing software on Windows ARM64
> - WinRM connections **drop during heavy I/O** (VS Build Tools install) — keep polling
> - PowerShell's `-EncodedCommand` requires **UTF-16LE base64**, not UTF-8 — `atob()` corrupts multi-byte characters, use `Buffer`
> - Scheduled tasks run as SYSTEM, bypassing UAC entirely

---

## 6. MCP Integration

New `mcp.ts` task configures AI-assisted development:

### Servers Configured

| Server | Command | Purpose |
|---|---|---|
| **context7** | `bunx @upstash/context7-mcp@latest` | Live documentation retrieval for any library |
| **mise** | `mise mcp` | Exposes tools, tasks, env, config as MCP resources |

### What mise MCP Exposes

| Resource | URI | Content |
|---|---|---|
| Tools | `mise://tools` | All installed tools with versions |
| Tasks | `mise://tasks` | Available tasks with descriptions |
| Environment | `mise://env` | Current env vars set by mise |
| Config | `mise://config` | mise.toml parsed as structured data |

| Tool | Description |
|---|---|
| `run_task` | Execute any mise task programmatically |
| `install_tool` | Install a tool via mise |

### Auto-Permissions

`mcp.ts` writes `.claude/settings.json` with `permissions.allow = ["mcp__*__*"]` — all MCP tools auto-allowed, no prompting.

---

## 7. Screenshot Automation

New WebDriver-based screenshot system:

```mermaid
sequenceDiagram
    participant Task as screenshot.ts
    participant SS as _screenshot.ts
    participant WD as tauri-webdriver :4444
    participant App as Tauri App

    Task->>SS: startSession(projectRoot)
    SS->>SS: cargo install tauri-webdriver --locked
    SS->>SS: cargo build --features webdriver
    SS->>SS: Clean stale single-instance .sock files
    SS->>WD: Spawn WebDriver proxy
    SS->>App: Spawn app with TAURI_WEBVIEW_AUTOMATION=true
    SS->>WD: Poll: POST /session (45s timeout)
    WD-->>SS: sessionId
    SS->>WD: Poll: document.readyState === 'complete'
    WD-->>SS: Ready
    SS-->>Task: { url, sessionId, cleanup() }

    alt Custom take.ts exists
        Task->>Task: Run screenshots/take.ts with<br/>WEBDRIVER_URL + WEBDRIVER_SESSION
    else Default
        Task->>WD: GET /session/{id}/screenshot
        WD-->>Task: Base64 PNG
        Task->>Task: Save to screenshots/app.png
    end
```

> **Keep in mind:**
> - WebDriver is **feature-gated** (`--features webdriver`) — not in production builds
> - The `tauri-plugin-single-instance` leaves stale `.sock` files that block WebDriver sessions — `_screenshot.ts` cleans them
> - Custom `screenshots/take.ts` scripts get `WEBDRIVER_URL` and `WEBDRIVER_SESSION` as env vars

---

## 8. Disk Cleanup

New `clean:disk` task with intelligent safe cleanup:

### Protected Directories (NEVER touched)

| Path | Why |
|---|---|
| `~/.cache/utm-dev/` | Box cache (6GB+ downloads) |
| `~/Library/Containers/com.utmapp.UTM` | Live VMs |
| `~/.rustup/toolchains` | Rust installations |
| `~/.android-sdk` | Android SDK |

### Standard Cleanup (50+ MB threshold)

| Target | Typical Size |
|---|---|
| `target/` directories (Rust builds) | 1–30 GB |
| Unavailable iOS simulators | 1–5 GB |
| CoreSimulator caches/logs | 500 MB–2 GB |
| Xcode DerivedData | 1–10 GB |
| Cargo registry cache | 200 MB–1 GB |
| Gradle caches | 500 MB–3 GB |
| Bun install cache | 100 MB–1 GB |
| npm cache | 100 MB–500 MB |
| CocoaPods cache | 200 MB–1 GB |

### `--deep` Mode (additional)

| Target | Typical Size |
|---|---|
| Homebrew cache | 500 MB–3 GB |
| Xcode Archives | 1–20 GB |
| Xcode iOS DeviceSupport | 2–10 GB |
| Docker images/build cache | 5–50 GB |
| System logs (`/private/var/log`) | 100 MB–1 GB |

> **Keep in mind:** Always use `--dry-run` first. The standard mode is safe for daily use; `--deep` should be reserved for disk pressure situations.

---

## 9. The vm:build Pipeline

The most complex hidden task — handles the full sync → build → retrieve cycle:

```mermaid
sequenceDiagram
    participant Dev as Developer
    participant Build as vm:build
    participant VM as Target VM

    Dev->>Build: mise run vm:build linux-build

    Build->>Build: Check SSH to linux-build
    alt SSH unreachable
        Build->>Build: mise run vm:up linux-build
        Note over Build: Auto-cascade: starts + bootstraps VM
    end

    Build->>Build: tar.gz project<br/>(exclude: target/, node_modules/,<br/>.git/, .gradle/, dist/)
    Build->>VM: SCP archive → /tmp/project.tar.gz
    Build->>VM: SSH: tar xzf + cd project

    Build->>VM: SSH: mise trust && mise install
    Note over VM: Installs identical toolchain

    Build->>VM: SSH: mise run build
    Note over VM: cargo tauri build --release

    VM-->>Build: SSH: tar.gz bundle/ directory
    Build->>Build: SCP artifacts → .build/{platform}/
    Build->>Build: List artifacts with sizes

    Build-->>Dev: ✓ .build/linux-build/<br/>  app.deb (15 MB)<br/>  app.AppImage (22 MB)
```

> **Keep in mind:**
> - `vm:build` **auto-starts the VM** if SSH is unreachable — fully self-healing
> - Source sync uses tar+scp, not rsync — simpler, works on all platforms
> - Inside the VM, `mise trust && mise install` ensures identical toolchain
> - Artifacts land in `.build/{profile-name}/` on the host

---

## 10. The Consumer Example Pattern

### How Projects Consume utm-dev

The `examples/tauri-basic/mise.toml` shows the recommended consumption pattern:

```toml
# Local development (symlink to utm-dev tasks)
[task_config]
includes = ["../../.mise/tasks"]

# Production (remote git include)
# [task_config]
# includes = ["git::https://github.com/joeblew999/utm-dev.git//.mise/tasks?ref=main"]
```

### Consumer-Level Tasks (what devs actually run)

```toml
# These are in the CONSUMER'S mise.toml, NOT in utm-dev
[tasks.mac:build]
run = "cargo tauri build"

[tasks.ios:build]
run = "cargo tauri ios build"

[tasks.android:build]
run = "cargo tauri android build"

[tasks.windows:build]
run = "mise run vm:build windows-build"

[tasks.linux:build]
run = "mise run vm:build linux-build"

[tasks.all:build]
depends = ["mac:build", "ios:build", "android:build", "windows:build", "linux:build"]
```

> **Keep in mind:** The `vm:*` tasks are **hidden** (`hide=true`). Users never see them in `mise tasks`. They use platform-level aliases. This is intentional UX design.

---

## 11. Tauri 2.x Capabilities System

The example app demonstrates Tauri's **fine-grained permission model** with capabilities split by platform:

### Capability Files

```
src-tauri/capabilities/
├── default.json     # Shared across ALL platforms
├── desktop.json     # Desktop-only permissions
└── mobile.json      # Mobile-only permissions
```

### Permission Split

```mermaid
flowchart TD
    subgraph DEFAULT["default.json (all platforms)"]
        core["core:default"]
        os["os:default"]
        dialog["dialog:default"]
        store["store:default"]
        notification["notification:default"]
        clipboard["clipboard-manager:default"]
        opener["opener:default"]
        log["log:default"]
        fs["fs:default"]
    end

    subgraph DESKTOP["desktop.json"]
        shell["shell:default"]
        process["process:default"]
        updater["updater:default"]
        window_state["window-state:default"]
        global_shortcut["global-shortcut:default"]
        autostart["autostart:default"]
        deep_link_desktop["deep-link:default"]
    end

    subgraph MOBILE["mobile.json"]
        opener_mobile["opener:default"]
        deep_link_mobile["deep-link:default"]
    end

    style DEFAULT fill:#2d6a4f,stroke:#1b4332,color:#fff
    style DESKTOP fill:#0f3460,stroke:#533483,color:#e0e0e0
    style MOBILE fill:#533483,stroke:#e94560,color:#e0e0e0
```

### Plugins in Use (16 total)

| Plugin | Purpose | Platform |
|---|---|---|
| Shell | Execute system commands | Desktop |
| OS | System info (arch, OS, locale) | All |
| Dialog | File open/save, message boxes | All |
| Store | Persistent key-value storage | All |
| Notification | System notifications | All |
| Clipboard | Copy/paste | All |
| Opener | Open URLs/files in default app | All |
| Process | Manage app lifecycle, exit, restart | Desktop |
| Log | Structured logging to file | All |
| Filesystem | File read/write (scoped) | All |
| Updater | Auto-update with signing | Desktop |
| Window State | Remember window position/size | Desktop |
| Single Instance | Prevent multiple app instances | Desktop |
| Global Shortcut | System-wide keyboard shortcuts | Desktop |
| Autostart | Launch on login | Desktop |
| Deep Link | Handle `tauri-basic://` URLs | All |

> **Keep in mind:**
> - **Single Instance** must register FIRST in the plugin chain (before anything that spawns windows)
> - **WebDriver** is feature-gated: `cargo build --features webdriver` — NEVER in production builds
> - Updater uses Tauri's built-in signing keys (`.tauri-key` file)
> - Capabilities are **additive** — you can't deny a permission, only grant

---

## 12. Bug Fixes and Hard-Learned Lessons

These are the things that will bite you if you don't know about them:

### VM Import UUID Bug (commit `8994abf`)

**Problem:** `importBox()` used `getFirstVm()` to find the newly imported VM. With multiple VMs, this always returned the Windows VM — even when importing Linux.

**Fix:** Snapshot all UUIDs before import, snapshot after, diff to find the new one.

**Lesson:** Never assume ordering in `utmctl list`. Always diff state.

### SSH Host Key Rejection (commit `8994abf`)

**Problem:** Reimporting a VM (after delete + recreate) produces a new SSH host key. The old key in `~/.ssh/known_hosts` causes SSH to refuse connection.

**Fix:** All SSH/SCP calls now use:
```
-o StrictHostKeyChecking=no -o UserKnownHostsFile=/dev/null
```

**Lesson:** For disposable VMs, never trust host key verification. It's a development tool, not production SSH.

### Linux Boot Timeout (commit `8994abf`)

**Problem:** Boot timeout was 300s (5 min) for all VMs. Linux cloud-init + SSH key generation takes 5–10 minutes.

**Fix:** 600s timeout for Linux, 300s for Windows.

**Lesson:** Don't assume boot times are uniform across OS types.

### VS Build Tools on ARM64 (commit `de1f9c4`)

**Problem:** `winget install` with `--override` flags doesn't work for VS Build Tools on ARM64.

**Fix:** Direct bootstrapper download + `--wait` flag:
```
vs_BuildTools.exe --wait --add Microsoft.VisualStudio.Workload.VCTools
```

**Lesson:** winget is not reliable for complex installers on ARM64. Use direct installers.

### WinRM Drops During Heavy I/O (commit `97dd189`)

**Problem:** Installing VS Build Tools causes WinRM to become unresponsive. Task appears to hang.

**Fix:** Keep polling even after connection drops. The installation continues inside the VM regardless of WinRM state.

**Lesson:** WinRM is fragile under load. Design for disconnection resilience.

### mise Task Syntax (commit `2734b9e`)

**Problem:** TypeScript files used `#MISE` (bash syntax). mise silently ignored all metadata.

**Fix:** Use `//MISE` for TypeScript/JavaScript files.

**Lesson:** mise task metadata syntax varies by file extension: `#MISE` for bash/shell, `//MISE` for TypeScript/JavaScript.

### 12GB RAM for Windows Build (commit `37a3e1f`)

**Problem:** VS Build Tools crashes or hangs during compilation with 8GB RAM on ARM64.

**Fix:** Bumped `windows-build` profile to 12GB RAM.

**Lesson:** ARM64 Windows emulation of x86 build tools has significantly higher memory overhead than native.

---

## 13. Architecture Comparison: Then vs Now

```mermaid
flowchart LR
    subgraph THEN["exploration.md Assumed"]
        direction TB
        bash["Bash scripts<br/>(~50KB)"]
        hash_mise["#MISE metadata"]
        three_tasks["3 vm tasks<br/>(up, down, exec)"]
        one_vm["1 VM<br/>(Windows)"]
        manual_boot["Manual bootstrap"]
        no_mcp["No MCP"]
        no_screenshot["No screenshots"]
    end

    subgraph NOW["Actual Current State"]
        direction TB
        ts["TypeScript/Bun<br/>(~30 files)"]
        slash_mise["//MISE metadata"]
        twelve_tasks["12 tasks<br/>(6 vm + 6 public)"]
        five_vm["5 VM profiles<br/>(2 Win + 3 Linux)"]
        auto_boot["Auto-bootstrap<br/>(full / ssh-only)"]
        mcp["MCP integration<br/>(context7 + mise)"]
        screenshot["WebDriver screenshots"]
    end

    THEN -->|"reality gap"| NOW

    style THEN fill:#e94560,stroke:#e94560,color:#fff
    style NOW fill:#2d6a4f,stroke:#1b4332,color:#fff
```

---

## 14. How mise Is Actually Used Now

### Tool Management (mise.toml)

```toml
[tools]
"cargo:tauri-cli" = {version = "2", os = ["macos", "windows"]}
bun = "latest"                           # Task runtime
xcodegen = {version = "latest", os = ["macos"]}
ruby = {version = "3.3", os = ["macos"]} # CocoaPods
java = "temurin-17.0.18+8"              # Android SDK
```

**Important split:**
- mise installs: Rust, Bun, Java, Ruby, cargo-tauri, xcodegen
- `setup.ts` installs: Android SDK (cmdline-tools, NDK), Xcode CLI, CocoaPods gem, Linux system libs

### Environment Management

```toml
[env]
ANDROID_HOME = "{{env.HOME}}/.android-sdk"
NDK_HOME = "{{env.HOME}}/.android-sdk/ndk/27.2.12479018"
JAVA_HOME = "{{env.HOME}}/.local/share/mise/installs/java/temurin-17.0.18+8"
_.path = [
  "{{env.HOME}}/.android-sdk/platform-tools",
  "{{env.HOME}}/.android-sdk/cmdline-tools/latest/bin",
]
```

### Task Loading

Tasks are loaded from `.mise/tasks/` automatically — no `[task_config]` needed in the utm-dev repo itself. Consumers use `includes` to pull them in:

```toml
# Consumer project's mise.toml
[task_config]
includes = ["git::https://github.com/joeblew999/utm-dev.git//.mise/tasks?ref=main"]
```

### mise Inside VMs

Every full-bootstrap VM gets mise installed. The flow inside the VM:

```
1. curl https://mise.run | sh                    # Install mise
2. eval "$(~/.local/bin/mise activate bash)"     # Activate
3. cd /project && mise trust                     # Trust project config
4. mise install --yes                            # Install SAME tool versions
5. mise run build                                # Build with identical toolchain
```

This is the **reproducibility chain**: committed `mise.toml` → identical tools on host, in every VM, and in CI.

---

## 15. State Management

### Per-VM State Files

```
.mise/state/
├── vm-windows-build.env     # VM_UUID=xxx VM_DISPLAY_NAME=yyy
├── vm-windows-test.env
├── vm-linux-build.env
├── vm-linux-test.env
└── vm-linux-dev.env
```

Each file is just `key=value` pairs. Tasks read them to find the VM UUID for `utmctl` commands.

### Migration

The old single `vm.env` file is auto-migrated on first access — backward compatible.

### Logging

```
.mise/logs/
├── vm-up.log              # VM lifecycle events
├── vm-bootstrap.log       # Bootstrap progress (long-running)
├── setup.log              # SDK installation
└── doctor.log             # Health check results
```

All logging goes through `_lib.ts` helpers: `log()`, `info()`, `ok()`, `die()`.

---

## 16. What Our Vagrant Documents Need to Account For

The `utm-dev-v2-vagrant.md` and `utm-dev-v2-vagrant-primer.md` were written assuming bash tasks. They're still valuable for Vagrant integration concepts, but need these mental adjustments:

| Vagrant Doc Assumption | Actual Reality |
|---|---|
| Tasks are bash scripts | Tasks are TypeScript/Bun |
| `#MISE` metadata | `//MISE` metadata |
| `log_json()` bash function | `_lib.ts` TypeScript helpers |
| Single Windows + Linux VM | 5 VM profiles with distinct purposes |
| Manual provisioning scripts | Auto-bootstrap via `_bootstrap.ts` / `_bootstrapLinux.ts` |
| `sshpass` only | WinRM SOAP client for Windows, SSH for Linux |

The Vagrant integration path described in those docs (replacing UTM with Vagrant for cross-platform host support) remains valid as a **future direction**. The current system uses UTM exclusively but the architectural concepts (multi-VM, provisioning, snapshot management) are directly applicable.

---

## Quick Reference Card

### Daily Commands

```bash
# Health check
mise run doctor

# Start Windows build VM
mise run vm:up windows-build

# Build for Windows
mise run vm:build windows-build

# Build for Linux
mise run vm:build linux-build

# Stop all VMs
mise run vm:down windows-build
mise run vm:down linux-build

# Free disk space
mise run clean:disk --dry-run
mise run clean:disk

# Configure AI tooling
mise run mcp

# Take screenshots
mise run screenshot
```

### Key Ports

| Profile | SSH | RDP | WinRM |
|---|---|---|---|
| windows-build | 2222 | 3389 | 5985 |
| windows-test | 2322 | 3489 | 6985 |
| linux-build | 2422 | — | — |
| linux-test | 2522 | — | — |
| linux-dev | 2622 | — | — |

### Key Paths

| Path | Purpose |
|---|---|
| `~/.cache/utm-dev/` | Box cache (6GB+ Windows, 1-2GB Linux) |
| `.mise/state/vm-*.env` | Per-VM UUID tracking |
| `.mise/logs/` | Task execution logs |
| `.build/{profile}/` | Build artifacts from VMs |
