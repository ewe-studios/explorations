# Tau CLI - Comprehensive Deep Dive Exploration

**Path:** `/home/darkvoid/Boxxed/@formulas/src.rust/src.Taubyte/tau-cli/`

**Version:** 1.2.x (based on tau dependency v1.2.0-pre)

**Language:** Go 1.21.1+

---

## Executive Summary

Tau CLI (`tau`) is the primary command-line interface for interacting with Taubyte-based clouds. It provides a comprehensive set of commands for managing projects, applications, and cloud resources. The CLI follows a modular architecture with separate packages for commands, flags, prompts, and library functions.

**Tagline:** *Local Coding Equals Global Production*

---

## Architecture Overview

### Directory Structure

```
tau-cli/
├── cli/                      # Main CLI application
│   ├── args/                 # Argument parsing utilities
│   ├── commands/             # Command implementations
│   │   ├── autocomplete/     # Shell autocomplete
│   │   ├── current/          # Current context commands
│   │   ├── dream/            # Dream integration
│   │   ├── exit/             # Exit commands
│   │   ├── login/            # Authentication
│   │   ├── resources/        # Resource management
│   │   │   ├── application/
│   │   │   ├── builds/
│   │   │   ├── database/
│   │   │   ├── domain/
│   │   │   ├── function/
│   │   │   ├── library/
│   │   │   ├── logs/
│   │   │   ├── messaging/
│   │   │   ├── network/
│   │   │   ├── project/
│   │   │   ├── service/
│   │   │   ├── smartops/
│   │   │   ├── storage/
│   │   │   └── website/
│   │   └── version/          # Version command
│   ├── common/               # Shared utilities
│   └── login/                # Login library
├── common/                   # Common utilities
├── constants/                # CLI constants
├── env/                      # Environment handling
├── flags/                    # Flag definitions
├── i18n/                     # Internationalization
├── images/                   # CLI images/assets
├── lib/                      # Library functions
│   ├── application/
│   ├── codefile/
│   ├── database/
│   ├── domain/
│   ├── dream/
│   ├── function/
│   ├── library/
│   ├── login/
│   ├── messaging/
│   ├── project/
│   ├── repository/
│   ├── service/
│   ├── smartops/
│   ├── storage/
│   └── website/
├── npm/                      # NPM wrapper
├── prompts/                  # Interactive prompts
├── singletons/               # Singleton patterns
├── states/                   # State management
├── table/                    # Table formatting
├── tests/                    # Test suite
└── validate/                 # Input validation
```

### Core Entry Points

**main.go:**
```go
func main() {
    err := cli.Run(os.Args...)
    if err != nil {
        log.Fatal(i18n.AppCrashed(err))
    }
}
```

**cli/run.go:**
```go
func Run(args ...string) error {
    app, err := New()
    if err != nil {
        return i18n.AppCreateFailed(err)
    }
    args = argsLib.ParseArguments(app.Flags, app.Commands, args...)
    return app.Run(args)
}
```

---

## Command Architecture

### Application Initialization (cli/new.go)

The CLI is built using the `urfave/cli/v2` framework with a modular command structure:

```go
func New() (*cli.App, error) {
    globalFlags := []cli.Flag{
        flags.Env,
        flags.Color,
    }

    app := &cli.App{
        UseShortOptionHandling: true,
        Flags:                  globalFlags,
        EnableBashCompletion:   true,
        Before: func(ctx *cli.Context) error {
            states.New(ctx.Context)
            // Color handling
        },
        Commands: []*cli.Command{
            login.Command,
            current.Command,
            exit.Command,
            dream.Command,
        },
    }

    // Attach resource commands
    common.Attach(app,
        project.New,
        application.New,
        network.New,
        database.New,
        domain.New,
        function.New,
        library.New,
        messaging.New,
        service.New,
        smartops.New,
        storage.New,
        website.New,
        builds.New,
        build.New,
        logs.New,
    )

    return app, nil
}
```

### Command Patterns

All resource commands follow a consistent pattern:

1. **Base Command** - Defines the resource type and common options
2. **CRUD Operations** - `new`, `list`, `edit`, `delete`, `select`
3. **Query Interface** - For accessing resource data

