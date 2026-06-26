# Milestone 02: Review surface

**Plan:** ./main.md · **Specs:** ../../../specs/ — the living reference this plan delivers

## Goal

Turn the `All files` browser into a full review surface: comment in the File view with correct staleness, annotate the tree with the active scope's changes (re-marking in place), and seed a tab switch from the current file carrying the cursor line.

## Why This Comes Next

It builds on M1's proven contract — per-tab state, the File view, and the navigator — and completes every Draft spec the branch touches. No new unknowns; this is the contract's payoff.

## Entry State

M1's exit (`ff8c68f`): the read-only `All files` browser, File view, per-tab state, and `changed_paths` backing the count and diff-comment staleness. Commenting in the File view already functions via shared machinery — a `Context`-row selection yields a `Side::New`, space-prefixed, `path:start-end` comment — but its staleness is wrong (see DoD) and the tree is unannotated.

## Definition of Done

- Selecting lines in the File view and `c` makes a comment that exports as `path:start-end` with space-prefixed content lines, joins the one set, and shows in the `l` list and `Send` count.
- A File-view comment is flagged stale only when its file is deleted from the worktree; a diff comment stays stale when it leaves the changeset.
- The `All files` tree shows each changed file's marker and `+a −d` stats for the active scope; switching scope re-marks in place — cursor, scroll, and expanded directories hold.
- Switching into a tab with no selection seeds it from the file last viewed, carrying the cursor line and revealing it comfortably; a non-seedable file (a `Changes` deletion) leaves `All files` empty.
- `cargo test`, `cargo clippy --all-targets`, and `cargo fmt --check` are clean.

## Exit State

A **closed** list — anything not named here is not built.

- `Comment` gains an origin marker (`diff_anchored: bool`), set at creation from the active view (`self.diff.view`). `stale_files` branches on it: a diff comment is stale when its file is absent from the changeset; a content comment when its file is absent from the worktree.
- `App.changed: HashMap<String, file_list::Annotation>` replaces `changed_paths` — keys back the changed-file count and diff-comment staleness; values annotate `All files` entries.
- `reload` annotates each `All files` entry whose path is in `changed` (so `file_row_item`, which already renders `Some`, shows the marker and stats).
- `set_scope` preserves the cursor, scroll, and expanded directories on `All files` (no cursor reset; `reload` re-anchors by path), so a scope switch re-marks in place.
- `set_tab` captures the source tab's current file and cursor line before the swap; after restoring the target, if it has no selection and that file exists there, it seeds the selection — revealing the file and landing the diff cursor on the carried line.

## Specs Touched

This is the branch's final milestone, so its gate is the merge gate; every Draft spec the branch realized promotes here.

| Spec | What this milestone completes | At the gate |
| --- | --- | --- |
| `review-model.md` | the file-content comment and its staleness | Draft → Current |
| `file-list.md` | the scope annotations on the whole-repo tree | Draft → Current |
| `tui.md` | tab-switch seeding, the cursor-line carry, scope re-mark | Draft → Current |
| `diff-view.md` | (already fully realized in M1; verified against code) | Draft → Current |
| `overview.md` | the `All files` tab as a complete review surface | Draft → Current |

## Out of Scope

- Hopping between the agent's changed files while browsing `All files` → roadmap (`overview.md`).
- A comment-position rail / minimap → polish backlog.

## Likely Files

- `src/model.rs` — `Comment.diff_anchored`.
- `src/app.rs` — `changed` map, entry annotation, `set_scope` position guard, `set_tab` seeding, `stale_files` branch, `build_comment` origin.
- `src/file_list.rs` — none expected (the `Option<Annotation>` plumbing already exists).
- `tests/app_flow.rs`, `tests/git_repo.rs` — annotation, re-mark, staleness, seeding, export.

## Execution Plan

1. Replace `changed_paths` with `changed: HashMap<String, Annotation>`; annotate `All files` entries from it; keep `changed_count`/staleness reading the keys.
2. Guard `set_scope` so it preserves position on `All files` (re-mark in place), still resetting on `Changes`.
3. Add `Comment.diff_anchored`, set it in `build_comment` from the view; branch `stale_files` (changeset vs worktree presence).
4. Add tab-switch seeding with the cursor-line carry in `set_tab` (capture pre-swap; reveal + land the cursor post-restore).
5. Tests: an annotated changed file in the tree; a scope switch holding the cursor while re-marking; a File-view comment that is stale only when deleted; a seed that carries the line and is not re-applied on the second visit; the File-view export snippet.

## Verification

- **Done:** `cargo test`; in the live sidebar — `2`, a changed file shows its marker; comment on an unchanged file, `l` lists it, `Send` counts it; switch back and forth lands on the same line.
- **Tight:** the diff equals Exit State — `diff_anchored` is the only new `Comment` field; `changed` replaces `changed_paths`; no annotation surface beyond the marker/stats already rendered.
- **Invariants upheld:** comments never lost to a refresh (`overview.md`) — the seed and annotate paths touch neither the store nor the input; the sidebar makes no new git write (`overview.md`); `#![forbid(unsafe_code)]` holds.

## Replan Triggers

- If `diff_anchored` proves too coarse (e.g. a comment must survive a file moving between tabs' domains), promote it to an explicit `Anchor` enum — still M2.
- If seeding's cursor-line map across the diff↔file views is ambiguous on a folded diff, land on the nearest visible row and note it.
