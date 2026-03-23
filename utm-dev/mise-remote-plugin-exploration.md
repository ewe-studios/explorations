---
location: /home/darkvoid/Boxxed/@formulas/src.rust/src.llamacpp/src.GedWeb/utm-dev
repository: git@github.com:joeblew999/utm-dev.git
explored_at: 2026-03-23T00:00:00Z
language: Go (100%)
update_type: mise remote plugin addition
---

# UT M-Dev: Mise Remote Plugin Exploration

## Overview

**utm-dev** has been updated to use a **mise remote plugin architecture**, shifting from a standalone Go CLI tool to a **distributed task system** powered by mise's remote task includes feature. This represents a significant architectural shift that simplifies distribution, reduces dependencies, and integrates utm-dev directly into users' existing mise workflows.

### What Changed

| Before (Go CLI) | After (Mise Remote Plugin) |
|-----------------|----------------------------|
| Go binary (~50MB) | Bash scripts (~50KB) |
| Cobra CLI framework | mise task system |
| Manual installation | `git::` remote include |
| Self-contained tool | Integrated into user's mise.toml |
| Separate versioning | Follows mise versioning |

## Repository Structure

```
utm-dev/
├── .mise/
│   └── tasks/
│       ├── init                          # One-time project initialization
│       ├── setup                         # Install Tauri dev prereqs
│       └── vm/
│           ├── up                        # UTM VM lifecycle: install + start
│           ├── down                      # Stop the VM
│           ├── delete                    # Delete VM + optional cleanup
│           └── exec                      # Run command in VM via SSH
├── mise.toml                             # Root mise config for utm-dev itself
├── README.md                             # User-facing documentation
├── CLAUDE.md                             # Development context
└── examples/
    └── tauri-basic/
        ├── mise.toml                     # Example project using utm-dev
        └── src-tauri/
            ├── Cargo.toml
            └── src/
```

## Mise Remote Plugin Architecture

### How It Works

Users add utm-dev to their project via mise's remote task include syntax:

```toml
# In user's project mise.toml
[task_config]
includes = ["git::https://github.com/joeblew999/utm-dev.git//.mise/tasks?ref=main"]
```

This pulls the task definitions directly from the git repository. mise automatically:
1. Clones the repository
2. Reads task definitions from `.mise/tasks/`
3. Makes tasks available via `mise run <task>`

### Task Definitions

Tasks are **bash scripts** with special `#MISE` comments that define metadata:

```bash
#!/usr/bin/env bash
set -eu

#MISE description="Install UTM + download Windows VM + start + wait"
#MISE alias="vm-up"

PROJECT_DIR="$(pwd)"
# ... task logic ...
```

The `#MISE` comments provide:
- `description` - Shown in `mise run --list`
- `alias` - Alternative short name

### Available Tasks

| Task | Description | Idempotent |
|------|-------------|------------|
| `mise run init` | Add tools + env to user's mise.toml | Yes (one-time) |
| `mise run setup` | Install Tauri dev prereqs (Rust, Android SDK, iOS deps) | Yes |
| `mise run vm:up` | Install UTM + download Windows VM + configure + start | Yes |
| `mise run vm:down` | Stop the VM gracefully | Yes |
| `mise run vm:delete vm` | Delete VM (keeps cached 6GB box) | Yes |
| `mise run vm:delete utm` | Delete VM + uninstall UTM | Yes |
| `mise run vm:delete all` | Nuclear option (still keeps box cache) | Yes |
| `mise run vm:exec <cmd>` | Run command in VM via SSH | Yes |

## Deep Dive: vm:up Task

The `vm:up` task is the most complex, handling the complete UTM VM lifecycle:

### Stage 1: Install UTM

```bash
# Install via Homebrew
HOMEBREW_NO_AUTO_UPDATE=1 brew install --cask utm

# Suppress "What's New" dialog
UTM_VERSION=$(PlistBuddy -c "Print :CFBundleShortVersionString" /Applications/UTM.app/Contents/Info.plist)
defaults write "${CONTAINER_PREFS}/com.utmapp.UTM" ReleaseNotesLastVersion -string "${UTM_VERSION}"

# Launch UTM in background
open -g /Applications/UTM.app
wait_for_utmctl 30
```

### Stage 2: Find or Download VM

