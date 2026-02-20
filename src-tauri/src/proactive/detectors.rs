//! Pattern detectors for proactive surfacing (I260).
//!
//! Each detector is a pure function that queries the database for a specific
//! pattern and returns zero or more `RawInsight` values. Detectors do no AI
//! calls — they are pure SQL + Rust logic.

use chrono::{Datelike, Duration};
use rusqlite::params;

use crate::db::ActionDb;
use crate::helpers;
use super::engine::{DetectorContext, RawInsight, fingerprint};

// ---------------------------------------------------------------------------
// Detector 1: Renewal gap
// ---------------------------------------------------------------------------

/// Account with renewal ≤60d + no QBR scheduled + last exec contact >30d.
pub fn detect_renewal_gap(db: &ActionDb, ctx: &DetectorContext) -> Vec<RawInsight> {
    let accounts = match db.get_renewal_alerts(60) {
        Ok(a) => a,
        Err(_) => return Vec::new(),
    };

    let mut insights = Vec::new();
    for acct in accounts {
        let contract_end = match &acct.contract_end {
            Some(d) => d.clone(),
            None => continue,
        };

        // Check if there's been a meeting with this account in the last 30 days
        let recent_meeting_count: i32 = db
            .conn_ref()
            .query_row(
                "SELECT COUNT(*) FROM meetings_history mh
                 JOIN meeting_entities me ON me.meeting_id = mh.id
                 WHERE me.entity_id = ?1 AND me.entity_type = 'account'
                 AND mh.start_time >= datetime('now', '-30 days')",
                params![acct.id],
                |row| row.get(0),
            )
            .unwrap_or(0);

        if recent_meeting_count > 0 {
            continue;
        }

        let days_until = if let Ok(end_date) = chrono::NaiveDate::parse_from_str(&contract_end, "%Y-%m-%d") {
            (end_date - ctx.today).num_days()
        } else {
            60
        };

        let fp = fingerprint(&["account", &acct.id, "renewal_gap"]);
        let context_json = serde_json::json!({
            "account_name": acct.name,
            "renewal_date": contract_end,
            "days_until_renewal": days_until,
            "last_contact_days": 30
        });

        insights.push(RawInsight {
            detector_name: "detect_renewal_gap".to_string(),
            fingerprint: fp,
            entity_type: "account".to_string(),
            entity_id: acct.id.clone(),
            signal_type: "proactive_renewal_gap".to_string(),
            headline: format!("{} renewal in {}d with no recent contact", acct.name, days_until),
            detail: format!(
                "Account {} has a renewal on {} ({} days away) but no meetings in the last 30 days.",
                acct.name, contract_end, days_until
            ),
            confidence: 0.90,
            context_json: Some(context_json.to_string()),
        });
    }

    insights
}

// ---------------------------------------------------------------------------
// Detector 2: Relationship drift
// ---------------------------------------------------------------------------

/// Person with 3+ historical meetings where current 30d freq < 50% of trailing 90d avg.
pub fn detect_relationship_drift(db: &ActionDb, _ctx: &DetectorContext) -> Vec<RawInsight> {
    let sql = "SELECT p.id, p.name,
        (SELECT COUNT(*) FROM meetings_history mh
         JOIN meeting_entities me ON me.meeting_id = mh.id
         WHERE me.entity_id = p.id AND me.entity_type = 'person'
         AND mh.start_time >= datetime('now', '-30 days')) as meetings_30d,
        (SELECT COUNT(*) FROM meetings_history mh
         JOIN meeting_entities me ON me.meeting_id = mh.id
         WHERE me.entity_id = p.id AND me.entity_type = 'person'
         AND mh.start_time >= datetime('now', '-90 days')) as meetings_90d
    FROM people p
    WHERE p.archived = 0";

    let conn = db.conn_ref();
    let mut stmt = match conn.prepare(sql) {
        Ok(s) => s,
        Err(_) => return Vec::new(),
    };

    let rows = match stmt.query_map([], |row| {
        Ok((
            row.get::<_, String>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, i32>(2)?,
            row.get::<_, i32>(3)?,
        ))
    }) {
        Ok(r) => r,
        Err(_) => return Vec::new(),
    };

    let mut insights = Vec::new();
    for row in rows {
        let (person_id, person_name, meetings_30d, meetings_90d) = match row {
            Ok(r) => r,
            Err(_) => continue,
        };

        if meetings_90d < 3 {
            continue;
        }

        // 30d rate < 50% of 90d average per 30d period
        // 90d avg per 30d = meetings_90d / 3.0
        // Check: meetings_30d < 0.5 * (meetings_90d / 3.0)
        // Equivalent: meetings_30d * 6 < meetings_90d
        if meetings_30d * 6 >= meetings_90d {
            continue;
        }

        let avg_90d_per_30d = meetings_90d as f64 / 3.0;
        let drop_pct = if avg_90d_per_30d > 0.0 {
            ((1.0 - (meetings_30d as f64 / avg_90d_per_30d)) * 100.0) as i32
        } else {
            100
        };

        let fp = fingerprint(&["person", &person_id, "relationship_drift"]);
        let context_json = serde_json::json!({
            "person_name": person_name,
            "meetings_30d": meetings_30d,
            "meetings_90d": meetings_90d,
            "drop_pct": drop_pct
        });

        insights.push(RawInsight {
            detector_name: "detect_relationship_drift".to_string(),
            fingerprint: fp,
            entity_type: "person".to_string(),
            entity_id: person_id,
            signal_type: "proactive_relationship_drift".to_string(),
            headline: format!("{} meeting frequency dropped {}%", person_name, drop_pct),
            detail: format!(
                "{} had {} meetings in 90d but only {} in the last 30d ({}% drop).",
                person_name, meetings_90d, meetings_30d, drop_pct
            ),
            confidence: 0.75,
            context_json: Some(context_json.to_string()),
        });
    }

    insights
}

// ---------------------------------------------------------------------------
// Detector 3: Email volume spike
// ---------------------------------------------------------------------------

