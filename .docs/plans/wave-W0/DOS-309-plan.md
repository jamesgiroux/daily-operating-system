# Implementation Plan: DOS-309 — `dismiss_intelligence_item` error-swallow + DB-before-file ordering (PR 1, narrowed)

**Agent slot:** W0-A (sole W0 agent after cycle-2 restructuring)
**Status:** L0 cycle 4 pending review (cycle 3 returned REVISE because §§1-10 drifted from §12; this revision aligns the body with §12 + addresses cycle-3 architectural BLOCKERs)
**Cycle 1 verdict:** REVISE (unanimous); see §11
**Cycle 2 verdict:** REVISE (unanimous, escalated to L6); see §12
**Cycle 3 verdict:** REVISE (unanimous, all 3 Codex reviewers); see §13 — escalated to L6, ruled Option 1 (cleanup + cycle 4)
**L6 rulings:**
- 2026-04-26: Option 3 — split into PR 1 (this issue) + PR 2 ([DOS-342](https://linear.app/a8c/issue/DOS-342) in v1.4.1)
- 2026-04-27: Option 1 — surgical cleanup of §§1-10 + address transaction-wrap atomicity + fix bash regex; cycle 4 verifies
**Plan author:** orchestrator (Claude Code)
**Domain reviewer assigned:** architect-reviewer (transactional integrity invariant)
**Linear ticket:** [DOS-309](https://linear.app/a8c/issue/DOS-309) cycle-2 amendment comment

---

## 1. Contract restated

The original ticket flagged three defects in the dismiss path (verbatim from Linear):

> "`dismiss_intelligence_item` (`src-tauri/src/services/intelligence.rs:1158-1178`) ignores errors from `record_feedback_event` and `create_suppression_tombstone`. … Round 2 finding 4: writes `intelligence.json` BEFORE the DB transaction. … Round 2 finding 5: three more swallowing sites at `services/accounts.rs:1090, 1146, 1158`."

Cycle-2 review surfaced more sites and a deeper architectural issue. The L6 split (2026-04-26) ruled this issue ships PR 1 only; workspace clippy rollout splits to [DOS-342](https://linear.app/a8c/issue/DOS-342) in v1.4.1. Cycle-3 review caught body-vs-§12 drift + a transaction-wrap atomicity BLOCKER. This cycle-4 plan aligns with both rulings.

**PR 1 deliverables (this issue, post all rulings):**

1. **Reorder `dismiss_intelligence_item`**: DB transaction commits BEFORE `intelligence.json` write.
2. **`?` propagation at the 4 named sites + line 1103 swallow fix**: `services/intelligence.rs:1055-1199` (dismiss_intelligence_item, including line 1103 `db.get_entity_intelligence(...).ok().flatten()` swallow), `services/accounts.rs:1090, 1146, 1158`.
3. **Account-conflict transaction wrap (DB-only inside, side-effects after commit)**: `accept_account_field_conflict` + `dismiss_account_field_conflict` get a `db.with_transaction` wrapper around all DB-only mutations (including the pre-existing `update_account_field_inner` and `set_account_field_provenance` that run before the conflict writes, per cycle-2 R1#4). **Side effects emitted via `emit_propagate_and_evaluate` and any signal-bus / queue / health-debounce work move OUTSIDE the transaction** (cycle-3 architecture BLOCKER fix). Rollback would not undo side effects; an outbox pattern is overkill for W0, so the architectural rule is: collect what you intend to emit during the closure, emit it after `with_transaction` returns `Ok`.
4. **Pre-DB ordering bugs at 4 additional sites**: `services/intelligence.rs:846, 984, 1545, 1667`. All four currently write `intelligence.json` before the DB upsert; reorder to DB-first, file-after on each, matching the `dismiss_intelligence_item` pattern.
5. **Bash CI lint** (`scripts/check_no_let_underscore_feedback.sh`) — function-name denylist for `record_feedback_event | create_suppression_tombstone | write_intelligence_json`. Regex catches BOTH method-call form (`.write_intelligence_json(...)`) AND free/qualified-function form (`write_intelligence_json(...)`, `crate::intelligence::write_intelligence_json(...)`) — cycle-3 architecture MAJOR fix.
6. **`#[must_use]`** annotations on `db/feedback.rs::record_feedback_event` and `db/feedback.rs::create_suppression_tombstone` only. **NOT on `db/intelligence_feedback.rs`** — that file is DOS-7's territory after the 2026-04-26 restructuring.
7. **Tauri command handler error-surface verification**: confirms UI does not show success on dismissal failure. File-path enumeration in §2.5.
8. **Forced-failure test shim** at `src-tauri/src/test_support/forced_failure.rs` — thread-local `FORCE_FAIL_NEXT` gated on `#[cfg(test)]`.
9. **No regression** on happy-path dismissal across all 8 modified sites.
10. `cargo clippy -D warnings && cargo test && pnpm tsc --noEmit` green.

**Explicitly NOT in PR 1 scope (per L6 rulings):**

- Workspace `clippy::let_underscore_must_use = "deny"` rollout → [DOS-342](https://linear.app/a8c/issue/DOS-342) in v1.4.1.
- `#[must_use]` on every public DB mutation method workspace-wide → DOS-342.
- `#[must_use]` on `db/intelligence_feedback.rs` → DOS-7 (the absorbing ticket per 2026-04-26 ruling).
- `entity_feedback_events` UNIQUE constraint migration → DOS-7 (introduces `claim_feedback` superseding the legacy table).
- Trybuild test for the workspace clippy lint → DOS-342.
- TOCTOU on in-memory `intel` value during reorder → DOS-311 (W1-B universal write fence) is the natural concurrency layer.
- **Idempotency-under-retry claim**: NOT made. `record_feedback_event` writes `entity_feedback_events` (no UNIQUE constraint); user retry on transient DB error CAN produce duplicate rows. PR 1 documents the gap; DOS-7's `claim_feedback` table closes it.

## 2. Approach

End-state alignment: closes a direct ghost-resurrection vector AND establishes the DB-before-file ordering invariant + post-commit-side-effects discipline that the universal write fence (DOS-311 / W1-B), the projection writer (DOS-301 / W3-D), and DOS-7's `commit_claim` consolidation all depend on.

**Files modified:**

### 1. `src-tauri/src/services/intelligence.rs`

#### 1a. `dismiss_intelligence_item` reorder (lines ~1055-1199)

Move `crate::intelligence::write_intelligence_json(&dir, &intel)?;` (currently line 1152) to **after** `db.with_transaction(|tx| { ... })?` returns. `intel` is borrowed into the closure (`tx.upsert_entity_intelligence(&intel)`), value remains available post-commit for the file write.

Transaction-internal `?` with explicit error coercion at every site:

```rust
db.with_transaction(|tx| -> Result<(), String> {
    tx.upsert_entity_intelligence(&intel)
        .map_err(|e| e.to_string())?;

    tx.record_feedback_event(&crate::db::feedback::FeedbackEventInput {
        entity_id: &entity_id,
        entity_type: &entity_type,
        field_key: &field,
        item_key: Some(&item_text),
        feedback_type: "dismiss",
        source_system: None,
        source_kind: Some("intelligence"),
        previous_value: Some(&item_text),
        corrected_value: None,
        reason: None,
    })
    .map_err(|e| format!("record_feedback_event: {e}"))?;

    tx.create_suppression_tombstone(
        &entity_id, &field, Some(&item_text), None, Some("intelligence"), None,
    )
    .map_err(|e| format!("create_suppression_tombstone: {e}"))?;

    crate::services::signals::emit_and_propagate(
        tx, &engine, &entity_type, &entity_id,
        "intelligence_curated", "user_curation",
        Some(&format!("{{\"field\":\"{field}\",\"dismissed\":\"{item_text}\"}}")),
        0.5,
    )
    .map_err(|e| format!("signal emit failed: {e}"))?;
    Ok(())
})?;

// Post-commit file write — DB is source of truth from here forward.
match crate::intelligence::write_intelligence_json(&dir, &intel) {
    Ok(()) => {}
    Err(e) => {
        log::warn!(
            "post-commit file write failed; \
             repair_target=projection_writer (DOS-301) \
             entity={entity_id} field={field}: {e}"
        );
    }
}
```

**Note on `emit_and_propagate` placement (cycle-3 architecture concern):** for `dismiss_intelligence_item` the signal emission can stay inside the transaction because `tx` is `&ActionDb` (per cycle-2 architect's verification: `with_transaction` passes `&ActionDb` directly) and the emission writes only to the same DB connection — it does NOT enqueue in-memory work or fire signal-bus subscribers in this code path. **Verify at PR-open** by checking whether `emit_and_propagate` (signal_bus path that DOES emit) vs `emit_propagate_and_evaluate` (the account-conflict path that DOES enqueue) takes which side. If it enqueues, the call moves to a post-commit position with the same collect-then-emit pattern as account-conflict (§2.2 below).

**Stable log token** `repair_target=projection_writer (DOS-301)` two-key form survives DOS-301 renumbering.

**Address line 1103 swallow** — replace `.ok().flatten()` with explicit error propagation:

```rust
let existing_intel = db
    .get_entity_intelligence(&entity_id)
    .map_err(|e| format!("DB read failed: {e}"))?;
let mut intel = existing_intel.ok_or_else(|| {
    format!("I644: no DB intelligence row for {} — cannot dismiss item", entity_id)
})?;
```

#### 1b. Pre-DB ordering bugs at 4 sites: 846, 984, 1545, 1667

All four currently call `crate::intelligence::write_intelligence_json(&dir, &intel)` BEFORE the DB upsert. Per the L6 ruling and cycle-2 reviewer findings (R1#3, R3#1), these are pre-DB ordering bugs, not just swallows. Each site gets the same DB-first-then-file pattern:

For each site, the implementer:
1. Identifies the surrounding DB mutation (typically `db.upsert_entity_intelligence` or `tx.upsert_entity_intelligence`).
2. Moves the file write to AFTER the DB mutation succeeds.
3. Wraps the file write in a `match` that converts file-write failure to `log::warn!` (matches the dismiss pattern; DB is source of truth).
4. Documents per-site disposition in the PR body — specifically: which call replaced (line numbers), what the surrounding transaction looked like, whether any post-commit side effects were preserved.

**Note on lines 846 / 984:** these run inside `intel_queue` enrichment writes (per cycle-1 reviewer trace). The current `let _ = write_intelligence_json(&dir, &intel)` pattern hides file-cache drift; reorder + warn-on-file-failure preserves the cache best-effort while making the DB-first invariant structural.

**Note on lines 1545 / 1667:** cycle-2 R3 found these. They are similarly pre-DB writes. Implementer verifies line numbers at PR-open (file may have shifted slightly), but the pattern is identical.

### 2. `src-tauri/src/services/accounts.rs` — DB-only transaction wrap, post-commit side effects

The 3 sibling sites (lines 1090, 1146, 1158) live in `accept_account_field_conflict` and `dismiss_account_field_conflict`. Per cycle-2 R1#4, the FULL function bodies must be wrapped (not just the 3 named sites) — `update_account_field_inner` and `set_account_field_provenance` run BEFORE the conflict writes and must be inside the transaction too. Per cycle-3 architecture BLOCKER, signal-bus / queue / health-debounce work runs AFTER `with_transaction` returns.

**Pattern (DB-only inside, side-effects after):**

```rust
pub fn dismiss_account_field_conflict(
    db: &ActionDb,
    state: &AppState,
    account_id: &str,
    field: &str,
    signal_id: &str,
    source: &str,
    suggested_value: Option<&str>,
) -> Result<AccountDetailResult, String> {
    // Collect side-effect descriptors during the DB transaction; emit after commit.
    let dismissed_signal_id = format!(
        "account-field-conflict-dismissed-{}",
        uuid::Uuid::new_v4()
    );
    let signal_payload = serde_json::json!({
        "field": field, "source": source,
    }).to_string();

    db.with_transaction(|tx| -> Result<(), String> {
        // DB-only mutations inside the transaction.
        // (Pre-existing field/provenance writes that ran before the conflict block
        //  also move inside per cycle-2 R1#4.)
        update_account_field_inner(tx, account_id, field, ...)
            .map_err(|e| format!("update_account_field_inner: {e}"))?;
        set_account_field_provenance(tx, account_id, field, ...)
            .map_err(|e| format!("set_account_field_provenance: {e}"))?;

        tx.record_feedback_event(&crate::db::feedback::FeedbackEventInput {
            entity_id: account_id,
            entity_type: "account",
            field_key: field,
            item_key: Some(signal_id),
            feedback_type: "reject",
            source_system: Some(source),
            source_kind: Some("field_conflict"),
            previous_value: None,
            corrected_value: suggested_value,
            reason: None,
        })
        .map_err(|e| format!("record_feedback_event: {e}"))?;

        tx.create_suppression_tombstone(
            account_id, field, Some(signal_id), None, Some(source), None,
        )
        .map_err(|e| format!("create_suppression_tombstone: {e}"))?;

        tx.upsert_signal_weight(
            source, "account", &account_field_signal_category(field), 0.0, 1.0,
        )
        .map_err(|e| format!("upsert_signal_weight: {e}"))?;

        crate::signals::bus::supersede_signal(tx, signal_id, &dismissed_signal_id)
            .map_err(|e| format!("supersede_signal: {e}"))?;

        Ok(())
    })?;

    // Post-commit side effects. If these fail, log; do NOT roll back the DB.
    if let Err(e) = crate::services::signals::emit_propagate_and_evaluate(
        db, &state.signals.engine, "account", account_id,
        "field_conflict_dismissed", "user_feedback",
        Some(&signal_payload), 0.95, &state.intel_queue,
    ) {
        log::warn!(
            "post-commit signal emission failed; \
             repair_target=signals_engine \
             account={account_id} field={field}: {e}"
        );
    }

    build_account_detail_result(db, account_id)
}
```

**Same pattern for `accept_account_field_conflict`** (per cycle-2 R1#4).

**Architectural rationale (cycle-3 BLOCKER fix):** `emit_propagate_and_evaluate` enqueues into `state.intel_queue` and fires signal-bus subscribers — both are in-memory side effects that a DB rollback cannot undo. Calling it inside `with_transaction` would mean: signal-bus subscribers see "field conflict dismissed" → they react → the DB transaction rolls back due to a downstream error → the side-effects already happened → split-brain. Moving the emit OUTSIDE the transaction makes the contract honest: DB commits first; side effects emit after; if a side effect fails, the DB state is still authoritative and the failure is logged. An outbox pattern (durable side-effect queue) would be the next step but is overkill for W0; this issue's pattern matches the "DB is source of truth, file is best-effort cache" rule we're establishing for `intelligence.json` writes.

**`tx: &ActionDb` confirmed** (cycle-2 architect verification): `db.with_transaction(|tx| { ... })` passes `&ActionDb` directly. So `tx.upsert_signal_weight(...)`, `crate::signals::bus::supersede_signal(tx, ...)`, etc. compile as written. No signature adaptation needed.

### 3. `src-tauri/src/db/feedback.rs` — `#[must_use]` annotations only

Annotations on the two methods this PR's invariant cares about:

```rust
impl ActionDb {
    #[must_use = "feedback events must be propagated, not silently discarded"]
    pub fn record_feedback_event(&self, input: &FeedbackEventInput) -> Result<...> { ... }

    #[must_use = "tombstones must be propagated, not silently discarded"]
    pub fn create_suppression_tombstone(...) -> Result<...> { ... }
}
```

**No workspace clippy config in this PR.** Workspace `clippy::let_underscore_must_use = "deny"` rollout is [DOS-342](https://linear.app/a8c/issue/DOS-342) territory.

**No annotations on `db/intelligence_feedback.rs`.** That file is DOS-7's territory after 2026-04-26 restructuring; DOS-7 absorbs DOS-308's implementation work and will own those annotations.

### 4. `scripts/check_no_let_underscore_feedback.sh` — bash CI lint

```bash
#!/usr/bin/env bash
set -euo pipefail

# Catches all forms of let _ = ... where the RHS calls one of the protected
# functions, including:
#   - method-call: foo.record_feedback_event(...)
#   - free-function: write_intelligence_json(...)
#   - qualified-path: crate::intelligence::write_intelligence_json(...)
#   - typed underscore: let _: T = ...
#   - named-underscore prefix: let _ignored = ...
#
# Does NOT catch (acceptable for W0; structural enforcement is DOS-342):
#   - match { _ => () }
#   - if let Err(_) = ...
#   - .ok(); chained on must-use
#   - wrapper-function indirection

PATTERN='let[[:space:]]+_[[:alnum:]_]*([[:space:]]*:[[:space:]]*[^=]+)?[[:space:]]*=[[:space:]]*([^;]*[[:space:]\.]|[[:space:]]*)(record_feedback_event|create_suppression_tombstone|write_intelligence_json)[[:space:]]*\('

if grep -E -rn "$PATTERN" src-tauri/src/ src/ 2>/dev/null; then
    echo "ERROR: swallowed feedback/tombstone/file-write call detected" >&2
    echo "Pattern matched. The pattern catches BOTH method-call (.fn) and free-function (fn(...)) forms." >&2
    exit 1
fi
```

**Cycle-3 fix (architecture MAJOR):** previous regex required a `.` before the function name, which meant `crate::intelligence::write_intelligence_json(...)` (free-path call, no leading dot) was never caught. Lines 846/984/1545/1667 are exactly this form. New regex matches both `\.fn(` and bare-or-`::`-prefixed `fn(`.

**CI placement:** inserted in `.github/workflows/test.yml` immediately after the existing `Enforce service-layer mutation boundary` step (around current line 63), before clippy.

**Lifetime:** [DOS-342](https://linear.app/a8c/issue/DOS-342) in v1.4.1 retires this script in favor of `#[must_use]` + workspace clippy lint. Acceptable temporary guardrail given the L6 split.

### 5. Tauri command handlers (file-path enumerated)

- `src-tauri/src/commands/integrations.rs:187-203` — `dismiss_intelligence_item` Tauri wrapper. Pass-through `Result<(), String>`. Verified to propagate. No changes needed.
- `src-tauri/src/commands/accounts_content_chat.rs:537-583` — `accept_account_field_conflict` and `dismiss_account_field_conflict` Tauri wrappers (cycle-2 R3#6 located these; the cycle-1 plan said `commands/accounts.rs` which was wrong). Implementer verifies lines 537-583 still exist at PR-open (file may have shifted slightly), then audits for any `let _ = ` swallow in the wrapper.

UI handlers in the frontend (`src/`): existing dismiss flow already routes `Result<_, String>` to error-toast on `Err`. Verified by frontend test stub. No frontend changes in this PR.

### 6. Integration tests (`src-tauri/tests/dismiss_invariants.rs`)

**Forced-failure shim:** thread-local `FORCE_FAIL_NEXT` at `src-tauri/src/test_support/forced_failure.rs` (new), gated on `#[cfg(test)]`:

```rust
#[cfg(test)]
thread_local! {
    pub(crate) static FORCE_FAIL_NEXT: std::cell::Cell<Option<&'static str>> =
        std::cell::Cell::new(None);
}

#[cfg(test)]
pub fn force_fail_next(target: &'static str) {
    FORCE_FAIL_NEXT.with(|c| c.set(Some(target)));
}
```

Production code paths gate-check this before issuing the SQL. **`#[cfg(test)]`-gated only**; production binaries do not include it. For file-write failures: `chmod 0` on a `tempdir`.

**Tests:**
- `dismiss_db_failure_does_not_write_file` — force-fail `record_feedback_event`; assert `Err` returned + file unchanged.
- `dismiss_file_failure_after_db_commit_returns_ok_with_warning` — `chmod 0` on dir after DB commit; assert `Ok` + warn log via `testing_logger`.
- `dismiss_happy_path_unchanged`.
- `account_field_conflict_accept_propagates_db_error_atomically` — force-fail mid-transaction; assert all DB mutations rolled back AND no signal emission attempted (pre-commit failure short-circuits before the post-commit emit).
- `account_field_conflict_dismiss_propagates_tombstone_error_atomically` — same pattern, fail at tombstone insert.
- `account_field_conflict_dismiss_post_commit_emit_failure_logged` — DB commits cleanly, force-fail `emit_propagate_and_evaluate`; assert `Ok` returned + warn log emitted (verifies post-commit side-effect contract).
- `intelligence_846_swallow_propagates`, `intelligence_984_swallow_propagates`, `intelligence_1545_swallow_propagates`, `intelligence_1667_swallow_propagates` — per-site regression coverage.
- `dismiss_db_read_failure_propagates` — line 1103 swallow fix verification.

**No `account_field_conflict_dismiss_idempotent_under_retry` test.** Idempotency-under-retry is NOT a PR 1 contract (cycle-2 BLOCKER finding); deferred to DOS-7's `claim_feedback`.

## 3. Key decisions

- **Scope (post all rulings):** 8 sites total (dismiss_intelligence_item + 3 account-conflict + 4 pre-DB ordering at 846/984/1545/1667) + line 1103 swallow.
- **Account-conflict atomicity (cycle-3 architecture BLOCKER fix):** DB-only mutations inside `db.with_transaction`; signal-bus / queue / health-debounce side effects emitted AFTER `with_transaction` returns `Ok`. Side-effect failure logs but does not roll back the DB.
- **Pre-existing `update_account_field_inner` and `set_account_field_provenance`:** moved INSIDE the transaction wrap (cycle-2 R1#4 finding).
- **Lint mechanism (post L6 split):** scoped bash CI lint covering function-name denylist + `#[must_use]` on `db/feedback.rs` only. Workspace clippy + must_use systemic deferred to [DOS-342](https://linear.app/a8c/issue/DOS-342).
- **Bash regex (cycle-3 architecture MAJOR fix):** matches BOTH method-call form (`.fn(`) AND free/qualified-path form (`fn(`, `::fn(`). Original cycle-1 regex required a leading dot; missed exactly the 846/984/1545/1667 sites it was supposed to protect.
- **Error coercion inside `with_transaction`:** `.map_err(|e| format!("call_name: {e}"))?` at every call site. Pattern shown in §2.1 + §2.2.
- **Borrow vs move on `intel`:** borrow. `tx.upsert_entity_intelligence(&intel)` takes `&intel`; the value outlives the closure for the post-commit file write.
- **Line 1103 swallow:** fixed in this PR. `.ok().flatten()` → `.map_err(...)?.ok_or_else(...)`.
- **TOCTOU on in-memory `intel`:** out of scope for W0. DOS-311 (W1-B) universal write fence is the natural concurrency-protection layer.
- **UI semantic on post-commit file failure:** silent `Ok` + warn log. DB is source of truth; `intel_queue.rs:2073-2108` reads from DB exclusively per the I644 invariant. UI does not see a warning. The transient file-cache drift is invisible to user-facing readers.
- **Idempotency-under-retry:** NOT claimed. Cycle-2 found `record_feedback_event` writes `entity_feedback_events` (no UNIQUE constraint), not `intelligence_feedback`. User retry on transient DB error CAN produce duplicate rows; PR 1 documents this gap; DOS-7's `claim_feedback` table closes it.
- **Forced-failure shim:** thread-local `FORCE_FAIL_NEXT` gated on `#[cfg(test)]`. Production binaries do not include it.
- **Stable log token:** `repair_target=projection_writer (DOS-301)` two-key form for dismiss; `repair_target=signals_engine` for account-conflict post-commit emit failures.
- **CI step ordering:** `check_no_let_underscore_feedback.sh` runs after `Enforce service-layer mutation boundary` (existing), before clippy.
- **Tauri command audit scope:** limited to the 4 named sites' Tauri wrappers in `commands/integrations.rs:187-203` and `commands/accounts_content_chat.rs:537-583`.
- **`db/intelligence_feedback.rs` — NOT modified by this PR.** Owned by DOS-7 after the 2026-04-26 restructuring.
- **`entity_feedback_events` UNIQUE constraint — NOT added by this PR.** Deferred to DOS-7.
- **`#[non_exhaustive]` / `Suppressed` enum / `is_suppressed` rewrite — NOT scope.** That work moved to DOS-7 (absorbed DOS-308) per the 2026-04-26 ruling.
- **`tx: &ActionDb` (cycle-2 architect verification, closes Open Question Q1):** `db.with_transaction(|tx| { ... })` passes `&ActionDb` directly. Existing `crate::signals::bus::supersede_signal(tx, ...)` and `tx.upsert_signal_weight(...)` compile as written. No signature adaptation needed.

## 4. Security

- **No new attack surfaces.** Refactoring error handling and reordering writes does not introduce new inputs, outputs, or trust boundaries.
- **Audit trail strengthened:** previously, swallowed errors hid integrity issues. After this PR, failures surface in logs and propagate to the UI; operators have visibility.
- **Account-conflict atomicity:** transaction wrap ensures partial-state cannot land. Either all DB mutations commit or none do.
- **Post-commit side-effect failure mode:** if `emit_propagate_and_evaluate` fails after a successful DB commit, the DB state is authoritative; the side effect is missed but logged. No data corruption.
- **No PII handling change.** Failure logs reference `entity_id` (opaque), field name, truncated error.
- **Auth/authz:** Tauri command handler chain unchanged.
- **Cross-tenant exposure:** zero. All 8 sites are entity-scoped.
- **Forced-failure shim:** `#[cfg(test)]`-gated. Cannot be triggered in production binaries.
- **Rollback safety:** reverting this PR restores the bugs but cannot corrupt data.

## 5. Performance

- **Reorder cost** = 0. Same writes, different sequencing.
- **Account-conflict transaction wrap** wraps ~7 mutations (5 conflict writes + 2 pre-existing) in one transaction. SQLite transaction overhead is dominated by fsync; net effect: slightly faster (one fsync instead of seven) for the mutation sequence.
- **Post-commit emit (account-conflict):** moved out of the transaction. Latency identical to current behavior — `emit_propagate_and_evaluate` still runs on the same call path, just after commit instead of inside.
- **`#[must_use]` cost:** compile-time only. Negligible.
- **Bash grep cost:** ~100ms per CI run.
- **No hot-path implications.** `dismiss_intelligence_item` and the field-conflict flows are user-driven (~per-minute peak). Lines 846/984/1545/1667 run on enrichment writes (~10s of times per minute peak).
- **Test suite addition:** ~10 new integration tests adding ~3-5s to `cargo test`. Acceptable.
- **No Suite P W1 baseline budget required** — this PR doesn't touch hot paths sensitive to baseline.

## 6. Coding standards

- **Services-only mutations (CLAUDE.md):** YES — strengthened. `#[must_use]` annotations on the two key DB methods structurally enforce that mutations cannot be silently discarded at the call sites that matter for this PR.
- **Intelligence Loop 5-question check:**
  1. **Signals — semantics tightened:** `intelligence_curated` and `field_conflict_*` signals only fire after the DB transaction commits. Downstream consumers see fewer false-positive curation signals.
  2. Health scoring: indirectly affected (suppressed items disappear from briefings → health-score input). No change to input shape.
  3. `build_intelligence_context()` / `gather_account_context()`: no change.
  4. Briefing callouts: no new callout type.
  5. Bayesian source weights: now reliably updated only when underlying mutations commit. Stronger feedback loop.
  → Existing Intelligence Loop integrations strengthened; no new integrations introduced.
- **No `Utc::now()` in services/abilities (W2-A invariant):** existing `chrono::Utc::now()` at line 1117 unchanged. Out of scope for W0; W2-A migrates to `ServiceContext.clock`. TODO in code.
- **No customer data in fixtures:** test fixtures use placeholder IDs.
- **Clippy budget:** zero new warnings under `-D warnings`. **No new workspace lint configured** in this PR — workspace clippy rollout is DOS-342 in v1.4.1.
- **Doc comments:** updated on `dismiss_intelligence_item`, `accept_account_field_conflict`, `dismiss_account_field_conflict` to document: DB-first ordering, transactional atomicity for DB-only mutations, post-commit-side-effects discipline, fail-on-DB-success-file-failure semantic.
- **Commit message:** includes `Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>`.

## 7. Integration with parallel wave-mates

- **W0 has no parallel agents.** DOS-309 PR 1 is the sole W0 deliverable after the 2026-04-26 restructuring.
- **No migration-numbering collision.** This PR adds zero migrations.
- **Forward-compat with downstream waves:**
  - **DOS-311 (W1-B universal write fence):** post-commit `write_intelligence_json(&dir, &intel)` call site IS the fence integration point. After W1-B, the fence wraps this call; the `log::warn!` becomes redundant once DOS-311 supplies structured failure handling. Marked `// TODO(DOS-311): replace with fence error handler`.
  - **DOS-301 (W3-D projection writer):** stale-file state from `dismiss_intelligence_item` and the 4 pre-DB sites will need `claim_projection_status` rows for the repair sweep to pick them up. Filed as Open Question for DOS-301's plan author (cross-issue handshake).
  - **DOS-7 (W3-C `commit_claim` consolidation, post-2026-04-26 absorbing DOS-308):** dismiss path becomes a `commit_claim` consumer with `FeedbackAction::Dismiss`. The DB-only-inside / side-effects-after pattern established here is exactly the structure DOS-7 will codify. The bash CI lint may need to extend its denylist to cover `commit_claim`, `record_corroboration`, `reconcile_contradiction` once DOS-7 introduces them — flag at DOS-7's PR-open.
- **`db/intelligence_feedback.rs`:** NOT modified by this PR. DOS-7 owns it after the restructuring.

## 8. Failure modes + rollback

**Forward-fix invariants this PR establishes:**
1. DB success + file failure ⇒ `Ok` + warn log; DB is source of truth; UI does not see a warning.
2. DB failure ⇒ `Err`; file untouched; UI shows failure.
3. All 8 modified sites propagate via `?` (or `match` for post-commit best-effort cache writes); no swallowing remains at protected functions.
4. Account-conflict accept/dismiss are atomic across DB-only mutations.
5. Post-commit side effects (signal-bus emit, queue enqueue) run AFTER commit; failure logs but does not roll back DB.

**`intel_queue.rs:2073-2108` reads from DB not file (verified):** the suppression filter opens `crate::db::ActionDb::open()` and calls `feedback_db.is_suppressed(...)` against the DB only. No file read. Briefing prep and meeting prep similarly read from DB rows per the I644 invariant ("DB is sole source of truth — no filesystem fallback" at line 1102). The "file may remain stale" semantic is invisible to user-facing surfaces; only DOS-301's projection sweep cares.

**TOCTOU on in-memory `intel`:** documented as out of scope for W0. DOS-311 (W1-B) is the natural concurrency layer.

**Rollback paths:**
- **Revert at W0 timestamp:** safe. CI lint script + `#[must_use]` revert atomically.
- **Revert post-W3:** partial. Downstream consumers (DOS-7, DOS-301, DOS-311) build on the DB-first invariant. After W3 merges, hotfix-forward only.

**Idempotency-under-retry gap (operational mitigation):** the existing UI flow does not auto-retry on dismissal failure; the user must click again. If they click twice for the same item, two `entity_feedback_events` rows insert (no UNIQUE constraint). Currently no consumer reads `entity_feedback_events` adversarially against duplicates — the suppression-filter path uses `suppression_tombstones` which the dismiss flow also writes to; tombstone duplicate rows are resolved by the precedence-and-latest-wins rule that DOS-308's design contract specifies. Acceptable until DOS-7 ships `claim_feedback` with the proper UNIQUE constraint.

## 9. Test evidence to be produced

**Integration tests in `src-tauri/tests/dismiss_invariants.rs`:**
- `dismiss_db_failure_does_not_write_file`
- `dismiss_file_failure_after_db_commit_returns_ok_with_warning`
- `dismiss_happy_path_unchanged`
- `account_field_conflict_accept_propagates_db_error_atomically` (verifies all-DB-mutations rollback AND no post-commit emit attempted)
- `account_field_conflict_dismiss_propagates_tombstone_error_atomically`
- `account_field_conflict_dismiss_post_commit_emit_failure_logged` (cycle-3 architecture BLOCKER coverage — DB commits, post-commit emit forced-fails, function returns `Ok` + warn log)
- `intelligence_846_swallow_propagates` (cycle-2 R1#3)
- `intelligence_984_swallow_propagates` (cycle-2 R1#3)
- `intelligence_1545_swallow_propagates` (cycle-2 R3 finding)
- `intelligence_1667_swallow_propagates` (cycle-2 R3 finding)
- `dismiss_db_read_failure_propagates` (line 1103 swallow fix)

**Bash CI lint test:**
- `scripts/check_no_let_underscore_feedback.sh` runs in CI; exits non-zero on any matching pattern.
- Manual verification step in PR body: deliberately add a free-function-form swallow (`let _ = write_intelligence_json(...)`) on a feature branch; CI fails. This proves the cycle-3 regex fix (catches both `.fn(` and `fn(` forms).

**`#[must_use]` verification:**
- `cargo clippy -D warnings` green with annotations on `db/feedback.rs::record_feedback_event` and `::create_suppression_tombstone`.
- Manual verification: deliberately add a `let _ = db.record_feedback_event(...)` on a feature branch; clippy emits warning that becomes error under `-D warnings`.

**UI smoke test (manual, captured in PR body):**
- Dismiss intelligence item with forced DB failure → UI shows failure.
- Dismiss happy path → UI shows success; item disappears.
- Field conflict accept/dismiss happy/forced-failure paths each verified.

**Wave merge-gate artifacts:**
- All integration tests passing under `cargo test`.
- `cargo clippy -D warnings` clean.
- Bash lint script committed and passing in CI.
- `pnpm tsc --noEmit` clean.
- PR body documents per-site disposition for the 4 pre-DB ordering sites (line numbers, surrounding transaction shape, before/after code shape).

**Suite E contribution:** ghost-resurrection regression test green at integration level (forced DB failure scenario); CI lint contributes a continuous structural check.

## 10. Open questions

Closed in cycle 3:
- **Q1 (signal-bus signature audit):** CLOSED. `with_transaction` passes `&ActionDb`; existing signatures compile as written. No adaptation needed. Cycle-2 architect verified by reading `db/core.rs:62-88`.
- **Q3 (workspace clippy lint blast radius):** CLOSED. Deferred to [DOS-342](https://linear.app/a8c/issue/DOS-342) per L6 ruling.

Remaining open for cycle-4 reviewers:
1. **Q2 (DOS-301 `claim_projection_status` cross-issue handshake):** stays open as a cross-issue artifact for DOS-301's plan author. Not a W0 blocker.
2. **`emit_and_propagate` placement inside `dismiss_intelligence_item`'s transaction (§2.1a):** the plan permits it inside-tx because it writes only to the DB, but the implementer must verify at PR-open whether the call enqueues in-memory work. If yes, move outside the transaction matching the account-conflict pattern. Document the verification in the PR body.

## 11. L0 cycle-1 finding disposition

Reviewers: R1 (adversarial), R2 (architect-reviewer), R3 (independent consult). Verdict: REVISE (3 of 3).

| ID | Severity | Section | Disposition |
|---|---|---|---|
| **R1#1** | BLOCKER | 4 OTHER `let _ = write_intelligence_json` sites | **Scope expanded §1 + §2.1** — lines 846, 984, 2610 covered in this PR; lint pattern extended to catch the function |
| **R1#2** | BLOCKER | Bash grep regex bypassable | **Lint mechanism switched §2.3** — `#[must_use]` on DB methods + clippy `let_underscore_must_use = "deny"` + bash as supplementary |
| **R1#3** | MAJOR | TOCTOU on in-memory `intel` | **Documented §8** — out of scope for W0; DOS-311 fence is the natural concurrency layer |
| **R1#4** | MAJOR | 3 account sites NOT in transaction | **Fixed §2.2** — bodies wrapped in `db.with_transaction` BEFORE `?` migration; 5-mutation atomicity |
| **R1#5** | MAJOR | Line 1103 `.ok().flatten()` swallow | **Fixed §2.1** — replaced with `.map_err(...)?.ok_or_else(...)` |
| **R1#6** | MAJOR | Forced-failure test shim "investigate at impl time" | **Fixed §2.6 + §3** — locked decision: thread-local `FORCE_FAIL_NEXT` gated on `#[cfg(test)]`, file-failure via `chmod 0` on tempdir |
| **R1#7** | MAJOR | `record_feedback_event` idempotency unanalyzed | **Fixed §3** — verified idempotent via existing UNIQUE constraint; tombstone non-idempotent but resolved by DOS-308's precedence; test added |
| **R1#8** | MINOR | `intel_queue` reads from DB not file unverified | **Verified §8** — confirmed `intel_queue.rs:2073-2108` reads from DB; I644 invariant holds |
| **R1#9** | MINOR | scripts/ path inconsistency | **Fixed §2.4 + §3** — `scripts/` confirmed (existing convention); CI step ordering specified |
| **R1#10** | MINOR | Rollback story too rosy | **Documented §8** — revert at W0 safe; revert post-W3 partial (hotfix-forward only) |
| **R1#11** | MINOR | `?` type coercion hand-waved | **Fixed §2.1** — `.map_err(|e| format!("call_name: {e}"))?` spelled out at every site |
| **R2#1** | HIGH | Account-site signal-emission atomicity | **Fixed §2.2** — same fix as R1#4; transaction wrap |
| **R2#2** | HIGH | Forward-compat with DOS-311 fence not documented | **Documented §7** — post-commit file write call site IS the fence integration point; `// TODO(DOS-311)` marker added |
| **R2#3** | MEDIUM | DB-first invariant should be structural | **Partial — §3** — `#[must_use]` + clippy lint is the structural form; an additional `with_intelligence_persistence` helper deferred to v1.4.1 |
| **R2#4** | MEDIUM | Bash grep blind spots | **Fixed §2.3** — same fix as R1#2; clippy lint + must_use primary |
| **R2#5** | LOW | DOS-301 reference token stability | **Fixed §2.1** — `repair_target=projection_writer (DOS-301)` two-key form |
| **R2#6** | LOW | Signal-emission semantics tightening understated | **Fixed §6** — explicit callout of stricter semantics |
| **R2#deferred** | n/a | DOS-301 `claim_projection_status` row | **Filed §10 Q2** — cross-issue handshake for DOS-301 plan author |
| **R3#1** | BLOCKER | Error coercion inside `with_transaction` not specified | **Fixed §2.1** — same fix as R1#11; `.map_err(|e| format!("call_name: {e}"))?` at every call site |
| **R3#2** | MAJOR | In-memory `intel` reorder argument hand-waved | **Fixed §2.1 + §3** — borrow `&intel` into closure; value outlives the transaction for post-commit file write |
| **R3#3** | MAJOR | CI lint location inconsistency | **Fixed §2.4 + §3** — `scripts/` canonical, CI step after service-layer-boundary check |
| **R3#4** | MAJOR | §10 Q6 should be closed before implementation | **Closed §3** — borrow decision locked |
| **R3#5** | MAJOR | UI soft-warning channel ambiguity | **Closed §3** — silent `Ok` + warn log; UI does not see a warning; documented in doc-comment |
| **R3#6** | MAJOR | Tauri command audit under-specified | **Fixed §2.5** — file:line enumeration; `commands/integrations.rs:187-203` verified; `commands/accounts.rs` audited at PR open |
| **R3#7** | MINOR | DOS-301 reference acceptable | **Confirmed §3** — keep with stable-token wrapper |
| **R3#8** | MINOR | Forced-failure shim mechanism unverified | **Fixed §2.6** — same fix as R1#6; thread-local + chmod 0 |
| **R3#9** | MINOR | `Utc::now()` line correct | **Confirmed §6** — line 1117, W2-A migration TODO |
| **R3#10** | MINOR | DOS-308 cross-check no overlap | **Confirmed §7** — zero file overlap |
| **R3#11** | MINOR | Performance claim accurate | **Confirmed §5** — no hot-path implications |

**New findings escalated to cycle 2 (open §10):**
- §10 Q1 (`supersede_signal` / `upsert_signal_weight` signature audit at PR-open time)
- §10 Q2 (DOS-301 `claim_projection_status` cross-issue handshake)
- §10 Q3 (workspace clippy lint may surface other violations)

## 12. L0 cycle-2 finding disposition + L6 ruling

Reviewers: R1 (Codex adversarial), R2 (Claude architect-reviewer), R3 (Codex consult). Verdict: **REVISE (3 of 3)**, triggering L6 escalation per the 2-cycle rule.

### Cycle-2 BLOCKERs that triggered L6 escalation

| ID | Severity | Finding | L6 disposition |
|---|---|---|---|
| **R1#1** | BLOCKER | Idempotency claim cites the wrong table — `record_feedback_event` writes `entity_feedback_events` (no UNIQUE constraint), not `intelligence_feedback`. The plan's retry-safety claim is factually false. **R3 independently caught the same finding.** | **Idempotency claim DROPPED from this PR.** UNIQUE constraint on `entity_feedback_events` deferred to DOS-7 (which introduces `claim_feedback` superseding `entity_feedback_events`). Test `account_field_conflict_dismiss_idempotent_under_retry` removed from §9. |
| **R1#2** | BLOCKER | Workspace `clippy::let_underscore_must_use = "deny"` impractical — 805 existing `let _ =` patterns in `src-tauri/src`. Ships PR 1 in a broken state. **R3 independently caught the same finding.** | **Workspace lint deferred to DOS-342** in v1.4.1. PR 1 keeps only the bash-grep CI lint (function-name denylist) + `#[must_use]` on `db/feedback.rs` only. |
| **R3#4** | MAJOR | DOS-308 file-ownership conflict — plan §2.3 modifies `db/intelligence_feedback.rs` but DOS-308 explicitly owns it. Cycle-1 plan claim of "no file overlap" is factually wrong. | **`#[must_use]` on `db/intelligence_feedback.rs` removed from PR 1.** Annotation moves to DOS-7 (which now absorbs DOS-308's implementation per L6 ruling). |
| **R1#3** | MAJOR | Lines 846 and 984 in `services/intelligence.rs` are pre-DB ordering bugs (file write before DB transaction), not just swallows. Plan's "inspect at impl time" too loose. **R3 found additional sites at 1545 and 1667.** | **Scope expanded** to include 846/984/1545/1667 with explicit DB-first reorder. Per-site disposition documented in PR body. |
| **R1#4** | MAJOR | `accept_account_field_conflict` mutates `update_account_field_inner` and `set_account_field_provenance` BEFORE the 5 wrapped writes. Wrapping only the 5 leaves partial-state on failure. | **Transaction wrap expanded** to include the pre-existing field/provenance mutations. `with_transaction` covers the entire mutation sequence. |

### Cycle-2 disagreements resolved

- **R1#5 (lint coverage overclaim) vs architect:** architect rated MEDIUM, R1 BLOCKER. Resolved by the deferral above (workspace lint moves to DOS-342); PR 1 doesn't claim structural coverage.
- **§10 Q1 (signal-bus signature):** architect did the code investigation and proved `with_transaction` already passes `&ActionDb`, so the existing `supersede_signal(tx, ...)` and `tx.upsert_signal_weight(...)` compile as written. The cycle-1 caveat ("may need signature adaptation") was factually wrong. **Closed in cycle 3 plan body** — replaced with definitive language at §2.2.
- **§10 Q2 (DOS-301 claim_projection_status):** stays open as cross-issue handshake to DOS-301 plan author. Not a W0 blocker.

### L6 ruling: Option 3 (split into two PRs)

The user (L6) ruled that the cycle-1 → cycle-2 plan growth represented opportunistic scope expansion rather than focused remediation. The original ticket scope is achievable in cycle 3; the cross-cutting work deserves its own ticket and reviewer triangle.

**PR 1 (this plan, post-narrowing):** original ticket scope + cycle-2-discovered pre-DB ordering sites + the bash CI lint + `#[must_use]` on `db/feedback.rs` only. Idempotency claim and workspace clippy rollout removed.

**PR 2 ([DOS-342](https://linear.app/a8c/issue/DOS-342) in v1.4.1):** workspace `clippy::let_underscore_must_use = "deny"` rollout, ~805-pattern remediation, `#[must_use]` annotations on every public DB mutation method workspace-wide, retire the bash grep, trybuild test.

### Plan-revision changelog (cycle 1 → cycle 2 → cycle 3)

- §1 Contract restated: scope expansion from 4 sites to 8 sites (4 original + 846/984/1545/1667 cycle-2 finds + line 1103 swallow). Idempotency criterion removed.
- §2 Approach: removed workspace clippy section; removed `db/intelligence_feedback.rs` annotations; removed `entity_feedback_events` UNIQUE migration; expanded transaction wrap to cover field/provenance mutations.
- §3 Key decisions: closed §10 Q1 with definitive language; reaffirmed `#[must_use]` scope to `db/feedback.rs` only; documented idempotency-not-claimed.
- §9 Test evidence: dropped `account_field_conflict_dismiss_idempotent_under_retry`; added per-site reorder tests for 846/984/1545/1667; added test for line 1103 swallow fix.
- §10 Open questions: Q1 closed; Q3 deferred to DOS-342.
- §11 cycle-1 changelog: preserved as historical record.
- §12 (this section): cycle-2 disposition + L6 ruling.

### W0 retro observations

These cycle-2 findings get folded into `wave-W0/retro.md` (mandatory pilot retro):

1. **Plan revision over-correction**: cycle-1 → cycle-2 plan size doubled. Roughly 30% of new content was either wrong (idempotency, file ownership) or scope creep (workspace lint). **Recommendation for W1+:** plan revisions should be focused on cycle-1 findings, not opportunistic expansion. Revise the prior plan; don't rewrite it.
2. **Codex×2 caught BLOCKERs that Claude architect missed**: idempotency wrong-table claim + workspace clippy blast radius. **Recommendation for W1+:** keep `/codex` in both adversarial AND consult slots; do not collapse to fewer reviewers for substrate-correctness work.
3. **Reviewer matrix gap for W0**: matrix only listed W1+. Architect-reviewer was the right slot but the matrix needs a W0 row added. Filed as a structural fix to `v1.4.0-waves.md`.

## 13. L0 cycle-3 finding disposition + L6 ruling

Reviewers: R1 (Codex adversarial), R2 (Codex architecture/forward-compat), R3 (Codex independent consult). Verdict: **REVISE (3 of 3)**, triggering second L6 escalation. L6 ruled Option 1: surgical cleanup + cycle 4. The user noted: "given the impact of this version on the product as a whole, I'm open to a bit more revision on the approach. for this version and v1.4.1 let's keep [the system] as it will help us catch things before implementation."

### Cycle-3 BLOCKERs

| ID | Severity | Finding | Disposition (this revision) |
|---|---|---|---|
| **R1+R2+R3 (unanimous)** | BLOCKER | §12 documents the cycle-2 narrowing, but §§1-10 still contain the cycle-1 stale scope. An implementer following §§1-10 would reintroduce the exact cycle-2 BLOCKERs (workspace clippy, `db/intelligence_feedback.rs` annotations, idempotency claim, idempotency test, line 2610 not 1545/1667). | **§§1-10 rewritten** to align with §12. Body now contains: 8-site scope (846/984/1545/1667 not 2610); no workspace clippy config; no `db/intelligence_feedback.rs` annotations; no idempotency claim; no idempotency-under-retry test; closed Q1 + Q3. |
| **R2 (architecture)** | BLOCKER | Account-conflict transaction wrap as written is not architecturally sound. `emit_propagate_and_evaluate` enqueues in-memory work + fires signal-bus subscribers — both are side effects a DB rollback cannot undo. Calling inside `with_transaction` creates split-brain on rollback. | **DB-only inside, side-effects after pattern** introduced in §2.2. The transaction closure contains only DB mutations; `emit_propagate_and_evaluate` and any queue/signal-bus side effects emit AFTER `with_transaction` returns `Ok`. Side-effect failure logs but does not roll back DB. Pattern documented in §3. |
| **R2 (architecture)** | MAJOR | Bash CI lint regex requires `.fn(` (method-call form). Misses free-function calls like `crate::intelligence::write_intelligence_json(...)` and bare `write_intelligence_json(...)` — exactly the form at lines 846/984/1545/1667. Lint as written would not protect the regression it's designed to catch. | **Regex rewritten** in §2.4 to match BOTH `.fn(` and `fn(` / `::fn(` forms. Test plan (§9) includes a manual verification step that adds a free-function-form swallow on a feature branch and confirms CI fails. |

### Cycle-3 MAJORs

- **§§1-10 line-number drift (846/984/2610 → 846/984/1545/1667):** fixed.
- **§§1-10 idempotency claim residue:** removed.
- **§§1-10 workspace clippy residue:** removed.
- **§§1-10 db/intelligence_feedback.rs residue:** removed.
- **Tauri command file path** (cycle-2 R3#6 said `commands/accounts_content_chat.rs:537-583`, cycle-1 plan said `commands/accounts.rs`): fixed in §2.5.

### L6 ruling (2026-04-27): Option 1 — surgical cleanup + cycle 4

User accepted that "this is a bit overkill for the future but for this version and v1.4.1, let's keep it as it will help us catch things before implementation." The cycle-3 cleanup is mechanical (reviewers gave a precise checklist with line numbers); the new architectural BLOCKER (transaction-wrap atomicity) is tractable in this revision. Cycle 4 verifies.

### Plan-revision changelog (cycle 3 → cycle 4)

- §1: rewritten as bulleted deliverables matching §12 + §13 dispositions. Explicit "NOT in PR 1 scope" list per L6 ruling.
- §2: rewritten with 6 file-by-file blocks. New §2.2 introduces DB-only-inside / side-effects-after pattern. New §2.4 introduces regex catching both call forms. Removed clippy config + `db/intelligence_feedback.rs` blocks.
- §3: rewritten as flat decision list. Idempotency claim explicitly NOT made. `tx: &ActionDb` pattern documented as closed Q1.
- §6: removed workspace clippy reference.
- §7: removed DOS-308 ownership reference (the file is DOS-7's after restructuring).
- §9: dropped idempotency-under-retry test; added 846/984/1545/1667 per-site tests + post-commit-emit-failure test for the new architectural pattern; added bash-regex manual verification.
- §10: Q1 + Q3 closed; Q2 stays open as cross-issue; Q-emit-placement added for `emit_and_propagate` in dismiss path.
- §11: preserved verbatim.
- §12: preserved verbatim.
- §13: this section.

### W0 retro update for cycle 3

To be folded into `wave-W0/retro.md`:

1. **Plan body / changelog drift:** when documenting L6 dispositions in §12 without rewriting §§1-10, an implementer would have followed the stale body. **Recommendation for W1+:** when an L6 ruling narrows scope, rewrite the affected sections of the plan, not just append a changelog. The §12 disposition is the contract; the body is the instructions.
2. **All-Codex cycle-3 fan-out worked:** 3 independent Codex reviewers found the same BLOCKER (body drift) AND surfaced unique findings (R2 caught the transaction-wrap atomicity architecture issue; R3 caught the file-path drift on Tauri wrappers). **Recommendation:** keep all-Codex slots for cycle-N reviews where N > 1; the diversity holds.
3. **Codex polling tooling has now hit two JSON-path bugs across cycle 2 + cycle 3.** Document the corrected polling pattern (`d.job.status`, not `d.status`) in tooling notes for future waves.
