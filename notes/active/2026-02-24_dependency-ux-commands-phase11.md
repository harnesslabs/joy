---
date: 2026-02-24
commit: pending
tags:
  - phase-11
  - commands
  - dependency-management
  - machine-json
  - tests
related_components:
  - /Users/autoparallel/Code/joy/src/cli.rs
  - /Users/autoparallel/Code/joy/src/commands/mod.rs
  - /Users/autoparallel/Code/joy/src/commands/remove.rs
  - /Users/autoparallel/Code/joy/src/commands/update.rs
  - /Users/autoparallel/Code/joy/src/commands/tree.rs
  - /Users/autoparallel/Code/joy/src/manifest.rs
  - /Users/autoparallel/Code/joy/docs/machine-interface.md
  - /Users/autoparallel/Code/joy/tests/add_command.rs
  - https://github.com/harnesslabs/joy/issues/42
  - https://github.com/harnesslabs/joy/issues/43
  - https://github.com/harnesslabs/joy/issues/44
  - https://github.com/harnesslabs/joy/issues/45
  - https://github.com/harnesslabs/joy/issues/46
---

# Phase 11: Dependency UX Commands (`remove`, `update`, `tree`)

## What Shipped

- Added new CLI commands:
  - `joy remove <package>`
  - `joy update [package] [--rev <rev>]`
  - `joy tree`
- Added manifest helper `remove_dependency(...)`.
- Added deterministic JSON payloads for `remove`, `update`, and `tree`.
- Added machine-interface docs for `add/remove/update/tree` payloads and compatibility policy in `/Users/autoparallel/Code/joy/docs/machine-interface.md`.

## Behavior Notes

- `remove` removes the dependency from `joy.toml`, deletes the staged header link for the package (if present), updates `install-index.json`, and warns when `joy.lock` exists (stale after manifest mutation).
- `update` refreshes dependency headers and fetches, optionally changing the manifest rev when `--rev` is provided for a specific package. It warns when `joy.lock` exists.
- `tree` resolves the current dependency graph and emits human + JSON views with deterministic package ordering.
- `--frozen` rejects `add`, `remove`, and `update` mutations with stable machine error codes.
- `--offline` is respected by `update` and `tree` through the fetch runtime guard.

## Coverage Added

- `remove` manifest/header cleanup + stale lockfile warning
- `update --rev` manifest mutation + stale lockfile warning (using a local two-commit git fixture)
- `tree` deterministic JSON + human output for a direct dependency
- `remove/update` frozen-mode rejection
- Dedicated JSON payload shape stability assertions for `add/remove/update/tree`

## Follow-Up

- Phase 11 intentionally keeps dependency editing as exact-ref workflows. Semver ranges and solver semantics remain deferred to the later solver/registry phases.
