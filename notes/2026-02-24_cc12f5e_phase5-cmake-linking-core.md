---
date: 2026-02-24
commit: cc12f5e
tags:
  - phase5
  - cmake
  - linking
  - cache
related_components:
  - /Users/autoparallel/Code/joy/src/cmake.rs
  - /Users/autoparallel/Code/joy/src/linking.rs
  - /Users/autoparallel/Code/joy/src/ninja.rs
---

# Phase 5 CMake Adapter and Compiled Linking Core

## Why

Phase 5 needs an executable path for compiled third-party libraries before command integration can be completed. This milestone adds the reusable primitives: a CMake adapter that builds into the ABI cache and a compiled-library installer that stages artifacts into `.joy/lib` for application linking.

## Blockers & Solutions

- Blocker: Compiled dependency support needs a cacheable build adapter that can detect cache hits and avoid rebuilding every invocation.
- Solution: Added `src/cmake.rs` with a `build_into_cache` flow that configures/builds with `cmake -G Ninja`, copies artifacts into the ABI cache layout, and writes a cache manifest used for subsequent cache-hit detection.

- Blocker: Application linking currently only handled headers and include paths.
- Solution: Extended `src/linking.rs` with compiled library artifact installation (`install_compiled_libraries`) and updated `src/ninja.rs` to support link directories and `-l<name>` flags in generated `ldflags`.

- Blocker: The adapter needed confidence without depending on external packages or network.
- Solution: Added a local CMake fixture test that builds a tiny static library into the cache, plus artifact classification tests and compiled-library install tests.

## Fallback Plan

If command integration reveals platform-specific CMake or artifact naming edge cases, keep `src/cmake.rs` as the shared adapter and add recipe-specific post-processing hooks (artifact selectors, target overrides) without changing the ABI cache layout or linker installer contracts.
