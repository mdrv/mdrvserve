```yaml
mid: mdrvserve-rendering
label: "40 — Rendering"
description: Markdown → HTML pipeline, D2/Mermaid/LaTeX/Typst diagrams, the Svelte frontend, sidebar tree, and themes.
time_created: 2026-06-26T00:00:00+07:00
time_updated: 2026-07-23T00:00:00+07:00
scores:
  mdrv/mdrvserve: 1000
  rendering: 900
  markdown: 600
  d2: 500
  svelte: 400
  themes: 300
tags: [rendering, markdown, gfm, d2, mermaid, latex, typst, svelte, themes]
tags_excluded: [minijinja, templates]
```

# Rendering

mdrvserve renders markdown to HTML **once per change**, stores the HTML in the
tracked file, and reuses it for every request. The request path never parses
markdown.

## Markdown pipeline

- **Library:** [`markdown-rs`](https://github.com/wooorm/markdown-rs) (`markdown` crate), invoked via `markdown::to_html_with_options`.
- **Options:** `Options::gfm()` — GitHub-Flavoured Markdown. Tables, task lists, strikethrough, and fenced code blocks all render natively.
- No syntax highlighting pass of its own; code blocks get a `class="language-<lang>"` hook that the frontend CSS can style.
- The rendered HTML is stored as the `html` field on the `TrackedFile`.

## Diagram support

Both diagram engines are **opt-in** via CLI flags. With no flags, `` ```mermaid ``
and `` ```d2 `` blocks render as plain fenced code — no payload, no latency.
LaTeX math (`$...$` / `$$...$$`) and `` ```typst `` blocks are likewise opt-in.

### D2 (server-side, `--with-d2`)

A fenced block tagged `` ```d2 `` is rendered by markdown-rs as
`<code class="language-d2">`. When `--with-d2` is passed, mdrvserve:

1. Scans the rendered HTML for `language-d2` code blocks.
2. Pipes each block's source to the `d2` binary (`d2 - --no-xml-tag --salt=<n>`) on stdin.
3. Replaces the `<pre><code>` wrapper with `<div class="d2-diagram">…<svg/></div>`.

D2 renders **server-side at render time**, so the SVG is inlined into the stored
HTML — no client JS, no render delay. Each diagram gets a unique `--salt` so
multiple SVGs in one page never collide on element IDs.

**Graceful degradation.** If the `d2` binary is not on `PATH`, mdrvserve logs a
warning at startup and the blocks fall back to plain source listings. The page
still serves.

### Mermaid (client-side, `--with-mermaid`)

A fenced block tagged `` ```mermaid `` is detected by content scan. When
`--with-mermaid` is passed, mdrvserve sets `mermaidEnabled: true` in the server
data, and the Svelte frontend lazy-loads the bundled `mermaid.min.js` (served
from `/mermaid.min.js`) only on pages that contain a diagram. The frontend
transforms `<pre><code class="language-mermaid">` blocks into
`<div class="mermaid">` elements, then calls `mermaid.run()`.

Mermaid is heavier than D2 (a ~2.7 MB JS bundle parsed and executed in the
browser) and has a visible render delay. For server-side rendering without the

### LaTeX math (server-side, `--with-latex`)

When `--with-latex` is passed, mdrvserve enables the markdown-rs math
extension (`math_flow` + `math_text` + `math_text_single_dollar`). Inline
math (`$...$`) is emitted as `<code class="math math-inline">` and display
math (`$$...$$`) as `<pre><code class="language-math math-display">`.
mdrvserve post-processes these into SVGs using
[RaTeX](https://crates.io/crates/ratex-svg) (parser → layout → SVG with
embedded KaTeX glyph outlines).

Each SVG's `width`/`height` attributes are stripped and replaced with an
inline `height` computed from the viewBox, so expressions scale naturally
with the surrounding text and the text-zoom slider. Inline math is wrapped
in `<span class="latex-inline">`, display math in `<div
class="latex-display">`. Dark themes apply `filter: invert(1)` to the black
glyphs.

Like D2, LaTeX renders **server-side at render time** — the SVG is inlined
into the stored HTML. No client JS, no external fonts.

### Typst (server-side, `--with-typst` / `--include-typst`)

Typst support has two distinct entry points, both rendered server-side by
shelling out to the `typst` binary:

- **`--with-typst`** enables the `` ```typst `` _fenced-block_ engine for
  `.md`/`.html` content. Each block is piped to `typst compile - - --format svg`
  (stdin → stdout) and replaced with
  `<div class="typst-doc"><div class="typst-doc-page">…<svg/></div></div>`.
  This mirrors the D2 pipeline exactly. Fenced blocks are single-page; a snippet
  that overflows one page falls back to its source listing.
- **`--include-typst`** tracks standalone `.typ` files in directory mode. The
  whole file is compiled with a page-number template
  (`page-{0p}-of-{t}.svg`) into a temp directory, so **multi-page documents**
  are supported: each page becomes a `.typst-doc-page`, separated by an
  `.typst-page-break` rule, all wrapped in `.typst-doc`.

Typst's output model is paged paper, so the inlined SVGs carry their own white
background and fixed layout. **On dark themes the pages are left white** (not
inverted) — this preserves Typst's intended rendering, matching how a PDF or
printed page would look.

**Graceful degradation.** If the `typst` binary is not on `PATH`, mdrvserve
logs a warning at startup; fenced blocks fall back to plain source listings and
`.typ` files are served as an escaped `<pre><code class="language-typst">`
listing. The page still serves.

## Frontend

- **Stack:** [Svelte 5](https://svelte.dev/) + Vite, built to a single
  self-contained `frontend/dist/index.html` (JS and CSS inlined by
  `vite-plugin-singlefile`).
- **Embedding:** the built `index.html` is baked into the binary at compile
  time via `include_str!`. **Editing a Svelte component requires a frontend
  rebuild** (`bun run build`) — there is no runtime template loading.
- **Data injection:** at serve time, mdrvserve replaces the
  `__MDRV_DATA_PLACEHOLDER__` marker inside the embedded HTML with a JSON blob
  containing the page content and metadata. The `<` character is escaped to
  `\u003c` so the blob can never close its own `<script>` tag prematurely.

### Server data fields

| Field            | Type          | Purpose                                                              |
| ---------------- | ------------- | -------------------------------------------------------------------- |
| `content`        | string (HTML) | The pre-rendered markdown body.                                      |
| `navItems`       | NavNode[]     | The sidebar tree (directory mode only). Empty when nav is off.       |
| `pageTitle`      | string        | Filename stem, used for the `<title>`.                               |
| `showNavigation` | boolean       | Whether to render the sidebar (directory mode only).                 |
| `mermaidEnabled` | boolean       | Whether the page contains mermaid blocks AND `--with-mermaid` is on. |

## Sidebar tree

Because recursive mode keys can be nested (`guide/intro.md`), the sidebar is
assembled **server-side** into a tree of `NavNode` values (see
[20 — Modes](./20-modes.md)), then serialized to JSON. The Svelte
`SidebarList` component recurses through the tree to render the navigation.
Active-file highlighting is applied client-side by matching the current URL.

## Themes

Five built-in themes selectable from the picker in the top-right corner:

- light, dark
- Catppuccin Latte, Macchiato, Frappé, Mocha (plus the base light/dark)

Selection is stored in `localStorage` and persists across sessions and files.
Theme switching, the zoom slider, sidebar collapse, and the WebSocket reload
listener are the pieces of client-side logic the Svelte app manages.

## Styling

All CSS lives in `frontend/src/app.css`, compiled into the inlined bundle at
build time. D2 diagrams use the `.d2-diagram` wrapper class (centered,
constrained to `max-width: 100%`). LaTeX uses `.latex-inline` (inline-block,
`vertical-align: -0.25ex`) and `.latex-display` (flex, centered). Typst uses
`.typst-doc` (centered, `max-width: 100%`) with one `.typst-doc-page` per
rendered page and an `.typst-page-break` rule between pages. Directory
entries in the sidebar use the `.nav-dir` / `.nav-dir-name` classes, with
nested `.file-list` indented. There is no external stylesheet to theme
separately — keep it server-side and single-file.
