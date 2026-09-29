# CLAUDE.md

## Project

mdrvserve is a markdown preview server built as a companion for AI coding agents.
See the [README](README.md) for project overview and the
[architecture doc](docs/10-architecture.md) for design details.

## Build and test

```bash
just build-frontend                   # only needed after frontend/ changes
cargo build --release
cargo test                            # all tests
cargo test --test integration_test    # integration tests only
```

Rust 1.82+, 2021 edition. The Svelte 5 frontend is compiled to a single
inlined `frontend/dist/index.html` (`just build-frontend`) and embedded into
the binary via `include_str!`, so frontend changes require a rebuild.

## Project structure

- `src/main.rs` - CLI parsing and entry point
- `src/app.rs` - Axum router, handlers, state management, file watcher, markdown rendering
- `frontend/` - Svelte 5 app; built to `frontend/dist/index.html` and embedded at compile time
- `static/` - assets embedded at compile time (mermaid.min.js)
- `tests/integration_test.rs` - Integration tests using axum-test
- `examples/` - Sample Markdown/HTML/Typst files (Japanese competitive-programming lessons) for trying every rendering engine
- `install.sh` / `install.ps1` - installers attached to every GitHub Release

## Design constraints

- **Agent-companion scope.** mdrvserve renders markdown that AI agents produce
  during coding sessions. Features that push it toward a documentation platform,
  configurable server, or deployment target are out of scope.
- **Zero config.** `mdrvserve file.md` must work with no flags or config files.
- **Non-recursive.** Directory mode watches only the immediate directory, never
  subdirectories. This is intentional.
- **Rendered on demand, served from memory.** Tracked files are rendered to HTML
  on first request and cached; cached entries are invalidated on change. Startup
  only reads file contents and builds the index (no rendering), so large
  directories boot fast. Serving is always from memory.
- **Minimal client-side JS.** Most logic is server-side. Client JS handles
  theme selection, the WebSocket reload, scroll restoration, and small reading
  affordances (copy button, last-modified label).

## Changelog

Managed by lhg: complete entries (a `yaml` block with `mid: vX.Y.Z` plus a
`## vX.Y.Z — Title` section) are prepended to `docs/changelogs/v267.x.md`,
newest first; `docs/changelogs/CHANGELOG.md` is the series index. Release
notes are extracted from the entry's `mid:` block, so the entry must exist
before the tag is pushed.

## Commits

Use conventional commits: `type: lowercase description` (e.g. `feat:`, `fix:`,
`chore:`, `docs:`, `refactor:`, `test:`). No scopes, no emojis. Subject line
max 72 chars, imperative mood. Body optional, wrap at 72 chars, explain why not
what.
