# joy

`joy` is a native C++ package and build manager with a `cargo`/`uv`-inspired CLI.

It aims to make common C++ workflows feel predictable and reproducible:

- create projects quickly
- add dependencies with clear source semantics
- build/run with a local project environment (`.joy/`)
- use lockfiles and offline/frozen modes for repeatability
- automate via stable JSON output (`--json` / `--machine`)

## Read the Book

The full user guide and reference docs live in the mdBook:

- Live docs (custom domain): <https://joy.harnesslabs.dev/>
- GitHub Pages fallback URL: <https://harnesslabs.github.io/joy/>
- Local docs source: [`book/`](book/)

## Status (Pre-1.0)

`joy` is functional and actively developed. It currently supports:

- scaffolding (`joy new`, `joy init`)
- dependency management (`joy add`, `joy remove`, `joy update`, `joy tree`)
- reproducible workflows (`joy sync`, `--locked`, `--offline`, `--frozen`)
- compiled recipe-backed dependencies (CMake + Ninja)
- multi-file builds, named targets, and workspace member routing
- a git-backed registry index mode for direct dependencies
- machine-readable JSON output for all commands

Important current limitations are documented in the book's status/deferred pages.

## Quickstart

```bash
joy new hello_cpp
cd hello_cpp
joy add nlohmann/json
joy run
joy tree
```

## Install

### Build from source

```bash
cargo build --workspace
cargo install --path .
```

### Release binaries and packaging metadata

`joy` publishes GitHub release artifacts and includes package metadata templates for:

- Homebrew (`packaging/homebrew/joy.rb`)
- Scoop (`packaging/scoop/joy.json`)

See the docs book for installation and release details.

## Host Tools (for `joy build` / `joy run`)

You need:

- a C++ compiler (`clang++`, `g++`, or `cl.exe` on Windows MSVC)
- `ninja`
- `cmake` (for compiled dependencies)
- `git`

Check your environment with:

```bash
joy doctor
```

## Why `joy`?

`joy` is trying to bring a better default developer experience to native C++ projects:

- deterministic workflows first
- clear machine and human interfaces
- practical local builds (Ninja/CMake) without hiding what's happening
- incremental ergonomics inspired by tools like `cargo` and `uv`

## Machine Interface

Automation should use `--json` / `--machine` and rely on stable `error.code` values.

Reference docs live in the book and legacy stubs under [`docs/`](docs/).

JSON envelopes include:

- `schema_version`
- `joy_version`
- `ok`
- `command`
- `data` or `error`

Use `joy --json version` for build/version metadata.

## License

MIT. See [`LICENSE`](LICENSE).

## Development

Contributor docs and workflow notes:

- [`CONTRIBUTING.md`](CONTRIBUTING.md)
- [`notes/`](notes/)
- [`justfile`](justfile)
