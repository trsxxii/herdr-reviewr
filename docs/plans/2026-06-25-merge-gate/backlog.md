# Merge-gate deferred backlog (2026-06-25)

From the xhigh 10-angle branch review. The 🔴 correctness/build + 🟡 spec items
were fixed in `d624dc6`. These are the consciously-deferred findings — real but
not merge-blocking. Grouped by kind, roughly by value.

## Edge correctness (fix when touched)

- **Composer has no scroll-to-caret** (`ui.rs` composer) — a comment taller than the box's content height clips its own caret; you type blind past the cap. Add caret-follow scrolling to the box.
- **Pending `v`-selection not re-anchored after a poll** (`app.rs` set_diff) — in Normal mode, if the agent edits the file between selecting and pressing `c`, the indices shift and the comment anchors to the wrong lines. Snapshot the selection's line numbers, or freeze on selection like compose does.
- **`start_edit` can't reach a fold-hidden line** (`app.rs:727`) — editing a comment whose line sits inside a collapsed fold opens the box on the wrong row (anchor preserved, so cosmetic). Expand the containing fold before positioning, mirroring the collapsed-directory handling.
- **`expanded_folds` re-collapses on an in-file edit** — keyed by first-hidden-line number, so an edit that shifts lines changes the anchor and re-folds. Needs a stable fold identity to fully honor "expansion persists." (Spec already softened.)
- **Send button hit-zone vs paint on a narrow sidebar** (`ui.rs:363`) — `hit_header` assumes right-alignment, but the painter collapses padding to 0 when the header overflows, so clicks near the right edge mis-fire. Derive the hit-zone from the same layout the painter uses.
- **Greedy `--flag` parsing eats the repo path** (`config.rs:39`) — `herdr-review --wrap /path` consumes the path as the flag value. Validate the next token isn't the positional. (Low impact: the plugin launches with no args.)
- **Unbounded `h_scroll`** (`app.rs:391`) — horizontal scroll has no upper clamp, so scrolling right past content shows blank until wound back. Clamp to the widest line.
- **Resize mid-frame stale hit-test** (`lib.rs:105`) — a `Resize` between the `size()` read and a click maps the click against pre-resize geometry for one frame. Handle `Event::Resize` (or recompute on it).
- **Send resolution is a point patch** (`herdr.rs:70`) — excludes only our own pane; filtering on the entry's actual `agent` field would be more robust against any future non-agent pane in `agent list`.

## Performance (per-frame / per-poll; measure before optimizing)

- **`render_diff_view` builds Lines for the whole file tail every frame** then truncates to viewport height — lazily take by accumulated display height instead. (The M5-deferred "single measure+paint pass.")
- **`diff_row_heights` / `comment_cards` / `commented_lines` recomputed 2–3×/frame** over all rows — compute once per frame and thread through; bucket comments by line into a per-row map.
- **`comment_card_lines(c).len()`** builds a full styled card just to count — add a height-only helper.
- **Poll deep-clones the open diff twice** (cache `get` clone + `rebuild_visible` clone) even when unchanged — hand back a borrow/Rc and skip rebuild when diff identity + fold set are unchanged.
- **Branch scope re-shells `merge_base` every poll/keystroke** before the cache lookup — memoize until base/HEAD changes. (M5 deferred for freshness; revisit.)
- **`untracked_additions` reads every untracked file in full every poll** — real cliff with a large untracked tree (e.g. a not-yet-ignored `build/`). Cache by path + mtime/size, or defer until the row is shown.

## Cleanup / duplication (non-blocking)

- **Two word-wrap engines** — composer `box_wrap`/`box_rows` (char-based, keeps break space) vs diff `wrap_segments` (cell-based, drops leading spaces), at different widths. Real divergence risk: a wrap fix must land in both, and the box can wrap differently from the saved card. Route the composer through one lossless char-range primitive. *(3 finders.)*
- **Derivable state**: `diff_path: Option<String>` duplicates `diff.path`; `resume_list` belongs in `Mode::Composing`; the single-use `Anchor` enum re-encodes `RowKind`.
- **git.rs helpers bypass the wrapper**: `is_repo`/`toplevel`/`ref_exists`/`merge_base` rebuild the `git -C` skeleton instead of routing through `git()`/`git_lenient()`, omitting `core.quotepath=false`.
- **Hand-rolled vs ratatui/std**: `inner_rect` vs `Block::inner` (M5: provably equal, tests enforce), `contains` vs `Rect::contains`, `centered` vs `Flex::Center`, repeated unicode-width accumulation in `truncate_width`/`elide_head`/`box_wrap`.
