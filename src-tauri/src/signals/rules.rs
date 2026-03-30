//! Cross-entity propagation rules (I308 — ADR-0080 Phase 4).
//!
//! Each rule function takes a source `SignalEvent` and DB access, and returns
//! zero or more `DerivedSignal`s to emit on related entities.

use crate::db::ActionDb;
use crate::entity::EntityType;

use super::bus::SignalEvent;
use super::propagation::DerivedSignal;

// ---------------------------------------------------------------------------
// Rule: Person job change → Account stakeholder_change
// ---------------------------------------------------------------------------

/// When a person's title or company changes (Clay enrichment), emit
/// `stakeholder_change` on each linked account.
pub fn rule_person_job_change(signal: &SignalEvent, db: &ActionDb) -> Vec<DerivedSignal> {
    if signal.entity_type != "person" {
        return Vec::new();
    }
    if signal.signal_type != "title_change" && signal.signal_type != "company_change" {
        return Vec::new();
    }

    let entities = match db.get_entities_for_person(&signal.entity_id) {
        Ok(e) => e,
        Err(_) => return Vec::new(),
    };

    entities
        .into_iter()
        .filter(|e| matches!(e.entity_type, EntityType::Account))
        .map(|e| {
            let value = serde_json::json!({
                "person_id": signal.entity_id,
                "change_type": signal.signal_type,
                "source_signal": signal.id,
                "detail": signal.value,
            })
            .to_string();

            DerivedSignal {
                entity_type: "account".to_string(),
                entity_id: e.id,
                signal_type: "stakeholder_change".to_string(),
                source: "propagation".to_string(),
                value: Some(value),
                confidence: 0.85,
            }
        })
        .collect()
}

// ---------------------------------------------------------------------------
// Rule: Meeting frequency drop → Account engagement_warning
// ---------------------------------------------------------------------------

/// When an account's meeting frequency signal indicates a >50% drop,
/// emit `engagement_warning`.
///
/// I555: Re-activated — `services::meetings::process_transcript` now emits
/// `meeting_frequency` signals after each transcript is processed.
/// Re-registered in `default_engine()`.
pub fn rule_meeting_frequency_drop(signal: &SignalEvent, _db: &ActionDb) -> Vec<DerivedSignal> {
    if signal.entity_type != "account" || signal.signal_type != "meeting_frequency" {
        return Vec::new();
    }

    // Parse the value JSON for current/previous counts
    let value_json: serde_json::Value = match signal
        .value
        .as_deref()
        .and_then(|v| serde_json::from_str(v).ok())
    {
        Some(v) => v,
        None => return Vec::new(),
    };

    let current = value_json
        .get("current_count")
        .and_then(|v| v.as_f64())
        .unwrap_or(0.0);
    let previous = value_json
        .get("previous_count")
        .and_then(|v| v.as_f64())
        .unwrap_or(0.0);

    if previous <= 0.0 || current >= previous * 0.5 {
        return Vec::new();
    }

    let drop_pct = ((previous - current) / previous * 100.0).round();
    let value = serde_json::json!({
        "source_signal": signal.id,
        "current_count": current,
        "previous_count": previous,
        "drop_percentage": drop_pct,
    })
    .to_string();

    vec![DerivedSignal {
        entity_type: "account".to_string(),
        entity_id: signal.entity_id.clone(),
        signal_type: "engagement_warning".to_string(),
        source: "propagation".to_string(),
        value: Some(value),
        confidence: 0.75,
    }]
}

// ---------------------------------------------------------------------------
// Rule: Overdue actions threshold → project_health_warning
// ---------------------------------------------------------------------------

/// When an entity has a `proactive_action_cluster` signal (emitted by the
/// proactive detector for accounts/projects with many pending+overdue actions),
/// check if ≥3 overdue actions exist. If so, emit `project_health_warning`.
pub fn rule_overdue_actions(signal: &SignalEvent, db: &ActionDb) -> Vec<DerivedSignal> {
    if signal.signal_type != "proactive_action_cluster" {
        return Vec::new();
    }

    let count = match db.count_overdue_actions_for_entity(&signal.entity_type, &signal.entity_id) {
        Ok(c) => c,
        Err(_) => return Vec::new(),
    };

    if count < 3 {
        return Vec::new();
    }

    let value = serde_json::json!({
        "source_signal": signal.id,
        "overdue_count": count,
    })
    .to_string();

    vec![DerivedSignal {
        entity_type: signal.entity_type.clone(),
        entity_id: signal.entity_id.clone(),
        signal_type: "project_health_warning".to_string(),
        source: "propagation".to_string(),
        value: Some(value),
        confidence: 0.70,
    }]
}

// ---------------------------------------------------------------------------
// Rule: Champion negative sentiment → Account champion_risk
// ---------------------------------------------------------------------------

/// When a person has a `negative_sentiment` signal, check if they are a
/// champion on any account. If so, emit `champion_risk` on those accounts.
pub fn rule_champion_sentiment(signal: &SignalEvent, db: &ActionDb) -> Vec<DerivedSignal> {
    if signal.entity_type != "person" || signal.signal_type != "negative_sentiment" {
        return Vec::new();
    }

    let entities = match db.get_entities_for_person(&signal.entity_id) {
        Ok(e) => e,
        Err(_) => return Vec::new(),
    };

    let mut derived = Vec::new();
    for entity in entities
        .iter()
        .filter(|e| matches!(e.entity_type, EntityType::Account))
    {
        // Check if person is champion on this account
        let team = match db.get_account_team(&entity.id) {
            Ok(t) => t,
            Err(_) => continue,
        };

        let is_champion = team
            .iter()
            .any(|m| m.person_id == signal.entity_id && m.role.to_lowercase() == "champion");

        if is_champion {
            let value = serde_json::json!({
                "person_id": signal.entity_id,
                "account_id": entity.id,
                "source_signal": signal.id,
                "detail": signal.value,
            })
            .to_string();

            derived.push(DerivedSignal {
                entity_type: "account".to_string(),
                entity_id: entity.id.clone(),
                signal_type: "champion_risk".to_string(),
                source: "propagation".to_string(),
                value: Some(value),
                confidence: 0.80,
            });
        }
    }

    derived
}

// ---------------------------------------------------------------------------
// Rule: Person departure + renewal ≤90d → Account renewal_risk_escalation
// ---------------------------------------------------------------------------

