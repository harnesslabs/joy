---
date: 2026-02-24
commit: ed5f6b2
tags:
  - phase5
  - build
  - compiled-deps
  - cmake
  - linking
related_components:
  - /Users/autoparallel/Code/joy/src/commands/build.rs
  - /Users/autoparallel/Code/joy/src/cmake.rs
  - /Users/autoparallel/Code/joy/src/resolver.rs
  - /Users/autoparallel/Code/joy/src/abi.rs
  - /Users/autoparallel/Code/joy/src/linking.rs
  - /Users/autoparallel/Code/joy/tests/add_command.rs
---

# Phase 5 Compiled Dependency Build Integration

## Why

The earlier Phase 5 slices created the pieces (recipes, resolver DAG, ABI cache keys, CMake adapter, and compiled library installer), but `joy build` still needed to orchestrate them. This milestone wires those parts into the actual build path so recipe-backed compiled dependencies are resolved, built, cached, staged into `.joy/lib`, and linked into the application binary.

## Blockers & Solutions

- Blocker: Command integration required joining multiple new subsystems (resolver, cache, ABI hashing, CMake adapter, linker staging) while preserving Phase 4 behavior for projects with no dependencies.
- Solution: Added a focused `prepare_compiled_dependencies(...)` path in `src/commands/build.rs` that no-ops when there are no deps and otherwise computes native link inputs (`link_dirs`, `link_libs`) for Ninja generation.

- Blocker: This integration needed a reliable test without network access or dependence on real third-party repos.
- Solution: Added a local Git “GitHub mirror” fixture for `fmtlib/fmt` (simple CMake static library + header) and an end-to-end test that runs `joy add`, `joy build`, and `joy run` using the curated `fmt` recipe.

- Blocker: Ninja generation previously had no link flag plumbing for compiled libs.
- Solution: Extended `NinjaBuildSpec`/renderer support for `ldflags` (`-L...`, `-l...`) and passed the compiled dependency linker inputs from `build_project()`.

## Fallback Plan

If wider recipe coverage reveals linker/runtime edge cases (shared libraries, Windows import libs, multiple outputs per target), keep the current command integration structure and move platform-specific selection logic into recipe metadata and `linking.rs` helpers. The orchestration boundary in `prepare_compiled_dependencies(...)` is a stable insertion point for that refinement.
