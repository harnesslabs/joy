# Common Failures

## `manifest_not_found`

You ran a project-scoped command outside a `joy` project directory.

Fix:

- run `joy init` in the current directory, or
- run `joy new <name>` and move into the created project

## Toolchain / Ninja Not Found

Use `joy doctor` to inspect compiler and `ninja` availability.

Common fixes:

- install a compiler (`clang++`, `g++`, or MSVC toolchain)
- install `ninja`
- install `cmake` if you use compiled dependencies

## Workspace Member Errors

If running from a workspace root, use `-p/--workspace-package <member>` unless `default_member` is configured.

Common related codes:

- `workspace_member_required`
- `workspace_member_not_found`
- `workspace_member_invalid`

## Dependency Source Validation Errors

If `joy add`/`joy update` fails with source-validation style errors:

- verify source-specific argument contract:
  - `registry`: requires `--version`
  - `git`: requires `--rev`
  - `archive`: requires `--sha256`
- verify prefixes are correct (`registry:`, `git:`, `path:`, `archive:`)
- use `--json` and inspect `error.code` (`invalid_add_args`, `invalid_update_args`, `invalid_dependency_source`)

## Registry Configuration / Lookup Failures

If registry flows fail:

- verify registry exists with `joy registry list`
- ensure expected default registry is set (`joy registry set-default <name>`)
- ensure scope is correct (`--project` vs user default scope)
- re-run in `--json` mode and inspect codes such as:
  - `registry_not_configured`
  - `registry_config_error`
  - `registry_load_failed`
  - `registry_package_not_found`

## VSCode / clangd Cannot Find Dependency Headers

If you see include errors after `joy add`, verify:

1. `joy sync` or `joy build` has been run after adding dependencies.
2. `compile_commands.json` exists at project root.
3. `.joy/include/...` contains expected dependency headers.

`joy` writes:

- project root `compile_commands.json`
- target-specific `.joy/build/compile_commands.<target>.json`

Use `joy doctor` and `joy metadata` to inspect compile-db and graph artifact state.

### clangd

clangd usually auto-discovers `compile_commands.json` from workspace root.

### VSCode C/C++ Extension (cpptools)

If cpptools does not auto-detect the compile database, set `C_Cpp.default.compileCommands` to project-root `compile_commands.json`.

## Nested Dependencies Missing

If transitive dependencies appear missing:

- rerun `joy sync` to refresh lockfile + graph + materialization
- run `joy tree` / `joy why <package>`
- inspect `joy metadata` for graph/lockfile state

If package metadata digests disagree between sources, expect `package_metadata_mismatch` until metadata is reconciled.

## Offline Cache Misses

If `joy --offline ...` fails:

```bash
joy fetch
joy sync
```

Then retry offline/frozen commands.

## `lockfile_not_found` (`verify` / `vendor`)

`joy verify` and `joy vendor` require `joy.lock`.

Fix:

```bash
joy sync
# or
joy sync --update-lock
```

## Verification Failures (`verify_failed`)

When `joy verify` fails:

- inspect failing package rows in JSON `data.results`
- check source provenance fields in `joy.lock`
- for archive sources, ensure `source_checksum_sha256` is present and valid
- if vendored source exists, compare against resolved source checksum

## Publishing Transport / Auth Failures

If publish workflows fail against remote registry URLs:

- confirm registry URL is reachable and writable
- confirm git credentials/token permissions for push
- inspect JSON error code:
  - `registry_transport_failed`
  - `registry_auth_failed`

## Build Failures

When `joy build` fails, focus on compiler diagnostics first:

- inspect ninja/compiler stderr/stdout details
- inspect generated build graph and compile DB paths in JSON payload
- use `--json` for stable failure envelopes
