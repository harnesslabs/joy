# Welcome

`joy` is a native C++ package and build manager with a CLI designed to feel familiar to people who like `cargo` and `uv`:

- predictable commands
- reproducible workflows (`sync`, `--locked`, `--offline`, `--frozen`)
- machine-readable JSON output for automation
- human-friendly defaults for local development

`joy` is still pre-1.0, but it already supports:

- project scaffolding (`joy new`, `joy init`)
- dependency management (`joy add`, `joy remove`, `joy update`, `joy tree`)
- local C++ builds via Ninja (and CMake for compiled recipe-backed dependencies)
- workspaces and named targets
- a git-backed registry index mode for direct dependencies

This book is the primary usage documentation for `joy`.

## Who This Book Is For

- C++ developers who want a faster path from project creation to reproducible local builds
- contributors evaluating how `joy` works today
- automation authors integrating with `joy --json`

## Documentation Strategy

- The `README` is now the overview and entrypoint.
- This mdBook contains the detailed workflows and reference material.
- Deferred/incomplete features are documented explicitly so expectations stay clear.
