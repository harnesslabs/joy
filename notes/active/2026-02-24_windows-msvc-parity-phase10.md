---
date: 2026-02-24
commit: pending
tags:
  - phase-10
  - windows
  - msvc
  - ninja
  - cmake
  - ci
related_components:
  - /Users/autoparallel/Code/joy/src/toolchain.rs
  - /Users/autoparallel/Code/joy/src/ninja.rs
  - /Users/autoparallel/Code/joy/src/cmake.rs
  - /Users/autoparallel/Code/joy/src/commands/build.rs
  - /Users/autoparallel/Code/joy/tests/build_run.rs
  - /Users/autoparallel/Code/joy/.github/workflows/check.yaml
  - /Users/autoparallel/Code/joy/justfile
  - /Users/autoparallel/Code/joy/README.md
  - /Users/autoparallel/Code/joy/docs/releasing.md
  - https://github.com/harnesslabs/joy/issues/37
  - https://github.com/harnesslabs/joy/issues/38
  - https://github.com/harnesslabs/joy/issues/39
  - https://github.com/harnesslabs/joy/issues/40
  - https://github.com/harnesslabs/joy/issues/41
---

# Phase 10: Windows MSVC Parity

## What Shipped

- Enabled `cl.exe` as a supported compiler path in toolchain discovery (removed the hard failure for MSVC detection).
- Added MSVC-aware Ninja generation:
  - `/showIncludes` + `deps = msvc`
  - MSVC compile/link flag rendering (`/std:`, `/I`, `/LIBPATH:`)
  - `.lib` name normalization for link inputs
- Wired the discovered compiler kind/path into CMake configure so compiled recipe builds pin the chosen toolchain (`CMAKE_CXX_COMPILER`, and `CMAKE_C_COMPILER` for MSVC).
- Fixed ABI target-triple inference on Windows to distinguish GNU vs MSVC when `TARGET` is not set.
- Expanded CI coverage to include Windows MSVC build/lint/test and compiled-e2e jobs with `ilammy/msvc-dev-cmd` setup.
- Added MSVC-focused unit tests plus Windows CRLF source build/run regression coverage.

## Validation Notes

- Local validation (non-Windows host): `cargo fmt`, `just dist-metadata-check`, `just lint`, `just test`
- Windows MSVC execution is covered by the updated GitHub Actions matrix (`x86_64-pc-windows-msvc`).

## Scope Notes

This phase delivers MSVC build parity for local/CI workflows. The GitHub release workflow remains Windows GNU-targeted for packaged artifacts, which is now documented explicitly in release docs and the README platform matrix.
