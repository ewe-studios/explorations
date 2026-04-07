# hermes_cli/ Deep Dive Exploration

**Source Directory:** `/home/darkvoid/Boxxed/@formulas/src.rust/src.llamacpp/src.ClaudOpen/hermes-agent/hermes_cli/`

**Status:** complete

---

## Module Overview

The `hermes_cli/` module is the unified command-line interface for Hermes Agent. This ~35,274 line module provides all user-facing commands for chat interaction, gateway management, configuration, authentication, skills, tools, plugins, and system administration.

Key features:
- **Interactive chat** - Terminal-based AI coding assistant
- **Gateway management** - Start/stop/install daemon service
- **Configuration** - YAML-based config with subcommands
- **Authentication** - Multi-provider auth management (Nous Portal, OpenRouter, Anthropic, etc.)
- **Skills system** - Browse, install, enable, disable skills
- **Tools management** - Configure toolsets and availability
- **Plugins** - Memory plugin management
- **Diagnostics** - Doctor command for system health checks
- **Profiles** - Named configuration presets

The CLI is the primary user interface for Hermes, with `hermes chat` being the main interactive mode.

---

## Directory Structure

### Core Entry Points

| File | Lines | Purpose |
|------|-------|---------|
| `__init__.py` | 15 | Package exports, version |
| `main.py` | 5,546 | Main entry point, CLI orchestration |
| `commands.py` | 1,033 | Core command implementations |

### Authentication & Setup

| File | Lines | Purpose |
|------|-------|---------|
| `auth.py` | 2,831 | Authentication system |
| `auth_commands.py` | 519 | Auth CLI commands |
| `setup.py` | 3,074 | Interactive setup wizard |
| `copilot_auth.py` | 294 | GitHub Copilot auth |
| `nous_subscription.py` | 524 | Nous Research subscription |
| `providers.py` | 519 | Provider management |
| `pairing.py` | 97 | Device pairing |

### Configuration

| File | Lines | Purpose |
|------|-------|---------|
| `config.py` | 2,725 | Configuration loading/saving |
| `profiles.py` | 1,070 | Configuration profiles |
| `env_loader.py` | 45 | Environment file loading |

### Model Management

| File | Lines | Purpose |
|------|-------|---------|
| `models.py` | 1,522 | Model definitions and listing |
| `model_switch.py` | 927 | Model switching commands |
| `model_normalize.py` | 361 | Model name normalization |
| `codex_models.py` | 176 | Codex/Responses API models |

### Gateway Integration

| File | Lines | Purpose |
|------|-------|---------|
| `gateway.py` | 2,237 | Gateway CLI commands |
| `status.py` | 424 | Status reporting |

### Skills & Tools

| File | Lines | Purpose |
|------|-------|---------|
| `skills_config.py` | 188 | Skills configuration |
| `skills_hub.py` | 1,219 | Skills discovery/management |
| `tools_config.py` | 1,800 | Tools configuration |

### Plugins & Memory

| File | Lines | Purpose |
|------|-------|---------|
| `plugins.py` | 609 | Plugin management |
| `plugins_cmd.py` | 689 | Plugin CLI commands |
| `memory_setup.py` | 521 | Memory initialization |

### Runtime & Providers

| File | Lines | Purpose |
|------|-------|---------|
| `runtime_provider.py` | 752 | Runtime provider selection |

### UI & Display

| File | Lines | Purpose |
|------|-------|---------|
| `banner.py` | 463 | ASCII art banner |
| `colors.py` | 38 | ANSI color codes |
| `curses_ui.py` | 172 | Terminal UI (optional) |
| `skin_engine.py` | 724 | Theme/skin system |
| `clipboard.py` | 360 | Clipboard integration |

### Diagnostics & Maintenance

| File | Lines | Purpose |
|------|-------|---------|
| `doctor.py` | 956 | System health checks |
| `logs.py` | 336 | Log viewing |
| `uninstall.py` | 326 | Uninstallation |
| `checklist.py` | 140 | Setup checklist |

### Utilities

| File | Lines | Purpose |
|------|-------|---------|
| `callbacks.py` | 283 | Callback handlers |
| `claw.py` | 568 | Claw/cursor helpers |
| `cron.py` | 275 | Cron CLI commands |
| `default_soul.py` | 11 | Default personality |
| `mcp_config.py` | 645 | MCP configuration |
| `webhook.py` | 260 | Webhook commands |

**Total:** ~35,274 lines across 40+ files

---

## Key Components

### 1. Main Entry Point (`main.py`)

CLI argument parsing and command dispatch.

