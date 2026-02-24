# Joy Bug-Bash Playbook

Use this playbook when a user asks to find bugs, UX friction, or debugging regressions and record them as GitHub issues.

## Goals

- Reproduce real failures and clunky UX in `joy`.
- File one GitHub issue per confirmed bug or improvement.
- Keep a tracker issue with scope, progress, and links.
- Collect cross-platform evidence from CI via a temporary PR.

## Setup (Tracker + Labels + CI PR)

1. Confirm repo and auth:
   - `git status --short --branch`
   - `gh auth status -h github.com`
   - `gh repo view --json nameWithOwner,defaultBranchRef,url`
2. Ensure labels exist (create if missing):
   - `audit`
   - `ux`
   - `performance`
   - `needs-investigation`
   - `platform:macos`
   - `platform:linux`
   - `platform:windows`
3. Create an umbrella tracker issue labeled `audit`.
4. Create a temporary audit branch using the required prefix:
   - `codex/joy-bug-bash-YYYY-MM-DD`
5. Push an empty or docs-only audit marker commit and open a temporary PR to trigger CI.
6. Add a PR comment that the PR exists only to collect CI triage evidence and link the tracker issue.

## Issue Filing Rules

- File each finding immediately after a reliable repro ("file as found").
- Use one issue per bug or UX improvement.
- Add `audit` to every issue in the bug-bash.
- Add `bug` or `enhancement`.
- Add `ux`, `performance`, and platform labels when applicable.
- Link the tracker issue in every issue body.

## Issue Title Conventions

- Bugs: `[cmd:<command>] <short symptom>`
- UX improvements: `[ux][cmd:<command>] <short friction>`
- Machine-mode issues: `[machine] ...` or `[ux][machine] ...`
- Cross-cutting/core issues: `[core] ...`, `[docs] ...`, `[lockfile] ...`

## Issue Body Template

Use this structure for all findings:

```md
## Environment
- OS:
- Repo/ref:
- joy binary path:
- Env overrides (`JOY_HOME`, `JOY_GITHUB_BASE`, PATH changes):

## Reproduction steps
1. ...
2. ...

## Expected behavior
...

## Actual behavior
...

## Impact / severity
...

## Notes / evidence
- stdout/stderr excerpts
- paths
- CI log links
- likely root cause (optional)

## Suggested direction (optional)
...

## Related issues
- Tracker: #<tracker>
```

## Local Audit Order (Default)

### 1. README workflows first

Exercise the documented flows exactly as written in `README.md`:

- `joy new` quickstart and `joy run`
- Header-only dependency flow (`joy add nlohmann/json`, then `joy run`)
- `joy build` and `joy run -- ...`
- Compiled dependency flow (`joy add fmtlib/fmt`, then `joy build`)
- Machine mode examples (`--json`, `--machine`)

Notes:

- Use a fresh temp workspace and fresh `JOY_HOME` first.
- Repeat key commands with a warm cache.
- Time slow operations and note periods with no output.

### 2. Edge-case matrix

Cover at least:

- `new`
  - Existing non-empty directory with and without `--force`
  - Absolute target paths
  - Names/paths with spaces
  - JSON success/failure envelopes
- `init`
  - Existing manifest with and without `--force`
  - Idempotency expectations
- `add`
  - Invalid package id
  - Missing manifest
  - Bad `--rev`
  - Duplicate add / changed rev
  - Fresh cache vs warm cache UX
  - `JOY_GITHUB_BASE` local fixture behavior
  - Lockfile stale warning when `joy.lock` exists
  - Failure after side effects (for example read-only `joy.toml`)
- `build`
  - Missing manifest
  - `--locked` without `joy.lock`
  - `--update-lock` refresh path
  - Missing tool binaries (PATH-restricted repro)
  - JSON error envelopes
- `run`
  - Passthrough args after `--`
  - JSON success/failure envelopes
  - Exit code propagation (when feasible)

### 3. UX/clunkiness pass

File `enhancement` + `ux` issues for concrete friction:

- Long-running commands with no visible progress
- Weak error messages or missing next-step guidance
- Help text / docs discoverability gaps
- Output consistency problems in human vs JSON mode

## Local Fixture Pattern (Recommended for Fast `add` Repros)

Use `JOY_GITHUB_BASE` with local bare repos to make `joy add owner/repo` deterministic and fast:

- Create local bare repos at `<base>/<owner>/<repo>.git`
- Set `JOY_GITHUB_BASE=<base>`
- Run normal `joy add owner/repo`

This keeps `joy` code paths realistic while removing network and upstream repo size variability.

## CI Triage Workflow

1. Trigger CI from the temporary audit PR.
2. Track checks:
   - `gh pr checks <PR_NUMBER>`
3. If a check fails:
   - pull logs (`gh run view` / `gh run view --log` on the relevant run/job)
   - identify platform and failing step
   - reproduce locally when feasible
   - file a platform-labeled issue with CI links
4. If all checks pass:
   - record the CI run/PR in the tracker issue and note "no CI failures observed in this audit run"

## Tracker Issue Update Checklist

Keep the tracker issue current:

- Mark setup complete (labels, tracker, PR)
- Add links to each filed issue under Findings
- Mark README pass, edge matrix pass, CI pass, and UX pass complete
- Note any intentionally untested or partially tested scenarios with reasons
- Link the temporary PR and CI run summary
- Close the temporary PR after the tracker is updated

## Scope Guardrails

- Do not fix product code unless the user explicitly asks to switch from triage to implementation.
- Do not collapse multiple unrelated problems into a single issue.
- Do not file vague UX issues without a reproducible pain point and concrete examples.
