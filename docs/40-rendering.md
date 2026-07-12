```yaml
mid: mdrvserve-rendering
label: "40 — Rendering"
description: Markdown → HTML pipeline, Mermaid, templates, the sidebar tree, and themes.
time_created: 2026-06-26T00:00:00+07:00
time_updated: 2026-06-26T00:00:00+07:00
scores:
  mdrv/mdrvserve: 1000
  rendering: 900
  markdown: 600
  mermaid: 500
  templates: 500
tags: [rendering, markdown, gfm, mermaid, minijinja, templates, themes]
tags_excluded: []
```

# Rendering

mdrvserve renders markdown to HTML **once per change**, stores the HTML in the
tracked file, and reuses it for every request. The request path never parses
markdown.

## Markdown pipeline

- **Library:** [`markdown-rs`](https://github.com/wooorm/markdown-rs) (`markdown` crate), invoked via `markdown::to_html_with_options`.
- **Options:** `Options::gfm()` — GitHub-Flavoured Markdown. Tables, task lists, strikethrough, and fenced code blocks all render natively.
- No syntax highlighting pass of its own; code blocks get a `class="language-<lang>"` hook that the template/theme can style.
- The rendered HTML is stored as the `html` field on the `TrackedFile`.

## Mermaid diagrams

A fenced block tagged `` ```mermaid `` is rendered by markdown-rs as
`<code class="language-mermaid">`. mdrvserve scans the rendered HTML for that
class; if present it sets `mermaid_enabled = true`, which conditionally includes
the bundled `mermaid.min.js` (served from `/mermaid.min.js`) and the init
script. Pages without diagrams ship no Mermaid payload.

## Templates

- **Engine:** [MiniJinja](https://github.com/mitsuhiko/minijinja) (Jinja2 syntax).
- **Embedding:** templates live in `templates/` and are baked into the binary at compile time via `minijinja-embed`. **Editing a template requires a rebuild** — there is no runtime template loading.
- **Layout:** a single `templates/main.html` wraps the content with the chrome (header, theme picker, sidebar slot, reload script).

### Template variables

| Variable          | Type      | Purpose                                                          |
| ----------------- | --------- | ---------------------------------------------------------------- |
| `content`         | safe HTML | The pre-rendered markdown body.                                  |
| `mermaid_enabled` | bool      | Conditionally includes Mermaid JS + init.                        |
| `show_navigation` | bool      | Whether to render the sidebar slot (directory mode only).        |
| `nav_html`        | safe HTML | Pre-built sidebar fragment (see below). Omitted when nav is off. |
| `page_title`      | string    | Filename stem, used for the `<title>`.                           |

The old `files` list / `current_file` variables have been replaced by the
server-built `nav_html` fragment — the template no longer iterates files.

## Sidebar tree

Because recursive mode keys can be nested (`guide/intro.md`), the sidebar is
assembled **server-side** into a tree before the template runs (see
[20 — Modes](./20-modes.md)). The template just drops `{{ nav_html }}` into the
sidebar slot. Active-file highlighting is baked into the fragment as
`class="active"` on the matching anchor.

## Themes

Five built-in themes selectable from the picker in the top-right corner:

- light, dark
- Catppuccin Latte, Macchiato, Frappé, Mocha (plus the base light/dark)

Selection is stored in `localStorage` and persists across sessions and files.
Theme switching is one of the two pieces of client-side JS mdrvserve ships; the
one is the WebSocket reload listener.

## Styling

All CSS lives inline in `main.html`. Directory entries in the sidebar use the
`.nav-dir` / `.nav-dir-name` classes, with nested `.file-list` indented. There
is no external stylesheet to theme separately — keep it server-side and
single-file.
