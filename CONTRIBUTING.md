# Contributing to joy

## Scope

`joy` is a Rust CLI for C++ project scaffolding, dependency management, reproducible local builds, and machine-readable automation workflows.

The project is currently optimized for rapid iteration with strong tests, milestone notes, and issue-tracked roadmap execution.

## Local Prerequisites

- Rust toolchain
- `just`
- `git`
- For build/E2E coverage: `ninja`, `cmake`, and a C++ compiler (`clang++` or `g++` / MinGW on Windows)
- Optional (distribution metadata validation): `ruby`, `jq`

Use `cargo run -- --json doctor` (or installed `joy --json doctor`) to inspect local toolchain/cache/recipe health.

## Development Commands

```bash
just build
just lint
just recipe-check
just test
just ci
just ci-local
```

Useful targeted commands:

```bash
just compiled-e2e
just dist-metadata-check
cargo run -- --json doctor
```

## Roadmap / GitHub Workflow

`joy` uses an issue-first roadmap workflow (milestones + phase trackers + implementation issues).

Contributor expectations:

- scope work to a specific issue outcome
- keep `/Users/autoparallel/Code/joy/notes/roadmap.md` current when milestone/phase status changes
- add an active note under `/Users/autoparallel/Code/joy/notes/active/` for major slices (preserve YAML frontmatter)
- close/update GitHub issues when work lands and include validation details
- check PR CI status with `gh pr checks <pr-number>` before declaring completion

## Notes / Long-Term Memory

Development history and milestone notes live in `/Users/autoparallel/Code/joy/notes`.

- active work notes go under `/Users/autoparallel/Code/joy/notes/active`
- completed milestones are archived under `/Users/autoparallel/Code/joy/notes/archive`
- preserve YAML frontmatter (`date`, `commit`, `tags`, `related_components`) for searchability

## PR Expectations

- keep changes coherent and scoped
- add tests for behavior changes
- run `just lint` and `just test` before code-changing commits
- run `just recipe-check` when touching recipe metadata or recipe loading/validation logic
- document non-obvious design choices in notes for future contributors
- call out platform caveats explicitly (for example GNU vs MSVC on Windows)
- when changing human-mode CLI output, add/update selective human-output tests (section/key-phrase assertions) and keep `--json` machine output compatibility intact

## Release Process

Release/versioning policy and the tagged GitHub release workflow are documented in:

- `/Users/autoparallel/Code/joy/docs/releasing.md`
- `/Users/autoparallel/Code/joy/.github/workflows/release.yaml`
- `/Users/autoparallel/Code/joy/packaging/README.md`
