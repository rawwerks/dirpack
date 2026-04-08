#!/usr/bin/env bash
# dirpack plugin — config helper for the /dirpack slash command.
#
# Usage:
#   dirpack-cc-config.sh                      # print status
#   dirpack-cc-config.sh status
#   dirpack-cc-config.sh on
#   dirpack-cc-config.sh off
#   dirpack-cc-config.sh budget <N>
#
# Persists to ${XDG_CONFIG_HOME:-$HOME/.config}/dirpack/cc-plugin.json
set -euo pipefail

CONFIG_DIR="${XDG_CONFIG_HOME:-$HOME/.config}/dirpack"
CONFIG_FILE="$CONFIG_DIR/cc-plugin.json"

mkdir -p "$CONFIG_DIR"

read_enabled() {
  # Use `has()` to distinguish missing from explicit `false` —
  # jq's `//` operator would incorrectly treat `false` as default-worthy.
  if [ -f "$CONFIG_FILE" ]; then
    jq -r 'if has("enabled") then .enabled else true end' "$CONFIG_FILE" 2>/dev/null || echo true
  else
    echo true
  fi
}

read_budget() {
  if [ -f "$CONFIG_FILE" ]; then
    jq -r 'if has("budget_tokens") then .budget_tokens else 2000 end' "$CONFIG_FILE" 2>/dev/null || echo 2000
  else
    echo 2000
  fi
}

write_config() {
  local enabled="$1" budget="$2"
  jq -n --argjson enabled "$enabled" --argjson budget "$budget" \
    '{enabled: $enabled, budget_tokens: $budget}' > "$CONFIG_FILE"
}

print_status() {
  local enabled budget
  enabled="$(read_enabled)"
  budget="$(read_budget)"
  printf 'dirpack plugin status:\n'
  printf '  enabled: %s\n' "$enabled"
  printf '  budget_tokens: %s\n' "$budget"
  printf '  config: %s\n' "$CONFIG_FILE"
}

cmd="${1:-status}"
case "$cmd" in
  status|'')
    print_status
    ;;
  on|enable)
    write_config true "$(read_budget)"
    printf 'dirpack plugin: ENABLED\n'
    print_status
    ;;
  off|disable)
    write_config false "$(read_budget)"
    printf 'dirpack plugin: DISABLED\n'
    print_status
    ;;
  budget)
    n="${2:-}"
    if [ -z "$n" ] || ! [[ "$n" =~ ^[0-9]+$ ]] || [ "$n" -le 0 ]; then
      printf 'error: /dirpack budget requires a positive integer (e.g., /dirpack budget 4000)\n' >&2
      exit 2
    fi
    write_config "$(read_enabled)" "$n"
    printf 'dirpack plugin: budget set to %s tokens\n' "$n"
    print_status
    ;;
  *)
    printf 'error: unknown subcommand %q\n' "$cmd" >&2
    printf 'usage: /dirpack [status|on|off|budget <N>]\n' >&2
    exit 2
    ;;
esac
