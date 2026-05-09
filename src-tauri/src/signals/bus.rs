//! Signal event CRUD and source tier weights (ADR-0080).
//!
//! ## Signal Taxonomy
//!
//! User-facing actions emit these signal types:
//!
//! | Signal Type              | Source           | Weight Change       | Trigger                |
//! |--------------------------|------------------|---------------------|------------------------|
//! | `intelligence_confirmed` | `user_feedback`  | alpha += 1          | Thumbs up        |
//! | `intelligence_rejected`  | `user_feedback`  | beta  += 1          | Thumbs down      |
//! | `user_correction`        | `user_edit`      | beta  += 1          | Edit intelligence field |
//! | `intelligence_curated`   | `user_curation`  | (no weight change)  | Delete / remove item   |
//! | `email_signal_dismissed` | `user_correction`| (no weight change)  | Dismiss email signal   |
//! | `email_item_dismissed`   | (item_type)      | (no weight change)  | Dismiss email item     |
//!
//! Corrections (edit, thumbs-down) penalize the wrong source. Curation (delete,
//! dismiss) records user preference without penalizing—the AI wasn't necessarily
//! wrong, the user just doesn't need that item.

use std::collections::{HashMap, VecDeque};
use std::sync::OnceLock;
use std::time::{Duration, Instant};

use chrono::Utc;
use parking_lot::Mutex;
use rusqlite::params;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::db::{ActionDb, DbError};

use super::policy_registry::{
    policy_for, CoalescingPolicy, PropagationPolicy, SignalEmissionChannel, SignalType,
};

const COALESCING_STATE_MAX_KEYS: usize = 4_096;
const COALESCING_STATE_PRUNE_AFTER: Duration = Duration::from_secs(60);
const CLAIM_RATE_LIMIT_PER_MINUTE: usize = 50;
const CLAIM_RATE_LIMIT_WINDOW: Duration = Duration::from_secs(60);
const CLAIM_ADAPTIVE_COALESCE_WINDOW: Duration = Duration::from_secs(60);

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

/// A row from the `signal_events` table.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SignalEvent {
    pub id: String,
    pub entity_type: String,
    pub entity_id: String,
    pub signal_type: String,
    pub source: String,
    pub value: Option<String>,
    pub confidence: f64,
    pub decay_half_life_days: i32,
    pub created_at: String,
    pub superseded_by: Option<String>,
    /// Context tag for the signal source (e.g. "inbound_email", "outbound_email").
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_context: Option<String>,
}

#[derive(Debug, Clone, Copy)]
enum SignalInsertMode {
    Insert,
    InsertOrReplace,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SignalEmitOutcome {
    pub id: String,
    pub coalesced: bool,
}

#[derive(Debug)]
struct EmitSignalEventOutcome {
    event: SignalEvent,
    coalesced: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct CoalescingKey {
    signal_type: String,
    entity_id: String,
}

impl CoalescingKey {
    fn new(signal_type: &SignalType, entity_id: &str) -> Self {
        Self {
            signal_type: signal_type.canonical_name().to_string(),
            entity_id: entity_id.to_string(),
        }
    }
}

#[derive(Debug, Clone)]
struct CoalescingEntry {
    signal_id: String,
    emitted_at: Instant,
}

#[derive(Debug, Default)]
pub(crate) struct CoalescingState {
    recent: HashMap<CoalescingKey, CoalescingEntry>,
    order: VecDeque<(CoalescingKey, Instant)>,
    claim_attempts_by_entity: HashMap<String, VecDeque<Instant>>,
    adaptive_claim_entities: HashMap<String, Instant>,
}

impl CoalescingState {
    fn coalesced_signal_id(
        &mut self,
        key: &CoalescingKey,
        is_claim_rate_limited: bool,
        now: Instant,
        base_window: Duration,
    ) -> Option<String> {
        self.prune(now);
        let adaptive = is_claim_rate_limited && self.observe_claim_attempt(&key.entity_id, now);
        let window = if adaptive && CLAIM_ADAPTIVE_COALESCE_WINDOW > base_window {
            CLAIM_ADAPTIVE_COALESCE_WINDOW
        } else {
            base_window
        };

        self.recent.get(key).and_then(|entry| {
            (now.duration_since(entry.emitted_at) <= window).then(|| entry.signal_id.clone())
        })
    }

    fn record_emit(&mut self, key: CoalescingKey, signal_id: String, now: Instant) {
        self.recent.insert(
            key.clone(),
            CoalescingEntry {
                signal_id,
                emitted_at: now,
            },
        );
        self.order.push_back((key, now));
        self.prune(now);
    }

