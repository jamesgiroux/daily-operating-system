# W4-E L0 packet - user-presence nonce lifecycle for feedback writes
Date: 2026-05-13 (V2)
Project: v1.4.2 - Personal Intelligence Engine: WordPress Foundation
Parent: DOS-546
Wave: 4 stage-2 (after W4-B; before W5-A feedback routing)
Issue: DOS-571 (W4-E: user-presence nonce lifecycle for feedback writes)
Linear: https://linear.app/a8c/issue/DOS-571
Working branch: dos-546-w4-e-l0-prep
This packet captures the W4-E contract decisions resolved at L0. Linear remains the canonical execution contract. This packet supersedes it only where it makes an implicit decision explicit, or where it reconciles Linear, Phase 0 artifact 10, and W4-B V8.

## Changelog

- **V2 (2026-05-13):** Cycle 1 reviewer fold. eng CONDITIONAL APPROVE, /cso CONDITIONAL, /plan-devex-review CONDITIONAL, and /codex challenge CONDITIONAL findings are folded into the contract. Material changes: inherit W4-B V8 §17 `wp_user_id` session binding and §37 `src-tauri/src/bridges/surface_client.rs` route ownership; make `claim_version` mandatory in the nonce binding; pin immutable `PresenceNonceBinding` lifecycle transitions; pin HMAC-SHA256 digest derivation from service-local W2-B secret material; pin W2-D rate-limit keys and per-session outstanding nonce ceiling; require store-pressure LRU invalidation; serialize freshness checks with consume; require W5-A to carry nonce-bound expected versions into W4-B CAS; pin `tokio::sync::Mutex<HashMap<NonceDigest, PresenceNonceBinding>>` plus composition invalidation index; make lazy invalidation on verify the correctness boundary; require `Actor::SurfaceClient` for `/nonce/*`; add authenticated rejection audit shape; add v175 persistence restart-drain rule; add fixtures for immutability, malformed `claim_version`, rate exhaustion, store pressure, cross-actor calls, stale claim version, freshness/consume race, and restart replay.
- **V1 (2026-05-13):** Initial L0 packet draft for DOS-571.
## Status snapshot
- Linear DOS-571 is Todo, High priority.
- Linear scope: nonce issue/verify endpoints and WP-mediated nonce request path.
- Linear dependency: blocked by DOS-567, DOS-557, DOS-558, DOS-559.
- Linear downstream: blocks DOS-573.
- W4-B V8 is the upstream composition/version and SurfaceClient bridge contract.
- W4-B V8 reserves W4-E migration slot v175 only if persisted.
- Local branch migration tail currently reaches v167.
- W4-B V8 says dev has v168, v1.4.1 owns v169, W4-B owns v170.
- W4-C owns v171-v174 by W4-B V8 recommendation.
- W4-E must not use v168-v174 even if this temp branch does not show them.
- Trust-boundary work: `/cso` is required at L0 and L2.
- Cycle 1 panel: eng CONDITIONAL APPROVE; /cso CONDITIONAL with 3 CRITICAL, 3 HIGH, 3 MEDIUM, 3 LOW; /plan-devex-review CONDITIONAL; /codex challenge CONDITIONAL with 1 HIGH, 2 MEDIUM, 1 LOW.
- V2 folds all named Cycle 1 findings into the packet and requires a Cycle 2 panel before L0 closure.
- This packet does not implement code.
- This packet does not commit.
## Scope from DOS-571
W4-E proves fresh human presence for feedback writes.
W4-E ships:
1. `POST /v1/surface/nonce/issue`.
2. `POST /v1/surface/nonce/verify`.
3. Single-use server-bound nonces.
4. Binding to `session_id`.
5. Binding to `wp_user_id`.
6. Binding to `claim_id`.
7. Binding to `field_path`.
8. Binding to `action`.
9. Binding to `composition_version`.
10. Binding to `claim_version` when W4-B claim versions are available.
11. Expiry at or below 60 seconds.
12. Atomic consume.
13. Audit emission on issue, verify, reject, invalidation.
14. WP block-editor nonce request through WP REST and PHP.
W4-E does not ship:
1. Full W5-A feedback router.
2. `/v1/surface/feedback` write semantics.
3. Gutenberg re-render UI.
4. W5-A version-changed re-confirm UX.
5. W4-B `commit_composition`.
6. W4-C projection signing.
7. DOS-589 dispatcher.
8. New claim feedback substrate.
## Acceptance reconciliation
There is a real contract conflict.
Phase 0 artifact 10 says:
- replay is `409`;
- expiry is `410`;
- wrong field/action are `409`;
- wrong version is `409`.
The v1.4.2 project issue slice says:
- mismatched action is `403`;
- wrong field is `403`;
- expired is `403`;
- already consumed is `403`.
Linear DOS-571 says:
- wrong version is `409`;
- not generic authorization failure;
- only when actor/session are otherwise valid.
L0 decision:
- Presence-token failures return `403`.
- Stale claim or composition versions return `409`.
- Malformed requests return `400`.
- Auth failures return `401` or `403` before nonce lookup.
- Rate limits return `429`.
- Reason codes stay specific for W5-A UI.
This preserves Linear's stale-version rule and the task's `403` nonce-failure rule.
## Pre-work confirmed
**Headline finding:** W4-E is a gate, not a new feedback path.
It composes with:
- W2-A loopback HTTP endpoint.
- W2-B HMAC signing.
- W2-C pairing/session.
- W2-D rate-limit matrix.
- W4-B composition and claim watermarks.
- W5-A feedback router.
- Existing claim feedback services.
- Existing SurfaceClient audit helper.
### Already in W4-B V8
- **W4-B §1:** `ClaimRef.field_path` gives W4-E a canonical field target.
- **W4-B §3:** `composition_version` is assigned by `commit_composition`.
- **W4-B §3:** every accepted composition commit advances the version.
- **W4-B §8:** `composition_versions` is durable.
- **W4-B §8:** `commit_composition` uses `BEGIN IMMEDIATE`.
- **W4-B §15:** `version_events` is the version-change outbox.
- **W4-B §15:** replay ordering uses `event_seq`.
- **W4-B §15:** cursor is UUIDv4.
- **W4-B §17:** `validate_session_bound_wp_user_id(actor, payload)` is the inherited SurfaceClient bridge precondition for every request carrying `wp_user_id`.
- **W4-B §17:** body/query/header asserted `wp_user_id` never becomes the trusted actor identity; mismatch returns `403 wrong_user` before downstream nonce code.
- **W4-B §37:** `src-tauri/src/bridges/surface_client.rs` is the canonical owner for all `/v1/surface/*` endpoints.
- **W4-B §37:** W4-E registers `/v1/surface/nonce/{issue,verify}` in that module and does not create a second SurfaceClient route owner.
- **W4-B interlock row:** W4-E nonce tuple includes `composition_version`.
- **W4-B §13 task note plus interlock row:** composition refresh invalidates nonces.
- **W4-B interlock row:** W5-A consumes presence nonce.
### Already in audit infrastructure
- `AuditFields::new(category, detail)` exists.
- `AuditFields::with_wp_user_id(wp_user_id)` exists.
- `emit_surface_audit(logger, event_kind, actor, fields)` exists at line 203.
- `Actor::SurfaceClient` audit requires `wp_user_id`.
- `actor_instance` comes from the actor, not caller-controlled detail.
- `actor_scopes` comes from the actor, not caller-controlled detail.
- Tests already cover SurfaceClient audit write and missing `wp_user_id`.
- W4-E nonce event is named in audit helper docs.
### Already in feedback substrate
- `FeedbackAction` is a closed typed enum.
- `record_claim_feedback` is the service-layer feedback write entry.
- `record_claim_feedback` persists `claim_feedback`.
- `record_claim_feedback` applies lifecycle effects.
- `record_claim_feedback` emits feedback signals.
- W5-A calls this service after nonce verification.
- W4-E does not call it during issue or verify.
### Already in WP planning
- `DailyOS_Runtime_Client` is the PHP runtime client surface.
- WP executor verifies capability and pairing.
- JS never receives bearer material.
- JS never receives HMAC key material.
- JS calls WP REST.
- PHP calls Rust runtime.
- Direct browser-to-runtime calls are out of scope.
### Already in rate-limit planning
- Artifact 09 places nonce validation inside `SurfaceClientBridge`.
- Feedback writes consume SurfaceClient budget.
- Feedback writes consume WP user budget.
- Feedback writes consume WP site budget.
- Feedback writes consume ability budget.
- Feedback writes consume scope budget.
- Nonce failures also count against nonce-failure budget.
### Migration slot audit
- Local branch: migration list reaches v167.
- W4-B V8: v168 already merged on dev.
- W4-B V8: v169 is v1.4.1 in-flight.
- W4-B V8: W4-B owns v170.
- W4-B V8: W4-C owns v171-v174.
- W4-B V8: W4-E owns v175 if persisted.
- Default W4-E storage is in-memory, so no migration.
- Persistence requires v175 and packet revision.
## What W4-E authors net-new
| Surface | Status | Scope |
|---|---|---|
| `SurfaceNonceService` | Missing | issue, verify, consume, expire, invalidate |
| In-memory nonce store | Missing | digest-keyed binding map |
| Optional persisted nonce store | Conditional | v175 only |
| `/v1/surface/nonce/issue` | Missing | runtime endpoint |
| `/v1/surface/nonce/verify` | Missing | runtime endpoint |
| `src-tauri/src/bridges/surface_client.rs` route registration | Inherited from W4-B §37 | register nonce endpoints in canonical SurfaceClient module |
| `validate_session_bound_wp_user_id` precondition use | Inherited from W4-B §17 | bridge runs before nonce service dispatch |
| `PresenceNonceBinding` | Missing | tuple plus timestamps |
| `PresenceNonceAction` | Missing | four feedback verbs |
| Atomic consume primitive | Missing | single winner |
| Invalidation observer | Missing | W4-B composition events |
| Lazy invalidation on verify | Missing | correctness boundary for stale compositions |
| W2-D nonce budget integration | Missing | issue/verify/failure charging by pinned axes |
| Outstanding nonce ceiling | Missing | per-session cap and 429 response |
| Store-pressure eviction | Missing | LRU invalidation and audit |
| Audit detail builder | Missing | shared nonce audit shape |
| WP nonce request JS | Missing | block editor gesture path |
| Negative fixtures | Missing | all rejection paths |
## Directional decisions resolved at L0
### §1. Upstream contracts consumed from W4-B
W4-E consumes W4-B rather than redefining watermarks.
Contracts:
- Field identity comes from W4-B §1.
- Composition freshness comes from W4-B §3 and §8.
- Version-change delivery comes from W4-B §15.
- Session-bound `wp_user_id` validation comes from W4-B §17.
- SurfaceClient endpoint ownership comes from W4-B §37.
- Composition refresh invalidation is the W4-B interlock row for W4-E.
- Stale composition handling aligns with W4-B stale composition `409`.
Refresh rule:
```text
commit_composition(composition_id) advances N -> N+1
  -> outstanding nonces for composition_id and version <= N become invalid.
```
The user must see the refreshed block and click again.