```bash
# Check if VM exists by UUID
if ${UTMCTL} list | grep -q "${VM_UUID}"; then
  ok "VM exists"
fi

# Download from Vagrant Cloud
BOX_VERSIONS_API="https://api.cloud.hashicorp.com/vagrant/2022-09-30/registry/utm/box/windows-11/versions"
BOX_VERSION=$(curl -sSf "${BOX_VERSIONS_API}" | grep -o '"name":"[^"]*' | head -1)

# Cache at ~/.cache/utm-dev/
BOX_FILE="${HOME}/.cache/utm-dev/windows-11_${BOX_VERSION}_arm64.box"

# Extract and import
tar -xf "${BOX_FILE}" -C "${TMPDIR_BOX}"
osascript -e "tell application \"UTM\" to import new virtual machine from POSIX file \"${UTM_FOLDER}\""
```

### Stage 3: Configure Network (AppleScript)

Uses Vagrant-UTM's approach: read config, mutate, write back via AppleScript:

```applescript
tell application "UTM"
  set vm to virtual machine id "${VM_UUID}"
  set config to configuration of vm
  set networkInterfaces to network interfaces of config
  repeat with anInterface in networkInterfaces
    if mode of anInterface is emulated then
      set port forwards of anInterface to {}
      -- Add SSH, RDP, WinRM forwards
    end if
  end repeat
  update configuration of vm with config
end tell
```

Port forwards configured:
- SSH: localhost:2222 → VM:22
- RDP: localhost:3389 → VM:3389
- WinRM: localhost:5985 → VM:5985

### Stage 4: Start VM + Wait for Boot

```bash
${UTMCTL} start "${VM_DISPLAY_NAME}"

# Wait for Windows boot (up to 5 min)
TIMEOUT=300
while [ $ELAPSED -lt $TIMEOUT ]; do
  if curl -s "http://127.0.0.1:${WINRM_PORT}/wsman" >/dev/null; then
    ok "Windows ready"
    break
  fi
  sleep 5
done
```

## Deep Dive: setup Task

The `setup` task installs all Tauri development prerequisites in 4 stages:

### Stage 1: Host Tools

```bash
# Rust
if ! command -v cargo; then
  curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
fi

# cargo-tauri
if ! cargo tauri --version; then
  cargo install tauri-cli
fi

# Xcode check
if ! xcode-select -p; then
  echo "Install Xcode from App Store"
  exit 1
fi
```

### Stage 2: Mobile SDKs

```bash
# Java via mise
mise use --global java@temurin-17.0.18+8

# Android SDK cmdline-tools
CMDLINE_TOOLS_URL="https://dl.google.com/android/repository/commandlinetools-mac-14742923_latest.zip"
curl -sSfL -o "${TMPZIP}" "${CMDLINE_TOOLS_URL}"
unzip -qo "${TMPZIP}" -d "${ANDROID_HOME}/cmdline-tools-tmp"
mv "${ANDROID_HOME}/cmdline-tools-tmp/cmdline-tools" "${ANDROID_HOME}/cmdline-tools/latest"

# Install SDK components via sdkmanager
sdkmanager "platforms;android-35"
sdkmanager "build-tools;35.0.0"
sdkmanager "platform-tools"
sdkmanager "ndk;${NDK_VERSION}"
```

### Stage 3: Rust Android Targets

```bash
TARGETS="aarch64-linux-android armv7-linux-androideabi i686-linux-android x86_64-linux-android"
for target in $TARGETS; do
  rustup target add "$target"
done
```

### Stage 4: iOS Dependencies

```bash
# CocoaPods (requires Ruby)
if ! command -v pod; then
  gem install cocoapods
fi
```

## Production Usage Patterns

### Current Usage Flow

```bash
# 1. Add to project
echo '[task_config]
includes = ["git::https://github.com/joeblew999/utm-dev.git//.mise/tasks?ref=main"]' >> mise.toml

# 2. Initialize project
mise run init

# 3. Install tools
mise install

# 4. Setup SDKs
mise run setup

# 5. Start Windows VM
mise run vm:up

# 6. Build for all platforms
cargo tauri build --target aarch64-apple-darwin   # macOS
cargo tauri ios build                            # iOS
cargo tauri android build                        # Android
# Windows: RDP into VM and build there
```

### Integration with User Projects

Example `examples/tauri-basic/mise.toml`:

```toml
[task_config]
includes = ["git::https://github.com/joeblew999/utm-dev.git//.mise/tasks?ref=main"]

[tools]
# Tauri CLI
"cargo:tauri-cli" = {version = "2", os = ["macos", "windows"]}

# macOS/iOS only
xcodegen = {version = "latest", os = ["macos"]}
ruby = {version = "3.3", os = ["macos"]}

# Java for Android builds
java = "temurin-17.0.18+8"

[env]
ANDROID_HOME = "{{env.HOME}}/.android-sdk"
NDK_HOME = "{{env.HOME}}/.android-sdk/ndk/27.2.12479018"
```

## What's Missing

### 1. No Windows Build Automation

