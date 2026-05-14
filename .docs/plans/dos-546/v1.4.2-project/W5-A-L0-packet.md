# W5-A L0 packet - click-bound WordPress feedback router

Date: 2026-05-13 (V2)
Project: v1.4.2 - Personal Intelligence Engine: WordPress Foundation
Parent: DOS-546
Wave: 5 - Feedback + theme + negative fixtures
Issue: DOS-573 (W5-A: WordPress-side feedback router)
Linear: https://linear.app/a8c/issue/DOS-573
Working branch: dos-546-w5-a-l0-prep
Worktree: /private/tmp/dailyos-w5-a-l0-prep

This packet captures the W5-A plan contract for L0 review.
Linear remains the canonical execution surface.
This packet supersedes the issue text only where it makes an implicit decision explicit.

W5-A is the WordPress-side feedback router.
It is the client-side and PHP-side path that turns a deliberate block affordance click into a typed runtime feedback write.
It does not infer corrections from Gutenberg save diffs.
It does not write substrate state directly.
It does not invent a second feedback substrate.

The load-bearing user outcome is:

```text
user click
  -> WP JS validates W4-D edit-route affordance metadata
  -> WP PHP verifies WordPress REST nonce and edit_post(post_id)
  -> WP PHP confirms the post/block signed projection envelope is current
  -> WP PHP submits typed feedback through DailyOS_Runtime_Client
  -> runtime issues, atomically verifies, and consumes the W4-E nonce inside the feedback path
  -> runtime applies feedback through existing claim services and W4-B commit path
  -> runtime returns the post-commit composition_version
  -> WP re-invokes W4-A0/W4-A and swaps the rendered block with current scoped HTML
```

## Status snapshot

- W5-A is Wave 5, not Wave 4.
- A V1 L0 packet can be authored now because the upstream contracts are stable enough:
  W4-B V9 owns watermark, bridge errors, route module ownership, and session-bound user checks.
- W4-D V3 owns edit-route projection semantics and refusal reasons.
- W4-E V2 owns presence nonce issue/verify and click-bound nonce lifecycle.
- W3-B owns the PHP runtime client and HMAC-signed transport once implemented.
- DOS-589 owns dispatcher delivery for retry-after cursors and scope-filtered replay.
- W4-A renderer work is upstream of W5-A for visible affordances and block re-rendering.
- The current prep worktree contains the project docs but not W4 stage packet files; the upstream packets were read from the main worktree and the W4-B model packet from `/private/tmp/dailyos-w4-l0-prep`.
- This packet is documentation only.
- This packet does not implement code.
- This packet does not commit.
- This packet does not create a PR.

## Pre-work

### Upstream packets read

- W4-B V9 model packet:
  `/private/tmp/dailyos-w4-l0-prep/.docs/plans/dos-546/v1.4.2-project/W4-B-L0-packet.md`.
- W4-B V9 upstream packet:
  `/Users/jamesgiroux/Documents/dailyos-repo/.docs/plans/dos-546/v1.4.2-project/W4-B-L0-packet.md`.
- W4-A0 packet:
  `/Users/jamesgiroux/Documents/dailyos-repo/.docs/plans/dos-546/v1.4.2-project/W4-A0-L0-packet.md`.
- W4-D V3 packet:
  `/Users/jamesgiroux/Documents/dailyos-repo/.docs/plans/dos-546/v1.4.2-project/W4-D-L0-packet.md`.
- W4-E V2 packet:
  `/Users/jamesgiroux/Documents/dailyos-repo/.docs/plans/dos-546/v1.4.2-project/W4-E-L0-packet.md`.
- W3 packet:
  `/Users/jamesgiroux/Documents/dailyos-repo/.docs/plans/dos-546/v1.4.2-project/W3-L0-packet.md`.
- DOS-573 issue slice:
  `/Users/jamesgiroux/Documents/dailyos-repo/.docs/plans/dos-546/v1.4.2-project/02-issues.md`.
- Wave plan:
  `/Users/jamesgiroux/Documents/dailyos-repo/.docs/plans/dos-546/v1.4.2-project/03-wave-plan.md`.

### W4-B V9 contracts inherited

- W4-B section 1 gives W5-A `ClaimRef.field_path` and `Block.field_bindings`.
- W4-B section 2 gives W5-A the `409 stale_watermark` envelope and scope-filtered correction behavior.
- W4-B section 3 makes `composition_version` bridge-assigned through `commit_composition`.
- W4-B section 5 defines version event rows and assigns delivery to DOS-589.
- W4-B section 6 requires W4-C signature check before W4-B 409 handling.
- W4-B section 6.5 defines `BridgeSurfaceError` precedence.
- W4-B section 7 defines `MidFlightMutation` and durable `retry_after_event` cursor behavior.
- W4-B section 13 defines the claim mutation target model and `commit_claim` boundary.
- W4-B section 14 gives retry-after guidance for 409 envelopes.
- W4-B section 15 defines the `version_events` outbox used by DOS-589.
- W4-B section 16 gives the class-level scope-filter rule inherited by DOS-573.
- W4-B section 17 gives `wp_user_id` session binding inherited by DOS-573.
- W4-B acceptance 37 pins `src-tauri/src/bridges/surface_client.rs` as the owner for `/v1/surface/*` routes, including `/v1/surface/feedback`.

### W4-D V3 contracts inherited

- W4-D section 1 publishes `project_composition_for_surface(composition, ctx)`.
- W4-D section 1.1 pins `ProjectedComposition`, `ProjectedBlock`, and `EditRoute`.
- W4-D section 4 pins `BindingRole` dispatch.
- `BindingRole::Source` preserves claim attribution and visible provenance.
- `BindingRole::Source` alone is not enough for correction routing.
- `BindingRole::ComputedFrom` must refuse feedback.
- `BindingRole::DisplayOnly` must expose no feedback UI.
- `BindingRole::FeedbackTarget` is the explicit feedback receiver.
- `FeedbackTarget` with zero claim refs is not claim-correctable in v1.4.2.
- W4-D section 11 publishes `edit_routes` with `feedback_allowed` and refusal reasons.
- W4-D acceptance 21 requires W5-A to refuse ComputedFrom, DisplayOnly, Source-only, unknown role, missing receiver, zero-ref receiver, and ambiguous receiver routes.

### W4-E V2 contracts inherited

- W4-E section 2 defines the nonce tuple:
  `(surface_client_id, session_id, wp_user_id, claim_id, field_path, action, claim_version, composition_id, composition_version)`.
- W4-E section 3 defines the click action vocabulary:
  `correct`, `dismiss`, `corroborate`, `contradict`.
- W4-E section 6 registers `POST /v1/surface/nonce/issue`.
- W4-E section 7 registers `POST /v1/surface/nonce/verify`.
- W4-E section 7 returns nonce-bound expected versions on verify success.
- W4-E section 8 makes nonce issue click-bound, not page-load-bound or save-bound.
- W4-E section 9 requires atomic consume.
- W4-E section 11 makes lazy invalidation during verify the correctness boundary.
- W4-E section 12 pins safe audit emission.
- W4-E section 13 pins rejection taxonomy.
- W4-E section 14 pins browser-to-WP-to-PHP-to-runtime nonce request flow.
- W4-E section 15 requires all mutations to go through services.

### W3 and transport contracts inherited

- W3 section W3-B owns `DailyOS_Runtime_Client` and HMAC signing.
- W3 section 72 defaults PHP runtime transport to WordPress HTTP API wrapped in `DailyOS_Runtime_Client`.
- W3 section W3-B requires exact-byte canonicalization to match W2 HMAC.
- W3 section W3-B forbids exposing bearer or HMAC key material to JS.
- W3 section W3-B requires key material redaction in debug paths.
- W2 transport and HMAC remain the runtime trust boundary for `/v1/surface/*`.
- W5-A therefore adds methods to the runtime client, not browser calls to Rust.

### DOS-589 contracts inherited

- DOS-589 consumes W4-B `version_events` with `event_seq` replay ordering.
- DOS-589 resolves `retry_after_event` cursors for 423 `MidFlightMutation`.
- DOS-589 routes `claim.write_rejected` and related events scope-filtered.
- W5-A consumes DOS-589 events for retry/re-render behavior.
- W5-A does not implement the dispatcher.
- W5-A must not poll in a tight loop on 409 or 423.

### Substrate reuse audit

- Rust has an existing signed-route candidate for `POST /v1/surface/feedback` in `src-tauri/src/surface_runtime/mod.rs`.
- Rust has existing SurfaceClient rate-limit classes including `FeedbackWrite` in `src-tauri/src/bridges/surface_client.rs`.
- Rust has existing `submit.feedback` scope in SurfaceClient pairing defaults.
- Rust has existing typed feedback substrate:
  `src-tauri/abilities-runtime/src/abilities/feedback.rs`.
