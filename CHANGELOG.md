# Changelog

All notable changes to this project are documented here. The format is based on
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and this project
adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.1.1] — 2026-06-26

### Fixed
- Corrected the keybinding example in the herdr API notes: the `plugin_action`
  command is `persiyanov.reviewr.toggle` (the manifest `id`), not `reviewr.toggle`
  (the `name`). The wrong id resolves to a non-existent plugin and herdr reports
  "plugin action not found".

## [0.1.0] — 2026-06-26

First public release as the herdr plugin `persiyanov.reviewr`.

### Added
- **Changes tab** — changed files for the active scope (`uncommitted` / `branch` /
  `last-turn`) with `+/-` stats and syntax-highlighted unified diffs.
- **All files tab** — browse the whole worktree tree and read any file's current
  content in the diff pane.
- **Comment surface** — select a line range, write a comment, and **Add all to chat**
  to send the set to the agent as `path:start-end — comment`; clipboard export via
  `pbcopy` / `wl-copy` / `xclip` / `xsel`.
- **last-turn scope** — snapshots the worktree on each observed agent turn start
  (private `refs/reviewr/` baseline ref) to show only the agent's latest changes.
- Packaged as a herdr plugin: `sidebar` pane, `toggle` action, `worktree.created`
  auto-open. Prebuilt binaries downloaded on `herdr plugin install` via
  `herdr/install.sh` from GitHub Releases (no Rust toolchain required).
- Project scaffold: edition 2024, pinned toolchain, centralized `[lints]`,
  CI (fmt + clippy `-D warnings` + test + build), release workflow, `just` tasks,
  `cargo-deny` config, MIT license.