**Key Structure:**
```python
def main():
    """Main entry point for hermes CLI."""
    parser = argparse.ArgumentParser(
        prog="hermes",
        description="Hermes Agent - AI coding assistant"
    )
    
    subparsers = parser.add_subparsers(dest="command", help="Available commands")
    
    # Chat command (default)
    chat_parser = subparsers.add_parser("chat", help="Interactive chat")
    
    # Gateway commands
    gateway_parser = subparsers.add_parser("gateway", help="Gateway management")
    gateway_sub = gateway_parser.add_subparsers()
    gateway_sub.add_parser("start", help="Start gateway")
    gateway_sub.add_parser("stop", help="Stop gateway")
    gateway_sub.add_parser("install", help="Install as service")
    
    # Setup wizard
    subparsers.add_parser("setup", help="Interactive setup")
    
    # Status
    subparsers.add_parser("status", help="Show system status")
    
    # Auth commands
    auth_parser = subparsers.add_parser("auth", help="Authentication management")
    
    # Skills commands
    skills_parser = subparsers.add_parser("skills", help="Skills management")
    
    # Tools commands
    tools_parser = subparsers.add_parser("tools", help="Tools configuration")
    
    # ... more subcommands
    
    args = parser.parse_args()
    dispatch(args)
```

### 2. Chat Command (`main.py`, `commands.py`)

Interactive terminal chat interface.

**Features:**
- Multi-line input support
- Syntax highlighting (rich library)
- Tool call visualization
- Streaming responses
- Session persistence
- Model switching mid-chat
- @-reference support

**Key Code:**
```python
def chat_command(args):
    """Run interactive chat session."""
    from run_agent import AIAgent
    
    # Load config and initialize
    config = load_config()
    provider = get_runtime_provider(config)
    
    # Create agent
    agent = AIAgent(
        model=args.model or config.get("model.name"),
        system_prompt=build_system_prompt(config),
        tools=get_enabled_tools(config),
    )
    
    # Load session history
    session_id = args.session or get_default_session()
    load_history(agent, session_id)
    
    # Main chat loop
    while True:
        try:
            user_input = read_user_input()
            if user_input in ("quit", "exit", "/q"):
                break
            
            # Process @-references
            user_input, references = resolve_references(user_input)
            
            # Run agent turn
            for event in agent.run(user_input):
                handle_event(event)
                
        except KeyboardInterrupt:
            print("\nUse /quit to exit")
```

### 3. Authentication System (`auth.py`, `auth_commands.py`)

Multi-provider authentication management.

**Supported Providers:**
| Provider | Auth Method | Storage |
|----------|-------------|---------|
| Nous Portal | OAuth | `~/.hermes/auth.json` |
| OpenRouter | API Key | Environment/Config |
| Anthropic | API Key | Environment/Config |
| Codex (ChatGPT) | OAuth | `~/.claude/.credentials.json` |
| Custom OpenAI | API Key + URL | Config |

**Key Commands:**
```bash
hermes auth login nous       # Login to Nous Portal
hermes auth logout nous      # Logout from Nous
hermes auth status           # Show auth status
hermes auth list-providers   # List available providers
hermes auth set-key anthropic KEY  # Set API key
```

**Auth File Format:**
```json
{
  "nous": {
    "access_token": "eyJ...",
    "refresh_token": "dGhpcyBpcyBh...",
    "expires_at": "2026-04-08T10:00:00Z"
  },
  "active_provider": "nous"
}
```

### 4. Setup Wizard (`setup.py`)

Interactive first-time setup.

**Flow:**
1. Welcome banner
2. Provider selection
3. Authentication (OAuth or API key)
4. Model selection
5. Toolset configuration
6. Skills selection
7. Gateway setup (optional)
8. Test message

**Key Function:**
```python
def run_setup_wizard():
    """Interactive setup wizard."""
    print(BANNER)
    print("Welcome to Hermes Agent!")
    
    # Step 1: Provider selection
    provider = select_provider()
    
    # Step 2: Authentication
    if provider == "nous":
        auth_via_nous()
    elif provider == "openrouter":
        enter_api_key("OPENROUTER_API_KEY")
    # ...
    
    # Step 3: Model selection
    model = select_model(provider)
    
    # Step 4: Toolset configuration
    toolsets = select_toolsets()
    
    # Step 5: Save config
    save_config({
        "provider": provider,
        "model": model,
        "toolsets": toolsets,
    })
    
    print("Setup complete! Run 'hermes chat' to start.")
```

### 5. Gateway Commands (`gateway.py`)

Gateway daemon management.

**Commands:**
```bash
hermes gateway              # Run in foreground
hermes gateway start        # Start background service
hermes gateway stop         # Stop service
hermes gateway restart      # Restart service
hermes gateway install      # Install as systemd service
hermes gateway status       # Show service status
hermes gateway logs         # View logs
```

**Systemd Installation:**
```python
def gateway_install(args):
    """Install gateway as systemd service."""
    if args.system:
        # System-wide service
        unit_file = "/etc/systemd/system/hermes-gateway.service"
        cmd = "sudo systemctl daemon-reload"
    else:
        # User service
        unit_file = "~/.config/systemd/user/hermes-gateway.service"
        cmd = "systemctl --user daemon-reload"
    
    # Write unit file
    write_unit_file(unit_file)
    
    # Enable and start
    run(cmd)
    run(f"systemctl {'--user ' if not args.system else ''}enable hermes-gateway")
    run(f"systemctl {'--user ' if not args.system else ''}start hermes-gateway")
```

