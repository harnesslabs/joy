---
date: 2026-02-24
commit: 14aabda
tags:
  - phase4
  - toolchain
  - ninja
  - build
  - run
related_components:
  - /Users/autoparallel/Code/joy/src/toolchain.rs
  - /Users/autoparallel/Code/joy/src/ninja.rs
  - /Users/autoparallel/Code/joy/src/commands/build.rs
  - /Users/autoparallel/Code/joy/src/commands/run.rs
  - /Users/autoparallel/Code/joy/tests/build_run.rs
---

# Phase 4 Local Build Engine (`joy build` / `joy run`)

## Why

Phase 4 turns `joy` from a package-aware scaffold tool into a usable C++ workflow by compiling and running local projects. Toolchain discovery and Ninja generation also create the foundation that Phase 5 will reuse for compiling third-party dependencies.

## Blockers & Solutions

- Blocker: Generated `build.ninja` rule bindings were invalid because Rust string line continuation trimmed the intended leading spaces required by Ninja.
- Solution: Rewrote the renderer to build explicit line arrays and join with `\n`, preserving exact indentation for `command`, `depfile`, and `deps` bindings.

- Blocker: `joy build --json` emitted mixed output because Ninja wrote directly to stdout/stderr, breaking machine-mode JSON parsing in tests.
- Solution: Switched Ninja invocation to `Command::output()` and captured stdout/stderr into the build result / error path so JSON mode remains clean and deterministic.

- Blocker: Older integration tests still asserted Phase 1 stub behavior for `build` and `run`.
- Solution: Updated tests to reflect current behavior (`manifest_not_found` in empty dirs) and added dedicated Phase 4 `build`/`run` integration coverage.

## Fallback Plan

If the single-file Ninja generator becomes too limiting while implementing Phase 5 compiled dependencies, keep `src/ninja.rs` as the application build path and introduce a separate dependency-build generator/adapter layer. The current `toolchain` discovery and `build_project()` API are isolated enough to be reused by both paths.
