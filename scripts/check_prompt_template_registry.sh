#!/usr/bin/env bash
# Validate ADR-0106 prompt template registry integrity.
#
# Rules:
# - manifest.toml entries match {name}.v{version}.txt files
# - template files are normalized: Unix line endings, no trailing whitespace,
#   no tabs, exactly one trailing newline
# - manifest sha256 matches the canonicalized template bytes
# - existing template files are immutable; prompt changes require adding a new
#   versioned file and manifest entry

set -euo pipefail

if [ "${1:-}" != "" ]; then
  ROOT_DIR="$(cd "$1" && pwd)"
else
  ROOT_DIR="$(git rev-parse --show-toplevel)"
fi

PROMPT_DIR="$ROOT_DIR/src-tauri/src/abilities/prompts"
PROMPT_GIT_DIR="${PROMPT_DIR#$ROOT_DIR/}"
MANIFEST="$PROMPT_DIR/manifest.toml"

if [ ! -f "$MANIFEST" ]; then
  echo "prompt registry: missing manifest: $MANIFEST"
  exit 1
fi

if ! command -v perl >/dev/null 2>&1; then
  echo "prompt registry: missing required tool: perl"
  exit 1
fi

hash_stream() {
  if command -v shasum >/dev/null 2>&1; then
    shasum -a 256 | awk '{print $1}'
  elif command -v sha256sum >/dev/null 2>&1; then
    sha256sum | awk '{print $1}'
  else
    echo "prompt registry: missing shasum or sha256sum" >&2
    return 1
  fi
}

canonical_hash() {
  local file="$1"
  perl -0pe 's/\r\n?/\n/g; s/[ \t]+(?=\n)//g; s/\n+\z/\n/s' "$file" | hash_stream
}

resolve_default_base_ref() {
  if [ -n "${BASE_REF:-}" ]; then
    echo "$BASE_REF"
    return 0
  fi

  if [ -n "${GITHUB_BASE_REF:-}" ]; then
    echo "origin/$GITHUB_BASE_REF"
    return 0
  fi

  local candidate
  for candidate in origin/dev public/dev origin/trunk public/trunk; do
    if git -C "$ROOT_DIR" rev-parse --verify "$candidate^{commit}" >/dev/null 2>&1; then
      echo "$candidate"
      return 0
    fi
  done
}

resolve_base_commit() {
  local base_ref="$1"
  if [ -z "$base_ref" ]; then
    return 0
  fi

  if ! git -C "$ROOT_DIR" rev-parse --verify "$base_ref^{commit}" >/dev/null 2>&1; then
    if [ -n "${GITHUB_BASE_REF:-}" ] && [ "$base_ref" = "origin/$GITHUB_BASE_REF" ]; then
      git -C "$ROOT_DIR" fetch --quiet --depth=1 origin \
        "$GITHUB_BASE_REF:refs/remotes/origin/$GITHUB_BASE_REF" >/dev/null 2>&1 || true
    fi
  fi

  if ! git -C "$ROOT_DIR" rev-parse --verify "$base_ref^{commit}" >/dev/null 2>&1; then
    if [ -n "${BASE_REF:-}" ] || [ -n "${GITHUB_BASE_REF:-}" ]; then
      echo "prompt registry: unable to resolve base ref '$base_ref'" >&2
      return 1
    fi
    return 0
  fi

  git -C "$ROOT_DIR" merge-base "$base_ref" HEAD 2>/dev/null \
    || git -C "$ROOT_DIR" rev-parse "$base_ref^{commit}"
}

base_ref="$(resolve_default_base_ref)"
base_commit=""
if git -C "$ROOT_DIR" rev-parse --verify HEAD >/dev/null 2>&1; then
  base_commit="$(resolve_base_commit "$base_ref")"
fi

entries_file="$(mktemp)"
listed_paths_file="$(mktemp)"
base_template_paths_file="$(mktemp)"
trap 'rm -f "$entries_file" "$listed_paths_file" "$base_template_paths_file"' EXIT