    fn observe_claim_attempt(&mut self, entity_id: &str, now: Instant) -> bool {
        let attempts = self
            .claim_attempts_by_entity
            .entry(entity_id.to_string())
            .or_default();
        prune_instants(attempts, now, CLAIM_RATE_LIMIT_WINDOW);
        attempts.push_back(now);

        if attempts.len() > CLAIM_RATE_LIMIT_PER_MINUTE {
            self.adaptive_claim_entities
                .insert(entity_id.to_string(), now + CLAIM_RATE_LIMIT_WINDOW);
            return true;
        }

        self.adaptive_claim_entities
            .get(entity_id)
            .is_some_and(|expires_at| *expires_at > now)
    }

    fn prune(&mut self, now: Instant) {
        while let Some((_, emitted_at)) = self.order.front() {
            let expired = now.duration_since(*emitted_at) > COALESCING_STATE_PRUNE_AFTER;
            let over_capacity = self.order.len() > COALESCING_STATE_MAX_KEYS;
            if !expired && !over_capacity {
                break;
            }

            let (key, emitted_at) = self.order.pop_front().expect("front exists");
            if self
                .recent
                .get(&key)
                .is_some_and(|entry| entry.emitted_at == emitted_at)
            {
                self.recent.remove(&key);
            }
        }

        self.claim_attempts_by_entity.retain(|_, attempts| {
            prune_instants(attempts, now, CLAIM_RATE_LIMIT_WINDOW);
            !attempts.is_empty()
        });
        self.adaptive_claim_entities
            .retain(|_, expires_at| *expires_at > now);
    }
}

fn prune_instants(instants: &mut VecDeque<Instant>, now: Instant, window: Duration) {
    while instants
        .front()
        .is_some_and(|instant| now.duration_since(*instant) > window)
    {
        instants.pop_front();
    }
}

static COALESCING_STATE: OnceLock<Mutex<CoalescingState>> = OnceLock::new();

fn coalescing_state() -> &'static Mutex<CoalescingState> {
    COALESCING_STATE.get_or_init(|| Mutex::new(CoalescingState::default()))
}

// ---------------------------------------------------------------------------
// Source tier weights (ADR-0080)
// ---------------------------------------------------------------------------

/// Base reliability weight for a signal source.
///
/// Tier 1 (highest): user corrections, transcript evidence
/// Tier 2: attendee patterns, email threads
/// Tier 3: third-party enrichment (Clay, Gravatar)
/// Tier 4 (lowest): keyword heuristics, AI inference
pub fn source_base_weight(source: &str) -> f64 {
    match source {
        "user_correction" | "user_feedback" | "explicit" => 1.0,
        "user_curation" => 0.9, // I530: curation signals — no weight penalty but high trust
        "transcript" | "notes" => 0.9,
        "attendee" | "attendee_vote" | "email_thread" | "junction" => 0.8,
        "group_pattern" => 0.75,
        "proactive" => 0.7,
        // /ADR-0100: Tiered Glean source confidence
        "glean_crm" | "glean_REDACTED" => 0.9, // REDACTED — system of record
        "glean_zendesk" | "glean_support" => 0.85, // Zendesk — ticket data is factual
        "glean_gong" => 0.8,                   // Gong — recorded calls, AI summaries synthesized
        "glean" | "glean_search" | "glean_org" => 0.7,
        "glean_chat" | "glean_synthesis" => 0.7, // Glean AI synthesis — same tier as PTY
        "glean_slack" => 0.5,                    // Slack — context signal, noisy
        "clay" | "gravatar" => 0.6,
        "keyword" | "keyword_fuzzy" | "heuristic" | "embedding" => 0.4,
        _ => 0.5,
    }
}

/// Default half-life in days for a signal source.
pub fn default_half_life(source: &str) -> i32 {
    match source {
        "user_correction" | "user_feedback" | "explicit" => 365,
        "user_curation" => 180, // I530: curation decays faster than corrections
        "transcript" | "notes" => 60,
        "attendee" | "attendee_vote" | "junction" => 30,
        "group_pattern" => 60,
        "proactive" => 3,
        // /ADR-0100: Tiered Glean half-lives
        "glean_crm" | "glean_REDACTED" => 90, // CRM data refreshes on enrichment cycle
        "glean_zendesk" | "glean_support" => 30, // Support health is dynamic
        "glean_gong" => 60,                   // Call patterns are stable-ish
        "glean" | "glean_search" | "glean_org" => 60,
        "glean_chat" | "glean_synthesis" => 60, // AI synthesis stable
        "glean_slack" => 14,                    // Slack context decays fast
        "clay" | "gravatar" => 90,
        "keyword" | "keyword_fuzzy" | "heuristic" | "embedding" => 7,
        _ => 30,
    }
}

// ---------------------------------------------------------------------------
// Builder struct for signal emission (ADR-0080 cleanup)
// ---------------------------------------------------------------------------

/// Parameters for inserting a signal event row into the DB.
#[derive(Debug)]
struct InsertSignalRow<'a> {
    pub id: &'a str,
    pub entity_type: &'a str,
    pub entity_id: &'a str,
    pub signal_type: &'a str,
    pub source: &'a str,
    pub value: Option<&'a str>,
    pub confidence: f64,
    pub decay_half_life_days: i32,
    pub created_at: &'a str,
    pub source_context: Option<&'a str>,
}

