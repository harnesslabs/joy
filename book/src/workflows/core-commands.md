# Core Commands

This chapter covers the most common day-to-day `joy` command flow, including command families added after the original docs wave.

## Project Scaffolding

Create a new project:

```bash
joy new demo
```

Initialize the current directory:

```bash
joy init
```

## Dependency Management

Add dependencies from different sources:

```bash
joy add nlohmann/json
joy add registry:nlohmann/json --version ^3
joy add git:https://github.com/fmtlib/fmt.git --as fmtlib/fmt --rev 11.0.2
joy add path:vendor/localdep --as localdep
joy add archive:https://example.com/libfoo.tar.gz --as acme/libfoo --sha256 <sha256>
```

Update dependencies:

```bash
joy update
joy update fmtlib/fmt --rev 11.1.0
joy update registry:nlohmann/json --version ^3
```

Remove dependencies:

```bash
joy remove fmtlib/fmt
```

Inspect the graph:

```bash
joy tree
joy tree --locked
joy why nlohmann/json
joy why nlohmann/json --locked
joy outdated
joy outdated --sources registry
joy outdated --sources github
joy outdated --sources git
joy outdated --sources path
joy outdated --sources archive
```

## Registry Discovery and Setup

Configure and inspect registries:

```bash
joy registry list
joy registry add internal file:///path/to/internal-index.git --default
joy registry set-default internal
joy registry list --project
```

Search and inspect package metadata:

```bash
joy search json --registry internal --limit 20
joy info nlohmann/json --registry internal
```

## Build, Sync, and Run

Build and run:

```bash
joy build
joy sync
joy run
joy run -- --app-arg value
```

Target-specific execution:

```bash
joy build --target tool
joy run --target tool
```

Lockfile controls:

```bash
joy build --locked
joy sync --update-lock
joy --frozen build
```

## Metadata, Diagnostics, and Validation

```bash
joy metadata
joy doctor
joy recipe-check
joy verify
joy verify --strict --sbom sbom.json
```

`joy metadata` and `joy doctor` expose compile-db/graph/lockfile state and editor-gate diagnostics for CLI-first workflows.

## Cache and Offline Preparation

```bash
joy fetch
joy vendor
joy vendor --output third_party/vendor
joy cache gc
joy cache gc --aggressive
```

## Package Publishing Workflow

```bash
joy package init acme/widgets --kind header-only --version 0.1.0
joy publish --registry internal --rev v0.1.0
joy owner add acme/widgets alice --registry internal
joy owner list acme/widgets --registry internal
joy yank acme/widgets --version 0.1.0 --registry internal
joy yank acme/widgets --version 0.1.0 --undo --registry internal
```

## Version Metadata

```bash
joy version
joy --json version
```

## JSON / Machine Mode

Use `--json` (alias `--machine`) for stable automation envelopes:

```bash
joy --json tree
joy --json doctor
joy --json publish --registry internal
```

See [Machine Interface](../reference/machine-interface.md) and [Machine Payload Matrix](../reference/machine-payloads.md).
