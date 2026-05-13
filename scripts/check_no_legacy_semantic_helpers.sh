#!/usr/bin/env bash
set -euo pipefail

ROOTS=(
  src
  src-tauri/src
  src-tauri/tests
  src-tauri/examples
  src-tauri/abilities-runtime/src
  src-tauri/abilities-runtime/tests
  src-tauri/abilities-macro/src
  src-tauri/abilities-macro/tests
  scripts
  src-tauri/scripts
  .github
  index.html
  src-tauri/Cargo.toml
)

LEGACY_HELPER='\b(canonicalize_semantic_text|lookup_semantic_term|is_semantic_negator|is_semantic_stopword|semantic_signature_for_text|semantic_stem|combine_semantic_status|semantic_status_compatible|semantic_high_salience_qualifiers|is_semantic_named_entity|is_semantic_low_salience_token|metadata_with_semantic_qualifiers|semantic_qualifiers_from_metadata|is_semantic_metadata_qualifier|semantic_claim_qualifiers|semantic_signatures_near_duplicate|semantic_near_duplicate|semantic_near_duplicate_with_qualifiers|semantic_claim_near_duplicate)\b'

if rg -n "${LEGACY_HELPER}" "${ROOTS[@]}" \
  --glob '!target/**' \
  --glob '!node_modules/**' \
  --glob '!dist/**' \
  --glob '!.git/**' \
  --glob '!.docs/decisions/0131-*.md' \
  --glob '!.docs/plans/v1.4.1-*.md' \
  --glob '!scripts/check_no_legacy_semantic_helpers.sh'; then
  cat <<'MSG'

Legacy semantic helper name found in source.
ADR-0131 §10 and the Phase C cutover require canonical_match_v2 as the replacement mechanism.
MSG
  exit 1
fi