/// A structured parameter object for emitting signals, replacing long
/// positional argument lists.
pub struct SignalEmission<'a> {
    pub entity_type: &'a str,
    pub entity_id: &'a str,
    pub signal_type: &'a str,
    pub source: &'a str,
    pub value: Option<&'a str>,
    pub confidence: f64,
    pub source_context: Option<&'a str>,
}

/// Emit a signal using the builder struct. Returns the generated signal ID.
pub fn emit(db: &ActionDb, signal: SignalEmission<'_>) -> Result<String, DbError> {
    let outcome = emit_signal_event(
        db,
        EmitSignalEvent {
            entity_type: signal.entity_type,
            entity_id: signal.entity_id,
            signal_type: signal.signal_type,
            source: signal.source,
            value: signal.value,
            confidence: signal.confidence,
            source_context: signal.source_context,
            id: None,
            created_at: None,
            decay_half_life_days: None,
            insert_mode: SignalInsertMode::Insert,
            channel: SignalEmissionChannel::Infrastructure,
            refresh_meetings: true,
        },
    )?;
    Ok(outcome.event.id)
}

// ---------------------------------------------------------------------------
// Signal event operations
// ---------------------------------------------------------------------------

/// Emit a new signal event. Returns the generated signal ID.
///
/// Prefer [`emit`] with [`SignalEmission`] for new call sites.
pub fn emit_signal(
    db: &ActionDb,
    entity_type: &str,
    entity_id: &str,
    signal_type: &str,
    source: &str,
    value: Option<&str>,
    confidence: f64,
) -> Result<String, DbError> {
    let outcome = emit_signal_event(
        db,
        EmitSignalEvent {
            entity_type,
            entity_id,
            signal_type,
            source,
            value,
            confidence,
            source_context: None,
            id: None,
            created_at: None,
            decay_half_life_days: None,
            insert_mode: SignalInsertMode::Insert,
            channel: SignalEmissionChannel::Infrastructure,
            refresh_meetings: true,
        },
    )?;
    Ok(outcome.event.id)
}

/// Emit a signal row inside the caller's active transaction.
///
/// This helper only appends to `signal_events`; it does not run propagation,
/// evaluation, or meeting refresh side effects. Synchronous derived-state
/// subscribers are invoked by the service facade after this insert succeeds.
pub fn emit_signal_in_active_tx(
    db: &ActionDb,
    entity_type: &str,
    entity_id: &str,
    signal_type: &str,
    source: &str,
    payload: &serde_json::Value,
) -> Result<SignalEmitOutcome, DbError> {
    let value = payload.to_string();
    let outcome = emit_signal_event(
        db,
        EmitSignalEvent {
            entity_type,
            entity_id,
            signal_type,
            source,
            value: Some(&value),
            confidence: 1.0,
            source_context: None,
            id: None,
            created_at: None,
            decay_half_life_days: None,
            insert_mode: SignalInsertMode::Insert,
            channel: SignalEmissionChannel::ActiveTransaction,
            refresh_meetings: false,
        },
    )?;

    Ok(SignalEmitOutcome {
        id: outcome.event.id,
        coalesced: outcome.coalesced,
    })
}

pub(crate) fn emit_signal_derived_in_active_tx(
    db: &ActionDb,
    entity_type: &str,
    entity_id: &str,
    signal_type: &str,
    source: &str,
    value: Option<&str>,
    confidence: f64,
) -> Result<SignalEvent, DbError> {
    emit_signal_event(
        db,
        EmitSignalEvent {
            entity_type,
            entity_id,
            signal_type,
            source,
            value,
            confidence,
            source_context: None,
            id: None,
            created_at: None,
            decay_half_life_days: None,
            insert_mode: SignalInsertMode::Insert,
            channel: SignalEmissionChannel::PropagationDerived,
            refresh_meetings: false,
        },
    )
    .map(|outcome| outcome.event)
}

