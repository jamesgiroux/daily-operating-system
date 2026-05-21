# v1.4.5 W2 L0 — L1 Residuals from Cycle 5 Partial Convergence

**Status:** L0 declared **partial convergence** at cycle 5 (2026-05-21). Five-cycle adversarial review run on V1.0 → V1.1 → V1.2 → V1.3 → V1.4 packets; findings volume curve 7-9 → 5-7 → 4-7 → 3-5 → 1-4 per lane (diminishing). Codex consult panel flagged CLASS-PATTERN-RECURRING (compile-shape divergence from live code) on W2-C and W2-D at cycle 5, triggering partial-convergence per memory `feedback_l0_partial_convergence_when_class_recurs`. W2-D codex challenge gave **first APPROVE** at cycle 5.

**Authority:** Cycle 5 verdict files at `/tmp/w2-l0-reviews/cycle5/{lane}-codex-{challenge,consult}.out`. Cycle-13 wave amendment at `.docs/plans/v1.4.5-waves.md` §"Cycle 13 amendments". §0 W2 shared contract V1.3 at `.docs/plans/v1.4.5-workspace-memory/W2-shared-contract.md`. Per-lane V1.4 packets in same dir.

This document is the **L1 implementer kickoff checklist** — every residual below is either (a) an L1-precondition resolvable against grepped live substrate at impl time, or (b) a Codebase Maintenance ticket (DOS-751 / `b8e6aea4-d47e-4f3a-b03d-a05bec914aeb`) parked for separate work.

L1 implementer MUST resolve all L1-precondition residuals before declaring the corresponding wave-W2 lane DONE. Maintenance residuals do NOT block L1 but should be filed before L1 close.

---

## W2-A — DOS-466 residuals

### L1-preconditions (resolve at L1 kickoff)

1. **`entity_name` semantics in `WorkspaceCategoryRegistry::resolve_path`** (cycle-5 challenge F1). Packet V1.4 §7 says `entity_name` is display-only and "never" a path segment; but live `registry.rs:418-424,448` joins `entity_name` into the resolved path. L1 implementer: read `registry.rs:418-448` literally; if `entity_name` is actually a path segment, REVERT packet §7 wording and reinstate the `^[a-z0-9_-]+$` slug-validation gate at the construction site. If it's NOT a path segment (only used for display in the canonical path), keep packet wording. Document the resolution in the L1 PR description.

2. **`workspace_root` flow through `IngestPipeline`** (cycle-5 consult finding 1). Packet V1.4 §9 defines `IngestPipeline::run(&Connection, IngestRequest)` without `workspace_root`. The bridge impl (`workspace_intake_impl.rs`) has `workspace_root` bound at construction (per cycle-3 architect F6 fold), but §7 requires `pipeline.run` to re-derive `file_id` from `workspace_root`. L1 implementer: either (a) thread `workspace_root` into `IngestPipeline` via constructor + propagate; (b) thread through `IngestRequest` (caller-supplied, validated against bridge value before run); or (c) document that `file_id` is always pre-derived by the caller and the pipeline trusts it. PICK ONE at L1; verify W2-B + W2-D consumers compose.

3. **§0 shared contract still has stale typed-DTO duplicate** (cycle-5 consult finding 3). `W2-shared-contract.md` §4 has the raw-slug `WorkspaceIntakeRequest` (canonical V1.3) AND the typed-DTO block lower in the same section. L1 implementer (or anyone reading at L1 kickoff): delete the typed-DTO duplicate from §0; raw-slug is canonical. This is a 1-paragraph edit.

### Maintenance tickets (file before L1 close; do not block L1)

- **DOS-751 candidate:** AST-based `pipeline.rs no-direct-open` lint (cycle 1 + 4 architect path-α) — current grep-based gate covers the named tokens; AST gate would catch tokens via re-export or alias.
- **DOS-751 candidate:** Strict-typed `IngestError::DbError` (substrate-wide pattern — claims service, runs repo, link repo all use `DbError(String)`).

---

## W2-B — DOS-467 residuals

### L1-preconditions (resolve at L1 kickoff)

1. **Transitive transcript writes via `quill/sync.rs` + `processor/transcript.rs`** (cycle-5 challenge: STILL PRESENT). Packet V1.4 §4 scopes lint to "W2-B-touched files". But `granola/poller.rs:299` → `quill/sync.rs:177/182/185`, and `quill/poller.rs:377` → `quill/sync.rs:*`, then `processor/transcript.rs:237-249` writes transcript files. These transcript-write paths are workspace-file mutations that V1.4 did not allowlist. L1 implementer: enumerate every transitive workspace-file-write reachable from W2-B-owned call sites (`watcher.rs`, `*/poller.rs`); decide per-call-site whether to (a) route through `IngestPipeline::run` (Drive/transcript content needs a staging API — v1.4.6 per cycle-13 §13.3.2), (b) add `dos7-allowed: transcript-direct-write-v146` rationale, or (c) document the gap with a Maintenance ticket. The W2-B mutation lint script MUST cover the transitive set.

