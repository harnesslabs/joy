# Machine Interface

`joy` supports machine-readable output for all commands via `--json` (alias `--machine`).

JSON output always uses one of two top-level envelopes:

```json
{
  "schema_version": "1",
  "joy_version": "<semver>",
  "ok": true,
  "command": "<name>",
  "data": { "...": "..." }
}
```

```json
{
  "schema_version": "1",
  "joy_version": "<semver>",
  "ok": false,
  "command": "<name>",
  "error": {
    "code": "<stable_code>",
    "message": "<human_text>"
  }
}
```

## Compatibility Policy

- Top-level envelope keys are stable: `schema_version`, `joy_version`, `ok`, `command`, plus `data` or `error`.
- Existing payload keys are additive within a fixed `schema_version`.
- `schema_version` changes only for explicit contract revisions.
- `error.code` is the automation key; `error.message` is human-oriented and may evolve.
- Human output is not a machine contract; automation should use `--json`.

## Workspace Routing Metadata

Project-scoped command payloads may include additive workspace fields when invoked from workspace root routing:

- `workspace_root` (`string | null`)
- `workspace_member` (`string | null`)

## Build/Run/Sync Target Metadata

`build` and `run` include target metadata:

- `target`
- `target_default`

`build`, `sync`, and `run` include lockfile metadata:

- `lockfile_path`
- `lockfile_updated`

## Machine Payload Catalog

See [Machine Payload Matrix](machine-payloads.md) for command-specific key maps.

## Error Code Catalog

See [Error Codes](error-codes.md) for stable machine error keys.
