---
date: 2026-02-24
commit: pending
tags:
  - phase-9
  - scaffold
  - templates
  - docs
  - multifile
related_components:
  - /Users/autoparallel/Code/joy/src/templates.rs
  - /Users/autoparallel/Code/joy/tests/cli_scaffold.rs
  - https://github.com/harnesslabs/joy/issues/35
---

# Scaffold Hints for Multi-File Project Configuration

## Why

Phase 9 added multi-file manifest fields and build support, but freshly scaffolded projects still only showed the single-file shape. New users needed a discoverable hint that `project.extra_sources` and `project.include_dirs` exist.

## What Changed

- Added commented examples for `extra_sources` and `include_dirs` directly beneath `project.entry` in the generated `joy.toml` template.
- Kept defaults unchanged (comments only; no behavior change for scaffolded projects).
- Updated exact-string scaffold/template tests to match the new template output.

## Result

Phase 9 now includes the schema, build implementation, tests, and a discoverable scaffold path for multi-file configuration.
