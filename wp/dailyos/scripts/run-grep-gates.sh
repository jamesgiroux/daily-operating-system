#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(cd "${SCRIPT_DIR}/../../.." && pwd)"
CONFIG="${ROOT_DIR}/wp/dailyos/scripts/grep-gates.json"
STATUS=0

cd "${ROOT_DIR}"

while IFS='|' read -r ID_B64 DESCRIPTION_B64 PATTERN_B64 INCLUDE_B64 EXCLUDE_B64 BASELINE_B64; do
	ID="$(php -r 'echo base64_decode($argv[1]);' "${ID_B64}")"
	DESCRIPTION="$(php -r 'echo base64_decode($argv[1]);' "${DESCRIPTION_B64}")"
	PATTERN="$(php -r 'echo base64_decode($argv[1]);' "${PATTERN_B64}")"
	INCLUDE_GLOB="$(php -r 'echo base64_decode($argv[1]);' "${INCLUDE_B64}")"
	EXCLUDE_GLOB="$(php -r 'echo base64_decode($argv[1]);' "${EXCLUDE_B64}")"
	BASELINE_FILE="$(php -r 'echo base64_decode($argv[1]);' "${BASELINE_B64}")"

	RG_ARGS=(--pcre2 --multiline --line-number --with-filename --glob "${INCLUDE_GLOB}")

	if [[ -n "${EXCLUDE_GLOB}" ]]; then
		RG_ARGS+=(--glob "!${EXCLUDE_GLOB}")
	fi

	set +e
	OUTPUT="$(rg "${RG_ARGS[@]}" -- "${PATTERN}" .)"
	RC=$?
	set -e

	if [[ ${RC} -eq 0 ]]; then
		if [[ -n "${BASELINE_FILE}" ]]; then
			BASELINE_PATH="${ROOT_DIR}/${BASELINE_FILE}"
			if [[ -f "${BASELINE_PATH}" ]]; then
				OUTPUT="$(
					printf '%s\n' "${OUTPUT}" | awk -F: -v baseline="${BASELINE_PATH}" '
						BEGIN {
							while ((getline line < baseline) > 0) {
								allowed[line] = 1
							}
						}
						{
							path = $1
							sub(/^\.\//, "", path)
							content = $0
							sub(/^[^:]+:[0-9]+:/, "", content)
							key = path "\t" content
							if (!(key in allowed)) {
								print $0
							}
						}
					'
				)"
				if [[ -z "${OUTPUT}" ]]; then
					RC=1
				fi
			else
				printf 'grep gate missing baseline: %s\n%s\n' "${ID}" "${BASELINE_PATH}"
				STATUS=1
				continue
			fi
		fi
	fi

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
			echo base64_encode($gate["id"]) . "|";
			echo base64_encode($gate["description"]) . "|";
			echo base64_encode($gate["pattern"]) . "|";
			echo base64_encode($gate["include_glob"]) . "|";
			echo base64_encode($gate["exclude_glob"] ?? "") . "|";
			echo base64_encode($gate["baseline_file"] ?? "") . "\n";
		}
	' "${CONFIG}"
)

exit "${STATUS}"
