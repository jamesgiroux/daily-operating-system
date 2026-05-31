---
status: spec:ready
date: 2026-05-10
spike: DOS-546
phase: 0
wave: 2
artifact: 10
related_adrs: [0102, 0105, 0111, 0129, 0130]
open_questions: see ./INDEX.md (routed to W4-F and W5-A L0 Prep)
---

# 10 — User-presence nonce lifecycle

## Summary

Substrate-mutating actions that originate in the WordPress Gutenberg editor need
a proof that the current human user is deliberately acting on the specific
substrate-authored claim field now.

HMAC proves the channel. Bearer credentials prove the paired surface session.
Rate limits prevent flooding. None of those prove that a human in the current
editor session is deliberately acting on this specific claim field right now.
The user-presence nonce closes that gap.

The nonce is required for feedback-style writes from the WordPress
`SurfaceClient`: corrections, dismissals, corroborations, and contradictions of
substrate-authored content. It is single-use, bound server-side to the action
context, valid for no more than 60 seconds, and consumed atomically on the
feedback path.

The nonce is not an authorization token by itself. It is an additional freshness
and user-presence proof that must be checked alongside the paired
`SurfaceClient` session, the bearer credential, HMAC request authentication,
ability policy, scope grants, and rate limits.

## Binding payload

The nonce is an opaque token plus a server-side binding record. The token is 32
bytes from an OS cryptographic RNG, base64url-encoded without padding.

```json
{
  "nonce": "<32B base64url>",
  "session_id": "<surface client session>",
  "wp_user_id": 123,
  "claim_id": "<uuid>",
  "field_path": "<dot-path>",
  "action": "correct",
  "composition_version": "<watermark>",
  "generated_at": "<RFC3339 UTC>",
  "expires_at": "<RFC3339 UTC>"
}
```

Allowed `action` values:

```text
correct | dismiss | corroborate | contradict
```

Field definitions:

| Field | Purpose | Attack defeated |
| --- | --- | --- |
| `nonce` | Opaque lookup key generated from 32 bytes of OS RNG. The browser receives only this value, never the binding record. | Guessing, client-side binding forgery. |
| `session_id` | Binds the nonce to one paired WordPress `SurfaceClient` session. | Cross-session theft, replay from an old or different paired session. |
| `wp_user_id` | Binds the nonce to the concrete WordPress user who initiated the action. | Cross-user submission where one admin's browser or plugin submits another user's token. |
| `claim_id` | Binds the nonce to one substrate-authored claim. | Claim redirection, where a token minted for one claim is attached to feedback for another claim. |
| `field_path` | Binds the nonce to one field within the claim or composition output. Dot paths are canonicalized before storage. | Field redirection, where feedback intended for `summary.score` is applied to `risk.reason`. |
| `action` | Binds the nonce to the exact feedback verb. | Action redirection, where a corroboration token is reused as a correction or dismissal. |
| `composition_version` | Binds the nonce to the rendered composition or claim watermark observed by Gutenberg. | Stale-tab replay and version-skew tamper after the substrate produced a newer claim version or composition projection. |
| `generated_at` | Records the runtime clock time when the nonce was issued. | Audit ambiguity and issuance backdating; supports deterministic expiry tests. |
| `expires_at` | Explicit expiry timestamp, normally `generated_at + 60s`. | Sleep-and-replay and screen-locked stale tab submissions. |

`composition_version` is the freshness watermark for the rendered claim field.
It may be a composition version, claim version, ETag-style digest, or monotonic
watermark, depending on the artifact 13/15 runtime shape. The important
property is that it changes when the rendered substrate-authored claim field is
no longer the same version the user saw.

The binding record is never serialized into the Gutenberg DOM, block attributes,
post content, REST preload state, browser local storage, or editor logs. Only the
opaque token is allowed to cross the surface boundary.

## Generation

Nonce generation happens server-side at action-initiation time.

The Gutenberg editor calls a runtime endpoint defined by artifact 15:

```http
POST /v1/presence-nonce
```

The request carries the proposed action context:

```json
{
  "session_id": "wp-site-7/session-9f1c",
  "wp_user_id": 42,
  "claim_id": "8de3a9d2-72c3-4e0c-97f7-5b8e1e71c82f",
  "field_path": "claims[0].summary",
  "action": "correct",
  "composition_version": "composition:v17:sha256:0f2c..."
}
```

Before issuing a nonce, the runtime must validate:

- The request has a valid bearer credential for the paired `SurfaceClient`.
- The request has a valid HMAC signature per artifact 08.
- The `SurfaceClient` session is active and not revoked.
- The paired instance has the scope required to submit feedback.
- The WordPress user is the user associated with the current editor session.
- The claim exists and is substrate-authored.
- The field path is canonical, known, and feedback-eligible.
- The action is allowed for that field.
- The supplied `composition_version` matches the current rendered version or an
  accepted current watermark for that claim field.
- Nonce issuance is within the rate limits from artifact 09.

If validation passes, the runtime:

1. Generates 32 random bytes with the OS cryptographic RNG.
2. Encodes the bytes as base64url without padding.
3. Creates a binding record with `generated_at = now`.
4. Sets `expires_at = generated_at + 60 seconds`.
5. Stores the binding record in the canonical nonce store.
6. Returns only the opaque nonce token and expiry metadata safe for UI timing.

Example response:

```json
{
  "presence_nonce": "YMRd1w8n1Yv9P36tFDhd3Xn6NgZ7RDoLcFPXCBWLk_8",
  "expires_at": "2026-05-10T22:31:09Z"
}
```

The DOM never sees the binding record. The browser cannot create, edit, or
extend a binding. A request that attempts to include a full binding object is
rejected as malformed.

## Transport

The Gutenberg block embeds the opaque token in two places:

1. A `data-dailyos-presence-nonce` attribute on the editable element.
2. A hidden form/input value re-sent in the action's `POST /v1/feedback` body.

The feedback submission includes both observed values:

```json
{
  "presence_nonce_attr": "YMRd1w8n1Yv9P36tFDhd3Xn6NgZ7RDoLcFPXCBWLk_8",
  "presence_nonce_body": "YMRd1w8n1Yv9P36tFDhd3Xn6NgZ7RDoLcFPXCBWLk_8",
  "claim_id": "8de3a9d2-72c3-4e0c-97f7-5b8e1e71c82f",
  "field_path": "claims[0].summary",
  "action": "correct",
  "composition_version": "composition:v17:sha256:0f2c...",
  "value": "Corrected text from the editor"
}
```

Both nonce values must match server-side before binding lookup.

Redundant transport is intentional:

- The data attribute binds the token to the concrete editable element the user
  interacted with.
- The hidden value binds the token to the submitted action payload.
- A mismatch catches client-side tamper, editor plugin bugs, copied DOM
  fragments, and event handlers that submit a token from a different field.
- The runtime still treats both values as untrusted hints; the server-side
  binding record is the source of truth.

The token must not be written into saved post content. It is editor-session
state only. If Gutenberg serializes the block, the nonce attribute must be
stripped before persistence.

## Validation

Feedback validation happens on the artifact 15 endpoint:

```http
POST /v1/feedback
```

The runtime must perform validation in this order:

1. Verify bearer credential and HMAC signature for the feedback request.
2. Verify the paired `SurfaceClient` session is active and has the required
   feedback scope.
3. Verify `presence_nonce_attr` and `presence_nonce_body` are present and equal.
4. Look up the binding by nonce.
5. Compare every field of the binding against the inbound request:
   `session_id`, `wp_user_id`, `claim_id`, `field_path`, `action`, and
   `composition_version`.
6. Check `expires_at` against the runtime clock.
7. Atomically mark the nonce consumed.
8. Apply the feedback event through the substrate's feedback path.
9. Audit the accepted mutation with the nonce id, binding fields, consumed time,
   `SurfaceClient` instance id, and rendered provenance reference.

Any mismatch rejects the request before consumption unless the implementation
chooses to consume suspicious mismatches as a defensive replay measure. If it
does, that policy must be explicit and consistent. Phase 0 recommendation:
consume only on a complete binding match and successful expiry check; log
mismatches for abuse detection.

Atomic consumption is mandatory. The store operation must behave like:

```text
UPDATE presence_nonces
SET consumed_at = now
WHERE nonce = :nonce
  AND consumed_at IS NULL
  AND expires_at >= now
```

Exactly one concurrent request may observe success. Every later request for the
same nonce is `nonce_replayed`.

## Storage and expiry

The runtime keeps two nonce data structures:

1. A DB-backed canonical store for definitive validation and single-use
   enforcement.
2. A short-lived bloom filter for opportunistic replay detection and hot-path
   abuse throttling.

