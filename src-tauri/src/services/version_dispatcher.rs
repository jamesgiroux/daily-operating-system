//! W4-B-signals dispatcher.
//!
//! Pub/sub layer over the W4-B V9 §15 `version_events` outbox. Reads
//! post-COMMIT rows in `event_seq` order and delivers them to subscribers
//! whose scope grant authorizes the underlying claim or composition. Replay
//! uses the W4-B UUIDv4 cursor as the row address; live and replay paths run
//! the same per-row `scope_permits_claim_read` predicate so a subscriber
//! that gained or lost scopes mid-stream sees the correct projection from
//! the next row forward.
//!
//! The dispatcher does NOT own:
//!   - mutation transactions or version assignment (W4-B owns those),
//!   - the `version_events` table schema (W4-B v172 owns it),
//!   - WordPress or Gutenberg renderer state (W4-A consumes our events),
//!   - claim-read SQL projection — that lives in
//!     [`crate::bridges::correction_payload::project_claim_for_scope`] and
//!     is the single source of truth for scope/sensitivity gating.
//!
//! Wire types match Phase 0 artifact 02 lines 595-683 (Subscribe / Replay /
//! Backpressure). Cursor envelope is a base64 of `{cursor_uuid,
//! subscription_id, mac}` per packet §3 V3 — the W4-B UUIDv4 row PK is
//! preserved verbatim and the per-subscription HMAC binds the envelope to
//! the requester so a captured cursor cannot probe event_seq lifetime.

use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine as _;
use chrono::Utc;
use dashmap::DashMap;
use rusqlite::{params, OptionalExtension};
use ring::hmac;
use ring::rand::{SecureRandom, SystemRandom};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use tokio::sync::mpsc;
use uuid::Uuid;

use crate::abilities::registry::ScopeSet;
use crate::abilities::Actor;
use crate::bridges::project_claim_for_scope;
use crate::db::ActionDb;
use crate::services::versioning::VersionEventKind;

/// Soft backpressure threshold per packet §7 / eng P3-A. Crossing this emits
/// `subscription.backpressure` to the affected subscriber only; live events
/// continue to enqueue until the hard cap.
pub const BACKPRESSURE_SOFT_THRESHOLD: usize = 256;

/// Hard backpressure cap per packet §7 / eng P3-A. Above this the dispatcher
/// stops enqueueing ordinary live events for the subscriber and marks
/// `replay_required = true`; reconnect with `from_cursor` is the recovery
/// path.
pub const BACKPRESSURE_HARD_CAP: usize = 1024;

/// Maximum replay batch size per packet §3 / devex P2-5. Larger requested
/// batches are silently capped; `ReplayResponse.has_more = true` signals the
/// caller to iterate.
pub const MAX_REPLAY_BATCH: usize = 500;

// ---------------------------------------------------------------------------
// Wire types (Phase 0 artifact 02 lines 595-683)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SubscribeRequest {
    pub surface_client_id: String,
    pub surface: String,
    pub streams: Vec<String>,
    #[serde(default)]
    pub subjects: SubjectFilter,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub from_cursor: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_batch_size: Option<u32>,
    /// W4-B V9 §17 session-bound `wp_user_id`. Validated by the bridge before
    /// reaching the dispatcher; included on the wire so an unbound or
    /// mismatched assertion is rejected before any subscription state is
    /// created.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub wp_user_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct SubjectFilter {
    #[serde(default)]
    pub claim_ids: Vec<String>,
    #[serde(default)]
    pub composition_ids: Vec<String>,
}

impl SubjectFilter {
    pub fn is_empty(&self) -> bool {
        self.claim_ids.is_empty() && self.composition_ids.is_empty()
    }

    /// Per-kind subject matching. A subscription that explicitly lists
    /// `composition_ids` is asking ONLY for those compositions — it should
    /// not also receive every permitted claim event by virtue of leaving
    /// `claim_ids` empty. The empty-list-wildcard semantic only applies
    /// when the WHOLE filter is empty (subscribe-to-everything).
    pub fn matches_claim(&self, claim_id: &str) -> bool {
        if self.is_empty() {
            return true;
        }
        self.claim_ids.iter().any(|id| id == claim_id)
    }

