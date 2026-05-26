#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="${V146_VALIDATION_ROOT:-$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)}"
SELF_TEST="${1:-}"

usage() {
  cat <<'USAGE'
Usage:
  bash tests/v146_validation/redaction_lint.sh [artifact ...]
  bash tests/v146_validation/redaction_lint.sh --self-test

Scans W5-B validation artifacts for evidence shapes that are not allowed in
committed docs, PR bodies, generated logs, proof bundles, or Linear-ready text.
USAGE
}

lint_targets() {
  local targets=("$@")

  if [ "${#targets[@]}" -eq 0 ]; then
    while IFS= read -r target; do
      targets+=("$target")
    done < <(default_targets)
  fi

  if [ "${#targets[@]}" -eq 0 ]; then
    echo "PASS v146 redaction lint: no existing artifacts to scan"
    return 0
  fi

  (cd "$ROOT_DIR" && node scripts/lint-evidence-artifacts.mjs "${targets[@]}")
  python3 - "$ROOT_DIR" "${targets[@]}" <<'PY'
import pathlib
import re
import sys

root = pathlib.Path(sys.argv[1]).resolve()
targets = sys.argv[2:]

blocked_key = re.compile(
    r'(?i)(["\']?(?:claim_text|file_content|prompt_text|output_body|raw_path|raw_file_id|raw_source_handle|raw_content_hash|provenance_blob)["\']?\s*[:=])'
)
workspace_source_ref = re.compile(r'\bworkspace_file:[A-Za-z0-9_-]{8,}\b')

violations = []
for target in targets:
    path = (root / target).resolve()
    if not path.exists():
        continue
    if path.is_dir():
        files = [p for p in path.rglob("*") if p.is_file()]
    else:
        files = [path]
    for file_path in files:
        try:
            text = file_path.read_text(encoding="utf-8")
        except UnicodeDecodeError:
            violations.append((file_path, 1, "binary-or-non-utf8-artifact"))
            continue
        for index, line in enumerate(text.splitlines(), start=1):
            if blocked_key.search(line):
                violations.append((file_path, index, "raw-evidence-key"))
            if workspace_source_ref.search(line):
                violations.append((file_path, index, "raw-workspace-source-ref"))

if violations:
    for path, line, rule in violations:
        try:
            display = path.relative_to(root)
        except ValueError:
            display = path
        print(f"{display}:{line}: {rule}", file=sys.stderr)
    sys.exit(1)
PY
}

default_targets() {
  local candidates=(
    ".docs/plans/wave-W5-v146/validation-report.md"
    ".docs/plans/wave-W5-v146/proof-bundle.md"
    "src-tauri/target/release-gate/v146-validation.json"
    "src-tauri/target/release-gate/v146-validation.log"
    "src-tauri/target/release-gate/v146-validation-linear-ready.md"
  )

  for candidate in "${candidates[@]}"; do
    if [ -e "$ROOT_DIR/$candidate" ]; then
      printf '%s\n' "$candidate"
    fi
  done
}

self_test() {
  local temp_dir
  temp_dir="$(mktemp -d)"
  trap 'rm -rf "$temp_dir"' RETURN

  cat >"$temp_dir/good.md" <<'EOF'
# Safe Evidence

- entity: `entity_1`
- count: `3`
- trust_band_distribution: `likely_current=1,use_with_caution=2`
- hmac_handle: `7b3f8a12c0d4`
EOF

  cat >"$temp_dir/bad.md" <<'EOF'
# Unsafe Evidence

Local path: /Users/example/Documents/private.md
Email: person@nonexample.test
raw_path: /tmp/private.md
source_ref: workspace_file:abcdef1234567890
EOF

  lint_targets "$temp_dir/good.md"
  if lint_targets "$temp_dir/bad.md" >/dev/null 2>&1; then
    echo "FAIL v146 redaction lint self-test: unsafe artifact passed" >&2
    return 1
  fi

  echo "PASS v146 redaction lint self-test"
}

case "$SELF_TEST" in
  --help|-h)
    usage
    ;;
  --self-test)
    self_test
    ;;
  *)
    lint_targets "$@"
    ;;
esac
