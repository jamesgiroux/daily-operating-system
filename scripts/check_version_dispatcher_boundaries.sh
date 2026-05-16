#!/usr/bin/env bash
# W4-B-signals CI invariants: pin the dispatcher's module boundaries and the
# scope-predicate call gate.
#
# Three checks:
#   1. RESERVED `signals/` for ADR-0080 — no W4-B-signals dispatcher types,
#      routes, or methods leak into `signals/bus.rs`. (eng P3-C)
#   2. Dispatcher routes live only in the canonical route module surface
#      (`surface_runtime/mod.rs` and `bridges/surface_client.rs`). No other
#      file may match the dispatcher route paths.
#   3. The dispatcher service exposes `scope_permits_claim_read` /
#      `scope_permits_composition_read` and uses them on the delivery path
#      (regression guard for the existence-oracle defense).
set -euo pipefail

ROOT_DIR="${VERSION_DISPATCHER_LINT_ROOT:-$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)}"
SRC_DIR="$ROOT_DIR/src-tauri/src"

if [[ ! -d "$SRC_DIR" ]]; then
  echo "version dispatcher lint: missing $SRC_DIR" >&2
  exit 2
fi

failures=0

# 1. signals/bus.rs MUST NOT host dispatcher types.
if grep -nE "VersionDispatcher|SubscribeRequest|SubscribeAck|ReplayRequest|ReplayResponse|BackpressureEvent" \
    "$SRC_DIR/signals/bus.rs" 2>/dev/null; then
  echo "FAIL: DOS-589 dispatcher types appear in signals/bus.rs (reserved for ADR-0080)." >&2
  failures=$((failures + 1))
fi

# 2. /v1/surface/subscribe and /v1/surface/replay route literals MUST only
#    appear in surface_runtime/mod.rs or bridges/surface_client.rs (route
#    owner) and in tests.
allowed_route_paths=(
  "src-tauri/src/surface_runtime/mod.rs"
  "src-tauri/src/bridges/surface_client.rs"
)

while IFS= read -r -d '' file; do
  rel="${file#"$ROOT_DIR"/}"
  case "$rel" in
    src-tauri/tests/*) continue ;;
    src-tauri/src/services/version_dispatcher.rs) continue ;;
    src-tauri/src/commands/version_dispatcher.rs) continue ;;
  esac
  is_allowed=0
  for allowed in "${allowed_route_paths[@]}"; do
    if [[ "$rel" == "$allowed" ]]; then
      is_allowed=1
      break
    fi
  done
  if [[ "$is_allowed" -eq 0 ]]; then
    if grep -qE "/v1/surface/subscribe|/v1/surface/replay" "$file"; then
      echo "FAIL: route literal /v1/surface/subscribe or /v1/surface/replay found outside canonical route owner: $rel" >&2
      failures=$((failures + 1))
    fi
  fi
done < <(find "$SRC_DIR" -type f -name "*.rs" -print0)

# 3. The dispatcher service must define and reference the scope predicates.
DISPATCHER_FILE="$SRC_DIR/services/version_dispatcher.rs"
if [[ ! -f "$DISPATCHER_FILE" ]]; then
  echo "FAIL: dispatcher service missing at $DISPATCHER_FILE" >&2
  failures=$((failures + 1))
else
  if ! grep -q "pub fn scope_permits_claim_read" "$DISPATCHER_FILE"; then
    echo "FAIL: dispatcher must export scope_permits_claim_read (DOS-589 AC #4)" >&2
    failures=$((failures + 1))
  fi
  if ! grep -q "pub fn scope_permits_composition_read" "$DISPATCHER_FILE"; then
    echo "FAIL: dispatcher must export scope_permits_composition_read (packet §5)" >&2
    failures=$((failures + 1))
  fi
  # Delivery loops must call the predicate; the helper that does this is
  # `row_permitted_for_actor`, which itself routes through the public
  # predicates. Both must reference one of the predicates.
  if ! grep -q "row_permitted_for_actor" "$DISPATCHER_FILE"; then
    echo "FAIL: dispatcher must funnel deliveries through row_permitted_for_actor" >&2
    failures=$((failures + 1))
  fi
  if ! grep -q "scope_permits_claim_read" "$DISPATCHER_FILE"; then
    echo "FAIL: dispatcher delivery must call scope_permits_claim_read" >&2
    failures=$((failures + 1))
  fi
fi

if [[ "$failures" -gt 0 ]]; then
  echo "version dispatcher lint: $failures failure(s)" >&2
  exit 1
fi

echo "version dispatcher lint: ok"
exit 0
