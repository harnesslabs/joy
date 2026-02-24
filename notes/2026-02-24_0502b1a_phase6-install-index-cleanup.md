---
date: 2026-02-24
commit: 0502b1a
tags:
  - phase6
  - cleanup
  - cache
  - state
related_components:
  - /Users/autoparallel/Code/joy/src/install_index.rs
  - /Users/autoparallel/Code/joy/src/commands/add.rs
  - /Users/autoparallel/Code/joy/src/commands/build.rs
---

# Phase 6 Install Index and Orphan Cleanup

## Why

With compiled dependencies and staged library artifacts in `.joy/lib`, the local project cache can accumulate stale files as dependencies change. Phase 6 requires safe cleanup that only removes files `joy` created, without touching unknown user files in the project workspace.

## Blockers & Solutions

- Blocker: Cleanup needed a reliable ownership model so `joy` would not delete user-created files under `.joy`.
- Solution: Added `src/install_index.rs` with `.joy/state/install-index.json` to track header-link directories and staged library files created by `joy`.

- Blocker: Cleanup needed to run during normal workflows and not just in a manual maintenance command.
- Solution: `joy add` now records installed header paths, and `joy build` computes desired tracked entries from the manifest + compiled dependency staging results, removes tracked orphans, and rewrites the install index.

- Blocker: Cleanup logic is easy to get wrong across files vs directories vs missing paths.
- Solution: Added unit coverage for install-index roundtrip and orphan cleanup behavior, including protection of an unknown user file in `.joy/lib`.

## Fallback Plan

If later phases need more granular cleanup (shared runtime files, generated headers, per-package ownership), extend the install index schema with package-scoped entries and artifact categories while keeping the same “tracked-only deletion” rule.
