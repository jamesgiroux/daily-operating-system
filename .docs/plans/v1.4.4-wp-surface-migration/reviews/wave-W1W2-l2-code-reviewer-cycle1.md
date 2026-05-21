# L2 (Diff) — code-reviewer (correctness) — v1.4.4 W1+W2 wave, cycle-1

**Range:** `0f8533e1..bdade2cc` (`wave/v1.4.4-w1-stage1a`)
**Scope bounding:** L2 blockers = literal AC violations / ADR-named contract violations / PR-introduced regressions. Theoretical hardening routed path-α per memory `feedback_l2_must_review_against_acceptance_criteria` and `feedback_l2_path_alpha_to_maintenance_project`.

## VERDICT: **CONDITIONAL APPROVE**

Two correctness defects to fix before merge; remainder path-α. Substrate (W1 extensions: Meeting EntityKind composer, MergeIntent variant + migration v245 + ADR-0123 V1.1 payload validation, list_accounts/people/projects + watermark + cursor invalidation) verified correct. TS `useAbilityCursor` field-naming aligns with the actual serde tags (`CursorState` snake_case inner, `Paginated` camelCase outer). MergeIntent affordance emits a data-attr button only — no mutation on render.

## Findings

### F1 [HIGH] — `*_claim_inner_consumer` invokes `record_claim_feedback` (db_write) on every consumer call

**Files:**
- `/Users/jamesgiroux/Documents/dailyos-repo/wp/dailyos/blocks/account-detail/render-functions.php:192-196`
- `/Users/jamesgiroux/Documents/dailyos-repo/wp/dailyos/blocks/project-detail/render-functions.php:210-217`
- `/Users/jamesgiroux/Documents/dailyos-repo/wp/dailyos/blocks/person-detail/render-functions.php:214-218`

**ADR cite:** ADR-0123 V1.1 + memory `feedback_wp_is_local_surface_not_remote` — "reads don't mutate sessions/audit/rate-limits; only feedback writes." `record_claim_feedback` resolves to `services::claims::record_claim_feedback` which opens a `db_write` connection and persists an append-only `claim_feedback` row.

**Trace:** Helper takes a `$claim_ref` and unconditionally fires both `claim_receipt` (read) AND `record_claim_feedback` (write). Today no inner block actually calls these helpers — quote-wall, value-commitments, on-track-chapter etc. only call `dailyos_envelope_consume_claim` (claim_receipt only). The helper is documented (account-detail line 152 +159, person-detail line 169-176) as the "single wiring authority" path for claim receipt fanout. Any future inner block following the documented contract will write a feedback row per claim per render. The MergeIntent affordance docs (recommended-actions line 159-172) even point to this helper as the merge-execution entrypoint — but here it fires on plain claim_ref render flow without an `action` payload, which would also produce a 400 from `validate_and_sanitize_metadata` (since MergeIntent requires `merge_target`).

**Fix:** Split into two helpers: `dailyos_*_claim_receipt_only( $claim_ref )` for render-time fanout (call `claim_receipt` only) and `dailyos_*_record_feedback( $claim_ref, $action, $metadata )` for nonce-gated POST handlers that emit typed feedback (MergeIntent etc.). Remove the unconditional `record_claim_feedback` from the render-path helper.

### F2 [MEDIUM] — envelope fallback hash uses `spl_object_hash((object) $envelope)` — breaks single-fetch promise when `envelope_render_id` absent

**File:** `/Users/jamesgiroux/Documents/dailyos-repo/wp/dailyos/blocks/_shared/envelope/envelope-resolver.php:105`

**Contract cite:** L0 packet W2 V1.2.1 §5.1 "envelope fetched once per request" + AccountDetailBlockTest line 118 `'producer invoked exactly once per outer render'`.

**Trace:** When the runtime response lacks both `envelopeRenderId` and `envelope_render_id`, the resolver falls back to `hash('sha256', $entity_type . '|' . $entity_id . '|' . spl_object_hash( (object) $envelope ))`. Casting array→object in PHP creates a fresh `stdClass` each call, so `spl_object_hash` returns a different value every invocation. Result:
1. Outer block calls `dailyos_envelope_handle_from_response($r1, 'account', 'a1')` → `H1` ← cached under `H1`, set as `$GLOBALS['dailyos_envelope_handle_for_request'] = H1`.
2. Inner block reads global → `H1` → `dailyos_resolve_envelope(H1, ...)` → cache hit. ✓ (this path works, because the outer cached under the same `H1` it computed).

So actually within a single render, the outer's `H1` is also stored in the cache under `H1`, and inner blocks reading `H1` from the global get a cache hit. **The bug surfaces only if any code re-derives the handle from the same response object** (e.g., re-passing the response into `dailyos_envelope_handle_from_response` will produce a different `H2`). Today no caller does this, so this is **MEDIUM (latent)**: any future code that retries handle derivation will silently bypass the cache and fire a duplicate producer call. Also the test at AccountDetailBlockTest line 118 verifies outer == 1 but does NOT verify outer + inner == 1.

