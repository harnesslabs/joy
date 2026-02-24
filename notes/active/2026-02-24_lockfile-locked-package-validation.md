---
date: 2026-02-24
commit: 4efd077
tags:
  - lockfile
  - locked
  - reproducibility
  - validation
  - tests
related_components:
  - /Users/autoparallel/Code/joy/src/commands/build.rs
  - /Users/autoparallel/Code/joy/tests/add_command.rs
  - /Users/autoparallel/Code/joy/tests/lockfile_behavior.rs
  - https://github.com/harnesslabs/joy/issues/23
---

# Strict `--locked` Validation for Lockfile Package Metadata

## Why

After populating `joy.lock.packages[]`, `--locked` still only checked the manifest hash. That left a reproducibility hole: users could keep a matching manifest hash while manually deleting or mutating package metadata, and `joy build --locked` would continue. Phase 7 needs `--locked` to reject incomplete or mismatched lockfile package records with stable error codes.

## Blockers & Solutions

- Blocker: The existing lockfile plan only carried a `write_after_build` boolean, so the later build pipeline had no access to the loaded lockfile for strict validation.
- Solution: Extended `LockfilePlan` to retain the loaded lockfile in `--locked` mode and validated package metadata against the expected lock packages after dependency preparation but before final build execution.

- Blocker: Old minimal lockfiles (manifest hash only, no `packages[]`) need a clear distinction from generic mismatches so the user gets the right remediation step.
- Solution: Added `lockfile_incomplete` for missing package metadata when dependencies exist, and `lockfile_mismatch` for populated-but-divergent metadata, both with `--update-lock` guidance.

- Blocker: Lockfile package comparisons should be order-insensitive to avoid false negatives if file ordering is altered manually.
- Solution: Added canonical sorting for `LockedPackage` lists before equality comparison, using package ID + requested revision.

- Blocker: There was no integration coverage for strict `--locked` package validation failures.
- Solution: Added a fixture-driven test that builds a header-only dependency lockfile, then intentionally corrupts `packages[]` to assert both `lockfile_incomplete` and `lockfile_mismatch` error codes under `joy build --locked`.

## Fallback Plan

If strict package validation blocks legitimate workflows unexpectedly, keep `lockfile_incomplete` validation for empty `packages[]` (which is the main reproducibility gap) and temporarily relax full structural equality checks while gathering concrete mismatch cases in issue `#23`.
