# I508b — Intelligence Enrichment Prompt/Schema Update

**Priority:** P0  
**Area:** Backend / Intelligence  
**Version:** v1.0.0 (Phase 2)  
**Depends on:** I508a  
**Parent:** I508

## Problem

Even with expanded schema types, enrichment remains constrained if prompt framing and JSON schema instructions stay meeting-centric.

## Scope

1. Update enrichment prompt framing to source-agnostic intelligence extraction.
2. Expand prompt JSON schema guidance to cover new I508 fields.
3. Preserve dual-mode behavior:
- Local mode fills what evidence supports and omits unsupported fields.
- Remote mode can fill richer fields when Glean context exists.
4. Keep gap-query evolution out of this issue.

## Files to Modify

| File | Change |
|---|---|
| `src-tauri/src/intelligence/prompts.rs` | Prompt framing and JSON schema guidance updates |

## Acceptance Criteria

1. Prompt guidance includes all new I508 dimensions/fields with evidence requirements.
2. Local enrichment still produces core fields and does not fabricate unsupported fields.
3. Remote enrichment fills at least two new fields when Glean evidence exists.
4. Existing enrichment pipeline/call-site count remains unchanged.
5. `cargo test` and strict clippy pass.

## Out of Scope

- Type/schema composition (I508a)
- Semantic gap query API/behavior changes (I508c)
