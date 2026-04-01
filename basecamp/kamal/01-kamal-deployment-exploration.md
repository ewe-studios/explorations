---
location: /home/darkvoid/Boxxed/@formulas/src.UIFrameworks/src.basecamp/kamal
repository: git@github.com:basecamp/kamal.git
explored_at: 2026-03-29
language: Ruby
framework: SSHKit, Docker
category: Deployment Automation
---

# Kamal Deployment System - Exploration

## Overview

Kamal is a **zero-downtime deployment tool** that works with any containerized web application. It uses SSH to execute commands across multiple servers and [kamal-proxy](https://github.com/basecamp/kamal-proxy) to seamlessly switch traffic between containers. Originally built for Rails apps, Kamal works with any Dockerized application.

### Key Value Proposition

- **Zero-Downtime Deploys**: Traffic switches atomically via proxy
- **Multi-Server**: Deploy to hundreds of servers in parallel
- **Bare Metal to Cloud**: Works on any SSH-accessible Linux server
- **No Vendor Lock-in**: Direct Docker control, no Kubernetes complexity
- **Role-Based**: Different server types (web, workers, accessories)
- **Asset Handling**: Automatic asset compression and caching
- **Secrets Management**: Integration with 1Password, Bitwarden, AWS Secrets Manager, etc.

## Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                    Kamal CLI (Ruby)                             │
│                                                                 │
│  ┌─────────────────┐  ┌─────────────────┐  ┌─────────────────┐ │
│  │  Commander      │  │  Configuration  │  │  Commands       │ │
│  │  (orchestrator) │  │  (deploy.yml)   │  │  (SSH scripts)  │ │
│  └────────┬────────┘  └─────────────────┘  └─────────────────┘ │
│           │                                                     │
│           │ SSHKit (parallel SSH)                               │
└───────────┼─────────────────────────────────────────────────────┘
            │
    ┌───────┼────────────┬────────────────┬────────────────┐
    │       │            │                │                │
    ▼       ▼            ▼                ▼                ▼
┌────────┐ ┌────────┐ ┌────────┐  ┌────────────┐  ┌────────────┐
│ Web 1  │ │ Web 2  │ │ Web N  │  │ Worker 1   │  │ Accessory  │
│        │ │        │ │        │  │            │  │ (Redis)    │
│ ┌────┐ │ │ ┌────┐ │ │ ┌────┐ │  │ ┌────┐     │  │            │
│ │app │ │ │ │app │ │ │ │app │ │  │ │app │     │  │ ┌────┐     │
│ └─┬──┘ │ │ └─┬──┘ │ │ └─┬──┘ │  │ └─┬──┘     │  │ │redis│    │
│   │    │ │   │    │ │   │    │  │   │        │  │ └────┘     │
│ ┌─▼────┐ │ │ ┌─▼────┐ │ │ ┌─▼────┐ │  │        │  │            │
│ │proxy │ │ │ │proxy │ │ │ │proxy │ │  │        │  │            │
│ └─┬────┘ │ │ └─┬────┘ │ │ └─┬────┘ │  │        │  │            │
│   │:80  │ │ │   │:80  │ │ │   │:80  │  │        │  │            │
│ ┌─▼────┐ │ │ ┌─▼────┐ │ │ ┌─▼────┐ │  │        │  │            │
│ │ traef│ │ │ │ traef│ │ │ │ traef│ │  │        │  │            │
│ └──────┘ │ │ └──────┘ │ │ └──────┘ │  │        │  │            │
└────────┘ └────────┘ └────────┘  └────────────┘  └────────────┘
    │            │            │
    └────────────┴────────────┘
              │
              ▼
         Load Balancer / DNS
```

## Monorepo Structure

```
kamal/
├── lib/
│   └── kamal/
│       ├── cli/                    # CLI command handlers (Commander.js style)
│       │   ├── main.rb             # Root command
│       │   ├── app.rb              # Application deploys
│       │   ├── proxy.rb            # Proxy management
│       │   ├── accessory.rb        # Accessory services
│       │   ├── build.rb            # Image building
│       │   ├── deploy.rb           # Full deploy workflow
│       │   ├── prune.rb            # Cleanup old images/containers
│       │   ├── lock.rb             # Deploy locking
│       │   ├── secrets.rb          # Secrets management
│       │   ├── healthcheck/        # Health check utilities
│       │   └── ...
│       ├── commands/               # Shell command generators
│       │   ├── app.rb              # Docker commands for apps
│       │   ├── proxy.rb            # Proxy commands
│       │   ├── builder.rb          # Build commands
│       │   ├── docker.rb           # Raw Docker commands
│       │   ├── hook.rb             # Hook script execution
│       │   └── ...
│       ├── configuration/          # Configuration parsing
│       │   ├── servers.rb          # Server definitions
│       │   ├── role.rb             # Role configuration
│       │   ├── env.rb              # Environment variables
│       │   ├── builder.rb          # Build configuration
│       │   ├── proxy.rb            # Proxy configuration
│       │   ├── validator/          # YAML schema validation
│       │   └── docs/               # Configuration documentation (YAML schemas)
│       ├── commander/              # Orchestration logic
│       │   └── specifics.rb        # Target host/role filtering
│       ├── secrets/                # Secrets provider integrations
│       │   ├── adapters/
│       │   │   ├── base.rb
│       │   │   ├── 1password.rb
│       │   │   ├── bitwarden.rb
│       │   │   ├── aws_secrets_manager.rb
│       │   │   ├── doppler.rb
│       │   │   └── ...
│       ├── utils.rb                # Utility functions
│       ├── docker.rb               # Docker helper methods
│       └── env_file.rb             # .env file parsing
├── test/
│   ├── cli/                        # CLI tests
│   ├── commands/                   # Command generation tests
│   ├── configuration/              # Config parsing tests
│   └── fixtures/                   # Test deploy.yml files
├── kamal.gemspec                   # Gem specification
├── Gemfile                         # Dependencies
└── README.md                       # Documentation
```

## Core Concepts

### 1. Configuration

Kamal uses a YAML configuration file (`deploy.yml`):

```yaml
# Basic configuration
service: myapp
image: myorg/myapp
servers:
  - 192.168.0.1
  - 192.168.0.2
  - 192.168.0.3
env:
  secret:
    - RAILS_MASTER_KEY
    - DATABASE_URL

# Optional: Roles for different server types
roles:
  web:
    servers:
      - 192.168.0.1
      - 192.168.0.2
    proxy: true  # Run kamal-proxy on these hosts
  workers:
    servers:
      - 192.168.0.3
    cmd: bundle exec sidekiq

# Optional: Accessory services
accessories:
  redis:
    image: redis:7
    port: 6379
    volumes:
      - redis_data:/data
  mysql:
    image: mysql:8
    env:
      MYSQL_ROOT_PASSWORD: secret

# Optional: Asset handling
assets:
  roles:
    - web
  path: /public/assets

# Optional: Builder configuration
builder:
  arch: amd64  # or arm64, or hybrid for multi-arch
  remote: ssh://build-server  # Optional remote builder
```

### 2. Roles

Roles define different server types with specific configurations:

```ruby
class Kamal::Configuration::Role
  attr_reader :name, :servers, :proxy, :cmd, :env, :labels

  def initialize(config, name:)
    @name = name
    @servers = config.servers.for_role(name)
    @proxy = config.proxy if name == "web"
    @cmd = config.cmd_for_role(name)
    @env = config.env_for_role(name)
    @labels = config.labels_for_role(name)
  end

  def env_args(host)
    # Build --env flags for Docker run
    env.render(host).map { |k, v| ["--env", "#{k}=#{v}"] }.flatten
  end

  def label_args
    # Build --label flags for Docker run
    labels.map { |k, v| ["--label", "#{k}=#{v}"] }.flatten
  end

  def container_prefix
    "#{service}-#{name}-#{destination}"
  end
end
```

### 3. Docker Commands

Kamal generates Docker commands for deployment:

```ruby
class Kamal::Commands::App
  def run(hostname: nil)
    docker :run,
      "--detach",
      "--restart unless-stopped",
      "--name", container_name,
      "--network", "kamal",
      *([ "--hostname", hostname ] if hostname),
      "--env", "KAMAL_CONTAINER_NAME=\"#{container_name}\"",
      "--env", "KAMAL_VERSION=\"#{config.version}\"",
      "--env", "KAMAL_HOST=\"#{host}\"",
      *role.env_args(host),      # --env flags from config
      *role.logging_args,        # --log-opt flags
      *config.volume_args,       # --volume flags
      *role.asset_volume_args,   # Asset volume mounts
      *role.label_args,          # --label flags
      *role.option_args,         # Custom Docker options
      config.absolute_image,
      role.cmd                   # Command to run
  end

  def stop(version: nil)
    pipe \
      version ? container_id_for_version(version) : current_running_container_id,
      xargs(docker(:stop, *role.stop_args))
  end

  def current_running_container_id
    current_running_container(format: "--quiet")
  end

  private
    def current_running_container(format:)
      pipe \
        shell(chain(latest_image_container(format: format), latest_container(format: format))),
        [ :head, "-1" ]
    end
```

**Container naming convention:**
```
{service}-{role}-{destination}-{version}
Example: myapp-web-production-abc123
```

### 4. Proxy Integration

Kamal uses `kamal-proxy` for zero-downtime deploys:

```ruby
class Kamal::Commands::Proxy
  def run
    docker :run,
      "--name", container_name,
      "--network", "kamal",
      "--publish", "#{http_port}:80",
      "--publish", "#{https_port}:443",
      "--volume", "kamal-proxy-config:/home/kamal-proxy/.config/kamal-proxy",
      "--restart", "unless-stopped",
      "basecamp/kamal-proxy:#{PROXY_VERSION}"
  end

  def deploy(app_name:, target:, host:, tls:)
    # Tell proxy to route traffic to new container
    # Proxy health-checks target before switching
    proxy_cmd :deploy,
      "--app", app_name,
      "--target", target,
      "--host", host,
      *("--tls" if tls)
  end
end
```

**Proxy deployment flow:**
1. New container starts and registers with proxy
2. Proxy health-checks the new container
3. Once healthy, proxy atomically switches traffic
4. Old container is stopped after grace period

### 5. Deployment Workflow

The deploy command orchestrates the full workflow:

```ruby
# kamal deploy
namespace :deploy do
  desc "Deploy to servers"
  task :deploy do
    on_roles(all) do |host, role|
      execute *KAMAL.lock.acquire("Deploying #{config.version}")

      # 1. Build/push image
      invoke "deploy:build"

      # 2. Start new container
      execute *KAMAL.app(role: role, host: host).run(hostname: host)

      # 3. Register with proxy (for web roles)
      if role.proxy?
        execute *KAMAL.proxy(host: host).deploy(
          app_name: config.service,
          target: container_id,
          host: host,
          tls: config.tls_enabled?
        )
      end

      # 4. Remove old containers
      execute *KAMAL.app(role: role, host: host).stop
      execute *KAMAL.app(role: role, host: host).remove
    end

    # 5. Release lock
    execute *KAMAL.lock.release
  end
end
```

### 6. Health Checks

Kamal supports custom health checks:

```ruby
class Kamal::Cli::Healthcheck::Poller
  def wait_for_healthy(container_name, timeout: 30)
    start = Time.now
    while Time.now - start < timeout
      status = docker_inspect_health(container_name)
      return true if status == "healthy"
      return false if status == "unhealthy"
      sleep 1
    end
    raise TimeoutError, "Health check timed out"
  end

  def docker_inspect_health(container_name)
    # Check Docker health status
    inspect = `docker inspect --format='{{.State.Health.Status}}' #{container_name}`
    inspect.strip
  end
end
```

**Dockerfile health check:**
```dockerfile
HEALTHCHECK --interval=5s --timeout=3s \
  CMD curl -f http://localhost:3000/up || exit 1
```

### 7. Locking

Prevents concurrent deploys:

```ruby
class Kamal::Commands::Lock
  def acquire(reason)
    # Create lock file on remote server
    docker :exec,
      "kamal-proxy",
      "echo '#{reason}' > /tmp/kamal-lock"
  end

  def release
    docker :exec,
      "kamal-proxy",
      "rm -f /tmp/kamal-lock"
  end

  def check
    # Check if lock exists
    docker :exec,
      "kamal-proxy",
      "test -f /tmp/kamal-lock"
  end
end
```

### 8. Secrets Management

Integration with secrets providers:

```ruby
class Kamal::Secrets::Adapters::OnePassword
  def fetch(secret_name)
    # Fetch from 1Password
    response = `op read "op://#{vault}/#{item}/#{field}"`
    response.strip
  end
end

class Kamal::Secrets::Adapters::AWSSecretsManager
  def fetch(secret_name)
    # Fetch from AWS
    response = `aws secretsmanager get-secret-value --secret-id #{secret_name}`
    JSON.parse(response)["SecretString"]
  end
end
```

**Usage in deploy.yml:**
```yaml
env:
  secret:
    - op://my-vault/RAILS_MASTER_KEY
    - aws:my-secret-key
```

## CLI Commands

```bash
# Deploy to production
kamal deploy

# Deploy with specific version
kamal deploy -v abc123

# Deploy only to web role
kamal deploy --roles web

# Deploy to specific host
kamal deploy --hosts 192.168.0.1

# Reboot all apps (stop, remove, start fresh)
kamal app reboot

# Rollback to previous version
kamal rollback

# Check status
kamal app status

# View logs
kamal app logs

# Run command on servers
kamal app exec "bundle exec rails db:migrate"

# Deploy accessory
kamal accessory deploy redis

# Manage proxy
kamal proxy boot
kamal proxy reboot

# Build image
kamal build

# Clean up old images
kamal prune
```

## SSHKit Integration

Kamal uses SSHKit for parallel SSH execution:

```ruby
# lib/kamal/cli.rb
KAMAL = Kamal::Commander.new

# Configure SSHKit
SSHKit::Backend::Netssh.configure do |sshkit|
  sshkit.max_concurrent_starts = 10  # Limit parallel connections
  sshkit.dns_retries = 3
  sshkit.ssh_options = {
    user: config.ssh_user,
    port: config.ssh_port,
    keys: config.ssh_keys,
    forward_agent: true
  }
end

# Execute on all servers
on_roles(all) do |host, role|
  execute *command  # Runs command on remote server
end
```

## Builder Patterns

### Local Build
```ruby
class Kamal::Commands::Builder::Local
  def build
    docker :buildx, :build,
      "--platform", config.builder.arch,
      "--tag", config.absolute_image,
      "--push",
      "."
  end
end
```

### Remote Build
```ruby
class Kamal::Commands::Builder::Remote
  def build
    # SSH to remote builder
    # Build on remote server
    # Push to registry
  end
end
```

### Hybrid Build (Multi-arch)
```ruby
class Kamal::Commands::Builder::Hybrid
  def build
    # Build arm64 locally
    # Build amd64 on remote
    # Create multi-arch manifest
  end
end
```

## Production Considerations

### Scaling

- Use `--hosts` or `--roles` for targeted deploys
- Set `max_concurrent_starts` to limit SSH connections
- Deploy to subsets of servers in waves

### Security

- Secrets never stored in config files (fetched at deploy time)
- SSH key forwarding for private registry access
- TLS termination at proxy

### Monitoring

```bash
# Check container health
kamal app status

# View logs
kamal app logs --follow

# Check proxy status
kamal proxy status

# List versions
kamal app list
```

### Rollback Strategy

```bash
# Rollback to previous version
kamal rollback

# Rollback to specific version
kamal rollback -v abc123

# Manual rollback
kamal app stop
kamal app start --version abc123
```

### Cost

- Free: Kamal is MIT licensed
- Infrastructure: Pay for servers only (no Kubernetes tax)
- Efficient: SSH-based, no agents running on servers

## Related Deep Dives

- [00-zero-to-kamal-engineer.md](./00-zero-to-kamal-engineer.md) - Fundamentals
- [02-proxy-deep-dive.md](./02-proxy-deep-dive.md) - kamal-proxy internals
- [03-secrets-management-deep-dive.md](./03-secrets-management-deep-dive.md) - Secrets adapters
- [04-asset-handling-deep-dive.md](./04-asset-handling-deep-dive.md) - Asset compression/caching
- [rust-revision.md](./rust-revision.md) - Rust implementation considerations
- [production-grade.md](./production-grade.md) - Production deployment guide
