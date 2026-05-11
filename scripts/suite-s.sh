#!/usr/bin/env bash
# Suite S — Security invariants run on the integrated scope state.
#
# Wraps every CI policy script + cargo-audit + clippy with -D warnings into a
# single fail-closed runner. Outputs a JSON summary for the L3 aggregator.
#
# Usage: scripts/suite-s.sh [--out path] [--scope SCOPE-ID]
#   --out  Write JSON summary to this path (default: stdout)
#   --scope L3 scope identifier (free-form, e.g. v1.4.1-W0 or DOS-cleanup-batch); not enforced
#
# Exit: 0 if all checks pass; 1 if any check fails.

set -euo pipefail

OUT=""
SCOPE=""
while [[ $# -gt 0 ]]; do
  case "$1" in
    --) shift ;;
    --out) OUT="$2"; shift 2 ;;
    --scope) SCOPE="$2"; shift 2 ;;
    *) echo "unknown arg: $1" >&2; exit 2 ;;
  esac
done

REPO_ROOT="$(git rev-parse --show-toplevel)"
cd "$REPO_ROOT"

OAUTH_SECRET_SCAN_SCRIPT="$(mktemp "${TMPDIR:-/tmp}/dailyos-oauth-secret-scan.XXXXXX")"
trap 'rm -f "$OAUTH_SECRET_SCAN_SCRIPT"' EXIT

# Each entry: "label::command"
CHECKS=(
  "service-layer-boundary::./scripts/check_service_layer_boundary.sh"
  "no-let-underscore::./scripts/check_no_let_underscore_feedback.sh"
  "write-fence-usage::./scripts/check_write_fence_usage.sh"
  "ability-surface-drift::bash src-tauri/scripts/check_ability_surface_drift.sh"
  "no-live-external-clients::bash src-tauri/scripts/check_no_live_external_clients_in_eval.sh"
  "fixture-anonymization::bash src-tauri/scripts/check_fixture_anonymization.sh"
  "durable-source-comments::./scripts/check_no_ephemeral_issue_refs_in_comments.sh"
  "oauth-secret-scan::bash \"$OAUTH_SECRET_SCAN_SCRIPT\""
  "clippy-deny-warnings::cargo clippy --manifest-path src-tauri/Cargo.toml --workspace --all-features --lib --bins -- -D warnings"
  "cargo-audit::cd src-tauri && cargo audit --file audit.toml"
)

# Inline OAuth secret scan (matches CI policy step)
cat > "$OAUTH_SECRET_SCAN_SCRIPT" <<'INNER'
#!/usr/bin/env bash
set -euo pipefail
if rg -n "GOCSPX-[A-Za-z0-9_-]+" --glob '!target/**' --glob '!node_modules/**' --glob '!.git/**' --glob '!_archive/**' .; then
  echo "Committed Google OAuth secret pattern detected." >&2
  exit 1
fi
INNER

results_json="["
total=0
failed=0
first=1

for entry in "${CHECKS[@]}"; do
  label="${entry%%::*}"
  cmd="${entry#*::}"
  total=$((total + 1))

  echo "─── Suite S: $label ───" >&2
  if eval "$cmd" >/tmp/suite-s-${label}.log 2>&1; then
    status="pass"
  else
    status="fail"
    failed=$((failed + 1))
    cat /tmp/suite-s-${label}.log >&2 || true
  fi

  if [[ $first -eq 1 ]]; then first=0; else results_json+=","; fi
  log_path="/tmp/suite-s-${label}.log"
  results_json+="{\"check\":\"$label\",\"status\":\"$status\",\"log\":\"$log_path\"}"
done

results_json+="]"

summary=$(python3 - "$SCOPE" "$total" "$failed" "$results_json" <<'PY'
import json, sys
scope, total, failed, checks = sys.argv[1:]
print(json.dumps({
    "suite": "S",
    "scope": scope,
    "total": int(total),
    "failed": int(failed),
    "checks": json.loads(checks),
}, separators=(",",":")))
PY
)

if [[ -n "$OUT" ]]; then
  mkdir -p "$(dirname "$OUT")"
fi
if [[ -n "$OUT" ]]; then
  printf '%s\n' "$summary" > "$OUT"
else
  printf '%s\n' "$summary"
fi

[[ $failed -eq 0 ]] || exit 1