- Rust has existing service write entry:
  `src-tauri/src/services/claims.rs::record_claim_feedback`.
- Rust feedback actions include `ConfirmCurrent`, `MarkOutdated`, `MarkFalse`, `WrongSubject`, `WrongSource`, `CannotVerify`, `NeedsNuance`, `SurfaceInappropriate`, and `NotRelevantHere`.
- Rust `ClaimFeedbackInput` carries `claim_id`, `action`, `actor`, `actor_id`, and `payload_json`.
- Rust tests already cover feedback persistence, lifecycle changes, repair enqueueing, idempotent replay behavior, unknown claim rejection, and simulate-mode rejection.
- No WP plugin `register_rest_route` precedent was found in this worktree or the main worktree.
- No PHP `DailyOS_Runtime_Client::submit_feedback` method was found in this worktree or the main worktree.
- No W4-E nonce request integration exists yet in PHP source in this worktree.
- Therefore W5-A authors the WP REST route, PHP feedback router, runtime-client feedback method, and JS click integration as net-new WordPress surfaces.

## What W5-A authors net-new

| Surface | Status | W5-A scope |
|---|---|---|
| `wp/dailyos/includes/class-dailyos-feedback-router.php` | Missing | WP REST endpoint, WP REST nonce gate, `edit_post(post_id)` gate, post/block projection membership check, nonce request, feedback submission |
| `/wp-json/dailyos/v1/feedback` | Missing | Browser JS calls this WP endpoint, never Rust directly |
| `wp/dailyos/blocks/account-overview/feedback.js` | Missing | Enqueued front-end click handler with delegated listeners for W4-A `data-dailyos-feedback-action` affordances |
| Client-side route lookup | Missing | Resolve clicked field against W4-D `edit_routes`; fail closed if absent/refused; server still replays route authorization |
| W4-A affordance data attrs | W4-A-owned output consumed by W5-A | Require `data-dailyos-feedback-action`, `data-dailyos-field-path`, `data-dailyos-claim-id`, `data-dailyos-claim-version`, `data-dailyos-composition-id`, `data-dailyos-composition-version`, and `data-dailyos-block-id` |
| `DailyOS_Runtime_Client::issue_nonce` | W3-B obligation, consumed by W5-A | Accept array context and return runtime response array with the same HMAC signing/canonicalization as `submit_feedback` |
| `DailyOS_Runtime_Client::submit_feedback` | Missing | HMAC-signed POST to `/v1/surface/feedback` |
| Runtime `/v1/surface/feedback` handler registration | Partially represented as signed route candidate | Implement in `src-tauri/src/bridges/surface_client.rs` per W4-B section 37 |
| Runtime feedback request DTO | Missing | Typed payload with nonce-bound versions and feedback action |
| Feedback action mapper | Missing | Map W4-E wire verbs to substrate `FeedbackAction` variants |
| W4-E nonce verify inside feedback handler | Missing | `/v1/surface/feedback` delegates atomic verify-and-consume to W4-E; PHP never calls `/v1/surface/nonce/verify` directly |
| W4-B CAS handoff | Missing | Pass nonce-bound expected claim/composition versions into commit path |
| W4-B error envelope mapper | Missing | Preserve 422/409/423 precedence and reason codes back to WP |
| WP-side 409 refresh path | Missing | Reinvoke W4-A0 producer path and scope-filter correction |
| WP-side 423 wait path | Missing | Subscribe/wait via DOS-589 cursor, no tight retry loop |
| WP-side 422 tamper path | Missing | Show tamper banner, do not retry until W4-C clears |
| Feedback audit details | Missing | Submit/reject audit through `emit_surface_audit` |
| Negative fixtures | Missing | `dos573_fixture_*` suite |
| CI lint gates | Missing | No save-diff inference, no direct claim writes, no browser-to-runtime feedback |

## Directional decisions

### 1. Feedback is click-bound, not save-diff inferred

Decision:
W5-A routes feedback only from an explicit feedback affordance click.

Accepted gestures:
- `correct`
- `dismiss`
- `corroborate`
- `contradict`

Rejected sources:
- Gutenberg save diff.
- Attribute autosave.
- Paste/nesting/reorder diff.
- Visual text changes without a feedback affordance event.
- Inferred field changes from block JSON.
- Server-side comparison between old and new post content.

Rationale:
- DOS-573 issue text says the L0 cycle-3 reconciliation made feedback click-bound.
- W4-E section 8 says each discrete feedback gesture issues and immediately consumes a nonce.
- W4-D separates substrate-bound feedback from surface-local display/layout attrs.
- Save-diff inference would create nonces too early, after expiry, or without proof of deliberate user intent.
- Save-diff inference would also tempt WP to infer binding roles from field names, violating W4-D section 4.

Implementation rule:
- The JS event object must carry a click-generated `feedback_request_id`.
- `feedback_request_id` is client-correlation only; runtime replay protection is the W4-E nonce, not this browser-provided id.
- The router must reject requests that lack a recognized affordance action.
- The router must not inspect arbitrary changed attributes to synthesize feedback.
- A display/layout save may persist WP-local state, but it emits no substrate feedback event.

### 2. WP REST endpoint shape

Decision:
W5-A registers:

```http
POST /wp-json/dailyos/v1/feedback
```

The browser request body is:

```json
{
  "post_id": 123,
  "block_client_id": "editor-block-id",
  "composition_id": "composition-uuid",
  "composition_version": 17,
  "block_id": "block-uuid",
  "field_path": "claims[0].summary",
  "claim_id": "claim-uuid",
  "claim_version": 7,
  "action": "correct",
  "value": "Corrected value when action requires it",
  "feedback_request_id": "request-uuid"
}
```

`register_rest_route` args:

| Arg | Type | Required | `validate_callback` rule |
|---|---|---:|---|
| `post_id` | integer | yes | positive integer; permission callback later requires `current_user_can('edit_post', post_id)` |
| `block_client_id` | string | yes | non-empty string matching the rendered wrapper id/client id format |
| `composition_id` | string | yes | UUID string |
| `composition_version` | integer | yes | positive integer |
| `block_id` | string | yes | UUID string from the signed projected block |
| `field_path` | string | yes | non-empty projected field path; no arbitrary JSONPath execution |
| `claim_id` | string | yes | UUID string |
| `claim_version` | integer | yes | positive integer |
| `action` | string enum | yes | one of `correct`, `dismiss`, `corroborate`, `contradict` |
| `value` | string or object | conditional | required for `correct`; rejected for `corroborate`; optional structured reason for `contradict`; absent for `dismiss` |
| `feedback_request_id` | string | yes | UUID string; client-correlation only |

Validation rules:
- Field validation lives in `args` callbacks, not in `handle_feedback`.
- `permission_callback` verifies the caller is logged in.
- `permission_callback` verifies `X-WP-Nonce` with `wp_verify_nonce($nonce, 'wp_rest')` before any runtime call.
- `permission_callback` requires `current_user_can('edit_post', post_id)`, not only `edit_posts`.
- A user who can `edit_posts` globally but cannot edit the requested `post_id` receives WP 403 before nonce issue.
- PHP verifies `block_id`, `composition_id`, `composition_version`, and the referenced `edit_routes` belong to the current signed projection envelope for that `post_id` and block before requesting a nonce.
- PHP derives `wp_user_id` from `get_current_user_id()`.
- Browser-provided `wp_user_id` is ignored or rejected.
- JS may pass `block_client_id` for editor correlation only.
- JS must not pass bearer token or HMAC material.
- JS must not pass, receive, or store the presence nonce in block attributes, post content, local storage, or JS globals.
- The REST `OPTIONS` schema exposes only the typed request shape above; it must not disclose internal field names, claim inventories, scope tokens, or concrete claim ids beyond the schema type.

PHP class shape:

```php
final class DailyOS_Feedback_Router {
    public function __construct( DailyOS_Runtime_Client $runtime_client ) {}
    public function register_routes(): void {}
    public function permission_callback( WP_REST_Request $request ): bool {}
    public function handle_feedback( WP_REST_Request $request ): WP_REST_Response {}
    private function build_runtime_event( WP_REST_Request $request, string $nonce ): array {}
    private function map_bridge_error_to_wp_response( array $runtime_response ): WP_REST_Response {}
}
```

### 3. Runtime feedback endpoint shape

Decision:
W5-A submits to:

```http
POST /v1/surface/feedback
```

The runtime request body is:

```json
{
  "session_id": "surface-session-1",
  "wp_user_id": 42,
  "post_id": 123,
  "block_id": "block-uuid",
  "claim_id": "claim-uuid",
  "field_path": "claims[0].summary",
  "action": "correct",
  "claim_version": 7,
  "composition_id": "composition-uuid",
  "composition_version": 17,
  "presence_nonce": "opaque-token",
  "value": "Corrected value when action requires it",
  "feedback_request_id": "request-uuid"
}
```