V8 inheritance rule:
- `src-tauri/src/bridges/surface_client.rs` owns both nonce endpoints.
- The bridge entry point calls `validate_session_bound_wp_user_id(actor, payload)` before `services::surface_nonce::*` is reached.
- W4-E must not add an independent body-level `wp_user_id` trust path.
- W4-E may compare tuple `wp_user_id` values only after the bridge has established the truthful session-bound user.
- Body mismatch returns `403 wrong_user` and never reaches nonce lookup, claim lookup, rate-limited issue, or consume code.
- The W4-E acceptance suite includes an ordering fixture proving the bridge precondition runs before nonce code.
### §2. Nonce tuple definition
Minimum tuple required by this task:
```text
(session_id, wp_user_id, claim_id, field_path, action, claim_version, composition_version)
```
Implementation tuple:
```text
(
  surface_client_id,
  session_id,
  wp_user_id,
  claim_id,
  field_path,
  action,
  claim_version,
  composition_id,
  composition_version
)
```
Why include the extra fields:
- Linear DOS-571 explicitly includes `claim_version`.
- W4-B W5-A interlock names `(claim_id, claim_version, field_path)`.
- `surface_client_id` is needed for audit correlation.
- `composition_id` is needed for refresh invalidation.
Rules:
- `session_id` binds one paired session.
- `wp_user_id` binds one WP editor user.
- `claim_id` binds one substrate-authored claim.
- `field_path` binds one W4-B field path.
- `action` binds one feedback verb.
- `claim_version` binds the observed claim version and is mandatory, not `Option`.
- `composition_id` binds invalidation scope.
- `composition_version` binds the rendered composition version.

