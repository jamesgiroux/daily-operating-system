# W4-C L0 packet - projection signing + offline tamper verification

Date: 2026-05-13 (V3)
Project: v1.4.2 - Personal Intelligence Engine: WordPress Foundation
Parent: DOS-546
Wave: 4 stage-1 (gates W4-A renderer trust and W5 markdown projection)
Issue: DOS-569 (W4-C: projection signing + offline tamper verification)
Canonical Linear: https://linear.app/a8c/issue/DOS-569

This packet captures the W4-C contract decisions resolved at L0.
The Linear issue description remains the canonical execution contract.
This packet supersedes it only where it makes explicit a decision the issue leaves open.

## Changelog

- **V3 (2026-05-13):** Cycle 2 reviewer fold. `/cso`, `/devex`, and Codex approved; eng conditional approval is closed by the W4-B V9 coordination amendment in commit `e8570cc4`.
  - **Eng P1-N1 closed in §6, §18, §22:** W4-B V9 §6.5 has added precedence 0 `ProjectionTampered` and precedence 1 `ProjectionVersionRollback` ahead of the stale-watermark family; W4-B V9 ac §45/§46 add pairwise precedence fixtures for tamper-over-stale and rollback-over-stale.
  - **Variant substrate contract pinned in §6 and §18:** W4-C implements `BridgeSurfaceError::ProjectionTampered` and `BridgeSurfaceError::ProjectionVersionRollback` with concrete field shapes, HTTP 422 mapping, quarantine linkage, and no `correction.claim`.
  - **CSO path-alpha folded in §16:** W4-B owns the class-level scope-filter helper name; W4-C uses the published helper, with proposed symbol `bridge_scope_filter::filter_claim_bound_envelope`.
  - **Eng P2/P3 cleanup folded in §8, §9, §12, §14, §21:** queued re-sign is committed v1.4.2 behavior, operator escalation is audit/CLI only with no UI, markdown sentinel parsing handles one UTF-8 BOM explicitly, and `keyring_version` bumps sweep verification-cache entries instead of waiting for lazy misses.
  - **Devex P3 cleanup folded in §8, §10:** the signed payload hash includes the surface domain separator, and audit envelope raw `composition_id` exposure is an admin/debug branch only.
- **V2 (2026-05-13):** Cycle 1 reviewer fold. Material changes:
  - **Codex CRITICAL-1 folded in §3, §13, §14, §15, §18:** `SignedProjectionPayload` now excludes the detached signature envelope entirely. `key_id`, `signature_id`, `signed_at`, `alg`, `canonicalization`, and `signature_b64` live only in the envelope or ledger metadata. Replacement-key re-sign can keep canonical payload bytes unchanged because key/signature metadata is not signed.
  - **Codex CRITICAL-2 folded in §4, §13, §14, §15, §18:** single-row `projection_ledger` signature columns replaced by stable projection ledger rows plus `projection_signatures` rows keyed by `signature_id`; `current_signature_id`, `signature_status`, `superseded_by_signature_id`, and one-active-signature uniqueness are now explicit.
  - **CSO CRITICAL shadow-render folded in §2, §5, §18, §20:** Phase 1 shadow signing no longer allows trusted affordances. W4-A must visibly downgrade unless `projection_signature_enforcement = enforce`; mode changes emit `projection.enforcement_mode_changed`.
  - **CSO CRITICAL quarantine DoS folded in §4, §9, §14, §17, §18, §19:** quarantine event writes coalesce per `projection_id` for 60 seconds, re-sign retries cap at 5 attempts, and workers stop looping when a projection is re-tampered within 120 seconds of the last re-sign.
  - **Codex HIGH-3 folded in §4, §16, §18, §19:** multi-claim blocks now use `projection_ledger_block_refs` with per-ref scope metadata; mixed-scope multi-claim fixtures are required.
  - **Codex HIGH-4 and CSO HIGH H2 folded in §5, §6, §12, §17, §18, §19:** valid old signed bytes are not trusted unless the ledger currentness check passes: live projection id, current signature id, non-tombstoned locator, and expected composition/claim watermark. Rollbacks raise `ProjectionVersionRollback` and quarantine.
  - **Codex MEDIUM-5 folded in §8, §18, §19, §20:** verification hot path has bounded payload/block bytes, cache key `(canonical_signed_payload_sha256, signature_id, keyring_version)`, batch verification, and a worst-case render perf fixture.
  - **CSO HIGH H1 folded in §2, §7, §18, §19:** private-key custody now requires macOS Keychain ACL scoped to the runtime binary, zeroize on secret-key paths where supported, rotation crash recovery, and atomic commit semantics across DB key status and Keychain writes.
  - **CSO HIGH H3 folded in §10, §16, §18:** audit detail redacts or hashes claim/composition identifiers for non-admin or out-of-scope consumers and defines `scope_redacted`.
  - **Eng P1-1 folded in §6, §18:** W4-C names `BridgeSurfaceError::ProjectionTampered` and places it in W4-B §6.5 precedence before stale-watermark correction.
  - **Eng P1-2 folded in §1, §3, §18:** domain separators are surface-suffixed: `dailyos.wp_studio.projection.v1` and `dailyos.markdown.projection.v1`.
  - **Devex P1-1 folded in §12, §18:** markdown projections use a file-head HTML sentinel comment carrying a base64url detached envelope.
  - **W4-B V8 inheritance folded in §5, §11, §16, §18, §22:** all W4-C `/v1/surface/*` endpoints land in `src-tauri/src/bridges/surface_client.rs`, inherit `wp_user_id` session binding, and retain the W4-B class-level scope-filter rule.
- **V1 (2026-05-13):** Initial W4-C L0 packet. Mirrors W4-B V7 section order and lifts W4-B load-bearing interlocks: signature verification precedes W4-B stale-write handling, direct-key reads remain scope-gated through `Actor::SurfaceClient`, and the watermark quadruple anchors the signed projection.

## Status snapshot

- W3 L0 packet closed the host-boundary posture for Studio and W3-A pinned the projection envelope.
- W4-B V9 reserves the concurrency watermark contract, promotes `src-tauri/src/bridges/surface_client.rs` as the canonical `/v1/surface/*` route owner, lifts `wp_user_id` session binding into all SurfaceClient consumers, and amends §6.5 precedence for W4-C tamper/rollback before stale-watermark handling.
- DOS-569 is blocked by W1-E, W2-C runtime anchor/pairing, and W4-B versions.
- W4-A cannot render cached projections as trusted until W4-C lands offline verification.
- W4-C is detection and quarantine, not bidirectional edit ingestion.
- Markdown-as-input reconciliation remains v1.4.6 scope.

## Pre-work confirmed

**Headline finding:** W4-C adds new cryptographic signing substrate. The repository has keychain, audit, composition, provenance, SurfaceClient scope, and projection-status primitives, but no Ed25519 projection-signing implementation.

| Surface | Confirmed substrate | W4-C decision |
|---|---|---|
| Crypto crate | `src-tauri/Cargo.toml` has `sha2`, `base64`, `rand`, `zeroize`, no `ed25519-dalek`, no `ring` | Add `ed25519-dalek` for Ed25519 sign/verify, `signature` traits as needed; no `ring` |
| Existing signing | HMAC/provenance token patterns exist; updater uses Minisign externally; claims has semantic-signature naming only | Do not reuse HMAC or semantic signatures for projection authenticity |
| Key storage | `LocalKeychain` uses macOS `security` CLI under service `com.dailyos.desktop.db`; connector token stores use same pattern | Store Ed25519 private keys in a new projection-signing keychain service/account namespace |
| Audit | `emit_surface_audit` writes actor-kind, actor-instance, `wp_user_id`, and actor scopes for `Actor::SurfaceClient` | Tamper events route through this helper with a distinct event kind, not correction paths |
| Composition | `CompositionMetadata.composition_version`, `ClaimRef.claim_version`, `ProvenanceRef`, `Block` exist | W4-C signs composition, block, claim refs, provenance ref, ordering, and W3-A envelope |
| Projection status | `claim_projection_status` tracks derived projection failures, not authenticity | W4-C creates a dedicated projection ledger and quarantine state |
| W3-A envelope | W3-A pins `dailyos_canonical_id`, `dailyos_signature`, `dailyos_source_runtime`, `dailyos_projection_version` | W4-C signs the payload fields derived from this envelope and stores signature metadata in the detached `dailyos_signature` envelope |
| W4-A render path | W4-A consumes W4-C verification state before rendering cached bytes as trusted | W4-A renders degraded state on tamper/quarantine; it does not verify by network round trip |
| W4-B mutation gate | W4-B owns versions and 409 stale-watermark path | W4-C verification runs before W4-B 409 handling |
| Surface bridge owner | W4-B V8 promotes `src-tauri/src/bridges/surface_client.rs` | W4-C keyring, projection, ledger, and quarantine endpoints register in that module and inherit bridge preconditions |