Rules:
- The request is HMAC-signed through `DailyOS_Runtime_Client::submit_feedback`.
- `feedback_request_id` is passed through for log and browser correlation only; runtime does not trust it for replay protection.
- The route lands in `src-tauri/src/bridges/surface_client.rs` per W4-B V9 acceptance 37.
- Handler registration for `/v1/surface/feedback` is explicit W5-A scope; it is not deferred for discovery in a later packet.
- The bridge runs W4-B section 17 `wp_user_id` session binding before nonce verify or claim lookup.
- The bridge resolves `composition_id`, `composition_version`, `block_id`, and `field_path` to the signed `ProjectedComposition.edit_routes` and refuses non-`FeedbackTarget` routes server-side.
- Runtime-side route refusal is authoritative even if JS and PHP both supplied matching route metadata.
- The feedback handler delegates to the W4-E nonce authority for atomic verify-and-consume.
- PHP never calls `/v1/surface/nonce/verify` directly.
- Verify success returns nonce-bound expected versions.
- The handler passes those expected versions into W4-B CAS while applying feedback.
- The handler calls `record_claim_feedback` or its W4-B-updated service wrapper.
- No command handler or bridge handler writes DB rows directly.

### 4. Nonce lifecycle

Decision:
Nonce acquisition happens inside the PHP feedback submit path, not in JS and not at affordance render time:

```text
click
  -> JS POST /wp-json/dailyos/v1/feedback with typed request body and X-WP-Nonce
  -> PHP validates WP REST nonce, edit_post(post_id), and current post/block projection membership
  -> PHP calls /v1/surface/nonce/issue through DailyOS_Runtime_Client::issue_nonce
  -> PHP receives opaque nonce
  -> PHP immediately POSTs /v1/surface/feedback with nonce
  -> runtime feedback handler delegates atomic verify-and-consume to W4-E nonce store
  -> runtime commits feedback through services in the same protected path
```

Canonical flow pins:
- Affordances are W4-D edit-route-gated, not nonce-gated.
- W4-A may render affordance DOM only when `edit_routes` allow feedback, but that render does not issue or embed a nonce.
- JS never receives a W4-E presence nonce.
- JS never calls `/v1/surface/nonce/issue`, `/v1/surface/nonce/verify`, or `/v1/surface/feedback`.
- PHP requests a nonce only after WP REST nonce, post capability, and post/block projection membership pass.
- PHP never calls `/v1/surface/nonce/verify` directly.
- The W4-E WP-visible `/wp-json/dailyos/v1/nonce` endpoint is reserved for future surfaces that decouple nonce acquisition; DOS-573 click-then-submit does not call it.

Rules:
- W5-A does not request nonces at page load.
- W5-A does not request nonces on save.
- W5-A does not cache nonces in post content, block attributes, local storage, or transient PHP state.
- If nonce issue fails, W5-A does not submit feedback.
- If nonce verify fails inside `/v1/surface/feedback`, runtime does not apply feedback.
- On W4-E `409 claim_version_stale` or `409 composition_version_stale`, W5-A refreshes rather than asking the user to reuse the old gesture.
- On expired, replayed, mismatched field, mismatched action, wrong claim, or wrong user, W5-A shows the appropriate failure state and requires a new click.

#### 4.5 Loading state

- The clicked affordance sets `aria-busy="true"`, becomes disabled, and replaces the button glyph with an inline spinner while the request is pending.
- Loading state lasts no more than 2 seconds for ordinary request latency.
- If the runtime returns or implies a longer wait, W5-A switches to the 423 wait pattern instead of leaving the button in indefinite busy state.
- Loading state is cleared only after success re-render, refresh replacement, terminal rejection, or undo-state transition completes.

### 5. Edit-route consumption from W4-D

Decision:
W5-A treats `ProjectedComposition.blocks[].edit_routes` as the only route source.

Allowed route:
- `role = FeedbackTarget`
- `feedback_allowed = true`
- `claim_refs.length >= 1`
- requested `claim_id`, `claim_version`, and `field_path` match one receiver
- requester has the SurfaceClient scope needed to submit feedback

Refused route:
- `ComputedFrom`
- `DisplayOnly`
- `Source` without an adjacent explicit `FeedbackTarget`
- `FeedbackTarget` with zero claim refs
- missing route
- unknown role
- ambiguous receiver
- sensitivity blocked
- out of scope
- fallback degraded without receiver

Render-time rule:
- W4-A render output must not emit affordance DOM for `ComputedFrom`, `DisplayOnly`, `Source`-only, zero-ref `FeedbackTarget`, missing, unknown, ambiguous, sensitivity-blocked, or out-of-scope routes.
- This is a rendered HTML invariant, not only a submit-time refusal.
- Valid account-overview affordances render inline and expose `data-ds-name="IntelligenceCorrection"`.

W4-A data attrs consumed by W5-A:
- `data-dailyos-feedback-action`
- `data-dailyos-field-path`
- `data-dailyos-claim-id`
- `data-dailyos-claim-version`
- `data-dailyos-composition-id`
- `data-dailyos-composition-version`
- `data-dailyos-block-id`

W5-A JS integration:
- W5-A authors `wp/dailyos/blocks/account-overview/feedback.js`.
- The script is enqueued on rendered front-end pages that include `dailyos/account-overview`.
- The script uses delegated click listeners on `data-dailyos-feedback-action` attributes emitted by W4-A `render.php`.
- The script reads the data attrs, builds the typed WP REST request, and never reads arbitrary visible text as route authority.

Server-authoritative rule:
- JS route validation is an early UX fail-closed check only.
- PHP validates that the request belongs to the current signed post/block projection before nonce issue.
- Runtime resolves `composition_id`, `composition_version`, `block_id`, and `field_path` to `edit_routes` again and refuses non-`FeedbackTarget` routes before mutation.
- W5-A must not infer feedback eligibility from Gutenberg attribute names, visible labels, DOM text, CSS classes, block type alone, or claim refs without a `FeedbackTarget`.

### 6. Feedback action mapping

Decision:
W5-A maps the four WP affordance verbs to typed substrate feedback.

Initial mapping:

| WP action | Runtime action | Substrate action |
|---|---|---|
| `corroborate` | `corroborate` | `ConfirmCurrent` |
| `contradict` | `contradict` | `MarkFalse` |
| `correct` | `correct` | `NeedsNuance` |
| `dismiss` | `dismiss` | `SurfaceInappropriate` |

Rules:
- `correct` requires `value` or a typed correction payload.
- `corroborate` must not carry a corrected value.
- `contradict` may carry optional reason text but must not be required to.
- `dismiss` is surface dismissal because DOS-573 is a rendered-block feedback router, not a global claim-deletion surface.
- A future global tombstone affordance requires distinct UX and a packet amendment.
- The mapper must be a closed enum mapping, not caller-provided substrate action strings.

##### 6.1 SurfaceInappropriate surface metadata (V3 per codex C2 HIGH)

`SurfaceInappropriate` is the only feedback variant whose persisted payload requires a non-empty `surface` field — substrate validator at `claims.rs:5586` rejects a `SurfaceInappropriate` event with empty or null surface. W5-A's wire schema (§2 WP REST) does NOT accept a `surface` field from the browser (caller cannot assert which surface they are on; the substrate would have to trust client-provided context).

Rule:
- The runtime endpoint `/v1/surface/feedback` constructs the `surface` metadata server-side from the validated `composition_id` + the block type extracted from the W4-D projection envelope (every W4-A0 producer is keyed `BlockType → surface_kind` per W4-A0 V5 §6.3). For the v1.4.2 v1 block this resolves to `"account_overview"`.
- The mapping table `block_type → surface_kind` is closed enum and lives in the runtime; JS/PHP never supplies a surface string.
- The constructed `surface` metadata also feeds the SurfaceInappropriate event's audit emission (so dismiss audits carry which surface was dismissed from, not just the claim id).
- If the block type is unknown (e.g., projection envelope corrupt or an experimental block), the runtime rejects the dismiss with `422 InvalidBlockSurface`; no SurfaceInappropriate event is persisted with empty surface.

#### 6.3 Undo window (V3 contract per codex C2 MEDIUM)

Undo is a server-authoritative inverse-mutation path. The browser cannot supply substrate action strings, cannot extend the window, and cannot replay the undo.

Authoritative state:
- Each successful feedback submit returns an opaque `undo_token` in the response body — an HMAC over `(feedback_event_id || actor_session_id || expires_at)` signed with the runtime's W4-E HMAC key.
- The 30-second window is enforced server-side via `expires_at` carried in the HMAC payload. UI-visible expiry is advisory only.
- The `undo_token` is single-use: the runtime maintains a server-side consumed-tokens table (TTL 60s, double the window for replay margin) and rejects any second use with `409 UndoAlreadyApplied`.

