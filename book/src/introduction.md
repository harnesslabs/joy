# Welcome

`joy` is a native C++ package and build manager with a CLI designed to feel familiar to teams that prefer `cargo`/`uv` style workflows.

Core characteristics:

- predictable command surface and lock-aware flows
- explicit dependency source semantics (`github`, `registry`, `git`, `path`, `archive`)
- reproducible workflows (`sync`, `--locked`, `--offline`, `--frozen`)
- machine-readable JSON output for all commands (`--json` / `--machine`)
- CLI-first developer ergonomics with objective editor integration gates

As of **March 6, 2026**, `joy` ships:

- scaffolding (`joy new`, `joy init`)
- dependency lifecycle (`add`, `remove`, `update`, `tree`, `why`, `outdated`)
- registry config/discovery (`registry`, `search`, `info`)
- offline/cache workflows (`fetch`, `vendor`, `cache gc`)
- supply-chain verification (`verify` + SBOM output)
- package publishing/ownership/yank workflows (`package`, `publish`, `owner`, `yank`)
- local builds and execution (`build`, `sync`, `run`) with workspace routing and named targets
- metadata and diagnostics (`metadata`, `doctor`, `recipe-check`, `version`)

## Who This Book Is For

- C++ developers who want faster reproducible local builds
- contributors tracking current `joy` behavior and boundaries
- automation authors integrating against stable JSON envelopes/error codes

## Documentation Strategy

- `README.md` is the overview and entrypoint.
- This mdBook is the canonical detailed usage/reference documentation.
- Planned/deferred features are explicitly labeled so expectations remain clear.