/// Entity with 3+ email_signals in 7d when trailing 30d avg is <1/week.
pub fn detect_email_volume_spike(db: &ActionDb, _ctx: &DetectorContext) -> Vec<RawInsight> {
    let sql = "SELECT entity_id, entity_type,
        SUM(CASE WHEN detected_at >= datetime('now', '-7 days') THEN 1 ELSE 0 END) as recent_7d,
        SUM(CASE WHEN detected_at >= datetime('now', '-30 days') THEN 1 ELSE 0 END) as total_30d
    FROM email_signals
    GROUP BY entity_id, entity_type
    HAVING recent_7d >= 3 AND (total_30d - recent_7d) < 4";

    let conn = db.conn_ref();
    let mut stmt = match conn.prepare(sql) {
        Ok(s) => s,
        Err(_) => return Vec::new(),
    };

    let rows = match stmt.query_map([], |row| {
        Ok((
            row.get::<_, String>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, i32>(2)?,
            row.get::<_, i32>(3)?,
        ))
    }) {
        Ok(r) => r,
        Err(_) => return Vec::new(),
    };

    let mut insights = Vec::new();
    for row in rows {
        let (entity_id, entity_type, recent_7d, total_30d) = match row {
            Ok(r) => r,
            Err(_) => continue,
        };

        let entity_name = helpers::resolve_entity_name(db, &entity_type, &entity_id);
        let baseline = total_30d - recent_7d;
        let avg_weekly = baseline as f64 / 3.3;

        let fp = fingerprint(&[&entity_type, &entity_id, "email_spike"]);
        let context_json = serde_json::json!({
            "entity_name": entity_name,
            "count_7d": recent_7d,
            "avg_weekly": (avg_weekly * 100.0).round() / 100.0
        });

        insights.push(RawInsight {
            detector_name: "detect_email_volume_spike".to_string(),
            fingerprint: fp,
            entity_type: entity_type.clone(),
            entity_id,
            signal_type: "proactive_email_spike".to_string(),
            headline: format!("{} email spike: {} in 7d vs {:.1}/wk baseline", entity_name, recent_7d, avg_weekly),
            detail: format!(
                "{} received {} email signals in the last 7 days, up from a baseline of {:.1} per week.",
                entity_name, recent_7d, avg_weekly
            ),
            confidence: 0.70,
            context_json: Some(context_json.to_string()),
        });
    }

    insights
}


// ---------------------------------------------------------------------------
// Detector 4: Meeting load forecast
// ---------------------------------------------------------------------------

/// Next week has 2x+ meetings vs this week, with >5 total next-week meetings.
pub fn detect_meeting_load_forecast(db: &ActionDb, ctx: &DetectorContext) -> Vec<RawInsight> {
    let days_since_monday = ctx.today.weekday().num_days_from_monday();
    let this_monday = ctx.today - Duration::days(days_since_monday as i64);
    let next_monday = this_monday + Duration::days(7);
    let week_after = next_monday + Duration::days(7);

    let this_week_start = this_monday.format("%Y-%m-%d").to_string();
    let next_week_start = next_monday.format("%Y-%m-%d").to_string();
    let week_after_start = week_after.format("%Y-%m-%d").to_string();

    let this_week_count: i32 = db
        .conn_ref()
        .query_row(
            "SELECT COUNT(*) FROM meetings_history
             WHERE start_time >= ?1 AND start_time < ?2",
            params![this_week_start, next_week_start],
            |row| row.get(0),
        )
        .unwrap_or(0);

    let next_week_count: i32 = db
        .conn_ref()
        .query_row(
            "SELECT COUNT(*) FROM meetings_history
             WHERE start_time >= ?1 AND start_time < ?2",
            params![next_week_start, week_after_start],
            |row| row.get(0),
        )
        .unwrap_or(0);

    if next_week_count <= 5 || this_week_count == 0 {
        return Vec::new();
    }

    if next_week_count < 2 * this_week_count {
        return Vec::new();
    }

    let fp = fingerprint(&["meeting_load", &next_week_start]);
    let context_json = serde_json::json!({
        "this_week_count": this_week_count,
        "next_week_count": next_week_count
    });

    vec![RawInsight {
        detector_name: "detect_meeting_load_forecast".to_string(),
        fingerprint: fp,
        entity_type: "user".to_string(),
        entity_id: "self".to_string(),
        signal_type: "proactive_meeting_load".to_string(),
        headline: format!(
            "Next week has {}x more meetings ({} vs {})",
            next_week_count / this_week_count,
            next_week_count,
            this_week_count
        ),
        detail: format!(
            "Next week has {} meetings compared to {} this week — a significant increase that may need prep time.",
            next_week_count, this_week_count
        ),
        confidence: 0.65,
        context_json: Some(context_json.to_string()),
    }]
}

// ---------------------------------------------------------------------------
// Detector 5: Stale champion
// ---------------------------------------------------------------------------