Wire flow:
- Browser POST to WP REST `/dailyos/v1/feedback/undo` with `{ undo_token }` + WP REST nonce; no other fields.
- WP REST passes through to runtime `/v1/surface/feedback/undo`, which:
  1. Verifies HMAC + `expires_at` not elapsed + token not already consumed.
  2. Loads the original `feedback_event_id`; the substrate computes the inverse action server-side via a closed mapping (`ConfirmCurrent → revoke`, `MarkFalse → revoke`, `NeedsNuance → revoke-with-tombstone`, `SurfaceInappropriate → revoke-dismiss`). Browser supplies no action.
  3. Re-runs the W4-B precedence/concurrency checks for the inverse mutation (it is a real mutation, not a logical erasure).
  4. Marks token consumed.

Rules:
- Undo follows the same WP REST nonce, `edit_post(post_id)`, post/block projection membership, route, scope, and service-boundary checks as the original feedback.
- Undo does NOT consume a fresh W4-E presence nonce — the `undo_token` is the W4-E authority token for the inverse mutation, scoped tighter (single feedback event, single use, 30s).
- Undo emits an `audit_event` distinct from the original feedback (`feedback_undone`), carrying the original `feedback_event_id`, the inverse action applied, the actor, and the surface kind.
- Undo expiry is UI-visible but does not store nonce/HMAC material in JS state beyond `undo_token` itself; a click after expiry returns `410 UndoExpired` and the inline row clears the undo control on response.
- Browser cannot construct, derive, or interpret the `undo_token` — it is treated as opaque bytes.

#### 6.5 Affordance labels and placement

| Action | User-visible label |
|---|---|
| `corroborate` | `Looks right` |
| `contradict` | `Mark wrong` |
| `correct` | `Fix this` |
| `dismiss` | `Hide` |

Rules:
- Primary feedback-action DOM exposes only these four labels for the four W4-E actions.
- Raw verbs such as `corroborate`, `contradict`, `correct`, and `dismiss` must not leak as visible button text.
- The transient Undo control is a post-submit state control, not a `data-dailyos-feedback-action` primary affordance.
- Affordances render inline in the block body with `data-ds-name="IntelligenceCorrection"`.
- Affordances must not render in the Gutenberg sidebar, a modal, or a top-bar control.
- The inline correct editor expands in place and matches the shipped `IntelligenceCorrection` variant `correct`.

### 7. Error handling preserves W4-B precedence

Decision:
Runtime evaluates and returns the W4-B V9 precedence order:

| Precedence | Variant | HTTP | W5-A handling |
|---:|---|---:|---|
| 0 | `ProjectionTampered` | 422 | show tamper banner; do not retry until W4-C clears |
| 1 | `ProjectionVersionRollback` | 422 | show rollback/tamper banner; do not retry until refreshed projection clears |
| 2 | `MissingExpectedClaimVersion` | 400 | fail closed; W5-A bug or stale render; refresh block |
| 3 | `MidFlightMutation` | 423 | subscribe/wait using DOS-589 cursor and `retry_after_event` |
| 4 | `ClaimVersionOverflow` | 500 | fail loud; no retry loop |
| 5 | `StaleVersion` | 409 | refresh through W4-A0/W4-A with scope-filtered correction |
| 6 | `StaleComposition` | 409 | refresh composition; require a new click |

Canonical call graph:
- PHP calls `/v1/surface/nonce/issue` and then `/v1/surface/feedback`.
- PHP never calls `/v1/surface/nonce/verify` directly.
- `/v1/surface/feedback` atomically verifies-and-consumes the nonce through W4-E and commits feedback through W4-B/services.
- W4-E remains the nonce authority; W5-A delegates verification to runtime.

Rules:
- 422 wins over 409 when both tamper and stale are true.
- 423 wins over stale when the mutation lock is held.
- Missing expected version is a 400 bug path, not a feedback user error.
- W5-A must not collapse all runtime failures into `403`.
- W5-A must pass through safe machine-readable reason codes to JS.

#### 7.5 Error-state surface map

| Runtime result | Pattern | Severity/state | User-visible copy |
|---|---|---|---|
| 422 `ProjectionTampered` | TamperBanner / `ConsistencyFindingBanner` | high | `This block cannot be edited until verified.` |
| 422 `ProjectionVersionRollback` | `ConsistencyFindingBanner` | medium | `This block is on older version. Refreshing…` |
| 423 `MidFlightMutation` | inline spinner on affordance row | busy | `Saving someone else edit…` |
| 409 `StaleVersion` or `StaleComposition` | silent refresh plus toast | status, 2s auto-dismiss | `Updated to latest.` |
| 400 `MissingExpectedClaimVersion` | DevTools console only | n/a | no in-block user copy |
| 500 `ClaimVersionOverflow` | `ConsistencyFindingBanner` | high | `This claim has reached its edit limit.` |
| 429 rate-limited | inline below affordance row | warning | `Too many edits — try again in N seconds.` |
| sensitivity or scope-filtered refusal | inline unavailable state | neutral | `This content is no longer available here.` |

Rules:
- Sensitivity refusals never say `you do not have permission` in user-facing copy.
- The wire response for sensitivity refusals carries no claim metadata; audit detail may record reason `scope_filtered` server-side.
- 409 toast auto-dismisses after 2 seconds.

### 8. 409 stale-watermark handling

Decision:
W5-A refreshes rendered block HTML through W4-A0/W4-A after success and after stale-watermark responses.

Success flow:

```text
runtime 200 applied
  -> runtime returns post-commit composition_version
  -> WP re-invokes W4-A0 producer/read path with the same SurfaceClient scope set used for the original render
  -> W4-A renderer produces current scoped block HTML
  -> WP REST response returns rendered_html and composition_version
  -> JS replaces the block outer wrapper DOM
  -> clicked affordance loading state clears only after replacement completes
```

Success response body:

```json
{
  "outcome": "applied",
  "composition_id": "composition-uuid",
  "composition_version": 18,
  "rendered_html": "<div data-dailyos-block-id=\"block-uuid\">...</div>",
  "feedback_request_id": "request-uuid",
  "undo_token": "opaque-base64-bytes",
  "undo_expires_at": "2026-05-13T15:30:30Z"
}
```

`undo_token` is an opaque base64 string produced by the runtime per §6.3 V3; it carries the HMAC-bound `(feedback_event_id || actor_session_id || expires_at)` payload. The browser MUST treat it as opaque bytes; the substrate is the sole authority on validity, expiry, and inverse-action derivation. `undo_expires_at` is advisory UI-only; the substrate enforces expiry from the HMAC payload.

409 flow:

```text
runtime 409 stale_watermark or stale_composition_watermark
  -> WP receives scope-filtered correction envelope or correction_ref
  -> WP re-invokes W4-A0 producer/read path for the block scope
  -> W4-A renderer returns refreshed scoped HTML
  -> WP REST response returns outcome refresh_required and corrected_html
  -> JS replaces the block outer wrapper DOM
  -> prior click is spent; a new click is required for another feedback write
```

409 response body:

```json
{
  "outcome": "refresh_required",
  "composition_id": "composition-uuid",
  "composition_version": 18,
  "corrected_html": "<div data-dailyos-block-id=\"block-uuid\">...</div>",
  "toast": "Updated to latest."
}
```

Rules:
- `corrected_html` is re-rendered block HTML, not raw claim JSON.
- Inline `correction.claim` is already scope-filtered by W4-B section 2.
- If `scope_redacted = true`, WP must not attempt a direct claim fetch by id.
- Refresh must use the same SurfaceClient scopes as the original render; no elevated scope, `Actor::System`, or admin bypass is allowed.
- If a `correction_ref` arrives via DOS-589, the event-log lookup is scope-filtered through `Actor::SurfaceClient` with the original scope set.
- WP may use `correction.retry_after_ms` as a fallback wait ceiling.
- WP must prefer DOS-589 event/cursor behavior where supplied.
- WP must not tight-loop feedback submission after 409.

### 9. 423 mid-flight handling

Decision:
On 423 `MidFlightMutation`, W5-A waits for the W4-B/DOS-589 cursor.

Rules:
- Runtime response includes `retry_after_event`.
- W5-A subscribes or waits through DOS-589.
- W5-A never polls `/v1/surface/feedback` in a tight loop.
- Cursor resolution can produce committed update or mutation-aborted terminal event.
- On committed update, W5-A refreshes the block and requires a new click if the user still wants to act.
- On mutation aborted, W5-A may re-enable the affordance after refreshing current claim/composition versions.

### 10. 422 tamper handling

Decision:
On 422 `ProjectionTampered` or `ProjectionVersionRollback`, W5-A refuses retry.

