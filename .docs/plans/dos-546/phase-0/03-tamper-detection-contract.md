---
status: spec:ready
date: 2026-05-10
related_adrs: [0102, 0105, 0108, 0129, 0130]
sub_contract: B — Tamper Detection
companion: ./02-concurrency-contract.md
open_questions: see ./INDEX.md (routed to W4-D L0 Prep)
---

# Tamper Detection Contract

## Context

This contract specifies the tamper-detection half of the DOS-546 Phase 0 split.

The companion concurrency contract owns write ordering, leases, stale writer rejection, and conflict policy.

This contract owns authenticity: detecting when projected claim data was changed outside the DailyOS runtime mutation path.

The target surface is WordPress Studio, with the WordPress database treated as a projection target, not source of truth.

Markdown projections are also projection targets and are covered by the same authenticity model.

The runtime remains authoritative for claims, provenance, projection manifests, signatures, and reconciliation state.

The surface may render a projection, but it must not silently promote a projection into canonical runtime truth.

The runtime mutation path is the only path allowed to create a trusted projection signature.

Out-of-band edits include direct markdown edits, direct WordPress DB row edits, WP plugin transforms, DB restores, SQL imports, admin UI edits, or any change that bypasses the runtime.

This contract assumes the claim model and `AbilityOutput<T>` shape from ADR-0102.

Every projected block originates from a typed ability output whose provenance lives exactly once in the `AbilityOutput<T>` wrapper.

This contract assumes the provenance envelope from ADR-0105 is the carrier for authenticity metadata references.

This contract also honors ADR-0108 rendering and privacy rules, including the 64KB serialized provenance cap.

Projection signatures must not cause provenance envelopes to exceed that cap.

For that reason, signed projection metadata references provenance by `invocation_id` and field paths rather than embedding full provenance into every WP block.

ADR-0129 and ADR-0130 were not present in this worktree at drafting time.

The contract therefore treats the following as Phase 0 assumptions requiring validation when those ADR files are available:

- WordPress database rows are projection rows.
- A composition `Block` is the surface-independent unit rendered into WordPress blocks and markdown sections.
- A block has a stable identity, schema version, payload, source claim refs, and provenance refs.
- Surface adapters can add surface-local attributes so long as canonical block payload remains reconstructable.

The design goal is detection, not prevention.

WordPress administrators, DB users, filesystem users, and plugins may still modify projection bytes.

DailyOS must detect those changes, downgrade trust, and reconcile from canonical runtime state.

Tamper detection is not authorization.

A valid projection signature only says "DailyOS runtime wrote these projection bytes for this claim snapshot."

It does not make Transform output trusted for mutation authorization; ADR-0102 trust rules still apply.

## Signing Model

DailyOS signs both canonical claim projection fields and canonical composition block payloads.

Signing only claim fields would miss hostile edits to presentation-critical block attributes such as title, emphasis, ordering, CTA text, and rendered markdown.

Signing only block payloads would miss an adapter that preserves visible bytes but swaps source claim refs, field paths, trust bands, or provenance refs.

The signed message is a `SignedProjectionPayload`.

It contains a canonical claim slice, a canonical block payload, and projection metadata that binds the two.

The claim slice includes only claim fields that are expected to survive projection.

It does not include mutable runtime-only bookkeeping that surfaces never see.

Signed claim fields:

- `claim_id`
- `claim_version`
- `claim_state`
- `claim_type`
- `subject_ref`
- `field_path`
- `text`
- `structured_value`
- `trust_score`
- `trust_band`
- `sensitivity`
- `data_source`
- `source_ref`
- `source_asof`
- `observed_at`
- `created_at`
- `updated_at`
- `provenance_ref`

Unsigned claim fields:

- runtime-local rowids
- lease ids
- sync attempt counters
- retry timestamps
- local repair flags
- fields explicitly marked non-projectable by the claim schema

The canonical block payload includes the surface-independent block shape before WP serialization.

Signed block fields:

- `block_id`
- `block_schema_version`
- `block_type`
- `claim_refs`
- `field_refs`
- `payload`
- `render_hints`
- `trust_band`
- `sensitivity`
- `provenance_ref`

Unsigned block fields:

- WordPress row id
- WordPress post id
- WordPress `post_modified`
- transient editor selection state
- adapter cache timestamps
- CSS classnames generated solely by WordPress
- plugin-owned attributes not interpreted by DailyOS

Surface adapters may add unsigned local fields, but those fields must live under a reserved adapter namespace such as `wp_local`.

DailyOS-rendered visible text must not live only in unsigned local fields.

If a user-visible field is rendered by DailyOS, it must be part of `block.payload` or `claim_slice` and therefore signed.

The signing key is generated and stored by the DailyOS runtime.

The signing private key lives in the runtime keychain.

For macOS, Phase 1 uses the platform keychain with an access control entry scoped to the DailyOS runtime identity.

The private key is never stored in WordPress, markdown files, WP options, postmeta, plugin settings, browser local storage, or exported block JSON.

The public verification key may be copied to WordPress and markdown projections.

The public key is not secret.

The public key is identified by a `key_id`.

`key_id` is a runtime-generated opaque id, not a filesystem path or platform keychain label.

DailyOS signs with Ed25519 and no other signature algorithm.

Ed25519 is chosen because it has small public keys and signatures, deterministic signatures, broad implementation support, strong verification performance for UI-adjacent reads, and avoids RSA size overhead and ECDSA nonce-footguns.

No RSA, P-256 ECDSA, HMAC, or algorithm negotiation is permitted for Phase 1.

The signed bytes are the RFC 8785 canonical JSON serialization of `SignedProjectionPayload`.

The signature envelope carries `alg: "Ed25519"` and `canonicalization: "RFC8785-JSON"`.

The signed payload includes an explicit domain separator string: `dailyos.wp_studio.projection.v1`.

The domain separator prevents the same signature from being reused as a signature over a different DailyOS object type.

The projection signature is stored in two places:

- Inside the projected WP block attribute or markdown comment, so the SurfaceClient can verify without a runtime round trip.
- Inside the runtime projection ledger, so reconciliation can compare observed projection bytes against last runtime-authored bytes.

The WP copy is a detached signature envelope embedded in block attributes under `dailyos.signature`.

The markdown copy is a detached signature envelope in a DailyOS-owned HTML comment next to the projected block.

The runtime copy is stored in `projection_ledger` keyed by `projection_id`, `surface`, `surface_locator`, `block_id`, `claim_id`, `claim_version`, and `signature_id`.

Do not store the only signature in a separate WordPress row.

A separate WP postmeta row may mirror the signature for query convenience, but it is not authoritative.

If the block attribute signature and WP postmeta mirror disagree, the block attribute is used for surface verification and the runtime ledger is used for reconciliation.

The provenance envelope carries authenticity metadata by reference.

It does not embed the full signed projection.

The provenance envelope may include:

```json
{
  "projection_authenticity": {
    "projection_id": "proj_wp_01JZ8WQ4P8GQ2M7K9S0N1B2C3D",
    "signature_id": "sig_01JZ8WQ7K64QH2F1Q6CVG4J17V",
    "key_id": "dailyos-runtime-ed25519-2026-05",
    "alg": "Ed25519",
    "canonicalization": "RFC8785-JSON",
    "signed_at": "2026-05-10T17:20:31Z"
  }
}
```

The runtime ledger stores the full canonical signed payload and signature.

The provenance envelope stores only the reference metadata needed to find that ledger record.

This keeps provenance within ADR-0108's 64KB serialized cap.

The projection signature must include the provenance `invocation_id` and field paths, not the full provenance JSON.

If a provenance envelope is later masked under ADR-0108, the signature remains valid for the historic projection bytes, but the rendered provenance details may be unavailable.

Masked provenance must render as masked, not as a signature failure.

Signing pseudocode:

```rust
fn sign_projection(
    runtime: &RuntimeContext,
    ability_output: &AbilityOutput<ClaimBackedComposition>,
    block: &Block,
    surface: SurfaceTarget,
    locator: SurfaceLocator,
) -> SignedProjection {
    let claim_slice = canonical_claim_slice(&ability_output.data.claims, block.claim_refs);
    let block_payload = canonical_block_payload(block);
    let payload = SignedProjectionPayload {
        domain: "dailyos.wp_studio.projection.v1",
        schema_version: 1,
        projection_id: ProjectionId::new(),
        surface,
        surface_locator: locator.stable_identity(),
        claim_slice,
        block_payload,
        provenance_ref: ProvenanceRef {
            invocation_id: ability_output.provenance.invocation_id,
            field_paths: block.field_refs.clone(),
        },
        produced_at: ability_output.provenance.produced_at,
        expires_at: None,
    };

    let canonical_bytes = rfc8785_json(&payload);
    let key = runtime.keychain.load_ed25519_signing_key(active_key_id())?;
    let signature = key.sign(&canonical_bytes);
    let envelope = SignatureEnvelope {
        signature_id: SignatureId::new(),
        alg: "Ed25519",
        canonicalization: "RFC8785-JSON",
        key_id: key.key_id(),
        signed_at: runtime.clock.now(),
        signature_b64: base64url(signature),
    };

    runtime.projection_ledger.insert(payload.clone(), envelope.clone(), canonical_bytes)?;

    SignedProjection {
        block: attach_signature_metadata(block, &payload.projection_id, &envelope),
        payload,
        envelope,
    }
}
```

JSON example of a signed Block:

```json
{
  "block_id": "blk_01JZ8Y6TZQ1NQ3K8D6SC2HT0WA",
  "block_schema_version": 1,
  "block_type": "dailyos/claim-callout",
  "claim_refs": [
    {
      "claim_id": "claim_01JZ8Y5JX2R0RFEXAMPLE",
      "claim_version": 7
    }
  ],
  "field_refs": [
    "/claims/[id=claim_01JZ8Y5JX2R0RFEXAMPLE]/text",
    "/claims/[id=claim_01JZ8Y5JX2R0RFEXAMPLE]/trust_band"
  ],
  "payload": {
    "title": "Renewal risk increased",
    "body": "The account has three unresolved executive escalations and no scheduled sponsor meeting.",
    "severity": "high"
  },
  "render_hints": {
    "variant": "warning",
    "density": "compact"
  },
  "trust_band": "untrusted",
  "sensitivity": "internal",
  "provenance_ref": {
    "invocation_id": "inv_01JZ8Y52PGQGEXAMPLE",
    "field_paths": [
      "/data/claims/0/text",
      "/data/claims/0/trust_band"
    ]
  },
  "dailyos": {
    "projection_id": "proj_wp_01JZ8Y7RGDVTEXAMPLE",
    "signature": {
      "signature_id": "sig_01JZ8Y83GVMWEXAMPLE",
      "alg": "Ed25519",
      "canonicalization": "RFC8785-JSON",
      "key_id": "dailyos-runtime-ed25519-2026-05",
      "signed_at": "2026-05-10T17:20:31Z",
      "signature_b64": "S6vU7b2JQ4t-EXAMPLE-7ZzA6S8R5j8tNQ83E1n0fQ"
    }
  }
}
```

The actual signed bytes exclude the `dailyos.signature.signature_b64` field.

They include the `projection_id`, `key_id`, algorithm metadata, canonical claim slice, block payload, and provenance ref.

## Verification Model

Verification runs in both the SurfaceClient and the runtime.

The SurfaceClient verifies before presenting the block as trusted.

The runtime verifies during event-bus replay, projection reads, and reconciliation.

SurfaceClient verification protects the user at the moment of rendering.

Runtime verification protects the canonical system from accepting tampered projection state during replay or repair.

Verification is fail-closed for trust and fail-open for pixels.

That means a block may optimistically render for first paint, but it must not render with a trusted provenance affordance until verification succeeds.

The SurfaceClient must not block first paint on signature verification.