/// Account champion with no meeting in 45+ days and renewal within 90d.
pub fn detect_stale_champion(db: &ActionDb, ctx: &DetectorContext) -> Vec<RawInsight> {
    let conn = db.conn_ref();
    let mut results = Vec::new();

    // Find accounts with renewal within 90 days
    let mut acct_stmt = conn
        .prepare(
            "SELECT id, name, contract_end FROM accounts
             WHERE contract_end IS NOT NULL
               AND contract_end >= date('now')
               AND contract_end <= date('now', '+90 days')
               AND archived = 0",
        )
        .unwrap_or_else(|_| conn.prepare("SELECT 1 WHERE 0").unwrap());

    let accounts: Vec<(String, String, String)> = acct_stmt
        .query_map([], |row| {
            Ok((row.get(0)?, row.get(1)?, row.get(2)?))
        })
        .ok()
        .map(|rows| rows.filter_map(|r| r.ok()).collect())
        .unwrap_or_default();

    for (account_id, account_name, contract_end) in &accounts {
        // Find champion(s) for this account
        let champions: Vec<String> = conn
            .prepare(
                "SELECT person_id FROM account_team WHERE account_id = ?1 AND role = 'champion'",
            )
            .and_then(|mut stmt| {
                stmt.query_map(params![account_id], |row| row.get(0))
                    .map(|rows| rows.filter_map(|r| r.ok()).collect())
            })
            .unwrap_or_default();

        for person_id in &champions {
            // Check last meeting with this person
            let last_meeting: Option<String> = conn
                .query_row(
                    "SELECT MAX(mh.start_time) FROM meetings_history mh
                     JOIN meeting_entities me ON me.meeting_id = mh.id
                     WHERE me.entity_id = ?1 AND me.entity_type = 'person'",
                    params![person_id],
                    |row| row.get(0),
                )
                .ok()
                .flatten();

            let days_since = match &last_meeting {
                Some(ts) => {
                    if let Ok(dt) = chrono::NaiveDateTime::parse_from_str(ts, "%Y-%m-%dT%H:%M:%S")
                        .or_else(|_| chrono::NaiveDateTime::parse_from_str(ts, "%Y-%m-%d %H:%M:%S"))
                    {
                        (ctx.today - dt.date()).num_days()
                    } else {
                        999
                    }
                }
                None => 999,
            };

            if days_since >= 45 {
                let renewal_days = if let Ok(ce) =
                    chrono::NaiveDate::parse_from_str(contract_end, "%Y-%m-%d")
                {
                    (ce - ctx.today).num_days()
                } else {
                    0
                };

                let person_name: String = conn
                    .query_row(
                        "SELECT name FROM people WHERE id = ?1",
                        params![person_id],
                        |row| row.get(0),
                    )
                    .unwrap_or_else(|_| "Unknown".to_string());

                results.push(RawInsight {
                    detector_name: "detect_stale_champion".to_string(),
                    fingerprint: fingerprint(&["stale_champion", account_id, person_id]),
                    entity_type: "account".to_string(),
                    entity_id: account_id.clone(),
                    signal_type: "proactive_stale_champion".to_string(),
                    headline: format!(
                        "Champion {} at {} has gone dark ({} days)",
                        person_name, account_name, days_since
                    ),
                    detail: format!(
                        "No meeting with champion {} in {} days. Renewal in {} days.",
                        person_name, days_since, renewal_days
                    ),
                    confidence: 0.85,
                    context_json: Some(format!(
                        r#"{{"person_name":"{}","account_name":"{}","days_since_contact":{},"renewal_days":{}}}"#,
                        person_name, account_name, days_since, renewal_days
                    )),
                });
            }
        }
    }

    results
}

// ---------------------------------------------------------------------------
// Detector 6: Action cluster
// ---------------------------------------------------------------------------

/// Entity with 5+ pending actions, 3+ overdue.
pub fn detect_action_cluster(db: &ActionDb, ctx: &DetectorContext) -> Vec<RawInsight> {
    let conn = db.conn_ref();
    let mut results = Vec::new();
    let _ = ctx;

    // Check accounts with action clusters
    let account_clusters: Vec<(String, i64, i64)> = conn
        .prepare(
            "SELECT account_id,
                    COUNT(*) as pending_count,
                    SUM(CASE WHEN due_date IS NOT NULL AND due_date < date('now') THEN 1 ELSE 0 END) as overdue_count
             FROM actions
             WHERE status = 'pending' AND account_id IS NOT NULL
             GROUP BY account_id
             HAVING pending_count >= 5 AND overdue_count >= 3",
        )
        .and_then(|mut stmt| {
            stmt.query_map([], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)))
                .map(|rows| rows.filter_map(|r| r.ok()).collect())
        })
        .unwrap_or_default();

    for (account_id, pending_count, overdue_count) in &account_clusters {
        let name: String = conn
            .query_row(
                "SELECT name FROM accounts WHERE id = ?1",
                params![account_id],
                |row| row.get(0),
            )
            .unwrap_or_else(|_| "Unknown".to_string());

        results.push(RawInsight {
            detector_name: "detect_action_cluster".to_string(),
            fingerprint: fingerprint(&["action_cluster", "account", account_id]),
            entity_type: "account".to_string(),
            entity_id: account_id.clone(),
            signal_type: "proactive_action_cluster".to_string(),
            headline: format!(
                "{} has {} pending actions ({} overdue)",
                name, pending_count, overdue_count
            ),
            detail: format!(
                "Account {} has {} pending actions with {} overdue. Consider triaging.",
                name, pending_count, overdue_count
            ),
            confidence: 0.70,
            context_json: Some(format!(
                r#"{{"entity_name":"{}","pending_count":{},"overdue_count":{}}}"#,
                name, pending_count, overdue_count
            )),
        });
    }

    // Check projects with action clusters
    let project_clusters: Vec<(String, i64, i64)> = conn
        .prepare(
            "SELECT project_id,
                    COUNT(*) as pending_count,
                    SUM(CASE WHEN due_date IS NOT NULL AND due_date < date('now') THEN 1 ELSE 0 END) as overdue_count
             FROM actions
             WHERE status = 'pending' AND project_id IS NOT NULL
             GROUP BY project_id
             HAVING pending_count >= 5 AND overdue_count >= 3",
        )
        .and_then(|mut stmt| {
            stmt.query_map([], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)))
                .map(|rows| rows.filter_map(|r| r.ok()).collect())
        })
        .unwrap_or_default();

    for (project_id, pending_count, overdue_count) in &project_clusters {
        let name: String = conn
            .query_row(
                "SELECT name FROM projects WHERE id = ?1",
                params![project_id],
                |row| row.get(0),
            )
            .unwrap_or_else(|_| "Unknown".to_string());

        results.push(RawInsight {
            detector_name: "detect_action_cluster".to_string(),
            fingerprint: fingerprint(&["action_cluster", "project", project_id]),
            entity_type: "project".to_string(),
            entity_id: project_id.clone(),
            signal_type: "proactive_action_cluster".to_string(),
            headline: format!(
                "{} has {} pending actions ({} overdue)",
                name, pending_count, overdue_count
            ),
            detail: format!(
                "Project {} has {} pending actions with {} overdue. Consider triaging.",
                name, pending_count, overdue_count
            ),
            confidence: 0.70,
            context_json: Some(format!(
                r#"{{"entity_name":"{}","pending_count":{},"overdue_count":{}}}"#,
                name, pending_count, overdue_count
            )),
        });
    }

    results
}

