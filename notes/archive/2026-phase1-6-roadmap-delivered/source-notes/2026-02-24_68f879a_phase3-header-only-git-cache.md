---
date: 2026-02-24
commit: 68f879a
tags:
  - phase3
  - cache
  - fetch
  - headers
  - git
related_components:
  - /Users/autoparallel/Code/joy/src/global_cache.rs
  - /Users/autoparallel/Code/joy/src/fetch.rs
  - /Users/autoparallel/Code/joy/src/linking.rs
  - /Users/autoparallel/Code/joy/src/commands/add.rs
  - /Users/autoparallel/Code/joy/tests/add_command.rs
---

# Phase 3 Header-Only Git Cache MVP

## Why

This milestone delivers the key Phase 3 outcome: `joy add <owner/repo>` now fetches a GitHub-style package, caches it globally, detects a header root (`include`/`single_include`), and installs headers into the project-local `.joy/include/deps/<slug>` path. That unblocks real header-only package consumption in later build phases.

I also introduced a dedicated `PackageId` type so cache/fetch/link code can share package parsing and slug generation instead of duplicating string parsing logic in command handlers.

## Blockers & Solutions

- Blocker: Existing `add` integration tests assumed `joy add` was purely local and would not touch the network.
- Solution: Added a local git-remote test harness using `JOY_GITHUB_BASE` and `JOY_HOME`, so tests exercise the full cache/fetch/link path without live network dependency.

- Blocker: Building git command arguments with temporary path string borrows caused lifetime issues during compilation.
- Solution: Added owned-argument git helpers in `src/fetch.rs` (and owned helper usage in tests) so dynamic paths can be passed safely to `std::process::Command`.

- Blocker: Clippy flagged `needless_return` in the symlink/copy fallback path.
- Solution: Simplified return expressions in `src/linking.rs` and reran `just lint`.

## Fallback Plan

If the `git` CLI-based fetcher proves too fragile across platforms/CI environments, keep the current cache path and `FetchResult` interface but replace the internals with `git2` behind `src/fetch.rs`. The command integration in `src/commands/add.rs` and the local-remote tests can remain mostly unchanged because they only depend on fetch outputs (resolved commit, cache source dir) rather than the underlying transport implementation.
