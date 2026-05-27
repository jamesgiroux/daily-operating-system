#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="${V146_VALIDATION_ROOT:-$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)}"
OUTPUT_DIR="${V146_VALIDATION_OUTPUT_DIR:-$ROOT_DIR/src-tauri/target/release-gate}"
REPORT_PATH="${V146_VALIDATION_REPORT:-$OUTPUT_DIR/v146-validation.json}"
GIT_SHA="${V146_VALIDATION_GIT_SHA:-$(cd "$ROOT_DIR" && git rev-parse HEAD 2>/dev/null || printf unknown)}"

mkdir -p "$OUTPUT_DIR"

(
  cd "$ROOT_DIR"
  V146_VALIDATION_OUTPUT_DIR="$OUTPUT_DIR" \
    V146_VALIDATION_REPORT="$REPORT_PATH" \
    V146_VALIDATION_GIT_SHA="$GIT_SHA" \
    bash tests/v146_validation/run.sh all
)

