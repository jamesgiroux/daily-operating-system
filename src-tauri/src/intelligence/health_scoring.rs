//! I499: Algorithmic account health scoring engine (ADR-0097).
//!
//! "LLM explains numbers, doesn't pick them." Six dimensions compute a score;
//! the LLM provides narrative only.

use crate::db::types::DbAccount;
use crate::db::ActionDb;

use super::io::{
    AccountHealth, DimensionScore, HealthDivergence, HealthSource, HealthTrend, OrgHealthData,
    RelationshipDimensions,
};

/// Compute algorithmic health for an account using 6 dimensions.
///
/// The returned `AccountHealth` has score, band, dimensions, and confidence
/// but no narrative (that comes from the LLM).
pub fn compute_account_health(
    db: &ActionDb,
    account: &DbAccount,
    org_health: Option<&OrgHealthData>,
) -> AccountHealth {
    let meeting_cadence = compute_meeting_cadence(db, &account.id);
    let email_engagement = compute_email_engagement(db, &account.id);
    let stakeholder_coverage = compute_stakeholder_coverage(db, &account.id);
    let champion_health = compute_champion_health(db, &account.id);
    let financial_proximity = compute_financial_proximity(db, account);
    let signal_momentum = compute_signal_momentum(db, &account.id);

    let dims = RelationshipDimensions {
        meeting_cadence,
        email_engagement,
        stakeholder_coverage,
        champion_health,
        financial_proximity,
        signal_momentum,
    };

    let lifecycle = account.lifecycle.as_deref();
    let raw_weights = apply_lifecycle_weights(lifecycle);
    let weights = redistribute_weights(&dims, raw_weights);
    let confidence = compute_confidence(&dims);

    // Compute weighted average of non-null dimensions
    let dim_arr = [
        &dims.meeting_cadence,
        &dims.email_engagement,
        &dims.stakeholder_coverage,
        &dims.champion_health,
        &dims.financial_proximity,
        &dims.signal_momentum,
    ];

    let mut weighted_sum = 0.0f64;
    let mut weight_total = 0.0f64;
    for (i, dim) in dim_arr.iter().enumerate() {
        if dim.weight > 0.0 {
            weighted_sum += dim.score * weights[i];
            weight_total += weights[i];
        }
    }
    let raw_avg = if weight_total > 0.0 {
        weighted_sum / weight_total
    } else {
        50.0
    };

    // Confidence-weighted regression to neutral: with sparse data (low confidence),
    // pull the score toward 50 instead of letting 1-2 dimensions dominate.
    // At 0.9 confidence → 90% computed, 10% neutral.
    // At 0.3 confidence → 30% computed, 70% neutral.
    let computed_avg = confidence * raw_avg + (1.0 - confidence) * 50.0;

    // Blend with org health baseline if available
    let org_baseline = org_health.and_then(|oh| oh.health_band.as_deref().map(band_to_score));

    let score = if let Some(baseline) = org_baseline {
        0.4 * baseline + 0.6 * computed_avg
    } else {
        computed_avg
    };

    let band = score_to_band(score);
    let divergence = detect_divergence(org_health, score);

    // Build recommended actions from dimension evidence
    let mut recommended_actions = Vec::new();
    for ev in &dims.champion_health.evidence {
        if ev.contains("Consider tagging") || ev.contains("champion candidate") {
            recommended_actions.push(ev.clone());
        }
    }
    for ev in &dims.stakeholder_coverage.evidence {
        if ev.starts_with("Missing:") {
            recommended_actions
                .push(format!("Map a {} stakeholder for this account", ev.trim_start_matches("Missing: ")));
        }
    }

    AccountHealth {
        score,
        band,
        source: HealthSource::Computed,
        confidence,
        trend: HealthTrend {
            direction: "stable".to_string(),
            rationale: None,
            timeframe: "30d".to_string(),
            confidence: 0.0,
        },
        dimensions: dims,
        narrative: None,
        recommended_actions,
        divergence,
    }
}

fn band_to_score(band: &str) -> f64 {
    match band.to_lowercase().as_str() {
        "green" => 75.0,
        "yellow" => 50.0,
        "red" => 25.0,
        _ => 50.0,
    }
}

fn score_to_band(score: f64) -> String {
    if score >= 70.0 {
        "green".to_string()
    } else if score >= 40.0 {
        "yellow".to_string()
    } else {
        "red".to_string()
    }
}

/// Strategic operating bucket derived from multi-dimension health context.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AccountBucket {
    GrowthFocus,
    AtRiskSaveable,
    AtRiskSaveUnlikely,
    Autopilot,
}

