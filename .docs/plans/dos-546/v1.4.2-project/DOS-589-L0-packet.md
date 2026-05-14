# W4-B-signals L0 packet - signal subscriber + scope-filter dispatcher

Date: 2026-05-13 (V1)
Project: v1.4.2 - Personal Intelligence Engine: WordPress Foundation
Parent: DOS-546
Wave: W4-B-signals (stage-1 sibling under DOS-546 v1.4.2)
Issue: DOS-589 (signal subscriber + scope-filter dispatch)
Sibling: DOS-567 (W4-B: three-view consistency concurrency contract)
Status: L0 prep

This packet captures the substrate-side dispatcher plan for DOS-589. The Linear issue remains the canonical execution record; this packet hardens the implementation contract where the issue leaves decisions open or where W4-B V9 supersedes older wording.

DOS-589 is pure Rust substrate work. It is not WP/PHP work. It consumes `version_events` rows written by DOS-567's W4-B outbox after commit and delivers ordered, scope-filtered events to subscribers that are already authorized to see the underlying claim or composition.

W4-B V9 is the upstream contract inherited here:

- W4-B V9 section 5 defines `ClaimVersionEvent`, `CompositionVersionEvent`, and `CorrectionRef`.
- W4-B V9 section 15 defines the dedicated `version_events` table, `event_seq INTEGER PRIMARY KEY AUTOINCREMENT`, UUIDv4 cursor, and replay clause `WHERE event_seq > ? ORDER BY event_seq`.
- W4-B V9 section 16 defines the class-level scope-filter rule.
- W4-B V9 section 17 defines `wp_user_id` session binding for SurfaceClient requests.
- W4-B V9 section 37 pins `src-tauri/src/bridges/surface_client.rs` as the canonical `/v1/surface/*` route module.

One drift is intentional in this packet: Linear DOS-589 still says `signal_events` in its scope and acceptance text. W4-B V9 supersedes that wording for this issue. DOS-589 dispatches from `version_events`, not the legacy ADR-0080 `signal_events` table.

## Changelog

- **V3.2 (2026-05-13):** Cycle 4 codex confirmation pass — closed one literal acceptance violation:
  - §16 (SurfaceClient session expiry handling, V2 NEW) renumbered to §13.5 to preserve §13 → §14 → §15 ordering required by L0 closure acceptance criterion ("All 15 sections remain present in order"). V2 inserted §16 between §13 and §14 which created `13 → 16 → 14 → 15`. V3.2 keeps the content unchanged; only the section index changes.
  - V2 changelog reference to "§16" updated to "§13.5".
  - All trust-boundary and substrate-contract findings already closed in V3.1 — section ordering was the residual literal-acceptance gap.

- **V3.1 (2026-05-13):** Cycle 3 codex confirmation pass — closed one stale-contract drift after V3 §3 cursor envelope landed:
  - §10 reconnect flow rewritten as envelope-first: dispatcher decodes envelope → reads embedded `subscription_id` → looks up `subscriber_local_key` + checkpoint by `subscription_id` (durable key, not cursor) → recomputes HMAC → on mismatch returns "no events" indistinguishably without resolving cursor_uuid. V2 said "lookup subscription_id via cursor → event_seq → checkpoint" which reintroduced the cursor-as-lookup-authority path that V3 §3 explicitly removed.
  - Added explicit rule: if envelope is unverifiable, `from_cursor` is ignored entirely; no implicit subscription allocation tied to a foreign cursor.

- **V3 (2026-05-13):** Cycle 2 fold — codex BLOCK. Material changes:
  - **CRITICAL fix (codex C2):** §3 cursor envelope shape. V2 conflated "HMAC-as-cursor" with "UUIDv4-as-row-PK" — codex correctly flagged this contradicts W4-B V9 §15 (which fixes the `version_events.cursor` PK as UUIDv4). V3: the W4-B UUIDv4 PK is preserved verbatim; on-wire cursor is a base64 envelope `{cursor_uuid: UUIDv4, subscription_id: UUIDv4, mac: HMAC-SHA256(subscriber_local_key, cursor_uuid || subscription_id)}`. Envelope is the transport-layer authenticity check; row identity remains the UUIDv4. Resolution: decode → lookup subscriber_local_key by subscription_id → recompute MAC → on match, resolve cursor_uuid to event_seq via W4-B V9 §15 PK lookup.
  - **HIGH fix (codex C2):** §4 + Q4 — `SubscribeAck.current_cursor` rewritten as subscriber-visible cursor (most recent event whose `scope_permits_*_read` predicate returns true under current scopes, or `null`). V2 said "global latest is acceptable since UUIDv4 is opaque"; codex correctly noted that returning ANY reachable cursor in a global-latest scheme is still an existence-oracle signal. V3: internal `event_seq_at_subscribe` high-water mark is dispatcher-private and never surfaces; subscribers see only permitted-event cursors.
  - **MEDIUM fix (codex C2):** §5 `scope_permits_composition_read` predicate now takes `composition_version` and authorizes against the FROZEN claim_refs at that exact version, not the most recent reflection. Prevents a race where a subsequent version drops the actor's only permitted claim_ref but the dispatcher still delivers the earlier event's notification using current reflection.

- **V2 (2026-05-13):** Cycle 1 fold — eng + devex CONDITIONAL APPROVE; /cso CONDITIONAL (2 CRITICAL + 3 HIGH + 3 MEDIUM + 2 LOW); codex BLOCK (1 CRITICAL + 3 HIGH + 2 MEDIUM + 1 LOW). Material changes:
  - **CRITICAL fix (codex):** §3 replay cursor semantics rewritten. V1 conflated "internal scan position" with "client-visible cursor" — out-of-scope rows trapped subscribers in infinite loop OR leaked via cursor advancement. V2 distinguishes: internal `last_scanned_event_seq` (durable, ratchets through ALL event_seqs regardless of scope; prevents replay loops) vs client-visible `cursor` (UUIDv4, references only PERMITTED events the subscriber actually received; prevents existence-oracle leak).
  - **HIGH fix (codex):** §5 composition event scope-gating no longer relies on `affected_claim_ids` (which W4-B V9 §5 does not provide). V2 adds `scope_permits_composition_read(actor, composition_id)` predicate — DOS-589-owned, queries `composition_versions` + composition's known claim_refs. No W4-B amendment needed.
  - **HIGH fix (codex):** Replay/live race ordering. On Subscribe with `from_cursor`, dispatcher captures high-water `event_seq_at_subscribe`. Replay runs through that high-water. Live events with `event_seq > high_water` BUFFER on the subscriber's outbound queue until replay completes. After replay close, buffered events drain in order.
  - **HIGH fix (codex):** Durable checkpoint identity §10/§11 now keyed by `(subscription_id, actor, scopes_digest, subject_filter_digest)`. `subscription_id` is server-allocated per Subscribe call. Concurrent same-actor connections don't poison each other.
  - **CRITICAL fix (cso C1):** §4 — scope set re-resolved from CURRENT authenticated actor for each row, not snapshotted at subscribe or replay-start. Per-row scope rebind catches mid-replay scope tightening.
  - **CRITICAL fix (cso C2):** Cursor HMAC binding. Client-visible cursor is `HMAC(per-subscriber-key, event_seq)`. Foreign cursor → bridge resolves cursor to event_seq; HMAC mismatch returns identical "no events" envelope (not differentiated from "valid but no events"). Cross-actor cursor capture cannot probe event_seq lifetime.
  - **HIGH fix (cso H1):** §7 backpressure event delivered ONLY to affected subscriber. `dropped_event_count` counts events in subscriber's permitted set; never includes filtered-away rows. No cross-subscriber backpressure metric on any SurfaceClient route.
  - **HIGH fix (cso H2):** §8 transport — push channel inherits per-frame HMAC matching W2-B contract; transport choice deferred but integrity property locked.
  - **HIGH fix (cso H3):** §8 wp_user_id binding for SubscribeRequest — body-asserted rejected if not matching session-bound; dispatcher never reads from body without bridge validation.
  - **MEDIUM (cso M1):** §4 predicate evaluation + payload projection inside consistent claim-read-policy snapshot (per-claim-version-bound read).
  - **Eng P1-A:** §9 + §11 — dispatch reader takes SELECT snapshot ordered by event_seq; advances checkpoint to `min(committed_event_seq_observed)`. Never advances past lower-seq row not yet observed.
  - **Eng P1-B:** §4 + Q4 — `current_cursor` = global latest event_seq cursor; scope evaluation deferred to first replay/push (acceptable since cursor opaque UUIDv4).
  - **Eng P2-B:** §8 heartbeat semantics — dispatcher-emits on `heartbeat_ms` interval; subscriber-miss detection in transport adapter; heartbeat does NOT advance cursor.
  - **Eng P3-A:** backpressure thresholds pinned `soft=256, hard=1024` per-subscriber-queue events.
  - **Eng P3-B:** migration slot reservation v176-v179 (after W4-B v170, W4-C v171-v174, W4-E v175).
  - **Eng P3-C:** CI invariant — dispatcher routes/methods NEVER live in `signals/bus.rs`.
  - **Devex P1-1 dashmap:** §13 + What-W4-B-Signals-Authors net-new — `DashMap<SubscriptionId, SubscriberHandle>` (lock-free shard) + `DashMap<SubscriberDigest, SubscriptionId>` (reconnect) + bounded `tokio::sync::mpsc::channel`. NOT `tokio::Mutex<HashMap>`.
  - **Devex P2-1 module placement:** §13 — `src-tauri/src/services/version_dispatcher.rs` + `services/version_events.rs`. Reserves `signals/` for ADR-0080.
  - **Devex P2-2 Tauri entrypoint:** `commands/version_dispatcher.rs`; `tauri::ipc::Channel<DispatchedEvent>` per subscription (NOT `Window::emit`). Channel close = auto-unsubscribe.
  - **Devex P2-3 session expiry:** §13.5 NEW (originally numbered §16 in V2; renumbered V3.2 to preserve ordering) — SurfaceClient session expiry triggers immediate unsubscribe, drain queue without dispatching, require new SubscribeRequest with fresh wp_user_id binding.
  - **Devex P2-4 backpressure discipline:** §7 — `tokio::sync::mpsc::channel` bounded; reserved slots for `claim.write_rejected`/`claim.corrected`.
  - **Devex P2-5 replay cap:** server caps LIMIT to `MAX_REPLAY_BATCH = 500`; `ReplayResponse.has_more = true` when capped.

