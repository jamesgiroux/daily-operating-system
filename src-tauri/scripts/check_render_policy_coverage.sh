#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$ROOT"

if ! rg -n "render_policy_for_surface" src-tauri/src/services/sensitivity.rs >/dev/null; then
  echo "DOS-412 render policy helper missing from src-tauri/src/services/sensitivity.rs" >&2
  exit 1
fi

violations="$(
  rg -n --pcre2 \
    "(claim\\.text|SELECT\\s+[^;]*\\btext\\b[^;]*\\bFROM\\s+intelligence_claims|source\\.text|source_text)" \
    src-tauri/src/commands src-tauri/src/mcp \
    -g '*.rs' \
    | rg -v "dos412-render-policy-covered|renderable_claim_text|RenderableClaimText|reveal_claim_text_for_tauri" \
    || true
)"

if [[ -n "$violations" ]]; then
  cat >&2 <<'MSG'
DOS-412 render policy coverage failed.

Claim-derived text leaving commands/* or mcp/ must be wrapped through
services::sensitivity::{renderable_claim_text, renderable_claim_text_with_value,
reveal_claim_text_for_tauri} or be explicitly marked with a
dos412-render-policy-covered justification.

Violations:
MSG
  echo "$violations" >&2
  exit 1
fi

echo "DOS-412 render policy coverage OK"
