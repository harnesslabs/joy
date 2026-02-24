---
date: 2026-02-24
commit: ba47dfc
tags:
  - phase3
  - archive
  - reqwest
  - tar
  - flate2
  - testing
related_components:
  - /Users/autoparallel/Code/joy/src/fetch.rs
  - /Users/autoparallel/Code/joy/Cargo.toml
  - /Users/autoparallel/Code/joy/Cargo.lock
---

# Phase 3 Archive Download and Extraction Backend

## Why

Phase 3 called for both git-based fetching and archive-based fetching. This commit adds the archive path using `reqwest` (blocking client) plus `flate2` and `tar`, which gives `joy` a reusable download-and-extract backend for `.tar.gz` sources and lets later recipe-driven fetch flows choose between git and archive transports.

The `mockito` test is important because it validates archive download and extraction behavior without any live network dependency, matching the roadmap requirement to use mock HTTP for fetch testing.

## Blockers & Solutions

- Blocker: Adding `reqwest`/TLS/mocking crates significantly expanded dependency resolution and compile time.
- Solution: Kept the implementation small and self-contained in `src/fetch.rs`, using a blocking client for Phase 3 to avoid introducing async runtime design decisions before Phase 6.

- Blocker: The archive backend needed reliable tests that do not depend on external tarball URLs.
- Solution: Built a tar.gz fixture in memory and served it from a `mockito` test server, then asserted extracted file paths on disk.

## Fallback Plan

If the blocking `reqwest` client becomes a bottleneck or conflicts with future async fetch orchestration, keep the public `download_and_extract_tar_gz` function semantics but replace its internals with an async implementation plus a small synchronous wrapper (or move it behind a transport trait) when Phase 6 introduces `tokio` parallel fetch execution.