// ---------------------------------------------------------------------------
// Detector 7: Prep coverage gap
// ---------------------------------------------------------------------------

/// Tomorrow has 3+ external meetings, <60% have entity intelligence.
pub fn detect_prep_coverage_gap(db: &ActionDb, ctx: &DetectorContext) -> Vec<RawInsight> {
    let conn = db.conn_ref();
    let tomorrow = ctx.today + Duration::days(1);
    let tomorrow_str = tomorrow.format("%Y-%m-%d").to_string();

    // Get external meetings tomorrow
    let meetings: Vec<String> = conn
        .prepare(
            "SELECT id FROM meetings_history
             WHERE date(start_time) = ?1
               AND meeting_type NOT IN ('internal', 'personal', 'focus', 'blocked')",
        )
        .and_then(|mut stmt| {
            stmt.query_map(params![tomorrow_str], |row| row.get(0))
                .map(|rows| rows.filter_map(|r| r.ok()).collect())
        })
        .unwrap_or_default();

    let total_external = meetings.len();
    if total_external < 3 {
        return Vec::new();
    }

    // Count meetings that have linked entities (intel coverage)
    let mut with_intel = 0usize;
    for meeting_id in &meetings {
        let entity_count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM meeting_entities WHERE meeting_id = ?1",
                params![meeting_id],
                |row| row.get(0),
            )
            .unwrap_or(0);
        if entity_count > 0 {
            with_intel += 1;
        }
    }

    let coverage = with_intel as f64 / total_external as f64;
    if coverage >= 0.6 {
        return Vec::new();
    }

    vec![RawInsight {
        detector_name: "detect_prep_coverage_gap".to_string(),
        fingerprint: fingerprint(&["prep_gap", &tomorrow_str]),
        entity_type: "meeting".to_string(),
        entity_id: tomorrow_str.clone(),
        signal_type: "proactive_prep_gap".to_string(),
        headline: format!(
            "Tomorrow has {} external meetings, only {:.0}% prepped",
            total_external,
            coverage * 100.0
        ),
        detail: format!(
            "{} of {} external meetings tomorrow have entity intelligence linked.",
            with_intel, total_external
        ),
        confidence: 0.80,
        context_json: Some(format!(
            r#"{{"total_external":{},"with_intel":{},"date":"{}"}}"#,
            total_external, with_intel, tomorrow_str
        )),
    }]
}

// ---------------------------------------------------------------------------
// Detector 8: No contact accounts
// ---------------------------------------------------------------------------

/// Entities with no meeting/email signal in 30+ days (not archived).
pub fn detect_no_contact_accounts(db: &ActionDb, ctx: &DetectorContext) -> Vec<RawInsight> {
    let conn = db.conn_ref();
    let _ = ctx;

    let accounts: Vec<(String, String)> = conn
        .prepare(
            "SELECT a.id, a.name FROM accounts a
             WHERE a.archived = 0 AND a.is_internal = 0
             AND NOT EXISTS (
                 SELECT 1 FROM meetings_history mh
                 JOIN meeting_entities me ON me.meeting_id = mh.id
                 WHERE me.entity_id = a.id AND me.entity_type = 'account'
                   AND mh.start_time >= datetime('now', '-30 days')
             )
             AND NOT EXISTS (
                 SELECT 1 FROM email_signals es
                 WHERE es.entity_id = a.id AND es.entity_type = 'account'
                   AND es.detected_at >= datetime('now', '-30 days')
             )",
        )
        .and_then(|mut stmt| {
            stmt.query_map([], |row| Ok((row.get(0)?, row.get(1)?)))
                .map(|rows| rows.filter_map(|r| r.ok()).collect())
        })
        .unwrap_or_default();

    accounts
        .into_iter()
        .map(|(id, name)| RawInsight {
            detector_name: "detect_no_contact_accounts".to_string(),
            fingerprint: fingerprint(&["no_contact", &id]),
            entity_type: "account".to_string(),
            entity_id: id,
            signal_type: "proactive_no_contact".to_string(),
            headline: format!("{} has had no contact in 30+ days", name),
            detail: format!(
                "No meetings or email signals for {} in the last 30 days.",
                name
            ),
            confidence: 0.60,
            context_json: Some(format!(r#"{{"account_name":"{}"}}"#, name)),
        })
        .collect()
}

// ---------------------------------------------------------------------------
// Detector 9: Renewal proximity (standalone)
// ---------------------------------------------------------------------------

/// Account with contract_end within 90 days. Tiered confidence by proximity.
/// Skips accounts that already have a churn event recorded.
pub fn detect_renewal_proximity(db: &ActionDb, ctx: &DetectorContext) -> Vec<RawInsight> {
    let conn = db.conn_ref();

    // Get accounts with renewal within 90 days
    let accounts: Vec<(String, String, String)> = conn
        .prepare(
            "SELECT a.id, a.name, a.contract_end FROM accounts a
             WHERE a.contract_end IS NOT NULL
               AND a.contract_end >= date('now')
               AND a.contract_end <= date('now', '+90 days')
               AND a.archived = 0
               AND a.is_internal = 0
               AND NOT EXISTS (
                   SELECT 1 FROM account_events ae
                   WHERE ae.account_id = a.id AND ae.event_type = 'churn'
               )",
        )
        .and_then(|mut stmt| {
            stmt.query_map([], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)))
                .map(|rows| rows.filter_map(|r| r.ok()).collect())
        })
        .unwrap_or_default();

    let mut insights = Vec::new();
    for (account_id, account_name, contract_end) in accounts {
        let days_until = if let Ok(end_date) =
            chrono::NaiveDate::parse_from_str(&contract_end, "%Y-%m-%d")
        {
            (end_date - ctx.today).num_days()
        } else {
            90
        };

        // Tiered confidence
        let confidence = if days_until <= 30 {
            0.90
        } else if days_until <= 60 {
            0.70
        } else {
            0.50
        };

        let fp = fingerprint(&["account", &account_id, "renewal_proximity"]);
        let context_json = serde_json::json!({
            "account_name": account_name,
            "renewal_date": contract_end,
            "days_until_renewal": days_until,
        });

        insights.push(RawInsight {
            detector_name: "detect_renewal_proximity".to_string(),
            fingerprint: fp,
            entity_type: "account".to_string(),
            entity_id: account_id,
            signal_type: "renewal_proximity".to_string(),
            headline: format!("{} renews in {} days", account_name, days_until),
            detail: format!(
                "Account {} has a renewal on {} ({} days away).",
                account_name, contract_end, days_until
            ),
            confidence,
            context_json: Some(context_json.to_string()),
        });
    }

    insights
}

