# Workspaces and Targets

## Workspaces (Shipped)

`joy` supports a workspace root manifest that routes project-scoped commands to member projects.

Workspace root `joy.toml`:

```toml
[workspace]
members = ["apps/app", "tools/tooling"]
default_member = "apps/app" # optional
profile = "release"         # optional: dev|release
```

Run project-scoped commands from the workspace root:

```bash
joy -p apps/app build
joy -p apps/app run
joy -p apps/app tree
joy -p apps/app sync
joy -p apps/app verify
```

If `default_member` is unset, workspace-root project commands require `-p/--workspace-package`.

## Workspace Lockfile Behavior (Shipped)

When routed from workspace root with `-p`:

- lockfile path is `<workspace-root>/joy.lock`
- lock hash is computed from root manifest + member manifests
- lock package entries aggregate across workspace members

Use this to keep workspace member dependency state deterministic in CI.

## Workspace Profile Default (Shipped)

If `[workspace] profile = "release"` is set, `sync/build/run` default to release profile for workspace-routed commands unless `--release` explicitly overrides behavior.

## Named Targets (Shipped)

Projects can define additional binary targets using `[[project.targets]]`.

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

Build or run a named target:

```bash
joy build --target tool
joy run --target tool
```
