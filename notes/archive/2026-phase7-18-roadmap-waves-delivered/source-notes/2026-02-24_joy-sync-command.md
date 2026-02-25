---
date: 2026-02-24
commit: pending
tags:
  - phase-8
  - sync
  - lockfile
  - dependencies
  - tests
related_components:
  - /Users/autoparallel/Code/joy/src/cli.rs
  - /Users/autoparallel/Code/joy/src/commands/mod.rs
  - /Users/autoparallel/Code/joy/src/commands/sync.rs
  - /Users/autoparallel/Code/joy/src/commands/build.rs
  - /Users/autoparallel/Code/joy/tests/add_command.rs
  - https://github.com/harnesslabs/joy/issues/27
---

# `joy sync` Command (Phase 8 Bootstrap)

## Why

Phase 8 starts by separating dependency materialization and lockfile refresh from application compilation. `joy build` and `joy run` worked, but there was no command for CI/local workflows that only needed `.joy` state and `joy.lock` updated without producing an application binary.

## What Changed

- Added `joy sync` CLI parsing and command dispatch.
- Implemented `sync_project(...)` in `src/commands/build.rs` to reuse the shared dependency pipeline while skipping final Ninja generation and app compilation.
- Reused the staged dependency helpers introduced in Phase 7 (`resolve`, `prefetch`, compiled-dependency build) so `sync` and `build` stay aligned on lockfile package metadata.
- Added command-aware lockfile/toolchain helper usage so `sync` errors report the `sync` command and `joy sync --update-lock` examples instead of `build`.

## Coverage

- Added integration coverage for a header-only dependency (`nlohmann/json`) proving `joy sync`:
  - materializes dependency headers and lockfile package records
  - reports JSON `command = "sync"`
  - does not emit `.joy/build/build.ninja`
  - does not produce an app binary in `.joy/bin`

## Follow-Up

Phase 8 still needs `--offline` / `--frozen` semantics and fetch retry/backoff behavior. `sync_project(...)` now provides the command seam those features can target consistently with `build`/`run`.