Required-field rule:
- `claim_version` is a wire-required `u64` on both issue and verify.
- Missing, null, empty string, float, negative, object, array, or stringly typed `claim_version` rejects with `400 malformed_claim_version`.
- W4-E does not infer `claim_version` from `claim_id` during issue to paper over a malformed request.
- W4-E does query the claim store for the current version to decide freshness; that query is validation, not inference of the request tuple.
### §3. Action vocabulary
W4-E wire actions:
```text
correct | dismiss | corroborate | contradict
```
W5-A maps those to typed claim feedback.
Recommended mapping:
| W4-E action | W5-A meaning | Candidate substrate action |
|---|---|---|
| `correct` | corrected value | `NeedsNuance` |
| `dismiss` | hide/reject on surface | context-dependent |
| `corroborate` | confirm current | `ConfirmCurrent` |
| `contradict` | mark wrong | `MarkFalse` |
W4-E validates action identity only.
W5-A owns semantic mapping.
### §4. Storage model
Default storage is in-memory primary.
Rationale:
- TTL is at most 60 seconds.
- Runtime restart should force re-click.
- Nonce is not an authorization token.
- Durable audit already exists.
- The task explicitly requests in-memory primary.
Binding shape:
```rust
pub struct PresenceNonceBinding {
    nonce_digest: NonceDigest,
    fields: PresenceNonceBindingFields,
    lifecycle: PresenceNonceLifecycle,
    _sealed: private::Sealed,
}

pub struct PresenceNonceBindingFields {
    surface_client_id: SurfaceClientId,
    session_id: SessionId,
    wp_user_id: u64,
    claim_id: ClaimId,
    field_path: FieldPath,
    action: PresenceNonceAction,
    claim_version: u64,
    composition_id: CompositionId,
    composition_version: u64,
    generated_at: DateTime<Utc>,
    expires_at: DateTime<Utc>,
}

pub struct PresenceNonceLifecycle {
    consumed_at: Option<DateTime<Utc>>,
    invalidated_at: Option<DateTime<Utc>>,
}
```
Immutability rule:
- `PresenceNonceBinding` is immutable after insertion.
- Tuple fields are constructor-only.
- The struct is sealed; external modules cannot construct partial bindings or mutate fields.
- There are no public setters for tuple fields.
- The only permitted lifecycle mutations are atomic compare-and-set transitions from `None` to `Some(now)` for `consumed_at` or `invalidated_at`.
- Lifecycle methods return `AlreadyConsumed`, `AlreadyInvalidated`, or `LostRace` instead of overwriting prior timestamps.
- `consumed_at` and `invalidated_at` are mutually exclusive terminal states.
- Tests must fail if a future patch exposes a mutable tuple-field setter or writes tuple fields after insertion.

Type-system enforcement:
```rust
mod private {
    pub struct Sealed;
}

impl PresenceNonceBinding {
    pub fn new(fields: PresenceNonceBindingFields, digest: NonceDigest) -> Self;
    pub fn try_mark_consumed(&mut self, now: DateTime<Utc>) -> Result<(), ConsumeRace>;
    pub fn try_mark_invalidated(&mut self, now: DateTime<Utc>, reason: InvalidationReason) -> Result<(), InvalidateRace>;
}
```
Persistence variant:
- Claim v175.
- Store keyed digest, not raw token.
- Add SQL consume guard.
- Add sweep and retention rules.
- Drain the in-memory store before enabling persisted rows on restart or migration.
- Refuse startup if both in-memory and persisted nonce stores contain live entries for the same runtime instance.
- Add migration tests.
- Update this packet before coding.
### §5. Token generation
Generation follows artifact 10:
- 32 bytes from OS cryptographic RNG.
- base64url without padding.
- opaque token returned to browser.
- keyed digest stored server-side.
- digest is `HMAC-SHA256(service_local_nonce_key, raw_nonce_bytes)`.
- `service_local_nonce_key` is derived once at startup from W2-B HMAC secret material with a nonce-specific context label.
- the derived key is pinned for the runtime lifetime and never rotated while live nonces exist.
- startup key derivation failure fail-closes endpoint registration.
- no binding record in DOM.
- no binding record in block attributes.
- no binding record in post content.
- no raw token in audit.
- no raw token in logs.

