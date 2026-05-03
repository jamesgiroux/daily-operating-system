#!/bin/bash
# Appends a line to .docs/design/_pending-inventory-updates.log when a
# surface page (route or full-screen component) is created or modified.
#
# DS-INV-01 (the surface inventory) consumes this log when reconciling
# the canonical inventory at .docs/design/INVENTORY.md. Until DS-INV-01
# lands, this log accumulates pending changes for batch review.
#
# Triggered by arch-doc-updater.sh on src/pages/*.tsx,
# src/components/onboarding/*.tsx, src/components/startup/*.tsx changes.
#
# Idempotent: appends, doesn't fail if the same file appears multiple times.
# (DS-INV-01 dedupes on consume.)

set -euo pipefail

ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
LOG="$ROOT/.docs/design/_pending-inventory-updates.log"
FILE_PATH="${1:-unknown}"

mkdir -p "$(dirname "$LOG")"

# Make path repo-relative for readability
REL_PATH="${FILE_PATH#$ROOT/}"

printf "%s\t%s\n" "$(date -u +%Y-%m-%dT%H:%M:%SZ)" "$REL_PATH" >> "$LOG"
