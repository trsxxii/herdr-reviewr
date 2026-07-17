---
Status: Current
Created: 2026-07-17
Last edited: 2026-07-17
---

# Input

Driving the review: the keymap, the changeset traversal, the live footer, and the comment editor.

## Overview

Every action has a key. The mouse-relevant ones also work by click or drag.

The keymap is rebindable per action through `[keybindings]` in the plugin config (`config.md`):

- The `action` column names the action for `[keybindings]`.
- The character keys are defaults.
- A key that is not a bare character (the arrows, `tab`, `esc`, `enter`, the page keys) is fixed.
- A key hint in the header or the footer shows its action's first bound key.
- The comments list acts through the same bindings and closes on `esc` and the `comments` binding.
- Prose and mockups elsewhere show the default keys.

| action                                                   | does                                        | keys                                        | mouse                         |
| -------------------------------------------------------- | ------------------------------------------- | ------------------------------------------- | ----------------------------- |
| `down` / `up`                                            | move the cursor in the focused pane         | `j` / `k` / `↓` / `↑`                       | click a row                   |
| `next-hunk` / `prev-hunk`                                | jump to the next / previous hunk            | `]` / `[`                                   | —                             |
| `next-file` / `prev-file`                                | jump to the next / previous file            | `f` / `F`                                   | —                             |
| —                                                        | collapse / expand a directory               | `←` / `→`                                   | click the directory row       |
| —                                                        | switch focus between list and diff          | `tab`                                       | click a pane                  |
| —                                                        | move a page                                 | `PageUp` / `PageDown` / `ctrl+u` / `ctrl+d` | —                             |
| —                                                        | scroll the viewport, selection put          | —                                           | wheel over the pane           |
| —                                                        | scroll the diff horizontally (wrap off)     | `←` / `→`                                   | —                             |
| `scope-uncommitted` / `scope-branch` / `scope-last-turn` | switch scope                                | `u` / `b` / `t`                             | click the scope chip to cycle |
| `tab-changes` / `tab-all-files` / `tab-pr`               | switch tab                                  | `1` / `2` / `3`                             | click a tab name              |
| —                                                        | expand the fold under the cursor            | `→`                                         | click the `⋯` row             |
| —                                                        | open a link in rendered markdown            | —                                           | click the link                |
| `wrap`                                                   | toggle line wrap                            | `w`                                         | —                             |
| `preview`                                                | toggle the markdown preview                 | `m`                                         | —                             |
| `navigator-position`                                     | move the navigator clockwise                | `p`                                         | —                             |
| `navigator-grow` / `navigator-shrink`                    | grow / shrink the navigator                 | `<` / `>`                                   | drag the divider              |
| `select`                                                 | select a line range, removed lines included | `v` then move                               | click-drag in the diff        |
| —                                                        | clear the selection                         | `esc`                                       | —                             |
| `comment`                                                | comment on the selection                    | `c`, type, `enter`                          | after a drag-select           |
| `edit`                                                   | edit the comment under the cursor           | `e`                                         | —                             |
| `delete`                                                 | delete the comment under the cursor         | `d`                                         | —                             |
| `next-comment` / `prev-comment`                          | jump to next / previous comment             | `n` / `N`                                   | —                             |
| `comments`                                               | list and manage all comments                | `l`                                         | —                             |
| `send`                                                   | send all comments to the agent              | `s` / `S`                                   | click `Send`                  |
| `copy`                                                   | copy all comments to the clipboard          | `y` / `Y`                                   | —                             |
| `open-pr`                                                | open the PR in the browser (`pr-tab.md`)    | `o`                                         | click the status chip         |
| `refresh`                                                | refresh now                                 | `r`                                         | —                             |
| `quit`                                                   | quit                                        | `q`                                         | —                             |

`navigator-position` cycles `right` → `bottom` → `left` → `top` → `right`.

`navigator-grow` and `navigator-shrink` change the active share by four percentage points. The allowed range clamps every change.

These three navigator actions work from either main pane on every tab. While the comment editor is open, their printable characters are text. In the comments list they are inert. Those local modes omit the navigator actions from the footer.

A divider drag belongs to the navigator position and split axis at mouse-down. A keypress, terminal resize, or config-driven layout change cancels it. After cancellation, drag events are consumed until mouse-up rather than becoming a selection in the read pane.

Writing a comment: select a range or land on a line, press `c`, type into the inline box, `enter` saves and `esc` cancels. A saved comment renders as a read-only card spliced under its line, titled with its location, so written feedback stays on screen. `e` reopens the card as an edit box in place, hiding the card while editing. `d` deletes it. A successful send reports that the comments were added to the agent input; a successful copy reports that they were copied. The transient status pluralizes `comment` and fades.

## Behavior

### Changeset traversal

`next-hunk` / `prev-hunk` step the diff cursor between changes, from either pane. A step lands on the first row of a run of changed rows. A context line or a fold ends a run, so two edits three lines apart are two stops.

- Each press jumps to the nearest run past the cursor: `next-hunk` below, `prev-hunk` above.
- With no run left that way, the first press arms a crossing and holds the cursor still. The next press the same way opens the adjacent file on its nearest run. A notice diff, which has no runs at all, arms on the first press like any other file.
- The armed crossing leads the footer, keyed to the step that armed it. It is the one movement key the footer names.
- Any other input drops the arm and still does its own work. A background poll keeps it, unless it changes the open file.
- A crossing arms only when a file to cross to exists. At the changeset's ends nothing is offered and nothing moves.
- A file with no changed rows is crossed over, notice diffs (`binary`, `too_large`) included.
- The steps are inert in `All files` and in the markdown preview, which paint no changed rows.