- **V1 (2026-05-13):** Initial L0 packet for DOS-589. Mirrors W4-B V9 section shape, inherits W4-B V9 sections 5/15/16/17/37, resolves dispatcher decisions, maps each Linear acceptance criterion to directional decisions and negative fixtures, and names the CI invariants needed to prevent scope-filter bypass.

## Status snapshot

- DOS-589 is the delivery half of W4-B-signals.
- DOS-567 owns version assignment, outbox row schema, cursor allocation, and atomic insert into `version_events`.
- DOS-589 owns subscriber state, dispatch loop, scope-filtered delivery, replay, backpressure, and dispatcher-facing routes.
- Current `src-tauri/src/signals/bus.rs` is an ADR-0080 SQLite event-log emitter for `signal_events`.
- Current `signals/bus.rs` is not a pub/sub dispatcher.
- It has no subscriber registry.
- It has no dispatch loop.
- It has no `scope_permits_claim_read(subscriber, claim_id)` predicate.
- It has no per-subscriber outbound queues.
- It has no replay cursor resolution over `version_events`.
- It has no `subscription.backpressure` event.
- Current `src-tauri/src/bridges/surface_client.rs` exists and already owns SurfaceClient authorization/rate-limit substrate.
- Current `src-tauri/src/surface_runtime/mod.rs` supports signed `/v1/surface/*` route dispatch.
- Current signed route allowlist includes `/v1/surface/invoke`, `/v1/surface/feedback`, `/v1/surface/abilities`, and `/v1/surface/keyring`.
- DOS-589 adds dispatcher routes to that signed SurfaceClient route surface.
- W4-B V9 section 37 makes `bridges/surface_client.rs` the canonical route module for those routes.
- DOS-589 gates W4-C dispatcher consumers.
- W4-C expects invalidation when signed/tamper cache entries are stale after a version event.
- DOS-589 gates W4-A renderer cache-bust.
- W4-A expects mounted block render state to refresh without polling when claim/composition versions move.
- DOS-589 gates W5-A retry-after-event.
- W5-A expects 423/409 retry loops to wait on cursor events rather than tight polling.
- DOS-589 is blocked by DOS-567 because the dispatcher needs W4-B V9 section 15 `version_events`.
- DOS-589 blocks DOS-569, DOS-572, and DOS-573 because those consumers need push delivery.
- No design review is required for L0 because this packet is substrate-only and has no user-facing UI.

## Pre-work confirmed (substrate reuse audit)

**Headline finding:** DOS-589 is net-new pub/sub infrastructure over an inherited event log. It should reuse the existing SurfaceClient bridge, actor/scopes types, and W4-B `version_events` contract, but it should not extend the legacy ADR-0080 `signal_events` emitter into a correctness dispatcher.

### Existing `src-tauri/src/signals/bus.rs`

- The module header describes "Signal event CRUD and source tier weights (ADR-0080)."
- `SignalEvent` is documented as a row from the `signal_events` table.
- `emit_signal_event(...)` returns `SignalEmitOutcome { id, coalesced }`.
- The bus has coalescing state and rate-limited claim attempt heuristics.
- The bus writes generic `signal_events` rows.
- The bus is useful for legacy intelligence signals and propagation.
- The bus is not an ordered subscriber dispatcher.
- There is no `Subscriber`, `Subscription`, `SubscribeRequest`, or `SubscribeAck` type.
- There is no registry keyed by `(actor, scopes, subject_filter)`.
- There is no outbound queue per subscriber.
- There is no replay loop using `event_seq`.
- There is no post-commit watcher over `version_events`.
- There is no scope predicate for claim-read authorization.
- Conclusion: DOS-589 should create a dedicated dispatcher module rather than overloading `signals/bus.rs` with two different meanings of "signal."

### Existing `src-tauri/src/migrations/018_signal_bus.sql`

- `signal_events` is a generic ADR-0080 table with `id`, `entity_type`, `entity_id`, `signal_type`, `source`, `value`, `confidence`, `decay_half_life_days`, `created_at`, and `superseded_by`.
- The schema has no `event_seq`.
- The schema has no UUIDv4 cursor contract.
- The schema has no claim/composition XOR constraint.
- The schema has no W4-B event-kind enum.
- The schema has no correction event-log pointer.
- Therefore it cannot satisfy W4-B V9 section 15 replay and payload contracts without weakening them.

### Inherited `version_events` table from W4-B V9 section 15

- `event_seq INTEGER PRIMARY KEY AUTOINCREMENT` is the replay-order source of truth.
- `cursor TEXT NOT NULL UNIQUE` is a UUIDv4 opaque cursor.
- `event_kind` is constrained to W4-B event kinds.
- Exactly one of `claim_id` or `composition_id` is present.
- `previous_version` and `current_version` carry watermark movement.
- `correction_event_log_id` points to the durable body for `CorrectionRef`.
- `mutation_id` ties rows back to `mutation_attempts` when relevant.
- `created_at` and `actor_kind` support audit and consumer diagnostics.
- Replay clause is `SELECT * FROM version_events WHERE event_seq > ? ORDER BY event_seq LIMIT ?`.
- Cursor resolution is `SELECT event_seq FROM version_events WHERE cursor = ?`.
- DOS-589 consumes these rows after W4-B commits.
- DOS-589 does not define or migrate this table.

### Existing actor/scope substrate

