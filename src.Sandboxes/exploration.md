# Sandboxes Collection Exploration

---
location: /home/darkvoid/Boxxed/@formulas/src.rust/src.llamacpp/src.Sandboxes/
repository: N/A - Collection of 9 separate repositories
explored_at: 2026-06-01T00:00:00Z
language: Multi-language (Rust, Python, TypeScript, Go, Shell)
projects_count: 9
---

# Project Exploration: Sandboxes Collection

## Overview

This exploration covers a curated collection of 9 distinct sandbox-related projects, each addressing different aspects of secure, isolated execution environments for AI agents and code. The collection spans from macOS-specific sandboxing tools to cloud-native virtual machine sandboxes, agent harnesses, and browser extensions.

The projects can be categorized into four main themes:

1. **Sandbox Security & Isolation** (`agent-safehouse`, `CubeSandbox`, `shuru`) - Focus on providing secure execution environments with varying isolation levels, from macOS Seatbelt profiles to KVM-based microVMs.

2. **Agent Frameworks & Orchestration** (`deer-flow`, `flue`, `ml-intern`) - Higher-level abstractions that orchestrate AI agents with sandboxed execution capabilities, sub-agent spawning, and tool integrations.

3. **Developer Tools & Productivity** (`Kami`, `superhq`, `superpowers`) - End-user applications that leverage sandboxing for safe AI agent execution, document generation, and browser-based AI interactions.

Together, these projects represent a comprehensive ecosystem for safe AI agent execution, addressing concerns from kernel-level isolation to user-facing applications, with strong emphasis on security, performance, and developer experience.


## Repository Collection

| Project | Language | Primary Purpose | Remote URL |
|---------|----------|-----------------|------------|
| agent-safehouse | Shell/Bash | macOS Seatbelt sandboxing for LLM agents | git@github.com:eugene1g/agent-safehouse.git |
| CubeSandbox | Rust/Go | High-performance VM sandbox service | git@github.com:TencentCloud/CubeSandbox.git |
| deer-flow | Python/TypeScript | Super agent harness with sub-agents | git@github.com:bytedance/deer-flow.git |
| flue | TypeScript | Sandbox agent framework | git@github.com:withastro/flue.git |
| Kami | Python/HTML | Document design system | git@github.com:tw93/Kami.git |
| ml-intern | Python | ML research agent with sandbox tools | git@github.com:huggingface/ml-intern.git |
| shuru | Rust | Local microVM sandbox for AI agents | git@github.com:superhq-ai/shuru.git |
| superhq | Rust | Sandboxed AI agent orchestration platform | git@github.com:superhq-ai/superhq.git |
| superpowers | TypeScript | Chrome extension for AI interactions | git@github.com:superhq-ai/superpowers.git |

---

## Project 1: agent-safehouse

### Repository Details

- **Location:** `/home/darkvoid/Boxxed/@formulas/src.rust/src.llamacpp/src.Sandboxes/agent-safehouse/`
- **Remote:** git@github.com:eugene1g/agent-safehouse.git
- **Primary Language:** Shell/Bash
- **License:** Apache 2.0

### Directory Structure

```
agent-safehouse/
├── bin/
│   ├── lib/
│   │   ├── bootstrap/      # Bootstrap scripts
│   │   ├── cli/            # CLI implementation
│   │   ├── commands/       # Command handlers
│   │   ├── policy/         # Policy management
│   │   ├── runtime/        # Runtime utilities
│   │   └── support/        # Support utilities
│   └── safehouse.sh        # Main entry point
├── profiles/
│   ├── 00-base.sb          # Base restrictions
│   ├── 10-system-runtime.sb
│   ├── 20-network.sb
│   ├── 30-toolchains/      # Language-specific profiles
│   │   ├── bun.sb
│   │   ├── deno.sb
│   │   ├── elixir.sb
│   │   ├── go.sb
│   │   ├── java.sb
│   │   ├── node.sb
│   │   ├── python.sb
│   │   ├── ruby.sb
│   │   └── rust.sb
│   ├── 40-shared/          # Shared agent profiles
│   ├── 50-integrations-core/
│   ├── 55-integrations-optional/
│   ├── 60-agents/          # Agent-specific profiles
│   │   ├── aider.sb
│   │   ├── claude-code.sb
│   │   ├── cline.sb
│   │   ├── codex.sb
│   │   ├── copilot-cli.sb
│   │   ├── cursor-agent.sb
│   │   └── goose.sb
│   └── 65-apps/            # App-specific profiles
├── tests/
│   ├── e2e/                # End-to-end tests
│   ├── policy/             # Policy tests
│   └── surface/            # Surface tests
└── docs/                   # VitePress documentation
```

### Architecture

agent-safehouse uses a layered policy composition approach where sandbox profiles are assembled from modular components based on the target agent and user requirements.

### Component Breakdown

#### Policy Assembler
- **Location:** `bin/lib/policy/`
- **Purpose:** Composes sandbox profiles from modular components
- **Dependencies:** macOS sandbox-exec, profile files
- **Dependents:** CLI commands

#### Profile System
- **Location:** `profiles/`
- **Purpose:** Modular Seatbelt policy definitions
- **Key Features:**
  - Layered profile loading (00-65 prefix order)
  - Toolchain-specific permissions
  - Integration profiles for 1Password, Docker, etc.
  - Agent-specific hardening

