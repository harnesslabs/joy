---
date: 2026-02-24
commit: pending
tags:
  - phase-13
  - ux
  - machine-interface
  - diagnostics
  - output
related_components:
  - /Users/autoparallel/Code/joy/src/cli.rs
  - /Users/autoparallel/Code/joy/src/commands/mod.rs
  - /Users/autoparallel/Code/joy/src/commands/doctor.rs
  - /Users/autoparallel/Code/joy/src/commands/add.rs
  - /Users/autoparallel/Code/joy/src/commands/build.rs
  - /Users/autoparallel/Code/joy/src/commands/run.rs
  - /Users/autoparallel/Code/joy/src/commands/sync.rs
  - /Users/autoparallel/Code/joy/src/commands/update.rs
  - /Users/autoparallel/Code/joy/src/fetch.rs
  - /Users/autoparallel/Code/joy/src/output.rs
  - /Users/autoparallel/Code/joy/docs/machine-interface.md
  - /Users/autoparallel/Code/joy/docs/error-codes.md
  - /Users/autoparallel/Code/joy/tests/smoke.rs
  - https://github.com/harnesslabs/joy/issues/51
  - https://github.com/harnesslabs/joy/issues/52
  - https://github.com/harnesslabs/joy/issues/53
  - https://github.com/harnesslabs/joy/issues/54
---

# Phase 13: UX + Machine Interface Stability

## What Shipped

- Added `joy doctor` (human + JSON) for local environment diagnostics (toolchain, cache, recipes, tool presence).
- Added human-mode progress/status output for long-running dependency and build phases while keeping JSON mode output clean.
- Centralized success envelope shaping in `src/output.rs` and added typed `CommandOutput::from_data(...)` helper for command payloads.
- Converted selected commands (`new`, `init`, `recipe-check`) to typed response payloads.
- Documented a machine-readable error-code catalog and expanded machine interface docs with `doctor` payload details.

## Diagnostics Coverage

`joy doctor --json` now reports structured checks for:

- environment (`PATH`, `JOY_HOME`, cwd)
- common tools (`git`, `cmake`, `ninja`, compilers)
- toolchain discovery result
- global cache resolve/layout health
- bundled recipe store load/recipe count

## Output / UX Notes

- Progress messages are emitted only in human mode (stderr) and are disabled in `--json` mode.
- JSON success envelope shape remains additive and has a dedicated stability test.
- `doctor` is success-oriented; individual subsystem issues are surfaced in structured `data` rather than forcing command failure.