- `abilities-runtime/src/abilities/registry.rs` defines `Actor::SurfaceClient { instance, scopes }`.
- `ScopeSet` is typed, sorted, and non-empty for SurfaceClient actors.
- `SurfaceScope` is string-backed and allowlist-aware.
- `AbilityPolicy.required_scopes` exists for ability invocation gating.
- These types provide the subscriber identity and scope material for dispatcher authorization.
- DOS-589 still needs a claim-specific read predicate because ability-level required scopes are not enough to prove a subscriber may read a particular `claim_id`.

### Existing `src-tauri/src/bridges/surface_client.rs`

- The file exists in the target worktree.
- It already contains `SurfaceClientBridge`, rate-limit budgets, request class limits, and authorization plumbing.
- W4-B V9 section 37 promotes this module as the canonical owner for `/v1/surface/*` routes.
- DOS-589 dispatcher routes land here, not in a new route-owner module.
- Surface runtime signed-route dispatch must add DOS-589 routes to the supported route allowlist.

### Phase 0 artifact 02 subscription contract

- `.docs/plans/dos-546/phase-0/02-concurrency-contract.md` lines 595-616 define `SubscribeRequest` and `SubscribeAck`.
- Lines 624-650 define backpressure semantics and the `subscription.backpressure` event.
- Lines 658-683 define replay request/response semantics.
- DOS-589 should implement the Phase 0 shape with W4-B V9 `version_events` as the backing log.
- Any transport-specific choice must preserve those transport-independent semantics.

## What DOS-589 authors net-new

| Surface | Status today | DOS-589 authoring scope |
|---|---|---|
| `signals/version_dispatcher.rs` or equivalent substrate module | Missing | Dispatcher service over `version_events` |
| `SubscribeRequest` / `SubscribeAck` Rust wire types | Missing | Exact Phase 0 artifact 02 lines 595-616 shape |
| `ReplayRequest` / `ReplayResponse` Rust wire types | Missing | Phase 0 artifact 02 lines 668-683 plus W4-B V9 cursor-to-event_seq resolution |
| `BackpressureEvent` Rust wire type | Missing | Phase 0 artifact 02 lines 640-649 |
| Subscriber registry keyed by `(actor, scopes, subject_filter)` | Missing | In-memory live handles plus persisted checkpoint rows |
| Persistent reconnect checkpoint | Missing | Durable last-seen cursor/event_seq by subscriber identity and subject filter |
| Dispatch loop | Missing | Post-commit reader over `version_events` ordered by `event_seq` |
| `scope_permits_claim_read(subscriber, claim_id)` | Missing | Mandatory delivery predicate for every claim-bound event |
| Composition-to-claim expansion | Missing | Dispatcher maps composition events to affected claims where payload supplies them, then applies claim predicate |
| Subject filter matcher | Missing | Supports claim, composition, and subject filters from Phase 0 |
| Per-subscriber outbound queue | Missing | Bounded queue with backpressure emission and replay-required state |
| Scope-filtered replay endpoint | Missing | Streams only permitted rows in event_seq order |
| `GET /v1/surface/event-log/{event_log_id}` route | Missing as DOS-589 route work | Scope-filtered `CorrectionRef` event-log body fetch through `bridges/surface_client.rs` |
| Tauri command subscription channel | Missing | User/Agent actor transport for native app consumers |
| HTTP loopback subscription route | Missing | SurfaceClient transport through signed `/v1/surface/*` route surface |
| CI bypass gate | Missing | Tests/lint proving every delivery path calls the scope predicate |

**Module placement:** the implementation should keep durable/domain logic in `services/` per repository rule. Route handlers and Tauri commands call service methods; they do not write subscriber state or mutate dispatcher rows directly.

**Suggested module split:**

- `src-tauri/src/services/version_dispatcher.rs` - registry, dispatch loop, replay, predicates, persistence.
- `src-tauri/src/signals/version_events.rs` - typed decoding helpers for W4-B V9 sections 5/15 rows, if not already created by DOS-567.
- `src-tauri/src/bridges/surface_client.rs` - HTTP loopback route handlers and signed SurfaceClient request/response projection.
- `src-tauri/src/commands.rs` or existing Tauri command owner - native app subscription entrypoint for User/Agent actors.
- `src-tauri/tests/dos589_fixture_*.rs` - negative fixtures and replay/backpressure tests.

## Directional decisions resolved at L0

### section 1. Pub/sub model: push first, replay as the correctness backstop

DOS-589 implements push delivery, not client polling. This follows Phase 0 artifact 02 lines 587-593.

The dispatcher continuously advances through committed `version_events` rows and pushes rows to active subscribers with bounded queues.

Replay remains mandatory because push transport is lossy across process pause, network loss, and backpressure.

Correctness is in the event log plus replay, not in the live socket.

A subscriber that misses live delivery reconnects with its last-seen cursor and receives ordered, deduplicated replay.

### section 2. Subscriber persistence: live handles in memory, durable subscription checkpoint in SQLite

The registry has an in-memory live handle keyed by `subscription_id` for connected transports.

The registry has a durable checkpoint keyed by `(actor, scopes_digest, subject_filter_digest)` for reconnect continuity.

The durable row stores `subscription_id`, actor kind/instance, scopes digest, canonical subject filter, timestamps, last acked cursor, last acked `event_seq`, and backpressure state.

The durable row does not store raw customer names, domains, or subject labels.

Rationale: replay reconstructs missed rows from `version_events`, while a purely in-memory registry fails reconnect persistence.

### section 3. Replay strategy: `event_seq` is order; cursor is address (V2 REWRITE per codex CRITICAL)

V1 conflated internal scan position with the client-visible cursor. V2 distinguishes them:

- **`last_scanned_event_seq` (internal, durable, per-subscriber):** ratchets forward through ALL event_seqs scanned, regardless of scope. Stored in `subscription_checkpoints` table. Prevents infinite replay loops on out-of-scope rows.
- **`cursor_uuid` (W4-B V9 §15 PRIMARY KEY, UUIDv4):** the row identifier in `version_events`. Stays as the canonical event address.
- **`cursor_envelope` (client-visible, V3 per codex C2 CRITICAL):** the actual on-wire cursor is a structured envelope `{cursor_uuid: UUIDv4, subscription_id: UUIDv4, mac: bytes32}` — the UUIDv4 IS the row PK from W4-B V9 §15 (preserved verbatim); the HMAC binds the envelope to the subscription. V2 conflated "HMAC-as-cursor" with "UUIDv4-as-row-key"; V3 separates them: row identity uses UUIDv4 (W4-B contract preserved), envelope authenticity uses HMAC over the envelope contents.

**Cursor envelope construction (V3 per codex C2 CRITICAL):** every client-visible cursor is serialized as a base64-encoded envelope: `cursor_envelope = base64({ "cursor_uuid": <UUIDv4>, "subscription_id": <UUIDv4>, "mac": HMAC-SHA256(subscriber_local_key, cursor_uuid || subscription_id) })`. The `subscriber_local_key` is generated per-subscription at Subscribe time and stored alongside the durable checkpoint. On reconnect with `from_cursor`, dispatcher:
1. Decodes the envelope.
2. Looks up `subscriber_local_key` by `subscription_id`.
3. Recomputes the expected HMAC over `cursor_uuid || subscription_id`.
4. On HMAC mismatch OR unknown subscription_id, returns "no events" indistinguishably from "valid cursor but no permitted events in the requested range."
5. On HMAC match, resolves the UUIDv4 to event_seq via `SELECT event_seq FROM version_events WHERE cursor = ?` — the W4-B V9 §15 row lookup, unchanged.

Foreign cursor capture cannot probe event_seq lifetime; cursor authenticity does NOT alter the W4-B V9 cursor schema (UUIDv4 PK preserved); the HMAC is a transport-layer envelope check, not a substrate schema change.

