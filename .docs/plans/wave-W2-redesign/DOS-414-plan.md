# DOS-414 — Moving composer (W2a)

**Status:** L0 plan, awaiting reviewer signoff before impl.
**Anchor for the rest of W2a remaining work** (DOS-415, DOS-416, DOS-419 lifecycle adapter all coordinate with this).

## 1. Acceptance criteria

- [ ] `compose_moving(state: &AppState) -> MovingViewModel` populates `entities: Vec<MovingEntityViewModel>` (≤3, service-capped) when upstream data is available.
- [ ] Per entity: a kind classification, a state pill, a 1-2 sentence lede, a signal feed of 3-5 ordered `MovingSignalViewModel`, and a stack of `ProvenanceStat` metrics.
- [ ] Signal-feed items carry the correct `SignalDotKind` per source (meeting / action / email / lifecycle / gong-call / zendesk-ticket / slack-thread / linear-issue).
- [ ] Trust band on every fact-bearing field (signals, stats) is real per the trust-source declaration below — not blanket `unscored`.
- [ ] `LifecycleMixin.correctionState` is populated on signals whose underlying claim has been corrected/contested via DOS-411.
- [ ] When upstream sources return Empty/Error, the composer falls back to the existing empty-branch shape (zero entities, "Quiet." summary).
- [ ] `cargo test services::briefing::moving` exercises: per-source signal mapping, entity-kind classification, ranking by 24h change-magnitude, ≤3 cap, missing-source graceful fallback, lifecycle adapter coordination (DOS-419).
- [ ] `cargo clippy --lib -- -D warnings` clean.

## 2. Trust-source declaration (architect's W2a merge gate)

Moving is the heaviest service because it aggregates from multiple upstream sources, each with its own trust source. Per architect's M2 finding from DOS-413 L2, this declaration is mandatory.

| Source | Upstream | Today's state | W2a default | Unblocked at |
|---|---|---|---|---|
| **Meeting signals** | `services::dashboard.meetings` (Google Calendar via existing pipeline). Each meeting has prep claims with their own trust band when present. | Wired today. | `trustBand` from `meeting.intelligence_quality.trust_band` if present; else `Unscored`. | DOS-427 W4 trust-band wire-in promotes `Unscored` to scored. |
| **Action signals** | `services::actions::get_all_actions`. Actions have no claim trust today. | Wired today. | `Unscored`. | Pending action-as-claim modeling (post-v1.4.x track). |
| **Email signals** | `services::emails` + DOS-416 lift. Email claims have provenance from Glean. | DOS-416 lift not yet shipped. | Empty signal list from email source. | DOS-416. |
| **Gong call signals** | Gong integration claim type. Producer not yet wired to briefing. | Not wired. | Empty signal list. | Forward-feed producer ticket (post-v1.4.x). |
| **Zendesk ticket signals** | Zendesk integration claim type. Producer not yet wired. | Not wired. | Empty signal list. | Forward-feed producer ticket (post-v1.4.x). |
| **Slack thread signals** | Slack integration claim type via Glean. Producer not yet wired. | Not wired. | Empty signal list. | Forward-feed producer ticket (post-v1.4.x). |
| **Linear issue signals** | Linear integration claim type via Glean. Producer not yet wired. | Not wired. | Empty signal list. | Forward-feed producer ticket (post-v1.4.x). |
| **Lifecycle signals** | `services::dashboard.lifecycle_updates` (existing `DashboardLifecycleUpdate` shape) → DOS-419 adapter. | Lifecycle updates wired in dashboard service today; DOS-419 adapter is W2b. | `Unscored` until DOS-411 lifecycle claims surface a trust band on the change. | DOS-419 lands the adapter; DOS-411 lifecycle claim trust is parent-track work. |
| **Provenance stats per entity (Health, Stage, Confidence, Owner, Last touch, Tenure)** | Mix of intelligence-claims (Health, Confidence) and entity entity-fields (Stage, Owner). | Health and Confidence have trust bands today via DOS-320 backend trust per-field. Stage/Owner are entity-fields without trust. | `trustBand` per stat from intelligence-claims when available; `Unscored` for entity-field stats. | DOS-320 already shipped at fork point; this is wire-up. |

**Net:** at v1 ship, Moving signals come from meetings + actions + lifecycle. Email signals join when DOS-416 ships. Gong/Zendesk/Slack/Linear are placeholder kinds with no producer yet — render only when a producer ticket lands. The composer must handle each source as optional and degrade gracefully.

## 3. Mutation-existence verification (per W0 plan rev 3.1 merge gate)

