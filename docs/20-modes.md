```yaml
mid: mdserve-modes
label: "20 — Modes"
description: Single-file, directory, and recursive directory modes — how they differ and what they share.
time_created: 2026-06-26T00:00:00+07:00
time_updated: 2026-06-26T00:00:00+07:00
scores:
  mdrv/mdserve: 1000
  modes: 800
  recursive: 500
  sidebar: 400
tags: [modes, single-file, directory, recursive, sidebar]
tags_excluded: []
```

# Modes

mdserve has one code path and three operating modes, selected by what you pass
on the command line. The mode decides what is scanned, whether subdirectories
are watched, and whether the sidebar shows.

## At a glance

| Mode                  | Invocation         | Watches             | Sidebar    | Keys          |
| --------------------- | ------------------ | ------------------- | ---------- | ------------- |
| Single-file           | `mdserve file.md`  | parent directory    | no         | filename only |
| Directory (flat)      | `mdserve docs/`    | immediate directory | yes        | filename only |
| Directory (recursive) | `mdserve docs/ -r` | directory + subdirs | yes (tree) | relative path |

## Single-file mode

```bash
mdserve README.md
```

- Argument is a file. The base directory becomes its parent; the tracked list is just that file.
- The watcher observes the parent directory (so an editor's atomic save is caught).
- **No sidebar** — the page is the rendered file, focused.
- `is_directory_mode = false`. New files appearing in the parent are **not** added.

## Directory mode (flat)

```bash
mdserve docs/
```

- Argument is a directory. `scan_markdown_files(dir, false)` collects every `*.md` / `*.markdown` in the **immediate** directory only.
- Watcher runs in `NonRecursive` mode: only the top level.
- Sidebar lists every tracked file alphabetically.
- `is_directory_mode = true`. Markdown files added to the directory later are picked up automatically.

Non-recursive is intentional. It keeps the keys flat, the sidebar shallow, and
avoids surprising the user with files from deep subdirectories they did not ask
to serve.

## Recursive directory mode

```bash
mdserve docs/ --recursive
# or: mdserve docs/ -r
```

- `scan_markdown_files(dir, true)` walks the tree (a manual DFS — no extra dependency) and collects every markdown file beneath the directory.
- Watcher runs in `Recursive` mode: nested create/modify/delete events are all caught.
- Tracked files are keyed by their **relative path** from the base directory (`guide/intro.md`), which doubles as their request URL.
- The sidebar renders as a **tree**: nested directories become collapsible groups.

```text
docs/
  index.md            → key "index.md"
  guide/
    intro.md          → key "guide/intro.md"
    advanced.md       → key "guide/advanced.md"
```

`--recursive` is ignored for single-file mode (there is no directory to descend into); passing it there is harmless.

## How the sidebar tree is built

The navigation is rendered **server-side** as an HTML fragment from the sorted
list of keys, then handed to the template as already-safe HTML (`nav_html`).
There is no client-side tree logic.

- `NavNode` is either a `File { name, full_path }` or a `Dir { name, children }`.
- Each key is split on `/` and inserted into the tree (`nav_insert`).
- Directories render as `<li class="nav-dir"><span class="nav-dir-name">…</span><ul class="file-list">…</ul></li>`; files render as `<li><a href="/{key}">…</a></li>`.
- The entry matching the current file gets `class="active"`.

Because the tree is derived purely from keys, a flat directory renders exactly
as it always did — the recursive mode is a strict superset, never a visual
regression.