    pub fn matches_composition(&self, composition_id: &str) -> bool {
        if self.is_empty() {
            return true;
        }
        self.composition_ids.iter().any(|id| id == composition_id)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SubscribeAck {
    pub ok: bool,
    pub subscription_id: String,
    pub heartbeat_ms: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub replay_from: Option<String>,
    /// Subscriber-visible cursor: the most recent permitted event under the
    /// subscriber's current scopes, or `null` if none exist. Packet §4 V3
    /// rules out global-latest to defend against the existence oracle.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub current_cursor: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ReplayRequest {
    pub subscription_id: String,
    pub from_cursor: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_batch_size: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ReplayResponse {
    pub events: Vec<DispatchedEvent>,
    pub has_more: bool,
    /// The cursor of the last DELIVERED (permitted) event, never the last
    /// scanned event_seq. Packet §3.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub next_cursor: Option<String>,
    /// True when the supplied `from_cursor` could not be authenticated
    /// (HMAC mismatch, unknown subscription, or row gone). Mapped to Phase 0
    /// `replay-expired` envelope.
    #[serde(default)]
    pub replay_expired: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DispatchedEvent {
    pub cursor: String,
    pub event_kind: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub claim_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub composition_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub previous_version: Option<u64>,
    pub current_version: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub correction_event_log_id: Option<String>,
    pub actor_kind: String,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct BackpressureEvent {
    pub event_kind: &'static str,
    pub subscription_id: String,
    pub cursor: Option<String>,
    pub replay_required: bool,
    pub replay_from: Option<String>,
    pub dropped_event_count: u64,
}

impl BackpressureEvent {
    pub const KIND: &'static str = "subscription.backpressure";
}

// ---------------------------------------------------------------------------
// Errors
// ---------------------------------------------------------------------------

#[derive(Debug, thiserror::Error)]
pub enum DispatcherError {
    #[error("subscriber actor not authorized for dispatcher: {0}")]
    ActorNotAuthorized(String),
    #[error("subscription not found")]
    SubscriptionNotFound,
    #[error("database error: {0}")]
    Database(#[from] rusqlite::Error),
    #[error("internal: {0}")]
    Internal(String),
}

// ---------------------------------------------------------------------------
// Cursor envelope (packet §3 V3)
// ---------------------------------------------------------------------------

/// On-wire cursor envelope. `cursor_uuid` is the W4-B V9 §15 PK; `mac` binds
/// the envelope to the per-subscription HMAC key so a captured cursor cannot
/// be used by a foreign subscription to probe event_seq lifetime.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct CursorEnvelope {
    #[serde(rename = "cursor_uuid")]
    cursor_uuid: String,
    #[serde(rename = "subscription_id")]
    subscription_id: String,
    #[serde(rename = "mac")]
    mac: String,
}

impl CursorEnvelope {
    fn encode(cursor_uuid: &str, subscription_id: &str, local_key: &[u8]) -> String {
        let mac = compute_mac(local_key, cursor_uuid, subscription_id);
        let envelope = Self {
            cursor_uuid: cursor_uuid.to_string(),
            subscription_id: subscription_id.to_string(),
            mac,
        };
        let json = serde_json::to_vec(&envelope).expect("envelope is always serializable");
        URL_SAFE_NO_PAD.encode(json)
    }

    fn decode(wire: &str) -> Option<Self> {
        let bytes = URL_SAFE_NO_PAD.decode(wire.as_bytes()).ok()?;
        serde_json::from_slice(&bytes).ok()
    }
}

fn compute_mac(local_key: &[u8], cursor_uuid: &str, subscription_id: &str) -> String {
    let key = hmac::Key::new(hmac::HMAC_SHA256, local_key);
    let mut ctx = hmac::Context::with_key(&key);
    ctx.update(cursor_uuid.as_bytes());
    ctx.update(subscription_id.as_bytes());
    hex::encode(ctx.sign().as_ref())
}

fn constant_time_eq(a: &str, b: &str) -> bool {
    if a.len() != b.len() {
        return false;
    }
    let mut diff: u8 = 0;
    for (x, y) in a.bytes().zip(b.bytes()) {
        diff |= x ^ y;
    }
    diff == 0
}

// ---------------------------------------------------------------------------
// Digests for the reconnect-lookup index
// ---------------------------------------------------------------------------

fn scopes_digest(scopes: &ScopeSet) -> String {
    let sorted: Vec<&str> = scopes.iter().map(|s| s.as_str()).collect();
    let mut hasher = Sha256::new();
    hasher.update(sorted.join("\n").as_bytes());
    hex::encode(hasher.finalize())
}

fn subject_filter_digest(filter: &SubjectFilter) -> String {
    // Canonical JSON: sorted ids inside sorted arrays.
    let mut claims: Vec<&str> = filter.claim_ids.iter().map(|s| s.as_str()).collect();
    claims.sort_unstable();
    let mut comps: Vec<&str> = filter.composition_ids.iter().map(|s| s.as_str()).collect();
    comps.sort_unstable();
    let canonical = serde_json::json!({
        "claim_ids": claims,
        "composition_ids": comps,
    });
    let mut hasher = Sha256::new();
    hasher.update(canonical.to_string().as_bytes());
    hex::encode(hasher.finalize())
}

fn actor_kind_str(actor: &Actor) -> &'static str {
    match actor {
        Actor::User => "user",
        Actor::Agent => "agent",
        Actor::Admin => "admin",
        Actor::System => "system",
        Actor::SurfaceClient { .. } => "surface_client",
    }
}

fn actor_instance_str(actor: &Actor) -> String {
    match actor {
        Actor::SurfaceClient { instance, .. } => instance.as_str().to_string(),
        // Non-SurfaceClient actors do not carry an instance id; subscriptions
        // from those actors are scoped per-Tauri-channel and use the
        // subscription_id itself as the instance discriminator. Callers pass
        // a stable session identifier here when reconnect continuity is
        // required.
        _ => format!("{}-default", actor_kind_str(actor)),
    }
}

fn actor_scopes(actor: &Actor) -> Result<&ScopeSet, DispatcherError> {
    match actor {
        Actor::SurfaceClient { scopes, .. } => Ok(scopes),
        _ => Err(DispatcherError::ActorNotAuthorized(format!(
            "{} actors must be paired with a ScopeSet via SurfaceClient binding",
            actor_kind_str(actor)
        ))),
    }
}

// ---------------------------------------------------------------------------
// Subscriber records
// ---------------------------------------------------------------------------

/// In-memory live subscriber handle. The outbound queue is `tokio::mpsc`; the
/// dispatcher uses `try_send` so a slow consumer cannot block live commits.
pub struct SubscriberHandle {
    pub subscription_id: String,
    pub sender: mpsc::Sender<DispatchedEvent>,
    pub backpressure_tx: mpsc::Sender<BackpressureEvent>,
    pub subject_filter: SubjectFilter,
    /// Snapshot of the actor's scope grant at subscribe-time. Per-row
    /// dispatch re-fetches the current scope (packet §4 V2) from this snapshot
    /// in the same process; if the SurfaceClient session is revoked the
    /// bridge layer drops the handle from the registry.
    pub scopes: ScopeSet,
    pub actor: Actor,
}

/// Bundled inputs to [`VersionDispatcher::replay_after_seq`] so the helper
/// stays under the project clippy `too_many_arguments` ceiling (CLAUDE.md
/// limit is 7).
struct ReplayAfterSeqCtx<'a> {
    subscription_id: &'a str,
    checkpoint: &'a SubscriptionCheckpoint,
    event_seq: i64,
    actor: &'a Actor,
    subjects: &'a SubjectFilter,
    max_batch_size: Option<u32>,
}

/// Bundled inputs for [`VersionDispatcher::load_or_create_checkpoint`].
struct CheckpointIdentity<'a> {
    actor_kind: &'a str,
    actor_instance: &'a str,
    scopes_digest: &'a str,
    subject_filter_digest: &'a str,
    now: &'a str,
}

/// Durable subscription checkpoint as persisted in `subscription_checkpoints`.
#[derive(Debug, Clone)]
struct SubscriptionCheckpoint {
    subscription_id: String,
    actor_kind: String,
    actor_instance: String,
    scopes_digest: String,
    subject_filter_digest: String,
    subscriber_local_key: Vec<u8>,
    last_acked_event_seq: i64,
    last_acked_cursor_uuid: Option<String>,
    last_scanned_event_seq: i64,
    replay_required: bool,
}

// ---------------------------------------------------------------------------
// Version event decoder (W4-B V9 §15)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub(crate) struct VersionEventRow {
    pub event_seq: i64,
    pub cursor: String,
    pub event_kind: VersionEventKind,
    pub claim_id: Option<String>,
    pub composition_id: Option<String>,
    pub previous_version: Option<u64>,
    pub current_version: u64,
    pub reason: Option<String>,
    pub correction_event_log_id: Option<String>,
    pub actor_kind: String,
    pub created_at: String,
}

fn parse_event_kind(raw: &str) -> Option<VersionEventKind> {
    use VersionEventKind::*;
    Some(match raw {
        "claim.updated" => ClaimUpdated,
        "claim.corrected" => ClaimCorrected,
        "claim.superseded" => ClaimSuperseded,
        "claim.tombstoned" => ClaimTombstoned,
        "claim.write_rejected" => ClaimWriteRejected,
        "claim.conflict_detected" => ClaimConflictDetected,
        "composition.updated" => CompositionUpdated,
        "composition.write_rejected" => CompositionWriteRejected,
        "mutation_aborted" => MutationAborted,
        _ => return None,
    })
}

fn read_version_events_after(
    db: &ActionDb,
    after_event_seq: i64,
    limit: usize,
) -> Result<Vec<VersionEventRow>, rusqlite::Error> {
    let mut stmt = db.conn_ref().prepare(
        "SELECT event_seq, cursor, event_kind, claim_id, composition_id, \
                previous_version, current_version, reason, correction_event_log_id, \
                actor_kind, created_at \
         FROM version_events \
         WHERE event_seq > ?1 \
         ORDER BY event_seq \
         LIMIT ?2",
    )?;
    let rows = stmt.query_map(params![after_event_seq, limit as i64], |row| {
        let kind_raw: String = row.get(2)?;
        let event_kind = parse_event_kind(&kind_raw).ok_or_else(|| {
            rusqlite::Error::InvalidColumnType(2, kind_raw.clone(), rusqlite::types::Type::Text)
        })?;
        let prev: Option<i64> = row.get(5)?;
        let current: i64 = row.get(6)?;
        Ok(VersionEventRow {
            event_seq: row.get(0)?,
            cursor: row.get(1)?,
            event_kind,
            claim_id: row.get(3)?,
            composition_id: row.get(4)?,
            previous_version: prev.map(|v| v as u64),
            current_version: current as u64,
            reason: row.get(7)?,
            correction_event_log_id: row.get(8)?,
            actor_kind: row.get(9)?,
            created_at: row.get(10)?,
        })
    })?;
    rows.collect::<Result<Vec<_>, _>>()
}

fn resolve_cursor_to_event_seq(
    db: &ActionDb,
    cursor_uuid: &str,
) -> Result<Option<i64>, rusqlite::Error> {
    db.conn_ref()
        .query_row(
            "SELECT event_seq FROM version_events WHERE cursor = ?1",
            params![cursor_uuid],
            |row| row.get::<_, i64>(0),
        )
        .optional()
}

// ---------------------------------------------------------------------------
// Scope predicate
// ---------------------------------------------------------------------------

/// `scope_permits_claim_read(actor, claim_id)` per packet §4. Delegates to
/// the existing bridge projection so live, replay, and correction-fetch
/// paths all share one decision rule. Out-of-scope ⇒ `false` ⇒ zero delivery
/// (no redacted notification; existence-oracle defense per §4).
pub fn scope_permits_claim_read(
    db: &ActionDb,
    actor: &Actor,
    claim_id: &str,
) -> bool {
    let Some(payload) = project_claim_for_scope(db, claim_id, actor) else {
        // Claim not found ⇒ nothing to deliver. Treat as out-of-scope from the
        // subscriber's perspective so dispatch never emits a "you missed an
        // event for a claim you can't see" signal.
        return false;
    };
    !payload.scope_redacted && payload.claim.is_some()
}

/// `scope_permits_composition_read(actor, composition_id)` per packet §5 V3.
/// The substrate stores a single `composition_versions` row per composition
/// (current version only); per-version frozen claim_refs are not retained.
/// This predicate therefore authorizes against the current composition row.
/// Per-version-frozen authorization is filed as a substrate follow-up in
/// the v1.4.2 wave maintenance backlog.
pub fn scope_permits_composition_read(
    db: &ActionDb,
    actor: &Actor,
    composition_id: &str,
) -> bool {
    use crate::bridges::project_composition_for_scope;
    let Some(payload) = project_composition_for_scope(db, composition_id, actor) else {
        return false;
    };
    !payload.scope_redacted
}

// ---------------------------------------------------------------------------
// Dispatcher service
// ---------------------------------------------------------------------------

/// Version-event dispatcher service. Holds the in-memory subscriber registry
/// and drives delivery loops against `version_events`. Persistence is the
/// responsibility of the caller's `ActionDb`; this service writes via
/// `with_transaction` so route adapters and Tauri commands never mutate
/// `subscription_checkpoints` directly (CLAUDE.md "All mutations go through
/// services/" rule).
pub struct VersionDispatcher {
    handles: DashMap<String, SubscriberHandle>,
    /// Reverse index for reconnect: maps the digested subscriber identity to
    /// its durable subscription_id. Reads serve `from_cursor` envelope
    /// validation; writes only happen at subscribe-time.
    reconnect_index: DashMap<String, String>,
    rng: SystemRandom,
    soft_threshold: usize,
    hard_cap: usize,
}

impl Default for VersionDispatcher {
    fn default() -> Self {
        Self::new()
    }
}

impl VersionDispatcher {
    pub fn new() -> Self {
        Self::with_capacity(BACKPRESSURE_SOFT_THRESHOLD, BACKPRESSURE_HARD_CAP)
    }

