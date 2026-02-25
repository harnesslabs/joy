---
date: 2026-02-24
commit: pending
tags:
  - roadmap
  - phase15
  - phase16
  - phase17
  - phase18
  - github
  - milestones
  - issues
related_components:
  - /Users/autoparallel/Code/joy/notes/roadmap.md
  - https://github.com/harnesslabs/joy/issues/60
  - https://github.com/harnesslabs/joy/issues/61
  - https://github.com/harnesslabs/joy/issues/62
  - https://github.com/harnesslabs/joy/issues/63
  - https://github.com/harnesslabs/joy/issues/64
  - https://github.com/harnesslabs/joy/issues/65
  - https://github.com/harnesslabs/joy/issues/84
---

# Phase 15-18 Roadmap Seeding (CLI UX + Workspaces + Solver + Registry)

## Why

Phase 7-14 is complete, but the next wave needed a concrete GitHub backlog before implementation could proceed. The user also requested an execution strategy with a single final PR, so the roadmap needed to be seeded in a way that supports long-running branch work without losing issue-level traceability.

## What Changed

- Created new GitHub milestones:
  - Phase 15 - CLI Human UX and Helpfulness
  - Phase 16 - Workspace Support and Multiple Targets
  - Phase 17 - Semver Ranges and Solver Integration
  - Phase 18 - Registry/Index Strategy Beyond GitHub Shorthand
- Created umbrella tracker issue `#60` and phase trackers `#61`-`#64`.
- Created 20 medium-grain implementation issues `#65`-`#84` (5 per phase) with explicit scope, acceptance criteria, tests, references, and compatibility notes.
- Updated `/Users/autoparallel/Code/joy/notes/roadmap.md` to make the new wave the active tracker state and preserve Phase 7-14 as the prior delivered wave.

## Blockers & Solutions

- Blocker: Creating 29 GitHub objects (4 milestones + 25 issues) manually is error-prone and difficult to retry safely.
- Solution: Used an idempotent `gh` + Python seeding script with exact-title duplicate checks and centralized issue body templates.

- Blocker: The local roadmap still treated Phase 15-17 as unscheduled stretch phases.
- Solution: Rewrote `/Users/autoparallel/Code/joy/notes/roadmap.md` to make Phase 15-18 the active wave with seeded issue inventory and execution order.

## Fallback Plan

If any seeded issue is mis-scoped, preserve the milestone/tracker structure and edit/close the affected issue in place rather than recreating the roadmap. The durable sources of truth for sequencing remain:

- `/Users/autoparallel/Code/joy/notes/roadmap.md`
- umbrella tracker `#60`
- phase trackers `#61`-`#64`

