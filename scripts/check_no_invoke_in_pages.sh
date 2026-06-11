#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

# Surfaces never handle logic: src/pages/ renders props and dispatches user
# intent only. Direct Tauri invoke() calls belong in hooks (src/hooks/) or
# services (src/services/).
#
# Allowlist entries are temporary exceptions, not architecture.
# AccountDetailPage.tsx expires when the v1.5.0 WR wave lands - remove the
# entry (and extract its invoke() calls) as part of that wave.
ALLOWLIST=(
  "src/pages/AccountDetailPage.tsx"
)

violations=0

while IFS= read -r file_path; do
  rel_path="${file_path#$ROOT_DIR/}"

  allowed=0
  for allowed_path in "${ALLOWLIST[@]}"; do
    if [[ "$rel_path" == "$allowed_path" ]]; then
      allowed=1
      break
    fi
  done
  if [[ "$allowed" -eq 1 ]]; then
    continue
  fi

  while IFS=: read -r line_no line_text; do
    printf 'Surface boundary violation: %s:%s\n' "$rel_path" "$line_no"
    echo "  direct invoke() call detected in a page surface"
    violations=1
  done < <(grep -En 'invoke[<(]' "$file_path" || true)

  # Aliasing an import (e.g. `import { invoke as inv }`) would dodge the
  # call-site grep above, so importing the Tauri invoke module at all is a
  # violation in a page surface.
  while IFS=: read -r line_no line_text; do
    printf 'Surface boundary violation: %s:%s\n' "$rel_path" "$line_no"
    echo "  page surface imports @tauri-apps/api/core (invoke belongs in hooks/services)"
    violations=1
  done < <(grep -n '@tauri-apps/api/core' "$file_path" || true)
done < <(find "$ROOT_DIR/src/pages" -name '*.tsx' ! -name '*.test.tsx' | sort)

if [[ "$violations" -ne 0 ]]; then
  cat <<'EOF'
One or more direct invoke() calls were found in src/pages/ surfaces.
Surfaces never handle logic: move Tauri command invocations into a hook in
src/hooks/ (or a service in src/services/) and dispatch through it.
EOF
  exit 1
fi

echo "No-invoke-in-pages check passed."
