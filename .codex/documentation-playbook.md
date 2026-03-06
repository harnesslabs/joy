# Joy Documentation Playbook

Use this playbook when the goal is to update user-facing docs, close mdBook drift, or deliver docs parity with shipped CLI behavior.

## Goals

- Keep `README.md` concise as the project entrypoint.
- Keep `book/` as the canonical detailed documentation.
- Ensure docs reflect shipped behavior on `main` and clearly label deferred/planned work.
- Maintain machine-interface and error-code references as stable automation contracts.

## Sources of Truth (Use in This Order)

1. `src/cli.rs` command surface, args, flags, subcommands
2. `src/commands/*.rs` runtime behavior and JSON payload fields
3. `src/manifest.rs`, `src/lockfile.rs`, `src/registry_config.rs` schema/config behavior
4. `tests/*.rs` integration expectations and examples
5. `notes/active/*.md` and `notes/roadmap.md` for roadmap context/deferred scope
6. `README.md`, `CHANGELOG.md`, and existing `book/src/**` copy for wording continuity

If docs text conflicts with source behavior, source code + tests win.

## Required Documentation Coverage

For a docs parity wave, ensure all shipped top-level commands are represented with examples:

- `version`, `new`, `init`, `add`, `remove`, `update`, `tree`, `why`, `outdated`
- `registry`, `search`, `info`
- `fetch`, `vendor`, `cache`
- `verify`
- `package`, `publish`, `owner`, `yank`
- `metadata`, `recipe-check`, `doctor`, `build`, `sync`, `run`

Grouped commands must include shipped subcommands and flags, not only parent command summaries.

## Authoring Rules

- Prefer task-first structure in workflow chapters, then reference detail.
- Keep examples copy/paste-ready and realistic.
- Avoid over-specific human-output screenshots/formatting unless that exact formatting is the subject.
- Use explicit status markers for non-shipped sections (`Planned`, `Deferred`, `Not shipped`).
- Never present roadmap intent as shipped behavior.
- Include concrete dates when stating roadmap/status snapshots.

## Machine Contract Documentation Rules

- Keep envelope contract explicit: `schema_version`, `joy_version`, `ok`, `command`, and `data|error`.
- Treat machine docs as additive contract docs: describe stable keys and compatibility policy.
- Update error-code catalog from emitted source codes (`JoyError::new`, `JoyError::io`, and mapped resolver/registry/toolchain/fetch errors).
- For command payload docs, prefer field tables grouped by command family.

## Recommended mdBook Layout

- `workflows/`: user task flows and command recipes
- `dependencies/`: dependency model and source backends
- `reference/`: schemas, machine contract, error catalog, command reference
- `status/`: shipped boundary + deferred/planned roadmap items
- `troubleshooting/`: failure-mode diagnosis and fixes

## Docs Change Workflow

1. Build a command coverage matrix from `joy --help` plus subcommand help.
2. Map each command/subcommand to an existing or new mdBook section.
3. Update `book/src/SUMMARY.md` before large content edits so structure is clear.
4. Apply workflow chapter updates.
5. Apply reference chapter updates (schemas, payloads, error codes).
6. Refresh status/troubleshooting and roadmap labels.
7. Run docs validation and fix all failures before PR.

## Validation Checklist

Run at minimum:

- `just docs-lint`

For larger docs waves, also run:

- `just ci-pr`

Before PR, confirm:

- every shipped command is documented with at least one example
- machine envelope docs match current behavior
- error-code docs include current emitted stable codes
- planned/deferred items are clearly labeled as non-shipped

## PR Checklist Template

Use this in docs PR descriptions:

```md
## Docs Parity Checklist
- [ ] Command coverage parity (`joy --help` + grouped subcommands)
- [ ] Workflow docs updated for shipped behavior
- [ ] Machine payload docs updated
- [ ] Error-code catalog updated from source
- [ ] Deferred/planned roadmap labels verified
- [ ] `just docs-lint` passing
- [ ] `just ci-pr` passing (or rationale recorded)
```

## Notes Integration

Record major docs waves in `notes/active/` with:

- Why
- What changed
- Validation snapshot
- Blockers & Solutions
- Fallback Plan

Move stale notes to `notes/archive/` when a wave is complete.