    /// Construct with custom backpressure thresholds. Tests use this to
    /// exercise hard-cap behavior without seeding 1024+ events.
    pub fn with_capacity(soft_threshold: usize, hard_cap: usize) -> Self {
        Self {
            handles: DashMap::new(),
            reconnect_index: DashMap::new(),
            rng: SystemRandom::new(),
            soft_threshold,
            hard_cap,
        }
    }

    /// SurfaceClient stateless-poll subscribe: allocates the durable
    /// `subscription_id` + per-subscription HMAC key (or reuses the existing
    /// checkpoint for reconnect) but does NOT register a live push handle.
    /// WordPress-side callers poll [`Self::replay`] with `from_cursor`; the
    /// dispatcher never accumulates queue state for them.
    pub fn subscribe_stateless(
        &self,
        db: &ActionDb,
        request: &SubscribeRequest,
        actor: Actor,
    ) -> Result<SubscribeAck, DispatcherError> {
        let scopes = actor_scopes(&actor)?;
        let scopes_d = scopes_digest(scopes);
        let filter_d = subject_filter_digest(&request.subjects);
        let kind = actor_kind_str(&actor);
        let instance = actor_instance_str(&actor);
        let reconnect_key = format!("{kind}|{instance}|{scopes_d}|{filter_d}");
        let now = Utc::now().to_rfc3339();
        let checkpoint = self.load_or_create_checkpoint(
            db,
            CheckpointIdentity {
                actor_kind: kind,
                actor_instance: &instance,
                scopes_digest: &scopes_d,
                subject_filter_digest: &filter_d,
                now: &now,
            },
        )?;
        self.reconnect_index
            .insert(reconnect_key, checkpoint.subscription_id.clone());

        let current_cursor =
            self.current_subscriber_visible_cursor(&checkpoint, &request.subjects)?;
        let replay_from = if checkpoint.replay_required {
            checkpoint.last_acked_cursor_uuid.as_ref().map(|uuid| {
                CursorEnvelope::encode(uuid, &checkpoint.subscription_id, &checkpoint.subscriber_local_key)
            })
        } else {
            None
        };
        Ok(SubscribeAck {
            ok: true,
            subscription_id: checkpoint.subscription_id,
            heartbeat_ms: 30_000,
            replay_from,
            current_cursor,
        })
    }

