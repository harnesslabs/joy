---
date: 2026-02-24
commit: 038afdf
tags:
  - phase6
  - parallelism
  - tokio
  - fetch
related_components:
  - /Users/autoparallel/Code/joy/src/fetch.rs
  - /Users/autoparallel/Code/joy/src/commands/build.rs
  - /Users/autoparallel/Code/joy/Cargo.toml
---

# Phase 6 Parallel Dependency Prefetch

## Why

By Phase 5, `joy build` could resolve and build compiled dependencies, but source checkout materialization still happened sequentially. Phase 6 calls for parallel network/cache work so multi-dependency builds spend less time waiting on serialized fetch operations.

## Blockers & Solutions

- Blocker: Existing fetch APIs are blocking (git subprocess + blocking HTTP paths), while Phase 6 calls for bounded concurrency.
- Solution: Added a tokio-backed parallel prefetch wrapper in `src/fetch.rs` that uses a bounded semaphore plus `spawn_blocking` to run existing blocking fetch operations concurrently without rewriting the fetch implementation to async.

- Blocker: Dependency builds themselves must remain ordered by the resolver DAG.
- Solution: Integrated parallelism only as a prefetch/warm-cache stage in `prepare_compiled_dependencies(...)`; the subsequent compiled build/link loop remains sequential and DAG-ordered.

## Fallback Plan

If future profiling shows runtime creation overhead or insufficient throughput, keep the current `tokio` wrapper API and move to a longer-lived runtime/context in the build engine. The bounded prefetch interface can stay stable while the execution backend changes.
