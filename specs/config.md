---
Status: Current
Created: 2026-06-26
Last edited: 2026-06-26
---

# Config

How a user configures reviewr: where the config lives, what it carries today, and how it is meant to grow.

## Overview

reviewr reads one user-global config file, `config.toml`, from the directory herdr provisions for the plugin (`$HERDR_PLUGIN_CONFIG_DIR`, e.g. `~/.config/herdr/plugins/config/persiyanov.reviewr/`). It is authored by the user; reviewr never writes it.

```toml
# ~/.config/herdr/plugins/config/persiyanov.reviewr/config.toml
keep = [
  "docs/plans/",
  ".env.example",
]
```

| key | type | meaning |
|-----|------|---------|
| `keep` | string[] | gitignore-syntax globs; an ignored path matching one is reviewable (`review-model.md`). Default empty. |

The file is the home reviewr config grows into — theme, keybindings, and the present CLI flags (`--poll`, `--base`, `--theme`, `--wrap`) may move here later. Only `keep` is defined now; the schema must not preclude those keys.

## Behavior

- The config loads at startup and re-reads on a manual refresh (`r`), so an edit takes effect without reopening the pane.
- `keep` patterns use gitignore glob syntax, matched against repo-relative paths.
- A path git ignores that matches `keep` is **kept** — treated as untracked in the `Changes` scopes (`review-model.md`), so it lists as an addition.
- A path git ignores that matches no `keep` pattern never enters `Changes`, so build output stays out.
- `keep` does not affect `All files`, which lists every file regardless, ignored dimmed (`file-list.md`).
- Where a future config key overlaps a CLI flag, the flag wins; `keep` has no flag.

## Failure semantics

reviewr only ever reads the config; the file is the user's to write.

- No file, or no `keep` key → an empty keep list; `Changes` lists untracked-not-ignored files only, as before.
- Malformed TOML, or a non-string `keep` entry → the config is ignored for that load and a status notice says so; reviewr keeps running on defaults rather than failing.
- A concurrent edit is picked up on the next load — the next startup or `r` — never mid-frame; there is no partial application.

## Non-goals

- No committed, per-repo config in this design — a `.reviewrkeep` file (gitignore syntax) that merges over the global `keep`, mirroring git's three-tier ignore model, is the planned next tier; the `keep` model must not preclude it.
- No theme, keybinding, or flag keys yet — named as the growth path, not built here.
- reviewr never writes the config; there is no in-app settings editor.

## Decisions

- The config home is the herdr plugin config dir, not a repo file or herdr's own `config.toml` — herdr provisions `$HERDR_PLUGIN_CONFIG_DIR` per plugin, and herdr's `config.toml` is herdr's to manage. Rejected: a committed repo file as the primary home; writing into herdr's config.
- `keep` is global and uncommitted by default — reviewr is a personal tool, and a pattern like `docs/plans/` is harmless across repos, so a user opts paths in once for every repo without committing anything. Rejected: requiring a committed repo file to see one's own working docs.
- A committed `.reviewrkeep` is designed-in, not built — git's three-tier ignore model (global, repo-local, repo-shared) is the target shape, but v1 ships only the global tier. Rejected: building all tiers at once (YAGNI).
- `keep` gates `Changes` only, never `All files` — `All files` shows every file regardless; `keep` decides which ignored paths additionally count as changes. Rejected: `keep` controlling visibility in both, which would re-hide the worktree.

## Open decisions

- None.

## Related specs

- `./review-model.md`
- `./file-list.md`
- `./herdr-host.md`
