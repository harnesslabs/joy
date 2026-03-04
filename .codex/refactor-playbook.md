# Joy Refactor Playbook

Use this playbook when the goal is to simplify `joy` as aggressively as possible without losing functionality.

## Goals

- Delete as much code as possible while preserving behavior.
- Reduce moving parts (files, modules, dependencies, branching logic).
- Make core flows easier to understand, test, and maintain.

## Non-Goals

- Shipping new product features.
- Changing public behavior without explicit approval.
- "Clever" rewrites that increase complexity.

## Functional Parity Definition

Refactors are valid only if all are true:

- Existing tests pass.
- Documented CLI flows still work (`README.md` workflows).
- JSON/machine output contracts remain compatible.
- Exit codes and core error semantics stay equivalent.

## Refactor Score (Primary Metric)

Primary score is based on net code deletion:

`Refactor Score = lines_deleted - lines_added`

Rules:

- Count only merged changes that pass validation checks.
- If parity breaks, the score for that change is `0` until fixed.
- Higher positive score is better.

Optional secondary metrics (tie-breakers):

- Files removed
- Dependencies removed
- Public API surface reduced

## Required Validation

Run before claiming score for a refactor batch:

1. `just lint`
2. `just test`
3. Smoke-check key CLI flows from `README.md`

If any check fails, fix first. Do not count score yet.

## Target Selection (Delete-First Order)

Prioritize work that tends to remove the most code safely:

1. Dead code and unreachable branches
2. Duplicate implementations (merge into one path)
3. Thin pass-through wrappers with no real abstraction value
4. One-off helpers that can be inlined cleanly
5. Redundant configuration plumbing
6. Unused dependencies and feature flags
7. Over-split modules that can be collapsed

## Refactor Loop

1. Pick one bounded area with high deletion potential.
2. Record current behavior (tests + command repros).
3. Add missing safety tests if behavior is under-specified.
4. Simplify implementation and delete redundant code.
5. Run required validation.
6. Compute and record score.
7. Repeat with the next highest-value target.

## Risk Guardrails

- Prefer small, reversible batches over big-bang rewrites.
- Do not mix unrelated simplifications in one change.
- Keep commits understandable: each commit should explain what was deleted and why it is safe.
- Never delete tests to make refactors pass.

## Score Logging Template

Use this in PR descriptions or tracking notes:

```md
## Refactor Batch
- Scope:
- Functional parity checks:
  - [ ] just lint
  - [ ] just test
  - [ ] README smoke flows

## Score
- Lines deleted:
- Lines added:
- Net score (deleted - added):
- Files removed:
- Dependencies removed:

## Notes
- What complexity was removed:
- Why behavior is unchanged:
```

## Suggested Commands

Quick net score for current branch (vs `main`):

```bash
git diff --numstat main...HEAD | awk '{add+=$1; del+=$2} END {print "added=" add ", deleted=" del ", net=" del-add}'
```

Quick change size for last commit:

```bash
git show --shortstat --oneline HEAD
```
