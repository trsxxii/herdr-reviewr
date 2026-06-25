---
Status: Current
Created: 2026-06-23
Last edited: 2026-06-25
---

# herdr host

How herdr-review runs inside herdr, finds its repo, sends comments to the agent, and exports to the clipboard.

## Overview

herdr-review ships as a herdr plugin (`herdr-plugin.toml`): a `sidebar` pane entrypoint that runs the binary, a `toggle` action bound to a key, and a `worktree.created` event that auto-opens it for a freshly created worktree. Opening and closing the pane is herdr's job; the binary just runs in it.

The plugin opens the sidebar as a right split (see `../docs/herdr-api-notes.md`):

```
herdr plugin pane open --plugin reviewr --entrypoint sidebar \
  --placement split --direction right --target-pane <pane> --cwd <repo> --no-focus
```

The toggle script (`herdr/sidebar.sh`) opens the sidebar for the focused pane's repo, or closes it if one is already open in the workspace (tracked in `HERDR_PLUGIN_STATE_DIR`). It is bound in user config with `[[keys.command]] type = "plugin_action"`. The pane runs `herdr-review` **by name** — resolved on `PATH`, since the pane's cwd is the repo under review, not the plugin root — so the plugin's build step installs the binary with `cargo install --path .`.

### Repo discovery

The binary reviews one worktree: the pane's working directory, normalized to its git top-level with `git rev-parse --show-toplevel`. If that path is not a git repo, it shows an empty state rather than failing.

### Sending to the agent

The sidebar is split from the agent's pane, so they share a tab. `Send` always hands over every written comment at once. To send, the binary:

- resolves the target from `herdr agent list`: the agent in the sidebar's `$HERDR_TAB_ID`, else the sole agent in its `$HERDR_WORKSPACE_ID`;
- writes all comment blocks into that pane with `herdr agent send <agent_pane> "<text>"`, without submitting;
- focuses that pane with `herdr agent focus <agent_pane>`, so you add context and press enter.

If no agent resolves, or there are two and none shares the tab, the send fails and the status says so; the comments stay in the list. Clipboard copy (also the whole set) still works.

### Clipboard

The clipboard export uses the OS utility (`pbcopy` on macOS). The binary runs where the terminal renders, a local Ghostty, so the clipboard is the user's machine.

## Failure semantics

- The send path needs the herdr CLI; browsing diffs and the clipboard export do not, so the core works from a plain shell minus the agent send.
- If the clipboard utility or `herdr agent send` fails, the export reports an error and the comment stays in the list (see `review-model.md`).

## Non-goals

These are not built here; the architecture only stays open to them.

- No server-side clipboard under herdr-over-SSH; the export targets the machine the binary runs on.
- No `last-turn` scope — the binary does not subscribe to `pane.agent_status_changed` or snapshot the worktree. A future snapshot must be non-disruptive, using private refs only.

## Decisions

- A herdr plugin, not raw pane splits — the official plugin system (`herdr-plugin.toml` with pane entrypoints, actions, and events) gives the keybind, the right-split sidebar, and worktree autolaunch, and is installable/shareable via `herdr plugin install`. Rejected: a user-config `[[keys.command]]` shell script driving `herdr pane split`, which can't declare an entrypoint pane or an event hook.
- Pane command by name, not a relative path — a split pane runs with the repo as its cwd, so `./target/release/herdr-review` resolves against the wrong directory; the binary is invoked as `herdr-review` on `PATH`.
- Send via the herdr CLI, not the raw socket — `$HERDR_BIN_PATH agent send/focus/list` is the documented, transport-stable interface.
- Browsing and clipboard need no herdr — only the agent-send export depends on herdr, so the review loop degrades gracefully without it.

## Open decisions

- The exact JSON envelope of `herdr agent list` is not pinned by the CLI contract. The resolver accepts the three plausible shapes (a bare array, `result.agents`, and `agents`) until the format is confirmed, then collapses to the one real shape.

## Related specs

- `./overview.md`
- `./review-model.md`