The canonical store is authoritative. It contains the binding record,
`consumed_at`, `consume_request_id`, `surface_client_instance_id`, and minimal
audit metadata such as hashed IP and user-agent values. The raw nonce should not
be stored if avoidable; store a keyed hash or strong digest and compare by digest
on lookup.

The bloom filter contains recently consumed nonce digests and obvious replay
attempts. Size it for expected peak issuance plus a safety factor:

```text
capacity = peak_feedback_actions_per_second * 120
target_false_positive_rate = 0.001
```

The bloom filter is opportunistic. If it says "maybe seen" but the DB says the
nonce exists and is unconsumed, the DB wins. If the DB says consumed, expired, or
unknown, the DB result wins. Bloom false positives must never reject a valid
feedback event.

Sweep expired unconsumed rows every 30 seconds. The hot validation path must
reject expired records even before sweep. Consumed rows follow the feedback audit
retention policy because they explain accepted substrate mutations.

## Lifetime

The nonce lifetime is 60 seconds maximum.

Sixty seconds is long enough for a normal human click, small edits, brief editor
latency, and slow local network hops. It is short enough that a screen-locked
tab, abandoned editor modal, copied DOM fragment, or delayed extension replay
cannot be used when the user returns later.

The runtime may issue a shorter lifetime for high-risk contexts, but it must not
issue a longer lifetime without a new Phase 1 security review.

When a nonce expires while the user is still editing, Gutenberg must request a
fresh nonce for the same action context before submission. A refreshed nonce is a
new binding record and does not extend the old one.

## Attack scenarios defeated

- Extension replay: cannot reuse because the nonce is single-use and atomically
  consumed.
- Cross-user submission: `wp_user_id` mismatch rejects the request.
- Cross-session theft: `session_id` mismatch rejects the request.
- Stale-tab replay after version bump: `composition_version` mismatch rejects
  feedback against old substrate-authored content.
- Field redirection: `field_path` mismatch rejects the request.
- Action redirection: `action` mismatch rejects the request.
- Sleep-and-replay after 60 seconds: `expires_at` rejects delayed submission.
- Version-skew tamper: the inbound version must match the binding and the
  runtime's accepted current watermark policy.

## Failure modes

All failures return a JSON error body with `code`, `message`, `remediation`,
and `request_id`. Messages must be safe for the WordPress editor UI. Detailed
diagnostics go to server logs.

| HTTP | Code | Meaning | Remediation |
| --- | --- | --- | --- |
| `400` | `nonce_missing` | One or both nonce transport values are absent. | Re-render the feedback control and request a new nonce. |
| `400` | `nonce_transport_mismatch` | `presence_nonce_attr` and `presence_nonce_body` differ. | Discard the client submission, refresh the control, and log client tamper/bug telemetry. |
| `404` | `nonce_unknown` | No canonical binding exists for the nonce digest. | Request a fresh nonce; do not retry the same token. |
| `409` | `nonce_replayed` | The nonce was already consumed. | Surface "Already submitted" if the prior request succeeded; otherwise request a fresh nonce. |
| `410` | `nonce_expired` | The nonce is older than its 60 second lifetime. | Request a fresh nonce for the same visible action context. |
| `403` | `nonce_binding_mismatch.wrong_session` | Binding `session_id` differs from the active session. | Re-pair or refresh the editor session. |
| `403` | `nonce_binding_mismatch.wrong_user` | Binding `wp_user_id` differs from the active WordPress user. | Refresh the editor under the current user; do not reuse another user's editor state. |
| `409` | `nonce_binding_mismatch.wrong_claim` | Binding `claim_id` differs from the submitted claim. | Re-render the block from the current composition. |
| `409` | `nonce_binding_mismatch.wrong_field` | Binding `field_path` differs from the submitted field. | Re-render the field affordance and request a field-specific nonce. |
| `409` | `nonce_binding_mismatch.wrong_action` | Binding `action` differs from the submitted action. | Request a new nonce for the chosen action. |
| `409` | `nonce_binding_mismatch.wrong_version` | Binding `composition_version` differs from the submitted or current version. | Refresh the composition and ask the user to review the latest claim before acting. |

`wrong_version` should use `409 Conflict`, not `403 Forbidden`, because the user
may still be authorized; the visible content is stale.

## Interaction with other Wave 2 artifacts

- 08 HMAC: signature is still required on the request that uses the nonce. HMAC
  proves request integrity; the nonce proves fresh user presence for a specific
  field/action.
- 09 Rate limits: nonce issuance is rate-limited by `SurfaceClient` instance,
  `wp_user_id`, `claim_id`, field, and action. Nonce consumption is rate-limited
  on a separate axis so replay storms do not block legitimate issuance.
