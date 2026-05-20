#!/usr/bin/env bash
#
# v1.4.4 W1 consumer-skeleton lint (AC-W1.9).
#
# AC-W1.9 (verbatim): "scripts/check_w1_consumer_skeleton.sh (modeled on
# check_claim_writer_allowlist.sh): for each W1-shipped producer ... assert
# at least one block render PHP entry point under
# wp/dailyos/blocks/**/render-functions.php invokes it via the abilities
# runtime handle. Substrate-only PRs without at least one downstream
# consumer reference fail CI. The script IS the mechanical enforcement for
# AC-W1.2's 'wiring IS the work' obligation."
#
# Mechanism: for each named W1 producer in the inventory below, grep every
# wp/dailyos/blocks/*/render-functions.php for an invocation token. Missing
# producer-to-consumer pairs fail the lint with the pair named.
#
# Invocation: bash src-tauri/scripts/check_w1_consumer_skeleton.sh

set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"

# Optional override for self-test (negative-fixture harness sets this).
search_root="${DAILYOS_W1_CONSUMER_ROOT:-$repo_root}"

# W1 producers shipped in wave/v1.4.4-w1-stage1a.
#
# Each entry is "producer_identifier:human_label". The identifier is the
# token the consumer skeleton must contain (matching the ability name or
# service module path the WP runtime client invokes).
declare -a producers=(
  "get_entity_intelligence:get_entity_intelligence ability (account/project/person detail)"
  "get_daily_briefing:get_daily_briefing ability (daily briefing surface)"
  "claim_receipt:claim_receipt service (claim receipt audience builders)"
  "meeting_prep_status:meeting_prep_status state machine (prep status surface)"
  "record_claim_feedback:record_claim_feedback ability (claim feedback affordance)"
)

# Consumer search glob: every WP block render entry point.
consumers_glob="$search_root/wp/dailyos/blocks/*/render-functions.php"

# Expand glob; abort early if no consumer files exist at all (which would
# be a different shape of error — a wave with no WP blocks).
shopt -s nullglob
consumer_files=( $consumers_glob )
shopt -u nullglob

if [[ ${#consumer_files[@]} -eq 0 ]]; then
  echo "AC-W1.9 FAIL: no wp/dailyos/blocks/*/render-functions.php files found under $search_root/wp/dailyos/blocks/" >&2
  echo "Every W1 producer requires at least one downstream consumer skeleton." >&2
  exit 1
fi

missing_pairs=()

for entry in "${producers[@]}"; do
  identifier="${entry%%:*}"
  label="${entry#*:}"

  # grep across all consumer files; literal-string match. Producer
  # identifiers are unambiguous tokens (ability names / service paths),
  # so plain fixed-string grep is sufficient.
  if ! grep -lF "$identifier" "${consumer_files[@]}" >/dev/null 2>&1; then
    missing_pairs+=( "$identifier  ($label)" )
  fi
done

if [[ ${#missing_pairs[@]} -gt 0 ]]; then
  echo "AC-W1.9 FAIL: W1 producer(s) shipped without a downstream WP block consumer skeleton:" >&2
  for pair in "${missing_pairs[@]}"; do
    echo "  - $pair" >&2
  done
  echo >&2
  echo "Land at least one wp/dailyos/blocks/<name>/render-functions.php that invokes" >&2
  echo "each producer via the runtime client (DailyOS_Runtime_Client::invoke_ability or" >&2
  echo "::project_composition_for_surface). 'Wiring IS the work' per AC-W1.2." >&2
  exit 1
fi

echo "AC-W1.9 PASS: every W1 producer has at least one downstream WP block consumer skeleton."
