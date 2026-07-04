# Goose Benchmarks and Testing

The `goose-bench` and `goose-test` crates provide benchmarking and testing infrastructure.

Source: `crates/goose-bench/src/` (38 files), `crates/goose-test/src/` (6 files).

## goose-bench

Source: `crates/goose-bench/src/`

### Runners

| Runner | Purpose |
|--------|---------|
| `BenchRunner` | Run benchmark suites |
| `EvalRunner` | Run evaluations |
| `ModelRunner` | Run model-specific tests |
| `MetricAggregator` | Aggregate benchmark metrics |

### Bench Config

Source: `bench_config.rs`

```rust
pub struct BenchRunConfig {
    // Configuration for benchmark runs
}
```

## goose-test

Source: `crates/goose-test/src/`

Test utilities for goose components.

## Cross-References

- [Overview](00-overview.md) — Architecture summary
- [Core Library](01-core-library.md) — agents, providers
- [CLI](02-cli.md) — CLI commands with bench subcommands
