#!/usr/bin/env bash
set -u -o pipefail

ROOT_DIR="${W6_FIXTURE_ROOT:-$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)}"
OUTPUT_DIR="${W6_FIXTURE_OUTPUT_DIR:-$ROOT_DIR/src-tauri/target/release-gate}"
REPORT_PATH="$OUTPUT_DIR/w6-fixtures.json"
GIT_SHA="${W6_FIXTURE_GIT_SHA:-unknown}"
FIXTURES_HASH="${W6_FIXTURE_FIXTURES_HASH:-unknown}"

mkdir -p "$OUTPUT_DIR"
RESULTS_TSV="$(mktemp)"
trap 'rm "$RESULTS_TSV"' EXIT

run_fixture() {
	local id="$1"
	local owner_issue="$2"
	local expected="$3"
	local command="$4"
	local status="pass"

	printf 'W6 fixture %s: %s\n' "$id" "$expected"
	if (cd "$ROOT_DIR" && bash -lc "$command"); then
		status="pass"
	else
		status="fail"
	fi

	printf '%s\t%s\t%s\t%s\t%s\n' "$id" "$owner_issue" "$expected" "$command" "$status" >>"$RESULTS_TSV"
}

run_fixture "w6-01-default-wp-mcp-no-dailyos" "DOS-575" "zero dailyos/* tools exposed by default WP MCP" \
	"cd wp/dailyos && vendor/bin/phpunit --filter test_generic_mcp_server_enumerates_zero_dailyos_tools tests/mcp/McpExposureNoneTest.php"
run_fixture "w6-02-mcp-exposure-none-hidden" "DOS-575" "mcp_exposure None abilities remain hidden" \
	"cd wp/dailyos && vendor/bin/phpunit --filter 'test_build_allowlist_excludes_none_and_disallowed_categories|test_all_none_inventory_registers_no_dailyos_tools_with_adapter' tests/mcp/McpExposureNoneTest.php"
run_fixture "w6-03-frontend-js-no-dailyos-secrets" "DOS-575" "frontend JS contains no DailyOS secret-shaped material" \
	"bash scripts/release-gate/check-no-frontend-secrets.sh"
run_fixture "w6-04-gutenberg-rejects-raw-runtime-payloads" "DOS-575" "saved block content strips raw ability payload/provenance/sensitive shapes" \
	"cd wp/dailyos && vendor/bin/phpunit --filter test_block_serialization_rejects_raw_ability_payloads_provenance_and_unknown_sensitive_shapes tests/PresenceNonceTest.php"
run_fixture "w6-05-projection-tampered-typed-error" "DOS-575" "tampered projection returns typed projection_tampered" \
	"cargo test --manifest-path src-tauri/Cargo.toml --lib dos575_projection_tampered_maps_to_typed_http_error"
run_fixture "w6-06-stale-claim-version-feedback-409" "DOS-575" "stale claim_version returns HTTP 409 and writes no feedback mutation" \
	"cargo test --manifest-path src-tauri/Cargo.toml --lib dos571_fixture_claim_version_drift"
run_fixture "w6-07-cross-user-presence-nonce" "DOS-575" "presence nonce issued for user A rejects verification as user B" \
	"cargo test --manifest-path src-tauri/Cargo.toml --lib dos575_cross_user_presence_nonce_rejected"
run_fixture "w6-08-presence-nonce-replay-rejected" "DOS-575" "replay after verify is rejected as replayed" \
	"cargo test --manifest-path src-tauri/Cargo.toml --lib dos683_e2e_replay_rejection_after_verify"
run_fixture "w6-09-phase3-budget-charge-fail-closed" "DOS-719" "phase-3 feedback write failure charges failure budget and keeps nonce consumed" \
	"cargo test --manifest-path src-tauri/Cargo.toml --lib dos719_phase_three_failure_charges_failure_budget_fail_closed"
run_fixture "w6-10-direct-plugin-claim-table-write-lint" "DOS-575" "direct plugin claim-table writes are lint-blocked and self-test catches a planted violation" \
	"bash scripts/release-gate/check-no-direct-plugin-claim-table-writes.sh --self-test"
run_fixture "w6-11-payload-json-redaction" "LOCK-13" "payload_json is absent from nonce issue and verify response paths" \
	"cd wp/dailyos && vendor/bin/phpunit --filter 'test_verify_response_does_not_echo_payload_json_back_to_caller|test_issue_response_does_not_echo_payload_json_back_to_caller' tests/FeedbackPayloadRedactionTest.php"
run_fixture "w6-12-stock-theme-account-overview-render" "DOS-575" "account-overview renders trust band and visible provenance under stock-theme fallback" \
	"cd wp/dailyos && vendor/bin/phpunit --filter test_stock_theme_account_overview_renders_trust_band_and_provenance_markup tests/blocks/AccountOverviewBlockTest.php"
run_fixture "w6-13-cold-start-stale-marker-notice" "DOS-575" "stale marker transport error renders runtime_unavailable_notice, not is-empty" \
	"cd wp/dailyos && vendor/bin/phpunit --filter test_stale_marker_transport_error_renders_runtime_unavailable_notice_not_empty_state tests/blocks/AccountOverviewBlockTest.php"
run_fixture "w6-14-hot-tauri-restart-sentinel-discovery" "DOS-575" "hot Tauri restart sentinel discovery uses the new port" \
	"cd wp/dailyos && vendor/bin/phpunit --filter test_runtime_sentinel_cache_resets_after_restart_and_uses_new_port tests/transport/RuntimeClientTest.php"
run_fixture "w6-15-hot-studio-restart-first-render" "DOS-575" "first render after Studio boot succeeds from the first runtime response" \
	"cd wp/dailyos && vendor/bin/phpunit --filter test_hot_studio_restart_first_render_after_boot_succeeds tests/blocks/AccountOverviewBlockTest.php"

python3 - "$RESULTS_TSV" "$REPORT_PATH" "$GIT_SHA" "$FIXTURES_HASH" <<'PY'
import csv
import json
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
    "schema_version": "w6_negative_fixture_results_v1",
    "status": "pass" if not failed else "fail",
    "git_sha": git_sha,
    "fixtures_hash": fixtures_hash,
    "total": len(fixtures),
    "passed": len(fixtures) - len(failed),
    "failed": len(failed),
    "skipped": 0,
    "fixtures": fixtures,
}

with open(report_path, "w", encoding="utf-8") as handle:
    json.dump(report, handle, indent=2, sort_keys=True)
    handle.write("\n")

sys.exit(1 if failed else 0)
PY
report_status=$?

if [ "$report_status" -ne 0 ]; then
	echo "FAIL: one or more W6 fixtures failed; see $REPORT_PATH" >&2
	exit 1
fi

echo "PASS: W6 fixtures passed; report at $REPORT_PATH"
