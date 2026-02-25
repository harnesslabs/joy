---
date: 2026-02-25
commit: d225110
tags:
  - roadmap
  - phase19
  - phase37
  - ux
  - docs
  - mdbook
  - notes
related_components:
  - /Users/autoparallel/Code/joy/src/output.rs
  - /Users/autoparallel/Code/joy/src/cli.rs
  - /Users/autoparallel/Code/joy/README.md
  - /Users/autoparallel/Code/joy/book/
  - /Users/autoparallel/Code/joy/notes/
  - https://github.com/harnesslabs/joy/issues/86
  - https://github.com/harnesslabs/joy/issues/87
  - https://github.com/harnesslabs/joy/issues/88
  - https://github.com/harnesslabs/joy/issues/89
  - https://github.com/harnesslabs/joy/issues/90
  - https://github.com/harnesslabs/joy/issues/91
  - https://github.com/harnesslabs/joy/issues/92
  - https://github.com/harnesslabs/joy/issues/93
  - https://github.com/harnesslabs/joy/issues/94
  - https://github.com/harnesslabs/joy/issues/95
  - https://github.com/harnesslabs/joy/issues/96
  - https://github.com/harnesslabs/joy/issues/97
  - https://github.com/harnesslabs/joy/issues/98
  - https://github.com/harnesslabs/joy/issues/99
---

# Phase 19-37: CLI UX + Docs + Notes Compaction Wave

## Why

After shipping the Phase 15-18 roadmap wave, `joy` had strong functionality but still needed a larger UX/docs push:

- terminal output was still mostly plain text and prefix-based
- no mdBook user guide existed yet
- `README.md` had become too usage-heavy and long
- `notes/active/` contained many delivered notes from prior waves

This wave focuses on the user experience and contributor experience without changing the machine JSON contract.

## Backlog Structure (Seeded)

Umbrella tracker:

- #86 `[roadmap] Joy Phase 19-37 CLI UX + docs + notes compaction roadmap tracker`

Milestone trackers:

- #87 `phase19-23` CLI UX foundation and theming (milestone 13)
- #88 `phase24-29` progress UX and human output hardening (milestone 14)
- #89 `phase30-34` mdBook docs and GitHub Pages (milestone 15)
- #90 `phase35-36` developer docs and notes compaction (milestone 16)
- #91 `phase37` QA and release prep (milestone 17)

Implementation issues (medium-grain):

- #92 baseline UX audit
- #93 UI controls + themed renderer foundation
- #94 progress engine + long-running command instrumentation
- #95 diagnostics/help/terminal hardening polish
- #96 mdBook content authoring (core + advanced + reference)
- #97 README + docs CI + GitHub Pages deployment
- #98 developer docs + notes compaction
- #99 wave-wide QA and release-facing closeout

## Current Execution Notes

This note tracked the active implementation pass that started with roadmap seeding and then immediately executed the UX/docs wave in the same session. Local implementation and validation completed in the same cycle; see the Phase 37 QA closeout note for the validation snapshot.

## Blockers & Solutions

- Blocker: `notes/active/` contained many delivered notes, reducing discoverability of current work.
- Solution: archive all dated delivered notes into `notes/archive/2026-phase7-18-roadmap-waves-delivered/` and re-seed active notes for the new wave.

## Fallback Plan

If the Phase 19-37 wave needs to be split across multiple PRs, keep the milestone/issue tracker structure stable and ship in this order:

1. CLI UX foundation + progress engine
2. mdBook + README + docs CI/Pages
3. contributor docs + notes compaction + QA closeout