Digest rules:
- Raw nonce bytes are never stored.
- Digest comparison uses constant-time equality.
- Digest prefixes in audit are truncated and non-reversible.
- Key rotation across restart is allowed because in-memory nonces intentionally die on restart.
- The v175 persisted variant cannot rotate the digest key until all persisted unexpired rows are drained or invalidated.
### §6. Issue endpoint
Runtime endpoint:
```http
POST /v1/surface/nonce/issue
```
Request:
```json
{
  "session_id": "surface-session-1",
  "wp_user_id": 42,
  "claim_id": "claim-uuid",
  "field_path": "claims[0].summary",
  "action": "correct",
  "claim_version": 7,
  "composition_id": "composition-uuid",
  "composition_version": 17,
  "request_id": "request-uuid"
}
```
Response:
```json
{
  "ok": true,
  "presence_nonce": "base64url-32-byte-token",
  "expires_at": "2026-05-13T15:01:00Z",
  "ttl_seconds": 60,
  "request_id": "request-uuid"
}
```
Validation order:
1. Bridge authenticates the SurfaceClient actor enough to load the W2-C session binding.
2. Bridge rejects any non-`Actor::SurfaceClient` caller with `403 wrong_actor`.
3. Bridge runs W4-B §17 `validate_session_bound_wp_user_id(actor, payload)`.
4. Body/session mismatch returns `403 wrong_user` before nonce service dispatch.
5. Verify request shape, including mandatory `claim_version: u64`.
6. Verify bearer.
7. Verify HMAC.
8. Verify active session.
9. Verify SurfaceClient scope.
10. Charge W2-D issue budgets.
11. Verify claim exists.
12. Verify claim is substrate-authored.
13. Verify canonical field path.
14. Verify action allowed for field.
15. Query claim store for current `claim_version` via `services::claims::current_claim_version_for_claim(claim_id)`.
16. Verify current composition version.
17. Verify current claim version.
18. Enforce per-session outstanding nonce ceiling.
19. Generate nonce.
20. Store binding.
21. Emit `presence_nonce_issued`.
Clients may propose context.
The runtime creates the binding.

W2-D issue budget keys:
```text
surface_client_id
wp_user_id
claim_id
field_path
action
```
Rules:
- Every issue request consumes the SurfaceClient issue budget after actor/session binding succeeds.
- The budget key is exactly the five axes above; do not add request_id, nonce digest, composition version, or arbitrary WP metadata.
- HMAC failure after actor/session association charges the nonce-failure budget before returning.
- Per-session outstanding nonce ceiling applies before RNG generation.
- A session with too many live nonces receives `429 rate_limited`.
- Acceptance fixture issues 1000 sequential nonces for one session without verification and observes the first configured ceiling breach as `429`.
- The configured ceiling may be lower than 1000, but the fixture must drive 1000 attempts and assert that exhaustion occurs before all succeed.

Store-size cap:
- The service has a global in-memory live-binding cap.
- When the cap is exceeded, evict least-recently-issued unconsumed bindings.
- Eviction marks bindings invalidated with `reason = "store_pressure"`.
- Eviction emits `presence_nonce_invalidated` aggregate audit with safe count and reason.
- Store pressure never silently drops a nonce without changing lifecycle state.
### §7. Verify endpoint
Runtime endpoint:
```http
POST /v1/surface/nonce/verify
```
Request:
```json
{
  "presence_nonce": "base64url-32-byte-token",
  "session_id": "surface-session-1",
  "wp_user_id": 42,
  "claim_id": "claim-uuid",
  "field_path": "claims[0].summary",
  "action": "correct",
  "claim_version": 7,
  "composition_id": "composition-uuid",
  "composition_version": 17,
  "feedback_request_id": "request-uuid"
}
```
Success:
```json
{
  "ok": true,
  "verified": true,
  "consumed_at": "2026-05-13T15:00:21Z",
  "request_id": "request-uuid"
}
```
Reject:
```json
{
  "ok": false,
  "error": "presence_nonce_rejected",
  "reason": "mismatched_action",
  "message": "Refresh this block and try again.",
  "request_id": "request-uuid"
}
```
Validation order:
1. Bridge authenticates the SurfaceClient actor enough to load the W2-C session binding.
2. Bridge rejects any non-`Actor::SurfaceClient` caller with `403 wrong_actor`.
3. Bridge runs W4-B §17 `validate_session_bound_wp_user_id(actor, payload)`.
4. Body/session mismatch returns `403 wrong_user` before nonce service dispatch.
5. Verify request shape, including mandatory `claim_version: u64`.
6. Verify bearer and HMAC.
7. Verify session and scope.
8. Charge W2-D verify/failure budgets.
9. Lookup nonce digest.
10. Compare tuple.
11. Check expiry.
12. Query claim store for current `claim_version`.
13. Check current composition version.
14. Consume atomically in the same serialized decision as steps 10-13.
15. Emit `presence_nonce_verified`.
16. Return success to W5-A with nonce-bound expected versions.
Verify does not apply feedback.

Freshness and consume race rule:
- The version checks and lifecycle transition are one serialized service decision.
- The service must not check current versions, release the mutex, then later consume the nonce.
- If composition advances between a speculative verify check and consume, the real consume decision returns `409 composition_version_stale`.
- If claim version advances between a speculative verify check and consume, the real consume decision returns `409 claim_version_stale`.
- W5-A receives the nonce-bound `claim_version` and `composition_version` on success and carries those expected versions into W4-B CAS when applying feedback.
- This second CAS is required because a version may advance after W4-E verify succeeds but before W5-A applies feedback.

Pinned claim-version query path:
```rust
services::claims::current_claim_version_for_claim_id(ctx, db, claim_id) -> Result<u64, ClaimLookupError>
```
Rules:
- W4-E queries this service during issue and verify.
- Direct SQL from the bridge or handler is forbidden.
- A missing claim returns `403 wrong_claim` only after the nonce tuple can be trusted.
- A current-version mismatch returns `409 claim_version_stale`.
- The response includes no claim body; W5-A owns correction and re-confirm UX.