Rules:
- Do not request another nonce for the same stale/tampered projection.
- Do not submit feedback again with a fresh nonce.
- Show the W4-A/W4-C tamper banner state.
- Tamper UI displays no `signed_payload`, `claim_id`, `composition_id`, or `field_path`.
- Tamper UI may show only an opaque incident id and an operator reason code.
- Wait until W4-C clears quarantine or a refreshed signed projection arrives.
- Do not expose `correction.claim`; W4-B V9 says tamper variants return no correction payload.

### 11. Capability, auth, and scope

Decision:
W5-A enforces WordPress authorization, post-bound edit authorization, and runtime SurfaceClient authorization.

WP side:
- `post_id` is required in the WP REST request body.
- `permission_callback` requires a logged-in user.
- `permission_callback` verifies `X-WP-Nonce` against the `wp_rest` action.
- Missing or invalid `X-WP-Nonce` returns WP 403 before nonce issue or runtime feedback calls.
- `permission_callback` requires `current_user_can('edit_post', post_id)`.
- `current_user_can('edit_posts')` alone is not sufficient.
- PHP derives `wp_user_id` from WordPress current user.
- browser-provided `wp_user_id` is ignored or rejected.
- PHP confirms the signed projected composition belongs to the requested `post_id` and block before nonce issue.

Runtime side:
- W4-B section 17 validates request `wp_user_id` against session-bound `wp_user_id`.
- W2/W3 HMAC and bearer validation run before protected route dispatch.
- SurfaceClient must have `submit.feedback`.
- W2-D feedback budgets apply.
- Runtime resolves route authority from the signed projected composition, not from browser trust.
- W4-E nonce verification runs inside `/v1/surface/feedback` before feedback mutation.
- 409 refresh and DOS-589 correction-ref lookups use the original SurfaceClient scope set with no elevation.

### 12. Audit emission

Decision:
Every feedback submit and every authenticated reject emits safe audit through the runtime path when the runtime is reached.
WP-side rejects that happen before runtime calls emit a local WP log entry that W6-A can correlate.

Submit event kind:
- `surface_feedback_submitted`

Applied event kind:
- `surface_feedback_applied`

Reject event kind:
- `surface_feedback_rejected`

WP local reject scope:
- Missing or invalid WP REST nonce returns 403 before nonce issue; if logged safely, the local WP log includes only opaque incident id, route name, and reason code.
- `edit_post(post_id)` failure returns 403 before nonce issue; local WP log records `post_edit_denied` with `wp_user_id`, `post_id`, `feedback_request_id` when authenticated.
- Missing route, refused route, malformed body, or projection-membership failure before nonce issue records a local WP log with safe correlation ids and no raw block payload.
- W6-A correlates WP local logs with runtime audit by `feedback_request_id`, `wp_user_id`, route name, and incident id.

Safe runtime detail shape:

```json
{
  "feedback_request_id": "request-uuid",
  "surface_client_id": "surface-client-id",
  "wp_user_id": 42,
  "claim_id": "claim-uuid",
  "field_path": "claims[0].summary",
  "claim_version": 7,
  "composition_id": "composition-uuid",
  "composition_version": 17,
  "action": "correct",
  "outcome": "applied",
  "reason": null,
  "bridge_error": null,
  "scope_redacted": false
}
```

Prohibitions:
- no raw nonce;
- no HMAC key;
- no bearer token;
- no corrected free-text value in audit detail unless a later privacy review explicitly allows a redacted/hash representation;
- no customer names, domains, or emails;
- no raw post content;
- no raw block payload;
- no `signed_payload` in UI, logs, or wire responses.

### 13. Intelligence Loop fit

1. Claim model:
   W5-A writes feedback against existing claims by `(claim_id, claim_version, field_path)`.
   It does not create display-only data.
2. Provenance and trust:
   `record_claim_feedback` preserves actor and action semantics; trust/source effects remain in the existing typed feedback substrate.
3. Signals and invalidation:
   feedback application emits existing claim feedback signals and W4-B version events through the service path.
4. Runtime and surfaces:
   WP is one SurfaceClient consumer; MCP/Tauri consumers must observe the same claim state after feedback commits.
5. Feedback loop:
   user corrections, dismissals, corroborations, and contradictions feed back through claim lifecycle, source reliability, trust inputs, and W4-A0 recomposition.

## Acceptance criteria

