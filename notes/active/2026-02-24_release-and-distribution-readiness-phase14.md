---
date: 2026-02-24
commit: pending
tags:
  - phase-14
  - release
  - distribution
  - docs
  - ci
related_components:
  - /Users/autoparallel/Code/joy/.github/workflows/release.yaml
  - /Users/autoparallel/Code/joy/.github/workflows/check.yaml
  - /Users/autoparallel/Code/joy/docs/releasing.md
  - /Users/autoparallel/Code/joy/packaging/README.md
  - /Users/autoparallel/Code/joy/packaging/homebrew/joy.rb
  - /Users/autoparallel/Code/joy/packaging/scoop/joy.json
  - /Users/autoparallel/Code/joy/README.md
  - /Users/autoparallel/Code/joy/CONTRIBUTING.md
  - /Users/autoparallel/Code/joy/justfile
  - /Users/autoparallel/Code/joy/notes/roadmap.md
  - https://github.com/harnesslabs/joy/issues/55
  - https://github.com/harnesslabs/joy/issues/56
  - https://github.com/harnesslabs/joy/issues/57
  - https://github.com/harnesslabs/joy/issues/58
---

# Phase 14: Release and Distribution Readiness

## What Shipped

- Added a tagged GitHub release workflow (`.github/workflows/release.yaml`) that builds platform binaries, packages archives, emits SHA256 files, and publishes GitHub Release assets.
- Documented release/versioning policy and a repeatable release checklist in `docs/releasing.md`.
- Added Homebrew and Scoop packaging metadata templates plus packaging notes under `packaging/`.
- Added distribution metadata syntax validation (`ruby -c`, `jq`) in CI and `just dist-metadata-check`.
- Refreshed `README.md` and `CONTRIBUTING.md` to reflect current feature set (Phases 7-9, 11-14), reproducible workflows, diagnostics, and platform caveats.

## Release Artifact Conventions

Artifacts are published as `joy-vX.Y.Z-<target>.<ext>` with matching `.sha256` files for:

- Linux (`x86_64-unknown-linux-gnu`)
- macOS (`aarch64-apple-darwin`)
- Windows GNU (`x86_64-pc-windows-gnu`)

## Notes

- Windows MSVC packaging remains deferred alongside Phase 10 (MSVC parity).
- Homebrew/Scoop files are template-driven and intended to be copied into dedicated tap/bucket repos or used for local installs after updating version/SHA values per release.