**Replay query semantics:**
- Replay reads via `SELECT * FROM version_events WHERE event_seq > ? ORDER BY event_seq LIMIT ?`.
- `WHERE event_seq > last_scanned_event_seq` ensures scan ratchets through ALL rows; out-of-scope rows are filtered but `last_scanned` still advances.
- `MAX_REPLAY_BATCH = 500` cap; `ReplayResponse.has_more = true` when capped; client iterates with returned `next_cursor`.
- `ReplayResponse.next_cursor` is the last DELIVERED (permitted) event's cursor — NOT the last scanned event_seq.

**Cursor resolution:**
- `SELECT event_seq FROM version_events WHERE cursor = ?` resolves the envelope's `cursor_uuid` to its event_seq (the W4-B V9 §15 PK column).
- HMAC over `(cursor_uuid || subscription_id)` validates the envelope belongs to the requesting subscriber.
- Unknown cursor (no row found) → return Phase 0 `replay-expired` envelope.
- Unknown cursor is NEVER treated as "from beginning."
- Deduplication is by event_seq: the row at the supplied cursor is not replayed.

**Replay/live race barrier (V2 NEW per codex HIGH):**
- On Subscribe (or replay request) with `from_cursor`, dispatcher captures `event_seq_at_subscribe = SELECT MAX(event_seq) FROM version_events` as a high-water mark.
- Replay runs through that high-water mark.
- Live events with `event_seq > event_seq_at_subscribe` are BUFFERED on the subscriber's outbound queue but NOT delivered until replay completes (signaled by `ReplayResponse.has_more = false`).
- After replay close, buffered live events drain in order. No live event with `event_seq < event_seq_at_subscribe` can arrive after replay close (post-COMMIT ordering enforced by W4-B V9 §15 outbox).

### section 4. Scope-filter at dispatch and replay, not only at subscribe (V2 — per-row scope re-resolution per cso CRITICAL-1)

Subscription acceptance verifies actor and route shape, but final claim visibility is late-bound at each individual delivery.

**Per-row scope re-resolution (V2 critical rule):** scope set is resolved from the CURRENT authenticated actor for each row, NOT snapshotted at subscribe or replay-start. The dispatcher's delivery loop:

```rust
for row in version_events_batch {
    let current_scopes = resolve_session_scopes(subscriber.session_token)?;
    if !scope_permits_claim_read(&current_scopes, &row.claim_id)? {
        continue;  // ratchet last_scanned forward; do NOT advance client-visible cursor
    }
    deliver(row, subscriber)?;
}
```

This catches mid-replay scope tightening, mid-replay session re-pairing with narrower scopes, and post-Subscribe scope revocation. If a subscriber's session is invalidated mid-replay, the next row's `resolve_session_scopes` returns the new (or empty) scope set; out-of-scope rows are skipped from that row forward.

**Predicate consistency (cso MEDIUM-1):** `scope_permits_claim_read` is referentially transparent for `(actor, claim_id, claim_state, claim_version)`. Predicate evaluation + payload projection occur inside a consistent claim-read-policy snapshot via versioned read against `claim_version`. Two concurrent dispatcher workers reading the same row see the same predicate result.

**Existence-oracle defense:** cross-scope claim events produce ZERO delivery. The dispatcher does NOT send redacted push/replay notifications for out-of-scope claim events — even a notification that "an event happened" reveals existence.

**`current_cursor` semantics (V3 REWRITE per codex C2 HIGH):** V2 said `SubscribeAck.current_cursor` is the global latest event_seq cursor; codex correctly flagged this contradicts the §4 existence-oracle defense (returning ANY cursor reachable by global latest = signal that an event exists even if out-of-scope). V3:
- `SubscribeAck.current_cursor` is a SUBSCRIBER-VISIBLE cursor: the most recent event whose `scope_permits_*_read` predicate returns true under the subscriber's CURRENT scopes, OR `null` if none.
- Internally, the dispatcher maintains `event_seq_at_subscribe = SELECT MAX(event_seq) FROM version_events` as a high-water mark for the replay/live race barrier (§3) — this state is internal and never surfaced to the subscriber.
- The two values may diverge (global high-water is ahead of subscriber-visible cursor); that divergence is the explicit oracle defense. Subscribers see only events they are permitted to see; they cannot infer the existence of out-of-scope events from `current_cursor`.

### section 5. Composition events gated by `scope_permits_composition_read` (V2 REWRITE per codex HIGH)

V1 relied on `affected_claim_ids` carried in `CompositionVersionEvent`. W4-B V9 §5's actual `CompositionVersionEvent` payload has only `composition_id`, `previous_version`, `current_version`, `cursor`, `reason` — NO `affected_claim_ids`. V2 fix: DOS-589 owns its own composition-scope predicate, no W4-B amendment required.

**`scope_permits_composition_read(actor, composition_id, composition_version)` predicate (DOS-589-owned, V3 versioned per codex C2 MEDIUM):**

V2 resolved claim_refs from "the most recent ability output," which authorizes the WRONG version: the dispatcher must authorize against the claim_refs of the version that was actually published in this event, not whatever the most recent composition reflection happens to carry. V3 pins the version explicitly.

```rust
/// Returns true if the actor's scope set permits reading the composition
/// AT THE EXACT VERSION mutated in this event — i.e., at least one of the
/// composition_version's frozen claim_refs is permitted by the actor's scopes.
fn scope_permits_composition_read(
    actor: &Actor::SurfaceClient,
    composition_id: &CompositionDocId,
    composition_version: i64,
) -> Result<bool> {
    // Resolve the FROZEN claim_refs for this exact composition_version.
    // W4-B V9 §8 composition_versions row carries
    // generated_by_invocation_id for the version; the substrate
    // resolves the version-frozen claim_refs (not the most recent
    // reflection). For each claim_ref, run scope_permits_claim_read.
    let claim_refs = composition_version_frozen_claim_refs(
        composition_id,
        composition_version,
    )?;
    Ok(claim_refs.iter().any(|cr| scope_permits_claim_read(actor, &cr.claim_id)))
}
```

**Delivery rule:**
- For a `CompositionVersionEvent { composition_id, current_version, .. }`, dispatcher calls `scope_permits_composition_read(actor, composition_id, current_version)`.
- If returns true: deliver the event (version-only payload — no claim list).
- If returns false: zero delivery (existence-oracle defense per §4).
- The version-frozen authorization prevents a race where a subsequent version drops the actor's only permitted claim_ref but the dispatcher would still deliver the earlier event's notification using current reflection.

**No `affected_claim_ids` in event payload:** the payload remains version-only (`{composition_id, previous_version, current_version, cursor, reason}`). Subscribers re-invoke the producing ability to get the scope-filtered Composition. The W4-B V9 contract is preserved; the per-event affected-claim leak vector is eliminated.

### section 6. CorrectionRef fetch route shares the inline 409 projection rule

`CorrectionRef.event_log_id` is a bearer pointer.

W4-B V9 section 16 says direct-key fetches by `event_log_id` are scope-gated just like inline correction payloads.

DOS-589 implements `GET /v1/surface/event-log/{event_log_id}` in `src-tauri/src/bridges/surface_client.rs` per W4-B V9 section 37.

The route resolves `event_log_id` to the underlying claim and calls the same scope predicate used by inline 409 correction projection.

Out-of-scope returns redacted envelope only when the caller already has an in-scope reason to know the event exists; otherwise it returns 404.

The route never returns a claim body to an out-of-scope caller.

### section 7. Backpressure semantics: bounded queue, explicit event, replay-required recovery (V2 — thresholds + side-channel defense)

Every connected subscriber has a bounded outbound queue: `tokio::sync::mpsc::channel(1024)`. Thresholds (V2 NEW per eng P3-A + devex P2-4):

- **Soft threshold = 256 events** in queue → dispatcher emits `subscription.backpressure` event (delivered only to the affected subscriber per cso H1), keeps enqueueing.
- **Hard cap = 1024 events** → dispatcher stops enqueueing ordinary live events for that subscriber and marks `replay_required = true` on the subscription. Subscriber must reconnect with `from_cursor` to resume.

