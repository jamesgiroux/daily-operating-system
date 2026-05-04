# I508a — Intelligence Schema Composition (Types Only)

**Priority:** P0  
**Area:** Backend / Intelligence + Frontend  
**Version:** v1.0.0 (Phase 2)  
**Depends on:** I503  
**Parent:** I508

## Problem

I508 scope is too broad for clean parallel execution. Type and schema composition must land first as a stable contract before prompt and gap-query behavior changes.

## Scope

1. Add/normalize I508 intelligence sub-struct types in Rust (`io.rs`).
2. Add matching TypeScript types in `src/types/index.ts`.
3. Extend `IntelligenceJson` with additive, backward-compatible fields using `#[serde(default)]`.
4. Keep behavior unchanged: no prompt, ranking, or query semantics changes in this issue.

## Files to Modify

| File | Change |
|---|---|
| `src-tauri/src/intelligence/io.rs` | Add type definitions and `IntelligenceJson` fields |
| `src/types/index.ts` | Add matching TS type definitions |

## Acceptance Criteria

1. All I508 schema types compile in Rust and TypeScript with matching field names/casing.
2. Existing serialized intelligence payloads deserialize without error.
3. No enrichment prompt behavior changes are included.
4. No semantic-gap query behavior changes are included.
5. `cargo test`, strict clippy, and `pnpm tsc --noEmit` pass.

## Out of Scope

- Prompt/schema instruction updates (I508b)
- Gap query evolution and dimension-aware search terms (I508c)
