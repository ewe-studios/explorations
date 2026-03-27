---
title: "Colima Runtime Integration Deep Dive"
subtitle: "Docker, containerd, Incus, and Kubernetes provisioning and management"
based_on: "Colima - Lima-based Container Runtime"
level: "Intermediate to Advanced"
prerequisites: "[VM Management Deep Dive](01-vm-management-deep-dive.md)"
---

# Runtime Integration Deep Dive

## Table of Contents

1. [Container Runtime Interface](#1-container-runtime-interface)
2. [Docker Runtime](#2-docker-runtime)
3. [Containerd Runtime](#3-containerd-runtime)
4. [Incus Runtime](#4-incus-runtime)
5. [Kubernetes Runtime](#5-kubernetes-runtime)
6. [Runtime Switching](#6-runtime-switching)
7. [Socket Management](#7-socket-management)

---

## 1. Container Runtime Interface

### 1.1 The environment.Container Interface

All container runtimes in Colima implement a common interface:

```go
// From environment/environment.go
type Container interface {
    Dependencies      // Must declare dependencies

    // Provision sets up the runtime (config, services)
    Provision(ctx context.Context) error

    // Start starts the runtime services
    Start(ctx context.Context) error

    // Stop stops the runtime services
    Stop(ctx context.Context, force bool) error

    // Running checks if the runtime is active
    Running(ctx context.Context) bool

    // Teardown cleans up runtime configuration
    Teardown(ctx context.Context) error

    // Version returns runtime version info
    Version(ctx context.Context) string

    // Update updates the runtime (optional)
    Update(ctx context.Context) (bool, error)

    // Name returns the runtime name
    Name() string
}
```

### 1.2 Runtime Registration

Runtimes are registered at init time:

```go
// From docker/docker.go
func init() {
    environment.RegisterContainer(Name, newRuntime, false)
}

// From environment/container.go
var containers = map[string]runtimeConstructor{}

func RegisterContainer(name string, constructor runtimeConstructor, hidden bool) {
    containers[name] = constructor
}

func NewContainer(name string, host HostActions, guest GuestActions) (Container, error) {
    constructor, ok := containers[name]
    if !ok {
        return nil, fmt.Errorf("unknown runtime '%s'", name)
    }
    return constructor(host, guest), nil
}
```

### 1.3 Runtime Startup Sequence

```
┌─────────────────────────────────────────────────────────┐
│              Colima Start Sequence                      │
├─────────────────────────────────────────────────────────┤
│  1. Initialize App                                      │
│  2. Create container environments list                  │
│  3. Start VM (Lima)                                     │
│  4. For each runtime:                                   │
│     a. Provision (config, services)                     │
│     b. Start (launch daemons)                           │
│  5. Run ready provision scripts                         │
│  6. Persist runtime settings                            │
└─────────────────────────────────────────────────────────┘
```

```go
// From app/app.go
func (c colimaApp) startWithRuntime(conf config.Config) ([]environment.Container, error) {
    kubernetesEnabled := conf.Kubernetes.Enabled

    // Kubernetes can only be enabled for docker and containerd
    switch conf.Runtime {
    case docker.Name, containerd.Name:
    default:
        kubernetesEnabled = false
    }

    var containers []environment.Container

    // Primary runtime
    {
        env, err := c.containerEnvironment(conf.Runtime)
        if err != nil {
            return nil, err
        }
        containers = append(containers, env)
    }

    // Kubernetes (after primary runtime)
    if kubernetesEnabled {
        env, err := c.containerEnvironment(kubernetes.Name)
        if err != nil {
            return nil, err
        }
        containers = append(containers, env)
    }

    return containers, nil
}
```

---

## 2. Docker Runtime

### 2.1 Docker Architecture in Colima

```
┌─────────────────────────────────────────────────────────┐
│                    macOS Host                           │
│  ┌─────────────┐                                        │
│  │ Docker CLI  │                                        │
│  └──────┬──────┘                                        │
│         │ socket forward                                 │
│         v                                                │
│  ┌──────────────────────────────────────────────────┐   │
│  │              Lima VM (Linux)                     │   │
│  │  ┌──────────────────────────────────────────┐    │   │
│  │  │           Docker Engine                  │    │   │
│  │  │  ┌──────────┐  ┌──────────┐  ┌────────┐ │    │   │
│  │  │  │ containerd│  │ runc     │  │ images │ │    │   │
│  │  │  └──────────┘  └──────────┘  └────────┘ │    │   │
│  │  └──────────────────────────────────────────┘    │   │
│  └──────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────┘
```

### 2.2 Docker Provisioning

```go
// From docker/docker.go
func (d dockerRuntime) Provision(ctx context.Context) error {
    a := d.Init(ctx)
    log := d.Logger(ctx)
    conf, _ := ctx.Value(config.CtxKey()).(config.Config)

    // 1. Provision containerd (Docker uses containerd internally)
    a.Add(func() error {
        return d.provisionContainerd(ctx)
    })

    // 2. Create daemon.json configuration
    a.Add(func() error {
        if err := d.createDaemonFile(conf.Docker, conf.Env); err != nil {
            log.Warnln(err)
        }
        if err := d.addHostGateway(conf.Docker); err != nil {
            log.Warnln(err)
        }
        if err := d.reloadAndRestartSystemdService(); err != nil {
            log.Warnln(err)
        }
        return nil
    })

    // 3. Setup Docker context for host client
    a.Add(d.setupContext)
    if conf.AutoActivate() {
        a.Add(d.useContext)
    }

    return a.Exec()
}
```

### 2.3 Docker daemon.json

```go
// From docker/daemon.go
func (d dockerRuntime) createDaemonFile(conf map[string]any, env map[string]string) error {
    // Default config
    m := map[string]any{
        "feature": map[string]any{
            "containerd-snapshotter": true,  // Use containerd image store
        },
        "exec-opts": []any{"native.cgroupdriver=cgroupfs"},
        "log-driver": "json-file",
        "log-opts": map[string]any{
            "max-size": "10m",
            "max-file": "3",
        },
    }

    // Merge user-provided config
    for k, v := range conf {
        m[k] = v
    }

    // Add environment variables
    if len(env) > 0 {
        m["env"] = append(m["env"].([]string), envSlice...)
    }

    // Write to /etc/docker/daemon.json
    buf := new(bytes.Buffer)
    json.NewEncoder(buf).Encode(m)
    return d.guest.Write("/etc/docker/daemon.json", buf.Bytes())
}
```

### 2.4 Docker Context Setup

```go
// From docker/context.go
func (d dockerRuntime) setupContext() error {
    name := config.CurrentProfile().ID
    socket := "unix://" + HostSocketFile()

    // Create context if not exists
    if !d.hasContext(name) {
        if err := d.host.RunQuiet("docker", "context", "create", name,
            "--description", name,
            "--docker", "host="+socket); err != nil {
            return err
        }
    }

    return nil
}

func (d dockerRuntime) useContext() error {
    return d.host.RunQuiet("docker", "context", "use", config.CurrentProfile().ID)
}

func (d dockerRuntime) teardownContext() error {
    if d.isDefaultRemote() {
        d.host.RunQuiet("docker", "context", "use", "default")
    }
    d.host.RunQuiet("docker", "context", "rm", "--force", config.CurrentProfile().ID)
    return nil
}
```

### 2.5 Docker Start

```go
// From docker/docker.go
func (d dockerRuntime) Start(ctx context.Context) error {
    a := d.Init(ctx)

    // Start docker.service with retry
    a.Retry("", time.Second, 60, func(int) error {
        return d.systemctl.Start("docker.service")
    })

    // Verify docker is responsive
    a.Retry("", time.Second, 60, func(int) error {
        return d.guest.RunQuiet("sudo", "docker", "info")
    })

    // Ensure docker is accessible without root
    a.Add(func() error {
        if err := d.guest.RunQuiet("docker", "info"); err == nil {
            return nil
        }
        // Restart to add user to docker group
        ctx := context.WithValue(ctx, cli.CtxKeyQuiet, true)
        return d.guest.Restart(ctx)
    })

    return a.Exec()
}
```

### 2.6 Docker Data Disk

```go
// From docker/docker.go
func DataDisk() environment.DataDisk {
    return environment.DataDisk{
        Dirs: []environment.DiskDir{
            {Name: "docker", Path: "/var/lib/docker"},      // Images, containers
            {Name: "containerd", Path: "/var/lib/containerd"}, // containerd data
            {Name: "rancher", Path: "/var/lib/rancher"},     // K3s data
            {Name: "cni", Path: "/var/lib/cni"},             // Network plugins
            {Name: "ramalama", Path: "/var/lib/ramalama"},   // AI models
        },
        FSType: "ext4",
        PreMount: []string{
            "systemctl stop docker.service",
            "systemctl stop containerd.service",
        },
    }
}
```

---

## 3. Containerd Runtime

### 3.1 Containerd vs Docker

| Aspect | Docker | containerd |
|--------|--------|------------|
| **Level** | High-level (CLI, API, daemon) | Low-level (runtime only) |
| **Client** | `docker` CLI | `nerdctl` CLI |
| **Image Store** | Docker format | OCI format |
| **Orchestration** | Docker Compose, Swarm | Kubernetes, Nomad |
| **Overhead** | Higher | Lower |
| **Use Case** | Development, single-node | Production, orchestration |

### 3.2 Containerd Provisioning

```go
// From containerd/containerd.go
func (c containerdRuntime) Provision(ctx context.Context) error {
    a := c.Init(ctx)

    // 1. Write containerd config
    a.Add(func() error {
        profilePath := filepath.Join(configDir(), "containerd", "config.toml")
        centralPath := filepath.Join(userConfigDir(), "containerd", "config.toml")
        return c.provisionConfig(profilePath, centralPath,
            "/etc/containerd/config.toml", containerdConf)
    })

    // 2. Write buildkit config
    a.Add(func() error {
        profilePath := filepath.Join(configDir(), "containerd", "buildkitd.toml")
        centralPath := filepath.Join(userConfigDir(), "buildkit", "buildkitd.toml")
        return c.provisionConfig(profilePath, centralPath,
            "/etc/buildkit/buildkitd.toml", buildKitConf)
    })

    return a.Exec()
}

// Config hierarchy: per-profile > central > embedded default
func (c containerdRuntime) provisionConfig(profilePath, centralPath, guestPath string, defaultConf []byte) error {
    // 1. Per-profile override (highest priority)
    if data, err := os.ReadFile(profilePath); err == nil {
        return c.guest.Write(guestPath, data)
    }

    // 2. Central config
    if data, err := os.ReadFile(centralPath); err == nil {
        return c.guest.Write(guestPath, data)
    }

    // 3. Default config (write to central for discoverability)
    os.MkdirAll(filepath.Dir(centralPath), 0755)
    os.WriteFile(centralPath, defaultConf, 0644)
    return c.guest.Write(guestPath, defaultConf)
}
```

### 3.3 containerd Config

```toml
# Embedded default containerd config
# From containerd/config.toml
version = 2

[plugins."io.containerd.runtime.v1.linux"]
  shim_debug = true

[plugins."io.containerd.grpc.v1.cri"]
  sandbox_image = "registry.k8s.io/pause:3.9"

  [plugins."io.containerd.grpc.v1.cri".containerd]
    snapshotter = "overlayfs"
    default_runtime_name = "runc"

    [plugins."io.containerd.grpc.v1.cri".containerd.runtimes.runc]
      runtime_type = "io.containerd.runc.v2"

      [plugins."io.containerd.grpc.v1.cri".containerd.runtimes.runc.options]
        SystemdCgroup = true
```

### 3.4 Containerd Start

```go
// From containerd/containerd.go
func (c containerdRuntime) Start(ctx context.Context) error {
    a := c.Init(ctx)

    // Start containerd.service
    a.Add(func() error {
        return c.systemctl.Restart("containerd.service")
    })

    // Verify containerd is responsive
    a.Retry("", time.Second*5, 10, func(int) error {
        return c.guest.RunQuiet("sudo", "nerdctl", "info")
    })

    // Start buildkit (for builds)
    a.Add(func() error {
        return c.systemctl.Start("buildkit.service")
    })

    return a.Exec()
}
```

### 3.5 nerdctl Integration

```go
// From cmd/nerdctl.go
var nerdctlCmd = &cobra.Command{
    Use:   "nerdctl",
    Short: "run nerdctl",
    Long:  "Run nerdctl with the current Colima instance.",
    RunE: func(cmd *cobra.Command, args []string) error {
        return newApp().SSH(append([]string{"sudo", "nerdctl"}, args...)...)
    },
}

// Install nerdctl alias
var nerdctlInstallCmd = &cobra.Command{
    Use:   "install",
    Short: "install nerdctl alias",
    RunE: func(cmd *cobra.Command, args []string) error {
        script := `#!/bin/sh
sudo nerdctl "$@"`
        return osutil.WriteToPath(script, 0755, "nerdctl")
    },
}
```

### 3.6 Containerd Data Disk

```go
// From containerd/containerd.go
func DataDisk() environment.DataDisk {
    return environment.DataDisk{
        Dirs: []environment.DiskDir{
            {Name: "containerd", Path: "/var/lib/containerd"},
            {Name: "buildkit", Path: "/var/lib/buildkit"},
            {Name: "nerdctl", Path: "/var/lib/nerdctl"},
            {Name: "rancher", Path: "/var/lib/rancher"},
            {Name: "cni", Path: "/var/lib/cni"},
        },
        FSType: "ext4",
        PreMount: []string{
            "systemctl stop containerd.service",
            "systemctl stop buildkit.service",
        },
    }
}
```

---

## 4. Incus Runtime

### 4.1 What is Incus?

**Incus** is a powerful system container and virtual machine manager forked from LXD. It supports:

- **System containers** (lightweight, shared kernel)
- **Virtual machines** (full isolation)
- **Storage pools** (zfs, lvm, ceph, etc.)
- **Network management** (bridges, networks)
- **Image management** (OCI, distro images)

### 4.2 Incus Architecture

```
┌─────────────────────────────────────────────────────────┐
│                    macOS Host                           │
│  ┌─────────────┐                                        │
│  │  Incus CLI  │                                        │
│  └──────┬──────┘                                        │
│         │ socket forward                                 │
│         v                                                │
│  ┌──────────────────────────────────────────────────┐   │
│  │              Lima VM (Linux)                     │   │
│  │  ┌──────────────────────────────────────────┐    │   │
│  │  │           Incus Daemon                   │    │   │
│  │  │  ┌──────────┐  ┌──────────┐  ┌────────┐ │    │   │
│  │  │  │ ZFS Pool │  │ Networks │  │ Images │ │    │   │
│  │  │  └──────────┘  └──────────┘  └────────┘ │    │   │
│  │  └──────────────────────────────────────────┘    │   │
│  └──────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────┘
```

### 4.3 Incus Provisioning

```go
// From incus/incus.go
func (c incusRuntime) Provision(ctx context.Context) error {
    conf := ctx.Value(config.CtxKey()).(config.Config)

    // Start incus to check if already provisioned
    _ = c.systemctl.Start("incus.service")

    // Check if already provisioned
    if found, _, _ := c.findNetwork(incusBridgeInterface); found {
        return nil  // Already provisioned
    }

    emptyDisk := true
    recoverStorage := false

    if limautil.DiskProvisioned(Name) {
        emptyDisk = false
        recoverStorage = cli.Prompt("existing Incus data found, recover storage pool(s)")
    }

    // Prepare preseed config
    var value struct {
        Disk          int
        Interface     string
        BridgeGateway string
        SetStorage    bool
    }
    value.Disk = conf.Disk
    value.Interface = incusBridgeInterface
    value.BridgeGateway = bridgeGateway
    value.SetStorage = emptyDisk

    buf, err := util.ParseTemplate(configYaml, value)
    if err != nil {
        return fmt.Errorf("error parsing incus config template: %w", err)
    }

    // Apply preseed config
    stdin := bytes.NewReader(buf)
    if err := c.guest.RunWith(stdin, nil, "sudo", "incus", "admin", "init", "--preseed"); err != nil {
        return fmt.Errorf("error setting up incus: %w", err)
    }

    // Handle existing disk
    if !emptyDisk {
        if !recoverStorage {
            return c.wipeDisk(conf.Disk)
        }
        if err := c.recoverDisk(ctx); err != nil {
            return c.wipeDisk(conf.Disk)
        }
    }

    return nil
}
```

### 4.4 Incus Preseed Config

```yaml
# From incus/config.yaml
config:
  core.https_address: '{{ .Interface }}:8443'

networks:
  - config:
      ipv4.address: {{ .BridgeGateway }}/24
      ipv4.nat: "true"
      ipv6.address: auto
    description: ""
    name: {{ .Interface }}
    type: ""
    project: default

storage_pools:
  - config:
      size: {{ .Disk }}GiB
    description: ""
    name: default
    driver: zfs

profiles:
  - config: {}
    description: ""
    devices:
      eth0:
        name: eth0
        network: {{ .Interface }}
        type: nic
      root:
        path: /
        pool: default
        type: disk
    name: default

projects: []
```

### 4.5 Incus Remote Setup

```go
// From incus/incus.go
func (c incusRuntime) setRemote(activate bool) error {
    name := config.CurrentProfile().ID

    // Add remote
    if !c.hasRemote(name) {
        if err := c.host.RunQuiet("incus", "remote", "add", name,
            "unix://"+HostSocketFile()); err != nil {
            return err
        }
    }

    // Set as default if requested
    if activate {
        return c.host.RunQuiet("incus", "remote", "switch", name)
    }

    return nil
}

func (c incusRuntime) unsetRemote() error {
    // Reset to local if this was default
    if c.isDefaultRemote() {
        c.host.RunQuiet("incus", "remote", "switch", "local")
    }

    // Remove remote
    if c.hasRemote(config.CurrentProfile().ID) {
        c.host.RunQuiet("incus", "remote", "remove", config.CurrentProfile().ID)
    }

    return nil
}
```

### 4.6 Incus Storage Recovery

```go
// From incus/incus.go
func (c *incusRuntime) recoverDisk(ctx context.Context) error {
    var disks []string
    str, err := c.guest.RunOutput("sh", "-c",
        "sudo ls "+poolDisksDir+" | grep '.img$'")

    if err != nil {
        return fmt.Errorf("cannot list storage pool disks: %w", err)
    }

    disks = strings.Fields(str)
    if len(disks) == 0 {
        return fmt.Errorf("no existing storage pool disks found")
    }

    log := c.Logger(ctx)
    log.Println("Running 'incus admin recover' ...")
    log.Println(fmt.Sprintf("Found %d storage pool source(s):", len(disks)))

    // Interactive recovery
    if err := c.guest.RunInteractive("sudo", "incus", "admin", "recover"); err != nil {
        return fmt.Errorf("error recovering storage pool: %w", err)
    }

    // Verify recovery succeeded
    out, err := c.guest.RunOutput("sudo", "incus", "storage", "list",
        "name="+poolName, "-c", "n", "--format", "compact,noheader")
    if out != poolName {
        return fmt.Errorf("default storage pool recovery failure")
    }

    return nil
}
```

### 4.7 Docker Remote Integration

```go
// From incus/incus.go
func (c incusRuntime) addDockerRemote() error {
    if c.hasRemote("docker") {
        return nil  // Already added
    }
    return c.host.RunQuiet("incus", "remote", "add", "docker",
        "https://docker.io", "--protocol=oci")
}
```

This allows pulling OCI images from Docker Hub:
```bash
incus launch docker:alpine mycontainer
```

---

## 5. Kubernetes Runtime

### 5.1 K3s Integration

Colima uses k3s (lightweight Kubernetes) for Kubernetes support:

```go
// From kubernetes/kubernetes.go
const (
    Name = "kubernetes"
    k3s  = "k3s"
)

func (k kubernetesRuntime) Provision(ctx context.Context) error {
    a := k.Init(ctx)
    conf, _ := ctx.Value(config.CtxKey()).(config.Config)

    // Install k3s
    a.Add(func() error {
        return k.installK3s(conf.Kubernetes.Version)
    })

    // Configure k3s with provided args
    a.Add(func() error {
        args := conf.Kubernetes.K3sArgs
        return k.configureK3s(args)
    })

    return a.Exec()
}
```

### 5.2 Kubernetes Start

```go
// From kubernetes/kubernetes.go
func (k kubernetesRuntime) Start(ctx context.Context) error {
    a := k.Init(ctx)
    conf, _ := ctx.Value(config.CtxKey()).(config.Config)

    // Start k3s service
    a.Add(func() error {
        return k.systemctl.Start("k3s.service")
    })

    // Wait for k3s to be ready
    a.Retry("", time.Second*5, 30, func(int) error {
        return k.guest.RunQuiet("sudo", "k3s", "kubectl", "cluster-info")
    })

    // Setup kubectl context on host
    a.Add(func() error {
        return k.setupKubectl()
    })

    return a.Exec()
}
```

### 5.3 Kubernetes Config

```go
// From config/config.go
type Kubernetes struct {
    Enabled bool     `yaml:"enabled"`
    Version string   `yaml:"version"`
    K3sArgs []string `yaml:"k3sArgs"`  // Additional k3s args
    Port    int      `yaml:"port,omitempty"`  // Listen port
}

// Default k3s args
var defaultK3sArgs = []string{"--disable=traefik"}

// Usage:
// colima start --kubernetes --k3s-arg="--disable=servicelb,local-storage"
```

---

## 6. Runtime Switching

### 6.1 Runtime Persistence

```go
// From app/app.go
func (c colimaApp) setRuntime(runtime string) error {
    err := store.Set(func(s *store.Store) {
        if s.DiskFormatted {
            s.DiskRuntime = runtime
        }
    })

    if err != nil {
        log.Traceln(fmt.Errorf("error persisting store: %w", err))
    }

    return c.guest.Set(environment.ContainerRuntimeKey, runtime)
}

func (c colimaApp) currentRuntime(ctx context.Context) (string, error) {
    if !c.guest.Running(ctx) {
        return "", fmt.Errorf("%s is not running", config.CurrentProfile().DisplayName)
    }

    r := c.guest.Get(environment.ContainerRuntimeKey)
    if r == "" {
        return "", fmt.Errorf("error retrieving current runtime: empty value")
    }

    return r, nil
}
```

### 6.2 Runtime Switching Process

To switch runtimes:

```bash
# Stop current runtime
colima stop

# Start with different runtime
colima start --runtime containerd

# Or edit config
colima start --edit
# Change runtime: docker -> containerd
```

**Note:** Switching runtimes reuses the VM but provisions new container runtime.

### 6.3 Runtime Detection

```go
// From app/app.go
func (c colimaApp) currentContainerEnvironments(ctx context.Context) ([]environment.Container, error) {
    var containers []environment.Container

    // Get primary runtime
    runtime, err := c.currentRuntime(ctx)
    if err != nil {
        return nil, err
    }

    if environment.IsNoneRuntime(runtime) {
        return nil, nil
    }

    env, err := c.containerEnvironment(runtime)
    containers = append(containers, env)

    // Detect and add kubernetes if running
    if k, err := c.containerEnvironment(kubernetes.Name); err == nil && k.Running(ctx) {
        containers = append(containers, k)
    }

    return containers, nil
}
```

---

## 7. Socket Management

### 7.1 Socket Forwarding

Colima forwards Unix sockets from VM to host:

```go
// Docker socket
GuestSocket: "/var/run/docker.sock"
HostSocket:  ~/.colima/default/docker.sock

// Containerd socket
GuestSocket: "/run/containerd/containerd.sock"
HostSocket:  ~/.colima/default/containerd/containerd.sock

// Incus socket
GuestSocket: "/var/lib/incus/unix.socket"
HostSocket:  ~/.colima/default/incus.sock

// Kubernetes config
GuestSocket: "/etc/rancher/k3s/k3s.yaml"
HostSocket:  ~/.colima/default/kubeconfig
```

### 7.2 Socket Files

```go
// From docker/docker.go
func HostSocketFile() string {
    return filepath.Join(config.CurrentProfile().ConfigDir(), "docker.sock")
}

// From containerd/containerd.go
func HostSocketFiles() (files struct {
    Containerd string
    Buildkitd  string
}) {
    files.Containerd = filepath.Join(configDir(), "containerd.sock")
    files.Buildkitd = filepath.Join(configDir(), "buildkitd.sock")
    return files
}

// From incus/incus.go
func HostSocketFile() string {
    return filepath.Join(configDir(), "incus.sock")
}
```

### 7.3 Docker Context Auto-Switch

When starting Colima with Docker runtime:

```go
// From docker/context.go
func (d dockerRuntime) useContext() error {
    // Set colima profile as default Docker context
    return d.host.RunQuiet("docker", "context", "use", config.CurrentProfile().ID)
}

// This makes docker CLI commands use Colima automatically
```

---

## Summary

| Runtime | Use Case | Key Features |
|---------|----------|--------------|
| **Docker** | Development, single-node | Full Docker CLI, compose, mature ecosystem |
| **containerd** | Production, Kubernetes | Lower overhead, OCI-native, nerdctl |
| **Incus** | System containers/VMs | Full LXD fork, storage pools, network management |
| **Kubernetes** | Orchestration | k3s, multi-node simulation, Helm charts |

---

*Next: [Volume Mounting Deep Dive](03-volume-mounting-deep-dive.md)*
