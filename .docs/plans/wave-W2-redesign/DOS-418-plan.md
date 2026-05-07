# DOS-418 — Predictions adapter (W2a scout)

**Status:** scout impl, drives the W2a function shape end-to-end.

## What it produces

`compose_predictions(state: &AppState) -> PredictionsViewModel` — populates
the Predictions slice of `BriefingViewModel`. Service-capped at ≤10 items.

Wraps the abilities-runtime prediction outputs (DOS-218 / DOS-219) into the
locked `PredictionItem` shape from
`src-tauri/src/services/briefing_view_model.rs`.

## Trust source declaration

**Per architect's W2a merge gate.**

- **Source upstream:** the abilities-runtime (DOS-218 emits prediction
  invocations; DOS-219 reconciles them post-meeting). Each prediction is
  produced by a named ability (e.g. `predict_meeting_friction`) and carries a
  `rendered_provenance` tree from the ability call site.
- **Today's state:** the abilities-runtime predictions feed for the briefing
  is **unavailable** — there is no producer that hands a list of "today's
  predictions" to the briefing service. DOS-218 produces predictions when a
  meeting prep is run; DOS-219 reconciles after the meeting. The briefing's
  Predictions section needs a *forward-looking* feed that doesn't yet have a
  producer.
- **W2a default:** `predictions: vec![]`, `count: 0`, `collapsedLabel: "0
  predictions today"`. Trust band on individual items is `Unscored`; n/a
  while the list is empty.
- **Unblocked at:** DOS-431 (canonical cutover) or earlier if a forward-feed
  producer ships. Track explicitly so this empty branch doesn't become a
  cultural default.

## Function signature

```rust
pub async fn compose_predictions(_state: &AppState) -> PredictionsViewModel {
    PredictionsViewModel {
        label: "Predictions".to_string(),
        count_label: "0 today".to_string(),
        collapsed_label: "0 predictions today".to_string(),
        expand_hint: "expand".to_string(),
        count: 0,
        predictions: vec![],
    }
}
```

`async` so the function shape matches what the orchestrator (`compose()` in
W2b) will call via `tokio::try_join!`. Even though there's no I/O today, the
shape is stable for when the upstream feed wires in.

## Files this lands

- `src-tauri/src/services/briefing/predictions.rs` (new, ~70 LOC including
  test)
- `src-tauri/src/services/briefing/mod.rs` (new, registers the submodule)
- `src-tauri/src/services/mod.rs` (1-line edit: `pub mod briefing;`)

The `briefing` submodule is the W2 home for the per-section composers.
`briefing_view_model.rs` (the W0 contract types + orchestrator stub) stays
where it is; the section composers live alongside as `briefing::{predictions,
schedule, lead, moving, watch}`.

## What this scout verifies (W2a function-shape probe)

1. **Per-section composer signature is sound.** `async fn compose_<X>(state:
   &AppState) -> <X>ViewModel` — no `Result`, no error variant per section
   (the envelope's `Error` is owned by the orchestrator, not per-section).
2. **The W2a-W2b boundary is clean.** Composers don't call each other,
   don't touch `compose()`, don't write to the orchestrator. They produce
   their slice; the orchestrator composes them.
3. **Trust-source declaration discipline lands.** This plan is the
   acceptance template for the other 4 W2a tickets: each names its upstream
   source, today's state, default behavior, and the unblock condition.
4. **Module layout works.** `services::briefing::predictions` vs the
   existing `services::briefing_view_model` for the contract types. (Could
   also collapse to one module — flagging for L2 review preference.)

## Out of scope

- Upstream wire-in. No producer feeds the briefing predictions yet; that's
  a separate ticket (forward-feed producer) tracked alongside DOS-431.
- Mutations. `predictions::ack(prediction_id)` is a separate command (read-
  side contract has the `id` field; mutation registration is its own
  follow-up if not yet wired).
- Frontend wiring. The PredictionsSection pattern (DOS-425, W3) consumes
  this slice; no frontend work in W2a.

## L1 self-validation gates

- `cargo check --lib` clean
- `cargo clippy --lib -- -D warnings` clean
- `cargo test --lib services::briefing::predictions` — at least one test
  asserting the empty-branch wire shape (count=0, predictions empty, copy
  fields rendered).

## L2

- code-reviewer subagent — confirm the module layout and function
  signature are appropriate for the next 4 W2a services to inherit.
- Domain reviewer (architect-reviewer) — confirm the trust-source
  declaration template is what architect intended; flag if the empty
  branch needs a different wire shape (e.g., `count_label: "—"`).
