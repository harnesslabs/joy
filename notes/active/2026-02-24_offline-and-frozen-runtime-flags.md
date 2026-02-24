---
date: 2026-02-24
commit: pending
tags:
  - phase-8
  - offline
  - frozen
  - fetch
  - cli
  - tests
related_components:
  - /Users/autoparallel/Code/joy/src/cli.rs
  - /Users/autoparallel/Code/joy/src/lib.rs
  - /Users/autoparallel/Code/joy/src/commands/mod.rs
  - /Users/autoparallel/Code/joy/src/commands/add.rs
  - /Users/autoparallel/Code/joy/src/commands/build.rs
  - /Users/autoparallel/Code/joy/src/commands/run.rs
  - /Users/autoparallel/Code/joy/src/commands/sync.rs
  - /Users/autoparallel/Code/joy/src/fetch.rs
  - /Users/autoparallel/Code/joy/tests/add_command.rs
  - https://github.com/harnesslabs/joy/issues/28
  - https://github.com/harnesslabs/joy/issues/29
  - https://github.com/harnesslabs/joy/issues/31
---

# Global `--offline` and `--frozen` Runtime Flags

## Why

Phase 8 needs reproducible, CI-friendly command execution without network access and with lockfile immutability guarantees. `joy` previously had `--locked`/`--update-lock` on build-like commands, but no global cache-only mode and no cargo-style `--frozen` behavior.

## What Changed

- Added global CLI flags:
  - `--offline` (cache-only dependency resolution/fetch behavior)
  - `--frozen` (implies `--offline` and `--locked` for `build`/`run`/`sync`)
- Threaded runtime flags through command dispatch (`add`, `build`, `run`, `sync`).
- Implemented fetch runtime mode in `src/fetch.rs` via a scoped process-wide runtime flag guard used by command execution.
- Added explicit fetch offline error variants:
  - missing cached mirror / missing cached revision
  - offline network-disabled archive download
- Mapped offline fetch failures to stable machine error codes (`offline_cache_miss`, `offline_network_disabled`) in command/build paths.
- `joy add` now rejects `--frozen` with a dedicated machine error (`frozen_disallows_add`) since it mutates `joy.toml`.

## Test Coverage Added

- CLI parse coverage for global `--offline` + `--frozen`.
- `joy add --frozen` rejects with a stable error code.
- `joy sync --offline` fails on a cold cache with `offline_cache_miss`.
- `joy sync --offline` succeeds with a warm cache (using a bogus remote base to confirm cache-only behavior).
- `joy sync --frozen` succeeds with a warm cache + current lockfile and rejects `--update-lock`.
- `joy build --offline` fails on a cold cache with `offline_cache_miss`.
- `joy build --offline` succeeds with a warm cache.
- `joy run --frozen` succeeds with a warm cache + existing lockfile (using a bogus remote base to confirm cache-only behavior).

## Follow-Up

- `#30`: add retry/backoff for transient network failures (online mode only).
- Consider a compiled-dependency offline/frozen matrix extension (e.g. `fmt` fixture) if cache ABI/build-artifact regressions become a recurring source of bugs.
