---
date: 2026-02-25
commit: pending
tags:
  - ci
  - windows
  - tests
  - compile-db
related_components:
  - /Users/autoparallel/Code/joy/tests/add_command.rs
  - https://github.com/harnesslabs/joy/pull/147
---

# Windows CI Follow-Up: Compile DB Assertion Path Normalization

## Why

PR #147 (Phase 38-76 delivery wave) passed local CI parity but failed on Windows `test` matrix jobs (`x86_64-pc-windows-gnu` and `x86_64-pc-windows-msvc`) in `tests/add_command.rs`.

The product code was working; the failing assertions were test-only checks for dependency include paths in generated `compile_commands.json`.

## What Failed

Two tests failed with:

- `expected dependency include path in compile db`

Affected tests in `/Users/autoparallel/Code/joy/tests/add_command.rs`:

- `sync_materializes_header_only_dependencies_and_lockfile_without_app_build`
- `add_generates_compile_commands_json_when_toolchain_is_available`

## Root Cause

The tests normalized Windows paths by replacing `\\` with `/` on the raw compile database JSON text. On Windows, JSON strings store escaped backslashes (`\\\\` in raw text), so naive replacement produced doubled slashes (`//`) and the path-fragment substring checks no longer matched.

## Solution

Added a small test helper (`normalize_pathish_text`) that:

1. replaces backslashes with `/`
2. collapses repeated `/` sequences

This keeps the assertions platform-agnostic without changing product behavior or compile DB generation.

## Validation

- `just lint`
- `just test`

## Fallback Plan

If future compile DB assertions become fragile across platforms, parse the compile database JSON and inspect structured `command` / `arguments` fields instead of matching raw JSON text.
