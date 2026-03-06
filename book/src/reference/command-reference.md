# Command Reference

This chapter maps shipped `joy` commands and grouped subcommands.

Use workflow chapters for task-first guidance and this chapter for quick command/flag lookup.

## Global Flags

Global flags apply before subcommands:

```bash
joy --json <command>
joy --offline <command>
joy --frozen <command>
joy -p <workspace-member> <command>
```

Human UX controls:

```bash
joy --color auto|always|never <command>
joy --progress auto|always|never <command>
joy --glyphs auto|unicode|ascii <command>
```

## Top-Level Commands

### Project Scaffolding

- `joy new <name> [--force]`
- `joy init [--force]`

### Dependency Lifecycle

- `joy add <package> [--as <name>] [--rev <rev>] [--version <range>] [--registry <name>] [--sha256 <sum>] [--no-sync]`
- `joy remove <package>`
- `joy update [<package>] [--rev <rev>] [--version <range>] [--registry <name>] [--sha256 <sum>]`
- `joy tree [--locked]`
- `joy why <package> [--locked]`
- `joy outdated [--sources all|registry|github|git|path|archive]`

### Registry Discovery

- `joy registry <subcommand>`
- `joy search <query> [--registry <name>] [--limit <n>]`
- `joy info <package> [--registry <name>]`

### Dependency State / Integrity

- `joy fetch`
- `joy vendor [--output <dir>]`
- `joy verify [--strict] [--sbom <path>]`
- `joy cache gc [--aggressive]`
- `joy recipe-check`

### Publishing + Ownership

- `joy package init <id> [--version <semver>] [--kind header-only|cmake] [--force]`
- `joy publish [--registry <name>] [--rev <rev>] [--source-package <id>]`
- `joy owner <subcommand>`
- `joy yank <package> --version <semver> [--undo] [--registry <name>]`

### Build and Execution

- `joy build [--release] [--target <name>] [--locked] [--update-lock]`
- `joy sync [--release] [--locked] [--update-lock]`
- `joy run [--release] [--target <name>] [--locked] [--update-lock] [-- <args...>]`

### Diagnostics and Metadata

- `joy metadata`
- `joy doctor`
- `joy version`

## Grouped Subcommands

### `joy registry`

- `list [--project]`
- `add <name> <index> [--default] [--project]`
- `remove <name> [--project]`
- `set-default <name> [--project]`

### `joy owner`

- `list <package> [--registry <name>]`
- `add <package> <owner> [--registry <name>]`
- `remove <package> <owner> [--registry <name>]`

### `joy package`

- `init <id> [--version <semver>] [--kind header-only|cmake] [--force]`

### `joy cache`

- `gc [--aggressive]`

## Minimal Example Sweep

```bash
joy new demo
joy add nlohmann/json
joy tree
joy why nlohmann/json
joy registry list
joy search json
joy fetch
joy vendor
joy verify --strict
joy cache gc
joy package init acme/widgets
joy publish --registry internal --rev v0.1.0
joy owner list acme/widgets --registry internal
joy yank acme/widgets --version 0.1.0 --registry internal
joy metadata
joy doctor
joy build
joy sync
joy run -- --help
```
