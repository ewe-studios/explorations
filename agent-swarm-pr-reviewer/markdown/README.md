---
title: agent-swarm-pr-reviewer Documentation Index
---

# agent-swarm-pr-reviewer Documentation

> **An automated pull request (PR) reviewer built from several artificial intelligence (AI) agents.** Specialists each look for one type of problem. A senior coordinator checks their claims against the real code. The author gets one clean review, not six opinions.

This set distills a seven-part design guide into a clear, correct, complete exploration. Every factual claim traces to a line in the source guide `agent-swarm-guide.txt`. Code shown is marked **illustrative** and reconstructs the described design in TypeScript.

## Foundation

- [Overview](00-overview.md): the problem, the build, the core idea, and the word "swarm"
- [The original guide](01-the-original-guide.md): the verbatim source, copied as the grandfather

## Core Deep Dives

- [The pipeline](02-the-pipeline.md): the seven steps — fetch, filter, score, prepare, fan out, fan in, publish
- [Risk scoring](03-risk-scoring.md): trivial, lite, and full; the exact thresholds and sensitive paths
- [The specialists](04-the-specialists.md): the fan-out — five roles, model tiers, and "do not flag" lists
- [The coordinator](05-the-coordinator.md): the fan-in — merge, verify, drop, rank, cap, and verdict

## Cross-Cutting

- [Fail-closed and failure handling](06-fail-closed-and-failure-handling.md): never approve on failure, timeouts, fallback models
- [Security and prompt injection](07-security-and-prompt-injection.md): untrusted pull request content, read-only agents, permission denial
- [The scope bug](08-the-scope-bug.md): why an agent with nothing to do invents work, and the fix

## Build and Interview

- [The stack and build order](09-the-stack-and-build-order.md): the tools, the nine-step build order, what to test
- [Interview preparation](10-interview-prep.md): the pitch, the question-and-answer set, the terms, the tips
- [Critical assessment](11-critical-assessment.md): what it does right, what it does wrong, what to copy

## Reference

- [Glossary](12-glossary.md): every term and abbreviation in one place

---

Distilled from a design guide. Every claim traces back to `agent-swarm-guide.txt`.