Moving is a read-only display contract. Mutations triggered from Moving UI:

- `claims::correct(claim_id, correction)` — DOS-411 user_note flow. **Exists** at parent track fork point.
- `claims::contest(claim_id, reason)` — DOS-411 user_note flow. **Exists** at parent track fork point.

No new mutations needed for DOS-414 itself.

## 4. Function signature + module layout

```rust
// src-tauri/src/services/briefing/moving.rs

pub async fn compose_moving(state: &AppState) -> MovingViewModel {
    // 1. Fetch from each source (per-source helper).
    let dashboard = get_dashboard_data(state).await;
    let lifecycle_signals = collect_lifecycle_signals(&dashboard);
    let meeting_signals = collect_meeting_signals(&dashboard);
    let action_signals = collect_action_signals(state).await;
    let email_signals = collect_email_signals(state).await;  // DOS-416 dependency

    // 2. Group signals per entity. Each signal carries entity_id; signals for the
    //    same entity_id roll up into one MovingEntityViewModel.
    let by_entity = group_signals_by_entity(
        meeting_signals
            .into_iter()
            .chain(action_signals)
            .chain(email_signals)
            .chain(lifecycle_signals),
    );

    // 3. For each entity, compose 24h change-magnitude rank.
    //    See section 5 for the ranking algorithm.
    let ranked: Vec<MovingEntityViewModel> = by_entity
        .into_iter()
        .map(|(entity, signals)| build_entity_view(entity, signals, state))
        .filter(|e| !e.signals.is_empty())  // drop entities with no signals
        .collect();
    let mut ranked = ranked;
    ranked.sort_by(|a, b| change_magnitude(b).cmp(&change_magnitude(a)));

    // 4. Cap at 3 (service-capped per contract comment).
    let entities: Vec<_> = ranked.into_iter().take(3).collect();

    MovingViewModel {
        label: "Moving".into(),
        heading: "What's moving".into(),
        count_label: format_count_label(entities.len()),
        summary: format_summary(&entities),
        entities,
    }
}
```

