#!/usr/bin/env bash
set -euo pipefail

cd "$(dirname "${BASH_SOURCE[0]}")/.."
exec src-tauri/scripts/check_trust_factor_thresholds.sh "$@"
