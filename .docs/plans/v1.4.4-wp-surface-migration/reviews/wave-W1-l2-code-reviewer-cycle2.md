# W1 wave L2 — code-reviewer verdict (cycle 2)

**Branch:** `wave/v1.4.4-w1-stage1a`
**Diff range:** `0f8533e1..HEAD` (cycle-2 patches: `0a586218` substrate + `c2e857b8` consumer skeletons + CI)
**Reviewer:** correctness / code-reviewer subagent (per engineering-ladder.md L2 matrix)
**Date:** 2026-05-20

---

## VERDICT: APPROVE

All three cycle-1 HIGH findings (F1 / F2 / F3) are remediated. No new HIGH or MEDIUM findings surfaced against the cycle-2 patch surface. Path-α residuals from cycle-1 (F4–F6) are unchanged — already routed to maintenance per the cycle-1 verdict.

---

## Cycle-1 blocker resolution

### F1 (DOS-341) — RESOLVED

**Path:** `src-tauri/src/services/claim_receipt/render.rs:33-41, 59-114, 342-461`

`render_receipt_for` now dispatches through `build_receipt_for_audience` with an `Audience` derived from the supplied `SurfaceContext` via the new `audience_for_surface()` helper. Mcp → `Audience::AgentMcp`; ActionsWork / EntityDetail / DailyBriefing / MeetingDetail → `Audience::UserTauri`. Three new tests lock the contract:

- `ac_341_4_mcp_surface_routes_through_agent_mcp_audience` — asserts the Mcp render strips `source_asof`, `provenance.sources`, `field_path`, `updated_at`, claim_id, and subject_id (the AgentMcp denylist).
- `ac_341_4_tauri_surface_routes_through_user_tauri_audience` — confirms all four Tauri-class surfaces carry source labels + `source_asof`.
- `audience_for_surface_mapping` — locks the surface→audience map against future regression.

The privacy module is no longer test-only; it is the production construction path. AC-341.1 / .4 / .10 / .12 all bind at the runtime path.

### F2 (AC-W1.9 / AC-W1.2) — RESOLVED

**Paths:** `src-tauri/scripts/check_w1_consumer_skeleton.sh` (+ `.test` companion), `wp/dailyos/blocks/{account-detail,daily-briefing,person-detail,project-detail}/render-functions.php`, `.github/workflows/lint-frontend.yml:130-131`.

The lint enumerates 5 W1 producers (`get_entity_intelligence`, `get_daily_briefing`, `claim_receipt`, `meeting_prep_status`, `record_claim_feedback`) and `grep -lF`'s each against the consumer-skeleton glob. Negative-fixture self-test in `.sh.test` confirms the lint fails when a producer goes unconsumed (1-of-5 fixture rejected) and passes on the full set. Wired into CI as "Enforce v1.4.4 W1 consumer-skeleton wiring (AC-W1.9)" in `lint-frontend.yml:130`.

Local run both scripts: both PASS.

### F3 (DOS-477) — RESOLVED via documented option-(b) path

**Paths:** `src-tauri/src/commands/claim_feedback.rs:93-163`, `src-tauri/src/services/entity_intelligence/envelope_cache.rs` (new, 200 LOC).

The Tauri command now takes `envelope_render_id: Option<String>` and consults `envelope_cache::lookup_envelope_for_render` before constructing the `EnvelopeSet`. Cache hit ⇒ `CachedEnvelopeAdapter` carries the producer-recorded claim_ids + proposal_ids; cache miss ⇒ logged fallback to the cycle-1 single-claim envelope with explicit `path_alpha_envelope_cache_v2` marker symbol. The cycle-1 verdict explicitly named this as the acceptable remediation: "explicitly file AC-477.2 enforcement as a W2 dependency... the binding check is satisfied at the substrate layer but not at the Tauri command surface in v1.4.4 W1."

**Observation (not blocking):** no production producer currently calls `record_envelope_for_render` — the cache-hit path is exercised only by the in-module unit tests. In production the fallback runs on every invocation, and the binding remains tautological at the command boundary until W2 wires `get_entity_intelligence` / `get_daily_briefing` to record their envelopes. This is the documented option-(b) path, surfaced explicitly via the `path_alpha_envelope_cache_v2` symbol and the warn-log on every cache miss. Recommend the wave retro K-out captures that W2 sub-L0 must include "ability-side `record_envelope_for_render` wiring on the 4 named renderable-envelope producers" as an acceptance criterion.

---

## New findings (cycle-2 patch surface)

None at HIGH or MEDIUM. Cycle-2 patches are surgical and contain their own test coverage. Spot-checked:

- `render.rs` db_read closure correctly wraps `Result<ClaimReceipt, String>` inside `Ok(...)` and re-types via `render_error_from_tagged` after the await — no error-path swallowing.
- `event_bridge.rs` (DOS-339): `set_app_handle` uses `OnceLock::set` correctly (subsequent calls no-op with debug log, not panic). `emit_claim_receipt_invalidated` is best-effort with `None` handle returning false; mutation path is unaffected.
- `claims.rs:8734-8747` calls the bridge AFTER the signal row commits — ordering is correct, bridge failure cannot strand a committed mutation.
- Migration v243 is additive (rebuilds `meeting_prep_status_view` for determinism); no schema break.
- `get_daily_briefing/producer.rs` sort: explicit `(Some, None) => Less` / `(None, Some) => Greater` puts undated meetings LAST as the comment promises — fix is correct.
- `wp/dailyos/blocks/*/render-functions.php` skeletons all guard `ABSPATH`, use `apply_filters('dailyos_runtime_client_for_block', null)` with `is_object` + `method_exists` checks, route through `invoke_ability`, and return typed error HTML on `is_wp_error`.

---

## Path-α residuals (carried from cycle-1, unchanged)

- **F4** — WrongSource source_ref mirror (maintenance ticket against `services::claims::record_claim_feedback`).
- **F5** — `transition_status` signal-emit-not-persist naming (rename to `emit_transition_signal` or document).
- **F6** — pre-existing clippy `let_underscore_must_use` failures on dev unrelated to W1.

All three remain path-α per cycle-1 routing. Not L2 blockers.

---

## Bounding rationale

Per memory `feedback_l2_must_review_against_acceptance_criteria` + `feedback_l2_path_alpha_to_maintenance_project`: cycle-2 patches resolve the three literal AC violations from cycle-1 (AC-341.1/4/10/12, AC-W1.9 / AC-W1.2, AC-477.2 via option-b deferral). No PR-introduced regressions on cycle-2 patch surface. APPROVE.

---

## Reviewer note

Cycle-2 is a clean recovery. F1 is a substantive wiring fix with property-test coverage. F2 lands the CI gate AND four minimal consumer skeletons that exercise every named producer. F3 chose the documented option-(b) path and surfaced the W2 obligation explicitly via `path_alpha_envelope_cache_v2`. The wave is ready to merge.

The single W2-track residual to capture in the retro K-out: the envelope cache is built but unpopulated in production — W2 must wire `record_envelope_for_render` into `get_entity_intelligence`, `get_daily_briefing`, `get_actions_work`, and `get_meeting_prep_brief` (the 4 renderable-envelope producers named in the marker function's docstring) so the binding check becomes substantive at the command boundary.