#### CLI Interface
- **Location:** `bin/safehouse.sh`, `bin/lib/cli/`
- **Purpose:** User-facing command interface
- **Key Commands:**
  - `safehouse <agent>` - Run agent in sandbox
  - `--add-dirs-ro` - Add read-only directories
  - `--append-profile` - Add custom policies

### Entry Points

#### Main Script
- **File:** `bin/safehouse.sh`
- **Description:** Primary entry point that assembles policies and executes agents
- **Flow:**
  1. Parse CLI arguments
  2. Load base profile (00-base.sb)
  3. Load toolchain profiles
  4. Load integration profiles
  5. Load agent-specific profile
  6. Apply user overrides
  7. Execute `sandbox-exec` with composed policy

### External Dependencies

| Dependency | Version | Purpose |
|------------|---------|---------|
| sandbox-exec | macOS built-in | Core sandbox mechanism |
| pnpm | latest | Package management |
| bash | 4+ | Script runtime |

### Configuration

- Environment variables:
  - `SAFEHOUSE_APPEND_PROFILE` - Path to custom profile
  - `HOME_DIR` - Home directory for path resolution
- Configuration files:
  - `profiles/*.sb` - Sandboxed policy definitions

### Testing

- **Framework:** bats (Bash Automated Testing System)
- **Test Types:**
  - E2E tests for each supported agent
  - Policy validation tests
  - Surface/integration tests
- **CI:** GitHub Actions with macOS runners

### Key Insights

- Uses deny-first model with explicit allow rules
- Supports 15+ AI agents out of the box
- Git worktree auto-detection for shared Git metadata access
- Machine-specific overrides via append profiles


---

## Project 2: CubeSandbox

### Repository Details

- **Location:** `/home/darkvoid/Boxxed/@formulas/src.rust/src.llamacpp/src.Sandboxes/CubeSandbox/`
- **Remote:** git@github.com:TencentCloud/CubeSandbox.git
- **Primary Language:** Rust/Go
- **License:** Apache 2.0

### Directory Structure

```
CubeSandbox/
├── agent/                  # Agent component (Rust)
│   ├── src/
│   │   ├── main.rs
│   │   ├── sandbox.rs      # Sandbox lifecycle
│   │   ├── mount.rs        # Mount management
│   │   ├── network.rs      # Network configuration
│   │   └── tracer.rs       # System call tracing
│   ├── libs/               # Shared libraries
│   └── rustjail/           # Container runtime
├── CubeAPI/                # REST API Gateway (Rust)
│   └── src/
│       ├── main.rs
│       ├── handlers/       # API endpoints
│       ├── models/         # Data models
│       └── middleware/
├── Cubelet/                # Node agent (Go)
│   ├── cmd/                # CLI commands
│   ├── pkg/                # Packages
│   │   ├── cubelet/        # Core cubelet logic
│   │   ├── container/      # Container management
│   │   └── network/        # Network plugin
│   └── services/           # Background services
├── CubeMaster/             # Cluster orchestrator (Go)
│   ├── cmd/
│   └── pkg/
│       ├── scheduler/      # Resource scheduling
│       └── selector/       # Node selection
├── CubeNet/                # eBPF virtual switch (Go/C)
│   ├── cubevs/             # Virtual switch implementation
│   └── src/                # eBPF programs
├── CubeProxy/              # Reverse proxy (OpenResty/Lua)
│   └── lua/                # Lua scripts
├── CubeShim/               # Containerd shim (Rust)
│   └── shim/
├── hypervisor/             # Cloud Hypervisor integration
│   └── src/                # VMM components
├── deploy/                 # Deployment configs
└── examples/               # Usage examples
```

### Architecture

CubeSandbox uses a distributed microservices architecture with clear separation between control plane (CubeMaster, CubeAPI) and data plane (Cubelet, CubeHypervisor, CubeVS).

### Component Breakdown

#### CubeMaster
- **Location:** `CubeMaster/`
- **Purpose:** Cluster orchestrator managing resource allocation and scheduling
- **Key Features:**
  - Multi-node cluster management
  - Template-based sandbox creation
  - Resource scheduling
- **Dependencies:** etcd (implied), gRPC

#### Cubelet
- **Location:** `Cubelet/`
- **Purpose:** Node-level agent managing local sandboxes
- **Key Features:**
  - Sandbox lifecycle management
  - Network plugin integration
  - Storage management
  - Image caching
- **Dependencies:** containerd, runc

#### CubeAPI
- **Location:** `CubeAPI/`
- **Purpose:** E2B-compatible REST API gateway
- **Key Features:**
  - E2B SDK compatibility
  - High-concurrency request handling
  - Template management
- **Dependencies:** Rust async runtime

#### CubeHypervisor
- **Location:** `hypervisor/`
- **Purpose:** KVM-based microVM management
- **Key Features:**
  - Firecracker-based microVMs
  - virtio-fs for filesystem sharing
  - vsock for agent communication
- **Dependencies:** KVM, Linux kernel

#### CubeVS
- **Location:** `CubeNet/`
- **Purpose:** eBPF-based virtual switch
- **Key Features:**
  - Kernel-level network isolation
  - Traffic filtering policies
  - NAT and port mapping
- **Dependencies:** Linux eBPF, BPF toolchain

