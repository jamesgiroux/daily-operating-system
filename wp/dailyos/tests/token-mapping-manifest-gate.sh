#!/usr/bin/env bash
set -euo pipefail

repo_root="${DAILYOS_TOKEN_MAPPING_ROOT:-$(pwd)}"
blocks_dir="${repo_root}/wp/dailyos/blocks"
theme_json="${repo_root}/wp/dailyos/theme/theme.json"

if [ ! -d "$blocks_dir" ]; then
	echo "No WordPress block directory found at ${blocks_dir}; token-mapping manifest gate skipped."
	exit 0
fi

block_dirs=()
while IFS= read -r block_dir; do
	block_dirs+=("$block_dir")
done < <(find "$blocks_dir" -mindepth 1 -maxdepth 1 -type d | sort)
if [ "${#block_dirs[@]}" -eq 0 ]; then
	echo "No WordPress block subdirectories found; token-mapping manifest gate skipped."
	exit 0
fi

if [ ! -f "$theme_json" ]; then
	echo "FAIL: theme.json missing at ${theme_json}" >&2
	exit 1
fi

palette_file="$(mktemp)"
cleanup() {
	rm -f "$palette_file"
}
trap cleanup EXIT

jq -r '.settings.color.palette[]?.slug // empty' "$theme_json" | sort -u > "$palette_file"

raw_color_escape_files=(
	"wp/dailyos/blocks/account-overview/style.css"
)

is_raw_color_escape_file() {
	local rel_path="$1"
	local allowed
	for allowed in "${raw_color_escape_files[@]}"; do
		if [ "$rel_path" = "$allowed" ]; then
			return 0
		fi
	done
	return 1
}

relative_path() {
	local abs_path="$1"
	printf '%s\n' "${abs_path#"$repo_root"/}"
}

for block_dir in "${block_dirs[@]}"; do
	style_file="${block_dir}/style.css"
	manifest_file="${block_dir}/.token-mapping.json"
	rel_style="$(relative_path "$style_file")"

	if [ ! -f "$style_file" ]; then
		continue
	fi

	style_tokens=()
	while IFS= read -r style_token; do
		style_tokens+=("${style_token#--}")
	done < <(grep -Eho -- '--wp--[a-z0-9-]+' "$style_file" | sort -u || true)
	if [ "${#style_tokens[@]}" -gt 0 ] && [ ! -f "$manifest_file" ]; then
		echo "FAIL: ${rel_style} uses WordPress token vars but ${manifest_file} is missing" >&2
		exit 1
	fi

	if [ -f "$manifest_file" ]; then
		jq -e '
			type == "array"
			and all(.[]; (.source_token | type == "string") and (.target_token | type == "string"))
		' "$manifest_file" >/dev/null

		manifest_tokens=()
		while IFS= read -r manifest_token; do
			manifest_tokens+=("$manifest_token")
		done < <(jq -r '.[].target_token' "$manifest_file" | sort -u)
		for style_token in "${style_tokens[@]}"; do
			if ! printf '%s\n' "${manifest_tokens[@]}" | grep -Fxq "$style_token"; then
				echo "FAIL: ${rel_style} uses ${style_token}, missing from .token-mapping.json" >&2
				exit 1
			fi
		done

		while IFS= read -r target_token; do
			[ -n "$target_token" ] || continue
			slug="${target_token#wp--preset--color--}"
			if ! grep -Fxq "$slug" "$palette_file"; then
				echo "FAIL: ${manifest_file} references ${target_token}, but ${slug} is not defined in theme.json settings.color.palette" >&2
				exit 1
			fi
		done < <(jq -r '.[] | select(.target_token | startswith("wp--preset--color--")) | .target_token' "$manifest_file")
	fi

	raw_color_output="$(mktemp)"
	if grep -Eni '#[0-9a-f]{3,8}\b|rgba?\(|hsla?\(' "$style_file" >"$raw_color_output" 2>/dev/null; then
		if ! is_raw_color_escape_file "$rel_style"; then
			echo "FAIL: ${rel_style} contains raw color literal(s):" >&2
			cat "$raw_color_output" >&2
			rm -f "$raw_color_output"
			exit 1
		fi
	fi
	rm -f "$raw_color_output"
done

echo "Token-mapping manifest gate passed (${#block_dirs[@]} block director$( [ "${#block_dirs[@]}" -eq 1 ] && printf 'y' || printf 'ies' ) scanned)."
