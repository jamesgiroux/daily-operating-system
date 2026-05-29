#!/usr/bin/env bash
#
# DailyOS data-path resolver lint.
#
# Hardcoded joins to the DailyOS data root (`.dailyos`) and the default
# workspace (`Documents/DailyOS`) must go through the DB-mode-aware resolver so
# non-Live modes (Replica/Mock) never resolve to production paths. The resolver
# chokepoints are:
#   - src-tauri/src/state.rs        (mode_paths_for + mode_scoped_state_path,
#                                     config_path, resolved_workspace_path, ...)
#   - src-tauri/src/db/core.rs      (dailyos_data_dir + ActionDb::db_path)
#
# New `.join(".dailyos")` / `.join("DailyOS")` call sites outside those files
# must either route through the resolver helpers above or carry a per-line
# `dailyos-path-allowed:` rationale explaining why the path is legitimately
# mode-blind (shared resource, read-only prefix check, debug-only, or test).

set -euo pipefail

if [[ -d "src-tauri" ]]; then
  roots=("src-tauri/src")
else
  roots=("src")
fi

# Resolver chokepoint files — the canonical home for mode-aware path resolution.
allowed_path_regex='(^|/)state\.rs:|(^|/)db/core\.rs:'

# Path-construction forms only: catches `.join(".dailyos")` / `.join("DailyOS")`
# and the `.push(".dailyos")` / `.push("DailyOS")` mutating form, but not string
# literals like "~/.dailyos/config.json" in user-facing messages.
pattern='\.(join|push)\("(\.dailyos|DailyOS)"\)'

matches="$(
  grep -rEn --include='*.rs' "$pattern" "${roots[@]}" 2>/dev/null \
    | grep -vE ':\s*(//|//!|///|\*)' \
    | grep -Ev "$allowed_path_regex" \
    | grep -v 'dailyos-path-allowed:' \
    || true
)"

if [[ -n "$matches" ]]; then
  echo "Hardcoded DailyOS data-path joins outside the DB-mode resolver are forbidden." >&2
  echo "Route through the resolver helpers (mode_scoped_state_path / config_path /" >&2
  echo "resolved_workspace_path / dailyos_data_dir), or add a per-line" >&2
  echo "'dailyos-path-allowed:' rationale for a legitimately mode-blind path." >&2
  echo >&2
  echo "Resolver chokepoint files:" >&2
  echo "  - src-tauri/src/state.rs" >&2
  echo "  - src-tauri/src/db/core.rs" >&2
  echo >&2
  echo "$matches" >&2
  exit 1
fi

echo "DailyOS data-path resolver allowlist clean."