## Directional decisions resolved at L0

### §1. Scope and outcome

W4-C lands projection authenticity for WordPress DB projections and markdown filesystem projections.
The runtime remains authoritative for claims, provenance, projection manifests, signatures, and reconciliation state.
The runtime mutation path is the only path allowed to create a trusted projection signature.
Out-of-band edits are detected, downgraded, quarantined, and audited.

In scope:

- Ed25519 signatures on every projection write.
- RFC 8785 canonical JSON serialization of `SignedProjectionPayload`.
- Surface-suffixed domain separators: `dailyos.wp_studio.projection.v1` and `dailyos.markdown.projection.v1`.
- Runtime projection ledger with stable projection rows, detached signature rows, canonical signed bytes, and signature metadata.
- Offline verification on projection read using local public keys.
- Ledger currentness verification after cryptographic verification, so old valid signed bytes cannot be replayed as trusted.
- Unknown-key refresh with exactly one keyring retry.
- Key lifecycle: generation, rotation, retirement, revocation, replacement-key provisioning, queued re-sign, retired-key historical verification.
- Quarantine state that preserves the observed tampered row or file bytes.
- Quarantine coalescing and retry caps that keep byte-flip attacks from generating unbounded audit or queue volume.
- Tamper-event audit emission distinct from correction events.
- `GET /v1/surface/keyring` public-key distribution, scope-gated through `Actor::SurfaceClient`.

Out of scope:

- Gutenberg renderer implementation; W4-A consumes the verification result.
- Automated repair UI; W4-C writes ledger/quarantine state and exposes audit/read endpoints.
- Bidirectional markdown edit ingestion; v1.4.6 owns markdown-as-input.
- New mutation authorization semantics; ADR-0102 trust rules still apply.
- Treating a valid projection signature as permission to mutate canonical claims.

Outcome:

- A projection can be read without network access and classified as verified, pending, stale, tampered, unknown-key, revoked-key, or quarantined.
- No tampered projection is silently promoted to substrate truth.
- W4-A can render cached projection bytes only after W4-C verification succeeds or can render them visibly degraded when verification fails.

### §2. Substrate reuse audit - Ed25519 crate selection, existing infra, keyring, envelope, renderer, gate

**Crypto crate selection:** choose `ed25519-dalek`, not `ring`.

| Candidate | Decision | Reason |
|---|---|---|
| `ed25519-dalek` | Selected | Direct Ed25519 API, RustCrypto `signature` traits, mature verification path, no algorithm negotiation pressure |
| `ring` | Rejected for W4-C | Broader crypto surface than needed; less ergonomic key lifecycle and signature trait integration for this narrow contract |
| HMAC | Rejected | W2 HMAC authenticates transport requests, not public offline projection verification |
| RSA/ECDSA | Rejected | Linear DOS-569 and artifact 03 explicitly ban RSA, ECDSA, and algorithm negotiation |

`ed25519-dalek` must be added to `src-tauri/Cargo.toml` when implementation starts.
The current checkout does not include an Ed25519 crate.
The implementation uses `zeroize` for in-memory private-key handling where supported and follows ADR-0092 operational hardening for key material custody.

Existing signature-like code is not reusable as projection authenticity:

- HMAC/provenance confirmation tokens prove a paired request, not a projection artifact.
- Updater Minisign verification is a distribution concern, not runtime projection signing.
- `compute_semantic_signature` in `services/claims.rs` is a dedup/canonicalization helper, not cryptography.
- `sha2` remains useful for ledger hashes and observed/canonical excerpt hashes, not as a signature substitute.

Keyring storage:

- Private signing keys live only in the runtime keychain under a projection-specific service/account namespace.
- The keychain item ACL is pinned to the signed DailyOS runtime binary, not a broad login-keychain read grant.
- Rotation commits are atomic across `projection_signing_keys.key_status` and keychain writes: if either side fails, the transaction rolls back and startup recovery marks the key transition incomplete.
- Secret-key material is zeroized on the Ed25519 secret-key path where the crate and platform APIs support it; public keys and signature bytes are not treated as secret.
- The W4-C tables store `key_id`, public keys, statuses, validity windows, and audit metadata.
- Public verification keys are not secret and may be distributed to WordPress and markdown projections.
- `key_id` is runtime-generated opaque identity, not a filesystem path and not a Keychain label.

W3-A envelope:

- Every W3-A-authored storage row destined for W4-C signature carries `dailyos_canonical_id`, `dailyos_signature`, `dailyos_source_runtime`, and `dailyos_projection_version`.
- `dailyos_signature` stores the detached signature envelope.
- `dailyos_source_runtime` binds to the W2-C runtime anchor / runtime instance identity.
- `dailyos_projection_version` is the projection-envelope version, distinct from W4-B `composition_version`.

W4-A render path:

- W4-A reads W4-C verification state and `projection_signature_enforcement` before rendering cached projection bytes as trusted.
- W4-A may paint pixels optimistically, but it cannot show a trusted provenance affordance until verification succeeds and enforcement mode is `enforce`.
- In `shadow` or `disabled`, verified bytes may be shown as content, but the trust affordance is visibly downgraded because the runtime is not enforcing signatures.
- If verification fails, W4-A renders degraded trust-band state and points to the W4-C ledger/quarantine record.

W4-B mutation gate:

- W4-B owns claim/composition version assignment and stale-write rejection.
- W4-C consumes the W4-B watermarks and runs before W4-B stale-write envelopes.
- Tamper errors never become W4-B correction payloads.

ADR anchors:

- ADR-0083 governs user-facing tamper copy vocabulary.
- ADR-0074 and ADR-0078 are substrate-reuse reminders: composition salience consumes existing retrieval, not a W4-C concern.
- ADR-0092 governs local key custody posture.
- ADR-0094 governs append-only audit hygiene.
- ADR-0102 governs ability trust and SurfaceClient policy.
- ADR-0105 and ADR-0108 govern provenance references and rendering caps.
- ADR-0111 governs `Actor::SurfaceClient` scopes and audit identity.
- ADR-0129 and ADR-0130 govern WordPress-as-surface and substrate-owned composition.

### §3. Watermark contract anchoring

W4-C signs the W4-B watermark quadruple:

| Field | Source | Required in signed payload |
|---|---|---|
| `claim_id` | `ClaimRef.claim_id` / `intelligence_claims.id` | Yes |
| `claim_version` | W4-B assigned claim version | Yes |
| `composition_id` | `CompositionDocId` / projection metadata | Yes |
| `composition_version` | W4-B assigned composition version | Yes |

The quadruple anchors the signature to a specific claim snapshot inside a specific composition snapshot.
Changing any one of the four values invalidates the signature.
This includes ref/version swaps where visible text is unchanged.

The W3-A envelope is split into signed projection payload and detached signature envelope:

| Field family | Signing rule |
|---|---|
| `dailyos_canonical_id` | Included in `SignedProjectionPayload` as projection canonical identity |
| `dailyos_source_runtime` | Included in `SignedProjectionPayload` as runtime anchor / source runtime binding |
| `dailyos_projection_version` | Included in `SignedProjectionPayload` as projection envelope schema version |
| `dailyos_signature` | Excluded from `SignedProjectionPayload` entirely; stored as detached envelope and ledger metadata |

Payload-excludes-envelope rule:

- `SignedProjectionPayload` contains the projection identity, runtime anchor, projection envelope version, surface target, composition watermark, block ordering, block payloads, claim refs, claim versions, field paths, and provenance refs.
- `SignedProjectionPayload` does **not** contain `key_id`, `signature_id`, `signed_at`, `alg`, `canonicalization`, `signature_b64`, `keyring_version`, or any other detached signature envelope field.
- `alg = Ed25519` and `canonicalization = RFC8785-JSON` are verified from the detached envelope and ledger metadata before running Ed25519 verification; they are not themselves part of the bytes signed by the key.
- Replacement-key re-signing therefore produces a new `signature_id` and signature envelope over the same canonical payload bytes when canonical claim/projection state has not changed.
- Envelope mutation outside `signature_b64` is rejected by envelope/ledger currentness checks, not by pretending those envelope bytes are inside `SignedProjectionPayload`.

Linear DOS-569 adds the payload shape constraint:

- Use `blocks[]` entries, not loose parallel arrays.
- Each block entry includes `block_id`, `block_type`, `claim_refs[]`, `claim_versions[]`, `field_paths[]`, `provenance_ref`, canonical block payload, and order index.
- The top-level payload includes composition id/version, projection target, runtime anchor, projection canonical identity, projection envelope version, surface-specific domain separator, and ordering.
- Signature verification fails if block id, block type, claim ref, claim version, field path, provenance ref, composition version, projection target, runtime anchor, or ordering changes.

Domain separator decision:

- WordPress DB projections use `domain: "dailyos.wp_studio.projection.v1"`.
- Markdown filesystem projections use `domain: "dailyos.markdown.projection.v1"`.
- The domain separator is inside `SignedProjectionPayload` and is covered by Ed25519.
- Surface-agnostic `dailyos.projection.v1` is rejected for v1.4.2 because a valid signed payload must not be copyable between WordPress block storage and markdown sentinel storage.

Canonicalization:

- Signed bytes are RFC 8785 canonical JSON serialization of `SignedProjectionPayload`.
- JSON object key order is canonicalized; array order remains semantically meaningful and signed.
- Timestamps, if present inside the projection payload, are serialized as RFC3339 UTC strings.
- Binary fields inside the payload are base64url without padding.
- The detached signature envelope is canonicalized separately for storage/transport stability, but those envelope bytes are not Ed25519-signed.

### §4. Migration slot reservations

W4-C has exactly four migration slots: v173 through v176 for this implementation.
Implementation note: dev already has v171 reserved by W3 and v172 reserved by W4-B, so DOS-569 uses v173-v176 as the non-overlapping block.

| Slot | Migration name | Concrete schema |
|---|---|---|
| v173 | `173_w4c_projection_signing_keys.sql` | `projection_signing_keys`, `projection_key_status_events` |
| v174 | `174_w4c_projection_ledger_signature.sql` | `projection_ledger`, `projection_signatures`, `projection_ledger_blocks`, `projection_ledger_block_refs` |
| v175 | `175_w4c_projection_quarantine.sql` | `projection_quarantine`, `projection_ledger.quarantine_state`, quarantine indexes |
| v176 | `176_w4c_replacement_key_provisioning.sql` | `projection_replacement_keys`, `projection_resign_queue`, retired/revoked key linkage |

Concrete table and column allocation:

| Slot | Table / alteration | Required columns |
|---|---|---|
| v173 | `projection_signing_keys` | `key_id` PK, `public_key_b64`, `key_status` (`active`/`rotating`/`retired`/`revoked`), `created_at`, `valid_from`, `valid_until`, `retired_at`, `revoked_at`, `replacement_key_id`, `keychain_service`, `keychain_account_ref` |
| v173 | `projection_key_status_events` | `event_id` PK, `key_id`, `previous_status`, `next_status`, `reason`, `created_at`, `actor_kind` |
| v174 | `projection_ledger` | `projection_id` PK, `surface`, `surface_locator`, `surface_locator_hash`, `locator_status` (`live`/`tombstoned`), `dailyos_canonical_id`, `dailyos_source_runtime`, `dailyos_projection_version`, `composition_id`, `composition_version`, `current_signature_id`, `canonical_signed_payload_sha256`, `claim_watermark_sha256`, `last_verified_at`, `verification_status` |
| v174 | `projection_signatures` | `signature_id` PK, `projection_id` FK, `key_id` FK, `signature_status` (`active`/`superseded`/`revoked`/`retired`), `alg`, `canonicalization`, `canonical_signed_payload_bytes`, `canonical_signed_payload_sha256`, `signature_bytes`, `signature_envelope_b64url`, `issued_at`, `superseded_by_signature_id`, `revoked_at`, `retired_at` |
| v174 | `projection_ledger_blocks` | `projection_id`, `block_id`, `block_order`, `block_type`, `block_payload_sha256` |
| v174 | `projection_ledger_block_refs` | `projection_id`, `block_id`, `claim_ref_index`, `claim_id`, `claim_version`, `field_path`, `provenance_invocation_id`, `provenance_field_path`, `scope_grant_hash` |
| v175 | `projection_ledger` alteration | `quarantine_state` with values `none`, `suspected`, `quarantined`, `resolved`; `last_quarantine_event_at`; `quarantine_event_count` |
| v175 | `projection_quarantine` | `quarantine_id` PK, `projection_id`, `surface`, `surface_locator_hash`, `observed_payload_hash`, `observed_signature_b64`, `expected_signature_id`, `verification_error`, `field_pointer`, `byte_range_start`, `byte_range_end`, `sanitized_observed_excerpt_hash`, `detected_by`, `detected_at`, `last_seen_at`, `seen_count`, `coalesced_until`, `status` |
| v176 | `projection_replacement_keys` | `replacement_id` PK, `old_key_id`, `new_key_id`, `reason`, `provisioned_at`, `activated_at`, `completed_at`, `recovery_status` |
| v176 | `projection_resign_queue` | `queue_id` PK, `projection_id`, `old_signature_id`, `old_key_id`, `new_key_id`, `status`, `attempts`, `max_attempts`, `last_error`, `last_resign_at`, `last_retampered_at`, `operator_escalated_at`, `queued_at`, `updated_at` |

Required constraints:

- `projection_signatures.alg = 'Ed25519'`.
- `projection_signatures.canonicalization = 'RFC8785-JSON'`.
- `projection_ledger.surface IN ('wordpress_db', 'markdown_file')`.
- `projection_ledger.locator_status IN ('live', 'tombstoned')`.
- `projection_signing_keys.key_status IN ('active', 'rotating', 'retired', 'revoked')`.
- `projection_signatures.signature_status IN ('active', 'superseded', 'revoked', 'retired')`.
- `projection_resign_queue.status IN ('queued', 'in_progress', 'succeeded', 'failed', 'operator_escalation')`.
- `projection_quarantine.status IN ('open', 'reprojected', 'dismissed', 'resolved')`.
- `projection_resign_queue.max_attempts DEFAULT 5 CHECK (max_attempts BETWEEN 1 AND 10)`.
- Partial unique index: exactly one `projection_signatures` row with `signature_status = 'active'` per live `projection_id`.
- `projection_ledger.current_signature_id` must reference the active row for that `projection_id`.
- `projection_signatures.superseded_by_signature_id` references a newer signature row when status is `superseded`.
- `projection_ledger_block_refs` primary key is `(projection_id, block_id, claim_ref_index)` so multi-claim blocks are scope-filtered by every referenced claim, not by a scalar shortcut.

### §5. Data flow

Write path:

1. Composition-producing ability returns `AbilityOutput<Composition>` per ADR-0102 and ADR-0130.
2. W4-B assigns `claim_version` and `composition_version`.
3. Projection writer builds the W3-A envelope.
4. Projection writer constructs `SignedProjectionPayload`.
5. Canonicalizer produces RFC 8785 JSON bytes.
6. Runtime loads the active Ed25519 private key from keychain.
7. Runtime signs canonical bytes and writes signature envelope.
8. Runtime writes projection bytes, the stable `projection_ledger` row, the active `projection_signatures` row, and block/ref rows in the same projection transaction.
9. Runtime emits audit `projection.signature_issued`.

Surface route ownership and preconditions:

- All W4-C `/v1/surface/*` endpoints land in `src-tauri/src/bridges/surface_client.rs`, the canonical module promoted by W4-B V8.
- The bridge runs W4-B V8 `validate_session_bound_wp_user_id(actor, payload)` before keyring, projection, ledger, or quarantine logic.
- Body/query/header supplied `wp_user_id` values are never trusted independently of the paired SurfaceClient session binding.
- The route module then applies W4-B's class-level scope-filter helper before returning any claim-bound ledger, quarantine, or projection metadata.

Read path:

1. SurfaceClient parses the projection and extracts the detached `dailyos_signature` envelope.
2. SurfaceClient reconstructs `SignedProjectionPayload` from block bytes and signed envelope-adjacent fields, excluding the detached signature envelope entirely.
3. SurfaceClient locates `key_id` from the envelope in its cached keyring.
4. SurfaceClient verifies envelope metadata (`alg`, `canonicalization`, runtime anchor, projection id) and then verifies Ed25519 signature against canonical payload bytes.
5. SurfaceClient or runtime reconciliation checks ledger currentness before trusted render.
6. Trusted rendering requires cryptographic verification success, currentness success, and `projection_signature_enforcement = enforce`.
7. Verification or currentness failure marks it degraded, emits tamper audit, and writes/coalesces quarantine state.
8. Runtime reconciliation compares observed bytes to `projection_ledger` and `projection_signatures`.

Offline verification rule:

- The normal verify path makes no network call.
- The public key needed for verification must already be in the embedded/cached keyring.
- Unknown-key refresh is the only allowed runtime fetch during verification and retries once.
- If the refresh cannot run, the projection remains unverified, not trusted.

Unknown-key path:

1. Verifier sees unknown `key_id`.
2. Verifier renders pending/unverified state.
3. Verifier calls `GET /v1/surface/keyring` once through the paired SurfaceClient route.
4. Verifier refreshes cache and retries verification once.
5. If still unknown, it records `UnknownKeyAfterRefresh` and enqueues reconciliation.

Currentness path:

1. After Ed25519 succeeds, runtime compares the detached envelope's `signature_id` to `projection_ledger.current_signature_id`.
2. Runtime verifies the `projection_id` exists, `locator_status = 'live'`, and the locator hash matches the current surface locator.
3. Runtime compares `composition_id`, `composition_version`, and `claim_watermark_sha256` against the ledger's expected live watermark.
4. Any mismatch is `ProjectionVersionRollback` if the bytes are validly signed but stale, replayed, restored from backup, imported by SQL, or resurrected after tombstone.
5. `ProjectionVersionRollback` creates quarantine and cannot render trusted, even though Ed25519 verification succeeded.

Quarantine path:

1. Verification fails with mismatch, missing signature, revoked key, wrong runtime anchor, unsupported algorithm, unsupported canonicalization, unknown key after refresh, or currentness rollback.
2. Runtime records or coalesces `projection_quarantine` preserving observed state hashes and locator hash.
3. Runtime updates `projection_ledger.quarantine_state = 'quarantined'`.
4. Runtime emits tamper audit, subject to the 60-second coalescing window.
5. W4-A renders degraded state from ledger/quarantine.

### §6. Out-of-band edits ordering - sig check BEFORE W4-B 409

This is the load-bearing W4-C acceptance criterion.

1. **W4-C signature check runs before W4-B 409 path** in the loopback endpoint.
2. **Tamper error does NOT emit `correction.claim`.**
3. A tampered projection is not a stale writer.
4. A stale writer may be corrected only after the projection being used as the write basis verifies.
5. Signature failure returns a tamper/degraded response and emits a tamper event, not `claim.write_rejected`.

Ordering table, aligned with W4-B V9 §6.5 in commit `e8570cc4`. W2-C pairing, HMAC, and actor validity remain pre-dispatch bridge preconditions; the W4-B/W4-C variant precedence starts only after those checks pass.

| Precedence | Check | Failure result | Emits `correction.claim` |
|---|---|---|---|
| Pre-dispatch | W2-C pairing / HMAC / actor validity | Auth/surface error | No |
| 0 | W4-C projection signature verification fails | `BridgeSurfaceError::ProjectionTampered` / 422 / quarantine | No |
| 1 | W4-C signed payload verifies but ledger currentness rolls back | `BridgeSurfaceError::ProjectionVersionRollback` / 422 / quarantine | No |
| 2 | W4-B missing expected version | 400 | No |
| 3 | W4-B mid-flight mutation | 423 | No |
| 4 | W4-B overflow | 500 | No |
| 5/6 | W4-B stale watermark | 409 stale envelope | Scope-filtered if permitted |

Out-of-band edits include:

- direct markdown edits inside DailyOS sentinels
- direct WordPress DB row edits
- WordPress admin UI edits of DailyOS-owned block attributes
- plugin transforms that strip or modify signed fields
- DB restore or SQL import
- Studio `wp_cli` passthrough mutations
- copied signatures between blocks

The W4-B 409 path remains available for legitimate stale writes after authenticity and currentness are established.
If authenticity is not established, there is no trustworthy basis for `correction.claim`.
W4-B V9 §6.5 has added `BridgeSurfaceError::ProjectionTampered` and `BridgeSurfaceError::ProjectionVersionRollback` ahead of stale-watermark variants in commit `e8570cc4`; W4-B V9 ac §45 and §46 own pairwise precedence fixtures for tamper-over-stale and rollback-over-stale.

W4-C-owned `BridgeSurfaceError` variants, substrate shape:

```rust
BridgeSurfaceError::ProjectionTampered {
    projection_id: ProjectionId,
    signature_id: SignatureId,
    key_id: KeyId,
    observed_signature_status: SignatureStatus,
    quarantine_id: QuarantineId,
}

BridgeSurfaceError::ProjectionVersionRollback {
    projection_id: ProjectionId,
    signed_composition_version: u64,
    ledger_composition_version: u64,
    signed_claim_version: Option<u64>,
    ledger_claim_version: Option<u64>,
}
```

Variant rules:

- Both variants are added to `src-tauri/src/bridges/types.rs` and map to HTTP 422 at the bridge boundary.
- `ProjectionTampered` fires for signature verification failure, malformed or missing signature carrier, unsupported envelope/canonicalization, wrong runtime anchor, copied signature, or revoked-key verification failure.
- `ProjectionVersionRollback` fires only after Ed25519 succeeds and the ledger currentness comparison proves the signed composition or claim watermark is older than live ledger state.
- `signed_claim_version` and `ledger_claim_version` serialize as `u64` or `null`; `null` is allowed for composition-wide payloads without a scalar claim version.
- Both variants must carry enough typed data to link the response, quarantine row, and audit event without stringly typed ids or raw claim bodies.
- Neither variant emits `correction.claim`, `claim.write_rejected`, or W5 feedback/correction payloads.

### §7. Key lifecycle state machine

Key states:

| State | Signs new projections | Verifies historical projections | Distributed in keyring | Notes |
|---|---|---|---|---|
| `active` | Yes | Yes | Yes | Exactly one active key signs normal writes |
| `rotating` | Yes, bounded transition | Yes | Yes | Replacement has been provisioned; re-sign queue active |
| `retired` | No | Yes | Yes | Historical verification only |
| `revoked` | No | No trusted verification | Yes, status included | Key compromise or invalid custody |

Valid transitions:

| From | To | Trigger |
|---|---|---|
| none | `active` | First key generation |
| `active` | `rotating` | Planned rotation begins |
| `rotating` | `retired` | Re-sign queue completes |
| `active` | `revoked` | Compromise, custody failure, or admin revocation |
| `rotating` | `revoked` | Compromise during rotation |
| `retired` | `revoked` | Historical key later declared compromised |
| `revoked` | none | No transition out; revocation is terminal |

Generation:

- Runtime generates Ed25519 keypair using OS CSPRNG.
- Private key is stored in runtime keychain with an ACL scoped to the signed DailyOS runtime binary.
- Public key row is inserted into `projection_signing_keys`.
- `key_id` is opaque and runtime-generated.
- Secret-key buffers are zeroized on drop where supported by the selected Ed25519 crate and platform wrapper.
- Audit emits `projection.key_generated`.

Rotation:

- Runtime provisions a replacement key.
- Old active key becomes `rotating`.
- New key becomes `active`.
- Re-sign queue is populated for live projections.
- Old key becomes `retired` after queued re-sign completes.
- The status transaction and keychain write are crash-safe: startup recovery reconciles DB rows whose keychain item is missing, and keychain items whose DB transaction did not commit.
- A failed or partial rotation leaves no trusted key in a split-brain state; verification downgrades until recovery resolves it.

Revocation:

- Runtime marks key `revoked`.
- Any signature under that key fails trusted verification with `KeyRevoked`.
- Replacement key is provisioned.
- Live projections signed by the revoked key are queued for re-sign but render degraded until refreshed.

Retirement:

- Retired keys stay in the public keyring for historical verification.
- Retired keys never sign new projections.
- Removing retired keys from keyring is not allowed in v1.4.2.

### §8. Unknown-key refresh

Unknown key is not immediately tamper.
It is an unverified state with a bounded refresh path.

Algorithm:

```rust
fn verify_with_unknown_key_refresh(projection, keyring_cache) -> VerificationResult {
    match verify_projection(projection, keyring_cache) {
        Err(UnknownKey(key_id)) => {
            refresh_keyring_once();
            verify_projection(projection, keyring_cache)
                .map_err(|err| err.with_refresh_attempted(true))
        }
        other => other,
    }
}
```

Rules:

- Refresh happens at most once per projection verification attempt.
- Retry happens immediately after successful keyring response.
- If refresh fails, verifier returns `UnknownKeyRefreshFailed`.
- If refresh succeeds but key remains absent, verifier returns `UnknownKeyAfterRefresh`.
- Both outcomes render unverified and enqueue reconciliation.
- Neither outcome emits `correction.claim`.

Cache TTL:

- WP plugin caches keyring for the paired session.
- Runtime response carries `max_age_seconds`.
- Default TTL is 300 seconds.
- Unknown-key failure bypasses TTL and forces the single refresh.
- Revocation status in a refreshed keyring overrides any cached active/retired state immediately.

Backoff:

- Unknown-key refresh uses the W2-D SurfaceClient read budget.
- Repeated unknown-key failures for the same `key_id` are coalesced for 60 seconds per SurfaceClient instance.
- Coalescing suppresses duplicate fetches, not verification failures.

Read-path performance budget:

- Canonical payload bytes are capped per projection and per block before verification; oversize projections return degraded `PayloadTooLarge` state and enqueue reconciliation.
- Verification cache key is `(canonical_signed_payload_sha256, signature_id, keyring_version)`.
- The surface `domain_separator` is inside `SignedProjectionPayload` and therefore inside `canonical_signed_payload_sha256`; the cache key does not need a separate domain-separator field.
- Cache hits still run the ledger currentness check; cryptographic verification may be reused only for the exact payload hash, signature id, and keyring version.
- When refreshed keyring state advances `keyring_version`, the verifier sweeps all cached cryptographic verification entries for older keyring versions. Lazy-on-miss eviction is rejected because revoked-key status must affect already warm render reads immediately after the keyring bump.
- Batch verification groups projections by `key_id` and `keyring_version` for W4-A render sweeps.
- Worst-case render acceptance must cover the largest allowed block count, maximum allowed canonical bytes, mixed scope refs, and a cold keyring cache.

