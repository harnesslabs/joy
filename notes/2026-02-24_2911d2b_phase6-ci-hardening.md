---
date: 2026-02-24
commit: 2911d2b
tags:
  - phase6
  - ci
  - e2e
  - compiled-deps
related_components:
  - /Users/autoparallel/Code/joy/.github/workflows/check.yaml
---

# Phase 6 CI Hardening for Compiled Dependencies

## Why

Compiled dependency support can silently regress in CI if tests that require `cmake`/`ninja` are skipped due missing tools. Phase 6 includes CI hardening so the new recipe-backed compiled dependency path is exercised intentionally across the primary target OS matrix.

## Blockers & Solutions

- Blocker: The existing `test` job may skip compiled-dependency fixture coverage depending on host tool availability.
- Solution: Added a dedicated `compiled-e2e` matrix job that explicitly installs `cmake` and `ninja` on Linux/macOS/Windows GNU and runs focused compiled-dependency tests (`add_command` compiled fixture and lockfile behavior checks).

- Blocker: Windows GNU CI paths need both MinGW and CMake/Ninja tools present and on `PATH`.
- Solution: Reused the existing MinGW setup pattern and added separate Chocolatey steps for CMake/Ninja to ensure the compiled fixture tests can execute on `x86_64-pc-windows-gnu`.

## Fallback Plan

If CI runtime becomes excessive, keep the dedicated job but narrow the matrix (e.g., run compiled-e2e on Linux + Windows only) or split fast smoke checks from heavier compiled fixture runs. The workflow structure now cleanly supports that tuning.