    /// Subscribe an actor. Allocates a fresh `subscription_id` and
    /// per-subscription HMAC key on first call; reconnect (same actor +
    /// scopes + filter) reuses the durable checkpoint.
    ///
    /// `(handle_sender, backpressure_sender, replay_required)` for the
    /// caller transport to bind.
    pub fn subscribe(
        &self,
        db: &ActionDb,
        request: &SubscribeRequest,
        actor: Actor,
    ) -> Result<
        (
            SubscribeAck,
            mpsc::Receiver<DispatchedEvent>,
            mpsc::Receiver<BackpressureEvent>,
        ),
        DispatcherError,
    > {
        let scopes = actor_scopes(&actor)?;
        let scopes_d = scopes_digest(scopes);
        let filter_d = subject_filter_digest(&request.subjects);
        let kind = actor_kind_str(&actor);
        let instance = actor_instance_str(&actor);
        let reconnect_key = format!("{kind}|{instance}|{scopes_d}|{filter_d}");

        let now = Utc::now().to_rfc3339();
        let checkpoint = self.load_or_create_checkpoint(
            db,
            CheckpointIdentity {
                actor_kind: kind,
                actor_instance: &instance,
                scopes_digest: &scopes_d,
                subject_filter_digest: &filter_d,
                now: &now,
            },
        )?;

        let (tx, rx) = mpsc::channel::<DispatchedEvent>(self.hard_cap);
        let (bp_tx, bp_rx) = mpsc::channel::<BackpressureEvent>(8);
        let handle = SubscriberHandle {
            subscription_id: checkpoint.subscription_id.clone(),
            sender: tx,
            backpressure_tx: bp_tx,
            subject_filter: request.subjects.clone(),
            scopes: scopes.clone(),
            actor,
        };
        self.handles
            .insert(checkpoint.subscription_id.clone(), handle);
        self.reconnect_index
            .insert(reconnect_key, checkpoint.subscription_id.clone());

        let current_cursor =
            self.current_subscriber_visible_cursor(&checkpoint, &request.subjects)?;

        let replay_from = if checkpoint.replay_required {
            checkpoint.last_acked_cursor_uuid.as_ref().map(|uuid| {
                CursorEnvelope::encode(uuid, &checkpoint.subscription_id, &checkpoint.subscriber_local_key)
            })
        } else {
            None
        };

        let ack = SubscribeAck {
            ok: true,
            subscription_id: checkpoint.subscription_id,
            heartbeat_ms: 30_000,
            replay_from,
            current_cursor,
        };
        Ok((ack, rx, bp_rx))
    }