Nonce failure budgets:
- Tuple mismatch, expiry, replay, invalidation, and unknown nonce charge the W2-D nonce-failure bucket.
- HMAC failure charges the nonce-failure bucket when a SurfaceClient actor/session can be identified safely.
- The budget axes for nonce failures are `surface_client_id`, `wp_user_id`, `claim_id`, `field_path`, and `action` when those fields are available after request-shape validation.
- If request shape is malformed before those fields parse, charge the narrowest authenticated actor/session bucket available.
### §8. Lifecycle
```text
feedback gesture
  -> WP JS requests nonce from WP REST
  -> PHP calls /v1/surface/nonce/issue
  -> runtime stores binding
  -> JS receives opaque token
  -> W5-A sends token for verification
  -> PHP calls /v1/surface/nonce/verify
  -> runtime consumes token
  -> W5-A applies feedback
```
States:
```text
issued
  -> consumed
  -> expired
  -> invalidated_by_composition_refresh
  -> rejected
```
Rules:
- `issued` can become `consumed` once.
- `issued` can become `expired`.
- `issued` can become `invalidated_by_composition_refresh`.
- `issued` can become `invalidated_by_store_pressure`.
- `expired` cannot become `consumed`.
- `invalidated` cannot become `consumed`.
- `consumed` is terminal.
- terminal timestamps are first-writer-wins.
- no lifecycle transition mutates tuple fields.
### §9. Atomic consume
Atomic consume is mandatory.
In-memory default:
- Store bindings behind a service-owned `tokio::sync::Mutex`.
- Primary map is `HashMap<NonceDigest, PresenceNonceBinding>`.
- Secondary invalidation index is `HashMap<CompositionId, HashSet<NonceDigest>>`.
- Check tuple, expiry, invalidation, and consumed state together.
- Check current claim and composition versions inside the same critical section.
- Transition under the same critical section.
- Exactly one verifier wins.
- Later verifiers get `replayed`.
- Do not hold a `std::sync::Mutex` across await points.
- Do not expose a lock guard outside `services::surface_nonce`.
- Do not use a read-only preflight verify API.
- The only success path consumes.

Async lock discipline:
```rust
pub struct SurfaceNonceStore {
    inner: tokio::sync::Mutex<SurfaceNonceStoreInner>,
}

struct SurfaceNonceStoreInner {
    by_digest: HashMap<NonceDigest, PresenceNonceBinding>,
    by_composition: HashMap<CompositionId, HashSet<NonceDigest>>,
    lru: VecDeque<NonceDigest>,
}
```
The store may call claim/composition query services before acquiring the mutex only as advisory preflight. The authoritative decision re-checks freshness under the mutex immediately before lifecycle transition. If a service call must await while the mutex is held, the implementation must restructure around a serialized command queue instead of switching to `std::sync::Mutex`.
Persistence variant:
```sql
UPDATE presence_nonces
SET consumed_at = :now,
    consume_request_id = :request_id
WHERE nonce_digest = :digest
  AND consumed_at IS NULL
  AND invalidated_at IS NULL
  AND expires_at >= :now;
```
Affected rows must equal one.
### §10. Expiry
TTL is 60 seconds maximum.
Rules:
- Default TTL is 60 seconds.
- Shorter TTL is allowed for high-risk actions.
- Longer TTL is forbidden in v1.4.2.
- Verify checks expiry synchronously.
- Sweep is cleanup only.
- Expired token use emits reject audit.
- Passive expiry does not emit one audit row per token in in-memory V1.
### §11. Composition refresh invalidation
W4-B makes the version available.
W4-E owns invalidation.
Default rule:
- Index outstanding nonces by `composition_id`.
- Observe W4-B `composition.updated` from `version_events`.
- When current version becomes `N+1`, invalidate version `<= N`.
- Verify returns `409 composition_version_stale` when actor/session are valid.
- Emit `presence_nonce_invalidated` audit with count.
Correctness boundary:
- Lazy invalidation during verify is mandatory and sufficient for correctness.
- The `version_events` subscriber is an optimization that reduces stale nonce lifetime.
- Missed, delayed, duplicated, or reordered subscriber delivery must not allow a stale nonce to verify.
- Every verify re-reads or otherwise confirms the current composition version before consume.
- If the binding's `composition_version` is lower than current, verify marks the binding invalidated and returns `409 composition_version_stale`.
- The invalidation index accelerates batch invalidation; it is not the only stale-detection mechanism.
Outbox rule:
- W4-B §15 remains the version outbox.
- W4-E consumes W4-B composition events as invalidation triggers.
- W4-E does not create a second outbox.
- W4-E does not silently add a new `version_events.event_kind`.
- A distinct durable `nonce.invalidated` event requires v175 and packet revision.
### §12. Audit emission
Every SurfaceClient nonce event uses:
```rust
emit_surface_audit(
    logger,
    event_kind,
    &actor,
    AuditFields::new("security", detail).with_wp_user_id(wp_user_id),
)
```
Event kinds:
- `presence_nonce_issued`.
- `presence_nonce_verified`.
- `presence_nonce_rejected`.
- `presence_nonce_invalidated`.
Detail fields:
- `request_id`.
- `surface_client_id`.
- `session_id_hash`.
- `nonce_digest_prefix`.
- `claim_id`.
- `field_path`.
- `claim_version`.
- `composition_id`.
- `composition_version`.
- `action`.
- `result`.
- `reason`.
Reject audit rule:
- Every authenticated rejection emits `presence_nonce_rejected`.
- "Authenticated" means the bridge has enough actor/session context to safely attribute to `Actor::SurfaceClient`.
- Unauthenticated transport failures may emit metrics, but do not forge actor audit rows.
- The reject detail shape is class-safe and contains no raw nonce, no proposed correction text, and no claim body.
- Rejection details use bounded enums, not caller-provided strings.

