# Milestone 02: Full viewer

**Plan:** ./main.md · **Specs:** ../../../specs/ — the living reference this plan delivers

## Goal

The diff viewer is complete: word-level emphasis on changed lines, foldable context, line wrap with horizontal scroll, aligned tabs, and correct rename diffs — and the right pane is a directory tree. The two comment-box fixes land alongside.

## Why This Comes Next

Milestone 1 proved the structured-diff spine end to end. This completes the contract on top of it. Its riskiest assumption — that folding can insert and remove rows while the cursor, scroll, comment anchors, and mouse hit-testing all stay correct — is built first, so the rest is added only once that holds.

## Entry State

Builds on milestone 1: the diff pane renders a static `FileDiff` (`Context`/`Deletion`/`Insertion` rows) with syntect/Catppuccin highlighting, a left change-bar gutter, and line numbers; the right pane is a flat file list; comments anchor by `(side, start..end)` + snippet; 67 tests green.

## Definition of Done

- Changed lines carry word-level `emphasis`; long unchanged runs fold to `⋯ N lines` and expand/collapse with `o` or a click.
- `w` toggles wrap, and `←`/`→` scroll horizontally when wrap is off; tabs align to the gutter.
- A renamed file shows `old → new` in the header and a real content diff, not an all-insertion.
- The right pane is a directory tree: dirs expand/collapse, single-child dirs fold into their child, names middle-ellipsis-truncate, stats right-align; selection and expansion survive a poll.
- In a comment, `Ctrl+W` deletes the previous word and the box grows as the text wraps, not only on explicit newlines.
- Comments still anchor to git's real line numbers across folds; `cargo test`, `clippy -D warnings`, and `fmt` are clean and the release builds.

## Exit State

A **closed** list — anything not named is roadmap.

- `src/diff.rs` — `Row::Fold { hidden, start, end }`; `emphasis` (changed char ranges) on `Deletion`/`Insertion`; `FileDiff.previous_path`; `build` groups into hunks with a 3-line context margin, emits `Fold` rows for longer runs, and computes inline emphasis via `similar`'s word-level diff over paired lines.
- `src/file_list.rs` — `Node` (dir/file) tree built from `Vec<ChangedFile>` with single-child-directory collapse and dirs-then-files alphabetical sort; flattened to visible rows honoring per-dir expansion; middle-ellipsis path truncation.
- `src/app.rs` — `wrap: bool`, `h_scroll: usize`; fold-expansion and directory-expansion state, both re-applied by path after a poll; `content_sides` reads a rename's old side from `previous_path`; `toggle_wrap` / horizontal scroll; cursor, selection, snippet, and mouse hit-testing operate over rows and skip folds; the comment box gains word-delete.
- `src/ui.rs` — render `emphasis` (brighter bg range), `Fold` rows, wrap (default) and horizontal scroll with a pinned gutter, tab expansion, and the `old → new` rename header; `render_file_list` renders the tree.
- `src/config.rs` — `--wrap on|off`.
- `src/lib.rs` — keymap `o` (fold), `t` (view), `w` (wrap), `←`/`→` (horizontal scroll), `enter`/click toggles a directory; composing `Ctrl+W`; wrap-aware `composer_height`.
- Per `diff-view.md`/`file-list.md`/`tui.md`, minus the roadmap items below.

## Specs Touched

This milestone completes the branch, so its gate is the merge gate and every spec promotes.

| Spec | What this milestone realizes | At the gate |
| --- | --- | --- |
| `diff-view.md` | emphasis, folds, wrap, tabs, rename — the whole viewer | Draft → Current |
| `file-list.md` | the whole directory-tree navigator | Draft → Current |
| `tui.md` | the diff/file keymap, layout, and the comment-box fixes | Draft → Current |
| `review-model.md` | the structured Diff section and snippet reconstruction over rows | Draft → Current |
| `overview.md` | the syntax-highlighted viewer scope | Draft → Current |

## Out of Scope

Orientation only — each → roadmap in `overview.md`.

- Side-by-side split view → roadmap.
- Reviewed-file state, in-diff search, live theme switching → roadmap.

## Likely Files

- `src/diff.rs` — `Fold`, `emphasis`, `previous_path`, hunk grouping, inline diff.
- `src/file_list.rs` — created: the tree model.
- `src/app.rs`, `src/ui.rs` — fold/view/wrap state and rendering; tree render; rename; box fixes.
- `src/config.rs`, `src/lib.rs` — flags and keymap.
- `tests/` — `app_flow`, `render`, `git_repo` extended; `diff`/`file_list` unit tests.

## Execution Plan

1. Folds — `Row::Fold`, hunk grouping with fold rows, fold-expansion state surviving a poll, and cursor/anchor/scroll/mouse over the now-dynamic row list; `enter`. Prove the contract before the rest. *(done)*
2. Word emphasis — `similar` inline diff per paired line → `emphasis`; render the brighter bg range. *(done)*
3. Wrap and horizontal scroll — wrap default, `w`, `←`/`→` with a pinned gutter, tab expansion.
4. Rename — `previous_path`, old content from the old path, the `old → new` header.
5. File-list tree — `src/file_list.rs`, tree render, expand/collapse, truncation, selection+expansion preserved across a poll, `enter`/click toggle.
6. Comment-box fixes — `Ctrl+W` delete-word and wrap-aware `composer_height`.

## Verification

- **Done:** live in the pane — emphasis on a changed line; fold and expand a large file; toggle wrap and `←`/`→` scroll; open a renamed file; the tree shows nested dirs; `Ctrl+W` and a multi-line comment; comment through a fold and confirm the agent receives the right `path:line`.
- **Tight:** the diff equals Exit State — no split view, no reviewed-state, no search.
- **Invariants upheld:**
  - comment anchors to git's real line numbers (`review-model.md`) → the absolute-line-number test holds across a fold-expand.
  - a comment is never lost to a refresh (`overview.md`) → test: a fold toggle plus a poll keeps a saved and an in-progress comment.
  - read-only git (`overview.md`) → grep: still only `show`/`diff`/`status`/`rev-parse`/`merge-base`.
  - degrade not block (`diff-view.md`) → binary/`too_large` still render a notice under the new render path.

## Replan Triggers

- If `FileDiff.change` (in `diff-view.md`'s field table) gains no reader at the tightness check, drop it from `FileDiff` and the spec — it is the file-list `Node`'s concern, not the viewer's.