**Reserved slots for correctness events:** `claim.write_rejected` and `claim.corrected` are NEVER dropped — they bypass the hard cap via a reserved slot in the channel (or a "must deliver" sub-channel). Coalescing rules: multiple `claim.updated` events may coalesce only when latest `claim_version` and `event_seq` remain correct per Phase 0 lines 630-636.

**Backpressure-as-side-channel defense (V2 per cso H1):**
- `subscription.backpressure` event delivered ONLY to the affected subscriber.
- `dropped_event_count` counts events already filtered to subscriber's permitted set — NEVER includes filtered-away rows.
- No cross-subscriber backpressure metric is exposed on any SurfaceClient route. No aggregate queue depth API.
- CI invariant: no route returns aggregate queue depth keyed by subscription_id.

**Mutation isolation:**
- Backpressure must NOT block W4-B mutation commits (per §12 failure isolation).
- A slow subscriber's queue overflow is its own problem; the substrate writer always commits.

### section 8. Transport: Tauri command channel for User/Agent, signed HTTP loopback for SurfaceClient (V2 — HMAC + wp_user_id + heartbeat pins)

**Tauri command channel** serves native app `Actor::User` and `Actor::Agent` consumers via `tauri::ipc::Channel<DispatchedEvent>` per subscription (per devex P2-2). Channel close = auto-unsubscribe.

**Signed HTTP loopback** serves `Actor::SurfaceClient` consumers. Routes land under `/v1/surface/*` in `src-tauri/src/bridges/surface_client.rs` per W4-B V9 §37.

**Per-frame HMAC integrity (V2 per cso H2):** the push channel (SSE/WebSocket — choice deferred at L0) inherits per-frame HMAC matching W2-B's bridge contract. Subscribe request is signed; subscribe response stream frames carry HMAC envelopes too. Transport-level integrity property is locked at L0; transport choice (SSE vs WebSocket) is implementation discretion.

**`wp_user_id` binding for SubscribeRequest (V2 per cso H3):** if `SubscribeRequest` body carries `wp_user_id`, the bridge precondition `validate_session_bound_wp_user_id` (per W4-B V9 §17) validates against the paired session before dispatcher state is created. Mismatch returns 403 `wrong_user`; no subscription_id is allocated, no checkpoint row is written. Fixture `dos589_fixture_signal_subscribe_wp_user_binding.rs`.

