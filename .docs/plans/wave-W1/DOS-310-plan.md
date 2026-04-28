# Implementation Plan: DOS-310 — per-entity claim invalidation primitive

**Status:** SUPERSEDED — see live Linear ticket. This plan was drafted without live-ticket access (Linear MCP was disconnected at write-time). L0 cycle-1 review (3 Codex slots) caught the drift across multiple BLOCKER findings; user (L6) ruled to skip cycle-2 plan rewrite and implement directly against the live ticket using cycle-1 findings as the acceptance checklist (per W0 retro lesson: revise-don't-rewrite + don't-opportunistically-expand-plans).

**Live ticket:** [DOS-310](https://linear.app/a8c/issue/DOS-310) — IS the spec. Read it.
**Cycle-1 review trail:** preserved below as the acceptance checklist for implementation. Codex jobs `task-moitadh0`, `task-moitb15d`, `task-moitbj1u`.

**Live-ticket vs this plan, key drift the cycle-1 reviewers caught:**
- `meetings_history` not `meetings` (table name)
- `global_claim_epoch` is a ROW in existing `migration_state` table, NOT a new table
- Multi sort order: `entity_type` lexicographic with precedence `Account < Meeting < Person < Project` (NOT enum-discriminant-derived)
- `SubjectRef` enum shape (variants: `Account { id }`, `Project { id }`, `Person { id }`, `Meeting { id }`, `Multi(Vec<SubjectRef>)`, `Global`) — not `SubjectKind` + `Subject` struct
- Spine restriction enforced at CLAIM_TYPE_REGISTRY layer (no claim_type registers `canonical_subject_types: &[SubjectType::Global]`), NOT at `SubjectRef::Global` construction. `Global` IS structurally available.
- Trigger on `intelligence_claims` insert/supersede/tombstone WHERE `subject_ref = 'Global'` increments `global_claim_epoch`. This was wrong in my plan (said "ship dormant; no production writer").
- Migration version: 124 (current head 123 occupied by DOS-321; plan said 123, was wrong).
- Acceptance includes burst test + `entity_graph_version` not-bumped test + 100-concurrent Multi test, all of which my plan omitted.

**The implementation below works directly from the live ticket text. The plan content below is preserved as a record of what cycle-1 review caught.**

---

# (original plan content preserved below as reviewer trail)

**Agent slot:** W1-A
**Status:** L0 cycle 1 pending review
**Plan author:** orchestrator (Claude Code)
**Domain reviewer assigned:** architect-reviewer (substrate primitive correctness)
**Linear ticket:** [DOS-310](https://linear.app/a8c/issue/DOS-310) (Linear MCP token expired at plan-write time; ticket body sourced from project description in `.docs/plans/v1.4.0-waves.md` and DOS-7 cycle-2 amendment context — cycle-1 reviewer to verify against live ticket and flag any missed amendments)

---

## 1. Contract restated

The wave plan (`.docs/plans/v1.4.0-waves.md`) and DOS-7 description state:

> "Per-entity claim invalidation (Option A picked: per-entity claim_version column, sync transactional). Multi sorted lock ordering. Global gets `global_claim_epoch`."

> "The earlier `DOS-7` acceptance criterion 'intelligence_claims added to entity_graph_version trigger set' is REMOVED. `entity_graph_version` is a singleton counter; bumping it on every claim write thrashes unrelated entity-linking evaluations. Replaced by DOS-310 which ships a per-entity claim invalidation primitive."

Codex round-1 + round-2 review captured the design pressure (singleton counter contention; Multi-subject deadlock potential; undefined Global-subject invalidation).

**Why now:** DOS-7 (W3) leans on this primitive to invalidate per-entity caches when claim writes land. Without it, DOS-7's `commit_claim` either (a) re-uses `entity_graph_version` and thrashes the entity-linking evaluation cache (Codex round-1 finding 4) or (b) ships without invalidation and stale caches surface. This issue is pure-substrate — no consumers in W0/W1.

**Mandatory deliverables:**

1. **Per-entity `claim_version INTEGER NOT NULL DEFAULT 0` column** on every claim-owning entity table: `accounts`, `projects`, `people`, `meetings`. Bumped synchronously inside the same transaction as the claim write.
2. **`global_claim_epoch`** singleton counter (separate from `entity_graph_version`) for any future Global-subject claims. Spine restriction: **no claim_type with Global subject ships in v1.4.0** — enforced by trybuild compile-time check; `global_claim_epoch` exists but no production write path bumps it during the spine.
3. **Multi-subject sorted-lock helper** to prevent deadlocks when a single claim affects multiple entities (e.g., a claim about both an account and a person). Acquires per-entity claim_version updates in a deterministic sorted order.
4. **Public API** in `services/claims.rs` (created here as a stub; DOS-7 fills it in W3) or in a new `db/claim_invalidation.rs` — the API entry that DOS-7 will call.
5. **Tests:** Multi sorted-lock test (concurrent claim writes affecting overlapping entity sets do not deadlock); per-entity claim_version bump verified for each entity type; Global subject restriction verified at compile time via trybuild fixture.

**Cycle-1 amendments referenced:**
- Codex round-1 finding 4 (singleton-counter cache thrash)
- Codex round-2 finding 9 (universal write fence broader than queue) — that's DOS-311 territory but the boundary is relevant.

## 2. Approach

**End-state alignment:** this primitive is the cache-invalidation layer DOS-7's `commit_claim` emits per-claim-write. It's deliberately separate from `entity_graph_version` (which serves entity-linking; DOS-258 territory) because the two caches have different invalidation domains. Mixing them would re-introduce the singleton-counter thrash.

**What this primitive forecloses:**
- DOS-7's invalidation API shape: it MUST call into `db/claim_invalidation.rs::bump_for_subject(...)` (or equivalent) and accept the per-entity `claim_version` values as cache keys.
- Future Global-subject claim_types (v1.5+): they route through `global_claim_epoch`, not `entity_graph_version`. This issue ships the column but no writer; v1.5 lifts the restriction.
- Multi-subject claims: ALWAYS sorted-lock-ordered. Plan author + cycle-1 reviewers must agree on the canonical sort key; recommendation is `(subject_kind: enum_discriminant, entity_id: TEXT)` lexicographic.

**Files created/modified:**

### 1. New migration: `src-tauri/src/migrations/123_dos_310_per_entity_claim_invalidation.sql`

```sql
-- DOS-310: per-entity claim invalidation primitive.
--
-- Replaces the entity_graph_version singleton-counter approach for
-- claim-cache invalidation. Each claim-owning entity table gains a
-- `claim_version` column that DOS-7's commit_claim bumps synchronously
-- inside the claim-write transaction.
--
-- entity_graph_version remains for entity-linking (DOS-258); the two
-- caches have different invalidation domains and must not share counters.
--
-- Spine restriction (v1.4.0): no claim_type may target a Global subject.
-- Enforced via trybuild compile-time check in db/claim_invalidation.rs.
-- The global_claim_epoch column exists for forward-compat; no production
-- writer bumps it during the spine.

ALTER TABLE accounts ADD COLUMN claim_version INTEGER NOT NULL DEFAULT 0;
ALTER TABLE projects ADD COLUMN claim_version INTEGER NOT NULL DEFAULT 0;
ALTER TABLE people   ADD COLUMN claim_version INTEGER NOT NULL DEFAULT 0;
ALTER TABLE meetings ADD COLUMN claim_version INTEGER NOT NULL DEFAULT 0;

CREATE TABLE IF NOT EXISTS global_claim_epoch (
    id    INTEGER PRIMARY KEY CHECK (id = 1),
    epoch INTEGER NOT NULL
);
INSERT OR IGNORE INTO global_claim_epoch (id, epoch) VALUES (1, 0);
```

**Decision:** ALTER TABLE add NOT NULL DEFAULT 0. SQLite supports this in O(1) (it doesn't rewrite the table; the default is stored in the column metadata). No backfill needed.

**Open question for cycle-1 reviewers:** does any existing `accounts` / `projects` / `people` / `meetings` schema constraint conflict with adding a column? Reviewer audits the table definitions in `migrations/*.sql` and confirms.

### 2. New module: `src-tauri/src/db/claim_invalidation.rs`

```rust
//! DOS-310: per-entity claim invalidation primitive.
//!
//! Sole writer for `accounts.claim_version`, `projects.claim_version`,
//! `people.claim_version`, `meetings.claim_version`, and
//! `global_claim_epoch.epoch`. CI lint rejects direct UPDATE of these
//! columns from anywhere else.
//!
//! Multi-subject writes use sorted-lock ordering to prevent deadlocks.

use crate::db::ActionDb;
use crate::db::DbError;

/// The kind of subject a claim targets. Spine restriction (v1.4.0):
/// `Global` is reserved; no production write path may pass it.
/// Enforced at compile time via trybuild fixture.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum SubjectKind {
    /// Discriminant 0 — sorted first when subjects collide on entity_id.
    Account,
    Project,
    Person,
    Meeting,
    /// Reserved for v1.5+. Sorts last; routes to `global_claim_epoch`.
    /// Production callers must NOT construct this variant; trybuild check
    /// blocks any non-test path that does.
    #[doc(hidden)]
    Global,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Subject {
    pub kind: SubjectKind,
    pub entity_id: String,
}

impl ActionDb {
    /// Bump the claim_version for a single entity.
    /// Synchronous; intended to be called inside `with_transaction`.
    #[must_use = "claim invalidation results must be propagated"]
    pub fn bump_claim_version(&self, subject: &Subject) -> Result<(), DbError> {
        match subject.kind {
            SubjectKind::Account => self.conn_ref().execute(
                "UPDATE accounts SET claim_version = claim_version + 1 WHERE id = ?1",
                rusqlite::params![&subject.entity_id],
            )?,
            SubjectKind::Project => self.conn_ref().execute(
                "UPDATE projects SET claim_version = claim_version + 1 WHERE id = ?1",
                rusqlite::params![&subject.entity_id],
            )?,
            SubjectKind::Person => self.conn_ref().execute(
                "UPDATE people SET claim_version = claim_version + 1 WHERE id = ?1",
                rusqlite::params![&subject.entity_id],
            )?,
            SubjectKind::Meeting => self.conn_ref().execute(
                "UPDATE meetings SET claim_version = claim_version + 1 WHERE id = ?1",
                rusqlite::params![&subject.entity_id],
            )?,
            SubjectKind::Global => self.conn_ref().execute(
                "UPDATE global_claim_epoch SET epoch = epoch + 1 WHERE id = 1",
                [],
            )?,
        };
        Ok(())
    }

    /// Bump claim_version for multiple subjects atomically. Subjects are
    /// SORTED before bumping to provide deterministic lock ordering across
    /// concurrent multi-subject writes — prevents deadlocks when two
    /// transactions touch overlapping subject sets.
    ///
    /// Sort key: `(SubjectKind discriminant, entity_id)` lexicographic.
    ///
    /// Caller must hold an active transaction (`with_transaction` closure).
    #[must_use = "claim invalidation results must be propagated"]
    pub fn bump_claim_versions_multi(
        &self,
        subjects: &[Subject],
    ) -> Result<(), DbError> {
        let mut sorted: Vec<&Subject> = subjects.iter().collect();
        sorted.sort();
        sorted.dedup();
        for subject in sorted {
            self.bump_claim_version(subject)?;
        }
        Ok(())
    }
}
```

### 3. CI lint addition

Extend `scripts/check_no_let_underscore_feedback.sh` (created by DOS-309 W0)
with a new function-name denylist for direct claim_version mutations? **No** —
that's a different lint shape. Instead:

**New lint:** `scripts/check_claim_version_writers.sh` greps for direct
`UPDATE accounts SET claim_version`, etc. outside `db/claim_invalidation.rs`.
Wires into `.github/workflows/test.yml` after the DOS-309 lint.

Alternatively (recommended by W0 retro's "audit code, not abstract patterns"
lesson): use a Rust-level unit test that introspects the codebase's compiled
SQL strings via grep at test time. Pick one in cycle-1 review.

### 4. Trybuild fixture: spine restriction on `SubjectKind::Global`

`src-tauri/tests/trybuild_dos310/global_subject_forbidden.rs` (new):

```rust
// This file is a trybuild fixture. It MUST NOT compile.
// If it does, the spine restriction has regressed.

fn main() {
    let s = dailyos_lib::db::claim_invalidation::Subject {
        kind: dailyos_lib::db::claim_invalidation::SubjectKind::Global,
        entity_id: "should-not-compile".to_string(),
    };
    let _ = s;
}
```

Combined with `#[doc(hidden)]` on `Global` and a `#[deprecated(...)] = "spine
restriction"` to make external construction impossible. Cycle-1 review:
verify trybuild infrastructure exists in the project (W0 added a
`dos309_lint_regex_test.rs` but not a trybuild crate).

**Open question for cycle-1 reviewers:** does this project use trybuild
already, or do we need to add the dev-dependency? If not, fall back to a
visibility trick (`pub(crate)` on `Global` + a public re-export in
`pub mod global_subject_v15_only`). Easier; no new dev-dep.

### 5. Tests in `db/claim_invalidation.rs::tests`

- `bump_claim_version_account_increments_column` — single-account bump goes 0→1.
- `bump_claim_version_each_kind_targets_correct_table` — one test per SubjectKind variant (4 production + 1 Global).
- `bump_claim_versions_multi_sorted_deterministic` — given subjects in random order, the SQL execution order matches the sorted order.
- `bump_claim_versions_multi_dedups` — same subject repeated → bumped once.
- `bump_claim_versions_multi_concurrent_no_deadlock` — two tasks, each with overlapping subject sets in different input orders, both complete successfully (no deadlock).

## 3. Key decisions

- **Per-entity column on each entity table** (Option A, ticket-locked): chosen over a separate `claim_versions` table to keep cache lookups single-row reads alongside the entity. SQLite ALTER TABLE ADD COLUMN is O(1) for NOT NULL DEFAULT.
- **`SubjectKind` enum with explicit `Global` variant**: documents the v1.5 forward-compat path without enabling it. `#[doc(hidden)]` + trybuild block.
- **Sort key for Multi**: `(SubjectKind discriminant, entity_id)` lexicographic. Stable and deterministic. Cycle-1 review: confirm this is acceptable for forward-compat with v1.5 thread/topic subjects.
- **`#[must_use]` on the bump methods** — same pattern as DOS-309 PR 1. Prevents silent discard.
- **Inside `with_transaction` only**: the bump SQL itself doesn't START a transaction; it expects to run inside one DOS-7 opens. Doc-comment makes this explicit.
- **Separate from `entity_graph_version`**: do NOT add claim_version triggers, do NOT bump `entity_graph_version` on claim writes (Codex round-1 finding 4). The two caches are separate domains.
- **Global counter exists but NOT bumped**: spine restriction. v1.5 lifts the restriction.
- **CI enforcement layer**: cycle-1 reviewer picks between bash-grep CI lint vs Rust unit test. Recommend bash-grep matching DOS-309's pattern (function-call denylist).

## 4. Security

- **No new attack surfaces.** ALTER TABLE ADD COLUMN; new module with no user input; CI lint that runs at build time.
- **No PII handling.** Counters and entity IDs only.
- **Cross-tenant exposure: zero.** Each `claim_version` is keyed by `entity_id` on the entity's own row; no cross-entity reads.
- **Spine restriction enforcement (`Global` variant blocked)**: trybuild + `#[doc(hidden)]` block external construction. v1.5 lifts the block when the consumer ships.
- **CI lint integrity**: the lint must run in CI before clippy (so a regression on the column-write boundary is caught early). Same step ordering as DOS-309's lint.

## 5. Performance

- **Per-claim-write overhead**: 1 additional `UPDATE entity SET claim_version = claim_version + 1 WHERE id = ?` per write. Indexed primary-key lookup; trivial.
- **Multi-subject sorted-lock cost**: Vec sort on a small (typically 1-3) subject list. Negligible.
- **Hot path frequency**: claim writes happen inside DOS-7's `commit_claim`. Frequency is per-claim — bounded by enrichment cycle rate (~10s of writes per minute peak per workspace).
- **NO trigger thrash**: by design, this primitive does NOT add triggers. The existing `entity_graph_version` triggers fire only on `account_domains` / `account_stakeholders` / keyword updates — none of which the claim layer touches.
- **Suite P W1 baseline (this wave's gate)**: this primitive becomes part of the criterion baseline. Provisional budget: per-entity bump < 1ms; multi (3 subjects) < 3ms. Reviewer confirms baseline-establishment rights at end of W1.

## 6. Coding standards

- **Services-only mutations**: this primitive lives in `db/`, but it's the EXCLUSIVE writer of `claim_version` columns. CI lint enforces. No service-layer code calls `UPDATE accounts SET claim_version` directly; they call `db.bump_claim_version(...)` or `db.bump_claim_versions_multi(...)`.
- **Intelligence Loop 5-question check (CLAUDE.md):**
  1. Signals: claim writes don't emit new signals as part of this primitive (DOS-7 emits the signal via the existing signal-bus path with the new claim_version included as cache key).
  2. Health scoring: cache invalidation only — no input-shape change. Health scoring's existing `entity_graph_version` reads remain unchanged.
  3. `build_intelligence_context()` / `gather_account_context()`: unchanged. (DOS-7 will read claim_version downstream as a cache key.)
  4. Briefing callouts: no new callout types.
  5. Bayesian source weights: unaffected.
  → No new Intelligence Loop integration. Pure substrate primitive.
- **No `Utc::now()` in services/abilities (W2-A invariant):** N/A — this is `db/` and the bump is a pure SQL UPDATE.
- **No customer data in fixtures:** test fixtures use placeholder IDs (`acc-1`, `proj-2`, etc.).
- **Clippy budget:** zero new warnings under `-D warnings` (lib-only; pre-existing test-target warnings unrelated to this PR).
- **Doc comments:** full doc on `SubjectKind`, `Subject`, `bump_claim_version`, `bump_claim_versions_multi`. Spine restriction documented prominently on the `Global` variant.
- **CO-AUTHORED-BY:** commit message includes `Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>`.

## 7. Integration with parallel wave-mates

- **DOS-311 (W1-B)** owns: schema epoch table, universal `write_intelligence_json` fence, intel_queue drain protocol, reconcile SQL. **No file overlap** with this PR.
- **Migration numbering**: DOS-310 claims migration 123 (next sequential after 122). DOS-311 claims 124. Whichever PR opens first locks the number; the second rebases. Confirmed at PR-open time.
- **Forward-compat with downstream waves:**
  - **DOS-7 (W3-C, post-2026-04-26 absorbing DOS-308)**: `commit_claim` calls `db.bump_claim_version(...)` or `db.bump_claim_versions_multi(...)` inside its transaction. The Multi case applies when a single claim's subject_ref carries multiple entities (e.g., a contract-end claim affecting both account and project).
  - **DOS-301 (W3-D projection writer)**: reads `claim_version` as a cache key when deciding whether to repaint derived state.
  - **DOS-216 (W3 Eval harness)**: fixture loaders set `claim_version` to known values; this primitive is the seam.
- **Cross-cutting deny list:** `accounts.claim_version`, `projects.claim_version`, `people.claim_version`, `meetings.claim_version`, `global_claim_epoch.epoch` — only writeable via `db/claim_invalidation.rs` per the new lint.

## 8. Failure modes + rollback

- **Migration safety**: ALTER TABLE ADD COLUMN with NOT NULL DEFAULT is forward-compatible. Existing rows get `claim_version = 0`. `global_claim_epoch` is a single-row table with default value 0.
- **Concurrent multi-subject deadlock**: prevented by sorted-lock. The test `bump_claim_versions_multi_concurrent_no_deadlock` stresses this.
- **Spine restriction violation (someone constructs `SubjectKind::Global` in production code)**: blocked at compile time by trybuild + `#[doc(hidden)]`. If trybuild infrastructure isn't available, the visibility-trick fallback (pub(crate) + restricted re-export) achieves the same effect.
- **Rollback path**: revert the migration would require ALTER TABLE DROP COLUMN — SQLite supports this since 3.35 (project's SQLite version: confirm at PR-open). If pre-3.35, fallback is to leave the column in place (it's harmless without writers) and rely on the missing `db/claim_invalidation.rs` to neuter the writer. Either way, no data corruption — the column is additive.
- **DOS-7 calling this BEFORE this issue ships**: not possible by wave ordering. Cross-checked.
- **Universal write fence (DOS-311 / W1-B) interaction**: the DOS-310 bump runs inside `with_transaction`, which DOS-311's fence wraps. Compatible. The fence drains intel_queue but doesn't block claim writes; the bump runs as fast as a single SQL UPDATE.

## 9. Test evidence to be produced

**Unit tests in `src-tauri/src/db/claim_invalidation.rs::tests`:**
- `bump_claim_version_account_increments` — 0 → 1 single-row.
- `bump_claim_version_project_increments` — same for projects.
- `bump_claim_version_person_increments` — same for people.
- `bump_claim_version_meeting_increments` — same for meetings.
- `bump_claim_version_global_increments_epoch_table` — Global routes to `global_claim_epoch.epoch`. (Test bypasses the spine restriction via `#[cfg(test)]` access.)
- `bump_claim_version_account_unknown_id_no_op` — unknown ID does not error (SQLite UPDATE no-op semantics; test asserts row count).
- `bump_claim_versions_multi_sorted` — input `[Project("b"), Account("a"), Person("c")]` produces the deterministic sorted order `[Account("a"), Person("c"), Project("b")]`. Verifies via captured SQL execution log.
- `bump_claim_versions_multi_dedups` — `[Account("a"), Account("a"), Project("b")]` → 2 bumps, not 3.
- `bump_claim_versions_multi_empty_no_op` — empty slice → Ok(()) with zero SQL.
- `bump_claim_versions_multi_concurrent_no_deadlock` — spawn 8 tokio tasks each bumping a randomly-shuffled overlapping subject set; all complete within 1s.

**Trybuild test (or visibility-trick test depending on cycle-1 ruling):**
- `trybuild_dos310/global_subject_forbidden.rs` — production code attempting to construct `SubjectKind::Global` fails to compile.

**CI lint test:**
- `scripts/check_claim_version_writers.sh` runs in CI; greps for direct `UPDATE accounts SET claim_version` etc. outside `db/claim_invalidation.rs` and `migrations/`. Exits non-zero on any match.

**Wave merge-gate artifacts:**
- All unit tests pass under `cargo test`.
- `cargo clippy -D warnings` lib-clean.
- New CI lint script runs in `.github/workflows/test.yml` after the DOS-309 lint.
- Trybuild fixture (or fallback) compile-test green.
- Suite P baseline: criterion bench results captured at end of W1.
- **Integration with DOS-311**: DOS-311 cycle-1 reviewer confirms migration 123 + 124 do not conflict; this primitive's bump path is compatible with DOS-311's fence.

**Suite E contribution:** sorted-lock ordering property test green (continuous); spine-restriction trybuild check green (continuous).

## 10. Open questions

For L0 cycle-1 reviewers to confirm or redirect:

1. **Trybuild availability:** does the project use trybuild as a dev-dep, or do we need to add it? If not, the visibility-trick fallback (pub(crate) on `Global` + restricted re-export) achieves the spine restriction without new infrastructure. Recommend: confirm at PR-open.
2. **Migration number coordination with DOS-311:** both target 123. Convention proposed: DOS-310 takes 123, DOS-311 takes 124. Cycle-1 reviewer confirms or proposes alternative.
3. **CI lint mechanism (bash grep vs Rust test):** bash-grep matches DOS-309's pattern; Rust test could be more structural. Recommend: bash-grep.
4. **Sort key for Multi**: `(SubjectKind discriminant, entity_id)` lexicographic. Acceptable for v1.5+ forward-compat (e.g., when Theme/Thread subjects ship), or should the sort key be more abstract (e.g., a `subject.lock_key() -> (u32, &str)` method)? Recommend: ship the simple form; extend in v1.5.
5. **`bump_claim_version_account_unknown_id_no_op` semantics**: SQLite UPDATE on a non-existent ID silently affects 0 rows. Should the bump method warn-log on 0-affected? Recommend: yes (helps debugging at DOS-7 PR-open) but not error.
6. **`global_claim_epoch` alongside spine restriction**: ship the column + table + bump path, but NO production writer. Reviewer confirms this is the right shape (vs. defer the table entirely to v1.5).

**Linear MCP token expired** at plan-write time. Cycle-1 reviewers should pull DOS-310 ticket directly to verify against the live ticket text and flag any 2026-04-24+ amendments not captured here.
