#!/usr/bin/env bash
# CI invariant — dailyos_runtime_client_for_block filter must be registered
# globally at plugin init, not only in scoped REST/test contexts.
#
# Without the global registration, every dailyos/* block render path
# short-circuits to is-empty regardless of runtime state, masking transport
# failures and pre-empting the typed runtime_unavailable_notice infrastructure
# that the renderer already implements correctly downstream.
#
# This gate fires CI red if the registration is removed from init() or if a
# new block render path is introduced that bypasses the filter entirely.
set -euo pipefail

ROOT_DIR="${DAILYOS_LINT_ROOT:-$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)}"
failures=0

fail() {
  echo "FAIL: $*" >&2
  failures=$((failures + 1))
}

runtime_filter_registered_in_plugin_init() {
  local plugin_php="$1"

  awk '
    function brace_delta(line, opens, closes) {
      opens = gsub(/\{/, "{", line)
      closes = gsub(/\}/, "}", line)
      return opens - closes
    }

    BEGIN {
      in_class = 0
      pending_class = 0
      class_depth = 0
      in_init = 0
      pending_init = 0
      init_depth = 0
      registration = "add_filter\\([[:space:]]*'\''dailyos_runtime_client_for_block'\''[[:space:]]*,[[:space:]]*\\[[[:space:]]*\\$this[[:space:]]*,[[:space:]]*'\''default_runtime_client_for_block'\''[[:space:]]*\\][[:space:]]*,[[:space:]]*5"
    }

    !in_class && !pending_class && $0 ~ /^[[:space:]]*(final[[:space:]]+)?class[[:space:]]+DailyOS_Plugin([[:space:]]|\{)/ {
      pending_class = 1
    }

    pending_class {
      if ($0 ~ /\{/) {
        in_class = 1
        pending_class = 0
        class_depth = brace_delta($0)
        if (class_depth <= 0) {
          in_class = 0
        }
      }
      next
    }

    in_class && !in_init && !pending_init && $0 ~ /^[[:space:]]*public[[:space:]]+function[[:space:]]+init[[:space:]]*\(/ {
      pending_init = 1
    }

    pending_init {
      if ($0 ~ /\{/) {
        in_init = 1
        pending_init = 0
        init_depth = brace_delta($0)
        if ($0 ~ registration) {
          found = 1
          exit
        }
        if (init_depth <= 0) {
          in_init = 0
        }
      }
      class_depth += brace_delta($0)
      if (class_depth <= 0) {
        in_class = 0
      }
      next
    }

    in_init {
      if ($0 ~ registration) {
        found = 1
        exit
      }
      init_depth += brace_delta($0)
      if (init_depth <= 0) {
        in_init = 0
      }
      class_depth += brace_delta($0)
      if (class_depth <= 0) {
        in_class = 0
      }
      next
    }

    in_class {
      class_depth += brace_delta($0)
      if (class_depth <= 0) {
        in_class = 0
      }
    }

    END {
      exit found ? 0 : 1
    }
  ' "$plugin_php"
}

# ----- inv #1: global filter registered in init() -----
PLUGIN_PHP="$ROOT_DIR/wp/dailyos/includes/class-dailyos-plugin.php"

if [ ! -f "$PLUGIN_PHP" ]; then
  fail "inv #1: $PLUGIN_PHP not found"
else
  # The registration must appear inside init() and reference the
  # default_runtime_client_for_block callback at priority 5.
  if ! runtime_filter_registered_in_plugin_init "$PLUGIN_PHP"; then
    fail "inv #1: default_runtime_client_for_block filter not registered at priority 5 inside DailyOS_Plugin::init() in $PLUGIN_PHP — every dailyos/* block will render is-empty regardless of runtime state"
  fi
fi

# ----- inv #2: default provider method exists -----
if ! grep -qE "public function default_runtime_client_for_block\(" "$PLUGIN_PHP"; then
  fail "inv #2: default_runtime_client_for_block method missing from $PLUGIN_PHP"
fi

# ----- inv #3: every block render-functions.php that consults the filter
#       receives the runtime client through it (no inline alternate paths
#       that bypass the filter and re-introduce silent failure) -----
BLOCKS_DIR="$ROOT_DIR/wp/dailyos/blocks"
if [ -d "$BLOCKS_DIR" ]; then
  while IFS= read -r render_file; do
    # If a render-functions file references project_composition_for_surface or
    # any runtime call, it MUST consult the filter to acquire the client.
    if grep -qE "project_composition_for_surface|->fetch_|->call_runtime" "$render_file"; then
      if ! grep -qE "apply_filters\(\s*'dailyos_runtime_client_for_block'" "$render_file"; then
        fail "inv #3: $render_file calls the runtime without consulting the dailyos_runtime_client_for_block filter — bypasses the global registration"
      fi
    fi
  done < <(find "$BLOCKS_DIR" -name 'render-functions.php' -type f)
fi

if [ "$failures" -gt 0 ]; then
  echo "" >&2
  echo "runtime-client filter gate failed ($failures issue(s))." >&2
  exit 1
fi

echo "runtime-client filter gate: PASS"