// ---------------------------------------------------------------------------
// Tests for detectors 5-9
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;
    use crate::db::test_utils::test_db;

    fn test_ctx(today: NaiveDate) -> DetectorContext {
        DetectorContext {
            today,
            user_domains: vec!["example.com".to_string()],
            profile: "general".to_string(),
        }
    }

    // -- Detector 1: Renewal gap --

    #[test]
    fn test_renewal_gap_no_accounts() {
        let db = test_db();
        let ctx = test_ctx(NaiveDate::from_ymd_opt(2026, 2, 18).unwrap());
        let insights = detect_renewal_gap(&db, &ctx);
        assert!(insights.is_empty());
    }

    #[test]
    fn test_renewal_gap_fires_when_no_recent_meeting() {
        let db = test_db();
        let ctx = test_ctx(NaiveDate::from_ymd_opt(2026, 2, 18).unwrap());

        db.conn_ref()
            .execute(
                "INSERT INTO accounts (id, name, contract_end, updated_at, archived, is_internal)
                 VALUES ('a1', 'Acme Corp', '2026-03-15', '2026-01-01', 0, 0)",
                [],
            )
            .unwrap();

        let insights = detect_renewal_gap(&db, &ctx);
        assert_eq!(insights.len(), 1);
        assert_eq!(insights[0].signal_type, "proactive_renewal_gap");
        assert_eq!(insights[0].entity_id, "a1");
        assert!(insights[0].confidence == 0.90);
    }

    #[test]
    fn test_renewal_gap_skipped_with_recent_meeting() {
        let db = test_db();
        let ctx = test_ctx(NaiveDate::from_ymd_opt(2026, 2, 18).unwrap());

        db.conn_ref()
            .execute(
                "INSERT INTO accounts (id, name, contract_end, updated_at, archived, is_internal)
                 VALUES ('a1', 'Acme Corp', '2026-03-15', '2026-01-01', 0, 0)",
                [],
            )
            .unwrap();

        db.conn_ref()
            .execute(
                "INSERT INTO meetings_history (id, title, meeting_type, start_time, created_at)
                 VALUES ('m1', 'QBR', 'external', '2026-02-10T10:00:00', '2026-02-10')",
                [],
            )
            .unwrap();
        db.conn_ref()
            .execute(
                "INSERT INTO meeting_entities (meeting_id, entity_id, entity_type)
                 VALUES ('m1', 'a1', 'account')",
                [],
            )
            .unwrap();

        let insights = detect_renewal_gap(&db, &ctx);
        assert!(insights.is_empty(), "Should not fire when recent meeting exists");
    }

    // -- Detector 2: Relationship drift --

    #[test]
    fn test_relationship_drift_no_people() {
        let db = test_db();
        let ctx = test_ctx(NaiveDate::from_ymd_opt(2026, 2, 18).unwrap());
        let insights = detect_relationship_drift(&db, &ctx);
        assert!(insights.is_empty());
    }

    #[test]
    fn test_relationship_drift_fires_on_drop() {
        let db = test_db();
        let ctx = test_ctx(NaiveDate::from_ymd_opt(2026, 2, 18).unwrap());

        db.conn_ref()
            .execute(
                "INSERT INTO people (id, email, name, archived, updated_at) VALUES ('p1', 'alice@example.com', 'Alice', 0, '2026-01-01')",
                [],
            )
            .unwrap();

        // Add 6 meetings in the 31-90d range (none in last 30d)
        for i in 0..6 {
            let meeting_id = format!("m{}", i);
            let date = format!("2025-12-{:02}T10:00:00", 5 + i);
            db.conn_ref()
                .execute(
                    "INSERT INTO meetings_history (id, title, meeting_type, start_time, created_at)
                     VALUES (?1, 'Sync', 'external', ?2, ?2)",
                    params![meeting_id, date],
                )
                .unwrap();
            db.conn_ref()
                .execute(
                    "INSERT INTO meeting_entities (meeting_id, entity_id, entity_type)
                     VALUES (?1, 'p1', 'person')",
                    params![meeting_id],
                )
                .unwrap();
        }

        let insights = detect_relationship_drift(&db, &ctx);
        assert_eq!(insights.len(), 1);
        assert_eq!(insights[0].signal_type, "proactive_relationship_drift");
        assert_eq!(insights[0].entity_id, "p1");
    }

    #[test]
    fn test_relationship_drift_not_enough_meetings() {
        let db = test_db();
        let ctx = test_ctx(NaiveDate::from_ymd_opt(2026, 2, 18).unwrap());

        db.conn_ref()
            .execute(
                "INSERT INTO people (id, email, name, archived, updated_at) VALUES ('p1', 'bob@example.com', 'Bob', 0, '2026-01-01')",
                [],
            )
            .unwrap();

        for i in 0..2 {
            let mid = format!("m{}", i);
            let date = format!("2025-12-{:02}T10:00:00", 10 + i * 10);
            db.conn_ref()
                .execute(
                    "INSERT INTO meetings_history (id, title, meeting_type, start_time, created_at)
                     VALUES (?1, 'Chat', 'external', ?2, ?2)",
                    params![mid, date],
                )
                .unwrap();
            db.conn_ref()
                .execute(
                    "INSERT INTO meeting_entities (meeting_id, entity_id, entity_type)
                     VALUES (?1, 'p1', 'person')",
                    params![mid],
                )
                .unwrap();
        }

        let insights = detect_relationship_drift(&db, &ctx);
        assert!(insights.is_empty(), "Should not fire with <3 meetings");
    }

    // -- Detector 3: Email volume spike --

    #[test]
    fn test_email_spike_no_signals() {
        let db = test_db();
        let ctx = test_ctx(NaiveDate::from_ymd_opt(2026, 2, 18).unwrap());
        let insights = detect_email_volume_spike(&db, &ctx);
        assert!(insights.is_empty());
    }

    #[test]
    fn test_email_spike_fires() {
        let db = test_db();
        let ctx = test_ctx(NaiveDate::from_ymd_opt(2026, 2, 18).unwrap());

        db.conn_ref()
            .execute(
                "INSERT INTO accounts (id, name, updated_at, archived, is_internal)
                 VALUES ('a1', 'SpikeCo', '2026-01-01', 0, 0)",
                [],
            )
            .unwrap();

        for i in 0..4 {
            db.conn_ref()
                .execute(
                    "INSERT INTO email_signals (email_id, entity_id, entity_type, signal_type, signal_text, detected_at)
                     VALUES (?1, 'a1', 'account', 'mention', 'test', datetime('now', '-' || ?2 || ' days'))",
                    params![format!("e{}", i), i],
                )
                .unwrap();
        }

        let insights = detect_email_volume_spike(&db, &ctx);
        assert_eq!(insights.len(), 1);
        assert_eq!(insights[0].signal_type, "proactive_email_spike");
        assert_eq!(insights[0].entity_type, "account");
    }

    // -- Detector 4: Meeting load forecast --

    #[test]
    fn test_meeting_load_no_meetings() {
        let db = test_db();
        let ctx = test_ctx(NaiveDate::from_ymd_opt(2026, 2, 18).unwrap());
        let insights = detect_meeting_load_forecast(&db, &ctx);
        assert!(insights.is_empty());
    }

    #[test]
    fn test_meeting_load_fires_on_spike() {
        let db = test_db();
        // Wednesday Feb 18 => this_monday = Feb 16, next_monday = Feb 23
        let ctx = test_ctx(NaiveDate::from_ymd_opt(2026, 2, 18).unwrap());

        // 3 meetings this week (Feb 16-22)
        for i in 0..3 {
            let mid = format!("tw{}", i);
            let date = format!("2026-02-{:02}T10:00:00", 16 + i);
            db.conn_ref()
                .execute(
                    "INSERT INTO meetings_history (id, title, meeting_type, start_time, created_at)
                     VALUES (?1, 'This week', 'external', ?2, ?2)",
                    params![mid, date],
                )
                .unwrap();
        }

        // 8 meetings next week (Feb 23-Mar 1)
        for i in 0..8 {
            let mid = format!("nw{}", i);
            let date = format!("2026-02-{:02}T10:00:00", 23 + i);
            db.conn_ref()
                .execute(
                    "INSERT INTO meetings_history (id, title, meeting_type, start_time, created_at)
                     VALUES (?1, 'Next week', 'external', ?2, ?2)",
                    params![mid, date],
                )
                .unwrap();
        }

        let insights = detect_meeting_load_forecast(&db, &ctx);
        assert_eq!(insights.len(), 1);
        assert_eq!(insights[0].signal_type, "proactive_meeting_load");
        assert!(insights[0].confidence == 0.65);
    }

    #[test]
    fn test_meeting_load_no_spike_when_similar() {
        let db = test_db();
        let ctx = test_ctx(NaiveDate::from_ymd_opt(2026, 2, 18).unwrap());

        // 5 meetings this week
        for i in 0..5 {
            let mid = format!("tw{}", i);
            let date = format!("2026-02-{:02}T10:00:00", 16 + i);
            db.conn_ref()
                .execute(
                    "INSERT INTO meetings_history (id, title, meeting_type, start_time, created_at)
                     VALUES (?1, 'This week', 'external', ?2, ?2)",
                    params![mid, date],
                )
                .unwrap();
        }

        // 6 meetings next week (not 2x)
        for i in 0..6 {
            let mid = format!("nw{}", i);
            let date = format!("2026-02-{:02}T10:00:00", 23 + i);
            db.conn_ref()
                .execute(
                    "INSERT INTO meetings_history (id, title, meeting_type, start_time, created_at)
                     VALUES (?1, 'Next week', 'external', ?2, ?2)",
                    params![mid, date],
                )
                .unwrap();
        }

        let insights = detect_meeting_load_forecast(&db, &ctx);
        assert!(insights.is_empty(), "Should not fire when next week is not 2x this week");
    }

    // -- Detector 5: Stale champion --

    #[test]
    fn test_stale_champion_fires_when_no_recent_meeting() {
        let db = test_db();
        let conn = db.conn_ref();
        let today = NaiveDate::from_ymd_opt(2026, 3, 15).unwrap();

        // Account with renewal in 60 days
        conn.execute(
            "INSERT INTO accounts (id, name, contract_end, archived, is_internal, updated_at)
             VALUES ('a1', 'AcmeCo', '2026-05-14', 0, 0, '2026-01-01')",
            [],
        ).unwrap();

        // Person as champion
        conn.execute(
            "INSERT INTO people (id, email, name, updated_at) VALUES ('p1', 'jane@acme.com', 'Jane Doe', '2026-01-01')",
            [],
        ).unwrap();
        conn.execute(
            "INSERT INTO account_team (account_id, person_id, role) VALUES ('a1', 'p1', 'champion')",
            [],
        ).unwrap();

        // Old meeting (60 days ago)
        conn.execute(
            "INSERT INTO meetings_history (id, title, meeting_type, start_time, created_at)
             VALUES ('m1', 'Sync', 'external', '2026-01-14 10:00:00', '2026-01-14')",
            [],
        ).unwrap();
        conn.execute(
            "INSERT INTO meeting_entities (meeting_id, entity_id, entity_type)
             VALUES ('m1', 'p1', 'person')",
            [],
        ).unwrap();

        let ctx = test_ctx(today);
        let results = detect_stale_champion(&db, &ctx);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].signal_type, "proactive_stale_champion");
        assert!(results[0].headline.contains("Jane Doe"));
        assert_eq!(results[0].confidence, 0.85);
    }

    #[test]
    fn test_stale_champion_skips_recent_contact() {
        let db = test_db();
        let conn = db.conn_ref();
        let today = NaiveDate::from_ymd_opt(2026, 3, 15).unwrap();

        conn.execute(
            "INSERT INTO accounts (id, name, contract_end, archived, is_internal, updated_at)
             VALUES ('a1', 'AcmeCo', '2026-05-14', 0, 0, '2026-01-01')",
            [],
        ).unwrap();
        conn.execute(
            "INSERT INTO people (id, email, name, updated_at) VALUES ('p1', 'jane@acme.com', 'Jane Doe', '2026-01-01')",
            [],
        ).unwrap();
        conn.execute(
            "INSERT INTO account_team (account_id, person_id, role) VALUES ('a1', 'p1', 'champion')",
            [],
        ).unwrap();

        // Recent meeting (10 days ago)
        conn.execute(
            "INSERT INTO meetings_history (id, title, meeting_type, start_time, created_at)
             VALUES ('m1', 'Sync', 'external', '2026-03-05 10:00:00', '2026-03-05')",
            [],
        ).unwrap();
        conn.execute(
            "INSERT INTO meeting_entities (meeting_id, entity_id, entity_type)
             VALUES ('m1', 'p1', 'person')",
            [],
        ).unwrap();

        let ctx = test_ctx(today);
        let results = detect_stale_champion(&db, &ctx);
        assert!(results.is_empty(), "Should not fire when contact is recent");
    }

    // -- Detector 6: Action cluster --

    #[test]
    fn test_action_cluster_fires_on_overdue_pile() {
        let db = test_db();
        let conn = db.conn_ref();
        let today = NaiveDate::from_ymd_opt(2026, 3, 15).unwrap();

        conn.execute(
            "INSERT INTO accounts (id, name, updated_at, is_internal, archived)
             VALUES ('a1', 'AcmeCo', '2026-01-01', 0, 0)",
            [],
        ).unwrap();

        // Insert 6 pending actions, 4 overdue
        for i in 0..6 {
            let due = if i < 4 { "2026-02-01" } else { "2026-04-01" };
            conn.execute(
                "INSERT INTO actions (id, title, status, account_id, due_date, created_at, updated_at)
                 VALUES (?1, ?2, 'pending', 'a1', ?3, '2026-01-01', '2026-01-01')",
                params![format!("act-{}", i), format!("Action {}", i), due],
            ).unwrap();
        }

        let ctx = test_ctx(today);
        let results = detect_action_cluster(&db, &ctx);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].signal_type, "proactive_action_cluster");
        assert!(results[0].headline.contains("AcmeCo"));
    }

    #[test]
    fn test_action_cluster_no_fire_below_threshold() {
        let db = test_db();
        let conn = db.conn_ref();
        let today = NaiveDate::from_ymd_opt(2026, 3, 15).unwrap();

        conn.execute(
            "INSERT INTO accounts (id, name, updated_at, is_internal, archived)
             VALUES ('a1', 'AcmeCo', '2026-01-01', 0, 0)",
            [],
        ).unwrap();

        // Only 3 pending actions (below threshold of 5)
        for i in 0..3 {
            conn.execute(
                "INSERT INTO actions (id, title, status, account_id, due_date, created_at, updated_at)
                 VALUES (?1, ?2, 'pending', 'a1', '2026-02-01', '2026-01-01', '2026-01-01')",
                params![format!("act-{}", i), format!("Action {}", i)],
            ).unwrap();
        }

        let ctx = test_ctx(today);
        let results = detect_action_cluster(&db, &ctx);
        assert!(results.is_empty());
    }

    // -- Detector 7: Prep coverage gap --

    #[test]
    fn test_prep_gap_fires_when_low_coverage() {
        let db = test_db();
        let conn = db.conn_ref();
        let today = NaiveDate::from_ymd_opt(2026, 3, 15).unwrap();
        let tomorrow = "2026-03-16";

        // Insert 4 external meetings tomorrow, none with entities
        for i in 0..4 {
            conn.execute(
                "INSERT INTO meetings_history (id, title, meeting_type, start_time, created_at)
                 VALUES (?1, ?2, 'external', ?3, '2026-03-15')",
                params![
                    format!("m{}", i),
                    format!("Meeting {}", i),
                    format!("{} 10:00:00", tomorrow)
                ],
            ).unwrap();
        }

        let ctx = test_ctx(today);
        let results = detect_prep_coverage_gap(&db, &ctx);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].signal_type, "proactive_prep_gap");
        assert!(results[0].headline.contains("4 external meetings"));
    }

    #[test]
    fn test_prep_gap_no_fire_when_good_coverage() {
        let db = test_db();
        let conn = db.conn_ref();
        let today = NaiveDate::from_ymd_opt(2026, 3, 15).unwrap();
        let tomorrow = "2026-03-16";

        // Insert 3 external meetings
        for i in 0..3 {
            conn.execute(
                "INSERT INTO meetings_history (id, title, meeting_type, start_time, created_at)
                 VALUES (?1, ?2, 'external', ?3, '2026-03-15')",
                params![
                    format!("m{}", i),
                    format!("Meeting {}", i),
                    format!("{} 10:00:00", tomorrow)
                ],
            ).unwrap();
        }

        // Link entities to 2 of 3 (67% > 60%)
        for i in 0..2 {
            conn.execute(
                "INSERT INTO meeting_entities (meeting_id, entity_id, entity_type)
                 VALUES (?1, ?2, 'account')",
                params![format!("m{}", i), format!("a{}", i)],
            ).unwrap();
        }

        let ctx = test_ctx(today);
        let results = detect_prep_coverage_gap(&db, &ctx);
        assert!(results.is_empty(), "67% coverage should not trigger");
    }

    #[test]
    fn test_prep_gap_no_fire_under_3_meetings() {
        let db = test_db();
        let conn = db.conn_ref();
        let today = NaiveDate::from_ymd_opt(2026, 3, 15).unwrap();
        let tomorrow = "2026-03-16";

        // Only 2 external meetings
        for i in 0..2 {
            conn.execute(
                "INSERT INTO meetings_history (id, title, meeting_type, start_time, created_at)
                 VALUES (?1, ?2, 'external', ?3, '2026-03-15')",
                params![
                    format!("m{}", i),
                    format!("Meeting {}", i),
                    format!("{} 10:00:00", tomorrow)
                ],
            ).unwrap();
        }

        let ctx = test_ctx(today);
        let results = detect_prep_coverage_gap(&db, &ctx);
        assert!(results.is_empty());
    }

    // -- Detector 8: No contact accounts --

    #[test]
    fn test_no_contact_fires_for_silent_account() {
        let db = test_db();
        let conn = db.conn_ref();
        let today = NaiveDate::from_ymd_opt(2026, 3, 15).unwrap();

        conn.execute(
            "INSERT INTO accounts (id, name, updated_at, is_internal, archived)
             VALUES ('a1', 'SilentCo', '2026-01-01', 0, 0)",
            [],
        ).unwrap();

        let ctx = test_ctx(today);
        let results = detect_no_contact_accounts(&db, &ctx);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].signal_type, "proactive_no_contact");
        assert!(results[0].headline.contains("SilentCo"));
    }

    #[test]
    fn test_no_contact_skips_internal_accounts() {
        let db = test_db();
        let conn = db.conn_ref();
        let today = NaiveDate::from_ymd_opt(2026, 3, 15).unwrap();

        conn.execute(
            "INSERT INTO accounts (id, name, updated_at, is_internal, archived)
             VALUES ('a1', 'InternalCo', '2026-01-01', 1, 0)",
            [],
        ).unwrap();

        let ctx = test_ctx(today);
        let results = detect_no_contact_accounts(&db, &ctx);
        assert!(results.is_empty(), "Internal accounts should be excluded");
    }

    // -- Detector 9: Renewal proximity --

    #[test]
    fn test_renewal_proximity_fires_tiered_confidence() {
        let db = test_db();
        let ctx = test_ctx(NaiveDate::from_ymd_opt(2026, 2, 18).unwrap());

        // Account renewing in 25 days (should get 0.90 confidence)
        db.conn_ref()
            .execute(
                "INSERT INTO accounts (id, name, contract_end, updated_at, archived, is_internal)
                 VALUES ('a1', 'NearCo', '2026-03-15', '2026-01-01', 0, 0)",
                [],
            )
            .unwrap();

        let insights = detect_renewal_proximity(&db, &ctx);
        assert_eq!(insights.len(), 1);
        assert_eq!(insights[0].signal_type, "renewal_proximity");
        assert_eq!(insights[0].confidence, 0.90);
    }

    #[test]
    fn test_renewal_proximity_skips_churned() {
        let db = test_db();
        let ctx = test_ctx(NaiveDate::from_ymd_opt(2026, 2, 18).unwrap());

        db.conn_ref()
            .execute(
                "INSERT INTO accounts (id, name, contract_end, updated_at, archived, is_internal)
                 VALUES ('a1', 'ChurnedCo', '2026-03-15', '2026-01-01', 0, 0)",
                [],
            )
            .unwrap();

        db.conn_ref()
            .execute(
                "INSERT INTO account_events (account_id, event_type, event_date)
                 VALUES ('a1', 'churn', '2026-02-01')",
                [],
            )
            .unwrap();

        let insights = detect_renewal_proximity(&db, &ctx);
        assert!(insights.is_empty(), "Should skip churned accounts");
    }

    #[test]
    fn test_renewal_proximity_60d_confidence() {
        let db = test_db();
        let ctx = test_ctx(NaiveDate::from_ymd_opt(2026, 2, 18).unwrap());

        // Account renewing in ~45 days (should get 0.70 confidence)
        db.conn_ref()
            .execute(
                "INSERT INTO accounts (id, name, contract_end, updated_at, archived, is_internal)
                 VALUES ('a1', 'MidCo', '2026-04-04', '2026-01-01', 0, 0)",
                [],
            )
            .unwrap();

        let insights = detect_renewal_proximity(&db, &ctx);
        assert_eq!(insights.len(), 1);
        assert_eq!(insights[0].confidence, 0.70);
    }

    #[test]
    fn test_no_contact_skips_accounts_with_recent_meeting() {
        let db = test_db();
        let conn = db.conn_ref();
        let today = NaiveDate::from_ymd_opt(2026, 3, 15).unwrap();

        conn.execute(
            "INSERT INTO accounts (id, name, updated_at, is_internal, archived)
             VALUES ('a1', 'ActiveCo', '2026-01-01', 0, 0)",
            [],
        ).unwrap();

        // Recent meeting (5 days ago)
        conn.execute(
            "INSERT INTO meetings_history (id, title, meeting_type, start_time, created_at)
             VALUES ('m1', 'Sync', 'external', '2026-03-10 10:00:00', '2026-03-10')",
            [],
        ).unwrap();
        conn.execute(
            "INSERT INTO meeting_entities (meeting_id, entity_id, entity_type)
             VALUES ('m1', 'a1', 'account')",
            [],
        ).unwrap();

        let ctx = test_ctx(today);
        let results = detect_no_contact_accounts(&db, &ctx);
        assert!(results.is_empty(), "Account with recent meeting should be excluded");
    }
}