2. **`wiring::build_pipeline()?` signature mismatch** (cycle-5 consult finding 2). Packet V1.4 §8 uses `wiring::build_pipeline()?` (fallible). W2-A V1.4 §4 declares `pub fn build_pipeline() -> IngestPipeline` (infallible). L1: either change W2-A's `build_pipeline` to return `Result<IngestPipeline, BuildPipelineError>` (and add tests), OR drop the `?` from all W2-B call sites.

3. **Markdown preservation on pipeline failure** (cycle-5 consult finding 3). V1.4 §8 runs `pipeline.run(...)?` BEFORE `write_account_markdown`, so pipeline rejection skips markdown regeneration. §10 done-when says markdown keeps regenerating. L1: either (a) call `write_account_markdown` BEFORE `pipeline.run` (markdown is always written; pipeline may fail downstream), (b) document that pipeline-failure cases also skip markdown (and update done-when), or (c) wrap in a recovery block that calls `write_account_markdown` in the `Err` arm of `pipeline.run`.

### Maintenance tickets (file before L1 close)

- **DOS-751 candidate:** Drive content staging API (`workspace_ingestion::staging`) for v1.4.6 — already named in cycle-13 §13.3.2.
- **DOS-751 candidate:** AST-aware mutation lint (multi-cycle path-α).

---

## W2-C — DOS-468 residuals

### L1-preconditions (resolve at L1 kickoff)

1. **CRITICAL — Ability return shape: wrap or not wrap** (cycle-5 challenge F1 + multi-cycle recurring). Packet V1.4 §9 returns `Ok(EntityIntakeOutput { ... })` (direct T). But cycle-5 codex reads live `account_overview.rs:117-122` as returning `Ok(AbilityOutput { ... })` (finalized wrapper). The §6 K-in cite at `account_overview.rs:106-115` showed "return T directly" — at line 122 the live code apparently DOES wrap. **L1 implementer: read `account_overview.rs:106-150` literally; mirror the actual return-shape pattern exactly.** Whichever the live code does, that's the canonical shape. Update §9 stub to match. This is the single most-recurring finding across cycles 2-5; settle by reading live code, not packet text.

2. **Write route ownership** (cycle-5 challenge F3 + cycle-3 devex G1). Packet V1.4 §9 names `POST /wp-json/dailyos/v1/entity-intake/ingest` as the editor write route. Live `class-dailyos-plugin.php:609-647` only has nonce + account-overview routes. L1 implementer: either (a) own a small extension to `class-dailyos-plugin.php` to register the new route with `can_edit_posts_rest` permission callback (out-of-scope for W2-C per V1.2; ownership escalation required), OR (b) reuse the existing generic SurfaceClient invoke route (if it exists; grep `dailyos/v1/ability/invoke` or similar), OR (c) defer block to v1.4.6 alongside the confirmation-token transport extension. Coordinator-level decision; do NOT silently extend `class-dailyos-plugin.php` without authorization.

3. **abilities-runtime self-crate imports** (cycle-5 consult finding 2). V1.4 §9 uses `abilities_runtime::abilities::trust::TrustBand` inside abilities-runtime. Should be `crate::abilities::trust::TrustBand` (self-crate). L1 implementer: fix imports at impl time; mechanical.

4. **§9 EntityIntakeClaim DTO missing rendered text field** (cycle-5 challenge F4). V1.4 §9 has `EntityIntakeClaim { claim_id, trust_band, sensitivity }` — no claim text/projection. §11 done-when requires fixture claim proposals render inline. L1 implementer: add a `display_text: String` (or `claim_text: String`) field to `EntityIntakeClaim` populated from the canonical claim-read projection (cite path at L1).

### Maintenance tickets (file before L1 close)

- **DOS-751 candidate:** Confirmation-token transport extension for v1.4.6 (already in cycle-13 §13.3.3).
- **DOS-751 candidate:** `pnpm gen:block` scaffolding generator (cycle-3 devex path-α).
- **DOS-751 candidate:** `dailyos_block_error_panel()` shared primitive lift (cycle-3 devex path-α).

### K-out commitment

L3 retro MUST capture `docs/solutions/conventions/wp-block-write-ability-pattern.md` per cycle-3 devex (covers editor write gesture, dynamic save, read-only render ability, sensitivity gate, MCP-hidden posture, discriminated error states, attribute minimalism).

---

## W2-D — DOS-469 residuals

### L1-preconditions (resolve at L1 kickoff)

1. **`State<'_, AppState>` vs `State<'_, Arc<AppState>>`** (cycle-5 consult finding 1). V1.4 §8 uses `State<'_, AppState>` and `state.db_write()?`. Live command handlers use `State<'_, Arc<AppState>>` with closure-based `AppState::db_write`. L1 implementer: align `assign_inbox_entity` signature to the live `Arc<AppState>` pattern; mechanical.

