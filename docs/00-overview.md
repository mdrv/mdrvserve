```yaml
mid: mdrvserve-overview
label: "00 — Overview & Vision"
description: What mdrvserve is, what it is not, design principles, stakeholders, non-goals.
time_created: 2026-06-26T00:00:00+07:00
time_updated: 2026-06-26T00:00:00+07:00
scores:
  mdrv/mdrvserve: 1000
  overview: 300
  vision: 200
  principles: 200
  design: 150
tags: [foundation, scope, vision]
tags_excluded: [documentation-site, production-server]
```

> **Binary:** `mdrvserve` · **Maintainer:** MDRV · **Rust** 1.82+ / 2021 edition · **MIT**

> [!CAUTION]
> mdrvserve is largely **LLM-assisted** code. It works and is tested, but it has
> not been hand-audited like a traditional codebase. **Use at your own risk.**

## What mdrvserve is

mdrvserve is a **markdown preview server built as a companion for AI coding
agents**. It runs locally during a coding session, renders markdown to HTML in
the browser, and live-reloads as files change. Concretely, it does four things:

1. **Serves rendered markdown over HTTP.** One file or a whole directory, with GFM (tables, task lists, code blocks) and opt-in D2 (server-side), Mermaid (client-side), and LaTeX math (server-side) support.
2. **Live-reloads on save.** A file watcher re-renders changed files and signals every connected browser to reload over WebSocket. The core loop is: an agent writes, a human reads.
3. **Presents a directory as a navigable tree.** In directory mode a sidebar lists every `.md`/`.markdown` file; with `--recursive` nested subdirectories become collapsible groups.
4. **Stays out of the way.** Zero config, zero runtime dependencies, one static binary. `mdrvserve file.md` just works.

## What mdrvserve is deliberately not

These exclusions are part of the contract, not gaps:

- **Not a documentation site generator.** It does not emit deployable HTML. Use mdBook, Docusaurus, MkDocs, or Astro Starlight for that. mdrvserve renders for the moment, not for publication.
- **Not a production server.** It binds to `127.0.0.1` by default, has no auth, no TLS, and no rate limiting. It is meant to be started and killed within a session.
- **Not a general authoring tool.** The optimisation is for content that AI agents produce during coding, not for long-form manual writing workflows.
- **Not a search index or knowledge base.** No full-text search, no backlinks, no persistence across runs.

## Design principles

These resolve the small decisions that recur during implementation.

### 1. Zero config is a hard constraint

`mdrvserve file.md` must work with no flags and no config file. Every flag that exists makes the next one easier to justify; resist both. Defaults exist precisely so users never have to set them.

### 2. Pre-rendered in memory

All tracked files are rendered to HTML on startup and re-rendered on change. Serving is always a lookup in a `HashMap`, never a disk read plus a parse. This keeps request handling trivial and reload instant.

### 3. Server-side logic, minimal client JS

Markdown rendering, file tracking, and the navigation tree all run server-side; the browser receives fully rendered HTML. A thin Svelte 5 frontend handles UI chrome — theme picker, zoom slider, sidebar toggle, and the WebSocket reload listener — and hydrates from a JSON blob injected at serve time. No routing, no fetch calls, no client-side markdown parsing.

### 4. Ephemeral by design

mdrvserve is started during a session and killed when it ends. State lives in memory; nothing is persisted. There is no database, no history, no daemon mode.

### 5. Agent-companion scope

Features are weighed against "does this help an agent's markdown reach a human's eyes?" Anything that pushes toward a documentation platform, a configurable server, or a deployment target is out of scope — no matter how easy it would be to add.

### 6. One code path for every mode

Single-file and directory modes share a single router and state shape. Mode is a flag on the state (`is_directory_mode`), set by user intent, not a branch in the request handler. "Directory mode with one file shows a sidebar; single-file mode never does" falls out of this naturally.

## Stakeholders

mdrvserve serves one role, sometimes split across two actors:

| Role             | What they want from mdrvserve                                         |
| ---------------- | --------------------------------------------------------------------- |
| **Agent**        | Emit markdown files; expect them to appear rendered without ceremony. |
| **Human reader** | Watch rendered output update live as the agent works; browse a tree.  |

Every flag and template branch should serve one of these two.

## Relationship to other tools

mdrvserve is **complementary** to documentation site generators, not a competitor.
mdBook/Docusaurus/Starlight publish a curated set of documents for an audience;
mdrvserve previews whatever an agent is currently producing for the person driving
the session. A typical flow: iterate with mdrvserve during development, then hand

## Non-goals for v1

Explicitly deferred:

- Deployable/static output (a site generator's job).
- Authentication, TLS, or multi-user serving.
- Full-text search, backlinks, or cross-session persistence.
- A client-side markdown parser or fetch-based content loading.
- Configurable rendering pipelines. GFM + opt-in D2/Mermaid/LaTeX + themes is the whole surface.
