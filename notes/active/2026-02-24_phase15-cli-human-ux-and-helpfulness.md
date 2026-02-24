---
date: 2026-02-24
commit: pending
tags:
  - phase15
  - ux
  - output
  - doctor
  - help
  - tests
  - docs
related_components:
  - /Users/autoparallel/Code/joy/src/output.rs
  - /Users/autoparallel/Code/joy/src/cli.rs
  - /Users/autoparallel/Code/joy/src/commands/doctor.rs
  - /Users/autoparallel/Code/joy/src/commands/build.rs
  - /Users/autoparallel/Code/joy/src/fetch.rs
  - /Users/autoparallel/Code/joy/tests/smoke.rs
  - /Users/autoparallel/Code/joy/tests/build_run.rs
  - /Users/autoparallel/Code/joy/docs/machine-interface.md
  - /Users/autoparallel/Code/joy/CONTRIBUTING.md
  - https://github.com/harnesslabs/joy/issues/61
  - https://github.com/harnesslabs/joy/issues/65
  - https://github.com/harnesslabs/joy/issues/66
  - https://github.com/harnesslabs/joy/issues/67
  - https://github.com/harnesslabs/joy/issues/68
  - https://github.com/harnesslabs/joy/issues/69
---

# Phase 15: CLI Human UX and Helpfulness

## Why

Phase 13 added machine-interface stability and some progress output, but human-mode UX was still mostly ad-hoc strings with inconsistent progress/status rendering. This phase front-loads human-mode polish so future work (workspaces, semver, registry) can reuse a better output foundation without risking JSON compatibility.

## What Shipped

- Added shared human output helpers in `/Users/autoparallel/Code/joy/src/output.rs`:
  - `HumanMessageBuilder`
  - centralized progress emitters (`progress_stage`, `progress_detail`, `progress_detail_tty`)
  - human error hints for common failure codes (manifest/toolchain/offline/lockfile cases)
- Migrated command/status output to shared helpers across:
  - `new`, `init`, `recipe-check`, `remove`, `add`, `update`, `tree`, `build`, `sync`, `run`
  - fetch progress emission in `/Users/autoparallel/Code/joy/src/fetch.rs`
- Reworked `joy doctor` human output to use sectioned summary/tool lists and actionable warnings/hints.
- Improved top-level and subcommand `--help` output with examples and workflow guidance.
- Added human-output regression coverage:
  - help examples
  - doctor/recipe-check structured human output
  - human build progress prefix assertions
  - JSON mode no-progress-prefix assertion for `build --json`
- Documented human-vs-machine output stability expectations in:
  - `/Users/autoparallel/Code/joy/docs/machine-interface.md`
  - `/Users/autoparallel/Code/joy/CONTRIBUTING.md`

## Validation

- `just lint`
- `just test`

## Blockers & Solutions

- Blocker: Human-mode output is intentionally flexible, but Phase 15 still needed regression coverage without brittle snapshots.
- Solution: Added selective assertions for stable structural cues (section names, key phrases, progress prefixes) while keeping exact-string freezing limited.

- Blocker: Progress messages were emitted from multiple layers (commands and fetch internals), risking inconsistent formatting.
- Solution: Centralized progress emission in `/Users/autoparallel/Code/joy/src/output.rs` and routed command/fetch paths through shared helpers.

- Blocker: Improving error helpfulness must not break machine consumers.
- Solution: Kept JSON envelopes/payloads untouched and added human-only hints inside `print_error(...)`.

## Fallback Plan

If any human-mode formatting change proves too noisy, keep the centralized helpers but narrow adoption to the most user-visible commands (`doctor`, `build`, `sync`, `add`) and revert specific command message changes. JSON envelope/payload stability remains preserved independently of the human renderer.

