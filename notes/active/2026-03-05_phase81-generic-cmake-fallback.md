---
date: 2026-03-05
commit: 1dab34d
tags:
  - phase81
  - cmake
  - compiled-dependencies
  - lockfile
related_components:
  - /Users/autoparallel/Code/joy/src/commands/build.rs
  - /Users/autoparallel/Code/joy/src/cmake.rs
  - /Users/autoparallel/Code/joy/src/linking.rs
  - /Users/autoparallel/Code/joy/tests/add_command.rs
---

# Phase 81: Generic Recipe-less CMake Fallback

## Why

`joy` previously required curated recipes for compiled dependency success. That made header-only consumption smooth but blocked many real-world CMake packages unless the recipe index already had explicit metadata. Phase 81 introduces a CMake-first fallback path so common packages can compile without a recipe.

## What Landed

1. Generic CMake candidate detection in dependency materialization:
   - if a dependency has no recipe but fetched source has a top-level `CMakeLists.txt`, `joy` treats it as a compiled candidate.
2. Generic compiled build path in `build_compiled_dependency_stage`:
   - uses default CMake configure/build invocation (no recipe args)
   - infers header roots from source layout
   - computes ABI hash for cache layout stability
3. Generic library inference:
   - scans cache `lib/` outputs and infers link names from `.a`, `.so`, `.dylib`, `.lib`, `.dll` artifacts
   - records inferred `libs` and optional linkage in lock metadata
4. Lockfile integration:
   - package entry is marked non-header-only when generic compiled metadata exists
   - inferred libs are persisted for reproducible downstream linking
5. End-to-end integration test:
   - local git fixture with only `CMakeLists.txt` + source/header
   - `joy add`, `joy build`, and `joy run` verifies compile/link/runtime output

## Validation

- `just lint` (pass)
- `just test` (pass)
- new tests in `/Users/autoparallel/Code/joy/src/commands/build.rs`:
  - generic library filename parsing
  - generic link inference/linkage behavior
- new integration coverage in `/Users/autoparallel/Code/joy/tests/add_command.rs`:
  - `build_and_run_with_local_compiled_generic_cmake_dependency`

## Blockers & Solutions

- Blocker: lockfile assertion in the new integration test indexed an optional field directly (`recipe`) and panicked when absent.
- Solution: switched to optional lookup (`get("recipe")`) so omission semantics remain additive-safe.

- Blocker: one pre-existing outdated-source filter test started failing due shared cache reuse (`JOY_HOME` not isolated) across package fixtures.
- Solution: isolate `JOY_HOME` in that test to keep fixture behavior deterministic.

## Fallback Plan

If generic CMake fallback causes unacceptable instability:

1. keep detection code but gate execution behind a feature flag or command opt-in.
2. retain lockfile/schema behavior and inference helpers for future re-enable.
3. route unresolved compiled packages back to recipe-only path with explicit guidance.