**Fix:** Replace `spl_object_hash((object) $envelope)` with a content-stable hash, e.g. `hash('sha256', $entity_type . '|' . $entity_id . '|' . md5(serialize($envelope)))` (or `wp_json_encode($envelope)` if available). Add a regression test that calls outer render then a representative inner block render with the same response and asserts `client->calls === 1`.

### F3 [LOW path-α → maintenance] — N+1 claim_receipt invocations per inner block

**File:** `/Users/jamesgiroux/Documents/dailyos-repo/wp/dailyos/blocks/account-detail/inner/quote-wall/render-functions.php:93-100` (pattern repeats in value-commitments, on-track-chapter, account-technical-footprint, triage-section, stakeholder-grid).

**Trace:** Each claim-bearing inner block iterates `$projected_claim_refs` and calls `dailyos_envelope_consume_claim` per claim, which invokes `claim_receipt` once per claim_ref. A facts-heavy account with 50 quotes triggers 50 `claim_receipt` invocations from quote-wall alone; multiply across 6 claim-bearing inner blocks → potentially 300+ invocations per detail render.

**Status:** Not an AC violation (AC-462.3 only specifies "every claim-bearing inner block routes receipts through build_receipt_for_audience"). Route to maintenance project as a hot-path optimization (batch `claim_receipt_for_refs` ability or fold into `get_entity_intelligence` envelope projection).

### F4 [LOW path-α → maintenance] — touchpoints-feed missing AgentMcp audience gate

**File:** `/Users/jamesgiroux/Documents/dailyos-repo/wp/dailyos/blocks/touchpoints-feed/render-functions.php:36-72`

**Trace:** `block.json` description: "AgentMcp audience renders aggregate-only" but the renderer does not branch on `$envelope['audience']`. L1 body is empty (`<div class="dailyos-touchpoints-feed__body"></div>`) so nothing leaks today; gate must be added when the typed projection lands per AC-484.5.

## Substrate verified clean

- **Meeting EntityKind composer** (`producer.rs:130-136`): correctly gates on `EnvelopeSection::Health` + `EntityKind::Meeting`; `compose_meeting_health` invoked once per Meeting render.
- **MergeIntent variant** (`feedback.rs:632-681`, `migration 245`): payload validates `merge_target: SubjectRef` (required), `supporting_evidence` (≤500 chars + sanitizer pipeline). Migration widens CHECK constraint via canonical SQLite table-rebuild + preserves both indexes.
- **MergeIntent semantics** (`feedback.rs:359-377`): `verification_state: Active`, `trust_effect: NONE`, `repair: None`, `render: Default`, `is_truth_feedback: false`, `requires_action_metadata: true`. Read-doesn't-mutate-source-claim contract honored.
- **list_accounts/people/projects pagination** (`producer.rs:24-95`): watermark derived from `request_fingerprint` (filter + page_size + ability); cursor mismatch returns `CursorState::Invalidated { restart_required: true }`; off-by-one boundaries safe (`consumed = offset + items.len()` vs `total_after_filter`).
- **useAbilityCursor TS hook** (`useAbilityCursor.ts:108-279`): field-naming aligns (`response.cursorState` camelCase, inner `restart_required` snake_case per `CursorState`'s `rename_all = "snake_case"`); request-seq guard drops stale responses; reset on watermark/payload/scope change.
- **MergeIntent affordance UI** (`recommended-actions/render-functions.php:109-209`): Agent audience denied (AC-8.13); data-attrs only emitted; click handler is documented as POST→nonce→record_claim_feedback Tauri-side. No mutation on render.
- **Envelope cache (read path)**: outer→`$GLOBALS`→inner cache-hit path verified; only the fallback hash (F2) is latent.

## Residual risks (post-merge watch)

- F2's content-hash fix needs a regression test that pierces the outer+inner round-trip.
- AgentMcp audience gating will need to be added to every typed-projection landing in subsequent L1 passes (currently only metadata-proposal-cue gates correctly).
- The 24+15+12+10 inner block bodies are empty placeholders; AC-462.6 / AC-483.6 / AC-484.5 / W2 §5.4 visible-QA matrices are NOT exercised yet. The L1 wire-up deliverable is bounded (envelope-handle context, empty-chip pattern, claim-consumer delegation) and matches the AC stated in each render-functions.php docblock — but the wave retro should call out that "structural-wiring L1" is the deliverable and typed-projection L1 is downstream.

## Test gaps

1. No test asserts outer + inner render together fires `client->calls === 1` (the single-fetch contract end-to-end).
2. No test asserts `record_claim_feedback` is NOT invoked during a plain render path (F1 regression guard).
3. No test exercises the merge-intent payload round-trip from a clicked button through the nonce endpoint into `record_claim_feedback`.
4. No test exercises cursor invalidation in the TS hook (server returns `CursorState::Invalidated { restart_required: true }` → hook resets state).