Module layout (`src-tauri/src/services/briefing/moving.rs`):
- `compose_moving` — public entry point (~30 LOC orchestration)
- `collect_meeting_signals` — Meeting → Vec<(EntityId, MovingSignalViewModel)> (~50 LOC)
- `collect_action_signals` — Action → Vec<(EntityId, MovingSignalViewModel)> (~50 LOC)
- `collect_email_signals` — Email → Vec<(EntityId, MovingSignalViewModel)> (~50 LOC, DOS-416 feeds)
- `collect_lifecycle_signals` — DashboardLifecycleUpdate → Vec<(EntityId, MovingSignalViewModel)> (~30 LOC, DOS-419 absorbs into a separate adapter module)
- `group_signals_by_entity` — pure function, HashMap-based group-by (~20 LOC)
- `build_entity_view` — produces a MovingEntityViewModel from a (LinkedEntity, Vec<Signal>) (~80 LOC; reads the entity's provenance stats)
- `change_magnitude` — pure ranking function (~20 LOC)
- `format_count_label`, `format_summary` — pure label formatters (~15 LOC each)

Estimate ~350 LOC for the composer + ~250 LOC of tests.

## 5. Ranking algorithm — 24h change-magnitude

Each entity gets a `magnitude: f64` derived from its signal set. Rank by descending magnitude, take top 3.

Magnitude per entity = sum of per-signal weights:
- `lifecycle` signal (account moved between stages) → weight 5.0 (highest — these are the most strategically important)
- `meeting` signal where the meeting is today + has prep ready → 3.0
- `meeting` signal where the meeting is today + no prep → 2.5
- `action` signal where `is_overdue=true` → 2.0
- `email` signal where Glean tagged the email as customer-relevant + recent (within 24h) → 1.5
- `gong-call`, `zendesk-ticket`, `slack-thread`, `linear-issue` → 1.0 each
- `meeting` signal where the meeting is past or future-not-today → 0.5
- `action` signal where status is Started/Unstarted but not overdue → 0.5

Tie-breaker: more recent `source_asof` wins.

The weights are tunable; first-cut values land here and a follow-up tracks user-feedback-driven adjustment (post-W6 cleanup).

## 6. Entity-kind classification

`MovingEntityKind` (5 variants): `customer`, `person`, `project`, `internal`, `lifecycle`.

Rule:
- The entity's `LinkedEntity.entity_type` ("account" / "project" / "person") drives the kind:
  - `account` + `intelligence.is_internal=false` → `customer`
  - `account` + `intelligence.is_internal=true` → `internal`
  - `project` → `project`
  - `person` → `person`
- A row dominated by lifecycle signals (≥50% of the signal set is lifecycle kind) renders as `lifecycle` instead — surfaces the "this account just moved stages" story over the entity-identity story.

## 7. Files this lands

```
src-tauri/src/services/briefing/
  moving.rs                        ← rewrite from empty-branch (~350 LOC)
src-tauri/src/services/briefing/
  moving/
    signals.rs                     ← optional split if moving.rs grows past ~500 LOC
.docs/plans/wave-W2-redesign/
  DOS-414-plan.md                  ← this file (already lands)
```

## 8. Out of scope

- **DOS-419 lifecycle adapter** is its own ticket. This plan calls into the adapter via `collect_lifecycle_signals` but the adapter implementation is W2b.
- **Forward-feed producers** for Gong / Zendesk / Slack / Linear. Those source kinds render as empty signal lists until producer tickets land (post-v1.4.x).
- **Ranking-weight tuning.** First-cut values land; user-feedback adjustment is a post-W6 ticket.
- **Frontend rendering** — MovingRow already ships from W3. This composer only produces the view-model shape MovingRow consumes.
- **Mutation handlers** — MovingRow's `onNavigate` and `onThreadAction` callbacks are surface-owned; this composer doesn't wire them.

## 9. L1 self-validation gates

- `cargo check --lib` clean
- `cargo clippy --lib -- -D warnings` clean
- `cargo test services::briefing::moving` covers:
  - Empty upstream → empty entities (graceful)
  - Single meeting source → one entity with one meeting signal
  - Multiple sources → grouping per entity (HashMap test)
  - Ranking respects weights (lifecycle > meeting+prep > overdue action > etc.)
  - ≤3 cap enforced when 5+ entities have signals
  - `MovingEntityKind::lifecycle` selected when ≥50% lifecycle signals
  - Trust band passes through from per-source signals (not blanket Unscored when upstream provides)
  - Wire-shape serializes to camelCase per the existing pattern

## 10. L2 reviewers

- **code-reviewer subagent** — diff review on moving.rs. Focus: per-source mapping correctness, ranking algorithm boundary cases, no inline business logic in the composer (it's an orchestrator over per-source helpers).
- **architect-reviewer subagent** — confirms the per-source trust-source declarations are sufficient, ranking weights aren't pathological at the boundaries, and the empty-source degradation path doesn't surface partial-data confusion.
- **codex review** — independent shape check on the wire output; pin the camelCase field names + tagged-union behavior across all 8 SignalDotKind variants in the test fixtures.

## 11. Risk + sequencing notes

- DOS-414 is the architectural anchor for W2a remaining. DOS-415 (Watch full triage), DOS-416 (email lift), DOS-417 (calendar full lift), and DOS-419 (lifecycle adapter) all depend on patterns established here.
- DOS-419 lifecycle adapter and DOS-414 share `services/briefing/moving.rs`. Per architect's W2a/W2b split: DOS-414 lands in W2a with `collect_lifecycle_signals` stubbed (returns empty); DOS-419 lands in W2b and replaces the stub with the real adapter calling DOS-411 claim_lifecycle.
- Email signals (DOS-416) land separately. This composer's `collect_email_signals` returns empty until DOS-416 ships.
- Per-source helpers must be unit-testable without AppState. Use the same fixture pattern established in `services::briefing::schedule` MVP — pure mapping functions taking the raw upstream data type, returning `Vec<(EntityId, MovingSignalViewModel)>`.

## 12. Verification sequencing

L0 reviewer dispatch:
1. **architect-reviewer** — strategic check (trust source declarations, ranking algorithm soundness, source coverage)
2. **codex adversarial-review** — challenge the per-source weights and the entity-kind classification rule
3. **code-reviewer** — diff template review (the plan, not the code)

After signoff, implementation can fan out to a single codex impl agent or be hand-written. Given the size (~600 LOC including tests) and the architectural novelty, recommend hand-written by Claude with codex review at L2.

## 13. Post-impl follow-ups

- **DOS-411 lifecycle adapter wire-in** (DOS-419, W2b) replaces the stubbed `collect_lifecycle_signals`.
- **DOS-416 email feeder** populates `collect_email_signals` once email lift ships.
- **Gong/Zendesk/Slack/Linear forward-feed producers** are post-v1.4.x track. The composer is shaped to absorb them with no contract change.
- **Ranking-weight tuning** based on user feedback — tracked separately, not blocking.
