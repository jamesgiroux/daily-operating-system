#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="${V147_E2E_ROOT:-$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)}"
AXIS="${1:-all}"
REPORT_PATH="${V147_E2E_REPORT:-$ROOT_DIR/src-tauri/target/release-gate/v147-e2e.json}"

cd "$ROOT_DIR"
node tests/v147_e2e/run.mjs --axis "$AXIS" --out "$REPORT_PATH"
