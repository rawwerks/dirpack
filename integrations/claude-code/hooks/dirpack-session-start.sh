#!/usr/bin/env bash
# dirpack plugin — SessionStart hook
# Injects a fresh token-budgeted dirpack index of $CWD into the new session.
#
# Honours per-user config at ${XDG_CONFIG_HOME:-$HOME/.config}/dirpack/cc-plugin.json:
#   { "enabled": true, "budget_tokens": 2000 }
# Defaults: enabled=true, budget_tokens=2000.
set -euo pipefail

INPUT="$(cat || true)"
CWD="$(printf '%s' "$INPUT" | jq -r '.cwd // empty' 2>/dev/null || true)"
[ -z "$CWD" ] && exit 0
[ -d "$CWD" ] || exit 0

CONFIG_DIR="${XDG_CONFIG_HOME:-$HOME/.config}/dirpack"
CONFIG_FILE="$CONFIG_DIR/cc-plugin.json"

# Defaults
ENABLED="true"
BUDGET="2000"

if [ -f "$CONFIG_FILE" ]; then
  # Use has() so an explicit `false` isn't mistaken for an unset default.
  ENABLED="$(jq -r 'if has("enabled") then .enabled else true end' "$CONFIG_FILE" 2>/dev/null || echo true)"
  BUDGET="$(jq -r 'if has("budget_tokens") then .budget_tokens else 2000 end' "$CONFIG_FILE" 2>/dev/null || echo 2000)"
fi

# Respect off switch
case "$ENABLED" in
  false|0|no|off) exit 0 ;;
esac

# Validate budget is a positive integer
case "$BUDGET" in
  ''|*[!0-9]*) BUDGET=2000 ;;
esac
[ "$BUDGET" -gt 0 ] || BUDGET=2000

# Locate dirpack binary
DIRPACK=""
for candidate in \
  "$(command -v dirpack 2>/dev/null || true)" \
  "$HOME/.local/bin/dirpack" \
  "$HOME/.cargo/bin/dirpack" \
  "/usr/local/bin/dirpack" \
  "/usr/bin/dirpack"; do
  if [ -n "${candidate:-}" ] && [ -x "$candidate" ]; then
    DIRPACK="$candidate"
    break
  fi
done
[ -z "$DIRPACK" ] && exit 0

# Pack the current directory, capped by time so a pathological repo can't stall startup
PACK_OUTPUT="$(cd "$CWD" && timeout 10s "$DIRPACK" pack . \
    --target-tokens "$BUDGET" \
    --format pipe \
    --root-label . 2>/dev/null || true)"

[ -z "$PACK_OUTPUT" ] && exit 0

HEADER="## dirpack repo index (fresh, budget: ${BUDGET} tokens)"
FOOTER="_Re-run \`dirpack pack . -t <N>\` for a larger snapshot. Toggle with \`/dirpack off\`._"
CONTEXT="$(printf '%s\n\n```\n%s\n```\n\n%s\n' "$HEADER" "$PACK_OUTPUT" "$FOOTER")"

# Emit JSON with additionalContext
printf '%s' "$CONTEXT" | jq -Rs '{
  hookSpecificOutput: {
    hookEventName: "SessionStart",
    additionalContext: .
  }
}'
