# Machine Interface

`joy` supports machine-readable output for all commands via `--json` (alias `--machine`).

JSON output always uses one of two top-level envelopes:

```json
{ "ok": true, "command": "<name>", "data": { ... } }
```

```json
{ "ok": false, "command": "<name>", "error": { "code": "<stable_code>", "message": "<text>" } }
```

## Compatibility Policy

- The top-level envelope keys are stable: `ok`, `command`, and `data` or `error`.
- Existing command payload keys are additive-only within the current roadmap phase series.
- New commands may introduce new payload shapes, but should preserve deterministic ordering where practical.
- Stable machine consumers should key off `error.code`, not free-form `error.message`.

## Dependency Command Payloads (Phase 11)

The following payload keys are covered by integration shape assertions in `/Users/autoparallel/Code/joy/tests/add_command.rs`.

### `joy add --json`

`data` keys:

- `package`
- `rev`
- `changed`
- `resolved_commit`
- `remote_url`
- `cache_source_dir`
- `cache_hit`
- `header_root`
- `header_link_path`
- `header_link_kind`
- `manifest_path`
- `project_root`
- `created_env_paths`
- `state_index_path`
- `warnings`

### `joy remove --json`

`data` keys:

- `package`
- `removed`
- `manifest_path`
- `project_root`
- `header_link_path`
- `header_link_removed`
- `state_index_path`
- `warnings`

### `joy update --json`

`data` keys:

- `project_root`
- `manifest_path`
- `state_index_path`
- `manifest_changed`
- `updated_count`
- `updated`
- `warnings`

Each `updated[]` item contains:

- `package`
- `rev`
- `resolved_commit`
- `cache_hit`
- `header_link_path`

### `joy tree --json`

`data` keys:

- `project_root`
- `manifest_path`
- `roots`
- `packages`

Each `packages[]` item contains:

- `id`
- `direct`
- `header_only`
- `requested_rev`
- `resolved_commit`
- `recipe`
- `deps`
