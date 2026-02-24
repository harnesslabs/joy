---
date: 2026-02-24
commit: pending
tags:
  - lockfile
  - ux
  - errors
  - json
  - tests
related_components:
  - /Users/autoparallel/Code/joy/src/commands/build.rs
  - /Users/autoparallel/Code/joy/tests/lockfile_behavior.rs
  - /Users/autoparallel/Code/joy/tests/add_command.rs
  - https://github.com/harnesslabs/joy/issues/26
---

# Lockfile Error Guidance and Next-Step Messaging Polish

## Why

Phase 7 added new lockfile error codes (`lockfile_incomplete`, `lockfile_mismatch`) and stricter `--locked` validation, but the user-facing messages still varied in style and did not consistently include an explicit remediation command. This pass standardizes lockfile error guidance so both human mode and JSON mode always point to `--update-lock` with a concrete example command.

## Blockers & Solutions

- Blocker: Lockfile error strings were duplicated across manifest-hash stale checks, missing lockfile checks, and package metadata validation paths.
- Solution: Added small helpers in `src/commands/build.rs` to centralize lockfile refresh guidance text and example command generation, then reused them across `lockfile_missing`, `lockfile_stale`, `lockfile_incomplete`, and `lockfile_mismatch`.

- Blocker: Error-code tests mostly asserted codes, not whether messages remained actionable.
- Solution: Extended lockfile-focused integration tests (`tests/lockfile_behavior.rs` and `tests/add_command.rs`) to assert that JSON error messages include `--update-lock` and an explicit example command (`joy build --update-lock`).

## Fallback Plan

If centralized guidance helpers become too rigid when `joy sync` lands, keep the stable error codes and `--update-lock` guidance requirement but move command-specific examples into the command handlers while preserving the same test assertions about actionable messaging.