/// When a person departs or changes company and is a champion on accounts
/// with renewal within 90 days, emit `renewal_risk_escalation`.
pub fn rule_departure_renewal(signal: &SignalEvent, db: &ActionDb) -> Vec<DerivedSignal> {
    if signal.entity_type != "person" {
        return Vec::new();
    }
    // person_departed has no emitter yet (no UI path to mark departure).
    // company_change from the Clay integration covers the real-world departure case.
    if signal.signal_type != "company_change" {
        return Vec::new();
    }

    let entities = match db.get_entities_for_person(&signal.entity_id) {
        Ok(e) => e,
        Err(_) => return Vec::new(),
    };

    let ninety_days_from_now = chrono::Utc::now()
        .checked_add_signed(chrono::Duration::days(90))
        .map(|dt| dt.format("%Y-%m-%d").to_string())
        .unwrap_or_default();
    let today = chrono::Utc::now().format("%Y-%m-%d").to_string();

    let mut derived = Vec::new();
    for entity in entities
        .iter()
        .filter(|e| matches!(e.entity_type, EntityType::Account))
    {
        // Check champion role
        let team = match db.get_account_team(&entity.id) {
            Ok(t) => t,
            Err(_) => continue,
        };

        let is_champion = team
            .iter()
            .any(|m| m.person_id == signal.entity_id && m.role.to_lowercase() == "champion");

        if !is_champion {
            continue;
        }

        // Check renewal within 90 days
        let has_near_renewal = match db.get_account_events(&entity.id) {
            Ok(events) => events.iter().any(|ev| {
                ev.event_type == "renewal"
                    && ev.event_date >= today
                    && ev.event_date <= ninety_days_from_now
            }),
            Err(_) => false,
        };

        if has_near_renewal {
            let value = serde_json::json!({
                "person_id": signal.entity_id,
                "account_id": entity.id,
                "change_type": signal.signal_type,
                "source_signal": signal.id,
                "detail": signal.value,
            })
            .to_string();

            derived.push(DerivedSignal {
                entity_type: "account".to_string(),
                entity_id: entity.id.clone(),
                signal_type: "renewal_risk_escalation".to_string(),
                source: "propagation".to_string(),
                value: Some(value),
                confidence: 0.90,
            });
        }
    }

    derived
}

// ---------------------------------------------------------------------------
// Rule: Renewal proximity + no recent meeting -> renewal_at_risk
// ---------------------------------------------------------------------------

/// When `renewal_proximity` fires for an account, check if there has been
/// no meeting in the last 30 days. If so, derive `renewal_at_risk`.
pub fn rule_renewal_engagement_compound(signal: &SignalEvent, db: &ActionDb) -> Vec<DerivedSignal> {
    if signal.entity_type != "account" || signal.signal_type != "renewal_proximity" {
        return Vec::new();
    }

    let thirty_days_ago = chrono::Utc::now()
        .checked_sub_signed(chrono::Duration::days(30))
        .map(|dt| dt.format("%Y-%m-%dT%H:%M:%S").to_string())
        .unwrap_or_default();
    let now = chrono::Utc::now().format("%Y-%m-%dT%H:%M:%S").to_string();

    let recent_meeting_count = db
        .get_meeting_count_for_account(&signal.entity_id, &thirty_days_ago, &now)
        .unwrap_or(0);

    if recent_meeting_count > 0 {
        return Vec::new();
    }

    let value = serde_json::json!({
        "source_signal": signal.id,
        "account_id": signal.entity_id,
        "days_without_meeting": 30,
        "detail": signal.value,
    })
    .to_string();

    vec![DerivedSignal {
        entity_type: "account".to_string(),
        entity_id: signal.entity_id.clone(),
        signal_type: "renewal_at_risk".to_string(),
        source: "propagation".to_string(),
        value: Some(value),
        confidence: 0.85,
    }]
}

// ---------------------------------------------------------------------------
// Rule: Person signal → Connected persons (network graph, I391)
// ---------------------------------------------------------------------------

/// When a person emits a signal, propagate attenuated copies to connected
/// persons via `person_relationships` edges. Loop prevention: signals that
/// already originated from network propagation are not re-propagated.
pub fn rule_person_network(signal: &SignalEvent, db: &ActionDb) -> Vec<DerivedSignal> {
    if signal.entity_type != "person" {
        return Vec::new();
    }
    // Loop prevention
    if signal.source.contains("propagation:network") {
        return Vec::new();
    }
    // Confidence gate
    if signal.confidence < 0.65 {
        return Vec::new();
    }

    let edges = match db.get_relationships_for_person(&signal.entity_id) {
        Ok(e) => e,
        Err(_) => return Vec::new(),
    };

    let base_multiplier = 0.4_f64;
    edges
        .iter()
        .filter_map(|edge| {
            use crate::db::person_relationships::RelationshipType;
            let type_multiplier: f64 = match edge.relationship_type {
                RelationshipType::Manager | RelationshipType::Partner => 1.0,
                RelationshipType::Mentor => 0.8,
                RelationshipType::Peer | RelationshipType::Ally => 0.7,
                RelationshipType::Collaborator => {
                    if edge.effective_confidence < 0.7 {
                        return None;
                    }
                    0.8
                }
                RelationshipType::IntroducedBy => 0.5,
            };
            let derived_confidence =
                signal.confidence * edge.effective_confidence * base_multiplier * type_multiplier;
            let target_id = if edge.from_person_id == signal.entity_id {
                edge.to_person_id.clone()
            } else {
                edge.from_person_id.clone()
            };
            let value = serde_json::json!({
                "source_signal": signal.id,
                "source_person_id": signal.entity_id,
                "relationship_type": edge.relationship_type.to_string(),
                "edge_confidence": edge.effective_confidence,
            })
            .to_string();
            Some(DerivedSignal {
                entity_type: "person".to_string(),
                entity_id: target_id,
                signal_type: signal.signal_type.clone(),
                source: "propagation:network".to_string(),
                value: Some(value),
                confidence: derived_confidence,
            })
        })
        .collect()
}

// ---------------------------------------------------------------------------
// Rule: Account signal → Parent account (hierarchy up)
// ---------------------------------------------------------------------------

