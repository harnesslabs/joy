---
date: 2026-02-24
commit: b7b8b6b
tags:
  - phase1
  - scaffolding
  - cli
  - tests
related_components:
  - /Users/autoparallel/Code/joy/src/commands/new.rs
  - /Users/autoparallel/Code/joy/src/commands/init.rs
  - /Users/autoparallel/Code/joy/src/templates.rs
  - /Users/autoparallel/Code/joy/tests/cli_scaffold.rs
---

# Phase 1 Scaffolding and Integration Tests

## Why

Phase 1 needed to produce a genuinely useful CLI binary, not just command parsing. This commit makes `joy new` and `joy init` create a valid project layout with deterministic templates and adds integration tests to lock down filesystem behavior, force semantics, and JSON success/error envelopes.

## Blockers & Solutions

- Blocker: JSON path assertions failed on macOS because temp directories can appear as `/var/...` while the process resolves them as `/private/var/...`.
- Solution: Canonicalized both expected and actual paths in tests before comparing, making the tests portable across path aliases.

- Blocker: Shared scaffolding logic between `new` and `init` risked code duplication and inconsistent overwrite behavior.
- Solution: Moved file-writing and overwrite checks into shared helpers in `src/commands/mod.rs`, so both commands use the same semantics and error codes.

## Fallback Plan

If command helper sharing in `src/commands/mod.rs` becomes hard to maintain as Phase 2+ expands, extract the shared scaffolding logic into a dedicated internal module (e.g., `src/commands/scaffold.rs`) and keep `new`/`init` handlers thin wrappers around it.