**Heartbeat semantics (V2 NEW per eng P2-B):** dispatcher emits heartbeat on `heartbeat_ms` interval (per `SubscribeAck.heartbeat_ms` from Phase 0 line 612). Subscriber-side miss detection lives in the transport adapter, NOT the service. Heartbeat does NOT advance cursor (it's a liveness signal only).

**L0 contract:**
- Forces route owner (`bridges/surface_client.rs`), service contract (transport-neutral interface), and Phase 0 wire semantics.
- Defers WebSocket vs SSE choice.
- The service API must be transport-neutral enough to support either.

### section 9. Post-COMMIT delivery only

The dispatcher never reads uncommitted mutation state.

W4-B V9 section 15 owns atomic insert of `version_events` inside the mutation transaction.

DOS-589 begins work only after that transaction commits.

Polling `version_events` by `event_seq` is acceptable.

An in-process notification after W4-B commit is acceptable if it still reads the committed row.

Both options must preserve event_seq order.

### section 10. Subscriber identity key and digesting (V2 — per-subscription_id keyed per codex HIGH)

V1 keyed durable checkpoints by `(actor, scopes_digest, subject_filter_digest)` only. That conflated concurrent connections from the same actor (e.g., two browser tabs with same SurfaceClient session) — checkpoint advancement from one tab could starve the other.

**V2 canonical subscriber key (durable):** `(subscription_id, actor_kind, actor_instance, scopes_digest, subject_filter_digest)`.

- `subscription_id` is server-allocated UUIDv4 at Subscribe time, returned in `SubscribeAck.subscription_id`. Per-Subscribe-call, not per-actor.
- Concurrent same-actor Subscribes allocate distinct subscription_ids and maintain independent checkpoints.
- The user-facing shorthand `(actor, scopes, subject_filter)` remains in the Linear acceptance and SubscribeRequest body for filter selection; the subscription_id is the durable key.

**Reconnect with `from_cursor` (V3 REWRITE per codex C3 HIGH):** V2 said reconnect looks up the subscription_id via `cursor → event_seq → checkpoint`. Codex correctly flagged this contradicts §3 V3: it would resolve cursor identity BEFORE authenticating the envelope HMAC, reintroducing the "cursor-as-lookup-authority" path. V3 ordering is envelope-first:

1. Subscriber sends `from_cursor = <envelope>` (the base64 envelope per §3 V3).
2. Dispatcher decodes the envelope → reads embedded `subscription_id`.
3. Dispatcher looks up `subscriber_local_key` and the durable checkpoint by `subscription_id` (the durable key, not the cursor).
4. Dispatcher recomputes the HMAC over `cursor_uuid || subscription_id`; on mismatch OR unknown `subscription_id`, returns the indistinguishable "no events" envelope per §3 V3 — NEVER resolves `cursor_uuid` to `event_seq`.
5. On HMAC match, dispatcher resolves `cursor_uuid` to `event_seq` via the W4-B V9 §15 PK lookup, then continues with the §3 replay query.
6. If the subscriber has no checkpoint AND the envelope is otherwise unverifiable, the request is treated as a fresh Subscribe (no implicit allocation tied to a foreign cursor) — `from_cursor` is ignored, the subscriber must re-Subscribe to receive a fresh `subscription_id` + `subscriber_local_key`.

The cursor is NEVER the lookup authority for the subscription. The `subscription_id` embedded in the envelope IS the lookup key, and the HMAC is what authenticates the envelope's right to make that lookup.

**Digest computation:**
- `scopes_digest` = SHA256 of sorted scope strings joined by `\n`.
- `subject_filter_digest` = SHA256 of canonical JSON for requested subjects.
- Both digests are computed at Subscribe time and stored in `subscription_checkpoints`.

**Durable storage rules:**
- May include opaque claim/composition IDs needed for matching.
- MUST NOT include real customer names, domains, emails, or account details.
- The per-subscription HMAC key for cursor binding (§3) is stored alongside the checkpoint.

### section 11. Cursor acknowledgment and checkpoint timing

The dispatcher advances a subscriber checkpoint only after the event is accepted by transport or serialized into a replay response.

For live push, the service should prefer explicit ack when transport supports it.

If v1.4.2 transport lacks per-event ack, checkpoint advances on enqueue only for events replayable from the prior cursor.

For replay, `ReplayResponse.next_cursor` is the last event included in the response.

`ReplayResponse.next_cursor` is not the newest global cursor.

### section 12. Failure isolation

A slow subscriber cannot block W4-B mutation commit.

A slow subscriber cannot block other subscribers.

A slow subscriber cannot block replay for another subscriber.

A slow subscriber cannot block CorrectionRef fetch for a permitted caller.

Each subscriber queue is isolated.

Dispatcher errors are logged or audited with safe IDs and request IDs, not customer content.

### section 13. Runtime and service ownership (V2 — module placement + concurrency primitives pinned)

**Module placement (V2 per devex P2-1):**
- `src-tauri/src/services/version_dispatcher.rs` — owns subscription registry, dispatch loop, checkpoint persistence, scope-predicate evaluation.
- `src-tauri/src/services/version_events.rs` — typed decoders for `ClaimVersionEvent`, `CompositionVersionEvent`, `CorrectionRef` from W4-B V9 §15 outbox rows.
- `src-tauri/src/commands/version_dispatcher.rs` — Tauri command entrypoint (per devex P2-2).
- `src-tauri/src/bridges/surface_client.rs` — SurfaceClient HTTP routes (per W4-B V9 §37).

**RESERVED `signals/` for ADR-0080 only** (per devex P2-1): no DOS-589 code lives under `signals/`. CI invariant per eng P3-C: version dispatcher routes/methods MUST NOT live in `signals/bus.rs`.

**Concurrency primitives (V2 per devex P1-1):**
- Live subscription registry: `DashMap<SubscriptionId, SubscriberHandle>` — lock-free sharded reads; dispatcher hot path scans per-event registry on subject filter + scope predicate.
- Reconnect lookup index: `DashMap<SubscriberDigest, SubscriptionId>` — supports reconnect-with-cursor flow per §10.
- Per-subscriber outbound queue: bounded `tokio::sync::mpsc::channel(1024)` — backpressure thresholds per §7.
- Durable subscription_checkpoints: SQLite via service layer; never written from command/route handlers.
- NOT `tokio::Mutex<HashMap<..>>` (serializes dispatch); NOT `parking_lot::Mutex` (blocks under contention).
- "No `std::sync::Mutex` across await points" — async-aware Tokio primitives only.

**Boundaries:**
- All persistent mutations go through `services/version_dispatcher.rs`.
- Route handlers in `bridges/surface_client.rs` validate signed session, build `Actor::SurfaceClient`, call service, serialize response.
- Tauri command handlers build `Actor::User`/`Actor::Agent`, call service, emit through Tauri `ipc::Channel`.
- No command handler writes subscriber checkpoints directly.
- No route handler writes event-log state directly.
- `signals/bus.rs` (legacy ADR-0080 emitter) is NOT modified by W4-B-signals work.

### section 13.5. SurfaceClient session expiry handling (V2 NEW per devex P2-3; renumbered V3.2 per codex C4 to preserve §13→§14→§15 ordering)

When a SurfaceClient session expires or is invalidated:

1. Bridge layer detects expiry (any signed request with expired token → 401).
2. Dispatcher receives session-expired signal (in-process notification from bridge OR auto-detected on next delivery attempt).
3. Live handle for the subscription_id is immediately removed from the in-memory registry.
4. Outbound queue is drained without dispatching remaining events.
5. The durable `subscription_checkpoints` row survives — reconnect with fresh session and `from_cursor` is supported.
6. New SubscribeRequest with fresh `wp_user_id` binding is required to resume.
7. Audit emits `subscription.expired` event (operator-visible, NOT subscriber-visible per cso H1 backpressure-as-side-channel rule).
8. Fixture `dos589_fixture_session_expiry.rs` exercises: subscribe → mid-stream session invalidate → no further deliveries → reconnect with fresh session + last-seen cursor → resume.

### section 14. Intelligence Loop fit

DOS-589 does not create new claims, but it is part of the claim lifecycle runtime.

It must preserve subject attribution via claim/composition IDs.

It must preserve temporal ordering via `event_seq`.

It must preserve lifecycle state via event kind.

It must preserve feedback/retry handoff via `CorrectionRef`.

It must preserve trust and existence defense through scope-filtered delivery.

The dispatcher is incomplete if it only renders frontend notifications; runtime consumers must receive the events needed to keep derived state current.

### section 15. Linear issue drift resolution

Linear DOS-589 says "Dispatcher consumes `signal_events` rows" and "Implement pub/sub layer that consumes `signal_events`."

That was true before W4-B V9 introduced the dedicated `version_events` table.

For implementation, W4-B V9 section 15 wins.

DOS-589 dispatches from `version_events`.

DOS-589 orders by `event_seq`.

DOS-589 resolves cursor to `event_seq`.

DOS-589 preserves W4-B V9 section 5 payload schema.

The legacy `signal_events` table remains out of scope except as historical context for why `signals/bus.rs` is not enough.

## Acceptance criteria lifted into DOS-589

### Implementation acceptance

1. **Subscribe/SubscribeAck wire shape.**
   - Source: Phase 0 artifact 02 lines 595-616.
   - Directional decision: section 1, section 8.
   - File targets: `src-tauri/src/services/version_dispatcher.rs`, `src-tauri/src/bridges/surface_client.rs`.
   - Negative fixture: `dos589_fixture_signal_replay_cursor.rs`.
   - Test condition: `SubscribeRequest` accepts `surface_client_id`, `surface`, `streams`, `subjects`, optional `from_cursor`, and optional `max_batch_size`; `SubscribeAck` returns `ok`, `subscription_id`, `heartbeat_ms`, optional `replay_from`, and `current_cursor`.

2. **Subscriber registry keyed on `(actor, scopes, subject_filter)`, persistent across reconnects.**
   - Source: Linear DOS-589 acceptance.
   - Directional decision: section 2, section 10, section 11.
   - File targets: `src-tauri/src/services/version_dispatcher.rs`, W4-B-successor migration if DOS-589 needs a persisted table.
   - Negative fixture: `dos589_fixture_signal_replay_cursor.rs`.
   - Test condition: Reconnect with equivalent actor, sorted scopes, and canonical subject filter recovers the durable checkpoint and subscription identity.

3. **Dispatcher consumes `version_events` rows after W4-B commit.**
   - Source: W4-B V9 section 15 superseding Linear stale `signal_events` wording.
   - Directional decision: section 3, section 9, section 15.
   - File targets: `src-tauri/src/services/version_dispatcher.rs`, W4-B row decoder.
   - Negative fixture: `dos589_fixture_signal_replay_cursor.rs`.
   - Test condition: Dispatcher never observes rolled-back version rows and emits only committed rows in `event_seq` order.

4. **`scope_permits_claim_read(subscriber, claim_id)` gates every delivery.**
   - Source: W4-B V9 section 16 and Linear DOS-589 acceptance.
   - Directional decision: section 4, section 5, section 12.
   - File targets: `src-tauri/src/services/version_dispatcher.rs`, claim projection/read service.
   - Negative fixture: `dos589_fixture_signal_scope_leak.rs`.
   - Test condition: Out-of-scope subscriber receives zero event, zero redacted event, and zero existence hint for that claim_id.

5. **`CorrectionRef.event_log_id` lookup endpoint is scope-filtered identically to inline 409.**
   - Source: W4-B V9 section 5, section 16, section 37 and Linear DOS-589 acceptance.
   - Directional decision: section 6, section 8, section 13.
   - File targets: `src-tauri/src/bridges/surface_client.rs`, `src-tauri/src/services/version_dispatcher.rs`.
   - Negative fixture: `dos589_fixture_correction_ref_replay_scope_leak.rs`.
   - Test condition: Out-of-scope fetch returns redacted envelope or 404 and never returns claim body.

6. **`subscription.backpressure` event when outbound queue exceeds threshold.**
   - Source: Phase 0 artifact 02 lines 624-650 and Linear DOS-589 acceptance.
   - Directional decision: section 7, section 12.
   - File targets: `src-tauri/src/services/version_dispatcher.rs`.
   - Negative fixture: `dos589_fixture_signal_backpressure.rs`.
   - Test condition: Slow subscriber triggers backpressure event and W4-B mutation commit path is not blocked.

7. **Replay-from-cursor streams missed events in order, scope-filtered, with cursor-to-event_seq resolution.**
   - Source: W4-B V9 section 15 and Phase 0 artifact 02 lines 658-683.
   - Directional decision: section 3, section 4, section 11.
   - File targets: `src-tauri/src/services/version_dispatcher.rs`, `src-tauri/src/bridges/surface_client.rs`.
   - Negative fixture: `dos589_fixture_signal_replay_cursor.rs`, `dos589_fixture_signal_scope_leak.rs`.
   - Test condition: Replay resolves cursor to event_seq, queries `event_seq > ? ORDER BY event_seq`, filters every row, and returns `next_cursor`.

8. **Reconnect resilience: ordered, deduplicated, scope-filtered replay from last-seen cursor.**
   - Source: Linear DOS-589 acceptance and Phase 0 artifact 02 reconnection section.
   - Directional decision: section 2, section 3, section 4, section 11.
   - File targets: `src-tauri/src/services/version_dispatcher.rs`.
   - Negative fixture: `dos589_fixture_signal_replay_cursor.rs`.
   - Test condition: Drop and reconnect with last delivered cursor yields no duplicate of that cursor row and delivers all permitted later rows in order.

### Acceptance cross-reference matrix

| AC | Directional decision(s) | Required negative fixture(s) |
|---|---|---|
| AC1 Subscribe/SubscribeAck wire shape | section 1, section 8 | `dos589_fixture_signal_replay_cursor.rs` |
| AC2 Subscriber registry keyed on `(actor, scopes, subject_filter)`, persistent across reconnects | section 2, section 10, section 11 | `dos589_fixture_signal_replay_cursor.rs` |
| AC3 Dispatcher consumes `version_events` rows after W4-B commit | section 3, section 9, section 15 | `dos589_fixture_signal_replay_cursor.rs` |
| AC4 `scope_permits_claim_read(subscriber, claim_id)` gates every delivery | section 4, section 5, section 12 | `dos589_fixture_signal_scope_leak.rs` |
| AC5 `CorrectionRef.event_log_id` lookup endpoint is scope-filtered identically to inline 409 | section 6, section 8, section 13 | `dos589_fixture_correction_ref_replay_scope_leak.rs` |
| AC6 `subscription.backpressure` event when outbound queue exceeds threshold | section 7, section 12 | `dos589_fixture_signal_backpressure.rs` |
| AC7 Replay-from-cursor streams missed events in order, scope-filtered, with cursor-to-event_seq resolution | section 3, section 4, section 11 | `dos589_fixture_signal_replay_cursor.rs`, `dos589_fixture_signal_scope_leak.rs` |
| AC8 Reconnect resilience: ordered, deduplicated, scope-filtered replay from last-seen cursor | section 2, section 3, section 4, section 11 | `dos589_fixture_signal_replay_cursor.rs` |

## Negative fixtures (`src-tauri/tests/dos589_fixture_<name>.rs`)

### `dos589_fixture_signal_scope_leak.rs`

Purpose: prove the dispatcher never creates an existence oracle for claim-bound events.

Setup:

- Seed two claims with different required read scopes.
- Seed W4-B-style `version_events` rows for both claims.
- Register one subscriber with scopes permitting only the first claim.
- Register another subscriber with scopes permitting only the second claim.
- Run live dispatch and replay from an earlier cursor.

Assertions:

- Each subscriber receives only events for permitted claims.
- Out-of-scope claim events are absent, not redacted.
- Queue metrics do not include per-claim denied counts visible to the subscriber.
- Replay does not leak out-of-scope rows that live dispatch suppressed.
- The test fails if any delivery path serializes before calling `scope_permits_claim_read`.

Failure mode caught:

- Filtering at subscribe time only.
- Redacted push notifications that reveal claim existence.
- Replay path bypassing live dispatch filter.
- Composition event leaking affected out-of-scope claim IDs.

### `dos589_fixture_correction_ref_replay_scope_leak.rs`

Purpose: prove direct event-log body fetch follows the same scope rule as inline 409 correction payload.

Setup:

- Seed a W4-B `version_events` row with `correction_event_log_id`.
- Seed durable correction body containing claim-bound content.
- Build a permitted `Actor::SurfaceClient { scopes }`.
- Build an out-of-scope `Actor::SurfaceClient { scopes }`.
- Request `GET /v1/surface/event-log/{event_log_id}` through the SurfaceClient bridge.

Assertions:

- Permitted actor receives the correction body projected through the same claim read service as inline 409.
- Out-of-scope actor receives 404 or redacted envelope.
- Out-of-scope actor never receives claim text, field values, provenance body, or subject metadata.
- Route lives under `bridges/surface_client.rs`.
- `wp_user_id` session binding check runs before event-log fetch when a request carries `wp_user_id`.

Failure mode caught:

- Treating `event_log_id` as sufficient authorization.
- Scope-filtering inline 409 but not replay/fetch.
- Placing route logic outside the canonical SurfaceClient bridge.

### `dos589_fixture_signal_backpressure.rs`

Purpose: prove bounded queues fail closed into replay without blocking substrate commits.

Setup:

- Configure a small outbound queue threshold.
- Register a subscriber whose transport does not drain.
- Insert a burst of committed `version_events` rows.
- Run dispatcher until threshold is exceeded.

Assertions:

- Dispatcher emits `subscription.backpressure` with `subscription_id`, `cursor`, `replay_required`, and `replay_from`.
- The subscriber is marked replay-required.
- Ordinary events stop enqueueing after the hard bound.
- `claim.write_rejected` and `claim.corrected` are not coalesced away.
- W4-B mutation/outbox write path completes independently of subscriber queue state.
- A replay from `replay_from` recovers ordered permitted events.

Failure mode caught:

- Unbounded queue memory growth.
- Slow subscriber blocking the dispatcher or mutation commit path.
- Backpressure without a usable replay cursor.
- Coalescing correctness events away.

### `dos589_fixture_signal_replay_cursor.rs`

Purpose: prove cursor replay is ordered, deduplicated, and reconnect-safe.

Setup:

- Seed `version_events` rows with monotonic `event_seq` and UUIDv4 cursors.
- Register a subscriber with claim/composition/subject filters.
- Deliver rows through live dispatch.
- Drop connection after acknowledging a known cursor.
- Insert more committed rows.
- Reconnect with `from_cursor` set to the last-seen cursor.

Assertions:

- Dispatcher resolves cursor to event_seq.
- Replay query uses `WHERE event_seq > ? ORDER BY event_seq`.
- The row at `from_cursor` is not duplicated.
- Later permitted rows arrive in event_seq order.
- Out-of-scope rows between permitted rows are skipped without changing order of delivered rows.
- `ReplayResponse.next_cursor` equals the last delivered permitted event cursor.
- Unknown/expired cursor returns replay-expired shape, not silent full replay.

Failure mode caught:

- Cursor lexical ordering.
- Timestamp ordering.
- Duplicate replay of last-seen row.
- Replay path skipping scope filter.
- Treating unknown cursor as "start from zero."

## CI invariants

1. **Scope predicate call gate.**
   - Add a test or lint that every function serializing `ClaimVersionEvent`, `CompositionVersionEvent` with affected claims, `ReplayResponse`, or `CorrectionRef` body calls the scope-filter service.
   - Preferred shape: a narrow AST or grep gate over dispatcher module allowlists, similar in spirit to W4-B V9 acceptance section 33.
   - The gate should fail on new dispatcher delivery functions that bypass `scope_permits_claim_read`.

2. **Existence-oracle regression gate.**
   - `dos589_fixture_signal_scope_leak.rs` must cover live dispatch and replay.
   - `dos589_fixture_correction_ref_replay_scope_leak.rs` must cover direct event-log fetch.
   - Both must assert absence, not only redaction, for push/replay delivery.

3. **Version log ordering gate.**
   - Tests must assert `event_seq` ordering.
   - Tests must reject cursor lexical ordering and timestamp ordering.
   - The replay SQL in service code should be easy to locate and should match W4-B V9 section 15.

4. **Post-COMMIT boundary gate.**
   - Dispatcher tests should inject a rolled-back mutation transaction or failed event insert and prove no delivery occurs.
   - If implementation uses in-process notification, notification must happen after commit and still read from SQLite.

5. **Route owner gate.**
   - SurfaceClient dispatcher routes must live in `src-tauri/src/bridges/surface_client.rs`.
   - `src-tauri/src/surface_runtime/mod.rs` may route to that bridge, but should not own dispatcher domain logic.

6. **No direct DB writes from handlers.**
   - Command handlers and route adapters call service methods only.
   - Subscriber checkpoint updates go through `services/version_dispatcher.rs`.

7. **No PII in fixtures.**
   - Tests use generic IDs and examples only.
   - No real customer domains, company names, email addresses, or account details.

8. **Standard gate remains required before implementation closes.**
   - `cargo clippy -- -D warnings`.
   - `cargo test`.
   - `pnpm tsc --noEmit`.

## Interlocks with W4-B + W4-C + W4-A + W5-A

| Consumer | What it expects from DOS-589 | DOS-589 guarantee |
|---|---|---|
| DOS-567 / W4-B outbox | Delivery begins only after `version_events` commit | Dispatcher reads committed rows by `event_seq`; no mutation transaction coupling |
| DOS-567 / W4-B event schema | W4-B V9 section 5 payloads are preserved | Dispatcher decodes and forwards `ClaimVersionEvent`, `CompositionVersionEvent`, and `CorrectionRef` without changing schema semantics |
| DOS-567 / W4-B replay contract | W4-B V9 section 15 cursor/event_seq split is honored | Cursor resolves to event_seq; replay uses `WHERE event_seq > ? ORDER BY event_seq` |
| DOS-569 / W4-C tamper | Cache invalidation fires only for permitted affected claims/compositions | Dispatcher delivers cache-bust events to W4-C subscribers without leaking unrelated claim existence |
| DOS-569 / W4-C scope rule | W4-B V9 section 16 direct-key fetches are scope-gated | `GET /v1/surface/event-log/{event_log_id}` uses same predicate as inline 409 |
| DOS-572 / W4-A renderer | Mounted block state receives cache-bust without polling | Subscribe/SubscribeAck plus replay provides current cursor and missed events |
| DOS-572 / W4-A renderer | Backpressure puts blocks into refreshing/replay state | `subscription.backpressure` includes replay cursor and replay-required flag |
| DOS-573 / W5-A feedback router | 423/409 retry-after-event can wait on cursor | Dispatcher resolves retry cursor to event_seq and pushes terminal event or supports replay |
| DOS-573 / W5-A feedback router | Cross-scope feedback subscribers learn nothing | Out-of-scope claim events receive zero delivery |
| Tauri native surface | User/Agent subscribers can consume runtime events | Tauri command channel uses same service and scope predicate |
| SurfaceClient loopback | WP Studio subscribers can consume runtime events | Signed `/v1/surface/*` route uses `bridges/surface_client.rs` and session-bound actor/scopes |

## What DOS-589 explicitly does NOT own

- W4-B outbox row schema.
- W4-B `version_events` migration.
- W4-B `event_seq` and cursor generation.
- W4-B version assignment.
- W4-B `ClaimVersionEvent`, `CompositionVersionEvent`, and `CorrectionRef` schema definition.
- W4-B mutation transaction and post-rollback `mutation_aborted` guarantees.
- W4-C Ed25519 projection signing.
- W4-C tamper cache storage.
- W4-C quarantine behavior.
- W4-A Gutenberg block renderer UI.
- W4-A cache-bust rendering policy beyond delivery of events.
- W5-A feedback routing and correction UX.
- WP-side JavaScript subscription client.
- PHP route implementation.
- Product/design UI copy.
- Legacy ADR-0080 `signal_events` propagation.
- Generic intelligence signal scoring.
- Multi-process distributed dispatcher leadership.
- Retention policy beyond the Phase 0 minimum replay assumption unless implementation needs a migration.

## Open questions

| ID | Question | Proposed L0 resolution |
|---|---|---|
| Q1 | Does DOS-589 need a new migration for durable subscriber checkpoints? | Yes if no existing table can hold `(actor, scopes_digest, subject_filter_digest, last_event_seq, last_cursor)`. Keep it small and dispatcher-specific. |
| Q2 | Is subscriber persistence durable table or config file? | SQLite table. It is runtime state tied to DB event_seq, so DB is the right authority. |
| Q3 | Does backpressure persist across reconnects? | Persist replay-required state and last safe cursor; do not persist the outbound queue. |
| Q4 | Does `current_cursor` in `SubscribeAck` mean global latest or subscriber-visible latest? | Subscriber-visible latest after scope filtering. Global latest can create an existence/timing oracle. |
| Q5 | What is the exact transport for SurfaceClient push? | L0 does not force WebSocket vs SSE. It forces route owner, service contract, and Phase 0 wire semantics. |
| Q6 | How is `scope_permits_claim_read` implemented if no helper exists? | DOS-589 authors it in the service layer using existing claim projection/read services and `Actor::SurfaceClient { scopes }`; do not use ad-hoc string checks in route handlers. |
| Q7 | How should composition-only events without affected claims be filtered? | Use composition/subject read predicate when available; otherwise require affected claim expansion before delivering claim-bound content. |
| Q8 | Should out-of-scope CorrectionRef fetch return 404 or redacted envelope? | Use redacted envelope only when the caller already learned the event exists through an in-scope synchronous path; otherwise 404 to avoid oracle. |
| Q9 | Does DOS-589 update Linear issue text from `signal_events` to `version_events`? | L0 closure should comment/update Linear so implementation follows W4-B V9 section 15. |
| Q10 | Does this require design review? | No. Pure Rust substrate; no UI or user-facing prose. |

## Linear dependency edges

- DOS-589 is a sibling of DOS-567 under DOS-546 v1.4.2.
- DOS-589 is blocked by DOS-567.
- DOS-567 provides W4-B V9 section 5 payload schemas.
- DOS-567 provides W4-B V9 section 15 `version_events` schema and outbox atomicity.
- DOS-567 provides W4-B V9 section 16 scope-filter class rule.
- DOS-567 provides W4-B V9 section 17 `wp_user_id` session-binding rule.
- DOS-567 provides W4-B V9 section 37 route owner rule.
- DOS-589 blocks DOS-569.
- DOS-589 blocks DOS-572.
- DOS-589 blocks DOS-573.
- DOS-569 depends on DOS-589 for W4-C tamper cache invalidation delivery.
- DOS-572 depends on DOS-589 for W4-A renderer cache-bust delivery.
- DOS-573 depends on DOS-589 for W5-A retry-after-event delivery.
- DOS-589 should be linked as related to DOS-546 project execution.
- DOS-589 should carry a Linear comment noting that `version_events` supersedes stale `signal_events` wording in the original issue body.

## L0 reviewer panel - required runners

- `/plan-eng-review` - required. Focus: service/module boundary, replay correctness, failure isolation, and migration/checkpoint shape.
- `/cso` - required. Focus: scope-filter bypass, direct-key `event_log_id` fetch, existence-oracle defense, session-bound actor/scopes.
- `/plan-devex-review` - required. Focus: route owner clarity, transport-neutral API, testability, and implementation handoff to W4-C/W4-A/W5-A consumers.
- `/codex challenge` - required. Focus: adversarial replay ordering, cursor semantics, backpressure edge cases, and Linear/W4-B contract drift.
- `/plan-design-review` - not required. Focus: pure substrate, no visual surface, no copy/interaction design.

Reviewer convergence target:

- eng APPROVE
- cso APPROVE
- devex APPROVE
- codex APPROVE

Conditional approvals must be folded into a successor packet version before implementation starts.

## Acceptance for L0 closure

This packet is L0-ready when:

1. All 15 sections remain present in order.
2. W4-B V9 references are preserved by section number: 5, 15, 16, 17, and 37.
3. Linear DOS-589 acceptance criteria are restated as testable implementation items.
4. Every acceptance criterion cross-references a directional decision and at least one negative fixture.
5. The packet explicitly resolves the `signal_events` vs `version_events` drift in favor of W4-B V9 section 15.
6. The packet keeps DOS-589 scoped to pure Rust substrate.
7. The packet says route adapters do not mutate DB state directly.
8. Negative fixtures cover live dispatch, replay, CorrectionRef fetch, and backpressure.
9. CI invariants include a class-wide scope-filter bypass gate.
10. Interlocks with DOS-567, DOS-569, DOS-572, and DOS-573 are explicit.
11. L0 reviewer panel is eng + cso + devex + codex, with no design review.
12. Linear dependency edges are posted or confirmed.
13. Any L0 reviewer findings are folded into V2+ changelog entries.
14. The output file remains uncommitted until the user explicitly asks for a commit.
15. Implementation starts only after unanimous L0 approval or explicit L6 override.

DOS-589 implementation may start after L0 closes and DOS-567's `version_events` schema/outbox guarantee is available. W4-C, W4-A, and W5-A should not depend on ad-hoc polling while waiting for this issue; their cache/retry consumers should bind to this dispatcher contract.
