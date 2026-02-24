# Joy Roadmap Tracker (Phase 7–14 Seeded)

The original Phase 1–6 roadmap was completed and archived. On February 24, 2026, the next roadmap wave (Phase 7–14) was seeded in GitHub with milestones, a roadmap umbrella tracker, phase trackers, and medium-grain implementation issues.

Archived milestone (Phase 1–6):
- `/Users/autoparallel/Code/joy/notes/archive/2026-phase1-6-roadmap-delivered/MILESTONE.md`
- `/Users/autoparallel/Code/joy/notes/archive/2026-phase1-6-roadmap-delivered/source-notes/roadmap.md`

## Current Status

- Roadmap umbrella tracker: [#13](https://github.com/harnesslabs/joy/issues/13)
- Current phase: Phase 10 (`Windows MSVC Parity`) is deferred by choice while GNU Windows remains supported
- Current phase tracker: [#17](https://github.com/harnesslabs/joy/issues/17) (deferred / open)
- Completed phases in this wave: 7, 8, 9, 11, 12, 13, 14
- Latest completed milestone notes:
  - `/Users/autoparallel/Code/joy/notes/active/2026-02-24_ux-machine-interface-stability-phase13.md`
  - `/Users/autoparallel/Code/joy/notes/active/2026-02-24_release-and-distribution-readiness-phase14.md`
- Roadmap bootstrap note: `/Users/autoparallel/Code/joy/notes/active/2026-02-24_phase7-14-roadmap-bootstrap-and-github-seeding.md`

## Phase 7–14 Milestones

- Phase 7: [Phase 7 - Lockfile Fidelity & Determinism](https://github.com/harnesslabs/joy/milestone/1) (tracker [#14](https://github.com/harnesslabs/joy/issues/14))
- Phase 8: [Phase 8 - Sync / Offline / Frozen Workflows](https://github.com/harnesslabs/joy/milestone/2) (tracker [#15](https://github.com/harnesslabs/joy/issues/15))
- Phase 9: [Phase 9 - Multi-File Project Build Model](https://github.com/harnesslabs/joy/milestone/3) (tracker [#16](https://github.com/harnesslabs/joy/issues/16))
- Phase 10: [Phase 10 - Windows MSVC Parity](https://github.com/harnesslabs/joy/milestone/4) (tracker [#17](https://github.com/harnesslabs/joy/issues/17))
- Phase 11: [Phase 11 - Dependency UX Commands](https://github.com/harnesslabs/joy/milestone/5) (tracker [#18](https://github.com/harnesslabs/joy/issues/18))
- Phase 12: [Phase 12 - Recipe Ecosystem Expansion](https://github.com/harnesslabs/joy/milestone/6) (tracker [#19](https://github.com/harnesslabs/joy/issues/19))
- Phase 13: [Phase 13 - UX + Machine Interface Stability](https://github.com/harnesslabs/joy/milestone/7) (tracker [#20](https://github.com/harnesslabs/joy/issues/20))
- Phase 14: [Phase 14 - Release & Distribution Readiness](https://github.com/harnesslabs/joy/milestone/8) (tracker [#21](https://github.com/harnesslabs/joy/issues/21))

## Immediate Execution Queue (Deferred Phase 10)

Execution order when Windows MSVC parity resumes:

1. [#37](https://github.com/harnesslabs/joy/issues/37) `[toolchain][windows] implement MSVC compiler discovery path instead of MsvcUnsupportedPhase4`
2. [#38](https://github.com/harnesslabs/joy/issues/38) `[ninja][windows] add MSVC compile/link flag rendering and dependency tracking behavior`
3. [#39](https://github.com/harnesslabs/joy/issues/39) `[cmake][windows] support MSVC recipe builds and artifact detection in cache pipeline`
4. [#40](https://github.com/harnesslabs/joy/issues/40) `[ci] add windows-msvc build/lint/test matrix jobs (and compiled-e2e where feasible)`
5. [#41](https://github.com/harnesslabs/joy/issues/41) `[tests][windows] add MSVC-targeted fixture coverage and path/line-ending regressions`

## Phase Issue Inventory (Seeded)

- Phase 8 tracker [#15](https://github.com/harnesslabs/joy/issues/15) (closed): `#27`, `#28`, `#29`, `#30`, `#31`
- Phase 9 tracker [#16](https://github.com/harnesslabs/joy/issues/16) (closed): `#32`, `#33`, `#34`, `#35`, `#36`
- Phase 10 tracker [#17](https://github.com/harnesslabs/joy/issues/17) (open/deferred): `#37`, `#38`, `#39`, `#40`, `#41`
- Phase 11 tracker [#18](https://github.com/harnesslabs/joy/issues/18) (closed): `#42`, `#43`, `#44`, `#45`, `#46`
- Phase 12 tracker [#19](https://github.com/harnesslabs/joy/issues/19) (closed): `#47`, `#48`, `#49`, `#50`
- Phase 13 tracker [#20](https://github.com/harnesslabs/joy/issues/20) (closed): `#51`, `#52`, `#53`, `#54`
- Phase 14 tracker [#21](https://github.com/harnesslabs/joy/issues/21) (pending close): `#55`, `#56`, `#57`, `#58`

## Locked Roadmap Priorities

- Reproducibility first (lockfile fidelity + deterministic behavior)
- Additive CLI/JSON compatibility (fields/commands only added, not renamed/removed)
- Issue-first execution via GitHub milestones + trackers
- Cargo/uv-style usability path: deterministic workflows, strong UX diagnostics, platform parity, dependency ergonomics

## Stretch Phases (Planned, Not Scheduled)

- Phase 15: workspace support and multiple targets
- Phase 16: semver ranges and solver integration
- Phase 17: registry/index strategy beyond GitHub shorthand
