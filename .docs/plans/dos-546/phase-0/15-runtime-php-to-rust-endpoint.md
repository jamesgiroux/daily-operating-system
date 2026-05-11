---
status: spec:ready
date: 2026-05-10
spike: DOS-546
phase: 0
wave: 2
artifact: 15
related_adrs: [0102, 0105, 0111, 0128, 0129, 0130]
open_questions: see ./INDEX.md (routed to W2-A L0 Prep)
---
# 15 — Runtime HTTP endpoint: WP plugin ↔ Rust runtime
## Summary
The DailyOS Rust runtime exposes a *minimal*, *loopback-bound*, *surface-paired* HTTP endpoint that the WordPress plugin invokes to:
- Run DailyOS abilities and receive a Composition.
- Submit feedback events from the Gutenberg save handler.
- Negotiate pairing and key derivation.
This endpoint is NOT the MCP head (ADR-0128) and NOT a remote-accessible API.
It is a transport for the WP-as-renderer surface only.
WordPress is treated as a paired `SurfaceClient` under ADR-0111, not as the user, an agent, or a privileged substrate process.
The runtime remains authoritative for ability lookup, `AbilityPolicy` enforcement, scope validation, feedback application, provenance, audit logging, and composition generation.
The WP plugin remains responsible for WP capability checks, server-side request signing, block rendering, and translating explicit editor actions into feedback events.
This artifact specifies endpoint shapes, request/response schemas, error model, audit log structure, and binding/transport rules.
It is not Rust implementation.
ADR-0129 and ADR-0130 may not exist on this branch; this spec treats them conceptually.
## Bind + lifetime
- Bind only to `127.0.0.1:<random_free_port>`.
- The runtime chooses a randomized free port on each startup.
- No fixed port, no `localhost` bind, no `0.0.0.0`, no `::1`, and no IPv6 dual-bind.
- The listener exists only while the runtime process is alive.
- Runtime teardown kills the listener.
- Pairing must re-handshake after runtime restart because the port and session keys change.
- Port advertisement to the WP plugin happens only through the pairing handshake.
- The live port must not be written to a persistent file readable by the WP-side process at arbitrary times.
- The endpoint uses loopback HTTP for Phase 1; TLS is deferred because this is not a remote transport.
Every request must carry:
```http
Host: 127.0.0.1:<port>
```
Reject any request whose `Host` header is absent or does not normalize to `127.0.0.1:<bound_port>`.
Rejected examples:
- `Host: localhost:<port>`
- `Host: [::1]:<port>`
- `Host: 0.0.0.0:<port>`
- `Host: 127.0.0.1:<different-port>`
Reject any request whose `Origin` header indicates a browser per ADR-0111 §8.
**Origin guard, PHP-curl-primary** (reconciled with W2-A at L0 cycle-3):
- Primary accept path: empty or absent `Origin` — the canonical caller is the WP plugin's PHP runtime client (`class-dailyos-runtime-client.php` per W3-B), which makes server-side cURL requests with no `Origin` header.
- Defense-in-depth backup: `Origin` whose origin (scheme+host+port) exactly matches the paired site's `site_url` captured at first-pair is also accepted. This backup keeps the door open for narrow PHP-side proxies that some hosting stacks inject; it is NOT an invitation to browser-direct calls.
- Reject all other `Origin` values. Reject `Origin: null` explicitly.
The WP plugin's server-side PHP client should not send `Origin`; if a hosting-stack proxy attaches one, the `site_url`-match backup keeps the request acceptable.
Host and Origin checks happen before auth, rate limits, ability lookup, or feedback handling.
## Pairing
Pairing is a user-mediated admin flow:
1. The runtime UI displays a one-time pairing code.
2. The WordPress admin enters the code in the WP plugin.
3. The WP plugin calls `POST /v1/pairing/handshake`.
4. The runtime validates the code and issues session material.
The pairing code is displayed once, short-lived, single-use, bound to the runtime process instance, invalid after runtime restart, and invalid after too many failed attempts.
Pairing creates or refreshes:
- `surface_client_id`
- short-lived bearer token
- per-session HMAC key
- endpoint version
- granted scopes
- WP-allowlisted ability listing
Pairing grants DailyOS scopes, not all host-OS or WordPress privileges.
### `POST /v1/pairing/handshake`
Purpose: initiated by the WP plugin during admin pairing flow.
Auth: none before successful handshake.
Rate limit: artifact 09, very low burst per loopback peer and pairing code.
Request:
```json
{
  "pairing_code": "123-456",
  "wp_context": {
    "wp_user_id": 42,
    "wp_site_id": "site_01JZ8WQ4P8GQ2M7K9S0N1B2C3D",
    "post_id": null,
    "request_id": "018f4e8f-7f57-7c7c-9cb4-08fc4b753b92"
  },
  "client": {
    "kind": "wordpress",
    "plugin_version": "0.1.0",
    "wp_version": "6.6",
    "php_version": "8.3",
    "site_url_hash": "sha256:9c4a1d42"
  }
}
```
Response `200`:
```json
{
  "surface_client_id": "sc_wp_01JZ8WQ7K64QH2F1Q6CVG4J17V",
  "token_type": "Bearer",
  "access_token": "opaque-short-lived-token",
  "access_token_expires_at": "2026-05-10T22:24:00Z",
  "hmac": {
    "key_id": "hmac_01JZ8WRJAX20HQ6HSJG8M4JFAA",
    "key": "base64url-session-key",
    "alg": "HMAC-SHA256",
    "signature_version": 1
  },
  "endpoint": {
    "base_url": "http://127.0.0.1:49231",
    "version": "v1"
  },
  "grants": {
    "scopes": [
      "read.composition",
      "invoke.ability",
      "submit.feedback",
      "issue.presence_nonce",
      "manage.pairing"
    ]
  },
  "abilities": [
    {
      "name": "prepare_meeting",
      "category": "Transform",
      "input_schema": {},
      "output_schema": {},
      "required_scopes": ["invoke.ability", "read.composition"],
      "mcp_exposure": false,
      "surface_return": "composition"
    }
  ]
}
```
Errors:
- `401 pairing_code_invalid`
- `401 pairing_code_expired`
- `401 pairing_code_consumed`
- `403 browser_origin_forbidden`
- `403 host_invalid`
- `409 already_paired`
- `429 rate_limited`
- `500 runtime_error`
### `GET /v1/pairing/status`
Purpose: returns pairing health.
Auth: bearer token, HMAC, and `X-DailyOS-SurfaceClient` required.
Rate limit: artifact 09, per site and per SurfaceClient.
Response `200`:
```json
{
  "paired": true,
  "surface_client_id": "sc_wp_01JZ8WQ7K64QH2F1Q6CVG4J17V",
  "last_activity_at": "2026-05-10T22:20:31Z",
  "access_token_expires_at": "2026-05-10T22:24:00Z",
  "key_rotation_needed": false,
  "runtime_version": "1.4.0-dos546",
  "endpoint_version": "v1"
}
```
Errors:
- `401 auth_missing`
- `401 token_invalid`
- `401 signature_invalid`
- `403 surface_client_mismatch`
- `429 rate_limited`
- `500 runtime_error`
## Authentication on every non-handshake call
Three layers are required on every non-handshake call:
1. Bearer token in `Authorization: Bearer <token>`.
2. HMAC signature per artifact 08.
3. SurfaceClient identity header `X-DailyOS-SurfaceClient: <id>` per ADR-0111.
Missing any layer returns `401 auth_missing`.
Required headers:
```http
Authorization: Bearer <token>
X-DailyOS-SurfaceClient: sc_wp_01JZ8WQ7K64QH2F1Q6CVG4J17V
X-DailyOS-Timestamp: 2026-05-10T22:20:31Z
X-DailyOS-Nonce: nonce_01JZ8WSH1...
X-DailyOS-Signature: v1:key_id:base64url(signature)
Content-Type: application/json
```
Verification order:
1. Bind-level checks: listener address, `Host`, `Origin`.
2. Bearer token presence, expiry, revocation, and token binding.
3. SurfaceClient header presence and match to token binding.
4. HMAC timestamp, nonce, and signature.
5. Rate limit.
6. Route-specific policy and payload validation.
Artifact 08 owns the final signing string.
This endpoint requires the signature to cover at least method, path/query, timestamp, nonce, SurfaceClient id, and SHA-256 of the exact request body bytes.
The runtime rejects signatures over normalized or reserialized JSON.
Nonce replay returns `401 signature_replay`.
Possession of bearer token alone is insufficient.
Possession of bearer token plus HMAC key is still insufficient for feedback writes without a user-presence nonce.
## Shared `wp_context`
All ability, feedback, and nonce requests carry `wp_context`:
```json
{
  "wp_user_id": 42,
  "wp_site_id": "site_01JZ8WQ4P8GQ2M7K9S0N1B2C3D",
  "post_id": 123,
  "request_id": "018f4e8f-7f57-7c7c-9cb4-08fc4b753b92"
}
```
Rules:
- `wp_user_id` is required for editor-originated calls.
- `wp_site_id` is required after pairing.
- `post_id` is nullable.
- `request_id` is caller-generated when possible and echoed by the runtime.
- If `request_id` is absent, the runtime generates one.
- Raw `wp_user_id` is never logged; audit stores a hash prefix.
## Route table
### `POST /v1/abilities/{name}/invoke`
Purpose: invoke a DailyOS ability and return a Composition or claim-shaped JSON.
Related artifacts: 02, 08, 09, 12, 13, 14.
Policy checks:
- Ability exists in the ADR-0102 registry.
- `AbilityPolicy.allowed_actors` includes `SurfaceClient`.
- Ability is allowlisted for the WordPress surface.
- Requested `scope` is permitted by the ability policy.
- Paired SurfaceClient grants include every required scope.
- `args` validates against the ability input schema.
- Invocation constructs `AbilityContext` with `Actor::SurfaceClient`.
- The bridge invokes through the registry, never by importing a concrete ability function.
Request:
```json
{
  "args": {
    "meeting_id": "meet_01JZ8X1FQAYV4PX5WZJ7RW4BZM",
    "depth": "standard",
    "schema_version": 1
  },
  "scope": "read.composition",
  "wp_context": {
    "wp_user_id": 42,
    "wp_site_id": "site_01JZ8WQ4P8GQ2M7K9S0N1B2C3D",
    "post_id": 123,
    "request_id": "018f4e8f-7f57-7c7c-9cb4-08fc4b753b92"
  }
}
```
Response `200`:
```json
{
  "composition": {
    "composition_id": "comp_01JZ8X3P7PCQZFF2HPX6JHQZ4R",
    "composition_version": "12",
    "schema_version": 1,
    "blocks": []
  },
  "request_id": "018f4e8f-7f57-7c7c-9cb4-08fc4b753b92"
}
```
Alternative response `200` for an ability whose `AbilityPolicy` declares claim-shaped JSON:
```json
{
  "result": {
    "claims": [],
    "provenance_refs": []
  },
  "request_id": "018f4e8f-7f57-7c7c-9cb4-08fc4b753b92"
}
```
Response `200` with degradation per artifact 12 F-05:
```json
{
  "composition": {
    "composition_id": "comp_01JZ8X3P7PCQZFF2HPX6JHQZ4R",
    "composition_version": "12",
    "schema_version": 1,
    "blocks": []
  },
  "partial_failures": [
    {
      "block_id": "block_01JZ8X4E1K8MD3CEVPTAYT5BC9",
      "reason": "source_unavailable"
    }
  ],
  "request_id": "018f4e8f-7f57-7c7c-9cb4-08fc4b753b92"
}
```
Errors:
- `401 auth_missing`
- `401 token_invalid`
- `401 signature_invalid`
- `401 signature_replay`
- `403 scope_forbidden`
- `403 ability_not_allowlisted_for_surface`
- `403 surface_client_mismatch`
- `404 ability_not_found`
- `409 version_skew`
- `422 args_invalid`
- `429 rate_limited`
- `500 runtime_error`
`409 version_skew` returns the current composition version when known:
```json
{
  "error": "version_skew",
  "message": "The composition version is stale.",
  "current_composition_version": "13",
  "request_id": "018f4e8f-7f57-7c7c-9cb4-08fc4b753b92"
}
```
### `POST /v1/feedback`
Purpose: receive batched feedback events from the WP save handler.
Related artifacts: 02, 08, 09, 10, 11, 12, 13.
The route accepts only feedback writes: `correct`, `dismiss`, `corroborate`, and `contradict`.
It does not accept direct claim creation, raw post diffs, source-reliability mutation, or tombstoning outside the feedback path.
Each event that changes claim state must include a valid artifact 10 presence nonce.
Request:
```json
{
  "events": [
    {
      "type": "correct",
      "claim_id": "claim_01JZ8X6YRVH63CHVZ4DGHHN5FH",
      "field_path": "summary.title",
      "before": "Old title",
      "after": "Corrected title",
      "composition_version": "12",
      "presence_nonce": "presence_01JZ8X7HZSYMJ72AQB6AAKHHFZ",
      "wp_context": {
        "wp_user_id": 42,
        "wp_site_id": "site_01JZ8WQ4P8GQ2M7K9S0N1B2C3D",
        "post_id": 123,
        "request_id": "018f4e8f-7f57-7c7c-9cb4-08fc4b753b92"
      }
    }
  ],
  "wp_context": {
    "wp_user_id": 42,
    "wp_site_id": "site_01JZ8WQ4P8GQ2M7K9S0N1B2C3D",
    "post_id": 123,
    "request_id": "018f4e8f-7f57-7c7c-9cb4-08fc4b753b92"
  }
}
```
Response `200`:
```json
{
  "applied": [
    {
      "event_index": 0,
      "claim_id": "claim_01JZ8X6YRVH63CHVZ4DGHHN5FH",
      "new_state": "corrected"
    }
  ],
  "rejected": [],
  "updated_composition": {
    "composition_id": "comp_01JZ8X3P7PCQZFF2HPX6JHQZ4R",
    "composition_version": "13",
    "schema_version": 1,
    "blocks": []
  },
  "request_id": "018f4e8f-7f57-7c7c-9cb4-08fc4b753b92"
}
```
Partial rejection remains `200` when at least one event is processable:
```json
{
  "applied": [
    {
      "event_index": 0,
      "claim_id": "claim_01JZ8X6YRVH63CHVZ4DGHHN5FH",
      "new_state": "corrected"
    }
  ],
  "rejected": [
    {
      "event_index": 1,
      "reason": "nonce_invalid_consumed"
    }
  ],
  "updated_composition": null,
  "request_id": "018f4e8f-7f57-7c7c-9cb4-08fc4b753b92"
}
```
Errors:
- `401 auth_missing`
- `401 token_invalid`
- `401 signature_invalid`
- `401 signature_replay`
- `403 scope_forbidden`
- `409 version_skew`
- `422 feedback_invalid`
- `422 nonce_invalid_missing`
- `422 nonce_invalid_expired`
- `422 nonce_invalid_consumed`
- `422 nonce_invalid_mismatch`
- `429 rate_limited`
- `500 runtime_error`
### `POST /v1/presence-nonce`
Purpose: issue a user-presence nonce per artifact 10.
Related artifacts: 08, 09, 10, 11, 13.
The nonce is scoped to SurfaceClient id, WP user id, WP site id, claim id, field path, action, and composition version.
Request:
```json
{
  "claim_id": "claim_01JZ8X6YRVH63CHVZ4DGHHN5FH",
  "field_path": "summary.title",
  "action": "correct",
  "composition_version": "12",
  "wp_context": {
    "wp_user_id": 42,
    "wp_site_id": "site_01JZ8WQ4P8GQ2M7K9S0N1B2C3D",
    "post_id": 123,
    "request_id": "018f4e8f-7f57-7c7c-9cb4-08fc4b753b92"
  }
}
```
Response `200`:
```json
{
  "nonce": "presence_01JZ8X7HZSYMJ72AQB6AAKHHFZ",
  "expires_at": "2026-05-10T22:22:31Z",
  "request_id": "018f4e8f-7f57-7c7c-9cb4-08fc4b753b92"
}
```
Errors:
- `401 auth_missing`
- `401 token_invalid`
- `401 signature_invalid`
- `403 scope_forbidden`
- `409 version_skew`
- `422 nonce_request_invalid`
- `429 rate_limited`
- `500 runtime_error`
### `GET /v1/abilities`
Purpose: enumerate abilities allowlisted for the paired SurfaceClient.
Related artifacts: 08, 09, 12, 13.
The WP plugin uses this to register abilities into WordPress's Abilities API on pairing.
Filtering is two-level per ADR-0111:
1. `AbilityPolicy.allowed_actors` must include `SurfaceClient`.
2. The paired instance must have the scopes required by the ability.
The route includes only abilities allowlisted for the WordPress surface.
It excludes the rest and must not leak excluded names, descriptions, schemas, categories, or existence.
This cross-references artifact 12 F-08 discovery leakage.
Response `200`:
```json
{
  "abilities": [
    {
      "name": "prepare_meeting",
      "description": "Prepare a publishable meeting context composition for the paired WordPress surface.",
      "category": "Transform",
      "input_schema": {},
      "output_schema": {},
      "required_scopes": ["invoke.ability", "read.composition"],
      "allowed_modes": ["Live"],
      "requires_confirmation": false,
      "mcp_exposure": false,
      "surface_return": "composition"
    }
  ],
  "request_id": "018f4e8f-7f57-7c7c-9cb4-08fc4b753b92"
}
```
Errors:
- `401 auth_missing`
- `401 token_invalid`
- `401 signature_invalid`
- `403 surface_client_mismatch`
- `429 rate_limited`
- `500 runtime_error`
### `GET /v1/healthz`
Purpose: liveness check.
Auth: none.
The route is loopback-only and low information.
It does not reveal pairing status, configured abilities, port history, grants, or SurfaceClient identity.
Response:
```json
{
  "ok": true,
  "version": "1.4.0-dos546"
}
```
Errors:
- `403 host_invalid`
- `403 browser_origin_forbidden`
- `500 runtime_error`
## Error model (uniform)
All non-2xx responses use this envelope:
```json
{
  "error": "machine_code",
  "message": "Safe human text.",
  "axis": "per_surface_client",
  "retry_after_ms": 2500,
  "request_id": "018f4e8f-7f57-7c7c-9cb4-08fc4b753b92"
}
```
Rules:
- `error` is stable and machine-readable.
- `message` is safe for WP admin UI logs.
- `axis` is populated only for `429`.
- `retry_after_ms` is populated only when retry is meaningful.
- `request_id` is always present.
- Route-specific safe fields are allowed for `current_composition_version`, `current_claim_version`, and caller-supplied `ability_name`.
Never include internal stack traces, filesystem paths, platform keychain labels, raw bearer tokens, raw HMAC keys, full provenance envelopes, or raw substrate identifiers unrelated to the request.
## Telemetry / audit log shape
Every request logs one JSONL line to the runtime audit log:
```json
{
  "ts": "2026-05-10T22:20:31.120Z",
  "request_id": "018f4e8f-7f57-7c7c-9cb4-08fc4b753b92",
  "surface_client_id": "sc_wp_01JZ8WQ7K64QH2F1Q6CVG4J17V",
  "wp_user_id_hash": "sha256:8f14e45f",
  "wp_site_id": "site_01JZ8WQ4P8GQ2M7K9S0N1B2C3D",
  "route": "POST /v1/abilities/{name}/invoke",
  "ability_name": "prepare_meeting",
  "scope": "read.composition",
  "outcome": "ok",
  "error_code": null,
  "latency_ms": 118,
  "rate_limit_axis_hit": null,
  "signature_outcome": "ok",
  "nonce_outcome": "n_a"
}
```
Field rules:
- `ts` is RFC3339 UTC from the runtime clock.
- `route` is the method plus route pattern, not a raw path containing secrets.
- `wp_user_id_hash` is a deterministic SHA-256 prefix over WP site id plus WP user id.
- `surface_client_id` is `null` only before pairing has been established.
- `signature_outcome` is `ok` or a failure code such as `missing`, `invalid`, `timestamp_invalid`, or `replay`.
- `nonce_outcome` is `ok`, a failure code, or `n_a`.
Destination:
- Runtime-local audit file under the runtime application data directory.
- Not inside WordPress.
- Not writable by the WP plugin.
- Not exposed through this HTTP endpoint.
Rotation and retention:
- Rotate at 10 MB per file.
- Keep 14 daily files.
- Keep at most 200 MB total.
- Delete oldest files first.
The audit log is not a claim.
Audit lines do not enter the claim graph by default.
Unusual patterns may emit a security signal per ADR-0105, such as repeated signature replay, browser-origin attempts, non-loopback Host attempts, nonce mismatch bursts, or ability enumeration probing.
Security signals carry summarized pattern metadata, not raw audit lines.
## Concurrency
Artifact 02 is the authority for concurrent ability invocations and feedback writes from the same SurfaceClient.
This endpoint applies bridge-level locking before ability invocation or feedback application.
Lock dimensions:
- `surface_client_id`
- `wp_site_id`
- `post_id` when present
- `composition_id` when present
- `claim_id` for feedback events
Read-only ability invocations may run concurrently when artifact 02 permits them.
Feedback writes serialize per affected claim.
Composition refreshes serialize per composition.
Stale composition state returns `409 version_skew`.
The runtime never accepts WordPress last-writer-wins behavior as authoritative substrate state.
## Failure modes summary table
| Symptom | HTTP | error code | Remediation |
|---|---:|---|---|
| Missing bearer, HMAC header, or SurfaceClient header | 401 | `auth_missing` | Re-pair if credentials are absent; fix PHP signing if headers are omitted. |
| Bearer token expired, revoked, or unknown | 401 | `token_invalid` | Re-handshake through WP admin pairing. |
| HMAC does not verify | 401 | `signature_invalid` | Match artifact 08 canonical input and exact body hash. |
| HMAC timestamp outside skew window | 401 | `signature_timestamp_invalid` | Check clocks and retry with a fresh signed request. |
| HMAC nonce reused | 401 | `signature_replay` | Generate a fresh nonce; do not retry with the same signature. |
| SurfaceClient header mismatches token binding | 403 | `surface_client_mismatch` | Clear stale WP credentials and re-pair. |
| Browser-origin request reaches endpoint | 403 | `browser_origin_forbidden` | Call through server-side WP PHP, not browser JavaScript. |
| Host is not `127.0.0.1:<port>` | 403/401 | `host_invalid` | Use the paired endpoint base URL exactly. |
| Pairing code wrong | 401 | `pairing_code_invalid` | Enter the current runtime-displayed code. |
| Pairing code expired | 401 | `pairing_code_expired` | Generate a new runtime pairing code. |
| Pairing code already used | 401 | `pairing_code_consumed` | Start a new pairing ceremony. |
| Existing pairing conflicts with handshake | 409 | `already_paired` | Revoke or rotate the existing pairing first. |
| Ability name not in registry | 404 | `ability_not_found` | Refresh `/v1/abilities` and call only enumerated abilities. |
| Ability exists but is not WordPress-allowlisted | 403 | `ability_not_allowlisted_for_surface` | Do not expose it in WP without policy change. |
| Paired instance lacks requested scope | 403 | `scope_forbidden` | Re-pair with required grants or choose a lower-scope action. |
| Ability args fail schema validation | 422 | `args_invalid` | Match the `AbilityPolicy` input schema. |
| Feedback payload invalid | 422 | `feedback_invalid` | Drop or repair malformed event before retrying. |
| Presence nonce missing | 422 | `nonce_invalid_missing` | Request `/v1/presence-nonce` from a user gesture. |
| Presence nonce expired | 422 | `nonce_invalid_expired` | Ask the user to repeat the action. |
| Presence nonce already consumed | 422 | `nonce_invalid_consumed` | Do not retry consumed events; issue a new nonce from a new gesture. |
| Presence nonce bound to different event | 422 | `nonce_invalid_mismatch` | Recreate the nonce for the exact claim, field, action, user, site, and version. |
| Presence nonce request invalid | 422 | `nonce_request_invalid` | Validate claim id, field path, action, and composition version. |
| Composition version stale | 409 | `version_skew` | Refresh composition and replay the user-visible action if still applicable. |
| Rate limit exceeded | 429 | `rate_limited` | Honor `retry_after_ms`; surface gentle retry UX in WP. |
| Unexpected runtime failure | 500 | `runtime_error` | Show sanitized failure and direct user to runtime diagnostics. |
## Interaction with other Wave 2 artifacts
- Artifact 08 (HMAC): owns canonical signing, timestamp skew, nonce replay window, and key rotation. This endpoint requires HMAC for every non-handshake route.
- Artifact 09 (rate limits): owns axes and budgets for handshake, status, invoke, feedback, nonce, and discovery. `429` includes `axis`, `retry_after_ms`, and `request_id`.
- Artifact 10 (nonce): owns user-presence nonce semantics. This endpoint issues nonces and verifies them during feedback.
- Artifact 11 (feedback routing): owns Gutenberg save-handler event mapping. This endpoint accepts the batch and returns applied/rejected partitions.
- Artifact 12 (negative fixtures): covers partial projection degradation F-05, discovery leakage F-08, browser-origin rejection, Host rejection, auth failures, and replay failures.
- Artifact 13 (WP plugin client): owns the PHP client that uses the paired base URL, signs requests, sends `wp_context`, and never calls from browser JavaScript.
- Artifact 14 (block invocation): owns block use of `/v1/abilities/{name}/invoke`, including response handling and version-skew UX.
- Wave 1 artifact 02 (concurrency): owns stale writer and concurrent mutation behavior. This endpoint maps stale composition state to `409 version_skew`.
- Wave 1 artifact 03 (tamper detection): owns projection authenticity. This endpoint returns runtime-authored compositions; WP must not promote unsigned local block mutations into trusted DailyOS state.
## Test fixtures
### Pairing happy path
Given a fresh runtime with a displayed pairing code, when WP submits the correct code to `POST /v1/pairing/handshake`, then runtime returns bearer token, HMAC key, endpoint version, SurfaceClient id, grants, and allowlisted abilities.
### Pairing replay rejected
Given a consumed pairing code, when WP submits it again, then runtime returns `401 pairing_code_consumed` and issues no new session material.
### Ability invoke happy path
Given a paired SurfaceClient with `read.composition` and `invoke.ability`, when WP signs `POST /v1/abilities/prepare_meeting/invoke` with valid args, then runtime returns `200` with `composition`.
### Ability invoke `409 version_skew`
Given WP submits stale composition version `12` and runtime current version is `13`, when concurrency validation runs, then runtime returns `409 version_skew` with `current_composition_version: "13"`.
### Feedback batched success
Given valid feedback events with fresh presence nonces, when WP submits `POST /v1/feedback`, then runtime returns all events in `applied` and an updated composition or `null`.
### Feedback partial reject
Given one valid event and one consumed nonce, when WP submits both in one batch, then runtime returns `200`, places the valid event in `applied`, and places the invalid event in `rejected`.
### Presence-nonce issue + consume
Given an explicit correction gesture, when WP calls `POST /v1/presence-nonce`, then runtime returns nonce and expiry; first feedback consumes it; replay returns `422 nonce_invalid_consumed`.
### `/v1/abilities` returns only allowlisted
Given mixed registry abilities, when WP calls `GET /v1/abilities`, then only WordPress-allowlisted `SurfaceClient` abilities with matching scopes appear.
### Browser-origin rejected
Given any request with `Origin: http://localhost:8888`, when it reaches the endpoint, then runtime returns `403 browser_origin_forbidden`.
### Non-loopback Host rejected
Given any request with `Host: localhost:<port>` or `[::1]:<port>`, when it reaches the endpoint, then runtime returns `host_invalid` and performs no ability lookup.
### Audit log line shape verified
Given any request completes, then exactly one JSONL audit line is written and it contains no bearer token, HMAC key, raw WP user id, stack trace, or unrelated raw substrate id.
## Open questions
1. Should WP runtime-restart recovery be a guided re-pairing UX or a disconnected state that polls `/v1/healthz`?
2. Is any ephemeral in-memory port handoff from runtime UI to WP admin acceptable, or must port transfer be manual during pairing?
3. What are the final bearer token and HMAC key expiration windows in artifact 08?
4. Should `/v1/pairing/status` support key rotation in Phase 1, or should rotation require full re-handshake?
5. Should feedback batches be atomic per batch or best-effort per event? This artifact chooses best-effort with applied/rejected partitions.
6. What is the final namespace for WordPress SurfaceClient scopes?
7. Should all non-empty `Origin` values be rejected, even non-browser internal clients? This artifact rejects browser-shaped origins and `null`.
8. How much of `AbilityPolicy` should `/v1/abilities` return versus a WordPress-specific projection?
9. Should audit retention be user-configurable in Phase 1 or fixed until the privacy model is reviewed?
10. How should runtime diagnostics expose repeated local attack patterns without turning audit logs into claim graph data by accident?
