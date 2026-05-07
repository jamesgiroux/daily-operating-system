# DOS-419 — W2b: lifecycle adapter + orchestrator wire-up

**Status:** L0 plan, rev 2 — expanded to absorb the W2b orchestrator wire-up + latency budget table + Tauri-command integration test per architect's M6 split rule. Awaiting reviewer signoff before impl.
**Depends on:** DOS-414 (Moving composer must land first; this layers on its `collect_lifecycle_signals` stub) + DOS-415 + DOS-416 + DOS-417 + DOS-418 (all 5 W2a per-section composers must exist before the orchestrator wires them via `tokio::try_join!`).
**Wave:** W2b — the second sub-wave per architect's M6 split. This is **the only ticket in W2b** and absorbs every cross-section integration concern.

## Scope

DOS-419 has three deliverables, sequenced together because they share `services/briefing_view_model.rs::compose()`:

1. **Lifecycle adapter** — implement `collect_lifecycle_signals` (replaces the DOS-414 stub).
2. **Orchestrator wire-up** — edit `briefing_view_model::compose()` to run all 5 W2a composers concurrently via `tokio::try_join!` (or `tokio::join!` if no composer returns `Result`).
3. **Latency budget table** — add per-section `BRIEFING_*_LATENCY_BUDGET_MS` constants alongside `compose()` mirroring `services/dashboard.rs:155 DASHBOARD_LATENCY_BUDGET_MS`. Log section-level latency on each call so the "slow service blocks assembly" failure mode is observable, not theoretical.

Plus the integration test that the W2b merge gate requires: `get_briefing_view_model` Tauri command returns `BriefingResult::Success` on a populated fixture exercising every section.

## 1. Acceptance criteria

### Lifecycle adapter
- [ ] `collect_lifecycle_signals(dashboard: &DashboardData) -> Vec<(EntityId, MovingSignalViewModel)>` returns one signal per `DashboardLifecycleUpdate`.
- [ ] Each emitted signal carries `kind: SignalDotKind::Lifecycle` and a `whatSegments` description like `"Moved to renewing"` or `"Renewal stage: prospecting → engaged"`.
- [ ] Each signal carries `LifecycleMixin.correctionState` populated from DOS-411 user_note claim lifecycle when the underlying lifecycle change has been corrected/contested.
- [ ] Signals attribute to the correct `EntityId` (the `account_id` from the lifecycle update).
- [ ] Trust band: scored from the lifecycle change's `confidence` field — `confidence >= 0.85` → `LikelyCurrent`, `0.6-0.85` → `UseWithCaution`, `<0.6` → `NeedsVerification`. Falls back to `Unscored` if confidence is missing.
- [ ] When `dashboard.lifecycle_updates` is `None` or empty, returns `vec![]` (graceful empty).
- [ ] `cargo test services::briefing::moving::lifecycle` covers: per-update mapping, confidence → trust-band classification at all 4 boundaries, correctionState pickup from DOS-411, missing-evidence handling, multi-update grouping by entity.

