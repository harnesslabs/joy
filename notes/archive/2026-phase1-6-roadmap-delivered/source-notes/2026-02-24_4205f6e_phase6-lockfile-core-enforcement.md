---
date: 2026-02-24
commit: 4205f6e
tags:
  - phase6
  - lockfile
  - build
  - run
related_components:
  - /Users/autoparallel/Code/joy/src/lockfile.rs
  - /Users/autoparallel/Code/joy/src/commands/build.rs
  - /Users/autoparallel/Code/joy/src/commands/run.rs
  - /Users/autoparallel/Code/joy/tests/lockfile_behavior.rs
---

# Phase 6 Lockfile Core and Build/Run Enforcement

## Why

After Phase 5 delivered compiled dependency builds, Phase 6 begins by making the build graph reproducible and drift-aware. A lockfile schema plus build-time enforcement is the minimum foundation needed to detect manifest drift and support predictable CI/build workflows.

## Blockers & Solutions

- Blocker: `build` and `run` already shared a common build path but did not thread lock-related CLI flags into that path.
- Solution: Introduced `BuildOptions` and passed `--locked` / `--update-lock` through `run -> build_project`, so lockfile logic lives in one place.

- Blocker: Lockfile enforcement needed to happen early enough to fail fast on stale or missing lockfiles without invoking toolchain discovery or compilation.
- Solution: Added manifest-hash computation and lockfile plan evaluation before toolchain probing/build execution in `src/commands/build.rs`.

- Blocker: This behavior needed integration coverage, especially stale-lock handling and `--locked` semantics.
- Solution: Added `tests/lockfile_behavior.rs` covering lockfile creation, stale lockfile rejection, `--update-lock` refresh, and `--locked` success/failure paths.

## Fallback Plan

If lockfile package entries need stricter validation than the current manifest-hash-based enforcement (e.g., resolved package set drift, ABI hash mismatches), extend `src/lockfile.rs` population and validation in `build_project()` while keeping the existing early lockfile-plan check as the fast path for obvious drift.
