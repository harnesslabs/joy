---
date: 2026-02-24
commit: pending
tags:
  - lockfile
  - reproducibility
  - resolver
  - build
  - tests
related_components:
  - /Users/autoparallel/Code/joy/src/commands/build.rs
  - /Users/autoparallel/Code/joy/src/resolver.rs
  - /Users/autoparallel/Code/joy/src/lockfile.rs
  - /Users/autoparallel/Code/joy/tests/add_command.rs
  - https://github.com/harnesslabs/joy/issues/22
---

# Lockfile Package Population From Resolved Graph

## Why

`joy.lock` previously only enforced the manifest hash and always wrote `packages = []`, which meant `--locked` could not evolve into a true reproducibility gate. Phase 7 starts by serializing the actual resolved dependency graph and compiled build metadata into `joy.lock` so lockfiles capture package identity, resolved commits, recipe metadata, dependencies, and compiled ABI/link details.

## Blockers & Solutions

- Blocker: `write_lockfile_if_needed(...)` ran after the build but only received a manifest hash, while the package metadata needed for lockfile entries was computed earlier inside `prepare_compiled_dependencies(...)`.
- Solution: Extended `NativeLinkInputs` with `lockfile_packages` and assembled deterministic `LockedPackage` records from resolver graph + prefetch results + compiled build metadata before the lockfile write step.

- Blocker: Lockfile `deps` should reflect the resolved graph, not ad-hoc recipe parsing at serialization time.
- Solution: Added `ResolvedGraph::dependency_ids(...)` to expose stable, sorted direct dependency IDs using the resolver's dependency -> dependent edge direction.

- Blocker: Header-only packages without explicit recipe metadata still need useful `header_roots` in lockfile entries.
- Solution: Added a small source-tree heuristic (`include`, `single_include`, fallback to discovered header root) to infer lockfile header roots from prefetched source checkouts when no recipe is available.

- Blocker: Existing tests only asserted manifest-hash lockfile behavior, not populated package records.
- Solution: Expanded `tests/add_command.rs` fixture coverage to assert populated lockfile package entries for both a header-only package (`nlohmann/json`) and a compiled recipe-backed package (`fmtlib/fmt`), including ABI hash and link metadata checks.

## Fallback Plan

If populated lockfile entries cause unexpected compatibility issues, keep manifest-hash lockfile enforcement intact and temporarily gate package population behind a focused revert of the `src/commands/build.rs` lockfile assembly changes while preserving the new resolver dependency helper and test scaffolding for future reintroduction.

