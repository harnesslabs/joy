---
date: 2026-02-25
commit: 8524cae
tags:
  - phase38
  - phase76
  - resolver
  - nested-deps
  - package-manifest
  - registry
  - registry-v2
  - lockfile
related_components:
  - /Users/autoparallel/Code/joy/src/resolver.rs
  - /Users/autoparallel/Code/joy/src/manifest.rs
  - /Users/autoparallel/Code/joy/src/registry.rs
  - /Users/autoparallel/Code/joy/src/lockfile.rs
  - /Users/autoparallel/Code/joy/src/commands/build.rs
  - /Users/autoparallel/Code/joy/src/commands/tree.rs
  - /Users/autoparallel/Code/joy/src/commands/why.rs
  - /Users/autoparallel/Code/joy/tests/add_command.rs
---

# Phase 38-76: Nested Package Manifests + Registry v2 Progress Tranche

## Why

The first Phase 38-76 tranche fixed graph-driven materialization, compile databases, and `joy add` sync-lite, but the resolver still lacked the core nested dependency path for reusable library metadata (`joy.toml`) and registry-backed transitive edges.

This tranche focused on making nested dependency resolution real in both of the expected day-one flows:

- package manifest recursion from fetched source checkouts
- hybrid nested deps where a package manifest can declare `source = "registry"`

It also started the registry v2 metadata path needed for offline/fallback recursive expansion and provenance verification.

## What Landed

### Resolver / Manifest Recursion

- `src/manifest.rs` now supports reusable package manifests via `ManifestDocument::Package`
- `src/resolver.rs` recursively expands package `joy.toml` dependencies for fetched packages when no curated recipe exists
- recipe dependency edges remain authoritative when a recipe is present (compatibility-first precedence)
- resolver callback was refactored to preserve transitive `DependencySource` so nested registry deps no longer silently default to GitHub
- lazy registry index loading was added so nested registry dependencies do not panic when the root manifest has only GitHub deps

### Lockfile Provenance

- `src/lockfile.rs` gained additive per-package fields:
  - `metadata_source`
  - `package_manifest_digest`
  - `declared_deps_source`
- `src/commands/build.rs` now populates these fields during lockfile assembly using best-effort source inspection:
  - `recipe`
  - `package_manifest`
  - `none`

### Registry v2 Metadata (Initial Cut)

- `src/registry.rs` now accepts registry index `version = 1` and `version = 2`
- v2 release entries may include an embedded manifest summary (`digest`, `kind`, `headers_include_roots`, dependency list)
- embedded summary dependency requirement shapes are validated (`rev` xor `version`, registry deps require `version`)
- `ResolvedRegistryRelease` now exposes optional parsed manifest summary

### Registry v2 Resolver Fallback + Verification

- For registry releases, resolver now:
  - prefers fetched package `joy.toml` package metadata when present
  - falls back to registry v2 embedded manifest summary when source checkout has no package manifest
  - detects and errors on digest mismatch between registry v2 summary and fetched package `joy.toml`
- `tree`, `why`, and `build/sync` resolver error mapping now surface stable code `package_metadata_mismatch`

## Tests Added / Updated

- resolver unit tests:
  - package-manifest recursive edge expansion
  - recipe precedence over package-manifest edges
  - registry manifest digest mismatch detection
- registry unit tests:
  - v2 embedded manifest summary parsing
  - v2 summary dependency validation errors
- add/cli integration tests:
  - nested package-manifest recursion (`harnesslabs/igneous -> xsimd/xsimd`)
  - nested registry dependency declared in package manifest
  - registry v2 summary fallback without source `joy.toml`
  - registry v2 digest mismatch stable error code (`package_metadata_mismatch`)

## Blockers & Solutions

- Blocker: nested package-manifest registry dependencies panicked because registry store preloading only checked root manifest dependencies.
- Solution: make registry store loading lazy inside `DependencySource::Registry` resolution branch.

- Blocker: resolver originally dropped transitive dependency source information and defaulted non-root deps to GitHub.
- Solution: change resolver selector callback to receive `DependencySource` from the pending dependency queue.

- Blocker: `just lint` failed on clippy after compile DB work (`too_many_arguments`, `if_same_then_else`).
- Solution: refactor compile DB writer args into `CompileCommandsSpec` and simplify `sanitize_header_root_key`.

## Fallback Plan

If registry v2 fallback/verification proves too strict for early adopters:

1. Keep v2 parsing and summary storage enabled.
2. Temporarily disable digest mismatch enforcement in resolver (warning only) while preserving summary fallback.
3. Continue recording lockfile provenance (`metadata_source`, `package_manifest_digest`) so stricter verification can be re-enabled later without data-model churn.
