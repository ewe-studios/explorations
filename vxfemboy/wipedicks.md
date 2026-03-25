# WipeDicks - Secure Data Wiping

**Source:** `/home/darkvoid/Boxxed/@formulas/src.rust/vxfemboy/wipedicks/`

---

## Overview

**WipeDicks** is a multi-threaded secure file and device wiping tool that overwrites data with ASCII art patterns before deletion. It's designed for secure data erasure with humor injected through its overwrite patterns.

### What It Does

1. **Overwrites files/devices** with random ASCII art patterns
2. **Supports multiple passes** for thorough data destruction
3. **Multi-threaded wiping** for parallel processing
4. **Recursive directory wiping** with option toggle
5. **Free space wiping** to clean deleted file remnants
6. **Device wiping** for entire disks/partitions

### Important Warning

> **This tool permanently destroys data.** Use with extreme caution. Once wiped, data cannot be recovered.

---

## Architecture

### High-Level Design

```
┌─────────────────────────────────────────────────────────────┐
│                      WipeDicks                               │
├─────────────────────────────────────────────────────────────┤
│                                                              │
│  ┌─────────────┐    ┌─────────────┐    ┌─────────────┐     │
│  │     CLI     │───▶│    Parse    │───▶│   Spawn     │     │
│  │   Parser    │    │   Filelist  │    │   Threads   │     │
│  │   (clap)    │    │             │    │             │     │
│  └─────────────┘    └─────────────┘    └──────┬──────┘     │
│                                               │             │
│                                               ▼             │
│                                      ┌─────────────────┐   │
│                                      │  Thread Pool    │   │
│                                      │  ┌───────────┐  │   │
│                                      │  │ Thread 1  │  │   │
│                                      │  │ Thread 2  │  │   │
│                                      │  │ Thread 3  │  │   │
│                                      │  │    ...    │  │   │
│                                      │  └───────────┘  │   │
│                                      └─────────────────┘   │
│                                               │             │
│                                               ▼             │
│                                      ┌─────────────────┐   │
│                                      │   Wipe Function │   │
│                                      │  - Generate     │   │
│                                      │    patterns     │   │
│                                      │  - Write to     │   │
│                                      │    file/device  │   │
│                                      │  - Repeat N     │   │
│                                      │    times        │   │
│                                      └─────────────────┘   │
│                                               │             │
│                                               ▼             │
│                                      ┌─────────────────┐   │
│                                      │   Delete File   │   │
│                                      └─────────────────┘   │
│                                                              │
└─────────────────────────────────────────────────────────────┘
```

### Module Structure

```
src/
└── main.rs           # Everything in one file: CLI, wiping, threading
```

---

## Implementation Details

### 1. ASCII Art Patterns

```rust
const DICKS: &[&str] = &[
    "8=D ", "8=D~ ", "8=D~~ ", "8=D~~~ ",
    "8==D ", "8==D~ ", "8==D~~ ", "8==D~~~ ",
    "8===D ", "8===D~ ", "8===D~~ ", "8===D~~~ ",
    "8====D ", "8====D~ ", "8====D~~ ", "8====D~~~ ",
    // ... 100+ patterns with varying lengths
    "8#============D~ ", "8#===========D~~ ", "8#===========D~~~ ",
];
```

### 2. Pattern Generation

```rust
fn generate_dicks() -> Vec<String> {
    let mut dicks = Vec::new();

    // Generate patterns programmatically
    for a in 0..2 {           // 0-1 '#' characters
        for b in 1..13 {      // 1-12 '=' characters
            for c in 0..4 {   // 0-3 '~' characters
                let dick = format!(
                    "8{}{}D{} ",
                    "#".repeat(a),
                    "=".repeat(b),
                    "~".repeat(c)
                );
                dicks.push(dick);
            }
        }
    }
    dicks
}
```

### 3. Random Pattern Selection

```rust
use rand::prelude::*;

fn rand_dick(rng: &mut ThreadRng) -> &'static str {
    let index = rng.gen_range(0..DICKS.len());
    DICKS[index]
}
```

### 4. Cached Pattern Generation (Optimization)

```rust
fn fast_rand_dick<'a>(
    cache: &'a mut String,
    count: &mut usize,
    rng: &mut ThreadRng
) -> &'a str {
    // Regenerate cache when empty or exhausted
    if cache.is_empty() || *count == 0 {
        *cache = String::new();
        *count = rng.gen_range(1000..10000);

        // Pre-generate 150-300 patterns
        for _ in 0..rng.gen_range(150..300) {
            cache.push_str(rand_dick(rng));
        }
    }

    *count -= 1;
    cache
}
```

### 5. Core Wipe Function

