# Dependencies

`joy` supports direct dependencies from GitHub shorthand and a git-backed registry index mode.

It also supports bundled recipes for compiled dependencies that need CMake/Ninja build steps.

For update reporting, `joy outdated` supports source filters:

- `joy outdated --sources all` (default)
- `joy outdated --sources registry`
- `joy outdated --sources github`
