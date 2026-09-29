# mdrvserve

> **Binary:** `mdrvserve` · **Maintainer:** MDRV (Umar Alfarouk) · **Rust** 1.82+ / 2021 edition · **MIT**

Markdown preview server for AI coding agents. Follow along as your agent writes
markdown — rendered live in the browser instead of scrolling past as raw text in
the terminal.

> [!CAUTION]
> mdrvserve is **largely LLM-assisted** code. Most of it was written by AI coding
> agents, not hand-reviewed line by line. It works and is exercised by a test
> suite, but it has not had the scrutiny of a traditional, human-audited
> codebase. Inspect it before you rely on it. **Use at your own risk.**

## Features

**Zero config.** `mdrvserve file.md` just works. No config files, no required
flags.

**Single binary.** One statically-compiled executable, no runtime dependencies.

**Instant live reload.** File changes appear in the browser immediately via
WebSocket. This is the core interaction: an agent writes, a human reads.

**Ephemeral sessions.** Start it during a session, kill it when you're done. It
is not a long-running server and doesn't need to be.

**Agent-friendly content.** GFM (tables, task lists, code blocks, alert callouts), opt-in
D2 / Mermaid / LaTeX math / Typst support, HTML file serving (`--include-html`),
Typst file serving (`--include-typst`), and directory mode with a navigation sidebar — including `--recursive`
support for nested subdirectories shown as collapsible groups. Sidebar
filter with wildcard patterns, source view, and scroll persistence round
out the reading experience.

**Structured logging.** `--debug`/`-d` and `--trace` raise verbosity (DEBUG/TRACE);
`RUST_LOG` overrides. Startup, each request, and the lazy-render cache are observable.

**Example docs.** An `examples/` directory of Japanese competitive-programming
samples exercises every rendering engine end to end.

## What mdrvserve is not

- Not a documentation site generator (use mdBook, Docusaurus, MkDocs, or Astro Starlight).
- Not a static site server or anything you deploy to production.
- Not a general-purpose authoring tool for manual writing workflows.

## Installation

### Linux & macOS

```bash
curl -fsSL https://github.com/mdrv/mdrvserve/releases/latest/download/install.sh | sh
```

Verifies checksums and installs into `~/.local/bin`. Rerun to upgrade; supports `--version`, `--prefix`, and `--no-path`. armv7 (entware/ASUS routers) is covered.

### Windows (PowerShell)

```powershell
irm https://github.com/mdrv/mdrvserve/releases/latest/download/install.ps1 | iex
```

Installs into `%LOCALAPPDATA%\Programs\mdrvserve` and adds it to your PATH. x64 and ARM64.

### Other methods

```bash
cargo install mdrvserve                                  # Cargo
sudo pacman -S mdrvserve                                 # Arch Linux
nix profile install github:mdrv/mdrvserve                # Nix
```

Build from source:

```bash
git clone https://github.com/mdrv/mdrvserve.git
cd mdrvserve
cargo build --release
cp target/release/mdrvserve <folder in your PATH>
```

Or grab an archive from the [latest release](https://github.com/mdrv/mdrvserve/releases/latest); a `SHA256SUMS.txt` covers every asset.

## Usage

```bash
# Serve a single markdown file (default port 3000)
mdrvserve README.md

# Serve all markdown files in a directory, with a sidebar
mdrvserve docs/

# Include nested subdirectories in directory mode
mdrvserve docs/ --recursive

# Custom hostname/port, and open in the browser
mdrvserve README.md --hostname 0.0.0.0 --port 8080 --open

# Render D2 diagrams server-side, LaTeX math, or Mermaid client-side
mdrvserve doc.md --with-d2 --with-latex --with-mermaid

# Render typst fenced blocks, or serve standalone .typ files
mdrvserve doc.md --with-typst
mdrvserve docs/ --include-typst

# Include HTML files alongside markdown in directory mode
mdrvserve docs/ --include-html --with-latex

# GFM alert callouts, with debug logging
mdrvserve doc.md --with-gfm --debug

# Try every engine against the bundled samples
mdrvserve examples/ --include-html --include-typst \
  --with-d2 --with-latex --with-mermaid --with-typst --with-gfm --open
```

### Single-file vs directory mode

**Single-file mode** serves one file in a clean, focused view.

**Directory mode** scans and serves all `.md` and `.markdown` files with a
navigation sidebar, watches for new files, and live-reloads on change. By
default it watches the immediate directory only; pass `--recursive` (`-r`) to
include nested subdirectories, which then appear as collapsible groups in the
sidebar.

## Themes

Five built-in themes (light, dark, and Catppuccin variants) from the theme
picker in the top-right corner. Your choice persists across sessions.

## Development

Requires Rust 1.82+ (2021 edition).

```bash
cargo build --release
cargo test
```

For architecture, modes, rendering internals, and design decisions, see the
[documentation index](docs/README.md).

## License

MIT — see [LICENSE](LICENSE).

## Acknowledgments

- Built with [Axum](https://github.com/tokio-rs/axum)
- Markdown parsing by [markdown-rs](https://github.com/wooorm/markdown-rs)
- [Catppuccin](https://catppuccin.com/) color themes
- LaTeX math rendering by [RaTeX](https://crates.io/crates/ratex-svg)
- Typst rendering via the [typst](https://github.com/typst/typst) CLI
- Part of the [MDRV](https://github.com/mdrv) ecosystem