```rust
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::Path;

fn wipe(dev: &Path, rounds: usize, rng: &mut ThreadRng) -> io::Result<()> {
    let size = fs::metadata(dev).map(|m| m.len()).unwrap_or(0);

    for _ in 0..rounds {
        let mut file = OpenOptions::new().write(true).open(dev)?;

        if size == 0 {
            // Unknown size (e.g., device) - write until full
            loop {
                let dick = rand_dick(rng);
                if file.write_all(dick.as_bytes()).is_err() {
                    break;  // Device full or error
                }
            }
        } else {
            // Known size - write exact amount
            let mut dlen = 0;
            while dlen < size {
                let dick = rand_dick(rng);
                dlen += dick.len() as u64;
                if file.write_all(dick.as_bytes()).is_err() {
                    break;
                }
            }
        }
    }

    // Delete the file after wiping
    fs::remove_file(dev)?;

    Ok(())
}
```

### 6. Directory Parsing (Recursive)

```rust
fn parse_dir(dir: &Path, recursive: bool) -> io::Result<Vec<PathBuf>> {
    let mut filelist = Vec::new();

    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();

        if path.is_dir() {
            if recursive {
                // Recursively descend into subdirectory
                filelist.extend(parse_dir(&path, recursive)?);
            }
        } else {
            // Add file to list
            filelist.push(path);
        }
    }

    Ok(filelist)
}

fn parse_filelist(filelist: &[PathBuf], recursive: bool) -> io::Result<Vec<PathBuf>> {
    let mut files = Vec::new();

    for item in filelist {
        if item.is_dir() {
            if recursive {
                files.extend(parse_dir(item, recursive)?);
            } else {
                eprintln!(
                    "WARNING: {:?} is a directory and recursive is off.",
                    item
                );
            }
        } else if item.exists() {
            files.push(item.to_path_buf());
        }
    }

    Ok(files)
}
```

### 7. Multi-threaded Wiping

```rust
use std::thread;

fn main() {
    // ... parse arguments ...

    let mut handles = Vec::new();

    // Spawn one thread per file
    for f in file_list {
        let handle = thread::spawn(move || {
            let mut rng = thread_rng();  // Each thread gets its own RNG
            if let Err(e) = wipe(&f, numrounds, &mut rng) {
                eprintln!("ERROR: {:?}: {:?}", f, e);
            }
        });
        handles.push(handle);
    }

    // Wait for all threads to complete
    for handle in handles {
        handle.join().unwrap();
    }
}
```

### 8. CLI Argument Parsing (clap v4)

```rust
use clap::{Command, Arg};

let matches = Command::new("Wipe files/devices with dicks")
    .version("0.0.1")
    .author("vxfemboy")
    .arg(
        Arg::new("recursive")
            .short('r')
            .long("recursive")
            .help("Recursively wipe directories")
            .action(clap::ArgAction::SetTrue)
    )
    .arg(
        Arg::new("numrounds")
            .short('n')
            .long("numrounds")
            .help("The number of rounds to wipe the file/device")
            .value_parser(clap::value_parser!(usize))
            .default_value("1")
    )
    .arg(
        Arg::new("wipefree")
            .short('w')
            .long("wipefree")
            .help("Wipe free space on device")
            .action(clap::ArgAction::SetTrue)
    )
    .arg(
        Arg::new("slow")
            .short('s')
            .long("slow")
            .help("Use more randomness, tends to be slower")
            .action(clap::ArgAction::SetTrue)
    )
    .arg(
        Arg::new("files")
            .help("Files or directories to wipe")
            .num_args(1..)
            .required(true)
    )
    .get_matches();
```

---

## Dependencies

```toml
[package]
name = "wipedicks"
version = "0.1.0"
edition = "2021"

[dependencies]
rand = "0.8.5"    # Random number generation
clap = "4.5.9"    # CLI argument parsing
```

---

## Usage

### Building

```bash
git clone https://github.com/vxfemboy/wipedicks.git
cd wipedicks
cargo build --release
```

### Basic Usage

```bash
# Wipe a single file
./wipedicks /path/to/file

# Wipe a directory recursively
./wipedicks -r /path/to/directory

# Wipe with 3 passes
./wipedicks -n 3 /path/to/file

# Wipe entire device (DANGEROUS)
./wipedicks /dev/sdX

# Wipe free space
./wipedicks -w /mount/point

# Slow mode (more randomness)
./wipedicks -s /path/to/file
```

### Command-Line Options

| Option | Description | Default |
|--------|-------------|---------|
| `-r, --recursive` | Recursively wipe directories | Off |
| `-n, --numrounds <N>` | Number of overwrite passes | 1 |
| `-w, --wipefree` | Wipe free space | Off |
| `-s, --slow` | Use more randomness | Off |

