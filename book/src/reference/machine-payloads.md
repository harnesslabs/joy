# Machine Payload Matrix

This chapter summarizes command-specific JSON payload keys emitted under the machine envelope.

For envelope compatibility policy, see [Machine Interface](machine-interface.md).

## Core Project Commands

### `new`

- `project_root`
- `manifest_path`
- `created_paths`

### `init`

- `project_root`
- `manifest_path`
- `created_paths`

### `add`

- `project_root`, `manifest_path`
- `dependency`, `source`, `registry`, `source_package`
- `fetched` (source fetch metadata when applicable)
- `installed` (header install metadata)
- `sync` (sync-lite result)

### `remove`

- `project_root`, `manifest_path`
- `dependency`
- `header_link_removed`
- `warnings`

### `update`

- `project_root`, `manifest_path`
- `updated_count`, `manifest_changed`
- `updated[]` entries with source/resolve/install metadata
- `warnings`

### `tree`

- `project_root`, `manifest_path`
- `roots[]`
- `packages[]` with source/provenance/dependency edges

### `why`

- `project_root`, `manifest_path`
- `package`
- `locked`
- `roots[]`, `paths[]`
- `package_info`

### `outdated`

- `project_root`, `manifest_path`, `lockfile_path`
- `roots[]`
- `sources`
- `summary` (direct/transitive/source counts)
- `packages[]`
- `outdated[]`

## Registry and Discovery

### `registry` (all subcommands)

- `action` (`add|remove|set-default`) for mutating subcommands
- `scope`
- `name`, `index`, `default_set`, `removed` (as applicable)
- `registries[]` and `default` for `list`

### `search`

- `query`
- `registry`
- `count`
- `packages[]` (`id`, `latest_version`)

### `info`

- `registry`
- `package`
- `latest_version`
- `versions[]`

## Dependency State, Integrity, and Cache

### `fetch`

- `project_root`, `manifest_path`
- `fetched_count`, `skipped_count`
- `fetched[]` with source/provenance/cache metadata

### `vendor`

- `project_root`, `lockfile_path`, `output_dir`
- `vendored_count`, `skipped_count`
- `vendored[]`, `skipped[]`

### `verify`

- `project_root`, `lockfile_path`
- `strict`
- `summary` (`package_count`, `passed_count`, `warning_count`, `failed_count`)
- `results[]`
- `sbom`
- `sbom_path`

### `cache` (`gc`)

- `action` (`gc`)
- `aggressive`
- `removed_paths[]`
- `cache_root`

### `recipe-check`

- `recipes_root`
- `recipe_count`
- `packages[]`

## Publishing and Ownership

### `package` (`init`)

- `action` (`init`)
- `manifest_path`
- `id`, `version`, `kind`

### `publish`

- `package`
- `version`
- `registry`
- `index_path`
- `git_committed`

### `owner`

- `action` (`list|add|remove`)
- `package`
- `registry`
- `owners[]` (for `list`)
- `changed` (for mutation flows)

### `yank`

- `package`
- `version`
- `registry`
- `yanked`
- `changed`
- `index_path`

## Build, Run, and Sync

### `build`

- `project_root`, `manifest_path`
- `build_file`, `binary_path`, `source_file`, `compiled_sources[]`
- `target`, `target_default`, `profile`
- `include_dirs[]`, `link_dirs[]`, `link_libs[]`
- `compiled_dependencies_built[]`
- `toolchain`
- `ninja_status`, `ninja_stdout`, `ninja_stderr`
- `lockfile_path`, `lockfile_updated`

### `sync`

- `project_root`, `manifest_path`
- `profile`
- `include_dirs[]`, `link_dirs[]`, `link_libs[]`
- `compiled_dependencies_built[]`
- `toolchain` (optional)
- `lockfile_path`, `lockfile_updated`

### `run`

- `project_root`, `binary_path`, `build_file`
- `toolchain`, `profile`, `target`, `target_default`
- `args[]`, `exit_code`
- `stdout`, `stderr`
- `lockfile_path`, `lockfile_updated`

## Diagnostics and Metadata

### `metadata`

- `project_root`, `manifest_path`
- `roots[]`
- `artifacts` (joy/state/build/compile-db paths + flags)
- `lockfile`
- `graph`
- `editor_extension_gate`

### `doctor`

- `ok`, `cwd`
- `env`, `tools`, `toolchain`, `cache`, `recipes`
- `project`, `artifacts`, `lockfile`, `dependency_metadata`
- `editor_extension_gate`
- `project_warnings[]`, `project_hints[]`

### `version`

- `joy_version`
- `schema_version`
- `build_target`
- `build_profile`
- `git_commit`

## Workspace Metadata Note

Project-scoped command payloads can include additive `workspace_root` and `workspace_member` keys when invoked from workspace-routed contexts.
