```yaml
mid: mdrvserve-guide-agent
label: "10 — Agent Integration"
description: Using mdrvserve with AI coding agents — the Claude Code plugin and the ephemeral-session pattern.
time_created: 2026-06-26T00:00:00+07:00
time_updated: 2026-06-26T00:00:00+07:00
scores:
  mdrv/mdrvserve: 1000
  agent: 700
  claude-code: 500
  plugin: 400
tags: [guide, agent, claude-code, plugin, workflow]
tags_excluded: []
```

# Agent integration

mdrvserve exists to close the gap between **what an AI coding agent writes** (raw
markdown in the terminal) and **what a human can read** (rendered, navigable
HTML). This guide covers the two ways to wire it in.

## The ephemeral-session pattern

mdrvserve is not a daemon. The intended lifecycle is:

1. A session starts — you ask an agent to draft a design doc, an architecture note, or a README.
2. In another terminal: `mdrvserve path/to/that.md` (or `mdrvserve docs/ --recursive`).
3. Watch the rendered output update live as the agent writes.
4. When the session ends, kill the server (`Ctrl-C`). Nothing to clean up; nothing persisted.

Because mdrvserve live-reloads over WebSocket, you never refresh — every save the
agent makes appears in the browser within a moment.

## Claude Code plugin

mdrvserve ships a [Claude Code plugin](https://code.claude.com/docs/en/plugins-reference.md)
that teaches the agent **when** and **how** to launch a preview. Install it:

```
/plugin install mdrvserve@mdrvserve
```

- Installs to **user scope** by default.
- `--scope project` shares it with all collaborators; `--scope local` keeps it to you in this repo.
- The `mdrvserve` **binary** must also be on your PATH (see [Getting Started](./00-getting-started.md)).

With the plugin installed, Claude Code automatically serves a markdown preview
when the content benefits from rendering — tables, Mermaid diagrams, long
structured documents — and skips it for short replies that read fine inline. You
stay in the terminal; the rendered view opens alongside.

## When to serve vs. not

| Content                                 | Serve? | Why                                            |
| --------------------------------------- | ------ | ---------------------------------------------- |
| A long design doc with headings         | yes    | Structure is hard to parse as terminal scroll. |
| Tables / matrices                       | yes    | Rendered tables are far more legible.          |
| Mermaid / sequence diagrams             | yes    | Only readable rendered.                        |
| A directory of related docs             | yes    | `--recursive` gives a browsable tree.          |
| A one-line answer, a short code snippet | no     | Terminal is faster and less disruptive.        |

The rule of thumb the plugin encodes: **if rendering changes how legible it is, serve it.**

## Tips

- **One server per concern.** Run `mdrvserve docs/ -r` for a doc tree; run a separate `mdrvserve some-file.md` for a single in-progress file. Each is cheap.
- **Don't fight the port.** If `3000` is busy, mdrvserve moves to the next free port and prints the URL it actually used.
- **Keep it local.** mdrvserve binds to `127.0.0.1`. Use `--hostname 0.0.0.0` only if you deliberately want another machine on your LAN to reach it, and remember there is no auth.
- **Let the agent own the files.** mdrvserve renders whatever is on disk; it never writes back. Your agent's edits are the source of truth.