#[allow(clippy::too_many_arguments)]
pub fn emit_signal_fixture_event(
    db: &ActionDb,
    id: &str,
    entity_type: &str,
    entity_id: &str,
    signal_type: &str,
    source: &str,
    value: Option<&str>,
    confidence: f64,
    decay_half_life_days: Option<i32>,
    created_at: &str,
) -> Result<String, DbError> {
    let outcome = emit_signal_event(
        db,
        EmitSignalEvent {
            entity_type,
            entity_id,
            signal_type,
            source,
            value,
            confidence,
            source_context: None,
            id: Some(id),
            created_at: Some(created_at),
            decay_half_life_days,
            insert_mode: SignalInsertMode::InsertOrReplace,
            channel: SignalEmissionChannel::FixtureSeed,
            refresh_meetings: false,
        },
    )?;
    Ok(outcome.event.id)
}

struct EmitSignalEvent<'a> {
    entity_type: &'a str,
    entity_id: &'a str,
    signal_type: &'a str,
    source: &'a str,
    value: Option<&'a str>,
    confidence: f64,
    source_context: Option<&'a str>,
    id: Option<&'a str>,
    created_at: Option<&'a str>,
    decay_half_life_days: Option<i32>,
    insert_mode: SignalInsertMode,
    channel: SignalEmissionChannel,
    refresh_meetings: bool,
}

fn emit_signal_event(
    db: &ActionDb,
    signal: EmitSignalEvent<'_>,
) -> Result<EmitSignalEventOutcome, DbError> {
    let typed_signal = SignalType::from_name(signal.signal_type);
    let policy = policy_for(&typed_signal);
    if !policy.channel_eligibility.allows(signal.channel) {
        return Err(DbError::InvalidArgument(format!(
            "signal channel {:?} is not eligible for {}",
            signal.channel, signal.signal_type
        )));
    }

    let coalescing_key = CoalescingKey::new(&typed_signal, signal.entity_id);
    let coalescing_window =
        if typed_signal.uses_emit_path_coalescing() && channel_allows_coalescing(signal.channel) {
            propagation_coalescing_window(policy.propagation)
        } else {
            None
        };
    let emitted_at = Instant::now();
    if let Some(window) = coalescing_window {
        if let Some(id) = coalescing_state().lock().coalesced_signal_id(
            &coalescing_key,
            typed_signal.is_claim_rate_limited(),
            emitted_at,
            window,
        ) {
            let event = SignalEvent {
                id,
                entity_type: signal.entity_type.to_string(),
                entity_id: signal.entity_id.to_string(),
                signal_type: signal.signal_type.to_string(),
                source: signal.source.to_string(),
                value: signal.value.map(str::to_string),
                confidence: signal.confidence,
                decay_half_life_days: signal
                    .decay_half_life_days
                    .unwrap_or_else(|| default_half_life(signal.source)),
                created_at: Utc::now().to_rfc3339(),
                superseded_by: None,
                source_context: signal.source_context.map(str::to_string),
            };
            return Ok(EmitSignalEventOutcome {
                event,
                coalesced: true,
            });
        }
    }

    let id = signal
        .id
        .map(str::to_string)
        .unwrap_or_else(|| format!("sig-{}", Uuid::new_v4()));
    let created_at = signal
        .created_at
        .map(str::to_string)
        .unwrap_or_else(|| Utc::now().to_rfc3339());
    let decay_half_life_days = signal
        .decay_half_life_days
        .unwrap_or_else(|| default_half_life(signal.source));

    emit_signal_insert_event_row(
        db,
        &InsertSignalRow {
            id: &id,
            entity_type: signal.entity_type,
            entity_id: signal.entity_id,
            signal_type: signal.signal_type,
            source: signal.source,
            value: signal.value,
            confidence: signal.confidence,
            decay_half_life_days,
            created_at: &created_at,
            source_context: signal.source_context,
        },
        signal.insert_mode,
    )?;

    if coalescing_window.is_some() {
        coalescing_state()
            .lock()
            .record_emit(coalescing_key, id.clone(), emitted_at);
    }

    if signal.refresh_meetings {
        emit_signal_flag_upcoming_meetings(db, signal.entity_type, signal.entity_id);
    }

    Ok(EmitSignalEventOutcome {
        event: SignalEvent {
            id,
            entity_type: signal.entity_type.to_string(),
            entity_id: signal.entity_id.to_string(),
            signal_type: signal.signal_type.to_string(),
            source: signal.source.to_string(),
            value: signal.value.map(str::to_string),
            confidence: signal.confidence,
            decay_half_life_days,
            created_at,
            superseded_by: None,
            source_context: signal.source_context.map(str::to_string),
        },
        coalesced: false,
    })
}

