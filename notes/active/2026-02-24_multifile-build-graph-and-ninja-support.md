---
date: 2026-02-24
commit: pending
tags:
  - phase-9
  - build
  - ninja
  - multifile
  - include-dirs
  - tests
related_components:
  - /Users/autoparallel/Code/joy/src/commands/build.rs
  - /Users/autoparallel/Code/joy/src/ninja.rs
  - /Users/autoparallel/Code/joy/tests/build_run.rs
  - https://github.com/harnesslabs/joy/issues/33
  - https://github.com/harnesslabs/joy/issues/34
  - https://github.com/harnesslabs/joy/issues/36
---

# Multi-File Build Pipeline + Ninja Graph Support

## Why

After adding manifest schema support (`project.extra_sources` / `project.include_dirs`), `joy` still compiled exactly one translation unit. Phase 9 required the build pipeline and Ninja generator to consume multiple source files, avoid object-name collisions, and expose the compiled source set in JSON output.

## What Changed

- `src/commands/build.rs`
  - Collects project sources from `project.entry + project.extra_sources` (deduplicated, stable order).
  - Validates extra source paths and user include directories before build generation.
  - Merges user include dirs (`project.include_dirs`) with staged dependency include dirs.
  - Generates deterministic object names for all sources using a sanitized basename + SHA-256 path hash suffix (collision-safe for duplicate basenames).
  - Adds `compiled_sources` to build JSON output (additive field).
- `src/ninja.rs`
  - Reworked `NinjaBuildSpec` from single source/object to `compile_units: Vec<NinjaCompileUnit>`.
  - Emits one compile edge per source and links all generated objects into the final binary.
  - Keeps depfile generation per object compile edge.

## Coverage

- Updated Ninja unit tests for multi-object rendering.
- Added integration E2E coverage proving:
  - `extra_sources` and `include_dirs` are honored
  - duplicate source basenames do not collide in `.joy/build/obj`
  - `build --json` includes both compiled source paths
  - `run` executes the resulting multi-file program successfully

## Follow-Up

`#35` remains for scaffolding/docs polish so newly generated projects can discover the multi-file manifest fields without reading source/tests.
