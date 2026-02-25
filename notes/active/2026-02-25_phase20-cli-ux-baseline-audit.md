---
date: 2026-02-25
commit: TBD
tags:
  - phase20
  - ux
  - audit
  - cli
  - output
related_components:
  - /Users/autoparallel/Code/joy/src/output.rs
  - /Users/autoparallel/Code/joy/src/cli.rs
  - /Users/autoparallel/Code/joy/src/commands/build.rs
  - /Users/autoparallel/Code/joy/src/commands/doctor.rs
  - /Users/autoparallel/Code/joy/tests/smoke.rs
  - /Users/autoparallel/Code/joy/tests/build_run.rs
  - https://github.com/harnesslabs/joy/issues/92
---

# Phase 20: CLI UX Baseline Audit (Pre-Polish Snapshot)

## Why

Before broadening human-mode CLI UX work, we needed a crisp baseline of where the current experience was weakest so improvements could be prioritized without touching JSON compatibility.

## Baseline Findings (Pre-Change)

- Human progress output relied on plain textual prefixes (`==>`, `->`) with no colors or glyph hierarchy.
- Long-running dependency/build operations could feel idle because there was no spinner/progress backend.
- Error output had actionable hints for many codes, but lacked stronger visual emphasis and explicit “where to look” pointers for build failures.
- `README.md` contained too much detailed usage/reference content and was acting as both overview and full manual.
- `docs/` contained useful reference markdown, but there was no cohesive user guide or navigable docs site.
- `notes/active/` contained many completed notes from prior phases, making current work harder to locate.

## Target Improvements (Prioritized)

1. Centralize human rendering and add color/glyph semantics with strict JSON preservation.
2. Add a shared progress backend (spinner-first) and reuse existing progress call sites.
3. Improve build failure narration with stable log locations.
4. Create an mdBook as the canonical user/reference guide and slim the README.
5. Compact `notes/active/` so active execution context is easy to scan.

## Evidence Sources

- Code inspection in `/Users/autoparallel/Code/joy/src/output.rs` and command handlers
- Existing human-output tests in `/Users/autoparallel/Code/joy/tests/smoke.rs` and `/Users/autoparallel/Code/joy/tests/build_run.rs`
- Current docs layout (`README.md` + `docs/*.md` + `notes/`)

## Blockers & Solutions

- Blocker: Human-mode output tests should not become brittle while formatting evolves.
- Solution: keep regression assertions focused on sections/key phrases and JSON contract checks, not full-string snapshots.

## Fallback Plan

If UX polish causes regressions, retain the centralized renderer and disable only the specific styling/progress feature behind defaults or flags. Keep JSON and command semantics unchanged.