    /// Stateless replay for SurfaceClient pollers. The route adapter supplies
    /// the validated `Actor::SurfaceClient` from the session and the
    /// `SubjectFilter` the client carries on the wire; the dispatcher
    /// verifies the filter digest matches the durable checkpoint before
    /// resolving the cursor, so a foreign caller cannot widen its own filter
    /// after subscribe.
    pub fn replay_stateless(
        &self,
        db: &ActionDb,
        request: &ReplayRequest,
        actor: &Actor,
        subjects: &SubjectFilter,
    ) -> Result<ReplayResponse, DispatcherError> {
        let Some(envelope) = CursorEnvelope::decode(&request.from_cursor) else {
            return Ok(replay_expired_response());
        };
        if envelope.subscription_id != request.subscription_id {
            return Ok(replay_expired_response());
        }
        let Some(checkpoint) = self.load_checkpoint_by_id(db, &request.subscription_id)? else {
            return Ok(replay_expired_response());
        };

        // Actor-binding check: the requesting SurfaceClient session must
        // match the subscription's stored owner. A captured envelope from
        // another session cannot be replayed against the same
        // subscription_id by a foreign actor — that would let the foreign
        // session probe existence + advance the cursor (checkpoint-DoS +
        // existence oracle). All three digests must align before any
        // cursor resolution.
        if !actor_matches_checkpoint(actor, &checkpoint) {
            return Ok(replay_expired_response());
        }

        let expected_digest = subject_filter_digest(subjects);
        if !constant_time_eq(&expected_digest, &checkpoint.subject_filter_digest) {
            return Ok(replay_expired_response());
        }

        let expected_mac = compute_mac(
            &checkpoint.subscriber_local_key,
            &envelope.cursor_uuid,
            &envelope.subscription_id,
        );
        if !constant_time_eq(&expected_mac, &envelope.mac) {
            return Ok(replay_expired_response());
        }

        let Some(event_seq) = resolve_cursor_to_event_seq(db, &envelope.cursor_uuid)? else {
            return Ok(replay_expired_response());
        };

        self.replay_after_seq(
            db,
            ReplayAfterSeqCtx {
                subscription_id: &request.subscription_id,
                checkpoint: &checkpoint,
                event_seq,
                actor,
                subjects,
                max_batch_size: request.max_batch_size,
            },
        )
    }

    /// Replay scope-filtered events strictly after the supplied cursor.
    /// Returns `replay_expired = true` when the envelope cannot be
    /// authenticated or the cursor row is unknown — the response shape is
    /// otherwise identical so a caller probing with foreign cursors cannot
    /// distinguish the failure modes.
    pub fn replay(
        &self,
        db: &ActionDb,
        request: &ReplayRequest,
    ) -> Result<ReplayResponse, DispatcherError> {
        let Some(envelope) = CursorEnvelope::decode(&request.from_cursor) else {
            return Ok(replay_expired_response());
        };

        // Envelope-first lookup (packet §10 V3): subscription_id is the
        // durable key, not the cursor_uuid.
        if envelope.subscription_id != request.subscription_id {
            return Ok(replay_expired_response());
        }

        let Some(checkpoint) = self.load_checkpoint_by_id(db, &request.subscription_id)? else {
            return Ok(replay_expired_response());
        };

        // Need the live handle for subject_filter + actor; if the subscriber
        // disconnected we still owe replay against its durable identity.
        // The live handle's actor must also match the stored checkpoint —
        // a stale handle whose session has been re-paired to a different
        // actor (or a handle reused via DashMap collision) cannot replay
        // events for the previously-bound subscription.
        if let Some(handle) = self.handles.get(&request.subscription_id) {
            if !actor_matches_checkpoint(&handle.actor, &checkpoint) {
                return Ok(replay_expired_response());
            }
        }

        let expected_mac = compute_mac(
            &checkpoint.subscriber_local_key,
            &envelope.cursor_uuid,
            &envelope.subscription_id,
        );
        if !constant_time_eq(&expected_mac, &envelope.mac) {
            return Ok(replay_expired_response());
        }

        let Some(event_seq) = resolve_cursor_to_event_seq(db, &envelope.cursor_uuid)? else {
            return Ok(replay_expired_response());
        };

        // Need the live handle for subject_filter + actor; if the subscriber
        // disconnected we still owe replay against its durable identity.
        let (subject_filter, actor) = match self.handles.get(&request.subscription_id) {
            Some(handle) => (handle.subject_filter.clone(), handle.actor.clone()),
            None => return Ok(replay_expired_response()),
        };

        self.replay_after_seq(
            db,
            ReplayAfterSeqCtx {
                subscription_id: &request.subscription_id,
                checkpoint: &checkpoint,
                event_seq,
                actor: &actor,
                subjects: &subject_filter,
                max_batch_size: request.max_batch_size,
            },
        )
    }