---

## How It Works

### Step-by-Step Wiping Process

```
┌─────────────────────────────────────────────────────────────┐
│                    WipeDicks Process                         │
├─────────────────────────────────────────────────────────────┤
│                                                              │
│  1. Parse Arguments                                          │
│     - Get file list from CLI                                │
│     - Apply recursive flag                                   │
│                                                              │
│  2. Spawn Threads                                            │
│     - One thread per file                                    │
│     - Each thread has independent RNG                        │
│                                                              │
│  3. For Each Round (1 to N)                                  │
│     a. Open file for writing                                 │
│     b. Generate random ASCII pattern                         │
│     c. Write pattern repeatedly until full                   │
│     d. Close file                                            │
│                                                              │
│  4. Delete File                                              │
│     - fs::remove_file()                                      │
│                                                              │
│  5. Join All Threads                                         │
│     - Wait for completion                                    │
│                                                              │
└─────────────────────────────────────────────────────────────┘
```

### Pattern Example

```
Original file content:
"Sensitive credit card data: 1234-5678-9012-3456"

After pass 1:
"8==D~ 8===D~~ 8====D 8=D~ 8#==D~~ 8===D~ 8====D~~~ ..."

After pass 3:
"8#===D~ 8==D~~~ 8=D 8====D~ 8#==D 8===D~~ 8=D~~~ ..."

File deleted.
```

---

## Security Considerations

### What This Tool Does

1. **Overwrites data** - Replaces original bytes with pattern data
2. **Multiple passes** - Can overwrite multiple times for thorough destruction
3. **Deletes file** - Removes file entry after wiping

### Limitations

1. **SSD Wear Leveling** - SSDs may redirect writes, leaving original data intact
2. **Journaling Filesystems** - Ext4, NTFS, APFS may keep journal copies
3. **Backups/Snapshots** - Time Machine, ZFS snapshots, cloud backups unaffected
4. **Not Cryptographic** - Patterns are random but not cryptographically secure
5. **No Verification** - Doesn't verify overwrite was successful

### For Serious Use

Consider these alternatives for production environments:

- **shred** (GNU coreutils) - Multiple passes with random data
- **srm** (secure rm) - Built for secure deletion
- **hdparm --secure-erase** - ATA secure erase command
- **blkdiscard** - Discard blocks on SSDs
- **Physical destruction** - For highest security

---

## Performance

### Multi-threading Benefits

```
Single-threaded:
File1 [████████] File2 [████████] File3 [████████] = 3x time

Multi-threaded:
File1 [████████]
File2 [████████]  = 1x time (parallel)
File3 [████████]
```

### Optimization: Cached Pattern Generation

The `fast_rand_dick` function reduces RNG calls by pre-generating 150-300 patterns at once:

```rust
// Without cache: RNG call per pattern
for _ in 0..1000 {
    rand_dick(rng);  // 1000 RNG calls
}

// With cache: RNG calls batched
fast_rand_dick(&mut cache, &mut count, rng);
// ~5-10 RNG calls for 1000 patterns
```

---

## Example Output

```
$ ./wipedicks -r -n 3 ~/secret_files/

# Files wiped:
# ~/secret_files/passwords.txt
# ~/secret_files/keys.pem
# ~/secret_files/notes.txt
# All overwritten with 3 passes of ASCII art patterns
# Then deleted
```

---

## Files

- **Main Entry:** `/home/darkvoid/Boxxed/@formulas/src.rust/vxfemboy/wipedicks/src/main.rs`
- **Cargo.toml:** `/home/darkvoid/Boxxed/@formulas/src.rust/vxfemboy/wipedicks/Cargo.toml`
- **Documentation:** `/home/darkvoid/Boxxed/@formulas/src.rust/vxfemboy/wipedicks/readme.md`

---

## Comparison to Similar Tools

| Tool | Language | Patterns | Threads | Verify |
|------|----------|----------|---------|--------|
| WipeDicks | Rust | ASCII art | Yes | No |
| shred | C | Random/fixed | No | No |
| srm | C | Multiple passes | No | Yes |
| bcwipe | C | DoD patterns | No | Yes |

---

## Summary

WipeDicks demonstrates:

1. **Clap v4 API** - Modern command/argument builder pattern
2. **Thread spawning** - One thread per file for parallel wiping
3. **File I/O** - OpenOptions for low-level file operations
4. **Random generation** - rand crate for pattern selection
5. **Recursive directory traversal** - fs::read_dir with recursion
6. **Metadata handling** - fs::metadata for file size

It's a humorous but functional example of secure deletion concepts in Rust.
