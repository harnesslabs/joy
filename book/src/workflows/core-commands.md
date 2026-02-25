# Core Commands

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

Add a direct dependency from GitHub shorthand (`owner/repo`):

```bash
joy add nlohmann/json
```

Add/update with an exact revision or tag:

```bash
joy add fmtlib/fmt --rev 11.0.2
joy update fmtlib/fmt --rev 11.1.0
```

Add/update with a semver range (direct dependencies):

```bash
joy add fmtlib/fmt --version ^11
joy update fmtlib/fmt --version ^11
```

Remove a dependency:

```bash
joy remove fmtlib/fmt
```

Inspect the resolved graph:

```bash
joy tree
joy --json tree
```

## Build and Run

Build the current project:

```bash
joy build
```

Run the current project:

```bash
joy run
```

Pass arguments through to your program:

```bash
joy run -- --app-arg value
```

## Diagnose Local Environment

```bash
joy doctor
joy --json doctor
```

`joy doctor` is the fastest way to confirm compiler/Ninja/CMake/Git visibility and bundled recipe health.