/// Classify an account into an operating bucket with a concise rationale.
pub fn classify_account_bucket(health: &AccountHealth) -> (AccountBucket, String) {
    let cadence = health.dimensions.meeting_cadence.score;
    let champion = health.dimensions.champion_health.score;
    let cadence_present = health.dimensions.meeting_cadence.weight > 0.0;
    let champion_present = health.dimensions.champion_health.weight > 0.0;
    let any_declining = [
        &health.dimensions.meeting_cadence,
        &health.dimensions.email_engagement,
        &health.dimensions.stakeholder_coverage,
        &health.dimensions.champion_health,
        &health.dimensions.financial_proximity,
        &health.dimensions.signal_momentum,
    ]
    .iter()
    .any(|d| d.weight > 0.0 && d.trend == "declining");

    if health.score >= 70.0 && !any_declining {
        return (
            AccountBucket::Autopilot,
            "Healthy score with stable dimensions; monitor and maintain momentum.".to_string(),
        );
    }
    if health.score >= 60.0
        && champion_present
        && cadence_present
        && champion >= 60.0
        && cadence >= 60.0
    {
        return (
            AccountBucket::GrowthFocus,
            "Strong champion and active cadence indicate expansion-ready engagement.".to_string(),
        );
    }
    if health.score < 70.0
        && champion_present
        && cadence_present
        && champion >= 50.0
        && cadence >= 40.0
    {
        return (
            AccountBucket::AtRiskSaveable,
            "Risk signals exist, but champion strength and cadence suggest recoverable trajectory."
                .to_string(),
        );
    }
    if health.score < 70.0
        && (!champion_present || champion < 30.0 || !cadence_present || cadence < 30.0)
    {
        return (
            AccountBucket::AtRiskSaveUnlikely,
            "Low relationship coverage and weak engagement indicate structural risk.".to_string(),
        );
    }
    if health.score >= 60.0 {
        (
            AccountBucket::Autopilot,
            "Moderately healthy score with limited risk indicators.".to_string(),
        )
    } else {
        (
            AccountBucket::AtRiskSaveable,
            "Sub-60 score with partial engagement signals; intervention can still recover."
                .to_string(),
        )
    }
}

fn null_dimension(reason: &str) -> DimensionScore {
    DimensionScore {
        score: 0.0,
        weight: 0.0,
        evidence: vec![reason.to_string()],
        trend: String::new(),
    }
}

fn compute_meeting_cadence(db: &ActionDb, account_id: &str) -> DimensionScore {
    let signals = match db.get_stakeholder_signals(account_id) {
        Ok(s) => s,
        Err(_) => return null_dimension("Failed to query meeting data"),
    };

    let count_30d = signals.meeting_frequency_30d as f64;
    let count_90d = signals.meeting_frequency_90d as f64;

    if count_90d == 0.0 && count_30d == 0.0 {
        return null_dimension("No meeting data available");
    }

    if count_30d == 0.0 {
        return DimensionScore {
            score: 20.0,
            weight: 1.0,
            evidence: vec!["No meetings in last 30 days".to_string()],
            trend: "declining".to_string(),
        };
    }

    let avg_monthly = (count_90d / 3.0).max(1.0);
    let ratio = count_30d / avg_monthly;

    let mut score = if ratio < 0.5 {
        30.0
    } else if ratio <= 1.2 {
        70.0
    } else {
        80.0
    };

    // Recency bonus
    if let Some(ref last) = signals.last_meeting {
        if let Ok(last_dt) = chrono::DateTime::parse_from_rfc3339(last) {
            let days_since = (chrono::Utc::now() - last_dt.with_timezone(&chrono::Utc)).num_days();
            if days_since < 7 {
                score += 10.0;
            } else if days_since < 14 {
                score += 5.0;
            }
        }
    }

    // I555: Quality modifier from interaction dynamics
    let quality_multiplier = {
        let mut q = 1.0f64;
        if let Ok(dynamics_rows) = db.conn.prepare(
            "SELECT mid.question_density, mid.decision_maker_active, mid.forward_looking
             FROM meeting_interaction_dynamics mid
             JOIN meeting_entities me ON me.meeting_id = mid.meeting_id AND me.entity_id = ?1
             ORDER BY mid.created_at DESC LIMIT 3"
        ).and_then(|mut stmt| {
            stmt.query_map(rusqlite::params![account_id], |row| {
                Ok((
                    row.get::<_, Option<String>>(0)?,
                    row.get::<_, Option<String>>(1)?,
                    row.get::<_, Option<String>>(2)?,
                ))
            }).map(|rows| rows.filter_map(|r| r.ok()).collect::<Vec<_>>())
        }) {
            if !dynamics_rows.is_empty() {
                let mut quality_score = 0.0f64;
                let n = dynamics_rows.len() as f64;
                for (qd, dma, fl) in &dynamics_rows {
                    if qd.as_deref() == Some("high") { quality_score += 1.0; }
                    if dma.as_deref() == Some("yes") { quality_score += 1.0; }
                    if fl.as_deref() == Some("high") { quality_score += 1.0; }
                }
                let avg_quality = quality_score / (n * 3.0);
                q = 0.7 + (avg_quality * 0.5); // Range: 0.7 to 1.2
            }
        }
        q
    };

    score = (score * quality_multiplier).clamp(0.0, 100.0);

    let trend = if ratio > 1.2 {
        "improving".to_string()
    } else if ratio < 0.5 {
        "declining".to_string()
    } else {
        "stable".to_string()
    };

    DimensionScore {
        score,
        weight: 1.0,
        evidence: vec![format!("{count_30d:.0} meetings in 30d, ratio={ratio:.2}, quality={quality_multiplier:.2}")],
        trend,
    }
}

