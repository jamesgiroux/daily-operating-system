#!/usr/bin/env bash
# Suite S — Security invariants run on the integrated scope state.
#
# Wraps every CI policy script + cargo-audit + clippy with -D warnings into a
# single fail-closed runner. Outputs a JSON summary for the L3 aggregator.
#
# Usage: scripts/suite-s.sh [--out path] [--scope SCOPE-ID] [--self-test]
#   --out  Write JSON summary to this path (default: stdout)
#   --scope L3 scope identifier (free-form, e.g. v1.4.1-W0 or DOS-cleanup-batch); not enforced
#   --self-test  Verify Suite S wrapper command/config wiring without running the full suite
#
# Exit: 0 if all checks pass; 1 if any check fails.

set -euo pipefail

OUT=""
SCOPE=""
SELF_TEST=0
while [[ $# -gt 0 ]]; do
  case "$1" in
    --) shift ;;
    --out) OUT="$2"; shift 2 ;;
    --scope) SCOPE="$2"; shift 2 ;;
    --self-test) SELF_TEST=1; shift ;;
    *) echo "unknown arg: $1" >&2; exit 2 ;;
  esac
done

REPO_ROOT="$(git rev-parse --show-toplevel)"
cd "$REPO_ROOT"

SUITE_S_TMP_ROOT="$(mktemp -d "${TMPDIR:-/tmp}/dailyos-suite-s.XXXXXX")"
trap 'rm -rf "$SUITE_S_TMP_ROOT"' EXIT

OAUTH_SECRET_SCAN_SCRIPT="$SUITE_S_TMP_ROOT/oauth-secret-scan.sh"
CARGO_AUDIT_POLICY="$REPO_ROOT/audit.toml"
CARGO_AUDIT_LOCKFILE="$REPO_ROOT/src-tauri/Cargo.lock"
CARGO_AUDIT_WORKDIR="$SUITE_S_TMP_ROOT/cargo-audit"
CARGO_AUDIT_CONFIG="$CARGO_AUDIT_WORKDIR/.cargo/audit.toml"

prepare_cargo_audit_config() {
  if [[ ! -f "$CARGO_AUDIT_POLICY" ]]; then
    echo "missing cargo-audit policy: $CARGO_AUDIT_POLICY" >&2
    return 1
  fi
  if [[ ! -f "$CARGO_AUDIT_LOCKFILE" ]]; then
    echo "missing cargo-audit lockfile: $CARGO_AUDIT_LOCKFILE" >&2
    return 1
  fi

  mkdir -p "$(dirname "$CARGO_AUDIT_CONFIG")"
  # cargo-audit discovers policy from .cargo/audit.toml, not repo-root audit.toml.
  # Keep the generated helper in temp while preserving the repo-root policy intent.
  sed 's/^severity-threshold[[:space:]]*=/severity_threshold =/' \
    "$CARGO_AUDIT_POLICY" > "$CARGO_AUDIT_CONFIG"
}

prepare_cargo_audit_config

if [[ "$SELF_TEST" -eq 1 ]]; then
  if ! command -v cargo >/dev/null 2>&1; then
    echo "FAIL: cargo is required for Suite S self-test" >&2
    exit 2
  fi
  if ! cargo audit --version >/dev/null 2>&1; then
    echo "FAIL: cargo-audit is required for Suite S self-test" >&2
    exit 2
  fi
  if [[ "$CARGO_AUDIT_CONFIG" != "$SUITE_S_TMP_ROOT"/* ]]; then
    echo "FAIL: generated cargo-audit config is not under temp root" >&2
    exit 1
  fi
  if ! rg -q '^severity_threshold[[:space:]]*=[[:space:]]*"high"' "$CARGO_AUDIT_CONFIG"; then
    echo "FAIL: generated cargo-audit config does not carry the repo policy threshold" >&2
    exit 1
  fi
  if rg -q '^severity-threshold[[:space:]]*=' "$CARGO_AUDIT_CONFIG"; then
    echo "FAIL: generated cargo-audit config still uses unsupported severity-threshold spelling" >&2
    exit 1
  fi
  audit_settings="$(
    cd "$CARGO_AUDIT_WORKDIR"
    cargo audit --file "$CARGO_AUDIT_LOCKFILE" --no-fetch --stale --format json \
      | python3 -c 'import json,sys; print(json.load(sys.stdin)["settings"]["severity"])'
  )"
  if [[ "$audit_settings" != "high" ]]; then
    echo "FAIL: cargo-audit did not load generated policy; severity=$audit_settings" >&2
    exit 1
  fi
  echo "PASS: Suite S cargo-audit wrapper resolves repo lockfile and temp policy config"
  exit 0
fi

# Each entry: "label::command"
CHECKS=(
  "service-layer-boundary::./scripts/check_service_layer_boundary.sh"
  "no-let-underscore::bash src-tauri/scripts/check_no_let_underscore_in_writer_paths.sh"
  "write-fence-usage::./scripts/check_write_fence_usage.sh"
  "ability-surface-drift::bash src-tauri/scripts/check_ability_surface_drift.sh"
  "no-live-external-clients::bash src-tauri/scripts/check_no_live_external_clients_in_eval.sh"
  "fixture-anonymization::bash src-tauri/scripts/check_fixture_anonymization.sh"
  "durable-source-comments::./scripts/check_no_ephemeral_issue_refs_in_comments.sh"
  "oauth-secret-scan::bash \"$OAUTH_SECRET_SCAN_SCRIPT\""
  "clippy-deny-warnings::bash src-tauri/scripts/build-mcp.sh --stub && cargo clippy --manifest-path src-tauri/Cargo.toml --workspace --all-features --lib --bins -- -D warnings"
  "cargo-audit::cd \"$CARGO_AUDIT_WORKDIR\" && cargo audit --file \"$CARGO_AUDIT_LOCKFILE\""
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
  log_path="/tmp/suite-s-${label}.log"
  total=$((total + 1))

  echo "─── Suite S: $label ───" >&2
  if (cd "$REPO_ROOT" && eval "$cmd") >"$log_path" 2>&1; then
    status="pass"
  else
    status="fail"
    failed=$((failed + 1))
    cat "$log_path" >&2 || true
  fi

  if [[ $first -eq 1 ]]; then first=0; else results_json+=","; fi
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