fn propagation_coalescing_window(propagation: PropagationPolicy) -> Option<Duration> {
    let PropagationPolicy::PropagateAsync {
        coalesce: Some(policy),
    } = propagation
    else {
        return None;
    };

    Some(match policy {
        CoalescingPolicy::EntitySignal { window }
        | CoalescingPolicy::SubjectAbilityInput { window }
        | CoalescingPolicy::SourceVersion { window } => window,
    })
}

fn channel_allows_coalescing(channel: SignalEmissionChannel) -> bool {
    matches!(
        channel,
        SignalEmissionChannel::ServiceFacade
            | SignalEmissionChannel::Infrastructure
            | SignalEmissionChannel::ActiveTransaction
    )
}

fn emit_signal_insert_event_row(
    db: &ActionDb,
    row: &InsertSignalRow<'_>,
    mode: SignalInsertMode,
) -> Result<(), DbError> {
    let sql = match mode {
        SignalInsertMode::Insert => {
            "INSERT INTO signal_events
                (id, entity_type, entity_id, signal_type, data_source, value, confidence, decay_half_life_days, created_at, source_context)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)"
        }
        SignalInsertMode::InsertOrReplace => {
            "INSERT OR REPLACE INTO signal_events
                (id, entity_type, entity_id, signal_type, data_source, value, confidence, decay_half_life_days, created_at, source_context)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)"
        }
    };

    db.conn_ref().execute(
        sql,
        params![
            row.id,
            row.entity_type,
            row.entity_id,
            row.signal_type,
            row.source,
            row.value,
            row.confidence,
            row.decay_half_life_days,
            row.created_at,
            row.source_context,
        ],
    )?;
    Ok(())
}

fn emit_signal_flag_upcoming_meetings(db: &ActionDb, entity_type: &str, entity_id: &str) {
    if let Err(e) = db.conn_ref().execute(
        "UPDATE meeting_transcripts SET has_new_signals = 1
         WHERE meeting_id IN (
             SELECT me.meeting_id FROM meeting_entities me
             INNER JOIN meetings m ON m.id = me.meeting_id
             WHERE me.entity_id = ?1 AND me.entity_type = ?2
             AND julianday(m.start_time) > julianday('now')
             AND (meeting_transcripts.intelligence_state IS NULL OR meeting_transcripts.intelligence_state != 'archived')
         )",
        params![entity_id, entity_type],
    ) {
        log::warn!("Failed to flag upcoming meetings for signal refresh: {}", e);
    }
}

/// Emit a signal and run propagation rules, returning the original signal ID
/// and any derived signal IDs.
///
/// Atomic via `db.with_transaction`: the source signal insert, every derived
/// signal insert, every derivation link, and the meeting-fanout writes share
/// a single rollback boundary. Without this wrapper, a propagation failure
/// after the source signal already committed would leave durable orphan
/// signals that retries cannot deduplicate (each retry mints a fresh UUID),
/// which the cycle-11 review flagged as a real partial-state hazard.
///
/// Meeting fanout is best-effort by design — it is a denormalized write to
/// a join table and a failure there is not load-bearing for the source
/// signal. We log but do NOT abort the transaction on meeting-fanout
/// errors, matching the pre-existing behavior; only the upstream
/// emit/propagation errors trigger rollback.
#[allow(clippy::too_many_arguments)]
pub fn emit_signal_and_propagate(
    db: &ActionDb,
    engine: &super::propagation::PropagationEngine,
    entity_type: &str,
    entity_id: &str,
    signal_type: &str,
    source: &str,
    value: Option<&str>,
    confidence: f64,
) -> Result<(String, Vec<String>), DbError> {
    db.with_transaction(|tx_db| {
        let outcome = emit_signal_event(
            tx_db,
            EmitSignalEvent {
                entity_type,
                entity_id,
                signal_type,
                source,
                value,
                confidence,
                source_context: None,
                id: None,
                created_at: None,
                decay_half_life_days: None,
                insert_mode: SignalInsertMode::Insert,
                channel: SignalEmissionChannel::Infrastructure,
                refresh_meetings: true,
            },
        )
        .map_err(|e| e.to_string())?;
        let signal = outcome.event;
        let id = signal.id.clone();
        if outcome.coalesced {
            return Ok((id, Vec::new()));
        }

        let derived_ids = engine
            .propagate(tx_db, &signal)
            .map_err(|e| e.to_string())?;

        if let Err(e) = propagate_signal_to_meetings(tx_db, entity_id) {
            log::warn!("Failed to propagate signal to meetings: {}", e);
        }

        Ok((id, derived_ids))
    })
    .map_err(DbError::Migration)
}