Example from `cli/commands/resources/project/base.go`:
```go
func (link) Base() (*cli.Command, []common.Option) {
    selected, err := env.GetSelectedProject()
    if err != nil {
        selected = "selected"
    }
    return common.Base(&cli.Command{
        Name:      "project",
        ArgsUsage: i18n.ArgsUsageName,
    }, options.NameFlagSelectedArg0(selected))
}
```

---

## Authentication System

### Login Flow (cli/commands/login/command.go)

The login system supports multiple authentication profiles:

```go
var Command = &cli.Command{
    Name: "login",
    Flags: flags.Combine(
        flags.Name,
        loginFlags.Token,
        loginFlags.Provider,
        loginFlags.New,
        loginFlags.SetDefault,
    ),
    ArgsUsage: i18n.ArgsUsageName,
    Action:    Run,
    Before:    options.SetNameAsArgs0,
}

func Run(ctx *cli.Context) error {
    _default, options, err := loginLib.GetProfiles()

    // New profile if --new or no selectable profiles
    if ctx.Bool(loginFlags.New.Name) || len(options) == 0 {
        return New(ctx, options)
    }

    // Selection logic
    var name string
    if ctx.IsSet(flags.Name.Name) {
        name = ctx.String(flags.Name.Name)
    } else {
        name, err = prompts.SelectInterface(options, loginPrompts.SelectAProfile, _default)
    }

    return Select(ctx, name, ctx.Bool(loginFlags.SetDefault.Name))
}
```

### Login Flags

- `--name` / `-n`: Profile name
- `--token`: Authentication token
- `--provider`: Auth provider
- `--new`: Create new profile
- `--set-default`: Set as default profile

---

## Environment Variables

| Variable | Default | Description |
|----------|---------|-------------|
| `TAUBYTE_PROJECT` | - | Selected project |
| `TAUBYTE_PROFILE` | - | Selected profile |
| `TAUBYTE_APPLICATION` | - | Selected application |
| `TAUBYTE_CONFIG` | `~/tau.yaml` | Config location |
| `TAUBYTE_SESSION` | `/tmp/tau-<shell-pid>` | Session location |
| `DREAM_BINARY` | `$GOPATH/dream` | Dream binary location |

---

## Key Dependencies

### Core Taubyte Dependencies

```go
require (
    github.com/taubyte/go-project-schema v0.9.3
    github.com/taubyte/go-seer v1.0.6
    github.com/taubyte/go-specs v0.10.8
    github.com/taubyte/http v0.10.5
    github.com/taubyte/tau v1.2.0-pre.0.20240715060437-2366343b2c22
    github.com/taubyte/utils v0.1.7
    github.com/taubyte/domain-validation v1.0.1
)
```

### UI/UX Libraries

- `github.com/pterm/pterm` - Terminal UI
- `github.com/AlecAivazis/survey/v2` - Interactive prompts
- `github.com/jedib0t/go-pretty/v6` - Table formatting
- `github.com/briandowns/spinner` - Loading spinners

### Git/Repository

- `github.com/taubyte/go-simple-git` - Git operations
- `github.com/go-git/go-git/v5` - Git library

---

## Resource Management

### Supported Resource Types

1. **Application** - Application lifecycle management
2. **Database** - Database configuration
3. **Domain** - Domain and SSL management
4. **Function** - Serverless functions
5. **Library** - Shared code libraries
6. **Messaging** - Pub/sub messaging
7. **Network** - Network configuration
8. **Service** - Service definitions
9. **SmartOps** - Smart operations
10. **Storage** - Storage buckets
11. **Website** - Website deployment

### Common Operations Pattern

Each resource supports:
- `tau <resource> new` - Create new resource
- `tau <resource> list` - List resources
- `tau <resource> select` - Select active resource
- `tau <resource> edit` - Edit resource config
- `tau <resource> delete` - Delete resource

---

## Dream Integration

Dream is the local development cloud. The CLI provides tight integration:

```go
// cli/commands/dream/command.go
dream.Command - Main dream command
dream/build/   - Build resources for dream
```

### Dream Commands