awk '
function trim(s) {
  gsub(/^[ \t]+|[ \t]+$/, "", s)
  return s
}
function unquote(s) {
  s = trim(s)
  gsub(/^"/, "", s)
  gsub(/"$/, "", s)
  return s
}
function emit() {
  if (id != "" || version != "" || path != "" || sha != "") {
    print id "\t" version "\t" path "\t" sha
  }
}
/^[ \t]*\[\[template\]\][ \t]*$/ {
  emit()
  id = ""; version = ""; path = ""; sha = ""
  next
}
/^[ \t]*id[ \t]*=/ {
  split($0, parts, "=")
  id = unquote(parts[2])
  next
}
/^[ \t]*version[ \t]*=/ {
  split($0, parts, "=")
  version = unquote(parts[2])
  next
}
/^[ \t]*path[ \t]*=/ {
  split($0, parts, "=")
  path = unquote(parts[2])
  next
}
/^[ \t]*sha256[ \t]*=/ {
  split($0, parts, "=")
  sha = unquote(parts[2])
  next
}
END { emit() }
' "$MANIFEST" > "$entries_file"

if [ ! -s "$entries_file" ]; then
  echo "prompt registry: manifest has no [[template]] entries"
  exit 1
fi

violations=0
seen_keys="$(mktemp)"
trap 'rm -f "$entries_file" "$listed_paths_file" "$base_template_paths_file" "$seen_keys"' EXIT

while IFS="$(printf '\t')" read -r id version rel_path sha; do
  if [ -z "$id" ] || [ -z "$version" ] || [ -z "$rel_path" ] || [ -z "$sha" ]; then
    echo "prompt registry: incomplete manifest entry id='$id' version='$version' path='$rel_path'"
    violations=$((violations + 1))
    continue
  fi

  key="$id@$version"
  if grep -Fxq "$key" "$seen_keys"; then
    echo "prompt registry: duplicate template id/version: $key"
    violations=$((violations + 1))
  fi
  echo "$key" >> "$seen_keys"

  expected_path="${id}.v${version}.txt"
  if [ "$rel_path" != "$expected_path" ]; then
    echo "prompt registry: $key path must be $expected_path, got $rel_path"
    violations=$((violations + 1))
  fi

  file="$PROMPT_DIR/$rel_path"
  echo "$file" >> "$listed_paths_file"
  if [ ! -f "$file" ]; then
    echo "prompt registry: missing template file: $rel_path"
    violations=$((violations + 1))
    continue
  fi

  if perl -ne 'if (/\r/) { print "$ARGV:$.: CR line ending\n"; $bad=1 } END { exit($bad ? 1 : 0) }' "$file"; then
    :
  else
    violations=$((violations + 1))
  fi

  trailing_ws="$(grep -nE '[[:blank:]]$' "$file" || true)"
  if [ -n "$trailing_ws" ]; then
    echo "prompt registry: trailing whitespace in $rel_path"
    echo "$trailing_ws"
    violations=$((violations + 1))
  fi

  tab_hits="$(grep -n "$(printf '\t')" "$file" || true)"
  if [ -n "$tab_hits" ]; then
    echo "prompt registry: tab characters in $rel_path"
    echo "$tab_hits"
    violations=$((violations + 1))
  fi

  if ! perl -0ne 'exit(/\n\z/ ? 0 : 1)' "$file"; then
    echo "prompt registry: $rel_path must end with a newline"
    violations=$((violations + 1))
  fi
  if perl -0ne 'exit(/\n\n\z/ ? 0 : 1)' "$file"; then
    echo "prompt registry: $rel_path must have exactly one trailing newline"
    violations=$((violations + 1))
  fi

  actual_sha="$(canonical_hash "$file")"
  if [ "$actual_sha" != "$sha" ]; then
    echo "prompt registry: sha256 mismatch for $rel_path"
    echo "  manifest: $sha"
    echo "  actual:   $actual_sha"
    violations=$((violations + 1))
  fi

  if git -C "$ROOT_DIR" rev-parse --verify HEAD >/dev/null 2>&1; then
    git_rel="${file#$ROOT_DIR/}"
    if git -C "$ROOT_DIR" cat-file -e "HEAD:$git_rel" 2>/dev/null; then
      if ! git -C "$ROOT_DIR" diff --quiet -- "$git_rel"; then
        echo "prompt registry: existing template version changed: $git_rel"
        echo "  Add a new ${id}.v<next-version>.txt file and manifest entry instead."
        violations=$((violations + 1))
      fi
    fi
  fi
done < "$entries_file"

if [ -n "$base_commit" ]; then
  git -C "$ROOT_DIR" ls-tree -r --name-only "$base_commit" -- "$PROMPT_GIT_DIR" \
    | grep -E '\.txt$' > "$base_template_paths_file" || true

  while IFS= read -r base_template_path; do
    if [ -z "$base_template_path" ]; then
      continue
    fi

    if ! git -C "$ROOT_DIR" cat-file -e "HEAD:$base_template_path" 2>/dev/null; then
      echo "prompt registry: existing template version removed: $base_template_path"
      echo "  Add a new versioned template file and manifest entry instead."
      violations=$((violations + 1))
      continue
    fi

    if ! git -C "$ROOT_DIR" diff --quiet "$base_commit" HEAD -- "$base_template_path"; then
      echo "prompt registry: existing template version changed: $base_template_path"
      echo "  Add a new versioned template file and manifest entry instead."
      violations=$((violations + 1))
    fi
  done < "$base_template_paths_file"
fi

while IFS= read -r template_file; do
  if ! grep -Fxq "$template_file" "$listed_paths_file"; then
    echo "prompt registry: template file is not listed in manifest: ${template_file#$PROMPT_DIR/}"
    violations=$((violations + 1))
  fi
done < <(find "$PROMPT_DIR" -maxdepth 1 -type f -name '*.txt' | sort)

if [ "$violations" -gt 0 ]; then
  echo
  echo "ERROR: prompt template registry check failed with $violations violation(s)."
  exit 1
fi

echo "prompt registry: ok"
