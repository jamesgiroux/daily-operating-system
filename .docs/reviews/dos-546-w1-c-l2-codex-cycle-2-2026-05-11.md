# DOS-546 W1-C L2 codex cycle-2 verification — 2026-05-11

**Commit under review:** `6230511d` (W1-C.2: close codex L2 findings)
**Cycle-1 source:** `.docs/reviews/dos-546-w1-c-l2-codex-2026-05-11.md` (REVISE, 3 findings)
**Reviewer:** codex (OpenAI Codex CLI, gpt-5.5, xhigh reasoning)
**Branch:** dos-546-wp-studio-spike

## Verdict: APPROVE

All three cycle-1 findings verified closed against acceptance criteria and source.

### F1 — CLOSED — mcp_client actor projection (P1, AC line 546)

`AbilityActor::project(kind, exposure)` now emits `McpClient` only for `ActorKind::Agent + McpExposure::Invocable` per artifact 05. `System`/`Admin` actors stay `Runtime` regardless of exposure; `Agent + None|MetadataOnly` → `Runtime`. `from_descriptor` threads exposure through to projection.

Source: `src-tauri/abilities-runtime/src/inventory.rs:372` and surrounding projection logic. Two new projection tests cover the matrix.

### F2 — CLOSED — exit code contract (P1)

Emitter doc-comment and branch logic now match the documented contract:
- `1` = registry build failed
- `2` = CLI / I/O / serialization error

Previously reversed. Source: `src-tauri/abilities-runtime/src/bin/emit_ability_inventory.rs:21`.

### F3 — CLOSED — AnnotationValue + CompositionKind schema (P2, AC line 539)

- `AnnotationValue::Number(f64)` implemented (was integer-only).
- `CompositionKind` is now a discriminated enum (`NotComposition | Composition { block_types }`) with bespoke serde that rejects both invalid mixed shapes:
  - `{ produces_blocks: false, block_types: <non-empty> }`
  - `{ produces_blocks: true, block_types: <empty> }`
- TS mirror at `web/types/ability-surface.ts:55` is a matching discriminated union.

### Verification commands

- `cargo test --package abilities-runtime --lib inventory` → **11 passed, 0 failed**
  - (Note: tests live in `abilities-runtime` crate, not `dailyos` lib — `cargo test --package dailyos --lib abilities::inventory` runs 0 tests with exit 0.)
- `pnpm tsc --noEmit` → **clean** (after offline `pnpm install --frozen-lockfile --offline --ignore-scripts` to hydrate `node_modules` in this worktree)

## Outcome

W1-C.2 cycle-2 closes all three cycle-1 findings. The codex reviewer panel verdict on this lane is **APPROVE**. No further cycles required for codex on W1-C.