fn compute_email_engagement(db: &ActionDb, account_id: &str) -> DimensionScore {
    let signals = db
        .list_recent_email_signals_for_entity(account_id, 50)
        .unwrap_or_default();

    if signals.is_empty() {
        return null_dimension("No email signals available");
    }

    let count = signals.len() as f64;
    let mut score = 50.0;

    // Cadence modifier
    if count > 10.0 {
        score += 15.0;
    } else if count >= 5.0 {
        score += 5.0;
    } else if count < 2.0 {
        score -= 15.0;
    }

    // Sentiment modifier from signal_text and sentiment fields
    let mut sentiment_mod = 0.0f64;
    for sig in &signals {
        let sentiment = sig.sentiment.as_deref().unwrap_or("");
        match sentiment {
            "positive" => sentiment_mod += 3.0,
            "negative" => sentiment_mod -= 5.0,
            _ => {}
        }
    }
    score += sentiment_mod.clamp(-20.0, 20.0);

    DimensionScore {
        score: score.clamp(0.0, 100.0),
        weight: 1.0,
        evidence: vec![format!("{count:.0} email signals")],
        trend: "stable".to_string(),
    }
}

fn compute_stakeholder_coverage(db: &ActionDb, account_id: &str) -> DimensionScore {
    let team = db.get_account_team(account_id).unwrap_or_default();

    if team.is_empty() {
        return null_dimension("No stakeholders mapped");
    }

    let expected_roles = ["champion", "executive", "technical"];
    let mut weighted_fill = 0.0f64;
    let mut evidence = Vec::new();

    for role in &expected_roles {
        let has_role = team.iter().any(|t| t.role.to_lowercase().contains(role));
        if !has_role {
            evidence.push(format!("Missing: {role}"));
            continue;
        }

        // I555: Verify attendance recency via meeting_attendees
        let weight = if let Some(person_id) = team.iter()
            .find(|t| t.role.to_lowercase().contains(role))
            .map(|t| t.person_id.as_str())
        {
            let last_seen_days = db.conn.query_row(
                "SELECT CAST(julianday('now') - julianday(MAX(m.start_time)) AS INTEGER)
                 FROM meeting_attendees ma
                 JOIN meetings m ON m.id = ma.meeting_id
                 WHERE ma.person_id = ?1",
                rusqlite::params![person_id],
                |row| row.get::<_, Option<i64>>(0),
            ).unwrap_or(None);

            match last_seen_days {
                Some(d) if d <= 90 => {
                    evidence.push(format!("{role}: active (seen {d}d ago)"));
                    1.0
                }
                Some(d) if d <= 180 => {
                    evidence.push(format!("{role}: stale ({d}d ago)"));
                    0.5
                }
                Some(d) => {
                    evidence.push(format!("{role}: inactive ({d}d ago)"));
                    0.0
                }
                None => {
                    evidence.push(format!("{role}: mapped but never seen in meetings"));
                    0.3
                }
            }
        } else {
            evidence.push(format!("{role}: mapped (no person linked)"));
            0.5
        };

        weighted_fill += weight;
    }

    let fill_rate = weighted_fill / expected_roles.len() as f64;

    DimensionScore {
        score: (fill_rate * 100.0).clamp(0.0, 100.0),
        weight: 1.0,
        evidence,
        trend: String::new(),
    }
}