- `tau dream` - Dream management
- `tau dream build` - Build for dream
- `tau dream attach` - Attach to dream
- `tau dream inject` - Inject configurations

---

## Build & Installation

### Installation Methods

1. **NPM:**
   ```bash
   npm i @taubyte/cli
   ```

2. **Self-extracting:**
   ```bash
   curl https://get.tau.link/cli | sh
   ```

3. **Go Install:**
   ```bash
   go install github.com/taubyte/tau-cli@latest
   ```

4. **Build from Source:**
   ```bash
   git clone https://github.com/taubyte/tau-cli
   cd tau-cli
   go build -o ~/go/bin/tau
   ```

5. **Offline Version:**
   ```bash
   go build -o ~/go/bin/otau -tags=localAuthClient
   ```

---

## Testing Strategy

### Test Configuration

```bash
# All tests
go test -v ./...

# Coverage calculation
go test -v ./... -tags=localAuthClient,projectCreateable,localPatrick,cover,noPrompt \
  -coverprofile cover.out -coverpkg ./...

# HTML coverage report
go tool cover -html=cover.out

# Function coverage
go tool cover -func=cover.out
```

### Hot Reload Testing

Using `air` for hot reload tests:
```bash
cd tests
air
```

Configuration in `tests/.air.toml`:
```toml
cmd = "go test -v --run <Function|Database|...> [-tags=no_rebuild]"
```

### Build Tags

- `localAuthClient` - Local authentication
- `projectCreateable` - Enable project creation
- `localPatrick` - Local patrick service
- `cover` - Coverage reporting
- `noPrompt` - Disable interactive prompts

---

## Integration Points

### Core Tau Services

1. **Patrick** - HTTP server runtime
2. **Monkey** - Protocol handler
3. **Seer** - Observability
4. **Substrate** - Core runtime
5. **TNS** - Naming service
6. **Hoarder** - Storage service

### External Integrations

- **GitHub** - Repository management via `go-github`
- **GitLab** - Alternative Git provider
- **Docker** - Container operations
- **LibP2P** - P2P networking

---

## State Management

### Session State

The CLI maintains session state in `/tmp/tau-<shell-pid>`:
- Current project
- Current profile
- Current application
- Authentication tokens

### Configuration State

Configuration stored in `~/tau.yaml`:
- Profile definitions
- Default settings
- Resource mappings

---

## Internationalization (i18n)

The CLI has a dedicated i18n package for:
- Error messages
- Help text
- Prompts
- Status messages

---

## Production Deployment Patterns

### CI/CD Integration

The CLI is designed for CI/CD pipelines:
- Non-interactive mode via flags
- Environment variable configuration
- Profile-based authentication
- Offline mode support

### Multi-Profile Management

Support for multiple cloud profiles:
```bash
tau login --new --set-default  # Create new default
tau login production            # Switch to production
tau login staging               # Switch to staging
```

---

## Security Considerations

### Authentication

- JWT-based tokens
- ECDSA signing (ES256)
- Secure token storage
- Profile isolation

### Network Security

- TLS for all API communications
- Domain validation integration
- mTLS support (via core tau)

---

## Maintainers

Based on the codebase structure and related packages:
- Samy Fodil (@samyfodil)
- Sam Stoltenberg (@skelouse)
- Tafseer Khan (@tafseer-khan)
- Aron Jalbuena (@arontaubyte)

---

## Related Documentation

- [tau.how](https://tau.how/docs/tau) - Official documentation
- [GitHub Repository](https://github.com/taubyte/tau-cli)
- [NPM Package](https://www.npmjs.com/package/@taubyte/cli)
- [Go Package](https://pkg.go.dev/github.com/taubyte/tau-cli)

---

## Summary

Tau CLI is a comprehensive, production-ready CLI tool that serves as the primary interface for managing Taubyte cloud resources. Its modular architecture, extensive flag system, and integration with the Dream local development environment make it suitable for both local development and production deployments.

**Key Strengths:**
- Modular command architecture
- Multi-profile authentication
- Comprehensive resource management
- Local/production parity with Dream
- Extensive testing infrastructure
- CI/CD friendly
