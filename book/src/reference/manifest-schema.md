# Manifest Schema

`joy` supports three `joy.toml` document shapes:

- project manifest (`[project]`)
- workspace manifest (`[workspace]`)
- reusable package manifest (`[package]`)

Source of truth: `src/manifest.rs`.

## Project Manifest

```toml
[project]
name = "demo"
version = "0.1.0"
cpp_standard = "c++20"
entry = "src/main.cpp"
extra_sources = ["src/lib.cpp"]
include_dirs = ["include"]

[[project.targets]]
name = "tool"
entry = "src/tool.cpp"
extra_sources = []
include_dirs = []

[dependencies]
"nlohmann/json" = { source = "github", rev = "HEAD" }
```

Required `project` fields:

- `name`
- `version`
- `cpp_standard`
- `entry`

Optional project fields:

- `extra_sources` (`string[]`)
- `include_dirs` (`string[]`)
- `[[project.targets]]` (named target list)

## Workspace Manifest

```toml
[workspace]
members = ["apps/app", "tools/tooling"]
default_member = "apps/app"
profile = "release"
```

Required workspace fields:

- `members` (`string[]`, non-empty)

Optional workspace fields:

- `default_member` (must be present in `members`)
- `profile` (`dev|release`)

## Package Manifest

Used for reusable package publishing workflows.

```toml
[package]
id = "acme/widgets"
version = "0.1.0"
kind = "header_only"

[headers]
include_roots = ["include"]

[dependencies]
"fmtlib/fmt" = { source = "github", version = "^11" }
```

Required package fields:

- `id` (canonical package id)
- `version`
- `kind` (`header_only|cmake`)

Optional package fields:

- `[headers].include_roots`
- package-level `[dependencies]`

## Dependency Spec Contract

Each dependency entry is a `DependencySpec`:

- `source`
- optional `package`
- source-specific fields: `rev`, `version`, `registry`, `git`, `path`, `url`, `sha256`

### Source Rules

#### `source = "github"`

- supports `rev` **or** `version` (mutually exclusive)
- does not allow `registry`, `git`, `path`, `url`, `sha256`

#### `source = "registry"`

- requires `version`
- disallows `rev`
- optional `registry`
- does not allow `git`, `path`, `url`, `sha256`

#### `source = "git"`

- requires `git` and `rev`
- does not support `version`
- does not allow `registry`, `path`, `url`, `sha256`

#### `source = "path"`

- requires `path`
- disallows `rev` and `version`
- does not allow `registry`, `git`, `url`, `sha256`

#### `source = "archive"`

- requires `url` and `sha256`
- disallows `rev` and `version`
- does not allow `registry`, `git`, `path`

## Dependency Prefix Parsing (CLI Input)

Input prefixes map to source backends:

- `registry:<id>` -> `registry`
- `github:<id>` -> `github`
- `git+<url-or-path>` or `git:<url-or-path>` -> `git`
- `path:<relative-or-absolute-path>` -> `path`
- `archive:<url>` -> `archive`
- no prefix -> `github`