### Entry Points

#### CubeMaster
- **File:** `CubeMaster/cmd/cubemaster/main.go`
- **Flow:**
  1. Load configuration
  2. Connect to etcd/consensus
  3. Initialize scheduler
  4. Start gRPC server
  5. Handle template/sandbox operations

#### Cubelet
- **File:** `Cubelet/cmd/cubelet/main.go`
- **Flow:**
  1. Register with CubeMaster
  2. Initialize runtime
  3. Start sandbox event loop
  4. Manage local sandboxes

#### CubeAPI
- **File:** `CubeAPI/src/main.rs`
- **Flow:**
  1. Parse configuration
  2. Initialize handlers
  3. Start HTTP server
  4. Route E2B-compatible requests

### Data Flow

1. Client sends Create Sandbox request to CubeAPI
2. CubeAPI forwards to CubeMaster for scheduling
3. CubeMaster selects node and instructs Cubelet
4. Cubelet creates microVM via CubeHypervisor
5. MicroVM boots and signals readiness
6. Execution requests flow through CubeProxy to sandbox

### External Dependencies

| Dependency | Version | Purpose |
|------------|---------|---------|
| Rust | 1.70+ | Core implementation |
| Go | 1.21+ | Cubelet/CubeMaster |
| KVM | Linux kernel | Virtualization |
| eBPF | Linux 5.x | Network virtualization |
| containerd | 1.7+ | Container runtime |
| Cloud Hypervisor | 35+ | VMM base |

### Configuration

- Files:
  - `configs/*.yaml` - Service configurations
  - `deploy/one-click/` - Deployment scripts
- Environment:
  - `E2B_API_URL` - API endpoint
  - `CUBE_TEMPLATE_ID` - Default template

### Testing

- **Framework:** Go testing, Rust cargo test
- **Integration Tests:** `Cubelet/integration/`
- **E2E Tests:** GitHub Actions workflows

### Key Insights

- <60ms cold start via snapshot cloning
- <5MB memory overhead per sandbox
- True kernel isolation (not namespace-based)
- E2B SDK drop-in replacement
- Production-tested at Tencent Cloud scale


---

## Project 3: deer-flow

### Repository Details

- **Location:** `/home/darkvoid/Boxxed/@formulas/src.rust/src.llamacpp/src.Sandboxes/deer-flow/`
- **Remote:** git@github.com:bytedance/deer-flow.git
- **Primary Language:** Python/TypeScript
- **License:** MIT

### Directory Structure

```
deer-flow/
├── backend/                # Python backend
│   ├── app/
│   │   ├── channels/       # IM integrations
│   │   └── gateway/        # Gateway API
│   ├── packages/
│   │   └── harness/        # Core agent harness
│   ├── tests/              # Comprehensive test suite
│   └── docs/               # Architecture docs
├── frontend/               # Next.js frontend
│   ├── src/
│   │   ├── app/            # App router
│   │   ├── components/     # React components
│   │   └── core/           # Core utilities
│   └── tests/
├── docker/                 # Docker configurations
├── skills/
│   └── public/             # Built-in skills
│       ├── deep-research/
│       ├── slide-creation/
│       └── web-design/
└── scripts/                # Utility scripts
```

### Architecture

deer-flow uses a microservices architecture with a Gateway API, LangGraph-based agent runtime, and multiple sandbox backends.

### Component Breakdown

#### Gateway API
- **Location:** `backend/app/gateway/`
- **Purpose:** FastAPI-based REST API and WebSocket gateway
- **Key Features:**
  - Model management
  - Skill registry
  - Thread management
  - File uploads
- **Dependencies:** FastAPI, Uvicorn, LangChain

#### Agent Harness
- **Location:** `backend/packages/harness/`
- **Purpose:** Core agent execution framework
- **Key Features:**
  - Lead agent orchestration
  - Sub-agent spawning
  - Tool routing
  - Context management
- **Dependencies:** LangGraph, LangChain

#### Memory System
- **Location:** `backend/app/memory/`
- **Purpose:** Long-term memory and context persistence
- **Key Features:**
  - Cross-session memory
  - Preference learning
  - Knowledge accumulation
- **Dependencies:** SQLite/PostgreSQL

#### Channels
- **Location:** `backend/app/channels/`
- **Purpose:** IM platform integrations
- **Supported:** Telegram, Slack, Feishu/Lark, WeChat, WeCom
- **Dependencies:** Platform-specific SDKs

### Entry Points

#### Backend
- **File:** `backend/debug.py` or via `make dev`
- **Flow:**
  1. Load configuration (config.yaml)
  2. Initialize Gateway
  3. Start LangGraph server (standard mode) OR
  4. Initialize Gateway with embedded runtime (gateway mode)
  5. Start HTTP/WebSocket servers

#### Frontend
- **File:** `frontend/src/app/page.tsx`
- **Flow:**
  1. Initialize Next.js app
  2. Connect to Gateway API
  3. Render chat interface
  4. Handle real-time updates

### Data Flow

1. User sends message via Frontend
2. Gateway routes to Lead Agent
3. Lead Agent plans and spawns Sub-Agents
4. Sub-Agents execute tools in Sandboxes
5. Results synthesized and returned

### External Dependencies