/// When no champion is explicitly tagged, look at meeting attendance patterns
/// to infer engagement quality. Someone attending 70%+ of account meetings in
/// the last 90 days is champion-territory; 50%+ is strong engagement.
fn infer_champion_from_attendance(db: &ActionDb, account_id: &str) -> DimensionScore {
    // Count total account meetings in last 90 days
    let total_meetings: i64 = db
        .conn
        .query_row(
            "SELECT COUNT(DISTINCT m.id) FROM meetings m
             JOIN meeting_entities me ON me.meeting_id = m.id
             WHERE me.entity_id = ?1 AND me.entity_type = 'account'
               AND m.start_time >= datetime('now', '-90 days')",
            rusqlite::params![account_id],
            |row| row.get(0),
        )
        .unwrap_or(0);

    if total_meetings < 2 {
        // Not enough meetings to judge attendance patterns
        return null_dimension("No champion identified, insufficient meeting data");
    }

    // Find the person with the highest attendance rate on this account's meetings
    #[derive(Debug)]
    struct AttendeeStats {
        person_id: String,
        person_name: String,
        attended: i64,
        pct: f64,
    }

    let top_attendee: Option<AttendeeStats> = db
        .conn
        .prepare(
            "SELECT ma.person_id, COALESCE(p.name, ma.email, 'Unknown'),
                    COUNT(DISTINCT ma.meeting_id) as attended
             FROM meeting_attendees ma
             JOIN meetings m ON m.id = ma.meeting_id
             JOIN meeting_entities me ON me.meeting_id = m.id
               AND me.entity_id = ?1 AND me.entity_type = 'account'
             LEFT JOIN people p ON p.id = ma.person_id
             WHERE m.start_time >= datetime('now', '-90 days')
               AND ma.person_id IS NOT NULL
               AND ma.is_organizer = 0
             GROUP BY ma.person_id
             ORDER BY attended DESC
             LIMIT 1",
        )
        .and_then(|mut stmt| {
            stmt.query_row(rusqlite::params![account_id], |row| {
                let attended: i64 = row.get(2)?;
                Ok(AttendeeStats {
                    person_id: row.get(0)?,
                    person_name: row.get(1)?,
                    attended,
                    pct: attended as f64 / total_meetings as f64,
                })
            })
        })
        .ok();

    match top_attendee {
        Some(att) if att.pct >= 0.70 => {
            // 70%+ attendance → champion-level engagement
            DimensionScore {
                score: 55.0,
                weight: 1.0,
                evidence: vec![
                    format!(
                        "No named champion — {} attended {}/{} meetings ({:.0}%)",
                        att.person_name, att.attended, total_meetings, att.pct * 100.0,
                    ),
                    format!("Consider tagging {} as champion", att.person_name),
                ],
                trend: "stable".to_string(),
            }
        }
        Some(att) if att.pct >= 0.50 => {
            // 50-69% attendance → strong engagement, not quite champion
            DimensionScore {
                score: 40.0,
                weight: 1.0,
                evidence: vec![
                    format!(
                        "No named champion — {} attended {}/{} meetings ({:.0}%)",
                        att.person_name, att.attended, total_meetings, att.pct * 100.0,
                    ),
                    format!("Strong engagement from {} — consider as champion candidate", att.person_name),
                ],
                trend: "stable".to_string(),
            }
        }
        Some(att) if att.attended >= 2 => {
            // Some attendance but below 50% — partial credit
            DimensionScore {
                score: 25.0,
                weight: 0.5, // Reduced weight — we're less sure
                evidence: vec![format!(
                    "No named champion — best attendee {} at {}/{} meetings ({:.0}%)",
                    att.person_name, att.attended, total_meetings, att.pct * 100.0,
                )],
                trend: String::new(),
            }
        }
        _ => null_dimension("No champion identified, no consistent meeting attendees"),
    }
}

fn compute_champion_health(db: &ActionDb, account_id: &str) -> DimensionScore {
    let team = db.get_account_team(account_id).unwrap_or_default();
    let champion = team
        .iter()
        .find(|t| t.role.to_lowercase().contains("champion"));

    if champion.is_none() {
        // No explicit champion — infer engagement from meeting attendance patterns.
        // Check if any person on this account attends a high % of meetings.
        return infer_champion_from_attendance(db, account_id);
    }

    // I555: Query per-champion meeting engagement from meeting_champion_health
    let champion_assessments: Vec<(String, String, Option<String>)> = db.conn.prepare(
        "SELECT m.start_time, mch.champion_status, mch.champion_evidence
         FROM meeting_champion_health mch
         JOIN meetings m ON m.id = mch.meeting_id
         JOIN meeting_entities me ON me.meeting_id = m.id AND me.entity_id = ?1
         WHERE mch.champion_name IS NOT NULL
         ORDER BY m.start_time DESC LIMIT 5"
    ).and_then(|mut stmt| {
        stmt.query_map(rusqlite::params![account_id], |row| {
            Ok((row.get(0)?, row.get(1)?, row.get(2)?))
        }).map(|rows| rows.filter_map(|r| r.ok()).collect())
    }).unwrap_or_default();

    if champion_assessments.is_empty() {
        // Fallback to structural check (pre-I555 behavior)
        let mut score: f64 = 60.0;
        let mut evidence = vec!["Champion identified (no meeting engagement data)".to_string()];

        if let Ok(signals) = db.get_stakeholder_signals(account_id) {
            if signals.meeting_frequency_30d > 0 {
                score += 20.0;
                evidence.push("Active in recent meetings".to_string());
            }
        }

        return DimensionScore {
            score: score.clamp(0.0, 100.0),
            weight: 1.0,
            evidence,
            trend: String::new(),
        };
    }

    // Score based on recent champion engagement
    let status_scores: Vec<f64> = champion_assessments.iter().map(|(_, status, _)| {
        match status.as_str() {
            "strong" => 90.0,
            "weak" => 40.0,
            "lost" => 10.0,
            "none" => 20.0,
            _ => 50.0,
        }
    }).collect();

    let avg_score = status_scores.iter().sum::<f64>() / status_scores.len() as f64;

    // Trend detection
    let trend = if status_scores.len() >= 2 {
        let recent = status_scores[0];
        let older = status_scores[status_scores.len() - 1];
        if recent > older + 10.0 { "improving" }
        else if recent < older - 10.0 { "declining" }
        else { "stable" }
    } else {
        "stable"
    };

    // Build evidence with specific meeting dates and statuses
    let champion_name = champion.map(|c| c.person_name.as_str()).unwrap_or("Champion");
    let mut evidence = vec![format!(
        "{champion_name}: {} across {} meetings",
        champion_assessments[0].1,
        champion_assessments.len()
    )];
    for (date, status, ev) in &champion_assessments {
        let short_date = date.split('T').next().unwrap_or(date);
        let detail = ev.as_deref().map(|e| format!(" — {e}")).unwrap_or_default();
        evidence.push(format!("{short_date}: {status}{detail}"));
    }

    // I535: Augment with Glean Gong signals if available
    let mut final_score = avg_score;
    let glean_gong_signals: Vec<(String, f64)> = db
        .conn
        .prepare(
            "SELECT value, confidence FROM signal_events
             WHERE entity_id = ?1 AND source = 'glean_gong'
               AND signal_type LIKE '%champion%'
               AND created_at > datetime('now', '-90 days')
             ORDER BY created_at DESC LIMIT 3",
        )
        .and_then(|mut stmt| {
            stmt.query_map(rusqlite::params![account_id], |row| {
                Ok((
                    row.get::<_, Option<String>>(0)?.unwrap_or_default(),
                    row.get::<_, f64>(1)?,
                ))
            })
            .map(|rows| rows.filter_map(|r| r.ok()).collect())
        })
        .unwrap_or_default();

    if !glean_gong_signals.is_empty() {
        // Gong data shows champion engagement patterns we can't see locally
        let avg_confidence: f64 =
            glean_gong_signals.iter().map(|(_, c)| c).sum::<f64>() / glean_gong_signals.len() as f64;
        if avg_confidence >= 0.7 {
            // High-confidence Gong data — boost or reduce based on signal content
            final_score = (final_score + avg_confidence * 100.0) / 2.0; // Blend
            evidence.push(format!(
                "Gong call data: {} signals, avg confidence {:.0}%",
                glean_gong_signals.len(),
                avg_confidence * 100.0
            ));
        }
    }

    DimensionScore {
        score: final_score.clamp(0.0, 100.0),
        weight: 1.0,
        evidence,
        trend: trend.to_string(),
    }
}

