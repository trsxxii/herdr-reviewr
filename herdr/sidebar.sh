#!/usr/bin/env bash
# The reviewr sidebar actions and event hook (specs/herdr-host.md#sidebar-actions).
#
#   sidebar.sh toggle      open the sidebar, or close it if open
#   sidebar.sh open        open the sidebar, no-op if one is open
#   sidebar.sh close       close every reviewr pane, no-op if none
#   sidebar.sh auto-open   worktree.created hook: open, gated by auto_open and placement
#
# The workspace's sidebar is any pane labeled "reviewr" in the live pane list.
# There is no state file. Actions refuse loudly (exit 1, one stderr line) and
# report successes on stdout; the event exits silently either way.
set -uo pipefail

# herdr runs plugin commands with a minimal PATH; ensure jq/git resolve on common installs.
export PATH="/opt/homebrew/bin:/usr/local/bin:/usr/bin:/bin:${PATH:-}"

mode="${1:-toggle}"
H="${HERDR_BIN_PATH:-herdr}"

refuse() {
  [ "$mode" = auto-open ] && exit 0 # an event has no caller to inform
  printf 'reviewr: %s\n' "$1" >&2
  exit 1
}

ws="${HERDR_WORKSPACE_ID:-}"
pane="${HERDR_PANE_ID:-}"
cwd=""
[ -n "${HERDR_PLUGIN_CONTEXT_JSON:-}" ] &&
  cwd=$(printf '%s' "$HERDR_PLUGIN_CONTEXT_JSON" | jq -r '.focused_pane_cwd // .workspace_cwd // empty' 2>/dev/null)

# The event fires without a focused pane; target the fresh workspace from its payload
# (worktree.created shape: .data.workspace.workspace_id, .data.workspace.worktree.checkout_path).
if [ "$mode" = auto-open ] && [ -n "${HERDR_PLUGIN_EVENT_JSON:-}" ]; then
  ev="$HERDR_PLUGIN_EVENT_JSON"
  ws=$(printf '%s' "$ev" | jq -r '.data.workspace.workspace_id // .data.worktree.open_workspace_id // empty' 2>/dev/null)
  cwd=$(printf '%s' "$ev" | jq -r '.data.workspace.worktree.checkout_path // .data.worktree.path // empty' 2>/dev/null)
  pane=""
fi

[ -n "$ws" ] || refuse "no workspace context (invoke from inside herdr)"

# One pane-list snapshot serves the whole run. A failed listing must not read as
# "no sidebar" — that would stack a duplicate on toggle and false-succeed a close.
panes_json=$("$H" pane list --workspace "$ws" 2>/dev/null) && [ -n "$panes_json" ] ||
  refuse "herdr pane list failed for $ws"

# The workspace's sidebar: every reviewr-labeled pane, any tab, any placement (spec A5).
existing=$(printf '%s' "$panes_json" | jq -r '.result.panes[] | select(.label == "reviewr") | .pane_id' 2>/dev/null)

# Plain `pane close`, not `plugin pane close`: the plugin-pane registry does not
# survive a herdr restart and would strand the pane (spec A7).
close_all() {
  closed="" failed=""
  while IFS= read -r p; do
    [ -n "$p" ] || continue
    if "$H" pane close "$p" >/dev/null 2>&1; then closed="$closed $p"; else failed="$failed $p"; fi
  done <<EOF
$existing
EOF
  [ -z "$failed" ] || refuse "failed to close$failed in $ws"
  printf 'closed%s in %s\n' "$closed" "$ws"
}

case "$mode" in
close)
  [ -n "$existing" ] || { printf 'close: nothing open in %s\n' "$ws"; exit 0; }
  close_all
  exit 0
  ;;
toggle)
  if [ -n "$existing" ]; then
    close_all
    exit 0
  fi
  ;;
open | auto-open)
  if [ -n "$existing" ]; then
    [ "$mode" = open ] && printf 'open: already open (%s) in %s\n' "$(printf '%s' "$existing" | tr '\n' ' ' | sed 's/ $//')" "$ws"
    exit 0
  fi
  ;;
*)
  refuse "unknown mode '$mode' (toggle | open | close | auto-open)"
  ;;
esac

# Opening from here on. Only inside a git repo.
if [ -z "$cwd" ] || ! git -C "$cwd" rev-parse --show-toplevel >/dev/null 2>&1; then
  refuse "not a git repo: '${cwd:-<no cwd>}'"
fi

# Placement, direction, and auto_open come from reviewr's config, re-read every run;
# an unknown value falls back to its default (spec P1, P2).
placement="split"
direction="right"
auto_open="true"
conf="${HERDR_PLUGIN_CONFIG_DIR:-}/config.toml"
if [ -n "${HERDR_PLUGIN_CONFIG_DIR:-}" ] && [ -f "$conf" ]; then
  p=$(sed -n "s/^[[:space:]]*toggle_placement[[:space:]]*=[[:space:]]*[\"']\([^\"']*\)[\"'].*/\1/p" "$conf" 2>/dev/null | tail -n1)
  d=$(sed -n "s/^[[:space:]]*toggle_direction[[:space:]]*=[[:space:]]*[\"']\([^\"']*\)[\"'].*/\1/p" "$conf" 2>/dev/null | tail -n1)
  a=$(sed -n "s/^[[:space:]]*auto_open[[:space:]]*=[[:space:]]*[\"']*\([a-z]*\)[\"']*.*/\1/p" "$conf" 2>/dev/null | tail -n1)
  case "$p" in split | overlay | zoomed | tab) placement="$p" ;; esac
  case "$d" in right | down) direction="$d" ;; esac
  case "$a" in true | false) auto_open="$a" ;; esac
fi

# Event policy gates the event alone (spec A2, P4, P8): explicit actions ignore it.
if [ "$mode" = auto-open ]; then
  [ "$auto_open" = "false" ] && exit 0
  if [ "$placement" != "split" ] && [ "$placement" != "tab" ]; then
    exit 0
  fi
fi

# Focus follows the placement on a manual open; the event never takes it (spec A3, P5, P6).
focus=--no-focus
[ "$mode" != auto-open ] && [ "$placement" != "split" ] && focus=--focus

# Placement decides the pane-open shape (spec: Sidebar placement). A split or zoomed
# open attaches to the focused pane, else the workspace's first pane.
case "$placement" in
split | zoomed)
  if [ -z "$pane" ]; then
    pane=$(printf '%s' "$panes_json" | jq -r '.result.panes[0].pane_id // empty' 2>/dev/null)
  fi
  [ -n "$pane" ] || refuse "no pane to attach to in $ws"
  set -- --placement "$placement" --target-pane "$pane"
  [ "$placement" = "split" ] && set -- "$@" --direction "$direction"
  ;;
tab)
  set -- --placement tab --workspace "$ws"
  ;;
overlay)
  set -- --placement overlay
  ;;
*)
  refuse "unreachable placement '$placement'" # guard against a future value leaking $@
  ;;
esac

new=$("$H" plugin pane open --plugin "${HERDR_PLUGIN_ID:-persiyanov.reviewr}" --entrypoint sidebar \
  "$@" --cwd "$cwd" "$focus" 2>/dev/null |
  jq -r '.result.plugin_pane.pane.pane_id // empty' 2>/dev/null)
[ -n "$new" ] || refuse "herdr plugin pane open failed"
[ "$mode" = auto-open ] || printf 'opened %s (%s) in %s\n' "$new" "$placement" "$ws"
