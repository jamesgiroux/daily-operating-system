#!/usr/bin/env bash
# Purpose: fixture-test the runtime-client filter lint gate.
# Exit codes: 0 when all fixture assertions pass; 1 when any assertion fails.
# How to run: ./scripts/check_runtime_client_filter_registered_test.sh

set -euo pipefail

ROOT_DIR="$(git rev-parse --show-toplevel)"
LINT_SCRIPT="${ROOT_DIR}/scripts/check_runtime_client_filter_registered.sh"
TMP_DIR="$(mktemp -d "${TMPDIR:-/tmp}/runtime-client-filter-lint.XXXXXX")"

cleanup() {
  rm -rf "$TMP_DIR"
}
trap cleanup EXIT

fail() {
  echo "runtime-client filter lint fixture test failed: $1" >&2
  if [ "${2:-}" != "" ] && [ -f "$2" ]; then
    sed 's/^/  /' "$2" >&2
  fi
  exit 1
}

make_fixture_root() {
  local root="$1"

  mkdir -p \
    "$root/wp/dailyos/includes" \
    "$root/wp/dailyos/blocks/example"

  cat > "$root/wp/dailyos/blocks/example/render-functions.php" <<'PHP'
<?php

$runtime_client = apply_filters( 'dailyos_runtime_client_for_block', null );
$runtime_client->fetch_account_overview();
PHP
}

VALID_ROOT="$TMP_DIR/valid"
make_fixture_root "$VALID_ROOT"
cat > "$VALID_ROOT/wp/dailyos/includes/class-dailyos-plugin.php" <<'PHP'
<?php

class DailyOS_Plugin {
	public function init(): void {
		add_filter( 'dailyos_runtime_client_for_block', [ $this, 'default_runtime_client_for_block' ], 5, 1 );
	}

	public function default_runtime_client_for_block( mixed $existing ): mixed {
		return $existing;
	}
}
PHP

VALID_OUT="$TMP_DIR/valid.out"
set +e
DAILYOS_LINT_ROOT="$VALID_ROOT" "$LINT_SCRIPT" > "$VALID_OUT" 2>&1
VALID_STATUS=$?
set -e

if [ "$VALID_STATUS" -ne 0 ]; then
  fail "expected init-scoped registration fixture to exit 0, got ${VALID_STATUS}" "$VALID_OUT"
fi

INVALID_ROOT="$TMP_DIR/invalid"
make_fixture_root "$INVALID_ROOT"
cat > "$INVALID_ROOT/wp/dailyos/includes/class-dailyos-plugin.php" <<'PHP'
<?php

class Other_Plugin {
	public function init(): void {
		add_filter( 'dailyos_runtime_client_for_block', [ $this, 'default_runtime_client_for_block' ], 5, 1 );
	}
}

class DailyOS_Plugin {
	public function init(): void {
		$this->register_blocks();
	}

	public function register_rest_routes(): void {
		add_filter( 'dailyos_runtime_client_for_block', [ $this, 'default_runtime_client_for_block' ], 5, 1 );
	}

	public function default_runtime_client_for_block( mixed $existing ): mixed {
		return $existing;
	}
}
PHP

INVALID_OUT="$TMP_DIR/invalid.out"
set +e
DAILYOS_LINT_ROOT="$INVALID_ROOT" "$LINT_SCRIPT" > "$INVALID_OUT" 2>&1
INVALID_STATUS=$?
set -e

if [ "$INVALID_STATUS" -ne 1 ]; then
  fail "expected out-of-init registration fixture to exit 1, got ${INVALID_STATUS}" "$INVALID_OUT"
fi

if ! grep -q "inside DailyOS_Plugin::init()" "$INVALID_OUT"; then
  fail "expected out-of-init failure to name DailyOS_Plugin::init()" "$INVALID_OUT"
fi

echo "runtime-client filter lint fixture test: ok"