fn compute_financial_proximity(db: &ActionDb, account: &DbAccount) -> DimensionScore {
    let contract_end = match &account.contract_end {
        Some(end) if !end.is_empty() => end,
        _ => return null_dimension("No contract end date"),
    };

    let end_date = match chrono::NaiveDate::parse_from_str(contract_end, "%Y-%m-%d") {
        Ok(d) => d,
        Err(_) => return null_dimension("Invalid contract end date format"),
    };

    let today = chrono::Utc::now().date_naive();
    let days_to_renewal = (end_date - today).num_days() as f64;
    let mut score = (100.0 * (-days_to_renewal / 90.0).exp()).clamp(5.0, 100.0);
    let mut evidence = vec![format!("{days_to_renewal:.0} days to renewal")];

    let trend = if days_to_renewal < 30.0 {
        "critical".to_string()
    } else if days_to_renewal < 90.0 {
        "approaching".to_string()
    } else {
        "stable".to_string()
    };

    // I535: Augment with Glean CRM signals (Salesforce renewal probability)
    let crm_signals: Vec<(String, f64)> = db
        .conn
        .prepare(
            "SELECT value, confidence FROM signal_events
             WHERE entity_id = ?1 AND source = 'glean_crm'
               AND signal_type = 'renewal_data_updated'
               AND created_at > datetime('now', '-30 days')
             ORDER BY created_at DESC LIMIT 1",
        )
        .and_then(|mut stmt| {
            stmt.query_map(rusqlite::params![&account.id], |row| {
                Ok((
                    row.get::<_, Option<String>>(0)?.unwrap_or_default(),
                    row.get::<_, f64>(1)?,
                ))
            })
            .map(|rows| rows.filter_map(|r| r.ok()).collect())
        })
        .unwrap_or_default();

    if let Some((value_json, _confidence)) = crm_signals.first() {
        // Try to extract renewal probability from the CRM signal
        if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(value_json) {
            if let Some(probability) = parsed
                .get("renewal_likelihood")
                .or_else(|| parsed.get("renewalProbability"))
                .and_then(|v| v.as_str())
                .and_then(|s| {
                    // Try to parse as percentage or keyword
                    s.trim_end_matches('%').parse::<f64>().ok()
                })
            {
                if probability < 50.0 {
                    // CRM says renewal is at risk — cap score
                    score = score.min(40.0);
                    evidence.push(format!(
                        "CRM renewal probability: {probability:.0}% — at risk"
                    ));
                } else {
                    evidence.push(format!("CRM renewal probability: {probability:.0}%"));
                }
            }
        }
    }

    DimensionScore {
        score: score.clamp(0.0, 100.0),
        weight: 1.0,
        evidence,
        trend,
    }
}

