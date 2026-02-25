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
