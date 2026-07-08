# mdserve

> **Binary:** `mdserve` · **Maintainer:** MDRV (Umar Alfarouk) · **Rust** 1.82+ / 2021 edition · **MIT**

Markdown preview server for AI coding agents. Follow along as your agent writes
markdown — rendered live in the browser instead of scrolling past as raw text in
the terminal.

> [!CAUTION]
> mdserve is **largely LLM-assisted** code. Most of it was written by AI coding
> agents, not hand-reviewed line by line. It works and is exercised by a test
> suite, but it has not had the scrutiny of a traditional, human-audited
> codebase. Inspect it before you rely on it. **Use at your own risk.**

## Features

**Zero config.** `mdserve file.md` just works. No config files, no required
flags.

**Single binary.** One statically-compiled executable, no runtime dependencies.

**Instant live reload.** File changes appear in the browser immediately via
WebSocket. This is the core interaction: an agent writes, a human reads.

**Ephemeral sessions.** Start it during a session, kill it when you're done. It
is not a long-running server and doesn't need to be.

**Agent-friendly content.** GFM (tables, task lists, code blocks), Mermaid
diagrams, and directory mode with a navigation sidebar — including `--recursive`
support for nested subdirectories shown as collapsible groups.

## What mdserve is not

- Not a documentation site generator (use mdBook, Docusaurus, MkDocs, or Astro Starlight).
- Not a static site server or anything you deploy to production.
- Not a general-purpose authoring tool for manual writing workflows.

## Installation

### macOS (Homebrew)

```bash
brew install mdserve
```

### Linux

```bash
curl -sSfL https://raw.githubusercontent.com/mdrv/mdserve/main/install.sh | bash
```

Detects your platform and installs the latest binary.

### Other methods

```bash
cargo install mdserve                                  # Cargo
sudo pacman -S mdserve                                 # Arch Linux
nix profile install github:mdrv/mdserve                # Nix
```

Build from source:

```bash
git clone https://github.com/mdrv/mdserve.git
cd mdserve
cargo build --release
cp target/release/mdserve <folder in your PATH>
```

Or download a binary from the [latest release](https://github.com/mdrv/mdserve/releases/latest).

## Claude Code plugin

mdserve ships a [Claude Code plugin](https://code.claude.com/docs/en/plugins-reference.md)
that teaches the agent when and how to launch markdown previews. With it
installed, Claude Code automatically serves markdown when the content benefits
from rendered presentation (tables, diagrams, long documents) and skips the
preview for short responses that read fine in the terminal.

```
/plugin install mdserve@mdserve
```

Installs to user scope by default. Add `--scope project` (all collaborators) or
`--scope local` (just you in this repo). The `mdserve` binary must also be
installed (see [Installation](#installation)).

## Usage

```bash
# Serve a single markdown file (default port 3000)
mdserve README.md

# Serve all markdown files in a directory, with a sidebar
mdserve docs/

# Include nested subdirectories in directory mode
mdserve docs/ --recursive

# Custom hostname/port, and open in the browser
mdserve README.md --hostname 0.0.0.0 --port 8080 --open
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