The initial render may show the projection with a pending authenticity state.

The client schedules verification immediately after parsing block attributes.

The target budget is:

- first paint is not blocked by signature verification
- verification starts within 100ms of block parse
- visible authenticity state updates within 500ms for typical pages
- runtime reconciliation enqueue happens within 2 seconds of a local verification failure when the runtime is reachable

The initial optimistic render must avoid overstating authenticity.

Allowed pending UI:

- neutral provenance affordance
- "checking authenticity" state in the details panel
- no trusted checkmark

Disallowed pending UI:

- showing "verified"
- allowing the projection to authorize a mutation
- hiding known prior verification failure for the same `projection_id`

When verification succeeds, the SurfaceClient may render the normal provenance affordance per ADR-0108.

When verification fails, the SurfaceClient shows a tamper banner and downgrades the block's trust band for rendering.

The block body may remain visible if sensitivity policy allows it, but it must be marked as unverified projection data.

For high-sensitivity blocks, the SurfaceClient may collapse the body and show only the banner until runtime reconciliation completes.

The failure banner copy should be short and operational:

`This DailyOS projection was changed outside the runtime. DailyOS is reconciling it.`

The banner must not expose internal keys, raw signatures, or field values to actors who would not otherwise see them.

Failure modes:

- Missing signature: render as unverified and enqueue reconciliation.
- Unknown `key_id`: render as unverified and ask runtime for key-ring refresh; if still unknown, enqueue reconciliation.
- Unsupported `alg`: refuse trusted rendering and enqueue reconciliation.
- Canonicalization error: render as unverified; runtime records `ProjectionCanonicalizationFailed`.
- Signature mismatch: render tamper banner and enqueue reconciliation.
- Signature valid but key revoked: render tamper banner with revoked-key state and enqueue re-sign pass.
- Signature valid but provenance ref missing: render soft warning; authenticity may be valid but explainability is degraded.
- Signature valid but claim version stale: render stale state; concurrency contract owns whether newer canonical data replaces it.

Verification pseudocode:

```rust
fn verify_projection(
    block_json: Value,
    public_keys: &ProjectionKeyRing,
    runtime_hint: Option<&RuntimeLedgerRecord>,
) -> VerificationResult {
    let envelope = extract_signature_envelope(&block_json)
        .ok_or(VerificationError::MissingSignature)?;

    if envelope.alg != "Ed25519" {
        return Err(VerificationError::UnsupportedAlgorithm(envelope.alg));
    }

    if envelope.canonicalization != "RFC8785-JSON" {
        return Err(VerificationError::UnsupportedCanonicalization(envelope.canonicalization));
    }

    let public_key = public_keys
        .lookup(&envelope.key_id)
        .ok_or(VerificationError::UnknownKey(envelope.key_id.clone()))?;

    if public_key.revoked {
        return Err(VerificationError::KeyRevoked(envelope.key_id.clone()));
    }

    let payload = reconstruct_signed_payload_from_block(&block_json, runtime_hint)?;
    require_domain(&payload, "dailyos.wp_studio.projection.v1")?;
    let canonical_bytes = rfc8785_json(&payload);
    let signature = base64url_decode(&envelope.signature_b64)?;

    public_key
        .verify(&canonical_bytes, &signature)
        .map_err(|_| VerificationError::SignatureMismatch)?;

    Ok(VerificationSuccess {
        projection_id: payload.projection_id,
        claim_versions: payload.claim_slice.claim_versions(),
        provenance_ref: payload.provenance_ref,
        verified_at: now(),
    })
}
```

Event-bus replay rule:

The runtime may replay projection events only if the event's projection payload verifies against the runtime ledger or was produced inside the current runtime transaction.

The runtime must not accept a WP row, markdown file, or external projection event as a canonical claim mutation merely because it contains a valid block shape.

Only signed DailyOS runtime projection payloads may refresh projection ledger state.

Only claim service mutation APIs may update canonical claim rows.

## Out-of-Band Detection

### Hostile Markdown Edit

