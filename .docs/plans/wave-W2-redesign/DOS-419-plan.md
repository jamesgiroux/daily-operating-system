# DOS-419 — Lifecycle adapter for Moving (W2b)

**Status:** L0 plan, awaiting reviewer signoff before impl.
**Depends on:** DOS-414 (Moving composer must land first; this layers on its `collect_lifecycle_signals` stub).
**Wave:** W2b. Per architect's W2a/W2b split, this lands in the second sub-wave alongside the orchestrator wire-up.

## 1. Acceptance criteria

- [ ] `collect_lifecycle_signals(dashboard: &DashboardData) -> Vec<(EntityId, MovingSignalViewModel)>` returns one signal per `DashboardLifecycleUpdate`.
- [ ] Each emitted signal carries `kind: SignalDotKind::Lifecycle` and a `whatSegments` description like `"Moved to renewing"` or `"Renewal stage: prospecting → engaged"`.
- [ ] Each signal carries `LifecycleMixin.correctionState` populated from DOS-411 user_note claim lifecycle when the underlying lifecycle change has been corrected/contested.
- [ ] Signals attribute to the correct `EntityId` (the `account_id` from the lifecycle update).
- [ ] Trust band: scored from the lifecycle change's `confidence` field — `confidence >= 0.85` → `LikelyCurrent`, `0.6-0.85` → `UseWithCaution`, `<0.6` → `NeedsVerification`. Falls back to `Unscored` if confidence is missing.
- [ ] When `dashboard.lifecycle_updates` is `None` or empty, returns `vec![]` (graceful empty).
- [ ] `cargo test services::briefing::moving::lifecycle` covers: per-update mapping, confidence → trust-band classification at all 4 boundaries, correctionState pickup from DOS-411, missing-evidence handling, multi-update grouping by entity.

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
  lifecycle.rs                     ← new, ~200 LOC including tests
src-tauri/src/services/briefing/
  moving.rs                        ← edit: replace stub `collect_lifecycle_signals`
                                     in section 4 of DOS-414 with real call
src-tauri/src/services/claims.rs
  (optional ~30 LOC)               ← if `lifecycle_state_for_change` doesn't
                                     exist; trivial DB lookup
.docs/plans/wave-W2-redesign/
  DOS-419-plan.md                  ← this file
```

## 9. Out of scope

- **DOS-411 user_note claim type** — already shipped on parent track at fork point. No work here.
- **Claim correction UI** — the SignalDot's threadAction surface, not the adapter's concern. Existing W1 SignalDot shipped the rendering for `correctionState` already.
- **Lifecycle change creation** — the dashboard service produces them today. This adapter only consumes.
- **Trust-band threshold tuning** — follows parent-track DOS-320 thresholds. Tuning is post-W6.
- **Signal ordering or ranking** — handled by DOS-414 `change_magnitude` (the ranking algorithm). This adapter only emits one signal per update.

## 10. L1 self-validation gates

- `cargo check --lib` clean
- `cargo clippy --lib -- -D warnings` clean
- `cargo test services::briefing::moving::lifecycle` exercises:
  - Per-update mapping (entity_id, kind, whatSegments, when)
  - Confidence boundary classification: 0.85, 0.60, exactly-on-boundary, NaN-defensive
  - correctionState pickup when lookup returns Some
  - correctionState omitted when lookup returns None
  - Empty `lifecycle_updates` → empty Vec
  - Multiple updates for same entity → multiple signals (grouping happens in DOS-414's `group_signals_by_entity`, not here)
  - whatSegments format for previous→new vs initial-classification vs renewal-stage transition
  - Wire shape serializes with camelCase (existing pattern)

## 11. L2 reviewers

- **code-reviewer subagent** — diff review on the new file. Focus: confidence classification correctness, segment formatting, defensive handling of optional fields.
- **architect-reviewer subagent** — confirms the closure-injection pattern for `correction_lookup` is appropriate vs hard import; sanity-check trust-band thresholds match DOS-320.
- **codex review** — independent shape check; pin the SignalDotKind="lifecycle" wire string and the LifecycleMixin flatten behavior in tests.

## 12. Risk + sequencing notes

- **Hard dependency on DOS-414.** This adapter's signature uses types defined in the Moving composer's module (`EntityId`). DOS-414 must land first with the stub `collect_lifecycle_signals` returning empty; DOS-419 replaces the stub with the real implementation.
- **Soft dependency on `claims::lifecycle_state_for_change`.** Verify it exists during L0 review. If not, the implementation includes a ~30-line addition to `services::claims`.
- **Wave timing:** per architect's split, DOS-414 lands in W2a, this lands in W2b alongside the orchestrator wire-up. The W2b sub-wave is sequential (DOS-419 + orchestrator + latency budget table all touch the same surfaces).
- **Trust-band threshold consistency:** if parent track DOS-320 has tuned thresholds since fork, this adapter must follow. Verify during impl.

## 13. Post-impl follow-ups

- **Exact lifecycle-change timestamps** when the dashboard service surfaces them.
- **Trust-band threshold tuning** if parent track DOS-320 evolves.
- **Multi-source corroboration:** if the same lifecycle change appears in claims from multiple sources, merge the trust attribution rather than picking one.
