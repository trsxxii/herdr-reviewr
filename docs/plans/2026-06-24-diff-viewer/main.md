# Diff viewer — Delivery Strategy

**Specs:** ../../../specs/ — the living reference this plan delivers

## Goal

Replace the raw-`git diff`-text diff pane with a structured viewer: a `FileDiff` model built from old/new file content, syntax-highlighted (Catppuccin Mocha), with word emphasis, a line-number/change-bar gutter, foldable context, and line wrap — plus a directory-tree file list and two comment-box fixes. Comments keep anchoring by `(side, start..end)` and snippet.

## Milestone Map

1. **Walking skeleton** — a real syntax-highlighted structured diff of the selected file in unified view, with the existing comment flow intact. Proves the content→`similar`→`syntect`→ratatui pipeline and the Catppuccin look. Ends on an information boundary: confirm the aesthetic and per-poll performance live before building the rest.
2. **Full viewer** — word emphasis, folds, line wrap + horizontal scroll, the directory-tree file list, and the two comment-box fixes. No internal must-stop; completes the contract.

## Current Milestone

`02-full-viewer.md` — milestone 1 shipped.

## Deferred Decisions

- The exact default `syntect` theme (the one Open Decision in `diff-view.md`) is finalized by eye during milestone 1's spike, seeded with Catppuccin Mocha.

## Replan Log

- 2026-06-24: initial strategy from approved contract.
- 2026-06-24: milestone 1 shipped. Added `two-face` (syntect defaults lack TOML/TypeScript, which the spec's broad-coverage promise requires). Change bar moved to the far-left gutter edge per live feedback. `FileDiff.previous_path` deferred to milestone 2 (renames render as all-insertions until then) — note added to `02`.