Safe reject detail shape:
```json
{
  "request_id": "<uuid | null>",
  "surface_client_id": "<opaque id>",
  "session_id_hash": "<hash>",
  "wp_user_id": 42,
  "claim_id": "<string | null>",
  "field_path": "<string | null>",
  "claim_version": 7,
  "composition_id": "<string | null>",
  "composition_version": 17,
  "action": "correct",
  "reason": "wrong_user",
  "rejection_class": "actor" 
}
```
Audit prohibitions:
- no raw nonce;
- no corrected value;
- no raw IP unless existing policy permits hashing;
- no raw user-agent unless existing policy permits hashing;
- no customer names;
- no customer domains;
- no emails.
### §13. Rejection taxonomy
| HTTP | Reason | Meaning |
|---:|---|---|
| 400 | `malformed_request` | invalid shape |
| 400 | `missing_nonce` | token absent |
| 400 | `malformed_claim_version` | missing/null/empty/wrong-type claim version |
| 401 | `unauthenticated_surface` | bearer/HMAC missing or invalid |
| 403 | `wrong_actor` | caller is not `Actor::SurfaceClient` |
| 403 | `scope_denied` | missing feedback scope |
| 403 | `wrong_session` | session mismatch |
| 403 | `wrong_user` | WP user mismatch |
| 403 | `wrong_claim` | claim mismatch |
| 403 | `wrong_field` | field mismatch |
| 403 | `mismatched_action` | action mismatch |
| 403 | `expired` | TTL exceeded |
| 403 | `replayed` | already consumed |
| 403 | `invalidated` | invalidated without version lookup |
| 409 | `claim_version_stale` | claim watermark stale |
| 409 | `composition_version_stale` | composition watermark stale |
| 429 | `rate_limited` | budget exhausted |
Precedence:
1. Minimal actor/session binding needed for W4-B §17.
2. `wrong_actor`.
3. W4-B §17 `wrong_user` precondition.
4. Transport auth.
5. Pairing/session/scope.
6. Request shape, including `claim_version`.
7. Rate limit.
8. Nonce lookup.
9. Binding mismatch.
10. Expiry.
11. Version stale.
12. Atomic consume.

Reason-class mapping:
- `actor`: `wrong_actor`, `wrong_user`, `wrong_session`.
- `auth`: `unauthenticated_surface`, HMAC failure.
- `shape`: `malformed_request`, `missing_nonce`, `malformed_claim_version`.
- `binding`: `wrong_claim`, `wrong_field`, `mismatched_action`.
- `lifecycle`: `expired`, `replayed`, `invalidated`.
- `freshness`: `claim_version_stale`, `composition_version_stale`.
- `budget`: `rate_limited`.
### §14. WP-side JS contract
Browser JS never calls Rust directly.
Flow:
```text
Gutenberg affordance click
  -> JS POST /wp-json/dailyos/v1/nonce
  -> PHP checks WP capability and current user
  -> PHP loads pairing/session
  -> PHP calls DailyOS_Runtime_Client::issue_nonce()
  -> PHP returns opaque nonce and expires_at
```
JS request to WP REST:
```json
{
  "block_client_id": "editor-block-id",
  "claim_id": "claim-uuid",
  "field_path": "claims[0].summary",
  "action": "correct",
  "claim_version": 7,
  "composition_id": "composition-uuid",
  "composition_version": 17
}
```
JS rules:
- request nonce on feedback gesture;
- do not request at page load;
- do not request only at save time;
- send `claim_version` as a JSON number, not a string;
- fail closed if the rendered block lacks `claim_version`;
- do not store nonce in post content;
- do not store nonce in block attributes;
- strip nonce attributes before serialization;
- never receive bearer or HMAC key.
### §15. Service boundary
All mutations go through `services/`.
Shape:
```text
src-tauri/src/bridges/surface_client.rs
  -> auth/context
  -> validate_session_bound_wp_user_id()
  -> services::surface_nonce::issue_nonce()
  -> services::surface_nonce::verify_nonce()
```
Rules:
- Handlers do not write DB tables directly.
- Handlers do not mutate nonce maps directly.
- Route handlers live in `bridges/surface_client.rs`, not a new nonce-specific bridge module.
- The bridge precondition runs before nonce service code for both issue and verify.
- Service owns issue, verify, sweep, invalidation, and audit.
- Service accepts frozen clock in tests.
- Service accepts deterministic RNG in tests.
- Service exposes no non-consuming verify.
- Service owns the in-memory mutex and composition index.
- Service owns all W2-D budget charging calls for nonce issue, verify, and nonce-failure buckets.
### §16. Interlocks
W4-B must land first.
W4-E needs:
- canonical `field_path`;
- current `claim_version`;
- current `composition_version`;
- `composition_id`;
- W4-B stale composition semantics;
- W4-B `version_events`.
- W4-B §17 session-bound `wp_user_id` bridge precondition.
- W4-B §37 canonical SurfaceClient route module.
W2-D supplies:
- SurfaceClient rate-limit budget primitives.
- budget buckets for issue, verify, and nonce failure.
- HMAC failure accounting once actor/session context is known.
W5-A consumes W4-E.
W5-A needs:
- opaque nonce token;
- verify success/failure;
- stable reason codes;
- 409 stale-version responses;
- request-id audit correlation.
- nonce-bound expected `claim_version` and `composition_version` on verify success.
W5-A owns:
- feedback payload;
- action semantic mapping;
- calling `record_claim_feedback`;
- carrying nonce-bound expected versions into W4-B CAS;
- block re-render;
- version-changed re-confirm UX.
### §17. Intelligence Loop fit
Claim model:
- Nonce is not a claim.
- Feedback gated by nonce can mutate claim lifecycle and trust inputs.
- W4-E must not create display-only intelligence data.
Provenance and trust:
- Audit binds actor, WP user, claim, field, action, claim version, composition version.
- W5-A records provenance reference during feedback application.
- W4-E does not adjust trust scores.
- Stale-version rejects preserve trust by forcing feedback to re-anchor on current claim state.
Signals and invalidation:
- W4-E invalidation is driven by W4-B composition events.
- W4-E lazy-invalidates on verify when event delivery lags.
- W5-A feedback emits feedback signals through existing services.
- W4-E emits audit, not trust signals.
Runtime and surfaces:
- Tauri runtime owns issue and verify.
- WordPress mediates browser calls through PHP.
- MCP is not in scope unless a future SurfaceClient feedback path uses it.
Feedback loop:
- corrections, dismissals, corroborations, contradictions reach feedback services only after verify;
- rejects do not create feedback facts;
- repeated rejects can feed abuse telemetry, rate limits, or pairing health without changing claim state.
## Acceptance criteria lifted into DOS-571