### §9. Quarantine semantics

Quarantine preserves tampered state.
It does not delete or overwrite the bad row or file.

Quarantine writes:

- `projection_ledger.quarantine_state = 'quarantined'`.
- `projection_quarantine` row stores the locator, observed payload hash, observed signature, expected signature id, verification error, and safe diff pointers.
- The WP row or markdown file remains in place until repair/reprojection.
- If an auto-reproject follow-up later writes canonical bytes, it writes a new signature and leaves the quarantine record queryable.

Do-not-overwrite and coalescing rule:

- W4-C detection never overwrites the observed tampered bytes as part of the detection transaction.
- Reprojection is a separate action with a separate audit event.
- If the same tampered bytes are observed repeatedly, increment `seen_count` and update `last_seen_at`.
- If observed bytes change within 60 seconds for the same `projection_id`, coalesce into the open quarantine row, increment `seen_count`, update `last_seen_at`, and append only hashed/sanitized diff pointers.
- If observed bytes change after the 60-second coalescing window, create a new divergence revision under the same projection thread.
- The audit writer emits at most one `projection.quarantined` event per `projection_id` per coalescing window and increments counters for suppressed duplicates.
- This is the byte-flip DoS defense: an attacker cannot create unbounded quarantine revisions, audit rows, or re-sign queue entries by changing one byte repeatedly.

Surface behavior:

- W4-A reads W4-C ledger/quarantine state and degrades the trust band.
- High-sensitivity blocks may collapse body content per ADR-0108 rendering policy.
- Banner copy must follow ADR-0083 product vocabulary.
- Internal key ids, raw signatures, and raw tampered sensitive text are not shown to unauthorized actors.

Quarantine status values:

| Status | Meaning |
|---|---|
| `open` | Tamper is active and unrepaired |
| `reprojected` | Runtime wrote a fresh signed projection |
| `dismissed` | Operator acknowledged without repair |
| `resolved` | Subsequent verification succeeded and audit remains |

Re-sign loop guard:

- Re-sign queue rows default to `max_attempts = 5`.
- After `max_attempts`, the queue row transitions to `operator_escalation`, quarantine remains `open`, and W4-A keeps degraded rendering.
- Re-sign worker refuses to process a projection re-tampered within 120 seconds of `last_resign_at`.
- A re-tampered projection updates `last_retampered_at` and waits for the cooldown instead of looping write/sign/write/sign.
- Operator escalation is audited once per queue row, not once per retry.
- Operator escalation is visible through audit/CLI inspection in v1.4.2; W4-C does not add a new UI surface for escalation state.

### §10. Tamper-event audit envelope

Tamper events are integrity events.
They are distinct from correction events and do not flow through the W4-B 409 path.

Event kinds:

| Event kind | When |
|---|---|
| `projection.signature_issued` | Runtime signs a projection |
| `projection.verify_succeeded` | Read verification succeeds |
| `projection.tamper_detected` | Signature/canonicalization/runtime-anchor mismatch |
| `projection.unknown_key_refresh` | Verifier refreshes keyring for unknown key |
| `projection.key_revoked_detected` | Projection uses revoked key |
| `projection.quarantined` | Quarantine row is created |
| `projection.resign_queued` | Replacement-key flow queues projection |
| `projection.resigned` | Re-sign writes a new signature |

Audit detail shape:

```jsonc
{
  "projection_id": "<string>",
  "surface": "wordpress_db | markdown_file",
  "surface_locator_hash": "<sha256>",
  "dailyos_canonical_id": "<string>",
  "composition_id_hash": "<sha256>",
  "composition_id": "<admin-only raw id, omitted by default>",
  "composition_version": 12,
  "claim_id_hashes": ["<sha256 | admin-only raw ids>"],
  "claim_versions": [7],
  "signature_id": "<string | null>",
  "key_id": "<string | null>",
  "verification_error": "SignatureMismatch | MissingSignature | UnknownKeyAfterRefresh | KeyRevoked | WrongRuntimeAnchor | UnsupportedAlgorithm | UnsupportedCanonicalization | ProjectionIdentityMismatch",
  "quarantine_id": "<string | null>",
  "refresh_attempted": false,
  "scope_redacted": false
}
```

Audit hygiene:

- Use `emit_surface_audit` for SurfaceClient-originated reads.
- `wp_user_id` is required for `Actor::SurfaceClient`.
- `actor_instance` and `actor_scopes` are derived from the actor variant.
- Detail payload stores hashes and identifiers, not raw sensitive content.
- Non-admin or out-of-scope consumers receive hashed `claim_id`, hashed `composition_id`, hashed locator, and `scope_redacted = true`.
- `composition_id_hash` is the default audit field; admin/debug raw-id access is a separate scope-gated branch that may return `composition_id` to authorized operators only. Do not place raw composition ids under a `*_hash` key in default streams.
- Admin/debug consumers may resolve raw ids through a separate scope-gated inspection path, not through default audit streams.
- `scope_redacted = true` means at least one claim-bound field, locator, or excerpt was withheld because the actor lacked scope.
- Audit category is `security` or `anomaly`; never `correction`.

### §11. Keyring distribution endpoint

Endpoint:

`GET /v1/surface/keyring`

Owner module:

`src-tauri/src/bridges/surface_client.rs` owns this route per W4-B V8 §37. The same module owns W4-C projection, ledger, and quarantine read endpoints.

Purpose:

- Distribute public Ed25519 verification keys to paired SurfaceClients.
- Support offline verification for normal reads.
- Publish retired keys for historical verification.
- Publish revoked key status so clients fail closed.

Request:

- Requires W2-C pairing and HMAC request authentication.
- Requires `Actor::SurfaceClient { instance, scopes }`.
- Runs W4-B V8 `wp_user_id` session binding before any keyring lookup when the request carries a user id.
- Requires a read scope that permits projection verification for the requested surface.
- Does not accept a bearer key id as authorization.

Response:

```jsonc
{
  "ok": true,
  "runtime_anchor_id": "<runtime-anchor>",
  "keyring_version": 14,
  "max_age_seconds": 300,
  "keys": [
    {
      "key_id": "dailyos-runtime-ed25519-2026-05",
      "alg": "Ed25519",
      "public_key_b64": "<base64url>",
      "status": "active | rotating | retired | revoked",
      "valid_from": "2026-05-13T00:00:00Z",
      "valid_until": null,
      "revoked_at": null,
      "replacement_key_id": null
    }
  ]
}
```

Scope gating:

- The endpoint returns only keys reachable for the SurfaceClient's granted projection surfaces.
- A WP SurfaceClient without markdown scope does not receive markdown-only runtime anchors if those become distinct.
- If the request is out of scope, return a redacted envelope or 404; never leak claim-bound ledger content.
- Key material is public, but endpoint existence, runtime anchors, and ledger associations still follow SurfaceClient scope rules.

### §12. Studio wp_cli out-of-band surface

W3 L0 V4 resolved the host-boundary posture:

- Studio MCP and DailyOS plugin MCP are at different layers, not competing.
- Studio's `wp_cli` and `site_import/export` tools can mutate the WP DB independently of DailyOS scope enforcement.
- DailyOS treats those mutations as out-of-band edits.
- They hit the W4-B watermark contract plus W4-C tamper detection.
- They surface in trust-band rendering as degraded verification state.
- They are never silently promoted to canonical substrate.

W4-C handling:

- A `studio mcp wp_cli` edit to a DailyOS-owned block invalidates the signature if signed fields change.
- W4-C records the divergence in `projection_quarantine`.
- W4-C emits `projection.tamper_detected` and `projection.quarantined`.
- W4-A reads ledger/quarantine state and renders degraded trust.
- W3-C audit can carry the host-side divergence signal, but W3-C does not own UI degradation.

Developer experience:

- Developers exercising Studio host-layer commands may see degraded verification states.
- This is intentional detection behavior, not a W3-C bug.
- A future surface affordance can support "this was my edit" attribution; v1.4.2 detects, records, and refuses silent promotion.

Markdown signature carrier:

- Markdown projections carry the detached signature envelope in a file-head HTML-style sentinel comment.
- Format: `<!-- dailyos-signature: <base64url-no-padding-envelope-json> -->` as the first non-BOM bytes of the file.
- Sentinel parsing strips exactly one UTF-8 BOM before checking the file-head signature comment; any other leading bytes, comments, whitespace, or duplicated sentinels degrade to malformed/missing signature and quarantine.
- The base64url payload decodes to the detached envelope containing `signature_id`, `key_id`, `alg`, `canonicalization`, `signed_at`, `signature_b64`, `keyring_version`, `dailyos_source_runtime`, and `dailyos_projection_version`.
- The sentinel is not part of `SignedProjectionPayload`; the payload is reconstructed from the markdown projection body plus signed metadata fields.
- If the sentinel is missing, duplicated, not at file head, malformed, or claims an unsupported envelope version, verification returns degraded state and quarantine.
- W5 markdown projection inherits this carrier; sidecar files are rejected for v1.4.2 because filesystem moves can separate content from signature metadata.

Replay surfaces:

- DB restore, SQL import, and markdown file restore can replay old bytes with a still-valid Ed25519 signature.
- Those cases are caught by ledger currentness, not cryptography alone.
- Fixtures must cover signed rollback, signed replay under a live locator, and tombstoned projection resurrection.

### §13. Fixture C wiring

Artifact 03 Fixture C is the W4-C key-compromise golden fixture.

Setup:

- Runtime has active key `dailyos-runtime-ed25519-2026-05`.
- Runtime signs a WP projection and a markdown projection.
- Ledger records stable projection rows and active `projection_signatures` rows for both projections.
- Test marks the active key compromised in `projection_signing_keys`.

Expected:

- SurfaceClient treats signatures from the revoked key as verification failures with `KeyRevoked`.
- Runtime revokes the entire pairing for that key id.
- Runtime provisions a replacement Ed25519 key in the runtime keychain.
- Runtime re-signs all live projections reachable from the projection ledger.
- Re-signed projections keep the same canonical signed payload bytes unless canonical claim state has changed, because detached envelope fields are not signed payload fields.
- Ledger records old signature ids as `revoked` or `superseded` and new signature ids as `active`; the stable `projection_ledger.current_signature_id` points at the new active row.
- No projection signed by the revoked key renders as trusted after revocation.

W4-C fixture implementation names:

| Fixture | Assertion |
|---|---|
| `dos569_fixture_c_key_compromise.rs` | Full Fixture C flow |
| `dos569_fixture_key_revoked_render.rs` | Revoked key fails read verification |
| `dos569_fixture_replacement_resign_queue.rs` | Live projections are queued and re-signed |
| `dos569_fixture_retired_key_history.rs` | Retired keys verify old bytes but cannot sign |

Fixture C is the acceptance fixture for key revocation, replacement-key provisioning, queued re-sign, and revoked-key rendering.

### §14. Replacement-key provisioning

Replacement-key provisioning is queued, not synchronous.
Artifact 03 leaves synchronous visible re-sign as an open question; DOS-569 commits v1.4.2 to queued re-sign with degraded render until refreshed.

Flow:

1. Active key is marked `revoked` or `rotating`.
2. Runtime generates a replacement Ed25519 keypair.
3. New key is written to keychain and `projection_signing_keys`.
4. `projection_replacement_keys` records old/new relationship.
5. Runtime walks `projection_ledger` joined to `projection_signatures` for live projections signed by old key.
6. Runtime inserts `projection_resign_queue` rows with `max_attempts = 5`.
7. Worker reuses canonical signed payload bytes from the active or last-valid signature row.
8. Worker signs with new key and writes only the detached signature envelope to the WP block attribute or markdown sentinel unless canonical state changed.
9. Worker inserts a new `projection_signatures` row, marks the old row `superseded` or `revoked`, updates `projection_ledger.current_signature_id`, and marks queue row `succeeded`.

Rules:

- Canonical payload bytes remain unchanged unless canonical claim state has changed.
- Re-sign does not "fix" tampered observed bytes; it signs canonical runtime-authored bytes.
- If projection storage write fails, queue row stays `failed` with `last_error`.
- Failed re-sign does not remove quarantine or degraded rendering.
- Replacement key activation is audited.
- A projection re-tampered within 120 seconds of the last re-sign is not re-signed again until cooldown expires.
- After `max_attempts`, the queue row transitions to `operator_escalation`; it does not loop indefinitely.

Queue idempotency:

- Unique key: `projection_id + old_signature_id + new_key_id`.
- Retrying a failed row reuses the same queue row and increments `attempts`.
- A projection that has already been re-signed with the new key is skipped.

### §15. Retired-key historical verification

Retired keys are verify-only.

Historical verification rule:

- `retired` keys remain published in `GET /v1/surface/keyring`.
- A projection signed while the key was active verifies successfully if the key is now retired.
- A retired key cannot sign new projections.
- A retired key can later become revoked if compromise is discovered.
- If a retired key becomes revoked, historical projections under that key become untrusted and render degraded.

Ledger rule:

- `projection_ledger.projection_id` remains stable for the live projection identity.
- Historical signature rows live in `projection_signatures`; `signature_id` is the primary key for each signature instance.
- Re-sign creates a new signature id, links it through `superseded_by_signature_id`, and updates `projection_ledger.current_signature_id`; it does not erase the old signature row.
- Audit replay can verify historical bytes with retired public keys by loading the historical `projection_signatures` row.

Validity windows:

- `valid_from` and `valid_until` are used for audit and clock-skew diagnostics.
- Verification does not trust the WP host clock.
- Runtime compares `issued_at` against key validity using runtime clock.
- Clock skew warnings do not convert a valid retired-key signature into tamper unless the issued timestamp is outside a credible runtime-issued window.

### §16. Class-level scope-filter conformance

W4-C conforms to the W4-B class-level scope-filter rule.
This section is conformance, not a reference.

Inherited rule:

> Class-level acceptance criterion (W4-C, W5-A, and any future claim-bound consumer): every read endpoint that returns claim-bound content must route through the same `Actor::SurfaceClient { scopes }` projection pipeline. Direct-key fetches by `event_log_id`, `mutation_id`, `correction_id`, `projection_id`, `composition_id`, `claim_id`, or any other bearer pointer are equally scope-gated. Out-of-scope requesters receive a redacted envelope or 404; never a leaked claim body.

W4-C endpoints and conformance:

Path-alpha helper contract:

- W4-B owns the published helper symbol for class-level scope filtering.
- W4-C uses whatever helper W4-B publishes; it does not fork a W4-C-local filter.
- Proposed W4-B-owned symbol name: `bridge_scope_filter::filter_claim_bound_envelope`.
- W4-C direct-key, ledger, quarantine, and projection-state reads call that helper after resolving claim refs and before returning any claim-bound envelope.


| Endpoint/read | Claim-bound? | Required conformance |
|---|---|---|
| `GET /v1/surface/keyring` | Indirectly, via runtime anchor and key status | Route through SurfaceClient; return scoped key set and no ledger bodies |
| `GET /v1/surface/projections/{projection_id}` | Yes | Verify actor scopes against all claim refs before returning projection metadata |
| `GET /v1/surface/projections/{projection_id}/ledger` | Yes | Return redacted ledger if scope misses any claim ref |
| `GET /v1/surface/quarantine/{quarantine_id}` | Yes | Never return observed excerpts unless claim scope permits |
| `GET /v1/surface/compositions/{composition_id}/verification` | Yes | Scope-gate by composition and contained claim refs |
| `GET /v1/surface/claims/{claim_id}/projection-state` | Yes | Scope-gate by claim id and surface grants |

Direct-key handling:

- `projection_id` is not authorization.
- `composition_id` is not authorization.
- `claim_id` is not authorization.
- `signature_id` is not authorization.
- `quarantine_id` is not authorization.
- `key_id` is not authorization.

Projection read implementation rule:

1. Parse actor as `Actor::SurfaceClient { instance, scopes }`.
2. Resolve requested object to its claim refs without returning body.
3. Evaluate scope permits for every claim-bound ref.
4. If permitted, return full scoped projection/ledger/quarantine envelope.
5. If not permitted, return redacted envelope or 404.
6. Emit audit with `scope_redacted = true` when redacted.

Keyring endpoint nuance:

- Public keys are public cryptographic material, but the endpoint still exposes runtime anchor state, key lifecycle, and surface pairing posture.
- It therefore uses SurfaceClient scope gating and audit.
- It never returns private key material or keychain labels.

Multi-claim block scope rule:

- A block can cite multiple claims. `projection_ledger_blocks.claim_id` is therefore forbidden as a scalar authorization shortcut.
- `projection_ledger_block_refs` is the only scope source for block-level claim refs.
- A read is fully scoped only when the actor has scope for every ref row attached to every returned block.
- If the actor has partial scope, return a redacted block envelope with `scope_redacted = true`; do not leak the unscoped claim body, field path, or raw provenance pointer.
- Mixed-scope fixtures must include a single block with two claim refs where only one is in scope.

W4-B V8 session-binding inheritance:

- W4-C endpoints inherit the W4-B V8 class-level `wp_user_id` session-binding precondition.
- If a W4-C request carries any asserted `wp_user_id`, the bridge compares it to the paired session's bound user before signature, currentness, scope, or keyring checks.
- Mismatch returns 403 `wrong_user`, emits `wrong_user_rejected`, and does not look up ledger rows.

### §17. Risks and edge cases

| Risk | Decision / mitigation |
|---|---|
| Clock skew on key validity | Runtime clock is authoritative; WP clock is ignored for verification |
| Concurrent rotation | `projection_signing_keys` transition transaction serializes active/rotating state |
| Partial re-sign failure | Queue row remains `failed`; old projection remains degraded if revoked-key signed |
| Envelope evolution | `dailyos_projection_version` is signed; version parser is explicit |
| Empty projection | Sign canonical empty `blocks[]` payload |
| Masked provenance | Masking renders as masked, not signature failure, per ADR-0108 |
| Copied signature | Fails because `projection_id`, block id, claim refs, and ordering are signed |
| Plugin JSON reorder | No failure if canonical DailyOS payload reconstructs identically |
| Plugin strips signature | `MissingSignature`, quarantine, audit |
| Unknown key while offline | Unverified state; no trusted render; refresh waits until available |
| Revoked retired key | Historical verification fails after revocation |
| Same bytes observed repeatedly | Increment `seen_count`; no duplicate quarantine spam |
| Byte-flip quarantine DoS | Coalesce per `projection_id` for 60 seconds and cap re-sign retries |
| DB restore signed rollback | Ed25519 may pass; ledger currentness returns `ProjectionVersionRollback` |
| SQL import signed replay | Current signature id and claim watermark must match live ledger before trusted render |
| Tombstoned projection resurrection | `locator_status = tombstoned` blocks trusted render and quarantines |
| Keychain/DB split-brain | Startup recovery reconciles keychain item and `projection_signing_keys` status |
| High-sensitivity tamper | W4-A may collapse body and show only degraded state |
| Runtime anchor mismatch | Quarantine; do not trust WP-provided anchor |
| Canonicalization bug | Property tests and golden vectors lock RFC 8785 output |

### §18. Acceptance criteria

1. **Every projection write is signed with Ed25519.** Fixture: `dos569_fixture_nominal_wp_projection.rs`.
2. **No HMAC/RSA/ECDSA/algorithm negotiation is accepted for projection authenticity.** Fixture: `dos569_fixture_unsupported_alg.rs`.
3. **Signed bytes use RFC 8785 canonical JSON and surface-suffixed domain separators.** WordPress uses `dailyos.wp_studio.projection.v1`; markdown uses `dailyos.markdown.projection.v1`. Fixture: `dos569_fixture_domain_separator.rs`.
4. **`SignedProjectionPayload` excludes the detached signature envelope entirely.** `key_id`, `signature_id`, `signed_at`, `alg`, `canonicalization`, `signature_b64`, and `keyring_version` are envelope/ledger metadata only. Fixture: `dos569_fixture_payload_excludes_envelope.rs`.
5. **Replacement-key re-sign over unchanged canonical state keeps `canonical_signed_payload_sha256` unchanged while producing a new `signature_id`.** Fixture: `dos569_fixture_resign_same_payload_new_signature.rs`.
6. **The signed payload uses structured `blocks[]`, not loose parallel arrays.** Fixture: `dos569_fixture_block_ordering_mutation.rs`.
7. **The W4-B quadruple `claim_id`, `claim_version`, `composition_id`, `composition_version` is signed.** Fixture: `dos569_fixture_ref_version_swap.rs`.
8. **W3-A envelope fields `dailyos_canonical_id`, `dailyos_signature`, `dailyos_source_runtime`, `dailyos_projection_version` are honored with payload/envelope split semantics.** Fixture: `dos569_fixture_w3a_envelope.rs`.
9. **Ledger schema preserves old and new signatures.** `projection_ledger` is stable by `projection_id`; `projection_signatures` is keyed by `signature_id`; only one active signature exists per live projection. Fixture: `dos569_fixture_signature_history_schema.rs`.
10. **Currentness check gates trusted render after Ed25519 succeeds.** Fixture: `dos569_fixture_current_signature_required.rs`.
11. **DB restore signed rollback returns `ProjectionVersionRollback` and quarantines.** Fixture: `dos569_fixture_db_restore_signed_rollback.rs`.
12. **SQL import signed replay returns `ProjectionVersionRollback` and quarantines.** Fixture: `dos569_fixture_sql_import_signed_replay.rs`.
13. **Tombstoned projection resurrection cannot render trusted.** Fixture: `dos569_fixture_tombstoned_projection_resurrection.rs`.
14. **Offline verification succeeds with cached public key and no network call.** Fixture: `dos569_fixture_offline_verify_no_runtime.rs`.
15. **Unknown key refresh calls keyring once and retries once.** Fixture: `dos569_fixture_unknown_key_refresh_once.rs`.
16. **Unknown key after refresh renders unverified and enqueues reconciliation.** Fixture: `dos569_fixture_unknown_key_after_refresh.rs`.
17. **Revoked key fails verification with `KeyRevoked`.** Fixture: `dos569_fixture_key_revoked_render.rs`.
18. **Retired key verifies historical bytes but cannot sign new projections.** Fixture: `dos569_fixture_retired_key_history.rs`.
19. **Private key storage is hardened.** Keychain ACL is scoped to runtime binary, secret-key path zeroizes where supported, and rotation crash recovery resolves DB/keychain split-brain. Fixtures: `dos569_fixture_keychain_acl.rs`, `dos569_fixture_rotation_crash_recovery.rs`.
20. **Replacement-key provisioning queues all live projections signed by old key and caps retries.** Fixture: `dos569_fixture_replacement_resign_queue.rs`.
21. **Fixture C key compromise flow passes end to end.** Fixture: `dos569_fixture_c_key_compromise.rs`.
22. **Re-sign worker does not loop on re-tampered projections.** Fixture: `dos569_fixture_resign_retamper_cooldown.rs`.
23. **Quarantine preserves tampered WP row or markdown bytes.** Fixture: `dos569_fixture_quarantine_preserves_observed.rs`.
24. **Quarantine events coalesce per `projection_id` for 60 seconds.** Fixture: `dos569_fixture_quarantine_coalescing.rs`.
25. **Tamper event audit is distinct from correction events.** Fixture: `dos569_fixture_tamper_not_correction.rs`.
26. **Audit fields are hashed/redacted for out-of-scope consumers with `scope_redacted = true`.** Fixture: `dos569_fixture_audit_scope_redaction.rs`.
27. **Signature and currentness checks run before W4-B 409 path.** Fixture: `dos569_fixture_tamper_before_409.rs`.
28. **Tamper errors map to `BridgeSurfaceError::ProjectionTampered` and do not emit `correction.claim`.** Fixture: `dos569_fixture_no_correction_payload_on_tamper.rs`.
29. **`GET /v1/surface/keyring` is SurfaceClient scope-gated and owned by `src-tauri/src/bridges/surface_client.rs`.** Fixture: `dos569_fixture_keyring_scope_gate.rs`.
30. **W4-C routes inherit W4-B V8 `wp_user_id` session binding.** Fixture: `dos569_fixture_wrong_user_rejected.rs`.
31. **Direct-key ledger/quarantine reads by `projection_id`, `composition_id`, `claim_id`, `signature_id`, or `quarantine_id` are scope-gated.** Fixture: `dos569_fixture_direct_key_scope_gate.rs`.
32. **Multi-claim blocks scope-filter through `projection_ledger_block_refs`, including mixed-scope refs in the same block.** Fixture: `dos569_fixture_mixed_scope_multi_claim_block.rs`.
33. **Studio `wp_cli` out-of-band edit lands in tamper ledger.** Fixture: `dos569_fixture_studio_wp_cli_tamper.rs`.
34. **Markdown signature carrier is a file-head HTML sentinel comment with base64url detached envelope.** Fixture: `dos569_fixture_markdown_signature_sentinel.rs`.
35. **Masked provenance remains a rendering mask, not a signature failure.** Fixture: `dos569_fixture_masked_provenance.rs`.
36. **Canonicalization property tests reject unstable map ordering and preserve block array ordering.** Fixture: `dos569_property_canonicalization.rs`.
37. **Verification hot path is bounded and cached by `(canonical_signed_payload_sha256, signature_id, keyring_version)`.** Fixture: `dos569_fixture_verification_perf_budget.rs`.
38. **W4-A trusted affordance is gated on `projection_signature_enforcement = enforce`; shadow mode renders visibly unverified/degraded.** Fixture: `dos569_fixture_shadow_mode_downgrades_trust.rs`.
39. **Enforcement mode changes emit `projection.enforcement_mode_changed`.** Fixture: `dos569_fixture_enforcement_mode_audit.rs`.
40. **W4-C implements `BridgeSurfaceError::ProjectionTampered` substrate-side** with `projection_id`, `signature_id`, `key_id`, `observed_signature_status`, and `quarantine_id`; the variant maps to 422 and never carries `correction.claim`. Fixture: `dos569_fixture_projection_tampered_variant_shape.rs`.
41. **W4-C implements `BridgeSurfaceError::ProjectionVersionRollback` substrate-side** with `projection_id`, `signed_composition_version`, `ledger_composition_version`, `signed_claim_version`, and `ledger_claim_version`; claim versions serialize as `u64` or `null`. Fixture: `dos569_fixture_projection_version_rollback_variant_shape.rs`.
42. **W4-C defers pairwise stale precedence fixtures to W4-B V9 ac §45/§46** while still returning the typed W4-C variants that those fixtures assert. W4-C substrate fixtures cover variant construction, quarantine linkage, and audit payloads.