    fn replay_after_seq(
        &self,
        db: &ActionDb,
        ctx: ReplayAfterSeqCtx<'_>,
    ) -> Result<ReplayResponse, DispatcherError> {
        let ReplayAfterSeqCtx {
            subscription_id,
            checkpoint,
            event_seq,
            actor,
            subjects,
            max_batch_size,
        } = ctx;
        let limit = max_batch_size
            .map(|n| (n as usize).min(MAX_REPLAY_BATCH))
            .unwrap_or(MAX_REPLAY_BATCH);

        let rows = read_version_events_after(db, event_seq, limit)?;
        let has_more = rows.len() == limit;
        let mut delivered = Vec::with_capacity(rows.len());
        let mut last_scanned = event_seq;

        for row in &rows {
            last_scanned = row.event_seq;
            if !row_matches_filter(row, subjects) {
                continue;
            }
            if !row_permitted_for_actor(db, actor, row) {
                continue;
            }
            delivered.push(dispatched_event_from_row(row));
        }

        self.advance_last_scanned(db, subscription_id, last_scanned)?;
        if let Some(last) = delivered.last() {
            self.advance_acked(db, subscription_id, &last.cursor, last_scanned)?;
        }

        let next_cursor = delivered.last().map(|ev| {
            CursorEnvelope::encode(
                &ev.cursor,
                subscription_id,
                &checkpoint.subscriber_local_key,
            )
        });

        Ok(ReplayResponse {
            events: delivered,
            has_more,
            next_cursor,
            replay_expired: false,
        })
    }

    /// Drain newly-committed `version_events` rows into the bounded outbound
    /// queue of every live subscriber. Caller drives the loop (e.g. a
    /// post-commit notification from W4-B, or a periodic poll); the
    /// dispatcher never opens its own mutation transaction.
    pub fn dispatch_pending(&self, db: &ActionDb) -> Result<usize, DispatcherError> {
        let mut total_delivered = 0usize;

        // Snapshot subscription_ids so we don't hold the DashMap across the
        // synchronous send.
        let subscription_ids: Vec<String> = self
            .handles
            .iter()
            .map(|entry| entry.key().clone())
            .collect();

        for subscription_id in subscription_ids {
            let Some(handle_ref) = self.handles.get(&subscription_id) else {
                continue;
            };
            let actor = handle_ref.actor.clone();
            let subject_filter = handle_ref.subject_filter.clone();
            let sender = handle_ref.sender.clone();
            let backpressure_tx = handle_ref.backpressure_tx.clone();
            drop(handle_ref);

            let Some(checkpoint) = self.load_checkpoint_by_id(db, &subscription_id)? else {
                continue;
            };
            if checkpoint.replay_required {
                // Hard cap exceeded earlier; subscriber must reconnect with
                // from_cursor before live delivery resumes.
                continue;
            }

            let rows = read_version_events_after(
                db,
                checkpoint.last_scanned_event_seq,
                MAX_REPLAY_BATCH,
            )?;
            if rows.is_empty() {
                continue;
            }

            let mut last_scanned = checkpoint.last_scanned_event_seq;
            let mut last_delivered_cursor: Option<String> = None;
            let mut last_delivered_seq = checkpoint.last_acked_event_seq;
            let mut dropped_in_batch = 0u64;

            for row in rows {
                last_scanned = row.event_seq;

                if !row_matches_filter(&row, &subject_filter) {
                    continue;
                }
                if !row_permitted_for_actor(db, &actor, &row) {
                    continue;
                }

                let event = dispatched_event_from_row(&row);
                let is_reserved = matches!(
                    row.event_kind,
                    VersionEventKind::ClaimWriteRejected | VersionEventKind::ClaimCorrected
                );

                match sender.try_send(event.clone()) {
                    Ok(()) => {
                        last_delivered_cursor = Some(event.cursor.clone());
                        last_delivered_seq = row.event_seq;
                    }
                    Err(mpsc::error::TrySendError::Full(_)) => {
                        if is_reserved {
                            // Reserved-slot semantics (packet §7): block the
                            // dispatch for a synchronous best-effort send
                            // attempt by re-enqueueing into a fresh slot via
                            // a `try_reserve`. SQLite single-process model
                            // means the dispatch loop is not on the W4-B
                            // commit path, so we can wait.
                            if sender.blocking_send(event.clone()).is_ok() {
                                last_delivered_cursor = Some(event.cursor.clone());
                                last_delivered_seq = row.event_seq;
                                continue;
                            }
                        }
                        dropped_in_batch += 1;
                        self.mark_replay_required(db, &subscription_id)?;
                        #[allow(
                            clippy::let_underscore_must_use,
                            reason = "backpressure best-effort: subscriber may already be slow consuming its own backpressure channel"
                        )]
                        let _ = backpressure_tx.try_send(BackpressureEvent {
                            event_kind: BackpressureEvent::KIND,
                            subscription_id: subscription_id.clone(),
                            cursor: last_delivered_cursor.clone(),
                            replay_required: true,
                            replay_from: last_delivered_cursor.clone().map(|uuid| {
                                CursorEnvelope::encode(
                                    &uuid,
                                    &subscription_id,
                                    &checkpoint.subscriber_local_key,
                                )
                            }),
                            dropped_event_count: dropped_in_batch,
                        });
                        // Stop scanning this subscriber; replay required.
                        break;
                    }
                    Err(mpsc::error::TrySendError::Closed(_)) => {
                        // Subscriber dropped its receiver; remove the handle
                        // and leave the durable checkpoint for reconnect.
                        self.handles.remove(&subscription_id);
                        break;
                    }
                }

                {
                    let cap = self.soft_threshold;
                    let len = sender.capacity();
                    if sender.max_capacity().saturating_sub(len) >= cap {
                        #[allow(
                            clippy::let_underscore_must_use,
                            reason = "backpressure best-effort: soft signal is informational; drop if channel is full"
                        )]
                        // Soft threshold crossed: emit a one-shot signal.
                        let _ = backpressure_tx.try_send(BackpressureEvent {
                            event_kind: BackpressureEvent::KIND,
                            subscription_id: subscription_id.clone(),
                            cursor: Some(event.cursor.clone()),
                            replay_required: false,
                            replay_from: None,
                            dropped_event_count: 0,
                        });
                    }
                }

