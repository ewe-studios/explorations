# DST Implementation Tasks

## Completed

- [x] Create `exploration.md` - Architecture analysis, Hiisi patterns, RawConn design
- [x] Create `rust-revision.md` - Complete Rust implementation with code examples
- [x] Create `README.md` - Documentation and quick start guide
- [x] Create example projects:
  - [x] `rawconn-traits/` - RawConn trait abstraction example
  - [x] `io-dispatcher/` - Hiisi-style I/O dispatcher
  - [x] `simulation-kernel/` - Deterministic simulation kernel

## Pending

### Phase 1: Core Abstractions for foundation_core

- [ ] Add `RawConn` trait to `foundation_core/src/io/raw_conn.rs`
- [ ] Add `TokioConn` implementation for production
- [ ] Add `Clock` trait for time abstraction
- [ ] Create feature gate infrastructure (`simulation` feature)
- [ ] Update `foundation_core/src/io/mod.rs` with conditional exports

### Phase 2: Simulation Implementation

- [ ] Implement `SimKernel` with virtual time
- [ ] Implement `SimConn` for simulation mode
- [ ] Create `VirtualStream`, `VirtualListener`, `VirtualUdpSocket`
- [ ] Implement network simulation (buffers, queues, delivery)
- [ ] Add basic fault injection (drop, delay)

### Phase 3: Test Infrastructure

- [ ] Create simulation test harness
- [ ] Add example simulation tests
- [ ] Integrate with proptest for property-based testing
- [ ] Create reproducible test case templates

### Phase 4: Multi-Node Simulation

- [ ] Implement `MultiNodeSim` framework
- [ ] Add network partition simulation
- [ ] Implement fault scheduling
- [ ] Create distributed protocol test templates

### Phase 5: ewe_platform Integration

- [ ] Migrate foundation_core networking to use `RawConn`
- [ ] Update existing tests to use simulation mode
- [ ] Add CI configuration for simulation tests
- [ ] Document patterns and best practices

### Phase 6: Advanced Features

- [ ] Add OCSP stapling simulation
- [ ] Implement bandwidth limiting
- [ ] Add packet reordering simulation
- [ ] Create execution trace visualization
- [ ] Add seed-based test replay tooling

## Implementation Notes

### Key Files to Create/Modify

```
foundation_core/
├── src/
│   ├── io/
│   │   ├── mod.rs              # MODIFY: feature-gated exports
│   │   ├── raw_conn.rs         # NEW: RawConn trait
│   │   ├── tokio_conn.rs       # NEW: Production impl
│   │   └── simulation/         # NEW: Simulation module
│   │       ├── mod.rs
│   │       ├── kernel.rs       # SimKernel
│   │       ├── conn.rs         # SimConn impl
│   │       ├── virtual.rs      # Virtual types
│   │       └── network.rs      # Network config
│   └── time/
│       └── clock.rs            # NEW: Clock abstraction
├── Cargo.toml                  # MODIFY: add simulation feature
└── tests/
    └── simulation/             # NEW: Simulation tests
```

### Migration Checklist

For each module using networking:

1. Replace direct `tokio::net` usage with `RawConn` generic
2. Replace `Instant::now()` with `C::now()` or `Clock::now()`
3. Replace `tokio::time::sleep()` with `C::sleep()`
4. Add `C: RawConn` generic parameter
5. Test in both production and simulation modes

### Testing Strategy

```rust
// Unit tests with simulation
#[test]
#[cfg(feature = "simulation")]
fn test_basic_operation() {
    let kernel = SimKernel::with_seed(42);
    kernel.run(|| async {
        // Test code
    });
}

// Property-based tests
proptest! {
    #[test]
    fn test_with_any_seed(seed in any::<u64>()) {
        let kernel = SimKernel::with_seed(seed);
        // Property should hold for all seeds
    }
}

// Regression tests (reproduce bugs)
#[test]
fn test_issue_123() {
    // Seed from failed CI run
    let kernel = SimKernel::with_seed(987654321);
    kernel.run(|| async {
        // Verify bug is fixed
    });
}
```

## References

- Hiisi Architecture: `/home/darkvoid/Boxxed/@formulas/src.rust/src.turso/hiisi/ARCHITECTURE.md`
- TigerBeetle I/O Dispatch: https://tigerbeetle.com/blog/a-friendly-abstraction-over-iouring-and-kqueue
- libxev: https://github.com/mitchellh/libxev