Scenario:

A user or attacker edits a markdown projection on disk between runtime writes.

Examples:

- changes a risk statement from "low" to "high"
- removes the tamper signature comment
- changes a source claim id
- inserts prompt-injection text into a projected claim body

Detection actor:

- runtime file watcher when available
- runtime reconciliation pass
- any SurfaceClient that renders markdown-derived blocks

What gets detected:

- missing signature comment
- signature mismatch after reconstructing canonical payload
- changed bytes in signed text ranges
- changed claim refs or field refs
- changed provenance ref

User-visible response:

- markdown render shows an authenticity banner on the affected block
- provenance details show `projection_tampered`
- runtime emits a divergence claim with field-pointer and byte range
- reconciliation restores the projection if auto-restore policy allows

The runtime must distinguish markdown user notes from DailyOS-owned projected blocks.

Only DailyOS-owned blocks are verified under this contract.

User-authored markdown outside DailyOS projection sentinels is not tamper.

### Hostile WP DB Row Mod

Scenario:

An attacker with DB access modifies `wp_posts.post_content` or a block attribute directly.

Examples:

- changes `payload.body`
- changes `trust_band`
- changes `claim_refs`
- replaces the signature envelope
- copies a valid signature from another block

Detection actor:

- SurfaceClient on next render
- runtime WP projection reader
- runtime reconciliation pass

What gets detected:

- signature mismatch for payload edits
- domain/projection id mismatch for copied signatures
- field-ref mismatch for swapped claim refs
- unknown or revoked key for fake key ids
- missing ledger entry for unknown projection ids

User-visible response:

- next render shows the tamper banner
- trusted provenance affordance is disabled
- block is marked `unverified_projection`
- runtime enqueues repair from canonical claim and block state
- after repair, the banner clears on refresh or live update

The SurfaceClient does not need DB write access to detect the tamper.

It only needs the public key and the block bytes.

The runtime needs write access only for reconciliation.

### WP Plugin Update That Mutates Rows

Scenario:

A plugin update rewrites block markup or attributes without malicious intent.

Examples:

- normalizes JSON attribute order
- strips unknown `dailyos` attributes
- rewrites HTML comments
- migrates block names
- expands shorthand attributes into default values

Detection actor:

- SurfaceClient on next render
- runtime reconciliation pass
- runtime WP adapter compatibility check

What gets detected:

- no issue if canonical DailyOS payload reconstructs identically
- signature mismatch if signed payload fields changed
- missing signature if plugin strips DailyOS metadata
- adapter migration warning if block type changed but payload remains reconstructable

User-visible response:

- if only byte formatting changed outside signed fields, no banner
- if signed fields changed or signature is stripped, render authenticity banner
- runtime classifies the divergence reason as `plugin_mutation_suspected` when WP plugin version changed near the edit timestamp
- reconciliation restores or reprojects with the current adapter

Plugin transforms are not automatically trusted.

A benign plugin edit still bypassed the runtime mutation path.

The only difference from hostile tamper is incident classification and user copy.

## Reconciliation Pass

Reconciliation is event-driven, on-read, and periodic.

Event-driven triggers:

- SurfaceClient verification failure
- WP webhook or polling delta for a projected post
- filesystem watcher delta for a markdown projection
- runtime event-bus replay detecting a projection mismatch
- key revocation

On-read triggers:

- SurfaceClient sees a block with missing, invalid, unknown, or revoked signature
- runtime reads a projection for background sync and sees ledger mismatch

Periodic triggers:

- Phase 1 runs a sweep every 5 minutes while the runtime is active
- the sweep is bounded by a per-pass limit to avoid starving normal work
- high-priority tamper reports jump the queue

When a tampered projection is detected, DailyOS records a `ProjectionDivergence` runtime claim.

The divergence claim is a first-class claim-like record, not a silent log line.

It contains:

- `projection_id`
- `surface`
- `surface_locator`
- `block_id`
- `claim_id`
- `claim_version`
- `detected_by`
- `detected_at`
- `verification_error`
- `field_pointer`
- `byte_range`
- `observed_excerpt_hash`
- `canonical_excerpt_hash`
- `reconciliation_status`

The divergence claim must not store sensitive raw tampered text unless sensitivity policy permits it.

It should store hashes and sanitized excerpts by default.

Auto-restore policy:

- Restore automatically when the projection is DailyOS-owned and canonical runtime state is available.
- Do not auto-restore when the block has been detached from DailyOS ownership.
- Do not auto-restore if the companion concurrency contract reports an unresolved legitimate edit conflict.
- Do not auto-restore if the current runtime key is revoked and no replacement key has been provisioned.

Auto-restore writes a fresh projection from canonical runtime claims.

Auto-restore signs with the active Ed25519 key.

Auto-restore updates the runtime projection ledger.

Auto-restore appends a reconciliation event to the audit trail.

If auto-restore is disabled or blocked, DailyOS keeps the banner visible and surfaces the divergence claim in the appropriate admin/user review surface.

Repeated detection must be idempotent.

Idempotency key:

`projection_id + surface_locator + block_id + verification_error + observed_payload_hash`

If the same tampered bytes are seen repeatedly, DailyOS updates `last_seen_at` and `seen_count`.

It does not create duplicate divergence claims.

If the tampered bytes change, DailyOS records a new divergence revision under the same projection divergence thread.

If reconciliation restores the canonical projection and the next verification succeeds, DailyOS marks the divergence claim `resolved`.

Resolved divergence claims remain queryable for audit.

They do not keep rendering a banner after the projection verifies.

## Byte-Diff Approach

Diffs are computed against the runtime's last-known canonical projection payload, not against the current canonical claim alone.

Reason:

The user-visible projection is the signed artifact.

A newer canonical claim may have legitimately changed after the projection was written.

Diffing only against latest canonical claim would conflate staleness with tamper.

The primary diff baseline is the canonical signed projection bytes stored in the runtime ledger.

The secondary baseline is the current canonical claim state, used only to determine the repair target.

The runtime ledger stores:

- `projection_id`
- `surface`
- `surface_locator`
- `block_id`
- `claim_id`
- `claim_version`
- `canonical_signed_payload_json`
- `canonical_block_payload_json`
- `canonical_claim_slice_json`
- `signature_envelope_json`
- `signed_at`
- `key_id`
- `last_verified_at`
- `last_seen_observed_hash`
- `reconciliation_status`

Granularity is field-pointer first, byte-range second.

Field-pointer granularity uses JSON Pointer for canonical block and claim structures.

Examples:

- `/block_payload/payload/body`
- `/block_payload/trust_band`
- `/claim_slice/claims/[id=claim_01]/text`
- `/provenance_ref/invocation_id`

Byte-range granularity is used for markdown projections and WP serialized block text where a direct JSON pointer is unavailable.

For markdown, the runtime maps byte ranges inside DailyOS sentinels back to field pointers using the render map captured at write time.

For WP blocks, the adapter maps parsed block attributes back to canonical block field pointers.

Whole-block granularity is acceptable only when parsing fails completely.

Storage cost:

Phase 1 stores one canonical signed projection payload per projected block.

The expected payload size is small relative to provenance: claim slice plus block payload plus signature envelope.

The ledger does not duplicate full provenance.

The ledger references provenance by `invocation_id` and field paths.

The ledger may compress canonical payload JSON if storage measurements require it, but compression is an implementation detail.

The signature is over uncompressed canonical JSON bytes.

The runtime must retain canonical baselines at least as long as the corresponding projection is live.

When a projection is deleted by DailyOS, the ledger row is tombstoned, not immediately purged.

Tombstones allow detection of projection resurrection by external restore or DB rollback.

## Phase 1 Acceptance Fixtures

### Fixture A: Hostile Markdown Edit

Setup:

- Runtime writes a DailyOS-owned markdown projection for `claim_001`.
- Projection contains a signed block with field pointer `/claim_slice/claims/[id=claim_001]/text`.
- Test mutates the markdown body bytes inside the DailyOS sentinel without updating the signature.

