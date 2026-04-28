# Implementation Plan: DOS-311 — migration fence + schema epoch + universal write fence

**Status:** SUPERSEDED — see live Linear ticket. This plan was drafted without live-ticket access (Linear MCP was disconnected at write-time). L0 cycle-1 review (3 Codex slots) returned unanimous BLOCK; user (L6) ruled to skip cycle-2 plan rewrite and implement directly against the live ticket using cycle-1 findings as the acceptance checklist (per W0 retro lesson: revise-don't-rewrite + don't-opportunistically-expand-plans).

**Live ticket:** [DOS-311](https://linear.app/a8c/issue/DOS-311) — IS the spec. Read it.
**Cycle-1 review trail:** preserved below as the acceptance checklist for implementation. Codex jobs `task-moitcc8l`, `task-moitcxtq`, `task-moitdfbb`.

**Live-ticket vs this plan, key drift the cycle-1 reviewers caught:**
- `migration_state(key TEXT PRIMARY KEY, value INTEGER NOT NULL)` table with rows for `'schema_epoch'` AND `'global_claim_epoch'` (DOS-310 also writes the `global_claim_epoch` row). My plan invented a separate `schema_epoch` table.
- `intel_queue.schema_epoch INTEGER NOT NULL DEFAULT 1` column on the queue itself (per-job tracking) — my plan didn't have this.
- Worker checkpoint checks at job pickup AND at every natural checkpoint (between dimensions, between Glean calls, before write-back) — my plan only had start_cycle.
- Pre-flight counts in-flight non-queue writers (so an in-flight tracker is required, not just queue depth).
- Drain timeout configurable, default 60s, force-abort path tested.
- Reconcile lives at `scripts/reconcile_ghost_resurrection.sql` — anti-join shape spelled out in live ticket.
- Three fixtures with semantic names: `tombstoned-correctly-hidden`, `tombstoned-with-new-evidence`, `tombstoned-resurrected`. My plan's fixtures (`fixture_a/b/c` clean/concurrent/stale) were wrong shape.
- `--repair` binary uses `services/claims.rs::commit_claim` idempotently (re-tombstone) and verifies post-repair zero findings.
- User-visible "operation interrupted by migration; retry" error (NOT silent drop).
- Migration version: 125 (current head 123, DOS-310 takes 124).
- "Universal fence allowlist for `services/intelligence.rs`" was the exact gap DOS-311 exists to close — all reviewers flagged this. No allowlist; non-queue writers go through the wrapped path.
- TOCTOU reasoning in my plan was contradictory (drain-then-bump vs bump-then-drain). Live: bump epoch FIRST, then wait for drain.

**The implementation below works directly from the live ticket text. The plan content below is preserved as a record of what cycle-1 review caught.**

---

# (original plan content preserved below as reviewer trail)

**Agent slot:** W1-B
**Status:** L0 cycle 1 pending review
**Plan author:** orchestrator (Claude Code)
**Domain reviewer assigned:** architect-reviewer + performance-engineer (substrate primitive + hot-path implications)
**Linear ticket:** [DOS-311](https://linear.app/a8c/issue/DOS-311) (Linear MCP token expired at plan-write time; ticket body sourced from project description in `.docs/plans/v1.4.0-waves.md` and DOS-7 cycle-2 amendment context — cycle-1 reviewer to verify against live ticket and flag any missed amendments)

---

## 1. Contract restated

The wave plan and DOS-7 description state:

> "Migration fence + schema epoch + universal write fence (covers `write_intelligence_json` non-queue callers too) + reconcile SQL with 3 named test fixtures."

> "DOS-7's migration script follows DOS-311's 7-step sequence: pre-flight log, bump epoch, drain workers, run backfill, requeue, reconcile, re-enable workers. No worker drains = no migration. No reconcile pass = no migration complete."

Codex round-1 finding 7 named the cutover-window race; round-2 finding 9 generalized: the round-1 fence covered `intel_queue` only, but `write_intelligence_json` is a **public direct writer** with multiple non-queue callers (DOS-309 PR1 verified 11 production sites + 2 test sites). Without a universal fence, migrating `intelligence.json` to derived state during DOS-7 backfill leaves a window where a non-queue caller writes stale content.

**Mandatory deliverables:**

1. **Schema epoch table** (`schema_epoch`, single row) — bumped by DOS-7's migration during the cutover. Workers and direct-write paths read the epoch and refuse stale writes when the epoch advances mid-flight.
2. **Universal `write_intelligence_json` fence** wrapping the function in `intelligence/io.rs` — every caller (queue + non-queue) goes through the fence. The fence reads the epoch at function entry; if the epoch has advanced since the caller started its enrichment cycle, the write is rejected (or coerced to a warn-log path; cycle-1 reviewers decide which).
3. **Worker drain protocol** — a way to tell `intel_queue` (and any other workers) to finish in-flight work and pause new dequeues during a migration. DOS-7's migration runs the 7-step sequence: pre-flight log → bump epoch → drain workers → backfill → requeue → reconcile → re-enable.
4. **Reconcile SQL** — runs after backfill; verifies invariants (e.g., no orphaned `entity_intelligence` rows for accounts that lack matching `intelligence_claims` after consolidation). 3 named fixtures: `fixture_a_clean_state`, `fixture_b_concurrent_write_during_backfill`, `fixture_c_stale_file_after_backfill`.
5. **Public API**: a single `WriteFence::write_intelligence_json(&self, &dir, &intel) -> Result<...>` that DOS-7 + W3+ callers route through. CI lint enforces no direct calls to the legacy `intelligence::io::write_intelligence_json` outside the fence.

**Cycle-1 amendments referenced:**
- Codex round-1 finding 7 (cutover race with in-flight enrichment)
- Codex round-2 finding 9 (universal fence broader than queue)

## 2. Approach

**End-state alignment:** this is the migration-safety substrate that DOS-7's 9-mechanism backfill (W3) consumes. Without it, cutover from inline-JSON-state to claim-as-source-of-truth is unsafe. Once DOS-7 ships and stabilizes (v1.4.1+), the fence becomes the canonical write path for `intelligence.json` until DOS-301 retires the file entirely.

**What this primitive forecloses:**
- DOS-7's migration script: 7-step sequence is locked. The script may not skip drain or reconcile.
- All future `write_intelligence_json` callers: route through the fence. Direct calls to the legacy function get banned via lint.
- Worker shutdown semantics: `intel_queue` gains a drain mechanism; cycle-1 reviewer confirms whether this is graceful (finish in-flight items) or hard (cancel + requeue).

**Files created/modified:**

### 1. New migration: `src-tauri/src/migrations/124_dos_311_schema_epoch.sql`

```sql
-- DOS-311: schema epoch for migration cutover safety.
--
-- Bumped by DOS-7's migration during the 9-mechanism consolidation
-- (and by any future structural migration that requires worker drain).
-- Workers and direct write paths read the epoch at start-of-cycle;
-- if the epoch has advanced mid-flight, the write is rejected.

CREATE TABLE IF NOT EXISTS schema_epoch (
    id    INTEGER PRIMARY KEY CHECK (id = 1),
    epoch INTEGER NOT NULL,
    bumped_at TEXT NOT NULL DEFAULT (datetime('now')),
    bumped_by TEXT NOT NULL DEFAULT 'system'
);
INSERT OR IGNORE INTO schema_epoch (id, epoch, bumped_at, bumped_by)
    VALUES (1, 0, datetime('now'), 'initial');
```

### 2. New module: `src-tauri/src/intelligence/write_fence.rs`

```rust
//! DOS-311: universal write fence for intelligence.json + future
//! claim-derived projection writes.
//!
//! All writers route through `WriteFence::write_intelligence_json`. The
//! fence captures the schema_epoch at the start of an enrichment cycle
//! (via `WriteFence::start_cycle`) and verifies it has not advanced
//! before committing the write. If the epoch has advanced (because a
//! migration ran mid-flight), the write is rejected and the caller
//! treats it as a soft-skip — the migration's backfill is now the
//! source of truth for that entity.

use std::path::Path;
use crate::db::ActionDb;
use crate::intelligence::io::IntelligenceJson;

/// A single enrichment cycle's fence handle. Captures the schema_epoch
/// at start-of-cycle; the write call later verifies the epoch is unchanged.
pub struct FenceCycle {
    captured_epoch: i64,
}

pub struct WriteFence;

impl WriteFence {
    /// Capture the current schema_epoch. Call at the start of an
    /// enrichment cycle; pass the returned handle to `write_intelligence_json`.
    pub fn start_cycle(db: &ActionDb) -> Result<FenceCycle, String> {
        let epoch: i64 = db.conn_ref()
            .query_row("SELECT epoch FROM schema_epoch WHERE id = 1", [], |r| r.get(0))
            .map_err(|e| format!("schema_epoch read: {e}"))?;
        Ok(FenceCycle { captured_epoch: epoch })
    }

    /// Write intelligence.json IF the schema_epoch has not advanced since
    /// `start_cycle` was called. Otherwise, returns FenceRejected with
    /// the captured + current epoch values (caller logs and skips).
    pub fn write_intelligence_json(
        cycle: &FenceCycle,
        db: &ActionDb,
        dir: &Path,
        intel: &IntelligenceJson,
    ) -> Result<(), FenceError> {
        let current: i64 = db.conn_ref()
            .query_row("SELECT epoch FROM schema_epoch WHERE id = 1", [], |r| r.get(0))
            .map_err(|e| FenceError::DbRead(e.to_string()))?;
        if current != cycle.captured_epoch {
            return Err(FenceError::EpochAdvanced {
                captured: cycle.captured_epoch,
                current,
            });
        }
        crate::intelligence::io::write_intelligence_json(dir, intel)
            .map_err(FenceError::WriteFailed)
    }
}

#[derive(Debug)]
pub enum FenceError {
    DbRead(String),
    EpochAdvanced { captured: i64, current: i64 },
    WriteFailed(String),
}
```

**Open question for cycle-1 reviewers:** the fence captures epoch at `start_cycle` and re-reads at `write_intelligence_json`. There's a TOCTOU window between the re-read and the actual file write. Acceptable because:
- DOS-7's migration drains workers BEFORE bumping the epoch (the worker's `start_cycle` capture happens BEFORE the bump).
- A non-queue caller that captures epoch, then a migration runs concurrently — the re-read catches it.
- Two concurrent enrichment cycles in different workers have NO interaction with the fence — both capture the same epoch; both succeed; both rejected if the epoch bumps mid-write.

### 3. CI lint addition: `scripts/check_write_fence_usage.sh`

```bash
#!/usr/bin/env bash
# DOS-311: enforce universal write fence — no direct calls to the legacy
# write_intelligence_json outside intelligence/write_fence.rs and tests.
#
# After this PR: every production caller routes through
# WriteFence::write_intelligence_json. The legacy function remains in
# intelligence/io.rs as the implementation behind the fence; only
# write_fence.rs may call it directly.

set -euo pipefail
ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

# Match direct calls (matching DOS-309's regex shape: .fn(, ::fn(, fn():
PATTERN='\b(write_intelligence_json)[[:space:]]*\('

violations=0
while IFS= read -r line; do
  case "$line" in
    "$ROOT_DIR/src-tauri/src/intelligence/write_fence.rs"*) continue ;;
    "$ROOT_DIR/src-tauri/src/intelligence/io.rs"*) continue ;;
    "$ROOT_DIR/src-tauri/tests/"*) continue ;;
    "$ROOT_DIR/src-tauri/src/services/intelligence.rs"*) continue ;;  # transitional; remove after W3 fence-migration
    "$ROOT_DIR/.docs/"*) continue ;;
  esac
  echo "$line"
  violations=$((violations + 1))
done < <(grep -rEn "$PATTERN" "$ROOT_DIR/src-tauri/src/" 2>/dev/null || true)

if [ "$violations" -gt 0 ]; then
    echo "ERROR: $violations direct write_intelligence_json call(s) outside fence."
    exit 1
fi
```

**Open question for cycle-1 reviewers:** the lint allowlists `services/intelligence.rs` transitionally (DOS-309 PR1 has 5 post-commit warn-log paths to it). Should DOS-311 PR1 also migrate those 5 sites to the fence, or defer to W3? Recommend: defer to W3 alongside DOS-7 — keeps DOS-311's PR scope focused on the fence + drain, not on the migration of every existing caller.

### 4. Worker drain protocol

**`IntelligenceQueue` gains:**
- `pub fn drain(&self) -> Vec<IntelRequest>` — empties the queue, returns pending items.
- `pub fn pause(&self) -> ()` and `pub fn resume(&self) -> ()` — flag-controlled pause; new `enqueue` calls respect it (drop or buffer; cycle-1 reviewer picks).
- The processor loop (`intel_queue.rs:540+`) honors the pause flag.

**Production migration sequence (DOS-7 will execute):**

```
1. pre-flight log:    INSERT into migration_audit (id, started_at, ...)
2. bump epoch:        UPDATE schema_epoch SET epoch = epoch + 1
3. drain workers:     state.intel_queue.pause(); wait for in-flight items
4. run backfill:      DOS-7's 9-mechanism consolidation
5. requeue drained:   state.intel_queue.enqueue(...) for each drained item
6. reconcile:         run reconcile SQL; assert zero invariant violations
7. resume workers:    state.intel_queue.resume()
```

This issue ships steps 2 + 3 + 7 (the universal mechanisms). DOS-7 (W3) supplies steps 1, 4, 5, 6.

### 5. Reconcile SQL + 3 named fixtures

`src-tauri/src/migrations/124_dos_311_schema_epoch.sql` ships only the schema_epoch table.
**Reconcile SQL** lives in `src-tauri/src/migrations/dos_311_reconcile.sql.template` (loaded as a string by DOS-7's migration). Cycle-1 reviewer confirms placement convention — does the project ship .sql files referenced via include_str! or as runtime-loaded scripts?

**3 named fixtures** in `src-tauri/tests/dos311_fence_fixtures/`:
- `fixture_a_clean_state.sql` — pre-state matches expected post-state; reconcile passes with zero deltas.
- `fixture_b_concurrent_write_during_backfill.sql` — a `write_intelligence_json` caller captures epoch=N, mid-backfill the epoch bumps to N+1, the write attempts after; reconcile detects the rejected write (no stale data lands).
- `fixture_c_stale_file_after_backfill.sql` — `intelligence.json` on disk reflects pre-migration state; reconcile detects the drift and flags it for DOS-301's repair sweep.

### 6. Tests

**Unit tests in `intelligence/write_fence.rs::tests`:**
- `start_cycle_captures_current_epoch`
- `write_succeeds_when_epoch_unchanged`
- `write_rejects_when_epoch_advanced` — bump epoch between start_cycle and write; assert FenceError::EpochAdvanced.
- `pause_and_resume_lifecycle` — queue pause flag respected by enqueue + processor.
- `drain_returns_pending_and_empties_queue`

**Integration tests in `src-tauri/tests/dos311_fence_integration.rs`:**
- `migration_sequence_drain_bump_backfill_resume` — simulates the 7-step sequence end-to-end.
- `concurrent_writer_during_drain_rejected` — a worker writes during the drain phase; assert FenceError::EpochAdvanced.
- `reconcile_zero_findings_on_clean_fixture_a`
- `reconcile_detects_concurrent_write_fixture_b`
- `reconcile_detects_stale_file_fixture_c`

**CI lint test:**
- `scripts/check_write_fence_usage.sh` runs in CI; greps for direct `write_intelligence_json(` calls outside the allowlist. Exits non-zero on regression.

## 3. Key decisions

- **Epoch table separate from `entity_graph_version` and `global_claim_epoch`**: three different invalidation domains. `entity_graph_version` is entity-linking (DOS-258). `global_claim_epoch` is Global-subject claims (DOS-310). `schema_epoch` is structural-migration cutover (DOS-311). Sharing counters would re-introduce the singleton-counter thrash (Codex round-1 finding 4).
- **Captured-epoch fence pattern**: caller calls `start_cycle` once; passes handle to subsequent writes. Fence rejects writes if epoch advanced. Simpler than a long-running advisory lock.
- **DOS-311 PR1 does NOT migrate existing callers** to the fence. Allowlist transitions them in W3 alongside DOS-7's 9-mechanism consolidation. PR1 ships the fence + drain + reconcile + 3 fixtures.
- **Drain protocol on `IntelligenceQueue`** only. Other workers (e.g., transcript processor, signal evaluator) are out of scope for W0/W1 — the wave plan does not list them as fence consumers. Cycle-1 reviewer: confirm.
- **Reconcile SQL placement**: `migrations/dos_311_reconcile.sql.template` (referenced by DOS-7's migration via `include_str!`). Not run automatically by W1's migration runner; only by DOS-7's migration script.
- **`#[must_use]` on `FenceCycle` and the fence write methods**: prevents silent discard. Pattern matches DOS-309 PR1.
- **CI lint allowlist scope**: `intelligence/write_fence.rs`, `intelligence/io.rs`, `src-tauri/tests/`, `services/intelligence.rs` (transitional). Cycle-1 reviewer confirms allowlist or proposes tightening.
- **Worker pause semantics**: when paused, new `enqueue` calls drop the request silently with a warn-log (rather than buffer). Cycle-1 reviewer challenges; alternative is buffering with bounded depth.

## 4. Security

- **No new attack surfaces.** New schema table + new module with no user input + new lint script.
- **Cross-tenant exposure: zero.** Epoch counter is global to the workspace, not per-tenant; same trust boundary.
- **Migration audit trail**: `schema_epoch.bumped_at` + `bumped_by` fields capture who bumped the epoch (v1.4.0: `system`; v1.5+: optional user identifier for manual ops). Cycle-1 reviewer confirms.
- **PII in logs**: epoch values + entity IDs in fence-rejection logs only. No customer text.
- **Worker drain race**: a malicious caller can't trigger the fence-rejection path because epoch bumps require a privileged migration call (`db.bump_schema_epoch(...)` with a guard).
- **Spine restriction interaction**: DOS-310's `Global` variant blocked at compile time; DOS-311's fence operates orthogonally (it's about file-cache cutover, not claim-write).

## 5. Performance

- **Per-write fence overhead**: 1 SELECT on `schema_epoch` (single-row indexed lookup) — ~10-50µs on SQLite. Negligible vs. the file write itself (~1ms+).
- **`start_cycle` overhead**: 1 SELECT + 1 struct alloc. Once per enrichment cycle, not per claim. Trivial.
- **Drain protocol**: pause flag is an atomic bool; processor loop checks it on each dequeue iteration. No per-item overhead.
- **Hot path frequency**: every `write_intelligence_json` caller. Production rate ~10s of writes per minute peak per workspace.
- **Suite P W1 baseline**: this primitive establishes the W1 baseline along with DOS-310. Provisional budget: fence-wrapped write < (legacy write latency + 100µs). Reviewer captures bench results at end of W1.
- **Migration-time cost**: drain is bounded by in-flight enrichment latency (~30s worst case for a Glean-fallback PTY job). Acceptable for a one-time migration.

## 6. Coding standards

- **Services-only mutations**: schema_epoch writes flow through `db.bump_schema_epoch(...)` (new helper); CI lint enforces. Direct UPDATE rejected.
- **Intelligence Loop 5-question check (CLAUDE.md):**
  1. Signals: no new signals from this primitive (the fence is invisible to the signal layer).
  2. Health scoring: unaffected.
  3. `build_intelligence_context()` / `gather_account_context()`: unchanged.
  4. Briefing callouts: no new callout types.
  5. Bayesian source weights: unaffected.
  → No new Intelligence Loop integration. Pure substrate primitive.
- **No `Utc::now()` in services/abilities (W2-A invariant)**: the fence module uses `chrono::Utc::now()` only in the `bumped_at` SQL default (`datetime('now')`); no Rust-level `Utc::now()` calls. Acceptable for W1 (W2-A migrates services).
- **No customer data in fixtures**: fixture SQL uses placeholder IDs.
- **Clippy budget**: zero new warnings under `-D warnings` lib-only.
- **Doc comments**: full doc on `WriteFence`, `FenceCycle`, `FenceError`. Migration sequence documented prominently.
- **CO-AUTHORED-BY**: commit message includes `Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>`.

## 7. Integration with parallel wave-mates

- **DOS-310 (W1-A)** owns: per-entity claim_version columns, `db/claim_invalidation.rs`, sorted-lock helper. **No file overlap** with this PR.
- **Migration numbering**: DOS-310 takes 123 (per its plan §10 Q2 proposal); DOS-311 takes 124. Confirmed at PR-open.
- **Forward-compat with downstream waves:**
  - **DOS-7 (W3-C, post-2026-04-26 absorbing DOS-308)**: executes the 7-step migration sequence using this PR's primitives. DOS-7's migration consumes `bump_schema_epoch` + `intel_queue.pause()/drain()/resume()` + the reconcile fixtures.
  - **DOS-301 (W3-D projection writer)**: when DOS-7 retires legacy AI columns, DOS-301 takes over the projection. The fence's `start_cycle` pattern remains the canonical entry; DOS-301's writes go through it.
  - **DOS-216 (W3 Eval harness)**: replays migration sequences against the 3 named fixtures.
  - **W3 PR-train coordination**: DOS-311's allowlist transitionally permits `services/intelligence.rs` direct calls. After W3 lands, the W3 cleanup PR removes the allowlist entry.

## 8. Failure modes + rollback

- **Migration safety**: `schema_epoch` table is additive. Existing queries are unaffected (no joins against it). Default value 0 means existing fence-cycle reads succeed against a fresh DB.
- **Drain timeout**: if drain doesn't complete within a reasonable window (e.g., 60s), the migration aborts with a hard error. Operators retry. Cycle-1 reviewer: confirm timeout mechanism is in scope vs. deferred to DOS-7.
- **Concurrent writes during drain**: rejected by the fence (epoch advanced). The caller logs FenceError::EpochAdvanced; DOS-301's projection sweep repaints the entity on next claim touch.
- **Worker that ignores pause**: blocked by code review + CI lint (no direct queue access outside the IntelligenceQueue impl). The processor loop is the only consumer of the queue's dequeue method.
- **Schema_epoch write contention**: the bump is a single-row UPDATE; uncontended in normal operation. During migration, only one writer (the migration script).
- **Rollback path**: revert the migration drops the `schema_epoch` table (SQLite ≥3.35) or leaves it as a no-op (older SQLite). Either way, no data corruption — the table is additive.
- **DOS-309 PR1 `services/intelligence.rs` post-commit warn-log paths**: these still call the legacy `write_intelligence_json` directly per the lint allowlist. After W3 lands and the W3 cleanup migrates them to the fence, the allowlist entry is removed. Until then, the W0 ship is unaffected.

## 9. Test evidence to be produced

**Unit tests in `intelligence/write_fence.rs::tests`:**
- `start_cycle_captures_current_epoch`
- `write_succeeds_when_epoch_unchanged`
- `write_rejects_when_epoch_advanced`
- `bump_schema_epoch_increments_and_records_who`
- `intel_queue_pause_drops_new_enqueue`
- `intel_queue_drain_returns_pending`
- `intel_queue_resume_re_enables_processing`

**Integration tests in `src-tauri/tests/dos311_fence_integration.rs`:**
- `migration_sequence_drain_bump_backfill_resume_end_to_end`
- `concurrent_writer_during_drain_rejected`
- `reconcile_zero_findings_on_clean_fixture_a`
- `reconcile_detects_concurrent_write_fixture_b`
- `reconcile_detects_stale_file_fixture_c`

**CI lint:**
- `scripts/check_write_fence_usage.sh` exits 0 on current workspace (lint allowlists existing callers); deliberately add a non-allowlist caller in a feature branch → CI fails.

**Wave merge-gate artifacts:**
- All unit + integration tests pass under `cargo test`.
- `cargo clippy -D warnings` lib-clean.
- New CI lint script wired in `.github/workflows/test.yml` after DOS-309's lint and DOS-310's lint.
- Trybuild fixtures green (if applicable).
- Suite P baseline: criterion bench results on (a) bare write_intelligence_json (legacy), (b) WriteFence::write_intelligence_json (post-this-PR). Captured at end of W1.

**Suite E contribution**: 3 named fixtures (a, b, c) running continuously; concurrent-writer rejection test green (continuous).

## 10. Open questions

For L0 cycle-1 reviewers to confirm or redirect:

1. **`services/intelligence.rs` allowlist**: PR1 keeps direct `write_intelligence_json` calls there (5 sites from DOS-309 PR1). Acceptable as transitional, or should this PR migrate them to the fence? Recommend: defer to W3 (alongside DOS-7's broader cutover).
2. **Drain timeout**: does this PR ship a timeout mechanism, or defer to DOS-7's migration script? Recommend: defer to DOS-7. PR1 ships pause/resume/drain primitives; DOS-7 supplies timeout policy.
3. **Worker pause semantics**: pause-then-drop new enqueues vs. pause-then-buffer. Recommend: drop with warn-log (simpler; the migration is a one-time event; bounded buffer adds complexity).
4. **Reconcile SQL placement**: `migrations/dos_311_reconcile.sql.template` referenced via `include_str!` from DOS-7's migration code? Or runtime-loaded? Recommend: `include_str!` (matches existing project convention; verified at PR-open by reviewer).
5. **Other workers (transcript processor, signal evaluator)**: out of scope for W0/W1 per wave plan? Recommend: yes; the fence's drain protocol applies to `IntelligenceQueue` only. Other workers don't write `intelligence.json` directly.
6. **Spine restriction on the fence**: should the fence reject writes if the captured epoch is outside an acceptable window (e.g., > 1 cycle behind)? Recommend: not in PR1; cycle-1 review may flag if there's a real scenario.
7. **`schema_epoch.bumped_by` field semantics**: defaulted to `'system'` for v1.4.0. Should the type be a fixed enum (e.g., `'system' | 'migration' | 'manual'`) or free-form string? Recommend: free-form for now; tighten in v1.5 if needed.

**Linear MCP token expired** at plan-write time. Cycle-1 reviewers should pull DOS-311 ticket directly to verify against the live ticket text and flag any 2026-04-24+ amendments not captured here.
