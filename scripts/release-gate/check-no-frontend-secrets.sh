#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="${W6_FIXTURE_ROOT:-$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)}"

if ! command -v rg >/dev/null 2>&1; then
	echo "FAIL: rg is required for frontend secret scan" >&2
	exit 2
fi

paths=()
for candidate in "$ROOT_DIR/wp/dailyos/blocks" "$ROOT_DIR/wp/dailyos/src"; do
	if [ -d "$candidate" ]; then
		paths+=("$candidate")
	fi
done

if [ "${#paths[@]}" -eq 0 ]; then
	echo "FAIL: no frontend source roots found" >&2
	exit 2
fi

pattern='(hmac_key|session_key|site_nonce_full|site_nonce_hash|site_binding_digest|runtime_instance_id|surface_client_id|paired_wp_user_id|X-DailyOS-Signature|X-DailyOS-Session-Id|Authorization:[[:space:]]*Bearer|Bearer[[:space:]]+[A-Za-z0-9._-]{10,}|127\.0\.0\.1:[0-9]{2,5}|localhost:[0-9]{2,5})'

if rg -n -i --glob '*.js' --glob '*.jsx' --glob '*.ts' --glob '*.tsx' --glob '*.asset.php' "$pattern" "${paths[@]}"; then
	echo "FAIL: frontend bundle/source exposes DailyOS transport secret-shaped material" >&2
	exit 1
fi

echo "PASS: frontend bundle/source contains no DailyOS secret-shaped material"
