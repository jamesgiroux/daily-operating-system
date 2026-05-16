#!/usr/bin/env bash
# W4-A account-overview block CI invariants (packet §9).
#
# Static-grep gates that protect the block's trust-boundary contract:
#   1. Block attributes MUST NOT carry scope/secret/cache material.
#   2. Browser JS MUST NOT reach loopback runtime URLs or signer logic.
#   3. PHP render path MUST go through the runtime client (no raw HTTP).
#   4. save.js MUST return null and emit no DailyOS HTML.
#   5. No `cached_projection` / `actor_scope_fingerprint` /
#      `actor_context_hint` attributes anywhere in the block dir.
#   6. block.json must be apiVersion 3 + dynamic render.
set -euo pipefail

ROOT_DIR="${W4A_BLOCK_LINT_ROOT:-$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)}"
BLOCK_DIR="$ROOT_DIR/wp/dailyos/blocks/account-overview"

if [[ ! -d "$BLOCK_DIR" ]]; then
  echo "w4-a block lint: missing block dir at $BLOCK_DIR" >&2
  exit 2
fi

failures=0

# 1. Forbidden attribute names (V3 §6.1 / AC §32 / packet §9 #4).
for forbidden in cached_projection actor_scope_fingerprint actor_context_hint hmac_key session_token presence_nonce_token; do
  if grep -nE "\"${forbidden}\"\s*:" "$BLOCK_DIR/block.json" 2>/dev/null; then
    echo "FAIL: forbidden block attribute '${forbidden}' in block.json" >&2
    failures=$((failures + 1))
  fi
done

# Also forbid in render and edit (defense-in-depth).
for forbidden in cached_projection actor_scope_fingerprint actor_context_hint; do
  if grep -rnE "${forbidden}" "$BLOCK_DIR" 2>/dev/null; then
    echo "FAIL: forbidden token '${forbidden}' appears in block source" >&2
    failures=$((failures + 1))
  fi
done

# 2. Browser JS must not reach loopback runtime URLs.
for js in "$BLOCK_DIR/edit.js" "$BLOCK_DIR/save.js"; do
  [[ -f "$js" ]] || continue
  if grep -nE "127\.0\.0\.1|localhost:[0-9]+|/v1/surface/(invoke|project-composition)" "$js" 2>/dev/null; then
    echo "FAIL: browser JS reaches loopback runtime URL: $js" >&2
    failures=$((failures + 1))
  fi
  if grep -inE "create.*hmac|crypto\.subtle|sign\(.*key" "$js" 2>/dev/null; then
    echo "FAIL: browser JS reconstructs signing logic: $js" >&2
    failures=$((failures + 1))
  fi
done

# 3. PHP render path must not make raw HTTP calls or reach into the
#    abilities-runtime crate directly.
for php in "$BLOCK_DIR/render-functions.php" "$BLOCK_DIR/render.php"; do
  [[ -f "$php" ]] || continue
  if grep -nE "wp_remote_post|wp_remote_get|curl_exec|file_get_contents\s*\(\s*['\"]https?" "$php" 2>/dev/null; then
    echo "FAIL: render PHP makes raw HTTP call: $php" >&2
    failures=$((failures + 1))
  fi
  if grep -nE "fallback_projection|project_composition_for_surface_internal" "$php" 2>/dev/null; then
    echo "FAIL: render PHP reaches abilities-runtime directly: $php" >&2
    failures=$((failures + 1))
  fi
done

# 4. save.js must return null and emit no rendered HTML.
SAVE_JS="$BLOCK_DIR/save.js"
if [[ -f "$SAVE_JS" ]]; then
  if ! grep -qE "return\s+null" "$SAVE_JS"; then
    echo "FAIL: save.js must return null" >&2
    failures=$((failures + 1))
  fi
  if grep -nE "data-ds-trust-band|<article|wp-block-dailyos" "$SAVE_JS" 2>/dev/null; then
    echo "FAIL: save.js emits DailyOS HTML — dynamic blocks save null" >&2
    failures=$((failures + 1))
  fi
fi

# 5. block.json sanity.
BLOCK_JSON="$BLOCK_DIR/block.json"
if [[ -f "$BLOCK_JSON" ]]; then
  if ! grep -q "\"apiVersion\": 3" "$BLOCK_JSON"; then
    echo "FAIL: block.json must declare apiVersion 3" >&2
    failures=$((failures + 1))
  fi
  if ! grep -q "\"render\": \"file:./render.php\"" "$BLOCK_JSON"; then
    echo "FAIL: block.json must declare dynamic render via file:./render.php" >&2
    failures=$((failures + 1))
  fi
  if ! grep -q "\"name\": \"dailyos/account-overview\"" "$BLOCK_JSON"; then
    echo "FAIL: block.json must declare the dailyos/account-overview namespace" >&2
    failures=$((failures + 1))
  fi
fi

# 6. Runtime client must expose project_composition_for_surface (packet §6.3.1).
CLIENT_PHP="$ROOT_DIR/wp/dailyos/includes/transport/class-dailyos-runtime-client.php"
if [[ -f "$CLIENT_PHP" ]]; then
  if ! grep -qE "function project_composition_for_surface" "$CLIENT_PHP"; then
    echo "FAIL: DailyOS_Runtime_Client must expose project_composition_for_surface" >&2
    failures=$((failures + 1))
  fi
  if ! grep -qE "'/v1/surface/project-composition'" "$CLIENT_PHP"; then
    echo "FAIL: runtime client method must POST to /v1/surface/project-composition" >&2
    failures=$((failures + 1))
  fi
fi

if [[ "$failures" -gt 0 ]]; then
  echo "w4-a block lint: $failures failure(s)" >&2
  exit 1
fi

echo "w4-a block lint: ok"
exit 0
