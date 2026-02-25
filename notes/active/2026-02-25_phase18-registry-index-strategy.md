---
date: 2026-02-25
commit: b75825e
tags:
  - phase18
  - registry
  - index
  - resolver
  - manifest
  - cli
  - lockfile
  - docs
  - tests
related_components:
  - /Users/autoparallel/Code/joy/src/registry.rs
  - /Users/autoparallel/Code/joy/src/manifest.rs
  - /Users/autoparallel/Code/joy/src/resolver.rs
  - /Users/autoparallel/Code/joy/src/commands/add.rs
  - /Users/autoparallel/Code/joy/src/commands/update.rs
  - /Users/autoparallel/Code/joy/src/commands/remove.rs
  - /Users/autoparallel/Code/joy/src/commands/tree.rs
  - /Users/autoparallel/Code/joy/src/commands/build.rs
  - /Users/autoparallel/Code/joy/src/lockfile.rs
  - /Users/autoparallel/Code/joy/tests/add_command.rs
  - /Users/autoparallel/Code/joy/docs/machine-interface.md
  - /Users/autoparallel/Code/joy/docs/error-codes.md
  - /Users/autoparallel/Code/joy/README.md
  - https://github.com/harnesslabs/joy/issues/64
  - https://github.com/harnesslabs/joy/issues/80
  - https://github.com/harnesslabs/joy/issues/81
  - https://github.com/harnesslabs/joy/issues/82
  - https://github.com/harnesslabs/joy/issues/83
  - https://github.com/harnesslabs/joy/issues/84
---

# Phase 18: Registry/Index Strategy Beyond GitHub Shorthand

## Why

Phase 17 added semver ranges for direct GitHub dependencies, but version discovery still depended on remote git tags per package. Phase 18 introduces a registry/index source path so version candidates can come from a curated index cache, which is a better foundation for reproducible solver behavior, offline workflows, and future source/provider expansion.

## What Shipped

- New registry module in `/Users/autoparallel/Code/joy/src/registry.rs`:
  - git-backed registry index mirror/cache under `JOY_HOME`
  - cached checkout materialization keyed by registry index commit
  - semver version selection from `index.toml`
  - deterministic package/release validation and stable registry errors
- Manifest source model expanded in `/Users/autoparallel/Code/joy/src/manifest.rs`:
  - `source = "registry"` added to dependency entries
  - registry dependencies require `version = "..."` in this phase cut (no registry `rev` entries)
- Resolver integration in `/Users/autoparallel/Code/joy/src/resolver.rs`:
  - direct `source = "registry"` dependencies resolve versions from the registry index first
  - selected registry release maps to a concrete GitHub source rev, then reuses the existing fetch/recipe/build pipeline
  - resolved nodes now carry additive source provenance (`registry`, `source_package`)
- Command UX / JSON updates:
  - `joy add registry:<owner/repo> --version <range>`
  - `joy update <owner/repo>` refreshes registry-backed dependencies using the stored range
  - `joy remove registry:<owner/repo>` strips the prefix and removes the canonical manifest entry
  - additive JSON fields on dependency commands and tree package entries:
    - `source`
    - `registry`
    - `source_package`
- Lockfile provenance in `/Users/autoparallel/Code/joy/src/lockfile.rs`:
  - additive `registry` and `source_package` per locked package
  - retained semver metadata from Phase 17 (`requested_requirement`, `resolved_version`)
- Error/hint/docs updates:
  - registry error codes (`registry_not_configured`, `registry_load_failed`, `registry_package_not_found`, `registry_alias_unsupported`)
  - README registry usage docs (`JOY_REGISTRY_DEFAULT`, `registry:<pkg>` syntax)
  - machine-interface docs updated for additive source-provenance fields

## Validation

- `just lint`
- `just test`
- `just recipe-check`
- `just ci-local`

## Coverage Added

- `/Users/autoparallel/Code/joy/src/registry.rs` unit tests for:
  - highest matching semver selection from registry index entries
  - explicit validation failure for alias package mappings in the initial phase cut
- `/Users/autoparallel/Code/joy/tests/add_command.rs` integration coverage for:
  - `joy add registry:<pkg> --version <range>`
  - registry-backed `joy tree` / `joy sync --update-lock` source provenance and semver lock metadata
  - registry-backed `joy update` after registry index changes (git mirror refresh path)
  - offline cold-cache registry failure (`offline_cache_miss`) and warm-cache offline tree success

## Blockers & Solutions

- Blocker: The existing build/recipe/link/install pipeline assumes canonical `owner/repo` IDs (`PackageId`) for cache layout, recipes, and header install slugs.
- Solution: Phase 18 initial cut introduces explicit registry source metadata while keeping canonical package IDs in manifests and resolved nodes. Registry index entries map to concrete GitHub source revs, so the existing pipeline is reused safely.

- Blocker: Registry index transport needed to honor the same offline/progress semantics as source fetching without destabilizing the fetch API.
- Solution: Added a lightweight fetch-runtime accessor and implemented a separate git-backed registry cache loader that reads the shared runtime options but keeps transport/cache logic isolated in `/Users/autoparallel/Code/joy/src/registry.rs`.

- Blocker: Phase 18 issue #80 targets package-coordinate generalization, but fully supporting alias registry package IDs would require broader changes across recipes, lockfile semantics, and header install path identity.
- Solution: Implemented explicit alias detection with a stable `registry_alias_unsupported` error and documented the limitation. This keeps the registry architecture and CLI UX in place while deferring the higher-risk identity refactor.

## Fallback Plan

If the git-backed registry index path proves fragile in real-world usage, keep the manifest/resolver/lockfile source metadata and switch registry loading to a local-path-only index during stabilization (same schema, no network refresh). The rollback boundary is the transport/cache portion of `/Users/autoparallel/Code/joy/src/registry.rs`; command/resolver interfaces can remain intact.
