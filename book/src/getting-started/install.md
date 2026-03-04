# Install

## Build From Source (All Platforms)

```bash
cargo build --workspace
cargo install --path .
```

## GitHub Release Binaries

`joy` publishes versioned release artifacts to GitHub Releases.

Current artifact set (subject to release workflow updates):

- `joy-vX.Y.Z-x86_64-unknown-linux-gnu.tar.gz`
- `joy-vX.Y.Z-aarch64-apple-darwin.tar.gz`
- `joy-vX.Y.Z-x86_64-apple-darwin.tar.gz`
- `joy-vX.Y.Z-x86_64-pc-windows-msvc.zip`
- `joy-vX.Y.Z-x86_64-pc-windows-gnu.zip` (compatibility artifact)

## Homebrew / Scoop Metadata

This repository includes release-managed package metadata templates:

- `packaging/homebrew/joy.rb`
- `packaging/scoop/joy.json`

Release workflows publish generated package-manager metadata with concrete checksums as release artifacts.

See the [Releasing reference](../reference/releasing.md) for the exact release and packaging workflow.

## Required Host Tools (for `joy build` / `joy run`)

You need host tools for local C++ builds and compiled recipe-backed dependencies:

- C++ compiler (`clang++`, `g++`, or `cl.exe` on Windows MSVC)
- `ninja`
- `cmake` (for compiled recipes)
- `git`

Use `joy doctor` to inspect tool availability and cache/recipe health.