### Implementation criteria
1. `POST /v1/surface/nonce/issue` and `POST /v1/surface/nonce/verify` are registered in `src-tauri/src/bridges/surface_client.rs` per W4-B V8 §37.
2. Both endpoints run through W4-B V8 §17 `validate_session_bound_wp_user_id` before any W4-E nonce service code.
3. Body/session `wp_user_id` mismatch returns `403 wrong_user` before nonce lookup, claim lookup, rate-limited issue, RNG generation, or consume.
4. Only `Actor::SurfaceClient` can call `/v1/surface/nonce/*`; all other actors return `403 wrong_actor`.
5. `claim_version` is a required `u64` in issue and verify requests and in `PresenceNonceBinding`.
6. `PresenceNonceBinding` tuple fields are immutable after insertion.
7. Lifecycle mutation is limited to first-writer-wins `consumed_at` or `invalidated_at` transitions.
8. Nonce digest uses HMAC-SHA256 with a service-local key derived at startup from W2-B HMAC secret material.
9. The nonce digest key is pinned for the runtime lifetime and is not rotated while live nonces exist.
10. In-memory store uses `tokio::sync::Mutex<HashMap<NonceDigest, PresenceNonceBinding>>`.
11. Invalidation index uses `HashMap<CompositionId, HashSet<NonceDigest>>`.
12. No `std::sync::Mutex` is held across await points.
13. Issue budgets use W2-D axes `surface_client_id`, `wp_user_id`, `claim_id`, `field_path`, and `action`.
14. Verify/failure budgets use the same axes when parseable and the narrowest authenticated actor/session bucket otherwise.
15. Per-session outstanding nonce ceiling is enforced before RNG generation.
16. Global store-size cap evicts by LRU and emits `presence_nonce_invalidated reason=store_pressure`.
17. Verify re-checks current claim version with `services::claims::current_claim_version_for_claim_id(ctx, db, claim_id)`.
18. Verify re-checks current composition version before consume.
19. Freshness checks and consume are one serialized service decision.
20. W5-A receives nonce-bound expected versions on verify success and passes them into W4-B CAS during feedback write.
21. Lazy invalidation during verify is the correctness boundary; the `version_events` subscriber is an optimization.
22. Every authenticated rejection emits `presence_nonce_rejected` with the safe reject detail shape from §12.
23. v175 persistence variant drains or invalidates the in-memory store before accepting persisted live nonces after restart.