### 6. Skills Management (`skills_hub.py`, `skills_config.py`)

Skills discovery and management.

**Commands:**
```bash
hermes skills list              # List installed skills
hermes skills browse            # Browse available skills
hermes skills search <query>    # Search skills
hermes skills install <skill>   # Install a skill
hermes skills enable <skill>    # Enable installed skill
hermes skills disable <skill>   # Disable skill
hermes skills uninstall <skill> # Remove skill
```

**Skill Sources:**
- `~/.hermes/skills/` - User-installed skills
- `skills/` - Bundled official skills
- `optional-skills/` - Optional official skills
- Remote repositories (GitHub)

### 7. Tools Configuration (`tools_config.py`)

Tool and toolset management.

**Commands:**
```bash
hermes tools list               # List available tools
hermes tools list-toolsets      # List toolsets
hermes tools enable <toolset>   # Enable toolset
hermes tools disable <toolset>  # Disable toolset
hermes tools status             # Show tool status
```

**Toolsets:**
- `terminal` - Terminal commands
- `file` - File operations
- `web` - Web search and extraction
- `browser` - Browser automation
- `code` - Code execution
- `memory` - Memory operations
- `mcp` - MCP client tools

### 8. Doctor Command (`doctor.py`)

System health diagnostics.

**Checks:**
```python
def doctor_command(args):
    """Run system diagnostics."""
    checks = [
        check_python_version,
        check_hermes_home,
        check_config_file,
        check_auth_status,
        check_model_access,
        check_tool_requirements,
        check_terminal_backend,
        check_gateway_status,
    ]
    
    results = []
    for check in checks:
        result = check()
        results.append(result)
        print_status(result)
    
    # Summary
    passed = sum(1 for r in results if r.passed)
    print(f"\n{passed}/{len(results)} checks passed")
```

### 9. Status Command (`status.py`)

System status reporting.

**Output:**
```
Hermes Agent Status
====================

Provider: Nous Research
Model: claude-sonnet-4-5
Active Toolsets: terminal, file, web

Gateway: running (PID 12345)
  - Telegram: connected
  - Discord: connected

Cron Jobs: 3 active
  - morning-standup: due in 8h
  - backup: due in 23h

Memory Provider: honcho
  - Connected: yes
  - Entries: 1,234
```

### 10. Configuration System (`config.py`)

YAML-based configuration.

**Config Structure:**
```yaml
# ~/.hermes/config.yaml

provider: nous
model:
  name: claude-sonnet-4-5
  base_url: null

toolsets:
  enabled:
    - terminal
    - file
    - web
  disabled: []

skills:
  enabled:
    - github
    - docker
  disabled: []

memory:
  provider: null  # null = builtin only

gateway:
  platforms:
    telegram:
      enabled: true
      token: "${TELEGRAM_BOT_TOKEN}"
```

### 11. Profiles (`profiles.py`)

Named configuration presets.

**Commands:**
```bash
hermes profiles list          # List profiles
hermes profiles create <name> # Create profile
hermes profiles use <name>    # Switch profile
hermes profiles delete <name> # Delete profile
```

**Use Cases:**
- `work` - Work models and tools
- `personal` - Personal preferences
- `rl-training` - RL training configuration
- `dev` - Development/testing setup

---

## CLI Command Reference

| Command | Description |
|---------|-------------|
| `hermes chat` | Interactive chat (default) |
| `hermes gateway` | Gateway management |
| `hermes setup` | Interactive setup |
| `hermes status` | System status |
| `hermes auth` | Authentication |
| `hermes skills` | Skills management |
| `hermes tools` | Tools configuration |
| `hermes plugins` | Plugin management |
| `hermes profiles` | Configuration profiles |
| `hermes cron` | Cron job management |
| `hermes doctor` | System diagnostics |
| `hermes logs` | View logs |
| `hermes update` | Update Hermes |
| `hermes uninstall` | Remove Hermes |

---

## Integration Points

### With Agent (`agent/`)
- Chat command creates AIAgent instances
- Model metadata from `agent/model_metadata.py`
- Skill commands use `agent/skill_commands.py`

### With Gateway (`gateway/`)
- Gateway commands control daemon
- Status queries gateway state

### With Tools (`tools/`)
- Tools configuration affects tool registry
- Tool requirements checked by doctor

### With Plugins (`plugins/`)
- Plugin discovery and loading
- Memory provider selection

---

## Related Files

**Individual File Explorations:**
- [main.md](./hermes_cli/main.md)
- [auth.md](./hermes_cli/auth.md)
- [config.md](./hermes_cli/config.md)
- [gateway.md](./hermes_cli/gateway.md)
- [skills_hub.md](./hermes_cli/skills_hub.md)
- [tools_config.md](./hermes_cli/tools_config.md)
- [doctor.md](./hermes_cli/doctor.md)

**Related Modules:**
- [agent/exploration.md](../agent/exploration.md) - Agent internals
- [gateway/exploration.md](../gateway/exploration.md) - Gateway system

---

*Deep dive created: 2026-04-07*
