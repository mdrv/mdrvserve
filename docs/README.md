```yaml
mid: mdserve-readme
label: "mdserve Documentation Index"
description: Reading order, conventions, and entry point for the mdserve docs.
time_created: 2026-06-26T00:00:00+07:00
time_updated: 2026-06-26T00:00:00+07:00
scores:
  mdrv/mdserve: 1000
  index: 500
  docs: 300
  readme: 200
tags: [index, navigation, conventions]
tags_excluded: []
```

> **Binary:** `mdserve` · **Maintainer:** MDRV (Umar Alfarouk) · **Language:** Rust 1.82+ (2021 edition) · **License:** MIT

> [!CAUTION]
> mdserve is **largely LLM-assisted** code — most of it was written by AI coding
> agents, not hand-reviewed line by line. It works and is exercised by a test
> suite, but it has not had the scrutiny of a traditional, human-audited
> codebase. Inspect it before you rely on it. **Use at your own risk.**

mdserve is a **markdown preview server built as a companion for AI coding
agents**. Start it during a coding session; as an agent writes markdown —
design docs, architecture notes, tables, diagrams — mdserve renders it live in
the browser instead of leaving it as raw text scrolling in the terminal.

These docs describe how mdserve works inside, how to run it, and the scope it
intentionally does not try to cover. The project is small; so are these docs.

## Reading order

### Foundation

- [00 — Overview & Vision](./00-overview.md) — what mdserve is, what it is not, design principles, non-goals
- [10 — Architecture](./10-architecture.md) — components, state, data flow, routing, process model
- [20 — Modes](./20-modes.md) — single-file vs directory vs recursive directory
- [30 — CLI Surface](./30-cli-surface.md) — args, flags, defaults, error behaviour
- [40 — Rendering](./40-rendering.md) — markdown pipeline, Mermaid, templates, sidebar, themes

### Quality

- [99 — Known Issues & Scope](./99-known-issues.md) — limitations and explicit out-of-scope

### Guides

- [00 — Getting Started](./guides/00-getting-started.md) — install and run your first preview
- [10 — Agent Integration](./guides/10-agent-integration.md) — the Claude Code plugin and the ephemeral-session pattern

### Releases

- [v266.0.0](./releases/v266.0.0.md) — Recursive serving, nav tree, refreshed docs & release pipeline

## Conventions used throughout

- **One binary.** `mdserve`. No config files, no flags required to start.
- **Two modes, one code path.** Single-file and directory modes share a unified router; mode is a user intent, not a code fork.
- **Pre-rendered in memory.** Every tracked file is rendered to HTML on startup and on change. Serving is always from memory, never from disk.
- **Server-side logic.** Markdown rendering, file tracking, the navigation tree, and reload triggers all live server-side. Client JS handles theme selection and the WebSocket reload only.
- **Mermaid.** Diagram blocks are detected in rendered HTML and the library is conditionally loaded.
- **Code in blocks labelled `bash`** are commands you run; blocks labelled `text` are output or structure.
