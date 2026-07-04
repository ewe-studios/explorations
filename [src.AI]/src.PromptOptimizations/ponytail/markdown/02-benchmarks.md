# Ponytail Benchmarks

Ponytail's benchmarks measure code reduction, cost, and speed against real agentic baselines — not static prompts but the same agent doing the same tasks without ponytail.

Source: `benchmarks/` — 12 files.

## Benchmark Types

| Benchmark | File | Purpose |
|-----------|------|---------|
| Agentic | `agentic/run.py` | Real Claude Code sessions on FastAPI + React |
| Correctness | `correctness.js` | Verify output code works correctly |
| Robustness | `robustness-audit.js` | Test ponytail against edge cases |
| Behavior | `behavior.js` | Measure agent behavior with/without ponytail |
| LOC | `loc.js` | Lines of code comparison |

## Agentic Benchmark

Source: `benchmarks/agentic/run.py` — main runner.

Runs 12 feature tasks on a real repo (FastAPI + React) with:
- **Control**: Same agent, no ponytail skill
- **Treatment**: Same agent, with ponytail skill

Measured across 4 runs per task with Haiku 4.5.

### Results

| Metric | Mean | Range |
|--------|------|-------|
| Code reduction | 54% | 0%–94% |
| Cost reduction | 20% | — |
| Speed improvement | 27% | — |

The 94% ceiling occurs where an agent over-builds (e.g., installing flatpickr + wrapper + stylesheet for a date picker that `<input type="date">` handles natively). The 0% floor occurs where the code is already minimal.

### Judge

Source: `benchmarks/agentic/judge.py`

Evaluates whether the ponytail output is:
1. **Correct**: Does the code work?
2. **Safe**: Are security/accessibility guards maintained?
3. **Complete**: Does it fulfill the task requirements?

### Tasks

Source: `benchmarks/agentic/tasks.py`

12 feature tasks across FastAPI backend and React frontend.

## Correctness Benchmark

Source: `benchmarks/correctness.js`

Verifies that ponytail-simplified code produces the same behavior as the original.

## Robustness Audit

Source: `benchmarks/robustness-audit.js`

Tests ponytail against edge cases:
- Malformed input
- Empty responses
- Unicode handling
- Path injection attempts

## LOC Benchmark

Source: `benchmarks/loc.js`

Measures lines of code with and without ponytail across multiple tasks.

## Cross-References

- [Overview](00-overview.md) — What ponytail is
- [Architecture](01-architecture.md) — Hooks, config resolution
- [Integrations](03-integrations.md) — Agent plugins, hooks

[Next → Integrations](03-integrations.md)
