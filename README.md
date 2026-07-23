# herdr-reviewr

[![CI](https://github.com/persiyanov/herdr-reviewr/actions/workflows/ci.yml/badge.svg)](https://github.com/persiyanov/herdr-reviewr/actions/workflows/ci.yml)
[![Release](https://img.shields.io/github/v/release/persiyanov/herdr-reviewr)](https://github.com/persiyanov/herdr-reviewr/releases/latest)
[![License](https://img.shields.io/github/license/persiyanov/herdr-reviewr)](LICENSE)

<p align="center">
  <a href="#install">install</a> · <a href="#quick-start">quick start</a> · <a href="#controls">controls</a> · <a href="#diff-scopes">scopes</a> · <a href="#configuration">configuration</a> · <a href="#limitations">limitations</a> · <a href="CHANGELOG.md">changelog</a>
</p>

A code-review sidebar for [herdr](https://herdr.dev). Your agent writes the code. You read its
diff in a pane beside the chat, comment on the lines, and send the notes back. You never leave
the terminal.

![demo](assets/demo.gif)

One persistent pane, pointed at a git worktree:

- **Diff review** — the agent's changed files, syntax-highlighted, scoped to *uncommitted* or
  the whole *branch*. Walk hunks with `]` and `[`, files with `f` and `F`.
- **Last-turn diff** — what the agent's latest turn changed, by itself, even when the branch
  carries earlier work.
- **Line comments** — select a range, write a note. It stays visible as a card under the code,
  never hidden behind a marker.
- **Send** — one keystroke drops every comment into the agent's input as
  `path:start-end — comment`. Add context, hit enter.
- **File viewer** — the whole worktree, any file's current content in the pane.
- **Search** — `/` from any tab opens one screen over the worktree: fuzzy file names and live
  code grep, powered by [fff](https://github.com/dmtrKovalenko/fff).
  Pick a result and land on its line.
- **Find in file** — `Ctrl+F` searches the open file. Every match lights up, and `enter` and the
  arrows step the cursor between them.
- **PR view** — the branch's pull request without leaving the pane, read-only, rendered as
  styled markdown.
- **Markdown preview** — one key flips a `.md` file between source and rendered view, code
  blocks highlighted like the diff. Keeps your reading position.
- **Themes** — 18 palettes in dark and light, one config line away. Catppuccin, Dracula, Nord,
  Gruvbox, Tokyo Night, Rosé Pine, Solarized, more.

It **never edits your worktree** and sends nothing on its own. Its only git write is a private
baseline ref under `refs/reviewr/`. The **PR** tab reads GitHub, GitLab, or Azure DevOps and never posts.

## Requirements

- **herdr ≥ 0.7.0** (the plugin system).
- **git** on `PATH`.
- A **truecolor** terminal with Unicode box-drawing. Pick a theme matching its background
  ([Theme](#theme)).
- **macOS or Linux.**
- **`gh`** (GitHub), **`glab`** (GitLab), or **`az`** (Azure DevOps, with the `azure-devops` extension), authenticated — only the **PR** tab needs one.

## Install

Prebuilt binaries, no Rust toolchain needed:

```bash
herdr plugin install persiyanov/herdr-reviewr
```

Open it in the current workspace:

```bash
herdr plugin action invoke open --plugin persiyanov.reviewr
```

reviewr auto-opens in new worktrees. `auto_open = false` keeps it hidden until you ask
([Configuration](#configuration)).

**To update**, reinstall. Your config is keyed by plugin id and survives:

```bash
herdr plugin uninstall persiyanov.reviewr && herdr plugin install persiyanov/herdr-reviewr
```

**Without herdr**, reviewr runs as a plain terminal app. Grab a
[release binary](https://github.com/persiyanov/herdr-reviewr/releases/latest) and point it at a
repo:

```bash
herdr-reviewr ~/some/repo
```

Everything works except **Send** and the **last turn** scope — those need herdr around.

## Quick start

Open the sidebar next to your agent:

1. **Pick a file.** Changed files are in the navigator. `j` / `k` moves, the diff follows. Or
   `]` walks the changes hunk by hunk, file after file.
2. **Focus the diff.** `Tab` switches panes.
3. **Select lines.** `v`, then `j` / `k` to extend (or click-drag).
4. **Comment.** `c`, type, `Enter`. The note stays as a card under the line.
5. **Send.** `s` drops every comment into the agent's input as `path:start-end — comment`. Add
   context, send.

The footer shows the next step to take. Press `?` to see every key that works right now.

For a shortcut, bind a key to the toggle in your herdr config. Keybindings live in user config,
not the plugin manifest:

```toml
[[keys.command]]
key = "cmd+r"
type = "plugin_action"
command = "persiyanov.reviewr.toggle"   # <plugin_id>.<action_id> — note the id, not the name
```

`cmd+…` chords reach herdr. Many macOS terminals swallow `alt+…` themselves.

## Controls

The keys below are defaults. You can rebind every action, even to several keys at once
([Keybindings](#keybindings)).

**Getting around**

| Key | Action |
| --- | --- |
| `1` `2` `3` | Switch tab — Changes / All files / PR |
| `u` `b` `t` | Switch scope — uncommitted / branch / last turn |
| `j` `k` · `↑` `↓` | Move the cursor in the focused pane |
| `]` `[` | Jump to the next / previous hunk — press again at a file's edge to cross into the adjacent file |
| `f` `F` | Jump to the next / previous file |
| `PageUp` `PageDown` | Move a page |
| `Ctrl+U` `Ctrl+D` | Move a half-page |
| `Tab` | Switch focus between the navigator and read pane |
| `→` `←` | Expand or collapse a directory or fold, or scroll the diff sideways |
| `/` | Search files and code from any tab — type to filter, then pick a result with `↑` `↓` and open it with `enter` |
| `Ctrl+F` | Find in the open file — every match lights up, `enter` and the arrows step between them |
| `w` | Toggle line wrap |
| `m` | Toggle the markdown preview of a `.md` file (Changes or All files) |
| `p` | Move the navigator clockwise: right / bottom / left / top |
| `<` `>` | Grow / shrink the navigator |
| `r` | Refresh now |
| `?` | Show every shortcut that works in the current context |
| `q` | Quit |

**Reviewing** (in the diff)

| Key | Action |
| --- | --- |
| `v` | Start a line selection, then `j` / `k` to extend (or click-drag) |
| `c` | Comment on the selection — or on the current line |
| `e` `d` | Edit / delete the comment under the cursor |
| `n` `N` | Jump to the next / previous comment |
| `l` | List every comment |
| `s` | Send all comments to the agent |
| `y` | Copy all comments to the clipboard |
| `esc` | Clear the selection |

**In the comment box**

| Key | Action |
| --- | --- |
| `Enter` | Save the comment |
| `Esc` | Cancel |
| `Shift+Enter` · `Alt+Enter` · `Ctrl+J` | Insert a newline |

Plus the usual caret moves: arrows, `Home` / `End`, `Ctrl+A` / `Ctrl+E`, `Alt+b` / `Alt+f` word
jumps, and `Ctrl+W` / `Ctrl+U` / `Ctrl+K` deletes.

**PR tab** (read-only)

| Key | Action |
| --- | --- |
| `j` `k` | Move through the description and comments |
| `PageUp` `PageDown` | Scroll the focused pane |
| `o` | Open the PR in your browser |
| `r` | Refresh |

The mouse works too: click files and tabs, drag to select, scroll. A link in rendered markdown
opens in your browser (`http`/`https` only), and an anchor link jumps to its heading.

## The three tabs

- **Changes** — the active scope's changed files with `+/-` stats, totals in the header. Pick a
  file, read its highlighted diff, comment. On a `.md` file, `m` opens a rendered preview and
  returns where you left off.
- **All files** — the whole worktree. The read pane shows any file's current content, and you
  can comment here too. Ignored paths show dimmed. A wholly-ignored directory (`target/`,
  `node_modules/`) stays one collapsed row until you expand it. `m` flips a `.md` file to a
  read-only preview, so commenting stays in the source view.
- **PR** — a read-only mirror of the branch's pull request on GitHub or Azure DevOps, or
  merge request on GitLab: state (draft, open, merged, or closed, plus mergeability and sync), checks with a
  pass/fail rollup, the description, and every comment, newest first, with `resolved` and
  `outdated` markers. Bodies render as markdown, code blocks in your theme. `o` opens the PR
  in the browser. reviewr never posts, resolves, re-runs, or merges.

## Diff scopes

- **uncommitted** — the working tree vs `HEAD` (staged, unstaged, and untracked).
- **branch** — the working tree vs the merge-base with the base branch: **uncommitted** plus
  the branch's commits. Default base `origin/main`, then `origin/master`, `main`, `master`
  ([Base branch](#base-branch)).
- **last turn** — only what the agent changed since its most recent turn started
  ([Limitations](#limitations)).

The sidebar starts in **uncommitted**. `default_scope` changes that. Switching with `u`/`b`/`t`
wins for the rest of the session.

Every scope respects `.gitignore`, so build output never clutters **Changes**. To review a
file, track it — an intentional ignored file (a plan, a sample env) belongs in the repo, where
it shows as a change and ages out once committed. **All files** still browses any ignored path,
dimmed.

## Configuration

CLI flags on the pane command:

| Flag | Default | Meaning |
| --- | --- | --- |
| `--poll <ms>` | `2000` | worktree poll interval (min `200`) |
| `--base <ref>` | auto | base for `branch` scope, any rev, overrides `base_branches` |
| `--theme <name>` | `catppuccin` | UI + syntax theme (see below) |
| `--wrap <on\|off>` | `on` | soft-wrap long diff lines (`w` toggles at runtime) |

Everything else lives in reviewr's config file:

```text
~/.config/herdr/plugins/config/persiyanov.reviewr/config.toml
```

Create it if missing. It is reviewr's file — settings in herdr's `~/.config/herdr/config.toml`
never reach reviewr. reviewr re-reads it on every refresh and toggle, so edits apply without a
relaunch.

The file accepts these keys:

```toml
theme = "tokyo-night"
base_branches = ["develop", "main", "master"]
default_scope = "branch"
navigator_position = "right"
toggle_placement = "overlay"
toggle_direction = "down"
auto_open = false
github_host = "github.example.com"

[keybindings]
comment = ["c", "ㅊ"]
select  = ["v", "ㅍ"]
```

A missing file or omitted key uses its default. Any unknown key, wrong type, or invalid value
makes the whole file invalid — reviewr never applies the valid-looking parts. The sidebar shows
the config error until you fix the file, then recovers on its next refresh. Replace the file
atomically if your editor might expose a partial save.

### Theme

One theme colors the whole UI, chrome and syntax together:

```toml
theme = "tokyo-night"
```

`--theme` overrides the file (handy for a dev run). Match your terminal's light or dark
background — the pane keeps it, so a mismatched theme reads poorly. Available:

- **Dark:** `catppuccin`, `catppuccin-frappe`, `catppuccin-macchiato`, `dracula`, `nord`,
  `gruvbox`, `one-dark`, `solarized`, `monokai`, `tokyo-night`, `rose-pine`.
- **Light:** `catppuccin-latte`, `gruvbox-light`, `one-light`, `solarized-light`,
  `github-light`, `tokyo-night-day`, `rose-pine-dawn`.

Names match herdr's where both ship a palette. An unknown name is an error. The standalone
`--theme` flag keeps its older fallback to `catppuccin`.

### Navigator position

The navigator starts on the right. Set `navigator_position` to `right`, `bottom`, `left`, or
`top`, or press `p` to cycle clockwise:

```toml
navigator_position = "bottom"
```

Side layouts start at 32% of the width (15–60%), stacked at 25% of the height (15–50%), each
remembered separately for the session. `<` grows, `>` shrinks, or drag the divider.

### Base branch

The **branch** scope diffs against the merge-base with the first base candidate that resolves,
so one setting works across repos with different trunks. Default `main`, then `master` — each
checks `origin/<name>` first, then the local branch. For a `develop` trunk:

```toml
base_branches = ["develop", "main", "master"]
```

`--base <ref>` wins over the list and takes any rev — a branch, a tag, a SHA. When nothing in
the list resolves, the branch `origin/HEAD` names is the fallback.

### Keybindings

`[keybindings]` maps an action name to an array of keys. The array replaces that action's
defaults, actions you don't mention keep theirs, and hints show the first key:

```toml
[keybindings]
comment = ["c", "ㅊ"]
select  = ["v", "ㅍ"]
```

Several keys per action is the point when a CJK input source is active — the OS sends the
composed character, so the ASCII shortcut never arrives. Bind the character your layout
produces on the same physical key.

The action names and their defaults:

| Action | Default |
| --- | --- |
| `down` / `up` | `j` / `k` |
| `next-hunk` / `prev-hunk` | `]` / `[` |
| `next-file` / `prev-file` | `f` / `F` |
| `scope-uncommitted` / `scope-branch` / `scope-last-turn` | `u` / `b` / `t` |
| `tab-changes` / `tab-all-files` / `tab-pr` | `1` / `2` / `3` |
| `wrap` | `w` |
| `preview` | `m` |
| `navigator-position` | `p` |
| `navigator-grow` / `navigator-shrink` | `<` / `>` |
| `select` | `v` |
| `comment` | `c` |
| `edit` / `delete` | `e` / `d` |
| `next-comment` / `prev-comment` | `n` / `N` |
| `comments` | `l` |
| `search` | `/` |
| `find` | `ctrl+f` |
| `keys` | `?` |
| `send` | `s`, `S` |
| `copy` | `y`, `Y` |
| `open-pr` | `o` |
| `refresh` | `r` |
| `quit` | `q` |

A key is one printable character, or a `ctrl+`/`alt+` chord like `ctrl+f`. The arrows, `Tab`,
`Esc`, `Enter`, and the page keys are fixed and always work. Keys still type normally in the
comment box. Two actions can never share a key — a collision invalidates the whole file, and the
error names both actions.

`list-wider` and `list-narrower` remain accepted aliases for `navigator-grow` and
`navigator-shrink`. Normalized config output uses the canonical names.

### Forge repositories and hosts

A remote named exactly `upstream` with a recognized forge repository fetch URL wins. Otherwise
the PR tab reads `origin`. A Git read failure stays visible and never falls through. A standard
fork clone — fork at `origin`, base repository at `upstream` — works without setup. Both
remotes use their primary fetch URL after Git's `url.*.insteadOf` rewrite. A separate push URL
does not affect PR reads.

GitHub.com, GitLab.com, dev.azure.com, and the `*.visualstudio.com` organization hosts work
without configuration. For one self-hosted instance per forge, set its bare hostname:

```toml
github_host = "github.example.com"
gitlab_host = "git.corp.example"
azure_devops_host = "tfs.corp.example"
```

Matching is exact, and a hostname belongs to at most one forge. The one exception is
Azure DevOps' `*.visualstudio.com` family, which matches any organization label. SSH aliases like
`github.com-work` are not inferred — use a canonical-host remote or an `insteadOf` rewrite. A
literal hostname beginning with `github.com-` is valid when configured exactly. `GH_HOST` and
`GITLAB_HOST` cannot redirect a PR read, and every `az` call pins its organization.
Authenticate with `gh auth login --hostname github.example.com`,
`glab auth login --hostname git.corp.example`, or `az login`.

### Sidebar placement

The toggle opens reviewr as a split to the right of your agent. `toggle_placement` changes the
shape:

```toml
toggle_placement = "overlay"   # split | overlay | zoomed | tab   (default: split)
toggle_direction = "down"      # right | down — split only        (default: right)
```

- **`split`** sits next to your agent and leaves the keyboard with it. `toggle_direction` puts
  reviewr on the right (default) or below.
- **`overlay`** covers the tab and takes the keyboard. Toggle again to drop back.
- **`zoomed`** fills the tab like overlay and takes the keyboard.
- **`tab`** opens its own tab and takes the keyboard.

New worktrees auto-open only `split` and `tab` — `overlay` and `zoomed` wait for your toggle.
An unrecognized value invalidates the config. The next section turns auto-open off entirely.

### Auto-open and layout plugins

reviewr auto-opens in every new worktree. `auto_open = false` makes it wait for the toggle:

```toml
auto_open = false   # default: true
```

Set this when another plugin arranges your new worktrees, like
[herdr-plus](https://github.com/cloudmanic/herdr-plus) layouts. Both plugins react to the same
worktree event and race, and either can lose — the layout skipped, or reviewr dropped as a
split in the middle of it. With auto-open off, the layout builds undisturbed and your toggle
opens reviewr on top in whatever placement you configured.

A layout can also open reviewr itself, once its panes are in place:

```bash
herdr plugin action invoke open --plugin persiyanov.reviewr
```

`open` ignores `auto_open` — an explicit call is you asking. It does nothing when a sidebar is
already open, so a layout can run it on every pass. `close` does nothing when none is open.
Invoke them as `persiyanov.reviewr.open` and `persiyanov.reviewr.close`.

The action targets the **focused** workspace, so invoke it while the new workspace has focus.
It also opens reviewr as its **own new pane** — run the invoke as a one-shot from your layout
hook, not as a pane that should stay, because a pane whose command is the invoke exits when the
invoke returns.

## Limitations

This is a focused, young tool. The known constraints:

**Terminal & theme**
- **Truecolor required** — colors are 24-bit RGB with no 256/8-color fallback. Basic terminals
  render wrong colors.
- **Theme must match the terminal** — the pane keeps the terminal's background, and there is no
  auto light/dark detection yet. You match the theme by hand.
- **Add / remove are red / green** — no secondary cue for colorblind users yet.
- **Box-drawing glyphs required** — the UI draws with Unicode box characters. No Nerd Font
  needed.

**Platform**
- **macOS and Linux only** — no Windows.
- **Clipboard export** uses `pbcopy`, `wl-copy`, `xclip`, or `xsel`. With none installed it
  says so, and **Send** still works. OSC 52 and Windows are on the roadmap.

**herdr coupling**
- **Send needs a findable agent pane** — the agent in your tab, or the sole agent in the
  workspace. Otherwise Send does nothing and keeps your comments.
- **last turn relies on polling** (2 s default) — a turn that starts and finishes inside one
  poll is missed, and the scope shows everything since the last *observed* turn start. Never
  lines the agent didn't write, possibly more than one turn.

**PR tab (GitHub, GitLab, and Azure DevOps)**
- **Read-only** — needs the forge's authenticated CLI (`gh`, `glab`, or `az`) and a
  recognized `upstream` or `origin`. Without either it tells you what to fix, and the other
  tabs keep working. Other forges are not supported.
- **One repository, never a cross-repository search** — a readable, recognized `upstream` is
  authoritative, otherwise `origin`. Clones that target different parent repositories stay
  separate.
- **Mirrors the branch's *open* PR or MR** — merged or closed shows as history. Each comment
  surface caps at its newest 100 rows, with a `+more` marker naming the forge when there is
  more.

**Review model**
- **Comments are in-memory and single-session** — closing the pane loses any you haven't sent
  or copied out.
- **Sending is all-or-nothing** — Send (or copy) delivers the whole set and clears it. No
  per-comment send, no duplicate delivery, and a failure leaves everything in place.
- **No line-number rebasing** — a comment stays locatable by its diff snippet, not its line
  number. reviewr flags a stale comment instead of dropping it.
- **One sidebar per worktree** — two on the same worktree race the baseline ref, and the last
  writer wins.

**Budgets**
- Files over **2 MB** or **50,000 lines** show a "too large" notice. **Binary** files get no
  diff.

## Building from source

For the dev setup, tests, and benchmarks, see [CONTRIBUTING.md](CONTRIBUTING.md). To run your
own build inside herdr panes, link the checkout — `herdr plugin link` runs the binary you build
at `bin/herdr-reviewr`:

```bash
git clone https://github.com/persiyanov/herdr-reviewr
cd herdr-reviewr
just install   # build release → bin/herdr-reviewr, ad-hoc re-signed on macOS
herdr plugin link .
```

After every `just install`, toggle the sidebar off and on — an open pane keeps running the old
process. The loop only works while the plugin is linked: a `github:…` source in
`herdr plugin list` runs a downloaded binary that local rebuilds never touch. Switch with:

```bash
herdr plugin uninstall persiyanov.reviewr   # config is keyed by id and survives
herdr plugin link .
```

## Roadmap

Structured (JSON) export, a side-by-side split view, mark-file-reviewed,
named-key notation for keybindings, OSC light/dark theme autodetect, more themes
(`kanagawa`, `vesper`, `everforest`, `ayu`, a dark `github`), a `terminal`-following palette,
and OSC 52 clipboard.

## Design

The living design lives in [`specs/`](specs/), one concept per doc, always current.

## License

[MIT](LICENSE). Syntax highlighting comes from [syntect](https://github.com/trishume/syntect)
and [two-face](https://github.com/CosmicHorrorDev/two-face). Most themes' syntax colors come
from two-face's bundled set.

Bundled `.tmTheme` syntax files in `assets/`, each under its own license:

- [Catppuccin Mocha](https://github.com/catppuccin/bat) — MIT.
- [Tokyo Night](https://github.com/folke/tokyonight.nvim) (`tokyo-night`, `tokyo-night-day`) — Apache-2.0.
- [Rosé Pine](https://github.com/rose-pine/tm-theme) (`rose-pine`, `rose-pine-dawn`) — MIT.
