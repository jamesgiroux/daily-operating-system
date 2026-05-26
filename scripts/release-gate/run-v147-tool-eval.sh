#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
REPORT_PATH="${V147_TOOL_EVAL_REPORT:-$ROOT_DIR/src-tauri/target/v147_tool_eval/report.json}"

cd "$ROOT_DIR"
node tests/v147_tool_eval/run.mjs --out "$REPORT_PATH"
