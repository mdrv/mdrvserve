```yaml
mid: mdrvserve-cli
label: "30 — CLI Surface"
description: mdrvserve invocation, positional arg, flags, defaults, and error behaviour.
time_created: 2026-06-26T00:00:00+07:00
time_updated: 2026-06-26T00:00:00+07:00
scores:
  mdrv/mdrvserve: 1000
  cli: 900
  flags: 500
  clap: 300
tags: [cli, flags, clap, args, defaults]
tags_excluded: []
```

# CLI surface

Built with **clap** (`#[derive(Parser)]`). One binary, `mdrvserve`. There is no
subcommand layer and no config file — a single positional path plus a handful of
flags is the entire surface.

## Invocation

```bash
mdrvserve <PATH> [FLAGS]
```

`PATH` is required and may be a file or a directory. Everything else is optional.

## Positional argument

| Arg    | Type | Required | Purpose                                           |
| ------ | ---- | -------- | ------------------------------------------------- |
| `path` | path | yes      | Markdown file, or directory of markdown to serve. |

## Flags

| Flag             | Short | Type   | Default     | Purpose                                                                                                      |
| ---------------- | ----- | ------ | ----------- | ------------------------------------------------------------------------------------------------------------ |
| `--hostname`     | `-H`  | string | `127.0.0.1` | Interface/domain to bind. Use `0.0.0.0` to expose on the LAN.                                                |
| `--port`         | `-p`  | number | `3000`      | First port to try (see Port selection).                                                                      |
| `--open`         | `-o`  | bool   | false       | Open the preview in the default browser on start.                                                            |
| `--recursive`    | `-r`  | bool   | false       | Descend into subdirectories (directory mode only). See [20 — Modes](./20-modes.md).                          |
| `--with-mermaid` |       | bool   | false       | Lazy-load bundled Mermaid JS for `` ```mermaid `` blocks (client-side).                                      |
| `--with-d2`      |       | bool   | false       | Render `` ```d2 `` blocks to SVG via the `d2` binary (server-side). See [40 — Rendering](./40-rendering.md). |
| `--with-latex`   |       | bool   | false       | Render `$...$` and `$$...$$` math to SVG via RaTeX (server-side). See [40 — Rendering](./40-rendering.md).  |
| `--help`         | `-h`  |        |             | Print help.                                                                                                  |
| `--version`      | `-V`  |        |             | Print version.                                                                                               |

## Defaults

- **Bind address:** `127.0.0.1:3000` — local only by default. Override with `--hostname` to expose.
- **Mode:** inferred from whether `path` is a file or a directory. There is no `--mode` flag.
- **Sidebar:** on for directory mode, off for single-file mode. Not a flag.
- **Diagrams:** opt-in. Mermaid, D2, and LaTeX are off by default; pass `--with-mermaid`, `--with-d2`, or `--with-latex` to enable. Themes are always available.

## Port selection

If the requested port is already in use, mdrvserve retries the next port
automatically — up to **10** attempts (`3000` → `3009` by default) — and logs
the port it actually bound. If none in the range is free, it exits with an
error. This keeps `mdrvserve file.md` resilient to "leftover server still
running" without forcing the user to pick a port.

## Browser open

`--open` launches the user's default browser at the served URL. Without it,
mdrvserve prints the URL and waits. The open behaviour uses the platform's default
handler; there is no browser configuration.

## Error behaviour

mdrvserve fails fast and loud rather than serving an empty state:

| Condition                              | Exit  | Message                                |
| -------------------------------------- | ----- | -------------------------------------- |
| Path is neither a file nor a directory | error | `Path must be a file or directory`     |
| Directory contains no markdown files   | error | `No markdown files found in directory` |
| No free port in the tried range        | error | `could not bind to ports <p>--<p+9>`   |
| `--recursive` on a single file         | ok    | Ignored (no directory to descend).     |

There is no `--quiet` flag and no JSON output mode; mdrvserve is meant for humans
watching a terminal while an agent works.
