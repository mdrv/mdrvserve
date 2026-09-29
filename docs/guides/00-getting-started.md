```yaml
mid: mdrvserve-guide-start
label: "00 — Getting Started"
description: Install mdrvserve and run your first markdown preview in each mode.
time_created: 2026-06-26T00:00:00+07:00
time_updated: 2026-06-26T00:00:00+07:00
scores:
  mdrv/mdrvserve: 1000
  guide: 500
  install: 400
tags: [guide, install, setup, beginner]
tags_excluded: []
```

# Getting started

## Prerequisites

- A Rust toolchain (**1.82+**, 2021 edition) only if you build from source.
- A markdown file or a directory of them.
- Any modern browser.

> [!CAUTION]
> mdrvserve is largely LLM-assisted code. It works and is tested, but has not been
> hand-audited. **Use at your own risk.**

## Install

Pick one:

```bash
# Linux & macOS (verifies checksums, installs to ~/.local/bin)
curl -fsSL https://github.com/mdrv/mdrvserve/releases/latest/download/install.sh | sh

# Windows (PowerShell, x64 and ARM64)
irm https://github.com/mdrv/mdrvserve/releases/latest/download/install.ps1 | iex

# Elsewhere
cargo install mdrvserve        # Cargo
sudo pacman -S mdrvserve       # Arch
nix profile install github:mdrv/mdrvserve   # Nix
```

Build from source:

```bash
git clone https://github.com/mdrv/mdrvserve.git
cd mdrvserve
cargo build --release
# binary at target/release/mdrvserve — put it on your PATH
```

Verify:

```bash
mdrvserve --version
```

## Run a single file

```bash
mdrvserve README.md
```

Open the printed URL (default `http://127.0.0.1:3000`). Edit the file in
another window; the browser reloads on save. Add `--open` (`-o`) to launch the
browser automatically.

## Run a directory

```bash
mdrvserve docs/
```

Every `*.md` / `*.markdown` in `docs/` becomes a sidebar entry; new files are
picked up automatically. The first file alphabetically is served at `/`.

## Run a nested tree

```bash
mdrvserve docs/ --recursive
# or: mdrvserve docs/ -r
```

Subdirectories are scanned and watched; nested files appear as collapsible
groups in the sidebar and are served at their relative path
(`http://127.0.0.1:3000/guide/intro.md`).

## Customise the bind

```bash
# expose on the LAN and open the browser
mdrvserve README.md --hostname 0.0.0.0 --port 8080 --open
```

If the port is busy, mdrvserve walks up to the next 10 ports automatically.

## Themes

Click the picker (top-right) for light, dark, and Catppuccin variants. Your
choice persists in the browser across sessions.

## Where next

- [10 — Agent Integration](./10-agent-integration.md) — wire mdrvserve into an AI coding agent.
- For how it works inside: [../10-architecture.md](../10-architecture.md).
