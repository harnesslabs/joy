---
date: 2026-02-24
commit: 28dabc0
tags:
  - roadmap
  - github
  - milestones
  - issues
  - agents
  - notes
related_components:
  - /Users/autoparallel/Code/joy/.codex/AGENTS.md
  - /Users/autoparallel/Code/joy/notes/roadmap.md
  - https://github.com/harnesslabs/joy/issues/13
  - https://github.com/harnesslabs/joy/issues/14
  - https://github.com/harnesslabs/joy/issues/15
  - https://github.com/harnesslabs/joy/issues/16
  - https://github.com/harnesslabs/joy/issues/17
  - https://github.com/harnesslabs/joy/issues/18
  - https://github.com/harnesslabs/joy/issues/19
  - https://github.com/harnesslabs/joy/issues/20
  - https://github.com/harnesslabs/joy/issues/21
---

# Phase 7-14 Roadmap Bootstrap and GitHub Backlog Seeding

## Why

Phase 1-6 was already delivered and archived, but the repo only had a lightweight placeholder in `notes/roadmap.md` for the next stage. To support autonomous development, the next milestone set needed to be concretely seeded in GitHub (milestones + trackers + implementation issues) and codified in `.codex/AGENTS.md` so future turns follow the same process without re-planning from scratch.

## Blockers & Solutions

- Blocker: The roadmap plan required a large number of GitHub objects (8 milestones, 1 umbrella issue, 8 phase trackers, 37 implementation issues), which is error-prone to create manually and easy to duplicate on retries.
- Solution: Used a scripted `gh` seeding pass with exact-title duplicate checks and milestone/issue body templates, then captured the resulting issue map to update `notes/roadmap.md`.

- Blocker: Existing `.codex/AGENTS.md` covered `gh` for CI/triage workflows but did not define a general roadmap-seeding workflow.
- Solution: Added a new mandatory roadmap planning and GitHub backlog seeding section (preflight checks, milestone/issue sequence, issue content requirements, and safety rules).

- Blocker: `notes/roadmap.md` still referenced the old post-merge cleanup state (including a merged PR as if it were open), which would mislead autonomous execution.
- Solution: Rewrote `notes/roadmap.md` to point at the new Phase 7-14 milestones, umbrella tracker `#13`, phase trackers `#14-#21`, and the immediate Phase 7 execution queue `#22-#26`.

## Fallback Plan

If any part of the seeded roadmap issue set proves too granular or mis-scoped, keep the milestone structure and umbrella tracker `#13`, then close or rewrite individual implementation issues in place rather than recreating the whole roadmap. The durable source of truth should remain:

- `/Users/autoparallel/Code/joy/notes/roadmap.md` for local sequencing
- GitHub milestones 1-8 (Phase 7-14)
- phase tracker issues `#14-#21`