1. W5-A registers `POST /wp-json/dailyos/v1/feedback`.
2. W5-A implements `DailyOS_Feedback_Router` with constructor, `register_routes`, `permission_callback`, `handle_feedback`, `build_runtime_event`, and `map_bridge_error_to_wp_response`.
3. The REST route defines args callbacks for `post_id`, `block_client_id`, `composition_id`, `composition_version`, `block_id`, `field_path`, `claim_id`, `claim_version`, `action`, `value`, and `feedback_request_id`.
4. Request validation runs in args callbacks, not in `handle_feedback`.
5. The request body requires `post_id`.
6. The route has a WordPress `permission_callback`.
7. The permission callback requires a logged-in user.
8. The permission callback verifies `X-WP-Nonce` with `wp_verify_nonce($nonce, 'wp_rest')` before runtime calls.
9. Missing or invalid `X-WP-Nonce` returns WP 403 before nonce issue.
10. The permission callback requires `current_user_can('edit_post', post_id)`, not only `edit_posts`.
11. A logged-in user with `edit_posts` but without `edit_post` for `post_id` cannot submit and triggers no runtime call.
12. Browser JSON cannot assert trusted `wp_user_id`.
13. PHP derives `wp_user_id` from `get_current_user_id()`.
14. PHP verifies `block_id`, `composition_id`, `composition_version`, and `edit_routes` belong to the current signed projection for the requested post/block before nonce issue.
15. The REST `OPTIONS` response exposes only typed request schema and no internal field names, claim inventories, claim ids, or scope tokens.
16. JS click handlers exist for `correct`, `dismiss`, `corroborate`, and `contradict`.
17. W5-A authors and enqueues `wp/dailyos/blocks/account-overview/feedback.js` on rendered front-end pages.
18. W4-A `render.php` emits `data-dailyos-feedback-action`, `data-dailyos-field-path`, `data-dailyos-claim-id`, `data-dailyos-claim-version`, `data-dailyos-composition-id`, `data-dailyos-composition-version`, and `data-dailyos-block-id` for valid affordances.
19. Feedback is emitted only from explicit affordance clicks.
20. Gutenberg save diffs do not emit feedback events.
21. Surface-local display/layout autosaves do not consume nonces.
22. Affordance DOM is not emitted for `ComputedFrom`, `DisplayOnly`, `Source`-only, or zero-ref `FeedbackTarget` routes.
23. JS validates the clicked field against W4-D `edit_routes`.
24. Runtime re-resolves `composition_id`, `composition_version`, `block_id`, and `field_path` to signed `edit_routes` and refuses non-`FeedbackTarget` routes server-side.
25. Missing route fails closed before nonce issue and emits only safe WP local reject detail.
26. `ComputedFrom` routes expose no feedback UI and cannot submit.
27. `DisplayOnly` routes expose no feedback UI and cannot submit.
28. `Source` without `FeedbackTarget` cannot submit.
29. `FeedbackTarget` with zero claim refs cannot submit.
30. Ambiguous receiver cannot submit.
31. Unknown/future role cannot submit.
32. Sensitivity or scope-filtered refusal surfaces `This content is no longer available here.` and no claim metadata in the wire response.
33. `correct` carries a typed corrected value payload.
34. `corroborate` maps to `ConfirmCurrent`.
35. `contradict` maps to `MarkFalse`.
36. `correct` maps to `NeedsNuance`.
37. `dismiss` maps to `SurfaceInappropriate`.
38. The action mapper is a closed enum and never accepts caller-provided substrate action strings.
39. Primary affordance labels are exactly `Looks right`, `Mark wrong`, `Fix this`, and `Hide`.
40. Raw action verbs do not leak into visible DOM as button labels.
41. Affordances render inline with `data-ds-name="IntelligenceCorrection"`, not in Gutenberg sidebar, modal, or top-bar.
42. Inline correct editor expands in place and matches shipped `IntelligenceCorrection` variant `correct`.
43. Every successful feedback submit response body includes an opaque `undo_token` (HMAC over `feedback_event_id || actor_session_id || expires_at` signed by W4-E HMAC key) and an advisory `undo_expires_at` (UI display only). The 30-second window is server-enforced from the HMAC payload, not the wire-visible timestamp.
44. Undo POST to `/dailyos/v1/feedback/undo` with `{ undo_token }` + WP REST nonce; the substrate verifies HMAC + expiry + single-use consumed-token table; on success the runtime derives the inverse action server-side from a closed mapping (browser supplies NO action string and NO fresh W4-E presence nonce — the `undo_token` IS the W4-E authority for the inverse mutation, scoped tighter to single-event/single-use/30s).
45. Clicked affordance loading state sets `aria-busy="true"`, disables the control, and replaces the glyph with an inline spinner.
46. Loading state lasts no more than 2 seconds before switching to the 423 wait pattern when a longer wait is needed.
47. W5-A calls W4-E `/v1/surface/nonce/issue` through PHP runtime client after WP REST nonce, post capability, and post/block projection membership pass.
48. Affordances are W4-D edit-route-gated, not nonce-gated.
49. JS never receives a W4-E presence nonce.
50. The W4-E WP-visible `/wp-json/dailyos/v1/nonce` endpoint is not called by DOS-573 click-then-submit.
51. W5-A immediately submits `/v1/surface/feedback` with the nonce.
52. PHP never calls `/v1/surface/nonce/verify` directly.
53. JS never calls Rust runtime directly.
54. JS never receives HMAC key material.
55. JS never receives bearer token material.
56. Nonce is not stored in block attributes, post content, local storage, logs, or audit details.
57. `DailyOS_Runtime_Client::issue_nonce` accepts array context and signs with the same W3-B HMAC path as `submit_feedback`.
58. `DailyOS_Runtime_Client::submit_feedback` signs the request with the W3-B HMAC path.
59. Runtime `/v1/surface/feedback` handler registration is W5-A scope.
60. Runtime `/v1/surface/feedback` lands in `src-tauri/src/bridges/surface_client.rs`.
61. Runtime bridge runs W4-B section 17 `wp_user_id` session binding before nonce verify or claim lookup.
62. Runtime authoritatively refuses non-`FeedbackTarget` route submissions before mutation.
63. Runtime atomically verifies and consumes the nonce at the W4-E nonce store inside `/v1/surface/feedback` before feedback mutation.
64. Runtime does not trust `feedback_request_id` for replay protection; the nonce provides replay protection.
65. Runtime passes nonce-bound expected versions into W4-B CAS.
66. Runtime applies feedback through `services::claims::record_claim_feedback` or the W4-B-updated service entry.
67. No direct DB write exists in WP PHP.
68. No direct DB write exists in runtime bridge handler.
69. Every mutation goes through `services/`.
70. On success, feedback row persists in the existing claim feedback substrate.
71. On success, claim lifecycle/trust effects match the typed feedback matrix.
72. On success, W4-B version events or feedback signals are emitted through existing substrate paths.
73. On 200 success, WP receives post-commit `composition_version`, re-invokes W4-A0/W4-A, replaces rendered block HTML, and only then clears affordance loading state.
74. On 409 stale claim watermark, W5-A refreshes with scope-filtered correction behavior using the same SurfaceClient scopes as original render.
75. On 409 stale composition watermark, W5-A refreshes composition and requires a new click.
76. On 409, WP REST response body returns `outcome: "refresh_required"` and `corrected_html`; it does not return raw claim JSON.
77. On 409 `correction_ref` from DOS-589, event-log lookup is scope-filtered through `Actor::SurfaceClient` with no `Actor::System` or scope elevation.
78. On 423 mid-flight mutation, W5-A waits on DOS-589 cursor and does not tight-loop.
79. On 422 projection tamper, W5-A shows tamper banner and refuses retry until W4-C clears.
80. Tamper UI exposes no `signed_payload`, `claim_id`, `composition_id`, or `field_path`; it shows only opaque incident id and operator reason code.
81. On 422 projection rollback, W5-A refreshes signed projection and refuses feedback retry against old projection.
82. On 400 missing expected version, W5-A treats it as a bug/stale render, writes DevTools console detail only, and refreshes.
83. On 403 nonce mismatch, expired, replayed, wrong field, wrong action, wrong claim, or wrong user, no feedback write occurs.
84. On 429 rate limit, UI surfaces `Too many edits — try again in N seconds.` without replaying automatically.
85. Every BridgeSurfaceError maps to the section 7.5 DS pattern and copy.
86. Every authenticated feedback submit emits `surface_feedback_submitted`.
87. Every applied feedback emits `surface_feedback_applied`.
88. Every authenticated runtime reject emits `surface_feedback_rejected`.
89. WP-side rejects for capability failure, malformed body, projection membership failure, or missing route emit safe local WP logs for W6-A correlation.
90. Audit details contain no raw nonce, no HMAC key, no bearer token, no raw corrected text, no raw post content, no raw block payload, and no customer names, domains, or emails.
91. WP-side route tests cover WP REST nonce rejection and post-bound capability rejection.
92. PHP unit tests cover runtime-client request body, `issue_nonce`, `submit_feedback`, and HMAC canonical path.
93. Runtime tests cover W4-B error precedence for feedback and runtime-side route refusal.
94. End-to-end fixture covers user click through feedback apply through block re-render.
95. PHP fuzz fixtures cover missing WP REST nonce, minimal OPTIONS schema, and malformed body rejection.
96. Accessibility tests cover visible-name matching accessible names, Enter/Space activation, focus preservation through state changes, `role="status"` announcements, trust-band differentiation via `data-ds-state` plus iconography, and inline editor focus trapping until submit or cancel.
97. `cargo clippy -- -D warnings && cargo test && pnpm tsc --noEmit` green before L1 close.

## Negative fixtures

All W5-A fixtures use the per-DOS prefix `dos573_fixture_`.

1. **`dos573_fixture_1_click_bound_correct.rs`**
   User clicks `correct` on a FeedbackTarget route; nonce issued; feedback submitted; runtime maps to `NeedsNuance`; block refreshes.
2. **`dos573_fixture_2_click_bound_dismiss.rs`**
   User clicks `dismiss`; route maps to surface dismissal; claim no longer renders on the surface after refresh.
3. **`dos573_fixture_3_click_bound_corroborate.rs`**
   User clicks `corroborate`; maps to `ConfirmCurrent`; trust/user-confirmation state updates.
4. **`dos573_fixture_4_click_bound_contradict.rs`**
   User clicks `contradict`; maps to `MarkFalse`; active read surfaces stop rendering the claim.
5. **`dos573_fixture_5_save_diff_no_feedback.rs`**
   Gutenberg attribute diff without affordance click does not request nonce and does not submit feedback.
6. **`dos573_fixture_6_computed_from_refused.rs`**
   W4-D route is `ComputedFrom`; rendered HTML exposes no feedback affordance; forced POST is rejected before nonce issue.
7. **`dos573_fixture_7_display_only_refused.rs`**
   W4-D route is `DisplayOnly`; rendered HTML exposes no feedback affordance; forced POST is rejected before nonce issue.
8. **`dos573_fixture_8_source_only_refused.rs`**
   Source binding without explicit FeedbackTarget renders provenance but no feedback route.
9. **`dos573_fixture_9_feedback_target_zero_refs.rs`**
   FeedbackTarget with zero claim refs exposes no nonce-consuming affordance and forced POST rejects.
10. **`dos573_fixture_10_ambiguous_receiver_refused.rs`**
    Multiple incompatible receivers for one field reject with no nonce issue.
11. **`dos573_fixture_11_missing_route_refused.rs`**
    Field path absent from `edit_routes` rejects; no runtime feedback call.
12. **`dos573_fixture_12_post_edit_capability_refused.php`**
    Logged-in user has `edit_posts` but not `edit_post` for `post_id`; WP 403 returns before nonce issue and no runtime call occurs.
13. **`dos573_fixture_13_browser_wp_user_ignored.rs`**
    Browser asserts a different `wp_user_id`; PHP ignores/rejects it and uses current user only.
14. **`dos573_fixture_14_wrong_user_bridge_precondition.rs`**
    Signed runtime request body has `wp_user_id` mismatch; W4-B section 17 returns `403 wrong_user` before nonce verify.
15. **`dos573_fixture_15_missing_nonce.rs`**
    Runtime feedback POST without nonce rejects and writes no feedback row.
16. **`dos573_fixture_16_expired_nonce.rs`**
    W4-E returns `403 expired`; W5-A does not retry automatically.
17. **`dos573_fixture_17_replayed_nonce.rs`**
    Two submits with same nonce: one apply, one `403 replayed`; no duplicate feedback effect.
18. **`dos573_fixture_18_mismatched_field_nonce.rs`**
    Nonce bound to one field is submitted for another; `403 wrong_field`; no write.
19. **`dos573_fixture_19_mismatched_action_nonce.rs`**
    Nonce bound to `correct` is used for `dismiss`; `403 mismatched_action`; no write.
20. **`dos573_fixture_20_stale_claim_409.rs`**
    Claim advances after render; feedback returns 409 stale watermark; WP refreshes with scope-filtered correction.
21. **`dos573_fixture_21_stale_composition_409.rs`**
    Composition advances after render; feedback returns 409 stale composition; WP refreshes and requires new click.
22. **`dos573_fixture_22_mid_flight_423.rs`**
    Mutation lock held; response includes `retry_after_event`; W5-A waits through DOS-589; no tight loop.
