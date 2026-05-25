#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="${V146_VALIDATION_ROOT:-$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)}"
OUTPUT_DIR="${V146_VALIDATION_OUTPUT_DIR:-$ROOT_DIR/src-tauri/target/release-gate}"
REPORT_PATH="${V146_VALIDATION_REPORT:-$OUTPUT_DIR/v146-validation.json}"
GIT_SHA="${V146_VALIDATION_GIT_SHA:-$(cd "$ROOT_DIR" && git rev-parse HEAD 2>/dev/null || printf unknown)}"
AXIS="${1:-all}"

AXES=(backfill graph-audit trust signals contexts lifecycle filesystem redaction)

usage() {
  cat <<'USAGE'
Usage:
  bash tests/v146_validation/run.sh <axis|all>

Axes:
  backfill     W5-A conservative historical registration
  graph-audit  Explicit ingestion provenance graph assertions
  trust        Trust-band discipline
  signals      Workspace signal chain and prep invalidation
  contexts     Tauri plus MCP/headless context parity
  lifecycle    Source lifecycle and typed correction round trip
  filesystem   Negative filesystem/path validation cases
  redaction    Evidence redaction lint
USAGE
}

status_for_axis() {
  local axis="$1"

  case "$axis" in
    redaction)
      if (cd "$ROOT_DIR" && bash tests/v146_validation/redaction_lint.sh >/dev/null); then
        printf 'pass\tredaction-lint\tredaction lint passed\n'
      else
        printf 'fail\tredaction-lint\tredaction lint failed\n'
      fi
      ;;
    backfill)
      if (
        cd "$ROOT_DIR"
        cargo test --manifest-path src-tauri/Cargo.toml workspace_backfill --lib --bins >/dev/null
        cargo test --manifest-path src-tauri/Cargo.toml --test workspace_ingestion_w1_migrations w5_a_backfill_state_migration_creates_privacy_safe_run_item_operation_tables >/dev/null
      ); then
        printf 'pass\tcargo test workspace_backfill --lib --bins + W5-A migration coverage\tW5-A conservative backfill registration tests passed on the stacked base\n'
      else
        printf 'fail\tcargo test workspace_backfill --lib --bins + W5-A migration coverage\tW5-A conservative backfill registration tests failed on the stacked base\n'
      fi
      ;;
    graph-audit)
      printf 'blocked\tcargo test --test v146_validation graph_audit_zero_gaps_on_hermetic_fixture_db\tblocked until graph projection and explicit ingestion dependencies are merged/rebased\n'
      ;;
    trust)
      printf 'blocked\tcargo test --test v146_validation trust_band_discipline\tblocked until trust recompute path is available on the W5 base\n'
      ;;
    signals)
      printf 'blocked\tcargo test --test v146_validation signal_propagation_invalidates_prep\tblocked until WorkspaceFileIngested -> EntityIntelligenceUpdated -> prep invalidation is present\n'
      ;;
    contexts)
      printf 'blocked\tcargo test --test v146_validation context_inclusion_privacy_parity\tblocked until actual MCP/gateway or registered-handler path is available\n'
      ;;
    lifecycle)
      printf 'blocked\tcargo test --test v146_validation lifecycle_actions_and_user_correction_round_trip\tblocked until all named lifecycle actions are service-callable\n'
      ;;
    filesystem)
      printf 'blocked\tcargo test --test v146_validation filesystem_validation_negative_fixtures\tblocked until Rust negative fixtures are implemented against the merged base\n'
      ;;
    *)
      echo "unknown axis: $axis" >&2
      usage >&2
      return 2
      ;;
  esac
}

selected_axes() {
  if [ "$AXIS" = "all" ]; then
    printf '%s\n' "${AXES[@]}"
  else
    printf '%s\n' "$AXIS"
  fi
}

write_report() {
  local results_tsv="$1"

  mkdir -p "$OUTPUT_DIR"
  python3 - "$results_tsv" "$REPORT_PATH" "$GIT_SHA" <<'PY'
import csv
import json
import os
import sys
from datetime import datetime, timezone

tsv_path, report_path, git_sha = sys.argv[1:4]
axes = []
with open(tsv_path, newline="", encoding="utf-8") as handle:
    for row in csv.reader(handle, delimiter="\t"):
        if not row:
            continue
        axis, status, evidence_ref, summary = row
        axes.append(
            {
                "axis": axis,
                "status": status,
                "evidence_ref": evidence_ref,
                "summary": summary,
            }
        )

status_order = {"fail": 0, "blocked": 1, "pass": 2}
overall = "pass"
if any(axis["status"] == "fail" for axis in axes):
    overall = "fail"
elif any(axis["status"] == "blocked" for axis in axes):
    overall = "blocked"

report = {
    "schema_version": "v146_workspace_validation_v1",
    "generated_at": datetime.now(timezone.utc).isoformat().replace("+00:00", "Z"),
    "git_sha": git_sha,
    "status": overall,
    "total": len(axes),
    "passed": sum(1 for axis in axes if axis["status"] == "pass"),
    "failed": sum(1 for axis in axes if axis["status"] == "fail"),
    "blocked": sum(1 for axis in axes if axis["status"] == "blocked"),
    "axes": axes,
}

os.makedirs(os.path.dirname(report_path), exist_ok=True)
with open(report_path, "w", encoding="utf-8") as handle:
    json.dump(report, handle, indent=2, sort_keys=True)
    handle.write("\n")

if overall == "pass":
    sys.exit(0)
if overall == "fail":
    sys.exit(1)
sys.exit(2)
PY
}

main() {
  if [ "$AXIS" = "--help" ] || [ "$AXIS" = "-h" ]; then
    usage
    return 0
  fi

  local results_tsv
  results_tsv="$(mktemp)"
  trap "rm -f '$results_tsv'" EXIT

  local axis status evidence_ref summary
  while IFS= read -r axis; do
    if result="$(status_for_axis "$axis")"; then
      IFS=$'\t' read -r status evidence_ref summary <<<"$result"
      printf '%s\t%s\t%s\t%s\n' "$axis" "$status" "$evidence_ref" "$summary" >>"$results_tsv"
      printf 'v146 %s: %s\n' "$axis" "$status"
    else
      return $?
    fi
  done < <(selected_axes)

  write_report "$results_tsv"
}

main
