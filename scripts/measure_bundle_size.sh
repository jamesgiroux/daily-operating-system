#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "$0")/.." && pwd)"
DIST_DIR="$ROOT_DIR/dist"
REPORT_PATH="$ROOT_DIR/docs/bundle-size-report.txt"

pnpm --dir "$ROOT_DIR" build

if [[ ! -d "$DIST_DIR" ]]; then
  echo "dist-not-found: $DIST_DIR" >&2
  exit 1
fi

{
  echo "Bundle Size Report"
  echo "generated_at=$(date -u +"%Y-%m-%dT%H:%M:%SZ")"
  echo
  echo "Top dist files by bytes:"
  find "$DIST_DIR" -type f -print0 | xargs -0 stat -f "%z %N" | sort -nr | head -n 25
  echo
  echo "Total dist bytes:"
  find "$DIST_DIR" -type f -print0 | xargs -0 stat -f "%z" | awk '{sum+=$1} END {print sum}'
} | tee "$REPORT_PATH"