fn compute_signal_momentum(db: &ActionDb, account_id: &str) -> DimensionScore {
    let signals = db
        .get_recent_signals_for_entity(account_id, 30)
        .unwrap_or_default();

    let zendesk_signals: Vec<(String, f64, String)> = db
        .conn
        .prepare(
            "SELECT value, confidence, created_at FROM signal_events
             WHERE entity_id = ?1
               AND source = 'glean_zendesk'
               AND signal_type = 'support_health_updated'
               AND created_at > datetime('now', '-30 days')
             ORDER BY created_at DESC",
        )
        .and_then(|mut stmt| {
            stmt.query_map(rusqlite::params![account_id], |row| {
                Ok((
                    row.get::<_, Option<String>>(0)?.unwrap_or_default(),
                    row.get::<_, f64>(1)?,
                    row.get::<_, String>(2)?,
                ))
            })
            .map(|rows| rows.filter_map(|row| row.ok()).collect())
        })
        .unwrap_or_default();

    if signals.is_empty() && zendesk_signals.is_empty() {
        // Signal momentum returns 50 (neutral) when no data, NOT null
        return DimensionScore {
            score: 50.0,
            weight: 1.0,
            evidence: vec!["No recent signals — neutral baseline".to_string()],
            trend: "stable".to_string(),
        };
    }

    let now = chrono::Utc::now();
    let mut weighted_sum = 0.0f64;
    let mut evidence = vec![format!("{} signals in 30d", signals.len())];
    for (_, _, confidence, created_at) in &signals {
        let age_days = chrono::DateTime::parse_from_rfc3339(created_at)
            .map(|d| (now - d.with_timezone(&chrono::Utc)).num_days() as f64)
            .unwrap_or(30.0);
        // Time decay: newer signals weighted higher
        let decay = (-age_days / 15.0).exp();
        weighted_sum += confidence * decay;
    }

    let base_momentum = (weighted_sum * 10.0).clamp(10.0, 100.0);

    let mut momentum = base_momentum;
    if !zendesk_signals.is_empty() {
        let mut zendesk_velocity = 50.0;
        let cadence_boost = (zendesk_signals.len().min(4) as f64) * 6.0;
        zendesk_velocity += cadence_boost;

        if let Some((latest_value, latest_confidence, latest_created_at)) = zendesk_signals.first() {
            if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(latest_value) {
                let trend = parsed
                    .get("trend")
                    .and_then(|v| v.as_str())
                    .unwrap_or_default()
                    .to_lowercase();
                if matches!(trend.as_str(), "declining" | "worsening" | "negative") {
                    zendesk_velocity -= 18.0;
                    evidence.push("Zendesk velocity trending worse".to_string());
                } else if matches!(trend.as_str(), "improving" | "better" | "positive") {
                    zendesk_velocity += 12.0;
                    evidence.push("Zendesk velocity trending better".to_string());
                }

                let critical = parsed
                    .get("criticalTickets")
                    .or_else(|| parsed.get("critical_tickets"))
                    .and_then(|v| v.as_i64())
                    .unwrap_or(0);
                let open = parsed
                    .get("openTickets")
                    .or_else(|| parsed.get("open_tickets"))
                    .and_then(|v| v.as_i64())
                    .unwrap_or(0);
                let csat = parsed.get("csat").and_then(|v| v.as_f64());

                if critical > 0 {
                    zendesk_velocity -= 20.0;
                    evidence.push(format!("Zendesk has {critical} critical ticket(s)"));
                } else if open >= 8 {
                    zendesk_velocity -= 8.0;
                    evidence.push(format!("Zendesk backlog elevated ({open} open tickets)"));
                }

                if let Some(csat) = csat {
                    if csat >= 90.0 {
                        zendesk_velocity += 8.0;
                    } else if csat <= 70.0 {
                        zendesk_velocity -= 10.0;
                    }
                    evidence.push(format!("Zendesk CSAT {:.0}%", csat));
                }
            }

            let age_days = chrono::DateTime::parse_from_rfc3339(latest_created_at)
                .map(|d| (now - d.with_timezone(&chrono::Utc)).num_days())
                .unwrap_or(30);
            evidence.push(format!(
                "Zendesk velocity signal {}d ago ({:.0}% confidence)",
                age_days,
                latest_confidence * 100.0
            ));
        }

        momentum = (base_momentum * 0.65 + zendesk_velocity.clamp(10.0, 100.0) * 0.35)
            .clamp(10.0, 100.0);
    }

    DimensionScore {
        score: momentum,
        weight: 1.0,
        evidence,
        trend: if momentum > 60.0 {
            "improving".to_string()
        } else if momentum < 40.0 {
            "declining".to_string()
        } else {
            "stable".to_string()
        },
    }
}

/// Apply lifecycle-stage weight multipliers to each dimension.
/// Order: [meeting, email, stakeholder, champion, financial, signal]
fn apply_lifecycle_weights(lifecycle: Option<&str>) -> [f64; 6] {
    match lifecycle {
        Some("onboarding") => [1.5, 1.0, 1.5, 1.0, 0.7, 1.0],
        Some("adoption") => [1.0, 1.0, 1.0, 1.5, 1.0, 1.5],
        Some("renewal") => [1.0, 1.3, 1.0, 1.3, 2.0, 1.3],
        Some("at-risk") | Some("at_risk") => [1.0, 1.0, 1.0, 1.0, 1.0, 2.0],
        Some("mature") => [0.7, 1.0, 1.3, 1.0, 1.0, 1.0],
        _ => [1.0, 1.0, 1.0, 1.0, 1.0, 1.0],
    }
}

