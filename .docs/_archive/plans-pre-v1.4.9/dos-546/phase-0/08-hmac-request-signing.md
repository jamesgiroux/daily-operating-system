---
status: spec:ready
date: 2026-05-10
spike: DOS-546
phase: 0
wave: 2
artifact: 08
related_adrs: [0102, 0105, 0111, 0128, 0129, 0130]
open_questions: see ./INDEX.md (routed to W2-B L0 Prep)
---

# 08 — HMAC request signing for WP-plugin → Rust runtime loopback

## Summary

This specification defines HMAC request signing for the loopback HTTP channel from the DailyOS WordPress plugin to the local DailyOS Rust runtime. It exists because the bearer/session token used by a paired WordPress `SurfaceClient` is an authorization credential, not proof that a specific HTTP request was produced by the paired plugin session. The Rust runtime must reject forged, replayed, or tampered ability invocations even when an attacker can reach `localhost` or has copied a bearer token from WordPress state.

## Threat model

This transport defends the PHP-to-Rust loopback boundary for the WordPress Studio surface described by ADR-0129 and the `SurfaceClient` actor class described by ADR-0111 §8.

Attackers this defeats:

- **Local malicious process.** A local process can open a socket to `127.0.0.1`, `::1`, or the runtime's loopback bind address and send HTTP requests. Without the derived signing key, it cannot forge a valid ability invocation.
- **Malicious WordPress plugin.** Another plugin running in the same WordPress install may read some WordPress options, call local HTTP endpoints, or attempt to imitate the DailyOS plugin. Possession of the bearer token alone is insufficient; every request also requires a fresh HMAC over the exact request bytes.
- **Browser extension that resolves localhost.** A browser extension, injected script, or web page that can cause requests to `localhost` cannot synthesize signed requests unless it also obtains the per-session signing key.
- **MITM on loopback.** A process observing or relaying loopback HTTP cannot modify method, request target, content type, body, timestamp, or nonce without invalidating the signature.
- **Replay across sessions.** A signed request from an old SurfaceClient pairing is not valid under a new session-derived key.
- **Replay within a session.** A signed request captured within the freshness window is rejected if the nonce has already been used.
- **Token-only forgery.** A stolen `Authorization: Bearer ...` value is not enough to invoke abilities because the runtime verifies the HMAC independently before constructing a `SurfaceClientBridge`.

Attackers this does not defeat:

- A root-level attacker who can read the Rust runtime process memory, PHP process memory, keychain material, or loopback traffic before signing.
- Kernel compromise, hypervisor compromise, or a malicious OS keychain implementation.
- A compromised DailyOS Rust runtime binary. The runtime is the key issuer and verifier.
- A fully compromised WordPress process after the per-session derived key has legitimately been delivered to it. This spec reduces the value of copied tokens and replayed requests; it does not make arbitrary PHP execution safe.
- User-approved ability invocations that are malicious at the product-policy layer. Ability policy, scope checks, user-presence nonces, provenance rules, and feedback-only write boundaries still apply above transport signing.

## Algorithm

Every signed request uses HMAC-SHA256 over a canonicalized request representation. The MAC is computed by the WordPress plugin using the active per-session derived key and verified by the Rust runtime before ability dispatch.

Canonicalization is byte-oriented. The signer must sign the exact request body bytes that will be transmitted, before PHP deserialization, JSON pretty-printing, HTTP client mutation, compression, or retry wrapping.

Canonical fields, in order:

1. `method` — the HTTP method converted to uppercase ASCII, for example `POST`.
2. `path_query` — the request target path including query string exactly as the runtime receives it, for example `/v1/surface/invoke?ability=briefing.daily&ability=briefing.daily`. No URL decoding, path normalization, query sorting, duplicate-key collapsing, or trailing-slash normalization is allowed.
3. `content_type` — the `Content-Type` header value exactly as sent after trimming leading and trailing ASCII whitespace. If absent, the empty string is signed. Header name casing is ignored by HTTP, but header value casing is not normalized.
4. `body` — the raw request body bytes. For an empty body, sign zero bytes.
5. `nonce` — the value of `X-DailyOS-Nonce`.
6. `timestamp` — the value of `X-DailyOS-Timestamp`, formatted as RFC3339 UTC with a trailing `Z`, for example `2026-05-10T17:20:31Z`.

Canonical serialization:

```text
DAILYOS-WP-BRIDGE-HMAC-V1\n
method:<decimal-byte-length>\n
<method-bytes>\n
path_query:<decimal-byte-length>\n
<path-query-bytes>\n
content_type:<decimal-byte-length>\n
<content-type-bytes>\n
body:<decimal-byte-length>\n
<body-bytes>\n
nonce:<decimal-byte-length>\n
<nonce-bytes>\n
timestamp:<decimal-byte-length>\n
<timestamp-bytes>\n
```

Rules:

- All labels and decimal lengths are ASCII.
- All string fields are UTF-8 bytes after the normalization rules above. Values that are not valid UTF-8 are rejected before signing or verification, except for `body`, which is arbitrary bytes.
- The line separator is exactly `\n` (`0x0a`).
- The domain separator is exactly `DAILYOS-WP-BRIDGE-HMAC-V1`.
- Each field is length-prefixed in bytes. The verifier reads exactly that many bytes for the field value, then expects the following `\n`.
- Decimal lengths have no leading plus sign, no leading zeros unless the length is exactly `0`, and no surrounding whitespace.
- No field may be omitted. Empty fields are encoded with length `0` and no value bytes before the following `\n`.
- The algorithm identifier is fixed to HMAC-SHA256 for v1. There is no client-selected algorithm negotiation.

MAC computation:

```text
signature = HMAC-SHA256(session_key, canonical_request_bytes)
signature_hex = lowercase_hex(signature)
```

`signature_hex` is 64 lowercase hexadecimal characters. Uppercase hex is rejected to keep transport diagnostics deterministic.

Path and query handling is intentionally strict. If PHP signs `/v1/surface/invoke/?a=1&a=2` and the HTTP stack sends `/v1/surface/invoke?a=1&a=2`, verification fails. That failure is classified as `canonicalization_mismatch` when the runtime can determine that the signed request was otherwise well-formed.

## Key derivation and storage

The Rust runtime is the key issuer. WordPress never creates a master signing key and never treats WordPress administrator status as substrate authority.

Master key:

- Generated by the Rust runtime at runtime startup if no active key exists.
- Exactly 32 random bytes from the OS RNG.
- Stored in the macOS keychain entry owned by the Rust runtime, or the platform equivalent on non-macOS hosts.
- Identified by an opaque `master_key_id` generated by the runtime. The key id is not a path, username, keychain label, or other environmental value.
- Never stored in WordPress options, post meta, transients, browser storage, block attributes, logs, crash reports, or exported Studio data.

WordPress-side retrieval:

- WordPress receives only a per-session derived key, never the master key.
- The derived key is retrievable WP-side only through the filter `dailyos_wp_bridge_session_key`.
- That filter returns key material only when both gates pass:
  - The current WordPress user has `manage_options`.
  - The single-use pairing handshake has completed for this exact plugin instance and session, represented by the runtime-issued handshake gate `dailyos_pairing_handshake_complete:<pairing_id>:<session_id>`.
- The handshake gate is consumed on first successful key retrieval. A second attempt with the same gate returns no key and must start a fresh pairing or session refresh.
- The filter result is process-local secret material. It must not be persisted by the plugin after the active session expires.

Trust assumption:

- The Rust runtime is trusted to generate, store, rotate, derive, and verify keys.
- WordPress is a paired `SurfaceClient` and receives only bounded session authority.
- WordPress local capabilities gate who may complete pairing and retrieve the session key, but DailyOS scopes and runtime-side pairing state remain authoritative.

## Per-session derived key

Each SurfaceClient pairing receives a per-session signing key derived from the master key:

```text
session_key = HKDF-SHA256(
  ikm = master_key,
  salt = utf8(session_id),
  info = "dailyos-wp-bridge-v1",
  length = 32
)
```

Rules:

- `session_id` is a runtime-generated opaque identifier with at least 128 bits of entropy.
- The HKDF salt is exactly the UTF-8 bytes of `session_id`.
- The HKDF info string is exactly `dailyos-wp-bridge-v1`.
- A new session key is derived on every new `SurfaceClient` pairing and every session refresh.
- The runtime does not persist the derived key if it can re-derive it from the master key and active session record.
- The WordPress plugin keeps the derived key in memory only for the active session. If PHP process lifetime makes memory-only storage unreliable, the plugin must request a fresh handshake rather than persist the key to WordPress storage.
- The derived key authorizes transport signing only. It does not expand scopes, bypass actor policy, satisfy user-presence requirements, or authorize writes.

## Signature transport

Signed requests carry these HTTP headers:

```http
Authorization: Bearer <surface-session-token>
X-DailyOS-Key-Id: <session_id>
X-DailyOS-Signature: v1=<lowercase-hex-hmac>
X-DailyOS-Timestamp: 2026-05-10T17:20:31Z
X-DailyOS-Nonce: <nonce>
```

Header semantics:

- `Authorization` carries the short-lived SurfaceClient bearer/session token.
- `X-DailyOS-Key-Id` carries the active `session_id` used to select the runtime session record and derive the verification key. It is public lookup metadata, not a secret.
- `X-DailyOS-Signature` carries only the signature value in the form `v1=<hex>`.
- `X-DailyOS-Timestamp` carries the RFC3339 UTC timestamp signed in the canonical request.
- `X-DailyOS-Nonce` carries a random, single-use value signed in the canonical request.

Nonce requirements:

- At least 128 bits of entropy.
- Encoded as lowercase hex or base64url without padding.
- Unique within the session.
- Stored runtime-side in a bounded replay window keyed by `session_id`.

The signature is a separate header rather than embedded in the bearer token because the two prove different facts. The bearer token identifies an authorized paired session and its scopes. The HMAC proves that this exact HTTP request, including the raw body bytes, was produced by a holder of the active session signing key. Keeping them separate lets the runtime rotate session signing material, reject replay, and log transport-authentication failures without minting a new bearer token format for every canonicalization change.

## Key rotation

Master key rotation:

- The master key rotates only on explicit user action: "re-pair", "disconnect and pair again", or a security recovery flow that clearly tells the user pairing material will be invalidated.
- The master key is never silently rotated during startup, crash recovery, plugin update, runtime update, or clock-skew recovery.
- A new master key creates a new `master_key_id` and invalidates all derived keys under the old master unless the user explicitly approves migration.

Derived key rotation:

- A derived key rotates on every new `SurfaceClient` pairing.
- A derived key rotates on session timeout. Phase 0 defines the timeout as 15 minutes of inactivity or 8 hours absolute lifetime, whichever comes first.
- A derived key rotates when the runtime detects a clock-skew tamper signal, including repeated future timestamps, sudden backward local-clock jumps, or impossible nonce/timestamp ordering for the same session.
- A derived key rotates when the user narrows scopes for the WordPress SurfaceClient, because transport authority should align with the active scope grant snapshot.

How WordPress learns of rotation:

- Rotation is communicated only through the pairing or session-refresh handshake response.
- The response includes the new `session_id`, new bearer/session token, absolute expiry, inactivity timeout, allowed clock skew, and the one-time handshake gate needed for `dailyos_wp_bridge_session_key`.
- WordPress must stop using the prior `session_id`, derived key, bearer token, and nonce namespace immediately after accepting the handshake response.

In-flight requests:

- Requests signed with the old derived key after rotation are rejected with `key_rotated`.
- The runtime does not accept an overlap window for old and new keys. Overlap makes replay analysis ambiguous.
- The WordPress plugin remediation is to force a fresh handshake and retry only if the ability invocation is idempotent under ADR-0102 policy.

## Verification at the runtime endpoint

The Rust endpoint verifies transport signing before ability lookup, actor-filtered discovery, scope checks, user-presence nonce checks, ability input validation, or provenance rendering.

Verification algorithm:

1. Parse `Authorization`, `X-DailyOS-Key-Id`, `X-DailyOS-Signature`, `X-DailyOS-Timestamp`, and `X-DailyOS-Nonce`. Reject malformed or missing fields before canonicalization.
2. Parse `X-DailyOS-Timestamp` as RFC3339 UTC. Reject `timestamp_stale` if it is more than 30 seconds older than runtime time. Reject `timestamp_future` if it is more than 5 seconds newer than runtime time.
3. Look up the active session record by `X-DailyOS-Key-Id`. Verify the bearer token maps to the same session id and is not expired, revoked, or scope-rotated. Return `key_not_found` or `key_rotated` before deriving a key when the session record is absent or obsolete.
4. Check the nonce replay table for the `(session_id, nonce)` pair. If present, reject `nonce_replay`. If absent, reserve the nonce as pending for the duration of verification.
5. Reconstruct the canonical request from the method, request target path including query, trimmed content type value, raw body bytes, nonce, and timestamp exactly as received by the runtime.
6. Derive `session_key` using HKDF-SHA256 from the active master key, session id, and fixed info string.
7. Compute HMAC-SHA256 over the canonical request.
8. Compare the computed signature to `X-DailyOS-Signature` using constant-time comparison.
9. On match, mark the pending nonce as consumed and continue to SurfaceClient policy enforcement.
10. On mismatch, mark the pending nonce as consumed for the freshness window to prevent brute-force retry with the same nonce, then return `signature_invalid` or `canonicalization_mismatch` if request-shape diagnostics identify canonicalization drift.