/// Emit a signal, propagate, AND evaluate for self-healing re-enrichment.
///
/// Wrapper around `emit_signal_and_propagate` that additionally checks whether
/// the affected entity should be re-enriched based on its trigger score.
/// Use this from service-layer call sites that have access to the IntelligenceQueue.
#[allow(clippy::too_many_arguments)]
pub fn emit_signal_propagate_and_evaluate(
    db: &ActionDb,
    engine: &super::propagation::PropagationEngine,
    entity_type: &str,
    entity_id: &str,
    signal_type: &str,
    source: &str,
    value: Option<&str>,
    confidence: f64,
    queue: &crate::intel_queue::IntelligenceQueue,
) -> Result<(String, Vec<String>), DbError> {
    let result = emit_signal_and_propagate(
        db,
        engine,
        entity_type,
        entity_id,
        signal_type,
        source,
        value,
        confidence,
    )?;

    // Self-healing: event-driven trigger evaluation
    #[allow(
        clippy::let_underscore_must_use,
        reason = "intentional best-effort discard; preserves existing non-blocking behavior"
    )]
    let _ = crate::self_healing::scheduler::evaluate_on_signal(
        db,
        entity_id,
        entity_type,
        signal_type,
        queue,
    );

    Ok(result)
}

/// When a signal is emitted for an entity, flag all future meetings
/// linked to that entity as having new signals.
pub fn propagate_signal_to_meetings(db: &ActionDb, entity_id: &str) -> Result<usize, DbError> {
    let conn = db.conn_ref();
    let mut stmt = conn.prepare(
        "SELECT me.meeting_id FROM meeting_entities me
         INNER JOIN meetings m ON m.id = me.meeting_id
         LEFT JOIN meeting_transcripts mt ON mt.meeting_id = m.id
         WHERE me.entity_id = ?1
         AND m.start_time > datetime('now')
         AND (mt.intelligence_state IS NULL OR mt.intelligence_state != 'archived')",
    )?;

    let meeting_ids: Vec<String> = stmt
        .query_map(params![entity_id], |row| row.get(0))?
        .filter_map(|r| r.ok())
        .collect();

    let count = meeting_ids.len();
    for meeting_id in &meeting_ids {
        db.mark_meeting_new_signals(meeting_id)?;
    }

    if count > 0 {
        log::info!(
            "Propagated signal for entity {} to {} future meeting(s)",
            entity_id,
            count
        );
    }

    Ok(count)
}

/// Mark an old signal as superseded by a new one.
pub fn supersede_signal(db: &ActionDb, old_id: &str, new_id: &str) -> Result<(), DbError> {
    db.update_signal_superseded(old_id, new_id)
}

/// Get all active (non-superseded) signals for an entity.
pub fn get_active_signals(
    db: &ActionDb,
    entity_type: &str,
    entity_id: &str,
) -> Result<Vec<SignalEvent>, DbError> {
    db.get_signal_events(entity_type, entity_id, None)
}

/// Get active signals filtered by signal_type.
pub fn get_active_signals_by_type(
    db: &ActionDb,
    entity_type: &str,
    entity_id: &str,
    signal_type: &str,
) -> Result<Vec<SignalEvent>, DbError> {
    db.get_signal_events(entity_type, entity_id, Some(signal_type))
}

/// Read the learned reliability for a source from the signal_weights table.
///
/// When the system has enough data (>= 5 updates), uses Thompson Sampling
/// to explore/exploit weight learning. Otherwise returns 0.5 (uninformative prior).
pub fn get_learned_reliability(
    db: &ActionDb,
    source: &str,
    entity_type: &str,
    signal_type: &str,
) -> f64 {
    match db.get_signal_weight(source, entity_type, signal_type) {
        Ok(Some((alpha, beta, update_count))) if update_count >= 5 => {
            super::sampling::sample_reliability(alpha, beta)
        }
        Ok(Some(_)) => 0.5,
        _ => 0.5,
    }
}

