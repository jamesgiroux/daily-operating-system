#!/usr/bin/env bash
# Purpose: enforce ADR-0130 §1 substrate-owned authorship — only code inside
# the `abilities-runtime` abilities module may construct `Composition`
# directly. Surfaces, command handlers, hooks, and other consumer crates
# receive `Composition` through the normal ability-invocation path and
# render it; they never author it.
#
# Primary enforcement is the Rust `pub(crate)` visibility on
# `Composition::new` in src-tauri/abilities-runtime/src/abilities/composition.rs.
# This script is defense-in-depth: it catches `Composition { ... }` struct
# literal constructors and `Composition::new(` call sites in non-abilities
# paths, which would silently re-enable cross-crate authorship if the
# constructor were ever opened up by accident.
#
# Mechanism: ripgrep for the two construction shapes across the workspace,
# excluding the abilities-runtime crate (the substrate that owns
# authorship), generated/target directories, and test/eval fixtures inside
# the substrate. Fail with the offending file:line list if any match.
#
# Exit codes:
#   0  no non-substrate construction sites found.
#   1  drift detected (or ripgrep missing).
#
# How to run: ./scripts/check_composition_authorship.sh
#
# DailyOS substrate-owned authorship gate (ADR-0130 §1).

set -euo pipefail

ROOT_DIR="$(git rev-parse --show-toplevel)"
cd "$ROOT_DIR"

if ! command -v rg >/dev/null 2>&1; then
  echo "check_composition_authorship: ripgrep (rg) not installed" >&2
  exit 1
fi

# Construction patterns to scan for:
#   - `Composition::new(`      (associated-function constructor)
#   - `Composition {`          (struct-literal constructor)
#
# Exclusions:
#   - The substrate crate `src-tauri/abilities-runtime/**` owns authorship
#     and is allowed to construct compositions.
#   - Generated artifacts, target, node_modules, _archive (historical),
#     and this script itself.
#   - Markdown / ADR documentation files, which legitimately quote the
#     type name when describing the contract.
PATTERN='\bComposition\s*(::new\s*\(|\{)'

# Run two separate scans so we can give a focused error message per shape.
FOUND_NEW=$(rg -n --no-heading --color=never \
  --glob '!src-tauri/abilities-runtime/**' \
  --glob '!src-tauri/target/**' \
  --glob '!target/**' \
  --glob '!node_modules/**' \
  --glob '!.git/**' \
  --glob '!_archive/**' \
  --glob '!.docs/_archive/**' \
  --glob '!.docs/**/*.md' \
  --glob '!**/*.md' \
  --glob '!scripts/check_composition_authorship.sh' \
  -e 'Composition::new\s*\(' \
  . 2>/dev/null || true)

FOUND_LITERAL=$(rg -n --no-heading --color=never \
  --glob '!src-tauri/abilities-runtime/**' \
  --glob '!src-tauri/target/**' \
  --glob '!target/**' \
  --glob '!node_modules/**' \
  --glob '!.git/**' \
  --glob '!_archive/**' \
  --glob '!.docs/_archive/**' \
  --glob '!.docs/**/*.md' \
  --glob '!**/*.md' \
  --glob '!scripts/check_composition_authorship.sh' \
  -e '\bComposition\s*\{' \
  . 2>/dev/null || true)

FAILED=0

if [[ -n "$FOUND_NEW" ]]; then
  echo "check_composition_authorship: non-substrate \`Composition::new(\` call site(s) found." >&2
  echo "ADR-0130 §1 reserves Composition authorship to abilities-runtime substrate." >&2
  echo "$FOUND_NEW" >&2
  FAILED=1
fi

if [[ -n "$FOUND_LITERAL" ]]; then
  echo "check_composition_authorship: non-substrate \`Composition { ... }\` literal(s) found." >&2
  echo "ADR-0130 §1 reserves Composition authorship to abilities-runtime substrate." >&2
  echo "$FOUND_LITERAL" >&2
  FAILED=1
fi

if [[ "$FAILED" -ne 0 ]]; then
  echo "" >&2
  echo "remediation: move composition construction into an ability under" >&2
  echo "  src-tauri/abilities-runtime/src/abilities/, then have the consumer" >&2
  echo "  invoke the ability through the normal AbilityOutput path." >&2
  exit 1
fi

exit 0
