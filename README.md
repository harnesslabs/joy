# joy

Native C++ package and build manager with a `cargo`-like CLI (`new`, `add`, `build`, `run`) and a local `.joy/` environment.

`joy` currently supports:
- project scaffolding (`joy new`, `joy init`)
- manifest + local environment management (`joy.toml`, `.joy/`)
- header-only dependency fetching/install from GitHub shorthand IDs (for example `nlohmann/json`)
- recipe-backed compiled dependencies via CMake + Ninja (for example `fmtlib/fmt`, `madler/zlib`)
- machine-readable JSON output (`--json` / `--machine`)
- lockfile manifest-hash enforcement (`joy.lock`, `--locked`, `--update-lock`)

## Status / Caveats

This is an early but functional implementation of the Phase 1–6 roadmap milestone.

Current constraints:
- dependency versions are exact refs (`HEAD`, tag, branch, or commit SHA); there is no semver solver yet
- Windows support is currently optimized for MinGW (`x86_64-pc-windows-gnu`) first
- user project build support is currently a single entry translation unit (`src/main.cpp`)
- lockfile enforcement is implemented, but lockfile package population is still minimal (manifest-hash is the primary gate)

## Install / Build From Source

```bash
# Build the CLI
cargo build --workspace

# Or install it locally from this repo
cargo install --path /Users/autoparallel/Code/joy
```

You will also need host tools for full builds:
- a C++ compiler (`clang++` or `g++`; MinGW `g++` on Windows)
- `ninja`
- `cmake` (for compiled recipe-backed dependencies)
- `git` (for package fetching)

## Quickstart (New Project)

```bash
joy new hello_cpp
cd hello_cpp
joy run
```

Expected output (human mode):

```text
Hello from joy!
Ran `.../.joy/bin/hello_cpp` (exit 0)
```

## Add a Header-Only Dependency (`nlohmann/json`)

```bash
joy add nlohmann/json
```

What `joy add` currently does:
- validates `owner/repo`
- updates `joy.toml`
- creates local `.joy/` directories (if missing)
- fetches the dependency source into the global cache (`~/.joy/cache/...` or `JOY_HOME` override)
- installs headers into `.joy/include/deps/<slug>` (symlink preferred, copy fallback)

Example `src/main.cpp` using `nlohmann/json`:

```cpp
#include <iostream>
#include <nlohmann/json.hpp>

int main() {
  nlohmann::json payload = {
    {"project", "joy"},
    {"features", {"fetch", "build", "run"}}
  };

  std::cout << payload.dump(2) << std::endl;
  return 0;
}
```

Then build/run:

```bash
joy run
```

## Build and Run

```bash
joy build
joy run -- --example-flag value
```

`joy run` forwards everything after `--` to the compiled binary.

## Example `joy.toml`

```toml
[project]
name = "hello_cpp"
version = "0.1.0"
cpp_standard = "c++20"
entry = "src/main.cpp"

[dependencies]
"nlohmann/json" = { source = "github", rev = "HEAD" }
"fmtlib/fmt" = { source = "github", rev = "11.0.2" }
```

## Compiled Dependency Example (`fmtlib/fmt`)

`joy` ships a curated recipe index in `/Users/autoparallel/Code/joy/recipes` with recipe files under `/Users/autoparallel/Code/joy/recipes/packages`.

Included examples:
- `nlohmann/json` (header-only)
- `fmtlib/fmt` (CMake / compiled)
- `madler/zlib` (CMake / compiled)

Example:

```bash
joy add fmtlib/fmt
joy build
```

At build time, `joy` resolves recipe metadata, computes an ABI hash, builds into the global cache (`~/.joy/cache/builds/<abi_hash>`), and installs library artifacts into `.joy/lib` for linking.

## Machine Mode (`--json` / `--machine`)

All commands support machine-readable JSON output.

```bash
joy --json new demo_json
```

Example response (paths shortened):

```json
{
  "ok": true,
  "command": "new",
  "data": {
    "created_paths": [".../demo_json", ".../demo_json/src", ".../demo_json/joy.toml", ".../demo_json/src/main.cpp", ".../demo_json/.gitignore"],
    "overwritten_paths": [],
    "project_name": "demo_json",
    "project_root": ".../demo_json"
  }
}
```

Errors also use JSON envelopes (non-zero exit code still applies):

```json
{
  "ok": false,
  "command": "build",
  "error": {
    "code": "manifest_not_found",
    "message": "no `joy.toml` found at /path/to/joy.toml"
  }
}
```

## How It Works (High-Level)

1. `joy add` fetches and caches package sources, then installs headers into `.joy/include/deps`.
2. `joy build` loads `joy.toml`, enforces/updates `joy.lock`, discovers toolchain (`ninja` + compiler), and generates a Ninja build file.
3. For compiled dependencies, `joy` loads recipes, resolves a dependency DAG, prefetches sources, computes ABI hashes, builds with CMake+Ninja into the global cache, and installs libraries into `.joy/lib`.
4. `joy run` reuses the same build pipeline, then executes the produced binary.

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
just test
just ci      # lint + test
just clean
```

## Testing Notes

- Unit tests cover manifest parsing, recipes, resolver/DAG behavior, ABI hashing, linking, and cache management.
- Integration tests cover CLI scaffolding, `add`, `build`, `run`, and lockfile enforcement.
- Some tests are toolchain-dependent (compiler, Ninja, CMake) and skip with explicit messages when tools are unavailable.
- CI includes a compiled-dependency focused matrix job in `/Users/autoparallel/Code/joy/.github/workflows/check.yaml`.

## Current Limitations / Known Gaps

- No semver range solving (exact refs only)
- No first-class MSVC build path yet (detection exists; MinGW-first support)
- Single-translation-unit user build model for now (`src/main.cpp`)
- Lockfile package records are not fully populated yet
- Recipe set is intentionally small and curated in-repo

## Roadmap Milestone Archive

The completed Phase 1–6 execution notes and roadmap tracker are archived in:

- `/Users/autoparallel/Code/joy/notes/archive/2026-phase1-6-roadmap-delivered/MILESTONE.md`
- `/Users/autoparallel/Code/joy/notes/archive/2026-phase1-6-roadmap-delivered/source-notes/`

## Contributing

See `/Users/autoparallel/Code/joy/CONTRIBUTING.md` for development workflow and notes/archival conventions.