`next-file` / `prev-file` skip a file per press, from either pane, and never arm:

- In the diff, each press opens the next or previous file, cursor on its first row. Focus stays on the diff.
- In the file list, each press moves the cursor to the nearest file row, skipping directories.
- The skips land on every file, notice diffs included. From a preview, the opened file starts in source (`diff-view.md`).

The steps and the skips share the rest:

- Adjacency is the list's visible order, so a collapsed subtree is skipped.
- Opening a file this way moves the list selection onto it.
- With no target in the pressed direction, a press does nothing.
- Both are inert while a line selection is live and while the comments list is open.
- The `PR` tab has neither.

### Footer

The footer is a live action bar. It shows the actions available right now, the most likely one highlighted, and drops the least relevant when the line fills. It never lists a key that would not work in the current state.

```
 c comment · v select lines                 │ ⇥ files · p position · 1·2·3 · q
```

The bar fills by priority until the width runs out, and a trailing `…` marks anything clipped:

| slot       | content                                                                        |
| ---------- | ------------------------------------------------------------------------------ |
| primary    | the most likely next step, in a bright accent, always shown                    |
| send       | `s send N`, present once any comment is written, just below the primary        |
| actions    | what else works here, in normal text                                           |
| navigation | dim, stable: pane toggle, navigator position, tab digits, quit; dropped first |
| status     | a transient message (`comment added`) that fades, never replacing actions      |

The actions follow the cursor:

| cursor on                                | primary                        | also                    |
| ---------------------------------------- | ------------------------------ | ----------------------- |
| an armed crossing                        | `] next file` / `[ prev file`  | the cursor's own actions, demoted |
| a diff line                              | `c comment`                    | `v select`              |
| a line of a markdown file that previews  | `c comment`                    | `v select · m preview`  |
| a live selection                         | `c comment`                    | `esc clear`             |
| a commented line                         | `e edit`                       | `d delete · n/N jump`   |
| a fold                                   | `→ expand fold`                | —                       |
| an open markdown preview                 | `m source`                     | —                       |
| a file (file list)                       | `⇥ diff`                       | —                       |
| a collapsed directory                    | `→ expand`                     | —                       |
| an expanded directory                    | `← collapse`                   | —                       |
| nothing to review (awaiting turn)        | `u/b/t scope`                  | `r refresh`             |

- An armed crossing outranks the cursor's own action, since only the footer says the next press leaves the file.
- `u/b/t scope` shows in every `Changes` and `All files` context, except where it is itself the primary.
- Movement keys are never shown. The armed crossing is the one exception.
- The comment editor and the comments list show their own actions.
- The changed-file count and line totals live in the header. The footer carries only the comment count, inside `s send N`.
- On `PR` the bar leads with the PR's state, then `o open ↗`, then navigation (`pr-tab.md`).

### Comment editor

A plain-text field that edits at the caret, not only at the end. An empty box shows a dim `Leave a comment…` placeholder. `e` preloads the existing text with the caret at the end.

```
┌ comment · llm_registry.py:41 ───────────┐
│ this import path looks wrong█            │
│ and breaks on 3.12                       │
└──────────────────────────────────────────┘
```

| key                                             | does                                            |
| ----------------------------------------------- | ------------------------------------------------ |
| `←` / `→`                                       | move the caret one character                     |
| `↑` / `↓`                                       | move the caret one wrapped row, keeping column   |
| `Home` / `End`, `Ctrl+A` / `Ctrl+E`             | move to the start / end of the logical line      |
| `Alt+b` / `Alt+f`, `Alt`/`Ctrl` + `←` / `→`     | move by a word                                   |
| `Backspace` / `Delete`                          | delete before / after the caret                  |
| `Ctrl+W`                                        | delete the word before the caret                 |
| `Ctrl+U` / `Ctrl+K`                             | delete to the start / end of the logical line    |
| `Alt+Enter` / `Shift+Enter` / `Ctrl+J`          | insert a newline                                 |
| `Enter` / `Esc`                                 | save / cancel, cancel discards the draft         |

- A paste arrives whole via bracketed paste. A multi-line paste keeps its newlines. `\r\n` and `\r` normalize to `\n`.
- A paste outside the comment editor is ignored. It never starts or mutates a comment.
- Movement, insertion, and deletion are character-wise. Multi-byte and wide characters count as whole characters.
- `↑`/`↓` move by wrapped rows. `Home`/`End` and the kill keys act on the logical line, the run of text between explicit newlines.
- `Alt+b`/`Alt+f` always survive as ESC-prefixed sequences. The modified arrows work where the terminal delivers them. The character arrows, `Home`/`End`, and `Ctrl+A`/`Ctrl+E` always work.

## Non-goals

- No text selection, cut/copy, undo/redo, markdown rendering, or click-to-place-caret in the comment editor.
- No modifier, named-key, or sequence notation in the keymap. Single characters are the v1 surface.
- No `down` / `up` crossing at a file's edges. The line cursor clamps there.

## Related specs

- [tui](./tui.md)
- [config](./config.md)
- [diff-view](./diff-view.md)
- [review-model](./review-model.md)
- [pr-tab](./pr-tab.md)
