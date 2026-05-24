#!/usr/bin/env bash
set -u -o pipefail

ROOT_DIR="${V144A_W6_FIXTURE_ROOT:-${W6_FIXTURE_ROOT:-$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)}}"
OUTPUT_DIR="${V144A_W6_FIXTURE_OUTPUT_DIR:-${W6_FIXTURE_OUTPUT_DIR:-$ROOT_DIR/src-tauri/target/release-gate}}"
REPORT_PATH="${V144A_W6_FIXTURE_REPORT:-${W6_FIXTURE_REPORT:-$OUTPUT_DIR/v144a-w6-fixtures.json}}"
GIT_SHA="${V144A_W6_FIXTURE_GIT_SHA:-${W6_FIXTURE_GIT_SHA:-$(cd "$ROOT_DIR" && git rev-parse HEAD 2>/dev/null || printf unknown)}}"
FIXTURES_HASH="${V144A_W6_FIXTURE_FIXTURES_HASH:-${W6_FIXTURE_FIXTURES_HASH:-unknown}}"

mkdir -p "$OUTPUT_DIR"
RESULTS_TSV="$(mktemp)"
trap 'rm -f "$RESULTS_TSV"' EXIT

cargo_filtered_test() {
  local expected_test="$1"
  shift
  local list_output

  if ! list_output=$("$@" -- --list 2>&1); then
    printf '%s\n' "$list_output" >&2
    return 1
  fi
  if ! printf '%s\n' "$list_output" | grep -F -- "$expected_test" >/dev/null; then
    printf 'FAIL: expected cargo test filter not listed: %s\n' "$expected_test" >&2
    printf '%s\n' "$list_output" >&2
    return 1
  fi

  "$@"
}

run_fixture() {
  local id="$1"
  local owner_issue="$2"
  local expected="$3"
  local command="$4"
  local status="pass"

  printf 'v1.4.4a W6 fixture %s: %s\n' "$id" "$expected"
  mkdir -p "$OUTPUT_DIR" "$ROOT_DIR/src-tauri/target/debug/deps"
  if (cd "$ROOT_DIR" && eval "$command"); then
    status="pass"
  else
    status="fail"
  fi

  printf '%s\t%s\t%s\t%s\t%s\n' "$id" "$owner_issue" "$expected" "$command" "$status" >>"$RESULTS_TSV"
}

run_fixture "v144a-w6-01-frontend-surface-contracts" "DOS-514" "Daily, meeting, entity, actions, and email-adjacent surface tests pass against claim-backed readers" \
  "pnpm test -- src/components/dashboard/DailyBriefing.test.tsx src/hooks/useDailyBriefingAbility.test.tsx src/hooks/useMeetingEntityIntelligence.test.tsx src/hooks/useEntityDetailIntelligence.test.tsx src/services/entity-intelligence/entity-detail-mapper.test.ts src/components/shared/SuggestedActionRow.test.tsx src/components/work/WorkSurface.test.tsx src/lib/email-ranking.test.ts"

run_fixture "v144a-w6-02-daily-briefing-bounded-expansion" "DOS-514" "Daily Briefing expands current, next, and visible upcoming meetings only" \
  "cargo_filtered_test daily_briefing_expansion_ids_are_bounded_to_visible_page_current_and_next cargo test --manifest-path src-tauri/abilities-runtime/Cargo.toml --lib daily_briefing_expansion_ids_are_bounded_to_visible_page_current_and_next && cargo_filtered_test daily_briefing_producer_expands_only_current_next_and_requested_page cargo test --manifest-path src-tauri/abilities-runtime/Cargo.toml --lib daily_briefing_producer_expands_only_current_next_and_requested_page && cargo_filtered_test daily_briefing_entity_sections_omit_open_loops_for_aggregate_pass cargo test --manifest-path src-tauri/abilities-runtime/Cargo.toml --lib daily_briefing_entity_sections_omit_open_loops_for_aggregate_pass"

run_fixture "v144a-w6-03-daily-briefing-cursor" "DOS-514" "Daily Briefing upcoming cursor remains stable after bounded expansion" \
  "cargo_filtered_test upcoming_meetings_cursor_roundtrip cargo test --manifest-path src-tauri/abilities-runtime/Cargo.toml --lib upcoming_meetings_cursor_roundtrip"

