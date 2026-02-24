# Notes Organization

`notes/` is the project's long-term memory for roadmap tracking, architectural decisions, blockers, and milestone summaries.

## Layout

- `/Users/autoparallel/Code/joy/notes/active/` — current and upcoming work notes
- `/Users/autoparallel/Code/joy/notes/archive/` — completed milestone archives (summary + source notes)
- `/Users/autoparallel/Code/joy/notes/roadmap.md` — current top-level tracker / pointer for the next milestone cycle

## Naming Convention (Preferred)

- `YYYY-MM-DD_<commit-hash>_<topic-slug>.md`

Allowed when avoiding extra commit churn:
- `YYYY-MM-DD_<topic-slug>.md`

The `commit` field in YAML frontmatter is the authoritative commit reference.

## Required Note Content

Each milestone note should include:
- Why
- Blockers & Solutions
- Fallback Plan

## Search Tips

```bash
# Find all notes related to resolver work
rg -n "resolver" /Users/autoparallel/Code/joy/notes

# Find all notes tagged phase6
rg -n "phase6" /Users/autoparallel/Code/joy/notes/archive

# List archived milestone summaries
find /Users/autoparallel/Code/joy/notes/archive -name MILESTONE.md | sort
```
