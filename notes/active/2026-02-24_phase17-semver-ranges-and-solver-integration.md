---
date: 2026-02-24
commit: 05b03b6
tags:
  - phase17
  - semver
  - resolver
  - fetch
  - lockfile
  - cli
  - docs
  - tests
related_components:
  - /Users/autoparallel/Code/joy/src/manifest.rs
  - /Users/autoparallel/Code/joy/src/fetch.rs
  - /Users/autoparallel/Code/joy/src/resolver.rs
  - /Users/autoparallel/Code/joy/src/lockfile.rs
  - /Users/autoparallel/Code/joy/src/commands/add.rs
  - /Users/autoparallel/Code/joy/src/commands/update.rs
  - /Users/autoparallel/Code/joy/src/commands/tree.rs
  - /Users/autoparallel/Code/joy/src/commands/build.rs
  - /Users/autoparallel/Code/joy/src/output.rs
  - /Users/autoparallel/Code/joy/tests/add_command.rs
  - /Users/autoparallel/Code/joy/docs/machine-interface.md
  - /Users/autoparallel/Code/joy/docs/error-codes.md
  - /Users/autoparallel/Code/joy/README.md
  - https://github.com/harnesslabs/joy/issues/63
  - https://github.com/harnesslabs/joy/issues/75
  - https://github.com/harnesslabs/joy/issues/76
  - https://github.com/harnesslabs/joy/issues/77
  - https://github.com/harnesslabs/joy/issues/78
  - https://github.com/harnesslabs/joy/issues/79
---

# Phase 17: Semver Ranges and Solver Integration

## Why

Phase 16 made larger project layouts viable, but direct dependency versioning was still exact-ref only. That made upgrades awkward (`joy update` needed explicit tags/SHAs) and prevented a lockfile from recording the common split of "requested range" vs "resolved selection". Phase 17 adds the first semver workflow while preserving deterministic exact-ref behavior and keeping transitive recipe metadata unchanged.

## What Shipped

- Manifest dependency model in `/Users/autoparallel/Code/joy/src/manifest.rs` now supports either:
  - `rev = "..."` (exact ref) or
  - `version = "..."` (semver range)
- Validation enforces mutual exclusivity (`rev` xor `version`) and exposes a borrowed `dependency_requirement(...)` helper used by the resolver.
- Git-tag semver resolution in `/Users/autoparallel/Code/joy/src/fetch.rs`:
  - `fetch_github_semver[_with_cache]`
  - semver range parsing via `semver`
  - highest matching tag selection from cached mirror tags (plain or `v`-prefixed)
  - additive fetch metadata: `requested_requirement`, `resolved_version`
- Resolver seam in `/Users/autoparallel/Code/joy/src/resolver.rs`:
  - direct dependency requests now flow through an internal request enum (exact rev vs semver range)
  - existing exact-mode test hook (`resolve_manifest_with`) is preserved
  - resolved graph nodes now record additive semver metadata (`requested_requirement`, `resolved_version`)
- Lockfile schema in `/Users/autoparallel/Code/joy/src/lockfile.rs` records additive semver metadata per package:
  - `requested_requirement`
  - `resolved_version`
- Command UX and payload updates:
  - `joy add --version <range>`
  - `joy update <pkg> --version <range>`
  - `joy update <pkg>` refreshes semver-managed deps using the stored range
  - `joy tree` shows semver metadata in human and JSON output
  - `joy build` / `joy sync` lockfile assembly preserves semver metadata
- Stable machine error-code mapping for semver failures:
  - `invalid_version_requirement`
  - `version_not_found`
  - human-mode hints added in `/Users/autoparallel/Code/joy/src/output.rs`

## Validation

- `just lint`
- `just test`

## Coverage Added

- `/Users/autoparallel/Code/joy/src/resolver.rs` unit test for direct semver dependency metadata recording via the new resolver seam.
- `/Users/autoparallel/Code/joy/src/fetch.rs` unit test for semver tag parsing (`v`-prefixed and plain tags).
- `/Users/autoparallel/Code/joy/tests/add_command.rs` integration coverage for:
  - `joy add --version` selecting the highest matching tag
  - `joy update` refreshing a semver-managed dependency when a newer matching tag appears
  - `joy tree` JSON semver metadata
  - `joy sync --update-lock` semver metadata persisted in `joy.lock`
- Existing dependency command JSON shape tests updated additively for `joy add` semver fields.

## Blockers & Solutions

- Blocker: The resolver's original injected test hook only accepted exact revisions (`FnMut(&PackageId, &str) -> commit`), which was too narrow for semver range selection.
- Solution: Added an internal selector seam (`ResolveRequest` + `ResolvedSelection`) for production and semver-aware unit tests, while preserving the public exact-mode test helper as a compatibility wrapper.

- Blocker: Build/sync lockfile assembly and prefetch stages key source checkouts by `requested_rev`.
- Solution: Semver direct dependencies normalize to a concrete selected tag in `requested_rev`, and carry the range/version as additive metadata. This kept the existing prefetch/cache and lockfile comparison logic deterministic with minimal churn.

- Blocker: `joy add` / `joy update` needed semver support without breaking exact-ref workflows or JSON consumers.
- Solution: Added `--version` as an additive CLI flag, kept `rev` payload behavior for exact flows, and added semver metadata fields additively (`requested_requirement`, `resolved_version`).

## Fallback Plan

If semver tag resolution proves brittle for edge-case tag naming across repositories, keep the manifest/lockfile semver schema and resolver metadata, but temporarily gate semver fetching to tags that parse cleanly (`vX.Y.Z` / `X.Y.Z`) and document that policy explicitly. Exact-ref workflows remain untouched and can be used as the escape hatch while broadening tag normalization logic later.