| Dependency | Version | Purpose |
|------------|---------|---------|
| Python | 3.12+ | Backend runtime |
| Node.js | 22+ | Frontend runtime |
| LangGraph | latest | Agent orchestration |
| LangChain | latest | LLM abstractions |
| Docker | latest | Sandbox execution |
| PostgreSQL | 14+ | Database |

### Configuration

- **File:** `config.yaml`
- **Sections:**
  - `models` - LLM configurations
  - `sandbox` - Execution mode
  - `channels` - IM integrations
  - `skills` - Skill registry
- **Environment:** `.env` for secrets

### Testing

- **Backend:** pytest with 100+ test files
- **Frontend:** Vitest + Playwright
- **Coverage:** E2E, integration, unit tests
- **CI:** GitHub Actions with multiple workflows

### Key Insights

- Reached #1 on GitHub Trending (Feb 2026)
- Supports 5+ IM channels
- Sub-agent architecture for complex tasks
- Context engineering with auto-compaction
- Skills-based extensibility

---

## Project 4: flue

### Repository Details

- **Location:** `/home/darkvoid/Boxxed/@formulas/src.rust/src.llamacpp/src.Sandboxes/flue/`
- **Remote:** git@github.com:withastro/flue.git
- **Primary Language:** TypeScript
- **License:** Not specified (Astro project)

### Directory Structure

```
flue/
├── packages/
│   ├── sdk/                # Core SDK
│   │   └── src/
│   │       ├── client.ts   # Client utilities
│   │       ├── cloudflare.ts
│   │       └── node.ts
│   ├── cli/                # CLI tool
│   │   └── bin/
│   │       └── flue.ts     # CLI entry
│   └── connectors/         # Third-party connectors
│       └── src/
│           └── daytona.ts  # Daytona integration
├── examples/
│   ├── assistant/          # Simple agent example
│   └── hello-world/        # Hello world example
├── apps/
│   └── www/                # Documentation website
└── docs/                   # Deployment guides
```

### Architecture

flue provides a TypeScript-first framework for building sandbox agents with support for virtual, local, and container sandboxes.

### Component Breakdown

#### SDK
- **Location:** `packages/sdk/`
- **Purpose:** Core agent framework
- **Key Features:**
  - Session management
  - Sandbox abstraction
  - Tool definitions
  - Result schemas

#### CLI
- **Location:** `packages/cli/`
- **Purpose:** Build and run agents
- **Commands:**
  - `flue run <agent>` - Execute locally
  - `flue build --target <platform>` - Build for deployment

#### Connectors
- **Location:** `packages/connectors/`
- **Purpose:** Third-party integrations
- **Current:** Daytona connector for container sandboxes

### Entry Points

#### Agent Definition
- **File:** `.flue/agents/*.ts`
- **Structure:**
  ```typescript
  export const triggers = { webhook: true };
  export default async function ({ init, payload }: FlueContext) {
    const session = await init();
    return await session.prompt(...);
  }
  ```

#### CLI
- **File:** `packages/cli/bin/flue.ts`
- **Flow:**
  1. Parse agent definition
  2. Load sandbox configuration
  3. Initialize session
  4. Execute agent handler

### External Dependencies

| Dependency | Version | Purpose |
|------------|---------|---------|
| TypeScript | 5.x | Language |
| Valibot | latest | Schema validation |
| Daytona SDK | latest | Container sandboxes |
| just-bash | latest | Virtual sandbox |

### Configuration

- **File:** `.flue/config` (optional)
- **Environment:** `.env` for secrets

### Key Insights

- Virtual sandbox by default (no containers)
- Cloudflare Workers + Durable Objects support
- Schema-validated results
- Skill-based agent enhancement


---

## Project 5: Kami

### Repository Details

- **Location:** `/home/darkvoid/Boxxed/@formulas/src.rust/src.llamacpp/src.Sandboxes/Kami/`
- **Remote:** git@github.com:tw93/Kami.git
- **Primary Language:** Python (document generation)
- **License:** MIT

### Directory Structure

```
Kami/
├── assets/
│   ├── demos/              # Example documents
│   ├── fonts/              # Custom fonts
│   ├── images/             # Logo and graphics
│   └── templates/          # HTML templates
│       ├── letter.html
│       ├── long-doc.html
│       ├── one-pager.html
│       ├── portfolio.html
│       ├── resume.html
│       └── slides.py       # Slide generation
├── references/
│   ├── design.md           # Design system docs
│   └── writing.md          # Writing guidelines
├── scripts/
│   ├── build.py            # Build script
│   └── package-skill.sh    # Skill packaging
├── index.html              # Main web interface
└── styles.css              # Shared styles
```

### Architecture

Kami is a document design system for AI agents, generating professional documents from natural language descriptions.

### Component Breakdown

#### Template System
- **Location:** `assets/templates/`
- **Purpose:** Document layout definitions
- **Variants:**
  - One-pager (single page overview)
  - Long-doc (multi-page document)
  - Letter (formal correspondence)
  - Portfolio (project showcase)
  - Resume (curriculum vitae)
  - Slides (presentation deck)

