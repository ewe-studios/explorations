# Dream - Local Development Cloud Deep Dive Exploration

**Path:** `/home/darkvoid/Boxxed/@formulas/src.rust/src.Taubyte/dream/`

**Version:** 1.1.2

**Language:** Node.js wrapper + Go binary

---

## Executive Summary

**Dream** is Taubyte's local development cloud - a self-contained, offline-capable Tau environment that runs on your local machine. It enables developers to build, test, and debug cloud applications locally before deploying to production, ensuring parity between development and production environments.

**Key Concept:** Dream brings the full Tau cloud experience to your laptop.

---

## Architecture Overview

### Directory Structure

```
dream/
├── .github/           # GitHub Actions
├── images/            # Assets and logos
├── tau/               # Tau integration
├── .gitignore
├── .gitmodules
├── .goreleaser.darwin.yml
├── .goreleaser.linux.yml
├── .goreleaser.windows.yml
├── index.js           # NPM wrapper entry point
├── LICENSE
├── .npmignore
├── package.json
├── package-lock.json
└── README.md
```

### Component Architecture

Dream consists of two main components:

1. **Go Binary** - The actual Dream runtime
2. **Node.js Wrapper** - NPM package for easy installation

---

## Node.js Wrapper (index.js)

The NPM package serves as an installer and wrapper for the Go binary:

### Installation Flow

```javascript
const binaryDir = path.join(__dirname, "bin");
const binaryPath = path.join(binaryDir,
  process.platform === "win32" ? "dream.exe" : "dream");
const versionFilePath = path.join(binaryDir, "version.txt");
```

### Version Management

```javascript
function versionMatches() {
  if (!fs.existsSync(versionFilePath)) {
    return false;
  }
  const installedVersion = fs.readFileSync(versionFilePath, "utf-8").trim();
  return installedVersion === packageVersion;
}
```

### Auto-Download and Installation

```javascript
async function downloadAndExtractBinary() {
  if (binaryExists() && versionMatches()) {
    return;
  }

  const { os: currentOs, arch: currentArch } = parseAssetName();
  const assetName = `dream_${version}_${currentOs}_${currentArch}.tar.gz`;
  const assetUrl = `https://github.com/taubyte/dream/releases/download/v${version}/${assetName}`;

  // Download with progress bar
  const { data, headers } = await axios({
    url: assetUrl,
    method: "GET",
    responseType: "stream",
  });

  const progressBar = new ProgressBar(
    "-> downloading [:bar] :percent :etas", {
      width: 40,
      complete: "=",
      incomplete: " ",
      renderThrottle: 1,
      total: parseInt(headers["content-length"]),
    }
  );

  // Extract tarball
  await tar.x({
    file: tarPath,
    C: binaryDir,
  });
}
```

### Platform Detection

```javascript
function parseAssetName() {
  let os, arch;

  // OS Detection
  if (process.platform === "darwin") os = "darwin";
  else if (process.platform === "linux") os = "linux";
  else if (process.platform === "win32") os = "windows";

  // Architecture Detection
  if (process.arch === "x64") arch = "amd64";
  else if (process.arch === "arm64") arch = "arm64";

  return { os, arch };
}
```

---

## Package Configuration

### package.json

```json
{
  "name": "@taubyte/dream",
  "version": "1.1.2",
  "description": "Node wrapper for taubyte/dream",
  "bin": {
    "dream": "./index.js"
  },
  "main": "index.js",
  "keywords": [
    "serverless",
    "cloud",
    "hosting",
    "webassembly",
    "wasm"
  ],
  "dependencies": {
    "@taubyte/dream": "^0.1.7",
    "axios": "^1.6.5",
    "progress": "^2.0.3",
    "tar": "^6.2.0"
  }
}
```

---

## Installation Methods

### NPM Installation

```bash
npm install -g @taubyte/dream
```

This installs Dream globally and makes the `dream` command available.

### Direct Binary

Download from [GitHub Releases](https://github.com/taubyte/dream/releases):
- `dream_darwin_amd64.tar.gz`
- `dream_darwin_arm64.tar.gz`
- `dream_linux_amd64.tar.gz`
- `dream_linux_arm64.tar.gz`
- `dream_windows_amd64.tar.gz`

### Go Installation

```bash
go install github.com/taubyte/dream@latest
```

---

## Usage

### Starting Dream

```bash
dream
```

This starts the local cloud environment.

### Dream with Universe

```bash
dream with <universe-name>
```

Creates or connects to a named universe (isolated environment).

### Integration with Tau CLI

```bash
# Start Dream
dream

# In another terminal, use Tau CLI
tau project new my-project
tau application new my-app
```

---

## Universe Concept

A **Universe** in Dream is an isolated cloud environment:

### Universe Characteristics

- **Isolation** - Each universe has its own configuration
- **Resources** - Separate resource namespaces
- **Networking** - Independent P2P network
- **State** - Persistent local state

### Universe Management

```bash
# Create new universe
dream with production-like

# Switch universe
dream with staging

