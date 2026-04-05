# cf-daemonize Documentation

**Last Updated:** 2026-04-05

---

## Overview

**cf-daemonize** is a fork of the [`daemonize` crate](https://github.com/knsd/daemonize), maintained by Cloudflare for use in [`boringtun`](https://github.com/cloudflare/boringtun) (Cloudflare's Userspace WireGuard implementation).

This fork exists because the original `daemonize` crate was inactive at the time of forking. Cloudflare needed specific features for boringtun that required modifications to the original crate.

---

## Purpose

### What is Daemonization?

Daemonization is the process of running a program as a **background daemon** (Unix service) that:

1. **Detaches from terminal** - Runs independently of user session
2. **Runs in background** - No controlling terminal
3. **Has proper privileges** - Correct user/group permissions
4. **Has correct file descriptors** - stdin/stdout/stderr redirected
5. **Has a PID file** - Track process ID for management

### Why Fork?

The original `daemonize` crate was inactive, and Cloudflare needed:

- **Specific features for boringtun** - WireGuard daemon requirements
- **Bug fixes** - Issues not addressed in original
- **Maintenance** - Active development and support

---

## Installation

### Cargo (from GitHub)

```toml
# Cargo.toml
[dependencies]
daemonize = { git = "https://github.com/cloudflare/cf-daemonize" }
```

### Cargo (from crates.io - if published)

```toml
# Cargo.toml
[dependencies]
daemonize = "0.5"  # or latest version
```

---

## Usage

### Basic Daemonization

```rust
use daemonize::Daemonize;
use std::fs::File;

fn main() {
    let stdout = File::create("/var/log/mydaemon.log").unwrap();
    let stderr = File::create("/var/log/mydaemon.err").unwrap();

    let daemonize = Daemonize::new()
        .pid_file("/var/run/mydaemon.pid")
        .chown_pid_file(true)
        .working_directory("/tmp")
        .user("nobody")
        .group("nogroup")
        .umask(0o027)
        .stdout(stdout)
        .stderr(stderr)
        .exit_process(false);

    match daemonize.start() {
        Ok(_) => println!("Success, daemonized"),
        Err(e) => eprintln!("Error, {}", e),
    }

    // Daemon code continues here
    run_daemon();
}

fn run_daemon() {
    // Your daemon logic
    loop {
        // Do work...
    }
}
```

### Minimal Example

```rust
use daemonize::Daemonize;

fn main() {
    let daemonize = Daemonize::new();
    
    match daemonize.start() {
        Ok(_) => {
            // Running as daemon
            println!("Daemon started with PID: {}", std::process::id());
        }
        Err(e) => {
            eprintln!("Failed to daemonize: {}", e);
            std::process::exit(1);
        }
    }

    // Daemon work
}
```

### With Callbacks

```rust
use daemonize::Daemonize;

fn main() {
    let daemonize = Daemonize::new()
        .pid_file("/var/run/mydaemon.pid")
        .chown_pid_file(true)
        .user("myuser")
        .group("mygroup")
        .umask(0o027)
        .working_directory("/var/lib/mydaemon")
        .stdout(File::create("/var/log/mydaemon.out").unwrap())
        .stderr(File::create("/var/log/mydaemon.err").unwrap())
        .privileged_action(|| println!("Running as privileged user before drop"))
        .exit_process(false);

    match daemonize.start() {
        Ok(_) => println!("Daemon started"),
        Err(e) => eprintln!("Error: {}", e),
    }

    // Daemon continues with dropped privileges
}
```

---

## API Reference

### Daemonize Builder

```rust
pub struct Daemonize<'a> {
    // Configuration options
}

impl<'a> Daemonize<'a> {
    /// Create new Daemonize instance with defaults
    pub fn new() -> Self;

    /// Set PID file path
    pub fn pid_file<P: Into<&'a str>>(self, path: P) -> Self;

    /// Chown PID file to daemon user (requires root)
    pub fn chown_pid_file(self, chown: bool) -> Self;

    /// Set working directory
    pub fn working_directory<P: Into<&'a str>>(self, path: P) -> Self;

    /// Set user to run as (requires root)
    pub fn user<U: Into<&'a str>>(self, user: U) -> Self;

    /// Set group to run as (requires root)
    pub fn group<G: Into<&'a str>>(self, group: G) -> Self;

    /// Set umask
    pub fn umask(self, umask: u16) -> Self;

    /// Redirect stdout
    pub fn stdout<T: Into<File>>(self, file: T) -> Self;

    /// Redirect stderr
    pub fn stderr<T: Into<File>>(self, file: T) -> Self;

    /// Redirect stdin
    pub fn stdin<T: Into<File>>(self, file: T) -> Self;

    /// Action to run as privileged user before dropping privileges
    pub fn privileged_action<F>(self, action: F) -> Self
    where
        F: Fn() + Send + Sync;

    /// Exit process on error (default: true)
    pub fn exit_process(self, exit: bool) -> Self;

    /// Start daemonization process
    pub fn start(self) -> Result<(), Error>;
}
```

### Default Values

```rust
Daemonize::new()
    .pid_file(None)           // No PID file by default
    .chown_pid_file(false)    // Don't chown PID file
    .working_directory("/")   // Root directory
    .user(None)               // Current user
    .group(None)              // Current group
    .umask(0o022)             // Default umask
    .stdout(File::null())     // /dev/null
    .stderr(File::null())     // /dev/null
    .exit_process(true)       // Exit on error
```

---

## Daemonization Process

The `start()` method performs the following steps:

1. **First fork** - Parent exits, child continues
2. **Change session** - `setsid()` to become session leader
3. **Second fork** - Prevent acquiring controlling terminal
4. **Change working directory** - To specified path
5. **Set umask** - For file creation permissions
6. **Close file descriptors** - Close all inherited FDs
7. **Redirect stdio** - stdin/stdout/stderr to specified files
8. **Drop privileges** - Change user/group if specified
9. **Write PID file** - If pid_file is set
10. **Run privileged action** - If specified (before drop)

---

## boringtun Integration

Cloudflare uses cf-daemonize in boringtun:

```rust
// Simplified from boringtun
use daemonize::Daemonize;

fn main() {
    let daemonize = Daemonize::new()
        .pid_file("/var/run/boringtun.pid")
        .user("wireguard")
        .group("wireguard")
        .umask(0o077);

    match daemonize.start() {
        Ok(_) => run_wireguard_daemon(),
        Err(e) => {
            eprintln!("Failed to daemonize: {}", e);
            std::process::exit(1);
        }
    }
}
```

---

## Systemd Service Example

```ini
# /etc/systemd/system/boringtun.service

[Unit]
Description=BoringTun WireGuard Daemon
After=network.target

[Service]
Type=forking
PIDFile=/var/run/boringtun.pid
ExecStart=/usr/bin/boringtun
ExecStop=/bin/kill -TERM $MAINPID
Restart=on-failure

# Security hardening
NoNewPrivileges=true
ProtectSystem=strict
ProtectHome=true
PrivateTmp=true

[Install]
WantedBy=multi-user.target
```

---

## Error Handling

```rust
use daemonize::{Daemonize, Error};

fn main() {
    let daemonize = Daemonize::new()
        .pid_file("/var/run/mydaemon.pid");

    match daemonize.start() {
        Ok(_) => {
            // Success
            run_daemon();
        }
        Err(Error::Io(e)) => {
            eprintln!("IO error: {}", e);
        }
        Err(Error::User(e)) => {
            eprintln!("User error: {}", e);
        }
        Err(Error::Group(e)) => {
            eprintln!("Group error: {}", e);
        }
        Err(Error::Chown(e)) => {
            eprintln!("Chown error: {}", e);
        }
        Err(Error::Pid(e)) => {
            eprintln!("PID file error: {}", e);
        }
    }
}
```

---

## Comparison: Original vs cf-daemonize

| Feature | Original daemonize | cf-daemonize |
|---------|-------------------|--------------|
| Repository | github.com/knsd/daemonize | github.com/cloudflare/cf-daemonize |
| Maintenance | Inactive (at fork time) | Active (for boringtun) |
| Features | Basic daemonization | Additional features for WireGuard |
| Bug Fixes | Limited | Cloudflare-maintained |

---

## Resources

- **cf-daemonize Repository**: https://github.com/cloudflare/cf-daemonize
- **Original daemonize Crate**: https://github.com/knsd/daemonize
- **boringtun**: https://github.com/cloudflare/boringtun
- **WireGuard**: https://www.wireguard.com/

---

## Related Documents

- [Cloudflare Tunnel](../cloudflared/00-zero-to-cloudflared.md)
- [Deep Dive: Tunnel Protocol](../cloudflared/01-tunnel-protocol-deep-dive.md)