                total_delivered += 1;
            }

            self.advance_last_scanned(db, &subscription_id, last_scanned)?;
            if let Some(cursor) = last_delivered_cursor {
                self.advance_acked(db, &subscription_id, &cursor, last_delivered_seq)?;
            }
        }

        Ok(total_delivered)
    }

    /// Test-only / route-only helper: drop the live handle without touching
    /// the durable checkpoint. Used by session-expiry plumbing (packet §13.5)
    /// and by the Tauri channel close hook.
    pub fn drop_handle(&self, subscription_id: &str) {
        self.handles.remove(subscription_id);
    }

    /// Test introspection: number of live subscribers.
    pub fn live_subscriber_count(&self) -> usize {
        self.handles.len()
    }

    // ---- internal helpers ----

    fn current_subscriber_visible_cursor(
        &self,
        checkpoint: &SubscriptionCheckpoint,
        _filter: &SubjectFilter,
    ) -> Result<Option<String>, DispatcherError> {
        // Subscribe-time current_cursor is the last delivered (acked) cursor
        // per packet §4 V3: returning anything else (e.g. global latest)
        // would leak existence of out-of-scope rows.
        Ok(checkpoint.last_acked_cursor_uuid.as_ref().map(|uuid| {
            CursorEnvelope::encode(
                uuid,
                &checkpoint.subscription_id,
                &checkpoint.subscriber_local_key,
            )
        }))
    }

    fn load_checkpoint_by_id(
        &self,
        db: &ActionDb,
        subscription_id: &str,
    ) -> Result<Option<SubscriptionCheckpoint>, rusqlite::Error> {
        db.conn_ref()
            .query_row(
                "SELECT subscription_id, actor_kind, actor_instance, scopes_digest, \
                        subject_filter_digest, subscriber_local_key, last_acked_event_seq, \
                        last_acked_cursor_uuid, last_scanned_event_seq, replay_required \
                 FROM subscription_checkpoints WHERE subscription_id = ?1",
                params![subscription_id],
                row_to_checkpoint,
            )
            .optional()
    }

    fn load_or_create_checkpoint(
        &self,
        db: &ActionDb,
        identity: CheckpointIdentity<'_>,
    ) -> Result<SubscriptionCheckpoint, DispatcherError> {
        let CheckpointIdentity {
            actor_kind,
            actor_instance,
            scopes_digest: scopes_d,
            subject_filter_digest: filter_d,
            now,
        } = identity;
        let existing = db
            .conn_ref()
            .query_row(
                "SELECT subscription_id, actor_kind, actor_instance, scopes_digest, \
                        subject_filter_digest, subscriber_local_key, last_acked_event_seq, \
                        last_acked_cursor_uuid, last_scanned_event_seq, replay_required \
                 FROM subscription_checkpoints \
                 WHERE actor_kind = ?1 AND actor_instance = ?2 \
                   AND scopes_digest = ?3 AND subject_filter_digest = ?4",
                params![actor_kind, actor_instance, scopes_d, filter_d],
                row_to_checkpoint,
            )
            .optional()?;
        if let Some(checkpoint) = existing {
            return Ok(checkpoint);
        }

        let subscription_id = Uuid::new_v4().to_string();
        let mut local_key = vec![0u8; 32];
        self.rng
            .fill(&mut local_key)
            .map_err(|e| DispatcherError::Internal(format!("rng failed: {e}")))?;

        db.with_transaction(|tx| {
                tx.conn_ref()
                    .execute(
                        "INSERT INTO subscription_checkpoints (\
                            subscription_id, actor_kind, actor_instance, scopes_digest, \
                            subject_filter_digest, subscriber_local_key, \
                            last_acked_event_seq, last_acked_cursor_uuid, \
                            last_scanned_event_seq, replay_required, \
                            created_at, updated_at\
                        ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, 0, NULL, 0, 0, ?7, ?7)",
                        params![
                            &subscription_id,
                            actor_kind,
                            actor_instance,
                            scopes_d,
                            filter_d,
                            &local_key,
                            now,
                        ],
                    )
                    .map_err(|e| e.to_string())?;
                Ok(())
            })
            .map_err(DispatcherError::Internal)?;

        Ok(SubscriptionCheckpoint {
            subscription_id,
            actor_kind: actor_kind.to_string(),
            actor_instance: actor_instance.to_string(),
            scopes_digest: scopes_d.to_string(),
            subject_filter_digest: filter_d.to_string(),
            subscriber_local_key: local_key,
            last_acked_event_seq: 0,
            last_acked_cursor_uuid: None,
            last_scanned_event_seq: 0,
            replay_required: false,
        })
    }

    fn advance_acked(
        &self,
        db: &ActionDb,
        subscription_id: &str,
        cursor_uuid: &str,
        event_seq: i64,
    ) -> Result<(), DispatcherError> {
        db.with_transaction(|tx| {
                tx.conn_ref()
                    .execute(
                        "UPDATE subscription_checkpoints \
                         SET last_acked_event_seq = ?2, \
                             last_acked_cursor_uuid = ?3, \
                             updated_at = ?4 \
                         WHERE subscription_id = ?1 \
                           AND last_acked_event_seq <= ?2",
                        params![
                            subscription_id,
                            event_seq,
                            cursor_uuid,
                            Utc::now().to_rfc3339()
                        ],
                    )
                    .map_err(|e| e.to_string())?;
                Ok(())
            })
            .map_err(DispatcherError::Internal)
    }

    fn advance_last_scanned(
        &self,
        db: &ActionDb,
        subscription_id: &str,
        event_seq: i64,
    ) -> Result<(), DispatcherError> {
        db.with_transaction(|tx| {
                tx.conn_ref()
                    .execute(
                        "UPDATE subscription_checkpoints \
                         SET last_scanned_event_seq = MAX(last_scanned_event_seq, ?2), \
                             updated_at = ?3 \
                         WHERE subscription_id = ?1",
                        params![subscription_id, event_seq, Utc::now().to_rfc3339()],
                    )
                    .map_err(|e| e.to_string())?;
                Ok(())
            })
            .map_err(DispatcherError::Internal)
    }

    fn mark_replay_required(
        &self,
        db: &ActionDb,
        subscription_id: &str,
    ) -> Result<(), DispatcherError> {
        db.with_transaction(|tx| {
                tx.conn_ref()
                    .execute(
                        "UPDATE subscription_checkpoints \
                         SET replay_required = 1, updated_at = ?2 \
                         WHERE subscription_id = ?1",
                        params![subscription_id, Utc::now().to_rfc3339()],
                    )
                    .map_err(|e| e.to_string())?;
                Ok(())
            })
            .map_err(DispatcherError::Internal)
    }
}

