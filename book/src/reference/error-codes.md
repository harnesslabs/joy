# Error Codes

`joy` command failures return machine-readable error codes via the JSON error envelope.

```json
{
  "ok": false,
  "command": "build",
  "error": {
    "code": "manifest_not_found",
    "message": "..."
  }
}
```

## Stability Policy

- `error.code` is the stable machine key for automation.
- `error.message` is human-facing and may change while preserving semantics.
- New codes may be added additively.

## Core / CLI

- `cli_parse_error`
- `cwd_unavailable`
- `io_error`
- `output_serialize_failed`
- `not_implemented`

## Manifest / Project Scaffolding

- `manifest_not_found`
- `manifest_parse_error`
- `manifest_write_error`
- `invalid_target`
- `non_empty_directory`
- `path_exists`
- `invalid_package_id`
- `dependency_not_found`
- `workspace_member_invalid`
- `workspace_member_required`
- `workspace_member_not_found`

## Lockfile / Reproducibility

- `invalid_lock_flags`
- `lockfile_missing`
- `lockfile_stale`
- `lockfile_incomplete`
- `lockfile_mismatch`
- `lockfile_parse_error`
- `lockfile_hash_failed`
- `lockfile_write_failed`

## Fetch / Network / Offline

- `fetch_failed`
- `offline_cache_miss`
- `offline_network_disabled`
- `invalid_version_requirement`
- `version_not_found`
- `registry_not_configured`
- `registry_load_failed`
- `registry_package_not_found`
- `registry_alias_unsupported`

## Toolchain / Build Execution

- `toolchain_not_found`
- `toolchain_probe_failed`
- `env_setup_failed`
- `recipe_load_failed`
- `dependency_resolve_failed`
- `dependency_graph_invalid`
- `missing_recipe`
- `missing_cmake_metadata`
- `missing_link_metadata`
- `cache_setup_failed`
- `cmake_build_failed`
- `library_install_failed`
- `header_install_failed`
- `ninja_file_write_failed`
- `ninja_spawn_failed`
- `build_failed`
- `entry_not_found`
- `source_not_found`
- `include_dir_not_found`
- `state_index_error`
- `state_cleanup_failed`
- `lockfile_package_assembly_failed`

## Command-Specific Lifecycle / Policy

### `add`

- `frozen_disallows_add`
- `invalid_add_args`

### `remove`

- `frozen_disallows_remove`

### `update`

- `frozen_disallows_update`
- `invalid_update_args`
- `unsupported_dependency_source`

### `recipe-check`

- `recipe_validation_failed`

### `run`

- `run_spawn_failed`
- `run_failed`