**Current state:** Windows builds require manual RDP into VM.

**What's missing:**
- SSH-based code sync to VM
- Automated build trigger from macOS
- Build artifact retrieval

**Recommendation:**
```bash
# New task: vm:build
mise run vm:build -- my-app

# Would:
# 1. Sync source to VM via scp
# 2. Trigger build via SSH
# 3. Wait for completion
# 4. Retrieve .exe/.msi artifacts
```

### 2. No VM Snapshot Management

**Current state:** Single VM state, no snapshots.

**What's missing:**
- Pre-build snapshots
- Snapshot restore
- Multiple VM configurations

**Recommendation:**
```bash
mise run vm:snapshot:create --name pre-build
mise run vm:snapshot:restore --name pre-build
mise run vm:snapshot:list
```

### 3. No Multi-VM Support

**Current state:** Single Windows 11 VM.

**What's missing:**
- Pi emulation support
- Android VM support
- Multiple Windows VMs (different versions)

**Recommendation:**
```toml
# In user's mise.toml
[utm-dev]
default_vm = "windows-11"
vms = ["windows-11", "raspbian", "android"]
```

### 4. No Health Monitoring

**Current state:** No VM health checks after boot.

**What's missing:**
- Periodic health checks
- Auto-restart on failure
- Resource monitoring (CPU, RAM, disk)

**Recommendation:**
```bash
mise run vm:health    # Check VM status
mise run vm:monitor   # Start background monitoring
```

### 5. Limited Error Recovery

**Current state:** Fails on network issues, partial installs.

**What's missing:**
- Resume from failed state
- Automatic retry with backoff
- Rollback on failure

### 6. No Configuration Management

**Current state:** Hardcoded ports, credentials, paths.

**What's missing:**
- User-configurable settings
- Environment-based configuration
- Per-project VM configs

**Recommendation:**
```toml
# .utm-dev.toml in project root
[vm]
ssh_port = 2222
rdp_port = 3389
user = "vagrant"
password = "vagrant"  # Or use mise secrets
```

### 7. No Logging/Observability

**Current state:** Logs to `.mise/logs/`, basic tee output.

**What's missing:**
- Structured logging
- Log rotation
- Remote log aggregation
- Debug mode

### 8. No Testing Framework

**Current state:** Manual testing.

**What's missing:**
- E2E tests for VM lifecycle
- Unit tests for bash scripts
- CI integration

## What We Can Add

### 1. SSH Build Automation

Add `vm:sync` and `vm:build` tasks:

```bash
#!/usr/bin/env bash
#MISE description="Sync project to VM and trigger build"

PROJECT_DIR="$(pwd)"
VM_HOST="127.0.0.1"
VM_PORT="2222"
VM_USER="vagrant"
VM_PASS="vagrant"
REMOTE_DIR="/vagrant/project"

# Install sshpass if not present
if ! command -v sshpass; then
  brew install hudochenkov/sshpass/sshpass
fi

# Sync via rsync over SSH
rsync -avz --delete \
  --exclude '.git' \
  --exclude 'node_modules' \
  --exclude 'target' \
  "${PROJECT_DIR}/" \
  "${VM_USER}@${VM_HOST}:${REMOTE_DIR}"

# Trigger build via SSH
sshpass -p "${VM_PASS}" ssh -p "${VM_PORT}" "${VM_USER}@${VM_HOST}" \
  "cd ${REMOTE_DIR} && cargo tauri build"

# Retrieve artifacts
scp -P "${VM_PORT}" "${VM_USER}@${VM_HOST}:${REMOTE_DIR}/src-tauri/target/release/*.exe" \
  "${PROJECT_DIR}/dist/"
```

### 2. VM Snapshot System

```bash
#!/usr/bin/env bash
#MISE description="Create VM snapshot"

ACTION="${1:-create}"
NAME="${2:-}"

case "${ACTION}" in
  create)
    [ -z "${NAME}" ] && die "Snapshot name required"
    osascript -e "tell application \"UTM\" to save state of virtual machine id \"${VM_UUID}\" as \"${NAME}\""
    ;;
  restore)
    [ -z "${NAME}" ] && die "Snapshot name required"
    osascript -e "tell application \"UTM\" to restore state of virtual machine id \"${VM_UUID}\" from \"${NAME}\""
    ;;
  list)
    osascript -e "tell application \"UTM\" to get states of virtual machine id \"${VM_UUID}\""
    ;;
esac
```

### 3. Health Monitoring

