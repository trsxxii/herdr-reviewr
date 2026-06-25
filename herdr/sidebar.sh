#!/usr/bin/env bash
# Open / toggle the reviewr sidebar as a right split. Invoked by herdr with the plugin
# runtime env set (HERDR_BIN_PATH, HERDR_PANE_ID, HERDR_WORKSPACE_ID, HERDR_PLUGIN_*,
# HERDR_PLUGIN_CONTEXT_JSON, and HERDR_PLUGIN_EVENT_JSON for events).
#
#   sidebar.sh toggle   key action: open the sidebar, or close it if already open
#   sidebar.sh open     event hook: open the sidebar (e.g. on worktree.created)
set -euo pipefail

# herdr runs plugin commands with a minimal PATH; ensure jq/git resolve on common installs.
export PATH="/opt/homebrew/bin:/usr/local/bin:/usr/bin:/bin:${PATH:-}"

mode="${1:-toggle}"
H="${HERDR_BIN_PATH:-herdr}"

ws="${HERDR_WORKSPACE_ID:-}"
pane="${HERDR_PANE_ID:-}"
cwd=""
[ -n "${HERDR_PLUGIN_CONTEXT_JSON:-}" ] &&
  cwd=$(printf '%s' "$HERDR_PLUGIN_CONTEXT_JSON" | jq -r '.focused_pane_cwd // .workspace_cwd // empty')

# An event fires without a focused pane; target the new worktree's workspace from the payload
# (worktree.created shape: .data.workspace.workspace_id, .data.workspace.worktree.checkout_path).
if [ -n "${HERDR_PLUGIN_EVENT_JSON:-}" ]; then
  ev="$HERDR_PLUGIN_EVENT_JSON"
  ws=$(printf '%s' "$ev" | jq -r '.data.workspace.workspace_id // .data.worktree.open_workspace_id // empty')
  cwd=$(printf '%s' "$ev" | jq -r '.data.workspace.worktree.checkout_path // .data.worktree.path // empty')
  pane=""
fi

statedir="${HERDR_PLUGIN_STATE_DIR:-${TMPDIR:-/tmp}}"
mkdir -p "$statedir"
state="$statedir/pane-${ws:-default}"

# Toggle off: if the sidebar we opened is still alive in this workspace, close it.
if [ "$mode" = "toggle" ] && [ -f "$state" ]; then
  prev=$(cat "$state")
  if "$H" pane list --workspace "$ws" \
      | jq -e --arg p "$prev" '.result.panes[] | select(.pane_id == $p)' >/dev/null 2>&1; then
    "$H" plugin pane close "$prev" >/dev/null 2>&1 || "$H" pane close "$prev" >/dev/null 2>&1 || true
    rm -f "$state"
    exit 0
  fi
  rm -f "$state" # stale (closed via `q`); fall through and open a fresh one
fi

# Only open inside a git repo.
[ -n "$cwd" ] && git -C "$cwd" rev-parse --show-toplevel >/dev/null 2>&1 || exit 0

# A split plugin pane must target an existing pane (which implies its workspace). An event
# fires without a focused pane, so fall back to the target workspace's first pane.
if [ -z "$pane" ] && [ -n "$ws" ]; then
  pane=$("$H" pane list --workspace "$ws" | jq -r '.result.panes[0].pane_id // empty')
fi
[ -n "$pane" ] || exit 0

new=$("$H" plugin pane open --plugin "${HERDR_PLUGIN_ID:-reviewr}" --entrypoint sidebar \
  --placement split --direction right --target-pane "$pane" --cwd "$cwd" --no-focus \
  | jq -r '.result.plugin_pane.pane.pane_id // empty')
[ -n "$new" ] && printf '%s' "$new" > "$state"