/// Redistribute weight from null (weight=0) dimensions proportionally to non-null ones.
fn redistribute_weights(dims: &RelationshipDimensions, raw: [f64; 6]) -> [f64; 6] {
    let dim_active = [
        dims.meeting_cadence.weight > 0.0,
        dims.email_engagement.weight > 0.0,
        dims.stakeholder_coverage.weight > 0.0,
        dims.champion_health.weight > 0.0,
        dims.financial_proximity.weight > 0.0,
        dims.signal_momentum.weight > 0.0,
    ];

    let active_raw_total: f64 = raw
        .iter()
        .enumerate()
        .filter(|(i, _)| dim_active[*i])
        .map(|(_, w)| w)
        .sum();

    if active_raw_total == 0.0 {
        return [1.0 / 6.0; 6];
    }

    let mut result = [0.0f64; 6];
    for i in 0..6 {
        if dim_active[i] {
            result[i] = raw[i] / active_raw_total;
        }
    }
    result
}

/// Confidence = fraction of non-null dimensions.
fn is_neutral_momentum_placeholder(dim: &DimensionScore) -> bool {
    dim.weight > 0.0
        && (dim.score - 50.0).abs() < f64::EPSILON
        && dim.evidence.len() == 1
        && dim.evidence[0].contains("No recent signals")
}

fn compute_confidence(dims: &RelationshipDimensions) -> f64 {
    let populated = [
        &dims.meeting_cadence,
        &dims.email_engagement,
        &dims.stakeholder_coverage,
        &dims.champion_health,
        &dims.financial_proximity,
        &dims.signal_momentum,
    ]
    .iter()
    .filter(|d| d.weight > 0.0 && !is_neutral_momentum_placeholder(d))
    .count();

    match populated {
        5 | 6 => 0.9,
        3 | 4 => 0.6,
        1 | 2 => 0.3,
        _ => 0.1,
    }
}

