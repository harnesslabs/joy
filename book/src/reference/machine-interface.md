# Machine Interface

`joy` supports machine-readable output for all commands via `--json` (alias `--machine`).

JSON output always uses one of two top-level envelopes:

```json
{
  "schema_version": "1",
  "joy_version": "<semver>",
  "ok": true,
  "command": "<name>",
  "data": { ... }
}
```

```json
{
  "schema_version": "1",
  "joy_version": "<semver>",
  "ok": false,
  "command": "<name>",
  "error": { "code": "<stable_code>", "message": "<text>" }
}
```

## Compatibility Policy

- Top-level envelope keys are stable: `ok`, `command`, and `data` or `error`.
- Existing command payload keys are additive-only within a given `schema_version`.
- `schema_version` changes only for intentional machine-interface contract revisions.
- `joy_version` reports the CLI build version that produced the envelope.
- Stable automation should key off `error.code`, not free-form `error.message`.
- Human-mode output may change for UX improvements; automation should use `--json`.

## Human Mode vs Machine Mode

- Human mode prioritizes readability and actionable guidance.
- Machine mode suppresses human formatting/progress and prints strict JSON to `stdout`.

## Selected Payload Additions (Recent Phases)

Project-scoped commands include additive workspace routing metadata:

- `workspace_root` (`string | null`)
- `workspace_member` (`string | null`)

`build` / `run` payloads include target-selection metadata:

- `target`
- `target_default`

For detailed field lists, inspect integration tests under `tests/` and command JSON payload builders in `src/commands/`.
