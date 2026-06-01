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
# Mechanism (cycle-2 hardening for codex-challenge F5):
#   For each named W1 producer in the inventory below, search every
#   wp/dailyos/blocks/*/render-functions.php for an invocation that uses the
#   PRODUCTION 3-arg signature
#     ->invoke_ability( '<producer>', <payload>, <scope_set> )
#   that the runtime client at wp/dailyos/includes/transport/
#   class-dailyos-runtime-client.php:85 actually accepts. A decorative stub
#   that only references the producer NAME (a comment, an unset+return [],
#   or a 2-arg invocation) no longer passes — invoking with the wrong arity
#   is treated as no consumer at all, because in production it would raise
#   a TypeError before reaching the runtime.
#
# Invocation:           bash src-tauri/scripts/check_w1_consumer_skeleton.sh
# Self-test (negative): DAILYOS_W1_CONSUMER_ROOT=<fixture> bash <this>

set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"

# Optional override for self-test (negative-fixture harness sets this).
search_root="${DAILYOS_W1_CONSUMER_ROOT:-$repo_root}"

# W1 producers shipped in wave/v1.4.4-w1-stage1a.
#
# Each entry is "producer_identifier:human_label". The identifier is the
# ability name passed as the first argument to invoke_ability(...).
declare -a producers=(
  "get_entity_intelligence:get_entity_intelligence ability (account/project/person detail)"
  "get_daily_briefing:get_daily_briefing ability (daily briefing surface)"
  "claim_receipt:claim_receipt ability (claim receipt audience builders)"
  "meeting_prep_status:meeting_prep_status ability (prep status surface)"
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

# Verify a producer is invoked with the 3-arg signature
#   ->invoke_ability( '<producer>' or "<producer>", <payload>, <scope_set> )
# in at least one consumer file.
#
# Detection strategy: scan each file with awk for an `->invoke_ability(` call,
# accumulate the body of the call across however many lines it spans until
# the matching close paren brings parenthesis depth back to zero, then count
# top-level comma-separated arguments (ignoring commas nested inside parens,
# brackets, braces, or string literals). Accept only when:
#   - first argument is a single-or-double-quoted '<producer>' literal, AND
#   - top-level argument count is exactly 3
# A 2-arg form (the bug F5 caught) does NOT pass.
producer_has_valid_consumer() {
  local producer="$1"
  local file
  for file in "${consumer_files[@]}"; do
    if awk -v producer="$producer" '
      function first_arg_is_producer(s, producer,    s2, ch, quote, end, name) {
        s2 = s
        sub(/^[ \t\r\n]+/, "", s2)
        ch = substr(s2, 1, 1)
        if (ch != "\x27" && ch != "\"") return 0
        quote = ch
        end = index(substr(s2, 2), quote)
        if (end == 0) return 0
        name = substr(s2, 2, end - 1)
        return (name == producer)
      }
      function count_top_level_args(s,    i, n, c, depth_p, depth_b, depth_c, in_s, q, prev, args) {
        n = length(s); depth_p = 0; depth_b = 0; depth_c = 0; in_s = 0; q = ""; args = 1; prev = ""
        for (i = 1; i <= n; i++) {
          c = substr(s, i, 1)
          if (in_s) {
            if (c == q && prev != "\\") { in_s = 0; q = "" }
          } else {
            if (c == "\"" || c == "\x27") { in_s = 1; q = c }
            else if (c == "(") depth_p++
            else if (c == ")") depth_p--
            else if (c == "[") depth_b++
            else if (c == "]") depth_b--
            else if (c == "{") depth_c++
            else if (c == "}") depth_c--
            else if (c == "," && depth_p == 0 && depth_b == 0 && depth_c == 0) args++
          }
          prev = c
        }
        return args
      }
      {
        line = $0
        if (collecting) {
          buf = buf " " line
        } else if (match(line, /->[ \t]*invoke_ability[ \t]*\(/)) {
          start = RSTART + RLENGTH
          buf = substr(line, start)
          collecting = 1
        } else {
          next
        }
        # Walk buf looking for the matching close paren (we are already past
        # the opening "(", so start at depth 1).
        n = length(buf); depth_p = 1; in_s = 0; q = ""; prev = ""; end_idx = 0
        for (i = 1; i <= n; i++) {
          c = substr(buf, i, 1)
          if (in_s) {
            if (c == q && prev != "\\") { in_s = 0; q = "" }
          } else {
            if (c == "\"" || c == "\x27") { in_s = 1; q = c }
            else if (c == "(") depth_p++
            else if (c == ")") { depth_p--; if (depth_p == 0) { end_idx = i; break } }
          }
          prev = c
        }
        if (end_idx == 0) next  # need more lines to close the call
        body = substr(buf, 1, end_idx - 1)
        collecting = 0; buf = ""
        if (first_arg_is_producer(body, producer)) {
          if (count_top_level_args(body) == 3) {
            found = 1
            exit 0
          }
        }
      }
      END { exit (found ? 0 : 1) }
    ' "$file"; then
      return 0
    fi
  done
  return 1
}

missing_pairs=()

for entry in "${producers[@]}"; do
  identifier="${entry%%:*}"
  label="${entry#*:}"

  if ! producer_has_valid_consumer "$identifier"; then
    missing_pairs+=( "$identifier  ($label)" )
  fi
done

if [[ ${#missing_pairs[@]} -gt 0 ]]; then
  echo "AC-W1.9 FAIL: W1 producer(s) shipped without a valid downstream WP block consumer skeleton:" >&2
  for pair in "${missing_pairs[@]}"; do
    echo "  - $pair" >&2
  done
  echo >&2
  echo "A valid consumer must invoke the producer via the runtime client's full" >&2
  echo "3-arg signature: \$client->invoke_ability( '<producer>', <payload>, <scope_set> )" >&2
  echo "per wp/dailyos/includes/transport/class-dailyos-runtime-client.php:85." >&2
  echo "Decorative stubs (no call, wrong arity, name-only mention) do not count." >&2
  echo "'Wiring IS the work' per AC-W1.2." >&2
  exit 1
fi

echo "AC-W1.9 PASS: every W1 producer has at least one valid downstream WP block consumer skeleton."
