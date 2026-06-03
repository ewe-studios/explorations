---
title: NSS
prev: 00-overview.md
next: 02-pam.md
---

# NSS Module

Name Service Switch for rauthy.

## Overview

`libnss_rauthy.so` resolves users, groups, and hosts from rauthy.

## Installation

```bash
# Install NSS module
sudo cp libnss_rauthy.so /lib/libnss_rauthy.so.2
```

### nsswitch.conf

```
# /etc/nsswitch.conf
passwd:         files rauthy
group:          files rauthy
shadow:         files
hosts:          files rauthy dns
```

**Aha:** `files` first, then `rauthy` for fallback.

## User Resolution

### getent passwd

```bash
$ getent passwd
root:x:0:0:root:/root:/bin/bash
alice:x:10001:10001:Alice:/home/alice:/bin/bash
```

Users from rauthy appear alongside local users.

### User Lookup

```rust
// src/nss-module/passwd.rs
use libnss::passwd::{Passwd, PasswdHooks};

struct NssRauthy;

impl PasswdHooks for NssRauthy {
    fn get_all_entries() -> Vec<Passwd> {
        // Fetch from rauthy
        let users = rauthy_client::get_users().unwrap();
        
        users.into_iter().map(|u| Passwd {
            name: u.email,
            passwd: "x".to_string(),
            uid: u.uid,
            gid: u.gid,
            gecos: u.display_name,
            dir: format!("/home/{}", u.email),
            shell: "/bin/bash".to_string(),
        }).collect()
    }
    
    fn get_entry_by_name(name: &str) -> Option<Passwd> {
        rauthy_client::get_user(name)
            .ok()
            .flatten()
            .map(|u| Passwd {
                name: u.email,
                passwd: "x".to_string(),
                uid: u.uid,
                gid: u.gid,
                gecos: u.display_name,
                dir: format!("/home/{}", u.email),
                shell: "/bin/bash".to_string(),
            })
    }
}
```

## Group Resolution

### getent group

```bash
$ getent group
wheel:x:10:
rauthy-users:x:10001:alice,bob
```

### Merged Groups

Rauthy can map to local GIDs:

```toml
# rauthy-pam-nss.toml
[[group]]
name = "rauthy-wheel"
local_gid = 10  # Maps to local wheel group
rauthy_gid = 10001
```

## Host Resolution

### getent hosts

```bash
$ getent hosts
192.168.1.10  server1
192.168.1.11  server2
```

**Note:** May not display with `getent hosts <name>` but works for `ping`.

## Caching

```rust
// NSS results cached for performance
const CACHE_TTL: Duration = Duration::from_secs(60);
```

## Next Steps

Continue to [PAM →](02-pam.html).