### §19. Test plan

Unit tests:

- `projection_signing::signs_ed25519_and_verifies`.
- `projection_signing::rejects_non_ed25519_alg`.
- `projection_keyring::state_machine_valid_transitions`.
- `projection_keyring::state_machine_rejects_revoked_to_active`.
- `projection_canonical::rfc8785_golden_vectors`.
- `projection_canonical::payload_excludes_detached_signature_envelope`.
- `projection_quarantine::idempotency_key_coalesces_same_observed_hash`.
- `projection_quarantine::coalesces_changed_bytes_within_window`.
- `projection_currentness::rejects_current_signature_mismatch`.
- `projection_currentness::returns_projection_version_rollback_variant_shape`.
- `projection_currentness::rejects_tombstoned_locator`.
- `projection_scope::direct_key_fetch_requires_surface_scope`.

Integration tests:

- WP projection write signs and inserts ledger plus signature rows.
- Markdown projection write signs and inserts ledger plus signature rows.
- Markdown projection writes file-head sentinel comment and no sidecar.
- Read verification succeeds offline with cached keyring.
- Unknown key triggers one refresh and one retry.
- Tamper before stale write returns tamper, not W4-B 409.
- W4-B V9 ac §45/§46 pairwise precedence fixtures pass against W4-C's typed tamper and rollback variants.
- Revoked key triggers replacement-key provisioning and queued re-sign.
- Re-sign retry cap transitions to operator escalation.
- DB restore, SQL import, and tombstone resurrection fixtures all return `ProjectionVersionRollback`.
- Studio `wp_cli` mutation is detected as out-of-band tamper.
- Keyring endpoint is HMAC-gated, session-bound, and scope-gated.

Fixture tests:

- Artifact 03 Fixture A hostile markdown edit.
- Artifact 03 Fixture B hostile WP DB row mutation.
- Artifact 03 Fixture C signature key compromise simulation.
- Artifact 03 Fixture D benign plugin row mutation.
- Artifact 03 Fixture E signature copy between blocks.
- DOS-569 nominal verification.
- DOS-569 ordering mutation.
- DOS-569 runtime anchor mismatch.
- DOS-569 mixed-scope multi-claim block.
- DOS-569 shadow-mode degraded render.
- DOS-569 keychain ACL and rotation crash recovery.

Property-based tests:

- JSON object key ordering does not change canonical bytes.
- Array ordering does change canonical bytes.
- Equivalent numeric JSON forms canonicalize identically under RFC 8785 rules.
- Random block permutations fail verification unless order is restored.
- Detached signature envelope mutation outside `signature_b64` fails envelope/ledger currentness checks even when payload hash is unchanged.
- Claim ref/version pair swaps fail verification even if visible text is unchanged.

CI gates:

- Grep gate forbids projection authenticity code from using HMAC signing helpers.
- Grep gate forbids private key serialization into WP options/postmeta/block JSON.
- AST or grep gate requires all projection ledger direct-key endpoints call W4-B's published scope-filter helper, proposed as `bridge_scope_filter::filter_claim_bound_envelope`.
- AST or grep gate requires all W4-C `/v1/surface/*` routes live under `src-tauri/src/bridges/surface_client.rs` and run the W4-B V8 session-binding precondition.
- Perf gate verifies worst-case render stays under the accepted verification budget with cold cache and then with cache hits.
- Golden fixture files pin canonical bytes and signatures.

### §20. Rollout

Phase 1 - shadow signing:

- Sign every projection write.
- Store ledger and signature rows.
- Run verification and currentness checks on read.
- Do not block content render yet, but trusted provenance affordances are not allowed in shadow mode.
- W4-A must mark shadow-mode projections visibly unverified/degraded even if Ed25519 succeeds.
- Collect audit counts for missing signature, unknown key, canonicalization error, mismatch, and currentness rollback.
- Emit `projection.enforcement_mode_changed` when entering or leaving shadow mode.

Phase 2 - degraded rendering enforcement:

- W4-A consumes verification result.
- Verified and current projections render trusted provenance affordance only when enforcement mode is `enforce`.
- Failed, stale, replayed, rollback, or shadow-mode projections render degraded trust-band state.
- Quarantine rows are created for failures.
- Tamper events are visible in audit.

Phase 3 - replacement-key enforcement:

- Enable revocation path.
- Enable replacement-key provisioning.
- Enable queued re-sign.
- Keep retired-key historical verification enabled.

Kill-switch:

- Runtime flag: `projection_signature_enforcement = shadow | enforce | disabled`.
- `shadow` means "measure and visibly downgrade," not "render trusted while measuring".
- `disabled` is emergency only and still logs unsigned-projection reads.
- Kill-switch does not allow tampered bytes to mutate canonical claims.
- Kill-switch state is audited as `projection.enforcement_mode_changed`.

Rollback:

- Schema migrations are additive.
- Existing projections can render degraded if signatures are absent.
- No canonical claim rows depend on W4-C tables for correctness.
- Re-enabling enforcement reuses ledger rows where present and backfills signatures on next projection write.

### §21. Open questions for L0 reviewers

| ID | Question | Proposed L0 answer |
|---|---|---|
| Q1 | Is `ed25519-dalek` acceptable vs `ring`? | Yes; narrower Ed25519 API and better signature-trait fit |
| Q2 | Should visible projections re-sign synchronously after revocation? | No. v1.4.2 commits queued re-sign with degraded render until refreshed; synchronous visible re-sign is out of scope |
| Q3 | Should keyring route be public unauthenticated because keys are public? | No; key material is public, endpoint metadata is scope-gated |
| Q4 | Should quarantine store raw observed text? | No by default; store hashes/sanitized excerpt hashes unless sensitivity policy permits |
| Q5 | Should a retired key ever be removed from keyring? | No in v1.4.2; historical verification requires it |
| Q6 | Should tamper emit correction events for feedback loops? | No; tamper is integrity, not correction |
| Q7 | Should `dailyos_projection_version` equal `composition_version`? | No; projection envelope schema and composition watermark are distinct |
| Q8 | Should offline verification require embedded public key per block? | No; cache/keyring supplies public key; signature envelope carries `key_id` |
| Q9 | Should signature envelope metadata be included in signed bytes? | No; payload-excludes-envelope is the Phase 0 contract |
| Q10 | Should markdown use sidecar signatures? | No; v1.4.2 uses a file-head HTML sentinel comment |

### §22. Cross-wave dependencies

| Consumer | W4-C provides | Dependency |
|---|---|---|
| W3-A plugin skeleton | Signed W3-A envelope semantics | W3-A pins envelope fields |
| W3-C MCP server | Host-layer out-of-band mutations route to tamper ledger | Studio `wp_cli` is out-of-band |
| W4-A0 producer | Projection writer signs substrate-authored compositions | W4-B watermarks must exist first |
| W4-A renderer | Offline verification result, ledger currentness, quarantine state, and enforcement mode | W4-C must merge before trusted cached render; shadow mode downgrades trust |
| W4-B concurrency | Signature check before 409, signed watermark quadruple, `BridgeSurfaceError::ProjectionTampered`, `BridgeSurfaceError::ProjectionVersionRollback`, route owner, scope-filter, and `wp_user_id` session binding | W4-B V9 §6.5 amendment in commit `e8570cc4` inherited; W4-B V9 ac §45/§46 test pairwise precedence |
| DOS-589 signal dispatcher | Tamper/re-sign events may become subscriber inputs later | W4-C emits audit/ledger now; dispatcher integration can follow |
| W4-D fallback | Unknown block fallback remains signed if payload is signed | W4-D must not render unsigned raw payload as trusted |
| W4-E nonce | Feedback writes still require user-presence nonce | W4-C does not authorize mutations |
| W5-A feedback | Tamper is not feedback/correction | Distinct event channel prevents correction pollution |
| W5 markdown projection | Markdown signatures and comments use same ledger/keyring | Bidirectional markdown ingestion remains v1.4.6 |

W4-C implementation may start when L0 closes and W4-B substrate watermarks are available.
Stage-2 rendering cannot claim trusted cached projection support until W4-C verification is merged.
