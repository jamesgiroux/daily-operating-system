#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(cd "${SCRIPT_DIR}/../../.." && pwd)"
CONFIG="${ROOT_DIR}/wp/dailyos/scripts/grep-gates.json"
STATUS=0

cd "${ROOT_DIR}"

while IFS=$'\t' read -r ID_B64 DESCRIPTION_B64 PATTERN_B64 INCLUDE_B64 EXCLUDE_B64; do
	ID="$(php -r 'echo base64_decode($argv[1]);' "${ID_B64}")"
	DESCRIPTION="$(php -r 'echo base64_decode($argv[1]);' "${DESCRIPTION_B64}")"
	PATTERN="$(php -r 'echo base64_decode($argv[1]);' "${PATTERN_B64}")"
	INCLUDE_GLOB="$(php -r 'echo base64_decode($argv[1]);' "${INCLUDE_B64}")"
	EXCLUDE_GLOB="$(php -r 'echo base64_decode($argv[1]);' "${EXCLUDE_B64}")"

	RG_ARGS=(--pcre2 --multiline --line-number --with-filename --glob "${INCLUDE_GLOB}")

	if [[ -n "${EXCLUDE_GLOB}" ]]; then
		RG_ARGS+=(--glob "!${EXCLUDE_GLOB}")
	fi

	set +e
	OUTPUT="$(rg "${RG_ARGS[@]}" -- "${PATTERN}" .)"
	RC=$?
	set -e

	if [[ ${RC} -eq 0 ]]; then
		printf 'grep gate failed: %s\n%s\n%s\n' "${ID}" "${DESCRIPTION}" "${OUTPUT}"
		STATUS=1
	elif [[ ${RC} -gt 1 ]]; then
		printf 'grep gate error: %s\n%s\n' "${ID}" "${OUTPUT}"
		STATUS=1
	fi
done < <(
	php -r '
		$config = json_decode(file_get_contents($argv[1]), true, 512, JSON_THROW_ON_ERROR);
		foreach ($config["gates"] as $gate) {
			echo base64_encode($gate["id"]) . "\t";
			echo base64_encode($gate["description"]) . "\t";
			echo base64_encode($gate["pattern"]) . "\t";
			echo base64_encode($gate["include_glob"]) . "\t";
			echo base64_encode($gate["exclude_glob"] ?? "") . "\n";
		}
	' "${CONFIG}"
)

exit "${STATUS}"