// ---------------------------------------------------------------------------
// ActionDb methods
// ---------------------------------------------------------------------------

impl ActionDb {
    /// Map a row from `signal_events` to a `SignalEvent`.
    ///
    /// Expected column order:
    /// `id, entity_type, entity_id, signal_type, data_source, value,
    ///  confidence, decay_half_life_days, created_at, superseded_by, source_context`
    pub fn map_signal_event_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<SignalEvent> {
        Ok(SignalEvent {
            id: row.get(0)?,
            entity_type: row.get(1)?,
            entity_id: row.get(2)?,
            signal_type: row.get(3)?,
            source: row.get(4)?,
            value: row.get(5)?,
            confidence: row.get(6)?,
            decay_half_life_days: row.get(7)?,
            created_at: row.get(8)?,
            superseded_by: row.get(9)?,
            source_context: row.get(10)?,
        })
    }

    /// Query non-superseded signal events for an entity, optionally filtered by signal_type.
    pub fn get_signal_events(
        &self,
        entity_type: &str,
        entity_id: &str,
        signal_type: Option<&str>,
    ) -> Result<Vec<SignalEvent>, DbError> {
        let (sql, params_vec): (&str, Vec<Box<dyn rusqlite::types::ToSql>>) = match signal_type {
            Some(st) => (
                "SELECT id, entity_type, entity_id, signal_type, data_source, value,
                        confidence, decay_half_life_days, created_at, superseded_by,
                        source_context
                 FROM signal_events
                 WHERE entity_type = ?1 AND entity_id = ?2 AND signal_type = ?3
                   AND superseded_by IS NULL
                 ORDER BY created_at DESC",
                vec![
                    Box::new(entity_type.to_string()),
                    Box::new(entity_id.to_string()),
                    Box::new(st.to_string()),
                ],
            ),
            None => (
                "SELECT id, entity_type, entity_id, signal_type, data_source, value,
                        confidence, decay_half_life_days, created_at, superseded_by,
                        source_context
                 FROM signal_events
                 WHERE entity_type = ?1 AND entity_id = ?2
                   AND superseded_by IS NULL
                 ORDER BY created_at DESC",
                vec![
                    Box::new(entity_type.to_string()),
                    Box::new(entity_id.to_string()),
                ],
            ),
        };

        let mut stmt = self.conn_ref().prepare(sql)?;
        let param_refs: Vec<&dyn rusqlite::types::ToSql> =
            params_vec.iter().map(|p| p.as_ref()).collect();
        let rows = stmt.query_map(param_refs.as_slice(), Self::map_signal_event_row)?;

        let mut events = Vec::new();
        for row in rows {
            events.push(row?);
        }
        Ok(events)
    }

    /// Mark a signal as superseded.
    pub fn update_signal_superseded(&self, old_id: &str, new_id: &str) -> Result<(), DbError> {
        self.conn_ref().execute(
            "UPDATE signal_events SET superseded_by = ?2 WHERE id = ?1",
            params![old_id, new_id],
        )?;
        Ok(())
    }