Why this order:

- Timestamp freshness is cheapest and bounds replay storage pressure before the runtime touches nonce or HMAC state.
- Session/key lookup comes before nonce storage so unknown sessions cannot fill per-session replay tables.
- Nonce single-use comes before HMAC compare so replayed valid signatures are rejected without recomputing expensive work and so replay attempts are visible as replay, not generic signature failures.
- Constant-time comparison is still mandatory for the final signature check to avoid leaking partial MAC information.

## Failure modes

All failures return JSON with this shape:

```json
{
  "error": {
    "code": "signature_invalid",
    "message": "DailyOS request signing failed.",
    "request_id": "req_01JZ8WQ4P8GQ2M7K9S0N1B2C3D",
    "remediation": "refresh_session"
  }
}
```

Log line shape:

```text
dailyos.wp_bridge.signing failure code=<code> request_id=<request_id> session_id=<redacted-or-prefix> pairing_id=<redacted-or-prefix> surface=wordpress remote_addr=<addr> method=<method> path_hash=<sha256> reason=<reason>
```

The runtime must never log bearer tokens, derived keys, master keys, raw signatures, raw nonces, or raw request bodies.

| Failure | HTTP status | Error code | Log reason | Remediation |
|---|---:|---|---|---|
| Missing or malformed signature header, wrong version prefix, wrong hex length, non-lowercase hex, or HMAC mismatch | 401 | `signature_invalid` | `hmac_compare_failed` or `malformed_signature` | WordPress discards the current request, refreshes the session if repeated, and shows a re-pair prompt after consecutive failures. |
| Timestamp older than 30 seconds | 401 | `timestamp_stale` | `timestamp_age_gt_30s` | WordPress refreshes session time state and retries idempotent requests once with a fresh timestamp and nonce. |
| Timestamp more than 5 seconds in the future | 401 | `timestamp_future` | `timestamp_ahead_gt_5s` | WordPress checks local clock, stops automatic retries, and prompts the user if repeated future timestamps trigger clock-skew tamper handling. |
| Nonce already consumed for the active session | 409 | `nonce_replay` | `nonce_seen` | WordPress must generate a new nonce. The runtime counts this as a replay signal and never retries the same request. |
| Session id has no active key, master key is unavailable, session has expired, or key was rotated | 401 | `key_not_found` or `key_rotated` | `session_missing`, `master_unavailable`, `expired`, or `rotated` | WordPress starts a fresh handshake. For `key_rotated`, in-flight requests are not retried unless idempotent. |
| Body, path, query, or content-type changed between signing and runtime receipt | 400 | `canonicalization_mismatch` | `canonical_request_drift` | WordPress must sign after final body serialization and must not let the HTTP client rewrite the signed request target or content type. |

Additional handling:

- `signature_invalid`, `timestamp_stale`, `timestamp_future`, `nonce_replay`, and `canonicalization_mismatch` are transport-authentication failures and count against the rate-limit budget in artifact 09.
- `key_not_found` can be benign after reinstall or timeout, but repeated failures from the same SurfaceClient are suspicious and should be elevated to pairing recovery diagnostics.
- `canonicalization_mismatch` is returned only when the runtime can classify the failure without leaking oracle detail. Otherwise the runtime returns `signature_invalid`.

## Interaction with other Wave 2 artifacts

- **Artifact 09 — rate-limit matrix.** Signature failures count against the WordPress bridge budget. Repeated `signature_invalid`, `timestamp_future`, `nonce_replay`, or `canonicalization_mismatch` events should move the session into stricter throttling before ability policy is evaluated.
- **Artifact 10 — user-presence nonce.** User-presence nonces are distinct from transport nonces. Transport nonces prove freshness of every HTTP request and live in the `wp_bridge_hmac` namespace. User-presence nonces prove a fresh user gesture for write-like feedback events, live in their own namespace, and are lower-frequency.
- **Artifact 15 — PHP→Rust endpoint design.** The endpoint described there applies this signature spec before request routing, ability lookup, schema validation, or `SurfaceClientBridge` construction. Endpoint handlers should receive only requests that have already passed HMAC verification.

## Test fixtures

Phase 1 must include deterministic fixtures for signer and verifier implementations in PHP and Rust. Fixtures use fixed test keys and timestamps; production keys remain random.

