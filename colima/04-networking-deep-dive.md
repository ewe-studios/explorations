---
title: "Colima Networking Deep Dive"
subtitle: "NAT, port forwarding, slirp, vmnet, bridged mode, and DNS configuration"
based_on: "Colima - Lima-based Container Runtime"
level: "Intermediate to Advanced"
prerequisites: "[VM Management Deep Dive](01-vm-management-deep-dive.md)"
---

# Networking Deep Dive

## Table of Contents

1. [Network Fundamentals](#1-network-fundamentals)
2. [NAT and Slirp Networking](#2-nat-and-slirp-networking)
3. [Port Forwarding](#3-port-forwarding)
4. [vmnet and Bridged Mode](#4-vmnet-and-bridged-mode)
5. [Network Address Assignment](#5-network-address-assignment)
6. [DNS Configuration](#6-dns-configuration)
7. [Network Troubleshooting](#7-network-troubleshooting)

---

## 1. Network Fundamentals

### 1.1 Network Stack Overview

```
┌─────────────────────────────────────────────────────────┐
│                    Host (macOS)                         │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐     │
│  │  Physical   │  │  Bridge     │  │  Loopback   │     │
│  │  (en0)      │  │  (vmnet)    │  │  (lo)       │     │
│  │  192.168.1.x│  │  192.168.106.x│ │  127.0.0.1 │     │
│  └─────────────┘  └─────────────┘  └─────────────┘     │
│                        │                                │
│                        │ network traffic                │
│                        v                                │
│  ┌──────────────────────────────────────────────────┐   │
│  │              Lima VM (Linux)                     │   │
│  │  ┌────────────────────────────────────────────┐  │   │
│  │  │           eth0 (VM Network)                │  │   │
│  │  │  192.168.5.x (NAT) or 192.168.106.x (vmnet)│  │   │
│  │  │                                            │  │   │
│  │  │  ┌──────────┐  ┌──────────┐  ┌──────────┐ │  │   │
│  │  │  │ Docker   │  │ k3s      │  │ Incus    │ │  │   │
│  │  │  │ :80, :443│  │ :6443    │  │ :8443    │ │  │   │
│  │  │  └──────────┘  └──────────┘  └──────────┘ │  │   │
│  │  └────────────────────────────────────────────┘  │   │
│  └──────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────┘
```

### 1.2 Network Modes

| Mode | Description | Use Case |
|------|-------------|----------|
| **NAT (slirp)** | VM behind NAT, outbound only | Default, simple setup |
| **vmnet (shared)** | Host-VM network via vmnet | Port forwarding, host access |
| **vmnet (bridged)** | VM on same network as host | External access to VM |
| **user-v2** | QEMU user networking | Alternative to slirp |

### 1.3 Network Configuration Structure

```go
// From config/config.go
type Network struct {
    Address         bool              `yaml:"address"`         // Reachable IP
    DNSResolvers    []net.IP          `yaml:"dns"`             // DNS servers
    DNSHosts        map[string]string `yaml:"dnsHosts"`        // Custom DNS entries
    HostAddresses   bool              `yaml:"hostAddresses"`   // Host IP forwarding
    Mode            string            `yaml:"mode"`            // shared, bridged
    BridgeInterface string            `yaml:"interface"`       // en0, en1, etc.
    PreferredRoute  bool              `yaml:"preferredRoute"`  // VM as default route
    GatewayAddress  net.IP            `yaml:"gatewayAddress"`  // Custom gateway
}
```

---

## 2. NAT and Slirp Networking

### 2.1 How NAT/Slirp Works

**Slirp** (Socket-Level Network) is a user-mode network stack that provides NAT for VMs:

```
┌─────────────────────────────────────────────────────────┐
│                    Host (macOS)                         │
│                                                         │
│         ┌───────────────────────────────────┐           │
│         │      QEMU Slirp (NAT)             │           │
│         │  ┌──────┐  ┌──────┐  ┌──────┐    │           │
│         │  │  TCP │  │  UDP │  │  DNS │    │           │
│         │  │Proxy │  │Proxy │  │Proxy │    │           │
│         │  └──────┘  └──────┘  └──────┘    │           │
│         └───────────────────────────────────┘           │
│                    │                                    │
│         (socket-based, no root required)                │
│                    │                                    │
│                    v                                    │
│  ┌──────────────────────────────────────────────────┐   │
│  │              Lima VM (Linux)                     │   │
│  │  eth0: 10.0.2.15 (NAT network)                   │   │
│  │  gateway: 10.0.2.2                               │   │
│  │  dns: 10.0.2.3                                   │   │
│  │                                                  │   │
│  │  Outbound: Works automatically                   │   │
│  │  Inbound: Requires port forwarding               │   │
│  └──────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────┘
```

### 2.2 Slirp Characteristics

**Advantages:**
- No root/sudo required
- Works out of the box
- Secure (VM isolated from host network)
- No configuration needed

**Disadvantages:**
- VM not reachable from host network
- Performance overhead (user-mode)
- Limited protocol support

**Default Configuration:**
```go
// Lima default slirp network
defaultLimaNetworkConfig = limautil.LimaNetwork{
    Networks: struct {
        UserV2 limautil.LimaNetworkConfig `yaml:"user-v2"`
    }{
        UserV2: limautil.LimaNetworkConfig{
            Mode:    "user-v2",
            Gateway: net.ParseIP("192.168.5.2"),
            Netmask: "255.255.255.0",
        },
    },
}
```

### 2.3 Custom Gateway Address

```bash
# Set custom gateway address
colima start --gateway-address 192.168.100.1
```

```go
// From lima/network.go
func (l *limaVM) writeNetworkFile(conf config.Config) error {
    gatewayAddress := conf.Network.GatewayAddress
    if gatewayAddress != nil {
        defaultLimaNetworkConfig.Networks.UserV2.Gateway = gatewayAddress
    }
    // Write network config
    os.WriteFile(networkFile, networkFileMarshalled, 0755)
}
```

---

## 3. Port Forwarding

### 3.1 Port Forwarding Mechanisms

Colima supports two port forwarding mechanisms:

| Mechanism | Protocol | Performance | Use Case |
|-----------|----------|-------------|----------|
| **SSH** | TCP | Moderate | Default, simple setup |
| **gRPC** | TCP + UDP | Better | Lima'sSSH port forwarder |

### 3.2 Automatic Port Forwarding

Colima automatically forwards container ports:

```go
// From lima/yaml.go
func newConf(ctx context.Context, conf config.Config) (l limaconfig.Config, error) {
    // Docker socket
    l.PortForwards = append(l.PortForwards,
        limaconfig.PortForward{
            GuestSocket: "/var/run/docker.sock",
            HostSocket:  docker.HostSocketFile(),
        })

    // Containerd socket
    l.PortForwards = append(l.PortForwards,
        limaconfig.PortForward{
            GuestSocket: "/run/containerd/containerd.sock",
            HostSocket:  containerd.HostSocketFiles().Containerd,
        })

    // Kubernetes API
    l.PortForwards = append(l.PortForwards,
        limaconfig.PortForward{
            GuestPort: 6443,
            HostPort:  6443,
        })

    // Incus socket
    l.PortForwards = append(l.PortForwards,
        limaconfig.PortForward{
            GuestSocket: "/var/lib/incus/unix.socket",
            HostSocket:  incus.HostSocketFile(),
        })

    return l, nil
}
```

### 3.3 Port Forward Configuration

```go
// From limaconfig/config.go
type PortForward struct {
    GuestIPMustBeZero bool   `yaml:"guestIPMustBeZero,omitempty"`
    GuestIP           net.IP `yaml:"guestIP,omitempty"`
    GuestPort         int    `yaml:"guestPort,omitempty"`
    GuestPortRange    [2]int `yaml:"guestPortRange,omitempty"`  // Port range
    GuestSocket       string `yaml:"guestSocket,omitempty"`     // Unix socket
    HostIP            net.IP `yaml:"hostIP,omitempty"`
    HostPort          int    `yaml:"hostPort,omitempty"`
    HostPortRange     [2]int `yaml:"hostPortRange,omitempty"`
    HostSocket        string `yaml:"hostSocket,omitempty"`      // Unix socket
    Proto             string `yaml:"proto,omitempty"`           // tcp, udp
    Ignore            bool   `yaml:"ignore,omitempty"`          // Skip this rule
}
```

### 3.4 Manual Port Forwarding

```bash
# Forward host port 8080 to VM port 80
colima start --port-forward 8080:80

# Forward specific IP
colima start --port-forward 127.0.0.1:8080:80

# Forward port range
colima start --port-forward 8000-8010:8000-8010
```

### 3.5 SSH Port Forwarder

```bash
# SSH forwarding is automatic
# Check forwarding status
ssh -O check -F ~/.colima/ssh_config colima-default

# Manual SSH tunnel (for debugging)
ssh -L 8080:localhost:80 -F ~/.colima/ssh_config colima-default
```

### 3.6 gRPC Port Forwarder

```yaml
# Use gRPC port forwarder (Lima native)
colima start --port-forwarder grpc
```

**Advantages of gRPC:**
- Better performance
- Supports UDP forwarding
- Lower CPU usage

**Disadvantages:**
- Requires Lima support
- Less debuggable

---

## 4. vmnet and Bridged Mode

### 4.1 What is vmnet?

**vmnet** is a macOS networking daemon that provides:
- Shared network between host and VM
- Bridged networking (VM on same network as host)
- Better performance than slirp

```
┌─────────────────────────────────────────────────────────┐
│                    Host (macOS)                         │
│  ┌──────────────────────────────────────────────────┐   │
│  │              vmnet Daemon (root)                 │   │
│  │  ┌──────────────┐  ┌──────────────┐             │   │
│  │  │  vmnet-shared│  │  vmnet-bridged│             │   │
│  │  │  192.168.106.x│ │  192.168.1.x  │             │   │
│  │  └──────────────┘  └──────────────┘             │   │
│  └──────────────────────────────────────────────────┘   │
│         │                   │                            │
│         │ socket            │ socket                     │
│         v                   v                            │
│  ┌──────────────────────────────────────────────────┐   │
│  │              Lima VM (Linux)                     │   │
│  │  eth0: 192.168.106.x (shared) or 192.168.1.x (bridged)│
│  └──────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────┘
```

### 4.2 vmnet Modes

**Shared Mode:**
- VM gets IP in private range (192.168.106.x)
- Host can reach VM
- External network cannot reach VM directly
- NAT for outbound traffic

**Bridged Mode:**
- VM gets IP on same network as host (192.168.1.x)
- VM appears as separate device on network
- External devices can reach VM
- Requires network interface selection

### 4.3 Enabling vmnet

```bash
# Enable network address (shared mode)
colima start --network-address

# Enable bridged mode
colima start --network-mode bridged --network-interface en0

# Custom configuration
colima start \
  --network-address \
  --network-mode shared \
  --network-interface en0
```

### 4.4 vmnet Process

```go
// From daemon/process/vmnet/vmnet.go
type vmnetProcess struct {
    mode         string  // shared, bridged
    netInterface string  // en0, en1, etc.
}

func (v *vmnetProcess) Start(ctx context.Context) error {
    info := Info()
    socket := info.Socket.File()
    pid := info.PidFile

    // Start vmnet daemon (requires root)
    var command *exec.Cmd

    if v.mode == "bridged" {
        command = cli.CommandInteractive("sudo", BinaryPath,
            "--vmnet-mode", "bridged",
            "--socket-group", "staff",
            "--vmnet-interface", v.netInterface,
            "--pidfile", pid,
            socket,
        )
    } else {
        command = cli.CommandInteractive("sudo", BinaryPath,
            "--vmnet-mode", "shared",
            "--socket-group", "staff",
            "--vmnet-gateway", NetGateway,      // 192.168.106.1
            "--vmnet-dhcp-end", NetDHCPEnd,     // 192.168.106.254
            "--pidfile", pid,
            socket,
        )
    }

    return command.Run()
}
```

### 4.5 vmnet Dependencies

```go
// From daemon/process/vmnet/deps.go
type sudoerFile struct{}

func (sudoerFile) Install(host environment.HostActions) error {
    // Add vmnet to sudoers
    script := `echo '%staff ALL=(ALL) NOPASSWD: /opt/colima/bin/colima-vmnet' | sudo tee /etc/sudoers.d/colima-vmnet`
    return host.Run("sh", "-c", script)
}

type vmnetFile struct{}

func (vmnetFile) Install(host environment.HostActions) error {
    // Download vmnet binary if not present
    // https://github.com/lima-vm/socket_vmnet
    return downloader.Download(vmnetURL, vmnetPath)
}
```

### 4.6 Network Mode Comparison

| Feature | NAT (slirp) | vmnet (shared) | vmnet (bridged) |
|---------|-------------|----------------|-----------------|
| **VM IP** | 10.0.2.x | 192.168.106.x | Same as host network |
| **Host Access** | Via port forward | Direct | Direct |
| **External Access** | No | No | Yes |
| **Setup** | Automatic | Requires vmnet | Requires vmnet |
| **Root Required** | No | Yes (vmnet) | Yes (vmnet) |
| **Performance** | Lower | Better | Best |

---

## 5. Network Address Assignment

### 5.1 Reachable IP Address

When `--network-address` is enabled, the VM gets a reachable IP:

```bash
# Start with network address
colima start --network-address

# VM IP (shared mode)
# 192.168.106.x

# VM IP (bridged mode)
# 192.168.1.x (same subnet as host)
```

### 5.2 Host IP Address Replication

```go
// From lima/network.go
func (l *limaVM) replicateHostAddresses(conf config.Config) error {
    if !conf.Network.Address && conf.Network.HostAddresses {
        for _, ip := range util.HostIPAddresses() {
            if err := l.RunQuiet("sudo", "ip", "address", "add",
                ip.String()+"/24", "dev", "lo"); err != nil {
                return err
            }
        }
    }
    return nil
}
```

This allows the VM to respond to host IP addresses.

### 5.3 Preferred Route

```yaml
# Use VM IP as preferred route (for Incus)
colima start --network-preferred-route
```

```go
// From start.go
if !cmd.Flag("network-preferred-route").Changed {
    if startCmdArgs.Runtime == incus.Name && startCmdArgs.VMType == "vz" {
        startCmdArgs.Network.PreferredRoute = true
    }
}
```

### 5.4 Network Interface Selection

```bash
# List available interfaces
networksetup -listallhardwareports

# Use specific interface for bridged mode
colima start --network-mode bridged --network-interface en0

# Common interfaces:
# en0 - Wi-Fi (or Ethernet on some Macs)
# en1 - Ethernet (or Thunderbolt)
# en2, en3, etc. - Additional interfaces
```

---

## 6. DNS Configuration

### 6.1 DNS Resolvers

```bash
# Custom DNS servers
colima start --dns 8.8.8.8 --dns 8.8.4.4
colima start --dns 1.1.1.1
```

```yaml
# In config file
network:
  dns:
    - 8.8.8.8
    - 1.1.1.1
```

### 6.2 Custom DNS Hosts

```bash
# Map custom hostnames
colima start --dns-host example.com=1.2.3.4
colima start --dns-host myapp.local=192.168.1.100
```

```go
// From start.go
func dnsHostsFromFlag(hosts []string) map[string]string {
    mapping := make(map[string]string)
    for _, h := range hosts {
        str := strings.SplitN(h, "=", 2)
        if len(str) != 2 {
            log.Warnf("unable to parse custom dns host: %v, skipping\n", h)
            continue
        }
        mapping[str[0]] = str[1]
    }
    return mapping
}
```

### 6.3 DNS Resolution Flow

```
┌─────────────────────────────────────────────────────────┐
│                    Application                          │
│                         │                               │
│                         v                               │
│  ┌──────────────────────────────────────────────────┐   │
│  │  /etc/resolv.conf (VM)                           │   │
│  │  nameserver 192.168.5.3 (Lima DNS)               │   │
│  └──────────────────────────────────────────────────┘   │
│                         │                               │
│                         v                               │
│  ┌──────────────────────────────────────────────────┐   │
│  │  Lima Host Resolver                              │   │
│  │  - Custom hosts (/etc/hosts)                     │   │
│  │  - Forward to host DNS                           │   │
│  └──────────────────────────────────────────────────┘   │
│                         │                               │
│                         v                               │
│  ┌──────────────────────────────────────────────────┐   │
│  │  macOS DNS (from network settings)               │   │
│  │  - ISP DNS                                     │   │
│  │  - 8.8.8.8                                     │   │
│  │  - 1.1.1.1                                     │   │
│  └──────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────┘
```

### 6.4 DNS in Containers

```bash
# Docker inherits DNS from VM
docker run alpine cat /etc/resolv.conf
# nameserver 192.168.5.3

# Override DNS for specific container
docker run --dns 8.8.8.8 alpine cat /etc/resolv.conf
```

---

## 7. Network Troubleshooting

### 7.1 Diagnostic Commands

```bash
# Check VM status and IP
colima status

# Get VM IP address
colima ssh -- hostname -I

# Test connectivity from VM
colima ssh -- ping -c 3 8.8.8.8
colima ssh -- curl -I https://google.com

# Check port forwarding
lsof -i :8080  # Check if port is forwarded

# DNS test
colima ssh -- nslookup google.com
colima ssh -- cat /etc/resolv.conf
```

### 7.2 Common Issues

| Issue | Symptoms | Solution |
|-------|----------|----------|
| No outbound | Cannot reach internet | Check VM DNS, restart colima |
| Port not forwarded | Cannot access container | Verify `--network-address`, check firewall |
| Slow DNS | Slow resolution | Try different DNS servers |
| Bridged not working | No IP on network | Check interface name, vmnet status |

### 7.3 Network Debug Flow

```bash
# 1. Check VM is running
colima status
# Should show "running"

# 2. Check IP address
colima ssh -- ip addr show eth0
# Should show IP in expected range

# 3. Test gateway
colima ssh -- ping -c 3 192.168.5.2
# Should respond

# 4. Test external
colima ssh -- ping -c 3 8.8.8.8
# Should respond

# 5. Test DNS
colima ssh -- nslookup google.com
# Should resolve

# 6. Check port forward
curl http://localhost:8080
# Should reach container
```

### 7.4 Firewall Considerations

```bash
# macOS firewall may block vmnet
# Check firewall status
sudo /usr/libexec/ApplicationFirewall/socketfilterfw --getglobalstate

# If enabled, allow vmnet
sudo /usr/libexec/ApplicationFirewall/socketfilterfw --add /opt/colima/bin/colima-vmnet
sudo /usr/libexec/ApplicationFirewall/socketfilterfw --unblockapp /opt/colima/bin/colima-vmnet
```

---

## Summary

| Topic | Key Points |
|-------|------------|
| **NAT/Slirp** | Default, no root, isolated, port forwarding needed |
| **Port Forwarding** | SSH (default), gRPC (better performance) |
| **vmnet** | Shared (192.168.106.x) or Bridged (host network) |
| **Network Address** | `--network-address` enables reachable IP |
| **DNS** | Custom resolvers, custom hosts, inherits from host |
| **Troubleshooting** | Check VM status, test connectivity, verify ports |

---

*Next: [Rust Revision](rust-revision.md)*
