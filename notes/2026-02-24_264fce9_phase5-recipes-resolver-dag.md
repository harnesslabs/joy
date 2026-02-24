---
date: 2026-02-24
commit: 264fce9
tags:
  - phase5
  - recipes
  - resolver
  - dag
  - petgraph
related_components:
  - /Users/autoparallel/Code/joy/src/recipes.rs
  - /Users/autoparallel/Code/joy/src/resolver.rs
  - /Users/autoparallel/Code/joy/recipes/index.toml
  - /Users/autoparallel/Code/joy/recipes/packages/fmt.toml
  - /Users/autoparallel/Code/joy/recipes/packages/nlohmann_json.toml
  - /Users/autoparallel/Code/joy/recipes/packages/zlib.toml
---

# Phase 5 Recipes, Resolver, and DAG Foundation

## Why

Compiled dependency support needs two foundations before any third-party build execution can work: a curated metadata source (recipes) and a deterministic dependency graph resolver. This milestone establishes both, including exact-ref resolution hooks and graph validation (cycle/conflict detection) that later ABI caching and CMake builds will depend on.

## Blockers & Solutions

- Blocker: Recipe dependency syntax needs to support both simple package IDs and entries with explicit revs while still remaining close to the roadmap example schema.
- Solution: Implemented an untagged `RecipeDependency` enum that accepts either a bare string or `{ id, rev }`, while resolver enforcement requires a concrete `rev` for transitive dependencies.

- Blocker: Clippy flagged `ResolverError` as too large because the conflict variant stored many inline `String` fields.
- Solution: Moved conflict details into a boxed `VersionConflictError` payload, preserving rich diagnostics without suppressing lints.

- Blocker: Resolver tests needed deterministic commit resolution without live network access.
- Solution: Added `resolve_manifest_with(...)` with an injected commit-resolution closure; production can use the fetch-backed wrapper while tests supply a local deterministic mapping.

## Fallback Plan

If exact-ref resolution grows more complex than a closure-based interface can comfortably express (e.g., lockfile reuse, batched fetching, remote metadata), keep `resolve_manifest_with` for tests and introduce a dedicated resolver context object that encapsulates fetch/cache state. The graph model and recipe parsing can remain unchanged.
