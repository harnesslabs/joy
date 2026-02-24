# Joy Roadmap Tracker (Phase 15–18 In Progress)

The original Phase 1–6 roadmap was completed and archived. On February 24, 2026, the next roadmap wave (Phase 7–14) was seeded in GitHub and delivered. A new wave (Phase 15–18) has now been seeded for CLI UX, workspaces/targets, semver solving, and registry/index support.

Archived milestone (Phase 1–6):
- `/Users/autoparallel/Code/joy/notes/archive/2026-phase1-6-roadmap-delivered/MILESTONE.md`
- `/Users/autoparallel/Code/joy/notes/archive/2026-phase1-6-roadmap-delivered/source-notes/roadmap.md`

## Current Status

- Roadmap umbrella tracker: [#60](https://github.com/harnesslabs/joy/issues/60)
- Umbrella tracker status: open (Phase 15–18 roadmap wave)
- Current phase: Phase 17 (Semver Ranges and Solver Integration)
- Current phase tracker: [#63](https://github.com/harnesslabs/joy/issues/63)
- Completed phases in this wave: 15, 16
- Previously completed wave: Phase 7–14 (tracker [#13](https://github.com/harnesslabs/joy/issues/13), closed)
- Latest completed milestone notes:
  - `/Users/autoparallel/Code/joy/notes/active/2026-02-24_windows-msvc-parity-phase10.md`
  - `/Users/autoparallel/Code/joy/notes/active/2026-02-24_ux-machine-interface-stability-phase13.md`
  - `/Users/autoparallel/Code/joy/notes/active/2026-02-24_release-and-distribution-readiness-phase14.md`
  - `/Users/autoparallel/Code/joy/notes/active/2026-02-24_phase15-cli-human-ux-and-helpfulness.md`
  - `/Users/autoparallel/Code/joy/notes/active/2026-02-24_phase16-workspace-support-and-multiple-targets.md`
- Roadmap bootstrap note: `/Users/autoparallel/Code/joy/notes/active/2026-02-24_phase7-14-roadmap-bootstrap-and-github-seeding.md`

## Phase 15–18 Milestones

- Phase 15: [Phase 15 - CLI Human UX and Helpfulness](https://github.com/harnesslabs/joy/milestone/9) (tracker [#61](https://github.com/harnesslabs/joy/issues/61))
- Phase 16: [Phase 16 - Workspace Support and Multiple Targets](https://github.com/harnesslabs/joy/milestone/10) (tracker [#62](https://github.com/harnesslabs/joy/issues/62))
- Phase 17: [Phase 17 - Semver Ranges and Solver Integration](https://github.com/harnesslabs/joy/milestone/11) (tracker [#63](https://github.com/harnesslabs/joy/issues/63))
- Phase 18: [Phase 18 - Registry/Index Strategy Beyond GitHub Shorthand](https://github.com/harnesslabs/joy/milestone/12) (tracker [#64](https://github.com/harnesslabs/joy/issues/64))

## Prior Wave (Phase 7–14, Delivered)

- Phase 7: [Phase 7 - Lockfile Fidelity & Determinism](https://github.com/harnesslabs/joy/milestone/1) (tracker [#14](https://github.com/harnesslabs/joy/issues/14), closed)
- Phase 8: [Phase 8 - Sync / Offline / Frozen Workflows](https://github.com/harnesslabs/joy/milestone/2) (tracker [#15](https://github.com/harnesslabs/joy/issues/15), closed)
- Phase 9: [Phase 9 - Multi-File Project Build Model](https://github.com/harnesslabs/joy/milestone/3) (tracker [#16](https://github.com/harnesslabs/joy/issues/16), closed)
- Phase 10: [Phase 10 - Windows MSVC Parity](https://github.com/harnesslabs/joy/milestone/4) (tracker [#17](https://github.com/harnesslabs/joy/issues/17), closed)
- Phase 11: [Phase 11 - Dependency UX Commands](https://github.com/harnesslabs/joy/milestone/5) (tracker [#18](https://github.com/harnesslabs/joy/issues/18), closed)
- Phase 12: [Phase 12 - Recipe Ecosystem Expansion](https://github.com/harnesslabs/joy/milestone/6) (tracker [#19](https://github.com/harnesslabs/joy/issues/19), closed)
- Phase 13: [Phase 13 - UX + Machine Interface Stability](https://github.com/harnesslabs/joy/milestone/7) (tracker [#20](https://github.com/harnesslabs/joy/issues/20), closed)
- Phase 14: [Phase 14 - Release & Distribution Readiness](https://github.com/harnesslabs/joy/milestone/8) (tracker [#21](https://github.com/harnesslabs/joy/issues/21), closed)

## Immediate Execution Queue

Start the new wave with Phase 15 (CLI human UX) and proceed issue-by-issue on the single wave branch:

1. [#75](https://github.com/harnesslabs/joy/issues/75) `[manifest]` exact ref vs semver requirement model
2. [#76](https://github.com/harnesslabs/joy/issues/76) `[resolver]` solver seam and candidate selection split
3. [#77](https://github.com/harnesslabs/joy/issues/77) `[fetch][semver]` semver range resolution from Git tags
4. [#78](https://github.com/harnesslabs/joy/issues/78) `[lockfile][ux]` semver lock metadata + diagnostics
5. [#79](https://github.com/harnesslabs/joy/issues/79) semver CLI/tree/docs payload additions
6. Phase 18 tracker [#64](https://github.com/harnesslabs/joy/issues/64)

## Phase Issue Inventory (Seeded)

- Phase 8 tracker [#15](https://github.com/harnesslabs/joy/issues/15) (closed): `#27`, `#28`, `#29`, `#30`, `#31`
- Phase 9 tracker [#16](https://github.com/harnesslabs/joy/issues/16) (closed): `#32`, `#33`, `#34`, `#35`, `#36`
- Phase 10 tracker [#17](https://github.com/harnesslabs/joy/issues/17) (closed): `#37`, `#38`, `#39`, `#40`, `#41`
- Phase 11 tracker [#18](https://github.com/harnesslabs/joy/issues/18) (closed): `#42`, `#43`, `#44`, `#45`, `#46`
- Phase 12 tracker [#19](https://github.com/harnesslabs/joy/issues/19) (closed): `#47`, `#48`, `#49`, `#50`
- Phase 13 tracker [#20](https://github.com/harnesslabs/joy/issues/20) (closed): `#51`, `#52`, `#53`, `#54`
- Phase 14 tracker [#21](https://github.com/harnesslabs/joy/issues/21) (closed): `#55`, `#56`, `#57`, `#58`
- Phase 15 tracker [#61](https://github.com/harnesslabs/joy/issues/61) (closed): `#65`, `#66`, `#67`, `#68`, `#69`
- Phase 16 tracker [#62](https://github.com/harnesslabs/joy/issues/62) (closed): `#70`, `#71`, `#72`, `#73`, `#74`
- Phase 17 tracker [#63](https://github.com/harnesslabs/joy/issues/63) (open): `#75`, `#76`, `#77`, `#78`, `#79`
- Phase 18 tracker [#64](https://github.com/harnesslabs/joy/issues/64) (open): `#80`, `#81`, `#82`, `#83`, `#84`

## Locked Roadmap Priorities

- Reproducibility first (lockfile fidelity + deterministic behavior)
- Additive CLI/JSON compatibility (fields/commands only added, not renamed/removed)
- Issue-first execution via GitHub milestones + trackers
- Cargo/uv-style usability path: deterministic workflows, strong UX diagnostics, platform parity, dependency ergonomics
- Human-first CLI usability improvements before larger architecture shifts (Phase 15)

## Next Stretch (Post-Phase 18, Unplanned)

- Workspace-wide lockfile strategy (deferred from Phase 16)
- Broader registry protocols / sparse index options (beyond Phase 18 git-backed index)
- Deeper package feature/variant support and non-binary target kinds