    /// Read a signal_weight row. Returns (alpha, beta, update_count) or None if no row.
    pub fn get_signal_weight(
        &self,
        source: &str,
        entity_type: &str,
        signal_type: &str,
    ) -> Result<Option<(f64, f64, i32)>, DbError> {
        match self.conn_ref().query_row(
            "SELECT alpha, beta, update_count FROM signal_weights
             WHERE source = ?1 AND entity_type = ?2 AND signal_type = ?3",
            params![source, entity_type, signal_type],
            |row| {
                Ok((
                    row.get::<_, f64>(0)?,
                    row.get::<_, f64>(1)?,
                    row.get::<_, i32>(2)?,
                ))
            },
        ) {
            Ok(triple) => Ok(Some(triple)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(DbError::Sqlite(e)),
        }
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::test_utils::test_db;

    #[test]
    fn test_source_base_weights() {
        assert_eq!(source_base_weight("user_correction"), 1.0);
        assert_eq!(source_base_weight("user_feedback"), 1.0); // I529
        assert_eq!(source_base_weight("user_curation"), 0.9); // I530
        assert_eq!(source_base_weight("transcript"), 0.9);
        assert_eq!(source_base_weight("attendee_vote"), 0.8);
        assert_eq!(source_base_weight("clay"), 0.6);
        assert_eq!(source_base_weight("keyword"), 0.4);
        assert_eq!(source_base_weight("unknown"), 0.5);
    }

    #[test]
    fn test_default_half_lives() {
        assert_eq!(default_half_life("user_correction"), 365);
        assert_eq!(default_half_life("user_feedback"), 365); // I529
        assert_eq!(default_half_life("user_curation"), 180); // I530
        assert_eq!(default_half_life("transcript"), 60);
        assert_eq!(default_half_life("clay"), 90);
        assert_eq!(default_half_life("heuristic"), 7);
    }

    #[test]
    fn test_emit_and_get_signals() {
        let db = test_db();
        let id = emit_signal(
            &db,
            "account",
            "acme-1",
            "entity_resolution",
            "keyword",
            Some("name match"),
            0.8,
        )
        .expect("emit");
        assert!(id.starts_with("sig-"));

        let signals = get_active_signals(&db, "account", "acme-1").expect("get");
        assert_eq!(signals.len(), 1);
        assert_eq!(signals[0].signal_type, "entity_resolution");
        assert_eq!(signals[0].source, "keyword");
        assert!((signals[0].confidence - 0.8).abs() < 0.01);
    }

    #[test]
    fn test_supersede_excludes_old() {
        let db = test_db();
        let old_id = emit_signal(
            &db,
            "person",
            "p1",
            "read_model_materialized",
            "clay",
            None,
            0.7,
        )
        .expect("emit old");
        let new_id = emit_signal(
            &db,
            "person",
            "p1",
            "read_model_materialized",
            "clay",
            None,
            0.85,
        )
        .expect("emit new");

        supersede_signal(&db, &old_id, &new_id).expect("supersede");

        let active = get_active_signals(&db, "person", "p1").expect("get");
        assert_eq!(active.len(), 1);
        assert_eq!(active[0].id, new_id);
    }

    #[test]
    fn dos237_coalesces_duplicate_entity_signals_inside_window() {
        *coalescing_state().lock() = CoalescingState::default();
        let db = test_db();

        let first = emit_signal(
            &db,
            "account",
            "dos237-coalesce",
            "EntityUpdated",
            "unit_test",
            None,
            1.0,
        )
        .expect("first emit");
        let second = emit_signal(
            &db,
            "account",
            "dos237-coalesce",
            "EntityUpdated",
            "unit_test",
            None,
            1.0,
        )
        .expect("second emit");

        assert_eq!(second, first);
        let count: i64 = db
            .conn_ref()
            .query_row(
                "SELECT count(*) FROM signal_events WHERE entity_id = 'dos237-coalesce'",
                [],
                |row| row.get(0),
            )
            .expect("count signal rows");
        assert_eq!(count, 1);
    }

    #[test]
    fn dos237_claim_rate_limit_extends_coalescing_window() {
        let mut state = CoalescingState::default();
        let key = CoalescingKey {
            signal_type: "claim_trust_changed".to_string(),
            entity_id: "dos237-rate".to_string(),
        };
        let start = Instant::now();
        state.record_emit(key.clone(), "sig-first".to_string(), start);

        for offset in 1..=CLAIM_RATE_LIMIT_PER_MINUTE {
            assert_eq!(
                state.coalesced_signal_id(
                    &key,
                    true,
                    start + Duration::from_millis(600 * offset as u64),
                    Duration::from_millis(500),
                ),
                None
            );
        }

        assert_eq!(
            state.coalesced_signal_id(
                &key,
                true,
                start + Duration::from_millis(600 * (CLAIM_RATE_LIMIT_PER_MINUTE as u64 + 1)),
                Duration::from_millis(500),
            ),
            Some("sig-first".to_string())
        );
    }

    #[test]
    fn test_get_signals_by_type() {
        let db = test_db();
        emit_signal(
            &db,
            "account",
            "a1",
            "entity_resolution",
            "keyword",
            None,
            0.8,
        )
        .expect("emit 1");
        emit_signal(
            &db,
            "account",
            "a1",
            "pre_meeting_context",
            "email_thread",
            None,
            0.7,
        )
        .expect("emit 2");

        let resolution_only =
            get_active_signals_by_type(&db, "account", "a1", "entity_resolution").expect("get");
        assert_eq!(resolution_only.len(), 1);
        assert_eq!(resolution_only[0].signal_type, "entity_resolution");
    }

    #[test]
    fn test_learned_reliability_default() {
        let db = test_db();
        let reliability = get_learned_reliability(&db, "clay", "person", "profile_update");
        assert!(
            (reliability - 0.5).abs() < 0.01,
            "uninformative prior should be 0.5"
        );
    }
}
