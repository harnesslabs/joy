---
date: 2026-02-24
commit: a1d2cc3
tags:
  - phase5
  - abi
  - cache
  - hashing
related_components:
  - /Users/autoparallel/Code/joy/src/abi.rs
  - /Users/autoparallel/Code/joy/src/global_cache.rs
---

# Phase 5 ABI Hashing and Build Cache Layout

## Why

Compiled dependency reuse depends on a stable cache key that reflects ABI-relevant inputs and a deterministic cache directory structure where build artifacts can be stored and later linked. This milestone adds both pieces so the upcoming CMake adapter can target a concrete cache contract instead of inventing one ad hoc.

## Blockers & Solutions

- Blocker: ABI inputs need deterministic serialization for hashing across platforms and test runs.
- Solution: Modeled ABI inputs as a typed `AbiHashInput` serde struct (with ordered `BTreeMap` env vars) and hashed the JSON bytes with SHA-256 to produce a stable 64-hex cache key.

- Blocker: The compiled build cache layout needed to coexist with existing source/git/archive caches without accidental overlap.
- Solution: Extended `GlobalCache` with a dedicated `BuildCacheLayout` under `~/.joy/cache/builds/<abi_hash>/` and explicit subdirectories (`work`, `lib`, `bin`, `include`, `state`).

## Fallback Plan

If later phases require additional ABI key inputs (lockfile schema version, recipe registry version, toolchain target ABI tags), extend `AbiHashInput` with new fields and keep serialization-based hashing. Existing cache invalidation will naturally occur via the hash change without altering the cache layout API.
