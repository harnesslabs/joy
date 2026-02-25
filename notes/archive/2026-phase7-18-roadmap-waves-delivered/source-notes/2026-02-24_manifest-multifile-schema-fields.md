---
date: 2026-02-24
commit: pending
tags:
  - phase-9
  - manifest
  - schema
  - multifile
  - compatibility
  - tests
related_components:
  - /Users/autoparallel/Code/joy/src/manifest.rs
  - /Users/autoparallel/Code/joy/src/resolver.rs
  - https://github.com/harnesslabs/joy/issues/32
---

# Manifest Schema: `project.extra_sources` and `project.include_dirs`

## Why

Phase 9 needs a backward-compatible manifest schema expansion before the build pipeline can support multi-file projects. `joy.toml` only modeled `project.entry`, which blocked any additive multi-source config without changing the schema first.

## What Changed

- Added optional `project.extra_sources: Vec<String>` and `project.include_dirs: Vec<String>` to `ProjectSection`.
- Marked both fields with `serde(default)` so existing manifests (without these keys) continue to parse without migration.
- Added validation to reject empty-string entries in either list.
- Kept serialization backward-compatible by omitting empty lists from `joy.toml` (`skip_serializing_if = "Vec::is_empty"`), so current single-file workflows don’t rewrite manifests with noisy empty arrays.

## Coverage

- Manifest round-trip serialization now includes populated multi-file fields.
- Added parsing test to confirm manifests without the new fields default to empty vectors.
- Updated resolver test fixture manifest construction for the expanded `ProjectSection` shape.

## Follow-Up

`#33` and `#34` still need the actual multi-source object graph and Ninja generation changes that consume these manifest fields.