run_fixture "v144a-w6-04-entity-claim-read-cap" "DOS-514" "Entity intelligence claim reader applies prompt-safe filtering, subject indexes, and a page cap before rendering" \
  "cargo_filtered_test entity_context_surface_limited_reader_caps_visible_claims cargo test --manifest-path src-tauri/Cargo.toml --lib entity_context_surface_limited_reader_caps_visible_claims && cargo_filtered_test entity_context_surface_limited_reader_applies_global_cap_across_related_subjects cargo test --manifest-path src-tauri/Cargo.toml --lib entity_context_surface_limited_reader_applies_global_cap_across_related_subjects && cargo_filtered_test entity_context_subject_lookup_uses_expression_index cargo test --manifest-path src-tauri/Cargo.toml --lib entity_context_subject_lookup_uses_expression_index && cargo_filtered_test entity_context_prompt_claim_reader_filters_sensitivity_before_cap cargo test --manifest-path src-tauri/Cargo.toml --lib entity_context_prompt_claim_reader_filters_sensitivity_before_cap && cargo_filtered_test agent_and_mcp_producer_read_prompt_safe_claims_before_page_cap cargo test --manifest-path src-tauri/abilities-runtime/Cargo.toml --lib agent_and_mcp_producer_read_prompt_safe_claims_before_page_cap && cargo_filtered_test migration_262_adds_claim_subject_lookup_index cargo test --manifest-path src-tauri/Cargo.toml --lib migration_262_adds_claim_subject_lookup_index"

run_fixture "v144a-w6-05-mcp-action-redaction" "DOS-514" "Action-derived open-loop claims are not synthesized into MCP prompt context without claim sensitivity" \
  "cargo_filtered_test action_open_loop_synthesis_is_not_mcp_visible_without_claim_sensitivity cargo test --manifest-path src-tauri/Cargo.toml --lib action_open_loop_synthesis_is_not_mcp_visible_without_claim_sensitivity && cargo_filtered_test mcp_action_db_reader_filters_prompt_unsafe_claims_before_page_cap cargo test --manifest-path src-tauri/Cargo.toml --lib mcp_action_db_reader_filters_prompt_unsafe_claims_before_page_cap"

run_fixture "v144a-w6-06-entity-fixture-harness" "DOS-514" "Entity fixture harness still passes after claim-reader routing changes" \
  "cargo test --manifest-path src-tauri/Cargo.toml --test entity_fixture_harness"

run_fixture "v144a-w6-07-foreground-contention" "DOS-514" "Foreground DB contention regression harness stays green" \
  "cargo test --manifest-path src-tauri/Cargo.toml --test foreground_db_contention_regression"

run_fixture "v144a-w6-08-email-refresh-coalescing" "DOS-514" "Email refresh events coalesce so foreground reads do not stampede after background updates" \
  "pnpm test -- src/pages/EmailsPage.test.tsx"

run_fixture "v144a-w6-09-dos412-render-policy-drift" "DOS-514" "Legacy render-policy migration fixture remains compatible with current surface rules" \
  "cargo test --manifest-path src-tauri/Cargo.toml --test dos412_render_policy_test"

run_fixture "v144a-w6-10-dos168-mcp-migration-drift" "DOS-514" "Legacy MCP v2 migration smoke test remains compatible with current nonce cleanup rules" \
  "cargo_filtered_test dos168_v255_v261_migrations_land_canonical_schema cargo test --manifest-path src-tauri/Cargo.toml --test dos168_mcp_v2_migration_smoke_test"

python3 - "$RESULTS_TSV" "$REPORT_PATH" "$GIT_SHA" "$FIXTURES_HASH" <<'PY'
import csv
import json
import os
import sys

tsv_path, report_path, git_sha, fixtures_hash = sys.argv[1:5]
fixtures = []
with open(tsv_path, newline="", encoding="utf-8") as handle:
    for row in csv.reader(handle, delimiter="\t"):
        if not row:
            continue
        fixture_id, owner_issue, expected, command, status = row
        fixtures.append(
            {
                "id": fixture_id,
                "owner_issue": owner_issue,
                "expected": expected,
                "proof_command": command,
                "status": status,
            }
        )

failed = [fixture for fixture in fixtures if fixture["status"] != "pass"]
report = {
    "schema_version": "v144a_w6_fixture_results_v1",
    "status": "pass" if not failed else "fail",
    "git_sha": git_sha,
    "fixtures_hash": fixtures_hash,
    "total": len(fixtures),
    "passed": len(fixtures) - len(failed),
    "failed": len(failed),
    "skipped": 0,
    "fixtures": fixtures,
}

os.makedirs(os.path.dirname(report_path), exist_ok=True)
with open(report_path, "w", encoding="utf-8") as handle:
    json.dump(report, handle, indent=2, sort_keys=True)
    handle.write("\n")

sys.exit(1 if failed else 0)
PY
report_status=$?

if [ "$report_status" -ne 0 ]; then
  echo "FAIL: one or more v1.4.4a W6 fixtures failed; see $REPORT_PATH" >&2
  exit 1
fi

echo "PASS: v1.4.4a W6 fixtures passed; report at $REPORT_PATH"
