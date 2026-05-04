# I508c — Dimension-Aware Semantic Gap Queries

**Priority:** P1  
**Area:** Backend / Intelligence + Context Provider  
**Version:** v1.0.0 (Phase 2)  
**Depends on:** I508a  
**Parent:** I508 (absorbs I488 mechanism)

## Problem

Current semantic gap detection is a single-string query and cannot target missing evidence by intelligence dimension.

## Scope

1. Evolve `semantic_gap_query` into `semantic_gap_queries` returning structured dimension-aware query items.
2. Preserve existing local ranking behavior while allowing richer query fan-out.
3. Wire gap queries into Glean search expansion when connected.
4. Deduplicate gap results against baseline context fetches.

## Files to Modify

| File | Change |
|---|---|
| `src-tauri/src/intelligence/prompts.rs` | Structured gap-query API and dimension mapping |
| `src-tauri/src/context_provider/glean.rs` | Gap-query fan-out and dedupe in remote mode |

## Acceptance Criteria

1. Missing evidence in any I508 dimension yields at least one targeted gap query item.
2. First enrichment path still emits a broad initial query set.
3. Local mode continues using gap queries for ranking without regression.
4. Remote mode sends gap queries to Glean and dedupes repeated documents.
5. `cargo test` and strict clippy pass.

## Out of Scope

- Intelligence schema type additions (I508a)
- Prompt framing/field guidance updates (I508b)
