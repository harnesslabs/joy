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
- install `cmake` if you use compiled recipe-backed dependencies

## VSCode / clangd Cannot Find Dependency Headers

If you see include errors after `joy add` (for example `nlohmann/json.hpp` not found), verify:

1. `joy sync` or `joy build` has been run successfully after adding dependencies.
2. `compile_commands.json` exists at the project root.
3. `.joy/include/...` contains the expected dependency headers.

`joy` writes:

- project root `compile_commands.json`
- target-specific `.joy/build/compile_commands.<target>.json`

Use `joy doctor` to confirm artifact presence.

### clangd

clangd usually auto-discovers `compile_commands.json` from the workspace root. No extra config is required if the file exists.

### VSCode C/C++ Extension (cpptools)

If cpptools does not auto-detect the compile database, set `C_Cpp.default.compileCommands` to your project root `compile_commands.json`.

## Nested Dependencies Missing

If a package depends on another package (for example `harnesslabs/igneous -> xsimd/xsimd`) and the transitive dependency is missing:

- rerun `joy sync` to refresh lockfile + graph + header materialization
- run `joy tree` / `joy why <package>` to inspect the resolved graph
- run `joy doctor` to check lockfile freshness and dependency metadata coverage

If a registry package release embeds metadata and it disagrees with the fetched package manifest, `joy` returns `package_metadata_mismatch`. Refresh the registry index or fix the package metadata digest.

## Offline Cache Misses

If `joy --offline ...` fails, warm the cache online first:

```bash
joy sync
```

Then retry offline.

## Build Failures

When `joy build` fails, focus on compiler diagnostics first:

- the human error output points to the Ninja stderr/stdout log files
- inspect the file/line diagnostics reported by the compiler output
- rerun with `--json` if you need stable machine-readable envelopes around the error code/message

## Workspace Member Errors

If running from a workspace root, use `-p/--package <member>` unless `default_member` is configured.
