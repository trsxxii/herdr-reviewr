---
Status: Draft
Created: 2026-06-24
Last edited: 2026-06-24
---

# File list

The right-pane navigator: the changed files for the current scope, shown as a directory tree you move over to open diffs.

## Overview

The list groups the scope's changed files into a collapsible tree of directories and files. Each file row shows a change marker, its name, and its add/remove stats; long paths shorten with a middle ellipsis so the name and stats never clip.

```
 src/
   M app.rs                    +562 −16
   A diff_view.rs              +210
   M ui.rs                     +437 −9
 specs/
   A diff-view.md              +96
   M …/2026-06-23-changes/plan +4 −2
 M  Cargo.toml                 +11 −1
 ?  herdr-plugin.toml          +25
```

### Node

The list is a flat sequence of visible rows over the tree; a row is a directory or a file.

| field | type | meaning |
|-------|------|---------|
| `kind` | enum | `dir` or `file`. |
| `name` | string | The segment shown — a directory name, or a file's basename. |
| `depth` | integer | Nesting level, for indentation. |
| `change` | enum? | On a `file`, its `added`/`modified`/`deleted`/`renamed`/`untracked` marker; absent on a `dir`. |
| `additions` | integer? | On a `file`, lines added in the scope; absent on a `dir`. |
| `deletions` | integer? | On a `file`, lines removed in the scope; absent on a `dir`. |
| `expanded` | bool? | On a `dir`, whether its children are shown. |

## Behavior

### Tree

- Files are grouped by directory; a directory with a single child collapses into its child's row (`specs/` not `specs/` then one file) to spend vertical space on names, not scaffolding.
- Directories sort first, then files; both alphabetical within a parent.
- Directories are expanded by default — a changeset is usually small, so the whole tree is visible at once.

### Selection

- The cursor moves over the visible rows; `j`/`k` and the arrows step row to row, skipping collapsed subtrees.
- Activating a file row opens its diff in the left pane; activating a directory row toggles its `expanded` state.
- A click selects the row under it — a file opens, a directory toggles.
- A poll preserves the selected file and which directories are expanded, matching them by path; if the selected file left the changeset, its neighbour is selected.

### Presentation

- A file row is `<marker> <name> <stats>`: the change marker colored by kind, the basename bright with its parent directories dimmed, and `+a −d` stats right-aligned against the pane edge.
- When the row is too narrow, the path shortens with a middle ellipsis (`…/2026-06-23-changes/plan`) so both the basename and the stats stay visible.
- A file that carries a comment but has left the changeset is marked stale, per `review-model.md`.

## Non-goals

- No reviewed-file state in this design — marking a file reviewed and greying it is roadmap.
- No file content or preview here — selecting a file renders its diff in `diff-view.md`.
- No whole-repo tree — this lists the scope's changed files; the roadmap `All files` tab reuses this navigator over the full tree.

## Decisions

- A tree, not a flat path list — directories group related changes and shorten rows; a flat list of full paths wastes width and truncates the name. Rejected: the flat `M path +a −d` list.
- Single-child directories collapse into their child — a chain of one-child directories is scaffolding; folding it keeps names readable in a narrow pane. Rejected: always rendering every directory level.

## Open decisions

- None.

## Related specs

- `./review-model.md`
- `./tui.md`
