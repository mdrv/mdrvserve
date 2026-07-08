```yaml
mid: mdserve-guide-start
label: "00 — Getting Started"
description: Install mdserve and run your first markdown preview in each mode.
time_created: 2026-06-26T00:00:00+07:00
time_updated: 2026-06-26T00:00:00+07:00
scores:
  mdrv/mdserve: 1000
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
> mdserve is largely LLM-assisted code. It works and is tested, but has not been
> hand-audited. **Use at your own risk.**

## Install

Pick one:

```bash
# macOS
brew install mdserve

# Linux (detects platform, installs latest binary)
curl -sSfL https://raw.githubusercontent.com/mdrv/mdserve/main/install.sh | bash

# Elsewhere
cargo install mdserve        # Cargo
sudo pacman -S mdserve       # Arch
nix profile install github:mdrv/mdserve   # Nix
```

Build from source:

```bash
git clone https://github.com/mdrv/mdserve.git
cd mdserve
cargo build --release
# binary at target/release/mdserve — put it on your PATH
```

Verify:

```bash
mdserve --version
```

## Run a single file

```bash
mdserve README.md
```

Open the printed URL (default `http://127.0.0.1:3000`). Edit the file in
another window; the browser reloads on save. Add `--open` (`-o`) to launch the
browser automatically.

## Run a directory

```bash
mdserve docs/
```

Every `*.md` / `*.markdown` in `docs/` becomes a sidebar entry; new files are
picked up automatically. The first file alphabetically is served at `/`.

## Run a nested tree

```bash
mdserve docs/ --recursive
# or: mdserve docs/ -r
```

Subdirectories are scanned and watched; nested files appear as collapsible
groups in the sidebar and are served at their relative path
(`http://127.0.0.1:3000/guide/intro.md`).

## Customise the bind

```bash
# expose on the LAN and open the browser
mdserve README.md --hostname 0.0.0.0 --port 8080 --open
```

If the port is busy, mdserve walks up to the next 10 ports automatically.

## Themes

Click the picker (top-right) for light, dark, and Catppuccin variants. Your
choice persists in the browser across sessions.

## Where next

- [10 — Agent Integration](./10-agent-integration.md) — wire mdserve into an AI coding agent.
- For how it works inside: [../10-architecture.md](../10-architecture.md).
