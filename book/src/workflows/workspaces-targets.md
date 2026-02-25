# Workspaces and Targets

## Workspaces (Shipped)

`joy` supports a workspace root manifest that routes project-scoped commands to member projects.

Workspace root `joy.toml`:

```toml
[workspace]
members = ["apps/app", "tools/tooling"]
default_member = "apps/app" # optional
```

Run a command from the workspace root:

```bash
joy -p apps/app build
joy -p apps/app run
joy -p apps/app tree
```

If no `default_member` is set, `joy` requires `-p/--package <member>` for project-scoped commands run from the workspace root.

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

## Deferred: Workspace-Wide Lockfile Strategy

Status: `Deferred`

Today, per-member `joy.lock` and `.joy/` state are the active behavior. A workspace-wide lockfile strategy is a future roadmap item.