- 11 Editable-composition overlay: nonces are required only for actions on
  substrate-authored content. User-added free-form Gutenberg blocks need no
  nonce because they are not feedback events.
- 15 Runtime endpoint: implements `POST /v1/presence-nonce`, validates and
  consumes on `POST /v1/feedback`, owns storage and expiry, and emits audit
  records with `SurfaceClient` instance identity.
- ADR-0102: feedback mutations pass through the ability/runtime contract, with
  `SurfaceClient` policy and scopes deciding whether WordPress may invoke the
  path.
- ADR-0105: `field_path` must correspond to a field-level attribution path or
  stable claim field, and accepted feedback records the provenance reference for
  the substrate-authored output the user acted on.
- ADR-0111: WordPress is a `SurfaceClient`; bearer possession is insufficient
  for writes, and user-presence proof is mandatory.
- ADR-0129 and ADR-0130: the nonce binds to the rendered composition projection
  without letting WordPress mutate substrate state directly.

## Test fixtures

Use a frozen runtime clock. Baseline binding:

```json
{
  "nonce": "fixture_nonce",
  "session_id": "session-a",
  "wp_user_id": 42,
  "claim_id": "8de3a9d2-72c3-4e0c-97f7-5b8e1e71c82f",
  "field_path": "claims[0].summary",
  "action": "correct",
  "composition_version": "composition:v17",
  "generated_at": "2026-05-10T22:30:09Z",
  "expires_at": "2026-05-10T22:31:09Z"
}
```

Required fixtures:

| Fixture | Input change | Expected result |
| --- | --- | --- |
| Happy path | Matching request at `22:30:30Z`. | Success, one feedback mutation, `consumed_at` set. |
| Replay | Reuse happy-path nonce after consumption. | `409 nonce_replayed`, no second mutation. |
| Expiry | Matching request at `22:31:10Z`. | `410 nonce_expired`, no mutation. |
| Wrong session | Request uses `session-b`. | `403 nonce_binding_mismatch.wrong_session`. |
| Wrong user | Request uses `wp_user_id = 84`. | `403 nonce_binding_mismatch.wrong_user`. |
| Wrong claim | Request uses another claim UUID. | `409 nonce_binding_mismatch.wrong_claim`. |
| Wrong field | Request uses `claims[0].risk_score`. | `409 nonce_binding_mismatch.wrong_field`. |
| Wrong action | Binding is `corroborate`; request is `dismiss`. | `409 nonce_binding_mismatch.wrong_action`. |
| Wrong version | Request or current watermark is `composition:v18`. | `409 nonce_binding_mismatch.wrong_version`. |
| Transport mismatch | Attribute nonce is `nonce-a`; body nonce is `nonce-b`. | `400 nonce_transport_mismatch` before lookup. |
| Unknown nonce | Matching transport values for `never-issued`. | `404 nonce_unknown`. |
| Atomic consume race | Two identical valid requests arrive concurrently. | Exactly one success; the other is `409 nonce_replayed`. |

Simultaneous issue race fixture: issue `nonce-a` at `22:30:09Z` and `nonce-b`
at `22:30:10Z` for the same session, user, claim, field, action, and version.
Both are valid until consumed or expired. Consuming `nonce-a` must not invalidate
`nonce-b`; `nonce-b` can also be consumed if its binding still matches.

## Open questions

1. Should binding mismatches consume the nonce defensively after a complete DB
   lookup, or should only successful binding matches consume? Phase 0 recommends
   not consuming mismatches, but Phase 1 may choose the stricter abuse posture.
2. What is the canonical `composition_version` format: monotonic integer,
   ETag-style digest, claim-version tuple, or provenance invocation plus field
   watermark?
3. Should nonce issuance happen when the feedback affordance is rendered, when
   it receives focus, or only when the user starts the final submit gesture?
4. What WordPress capability must pair with DailyOS `submit.feedback` scope for
   each action type?
5. How long should consumed nonce audit records be retained relative to the
   feedback event audit log?
6. Should high-risk actions such as `contradict` use a shorter lifetime or an
   explicit re-confirmation UI?
7. Should the runtime expose a lightweight nonce refresh endpoint, or should all
   refreshes call the same `POST /v1/presence-nonce` path?
8. How should offline or temporarily disconnected Gutenberg sessions present
   expired nonce failures without making the user believe their correction was
   saved to the substrate?
