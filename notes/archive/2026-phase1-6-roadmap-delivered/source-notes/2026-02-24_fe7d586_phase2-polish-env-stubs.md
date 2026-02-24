---
date: 2026-02-24
commit: fe7d586
tags:
  - phase2
  - tests
  - build-stub
  - run-stub
related_components:
  - /Users/autoparallel/Code/joy/src/commands/build.rs
  - /Users/autoparallel/Code/joy/src/commands/run.rs
  - /Users/autoparallel/Code/joy/src/commands/mod.rs
  - /Users/autoparallel/Code/joy/tests/add_command.rs
---

# Phase 2 Polish: Env Setup for Build/Run Stubs and Snapshot Tightening

## Why

Phase 2’s contract included integrating local `.joy` layout creation into `add/build/run`. The core Phase 2 commit implemented `add`, but this follow-up closes the gap for `build` and `run` stubs so they prepare the local environment when a manifest exists. It also tightens the `joy add` integration test to assert the exact serialized manifest shape rather than substring checks.

## Blockers & Solutions

- Blocker: `build`/`run` are still stubs, so full Phase 2 integration had to avoid changing their public stub behavior (`not_implemented`, exit code 2).
- Solution: Added a small shared helper that conditionally creates the local `.joy` layout only when `joy.toml` is present, then returns the existing stub error unchanged.

- Blocker: Phase 2 tracker wording mentioned “working tree” after the follow-up code was committed.
- Solution: Updated the roadmap tracker to reference the actual follow-up commit hash.

## Fallback Plan

If the shared `ensure_local_env_if_manifest_present` helper grows beyond stub setup concerns, move it into `project_env.rs` (or a future build orchestration module) and keep `commands/mod.rs` focused on dispatch/output helpers.