# List universes
dream list
```

---

## Local Cloud Services

Dream runs the full Tau service stack locally:

### Core Services

| Service | Port | Description |
|---------|------|-------------|
| **Auth** | - | Authentication service |
| **TNS** | - | Tau Naming Service |
| **Hoarder** | - | Storage service |
| **Seer** | - | Observability/telemetry |
| **Substrate** | - | Core runtime |
| **Patrick** | 4242 | HTTP server |
| **Monkey** | - | Protocol handler |

### Network Configuration

```
main port: 4242  # Primary HTTP
lite port: 4262  # Lightweight HTTP
```

---

## Development Workflow

### Local Development Cycle

1. **Start Dream**
   ```bash
   dream with my-project
   ```

2. **Configure Resources**
   ```bash
   tau function new my-function
   tau website new my-site
   ```

3. **Develop Locally**
   - Code your application
   - Test against local Dream

4. **Deploy to Production**
   - Push configuration
   - Deploy to cloud

### Hot Reload

Dream supports hot reload for:
- Function code changes
- Website asset updates
- Configuration changes

---

## Integration with Tau CLI

### Command Flow

```
┌─────────────┐     ┌─────────────┐     ┌─────────────┐
│  Tau CLI    │────▶│    Dream    │────▶│  Local Tau  │
│  Commands   │     │   Runtime   │     │   Services  │
└─────────────┘     └─────────────┘     └─────────────┘
```

### Example Workflow

```bash
# Start Dream
dream with my-app

# Create project
tau project new my-project

# Create application
tau application new my-app

# Create function
tau function new hello

# Deploy function (to local Dream)
tau function deploy hello

# Test function
curl http://localhost:4242/hello
```

---

## Configuration System

### Configuration Location

Dream stores configuration in:
- **macOS:** `~/Library/Application Support/dream/`
- **Linux:** `~/.config/dream/`
- **Windows:** `%APPDATA%/dream/`

### Configuration Files

- `config.yaml` - Main configuration
- `universes/` - Universe-specific configs
- `keys/` - Authentication keys
- `state/` - Runtime state

---

## Offline Capabilities

Dream is designed for offline development:

### Offline Features

- **No Internet Required** - Full cloud stack locally
- **Local DNS** - Built-in DNS resolution
- **Local Certificates** - Self-signed SSL
- **Persistent State** - Data survives restarts

### Offline Mode Flag

```bash
# Build offline version
go build -tags=localAuthClient -o dream
```

---

## P2P Network Simulation

Dream simulates a distributed P2P network locally:

### Virtual Nodes

Dream can simulate multiple nodes:
- Each node runs specific services
- Nodes communicate via loopback
- P2P protocols work as in production

### Bootstrap Configuration

```yaml
cloud:
  p2p:
    bootstrap:
      shape:
        all:
          nodes:
            - node1
            - node2
```

---

## Domain Management

### Local Domains

Dream provides local domain resolution:

```
*.g.pom.ac  -> localhost:4242
```

### Certificate Generation

Dream auto-generates SSL certificates:

```bash
# Auto-generated on first run
cloud.domain.validation.generate()
```

---

## Resource Types

Dream supports all Tau resource types:

### Application Resources

| Resource | Description |
|----------|-------------|
| **Function** | Serverless functions (Wasm) |
| **Website** | Static websites |
| **Database** | Key-value databases |
| **Storage** | Object storage |
| **Messaging** | Pub/sub messaging |
| **Domain** | Domain configuration |

---

## Debugging Features

### Logging

Dream provides comprehensive logging:

```bash
# View logs
dream logs

# Follow logs
dream logs -f

# Filter by service
dream logs --service patrick
```

### Debug Mode

```bash
# Start with debug output
dream --debug

# Enable verbose logging
export DREAM_DEBUG=true
```

---

## Build System

### GoReleaser Configuration

Dream uses GoReleaser for cross-platform builds:

**Targets:**
- darwin/amd64
- darwin/arm64
- linux/amd64
- linux/arm64
- windows/amd64

### Build Tags

- `localAuthClient` - Offline authentication
- `debug` - Debug output

---

## Environment Variables

| Variable | Default | Description |
|----------|---------|-------------|
| `DREAM_BINARY` | `$GOPATH/dream` | Binary location |
| `DREAM_CONFIG` | `~/.config/dream/` | Config directory |
| `DREAM_DEBUG` | `false` | Debug mode |
| `DREAM_LOG_LEVEL` | `info` | Log level |

---

## Production Parity

Dream ensures development/production parity:

### Same Codebase

- Same Tau binary
- Same service implementations
- Same protocols

### Configuration Portability

- YAML configs work identically
- Resource definitions are compatible
- Secrets management is consistent

### Network Behavior

- P2P protocols identical
- Service discovery works the same
- Load balancing behavior matches

---

## Use Cases

### Individual Development

- Local testing without cloud costs
- Offline development
- Rapid iteration

### Team Development

- Shared universe configurations
- Consistent development environments
- Reduced cloud dependency

### CI/CD

- Local CI testing
- Pre-deployment validation
- Integration testing

---

## Troubleshooting

### Common Issues

1. **Port Already in Use**
   ```bash
   # Change port
   dream --port 8080
   ```

2. **Universe Not Found**
   ```bash
   # Create universe
   dream with new-universe
   ```

3. **Binary Not Found**
   ```bash
   # Reinstall
   npm uninstall -g @taubyte/dream
   npm install -g @taubyte/dream
   ```

---

## Related Projects

- **Tau CLI** - Primary CLI interface
- **Taucorder** - Production runtime
- **Spore Drive** - Infrastructure as code
- **Config Compiler** - Configuration processing

---

## Summary

Dream is Taubyte's local development cloud that brings the full Tau experience to your local machine.

**Key Benefits:**
- Full cloud stack locally
- Offline-capable development
- Production parity
- Universe isolation
- Easy installation via NPM
- Cross-platform support

**Ideal For:**
- Local development and testing
- CI/CD pipelines
- Offline work
- Team development environments
