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
- Human-mode stdout/stderr formatting is intentionally allowed to improve over time; automation should use `--json`.
- See `/Users/autoparallel/Code/joy/docs/error-codes.md` for the current documented error-code catalog (grounded in the `JoyError` machine contract defined in `/Users/autoparallel/Code/joy/src/error.rs`).

## Human Mode vs Machine Mode

- Default human output prioritizes readability and actionable guidance (warnings, hints, progress lines).
- Human-mode strings may be reorganized or reworded between releases while preserving semantics.
- `--json` / `--machine` mode is the stable automation interface and should be used by tools, scripts, and agents.
- Human-mode regressions should be tested selectively (section presence / key phrases), not by freezing every full output string.

## Dependency Command Payloads (Phase 11 + Phase 16/17 additive fields)

Phase 16 adds additive workspace-routing metadata to project-scoped command payloads:

- `workspace_root` (string or `null`)
- `workspace_member` (string or `null`)

The following payload keys are covered by integration shape assertions in `/Users/autoparallel/Code/joy/tests/add_command.rs`.

### `joy add --json`

`data` keys:

- `package`
- `source` (`"github"` or `"registry"`)
- `registry` (string or `null`; current default registry name when `source = "registry"`)
- `source_package` (string or `null`; upstream source package coordinate)
- `rev`
- `requested_requirement` (string or `null`; semver range for direct semver dependencies)
- `resolved_version` (string or `null`; concrete semver selected from tags)
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
- `source`
- `registry` (string or `null`)
- `source_package` (string or `null`)
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
- `source` (`"github"` or `"registry"`)
- `registry` (string or `null`)
- `source_package` (string or `null`)
- `rev`
- `requested_requirement` (string or `null`)
- `resolved_version` (string or `null`)
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
- `source` (`"github"` or `"registry"`)
- `registry` (string or `null`)
- `source_package` (string or `null`)
- `direct`
- `header_only`
- `requested_rev`
- `requested_requirement` (string or `null`)
- `resolved_version` (string or `null`)
- `resolved_commit`
- `recipe`
- `deps`

## `joy build --json` / `joy run --json` (Phase 16 additive fields)

Selected additive `data` fields:

- `target` (selected target name; default target uses the project name)
- `target_default` (`true` for the top-level `[project]` target)
- `workspace_root` (string or `null`)
- `workspace_member` (string or `null`)

## `joy doctor --json` (Phase 13)

`data` keys:

- `ok`
- `cwd`
- `env`
- `tools`
- `toolchain`
- `cache`
- `recipes`

Selected nested fields:

- `env.path_present`
- `env.joy_home`
- `tools.<tool>.ok`
- `tools.<tool>.path`
- `toolchain.ok`
- `toolchain.compiler_kind` (when available)
- `cache.ok`
- `cache.root`
- `recipes.ok`
- `recipes.recipe_count`
