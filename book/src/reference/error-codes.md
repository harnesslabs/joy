# Error Codes

`joy` command failures return machine-readable error codes via JSON envelopes.

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

- `error.code` is the stable automation key.
- `error.message` is human text and may evolve.
- New codes may be introduced additively.

## Core / CLI

- `cli_parse_error`
- `cwd_unavailable`
- `io_error`
- `output_serialize_failed`
- `not_implemented`

## Manifest / Workspace / Package Identity

- `manifest_not_found`
- `manifest_parse_error`
- `manifest_write_error`
- `manifest_exists`
- `manifest_hash_failed`
- `non_empty_directory`
- `path_exists`
- `invalid_package_id`
- `invalid_package_version`
- `dependency_not_found`
- `workspace_member_invalid`
- `workspace_member_required`
- `workspace_member_not_found`

## Dependency Input / Source Validation

- `invalid_add_args`
- `invalid_update_args`
- `invalid_dependency_source`
- `source_backend_unsupported`
- `registry_alias_unsupported`
- `package_metadata_mismatch`
- `dependency_resolve_failed`

## Lockfile / Reproducibility

- `invalid_lock_flags`
- `lockfile_not_found`
- `lockfile_missing`
- `lockfile_parse_error`
- `lockfile_hash_failed`
- `lockfile_stale`
- `lockfile_incomplete`
- `lockfile_mismatch`
- `lockfile_write_failed`
- `lockfile_package_assembly_failed`

## Fetch / Network / Registry

- `fetch_failed`
- `offline_cache_miss`
- `offline_network_disabled`
- `invalid_version_requirement`
- `version_not_found`
- `invalid_checksum`
- `checksum_mismatch`
- `archive_format_unsupported`
- `registry_not_configured`
- `registry_load_failed`
- `registry_package_not_found`
- `registry_config_error`
- `registry_index_parse_error`
- `registry_index_serialize_failed`
- `registry_index_unsupported_version`

## Git / Registry Transport

- `git_failed`
- `registry_transport_failed`
- `registry_auth_failed`

## Toolchain / Build / Graph Materialization

- `toolchain_not_found`
- `toolchain_probe_failed`
- `env_setup_failed`
- `cache_setup_failed`
- `recipe_load_failed`
- `recipe_validation_failed`
- `dependency_graph_invalid`
- `missing_recipe`
- `missing_cmake_metadata`
- `missing_link_metadata`
- `generic_cmake_no_libraries`
- `cmake_build_failed`
- `library_install_failed`
- `header_install_failed`
- `include_dir_not_found`
- `entry_not_found`
- `source_not_found`
- `build_failed`
- `ninja_file_write_failed`
- `ninja_spawn_failed`
- `compile_db_write_failed`
- `state_index_error`
- `state_cleanup_failed`
- `state_graph_error`

## Command Policy / Lifecycle

- `frozen_disallows_add`
- `frozen_disallows_remove`
- `frozen_disallows_update`
- `add_sync_failed`
- `run_spawn_failed`
- `run_failed`
- `publish_version_exists`
- `invalid_owner`

## Verify / SBOM / Vendor

- `verify_failed`
- `verify_scan_failed`
- `verify_hash_path_missing`
- `sbom_serialize_failed`
- `vendor_copy_failed`