/// When any account emits a signal, propagate it upward to the parent account
/// (if one exists) with attenuated confidence. Loop prevention: signals that
/// originated from hierarchy propagation are not re-propagated.
///
/// I385: 48-hour sibling signal accumulation — if the parent already received
/// signals of the same type from other children in the last 48 hours, fuse
/// them using weighted log-odds combination for a stronger derived confidence.
pub fn rule_hierarchy_up(signal: &SignalEvent, db: &ActionDb) -> Vec<DerivedSignal> {
    if signal.entity_type != "account" && signal.entity_type != "project" {
        return Vec::new();
    }

    // Loop prevention: don't re-propagate hierarchy-derived signals
    if signal.source.contains("propagation:hierarchy") {
        return Vec::new();
    }

    // Resolve parent_id based on entity type
    let parent_id = if signal.entity_type == "account" {
        match db.get_account(&signal.entity_id) {
            Ok(Some(a)) => a.parent_id,
            _ => return Vec::new(),
        }
    } else {
        match db.get_project(&signal.entity_id) {
            Ok(Some(p)) => p.parent_id,
            _ => return Vec::new(),
        }
    };

    let parent_id = match parent_id {
        Some(pid) => pid,
        None => return Vec::new(),
    };

    // B1: 48-hour sibling signal accumulation
    let existing_siblings = db.get_recent_signals_by_type(&parent_id, &signal.signal_type, 48);

    let fused_confidence = if !existing_siblings.is_empty() {
        // Build (confidence, weight) pairs for fusion
        let mut pairs: Vec<(f64, f64)> = existing_siblings
            .iter()
            .map(|s| (s.confidence, super::fusion::compute_signal_weight(db, s)))
            .collect();
        // Add current signal's contribution (attenuated by 0.6)
        let current_weight = super::fusion::compute_signal_weight(db, signal);
        pairs.push((signal.confidence * 0.6, current_weight));
        super::fusion::fuse_confidence(&pairs)
    } else {
        signal.confidence * 0.6
    };

    let child_key = if signal.entity_type == "account" {
        "child_account_id"
    } else {
        "child_project_id"
    };

    let value = serde_json::json!({
        "source_signal": signal.id,
        child_key: signal.entity_id,
        "original_signal_type": signal.signal_type,
        "detail": signal.value,
    })
    .to_string();

    vec![DerivedSignal {
        entity_type: signal.entity_type.clone(),
        entity_id: parent_id,
        signal_type: signal.signal_type.clone(),
        source: "propagation:hierarchy_up".to_string(),
        value: Some(value),
        confidence: fused_confidence,
    }]
}

// ---------------------------------------------------------------------------
// Rule: Account signal → Child accounts (hierarchy down)
// ---------------------------------------------------------------------------

/// When an account emits a high-confidence signal (>= 0.7), propagate it
/// downward to all direct child accounts with attenuated confidence.
/// Direct children only — no recursion to grandchildren.
/// Loop prevention: signals from hierarchy propagation are not re-propagated.
pub fn rule_hierarchy_down(signal: &SignalEvent, db: &ActionDb) -> Vec<DerivedSignal> {
    if signal.entity_type != "account" && signal.entity_type != "project" {
        return Vec::new();
    }

    // Loop prevention: don't re-propagate hierarchy-derived signals
    if signal.source.contains("propagation:hierarchy") {
        return Vec::new();
    }

    // Confidence gate: only fan out for high-confidence signals
    if signal.confidence < 0.7 {
        return Vec::new();
    }

    let parent_key = if signal.entity_type == "account" {
        "parent_account_id"
    } else {
        "parent_project_id"
    };

    // Get children based on entity type
    let child_ids_and_types: Vec<(String, String)> = if signal.entity_type == "account" {
        match db.get_child_accounts(&signal.entity_id) {
            Ok(c) => c
                .into_iter()
                .map(|a| (a.id, "account".to_string()))
                .collect(),
            Err(_) => return Vec::new(),
        }
    } else {
        match db.get_child_projects(&signal.entity_id) {
            Ok(c) => c
                .into_iter()
                .map(|p| (p.id, "project".to_string()))
                .collect(),
            Err(_) => return Vec::new(),
        }
    };

    child_ids_and_types
        .into_iter()
        .map(|(child_id, entity_type)| {
            let value = serde_json::json!({
                "source_signal": signal.id,
                parent_key: signal.entity_id,
                "original_signal_type": signal.signal_type,
                "detail": signal.value,
            })
            .to_string();

            DerivedSignal {
                entity_type,
                entity_id: child_id,
                signal_type: signal.signal_type.clone(),
                source: "propagation:hierarchy_down".to_string(),
                value: Some(value),
                confidence: signal.confidence * 0.5,
            }
        })
        .collect()
}

// ---------------------------------------------------------------------------
// Rule: Person profile_discovered → Account profile_updated
// ---------------------------------------------------------------------------

/// When a person has a `profile_discovered` signal (e.g. from Gravatar),
/// emit a derived `profile_updated` signal on each linked account so that
/// account-level intelligence consumers are notified.
pub fn rule_person_profile_discovered(signal: &SignalEvent, db: &ActionDb) -> Vec<DerivedSignal> {
    if signal.entity_type != "person" || signal.signal_type != "profile_discovered" {
        return Vec::new();
    }

    let entities = match db.get_entities_for_person(&signal.entity_id) {
        Ok(e) => e,
        Err(_) => return Vec::new(),
    };

    entities
        .into_iter()
        .filter(|e| matches!(e.entity_type, EntityType::Account))
        .map(|e| {
            let value = serde_json::json!({
                "person_id": signal.entity_id,
                "source_signal": signal.id,
                "detail": signal.value,
            })
            .to_string();

            DerivedSignal {
                entity_type: "account".to_string(),
                entity_id: e.id,
                signal_type: "profile_updated".to_string(),
                source: "propagation".to_string(),
                value: Some(value),
                confidence: signal.confidence * 0.8,
            }
        })
        .collect()
}

// ---------------------------------------------------------------------------
// ActionDb helper methods for rules
// ---------------------------------------------------------------------------

