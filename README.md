# joy

Native C++ package and build manager with a `cargo`-like CLI and a local project environment (`.joy/`).

`joy` currently supports:

- project scaffolding (`joy new`, `joy init`)
- dependency management (`joy add`, `joy remove`, `joy update`, `joy tree`)
- reproducible dependency materialization (`joy sync`, `--locked`, `--offline`, `--frozen`)
- header-only and recipe-backed compiled dependencies (CMake + Ninja)
- multi-file project builds (`project.entry`, `project.extra_sources`, `project.include_dirs`)
- machine-readable JSON output (`--json` / `--machine`)
- diagnostics and metadata validation (`joy doctor`, `joy recipe-check`)

## Status / Caveats

`joy` is a functional pre-1.0 implementation that has completed Phases 7-18 of the current roadmap wave.

Current constraints:

- direct dependencies support exact refs (`rev`) or semver ranges (`version`) resolved from Git tags
- transitive recipe dependencies remain exact-rev metadata in the current phase
- Windows local builds are supported via both MinGW GNU and MSVC (`cl.exe` + Ninja)
- GitHub release artifacts currently publish the Windows GNU target (`x86_64-pc-windows-gnu`)
- registry/index support is available in a git-backed default-registry mode (configured via `JOY_REGISTRY_DEFAULT`)
- registry package aliases (registry name != canonical source package ID) are intentionally deferred; the initial registry cut keeps canonical `owner/repo` IDs
- package-manager channels (Homebrew tap / Scoop bucket) are template-driven and release-managed

## Install

### Build From Source (All Platforms)

```bash
cargo build --workspace
cargo install --path /Users/autoparallel/Code/joy
```

### GitHub Release Binaries (Recommended for end users)

Phase 14 adds a tagged release workflow that publishes versioned artifacts to GitHub Releases.

Artifact naming:

- `joy-vX.Y.Z-x86_64-unknown-linux-gnu.tar.gz`
- `joy-vX.Y.Z-aarch64-apple-darwin.tar.gz`
- `joy-vX.Y.Z-x86_64-pc-windows-gnu.zip`

Release process details: `/Users/autoparallel/Code/joy/docs/releasing.md`

### Homebrew / Scoop Metadata (Template-Based)

This repo now includes package-manager metadata templates:

- Homebrew formula template: `/Users/autoparallel/Code/joy/packaging/homebrew/joy.rb`
- Scoop manifest template: `/Users/autoparallel/Code/joy/packaging/scoop/joy.json`
- Packaging notes: `/Users/autoparallel/Code/joy/packaging/README.md`

After updating the template version + SHA256 values for a published release, you can test locally:

```bash
# macOS/Linux (Homebrew installed)
brew install --formula ./packaging/homebrew/joy.rb

# Windows (Scoop installed)
scoop install .\packaging\scoop\joy.json
```

## Host Tools (for `joy build` / `joy run`)

You will need host tools for local C++ builds and compiled recipe dependencies:

- a C++ compiler (`clang++` or `g++`; MinGW `g++` on Windows)
- `ninja`
- `cmake` (for compiled recipe-backed dependencies)
- `git` (for package fetching)

Use `joy doctor` to inspect tool availability and local cache/recipe health.

## Quickstart

```bash
joy new hello_cpp
cd hello_cpp
joy add nlohmann/json
joy run
joy tree
```

Expected run output (human mode):

```text
Hello from joy!
Ran `.../.joy/bin/hello_cpp` (exit 0)
```

## Reproducible Workflows (`sync`, `--locked`, `--offline`, `--frozen`)

```bash
# Refresh dependency/cache/materialized state without compiling the app
joy sync

# CI-safe mode: locked + offline (no lockfile writes, no network)
joy --frozen build

# Warm-cache local rebuild with no network access
joy --offline run
```

Notes:

- `--frozen` implies `--offline` and `--locked`
- `--offline` fails with a stable machine error when a dependency is missing from cache
- `joy sync` refreshes `.joy/` and `joy.lock` without compiling the final project binary

## Dependency Commands

```bash
joy add fmtlib/fmt --rev 11.0.2
joy add fmtlib/fmt --version ^11
joy add registry:fmtlib/fmt --version ^11
joy update fmtlib/fmt --rev 11.1.0
joy update fmtlib/fmt --version ^11
joy update fmtlib/fmt          # refreshes stored semver range (github or registry source)
joy remove fmtlib/fmt
joy tree --json
```

`joy tree` reports the resolved dependency graph (human or JSON mode) using deterministic ordering.

For semver-managed direct dependencies, `joy` stores the requested range in `joy.toml` and records the selected tag/version/commit in `joy.lock`.

## Registry Index (Phase 18)

`joy` can resolve direct dependencies from a git-backed registry index while still fetching package sources from GitHub.

- Use `registry:<owner/repo>` to add a dependency through the default registry.
- Configure the default registry remote with `JOY_REGISTRY_DEFAULT` (git URL or local git repo path).
- Registry dependencies currently require `--version <range>` and are stored as:
  - `source = "registry"`
  - `version = "..."` in `joy.toml`

Example:

```bash
export JOY_REGISTRY_DEFAULT=/path/to/joy-registry.git
joy add registry:nlohmann/json --version ^3
joy tree --json
```

## Multi-File Project Builds

`joy` now supports multi-file user projects while keeping `project.entry` as the required entry point.

```toml
[project]
name = "hello_cpp"
version = "0.1.0"
cpp_standard = "c++20"
entry = "src/main.cpp"
extra_sources = ["src/lib/math.cpp", "src/feature/print.cpp"]
include_dirs = ["include"]

[dependencies]
"nlohmann/json" = { source = "github", rev = "HEAD" }
"fmtlib/fmt" = { source = "github", rev = "11.0.2" }
```

## Multiple Targets (Phase 16)

Projects can define additional named binary targets using `[[project.targets]]` and select them with `joy build --target <name>` / `joy run --target <name>`.

```toml
[project]
name = "demo"
version = "0.1.0"
cpp_standard = "c++20"
entry = "src/main.cpp"

[[project.targets]]
name = "tool"
entry = "src/tool.cpp"
```

```bash
joy build --target tool
joy run --target tool
```

## Workspaces (Phase 16)

`joy` supports a workspace root manifest with member projects. Run project-scoped commands from the workspace root using `-p/--package <member>`.

Workspace root `joy.toml`:

```toml
[workspace]
members = ["apps/app", "tools/tooling"]
default_member = "apps/app" # optional
```

Examples:

```bash
joy -p apps/app tree
joy -p apps/app build
joy -p apps/app run --target tool
```

## Compiled Dependency Recipes

`joy` ships a curated recipe index in `/Users/autoparallel/Code/joy/recipes` with recipe files under `/Users/autoparallel/Code/joy/recipes/packages`.

Examples currently include header-only and compiled packages such as:

- `nlohmann/json`
- `fmtlib/fmt`
- `madler/zlib`
- `gabime/spdlog`
- `Neargye/magic_enum`
- `jarro2783/cxxopts`

Use `joy recipe-check` to validate bundled recipe metadata locally or in CI.

## Machine Mode (`--json` / `--machine`)

All commands support machine-readable JSON output.

```bash
joy --json doctor
joy --json tree
joy --json build
```

Machine interface docs:

- `/Users/autoparallel/Code/joy/docs/machine-interface.md`
- `/Users/autoparallel/Code/joy/docs/error-codes.md`

The JSON envelope shape is stable and additive for existing commands in the current roadmap wave.

## Platform Support Matrix (Current)

| Platform | Status | Notes |
| --- | --- | --- |
| Linux (x86_64 GNU) | Supported | CI build/lint/test + release artifact |
| macOS (Apple Silicon) | Supported | CI build/lint/test + release artifact |
| Windows (MinGW GNU) | Supported | CI build/lint/test + release artifact |
| Windows (MSVC) | Supported | CI build/lint/test + compiled-e2e; release artifact currently GNU-only |

## How It Works (High-Level)

1. `joy add` / `joy update` fetch and cache package sources, then install headers into `.joy/include/deps` as needed.
2. `joy sync` and `joy build` load `joy.toml`, resolve dependency metadata, enforce/refresh `joy.lock`, and materialize dependency state.
3. For compiled dependencies, `joy` loads recipes, resolves a dependency DAG, computes ABI hashes, builds with CMake+Ninja into the global cache, and installs artifacts into `.joy/lib`.
4. `joy build` generates a Ninja file for the user project and compiles/links the final binary.
5. `joy run` reuses the same build pipeline, then executes the produced binary.

## Project Layout

Project-local files (created lazily):

```text
<project>/
  joy.toml
  joy.lock
  src/main.cpp
  .joy/
    include/
      deps/
    lib/
    build/
      build.ninja
      obj/
    bin/
    state/
      install-index.json
```

Global cache (default `~/.joy`, overridable with `JOY_HOME`):

```text
~/.joy/cache/
  src/
  git/
  archives/
  builds/
  tmp/
```

## Development Workflow

From `/Users/autoparallel/Code/joy`:

```bash
just build
just lint
just recipe-check
just test
just ci
joy --json doctor
```

Release and distribution process docs:

- `/Users/autoparallel/Code/joy/docs/releasing.md`
- `/Users/autoparallel/Code/joy/packaging/README.md`

## Current Limitations / Next Big Gaps

- No semver range solving (exact refs only)
- Windows release artifacts are GNU-only for now (MSVC build/test support exists)
- Registry/index support beyond GitHub shorthand is not implemented yet
- Package-manager channels are template-driven until dedicated tap/bucket repos are maintained

## Roadmap Milestones and Notes

- Active roadmap tracker: `/Users/autoparallel/Code/joy/notes/roadmap.md`
- Completed Phase 1-6 milestone archive:
  - `/Users/autoparallel/Code/joy/notes/archive/2026-phase1-6-roadmap-delivered/MILESTONE.md`
  - `/Users/autoparallel/Code/joy/notes/archive/2026-phase1-6-roadmap-delivered/source-notes/`

## Contributing

See `/Users/autoparallel/Code/joy/CONTRIBUTING.md` for development workflow, release process pointers, and notes/archival conventions.
