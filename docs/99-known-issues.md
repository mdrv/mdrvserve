```yaml
mid: mdrvserve-issues
label: "99 — Known Issues & Scope"
description: Current limitations, deliberate scope cuts, and what is out of scope.
time_created: 2026-06-26T00:00:00+07:00
time_updated: 2026-06-26T00:00:00+07:00
scores:
  mdrv/mdrvserve: 1000
  limitations: 800
  scope: 500
tags: [limitations, scope, known-issues]
tags_excluded: []
```

# Known issues & scope

mdrvserve is small on purpose. This page records what it does not do, what is
genuinely rough, and what has been decided against.

## Current limitations

- **In-memory only.** Nothing is persisted across runs. Restarting loses no source files (they're on disk) but loses any rendered/scroll state in the browser.
- **Alphabetical ordering.** Sidebar entries are sorted alphabetically by key. There is no frontmatter `order`/`weight` field and no manual reordering. Directory groups sort by name; files sort by name within their group.
- **No search.** Directory mode gives you a tree to browse, not a search box.
- **No syntax highlighting engine.** Code blocks get `language-<lang>` classes; actual colouring depends on the theme / a browser extension. D2 diagrams render to SVG server-side when `--with-d2` is passed. LaTeX math renders to SVG server-side when `--with-latex` is passed.
- **Single frontend bundle.** One baked-in Svelte app (`frontend/dist/index.html`). Customising the look means editing the Svelte components and rebuilding.
- **No frontmatter awareness.** mdrvserve renders the file as-is; a YAML/TOML frontmatter block is shown verbatim (markdown-rs may render it as a table or HR). Strip frontmatter upstream if it should not appear.

## Rough edges

- **Frontend toolchain dependency.** Rebuilding the UI requires Node/Bun and `bun run build` before `cargo build`. The release binary ships pre-built; this only affects local development.
- **Atomic saves.** Editors that save by writing-then-renaming (the safe default) are handled correctly because the watcher observes the directory, not just the inode. If a change ever appears to be missed, check that the editor is saving into the watched base directory.

## Deliberately out of scope (decided)

- **Static site / HTML export.** Use mdBook, Docusaurus, MkDocs, or Astro Starlight.
- **Authentication, TLS, multi-user serving.** mdrvserve binds to localhost and assumes a trusted single user.
- **Config files.** Zero-config is a hard constraint; behaviour is flags-only.
- **Client-side markdown parsing or fetch-based content loading.** The browser receives fully rendered HTML; there is no client-side router or API fetching.
- **Search, backlinks, graph views.** That is a knowledge-base tool's job (see NX).
- **Persistence, history, daemon mode.** mdrvserve is ephemeral by design.