23. **`dos573_fixture_23_tamper_422.rs`**
    Projection signature mismatch and stale version both true; 422 `ProjectionTampered` wins over 409; no correction payload and no sensitive ids in UI.
24. **`dos573_fixture_24_rollback_422.rs`**
    Signed older projection and stale composition both true; 422 rollback wins over 409.
25. **`dos573_fixture_25_missing_expected_version_400.rs`**
    Missing `claim_version` in rendered route or request rejects before feedback write.
26. **`dos573_fixture_26_rate_limited_429.rs`**
    Feedback write budget exhausted; UI surfaces retry state; no automatic replay.
27. **`dos573_fixture_27_scope_redacted_409.rs`**
    Stale correction is out of SurfaceClient scope; response has `scope_redacted: true`; WP does not direct-fetch claim body.
28. **`dos573_fixture_28_audit_submit_apply_reject.rs`**
    Submit, apply, runtime reject, and local WP reject paths each emit safe audit or local log details.
29. **`dos573_fixture_29_no_raw_nonce_persisted.rs`**
    Search serialized post content, block attributes, logs, and audit details; raw nonce absent.
30. **`dos573_fixture_30_no_direct_claim_write_from_wp.rs`**
    Static gate finds no SQL claim writes or direct substrate writes in WP plugin source.
31. **`dos573_fixture_31_runtime_service_boundary.rs`**
    Runtime handler calls service entry; bridge code does not execute feedback SQL directly.
32. **`dos573_fixture_32_block_rerender_corrected_state.rs`**
    Real account-overview fixture click applies feedback, receives post-commit `composition_version`, and re-renders the block with current state before loading clears.
33. **`dos573_fixture_33_wp_rest_nonce_missing.php`**
    Missing `X-WP-Nonce` returns WP 403 before nonce issue and before any runtime call.
34. **`dos573_fixture_34_options_schema_minimal.php`**
    REST `OPTIONS` response exposes only the typed request schema and no internal field names, claim ids, or scope tokens.
35. **`dos573_fixture_35_malformed_body_rejected.php`**
    Malformed or wrong-typed body fails args validation before `handle_feedback` and before nonce issue.

## CI invariants

1. **No save-diff feedback gate.**
   Static test fails if `class-dailyos-feedback-router.php` reads arbitrary Gutenberg save diffs to synthesize feedback.
2. **WP REST route gate.**
   Test asserts `/wp-json/dailyos/v1/feedback` is registered with a permission callback.
3. **WP REST nonce gate.**
   Fixture asserts missing or invalid `X-WP-Nonce` fails with WP 403 before nonce issue or runtime call.
4. **Post-bound capability gate.**
   Fixture asserts `current_user_can('edit_post', post_id)` is required and `edit_posts` alone is insufficient.
5. **WP REST args schema gate.**
   Tests assert validation lives in args callbacks for every request field and malformed bodies never enter `handle_feedback`.
6. **No browser-to-runtime gate.**
   JS tests assert browser calls WP REST only.
7. **No nonce persistence gate.**
   Serialization tests assert no nonce in post content, block attrs, local storage, audit, or logs.
8. **No refused affordance DOM gate.**
   Rendered HTML tests assert no affordance DOM is emitted for ComputedFrom, DisplayOnly, Source-only, zero-ref FeedbackTarget, missing, unknown, or ambiguous routes.
9. **Edit-route gate.**
   Tests assert only W4-D `FeedbackTarget feedback_allowed=true` routes can submit.
10. **Runtime route authority gate.**
    Runtime tests assert `/v1/surface/feedback` re-resolves signed `edit_routes` and refuses non-FeedbackTarget submissions even when JS/PHP metadata claims eligibility.
11. **Runtime-client method gate.**
    PHP tests assert `DailyOS_Runtime_Client::submit_feedback` signs the exact JSON body sent to runtime.
12. **Runtime-client nonce issue gate.**
    PHP tests assert `DailyOS_Runtime_Client::issue_nonce(array $context): array` signs with the same W3-B HMAC canonicalization as `submit_feedback`.
13. **SurfaceClient route-owner gate.**
    Runtime route registration for `/v1/surface/feedback` lives in `src-tauri/src/bridges/surface_client.rs`.
14. **Session-bound user gate.**
    Runtime feedback tests assert W4-B section 17 rejects wrong `wp_user_id` before nonce verify or claim read.
15. **Nonce topology gate.**
    Static and runtime tests assert PHP never calls `/v1/surface/nonce/verify`; `/v1/surface/feedback` atomically verifies-and-consumes at the W4-E nonce store.
16. **409 response shape gate.**
    Tests assert 409 WP REST response includes `outcome: "refresh_required"` and `corrected_html`, not raw claim JSON.
17. **No scope elevation refresh gate.**
    Static/runtime tests assert W5-A 409 refresh and DOS-589 `correction_ref` lookup use `Actor::SurfaceClient` with original scopes, never `Actor::System` or elevated scopes.
18. **Bridge precedence gate.**
    Pairwise tests assert W4-B V9 precedence for tamper, rollback, missing version, mid-flight, overflow, stale claim, and stale composition.
19. **Error-state copy gate.**
    UI tests assert each BridgeSurfaceError maps to the section 7.5 pattern and copy, including no permission-language for sensitivity refusals.
20. **Affordance label gate.**
    JS/DOM tests assert primary feedback affordances expose only `Looks right`, `Mark wrong`, `Fix this`, and `Hide`; raw action verbs do not appear as visible labels.
21. **Accessibility gate.**
    Tests assert visible-name matching accessible names, Enter/Space activation, focus preservation through state changes, `role="status"` announcements, trust-band differentiation via `data-ds-state` plus iconography, and inline-editor focus trapping until submit or cancel.
22. **REST enumeration gate.**
    OPTIONS tests assert only typed request shape is exposed and no internal field names, concrete claim ids, or scope tokens are listed.
23. **Service-boundary gate.**
    Grep/static test fails direct DB writes from WP feedback router and runtime bridge handler.
24. **Audit safety gate.**
    Tests assert feedback audit and WP local reject details contain no raw nonce, raw corrected text, HMAC key, bearer token, post content, raw block payload, customer names, domains, emails, or signed payload.
25. **DOS-589 wait gate.**
    423 fixture asserts W5-A waits on `retry_after_event` and does not retry feedback in a tight loop.
26. **L1 commands.**
    `cargo clippy -- -D warnings && cargo test && pnpm tsc --noEmit`.

## Interlocks

| Upstream/downstream | W5-A dependency | W5-A obligation |
|---|---|---|
| W3-B runtime client | PHP HMAC transport and pairing/session material | Add `submit_feedback`; expose `issue_nonce(array $context): array` with HMAC signing identical to `submit_feedback`; surface DOS-565 Linear comment for W3-B owner ack; never expose secrets to JS |
| W4-A renderer | Visible inline affordances, `IntelligenceCorrection` placement, and required data attrs | Consume delegated clicks from `feedback.js`; require W4-A `render.php` attrs; do not render refused-route affordance DOM |
| W4-A0 producer | Recomposition after success, stale refresh, and rollback refresh | Reinvoke with the same SurfaceClient scopes as original render; do not synthesize block state locally |
| W4-B | Watermarks, CAS, bridge errors, scope-filter correction, session-bound `wp_user_id`, route owner | Preserve versions and error precedence; route through services |
| W4-C | Projection signatures, tamper and rollback detection | Treat 422 as non-retryable until projection is refreshed/cleared |
| W4-D | `ProjectedComposition.edit_routes` and refusal reasons | Route only explicit FeedbackTarget routes; fail closed otherwise |
| W4-E | Nonce issue, atomic verify-and-consume, expected-version return, and future WP nonce endpoint | Request nonce inside PHP submit path; never expose nonce to JS; never call verify from PHP; delegate verify to `/v1/surface/feedback` |
| DOS-589 | Cursor replay, retry-after delivery, scope-filter dispatch | Wait on 423 cursor and refresh on delivered version events |
| W5-C | Negative fixture catalog | Hand off all `dos573_fixture_*` fixtures for bundle consolidation |
| W6-A | Forensic/audit trace | Emit enough safe runtime audit and WP local reject detail for originator trace by `wp_user_id`, `feedback_request_id`, incident id, and SurfaceClient instance |

## What W5-A explicitly does NOT own

- W4-A visual design for affordance placement beyond necessary feedback states.
- W4-A0 producer logic or composition authoring.
- W4-B `commit_claim`, `commit_composition`, version tables, or bridge error definitions.
- W4-C Ed25519 signing, ledger, quarantine, or tamper clearance.
- W4-D projection policy or edit-route generation.
- W4-E nonce storage, verify semantics, TTL, or invalidation.
- DOS-589 dispatcher implementation, replay ordering, or subscription transport.
- New feedback tables.
- New claim lifecycle semantics.
- New trust-scoring model.
- Multi-block feedback beyond `dailyos/account-overview`.
- Bulk feedback operations.
- Browser direct runtime transport.
- Direct WP database writes to claim/substrate state.
- Markdown-as-input reconciliation.
- MCP-specific feedback UX.

