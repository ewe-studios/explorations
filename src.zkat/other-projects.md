# Other zkat Projects

**Source:** `/home/darkvoid/Boxxed/@formulas/src.rust/src.zkat/`

---

## Table of Contents

1. [big-brain (Utility AI)](#big-brain-utility-ai)
2. [orogene (Package Manager)](#orogene-package-manager)
3. [srisum-rs (CLI Tool)](#srisum-rs-cli-tool)

---

## big-brain (Utility AI)

**Version:** 0.23.0 | **License:** Apache-2.0

**Source:** `/home/darkvoid/Boxxed/@formulas/src.rust/src.zkat/big-brain/`

### Overview

`big-brain` is a **Utility AI** library for games, built for the [Bevy Game Engine](https://bevyengine.org/). It lets you define complex, intricate AI behaviors for entities based on their perception of the world.

### What is Utility AI?

Utility AI is a decision-making architecture where:
1. **Scorers** evaluate the world state and produce scores
2. **Pickers** select the best action based on scores
3. **Actions** are executed based on the selection

```
┌─────────────────────────────────────────────────────────────────┐
│                    Utility AI Architecture                       │
├─────────────────────────────────────────────────────────────────┤
│                                                                  │
│  World State                                                    │
│  (Entities, Components, Resources)                              │
│       │                                                          │
│       ▼                                                          │
│  ┌─────────────────────────────────────────────────────────┐    │
│  │                    Scorers                               │    │
│  │  ┌───────────┐  ┌───────────┐  ┌───────────┐            │    │
│  │  │ Thirsty   │  │ Hungry    │  │ Tired     │            │    │
│  │  │ Score: 0.9│  │ Score: 0.3│  │ Score: 0.1│            │    │
│  │  └───────────┘  └───────────┘  └───────────┘            │    │
│  └─────────────────────────────────────────────────────────┘    │
│       │                                                          │
│       ▼                                                          │
│  ┌─────────────────────────────────────────────────────────┐    │
│  │                    Picker                                │    │
│  │  "FirstToScore > 0.8" → Selects Thirsty                 │    │
│  └─────────────────────────────────────────────────────────┘    │
│       │                                                          │
│       ▼                                                          │
│  ┌─────────────────────────────────────────────────────────┐    │
│  │                    Actions                               │    │
│  │  ┌───────────┐  ┌───────────┐  ┌───────────┐            │    │
│  │  │ Drink     │  │ Eat       │  │ Sleep     │            │    │
│  │  │ (Running) │  │ (Queued)  │  │ (Queued)  │            │    │
│  │  └───────────┘  └───────────┘  └───────────┘            │    │
│  └─────────────────────────────────────────────────────────┘    │
│                                                                  │
└─────────────────────────────────────────────────────────────────┘
```

### Core Concepts

#### Scorers

`Scorer`s are entities that look at the world and evaluate into `Score` values.

```rust
use bevy::prelude::*;
use big_brain::prelude::*;

#[derive(Debug, Clone, Component, ScorerBuilder)]
pub struct Thirsty;

pub fn thirsty_scorer_system(
    thirsts: Query<&Thirst>,
    mut query: Query<(&Actor, &mut Score), With<Thirsty>>,
) {
    for (Actor(actor), mut score) in query.iter_mut() {
        if let Ok(thirst) = thirsts.get(*actor) {
            score.set(thirst.thirst);
        }
    }
}
```

#### Actions

`Action`s are the actual behaviors your entities will _do_.

```rust
use bevy::prelude::*;
use big_brain::prelude::*;

#[derive(Debug, Clone, Component, ActionBuilder)]
pub struct Drink;

fn drink_action_system(
    mut thirsts: Query<&mut Thirst>,
    mut query: Query<(&Actor, &mut ActionState), With<Drink>>,
) {
    for (Actor(actor), mut state) in query.iter_mut() {
        if let Ok(mut thirst) = thirsts.get_mut(*actor) {
            match *state {
                ActionState::Requested => {
                    thirst.thirst = 10.0;
                    *state = ActionState::Success;
                }
                ActionState::Cancelled => {
                    *state = ActionState::Failure;
                }
                _ => {}
            }
        }
    }
}
```

#### Thinkers

`Thinker`s combine scorers and actions with decision logic.

```rust
use big_brain::prelude::*;

fn spawn_entity(cmd: &mut Commands) {
    cmd.spawn((
        Thirst(70.0, 2.0),
        Thinker::build()
            .picker(FirstToScore { threshold: 0.8 })
            .when(Thirsty, Drink),
    ));
}
```

### Pickers (Decision Strategies)

| Picker | Description |
|--------|-------------|
| `FirstToScore` | Pick first scorer above threshold |
| `Highest` | Pick highest scoring option |
| `HighestToScore` | Pick highest scorer above threshold |

### Evaluators (Score Transformers)

| Evaluator | Formula | Use Case |
|-----------|---------|----------|
| `LinearEvaluator` | `y = mx + b` | Simple scaling |
| `PowerEvaluator` | `y = x^p` | Emphasize high scores |
| `SigmoidEvaluator` | `y = 1/(1+e^(-x))` | Smooth threshold |

### Scorers (Decision Factors)

| Scorer | Description |
|--------|-------------|
| `FixedScore` | Always returns fixed score |
| `MeasuredScorer` | Uses a Measure to combine scores |
| `AllOrNothing` | All child scorers must pass threshold |
| `SumOfScorers` | Sum of all child scorer scores |
| `ProductOfScorers` | Product of all child scorer scores |
| `WinningScorer` | Highest scoring child |
| `EvaluatingScorer` | Applies evaluator to child score |

### Measures (Score Combination)

| Measure | Description |
|---------|-------------|
| `WeightedSum` | Weighted sum of scores |
| `WeightedProduct` | Weighted product of scores |
| `ChebyshevDistance` | Maximum difference metric |

### Example: Thirst AI

```rust
use bevy::prelude::*;
use big_brain::prelude::*;

// Component to track thirst
#[derive(Component, Debug)]
pub struct Thirst {
    pub thirst: f32,
    pub drain: f32,
}

// Scorer: How thirsty am I?
#[derive(Debug, Clone, Component, ScorerBuilder)]
pub struct Thirsty;

pub fn thirsty_scorer_system(
    thirsts: Query<&Thirst>,
    mut query: Query<(&Actor, &mut Score), With<Thirsty>>,
) {
    for (Actor(actor), mut score) in query.iter_mut() {
        if let Ok(thirst) = thirsts.get(*actor) {
            // Score is 0.0 to 1.0 based on thirst level
            score.set(thirst.thirst / 100.0);
        }
    }
}

// Action: Drink water
#[derive(Debug, Clone, Component, ActionBuilder)]
pub struct Drink;

fn drink_action_system(
    mut thirsts: Query<&mut Thirst>,
    mut query: Query<(&Actor, &mut ActionState), With<Drink>>,
) {
    for (Actor(actor), mut state) in query.iter_mut() {
        if let Ok(mut thirst) = thirsts.get_mut(*actor) {
            match *state {
                ActionState::Requested => {
                    thirst.thirst = 10.0;
                    *state = ActionState::Success;
                }
                ActionState::Cancelled => {
                    *state = ActionState::Failure;
                }
                _ => {}
            }
        }
    }
}

// Setup
fn spawn_thirsty_entity(cmd: &mut Commands) {
    cmd.spawn((
        Thirst { thirst: 70.0, drain: 2.0 },
        Thinker::build()
            .picker(FirstToScore { threshold: 0.7 })
            .when(Thirsty, Drink),
    ));
}

// App setup
fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(BigBrainPlugin::new(PreUpdate))
        .add_systems(Startup, spawn_thirsty_entity)
        .add_systems(Update, thirst_system)
        .add_systems(PreUpdate, drink_action_system.in_set(BigBrainSet::Actions))
        .add_systems(PreUpdate, thirsty_scorer_system.in_set(BigBrainSet::Scorers))
        .run();
}
```

### Features

- **Highly Concurrent:** Parallel scorer evaluation
- **Bevy Integration:** Native ECS integration
- **Composable:** Build complex behaviors from simple parts
- **State Machine Actions:** Continuous actions with states
- **Action Cancellation:** Clean cancellation support
- **Reflection:** Runtime inspection via Bevy Reflect

---

## orogene (Package Manager)

**Version:** 0.3.34 | **License:** Apache-2.0

**Source:** `/home/darkvoid/Boxxed/@formulas/src.rust/src.zkat/orogene/`

### Overview

Orogene is a next-generation **package manager** for tools that use `node_modules/`. It's fast, robust, and designed for easy integration into development workflows.

### Key Features

1. **Central Content-Addressable Store:** Deduplicated package storage
2. **Copy-on-Write:** Reduced disk usage on supported filesystems
3. **Parallel Installation:** Fast concurrent package installation
4. **Lockfile Generation:** Reproducible installs
5. **NPM Registry Compatible:** Works with existing npm packages

### Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                    Orogene Architecture                          │
├─────────────────────────────────────────────────────────────────┤
│                                                                  │
│  ┌─────────────────────────────────────────────────────────┐    │
│  │                    oro (CLI)                            │    │
│  │  Commands: install, apply, resolve, fetch, etc.         │    │
│  └─────────────────────────────────────────────────────────┘    │
│       │                                                          │
│       ▼                                                          │
│  ┌─────────────────────────────────────────────────────────┐    │
│  │                 Workspace Crates                        │    │
│  │                                                          │    │
│  │  ┌─────────────────┐  ┌─────────────────┐              │    │
│  │  │   nassun        │  │ node-maintainer │              │    │
│  │  │  Package API    │  │  Tree Resolver  │              │    │
│  │  └─────────────────┘  └─────────────────┘              │    │
│  │                                                          │    │
│  │  ┌─────────────────┐  ┌─────────────────┐              │    │
│  │  │  oro-client     │  │  oro-common     │              │    │
│  │  │  Registry HTTP  │  │  Common Types   │              │    │
│  │  └─────────────────┘  └─────────────────┘              │    │
│  │                                                          │    │
│  │  ┌─────────────────┐  ┌─────────────────┐              │    │
│  │  │  oro-config     │  │ oro-package-spec│              │    │
│  │  │  Config Mgmt    │  │  Spec Parser    │              │    │
│  │  └─────────────────┘  └─────────────────┘              │    │
│  │                                                          │    │
│  │  ┌─────────────────┐  ┌─────────────────┐              │    │
│  │  │ oro-pretty-json │  │ oro-npm-account │              │    │
│  │  │  JSON Format    │  │  Auth Mgmt      │              │    │
│  │  └─────────────────┘  └─────────────────┘              │    │
│  └─────────────────────────────────────────────────────────┘    │
│       │                                                          │
│       ▼                                                          │
│  ┌─────────────────────────────────────────────────────────┐    │
│  │              External Dependencies                      │    │
│  │  cacache  │  ssri  │  miette  │  reqwest  │  tokio     │    │
│  └─────────────────────────────────────────────────────────┘    │
│                                                                  │
└─────────────────────────────────────────────────────────────────┘
```

### Workspace Crates

| Crate | Description |
|-------|-------------|
| **nassun** | Package resolution API - fetch metadata, resolve specifiers, download packages |
| **node-maintainer** | Dependency tree resolver - generates lockfiles, extracts to node_modules |
| **oro-client** | NPM registry HTTP client - package metadata, tarball downloads |
| **oro-common** | Common types - manifests, metadata, packuments |
| **oro-config** | Configuration management - config files, CLI options |
| **oro-config-derive** | Config derive macro - layer CLI with config files |
| **oro-package-spec** | Package specifier parser - `foo@^1.2.3`, aliases |
| **oro-pretty-json** | JSON formatting - pretty output |
| **oro-npm-account** | NPM account management - authentication |

### Package Specifier Support

```
npm:package@version    # NPM registry
git+https://...        # Git repositories
file:./local/path      # Local directories
npm:alias@npm:real     # Package aliases
```

### Example Usage

```bash
# Install dependencies (like npm install)
$ oro apply

# Ping the registry
$ oro ping

# Fetch package metadata
$ oro fetch express@4.18.0

# Resolve and show dependency tree
$ oro resolve

# Install to node_modules
$ oro install
```

### Dependencies on Other zkat Projects

```toml
[dependencies]
cacache = "12.0.0"      # Content-addressable cache
ssri = "9.0.0"          # Integrity verification
miette = { version = "5.8.0", features = ["fancy"] }  # Error reporting
supports-unicode = "2.0.0"  # Terminal detection
```

### Performance

Orogene benchmarks show:
- **Sub-second** installs for some non-trivial projects (warm cache)
- **Reduced disk usage** through deduplication and CoW
- **Lower memory** footprint than npm/yarn

---

## srisum-rs (CLI Tool)

**Version:** 5.0.1-alpha.0 | **License:** Apache-2.0

**Source:** `/home/darkvoid/Boxxed/@formulas/src.rust/src.zkat/srisum-rs/`

### Overview

`srisum` is a CLI tool for computing and verifying Subresource Integrity digests. It's like `sha256sum` but for SRI hashes.

### Features

- **Multiple Algorithms:** SHA-1, SHA-256, SHA-384, SHA-512, XXH3
- **Checksum Files:** Generate and verify from checksum files
- **stdin/stdout Support:** Pipe data through
- **GNU Compatible:** Similar interface to `*SUM` utilities

### Usage

#### Computing Digests

```bash
# Single file
$ srisum styles.css > styles.css.sri

# Multiple files
$ srisum styles.css index.js package.json > app.sri

# From stdin
$ cat styles.css | srisum -a sha1
sha1-hmkHOZdrfLUVOqpAgryfC8XNGtE -

# Specify algorithms
$ srisum styles.css --algorithms sha512 sha256 sha1
```

#### Verifying Integrity

```bash
# Check single file
$ srisum -c styles.css.sri
styles.css: OK (sha512)

# Check multiple checksum files
$ srisum -c styles.css.sri js-files.sri
styles.css: OK (sha512)
index.js: OK (sha512)

# Checksum from stdin
$ cat styles.css.sri | srisum -c

# Quiet mode (only errors)
$ srisum -c --quiet app.sri

# Ignore missing files
$ srisum -c --ignore-missing app.sri
```

### Command Line Options

```
OPTIONS:
    -a, --algorithms <ALGO>...    Hash algorithms to generate (sha256, sha512, etc.)
    -c, --check                   Read SRI sums from files and check them
    -d, --digest-only             Only output the digest, without filenames
        --strict                  Strict SRI compliance
        --ignore-missing          Don't fail for missing files
        --quiet                   Don't print OK for verified files
        --status                  Exit code only, no output
    -w, --warn                    Warn about improperly formatted lines
```

### Implementation

```rust
// Simplified from srisum-rs
use clap::Parser;
use ssri::{Integrity, IntegrityOpts, Algorithm};

#[derive(Parser)]
struct CliArgs {
    /// Check mode
    #[arg(short, long)]
    check: bool,

    /// Algorithms to use
    #[arg(short, long, default_value = "sha256")]
    algorithms: Vec<String>,

    /// Files to process
    files: Vec<String>,
}

fn compute(args: CliArgs) -> ssri::Result<()> {
    for file in &args.files {
        let data = std::fs::read(file)?;

        let mut opts = IntegrityOpts::new();
        for algo in &args.algorithms {
            opts = opts.algorithm(algo.parse()?);
        }

        let integrity = opts.chain(&data).result();

        if args.digest_only {
            println!("{}", integrity);
        } else {
            println!("{}  {}", integrity, file);
        }
    }
    Ok(())
}

fn check(args: CliArgs) -> ssri::Result<()> {
    for file in &args.files {
        let content = std::fs::read_to_string(file)?;

        for line in content.lines() {
            let parts: Vec<&str> = line.split("  ").collect();
            if parts.len() != 2 {
                eprintln!("{}: malformed line", file);
                continue;
            }

            let sri: Integrity = parts[0].parse()?;
            let filename = parts[1];

            let data = std::fs::read(filename)?;
            match sri.check(&data) {
                Ok(algo) => println!("{}: OK ({})", filename, algo),
                Err(_) => println!("{}: FAILED", filename),
            }
        }
    }
    Ok(())
}
```

### Dependencies

```toml
[dependencies]
ssri = "7.0.0"           # SRI parsing/generation
clap = { version = "4.1.4", features = ["derive"] }  # CLI parsing
miette = { version = "5.5.0", features = ["fancy"] } # Error reporting
thiserror = "1.0.38"     # Error handling
```

---

## Summary

The "other" zkat projects demonstrate the ecosystem's breadth:

| Project | Domain | Key Features |
|---------|--------|--------------|
| **big-brain** | Game AI | Utility AI, Bevy integration, composable behaviors |
| **orogene** | Package Management | Fast, deduplicated, NPM-compatible |
| **srisum-rs** | CLI Tools | SRI checksums, verification, GNU-compatible |

All projects share common design principles:
- Excellent error messages (miette)
- High performance
- Async-first where applicable
- Production-ready quality
