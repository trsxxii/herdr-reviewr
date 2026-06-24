# Milestone 01: Walking skeleton

**Plan:** ./main.md · **Specs:** ../../../specs/ — the living reference this plan delivers

## Goal

The selected file renders as a structured, syntax-highlighted diff in unified view — line numbers, change bars, Catppuccin Mocha colors, no `+`/`−` glyphs — and the existing comment flow works unchanged over the new model.

## Why This Comes Next

It proves the load-bearing unknown: that old/new content → `similar` → `syntect` → ratatui produces a beautiful, performant diff in the pane. The default theme is chosen here by eye (`diff-view.md` Open Decision). Folds, views, wrap, word emphasis, and the file tree all build on this model, so the model and the Catppuccin look are committed first and reviewed before the rest depends on them.

## Entry State

The baseline `changes-tab` branch: the diff pane parses `git diff` text into `Vec<DiffLine>` (`src/git.rs::parse_diff`) and paints it in `src/ui.rs::render_diff_view`; comments anchor over those lines; 60 tests green.

## Definition of Done

- Selecting a changed file shows its diff as a structured render: a line-number gutter, a one-cell change bar (red deletion / green insertion, no `+`/`−`), and Catppuccin Mocha syntax colors on a transparent background.
- A commented line shows its line number in the comment color; comment / edit / delete / jump / list / send / copy and the inline box all work over the new model.
- A poll reuses the cached model when the file is unchanged; the open file re-highlights only on a content change.
- Live in a herdr pane on this repo's own changes: the diff looks beautiful on the Catppuccin terminal; tests, clippy `-D warnings`, and fmt are green.

## Exit State

A **closed** list — anything not named is milestone 2.

- `src/diff.rs` — `FileDiff { path, previous_path, change, language, state, rows }`; `Row` with `Context` / `Deletion` / `Insertion` only; `Span { text, style }`. The builder reads old/new content, diffs with `similar` (3-line context), and assembles rows with `spans`. A content-keyed cache (`path` + old/new hash) returns the prior `FileDiff` when unchanged. No `emphasis`, no `Fold`.
- `src/highlight.rs` — a `syntect` highlighter with the Catppuccin Mocha theme bundled as an asset, `fancy-regex` (no C `onig`); per-line `Span`s over full file content; extension→language; foreground token colors only.
- `src/git.rs` — adds `file_content(repo, rev, path)` (via `git show <rev>:<path>`) and rev-per-scope (`HEAD` for uncommitted, merge-base for branch); removes `parse_diff`, `DiffLine`, `DiffLineKind`, and `file_diff`.
- `src/app.rs` — `diff: FileDiff` (was `Vec<DiffLine>`); `diff_cursor` / `diff_scroll` / `select_anchor`, `commented_lines`, `comment_under_cursor`, `selection_anchor`, `build_comment`, and snippet reconstruction operate over `rows`.
- `src/ui.rs` — `render_diff_view` paints rows: gutter (line number, comment-colored when commented, change bar), syntax spans with structural red/green tint, clean `path` / `previous_path → path` header; the inline comment box still splices in.
- `src/config.rs` — adds `--theme <name>` (default Catppuccin Mocha).
- `Cargo.toml` — adds `similar`, `syntect`.
- Per `diff-view.md`: `state` `binary` / `too_large` render a notice; an unknown `language` renders plain `spans`.

## Specs Touched

Each realized in part; none promotes at this checkpoint. All promote at the merge gate after milestone 2.

| Spec | What this milestone realizes | At the gate |
| --- | --- | --- |
| `diff-view.md` | the `FileDiff` model, highlighting, gutter, and color — unified only | stays Draft → merge |
| `review-model.md` | the structured Diff section and snippet reconstruction over rows | stays Draft → merge |
| `tui.md` | the diff-pane render; not the new keymap or box fixes | stays Draft → merge |
| `overview.md` | the syntax-highlighted-viewer scope, in part | stays Draft → merge |

## Out of Scope

Orientation only — each → milestone 2.

- Word `emphasis` and `Row::Fold` + folding (`enter`) → milestone 2.
- Line wrap + `←`/`→` and `--wrap` → milestone 2.
- The directory-tree file list (`file-list.md`) → milestone 2.
- `Ctrl+W` delete-word and wrap-aware box growth → milestone 2.

## Likely Files

- `src/diff.rs` — created.
- `src/highlight.rs` — created.
- `src/git.rs` — `file_content` added; `parse_diff` / `DiffLine` / `DiffLineKind` / `file_diff` removed.
- `src/app.rs`, `src/ui.rs` — diff field and rendering reshaped to `FileDiff` / `Row`.
- `src/config.rs`, `Cargo.toml`, `assets/` (the Catppuccin `.tmTheme`) — touched.
- `tests/` — `git_repo`, `app_flow`, `render` updated to the new model; `diff`/`highlight` unit tests added.

## Execution Plan

1. Spike the pipeline: `file_content` + `similar` + `syntect`/Catppuccin → one `FileDiff`, dumped to confirm spans and the theme load. Pick the default theme by eye.
2. Land `src/diff.rs` and `src/highlight.rs` with the content cache and unit tests.
3. Reshape `App` and `render_diff_view` onto `FileDiff` / `Row`; rewire anchoring and snippet reconstruction; recolor commented-line numbers.
4. Remove the dead `parse_diff` / `DiffLine` path; update tests to the new model.
5. Run in a herdr pane on this repo; tune the Catppuccin tints; confirm the look and per-poll cost.

## Verification

- **Done:** in a herdr split pane on this repo's changes — open a file, see a Catppuccin-highlighted structured diff with line numbers and change bars and no `+`/`−`; comment, edit, send, and copy all work; it looks beautiful.
- **Tight:** the diff equals Exit State — no `emphasis`, no `Fold`, no wrap, no tree, no box fixes.
- **Invariants upheld:**
  - read-only git (`overview.md`) → grep source: only `show` / `diff` / `status` / `rev-parse` / `merge-base` run.
  - a comment is never lost to a refresh (`review-model.md`) → test: a poll over the new model keeps a saved and an in-progress comment.
  - consume-on-success (`review-model.md`) → test: a failed export keeps every comment.
  - `unsafe` forbidden (`overview.md`) → crate lint holds with `similar` / `syntect` (`fancy-regex`, not `onig`).

## Replan Triggers

- If the Catppuccin theme renders wrong or per-poll highlighting is too slow even with the cache, revisit highlighting (theme handling or tree-sitter) before milestone 2.
- If `syntect` pulls the C `onig` regex, switch to its `fancy-regex` feature.
- If `similar`'s hunking diverges confusingly from `git`'s, reconsider the diff source before building folds on it.
