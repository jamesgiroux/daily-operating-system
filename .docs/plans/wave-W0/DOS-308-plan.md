# Implementation Plan: DOS-308 — `is_suppressed()` correctness

**Agent slot:** W0-A
**Status:** L0 cycle 2 pending review
**Cycle 1 verdict:** REVISE (unanimous, 3 of 3 reviewers); see §11 for disposition of every finding
**Plan author:** orchestrator (Claude Code)
**Domain reviewer assigned:** architect-reviewer (substrate primitive correctness; matrix gap noted in §7)
**Linear ticket:** [DOS-308](https://linear.app/a8c/issue/DOS-308)

---

## 1. Contract restated

The ticket calls out that `is_suppressed()` (`src-tauri/src/db/intelligence_feedback.rs:70-101`) is the "load-bearing primitive my v1.4.0 spine plan said to lean on" and that Codex round 1 + 2 caught six correctness defects:

> "1. Lexicographic string comparison of timestamps … 2. Reads one arbitrary matching row — no `ORDER BY` … 3. Ignores `item_hash` … 4. Ignores `superseded_by_evidence_after` … 5. Exact text match only on `item_key` … 6. Round 2 addition: existing callers use `.unwrap_or(false)` (`intel_queue.rs:2020`). When this fix starts returning errors for malformed tombstones, every caller silently fails open."

Round 2 also added:

> "Codex round 2 caught that round 1's spec only covered the tombstone side."

→ both sides of the timestamp comparison must be parsed (tombstone-side `dismissed_at` / `expires_at` / `superseded_by_evidence_after` AND item-side `sourced_at`).

**Note on caller line numbers:** the ticket cites `intel_queue.rs:2020,2024,2036`; current file has shifted to `:2078,2090`. Verified manually 2026-04-26.

Mandatory deliverables per the acceptance-criteria checklist:

1. Replace `bool` return with `SuppressionDecision` enum, **infallible signature** (per cycle-1 finding R1#6: `Result<SuppressionDecision>` lets callers `.map(...).unwrap_or(false)` and silently fail open). DB errors fold into a `Malformed { reason: DatabaseError }` variant.
2. Parse both tombstone-side and item-side timestamps via `chrono::DateTime::parse_from_rfc3339` (with legacy fallback).
3. Tombstone selection by precedence (item_hash > exact item_key > keyless field-wide), latest-wins within a precedence tier — implemented in Rust over a top-N candidate fetch (per cycle-1 R1#1+#2 BLOCKERs: pure SQL `ORDER BY dismissed_at DESC` inverts specificity).
4. Honor `item_hash`, `superseded_by_evidence_after`, `expires_at`.
5. 14 parity-test cases covering Suppressed/NotSuppressed/Malformed + each precedence level + each MalformedReason field + the `InvalidExpiry` semantic.
6. Caller migration at `intel_queue.rs:2078, 2090` — fail-closed on `Malformed` at the **caller** (substrate primitive returns the typed decision; caller picks the policy).
7. Production audit script + remediation script (quarantine table, multi-tombstone collapse, expired-row marking). **Migration gate enforced structurally**: DOS-7's first migration self-aborts if `suppression_tombstones_quarantine` is non-empty (per cycle-1 architect#4 + R1#8 — TOCTOU race, convention insufficient).

Cycle-1 amendments referenced: see §11 for the full mapping.

## 2. Approach

End-state alignment: this is the corrected substrate primitive that DOS-7's `commit_claim` PRE-GATE leans on per ADR-0113. After DOS-7 lands in W3, `is_suppressed()` will be rewritten to read from `intelligence_claims`; this PR is the bridge that makes round-1+2 findings impossible at the legacy substrate AND ensures the enum shape forward-ports cleanly without breaking changes (per cycle-1 architect#1+#2).

**Files created/modified:**

### 1. **`src-tauri/src/intelligence/canonicalization.rs`** (NEW shared module — cycle-1 architect#3 BLOCKER)

```rust
//! Canonical hashing for tombstone item identity.
//! Shared by W0 callers and DOS-7 commit_claim/propose_claim.

use sha2::{Digest, Sha256};
use unicode_normalization::UnicodeNormalization;

/// The kind of intelligence item being hashed; reserved for forward-compat
/// with DOS-7's claim_type registry (ADR-0125). For W0 we only emit `Risk`/`Win`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ItemKind {
    Risk,
    Win,
    /// Reserved for DOS-7 expansion; not emitted in W0.
    #[doc(hidden)]
    _Reserved,
}

/// Canonical hash for tombstone matching.
///
/// Stable rule (locked for v1.4.0):
///   1. Trim leading/trailing whitespace.
///   2. NFC-normalize Unicode.
///   3. Collapse internal whitespace runs to a single space.
///   4. SHA-256 hex digest.
///
/// **Not** case-folded (preserves trademark/proper-noun discrimination).
/// **Not** punctuation-stripped (preserves "ARR at risk" vs "ARR at risk?").
///
/// DOS-7's `dedup_key` (ADR-0113 §8) composes this helper plus `(entity_id, claim_type, field_path)`.
pub fn item_hash(_kind: ItemKind, text: &str) -> String {
    let trimmed = text.trim();
    let nfc: String = trimmed.nfc().collect();
    let collapsed: String = nfc.split_whitespace().collect::<Vec<_>>().join(" ");
    let mut hasher = Sha256::new();
    hasher.update(collapsed.as_bytes());
    format!("{:x}", hasher.finalize())
}
```

Doc-comment on `item_hash` warns: changing the canonicalization rule is a migration; v1.4.0 locks it.

### 2. **`src-tauri/src/db/intelligence_feedback.rs`** (modify)

New module-private types:

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SuppressionDecision {
    /// Tombstone present; suppress the item.
    Suppressed {
        tombstone_id: TombstoneId,
        reason: SuppressionReason,
        dismissed_at: DateTime<Utc>,        // cycle-1 architect#2: PRE-GATE audit lineage
        source_scope: Option<String>,        // cycle-1 architect#2: PRE-GATE audit lineage
    },
    /// No matching tombstone, OR all matches are expired/superseded.
    NotSuppressed,
    /// A candidate row was malformed. Caller MUST decide fail-closed or fail-open
    /// explicitly with audit emission. Compile-time enforcement via non-exhaustive
    /// match (function returns this value, not Result).
    Malformed {
        record_id: TombstoneId,
        reason: MalformedReason,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SuppressionReason {
    HashMatch,
    ExactTextMatch,
    KeylessFieldSuppression,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MalformedReason {
    UnparsableTimestamp { field: &'static str },     // cycle-1 R1#11: drop value capture (PII risk + bounded length)
    InvalidExpiry,                                    // expires_at < dismissed_at
    DatabaseError(String),                            // cycle-1 R1#6: rusqlite errors fold here
}

/// Opaque tombstone identifier. Forward-compatible with DOS-7's UUID claim_id —
/// stringifies legacy INTEGER PRIMARY KEY now; DOS-7 reads UUIDs from intelligence_claims.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TombstoneId(pub String);                  // cycle-1 architect#1: String, not i64
```

**Function signature** (cycle-1 R1#6 BLOCKER + architect#1):

```rust
pub fn is_suppressed(
    &self,
    entity_id: &str,
    field_key: &str,
    item_key: Option<&str>,
    item_hash: Option<&str>,                          // cycle-1 R1#5 + architect#3
    sourced_at: Option<&str>,
) -> SuppressionDecision {
    // ... infallible. DB errors fold into Malformed::DatabaseError.
}
```

**SQL: fetch top-N candidates, resolve precedence in Rust** (cycle-1 R1#1+#2 BLOCKERs):

```sql
SELECT id, dismissed_at, expires_at, superseded_by_evidence_after,
       item_hash, item_key, source_scope
FROM suppression_tombstones
WHERE entity_id = ?1
  AND field_key = ?2
  AND (item_key IS NULL                              -- keyless field-wide
       OR item_key = ?3                              -- exact item_key match
       OR (item_hash IS NOT NULL AND item_hash = ?4)) -- hash match
ORDER BY dismissed_at DESC
LIMIT 32;
```

The `LIMIT 32` is a sanity bound; precedence resolution in Rust:

1. **Hash-match candidates first** (if `item_hash` provided). Iterate hash-match rows newest-first; first non-malformed → `Suppressed { reason: HashMatch }`. If all hash-match rows are malformed, return `Malformed { record_id, reason }` for the most-recent malformed row (per cycle-1 R1#4 — scope `Malformed` to the matching tier, not the field).
2. **Exact item_key match** (if `item_key` provided AND no valid hash match). Same iterate-then-decide pattern.
3. **Keyless field-wide tombstone**. Same pattern.
4. **Otherwise** → `NotSuppressed`.

Within each tier, parse `dismissed_at`, `expires_at?`, `superseded_by_evidence_after?`, `sourced_at?`. Decision logic per row:
- `expires_at` < now → skip (treat as not matching this tier; iterate to next candidate or fall through).
- `superseded_by_evidence_after` is set AND parsed `sourced_at > superseded_by_evidence_after` → skip.
- `expires_at < dismissed_at` (logical inversion) → emit `Malformed { InvalidExpiry }` for this row.
- Any field unparseable → emit `Malformed { UnparsableTimestamp { field } }`.
- Else → `Suppressed`.

**Iterating-past-malformed semantics** (cycle-1 R1#4 fix): a single malformed row no longer "stuck-dismisses" the field. We iterate to the next non-malformed candidate within the same tier. We only return `Malformed` if ALL candidates in the matching tier are malformed.

### 3. **New covering index migration** `src-tauri/src/migrations/{N+1}_suppression_remediation.sql`

```sql
-- Covering index for is_suppressed precedence query (cycle-1 R1#3 + architect#perf).
CREATE INDEX IF NOT EXISTS idx_tombstones_lookup
  ON suppression_tombstones(entity_id, field_key, dismissed_at DESC);

-- Quarantine table for malformed rows.
CREATE TABLE IF NOT EXISTS suppression_tombstones_quarantine (
    id INTEGER PRIMARY KEY,                          -- mirrors source row id
    entity_id TEXT NOT NULL,
    field_key TEXT NOT NULL,
    item_key TEXT,
    item_hash TEXT,
    dismissed_at TEXT,                               -- raw value for operator review
    source_scope TEXT,
    expires_at TEXT,
    superseded_by_evidence_after TEXT,
    quarantined_at TEXT NOT NULL DEFAULT (datetime('now')),
    quarantine_reason TEXT NOT NULL                  -- enum of MalformedReason variants
);
```

Note: `expires_at_processed` BOOLEAN column dropped (cycle-1 R1#10: nobody reads it). Expired rows are filtered at query time; the audit/remediation script doesn't need a persistent flag.

### 4. **`src-tauri/src/intel_queue.rs`** (modify two call sites)

The closure-doesn't-compile concern (cycle-1 R1#7, R3#2 — `retain` takes `FnMut(&T) -> bool`, not `Result<bool>`). **Locked pattern**: handle `Malformed` inside the closure with the fail-closed default; no `?` propagation needed because the function is now infallible. Final code:

```rust
if let Ok(feedback_db) = crate::db::ActionDb::open() {
    use crate::db::{SuppressionDecision, SuppressionReason, MalformedReason};
    use crate::intelligence::canonicalization::{item_hash, ItemKind};

    let pre_risk_count = final_intel.risks.len();
    final_intel.risks.retain(|risk| {
        let hash = item_hash(ItemKind::Risk, &risk.text);
        let decision = feedback_db.is_suppressed(
            &input.entity_id,
            "risks",
            Some(risk.text.as_str()),
            Some(&hash),
            risk.item_source.as_ref().map(|s| s.sourced_at.as_str()),
        );
        match decision {
            SuppressionDecision::Suppressed { .. } => false,  // suppress (drop from retain set)
            SuppressionDecision::NotSuppressed => true,
            SuppressionDecision::Malformed { record_id, reason } => {
                log::error!(
                    "[is_suppressed] malformed tombstone {:?} for entity {} field risks; \
                     failing closed: {:?}",
                    record_id, input.entity_id, reason
                );
                // TODO(DOS-7): emit audit event via claim_repair_job once W3 lands.
                false  // fail-closed: over-suppress (drop from retain set)
            }
        }
    });
    // ... same shape for recent_wins.
}
```

This compiles. No `Result` to propagate. No `.unwrap_or(false)` possible.

### 5. **`src-tauri/scripts/audit_suppression_tombstones.rs`** (new) — unchanged from cycle 1, plus:

Cycle-1 R1#8 fix (TOCTOU): the audit script outputs a JSON report **and** writes a `quarantine_pending` row to a metadata table that DOS-7's migration consults. DOS-7's first migration runs:

```sql
SELECT count(*) FROM suppression_tombstones_quarantine;
-- Aborts migration if non-zero.
```

This is a structural gate (cycle-1 architect#4 + R1#8), enforceable in CI invariants and in the migration runtime.

### 6. **`src-tauri/scripts/remediate_suppression_tombstones.rs`** (new)

Three actions per the ticket. Multi-tombstone-collapse logs candidates with `TODO(DOS-7)` reference; does not modify production data (DOS-7's W3 backfill does the corroboration append).

### 7. **`v1.4.0-waves.md` invariant table update** (cycle-1 architect#4)

Add row:
```
| Quarantine table empty before DOS-7 migration | Migration self-abort SQL | W3 |
```

Filed as W0→W3 handshake artifact, not a W0 deliverable.

## 3. Key decisions (cycle-1 closures)

- **`TombstoneId` newtype**: `pub struct TombstoneId(pub String)`. Stringify legacy `INTEGER PRIMARY KEY` now; DOS-7 reads UUIDs unchanged. Forward-compatible. (cycle-1 architect#1)
- **Function signature: infallible**. `fn is_suppressed(...) -> SuppressionDecision`. Rusqlite errors fold into `Malformed { reason: DatabaseError(String) }`. No `Result` to unwrap; callers cannot `.unwrap_or(false)`. (cycle-1 R1#6)
- **`item_hash` parameter**: `item_hash: Option<&str>`. Caller computes via shared helper `crate::intelligence::canonicalization::item_hash(ItemKind, &str)`. (cycle-1 R1#5 + architect#3)
- **Canonicalization rule**: trim → NFC → collapse-whitespace → SHA-256 hex. NOT case-folded; NOT punctuation-stripped. Locked for v1.4.0. (cycle-1 R1#5 + architect#3)
- **SQL precedence**: top-32 candidate fetch + Rust-side precedence (hash > exact key > keyless), iterate-past-malformed within tier. (cycle-1 R1#1+#2 BLOCKERs + R1#4 stuck-dismissal fix)
- **Covering index**: `idx_tombstones_lookup (entity_id, field_key, dismissed_at DESC)` ships in this PR's migration. (cycle-1 R1#3)
- **`Suppressed` lineage fields**: `tombstone_id`, `reason`, `dismissed_at`, `source_scope`. PRE-GATE-ready for DOS-7. (cycle-1 architect#2)
- **`SuppressionReason`**: 3 variants (HashMatch, ExactTextMatch, KeylessFieldSuppression). Unchanged from cycle 1.
- **`MalformedReason`**: 3 variants (UnparsableTimestamp { field }, InvalidExpiry, DatabaseError(String)). `value` field dropped per cycle-1 R1#11 (PII surface, bounded-length argument was theatre).
- **`expires_at_processed` column**: DROPPED. Filter at query time. (cycle-1 R1#10)
- **Caller closure shape**: locked match-on-decision with fail-closed default inside the closure. Compiles cleanly; no `?` needed. (cycle-1 R1#7 + R3#2)
- **Migration gate**: structural via DOS-7 migration self-abort; convention is insufficient. Filed in v1.4.0-waves.md invariant table. (cycle-1 architect#4 + R1#8)
- **Audit emission target on `Malformed`**: log-only for W0 with `TODO(DOS-7)` to route to `claim_repair_job` once W3 lands. (cycle-1 closes prior §10 Q1)
- **Reviewer matrix gap for W0**: architect-reviewer is the right slot. Filed as W0 retro observation. (closes prior §10 Q4)

## 4. Security

- **No new attack surfaces.** Function is read-only on the existing schema.
- **Cross-tenant exposure:** zero (function is keyed by `entity_id`). Hash-match additionally narrows: `item_hash` is content-addressed within `(entity_id, field_key)` scope.
- **PII in logs:** `MalformedReason::UnparsableTimestamp { field }` — **field name only**, no value capture. (cycle-1 R1#11) Operators look up the row by `record_id` in the quarantine table.
- **`MalformedReason::DatabaseError(String)`**: rusqlite error message. Reviewed for PII surface — rusqlite errors carry SQL fragments and column names, not row data. Acceptable.
- **Audit emission integrity:** the fail-closed default at the caller is "over-suppress" — operationally safer than fail-open ghost-resurrection.
- **Stuck-dismissal fix** (cycle-1 R1#4): iterating past malformed candidates within a tier prevents a single corrupt row from blocking an entire field. The fail-closed default applies only when ALL candidates in the matching tier are malformed.
- **Production audit script:** read-only on `suppression_tombstones`. Remediation script writes only to the new `suppression_tombstones_quarantine` table. No destructive operations on the source table.
- **Quarantine table outside the universal write fence (DOS-311)**: intentional — operator-controlled remediation surface, not authoritative state. Documented in §8. (cycle-1 architect#5)

## 5. Performance

- **Hot-path call frequency:** `intel_queue.rs:2075` and `:2087` — `N+M` calls per enrichment cycle (typically 10-30 per entity).
- **SQL plan with new covering index** (`idx_tombstones_lookup (entity_id, field_key, dismissed_at DESC)`): index-seek on `(entity_id, field_key)` partition, ordered by `dismissed_at DESC` natively, `LIMIT 32` short-circuit. **No filesort**. (cycle-1 R1#3 fix)
- **EXPLAIN QUERY PLAN evidence**: implementer captures and attaches to PR body before merge.
- **Per-call overhead**: index seek + ≤32 row materializations + Rust-side precedence resolution (~1-2µs per row chrono parse, max 32 × 4 fields × 2µs = ~256µs worst case; typical ~5-10µs). Negligible.
- **Hash computation cost** (caller-side via `item_hash` helper): NFC-normalize + collapse-whitespace + SHA-256 over a short string ≈ 5-10µs. Per-item, hot path. Acceptable.
- **Allocation profile:** decision enum carries `String` for `TombstoneId` and optionally `source_scope`. One small allocation per `Suppressed` return; zero per `NotSuppressed`. Tested in W1 P-suite baseline.
- **Budget for W1 P-suite baseline**: provisional `is_suppressed` p99 < 200µs (revised up from prior 100µs to account for top-32 candidate read). Will refine after W1 criterion run.

## 6. Coding standards

- **Services-only mutations:** N/A — this is `db/` read code.
- **Intelligence Loop 5-question check (CLAUDE.md):** unchanged from cycle 1 — read-only path, no Loop integration applies.
- **No `Utc::now()` in services/abilities:** function is in `db/`. Uses `chrono::Utc::now()` for the `expires_at < now` check. Acceptable for W0; W2-A migrates to `ServiceContext.clock` later.
- **No customer data in fixtures:** test fixtures use placeholder IDs.
- **Clippy budget:** zero new warnings under `-D warnings`.
- **Doc comments on public APIs:** new public types and the rewritten `is_suppressed` get full doc comments. Doc-comment on `is_suppressed` documents: precedence ladder, fail-closed semantics, infallible signature rationale, item_key/item_hash/keyless interaction.
- **Spine restriction (no Global subject)** (cycle-1 architect#8): doc-comment states the function operates only on entity-scoped tombstones; DOS-7's rewrite preserves this restriction.
- **CO-AUTHORED-BY:** commit message includes `Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>`.

## 7. Integration with parallel wave-mates

- **DOS-309 (W0-B)** owns `services/intelligence.rs:1055-1180` + `services/accounts.rs:1090, 1146, 1158`. **No file overlap.**
- **Migration numbering**: this PR claims migration N+1 (one above current head at branch-open). DOS-309 adds no migrations, so no collision.
- **Reviewer matrix gap for W0**: architect-reviewer assigned. Filed as L0 retro observation.
- **Forward-compat guarantees** (cycle-1 architect#1+#2+#3 fixes ensure):
  - DOS-7's `commit_claim` PRE-GATE consumes `SuppressionDecision::Suppressed` directly (lineage fields populated).
  - DOS-7's rewrite of `is_suppressed` reads `intelligence_claims` rows where `claim_state = 'tombstoned'`, returns the same enum shape, no breaking change at callers.
  - DOS-7 will likely add a fourth variant `Superseded` (per cycle-1 R1#9 — claim lifecycle states don't all map to the W0 trichotomy). Mark `SuppressionDecision` as `#[non_exhaustive]` on the public surface to allow forward extension; internal callers still match exhaustively.
  - DOS-7's `dedup_key` (ADR-0113 §8) composes the same `canonicalization::item_hash` helper.

## 8. Failure modes + rollback

- **Production migration safety**: backward-compatible. New schema adds the `idx_tombstones_lookup` index and the `suppression_tombstones_quarantine` table. `expires_at_processed` column is NOT added (dropped per R1#10).
- **Malformed records in production**: audit script reports findings; remediation moves them to `suppression_tombstones_quarantine`; DOS-7's migration self-aborts if quarantine is non-empty (cycle-1 architect#4 + R1#8 — structural, not convention).
- **Caller fail-closed default** (cycle-1 R1#4 stuck-dismissal fix): only applies when ALL candidates in the matching tier are malformed. Single corrupt row no longer poisons the field.
- **Quarantine table outside universal fence** (cycle-1 architect#5): intentional. Operator-controlled remediation surface; not part of authoritative state. Drop-with-source-table after DOS-7 verifies empty.
- **`TombstoneId(String)` migration cost** (cycle-1 architect#1): zero — both legacy i64 and DOS-7 UUIDs stringify cleanly.
- **Rollback path**: revert PR. Risk: previous bugs return. Compensating control: quarantine rows preserved on revert; operators retain visibility.

## 9. Test evidence to be produced

**Unit tests in `src-tauri/src/db/intelligence_feedback.rs::tests` (15 cases — cycle-1 R3#3 InvalidExpiry + R3#6 null-key-with-hash):**

1. `is_suppressed_exact_text_match` → `Suppressed { reason: ExactTextMatch }`
2. `is_suppressed_null_item_key_keyless` → `Suppressed { reason: KeylessFieldSuppression }`
3. `is_suppressed_picks_latest_within_tier` (3 hash-match tombstones; latest wins)
4. `is_suppressed_expired_skipped_falls_through` (expired tier-1 candidate ignored; tier-2 candidate decides)
5. `is_suppressed_superseded_by_newer_sourced_at` → `NotSuppressed`
6. `is_suppressed_superseded_with_older_sourced_at` → `Suppressed`
7. `is_suppressed_timestamp_z_vs_offset_consistent`
8. `is_suppressed_subsecond_precision_consistent`
9. `is_suppressed_malformed_dismissed_at_within_tier_isolated` (one malformed candidate; iterate to next non-malformed and return `Suppressed`) — cycle-1 R1#4
10. `is_suppressed_all_malformed_in_tier_returns_malformed` (every candidate in matching tier malformed) — cycle-1 R1#4
11. `is_suppressed_malformed_item_sourced_at` → `Malformed { UnparsableTimestamp { field: "sourced_at" } }`
12. `is_suppressed_hash_match_beats_exact_key` (cycle-1 R3#6: precedence within mixed-tier candidate set)
13. `is_suppressed_inverted_expiry_emits_malformed` → `Malformed { InvalidExpiry }` (cycle-1 R3#3)
14. `is_suppressed_no_tombstone` → `NotSuppressed`
15. `is_suppressed_different_field_key` → `NotSuppressed`

**Property tests in `src-tauri/src/db/intelligence_feedback.rs::tests` (Suite E continuous):**

- `prop_is_suppressed_decision_total`: for any combination of well-formed timestamps **with valid expiry relations**, returns `Suppressed` or `NotSuppressed` (never `Malformed`). For any combination including a malformed timestamp OR an inverted expiry, the matching tier surfaces `Malformed` only when ALL candidates in that tier fail. (cycle-1 R3#12: contradiction with `InvalidExpiry` resolved by clarifying the property.)
- `prop_canonicalization_stable`: `item_hash(_, x) == item_hash(_, y)` iff `x` and `y` produce identical canonical forms (NFC-normalized, whitespace-collapsed, trimmed). (cycle-1 architect#3)

**Integration tests in `src-tauri/src/intel_queue.rs::tests`:**

- `intel_queue_fail_closed_on_malformed_tombstone` — insert a tombstone with malformed `dismissed_at`, run filter; assert risk is dropped from `final_intel.risks` AND `log::error!` was emitted (via `testing_logger`).
- `intel_queue_passes_through_when_not_suppressed`.
- `intel_queue_suppresses_on_hash_match_with_text_diff` (Codex finding 5 regression test).
- `intel_queue_does_not_stuck_dismiss_field_with_one_malformed_row` (cycle-1 R1#4).

**EXPLAIN QUERY PLAN evidence:**
- `cargo test --test query_plan` runs the new query against a populated test DB and asserts the plan uses `idx_tombstones_lookup`. Output captured in PR body.

**Production-data audit:**
- Audit script run output committed as `.docs/plans/wave-W0/DOS-308-audit-report.json` (gitignored if sensitive).
- Quarantine table empty pre-DOS-7-migration verified via the migration self-abort mechanism.

**Wave merge-gate artifacts:**
- All unit + property + integration tests above passing under `cargo test`.
- `cargo clippy -D warnings` clean.
- EXPLAIN QUERY PLAN output linked from PR body.
- Audit-report file linked from PR body.

**Suite E contribution:** ghost-resurrection regression test green; timestamp-parsing property test green; canonicalization-stability property test green.

## 10. Open questions

Closed in cycle 1: prior Q1 (audit emission target → log-only with TODO(DOS-7)), prior Q4 (reviewer matrix slot → architect-reviewer).

Remaining open for L0 cycle-2 reviewers:

1. **`#[non_exhaustive]` on `SuppressionDecision`** (cycle-1 architect#7 / R1#9): adding it now lets DOS-7 introduce a `Superseded` variant without a breaking change, BUT it weakens the "exhaustive matching at every caller" property. Trade-off: external crates can't pattern-match exhaustively (irrelevant — no external crates today); internal callers still match exhaustively because internal modules see the full enum. **Recommend: ship `#[non_exhaustive]`.** Confirm.
2. **`record_id: TombstoneId` field on `Suppressed` variant**: architect#2 added `dismissed_at` and `source_scope`; should we also add `tombstone_id`'s siblings (e.g., `item_hash` of the matching tombstone, for downstream attribution)? Recommend: defer to DOS-7 — the tombstone_id is enough for lookup; downstream can query.
3. **`audit_suppression_tombstones.rs` placement**: `src-tauri/scripts/` is the proposed home. Verify the project's existing convention (some scripts live at repo-root `scripts/`).

## 11. L0 cycle-1 finding disposition

Reviewers: R1 (adversarial), R2 (architect-reviewer), R3 (independent consult). Verdict: REVISE (3 of 3).

| ID | Severity | Section | Disposition |
|---|---|---|---|
| **R1#1** | BLOCKER | SQL `WHERE` makes keyless tombstones unreachable | **Fixed §2.2** — WHERE widened to `(item_key IS NULL OR item_key = ?3 OR (item_hash IS NOT NULL AND item_hash = ?4))`; precedence resolution moved to Rust |
| **R1#2** | BLOCKER | `ORDER BY DESC LIMIT 1` inverts precedence | **Fixed §2.2** — fetch top-32 candidates; resolve precedence in Rust (hash > exact > keyless) |
| **R1#3** | MAJOR | Index doesn't cover `ORDER BY dismissed_at DESC` | **Fixed §2.3** — new covering index `idx_tombstones_lookup`; EXPLAIN evidence required in PR (§9) |
| **R1#4** | MAJOR | Fail-closed creates stuck-dismissal | **Fixed §2.2** — iterate-past-malformed within tier; `Malformed` returned only if ALL candidates in tier fail |
| **R1#5** | MAJOR | `item_hash` canonicalization not specified | **Fixed §2.1** — shared module `intelligence/canonicalization.rs` with locked rule (trim → NFC → collapse-ws → SHA-256) |
| **R1#6** | MAJOR | `Result<>` allows fail-open via `.unwrap_or(false)` | **Fixed §2.2 + §3** — function infallible; rusqlite errors fold into `MalformedReason::DatabaseError` |
| **R1#7** | MAJOR | Caller closure doesn't compile (Result<bool> in retain) | **Fixed §2.4** — locked match-on-decision pattern with fail-closed inside closure; no `?` needed |
| **R1#8** | MAJOR | Migration gate is TOCTOU race | **Fixed §2.5 + §8** — DOS-7 migration self-aborts if quarantine non-empty; structural, not convention; filed in v1.4.0-waves invariant table |
| **R1#9** | MAJOR | DOS-7 forward-port asserted not demonstrated | **Fixed §7 + §10 Q1** — `#[non_exhaustive]` on `SuppressionDecision` allows DOS-7 to add `Superseded` variant without breaking change |
| **R1#10** | MINOR | `expires_at_processed` column nobody reads | **Fixed §2.3** — column dropped; expired rows filtered at query time |
| **R1#11** | MINOR | Truncating malformed value to 32 chars is theatre | **Fixed §2.2 + §4** — `MalformedReason::UnparsableTimestamp` carries `field` only, no value capture |
| **R1#12** | MINOR | Property test contradiction | **Fixed §9** — property restated to handle `InvalidExpiry` correctly |
| **R2#1** | MAJOR | `TombstoneId` should be String for forward-compat | **Fixed §2.2 + §3** — `pub struct TombstoneId(pub String)`; legacy i64 stringifies |
| **R2#2** | MAJOR | `Suppressed` variant lacks PRE-GATE lineage | **Fixed §2.2** — added `dismissed_at`, `source_scope` fields |
| **R2#3** | BLOCKER | `item_hash` canonicalization must be shared helper | **Fixed §2.1** — same fix as R1#5; shared module created |
| **R2#4** | MAJOR | Migration gate enforceability | **Fixed §2.5** — same fix as R1#8; structural via DOS-7 migration self-abort |
| **R2#5** | MINOR | Quarantine table outside DOS-311 fence | **Documented §4 + §8** — intentional, operator-controlled remediation surface |
| **R2#6** | MINOR | Fail-closed default at substrate vs caller | **Confirmed §3** — substrate returns typed decision; caller picks policy. Doc-comment makes contract explicit |
| **R2#7** | MINOR | Forward-compat enum extension | **Fixed §10 Q1** — `#[non_exhaustive]` recommendation |
| **R2#8** | MINOR | Spine restriction (no Global subject) | **Documented §6** — doc-comment states entity-scoped only |
| **R3#1** | MAJOR | Ticket line numbers paraphrased wrong | **Fixed §1** — verification note added |
| **R3#2** | MAJOR | Caller migration trivializes type problem | **Fixed §2.4** — same fix as R1#7; locked compilable pattern |
| **R3#3** | MAJOR | `InvalidExpiry` lacks ticket grounding | **Fixed §9** — added 13th test (`is_suppressed_inverted_expiry`) |
| **R3#4** | MAJOR | `item_hash` upstream writer canonicalization unaudited | **Partial — escalated to §10 Q for cycle-2**: existing `item_hash` rows in production may have been written by an unknown canonicalization. Mitigation: the new shared helper is the only writer going forward; existing rows are read-best-effort with the new canonicalization. If they don't match, those rows behave as if no `item_hash` was set, falling through to `item_key` matching. **Document this in the function's doc-comment.** Cycle-2 reviewers confirm this is acceptable. |
| **R3#5** | MAJOR | Migration-numbering coordination handwave | **Fixed §7** — DOS-309 adds no migrations; this PR claims N+1 unconditionally |
| **R3#6** | MAJOR | Test gap: NULL-key + hash precedence | **Fixed §9** — added `is_suppressed_hash_match_beats_exact_key` test |
| **R3#7** | MAJOR | Perf claim "B-tree descent unchanged" wrong | **Fixed §5** — same fix as R1#3; new covering index + EXPLAIN evidence |
| **R3#8** | MINOR | PII mitigation theatre | **Fixed §4** — same fix as R1#11 |
| **R3#9** | MINOR | Open Qs 1+4 closeable | **Fixed §10** — closed in cycle 1 |
| **R3#10** | MINOR | DOS-311 reference irrelevant for read-only function | **Fixed §8** — clarified |
| **R3#11** | MINOR | TombstoneId justification thin | **Fixed §3** — same fix as R2#1; full justification provided |

**New findings escalated to cycle 2 (open §10):**
- §10 Q1 (`#[non_exhaustive]` recommendation)
- §10 Q2 (additional Suppressed fields)
- §10 Q3 (script placement convention)
- §10 implicit (R3#4 — existing item_hash writer canonicalization audit acceptable as best-effort)

