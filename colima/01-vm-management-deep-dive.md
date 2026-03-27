---
title: "Colima VM Management Deep Dive"
subtitle: "Lima integration, VM lifecycle, disk management, and host communication"
based_on: "Colima - Lima-based Container Runtime"
level: "Intermediate to Advanced"
prerequisites: "[Zero to Container Engineer](00-zero-to-container-engineer.md)"
---

# VM Management Deep Dive

## Table of Contents

1. [Lima Integration Architecture](#1-lima-integration-architecture)
2. [VM Lifecycle Management](#2-vm-lifecycle-management)
3. [VM Backends: QEMU vs vz vs krunkit](#3-vm-backends-qemu-vs-vz-vs-krunkit)
4. [Disk Management](#4-disk-management)
5. [SSH and Host Communication](#5-ssh-and-host-communication)
6. [Provision Scripts](#6-provision-scripts)
7. [Multi-Instance Support](#7-multi-instance-support)

---

## 1. Lima Integration Architecture

### 1.1 What is Lima?

**Lima** (Linux Machines) is a project that launches Linux VMs with automatic file sharing and port forwarding. Colima uses Lima as its VM backend.

```
┌─────────────────────────────────────────────────────────┐
│                    Colima                               │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐     │
│  │   App       │  │   Config    │  │  Daemon     │     │
│  │   Layer     │  │   Manager   │  │  Manager    │     │
│  └─────────────┘  └─────────────┘  └─────────────┘     │
│                        │                                │
│  ┌─────────────────────┴─────────────────────────┐     │
│  │           Lima Integration Layer              │     │
│  │  ┌─────────┐  ┌─────────┐  ┌─────────────┐   │     │
│  │  │  lima   │  │ limactl │  │  Lima       │   │     │
│  │  │  VM     │  │  CLI    │  │  Config     │   │     │
│  │  └─────────┘  └─────────┘  └─────────────┘   │     │
│  └───────────────────────────────────────────────┘     │
└─────────────────────────────────────────────────────────┘
                        │
                        v
┌─────────────────────────────────────────────────────────┐
│              Lima (github.com/lima-vm/lima)             │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐     │
│  │  QEMU       │  │  vzt        │  │  WSL2       │     │
│  │  Backend    │  │  Backend    │  │  Backend    │     │
│  └─────────────┘  └─────────────┘  └─────────────┘     │
└─────────────────────────────────────────────────────────┘
```

### 1.2 Lima Config Structure

Colima generates Lima configs dynamically:

```go
// From limaconfig/config.go
type Config struct {
    VMType               VMType            // qemu, vz, krunkit
    VMOpts               VMOpts            // VM-specific options
    Arch                 Arch              // aarch64, x86_64
    Images               []File            // Disk images
    CPUs                 *int              // Number of CPUs
    Memory               string            // Memory size (e.g., "4GiB")
    Disk                 string            // Disk size (e.g., "100GiB")
    AdditionalDisks      []Disk            // Extra disks
    Mounts               []Mount           // Volume mounts
    MountType            MountType         // 9p, virtiofs, sshfs
    SSH                  SSH               // SSH configuration
    PortForwards         []PortForward     // Port forwarding rules
    Networks             []Network         // Network configurations
    Provision            []Provision       // Setup scripts
}
```

### 1.3 Colima to Lima Translation

Colima's `newConf()` function translates Colima config to Lima config:

```go
// From lima/yaml.go
func newConf(ctx context.Context, conf config.Config) (l limaconfig.Config, err error) {
    // Architecture
    l.Arch = environment.Arch(conf.Arch).Arch()

    // VM type (qemu, vz, krunkit)
    if conf.VMType == "vz" {
        l.VMType = limaconfig.VZ
    } else if conf.VMType == "krunkit" {
        l.VMType = limaconfig.Krunkit
    } else {
        l.VMType = limaconfig.QEMU
    }

    // Resources
    l.CPUs = &conf.CPU
    l.Memory = fmt.Sprintf("%dMiB", int(conf.Memory*1024))
    l.Disk = conf.Disk

    // Disk image
    l.Images, err = diskImages(ctx, conf)

    // Mounts
    for _, m := range conf.MountsOrDefault() {
        l.Mounts = append(l.Mounts, limaconfig.Mount{
            Location: m.Location,
            Writable: m.Writable,
        })
    }

    // Port forwarding (automatic for container ports)
    l.PortForwards = append(l.PortForwards,
        limaconfig.PortForward{
            GuestSocket: "/var/run/docker.sock",
            HostSocket:  docker.HostSocketFile(),
        },
        // ... more forwards
    )

    return l, nil
}
```

---

## 2. VM Lifecycle Management

### 2.1 VM States

```
                    ┌─────────────┐
                    │   Created   │
                    │ (config only)│
                    └──────┬──────┘
                           │ start
                           v
┌─────────────┐      ┌─────────────┐
│   Stopped   │<────>│  Starting   │
│ (shutdown)  │ stop │ (booting)   │
└──────┬──────┘      └──────┬──────┘
       │                    │
       │ restart            │ ready
       │                    v
       │             ┌─────────────┐
       └────────────>│   Running   │
                     │ (operational)│
                     └──────┬──────┘
                            │
                            │ delete
                            v
                     ┌─────────────┐
                     │  Deleted    │
                     │ (removed)   │
                     └─────────────┘
```

### 2.2 Start Flow

```go
// From app/app.go - Start method
func (c colimaApp) Start(conf config.Config) error {
    ctx := context.WithValue(context.Background(), config.CtxKey(), conf)

    // 1. Initialize container runtimes
    var containers []environment.Container
    if !environment.IsNoneRuntime(conf.Runtime) {
        cs, err := c.startWithRuntime(conf)
        if err != nil { return err }
        containers = cs
    }

    // 2. Start VM (Lima)
    // Order: VM start -> container provision -> container start
    if err := c.guest.Start(ctx, conf); err != nil {
        return fmt.Errorf("error starting vm: %w", err)
    }

    // 3. Run after-boot provision scripts
    c.runProvisionScripts(conf, config.ProvisionModeAfterBoot)

    // 4. Provision and start container runtimes
    for _, cont := range containers {
        log.Println("provisioning ...")
        if err := cont.Provision(ctx); err != nil {
            return fmt.Errorf("error provisioning %s: %w", cont.Name(), err)
        }
        log.Println("starting ...")
        if err := cont.Start(ctx); err != nil {
            return fmt.Errorf("error starting %s: %w", cont.Name(), err)
        }
    }

    // 5. Run ready provision scripts
    c.runProvisionScripts(conf, config.ProvisionModeReady)

    // 6. Persist runtime settings
    c.setRuntime(conf.Runtime)
    c.setKubernetes(conf.Kubernetes)

    return nil
}
```

### 2.3 Resume vs Fresh Start

Colima distinguishes between resuming an existing VM and creating a new one:

```go
// From lima/lima.go
func (l *limaVM) Start(ctx context.Context, conf config.Config) error {
    l.prepareHost(conf)

    if l.Created() {
        // Resume existing VM
        return l.resume(ctx, conf)
    }

    // Fresh VM creation
    a.Add(func() (err error) {
        ctx, err = l.startDaemon(ctx, conf)
        return err
    })

    a.Add(func() (err error) {
        l.limaConf, err = newConf(ctx, conf)
        return err
    })

    a.Add(func() error {
        return l.createRuntimeDisk(conf)
    })

    a.Add(func() error {
        return l.downloadDiskImage(ctx, conf)
    })

    a.Add(func() error {
        return yamlutil.WriteYAML(l.limaConf, confFile)
    })

    a.Add(func() error {
        return l.host.Run(limactl, "start", "--tty=false", confFile)
    })

    return a.Exec()
}
```

### 2.4 Stop Flow

```go
// From app/app.go - Stop method
func (c colimaApp) Stop(force bool) error {
    ctx := context.Background()

    // Order: container stop -> VM stop

    // 1. Stop container runtimes
    if c.guest.Running(ctx) {
        containers, err := c.currentContainerEnvironments(ctx)
        if err != nil {
            log.Warnln(fmt.Errorf("error retrieving runtimes: %w", err))
        }

        // Stop in reverse order of start
        for i := len(containers) - 1; i >= 0; i-- {
            cont := containers[i]
            log.Println("stopping ...")
            if err := cont.Stop(ctx, force); err != nil {
                log.Warnln(fmt.Errorf("error stopping %s: %w", cont.Name(), err))
            }
        }
    }

    // 2. Stop VM
    if err := c.guest.Stop(ctx, force); err != nil {
        return fmt.Errorf("error stopping vm: %w", err)
    }

    return nil
}
```

### 2.5 Restart Flow

```go
// From lima/lima.go
func (l limaVM) Restart(ctx context.Context) error {
    if l.conf.Empty() {
        return fmt.Errorf("cannot restart, VM not previously started")
    }

    // Stop with existing config
    if err := l.Stop(ctx, false); err != nil {
        return err
    }

    // Minor delay to prevent race conditions
    time.Sleep(time.Second * 2)

    // Start with same config
    if err := l.Start(ctx, l.conf); err != nil {
        return err
    }

    return nil
}
```

---

## 3. VM Backends: QEMU vs vz vs krunkit

### 3.1 Backend Comparison

| Feature | QEMU | vz (Virtualization.Framework) | krunkit |
|---------|------|-------------------------------|---------|
| **Type** | Full emulator | Apple hypervisor | GPU-accelerated hypervisor |
| **Architecture** | aarch64, x86_64 | aarch64 only | aarch64 only |
| **Performance** | Good | Excellent | Excellent + GPU |
| **Requirements** | `qemu` package | macOS 13+ | macOS 13+, M3+ |
| **Mount Type** | sshfs, 9p, virtiofs | virtiofs only | virtiofs only |
| **Nested Virtualization** | Yes | Limited | Yes |
| **GPU Passthrough** | No | No | Yes |
| **Rosetta Support** | No | Yes | Yes |

### 3.2 QEMU Backend

**When to use:** Maximum compatibility, cross-architecture emulation

```go
// QEMU-specific configuration
l.VMType = limaconfig.QEMU
l.VMOpts.QEMU = limaconfig.QEMUOpts{
    MinimumVersion: proto.String("7.0.0"),
    CPUType: map[limaconfig.Arch]string{
        "aarch64": "cortex-a72",
        "x86_64":  "qemu64",
    },
}

// Mount type defaults to sshfs for QEMU
if conf.MountType == "" {
    conf.MountType = "sshfs"
}
```

**QEMU advantages:**
- Works on Intel and Apple Silicon
- Supports foreign architecture emulation (x86_64 on M1)
- Mature, well-tested
- Flexible configuration

**QEMU disadvantages:**
- Higher CPU overhead than vz
- Requires homebrew installation
- Slower disk I/O

### 3.3 vz Backend

**When to use:** Best performance on Apple Silicon macOS 13+

```go
// vz-specific configuration
l.VMType = limaconfig.VZ
l.VMOpts.VZOpts = limaconfig.VZOpts{
    Rosetta: limaconfig.Rosetta{
        Enabled: conf.VZRosetta,  // Enable AMD64 emulation
        BinFmt:  true,
    },
}

// Mount type defaults to virtiofs for vz
if conf.MountType == "" {
    conf.MountType = "virtiofs"
}
```

**vz advantages:**
- Native Apple hypervisor
- Lowest overhead
- Rosetta 2 support for x86_64 containers
- No external dependencies

**vz disadvantages:**
- macOS 13+ only
- Apple Silicon only
- Less flexible than QEMU

### 3.4 krunkit Backend

**When to use:** GPU-accelerated AI/ML workloads

```go
// krunkit-specific configuration
l.VMType = limaconfig.Krunkit
l.VMOpts.VZOpts = limaconfig.VZOpts{
    Rosetta: limaconfig.Rosetta{
        Enabled: conf.VZRosetta,
        BinFmt:  true,
    },
}

// GPU acceleration for AI models
if conf.ModelRunner != "" {
    // docker or ramalama model runner
    log.Println("AI model runner:", conf.ModelRunner)
}
```

**krunkit advantages:**
- GPU passthrough for ML workloads
- All vz benefits
- Optimized for AI inference

**krunkit disadvantages:**
- Requires M3 or newer
- macOS 13+ only
- Additional `krunkit` dependency

### 3.5 Backend Selection Logic

```go
// From start.go - setFlagDefaults
func setFlagDefaults(cmd *cobra.Command) {
    if startCmdArgs.VMType == "" {
        startCmdArgs.VMType = defaultVMType
    }

    if util.MacOS13OrNewer() {
        // Changing to vz implies changing mount type to virtiofs
        if cmd.Flag("vm-type").Changed && startCmdArgs.VMType == "vz" {
            if !cmd.Flag("mount-type").Changed {
                startCmdArgs.MountType = "virtiofs"
            }
        }
    }

    // Convert mount type for qemu
    if startCmdArgs.VMType != "vz" && startCmdArgs.MountType == "virtiofs" {
        startCmdArgs.MountType = "sshfs"
    }
}
```

---

## 4. Disk Management

### 4.1 Disk Structure

```
~/.colima/default/
├── basedisk           # Base OS image (read-only)
├── diffdisk           # Writable disk (user data, containers)
├── cidata.iso         # Cloud-init configuration
└── colima.yaml        # Colima configuration
```

### 4.2 Disk Creation Flow

```go
// From lima/disk.go
func (l *limaVM) createRuntimeDisk(conf config.Config) error {
    disk := config.CurrentProfile().DiffDiskPath()

    // Create sparse disk file
    cmd := exec.Command("qemu-img", "create", "-f", "qcow2", disk, conf.Disk.GiB())
    return cmd.Run()
}

// Format disk on first boot
func (l *limaVM) useRuntimeDisk(conf config.Config) {
    // The disk is formatted by the Lima provisioning scripts
    // based on the AdditionalDisks configuration
    l.limaConf.AdditionalDisks = []limaconfig.Disk{
        {
            Name:   config.CurrentProfile().ID,
            Format: true,
            FSType: "ext4",
        },
    }
}
```

### 4.3 Disk Resize

Disk can be expanded after creation:

```go
// From lima/disk.go
func (l *limaVM) syncDiskSize(ctx context.Context, conf config.Config) config.Config {
    inst, err := limautil.Instance()
    if err != nil {
        return conf
    }

    // If configured disk is larger, resize
    if conf.Disk > inst.Disk {
        log.Printf("resizing disk from %dGiB to %dGiB", inst.Disk, conf.Disk)
        if err := limautil.ResizeDisk(conf.Disk); err != nil {
            log.Warnln(fmt.Errorf("error resizing disk: %w", err))
        }
    }

    return conf
}
```

### 4.4 Data Persistence

Colima stores persistent data in the diffdisk:

```go
// From store/store.go
type Store struct {
    DiskFormatted     bool   `json:"disk_formatted"`
    DiskRuntime       string `json:"disk_runtime"`  // docker, containerd, incus
    RamalamaProvisioned bool `json:"ramalama_provisioned"`
}

// Data directories on the disk
var diskDirs = []environment.DiskDir{
    {Name: "docker", Path: "/var/lib/docker"},      // Docker images/containers
    {Name: "containerd", Path: "/var/lib/containerd"}, // Containerd data
    {Name: "rancher", Path: "/var/lib/rancher"},     // K3s data
    {Name: "cni", Path: "/var/lib/cni"},             // CNI plugins
    {Name: "ramalama", Path: "/var/lib/ramalama"},   // AI models
}
```

### 4.5 Disk Deletion

```bash
# Delete instance with data
colima delete --data

# Manual disk removal
rm ~/.colima/default/diffdisk
rm ~/.colima/default/basedisk
```

---

## 5. SSH and Host Communication

### 5.1 SSH Configuration

Lima manages SSH access automatically:

```go
// From limaconfig/config.go
type SSH struct {
    LocalPort         int  `yaml:"localPort,omitempty"`
    LoadDotSSHPubKeys bool `yaml:"loadDotSSHPubKeys"`
    ForwardAgent      bool `yaml:"forwardAgent"`
}
```

### 5.2 SSH Connection

```go
// From app/app.go - SSH method
func (c colimaApp) SSH(args ...string) error {
    ctx := context.Background()
    if !c.guest.Running(ctx) {
        return fmt.Errorf("%s not running", config.CurrentProfile().DisplayName)
    }

    workDir, err := os.Getwd()
    if err != nil {
        return fmt.Errorf("error retrieving current working directory: %w", err)
    }

    // Verify PWD is mounted
    conf, _ := configmanager.LoadInstance()
    pwd, _ := util.CleanPath(workDir)
    for _, m := range conf.MountsOrDefault() {
        location := m.MountPoint
        if location == "" {
            location = m.Location
        }
        if strings.HasPrefix(pwd, location) {
            return nil
        }
    }

    guest := lima.New(host.New())
    return guest.SSH(workDir, args...)
}
```

### 5.3 SSH Config Generation

```go
// From app/app.go - generateSSHConfig
func generateSSHConfig(modifySSHConfig bool) error {
    instances, err := limautil.Instances()
    if err != nil {
        return fmt.Errorf("error retrieving instances: %w", err)
    }

    var buf bytes.Buffer
    for _, i := range instances {
        if !i.Running() {
            continue
        }

        profile := config.ProfileFromName(i.Name)
        resp, err := limautil.ShowSSH(profile.ID)
        if err != nil {
            continue
        }

        fmt.Fprintln(&buf, resp.Output)
    }

    // Write to ~/.colima/ssh_config
    sshFileColima := config.SSHConfigFile()
    os.WriteFile(sshFileColima, buf.Bytes(), 0644)

    // Optionally include in ~/.ssh/config
    if modifySSHConfig {
        includeLine := "Include " + sshFileColima
        sshFileSystem := filepath.Join(util.HomeDir(), ".ssh", "config")
        // Prepend include line to ~/.ssh/config
    }

    return nil
}
```

### 5.4 Port Forwarding

```go
// From limaconfig/config.go
type PortForward struct {
    GuestIPMustBeZero bool   `yaml:"guestIPMustBeZero,omitempty"`
    GuestIP           net.IP `yaml:"guestIP,omitempty"`
    GuestPort         int    `yaml:"guestPort,omitempty"`
    GuestPortRange    [2]int `yaml:"guestPortRange,omitempty"`
    GuestSocket       string `yaml:"guestSocket,omitempty"`
    HostIP            net.IP `yaml:"hostIP,omitempty"`
    HostPort          int    `yaml:"hostPort,omitempty"`
    HostSocket        string `yaml:"hostSocket,omitempty"`
    Proto             string `yaml:"proto,omitempty"`  // tcp, udp
    Ignore            bool   `yaml:"ignore,omitempty"`
}

// Default port forwards configured by Colima
l.PortForwards = []PortForward{
    // Docker socket
    {GuestSocket: "/var/run/docker.sock", HostSocket: docker.HostSocketFile()},
    // Containerd socket
    {GuestSocket: "/run/containerd/containerd.sock", HostSocket: containerd.HostSocketFiles().Containerd},
    // Kubernetes API
    {GuestPort: 6443, HostPort: 6443},
    // Incus socket
    {GuestSocket: "/var/lib/incus/unix.socket", HostSocket: incus.HostSocketFile()},
}
```

---

## 6. Provision Scripts

### 6.1 Provision Modes

```go
const (
    ProvisionModeAfterBoot = "after-boot"  // Run after VM boots
    ProvisionModeReady     = "ready"       // Run after runtimes start
)

type Provision struct {
    Mode   string `yaml:"mode"`
    Script string `yaml:"script"`
}
```

### 6.2 Built-in Provisioning

```go
// From lima/lima.go - addPostStartActions
func (l *limaVM) addPostStartActions(a *cli.ActiveCommandChain, conf config.Config) {
    // Setup DNS
    a.Add(func() error {
        if err := l.setupDNS(conf); err != nil {
            return fmt.Errorf("error setting up DNS: %w", err)
        }
        return nil
    })

    // Copy registry certs
    a.Add(l.copyCerts)

    // Cross-platform emulation (binfmt)
    a.Add(func() error {
        if conf.Binfmt != nil && *conf.Binfmt {
            if arch := environment.HostArch(); arch == environment.Arch(conf.Arch).Value() {
                core.SetupBinfmt(l.host, l, environment.Arch(conf.Arch))
            }
        }

        // Rosetta for vz
        if l.limaConf.VMOpts.VZOpts.Rosetta.Enabled {
            l.Run("sudo", "sh", "-c", `stat /proc/sys/fs/binfmt_misc/rosetta || echo ':rosetta:M::...' > /proc/sys/fs/binfmt_misc/register`)
        }

        return nil
    })

    // Preserve state
    a.Add(func() error {
        configmanager.SaveToFile(conf, config.CurrentProfile().StateFile())
        return nil
    })
}
```

### 6.3 Custom Provision Scripts

Users can add custom provision scripts in config:

```yaml
# ~/.colima/default/colima.yaml
provision:
  - mode: after-boot
    script: |
      #!/bin/bash
      # Runs as root after VM boots
      apt-get update
      apt-get install -y htop vim
  - mode: ready
    script: |
      #!/bin/bash
      # Runs as current user after runtimes start
      echo "Colima is ready!" >> /home/user/ready.log
```

```go
// From app/app.go - runProvisionScripts
func (c colimaApp) runProvisionScripts(conf config.Config, mode string) {
    var failed bool
    for _, s := range conf.Provision {
        if s.Mode != mode {
            continue
        }
        if err := c.guest.Run("sh", "-c", s.Script); err != nil {
            failed = true
        }
    }
    if failed {
        log.Warnln(fmt.Errorf("error running %s provision script(s)", mode))
    }
}
```

---

## 7. Multi-Instance Support

### 7.1 Profiles

Colima supports multiple isolated instances (profiles):

```bash
# Start default instance
colima start

# Start named instance
colima start dev --cpu 4 --memory 8
colima start prod --cpu 8 --memory 16

# List instances
colima list

# Status of specific instance
colima status dev

# Switch Docker context
docker context use colima-dev
```

### 7.2 Profile Isolation

Each profile has separate:
- VM configuration
- Disk storage
- Container runtime
- Network settings
- SSH config

```
~/.colima/
├── default/
│   ├── colima.yaml
│   ├── diffdisk
│   └── ...
├── dev/
│   ├── colima.yaml
│   ├── diffdisk
│   └── ...
└── prod/
    ├── colima.yaml
    ├── diffdisk
    └── ...
```

### 7.3 Profile Management

```go
// From config/profile.go
type Profile struct {
    Name        string
    DisplayName string
    ID          string  // Unique identifier
}

func CurrentProfile() *Profile {
    if name := EnvProfile(); name != "" {
        return ProfileFromName(name)
    }
    return ProfileFromName("default")
}

func ProfileFromName(name string) *Profile {
    return &Profile{
        Name:        name,
        DisplayName: fmt.Sprintf("colima (%s)", name),
        ID:          fmt.Sprintf("colima-%s", name),
    }
}
```

### 7.4 Resource Sharing

Profiles share:
- Lima cache (base images)
- Downloaded artifacts
- SSH keys

Profiles do NOT share:
- Running containers
- Container images
- Volumes
- Network configuration

---

## Summary

| Topic | Key Points |
|-------|------------|
| **Lima Integration** | Colima translates config to Lima YAML, uses limactl for VM ops |
| **VM Lifecycle** | Start: VM -> provision -> runtime; Stop: runtime -> VM |
| **VM Backends** | QEMU (compatible), vz (fast), krunkit (GPU) |
| **Disk Management** | Sparse qcow2, resizable, data persists across restarts |
| **SSH** | Auto-generated config, port forwarding for sockets |
| **Provision** | After-boot (root) and ready (user) modes |
| **Profiles** | Multiple isolated instances with shared infrastructure |

---

*Next: [Runtime Integration Deep Dive](02-runtime-integration-deep-dive.md)*
