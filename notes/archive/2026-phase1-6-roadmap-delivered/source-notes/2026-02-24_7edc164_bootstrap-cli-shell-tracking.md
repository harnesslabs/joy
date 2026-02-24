---
date: 2026-02-24
commit: 7edc164
tags:
  - bootstrap
  - cli
  - tracking
  - phase0
  - phase1
related_components:
  - /Users/autoparallel/Code/joy/justfile
  - /Users/autoparallel/Code/joy/src/cli.rs
  - /Users/autoparallel/Code/joy/src/output.rs
  - /Users/autoparallel/Code/joy/notes/roadmap.md
---

# Bootstrap CLI Shell and Tracking

## Why

The first commit establishes the operating system for future work: a stable `just` workflow, a canonical `notes/roadmap.md` tracker, and a testable crate layout (`lib.rs` + thin `main.rs`). I also implemented the CLI shape and JSON envelope early because every later phase depends on consistent command dispatch and machine-readable errors.

## Blockers & Solutions

- Blocker: Clap derive attribute `long_alias` failed to compile for the resolved clap version.
- Solution: Switched to `alias = "machine"` for the `--json` flag alias, preserving `--machine` behavior.

- Blocker: `run` passthrough args test panicked because `trailing_var_arg` and `last` were both enabled.
- Solution: Removed the subcommand-level `trailing_var_arg` and kept `#[arg(last = true)]`, which correctly supports `joy run -- ...`.

- Blocker: `assert_cmd::Command::cargo_bin` emitted a deprecation warning that would fail `clippy -D warnings`.
- Solution: Migrated tests to `assert_cmd::cargo::cargo_bin_cmd!`.

## Fallback Plan

If the CLI shell architecture becomes too rigid, revert to the commit before `7edc164` and reintroduce only the crate split + `justfile` first, then layer CLI parsing in a separate step. The command dispatch and JSON output are isolated in `src/commands` and `src/output.rs`, so they can be replaced without disturbing tracker/tests structure.