### Negative fixtures
24. Expired nonce rejects with `403 expired`.
25. Replayed nonce rejects with `403 replayed`.
26. Cross-user nonce rejects with `403 wrong_user`.
27. Cross-session nonce rejects with `403 wrong_session`.
28. Wrong action rejects with `403 mismatched_action`.
29. Wrong field rejects with `403 wrong_field`.
30. Wrong claim rejects with `403 wrong_claim`.
31. Missing nonce rejects before lookup.
32. Missing composition version rejects before issue.
33. Stale claim version returns `409 claim_version_stale`.
34. Stale composition version returns `409 composition_version_stale`.
35. Composition refresh invalidates prior outstanding nonce.
36. Atomic race yields one success and one replay.
37. Unknown nonce rejects without feedback write.
38. Missing `wp_user_id` audit emission fails.
39. WP block serialization strips nonce.
40. Direct browser-to-runtime attempt is rejected.
41. `dos571_fixture_bridge_precondition_order.rs` proves `validate_session_bound_wp_user_id` runs before nonce code by asserting no RNG call, no claim query, no rate-limit issue charge, and no nonce lookup on body mismatch.
42. `dos571_fixture_wrong_actor.rs` calls nonce issue/verify as any actor other than `Actor::SurfaceClient` and receives `403 wrong_actor`.
43. `dos571_fixture_binding_immutable.rs` attempts to mutate tuple fields after insertion and fails at compile time or through a dedicated mutation-gate test.
44. `dos571_fixture_lifecycle_cas.rs` races consume vs invalidation and observes exactly one terminal timestamp.
45. `dos571_fixture_claim_version_missing.rs` omits `claim_version` and receives `400 malformed_claim_version`.
46. `dos571_fixture_claim_version_empty.rs` sends empty/null `claim_version` and receives `400 malformed_claim_version`.
47. `dos571_fixture_claim_version_wrong_type.rs` sends string/object/float/negative `claim_version` and receives `400 malformed_claim_version`.
48. `dos571_fixture_issue_rate_exhaustion.rs` performs 1000 sequential issue attempts for one session and observes `429 rate_limited` before all succeed.
49. `dos571_fixture_store_pressure_lru.rs` exceeds global store cap and asserts oldest live nonce is invalidated with `reason=store_pressure`.
50. `dos571_fixture_hmac_failure_budget.rs` sends signed-context HMAC failures and asserts nonce-failure budget charging without unsafe audit attribution.
51. `dos571_fixture_claim_version_drift.rs` issues at claim version 7, advances the claim to 8, then verify returns `409 claim_version_stale`.
52. `dos571_fixture_freshness_consume_race.rs` advances composition between an advisory verify check and consume; real consume returns `409 composition_version_stale`.
53. `dos571_fixture_lazy_invalidation.rs` drops or delays the `version_events` subscriber and still rejects stale composition during verify.
54. `dos571_fixture_reject_audit_shape.rs` covers every authenticated rejection class and asserts `presence_nonce_rejected` detail contains only safe bounded fields.
55. `dos571_fixture_persistence_restart_drain.rs` covers v175 mode: restart drains in-memory live entries before persisted nonce replay is accepted.
## Test plan
### Code path diagram
```text
ISSUE
WP JS -> WP REST -> PHP client -> runtime issue
  -> SurfaceClientBridge
  -> W4-B §17 wp_user_id precondition
  -> auth/session/scope
  -> rate limits
  -> claim/field/action validation
  -> W4-B version check
  -> RNG + digest
  -> store binding
  -> audit issued
VERIFY
W5-A/PHP -> runtime verify
  -> SurfaceClientBridge
  -> W4-B §17 wp_user_id precondition
  -> auth/session/scope
  -> rate limits
  -> digest lookup
  -> tuple compare
  -> expiry/current-version check inside serialized decision
  -> atomic consume with nonce-bound expected versions
  -> audit verified
  -> W5-A applies feedback
INVALIDATE
W4-B commit_composition
  -> version_events composition.updated
  -> W4-E subscriber invalidates lower-version nonces when delivered
  -> W4-E verify lazily invalidates lower-version nonces when subscriber lags
  -> audit invalidated
```
### Integration tests
- Valid HTTP issue returns nonce and expiry.
- Valid HTTP verify consumes nonce.
- Concurrent duplicate verify returns one success.
- Verify success returns nonce-bound expected versions for W5-A CAS.
- WP REST handler calls PHP runtime client.
- Serialized Gutenberg block contains no nonce.
- W5-A mock refuses to write when verify fails.
- W5-A mock passes nonce-bound expected versions into W4-B CAS after verify.
- SurfaceClient route registration is in `bridges/surface_client.rs`.
### Security tests
- Cross-user replay rejects.
- Cross-session replay rejects.
- Cross-actor issue and verify reject.
- Changed field rejects.
- Changed action rejects.
- Stale tab after composition refresh returns `409`.
- Stale claim after issue returns `409`.
- Composition update racing consume returns `409`.
- Rate-limit exhaustion returns `429`.
- HMAC failures charge nonce-failure budget.
- Every authenticated rejection emits safe reject audit.
- Store pressure invalidates by LRU and emits aggregate invalidation audit.
### Persistence tests if v175 is chosen
- v175 migration applies idempotently.
- raw nonce absent from persisted rows.
- SQL consume updates exactly one row.
- restart drains in-memory store before persisted live rows can be replayed.
- digest-key rotation fails closed unless live persisted rows are drained.
- sweep preserves required audit evidence.
## Migration, rollout, and observability
Default rollout:
- no migration;
- in-memory store;
- durable audit log;
- endpoint registration after W4-B.
Persistence rollout:
- claim v175;
- add migration file;
- register migration;
- on startup, drain or invalidate the in-memory store before opening persisted nonce verification;
- reject mixed in-memory/persisted live stores for the same runtime instance;
- add migration tests;
- add retention and sweep policy;
- update this packet before code.
Metrics:
- issue count;
- verify success count;
- reject count by reason;
- invalidation count;
- replay count;
- wrong_actor count;
- wrong_user precondition count;
- malformed_claim_version count;
- store_pressure invalidation count;
- outstanding nonce gauge by session;
- average nonce age at verify;
- p95 issue-to-verify latency;
- missing `wp_user_id` audit failures.
Logs:
- WARN for current-version lookup failure.
- WARN for audit emission failure.
- WARN for persistence restart drain failure.
- INFO for invalidation batch count.
- INFO for store-pressure eviction batch count.
- DEBUG for normal issue/verify without raw token.
## NOT in scope
- Full W5-A feedback router.
- User-facing re-confirm UX.
- Persistent nonce table by default.
- Bloom filter replay cache by default.
- New trust scoring logic.
- New feedback actions.
- New signal dispatcher.
- Browser-direct runtime calls.
- Saving nonce to post content.
- Multi-process runtime redesign.
## Open questions
| ID | Question | Recommendation |
|---|---|---|
| Q1 | Add distinct `nonce.invalidated` outbox rows? | No for V1; use W4-B `composition.updated`. |
| Q2 | Consume mismatch attempts defensively? | No; consume only on full valid match. |
| Q3 | Is `claim_version` mandatory? | Yes when W4-B exposes it. |
| Q4 | How does `dismiss` map to feedback? | W5-A owns by UI context. |
| Q5 | Shorter TTL for `contradict`? | Optional later; max stays 60s. |
| Q6 | Which WP capability gates nonce request? | W5-A/WP plugin decides; DailyOS scope still required. |
| Q7 | Passive expiry audit rows? | No for in-memory V1; aggregate metrics only. |
| Q8 | Who owns `/v1/surface/nonce/*` route registration? | Closed by W4-B V8 §37: `src-tauri/src/bridges/surface_client.rs`. |
| Q9 | Does W4-E locally validate `wp_user_id`? | No; it inherits W4-B V8 §17 bridge precondition, then compares tuple values only after truthful actor binding. |
| Q10 | Can verify check freshness before consuming? | Only as advisory preflight; authoritative freshness and consume are one serialized decision. |