impl ActionDb {
    /// Get recent signal events for an entity by type within a given time window.
    /// Excludes signals originating from hierarchy propagation (loop prevention).
    pub fn get_recent_signals_by_type(
        &self,
        entity_id: &str,
        signal_type: &str,
        hours: i64,
    ) -> Vec<super::bus::SignalEvent> {
        let sql = "SELECT id, entity_type, entity_id, signal_type, source, value, confidence,
                   decay_half_life_days, created_at, superseded_by, source_context
                   FROM signal_events
                   WHERE entity_id = ?1 AND signal_type = ?2
                   AND created_at > datetime('now', ?3 || ' hours')
                   AND source NOT LIKE '%propagation:hierarchy%'
                   ORDER BY created_at DESC";
        let hours_str = format!("-{}", hours);
        let conn = self.conn_ref();
        let mut stmt = match conn.prepare(sql) {
            Ok(s) => s,
            Err(_) => return Vec::new(),
        };
        let rows = stmt.query_map(
            rusqlite::params![entity_id, signal_type, hours_str],
            |row| {
                Ok(super::bus::SignalEvent {
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
            },
        );
        match rows {
            Ok(r) => r.filter_map(|r| r.ok()).collect(),
            Err(_) => Vec::new(),
        }
    }

    /// Count meetings for an account in a date range.
    pub fn get_meeting_count_for_account(
        &self,
        account_id: &str,
        start: &str,
        end: &str,
    ) -> Result<i32, crate::db::DbError> {
        let count: i32 = self.conn_ref().query_row(
            "SELECT COUNT(*) FROM meetings mh
             JOIN meeting_entities me ON me.meeting_id = mh.id
             WHERE me.entity_id = ?1 AND mh.start_time >= ?2 AND mh.start_time <= ?3",
            rusqlite::params![account_id, start, end],
            |row| row.get(0),
        )?;
        Ok(count)
    }

    /// Count overdue actions for an entity.
    pub fn count_overdue_actions_for_entity(
        &self,
        entity_type: &str,
        entity_id: &str,
    ) -> Result<i32, crate::db::DbError> {
        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
        let id_column = match entity_type {
            "account" => "account_id",
            "project" => "project_id",
            _ => return Ok(0),
        };

        let sql = format!(
            "SELECT COUNT(*) FROM actions
             WHERE {} = ?1 AND status = 'pending' AND due_date IS NOT NULL AND due_date < ?2",
            id_column
        );
        let count: i32 =
            self.conn_ref()
                .query_row(&sql, rusqlite::params![entity_id, today], |row| row.get(0))?;
        Ok(count)
    }
}

// ---------------------------------------------------------------------------
// I353 Phase 2: Signal → Hygiene action rules
// ---------------------------------------------------------------------------

/// Evaluate signal-driven hygiene actions. Called after a signal is emitted.
/// Unlike propagation rules (which emit new signals), hygiene rules trigger
/// direct data maintenance operations and log the results.
pub fn evaluate_hygiene_actions(signal: &SignalEvent, db: &ActionDb) {
    // Rule 1: person_created → targeted duplicate check
    if signal.signal_type == "person_created" && signal.entity_type == "person" {
        if let Some(merge_result) = hygiene_check_person_duplicate(signal, db) {
            let _ = db.log_hygiene_action(
                Some(&signal.id),
                "duplicate_merge",
                &signal.entity_id,
                "person",
                signal.confidence,
                &merge_result,
            );
        }
    }

    // Rule 2: email with sender name → name resolution
    if signal.signal_type == "email_received" && signal.entity_type == "person" {
        if let Some(resolve_result) = hygiene_resolve_person_name(signal, db) {
            let _ = db.log_hygiene_action(
                Some(&signal.id),
                "name_resolved",
                &signal.entity_id,
                "person",
                signal.confidence,
                &resolve_result,
            );
        }
    }

    // Rule 3: meeting entity resolved → co-attendance linking
    if signal.signal_type == "entity_resolved" {
        if let Some(link_result) = hygiene_link_co_attendance(signal, db) {
            let _ = db.log_hygiene_action(
                Some(&signal.id),
                "co_attendance_linked",
                &signal.entity_id,
                &signal.entity_type,
                signal.confidence,
                &link_result,
            );
        }
    }
}

/// Check if a newly created person is a duplicate and auto-merge if high confidence.
fn hygiene_check_person_duplicate(signal: &SignalEvent, db: &ActionDb) -> Option<String> {
    let person = db.get_person(&signal.entity_id).ok()??;
    if person.name.is_empty() || person.name.contains('@') {
        return None;
    }

    // Search for existing people with similar names
    let people = db.get_people(None).ok()?;
    let name_lower = person.name.to_lowercase();

    for other in &people {
        if other.id == person.id || other.archived {
            continue;
        }
        let other_lower = other.name.to_lowercase();
        if other_lower == name_lower {
            // Exact name match — merge the newer into the older
            let (keep_id, merge_id) = if other.first_seen <= person.first_seen {
                (&other.id, &person.id)
            } else {
                (&person.id, &other.id)
            };
            match db.merge_people(keep_id, merge_id) {
                Ok(_) => {
                    log::info!(
                        "I353: auto-merged duplicate person {} into {} (signal: {})",
                        merge_id,
                        keep_id,
                        signal.id
                    );
                    return Some(format!("merged {} into {}", merge_id, keep_id));
                }
                Err(e) => {
                    log::warn!(
                        "I353: auto-merge failed for {} → {}: {}",
                        merge_id,
                        keep_id,
                        e
                    );
                    return Some(format!("merge_failed: {}", e));
                }
            }
        }
    }
    None
}

/// Resolve a person's name from email sender display name.
fn hygiene_resolve_person_name(signal: &SignalEvent, db: &ActionDb) -> Option<String> {
    let person = db.get_person(&signal.entity_id).ok()??;

    // Only resolve if name looks like an email (no display name yet)
    if !person.name.contains('@') && person.name.contains(' ') {
        return None; // Already has a proper name
    }

    // Try to extract name from the signal value (sender display name)
    let sender_name = signal.value.as_deref()?;
    if sender_name.is_empty() || sender_name.contains('@') || !sender_name.contains(' ') {
        return None;
    }

    match db.update_person_name(&signal.entity_id, sender_name) {
        Ok(_) => {
            log::info!(
                "I353: resolved person name '{}' → '{}' (signal: {})",
                person.name,
                sender_name,
                signal.id
            );
            Some(format!("renamed '{}' → '{}'", person.name, sender_name))
        }
        Err(e) => {
            log::warn!("I353: name resolve failed: {}", e);
            Some(format!("resolve_failed: {}", e))
        }
    }
}

/// Link meeting attendees via co-attendance when an entity is resolved for a meeting.
fn hygiene_link_co_attendance(signal: &SignalEvent, db: &ActionDb) -> Option<String> {
    // The signal value should contain the meeting_id
    let meeting_id = signal.value.as_deref()?;

    // Get meeting attendees
    let conn = db.conn_ref();
    let attendees_csv: String = conn
        .query_row(
            "SELECT COALESCE(attendees, '') FROM meetings WHERE id = ?1",
            rusqlite::params![meeting_id],
            |row| row.get(0),
        )
        .ok()?;

    if attendees_csv.is_empty() {
        return None;
    }

    let attendees: Vec<&str> = attendees_csv
        .split(',')
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .collect();
    if attendees.len() < 2 {
        return None;
    }

    // Find people from attendee emails and link them to the resolved entity
    let mut linked = 0;
    for email in &attendees {
        if let Ok(Some(person)) = db.get_person_by_email_or_alias(email) {
            // Link person to the resolved entity if not already linked
            if let Ok(existing) = db.get_entities_for_person(&person.id) {
                let already_linked = existing.iter().any(|e| e.id == signal.entity_id);
                if !already_linked {
                    let _ = db.link_person_to_entity(
                        &person.id,
                        &signal.entity_id,
                        &signal.entity_type,
                    );
                    linked += 1;
                }
            }
        }
    }

    if linked > 0 {
        log::info!(
            "I353: linked {} attendees to entity {} via co-attendance (signal: {})",
            linked,
            signal.entity_id,
            signal.id
        );
        Some(format!("linked {} people", linked))
    } else {
        None
    }
}

// ---------------------------------------------------------------------------
// I535/ADR-0100: Glean-sourced signal propagation rules
// ---------------------------------------------------------------------------

/// When Glean detects an org chart change for an account, propagate
/// `stakeholder_change` on the same account — triggers prep invalidation
/// and downstream effects (same as `rule_person_job_change` for persons).
pub fn rule_glean_org_change(signal: &SignalEvent, _db: &ActionDb) -> Vec<DerivedSignal> {
    if signal.signal_type != "glean_org_change" {
        return Vec::new();
    }

    // Derive a stakeholder_change on the same entity (already an account)
    vec![DerivedSignal {
        entity_type: signal.entity_type.clone(),
        entity_id: signal.entity_id.clone(),
        signal_type: "stakeholder_change".to_string(),
        source: "glean_propagation".to_string(),
        value: signal.value.clone(),
        confidence: signal.confidence * 0.9, // Slight attenuation for derived signal
    }]
}

/// When Glean detects a champion departure, propagate `champion_risk`
/// on the account — surfaces in morning briefing callouts.
pub fn rule_glean_champion_departed(signal: &SignalEvent, _db: &ActionDb) -> Vec<DerivedSignal> {
    if signal.signal_type != "glean_champion_departed" {
        return Vec::new();
    }

    vec![DerivedSignal {
        entity_type: signal.entity_type.clone(),
        entity_id: signal.entity_id.clone(),
        signal_type: "champion_risk".to_string(),
        source: "glean_propagation".to_string(),
        value: signal.value.clone(),
        confidence: signal.confidence * 0.95, // Champion departure is high-confidence
    }]
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::test_utils::test_db;

    fn make_signal(
        entity_type: &str,
        entity_id: &str,
        signal_type: &str,
        value: Option<&str>,
    ) -> SignalEvent {
        SignalEvent {
            id: format!("sig-test-{}", uuid::Uuid::new_v4()),
            entity_type: entity_type.to_string(),
            entity_id: entity_id.to_string(),
            signal_type: signal_type.to_string(),
            source: "clay".to_string(),
            value: value.map(|s| s.to_string()),
            confidence: 0.85,
            decay_half_life_days: 90,
            created_at: chrono::Utc::now().to_rfc3339(),
            superseded_by: None,
            source_context: None,
        }
    }

    #[test]
    fn test_rule_person_job_change_emits_stakeholder_change() {
        let db = test_db();
        let conn = db.conn_ref();

        // Create a person and link to an account
        conn.execute(
            "INSERT INTO people (id, email, name, updated_at) VALUES ('p1', 'alice@acme.com', 'Alice', '2026-01-01')",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO accounts (id, name, updated_at) VALUES ('a1', 'Acme', '2026-01-01')",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO entities (id, name, entity_type, updated_at) VALUES ('a1', 'Acme', 'account', '2026-01-01')",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO account_stakeholders (account_id, person_id) VALUES ('a1', 'p1')",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO account_stakeholder_roles (account_id, person_id, role) VALUES ('a1', 'p1', 'associated')",
            [],
        )
        .unwrap();

        let signal = make_signal(
            "person",
            "p1",
            "title_change",
            Some("{\"new_value\": \"CRO\"}"),
        );
        let derived = rule_person_job_change(&signal, &db);

        assert_eq!(derived.len(), 1);
        assert_eq!(derived[0].entity_type, "account");
        assert_eq!(derived[0].entity_id, "a1");
        assert_eq!(derived[0].signal_type, "stakeholder_change");
        assert!((derived[0].confidence - 0.85).abs() < 0.01);
    }

    #[test]
    fn test_rule_person_job_change_skips_non_person() {
        let db = test_db();
        let signal = make_signal("account", "a1", "title_change", None);
        let derived = rule_person_job_change(&signal, &db);
        assert!(derived.is_empty());
    }

    #[test]
    fn test_rule_meeting_frequency_drop() {
        let db = test_db();
        let value = serde_json::json!({
            "current_count": 2,
            "previous_count": 8,
        })
        .to_string();
        let signal = make_signal("account", "a1", "meeting_frequency", Some(&value));
        let derived = rule_meeting_frequency_drop(&signal, &db);

        assert_eq!(derived.len(), 1);
        assert_eq!(derived[0].signal_type, "engagement_warning");
        assert!((derived[0].confidence - 0.75).abs() < 0.01);
    }

    #[test]
    fn test_rule_meeting_frequency_no_drop() {
        let db = test_db();
        let value = serde_json::json!({
            "current_count": 6,
            "previous_count": 8,
        })
        .to_string();
        let signal = make_signal("account", "a1", "meeting_frequency", Some(&value));
        let derived = rule_meeting_frequency_drop(&signal, &db);
        assert!(
            derived.is_empty(),
            "25% drop should not trigger (need >50%)"
        );
    }

    #[test]
    fn test_rule_overdue_actions_threshold() {
        let db = test_db();
        let conn = db.conn_ref();

        conn.execute(
            "INSERT INTO accounts (id, name, updated_at) VALUES ('a1', 'Acme', '2026-01-01')",
            [],
        )
        .unwrap();

        // Create 3 overdue actions
        for i in 0..3 {
            conn.execute(
                &format!(
                    "INSERT INTO actions (id, title, status, due_date, account_id, created_at, updated_at)
                     VALUES ('act-{}', 'Task {}', 'pending', '2025-01-01', 'a1', '2025-01-01', '2025-01-01')",
                    i, i
                ),
                [],
            )
            .unwrap();
        }

        let signal = make_signal("account", "a1", "proactive_action_cluster", None);
        let derived = rule_overdue_actions(&signal, &db);

        assert_eq!(derived.len(), 1);
        assert_eq!(derived[0].signal_type, "project_health_warning");
    }

    #[test]
    fn test_rule_departure_renewal() {
        let db = test_db();
        let conn = db.conn_ref();

        // Create person, account, champion role, renewal event
        conn.execute(
            "INSERT INTO people (id, email, name, updated_at) VALUES ('p1', 'alice@acme.com', 'Alice', '2026-01-01')",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO accounts (id, name, updated_at) VALUES ('a1', 'Acme', '2026-01-01')",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO entities (id, name, entity_type, updated_at) VALUES ('a1', 'Acme', 'account', '2026-01-01')",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO account_stakeholders (account_id, person_id) VALUES ('a1', 'p1')",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO account_stakeholder_roles (account_id, person_id, role) VALUES ('a1', 'p1', 'champion')",
            [],
        )
        .unwrap();

        // Renewal in 60 days
        let renewal_date = (chrono::Utc::now() + chrono::Duration::days(60))
            .format("%Y-%m-%d")
            .to_string();
        conn.execute(
            "INSERT INTO account_events (account_id, event_type, event_date)
             VALUES ('a1', 'renewal', ?1)",
            rusqlite::params![renewal_date],
        )
        .unwrap();

        let signal = make_signal(
            "person",
            "p1",
            "company_change",
            Some("{\"new_value\": \"NewCo\"}"),
        );
        let derived = rule_departure_renewal(&signal, &db);

        assert_eq!(derived.len(), 1);
        assert_eq!(derived[0].signal_type, "renewal_risk_escalation");
        assert!((derived[0].confidence - 0.90).abs() < 0.01);
    }

    #[test]
    fn test_rule_renewal_engagement_compound_fires_no_meeting() {
        let db = test_db();
        let conn = db.conn_ref();

        conn.execute(
            "INSERT INTO accounts (id, name, updated_at) VALUES ('a1', 'RenewalCo', '2026-01-01')",
            [],
        )
        .unwrap();

        let signal = make_signal(
            "account",
            "a1",
            "renewal_proximity",
            Some("{\"days_until_renewal\": 25}"),
        );
        let derived = rule_renewal_engagement_compound(&signal, &db);

        assert_eq!(derived.len(), 1);
        assert_eq!(derived[0].signal_type, "renewal_at_risk");
        assert!((derived[0].confidence - 0.85).abs() < 0.01);
    }

    #[test]
    fn test_rule_renewal_engagement_compound_skips_with_meeting() {
        let db = test_db();
        let conn = db.conn_ref();

        conn.execute(
            "INSERT INTO accounts (id, name, updated_at) VALUES ('a1', 'ActiveCo', '2026-01-01')",
            [],
        )
        .unwrap();

        // Recent meeting
        conn.execute(
            "INSERT INTO meetings (id, title, meeting_type, start_time, created_at)
             VALUES ('m1', 'Sync', 'external', datetime('now', '-5 days'), datetime('now', '-5 days'))",
            [],
        ).unwrap();
        conn.execute(
            "INSERT OR IGNORE INTO meeting_prep (meeting_id) VALUES ('m1')",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT OR IGNORE INTO meeting_transcripts (meeting_id) VALUES ('m1')",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO meeting_entities (meeting_id, entity_id, entity_type)
             VALUES ('m1', 'a1', 'account')",
            [],
        )
        .unwrap();

        let signal = make_signal("account", "a1", "renewal_proximity", None);
        let derived = rule_renewal_engagement_compound(&signal, &db);
        assert!(
            derived.is_empty(),
            "Should not fire when recent meeting exists"
        );
    }

    // -----------------------------------------------------------------------
    // Hierarchy Up
    // -----------------------------------------------------------------------

    #[test]
    fn test_rule_hierarchy_up_propagates_to_parent() {
        let db = test_db();
        let conn = db.conn_ref();

        conn.execute(
            "INSERT INTO accounts (id, name, updated_at) VALUES ('parent1', 'ParentCo', '2026-01-01')",
            [],
        ).unwrap();
        conn.execute(
            "INSERT INTO accounts (id, name, parent_id, updated_at) VALUES ('child1', 'ChildCo', 'parent1', '2026-01-01')",
            [],
        ).unwrap();

        let signal = make_signal(
            "account",
            "child1",
            "health_change",
            Some("{\"new\": \"red\"}"),
        );
        let derived = rule_hierarchy_up(&signal, &db);

        assert_eq!(derived.len(), 1);
        assert_eq!(derived[0].entity_id, "parent1");
        assert_eq!(derived[0].signal_type, "health_change");
        assert_eq!(derived[0].source, "propagation:hierarchy_up");
        assert!((derived[0].confidence - 0.85 * 0.6).abs() < 0.01);
    }

    #[test]
    fn test_rule_hierarchy_up_no_parent() {
        let db = test_db();
        let conn = db.conn_ref();

        conn.execute(
            "INSERT INTO accounts (id, name, updated_at) VALUES ('top1', 'TopCo', '2026-01-01')",
            [],
        )
        .unwrap();

        let signal = make_signal("account", "top1", "health_change", None);
        let derived = rule_hierarchy_up(&signal, &db);
        assert!(
            derived.is_empty(),
            "Top-level account should not propagate up"
        );
    }

    #[test]
    fn test_rule_hierarchy_up_loop_prevention() {
        let db = test_db();
        let conn = db.conn_ref();

        conn.execute(
            "INSERT INTO accounts (id, name, updated_at) VALUES ('parent1', 'ParentCo', '2026-01-01')",
            [],
        ).unwrap();
        conn.execute(
            "INSERT INTO accounts (id, name, parent_id, updated_at) VALUES ('child1', 'ChildCo', 'parent1', '2026-01-01')",
            [],
        ).unwrap();

        // Signal that already came from hierarchy propagation
        let mut signal = make_signal("account", "child1", "health_change", None);
        signal.source = "propagation:hierarchy_down".to_string();
        let derived = rule_hierarchy_up(&signal, &db);
        assert!(
            derived.is_empty(),
            "Hierarchy-derived signals must not re-propagate"
        );
    }

    #[test]
    fn test_rule_hierarchy_up_skips_non_account() {
        let db = test_db();
        let signal = make_signal("person", "p1", "title_change", None);
        let derived = rule_hierarchy_up(&signal, &db);
        assert!(derived.is_empty());
    }

    // -----------------------------------------------------------------------
    // Hierarchy Down
    // -----------------------------------------------------------------------

    #[test]
    fn test_rule_hierarchy_down_propagates_to_children() {
        let db = test_db();
        let conn = db.conn_ref();

        conn.execute(
            "INSERT INTO accounts (id, name, updated_at) VALUES ('parent1', 'ParentCo', '2026-01-01')",
            [],
        ).unwrap();
        conn.execute(
            "INSERT INTO accounts (id, name, parent_id, updated_at) VALUES ('child1', 'Child1', 'parent1', '2026-01-01')",
            [],
        ).unwrap();
        conn.execute(
            "INSERT INTO accounts (id, name, parent_id, updated_at) VALUES ('child2', 'Child2', 'parent1', '2026-01-01')",
            [],
        ).unwrap();

        let signal = make_signal(
            "account",
            "parent1",
            "strategy_shift",
            Some("{\"detail\": \"pivot\"}"),
        );
        let derived = rule_hierarchy_down(&signal, &db);

        assert_eq!(derived.len(), 2);
        let ids: Vec<&str> = derived.iter().map(|d| d.entity_id.as_str()).collect();
        assert!(ids.contains(&"child1"));
        assert!(ids.contains(&"child2"));
        assert_eq!(derived[0].signal_type, "strategy_shift");
        assert_eq!(derived[0].source, "propagation:hierarchy_down");
        assert!((derived[0].confidence - 0.85 * 0.5).abs() < 0.01);
    }

    #[test]
    fn test_rule_hierarchy_down_no_children() {
        let db = test_db();
        let conn = db.conn_ref();

        conn.execute(
            "INSERT INTO accounts (id, name, updated_at) VALUES ('leaf1', 'LeafCo', '2026-01-01')",
            [],
        )
        .unwrap();

        let signal = make_signal("account", "leaf1", "health_change", None);
        let derived = rule_hierarchy_down(&signal, &db);
        assert!(derived.is_empty(), "Leaf account should not propagate down");
    }

    #[test]
    fn test_rule_hierarchy_down_confidence_gate() {
        let db = test_db();
        let conn = db.conn_ref();

        conn.execute(
            "INSERT INTO accounts (id, name, updated_at) VALUES ('parent1', 'ParentCo', '2026-01-01')",
            [],
        ).unwrap();
        conn.execute(
            "INSERT INTO accounts (id, name, parent_id, updated_at) VALUES ('child1', 'ChildCo', 'parent1', '2026-01-01')",
            [],
        ).unwrap();

        // Low confidence signal (0.5 < 0.7 threshold)
        let mut signal = make_signal("account", "parent1", "health_change", None);
        signal.confidence = 0.5;
        let derived = rule_hierarchy_down(&signal, &db);
        assert!(
            derived.is_empty(),
            "Low-confidence signals should not fan out downward"
        );
    }

    #[test]
    fn test_rule_hierarchy_down_loop_prevention() {
        let db = test_db();
        let conn = db.conn_ref();

        conn.execute(
            "INSERT INTO accounts (id, name, updated_at) VALUES ('parent1', 'ParentCo', '2026-01-01')",
            [],
        ).unwrap();
        conn.execute(
            "INSERT INTO accounts (id, name, parent_id, updated_at) VALUES ('child1', 'ChildCo', 'parent1', '2026-01-01')",
            [],
        ).unwrap();

        let mut signal = make_signal("account", "parent1", "health_change", None);
        signal.source = "propagation:hierarchy_up".to_string();
        let derived = rule_hierarchy_down(&signal, &db);
        assert!(
            derived.is_empty(),
            "Hierarchy-derived signals must not re-propagate"
        );
    }

    #[test]
    fn test_rule_hierarchy_down_direct_children_only() {
        let db = test_db();
        let conn = db.conn_ref();

        conn.execute(
            "INSERT INTO accounts (id, name, updated_at) VALUES ('grandparent', 'GrandParent', '2026-01-01')",
            [],
        ).unwrap();
        conn.execute(
            "INSERT INTO accounts (id, name, parent_id, updated_at) VALUES ('parent1', 'Parent', 'grandparent', '2026-01-01')",
            [],
        ).unwrap();
        conn.execute(
            "INSERT INTO accounts (id, name, parent_id, updated_at) VALUES ('child1', 'Child', 'parent1', '2026-01-01')",
            [],
        ).unwrap();

        // Signal on grandparent — should only reach parent1, not child1
        let signal = make_signal("account", "grandparent", "strategy_shift", None);
        let derived = rule_hierarchy_down(&signal, &db);

        assert_eq!(derived.len(), 1);
        assert_eq!(derived[0].entity_id, "parent1");
    }

    // -----------------------------------------------------------------------
    // B1: 48-hour sibling accumulation
    // -----------------------------------------------------------------------

    #[test]
    fn test_rule_hierarchy_up_accumulation_with_siblings() {
        let db = test_db();
        let conn = db.conn_ref();

        conn.execute(
            "INSERT INTO accounts (id, name, updated_at) VALUES ('parent1', 'ParentCo', '2026-01-01')",
            [],
        ).unwrap();
        conn.execute(
            "INSERT INTO accounts (id, name, parent_id, updated_at) VALUES ('child1', 'Child1', 'parent1', '2026-01-01')",
            [],
        ).unwrap();
        conn.execute(
            "INSERT INTO accounts (id, name, parent_id, updated_at) VALUES ('child2', 'Child2', 'parent1', '2026-01-01')",
            [],
        ).unwrap();

        // Insert a recent sibling signal on the parent (from child2, within 48 hours)
        conn.execute(
            "INSERT INTO signal_events (id, entity_type, entity_id, signal_type, source, confidence, decay_half_life_days, created_at)
             VALUES ('sib1', 'account', 'parent1', 'health_change', 'clay', 0.75, 90, datetime('now', '-1 hours'))",
            [],
        ).unwrap();

        // Now fire rule_hierarchy_up from child1
        let signal = make_signal(
            "account",
            "child1",
            "health_change",
            Some("{\"new\": \"red\"}"),
        );
        let derived_with_sibling = rule_hierarchy_up(&signal, &db);

        // With a sibling, the accumulated confidence should be higher than solo (0.85 * 0.6 = 0.51)
        assert_eq!(derived_with_sibling.len(), 1);
        let solo_confidence = 0.85 * 0.6;
        assert!(
            derived_with_sibling[0].confidence > solo_confidence,
            "Accumulated confidence ({}) should exceed solo confidence ({})",
            derived_with_sibling[0].confidence,
            solo_confidence
        );
    }

    #[test]
    fn test_rule_hierarchy_up_accumulation_excludes_hierarchy_signals() {
        let db = test_db();
        let conn = db.conn_ref();

        conn.execute(
            "INSERT INTO accounts (id, name, updated_at) VALUES ('parent1', 'ParentCo', '2026-01-01')",
            [],
        ).unwrap();
        conn.execute(
            "INSERT INTO accounts (id, name, parent_id, updated_at) VALUES ('child1', 'Child1', 'parent1', '2026-01-01')",
            [],
        ).unwrap();

        // Insert a signal on the parent that came from hierarchy propagation — should be excluded
        conn.execute(
            "INSERT INTO signal_events (id, entity_type, entity_id, signal_type, source, confidence, decay_half_life_days, created_at)
             VALUES ('hier1', 'account', 'parent1', 'health_change', 'propagation:hierarchy_up', 0.75, 90, datetime('now', '-1 hours'))",
            [],
        ).unwrap();

        let signal = make_signal("account", "child1", "health_change", None);
        let derived = rule_hierarchy_up(&signal, &db);

        assert_eq!(derived.len(), 1);
        // Should use solo path since hierarchy signals are filtered out
        assert!((derived[0].confidence - 0.85 * 0.6).abs() < 0.01);
    }

    // -----------------------------------------------------------------------
    // Person Network (I391)
    // -----------------------------------------------------------------------

    fn setup_person_pair(db: &crate::db::ActionDb) {
        let conn = db.conn_ref();
        conn.execute(
            "INSERT INTO people (id, email, name, updated_at) VALUES ('p1', 'a@test.com', 'Alice', '2026-01-01')",
            [],
        ).unwrap();
        conn.execute(
            "INSERT INTO people (id, email, name, updated_at) VALUES ('p2', 'b@test.com', 'Bob', '2026-01-01')",
            [],
        ).unwrap();
    }

    #[test]
    fn test_rule_person_network_basic_propagation() {
        let db = test_db();
        setup_person_pair(&db);
        db.upsert_person_relationship(&crate::db::person_relationships::UpsertRelationship {
            id: "rel-1",
            from_person_id: "p1",
            to_person_id: "p2",
            relationship_type: "manager",
            direction: "directed",
            confidence: 0.8,
            context_entity_id: None,
            context_entity_type: None,
            source: "user_confirmed",
            rationale: None,
        })
        .unwrap();

        let signal = make_signal("person", "p1", "sentiment_shift", None);
        let derived = rule_person_network(&signal, &db);

        assert_eq!(derived.len(), 1);
        assert_eq!(derived[0].entity_id, "p2");
        assert_eq!(derived[0].entity_type, "person");
        assert_eq!(derived[0].source, "propagation:network");
        // manager: 0.85 * 0.8 * 0.4 * 1.0 = 0.272
        assert!((derived[0].confidence - 0.85 * 0.8 * 0.4).abs() < 0.01);
    }

    #[test]
    fn test_rule_person_network_loop_prevention() {
        let db = test_db();
        setup_person_pair(&db);
        db.upsert_person_relationship(&crate::db::person_relationships::UpsertRelationship {
            id: "rel-1",
            from_person_id: "p1",
            to_person_id: "p2",
            relationship_type: "peer",
            direction: "symmetric",
            confidence: 0.8,
            context_entity_id: None,
            context_entity_type: None,
            source: "user_confirmed",
            rationale: None,
        })
        .unwrap();

        let mut signal = make_signal("person", "p1", "sentiment_shift", None);
        signal.source = "propagation:network".to_string();
        let derived = rule_person_network(&signal, &db);
        assert!(
            derived.is_empty(),
            "Network-derived signals must not re-propagate"
        );
    }

    #[test]
    fn test_rule_person_network_confidence_gate() {
        let db = test_db();
        setup_person_pair(&db);
        db.upsert_person_relationship(&crate::db::person_relationships::UpsertRelationship {
            id: "rel-1",
            from_person_id: "p1",
            to_person_id: "p2",
            relationship_type: "manager",
            direction: "directed",
            confidence: 0.9,
            context_entity_id: None,
            context_entity_type: None,
            source: "user_confirmed",
            rationale: None,
        })
        .unwrap();

        let mut signal = make_signal("person", "p1", "sentiment_shift", None);
        signal.confidence = 0.5; // Below 0.65 threshold
        let derived = rule_person_network(&signal, &db);
        assert!(
            derived.is_empty(),
            "Low-confidence signals should not propagate via network"
        );
    }

    #[test]
    fn test_rule_person_network_mentor_multiplier() {
        let db = test_db();
        setup_person_pair(&db);
        db.upsert_person_relationship(&crate::db::person_relationships::UpsertRelationship {
            id: "rel-1",
            from_person_id: "p1",
            to_person_id: "p2",
            relationship_type: "mentor",
            direction: "directed",
            confidence: 0.8,
            context_entity_id: None,
            context_entity_type: None,
            source: "user_confirmed",
            rationale: None,
        })
        .unwrap();

        let signal = make_signal("person", "p1", "sentiment_shift", None);
        let derived = rule_person_network(&signal, &db);

        assert_eq!(derived.len(), 1);
        // mentor: 0.85 * 0.8 * 0.4 * 0.8 = 0.2176
        let expected = 0.85 * 0.8 * 0.4 * 0.8;
        assert!((derived[0].confidence - expected).abs() < 0.01);
    }

    #[test]
    fn test_rule_person_network_introduced_by_low_multiplier() {
        let db = test_db();
        setup_person_pair(&db);
        db.upsert_person_relationship(&crate::db::person_relationships::UpsertRelationship {
            id: "rel-1",
            from_person_id: "p1",
            to_person_id: "p2",
            relationship_type: "introduced_by",
            direction: "directed",
            confidence: 0.9,
            context_entity_id: None,
            context_entity_type: None,
            source: "user_confirmed",
            rationale: None,
        })
        .unwrap();

        let signal = make_signal("person", "p1", "status_change", None);
        let derived = rule_person_network(&signal, &db);

        assert_eq!(derived.len(), 1);
        // introduced_by: 0.85 * 0.9 * 0.4 * 0.5 = 0.153
        let expected = 0.85 * 0.9 * 0.4 * 0.5;
        assert!((derived[0].confidence - expected).abs() < 0.01);
    }

    #[test]
    fn test_rule_person_network_skips_non_person() {
        let db = test_db();
        let signal = make_signal("account", "a1", "health_change", None);
        let derived = rule_person_network(&signal, &db);
        assert!(derived.is_empty());
    }
}
