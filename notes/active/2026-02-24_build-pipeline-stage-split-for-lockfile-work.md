---
date: 2026-02-24
commit: pending
tags:
  - build
  - refactor
  - lockfile
  - cmake
  - resolver
related_components:
  - /Users/autoparallel/Code/joy/src/commands/build.rs
  - https://github.com/harnesslabs/joy/issues/24
---

# Build Pipeline Stage Split for Lockfile Work

## Why

`prepare_compiled_dependencies(...)` had accumulated resolver, prefetch, compiled build/cache, install staging, and lockfile metadata assembly logic in one function. That made Phase 7 lockfile work harder to extend and harder to test in isolation. Splitting it into explicit stages reduces the surface area of `joy build` and clarifies where future diagnostics/validation should live.

## Blockers & Solutions

- Blocker: The function mixed multiple phases with shared local state (`recipes`, resolved order, prefetched sources, compiled metadata), making refactoring risky.
- Solution: Introduced explicit internal stage structs (`ResolvedDependencyStage`, `PrefetchedDependencyStage`, `CompiledDependencyBuildStage`) and small orchestrator helpers, while preserving the existing output contract (`NativeLinkInputs`).

- Blocker: The compiled build loop depends on both resolver graph data and prefetched source checkouts; naively moving pieces can create ownership/partial-move issues.
- Solution: Separated `prefetch` results into `all_by_key` (lockfile assembly) and `build_by_key` (compiled build consumption), and used `build_order_ids` + `resolved.package(...)` to avoid lifetime-heavy borrowed build-order vectors.

- Blocker: This refactor should not change user-visible behavior while Phase 7 is focused on lockfile fidelity.
- Solution: Kept all command outputs and lockfile semantics unchanged, and validated with full `just lint` + `just test`.

## Fallback Plan

If later stage-level changes introduce regressions, collapse only the new stage helper boundaries back into `prepare_compiled_dependencies(...)` while preserving the already-landed lockfile metadata assembly helpers (`assemble_lockfile_packages`, strict `--locked` validation) so Phase 7 work can continue.