#### Design System
- **Location:** `references/design.md`
- **Principles:**
  - Warm parchment canvas (#f5f4ed)
  - Ink blue accent (#1B365D)
  - Serif for headings, sans for body
  - Editorial whitespace

#### Font Assets
- **Location:** `assets/fonts/`
- **Chinese:** TsangerJinKai02 serif
- **English:** Newsreader serif + Inter sans

### Entry Points

#### Skill Integration
- **Install:** `npx skills add tw93/kami`
- **Trigger:** Natural language description
- **Examples:**
  - "make a one-pager for my startup"
  - "build me a resume"
  - "design a slide deck for my talk"

#### Python Script
- **File:** `assets/templates/slides.py`
- **Purpose:** Programmatic slide generation

### External Dependencies

| Dependency | Version | Purpose |
|------------|---------|---------|
| WeasyPrint | latest | HTML to PDF |
| Python | 3.x | Slide generation |

### Key Insights

- Agent skill for document generation
- 6 document types with Chinese/English variants
- Three inline SVG diagram types
- Focus on print-quality output

---

## Project 6: ml-intern

### Repository Details

- **Location:** `/home/darkvoid/Boxxed/@formulas/src.rust/src.llamacpp/src.Sandboxes/ml-intern/`
- **Remote:** git@github.com:huggingface/ml-intern.git
- **Primary Language:** Python
- **License:** Not specified

### Directory Structure

```
ml-intern/
├── agent/
│   ├── core/
│   │   ├── agent_loop.py    # Main agent loop
│   │   ├── session.py       # Session management
│   │   ├── tools.py         # Tool definitions
│   │   └── doom_loop.py     # Pattern detection
│   ├── context_manager/
│   │   └── manager.py       # Context compaction
│   ├── tools/
│   │   ├── dataset_tools.py
│   │   ├── docs_tools.py
│   │   ├── github_tools.py
│   │   ├── hf_repo_tools.py
│   │   ├── sandbox_client.py
│   │   └── research_tool.py
│   ├── prompts/
│   │   └── system_prompt.yaml
│   └── utils/
│       ├── terminal_display.py
│       └── reliability_checks.py
├── backend/
│   ├── main.py              # Backend API
│   ├── routes/
│   └── session_manager.py
├── frontend/
│   ├── src/
│   │   ├── App.tsx
│   │   └── components/
│   └── package.json
└── configs/
    └── main_agent_config.json
```

### Architecture

ml-intern is an ML-focused agent with deep Hugging Face ecosystem integration and sandboxed tool execution.

### Component Breakdown

#### Agent Loop
- **Location:** `agent/core/agent_loop.py`
- **Purpose:** Main agent execution orchestration
- **Features:**
  - Max 300 iterations
  - Tool call parsing
  - Approval checks
  - Context compaction at 170k tokens

#### Tool Router
- **Location:** `agent/core/tools.py`
- **Purpose:** Tool execution dispatch
- **Tools:**
  - Hugging Face docs/research
  - Repository operations
  - Dataset access
  - Job management
  - Sandbox execution

#### Context Manager
- **Location:** `agent/context_manager/manager.py`
- **Purpose:** Message history and auto-compaction
- **Features:**
  - Auto-compaction at threshold
  - Session upload to HF
  - Message history management

#### Doom Loop Detector
- **Location:** `agent/core/doom_loop.py`
- **Purpose:** Detect repetitive tool patterns
- **Action:** Inject corrective prompts

### Entry Points

#### CLI
- **Command:** `ml-intern ["prompt"] [options]`
- **Modes:**
  - Interactive (default)
  - Headless (single prompt)
- **Options:**
  - `--model` - Model selection
  - `--max-iterations` - Iteration limit
  - `--no-stream` - Disable streaming

#### Backend
- **File:** `backend/main.py`
- **Purpose:** API server for remote access

### Data Flow

1. User submits request via CLI
2. Agent Loop retrieves context from ContextManager
3. LLM call via litellm
4. Tool calls executed via ToolRouter
5. Results added to context
6. Loop continues until complete

### External Dependencies

| Dependency | Version | Purpose |
|------------|---------|---------|
| litellm | latest | LLM abstraction |
| smolagents | latest | Agent framework |
| huggingface-hub | latest | HF integration |
| PyGithub | latest | GitHub API |
| uv | latest | Package management |

### Configuration

- **File:** `configs/main_agent_config.json`
- **Sections:**
  - `model_name` - Default model
  - `mcpServers` - MCP server configs

### Key Insights

- Hugging Face ecosystem integration
- Context auto-compaction at 170k tokens
- Doom loop detection for reliability
- Sandbox tool for code execution
- Session persistence with HF upload


---

## Project 7: shuru

### Repository Details

- **Location:** `/home/darkvoid/Boxxed/@formulas/src.rust/src.llamacpp/src.Sandboxes/shuru/`
- **Remote:** git@github.com:superhq-ai/shuru.git
- **Primary Language:** Rust
- **License:** Not specified

### Directory Structure

```
shuru/
├── crates/
│   ├── shuru-cli/          # CLI implementation
│   │   └── src/
│   │       └── main.rs
│   ├── shuru-vm/           # VM management
│   │   └── src/
│   ├── shuru-linux/        # Linux KVM backend
│   │   └── src/
│   ├── shuru-darwin/       # macOS Virtualization.framework
│   │   └── src/
│   ├── shuru-guest/        # Guest agent
│   │   └── src/
│   ├── shuru-store/        # Checkpoint storage
│   │   └── src/
│   ├── shuru-proxy/        # Network proxy
│   │   └── src/
│   ├── shuru-sdk/          # TypeScript SDK
│   └── shuru-proto/        # Protocol definitions
├── packages/
│   └── sdk/                # TypeScript SDK package
│       └── src/
├── kernel/
│   └── shuru_defconfig     # Kernel configuration
├── scripts/
│   ├── build-kernel.sh
│   └── prepare-rootfs.sh
└── www/                    # Documentation website
```

### Architecture

shuru provides local microVM sandboxes using Apple Virtualization.framework on macOS and KVM on Linux.

### Component Breakdown

#### CLI
- **Location:** `crates/shuru-cli/`
- **Purpose:** User-facing command interface
- **Commands:**
  - `shuru run` - Run in sandbox
  - `shuru checkpoint` - Save/restore state
  - `shuru upgrade` - Update runtime

#### VM Management
- **Location:** `crates/shuru-vm/`
- **Purpose:** VM lifecycle management
- **Features:**
  - VM creation/destruction
  - Resource allocation
  - Checkpoint management

#### Platform Backends
- **Darwin:** `crates/shuru-darwin/` - Apple Virtualization.framework
- **Linux:** `crates/shuru-linux/` - KVM backend

#### Guest Agent
- **Location:** `crates/shuru-guest/`
- **Purpose:** In-VM command execution
- **Features:**
  - Command execution
  - Port forwarding
  - File operations

#### Network Proxy
- **Location:** `crates/shuru-proxy/`
- **Purpose:** API key injection and traffic filtering
- **Features:**
  - Secret substitution
  - Host allowlisting
  - HTTPS proxy

#### Store
- **Location:** `crates/shuru-store/`
- **Purpose:** Checkpoint storage with content-addressed deduplication

### Entry Points

#### CLI
- **File:** `crates/shuru-cli/src/main.rs`
- **Flow:**
  1. Parse arguments
  2. Load checkpoint (if specified)
  3. Initialize VM backend
  4. Configure networking (if allowed)
  5. Mount directories (if specified)
  6. Execute command
  7. Cleanup on exit

#### SDK
- **File:** `packages/sdk/src/index.ts`
- **Usage:**
  ```typescript
  const sb = await Sandbox.start({ from: "python-env" });
  const result = await sb.exec("python3 script.py");
  ```

### Data Flow

1. User runs `shuru run --allow-net`
2. CLI creates VM via platform backend
3. Guest agent executes command
4. Network requests go through proxy with secret substitution
5. Results returned to user

### External Dependencies

| Dependency | Version | Purpose |
|------------|---------|---------|
| Rust | 1.75+ | Implementation |
| Apple Virtualization.framework | macOS 14+ | macOS backend |
| KVM | Linux kernel | Linux backend |
| virtiofsd | latest | Filesystem sharing |

### Configuration

- **File:** `shuru.json`
- **Sections:**
  - `cpus`, `memory`, `disk_size` - Resources
  - `allow_net` - Network access
  - `mounts` - Directory mappings
  - `ports` - Port forwarding
  - `secrets` - API key injection
  - `network.allow` - Host allowlist

### Key Insights

- macOS-first with experimental Linux support
- Checkpoint system for environment reuse
- Secret proxy for API key protection
- VirtioFS for fast directory sharing
- Agent skill integration

---

## Project 8: superhq

### Repository Details

- **Location:** `/home/darkvoid/Boxxed/@formulas/src.rust/src.llamacpp/src.Sandboxes/superhq/`
- **Remote:** git@github.com:superhq-ai/superhq.git
- **Primary Language:** Rust
- **License:** GNU AGPL v3.0

### Directory Structure

```
superhq/
├── crates/
│   ├── gpui-terminal/      # Terminal component
│   ├── superhq-remote-client/
│   ├── superhq-remote-host/
│   └── superhq-remote-proto/
├── src/
│   ├── main.rs             # Application entry
│   ├── agents/
│   │   ├── mod.rs
│   │   ├── claude.rs       # Claude Code integration
│   │   ├── codex.rs        # Codex integration
│   │   ├── opencode.rs     # Opencode integration
│   │   └── pi.rs           # Pi integration
│   ├── sandbox/
│   │   ├── mod.rs
│   │   ├── manager.rs      # Sandbox manager
│   │   ├── service.rs      # Sandbox service
│   │   ├── auth_gateway.rs # Auth proxy
│   │   └── secrets.rs      # Secret management
│   ├── ui/
│   │   ├── mod.rs
│   │   ├── components/     # UI components
│   │   ├── terminal/       # Terminal UI
│   │   ├── settings/       # Settings UI
│   │   └── remote/         # Remote control UI
│   ├── db/
│   │   ├── mod.rs
│   │   ├── schema.rs
│   │   └── queries.rs
│   └── assets.rs
├── migrations/             # Database migrations
├── specs/                  # Feature specs
├── assets/
│   ├── icons/              # Agent icons
│   └── themes/             # UI themes
└── website/                # Landing page
```

### Architecture

superhq is a desktop application for running multiple AI agents in isolated sandboxes using GPUI (from Zed editor) and shuru.

### Component Breakdown

#### GPUI Interface
- **Location:** `src/ui/`
- **Purpose:** GPU-accelerated UI (from Zed editor)
- **Features:**
  - Terminal panels
  - File tree
  - Review panel (diff view)
  - Settings UI

#### Agent Integrations
- **Location:** `src/agents/`
- **Supported:**
  - Claude Code
  - OpenAI Codex
  - Opencode
  - Pi
- **Features:**
  - OAuth handling
  - API key management
  - Terminal-based execution

#### Sandbox Manager
- **Location:** `src/sandbox/`
- **Purpose:** shuru VM orchestration
- **Features:**
  - Workspace isolation
  - Port management
  - File change tracking

#### Auth Gateway
- **Location:** `src/sandbox/auth_gateway.rs`
- **Purpose:** Secure API credential handling
- **Features:**
  - Token injection
  - OAuth token refresh
  - No secrets in sandbox

#### Database
- **Location:** `src/db/`
- **Purpose:** SQLite with AES-256-GCM encryption
- **Stores:**
  - Workspace configs
  - Secrets
  - Port mappings
  - Agent settings

### Entry Points

#### Desktop App
- **File:** `src/main.rs`
- **Flow:**
  1. Initialize GPUI
  2. Load database
  3. Check first-run setup
  4. Initialize sandbox service
  5. Launch main window

#### Sandbox Service
- **File:** `src/sandbox/service.rs`
- **Purpose:** Background sandbox management

### Data Flow

1. User opens new agent tab
2. Sandbox created via shuru
3. User sends prompt
4. Auth Gateway injects credentials
5. Request forwarded to LLM API
6. Response displayed in terminal

### External Dependencies

| Dependency | Version | Purpose |
|------------|---------|---------|
| GPUI | latest | UI framework |
| shuru | latest | Sandbox runtime |
| SQLite | 3.x | Database |
| Rust | 1.75+ | Implementation |

### Configuration

- **Database:** SQLite with migrations
- **Settings:** UI-based configuration
- **Secrets:** AES-256-GCM encrypted in DB

### Key Insights

- Multiple agents side-by-side
- Secure auth gateway (agents never see keys)
- Port forwarding management
- Review panel for file changes
- Keyboard-first navigation


---

## Project 9: superpowers

### Repository Details

- **Location:** `/home/darkvoid/Boxxed/@formulas/src.rust/src.llamacpp/src.Sandboxes/superpowers/`
- **Remote:** git@github.com:superhq-ai/superpowers.git
- **Primary Language:** TypeScript
- **License:** MIT

### Directory Structure

```
superpowers/
├── src/
│   ├── App.tsx             # Main app component
│   ├── main.tsx            # Entry point
│   ├── index.css           # Global styles
│   ├── components/
│   │   ├── ChatMessage.tsx
│   │   ├── PromptInput.tsx
│   │   ├── ToolCallView.tsx
│   │   ├── ModelSelector.tsx
│   │   ├── PlannerPanel.tsx
│   │   ├── SlashCommands.tsx
│   │   └── ui/             # UI primitives
│   ├── hooks/
│   │   ├── useAgent.ts
│   │   ├── useModels.ts
│   │   ├── useScreenshot.ts
│   │   └── useView.ts
│   ├── lib/
│   │   ├── agent.ts        # Agent logic
│   │   ├── agent-instance.ts
│   │   ├── browser-actions.ts
│   │   ├── browser-handlers.ts
│   │   ├── browser-tools.ts
│   │   ├── content.ts      # Content script
│   │   ├── background.ts   # Background script
│   │   ├── sidebar.ts      # Sidebar injection
│   │   ├── llm/            # LLM clients
│   │   └── streaming-tool-parser.ts
│   ├── types/
│   │   ├── agent.ts
│   │   ├── browser.ts
│   │   └── messages.ts
│   ├── views/
│   │   ├── Chat.tsx
│   │   ├── History.tsx
│   │   └── Settings.tsx
│   └── services/
│       └── llm.ts
├── public/
│   ├── manifest.json       # Extension manifest
│   ├── icons/              # Extension icons
│   └── superpowers.svg
├── website/                # Landing page
└── sidebar.html            # Sidebar entry
```

### Architecture

superpowers is a Chrome extension that provides AI capabilities directly in the browser with local and cloud LLM support.

### Component Breakdown

#### Sidebar UI
- **File:** `src/App.tsx`, `sidebar.html`
- **Purpose:** Main user interface
- **Features:**
  - Chat interface
  - Model selection
  - Tool call visualization
  - Planner panel

#### Agent Logic
- **Location:** `src/lib/agent.ts`
- **Purpose:** Agent orchestration
- **Features:**
  - Message streaming
  - Tool execution
  - Conversation history

#### Browser Tools
- **Location:** `src/lib/browser-tools.ts`
- **Purpose:** Browser automation
- **Features:**
  - Screenshot capture
  - DOM interaction
  - Tab management
  - Page navigation

#### Content Script
- **File:** `src/lib/content.ts`
- **Purpose:** Page interaction
- **Features:**
  - Inject sidebar
  - Page scraping
  - DOM manipulation

#### Background Script
- **File:** `src/lib/background.ts`
- **Purpose:** Extension lifecycle
- **Features:**
  - Context menu
  - Keyboard shortcuts
  - Tab management

### Entry Points

#### Extension
- **Manifest:** `public/manifest.json`
- **Entry Points:**
  - `sidebar.html` - Sidebar panel
  - `src/main.tsx` - React app
  - `src/lib/background.ts` - Background worker
  - `src/lib/content.ts` - Content script

#### Keyboard Shortcut
- **Shortcut:** `Cmd+Shift+U` / `Ctrl+Shift+U`
- **Action:** Toggle sidebar

### Data Flow

1. User presses keyboard shortcut
2. Content script toggles sidebar
3. User sends message
4. Context captured (screenshot, DOM)
5. Request sent to LLM
6. Response streamed back
7. Tool calls executed in browser

### External Dependencies

| Dependency | Version | Purpose |
|------------|---------|---------|
| Chrome APIs | latest | Extension APIs |
| React | 18+ | UI framework |
| TypeScript | 5.x | Language |
| Vite | latest | Build tool |

### Configuration

- **Extension:** `public/manifest.json`
- **Settings:** In-app configuration
- **Storage:** Chrome storage API

### Key Insights

- Browser-native AI experience
- Works with Ollama (local) or Gemini (cloud)
- Screenshot and DOM access
- Zero ecosystem lock-in
- Privacy-first (data stays local)


---

## Cross-Project Comparisons

### Sandbox Technologies

| Project | Technology | Isolation Level | Startup Time |
|---------|------------|-----------------|--------------|
| agent-safehouse | macOS Seatbelt | Process-level | Instant |
| CubeSandbox | KVM MicroVMs | Kernel-level | <60ms |
| shuru | Apple VZ/KVM | VM-level | ~1s |
| deer-flow | Docker/K8s | Container-level | ~2s |
| ml-intern | Container | Container-level | Variable |

### Agent Support

| Project | Claude | Codex | Gemini | Custom |
|---------|--------|-------|--------|--------|
| agent-safehouse | Built-in | Built-in | N/A | Extensible |
| superhq | Full | Full | N/A | Via config |
| deer-flow | Via config | Via config | Via config | Yes |
| superpowers | N/A | N/A | Built-in | Via OpenAI |

### Deployment Targets

| Project | Local | Cloud | CI/CD | Browser |
|---------|-------|-------|-------|---------|
| agent-safehouse | macOS | N/A | N/A | N/A |
| CubeSandbox | Linux | Kubernetes | N/A | N/A |
| deer-flow | Docker | Docker/K8s | N/A | N/A |
| flue | Node | Cloudflare | GitHub Actions | N/A |
| shuru | macOS/Linux | N/A | N/A | N/A |
| superhq | macOS | N/A | N/A | N/A |
| superpowers | N/A | N/A | N/A | Chrome |

## Key Insights

1. **Isolation Spectrum:** The collection spans from process-level isolation (agent-safehouse Seatbelt) to full kernel-level isolation (CubeSandbox KVM), allowing trade-offs between security and performance.

2. **E2B Compatibility:** CubeSandbox provides drop-in E2B SDK compatibility, enabling migration from proprietary sandboxes.

3. **Multi-Agent Orchestration:** deer-flow and superhq both support running multiple AI agents simultaneously, with deer-flow focusing on sub-agent spawning and superhq on workspace isolation.

4. **Platform Specialization:** Several projects are platform-specific (agent-safehouse for macOS, shuru for macOS with experimental Linux), while others are cross-platform.

5. **Security Models:** Projects vary in their threat models - from agent-safehouse's "practical least privilege" to CubeSandbox's "hardware-level isolation" to superhq's "auth gateway" pattern.

6. **Integration Patterns:** Common patterns include:
   - Secret proxy/gateway for API key protection
   - Checkpoint/snapshot systems for environment reuse
   - VirtioFS for efficient filesystem sharing
   - Agent skill packaging for discoverability

## Open Questions

1. **Standardization:** Is there potential for shared protocols or interfaces between these sandbox implementations?

2. **Performance Benchmarks:** How do the different isolation technologies compare under realistic AI agent workloads?

3. **Security Audits:** Have any of these projects undergone formal security audits, particularly the kernel-level isolation implementations?

4. **Interoperability:** Could deer-flow's agent harness work with CubeSandbox's VM backend instead of Docker?

5. **Mobile Support:** Are there plans for iOS/Android sandboxing support in any of these projects?

6. **WebAssembly:** None of the projects appear to use WASM for sandboxing - is this an opportunity?

7. **Resource Limits:** How do the projects handle resource exhaustion attacks from untrusted agent code?

## Conclusion

The Sandboxes collection represents a diverse and comprehensive exploration of secure execution environments for AI agents. From lightweight process isolation to full virtualization, from CLI tools to desktop applications to browser extensions, these projects cover the full spectrum of sandboxing approaches. They reflect the growing recognition that safe AI agent execution requires robust isolation mechanisms, and that different use cases demand different trade-offs between security, performance, and usability.

For engineers looking to integrate sandboxing into their AI agent workflows, this collection provides options across multiple dimensions:
- **For macOS development:** agent-safehouse or shuru
- **For high-performance cloud deployment:** CubeSandbox
- **For complex agent orchestration:** deer-flow
- **For desktop IDE experience:** superhq
- **For browser-based AI:** superpowers
- **For ML research:** ml-intern
- **For document generation:** Kami
- **For TypeScript-first development:** flue

The projects demonstrate that the AI agent ecosystem is maturing rapidly, with strong emphasis on security, observability, and developer experience. The common patterns around secret management, checkpoint systems, and skill-based extensibility suggest converging best practices in the field.