## Open questions

All V2 questions are closed unless L0 reviewers reopen them.

| ID | Resolution |
|---|---|
| Q1: click-bound vs save-diff | Click-bound only. DOS-573 and W4-E section 8 settle this. |
| Q2: WP endpoint | `POST /wp-json/dailyos/v1/feedback`. |
| Q3: runtime endpoint | `POST /v1/surface/feedback` in `bridges/surface_client.rs`. |
| Q4: routability source | W4-D `edit_routes`; no WP inference. Runtime replays route authority server-side. |
| Q5: Source role feedback | Source alone is not enough; requires explicit FeedbackTarget. |
| Q6: zero-ref FeedbackTarget | Refused in v1.4.2 and no affordance DOM is emitted. |
| Q7: dismiss semantics | `dismiss` maps to `SurfaceInappropriate`; global tombstone needs a future packet and distinct UX. |
| Q8: 423 handling | DOS-589 cursor wait; no tight retry. |
| Q9: 422 handling | Tamper/rollback banner; no retry until W4-C clears/refreshed projection arrives. |
| Q10: audit value payload | Do not include raw corrected text in audit detail for V2. |
| Q11: cross-packet nonce UX | W4-E section 707's WP-visible nonce endpoint is reserved for future decoupled nonce surfaces. DOS-573 does not call it; nonce issue happens inside PHP submit immediately before `/v1/surface/feedback`, and JS never receives a nonce. |

## Linear dependency edges

- DOS-573 is blocked by DOS-565 (W3-B runtime client + HMAC).
- DOS-573 is blocked by DOS-567 (W4-B concurrency/bridge contract).
- DOS-573 is blocked by DOS-568 (W4-A0 producer).
- DOS-573 is blocked by DOS-569 (W4-C tamper/signature contract).
- DOS-573 is blocked by DOS-570 (W4-D edit-routing projection).
- DOS-573 is blocked by DOS-571 (W4-E nonce lifecycle).
- DOS-573 is blocked by DOS-572 (W4-A renderer).
- DOS-573 is blocked by DOS-589 for 423 retry-after and signal dispatch behavior.
- DOS-573 blocks W5-C fixture consolidation for DOS-573-specific cases.
- DOS-573 feeds W6-A audit forensic exercise.

## L0 reviewer panel runners

- `/plan-eng-review` for architecture, data flow, service boundaries, and interlocks.
- `/cso` because W5-A crosses WP/browser/PHP/runtime trust boundaries and handles nonce-gated feedback writes.
- `/plan-devex-review` because W5-A defines WP REST and PHP runtime-client behavior that future block authors must use.
- `/plan-design-review` because the feedback affordance UX and error states are user-visible.
- `/codex challenge` for adversarial review of nonce bypasses, route inference, stale/tamper precedence, and audit leakage.

Unanimous approval is required for L0 closure.

## Acceptance for L0 closure

This packet is L0-approved when:

1. eng, cso, devex, design, and codex all approve the V2 packet or a successor revision.
2. DOS-573 Linear description links to this packet.
3. DOS-573 dependency edges are updated to include W4-A, W4-B, W4-C, W4-D, W4-E, W3-B, and DOS-589.
4. W4-D refusal matrix is copied into DOS-573 acceptance or linked as inherited acceptance.
5. W4-E nonce tuple and click-bound lifecycle are copied into DOS-573 acceptance or linked as inherited acceptance.
6. W4-B V9 bridge precedence is copied into DOS-573 acceptance or linked as inherited acceptance.
7. DOS-589 cursor dependency is explicit for 423 behavior.
8. W5-C fixture catalog receives the `dos573_fixture_*` list.
9. Cycle-1 review findings are folded into this V2 dated packet revision and the changelog names each category.
10. No code is started from this packet until L0 closes or the issue owner explicitly accepts a conditional start.

W5-A implementation starts after L0 closure and after its blockers have landed enough for real integration.
The implementation is done only when a real click through the account-overview block produces a nonce-bound feedback write, updates the substrate through services, emits safe audit, and re-renders the block with corrected current state.

## Changelog

- **V1 (2026-05-13):** Initial L0 packet for DOS-573.
- Grounded on W4-B V9, W4-D V3, W4-E V2, W3-B/W2 transport contracts, DOS-589 dispatcher contract, and DOS-573 issue text.
- Resolved the click-bound model as explicit affordance feedback, not save-diff inference.
- Pinned `/wp-json/dailyos/v1/feedback` as the WP REST endpoint and `/v1/surface/feedback` as the runtime endpoint.
- Pinned W5-A consumption of `ProjectedComposition.blocks[].edit_routes` from W4-D.
- Pinned W4-B V9 `BridgeSurfaceError` precedence as the runtime rejection order W5-A must preserve.
- Pinned W4-E nonce acquisition before feedback submission and runtime-side nonce verify before feedback commit.
- Pinned 409, 423, and 422 handling as distinct UI/runtime paths.
- Pinned `edit_posts` as the initial WordPress capability gate before V2 post-bound tightening.
- Pinned audit emission on every submit and every authenticated reject through `emit_surface_audit`.
- Recorded that this worktree had no existing WP `register_rest_route` or PHP `DailyOS_Runtime_Client::submit_feedback` implementation to reuse.
- **V2 (2026-05-13):** Folded cycle-1 review findings from Codex, Eng, CSO, DevEx, and UI-Design in place.
- Codex findings: tightened WP authz to `edit_post(post_id)`, required `post_id`, bound request fields to current signed post/block projection, pinned canonical nonce UX, prohibited PHP direct verify, and added Q11 for W4-E section 707.
- Eng findings: added rendered-HTML no-affordance gate, post-success W4-A0 re-render before loading clear, WP local reject logs for W6-A correlation, closed dismiss mapping to `SurfaceInappropriate`, and explicit W5-A handler registration scope.
- CSO findings: added WP REST nonce gate, runtime-side route refusal, no scope elevation on 409/DOS-589 refresh, tamper UI redaction, W4-E atomic nonce store ownership, minimal OPTIONS schema, `feedback_request_id` trust boundary, and fixtures 33-35.
- DevEx findings: added W3-B `issue_nonce` obligation plus DOS-565 owner-ack comment, `feedback.js` integration/data attrs, `DailyOS_Feedback_Router` class skeleton, REST args schema, and 409 `corrected_html` response contract.
- UI-Design findings: added approved affordance labels, DS error-state map, inline `IntelligenceCorrection` placement, 30 second undo, loading-state behavior, sensitivity refusal copy, and accessibility pins.
- **V3.1 (2026-05-13):** Cycle 3 codex confirmation pass — closed two stale-contract drifts after V3 §6.3 landed:
  - Success response body shape (§2 example) now includes `undo_token` alongside `undo_expires_at`. Documented `undo_token` as opaque base64, substrate-only interpreter, advisory expiry on the wire vs HMAC-payload-enforced expiry server-side.
  - AC #43 + #44 rewritten to match §6.3 V3 contract: AC #43 names `undo_token` + advisory `undo_expires_at`, AC #44 names the undo POST + HMAC verify path + closed-mapping inverse derivation + explicit "no fresh W4-E presence nonce" rule (the V3 §6.3 boundary).

- **V3 (2026-05-13):** Folded cycle-2 codex CONDITIONAL findings. Material changes:
  - **HIGH fix (codex C2):** §6.1 added — `SurfaceInappropriate` requires non-empty `surface` metadata per substrate validator at `claims.rs:5586`. W5-A wire schema does NOT accept a client-supplied surface field; runtime constructs `surface` server-side from the closed `block_type → surface_kind` map sourced from W4-A0 V5 §6.3. Unknown block type fails closed with `422 InvalidBlockSurface` rather than persisting a SurfaceInappropriate event with empty surface. Audit emission for dismiss carries the constructed surface kind.
  - **MEDIUM fix (codex C2):** §6.3 Undo window rewritten as a full server-authoritative contract — opaque `undo_token` returned in submit response (HMAC over `feedback_event_id || actor_session_id || expires_at` signed by W4-E HMAC key), single-use enforced via server-side consumed-tokens table (TTL 60s, replay margin), 30-second window enforced via `expires_at` inside HMAC payload (UI-visible expiry advisory), inverse mutation derived server-side via closed mapping (`ConfirmCurrent/MarkFalse → revoke`, `NeedsNuance → revoke-with-tombstone`, `SurfaceInappropriate → revoke-dismiss`), W4-B precedence/concurrency re-runs for the inverse mutation, distinct `feedback_undone` audit event with original feedback_event_id and inverse action.
  - Browser cannot construct, derive, or interpret the `undo_token` — opaque bytes only.