```bash
#!/usr/bin/env bash
#MISE description="Check VM health"

check_ssh() {
  curl -s --connect-timeout 2 "http://127.0.0.1:2222" >/dev/null && echo "SSH: OK" || echo "SSH: FAIL"
}

check_rdp() {
  curl -s --connect-timeout 2 "http://127.0.0.1:3389" >/dev/null && echo "RDP: OK" || echo "RDP: FAIL"
}

check_winrm() {
  curl -s --connect-timeout 2 "http://127.0.0.1:5985/wsman" >/dev/null && echo "WinRM: OK" || echo "WinRM: FAIL"
}

check_ssh
check_rdp
check_winrm
```

### 4. Configuration File Support

```bash
#!/usr/bin/env bash

# Load project-specific config
if [ -f ".utm-dev.toml" ]; then
  eval "$(toml2env < .utm-dev.toml)"
fi

# Defaults
VM_SSH_PORT="${VM_SSH_PORT:-2222}"
VM_RDP_PORT="${VM_RDP_PORT:-3389}"
VM_USER="${VM_USER:-vagrant}"
VM_PASS="${VM_PASS:-vagrant}"
```

### 5. Structured Logging

```bash
#!/usr/bin/env bash

LOG_LEVEL="${LOG_LEVEL:-info}"

log_json() {
  local level="$1"
  local msg="$2"
  printf '{"timestamp":"%s","level":"%s","message":"%s"}\n' \
    "$(date -u +%Y-%m-%dT%H:%M:%SZ)" \
    "${level}" \
    "${msg}"
}

log_info() { log_json "info" "$1"; }
log_error() { log_json "error" "$1" >&2; }
log_debug() { [ "${LOG_LEVEL}" = "debug" ] && log_json "debug" "$1"; }
```

## Comparison: Before vs After

### Before (Go CLI)

```bash
# Installation
go install github.com/joeblew999/utm-dev@latest

# Usage
utm-dev setup
utm-dev vm up
utm-dev build windows
```

**Pros:**
- Single binary
- All logic in one place
- Easy to distribute (single file)

**Cons:**
- Go dependency for users
- Large binary (~50MB)
- Separate versioning from mise
- No integration with mise ecosystem

### After (Mise Remote Plugin)

```bash
# Installation
echo '[task_config]
includes = ["git::https://github.com/joeblew999/utm-dev.git//.mise/tasks?ref=main"]' >> mise.toml

# Usage
mise run setup
mise run vm:up
```

**Pros:**
- No Go runtime needed
- Tiny footprint (bash scripts ~50KB)
- Integrated with mise ecosystem
- Automatic updates via git
- Consistent with other mise tasks
- Easy to extend/override locally

**Cons:**
- Requires mise installed
- Bash scripts less portable than Go
- Network dependency for remote includes

## Production Readiness Assessment

### What Works Well

1. **Idempotent Operations:** All tasks can be run multiple times safely
2. **Caching:** 6GB box cached at `~/.cache/utm-dev/`
3. **Logging:** All operations logged to `.mise/logs/`
4. **State Management:** VM state persisted to `.mise/state/vm.env`
5. **Clean Exit:** Trap handlers clean up on failure

### What Needs Work

1. **Error Messages:** Could be more actionable
2. **Retry Logic:** Only implemented for VM boot wait
3. **Documentation:** README is good but could have more examples
4. **Testing:** No automated tests
5. **CI/CD:** No GitHub Actions for testing changes

### Security Considerations

1. **Credentials:** Hardcoded `vagrant/vagrant` - should support mise secrets
2. **Network:** Port forwards to localhost only - good
3. **AppleScript:** No validation of VM configuration input

## Recommendations

### Immediate (Must Have)

1. **Add `vm:build` task** for automated Windows builds
2. **Support mise secrets** for VM credentials
3. **Add health check task** for VM monitoring
4. **Improve error messages** with actionable guidance

### Short Term (Should Have)

1. **VM snapshot management** for pre-build checkpoints
2. **SSH build automation** with rsync sync
3. **Configuration file support** for per-project settings
4. **Structured logging** with JSON output

### Long Term (Nice to Have)

1. **Multi-VM support** for Pi/Android emulation
2. **Web UI** for VM management
3. **Plugin system** for custom VM configurations
4. **Integration with CI/CD** for automated testing

## Conclusion

The mise remote plugin architecture is a **significant improvement** over the Go CLI approach:

- **Simpler distribution** via git includes
- **Smaller footprint** (bash vs Go binary)
- **Better integration** with mise ecosystem
- **Easier to extend** locally

The architecture is **production-ready for early adopters** but needs additional work for enterprise adoption, particularly around:
- Automated Windows builds
- Credential management
- Monitoring/observability
- Testing/CI

The foundation is solid, and the recommended additions would make this a **complete production solution** for cross-platform Tauri development.
