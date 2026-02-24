---
date: 2026-02-24
commit: 9d7a05d
tags:
  - phase3
  - linking
  - tests
  - fallback
related_components:
  - /Users/autoparallel/Code/joy/src/linking.rs
---

# Phase 3 Linker Copy-Fallback Coverage

## Why

Phase 3 required confidence in cross-platform header installation behavior, especially when symlink creation fails (common on Windows without elevated permissions or developer mode). This commit adds deterministic test coverage for the copy path in `src/linking.rs` so fallback behavior is exercised even on systems where symlink creation usually succeeds.

## Blockers & Solutions

- Blocker: The symlink fallback path is hard to test deterministically because symlink creation may succeed on the host running the test.
- Solution: Added an internal `LinkMode` used by tests to force copy-mode installation, while keeping the public `install_headers` API unchanged (`Auto` mode).

- Blocker: Clippy flagged the test-only `CopyOnly` enum variant as dead code in non-test builds.
- Solution: Added a targeted `#[cfg_attr(not(test), allow(dead_code))]` on the enum so production builds remain warning-clean under `-D warnings`.

## Fallback Plan

If the internal `LinkMode` testing hook starts leaking into production logic, replace it with a smaller private helper dedicated to copying and test that helper directly. The public `install_headers` entrypoint should remain in automatic mode for normal operation.