### Orchestrator wire-up
- [ ] `briefing_view_model::compose()` calls `compose_lead`, `compose_schedule`, `compose_predictions`, `compose_moving`, `compose_watch` concurrently via `tokio::join!` (composers are non-fallible today; switch to `try_join!` if any composer's signature gains a `Result`).
- [ ] No per-section composer is invoked sequentially in `compose()` — concurrent execution is the whole point of the orchestrator.
- [ ] Section-level errors do NOT escalate to envelope `BriefingResult::Error` automatically. The orchestrator owns whether a degraded section (composer that returned an empty/fallback shape) surfaces an envelope-level error or a partial-success render. Default policy: degrade to per-section empty branch; envelope `Error` only when the orchestrator itself can't run (e.g. fatal app-state failure).
- [ ] The chrome slices (`date`, `folio`, `day_strip`) are composed inline from the current local date — no per-section composer needed (they're already in `compose()` from the W2a/W2b skeleton commit `626af13b`).

### Latency budget table
- [ ] New constants alongside `compose()`:
  ```rust
  const BRIEFING_LEAD_LATENCY_BUDGET_MS: u128 = 50;
  const BRIEFING_SCHEDULE_LATENCY_BUDGET_MS: u128 = 200;
  const BRIEFING_PREDICTIONS_LATENCY_BUDGET_MS: u128 = 100;
  const BRIEFING_MOVING_LATENCY_BUDGET_MS: u128 = 400;  // heaviest — multi-source
  const BRIEFING_WATCH_LATENCY_BUDGET_MS: u128 = 200;
  const BRIEFING_TOTAL_LATENCY_BUDGET_MS: u128 = 500;   // not the sum — we run concurrently
  ```
- [ ] Each composer's elapsed time is measured (`std::time::Instant`) and logged to `tracing` (or the existing `log_command_latency` helper if present) when it exceeds its budget.
- [ ] Total `compose()` elapsed time is measured against `BRIEFING_TOTAL_LATENCY_BUDGET_MS` and logged on overrun.
- [ ] Budgets are tunable; first-cut values land here. Tuning per real-world traces is a post-W6 follow-up.

### Tauri command integration test
- [ ] `cargo test --lib services::briefing_view_model::integration::populated_fixture_returns_success` — constructs an `AppState` test harness with seeded data (at least one meeting, one action, one lifecycle update) and asserts the Tauri command path produces `BriefingResult::Success` with non-empty `model.schedule.meetings`, non-empty `model.watch.rows`, and a Moving entity that includes the lifecycle signal.
- [ ] Test does NOT depend on Google Calendar auth or a live DB write — uses the dashboard service's existing test seam where possible. If the seam doesn't exist, this test is documented as an integration-style test that requires `AppState::for_tests()` (which doesn't currently exist) and is gated until that helper lands.

## 2. Trust-source declaration (architect's W2a merge gate)

| Source | Upstream | Today's state | W2b default | Unblocked at |
|---|---|---|---|---|
| **Lifecycle signals** | `services::dashboard.lifecycle_updates: Option<Vec<DashboardLifecycleUpdate>>`. Each carries `account_id`, `previous_lifecycle`, `new_lifecycle`, `renewal_stage`, `source`, `confidence`, `evidence`. | Wired today. | `trustBand` from `confidence` via classifier (≥0.85 LikelyCurrent / 0.60-0.85 UseWithCaution / <0.60 NeedsVerification / NaN Unscored). | Today. |
| **Correction state** | DOS-411 user_note claim lifecycle. Lookup via `claims::lifecycle_state_for_change(change_id)` returning `Option<CorrectionState>`. | DOS-411 user_note + cutover shipped at parent track fork point (138b1571). Lookup function existence MUST be verified at L1 — if absent, this plan adds the ~30-line lookup as part of scope. | `correctionState: None` (omitted from wire) when lookup returns None. Signal renders default modifier. | Today, modulo lookup-function verification. |
| **Latency timing** | `std::time::Instant` + existing `log_command_latency` (or `tracing` if not present in this layer). | Always available. | Logs only on overrun, not every call. | Today. |

## 2. Trust-source declaration (architect's W2a merge gate)

**Source upstream:** `services::dashboard.lifecycle_updates: Option<Vec<DashboardLifecycleUpdate>>`. Each `DashboardLifecycleUpdate` carries `account_id`, `previous_lifecycle`, `new_lifecycle`, `renewal_stage`, `source`, `confidence`, `evidence`.

**DOS-411 claim-lifecycle layer:** when an account's lifecycle change has a `user_note` claim contesting or correcting the auto-detected lifecycle, `LifecycleMixin.correctionState` reflects that. The DOS-411 claim_lifecycle table is the lookup source. If the table doesn't have an entry for this lifecycle change, `correctionState` is omitted (rendered as default state on SignalDot).

**Today's state:**
- `lifecycle_updates` is wired on the dashboard service today.
- DOS-411 user_note + cutover shipped at parent track fork point (138b1571).
- The lookup function `claims::lifecycle_state_for_change(change_id)` returns `None | Some("corrected") | Some("contested")` — must verify this exists or include in scope.

**W2a default:** if the DOS-411 lookup returns None for a given change, `correctionState` is omitted (signal renders default modifier). The Moving composer's empty-branch fallback is unaffected.

**Unblocked at:** today, modulo verification that `claims::lifecycle_state_for_change` exists (next step of this plan's mutation-existence check).

## 3. Mutation-existence verification (per W0 plan rev 3.1 merge gate)

This is the read-side adapter. Mutations triggered from the rendered lifecycle signals:

- `claims::correct(claim_id, correction)` — DOS-411 user_note flow. **Exists** at parent track fork point. The signal carries the underlying lifecycle-change claim_id so the user can correct it from the SignalDot threadAction.
- `claims::contest(claim_id, reason)` — DOS-411 user_note flow. **Exists** at parent track fork point.

Existence check needed for the read-side helper:
- `claims::lifecycle_state_for_change(change_id: i64) -> Option<CorrectionState>` — verify this exists. If not, include in scope as a tiny addition to `services::claims` returning the lookup.

## 4. Function signature + module layout

```rust
// src-tauri/src/services/briefing/moving/lifecycle.rs (new file, alongside moving.rs)

use crate::services::briefing_view_model::{
    CorrectionState, LifecycleMixin, MovingSignalViewModel, SignalDotKind, SignalUrgency,
    TrustBandWire, TrustMixin, WhatSegment,
};
use crate::types::DashboardLifecycleUpdate;

pub(super) fn collect_lifecycle_signals(
    updates: Option<&Vec<DashboardLifecycleUpdate>>,
    correction_lookup: &dyn Fn(i64) -> Option<CorrectionState>,
) -> Vec<(EntityId, MovingSignalViewModel)> {
    updates
        .map(|v| {
            v.iter()
                .map(|u| {
                    let entity_id = EntityId(u.account_id.clone());
                    let signal = build_signal(u, correction_lookup);
                    (entity_id, signal)
                })
                .collect()
        })
        .unwrap_or_default()
}

fn build_signal(
    u: &DashboardLifecycleUpdate,
    correction_lookup: &dyn Fn(i64) -> Option<CorrectionState>,
) -> MovingSignalViewModel {
    let what_segments = format_what_segments(u);
    let trust_band = classify_confidence(u.confidence);
    let correction_state = correction_lookup(u.change_id);
    MovingSignalViewModel {
        trust: TrustMixin {
            trust_band,
            trust_field_path: Some(format!("lifecycle_change.{}", u.change_id)),
            trust_source_date: u.evidence.as_ref().map(|e| Some(e.clone())),
            rendered_provenance: None,
        },
        lifecycle: LifecycleMixin { correction_state },
        kind: SignalDotKind::Lifecycle,
        when: format_when(u),
        what_segments,
        urgency: SignalUrgency::Normal,
        thread_action: None,
    }
}
```

Module layout:
- `src-tauri/src/services/briefing/moving/mod.rs` — split moving.rs into a dir module if it crosses ~500 LOC; otherwise inline lifecycle.rs as a sibling
- `src-tauri/src/services/briefing/moving/lifecycle.rs` — this adapter (~120 LOC + ~80 LOC tests)

The lookup function is taken as a closure rather than a hard import so the composer can wire either the real `services::claims::lifecycle_state_for_change` or a test stub. This keeps the adapter unit-testable without DB.

## 5. Confidence → trust-band classification

| confidence | TrustBandWire |
|---|---|
| ≥ 0.85 | LikelyCurrent |
| 0.60 ≤ c < 0.85 | UseWithCaution |
| 0 ≤ c < 0.60 | NeedsVerification |
| `f64::NAN` (shouldn't happen, defensive) | Unscored |

These thresholds match the parent-track DOS-320 trust-band thresholds for consistency. If parent track tunes them, this adapter follows.

## 6. `whatSegments` rendering

The signal's `what_segments` must be a typed list, not a composed string. Pattern:

- Lifecycle change with previous → new:
  - `[{ text: "Moved to " }, { text: "renewing", emphasized: true }]`
- Lifecycle change without previous (initial classification):
  - `[{ text: "Classified as " }, { text: "renewing", emphasized: true }]`
- Renewal-stage transition (when `renewal_stage` is set and the lifecycle didn't change):
  - `[{ text: "Renewal stage: " }, { text: "prospecting → engaged", emphasized: true }]`

The emphasis renders italic via SignalDot's existing CSS.

## 7. `when` formatting

`DashboardLifecycleUpdate` doesn't carry a timestamp on the wire today. Use a placeholder pattern matching other signals: best-effort relative ("today", "yesterday") if the dashboard service exposes it; else use the change_id ordering as a proxy and label as `"today"`. Track exact-timestamp upgrade as a follow-up.

## 8. Files this lands

```
src-tauri/src/services/briefing/moving/
  lifecycle.rs                     ← new, ~200 LOC including tests (deliverable 1)
src-tauri/src/services/briefing/
  moving.rs                        ← edit: replace stub `collect_lifecycle_signals`
                                     in section 4 of DOS-414 with real call
                                     (deliverable 1)
src-tauri/src/services/
  briefing_view_model.rs           ← edit: rewrite `compose()` to call all 5
                                     composers via tokio::join! + add
                                     BRIEFING_*_LATENCY_BUDGET_MS constants
                                     + per-section + total elapsed logging
                                     (deliverables 2 + 3)
                                   ← edit: add #[cfg(test)] integration test
                                     module exercising the full Tauri command
                                     path on a populated fixture (deliverable 4)
src-tauri/src/services/claims.rs
  (optional ~30 LOC)               ← if `lifecycle_state_for_change` doesn't
                                     exist; trivial DB lookup (deliverable 1)
.docs/plans/wave-W2-redesign/
  DOS-419-plan.md                  ← this file
```

## 9. Out of scope

- **DOS-411 user_note claim type** — already shipped on parent track at fork point. No work here.
- **Claim correction UI** — the SignalDot's threadAction surface, not the adapter's concern. Existing W1 SignalDot shipped the rendering for `correctionState` already.
- **Lifecycle change creation** — the dashboard service produces them today. This adapter only consumes.
- **Trust-band threshold tuning** — follows parent-track DOS-320 thresholds. Tuning is post-W6.
- **Signal ordering or ranking** — handled by DOS-414 `change_magnitude` (the ranking algorithm). This adapter only emits one signal per update.
- **Latency budget tuning** — first-cut values land here. Tuning per real-world traces is a post-W6 follow-up. Specifically out of scope: deciding whether to switch to `tokio::try_join!` (requires composer signatures to gain `Result`, which is a post-W6 evolution).
- **Per-section error escalation policy** — default: composer empty-branch fallback degrades that section silently; envelope `Error` only when orchestrator itself fails. Refining this (e.g., "if Schedule fails, escalate to envelope error because the briefing without a schedule is meaningless") is a post-W6 product call.

## 10. L1 self-validation gates

- `cargo check --lib` clean
- `cargo clippy --lib -- -D warnings` clean
- `cargo test services::briefing::moving::lifecycle` (deliverable 1) exercises:
  - Per-update mapping (entity_id, kind, whatSegments, when)
  - Confidence boundary classification: 0.85, 0.60, exactly-on-boundary, NaN-defensive
  - correctionState pickup when lookup returns Some
  - correctionState omitted when lookup returns None
  - Empty `lifecycle_updates` → empty Vec
  - Multiple updates for same entity → multiple signals (grouping happens in DOS-414's `group_signals_by_entity`, not here)
  - whatSegments format for previous→new vs initial-classification vs renewal-stage transition
  - Wire shape serializes with camelCase (existing pattern)
- `cargo test services::briefing_view_model::compose_runs_concurrently` (deliverable 2) exercises:
  - All 5 composers called within one `compose()` invocation
  - Concurrent execution proven by mocking each composer with a 50ms delay and asserting total elapsed < 200ms (i.e. not 5×50=250ms)
  - Chrome slices (date/folio/dayStrip) populated from `Local::now()`
  - `compose()` returns `BriefingResult::Success` with all 8 model slices populated
- `cargo test services::briefing_view_model::latency_logging` (deliverable 3) exercises:
  - Composer that exceeds its budget triggers a log line containing the section name + elapsed ms
  - Composer within budget produces no log line
  - Total `compose()` overrun produces a separate log line distinct from per-section
- `cargo test --lib services::briefing_view_model::integration::populated_fixture_returns_success` (deliverable 4) exercises the Tauri command path end-to-end with seeded fixture data; asserts non-empty `model.schedule.meetings`, non-empty `model.watch.rows`, and a Moving entity carrying a lifecycle signal with `correctionState` populated by the test stub of `lifecycle_state_for_change`.

## 11. L2 reviewers

- **code-reviewer subagent** — diff review across all 4 deliverables. Focus on:
  - Lifecycle adapter: confidence classification correctness, segment formatting, defensive handling of optional fields.
  - Orchestrator: concurrent execution actually achieved (no accidental sequential awaits), error-degradation policy enforced, chrome composition unchanged from existing skeleton.
  - Latency table: budgets reasonable, logging is gated on overrun (not every call).
  - Integration test: doesn't depend on live external services; uses a test seam.
- **architect-reviewer subagent** — confirms:
  - Closure-injection pattern for `correction_lookup` appropriate vs hard import.
  - Trust-band thresholds match DOS-320.
  - **W2b merge gate satisfied** per `waves.md:94` — `tokio::join!` orchestration, lifecycle adapter layered on Moving, latency budgets logged, Tauri command returns Success on populated fixture.
  - Per-section error policy is the right default.
- **codex review** — independent shape check; pin the SignalDotKind="lifecycle" wire string, the LifecycleMixin flatten behavior, and the orchestrator's concurrent-execution semantics in tests.

## 12. Risk + sequencing notes

- **Hard dependency on all 5 W2a composers.** Orchestrator wire-up cannot land before DOS-414/415/416/417/418 are complete. Lifecycle adapter cannot land before DOS-414 stubs `collect_lifecycle_signals`. **W2b is sequential after all of W2a clears L3.**
- **Soft dependency on `claims::lifecycle_state_for_change`.** Verify it exists during L0 review. If not, the implementation includes a ~30-line addition to `services::claims`.
- **Single-ticket sub-wave.** Per architect's M6 split rule, DOS-419 is the *only* W2b ticket — it absorbs all cross-section integration concerns (orchestrator + latency + lifecycle adapter) so no other ticket touches `briefing_view_model::compose()`. This avoids the merge contention the split was designed to prevent.
- **Trust-band threshold consistency:** if parent track DOS-320 has tuned thresholds since fork, this adapter must follow. Verify during impl.
- **`tokio::join!` vs `tokio::try_join!`:** today, every composer is non-fallible (returns its view-model directly, not `Result`). Use `join!`. If a future composer evolution makes one fallible, switching to `try_join!` is a single-line change — the orchestrator structure is identical.
- **AppState test seam:** the integration test (deliverable 4) needs an `AppState::for_tests()` helper that doesn't trigger heavy I/O (config load, audit log, DB open). If the helper doesn't exist when this ticket runs, scope expands by ~50 LOC to add it. Verify at L0 review.

## 13. Post-impl follow-ups

- **Exact lifecycle-change timestamps** when the dashboard service surfaces them.
- **Trust-band threshold tuning** if parent track DOS-320 evolves.
- **Multi-source corroboration:** if the same lifecycle change appears in claims from multiple sources, merge the trust attribution rather than picking one.
- **Latency-budget tuning** based on real-world traces. First-cut values are starting points.
- **`try_join!` migration** if any composer's signature evolves to return `Result` for typed errors.
- **Per-section error escalation policy refinement** if user feedback shows certain section failures should escalate to envelope `Error` rather than degrade silently.

## 14. Implementation notes

- Landed the lifecycle adapter in `src-tauri/src/services/briefing/moving/lifecycle.rs` and kept correction-state lookup closure-injected for focused unit coverage.
- Added `claims::lifecycle_state_for_change` using the existing `lifecycle_changes.user_response` review state. The current schema does not expose a claim/user-note row keyed directly by lifecycle `change_id`, so this is the durable source available today; the adapter seam can be swapped to a future claim-substrate lookup without changing signal mapping.
- Added `previous_renewal_stage` to the dashboard lifecycle update wire shape so renewal-stage transitions can render the planned previous-to-current segment.
- `briefing_view_model::compose()` now runs the five section composers through `tokio::join!`, with first-cut per-section and total latency overrun logging. Composer signatures remain non-fallible, so `try_join!` remains a follow-up.
- The command integration test uses Tauri's mock IPC path with an unencrypted test database fixture and enables `AppState::test_with_db_service` under `cfg(test)`.
- L1 validation passed:
  - `cargo check --lib`
  - `cargo clippy --lib -- -D warnings`
  - `cargo test services::briefing::moving::lifecycle`
  - `cargo test services::briefing_view_model::compose_runs_concurrently`
  - `cargo test services::briefing_view_model::latency_logging`
  - `cargo test --lib services::briefing_view_model::integration::populated_fixture_returns_success`
