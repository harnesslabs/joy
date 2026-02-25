# CLI UX Controls

Status: `Shipped` (human output controls)

`joy` has separate human and machine interfaces:

- human mode (default) for local developer experience
- machine mode (`--json` / `--machine`) for automation

## Human Output Controls

Global flags:

```bash
joy --color auto|always|never ...
joy --progress auto|always|never ...
joy --glyphs auto|unicode|ascii ...
```

Convenience aliases:

- `--no-progress` (same as `--progress=never`)
- `--ascii` (same as `--glyphs=ascii`)

## Environment Variables

`joy` also consults environment variables for human output behavior:

- `JOY_COLOR`
- `JOY_PROGRESS`
- `JOY_GLYPHS`
- `NO_COLOR`
- `CLICOLOR`
- `CLICOLOR_FORCE`
- `TERM`
- `CI`

## Compatibility Policy

Human output is allowed to improve over time (layout, colors, wording, progress rendering).

Automation should use `--json`, which remains the stable interface.
