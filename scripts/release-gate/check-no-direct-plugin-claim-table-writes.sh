#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="${W6_FIXTURE_ROOT:-$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)}"
SELF_TEST=0

if [ "${1:-}" = "--self-test" ]; then
	SELF_TEST=1
fi

if ! command -v rg >/dev/null 2>&1; then
	echo "FAIL: rg is required for claim-table write lint" >&2
	exit 2
fi

scan_root() {
	local root="$1"
	local -a paths=()
	for candidate in "$root/wp/dailyos/includes" "$root/wp/dailyos/blocks" "$root/wp/dailyos/dailyos.php"; do
		if [ -e "$candidate" ]; then
			paths+=("$candidate")
		fi
	done

	if [ "${#paths[@]}" -eq 0 ]; then
		echo "FAIL: no plugin source roots found under $root" >&2
		return 2
	fi

	local table_pattern='(claim_feedback|intelligence_claims|dailyos_claims|dailyos_claim|projection_ledger|surface_feedback_nonces)'
	local write_pattern='(\$wpdb\s*->\s*(query|insert|update|delete|replace)\s*\([^;\n]*(INSERT|UPDATE|DELETE|REPLACE)?[^;\n]*'"$table_pattern"'|(INSERT|UPDATE|DELETE|REPLACE)[^;\n]*'"$table_pattern"')'

	if rg -n -P --glob '*.php' "$write_pattern" "${paths[@]}"; then
		echo "FAIL: direct plugin claim-table write detected; route writes through the Rust substrate/API boundary" >&2
		return 1
	fi

	return 0
}

if [ "$SELF_TEST" -eq 1 ]; then
	tmp_root="$(mktemp -d)"
	self_out="$tmp_root/self-test.out"
	trap 'rm -r "$tmp_root"' EXIT
	mkdir -p "$tmp_root/wp/dailyos/includes"
	printf '%s\n' '<?php $wpdb->query("INSERT INTO claim_feedback (id) VALUES ('"'"'fixture'"'"')");' > "$tmp_root/wp/dailyos/includes/planted.php"
	if scan_root "$tmp_root" >"$self_out" 2>&1; then
		cat "$self_out" >&2
		echo "FAIL: self-test planted write was not caught" >&2
		exit 1
	fi
fi

scan_root "$ROOT_DIR"
echo "PASS: plugin source contains no direct claim-table writes"