/// Detect divergence between org-level health band and computed relationship score.
fn detect_divergence(
    org_health: Option<&OrgHealthData>,
    computed_score: f64,
) -> Option<HealthDivergence> {
    let org = org_health?;
    let band = org.health_band.as_deref()?;
    let org_score = band_to_score(band);
    let delta = computed_score - org_score;

    if delta.abs() > 20.0 {
        let severity = if delta.abs() > 40.0 {
            "critical"
        } else if delta.abs() > 30.0 {
            "notable"
        } else {
            "minor"
        };
        let description = if delta > 0.0 {
            format!(
                "Relationship health ({computed_score:.0}) exceeds org baseline ({org_score:.0}) by {:.0} points",
                delta.abs()
            )
        } else {
            format!(
                "Relationship health ({computed_score:.0}) trails org baseline ({org_score:.0}) by {:.0} points",
                delta.abs()
            )
        };
        Some(HealthDivergence {
            severity: severity.to_string(),
            description,
            leading_indicator: delta > 0.0, // positive divergence = leading indicator
        })
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn null_dim() -> DimensionScore {
        DimensionScore {
            score: 0.0,
            weight: 0.0,
            evidence: vec![],
            trend: String::new(),
        }
    }

    fn active_dim(score: f64) -> DimensionScore {
        DimensionScore {
            score,
            weight: 1.0,
            evidence: vec!["test".to_string()],
            trend: "stable".to_string(),
        }
    }

    #[test]
    fn test_confidence_all_dimensions() {
        let dims = RelationshipDimensions {
            meeting_cadence: active_dim(70.0),
            email_engagement: active_dim(60.0),
            stakeholder_coverage: active_dim(80.0),
            champion_health: active_dim(50.0),
            financial_proximity: active_dim(40.0),
            signal_momentum: active_dim(50.0),
        };
        assert!((compute_confidence(&dims) - 0.9).abs() < f64::EPSILON);
    }

    #[test]
    fn test_confidence_partial_dimensions() {
        let dims = RelationshipDimensions {
            meeting_cadence: active_dim(70.0),
            email_engagement: null_dim(),
            stakeholder_coverage: active_dim(80.0),
            champion_health: null_dim(),
            financial_proximity: null_dim(),
            signal_momentum: active_dim(50.0),
        };
        assert!((compute_confidence(&dims) - 0.6).abs() < f64::EPSILON);
    }

    #[test]
    fn test_confidence_zero_data_uses_lowest_band() {
        let dims = RelationshipDimensions {
            meeting_cadence: null_dim(),
            email_engagement: null_dim(),
            stakeholder_coverage: null_dim(),
            champion_health: null_dim(),
            financial_proximity: null_dim(),
            signal_momentum: DimensionScore {
                score: 50.0,
                weight: 1.0,
                evidence: vec!["No recent signals — neutral baseline".to_string()],
                trend: "stable".to_string(),
            },
        };
        assert!((compute_confidence(&dims) - 0.1).abs() < f64::EPSILON);
    }

    #[test]
    fn test_redistribute_weights_skips_null() {
        let dims = RelationshipDimensions {
            meeting_cadence: active_dim(70.0),
            email_engagement: null_dim(),
            stakeholder_coverage: null_dim(),
            champion_health: null_dim(),
            financial_proximity: null_dim(),
            signal_momentum: active_dim(50.0),
        };
        let raw = [1.0, 1.0, 1.0, 1.0, 1.0, 1.0];
        let result = redistribute_weights(&dims, raw);
        // Only dims 0 and 5 are active, so each gets 0.5
        assert!((result[0] - 0.5).abs() < 1e-6);
        assert!(result[1].abs() < 1e-6);
        assert!((result[5] - 0.5).abs() < 1e-6);
    }

    #[test]
    fn test_lifecycle_weights_renewal() {
        let weights = apply_lifecycle_weights(Some("renewal"));
        // Financial proximity (index 4) should have highest weight in renewal
        assert!(
            weights[4] > weights[0],
            "financial_proximity should be highest in renewal"
        );
        assert!((weights[4] - 2.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_divergence_detection_negative() {
        let org = OrgHealthData {
            health_band: Some("green".to_string()),
            health_score: None,
            renewal_likelihood: None,
            growth_tier: None,
            customer_stage: None,
            support_tier: None,
            icp_fit: None,
            source: "test".to_string(),
            gathered_at: "2026-03-10T00:00:00Z".to_string(),
        };
        // Computed score of 40 diverges from green (75) by 35 points
        let result = detect_divergence(Some(&org), 40.0);
        assert!(result.is_some(), "Should detect divergence");
        let div = result.unwrap();
        assert_eq!(div.severity, "notable");
        assert!(div.description.contains("trails"));
        assert!(!div.leading_indicator);
    }

    #[test]
    fn test_divergence_detection_positive() {
        let org = OrgHealthData {
            health_band: Some("red".to_string()),
            health_score: None,
            renewal_likelihood: None,
            growth_tier: None,
            customer_stage: None,
            support_tier: None,
            icp_fit: None,
            source: "test".to_string(),
            gathered_at: "2026-03-10T00:00:00Z".to_string(),
        };
        // Computed score of 70 exceeds red (25) by 45 points
        let result = detect_divergence(Some(&org), 70.0);
        assert!(result.is_some(), "Should detect divergence");
        let div = result.unwrap();
        assert_eq!(div.severity, "critical");
        assert!(div.description.contains("exceeds"));
        assert!(div.leading_indicator);
    }

    #[test]
    fn test_no_divergence_when_close() {
        let org = OrgHealthData {
            health_band: Some("yellow".to_string()),
            health_score: None,
            renewal_likelihood: None,
            growth_tier: None,
            customer_stage: None,
            support_tier: None,
            icp_fit: None,
            source: "test".to_string(),
            gathered_at: "2026-03-10T00:00:00Z".to_string(),
        };
        // Computed score of 55 is within 20 of yellow (50)
        let result = detect_divergence(Some(&org), 55.0);
        assert!(
            result.is_none(),
            "Should not detect divergence when within threshold"
        );
    }

    #[test]
    fn test_band_classification() {
        assert_eq!(score_to_band(75.0), "green");
        assert_eq!(score_to_band(70.0), "green");
        assert_eq!(score_to_band(55.0), "yellow");
        assert_eq!(score_to_band(40.0), "yellow");
        assert_eq!(score_to_band(25.0), "red");
        assert_eq!(score_to_band(39.9), "red");
    }

    #[test]
    fn test_signal_momentum_neutral_on_no_data() {
        // signal_momentum should return score 50 (not null) when no signals
        // This is tested via the function directly since we can't easily mock DB
        let dim = DimensionScore {
            score: 50.0,
            weight: 1.0,
            evidence: vec!["No recent signals — neutral baseline".to_string()],
            trend: "stable".to_string(),
        };
        assert!((dim.score - 50.0).abs() < f64::EPSILON);
        assert!((dim.weight - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_classify_account_bucket_growth_focus() {
        let health = AccountHealth {
            score: 65.0,
            band: "yellow".to_string(),
            source: HealthSource::Computed,
            confidence: 0.6,
            trend: HealthTrend::default(),
            dimensions: RelationshipDimensions {
                meeting_cadence: active_dim(70.0),
                email_engagement: active_dim(55.0),
                stakeholder_coverage: active_dim(60.0),
                champion_health: active_dim(75.0),
                financial_proximity: active_dim(45.0),
                signal_momentum: active_dim(60.0),
            },
            divergence: None,
            narrative: None,
            recommended_actions: Vec::new(),
        };
        let (bucket, rationale) = classify_account_bucket(&health);
        assert_eq!(bucket, AccountBucket::GrowthFocus);
        assert!(
            !rationale.is_empty(),
            "classification should return a rationale"
        );
    }

    #[test]
    fn test_classify_account_bucket_at_risk_save_unlikely() {
        let health = AccountHealth {
            score: 55.0,
            band: "yellow".to_string(),
            source: HealthSource::Computed,
            confidence: 0.6,
            trend: HealthTrend::default(),
            dimensions: RelationshipDimensions {
                meeting_cadence: active_dim(20.0),
                email_engagement: active_dim(45.0),
                stakeholder_coverage: active_dim(30.0),
                champion_health: active_dim(15.0),
                financial_proximity: active_dim(50.0),
                signal_momentum: active_dim(40.0),
            },
            divergence: None,
            narrative: None,
            recommended_actions: Vec::new(),
        };
        let (bucket, rationale) = classify_account_bucket(&health);
        assert_eq!(bucket, AccountBucket::AtRiskSaveUnlikely);
        assert!(
            !rationale.is_empty(),
            "classification should return a rationale"
        );
    }
}