Expected:

- Runtime file watcher or reconciliation sweep detects the edit within 10 seconds while runtime is active.
- Verification fails with `SignatureMismatch`.
- Runtime records exactly one `ProjectionDivergence` for the same observed payload hash.
- Divergence includes field pointer `/claim_slice/claims/[id=claim_001]/text`.
- Divergence includes a byte range for the changed markdown bytes.
- User-visible markdown render shows the tamper banner.
- Repeated reads of the same tampered file increment `seen_count` and do not create duplicate divergence claims.

### Fixture B: Hostile WP DB Row Mod

Setup:

- Runtime writes a signed `dailyos/claim-callout` block into a WP post projection.
- Test updates `wp_posts.post_content` directly to change `payload.body`.
- Test does not update the signature.

Expected:

- SurfaceClient verification fails on next render.
- Initial paint may render pending authenticity state.
- Within 500ms of client verification, the tamper banner is visible.
- Trusted provenance affordance is disabled for the block.
- Runtime receives or discovers the verification failure and enqueues reconciliation.
- Runtime restores the block from canonical projection or current canonical claim according to concurrency status.
- After restoration, verification succeeds and the banner clears.

### Fixture C: Signature Key Compromise Simulation

Setup:

- Runtime has active key `dailyos-runtime-ed25519-2026-05`.
- Test marks that key compromised in the runtime key registry.
- Existing projections still have signatures from the compromised key.

Expected:

- SurfaceClient treats signatures from the revoked key as verification failures with `KeyRevoked`.
- Runtime revokes the entire pairing for that key id.
- Runtime provisions a replacement Ed25519 key in the runtime keychain.
- Runtime re-signs all live projections reachable from the projection ledger.
- Re-signed projections keep the same canonical block payload unless canonical claim state has changed.
- Ledger records old signature ids as revoked and new signature ids as active.
- No projection signed by the revoked key renders as trusted after revocation.

### Fixture D: Benign WP Plugin Row Mutation

Setup:

- Runtime writes a signed WP block.
- Test simulates a plugin update that strips unknown `dailyos.signature` attributes while leaving visible payload unchanged.

Expected:

- SurfaceClient verification fails with `MissingSignature`.
- Runtime classifies divergence as `plugin_mutation_suspected` when plugin version changed near the row update.
- User-visible banner says the projection was changed outside DailyOS, without accusing a hostile actor.
- Runtime reprojects and re-signs the block.
- Subsequent verification succeeds.

### Fixture E: Signature Copy Between Blocks

Setup:

- Runtime writes two signed blocks for two different claims.
- Test copies the signature envelope from block A into block B.
- Test leaves block B payload otherwise unchanged.

Expected:

- Verification of block B fails because the signed payload binds `projection_id`, `block_id`, claim refs, and field refs.
- Runtime records divergence against block B only.
- Runtime does not mark block A as tampered.
- The failure mode is `ProjectionIdentityMismatch` or `SignatureMismatch`.

## Open Questions

ADR-0129 and ADR-0130 were not available in this worktree.

Validate the assumed `Block` shape against ADR-0130 before Phase 1 implementation.

Validate WordPress storage locator terms against ADR-0129 before creating ledger schema.

Decide whether Phase 1 needs a WP postmeta mirror for querying signatures, or whether block attributes plus runtime ledger are sufficient.

Decide whether high-sensitivity blocks should collapse by default during pending verification or only after failure.

Decide the exact user-facing location for divergence claims: Activity feed, provenance details, admin diagnostics, or all three.

Decide whether markdown auto-restore should preserve external user edits outside DailyOS sentinels in the same file by default.

Decide retention duration for tombstoned projection ledger rows after a projection is deleted by DailyOS.

Decide whether key compromise re-sign should be synchronous for visible projections or queued with banners until each projection is refreshed.

Confirm whether event-bus replay should reject all unsigned legacy projections immediately or support a one-time migration signer.
