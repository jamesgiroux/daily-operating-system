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
                rule_name: "rule_person_job_change".to_string(),
            }
        })
        .collect()
}

// ---------------------------------------------------------------------------
// Rule: Meeting frequency drop → Account engagement_warning
// ---------------------------------------------------------------------------

/// When an account's meeting frequency signal indicates a >50% drop,
/// emit `engagement_warning`.
pub fn rule_meeting_frequency_drop(signal: &SignalEvent, _db: &ActionDb) -> Vec<DerivedSignal> {
    if signal.entity_type != "account" || signal.signal_type != "meeting_frequency" {
        return Vec::new();
    }

    // Parse the value JSON for current/previous counts
    let value_json: serde_json::Value = match signal.value.as_deref().and_then(|v| serde_json::from_str(v).ok()) {
        Some(v) => v,
        None => return Vec::new(),
    };

    let current = value_json.get("current_count").and_then(|v| v.as_f64()).unwrap_or(0.0);
    let previous = value_json.get("previous_count").and_then(|v| v.as_f64()).unwrap_or(0.0);

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
        rule_name: "rule_meeting_frequency_drop".to_string(),
    }]
}

// ---------------------------------------------------------------------------
// Rule: Overdue actions threshold → project_health_warning
// ---------------------------------------------------------------------------

/// When an entity has an `action_overdue` signal, check if ≥3 overdue actions
/// exist. If so, emit `project_health_warning`.
pub fn rule_overdue_actions(signal: &SignalEvent, db: &ActionDb) -> Vec<DerivedSignal> {
    if signal.signal_type != "action_overdue" {
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
        rule_name: "rule_overdue_actions".to_string(),
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
    for entity in entities.iter().filter(|e| matches!(e.entity_type, EntityType::Account)) {
        // Check if person is champion on this account
        let team = match db.get_account_team(&entity.id) {
            Ok(t) => t,
            Err(_) => continue,
        };

        let is_champion = team.iter().any(|m| {
            m.person_id == signal.entity_id
                && m.role.to_lowercase() == "champion"
        });

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
                rule_name: "rule_champion_sentiment".to_string(),
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
    if signal.signal_type != "person_departed" && signal.signal_type != "company_change" {
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
    for entity in entities.iter().filter(|e| matches!(e.entity_type, EntityType::Account)) {
        // Check champion role
        let team = match db.get_account_team(&entity.id) {
            Ok(t) => t,
            Err(_) => continue,
        };

        let is_champion = team.iter().any(|m| {
            m.person_id == signal.entity_id
                && m.role.to_lowercase() == "champion"
        });

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
                rule_name: "rule_departure_renewal".to_string(),
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
        rule_name: "rule_renewal_engagement_compound".to_string(),
    }]
}

// ---------------------------------------------------------------------------
// ActionDb helper methods for rules
// ---------------------------------------------------------------------------

impl ActionDb {
    /// Count meetings for an account in a date range.
    pub fn get_meeting_count_for_account(
        &self,
        account_id: &str,
        start: &str,
        end: &str,
    ) -> Result<i32, crate::db::DbError> {
        let count: i32 = self.conn_ref().query_row(
            "SELECT COUNT(*) FROM meetings_history mh
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
        let count: i32 = self
            .conn_ref()
            .query_row(&sql, rusqlite::params![entity_id, today], |row| row.get(0))?;
        Ok(count)
    }

}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::signals::bus;

    fn test_db() -> ActionDb {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("test.db");
        std::mem::forget(dir);
        ActionDb::open_at(path).expect("open")
    }

    fn make_signal(entity_type: &str, entity_id: &str, signal_type: &str, value: Option<&str>) -> SignalEvent {
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
            "INSERT INTO entity_people (entity_id, person_id) VALUES ('a1', 'p1')",
            [],
        )
        .unwrap();

        let signal = make_signal("person", "p1", "title_change", Some("{\"new_value\": \"CRO\"}"));
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
        assert!(derived.is_empty(), "25% drop should not trigger (need >50%)");
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

        let signal = make_signal("account", "a1", "action_overdue", None);
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
            "INSERT INTO entity_people (entity_id, person_id) VALUES ('a1', 'p1')",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO account_team (account_id, person_id, role, created_at)
             VALUES ('a1', 'p1', 'champion', '2026-01-01')",
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

        let signal = make_signal("person", "p1", "company_change", Some("{\"new_value\": \"NewCo\"}"));
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
        ).unwrap();

        let signal = make_signal("account", "a1", "renewal_proximity", Some("{\"days_until_renewal\": 25}"));
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
        ).unwrap();

        // Recent meeting
        conn.execute(
            "INSERT INTO meetings_history (id, title, meeting_type, start_time, created_at)
             VALUES ('m1', 'Sync', 'external', datetime('now', '-5 days'), datetime('now', '-5 days'))",
            [],
        ).unwrap();
        conn.execute(
            "INSERT INTO meeting_entities (meeting_id, entity_id, entity_type)
             VALUES ('m1', 'a1', 'account')",
            [],
        ).unwrap();

        let signal = make_signal("account", "a1", "renewal_proximity", None);
        let derived = rule_renewal_engagement_compound(&signal, &db);
        assert!(derived.is_empty(), "Should not fire when recent meeting exists");
    }
}