fn row_to_checkpoint(row: &rusqlite::Row<'_>) -> rusqlite::Result<SubscriptionCheckpoint> {
    Ok(SubscriptionCheckpoint {
        subscription_id: row.get(0)?,
        actor_kind: row.get(1)?,
        actor_instance: row.get(2)?,
        scopes_digest: row.get(3)?,
        subject_filter_digest: row.get(4)?,
        subscriber_local_key: row.get(5)?,
        last_acked_event_seq: row.get(6)?,
        last_acked_cursor_uuid: row.get(7)?,
        last_scanned_event_seq: row.get(8)?,
        replay_required: row.get::<_, i64>(9)? != 0,
    })
}

fn row_matches_filter(row: &VersionEventRow, filter: &SubjectFilter) -> bool {
    if filter.is_empty() {
        return true;
    }
    if let Some(claim_id) = &row.claim_id {
        return filter.matches_claim(claim_id);
    }
    if let Some(composition_id) = &row.composition_id {
        return filter.matches_composition(composition_id);
    }
    false
}

fn row_permitted_for_actor(db: &ActionDb, actor: &Actor, row: &VersionEventRow) -> bool {
    // Per-row scope re-resolution (packet §4 V2). The predicate is called for
    // every delivery; a subscriber whose session is invalidated between
    // events will fail this call from the next row forward.
    if let Some(claim_id) = &row.claim_id {
        return scope_permits_claim_read(db, actor, claim_id);
    }
    if let Some(composition_id) = &row.composition_id {
        return scope_permits_composition_read(db, actor, composition_id);
    }
    false
}

/// Constant-time check that the requesting actor matches the stored
/// checkpoint owner. Compares actor_kind, actor_instance, and the canonical
/// scopes_digest. SurfaceClient-only actors carry the required identity
/// fields; other actor kinds fall back to instance-string parity but the
/// dispatcher only registers SurfaceClient subscriptions today.
fn actor_matches_checkpoint(actor: &Actor, checkpoint: &SubscriptionCheckpoint) -> bool {
    let actor_kind = actor_kind_str(actor);
    if !constant_time_eq(actor_kind, &checkpoint.actor_kind) {
        return false;
    }
    let actor_instance = actor_instance_str(actor);
    if !constant_time_eq(&actor_instance, &checkpoint.actor_instance) {
        return false;
    }
    match actor {
        Actor::SurfaceClient { scopes, .. } => {
            let digest = scopes_digest(scopes);
            constant_time_eq(&digest, &checkpoint.scopes_digest)
        }
        _ => true,
    }
}

fn replay_expired_response() -> ReplayResponse {
    ReplayResponse {
        events: vec![],
        has_more: false,
        next_cursor: None,
        replay_expired: true,
    }
}

fn dispatched_event_from_row(row: &VersionEventRow) -> DispatchedEvent {
    DispatchedEvent {
        cursor: row.cursor.clone(),
        event_kind: row.event_kind.as_str().to_string(),
        claim_id: row.claim_id.clone(),
        composition_id: row.composition_id.clone(),
        previous_version: row.previous_version,
        current_version: row.current_version,
        reason: row.reason.clone(),
        correction_event_log_id: row.correction_event_log_id.clone(),
        actor_kind: row.actor_kind.clone(),
        created_at: row.created_at.clone(),
    }
}

// ---------------------------------------------------------------------------
// Test introspection helpers (cargo test only)
// ---------------------------------------------------------------------------
//
// Marked `#[doc(hidden)]` to keep them off the public API docs. Reaching
// these requires an `ActionDb` handle, which is itself behind the
// substrate trust boundary — no remote route can call them. The tighter
// `#[cfg(any(test, feature = "test-harness"))]` gate would require the
// workspace CI invocation to opt into the feature for the dos589
// fixtures; see the linked maintenance ticket for that follow-up.

#[doc(hidden)]
pub fn __test_encode_cursor(cursor_uuid: &str, subscription_id: &str, local_key: &[u8]) -> String {
    CursorEnvelope::encode(cursor_uuid, subscription_id, local_key)
}

#[doc(hidden)]
pub fn __test_decode_cursor_subscription_id(wire: &str) -> Option<String> {
    CursorEnvelope::decode(wire).map(|env| env.subscription_id)
}

/// Test-only: read the per-subscription HMAC key so fixtures can mint a
/// valid cursor envelope without going through a delivered event. Production
/// callers never need this — the dispatcher always encodes envelopes on the
/// caller's behalf.
#[doc(hidden)]
pub fn __test_load_local_key(
    db: &ActionDb,
    subscription_id: &str,
) -> Result<Option<Vec<u8>>, rusqlite::Error> {
    db.conn_ref()
        .query_row(
            "SELECT subscriber_local_key FROM subscription_checkpoints \
             WHERE subscription_id = ?1",
            params![subscription_id],
            |row| row.get::<_, Vec<u8>>(0),
        )
        .optional()
}

