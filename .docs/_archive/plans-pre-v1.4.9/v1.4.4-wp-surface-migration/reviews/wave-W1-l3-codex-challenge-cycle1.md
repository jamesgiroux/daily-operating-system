# L3 Codex Challenge — v1.4.4 W1 Wave (cycle 1)

**Reviewer:** `/codex challenge` via `codex exec` direct (the `--adversarial-review` companion route OOM'd on the 19K-LOC diff; retried via `codex exec` with focused 5-point prompt)
**Diff:** `0f8533e1..ea05ddc6` (integrated wave through L2 cycle-3)
**Date:** 2026-05-20

## Verdict: 5 REAL DEFECTS — cycle-2 L3 patch required

Codex was prompted to stress-test 5 specific wave-level integration points. All 5 returned with real defects (no false-positives, all cite file:line evidence).

## Findings

### F1 — HIGH — Signal coalesce/ordering race (cross-signal discipline)

`signals/policy_registry.rs:380` documents `MeetingPrepStatusChanged` as 500ms coalesced async invalidation; `policy_registry.rs:860` excludes it from emit-path coalescing. Writer at `services/meeting_prep_status/write.rs:264` emits on every transition. Under rapid prep churn, event-storm + reorder vs `ClaimVerificationStateChanged` fan-out (post-write at `claims.rs:8713` + Tauri bridge at `claims.rs:8738`). No cross-signal ordering contract exists.

**Patch:** wire MeetingPrepStatusChanged into the emit-path coalescing per its declared policy; add cross-signal ordering test for simultaneous prep-transition + claim-feedback case.

### F2 — HIGH (privacy / ADR-0108 violation) — Touchpoint sensitivity bypass

`get_entity_intelligence` applies sensitivity gate to facts but touchpoints bypass `build_receipt_for_audience`. DB reader at `services/entity_intelligence/touchpoints.rs:173` selects `m.title`; projection at `abilities-runtime/src/abilities/get_entity_intelligence/producer.rs:566` stores as provenance label with `redacted: false`. AgentMcp envelopes see raw meeting titles despite the per-audience allowlist gate.

**Patch:** route touchpoint provenance label rendering through `build_receipt_for_audience` (or equivalent audience filter). Per-audience scrub of `m.title` for AgentMcp.

### F3 — MEDIUM — v243 migration view recreation race

`migrations/243_meeting_prep_status_indexed_view_deterministic.sql:16` uses `DROP VIEW; CREATE VIEW;` in separate statements; migration runner at `migrations.rs:3573` uses `execute_batch` (not per-migration transaction). Concurrent reader during migration window (multi-process / fresh-connection) can hit missing view via `read.rs:106` prepare. In-process startup readers safe (created after writer migrations per `db_service.rs:338`); multi-process not protected.

**Patch:** wrap v244 (new migration) in `CREATE OR REPLACE VIEW` semantics (SQLite doesn't have CREATE OR REPLACE for views — alternative: rename old to `_old`, create new, drop `_old`, all in a single transaction). Or split into two migrations with intermediate state where both views exist.

### F4 — HIGH (cross-actor poisoning / ADR-0125 violation) — Envelope cache lacks principal binding

`services/entity_intelligence/envelope_cache.rs:51` keyed by `envelope_render_id` only; no actor / surface / session / owner binding. Lookup at `envelope_cache.rs:144` by ID alone. Cache miss at `commands/claim_feedback.rs:114` falls back to single-claim envelope built from requested target — restoring the tautological check the cache was meant to remove. Command also hardcodes `RenderActor::user("user", None)` at `claim_feedback.rs:150`.

**Patch:** cache key becomes `(envelope_render_id, actor_principal, surface)`. Reject cross-actor lookups. Replace hardcoded actor with the actual authenticated principal from request context. Remove tautological fallback OR explicitly mark as a different code path (not a "cache miss" but "no envelope binding required" — and only for a narrowly-defined contract).

### F5 — HIGH (AC-W1.2 violation) — WP block skeletons decorative, not real consumers

`wp/dailyos/blocks/account-detail/render-functions.php:54` calls `invoke_ability('get_entity_intelligence', payload)` — but the production runtime client at `wp/dailyos/includes/transport/class-dailyos-runtime-client.php:85` requires a 3rd `$scope_set` argument. Inner consumer for claim receipt/feedback at `render-functions.php:101` is decorative: unsets input and returns `[]`. CI gate `check_w1_consumer_skeleton.sh` passed because it greps producer NAMES, not actual correct invocations.

**Patch:**
- Update all 4 WP block render-functions.php to pass `$scope_set` per the runtime client signature
- Replace decorative claim-receipt inner consumer with actual invocation (or document explicitly as W2-deferred and tighten the CI gate to NOT count decorative stubs as consumers — gate becomes "consumer invokes producer with valid signature")
- Update `check_w1_consumer_skeleton.sh` to verify signature, not just name presence

## Routing

These are L3 wave-level integration defects. Cycle-2 L3 patch path per engineering ladder line 27 (pass rule "codex challenge + architect-reviewer approve"). Architect-reviewer cycle 1 was APPROVE; codex challenge needs cycle-2.

Per memory `feedback_zoom_out_for_class_pattern_in_l2_loop`: F1 + F4 share a class-pattern (cross-sub-ticket consistency assumptions). Folding into a single cycle-2 patch ticket would be appropriate, but distinct ACs make distinct patches cleaner.