| Fixture | Scenario | Expected outcome |
|---|---|---|
| `hmac_v1_happy_path_json_post` | `POST /v1/surface/invoke?ability=briefing.daily` with `Content-Type: application/json`, valid bearer, matching session id, fresh timestamp, fresh nonce, and exact body bytes | Runtime returns signed-transport accepted and proceeds to SurfaceClient policy. |
| `hmac_v1_replayed_signature_same_nonce` | Repeat the exact happy-path request with the same nonce and timestamp inside the 30-second window | Runtime rejects with HTTP 409 and `nonce_replay`. |
| `hmac_v1_tampered_body_byte` | Sign `{"depth":"standard"}` but send `{"depth":"deep"}` with same headers | Runtime rejects with HTTP 401 and `signature_invalid`. |
| `hmac_v1_stale_timestamp` | Timestamp is runtime time minus 31 seconds | Runtime rejects with HTTP 401 and `timestamp_stale`. |
| `hmac_v1_future_timestamp` | Timestamp is runtime time plus 6 seconds | Runtime rejects with HTTP 401 and `timestamp_future`. |
| `hmac_v1_rotated_key_old_rejected_new_accepted` | Sign one request with a derived key from old `session_id`, rotate session, then sign another with the new key | Old request rejects with `key_rotated`; new request verifies. |
| `hmac_v1_unknown_key_id` | `X-DailyOS-Key-Id` names no active session, bearer token absent or mapped elsewhere | Runtime rejects with HTTP 401 and `key_not_found`; no ability lookup occurs. |
| `hmac_v1_trailing_slash_path` | Sign `/v1/surface/invoke/` and send `/v1/surface/invoke`, or the inverse | Runtime rejects with `canonicalization_mismatch` when diagnosable, otherwise `signature_invalid`. |
| `hmac_v1_duplicate_query_keys_preserved` | Sign and send `/v1/surface/invoke?a=1&a=2` exactly | Runtime accepts. A variant that signs sorted/collapsed query keys rejects. |
| `hmac_v1_content_type_value_case_sensitive` | Sign `application/json` but send `Application/JSON`, or sign with charset omitted but send `application/json; charset=utf-8` | Runtime rejects because content-type value bytes differ. |
| `hmac_v1_empty_body` | `GET` or `POST` request with no body and content length zero | Runtime signs body length `0` and accepts if all other fields match. |
| `hmac_v1_uppercase_hex_rejected` | Correct HMAC bytes encoded as uppercase hex | Runtime rejects with `signature_invalid` and log reason `malformed_signature`. |
| `hmac_v1_body_reserialized_after_sign` | PHP signs compact JSON, HTTP client or middleware sends pretty-printed JSON | Runtime rejects with `canonicalization_mismatch` when body hash diagnostics identify drift. |
| `hmac_v1_nonce_namespace_separate_from_user_presence` | Transport nonce equals a user-presence nonce value by coincidence | Runtime accepts transport freshness if the transport namespace is fresh; user-presence validation remains separate. |

Fixture data must include:

- Master key bytes.
- Session id.
- Derived session key.
- Method.
- Path plus query.
- Content type.
- Body bytes as hex.
- Timestamp.
- Nonce.
- Canonical request bytes as hex.
- Expected HMAC as lowercase hex.

## Open questions

- Whether Phase 1 should keep loopback HTTP as the primary transport or move the same signing contract onto a Unix domain socket fallback for macOS. The canonical request shape can survive either transport, but endpoint identity and deployment ergonomics differ.
- Whether the WordPress plugin can reliably keep the per-session derived key memory-only across the relevant PHP execution modes in WordPress Studio. If not, Phase 1 must define an acceptable encrypted-at-rest cache or shorten the session lifetime further.
- Whether `manage_options` is too broad for production pairing and should become a dedicated WordPress capability such as `manage_dailyos_pairing`.
- Whether timestamp skew thresholds should remain fixed at 30 seconds stale and 5 seconds future, or become runtime-configurable for hosts with known clock behavior.
- Whether body compression should be forbidden on the loopback channel in Phase 1. This spec signs the transmitted body bytes; forbidding compression would simplify diagnostics.
- Whether `X-DailyOS-Key-Id` should remain an explicit header or be folded into a structured bearer token after implementation experience. Explicit header lookup is easier to debug in Phase 1, but it adds one more signed-transport input to validate.
- Whether keychain access-control prompts on macOS create unacceptable pairing friction for the WordPress Studio flow. If they do, the runtime needs a UX-specific keychain access policy without weakening key ownership.