2. **`WorkspaceFileKind::from_slug` not in live source** (cycle-5 consult finding 1). V1.4 §6 makes it a pre-L1 confirmation. L1 implementer: read `abilities-runtime/src/abilities/provenance/source.rs` and either find an equivalent (`FromStr` impl, `try_from`, or `from_str_lossy`) or own a small extension to W1-A's `source.rs` to add `from_slug`. If extension needed, file as W1-extension addendum to cycle-13 §13.1.

3. **Rollback boundary for `set_entity → add_link → pipeline.run`** (cycle-5 consult finding 2). V1.4 says ordering is "verified" but doesn't name a transaction primitive. `AppState::db_write` serializes; doesn't transact. Live transaction primitive is `ActionDb::with_transaction`. L1 implementer: wrap the three writes in `ActionDb::with_transaction` (or compensate manually with a typed retry rule if transactions don't compose with async pipeline). Test: failure after `set_entity` + before `add_link` must roll back `set_entity`.

### Maintenance tickets (file before L1 close)

- **DOS-751 candidate:** Typed lifecycle-row query API replacing the `InboxLifecycleRow` read projection (cycle 3 + 4 + 5 path-α).
- **DOS-751 candidate:** Normalize `inbox-updated` payload shape (watcher emits `{ count }` vs executor emits `()`).

---

## Cross-lane residuals (§0 amendments deferred to L1 kickoff)

1. **§0 typed-DTO duplicate** — see W2-A residual 3.
2. **`workspace_root` placement** — see W2-A residual 2.
3. **`build_pipeline` signature** — see W2-B residual 2.
4. **Ability return shape canonical pattern** — see W2-C residual 1.

These are §0 V1.4 (post-merge) candidates; not blocking PR open. L1 implementer can resolve via §0 edit at L1 kickoff.

---

## L0 PASS RULE — partial convergence (per memory `feedback_l0_partial_convergence_when_class_recurs`)

L0 panel verdict: **PARTIAL CONVERGENCE at cycle 5.**

- Cycle 5 reviewer panel: 7 BLOCK + 1 APPROVE (W2-D codex challenge). Findings volume per lane: 1-4 (diminishing from cycle-1 7-9). All BLOCK reasons are concrete + grep-verifiable at L1 time, no architectural decisions remaining.
- CLASS-PATTERN-RECURRING flagged on W2-C and W2-D codex consult: compile-shape divergence from live code. Per memory rule, partial-convergence is correct.
- L1 implementer resolves the per-lane L1-preconditions against grepped live substrate before declaring each lane DONE. The 4 cross-lane §0 amendments above (typed-DTO duplicate, workspace_root, build_pipeline signature, ability return shape) are recommended §0 V1.4 edits at L1 kickoff but do not block PR open.
- All declared multi-actor security gates that would normally fire at /cso review are correctly dropped per the threat-topology framing (local-to-local single-user; memory `feedback_local_to_local_security_overreach_primary_concern`; engineering-ladder.md L0 framing per PR #348).

---

## Files committed in this L0 PR

| Path | Status | Notes |
|---|---|---|
| `.docs/plans/v1.4.5-waves.md` | M | Cycle-13 amendment block added |
| `.docs/plans/v1.4.5-workspace-memory/W2-shared-contract.md` | A | §0 V1.3 — raw-slug DTOs + LifecycleRepo additions |
| `.docs/plans/v1.4.5-workspace-memory/L0-packet-W2-A-DOS-466.md` | A | V1.4 — substrate + abilities-runtime bridge ownership |
| `.docs/plans/v1.4.5-workspace-memory/L0-packet-W2-B-DOS-467.md` | A | V1.4 — mutation refactor + helper-writer allowlist table |
| `.docs/plans/v1.4.5-workspace-memory/L0-packet-W2-C-DOS-468.md` | A | V1.4 — entity-intake block + ability (Transform category) |
| `.docs/plans/v1.4.5-workspace-memory/L0-packet-W2-D-DOS-469.md` | A | V1.4 — _inbox refactor + assign_inbox_entity command |
| `.docs/plans/v1.4.5-workspace-memory/L1-residuals-from-L0-cycle-5.md` | A | This file |

All cycle-1/2/3/4/5 reviewer verdict files live under `/tmp/w2-l0-reviews/cycle{1..5}/` in the worktree at `/private/tmp/dailyos-v145-w2-l0`. These are NOT committed (they're scratch); the substantive findings + folds are captured in each packet's §2 changelog + this residuals doc.

---

## Next steps for L1

1. Read this file end-to-end before claiming any W2 lane.
2. Per lane: address all L1-preconditions; file maintenance tickets for the parked items; cite cycle-5 verdict files in the L1 PR description.
3. Open W1-extension PR FIRST (cycle-13 §13.1) — `IngestionMode::EntitySeeded` + `IngestionMode::Realtime` are L1 preconditions for W2-A/C/D.
4. Wait for W1 PR #345 to land L3 (still BLOCKED-ON v1.4.4 v242–v245 per HANDOFF-2026-05-21.md).
5. Implement lanes in wave order: W2-A first (substrate + bridge), then W2-B/C/D in parallel after W2-A merges.
