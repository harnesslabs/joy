# Notes Organization

`notes/` is the project's long-term memory for roadmap tracking, architectural decisions, blockers, and milestone summaries.

This directory was compacted during the Phase 19-37 UX/docs wave so `notes/active/` stays focused on current execution only.

## Layout

- `/Users/autoparallel/Code/joy/notes/active/` — current roadmap wave notes, active audits, and short-horizon next-work notes
- `/Users/autoparallel/Code/joy/notes/archive/` — delivered milestone archives and preserved source notes from prior waves
- `/Users/autoparallel/Code/joy/notes/roadmap.md` — current top-level tracker and issue/milestone pointers

## Naming Convention (Preferred)

- `YYYY-MM-DD_<commit-hash>_<topic-slug>.md`

Allowed when avoiding extra commit churn:

- `YYYY-MM-DD_<topic-slug>.md`

The `commit` field in YAML frontmatter is the authoritative commit reference.

## Required Note Content

Each milestone or implementation note should include:

- Why
- Blockers & Solutions
- Fallback Plan

## Search Tips

```bash
# Current wave notes
find /Users/autoparallel/Code/joy/notes/active -maxdepth 1 -type f | sort

# Search archived implementation history
rg -n "resolver|lockfile|registry" /Users/autoparallel/Code/joy/notes/archive

# List milestone summaries
find /Users/autoparallel/Code/joy/notes/archive -name MILESTONE.md | sort
```
